**English** · [Türkçe](#türkçe)

## AfuDesk v1.10.2 — Mobile Experience, Session Resilience & Encoding Fixes

### Windows System Tray
- **Turkish Characters**: System tray context menu displays Turkish characters (ç, ğ, ı, İ, ö, ş, ü) perfectly without mojibake.

### Mobile Experience & Layout
- **Fullscreen Remote Display**: The remote desktop now spans 100% of the display; touch toolbar and navigation overlay cleanly over the screen without shrinking the desktop area.
- **Natural Space Key**: Space key interception removed in terminal mode so physical and on-screen keyboard spaces type immediately.

### Google Account & Session Resilience
- **Persistent Android Session**: Android secure storage is now fully wired, preventing repeated Google sign-in prompts when reconnecting.
- **Multi-Connection & Reconnect Support**: Reconnecting to Google-paired hosts cleanly accepts incoming session handshakes without protocol errors.
- **Account Device List & Profile**: Real account name, email address, and all paired devices (including Android) are now clearly displayed in Settings.
- **Seamless Resolution Switching**: Switching between Desktop and Terminal modes no longer drops the connection on in-flight tile frames.

---

## Türkçe

## AfuDesk v1.10.2 — Mobil Deneyim, Oturum Dayanıklılığı ve Karakter Düzeltmeleri

### Windows Sistem Tepsisi
- **Türkçe Karakterler**: Sistem tepsisi menüsündeki bozuk karakterler (Aç, Bağlantıları durdur, Çıkış) düzeltildi.

### Mobil Ekran ve Girdi
- **Tam Ekran Görünüm**: Uzak masaüstü görüntüsü artık tüm ekranı kaplar; kontrol panelleri görüntüyü daraltmaz, üzerine bindirilmiş (overlay) olarak çalışır.
- **Doğal Space Tuşu**: Terminal modundaki Space tuşu gecikmesi/engeli kaldırıldı, normal boşluk tuşu anında ve sorunsuz çalışır.

### Google Hesabı ve Oturum Dayanıklılığı
- **Android Kalıcı Oturumu**: Android güvenli anahtar kasası entegre edildi; aynı cihaza defalarca bağlanırken tekrar tekrar Google onayı istemez.
- **İkinci Bağlantı Desteği**: Bağlantıdan çıkıp yeniden bağlanıldığında oturum çakışması ve protokol hatası önlendi.
- **Hesap Bilgileri ve Cihaz Listesi**: Windows Ayarlar ekranında bağlı Google hesabının adı, e-postası ve bağlı tüm cihazlar eksiksiz listelenir.
- **Masaüstü ↔ Terminal Geçişi**: Çözünürlük değişimlerinde uçuşta kalan kareler oturumu koparmaz; geçişler akıcı ve kesintisizdir.
