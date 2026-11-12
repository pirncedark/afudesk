use afudesk_core::kod::{self, Davet};

#[test]
fn sifresi_cozulen_paylasim_bile_ip_tasimaz() {
    let davet = Davet {
        v: kod::DAVET_SURUMU, ad: "Ev bilgisayarı".into(),
        adresler: vec!["8.8.8.8:1234".into(), "100.100.1.2:1234".into(), "[2001:4860::1]:1234".into()],
        parmak_izi: "ab".repeat(32), relaylar: vec!["https://example.com/".into(), "https://8.8.8.8/".into()],
        bilet: kod::yeni_bilet(), bitis: kod::simdi() + 600,
    };
    let kod = kod::paylasim_kodu(&davet, "test-parolasi").unwrap();
    let acik = kod::coz(&kod, "test-parolasi", kod::simdi()).unwrap();
    assert!(acik.adresler.is_empty(), "davet doğrudan adres taşıyor");
    assert_eq!(acik.relaylar, vec!["https://example.com/"]);
    let json = serde_json::to_string(&acik).unwrap();
    for ip in ["8.8.8.8", "100.100.1.2", "2001:4860"] { assert!(!json.contains(ip)); }
    assert_eq!(davet.adresler.len(), 3, "Windows'un mevcut davet yükü değiştirilmez");
}
