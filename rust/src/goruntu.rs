//! Görüntü: ekranı 64×64 döşemelere böl, yalnız değişenleri JPEG olarak gönder.
//! İzleyici tarafı döşemeleri RGBA tampona yerleştirir.
use crate::protokol::{Doseme, Kare};
use image::{codecs::jpeg::JpegEncoder, ExtendedColorType, ImageDecoder};
use rayon::prelude::*;

/// Bu genişliği aşan ekranlar yayından önce yarıya küçültülür.
pub const AZAMI_GENISLIK: u32 = 2560;

/// 2×2 ortalamayla yarı ölçek (tek sayılı son satır/sütun atılır).
pub fn yarim_olcek(g: &Goruntu) -> Goruntu {
    let (yg, yy) = (g.genislik / 2, g.yukseklik / 2);
    let satir = (g.genislik * 4) as usize;
    let mut rgba = vec![0u8; (yg * yy * 4) as usize];
    rgba.par_chunks_mut((yg * 4) as usize).enumerate().for_each(|(y, hedef)| {
        let ust = &g.rgba[2 * y * satir..];
        let alt = &g.rgba[(2 * y + 1) * satir..];
        for x in 0..yg as usize {
            for k in 0..4 {
                let t = ust[8 * x + k] as u16 + ust[8 * x + 4 + k] as u16 + alt[8 * x + k] as u16 + alt[8 * x + 4 + k] as u16;
                hedef[4 * x + k] = (t / 4) as u8;
            }
        }
    });
    Goruntu { genislik: yg, yukseklik: yy, rgba }
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
    kalite: u8,
    sira: u64,
    /// Bu kadar karede bir tam kare gönder (paket kaybı/yeni izleyici için).
    tam_aralik: u64,
}

impl Kodlayici {
    pub fn new(kalite: u8) -> Self {
        Self { onceki: None, kalite, sira: 0, tam_aralik: 150 }
    }

    pub fn kalite_ayarla(&mut self, k: u8) {
        self.kalite = k.clamp(20, 95);
    }

    pub fn tam_kare_iste(&mut self) {
        self.onceki = None;
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
        let tam = boyut_degisti || self.sira % self.tam_aralik == 0;
        let mut konumlar = Vec::new();
        let mut y = 0;
        while y < g.yukseklik {
            let yuk = DOSEME.min(g.yukseklik - y);
            let mut x = 0;
            while x < g.genislik {
                let gen = DOSEME.min(g.genislik - x);
                if tam || self.onceki.as_ref().map_or(true, |o| farkli(o, g, x, y, gen, yuk)) {
                    konumlar.push((x, y, gen, yuk));
                }
                x += DOSEME;
            }
            y += DOSEME;
        }
        let kalite = self.kalite;
        let dosemeler = konumlar
            .into_par_iter()
            .map(|(x, y, gen, yuk)| Ok(Doseme { x, y, gen, yuk, jpeg: jpeg_kodla(g, x, y, gen, yuk, kalite)? }))
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.onceki = Some(g.clone());
        if dosemeler.is_empty() {
            return Ok(None);
        }
        self.sira += 1;
        Ok(Some(Kare { sira: self.sira, genislik: g.genislik, yukseklik: g.yukseklik, tam, dosemeler }))
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

fn jpeg_kodla(g: &Goruntu, x: u32, y: u32, gen: u32, yuk: u32, kalite: u8) -> anyhow::Result<Vec<u8>> {
    let mut rgb = Vec::with_capacity((gen * yuk * 3) as usize);
    let satir = (g.genislik * 4) as usize;
    for yy in y..y + yuk {
        let bas = yy as usize * satir + x as usize * 4;
        for p in g.rgba[bas..bas + gen as usize * 4].chunks_exact(4) {
            rgb.extend_from_slice(&p[..3]);
        }
    }
    let mut cikti = Vec::new();
    JpegEncoder::new_with_quality(&mut cikti, kalite).encode(&rgb, gen, yuk, ExtendedColorType::Rgb8)?;
    Ok(cikti)
}

/// İzleyici: gelen kareleri birleştirir.
pub struct Birlestirici {
    pub goruntu: Goruntu,
}

impl Default for Birlestirici {
    fn default() -> Self {
        Self { goruntu: Goruntu { genislik: 0, yukseklik: 0, rgba: Vec::new() } }
    }
}

impl Birlestirici {
    pub fn uygula(&mut self, k: &Kare) -> anyhow::Result<()> {
        anyhow::ensure!(k.genislik > 0 && k.yukseklik > 0 && k.genislik <= 16384 && k.yukseklik <= 16384, "geçersiz kare boyutu");
        if k.genislik != self.goruntu.genislik || k.yukseklik != self.goruntu.yukseklik {
            self.goruntu = Goruntu {
                genislik: k.genislik,
                yukseklik: k.yukseklik,
                rgba: vec![0; (k.genislik * k.yukseklik * 4) as usize],
            };
        }
        let satir = (k.genislik * 4) as usize;
        for d in &k.dosemeler {
            anyhow::ensure!(
                d.gen > 0 && d.yuk > 0 && d.x + d.gen <= k.genislik && d.y + d.yuk <= k.yukseklik,
                "döşeme ekran dışında"
            );
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
    Goruntu { genislik: g, yukseklik: y, rgba }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn yakin(a: &Goruntu, b: &Goruntu) -> bool {
        a.genislik == b.genislik
            && a.rgba.iter().zip(&b.rgba).all(|(x, y)| (*x as i32 - *y as i32).abs() <= 24)
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
                    assert_eq!(b.goruntu.rgba[i..i + 4], once[i..i + 4], "({xx},{yy}) değişmemeliydi");
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
        let mut kare = Kodlayici::new(80).kodla(&deneme_goruntu(64, 64, 0)).unwrap().unwrap();
        kare.dosemeler[0].x = 32; // ekran dışına taşar
        assert!(b.uygula(&kare).is_err());
        let mut kare2 = Kodlayici::new(80).kodla(&deneme_goruntu(64, 64, 0)).unwrap().unwrap();
        kare2.dosemeler[0].jpeg = vec![1, 2, 3];
        assert!(b.uygula(&kare2).is_err());
        let mut kare3 = kare2.clone();
        kare3.genislik = 0;
        assert!(b.uygula(&kare3).is_err());
    }

    #[test]
    fn yarim_olcek_ortalama_alir() {
        let mut g = Goruntu { genislik: 4, yukseklik: 2, rgba: vec![0; 32] };
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
