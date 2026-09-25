# AfuDesk Görev Kapısı — Denetim

Tarih: 2026-09-25 04:57  ·  Commit: `d36e711`

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

## KAYITLI_CIHAZLAR — Kay?tl? (g?venilen) cihazlar

Zorunlu: 4 · Tamam: 2 · Eksik: 2

| Madde | Durum | Kod | Test | Not |
|---|---|---|---|---|
| Kal?c? cihaz kimli?i ve TLS sertifikas? | ✅ DONE | ✅ | ✅ |  |
| Atomik JSON e?le?me kayd? | ✅ DONE | ✅ | ✅ |  |
| Kay?tl? cihaz kimlik/jeton do?rulamas? | 🟡 IN_PROGRESS | ✅ | ❌ | test geçmedi/yok: uctan_uca::kayitli_cihaz_kodsuz_baglanir, uctan_uca::kayitli_cihaz_onaysiz_baglanamaz, uctan_uca::kaldirilan_cihaz_reddedilir, uctan_uca::yanlis_jeton_reddedilir, uctan_uca::host_sertifikasi_degisirse_reddedilir |
| Kay?tl? cihazlar? y?netme aray?z? | ⬜ TODO | ❌ | ❌ | kod yok: Kay?tl? cihazlar, G?venilen cihazlar; test geçmedi/yok: kay?tl? cihazlar listesi, kay?tl? cihaz? unut, g?venilen cihaz? kald?r |

## Toplam

| İSTENEN | UYGULANAN | TEST EDİLEN | GEÇEN | BAŞARISIZ | ENGELLİ | EKSİK | TEST EDİLMEYEN |
|---|---|---|---|---|---|---|---|
| 20 | 17 | 16 | 16 | 1 | 0 | 4 | 4 |

**SONUÇ: ❌ NOT_READY**
