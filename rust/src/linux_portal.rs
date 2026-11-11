//! Wayland: one consented RemoteDesktop + ScreenCast session, latest frame only.
//! Pure validation helpers also compile in Windows tests; native code is Linux-only.
use crate::goruntu::Goruntu;
use anyhow::{ensure, Result};

const PORTAL_ERROR: &str = "Ekran paylaşım izni alınamadı; paylaşımı yeniden başlatıp izin ver.";
const MAX_PIXELS: usize = 16_777_216;

#[derive(Clone, Copy)]
enum PixelFormat {
    Rgb,
    Rgba,
    Rgbx,
    Bgra,
    Bgrx,
}

/// Bounds refer to SPA chunk offset/size, not to the whole mapped allocation.
fn rgba_frame(
    source: &[u8],
    offset: usize,
    chunk_size: usize,
    stride: i32,
    width: u32,
    height: u32,
    format: PixelFormat,
) -> Result<Goruntu> {
    let width_usize = width as usize;
    let height_usize = height as usize;
    let pixels = width_usize.checked_mul(height_usize).unwrap_or(usize::MAX);
    ensure!(
        width > 0 && height > 0 && width <= 8192 && height <= 8192 && pixels <= MAX_PIXELS,
        "invalid frame size"
    );
    let bpp = if matches!(format, PixelFormat::Rgb) {
        3
    } else {
        4
    };
    let row = width_usize
        .checked_mul(bpp)
        .ok_or_else(|| anyhow::anyhow!("invalid row"))?;
    ensure!(stride > 0 && stride as usize >= row, "unsupported stride");
    let required = (height_usize - 1)
        .checked_mul(stride as usize)
        .and_then(|v| v.checked_add(row))
        .ok_or_else(|| anyhow::anyhow!("invalid frame layout"))?;
    ensure!(
        required <= chunk_size
            && offset
                .checked_add(chunk_size)
                .is_some_and(|end| end <= source.len()),
        "truncated frame"
    );
    let mut rgba = vec![0; pixels * 4];
    for y in 0..height_usize {
        let start = offset + y * stride as usize;
        for (src, dst) in source[start..start + row]
            .chunks_exact(bpp)
            .zip(rgba[y * width_usize * 4..(y + 1) * width_usize * 4].chunks_exact_mut(4))
        {
            let (r, b) = if matches!(format, PixelFormat::Bgra | PixelFormat::Bgrx) {
                (src[2], src[0])
            } else {
                (src[0], src[2])
            };
            // Desktop content is opaque; ignore undefined X and alpha bytes.
            dst.copy_from_slice(&[r, src[1], b, 255]);
        }
    }
    Ok(Goruntu {
        genislik: width,
        yukseklik: height,
        rgba,
    })
}

fn logical_point(x: f32, y: f32, size: (u32, u32)) -> Result<(f64, f64)> {
    ensure!(
        x.is_finite() && y.is_finite() && size.0 > 0 && size.1 > 0,
        "invalid pointer coordinates"
    );
    Ok((
        f64::from(x.clamp(0.0, 1.0)) * f64::from(size.0 - 1),
        f64::from(y.clamp(0.0, 1.0)) * f64::from(size.1 - 1),
    ))
}

fn permission_granted(code: u32) -> Result<()> {
    ensure!(code == 0, PORTAL_ERROR);
    Ok(())
}

#[derive(Default)]
struct FrameSlot {
    sequence: u64,
    frame: Option<(Goruntu, u64)>,
    closed: bool,
}

impl FrameSlot {
    fn publish(&mut self, frame: Goruntu, timestamp: u64) {
        if !self.closed {
            self.sequence = self.sequence.wrapping_add(1);
            self.frame = Some((frame, timestamp));
        }
    }
    fn close(&mut self) {
        self.closed = true;
        self.frame = None;
    }
}

#[cfg(target_os = "linux")]
pub(crate) use native::{enjektor, wayland_oturumu, yakalayici};

#[cfg(target_os = "linux")]
mod native {
    use super::*;
    use crate::{
        platform::{Enjektor, Yakalayici},
        protokol::{FareTusu, Girdi},
    };
    use anyhow::{anyhow, bail, Context};
    use futures_lite::StreamExt;
    use pipewire::{
        context::ContextRc,
        keys::{MEDIA_CATEGORY, MEDIA_ROLE, MEDIA_TYPE},
        loop_::Timeout,
        main_loop::MainLoopRc,
        properties,
        spa::{
            param::{
                format::{FormatProperties, MediaSubtype, MediaType},
                format_utils,
                video::{VideoFormat, VideoInfoRaw},
                ParamType,
            },
            pod::{self, serialize::PodSerializer, Pod},
            utils::{Direction, Rectangle, SpaTypes},
        },
        stream::{StreamFlags, StreamRc, StreamState},
    };
    use std::{
        collections::HashMap,
        io::Cursor,
        os::fd::OwnedFd,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc, Condvar, Mutex, OnceLock, Weak,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };
    use tokio::sync::mpsc as async_mpsc;
    use zbus::{
        zvariant::{DynamicType, OwnedObjectPath, OwnedValue, Value},
        Connection, Proxy,
    };

    const DESKTOP: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const REMOTE: &str = "org.freedesktop.portal.RemoteDesktop";
    const SCREEN: &str = "org.freedesktop.portal.ScreenCast";
    type Options<'a> = HashMap<&'a str, Value<'a>>;
    type Results = HashMap<String, OwnedValue>;
    type Frames = Arc<(Mutex<FrameSlot>, Condvar)>;

    pub(crate) fn wayland_oturumu() -> bool {
        std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
            || std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
    }

    enum Command {
        Input(Girdi, Instant, mpsc::SyncSender<Result<()>>),
        Stop,
    }
    struct Metadata {
        node: u32,
        serial: Option<u64>,
        fd: OwnedFd,
    }

    struct Portal {
        frames: Frames,
        stop: Arc<AtomicBool>,
        input: async_mpsc::Sender<Command>,
        portal_thread: Mutex<Option<JoinHandle<()>>>,
        pipewire_thread: Mutex<Option<JoinHandle<()>>>,
    }

    fn close_frames(frames: &Frames) {
        frames.0.lock().unwrap_or_else(|e| e.into_inner()).close();
        frames.1.notify_all();
    }

    struct WorkerGuard {
        frames: Frames,
        stop: Arc<AtomicBool>,
    }
    impl Drop for WorkerGuard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            close_frames(&self.frames);
        }
    }

    impl Drop for Portal {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            close_frames(&self.frames);
            let _ = self.input.try_send(Command::Stop);
            // The actor closes Session even on error. PipeWire iterates at most
            // 100ms at a time and quits explicitly; neither thread is detached.
            for worker in [&self.portal_thread, &self.pipewire_thread] {
                if let Some(thread) = worker.lock().unwrap_or_else(|e| e.into_inner()).take() {
                    let _ = thread.join();
                }
            }
        }
    }

    fn shared_portal() -> Result<Arc<Portal>> {
        static SESSION: OnceLock<Mutex<Weak<Portal>>> = OnceLock::new();
        let mut cached = SESSION
            .get_or_init(|| Mutex::new(Weak::new()))
            .lock()
            .map_err(|_| anyhow!(PORTAL_ERROR))?;
        if let Some(existing) = cached.upgrade() {
            ensure!(!existing.stop.load(Ordering::Acquire), PORTAL_ERROR);
            return Ok(existing);
        }
        let frames = Arc::new((Mutex::new(FrameSlot::default()), Condvar::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (input, commands) = async_mpsc::channel(8);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let actor_frames = frames.clone();
        let actor_stop = stop.clone();
        let portal_thread = thread::Builder::new()
            .name("afudesk-portal".into())
            .spawn(move || {
                let _guard = WorkerGuard {
                    frames: actor_frames.clone(),
                    stop: actor_stop.clone(),
                };
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|runtime| {
                        runtime.block_on(portal_actor(
                            commands,
                            &ready_tx,
                            actor_frames.clone(),
                            actor_stop.clone(),
                        ))
                    });
                if result.is_err() {
                    let _ = ready_tx.try_send(Err(anyhow!(PORTAL_ERROR)));
                }
            })
            .map_err(|_| anyhow!(PORTAL_ERROR))?;
        let portal = Arc::new(Portal {
            frames: frames.clone(),
            stop: stop.clone(),
            input,
            portal_thread: Mutex::new(Some(portal_thread)),
            pipewire_thread: Mutex::new(None),
        });
        // Every portal request is bounded to 60s; initialization uses at most
        // four requests. A failed initialization drops and joins the actor.
        let metadata = ready_rx.recv().map_err(|_| anyhow!(PORTAL_ERROR))??;
        let pw_frames = frames.clone();
        let pw_stop = stop.clone();
        let pipewire_thread = thread::Builder::new()
            .name("afudesk-pipewire".into())
            .spawn(move || {
                let _guard = WorkerGuard {
                    frames: pw_frames.clone(),
                    stop: pw_stop.clone(),
                };
                let _ = pipewire_capture(metadata, pw_frames.clone(), pw_stop.clone());
            })
            .map_err(|_| anyhow!(PORTAL_ERROR))?;
        *portal
            .pipewire_thread
            .lock()
            .map_err(|_| anyhow!(PORTAL_ERROR))? = Some(pipewire_thread);
        *cached = Arc::downgrade(&portal);
        Ok(portal)
    }

    pub(crate) fn yakalayici() -> Result<Box<dyn Yakalayici>> {
        Ok(Box::new(PortalCapture {
            portal: shared_portal()?,
            sequence: 0,
        }))
    }
    pub(crate) fn enjektor() -> Result<Box<dyn Enjektor>> {
        Ok(Box::new(PortalInput {
            portal: shared_portal()?,
            mikrofon: Default::default(),
        }))
    }
    struct PortalCapture {
        portal: Arc<Portal>,
        sequence: u64,
    }
    impl Yakalayici for PortalCapture {
        fn yakala(&mut self) -> Result<Goruntu> {
            self.yakala_zamanli().map(|v| v.0)
        }
        fn yakala_zamanli(&mut self) -> Result<(Goruntu, u64)> {
            let (lock, changed) = &*self.portal.frames;
            let slot = lock.lock().map_err(|_| anyhow!(PORTAL_ERROR))?;
            let (slot, _) = changed
                .wait_timeout_while(slot, Duration::from_secs(2), |v| {
                    !v.closed && v.sequence == self.sequence
                })
                .map_err(|_| anyhow!(PORTAL_ERROR))?;
            ensure!(!slot.closed && slot.sequence != self.sequence, PORTAL_ERROR);
            self.sequence = slot.sequence;
            slot.frame.clone().ok_or_else(|| anyhow!(PORTAL_ERROR))
        }
    }
    struct PortalInput {
        portal: Arc<Portal>,
        mikrofon: crate::sanal_mikrofon::Mikrofon,
    }
    impl Enjektor for PortalInput {
        fn uygula(&mut self, input: &Girdi) -> Result<()> {
            ensure!(!self.portal.stop.load(Ordering::Acquire), PORTAL_ERROR);
            match input {
                Girdi::Kol(_) => return Ok(()),
                Girdi::SesAc | Girdi::Ses(_) | Girdi::SesKapat => {
                    return self.mikrofon.uygula(input)
                }
                Girdi::Metin(text) => ensure!(text.len() <= 16_384, PORTAL_ERROR),
                _ => {}
            }
            let (ack_tx, ack_rx) = mpsc::sync_channel(1);
            self.portal
                .input
                .try_send(Command::Input(
                    input.clone(),
                    Instant::now() + Duration::from_secs(5),
                    ack_tx,
                ))
                .map_err(|_| anyhow!(PORTAL_ERROR))?;
            ack_rx
                .recv_timeout(Duration::from_secs(6))
                .map_err(|_| anyhow!(PORTAL_ERROR))??;
            Ok(())
        }
    }

    async fn request<B: serde::Serialize + DynamicType>(
        connection: &Connection,
        interface: &str,
        method: &str,
        body: &B,
        token: &str,
    ) -> Result<Results> {
        let sender = connection
            .unique_name()
            .context("missing bus name")?
            .as_str()
            .trim_start_matches(':')
            .replace('.', "_");
        let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
        let response = tokio::time::timeout(Duration::from_secs(60), async {
            let request = Proxy::new(
                connection,
                DESKTOP,
                path.as_str(),
                "org.freedesktop.portal.Request",
            )
            .await?;
            // Subscribe before the method call so a quick denial cannot be missed.
            let mut responses = request.receive_signal("Response").await?;
            let proxy = Proxy::new(connection, DESKTOP, PATH, interface).await?;
            let returned: OwnedObjectPath = proxy.call(method, body).await?;
            ensure!(
                returned.as_str() == path,
                "unexpected portal request handle"
            );
            let message = responses
                .next()
                .await
                .context("portal response stream closed")?;
            let (code, results): (u32, Results) = message.body().deserialize()?;
            permission_granted(code)?;
            Ok::<_, anyhow::Error>(results)
        })
        .await;
        match response {
            Ok(Ok(results)) => Ok(results),
            _ => {
                let _ = tokio::time::timeout(
                    Duration::from_secs(2),
                    connection.call_method(
                        Some(DESKTOP),
                        path.as_str(),
                        Some("org.freedesktop.portal.Request"),
                        "Close",
                        &(),
                    ),
                )
                .await;
                bail!(PORTAL_ERROR)
            }
        }
    }

    fn token() -> String {
        format!("afudesk_{}", rand::random::<u64>())
    }
    fn options(token: &str) -> Options<'_> {
        HashMap::from([("handle_token", Value::from(token))])
    }

    async fn portal_actor(
        mut commands: async_mpsc::Receiver<Command>,
        ready: &mpsc::SyncSender<Result<Metadata>>,
        frames: Frames,
        stop: Arc<AtomicBool>,
    ) -> Result<()> {
        let connection =
            tokio::time::timeout(Duration::from_secs(5), Connection::session()).await??;
        let session_token = token();
        let create_token = token();
        let mut create_options = options(&create_token);
        create_options.insert("session_handle_token", Value::from(session_token.as_str()));
        let mut created = request(
            &connection,
            REMOTE,
            "CreateSession",
            &(create_options,),
            &create_token,
        )
        .await?;
        let session_path: String = created
            .remove("session_handle")
            .context("missing session")?
            .try_into()?;
        let session_path = OwnedObjectPath::try_from(session_path)?;
        let result = async {
            let session = tokio::time::timeout(Duration::from_secs(5), Proxy::new(&connection,
                DESKTOP, session_path.as_str(), "org.freedesktop.portal.Session")).await??;
            let mut closed = tokio::time::timeout(Duration::from_secs(5), session.receive_signal("Closed")).await??;
            let select_token = token();
            let mut select_options = options(&select_token);
            select_options.insert("types", Value::from(3u32)); // keyboard + pointer
            request(&connection, REMOTE, "SelectDevices", &(&session_path, select_options), &select_token).await?;
            let sources_token = token();
            let mut sources_options = options(&sources_token);
            sources_options.insert("types", Value::from(1u32)); // one monitor, never an unconsented fallback
            sources_options.insert("multiple", Value::from(false));
            request(&connection, SCREEN, "SelectSources", &(&session_path, sources_options), &sources_token).await?;
            let start_token = token();
            let mut started = request(&connection, REMOTE, "Start", &(&session_path, "", options(&start_token)), &start_token).await?;
            let devices = u32::try_from(started.remove("devices").context("missing device permissions")?)?;
            ensure!(devices & 3 == 3, PORTAL_ERROR);
            let streams: Vec<(u32, Results)> = started.remove("streams").context("missing streams")?.try_into()?;
            ensure!(streams.len() == 1, "one stream required");
            let (node, mut properties) = streams.into_iter().next().context("missing stream")?;
            let size: (i32, i32) = properties.remove("logical_size").or_else(|| properties.remove("size"))
                .context("missing stream coordinates")?.try_into()?;
            ensure!(size.0 > 0 && size.1 > 0 && size.0 <= 32768 && size.1 <= 32768, "invalid logical size");
            let logical = (size.0 as u32, size.1 as u32);
            let serial = properties.remove("pipewire-serial").map(u64::try_from).transpose()?;
            let screen = tokio::time::timeout(Duration::from_secs(5), Proxy::new(&connection, DESKTOP, PATH, SCREEN)).await??;
            let empty: Options<'_> = HashMap::new();
            let fd: zbus::zvariant::OwnedFd = tokio::time::timeout(Duration::from_secs(5), screen.call("OpenPipeWireRemote", &(&session_path, empty))).await??;
            ready.send(Ok(Metadata { node, serial, fd: fd.into() })).map_err(|_| anyhow!(PORTAL_ERROR))?;
            let remote = tokio::time::timeout(Duration::from_secs(5), Proxy::new(&connection, DESKTOP, PATH, REMOTE)).await??;
            loop {
                if stop.load(Ordering::Acquire) { break; }
                tokio::select! {
                    _ = closed.next() => break,
                    command = commands.recv() => match command {
                        Some(Command::Input(input, deadline, ack)) => {
                            let remaining = deadline.saturating_duration_since(Instant::now());
                            if remaining.is_zero() {
                                let _ = ack.send(Err(anyhow!(PORTAL_ERROR)));
                                continue;
                            }
                            let result = tokio::time::timeout(remaining, portal_input(&remote, &session_path, node, logical, &input)).await;
                            let ok = matches!(result, Ok(Ok(())));
                            let _ = ack.send(if ok { Ok(()) } else { Err(anyhow!(PORTAL_ERROR)) });
                            if !ok { break; }
                        },
                        Some(Command::Stop) | None => break,
                    },
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {},
                }
            }
            Ok::<_, anyhow::Error>(())
        }.await;
        stop.store(true, Ordering::Release);
        close_frames(&frames);
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            connection.call_method(
                Some(DESKTOP),
                session_path.as_str(),
                Some("org.freedesktop.portal.Session"),
                "Close",
                &(),
            ),
        )
        .await;
        result
    }

    async fn portal_input(
        remote: &Proxy<'_>,
        session: &OwnedObjectPath,
        node: u32,
        logical: (u32, u32),
        input: &Girdi,
    ) -> Result<()> {
        let empty: Options<'_> = HashMap::new();
        match input {
            Girdi::FareKonum { x, y } => {
                let (x, y) = logical_point(*x, *y, logical)?;
                remote
                    .call::<_, _, ()>("NotifyPointerMotionAbsolute", &(session, empty, node, x, y))
                    .await?;
            }
            Girdi::FareTus { tus, basili } => {
                let button: i32 = match tus {
                    FareTusu::Sol => 0x110,
                    FareTusu::Sag => 0x111,
                    FareTusu::Orta => 0x112,
                };
                remote
                    .call::<_, _, ()>(
                        "NotifyPointerButton",
                        &(session, empty, button, u32::from(*basili)),
                    )
                    .await?;
            }
            Girdi::Kaydir { dx, dy } => {
                for (axis, steps) in [(0u32, *dy), (1u32, *dx)] {
                    if steps != 0 {
                        remote
                            .call::<_, _, ()>(
                                "NotifyPointerAxisDiscrete",
                                &(session, &empty, axis, steps),
                            )
                            .await?;
                    }
                }
            }
            Girdi::Tus { ad, basili, .. } => {
                if let Some(key) = crate::platform::masaustu::tus_coz(ad) {
                    let symbol = xkeysym::Keysym::from(key).raw() as i32;
                    remote
                        .call::<_, _, ()>(
                            "NotifyKeyboardKeysym",
                            &(session, empty, symbol, u32::from(*basili)),
                        )
                        .await?;
                }
            }
            Girdi::Metin(text) => {
                ensure!(text.len() <= 16_384, "text input too large");
                for ch in text.chars() {
                    let symbol = match ch {
                        '\n' | '\r' => xkeysym::Keysym::Return,
                        '\t' => xkeysym::Keysym::Tab,
                        _ => xkeysym::Keysym::from_char(ch),
                    }
                    .raw() as i32;
                    for state in [1u32, 0u32] {
                        remote
                            .call::<_, _, ()>(
                                "NotifyKeyboardKeysym",
                                &(session, &empty, symbol, state),
                            )
                            .await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn pipewire_capture(metadata: Metadata, frames: Frames, stop: Arc<AtomicBool>) -> Result<()> {
        pipewire::init();
        let main_loop = MainLoopRc::new(None)?;
        let context = ContextRc::new(&main_loop, None)?;
        // Use only the portal-scoped remote, never the unrestricted default core.
        let core = context.connect_fd_rc(metadata.fd, None)?;
        let mut properties = properties::properties! {
            *MEDIA_TYPE => "Video", *MEDIA_CATEGORY => "Capture", *MEDIA_ROLE => "Screen",
        };
        if let Some(serial) = metadata.serial {
            properties.insert("target.object", serial.to_string());
        }
        let stream = StreamRc::new(core, "AfuDesk", properties)?;
        let state_frames = frames.clone();
        let state_stop = stop.clone();
        let process_frames = frames.clone();
        let process_stop = stop.clone();
        let _listener = stream
            .add_local_listener_with_user_data(VideoInfoRaw::default())
            .state_changed(move |_, _, old_state, state| {
                if matches!(state, StreamState::Error(_))
                    || (matches!(state, StreamState::Unconnected)
                        && !matches!(old_state, StreamState::Unconnected))
                {
                    state_stop.store(true, Ordering::Release);
                    close_frames(&state_frames);
                }
            })
            .param_changed(|_, format, id, param| {
                if id == ParamType::Format.as_raw() {
                    *format = VideoInfoRaw::default();
                    if let Some(param) = param {
                        if format_utils::parse_format(param).ok()
                            == Some((MediaType::Video, MediaSubtype::Raw))
                        {
                            let _ = format.parse(param);
                        }
                    }
                }
            })
            .process(move |stream, format| {
                if process_stop.load(Ordering::Acquire) {
                    return;
                }
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let datas = buffer.datas_mut();
                if datas.len() != 1 {
                    return;
                }
                let data = &mut datas[0];
                // Validate the chunk pointer before libspa's safe accessor,
                // which otherwise asserts on malformed native metadata.
                if data.as_raw().chunk.is_null() {
                    return;
                }
                let (offset, size, stride) = (
                    data.chunk().offset() as usize,
                    data.chunk().size() as usize,
                    data.chunk().stride(),
                );
                let Some(source) = data.data() else {
                    return;
                };
                let pixel_format = match format.format() {
                    VideoFormat::RGB => PixelFormat::Rgb,
                    VideoFormat::RGBA => PixelFormat::Rgba,
                    VideoFormat::RGBx => PixelFormat::Rgbx,
                    VideoFormat::BGRA => PixelFormat::Bgra,
                    VideoFormat::BGRx => PixelFormat::Bgrx,
                    _ => return,
                };
                let size_px = format.size();
                if let Ok(frame) = rgba_frame(
                    source,
                    offset,
                    size,
                    stride,
                    size_px.width,
                    size_px.height,
                    pixel_format,
                ) {
                    process_frames
                        .0
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .publish(frame, crate::platform::unix_ms());
                    process_frames.1.notify_all();
                }
            })
            .register()?;
        let format = pod::object!(
            SpaTypes::ObjectParamFormat,
            ParamType::EnumFormat,
            pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
            pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
            pod::property!(
                FormatProperties::VideoFormat,
                Choice,
                Enum,
                Id,
                VideoFormat::RGBA,
                VideoFormat::RGBx,
                VideoFormat::BGRA,
                VideoFormat::BGRx,
                VideoFormat::RGB
            ),
            pod::property!(
                FormatProperties::VideoSize,
                Choice,
                Range,
                Rectangle,
                Rectangle {
                    width: 1920,
                    height: 1080
                },
                Rectangle {
                    width: 1,
                    height: 1
                },
                Rectangle {
                    width: 8192,
                    height: 8192
                }
            )
        );
        let bytes = PodSerializer::serialize(Cursor::new(Vec::new()), &pod::Value::Object(format))?
            .0
            .into_inner();
        let mut params = [Pod::from_bytes(&bytes).context("invalid PipeWire format")?];
        stream.connect(
            Direction::Input,
            if metadata.serial.is_some() {
                None
            } else {
                Some(metadata.node)
            },
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
            &mut params,
        )?;
        // Finite dispatch, no detached infinite MainLoop::run(). Stop always
        // reaches quit/disconnect and the owning Portal joins this worker.
        while !stop.load(Ordering::Acquire) {
            if main_loop
                .loop_()
                .iterate(Timeout::Finite(Duration::from_millis(100)))
                < 0
            {
                break;
            }
        }
        main_loop.quit();
        let _ = stream.disconnect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portal_denial_and_cancellation_fail_closed() {
        assert!(permission_granted(0).is_ok());
        for code in [1, 2, u32::MAX] {
            assert_eq!(
                permission_granted(code).unwrap_err().to_string(),
                PORTAL_ERROR
            );
        }
    }
    #[test]
    fn stride_padding_offset_and_bgr_conversion() {
        let source = [99, 3, 2, 1, 0, 8, 8, 8, 8, 6, 5, 4, 0];
        let frame = rgba_frame(&source, 1, 12, 8, 1, 2, PixelFormat::Bgrx).unwrap();
        assert_eq!(frame.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
        let rgb = rgba_frame(&[1, 2, 3], 0, 3, 3, 1, 1, PixelFormat::Rgb).unwrap();
        assert_eq!(rgb.rgba, [1, 2, 3, 255]);
        for format in [PixelFormat::Rgba, PixelFormat::Rgbx, PixelFormat::Bgra] {
            assert_eq!(
                rgba_frame(&[2, 2, 2, 0], 0, 4, 4, 1, 1, format)
                    .unwrap()
                    .rgba,
                [2, 2, 2, 255]
            );
        }
    }
    #[test]
    fn invalid_layout_is_rejected_before_allocation() {
        for (offset, chunk, stride, width, height) in [
            (0, 3, 4, 1, 1),
            (1, 4, 4, 1, 1),
            (0, 4, -4, 1, 1),
            (0, 4, 3, 1, 1),
            (0, 4, 4, 0, 1),
            (0, 4, 4, u32::MAX, 2),
            (usize::MAX, 4, 4, 1, 1),
            (0, 4, 32768, 8192, 8192),
        ] {
            assert!(rgba_frame(
                &[0; 4],
                offset,
                chunk,
                stride,
                width,
                height,
                PixelFormat::Rgba
            )
            .is_err());
        }
    }
    #[test]
    fn selected_stream_uses_logical_not_pixel_coordinates() {
        assert_eq!(logical_point(1.0, 1.0, (960, 540)).unwrap(), (959.0, 539.0));
        assert_eq!(logical_point(-1.0, 2.0, (960, 540)).unwrap(), (0.0, 539.0));
        assert!(logical_point(f32::NAN, 0.0, (960, 540)).is_err());
        assert!(logical_point(0.0, 0.0, (0, 540)).is_err());
    }
    #[test]
    fn latest_frame_overwrites_and_close_discards_all_frames() {
        let mut slot = FrameSlot::default();
        for i in 0..1000 {
            slot.publish(
                Goruntu {
                    genislik: 1,
                    yukseklik: 1,
                    rgba: vec![i as u8; 4],
                },
                i,
            );
        }
        assert_eq!(slot.sequence, 1000);
        assert_eq!(slot.frame.as_ref().unwrap().1, 999);
        slot.close();
        slot.publish(
            Goruntu {
                genislik: 1,
                yukseklik: 1,
                rgba: vec![0; 4],
            },
            1001,
        );
        assert!(slot.closed && slot.frame.is_none());
        assert_eq!(slot.sequence, 1000);
    }
}
