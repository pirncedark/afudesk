//! Tel protokolü: iroh (QUIC) akışları üzerinde uzunluk önekli (u32 BE) bincode mesajları.
//! Kontrol akışı (çift yönlü): Merhaba|MerhabaKayitli|Devam / DevamJetonu+Kabul(+KayitJetonu) / Red /
//! Girdi / Pano / Kapat.
//! Görüntü akışı (tek yönlü, host → izleyici): Kare.
//! Dosya akışı (çift yönlü, izleyici açar; dosya başına bir akış):
//! DosyaBaslik → DosyaDevam | DosyaHata, sonra DosyaParca… ve DosyaTamam | DosyaHata.
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Protokol sürümü ALPN'nin içindedir: tel biçimi değişince ikisi birlikte artar.
/// Uyuşmayan sürümler el sıkışmada ayrılır (paket sürümüne bağlı değildir).
pub const SURUM: u32 = 8;
pub const ALPN: &[u8] = b"afudesk/8";
/// Tek mesaj için üst sınır (kötü niyetli uzunluk alanına karşı).
pub const AZAMI_MESAJ: usize = 32 * 1024 * 1024;
/// İzleyicide aynı anda işlenen tek yönlü akış tavanı (kareler + dosyalar).
pub const AZAMI_ESZAMANLI_AKIS: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FareTusu {
    Sol,
    Sag,
    Orta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Girdi {
    /// Ekran boyutundan bağımsız, 0.0..=1.0 aralığında konum.
    FareKonum {
        x: f32,
        y: f32,
    },
    FareTus {
        tus: FareTusu,
        basili: bool,
    },
    /// Satır cinsinden kaydırma; pozitif = aşağı / sağ.
    Kaydir {
        dx: i32,
        dy: i32,
    },
    /// Adlandırılmış tuş ("Enter", "Backspace", "Ctrl", "a", "ş" ...).
    Tus {
        /// USB HID usage (sayfa ve kullan?m birlikte); s?f?rsa ad yede?i.
        hid: u32,
        ad: String,
        basili: bool,
    },
    /// Doğrudan metin yazma (mobil klavye için).
    Metin(String),
    Kol(KolDurumu),
    /// PCM16 little endian, mono, 16 kHz; her Ses tam 20 ms (640 bayt).
    SesAc,
    Ses(Vec<u8>),
    SesKapat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KolDurumu {
    pub slot: u8,
    pub sira: u32,
    pub dugmeler: u16,
    pub sol_x: i16,
    pub sol_y: i16,
    pub sag_x: i16,
    pub sag_y: i16,
    pub sol_tetik: u8,
    pub sag_tetik: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Izinler {
    pub kontrol: bool,
    pub pano: bool,
    /// İzleyiciden dosya alma. Not: bincode kendini tanımlamadığı için `default`
    /// yalnız kendini tanımlayan biçimlerde (JSON vb.) işe yarar.
    #[serde(default)]
    pub dosya: bool,
    #[serde(default)]
    pub oyun_kolu: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Kontrol {
    /// İzleyici → host, ilk mesaj.
    Merhaba {
        bilet: String,
        ad: String,
    },
    /// Host → izleyici.
    Kabul {
        izinler: Izinler,
        genislik: u32,
        yukseklik: u32,
    },
    Red(String),
    Girdi(Girdi),
    Pano(String),
    Kapat(String),
    SaatSor {
        izleyici_ms: u64,
    },
    SaatCevap {
        izleyici_ms: u64,
        host_ms: u64,
    },
    /// İzleyici → host, dosya akışının ilk mesajı.
    DosyaBaslik {
        kimlik: [u8; 16],
        ad: String,
        boyut: u64,
        sha256: [u8; 32],
    },
    /// Host → izleyici: bu bayttan itibaren gönder (yarım dosya varsa > 0).
    DosyaDevam {
        baslangic: u64,
    },
    /// İzleyici → host: sıradaki dosya parçası (en çok `dosya::PARCA` bayt).
    DosyaParca(Vec<u8>),
    /// Host → izleyici: dosya eksiksiz geldi, SHA-256 doğru, yerine taşındı.
    DosyaTamam,
    DosyaHata(String),

    Titresim {
        slot: u8,
        buyuk: u8,
        kucuk: u8,
    },
    /// Host → izleyici, `Kabul`'den hemen önce: bağlantı istemeden koparsa izleyici bu
    /// jetonla onay sorulmadan geri dönebilir (yalnız aynı cihaz kimliğiyle, kısa süre).
    DevamJetonu(String),
    /// İzleyici → host, kopan oturuma dönüş (Merhaba yerine ilk mesaj).
    Devam {
        jeton: String,
        ad: String,
    },
    /// İzleyici → host, kayıtlı cihaz olarak bağlanma (kod yerine eşleşme jetonu).
    /// Cihaz kimliği mesajda değil, el sıkışmada doğrulanan uç kimliğindedir.
    MerhabaKayitli {
        jeton: String,
        ad: String,
    },
    /// Host → izleyici, kodla yapılan onaylı ilk oturumda (Kabul'den sonra): bundan sonra
    /// kodsuz bağlanabilmek için eşleşme jetonu ve host'a ulaşma bilgisi.
    KayitJetonu {
        host_ad: String,
        adresler: Vec<String>,
        relaylar: Vec<String>,
        jeton: String,
    },
    /// Google kan?t? yaln?z ?ifreli, kimli?i do?rulanm?? QUIC ak???nda ta??n?r.
    MerhabaGoogle {
        kanit: crate::google_baglanti::Kanit,
        ad: String,
    },
    /// İzin verilen mevcut terminal pencerelerini iste.
    TerminalListesi,
    TerminalPencereleri(Vec<crate::terminal::Pencere>),
    /// None: masaüstüne dön ve pencerenin önceki yerleşimini geri yükle.
    TerminalModu {
        ac: Option<u64>,
        en: u32,
        boy: u32,
    },
    TerminalDurum {
        acik: bool,
        hata: Option<String>,
    },
}

/// Eski host bunu normal SaatSor olarak yanıtlar. Eski istemci bu değeri üretmez.
/// Hız yoklaması (u64::MAX - 1) ile çakışmaz; yeni Kontrol varyantı gerektirmez.
pub const SES_SOR: u64 = u64::MAX - 2;
/// Normal host saati bu değeri alamaz.
pub const SES_VAR: u64 = u64::MAX;

pub fn ses_girdisi(g: &Girdi) -> bool {
    matches!(g, Girdi::SesAc | Girdi::Ses(_) | Girdi::SesKapat)
}

pub fn yetenek_yaniti(izleyici_ms: u64) -> Option<Kontrol> {
    (izleyici_ms == SES_SOR).then_some(Kontrol::SaatCevap {
        izleyici_ms: SES_SOR,
        host_ms: if cfg!(windows) { SES_VAR } else { 0 },
    })
}

pub fn ses_gonderilebilir(yetenek: bool, g: &Girdi) -> bool {
    !ses_girdisi(g) || yetenek
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Doseme {
    pub x: u32,
    pub y: u32,
    pub gen: u32,
    pub yuk: u32,
    /// JPEG baytları.
    pub jpeg: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kare {
    pub sira: u64,
    pub genislik: u32,
    pub yukseklik: u32,
    /// true: tüm ekran döşemeleri var (anahtar kare).
    pub tam: bool,
    pub dosemeler: Vec<Doseme>,
    /// Host saatine göre Unix milisaniye cinsinden ekranın yakalandığı an.
    pub yakalama_ms: u64,
}

pub async fn yaz<W: AsyncWriteExt + Unpin, T: Serialize>(w: &mut W, m: &T) -> anyhow::Result<()> {
    let b = bincode::serialize(m)?;
    anyhow::ensure!(b.len() <= AZAMI_MESAJ, "mesaj çok büyük: {}", b.len());
    w.write_all(&(b.len() as u32).to_be_bytes()).await?;
    w.write_all(&b).await?;
    Ok(())
}

/// Akış düzgün kapandıysa `None`.
pub async fn oku<R: AsyncReadExt + Unpin, T: for<'de> Deserialize<'de>>(
    r: &mut R,
) -> anyhow::Result<Option<T>> {
    let mut u = [0u8; 4];
    match r.read_exact(&mut u).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let n = u32::from_be_bytes(u) as usize;
    anyhow::ensure!(n <= AZAMI_MESAJ, "mesaj uzunluğu sınırı aşıyor: {n}");
    let mut b = vec![0u8; n];
    r.read_exact(&mut b).await?;
    Ok(Some(bincode::deserialize(&b)?))
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

    #[test]
    fn ses_saat_cevabi_saat_olcumunu_degistirmez() {
        for esit in [false, true] {
            for (host_ms, izinli, beklenen_ses) in [
                (u64::MAX, true, true),
                (u64::MAX, false, false),
                (0, true, false),
                (1720000000000, true, false),
            ] {
                let ses = AtomicBool::new(false);
                let fark = AtomicI64::new(-42);
                let esitlendi = AtomicBool::new(esit);
                crate::izleyici::saat_cevabini_isle(
                    SES_SOR, host_ms, 1720000000050, izinli,
                    &ses, &fark, &esitlendi,
                );
                assert_eq!(ses.load(Ordering::SeqCst), beklenen_ses);
                assert_eq!(fark.load(Ordering::Relaxed), -42);
                assert_eq!(esitlendi.load(Ordering::Relaxed), esit);
            }
        }
    }

    #[test]
    fn normal_saat_cevabi_olcumu_gunceller_sesi_degistirmez() {
        let ses = AtomicBool::new(true);
        let fark = AtomicI64::new(-42);
        let esitlendi = AtomicBool::new(false);
        crate::izleyici::saat_cevabini_isle(100, 120, 110, false, &ses, &fark, &esitlendi);
        assert!(ses.load(Ordering::SeqCst));
        assert_eq!(fark.load(Ordering::Relaxed), 15);
        assert!(esitlendi.load(Ordering::Relaxed));
    }

    #[test]
    fn ses_yoklamasi_mevcut_saat_cevabini_kullanir() {
        assert_eq!(
            yetenek_yaniti(u64::MAX - 2),
            Some(Kontrol::SaatCevap {
                izleyici_ms: u64::MAX - 2,
                host_ms: if cfg!(windows) { u64::MAX } else { 0 },
            })
        );
    }

    #[tokio::test]
    async fn yeni_host_ses_yanitini_mevcut_tel_biciminde_gonderir() {
        let (mut host, mut izleyici) = tokio::io::duplex(256);
        yaz(&mut host, &yetenek_yaniti(SES_SOR).unwrap()).await.unwrap();
        let Some(Kontrol::SaatCevap { izleyici_ms, host_ms }) =
            oku::<_, Kontrol>(&mut izleyici).await.unwrap() else { panic!("SaatCevap bekleniyordu") };
        assert_eq!(izleyici_ms, u64::MAX - 2);
        assert_eq!(host_ms, if cfg!(windows) { u64::MAX } else { 0 });
        let ses = AtomicBool::new(false);
        let fark = AtomicI64::new(42);
        let esitlendi = AtomicBool::new(false);
        crate::izleyici::saat_cevabini_isle(
            izleyici_ms, host_ms, 150, true, &ses, &fark, &esitlendi,
        );
        assert_eq!(ses.load(Ordering::SeqCst), cfg!(windows));
        assert_eq!(fark.load(Ordering::Relaxed), 42);
        assert!(!esitlendi.load(Ordering::Relaxed));
    }

    #[test]
    fn kontrol_son_indeksi_22_yeni_varyant_yok() {
        let son = Kontrol::TerminalDurum { acik: false, hata: None };
        assert_eq!(&bincode::serialize(&son).unwrap()[..4], &[22, 0, 0, 0]);
        assert!(bincode::deserialize::<Kontrol>(&[23, 0, 0, 0, 1]).is_err());
    }

    #[test]
    fn eski_tel_baytlari_aynen_kalir() {
        let kabul = Kontrol::Kabul {
            izinler: Izinler { kontrol: true, pano: false, dosya: true, oyun_kolu: false },
            genislik: 800,
            yukseklik: 600,
        };
        assert_eq!(bincode::serialize(&kabul).unwrap(), [
            1, 0, 0, 0, 1, 0, 1, 0, 32, 3, 0, 0, 88, 2, 0, 0,
        ]);
        let saat = Kontrol::SaatCevap { izleyici_ms: 1, host_ms: 2 };
        assert_eq!(bincode::serialize(&saat).unwrap(), [
            7, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
        ]);
    }

    // Ses eklenmeden önceki tel şeması; yeni tipten türetilmez.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum EskiGirdi {
        FareKonum { x: f32, y: f32 },
        FareTus { tus: FareTusu, basili: bool },
        Kaydir { dx: i32, dy: i32 },
        Tus { hid: u32, ad: String, basili: bool },
        Metin(String),
        Kol(KolDurumu),
    }
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum EskiKontrol {
        Merhaba {
            bilet: String,
            ad: String,
        },
        Kabul {
            izinler: Izinler,
            genislik: u32,
            yukseklik: u32,
        },
        Red(String),
        Girdi(EskiGirdi),
        Pano(String),
        Kapat(String),
        SaatSor {
            izleyici_ms: u64,
        },
        SaatCevap {
            izleyici_ms: u64,
            host_ms: u64,
        },
        DosyaBaslik {
            kimlik: [u8; 16],
            ad: String,
            boyut: u64,
            sha256: [u8; 32],
        },
        DosyaDevam {
            baslangic: u64,
        },
        DosyaParca(Vec<u8>),
        DosyaTamam,
        DosyaHata(String),
        Titresim {
            slot: u8,
            buyuk: u8,
            kucuk: u8,
        },
        DevamJetonu(String),
        Devam {
            jeton: String,
            ad: String,
        },
        MerhabaKayitli {
            jeton: String,
            ad: String,
        },
        KayitJetonu {
            host_ad: String,
            adresler: Vec<String>,
            relaylar: Vec<String>,
            jeton: String,
        },
        MerhabaGoogle {
            kanit: crate::google_baglanti::Kanit,
            ad: String,
        },
        TerminalListesi,
        TerminalPencereleri(Vec<crate::terminal::Pencere>),
        TerminalModu {
            ac: Option<u64>,
            en: u32,
            boy: u32,
        },
        TerminalDurum {
            acik: bool,
            hata: Option<String>,
        },
    }

    #[tokio::test]
    async fn eski_hosta_ses_gitmez_space_devam_eder() {
        let (mut yeni, mut eski) = tokio::io::duplex(4096);
        let yoklama = Kontrol::SaatSor {
            izleyici_ms: SES_SOR,
        };
        yaz(&mut yeni, &yoklama).await.unwrap();
        assert_eq!(
            oku::<_, EskiKontrol>(&mut eski).await.unwrap(),
            Some(EskiKontrol::SaatSor {
                izleyici_ms: SES_SOR
            })
        );
        // Eski hostun normal saat yanıtı yetenek bildirmez.
        let simdi = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        yaz(
            &mut eski,
            &EskiKontrol::SaatCevap {
                izleyici_ms: SES_SOR,
                host_ms: simdi,
            },
        )
        .await
        .unwrap();
        let Some(Kontrol::SaatCevap { izleyici_ms, host_ms }) =
            oku::<_, Kontrol>(&mut yeni).await.unwrap() else { panic!("SaatCevap bekleniyordu") };
        let ses = AtomicBool::new(false);
        let fark = AtomicI64::new(42);
        let esitlendi = AtomicBool::new(false);
        crate::izleyici::saat_cevabini_isle(
            izleyici_ms, host_ms, 150, true, &ses, &fark, &esitlendi,
        );
        assert!(!ses.load(Ordering::SeqCst));
        let mut gonderilen = 0;
        for g in [
            Girdi::SesAc,
            Girdi::Tus {
                hid: 0,
                ad: "Space".into(),
                basili: true,
            },
            Girdi::Ses(vec![0; 640]),
            Girdi::Tus {
                hid: 0,
                ad: "Space".into(),
                basili: false,
            },
            Girdi::SesKapat,
        ] {
            if ses_gonderilebilir(ses.load(Ordering::SeqCst), &g) {
                yaz(&mut yeni, &Kontrol::Girdi(g)).await.unwrap();
                gonderilen += 1;
            }
        }
        drop(yeni);
        assert_eq!(gonderilen, 2);
        for basili in [true, false] {
            assert_eq!(
                oku::<_, EskiKontrol>(&mut eski).await.unwrap(),
                Some(EskiKontrol::Girdi(EskiGirdi::Tus {
                    hid: 0,
                    ad: "Space".into(),
                    basili
                }))
            );
        }
        assert_eq!(oku::<_, EskiKontrol>(&mut eski).await.unwrap(), None);
        assert!(ses_gonderilebilir(true, &Girdi::SesAc));
        assert!(ses_gonderilebilir(true, &Girdi::Ses(vec![0; 640])));
        assert!(ses_gonderilebilir(true, &Girdi::SesKapat));
    }

    #[tokio::test]
    async fn eski_istemci_yeni_host_ile_kopmaz() {
        let (mut host, mut eski) = tokio::io::duplex(4096);
        let izinler = Izinler {
            kontrol: true,
            pano: false,
            dosya: false,
            oyun_kolu: false,
        };
        let kabul = Kontrol::Kabul {
            izinler: izinler.clone(),
            genislik: 800,
            yukseklik: 600,
        };
        let eski_kabul = EskiKontrol::Kabul {
            izinler,
            genislik: 800,
            yukseklik: 600,
        };
        assert_eq!(
            bincode::serialize(&kabul).unwrap(),
            bincode::serialize(&eski_kabul).unwrap()
        );
        yaz(&mut host, &kabul).await.unwrap();
        assert_eq!(
            oku::<_, EskiKontrol>(&mut eski).await.unwrap(),
            Some(eski_kabul)
        );
        for izleyici_ms in [0, 1, 1720000000000, u64::MAX] {
            assert_eq!(yetenek_yaniti(izleyici_ms), None);
            yaz(
                &mut host,
                &Kontrol::SaatCevap {
                    izleyici_ms,
                    host_ms: 456,
                },
            )
            .await
            .unwrap();
            assert!(matches!(
                oku::<_, EskiKontrol>(&mut eski).await.unwrap(),
                Some(EskiKontrol::SaatCevap { .. })
            ));
        }
        yaz(&mut host, &Kontrol::Pano("hâlâ bağlı".into()))
            .await
            .unwrap();
        assert_eq!(
            oku::<_, EskiKontrol>(&mut eski).await.unwrap(),
            Some(EskiKontrol::Pano("hâlâ bağlı".into()))
        );
        // Yeni hostun yoklama yanıtını eski okuyucu da çözebilir.
        let yanit = yetenek_yaniti(SES_SOR).unwrap();
        assert_eq!(
            bincode::deserialize::<EskiKontrol>(&bincode::serialize(&yanit).unwrap()).unwrap(),
            EskiKontrol::SaatCevap {
                izleyici_ms: SES_SOR,
                host_ms: if cfg!(windows) { SES_VAR } else { 0 },
            }
        );
        assert_eq!(SURUM, 8);
    }

    #[test]
    fn ses_sozlesmesi_ve_eski_indeksler() {
        for g in [Girdi::SesAc, Girdi::Ses(vec![0; 640]), Girdi::SesKapat] {
            let m = Kontrol::Girdi(g);
            assert_eq!(
                bincode::deserialize::<Kontrol>(&bincode::serialize(&m).unwrap()).unwrap(),
                m
            );
        }
        assert_eq!(
            bincode::serialize(&Girdi::Metin("".into())).unwrap()[..4],
            4u32.to_le_bytes()
        );
        assert_eq!(
            bincode::serialize(&Girdi::SesAc).unwrap(),
            6u32.to_le_bytes()
        );
        assert_eq!(
            bincode::serialize(&Kontrol::TerminalListesi).unwrap(),
            19u32.to_le_bytes()
        );
        assert_eq!(
            &bincode::serialize(&Girdi::Ses(vec![])).unwrap()[..4],
            &7u32.to_le_bytes()
        );
        assert_eq!(bincode::serialize(&Girdi::SesKapat).unwrap(), 8u32.to_le_bytes());
    }

    #[test]
    fn terminal_mesajlari_gidis_donus() {
        let mesajlar = [
            Kontrol::TerminalListesi,
            Kontrol::TerminalPencereleri(vec![crate::terminal::Pencere {
                id: 42,
                baslik: "Türkçe — Codex".into(),
                uygulama: "Windows Terminal".into(),
                tur: "terminal".into(),
            }]),
            Kontrol::TerminalModu {
                ac: Some(42),
                en: 390,
                boy: 844,
            },
            Kontrol::TerminalModu {
                ac: None,
                en: 0,
                boy: 0,
            },
            Kontrol::TerminalDurum {
                acik: true,
                hata: None,
            },
            Kontrol::TerminalDurum {
                acik: false,
                hata: Some("Terminal kapandı, yeniden seç.".into()),
            },
        ];
        for mesaj in mesajlar {
            let baytlar = bincode::serialize(&mesaj).unwrap();
            assert_eq!(bincode::deserialize::<Kontrol>(&baytlar).unwrap(), mesaj);
        }
    }

    #[test]
    fn alpn_protokol_surumunu_tasir() {
        assert_eq!(ALPN, format!("afudesk/{SURUM}").as_bytes());
    }

    #[tokio::test]
    async fn akim_tavani_bos_yeni_akimi_reddeder() {
        let sayac = std::sync::Arc::new(tokio::sync::Semaphore::new(AZAMI_ESZAMANLI_AKIS));
        let izinler: Vec<_> = (0..AZAMI_ESZAMANLI_AKIS)
            .map(|_| sayac.clone().try_acquire_owned().unwrap())
            .collect();
        assert!(sayac.clone().try_acquire_owned().is_err());
        drop(izinler);
        assert!(sayac.clone().try_acquire_owned().is_ok());
    }

    #[tokio::test]
    async fn yaz_oku_gidis_donus() {
        let (mut a, mut b) = tokio::io::duplex(1 << 20);
        let m = Kontrol::Girdi(Girdi::Tus {
            hid: 0x00070033,
            ad: "ş".into(),
            basili: true,
        });
        yaz(&mut a, &m).await.unwrap();
        yaz(&mut a, &Kontrol::Kapat("bitti".into())).await.unwrap();
        drop(a);
        assert_eq!(oku::<_, Kontrol>(&mut b).await.unwrap(), Some(m));
        assert_eq!(
            oku::<_, Kontrol>(&mut b).await.unwrap(),
            Some(Kontrol::Kapat("bitti".into()))
        );
        assert_eq!(oku::<_, Kontrol>(&mut b).await.unwrap(), None);
    }

    #[tokio::test]
    async fn dev_uzunluk_reddedilir() {
        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&(u32::MAX).to_be_bytes()).await.unwrap();
        assert!(oku::<_, Kontrol>(&mut b).await.is_err());
    }

    #[tokio::test]
    async fn cop_veri_reddedilir() {
        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&4u32.to_be_bytes()).await.unwrap();
        a.write_all(&[0xff, 0xff, 0xff, 0xff]).await.unwrap();
        assert!(oku::<_, Kontrol>(&mut b).await.is_err());
    }
}
