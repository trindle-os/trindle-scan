//! Naming saved files: the pattern, the clock, the counter, and writing the pictures.

use crate::profile::Format;
use image::ImageFormat;
use std::path::{Path, PathBuf};

pub struct Clock {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

pub fn now() -> Clock {

    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        Clock { year: tm.tm_year + 1900, month: tm.tm_mon as u32 + 1, day: tm.tm_mday as u32, hour: tm.tm_hour as u32, minute: tm.tm_min as u32, second: tm.tm_sec as u32 }
    }
}

pub fn name_from(pattern: &str, profile: &str, c: &Clock, counter: u32) -> String {
    fill_parts(pattern, profile, c, counter).chars().map(|ch| if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control() { '-' } else { ch }).collect()
}

pub fn fill_parts(pattern: &str, profile: &str, c: &Clock, counter: u32) -> String {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('}') else {
            out.push_str(&rest[i..]);
            rest = "";
            break;
        };
        let key = &rest[i + 1..i + j];
        match key {
            "year" => out += &format!("{:04}", c.year),
            "month" => out += &format!("{:02}", c.month),
            "day" => out += &format!("{:02}", c.day),
            "hour" => out += &format!("{:02}", c.hour),
            "minute" => out += &format!("{:02}", c.minute),
            "second" => out += &format!("{:02}", c.second),
            "profile" => out += profile,
            k if k.starts_with("counter") => {
                let digits = k.split(':').nth(1).and_then(|d| d.parse().ok()).unwrap_or(1usize).clamp(1, 9);
                out += &format!("{counter:0digits$}");
            }
            _ => out.push_str(&rest[i..=i + j]),
        }
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    out
}

pub fn claim_name(dir: &Path, stem: &str, ext: &str) -> std::io::Result<(PathBuf, std::fs::File)> {
    for n in 1u32.. {
        let path = if n == 1 { dir.join(format!("{stem}.{ext}")) } else { dir.join(format!("{stem} {n}.{ext}")) };
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => return Ok((path, f)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

pub fn counter_prefix(last_part: &str, profile: &str, c: &Clock) -> Option<String> {
    let at = last_part.find("{counter")?;
    Some(name_from(&last_part[..at], profile, c, 1))
}

pub fn next_counter(dir: &Path, prefix: &str) -> u32 {
    let mut best = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            let stem = n.rsplit_once('.').map(|(s, _)| s).unwrap_or(&n);
            let Some(rest) = stem.strip_prefix(prefix) else { continue };
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = digits.parse::<u32>() {
                best = best.max(v);
            }
        }
    }
    best + 1
}

pub fn write_pictures(pages: &[(Vec<u8>, bool)], format: Format, dir: &Path, stem: &str) -> std::io::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut out = Vec::new();
    if format == Format::Tiff && pages.len() > 1 {
        let (path, file) = claim_name(dir, stem, "tif")?;
        let mut enc = tiff::encoder::TiffEncoder::new(std::io::BufWriter::new(file)).map_err(std::io::Error::other)?;
        for (bytes, grey) in pages {
            let img = image::load_from_memory(bytes).map_err(std::io::Error::other)?;
            if *grey {
                let g = img.to_luma8();
                enc.write_image::<tiff::encoder::colortype::Gray8>(g.width(), g.height(), g.as_raw()).map_err(std::io::Error::other)?;
            } else {
                let c = img.to_rgb8();
                enc.write_image::<tiff::encoder::colortype::RGB8>(c.width(), c.height(), c.as_raw()).map_err(std::io::Error::other)?;
            }
        }
        return Ok(vec![path]);
    }
    let one = pages.len() == 1;
    for (i, (bytes, _)) in pages.iter().enumerate() {
        let name = if one { stem.to_string() } else { format!("{stem} {}", i + 1) };
        let (path, mut file) = claim_name(dir, &name, format.ext())?;
        match format {
            Format::Jpeg => std::io::Write::write_all(&mut file, bytes)?,
            Format::Png | Format::Tiff => {
                let img = image::load_from_memory(bytes).map_err(std::io::Error::other)?;
                let kind = if format == Format::Png { ImageFormat::Png } else { ImageFormat::Tiff };
                img.write_to(&mut std::io::BufWriter::new(file), kind).map_err(std::io::Error::other)?;
            }
            Format::Pdf => return Err(std::io::Error::other("a picture cannot be saved as a PDF")),
        }
        out.push(path);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tidy::jpeg;
    use image::{DynamicImage, Rgb, RgbImage};

    use crate::testdir::tmp;

    #[test]
    fn patterns_fill_in_and_stay_safe() {
        crate::words::ready();
        let c = Clock { year: 2026, month: 10, day: 9, hour: 8, minute: 5, second: 3 };
        assert_eq!(name_from("Scan {year}-{month}-{day} {hour}.{minute}.{second}", "P", &c, 1), "Scan 2026-10-09 08.05.03");
        assert_eq!(name_from("{profile}/a:{counter:3}", "Doc", &c, 7), "Doc-a-007");
    }

    #[test]
    fn counter_continues_from_the_highest() {
        crate::words::ready();
        let d = tmp("counter");
        std::fs::write(d.join("Scan 004.pdf"), b"x").unwrap();
        std::fs::write(d.join("Scan 011.pdf"), b"x").unwrap();
        std::fs::write(d.join("Invoice 2024.pdf"), b"x").unwrap();
        assert_eq!(next_counter(&d, "Scan "), 12);
        assert_eq!(claim_name(&d, "Scan 004", "pdf").unwrap().0, d.join("Scan 004 2.pdf"));
        assert_eq!(counter_prefix("Scan {counter:3}", "P", &now()), Some("Scan ".into()));
        assert_eq!(counter_prefix("Scan", "P", &now()), None);
    }

    #[test]
    fn broken_patterns_stay_sane() {
        let c = Clock { year: 2026, month: 10, day: 9, hour: 8, minute: 5, second: 3 };
        assert_eq!(name_from("Scan {year", "P", &c, 1), "Scan {year");
        assert_eq!(name_from("a{counter:99999999999}", "P", &c, 7).len(), 1 + 9);
    }

    #[test]
    fn pictures_get_counted_names() {
        crate::words::ready();
        let d = tmp("pics");
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(8, 8, Rgb([1, 2, 3])));
        let j = jpeg(&img).unwrap();
        let made = write_pictures(&[(j.clone(), false), (j, false)], Format::Png, &d.join("sub"), "Scan").unwrap();
        assert_eq!(made.len(), 2);
        assert!(made[1].ends_with("Scan 2.png"));
        assert!(image::open(&made[0]).is_ok());
    }

    #[test]
    fn several_tiff_pages_go_into_one_file() {
        crate::words::ready();
        let d = tmp("tiff");
        let img = |c: u8| {
            let mut v = Vec::new();
            DynamicImage::ImageRgb8(RgbImage::from_pixel(16, 24, Rgb([c, 50, 50]))).write_to(&mut std::io::Cursor::new(&mut v), ImageFormat::Jpeg).unwrap();
            (v, false)
        };
        let files = write_pictures(&[img(10), img(120), img(240)], Format::Tiff, &d, "Bills").unwrap();
        assert_eq!(files.len(), 1);
        let mut dec = tiff::decoder::Decoder::new(std::io::BufReader::new(std::fs::File::open(&files[0]).unwrap())).unwrap();
        let mut n = 1;
        while dec.more_images() {
            dec.next_image().unwrap();
            n += 1;
        }
        assert_eq!(n, 3);
    }
}
