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
    kayipsiz: bool,
    terminal: bool,
    sira: u64,
    /// Bu kadar karede bir tam kare gönder (paket kaybı/yeni izleyici için).
    tam_aralik: u64,
}

impl Kodlayici {
    pub fn new(kalite: u8) -> Self {
        Self {
            onceki: None,
            zorunlu: Default::default(),
            kalite,
            kayipsiz: false,
            terminal: false,
            sira: 0,
            tam_aralik: 150,
        }
    }

    pub fn kayipsiz_ayarla(&mut self, acik: bool) {
        if self.kayipsiz != acik {
            self.tam_kare_iste();
        }
        self.kayipsiz = acik;
    }

    pub fn terminal_ayarla(&mut self, acik: bool) {
        if self.terminal != acik {
            self.tam_kare_iste();
        }
        self.terminal = acik;
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
    }

    pub fn tam_kare_iste(&mut self) {
        self.onceki = None;
    }

    /// Değişiklik yoksa `None`.
    pub fn kodla(&mut self, g: &Goruntu) -> anyhow::Result<Option<Kare>> {
        anyhow::ensure!(
            g.genislik > 0
                && g.yukseklik > 0
                && g.genislik <= 16384
                && g.yukseklik <= 16384
                && Some(g.rgba.len())
                    == (g.genislik as usize)
                        .checked_mul(g.yukseklik as usize)
                        .and_then(|n| n.checked_mul(4)),
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
                if tam
                    || self.zorunlu.contains(&(x, y))
                    || self
                        .onceki
                        .as_ref()
                        .map_or(true, |o| farkli(o, g, x, y, gen, yuk))
                {
                    konumlar.push((x, y, gen, yuk));
                }
                x += DOSEME;
            }
            y += DOSEME;
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
                    jpeg: if self.kayipsiz {
                        uyarlamali_kodla(g, x, y, gen, yuk, kalite, self.terminal)?
                    } else {
                        jpeg_kodla(g, x, y, gen, yuk, kalite)?
                    },
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.onceki = Some(g.clone());
        self.zorunlu.clear();
        if dosemeler.is_empty() {
            return Ok(None);
        }
        self.sira += 1;
        Ok(Some(Kare {
            sira: self.sira,
            genislik: g.genislik,
            yukseklik: g.yukseklik,
            tam,
            dosemeler,
            yakalama_ms: 0,
        }))
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
    boyut_sira: u64,
    son_sira: u64,
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
            boyut_sira: 0,
            son_sira: 0,
        }
    }
}

/// Yalnız değişen parçalar; tam ekran RGBA kopyalamadan UI'ya taşınabilir.
#[derive(Debug, Clone, PartialEq)]
pub struct DosemeRgba {
    pub x: u32,
    pub y: u32,
    pub gen: u32,
    pub yuk: u32,
    pub rgba: Vec<u8>,
}

impl Birlestirici {
    pub fn parcalar(&mut self, k: &Kare) -> anyhow::Result<Vec<DosemeRgba>> {
        anyhow::ensure!(
            k.genislik > 0 && k.yukseklik > 0 && k.genislik <= 16384 && k.yukseklik <= 16384,
            "geçersiz kare boyutu"
        );
        let degisti = k.genislik != self.goruntu.genislik || k.yukseklik != self.goruntu.yukseklik;
        if k.sira < self.boyut_sira
            || (degisti && k.sira <= self.son_sira && self.goruntu.genislik != 0)
        {
            return Ok(Vec::new());
        }
        let mut parcalar = Vec::new();
        let mut konumlar = std::collections::HashSet::new();
        for d in &k.dosemeler {
            anyhow::ensure!(
                d.gen > 0
                    && d.yuk > 0
                    && d.x.checked_add(d.gen).is_some_and(|v| v <= k.genislik)
                    && d.y.checked_add(d.yuk).is_some_and(|v| v <= k.yukseklik),
                "döşeme ekran dışında"
            );
            anyhow::ensure!(
                d.x % DOSEME == 0
                    && d.y % DOSEME == 0
                    && d.gen == DOSEME.min(k.genislik - d.x)
                    && d.yuk == DOSEME.min(k.yukseklik - d.y)
                    && konumlar.insert((d.x, d.y)),
                "geçersiz veya tekrarlanan döşeme konumu"
            );
            if !degisti
                && self
                    .doseme_sira
                    .get(&(d.x, d.y))
                    .is_some_and(|s| *s >= k.sira)
            {
                continue;
            }
            parcalar.push(DosemeRgba {
                x: d.x,
                y: d.y,
                gen: d.gen,
                yuk: d.yuk,
                rgba: doseme_coz(d)?,
            });
        }
        // Bozuk kare durumunda sıra ve boyut bilgisi değişmez; tekrar gönderim çalışır.
        if degisti {
            if self.goruntu.genislik != 0 {
                self.boyut_sira = k.sira;
            }
            self.goruntu = Goruntu {
                genislik: k.genislik,
                yukseklik: k.yukseklik,
                rgba: Vec::new(),
            };
            self.doseme_sira.clear();
        }
        self.son_sira = self.son_sira.max(k.sira);
        for d in &parcalar {
            self.doseme_sira.insert((d.x, d.y), k.sira);
        }
        Ok(parcalar)
    }

    pub fn uygula(&mut self, k: &Kare) -> anyhow::Result<()> {
        let parcalar = self.parcalar(k)?;
        if parcalar.is_empty() {
            return Ok(());
        }
        let n = self.goruntu.genislik as usize * self.goruntu.yukseklik as usize * 4;
        self.goruntu.rgba.resize(n, 0);
        let satir = self.goruntu.genislik as usize * 4;
        for d in parcalar {
            let gen = d.gen as usize * 4;
            for yy in 0..d.yuk as usize {
                let hedef = (d.y as usize + yy) * satir + d.x as usize * 4;
                self.goruntu.rgba[hedef..hedef + gen]
                    .copy_from_slice(&d.rgba[yy * gen..(yy + 1) * gen]);
            }
        }
        Ok(())
    }
}

// AFDT + version(1) + format(1 palette RLE, 2 RGBA RLE).
// Dimensions remain in Doseme, keeping the legacy bincode layout unchanged.
const MAGIC: &[u8] = b"AFDT";
fn uyarlamali_kodla(
    g: &Goruntu,
    x: u32,
    y: u32,
    gen: u32,
    yuk: u32,
    kalite: u8,
    terminal: bool,
) -> anyhow::Result<Vec<u8>> {
    let mut pixels = Vec::<[u8; 4]>::with_capacity((gen * yuk) as usize);
    let mut palet = Vec::<[u8; 4]>::new();
    let mut indisler = Vec::new();
    let mut harita = std::collections::HashMap::new();
    for yy in y..y + yuk {
        let bas = ((yy * g.genislik + x) * 4) as usize;
        for p in g.rgba[bas..bas + gen as usize * 4].chunks_exact(4) {
            let p: [u8; 4] = p.try_into().unwrap();
            pixels.push(p);
            if palet.len() <= 256 {
                let i = *harita.entry(p).or_insert_with(|| {
                    let i = palet.len();
                    palet.push(p);
                    i
                });
                indisler.push(i as u8);
            }
        }
    }
    if palet.len() > 256 && !terminal {
        return jpeg_kodla(g, x, y, gen, yuk, kalite);
    }
    let palette = palet.len() <= 256;
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&[1, if palette { 1 } else { 2 }]);
    if palette {
        out.extend_from_slice(&(palet.len() as u16).to_le_bytes());
        for p in &palet {
            out.extend_from_slice(p);
        }
    }
    let mut i = 0;
    while i < pixels.len() {
        let mut son = i + 1;
        while son < pixels.len() && pixels[son] == pixels[i] && son - i < u16::MAX as usize {
            son += 1;
        }
        out.extend_from_slice(&((son - i) as u16).to_le_bytes());
        if palette {
            out.push(indisler[i]);
        } else {
            out.extend_from_slice(&pixels[i]);
        }
        i = son;
    }
    Ok(out)
}

/// Saf ve sınırlı çözücü: JPEG veya anlaşılmış AFDT döşemesi → RGBA.
pub fn doseme_coz(d: &Doseme) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        d.gen > 0 && d.yuk > 0 && d.gen <= DOSEME && d.yuk <= DOSEME,
        "geçersiz döşeme boyutu"
    );
    let adet = d.gen as usize * d.yuk as usize;
    anyhow::ensure!(d.jpeg.len() <= 128 * 1024, "döşeme çok büyük");
    if d.jpeg.starts_with(MAGIC) {
        let mut c = std::io::Cursor::new(&d.jpeg[4..]);
        fn al<const N: usize>(c: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<[u8; N]> {
            let mut v = [0; N];
            std::io::Read::read_exact(c, &mut v)?;
            Ok(v)
        }
        let [surum, bicim] = al::<2>(&mut c)?;
        anyhow::ensure!(
            surum == 1 && (bicim == 1 || bicim == 2),
            "bilinmeyen döşeme biçimi"
        );
        let mut palet = Vec::new();
        if bicim == 1 {
            let n = u16::from_le_bytes(al::<2>(&mut c)?) as usize;
            anyhow::ensure!(n > 0 && n <= 256, "geçersiz palet");
            for _ in 0..n {
                palet.push(al::<4>(&mut c)?);
            }
        }
        let mut out = Vec::with_capacity(adet * 4);
        while out.len() < adet * 4 {
            let n = u16::from_le_bytes(al::<2>(&mut c)?) as usize;
            anyhow::ensure!(
                n > 0 && n <= adet - out.len() / 4,
                "geçersiz tekrar uzunluğu"
            );
            let p = if bicim == 1 {
                let i = al::<1>(&mut c)?[0] as usize;
                *palet
                    .get(i)
                    .ok_or_else(|| anyhow::anyhow!("geçersiz palet indisi"))?
            } else {
                al::<4>(&mut c)?
            };
            for _ in 0..n {
                out.extend_from_slice(&p);
            }
        }
        anyhow::ensure!(
            c.position() as usize == d.jpeg.len() - 4,
            "döşemede artık veri"
        );
        return Ok(out);
    }
    let coz = image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(&d.jpeg))?;
    anyhow::ensure!(
        coz.dimensions() == (d.gen, d.yuk),
        "döşeme boyutu uyuşmuyor"
    );
    let rgb = image::DynamicImage::from_decoder(coz)?.to_rgba8();
    Ok(rgb.into_raw())
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
mod hiz_testleri {
    use super::*;
    #[test]
    fn turkce_terminal_renkleri_kayipsiz_ve_parcali() {
        // Türkçe gliflerin kenarlarındaki renk/alpha değerleri birebir korunmalı.
        let mut g = Goruntu {
            genislik: 64,
            yukseklik: 64,
            rgba: [13, 18, 31, 255].repeat(4096),
        };
        for (i, _) in "çğıöşüÇĞİÖŞÜ".chars().enumerate() {
            g.rgba[i * 4..i * 4 + 4].copy_from_slice(&[240, i as u8, 90, 199]);
        }
        let mut e = Kodlayici::new(80);
        e.kayipsiz_ayarla(true);
        e.terminal_ayarla(true);
        let k = e.kodla(&g).unwrap().unwrap();
        assert_eq!(doseme_coz(&k.dosemeler[0]).unwrap(), g.rgba);
        let mut b = Birlestirici::default();
        assert_eq!(b.parcalar(&k).unwrap()[0].rgba, g.rgba);
        assert!(b.parcalar(&k).unwrap().is_empty());
    }
    #[test]
    fn fotograf_jpeg_legacy_de_korunur() {
        let g = deneme_goruntu(64, 64, 42);
        let mut e = Kodlayici::new(90);
        e.kayipsiz_ayarla(true);
        let k = e.kodla(&g).unwrap().unwrap();
        assert!(k.dosemeler[0].jpeg.starts_with(&[255, 216]));
        assert_eq!(doseme_coz(&k.dosemeler[0]).unwrap().len(), 64 * 64 * 4);
        let flat = Goruntu {
            genislik: 1,
            yukseklik: 1,
            rgba: vec![3, 4, 5, 255],
        };
        assert!(
            Kodlayici::new(90).kodla(&flat).unwrap().unwrap().dosemeler[0]
                .jpeg
                .starts_with(&[255, 216])
        );
    }
    #[test]
    fn bozuk_ve_tasan_doseme_reddedilir_ve_sira_zehirlenmez() {
        let g = deneme_goruntu(64, 64, 7);
        let k = Kodlayici::new(90).kodla(&g).unwrap().unwrap();
        let mut bad = k.clone();
        bad.dosemeler[0].jpeg = b"AFDT\x01\x01".to_vec();
        let mut b = Birlestirici::default();
        assert!(b.parcalar(&bad).is_err());
        assert_eq!(b.parcalar(&k).unwrap().len(), 1);
        bad.dosemeler[0].x = u32::MAX;
        assert!(b.parcalar(&bad).is_err());
        bad.dosemeler[0].gen = u32::MAX;
        assert!(doseme_coz(&bad.dosemeler[0]).is_err());
    }
}

#[cfg(test)]
mod bicim_guvenlik_testleri {
    use super::*;
    #[test]
    fn terminal_fotograf_da_rgba_kayipsizdir() {
        let g = deneme_goruntu(64, 64, 19);
        let mut e = Kodlayici::new(80);
        e.kayipsiz_ayarla(true);
        e.terminal_ayarla(true);
        let k = e.kodla(&g).unwrap().unwrap();
        assert_eq!(&k.dosemeler[0].jpeg[..6], b"AFDT\x01\x02");
        assert_eq!(doseme_coz(&k.dosemeler[0]).unwrap(), g.rgba);
    }
    #[test]
    fn bozuk_kayipsiz_akislari_reddet() {
        let mut d = Doseme {
            x: 0,
            y: 0,
            gen: 1,
            yuk: 1,
            jpeg: vec![],
        };
        let veriler: &[&[u8]] = &[
            b"AFDT\x02\x02",                         // version
            b"AFDT\x01\x03",                         // format
            b"AFDT\x01\x01\0\0",                     // empty palette
            b"AFDT\x01\x02\0\0\0\0\0\0",             // zero run
            b"AFDT\x01\x02\x02\0\0\0\0\0",           // run exceeds tile
            b"AFDT\x01\x02\x01\0\0\0\0\0x",          // trailing data
            b"AFDT\x01\x01\x01\0\0\0\0\0\x01\0\x01", // invalid palette index
        ];
        for veri in veriler {
            d.jpeg = veri.to_vec();
            assert!(doseme_coz(&d).is_err(), "{veri:?}");
        }
        d.jpeg = vec![0; 128 * 1024 + 1];
        assert!(doseme_coz(&d).is_err());
    }
    #[test]
    fn jpeg_boyutu_ve_eksik_veri_reddedilir() {
        let k = Kodlayici::new(80)
            .kodla(&deneme_goruntu(64, 64, 9))
            .unwrap()
            .unwrap();
        let mut d = k.dosemeler[0].clone();
        d.gen = 63;
        assert!(doseme_coz(&d).is_err());
        d.gen = 64;
        d.jpeg.truncate(20);
        assert!(doseme_coz(&d).is_err());
    }
}

#[cfg(test)]
mod birlestirme_guvenlik_testleri {
    use super::*;
    fn kare(gen: u32, sira: u64) -> Kare {
        let mut k = Kodlayici::new(80)
            .kodla(&deneme_goruntu(gen, 64, 3))
            .unwrap()
            .unwrap();
        k.sira = sira;
        k
    }
    #[test]
    fn boyut_degisiminden_onceki_bos_doseme_eski_kareyi_canlandirmaz() {
        let mut b = Birlestirici::default();
        b.parcalar(&kare(128, 1)).unwrap();
        b.parcalar(&kare(64, 3)).unwrap();
        let mut eski = kare(64, 2);
        eski.dosemeler[0].x = 0;
        assert!(b.parcalar(&eski).unwrap().is_empty());
        let mut eski_boyut = kare(128, 2);
        eski_boyut.dosemeler.retain(|d| d.x == 64);
        assert!(b.parcalar(&eski_boyut).unwrap().is_empty());
    }
    #[test]
    fn ilk_delta_sonrasi_eksik_eski_doseme_doldurulur() {
        let mut yeni = kare(128, 2);
        yeni.dosemeler.retain(|d| d.x == 64);
        let mut eski = kare(128, 1);
        eski.dosemeler.retain(|d| d.x == 0);
        let mut b = Birlestirici::default();
        assert_eq!(b.parcalar(&yeni).unwrap().len(), 1);
        assert_eq!(b.parcalar(&eski).unwrap().len(), 1);
    }
    #[test]
    fn kaymis_ve_cakisan_dosemeler_reddedilir() {
        let mut k = kare(128, 1);
        k.dosemeler[0].x = 1;
        assert!(Birlestirici::default().parcalar(&k).is_err());
        let mut k = kare(64, 1);
        k.dosemeler.push(k.dosemeler[0].clone());
        assert!(Birlestirici::default().parcalar(&k).is_err());
    }
    #[test]
    fn bos_yeni_boyut_sira_esigini_korur() {
        let mut b = Birlestirici::default();
        b.parcalar(&kare(128, 1)).unwrap();
        let mut yeni = kare(64, 3);
        yeni.dosemeler.clear();
        b.parcalar(&yeni).unwrap();
        assert!(b.parcalar(&kare(128, 2)).unwrap().is_empty());
        assert_eq!(b.goruntu.genislik, 64);
    }
}
