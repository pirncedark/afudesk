//! Measurement helpers are tested without hardware; only `gercek_cable_*`
//! opens audio devices. Run those explicitly, serially, after installing VB-CABLE.
use std::f64::consts::TAU;

const HIZ: usize = 16_000;

fn sinus(hz: f64, genlik: f64) -> Vec<i16> {
    (0..2 * HIZ)
        .map(|n| (genlik * i16::MAX as f64 * (TAU * hz * n as f64 / HIZ as f64).sin()) as i16)
        .collect()
}

#[derive(Debug)]
struct Olcum {
    rms_dbfs: f64,
    frekans_hz: usize,
}

fn guc(pcm: &[f64], hz: usize) -> f64 {
    let c = 2.0 * (TAU * hz as f64 / HIZ as f64).cos();
    let (mut a, mut b) = (0.0, 0.0);
    for x in pcm {
        let yeni = x + c * a - b;
        b = a;
        a = yeni;
    }
    a * a + b * b - c * a * b
}

fn olc(pcm: &[i16]) -> anyhow::Result<Olcum> {
    anyhow::ensure!(
        pcm.len() >= 2 * HIZ,
        "Kayıt iki saniyeden kısa: {} örnek",
        pcm.len()
    );
    // Middle second excludes capture startup and stop transients. Hann window
    // makes a nearby non-bin-aligned frequency measurable without leakage.
    let pcm: Vec<_> = pcm[HIZ / 2..HIZ + HIZ / 2]
        .iter()
        .map(|x| *x as f64 / 32768.0)
        .collect();
    let rms = (pcm.iter().map(|x| x * x).sum::<f64>() / pcm.len() as f64).sqrt();
    let rms_dbfs = 20.0 * rms.log10();
    anyhow::ensure!(rms_dbfs > -30.0, "RMS={rms_dbfs:.2} dBFS <= -30 dBFS");
    let pencere: Vec<_> = pcm
        .iter()
        .enumerate()
        .map(|(n, x)| x * (0.5 - 0.5 * (TAU * n as f64 / (HIZ - 1) as f64).cos()))
        .collect();
    // Search the entire audible Nyquist interval, not just the expected tone.
    let kaba = (20..HIZ / 2)
        .step_by(20)
        .max_by(|a, b| guc(&pencere, *a).total_cmp(&guc(&pencere, *b)))
        .unwrap();
    let frekans_hz = (kaba.saturating_sub(20).max(1)..=(kaba + 20).min(HIZ / 2 - 1))
        .max_by(|a, b| guc(&pencere, *a).total_cmp(&guc(&pencere, *b)))
        .unwrap();
    anyhow::ensure!(
        frekans_hz.abs_diff(1000) <= 30,
        "Baskın frekans={frekans_hz} Hz, beklenen 1000 +/-30 Hz; RMS={rms_dbfs:.2} dBFS"
    );
    Ok(Olcum {
        rms_dbfs,
        frekans_hz,
    })
}

#[test]
fn telmik_olcum_bir_khz_sinusu_kabul_eder() {
    let m = olc(&sinus(1000.0, 0.5)).unwrap();
    assert!((m.rms_dbfs + 9.03).abs() < 0.1);
    assert_eq!(m.frekans_hz, 1000);
}

#[test]
fn telmik_olcum_sessiz_ve_dusuk_sesi_reddeder() {
    assert!(olc(&vec![0; 2 * HIZ])
        .unwrap_err()
        .to_string()
        .contains("RMS="));
    assert!(olc(&sinus(1000.0, 0.01))
        .unwrap_err()
        .to_string()
        .contains("RMS="));
}

#[test]
fn telmik_olcum_yanlis_baskin_frekansi_reddeder() {
    assert!(olc(&sinus(2000.0, 0.5))
        .unwrap_err()
        .to_string()
        .contains("2000 Hz"));
    // An existing 1 kHz component cannot hide a stronger unrelated tone.
    let karisik: Vec<_> = sinus(2000.0, 0.6)
        .iter()
        .zip(sinus(1000.0, 0.1))
        .map(|(a, b)| a + b)
        .collect();
    assert!(olc(&karisik).is_err());
}

#[test]
fn telmik_olcum_eksik_kaydi_reddeder() {
    assert!(olc(&sinus(1000.0, 0.5)[..HIZ]).is_err());
}

#[cfg(windows)]
mod donanim {
    use super::*;
    use crate::{
        platform::{Enjektor, Fabrika},
        protokol::Girdi,
        sanal_mikrofon::{win::WindowsSes, Mikrofon, SesApi},
    };
    use anyhow::{Context, Result};
    use std::{
        process::{Child, Command, Stdio},
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    static DONANIM: Mutex<()> = Mutex::new(());

    struct Kayit {
        islem: Child,
        dizin: tempfile::TempDir,
    }
    impl Kayit {
        fn baslat() -> Result<Self> {
            // Fail before launching ffmpeg when the driver is missing.
            anyhow::ensure!(
                crate::sanal_mikrofon::cable_var(),
                "VB-CABLE kurulu değil; donanım testi çalıştırılamaz"
            );
            let dizin = tempfile::tempdir()?;
            let stderr = std::fs::File::create(dizin.path().join("ffmpeg.log"))?;
            let islem = Command::new("ffmpeg")
                .args([
                    "-nostdin",
                    "-hide_banner",
                    "-y",
                    "-f",
                    "dshow",
                    "-audio_buffer_size",
                    "20",
                    "-i",
                    "audio=CABLE Output (VB-Audio Virtual Cable)",
                    "-t",
                    "2",
                    "-ar",
                    "16000",
                    "-ac",
                    "1",
                    "-c:a",
                    "pcm_s16le",
                    "-f",
                    "s16le",
                ])
                .arg(dizin.path().join("kayit.pcm"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(stderr)
                .spawn()
                .context("ffmpeg başlatılamadı")?;
            let mut kayit = Self { islem, dizin };
            let son = Instant::now() + Duration::from_secs(8);
            loop {
                let log = kayit.log();
                if log.contains("Output #0") {
                    return Ok(kayit);
                }
                anyhow::ensure!(
                    kayit.islem.try_wait()?.is_none(),
                    "ffmpeg erken kapandı:\n{log}"
                );
                anyhow::ensure!(Instant::now() < son, "ffmpeg hazır olmadı:\n{log}");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        fn log(&self) -> String {
            std::fs::read_to_string(self.dizin.path().join("ffmpeg.log")).unwrap_or_default()
        }
        fn bitir(mut self, ad: &str) -> Result<()> {
            let son = Instant::now() + Duration::from_secs(8);
            let durum = loop {
                if let Some(durum) = self.islem.try_wait()? {
                    break durum;
                }
                anyhow::ensure!(Instant::now() < son, "ffmpeg zaman aşımı:\n{}", self.log());
                std::thread::sleep(Duration::from_millis(10));
            };
            let log = self.log();
            eprintln!("--- {ad}: gerçek ffmpeg çıktısı ---\n{log}");
            anyhow::ensure!(durum.success(), "ffmpeg başarısız: {durum}");
            let veri = std::fs::read(self.dizin.path().join("kayit.pcm"))?;
            anyhow::ensure!(veri.len() % 2 == 0, "Yarım PCM16 örneği");
            let pcm: Vec<_> = veri
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]))
                .collect();
            let m = olc(&pcm)?;
            // This line is emitted ONLY from an actual ffmpeg recording.
            eprintln!(
                "KANIT {ad}: ornek={} RMS={:.2} dBFS baskin_frekans={} Hz",
                pcm.len(),
                m.rms_dbfs,
                m.frekans_hz
            );
            Ok(())
        }
    }
    impl Drop for Kayit {
        fn drop(&mut self) {
            // Also reaps the process on panic/timeout; never leave ffmpeg behind.
            let _ = self.islem.kill();
            let _ = self.islem.wait();
        }
    }

    #[test]
    #[ignore = "Gerçek VB-CABLE ve ffmpeg gerekir; yalnız açıkça, seri çalıştır"]
    fn gercek_cable_wasapi_sinus() -> Result<()> {
        let _kilit = DONANIM.lock().unwrap();
        let mut api = WindowsSes::yeni()?;
        let (_, cal) = api.cable()?;
        api.baslat(&cal)?;
        let kayit = Kayit::baslat()?;
        let pcm = sinus(1000.0, 0.5);
        let son = Instant::now() + Duration::from_secs(5);
        let mut n = 0;
        while n < pcm.len() {
            anyhow::ensure!(Instant::now() < son, "WASAPI yazma zaman aşımı");
            n += api.yaz(&pcm[n..(n + 160).min(pcm.len())])?;
            std::thread::sleep(Duration::from_millis(2));
        }
        std::thread::sleep(Duration::from_millis(100));
        api.durdur();
        kayit.bitir("wasapi")
    }

    #[derive(Default)]
    struct Durum {
        alinan: usize,
        acik: bool,
        kapandi: bool,
        hata: Option<String>,
    }

    struct SesEnjektor {
        mik: Mikrofon,
        durum: Arc<Mutex<Durum>>,
    }
    impl Enjektor for SesEnjektor {
        fn uygula(&mut self, g: &Girdi) -> Result<()> {
            let sonuc = self.mik.uygula(g);
            let mut d = self.durum.lock().unwrap();
            if let Err(e) = &sonuc {
                d.hata = Some(format!("{e:#}"));
            }
            if sonuc.is_ok() {
                match g {
                    Girdi::SesAc => d.acik = true,
                    Girdi::Ses(b) => d.alinan += b.len(),
                    Girdi::SesKapat => d.kapandi = true,
                    _ => {}
                }
            }
            sonuc
        }
    }
    struct SesFabrika {
        sahte: crate::platform::sahte::SahteFabrika,
        durum: Arc<Mutex<Durum>>,
    }

    // The production viewer has an explicit close API. Ensure every test exit,
    // including cancellation, closes it and lets the host restore audio roles.
    struct Telefon(crate::izleyici::Izleyici);
    impl std::ops::Deref for Telefon {
        type Target = crate::izleyici::Izleyici;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl std::ops::DerefMut for Telefon {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl Drop for Telefon {
        fn drop(&mut self) {
            self.0.kapat();
        }
    }

    fn varsayilanlar() -> Result<Vec<String>> {
        let mut api = WindowsSes::yeni()?;
        [0, 2].into_iter().map(|rol| api.varsayilan(rol)).collect()
    }
    impl Fabrika for SesFabrika {
        fn yakalayici(&self) -> Result<Box<dyn crate::platform::Yakalayici>> {
            self.sahte.yakalayici()
        }
        fn enjektor(&self) -> Result<Box<dyn Enjektor>> {
            Ok(Box::new(SesEnjektor {
                mik: Mikrofon::default(),
                durum: self.durum.clone(),
            }))
        }
        fn pano(&self) -> Result<Box<dyn crate::pano::Pano>> {
            self.sahte.pano()
        }
        fn kol_surucusu(&self) -> Result<Box<dyn crate::sanal_kol::KolSurucusu>> {
            self.sahte.kol_surucusu()
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "Gerçek VB-CABLE ve ffmpeg gerekir; varsayılan mikrofonu geçici değiştirir"]
    async fn gercek_cable_protokol_uctan_uca() -> Result<()> {
        let _kilit = DONANIM.lock().unwrap();
        anyhow::ensure!(crate::sanal_mikrofon::cable_var(), "VB-CABLE kurulu değil");
        tokio::time::timeout(Duration::from_secs(40), async {
            use crate::{host::{self, HostAyar, HostKomut, HostOlay}, izleyici, protokol::Izinler};
            let durum = Arc::new(Mutex::new(Durum::default()));
            let mut h = host::baslat(HostAyar {
                ad: "Mikrofon kanıt testi".into(), upnp: false, parola: "1234567890".into(),
                yalniz_yerel: true, dosya_klasoru: None, veri_klasoru: None, kod_acik: true,
            }, Arc::new(SesFabrika { sahte: crate::platform::sahte::SahteFabrika::new(64, 48), durum: durum.clone() })).await?;
            let kod = loop {
                match h.olaylar.recv().await.context("Host kapandı")? {
                    HostOlay::Hazir { kod, .. } => break kod,
                    HostOlay::Hata(e) => anyhow::bail!(e),
                    _ => {}
                }
            };
            let eski = varsayilanlar()?;
            let mut i = Telefon(izleyici::baglan(&kod, "1234567890", "Sahte telefon").await?);
            loop {
                if matches!(h.olaylar.recv().await.context("Host kapandı")?, HostOlay::Istek { .. }) { break; }
            }
            h.komut(HostKomut::Kabul(Izinler { kontrol: true, pano: false, dosya: false, oyun_kolu: false })).await;
            // Drain frame events continuously so a slow consumer cannot block
            // the real protocol/capability negotiation while sending audio.
            while !i.ses_destegi() {
                tokio::select! {
                    olay = i.olaylar.recv() => {
                        if let Some(izleyici::IzleyiciOlay::Koptu { sebep }) = olay { anyhow::bail!(sebep); }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(10)) => {}
                }
            }
            let mut olaylar = std::mem::replace(&mut i.olaylar, tokio::sync::mpsc::channel(1).1);
            let pompa = tokio::spawn(async move { while olaylar.recv().await.is_some() {} });
            // Abort the pump even if an assertion/timeout cancels this future.
            struct Pompa(tokio::task::JoinHandle<()>);
            impl Drop for Pompa { fn drop(&mut self) { self.0.abort(); } }
            let _pompa = Pompa(pompa);
            i.gonder(Girdi::SesAc).await;
            while !durum.lock().unwrap().acik {
                anyhow::ensure!(durum.lock().unwrap().hata.is_none(), "Ses açılamadı");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            let (cable, _) = WindowsSes::yeni()?.cable()?;
            assert_eq!(varsayilanlar()?, vec![cable.clone(), cable], "Bas-konuş sırasında iki rol de CABLE Output olmalı");
            let kayit = Kayit::baslat()?;
            let pcm = sinus(1000.0, 0.5);
            let baslangic = tokio::time::Instant::now();
            for (n, parca) in pcm.chunks_exact(320).enumerate() {
                tokio::time::sleep_until(baslangic + Duration::from_millis(n as u64 * 20)).await;
                i.gonder(Girdi::Ses(parca.iter().flat_map(|x| x.to_le_bytes()).collect())).await;
            }
            i.gonder(Girdi::SesKapat).await;
            while !durum.lock().unwrap().kapandi {
                anyhow::ensure!(durum.lock().unwrap().hata.is_none(), "Ses yazılamadı");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(durum.lock().unwrap().alinan, 64_000, "Protokolde PCM bayt kaybı");
            let sonuc = kayit.bitir("protokol_uctan_uca");
            i.kapat();
            loop {
                if matches!(h.olaylar.recv().await.context("Host kapandı")?, HostOlay::Koptu { .. }) { break; }
            }
            h.durdur();
            assert_eq!(varsayilanlar()?, eski, "Oturumdan sonra eski mikrofonlar geri yüklenmeli");
            sonuc
        }).await.context("Uçtan uca donanım testi zaman aşımı")?
    }
}
