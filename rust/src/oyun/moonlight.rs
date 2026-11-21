//! Moonlight Qt harici surec yoneticisi ve Sunshine 47990 salt-okur denetleyicisi.
//!
//! GPL-3.0 lisans uyumu geregi Moonlight Qt kutuphane olarak baglanmaz; yalniz
//! kabuksuz std::process::Command ile harici ikili olarak calistirilir.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::{OyunAyar, OyunDurum, OyunHata, MOONLIGHT_RESMI_INDIRME_URL};

const YASAK_KABUKLAR: &[&str] = &[
    "cmd",
    "cmd.exe",
    "powershell",
    "powershell.exe",
    "pwsh",
    "pwsh.exe",
    "sh",
    "bash",
    "zsh",
    "dash",
    "fish",
    "wsl",
    "wsl.exe",
    "mshta",
    "mshta.exe",
    "wscript",
    "wscript.exe",
    "cscript",
    "cscript.exe",
];

/// Harici komut calistirma ciktisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KomutCikti {
    pub cikis_kodu: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Calisan harici Moonlight sureci tutamagi.
#[derive(Debug)]
pub struct MoonlightSurec {
    cocuk: Option<Child>,
    durum: OyunDurum,
    son_cikis_kodu: Option<i32>,
}

fn host_dogrula(host: &str) -> Result<&str, OyunHata> {
    if host.is_empty() || host.trim().is_empty() {
        return Err(OyunHata::GecersizGirdi(
            "Hedef host adresi bos olamaz".to_string(),
        ));
    }
    if host.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(OyunHata::GecersizGirdi(
            "Hedef host adresi gecersiz karakter iceriyor".to_string(),
        ));
    }
    if host.starts_with('-') {
        return Err(OyunHata::GecersizGirdi(
            "Hedef host adresi '-' ile baslayamaz".to_string(),
        ));
    }
    Ok(host)
}

fn uygulama_dogrula(uygulama: &str) -> Result<&str, OyunHata> {
    if uygulama.is_empty() || uygulama.trim().is_empty() {
        return Err(OyunHata::GecersizGirdi(
            "Uygulama adi bos olamaz".to_string(),
        ));
    }
    if uygulama.chars().any(char::is_control) {
        return Err(OyunHata::GecersizGirdi(
            "Uygulama adi kontrol karakteri iceremez".to_string(),
        ));
    }
    Ok(uygulama)
}

fn pin_dogrula(pin: &str) -> Result<&str, OyunHata> {
    if pin.len() != 4 || !pin.bytes().all(|b| b.is_ascii_digit()) {
        return Err(OyunHata::GecersizGirdi(
            "PIN tam 4 rakam olmalidir".to_string(),
        ));
    }
    Ok(pin)
}

fn ikili_dogrula(ikili: &Path) -> Result<(), OyunHata> {
    if ikili.as_os_str().is_empty() {
        return Err(OyunHata::GecersizGirdi(
            "Calistirilacak ikili yolu bos olamaz".to_string(),
        ));
    }
    if ikili.to_string_lossy().chars().any(char::is_control) {
        return Err(OyunHata::GecersizGirdi(
            "Ikili yolu kontrol karakteri iceremez".to_string(),
        ));
    }
    let dosya_adi = ikili
        .file_name()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    let dosya_koku = ikili
        .file_stem()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);

    if dosya_adi
        .as_deref()
        .is_some_and(|ad| YASAK_KABUKLAR.contains(&ad))
        || dosya_koku
            .as_deref()
            .is_some_and(|kok| YASAK_KABUKLAR.contains(&kok))
    {
        return Err(OyunHata::GecersizGirdi(format!(
            "Kabuk uzerinden surec baslatmak yasaktir: {}",
            ikili.display()
        )));
    }
    Ok(())
}

fn argumanlar_dogrula(argumanlar: &[String]) -> Result<(), OyunHata> {
    for arg in argumanlar {
        if arg.chars().any(char::is_control) {
            return Err(OyunHata::GecersizGirdi(
                "Komut argumani kontrol karakteri iceremez".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn varsayilan_moonlight_yollari() -> Vec<PathBuf> {
    let mut yollar = Vec::new();

    #[cfg(windows)]
    {
        let mut aday_ekle = |yol: PathBuf| {
            if !yollar.contains(&yol) {
                yollar.push(yol);
            }
        };

        if let Some(pf) = std::env::var_os("ProgramFiles") {
            aday_ekle(
                PathBuf::from(pf)
                    .join("Moonlight Game Streaming")
                    .join("Moonlight.exe"),
            );
        } else {
            aday_ekle(PathBuf::from(
                r"C:\Program Files\Moonlight Game Streaming\Moonlight.exe",
            ));
        }

        if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
            aday_ekle(
                PathBuf::from(pf86)
                    .join("Moonlight Game Streaming")
                    .join("Moonlight.exe"),
            );
        }

        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local_pb = PathBuf::from(local);
            aday_ekle(
                local_pb
                    .join("Programs")
                    .join("Moonlight Game Streaming")
                    .join("Moonlight.exe"),
            );
            aday_ekle(
                local_pb
                    .join("Moonlight Game Streaming")
                    .join("Moonlight.exe"),
            );
        }
    }

    #[cfg(not(windows))]
    {
        for yol in [
            "/usr/bin/moonlight",
            "/usr/local/bin/moonlight",
            "/usr/bin/moonlight-qt",
            "/usr/local/bin/moonlight-qt",
            "/Applications/Moonlight.app/Contents/MacOS/Moonlight",
        ] {
            yollar.push(PathBuf::from(yol));
        }
    }

    yollar
}

pub fn moonlight_bul_yollarla(
    aday_yollar: &[PathBuf],
    path_env: Option<&OsStr>,
) -> Result<PathBuf, OyunHata> {
    for yol in aday_yollar {
        if yol.is_file() {
            return Ok(yol.clone());
        }
    }

    if let Some(path_degeri) = path_env {
        #[cfg(windows)]
        const ADAY_ADLAR: &[&str] = &["Moonlight.exe", "moonlight.exe", "moonlight-qt.exe"];
        #[cfg(not(windows))]
        const ADAY_ADLAR: &[&str] = &["moonlight", "moonlight-qt"];

        for klasor in std::env::split_paths(path_degeri) {
            if klasor.as_os_str().is_empty() {
                continue;
            }
            for ad in ADAY_ADLAR {
                let aday = klasor.join(ad);
                if aday.is_file() {
                    return Ok(aday);
                }
            }
        }
    }

    Err(OyunHata::MoonlightEksik {
        indirme_url: MOONLIGHT_RESMI_INDIRME_URL.to_string(),
    })
}

pub fn moonlight_bul() -> Result<PathBuf, OyunHata> {
    let adaylar = varsayilan_moonlight_yollari();
    let path_env = std::env::var_os("PATH");
    moonlight_bul_yollarla(&adaylar, path_env.as_deref())
}

pub fn list_argumanlari(host: &str) -> Result<Vec<String>, OyunHata> {
    let host = host_dogrula(host)?;
    Ok(vec!["list".to_string(), host.to_string()])
}

pub fn pair_argumanlari(host: &str, pin: Option<&str>) -> Result<Vec<String>, OyunHata> {
    let host = host_dogrula(host)?;
    let mut arglar = vec!["pair".to_string(), host.to_string()];
    if let Some(p) = pin {
        let p = pin_dogrula(p)?;
        arglar.push("--pin".to_string());
        arglar.push(p.to_string());
    }
    Ok(arglar)
}

pub fn quit_argumanlari(host: &str) -> Result<Vec<String>, OyunHata> {
    let host = host_dogrula(host)?;
    Ok(vec!["quit".to_string(), host.to_string()])
}

pub fn stream_argumanlari(
    host: &str,
    uygulama: &str,
    ayar: &OyunAyar,
) -> Result<Vec<String>, OyunHata> {
    let host = host_dogrula(host)?;
    let uygulama = uygulama_dogrula(uygulama)?;
    ayar.dogrula()?;

    let mut arglar = vec!["stream".to_string(), host.to_string(), uygulama.to_string()];
    arglar.extend(ayar.cozunurluk.cli_argumanlari());
    arglar.push("--fps".to_string());
    arglar.push(ayar.fps.to_string());

    if let Some(kbps) = ayar.bitrate_kbps {
        arglar.push("--bitrate".to_string());
        arglar.push(kbps.to_string());
    }

    arglar.push("--video-codec".to_string());
    arglar.push(ayar.codec.cli_degeri().to_string());
    arglar.push("--display-mode".to_string());
    arglar.push(ayar.ekran_modu.cli_degeri().to_string());

    if ayar.coklu_kumanda {
        arglar.push("--multi-controller".to_string());
    } else {
        arglar.push("--no-multi-controller".to_string());
    }

    Ok(arglar)
}

pub fn komut_olustur(ikili: &Path, argumanlar: &[String]) -> Result<Command, OyunHata> {
    ikili_dogrula(ikili)?;
    argumanlar_dogrula(argumanlar)?;
    let mut komut = Command::new(ikili);
    komut.args(argumanlar);
    Ok(komut)
}

pub fn komut_calistir(ikili: &Path, argumanlar: &[String]) -> Result<KomutCikti, OyunHata> {
    let mut komut = komut_olustur(ikili, argumanlar)?;
    komut.stdin(Stdio::null());
    let cikti = komut
        .output()
        .map_err(|e| OyunHata::SurecBaslatilamadi(e.to_string()))?;
    Ok(KomutCikti {
        cikis_kodu: cikti.status.code(),
        stdout: String::from_utf8_lossy(&cikti.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&cikti.stderr).into_owned(),
    })
}

impl MoonlightSurec {
    pub fn baslat(ikili: &Path, argumanlar: &[String]) -> Result<Self, OyunHata> {
        let mut komut = komut_olustur(ikili, argumanlar)?;
        komut.stdin(Stdio::null());
        let cocuk = komut
            .spawn()
            .map_err(|e| OyunHata::SurecBaslatilamadi(e.to_string()))?;
        Ok(Self {
            cocuk: Some(cocuk),
            durum: OyunDurum::Calisiyor,
            son_cikis_kodu: None,
        })
    }

    pub fn akis_baslat(
        ikili: &Path,
        host: &str,
        uygulama: &str,
        ayar: &OyunAyar,
    ) -> Result<Self, OyunHata> {
        let arglar = stream_argumanlari(host, uygulama, ayar)?;
        Self::baslat(ikili, &arglar)
    }

    pub fn pid(&self) -> Option<u32> {
        self.cocuk.as_ref().map(Child::id)
    }

    pub fn durum(&mut self) -> OyunDurum {
        if let Some(cocuk) = self.cocuk.as_mut() {
            if let Ok(Some(durum)) = cocuk.try_wait() {
                let kod = durum.code();
                self.son_cikis_kodu = kod;
                self.cocuk = None;
                self.durum = OyunDurum::Sonlandi { cikis_kodu: kod };
            }
        }
        self.durum.clone()
    }

    pub fn son_cikis_kodu(&self) -> Option<i32> {
        self.son_cikis_kodu
    }

    pub fn durdur(&mut self) -> Result<Option<i32>, OyunHata> {
        let Some(mut cocuk) = self.cocuk.take() else {
            self.durum = OyunDurum::Bosta;
            return Ok(self.son_cikis_kodu);
        };

        if let Ok(Some(durum)) = cocuk.try_wait() {
            let kod = durum.code();
            self.son_cikis_kodu = kod;
            self.durum = OyunDurum::Bosta;
            return Ok(kod);
        }

        if let Err(kill_hatasi) = cocuk.kill() {
            if let Ok(Some(durum)) = cocuk.try_wait() {
                let kod = durum.code();
                self.son_cikis_kodu = kod;
                self.durum = OyunDurum::Bosta;
                return Ok(kod);
            }
            return Err(OyunHata::SurecSonlandirilamadi(kill_hatasi.to_string()));
        }

        let durum = cocuk
            .wait()
            .map_err(|e| OyunHata::SurecSonlandirilamadi(e.to_string()))?;
        let kod = durum.code();
        self.son_cikis_kodu = kod;
        self.durum = OyunDurum::Bosta;
        Ok(kod)
    }
}

impl Drop for MoonlightSurec {
    fn drop(&mut self) {
        let _ = self.durdur();
    }
}

pub fn varsayilan_sunshine_conf_yolu() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let mut adaylar = Vec::new();
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            adaylar.push(
                PathBuf::from(pf)
                    .join("Sunshine")
                    .join("config")
                    .join("sunshine.conf"),
            );
        } else {
            adaylar.push(PathBuf::from(
                r"C:\Program Files\Sunshine\config\sunshine.conf",
            ));
        }
        if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
            adaylar.push(
                PathBuf::from(pf86)
                    .join("Sunshine")
                    .join("config")
                    .join("sunshine.conf"),
            );
        }
        for yol in &adaylar {
            if yol.is_file() {
                return Some(yol.clone());
            }
        }
        adaylar.into_iter().next()
    }

    #[cfg(not(windows))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("sunshine").join("sunshine.conf"));
        }
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("sunshine")
                .join("sunshine.conf")
        })
    }
}

pub fn sunshine_origin_denetle_metin(icerik: &str) -> Result<(), OyunHata> {
    let mut son_deger: Option<String> = None;

    for (indeks, ham_satir) in icerik.lines().enumerate() {
        let satir = if indeks == 0 {
            ham_satir.trim_start_matches('\u{feff}').trim()
        } else {
            ham_satir.trim()
        };
        if satir.is_empty() || satir.starts_with('#') || satir.starts_with(';') {
            continue;
        }
        let Some((anahtar, ham_deger)) = satir.split_once('=') else {
            continue;
        };
        if anahtar.trim().eq_ignore_ascii_case("origin_web_ui_allowed") {
            let deger = ham_deger
                .split(['#', ';'])
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .trim();
            son_deger = Some(deger.to_ascii_lowercase());
        }
    }

    if son_deger.as_deref() == Some("pc") {
        Ok(())
    } else {
        Err(OyunHata::YonetimPaneliDisaAcik)
    }
}

pub fn sunshine_conf_denetle(yol: &Path) -> Result<(), OyunHata> {
    let icerik = std::fs::read_to_string(yol)
        .map_err(|e| OyunHata::YapilandirmaOkunamadi(format!("{}: {e}", yol.display())))?;
    sunshine_origin_denetle_metin(&icerik)
}

#[cfg(test)]
fn sahte_moonlight_derle(klasor: &Path) -> PathBuf {
    let kaynak = klasor.join("sahte_moonlight.rs");
    let ikili = if cfg!(windows) {
        klasor.join("Moonlight.exe")
    } else {
        klasor.join("moonlight")
    };
    let kod = r#"
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") => {
            println!("Moonlight Qt 0.0.0-test");
        }
        Some("list") => {
            if args.get(1).map(String::as_str) == Some("127.0.0.1") {
                println!("Desktop");
            } else {
                std::process::exit(1);
            }
        }
        Some("stream") => {
            if args.get(1).map(String::as_str) != Some("127.0.0.1") {
                std::process::exit(2);
            }
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        Some("pair") | Some("quit") => {}
        _ => std::process::exit(1),
    }
}
"#;
    std::fs::write(&kaynak, kod).expect("sahte moonlight kaynagi yazilamadi");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let cikti = Command::new(rustc)
        .arg(&kaynak)
        .arg("-o")
        .arg(&ikili)
        .output()
        .expect("rustc sahte moonlight derleyemedi");
    assert!(
        cikti.status.success(),
        "sahte moonlight derleme hatasi: {}",
        String::from_utf8_lossy(&cikti.stderr)
    );
    ikili
}

#[cfg(test)]
#[test]
fn test_varsayilan_1080p60_cli_argumanlari() {
    let ayar = OyunAyar::varsayilan();
    let args = stream_argumanlari("127.0.0.1", "Desktop", &ayar).expect("gecerli varsayilan ayar");
    assert_eq!(
        args,
        vec![
            "stream",
            "127.0.0.1",
            "Desktop",
            "--1080",
            "--fps",
            "60",
            "--video-codec",
            "auto",
            "--display-mode",
            "fullscreen",
            "--multi-controller",
        ]
    );

    // Kabuksuz Command dogrulamasi (cmd/powershell yok)
    let komut = komut_olustur(Path::new("Moonlight.exe"), &args).expect("komut olusturulabilmeli");
    assert_eq!(komut.get_program(), OsStr::new("Moonlight.exe"));
    let komut_arglar: Vec<&OsStr> = komut.get_args().collect();
    assert_eq!(komut_arglar.len(), args.len());
    assert!(matches!(
        komut_olustur(Path::new("cmd.exe"), &args),
        Err(OyunHata::GecersizGirdi(_))
    ));
    assert!(matches!(
        komut_olustur(Path::new("powershell.exe"), &args),
        Err(OyunHata::GecersizGirdi(_))
    ));

    // list, pair, quit arguman ureticileri
    assert_eq!(
        list_argumanlari("127.0.0.1").unwrap(),
        vec!["list", "127.0.0.1"]
    );
    assert_eq!(
        pair_argumanlari("127.0.0.1", Some("4829")).unwrap(),
        vec!["pair", "127.0.0.1", "--pin", "4829"]
    );
    assert_eq!(
        pair_argumanlari("127.0.0.1", None).unwrap(),
        vec!["pair", "127.0.0.1"]
    );
    assert_eq!(
        quit_argumanlari("127.0.0.1").unwrap(),
        vec!["quit", "127.0.0.1"]
    );

    // Bos host ve kontrol karakteri korumalari
    assert!(matches!(
        stream_argumanlari("", "Desktop", &ayar),
        Err(OyunHata::GecersizGirdi(_))
    ));
    assert!(matches!(
        stream_argumanlari("127.0.0.1\ncalc", "Desktop", &ayar),
        Err(OyunHata::GecersizGirdi(_))
    ));
    assert!(matches!(
        stream_argumanlari("127.0.0.1", "Desktop\0", &ayar),
        Err(OyunHata::GecersizGirdi(_))
    ));
    assert!(matches!(
        pair_argumanlari("127.0.0.1", Some("12ab")),
        Err(OyunHata::GecersizGirdi(_))
    ));
}

#[cfg(test)]
#[test]
fn test_yuksek_profil_4k120_hevc_argumanlari() {
    use super::{Codec, Cozunurluk, EkranModu};

    let ayar = OyunAyar {
        cozunurluk: Cozunurluk::Uhd4k,
        fps: 120,
        bitrate_kbps: Some(60000),
        codec: Codec::Hevc,
        ekran_modu: EkranModu::Borderless,
        coklu_kumanda: true,
    };
    let args = stream_argumanlari("127.0.0.1", "Desktop", &ayar).expect("gecerli 4K120 ayar");
    let beklenen_parca = [
        "--4K",
        "--fps",
        "120",
        "--bitrate",
        "60000",
        "--video-codec",
        "HEVC",
        "--display-mode",
        "borderless",
    ];
    let eslesti = args
        .windows(beklenen_parca.len())
        .any(|w| w.iter().map(String::as_str).eq(beklenen_parca.iter().copied()));
    assert!(eslesti, "beklenen 4K120 parcasi bulunamadi: {args:?}");

    // 1440p + AV1 + H.264 + coklu kumanda kapali
    let qhd_av1 = OyunAyar {
        cozunurluk: Cozunurluk::Qhd1440,
        fps: 120,
        bitrate_kbps: Some(40000),
        codec: Codec::Av1,
        ekran_modu: EkranModu::Windowed,
        coklu_kumanda: false,
    };
    let qhd_args = stream_argumanlari("127.0.0.1", "Desktop", &qhd_av1).unwrap();
    assert!(qhd_args.contains(&"--1440".to_string()));
    assert!(qhd_args.contains(&"AV1".to_string()));
    assert!(qhd_args.contains(&"windowed".to_string()));
    assert!(qhd_args.contains(&"--no-multi-controller".to_string()));

    let h264 = OyunAyar {
        codec: Codec::H264,
        ..OyunAyar::varsayilan()
    };
    assert!(stream_argumanlari("127.0.0.1", "Desktop", &h264)
        .unwrap()
        .contains(&"H.264".to_string()));

    // FPS (10-480) ve bitrate (500-500000) aralik korumalari
    let dusuk_fps = OyunAyar {
        fps: 9,
        ..OyunAyar::varsayilan()
    };
    assert!(matches!(
        stream_argumanlari("127.0.0.1", "Desktop", &dusuk_fps),
        Err(OyunHata::GecersizGirdi(_))
    ));
    let yuksek_fps = OyunAyar {
        fps: 481,
        ..OyunAyar::varsayilan()
    };
    assert!(matches!(
        stream_argumanlari("127.0.0.1", "Desktop", &yuksek_fps),
        Err(OyunHata::GecersizGirdi(_))
    ));
    let dusuk_bitrate = OyunAyar {
        bitrate_kbps: Some(499),
        ..OyunAyar::varsayilan()
    };
    assert!(matches!(
        stream_argumanlari("127.0.0.1", "Desktop", &dusuk_bitrate),
        Err(OyunHata::GecersizGirdi(_))
    ));
    let yuksek_bitrate = OyunAyar {
        bitrate_kbps: Some(500_001),
        ..OyunAyar::varsayilan()
    };
    assert!(matches!(
        stream_argumanlari("127.0.0.1", "Desktop", &yuksek_bitrate),
        Err(OyunHata::GecersizGirdi(_))
    ));
}

#[cfg(test)]
#[test]
fn test_headless_127_0_0_1_surec_baslat_ve_sonlandir() {
    let gecici = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(hedef) if Path::new(&hedef).is_dir() => tempfile::tempdir_in(hedef).unwrap(),
        _ => tempfile::tempdir().unwrap(),
    };
    let sahte_exe = sahte_moonlight_derle(gecici.path());

    // Kisa komut (list 127.0.0.1) cikis kodu 0 kaniti
    let liste_arg = list_argumanlari("127.0.0.1").unwrap();
    let liste_cikti = komut_calistir(&sahte_exe, &liste_arg).expect("sahte list calismali");
    assert_eq!(liste_cikti.cikis_kodu, Some(0));
    assert!(liste_cikti.stdout.contains("Desktop"));

    // Akis surecini baslat, PID ve aktif durumu dogrula, durdur() ile temiz sonlandir
    let mut surec =
        MoonlightSurec::akis_baslat(&sahte_exe, "127.0.0.1", "Desktop", &OyunAyar::varsayilan())
            .expect("sahte akis baslamali");
    assert!(surec.pid().is_some(), "surec PID bos olmamali");
    assert!(matches!(
        surec.durum(),
        OyunDurum::Baglaniyor | OyunDurum::Calisiyor
    ));

    let cikis_kodu = surec.durdur().expect("surec temiz sonlanmali");
    assert_eq!(surec.durum(), OyunDurum::Bosta);
    assert!(surec.pid().is_none());
    #[cfg(windows)]
    assert!(
        cikis_kodu.is_some(),
        "Windows sonlandirma cikis kodu raporlanmali"
    );
    #[cfg(not(windows))]
    let _ = cikis_kodu;

    // Gercek Moonlight kuruluysa ek --version kaniti
    if let Ok(gercek_yol) = moonlight_bul() {
        let surum = komut_calistir(&gercek_yol, &["--version".to_string()]);
        assert!(surum.is_ok(), "gercek moonlight --version basarisiz");
    }
}

#[cfg(test)]
#[test]
fn test_moonlight_eksik_resmi_link_doner() {
    let sonuc = moonlight_bul_yollarla(&[], None);
    assert_eq!(
        sonuc,
        Err(OyunHata::MoonlightEksik {
            indirme_url: "https://moonlight-stream.org".to_string(),
        })
    );

    // Sahte ikili aday yolda bulunabilmeli
    let gecici = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(hedef) if Path::new(&hedef).is_dir() => tempfile::tempdir_in(hedef).unwrap(),
        _ => tempfile::tempdir().unwrap(),
    };
    let sahte_yol = gecici.path().join("Moonlight.exe");
    std::fs::write(&sahte_yol, b"").unwrap();
    let bulunan = moonlight_bul_yollarla(&[sahte_yol.clone()], None).expect("aday yolda bulunmali");
    assert_eq!(bulunan, sahte_yol);
}

#[cfg(test)]
#[test]
fn test_sunshine_47990_origin_pc_denetimi() {
    let lan_conf = "# Sunshine ornek yapilandirma\norigin_web_ui_allowed = lan\n";
    assert_eq!(
        sunshine_origin_denetle_metin(lan_conf),
        Err(OyunHata::YonetimPaneliDisaAcik)
    );

    let wan_conf = "origin_web_ui_allowed = wan\n";
    assert_eq!(
        sunshine_origin_denetle_metin(wan_conf),
        Err(OyunHata::YonetimPaneliDisaAcik)
    );

    // Anahtar hic yoksa Sunshine varsayilani lan oldugu icin reddedilmeli
    let bos_conf = "port = 47989\n# origin_web_ui_allowed = pc\n";
    assert_eq!(
        sunshine_origin_denetle_metin(bos_conf),
        Err(OyunHata::YonetimPaneliDisaAcik)
    );

    let pc_conf = "port = 47989\norigin_web_ui_allowed = pc\n";
    assert_eq!(sunshine_origin_denetle_metin(pc_conf), Ok(()));

    // Dosyadan salt-okur denetim
    let gecici = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(hedef) if Path::new(&hedef).is_dir() => tempfile::tempdir_in(hedef).unwrap(),
        _ => tempfile::tempdir().unwrap(),
    };
    let conf_yolu = gecici.path().join("sunshine.conf");
    std::fs::write(&conf_yolu, pc_conf).unwrap();
    assert_eq!(sunshine_conf_denetle(&conf_yolu), Ok(()));
    assert_eq!(std::fs::read_to_string(&conf_yolu).unwrap(), pc_conf);
}
