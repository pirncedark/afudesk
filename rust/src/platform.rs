//! Platforma bağlı parçalar: ekran yakalama, girdi enjeksiyonu ve pano.
//! Testler sahte uygulamaları kullanır; gerçek uygulamalar masaüstünde xcap + enigo + arboard.
use crate::{goruntu::Goruntu, pano::Pano, protokol::Girdi};
use anyhow::Result;

/// Send değil: bazı platform tutamaçları iş parçacığına bağlıdır; yakalayıcı
/// kullanılacağı iş parçacığında oluşturulur.
pub trait Yakalayici {
    fn uyandirici_ayarla(
        &mut self,
        _bildirim: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    ) {
    }
    fn alan_ayarla(&mut self, _alan: Option<crate::terminal::Alan>) {}
    fn degisim_gudumlu(&self) -> bool {
        false
    }
    fn kirpilmis(&self) -> bool {
        false
    }
    fn yakala(&mut self) -> Result<Goruntu>;

    /// Kareyle aynı ana ait host Unix zamanı (ms); yakalayıcılar bunu yakalama anında örtebilir.
    fn yakala_zamanli(&mut self) -> Result<(Goruntu, u64)> {
        let goruntu = self.yakala()?;
        let yakalama_ms = unix_ms();
        Ok((goruntu, yakalama_ms))
    }
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(not(windows))]
pub fn imlec() -> Option<crate::protokol::Imlec> {
    None
}

#[cfg(windows)]
pub fn imlec() -> Option<crate::protokol::Imlec> {
    windows_imlec::oku()
}

/// Start each new viewer with a complete cursor shape, even on a reused worker thread.
pub fn imlec_onbellegini_sifirla() {
    #[cfg(windows)]
    windows_imlec::sifirla();
}

#[cfg(windows)]
mod windows_imlec {
    use crate::protokol::Imlec;
    use std::{
        cell::RefCell,
        mem::{size_of, zeroed},
        ptr::null_mut,
    };
    use windows_sys::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::*};

    // Each capture/session thread owns its cache: a new viewer always gets its first shape.
    thread_local! { static SON: RefCell<Option<(usize, Imlec)>> = const { RefCell::new(None) }; }
    pub fn sifirla() {
        SON.with(|son| *son.borrow_mut() = None);
    }

    struct BitEslemler(ICONINFO);
    impl Drop for BitEslemler {
        fn drop(&mut self) {
            unsafe {
                if !self.0.hbmColor.is_null() {
                    DeleteObject(self.0.hbmColor);
                }
                if !self.0.hbmMask.is_null() {
                    DeleteObject(self.0.hbmMask);
                }
            }
        }
    }
    struct EkranDc(HDC);
    impl Drop for EkranDc {
        fn drop(&mut self) {
            unsafe {
                ReleaseDC(null_mut(), self.0);
            }
        }
    }

    fn bitmap(dc: HDC, kaynak: HBITMAP, en: u32, boy: u32) -> Option<Vec<u8>> {
        let mut bilgi: BITMAPINFO = unsafe { zeroed() };
        bilgi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bilgi.bmiHeader.biWidth = en as i32;
        bilgi.bmiHeader.biHeight = -(boy as i32);
        bilgi.bmiHeader.biPlanes = 1;
        bilgi.bmiHeader.biBitCount = 32;
        bilgi.bmiHeader.biCompression = BI_RGB;
        let mut veri = vec![0; en as usize * boy as usize * 4];
        (unsafe {
            GetDIBits(
                dc,
                kaynak,
                0,
                boy,
                veri.as_mut_ptr().cast(),
                &mut bilgi,
                DIB_RGB_COLORS,
            )
        } == boy as i32)
            .then_some(veri)
    }

    pub(super) fn rgba_yap(renk: &mut [u8], maske: &[u8]) {
        let alfa_var = renk.chunks_exact(4).any(|p| p[3] != 0);
        for (p, m) in renk.chunks_exact_mut(4).zip(maske.chunks_exact(4)) {
            p.swap(0, 2);
            if !alfa_var {
                p[3] = if m[0] == 0 { 255 } else { 0 };
            } else if p[3] > 0 && p[3] < 255 {
                let a = p[3] as u32;
                for kanal in &mut p[..3] {
                    *kanal = ((*kanal as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
    }

    fn sekil(handle: HCURSOR) -> Option<Imlec> {
        let mut bilgi: ICONINFO = unsafe { zeroed() };
        if unsafe { GetIconInfo(handle, &mut bilgi) } == 0 {
            return None;
        }
        let bilgi = BitEslemler(bilgi);
        let renkli = !bilgi.0.hbmColor.is_null();
        let kaynak = if renkli {
            bilgi.0.hbmColor
        } else {
            bilgi.0.hbmMask
        };
        let mut bm: BITMAP = unsafe { zeroed() };
        if unsafe {
            GetObjectW(
                kaynak,
                size_of::<BITMAP>() as i32,
                (&mut bm as *mut BITMAP).cast(),
            )
        } == 0
        {
            return None;
        }
        let en = u32::try_from(bm.bmWidth).ok()?;
        let boy = u32::try_from(bm.bmHeight).ok()? / if renkli { 1 } else { 2 };
        // Bound allocations for malformed/custom cursor bitmaps.
        if en == 0 || boy == 0 || en > 1024 || boy > 1024 {
            return None;
        }
        let dc = EkranDc(unsafe { GetDC(null_mut()) });
        if dc.0.is_null() {
            return None;
        }
        let mut rgba;
        if renkli {
            rgba = bitmap(dc.0, kaynak, en, boy)?;
            let maske = bitmap(dc.0, bilgi.0.hbmMask, en, boy)?;
            rgba_yap(&mut rgba, &maske);
        } else {
            let maske = bitmap(dc.0, kaynak, en, boy * 2)?;
            let (and, xor) = maske.split_at(en as usize * boy as usize * 4);
            rgba = xor.to_vec();
            for p in rgba.chunks_exact_mut(4) {
                p[3] = 0;
            }
            rgba_yap(&mut rgba, and);
            // XOR/inverted pixels have no exact alpha equivalent; show a visible white pixel.
            for ((p, a), x) in rgba
                .chunks_exact_mut(4)
                .zip(and.chunks_exact(4))
                .zip(xor.chunks_exact(4))
            {
                if a[0] != 0 && x[0] != 0 {
                    p.copy_from_slice(&[255; 4]);
                }
            }
        }
        Some(Imlec {
            x: 0.0,
            y: 0.0,
            gorunur: true,
            gen: en,
            yuk: boy,
            sicak_x: bilgi.0.xHotspot,
            sicak_y: bilgi.0.yHotspot,
            rgba,
        })
    }

    pub fn oku() -> Option<Imlec> {
        let mut bilgi: CURSORINFO = unsafe { zeroed() };
        bilgi.cbSize = size_of::<CURSORINFO>() as u32;
        if unsafe { GetCursorInfo(&mut bilgi) } == 0 {
            return None;
        }
        let gorunur = bilgi.flags & CURSOR_SHOWING != 0;
        let en = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(2);
        let boy = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(2);
        SON.with(|son| {
            let mut son = son.borrow_mut();
            let mut sonuc = Imlec {
                x: bilgi.ptScreenPos.x as f32 / (en - 1) as f32,
                y: bilgi.ptScreenPos.y as f32 / (boy - 1) as f32,
                gorunur,
                gen: 0,
                yuk: 0,
                sicak_x: 0,
                sicak_y: 0,
                rgba: Vec::new(),
            };
            let handle = bilgi.hCursor as usize;
            if gorunur && son.as_ref().map(|s| s.0) != Some(handle) {
                let mut yeni = sekil(bilgi.hCursor)?;
                yeni.x = sonuc.x;
                yeni.y = sonuc.y;
                sonuc = yeni.clone();
                *son = Some((handle, yeni));
            }
            Some(sonuc)
        })
    }
}

#[cfg(all(test, windows))]
mod imlec_testleri {
    #[test]
    fn renk_ve_maske_alfa_donusumu() {
        let mut renk = vec![10, 20, 30, 0, 0, 0, 0, 0];
        super::windows_imlec::rgba_yap(&mut renk, &[0, 0, 0, 0, 255, 255, 255, 0]);
        assert_eq!(renk, [30, 20, 10, 255, 0, 0, 0, 0]);
        let mut alfa = vec![25, 50, 100, 128];
        super::windows_imlec::rgba_yap(&mut alfa, &[255; 4]);
        assert_eq!(alfa, [199, 100, 50, 128]);
    }
}

pub trait Enjektor: Send {
    fn uygula(&mut self, g: &Girdi) -> Result<()>;
}

/// Her oturum için yeni yakalayıcı/enjektör üretir (ikisi de iş parçacığına bağlı olabilir).
pub trait Fabrika: Send + Sync {
    /// Sahte/headless fabrikalar hiçbir gerçek pencereye erişmez.
    fn terminal_destegi(&self) -> bool {
        false
    }
    fn yakalayici(&self) -> Result<Box<dyn Yakalayici>>;
    fn enjektor(&self) -> Result<Box<dyn Enjektor>>;
    fn pano(&self) -> Result<Box<dyn Pano>>;
    fn kol_surucusu(&self) -> Result<Box<dyn crate::sanal_kol::KolSurucusu>>;
}

#[cfg(not(target_os = "android"))]
pub mod masaustu {
    use super::*;
    use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

    pub struct Gercek;

    impl Fabrika for Gercek {
        fn terminal_destegi(&self) -> bool {
            cfg!(windows)
        }
        fn yakalayici(&self) -> Result<Box<dyn Yakalayici>> {
            #[cfg(windows)]
            if let Ok(mut yakalayici) = WgcYakalayici::new() {
                if yakalayici.yakala().is_ok() {
                    return Ok(Box::new(yakalayici));
                }
            }
            Ok(Box::new(XcapYakalayici::new()?))
        }
        fn enjektor(&self) -> Result<Box<dyn Enjektor>> {
            Ok(Box::new(EnigoEnjektor::new()?))
        }
        fn pano(&self) -> Result<Box<dyn Pano>> {
            Ok(Box::new(crate::pano::ArboardPano::new()?))
        }
        fn kol_surucusu(&self) -> Result<Box<dyn crate::sanal_kol::KolSurucusu>> {
            Ok(Box::new(crate::sanal_kol::ViGEmSurucusu::yeni()?))
        }
    }

    #[cfg(windows)]
    pub struct WgcYakalayici {
        kare: std::sync::Arc<(std::sync::Mutex<Option<(Goruntu, u64)>>, std::sync::Condvar)>,
        ayarlar: std::sync::Arc<std::sync::Mutex<WgcPaylasim>>,
        son: Option<(Goruntu, u64)>,
        alan_degisti: bool,
        _kontrol: windows_capture::capture::CaptureControl<WgcIsleyici, WgcHata>,
    }

    #[cfg(windows)]
    type WgcHata = Box<dyn std::error::Error + Send + Sync>;

    #[cfg(windows)]
    #[derive(Default)]
    struct WgcPaylasim {
        alan: Option<crate::terminal::Alan>,
        nesil: u64,
        bildirim: Option<std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>>,
    }

    #[cfg(windows)]
    struct WgcIsleyici {
        ayarlar: std::sync::Arc<std::sync::Mutex<WgcPaylasim>>,
        onceki: Option<Goruntu>,
        onceki_alan: Option<crate::terminal::Alan>,
        kare: std::sync::Arc<(std::sync::Mutex<Option<(Goruntu, u64)>>, std::sync::Condvar)>,
    }

    #[cfg(windows)]
    impl windows_capture::capture::GraphicsCaptureApiHandler for WgcIsleyici {
        type Flags = (
            std::sync::Arc<(std::sync::Mutex<Option<(Goruntu, u64)>>, std::sync::Condvar)>,
            std::sync::Arc<std::sync::Mutex<WgcPaylasim>>,
        );
        type Error = WgcHata;

        fn new(
            ctx: windows_capture::capture::Context<Self::Flags>,
        ) -> std::result::Result<Self, Self::Error> {
            Ok(Self {
                kare: ctx.flags.0,
                ayarlar: ctx.flags.1,
                onceki: None,
                onceki_alan: None,
            })
        }

        fn on_frame_arrived(
            &mut self,
            frame: &mut windows_capture::frame::Frame,
            _kontrol: windows_capture::graphics_capture_api::InternalCaptureControl,
        ) -> std::result::Result<(), Self::Error> {
            let ayarlar = self.ayarlar.lock().unwrap();
            let alan = ayarlar.alan;
            let nesil = ayarlar.nesil;
            let bildirim = ayarlar.bildirim.clone();
            drop(ayarlar);
            let tam = crate::terminal::Alan {
                x: 0,
                y: 0,
                en: frame.width(),
                boy: frame.height(),
            };
            let hedef =
                crate::terminal::kesisim(alan.unwrap_or(tam), frame.width(), frame.height())?;
            let yenile = self.onceki.as_ref().map(|g| (g.genislik, g.yukseklik))
                != Some((hedef.en, hedef.boy))
                || self.onceki_alan != Some(hedef);
            let bolgeler = if yenile {
                vec![hedef]
            } else {
                match frame.dirty_regions() {
                    Ok(bolgeler) => bolgeler
                        .into_iter()
                        .filter_map(|r| {
                            let x = r.x.max(hedef.x);
                            let y = r.y.max(hedef.y);
                            let sag =
                                (r.x as i64 + r.width as i64).min(hedef.x as i64 + hedef.en as i64);
                            let alt = (r.y as i64 + r.height as i64)
                                .min(hedef.y as i64 + hedef.boy as i64);
                            (sag > x as i64 && alt > y as i64).then_some(crate::terminal::Alan {
                                x,
                                y,
                                en: (sag - x as i64) as u32,
                                boy: (alt - y as i64) as u32,
                            })
                        })
                        .collect(),
                    // Older Windows has no DirtyRegions API: copy only the requested crop.
                    Err(_) => vec![hedef],
                }
            };
            if bolgeler.is_empty() {
                return Ok(());
            }
            if yenile {
                self.onceki = Some(Goruntu {
                    genislik: hedef.en,
                    yukseklik: hedef.boy,
                    rgba: vec![0; hedef.en as usize * hedef.boy as usize * 4],
                });
            }
            let goruntu = self.onceki.as_mut().unwrap();
            let mut duz = Vec::new();
            for b in bolgeler {
                let tampon = frame.buffer_crop(
                    b.x as u32,
                    b.y as u32,
                    b.x as u32 + b.en,
                    b.y as u32 + b.boy,
                )?;
                let piksel = tampon.as_nopadding_buffer(&mut duz);
                let satir = b.en as usize * 4;
                for y in 0..b.boy as usize {
                    let bas = ((b.y - hedef.y) as usize + y) * hedef.en as usize * 4
                        + (b.x - hedef.x) as usize * 4;
                    goruntu.rgba[bas..bas + satir]
                        .copy_from_slice(&piksel[y * satir..(y + 1) * satir]);
                }
            }
            self.onceki_alan = Some(hedef);
            // Area setters and publishers use the same lock order. An old-area callback
            // must never publish into the newly selected area's queue.
            let ayarlar = self.ayarlar.lock().unwrap();
            if ayarlar.nesil != nesil {
                return Ok(());
            }
            let (kilit, kosul) = &*self.kare;
            *kilit.lock().unwrap() = Some((goruntu.clone(), unix_ms()));
            drop(ayarlar);
            kosul.notify_all();
            if let Some(bildirim) = bildirim {
                *bildirim.0.lock().unwrap() = true;
                bildirim.1.notify_all();
            }
            Ok(())
        }
    }

    #[cfg(windows)]
    impl WgcYakalayici {
        pub fn new() -> Result<Self> {
            use windows_capture::{
                capture::GraphicsCaptureApiHandler,
                monitor::Monitor,
                settings::{
                    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
                    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
                },
            };
            let kare =
                std::sync::Arc::new((std::sync::Mutex::new(None), std::sync::Condvar::new()));
            let paylasim = std::sync::Arc::new(std::sync::Mutex::new(WgcPaylasim::default()));
            let kirli = windows_capture::graphics_capture_api::GraphicsCaptureApi::is_dirty_region_supported().unwrap_or(false);
            let ayarlar = Settings::new(
                Monitor::primary()?,
                CursorCaptureSettings::WithoutCursor,
                DrawBorderSettings::Default,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                if kirli {
                    DirtyRegionSettings::ReportOnly
                } else {
                    DirtyRegionSettings::Default
                },
                ColorFormat::Rgba8,
                (kare.clone(), paylasim.clone()),
            );
            let kontrol = WgcIsleyici::start_free_threaded(ayarlar)?;
            Ok(Self {
                kare,
                ayarlar: paylasim,
                son: None,
                alan_degisti: false,
                _kontrol: kontrol,
            })
        }
    }

    #[cfg(windows)]
    impl Yakalayici for WgcYakalayici {
        fn uyandirici_ayarla(
            &mut self,
            bildirim: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
        ) {
            self.ayarlar.lock().unwrap().bildirim = Some(bildirim);
        }
        fn alan_ayarla(&mut self, alan: Option<crate::terminal::Alan>) {
            let mut ayarlar = self.ayarlar.lock().unwrap();
            if ayarlar.alan != alan {
                ayarlar.alan = alan;
                ayarlar.nesil = ayarlar.nesil.wrapping_add(1);
                *self.kare.0.lock().unwrap() = None;
                self.son = None;
                self.alan_degisti = true;
            }
        }
        fn degisim_gudumlu(&self) -> bool {
            true
        }
        fn kirpilmis(&self) -> bool {
            self.ayarlar.lock().unwrap().alan.is_some()
        }

        fn yakala(&mut self) -> Result<Goruntu> {
            self.yakala_zamanli().map(|(goruntu, _)| goruntu)
        }

        fn yakala_zamanli(&mut self) -> Result<(Goruntu, u64)> {
            let (kilit, kosul) = &*self.kare;
            let mut kare = kilit.lock().unwrap();
            if kare.is_none() && self.alan_degisti {
                // A static desktop need not emit another WGC frame after a crop change.
                // Seed that crop once; later WGC events replace it with the dirty-region buffer.
                drop(kare);
                let mut ekran = XcapYakalayici::new()?;
                let g = ekran.yakala()?;
                let alan = self.ayarlar.lock().unwrap().alan;
                let g = match alan {
                    Some(a) => crate::terminal::kirp(&g, a)?,
                    None => g,
                };
                self.son = Some((g, unix_ms()));
                kare = kilit.lock().unwrap();
            }
            self.alan_degisti = false;
            if kare.is_none() && self.son.is_none() {
                kare = kosul
                    .wait_timeout_while(kare, std::time::Duration::from_millis(500), |v| {
                        v.is_none()
                    })
                    .unwrap()
                    .0;
            }
            if let Some(yeni) = kare.take() {
                self.son = Some(yeni);
            }
            self.son
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Ekran alınamadı. Yeniden bağlan."))
        }
    }

    pub struct XcapYakalayici {
        ekran: xcap::Monitor,
    }

    impl XcapYakalayici {
        pub fn new() -> Result<Self> {
            let ekranlar = xcap::Monitor::all()?;
            let birincil = ekranlar
                .iter()
                .find(|m| m.is_primary().unwrap_or(false))
                .cloned()
                .or_else(|| ekranlar.first().cloned())
                .ok_or_else(|| anyhow::anyhow!("Ekran bulunamadı."))?;
            Ok(Self { ekran: birincil })
        }
    }

    impl Yakalayici for XcapYakalayici {
        fn yakala(&mut self) -> Result<Goruntu> {
            let r = self.ekran.capture_image()?;
            Ok(Goruntu {
                genislik: r.width(),
                yukseklik: r.height(),
                rgba: r.into_raw(),
            })
        }
    }

    pub struct EnigoEnjektor {
        e: Enigo,
        boyut: (i32, i32),
        #[cfg(windows)]
        numpad_enter_basili: bool,
    }

    impl EnigoEnjektor {
        pub fn new() -> Result<Self> {
            let e = Enigo::new(&Settings::default())?;
            let boyut = e.main_display()?;
            Ok(Self {
                e,
                boyut,
                #[cfg(windows)]
                numpad_enter_basili: false,
            })
        }
    }

    /// USB HID usage -> Windows VK; ad yalnız yedektir.
    pub fn hid_vk(hid: u32) -> Option<u16> {
        let usage = hid & 0xffff;
        if hid >> 16 == 0x0c {
            return Some(match usage {
                0xb5 => 0xb0,
                0xb6 => 0xb1,
                0xb7 => 0xb2,
                0xcd => 0xb3,
                0xe2 => 0xad,
                0xe9 => 0xaf,
                0xea => 0xae,
                _ => return None,
            });
        }
        if hid >> 16 != 0x07 {
            return None;
        }
        Some(match usage {
            0x04..=0x1d => (0x41 + usage - 0x04) as u16,
            0x1e..=0x26 => (0x31 + usage - 0x1e) as u16,
            0x3a..=0x45 => (0x70 + usage - 0x3a) as u16,
            0x59..=0x61 => (0x61 + usage - 0x59) as u16,
            0x68..=0x73 => (0x7c + usage - 0x68) as u16,
            0x27 => 0x30,
            0x28 => 0x0d,
            0x29 => 0x1b,
            0x2a => 0x08,
            0x2b => 0x09,
            0x2c => 0x20,
            0x2d => 0xbd,
            0x2e => 0xbb,
            0x2f => 0xdb,
            0x30 => 0xdd,
            0x31 => 0xdc,
            0x32 => 0xdc,
            0x33 => 0xba,
            0x34 => 0xde,
            0x35 => 0xc0,
            0x36 => 0xbc,
            0x37 => 0xbe,
            0x38 => 0xbf,
            0x39 => 0x14,
            0x46 => 0x2c,
            0x47 => 0x91,
            0x48 => 0x13,
            0x49 => 0x2d,
            0x4a => 0x24,
            0x4b => 0x21,
            0x4c => 0x2e,
            0x4d => 0x23,
            0x4e => 0x22,
            0x4f => 0x27,
            0x50 => 0x25,
            0x51 => 0x28,
            0x52 => 0x26,
            0x53 => 0x90,
            0x54 => 0x6f,
            0x55 => 0x6a,
            0x56 => 0x6d,
            0x57 => 0x6b,
            0x58 => 0x0d,
            0x62 => 0x60,
            0x63 => 0x6e,
            0x64 => 0xe2,
            0x65 => 0x5d,
            0x7f => 0xad,
            0x80 => 0xaf,
            0x81 => 0xae,
            0x85 => 0x6c,
            0xe0 => 0xa2,
            0xe1 => 0xa0,
            0xe2 => 0xa4,
            0xe3 => 0x5b,
            0xe4 => 0xa3,
            0xe5 => 0xa1,
            0xe6 => 0xa5,
            0xe7 => 0x5c,
            _ => return None,
        })
    }

    // Enigo 0.6.1 numpad Enter için EXTENDEDKEY koymuyor; VK_RETURN
    // ana Enter ile aynı olduğundan bu kullanım ayrı SendInput yolundan geçer.
    #[cfg(windows)]
    mod numpad_enter {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct KeyboardInput {
            vk: u16,
            scan: u16,
            flags: u32,
            time: u32,
            extra: usize,
        }
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct MouseInput {
            x: i32,
            y: i32,
            data: u32,
            flags: u32,
            time: u32,
            extra: usize,
        }
        #[repr(C)]
        union InputData {
            keyboard: KeyboardInput,
            mouse: MouseInput,
        }
        #[repr(C)]
        struct Input {
            kind: u32,
            data: InputData,
        }
        #[link(name = "user32")]
        extern "system" {
            fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
        }

        fn unicode_inputs(metin: &str) -> Vec<Input> {
            metin
                .encode_utf16()
                .flat_map(|scan| {
                    [0x0004, 0x0006].map(|flags| {
                        let mut input: Input = unsafe { std::mem::zeroed() };
                        input.kind = 1;
                        input.data.keyboard = KeyboardInput {
                            vk: 0,
                            scan,
                            flags,
                            time: 0,
                            extra: 0,
                        };
                        input
                    })
                })
                .collect()
        }
        pub fn metin(metin: &str) -> anyhow::Result<()> {
            let inputs = unicode_inputs(metin);
            if inputs.is_empty() {
                return Ok(());
            }
            let sent = unsafe {
                SendInput(
                    inputs.len() as u32,
                    inputs.as_ptr(),
                    std::mem::size_of::<Input>() as i32,
                )
            };
            anyhow::ensure!(
                sent as usize == inputs.len(),
                "Metin gonderilemedi. Yeniden dene."
            );
            Ok(())
        }

        pub fn send(pressed: bool) -> anyhow::Result<()> {
            // Sıfırla: INPUT union padding baytları da başlatılır.
            let mut input: Input = unsafe { std::mem::zeroed() };
            input.kind = 1;
            input.data.keyboard = KeyboardInput {
                vk: 0x0d,
                scan: 0x1c,
                flags: 0x0001 | if pressed { 0 } else { 0x0002 },
                time: 0,
                extra: 0,
            };
            // repr(C) Win32 INPUT ABI ile aynı; input çağrı boyunca yaşar.
            let sent = unsafe { SendInput(1, &input, std::mem::size_of::<Input>() as i32) };
            anyhow::ensure!(
                sent == 1,
                "Numpad Enter gönderilemedi: {}",
                std::io::Error::last_os_error()
            );
            Ok(())
        }
        #[cfg(test)]
        #[test]
        fn unicode_paketleri_surrogate_ve_tus_birakma() {
            let paketler = unicode_inputs("\u{015f}\u{1f600}");
            assert_eq!(paketler.len(), 6);
            let beklenen = [0x015f, 0xd83d, 0xde00];
            for (cift, birim) in paketler.chunks_exact(2).zip(beklenen) {
                unsafe {
                    assert_eq!(cift[0].data.keyboard.scan, birim);
                    assert_eq!(cift[0].data.keyboard.vk, 0);
                    assert_eq!(cift[0].data.keyboard.flags, 4);
                    assert_eq!(cift[1].data.keyboard.flags, 6);
                }
            }
            assert!(unicode_inputs("").is_empty());
        }
        #[cfg(test)]
        #[test]
        fn input_abi_boyutu() {
            assert_eq!(
                std::mem::size_of::<Input>(),
                if cfg!(target_pointer_width = "64") {
                    40
                } else {
                    28
                }
            );
        }
    }

    #[cfg(windows)]
    impl Drop for EnigoEnjektor {
        fn drop(&mut self) {
            if self.numpad_enter_basili {
                let _ = numpad_enter::send(false);
            }
        }
    }

    pub fn tus_coz(ad: &str) -> Option<Key> {
        Some(match ad {
            "Enter" | "NumpadEnter" => Key::Return,
            "Backspace" => Key::Backspace,
            "Tab" => Key::Tab,
            "Escape" => Key::Escape,
            "Space" | " " => Key::Space,
            "Delete" => Key::Delete,
            "Home" => Key::Home,
            "End" => Key::End,
            "PageUp" => Key::PageUp,
            "PageDown" => Key::PageDown,
            "ArrowUp" => Key::UpArrow,
            "ArrowDown" => Key::DownArrow,
            "ArrowLeft" => Key::LeftArrow,
            "ArrowRight" => Key::RightArrow,
            "Shift" => Key::Shift,
            "Ctrl" | "Control" => Key::Control,
            "Alt" => Key::Alt,
            "Meta" | "Win" => Key::Meta,
            "CapsLock" => Key::CapsLock,
            "F1" => Key::F1,
            "F2" => Key::F2,
            "F3" => Key::F3,
            "F4" => Key::F4,
            "F5" => Key::F5,
            "F6" => Key::F6,
            "F7" => Key::F7,
            "F8" => Key::F8,
            "F9" => Key::F9,
            "F10" => Key::F10,
            "F11" => Key::F11,
            "F12" => Key::F12,
            "Insert" => Key::Insert,
            #[cfg(not(target_os = "macos"))]
            "PrintScreen" => Key::PrintScr,
            #[cfg(windows)]
            "ScrollLock" => Key::Scroll,
            #[cfg(all(unix, not(target_os = "macos")))]
            "ScrollLock" => Key::ScrollLock,
            #[cfg(not(target_os = "macos"))]
            "Pause" => Key::Pause,
            "VolumeMute" => Key::VolumeMute,
            "VolumeDown" => Key::VolumeDown,
            "VolumeUp" => Key::VolumeUp,
            "MediaPlayPause" => Key::MediaPlayPause,
            "MediaNextTrack" => Key::MediaNextTrack,
            "MediaPrevTrack" => Key::MediaPrevTrack,
            #[cfg(not(target_os = "macos"))]
            "MediaStop" => Key::MediaStop,
            "F13" => Key::F13,
            "F14" => Key::F14,
            "F15" => Key::F15,
            "F16" => Key::F16,
            "F17" => Key::F17,
            "F18" => Key::F18,
            "F19" => Key::F19,
            "F20" => Key::F20,
            "F21" => Key::F21,
            "F22" => Key::F22,
            "F23" => Key::F23,
            "F24" => Key::F24,

            _ => {
                let mut c = ad.chars();
                let ilk = c.next()?;
                if c.next().is_some() {
                    return None;
                }
                Key::Unicode(ilk.to_lowercase().next()?)
            }
        })
    }

    impl Enjektor for EnigoEnjektor {
        fn uygula(&mut self, g: &Girdi) -> Result<()> {
            match g {
                Girdi::FareKonum { x, y } => {
                    let x = (x.clamp(0.0, 1.0) * (self.boyut.0 - 1) as f32).round() as i32;
                    let y = (y.clamp(0.0, 1.0) * (self.boyut.1 - 1) as f32).round() as i32;
                    self.e.move_mouse(x, y, Coordinate::Abs)?;
                }
                Girdi::FareTus { tus, basili } => {
                    let b = match tus {
                        crate::protokol::FareTusu::Sol => Button::Left,
                        crate::protokol::FareTusu::Sag => Button::Right,
                        crate::protokol::FareTusu::Orta => Button::Middle,
                    };
                    self.e.button(
                        b,
                        if *basili {
                            Direction::Press
                        } else {
                            Direction::Release
                        },
                    )?;
                }
                Girdi::Kaydir { dx, dy } => {
                    if *dy != 0 {
                        self.e.scroll(*dy, Axis::Vertical)?;
                    }
                    if *dx != 0 {
                        self.e.scroll(*dx, Axis::Horizontal)?;
                    }
                }
                Girdi::Tus { hid, ad, basili } => {
                    #[cfg(windows)]
                    if *hid == 0x00070058 || (*hid == 0 && ad == "NumpadEnter") {
                        numpad_enter::send(*basili)?;
                        self.numpad_enter_basili = *basili;
                        return Ok(());
                    }
                    #[cfg(windows)]
                    let key = hid_vk(*hid)
                        .map(|vk| Key::Other(vk as u32))
                        .or_else(|| tus_coz(ad));
                    #[cfg(not(windows))]
                    let key = {
                        let _ = hid;
                        tus_coz(ad)
                    };
                    if let Some(k) = key {
                        self.e.key(
                            k,
                            if *basili {
                                Direction::Press
                            } else {
                                Direction::Release
                            },
                        )?;
                    }
                }
                Girdi::Metin(m) => {
                    if !m.is_empty() {
                        #[cfg(windows)]
                        numpad_enter::metin(m)?;
                        #[cfg(not(windows))]
                        self.e.text(m)?;
                    }
                }
                Girdi::Kol(_) => {}
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod testler {
        use super::*;

        #[test]
        fn tus_adlari() {
            assert_eq!(tus_coz("Enter"), Some(Key::Return));
            assert_eq!(tus_coz("ş"), Some(Key::Unicode('ş')));
            assert_eq!(tus_coz("A"), Some(Key::Unicode('a')));
            assert_eq!(tus_coz("Ş"), Some(Key::Unicode('ş')));
            assert_eq!(tus_coz("F24"), Some(Key::F24));
            #[cfg(not(target_os = "macos"))]
            assert_eq!(tus_coz("PrintScreen"), Some(Key::PrintScr));
            assert_eq!(tus_coz("ArrowLeft"), Some(Key::LeftArrow));
            assert_eq!(tus_coz("BilinmeyenTus"), None);
            assert_eq!(tus_coz(""), None);
        }

        #[test]
        fn fiziksel_hid_vk() {
            for (hid, vk) in [
                (0x70004, 0x41),
                (0x7001e, 0x31),
                (0x70027, 0x30),
                (0x70062, 0x60),
                (0x70057, 0x6b),
                (0x70068, 0x7c),
                (0x70073, 0x87),
                (0x70046, 0x2c),
                (0x700e4, 0xa3),
                (0x700e6, 0xa5),
                (0x70058, 0x0d),
                (0x70054, 0x6f),
                // Türkçe Q: ş, ğ, ü, ö, ç OEM; ı fiziksel I konumunda.
                (0x70033, 0xba),
                (0x7002f, 0xdb),
                (0x70030, 0xdd),
                (0x70036, 0xbc),
                (0x70037, 0xbe),
                (0x7000c, 0x49),
                (0xc00cd, 0xb3),
                (0xc00e9, 0xaf),
            ] {
                assert_eq!(hid_vk(hid), Some(vk), "HID {hid:x}");
            }
            assert_eq!(hid_vk(0), None);
            assert_eq!(hid_vk(0x80004), None);
            assert_eq!(hid_vk(0x7ffff), None);
        }

        /// Gerçek donanım: bu makinenin birincil ekranı yakalanabilmeli ve
        /// ardışık iki yakalama aynı boyutta olmalı.
        #[test]
        fn gercek_ekran_yakalanir() {
            let mut y = XcapYakalayici::new().expect("ekran bulunamadı");
            let a = y.yakala().expect("yakalama başarısız");
            assert!(
                a.genislik >= 640 && a.yukseklik >= 480,
                "{}x{}",
                a.genislik,
                a.yukseklik
            );
            assert_eq!(a.rgba.len(), (a.genislik * a.yukseklik * 4) as usize);
            let b = y.yakala().unwrap();
            assert_eq!((a.genislik, a.yukseklik), (b.genislik, b.yukseklik));
            // Tamamen siyah değil (yakalama gerçekten piksel döndürüyor).
            assert!(a
                .rgba
                .chunks_exact(4)
                .any(|p| p[0] > 10 || p[1] > 10 || p[2] > 10));
            // Gerçek ekran + kodlayıcı: 1 tam kare üretilebilmeli, süresi ölçülür.
            let t0 = std::time::Instant::now();
            let yayin = crate::goruntu::yayin_boyutu(a.clone());
            let k = crate::goruntu::Kodlayici::new(70)
                .kodla(&yayin)
                .unwrap()
                .unwrap();
            let sure = t0.elapsed();
            let bayt: usize = k.dosemeler.iter().map(|d| d.jpeg.len()).sum();
            eprintln!(
                "ekran {}x{} -> yayın {}x{}, tam kare {} döşeme, {} KB, {:?}",
                a.genislik,
                a.yukseklik,
                yayin.genislik,
                yayin.yukseklik,
                k.dosemeler.len(),
                bayt / 1024,
                sure
            );
        }

        #[cfg(windows)]
        #[test]
        fn gercek_wgc_yakalanir() {
            let mut wgc = WgcYakalayici::new().expect("WGC başlatılamadı");
            let mut xcap = XcapYakalayici::new().expect("xcap ekranı bulamadı");
            // Diğer donanım testleriyle eşzamanlı çalışırken WGC ilk sunumu gecikebilir.
            let son_sure = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                match wgc.yakala() {
                    Ok(g) => {
                        assert!(g.genislik > 0 && g.yukseklik > 0);
                        break;
                    }
                    Err(e) if std::time::Instant::now() < son_sure => {
                        let _ = e;
                    }
                    Err(e) => panic!("WGC ilk kareyi 15 saniyede vermedi: {e}"),
                }
            }
            let basla = std::time::Instant::now();
            for _ in 0..30 {
                let g = wgc.yakala().expect("WGC kare yakalayamadı");
                assert!(g.genislik > 0 && g.yukseklik > 0);
            }
            let wgc_ms = basla.elapsed().as_secs_f64() * 1000.0 / 30.0;
            let basla = std::time::Instant::now();
            for _ in 0..30 {
                let g = xcap.yakala().expect("xcap kare yakalayamadı");
                assert!(g.genislik > 0 && g.yukseklik > 0);
            }
            let xcap_ms = basla.elapsed().as_secs_f64() * 1000.0 / 30.0;
            eprintln!("WGC ortalama: {wgc_ms:.2} ms/kare; xcap ortalama: {xcap_ms:.2} ms/kare");
        }

        #[test]
        fn gercek_enjektor_acilir() {
            let e = EnigoEnjektor::new().expect("enigo açılamadı");
            assert!(e.boyut.0 > 0 && e.boyut.1 > 0);
        }
    }
}

/// Testler için: renk değiştiren sahte ekran, girdileri kaydeden enjektör ve bellekte pano.
pub mod sahte {
    use super::*;
    use crate::pano::SahtePano;
    use std::sync::{Arc, Mutex};

    pub struct SahteFabrika {
        pub girdiler: Arc<Mutex<Vec<Girdi>>>,
        pub boyut: (u32, u32),
        /// Host panosu; test içeriği değiştirip yazılanları okuyabilir (klonlar paylaşır).
        pub pano: SahtePano,
        pub kol_durumlari: Arc<Mutex<Vec<crate::protokol::KolDurumu>>>,
        pub kol_titresim: crate::sanal_kol::TitresimKuyrugu,
        pub kol_kaldirilan: Arc<Mutex<Vec<u8>>>,
    }

    impl SahteFabrika {
        pub fn new(g: u32, y: u32) -> Self {
            Self {
                girdiler: Default::default(),
                boyut: (g, y),
                pano: SahtePano::default(),
                kol_durumlari: Default::default(),
                kol_titresim: Default::default(),
                kol_kaldirilan: Default::default(),
            }
        }
    }

    struct SahteEkran {
        boyut: (u32, u32),
        sayac: u32,
    }

    impl Yakalayici for SahteEkran {
        fn yakala(&mut self) -> Result<Goruntu> {
            self.sayac += 1;
            let (g, y) = self.boyut;
            let mut rgba = vec![0u8; (g * y * 4) as usize];
            // Sol üst döşemeye sayaçla değişen bir kare çiz; gerisi sabit gri.
            for (i, p) in rgba.chunks_exact_mut(4).enumerate() {
                let (xx, yy) = (i as u32 % g, i as u32 / g);
                let v = if xx < 32 && yy < 32 {
                    (self.sayac * 40 % 256) as u8
                } else {
                    128
                };
                p.copy_from_slice(&[v, v, v, 255]);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            Ok(Goruntu {
                genislik: g,
                yukseklik: y,
                rgba,
            })
        }
    }

    struct KayitEnjektor(Arc<Mutex<Vec<Girdi>>>);

    impl Enjektor for KayitEnjektor {
        fn uygula(&mut self, g: &Girdi) -> Result<()> {
            self.0.lock().unwrap().push(g.clone());
            Ok(())
        }
    }

    impl Fabrika for SahteFabrika {
        fn kol_surucusu(&self) -> Result<Box<dyn crate::sanal_kol::KolSurucusu>> {
            Ok(Box::new(crate::sanal_kol::SahteKolSurucusu::yeni(
                self.kol_durumlari.clone(),
                self.kol_titresim.clone(),
                self.kol_kaldirilan.clone(),
            )))
        }
        fn yakalayici(&self) -> Result<Box<dyn Yakalayici>> {
            Ok(Box::new(SahteEkran {
                boyut: self.boyut,
                sayac: 0,
            }))
        }
        fn enjektor(&self) -> Result<Box<dyn Enjektor>> {
            Ok(Box::new(KayitEnjektor(self.girdiler.clone())))
        }
        fn pano(&self) -> Result<Box<dyn Pano>> {
            Ok(Box::new(self.pano.clone()))
        }
    }
}
