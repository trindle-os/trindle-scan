//! A small PDF writer: each page is a picture fitted to its page, written to the file as it goes; a password gives AES-256.

use aes::cipher::{
    BlockEncrypt, BlockEncryptMut, KeyInit, KeyIvInit,
    block_padding::{NoPadding, Pkcs7},
};
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::io::Write;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct PdfPage {
    pub jpeg: Arc<Vec<u8>>,

    pub mono: Option<Vec<u8>>,

    pub words: Vec<Word>,
    pub width: u32,
    pub height: u32,
    pub grey: bool,

    pub page: (f32, f32),
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Options {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,

    pub password: String,
}

pub fn mono_of(img: &image::DynamicImage) -> std::io::Result<Vec<u8>> {
    let g = img.to_luma8();
    let (w, h) = g.dimensions();
    let row = (w as usize).div_ceil(8);
    let mut bits = vec![0u8; row * h as usize];
    for (x, y, p) in g.enumerate_pixels() {
        if p[0] >= 128 {
            bits[y as usize * row + x as usize / 8] |= 0x80 >> (x % 8);
        }
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&bits)?;
    z.finish()
}

fn random<const N: usize>() -> std::io::Result<[u8; N]> {
    let mut b = [0u8; N];
    std::io::Read::read_exact(&mut std::fs::File::open("/dev/urandom")?, &mut b)?;
    Ok(b)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}

type Cbc128 = cbc::Encryptor<aes::Aes128>;
type Cbc256 = cbc::Encryptor<aes::Aes256>;

fn hash_r6(password: &[u8], salt: &[u8], udata: &[u8]) -> [u8; 32] {
    let mut k: Vec<u8> = Sha256::digest([password, salt, udata].concat()).to_vec();
    let mut round = 0usize;
    loop {
        let k1 = [password, &k, udata].concat().repeat(64);
        let e = Cbc128::new(k[..16].into(), k[16..32].into()).encrypt_padded_vec_mut::<NoPadding>(&k1);
        k = match e[..16].iter().map(|b| *b as u32).sum::<u32>() % 3 {
            0 => Sha256::digest(&e).to_vec(),
            1 => Sha384::digest(&e).to_vec(),
            _ => Sha512::digest(&e).to_vec(),
        };
        round += 1;
        if round >= 64 && *e.last().unwrap_or(&0) as usize <= round - 32 {
            break;
        }
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&k[..32]);
    out
}

const PERMISSIONS: i32 = -4;

struct Lock {
    key: [u8; 32],
    o: Vec<u8>,
    u: Vec<u8>,
    oe: Vec<u8>,
    ue: Vec<u8>,
    perms: [u8; 16],
    id: [u8; 16],
}

impl Lock {
    fn new(password: &str) -> std::io::Result<Lock> {
        let pw = &password.as_bytes()[..password.len().min(127)];
        let key: [u8; 32] = random()?;
        let wrap = |with: [u8; 32]| Cbc256::new((&with).into(), (&[0u8; 16]).into()).encrypt_padded_vec_mut::<NoPadding>(&key);
        let (vs, ks): ([u8; 8], [u8; 8]) = (random()?, random()?);
        let u = [&hash_r6(pw, &vs, &[])[..], &vs, &ks].concat();
        let ue = wrap(hash_r6(pw, &ks, &[]));
        let (vs, ks): ([u8; 8], [u8; 8]) = (random()?, random()?);
        let o = [&hash_r6(pw, &vs, &u)[..], &vs, &ks].concat();
        let oe = wrap(hash_r6(pw, &ks, &u));
        let mut perms: [u8; 16] = random()?;
        perms[..4].copy_from_slice(&PERMISSIONS.to_le_bytes());
        perms[4..8].copy_from_slice(&[0xFF; 4]);
        perms[8..12].copy_from_slice(b"Tadb");
        aes::Aes256::new((&key).into()).encrypt_block((&mut perms).into());
        Ok(Lock { key, o, u, oe, ue, perms, id: random()? })
    }

    fn seal(&self, data: &[u8]) -> std::io::Result<Vec<u8>> {
        let iv: [u8; 16] = random()?;
        let mut out = iv.to_vec();
        out.extend(Cbc256::new((&self.key).into(), (&iv).into()).encrypt_padded_vec_mut::<Pkcs7>(data));
        Ok(out)
    }

    fn dictionary(&self) -> String {
        format!(
            "/Filter /Standard /V 5 /R 6 /Length 256 /CF << /StdCF << /AuthEvent /DocOpen /CFM /AESV3 /Length 32 >> >> /StmF /StdCF /StrF /StdCF /P {PERMISSIONS} /O <{}> /U <{}> /OE <{}> /UE <{}> /Perms <{}>",
            hex(&self.o),
            hex(&self.u),
            hex(&self.oe),
            hex(&self.ue),
            hex(&self.perms)
        )
    }
}

fn text(s: &str, lock: Option<&Lock>) -> std::io::Result<String> {
    let bytes: Vec<u8> = if s.is_ascii() {
        s.bytes().filter(|b| !b.is_ascii_control()).collect()
    } else {
        let mut v = vec![0xFE, 0xFF];
        for u in s.encode_utf16() {
            v.extend_from_slice(&u.to_be_bytes());
        }
        v
    };
    Ok(match lock {
        Some(l) => format!("<{}>", hex(&l.seal(&bytes)?)),
        None if s.is_ascii() => format!("({})", String::from_utf8_lossy(&bytes).replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")),
        None => format!("<{}>", hex(&bytes)),
    })
}

const PER_FONT: usize = 250;

struct Letters {
    letters: Vec<char>,
}

impl Letters {
    fn of(pages: &[PdfPage]) -> Letters {
        if pages.iter().all(|p| p.words.is_empty()) {
            return Letters { letters: Vec::new() };
        }
        let mut set = std::collections::BTreeSet::new();
        for w in pages.iter().flat_map(|p| p.words.iter()) {
            set.extend(w.text.chars().filter(|c| !c.is_control()));
        }
        set.insert('?');
        Letters { letters: set.into_iter().collect() }
    }

    fn is_empty(&self) -> bool {
        self.letters.is_empty()
    }

    fn fonts(&self) -> usize {
        self.letters.len().div_ceil(PER_FONT)
    }

    fn code(&self, c: char) -> (usize, u8) {
        let at = self.letters.iter().position(|l| *l == c).or_else(|| self.letters.iter().position(|l| *l == '?')).unwrap_or(0);
        (at / PER_FONT, (at % PER_FONT + 1) as u8)
    }

    fn font(&self, k: usize, to_unicode: usize) -> String {
        let mine = &self.letters[k * PER_FONT..((k + 1) * PER_FONT).min(self.letters.len())];
        let names: String = mine.iter().map(|c| format!("/uni{:04X} ", *c as u32 & 0xFFFF)).collect();
        let widths = "500 ".repeat(mine.len());
        format!(
            "/Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 1 /LastChar {} /Widths [{widths}] /Encoding << /Type /Encoding /Differences [1 {names}] >> /ToUnicode {to_unicode} 0 R",
            mine.len()
        )
    }

    fn to_unicode(&self, k: usize) -> String {
        let mine = &self.letters[k * PER_FONT..((k + 1) * PER_FONT).min(self.letters.len())];
        let mut out = String::from(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<01> <FF>\nendcodespacerange\n",
        );
        for (b, chunk) in mine.chunks(100).enumerate() {
            out += &format!("{} beginbfchar\n", chunk.len());
            for (j, c) in chunk.iter().enumerate() {
                let units: String = c.encode_utf16(&mut [0u16; 2]).iter().map(|u| format!("{u:04X}")).collect();
                out += &format!("<{:02X}> <{units}>\n", b * 100 + j + 1);
            }
            out += "endbfchar\n";
        }
        out += "endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
        out
    }

    fn layer(&self, words: &[Word], s: f32, ox: f32, oy: f32, ih: f32) -> String {
        let mut out = String::from("\nBT 3 Tr\n");
        for w in words {
            let chars: Vec<char> = w.text.chars().filter(|c| !c.is_control()).collect();
            if chars.is_empty() || w.w <= 0.0 || w.h <= 0.0 {
                continue;
            }
            let size = (w.h * s).max(1.0);
            let natural = 0.5 * size * chars.len() as f32;
            let stretch = (w.w * s / natural * 100.0).clamp(10.0, 400.0);
            let (x, y) = (ox + w.x * s, oy + (ih - (w.y + w.h * 0.8)) * s);
            out += &format!("{stretch:.1} Tz 1 0 0 1 {x:.2} {y:.2} Tm");
            let mut run: (usize, String) = (usize::MAX, String::new());
            for c in &chars {
                let (font, code) = self.code(*c);
                if font != run.0 {
                    if !run.1.is_empty() {
                        out += &format!(" /F{} {size:.2} Tf <{}> Tj", run.0 + 1, run.1);
                    }
                    run = (font, String::new());
                }
                run.1 += &format!("{code:02X}");
            }
            out += &format!(" /F{} {size:.2} Tf <{}> Tj\n", run.0 + 1, run.1);
        }
        out + "ET"
    }
}

struct Writer<W: Write> {
    out: W,
    at: u64,
    offsets: Vec<u64>,
    lock: Option<Lock>,
}

impl<W: Write> Writer<W> {
    fn put(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.at += bytes.len() as u64;
        self.out.write_all(bytes)
    }

    fn dict(&mut self, dict: &str) -> std::io::Result<()> {
        self.offsets.push(self.at);
        let head = format!("{} 0 obj\n<< {dict} >>\nendobj\n", self.offsets.len());
        self.put(head.as_bytes())
    }

    fn stream(&mut self, dict: &str, data: &[u8]) -> std::io::Result<()> {
        self.offsets.push(self.at);
        let sealed;
        let data = match &self.lock {
            Some(l) => {
                sealed = l.seal(data)?;
                &sealed[..]
            }
            None => data,
        };
        let head = format!("{} 0 obj\n<< {dict} /Length {} >>\nstream\n", self.offsets.len(), data.len());
        self.put(head.as_bytes())?;
        self.put(data)?;
        self.put(b"\nendstream\nendobj\n")
    }
}

fn write_page<W: Write>(w: &mut Writer<W>, p: &PdfPage, id: usize, letters: &Letters, fonts: &str) -> std::io::Result<()> {
    let (pw, ph) = p.page;
    let font = if letters.is_empty() { String::new() } else { format!(" /Font << {fonts}>>") };
    w.dict(&format!("/Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2} {ph:.2}] /Resources << /XObject << /Im0 {} 0 R >>{font} >> /Contents {} 0 R", id + 2, id + 1))?;
    let (iw, ih) = (p.width as f32, p.height as f32);
    let s = (pw / iw).min(ph / ih);
    let (dw, dh) = (iw * s, ih * s);
    let (ox, oy) = ((pw - dw) / 2.0, (ph - dh) / 2.0);
    let mut stream = format!("q {dw:.2} 0 0 {dh:.2} {ox:.2} {oy:.2} cm /Im0 Do Q");
    if !p.words.is_empty() {
        stream += &letters.layer(&p.words, s, ox, oy, ih);
    }
    w.stream("", stream.as_bytes())?;
    match &p.mono {
        Some(bits) => w.stream(&format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /FlateDecode", p.width, p.height), bits)?,
        None => {
            let cs = if p.grey { "/DeviceGray" } else { "/DeviceRGB" };
            w.stream(&format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {cs} /BitsPerComponent 8 /Filter /DCTDecode", p.width, p.height), &p.jpeg)?;
        }
    }
    Ok(())
}

pub fn write(pages: &[PdfPage], opt: &Options, out: impl Write) -> std::io::Result<()> {
    let n = pages.len();
    let lock = if opt.password.is_empty() { None } else { Some(Lock::new(&opt.password)?) };
    let mut w = Writer { out, at: 0, offsets: Vec::new(), lock };
    w.put(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n")?;

    let letters = Letters::of(pages);
    w.dict(if w.lock.is_some() { "/Type /Catalog /Pages 2 0 R /Extensions << /ADBE << /BaseVersion /1.7 /ExtensionLevel 8 >> >>" } else { "/Type /Catalog /Pages 2 0 R" })?;
    let kids: String = (0..n).map(|i| format!("{} 0 R ", 4 + i * 3)).collect();
    w.dict(&format!("/Type /Pages /Kids [{kids}] /Count {n}"))?;
    let mut info = format!("/Title {} /Author {}", text(&opt.title, w.lock.as_ref())?, text(&opt.author, w.lock.as_ref())?);
    if !opt.subject.is_empty() {
        info += &format!(" /Subject {}", text(&opt.subject, w.lock.as_ref())?);
    }
    if !opt.keywords.is_empty() {
        info += &format!(" /Keywords {}", text(&opt.keywords, w.lock.as_ref())?);
    }
    w.dict(&info)?;
    let first_font = 4 + n * 3;
    let fonts: String = (0..letters.fonts()).map(|k| format!("/F{} {} 0 R ", k + 1, first_font + k * 2)).collect();
    for (i, p) in pages.iter().enumerate() {
        write_page(&mut w, p, 4 + i * 3, &letters, &fonts)?;
    }
    for k in 0..letters.fonts() {
        w.dict(&letters.font(k, first_font + k * 2 + 1))?;
        w.stream("", letters.to_unicode(k).as_bytes())?;
    }
    let mut trailer = String::new();
    if let Some(l) = &w.lock {
        let enc = w.offsets.len() + 1;
        let (dict, id) = (l.dictionary(), hex(&l.id));
        trailer = format!(" /Encrypt {enc} 0 R /ID [<{id}> <{id}>]");
        w.dict(&dict)?;
    }
    let xref = w.at;
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", w.offsets.len() + 1);
    for o in &w.offsets {
        table += &format!("{o:010} 00000 n \n");
    }
    table += &format!("trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R{trailer} >>\nstartxref\n{xref}\n%%EOF\n", w.offsets.len() + 1);
    w.put(table.as_bytes())?;
    w.out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_has_every_page_and_a_correct_table() {
        crate::words::ready();
        let page = || PdfPage { words: Vec::new(), jpeg: Arc::new(vec![0xFF, 0xD8, 0xFF, 0xD9]), mono: None, width: 10, height: 20, grey: false, page: (595.0, 842.0) };
        let mut out = Vec::new();
        write(&[page(), page()], &Options { title: "T (x)".into(), author: "A".into(), ..Default::default() }, &mut out).unwrap();
        let find = |h: &[u8], n: &[u8]| h.windows(n.len()).position(|w| w == n).unwrap();
        assert!(find(&out, b"/Count 2") > 0);
        assert!(find(&out, b"/Title (T \\(x\\))") > 0);
        let sx = find(&out, b"startxref\n");
        let xref_at: usize = String::from_utf8_lossy(&out[sx + 10..]).lines().next().unwrap().parse().unwrap();
        assert!(out[xref_at..].starts_with(b"xref"));
        let at = find(&out, b"4 0 obj");
        let table = String::from_utf8_lossy(&out[xref_at..]).into_owned();
        assert_eq!(table.lines().nth(6).unwrap()[..10].parse::<usize>().unwrap(), at);
    }

    #[test]
    fn subject_keywords_and_other_scripts_are_kept() {
        crate::words::ready();
        let page = PdfPage { words: Vec::new(), jpeg: Arc::new(vec![0xFF, 0xD8, 0xFF, 0xD9]), mono: None, width: 10, height: 20, grey: true, page: (595.0, 842.0) };
        let mut out = Vec::new();
        let o = Options { title: "Café".into(), subject: "Bills".into(), keywords: "a, b".into(), ..Default::default() };
        write(&[page], &o, &mut out).unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("/Subject (Bills)") && s.contains("/Keywords (a, b)"));
        assert!(s.contains("/Title <FEFF00430061006600E9>"));
    }

    #[test]
    fn words_are_written_as_invisible_searchable_text() {
        crate::words::ready();
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80).encode(&vec![255u8; 100 * 100 * 3], 100, 100, image::ExtendedColorType::Rgb8).unwrap();
        let word = |t: &str, x: f32| Word { text: t.into(), x, y: 40.0, w: 40.0, h: 10.0 };
        let page = PdfPage { words: vec![word("Hello", 5.0), word("w\u{f6}rld", 50.0)], jpeg: Arc::new(jpeg), mono: None, width: 100, height: 100, grey: false, page: (200.0, 200.0) };
        let mut out = Vec::new();
        write(&[page], &Options { title: "t".into(), ..Options::default() }, &mut out).unwrap();
        let dir = crate::testdir::tmp("pdf");
        let file = dir.join("text.pdf");
        std::fs::write(&file, &out).unwrap();
        let said = pdftotext(&file, None).expect("pdftotext (poppler-utils) is needed for this test");
        assert!(said.contains("Hello") && said.contains("w\u{f6}rld"), "pdftotext said {said:?}");
    }

    fn pdftotext(file: &std::path::Path, password: Option<&str>) -> Option<String> {
        let mut c = std::process::Command::new("pdftotext");
        if let Some(p) = password {
            c.arg("-upw").arg(p);
        }
        let r = c.arg(file).arg("-").output().expect("pdftotext (poppler-utils) is needed for this test");
        r.status.success().then(|| String::from_utf8_lossy(&r.stdout).into_owned())
    }

    fn white_jpeg() -> Vec<u8> {
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80).encode(&vec![255u8; 100 * 100 * 3], 100, 100, image::ExtendedColorType::Rgb8).unwrap();
        jpeg
    }

    fn lettered(words: Vec<Word>) -> PdfPage {
        PdfPage { words, jpeg: Arc::new(white_jpeg()), mono: None, width: 100, height: 100, grey: false, page: (200.0, 200.0) }
    }

    #[test]
    fn a_password_makes_an_aes_file_a_reader_opens_only_with_it() {
        crate::words::ready();
        let word = Word { text: "Secret".into(), x: 5.0, y: 40.0, w: 60.0, h: 10.0 };
        let o = Options { title: "Caf\u{e9} t".into(), password: "open sesame \u{e9}".into(), ..Options::default() };
        let mut out = Vec::new();
        write(&[lettered(vec![word])], &o, &mut out).unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("/AESV3") && s.contains("/R 6") && !s.contains("Secret"));
        let dir = crate::testdir::tmp("pdf");
        let file = dir.join("locked.pdf");
        std::fs::write(&file, &out).unwrap();
        assert!(pdftotext(&file, Some("open sesame \u{e9}")).unwrap().contains("Secret"));
        assert!(pdftotext(&file, Some("wrong")).is_none());
        assert!(pdftotext(&file, None).is_none());
    }

    #[test]
    fn more_than_250_different_letters_are_all_searchable() {
        crate::words::ready();
        let text: String = (0x4E00u32..0x4E00 + 400).filter_map(char::from_u32).collect();
        let all: Vec<char> = text.chars().collect();
        let words: Vec<Word> = all.chunks(8).enumerate().map(|(i, c)| Word { text: c.iter().collect(), x: 1.0, y: 2.0 * i as f32, w: 98.0, h: 2.0 }).collect();
        let mut out = Vec::new();
        write(&[lettered(words)], &Options::default(), &mut out).unwrap();
        let dir = crate::testdir::tmp("pdf");
        let file = dir.join("many.pdf");
        std::fs::write(&file, &out).unwrap();
        let said = pdftotext(&file, None).unwrap();
        assert_eq!(said.trim().chars().filter(|c| !c.is_whitespace()).collect::<String>(), text);
    }

    #[test]
    fn a_black_and_white_page_is_written_as_one_bit() {
        crate::words::ready();
        let img = image::DynamicImage::ImageLuma8(image::GrayImage::from_fn(64, 64, |x, _| image::Luma([if x < 32 { 0 } else { 255 }])));
        let mono = mono_of(&img).unwrap();
        assert!(mono.len() < 64 * 8, "deflated rows are smaller than raw ones");
        let page = PdfPage { words: Vec::new(), jpeg: Arc::new(vec![0xFF, 0xD8, 0xFF, 0xD9]), mono: Some(mono), width: 64, height: 64, grey: true, page: (200.0, 200.0) };
        let mut out = Vec::new();
        write(&[page], &Options::default(), &mut out).unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("/BitsPerComponent 1") && s.contains("/FlateDecode") && !s.contains("DCTDecode"));
    }
}
