//! Real X11 capture and input smoke, run only inside the isolated CI Xvfb.
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use afudesk_core::{
        platform::{masaustu::Gercek, Fabrika},
        protokol::{FareTusu, Girdi},
    };
    use anyhow::{bail, ensure, Context};
    use std::{
        path::PathBuf,
        process::Command,
        thread::sleep,
        time::{Duration, Instant},
    };

    ensure!(
        std::env::var("AFUDESK_ISOLATED_SMOKE").as_deref() == Ok("1"),
        "Run only in the isolated Xvfb smoke session"
    );
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("output directory required")?,
    );
    std::fs::create_dir_all(&output)?;
    let mut capture = Gercek.yakalayici()?;
    let frame = capture.yakala()?;
    ensure!(
        frame.genislik >= 640 && frame.yukseklik >= 480,
        "capture too small"
    );
    ensure!(
        frame.rgba.len() == (frame.genislik * frame.yukseklik * 4) as usize,
        "invalid RGBA length"
    );
    ensure!(
        frame
            .rgba
            .chunks_exact(4)
            .any(|p| p[0] > 10 || p[1] > 10 || p[2] > 10),
        "capture is entirely black"
    );
    image::save_buffer(
        output.join("rust-xcap.png"),
        &frame.rgba,
        frame.genislik,
        frame.yukseklik,
        image::ColorType::Rgba8,
    )?;
    let px: i32 = std::env::var("SMOKE_INPUT_X")?.parse()?;
    let py: i32 = std::env::var("SMOKE_INPUT_Y")?.parse()?;
    let mut input = Gercek.enjektor()?;
    input.uygula(&Girdi::FareKonum {
        x: px as f32 / (frame.genislik - 1) as f32,
        y: py as f32 / (frame.yukseklik - 1) as f32,
    })?;
    sleep(Duration::from_millis(150));
    let mouse = Command::new("xdotool")
        .args(["getmouselocation", "--shell"])
        .output()?;
    ensure!(mouse.status.success(), "xdotool could not observe mouse");
    let mouse = String::from_utf8(mouse.stdout)?;
    ensure!(
        mouse.lines().any(|l| l == format!("X={px}"))
            && mouse.lines().any(|l| l == format!("Y={py}")),
        "Enigo mouse position mismatch: {mouse}"
    );
    std::fs::write(output.join("mouse.txt"), mouse)?;
    input.uygula(&Girdi::FareTus {
        tus: FareTusu::Sol,
        basili: true,
    })?;
    input.uygula(&Girdi::FareTus {
        tus: FareTusu::Sol,
        basili: false,
    })?;
    input.uygula(&Girdi::Metin("afudesk-linux-smoke".into()))?;
    for basili in [true, false] {
        input.uygula(&Girdi::Tus {
            hid: 0x70028,
            ad: "Enter".into(),
            basili,
        })?;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(bytes) = std::fs::read(output.join("input.json")) {
            if let Ok(state) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if state["text"] == "afudesk-linux-smoke"
                    && state["enter"] == true
                    && state["clicks"].as_u64().unwrap_or(0) > 0
                {
                    println!(
                        "PASS: real xcap {}x{}, Enigo mouse/click/text/Enter",
                        frame.genislik, frame.yukseklik
                    );
                    return Ok(());
                }
            }
        }
        if Instant::now() >= deadline {
            bail!("input receiver did not observe Enigo click/text/Enter");
        }
        sleep(Duration::from_millis(100));
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("linux_smoke requires Linux and an isolated Xvfb session");
    std::process::exit(1);
}
