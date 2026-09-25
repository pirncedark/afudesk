//! AfuDesk çekirdeği. Merkezi sunucu yok: bağlantı veren cihaz kendi QUIC sunucusunu açar,
//! adreslerini parolayla şifreli tek bir koda koyar; karşı taraf kodla doğrudan bağlanır.
pub mod adres;
pub mod ag;
pub mod dosya;
pub mod goruntu;
pub mod guvenilen;
pub mod host;
pub mod izleyici;
pub mod kimlik;
pub mod kod;
pub mod kol;
pub mod pano;
pub mod platform;
pub mod protokol;
pub mod sanal_kol;
pub mod zaman;

#[cfg(test)]
mod uctan_uca;
