# Görev: Ubuntu ve Proxmox kurulum/güvenlik belgeleri
ONAY: Kapsam kullanıcı tarafından onaylı; doğrudan uygula.
Çalışma dizini C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux. AGENTS.md oku.
Sahiplik: docs/UBUNTU_KURULUM.md, docs/LINUX_SUNUCU.md, docs/SONUC_LINUX_BELGELER.md, deploy/linux/** (yeni güvenlik config/örnekler). Başka ajanlar aynı kod tabanında; değişikliklerini geri alma.
YASAK: yıkıcı git, commit/push, gerçek sunucuya bağlantı, Tailscale ACL uygulama, diğer dizinlere yazma, görünür pencere.

📍 C:\Users\afuuu\Desktop\afuproject\_gorev\2026-11-11\AFUDESK_LINUX.md:35 EK / :43 EK2 — dosyanın tamamını UTF8 oku; sunucu henüz yok. Ubuntu Server 24.04 VM Proxmox fiziksel host + Ubuntu Desktop istemci mimarisi. .deb kurulumu, X11 hazır yol ve Wayland portal/PipeWire eksiklerinin dürüst açıklaması.
📍 C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux\rust\src\platform.rs:186 XcapYakalayici — mevcut X11 yakalama/enigo input; portal oturum onayı yok. Tam Wayland destekli diye yazma.
📍 C:\Users\afuuu\Desktop\afuproject\_wt\afudesk-linux\rust\src\ag.rs:86 uc_nokta_kimlikli — ana ajan Linux ağ güvenliğini yazıyor: Tailscale varsa arayüze bind, yoksa loopback (güvenli kapalı). Relay n0 yalnız yedek, kimlik yayını yok, afudesk/8 korunur. Belgede kullanıcıyı genel IP açmaya veya UPnP'ye yönlendirme.

İstenen: kopyala-çalıştır yönetici komutları/configler: SSH Ed25519 anahtar, PasswordAuthentication no, KbdInteractiveAuthentication no, PermitRootLogin no, Tailscale IP'de ListenAddress (Ubuntu ssh.socket aktivasyonunu unutma); UFW default deny incoming allow sadece tailscale0 SSH/8006, IPv6 korunmalı; Proxmox pveproxy LISTEN_IP ve restart doğrulaması; fail2ban; unattended-upgrades Ubuntu vs Debian/Proxmox ayrımı (Proxmox major/kernel otomatik yükseltme yapma); en az yetkili Proxmox API token (root değil, privilege separation, sertifika kontrolü, secret loglama yok); örnek Tailscale ACL (mevcut ACL'yi değiştirme, örnek dar kapsamlı) ve sabit cihaz adları. Tailscale yoksa resmi apt repo kurulum yönergesi, curl|sh yerine imzalı repo. Firewall uygulaması kilitleyebileceği için mevcut yerel konsol/başarılı Tailscale SSH doğrulama önkoşulu.
Hiçbir komutu gerçek sunucuda çalıştırma. Kanıt, config söz dizimi kontrolleri ve UNVERIFIED donanım adımlarını ayrı yaz. GitHub PR açık mevcut: #17, #7; yeni PR yok.
