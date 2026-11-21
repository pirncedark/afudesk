//! AfuDesk Oyun Modu cekirdek modulu.
//!
//! Moonlight Qt ve Sunshine GPL-3.0 lisansli oldugu icin kutuphane olarak
//! gomulmez; yalniz harici surec olarak kabuksuz calistirilir.

pub mod moonlight;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MOONLIGHT_RESMI_INDIRME_URL: &str = "https://moonlight-stream.org";
pub const MOONLIGHT_RELEASES_URL: &str =
    "https://github.com/moonlight-stream/moonlight-qt/releases";
pub const SUNSHINE_RESMI_INDIRME_URL: &str = "https://sunshine.lizardbyte.dev/";

pub const FPS_MIN: u32 = 10;
pub const FPS_MAX: u32 = 480;
pub const BITRATE_KBPS_MIN: u32 = 500;
pub const BITRATE_KBPS_MAX: u32 = 500_000;

/// Oyun akisi cozunurluk profili.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cozunurluk {
    Hd720,
    Fhd1080,
    Qhd1440,
    Uhd4k,
    Ozel { genislik: u32, yukseklik: u32 },
}

impl Cozunurluk {
    pub fn cli_argumanlari(self) -> Vec<String> {
        match self {
            Self::Hd720 => vec!["--720".to_string()],
            Self::Fhd1080 => vec!["--1080".to_string()],
            Self::Qhd1440 => vec!["--1440".to_string()],
            Self::Uhd4k => vec!["--4K".to_string()],
            Self::Ozel {
                genislik,
                yukseklik,
            } => vec![
                "--resolution".to_string(),
                format!("{genislik}x{yukseklik}"),
            ],
        }
    }
}

/// Video kodlayici / kod cozucu secimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Codec {
    Auto,
    H264,
    Hevc,
    Av1,
}

impl Codec {
    pub fn cli_degeri(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::H264 => "H.264",
            Self::Hevc => "HEVC",
            Self::Av1 => "AV1",
        }
    }
}

/// Goruntu pencere modu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EkranModu {
    Fullscreen,
    Windowed,
    Borderless,
}

impl EkranModu {
    pub fn cli_degeri(self) -> &'static str {
        match self {
            Self::Fullscreen => "fullscreen",
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
        }
    }
}

/// Moonlight akis profili (varsayilan: 1080p60, otomatik codec, tam ekran, coklu kumanda).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OyunAyar {
    pub cozunurluk: Cozunurluk,
    pub fps: u32,
    pub bitrate_kbps: Option<u32>,
    pub codec: Codec,
    pub ekran_modu: EkranModu,
    pub coklu_kumanda: bool,
}

impl OyunAyar {
    /// Varsayilan profil: 1080p, 60 FPS, otomatik bitrate ve codec, tam ekran, coklu kumanda acik.
    pub fn varsayilan() -> Self {
        Self {
            cozunurluk: Cozunurluk::Fhd1080,
            fps: 60,
            bitrate_kbps: None,
            codec: Codec::Auto,
            ekran_modu: EkranModu::Fullscreen,
            coklu_kumanda: true,
        }
    }

    /// Ayar araliklarini Moonlight CLI calistirilmadan once dogrular.
    pub fn dogrula(&self) -> Result<(), OyunHata> {
        if !(FPS_MIN..=FPS_MAX).contains(&self.fps) {
            return Err(OyunHata::GecersizGirdi(format!(
                "FPS {FPS_MIN}-{FPS_MAX} araliginda olmalidir (verilen: {})",
                self.fps
            )));
        }
        if let Some(kbps) = self.bitrate_kbps {
            if !(BITRATE_KBPS_MIN..=BITRATE_KBPS_MAX).contains(&kbps) {
                return Err(OyunHata::GecersizGirdi(format!(
                    "Bitrate {BITRATE_KBPS_MIN}-{BITRATE_KBPS_MAX} Kbps araliginda olmalidir (verilen: {kbps})"
                )));
            }
        }
        if let Cozunurluk::Ozel {
            genislik,
            yukseklik,
        } = self.cozunurluk
        {
            if !(320..=15360).contains(&genislik) || !(240..=8640).contains(&yukseklik) {
                return Err(OyunHata::GecersizGirdi(format!(
                    "Gecersiz ozel cozunurluk: {genislik}x{yukseklik}"
                )));
            }
        }
        Ok(())
    }
}

impl Default for OyunAyar {
    fn default() -> Self {
        Self::varsayilan()
    }
}

/// Harici Moonlight oturumunun yasam dongusu durumu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OyunDurum {
    Bosta,
    Baglaniyor,
    Calisiyor,
    Sonlandi { cikis_kodu: Option<i32> },
}

/// Kesfedilen veya kayitli oyun cihazi ozeti.
/// Gecikme ve FPS alanlari yalniz gercekten olculduyse doldurulur; uydurma deger atanmaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OyunCihazi {
    pub ad: String,
    pub adres: String,
    pub eslesmis: bool,
    pub gecikme_ms: Option<u32>,
    pub olculen_fps: Option<u32>,
}

/// Oyun Modu acik hata turleri.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OyunHata {
    #[error("Oyun akisi icin Moonlight gerekli. Indirmek icin dokun.")]
    MoonlightEksik { indirme_url: String },

    #[error("Oyun baglantisi ayarlari gecersiz: {0}")]
    GecersizGirdi(String),

    #[error("Sunshine yonetim paneli dis aga acik gorunuyor. Yalniz bu bilgisayara izin verin.")]
    YonetimPaneliDisaAcik,

    #[error("Moonlight baslatilamadi. Kurulumu kontrol edin: {0}")]
    SurecBaslatilamadi(String),

    #[error("Moonlight oturumu kapatilamadi. Yeniden deneyin: {0}")]
    SurecSonlandirilamadi(String),

    #[error("Sunshine yapilandirmasi okunamadi. Kurulumu kontrol edin: {0}")]
    YapilandirmaOkunamadi(String),
}
