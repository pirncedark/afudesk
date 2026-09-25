//! Dosya aktarımı (izleyici → host), kaldığı yerden devam eder.
//!
//! Akış ayrımı: görüntü kareleri host → izleyici TEK YÖNLÜ akışlarda gider; kontrol
//! kanalı oturumun ilk çift yönlü akışıdır. Dosyalar izleyicinin açtığı YENİ çift
//! yönlü akışlarda gider (dosya başına bir akış). İzleyicinin `accept_uni` döngüsü
//! bu akışları hiç görmez; host da kontrol akışından sonra gelen her çift yönlü
//! akışı dosya akışı sayar ve ilk mesajın `DosyaBaslik` olmasını şart koşar.
//! Böylece ayrı bir tür baytına gerek kalmaz; çift yönlü akış ayrıca host'un
//! devam noktasını ve sonucu aynı akıştan bildirmesini sağlar.
//!
//! Sıra: `DosyaBaslik` → `DosyaDevam { baslangic }` (ya da `DosyaHata`) →
//! `DosyaParca`… → akış sonu → `DosyaTamam` (SHA-256 doğru) ya da `DosyaHata`.
use crate::protokol::{self, Kontrol};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

/// Tek parçanın azami boyu.
pub const PARCA: usize = 256 * 1024;
/// Yarım dosyaların uzantısı: `<kimlik-hex>.afudesk-parca`.
pub const YARIM_UZANTI: &str = "afudesk-parca";
pub const IZIN_YOK: &str = "Karşı taraf dosya almaya izin vermedi.";
pub const YARIDA_KALDI: &str =
    "Aktarım yarıda kesildi; yeniden gönderince kaldığı yerden devam eder.";
pub const BOZUK: &str = "Dosya bozuk geldi (SHA-256 uyuşmuyor); yeniden gönder.";
/// Başlık/devam yanıtı ve parçalar arası azami bekleme.
const YANIT_SURESI: Duration = Duration::from_secs(20);
/// Son parçadan sonra host'un sonucu bildirmesi için azami bekleme.
const SONUC_SURESI: Duration = Duration::from_secs(60);
const AZAMI_AD: usize = 200;

// --- saf yardımcılar ---

fn ayrilmis_mi(kok: &str) -> bool {
    let k = kok.trim_end_matches([' ', '.']).to_ascii_uppercase();
    matches!(k.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((k.starts_with("COM") || k.starts_with("LPT"))
            && k.len() == 4
            && matches!(k.as_bytes()[3], b'1'..=b'9'))
}

/// Karşıdan gelen adı bu makinede güvenle kullanılabilir tek bir dosya adına çevirir:
/// yol kısımları atılır (yalnız son parça kalır), `..`/`.` düşer, denetim karakterleri
/// silinir, Windows'ta yasak karakterler `_` olur, sondaki nokta/boşluk kırpılır,
/// CON/NUL/COM1… gibi ayrılmış adların başına `_` eklenir, uzunluk sınırlanır.
/// Hiçbir şey kalmazsa "dosya".
pub fn guvenli_ad(ad: &str) -> String {
    let son = ad.rsplit(['/', '\\']).next().unwrap_or("");
    let mut s: String = son
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    s = s.trim().trim_end_matches(['.', ' ']).to_owned();
    if s.chars().all(|c| c == '.') {
        s.clear();
    }
    if s.is_empty() {
        return "dosya".into();
    }
    let kok = s.split('.').next().unwrap_or("");
    if ayrilmis_mi(kok) {
        s.insert(0, '_');
    }
    if s.chars().count() > AZAMI_AD {
        let (govde, uzanti) = bol(&s);
        let uzanti: String = if uzanti.chars().count() <= 20 {
            uzanti.to_owned()
        } else {
            String::new()
        };
        let kalan = AZAMI_AD - uzanti.chars().count();
        s = govde.chars().take(kalan).collect::<String>() + &uzanti;
    }
    s
}

/// "rapor.pdf" → ("rapor", ".pdf"); ".bashrc" ve "README" uzantısız sayılır.
fn bol(ad: &str) -> (&str, &str) {
    match ad.rfind('.') {
        Some(i) if i > 0 => (&ad[..i], &ad[i..]),
        _ => (ad, ""),
    }
}

/// `ad` doluysa "ad (1).uzantı", "ad (2).uzantı"… ilk boş olanı döner.
pub fn cakismasiz_ad(ad: &str, var_mi: impl Fn(&str) -> bool) -> String {
    if !var_mi(ad) {
        return ad.to_owned();
    }
    let (govde, uzanti) = bol(ad);
    (1u32..)
        .map(|n| format!("{govde} ({n}){uzanti}"))
        .find(|a| !var_mi(a))
        .expect("sonsuz aralık")
}

/// Yarım dosyanın boyuna göre devam noktası. Yarım dosya beklenenden büyükse
/// (başka bir dosyaya ait olamaz ama bozulmuş olabilir) baştan başlanır.
pub fn devam_noktasi(yarim_boyut: Option<u64>, toplam: u64) -> u64 {
    match yarim_boyut {
        Some(n) if n <= toplam => n,
        _ => 0,
    }
}

/// Aktarım kimliği içerikten türetilir (ad + boyut + SHA-256): aynı dosya yeniden
/// gönderildiğinde — uygulama yeniden başlasa, yeni oturum açılsa bile — host yarım
/// dosyayı bulur. Rastgele kimlik bunun için izleyicide kalıcı durum gerektirirdi.
pub fn kimlik_uret(ad: &str, boyut: u64, sha256: &[u8; 32]) -> [u8; 16] {
    let mut h = Sha256::new();
    h.update(b"afudesk-dosya/1");
    h.update(sha256);
    h.update(boyut.to_be_bytes());
    h.update(ad.as_bytes());
    let o = h.finalize();
    let mut k = [0u8; 16];
    k.copy_from_slice(&o[..16]);
    k
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn yarim_yol(klasor: &Path, kimlik: &[u8; 16]) -> PathBuf {
    klasor.join(format!("{}.{YARIM_UZANTI}", hex(kimlik)))
}

/// Alınan dosyaların varsayılan klasörü: İndirilenler\AfuDesk.
pub fn varsayilan_klasor() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(std::env::temp_dir)
        .join("AfuDesk")
}

/// Dosyanın boyu ve SHA-256'sı (engelleyici; `spawn_blocking` içinde çağır).
fn ozet(yol: &Path, sinir: Option<u64>) -> std::io::Result<(u64, Sha256)> {
    use std::io::Read;
    let mut f = std::fs::File::open(yol)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; PARCA];
    let mut toplam = 0u64;
    loop {
        let istek = match sinir {
            Some(s) => ((s - toplam) as usize).min(buf.len()),
            None => buf.len(),
        };
        if istek == 0 {
            break;
        }
        let n = f.read(&mut buf[..istek])?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        toplam += n as u64;
    }
    Ok((toplam, h))
}

// --- host tarafı: alma ---

/// Bir oturumdaki dosya alma ayarı; akışlar arasında paylaşılır.
pub struct Alici {
    pub klasor: PathBuf,
    pub izin: bool,
    /// Aynı dosyanın iki akıştan aynı yarım dosyaya yazılmasını önler.
    surenler: Mutex<HashSet<[u8; 16]>>,
}

impl Alici {
    pub fn new(klasor: PathBuf, izin: bool) -> Arc<Self> {
        Arc::new(Self {
            klasor,
            izin,
            surenler: Default::default(),
        })
    }
}

struct SurenKaydi<'a>(&'a Alici, [u8; 16]);

impl Drop for SurenKaydi<'_> {
    fn drop(&mut self) {
        self.0.surenler.lock().unwrap().remove(&self.1);
    }
}

/// Kullanıcıya gösterilecek hata (akışa `DosyaHata` olarak yazılır).
struct Ret(String);

impl<E: std::fmt::Display> From<E> for Ret {
    fn from(e: E) -> Self {
        Ret(format!("Dosya kaydedilemedi: {e}"))
    }
}

fn ret(m: &str) -> Ret {
    Ret(m.to_owned())
}

/// Tek bir dosya akışını işler. Başarılıysa (gösterilen ad, son yol) döner.
pub async fn al(
    alici: &Alici,
    mut w: crate::ag::GonderAkisi,
    mut r: crate::ag::AlAkisi,
) -> Option<(String, PathBuf)> {
    let sonuc = al_ic(alici, &mut w, &mut r).await;
    let yanit = match &sonuc {
        Ok(_) => Kontrol::DosyaTamam,
        Err(Ret(m)) => Kontrol::DosyaHata(m.clone()),
    };
    let _ = protokol::yaz(&mut w, &yanit).await;
    let _ = w.finish();
    // Yanıt karşıya ulaşana kadar bekle (akış düşerse veri de düşebilir).
    let _ = tokio::time::timeout(Duration::from_secs(5), w.stopped()).await;
    sonuc.ok()
}

async fn al_ic(
    alici: &Alici,
    w: &mut crate::ag::GonderAkisi,
    r: &mut crate::ag::AlAkisi,
) -> Result<(String, PathBuf), Ret> {
    let baslik = tokio::time::timeout(YANIT_SURESI, protokol::oku::<_, Kontrol>(r)).await;
    let Ok(Ok(Some(Kontrol::DosyaBaslik {
        kimlik,
        ad,
        boyut,
        sha256,
    }))) = baslik
    else {
        return Err(ret("Geçersiz dosya başlığı."));
    };
    if !alici.izin {
        return Err(ret(IZIN_YOK));
    }
    if !alici.surenler.lock().unwrap().insert(kimlik) {
        return Err(ret("Bu dosya zaten aktarılıyor."));
    }
    let _kayit = SurenKaydi(alici, kimlik);
    let ad = guvenli_ad(&ad);
    tokio::fs::create_dir_all(&alici.klasor).await?;
    let yarim = yarim_yol(&alici.klasor, &kimlik);
    let mevcut = tokio::fs::metadata(&yarim)
        .await
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len());
    let baslangic = devam_noktasi(mevcut, boyut);

    // Yarım dosyadaki önek zaten doğru sırayla yazılmıştır: özetine oradan devam edilir.
    let (mut ozet_h, mut dosya) = if baslangic > 0 {
        let y = yarim.clone();
        let (okunan, h) = tokio::task::spawn_blocking(move || ozet(&y, Some(baslangic))).await??;
        if okunan != baslangic {
            return Err(ret("Yarım dosya okunamadı."));
        }
        let mut f = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&yarim)
            .await?;
        f.set_len(baslangic).await?;
        f.seek(std::io::SeekFrom::Start(baslangic)).await?;
        (h, f)
    } else {
        (Sha256::new(), tokio::fs::File::create(&yarim).await?)
    };
    if protokol::yaz(w, &Kontrol::DosyaDevam { baslangic })
        .await
        .is_err()
    {
        return Err(ret(YARIDA_KALDI));
    }

    let mut alinan = baslangic;
    let kesildi = loop {
        if alinan == boyut {
            break None;
        }
        match tokio::time::timeout(YANIT_SURESI, protokol::oku::<_, Kontrol>(r)).await {
            Ok(Ok(Some(Kontrol::DosyaParca(p)))) => {
                if alinan + p.len() as u64 > boyut {
                    break Some(ret("Dosya bildirilen boyuttan büyük geldi."));
                }
                dosya.write_all(&p).await?;
                ozet_h.update(&p);
                alinan += p.len() as u64;
            }
            Ok(Ok(Some(_))) => break Some(ret("Dosya akışında beklenmeyen mesaj.")),
            // Akış bitti/koptu/zaman aşımı: yazılan kısım kalır, sonra devam edilir.
            _ => break Some(ret(YARIDA_KALDI)),
        }
    };
    dosya.flush().await?;
    drop(dosya); // Windows'ta açık dosya taşınamaz.
    if let Some(e) = kesildi {
        return Err(e);
    }
    if ozet_h.finalize().as_slice() != sha256 {
        let _ = tokio::fs::remove_file(&yarim).await;
        return Err(ret(BOZUK));
    }
    let klasor = alici.klasor.clone();
    let son_ad = cakismasiz_ad(&ad, |a| klasor.join(a).exists());
    let hedef = alici.klasor.join(&son_ad);
    tokio::fs::rename(&yarim, &hedef).await?;
    Ok((son_ad, hedef))
}

// --- izleyici tarafı: gönderme ---

/// Bir gönderimin sonucu.
#[derive(Debug, Clone, PartialEq)]
pub struct Gonderim {
    pub ad: String,
    pub toplam: u64,
    /// Host'un bildirdiği devam noktası.
    pub baslangic: u64,
    /// Bu denemede gerçekten gönderilen dosya baytı.
    pub gonderilen: u64,
}

/// Test kancaları: gerçek kullanımda hep varsayılan.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GonderAyari {
    /// Bu kadar bayttan sonra akışı kapat (yarıda kesilme benzetimi).
    pub kes: Option<u64>,
    /// Başlıktaki SHA-256'yı boz.
    pub sha_boz: bool,
}

pub(crate) async fn gonder(
    c: &crate::ag::Baglanti,
    yol: &Path,
    ayar: GonderAyari,
    mut ilerleme: impl FnMut(u64, u64),
) -> Result<Gonderim> {
    let ad = yol
        .file_name()
        .map(|a| a.to_string_lossy().into_owned())
        .context("Dosya adı okunamadı.")?;
    let y = yol.to_owned();
    let (boyut, h) = tokio::task::spawn_blocking(move || ozet(&y, None))
        .await?
        .with_context(|| format!("Dosya okunamadı: {}", yol.display()))?;
    let mut sha256: [u8; 32] = h.finalize().into();
    if ayar.sha_boz {
        sha256[0] ^= 0xff;
    }
    let kimlik = kimlik_uret(&ad, boyut, &sha256);
    let (mut w, mut r) = c.open_bi().await.context("Bağlantı koptu.")?;
    protokol::yaz(
        &mut w,
        &Kontrol::DosyaBaslik {
            kimlik,
            ad: ad.clone(),
            boyut,
            sha256,
        },
    )
    .await?;
    let baslangic =
        match tokio::time::timeout(YANIT_SURESI, protokol::oku::<_, Kontrol>(&mut r)).await {
            Ok(Ok(Some(Kontrol::DosyaDevam { baslangic }))) if baslangic <= boyut => baslangic,
            Ok(Ok(Some(Kontrol::DosyaHata(m)))) => bail!(m),
            Err(_) => bail!("Karşı taraf dosya aktarımını desteklemiyor ya da yanıt vermedi."),
            _ => bail!("Dosya aktarımı başlatılamadı."),
        };
    let mut f = tokio::fs::File::open(yol).await?;
    f.seek(std::io::SeekFrom::Start(baslangic)).await?;
    let mut konum = baslangic;
    ilerleme(konum, boyut);
    let hedef = ayar.kes.map_or(boyut, |k| k.min(boyut));
    let mut buf = vec![0u8; PARCA];
    while konum < hedef {
        let istek = ((hedef - konum) as usize).min(PARCA);
        let n = f.read(&mut buf[..istek]).await?;
        if n == 0 {
            bail!("Dosya gönderilirken değişti.");
        }
        protokol::yaz(&mut w, &Kontrol::DosyaParca(buf[..n].to_vec()))
            .await
            .context("Bağlantı koptu.")?;
        konum += n as u64;
        ilerleme(konum, boyut);
    }
    let _ = w.finish();
    match tokio::time::timeout(SONUC_SURESI, protokol::oku::<_, Kontrol>(&mut r)).await {
        Ok(Ok(Some(Kontrol::DosyaTamam))) => Ok(Gonderim {
            ad,
            toplam: boyut,
            baslangic,
            gonderilen: konum - baslangic,
        }),
        Ok(Ok(Some(Kontrol::DosyaHata(m)))) => bail!(m),
        _ => bail!("{YARIDA_KALDI}"),
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn guvenli_ad_temizler() {
        assert_eq!(guvenli_ad("rapor.pdf"), "rapor.pdf");
        assert_eq!(guvenli_ad("şükrü çiğdem.txt"), "şükrü çiğdem.txt");
        // Yol ayırıcıları: yalnız son parça kalır.
        assert_eq!(guvenli_ad("../../Windows/System32/evil.dll"), "evil.dll");
        assert_eq!(guvenli_ad("..\\..\\a.txt"), "a.txt");
        assert_eq!(guvenli_ad("C:\\Users\\x\\b.txt"), "b.txt");
        assert_eq!(guvenli_ad("/etc/passwd"), "passwd");
        // Yalnız nokta/ayırıcı: boş kalır → "dosya".
        assert_eq!(guvenli_ad(".."), "dosya");
        assert_eq!(guvenli_ad("a/.."), "dosya");
        assert_eq!(guvenli_ad("..."), "dosya");
        assert_eq!(guvenli_ad(""), "dosya");
        assert_eq!(guvenli_ad("   "), "dosya");
        assert_eq!(guvenli_ad("klasor/"), "dosya");
        // Windows'ta yasak karakterler ve denetim karakterleri.
        assert_eq!(guvenli_ad("a<b>c:d\"e|f?g*.txt"), "a_b_c_d_e_f_g_.txt");
        assert_eq!(guvenli_ad("sat\u{0}ır\n.txt"), "satır.txt");
        // Sondaki nokta ve boşluklar (Windows sessizce atar).
        assert_eq!(guvenli_ad("belge.txt. . "), "belge.txt");
        // Ayrılmış aygıt adları, uzantılı ve küçük harfli halleri de.
        assert_eq!(guvenli_ad("CON"), "_CON");
        assert_eq!(guvenli_ad("nul.txt"), "_nul.txt");
        assert_eq!(guvenli_ad("Com1.log"), "_Com1.log");
        assert_eq!(guvenli_ad("LPT9"), "_LPT9");
        assert_eq!(guvenli_ad("aux.tar.gz"), "_aux.tar.gz");
        // Ayrılmış olmayan benzerleri dokunulmaz.
        assert_eq!(guvenli_ad("COM10.txt"), "COM10.txt");
        assert_eq!(guvenli_ad("CONSOLE.txt"), "CONSOLE.txt");
        assert_eq!(guvenli_ad("COM0"), "COM0");
        // Gizli dosya adı korunur.
        assert_eq!(guvenli_ad(".bashrc"), ".bashrc");
        // Uzun ad kısaltılır, uzantı korunur.
        let uzun = format!("{}.pdf", "a".repeat(500));
        let g = guvenli_ad(&uzun);
        assert_eq!(g.chars().count(), 200);
        assert!(g.ends_with(".pdf"));
    }

    #[test]
    fn cakismada_numara_eklenir() {
        fn var(liste: &'static [&'static str]) -> impl Fn(&str) -> bool {
            move |a| liste.contains(&a)
        }
        assert_eq!(cakismasiz_ad("rapor.pdf", var(&[])), "rapor.pdf");
        assert_eq!(
            cakismasiz_ad("rapor.pdf", var(&["rapor.pdf"])),
            "rapor (1).pdf"
        );
        assert_eq!(
            cakismasiz_ad(
                "rapor.pdf",
                var(&["rapor.pdf", "rapor (1).pdf", "rapor (2).pdf"])
            ),
            "rapor (3).pdf"
        );
        assert_eq!(cakismasiz_ad("README", var(&["README"])), "README (1)");
        assert_eq!(cakismasiz_ad(".bashrc", var(&[".bashrc"])), ".bashrc (1)");
        assert_eq!(
            cakismasiz_ad("arsiv.tar.gz", var(&["arsiv.tar.gz"])),
            "arsiv.tar (1).gz"
        );
        // Gerçek dosya sistemiyle.
        let k = std::env::temp_dir().join(format!("afudesk_cakisma_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&k).unwrap();
        std::fs::write(k.join("a.txt"), b"1").unwrap();
        assert_eq!(cakismasiz_ad("a.txt", |a| k.join(a).exists()), "a (1).txt");
        std::fs::remove_dir_all(&k).unwrap();
    }

    #[test]
    fn devam_noktasi_hesabi() {
        assert_eq!(devam_noktasi(None, 100), 0);
        assert_eq!(devam_noktasi(Some(0), 100), 0);
        assert_eq!(devam_noktasi(Some(40), 100), 40);
        assert_eq!(devam_noktasi(Some(100), 100), 100);
        assert_eq!(
            devam_noktasi(Some(101), 100),
            0,
            "fazla uzun yarım dosya: baştan"
        );
    }

    #[test]
    fn kimlik_icerige_bagli_ve_kararli() {
        let a = kimlik_uret("a.txt", 3, &[1; 32]);
        assert_eq!(a, kimlik_uret("a.txt", 3, &[1; 32]));
        assert_ne!(a, kimlik_uret("b.txt", 3, &[1; 32]));
        assert_ne!(a, kimlik_uret("a.txt", 4, &[1; 32]));
        assert_ne!(a, kimlik_uret("a.txt", 3, &[2; 32]));
        let y = yarim_yol(Path::new("k"), &a);
        assert!(y.to_string_lossy().ends_with(".afudesk-parca"));
    }

    #[test]
    fn varsayilan_klasor_afudesk_ile_biter() {
        assert!(varsayilan_klasor().ends_with("AfuDesk"));
    }
}
