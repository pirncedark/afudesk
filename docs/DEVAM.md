# AfuDesk - Devam Notu

Son guncelleme: 2026-09-29 (koordinator oturumu)

## Bu tur: merkeziyetsizlik denetimi E1-E9 - dal `fix/merkeziyetsiz-e1-e9` (taslak PR)
- Denetim + duzeltme ozeti: `reports/MERKEZIYETSIZ_DENETIM.md` ("Duzeltme durumu" bolumu).
- E1 n0 pkarr/DNS yayini kaldirildi (presets::Minimal + clear_address_lookup), E2 `AFUDESK_RELAY` ile relay listesi,
  E3 relay/genel adres yoksa uyari, E4 mDNS YalnizYerel disinda acik, E5 10 haneli parola + 3 hatali denemede yeni kod,
  E6 kod ekrani kapaninca eski kod gecersiz, E7 devam jetonu 120 sn / izleyici 90 sn, E8 akis tavani 3, E9 surumlu ALPN.

## Dogrulama (koordinator, sandbox disinda gercekten calistirildi)
- `cd rust && cargo test --release` -> 102/102 gecti (codex sandbox'inda 3 ekran/mDNS testi ortam yuzunden kalmisti). Cikti: `reports/cargo-test-release.txt`.
- `cargo clippy --release --all-targets` -> 9 uyari, HEPSI bu degisiklikten once de vardi (goruntu.rs, platform.rs, host.rs oturum/read_datagram); yeni uyari yok.
- Flutter bu makinede yok: `flutter analyze/test` CI'da (ci.yml) kosar.

## Relay karari (2026-09-29, kullanici: "n0 relay yedek olsun") - UYGULANMIS
`AFUDESK_RELAY` yok -> `RelayMode::Default` (n0 relay'leri YALNIZ trafik yedegi; pkarr/DNS kimlik yayini kapali, `clear_address_lookup`).
`AFUDESK_RELAY=kapali|yok|off|0` -> relay tamamen kapali. `AFUDESK_RELAY=url1,url2` -> yalniz bu relay'ler. Test: `relay_listesi_ayristirilir_ve_bos_liste_merkeze_dusmez`.

## Sonraki adim
1. CI yesil -> PR'i taslaktan cikar -> merge -> surum notu.
