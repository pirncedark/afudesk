//! Mevcut terminal pencereleri için geçici, geri yüklenen dik görünüm.
use crate::goruntu::Goruntu;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pencere {
    pub id: u64,
    pub baslik: String,
    pub uygulama: String,
}

/// Birincil ekran yakalamasına göre fiziksel piksel koordinatları.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alan {
    pub x: i32,
    pub y: i32,
    pub en: u32,
    pub boy: u32,
}

fn siniflandir(sinif: &str, islem: &str) -> Option<&'static str> {
    let islem = islem
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(islem)
        .to_ascii_lowercase();
    match islem.as_str() {
        "windowsterminal.exe" => Some("Windows Terminal"),
        "pwsh.exe" | "powershell.exe" => Some("PowerShell"),
        "cmd.exe" => Some("Komut İstemi"),
        "wezterm-gui.exe" | "wezterm.exe" => Some("WezTerm"),
        "alacritty.exe" => Some("Alacritty"),
        _ if sinif.eq_ignore_ascii_case("CASCADIA_HOSTING_WINDOW_CLASS") => {
            Some("Windows Terminal")
        }
        // Modern Windows console windows belong to conhost/OpenConsole, not cmd/pwsh.
        _ if sinif.eq_ignore_ascii_case("ConsoleWindowClass") => Some("Terminal"),
        _ => None,
    }
}

fn dik_alan(calisma: Alan, en: u32, boy: u32) -> Result<Alan> {
    ensure!(
        en > 0 && boy > 0 && calisma.en > 0 && calisma.boy > 0,
        "Terminal boyutu alınamadı. Yeniden dene."
    );
    let oran = en as f64 / boy as f64;
    let h = ((calisma.boy as f64 * 0.9)
        .min(calisma.en as f64 / oran)
        .floor() as u32)
        .max(1);
    let w = ((h as f64 * oran).round() as u32).clamp(1, calisma.en);
    Ok(Alan {
        x: calisma.x + ((calisma.en - w) / 2) as i32,
        y: calisma.y + ((calisma.boy - h) / 2) as i32,
        en: w,
        boy: h,
    })
}

pub fn kesisim(alan: Alan, en: u32, boy: u32) -> Result<Alan> {
    let x = i64::from(alan.x).max(0);
    let y = i64::from(alan.y).max(0);
    let sag = (i64::from(alan.x) + i64::from(alan.en)).min(i64::from(en));
    let alt = (i64::from(alan.y) + i64::from(alan.boy)).min(i64::from(boy));
    ensure!(sag > x && alt > y, "Terminal görünmüyor. Yeniden aç.");
    Ok(Alan {
        x: x as i32,
        y: y as i32,
        en: (sag - x) as u32,
        boy: (alt - y) as u32,
    })
}

pub fn kirp(g: &Goruntu, alan: Alan) -> Result<Goruntu> {
    let beklenen = (g.genislik as usize)
        .checked_mul(g.yukseklik as usize)
        .and_then(|n| n.checked_mul(4));
    ensure!(
        beklenen == Some(g.rgba.len()),
        "Terminal görüntüsü alınamadı. Yeniden dene."
    );
    let a = kesisim(alan, g.genislik, g.yukseklik)?;
    let satir = a.en as usize * 4;
    let mut rgba = Vec::with_capacity(satir * a.boy as usize);
    for y in a.y as usize..a.y as usize + a.boy as usize {
        let bas = (y * g.genislik as usize + a.x as usize) * 4;
        rgba.extend_from_slice(&g.rgba[bas..bas + satir]);
    }
    Ok(Goruntu {
        genislik: a.en,
        yukseklik: a.boy,
        rgba,
    })
}

pub fn fare_cevir(x: f32, y: f32, alan: Alan, ekran_en: u32, ekran_boy: u32) -> (f32, f32) {
    let Ok(a) = kesisim(alan, ekran_en, ekran_boy) else {
        return (0.0, 0.0);
    };
    let x = if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let y = if y.is_finite() {
        y.clamp(0.0, 1.0)
    } else {
        0.0
    };
    (
        (a.x as f32 + x * a.en.saturating_sub(1) as f32) / ekran_en.saturating_sub(1).max(1) as f32,
        (a.y as f32 + y * a.boy.saturating_sub(1) as f32)
            / ekran_boy.saturating_sub(1).max(1) as f32,
    )
}

struct GeriYukle(Option<Box<dyn FnOnce() + Send>>);
impl Drop for GeriYukle {
    fn drop(&mut self) {
        if let Some(geri) = self.0.take() {
            geri();
        }
    }
}

pub struct TerminalOturumu {
    #[cfg(windows)]
    id: u64,
    #[cfg(windows)]
    pid: u32,
    _geri: GeriYukle,
}

pub fn listele() -> Result<Vec<Pencere>> {
    #[cfg(windows)]
    {
        win::listele()
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("Terminal modu bu cihazda desteklenmiyor. Windows bilgisayar seç.")
    }
}

pub fn ekran_alani() -> Result<Alan> {
    #[cfg(windows)]
    {
        let (ekran, _) = win::monitor()?;
        Ok(Alan {
            x: 0,
            y: 0,
            ..ekran
        })
    }
    #[cfg(not(windows))]
    {
        anyhow::bail!("Terminal modu bu cihazda desteklenmiyor. Windows bilgisayar seç.")
    }
}

impl TerminalOturumu {
    /// Klavye/pano girdisi yalnız seçilen terminal gerçekten öndeyken gönderilir.
    pub fn odakla(&self) -> Result<()> {
        #[cfg(windows)]
        {
            win::odakla(self.id, self.pid)
        }
        #[cfg(not(windows))]
        {
            anyhow::bail!("Terminal modu bu cihazda desteklenmiyor. Windows bilgisayar seç.")
        }
    }

    pub fn ac(id: u64, en: u32, boy: u32) -> Result<Self> {
        #[cfg(windows)]
        {
            win::ac(id, en, boy)
        }
        #[cfg(not(windows))]
        {
            let _ = (id, en, boy);
            anyhow::bail!("Terminal modu bu cihazda desteklenmiyor. Windows bilgisayar seç.")
        }
    }

    pub fn alan(&self) -> Result<Alan> {
        #[cfg(windows)]
        {
            win::alan(self.id)
        }
        #[cfg(not(windows))]
        {
            anyhow::bail!("Terminal modu bu cihazda desteklenmiyor. Windows bilgisayar seç.")
        }
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::{
            HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
            WindowsAndMessaging::*,
        },
    };

    // A thread-local scope keeps all geometry in physical pixels without changing other threads.
    struct Dpi(isize);
    impl Dpi {
        fn new() -> Self {
            Self(
                unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }
                    as isize,
            )
        }
    }
    impl Drop for Dpi {
        fn drop(&mut self) {
            if self.0 != 0 {
                unsafe {
                    SetThreadDpiAwarenessContext(self.0 as _);
                }
            }
        }
    }

    pub(super) fn monitor() -> Result<(Alan, Alan)> {
        let _dpi = Dpi::new();
        unsafe {
            let m = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            ensure!(
                GetMonitorInfoW(m, &mut info) != 0,
                "Ekran alınamadı. Yeniden dene."
            );
            Ok((rect(info.rcMonitor), rect(info.rcWork)))
        }
    }

    fn rect(r: RECT) -> Alan {
        Alan {
            x: r.left,
            y: r.top,
            en: (r.right - r.left).max(0) as u32,
            boy: (r.bottom - r.top).max(0) as u32,
        }
    }

    unsafe fn islem_adi(hwnd: HWND) -> String {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if p.is_null() {
            return String::new();
        }
        let mut buf = vec![0u16; 32768];
        let mut len = buf.len() as u32;
        let sonuc = QueryFullProcessImageNameW(p, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(p);
        if sonuc == 0 {
            String::new()
        } else {
            String::from_utf16_lossy(&buf[..len as usize])
        }
    }

    unsafe extern "system" fn pencere(hwnd: HWND, data: LPARAM) -> i32 {
        if IsWindowVisible(hwnd) == 0 || !GetWindow(hwnd, GW_OWNER).is_null() {
            return 1;
        }
        let mut sinif = [0u16; 256];
        let n = GetClassNameW(hwnd, sinif.as_mut_ptr(), sinif.len() as i32);
        let Some(uygulama) = siniflandir(
            &String::from_utf16_lossy(&sinif[..n.max(0) as usize]),
            &islem_adi(hwnd),
        ) else {
            return 1;
        };
        let len = GetWindowTextLengthW(hwnd).clamp(0, 32767) as usize;
        let mut baslik = vec![0u16; len + 1];
        let n = GetWindowTextW(hwnd, baslik.as_mut_ptr(), baslik.len() as i32);
        let baslik = String::from_utf16_lossy(&baslik[..n.max(0) as usize]);
        let liste = &mut *(data as *mut Vec<Pencere>);
        liste.push(Pencere {
            id: hwnd as usize as u64,
            baslik: if baslik.trim().is_empty() {
                uygulama.into()
            } else {
                baslik
            },
            uygulama: uygulama.into(),
        });
        1
    }

    pub(super) fn listele() -> Result<Vec<Pencere>> {
        let mut liste = Vec::new();
        ensure!(
            unsafe { EnumWindows(Some(pencere), &mut liste as *mut _ as LPARAM) } != 0,
            "Terminaller alınamadı. Yeniden dene."
        );
        Ok(liste)
    }

    pub(super) fn ac(id: u64, en: u32, boy: u32) -> Result<TerminalOturumu> {
        ensure!(
            listele()?.iter().any(|p| p.id == id),
            "Terminal kapanmış. Başka terminal seç."
        );
        let _dpi = Dpi::new();
        let (_, calisma) = monitor()?;
        let hedef = dik_alan(calisma, en, boy)?;
        let hwnd = id as usize as HWND;
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            let mut onceki: WINDOWPLACEMENT = std::mem::zeroed();
            onceki.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            ensure!(
                GetWindowPlacement(hwnd, &mut onceki) != 0,
                "Terminal alınamadı. Yeniden seç."
            );
            // Install before the first mutation: even SetWindowPos failures restore placement.
            let geri = GeriYukle(Some(Box::new(move || {
                let _dpi = Dpi::new();
                let h = id as usize as HWND;
                let mut simdiki_pid = 0;
                GetWindowThreadProcessId(h, &mut simdiki_pid);
                if IsWindow(h) != 0 && simdiki_pid == pid {
                    SetWindowPlacement(h, &onceki);
                }
            })));
            ShowWindow(hwnd, SW_RESTORE);
            ensure!(
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    hedef.x,
                    hedef.y,
                    hedef.en as i32,
                    hedef.boy as i32,
                    SWP_SHOWWINDOW
                ) != 0,
                "Terminal açılamadı. Yeniden seç."
            );
            let oturum = TerminalOturumu {
                id,
                pid,
                _geri: geri,
            };
            oturum.odakla()?;
            oturum.alan()?;
            Ok(oturum)
        }
    }

    pub(super) fn odakla(id: u64, pid: u32) -> Result<()> {
        let hwnd = id as usize as HWND;
        unsafe {
            let mut simdiki_pid = 0;
            GetWindowThreadProcessId(hwnd, &mut simdiki_pid);
            ensure!(
                IsWindow(hwnd) != 0
                    && IsWindowVisible(hwnd) != 0
                    && IsIconic(hwnd) == 0
                    && simdiki_pid == pid,
                "Terminal görünmüyor. Yeniden aç."
            );
            if GetForegroundWindow() != hwnd {
                SetForegroundWindow(hwnd);
            }
            ensure!(
                GetForegroundWindow() == hwnd,
                "Terminal öne getirilemedi. Bilgisayarda terminali seçip yeniden dene."
            );
        }
        Ok(())
    }

    pub(super) fn alan(id: u64) -> Result<Alan> {
        let _dpi = Dpi::new();
        let hwnd = id as usize as HWND;
        let (ekran, _) = monitor()?;
        unsafe {
            ensure!(
                IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0,
                "Terminal görünmüyor. Yeniden aç."
            );
            let mut r: RECT = std::mem::zeroed();
            ensure!(
                GetWindowRect(hwnd, &mut r) != 0,
                "Terminal alınamadı. Yeniden seç."
            );
            let a = rect(r);
            kesisim(
                Alan {
                    x: a.x - ekran.x,
                    y: a.y - ekran.y,
                    ..a
                },
                ekran.en,
                ekran.boy,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_siniflandirma() {
        assert_eq!(
            siniflandir("CASCADIA_HOSTING_WINDOW_CLASS", ""),
            Some("Windows Terminal")
        );
        assert_eq!(
            siniflandir("ConsoleWindowClass", "powershell.exe"),
            Some("PowerShell")
        );
        assert_eq!(
            siniflandir("ConsoleWindowClass", "conhost.exe"),
            Some("Terminal")
        );
        assert_eq!(
            siniflandir("", "C:\\Apps\\WEZTERM-GUI.EXE"),
            Some("WezTerm")
        );
        assert_eq!(siniflandir("", "alacritty.exe"), Some("Alacritty"));
        assert_eq!(siniflandir("Chrome_WidgetWin_1", "chrome.exe"), None);
    }

    #[test]
    fn dik_boyut_orani_ve_sinirlar() {
        let calisma = Alan {
            x: -120,
            y: 30,
            en: 1920,
            boy: 1040,
        };
        let a = dik_alan(calisma, 1080, 2340).unwrap();
        assert_eq!(a.boy, 936);
        assert!((a.en as f64 / a.boy as f64 - 1080.0 / 2340.0).abs() < 0.002);
        assert!(a.x >= calisma.x && a.y >= calisma.y);
        assert!(dik_alan(calisma, 0, 200).is_err());
        let a = dik_alan(
            Alan {
                en: 200,
                boy: 1000,
                ..calisma
            },
            900,
            100,
        )
        .unwrap();
        assert!(a.en <= 200 && a.boy <= 900);
    }

    #[test]
    fn kirpma_ve_fare_ayni_ofseti_kullanir() {
        let g = Goruntu {
            genislik: 4,
            yukseklik: 3,
            rgba: (0..48).collect(),
        };
        let alan = Alan {
            x: 1,
            y: 1,
            en: 2,
            boy: 2,
        };
        let k = kirp(&g, alan).unwrap();
        assert_eq!((k.genislik, k.yukseklik), (2, 2));
        assert_eq!(k.rgba, [&g.rgba[20..28], &g.rgba[36..44]].concat());
        assert_eq!(fare_cevir(0.0, 0.0, alan, 4, 3), (1.0 / 3.0, 0.5));
        assert_eq!(fare_cevir(1.0, 1.0, alan, 4, 3), (2.0 / 3.0, 1.0));
        assert_eq!(
            kesisim(
                Alan {
                    x: -1,
                    y: -1,
                    en: 3,
                    boy: 3
                },
                4,
                3
            )
            .unwrap(),
            Alan {
                x: 0,
                y: 0,
                en: 2,
                boy: 2
            }
        );
        assert!(kirp(&g, Alan { x: 9, ..alan }).is_err());
    }

    #[test]
    fn guard_hata_ve_panikte_bir_kez_geri_yukler() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let sayac = Arc::new(AtomicUsize::new(0));
        let s = sayac.clone();
        let _ = std::panic::catch_unwind(move || {
            let _guard = GeriYukle(Some(Box::new(move || {
                s.fetch_add(1, Ordering::SeqCst);
            })));
            panic!("sahte yakalama hatası");
        });
        assert_eq!(sayac.load(Ordering::SeqCst), 1);
        let s = sayac.clone();
        let sonuc: Result<()> = (|| {
            let _guard = GeriYukle(Some(Box::new(move || {
                s.fetch_add(1, Ordering::SeqCst);
            })));
            anyhow::bail!("sahte kurulum hatası")
        })();
        assert!(sonuc.is_err());
        assert_eq!(sayac.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn sahte_pencere_konum_boyut_ve_kucultulmus_durumuna_doner() {
        use std::sync::{Arc, Mutex};
        #[derive(Clone, Debug, PartialEq)]
        struct SahteYerlesim {
            alan: Alan,
            kucultulmus: bool,
            buyutulmus: bool,
        }
        for (kucultulmus, buyutulmus) in [(false, false), (true, false), (false, true)] {
            let onceki = SahteYerlesim {
                alan: Alan {
                    x: -1800,
                    y: 80,
                    en: 900,
                    boy: 700,
                },
                kucultulmus,
                buyutulmus,
            };
            let pencere = Arc::new(Mutex::new(onceki.clone()));
            {
                let hedef = pencere.clone();
                let kayit = onceki.clone();
                let _guard = GeriYukle(Some(Box::new(move || *hedef.lock().unwrap() = kayit)));
                *pencere.lock().unwrap() = SahteYerlesim {
                    alan: Alan {
                        x: 500,
                        y: 50,
                        en: 450,
                        boy: 980,
                    },
                    kucultulmus: false,
                    buyutulmus: false,
                };
            }
            assert_eq!(*pencere.lock().unwrap(), onceki);
        }
    }
}
