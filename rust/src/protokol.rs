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
/// Yetenek mesajıyla karşılıklı etkinleştirilen hız uzantısı.
pub const HIZ_SURUM: u32 = 1;
/// Eski hostlarla da okunabilen yetenek sorgusu.
pub const HIZ_SOR: u64 = u64::MAX - 1;
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
        /// USB HID usage (sayfa ve kullanım birlikte); sıfırsa ad yedeği.
        hid: u32,
        ad: String,
        basili: bool,
    },
    /// Doğrudan metin yazma (mobil klavye için).
    Metin(String),
    Kol(KolDurumu),
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
    /// Google kanıtı yalnız şifreli, kimliği doğrulanmış QUIC akışında taşınır.
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
    /// Yalnız iki uç da ilan ederse yeni kare/kodek biçimleri kullanılır.
    Yetenekler {
        surum: u32,
    },
    GirdiZamanli {
        girdi: Girdi,
        izleyici_ms: u64,
    },
    Imlec(Imlec),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Imlec {
    pub x: f32,
    pub y: f32,
    pub gorunur: bool,
    pub gen: u32,
    pub yuk: u32,
    pub sicak_x: u32,
    pub sicak_y: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KarePaketi {
    pub kare: Kare,
    pub girdi_ms: u64,
    pub host_girdi_ms: u64,
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

/// Persistent paketli ve eski tek-kare akışlarını aynı uzunluk önekiyle okur.
/// Yalnız mesaj sınırındaki EOF normal kapanıştır.
pub async fn kare_oku<R: AsyncReadExt + Unpin>(r: &mut R) -> anyhow::Result<Option<KarePaketi>> {
    use bincode::Options;
    let mut uzunluk = [0u8; 4];
    if r.read(&mut uzunluk[..1]).await? == 0 {
        return Ok(None);
    }
    r.read_exact(&mut uzunluk[1..]).await?;
    let n = u32::from_be_bytes(uzunluk) as usize;
    anyhow::ensure!(n > 0 && n <= AZAMI_MESAJ, "geçersiz kare uzunluğu: {n}");
    let mut baytlar = vec![0; n];
    r.read_exact(&mut baytlar).await?;
    // serialize() uses fixed integers; DefaultOptions alone uses varints.
    let secenekler = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(n as u64)
        .reject_trailing_bytes();
    if let Ok(paket) = secenekler.deserialize::<KarePaketi>(&baytlar) {
        return Ok(Some(paket));
    }
    let kare = secenekler.deserialize::<Kare>(&baytlar)?;
    Ok(Some(KarePaketi {
        kare,
        girdi_ms: 0,
        host_girdi_ms: 0,
    }))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn terminal_mesajlari_gidis_donus() {
        let mesajlar = [
            Kontrol::TerminalListesi,
            Kontrol::TerminalPencereleri(vec![crate::terminal::Pencere {
                id: 42,
                baslik: "Türkçe — Codex".into(),
                uygulama: "Windows Terminal".into(),
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

#[cfg(test)]
mod hiz_uyumluluk_testleri {
    use super::*;
    #[test]
    fn eski_tel_baytlari_aynen_kalir() {
        assert_eq!(ALPN, b"afudesk/8");
        assert_eq!(
            bincode::serialize(&Kontrol::TerminalListesi).unwrap(),
            19u32.to_le_bytes()
        );
        assert_eq!(
            bincode::serialize(&Kontrol::Kapat("x".into())).unwrap(),
            vec![5, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'x']
        );
        let k = Kare {
            sira: 1,
            genislik: 2,
            yukseklik: 3,
            tam: true,
            dosemeler: vec![],
            yakalama_ms: 4,
        };
        let mut eski = Vec::new();
        eski.extend_from_slice(&1u64.to_le_bytes());
        eski.extend_from_slice(&2u32.to_le_bytes());
        eski.extend_from_slice(&3u32.to_le_bytes());
        eski.push(1);
        eski.extend_from_slice(&0u64.to_le_bytes());
        eski.extend_from_slice(&4u64.to_le_bytes());
        assert_eq!(bincode::serialize(&k).unwrap(), eski);
        assert_eq!(bincode::deserialize::<Kare>(&eski).unwrap(), k);
    }
    #[test]
    fn hiz_uzantilari_gidis_donus() {
        let m = Kontrol::GirdiZamanli {
            girdi: Girdi::Metin("çğıöşü".into()),
            izleyici_ms: 99,
        };
        assert_eq!(
            bincode::deserialize::<Kontrol>(&bincode::serialize(&m).unwrap()).unwrap(),
            m
        );
        assert_eq!(
            &bincode::serialize(&Kontrol::Yetenekler { surum: HIZ_SURUM }).unwrap()[..4],
            &23u32.to_le_bytes()
        );
    }
}

#[cfg(test)]
mod kare_okuma_testleri {
    use super::*;
    fn kare() -> Kare {
        Kare {
            sira: 7,
            genislik: 64,
            yukseklik: 64,
            tam: true,
            dosemeler: vec![],
            yakalama_ms: 12,
        }
    }
    #[tokio::test]
    async fn eski_host_karesi_okunur() {
        let k = kare();
        let mut veri = Vec::new();
        yaz(&mut veri, &k).await.unwrap();
        let mut r = veri.as_slice();
        assert_eq!(
            kare_oku(&mut r).await.unwrap(),
            Some(KarePaketi {
                kare: k,
                girdi_ms: 0,
                host_girdi_ms: 0
            })
        );
        assert!(kare_oku(&mut r).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn paketli_akista_iki_kare_okunur() {
        let a = KarePaketi {
            kare: kare(),
            girdi_ms: 10,
            host_girdi_ms: 11,
        };
        let mut b = a.clone();
        b.kare.sira += 1;
        let mut veri = Vec::new();
        yaz(&mut veri, &a).await.unwrap();
        yaz(&mut veri, &b).await.unwrap();
        let mut r = veri.as_slice();
        assert_eq!(kare_oku(&mut r).await.unwrap(), Some(a));
        assert_eq!(kare_oku(&mut r).await.unwrap(), Some(b));
        assert!(kare_oku(&mut r).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn bozuk_kare_ve_yarim_uzunluk_reddedilir() {
        let paket = KarePaketi {
            kare: kare(),
            girdi_ms: 10,
            host_girdi_ms: 11,
        };
        let mut eski_artik = bincode::serialize(&kare()).unwrap();
        eski_artik.push(0);
        let mut yeni_artik = bincode::serialize(&paket).unwrap();
        yeni_artik.push(0);
        let mut yeni_eksik = bincode::serialize(&paket).unwrap();
        yeni_eksik.pop();
        for baytlar in [eski_artik, yeni_artik, yeni_eksik, vec![255; 8]] {
            let mut veri = (baytlar.len() as u32).to_be_bytes().to_vec();
            veri.extend(baytlar);
            assert!(kare_oku(&mut veri.as_slice()).await.is_err());
        }
        for veri in [
            vec![0],
            vec![0, 0, 0],
            (AZAMI_MESAJ as u32 + 1).to_be_bytes().to_vec(),
            vec![0, 0, 0, 2, 0],
        ] {
            assert!(kare_oku(&mut veri.as_slice()).await.is_err());
        }
    }
}
