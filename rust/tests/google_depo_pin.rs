use afudesk_core::{google_depo, google_pin::Pin};

#[test]
fn pin_en_az_alti_rakam_ve_rastgele_tuz() {
    for p in ["12345", "12345a", "", "１２３４５６"] {
        assert!(Pin::olustur(p).is_err());
    }
    let a = Pin::olustur("123456").unwrap();
    let b = Pin::olustur("123456").unwrap();
    assert_ne!(a.phc(), b.phc());
    assert!(a.phc().starts_with("$argon2id$"));
    assert!(a.dogrula("123456"));
    assert!(!a.dogrula("654321"));
    assert!(!format!("{a:?}").contains("123456"));
}

#[test]
fn phc_yukleme_yalniz_sabit_maliyetli_argon2id_kabul_eder() {
    let p = Pin::olustur("123456").unwrap();
    assert!(Pin::phc_yukle(p.phc()).unwrap().dogrula("123456"));
    for phc in [
        "bozuk".to_owned(),
        p.phc().replace("argon2id", "argon2i"),
        p.phc().replace("m=19456", "m=4294967295"),
        p.phc().replace("t=2", "t=999999"),
        p.phc().replace("v=19", "v=16"),
    ] {
        assert!(Pin::phc_yukle(&phc).is_err());
    }
}

#[cfg(windows)]
#[test]
fn dpapi_acik_metin_yazmaz_ve_bozulursa_reddeder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("google.dpapi");
    google_depo::yaz(&path, b"secret-account-session").unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.windows(22).any(|s| s == b"secret-account-session"));
    assert_eq!(google_depo::oku(&path).unwrap(), b"secret-account-session");
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    assert!(google_depo::oku(&path).is_err());
    google_depo::sil(&path).unwrap();
    assert!(!path.exists());
    google_depo::sil(&path).unwrap();
}
