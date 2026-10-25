use crate::protokol::Girdi;
use std::time::{Duration, Instant};

/// Counts PCM payload only, at the transport boundary (not AudioRecord capture).
pub(crate) struct Sayac {
    taraf: &'static str,
    yon: &'static str,
    bayt: u64,
    son: Instant,
    acik: bool,
}

impl Sayac {
    pub(crate) fn yeni(taraf: &'static str, yon: &'static str) -> Self {
        Self {
            taraf,
            yon,
            bayt: 0,
            son: Instant::now(),
            acik: false,
        }
    }

    fn mesaj(&self) -> String {
        format!("telmik {} {}={}", self.taraf, self.yon, self.bayt)
    }

    fn guncelle(&mut self, g: &Girdi, simdi: Instant) -> Option<String> {
        match g {
            Girdi::SesAc => {
                self.bayt = 0;
                self.son = simdi;
                self.acik = true;
            }
            Girdi::Ses(b) => {
                self.bayt += b.len() as u64;
                self.acik = true;
                if simdi.saturating_duration_since(self.son) >= Duration::from_secs(1) {
                    self.son = simdi;
                    return Some(self.mesaj());
                }
            }
            Girdi::SesKapat => {
                self.acik = false;
                return Some(self.mesaj());
            }
            _ => {}
        }
        None
    }

    pub(crate) fn isle(&mut self, g: &Girdi) {
        if let Some(mesaj) = self.guncelle(g, Instant::now()) {
            kaydet(&mesaj);
        }
    }

    pub(crate) async fn gonder<W: tokio::io::AsyncWrite + Unpin>(
        &mut self,
        w: &mut W,
        g: Girdi,
    ) -> anyhow::Result<()> {
        let mesaj = crate::protokol::Kontrol::Girdi(g);
        crate::protokol::yaz(w, &mesaj).await?;
        if let crate::protokol::Kontrol::Girdi(g) = &mesaj {
            self.isle(g);
        }
        Ok(())
    }
}

impl Drop for Sayac {
    fn drop(&mut self) {
        // A disconnect must also leave a final count for phone/host comparison.
        if self.acik {
            kaydet(&self.mesaj());
        }
    }
}

fn kaydet(mesaj: &str) {
    log::info!(target: "AfuDeskMik", "{mesaj}");
    // The Rust bridge does not install an Android logger. Write to logcat
    // directly, only numeric diagnostics, never microphone contents.
    #[cfg(target_os = "android")]
    {
        #[link(name = "log")]
        extern "C" {
            fn __android_log_write(
                prio: i32,
                tag: *const std::ffi::c_char,
                text: *const std::ffi::c_char,
            ) -> i32;
        }
        if let Ok(text) = std::ffi::CString::new(mesaj) {
            unsafe {
                __android_log_write(4, c"AfuDeskMik".as_ptr(), text.as_ptr());
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn telmik_sayac_pcm_baytlarini_sayar_kapanista_raporlar() {
        let mut s = Sayac::yeni("telefon", "gonderilen");
        let t = Instant::now();
        assert_eq!(s.guncelle(&Girdi::SesAc, t), None);
        assert_eq!(s.guncelle(&Girdi::Ses(vec![0; 640]), t), None);
        assert_eq!(
            s.guncelle(&Girdi::Ses(vec![0; 640]), t + Duration::from_secs(1)),
            Some("telmik telefon gonderilen=1280".into())
        );
        assert_eq!(
            s.guncelle(&Girdi::SesKapat, t + Duration::from_secs(1)),
            Some("telmik telefon gonderilen=1280".into())
        );
    }

    #[test]
    fn telmik_sayac_yeni_bas_konusta_sifirlanir() {
        let mut s = Sayac::yeni("host", "alinan");
        let t = Instant::now();
        s.guncelle(&Girdi::Ses(vec![0; 640]), t);
        s.guncelle(&Girdi::SesAc, t);
        assert_eq!(
            s.guncelle(&Girdi::SesKapat, t),
            Some("telmik host alinan=0".into())
        );
    }

    #[tokio::test]
    async fn telmik_sayac_basarisiz_ag_yazimini_saymaz() {
        let mut s = Sayac::yeni("telefon", "gonderilen");
        let (mut tx, rx) = tokio::io::duplex(2048);
        drop(rx);
        assert!(s.gonder(&mut tx, Girdi::Ses(vec![0; 640])).await.is_err());
        assert_eq!(s.bayt, 0);
    }

    #[tokio::test]
    async fn telmik_sayac_agdan_gonderilen_pcm_ile_eslesir() {
        let mut s = Sayac::yeni("telefon", "gonderilen");
        let (mut tx, mut rx) = tokio::io::duplex(2048);
        let g = Girdi::Ses(vec![42; 640]);
        s.gonder(&mut tx, g.clone()).await.unwrap();
        let gelen: crate::protokol::Kontrol = crate::protokol::oku(&mut rx).await.unwrap().unwrap();
        assert_eq!(gelen, crate::protokol::Kontrol::Girdi(g));
        assert_eq!(s.bayt, 640);
    }
}
