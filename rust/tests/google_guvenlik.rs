use afudesk_core::google::{anahtar_nonce, karar, Dogrulayici, Karar};
use serde_json::Value;

fn veri() -> Value {
    serde_json::from_str(include_str!("fixtures/google.json")).unwrap()
}

#[test]
fn ayni_sub_kayitli_anahtarla_onaysiz_kabul() {
    let v = veri();
    let d = Dogrulayici::yeni(
        vec!["test.apps.googleusercontent.com".into()],
        &v["jwks"].to_string(),
    )
    .unwrap();
    let a = d
        .dogrula(v["token"].as_str().unwrap(), 1_700_000_100)
        .unwrap();
    assert_eq!(
        karar(
            Some(&a),
            Ok(&a),
            "host",
            "viewer",
            "viewer",
            true,
            true,
            1_700_000_100
        ),
        Karar::Kabul
    );
    assert_eq!(a.nonce(), anahtar_nonce("host", "viewer"));
}

#[test]
fn farkli_sub_ve_kapali_anahtar_onay_ister() {
    let v = veri();
    let d = Dogrulayici::yeni(
        vec!["test.apps.googleusercontent.com".into()],
        &v["jwks"].to_string(),
    )
    .unwrap();
    let a = d
        .dogrula(v["token"].as_str().unwrap(), 1_700_000_100)
        .unwrap();
    let b = d
        .dogrula(v["other"].as_str().unwrap(), 1_700_000_100)
        .unwrap();
    assert_eq!(
        karar(
            Some(&a),
            Ok(&b),
            "host",
            "viewer",
            "viewer",
            true,
            true,
            1_700_000_100
        ),
        Karar::OnaySor
    );
    assert_eq!(
        karar(
            Some(&a),
            Ok(&a),
            "host",
            "viewer",
            "viewer",
            true,
            false,
            1_700_000_100
        ),
        Karar::OnaySor
    );
    assert_eq!(
        karar(
            Some(&a),
            Ok(&a),
            "host",
            "viewer",
            "viewer",
            false,
            true,
            1_700_000_100
        ),
        Karar::OnaySor
    );
    assert_eq!(
        karar(
            Some(&a),
            Ok(&a),
            "host",
            "viewer",
            "sahte",
            true,
            true,
            1_700_000_100
        ),
        Karar::Red
    );
}

#[test]
fn gecersiz_token_anahtar_kapaliyken_bile_reddedilir() {
    let v = veri();
    let d = Dogrulayici::yeni(
        vec!["test.apps.googleusercontent.com".into()],
        &v["jwks"].to_string(),
    )
    .unwrap();
    for name in [
        "expired",
        "bad_signature",
        "wrong_aud",
        "wrong_iss",
        "future",
        "none",
    ] {
        assert!(
            d.dogrula(v[name].as_str().unwrap(), 1_700_000_100).is_err(),
            "{name}"
        );
    }
    assert_eq!(
        karar(
            None,
            Err(()),
            "host",
            "viewer",
            "viewer",
            true,
            false,
            1_700_000_100
        ),
        Karar::Red
    );
}

#[test]
fn dogrulama_sonrasi_suresi_dolan_token_yetki_vermez() {
    let v = veri();
    let d = Dogrulayici::yeni(
        vec!["test.apps.googleusercontent.com".into()],
        &v["jwks"].to_string(),
    )
    .unwrap();
    let a = d
        .dogrula(v["token"].as_str().unwrap(), 1_700_000_100)
        .unwrap();
    assert_eq!(
        karar(
            Some(&a),
            Ok(&a),
            "host",
            "viewer",
            "viewer",
            true,
            true,
            1_700_003_600
        ),
        Karar::Red
    );
}
