use crate::protokol::KolDurumu;
use anyhow::Result;

pub trait KolSurucusu: Send {
    fn guncelle(&mut self, d: &KolDurumu) -> Result<()>;
    fn titresim_al(&mut self) -> Vec<(u8, u8, u8)>;
    fn kaldir(&mut self, slot: u8);
}

pub const SURUCU_UYARISI: &str = "Oyun kolu için ViGEmBus sürücüsü gerekli: https://github.com/nefarius/ViGEmBus/releases";

#[cfg(windows)]
pub struct ViGEmSurucusu { kollar: std::collections::HashMap<u8, vigem_client::Xbox360Wired<vigem_client::Client>> }
#[cfg(windows)]
impl ViGEmSurucusu {
    pub fn yeni() -> Result<Self> { vigem_client::Client::connect().map_err(|_| anyhow::anyhow!(SURUCU_UYARISI))?; Ok(Self { kollar: Default::default() }) }
}
#[cfg(windows)]
impl KolSurucusu for ViGEmSurucusu {
    fn guncelle(&mut self, d: &KolDurumu) -> Result<()> {
        use vigem_client::{XButtons, XGamepad};
        if !self.kollar.contains_key(&d.slot) { let client = vigem_client::Client::connect().map_err(|_| anyhow::anyhow!(SURUCU_UYARISI))?; let mut k = vigem_client::Xbox360Wired::new(client, vigem_client::TargetId::XBOX360_WIRED); k.plugin().map_err(|_| anyhow::anyhow!(SURUCU_UYARISI))?; k.wait_ready().map_err(|_| anyhow::anyhow!(SURUCU_UYARISI))?; self.kollar.insert(d.slot, k); }
        let k = self.kollar.get_mut(&d.slot).unwrap();
        k.update(&XGamepad { buttons: XButtons(d.dugmeler), left_trigger: d.sol_tetik, right_trigger: d.sag_tetik, thumb_lx: d.sol_x, thumb_ly: d.sol_y, thumb_rx: d.sag_x, thumb_ry: d.sag_y }).map_err(Into::into)
    }
    fn titresim_al(&mut self) -> Vec<(u8,u8,u8)> { Vec::new() }
    fn kaldir(&mut self, slot:u8) { self.kollar.remove(&slot); }
}

#[cfg(not(windows))]
pub struct ViGEmSurucusu;
#[cfg(not(windows))]
impl ViGEmSurucusu { pub fn yeni() -> Result<Self> { anyhow::bail!(SURUCU_UYARISI) } }

#[derive(Default)]
pub struct SahteKolSurucusu { pub durumlar: std::sync::Arc<std::sync::Mutex<Vec<KolDurumu>>>, titresimler: Vec<(u8,u8,u8)> }
impl SahteKolSurucusu { pub fn titresim_ekle(&mut self, slot:u8, buyuk:u8, kucuk:u8) { self.titresimler.push((slot,buyuk,kucuk)); } }
impl SahteKolSurucusu { pub fn yeni(durumlar: std::sync::Arc<std::sync::Mutex<Vec<KolDurumu>>>) -> Self { Self { durumlar, titresimler: Vec::new() } } }
impl KolSurucusu for SahteKolSurucusu {
    fn guncelle(&mut self,d:&KolDurumu)->Result<()> { self.durumlar.lock().unwrap().push(d.clone()); Ok(()) }
    fn titresim_al(&mut self)->Vec<(u8,u8,u8)> { std::mem::take(&mut self.titresimler) }
    fn kaldir(&mut self,_:u8) {}
}

#[cfg(windows)]
impl Drop for ViGEmSurucusu { fn drop(&mut self) { self.kollar.clear(); } }

#[cfg(test)] mod testler {
 use super::*;
 #[test] fn sahte_surucu_durumlari_kaydeder() { let mut s=SahteKolSurucusu::default(); let d=KolDurumu{slot:0,sira:1,dugmeler:3,sol_x:1,sol_y:2,sag_x:3,sag_y:4,sol_tetik:5,sag_tetik:6}; s.guncelle(&d).unwrap(); assert_eq!(s.durumlar.lock().unwrap()[0],d); }
 #[cfg(windows)] #[test] fn gercek_vigem_kol_olusur() { let mut s=ViGEmSurucusu::yeni().unwrap(); let d=KolDurumu{slot:0,sira:1,dugmeler:0,sol_x:0,sol_y:0,sag_x:0,sag_y:0,sol_tetik:0,sag_tetik:0}; s.guncelle(&d).unwrap(); s.kaldir(0); }
}
