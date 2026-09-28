# AfuDesk — Merkeziyetsiz (P2P) Uzak Masaüstü Denetimi

Tarih: 2026-09-29 · Commit: `46a9ad8` (v1.4.0) · Yöntem: kaynak okuma + canlı çalıştırma

## Özet

AfuDesk bugün **merkeziyetçi değil ama merkezileştirilmiş**. Taşıma katmanı `presets::N0`
üzerinden **iki ayrı n0.computer bağımlılığı** getiriyor: relay sunucuları ve pkarr
adres yayıncısı/çözücüsü. Kod mDNS ekleyerek merkeziyetsiz *görünüm* kazanıyor, ama
`Builder::address_lookup` **ekler**, değiştirmez — bu yüzden n0 pkarr yayıncısı sessizce
çalışmaya devam ediyor. Kendi relay kümenizi kurup `RelayMode::Custom` kullanmak tek
satırda engelli: `ag.rs` içinde `relay_mode`/`relay_urls` yapılandırma noktası yok.

Test durumu: `cargo test --release` → **92 geçti, 0 kaldı** (22 sn, 164 sn derleme).
Testler `Kurulum::YalnizYerel` (relay kapalı, 127.0.0.1) üzerinde koşar; merkezi
bağımlılığın koptuğu senaryoyu hiç kapsamaz.

---

## E1 — CRITICAL: n0 pkarr yayıncısı merkeziyetsizliği sessizce ihlal ediyor

**Kanıt.** `rust/src/ag.rs:82` `Endpoint::builder(presets::N0)` kullanıyor. iroh 1.2.0
`presets.rs:125-133` bu preset'e şunları ekler:

```
PkarrPublisher::n0_dns()   -> https://dns.iroh.link/pkarr
PkarrResolver::n0_dns()     -> https://dns.iroh.link/pkarr
DnsAddressLookup::n0_dns()  -> iroh.link
relay_mode(default_relay_mode())  -> euc1-1/use1-1/usw1-1.relay.n0.iroh.link
```

`ag.rs:90-92` mDNS'i `address_lookup(...)` ile ekliyor. `iroh-1.2.0/src/endpoint.rs:612`:

```rust
pub fn address_lookup(mut self, address_lookup: impl AddressLookupBuilder) -> Self {
    self.address_lookup.push(Box::new(address_lookup));   // EKLE, değiştirme
    self
}
```

`clear_address_lookup()` (endpoint.rs:592) projede **hiçbir yerde çağrılmıyor**
(`grep -rn "clear_address_lookup" src/` → 0 sonuç).

**Canlı doğrulama** (gerçek endpoint, bu makinenin interneti üzerinden):

```
DEBUG iroh::address_lookup::pkarr: creating pkarr publisher that publishes to
      https://dns.iroh.link/pkarr
DEBUG iroh::address_lookup::pkarr: Publishing endpoint info to pkarr
      data=EndpointData { addrs: [Relay(https://euc1-1.relay.n0.iroh.link./)] }
      pkarr_relay=https://dns.iroh.link/pkarr
INFO  relay-actor: home is now relay https://euc1-1.relay.n0.iroh.link./, was None
```

**Sonuç.** Her açılışta cihaz kimliğiniz + relay adresiniz n0'ın DNS sunucusuna
yazılıyor. n0 kapanırsa ya da trafiği engellerse: relay ölür (ağ çöker), pkarr
çözümü ölür. Merkezileşme kaldırılmadı, gizlendi.

**Düzeltme.** `Kurulum::Internet` ve `Kurulum::SadeceRelay` için preset'i terk edin:

```rust
// ag.rs — presets::N0 yerine kendi yapılandırmanız
let mut b = match kurulum {
    Kurulum::YalnizYerel => Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")?,
    Kurulum::Internet | Kurulum::SadeceRelay => {
        let mut b = Endpoint::builder(presets::Minimal)
            .clear_address_lookup()          // <— n0 pkarr + dns silinir
            .relay_mode(relay_modu(RELAY_LISTESI)?);
        if kurulum == Kurulum::SadeceRelay {
            b = b.clear_ip_transports();
        }
        b
    }
};
```

`relay_modu` sarmalayıcısı: `RELAY_LISTESI` boşsa `RelayMode::Disabled`, doluysa
`RelayMode::Custom(BTreeMap<RelayUrl, RelayModeConfig>)`. Listeyi
`AFUDESK_RELAY` ortam değişkeninden okuyun (virgülle ayrılmış) — kullanıcı kendi
relay'ını kursun, varsayılan n0 olsun veya olmasın; karar sizde.

---

## E2 — CRITICAL: relay listesi yapılandırılamaz, kod sadece "keşfedilen" relay'leri taşır

**Kanıt.** `ag.rs:204`:

```rust
let relaylar = a.relay_urls().map(|r| r.to_string()).collect();
```

Bu yalnız endpoint'in **bağlı olduğu** relay'leri döndürür; kullanıcı listesi yoktur.
`host.rs:326` `adresleri_topla` → `ag::yayinlanacak` zincirinden geçer ve
`kod.rs:36` `relaylar: Vec<String>` alanına yazılır. `ag.rs:191-195` hedef adres
yapımında relay'ler `TransportAddr::Relay` olarak tüketilir.

**Sonuç.** Kullanıcı relay değiştiremez; endpoint hangi relay'e bağlandıysa kod
odaki liste ondan ibaret. Özel relay kümesi kullanan bir dağıtım imkânsız; kod
taşıyıcısı relay sağlayıcısını sabitler.

**Düzeltme.** `uc_nokta_kimlikli` bir `relay: Vec<RelayUrl>` parametresi alsın;
`yayinlanacak` `ep.addr().relay_urls()` ile o listeyi birleştirsin:

```rust
let mut relaylar: Vec<String> = a.relay_urls().map(|r| r.to_string()).collect();
for u in ek_relaylar { if !relaylar.contains(&u) { relaylar.push(u); } }
```

Böylece endpoint canlı olmasa bile kod, kodlayanın bildirdiği relay'leri taşır.

---

## E3 — HIGH: n0 dışı ağlarda kod üretimi sessizce "yalnız aynı ağ" moduna düşüyor

**Kanıt.** `host.rs:326-332`:

```rust
let (adresler, relaylar) = adresleri_topla(&ep, kurulum);
if adresler.is_empty() && relaylar.is_empty() {
    let _ = olay.send(HostOlay::Hata("Ağ bağlantısı bulunamadı.".into())).await;
    break;
}
```

`host.rs:206-214` `erisim_aciklamasi`: relay boşsa *"Yalnız aynı ağdan ulaşılabilir —
relay sunucusuna ulaşılamadı (internet var mı?)"*. Ancak relay listesine **yalnız
bağlanılan relay'ler** girdiği için (E2), `CEVRIMICI_BEKLE = 8 sn` (host.rs:31)
içinde relay'e çıkılamadıysa `relaylar` boş kalır. Sonuç: **kullanıcı geçerli bir kod
alır, kod relay'sizdir, karşı taraf farklı ağdan bağlanamaz** — hata mesajı yalnız
metin açıklamada, kod çalışır durumda görünür.

**Düzeltme.** E2'den sonra `relaylar` sabit listeye dayanacak. Buna ek olarak
`host.rs:364` öncesinde bir ön-kontrol koyun: `adresler` içinde `genel_mi()` (ag.rs:119)
doğrurulayan bir adres yoksa ve `relaylar` boşsa, `HostOlay::Uyari` üretin
("bu kod yalnız aynı ağda çalışır"). `Hata` yerine `Uyari` kullanın — süreç
çoktan kodu yayımladı, öldürmek yanıltıcı.

---

## E4 — HIGH: mDNS yalnız `Kurulum::Internet`'te, kayıtlı cihazlar yerel ağ dışında boşa düşüyor

**Kanıt.** `ag.rs:89-93`:

```rust
if kurulum == Kurulum::Internet {
    b = b.address_lookup(MdnsAddressLookup::builder().service_name(MDNS_SERVISI));
}
```

`Kurulum::SadeceRelay` (AFUDESK_SADECE_RELAY=1) mDNS almaz. Doğrudan yollar
kapatıldığı için bu doğru; ancak `izleyici.rs:202-214` `kurulum_sec` yalnız
tüm adreslerin loopback olmasına bakıyor, aksi hâlde `Kurulum::ortamdan()`
dönüyor — yani izleyici tarafı relay'e zorlandığında **kayıtlı cihaz keşfi devre
dışı** kalır ve `guvenilen::kayitli_baglan` yalnız kayıtlı `son_adresler`e düşer
(`izleyici.rs:261`).

**Düzeltme.** mDNS'i `Kurulum::SadeceRelay` için de ekleyin — mDNS yerel ağda
çalışır, relay kısıtlaması IP taşımalarını kapatır, keşfi etkilemez. Koşul
`kurulum != Kurulum::YalnizYerel` olmalı.

---

## E5 — MEDIUM: kod 10 dakika geçerli ama parola 6 haneli — kaba kuvvet sınırı yok

**Kanıt.** `kod.rs:13` `GECERLILIK_SN = 600`, `kod.rs:132-134`:

```rust
pub fn yeni_parola() -> String {
    format!("{:06}", rand::random::<u32>() % 1_000_000)
}
```

Argon2id 64 MiB (kod.rs:63) çevrimdışı kaba kuvveti yavaşlatır ama 10⁶ uzay
makul bir makinede dakikalarca erir. Kodu gören bir saldırgan parolayı dener,
`Davet`'i çözer, `bilet` + `parmak_izi` elde eder. `host.rs:475` bileti sabit
zamanlı karşılaştırıyor (iyi) ancak **deneme sayısı sınırı yok**: `host.rs:365-402`
döngüsü her `ep.accept()` için `oturum()` çağırır, `Kontrol::Merhaba` okunur,
`reddet` döner, döngü devam eder. 10 dakikada sınırsız deneme.

Not `rand::random::<u32>() % 1_000_000` **modulo yanlılığı** üretir: 2³² sayılabilir,
1e6'a bölümü 4294.967 → 4295 fazlalık, ilk 4296 değer bir fazla olasılıkla gelir.
Kriptografik olarak ölümlü değil, düzeltmesi bedava: `rand::random_range(0..1_000_000)`
veya 2 bölüm.

**Düzeltme.**

1. `yeni_parola()` → 10 haneli (`rand::random_range(0..1_000_000_000)`), geriye
   dönük uyum `coz()` tarafında korunur (parola uzunluğu doğrulanmıyor).
2. `host.rs:365` içinde bağlantı başına deneme sayacı: `bilet` eşleşmeyen
   `Merhaba` başına host 60 sn (`ONAY_SURESI`, host.rs:27) boyunca en fazla 3
   deneme kabul etsin, sonra `CEVRIMICI_BEKLE` kadar bekleyip yeniden kod üretin.
3. Alternatif: parolayı 6 haneden vazgeçip kullanıcı seçsin, `kod.rs` aynı kalır.

---

## E6 — MEDIUM: host, kodu `kod_acik` kapalıyken de dinlemeye devam ediyor

**Kanıt.** `host.rs:471-474` — bilet doğrulamasından **önce** `kod_acik` kontrolü
var ve reddediyor. Doğru davranış. Ancak `host.rs:364-402` döngüsü kod üretip
`ep.accept()` beklerken "Bağlantı ver" ekranı kapanırsa `HostKomut::Kes` gönderilir
(`kopru/src/api/afudesk.rs:256-258`) — bu komut yalnız `oturum()` içindeki
`select!`'te dinlenir, `calis` ana döngüsünde `durdur` değil. Yani ekran kapandığında
kod geçerliliğini yitirmiş olabilir ama host hâlâ o kodu kabul eder. Test
`kod_kapaliyken_kodla_girilmez` (uctan_uca.rs) yalnız reddi doğruluyor, kodun
üretilmesinin durmasını doğrulamıyor.

**Düzeltme.** `calis` ana döngüsüne `tokio::select!` ile `komut.recv()` ekleyin;
`HostKomut::Kes` geldiğinde `kod_acik` false ise ana döngüyü kapatın (`break 'kod`).
Mevcut `Devam` bekleme döngüsü (host.rs:280-306) zaten `durdur` dinliyor, aynı
deseni oraya da taşıyın.

---

## E7 — MEDIUM: yeniden bağlanma pencereleri asimetrik ve dar

**Kanıt.** `izleyici.rs:32` `YENIDEN_SURESI = 90 sn`, `host.rs:29`
`DEVAM_SURESI = 60 sn`. `izleyici.rs:392-401`: kopuştan sonra geri dönmek için
`istendi || istemsiz_kopus(&c)` gerekiyor. `host.rs:279` devam penceresi 60 sn
sonra kapanır ve `host.rs:316-322` "karşı taraf geri dönmedi" gönderir.

Somut aralık: host kopmayı ~20 sn geç fark ederse (iroh `max_idle_timeout` 20 sn,
ag.rs:59) devam penceresi ilk 40 sn'ini harcamış olur; izleyicinin `yeniden_kur`
ilk denemesi `BAGLANMA_SURESI = 20 sn` (izleyici.rs:29) sürüyor — pencere
sıfırlanırken ilk deneme tükenmiş olabilir. `izleyici.rs:382-385` bekleme
0.5/1/2/4/5/5 sn; 5. deneme ~12.5 sn'de başlar, toplam ~62 sn — 60 sn sınırının
tam sınırında.

**Düzeltme.** `DEVAM_SURESI`'yi 60 → 120 sn yapın, ya da izleyiciye standart bir
`DevamJetonu` ömrü koyun ve jeton geçerliliği host'ta saatsel olarak denetleyin
(şu an `DevamBilgisi` içinde süre yok, host.rs:218-223). İkincisi doğru olan:
pencere uzatsın, jeton kısa ömürlü olsun.

---

## E8 — LOW: `AZAMI_MESAJ = 32 MB` her akış için ayrı ayrı ayrılmış, sınır yok

**Kanıt.** `protokol.rs:13` `AZAMI_MESAJ: usize = 32 * 1024 * 1024`. `protokol.rs:177-191`
`oku()` her mesajda bu üst sınırı kontrol ediyor (test `dev_uzunluk_reddedilir` ile
doğrulanmış, iyi). Ancak `izleyici.rs:517-525` her `accept_uni()` için ayrı bir
`tokio::spawn` açıyor ve eşzamanlı akış sayısına sınır yok. Karşı taraf binlerce
akış açabilir.

**Düzeltme.** `izleyici.rs:517` etrafında bir atomik sayaçla eşzamanlı akışları
`AZAMI_UCUSTA` (host.rs:34, zaten 3) gibi sabit bir tavanla sınırlayın; fazlasını
kapatın. `host.rs` tarafındaki `accept_uni` döngüsü için de aynısı.

---

## E9 — LOW: `SURUM` uyumsuzluğu bağlantıyı öldürüyor, sürüm müzakere edilebilir olabilir

**Kanıt.** `protokol.rs:11` `SURUM: u32 = 4`, `host.rs:467-470` ve `izleyici.rs`
her ikisi de eşitlik zorunlu kılıyor (`SURUM_FARKLI` reddi). ALPN `afudesk/2`
(`protokol.rs:10`) zaten iki sürümü ayırabilirdi.

**Düzeltme.** `ALPN`'yi sürümlü yapın (`afudesk/2` vs `afudesk/3`) ve ALPN
uyuşmazlığında sessiz düşüş yerine açık hata verin. Kısa vadede mevcut davranış
güvenli — bu bir performans değil, bakım kolaylığı notu.

---

## Doğru olan taraflar (korunmalı)

- `baglan()` (ag.rs:215-231) `c.remote_id() == beklenen` doğrulamasını **taşıma
  katmanı seviyesinde** yapıyor; MITM kodu ele geçirerek değil, uç nokta taklit
  ederek yapılsa bile yakalanır.
- Bilet `sabit_zamanli_esit` ile karşılaştırılıyor (host.rs:426-432) — zamanlama
  sızıntısı yok.
- Komut yarışı düzeltilmiş: host.rs:541-543, onay komutu istekten **önce** kuyruk
  temizleniyor.
- `cevrimici` (ag.rs:172-174) yalnız `ep.online()` bekliyor; relay'e yazma
  göndermiyor.
- `genel_mi()` (ag.rs:119-132) CGNAT 100.64/10 ve özel adresleri doğru eliyor.

## Test kapsamı boşluğu

92 test geçti ama tamamı `Kurulum::YalnizYerel` üzerinde. `ag::testler` içindeki
`dogru_kimlige_baglanir_ve_yol_dogrudan` ve `gercek_mdns_kimlikle_bulunur`
(ag.rs:343) mDNS'i sınadığı için yararlı. Eksik olan:

1. **Merkez kapalıyken** endpoint hâlâ çalışıyor mu? (E1/E2'nin regresyon testi)
2. `yayinlanacak()` çıktısı kullanıcı tarafından verilen relay listesini içeriyor mu?
3. Çoklu eşzamanlı `accept_uni` fırtınası (E8).
4. `kod_acik` kapanınca kod üretimi duruyor mu (E6)?

## Uygulama sırası

1. **E1** — `presets::N0` → `presets::Minimal` + `clear_address_lookup()` +
   `relay_mode(...)`. Tek dosya, `ag.rs:77-110`. Merkezileşmeyi kaldıran asıl adım.
2. **E2** — `uc_nokta_kimlikli`'ye relay listesi parametresi; `AFUDESK_RELAY` ortam
   değişkeni. E1'den sonra zorunlu (aksi hâlde relay'siz yapılandırma imkânsız).
3. **E5.1** — parolayı 10 haneye çıkar (tek satır, `kod.rs:133`).
4. **E3** — relay'siz durumda `Uyari` üret (`host.rs:326`).
5. **E4** — mDNS koşulunu `!= YalnizYerel` yap (`ag.rs:89`).
6. **E7** — `DevamBilgisi`'na jeton ömrü + doğrulama (`host.rs:218`).
7. **E6** — `calis` ana döngüsünde komut dinleme (`host.rs:365`).
8. **E8** — eşzamanlı akış tavanı (`izleyici.rs:517`).

1-3 birlikte bir sürüm olarak çıkarılmalı: E1 tek başına relay'siz bırakır, E2
olmadan da merkeziyetçi kalır. İkisi birlikte "kendi relay'ın var" modunu açar.

## Düzeltme durumu

Codex uyguladı, Claude denetleyip 3 noktayı düzeltti (29 Eyl 2026).

- **E1 — tamamlandı.** `ag.rs`: `presets::N0` yerine `presets::Minimal` + `clear_address_lookup()`; hiçbir pkarr/DNS yayını yok, yalnız mDNS. Test: `kurulan_endpointte_n0_address_lookup_yoktur`.
- **E2 — tamamlandı.** `AFUDESK_RELAY`: boşsa varsayılan relay **yalnız yedek taşıma yolu** (kimlik yayınlanmaz; n0 kapansa da koddaki doğrudan adresler + mDNS çalışır), `kapali`/`yok`/`off`/`0` relay'i tamamen kapatır, URL listesi özel relay'leri kullanır ve koda yazar. *Denetim düzeltmesi:* codex boş listede relay'i tamamen kapatıyordu → NAT arkasındaki herkes için internet bağlantısı kırılırdı. Test: `relay_listesi_ayristirilir_ve_bos_liste_merkeze_dusmez`, `yayinlanan_relayler_kullanici_relayini_koda_katar`.
- **E3 — tamamlandı.** `host.rs`: relay'e ulaşılamadı ve genel adres yoksa "Bu kod yalnız aynı ağda çalışır." uyarısı. Test: `relay_ve_genel_adres_yoksa_erisim_uyarisi_gerekir`.
- **E4 — tamamlandı.** mDNS YalnizYerel dışında açık. Test: `mdns_yalniz_yerel_haric_tum_kurulumlarda_acik`.
- **E5 — tamamlandı.** 10 haneli parola (`gen_range`, yanlılık yok), kod başına 3 hatalı denemede yeni kod. Test: `uc_hatali_bilet_kod_dongusunu_yeniler`.
- **E6 — tamamlandı.** `kod_ac(false)` bekleyen kodu hemen geçersiz kılar. *Denetim düzeltmesi:* codex bunu `HostKomut::Kes` ile yapıyordu; uygulamadaki `hostDurdur` açık oturumu/bekleyen onayı keserdi → ayrı `Notify` kullanıldı. Test: `kod_ekrani_kapaninca_yeni_kod_uretilir`, `kod_kapaliyken_kodla_girilmez_kayitli_cihaz_yine_baglanir`.
- **E7 — tamamlandı.** Devam jetonu ömrü 120 sn (`DevamBilgisi.jeton_omru`), izleyici 90 sn içinde kalır. Test: `izleyici_yeniden_deneme_jeton_omrunden_kisa`.
- **E8 — tamamlandı.** İzleyicide eşzamanlı tek yönlü akış tavanı (semaphore). *Denetim düzeltmesi:* tavan 3 → 64 (dosya akışları + kareler birlikte kare düşürmesin). Test: `akim_tavani_bos_yeni_akimi_reddeder`.
- **E9 — tamamlandı.** ALPN `afudesk/5` (protokol sürümü; `SURUM` alanı mesajlardan çıktı). *Denetim düzeltmesi:* codex ALPN'yi Cargo paket sürümüne bağlamıştı (her yamada uyumsuzluk); ayrıca uyuşmazlıkta "ulaşılamadı" yerine açık `SURUM_FARKLI` mesajı. Test: `alpn_protokol_surumunu_tasir`, `farkli_protokol_surumu_acik_hata_verir`.

**Doğrulama (Claude, bu makine):** `cargo test --release` → **103 geçti, 0 kaldı** (`reports/cargo-test-release.txt`). `cargo clippy --release --all-targets` → yeni uyarı yok (kalan 9 uyarı değişmeyen dosyalarda, önceden vardı). Flutter SDK bu makinede yok: `flutter analyze/test` koşulmadı; app tarafında tek değişiklik entegrasyon testindeki parola deseni (6→10 hane).

Not: eski sürümle (ALPN `afudesk/2`) bağlantı kurulmaz; iki taraf da bu sürüme güncellenmeli.
