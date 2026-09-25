//! Uçtan uca: aynı süreçte host + izleyici, gerçek iroh/QUIC (yalnız 127.0.0.1), sahte ekran/girdi.
use crate::{
    host::{self, HostAyar, HostKomut, HostOlay},
    izleyici::{self, IzleyiciOlay},
    pano::SahtePano,
    platform::sahte::SahteFabrika,
    protokol::{FareTusu, Girdi, Izinler},
};
use std::{path::PathBuf, sync::Arc, time::Duration};

const SURE: Duration = Duration::from_secs(20);

fn ayar(parola: &str) -> HostAyar {
    HostAyar {
        ad: "Ali PC".into(),
        upnp: false,
        parola: parola.into(),
        yalniz_yerel: true,
        dosya_klasoru: None,
        veri_klasoru: None,
        kod_acik: true,
    }
}

async fn olay_bekle(h: &mut host::Host, f: impl Fn(&HostOlay) -> bool) -> HostOlay {
    tokio::time::timeout(SURE, async {
        loop {
            let o = h.olaylar.recv().await.expect("host kapandı");
            if f(&o) {
                return o;
            }
        }
    })
    .await
    .expect("host olayı gelmedi")
}

async fn iz_bekle(i: &mut izleyici::Izleyici, f: impl Fn(&IzleyiciOlay) -> bool) -> IzleyiciOlay {
    // Zaman aşımında teşhis için son görülen olaylar (kare içeriği atılır).
    let mut son: std::collections::VecDeque<String> = Default::default();
    let sonuc = tokio::time::timeout(SURE, async {
        loop {
            let Some(o) = i.olaylar.recv().await else {
                panic!("izleyici kapandı; son olaylar: {son:?}");
            };
            if f(&o) {
                return o;
            }
            let ozet = match &o {
                IzleyiciOlay::Kare { .. } => "Kare".to_owned(),
                d => format!("{d:?}"),
            };
            son.push_back(ozet);
            if son.len() > 8 {
                son.pop_front();
            }
        }
    })
    .await;
    match sonuc {
        Ok(o) => o,
        Err(_) => panic!("izleyici olayı gelmedi; son olaylar: {son:?}"),
    }
}

async fn hazir(h: &mut host::Host) -> (String, String) {
    match olay_bekle(h, |o| matches!(o, HostOlay::Hazir { .. })).await {
        HostOlay::Hazir { kod, parola, .. } => (kod, parola),
        _ => unreachable!(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn tam_akis_goruntu_ve_girdi() {
    let fab = Arc::new(SahteFabrika::new(160, 100));
    let girdiler = fab.girdiler.clone();
    let mut h = host::baslat(ayar("123456"), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    assert_eq!(parola, "123456");

    let mut i = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::OnayBekleniyor { .. })).await {
        IzleyiciOlay::OnayBekleniyor { karsi_ad } => assert_eq!(karsi_ad, "Ali PC"),
        _ => unreachable!(),
    }
    match olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await {
        HostOlay::Istek { ad, kayitli } => {
            assert_eq!(ad, "Veli");
            assert!(!kayitli, "kodla gelen istek kayıtlı sayılmaz");
        }
        _ => unreachable!(),
    }
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: true,
        pano: false,
        dosya: false,
        oyun_kolu: true,
    }))
    .await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await {
        IzleyiciOlay::Kabul {
            genislik,
            yukseklik,
            izinler,
        } => {
            assert_eq!((genislik, yukseklik), (160, 100));
            assert!(izinler.kontrol);
        }
        _ => unreachable!(),
    }
    // En az iki farklı kare gelmeli (sahte ekran sol üstü değiştiriyor).
    let mut goruler = Vec::new();
    while goruler.len() < 2 {
        if let IzleyiciOlay::Kare {
            genislik,
            yukseklik,
            rgba,
        } = iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await
        {
            assert_eq!((genislik, yukseklik), (160, 100));
            assert_eq!(rgba.len(), 160 * 100 * 4);
            // Sağ alt bölge sabit gri (128) olmalı — JPEG toleransı.
            let p = ((99 * 160 + 159) * 4) as usize;
            assert!((rgba[p] as i32 - 128).abs() < 20, "piksel {}", rgba[p]);
            goruler.push(rgba[0]);
        }
    }

    i.gonder(Girdi::FareKonum { x: 0.5, y: 0.25 }).await;
    i.gonder(Girdi::FareTus {
        tus: FareTusu::Sol,
        basili: true,
    })
    .await;
    i.gonder(Girdi::Tus {
        ad: "ş".into(),
        basili: true,
    })
    .await;
    tokio::time::timeout(SURE, async {
        while girdiler.lock().unwrap().len() < 3 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("girdiler hosta ulaşmadı");
    assert_eq!(
        girdiler.lock().unwrap()[0],
        Girdi::FareKonum { x: 0.5, y: 0.25 }
    );

    // Host keser → izleyici sebebini öğrenir, host yeni kod üretir (bilet tek kullanımlık).
    h.komut(HostKomut::Kes).await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Koptu { .. })).await {
        IzleyiciOlay::Koptu { sebep } => assert!(
            sebep.contains("kesti") || sebep.contains("kapandı"),
            "{sebep}"
        ),
        _ => unreachable!(),
    }
    let (kod2, _) = hazir(&mut h).await;
    assert_ne!(kod2, kod, "oturum sonrası yeni kod üretilmeli");
}

#[tokio::test(flavor = "multi_thread")]
async fn kontrol_izni_yoksa_girdi_uygulanmaz() {
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let girdiler = fab.girdiler.clone();
    let mut h = host::baslat(ayar("111111"), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: false,
        pano: false,
        dosya: false,
        oyun_kolu: false,
    }))
    .await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
    i.gonder(Girdi::FareTus {
        tus: FareTusu::Sol,
        basili: true,
    })
    .await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(girdiler.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn red_edilince_izleyici_sebebi_gorur() {
    let mut h = host::baslat(ayar("222222"), Arc::new(SahteFabrika::new(64, 64)))
        .await
        .unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Red).await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Koptu { .. })).await {
        IzleyiciOlay::Koptu { sebep } => assert_eq!(sebep, "Karşı taraf bağlantıyı reddetti."),
        _ => unreachable!(),
    }
    // Red sonrası aynı kod hâlâ geçerli: ikinci deneme yine istek üretir.
    let _i2 = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn yanlis_parola_baglanmadan_reddedilir() {
    let mut h = host::baslat(ayar("333333"), Arc::new(SahteFabrika::new(64, 64)))
        .await
        .unwrap();
    let (kod, _) = hazir(&mut h).await;
    let e = izleyici::baglan(&kod, "000000", "Veli")
        .await
        .err()
        .unwrap();
    assert_eq!(e.to_string(), "Parola yanlış.");
}

#[tokio::test(flavor = "multi_thread")]
async fn sahte_bilet_reddedilir() {
    // Saldırgan adresi ve parmak izini biliyor ama bileti bilmiyor (ör. eski kod).
    let mut h = host::baslat(ayar("444444"), Arc::new(SahteFabrika::new(64, 64)))
        .await
        .unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut d = crate::kod::coz(&kod, &parola, crate::kod::simdi()).unwrap();
    d.bilet = crate::kod::yeni_bilet();
    let sahte = crate::kod::kodla(&d, &parola).unwrap();
    let mut i = izleyici::baglan(&sahte, &parola, "Mallory").await.unwrap();
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Koptu { .. })).await {
        IzleyiciOlay::Koptu { sebep } => {
            assert_eq!(sebep, "Kod geçersiz ya da daha önce kullanılmış.")
        }
        _ => unreachable!(),
    }
    // Host kullanıcıya istek GÖSTERMEMELİ.
    let ek = tokio::time::timeout(Duration::from_millis(500), h.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::Istek { .. }))));
}

#[tokio::test(flavor = "multi_thread")]
async fn host_durdurulunca_baglanilamaz() {
    let mut h = host::baslat(ayar("555555"), Arc::new(SahteFabrika::new(64, 64)))
        .await
        .unwrap();
    let (kod, parola) = hazir(&mut h).await;
    h.durdur();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let e = izleyici::baglan(&kod, &parola, "Veli")
        .await
        .err()
        .unwrap()
        .to_string();
    // Paralel testlerde boşalan portu başka bir host alabilir: o zaman parmak izi
    // uyuşmaz ve bağlantı güvenlik kontrolünde reddedilir — bu da doğru davranış.
    assert!(
        e.starts_with("Karşı bilgisayara ulaşılamadı.")
            || e.starts_with("Güvenlik kontrolü başarısız"),
        "{e}"
    );
}

/// Pano testleri için: kabul edilmiş oturum; izleyici olayları arka planda boşaltılır,
/// gelen `Pano` olayları listede toplanır.
struct PanoOturumu {
    _h: host::Host,
    _i: izleyici::Izleyici,
    host_pano: SahtePano,
    iz_pano: SahtePano,
    iz_olaylari: Arc<std::sync::Mutex<Vec<String>>>,
}

async fn pano_oturumu(parola: &str, pano_izni: bool) -> PanoOturumu {
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let host_pano = fab.pano.clone();
    host_pano.ayarla("host başlangıç");
    let iz_pano = SahtePano::default();
    iz_pano.ayarla("izleyici başlangıç");
    let mut h = host::baslat(ayar(parola), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan_panolu(&kod, &parola, "Veli", Some(Box::new(iz_pano.clone())))
        .await
        .unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: false,
        pano: pano_izni,
        dosya: false,
        oyun_kolu: false,
    }))
    .await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await {
        IzleyiciOlay::Kabul { izinler, .. } => assert_eq!(izinler.pano, pano_izni),
        _ => unreachable!(),
    }
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
    let iz_olaylari: Arc<std::sync::Mutex<Vec<String>>> = Default::default();
    let mut alici = std::mem::replace(&mut i.olaylar, tokio::sync::mpsc::channel(1).1);
    let liste = iz_olaylari.clone();
    tokio::spawn(async move {
        while let Some(o) = alici.recv().await {
            if let IzleyiciOlay::Pano(m) = o {
                liste.lock().unwrap().push(m);
            }
        }
    });
    PanoOturumu {
        _h: h,
        _i: i,
        host_pano,
        iz_pano,
        iz_olaylari,
    }
}

async fn kosul_bekle(ne: &str, f: impl Fn() -> bool) {
    tokio::time::timeout(SURE, async {
        while !f() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{ne}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn pano_iki_yonlu_gider() {
    let o = pano_oturumu("610610", true).await;
    // Oturum başındaki içerik kendiliğinden gitmez.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert_eq!(o.iz_pano.icerik().as_deref(), Some("izleyici başlangıç"));
    assert_eq!(o.host_pano.icerik().as_deref(), Some("host başlangıç"));

    // Host → izleyici.
    o.host_pano.ayarla("hosttan merhaba ş");
    kosul_bekle("host panosu izleyiciye gitmedi", || {
        o.iz_pano.icerik().as_deref() == Some("hosttan merhaba ş")
    })
    .await;
    kosul_bekle("izleyici arayüzüne Pano olayı gelmedi", || {
        o.iz_olaylari
            .lock()
            .unwrap()
            .contains(&"hosttan merhaba ş".to_owned())
    })
    .await;

    // İzleyici → host.
    o.iz_pano.ayarla("izleyiciden selam");
    kosul_bekle("izleyici panosu hosta gitmedi", || {
        o.host_pano.icerik().as_deref() == Some("izleyiciden selam")
    })
    .await;
    assert_eq!(
        o.host_pano.yazilanlar(),
        vec!["izleyiciden selam".to_owned()]
    );
    assert_eq!(o.iz_pano.yazilanlar(), vec!["hosttan merhaba ş".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn pano_izni_yoksa_gitmez() {
    let o = pano_oturumu("620620", false).await;
    o.host_pano.ayarla("gizli host metni");
    o.iz_pano.ayarla("gizli izleyici metni");
    tokio::time::sleep(Duration::from_millis(2000)).await;
    assert!(
        o.host_pano.yazilanlar().is_empty(),
        "izinsiz host panosuna yazıldı"
    );
    assert!(
        o.iz_pano.yazilanlar().is_empty(),
        "izinsiz izleyici panosuna yazıldı"
    );
    assert_eq!(o.host_pano.icerik().as_deref(), Some("gizli host metni"));
    assert_eq!(o.iz_pano.icerik().as_deref(), Some("gizli izleyici metni"));
    assert!(
        o.iz_olaylari.lock().unwrap().is_empty(),
        "izinsiz Pano olayı geldi"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn pano_dongu_yapmaz() {
    let o = pano_oturumu("630630", true).await;
    o.host_pano.ayarla("tek sefer");
    kosul_bekle("host panosu izleyiciye gitmedi", || {
        o.iz_pano.icerik().as_deref() == Some("tek sefer")
    })
    .await;
    // Birkaç yoklama turu bekle: izleyici gelen metni geri yollarsa host panosuna yazılır.
    tokio::time::sleep(Duration::from_millis(2000)).await;
    assert!(
        o.host_pano.yazilanlar().is_empty(),
        "gelen metin hosta geri yankılandı: {:?}",
        o.host_pano.yazilanlar()
    );
    assert_eq!(
        o.iz_pano.yazilanlar(),
        vec!["tek sefer".to_owned()],
        "izleyiciye tekrar tekrar yazıldı"
    );

    // Ters yön de yankılanmamalı.
    o.iz_pano.ayarla("geri yön");
    kosul_bekle("izleyici panosu hosta gitmedi", || {
        o.host_pano.icerik().as_deref() == Some("geri yön")
    })
    .await;
    tokio::time::sleep(Duration::from_millis(2000)).await;
    assert_eq!(
        o.iz_pano.yazilanlar(),
        vec!["tek sefer".to_owned()],
        "host gelen metni izleyiciye geri yolladı"
    );
    assert_eq!(o.host_pano.yazilanlar(), vec!["geri yön".to_owned()]);
    assert_eq!(
        o.iz_olaylari.lock().unwrap().as_slice(),
        ["tek sefer".to_owned()]
    );
}
// --- dosya aktarımı ---

fn gecici_klasor(on: &str) -> PathBuf {
    let k = std::env::temp_dir().join(format!("afudesk_{on}_{:016x}", rand::random::<u64>()));
    std::fs::create_dir_all(&k).unwrap();
    k
}

fn rastgele_icerik(n: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut v = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut v);
    v
}

fn sha(b: &[u8]) -> Vec<u8> {
    use sha2::Digest;
    sha2::Sha256::digest(b).to_vec()
}

/// Klasördeki dosya adları (sıralı).
fn icindekiler(k: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = match std::fs::read_dir(k) {
        Ok(d) => d
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

/// Host (alınan dosyalar `hedef` klasörüne) + kabul edilmiş izleyici.
async fn dosya_oturumu(
    parola: &str,
    dosya_izni: bool,
    hedef: &std::path::Path,
) -> (host::Host, izleyici::Izleyici) {
    let mut a = ayar(parola);
    a.dosya_klasoru = Some(hedef.to_owned());
    let mut h = host::baslat(a, Arc::new(SahteFabrika::new(64, 64)))
        .await
        .unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: false,
        pano: false,
        dosya: dosya_izni,
        oyun_kolu: false,
    }))
    .await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await {
        IzleyiciOlay::Kabul { izinler, .. } => assert_eq!(izinler.dosya, dosya_izni),
        _ => unreachable!(),
    }
    (h, i)
}

/// Gönderimin son olayı: (gönderilen, toplam, hata, aradaki ilerlemeler).
async fn dosya_sonu(i: &mut izleyici::Izleyici) -> (u64, u64, String, Vec<u64>) {
    let mut ara = Vec::new();
    loop {
        match iz_bekle(i, |o| matches!(o, IzleyiciOlay::Dosya { .. })).await {
            IzleyiciOlay::Dosya {
                gonderilen,
                toplam,
                bitti: true,
                hata,
                ..
            } => return (gonderilen, toplam, hata, ara),
            IzleyiciOlay::Dosya { gonderilen, .. } => ara.push(gonderilen),
            _ => unreachable!(),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn dosya_aktarilir() {
    let hedef = gecici_klasor("hedef");
    let kaynak = gecici_klasor("kaynak");
    let icerik = rastgele_icerik(3 * 1024 * 1024);
    let yol = kaynak.join("veri.bin");
    std::fs::write(&yol, &icerik).unwrap();
    let (mut h, mut i) = dosya_oturumu("600001", true, &hedef).await;

    let gorev = i.dosya_gonder(&yol);
    let (gonderilen, toplam, hata, ara) = dosya_sonu(&mut i).await;
    assert_eq!(hata, "");
    assert_eq!(
        (gonderilen, toplam),
        (icerik.len() as u64, icerik.len() as u64)
    );
    assert!(
        ara.windows(2).all(|w| w[0] <= w[1]),
        "ilerleme geri gitmemeli: {ara:?}"
    );
    let g = gorev.await.unwrap().unwrap();
    assert_eq!((g.baslangic, g.gonderilen), (0, icerik.len() as u64));

    let (ad, alinan) = match olay_bekle(&mut h, |o| matches!(o, HostOlay::DosyaAlindi { .. })).await
    {
        HostOlay::DosyaAlindi { ad, yol } => (ad, PathBuf::from(yol)),
        _ => unreachable!(),
    };
    assert_eq!(ad, "veri.bin");
    assert_eq!(alinan, hedef.join("veri.bin"));
    let gelen = std::fs::read(&alinan).unwrap();
    assert!(gelen == icerik, "içerik birebir aynı olmalı");
    assert_eq!(sha(&gelen), sha(&icerik));
    assert_eq!(icindekiler(&hedef), ["veri.bin"], "yarım dosya kalmamalı");

    // Aynı dosya ikinci kez: üzerine yazılmaz, numaralı ad alır.
    i.dosya_gonder(&yol);
    assert_eq!(dosya_sonu(&mut i).await.2, "");
    assert_eq!(icindekiler(&hedef), ["veri (1).bin", "veri.bin"]);
    assert!(std::fs::read(hedef.join("veri (1).bin")).unwrap() == icerik);

    let _ = std::fs::remove_dir_all(&hedef);
    let _ = std::fs::remove_dir_all(&kaynak);
}

#[tokio::test(flavor = "multi_thread")]
async fn dosya_kaldigi_yerden_devam() {
    let hedef = gecici_klasor("hedef");
    let kaynak = gecici_klasor("kaynak");
    let icerik = rastgele_icerik(3 * 1024 * 1024);
    let yol = kaynak.join("yarim.bin");
    std::fs::write(&yol, &icerik).unwrap();
    let (mut h, mut i) = dosya_oturumu("600002", true, &hedef).await;

    // 1) Aktarım 1 000 000 bayttan sonra kesilir: host yazdığını saklar, asıl ad oluşmaz.
    const KES: u64 = 1_000_000;
    let gorev = i.dosya_gonder_ayarli(
        yol.clone(),
        crate::dosya::GonderAyari {
            kes: Some(KES),
            sha_boz: false,
        },
    );
    let (_, _, hata, _) = dosya_sonu(&mut i).await;
    assert_eq!(hata, crate::dosya::YARIDA_KALDI);
    assert!(gorev.await.unwrap().is_err());
    let yarimlar = icindekiler(&hedef);
    assert_eq!(yarimlar.len(), 1, "{yarimlar:?}");
    assert!(yarimlar[0].ends_with(".afudesk-parca"), "{yarimlar:?}");
    assert_eq!(
        std::fs::metadata(hedef.join(&yarimlar[0])).unwrap().len(),
        KES
    );
    assert!(!hedef.join("yarim.bin").exists());

    // 2) Yeniden gönder: yalnız kalan kısım gider.
    let gorev = i.dosya_gonder(&yol);
    let (gonderilen, toplam, hata, ara) = dosya_sonu(&mut i).await;
    assert_eq!(hata, "");
    assert_eq!(
        (gonderilen, toplam),
        (icerik.len() as u64, icerik.len() as u64)
    );
    assert!(
        ara.iter().all(|&g| g >= KES),
        "ilerleme devam noktasından başlamalı: {ara:?}"
    );
    let g = gorev.await.unwrap().unwrap();
    assert_eq!(g.baslangic, KES);
    assert_eq!(
        g.gonderilen,
        icerik.len() as u64 - KES,
        "yalnız kalan bayt gönderilmeli"
    );

    match olay_bekle(&mut h, |o| matches!(o, HostOlay::DosyaAlindi { .. })).await {
        HostOlay::DosyaAlindi { ad, .. } => assert_eq!(ad, "yarim.bin"),
        _ => unreachable!(),
    }
    let gelen = std::fs::read(hedef.join("yarim.bin")).unwrap();
    assert!(gelen == icerik, "içerik birebir aynı olmalı");
    assert_eq!(sha(&gelen), sha(&icerik));
    assert_eq!(
        icindekiler(&hedef),
        ["yarim.bin"],
        "yarım dosya temizlenmeli"
    );

    let _ = std::fs::remove_dir_all(&hedef);
    let _ = std::fs::remove_dir_all(&kaynak);
}

#[tokio::test(flavor = "multi_thread")]
async fn dosya_izni_yoksa_reddedilir() {
    let hedef = gecici_klasor("hedef");
    let kaynak = gecici_klasor("kaynak");
    let yol = kaynak.join("gizli.txt");
    std::fs::write(&yol, b"merhaba").unwrap();
    let (mut h, mut i) = dosya_oturumu("600003", false, &hedef).await;

    let gorev = i.dosya_gonder(&yol);
    let (_, _, hata, _) = dosya_sonu(&mut i).await;
    assert_eq!(hata, "Karşı taraf dosya almaya izin vermedi.");
    assert_eq!(
        gorev.await.unwrap().unwrap_err().to_string(),
        "Karşı taraf dosya almaya izin vermedi."
    );
    assert!(icindekiler(&hedef).is_empty(), "hiçbir şey yazılmamalı");
    let ek = tokio::time::timeout(Duration::from_millis(300), h.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::DosyaAlindi { .. }))));
    // Oturum reddedilen dosyadan etkilenmez: görüntü akmaya devam eder.
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;

    let _ = std::fs::remove_dir_all(&hedef);
    let _ = std::fs::remove_dir_all(&kaynak);
}

#[tokio::test(flavor = "multi_thread")]
async fn dosya_bozulursa_hata() {
    let hedef = gecici_klasor("hedef");
    let kaynak = gecici_klasor("kaynak");
    let icerik = rastgele_icerik(600 * 1024);
    let yol = kaynak.join("bozuk.bin");
    std::fs::write(&yol, &icerik).unwrap();
    let (mut h, mut i) = dosya_oturumu("600004", true, &hedef).await;

    let gorev = i.dosya_gonder_ayarli(
        yol.clone(),
        crate::dosya::GonderAyari {
            kes: None,
            sha_boz: true,
        },
    );
    let (_, _, hata, _) = dosya_sonu(&mut i).await;
    assert_eq!(hata, crate::dosya::BOZUK);
    assert!(gorev.await.unwrap().is_err());
    assert!(!hedef.join("bozuk.bin").exists(), "asıl ad oluşmamalı");
    assert!(
        icindekiler(&hedef).is_empty(),
        "bozuk yarım dosya da silinmeli: {:?}",
        icindekiler(&hedef)
    );
    let ek = tokio::time::timeout(Duration::from_millis(300), h.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::DosyaAlindi { .. }))));

    // Aynı dosya doğru özetle gönderilince sorunsuz gelir.
    i.dosya_gonder(&yol);
    assert_eq!(dosya_sonu(&mut i).await.2, "");
    assert_eq!(
        sha(&std::fs::read(hedef.join("bozuk.bin")).unwrap()),
        sha(&icerik)
    );

    let _ = std::fs::remove_dir_all(&hedef);
    let _ = std::fs::remove_dir_all(&kaynak);
}

#[tokio::test(flavor = "multi_thread")]
async fn disaridan_kol_durumu_hosta_ulasir() {
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let durumlar = fab.kol_durumlari.clone();
    let kaldirilan = fab.kol_kaldirilan.clone();
    let mut h = host::baslat(ayar("606060"), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let i = izleyici::baglan(&kod, &parola, "Oyuncu").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: false,
        pano: false,
        dosya: false,
        oyun_kolu: true,
    }))
    .await;
    i.gonder(Girdi::Kol(crate::protokol::KolDurumu {
        slot: 0,
        sira: 1,
        dugmeler: 0x1000,
        sol_x: 1234,
        sol_y: -2345,
        sag_x: 32767,
        sag_y: -32767,
        sol_tetik: 120,
        sag_tetik: 240,
    }))
    .await;
    tokio::time::timeout(SURE, async {
        while durumlar.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(durumlar.lock().unwrap()[0].dugmeler, 0x1000);
    assert_eq!(durumlar.lock().unwrap()[0].sol_x, 1234);
    assert_eq!(durumlar.lock().unwrap()[0].sol_y, -2345);
    assert_eq!(durumlar.lock().unwrap()[0].sag_x, 32767);
    assert_eq!(durumlar.lock().unwrap()[0].sag_y, -32767);
    assert_eq!(durumlar.lock().unwrap()[0].sol_tetik, 120);
    assert_eq!(durumlar.lock().unwrap()[0].sag_tetik, 240);
    i.kapat();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Koptu { .. })).await;
    assert_eq!(
        *kaldirilan.lock().unwrap(),
        vec![0],
        "oturum sonunda sanal kol kaldırılmalı"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn oyun_kolu_izni_yoksa_uygulanmaz() {
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let girdiler = fab.girdiler.clone();
    let durumlar = fab.kol_durumlari.clone();
    let mut h = host::baslat(ayar("616161"), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let i = izleyici::baglan(&kod, &parola, "Oyuncu").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: true,
        pano: false,
        dosya: false,
        oyun_kolu: false,
    }))
    .await;
    i.gonder(Girdi::Kol(crate::protokol::KolDurumu {
        slot: 0,
        sira: 1,
        dugmeler: 1,
        sol_x: 0,
        sol_y: 0,
        sag_x: 0,
        sag_y: 0,
        sol_tetik: 0,
        sag_tetik: 0,
    }))
    .await;
    i.gonder(Girdi::FareTus {
        tus: FareTusu::Sol,
        basili: true,
    })
    .await;
    // Fare girdisi ulaşana kadar bekle (sabit bekleme yük altında yetmeyebilir); kol
    // girdisi fareden önce gönderildiği için o ana kadar ulaşmadıysa hiç uygulanmamıştır.
    kosul_bekle("fare girdisi", || !girdiler.lock().unwrap().is_empty()).await;
    assert!(durumlar.lock().unwrap().is_empty());
    assert_eq!(
        *girdiler.lock().unwrap(),
        vec![Girdi::FareTus {
            tus: FareTusu::Sol,
            basili: true
        }],
        "fare kontrol izni çalışmaya devam etmeli"
    );
    h.komut(HostKomut::Kes).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn titresim_izleyiciye_doner() {
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let titresim = fab.kol_titresim.clone();
    let mut h = host::baslat(ayar("626262"), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Oyuncu").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: true,
        pano: false,
        dosya: false,
        oyun_kolu: true,
    }))
    .await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await;
    titresim.lock().unwrap().push((0, 180, 75));
    assert!(matches!(
        iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Titresim { .. })).await,
        IzleyiciOlay::Titresim {
            slot: 0,
            buyuk: 180,
            kucuk: 75
        }
    ));
    h.komut(HostKomut::Kes).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn eski_sirali_kol_durumu_atilir() {
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let durumlar = fab.kol_durumlari.clone();
    let kaldirilan = fab.kol_kaldirilan.clone();
    let mut h = host::baslat(ayar("636363"), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Oyuncu").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: false,
        pano: false,
        dosya: false,
        oyun_kolu: true,
    }))
    .await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await;
    let kol = |sira| {
        Girdi::Kol(crate::protokol::KolDurumu {
            slot: 0,
            sira,
            dugmeler: sira as u16,
            sol_x: 0,
            sol_y: 0,
            sag_x: 0,
            sag_y: 0,
            sol_tetik: 0,
            sag_tetik: 0,
        })
    };
    i.gonder(kol(10)).await;
    tokio::time::timeout(SURE, async {
        while durumlar.lock().unwrap().last().map(|d| d.sira) != Some(10) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    i.gonder(kol(9)).await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        durumlar
            .lock()
            .unwrap()
            .iter()
            .map(|d| d.sira)
            .collect::<Vec<_>>(),
        vec![10]
    );
    i.kapat();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Koptu { .. })).await;
    assert_eq!(*kaldirilan.lock().unwrap(), vec![0]);
}

/// Onaylı, kare akan bir oturum kurar.
async fn akan_oturum(fab: Arc<SahteFabrika>, parola: &str) -> (host::Host, izleyici::Izleyici) {
    let mut h = host::baslat(ayar(parola), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: true,
        pano: false,
        dosya: false,
        oyun_kolu: false,
    }))
    .await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
    (h, i)
}

#[tokio::test(flavor = "multi_thread")]
async fn kopunca_onay_sormadan_yeniden_baglanir_goruntu_ve_girdi_surer() {
    let fab = Arc::new(SahteFabrika::new(96, 64));
    let girdiler = fab.girdiler.clone();
    let (mut h, mut i) = akan_oturum(fab, "424242").await;
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Baglandi { .. })).await;

    // Ağ değişti: bağlantı düşer, izleyici kendiliğinden geri döner.
    i.yeniden_baglan();
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::YenidenBaglaniyor { .. })).await;
    // Host yeniden onay sormamalı: Istek gelmeden Baglandi gelmeli.
    let o = olay_bekle(&mut h, |o| {
        matches!(o, HostOlay::Istek { .. } | HostOlay::Baglandi { .. })
    })
    .await;
    assert!(matches!(o, HostOlay::Baglandi { .. }), "onay yeniden soruldu: {o:?}");
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;

    // Girdi yeni bağlantıdan ulaşır.
    let once = girdiler.lock().unwrap().len();
    i.gonder(Girdi::FareKonum { x: 0.1, y: 0.9 }).await;
    tokio::time::timeout(SURE, async {
        while girdiler.lock().unwrap().len() <= once {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("yeniden bağlandıktan sonra girdi ulaşmadı");
    assert_eq!(
        girdiler.lock().unwrap().last().cloned(),
        Some(Girdi::FareKonum { x: 0.1, y: 0.9 })
    );

    // İkinci kopuş da atlatılır (jeton her dönüşte yenilenir).
    i.yeniden_baglan();
    let o = olay_bekle(&mut h, |o| {
        matches!(o, HostOlay::Istek { .. } | HostOlay::Baglandi { .. })
    })
    .await;
    assert!(matches!(o, HostOlay::Baglandi { .. }), "{o:?}");
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn izleyici_kapatinca_yeniden_baglanmaz_host_yeni_kod_uretir() {
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let (mut h, mut i) = akan_oturum(fab, "515151").await;
    i.kapat();
    match iz_bekle(&mut i, |o| {
        matches!(o, IzleyiciOlay::Koptu { .. } | IzleyiciOlay::YenidenBaglaniyor { .. })
    })
    .await
    {
        IzleyiciOlay::Koptu { .. } => {}
        o => panic!("kapatılan izleyici yeniden bağlanmaya çalıştı: {o:?}"),
    }
    // Host devam beklemeden oturumu bitirir ve yeni kod verir.
    let o = olay_bekle(&mut h, |o| {
        matches!(o, HostOlay::Koptu { .. } | HostOlay::YenidenBekleniyor)
    })
    .await;
    assert!(matches!(o, HostOlay::Koptu { .. }), "{o:?}");
    hazir(&mut h).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn eski_bilet_devam_yerine_kullanilamaz() {
    // Kopuştan sonra host devam penceresinde; koddaki (kullanılmış) biletle yeni
    // bağlantı reddedilmeli.
    let fab = Arc::new(SahteFabrika::new(64, 64));
    let mut h = host::baslat(ayar("626262"), fab).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: false,
        pano: false,
        dosya: false,
        oyun_kolu: false,
    }))
    .await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
    i.yeniden_baglan();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Baglandi { .. })).await;
    // Oturum sürerken: kullanılmış biletle giren olmamalı. Host oturumdayken yeni bağlantı
    // ya hiç kurulamaz, ya kurulur ama yanıt almaz, ya da reddedilir — asla Kabul gelmez.
    let deneme = async {
        let mut ikinci = izleyici::baglan(&kod, &parola, "Hırsız").await.ok()?;
        loop {
            match ikinci.olaylar.recv().await? {
                o @ IzleyiciOlay::Kabul { .. } => return Some(o),
                IzleyiciOlay::Koptu { .. } => return None,
                _ => {}
            }
        }
    };
    if let Ok(Some(o)) = tokio::time::timeout(Duration::from_secs(8), deneme).await {
        panic!("kullanılmış biletle girildi: {o:?}");
    }
    // Asıl izleyici etkilenmeden sürer.
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
}

// ---------------------------------------------------------------- kayıtlı cihazlar

fn kayitli_ayar(parola: &str, veri: &std::path::Path) -> HostAyar {
    let mut a = ayar(parola);
    a.veri_klasoru = Some(veri.to_owned());
    a
}

fn izinler_kontrollu() -> Izinler {
    Izinler {
        kontrol: true,
        pano: false,
        dosya: false,
        oyun_kolu: false,
    }
}

/// Kodla bağlanıp onaylanan ilk oturum: izleyici kaydedilir. Oturum kapatılır; host
/// yeni kod üretene kadar beklenir. Dönen: izleyicinin kaydı ve host'un yeni kodu/parolası.
async fn eslestir_kodlu(
    h: &mut host::Host,
    izleyici_veri: &std::path::Path,
) -> (crate::guvenilen::IzleyiciKaydi, (String, String)) {
    let (kod, parola) = hazir(h).await;
    let mut i = izleyici::baglan_ayarli(&kod, &parola, "Veli", None, Some(izleyici_veri))
        .await
        .unwrap();
    olay_bekle(h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(izinler_kontrollu())).await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kaydedildi { .. })).await {
        IzleyiciOlay::Kaydedildi { ad } => assert_eq!(ad, "Ali PC"),
        _ => unreachable!(),
    }
    let kayitlar = crate::guvenilen::oku(&crate::guvenilen::kayit_yolu(izleyici_veri));
    assert_eq!(kayitlar.izleyiciler.len(), 1);
    i.kapat();
    olay_bekle(h, |o| matches!(o, HostOlay::Koptu { .. })).await;
    let yeni_kod = hazir(h).await;
    (kayitlar.izleyiciler[0].clone(), yeni_kod)
}

async fn eslestir(
    h: &mut host::Host,
    izleyici_veri: &std::path::Path,
) -> crate::guvenilen::IzleyiciKaydi {
    eslestir_kodlu(h, izleyici_veri).await.0
}

/// Kayıtlı bağlantının ilk olayı: Kabul ya da Koptu (sebep).
async fn kayitli_sonuc(i: &mut izleyici::Izleyici) -> IzleyiciOlay {
    iz_bekle(i, |o| {
        matches!(o, IzleyiciOlay::Kabul { .. } | IzleyiciOlay::Koptu { .. })
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn kayitli_cihaz_kodsuz_baglanir() {
    let (hv, iv) = (gecici_klasor("host_veri"), gecici_klasor("iz_veri"));
    let fab = Arc::new(SahteFabrika::new(64, 48));
    let girdiler = fab.girdiler.clone();
    let mut h = host::baslat(kayitli_ayar("111222", &hv), fab).await.unwrap();
    let kayit = eslestir(&mut h, &iv).await;
    assert_eq!(kayit.host_kimlik, h.kimlik(), "kayıt host'un kalıcı kimliğine bağlı");
    // Host da izleyiciyi hatırlıyor (jetonun kendisi değil, özeti).
    let host_kayitlari = crate::guvenilen::oku(&crate::guvenilen::kayit_yolu(&hv));
    assert_eq!(host_kayitlari.hostlar.len(), 1);
    assert_ne!(host_kayitlari.hostlar[0].jeton_sha256, kayit.jeton);

    // Kod ve parola OLMADAN: yalnız kayıtla bağlan, host yine onay verir.
    let mut i = izleyici::kayitli_baglan(&kayit.host_kimlik, &iv, "Veli", None)
        .await
        .unwrap();
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::OnayBekleniyor { .. })).await {
        IzleyiciOlay::OnayBekleniyor { karsi_ad } => assert_eq!(karsi_ad, "Ali PC"),
        _ => unreachable!(),
    }
    match olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await {
        HostOlay::Istek { ad, kayitli } => {
            assert_eq!(ad, "Veli");
            assert!(kayitli, "onay penceresinde 'kayıtlı cihaz' rozeti için");
        }
        _ => unreachable!(),
    }
    h.komut(HostKomut::Kabul(izinler_kontrollu())).await;
    assert!(matches!(kayitli_sonuc(&mut i).await, IzleyiciOlay::Kabul { .. }));
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
    i.gonder(Girdi::FareKonum { x: 0.2, y: 0.3 }).await;
    kosul_bekle("kayıtlı oturumda girdi", || !girdiler.lock().unwrap().is_empty()).await;
    // İkinci kez kayıtlı bağlantıda yeni jeton verilmez (kayıt tek kalır).
    assert_eq!(crate::guvenilen::oku(&crate::guvenilen::kayit_yolu(&hv)).hostlar.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn kayitli_cihaz_onaysiz_baglanamaz() {
    let (hv, iv) = (gecici_klasor("host_veri"), gecici_klasor("iz_veri"));
    let mut h = host::baslat(kayitli_ayar("333444", &hv), Arc::new(SahteFabrika::new(64, 48)))
        .await
        .unwrap();
    let kayit = eslestir(&mut h, &iv).await;
    let mut i = izleyici::kayitli_baglan(&kayit.host_kimlik, &iv, "Veli", None)
        .await
        .unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { kayitli: true, .. })).await;
    h.komut(HostKomut::Red).await;
    match kayitli_sonuc(&mut i).await {
        IzleyiciOlay::Koptu { sebep } => assert!(sebep.contains("reddetti"), "{sebep}"),
        o => panic!("onay verilmeden bağlandı: {o:?}"),
    }
    // Reddetmek kaydı silmez: bir dahaki sefere yine onay istenebilir.
    assert_eq!(crate::guvenilen::oku(&crate::guvenilen::kayit_yolu(&iv)).izleyiciler.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn kaldirilan_cihaz_reddedilir() {
    let (hv, iv) = (gecici_klasor("host_veri"), gecici_klasor("iz_veri"));
    let mut h = host::baslat(kayitli_ayar("555666", &hv), Arc::new(SahteFabrika::new(64, 48)))
        .await
        .unwrap();
    let kayit = eslestir(&mut h, &iv).await;
    // Host kullanıcısı "Güvenilen cihazlar"dan kaldırır.
    let izleyici_kimlik = crate::guvenilen::oku(&crate::guvenilen::kayit_yolu(&hv)).hostlar[0]
        .izleyici_kimlik
        .clone();
    crate::guvenilen::guncelle(&crate::guvenilen::kayit_yolu(&hv), |k| {
        crate::guvenilen::hosttan_kaldir(k, &izleyici_kimlik)
    })
    .unwrap();
    let mut i = izleyici::kayitli_baglan(&kayit.host_kimlik, &iv, "Veli", None)
        .await
        .unwrap();
    match kayitli_sonuc(&mut i).await {
        IzleyiciOlay::Koptu { sebep } => assert_eq!(sebep, crate::guvenilen::KALDIRILDI),
        o => panic!("kaldırılan cihaz bağlandı: {o:?}"),
    }
    // Onay penceresi hiç açılmadı; izleyicinin kaydı kendiliğinden silindi.
    let ek = tokio::time::timeout(Duration::from_millis(300), h.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::Istek { .. }))));
    kosul_bekle("izleyici kaydı silinmeli", || {
        crate::guvenilen::oku(&crate::guvenilen::kayit_yolu(&iv)).izleyiciler.is_empty()
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn yanlis_jeton_reddedilir() {
    let (hv, iv) = (gecici_klasor("host_veri"), gecici_klasor("iz_veri"));
    let mut h = host::baslat(kayitli_ayar("777888", &hv), Arc::new(SahteFabrika::new(64, 48)))
        .await
        .unwrap();
    let kayit = eslestir(&mut h, &iv).await;
    crate::guvenilen::guncelle(&crate::guvenilen::kayit_yolu(&iv), |k| {
        k.izleyiciler[0].jeton = crate::guvenilen::yeni_jeton();
    })
    .unwrap();
    let mut i = izleyici::kayitli_baglan(&kayit.host_kimlik, &iv, "Veli", None)
        .await
        .unwrap();
    match kayitli_sonuc(&mut i).await {
        IzleyiciOlay::Koptu { sebep } => assert_eq!(sebep, crate::guvenilen::GECERSIZ),
        o => panic!("yanlış jetonla bağlandı: {o:?}"),
    }
    let ek = tokio::time::timeout(Duration::from_millis(300), h.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::Istek { .. }))), "onay sorulmamalı");

    // Jeton doğru ama BAŞKA cihaz (farklı izleyici kimliği) kullanırsa da reddedilir.
    let hirsiz = gecici_klasor("hirsiz_veri");
    let mut dogru = kayit.clone();
    dogru.jeton = kayit.jeton.clone();
    crate::guvenilen::yaz(
        &crate::guvenilen::kayit_yolu(&hirsiz),
        &crate::guvenilen::Kayitlar {
            izleyiciler: vec![dogru],
            ..Default::default()
        },
    )
    .unwrap();
    let mut i2 = izleyici::kayitli_baglan(&kayit.host_kimlik, &hirsiz, "Hırsız", None)
        .await
        .unwrap();
    assert!(matches!(kayitli_sonuc(&mut i2).await, IzleyiciOlay::Koptu { .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn host_sertifikasi_degisirse_reddedilir() {
    // Kayıtlı host'un kimliği (anahtarı) değişirse — ör. aynı adreste başka bir cihaz —
    // izleyici bağlanmaz: kimlik el sıkışmada doğrulanır.
    let (hv, iv) = (gecici_klasor("host_veri"), gecici_klasor("iz_veri"));
    let mut h = host::baslat(kayitli_ayar("121212", &hv), Arc::new(SahteFabrika::new(64, 48)))
        .await
        .unwrap();
    let kayit = eslestir(&mut h, &iv).await;
    let baska_veri = gecici_klasor("baska_host");
    let mut baska = host::baslat(kayitli_ayar("343434", &baska_veri), Arc::new(SahteFabrika::new(64, 48)))
        .await
        .unwrap();
    let (baska_kod, baska_parola) = hazir(&mut baska).await;
    let baska_adres = crate::kod::coz(&baska_kod, &baska_parola, crate::kod::simdi())
        .unwrap()
        .adresler;
    assert_ne!(baska.kimlik(), kayit.host_kimlik);
    // Kayıtta eski kimlik, ama adres artık başka cihazın.
    crate::guvenilen::guncelle(&crate::guvenilen::kayit_yolu(&iv), |k| {
        k.izleyiciler[0].son_adresler = baska_adres.clone();
    })
    .unwrap();
    h.durdur();
    let e = match izleyici::kayitli_baglan(&kayit.host_kimlik, &iv, "Veli", None).await {
        Ok(_) => panic!("kimliği değişen host'a bağlanıldı"),
        Err(e) => e.to_string(),
    };
    assert!(
        e.starts_with("Karşı bilgisayara ulaşılamadı") || e.starts_with("Güvenlik kontrolü"),
        "{e}"
    );
    let ek = tokio::time::timeout(Duration::from_millis(300), baska.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::Istek { .. }))), "başka host'a istek gitmemeli");
    // Kayıt silinmez (host geri dönebilir).
    assert_eq!(crate::guvenilen::oku(&crate::guvenilen::kayit_yolu(&iv)).izleyiciler.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn kod_kapaliyken_kodla_girilmez_kayitli_cihaz_yine_baglanir() {
    // Uygulama arka planda dinlerken ("Bağlantı ver" kapalı): kodla gelen reddedilir,
    // kayıtlı cihaz onayla bağlanabilir.
    let (hv, iv) = (gecici_klasor("host_veri"), gecici_klasor("iz_veri"));
    let mut h = host::baslat(kayitli_ayar("565656", &hv), Arc::new(SahteFabrika::new(64, 48)))
        .await
        .unwrap();
    let (kayit, (kod, parola)) = eslestir_kodlu(&mut h, &iv).await;
    h.kod_ac(false);
    let mut yabanci = izleyici::baglan(&kod, &parola, "Yabancı").await.unwrap();
    match kayitli_sonuc(&mut yabanci).await {
        IzleyiciOlay::Koptu { sebep } => assert!(sebep.contains("bağlantı vermiyor"), "{sebep}"),
        o => panic!("kod kapalıyken kodla girildi: {o:?}"),
    }
    let mut i = izleyici::kayitli_baglan(&kayit.host_kimlik, &iv, "Veli", None)
        .await
        .unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { kayitli: true, .. })).await;
    h.komut(HostKomut::Kabul(izinler_kontrollu())).await;
    assert!(matches!(kayitli_sonuc(&mut i).await, IzleyiciOlay::Kabul { .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn host_kimligi_yeniden_acilista_ayni() {
    let hv = gecici_klasor("host_veri");
    let fab = Arc::new(SahteFabrika::new(64, 48));
    let mut a = host::baslat(kayitli_ayar("1", &hv), fab.clone()).await.unwrap();
    let ka = a.kimlik();
    a.durdur();
    let b = host::baslat(kayitli_ayar("1", &hv), fab.clone()).await.unwrap();
    assert_eq!(ka, b.kimlik(), "veri klasörüyle kimlik kalıcı");
    let c = host::baslat(ayar("1"), fab).await.unwrap();
    assert_ne!(ka, c.kimlik(), "veri klasörü yoksa her açılışta yeni kimlik");
}
