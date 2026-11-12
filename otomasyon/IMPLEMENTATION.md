# Adım 1–5 uygulama kaydı

Kaynak: `_gorev/2026-11-12/GOREV_GOOGLE_OTOMASYON.md`; plan: `_gorev/2026-11-11/AFUDESK_GOOGLE_OTOMASYON_PLAN.md`.

1. İskelet ve doğrulanan izinli liste.
2. Repo dışında kalıcı profil ve ilk elle giriş için başlatma bilgisi.
3. Yerel giriş/hesap seçici testi.
4. Her durma noktası, yetki ve gezinme sınırları testi.
5. Yalnız izinli alanları yazan günlük ve sızıntı testi.

Ruling: Kullanıcının verdiği mevcut ayrı dal kullanılacak; başka worktree açılmayacak.
Ruling: Yeni çalışma alanı/plan dosyası yerine bu kayıt otomasyon/ içinde tutulacak; görev dosyasının yazma sınırı geçerli.
Ruling: GUNLUK.md ve dış SONUC dosyası, görevde açıkça istenen teslim istisnalarıdır.
Ruling: İlk elle giriş, otomasyon bayrakları olmadan aynı profil için Chrome başlatma bilgisi döndürür; bu adımda masaüstü arayüzü/IPC yapılmaz.
Ruling: Testler localhost HTTP kullanır; üretimde HTTPS zorunludur. Gerçek Google testlerde ağ seviyesinde engellenir.

Arayüz kontrolü: Şema → site ayarları → giriş; profil → kalıcı tarayıcı; giriş sonucu → filtrelenmiş JSONL. Yetki, durma ve kayıt tipleri ortak tutulacak.

Adım 1: Şema ve Node/TS iskeleti hazır; paket kilidi üretildi. Tam npm test exit 1 (tarayıcı ortam engeli).
Adım 2: Profil varsayılanları, repo/traversal/symlink reddi ve elle giriş komutu testleri geçti; Linux 700 kontrolü Windows üzerinde koşmadı.
Adım 3–4: Yerel sahte giriş, hesap seçici, popup, altı durma noktası, yetki ve yönlendirme testleri hazır; tarayıcı erişim engelinden doğrulanamadı.
Adım 5: Üç kayıt testi geçti; şifre/çerez/token/URL/mesaj alanları kayda kopyalanmıyor.

Ruling: Playwright test runner'ın alt süreç borusu Windows ortamında EPERM verdi; testler Node 20 yerel tek süreçli node:test ile, gerçek tarayıcı etkileşimi Playwright ile yürütülüyor.
Ruling: Test tarayıcısı Windows adlandırılmış boru kısıtlarına karşı CDP kullanır; üretimde launchPersistentContext aynen korunur. Chrome Crashpad/Mojo yine erişim engeli (0x5) ile kapanır; güvenlik bayrakları gevşetilmedi.
Ruling: Tarayıcı başlatma engeli sonrası güvenlik doğrulamalarını atlama/başarılı gösterme yok; tam test çıkış kodu 1, 24 test cancelled. Sonuç YARIM raporlanır; kullanıcının commit+push talimatı uygulanır.

Son inceleme: Ayrı salt okunur incelemeci üç Important buldu; kayıt bağlantısının hesap oluşturma sayılması, gizli ilk güvenlik öğesinin görünür ikincisini örtmesi ve Google sign in aria adı düzeltildi. Ek regresyon senaryoları eklendi. Minor açık oku yetkisi testi de eklendi.
Regresyon kanıtı: Kayıt bağlantısı politikası önce 1 failed/1 passed; düzeltme sonrası 2 passed. Tarayıcı regresyonları ortam engelinden UNVERIFIED.
Son doğrulama: Node v20.19.6; npm test → 43 test, 19 pass, 0 fail, 24 cancelled, 0 skipped; exit 1. tsc derlemesi geçti. Windows profil/kayıt/politika birim kontrolleri geçti.
Güvenlik kontrolü: Gitleaks 8.30.1 kaynak anlık kopyasını (~51.60 KB) taradı, no leaks found; git ls-files içinde profil/kayit verisi yok, git check-ignore ikisini de doğruladı. Staged tarama boş olduğu için kanıt sayılmadı.
Git engeli: git add, .git/index.lock oluştururken Permission denied verdi; ortam .git klasörünü salt okunur açmış. Commit oluşturulamadı; gönderilecek yeni commit olmadığı için push çalıştırılmadı.
Durum: YARIM — tarayıcı testleri ve Linux izinleri doğrulanamadı; commit/push dosya sistemi izni nedeniyle tamamlanamadı.
