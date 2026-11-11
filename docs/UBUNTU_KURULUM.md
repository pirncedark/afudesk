# AfuDesk Linux v0.1.0 — Ubuntu kurulumu

Durum: kurulum yolu ve paket sözleşmesi hazırlanmıştır; gerçek Ubuntu/Proxmox donanımı henüz yoktur. Linux derlemesi, `.deb` kurulumu ve gerçek masaüstü oturumu ancak [test raporundaki](LINUX_TEST_RAPORU.md) kanıtla tamamlanmış sayılır. Bu belge kendisi bir başarılı derleme kanıtı değildir.

## Hangi makine ne yapar?

| Cihaz adı | Sistem | Görev |
|---|---|---|
| `afu-pve` | Fiziksel Proxmox VE sunucusu | Sanal makineleri çalıştırır; yönetim web arayüzü ve API |
| `afu-server` | Proxmox içindeki Ubuntu Server **24.04 LTS** VM | Sunucu işleri; SSH ile yönetim |
| `afu-masaustu` | Ubuntu Desktop **24.04 LTS**, amd64 | AfuDesk grafik istemcisi, ekran + klavye + fare oturumu |

Ubuntu Server VM'de grafik masaüstü bulunması gerekmez. AfuDesk bu sürümde başsız bir sistem servisi veya Proxmox yönetim paneli değildir. Desktop uygulaması oturum açmış kullanıcının masaüstünde çalışır; `sudo afudesk` kullanmayın. SSH/API yönetimi için aşağıdaki belgeler bir yönetici hazırlık yoludur; uygulamada tamamlanmış bir Proxmox entegrasyonu olduğunu göstermez.

## 1. Sunucuyu hazırlayın

Proxmox'u üretici ISO'suyla fiziksel makineye kurun; Ubuntu Server 24.04 ISO'suyla ayrı VM oluşturun. İlk kurulum ağını güvenilir yerel ağla sınırlayın. Proxmox hostuna Ubuntu deposu eklemeyin. Proxmox sürümüne karşılık gelen Debian deposunu koruyun.

VM'ye VirtIO disk/ağ ve QEMU guest agent seçin. Ubuntu VM içinde:

```bash
sudo apt-get update
sudo apt-get install -y qemu-guest-agent
sudo systemctl start qemu-guest-agent
systemctl is-active qemu-guest-agent
```

VM kaynakları ve ekran aktarım performansı fiziksel donanımda ölçülecektir; doğrulanmış bir minimum kaynak iddiası yoktur. Sunucu ve host için [Linux sunucu güvenliği](LINUX_SUNUCU.md) adımlarını tamamlayın. Tailscale'i hosta, VM'ye ve yönetim istemcisine ayrı kurun; VM'ye erişim için hostu subnet router/exit node yapmanız gerekmez.

## 2. Tailscale'i güvenli paket deposuyla kurun

Resmî imzalı apt deposunu kullanan betik Ubuntu 24.04 ve Proxmox'un Debian bookworm/trixie tabanlarını tanır. Betik depoda incelenebilir; uzaktan indirilmiş betiği shell'e aktarmayın.

```bash
sudo bash deploy/linux/install-tailscale.sh
# Her makinede yalnız ilgili cihaz adını seçin:
sudo tailscale up --hostname=afu-masaustu
tailscale status
```

Host için `afu-pve`, VM için `afu-server` adını kullanın. Tarayıcıda kendi tailnet hesabınızla onaylayın; komut satırına kayıtlı auth key/secret yazmayın. Var olan Tailscale kurulumunda yeniden `up` çalıştırmak yerine durumu inceleyin; yalnız ad gerekiyorsa `sudo tailscale set --hostname=afu-masaustu` kullanın. Mevcut ACL, netfilter modu, subnet/exit-node ve Tailscale SSH ayarları bu kurulum tarafından değiştirilmez. [Resmî paket kaynağı](https://pkgs.tailscale.com/stable/) dağıtım komutlarını yayımlar.

MagicDNS etkinse cihaz adlarıyla erişilir; Tailscale hesabından silip yeniden kaydetmek adresi değiştirebilir. Adlandırma kaydı, SSH host anahtarı ve sertifika doğrulaması birlikte korunmalıdır. Bu özel ağ yöneticilerin fiziksel bağlantı adreslerini göremeyeceği anlamına gelmez; paylaşım davetinde genel IP taşımamak ayrı uygulama kuralıdır.

## 3. Ubuntu Desktop'a paketi kurun

CI'nin Linux `.deb` artifact'ını indirin ve içindeki tek uygun amd64 paketini seçin. Artifact adı ve gerçek derleme sonucu CI/test raporundan doğrulanmalıdır. Aşağıdaki `AFUDESK_DEB` değerini indirdiğiniz dosyaya göre doldurun:

```bash
AFUDESK_DEB="$HOME/Downloads/afudesk_0.1.0_amd64.deb"
test -f "$AFUDESK_DEB"
dpkg-deb --info "$AFUDESK_DEB"
dpkg --print-architecture
sudo apt install "$AFUDESK_DEB"
afudesk
```

Kurulum sözleşmesi `/opt/afudesk` uygulama dosyaları ve `/usr/bin/afudesk` başlatıcısıdır. Paket bağımlılıklarını apt çözer. Mimari `amd64` olmalı; farklı mimari için bu paketi zorlamayın. CI çıktısı henüz yoksa kurulum adımı **UNVERIFIED** kalır. Uygulamalar menüsündeki AfuDesk ile açıp cihazı adıyla seçin ve ana bağlantı düğmesine basın. Giriş/eşleştirme izni gerektiğinde mevcut uygulama akışını izleyin; hesap girişi tek başına eşleşmiş cihaz anahtarının yerini tutmaz.

## 4. Bu sürümde X11 kullanın

Ubuntu giriş ekranında kullanıcıyı seçin; parola girmeden önce dişli menüsünden **Ubuntu on Xorg** seçeneğini seçin ve oturum açın. Kontrol komutu:

```bash
printf '%s\n' "$XDG_SESSION_TYPE"
afudesk
```

Beklenen oturum türü `x11`'dir. Mevcut Rust yolu `rust/src/platform.rs` içindeki `XcapYakalayici` ile ekran yakalama ve `EnigoEnjektor` ile klavye/fare girdisidir. Xvfb'de kare alınması, gerçek GPU/çoklu ekran/ölçekleme/klavye düzeninin doğrulanması anlamına gelmez.

**Wayland tam desteği tamamlanmış değildir.** Uygulamaya ait xdg-desktop-portal ScreenCast + RemoteDesktop izin oturumu, PipeWire akışı ve ilgili input yaşam döngüsünün tamamı doğrulanmadan Wayland'de ekran + klavye + fare çalışıyor denmez. PipeWire veya portal paketini kurmak tek başına bu eksiği çözmez; `uinput` için geniş cihaz izni vermeyin ve uygulamayı root çalıştırmayın. İlk kullanılabilir yol Xorg'dur; kalıcı sistem geneli Wayland kapatma işlemi gerekmez.

Ses/mikrofon, sanal gamepad, güncelleyici ve Google OAuth Linux davranışının her biri ayrıca test raporundan kontrol edilmelidir; Windows desteğinden Linux eşdeğerliği çıkarılamaz. Google OAuth token ömrü ile AfuDesk'in kendi 30 günlük cihaz güveni ayrı mekanizmalardır; Google Console Testing/Production durumu kodla değiştirilmez.

## 5. Güvenli çalışma kontrolü

Linux ağ yolu Tailscale bulunduğunda onun adresini tercih eder; bulunmadığında loopback ile güvenli kapalı kalma hedefi test raporunda doğrulanmalıdır. Port açma, UPnP, genel IP paylaşma veya parola tabanlı SSH alternatifi kurmayın. Tailscale kurulduktan sonra AfuDesk'i yeniden başlatın. `n0` relay yalnız yedek yoldur; uygulama kimliği genel keşif sistemine yayımlanmaz ve protokol `afudesk/8` olarak korunur. Relay kullanımı herkese erişim izni vermez.

İki kayıtlı cihazda şu gerçek kullanım adımları hâlâ **UNVERIFIED**'dır: adıyla keşif, yetkisiz anahtarın reddi, ilk ekran karesi, klavye/fare, yeniden başlatma, Tailscale yokken güvenli kapanma ve paylaşım verisinde genel IP bulunmaması. Son durum için [uyumluluk raporuna](UYUMLULUK_RAPORU.md) bakın.
