//! AfuDesk çekirdeği. Bağlantı veren cihaz kimliğini ve adreslerini parolayla şifreli tek bir
//! koda koyar; karşı taraf kodla bağlanır. Taşıma iroh: önce doğrudan (LAN/IPv6/UPnP),
//! sonra NAT delme, olmazsa relay yedeği — kullanıcı IP/port/modem ayarı yapmaz.
pub mod ag;
pub mod dosya;
pub mod goruntu;
pub mod host;
pub mod izleyici;
pub mod kod;
pub mod kol;
pub mod pano;
pub mod platform;
pub mod protokol;
pub mod sanal_kol;
pub mod zaman;

#[cfg(test)]
mod uctan_uca;
