//! AfuDesk çekirdeği. Merkezi sunucu yok: bağlantı veren cihaz kendi QUIC sunucusunu açar,
//! adreslerini parolayla şifreli tek bir koda koyar; karşı taraf kodla doğrudan bağlanır.
pub mod adres;
pub mod ag;
pub mod goruntu;
pub mod host;
pub mod izleyici;
pub mod kod;
pub mod platform;
pub mod protokol;

#[cfg(test)]
mod uctan_uca;
