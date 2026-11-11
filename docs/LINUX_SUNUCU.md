# AfuDesk — Ubuntu/Proxmox sunucu güvenliği

Bu belge yönetici içindir. Fiziksel sunucu henüz kurulmadı; aşağıdaki komutlar bu çalışma sırasında hiçbir sunucuda çalıştırılmadı. Kapsam tek Proxmox hostu + Ubuntu Server 24.04 VM'dir. Cluster, HA, replikasyon, subnet router veya exit node için bu profili doğrudan uygulamayın: Proxmox'un düğümler arası erişimi ve root SSH ihtiyaçları ayrıca tasarlanmalıdır.

## Önkoşul: kilitlenmeden ilerleyin

SSH/firewall/listen değişikliklerinden önce **açık ve denenmiş yerel fiziksel konsol** bulunmalı; Ubuntu VM için Proxmox konsolu da gerçekten açılmalıdır. Ayrıca ayrı yönetim cihazından Tailscale üzerinden **Ed25519 anahtarıyla yeni SSH oturumu ve sudo** çalıştığı doğrulanmalıdır. Mevcut oturumu kapatmayın. Sadece mevcut bir SSH oturumunun açık kalması yeni bağlantının çalıştığına kanıt değildir. Önkoşullardan biri eksikse listen/firewall adımlarını uygulamayın.

İlk kurulumları güvenilir, izole yerel ağda yapın. Her aşamada ayrı oturumla yeniden doğrulayın. Ayarların yedeğini değişiklikten önce alın; yerel konsoldan geri dönüş yolu aşağıdadır. İnternete SSH/8006 yönlendirmesi, UPnP, Tailscale Funnel veya genel IP daveti açılmaz. Tailscale erişim kuralları bu belgede **uygulanmaz/değiştirilmez**.

## 1. Ayrı yönetici ve Ed25519 anahtarı

Yönetim istemcisinde çalıştırın; benzersiz dosya adı mevcut anahtarı ezmez. Anahtar için parola belirleyin:

```bash
ssh-keygen -t ed25519 -a 100 -f "$HOME/.ssh/afudesk_admin" -C afudesk-admin
cat "$HOME/.ssh/afudesk_admin.pub"
```

Yeni sunucunun **yerel root konsolunda**, Ubuntu'da kurulumda oluşturulmuş sudo kullanıcısı yoksa (Proxmox'ta da ayrı Linux hesabı gerekir):

```bash
apt-get update
apt-get install -y sudo openssh-server ufw fail2ban unattended-upgrades
adduser afuadmin
usermod -aG sudo afuadmin
install -d -m 0700 -o afuadmin -g afuadmin /home/afuadmin/.ssh
# Aşağıdaki tek satırı istemcideki .pub çıktısıyla değiştirin; özel anahtarı ASLA kopyalamayın.
printf '%s\n' 'ssh-ed25519 YERINE_GERCEK_ACIK_ANAHTAR afudesk-admin' > /home/afuadmin/.ssh/authorized_keys
chown afuadmin:afuadmin /home/afuadmin/.ssh/authorized_keys
chmod 0600 /home/afuadmin/.ssh/authorized_keys
```

Kullanıcı zaten varsa `adduser`/`usermod` adımlarını yinelemeyin; mevcut `authorized_keys` dosyasını ezmeyin, açık anahtarı ekleyin. Konsol/sudo parolası yerel yetki içindir; SSH'de parola girişi kapatılacaktır. Özel anahtar, token ve kurtarma verisi repoya/loga girmez.

## 2. Tailscale ve sabit adlar

[Ubuntu kurulumundaki](UBUNTU_KURULUM.md#2-tailscalei-güvenli-paket-deposuyla-kurun) imzalı repo betiğini her makinede çalıştırın. Yeni hostta `sudo tailscale up --hostname=afu-pve`, yeni VM'de `sudo tailscale up --hostname=afu-server` çalıştırıp tarayıcıdan onaylayın. Mevcut kurulumda önce durumu inceleyin. Tailscale SSH özelliği etkinse port 22'yi kendi işleyebilir; bu belge normal **OpenSSH üzerinden Tailscale ağı** içindir. Mevcut Tailscale SSH ayarını otomatik değiştirmeyin; bu durumda anahtarlı OpenSSH doğrulaması ayrıca çözülmeden devam etmeyin.

İstemciden, ilk bağlantıda host anahtarı parmak izini yerel sunucu konsolundaki `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` çıktısıyla karşılaştırarak:

```bash
ssh -o PasswordAuthentication=no -o KbdInteractiveAuthentication=no -i "$HOME/.ssh/afudesk_admin" afuadmin@afu-server
# Yeni uzak oturum içinde:
sudo -v
```

Host için aynısını `afu-pve` ile doğrulayın. MagicDNS yoksa yönetici doğrulanmış Tailscale adresini kullanabilir; uygulamanın normal ekranında cihaz adı gösterilir.

## 3. SSH yalnız Tailscale IPv4'ünde ve yalnız anahtarla

Depo kökünden, ilgili makinedeki bash oturumunda (her host/VM kendi Tailscale adresini alır):

```bash
TAIL_IP=$(tailscale ip -4)
test -n "$TAIL_IP" && ip -4 addr show dev tailscale0 | grep -F "$TAIL_IP/"
sudo cp -a /etc/ssh /etc/ssh.afudesk-oncesi
sed "s/@TAILSCALE_IPV4@/$TAIL_IP/" deploy/linux/00-afudesk-ssh.conf.example | sudo tee /etc/ssh/sshd_config.d/00-afudesk.conf >/dev/null
sudo /usr/sbin/sshd -t
sudo /usr/sbin/sshd -T | grep -E '^(listenaddress|addressfamily|authenticationmethods|pubkeyauthentication|passwordauthentication|kbdinteractiveauthentication|permitrootlogin) '
```

Çıktı yalnız ilgili Tailscale IPv4 adresini; `publickey`, `passwordauthentication no`, `kbdinteractiveauthentication no`, `permitrootlogin no` değerlerini göstermelidir. OpenSSH birçok ayarda **ilk değeri** kullanır; `00-` öneki tek başına kanıt değildir. Ana dosyada daha önce gelen ayar, başka `ListenAddress` veya `Match` bloğu varsa inceleyin. İlgili kullanıcı/adres için de `sudo sshd -T -C user=afuadmin,host=afu-masaustu,addr=ISTEMCININ_TAILSCALE_IPSI` ile etkin ayarları doğrulayın. Beklenmeyen wildcard/başka adres veya parola varsa devam etmeyin. `AddressFamily inet` SSH için IPv6 dinleyicisini kapatır; sistem IPv6'sını kapatmaz.

**Ubuntu 24.04 `ssh.socket`:** sshd ayarı tek başına hâlihazırda açık socket'i daraltmaz. Bu profil Ubuntu'da socket etkinleştirmesinden normal servise geçer. Aşağıdaki geçişi yerel konsolda yapın (bağlantı kesilebilir):

```bash
sudo systemctl disable --now ssh.socket
sudo systemctl daemon-reload
sudo systemctl enable ssh.service
sudo systemctl restart ssh.service
sudo systemctl is-active ssh.service
sudo systemctl is-active ssh.socket  # Beklenen: inactive (çıkış kodu 3 normaldir)
sudo ss -lntp '( sport = :22 )'
```

Debian/Proxmox'ta `ssh.socket` yoksa yalnız mevcut `ssh.service`'i etkinleştirip yeniden başlatın. Aktif socket varsa aynı konsollu geçişi kullanın. `ss` yalnız `TAIL_IP:22` göstermeli; `0.0.0.0:22`, `*:22`, `[::]:22` olmamalı. İstemciden yeni anahtarlı SSH ve sudo oturumunu tekrar doğrulayın; parola ile ve root ile oturum açılmamalı. Ubuntu 24.04'ün socket/generator davranışı [Canonical açıklamasında](https://discourse.ubuntu.com/t/sshd-now-uses-socket-based-activation-ubuntu-22-10-and-later/30189/1) belgelenir.

Tailscale adresi açılışta henüz yoksa SSH bind başarısız olabilir; bu, genel ağa açılmaya dönüşmemelidir. Yeniden başlatma testi zorunludur. Konsolda Tailscale hazır olduktan sonra `sudo systemctl restart ssh.service` ile kurtarın. Boot sıralaması gerçek sunucuda doğrulanmadan gözetimsiz erişim garantisi yoktur.

## 4. Proxmox web/API yalnız Tailscale'de

**Yalnız tek düğümlü Proxmox hostunda**, bakım penceresinde, çalışan VM konsolu/işi yokken:

```bash
TAIL_IP=$(tailscale ip -4)
test -n "$TAIL_IP" && ip -4 addr show dev tailscale0 | grep -F "$TAIL_IP/"
sudo touch /etc/default/pveproxy
sudo cp -a /etc/default/pveproxy /etc/default/pveproxy.afudesk-oncesi
sudo sed -i '/^[[:space:]]*LISTEN_IP=/d' /etc/default/pveproxy
printf 'LISTEN_IP="%s"\n' "$TAIL_IP" | sudo tee -a /etc/default/pveproxy >/dev/null
sudo systemctl restart pveproxy.service spiceproxy.service
sudo systemctl is-active pveproxy.service spiceproxy.service
sudo ss -lntp '( sport = :8006 or sport = :3128 )'
```

8006 ve etkinse spiceproxy 3128 yalnız Tailscale adresinde olmalı. 3128 bu firewall/ACL profilinde erişime açılmaz; SPICE erişimi vaat edilmez. Tailscale yokken servis başlatılamazsa konsoldan düzeltin; wildcard'a otomatik geri dönmeyin. İstemciden `https://afu-pve:8006` açılırken sertifika zinciri ve kullanılan ad doğrulanmalıdır. Proxmox CA'sını güvenilir konsol kanalıyla alıp fingerprint'ini doğrulayın veya uygun ad için güvenilir sertifika kurun. `curl -k`, sertifika uyarısını atlama veya `verify=false` kullanmayın. Varsayılan sertifikada MagicDNS adı yoksa önce sertifika/SAN veya güvenilir ad eşleşmesini düzeltin. [Proxmox belgesi](https://github.com/proxmox/pve-docs/blob/master/pveproxy.adoc), `LISTEN_IP` için tam restart gerektiğini ve cluster sınırını açıklar.

## 5. UFW — gelen yönetim trafiğini daraltın

Yerel konsol + başarılı yeni Tailscale/Ed25519 SSH doğrulaması olmadan uygulamayın. Önce `sudo ufw status numbered`, `sudo nft list ruleset` ve `sudo iptables-save` ile mevcut kuralları inceleyin. Mevcut genel `allow 22`, `allow OpenSSH`, `allow 8006` kurallarını ilgili numaraları **büyükten küçüğe** `sudo ufw delete NUMARA` ile kaldırın; tüm firewall'ı resetlemeyin. Başka ihtiyacı olan servisi körlemesine silmeyin.

```bash
sudo sed -i 's/^IPV6=.*/IPV6=yes/' /etc/default/ufw
grep '^IPV6=' /etc/default/ufw
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow in on tailscale0 to any port 22 proto tcp comment 'Afu SSH'
# YALNIZ Proxmox hostunda; Ubuntu VM'de çalıştırmayın:
sudo ufw allow in on tailscale0 to any port 8006 proto tcp comment 'Afu Proxmox'
sudo ufw enable
sudo ufw status verbose
```

IPv6'da da varsayılan gelen politika deny olmalı; sistemi `IPV6=no` ile korumasız bırakmayın. Bu profil genel ağa Tailscale UDP portu açma kuralı eklemez. **Tailscale'in varsayılan netfilter kuralları UFW'den önce tailnet trafiğini kabul edebilir. Dolayısıyla bu UFW çıktısı tek başına tailnet içindeki tüm diğer portların engellendiği kanıtı değildir.** Asıl yönetim izolasyonu SSH/pveproxy'nin özel adrese bind edilmesidir; tailnet içindeki cihaz sınırı mevcut Tailscale ACL'siyle belirlenir. Tailscale netfilter/ACL ayarları burada değiştirilmez. [Tailscale netfilter belgesi](https://tailscale.com/docs/reference/netfilter-modes) öncelikli kabul davranışını açıklar.

Proxmox'un kendi firewall'ı veya başka firewall yöneticisi etkinse üzerine UFW'yi körlemesine etkinleştirmeyin; tek kural sahibiyle bakım planı gerekir. UFW'nin forward/bridge etkisi VM trafiğini kesebilir. Bu tarif hosta gelen trafiğindir; VM ağına `allow routed`/NAT istisnası eklemez. Host üzerinde üretimden önce bir deneme VM'sinde DHCP, DNS, apt ve yeniden başlatma bağlantısını doğrulayın. Fiziksel Proxmox firewall/bridge davranışı **UNVERIFIED**'dır.

İkinci tailnet cihazından SSH/web bağlantısını, izinsiz tailnet cihazından reddi ve tailnet dışındaki LAN cihazından 22/8006 reddini ayrı test edin. İnternet yönlendiricisinde port yönlendirmesi bulunmadığını ve IPv6 dış erişiminin kapalı olduğunu da doğrulayın. `ss -lntup` çıktısındaki her diğer wildcard servisini ayrıca inceleyin; bu belge tüm OS servislerini otomatik sertleştirmez.

## 6. Fail2ban

Depo kökünden ilgili host/VM üzerinde:

```bash
sudo apt-get install -y fail2ban
sudo install -m 0644 deploy/linux/fail2ban-sshd.local /etc/fail2ban/jail.d/afudesk-sshd.local
sudo fail2ban-client -t
sudo systemctl enable --now fail2ban
sudo systemctl restart fail2ban
sudo fail2ban-client status sshd
```

`backend=systemd` ile SSH journal'ı kullanılır; `/var/log/auth.log` varsayılmaz. Fail2ban ikinci katmandır; anahtar ve ACL'nin yerine geçmez. Tailscale/firewall zincirleri ban kuralını etkileyebilir; gerçek engelleme ayrıca doğrulanmalıdır. Deneme banlarını tek yönetim cihazınızda yapmayın.

## 7. Otomatik güvenlik güncellemeleri

**Ubuntu VM/masaüstü:** yalnız Ubuntu security pocket'ı; otomatik reboot kapalı. Ubuntu ESM/Pro depoları kullanılıyorsa bunları ayrıca değerlendirin; bu dar profil eklemez.

```bash
sudo apt-get install -y unattended-upgrades
sudo install -m 0644 deploy/linux/52-afudesk-ubuntu-security.conf /etc/apt/apt.conf.d/52-afudesk-security
sudo install -m 0644 deploy/linux/20-afudesk-auto-upgrades.conf /etc/apt/apt.conf.d/20-afudesk-auto-upgrades
sudo apt-get update
apt-config dump | grep -E 'Allowed-Origins|Origins-Pattern|Automatic-Reboot|Periodic'
sudo unattended-upgrade --dry-run --debug
sudo systemctl enable --now apt-daily.timer apt-daily-upgrade.timer
systemctl list-timers 'apt-daily*'
```

**Debian tabanlı Proxmox:** Ubuntu config'ini kopyalamayın. Aşağıdaki ayrı profil yalnız mevcut Debian sürümünün security origin'ini seçer; Proxmox depoları ve kernel/PVE paketleri otomatik güncellenmez. `#clear` yönergeleri önceki origin listelerinin birleşmesini engeller. Daha sonra sıralanan yerel drop-in'leri `apt-config dump` ile inceleyin.

```bash
sudo apt-get install -y unattended-upgrades
sudo install -m 0644 deploy/linux/52-afudesk-proxmox-security.conf /etc/apt/apt.conf.d/52-afudesk-security
sudo install -m 0644 deploy/linux/20-afudesk-auto-upgrades.conf /etc/apt/apt.conf.d/20-afudesk-auto-upgrades
sudo apt-get update
apt-config dump | grep -E 'Allowed-Origins|Origins-Pattern|Package-Blacklist|Automatic-Reboot|Periodic'
sudo unattended-upgrade --dry-run --debug
sudo systemctl enable --now apt-daily.timer apt-daily-upgrade.timer
```

Dry-run'da yalnız Debian security seçildiğini, PVE/kernel seçilmediğini doğrulamadan otomatik çalışmaya güvenmeyin. Config dosyası adı çalışan Debian codename'i varsaymaz; `${distro_codename}` makinede genişler. Proxmox major sürüm geçişi, repository/codename değişimi, kernel/hypervisor güncellemesi, reboot ve yedek geri yükleme testi **yönetici bakım penceresinde** yapılır. Debian security güncellemesinin de servis restart etkisi olabilir. Ubuntu'nun [resmî güncelleme belgesi](https://documentation.ubuntu.com/security/security-updates/) drop-in ve liste birleştirme davranışını açıklar.

## 8. En az yetkili Proxmox API token'ı

Bu hazırlık uygulamanın henüz test edilmemiş API entegrasyonunu tamamlanmış saymaz. İlk örnek yalnız **101 numaralı VM bilgisini okumak** içindir; gerçek VMID'yi seçin. Root token, `Administrator`, `/` düzeyinde geniş rol veya privilege separation kapatma yoktur. Proxmox yerel yönetici konsolunda:

```bash
pveum user add afudesk@pve --comment 'AfuDesk VM read-only'
pveum role add AfuDeskVmRead --privs 'VM.Audit'
pveum acl modify /vms/101 --users afudesk@pve --roles AfuDeskVmRead --propagate 0
# Bu komutu ekran kaydı/log/transcript olmayan güvenilir konsolda çalıştırın.
# Tek sefer görünen secret'ı doğrudan parola kasasına alın; çıktıyı rapora yapıştırmayın.
pveum user token add afudesk@pve desktop --privsep 1 --expire "$(date -d '+30 days' +%s)"
pveum acl modify /vms/101 --tokens 'afudesk@pve!desktop' --roles AfuDeskVmRead --propagate 0
pveum user token permissions afudesk@pve desktop --path /vms/101
```

Yeni hesap/rol/token adları mevcutsa önce inceleyin; mevcut yetkiyi sessizce genişletmeyin. Kullanıcı ile token yetkilerinin **kesişimi** geçerlidir; iki tarafa da aynı dar path verilir. İlerde VM açma/kapatma gerekirse yalnız ilgili VM path'inde `VM.PowerMgmt` ayrı değerlendirilir; console, disk yazma, `Sys.Modify` ve `Permissions.Modify` eklenmez. Her API metodunun gereksinimi [resmî API görüntüleyicisinden](https://pve.proxmox.com/pve-docs/api-viewer/) kontrol edilir. [Proxmox token belgesi](https://github.com/proxmox/pve-docs/blob/master/pveum.adoc) privilege separation ve iptal mekanizmasını açıklar.

İstemcide token'ı komut argümanı, shell history, URL veya debug loga koymadan, güvenilir CA ve sertifikadaki **aynı DNS adıyla** okumayı deneyin. Önce CA dosyası ve sunucu adı hazırlanmış olmalıdır. Aşağıdaki secret girişi ekrana yazılmaz; curl başlığı izin 600 geçici dosyadan okur:

```bash
set +x
umask 077
TOKEN_HEADER=$(mktemp)
trap 'rm -f -- "$TOKEN_HEADER"' EXIT
read -r -s -p 'Proxmox token secret: ' PVE_SECRET
printf '\n'
printf 'Authorization: PVEAPIToken=afudesk@pve!desktop=%s\n' "$PVE_SECRET" > "$TOKEN_HEADER"
unset PVE_SECRET
# DNS adı sertifikadaki SAN ile eşleşmeli; yolu ve node adını kendi kurulumunuza göre seçin.
curl --fail --silent --show-error --cacert "$HOME/.config/afudesk/pve-ca.pem" \
  --header "@$TOKEN_HEADER" \
  'https://afu-pve:8006/api2/json/nodes/afu-pve/qemu/101/status/current'
rm -f -- "$TOKEN_HEADER"
trap - EXIT
```

`afu-pve` Proxmox node adı farklıysa URL'deki node segmentini değiştirin; TLS hostname doğrulamasını kapatmayın. Token kalıcı olarak Secret Service/parola kasasında tutulmalıdır; geçici dosya örneği yalnız kısa test içindir. Başka VM path'inin 403 döndürmesini de deneyin. İptal: `pveum user token remove afudesk@pve desktop`. Süre/rotation ve iptal ayrı cihaz başına yönetilir; bu token ömrü Google OAuth veya AfuDesk 30 günlük cihaz güveniyle aynı şey değildir.

## 9. Tailscale ACL örneği — uygulanmaz

[Dar kapsamlı JSON örneği](../deploy/linux/tailscale-acl.example.json), yalnız belirtilen yönetici hesabından host 22/8006 ve VM 22 erişimini gösterir. Adres adı yerine server tag kullanır; `yonetici@example.com` gerçek yöneticiyle değiştirilmelidir. Mevcut tailnet ACL'sini bu dosyayla değiştirmeyin veya otomatik birleştirmeyin. `acls`/`grants` kuralları birikimlidir; mevcut geniş allow kuralı dar örneğin kısıtını geçersiz kılabilir. Bu örnek **sözdizimi açısından JSON**, Tailscale policy motorunda ise **UNVERIFIED**'dır; yönetici kendi mevcut policy'siyle preview/test yapmadan etkinleştirmez. Yönetici olmayan ayrı cihaz için deny testi de gerçek tailnet kimliğiyle eklenmelidir. [Resmî policy söz dizimi](https://tailscale.com/docs/reference/syntax/policy-file) etiket ve erişim/test alanlarını açıklar.

Bu örnek uygulamanın dinamik UDP uzak masaüstü akışına erişim vermez; yalnız sunucu yönetimini kapsar. Uygulama erişimi için ayrı dar cihaz politikası gerekir. Mevcut erişim politikası bu görevde değiştirilmemiştir.

## 10. Kanıt ve konsoldan kurtarma

| Kontrol | Beklenen | Mevcut durum |
|---|---|---|
| `sshd -t`, `sshd -T`, `ss -lntp` | Yalnız tail IP; parola/root yok | UNVERIFIED — Linux sunucusu yok |
| Yeni SSH/sudo + Proxmox TLS | Anahtar ve doğru sertifika ile çalışır | UNVERIFIED |
| LAN ve yetkisiz tailnet cihazı | SSH/8006 reddedilir | UNVERIFIED |
| `ufw status`, IPv6 dış erişim, VM ağı | İstenen izolasyon, çalışan bridge | UNVERIFIED |
| `fail2ban-client -t/status`, ban testi | Geçerli config ve gerçek engelleme | UNVERIFIED |
| `apt-config`, unattended dry-run | Yalnız uygun security origin'leri | UNVERIFIED |
| Token dar path, başka VM 403, revoke | En az yetki/iptal | UNVERIFIED |
| Yeniden başlatma | Tailscale sonrası yönetim geri gelir | UNVERIFIED |

Hata halinde önce yerel konsolda `tailscale status`, `ip addr show tailscale0`, `journalctl -u ssh -u pveproxy -b` inceleyin. Tailscale hazırsa servisi yeniden başlatın. Hatalı SSH drop-in'ini yerel konsolda yedekten düzeltip `sshd -t` sonrası restart edin; pveproxy dosyasını kendi yedeğinden geri alın. Kurtarma sırasında ilk kurulumdaki izole ağ koşulunu koruyun; parola SSH, genel bind veya genel firewall izni açmayın. UFW kuralını yerel konsoldan tek tek düzeltin; `ufw reset`/tüm firewall kapatma çözüm değildir. Çalışan kurulumun reboot, VM ağı ve yeni bağlantı kanıtları kaydedilene kadar uzak erişim tamamlanmış sayılmaz.
