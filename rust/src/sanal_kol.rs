//! Host tarafı sanal oyun kolu: uzak oyuncunun kolu host'ta Xbox 360 kolu olarak görünür.
//! Gerçek uygulama ViGEmBus sürücüsünü kullanır; testler sahte sürücüyle çalışır.
use crate::protokol::KolDurumu;
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// (slot, büyük motor, küçük motor)
pub type TitresimKuyrugu = Arc<Mutex<Vec<(u8, u8, u8)>>>;

pub trait KolSurucusu: Send {
    /// Slot için kol yoksa oluşturur, durumu uygular.
    fn guncelle(&mut self, d: &KolDurumu) -> Result<()>;
    /// Oyunun istediği titreşimleri (son okumadan beri) boşaltarak döndürür.
    fn titresim_al(&mut self) -> Vec<(u8, u8, u8)>;
    /// Slottaki kolu çıkarır (oyuncu ayrıldı / oturum bitti).
    fn kaldir(&mut self, slot: u8);
}

pub const SURUCU_UYARISI: &str =
    "Oyun kolu için ViGEmBus sürücüsü gerekli: https://github.com/nefarius/ViGEmBus/releases";

#[cfg(windows)]
mod vigem {
    use super::*;
    use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

    struct Kol {
        hedef: Xbox360Wired<Client>,
        dinleyici: Option<std::thread::JoinHandle<()>>,
    }

    pub struct ViGEmSurucusu {
        kollar: std::collections::HashMap<u8, Kol>,
        titresim: TitresimKuyrugu,
    }

    fn uyari<E>(_: E) -> anyhow::Error {
        anyhow::anyhow!(SURUCU_UYARISI)
    }

    impl ViGEmSurucusu {
        /// Sürücüye bağlanılabildiğini doğrular; kollar ilk girdide oluşturulur.
        pub fn yeni() -> Result<Self> {
            Client::connect().map_err(uyari)?;
            Ok(Self {
                kollar: Default::default(),
                titresim: Default::default(),
            })
        }

        fn kol_olustur(&mut self, slot: u8) -> Result<()> {
            let mut hedef =
                Xbox360Wired::new(Client::connect().map_err(uyari)?, TargetId::XBOX360_WIRED);
            hedef.plugin().map_err(uyari)?;
            hedef.wait_ready().map_err(uyari)?;
            // Oyunun titreşim isteklerini ayrı iş parçacığında dinle; hedef kaldırılınca iş parçacığı kendiliğinden biter.
            let kuyruk = self.titresim.clone();
            let dinleyici = hedef.request_notification().ok().map(|istek| {
                istek.spawn_thread(move |_, n| {
                    if let Ok(mut q) = kuyruk.lock() {
                        q.push((slot, n.large_motor, n.small_motor));
                    }
                })
            });
            self.kollar.insert(slot, Kol { hedef, dinleyici });
            Ok(())
        }
    }

    impl KolSurucusu for ViGEmSurucusu {
        fn guncelle(&mut self, d: &KolDurumu) -> Result<()> {
            if !self.kollar.contains_key(&d.slot) {
                self.kol_olustur(d.slot)?;
            }
            let kol = self.kollar.get_mut(&d.slot).expect("az önce oluşturuldu");
            kol.hedef
                .update(&XGamepad {
                    buttons: XButtons(d.dugmeler),
                    left_trigger: d.sol_tetik,
                    right_trigger: d.sag_tetik,
                    thumb_lx: d.sol_x,
                    thumb_ly: d.sol_y,
                    thumb_rx: d.sag_x,
                    thumb_ry: d.sag_y,
                })
                .map_err(Into::into)
        }

        fn titresim_al(&mut self) -> Vec<(u8, u8, u8)> {
            self.titresim
                .lock()
                .map(|mut q| std::mem::take(&mut *q))
                .unwrap_or_default()
        }

        fn kaldir(&mut self, slot: u8) {
            if let Some(Kol { hedef, dinleyici }) = self.kollar.remove(&slot) {
                drop(hedef); // unplug: bekleyen bildirim isteği iptal olur
                if let Some(d) = dinleyici {
                    let _ = d.join();
                }
            }
        }
    }

    impl Drop for ViGEmSurucusu {
        fn drop(&mut self) {
            for slot in self.kollar.keys().copied().collect::<Vec<_>>() {
                self.kaldir(slot);
            }
        }
    }
}

#[cfg(windows)]
pub use vigem::ViGEmSurucusu;

#[cfg(not(windows))]
pub struct ViGEmSurucusu;

#[cfg(not(windows))]
impl ViGEmSurucusu {
    pub fn yeni() -> Result<Self> {
        anyhow::bail!(SURUCU_UYARISI)
    }
}

/// Testler için: uygulanan durumları kaydeder; titreşim kuyruğu testten beslenir.
#[derive(Default)]
pub struct SahteKolSurucusu {
    pub durumlar: Arc<Mutex<Vec<KolDurumu>>>,
    pub titresim: TitresimKuyrugu,
    pub kaldirilan: Arc<Mutex<Vec<u8>>>,
}

impl SahteKolSurucusu {
    pub fn yeni(
        durumlar: Arc<Mutex<Vec<KolDurumu>>>,
        titresim: TitresimKuyrugu,
        kaldirilan: Arc<Mutex<Vec<u8>>>,
    ) -> Self {
        Self {
            durumlar,
            titresim,
            kaldirilan,
        }
    }
}

impl KolSurucusu for SahteKolSurucusu {
    fn guncelle(&mut self, d: &KolDurumu) -> Result<()> {
        self.durumlar.lock().unwrap().push(d.clone());
        Ok(())
    }
    fn titresim_al(&mut self) -> Vec<(u8, u8, u8)> {
        std::mem::take(&mut *self.titresim.lock().unwrap())
    }
    fn kaldir(&mut self, slot: u8) {
        self.kaldirilan.lock().unwrap().push(slot);
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn durum(slot: u8, sira: u32) -> KolDurumu {
        KolDurumu {
            slot,
            sira,
            dugmeler: 0x1000,
            sol_x: 1,
            sol_y: -2,
            sag_x: 3,
            sag_y: -4,
            sol_tetik: 5,
            sag_tetik: 255,
        }
    }

    #[test]
    fn sahte_surucu_durumlari_kaydeder() {
        let mut s = SahteKolSurucusu::default();
        s.guncelle(&durum(0, 1)).unwrap();
        s.titresim.lock().unwrap().push((0, 200, 50));
        assert_eq!(s.durumlar.lock().unwrap()[0], durum(0, 1));
        assert_eq!(s.titresim_al(), vec![(0, 200, 50)]);
        assert!(s.titresim_al().is_empty(), "kuyruk boşaltılmalı");
        s.kaldir(0);
        assert_eq!(*s.kaldirilan.lock().unwrap(), vec![0]);
    }

    /// Gerçek sürücü: kol takılır, iki durum gönderilir, çıkarılır. ViGEmBus yoksa açık uyarı verir.
    #[cfg(windows)]
    #[test]
    fn gercek_vigem_kol_olusur() {
        let mut s = match ViGEmSurucusu::yeni() {
            Ok(s) => s,
            Err(e) => {
                assert_eq!(e.to_string(), SURUCU_UYARISI);
                eprintln!("ViGEmBus yok, gerçek test atlandı");
                return;
            }
        };
        s.guncelle(&durum(0, 1)).unwrap();
        s.guncelle(&durum(0, 2)).unwrap();
        s.guncelle(&durum(1, 1)).unwrap(); // ikinci oyuncu
        s.kaldir(0);
        s.kaldir(1);
        // Sürücü takılınca (0,0) 'titreşim yok' bildirimi gönderebilir; gerçek titreşim beklenmez.
        assert!(s.titresim_al().iter().all(|&(_, b, k)| b == 0 && k == 0));
    }
}
