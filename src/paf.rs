//! Reads a PAF 5 database file (.paf) directly, so people don't need PAF installed to export a GEDCOM first.
//!
//! Clean-room: the layout below was worked out only by comparing a user's own .paf file with the GEDCOM
//! exported from the same tree. No PAF program code was looked at for this. What isn't understood yet is
//! counted and reported, never guessed.
//!
//! Layout (all numbers little-endian):
//! * The file is made of 8 KB pages. Page i starts at 0x3000 + i * 0x2000. Page 0 is the page map: byte i
//!   gives the record type stored on page i.
//! * Each record type has a fixed size and records sit back to back on their pages with no page header.
//!   Record number r of a type is on that type's ((r-1) / per_page)-th page.
//! * Text (names, places, date phrases) lives in string pages. Records point to text by its file offset.
//!
//! The reader turns the file into GEDCOM text, and the normal GEDCOM import reads that. So a .paf goes through
//! exactly the same checks, report and loss test as a .ged file.

use std::collections::BTreeMap;
use std::fmt::Write as _;

const PAGE: usize = 0x2000;
const BASE: usize = 0x3000;

const T_INDI: u8 = 2;
const T_FAM: u8 = 3;
const T_EXTRA: u8 = 4;
const T_CHILD: u8 = 5;

fn size_of(t: u8) -> usize {
    match t {
        T_INDI => 221,
        T_FAM => 106,
        T_EXTRA => 256,
        T_CHILD => 46,
        _ => 0,
    }
}

/// True when the bytes look like a PAF database (any version).
pub fn is_paf(b: &[u8]) -> bool {
    b.len() > 12 && &b[8..12] == b"PAF\0" && b[..3].iter().all(|c| c.is_ascii_digit())
}

pub struct PafFile<'a> {
    b: &'a [u8],
    pages: BTreeMap<u8, Vec<usize>>,
}

impl<'a> PafFile<'a> {
    pub fn open(b: &'a [u8]) -> Result<Self, String> {
        if !is_paf(b) {
            return Err("This is not a PAF database file.".into());
        }
        if &b[..4] != b"500\0" {
            let v = String::from_utf8_lossy(&b[..3]).to_string();
            return Err(format!(
                "This .paf file was saved by an older PAF (file version {v}). Gilded Roots reads PAF 5 files directly. \
                 Open it in PAF and choose File > Export (GEDCOM), then import the .ged file here."
            ));
        }
        if b.len() < BASE + PAGE {
            return Err("This .paf file is too short. It may be damaged or only partly copied.".into());
        }
        let n = (b.len() - BASE) / PAGE;
        let mut pages: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
        for i in 0..n.min(PAGE) {
            pages.entry(b[BASE + i]).or_default().push(i);
        }
        if !pages.contains_key(&T_INDI) {
            return Err("This .paf file has no people in it.".into());
        }
        Ok(PafFile { b, pages })
    }

    fn count(&self, t: u8) -> usize {
        let s = size_of(t);
        self.pages.get(&t).map_or(0, |p| p.len() * (PAGE / s))
    }

    fn rec(&self, t: u8, r: u32) -> Option<&'a [u8]> {
        let s = size_of(t);
        if r == 0 || s == 0 {
            return None;
        }
        let per = PAGE / s;
        let r = r as usize - 1;
        let pg = *self.pages.get(&t)?.get(r / per)?;
        let off = BASE + pg * PAGE + (r % per) * s;
        self.b.get(off..off + s)
    }

    /// NUL-terminated text starting at a file offset.
    fn cstr(&self, at: usize) -> String {
        let Some(rest) = self.b.get(at..) else { return String::new() };
        let end = rest.iter().position(|&c| c == 0).unwrap_or(0).min(4096);
        latin1(&rest[..end])
    }

    /// A string-page entry: 12 bytes of tree links, a kind byte, then the text.
    fn heap_text(&self, ptr: u32) -> String {
        if ptr == 0 {
            return String::new();
        }
        self.cstr(ptr as usize + 13).trim().to_string()
    }

    /// A name entry: 8 bytes, the person number, a kind byte, a 16-bit length, then the text.
    fn name_text(&self, ptr: u32) -> String {
        let p = ptr as usize;
        if p == 0 {
            return String::new();
        }
        let n = self.b.get(p + 13..p + 15).map_or(0, |x| u16::from_le_bytes([x[0], x[1]]) as usize);
        self.b.get(p + 15..p + 15 + n).map(latin1).unwrap_or_default().trim().to_string()
    }
}

fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

fn u32at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map_or(0, |x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]))
}

const MONTHS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

/// Julian day number to a Gregorian (year, month, day).
fn from_jdn(n: i64) -> (i64, i64, i64) {
    let a = n + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - 146097 * b / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - 1461 * d / 4;
    let m = (5 * e + 2) / 153;
    (100 * b + d - 4800 + m / 10, m + 3 - 12 * (m / 10), e - (153 * m + 2) / 5 + 1)
}

#[derive(Default)]
pub struct Notes {
    /// Dates stored in a form this reader doesn't know yet. They are written as a full date and listed.
    pub odd_dates: usize,
    /// People or families with data in slots this reader doesn't understand yet.
    pub unread: usize,
    /// Child links marked with a relationship other than birth (adopted, sealed and so on), not decoded yet.
    pub odd_links: usize,
}

/// A 48-byte event: kind, date, place.
fn event(p: &PafFile, b: &[u8], o: usize, notes: &mut Notes) -> Option<(String, String)> {
    let kind = b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]]))?;
    let place = p.heap_text(u32at(b, o + 12));
    let date = match kind {
        0 => String::new(),
        // Date kept as typed text ("ABT. 1613", "WFT Est. 1655-1708").
        0x22 => p.heap_text(u32at(b, o + 2)),
        // A calendar date: flags say which parts are known.
        0x12 => {
            let flags = b[o + 3];
            let (y, m, d) = from_jdn((u32at(b, o + 4) & 0x00FF_FFFF) as i64);
            let mon = MONTHS[(m.clamp(1, 12) - 1) as usize];
            match flags {
                0x61 => format!("{y}"),
                0x21 => format!("{mon} {y}"),
                0x01 => format!("{d} {mon} {y}"),
                _ => {
                    notes.odd_dates += 1;
                    format!("{d} {mon} {y}")
                }
            }
        }
        _ => {
            notes.odd_dates += 1;
            String::new()
        }
    };
    if date.is_empty() && place.is_empty() {
        None
    } else {
        Some((date, place))
    }
}

fn put_event(out: &mut String, tag: &str, ev: Option<(String, String)>) {
    if let Some((d, pl)) = ev {
        let _ = writeln!(out, "1 {tag}");
        if !d.is_empty() {
            let _ = writeln!(out, "2 DATE {d}");
        }
        if !pl.is_empty() {
            let _ = writeln!(out, "2 PLAC {pl}");
        }
    }
}

fn live(b: &[u8], guid: usize) -> bool {
    b[guid..guid + 16].iter().any(|&c| c != 0)
}

fn uid(b: &[u8], at: usize) -> String {
    b[at..at + 16].iter().map(|c| format!("{c:02X}")).collect()
}

/// Turn a PAF 5 file into GEDCOM text.
pub fn to_gedcom(bytes: &[u8]) -> Result<(String, Notes), String> {
    let p = PafFile::open(bytes)?;
    let mut notes = Notes::default();

    // Extra GEDCOM lines PAF kept for tags it has no field for (for example _MEND). Owner number -> lines.
    let mut extra_fam: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut extra_indi: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for r in 1..=p.count(T_EXTRA) as u32 {
        let Some(b) = p.rec(T_EXTRA, r) else { break };
        if b.iter().all(|&c| c == 0) {
            continue;
        }
        let owner = u32at(b, 8);
        let text = p.cstr(b.as_ptr() as usize - bytes.as_ptr() as usize + 14);
        let lines: Vec<String> = text
            .lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| !l.is_empty())
            .collect();
        match b[12] {
            b'M' => extra_fam.entry(owner).or_default().extend(lines),
            b'I' => extra_indi.entry(owner).or_default().extend(lines),
            _ => notes.unread += 1,
        }
    }

    let mut out = String::new();
    out.push_str("0 HEAD\n1 SOUR PAF\n2 NAME Personal Ancestral File (read directly by Gilded Roots)\n2 VERS 5\n1 GEDC\n2 VERS 5.5\n2 FORM LINEAGE-LINKED\n1 CHAR ANSI\n");

    // Families first, so each person's own families can be listed in PAF's order.
    let mut fams: BTreeMap<u32, &[u8]> = BTreeMap::new();
    for r in 1..=p.count(T_FAM) as u32 {
        if let Some(b) = p.rec(T_FAM, r) {
            if live(b, 36) {
                fams.insert(r, b);
            }
        }
    }

    for r in 1..=p.count(T_INDI) as u32 {
        let Some(b) = p.rec(T_INDI, r) else { break };
        if !live(b, 48) {
            continue;
        }
        let _ = writeln!(out, "0 @I{r}@ INDI");
        let name = p.name_text(u32at(b, 0));
        if !name.is_empty() {
            let _ = writeln!(out, "1 NAME {name}");
        }
        let sex = match b[202] {
            b'M' => "M",
            b'F' => "F",
            _ => "U",
        };
        let _ = writeln!(out, "1 SEX {sex}");
        put_event(&mut out, "BIRT", event(&p, b, 64, &mut notes));
        put_event(&mut out, "DEAT", event(&p, b, 112, &mut notes));
        // Own families: a chain through each family's "next family for the husband / wife" fields.
        let mut f = u32at(b, 4);
        let mut seen = Vec::new();
        while f != 0 && !seen.contains(&f) && seen.len() < 200 {
            seen.push(f);
            let _ = writeln!(out, "1 FAMS @F{f}@");
            let Some(fb) = fams.get(&f) else { break };
            f = if u32at(fb, 4) == r { u32at(fb, 16) } else { u32at(fb, 20) };
        }
        // Parent families: a ring of child links.
        let first = u32at(b, 8);
        let mut l = first;
        let mut n = 0;
        while l != 0 && n < 50 {
            let Some(cb) = p.rec(T_CHILD, l) else { break };
            let _ = writeln!(out, "1 FAMC @F{}@", u32at(cb, 0));
            // 'b' is a birth child, the default. Other relationships haven't been seen in a sample file yet.
            if cb[37] != b'b' && cb[37] != 0 {
                notes.odd_links += 1;
            }
            l = u32at(cb, 8);
            n += 1;
            if l == first {
                break;
            }
        }
        for line in extra_indi.get(&r).into_iter().flatten() {
            let _ = writeln!(out, "{line}");
        }
        let _ = writeln!(out, "1 _UID {}", uid(b, 48));
        let _ = writeln!(out, "1 RIN {r}");
        if b[12..20].iter().any(|&c| c != 0) {
            notes.unread += 1;
        }
    }

    for (&r, b) in &fams {
        let _ = writeln!(out, "0 @F{r}@ FAM");
        let h = u32at(b, 4);
        let w = u32at(b, 8);
        if h != 0 {
            let _ = writeln!(out, "1 HUSB @I{h}@");
        }
        if w != 0 {
            let _ = writeln!(out, "1 WIFE @I{w}@");
        }
        // Children: a ring of child links in birth order.
        let first = u32at(b, 12);
        let mut l = first;
        let mut n = 0;
        while l != 0 && n < 100 {
            let Some(cb) = p.rec(T_CHILD, l) else { break };
            let _ = writeln!(out, "1 CHIL @I{}@", u32at(cb, 12));
            l = u32at(cb, 4);
            n += 1;
            if l == first {
                break;
            }
        }
        put_event(&mut out, "MARR", event(&p, b, 52, &mut notes));
        for line in extra_fam.get(&r).into_iter().flatten() {
            let _ = writeln!(out, "{line}");
        }
        let _ = writeln!(out, "1 _UID {}", uid(b, 36));
        let _ = writeln!(out, "1 RIN {r}");
    }
    out.push_str("0 TRLR\n");
    Ok((out, notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jdn_dates() {
        assert_eq!(from_jdn(2_299_161), (1582, 10, 15));
        assert_eq!(from_jdn(2_451_545), (2000, 1, 1));
    }

    #[test]
    fn rejects_other_files() {
        assert!(!is_paf(b"0 HEAD\n1 SOUR X\n"));
        assert!(PafFile::open(b"400\0400\0PAF\0xxxxxxxxxxxxxxx").err().unwrap().contains("older PAF"));
    }

    /// Checks against a real PAF 5 file and the GEDCOM PAF exported from it, when both are given.
    /// GR_PAF=/path/file.paf GR_PAF_GED=/path/file.ged cargo test paf_matches_export -- --ignored
    #[test]
    #[ignore]
    fn paf_matches_export() {
        let paf = std::fs::read(std::env::var("GR_PAF").unwrap()).unwrap();
        let ged = std::fs::read(std::env::var("GR_PAF_GED").unwrap()).unwrap();
        let (text, notes) = to_gedcom(&paf).unwrap();
        let (a, _) = crate::gedcom::parse_ged_report(&text);
        let (b, _) = crate::gedcom::parse_ged_report(&String::from_utf8_lossy(&ged));
        assert_eq!(a.people.len(), b.people.len());
        assert_eq!(a.fams.len(), b.fams.len());
        assert_eq!(notes.odd_dates, 0);
        println!("{} people, {} families", a.people.len(), a.fams.len());
    }
}

#[cfg(test)]
mod full_import {
    /// GR_PAF=... GR_PAF_GED=... cargo test paf_full_import -- --ignored --nocapture
    #[test]
    #[ignore]
    fn paf_full_import() {
        let a = crate::read_ged_file(std::path::Path::new(&std::env::var("GR_PAF").unwrap())).unwrap();
        let b = crate::read_ged_file(std::path::Path::new(&std::env::var("GR_PAF_GED").unwrap())).unwrap();
        println!("program={} people={} fams={} warnings={:?} lost={:?}", a.1.program, a.1.people, a.1.families, a.1.warnings, a.1.lost.len());
        std::fs::write("/tmp/from_paf.ged", crate::to_ged(&a.0)).unwrap();
        std::fs::write("/tmp/from_ged.ged", crate::to_ged(&b.0)).unwrap();
    }
}
