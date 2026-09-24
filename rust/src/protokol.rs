//! Tel protokolü: QUIC akışları üzerinde uzunluk önekli (u32 BE) bincode mesajları.
//! Kontrol akışı (çift yönlü): Merhaba/Kabul/Red/Girdi/Pano/Kapat.
//! Görüntü akışı (tek yönlü, host → izleyici): Kare.
//! Dosya akışı (çift yönlü, izleyici açar; dosya başına bir akış):
//! DosyaBaslik → DosyaDevam | DosyaHata, sonra DosyaParca… ve DosyaTamam | DosyaHata.
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const ALPN: &[u8] = b"afudesk/1";
pub const SURUM: u32 = 1;
/// Tek mesaj için üst sınır (kötü niyetli uzunluk alanına karşı).
pub const AZAMI_MESAJ: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FareTusu {
    Sol,
    Sag,
    Orta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Girdi {
    /// Ekran boyutundan bağımsız, 0.0..=1.0 aralığında konum.
    FareKonum { x: f32, y: f32 },
    FareTus { tus: FareTusu, basili: bool },
    /// Satır cinsinden kaydırma; pozitif = aşağı / sağ.
    Kaydir { dx: i32, dy: i32 },
    /// Adlandırılmış tuş ("Enter", "Backspace", "Ctrl", "a", "ş" ...).
    Tus { ad: String, basili: bool },
    /// Doğrudan metin yazma (mobil klavye için).
    Metin(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Izinler {
    pub kontrol: bool,
    pub pano: bool,
    /// İzleyiciden dosya alma. Not: bincode kendini tanımlamadığı için `default`
    /// yalnız kendini tanımlayan biçimlerde (JSON vb.) işe yarar.
    #[serde(default)]
    pub dosya: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Kontrol {
    /// İzleyici → host, ilk mesaj.
    Merhaba { surum: u32, bilet: String, ad: String },
    /// Host → izleyici.
    Kabul { izinler: Izinler, genislik: u32, yukseklik: u32 },
    Red(String),
    Girdi(Girdi),
    Pano(String),
    Kapat(String),
    /// İzleyici → host, dosya akışının ilk mesajı.
    DosyaBaslik { kimlik: [u8; 16], ad: String, boyut: u64, sha256: [u8; 32] },
    /// Host → izleyici: bu bayttan itibaren gönder (yarım dosya varsa > 0).
    DosyaDevam { baslangic: u64 },
    /// İzleyici → host: sıradaki dosya parçası (en çok `dosya::PARCA` bayt).
    DosyaParca(Vec<u8>),
    /// Host → izleyici: dosya eksiksiz geldi, SHA-256 doğru, yerine taşındı.
    DosyaTamam,
    DosyaHata(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Doseme {
    pub x: u32,
    pub y: u32,
    pub gen: u32,
    pub yuk: u32,
    /// JPEG baytları.
    pub jpeg: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kare {
    pub sira: u64,
    pub genislik: u32,
    pub yukseklik: u32,
    /// true: tüm ekran döşemeleri var (anahtar kare).
    pub tam: bool,
    pub dosemeler: Vec<Doseme>,
}

pub async fn yaz<W: AsyncWriteExt + Unpin, T: Serialize>(w: &mut W, m: &T) -> anyhow::Result<()> {
    let b = bincode::serialize(m)?;
    anyhow::ensure!(b.len() <= AZAMI_MESAJ, "mesaj çok büyük: {}", b.len());
    w.write_all(&(b.len() as u32).to_be_bytes()).await?;
    w.write_all(&b).await?;
    Ok(())
}

/// Akış düzgün kapandıysa `None`.
pub async fn oku<R: AsyncReadExt + Unpin, T: for<'de> Deserialize<'de>>(
    r: &mut R,
) -> anyhow::Result<Option<T>> {
    let mut u = [0u8; 4];
    match r.read_exact(&mut u).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let n = u32::from_be_bytes(u) as usize;
    anyhow::ensure!(n <= AZAMI_MESAJ, "mesaj uzunluğu sınırı aşıyor: {n}");
    let mut b = vec![0u8; n];
    r.read_exact(&mut b).await?;
    Ok(Some(bincode::deserialize(&b)?))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[tokio::test]
    async fn yaz_oku_gidis_donus() {
        let (mut a, mut b) = tokio::io::duplex(1 << 20);
        let m = Kontrol::Girdi(Girdi::Tus { ad: "ş".into(), basili: true });
        yaz(&mut a, &m).await.unwrap();
        yaz(&mut a, &Kontrol::Kapat("bitti".into())).await.unwrap();
        drop(a);
        assert_eq!(oku::<_, Kontrol>(&mut b).await.unwrap(), Some(m));
        assert_eq!(oku::<_, Kontrol>(&mut b).await.unwrap(), Some(Kontrol::Kapat("bitti".into())));
        assert_eq!(oku::<_, Kontrol>(&mut b).await.unwrap(), None);
    }

    #[tokio::test]
    async fn dev_uzunluk_reddedilir() {
        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&(u32::MAX).to_be_bytes()).await.unwrap();
        assert!(oku::<_, Kontrol>(&mut b).await.is_err());
    }

    #[tokio::test]
    async fn cop_veri_reddedilir() {
        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&4u32.to_be_bytes()).await.unwrap();
        a.write_all(&[0xff, 0xff, 0xff, 0xff]).await.unwrap();
        assert!(oku::<_, Kontrol>(&mut b).await.is_err());
    }
}
