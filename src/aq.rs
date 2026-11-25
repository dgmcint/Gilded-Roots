//! Reads an Ancestral Quest database (.aq, and the .aqz backups that hold one) directly.
//!
//! Clean-room: the layout below was worked out only by comparing test databases with GEDCOM files of the
//! same trees (made up for the purpose). No Ancestral Quest program code was looked at. What isn't
//! understood yet is counted and reported, never guessed.
//!
//! Layout (all numbers little-endian):
//! * The header starts "AQ\0\0". The file is then made of 8 KB pages: page k holds data from
//!   0x800 + k*0x2000 for 0x1ffa bytes, and its last 6 bytes are `41 TT ff ff 00 00`, TT being the record
//!   type kept on that page. Page 1 (0x2800) also lists the types: byte i is the type of page i + 1.
//! * Records of a type sit back to back on that type's pages, in page order; record n (from 1) is on the
//!   type's ((n-1) / per_page)-th page.
//! * Words (names, place parts, captions, date phrases) are kept once each in "NAMZ" nodes, and records
//!   point to them by file offset. Long texts (notes, a citation's page) are "QNTZ" records.
//!
//! The reader turns the file into GEDCOM text, and the normal GEDCOM import reads that, so an .aq goes
//! through the same checks, report and import options as a .ged file.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

const PAGE: usize = 0x2000;
const BASE: usize = 0x800;
const DATA: usize = 0x1ffa;

const T_INDI: u8 = 0x02;
const T_FAM: u8 = 0x03;
const T_CHILD: u8 = 0x05;
const T_CITE: u8 = 0x06;
const T_SOUR: u8 = 0x07;
const T_EVENT: u8 = 0x0c;
const T_EVTYPES: u8 = 0x0d;
const T_EVCITE: u8 = 0x10;
const T_MEDIA: u8 = 0x11;

fn size_of(t: u8) -> usize {
    match t {
        T_INDI => 0x2a8,
        T_FAM => 0xae,
        T_CHILD => 0x40,
        T_CITE => 0x46,
        T_SOUR => 0x8e,
        T_EVENT => 0xc6,
        T_EVTYPES => 0x38,
        T_EVCITE => 0x2c,
        T_MEDIA => 0x40,
        _ => 0,
    }
}

/// True when the bytes look like an Ancestral Quest database, or a backup (.aqz) holding one.
pub fn is_aq(b: &[u8]) -> bool {
    (b.len() > 0x3000 && &b[..4] == b"AQ\0\0") || (b.starts_with(b"PK") && aq_in_zip(b).is_some())
}

/// The .aq file inside a backup (.aqz) zip.
fn aq_in_zip(b: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(b)).ok()?;
    let name = (0..z.len()).filter_map(|i| z.by_index(i).ok().and_then(|f| f.name().ok().map(|n| n.to_string()))).find(|n| n.to_lowercase().ends_with(".aq"))?;
    let mut f = z.by_name(&name).ok()?;
    if f.size() > 2_000_000_000 {
        return None;
    }
    let mut v = Vec::new();
    f.read_to_end(&mut v).ok()?;
    (v.len() > 0x3000 && &v[..4] == b"AQ\0\0").then_some(v)
}

/// What the reader couldn't fully read, for the import report.
#[derive(Default, Debug)]
pub struct Notes {
    /// Dates stored in a way this reader doesn't know yet (they are left out and counted).
    pub odd_dates: usize,
    /// Events of a kind number the reader has no name for (they came in as plain events).
    pub odd_events: usize,
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).unwrap_or(0)
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).unwrap_or(0)
}

/// Text in the file: UTF-8 when it is valid, otherwise the Windows Western character set.
fn text_of(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b
            .iter()
            .map(|&c| match c {
                0x80 => '€',
                0x91 => '\u{2018}',
                0x92 => '\u{2019}',
                0x93 => '\u{201c}',
                0x94 => '\u{201d}',
                0x96 => '\u{2013}',
                0x97 => '\u{2014}',
                _ => c as char,
            })
            .collect(),
    }
}

/// A GEDCOM tag (or, for kinds without one, an event name) for each built-in event kind number,
/// used when the file has no event-kind table of its own.
const KINDS: [&str; 97] = [
    "Illness", "DIV", "SLGC", "CHR", "ENDL", "Military Service", "NAME", "ADOP", "CHRA", "ANUL", "ANUL", "BAPM", "BAPL", "BARM",
    "BASM", "BIRT", "BLES", "BURI", "CAST", "Cemetery", "CENS", "CENS", "Circumcision", "Citizenship", "CONF", "CONL", "Court", "CREM",
    "DEAT", "Death of Spouse", "Deed", "Degree", "DIV", "DIVF", "DIVF", "EDUC", "Elected", "EMIG", "Employment", "ENGA", "ENGA",
    "Excommunicated", "FCOM", "Funeral", "Godfather", "Godmother", "GRAD", "Hebrew/Yiddish Name", "Hobby", "Honor", "Hospitalization",
    "IMMI", "MARR", "Marriage (Common Law)", "Marriage (de facto)", "Relationship (not married)", "MARC", "MARC", "MARB", "MARB", "MARL",
    "MARL", "MARS", "MARS", "Military Award", "Military Discharge", "Miscarriage", "Miscarriage", "Mission", "Mission", "Move", "Naming",
    "NATU", "NATI", "OCCU", "ORDI", "ORDN", "Organization", "Pension", "PROB", "PROP", "Relationship on Page of Testimony", "RELI",
    "RESI", "Residence before Deportation", "RETI", "SLGS", "Separation", "Separation", "SSN", "TITL", "Travel", "Tribe", "WILL",
    "_BATCH", "REFN", "Marriage for Time Only",
];

/// GEDCOM tags this reader writes as tags of their own (anything else becomes EVEN with a TYPE).
fn is_tag(s: &str) -> bool {
    s.len() >= 3 && s.len() <= 6 && s.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}

pub struct AqFile {
    b: Vec<u8>,
    pages: BTreeMap<u8, Vec<usize>>,
    words: HashMap<u32, String>,
}

impl AqFile {
    pub fn open(bytes: &[u8]) -> Result<Self, String> {
        let b = if bytes.starts_with(b"PK") {
            aq_in_zip(bytes).ok_or_else(|| "This backup (.aqz) has no Ancestral Quest database in it.".to_string())?
        } else {
            bytes.to_vec()
        };
        if b.len() < 0x3000 || &b[..4] != b"AQ\0\0" {
            return Err("This is not an Ancestral Quest database file.".into());
        }
        let mut pages: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
        let n = (b.len() - BASE) / PAGE;
        for k in 2..n {
            let o = BASE + k * PAGE;
            if b.get(o + DATA) == Some(&0x41) {
                pages.entry(b[o + DATA + 1]).or_default().push(o);
            }
        }
        // every word node: "NAMZ", three links, a flag byte, a length, the text and a 0 byte
        let mut words = HashMap::new();
        let mut i = 0;
        while let Some(p) = find(&b, b"NAMZ", i) {
            let len = u16_at(&b, p + 17) as usize;
            if let Some(t) = b.get(p + 19..p + 19 + len) {
                if b.get(p + 19 + len) == Some(&0) {
                    words.insert(p as u32, text_of(t));
                }
            }
            i = p + 4;
        }
        Ok(AqFile { b, pages, words })
    }

    fn word(&self, at: u32) -> Option<&str> {
        self.words.get(&at).map(|s| s.as_str())
    }

    /// Every record slot of a type, numbered from 1, with its offset.
    fn records(&self, t: u8) -> Vec<(usize, usize)> {
        let sz = size_of(t);
        let mut out = Vec::new();
        if sz == 0 {
            return out;
        }
        let per = DATA / sz;
        let mut n = 0;
        for &o in self.pages.get(&t).map(|v| v.as_slice()).unwrap_or(&[]) {
            for i in 0..per {
                n += 1;
                out.push((n, o + i * sz));
            }
        }
        out
    }

    fn slot_empty(&self, o: usize, sz: usize) -> bool {
        self.b[o..o + sz].iter().all(|c| *c == 0)
    }

    /// A place made of up to `n` links to place-part words, smallest first.
    fn place(&self, o: usize, n: usize) -> String {
        (0..n).filter_map(|i| self.word(u32_at(&self.b, o + i * 4))).filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join(", ")
    }

    /// A date kept in 6 bytes. None when it is in a form not understood yet.
    fn date(&self, o: usize) -> Option<String> {
        let y = u16_at(&self.b, o);
        let (flag, year) = (y & 7, y >> 3);
        if y == 0 && u32_at(&self.b, o + 2) == 0 {
            return Some(String::new());
        }
        match flag {
            1 => self.word(u32_at(&self.b, o + 2)).map(|s| s.to_string()),
            0 => {
                let (m, d) = ((self.b[o + 2] >> 3) as usize, (self.b[o + 3] >> 3) as usize);
                // the qualifier is one byte; the byte after it is not understood yet and doesn't change the date shown
                let q = self.b[o + 4];
                if m > 12 || d > 31 || q > 3 {
                    return None;
                }
                const MON: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
                let mut s = String::new();
                if d > 0 {
                    let _ = write!(s, "{d} ");
                }
                if m > 0 {
                    let _ = write!(s, "{} ", MON[m - 1]);
                }
                if year > 0 {
                    let _ = write!(s, "{year}");
                }
                let s = s.trim().to_string();
                Some(match q {
                    1 => format!("AFT {s}"),
                    2 => format!("BEF {s}"),
                    3 => format!("ABT {s}"),
                    _ => s,
                })
            }
            _ => None,
        }
    }
}

fn find(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    if from >= h.len() {
        return None;
    }
    h[from..].windows(n.len()).position(|w| w == n).map(|p| p + from)
}

/// Writes text as a GEDCOM value at a level: the first line on the tag, the rest as CONT lines.
fn put_text(g: &mut String, lvl: usize, tag: &str, text: &str) {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = text.split('\n');
    let _ = writeln!(g, "{lvl} {tag} {}", lines.next().unwrap_or("").trim_end());
    for l in lines {
        let _ = writeln!(g, "{} CONT {}", lvl + 1, l.trim_end());
    }
}

pub fn to_gedcom(bytes: &[u8]) -> Result<(String, Notes), String> {
    let f = AqFile::open(bytes)?;
    let b = &f.b;
    let mut notes = Notes::default();

    // event kinds: from the file's own table when it has one
    let mut kinds: Vec<String> = KINDS.iter().map(|s| s.to_string()).collect();
    let table: Vec<String> = f
        .records(T_EVTYPES)
        .iter()
        .filter(|(_, o)| !f.slot_empty(*o, size_of(T_EVTYPES)))
        .map(|(_, o)| {
            let tag = f.word(u32_at(b, o + 0x1c)).unwrap_or("").to_string();
            if is_tag(&tag) { tag } else { f.word(u32_at(b, *o)).unwrap_or("").to_string() }
        })
        .collect();
    if table.len() >= 50 {
        kinds = table;
    }

    // long texts: (kind letter, owner number) -> text
    let mut texts: BTreeMap<(u8, u32), String> = BTreeMap::new();
    let mut i = 0;
    while let Some(p) = find(b, b"QNTZ", i) {
        let owner = u32_at(b, p + 12);
        let kind = b.get(p + 16).copied().unwrap_or(0);
        let end = b[p + 17..].iter().position(|c| *c == 0).map(|e| p + 17 + e).unwrap_or(b.len());
        if kind.is_ascii_alphabetic() && end > p + 17 {
            texts.entry((kind, owner)).or_default().push_str(&text_of(&b[p + 17..end]));
        }
        i = p + 4;
    }

    // sources, by their own number
    let mut sources: BTreeMap<u32, String> = BTreeMap::new();
    for (_, o) in f.records(T_SOUR) {
        if f.slot_empty(o, size_of(T_SOUR)) {
            continue;
        }
        let id = u32_at(b, o + 4);
        let Some(title) = f.word(u32_at(b, o)) else { continue };
        let mut g = String::new();
        let _ = writeln!(g, "0 @S{id}@ SOUR");
        put_text(&mut g, 1, "TITL", title);
        for (off, tag) in [(0x10, "AUTH"), (0x18, "PUBL"), (0x14, "ABBR")] {
            if let Some(v) = f.word(u32_at(b, o + off)).filter(|v| !v.trim().is_empty() && (tag != "ABBR" || *v != title)) {
                put_text(&mut g, 1, tag, v);
            }
        }
        sources.insert(id, g);
    }
    // citations, by position: (source number, page text)
    let mut cites: HashMap<usize, (u32, String)> = HashMap::new();
    for (n, o) in f.records(T_CITE) {
        if b[o] == b'N' {
            let page = texts.get(&(b'T', n as u32)).cloned().unwrap_or_default();
            cites.insert(n, (u32_at(b, o + 8), page));
        }
    }
    // citation links, by position: (what is cited, owner, citation number)
    let mut evcites: HashMap<usize, (u8, u32, usize)> = HashMap::new();
    for (n, o) in f.records(T_EVCITE) {
        if b[o].is_ascii_uppercase() && b[o + 1] == 3 {
            evcites.insert(n, (b[o], u32_at(b, o + 4), u32_at(b, o + 12) as usize));
        }
    }
    let cite_lines = |g: &mut String, lvl: usize, link: usize| {
        if let Some((_, _, c)) = evcites.get(&link) {
            if let Some((sid, page)) = cites.get(c) {
                if sources.contains_key(sid) {
                    let _ = writeln!(g, "{lvl} SOUR @S{sid}@");
                    if !page.trim().is_empty() {
                        put_text(g, lvl + 1, "PAGE", page);
                    }
                }
            }
        }
    };

    // families, by position
    struct Fam {
        h: u32,
        w: u32,
        head: u32,
        at: usize,
    }
    let mut fams: BTreeMap<usize, Fam> = BTreeMap::new();
    for (n, o) in f.records(T_FAM) {
        if f.slot_empty(o, size_of(T_FAM)) || (u32_at(b, o + 4) == 0 && u32_at(b, o + 8) == 0) {
            continue;
        }
        fams.insert(n, Fam { h: u32_at(b, o + 4), w: u32_at(b, o + 8), head: u32_at(b, o + 0x7e), at: o });
    }
    // child links: family -> (link number, next link, child)
    let mut links: BTreeMap<u32, Vec<(u32, u32, u32)>> = BTreeMap::new();
    for (_, o) in f.records(T_CHILD) {
        if &b[o..o + 4] == b"bbV\0" {
            let (fam, next, me, child) = (u32_at(b, o + 4), u32_at(b, o + 8), u32_at(b, o + 12), u32_at(b, o + 16));
            links.entry(fam).or_default().push((me, next, child));
        }
    }
    let kids_of = |fam: u32, head: u32| -> Vec<u32> {
        let Some(v) = links.get(&fam) else { return vec![] };
        // follow the circle of links from the family's first child; fall back to link order
        let by: HashMap<u32, (u32, u32)> = v.iter().map(|(me, nx, c)| (*me, (*nx, *c))).collect();
        let mut out = Vec::new();
        let mut cur = head;
        while let Some((nx, c)) = by.get(&cur) {
            if out.contains(c) {
                break;
            }
            out.push(*c);
            cur = *nx;
            if out.len() > v.len() {
                break;
            }
        }
        if out.len() != v.len() {
            let mut s = v.clone();
            s.sort();
            out = s.into_iter().map(|x| x.2).collect();
        }
        out
    };

    // other events: owner kind ('I' person, 'M' family) and number -> GEDCOM lines
    let mut extra: BTreeMap<(u8, u32), String> = BTreeMap::new();
    for (_, o) in f.records(T_EVENT) {
        if f.slot_empty(o, size_of(T_EVENT)) {
            continue;
        }
        let (who, owner, k) = (b[o + 0xb0], u32_at(b, o + 8), u32_at(b, o + 12) as usize);
        if !(who == b'I' || who == b'M') || owner == 0 || k == 0 {
            continue;
        }
        let kind = kinds.get(k - 1).cloned().unwrap_or_default();
        if kind.is_empty() {
            notes.odd_events += 1;
        }
        let g = extra.entry((who, owner)).or_default();
        let value = f.word(u32_at(b, o + 0x10)).unwrap_or("").to_string();
        if is_tag(&kind) {
            if value.is_empty() { let _ = writeln!(g, "1 {kind}"); } else { put_text(g, 1, &kind, &value); }
        } else {
            if value.is_empty() { let _ = writeln!(g, "1 EVEN"); } else { put_text(g, 1, "EVEN", &value); }
            let _ = writeln!(g, "2 TYPE {}", if kind.is_empty() { format!("Event {k}") } else { kind });
        }
        match f.date(o + 0x1c) {
            Some(d) if !d.is_empty() => { let _ = writeln!(g, "2 DATE {d}"); }
            None => notes.odd_dates += 1,
            _ => {}
        }
        let pl = f.place(o + 0x22, 6);
        if !pl.is_empty() {
            let _ = writeln!(g, "2 PLAC {pl}");
        }
    }

    // media: owner -> lines
    let mut media: BTreeMap<u32, String> = BTreeMap::new();
    for (_, o) in f.records(T_MEDIA) {
        let owner = u32_at(b, o);
        let Some(file) = f.word(u32_at(b, o + 12)) else { continue };
        if owner == 0 {
            continue;
        }
        let g = media.entry(owner).or_default();
        let _ = writeln!(g, "1 OBJE");
        let _ = writeln!(g, "2 FILE {file}");
        if let Some(ext) = file.rsplit('.').next().filter(|e| e.len() <= 4 && *e != file) {
            let _ = writeln!(g, "3 FORM {}", ext.to_lowercase());
        }
        if let Some(t) = f.word(u32_at(b, o + 16)).filter(|t| !t.trim().is_empty()) {
            put_text(g, 2, "TITL", t);
        }
    }

    // people
    let mut g = String::new();
    let _ = writeln!(g, "0 HEAD\n1 SOUR ANCESTRAL_QUEST_FILE\n2 NAME Ancestral Quest database, read by Gilded Roots\n1 GEDC\n2 VERS 5.5.1\n2 FORM LINEAGE-LINKED\n1 CHAR UTF-8");
    let mut fams_of: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (n, fm) in &fams {
        for p in [fm.h, fm.w] {
            if p != 0 {
                fams_of.entry(p).or_default().push(*n);
            }
        }
    }
    let mut famc: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    let mut kids: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for (n, fm) in &fams {
        let k = kids_of(*n as u32, fm.head);
        for c in &k {
            famc.entry(*c).or_default().push(*n);
        }
        kids.insert(*n, k);
    }
    let mut people = 0;
    for (_, o) in f.records(T_INDI) {
        if f.slot_empty(o, size_of(T_INDI)) {
            continue;
        }
        let rin = u32_at(b, o + 0x26e);
        let sex = b[o];
        let surname = f.word(u32_at(b, o + 4));
        let given: Vec<&str> = (0..4).map_while(|i| f.word(u32_at(b, o + 8 + i * 4))).collect();
        if rin == 0 || (surname.is_none() && given.is_empty() && !matches!(sex, b'M' | b'F')) {
            continue;
        }
        people += 1;
        let _ = writeln!(g, "0 @I{rin}@ INDI");
        let _ = writeln!(g, "1 NAME {} /{}/", given.join(" "), surname.unwrap_or(""));
        let _ = writeln!(g, "1 SEX {}", match sex { b'M' => "M", b'F' => "F", _ => "U" });
        for (off, tag) in [(0x18, "BIRT"), (0x62, "CHR"), (0xac, "DEAT"), (0xf6, "BURI")] {
            let d = f.date(o + off);
            let pl = f.place(o + off + 6, 6);
            let link = u16_at(b, o + off + 0x1e) as usize;
            if d.as_deref().unwrap_or("").is_empty() && pl.is_empty() && link == 0 {
                if d.is_none() {
                    notes.odd_dates += 1;
                }
                continue;
            }
            let _ = writeln!(g, "1 {tag}");
            match d {
                Some(d) if !d.is_empty() => { let _ = writeln!(g, "2 DATE {d}"); }
                None => notes.odd_dates += 1,
                _ => {}
            }
            if !pl.is_empty() {
                let _ = writeln!(g, "2 PLAC {pl}");
            }
            if link > 0 {
                cite_lines(&mut g, 2, link);
            }
        }
        if let Some(x) = extra.get(&(b'I', rin)) {
            g.push_str(x);
        }
        if let Some(t) = texts.get(&(b'I', rin)).filter(|t| !t.trim().is_empty()) {
            put_text(&mut g, 1, "NOTE", t);
        }
        if let Some(m) = media.get(&rin) {
            g.push_str(m);
        }
        // citations on the person as a whole
        let mut whole: Vec<usize> = evcites.iter().filter(|(_, (w, owner, _))| *w == b'I' && *owner == rin).map(|(n, _)| *n).collect();
        whole.sort();
        for n in whole {
            cite_lines(&mut g, 1, n);
        }
        for fm in fams_of.get(&rin).map(|v| v.as_slice()).unwrap_or(&[]) {
            let _ = writeln!(g, "1 FAMS @F{fm}@");
        }
        for fm in famc.get(&rin).map(|v| v.as_slice()).unwrap_or(&[]) {
            let _ = writeln!(g, "1 FAMC @F{fm}@");
        }
    }
    if people == 0 {
        return Err("This Ancestral Quest file has no people in it.".into());
    }
    for (n, fm) in &fams {
        let _ = writeln!(g, "0 @F{n}@ FAM");
        if fm.h != 0 {
            let _ = writeln!(g, "1 HUSB @I{}@", fm.h);
        }
        if fm.w != 0 {
            let _ = writeln!(g, "1 WIFE @I{}@", fm.w);
        }
        for c in kids.get(n).map(|v| v.as_slice()).unwrap_or(&[]) {
            let _ = writeln!(g, "1 CHIL @I{c}@");
        }
        let o = fm.at;
        let d = f.date(o + 0x10);
        let pl = f.place(o + 0x16, 6);
        if !d.as_deref().unwrap_or("").is_empty() || !pl.is_empty() {
            let _ = writeln!(g, "1 MARR");
            match d {
                Some(d) if !d.is_empty() => { let _ = writeln!(g, "2 DATE {d}"); }
                None => notes.odd_dates += 1,
                _ => {}
            }
            if !pl.is_empty() {
                let _ = writeln!(g, "2 PLAC {pl}");
            }
        }
        if let Some(x) = extra.get(&(b'M', *n as u32)) {
            g.push_str(x);
        }
        if let Some(t) = texts.get(&(b'M', *n as u32)).filter(|t| !t.trim().is_empty()) {
            put_text(&mut g, 1, "NOTE", t);
        }
    }
    for s in sources.values() {
        g.push_str(s);
    }
    g.push_str("0 TRLR\n");
    Ok((g, notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    /// A made-up test tree saved by Ancestral Quest 16, compared with the GEDCOM it was imported from:
    /// GR_AQ=/path/AQSAMPLE-7people.aq cargo test aq_matches_its_gedcom -- --ignored --nocapture
    #[test]
    #[ignore]
    fn aq_matches_its_gedcom() {
        let Ok(p) = std::env::var("GR_AQ") else { return };
        let (g, notes) = to_gedcom(&std::fs::read(p).unwrap()).unwrap();
        println!("{g}\n{notes:?}");
    }

    /// A made-up tree imported into Ancestral Quest 16 from testdata/aq-sample-7people.ged and saved by it
    /// (the .aqz is AQ's backup format: a zip with the .aq inside). Reading it must give the same tree.
    #[test]
    fn the_sample_database_reads_like_its_gedcom() {
        let aqz = include_bytes!("../testdata/aq-sample-7people.aqz");
        assert!(is_aq(aqz));
        let (g, notes) = to_gedcom(aqz).unwrap();
        assert_eq!((notes.odd_dates, notes.odd_events), (0, 0));
        let (ours, _) = crate::parse_ged_report(&g);
        let (theirs, _) = crate::parse_ged_report(include_str!("../testdata/aq-sample-7people.ged"));
        assert_eq!(ours.people.len(), 7);
        assert_eq!(ours.fams.len(), 2);
        let key = |db: &Db| {
            let mut v: Vec<String> = db
                .people
                .values()
                .map(|p| format!("{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}", p.name, p.sex, p.bd, p.bp, p.chd, p.chp, p.dd, p.dp, p.bud, p.bup, p.notes.replace("\r", "")))
                .collect();
            v.sort();
            v
        };
        assert_eq!(key(&ours), key(&theirs));
        let albert = ours.people.values().find(|p| p.name.starts_with("Albert")).unwrap();
        assert!(albert.events.iter().any(|e| e.kind == "Occupation" && e.val == "Blacksmith"));
        assert!(albert.events.iter().any(|e| e.kind == "Immigration" && e.date == "4 JUL 1872"));
        assert_eq!(albert.media.len(), 1);
        assert!(albert.sources.iter().any(|c| c.page == "Folio 12, entry 34"));
        // children in the order they were given, and the divorce on the first marriage
        let f1 = ours.fams.values().find(|f| f.kids.len() == 3).unwrap();
        let names: Vec<&str> = f1.kids.iter().map(|k| ours.people[k].given.as_str()).collect();
        assert_eq!(names, ["Edith Rose", "George Henry", "Baby"]);
        assert!(f1.events.iter().any(|e| e.kind == "Divorce" && e.date == "1885"));
    }

    #[test]
    fn dates_are_decoded() {
        let mut b = vec![0u8; 0x4000];
        b[..4].copy_from_slice(b"AQ\0\0");
        // 13 Nov 1843
        b[0x900..0x906].copy_from_slice(&[0x98, 0x39, 0x58, 0x68, 0, 0]);
        // ABT 1900
        b[0x910..0x916].copy_from_slice(&[0x60, 0x3b, 0, 0, 3, 0]);
        // DEC 1920
        b[0x920..0x926].copy_from_slice(&[0x00, 0x3c, 0x60, 0, 0, 0]);
        let f = AqFile::open(&b).unwrap();
        assert_eq!(f.date(0x900).unwrap(), "13 NOV 1843");
        assert_eq!(f.date(0x910).unwrap(), "ABT 1900");
        assert_eq!(f.date(0x920).unwrap(), "DEC 1920");
        assert!(is_aq(&b));
        assert!(!is_aq(b"not a database"));
    }
}
