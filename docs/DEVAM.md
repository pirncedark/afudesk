# AfuDesk - Devam Notu

Son guncelleme: 2026-09-29 (koordinator oturumu)

## Bu tur: merkeziyetsizlik denetimi E1-E9 - dal `fix/merkeziyetsiz-e1-e9` (taslak PR)
- Denetim + duzeltme ozeti: `reports/MERKEZIYETSIZ_DENETIM.md` ("Duzeltme durumu" bolumu).
- E1 n0 pkarr/DNS yayini kaldirildi (presets::Minimal + clear_address_lookup), E2 `AFUDESK_RELAY` ile relay listesi,
  E3 relay/genel adres yoksa uyari, E4 mDNS YalnizYerel disinda acik, E5 10 haneli parola + 3 hatali denemede yeni kod,
  E6 kod ekrani kapaninca YALNIZ eski kod gecersiz (ayri `kod_kapandi` sinyali; acik oturum/bekleyen onay kesilmez),
  E7 devam jetonu 120 sn, E8 akis tavani 64 (3 dosya aktarimi + goruntu cakisinca kare dusururdu),
  E9 SABIT protokol surumu ALPN `afudesk/6` (uygulama surumune bagli degil); surum uyusmazsa acik "iki taraf da guncellemeli" mesaji.
- Codex'in 3 hatasini (relay kapali, E6 oturumu kesiyordu, E8/E9) AfuDesk terminali duzeltti; hepsi 868334e icinde.
- DIKKAT: eski surumlerle baglanti artik kurulmaz -> surum notunda "iki cihaz da guncellenmeli" yaz.

- DAVRANIS DEGISIKLIGI: pkarr kalktigi icin internet adresi degisen kayitli cihaz artik yalniz ayni agda (mDNS) kendiliginden bulunur;
  internetten bir kez kodla baglaninca kayit yenilenir (izleyici.rs ~632). 1.4.0 notu "internette adres aramasiyla bulunur" diyordu.
- Yayin oncesi: rust/Cargo.toml ve app/pubspec.yaml hala 1.4.0 -> 1.5.0'a artir; notu docs/SURUM_NOTLARI_1.5.0.md hazir.
- v1.7.0 yayin notu: `docs/SURUM_NOTLARI_1.7.0.md` (EN + TR); telefonda yakinlastirma ve otomatik guncelleme, v1.6.0 ile baglanti uyumlu.
- v1.6.0 yayin notu: `docs/SURUM_NOTLARI_1.6.0.md` (EN + TR); iki taraf da v1.6.0 kullanmali, kayitli cihazlar korunur.

## Dogrulama (koordinator, sandbox disinda gercekten calistirildi)
- `cd rust && cargo test --release` -> 102-103/102-103 gecti (koordinator 102, AfuDesk terminali 103) (codex sandbox'inda 3 ekran/mDNS testi ortam yuzunden kalmisti). Cikti: `reports/cargo-test-release.txt`.
- `cargo clippy --release --all-targets` -> 9 uyari, HEPSI bu degisiklikten once de vardi (goruntu.rs, platform.rs, host.rs oturum/read_datagram); yeni uyari yok.
- Flutter bu makinede yok; PR #1 CI `test` isi (cargo + flutter analyze/test) GECTI.

## Relay karari (2026-09-29, kullanici: "n0 relay yedek olsun") - UYGULANMIS
`AFUDESK_RELAY` yok -> `RelayMode::Default` (n0 relay'leri YALNIZ trafik yedegi; pkarr/DNS kimlik yayini kapali, `clear_address_lookup`).
`AFUDESK_RELAY=kapali|yok|off|0` -> relay tamamen kapali. `AFUDESK_RELAY=url1,url2` -> yalniz bu relay'ler. Test: `relay_listesi_ayristirilir_ve_bos_liste_merkeze_dusmez`.

## Sonraki adim
- PR #1 MERGE edildi (main 304cf98). Sonraki adim: surum 1.5.0 notu (uyumsuzluk uyarisi dahil) + yayin karari.
