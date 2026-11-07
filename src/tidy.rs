//! Tidying a page after it arrives: blank pages, rotation, trim, straightening, brightness and the JPEG written for it.

use crate::profile::{Colour, Profile};
use image::{DynamicImage, Rgb, RgbImage};
use trindle_toolkit::t;

fn paper_level(img: &RgbImage) -> u8 {
    let mut hist = [0u32; 256];
    for p in img.pixels().step_by(7) {
        hist[((p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32) / 10) as usize] += 1;
    }
    let total: u32 = hist.iter().sum();
    let mut seen = 0;
    for v in (0..256).rev() {
        seen += hist[v];
        if seen * 100 > total * 20 {
            return v as u8;
        }
    }
    255
}

pub fn is_blank(img: &DynamicImage, sensitivity: u32) -> bool {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    if w < 20 || h < 20 {
        return false;
    }
    let paper = paper_level(&rgb) as i32;
    let (mx, my) = (w / 25, h / 25);
    let mut marks = 0u64;
    let mut total = 0u64;
    for y in (my..h - my).step_by(2) {
        for x in (mx..w - mx).step_by(2) {
            let p = rgb.get_pixel(x, y);
            let v = (p[0] as i32 * 3 + p[1] as i32 * 6 + p[2] as i32) / 10;
            total += 1;
            if paper - v > 60 {
                marks += 1;
            }
        }
    }

    let limit = 0.0004 + sensitivity as f64 * 0.00006;
    (marks as f64 / total.max(1) as f64) < limit
}

pub fn brightness_contrast(img: &DynamicImage, brightness: i32, contrast: i32) -> DynamicImage {
    if brightness == 0 && contrast == 0 {
        return img.clone();
    }
    let k = (100.0 + contrast as f32) / 100.0;
    let k = k * k;
    let b = brightness as f32 * 2.55;
    let table: Vec<u8> = (0..256).map(|v| (((v as f32 - 128.0) * k + 128.0) + b).clamp(0.0, 255.0) as u8).collect();
    let mut out = img.to_rgb8();
    for p in out.pixels_mut() {
        *p = Rgb([table[p[0] as usize], table[p[1] as usize], table[p[2] as usize]]);
    }
    DynamicImage::ImageRgb8(out)
}

pub fn threshold(img: &DynamicImage, level: i32) -> DynamicImage {
    let mut g = img.to_luma8();
    let cut = (level.clamp(0, 100) as f32 * 2.55) as u8;
    for p in g.pixels_mut() {
        p[0] = if p[0] >= cut { 255 } else { 0 };
    }
    DynamicImage::ImageLuma8(g)
}

pub fn rotate(img: DynamicImage, degrees: u16) -> DynamicImage {
    match degrees {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    }
}

pub fn tidy(p: &Profile, img: DynamicImage) -> Option<DynamicImage> {
    let mut img = rotate(img, p.rotate);
    if p.straighten {
        img = straighten(&img);
    }
    if p.trim {
        img = trim(&img);
    }
    img = brightness_contrast(&img, p.brightness, p.contrast);
    img = match p.colour {
        Colour::Full => img,
        Colour::Grey => DynamicImage::ImageLuma8(img.to_luma8()),
        Colour::BlackWhite => threshold(&img, p.threshold),
    };
    if p.skip_blank && is_blank(&img, 50) {
        return None;
    }
    Some(img)
}

fn luma_at(img: &image::GrayImage, x: u32, y: u32) -> u8 {
    img.get_pixel(x, y)[0]
}

pub fn trim(img: &DynamicImage) -> DynamicImage {
    let g = img.to_luma8();
    let (w, h) = g.dimensions();
    if w < 40 || h < 40 {
        return img.clone();
    }
    let paper = paper_level(&img.to_rgb8()) as i32;
    let row_mean = |y: u32| (0..w).step_by(3).map(|x| luma_at(&g, x, y) as u32).sum::<u32>() / (w / 3 + 1);
    let col_mean = |x: u32| (0..h).step_by(3).map(|y| luma_at(&g, x, y) as u32).sum::<u32>() / (h / 3 + 1);
    let dark = 70;
    let (mut t, mut b, mut l, mut r) = (0, h - 1, 0, w - 1);
    while t < h / 4 && row_mean(t) < dark {
        t += 1;
    }
    while b > h - h / 4 && row_mean(b) < dark {
        b -= 1;
    }
    while l < w / 4 && col_mean(l) < dark {
        l += 1;
    }
    while r > w - w / 4 && col_mean(r) < dark {
        r -= 1;
    }

    let (mut x0, mut y0, mut x1, mut y1) = (r, b, l, t);
    for y in (t..=b).step_by(2) {
        for x in (l..=r).step_by(2) {
            if paper - luma_at(&g, x, y) as i32 > 60 {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return img.crop_imm(l, t, r - l + 1, b - t + 1);
    }
    let m = (w.max(h) / 40).max(4);
    let (cx0, cy0) = (x0.saturating_sub(m).max(l), y0.saturating_sub(m).max(t));
    let (cx1, cy1) = ((x1 + m).min(r), (y1 + m).min(b));
    img.crop_imm(cx0, cy0, cx1 - cx0 + 1, cy1 - cy0 + 1)
}

fn rotated<P: image::Pixel + 'static>(img: &image::ImageBuffer<P, Vec<P::Subpixel>>, degrees: f32, fill: P) -> image::ImageBuffer<P, Vec<P::Subpixel>> {
    let (w, h) = img.dimensions();
    let (s, c) = degrees.to_radians().sin_cos();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let mut out = image::ImageBuffer::from_pixel(w, h, fill);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            let (sx, sy) = (c * dx + s * dy + cx, -s * dx + c * dy + cy);
            if sx >= 0.0 && sy >= 0.0 && sx < (w - 1) as f32 && sy < (h - 1) as f32 {
                out.put_pixel(x, y, *img.get_pixel(sx as u32, sy as u32));
            }
        }
    }
    out
}

pub fn tilt_of(img: &DynamicImage) -> Option<f32> {
    let small = img.thumbnail(500, 500).to_luma8();
    let fill = paper_level(&DynamicImage::ImageLuma8(small.clone()).to_rgb8());
    let score = |deg: f32| -> f64 {
        let r = rotated(&small, deg, image::Luma([fill]));
        let (w, h) = r.dimensions();
        let rows: Vec<f64> = (0..h).map(|y| (0..w).map(|x| (fill as i32 - r.get_pixel(x, y)[0] as i32).max(0) as f64).sum()).collect();
        let mean = rows.iter().sum::<f64>() / rows.len() as f64;
        rows.iter().map(|v| (v - mean) * (v - mean)).sum()
    };
    let flat = score(0.0);
    let mut best = (0.0f32, flat);
    let mut d = -6.0f32;
    while d <= 6.0 {
        let sc = score(d);
        if sc > best.1 {
            best = (d, sc);
        }
        d += 0.5;
    }
    if flat <= 0.0 || best.1 < flat * 1.08 {
        return None;
    }
    Some(best.0)
}

pub fn straighten(img: &DynamicImage) -> DynamicImage {
    match tilt_of(img) {
        Some(deg) if deg.abs() >= 0.4 => {
            let fill = paper_level(&img.to_rgb8());
            let rgb = img.to_rgb8();
            DynamicImage::ImageRgb8(rotated(&rgb, deg, Rgb([fill, fill, fill])))
        }
        _ => img.clone(),
    }
}

pub fn jpeg(img: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88);
    match img {
        DynamicImage::ImageLuma8(g) => g.write_with_encoder(enc),
        other => other.to_rgb8().write_with_encoder(enc),
    }
    .map_err(|e| t!("scan-save-page-encode", why = &e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView as _;

    #[test]
    fn blank_pages_are_found_but_text_pages_are_not() {
        crate::words::ready();
        let blank = DynamicImage::ImageRgb8(RgbImage::from_pixel(300, 400, Rgb([240, 238, 232])));
        assert!(is_blank(&blank, 50));
        let mut inked = RgbImage::from_pixel(300, 400, Rgb([240, 238, 232]));
        for y in 100..110 {
            for x in 40..260 {
                inked.put_pixel(x, y, Rgb([20, 20, 20]));
            }
        }
        assert!(!is_blank(&DynamicImage::ImageRgb8(inked), 50));
        let mut noise = RgbImage::from_pixel(300, 400, Rgb([240, 238, 232]));
        noise.put_pixel(100, 100, Rgb([200, 200, 200]));
        assert!(is_blank(&DynamicImage::ImageRgb8(noise), 50));
    }

    fn lined(skew: f32) -> DynamicImage {
        let mut g = image::GrayImage::from_pixel(400, 500, image::Luma([240]));
        for line in 0..12 {
            let y0 = 60 + line * 32;
            for x in 40..360u32 {
                let y = y0 as f32 + (x as f32 - 200.0) * skew.to_radians().tan();
                for dy in 0..4 {
                    g.put_pixel(x, (y as u32 + dy).min(499), image::Luma([30]));
                }
            }
        }
        DynamicImage::ImageLuma8(g)
    }

    #[test]
    fn tilt_is_found_and_straightened() {
        crate::words::ready();
        let t = tilt_of(&lined(3.0)).expect("lines");
        assert!((t + 3.0).abs() <= 1.0, "found {t}");
        let level = lined(0.0);
        assert_eq!(tilt_of(&level), None, "a level page needs no turn");
        assert_eq!(straighten(&level).as_bytes(), level.as_bytes(), "and is left as it is");
        let blank = DynamicImage::ImageLuma8(image::GrayImage::from_pixel(400, 500, image::Luma([240])));
        assert!(tilt_of(&blank).is_none());
        let tilted = lined(3.0);
        let fixed = straighten(&tilted);
        assert_ne!(fixed.as_bytes(), tilted.to_rgb8().as_raw().as_slice(), "the tilted page was turned");
        let after = tilt_of(&fixed);
        assert!(after.is_none_or(|d| d.abs() < 1.0), "left {after:?}");
    }

    #[test]
    fn trim_removes_dark_border_and_margin() {
        crate::words::ready();
        let mut g = image::GrayImage::from_pixel(400, 500, image::Luma([20]));
        for y in 30..470 {
            for x in 30..370 {
                g.put_pixel(x, y, image::Luma([240]));
            }
        }
        for y in 200..210 {
            for x in 120..280 {
                g.put_pixel(x, y, image::Luma([20]));
            }
        }
        let out = trim(&DynamicImage::ImageLuma8(g));
        let (w, h) = out.dimensions();
        assert!(w < 250 && h < 80, "{w}x{h}");
        assert!(w > 160 && h > 10);
    }

    #[test]
    fn brightness_and_black_white() {
        crate::words::ready();
        let grey = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([100, 100, 100])));
        let lighter = brightness_contrast(&grey, 20, 0).to_rgb8();
        assert!(lighter.get_pixel(0, 0)[0] > 140);
        let bw = threshold(&grey, 30).to_luma8();
        assert_eq!(bw.get_pixel(0, 0)[0], 255);
        let bw = threshold(&grey, 60).to_luma8();
        assert_eq!(bw.get_pixel(0, 0)[0], 0);
    }
}
