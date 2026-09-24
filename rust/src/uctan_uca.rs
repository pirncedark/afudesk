//! Uçtan uca: aynı süreçte host + izleyici, gerçek QUIC, sahte ekran/girdi.
use crate::{
    host::{self, HostAyar, HostKomut, HostOlay},
    izleyici::{self, IzleyiciOlay},
    platform::sahte::SahteFabrika,
    protokol::{FareTusu, Girdi, Izinler},
};
use std::{sync::Arc, time::Duration};

const SURE: Duration = Duration::from_secs(20);

fn ayar(parola: &str) -> HostAyar {
    HostAyar {
        ad: "Ali PC".into(),
        port: 0,
        upnp: false,
        parola: parola.into(),
        yalniz_yerel: true,
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
    h.komut(HostKomut::Kabul(Izinler {
        kontrol: true,
        pano: false,
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
        e.starts_with("Karşı tarafa ulaşılamadı") || e.starts_with("Güvenlik kontrolü başarısız"),
        "{e}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn oyun_kolu_hosta_ulasir() {
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
        oyun_kolu: true,
    }))
    .await;
    i.gonder(Girdi::Kol(crate::protokol::KolDurumu {
        slot: 0,
        sira: 1,
        dugmeler: 0x1000,
        sol_x: 0,
        sol_y: 0,
        sag_x: 0,
        sag_y: 0,
        sol_tetik: 0,
        sag_tetik: 0,
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
    tokio::time::sleep(Duration::from_millis(250)).await;
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
