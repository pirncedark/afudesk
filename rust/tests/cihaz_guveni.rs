use afudesk_core::cihaz_guveni::CihazGuveni;
use iroh::SecretKey;

const NOW: i64 = 1_700_000_100;
const CLIENT: &str = "test.apps.googleusercontent.com";

fn anahtarlar() -> (SecretKey, iroh::EndpointId) {
    (
        SecretKey::from_bytes(&[1; 32]),
        SecretKey::from_bytes(&[2; 32]).public(),
    )
}

#[test]
fn guven_tam_otuz_gun_soner_ve_yenilemeyle_uzamaz() {
    let (host, viewer) = anahtarlar();
    let mut k = CihazGuveni::yeni(&host, viewer, "account", CLIENT, NOW).unwrap();
    assert!(k
        .dogrula(host.public(), viewer, "account", CLIENT, NOW + 2_591_999)
        .is_ok());
    k.ilerlet(&host, NOW + 2_591_999).unwrap();
    assert!(k
        .dogrula(host.public(), viewer, "account", CLIENT, NOW + 2_592_000)
        .is_err());
    assert!(k.ilerlet(&host, NOW + 2_592_000).is_err());
}

#[test]
fn baslangictan_ve_son_kullanimdan_geriye_saat_reddedilir() {
    let (host, viewer) = anahtarlar();
    let mut k = CihazGuveni::yeni(&host, viewer, "account", CLIENT, NOW).unwrap();
    assert!(k
        .dogrula(host.public(), viewer, "account", CLIENT, NOW - 1)
        .is_err());
    k.ilerlet(&host, NOW + 3_600).unwrap();
    assert!(k
        .dogrula(host.public(), viewer, "account", CLIENT, NOW + 3_599)
        .is_err());
    assert!(k
        .dogrula(host.public(), viewer, "account", CLIENT, NOW + 3_600)
        .is_ok());
}

#[test]
fn baska_cihaz_anahtari_hesap_ve_istemci_reddedilir() {
    let (host, viewer) = anahtarlar();
    let baska = SecretKey::from_bytes(&[3; 32]).public();
    let k = CihazGuveni::yeni(&host, viewer, "account", CLIENT, NOW).unwrap();
    assert!(k.dogrula(baska, viewer, "account", CLIENT, NOW).is_err());
    assert!(k
        .dogrula(host.public(), baska, "account", CLIENT, NOW)
        .is_err());
    assert!(k
        .dogrula(host.public(), viewer, "other", CLIENT, NOW)
        .is_err());
    assert!(k
        .dogrula(
            host.public(),
            viewer,
            "account",
            "other.apps.googleusercontent.com",
            NOW
        )
        .is_err());
}

#[test]
fn imzali_alanlarin_degistirilmesi_reddedilir() {
    let (host, viewer) = anahtarlar();
    let k = CihazGuveni::yeni(&host, viewer, "account", CLIENT, NOW).unwrap();
    let original: serde_json::Value = serde_json::from_str(&k.yaz().unwrap()).unwrap();
    for (alan, deger) in [
        ("sub", serde_json::json!("other")),
        (
            "client_id",
            serde_json::json!("other.apps.googleusercontent.com"),
        ),
        ("olusturuldu", serde_json::json!(NOW - 1)),
        ("sona_eriyor", serde_json::json!(NOW + 2_592_001)),
        ("son_kullanim", serde_json::json!(NOW + 1)),
        ("oturum_id", serde_json::json!("A".repeat(43))),
        ("izleyici", serde_json::json!(host.public().to_string())),
        ("host", serde_json::json!(viewer.to_string())),
        ("imza", serde_json::json!("A".repeat(86))),
    ] {
        let mut v = original.clone();
        v[alan] = deger;
        if let Ok(k) = CihazGuveni::oku(&v.to_string()) {
            assert!(
                k.dogrula(host.public(), viewer, "account", CLIENT, NOW)
                    .is_err(),
                "{alan}"
            );
        }
    }
}

#[test]
fn eski_imzasiz_ve_sinirsiz_kayit_kabul_edilmez() {
    assert!(CihazGuveni::oku("{}").is_err());
    assert!(CihazGuveni::oku(&"x".repeat(8_193)).is_err());
    let (host, viewer) = anahtarlar();
    assert!(CihazGuveni::yeni(&host, viewer, "", CLIENT, NOW).is_err());
    assert!(CihazGuveni::yeni(&host, viewer, &"a".repeat(257), CLIENT, NOW).is_err());
    assert!(CihazGuveni::yeni(&host, viewer, "account", CLIENT, -1).is_err());
    assert!(CihazGuveni::yeni(&host, viewer, "account", CLIENT, i64::MAX).is_err());
}

#[test]
fn kontrol_karakterli_istemci_kimligi_imzalanmaz() {
    let (host, viewer) = anahtarlar();
    assert!(CihazGuveni::yeni(&host, viewer, "account", "bad\0.apps.googleusercontent.com", NOW).is_err());
}

#[test]
fn ayni_cihaz_yeni_giris_ile_farkli_oturum_uretir() {
    let (host, viewer) = anahtarlar();
    let a = CihazGuveni::yeni(&host, viewer, "account", CLIENT, NOW).unwrap();
    let mut b = CihazGuveni::oku(&a.yaz().unwrap()).unwrap();
    b.ilerlet(&host, NOW + 60).unwrap();
    assert!(a.ayni_oturum(&b));
    let c = CihazGuveni::yeni(&host, viewer, "account", CLIENT, NOW).unwrap();
    assert!(!a.ayni_oturum(&c));
}
