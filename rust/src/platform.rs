//! Platforma bağlı parçalar: ekran yakalama ve girdi enjeksiyonu.
//! Testler sahte uygulamaları kullanır; gerçek uygulamalar masaüstünde xcap + enigo.
use crate::{goruntu::Goruntu, protokol::Girdi};
use anyhow::Result;

/// Send değil: bazı platform tutamaçları iş parçacığına bağlıdır; yakalayıcı
/// kullanılacağı iş parçacığında oluşturulur.
pub trait Yakalayici {
    fn yakala(&mut self) -> Result<Goruntu>;
}

pub trait Enjektor: Send {
    fn uygula(&mut self, g: &Girdi) -> Result<()>;
}

/// Her oturum için yeni yakalayıcı/enjektör üretir (ikisi de iş parçacığına bağlı olabilir).
pub trait Fabrika: Send + Sync {
    fn yakalayici(&self) -> Result<Box<dyn Yakalayici>>;
    fn enjektor(&self) -> Result<Box<dyn Enjektor>>;
    fn kol_surucusu(&self) -> Result<Box<dyn crate::sanal_kol::KolSurucusu>>;
}

#[cfg(not(target_os = "android"))]
pub mod masaustu {
    use super::*;
    use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

    pub struct Gercek;

    impl Fabrika for Gercek {
        fn yakalayici(&self) -> Result<Box<dyn Yakalayici>> {
            Ok(Box::new(XcapYakalayici::new()?))
        }
        fn enjektor(&self) -> Result<Box<dyn Enjektor>> {
            Ok(Box::new(EnigoEnjektor::new()?))
        }
        fn kol_surucusu(&self) -> Result<Box<dyn crate::sanal_kol::KolSurucusu>> {
            Ok(Box::new(crate::sanal_kol::ViGEmSurucusu::yeni()?))
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
    }

    impl EnigoEnjektor {
        pub fn new() -> Result<Self> {
            let e = Enigo::new(&Settings::default())?;
            let boyut = e.main_display()?;
            Ok(Self { e, boyut })
        }
    }

    pub fn tus_coz(ad: &str) -> Option<Key> {
        Some(match ad {
            "Enter" => Key::Return,
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
            _ => {
                let mut c = ad.chars();
                let ilk = c.next()?;
                if c.next().is_some() {
                    return None;
                }
                Key::Unicode(ilk)
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
                Girdi::Tus { ad, basili } => {
                    if let Some(k) = tus_coz(ad) {
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
            assert_eq!(tus_coz("ArrowLeft"), Some(Key::LeftArrow));
            assert_eq!(tus_coz("BilinmeyenTus"), None);
            assert_eq!(tus_coz(""), None);
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

        #[test]
        fn gercek_enjektor_acilir() {
            let e = EnigoEnjektor::new().expect("enigo açılamadı");
            assert!(e.boyut.0 > 0 && e.boyut.1 > 0);
        }
    }
}

/// Testler için: renk değiştiren sahte ekran ve girdileri kaydeden enjektör.
pub mod sahte {
    use super::*;
    use std::sync::{Arc, Mutex};

    pub struct SahteFabrika {
        pub girdiler: Arc<Mutex<Vec<Girdi>>>,
        pub boyut: (u32, u32),
        pub kol_durumlari: Arc<Mutex<Vec<crate::protokol::KolDurumu>>>,
        pub kol_titresim: crate::sanal_kol::TitresimKuyrugu,
        pub kol_kaldirilan: Arc<Mutex<Vec<u8>>>,
    }

    impl SahteFabrika {
        pub fn new(g: u32, y: u32) -> Self {
            Self {
                girdiler: Default::default(),
                boyut: (g, y),
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
    }
}
