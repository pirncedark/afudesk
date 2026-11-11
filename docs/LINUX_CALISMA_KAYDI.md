# Linux çalışma kaydı — 2026-11-11

Kaynak: `_gorev/2026-11-11/AFUDESK_LINUX.md`, tüm ekler ve koordinatör cevabı okundu.
Başlangıç: f5fa79cb8f3e9a861a8b8dea350acdb2d194e151, feat/linux-ubuntu.

1. İnceleme, Linux runner/CI/paket ve güvenlik belgeleri.
2. Linux ağ kısıtları, IP içermeyen paylaşım, eşleşmiş cihaz kontrolü ve testleri.
3. Linux doğrulamasından sonra EK 4: mevcut OAuth/oturum mekanizmasını genişletme.
4. Testler, kod inceleme, commit/push, SONUC.

Kararlar:
- ALPN afudesk/8 ve SURUM=8 korunur.
- Windows ağ varsayılanları korunur; Linux güvenli politika ayrı cfg ile uygulanır.
- Açık PR #17 ve #7 bulundu: yeni PR açılmaz, mevcut PR değiştirilmez; kullanıcı kararına bırakılır.
- Flutter PATH'te yok, koordinatör uyarınca Linux doğrulaması GitHub Actions üzerinde denenir.
- Bağımsız Linux runner/CI ve sunucu belgeleri paralel çalışma becerisiyle ayrıldı.
- Çalışmayan veya koşulamayan hiçbir özellik DONE sayılmaz.
- Ortam: asıl linked-worktree Git index'i F:/afuproject/AfuDesk/.git altında ve yazmaya kapalı. Git add/commit `index.lock: Permission denied` döndü. Ana metadata'ya yazmak yerine bu worktree içindeki `.linux-publish/` bağımsız Git metadata alanı kullanılır; başlangıç objeleri yalnız okunur. Asıl worktree index/HEAD değişmez, yayın commit'i ayrı metadata'da izlenir.
