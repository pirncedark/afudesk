//! Oyun kolu eşlemesi ve masaüstü kol desteği.
use crate::protokol::KolDurumu;

/// Gilrs eksenlerini XInput aralığına dönüştürür.
pub fn eksen(v: f32) -> i16 { (v.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16 }
pub fn tetik(v: f32) -> u8 { ((v.clamp(0.0, 1.0) * 255.0).round()) as u8 }

#[cfg(not(target_os = "android"))]
pub fn oku(tx: tokio::sync::mpsc::Sender<crate::protokol::Girdi>, olay: tokio::sync::mpsc::UnboundedSender<()>) {
    use crate::protokol::Girdi;
    use gilrs::{Axis, Button, EventType, Gilrs};
    let Ok(mut gs) = Gilrs::new() else { return };
    let mut sira = 0u32;
    let mut son = None;
    let mut son_gonderim = std::time::Instant::now();
    loop {
        let mut degisti = false;
        while let Some(e) = gs.next_event() {
            if matches!(e.event, EventType::Connected) || matches!(e.event, EventType::ButtonChanged(..) | EventType::AxisChanged(..)) { degisti = true; }
        }
        if degisti || son_gonderim.elapsed() >= std::time::Duration::from_millis(100) {
            let mut durum = None;
            if let Some((id, _)) = gs.gamepads().next() {
                let g = gs.gamepad(id);
                let mut dugmeler = 0u16;
                for (b, bit) in [(Button::DPadUp,0x0001),(Button::DPadDown,0x0002),(Button::DPadLeft,0x0004),(Button::DPadRight,0x0008),(Button::Start,0x0010),(Button::Select,0x0020),(Button::LeftThumb,0x0040),(Button::RightThumb,0x0080),(Button::LeftTrigger,0x0100),(Button::RightTrigger,0x0200),(Button::South,0x1000),(Button::East,0x2000),(Button::West,0x4000),(Button::North,0x8000)] {
                    if g.is_pressed(b) { dugmeler |= bit; }
                }
                durum = Some(KolDurumu { slot: 0, sira, dugmeler, sol_x: eksen(g.value(Axis::LeftStickX)), sol_y: eksen(g.value(Axis::LeftStickY)), sag_x: eksen(g.value(Axis::RightStickX)), sag_y: eksen(g.value(Axis::RightStickY)), sol_tetik: tetik(g.value(Axis::LeftZ)), sag_tetik: tetik(g.value(Axis::RightZ)) });
            }
            if durum != son {
                if durum.is_some() { let _ = olay.send(()); }
                if let Some(k) = durum.clone() { if tx.blocking_send(Girdi::Kol(k)).is_err() { return; } }
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
        assert_eq!(eksen(-1.0), -32767);
        assert_eq!(eksen(0.0), 0);
        assert_eq!(eksen(1.0), 32767);
        assert_eq!(tetik(0.0), 0);
        assert_eq!(tetik(1.0), 255);
    }
    #[test]
    fn dugme_maskesi_kol_durumunda_tasinir() {
        let d = KolDurumu { slot: 0, sira: 1, dugmeler: 0x1001, sol_x: 1, sol_y: 2, sag_x: 3, sag_y: 4, sol_tetik: 5, sag_tetik: 6 };
        assert_eq!(d.dugmeler, 0x1001);
    }
}
