//! Görüntü: ekranı 64×64 döşemelere böl, yalnız değişenleri JPEG olarak gönder.
//! İzleyici tarafı döşemeleri RGBA tampona yerleştirir.
use crate::protokol::{Doseme, Kare, KareKipi};
use crate::h264::{H264Kodlayici, H264Cozucu};
use image::{codecs::jpeg::JpegEncoder, ExtendedColorType, ImageDecoder};
use rayon::prelude::*;
use std::time::{Duration, Instant};

/// Bu genişliği aşan ekranlar yayından önce yarıya küçültülür.
pub const AZAMI_GENISLIK: u32 = 2560;

/// 2×2 ortalamayla yarı ölçek (tek sayılı son satır/sütun atılır).
pub fn yarim_olcek(g: &Goruntu) -> Goruntu {
    let (yg, yy) = (g.genislik / 2, g.yukseklik / 2);
    let satir = (g.genislik * 4) as usize;
    let mut rgba = vec![0u8; (yg * yy * 4) as usize];
    rgba.par_chunks_mut((yg * 4) as usize)
        .enumerate()
        .for_each(|(y, hedef)| {
            let ust = &g.rgba[2 * y * satir..];
            let alt = &g.rgba[(2 * y + 1) * satir..];
            for x in 0..yg as usize {
                for k in 0..4 {
                    let t = ust[8 * x + k] as u16
                        + ust[8 * x + 4 + k] as u16
                        + alt[8 * x + k] as u16
                        + alt[8 * x + 4 + k] as u16;
                    hedef[4 * x + k] = (t / 4) as u8;
                }
            }
        });
    Goruntu {
        genislik: yg,
        yukseklik: yy,
        rgba,
    }
}

/// Yayın boyutuna getir: gerekirse art arda yarıya indir.
pub fn yayin_boyutu(mut g: Goruntu) -> Goruntu {
    while g.genislik > AZAMI_GENISLIK {
        g = yarim_olcek(&g);
    }
    g
}

pub const DOSEME: u32 = 64;

/// Ham RGBA ekran görüntüsü.
#[derive(Clone)]
pub struct Goruntu {
    pub genislik: u32,
    pub yukseklik: u32,
    pub rgba: Vec<u8>,
}

pub struct Kodlayici {
    onceki: Option<Goruntu>,
    /// Ulaşmadan iptal edilen karelerin döşemeleri: değişmeseler de yeniden gönderilir.
    zorunlu: std::collections::HashSet<(u32, u32)>,
    kalite: u8,
    sira: u64,
    /// Bu kadar karede bir tam kare gönder (paket kaybı/yeni izleyici için).
    tam_aralik: u64,
    otomatik: bool,
    oranlar: std::collections::VecDeque<f64>,
    h264: Option<H264Kodlayici>,
    h264_etkin: bool,
    anahtar: bool,
    fps: u32,
    bitrate: u32,
    h264_sureleri: std::collections::VecDeque<Duration>,
    h264_kilit: Option<Instant>,
    #[cfg(test)]
    test_h264_suresi: Option<std::time::Duration>,
}

impl Kodlayici {
    pub fn new(kalite: u8) -> Self {
        Self {
            onceki: None,
            zorunlu: Default::default(),
            kalite,
            sira: 0,
            tam_aralik: 150,
            otomatik: false, oranlar: Default::default(), h264: None,
            h264_etkin: false, anahtar: true, fps: 15, bitrate: 8_000_000,
            h264_sureleri: Default::default(), h264_kilit: None,
            #[cfg(test)]
            test_h264_suresi: None,
        }
    }

    pub fn otomatik(kalite: u8) -> Self {
        Self { otomatik: true, ..Self::new(kalite) }
    }

    pub fn h264_etkin(&self) -> bool { self.h264_etkin }

    pub fn yayin_ayarla(&mut self, fps: u32, bitrate: u32) {
        let fps = fps.clamp(1, 60);
        let bitrate = bitrate.clamp(250_000, 20_000_000);
        if self.fps != fps {
            self.fps = fps;
            self.h264 = None;
            self.h264_sureleri.clear();
            self.anahtar = true;
        }
        if self.bitrate != bitrate {
            if let Some(h) = &mut self.h264 {
                // Hata halinde eski ayar korunur; sonraki çağrı tekrar dener.
                if h.bitrate_ayarla(bitrate).is_err() { return; }
            }
            self.bitrate = bitrate;
        }
    }

    pub fn kalite_ayarla(&mut self, k: u8) {
        self.kalite = k.clamp(20, 95);
    }

    pub fn kalite(&self) -> u8 {
        self.kalite
    }

    /// İptal edilen bir karenin döşemelerini bir sonraki karede yeniden gönder.
    pub fn yeniden_gonder(&mut self, konumlar: impl IntoIterator<Item = (u32, u32)>) {
        self.zorunlu.extend(konumlar);
        self.anahtar = true;
    }

    pub fn tam_kare_iste(&mut self) {
        self.onceki = None;
        self.anahtar = true;
    }

    /// Değişiklik yoksa `None`.
    pub fn kodla(&mut self, g: &Goruntu) -> anyhow::Result<Option<Kare>> {
        anyhow::ensure!(
            g.rgba.len() == (g.genislik * g.yukseklik * 4) as usize,
            "RGBA boyutu uyuşmuyor"
        );
        let boyut_degisti = self
            .onceki
            .as_ref()
            .map(|o| o.genislik != g.genislik || o.yukseklik != g.yukseklik)
            .unwrap_or(true);
        if boyut_degisti {
            self.oranlar.clear(); self.h264 = None; self.h264_etkin = false; self.anahtar = true;
            self.h264_sureleri.clear();
        }
        let mut tam = boyut_degisti || self.sira % self.tam_aralik == 0
            || (self.otomatik && self.anahtar);
        let mut degisen = 0usize;
        let mut konumlar = Vec::new();
        let mut y = 0;
        while y < g.yukseklik {
            let yuk = DOSEME.min(g.yukseklik - y);
            let mut x = 0;
            while x < g.genislik {
                let gen = DOSEME.min(g.genislik - x);
                let degisti = self.onceki.as_ref().is_some_and(|o|
                    !boyut_degisti && farkli(o, g, x, y, gen, yuk));
                degisen += usize::from(degisti);
                if tam
                    || self.zorunlu.contains(&(x, y))
                    || degisti
                {
                    konumlar.push((x, y, gen, yuk));
                }
                x += DOSEME;
            }
            y += DOSEME;
        }
        if self.otomatik {
            let toplam = g.genislik.div_ceil(DOSEME) * g.yukseklik.div_ceil(DOSEME);
            self.oranlar.push_back(degisen as f64 / toplam.max(1) as f64);
            if self.oranlar.len() > 30 { self.oranlar.pop_front(); }
            if self.oranlar.len() == 30 {
                let oran = self.oranlar.iter().sum::<f64>() / 30.0;
                // Kare bütçesinin %80'ini aşan 30 örnek CPU korumasını tetikler.
                let yavas = self.h264_sureleri.len() == 30
                    && self.h264_sureleri.iter().map(Duration::as_secs_f64).sum::<f64>() / 30.0
                        > 0.8 / self.fps as f64;
                if self.h264_etkin && yavas {
                    self.h264_kilit = Some(Instant::now() + Duration::from_secs(10));
                }
                let kilitli = self.h264_kilit.is_some_and(|t| Instant::now() < t);
                let etkin = !kilitli && if self.h264_etkin { oran >= 0.10 } else { oran > 0.40 };
                if etkin != self.h264_etkin {
                    self.h264_etkin = etkin; self.anahtar = true;
                    self.h264_sureleri.clear();
                    if !etkin { tam = true; }
                }
            }
            if self.h264_etkin {
                let bas = Instant::now();
                let sonuc = (|| -> anyhow::Result<Vec<u8>> {
                    if self.h264.is_none() {
                        self.h264 = Some(H264Kodlayici::yeni(g.genislik, g.yukseklik, self.fps, self.bitrate)?);
                    }
                    self.h264.as_mut().unwrap().kodla(g, self.anahtar)
                })();
                match sonuc {
                    Ok(veri) => {
                        let sure = bas.elapsed();
                        #[cfg(test)]
                        let sure = self.test_h264_suresi.unwrap_or(sure);
                        self.h264_sureleri.push_back(sure);
                        if self.h264_sureleri.len() > 30 { self.h264_sureleri.pop_front(); }
                        if veri.is_empty() { self.onceki = Some(g.clone()); return Ok(None); }
                        let tam = crate::h264::anahtar_mi(&veri);
                        self.anahtar = false;
                        self.onceki = Some(g.clone()); self.zorunlu.clear(); self.sira += 1;
                        return Ok(Some(Kare { sira: self.sira, genislik: g.genislik,
                            yukseklik: g.yukseklik, tam, dosemeler: Vec::new(),
                            kip: KareKipi::H264(veri), yakalama_ms: 0, kodlama_us: 0 }));
                    }
                    Err(_) => { self.h264_etkin = false; self.h264 = None; tam = true; }
                }
            }
        }
        // H.264 → JPEG geçişinde değişmeyen döşemeler de tam gönderilir.
        if tam && konumlar.len() != (g.genislik.div_ceil(DOSEME) * g.yukseklik.div_ceil(DOSEME)) as usize {
            konumlar.clear();
            for y in (0..g.yukseklik).step_by(DOSEME as usize) {
                for x in (0..g.genislik).step_by(DOSEME as usize) {
                    konumlar.push((x, y, DOSEME.min(g.genislik-x), DOSEME.min(g.yukseklik-y)));
                }
            }
        }
        let kalite = self.kalite;
        let dosemeler = konumlar
            .into_par_iter()
            .map(|(x, y, gen, yuk)| {
                Ok(Doseme {
                    x,
                    y,
                    gen,
                    yuk,
                    jpeg: jpeg_kodla(g, x, y, gen, yuk, kalite)?,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.onceki = Some(g.clone());
        self.zorunlu.clear();
        if dosemeler.is_empty() {
            return Ok(None);
        }
        self.sira += 1;
        self.anahtar = false;
        Ok(Some(Kare {
            sira: self.sira,
            genislik: g.genislik,
            yukseklik: g.yukseklik,
            tam,
            dosemeler,
            kip: KareKipi::Doseme,
            yakalama_ms: 0,
            kodlama_us: 0,
        }))
    }
}

#[cfg(test)]
mod duzelt_testleri {
    use super::*;
    use std::time::Duration;

    #[test]
    fn yavas_h264_otuz_ornekten_sonra_jpege_doner_ve_kilitlenir() {
        let mut k = Kodlayici::otomatik(80);
        k.test_h264_suresi = Some(Duration::from_millis(150));
        for n in 0..59 {
            let kare = k.kodla(&deneme_goruntu(128, 128, n)).unwrap().unwrap();
            assert_eq!(matches!(kare.kip, KareKipi::H264(_)), n >= 29);
        }
        let kare = k.kodla(&deneme_goruntu(128, 128, 59)).unwrap().unwrap();
        assert!(matches!(kare.kip, KareKipi::Doseme));
        assert!(kare.tam, "JPEG geçişi bütün döşemeleri göndermeli");
        assert_eq!(kare.dosemeler.len(), 4);
        for n in 60..100 {
            let kare = k.kodla(&deneme_goruntu(128, 128, n)).unwrap().unwrap();
            assert!(matches!(kare.kip, KareKipi::Doseme), "CPU kilidi korunmalı");
        }
        // Duvar saatinde 10 saniye beklemek yerine yalnız kilit sonunu geçmişe taşı.
        k.h264_kilit = Some(Instant::now() - Duration::from_millis(1));
        k.test_h264_suresi = Some(Duration::from_millis(1));
        let kare = k.kodla(&deneme_goruntu(128, 128, 100)).unwrap().unwrap();
        assert!(matches!(kare.kip, KareKipi::H264(_)));
        assert!(kare.tam, "kilit sonrası H264 yeni IDR ile başlamalı");
    }

    #[test]
    fn cpu_koruma_ortalama_ve_fps_butcesini_kullanir() {
        let mut k = Kodlayici::otomatik(80);
        // 15 FPS'te 20 ms bütçeye sığar, 60 FPS'te %80 bütçesini aşar.
        k.test_h264_suresi = Some(Duration::from_millis(20));
        for n in 0..65 { k.kodla(&deneme_goruntu(128, 128, n)).unwrap(); }
        assert!(k.h264_etkin());
        k.yayin_ayarla(60, 8_000_000);
        for n in 65..95 { k.kodla(&deneme_goruntu(128, 128, n)).unwrap(); }
        let kare = k.kodla(&deneme_goruntu(128, 128, 95)).unwrap().unwrap();
        assert!(matches!(kare.kip, KareKipi::Doseme));
    }

    #[test]
    fn kucuk_bitrate_adimlari_yeni_idr_uretmez() {
        let mut k = Kodlayici::otomatik(80);
        k.test_h264_suresi = Some(Duration::from_millis(1));
        for n in 0..30 { k.kodla(&deneme_goruntu(128, 128, n)).unwrap(); }
        assert!(k.h264_etkin());
        for (n, bitrate) in [7_900_000, 7_800_000, 7_700_000].into_iter().enumerate() {
            k.yayin_ayarla(15, bitrate);
            let kare = k.kodla(&deneme_goruntu(128, 128, 30 + n as u8)).unwrap().unwrap();
            let KareKipi::H264(veri) = kare.kip else { panic!("H264 bekleniyor") };
            assert!(!veri.is_empty());
            assert!(!crate::h264::anahtar_mi(&veri), "bitrate IDR üretmemeli");
        }
        k.yayin_ayarla(30, 7_700_000);
        let kare = k.kodla(&deneme_goruntu(128, 128, 34)).unwrap().unwrap();
        let KareKipi::H264(veri) = kare.kip else { panic!("H264 bekleniyor") };
        assert!(crate::h264::anahtar_mi(&veri), "FPS değişimi IDR üretmeli");
    }
}

fn farkli(a: &Goruntu, b: &Goruntu, x: u32, y: u32, gen: u32, yuk: u32) -> bool {
    let satir = (b.genislik * 4) as usize;
    for yy in y..y + yuk {
        let bas = yy as usize * satir + x as usize * 4;
        let son = bas + gen as usize * 4;
        if a.rgba[bas..son] != b.rgba[bas..son] {
            return true;
        }
    }
    false
}

fn jpeg_kodla(
    g: &Goruntu,
    x: u32,
    y: u32,
    gen: u32,
    yuk: u32,
    kalite: u8,
) -> anyhow::Result<Vec<u8>> {
    let mut rgb = Vec::with_capacity((gen * yuk * 3) as usize);
    let satir = (g.genislik * 4) as usize;
    for yy in y..y + yuk {
        let bas = yy as usize * satir + x as usize * 4;
        for p in g.rgba[bas..bas + gen as usize * 4].chunks_exact(4) {
            rgb.extend_from_slice(&p[..3]);
        }
    }
    let mut cikti = Vec::new();
    JpegEncoder::new_with_quality(&mut cikti, kalite).encode(
        &rgb,
        gen,
        yuk,
        ExtendedColorType::Rgb8,
    )?;
    Ok(cikti)
}

/// İzleyici: gelen kareleri birleştirir.
pub struct Birlestirici {
    pub goruntu: Goruntu,
    /// Her döşemeye en son uygulanan kare sırası: kareler bağımsız akışlardan
    /// sırasız gelebilir; eski kare yeni içeriğin üstüne yazamaz.
    doseme_sira: std::collections::HashMap<(u32, u32), u64>,
    h264: Option<H264Cozucu>,
    son_h264: Option<u64>,
    anahtar_bekle: bool,
    taban_sira: u64,
    en_yeni: u64,
}

impl Default for Birlestirici {
    fn default() -> Self {
        Self {
            goruntu: Goruntu {
                genislik: 0,
                yukseklik: 0,
                rgba: Vec::new(),
            },
            doseme_sira: Default::default(),
            h264: None, son_h264: None, anahtar_bekle: true, taban_sira: 0, en_yeni: 0,
        }
    }
}

impl Birlestirici {
    pub fn uygula(&mut self, k: &Kare) -> anyhow::Result<()> {
        anyhow::ensure!(
            k.genislik > 0 && k.yukseklik > 0 && k.genislik <= 16384 && k.yukseklik <= 16384,
            "geçersiz kare boyutu"
        );
        if k.sira <= self.taban_sira { return Ok(()); }
        if let KareKipi::H264(veri) = &k.kip {
            if k.sira <= self.en_yeni { return Ok(()); }
            let anahtar = crate::h264::anahtar_mi(veri);
            if !anahtar && (self.anahtar_bekle || self.son_h264 != k.sira.checked_sub(1)) {
                self.anahtar_bekle = true;
                return Ok(()); // Referans kayboldu: bir sonraki IDR ile toparlan.
            }
            if anahtar { self.h264 = Some(H264Cozucu::yeni()?); }
            let sonuc = self.h264.as_mut().ok_or_else(|| anyhow::anyhow!("H.264 anahtar kare eksik"))?.coz(veri);
            match sonuc {
                Ok(g) if (g.genislik, g.yukseklik) == (k.genislik, k.yukseklik) => {
                    self.goruntu = g; self.son_h264 = Some(k.sira); self.anahtar_bekle = false;
                    self.en_yeni = k.sira; self.taban_sira = k.sira; self.doseme_sira.clear();
                }
                _ => { self.anahtar_bekle = true; }
            }
            return Ok(());
        }
        if self.son_h264.is_some() && k.tam {
            self.h264 = None; self.son_h264 = None; self.anahtar_bekle = true;
            self.taban_sira = k.sira - 1;
        }
        // Geçişin tam JPEG karesini geçen deltaları uygula. Döşeme sıraları,
        // tam kare sonradan gelince bu yeni piksellerin ezilmesini önler.
        if k.genislik != self.goruntu.genislik || k.yukseklik != self.goruntu.yukseklik {
            if k.sira < self.en_yeni { return Ok(()); }
            self.goruntu = Goruntu {
                genislik: k.genislik,
                yukseklik: k.yukseklik,
                rgba: vec![0; (k.genislik * k.yukseklik * 4) as usize],
            };
            self.doseme_sira.clear();
        }
        self.en_yeni = self.en_yeni.max(k.sira);
        let satir = (k.genislik * 4) as usize;
        for d in &k.dosemeler {
            anyhow::ensure!(
                d.gen > 0 && d.yuk > 0 && d.x + d.gen <= k.genislik && d.y + d.yuk <= k.yukseklik,
                "döşeme ekran dışında"
            );
            let son = self.doseme_sira.entry((d.x, d.y)).or_insert(0);
            if *son >= k.sira {
                continue; // bu döşemenin daha yeni hali zaten ekranda
            }
            *son = k.sira;
            let coz = image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(&d.jpeg))?;
            let (gw, gh) = coz.dimensions();
            anyhow::ensure!(gw == d.gen && gh == d.yuk, "döşeme boyutu uyuşmuyor");
            let mut rgb = vec![0u8; coz.total_bytes() as usize];
            coz.read_image(&mut rgb)?;
            for yy in 0..d.yuk {
                let hedef = (d.y + yy) as usize * satir + d.x as usize * 4;
                let kaynak = (yy * d.gen * 3) as usize;
                for xx in 0..d.gen as usize {
                    let h = hedef + xx * 4;
                    let s = kaynak + xx * 3;
                    self.goruntu.rgba[h..h + 3].copy_from_slice(&rgb[s..s + 3]);
                    self.goruntu.rgba[h + 3] = 255;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub fn deneme_goruntu(g: u32, y: u32, tohum: u8) -> Goruntu {
    let mut rgba = Vec::with_capacity((g * y * 4) as usize);
    for yy in 0..y {
        for xx in 0..g {
            rgba.extend_from_slice(&[(xx % 256) as u8, (yy % 256) as u8, tohum, 255]);
        }
    }
    Goruntu {
        genislik: g,
        yukseklik: y,
        rgba,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn yakin(a: &Goruntu, b: &Goruntu) -> bool {
        a.genislik == b.genislik
            && a.rgba
                .iter()
                .zip(&b.rgba)
                .all(|(x, y)| (*x as i32 - *y as i32).abs() <= 24)
    }

    #[test]
    fn ilk_kare_tam_ve_birlesir() {
        let g = deneme_goruntu(200, 130, 10); // döşemeye tam bölünmeyen boyut
        let mut k = Kodlayici::new(90);
        let kare = k.kodla(&g).unwrap().unwrap();
        assert!(kare.tam);
        assert_eq!(kare.dosemeler.len(), 4 * 3);
        let mut b = Birlestirici::default();
        b.uygula(&kare).unwrap();
        assert!(yakin(&b.goruntu, &g));
    }

    #[test]
    fn degisiklik_yoksa_kare_yok() {
        let g = deneme_goruntu(128, 128, 0);
        let mut k = Kodlayici::new(80);
        k.kodla(&g).unwrap();
        assert!(k.kodla(&g).unwrap().is_none());
    }

    #[test]
    fn yalniz_degisen_doseme_gider() {
        let g = deneme_goruntu(256, 128, 0);
        let mut k = Kodlayici::new(80);
        let mut b = Birlestirici::default();
        b.uygula(&k.kodla(&g).unwrap().unwrap()).unwrap();
        let once = b.goruntu.rgba.clone();
        let mut g2 = g.clone();
        // (72..88, 72..88) bloğu ikinci satır, ikinci sütun döşemesinin içinde.
        for yy in 72..88u32 {
            for xx in 72..88u32 {
                let i = ((yy * 256 + xx) * 4) as usize;
                g2.rgba[i..i + 3].copy_from_slice(&[255, 0, 0]);
            }
        }
        let kare = k.kodla(&g2).unwrap().unwrap();
        assert!(!kare.tam);
        assert_eq!(kare.dosemeler.len(), 1);
        assert_eq!((kare.dosemeler[0].x, kare.dosemeler[0].y), (64, 64));
        b.uygula(&kare).unwrap();
        // Değişen döşemenin dışı bayt bayt aynı kalmalı.
        for yy in 0..128u32 {
            for xx in 0..256u32 {
                let icinde = (64..128).contains(&xx) && (64..128).contains(&yy);
                let i = ((yy * 256 + xx) * 4) as usize;
                if !icinde {
                    assert_eq!(
                        b.goruntu.rgba[i..i + 4],
                        once[i..i + 4],
                        "({xx},{yy}) değişmemeliydi"
                    );
                }
            }
        }
        // Bloğun ortası kırmızıya yakın olmalı.
        let m = ((80 * 256 + 80) * 4) as usize;
        let p = &b.goruntu.rgba[m..m + 3];
        assert!(p[0] > 200 && p[1] < 60 && p[2] < 60, "{p:?}");
    }

    #[test]
    fn boyut_degisince_tam_kare() {
        let mut k = Kodlayici::new(80);
        k.kodla(&deneme_goruntu(128, 128, 0)).unwrap();
        let kare = k.kodla(&deneme_goruntu(192, 64, 0)).unwrap().unwrap();
        assert!(kare.tam);
        let mut b = Birlestirici::default();
        b.uygula(&kare).unwrap();
        assert_eq!((b.goruntu.genislik, b.goruntu.yukseklik), (192, 64));
    }

    #[test]
    fn kotu_kare_reddedilir() {
        let mut b = Birlestirici::default();
        let mut kare = Kodlayici::new(80)
            .kodla(&deneme_goruntu(64, 64, 0))
            .unwrap()
            .unwrap();
        kare.dosemeler[0].x = 32; // ekran dışına taşar
        assert!(b.uygula(&kare).is_err());
        let mut kare2 = Kodlayici::new(80)
            .kodla(&deneme_goruntu(64, 64, 0))
            .unwrap()
            .unwrap();
        kare2.dosemeler[0].jpeg = vec![1, 2, 3];
        assert!(b.uygula(&kare2).is_err());
        let mut kare3 = kare2.clone();
        kare3.genislik = 0;
        assert!(b.uygula(&kare3).is_err());
    }

    #[test]
    fn sirasiz_gelen_eski_kare_yeniyi_ezmez() {
        let mut k = Kodlayici::new(90);
        let siyah = Goruntu {
            genislik: 64,
            yukseklik: 64,
            rgba: [0, 0, 0, 255].repeat(64 * 64),
        };
        let beyaz = Goruntu {
            genislik: 64,
            yukseklik: 64,
            rgba: [255, 255, 255, 255].repeat(64 * 64),
        };
        let k1 = k.kodla(&siyah).unwrap().unwrap();
        let k2 = k.kodla(&beyaz).unwrap().unwrap();
        let mut b = Birlestirici::default();
        b.uygula(&k2).unwrap(); // yeni önce geldi
        b.uygula(&k1).unwrap(); // eski geç geldi: yok sayılmalı
        assert!(
            b.goruntu.rgba[0] > 240,
            "beyaz kalmalı, {}",
            b.goruntu.rgba[0]
        );
    }

    #[test]
    fn iptal_edilen_kare_dosemeleri_yeniden_gider() {
        let g = deneme_goruntu(128, 64, 0);
        let mut k = Kodlayici::new(80);
        k.kodla(&g).unwrap();
        assert!(k.kodla(&g).unwrap().is_none(), "değişiklik yok");
        k.yeniden_gonder([(64, 0)]);
        let kare = k.kodla(&g).unwrap().unwrap();
        assert_eq!(kare.dosemeler.len(), 1);
        assert_eq!((kare.dosemeler[0].x, kare.dosemeler[0].y), (64, 0));
        assert!(k.kodla(&g).unwrap().is_none(), "zorunlu liste temizlenmeli");
    }

    #[test]
    fn yarim_olcek_ortalama_alir() {
        let mut g = Goruntu {
            genislik: 4,
            yukseklik: 2,
            rgba: vec![0; 32],
        };
        // Sol 2x2 bloğun R değerleri 0,100,200,100 -> ortalama 100.
        for (i, v) in [(0usize, 0u8), (1, 100), (4, 200), (5, 100)] {
            g.rgba[i * 4] = v;
        }
        let k = yarim_olcek(&g);
        assert_eq!((k.genislik, k.yukseklik), (2, 1));
        assert_eq!(k.rgba[0], 100);
        assert_eq!(k.rgba[4], 0);
    }

    #[test]
    fn yayin_boyutu_siniri() {
        assert_eq!(yayin_boyutu(deneme_goruntu(5120, 16, 0)).genislik, 2560);
        assert_eq!(yayin_boyutu(deneme_goruntu(1920, 16, 0)).genislik, 1920);
        assert_eq!(yayin_boyutu(deneme_goruntu(7680, 16, 0)).genislik, 1920);
    }

    #[test]
    fn yanlis_rgba_boyutu() {
        let mut g = deneme_goruntu(64, 64, 0);
        g.rgba.pop();
        assert!(Kodlayici::new(80).kodla(&g).is_err());
    }
}

#[cfg(test)]
mod h264_testleri {
    use super::*;
    #[test]
    fn otomatik_kip_histerezisi_ve_iptalden_sonra_anahtar() {
        let mut k = Kodlayici::otomatik(80);
        let mut b = Birlestirici::default();
        for n in 0..31 {
            let kare = k.kodla(&deneme_goruntu(128, 128, n)).unwrap().unwrap();
            b.uygula(&kare).unwrap();
            if n < 29 { assert!(matches!(kare.kip, crate::protokol::KareKipi::Doseme)); }
        }
        assert!(k.h264_etkin());
        k.yeniden_gonder([]);
        let kare = k.kodla(&deneme_goruntu(128, 128, 32)).unwrap().unwrap();
        assert!(kare.tam);
        b.uygula(&kare).unwrap();
        for _ in 0..31 { if let Some(kare) = k.kodla(&deneme_goruntu(128, 128, 32)).unwrap() { b.uygula(&kare).unwrap(); } }
        assert!(!k.h264_etkin());
    }
}

#[cfg(test)]
mod h264_kayip_testleri {
    use super::*;
    #[test]
    fn kayip_referanstan_sonra_idr_ile_toparlanir_eski_jpeg_ezmez() {
        let mut encoder = H264Kodlayici::yeni(128, 128, 15, 8_000_000).unwrap();
        let mut b = Birlestirici::default();
        let eski = Kodlayici::new(80).kodla(&deneme_goruntu(128,128,0)).unwrap().unwrap();
        let mut son_idr = 0;
        for sira in 2..66 {
            let veri = encoder.kodla(&deneme_goruntu(128,128,sira as u8), false).unwrap();
            if veri.is_empty() { continue; }
            let tam = crate::h264::anahtar_mi(&veri);
            if tam { assert!(sira - son_idr <= 32); son_idr = sira; }
            let k = Kare { sira, genislik:128, yukseklik:128, tam, dosemeler:vec![],
                kip: KareKipi::H264(veri), yakalama_ms:0, kodlama_us:0 };
            if sira == 3 { continue; } // taşıma sırasında iptal
            b.uygula(&k).unwrap();
            if sira == 4 { assert_eq!(b.son_h264, Some(2)); assert!(b.anahtar_bekle); }
            if tam { assert_eq!(b.son_h264, Some(sira)); assert!(!b.anahtar_bekle); }
        }
        let once = b.goruntu.rgba.clone();
        b.uygula(&eski).unwrap();
        assert_eq!(once, b.goruntu.rgba);
        assert!(son_idr >= 60, "iki saniyelik GOP");
    }
}

#[cfg(test)]
mod kip_gecis_sira_testleri {
    use super::*;
    #[test]
    fn jpeg_deltasi_gecis_tam_karesinden_once_gelirse_kaybolmaz() {
        let g = deneme_goruntu(128,128,10);
        let mut h = H264Kodlayici::yeni(128,128,15,8_000_000).unwrap();
        let mut b = Birlestirici::default();
        let ilk = Kare { sira:1, genislik:128, yukseklik:128, tam:true,
            dosemeler:vec![], kip:KareKipi::H264(h.kodla(&g,true).unwrap()), yakalama_ms:0,kodlama_us:0 };
        b.uygula(&ilk).unwrap();
        let mut jpeg = Kodlayici::new(90);
        let mut tam = jpeg.kodla(&g).unwrap().unwrap(); tam.sira=2;
        let mut yeni = g.clone();
        for p in yeni.rgba.as_chunks_mut::<4>().0.iter_mut().take(32) { p[..3].copy_from_slice(&[255,255,255]); }
        let mut delta = jpeg.kodla(&yeni).unwrap().unwrap(); delta.sira=3;
        b.uygula(&delta).unwrap(); b.uygula(&tam).unwrap();
        assert!(b.goruntu.rgba[0]>230, "geç gelen tam kare yeni JPEG deltasını ezmemeli");
    }
    #[test]
    fn otomatik_jpeg_iptalinde_tam_kare_tekrarlanir() {
        let mut k = Kodlayici::otomatik(80);
        let g = deneme_goruntu(128,128,10);
        k.kodla(&g).unwrap();
        k.yeniden_gonder([(64,0)]);
        assert!(k.kodla(&g).unwrap().unwrap().tam);
    }
}
