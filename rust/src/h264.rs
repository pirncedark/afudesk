//! Düşük gecikmeli yazılım H.264 (OpenH264, RGBA → YUV420).
use crate::goruntu::Goruntu;
use openh264::{
    decoder::Decoder,
    encoder::{
        BitRate, Encoder, EncoderConfig, FrameRate, IntraFramePeriod, Profile, RateControlMode,
        UsageType,
    },
    formats::{RgbaSliceU8, YUVBuffer, YUVSource},
};

pub struct H264Kodlayici {
    kodlayici: Encoder,
    yuv: YUVBuffer,
    gen: u32,
    yuk: u32,
    ilk: bool,
}

impl H264Kodlayici {
    pub fn yeni(gen: u32, yuk: u32, fps: u32, bitrate: u32) -> anyhow::Result<Self> {
        anyhow::ensure!(
            gen >= 2
                && yuk >= 2
                && gen.is_multiple_of(2)
                && yuk.is_multiple_of(2)
                && gen.max(yuk) <= 3840
                && gen.min(yuk) <= 2160,
            "H.264 boyutu desteklenmiyor"
        );
        let fps = fps.clamp(1, 60);
        let ayar = EncoderConfig::new()
            .usage_type(UsageType::ScreenContentRealTime)
            .profile(Profile::Baseline) // B-frame yok; tek zaman katmanı.
            .bitrate(BitRate::from_bps(bitrate.clamp(250_000, 20_000_000)))
            .max_frame_rate(FrameRate::from_hz(fps as f32))
            .rate_control_mode(RateControlMode::Bitrate)
            .skip_frames(true)
            .adaptive_quantization(false)
            .background_detection(false)
            .intra_frame_period(IntraFramePeriod::from_num_frames(fps * 2));
        Ok(Self {
            kodlayici: Encoder::with_api_config(openh264::OpenH264API::from_source(), ayar)?,
            yuv: YUVBuffer::new(gen as usize, yuk as usize),
            gen,
            yuk,
            ilk: true,
        })
    }

    pub fn kodla(&mut self, g: &Goruntu, anahtar_kare_iste: bool) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(
            (g.genislik, g.yukseklik) == (self.gen, self.yuk)
                && g.rgba.len() == self.gen as usize * self.yuk as usize * 4,
            "RGBA boyutu uyuşmuyor"
        );
        self.yuv.read_rgba8(RgbaSliceU8::new(
            &g.rgba,
            (self.gen as usize, self.yuk as usize),
        ));
        if anahtar_kare_iste && !self.ilk {
            self.kodlayici.force_intra_frame();
        }
        let bayt = self.kodlayici.encode(&self.yuv)?.to_vec();
        self.ilk = false;
        Ok(bayt)
    }
}

pub struct H264Cozucu {
    cozucu: Decoder,
}
impl H264Cozucu {
    pub fn yeni() -> anyhow::Result<Self> {
        Ok(Self {
            cozucu: Decoder::new()?,
        })
    }
    pub fn coz(&mut self, veri: &[u8]) -> anyhow::Result<Goruntu> {
        let yuv = self
            .cozucu
            .decode(veri)?
            .ok_or_else(|| anyhow::anyhow!("H.264 eksik kare"))?;
        let (gen, yuk) = yuv.dimensions();
        anyhow::ensure!(
            gen <= 3840 && yuk <= 3840 && gen.min(yuk) <= 2160,
            "H.264 boyutu desteklenmiyor"
        );
        let mut rgba = vec![0; gen * yuk * 4];
        yuv.write_rgba8(&mut rgba);
        Ok(Goruntu {
            genislik: gen as u32,
            yukseklik: yuk as u32,
            rgba,
        })
    }
}

/// Annex-B IDR NAL (tip 5); SPS/PPS öneki veya dilim sayısından bağımsız.
pub fn anahtar_mi(veri: &[u8]) -> bool {
    veri.windows(4)
        .any(|w| w[..3] == [0, 0, 1] && w[3] & 31 == 5)
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::goruntu::deneme_goruntu;
    #[test]
    fn kodla_coz_psnr_30_ustu() {
        let mut k = H264Kodlayici::yeni(320, 180, 15, 8_000_000).unwrap();
        let mut c = H264Cozucu::yeni().unwrap();
        for n in 0..8 {
            let g = deneme_goruntu(320, 180, n * 11);
            let v = k.kodla(&g, n == 4).unwrap();
            assert!(!v.is_empty());
            let sonuc = c.coz(&v).unwrap();
            assert_eq!((sonuc.genislik, sonuc.yukseklik), (320, 180));
            let mse = g
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(sonuc.rgba.as_chunks::<4>().0.iter())
                .flat_map(|(a, b)| (0..3).map(move |i| (a[i] as f64 - b[i] as f64).powi(2)))
                .sum::<f64>()
                / (320.0 * 180.0 * 3.0);
            let psnr = 10.0 * (255.0f64.powi(2) / mse).log10();
            assert!(psnr > 30.0, "PSNR={psnr}");
        }
    }
}

#[cfg(test)]
mod olcum {
    use super::*;
    use crate::goruntu::{deneme_goruntu, Kodlayici};
    #[test]
    #[ignore = "1080p karşılaştırma; elle tek iş parçacığında çalıştırılır"]
    fn jpeg_h264_1080p() {
        const N: u32 = 60;
        let mut jpeg = Kodlayici::new(70);
        let mut h264 = H264Kodlayici::yeni(1920, 1080, 15, 8_000_000).unwrap();
        let mut toplam = [(0.0, 0usize, 0u32); 2];
        // İki yol da aynı, her pikseli değişen 60 tohumlu sahneyi kodlar.
        for n in 0..N + 5 {
            let g = deneme_goruntu(1920, 1080, (n * 3) as u8);
            let bas = std::time::Instant::now();
            let j = jpeg.kodla(&g).unwrap().unwrap();
            let ms = bas.elapsed().as_secs_f64() * 1000.0;
            if n >= 5 {
                toplam[0].0 += ms;
                toplam[0].1 += j.dosemeler.iter().map(|d| d.jpeg.len()).sum::<usize>();
                toplam[0].2 += 1;
            }
            let bas = std::time::Instant::now();
            let h = h264.kodla(&g, false).unwrap();
            let ms = bas.elapsed().as_secs_f64() * 1000.0;
            if n >= 5 {
                toplam[1].0 += ms;
                toplam[1].1 += h.len();
                toplam[1].2 += u32::from(!h.is_empty());
            }
        }
        for (ad, (ms, bayt, uretilen)) in ["JPEG70", "H264_8Mbit_15fps"].into_iter().zip(toplam) {
            eprintln!(
                "1080p {ad}: kodla_ms={:.3}, KB/kare={:.3}, uretilen={uretilen}/{N} (KB=1000 bayt)",
                ms / N as f64,
                bayt as f64 / 1000.0 / N as f64
            );
        }
    }
}
