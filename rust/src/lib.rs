//! AfuDesk çekirdeği. Bağlantı veren cihaz kimliğini ve adreslerini parolayla şifreli tek bir
//! koda koyar; karşı taraf kodla bağlanır. Taşıma iroh: önce doğrudan (LAN/IPv6/UPnP),
//! sonra NAT delme, olmazsa relay yedeği — kullanıcı IP/port/modem ayarı yapmaz.
pub mod ag;
pub mod dosya;
pub mod google;
pub mod google_drive;
pub mod google_baglanti;
pub mod google_depo;
pub mod google_dizin;
pub mod google_oauth;
pub mod google_pin;
pub mod goruntu;
pub mod guvenilen;
pub mod host;
pub mod izleyici;
pub mod kimlik;
pub mod kod;
pub mod kol;
pub mod pano;
pub mod platform;
pub mod terminal;
pub mod protokol;
pub mod sanal_kol;
pub mod zaman;

#[cfg(test)]
mod uctan_uca;
