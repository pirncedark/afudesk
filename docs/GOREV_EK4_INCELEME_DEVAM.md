# EK4 devam kod incelemesi (salt okunur)
ONAY: Kapsam kullanıcı tarafından onaylı; requesting-code-review becerisinin istediği bağımsız inceleme. Yalnız oku, dosya yazma, cargo/test çalıştırma, commit/push yapma. Başka işler aynı kod tabanında; değişiklikleri geri alma.
ÇALIŞMA DİZİNİ: C:/Users/afuuu/Desktop/afuproject/_wt/afudesk-linux
ÖNCE OKU: AGENTS.md ve C:/Users/afuuu/Desktop/afuproject/_gorev/2026-11-11/AFUDESK_LINUX.md (özellikle EK4).
Git metadata: git --git-dir=.linux-publish --work-tree=.
Taban: 02fad08. Çalışma ağacında önceki oturumun EK4 uygulaması mevcut.

📍 C:/Users/afuuu/Desktop/afuproject/_wt/afudesk-linux/app/kopru/src/api/google.rs:74 KaliciOturum; :179 guveni_dogrula; :276 kalici_yenile; :413 islem_yap. Yerel imzalı trust/anchor, 30gün expiry, hesap/key/client bağlama, rotation, geriYukle/duraklat/revoke akışlarını incele. API çağrıları yanlışlıkla iptal edilmiş cihazı geri kaydediyor mu, aynı hesap girişinin bağımsız key onayını atlıyor mu? İncele, düzeltme yapma.
📍 C:/Users/afuuu/Desktop/afuproject/_wt/afudesk-linux/rust/src/cihaz_guveni.rs:64 alanlar ve :124 dogrula. İmza ve expiry/rollback kontrollerini incele. Kontrol karakterli client ID RED testi ana ajan tarafından düzeltilecek; ayrıca raporlamana gerek yok.
📍 C:/Users/afuuu/Desktop/afuproject/_wt/afudesk-linux/rust/src/google_oauth.rs:300 oturum_ile ve :452 yenileme_yaniti. Mock HTTP sadece cfg(test), üretim HTTPS/PKCE/JWT/nonce. Yanlış tip refresh_token RED ana ajan tarafından düzeltilecek.
📍 C:/Users/afuuu/Desktop/afuproject/_wt/afudesk-linux/app/lib/hesap/gercek_hesap_servisi.dart:180 _sonKaydiSakla ve :197 geriYukle. Rotation persistence, çıkış yarışı, G6/relogin, gizli saklama sınırlarını incele.
📍 C:/Users/afuuu/Desktop/afuproject/_wt/afudesk-linux/rust/src/host.rs: google_izinleri ve baglanti_isle; mevcut Google/key onayı, yetki/revoke kontrollerini incele. Windows legacy ağ varsayılanları ve ALPN afudesk/8, SURUM=8 korunmalı.

SONUC: Kritik/önemli bulguları tam dosya:satır + tetikleyici + kullanıcı etkisi + önerilen çözüm olarak ana ajana döndür. Yeni dosya üretme. Linux fallback secure storage şu anda eksik ve ana ajan değerlendirmesinde; bilinen eksik olarak tekrar raporlamana gerek yok. Gerçek Wayland/Google Console/donanım doğrulaması yok, DONE sayma.
