//! Oyun kolu eşlemesi ve masaüstü kol desteği.
use crate::protokol::KolDurumu;

/// Gilrs eksenlerini XInput aralığına dönüştürür.
pub fn eksen(v: f32) -> i16 {
    let v = v.clamp(-1.0, 1.0);
    let olcek = if v < 0.0 { 32768.0 } else { 32767.0 };
    (v * olcek).round() as i16
}
pub fn tetik(v: f32) -> u8 {
    ((v.clamp(0.0, 1.0) * 255.0).round()) as u8
}

/// Gilrs omuz düğmeleri LB/RB, Mode ise Guide bitine eşlenir.
#[cfg(not(target_os = "android"))]
fn dugme_maskesi(basili: impl Fn(gilrs::Button) -> bool) -> u16 {
    use gilrs::Button;
    [
        (Button::DPadUp, 0x0001),
        (Button::DPadDown, 0x0002),
        (Button::DPadLeft, 0x0004),
        (Button::DPadRight, 0x0008),
        (Button::Start, 0x0010),
        (Button::Select, 0x0020),
        (Button::LeftThumb, 0x0040),
        (Button::RightThumb, 0x0080),
        (Button::LeftTrigger, 0x0100),
        (Button::RightTrigger, 0x0200),
        (Button::Mode, 0x0400),
        (Button::South, 0x1000),
        (Button::East, 0x2000),
        (Button::West, 0x4000),
        (Button::North, 0x8000),
    ]
    .into_iter()
    .filter(|(b, _)| basili(*b))
    .fold(0, |maske, (_, bit)| maske | bit)
}

#[cfg(not(target_os = "android"))]
pub fn oku(
    tx: tokio::sync::mpsc::Sender<crate::protokol::Girdi>,
    olay: tokio::sync::mpsc::UnboundedSender<()>,
    titresim: std::sync::mpsc::Receiver<(u8, u8)>,
) {
    use crate::protokol::Girdi;
    use gilrs::{Axis, Button, EventType, Gilrs};
    let Ok(mut gs) = Gilrs::new() else { return };
    let mut sira = 0u32;
    let mut son = None;
    let mut son_gonderim = std::time::Instant::now();
    loop {
        if tx.is_closed() {
            return;
        }
        while let Ok((buyuk, kucuk)) = titresim.try_recv() {
            use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Repeat, Replay, Ticks};
            if let Some((id, gamepad)) = gs.gamepads().find(|(_, g)| g.is_ff_supported()) {
                let sure = Ticks::from_ms(100);
                let efekt = EffectBuilder::new()
                    .add_effect(BaseEffect {
                        kind: BaseEffectType::Strong {
                            magnitude: (buyuk as u16) * 257,
                        },
                        scheduling: Replay {
                            play_for: sure,
                            ..Default::default()
                        },
                        envelope: Default::default(),
                    })
                    .add_effect(BaseEffect {
                        kind: BaseEffectType::Weak {
                            magnitude: (kucuk as u16) * 257,
                        },
                        scheduling: Replay {
                            play_for: sure,
                            ..Default::default()
                        },
                        envelope: Default::default(),
                    })
                    .add_gamepad(&gamepad)
                    .repeat(Repeat::For(sure))
                    .finish(&mut gs);
                if let Ok(e) = efekt {
                    let _ = e.play();
                }
                let _ = id;
            }
        }
        let mut degisti = false;
        while let Some(e) = gs.next_event() {
            if matches!(e.event, EventType::Connected)
                || matches!(
                    e.event,
                    EventType::ButtonChanged(..) | EventType::AxisChanged(..)
                )
            {
                degisti = true;
            }
        }
        if degisti || son_gonderim.elapsed() >= std::time::Duration::from_millis(100) {
            let mut durum = None;
            if let Some((id, _)) = gs.gamepads().next() {
                let g = gs.gamepad(id);
                let dugmeler = dugme_maskesi(|b| g.is_pressed(b));
                let sol_tetik = g
                    .button_data(Button::LeftTrigger2)
                    .map(|d| d.value())
                    .unwrap_or_else(|| g.value(Axis::LeftZ));
                let sag_tetik = g
                    .button_data(Button::RightTrigger2)
                    .map(|d| d.value())
                    .unwrap_or_else(|| g.value(Axis::RightZ));
                durum = Some(KolDurumu {
                    slot: 0,
                    sira,
                    dugmeler,
                    sol_x: eksen(g.value(Axis::LeftStickX)),
                    sol_y: eksen(g.value(Axis::LeftStickY)),
                    sag_x: eksen(g.value(Axis::RightStickX)),
                    sag_y: eksen(g.value(Axis::RightStickY)),
                    sol_tetik: tetik(sol_tetik),
                    sag_tetik: tetik(sag_tetik),
                });
            }
            if durum != son {
                if durum.is_some() {
                    let _ = olay.send(());
                }
                if let Some(k) = durum.clone() {
                    if tx.blocking_send(Girdi::Kol(k)).is_err() {
                        return;
                    }
                }
                son = durum;
                sira = sira.wrapping_add(1);
            }
            son_gonderim = std::time::Instant::now();
        }
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    #[test]
    fn eksen_ve_tetik_sinir_eslemesi() {
        assert_eq!(eksen(-1.0), i16::MIN);
        assert_eq!(eksen(-2.0), i16::MIN);
        assert_eq!(eksen(-0.5), -16384);
        assert_eq!(eksen(0.0), 0);
        assert_eq!(eksen(1.0), 32767);
        assert_eq!(eksen(2.0), i16::MAX);
        assert_eq!(tetik(0.0), 0);
        assert_eq!(tetik(1.0), 255);
    }
    #[test]
    fn dugme_maskesi_kol_durumunda_tasinir() {
        let d = KolDurumu {
            slot: 0,
            sira: 1,
            dugmeler: 0x1001,
            sol_x: 1,
            sol_y: 2,
            sag_x: 3,
            sag_y: 4,
            sol_tetik: 5,
            sag_tetik: 6,
        };
        assert_eq!(d.dugmeler, 0x1001);
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn omuz_guide_ve_tetik_maskeleri() {
        use gilrs::Button;
        assert_eq!(dugme_maskesi(|b| b == Button::LeftTrigger), 0x0100);
        assert_eq!(dugme_maskesi(|b| b == Button::RightTrigger), 0x0200);
        assert_eq!(dugme_maskesi(|b| b == Button::Mode), 0x0400);
        assert_eq!(dugme_maskesi(|b| b == Button::LeftTrigger2), 0);
        assert_eq!(dugme_maskesi(|b| b == Button::RightTrigger2), 0);
        assert_eq!(dugme_maskesi(|_| true), 0xf7ff);
    }
}
