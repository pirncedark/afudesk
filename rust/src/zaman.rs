//! Host ve izleyici saatleri arasındaki fark ile kare gecikmesi.

/// Host saatinin izleyici saatine göre farkı. Ağ gidiş dönüşünün yarısı çıkarılır.
pub fn saat_farki(gonderim_ms: u64, host_ms: u64, alim_ms: u64) -> i64 {
    let izleyici_orta = (gonderim_ms as i128 + alim_ms as i128) / 2;
    (host_ms as i128 - izleyici_orta).clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

/// Yakalama anından izleyicide birleştirilene dek geçen süre.
pub fn gecikme_ms(simdi_izleyici_ms: u64, fark: i64, yakalama_ms: u64) -> u32 {
    let yakalama_izleyici = yakalama_ms as i128 - fark as i128;
    (simdi_izleyici_ms as i128 - yakalama_izleyici).clamp(0, u32::MAX as i128) as u32
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn saat_farki_rtt_yarisini_cikarir() {
        assert_eq!(saat_farki(1_000, 1_120, 1_040), 100);
    }

    #[test]
    fn negatif_saat_farki_desteklenir() {
        assert_eq!(saat_farki(1_100, 1_000, 1_140), -120);
        assert_eq!(gecikme_ms(1_200, -120, 1_050), 30);
    }

    #[test]
    fn gecikme_saat_geri_ve_tasimada_sinirlanir() {
        assert_eq!(gecikme_ms(100, -500, 200), 0);
        assert_eq!(gecikme_ms(u64::MAX, i64::MIN, 0), u32::MAX);
    }
}
