//! Windows donanım H.264. COM nesneleri yalnız MTA işçisinde yaşar.
use crate::goruntu::Goruntu;
use anyhow::{bail, ensure, Context};
use openh264::formats::{RgbaSliceU8, YUVBuffer, YUVSource};
use std::{mem::ManuallyDrop, sync::{mpsc, OnceLock}, time::{Duration, Instant}};
use windows::{core::{GUID, Interface}, Win32::{Media::MediaFoundation::*, System::{
    Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_MULTITHREADED},
    Variant::VARIANT,
}}};

#[derive(Debug, thiserror::Error)]
#[error("Donanım H.264 MFT bulunamadı")]
struct DonanimYok;

enum Istek {
    Kare(Goruntu, bool, mpsc::SyncSender<anyhow::Result<Vec<u8>>>),
    Bitrate(u32, mpsc::SyncSender<anyhow::Result<()>>),
}

/// Send/Sync COM işaretlemesi gerektirmez: arayüzler kanaldan geçirilmez.
pub struct MfKodlayici {
    kanal: mpsc::Sender<Istek>,
    pub(crate) gen: u32,
    pub(crate) yuk: u32,
    pub(crate) fps: u32,
    pub(crate) bitrate: u32,
    pub ad: String,
}

impl MfKodlayici {
    pub fn yeni(gen: u32, yuk: u32, fps: u32, bitrate: u32) -> anyhow::Result<Self> {
        ensure!(gen >= 2 && yuk >= 2 && gen % 2 == 0 && yuk % 2 == 0
            && gen.max(yuk) <= 3840 && gen.min(yuk) <= 2160, "H.264 boyutu desteklenmiyor");
        let fps = fps.clamp(1, 60);
        let bitrate = bitrate.clamp(250_000, 20_000_000);
        let (kanal, alici) = mpsc::channel();
        let (hazir, bekle) = mpsc::sync_channel(1);
        std::thread::Builder::new().name("afudesk-mf".into()).spawn(move || {
            // SAFETY: COM kurulumu, bütün COM çağrıları ve bırakma aynı işçidedir.
            let sonuc = unsafe { ComOturumu::yeni().and_then(|com| {
                let mf = Oturum::yeni(gen, yuk, fps, bitrate)?;
                Ok((com, mf))
            }) };
            let (_com, mut mf) = match sonuc {
                Ok(v) => v,
                Err(e) => { let _ = hazir.send(Err(e)); return; }
            };
            if hazir.send(Ok(mf.ad.clone())).is_err() { return; }
            while let Ok(istek) = alici.recv() {
                match istek {
                    Istek::Kare(g, anahtar, cevap) => {
                        let sonuc = unsafe { mf.kodla(&g, anahtar) };
                        let hata = sonuc.is_err();
                        let _ = cevap.send(sonuc);
                        if hata { break; }
                    }
                    Istek::Bitrate(b, cevap) => {
                        let _ = cevap.send(unsafe { mf.ayar(&CODECAPI_AVEncCommonMeanBitRate, b.into()) });
                    }
                }
            }
            // mf COM'dan önce bırakılır; kanal kapanınca işçi kendiliğinden sonlanır.
        })?;
        let ad = bekle.recv_timeout(Duration::from_secs(10)).context("MF başlatma zaman aşımı")??;
        log::info!("H.264 donanım kodlayıcısı: {ad}");
        Ok(Self { kanal, gen, yuk, fps, bitrate, ad })
    }

    pub fn kodla(&mut self, g: &Goruntu, anahtar_iste: bool) -> anyhow::Result<Vec<u8>> {
        ensure!((g.genislik, g.yukseklik) == (self.gen, self.yuk)
            && g.rgba.len() == self.gen as usize * self.yuk as usize * 4, "RGBA boyutu uyuşmuyor");
        let (cevap, alici) = mpsc::sync_channel(1);
        self.kanal.send(Istek::Kare(g.clone(), anahtar_iste, cevap))?;
        alici.recv_timeout(Duration::from_secs(2)).context("MF kodlama zaman aşımı")?
    }

    pub fn bitrate_ayarla(&mut self, bitrate: u32) -> anyhow::Result<()> {
        let bitrate = bitrate.clamp(250_000, 20_000_000);
        let (cevap, alici) = mpsc::sync_channel(1);
        self.kanal.send(Istek::Bitrate(bitrate, cevap))?;
        alici.recv_timeout(Duration::from_secs(2)).context("MF bitrate zaman aşımı")??;
        self.bitrate = bitrate;
        Ok(())
    }
}

struct ComOturumu;
impl ComOturumu {
    unsafe fn yeni() -> anyhow::Result<Self> {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let com = Self;
        static MF: OnceLock<Result<(), windows::core::HRESULT>> = OnceLock::new();
        MF.get_or_init(|| MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(|e| e.code()))
            .as_ref().map_err(|h| windows::core::Error::from_hresult(*h))?;
        Ok(com)
    }
}
impl Drop for ComOturumu {
    fn drop(&mut self) { unsafe { CoUninitialize() } }
}

/// MFTEnumEx dizisinin bütün arayüzlerini ve COM belleğini bırakır.
struct Etkinlestirmeler(*mut Option<IMFActivate>, u32);
impl Drop for Etkinlestirmeler {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                for i in 0..self.1 as usize { std::ptr::drop_in_place(self.0.add(i)); }
                CoTaskMemFree(Some(self.0.cast()));
            }
        }
    }
}

struct Oturum {
    mft: IMFTransform,
    etkin: IMFActivate,
    olaylar: IMFMediaEventGenerator,
    api: ICodecAPI,
    ad: String,
    gen: u32,
    yuk: u32,
    fps: u32,
    sira: i64,
    giris: u32,
    cikis: u32,
    giris_hazir: bool,
    cikis_hazir: bool,
    yuv: YUVBuffer,
    baslik: Vec<u8>,
    nal_uzunluk: usize,
}

impl Oturum {
    unsafe fn yeni(gen: u32, yuk: u32, fps: u32, bitrate: u32) -> anyhow::Result<Self> {
        let giris = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Video, guidSubtype: MFVideoFormat_NV12 };
        let cikis = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Video, guidSubtype: MFVideoFormat_H264 };
        let mut liste = Etkinlestirmeler(std::ptr::null_mut(), 0);
        MFTEnumEx(MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
            Some(&giris), Some(&cikis), &mut liste.0, &mut liste.1)?;
        if liste.1 == 0 || liste.0.is_null() { return Err(DonanimYok.into()); }
        let etkin = (*liste.0).as_ref().context("MFT etkinleştirmesi boş")?.clone();
        let uzunluk = etkin.GetStringLength(&MFT_FRIENDLY_NAME_Attribute)?;
        let mut ad = vec![0u16; uzunluk as usize + 1];
        etkin.GetString(&MFT_FRIENDLY_NAME_Attribute, &mut ad, None)?;
        let ad = String::from_utf16_lossy(&ad[..uzunluk as usize]);
        let mft: IMFTransform = etkin.ActivateObject()?;
        // Kısmi kurulumda bile ShutdownObject çalışır.
        let mut s = Self {
            olaylar: mft.cast()?, api: mft.cast()?, mft, etkin,
            ad, gen, yuk, fps, sira: 0, giris: 0, cikis: 0,
            giris_hazir: false, cikis_hazir: false,
            yuv: YUVBuffer::new(gen as usize, yuk as usize), baslik: Vec::new(), nal_uzunluk: 4,
        };
        s.mft.GetAttributes()?.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1)?;
        if let Err(e) = s.mft.GetStreamIDs(std::slice::from_mut(&mut s.giris), std::slice::from_mut(&mut s.cikis)) {
            // E_NOTIMPL: varsayılan akış kimlikleri 0'dır.
            if e.code() != windows::Win32::Foundation::E_NOTIMPL { return Err(e.into()); }
        }
        s.ayar(&CODECAPI_AVLowLatencyMode, true.into())?;
        s.ayar(&CODECAPI_AVEncCommonRateControlMode, (eAVEncCommonRateControlMode_CBR.0 as u32).into())?;
        let ulos = s.tyyppi(MFVideoFormat_H264)?;
        ulos.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
        // NVIDIA Baseline/CAVLC çıktısı OpenH264'ün CAVLC okuyucusunda reddediliyor
        // (FFmpeg aynı akışı çözüyor). Main/CABAC iki çözücüyle de uyumludur.
        ulos.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_Main.0 as u32)?;
        s.mft.SetOutputType(s.cikis, &ulos, 0)?;
        s.mft.SetInputType(s.giris, &s.tyyppi(MFVideoFormat_NV12)?, 0)?;
        // NVIDIA MFT bu iki CodecAPI özelliğini E_NOTIMPL olarak bildirir.
        // LowLatencyMode yeniden sıralamayı kapatır; GOP aşağıda IDR ile de uygulanır.
        s.istege_bagli_ayar(&CODECAPI_AVEncMPVDefaultBPictureCount, 0)?;
        s.istege_bagli_ayar(&CODECAPI_AVEncMPVGOPSize, fps * 2)?;
        s.ayar(&CODECAPI_AVEncCommonMeanBitRate, bitrate.into())?;
        s.baslik_guncelle()?;
        s.mft.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
        s.mft.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)?;
        Ok(s)
    }

    unsafe fn tyyppi(&self, alt: GUID) -> anyhow::Result<IMFMediaType> {
        let t = MFCreateMediaType()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        t.SetGUID(&MF_MT_SUBTYPE, &alt)?;
        t.SetUINT64(&MF_MT_FRAME_SIZE, ((self.gen as u64) << 32) | self.yuk as u64)?;
        t.SetUINT64(&MF_MT_FRAME_RATE, ((self.fps as u64) << 32) | 1)?;
        t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1)?;
        t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        Ok(t)
    }

    unsafe fn ayar(&self, anahtar: &GUID, deger: VARIANT) -> anyhow::Result<()> {
        self.api.SetValue(anahtar, &deger).with_context(|| {
            let mevcut = self.api.GetValue(anahtar).map(|v| format!("{:?}", v.vt()));
            format!("MF {} codec ayarı: {anahtar:?}, verilen={:?}, mevcut={mevcut:?}, destek={:?}",
                self.ad, deger.vt(), self.api.IsSupported(anahtar))
        })
    }

    unsafe fn istege_bagli_ayar(&self, anahtar: &GUID, deger: u32) -> anyhow::Result<()> {
        match self.ayar(anahtar, deger.into()) {
            Ok(()) => Ok(()),
            Err(e) if self.api.IsSupported(anahtar).err().is_some_and(|e|
                e.code() == windows::Win32::Foundation::E_NOTIMPL) => {
                log::info!("MF ayarı profil/IDR ile uygulanıyor: {e:#}");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    unsafe fn baslik_guncelle(&mut self) -> anyhow::Result<()> {
        let t = self.mft.GetOutputCurrentType(self.cikis)?;
        if let Ok(n) = t.GetBlobSize(&MF_MT_MPEG_SEQUENCE_HEADER) {
            let mut b = vec![0; n as usize];
            t.GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut b, None)?;
            if !b.is_empty() { (self.baslik, self.nal_uzunluk) = sira_basligi(&b)?; }
        }
        Ok(())
    }

    unsafe fn bekle(&mut self, cikis: bool) -> anyhow::Result<()> {
        let son = Instant::now() + Duration::from_millis(250);
        while if cikis { !self.cikis_hazir } else { !self.giris_hazir } {
            ensure!(Instant::now() < son, "MF olay zaman aşımı");
            match self.olaylar.GetEvent(MF_EVENT_FLAG_NO_WAIT) {
                Ok(e) => {
                    e.GetStatus()?.ok()?;
                    let tip = e.GetType()?;
                    if tip == METransformNeedInput.0 as u32 { self.giris_hazir = true; }
                    if tip == METransformHaveOutput.0 as u32 { self.cikis_hazir = true; }
                    if tip == MEError.0 as u32 { bail!("MF kodlayıcı olayı hata bildirdi"); }
                }
                Err(e) if e.code() == MF_E_NO_EVENTS_AVAILABLE => std::thread::sleep(Duration::from_millis(1)),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    unsafe fn kodla(&mut self, g: &Goruntu, anahtar: bool) -> anyhow::Result<Vec<u8>> {
        self.bekle(false)?;
        if anahtar || self.sira % (self.fps as i64 * 2) == 0 {
            self.ayar(&CODECAPI_AVEncVideoForceKeyFrame, 1u32.into())?;
        }
        // OpenH264 ile aynı SIMD/BT.601 limited dönüşümü; yalnız U/V paketlemesi değişir.
        self.yuv.read_rgba8(RgbaSliceU8::new(&g.rgba, (self.gen as usize, self.yuk as usize)));
        let boy = self.gen as usize * self.yuk as usize;
        let tampon = MFCreateMemoryBuffer((boy * 3 / 2) as u32)?;
        {
            let kilit = TamponKilidi::yeni(&tampon)?;
            let nv12 = std::slice::from_raw_parts_mut(kilit.ptr, boy * 3 / 2);
            nv12[..boy].copy_from_slice(self.yuv.y());
            for ((uv, u), v) in nv12[boy..].chunks_exact_mut(2).zip(self.yuv.u()).zip(self.yuv.v()) {
                uv.copy_from_slice(&[*u, *v]);
            }
        }
        tampon.SetCurrentLength((boy * 3 / 2) as u32)?;
        let kare = MFCreateSample()?;
        kare.AddBuffer(&tampon)?;
        kare.SetSampleTime(self.sira * 10_000_000 / self.fps as i64)?;
        kare.SetSampleDuration(10_000_000 / self.fps as i64)?;
        self.mft.ProcessInput(self.giris, &kare, 0)?;
        self.giris_hazir = false;
        self.sira += 1;
        self.bekle(true)?;
        self.cikis_hazir = false;
        let veri = self.cikti()?;
        self.baslik_guncelle()?;
        let mut annex = annex_b(&veri, self.nal_uzunluk)?;
        if crate::h264::anahtar_mi(&annex) {
            let tipler = nal_tipleri(&annex);
            if !tipler.contains(&7) || !tipler.contains(&8) {
                ensure!(!self.baslik.is_empty(), "MF SPS/PPS eksik");
                let mut birlesik = self.baslik.clone();
                birlesik.extend_from_slice(&annex);
                annex = birlesik;
            }
        }
        Ok(annex)
    }

    unsafe fn cikti(&mut self) -> anyhow::Result<Vec<u8>> {
        for _ in 0..2 {
            let bilgi = self.mft.GetOutputStreamInfo(self.cikis)?;
            let ornek = if bilgi.dwFlags & (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32
                | MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES.0 as u32) != 0 { None } else {
                let s = MFCreateSample()?;
                let boy = bilgi.cbSize.max(self.gen * self.yuk * 2);
                s.AddBuffer(&MFCreateAlignedMemoryBuffer(boy, bilgi.cbAlignment.saturating_sub(1))?)?;
                Some(s)
            };
            let mut o = MFT_OUTPUT_DATA_BUFFER { dwStreamID: self.cikis,
                pSample: ManuallyDrop::new(ornek), dwStatus: 0, pEvents: ManuallyDrop::new(None) };
            let mut durum = 0;
            let sonuc = self.mft.ProcessOutput(0, std::slice::from_mut(&mut o), &mut durum);
            // Her hata yolunda da MFT'nin verdiği COM referanslarını bırak.
            let ornek = ManuallyDrop::take(&mut o.pSample);
            ManuallyDrop::drop(&mut o.pEvents);
            match sonuc {
                Err(e) if e.code() == MF_E_TRANSFORM_STREAM_CHANGE => {
                    let t = self.mft.GetOutputAvailableType(self.cikis, 0)?;
                    self.mft.SetOutputType(self.cikis, &t, 0)?;
                    self.baslik_guncelle()?;
                }
                Err(e) => return Err(e.into()),
                Ok(()) => {
                    let b = ornek.context("MF çıktı örneği boş")?.ConvertToContiguousBuffer()?;
                    let kilit = TamponKilidi::yeni(&b)?;
                    return Ok(std::slice::from_raw_parts(kilit.ptr, kilit.len as usize).to_vec());
                }
            }
        }
        bail!("MF çıktı biçimi kararsız")
    }
}

impl Drop for Oturum {
    fn drop(&mut self) {
        unsafe {
            let _ = self.mft.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0);
            let _ = self.mft.ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0);
            let _ = self.etkin.ShutdownObject();
        }
    }
}

struct TamponKilidi<'a> { tampon: &'a IMFMediaBuffer, ptr: *mut u8, len: u32 }
impl<'a> TamponKilidi<'a> {
    unsafe fn yeni(tampon: &'a IMFMediaBuffer) -> windows::core::Result<Self> {
        let mut ptr = std::ptr::null_mut();
        let mut len = 0;
        tampon.Lock(&mut ptr, None, Some(&mut len))?;
        Ok(Self { tampon, ptr, len })
    }
}
impl Drop for TamponKilidi<'_> {
    fn drop(&mut self) { unsafe { let _ = self.tampon.Unlock(); } }
}

fn nal_tipleri(b: &[u8]) -> Vec<u8> {
    b.windows(4).filter(|w| w[..3] == [0, 0, 1]).map(|w| w[3] & 31).collect()
}

fn annex_b(b: &[u8], uzunluk: usize) -> anyhow::Result<Vec<u8>> {
    ensure!(!b.is_empty(), "MF boş çıktı");
    if b.starts_with(&[0, 0, 1]) || b.starts_with(&[0, 0, 0, 1]) { return Ok(b.to_vec()); }
    ensure!((1..=4).contains(&uzunluk), "AVCC NAL uzunluğu geçersiz");
    let mut sonuc = Vec::with_capacity(b.len() + 16);
    let mut kalan = b;
    while kalan.len() >= uzunluk {
        let n = kalan[..uzunluk].iter().fold(0usize, |n, v| (n << 8) | *v as usize);
        kalan = &kalan[uzunluk..];
        if n == 0 || n > kalan.len() { break; }
        sonuc.extend_from_slice(&[0, 0, 0, 1]);
        sonuc.extend_from_slice(&kalan[..n]);
        kalan = &kalan[n..];
        if kalan.is_empty() { return Ok(sonuc); }
    }
    // Bazı MFT'ler tek NAL'ı boy/başlangıç kodu olmadan verir.
    ensure!(sonuc.is_empty() && b[0] & 0x80 == 0 && (1..=12).contains(&(b[0] & 31)), "MF NAL biçimi geçersiz");
    Ok([&[0, 0, 0, 1][..], b].concat())
}

fn sira_basligi(b: &[u8]) -> anyhow::Result<(Vec<u8>, usize)> {
    if b.first() != Some(&1) { return Ok((annex_b(b, 4)?, 4)); }
    ensure!(b.len() >= 7, "avcC başlığı eksik");
    let uzunluk = (b[4] & 3) as usize + 1;
    let mut pos = 6;
    let mut sonuc = Vec::new();
    let mut sayi = b[5] & 31;
    for grup in 0..2 {
        if grup == 1 { ensure!(pos < b.len(), "avcC PPS sayısı eksik"); sayi = b[pos]; pos += 1; }
        for _ in 0..sayi {
            ensure!(pos + 2 <= b.len(), "avcC NAL boyu eksik");
            let n = u16::from_be_bytes([b[pos], b[pos + 1]]) as usize;
            pos += 2;
            ensure!(n > 0 && pos + n <= b.len(), "avcC NAL eksik");
            sonuc.extend_from_slice(&[0, 0, 0, 1]);
            sonuc.extend_from_slice(&b[pos..pos + n]);
            pos += n;
        }
    }
    ensure!(nal_tipleri(&sonuc).contains(&7) && nal_tipleri(&sonuc).contains(&8), "avcC SPS/PPS eksik");
    Ok((sonuc, uzunluk))
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::{goruntu::deneme_goruntu, h264::{anahtar_mi, H264Cozucu}};

    #[test]
    fn mf_hatasinda_ayni_kare_yazilim_idr_ile_devam_eder() {
        let (kanal, alici) = mpsc::channel();
        let isci = std::thread::spawn(move || {
            let Istek::Kare(_, _, cevap) = alici.recv().unwrap() else { panic!("kare bekleniyor") };
            cevap.send(Err(anyhow::anyhow!("test MF aygıt hatası"))).unwrap();
            assert!(alici.recv().is_err(), "MF tekrar denenmemeli");
        });
        let mf = MfKodlayici { kanal, gen: 128, yuk: 128, fps: 15,
            bitrate: 8_000_000, ad: "test hata".into() };
        let mut k = crate::h264::Kodlayici264::Donanim(mf);
        let mut c = H264Cozucu::yeni().unwrap();
        let veri = k.kodla(&deneme_goruntu(128, 128, 0), false).unwrap();
        assert!(k.yazilimda());
        assert!(anahtar_mi(&veri));
        c.coz(&veri).unwrap();
        k.bitrate_ayarla(7_000_000).unwrap();
        let veri = k.kodla(&deneme_goruntu(128, 128, 11), false).unwrap();
        assert!(!anahtar_mi(&veri));
        c.coz(&veri).unwrap();
        isci.join().unwrap();
    }

    #[test]
    fn avcc_ve_annex_b() {
        let b = [0, 0, 0, 2, 0x65, 0x80, 0, 0, 0, 2, 0x41, 0x80];
        let sonuc = annex_b(&b, 4).unwrap();
        assert!(anahtar_mi(&sonuc));
        assert_eq!(annex_b(&sonuc, 4).unwrap(), sonuc);
        assert!(annex_b(&[0, 0, 0, 20, 0x65], 4).is_err());
        let avcc = [1, 66, 0, 31, 255, 225, 0, 2, 0x67, 1, 1, 0, 2, 0x68, 2];
        let (s, n) = sira_basligi(&avcc).unwrap();
        assert_eq!(n, 4);
        assert_eq!(nal_tipleri(&s), [7, 8]);
        for len in 0..avcc.len() { assert!(sira_basligi(&avcc[..len]).is_err()); }
    }

    #[test]
    fn donanim_kodla_coz_psnr() {
        let mut k = match MfKodlayici::yeni(320, 180, 15, 8_000_000) {
            Ok(k) => k,
            Err(e) if e.downcast_ref::<DonanimYok>().is_some() => {
                eprintln!("MF kullanılamıyor (GPU'suz CI atlar): {e:#}"); return;
            }
            Err(e) => panic!("Donanım bulundu ancak MF kurulamadı: {e:#}"),
        };
        eprintln!("MF MFT: {}", k.ad);
        let mut c = H264Cozucu::yeni().unwrap();
        for n in 0..8 {
            if n == 3 { k.bitrate_ayarla(7_000_000).unwrap(); }
            let g = deneme_goruntu(320, 180, n * 11);
            let veri = k.kodla(&g, n == 4).unwrap();
            eprintln!("MF kare {n}: {} bayt, NAL={:?}", veri.len(), nal_tipleri(&veri));
            if n == 0 || n == 4 { assert!(anahtar_mi(&veri)); }
            let sonuc = c.coz(&veri).with_context(|| format!("MF kare {n} çözülemedi")).unwrap();
            assert_eq!((sonuc.genislik, sonuc.yukseklik), (320, 180));
            let mse = g.rgba.chunks_exact(4).zip(sonuc.rgba.chunks_exact(4))
                .flat_map(|(a,b)| (0..3).map(move |i| (a[i] as f64 - b[i] as f64).powi(2)))
                .sum::<f64>() / (320.0 * 180.0 * 3.0);
            let psnr = 10.0 * (255.0f64.powi(2) / mse).log10();
            eprintln!("MF kare {n}: PSNR={psnr:.2} dB");
            assert!(psnr > 30.0, "PSNR={psnr}");
        }
    }
}
