#![cfg(windows)]
use afudesk_core::google_pin::{PinKasasi, PinSonucu};
use iroh::SecretKey;

fn anahtar() -> String {
    SecretKey::generate().public().to_string()
}

#[test]
fn kilit_yeniden_acilista_surer_ve_cihaz_degistirerek_asilmaz() {
    let dir = tempfile::tempdir().unwrap();
    let yol = dir.path().join("pin.dpapi");
    let host = anahtar();
    PinKasasi::kur(&yol, "hesap", &host, "123456").unwrap();
    for _ in 0..4 {
        assert_eq!(
            PinKasasi::dene(&yol, "hesap", &host, &anahtar(), Some("999999"), 0, 100).unwrap(),
            PinSonucu::Yanlis
        );
    }
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &anahtar(), Some("999999"), 0, 100).unwrap(),
        PinSonucu::Kilitli
    );
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &anahtar(), Some("123456"), 0, 399).unwrap(),
        PinSonucu::Kilitli
    );
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &anahtar(), Some("123456"), 0, 400).unwrap(),
        PinSonucu::Kabul
    );
}

#[test]
fn hatirlama_anahtara_hesaba_hosta_sureye_bagli_ve_iptal_edilir() {
    let dir = tempfile::tempdir().unwrap();
    let yol = dir.path().join("pin.dpapi");
    let (host, iz) = (anahtar(), anahtar());
    PinKasasi::kur(&yol, "hesap", &host, "123456").unwrap();
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &iz, Some("123456"), 60, 100).unwrap(),
        PinSonucu::Kabul
    );
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &iz, None, 0, 101).unwrap(),
        PinSonucu::Kabul
    );
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &anahtar(), None, 0, 101).unwrap(),
        PinSonucu::Gerekli
    );
    assert!(PinKasasi::dene(&yol, "baska", &host, &iz, None, 0, 101).is_err());
    assert!(PinKasasi::dene(&yol, "hesap", &anahtar(), &iz, None, 0, 101).is_err());
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &iz, None, 0, 160).unwrap(),
        PinSonucu::Gerekli
    );
    PinKasasi::dene(&yol, "hesap", &host, &iz, Some("123456"), 60, 161).unwrap();
    PinKasasi::unut(&yol, "hesap", &host, &iz).unwrap();
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &iz, None, 0, 162).unwrap(),
        PinSonucu::Gerekli
    );
    PinKasasi::cikis(&yol).unwrap();
    assert!(PinKasasi::dene(&yol, "hesap", &host, &iz, Some("123456"), 0, 163).is_err());
}

#[test]
fn bozuk_depo_saat_geri_alma_ve_uzun_hatirlama_yetki_vermez() {
    let dir = tempfile::tempdir().unwrap();
    let yol = dir.path().join("pin.dpapi");
    let (host, iz) = (anahtar(), anahtar());
    PinKasasi::kur(&yol, "hesap", &host, "123456").unwrap();
    assert!(PinKasasi::kur(&yol, "hesap", &host, "654321").is_err());
    PinKasasi::dene(&yol, "hesap", &host, &iz, Some("123456"), 60, 100).unwrap();
    assert!(PinKasasi::dene(&yol, "hesap", &host, &iz, None, 0, 99).is_err());
    assert!(PinKasasi::dene(&yol, "hesap", &host, &iz, Some("123456"), u64::MAX, 101).is_err());
    std::fs::write(&yol, b"bozuk").unwrap();
    assert!(PinKasasi::dene(&yol, "hesap", &host, &iz, Some("123456"), 0, 101).is_err());
}

#[test]
fn paralel_denemeler_ayni_butceyi_tuketir() {
    let dir = tempfile::tempdir().unwrap();
    let yol = dir.path().join("pin.dpapi");
    let host = anahtar();
    PinKasasi::kur(&yol, "hesap", &host, "123456").unwrap();
    let isler: Vec<_> = (0..8)
        .map(|_| {
            let yol = yol.clone();
            let host = host.clone();
            std::thread::spawn(move || {
                PinKasasi::dene(&yol, "hesap", &host, &anahtar(), Some("999999"), 0, 100).unwrap()
            })
        })
        .collect();
    let sonuclar: Vec<_> = isler.into_iter().map(|is| is.join().unwrap()).collect();
    assert_eq!(
        sonuclar.iter().filter(|s| **s == PinSonucu::Yanlis).count(),
        4
    );
    assert_eq!(
        sonuclar
            .iter()
            .filter(|s| **s == PinSonucu::Kilitli)
            .count(),
        4
    );
    assert_eq!(
        PinKasasi::dene(&yol, "hesap", &host, &anahtar(), Some("123456"), 0, 101).unwrap(),
        PinSonucu::Kilitli
    );
}
