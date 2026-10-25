//! Telefon PCM16/16 kHz/mono sesini VB-CABLE'a taşır; Windows işlemleri işçi üzerindedir.

use crate::protokol::Girdi;
use anyhow::Result;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub const SURUCU_UYARISI: &str =
    "Telefon mikrofonu için VB-CABLE sürücüsü gerekli, kurmak için aç: https://vb-audio.com/Cable/";
const SES_UYARISI: &str =
    "Telefon mikrofonu açılamadı, bilgisayardaki ses aygıtlarını kontrol edip yeniden dene.";

pub fn kullanici_uyarisi(hata: &anyhow::Error) -> &'static str {
    if hata.to_string().contains("VB-CABLE") {
        SURUCU_UYARISI
    } else {
        SES_UYARISI
    }
}

/// COM/WASAPI yalnız bu arayüzün gerçek uygulamasında bulunur.
pub trait SesApi {
    fn cable(&mut self) -> Result<(String, String)>;
    fn varsayilan(&mut self, rol: u32) -> Result<String>;
    fn sec(&mut self, rol: u32, id: &str) -> Result<()>;
    fn baslat(&mut self, id: &str) -> Result<()>;
    /// Tüketilen örnek sayısı; dolu WASAPI tamponundaki ses kaybolmaz.
    fn yaz(&mut self, pcm: &[i16]) -> Result<usize>;
    fn durdur(&mut self);
}

fn cable_adi(ad: &str, kayit: bool) -> bool {
    let ad = ad
        .to_lowercase()
        .replace('ı', "i")
        .replace('ş', "s")
        .replace('ç', "c");
    ad.contains("vb-audio virtual cable")
        && if kayit {
            ad.contains("cable output") || ad.contains("cable cikisi")
        } else {
            ad.contains("cable input") || ad.contains("cable girisi")
        }
}

struct Oturum<A: SesApi> {
    api: A,
    eski: Vec<(u32, String)>,
    acik: bool,
    kapanis: Option<Instant>,
    tampon: VecDeque<i16>,
}
impl<A: SesApi> Oturum<A> {
    fn yeni(api: A) -> Self {
        Self {
            api,
            eski: vec![],
            acik: false,
            kapanis: None,
            tampon: VecDeque::new(),
        }
    }
    fn ac(&mut self) -> Result<()> {
        self.kapanis = None;
        if self.acik {
            return Ok(());
        }
        self.geri_yukle()?;
        let (kayit, cal) = self.api.cable()?;
        // İki rol de değiştirilmeden ÖNCE saklanır; ikinci rol hata verirse geri alınır.
        let eski = vec![(0, self.api.varsayilan(0)?), (2, self.api.varsayilan(2)?)];
        self.api.baslat(&cal)?;
        self.eski = eski;
        for rol in [0, 2] {
            if let Err(e) = self.api.sec(rol, &kayit) {
                let _ = self.geri_yukle();
                return Err(e);
            }
        }
        self.acik = true;
        Ok(())
    }
    fn ses(&mut self, b: &[u8]) -> Result<()> {
        anyhow::ensure!(b.len() == 640, "Ses parçası 640 bayt olmalı.");
        if !self.acik || self.kapanis.is_some() {
            return Ok(());
        }
        self.tampon
            .extend(b.chunks_exact(2).map(|x| i16::from_le_bytes([x[0], x[1]])));
        // En çok 60 ms: ağ yığılması eski sesi biriktirmez.
        while self.tampon.len() > 960 {
            self.tampon.pop_front();
        }
        Ok(())
    }
    fn kapat(&mut self, simdi: Instant) {
        if self.acik && self.kapanis.is_none() {
            self.kapanis = Some(simdi + Duration::from_millis(300));
        }
    }
    fn tick(&mut self, simdi: Instant) -> Result<()> {
        if self.kapanis.is_some_and(|t| simdi >= t) {
            return self.geri_yukle();
        }
        if self.acik {
            // 10 ms başına 160 örnek; eksik paket sessizlikle doldurulur.
            let mut pcm = [0i16; 160];
            for (x, ses) in pcm.iter_mut().zip(self.tampon.iter()) {
                *x = *ses;
            }
            let n = self.api.yaz(&pcm)?.min(self.tampon.len()).min(pcm.len());
            self.tampon.drain(..n);
        }
        Ok(())
    }
    fn geri_yukle(&mut self) -> Result<()> {
        self.acik = false;
        self.kapanis = None;
        self.tampon.clear();
        self.api.durdur();
        let mut kalan = vec![];
        for (rol, id) in self.eski.drain(..) {
            if self.api.sec(rol, &id).is_err() {
                kalan.push((rol, id));
            }
        }
        self.eski = kalan;
        anyhow::ensure!(self.eski.is_empty(), "Önceki mikrofon geri yüklenemedi.");
        Ok(())
    }
}
impl<A: SesApi> Drop for Oturum<A> {
    fn drop(&mut self) {
        // Geçici aygıt/COM hatasında her iki rol için de yeniden dene.
        for _ in 0..3 {
            if self.geri_yukle().is_ok() {
                return;
            }
        }
        log::error!("Önceki mikrofon geri yüklenemedi");
    }
}

enum Komut {
    Ac(std::sync::mpsc::SyncSender<Result<()>>),
    Ses(Vec<u8>),
    Kapat,
}

/// Enjektörle birlikte düşer: kopuşta/kapanışta işçi durur ve aygıtlar geri yüklenir.
#[derive(Default)]
pub struct Mikrofon {
    isci: Option<(
        std::sync::mpsc::SyncSender<Komut>,
        std::thread::JoinHandle<()>,
    )>,
    hata: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}
impl Mikrofon {
    fn durdur(&mut self) {
        if let Some((tx, isci)) = self.isci.take() {
            drop(tx);
            let _ = isci.join();
        }
    }
    pub fn uygula(&mut self, g: &Girdi) -> Result<()> {
        if let Some(hata) = self.hata.lock().unwrap().take() {
            anyhow::bail!(hata);
        }
        match g {
            Girdi::SesAc => {
                #[cfg(not(windows))]
                anyhow::bail!(SURUCU_UYARISI);
                #[cfg(windows)]
                {
                    if self.isci.is_none() {
                        let (tx, rx) = std::sync::mpsc::sync_channel(3);
                        let hata = self.hata.clone();
                        let isci = std::thread::Builder::new()
                            .name("telefon-mikrofonu".into())
                            .spawn(move || {
                                isci(rx, hata);
                            })?;
                        self.isci = Some((tx, isci));
                    }
                    let (tx, rx) = std::sync::mpsc::sync_channel(1);
                    self.isci.as_ref().unwrap().0.send(Komut::Ac(tx))?;
                    let sonuc = rx
                        .recv()
                        .map_err(|_| anyhow::anyhow!(SES_UYARISI))
                        .and_then(|s| s);
                    if sonuc.is_err() {
                        self.durdur();
                    }
                    sonuc?;
                }
            }
            Girdi::Ses(b) => {
                anyhow::ensure!(b.len() == 640, "Ses parçası 640 bayt olmalı.");
                if let Some((tx, _)) = &self.isci {
                    match tx.try_send(Komut::Ses(b.clone())) {
                        Ok(()) | Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                        Err(_) => anyhow::bail!(SES_UYARISI),
                    }
                }
            }
            Girdi::SesKapat => {
                if let Some((tx, _)) = &self.isci {
                    tx.send(Komut::Kapat)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
impl Drop for Mikrofon {
    fn drop(&mut self) {
        self.durdur();
    }
}

#[cfg(windows)]
fn isci(
    rx: std::sync::mpsc::Receiver<Komut>,
    hata: std::sync::Arc<std::sync::Mutex<Option<String>>>,
) {
    let api = match win::WindowsSes::yeni() {
        Ok(a) => a,
        Err(_) => {
            if let Ok(Komut::Ac(tx)) = rx.recv() {
                let _ = tx.send(Err(anyhow::anyhow!(SES_UYARISI)));
            }
            return;
        }
    };
    let mut s = Oturum::yeni(api);
    let mut tick = Instant::now() + Duration::from_millis(10);
    loop {
        match rx.recv_timeout(tick.saturating_duration_since(Instant::now())) {
            Ok(Komut::Ac(tx)) => {
                let _ = tx.send(s.ac());
            }
            Ok(Komut::Ses(b)) => {
                let _ = s.ses(&b);
            }
            Ok(Komut::Kapat) => s.kapat(Instant::now()),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        let simdi = Instant::now();
        if simdi >= tick {
            if let Err(e) = s.tick(simdi) {
                log::warn!("Telefon mikrofonu: {e}");
                let _ = s.geri_yukle();
                *hata.lock().unwrap() = Some(SES_UYARISI.into());
            }
            tick += Duration::from_millis(10);
            if tick <= simdi {
                tick = simdi + Duration::from_millis(10);
            }
        }
    }
}

/// Ham COM ABI: SDK'daki IMMDeviceEnumerator / IAudioClient / IAudioRenderClient.
/// Tüm tutamaçlar yalnız oluşturuldukları COM işçisinde kullanılıp bırakılır.
#[cfg(windows)]
mod win {
    use super::*;
    use std::{
        ffi::c_void,
        ptr::{null, null_mut},
    };
    type Ptr = *mut c_void;
    #[repr(C)]
    struct Guid {
        a: u32,
        b: u16,
        c: u16,
        d: [u8; 8],
    }
    const fn guid(n: u128) -> Guid {
        Guid {
            a: (n >> 96) as u32,
            b: (n >> 80) as u16,
            c: (n >> 64) as u16,
            d: (n as u64).to_be_bytes(),
        }
    }
    const ENUM_CLS: Guid = guid(0xbcde0395_e52f_467c_8e3d_c4579291692e);
    const ENUM_IID: Guid = guid(0xa95664d2_9614_4f35_a746_de8db63617e6);
    const POLICY_CLS: Guid = guid(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
    const POLICY_IID: Guid = guid(0xf8679f50_850a_41cf_9c72_430f290290c8);
    const CLIENT_IID: Guid = guid(0x1cb9ad4c_dbfa_4c32_b178_c2f568a703b2);
    const RENDER_IID: Guid = guid(0xf294acfc_3146_4483_a7bf_addca7c260e2);
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(p: Ptr, flags: u32) -> i32;
        fn CoUninitialize();
        fn CoCreateInstance(
            cls: *const Guid,
            outer: Ptr,
            context: u32,
            iid: *const Guid,
            out: *mut Ptr,
        ) -> i32;
        fn CoTaskMemFree(p: Ptr);
        fn PropVariantClear(p: *mut Prop) -> i32;
    }
    fn hr(n: i32) -> Result<()> {
        anyhow::ensure!(n >= 0, "Windows ses hatası: {n:#x}");
        Ok(())
    }
    struct Com(Ptr);
    // Safety: slot ve imza her çağrıda Windows SDK vtable sırasıyla eşleşir.
    // Com, Send/Sync değildir; gerçek iş parçacığının dışına çıkarılmaz.
    macro_rules! call {
        ($o:expr, $slot:expr, ($($t:ty),*) -> $r:ty $(, $a:expr)* $(,)?) => {{
            let p = $o.0;
            let f: unsafe extern "system" fn(Ptr, $($t),*) -> $r =
                std::mem::transmute(*(*(p as *const *const Ptr)).add($slot));
            f(p, $($a),*)
        }};
    }
    impl Com {
        fn yeni(cls: &Guid, iid: &Guid) -> Result<Self> {
            let mut p = null_mut();
            unsafe {
                hr(CoCreateInstance(cls, null_mut(), 1, iid, &mut p))?;
            }
            anyhow::ensure!(!p.is_null(), "Boş COM arayüzü");
            Ok(Self(p))
        }
        fn id(&self) -> Result<String> {
            let mut p: *mut u16 = null_mut();
            unsafe {
                hr(call!(self, 5, (*mut *mut u16) -> i32, &mut p))?;
                let id = metin(p);
                CoTaskMemFree(p.cast());
                Ok(id)
            }
        }
        fn ad(&self) -> Result<String> {
            let mut p = null_mut();
            unsafe {
                hr(call!(self, 4, (u32, *mut Ptr) -> i32, 0, &mut p))?;
                let store = Com(p);
                let key = Key {
                    guid: guid(0xa45c254e_df1c_4efd_8020_67d146a850e0),
                    pid: 14,
                };
                let mut prop: Prop = std::mem::zeroed();
                let sonuc = call!(store, 5, (*const Key, *mut Prop) -> i32, &key, &mut prop);
                let ad = if sonuc >= 0 && prop.vt == 31 {
                    metin(prop.data[0] as *const u16)
                } else {
                    String::new()
                };
                PropVariantClear(&mut prop);
                hr(sonuc)?;
                Ok(ad)
            }
        }
    }
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                call!(self, 2, () -> u32);
            }
        }
    }
    unsafe fn metin(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    #[repr(C)]
    struct Key {
        guid: Guid,
        pid: u32,
    }
    #[repr(C)]
    struct Prop {
        vt: u16,
        reserved: [u16; 3],
        data: [usize; 2],
    }
    #[repr(C, packed(1))]
    struct Format {
        tag: u16,
        kanallar: u16,
        hiz: u32,
        bayt_hiz: u32,
        blok: u16,
        bit: u16,
        ek: u16,
    }
    struct Apartment;
    impl Apartment {
        fn yeni() -> Result<Self> {
            unsafe {
                hr(CoInitializeEx(null_mut(), 0))?;
            }
            Ok(Self)
        }
    }
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    struct Akis {
        render: Com,
        client: Com,
        boy: u32,
    }
    impl Drop for Akis {
        fn drop(&mut self) {
            unsafe {
                call!(self.client, 11, () -> i32);
            }
        }
    }
    pub(super) struct WindowsSes {
        enumerator: Com,
        policy: Com,
        akis: Option<Akis>,
        // Alanlar sırayla düşer; COM bütün arayüzler bırakıldıktan sonra kapanır.
        _apartment: Apartment,
    }
    impl WindowsSes {
        pub fn yeni() -> Result<Self> {
            let apartment = Apartment::yeni()?;
            let enumerator = Com::yeni(&ENUM_CLS, &ENUM_IID)?;
            let policy = Com::yeni(&POLICY_CLS, &POLICY_IID)?;
            Ok(Self {
                enumerator,
                policy,
                akis: None,
                _apartment: apartment,
            })
        }
        fn bul(&self, flow: u32) -> Result<String> {
            unsafe {
                let mut p = null_mut();
                hr(call!(self.enumerator, 3, (u32, u32, *mut Ptr) -> i32, flow, 1, &mut p))?;
                let koleksiyon = Com(p);
                let mut n = 0;
                hr(call!(koleksiyon, 3, (*mut u32) -> i32, &mut n))?;
                for i in 0..n {
                    let mut p = null_mut();
                    hr(call!(koleksiyon, 4, (u32, *mut Ptr) -> i32, i, &mut p))?;
                    let aygit = Com(p);
                    if cable_adi(&aygit.ad()?, flow == 1) {
                        return aygit.id();
                    }
                }
            }
            anyhow::bail!(SURUCU_UYARISI)
        }
    }
    impl SesApi for WindowsSes {
        fn cable(&mut self) -> Result<(String, String)> {
            Ok((self.bul(1)?, self.bul(0)?))
        }
        fn varsayilan(&mut self, rol: u32) -> Result<String> {
            let mut p = null_mut();
            unsafe {
                hr(call!(self.enumerator, 4, (u32, u32, *mut Ptr) -> i32, 1, rol, &mut p))?;
            }
            Com(p).id()
        }
        fn sec(&mut self, rol: u32, id: &str) -> Result<()> {
            let id = wide(id);
            unsafe { hr(call!(self.policy, 13, (*const u16, u32) -> i32, id.as_ptr(), rol)) }
        }
        fn baslat(&mut self, id: &str) -> Result<()> {
            self.durdur();
            let id = wide(id);
            unsafe {
                let mut p = null_mut();
                hr(call!(self.enumerator, 5, (*const u16, *mut Ptr) -> i32, id.as_ptr(), &mut p))?;
                let aygit = Com(p);
                let mut p = null_mut();
                hr(
                    call!(aygit, 3, (*const Guid, u32, Ptr, *mut Ptr) -> i32, &CLIENT_IID, 1, null_mut(), &mut p),
                )?;
                let client = Com(p);
                let format = Format {
                    tag: 1,
                    kanallar: 1,
                    hiz: 16000,
                    bayt_hiz: 32000,
                    blok: 2,
                    bit: 16,
                    ek: 0,
                };
                // Shared mode: Windows motoru aygıtın karışım hızına/kanalına dönüştürür.
                hr(
                    call!(client, 3, (u32, u32, i64, i64, *const Format, *const Guid) -> i32,
                    0, 0x88000000, 600000, 0, &format, null()),
                )?;
                let mut boy = 0;
                hr(call!(client, 4, (*mut u32) -> i32, &mut boy))?;
                let mut p = null_mut();
                hr(call!(client, 14, (*const Guid, *mut Ptr) -> i32, &RENDER_IID, &mut p))?;
                let render = Com(p);
                hr(call!(client, 10, () -> i32))?;
                self.akis = Some(Akis {
                    render,
                    client,
                    boy,
                });
            }
            Ok(())
        }
        fn yaz(&mut self, pcm: &[i16]) -> Result<usize> {
            let s = self
                .akis
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!(SES_UYARISI))?;
            unsafe {
                let mut padding = 0;
                hr(call!(s.client, 6, (*mut u32) -> i32, &mut padding))?;
                // Donanım henüz tüketmediyse bu tick bekler; WASAPI'ye taşma olmaz.
                let n = (pcm.len() as u32).min(s.boy.saturating_sub(padding));
                if n == 0 {
                    return Ok(0);
                }
                let mut p: *mut u8 = null_mut();
                hr(call!(s.render, 3, (u32, *mut *mut u8) -> i32, n, &mut p))?;
                std::ptr::copy_nonoverlapping(pcm.as_ptr().cast::<u8>(), p, n as usize * 2);
                hr(call!(s.render, 4, (u32, u32) -> i32, n, 0))?;
                Ok(n as usize)
            }
        }
        fn durdur(&mut self) {
            self.akis = None;
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Durum {
        secimler: Vec<(u32, String)>,
        yok: bool,
        hata: bool,
        geri_hatasi: bool,
        yazilan: Vec<i16>,
    }
    struct Sahte(Arc<Mutex<Durum>>);
    impl SesApi for Sahte {
        fn cable(&mut self) -> anyhow::Result<(String, String)> {
            anyhow::ensure!(!self.0.lock().unwrap().yok, SURUCU_UYARISI);
            Ok(("cable-out".into(), "cable-in".into()))
        }
        fn varsayilan(&mut self, rol: u32) -> anyhow::Result<String> {
            Ok(format!("eski-{rol}"))
        }
        fn sec(&mut self, rol: u32, id: &str) -> anyhow::Result<()> {
            let mut d = self.0.lock().unwrap();
            if d.hata && rol == 2 && id == "cable-out" {
                anyhow::bail!("hata");
            }
            if d.geri_hatasi && rol == 0 && id == "eski-0" {
                d.geri_hatasi = false;
                anyhow::bail!("geçici hata");
            }
            d.secimler.push((rol, id.into()));
            Ok(())
        }
        fn baslat(&mut self, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn yaz(&mut self, pcm: &[i16]) -> anyhow::Result<usize> {
            self.0.lock().unwrap().yazilan.extend_from_slice(pcm);
            Ok(pcm.len())
        }
        fn durdur(&mut self) {}
    }
    #[test]
    fn aygit_adlari() {
        assert!(cable_adi("CABLE Output (VB-Audio Virtual Cable)", true));
        assert!(cable_adi("CABLE Çıkışı (VB-Audio Virtual Cable)", true));
        assert!(cable_adi("CABLE Girişi (VB-Audio Virtual Cable)", false));
        assert!(!cable_adi("CABLE-A Output (VB-Audio Cable A)", true));
        assert!(!cable_adi("Microphone", true));
    }
    #[test]
    fn kapatma_300ms_ve_drop_geri_yukler() {
        let d = Arc::new(Mutex::new(Durum::default()));
        let mut s = Oturum::yeni(Sahte(d.clone()));
        let t = std::time::Instant::now();
        s.ac().unwrap();
        s.kapat(t);
        s.tick(t + std::time::Duration::from_millis(299)).unwrap();
        assert_eq!(d.lock().unwrap().secimler.len(), 2);
        s.tick(t + std::time::Duration::from_millis(300)).unwrap();
        assert_eq!(
            d.lock().unwrap().secimler[2..],
            [(0, "eski-0".into()), (2, "eski-2".into())]
        );
        s.ac().unwrap();
        s.ac().unwrap();
        drop(s);
        assert_eq!(d.lock().unwrap().secimler.len(), 8);
    }
    #[test]
    fn surucu_yok_ve_yarim_degisim() {
        let d = Arc::new(Mutex::new(Durum {
            yok: true,
            ..Default::default()
        }));
        let mut s = Oturum::yeni(Sahte(d.clone()));
        assert!(s.ac().unwrap_err().to_string().contains("VB-CABLE"));
        assert!(d.lock().unwrap().secimler.is_empty());
        {
            let mut x = d.lock().unwrap();
            x.yok = false;
            x.hata = true;
        }
        assert!(s.ac().is_err());
        assert!(d.lock().unwrap().secimler.contains(&(0, "eski-0".into())));
    }
    #[test]
    fn pcm_sinirli_ve_kapaliyken_atilir() {
        let mut s = Oturum::yeni(Sahte(Arc::new(Mutex::new(Durum::default()))));
        assert!(s.ses(&[0; 640]).is_ok());
        s.ac().unwrap();
        assert!(s.ses(&[0; 639]).is_err());
        assert!(s.ses(&[0; 642]).is_err());
        for _ in 0..10 {
            s.ses(&[1; 640]).unwrap();
        }
        assert!(s.tampon.len() <= 960);
    }
    #[test]
    fn hizli_yeniden_acma_eski_aygitlari_korur() {
        let d = Arc::new(Mutex::new(Durum::default()));
        let mut s = Oturum::yeni(Sahte(d.clone()));
        let t = Instant::now();
        s.ac().unwrap();
        s.kapat(t);
        s.ac().unwrap();
        s.tick(t + Duration::from_secs(1)).unwrap();
        assert_eq!(d.lock().unwrap().secimler.len(), 2);
        drop(s);
        assert_eq!(d.lock().unwrap().secimler.len(), 4);
    }
    #[test]
    fn geri_yukleme_hatasinda_drop_yeniden_dener() {
        let d = Arc::new(Mutex::new(Durum::default()));
        let mut s = Oturum::yeni(Sahte(d.clone()));
        s.ac().unwrap();
        d.lock().unwrap().geri_hatasi = true;
        let t = Instant::now();
        s.kapat(t);
        assert!(s.tick(t + Duration::from_millis(300)).is_err());
        drop(s);
        assert!(d.lock().unwrap().secimler.contains(&(0, "eski-0".into())));
        assert!(d.lock().unwrap().secimler.contains(&(2, "eski-2".into())));
    }
    #[test]
    fn pcm_little_endian_ve_surucu_uyarisi() {
        let d = Arc::new(Mutex::new(Durum::default()));
        let mut s = Oturum::yeni(Sahte(d.clone()));
        s.ac().unwrap();
        s.ses(&[0x34, 0x12].repeat(320)).unwrap();
        s.tick(Instant::now()).unwrap();
        assert_eq!(d.lock().unwrap().yazilan, vec![0x1234; 160]);
        assert_eq!(s.tampon.len(), 160);
        assert_eq!(
            kullanici_uyarisi(&anyhow::anyhow!(SURUCU_UYARISI)),
            SURUCU_UYARISI
        );
        assert!(SURUCU_UYARISI.contains("https://vb-audio.com/Cable/"));
        assert_eq!(
            kullanici_uyarisi(&anyhow::anyhow!("COM hata 0x123")),
            SES_UYARISI
        );
    }
}
