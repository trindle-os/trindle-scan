//! Finding network scanners that advertise themselves (eSCL, AirScan) with one multicast question, sent only when a search starts.

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

pub const SERVICE: &str = "_uscan._tcp.local";
const MOST_HEARD: usize = 64;
const LONGEST_NAME: usize = 253;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub root: String,
}

#[derive(Debug, PartialEq)]
enum Data {
    Ptr(String),
    Srv(u16, String),
    Txt(Vec<(String, String)>),
    A(Ipv4Addr),
}

#[derive(Debug)]
struct Record {
    name: String,
    data: Data,
}

pub fn question(name: &str, kind: u16) -> Vec<u8> {
    let name = if name.len() > LONGEST_NAME { "" } else { name };
    let mut q = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for part in name.split('.') {
        let part = &part.as_bytes()[..part.len().min(63)];
        q.push(part.len() as u8);
        q.extend_from_slice(part);
    }
    q.push(0);
    q.extend_from_slice(&kind.to_be_bytes());

    q.extend_from_slice(&0x8001u16.to_be_bytes());
    q
}

fn name_at(p: &[u8], mut at: usize) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut end = None;
    let mut hops = 0;
    loop {
        let len = *p.get(at)? as usize;
        if len == 0 {
            return Some((out, end.unwrap_or(at + 1)));
        }
        if len & 0xC0 == 0xC0 {
            let low = *p.get(at + 1)? as usize;
            end.get_or_insert(at + 2);
            at = ((len & 0x3F) << 8) | low;
            hops += 1;
            if hops > 16 {
                return None;
            }
            continue;
        }
        let label = p.get(at + 1..at + 1 + len)?;
        if !out.is_empty() {
            out.push('.');
        }
        out.push_str(&String::from_utf8_lossy(label));
        at += 1 + len;
    }
}

fn parse(p: &[u8]) -> Vec<Record> {
    let mut out = Vec::new();
    if p.len() < 12 {
        return out;
    }
    let be = |i: usize| u16::from_be_bytes([p[i], p[i + 1]]) as usize;
    let (qd, others) = (be(4), be(6) + be(8) + be(10));
    let mut at = 12;
    for _ in 0..qd {
        let Some((_, next)) = name_at(p, at) else { return out };
        at = next + 4;
    }
    for _ in 0..others {
        let Some((name, next)) = name_at(p, at) else { return out };
        if next + 10 > p.len() {
            return out;
        }
        let kind = u16::from_be_bytes([p[next], p[next + 1]]);
        let len = u16::from_be_bytes([p[next + 8], p[next + 9]]) as usize;
        let start = next + 10;
        let Some(rd) = p.get(start..start + len) else { return out };
        at = start + len;
        let data = match kind {
            12 => name_at(p, start).map(|(n, _)| Data::Ptr(n)),
            33 if len > 6 => name_at(p, start + 6).map(|(n, _)| Data::Srv(u16::from_be_bytes([rd[4], rd[5]]), n)),
            16 => {
                let mut pairs = Vec::new();
                let mut i = 0;
                while i < rd.len() {
                    let l = rd[i] as usize;
                    let Some(s) = rd.get(i + 1..i + 1 + l) else { break };
                    let s = String::from_utf8_lossy(s);
                    if let Some((k, v)) = s.split_once('=') {
                        pairs.push((k.to_ascii_lowercase(), v.to_string()));
                    }
                    i += 1 + l;
                }
                Some(Data::Txt(pairs))
            }
            1 if len == 4 => Some(Data::A(Ipv4Addr::new(rd[0], rd[1], rd[2], rd[3]))),
            _ => None,
        };
        if let Some(data) = data {
            out.push(Record { name: name.to_ascii_lowercase(), data });
        }
    }
    out
}

type Instance = (Option<(u16, String)>, Vec<(String, String)>);

#[derive(Default)]
pub struct Heard {
    instances: BTreeMap<String, Instance>,
    addresses: BTreeMap<String, Ipv4Addr>,

    names: BTreeMap<String, String>,
    pub shown: Vec<String>,
}

impl Heard {
    pub fn take(&mut self, packet: &[u8], from: Ipv4Addr) -> Vec<Found> {
        if !crate::escl::is_private(IpAddr::V4(from)) {
            return Vec::new();
        }
        for r in parse(packet) {
            if r.name.len() > LONGEST_NAME {
                continue;
            }
            match r.data {
                Data::Ptr(inst) if r.name == SERVICE && inst.len() <= LONGEST_NAME && (self.instances.len() < MOST_HEARD || self.instances.contains_key(&inst.to_ascii_lowercase())) => {
                    self.names.entry(inst.to_ascii_lowercase()).or_insert_with(|| inst.clone());
                    self.instances.entry(inst.to_ascii_lowercase()).or_default();
                }
                Data::Srv(port, target) if self.instances.contains_key(&r.name) || self.instances.len() < MOST_HEARD => {
                    self.instances.entry(r.name).or_default().0 = Some((port, target.to_ascii_lowercase()));
                }
                Data::Txt(t) if self.instances.contains_key(&r.name) || self.instances.len() < MOST_HEARD => self.instances.entry(r.name).or_default().1 = t,
                Data::A(a) if a == from && (self.addresses.len() < MOST_HEARD || self.addresses.contains_key(&r.name)) => {
                    self.addresses.insert(r.name, a);
                }
                _ => {}
            }
        }
        let mut fresh = Vec::new();
        for (inst, (srv, txt)) in &self.instances {
            let Some((port, target)) = srv else { continue };
            let Some(addr) = self.addresses.get(target) else { continue };
            if self.shown.contains(inst) {
                continue;
            }
            let get = |k: &str| txt.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
            let suffix = format!(".{SERVICE}");
            let friendly = match self.names.get(inst) {
                Some(n) if n.len() >= suffix.len() && n.is_char_boundary(n.len() - suffix.len()) && n[n.len() - suffix.len()..].eq_ignore_ascii_case(&suffix) => {
                    n[..n.len() - suffix.len()].to_string()
                }
                _ => inst.strip_suffix(&suffix).unwrap_or(inst).to_string(),
            };
            let mut root = get("rs").unwrap_or_else(|| "eSCL".into());
            root = root.trim_matches('/').to_string();
            fresh.push(Found { name: friendly, host: addr.to_string(), port: *port, root });
            self.shown.push(inst.clone());
        }
        fresh
    }

    pub fn wanting(&self) -> Vec<String> {
        self.instances.values().filter_map(|(s, _)| s.as_ref().map(|(_, t)| t.clone())).filter(|t| !self.addresses.contains_key(t)).collect()
    }
}

pub fn search(secs: u64, stop: impl Fn() -> bool, mut found: impl FnMut(Found)) -> std::io::Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_read_timeout(Some(Duration::from_millis(250)))?;
    sock.set_multicast_ttl_v4(255).ok();
    let to = SocketAddr::from((Ipv4Addr::new(224, 0, 0, 251), 5353));
    let end = Instant::now() + Duration::from_secs(secs);
    let mut heard = Heard::default();
    let mut next_ask = Instant::now();
    let mut buf = [0u8; 9000];
    while Instant::now() < end && !stop() {
        if Instant::now() >= next_ask {
            sock.send_to(&question(SERVICE, 12), to)?;
            for t in heard.wanting() {
                sock.send_to(&question(&t, 1), to)?;
            }
            next_ask = Instant::now() + Duration::from_secs(2);
        }
        if let Ok((n, SocketAddr::V4(from))) = sock.recv_from(&mut buf) {
            for f in heard.take(&buf[..n], *from.ip()) {
                found(f);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(out: &mut Vec<u8>, n: &str) {
        for part in n.split('.') {
            out.push(part.len() as u8);
            out.extend_from_slice(part.as_bytes());
        }
        out.push(0);
    }
    fn rec(out: &mut Vec<u8>, n: &str, kind: u16, rd: &[u8]) {
        name(out, n);
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&[0, 1, 0, 0, 0, 120]);
        out.extend_from_slice(&(rd.len() as u16).to_be_bytes());
        out.extend_from_slice(rd);
    }

    #[test]
    fn a_scanner_is_put_together_from_its_records() {
        crate::words::ready();
        let mut p = vec![0, 0, 0x84, 0, 0, 0, 0, 4, 0, 0, 0, 0];
        let mut ptr = Vec::new();
        name(&mut ptr, "Office printer._uscan._tcp.local");
        rec(&mut p, SERVICE, 12, &ptr);
        let mut srv = vec![0, 0, 0, 0, 0x1F, 0x90];
        name(&mut srv, "office.local");
        rec(&mut p, "Office printer._uscan._tcp.local", 33, &srv);
        let txt = b"\x07rs=eSCL\x08ty=Model";
        rec(&mut p, "Office printer._uscan._tcp.local", 16, txt);
        rec(&mut p, "office.local", 1, &[192, 168, 1, 40]);
        let mut h = Heard::default();
        let office = Ipv4Addr::new(192, 168, 1, 40);
        let got = h.take(&p, office);
        assert_eq!(got, vec![Found { name: "Office printer".into(), host: "192.168.1.40".into(), port: 8080, root: "eSCL".into() }]);
        assert!(h.take(&p, office).is_empty(), "the same scanner is shown once");
        let mut other = Heard::default();
        assert!(other.take(&p, Ipv4Addr::new(192, 168, 1, 99)).is_empty(), "an address record from someone else is not believed");
        assert!(other.take(&p, Ipv4Addr::new(8, 8, 8, 8)).is_empty(), "a public sender is not believed");
    }

    #[test]
    fn a_hostile_flood_of_names_is_capped() {
        let mut h = Heard::default();
        for i in 0..200 {
            let mut p = vec![0, 0, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, 0];
            let mut ptr = Vec::new();
            name(&mut ptr, &format!("printer{i}._uscan._tcp.local"));
            rec(&mut p, SERVICE, 12, &ptr);
            h.take(&p, Ipv4Addr::new(192, 168, 1, 40));
        }
        assert!(h.instances.len() <= MOST_HEARD);
    }

    #[test]
    fn rubbish_does_not_panic() {
        crate::words::ready();
        let mut h = Heard::default();
        assert!(h.take(&[1, 2, 3], Ipv4Addr::new(192, 168, 1, 40)).is_empty());
        assert!(h.take(&[0u8; 40], Ipv4Addr::new(192, 168, 1, 40)).is_empty());
        let mut loopy = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0xC0, 12, 0, 1, 0, 1];
        assert!(h.take(&loopy, Ipv4Addr::new(192, 168, 1, 40)).is_empty());
        loopy.truncate(14);
        assert!(h.take(&loopy, Ipv4Addr::new(192, 168, 1, 40)).is_empty());
    }
}
