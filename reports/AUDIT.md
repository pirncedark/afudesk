# AfuDesk Görev Kapısı — Denetim

Tarih: 2026-09-24 13:47  ·  Commit: `285e14b`

## FASTPATH — Düşük gecikmeli görüntü ve ölçüm

Zorunlu: 6 · Tamam: 3 · Eksik: 3

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Her kare ayrı QUIC akışında, geciken kare iptal + döşemeleri yeniden gönderme | ✅ DONE | ✅ | ✅ |  |
| RTT ve iptal sayısına göre otomatik kalite/FPS | ✅ DONE | ✅ | ✅ |  |
| Oturumda RTT/FPS göstergesi | ✅ DONE | ✅ | ✅ |  |
| Yakalamadan izleyicide birleşmeye gerçek gecikme (saat farkı düzeltmeli) | ⬜ TODO | ❌ | ❌ | kod yok: fn saat_farki, fn gecikme_ms, yakalama_ms; test geçmedi/yok: zaman:: |
| Bilinen adres adayları arasında en düşük RTT'li yolun seçimi | 🟡 IN_PROGRESS | ❌ | ❌ | kod yok: fn en_iyi_yol; test geçmedi/yok: en_iyi_yol |
| Windows Graphics Capture ile yakalama (xcap yedeği) | ⬜ TODO | ❌ | ❌ | kod yok: WgcYakalayici; test geçmedi/yok: gercek_wgc_yakalanir |

## OYUN_MODU — Couch Co-op: uzak oyun kolu

Zorunlu: 6 · Tamam: 0 · Eksik: 6

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Kol durumu mesajı ve titreşim mesajı | 🟡 IN_PROGRESS | ❌ | ✅ | kod yok: KolDurumu, Titresim |
| Kol durumu QUIC datagram ile, eski sıra atılır (sarmal güvenli) | 🟡 IN_PROGRESS | ❌ | ✅ | kod yok: datagram |
| İzleyicide fiziksel kolu okuma ve XInput düzenine eşleme | 🟡 IN_PROGRESS | ❌ | ✅ | kod yok: gilrs |
| Host'ta sanal Xbox 360 kolu (ViGEmBus), yoksa açık uyarı | ⬜ TODO | ❌ | ❌ | kod yok: KolSurucusu, ViGEmBus; test geçmedi/yok: sanal_kol |
| Titreşimin izleyiciye geri gitmesi | ⬜ TODO | ❌ | ❌ | kod yok: Titresim; test geçmedi/yok: titresim |
| Onay penceresinde 'Oyun kolu' izni | ⬜ TODO | ❌ | ❌ | kod yok: oyun_kolu, Oyun kolu; test geçmedi/yok: oyun kolu |

## SONRAKI — Sıradaki özellikler

Zorunlu: 4 · Tamam: 0 · Eksik: 4

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Pano (metin) paylaşımı, izne bağlı | ⬜ TODO | ❌ | ❌ | kod yok: fn ; test geçmedi/yok: pano |
| Dosya aktarımı (ayrı düşük öncelikli akış, kaldığı yerden devam) | ⬜ TODO | ❌ | ❌ | kod yok: fn ; test geçmedi/yok: dosya |
| Donanımsal H.264 (NVENC/QSV/AMF) algılama ve düşük gecikmeli kodlama | ⬜ TODO | ❌ | ❌ | kod yok: H264; test geçmedi/yok: kodek |
| Ağa göre otomatik çözünürlük | ⬜ TODO | ❌ | ❌ | kod yok: cozunurluk; test geçmedi/yok: cozunurluk |

## Toplam

| İSTENEN | UYGULANAN | TEST EDİLEN | GEÇEN | BAŞARISIZ | ENGELLİ | EKSİK | TEST EDİLMEYEN |
|---|---|---|---|---|---|---|---|
| 16 | 3 | 7 | 6 | 1 | 0 | 13 | 9 |

**SONUÇ: ❌ NOT_READY**
