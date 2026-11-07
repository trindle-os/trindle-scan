//! Talking to a network scanner over eSCL (AirScan): plain HTTP and a little XML, to private network addresses only.

use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use trindle_toolkit::t;

#[derive(Clone, Debug, PartialEq)]
pub struct Scanner {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub root: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Caps {
    pub model: String,

    pub glass: Option<(u32, u32)>,
    pub feeder: bool,
    pub two_sided: bool,
    pub resolutions: Vec<u32>,
    pub colour: bool,
    pub grey: bool,
    pub black_white: bool,
}

#[derive(Debug, PartialEq)]
pub enum Fail {

    Offline(String),
    Busy,
    NoPaper,
    Jam,
    NotPrivate,
    Stopped,
    Other(String),
}

impl Fail {
    pub fn words(&self) -> String {
        match self {
            Fail::Offline(w) => t!("scan-escl-offline", detail = w),
            Fail::Busy => t!("scan-escl-busy").into(),
            Fail::NoPaper => t!("scan-escl-no-paper").into(),
            Fail::Jam => t!("scan-escl-jam").into(),
            Fail::NotPrivate => t!("scan-escl-not-private").into(),
            Fail::Stopped => t!("scan-escl-stopped").into(),
            Fail::Other(w) => t!("scan-escl-other", detail = w),
        }
    }
}

pub struct Reply {
    pub status: u16,
    pub location: Option<String>,
    pub body: Vec<u8>,
}

pub fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(a) => a.is_private() || a.is_link_local(),
        IpAddr::V6(a) => (a.segments()[0] & 0xFFC0) == 0xFE80 || (a.segments()[0] & 0xFE00) == 0xFC00,
    }
}

#[cfg(not(test))]
fn may_connect(ip: IpAddr) -> bool {
    is_private(ip)
}

#[cfg(test)]
fn may_connect(ip: IpAddr) -> bool {
    is_private(ip) || ip.is_loopback()
}

fn plain(s: &str, allowed: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || allowed.contains(c))
}

const LONGEST_REPLY: usize = 512 << 20;

impl Scanner {
    fn base(&self) -> String {
        if self.root.is_empty() { String::new() } else { format!("/{}", self.root) }
    }

    pub fn request(&self, method: &str, path: &str, body: Option<&str>, secs: u64) -> Result<Reply, Fail> {
        self.request_until(method, path, body, secs, &|| false)
    }

    fn request_until(&self, method: &str, path: &str, body: Option<&str>, secs: u64, stop: &dyn Fn() -> bool) -> Result<Reply, Fail> {
        if !plain(path, "._~/-") || !plain(&self.host, ".:-[]%") {
            return Err(Fail::Other(t!("scan-escl-detail-unsafe").into()));
        }
        let addr: SocketAddr = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|e| Fail::Offline(e.to_string()))?
            .next()
            .ok_or_else(|| Fail::Offline(t!("scan-escl-detail-no-address").into()))?;
        if !may_connect(addr.ip()) {
            return Err(Fail::NotPrivate);
        }
        let off = |e: std::io::Error| Fail::Offline(e.to_string());
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(off)?;
        s.set_read_timeout(Some(Duration::from_secs(1))).map_err(off)?;
        s.set_write_timeout(Some(Duration::from_secs(10))).map_err(off)?;
        let body = body.unwrap_or("");
        let mut req = format!("{method} {path} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\nUser-Agent: trindle-scan\r\n", self.host, self.port);
        if !body.is_empty() {
            req.push_str(&format!("Content-Type: text/xml\r\nContent-Length: {}\r\n", body.len()));
        }
        req.push_str("\r\n");
        req.push_str(body);
        s.write_all(req.as_bytes()).map_err(off)?;
        let end = Instant::now() + Duration::from_secs(secs * 2 + 30);
        let idle = Duration::from_secs(secs);
        let mut last = Instant::now();
        let mut raw = Vec::new();
        let mut buf = [0u8; 65536];
        loop {
            if stop() {
                return Err(Fail::Stopped);
            }
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    raw.extend_from_slice(&buf[..n]);
                    last = Instant::now();
                    if raw.len() > LONGEST_REPLY {
                        return Err(Fail::Other(t!("scan-escl-detail-broken").into()));
                    }
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {
                    if last.elapsed() > idle {
                        return Err(off(e));
                    }
                }
                Err(e) => return Err(off(e)),
            }
            if Instant::now() > end {
                return Err(Fail::Offline(t!("scan-escl-detail-too-slow").into()));
            }
        }
        parse_reply(&raw)
    }

    pub fn usb_device(&self) -> Option<&str> {
        self.host.strip_prefix(crate::sane::PREFIX)
    }

    pub fn capabilities(&self) -> Result<Caps, Fail> {
        if let Some(device) = self.usb_device() {
            return crate::sane::capabilities(device);
        }
        let r = self.request("GET", &format!("{}/ScannerCapabilities", self.base()), None, 15)?;
        if r.status != 200 {
            return Err(Fail::Other(t!("scan-escl-detail-answered", code = r.status)));
        }
        let c = parse_caps(&String::from_utf8_lossy(&r.body));
        if c.glass.is_none() && !c.feeder {
            return Err(Fail::Other(t!("scan-escl-detail-no-caps").into()));
        }
        Ok(c)
    }

    pub fn status(&self) -> Result<(String, String), Fail> {
        let r = self.request("GET", &format!("{}/ScannerStatus", self.base()), None, 10)?;
        let x = String::from_utf8_lossy(&r.body).into_owned();
        Ok((first(&x, "State").unwrap_or_default(), first(&x, "AdfState").unwrap_or_default()))
    }

    pub fn scan(&self, s: &Settings, stop: impl Fn() -> bool, mut page: impl FnMut(Vec<u8>)) -> Result<usize, Fail> {
        if let Some(device) = self.usb_device() {
            return crate::sane::scan(device, s, stop, page);
        }
        let r = self.request("POST", &format!("{}/ScanJobs", self.base()), Some(&s.xml()), 30)?;
        match r.status {
            200..=299 => {}
            503 => return Err(self.why_busy()),
            code => return Err(Fail::Other(t!("scan-escl-detail-refused", code = code))),
        }
        let job = r.location.ok_or_else(|| Fail::Other(t!("scan-escl-detail-no-job").into()))?;
        let job = job.split_once("://").map(|(_, rest)| rest.find('/').map(|i| rest[i..].to_string()).unwrap_or_default()).unwrap_or(job);
        let mut n = 0;
        let end = loop {
            if stop() {
                break Ok(n);
            }
            match self.request_until("GET", &format!("{job}/NextDocument"), None, 180, &stop) {
                Ok(r) => match r.status {
                    200 => {
                        n += 1;
                        page(r.body);
                    }
                    404 | 410 => return Ok(n),
                    503 if n > 0 => return Ok(n),
                    503 => break Err(self.why_busy()),
                    code => break Err(Fail::Other(t!("scan-escl-detail-stopped", code = code))),
                },
                Err(Fail::Stopped) => break Ok(n),
                Err(f) => break Err(f),
            }
        };
        let _ = self.request("DELETE", &job, None, 5);
        end
    }

    fn why_busy(&self) -> Fail {
        match self.status() {
            Ok((_, adf)) if adf.contains("Empty") => Fail::NoPaper,
            Ok((_, adf)) if adf.contains("Jam") => Fail::Jam,
            _ => Fail::Busy,
        }
    }
}

pub const RGB: &str = "RGB24";
pub const GREY: &str = "Grayscale8";
pub const BLACK_WHITE: &str = "BlackAndWhite1";

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {

    pub width: u32,
    pub height: u32,
    pub dpi: u32,

    pub mode: &'static str,
    pub feeder: bool,
    pub duplex: bool,
}

impl Settings {
    pub fn xml(&self) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<scan:ScanSettings xmlns:pwg=\"http://www.pwg.org/schemas/2010/12/sm\" xmlns:scan=\"http://schemas.hp.com/imaging/escl/2011/05/03\"><pwg:Version>2.0</pwg:Version><pwg:ScanRegions><pwg:ScanRegion><pwg:Height>{}</pwg:Height><pwg:ContentRegionUnits>escl:ThreeHundredthsOfInches</pwg:ContentRegionUnits><pwg:Width>{}</pwg:Width><pwg:XOffset>0</pwg:XOffset><pwg:YOffset>0</pwg:YOffset></pwg:ScanRegion></pwg:ScanRegions><pwg:InputSource>{}</pwg:InputSource><scan:ColorMode>{}</scan:ColorMode><scan:XResolution>{}</scan:XResolution><scan:YResolution>{}</scan:YResolution><pwg:DocumentFormat>image/jpeg</pwg:DocumentFormat>{}</scan:ScanSettings>",
            self.height, self.width, if self.feeder { "Feeder" } else { "Platen" }, self.mode, self.dpi, self.dpi,
            if self.feeder && self.duplex { "<scan:Duplex>true</scan:Duplex>" } else { "" }
        )
    }
}

fn parse_reply(raw: &[u8]) -> Result<Reply, Fail> {
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| Fail::Other(t!("scan-escl-detail-broken").into()))?;
    let head = String::from_utf8_lossy(&raw[..end]).into_owned();
    let mut lines = head.lines();
    let status = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse().ok()).ok_or_else(|| Fail::Other(t!("scan-escl-detail-broken").into()))?;
    let mut location = None;
    let mut chunked = false;
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            match k.trim().to_ascii_lowercase().as_str() {
                "location" => location = Some(v.trim().to_string()),
                "transfer-encoding" => chunked = v.to_ascii_lowercase().contains("chunked"),
                _ => {}
            }
        }
    }
    let rest = &raw[end + 4..];
    let body = if chunked { dechunk(rest) } else { rest.to_vec() };
    Ok(Reply { status, location, body })
}

fn dechunk(mut d: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(i) = d.windows(2).position(|w| w == b"\r\n") {
        let Ok(size) = usize::from_str_radix(String::from_utf8_lossy(&d[..i]).split(';').next().unwrap_or("").trim(), 16) else { break };
        let start = i + 2;
        let Some(stop) = start.checked_add(size) else { break };
        if size == 0 {
            break;
        }
        let Some(chunk) = d.get(start..stop) else {
            out.extend_from_slice(&d[start..]);
            break;
        };
        out.extend_from_slice(chunk);
        d = d.get(stop.saturating_add(2)..).unwrap_or(&[]);
    }
    out
}

pub fn all(xml: &str, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(lt) = xml[i..].find('<') {
        let s = i + lt;
        let Some(gt) = xml[s..].find('>') else { break };
        let inner = &xml[s + 1..s + gt];
        let name = inner.split_whitespace().next().unwrap_or("");
        let bare = name.rsplit(':').next().unwrap_or("");
        if bare == tag && !name.starts_with('/') && !inner.ends_with('/') {
            let close = xml[s + gt + 1..].find(&format!("</{}>", name)).map(|c| s + gt + 1 + c);
            if let Some(c) = close {
                out.push(xml[s + gt + 1..c].to_string());
            }
        }
        i = s + gt + 1;
    }
    out
}

fn first(xml: &str, tag: &str) -> Option<String> {
    all(xml, tag).into_iter().next().map(|s| s.trim().to_string())
}

pub fn parse_caps(xml: &str) -> Caps {
    let mut c = Caps { model: first(xml, "MakeAndModel").unwrap_or_default(), ..Caps::default() };
    if let Some(p) = all(xml, "PlatenInputCaps").first()
        && let (Some(w), Some(h)) = (first(p, "MaxWidth"), first(p, "MaxHeight")) {
            c.glass = w.parse().ok().zip(h.parse().ok());
        }
    let adf = all(xml, "Adf");
    c.feeder = !adf.is_empty();
    c.two_sided = adf.iter().any(|a| a.contains("AdfDuplexInputCaps"));
    let modes = all(xml, "ColorMode").join(" ");
    c.colour = modes.contains(RGB);
    c.grey = modes.contains(GREY);
    c.black_white = modes.contains(BLACK_WHITE);
    let mut res: Vec<u32> = all(xml, "XResolution").iter().filter_map(|r| r.trim().parse().ok()).collect();
    if res.is_empty() {
        let lo = first(xml, "Min").and_then(|v| v.parse::<u32>().ok());
        let hi = first(xml, "Max").and_then(|v| v.parse::<u32>().ok());
        if let (Some(lo), Some(hi)) = (lo, hi) {
            res = [75, 100, 150, 200, 300, 400, 600, 1200].into_iter().filter(|r| *r >= lo && *r <= hi).collect();
        }
    }
    res.sort_unstable();
    res.dedup();
    c.resolutions = res;
    c
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::net::TcpListener;

    pub(crate) const CAPS: &str = r#"<?xml version="1.0"?><scan:ScannerCapabilities xmlns:pwg="p" xmlns:scan="s"><pwg:Version>2.0</pwg:Version><pwg:MakeAndModel>Test Scanner 9</pwg:MakeAndModel>
<scan:Platen><scan:PlatenInputCaps><scan:MaxWidth>2550</scan:MaxWidth><scan:MaxHeight>3507</scan:MaxHeight>
<scan:SettingProfiles><scan:SettingProfile><scan:ColorModes><scan:ColorMode>BlackAndWhite1</scan:ColorMode><scan:ColorMode>Grayscale8</scan:ColorMode><scan:ColorMode>RGB24</scan:ColorMode></scan:ColorModes>
<scan:SupportedResolutions><scan:DiscreteResolutions><scan:DiscreteResolution><scan:XResolution>300</scan:XResolution><scan:YResolution>300</scan:YResolution></scan:DiscreteResolution><scan:DiscreteResolution><scan:XResolution>600</scan:XResolution><scan:YResolution>600</scan:YResolution></scan:DiscreteResolution></scan:DiscreteResolutions></scan:SupportedResolutions></scan:SettingProfile></scan:SettingProfiles></scan:PlatenInputCaps></scan:Platen>
<scan:Adf><scan:AdfSimplexInputCaps><scan:MaxWidth>2550</scan:MaxWidth></scan:AdfSimplexInputCaps><scan:AdfDuplexInputCaps><scan:MaxWidth>2550</scan:MaxWidth></scan:AdfDuplexInputCaps></scan:Adf></scan:ScannerCapabilities>"#;

    #[test]
    fn capabilities_are_read() {
        crate::words::ready();
        let c = parse_caps(CAPS);
        assert_eq!(c.model, "Test Scanner 9");
        assert_eq!(c.glass, Some((2550, 3507)));
        assert!(c.feeder && c.two_sided && c.colour && c.grey && c.black_white);
        assert_eq!(c.resolutions, vec![300, 600]);
    }

    #[test]
    fn only_local_addresses() {
        crate::words::ready();
        let s = Scanner { name: "x".into(), host: "8.8.8.8".into(), port: 80, root: "eSCL".into() };
        assert_eq!(s.capabilities().unwrap_err(), Fail::NotPrivate);
    }

    pub(crate) fn fake(pages: usize) -> u16 {
        fake_with(pages, vec![0xFF, 0xD8, 0xFF, 0xD9])
    }

    pub(crate) fn fake_with(pages: usize, jpeg: Vec<u8>) -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let mut left = pages;
            for s in l.incoming() {
                let mut s = s.unwrap();
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]).into_owned();
                let line = req.lines().next().unwrap_or("").to_string();
                let out: Vec<u8> = if line.starts_with("GET /eSCL/ScannerCapabilities") {
                    format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{CAPS}", CAPS.len()).into_bytes()
                } else if line.starts_with("POST /eSCL/ScanJobs") {
                    assert!(req.contains("<scan:XResolution>300<"));
                    left = pages;
                    b"HTTP/1.1 201 Created\r\nLocation: http://127.0.0.1/eSCL/ScanJobs/7\r\nContent-Length: 0\r\n\r\n".to_vec()
                } else if line.starts_with("GET /eSCL/ScanJobs/7/NextDocument") {
                    if left > 0 {
                        left -= 1;
                        let mut v = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", jpeg.len()).into_bytes();
                        v.extend_from_slice(&jpeg);
                        v
                    } else {
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec()
                    }
                } else {
                    b"HTTP/1.1 500 Oops\r\nContent-Length: 0\r\n\r\n".to_vec()
                };
                s.write_all(&out).unwrap();
            }
        });
        port
    }

    #[test]
    fn a_scan_brings_every_page() {
        crate::words::ready();
        let s = Scanner { name: "x".into(), host: "127.0.0.1".into(), port: fake(2), root: "eSCL".into() };
        assert!(s.capabilities().unwrap().feeder);
        let set = Settings { width: 2480, height: 3508, dpi: 300, mode: "RGB24", feeder: true, duplex: false };
        let mut got = Vec::new();
        let n = s.scan(&set, || false, |p| got.push(p)).unwrap();
        assert_eq!((n, got.len()), (2, 2));
    }

    #[test]
    fn a_hostile_chunk_size_ends_the_body_without_a_panic() {
        assert_eq!(dechunk(b"ffffffffffffffff\r\nabc"), b"");
        assert_eq!(dechunk(b"zz\r\nabc"), b"");
        assert_eq!(dechunk(b"3\r\nabc\r\n10\r\nde"), b"abcde");
    }

    #[test]
    fn line_breaks_in_a_scanner_path_never_reach_the_wire() {
        crate::words::ready();
        let s = Scanner { name: "x".into(), host: "127.0.0.1".into(), port: 9, root: "eSCL\r\nHost: x\r\n\r\nGET /evil".into() };
        assert!(matches!(s.capabilities(), Err(Fail::Other(_))));
        let s = Scanner { name: "x".into(), host: "127.0.0.1\r\n".into(), port: 9, root: "eSCL".into() };
        assert!(matches!(s.capabilities(), Err(Fail::Other(_))));
    }

    #[test]
    fn a_stop_ends_a_request_that_is_waiting() {
        crate::words::ready();
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let _keep = std::thread::spawn(move || {
            let c = l.accept();
            std::thread::sleep(Duration::from_secs(5));
            drop(c);
        });
        let s = Scanner { name: "x".into(), host: "127.0.0.1".into(), port, root: "eSCL".into() };
        let t = Instant::now();
        let r = s.request_until("GET", "/eSCL/x", None, 180, &|| t.elapsed() > Duration::from_millis(1500));
        assert_eq!(r.err(), Some(Fail::Stopped));
        assert!(t.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn chunked_bodies_are_joined() {
        crate::words::ready();
        let r = parse_reply(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n").unwrap();
        assert_eq!(r.body, b"abcde");
    }
}
