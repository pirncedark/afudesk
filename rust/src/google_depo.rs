//! Hesap/eşleşme verisinin Windows DPAPI deposu. Android'de platform Keystore
//! gereklidir; burada açık metin yedeği yoktur. Henüz mevcut kayitlar.json'u değiştirmez.
use std::{io, path::Path};

const AZAMI: u64 = 1024 * 1024;

pub fn yaz(yol: &Path, veri: &[u8]) -> io::Result<()> {
    if veri.len() as u64 > AZAMI {
        return Err(io::Error::other("Hesap kaydı çok büyük."));
    }
    crate::guvenilen::atomik_yaz(yol, &koru(veri, false)?)
}

pub fn oku(yol: &Path) -> io::Result<Vec<u8>> {
    if std::fs::metadata(yol)?.len() > AZAMI + 4096 {
        return Err(io::Error::other(
            "Hesap kaydı okunamadı. Yeniden giriş yap.",
        ));
    }
    koru(&std::fs::read(yol)?, true)
}

/// Çıkışta hata gizlenmez; silinemeyen dosya için başarılı çıkış bildirilmemeli.
pub fn sil(yol: &Path) -> io::Result<()> {
    match std::fs::remove_file(yol) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        r => r,
    }
}

#[cfg(not(windows))]
fn koru(_: &[u8], _: bool) -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Güvenli hesap deposu hazır değil. Yeniden dene.",
    ))
}

#[cfg(windows)]
fn koru(veri: &[u8], coz: bool) -> io::Result<Vec<u8>> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let girdi = CRYPT_INTEGER_BLOB {
        cbData: veri.len().try_into().map_err(io::Error::other)?,
        pbData: veri.as_ptr().cast_mut(),
    };
    let mut cikti = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    // Kullanıcı kapsamı: CRYPTPROTECT_LOCAL_MACHINE kullanılmaz. SYSTEM servis
    // aynı dosyayı sessizce çözemez; servis kimliği ayrı tasarlanmalıdır.
    // SAFETY: tüm girişler çağrı boyunca geçerli; DPAPI çıktısı LocalFree ile bırakılır.
    unsafe {
        let sonuc = if coz {
            CryptUnprotectData(
                &girdi,
                null_mut(),
                null(),
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut cikti,
            )
        } else {
            CryptProtectData(
                &girdi,
                null(),
                null(),
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut cikti,
            )
        };
        if sonuc == 0 {
            return Err(io::Error::other(
                "Hesap kaydı açılamadı. Yeniden giriş yap.",
            ));
        }
        let veri = if cikti.cbData == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(cikti.pbData, cikti.cbData as usize).to_vec()
        };
        if !cikti.pbData.is_null() {
            LocalFree(cikti.pbData.cast());
        }
        Ok(veri)
    }
}
