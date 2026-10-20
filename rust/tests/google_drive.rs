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
