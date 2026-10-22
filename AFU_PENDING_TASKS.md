# AfuDesk — Görev durumu

22 Ekim 2026; bağımsız düzeltme dalı fix/mobile-session-resource-audit.

| İş | Kod / otomatik test | Gerçek cihaz |
|---|---|---|
| WGC yakalama kapatma ve erken çıkış temizliği | Uygulandı; yerel sahte ekran testi | Sarı çerçeve ve 5 saat ölçüm bekliyor |
| 10 yeniden bağlantıda kaynak bırakma | Aynı host üzerinde 10 tur geçti | Google/telefon testi bekliyor |
| Masaüstü–pencere geçişi | 10 tur widget testi geçti | Bekliyor |
| Telefon yönü ve pencere oranı | Dikey/yatay başlangıç testi geçti | Canlı döndürmede yeniden seçim gerekir |
| Space bas/bırak/iptal | Otomatik testler geçti | Uzak konuşma uygulaması bekliyor |
| IME/Türkçe/dikte | Metin hattı var; otomatik IME testi var | Gboard/Samsung testi bekliyor |
| Google oturumu ve hesap görünümü | Mevcut testler geçti; yeniden yazılmadı | Canlı hesap testi bekliyor |
| Yeni sürümün görünmesi | Dönüşte denetim + görünür Güncelle düğmesi test edildi | Yeni APK kurulum testi bekliyor |
| Telefon mikrofonu sesi | EKSİK | Aktarım hattı uygulanmadı |
| Arka planda erişim | Mevcut Google hesabı/onay politikası korundu | Windows oturumunda test bekliyor |

Push/PR/CI/Release sonuçları AJAN_LOG.md içine yazılır. Gerçek cihaz sonucu olmadan görev bütünü DONE sayılmaz.
