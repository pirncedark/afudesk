//! Metin pano paylaşımı: iki taraf kendi panosunu yoklar, değişeni `Kontrol::Pano` ile
//! gönderir, gelen metni kendi panosuna yazar. Yalnız onay penceresinde izin verildiyse çalışır.
//! Döngü önleme: karşıdan gelip panoya yazılan metin "son görülen" sayılır; bir sonraki
//! yoklamada değişiklik olarak algılanıp geri gönderilmez.
use anyhow::Result;
use std::time::Duration;

/// Tek pano metni için üst sınır (bayt).
pub const AZAMI_PANO: usize = 1024 * 1024;
/// Pano yoklama aralığı.
pub const PANO_ARALIGI: Duration = Duration::from_millis(500);

pub trait Pano: Send {
    /// Panodaki metin; boşsa ya da metin değilse `None`.
    fn oku(&mut self) -> Option<String>;
    fn yaz(&mut self, metin: &str) -> Result<()>;
    fn resim_yaz(&mut self, genislik: usize, yukseklik: usize, rgba: Vec<u8>) -> Result<()>;
}

/// Satır sonu farkları (Windows panosu `\r\n` yazabilir) değişiklik sayılmaz.
fn ayni(a: &str, b: &str) -> bool {
    a == b || a.replace("\r\n", "\n") == b.replace("\r\n", "\n")
}

/// Pano gönderilmeye değer biçimde değişti mi? Boşalma (`None`) değişiklik sayılmaz.
pub fn degisti(onceki: &Option<String>, simdi: &Option<String>) -> bool {
    match (onceki, simdi) {
        (_, None) => false,
        (None, Some(s)) => !s.is_empty(),
        (Some(o), Some(s)) => !s.is_empty() && !ayni(o, s),
    }
}

/// Metin boyut sınırı içinde mi?
pub fn sinir_icinde(metin: &str) -> bool {
    metin.len() <= AZAMI_PANO
}

/// Bir panoyu karşı tarafla eşitler: değişikliği algılar, gelen metni yazar, yankıyı önler.
pub struct Esitleyici {
    pano: Box<dyn Pano>,
    son: Option<String>,
}

impl Esitleyici {
    /// Oturum başındaki pano içeriği gönderilmez; yalnız sonraki değişiklikler gider.
    pub fn new(mut pano: Box<dyn Pano>) -> Self {
        let son = pano.oku();
        Self { pano, son }
    }

    /// Panoyu yoklar; gönderilmesi gereken yeni metin varsa döndürür.
    /// Sınırı aşan metin "görüldü" sayılır ama gönderilmez.
    pub fn yokla(&mut self) -> Option<String> {
        let simdi = self.pano.oku();
        if !degisti(&self.son, &simdi) {
            return None;
        }
        self.son = simdi.clone();
        simdi.filter(|m| sinir_icinde(m))
    }

    /// Karşıdan gelen metni panoya yazar; yazılan metin tekrar gönderilmez.
    pub fn gelen(&mut self, metin: &str) -> Result<()> {
        anyhow::ensure!(
            sinir_icinde(metin),
            "pano metni çok büyük: {} bayt",
            metin.len()
        );
        if metin.is_empty() {
            return Ok(());
        }
        self.pano.yaz(metin)?;
        self.son = Some(metin.to_owned());
        Ok(())
    }

    /// Dosya izniyle alınmış görseli aynı oturumun panosuna koyar.
    /// Boyut sınırı veya desteklenmeyen tür için dosya korunur ve `false` döner.
    pub fn gelen_gorsel(&mut self, yol: &std::path::Path) -> Result<bool> {
        use std::io::{Read, Seek};
        let mut f = std::fs::File::open(yol)?;
        if !crate::dosya::gorsel_pano_sinirinda(f.metadata()?.len()) {
            return Ok(false);
        }
        let mut baslik = [0; 12];
        let n = f.read(&mut baslik)?;
        let Some(bicim) = crate::dosya::gorsel_bicimi(yol, &baslik[..n]) else {
            return Ok(false);
        };
        f.rewind()?;
        // Okuma sırasında büyüyen dosya da sınırsız belleğe alınmaz.
        let mut veri = Vec::new();
        f.by_ref()
            .take(crate::dosya::AZAMI_GORSEL_PANO + 1)
            .read_to_end(&mut veri)?;
        if !crate::dosya::gorsel_pano_sinirinda(veri.len() as u64) {
            return Ok(false);
        }
        let (genislik, yukseklik) =
            image::ImageReader::with_format(std::io::Cursor::new(&veri), bicim)
                .into_dimensions()?;
        anyhow::ensure!(
            genislik <= 16384
                && yukseklik <= 16384
                && u64::from(genislik) * u64::from(yukseklik) * 4
                    <= crate::dosya::AZAMI_GORSEL_PANO * 16,
            "Görselin çözülmüş boyutu çok büyük."
        );
        let mut okuyucu = image::ImageReader::with_format(std::io::Cursor::new(veri), bicim);
        let mut sinir = image::Limits::default();
        sinir.max_image_width = Some(16384);
        sinir.max_image_height = Some(16384);
        sinir.max_alloc = Some(crate::dosya::AZAMI_GORSEL_PANO * 16);
        okuyucu.limits(sinir);
        let resim = okuyucu.decode()?.to_rgba8();
        self.pano.resim_yaz(
            resim.width() as usize,
            resim.height() as usize,
            resim.into_raw(),
        )?;
        self.son = self.pano.oku();
        Ok(true)
    }
}

/// Masaüstü panosu (arboard).
#[cfg(not(target_os = "android"))]
pub struct ArboardPano(arboard::Clipboard);

#[cfg(not(target_os = "android"))]
impl ArboardPano {
    pub fn new() -> Result<Self> {
        Ok(Self(arboard::Clipboard::new()?))
    }
}

#[cfg(not(target_os = "android"))]
impl Pano for ArboardPano {
    fn oku(&mut self) -> Option<String> {
        self.0.get_text().ok()
    }
    fn yaz(&mut self, metin: &str) -> Result<()> {
        self.0.set_text(metin)?;
        Ok(())
    }
    fn resim_yaz(&mut self, genislik: usize, yukseklik: usize, rgba: Vec<u8>) -> Result<()> {
        self.0.set_image(arboard::ImageData {
            width: genislik,
            height: yukseklik,
            bytes: std::borrow::Cow::Owned(rgba),
        })?;
        Ok(())
    }
}

/// Testler için bellekte pano. Klonlar aynı hücreyi paylaşır; test içeriği dışarıdan
/// değiştirebilir ve `yaz` çağrılarını sayabilir.
#[derive(Clone, Default)]
pub struct SahtePano {
    icerik: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    yazilanlar: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    resimler: std::sync::Arc<std::sync::Mutex<Vec<(usize, usize, Vec<u8>)>>>,
}

impl SahtePano {
    pub fn resimler(&self) -> Vec<(usize, usize, Vec<u8>)> {
        self.resimler.lock().unwrap().clone()
    }
    /// Kullanıcı panoya bir şey kopyalamış gibi (yazım sayılmaz).
    pub fn ayarla(&self, metin: &str) {
        *self.icerik.lock().unwrap() = Some(metin.to_owned());
    }
    pub fn icerik(&self) -> Option<String> {
        self.icerik.lock().unwrap().clone()
    }
    /// Karşı taraftan gelip `yaz` ile yazılan metinler.
    pub fn yazilanlar(&self) -> Vec<String> {
        self.yazilanlar.lock().unwrap().clone()
    }
}

impl Pano for SahtePano {
    fn oku(&mut self) -> Option<String> {
        self.icerik()
    }
    fn yaz(&mut self, metin: &str) -> Result<()> {
        self.yazilanlar.lock().unwrap().push(metin.to_owned());
        self.ayarla(metin);
        Ok(())
    }
    fn resim_yaz(&mut self, genislik: usize, yukseklik: usize, rgba: Vec<u8>) -> Result<()> {
        self.resimler
            .lock()
            .unwrap()
            .push((genislik, yukseklik, rgba));
        *self.icerik.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn png() -> Vec<u8> {
        let mut veri = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 1, image::Rgba([12, 34, 56, 255]))
            .write_to(&mut veri, image::ImageFormat::Png)
            .unwrap();
        veri.into_inner()
    }

    #[test]
    fn gelen_gorsel_sahte_panoya_rgba_yazar() {
        let k = tempfile::tempdir().unwrap();
        let yol = k.path().join("ekran.PNG");
        std::fs::write(&yol, png()).unwrap();
        let p = SahtePano::default();
        let mut e = Esitleyici::new(Box::new(p.clone()));
        assert!(e.gelen_gorsel(&yol).unwrap());
        assert_eq!(
            p.resimler(),
            vec![(2, 1, vec![12, 34, 56, 255, 12, 34, 56, 255])]
        );
        assert_eq!(e.yokla(), None);
    }

    #[test]
    fn buyuk_gorsel_kayitli_kalir_panoya_yazilmaz() {
        let k = tempfile::tempdir().unwrap();
        let yol = k.path().join("buyuk.png");
        std::fs::write(&yol, png()).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&yol)
            .unwrap()
            .set_len(crate::dosya::AZAMI_GORSEL_PANO + 1)
            .unwrap();
        let p = SahtePano::default();
        let mut e = Esitleyici::new(Box::new(p.clone()));
        assert!(!e.gelen_gorsel(&yol).unwrap());
        assert!(p.resimler().is_empty());
        assert!(yol.exists());
        assert!(crate::dosya::gorsel_pano_sinirinda(
            crate::dosya::AZAMI_GORSEL_PANO
        ));
    }

    #[test]
    fn sahte_uzanti_ve_bozuk_gorsel_panoya_yazilmaz() {
        let k = tempfile::tempdir().unwrap();
        let yol = k.path().join("sahte.png");
        let p = SahtePano::default();
        let mut e = Esitleyici::new(Box::new(p.clone()));
        std::fs::write(&yol, b"bu bir resim degil").unwrap();
        assert!(!e.gelen_gorsel(&yol).unwrap());
        std::fs::write(&yol, b"\x89PNG\r\n\x1a\n").unwrap();
        assert!(e.gelen_gorsel(&yol).is_err());
        assert!(p.resimler().is_empty());
    }

    fn s(m: &str) -> Option<String> {
        Some(m.to_owned())
    }

    #[test]
    fn degisiklik_algilanir() {
        assert!(degisti(&None, &s("a")), "boştan metne");
        assert!(degisti(&s("a"), &s("b")), "farklı metin");
        assert!(!degisti(&s("a"), &s("a")), "aynı metin");
        assert!(!degisti(&s("a"), &None), "boşalma gönderilmez");
        assert!(!degisti(&None, &None));
        assert!(!degisti(&None, &s("")), "boş metin gönderilmez");
        assert!(
            !degisti(&s("a\nb"), &s("a\r\nb")),
            "satır sonu farkı değişiklik değil"
        );

        // Eşitleyici: başlangıç içeriği gitmez, değişiklik bir kez gider.
        let p = SahtePano::default();
        p.ayarla("eski");
        let mut e = Esitleyici::new(Box::new(p.clone()));
        assert_eq!(e.yokla(), None, "oturum başındaki içerik gönderilmez");
        p.ayarla("yeni");
        assert_eq!(e.yokla(), s("yeni"));
        assert_eq!(e.yokla(), None, "aynı değişiklik iki kez gitmez");
        // Karşıdan gelen yazılır ama geri gönderilmez (yankı yok).
        e.gelen("karşıdan").unwrap();
        assert_eq!(p.icerik(), s("karşıdan"));
        assert_eq!(e.yokla(), None, "gelen metin yankılanmamalı");
        assert_eq!(p.yazilanlar(), vec!["karşıdan".to_owned()]);
        // Sonraki yerel değişiklik yine algılanır.
        p.ayarla("yerel");
        assert_eq!(e.yokla(), s("yerel"));
    }

    #[test]
    fn buyuk_metin_reddedilir() {
        let sinirda = "a".repeat(AZAMI_PANO);
        let buyuk = "a".repeat(AZAMI_PANO + 1);
        assert!(sinir_icinde(&sinirda));
        assert!(!sinir_icinde(&buyuk));
        // Çok baytlı karakterler bayt olarak sayılır.
        assert!(!sinir_icinde(&"ş".repeat(AZAMI_PANO / 2 + 1)));

        let p = SahtePano::default();
        let mut e = Esitleyici::new(Box::new(p.clone()));
        p.ayarla(&buyuk);
        assert_eq!(e.yokla(), None, "büyük metin gönderilmez");
        assert_eq!(e.yokla(), None, "her yoklamada yeniden denenmez");
        p.ayarla("küçük");
        assert_eq!(
            e.yokla(),
            Some("küçük".to_owned()),
            "sonraki küçük metin gider"
        );

        assert!(e.gelen(&buyuk).is_err(), "gelen büyük metin reddedilir");
        assert!(
            p.yazilanlar().is_empty(),
            "reddedilen metin panoya yazılmaz"
        );
        assert!(e.gelen(&sinirda).is_ok());
        assert_eq!(p.yazilanlar().len(), 1);
    }
}
