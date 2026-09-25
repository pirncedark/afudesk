# AfuDesk Görev Kapısı — Denetim

Tarih: 2026-09-25 18:04  ·  Commit: `a54d791`

## FASTPATH — Düşük gecikmeli görüntü ve ölçüm

Zorunlu: 6 · Tamam: 6 · Eksik: 0

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Her kare ayrı QUIC akışında, geciken kare iptal + döşemeleri yeniden gönderme | ✅ DONE | ✅ | ✅ |  |
| RTT ve iptal sayısına göre otomatik kalite/FPS | ✅ DONE | ✅ | ✅ |  |
| Oturumda RTT/FPS göstergesi | ✅ DONE | ✅ | ✅ |  |
| Yakalamadan izleyicide birleşmeye gerçek gecikme (saat farkı düzeltmeli) | ✅ DONE | ✅ | ✅ |  |
| En düşük RTT'li yolun seçimi (v1.3: iroh yol seçicisi — doğrudan yollar RTT'ye göre, relay yedek) | ✅ DONE | ✅ | ✅ |  |
| Windows Graphics Capture ile yakalama (xcap yedeği) | ✅ DONE | ✅ | ✅ |  |

## OYUN_MODU — Couch Co-op: uzak oyun kolu

Zorunlu: 7 · Tamam: 7 · Eksik: 0

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Kol durumu mesajı ve titreşim mesajı | ✅ DONE | ✅ | ✅ |  |
| Kol durumu QUIC datagram ile, eski sıra atılır (sarmal güvenli) | ✅ DONE | ✅ | ✅ |  |
| İzleyicide fiziksel kolu okuma ve XInput düzenine eşleme | ✅ DONE | ✅ | ✅ |  |
| Host'ta sanal Xbox 360 kolu (ViGEmBus), yoksa açık uyarı | ✅ DONE | ✅ | ✅ |  |
| Titreşimin izleyiciye geri gitmesi | ✅ DONE | ✅ | ✅ |  |
| Onay penceresinde 'Oyun kolu' izni | ✅ DONE | ✅ | ✅ |  |
| Android Bluetooth ve dokunmatik kol, izin, çoklu dokunma ve sade bağlantı hatası | ✅ DONE | ✅ | ✅ |  |

## WAN — Farklı internet bağlantıları arasında ayarsız bağlantı

Zorunlu: 4 · Tamam: 4 · Eksik: 0

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Doğrudan + NAT delme + relay yedeği (iroh), kimlik koddan doğrulanır | ✅ DONE | ✅ | ✅ |  |
| Doğrudan yol kapatılınca relay'e düşme (AFUDESK_SADECE_RELAY) | ✅ DONE | ✅ | ✅ |  |
| Kopunca onay sormadan yeniden bağlanma; görüntü ve girdi sürer | ✅ DONE | ✅ | ✅ |  |
| Oturumda yeniden bağlanma ve 'aktarmalı' göstergesi | ✅ DONE | ✅ | ✅ |  |

## SONRAKI — Sıradaki özellikler

Zorunlu: 4 · Tamam: 2 · Eksik: 2

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Pano (metin) paylaşımı, izne bağlı | ✅ DONE | ✅ | ✅ |  |
| Dosya aktarımı (ayrı düşük öncelikli akış, kaldığı yerden devam) | ✅ DONE | ✅ | ✅ |  |
| Donanımsal H.264 (NVENC/QSV/AMF) algılama ve düşük gecikmeli kodlama | ⬜ TODO | ❌ | ❌ | kod yok: H264; test geçmedi/yok: kodek |
| Ağa göre otomatik çözünürlük | ⬜ TODO | ❌ | ❌ | kod yok: cozunurluk; test geçmedi/yok: cozunurluk |

## KAYITLI_CIHAZLAR — Kayıtlı (güvenilen) cihazlar

Zorunlu: 6 · Tamam: 6 · Eksik: 0

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Kalıcı cihaz kimliği (iroh anahtarı, host/izleyici ayrı) | ✅ DONE | ✅ | ✅ |  |
| Atomik JSON eşleşme kaydı | ✅ DONE | ✅ | ✅ |  |
| Kayıtlı cihaz kimlik/jeton doğrulaması, her seferinde onay | ✅ DONE | ✅ | ✅ |  |
| Uygulama açıkken arka planda dinleme; kod yalnız 'Bağlantı ver' açıkken | ✅ DONE | ✅ | ✅ |  |
| Yerel ağ keşfi (mDNS, cihaz kimliğiyle; IP değişse de) | ✅ DONE | ✅ | ✅ |  |
| Kayıtlı cihazları yönetme arayüzü | ✅ DONE | ✅ | ✅ |  |

## Toplam

| İSTENEN | UYGULANAN | TEST EDİLEN | GEÇEN | BAŞARISIZ | ENGELLİ | EKSİK | TEST EDİLMEYEN |
|---|---|---|---|---|---|---|---|
| 27 | 25 | 25 | 25 | 0 | 0 | 2 | 2 |

**SONUÇ: ❌ NOT_READY**
