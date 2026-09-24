# AfuDesk Görev Kapısı — Denetim

Tarih: 2026-09-24 15:01  ·  Commit: `52ad66a`

## FASTPATH — Düşük gecikmeli görüntü ve ölçüm

Zorunlu: 6 · Tamam: 6 · Eksik: 0

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Her kare ayrı QUIC akışında, geciken kare iptal + döşemeleri yeniden gönderme | ✅ DONE | ✅ | ✅ |  |
| RTT ve iptal sayısına göre otomatik kalite/FPS | ✅ DONE | ✅ | ✅ |  |
| Oturumda RTT/FPS göstergesi | ✅ DONE | ✅ | ✅ |  |
| Yakalamadan izleyicide birleşmeye gerçek gecikme (saat farkı düzeltmeli) | ✅ DONE | ✅ | ✅ |  |
| Bilinen adres adayları arasında en düşük RTT'li yolun seçimi | ✅ DONE | ✅ | ✅ |  |
| Windows Graphics Capture ile yakalama (xcap yedeği) | ✅ DONE | ✅ | ✅ |  |

## OYUN_MODU — Couch Co-op: uzak oyun kolu

Zorunlu: 6 · Tamam: 6 · Eksik: 0

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Kol durumu mesajı ve titreşim mesajı | ✅ DONE | ✅ | ✅ |  |
| Kol durumu QUIC datagram ile, eski sıra atılır (sarmal güvenli) | ✅ DONE | ✅ | ✅ |  |
| İzleyicide fiziksel kolu okuma ve XInput düzenine eşleme | ✅ DONE | ✅ | ✅ |  |
| Host'ta sanal Xbox 360 kolu (ViGEmBus), yoksa açık uyarı | ✅ DONE | ✅ | ✅ |  |
| Titreşimin izleyiciye geri gitmesi | ✅ DONE | ✅ | ✅ |  |
| Onay penceresinde 'Oyun kolu' izni | ✅ DONE | ✅ | ✅ |  |

## SONRAKI — Sıradaki özellikler

Zorunlu: 4 · Tamam: 2 · Eksik: 2

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Pano (metin) paylaşımı, izne bağlı | ✅ DONE | ✅ | ✅ |  |
| Dosya aktarımı (ayrı düşük öncelikli akış, kaldığı yerden devam) | ✅ DONE | ✅ | ✅ |  |
| Donanımsal H.264 (NVENC/QSV/AMF) algılama ve düşük gecikmeli kodlama | ⬜ TODO | ❌ | ❌ | kod yok: H264; test geçmedi/yok: kodek |
| Ağa göre otomatik çözünürlük | ⬜ TODO | ❌ | ❌ | kod yok: cozunurluk; test geçmedi/yok: cozunurluk |

## Toplam

| İSTENEN | UYGULANAN | TEST EDİLEN | GEÇEN | BAŞARISIZ | ENGELLİ | EKSİK | TEST EDİLMEYEN |
|---|---|---|---|---|---|---|---|
| 16 | 14 | 14 | 14 | 0 | 0 | 2 | 2 |

**SONUÇ: ❌ NOT_READY**
