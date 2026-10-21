use afudesk_core::google::{Dogrulayici, Karar};
use afudesk_core::google_drive::{drive_karari, CihazKaydi};
use serde_json::Value;
fn veri() -> Value {
    serde_json::from_str(include_str!("fixtures/google.json")).unwrap()
}
fn kayit() -> CihazKaydi {
    CihazKaydi {
        v: 1,
        sub: "1234567890".into(),
        host: "host".into(),
        izleyici: "viewer".into(),
        ad: "Ev".into(),
        platform: "windows".into(),
        adresler: vec![],
        relaylar: vec![],
        son_gorulme: 1_700_000_100,
        erisim: true,
    }
}
#[test]
fn drive_ayni_hesap_kayitli_uc_onaysiz_farkli_hesap_onay() {
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
    let mut k = kayit();
    k.sub = a.sub().into();
    assert_eq!(
        drive_karari(&a, Ok(&a), "viewer", &[k.clone()], 1_700_000_100),
        Karar::Kabul
    );
    assert_eq!(
        drive_karari(&a, Ok(&a), "viewer", &[k.clone(), k.clone()], 1_700_000_100),
        Karar::Red
    );
    assert_eq!(
        drive_karari(&a, Ok(&b), "viewer", &[k.clone()], 1_700_000_100),
        Karar::OnaySor
    );
    assert_eq!(
        drive_karari(&a, Ok(&a), "sahte", &[k.clone()], 1_700_000_100),
        Karar::Red
    );
    assert_eq!(
        drive_karari(&a, Ok(&a), "viewer", &[], 1_700_000_100),
        Karar::Red
    );
    k.son_gorulme -= 301;
    assert_eq!(
        drive_karari(&a, Ok(&a), "viewer", &[k], 1_700_000_100),
        Karar::Red
    );
}
#[test]
fn drive_bozuk_ve_suresi_dolan_kanit_reddedilir() {
    let v = veri();
    let d = Dogrulayici::yeni(
        vec!["test.apps.googleusercontent.com".into()],
        &v["jwks"].to_string(),
    )
    .unwrap();
    let a = d
        .dogrula(v["token"].as_str().unwrap(), 1_700_000_100)
        .unwrap();
    let mut k = kayit();
    k.sub = a.sub().into();
    assert_eq!(
        drive_karari(&a, Err(()), "viewer", &[k.clone()], 1_700_000_100),
        Karar::Red
    );
    assert_eq!(
        drive_karari(&a, Ok(&a), "viewer", &[k], 1_700_003_600),
        Karar::Red
    );
    for name in ["expired", "bad_signature", "wrong_aud"] {
        assert!(d.dogrula(v[name].as_str().unwrap(), 1_700_000_100).is_err());
    }
}

#[test]
fn kapali_cihaz_onaysiz_kabul_edilmez() {
    let v = veri();
    let d = Dogrulayici::yeni(
        vec!["test.apps.googleusercontent.com".into()],
        &v["jwks"].to_string(),
    )
    .unwrap();
    let a = d
        .dogrula(v["token"].as_str().unwrap(), 1_700_000_100)
        .unwrap();
    let mut k = kayit();
    k.sub = a.sub().into();
    k.erisim = false;
    assert_eq!(
        drive_karari(&a, Ok(&a), "viewer", &[k], 1_700_000_100),
        Karar::Red
    );
}

#[test]
fn drive_kaydi_yalniz_acik_cihaz_verisi_tasir() {
    let value = serde_json::to_value(kayit()).unwrap();
    let mut fields: Vec<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort();
    assert_eq!(
        fields,
        vec![
            "ad",
            "adresler",
            "erisim",
            "host",
            "izleyici",
            "platform",
            "relaylar",
            "son_gorulme",
            "sub",
            "v"
        ]
    );
    let proof = afudesk_core::google_baglanti::Kanit("secret-token".into());
    assert!(!format!("{proof:?}").contains("secret-token"));
}

#[test]
fn drive_kaydi_baska_dugume_ve_hesaba_tasinamaz() {
    let v = veri();
    let d = Dogrulayici::yeni(
        vec!["test.apps.googleusercontent.com".into()],
        &v["jwks"].to_string(),
    )
    .unwrap();
    let a = d
        .dogrula(v["token"].as_str().unwrap(), 1_700_000_100)
        .unwrap();
    for alan in ["host", "izleyici", "sub", "gelecek"] {
        let mut k = kayit();
        k.sub = a.sub().into();
        match alan {
            "host" => k.host = "baska-host".into(),
            "izleyici" => k.izleyici = "baska-izleyici".into(),
            "sub" => k.sub = "baska-hesap".into(),
            _ => k.son_gorulme += 1,
        }
        assert_eq!(
            drive_karari(&a, Ok(&a), "viewer", &[k], 1_700_000_100),
            Karar::Red,
            "{alan}"
        );
    }
}
