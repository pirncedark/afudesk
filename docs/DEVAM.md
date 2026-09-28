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

## ENGEL - kullanici karari bekliyor (merge EDILMEDI)
`AFUDESK_RELAY` bos iken ne olacak? Su anki kod: relay tamamen KAPALI (RelayMode::Disabled). Sonuc: ayni ag / genel IP disinda
uzaktan baglanti kutudan cikti haliyle calismayabilir. Rapor bu karari acikca kullaniciya birakti ("varsayilan n0 olsun veya olmasin").
Secenekler: (a) bos -> kapali (su anki), (b) bos -> n0 relay'leri yalniz relay olarak (pkarr/DNS yayini yine kapali), (c) varsayilan kendi relay adresi.

## Sonraki adim
1. Kullanicinin relay kararini uygula (gerekirse `ag.rs` relay_modu), testleri tekrar kos.
2. CI yesil -> PR'i taslaktan cikar -> merge -> surum notu.
