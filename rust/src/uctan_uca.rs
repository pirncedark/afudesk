//! Uçtan uca: aynı süreçte host + izleyici, gerçek QUIC, sahte ekran/girdi.
use crate::{
    host::{self, HostAyar, HostKomut, HostOlay},
    izleyici::{self, IzleyiciOlay},
    platform::sahte::SahteFabrika,
    protokol::{FareTusu, Girdi, Izinler},
};
use std::{path::PathBuf, sync::Arc, time::Duration};

const SURE: Duration = Duration::from_secs(20);

fn ayar(parola: &str) -> HostAyar {
    HostAyar { ad: "Ali PC".into(), port: 0, upnp: false, parola: parola.into(), yalniz_yerel: true, dosya_klasoru: None }
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
    tokio::time::timeout(SURE, async {
        loop {
            let o = i.olaylar.recv().await.expect("izleyici kapandı");
            if f(&o) {
                return o;
            }
        }
    })
    .await
    .expect("izleyici olayı gelmedi")
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
        HostOlay::Istek { ad } => assert_eq!(ad, "Veli"),
        _ => unreachable!(),
    }
    h.komut(HostKomut::Kabul(Izinler { kontrol: true, pano: false, dosya: false })).await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kabul { .. })).await {
        IzleyiciOlay::Kabul { genislik, yukseklik, izinler } => {
            assert_eq!((genislik, yukseklik), (160, 100));
            assert!(izinler.kontrol);
        }
        _ => unreachable!(),
    }
    // En az iki farklı kare gelmeli (sahte ekran sol üstü değiştiriyor).
    let mut goruler = Vec::new();
    while goruler.len() < 2 {
        if let IzleyiciOlay::Kare { genislik, yukseklik, rgba } =
            iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await
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
    i.gonder(Girdi::FareTus { tus: FareTusu::Sol, basili: true }).await;
    i.gonder(Girdi::Tus { ad: "ş".into(), basili: true }).await;
    tokio::time::timeout(SURE, async {
        while girdiler.lock().unwrap().len() < 3 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("girdiler hosta ulaşmadı");
    assert_eq!(girdiler.lock().unwrap()[0], Girdi::FareKonum { x: 0.5, y: 0.25 });

    // Host keser → izleyici sebebini öğrenir, host yeni kod üretir (bilet tek kullanımlık).
    h.komut(HostKomut::Kes).await;
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Koptu { .. })).await {
        IzleyiciOlay::Koptu { sebep } => assert!(sebep.contains("kesti") || sebep.contains("kapandı"), "{sebep}"),
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
    h.komut(HostKomut::Kabul(Izinler { kontrol: false, pano: false, dosya: false })).await;
    iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Kare { .. })).await;
    i.gonder(Girdi::FareTus { tus: FareTusu::Sol, basili: true }).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(girdiler.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn red_edilince_izleyici_sebebi_gorur() {
    let mut h = host::baslat(ayar("222222"), Arc::new(SahteFabrika::new(64, 64))).await.unwrap();
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
    let mut h = host::baslat(ayar("333333"), Arc::new(SahteFabrika::new(64, 64))).await.unwrap();
    let (kod, _) = hazir(&mut h).await;
    let e = izleyici::baglan(&kod, "000000", "Veli").await.err().unwrap();
    assert_eq!(e.to_string(), "Parola yanlış.");
}

#[tokio::test(flavor = "multi_thread")]
async fn sahte_bilet_reddedilir() {
    // Saldırgan adresi ve parmak izini biliyor ama bileti bilmiyor (ör. eski kod).
    let mut h = host::baslat(ayar("444444"), Arc::new(SahteFabrika::new(64, 64))).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut d = crate::kod::coz(&kod, &parola, crate::kod::simdi()).unwrap();
    d.bilet = crate::kod::yeni_bilet();
    let sahte = crate::kod::kodla(&d, &parola).unwrap();
    let mut i = izleyici::baglan(&sahte, &parola, "Mallory").await.unwrap();
    match iz_bekle(&mut i, |o| matches!(o, IzleyiciOlay::Koptu { .. })).await {
        IzleyiciOlay::Koptu { sebep } => assert_eq!(sebep, "Kod geçersiz ya da daha önce kullanılmış."),
        _ => unreachable!(),
    }
    // Host kullanıcıya istek GÖSTERMEMELİ.
    let ek = tokio::time::timeout(Duration::from_millis(500), h.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::Istek { .. }))));
}

#[tokio::test(flavor = "multi_thread")]
async fn host_durdurulunca_baglanilamaz() {
    let mut h = host::baslat(ayar("555555"), Arc::new(SahteFabrika::new(64, 64))).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    h.durdur();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let e = izleyici::baglan(&kod, &parola, "Veli").await.err().unwrap().to_string();
    // Paralel testlerde boşalan portu başka bir host alabilir: o zaman parmak izi
    // uyuşmaz ve bağlantı güvenlik kontrolünde reddedilir — bu da doğru davranış.
    assert!(
        e.starts_with("Karşı tarafa ulaşılamadı") || e.starts_with("Güvenlik kontrolü başarısız"),
        "{e}"
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
        Ok(d) => d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

/// Host (alınan dosyalar `hedef` klasörüne) + kabul edilmiş izleyici.
async fn dosya_oturumu(parola: &str, dosya_izni: bool, hedef: &std::path::Path) -> (host::Host, izleyici::Izleyici) {
    let mut a = ayar(parola);
    a.dosya_klasoru = Some(hedef.to_owned());
    let mut h = host::baslat(a, Arc::new(SahteFabrika::new(64, 64))).await.unwrap();
    let (kod, parola) = hazir(&mut h).await;
    let mut i = izleyici::baglan(&kod, &parola, "Veli").await.unwrap();
    olay_bekle(&mut h, |o| matches!(o, HostOlay::Istek { .. })).await;
    h.komut(HostKomut::Kabul(Izinler { kontrol: false, pano: false, dosya: dosya_izni })).await;
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
            IzleyiciOlay::Dosya { gonderilen, toplam, bitti: true, hata, .. } => return (gonderilen, toplam, hata, ara),
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
    assert_eq!((gonderilen, toplam), (icerik.len() as u64, icerik.len() as u64));
    assert!(ara.windows(2).all(|w| w[0] <= w[1]), "ilerleme geri gitmemeli: {ara:?}");
    let g = gorev.await.unwrap().unwrap();
    assert_eq!((g.baslangic, g.gonderilen), (0, icerik.len() as u64));

    let (ad, alinan) = match olay_bekle(&mut h, |o| matches!(o, HostOlay::DosyaAlindi { .. })).await {
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
    let gorev = i.dosya_gonder_ayarli(yol.clone(), crate::dosya::GonderAyari { kes: Some(KES), sha_boz: false });
    let (_, _, hata, _) = dosya_sonu(&mut i).await;
    assert_eq!(hata, crate::dosya::YARIDA_KALDI);
    assert!(gorev.await.unwrap().is_err());
    let yarimlar = icindekiler(&hedef);
    assert_eq!(yarimlar.len(), 1, "{yarimlar:?}");
    assert!(yarimlar[0].ends_with(".afudesk-parca"), "{yarimlar:?}");
    assert_eq!(std::fs::metadata(hedef.join(&yarimlar[0])).unwrap().len(), KES);
    assert!(!hedef.join("yarim.bin").exists());

    // 2) Yeniden gönder: yalnız kalan kısım gider.
    let gorev = i.dosya_gonder(&yol);
    let (gonderilen, toplam, hata, ara) = dosya_sonu(&mut i).await;
    assert_eq!(hata, "");
    assert_eq!((gonderilen, toplam), (icerik.len() as u64, icerik.len() as u64));
    assert!(ara.iter().all(|&g| g >= KES), "ilerleme devam noktasından başlamalı: {ara:?}");
    let g = gorev.await.unwrap().unwrap();
    assert_eq!(g.baslangic, KES);
    assert_eq!(g.gonderilen, icerik.len() as u64 - KES, "yalnız kalan bayt gönderilmeli");

    match olay_bekle(&mut h, |o| matches!(o, HostOlay::DosyaAlindi { .. })).await {
        HostOlay::DosyaAlindi { ad, .. } => assert_eq!(ad, "yarim.bin"),
        _ => unreachable!(),
    }
    let gelen = std::fs::read(hedef.join("yarim.bin")).unwrap();
    assert!(gelen == icerik, "içerik birebir aynı olmalı");
    assert_eq!(sha(&gelen), sha(&icerik));
    assert_eq!(icindekiler(&hedef), ["yarim.bin"], "yarım dosya temizlenmeli");

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
    assert_eq!(gorev.await.unwrap().unwrap_err().to_string(), "Karşı taraf dosya almaya izin vermedi.");
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

    let gorev = i.dosya_gonder_ayarli(yol.clone(), crate::dosya::GonderAyari { kes: None, sha_boz: true });
    let (_, _, hata, _) = dosya_sonu(&mut i).await;
    assert_eq!(hata, crate::dosya::BOZUK);
    assert!(gorev.await.unwrap().is_err());
    assert!(!hedef.join("bozuk.bin").exists(), "asıl ad oluşmamalı");
    assert!(icindekiler(&hedef).is_empty(), "bozuk yarım dosya da silinmeli: {:?}", icindekiler(&hedef));
    let ek = tokio::time::timeout(Duration::from_millis(300), h.olaylar.recv()).await;
    assert!(!matches!(ek, Ok(Some(HostOlay::DosyaAlindi { .. }))));

    // Aynı dosya doğru özetle gönderilince sorunsuz gelir.
    i.dosya_gonder(&yol);
    assert_eq!(dosya_sonu(&mut i).await.2, "");
    assert_eq!(sha(&std::fs::read(hedef.join("bozuk.bin")).unwrap()), sha(&icerik));

    let _ = std::fs::remove_dir_all(&hedef);
    let _ = std::fs::remove_dir_all(&kaynak);
}
