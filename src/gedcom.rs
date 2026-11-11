//! GEDCOM reading and writing.
//!
//! Design: the file is first read into a plain tree of lines (`Node`). Everything the app has a field
//! for is mapped into the model. Everything else is kept verbatim in `Raw` blocks and written back
//! on export, so nothing is silently thrown away. After an import, `loss_check` compares the original
//! file with what would be exported and lists anything that would not survive.

use super::*;
use std::collections::BTreeSet;

/* ------------------------------------------------------------------ */
/* Text decoding                                                       */
/* ------------------------------------------------------------------ */

const CP1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™',
    'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

fn cp1252(b: &[u8]) -> String {
    b.iter()
        .map(|&c| if (0x80..0xA0).contains(&c) { CP1252[(c - 0x80) as usize] } else { c as char })
        .collect()
}

/// ANSEL (the character set old genealogy programs such as PAF write) to Unicode.
fn ansel(b: &[u8]) -> String {
    fn base(c: u8) -> Option<char> {
        Some(match c {
            0xA1 => 'Ł',
            0xA2 => 'Ø',
            0xA3 => 'Đ',
            0xA4 => 'Þ',
            0xA5 => 'Æ',
            0xA6 => 'Œ',
            0xA7 => 'ʹ',
            0xA8 => '·',
            0xA9 => '♭',
            0xAA => '®',
            0xAB => '±',
            0xAC => 'Ơ',
            0xAD => 'Ư',
            0xAE => 'ʼ',
            0xB0 => 'ʻ',
            0xB1 => 'ł',
            0xB2 => 'ø',
            0xB3 => 'đ',
            0xB4 => 'þ',
            0xB5 => 'æ',
            0xB6 => 'œ',
            0xB7 => 'ʺ',
            0xB8 => 'ı',
            0xB9 => '£',
            0xBA => 'ð',
            0xBC => 'ơ',
            0xBD => 'ư',
            0xC0 => '°',
            0xC1 => 'ℓ',
            0xC2 => '℗',
            0xC3 => '©',
            0xC4 => '♯',
            0xC5 => '¿',
            0xC6 => '¡',
            0xCF => 'ß',
            _ => return None,
        })
    }
    fn mark(c: u8) -> Option<char> {
        Some(match c {
            0xE0 => '\u{309}',
            0xE1 => '\u{300}',
            0xE2 => '\u{301}',
            0xE3 => '\u{302}',
            0xE4 => '\u{303}',
            0xE5 => '\u{304}',
            0xE6 => '\u{306}',
            0xE7 => '\u{307}',
            0xE8 => '\u{308}',
            0xE9 => '\u{30C}',
            0xEA => '\u{30A}',
            0xEB => '\u{361}',
            0xEC => '\u{360}',
            0xED => '\u{315}',
            0xEE => '\u{30B}',
            0xEF => '\u{310}',
            0xF0 => '\u{327}',
            0xF1 => '\u{328}',
            0xF2 => '\u{323}',
            0xF3 => '\u{324}',
            0xF4 => '\u{325}',
            0xF5 => '\u{333}',
            0xF6 => '\u{332}',
            0xF7 => '\u{326}',
            0xF8 => '\u{31C}',
            0xF9 => '\u{32E}',
            0xFA => '\u{35C}',
            0xFB => '\u{35D}',
            0xFE => '\u{313}',
            _ => return None,
        })
    }
    use unicode_normalization::UnicodeNormalization;
    let mut out = String::new();
    let mut pending: Vec<char> = Vec::new();
    for &c in b {
        if let Some(m) = mark(c) {
            pending.push(m);
            continue;
        }
        let ch = if c < 0x80 { c as char } else { base(c).unwrap_or(c as char) };
        out.push(ch);
        while let Some(m) = pending.pop() {
            out.push(m);
        }
        if ch == '\n' || ch == '\r' {
            // a mark can never belong to the next line
        }
    }
    for m in pending.into_iter().rev() {
        out.push(m);
    }
    out.nfc().collect()
}

/// Turn the bytes of a GEDCOM file into text. Understands UTF-8, UTF-16, ANSEL and Windows ANSI.
/// Returns the text and a short description of the character set that was used.
pub fn decode_ged(b: &[u8]) -> (String, String) {
    if b.len() >= 2 && (b.starts_with(&[0xFF, 0xFE]) || b.starts_with(&[0xFE, 0xFF])) {
        let le = b[0] == 0xFF;
        let u: Vec<u16> = b[2..]
            .chunks_exact(2)
            .map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
            .collect();
        return (String::from_utf16_lossy(&u), "UTF-16".into());
    }
    if b.len() >= 4 && b[0] == b'0' && b[1] == 0 && b[2] == b' ' {
        let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return (String::from_utf16_lossy(&u), "UTF-16".into());
    }
    if b.len() >= 4 && b[0] == 0 && b[1] == b'0' && b[2] == 0 {
        let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        return (String::from_utf16_lossy(&u), "UTF-16".into());
    }
    if let Ok(s) = std::str::from_utf8(b) {
        let ascii = s.is_ascii();
        return (s.trim_start_matches('\u{feff}').to_string(), if ascii { "ASCII".into() } else { "UTF-8".into() });
    }
    // Not UTF-8: look at what the file says about itself.
    let head: String = b.iter().take(6000).map(|&c| c as char).collect();
    let declared = head
        .lines()
        .chain(head.split('\r'))
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("1 CHAR ").map(|v| v.trim().to_uppercase())
        })
        .unwrap_or_default();
    if declared.contains("ANSEL") {
        return (ansel(b), "ANSEL".into());
    }
    let label = if declared.is_empty() { "Windows ANSI".to_string() } else { format!("{} (read as Windows ANSI)", declared) };
    (cp1252(b), label)
}

/* ------------------------------------------------------------------ */
/* Dates                                                               */
/* ------------------------------------------------------------------ */

const MON: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
const MON_FULL: [&str; 12] = [
    "JANUARY", "FEBRUARY", "MARCH", "APRIL", "MAY", "JUNE", "JULY", "AUGUST", "SEPTEMBER", "OCTOBER", "NOVEMBER", "DECEMBER",
];

fn month_of(tok: &str) -> Option<usize> {
    let t = tok.trim_end_matches('.').to_uppercase();
    if t.len() < 3 || !t.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    MON_FULL.iter().position(|m| m.starts_with(t.as_str()) && (t.len() == 3 || t.len() >= 3 && m.starts_with(t.as_str())))
}

fn is_year_tok(t: &str) -> bool {
    let (a, b) = match t.split_once('/') {
        Some((a, b)) => (a, Some(b)),
        None => (t, None),
    };
    !a.is_empty() && a.len() <= 4 && a.chars().all(|c| c.is_ascii_digit()) && b.map_or(true, |b| b.len() == 2 && b.chars().all(|c| c.is_ascii_digit()))
}

/// One calendar date in GEDCOM form ("12 MAR 1850"), or None if it is not a plain date.
fn norm_single(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.contains("@#") {
        return Some(s.to_string());
    }
    // 1850-03-12
    let iso: Vec<&str> = s.split('-').collect();
    if iso.len() == 3 && iso[0].len() == 4 && iso[1].len() == 2 && iso[2].len() == 2 && iso.iter().all(|x| x.chars().all(|c| c.is_ascii_digit())) {
        let m: usize = iso[1].parse().ok()?;
        let d: u32 = iso[2].parse().ok()?;
        if (1..=12).contains(&m) && (1..=31).contains(&d) {
            return Some(format!("{} {} {}", d, MON[m - 1], iso[0]));
        }
    }
    let cleaned = s.replace(',', " ");
    let mut toks: Vec<&str> = cleaned.split_whitespace().collect();
    let mut bc = false;
    if let Some(l) = toks.last() {
        let lu = l.to_uppercase();
        if lu == "BC" || lu == "B.C." || lu == "B.C" {
            bc = true;
            toks.pop();
        }
    }
    let mi = toks.iter().position(|t| month_of(t).is_some());
    let (day, mon, year): (Option<&str>, Option<usize>, Option<&str>) = match mi {
        None => {
            if toks.len() == 1 && is_year_tok(toks[0]) {
                (None, None, Some(toks[0]))
            } else {
                return None;
            }
        }
        Some(i) => {
            let m = month_of(toks[i]);
            let before = &toks[..i];
            let after = &toks[i + 1..];
            match (before, after) {
                ([], []) => (None, m, None),
                ([], [y]) if is_year_tok(y) => (None, m, Some(*y)),
                ([d], []) if d.len() <= 2 && d.chars().all(|c| c.is_ascii_digit()) => (Some(*d), m, None),
                ([d], [y]) if d.len() <= 2 && d.chars().all(|c| c.is_ascii_digit()) && is_year_tok(y) => (Some(*d), m, Some(*y)),
                ([], [d, y]) if d.len() <= 2 && d.chars().all(|c| c.is_ascii_digit()) && is_year_tok(y) => (Some(*d), m, Some(*y)),
                _ => return None,
            }
        }
    };
    if let Some(d) = day {
        let n: u32 = d.parse().ok()?;
        if n == 0 || n > 31 {
            return None;
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(d) = day {
        parts.push(d.to_string());
    }
    if let Some(m) = mon {
        parts.push(MON[m].to_string());
    }
    if let Some(y) = year {
        parts.push(y.to_string());
    }
    if parts.is_empty() {
        return None;
    }
    let mut out = parts.join(" ");
    if bc {
        out.push_str(" BC");
    }
    Some(out)
}

/// Split off a leading keyword such as "ABT" or "about". Returns the canonical GEDCOM keyword and the rest.
fn date_prefix(u: &str) -> Option<(&'static str, &str)> {
    let mut it = u.splitn(2, char::is_whitespace);
    let first = it.next()?;
    let rest = it.next().unwrap_or("").trim();
    let k = first.trim_end_matches('.').to_lowercase();
    let g = match k.as_str() {
        "abt" | "about" | "circa" | "ca" | "c" | "approx" | "approximately" => "ABT",
        "bef" | "before" | "pre" => "BEF",
        "aft" | "after" | "post" => "AFT",
        "cal" | "calculated" => "CAL",
        "est" | "estimated" => "EST",
        "int" | "interpreted" => "INT",
        "bet" | "between" => "BET",
        "from" => "FROM",
        "to" => "TO",
        _ => return None,
    };
    Some((g, rest))
}

fn split_word<'a>(s: &'a str, w: &str) -> Option<(&'a str, &'a str)> {
    let l = s.to_lowercase();
    let pat = format!(" {} ", w);
    let i = l.find(&pat)?;
    Some((&s[..i], &s[i + pat.len()..]))
}

/// A date as the user sees it, from a GEDCOM date.
pub fn nice_date(v: &str) -> String {
    let u = v.trim();
    if u.starts_with('(') && u.ends_with(')') && u.len() >= 2 {
        return u[1..u.len() - 1].trim().to_string();
    }
    match date_prefix(u) {
        Some(("ABT", r)) => format!("about {}", r),
        Some(("BEF", r)) => format!("before {}", r),
        Some(("AFT", r)) => format!("after {}", r),
        Some(("CAL", r)) => format!("calculated {}", r),
        Some(("EST", r)) => format!("estimated {}", r),
        Some(("INT", r)) => format!("interpreted {}", r),
        Some(("BET", r)) => match split_word(r, "and") {
            Some((a, b)) => format!("between {} and {}", a.trim(), b.trim()),
            None => u.to_string(),
        },
        Some(("FROM", r)) => match split_word(r, "to") {
            Some((a, b)) => format!("from {} to {}", a.trim(), b.trim()),
            None => format!("from {}", r),
        },
        Some(("TO", r)) => format!("to {}", r),
        _ => u.to_string(),
    }
}

/// A date for a GEDCOM file, from whatever the user typed. Text that is not a date becomes a
/// date phrase in brackets, which is valid GEDCOM and is kept exactly.
pub fn ged_date(v: &str) -> String {
    let u = v.trim();
    if u.is_empty() {
        return String::new();
    }
    if u.starts_with('(') {
        return u.to_string();
    }
    let phrase = || format!("({})", u);
    match date_prefix(u) {
        Some(("INT", r)) => {
            // INT <date> (phrase)
            let (d, ph) = match r.find('(') {
                Some(i) => (r[..i].trim(), Some(r[i..].trim())),
                None => (r, None),
            };
            match norm_single(d) {
                Some(n) => match ph {
                    Some(p) => format!("INT {} {}", n, p),
                    None => format!("INT {} ({})", n, u),
                },
                None => phrase(),
            }
        }
        Some((k @ ("ABT" | "BEF" | "AFT" | "CAL" | "EST"), r)) => match norm_single(r) {
            Some(n) => format!("{} {}", k, n),
            None => phrase(),
        },
        Some(("BET", r)) => match split_word(r, "and") {
            Some((a, b)) => match (norm_single(a), norm_single(b)) {
                (Some(a), Some(b)) => format!("BET {} AND {}", a, b),
                _ => phrase(),
            },
            None => phrase(),
        },
        Some(("FROM", r)) => match split_word(r, "to") {
            Some((a, b)) => match (norm_single(a), norm_single(b)) {
                (Some(a), Some(b)) => format!("FROM {} TO {}", a, b),
                _ => phrase(),
            },
            None => match norm_single(r) {
                Some(a) => format!("FROM {}", a),
                None => phrase(),
            },
        },
        Some(("TO", r)) => match norm_single(r) {
            Some(a) => format!("TO {}", a),
            None => phrase(),
        },
        _ => match norm_single(u) {
            Some(n) => n,
            None => phrase(),
        },
    }
}

/* ------------------------------------------------------------------ */
/* The line tree                                                       */
/* ------------------------------------------------------------------ */

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub xref: Option<String>,
    pub tag: String,
    pub val: String,
    pub kids: Vec<Node>,
}

struct Flat {
    lvl: u32,
    xref: Option<String>,
    tag: String,
    val: String,
}

/// Read the lines of a GEDCOM file into a tree. CONC and CONT lines are joined into their parent's value.
/// Handles files that end lines with LF, CR LF or just CR, and lines that lost their level number.
pub fn build_tree(text: &str, warnings: &mut Vec<String>) -> Vec<Node> {
    let mut flat: Vec<Flat> = Vec::new();
    let mut bad = 0usize;
    for raw in text.split(|c| c == '\n' || c == '\r') {
        let line = raw.trim_start_matches('\u{feff}').trim_start_matches(|c| c == ' ' || c == '\t');
        if line.trim().is_empty() {
            continue;
        }
        let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
        let after = &line[digits.len()..];
        let lvl: Option<u32> = if !digits.is_empty() && (after.is_empty() || after.starts_with(' ') || after.starts_with('\t')) { digits.parse().ok() } else { None };
        let Some(lvl) = lvl else {
            // A line with no level number: treat as more text of the line before.
            bad += 1;
            if let Some(last) = flat.last_mut() {
                last.val.push('\n');
                last.val.push_str(line.trim_end());
            }
            continue;
        };
        let rest = after.trim_start_matches(|c| c == ' ' || c == '\t');
        let mut xref = None;
        let mut rest = rest;
        if rest.starts_with('@') && !rest.starts_with("@@") {
            if let Some((x, r)) = rest.split_once(char::is_whitespace) {
                if x.len() > 2 && x.ends_with('@') {
                    xref = Some(x.trim_matches('@').to_string());
                    rest = r.trim_start();
                }
            }
        }
        let (tag, val) = match rest.split_once(' ') {
            Some((t, v)) => (t, v),
            None => (rest, ""),
        };
        let tag = tag.to_ascii_uppercase();
        let val = if tag == "CONC" || tag == "CONT" { val.to_string() } else { val.trim().to_string() };
        flat.push(Flat { lvl, xref, tag, val });
    }
    if bad > 0 {
        warnings.push(format!("{} line(s) had no level number and were joined to the line before them.", bad));
    }
    let mut roots: Vec<Node> = Vec::new();
    let mut stack: Vec<(u32, Node)> = Vec::new();
    fn close(stack: &mut Vec<(u32, Node)>, roots: &mut Vec<Node>) {
        if let Some((_, n)) = stack.pop() {
            match stack.last_mut() {
                Some((_, parent)) => {
                    if n.tag == "CONC" {
                        parent.val.push_str(&n.val);
                    } else if n.tag == "CONT" {
                        parent.val.push('\n');
                        parent.val.push_str(&n.val);
                    } else {
                        parent.kids.push(n);
                    }
                }
                None => roots.push(n),
            }
        }
    }
    for f in flat {
        while let Some((l, _)) = stack.last() {
            if *l >= f.lvl {
                close(&mut stack, &mut roots);
            } else {
                break;
            }
        }
        stack.push((f.lvl, Node { xref: f.xref, tag: f.tag, val: f.val, kids: Vec::new() }));
    }
    while !stack.is_empty() {
        close(&mut stack, &mut roots);
    }
    roots
}

/* ------------------------------------------------------------------ */
/* Writing lines                                                       */
/* ------------------------------------------------------------------ */

fn split_chunk(s: &str, max: usize) -> (&str, &str) {
    if s.len() <= max {
        return (s, "");
    }
    let mut cut = max;
    while !s.is_char_boundary(cut) {
        cut -= 1;
    }
    // Prefer to break where neither side is a space, so no reader can lose a significant space.
    let b = s.as_bytes();
    let mut p = cut;
    while p > cut.saturating_sub(40) && p > 0 {
        if s.is_char_boundary(p) && b[p - 1] != b' ' && b[p] != b' ' {
            return (&s[..p], &s[p..]);
        }
        p -= 1;
    }
    (&s[..cut], &s[cut..])
}

/// Write one structure line, splitting long text with CONC and line breaks with CONT.
fn put(o: &mut String, lvl: u32, tag: &str, val: &str) {
    let mut first = true;
    for line in val.split('\n') {
        let line = line.trim_end_matches('\r');
        let mut rest = line;
        let mut head = true;
        loop {
            let (chunk, tail) = split_chunk(rest, 240);
            let (l, t) = match (first, head) {
                (true, true) => (lvl, tag),
                (false, true) => (lvl + 1, "CONT"),
                (_, false) => (lvl + 1, "CONC"),
            };
            if chunk.is_empty() {
                let _ = writeln!(o, "{} {}", l, t);
            } else {
                let _ = writeln!(o, "{} {} {}", l, t, chunk);
            }
            head = false;
            if tail.is_empty() {
                break;
            }
            rest = tail;
        }
        first = false;
    }
}

/// Like `put`, for free text typed by a person: a leading "@" is doubled as GEDCOM requires.
fn puttxt(o: &mut String, lvl: u32, tag: &str, val: &str) {
    if val.starts_with('@') {
        put(o, lvl, tag, &format!("@{}", val));
    } else {
        put(o, lvl, tag, val);
    }
}

fn putp(o: &mut String, lvl: u32, tag: &str, canon: &str) {
    let _ = writeln!(o, "{} {} @{}@", lvl, tag, canon);
}

fn txt(v: &str) -> String {
    match v.strip_prefix("@@") {
        Some(r) => format!("@{}", r),
        None => v.to_string(),
    }
}

fn is_ptr(v: &str) -> bool {
    v.len() > 2 && v.starts_with('@') && v.ends_with('@') && !v.starts_with("@@") && !v.contains(char::is_whitespace) && !v.starts_with("@#")
}

/// Add `base` to the level of every line of a stored block.
fn shift_lines(o: &mut String, text: &str, base: u32) {
    for line in text.lines() {
        let (d, rest) = line.split_once(' ').unwrap_or((line, ""));
        let n: u32 = d.parse().unwrap_or(0);
        if rest.is_empty() {
            let _ = writeln!(o, "{}", n + base);
        } else {
            let _ = writeln!(o, "{} {}", n + base, rest);
        }
    }
}

fn emit_raw(o: &mut String, raws: &[Raw], at: &str, lvl: u32) {
    for r in raws.iter().filter(|r| r.at == at) {
        shift_lines(o, &r.text, lvl);
    }
}

/* ------------------------------------------------------------------ */
/* Import report                                                       */
/* ------------------------------------------------------------------ */

#[derive(Clone, Default)]
pub struct ImportReport {
    pub file: String,
    pub program: String,
    pub charset: String,
    pub gedcom_version: String,
    pub people: usize,
    pub families: usize,
    pub sources: usize,
    pub citations: usize,
    pub notes: usize,
    pub events: usize,
    pub alt_names: usize,
    pub media: usize,
    pub media_found: usize,
    pub media_missing: usize,
    /// Pieces of the file the app has no field for. They are kept and written back on export.
    pub kept: BTreeMap<String, usize>,
    pub warnings: Vec<String>,
    /// Lines of the original file that would not appear in an export. Empty means nothing is lost.
    pub lost: Vec<String>,
}

impl ImportReport {
    pub fn kept_total(&self) -> usize {
        self.kept.values().sum()
    }
}

/* ------------------------------------------------------------------ */
/* Reading                                                             */
/* ------------------------------------------------------------------ */

struct Cx {
    rm: HashMap<String, String>,
    objs: HashMap<String, Node>,
    titles: HashMap<u32, String>,
    pedi: Vec<(u32, u32, Option<String>, Option<String>, Option<String>)>,
    used_objs: HashSet<String>,
    rep: ImportReport,
}

impl Cx {
    fn canon(&mut self, v: &str) -> String {
        if is_ptr(v) {
            let x = v.trim_matches('@');
            match self.rm.get(x) {
                Some(c) => format!("@{}@", c),
                None => {
                    self.rep.warnings.push(format!("The file points to {} but has no such record.", v));
                    v.to_string()
                }
            }
        } else {
            v.to_string()
        }
    }
    fn id_of(&self, v: &str, letter: char) -> Option<u32> {
        if !is_ptr(v) {
            return None;
        }
        let c = self.rm.get(v.trim_matches('@'))?;
        let mut ch = c.chars();
        if ch.next()? != letter {
            return None;
        }
        c[1..].parse().ok()
    }
    fn keep(&mut self, path: &str) {
        // Count by the shape of the path without record ids.
        *self.rep.kept.entry(path.to_string()).or_insert(0) += 1;
    }
}

fn node_text(cx: &mut Cx, n: &Node, lvl: u32, o: &mut String) {
    let val = cx.canon(&n.val);
    match &n.xref {
        Some(x) => {
            let c = cx.rm.get(x).cloned().unwrap_or_else(|| x.clone());
            put(o, lvl, &format!("@{}@ {}", c, n.tag), &val);
        }
        None => put(o, lvl, &n.tag, &val),
    }
    for k in &n.kids {
        node_text(cx, k, lvl + 1, o);
    }
}

fn mk_raw(cx: &mut Cx, at: &str, n: &Node, path: &str) -> Raw {
    let mut s = String::new();
    node_text(cx, n, 0, &mut s);
    cx.keep(&format!("{}.{}", path, n.tag));
    Raw { at: at.to_string(), text: s }
}

/// Keep the children of a date or place (time, map coordinates, forms...) as a block under it.
fn sub_raw(cx: &mut Cx, extra: &mut Vec<Raw>, at: &str, c: &Node, path: &str) {
    if c.kids.is_empty() {
        return;
    }
    let mut s = String::new();
    for k in &c.kids {
        node_text(cx, k, 0, &mut s);
        cx.keep(&format!("{}.{}.{}", path, c.tag, k.tag));
    }
    extra.push(Raw { at: at.to_string(), text: s });
}

fn push_note(text: &mut String, add: &str) {
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(add);
}

/// A NOTE line: inline text goes into `text`, a pointer to a shared note into `refs`.
/// Notes with sources or other structure under them are kept whole.
fn note_into(cx: &mut Cx, c: &Node, text: &mut String, refs: &mut Vec<u32>, extra: &mut Vec<Raw>, path: &str) {
    if c.kids.is_empty() {
        if is_ptr(&c.val) {
            if let Some(id) = cx.id_of(&c.val, 'N') {
                refs.push(id);
                return;
            }
        } else {
            push_note(text, &txt(&c.val));
            return;
        }
    }
    extra.push(mk_raw(cx, "", c, path));
}

fn cite_of(cx: &mut Cx, c: &Node, path: &str) -> Source {
    cx.rep.citations += 1;
    let mut s = Source::default();
    if is_ptr(&c.val) {
        match cx.id_of(&c.val, 'S') {
            Some(id) => {
                s.sid = Some(id);
                s.text = cx.titles.get(&id).cloned().unwrap_or_default();
            }
            None => {
                s.text = format!("(missing source {})", c.val);
                cx.rep.warnings.push(format!("A citation points to {} but the file has no such source.", c.val));
            }
        }
    } else {
        s.text = txt(&c.val);
    }
    for k in &c.kids {
        let sub = format!("{}.{}", path, k.tag);
        match k.tag.as_str() {
            "PAGE" if s.page.is_empty() && k.kids.is_empty() => s.page = txt(&k.val),
            "QUAY" if s.quality.is_empty() && k.kids.is_empty() => s.quality = k.val.clone(),
            "WWW" if s.url.is_empty() && k.kids.is_empty() => s.url = txt(&k.val),
            "_ACCESSED" if s.accessed.is_empty() && k.kids.is_empty() => s.accessed = txt(&k.val),
            "_ABOUT" if k.kids.is_empty() => s.about = txt(&k.val),
            "DATA" if s.data.is_empty() && s.ddate.is_empty() && k.val.is_empty() => {
                let mut rest = String::new();
                for d in &k.kids {
                    match d.tag.as_str() {
                        "DATE" if s.ddate.is_empty() && d.kids.is_empty() => s.ddate = d.val.clone(),
                        "TEXT" if d.kids.is_empty() => push_note(&mut s.data, &txt(&d.val)),
                        _ => {
                            node_text(cx, d, 0, &mut rest);
                            cx.keep(&format!("{}.DATA.{}", path, d.tag));
                        }
                    }
                }
                if !rest.is_empty() {
                    s.extra.push(Raw { at: "DATA".into(), text: rest });
                }
            }
            "NOTE" => {
                let (mut t, mut r, mut x) = (std::mem::take(&mut s.note), std::mem::take(&mut s.note_refs), std::mem::take(&mut s.extra));
                note_into(cx, k, &mut t, &mut r, &mut x, &sub);
                s.note = t;
                s.note_refs = r;
                s.extra = x;
            }
            _ => {
                let r = mk_raw(cx, "", k, path);
                s.extra.push(r);
            }
        }
    }
    s
}

/// Read the common parts of an event or fact: date, place, type, cause, age, agency, temple, sources, notes.
fn ev_of(cx: &mut Cx, k: &Node, path: &str) -> Event {
    let mut e = Event { val: txt(&k.val), ..Default::default() };
    for c in &k.kids {
        let sub = format!("{}.{}", path, c.tag);
        let plain = c.kids.is_empty();
        match c.tag.as_str() {
            "DATE" if e.date.is_empty() => {
                e.date = nice_date(&c.val);
                sub_raw(cx, &mut e.extra, "DATE", c, path);
            }
            "PLAC" if e.place.is_empty() => {
                e.place = txt(&c.val);
                sub_raw(cx, &mut e.extra, "PLAC", c, path);
            }
            "TYPE" if e.typ.is_empty() && plain => e.typ = txt(&c.val),
            "CAUS" if e.cause.is_empty() && plain => e.cause = txt(&c.val),
            "_CONF" if e.conf.is_empty() && plain && conf_label(&c.val).is_some() => e.conf = c.val.trim().to_lowercase(),
            "AGE" if e.age.is_empty() && plain => e.age = c.val.clone(),
            "AGNC" if e.agency.is_empty() && plain => e.agency = txt(&c.val),
            "TEMP" if e.temp.is_empty() && plain => e.temp = c.val.clone(),
            "STAT" if e.stat.is_empty() && plain => e.stat = c.val.clone(),
            "SOUR" => {
                let s = cite_of(cx, c, &sub);
                e.cites.push(s);
            }
            "NOTE" => {
                let (mut t, mut r, mut x) = (std::mem::take(&mut e.note), std::mem::take(&mut e.note_refs), std::mem::take(&mut e.extra));
                note_into(cx, c, &mut t, &mut r, &mut x, &sub);
                e.note = t;
                e.note_refs = r;
                e.extra = x;
            }
            _ => {
                let r = mk_raw(cx, "", c, path);
                e.extra.push(r);
            }
        }
    }
    e
}

fn label_of(tag: &str, table: &[(&'static str, &'static str)]) -> Option<&'static str> {
    table.iter().find(|(t, _)| *t == tag).map(|(_, l)| *l)
}

fn tag_of(kind: &str, table: &[(&'static str, &'static str)]) -> Option<&'static str> {
    table.iter().find(|(_, l)| *l == kind).map(|(t, _)| *t)
}

/// Build an event from an EVEN line or a known/custom event tag. Returns None for anything that is not event-shaped.
fn event_from(cx: &mut Cx, k: &Node, path: &str, table: &[(&'static str, &'static str)]) -> Option<Event> {
    let tag = k.tag.as_str();
    let known = label_of(tag, table);
    let has_dp = k.kids.iter().any(|c| c.tag == "DATE" || c.tag == "PLAC");
    if !(tag == "EVEN" || known.is_some() || has_dp) {
        return None;
    }
    let mut e = ev_of(cx, k, path);
    cx.rep.events += 1;
    if tag == "EVEN" {
        let t = std::mem::take(&mut e.typ);
        e.kind = if t.is_empty() { "Other".to_string() } else { t };
        e.tag = "EVEN".to_string();
    } else if let Some(l) = known {
        e.kind = l.to_string();
    } else {
        e.kind = tag.to_string();
        e.tag = tag.to_string();
    }
    Some(e)
}

fn parse_name_value(v: &str) -> (String, String, String) {
    if v.contains('/') {
        let mut parts = v.splitn(3, '/');
        let g = parts.next().unwrap_or("").trim().to_string();
        let sn = parts.next().unwrap_or("").trim().to_string();
        let sfx = parts.next().unwrap_or("").trim().to_string();
        (g, sn, sfx)
    } else {
        let (g, s) = split_name(v);
        (g, s, String::new())
    }
}

fn contains_ci(hay: &str, needle: &str) -> bool {
    hay.to_lowercase().contains(&needle.to_lowercase())
}

fn read_name(cx: &mut Cx, p: &mut Person, k: &Node, path: &str) {
    let nv = txt(&k.val);
    let (mut g, sn, sfx) = parse_name_value(&nv);
    let mut a = AltName::default();
    let mut ev = Event::default();
    let mut typ = String::new();
    for c in &k.kids {
        let sub = format!("{}.{}", path, c.tag);
        let plain = c.kids.is_empty();
        match c.tag.as_str() {
            "NPFX" if a.prefix.is_empty() && plain => a.prefix = txt(&c.val),
            "NSFX" if a.suffix.is_empty() && plain => a.suffix = txt(&c.val),
            "NICK" if a.nick.is_empty() && plain => a.nick = txt(&c.val),
            "TYPE" if typ.is_empty() && plain => typ = txt(&c.val),
            // Pieces of the name that are already in the name line carry no extra information.
            "GIVN" | "SURN" | "SPFX" | "UNKNOWN" if plain && contains_ci(&nv, &c.val) => {}
            "SOUR" => {
                let s = cite_of(cx, c, &sub);
                ev.cites.push(s);
            }
            "NOTE" => {
                let (mut t, mut r, mut x) = (std::mem::take(&mut ev.note), std::mem::take(&mut ev.note_refs), std::mem::take(&mut ev.extra));
                note_into(cx, c, &mut t, &mut r, &mut x, &sub);
                ev.note = t;
                ev.note_refs = r;
                ev.extra = x;
            }
            _ => {
                let r = mk_raw(cx, "", c, path);
                ev.extra.push(r);
            }
        }
    }
    if !a.prefix.is_empty() {
        if let Some(r) = g.strip_prefix(a.prefix.as_str()) {
            g = r.trim().to_string();
        }
    }
    if a.suffix.is_empty() && !sfx.is_empty() {
        a.suffix = sfx;
    }
    if p.name.is_empty() && p.given.is_empty() && p.surname.is_empty() && !p.fx.contains_key("NAME") {
        p.given = g;
        p.surname = sn;
        p.name = compose_name(&p.given, &p.surname);
        p.prefix = a.prefix;
        p.suffix = a.suffix;
        p.nick = a.nick;
        ev.typ = typ;
        p.fx.insert("NAME".to_string(), ev);
        return;
    }
    let kind = typ.to_lowercase();
    let simple = ev.cites.is_empty() && ev.note.is_empty() && ev.note_refs.is_empty() && ev.extra.is_empty() && a.prefix.is_empty() && a.suffix.is_empty() && a.nick.is_empty();
    if (kind == "birth" || kind == "maiden") && p.maiden.is_empty() && simple && !sn.is_empty() && (g.is_empty() || g == p.given) {
        p.maiden = sn;
        p.fvals.insert("MAIDEN_TYPE".to_string(), typ);
        return;
    }
    cx.rep.alt_names += 1;
    p.alt_names.push(AltName { kind: typ, given: g, surname: sn, prefix: a.prefix, suffix: a.suffix, nick: a.nick, note: ev.note, note_refs: ev.note_refs, cites: ev.cites, extra: ev.extra });
}

fn media_of(cx: &mut Cx, n: &Node, path: &str) -> Vec<Media> {
    // A pointer to an object record, or the object written out in place.
    let owned;
    let n: &Node = if is_ptr(&n.val) {
        match cx.objs.get(n.val.trim_matches('@')) {
            Some(o) => {
                cx.used_objs.insert(n.val.trim_matches('@').to_string());
                owned = o.clone();
                &owned
            }
            None => {
                let mut m = Media::default();
                m.extra.push(mk_raw(cx, "", n, path));
                return vec![m];
            }
        }
    } else {
        n
    };
    let mut out: Vec<Media> = Vec::new();
    let mut loose: Vec<Raw> = Vec::new();
    let (mut o_titl, mut o_form) = (String::new(), String::new());
    let mut o_form_extra: Vec<Raw> = Vec::new();
    for c in &n.kids {
        match c.tag.as_str() {
            "FILE" => {
                let mut m = Media { path: txt(&c.val), ..Default::default() };
                m.name = m.path.rsplit(|ch| ch == '/' || ch == '\\').next().unwrap_or("").to_string();
                for d in &c.kids {
                    match d.tag.as_str() {
                        "FORM" if m.form.is_empty() => {
                            m.form = txt(&d.val);
                            sub_raw(cx, &mut m.extra, "FORM", d, path);
                        }
                        "TITL" if m.caption.is_empty() && d.kids.is_empty() => m.caption = txt(&d.val),
                        _ => {
                            let mut s = String::new();
                            node_text(cx, d, 0, &mut s);
                            cx.keep(&format!("{}.FILE.{}", path, d.tag));
                            m.extra.push(Raw { at: "FILE".into(), text: s });
                        }
                    }
                }
                out.push(m);
            }
            "TITL" if o_titl.is_empty() && c.kids.is_empty() => o_titl = txt(&c.val),
            "FORM" if o_form.is_empty() => {
                // GEDCOM 5.5 puts FORM beside FILE; Ancestry also hangs TYPE, _SIZE... under it.
                o_form = txt(&c.val);
                sub_raw(cx, &mut o_form_extra, "FORM", c, path);
            }
            _ => loose.push(mk_raw(cx, "", c, path)),
        }
    }
    if out.is_empty() {
        out.push(Media::default());
    }
    for m in out.iter_mut() {
        if m.caption.is_empty() {
            m.caption = o_titl.clone();
        }
        if m.form.is_empty() {
            m.form = o_form.clone();
            m.extra.extend(o_form_extra.iter().cloned());
        }
    }
    out[0].extra.extend(loose);
    cx.rep.media += out.len();
    out
}

/// An `ASSO @I5@` line that only says who and in what role (and maybe a plain note).
fn assoc_of(cx: &Cx, k: &Node) -> Option<Assoc> {
    let other = cx.id_of(&k.val, 'I')?;
    let mut a = Assoc { other, ..Default::default() };
    let (mut saw_rela, mut saw_type) = (false, false);
    for c in &k.kids {
        if !c.kids.is_empty() {
            return None;
        }
        match c.tag.as_str() {
            "RELA" if !saw_rela && !c.val.trim().is_empty() => {
                saw_rela = true;
                a.role = txt(&c.val);
            }
            "TYPE" if !saw_type && c.val.trim().eq_ignore_ascii_case("indi") => saw_type = true,
            "NOTE" if a.note.is_empty() && !is_ptr(&c.val) => a.note = txt(&c.val),
            _ => return None,
        }
    }
    Some(a)
}

fn parse_indi(cx: &mut Cx, id: u32, n: &Node) -> Person {
    let mut p = Person::default();
    let path = "INDI";
    for k in &n.kids {
        let tag = k.tag.as_str();
        let sub = format!("{}.{}", path, tag);
        match tag {
            "NAME" => read_name(cx, &mut p, k, &sub),
            "SEX" if p.sex.is_empty() && !p.fvals.contains_key("SEX") && k.kids.is_empty() => match k.val.trim().chars().next() {
                Some('M') | Some('m') => p.sex = "M".into(),
                Some('F') | Some('f') => p.sex = "F".into(),
                _ => {
                    if !k.val.trim().is_empty() {
                        p.fvals.insert("SEX".into(), k.val.trim().to_string());
                    }
                }
            },
            "BIRT" | "DEAT" | "BURI" | "CHR" | "BAPM" if (k.val.is_empty() && k.kids.is_empty()) => {}
            "BIRT" | "DEAT" | "BURI" | "CHR" | "BAPM" if !p.fx.contains_key(if tag == "BAPM" { "CHR" } else { tag }) => {
                let key = if tag == "BAPM" { "CHR" } else { tag };
                let about = match key {
                    "BIRT" => "Birth",
                    "DEAT" => "Death",
                    "BURI" => "Burial",
                    _ => "Christening",
                };
                let mut e = ev_of(cx, k, &sub);
                for mut c in std::mem::take(&mut e.cites) {
                    c.about = about.to_string();
                    p.sources.push(c);
                }
                let (d, pl) = (std::mem::take(&mut e.date), std::mem::take(&mut e.place));
                match key {
                    "BIRT" => {
                        p.bd = d;
                        p.bp = pl;
                    }
                    "DEAT" => {
                        p.dd = d;
                        p.dp = pl;
                    }
                    "BURI" => {
                        p.bud = d;
                        p.bup = pl;
                    }
                    _ => {
                        p.chd = d;
                        p.chp = pl;
                        if tag == "BAPM" {
                            p.chtag = "BAPM".into();
                        }
                    }
                }
                p.fx.insert(key.to_string(), e);
            }
            "SOUR" => {
                let s = cite_of(cx, k, &sub);
                p.sources.push(s);
            }
            "NOTE" => {
                let (mut t, mut r, mut x) = (std::mem::take(&mut p.notes), std::mem::take(&mut p.note_refs), std::mem::take(&mut p.extra));
                note_into(cx, k, &mut t, &mut r, &mut x, &sub);
                p.notes = t;
                p.note_refs = r;
                p.extra = x;
            }
            "OBJE" => {
                let ms = media_of(cx, k, &sub);
                p.media.extend(ms);
            }
            "_TODO" if !k.val.trim().is_empty() && k.kids.iter().all(|c| c.kids.is_empty() && matches!(c.tag.as_str(), "DATE" | "STAT" | "NOTE")) => {
                let mut t = Todo { text: txt(&k.val), ..Default::default() };
                for c in &k.kids {
                    match c.tag.as_str() {
                        "DATE" => t.added = txt(&c.val),
                        "STAT" => t.done = c.val.trim().eq_ignore_ascii_case("DONE"),
                        _ => t.note = txt(&c.val),
                    }
                }
                p.todos.push(t);
            }
            "ADDR" if p.contact.addr.is_empty() && k.kids.is_empty() => p.contact.addr = txt(&k.val),
            "PHON" if p.contact.phone.is_empty() && k.kids.is_empty() => p.contact.phone = txt(&k.val),
            "EMAIL" | "_EMAIL" if p.contact.email.is_empty() && k.kids.is_empty() => p.contact.email = txt(&k.val),
            "WWW" | "_URL" if p.contact.url.is_empty() && k.kids.is_empty() => p.contact.url = txt(&k.val),
            "ASSO" if assoc_of(cx, k).is_some() => {
                let a = assoc_of(cx, k).unwrap();
                p.assoc.push(a);
            }
            "FAMS" if k.kids.is_empty() => {
                if let Some(fid) = cx.id_of(&k.val, 'F') {
                    if !p.fam_order.contains(&fid) {
                        p.fam_order.push(fid);
                    }
                }
            }
            "FAMS" => match cx.id_of(&k.val, 'F') {
                Some(fid) => {
                    if !p.fam_order.contains(&fid) {
                        p.fam_order.push(fid);
                    }
                    let mut rest = String::new();
                    for c in &k.kids {
                        node_text(cx, c, 0, &mut rest);
                        cx.keep(&format!("{}.FAMS.{}", path, c.tag));
                    }
                    p.extra.push(Raw { at: format!("FAMS@F{}", fid), text: rest });
                }
                None => {
                    let r = mk_raw(cx, "", k, path);
                    p.extra.push(r);
                }
            },
            "FAMC" => {
                match cx.id_of(&k.val, 'F') {
                    Some(fid) => {
                        let mut pedi = None;
                        let (mut frel, mut mrel): (Option<String>, Option<String>) = (None, None);
                        let mut rest = String::new();
                        for c in &k.kids {
                            if c.tag == "PEDI" && pedi.is_none() && c.kids.is_empty() {
                                pedi = Some(c.val.trim().to_lowercase());
                            } else if c.tag == "_FREL" && frel.is_none() && c.kids.is_empty() && !c.val.trim().is_empty() {
                                frel = Some(txt(&c.val));
                            } else if c.tag == "_MREL" && mrel.is_none() && c.kids.is_empty() && !c.val.trim().is_empty() {
                                mrel = Some(txt(&c.val));
                            } else if c.tag == "_PREF" && p.pref_parents.is_none() && c.kids.is_empty() && c.val.trim().eq_ignore_ascii_case("y") {
                                p.pref_parents = Some(fid);
                            } else {
                                node_text(cx, c, 0, &mut rest);
                                cx.keep(&format!("{}.FAMC.{}", path, c.tag));
                            }
                        }
                        if !rest.is_empty() {
                            p.extra.push(Raw { at: format!("FAMC@F{}", fid), text: rest });
                        }
                        cx.pedi.push((fid, id, pedi, frel, mrel));
                    }
                    None => {
                        let r = mk_raw(cx, "", k, path);
                        p.extra.push(r);
                    }
                }
            }
            _ => {
                if let Some(e) = event_from(cx, k, &sub, EVENT_TAGS) {
                    p.events.push(e);
                } else {
                    let r = mk_raw(cx, "", k, path);
                    p.extra.push(r);
                }
            }
        }
    }
    if p.name.is_empty() {
        p.name = compose_name(&p.given, &p.surname);
    }
    p
}

fn parse_fam(cx: &mut Cx, db: &Db, n: &Node) -> Family {
    let mut f = Family::default();
    let path = "FAM";
    let mut marr = false;
    for k in &n.kids {
        let tag = k.tag.as_str();
        let sub = format!("{}.{}", path, tag);
        match tag {
            "HUSB" | "WIFE" => match cx.id_of(&k.val, 'I').filter(|i| db.people.contains_key(i)) {
                Some(i) if k.kids.is_empty() => {
                    let slot = if tag == "HUSB" { &mut f.h } else { &mut f.w };
                    if slot.is_none() {
                        *slot = Some(i);
                    } else {
                        f.more.push((tag.to_string(), i));
                    }
                }
                _ => {
                    let r = mk_raw(cx, "", k, path);
                    f.extra.push(r);
                }
            },
            "CHIL" => match cx.id_of(&k.val, 'I').filter(|i| db.people.contains_key(i)) {
                Some(i) => {
                    if !f.kids.contains(&i) {
                        f.kids.push(i);
                    }
                    if !k.kids.is_empty() {
                        let mut rest = String::new();
                        for c in &k.kids {
                            node_text(cx, c, 0, &mut rest);
                            cx.keep(&format!("{}.CHIL.{}", path, c.tag));
                        }
                        f.extra.push(Raw { at: format!("CHIL@I{}", i), text: rest });
                    }
                }
                None => {
                    let r = mk_raw(cx, "", k, path);
                    f.extra.push(r);
                }
            },
            "MARR" if !marr && (!k.val.is_empty() || !k.kids.is_empty()) => {
                marr = true;
                let mut e = ev_of(cx, k, &sub);
                f.md = std::mem::take(&mut e.date);
                f.mp = std::mem::take(&mut e.place);
                f.mx = e;
            }
            "SOUR" => {
                let s = cite_of(cx, k, &sub);
                f.cites.push(s);
            }
            "NOTE" => {
                let (mut t, mut r, mut x) = (std::mem::take(&mut f.notes), std::mem::take(&mut f.note_refs), std::mem::take(&mut f.extra));
                note_into(cx, k, &mut t, &mut r, &mut x, &sub);
                f.notes = t;
                f.note_refs = r;
                f.extra = x;
            }
            "OBJE" => {
                let ms = media_of(cx, k, &sub);
                f.media.extend(ms);
            }
            "_STAT" if f.kind.is_empty() && k.kids.is_empty() && !k.val.trim().is_empty() => f.kind = txt(&k.val),
            _ => {
                if let Some(e) = event_from(cx, k, &sub, FAM_TAGS) {
                    f.events.push(e);
                } else {
                    let r = mk_raw(cx, "", k, path);
                    f.extra.push(r);
                }
            }
        }
    }
    f
}

/// A `REPO @R1@` line under a source that has nothing but a call number (and its medium) beneath it.
fn repo_link(cx: &Cx, k: &Node) -> Option<(u32, String, String)> {
    let id = cx.id_of(&k.val, 'R')?;
    let mut call = String::new();
    let mut medium = String::new();
    let mut seen = false;
    for c in &k.kids {
        if c.tag != "CALN" || seen || !c.val.is_empty() && c.val.contains('@') {
            return None;
        }
        seen = true;
        call = txt(&c.val);
        for d in &c.kids {
            if d.tag != "MEDI" || !d.kids.is_empty() || !medium.is_empty() {
                return None;
            }
            medium = txt(&d.val);
        }
    }
    Some((id, call, medium))
}

fn parse_repo(cx: &mut Cx, n: &Node) -> RepoRec {
    let mut r = RepoRec::default();
    let path = "REPO";
    for k in &n.kids {
        let sub = format!("{}.{}", path, k.tag);
        let plain = k.kids.is_empty();
        match k.tag.as_str() {
            "NAME" if r.name.is_empty() && plain => r.name = txt(&k.val),
            "ADDR" if r.addr.is_empty() && plain => r.addr = txt(&k.val),
            "PHON" if r.phone.is_empty() && plain => r.phone = txt(&k.val),
            "EMAIL" if r.email.is_empty() && plain => r.email = txt(&k.val),
            "WWW" if r.url.is_empty() && plain => r.url = txt(&k.val),
            "NOTE" => {
                let (mut t, mut rf, mut x) = (std::mem::take(&mut r.notes), std::mem::take(&mut r.note_refs), std::mem::take(&mut r.extra));
                note_into(cx, k, &mut t, &mut rf, &mut x, &sub);
                r.notes = t;
                r.note_refs = rf;
                r.extra = x;
            }
            _ => {
                let rr = mk_raw(cx, "", k, path);
                r.extra.push(rr);
            }
        }
    }
    r
}

fn parse_sour(cx: &mut Cx, n: &Node) -> SourceRec {
    let mut s = SourceRec::default();
    let path = "SOUR";
    for k in &n.kids {
        let sub = format!("{}.{}", path, k.tag);
        let plain = k.kids.is_empty();
        match k.tag.as_str() {
            "TITL" if s.title.is_empty() && plain => s.title = txt(&k.val),
            "AUTH" if s.author.is_empty() && plain => s.author = txt(&k.val),
            "ABBR" if s.abbr.is_empty() && plain => s.abbr = txt(&k.val),
            "PUBL" if s.publ.is_empty() && plain => s.publ = txt(&k.val),
            "TEXT" if s.text.is_empty() && plain => s.text = txt(&k.val),
            "WWW" if s.url.is_empty() && plain => s.url = txt(&k.val),
            "_TYPE" if s.kind.is_empty() && plain => s.kind = txt(&k.val),
            "REPO" if s.repo.is_none() && repo_link(cx, k).is_some() => {
                let (id, call, medium) = repo_link(cx, k).unwrap();
                s.repo = Some(id);
                s.call = call;
                s.medium = medium;
            }
            "NOTE" => {
                let (mut t, mut r, mut x) = (std::mem::take(&mut s.notes), std::mem::take(&mut s.note_refs), std::mem::take(&mut s.extra));
                note_into(cx, k, &mut t, &mut r, &mut x, &sub);
                s.notes = t;
                s.note_refs = r;
                s.extra = x;
            }
            _ => {
                let r = mk_raw(cx, "", k, path);
                s.extra.push(r);
            }
        }
    }
    s
}

/// Visits every stored raw block, shared-note number and source number in a database.
trait Vis {
    fn raw(&mut self, _r: &mut Raw) {}
    fn text(&mut self, _t: &mut String) {}
    fn note(&mut self, _n: &mut u32) {}
    fn src(&mut self, _s: &mut Option<u32>) {}
    fn repo(&mut self, _s: &mut Option<u32>) {}
}

fn walk(db: &mut Db, v: &mut dyn Vis) {
    fn raws(x: &mut Vec<Raw>, v: &mut dyn Vis) {
        for r in x.iter_mut() {
            v.raw(r);
        }
    }
    fn cites(x: &mut Vec<Source>, v: &mut dyn Vis) {
        for c in x.iter_mut() {
            raws(&mut c.extra, v);
            v.src(&mut c.sid);
            for n in c.note_refs.iter_mut() {
                v.note(n);
            }
        }
    }
    fn ev(e: &mut Event, v: &mut dyn Vis) {
        raws(&mut e.extra, v);
        cites(&mut e.cites, v);
        for n in e.note_refs.iter_mut() {
            v.note(n);
        }
    }
    fn media(x: &mut Vec<Media>, v: &mut dyn Vis) {
        for m in x.iter_mut() {
            raws(&mut m.extra, v);
        }
    }
    for p in db.people.values_mut() {
        raws(&mut p.extra, v);
        cites(&mut p.sources, v);
        media(&mut p.media, v);
        for n in p.note_refs.iter_mut() {
            v.note(n);
        }
        for e in p.events.iter_mut() {
            ev(e, v);
        }
        for e in p.fx.values_mut() {
            ev(e, v);
        }
        for a in p.alt_names.iter_mut() {
            raws(&mut a.extra, v);
            cites(&mut a.cites, v);
            for n in a.note_refs.iter_mut() {
                v.note(n);
            }
        }
    }
    for fm in db.fams.values_mut() {
        raws(&mut fm.extra, v);
        cites(&mut fm.cites, v);
        media(&mut fm.media, v);
        ev(&mut fm.mx, v);
        for n in fm.note_refs.iter_mut() {
            v.note(n);
        }
        for e in fm.events.iter_mut() {
            ev(e, v);
        }
    }
    for s in db.sources.values_mut() {
        raws(&mut s.extra, v);
        v.repo(&mut s.repo);
        for n in s.note_refs.iter_mut() {
            v.note(n);
        }
    }
    for r in db.repos.values_mut() {
        raws(&mut r.extra, v);
        for n in r.note_refs.iter_mut() {
            v.note(n);
        }
    }
    for n in db.notes.values_mut() {
        raws(&mut n.extra, v);
    }
    for r in db.records.iter_mut() {
        v.text(r);
    }
}

struct Mentions(HashSet<String>);
impl Vis for Mentions {
    fn raw(&mut self, r: &mut Raw) {
        self.0.extend(pointers(&r.text));
    }
    fn text(&mut self, t: &mut String) {
        self.0.extend(pointers(t));
    }
}

/// Every `@xref@` mentioned in some text.
fn pointers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line;
        while let Some(i) = rest.find('@') {
            let after = &rest[i + 1..];
            match after.find('@') {
                Some(j) if j > 0 && !after[..j].contains(char::is_whitespace) && !after.starts_with('@') && !after.starts_with('#') => {
                    out.push(after[..j].to_string());
                    rest = &after[j + 1..];
                }
                _ => rest = after,
            }
        }
    }
    out
}

/// Read GEDCOM text into a database, with a report of what was found and what was kept as-is.
pub fn parse_ged_report(text: &str) -> (Db, ImportReport) {
    let mut db = Db::default();
    let mut warnings = Vec::new();
    let roots = build_tree(text, &mut warnings);
    let mut cx = Cx { rm: HashMap::new(), objs: HashMap::new(), titles: HashMap::new(), pedi: Vec::new(), used_objs: HashSet::new(), rep: ImportReport::default() };
    cx.rep.warnings = warnings;

    // Give every record its place in the database and a stable name.
    // (If a file uses one name for two records, each still gets its own place; links go to the first.)
    let mut canon_of: Vec<String> = Vec::with_capacity(roots.len());
    for r in &roots {
        let mut canon = String::new();
        if let Some(x) = &r.xref {
            let letter = match r.tag.as_str() {
                "INDI" => 'I',
                "FAM" => 'F',
                "SOUR" => 'S',
                "NOTE" => 'N',
                "OBJE" => 'O',
                "REPO" => 'R',
                "SUBM" => 'U',
                _ => 'X',
            };
            let id = db.alloc();
            canon = format!("{}{}", letter, id);
            if cx.rm.contains_key(x) {
                cx.rep.warnings.push(format!("The record name @{}@ is used by more than one record. Each record was kept, but links to it go to the first one.", x));
            } else {
                cx.rm.insert(x.clone(), canon.clone());
            }
            match r.tag.as_str() {
                "INDI" => {
                    db.people.insert(id, Person::default());
                }
                "FAM" => {
                    db.fams.insert(id, Family::default());
                }
                _ => {}
            }
        }
        canon_of.push(canon);
    }
    // Citations that point to a source the file does not contain get an empty placeholder source,
    // so the exported file is consistent and the citation (page, details) is not lost.
    fn dangling(n: &Node, rm: &HashMap<String, String>, out: &mut Vec<String>) {
        if n.tag == "SOUR" && is_ptr(&n.val) && !rm.contains_key(n.val.trim_matches('@')) {
            out.push(n.val.trim_matches('@').to_string());
        }
        for k in &n.kids {
            dangling(k, rm, out);
        }
    }
    let mut missing: Vec<String> = Vec::new();
    for r in &roots {
        dangling(r, &cx.rm, &mut missing);
    }
    missing.sort();
    missing.dedup();
    for x in missing {
        let id = db.alloc();
        cx.rm.insert(x.clone(), format!("S{}", id));
        let title = format!("(source {} is missing from the file)", x);
        cx.titles.insert(id, title.clone());
        db.sources.insert(id, SourceRec { title, ..Default::default() });
        cx.rep.warnings.push(format!("A citation points to the source @{}@ but the file has no such source. An empty placeholder source was created for it.", x));
    }
    // Header
    for r in roots.iter().filter(|r| r.tag == "HEAD") {
        for k in &r.kids {
            match k.tag.as_str() {
                "SOUR" => {
                    let name = k.kids.iter().find(|c| c.tag == "NAME").map(|c| c.val.clone()).unwrap_or_else(|| k.val.clone());
                    let ver = k.kids.iter().find(|c| c.tag == "VERS").map(|c| c.val.clone()).unwrap_or_default();
                    cx.rep.program = format!("{} {}", name, ver).trim().to_string();
                }
                "GEDC" => {
                    if let Some(v) = k.kids.iter().find(|c| c.tag == "VERS") {
                        cx.rep.gedcom_version = v.val.clone();
                    }
                }
                // the home person, as Gilded Roots marks it when exporting
                "_HOME" => {
                    if let Some(id) = cx.rm.get(k.val.trim().trim_matches('@')).and_then(|c| c.strip_prefix('I')).and_then(|n| n.parse::<u32>().ok()) {
                        db.home = Some(id);
                    }
                }
                _ => {}
            }
        }
    }
    // Otherwise the home person is the first person in the file, as most programs (PAF,
    // Ancestry, FamilySearch...) put the home person first.
    if db.home.is_none() {
        db.home = roots
            .iter()
            .zip(canon_of.iter())
            .find(|(r, c)| r.tag == "INDI" && !c.is_empty())
            .and_then(|(_, c)| c.strip_prefix('I'))
            .and_then(|n| n.parse::<u32>().ok());
    }
    // Sources, shared notes and media objects first, because everything else points to them.
    let mut obj_records: Vec<(String, String, String)> = Vec::new();
    for (ri, r) in roots.iter().enumerate() {
        let Some(x) = &r.xref else { continue };
        let canon = canon_of[ri].clone();
        let id: u32 = canon.get(1..).and_then(|s| s.parse().ok()).unwrap_or(0);
        match r.tag.as_str() {
            "SOUR" => {
                let s = parse_sour(&mut cx, r);
                let title = if !s.title.is_empty() { s.title.clone() } else if !s.abbr.is_empty() { s.abbr.clone() } else { "(untitled source)".to_string() };
                cx.titles.insert(id, title);
                db.sources.insert(id, s);
            }
            "REPO" => {
                let rp = parse_repo(&mut cx, r);
                db.repos.insert(id, rp);
            }
            "NOTE" => {
                let mut extra = Vec::new();
                for k in &r.kids {
                    let rr = mk_raw(&mut cx, "", k, "NOTE");
                    extra.push(rr);
                }
                db.notes.insert(id, NoteRec { text: txt(&r.val), extra });
            }
            "OBJE" => {
                cx.objs.entry(x.clone()).or_insert_with(|| r.clone());
                let mut s = String::new();
                let mut rc = r.clone();
                let key = format!("\u{1}{}", canon);
                cx.rm.insert(key.clone(), canon.clone());
                rc.xref = Some(key);
                node_text(&mut cx, &rc, 0, &mut s);
                obj_records.push((x.clone(), canon, s));
            }
            _ => {}
        }
    }
    for (ri, r) in roots.iter().enumerate() {
        if r.xref.is_none() {
            continue;
        }
        let id: u32 = canon_of[ri].get(1..).and_then(|s| s.parse().ok()).unwrap_or(0);
        match r.tag.as_str() {
            "INDI" => {
                let p = parse_indi(&mut cx, id, r);
                db.people.insert(id, p);
            }
            _ => {}
        }
    }
    for (ri, r) in roots.iter().enumerate() {
        if r.xref.is_none() {
            continue;
        }
        let id: u32 = canon_of[ri].get(1..).and_then(|s| s.parse().ok()).unwrap_or(0);
        if r.tag == "FAM" {
            let f = parse_fam(&mut cx, &db, r);
            db.fams.insert(id, f);
        }
    }
    // Child links given from the child's side.
    let pedi = std::mem::take(&mut cx.pedi);
    for (fid, child, pd, fr, mr) in pedi {
        if let Some(f) = db.fams.get_mut(&fid) {
            if !f.kids.contains(&child) {
                f.kids.push(child);
                cx.rep.warnings.push(format!("{} says they are a child of a family that does not list them. They were added to it.", db.people.get(&child).map(|p| p.name.clone()).unwrap_or_default()));
            }
            if let Some(pd) = pd {
                f.pedi.insert(child, pd);
            }
            if let Some(v) = fr {
                f.frel.insert(child, v);
            }
            if let Some(v) = mr {
                f.mrel.insert(child, v);
            }
        }
    }
    // Records with no place in the model are kept whole.
    for (ri, r) in roots.iter().enumerate() {
        if r.tag == "HEAD" || r.tag == "TRLR" {
            continue;
        }
        let keep = match r.tag.as_str() {
            "INDI" | "FAM" | "SOUR" | "NOTE" | "OBJE" | "REPO" => false,
            _ => true,
        };
        if keep {
            let mut s = String::new();
            let mut rc = r.clone();
            if !canon_of[ri].is_empty() {
                let key = format!("\u{1}{}", canon_of[ri]);
                cx.rm.insert(key.clone(), canon_of[ri].clone());
                rc.xref = Some(key);
            }
            node_text(&mut cx, &rc, 0, &mut s);
            cx.keep(&format!("record.{}", r.tag));
            db.records.push(s);
        }
    }
    // Object records are only kept if something still points at them.
    let mut m = Mentions(HashSet::new());
    walk(&mut db, &mut m);
    let mentioned = m.0;
    for (x, canon, text) in obj_records {
        if mentioned.contains(&canon) || !cx.used_objs.contains(&x) {
            db.records.push(text);
        }
    }
    let n_media_before = cx.rep.media;
    cx.rep.people = db.people.len();
    cx.rep.families = db.fams.len();
    cx.rep.sources = db.sources.len();
    cx.rep.notes = db.notes.len();
    cx.rep.media = n_media_before;
    db.ver = 4;
    tidy_fam_order(&mut db);
    fix_names(&mut db);
    (db, cx.rep)
}

/// Keep a person's family order from the file only where it matters: two or more families of
/// their own, in an order other than by marriage date.
pub(crate) fn tidy_fam_order(db: &mut Db) {
    let ix = db.index();
    let mut changes: Vec<(u32, Vec<u32>)> = Vec::new();
    for (id, p) in &db.people {
        if p.fam_order.is_empty() {
            continue;
        }
        let mine = db.fams_of_ix(&ix, *id);
        let kept: Vec<u32> = p.fam_order.iter().copied().filter(|f| mine.contains(f)).collect();
        let mut by_date = mine.to_vec();
        by_date.sort_by_key(|f| (parse_date(&db.fams[f].md).0.unwrap_or(i32::MAX), *f));
        let mut chosen = kept.clone();
        chosen.extend(by_date.iter().filter(|f| !kept.contains(f)));
        let keep = mine.len() > 1 && chosen != by_date;
        changes.push((*id, if keep { kept } else { Vec::new() }));
    }
    for (id, v) in changes {
        if let Some(p) = db.people.get_mut(&id) {
            p.fam_order = v;
        }
    }
}

#[allow(dead_code)]
pub fn parse_ged(text: &str) -> Db {
    parse_ged_report(text).0
}

/// Upgrade data saved by older versions of the app. Safe to call on any database, any number of times.
pub fn migrate(db: &mut Db) {
    if db.ver >= 4 {
        return;
    }
    if db.ver < 2 {
        migrate_v2(db);
    }
    if db.ver < 3 {
        migrate_v3(db);
    }
    migrate_v4(db);
}

/// Version 4: how a child relates to each parent, the preferred parents and associations (guardian,
/// godparent...) become fields. Before, they were kept as stored GEDCOM text under the person.
fn migrate_v4(db: &mut Db) {
    let mut cx = Cx { rm: HashMap::new(), objs: HashMap::new(), titles: HashMap::new(), pedi: Vec::new(), used_objs: HashSet::new(), rep: ImportReport::default() };
    for k in db.people.keys() {
        cx.rm.insert(format!("I{}", k), format!("I{}", k));
    }
    let ids: Vec<u32> = db.people.keys().copied().collect();
    let mut rels: Vec<(u32, u32, Option<String>, Option<String>)> = Vec::new();
    for id in ids {
        let mut p = db.people.remove(&id).unwrap();
        let mut rest = Vec::new();
        for raw in std::mem::take(&mut p.extra) {
            if let Some(fid) = raw.at.strip_prefix("FAMC@F").and_then(|n| n.parse::<u32>().ok()) {
                let (mut fr, mut mr, mut left) = (None, None, String::new());
                for line in raw.text.lines() {
                    match line.strip_prefix("0 _FREL ") {
                        Some(v) if fr.is_none() && !v.trim().is_empty() => fr = Some(v.trim().to_string()),
                        _ => match line.strip_prefix("0 _MREL ") {
                            Some(v) if mr.is_none() && !v.trim().is_empty() => mr = Some(v.trim().to_string()),
                            _ => {
                                if line.trim() == "0 _PREF Y" && p.pref_parents.is_none() {
                                    p.pref_parents = Some(fid);
                                } else {
                                    left.push_str(line);
                                    left.push('\n');
                                }
                            }
                        },
                    }
                }
                if fr.is_some() || mr.is_some() {
                    rels.push((fid, id, fr, mr));
                }
                if !left.trim().is_empty() {
                    rest.push(Raw { at: raw.at, text: left });
                }
                continue;
            }
            if raw.at.is_empty() && raw.text.starts_with("0 ASSO ") {
                let mut w = Vec::new();
                let roots = build_tree(&raw.text, &mut w);
                if let Some(a) = roots.first().and_then(|n| assoc_of(&cx, n)) {
                    p.assoc.push(a);
                    continue;
                }
            }
            rest.push(raw);
        }
        p.extra = rest;
        db.people.insert(id, p);
    }
    for (fid, child, fr, mr) in rels {
        if let Some(f) = db.fams.get_mut(&fid) {
            if let Some(v) = fr {
                f.frel.insert(child, v);
            }
            if let Some(v) = mr {
                f.mrel.insert(child, v);
            }
        }
    }
    db.ver = 4;
}

/// Version 3: repositories become records of their own (before, they were kept as raw GEDCOM).
fn migrate_v3(db: &mut Db) {
    let mut cx = Cx { rm: HashMap::new(), objs: HashMap::new(), titles: HashMap::new(), pedi: Vec::new(), used_objs: HashSet::new(), rep: ImportReport::default() };
    for k in db.people.keys() {
        cx.rm.insert(format!("I{}", k), format!("I{}", k));
    }
    for k in db.fams.keys() {
        cx.rm.insert(format!("F{}", k), format!("F{}", k));
    }
    for k in db.sources.keys() {
        cx.rm.insert(format!("S{}", k), format!("S{}", k));
    }
    for k in db.notes.keys() {
        cx.rm.insert(format!("N{}", k), format!("N{}", k));
    }
    let mut keep: Vec<String> = Vec::new();
    for r in std::mem::take(&mut db.records) {
        let mut w = Vec::new();
        let roots = build_tree(&r, &mut w);
        match roots.first() {
            Some(n) if n.tag == "REPO" && n.xref.is_some() => {
                let id: Option<u32> = n.xref.as_ref().and_then(|x| x.strip_prefix('R')).and_then(|x| x.parse().ok());
                match id {
                    Some(id) => {
                        cx.rm.insert(format!("R{}", id), format!("R{}", id));
                        let rp = parse_repo(&mut cx, n);
                        db.repos.insert(id, rp);
                    }
                    None => keep.push(r),
                }
            }
            _ => keep.push(r),
        }
    }
    db.records = keep;
    let ids: Vec<u32> = db.sources.keys().copied().collect();
    for id in ids {
        let mut s = db.sources.remove(&id).unwrap();
        let mut rest = Vec::new();
        for raw in std::mem::take(&mut s.extra) {
            let mut w = Vec::new();
            let roots = if raw.at.is_empty() && raw.text.starts_with("0 REPO ") { build_tree(&raw.text, &mut w) } else { Vec::new() };
            let link = match roots.first() {
                Some(n) if s.repo.is_none() && db.repos.contains_key(&n.val.trim_matches('@').trim_start_matches('R').parse::<u32>().unwrap_or(0)) => repo_link(&cx, n),
                _ => None,
            };
            match link {
                Some((rid, call, medium)) => {
                    s.repo = Some(rid);
                    s.call = call;
                    s.medium = medium;
                }
                None => rest.push(raw),
            }
        }
        s.extra = rest;
        db.sources.insert(id, s);
    }
    db.ver = 3;
}

fn migrate_v2(db: &mut Db) {
    // Before version 2 the value of an occupation or education (or a custom event) was kept in `note`.
    for p in db.people.values_mut() {
        for e in p.events.iter_mut() {
            let value_kind = e.kind == "Occupation" || e.kind == "Education" || tag_of(&e.kind, EVENT_TAGS).is_none();
            if e.val.is_empty() && !e.note.is_empty() && value_kind {
                e.val = std::mem::take(&mut e.note);
            }
        }
    }
    db.ver = 2;
}

/* ------------------------------------------------------------------ */
/* Writing                                                             */
/* ------------------------------------------------------------------ */

#[derive(Clone, Default)]
pub struct ExportOpts {
    /// Leave out sources and notes nothing points to (used for the private copy).
    pub prune: bool,
    /// Folder name (next to the .ged file) that photos and documents are copied into.
    pub media_dir: Option<String>,
}

pub struct Export {
    pub text: String,
    /// (file inside the app's media folder, path written in the GEDCOM file)
    pub media: Vec<(String, String)>,
}

fn emit_cite(o: &mut String, db: &Db, lvl: u32, c: &Source, implied_about: &str) {
    match c.sid.filter(|s| db.sources.contains_key(s)) {
        Some(id) => putp(o, lvl, "SOUR", &format!("S{}", id)),
        None => puttxt(o, lvl, "SOUR", &c.text),
    }
    if !c.page.is_empty() {
        puttxt(o, lvl + 1, "PAGE", &c.page);
    }
    if !c.data.is_empty() || !c.ddate.is_empty() || c.extra.iter().any(|r| r.at == "DATA") {
        put(o, lvl + 1, "DATA", "");
        if !c.ddate.is_empty() {
            put(o, lvl + 2, "DATE", &c.ddate);
        }
        if !c.data.is_empty() {
            puttxt(o, lvl + 2, "TEXT", &c.data);
        }
        emit_raw(o, &c.extra, "DATA", lvl + 2);
    }
    if !c.quality.is_empty() {
        put(o, lvl + 1, "QUAY", &c.quality);
    }
    if !c.url.is_empty() {
        puttxt(o, lvl + 1, "WWW", &c.url);
    }
    if !c.accessed.is_empty() {
        puttxt(o, lvl + 1, "_ACCESSED", &c.accessed);
    }
    if !c.note.is_empty() {
        puttxt(o, lvl + 1, "NOTE", &c.note);
    }
    for n in c.note_refs.iter().filter(|n| db.notes.contains_key(n)) {
        putp(o, lvl + 1, "NOTE", &format!("N{}", n));
    }
    if !c.about.is_empty() && c.about != "General" && c.about != implied_about {
        puttxt(o, lvl + 1, "_ABOUT", &c.about);
    }
    emit_raw(o, &c.extra, "", lvl + 1);
}

fn emit_ev(o: &mut String, db: &Db, lvl: u32, tag: &str, e: &Event, typ: Option<&str>, cites: &[&Source], implied_about: &str) {
    puttxt(o, lvl, tag, &e.val);
    if !e.date.is_empty() {
        put(o, lvl + 1, "DATE", &ged_date(&e.date));
        emit_raw(o, &e.extra, "DATE", lvl + 2);
    }
    if !e.place.is_empty() {
        puttxt(o, lvl + 1, "PLAC", &e.place);
        emit_raw(o, &e.extra, "PLAC", lvl + 2);
    }
    let t = typ.map(|s| s.to_string()).unwrap_or_else(|| e.typ.clone());
    if !t.is_empty() {
        puttxt(o, lvl + 1, "TYPE", &t);
    }
    if !e.cause.is_empty() {
        puttxt(o, lvl + 1, "CAUS", &e.cause);
    }
    if !e.conf.is_empty() {
        put(o, lvl + 1, "_CONF", &e.conf);
    }
    if !e.age.is_empty() {
        put(o, lvl + 1, "AGE", &e.age);
    }
    if !e.agency.is_empty() {
        puttxt(o, lvl + 1, "AGNC", &e.agency);
    }
    if !e.temp.is_empty() {
        put(o, lvl + 1, "TEMP", &e.temp);
    }
    if !e.stat.is_empty() {
        put(o, lvl + 1, "STAT", &e.stat);
    }
    for c in e.cites.iter().chain(cites.iter().copied()) {
        emit_cite(o, db, lvl + 1, c, implied_about);
    }
    if !e.note.is_empty() {
        puttxt(o, lvl + 1, "NOTE", &e.note);
    }
    for n in e.note_refs.iter().filter(|n| db.notes.contains_key(n)) {
        putp(o, lvl + 1, "NOTE", &format!("N{}", n));
    }
    emit_raw(o, &e.extra, "", lvl + 1);
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[allow(clippy::too_many_arguments)]
fn emit_name(o: &mut String, db: &Db, kind: &str, prefix: &str, given: &str, surname: &str, suffix: &str, nick: &str, ev: &Event, force: bool) {
    if !force && given.is_empty() && surname.is_empty() && prefix.is_empty() && suffix.is_empty() && nick.is_empty() && ev.cites.is_empty() && ev.note.is_empty() {
        return;
    }
    let mut v = collapse(&format!("{} {}", prefix, given));
    if !v.is_empty() {
        v.push(' ');
    }
    v.push_str(&format!("/{}/", surname.trim()));
    if !suffix.is_empty() {
        v.push(' ');
        v.push_str(suffix.trim());
    }
    puttxt(o, 1, "NAME", &v);
    if !kind.is_empty() {
        puttxt(o, 2, "TYPE", kind);
    }
    if !prefix.is_empty() {
        puttxt(o, 2, "NPFX", prefix);
    }
    if !given.is_empty() {
        puttxt(o, 2, "GIVN", given);
    }
    if !nick.is_empty() {
        puttxt(o, 2, "NICK", nick);
    }
    if !surname.is_empty() {
        puttxt(o, 2, "SURN", surname);
    }
    if !suffix.is_empty() {
        puttxt(o, 2, "NSFX", suffix);
    }
    for c in &ev.cites {
        emit_cite(o, db, 2, c, "");
    }
    if !ev.note.is_empty() {
        puttxt(o, 2, "NOTE", &ev.note);
    }
    for n in ev.note_refs.iter().filter(|n| db.notes.contains_key(n)) {
        putp(o, 2, "NOTE", &format!("N{}", n));
    }
    emit_raw(o, &ev.extra, "", 2);
}

fn emit_media(o: &mut String, lvl: u32, m: &Media, names: &HashMap<String, String>) {
    put(o, lvl, "OBJE", "");
    let path = if !m.file.is_empty() {
        names.get(&m.file).cloned().unwrap_or_else(|| m.name.clone())
    } else {
        m.path.clone()
    };
    if path.is_empty() {
        // An object with no file (or one that is embedded in the old file): keep its title and form.
        if !m.form.is_empty() {
            puttxt(o, lvl + 1, "FORM", &m.form);
        }
        if !m.caption.is_empty() {
            puttxt(o, lvl + 1, "TITL", &m.caption);
        }
    } else {
        puttxt(o, lvl + 1, "FILE", &path);
        let form = if !m.form.is_empty() { m.form.clone() } else { ext_of(&path) };
        if !form.is_empty() {
            puttxt(o, lvl + 2, "FORM", &form);
            emit_raw(o, &m.extra, "FORM", lvl + 3);
        }
        let title = if !m.caption.is_empty() { m.caption.clone() } else { String::new() };
        if !title.is_empty() {
            puttxt(o, lvl + 2, "TITL", &title);
        }
        emit_raw(o, &m.extra, "FILE", lvl + 2);
    }
    emit_raw(o, &m.extra, "", lvl + 1);
}

fn media_names(db: &Db, dir: &Option<String>) -> (HashMap<String, String>, Vec<(String, String)>) {
    let mut names: HashMap<String, String> = HashMap::new();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    let all = db.people.values().flat_map(|p| p.media.iter()).chain(db.fams.values().flat_map(|f| f.media.iter()));
    for m in all {
        if m.file.is_empty() || names.contains_key(&m.file) {
            continue;
        }
        let base = if m.name.is_empty() { m.file.clone() } else { sanitize(&m.name) };
        let mut cand = base.clone();
        let mut n = 2;
        while !used.insert(cand.to_lowercase()) {
            let (stem, ext) = match base.rsplit_once('.') {
                Some((s, e)) => (s.to_string(), format!(".{}", e)),
                None => (base.clone(), String::new()),
            };
            cand = format!("{}-{}{}", stem, n, ext);
            n += 1;
        }
        let rel = match dir {
            Some(d) => format!("{}/{}", d, cand),
            None => cand.clone(),
        };
        names.insert(m.file.clone(), rel.clone());
        if dir.is_some() {
            files.push((m.file.clone(), rel));
        }
    }
    (names, files)
}

pub fn to_ged_full(db: &Db, opts: &ExportOpts) -> Export {
    let (names, media_files) = media_names(db, &opts.media_dir);
    let mut body = String::new();
    // Which families each person belongs to, worked out once (looking it up per person is far too slow for big trees).
    let mut as_child: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut as_partner: HashMap<u32, Vec<u32>> = HashMap::new();
    for (fid, f) in &db.fams {
        for k in &f.kids {
            as_child.entry(*k).or_default().push(*fid);
        }
        for x in f.h.iter().chain(f.w.iter()).chain(f.more.iter().map(|(_, x)| x)) {
            let v = as_partner.entry(*x).or_default();
            if !v.contains(fid) {
                v.push(*fid);
            }
        }
    }
    for (p, v) in as_partner.iter_mut() {
        v.sort_by_key(|f| db.fam_rank(*p, *f));
    }
    // The home person goes first: most programs take the first person in a file as its home
    // person. (The header also names them, for programs that read that.)
    let home = db.home.filter(|h| db.people.contains_key(h));
    let order: Vec<(&u32, &Person)> = home.and_then(|h| db.people.get_key_value(&h)).into_iter().chain(db.people.iter().filter(|(id, _)| Some(**id) != home)).collect();
    for (id, p) in order {
        let o = &mut body;
        putp_rec(o, &format!("I{}", id), "INDI");
        // Names
        let (gn, sn) = if p.given.is_empty() && p.surname.is_empty() { split_name(&p.name) } else { (p.given.clone(), p.surname.clone()) };
        let nev = p.fx.get("NAME").cloned().unwrap_or_default();
        emit_name(o, db, &nev.typ.clone(), &p.prefix, &gn, &sn, &p.suffix, &p.nick, &nev, false);
        if !p.maiden.is_empty() {
            let t = p.fvals.get("MAIDEN_TYPE").cloned().filter(|t| !t.is_empty()).unwrap_or_else(|| "birth".to_string());
            emit_name(o, db, &t, "", &gn, &p.maiden, "", "", &Event::default(), true);
        }
        for a in &p.alt_names {
            let ev = Event { cites: a.cites.clone(), note: a.note.clone(), note_refs: a.note_refs.clone(), extra: a.extra.clone(), ..Default::default() };
            emit_name(o, db, &a.kind, &a.prefix, &a.given, &a.surname, &a.suffix, &a.nick, &ev, true);
        }
        if !p.sex.is_empty() {
            put(o, 1, "SEX", &p.sex);
        } else if let Some(v) = p.fvals.get("SEX") {
            put(o, 1, "SEX", v);
        }
        // Birth, christening, death, burial
        let facts: [(&str, &str, &str, &str, &str); 4] = [
            ("BIRT", "Birth", &p.bd, &p.bp, "BIRT"),
            (if p.chtag == "BAPM" { "BAPM" } else { "CHR" }, "Christening", &p.chd, &p.chp, "CHR"),
            ("DEAT", "Death", &p.dd, &p.dp, "DEAT"),
            ("BURI", "Burial", &p.bud, &p.bup, "BURI"),
        ];
        for (tag, about, d, pl, key) in facts {
            let cites: Vec<&Source> = p.sources.iter().filter(|s| s.about == about).collect();
            let has_fx = p.fx.contains_key(key);
            if d.is_empty() && pl.is_empty() && cites.is_empty() && !has_fx {
                continue;
            }
            let mut e = p.fx.get(key).cloned().unwrap_or_default();
            e.date = d.to_string();
            e.place = pl.to_string();
            emit_ev(o, db, 1, tag, &e, None, &cites, about);
        }
        for e in &p.events {
            let tag = if !e.tag.is_empty() { Some(e.tag.as_str()) } else { tag_of(&e.kind, EVENT_TAGS) };
            match tag {
                Some("EVEN") | None => {
                    let t = if e.kind == "Other" || e.kind.is_empty() { None } else { Some(e.kind.as_str()) };
                    emit_ev(o, db, 1, "EVEN", e, Some(t.unwrap_or("")), &[], "");
                }
                Some(t) => emit_ev(o, db, 1, t, e, None, &[], ""),
            }
        }
        for c in p.sources.iter().filter(|s| !matches!(s.about.as_str(), "Birth" | "Christening" | "Death" | "Burial")) {
            emit_cite(o, db, 1, c, "");
        }
        for m in &p.media {
            emit_media(o, 1, m, &names);
        }
        for fid in as_child.get(id).map(|v| v.as_slice()).unwrap_or(&[]) {
            putp(o, 1, "FAMC", &format!("F{}", fid));
            let fam = &db.fams[fid];
            if let Some(pd) = fam.pedi.get(id) {
                put(o, 2, "PEDI", pd);
            }
            if let Some(v) = fam.frel.get(id).filter(|v| !v.is_empty()) {
                puttxt(o, 2, "_FREL", v);
            }
            if let Some(v) = fam.mrel.get(id).filter(|v| !v.is_empty()) {
                puttxt(o, 2, "_MREL", v);
            }
            if p.pref_parents == Some(*fid) {
                put(o, 2, "_PREF", "Y");
            }
            emit_raw(o, &p.extra, &format!("FAMC@F{}", fid), 2);
        }
        for (tag, v) in [("ADDR", &p.contact.addr), ("PHON", &p.contact.phone), ("EMAIL", &p.contact.email), ("WWW", &p.contact.url)] {
            if !v.trim().is_empty() {
                puttxt(o, 1, tag, v.trim());
            }
        }
        for a in p.assoc.iter().filter(|a| db.people.contains_key(&a.other) && a.other != *id) {
            putp(o, 1, "ASSO", &format!("I{}", a.other));
            put(o, 2, "TYPE", "INDI");
            if !a.role.is_empty() {
                puttxt(o, 2, "RELA", &a.role);
            }
            if !a.note.is_empty() {
                puttxt(o, 2, "NOTE", &a.note);
            }
        }
        for t in p.todos.iter().filter(|t| !t.text.trim().is_empty()) {
            puttxt(o, 1, "_TODO", &t.text);
            if !t.added.is_empty() {
                puttxt(o, 2, "DATE", &t.added);
            }
            put(o, 2, "STAT", if t.done { "DONE" } else { "OPEN" });
            if !t.note.is_empty() {
                puttxt(o, 2, "NOTE", &t.note);
            }
        }
        for f in as_partner.get(id).map(|v| v.as_slice()).unwrap_or(&[]) {
            putp(o, 1, "FAMS", &format!("F{}", f));
            emit_raw(o, &p.extra, &format!("FAMS@F{}", f), 2);
        }
        for t in p.notes.split("\n\n").filter(|t| !t.trim().is_empty()) {
            puttxt(o, 1, "NOTE", t.trim_end_matches('\n'));
        }
        for n in p.note_refs.iter().filter(|n| db.notes.contains_key(n)) {
            putp(o, 1, "NOTE", &format!("N{}", n));
        }
        emit_raw(o, &p.extra, "", 1);
    }
    for (id, f) in &db.fams {
        let o = &mut body;
        putp_rec(o, &format!("F{}", id), "FAM");
        if let Some(h) = f.h {
            putp(o, 1, "HUSB", &format!("I{}", h));
        }
        if let Some(w) = f.w {
            putp(o, 1, "WIFE", &format!("I{}", w));
        }
        for (role, x) in &f.more {
            putp(o, 1, role, &format!("I{}", x));
        }
        for k in &f.kids {
            putp(o, 1, "CHIL", &format!("I{}", k));
            emit_raw(o, &f.extra, &format!("CHIL@I{}", k), 2);
        }
        if !f.kind.is_empty() {
            puttxt(o, 1, "_STAT", &f.kind);
        }
        let mcites: Vec<&Source> = Vec::new();
        if !f.md.is_empty() || !f.mp.is_empty() || f.mx.val != "" || f.fvals.contains_key("MARR") || has_detail(&f.mx) {
            let mut e = f.mx.clone();
            e.date = f.md.clone();
            e.place = f.mp.clone();
            emit_ev(o, db, 1, "MARR", &e, None, &mcites, "Marriage");
        }
        for e in &f.events {
            let tag = if !e.tag.is_empty() { Some(e.tag.as_str()) } else { tag_of(&e.kind, FAM_TAGS) };
            match tag {
                Some("EVEN") | None => {
                    let t = if e.kind == "Other" || e.kind.is_empty() { None } else { Some(e.kind.as_str()) };
                    emit_ev(o, db, 1, "EVEN", e, Some(t.unwrap_or("")), &[], "");
                }
                Some(t) => emit_ev(o, db, 1, t, e, None, &[], ""),
            }
        }
        for c in &f.cites {
            emit_cite(o, db, 1, c, "");
        }
        for m in &f.media {
            emit_media(o, 1, m, &names);
        }
        for t in f.notes.split("\n\n").filter(|t| !t.trim().is_empty()) {
            puttxt(o, 1, "NOTE", t.trim_end_matches('\n'));
        }
        for n in f.note_refs.iter().filter(|n| db.notes.contains_key(n)) {
            putp(o, 1, "NOTE", &format!("N{}", n));
        }
        emit_raw(o, &f.extra, "", 1);
    }
    // Source and note records, each as its own block so unused ones can be left out of a private copy.
    let mut blocks: BTreeMap<String, String> = BTreeMap::new();
    for (id, s) in &db.sources {
        let mut b = String::new();
        putp_rec(&mut b, &format!("S{}", id), "SOUR");
        if !s.title.is_empty() {
            puttxt(&mut b, 1, "TITL", &s.title);
        }
        if !s.author.is_empty() {
            puttxt(&mut b, 1, "AUTH", &s.author);
        }
        if !s.abbr.is_empty() {
            puttxt(&mut b, 1, "ABBR", &s.abbr);
        }
        if !s.publ.is_empty() {
            puttxt(&mut b, 1, "PUBL", &s.publ);
        }
        if !s.text.is_empty() {
            puttxt(&mut b, 1, "TEXT", &s.text);
        }
        if !s.url.is_empty() {
            puttxt(&mut b, 1, "WWW", &s.url);
        }
        if !s.kind.is_empty() {
            puttxt(&mut b, 1, "_TYPE", &s.kind);
        }
        if let Some(r) = s.repo.filter(|r| db.repos.contains_key(r)) {
            putp(&mut b, 1, "REPO", &format!("R{}", r));
            if !s.call.is_empty() || !s.medium.is_empty() {
                puttxt(&mut b, 2, "CALN", &s.call);
                if !s.medium.is_empty() {
                    puttxt(&mut b, 3, "MEDI", &s.medium);
                }
            }
        }
        if !s.notes.is_empty() {
            puttxt(&mut b, 1, "NOTE", &s.notes);
        }
        for n in s.note_refs.iter().filter(|n| db.notes.contains_key(n)) {
            putp(&mut b, 1, "NOTE", &format!("N{}", n));
        }
        emit_raw(&mut b, &s.extra, "", 1);
        blocks.insert(format!("S{}", id), b);
    }
    for (id, r) in &db.repos {
        let mut b = String::new();
        putp_rec(&mut b, &format!("R{}", id), "REPO");
        if !r.name.is_empty() {
            puttxt(&mut b, 1, "NAME", &r.name);
        }
        if !r.addr.is_empty() {
            puttxt(&mut b, 1, "ADDR", &r.addr);
        }
        if !r.phone.is_empty() {
            puttxt(&mut b, 1, "PHON", &r.phone);
        }
        if !r.email.is_empty() {
            puttxt(&mut b, 1, "EMAIL", &r.email);
        }
        if !r.url.is_empty() {
            puttxt(&mut b, 1, "WWW", &r.url);
        }
        if !r.notes.is_empty() {
            puttxt(&mut b, 1, "NOTE", &r.notes);
        }
        for n in r.note_refs.iter().filter(|n| db.notes.contains_key(n)) {
            putp(&mut b, 1, "NOTE", &format!("N{}", n));
        }
        emit_raw(&mut b, &r.extra, "", 1);
        blocks.insert(format!("R{}", id), b);
    }
    for (id, n) in &db.notes {
        let mut b = String::new();
        let mut first = String::new();
        puttxt(&mut first, 0, "NOTE", &n.text);
        // The first line of a record carries its name: 0 @N1@ NOTE text
        let mut lines = first.lines();
        let l0 = lines.next().unwrap_or("0 NOTE");
        let rest = l0.strip_prefix("0 NOTE").unwrap_or("");
        let _ = writeln!(b, "0 @N{}@ NOTE{}", id, rest);
        for l in lines {
            let _ = writeln!(b, "{}", l);
        }
        emit_raw(&mut b, &n.extra, "", 1);
        blocks.insert(format!("N{}", id), b);
    }
    let mut records: Vec<(Option<String>, String)> = Vec::new();
    for r in &db.records {
        let first = r.lines().next().unwrap_or("");
        let mut it = first.split_whitespace();
        let _ = it.next();
        let (x, tag) = match (it.next(), it.next()) {
            (Some(a), Some(b)) if a.starts_with('@') => (Some(a.trim_matches('@').to_string()), b.to_string()),
            (Some(a), _) => (None, a.to_string()),
            _ => (None, String::new()),
        };
        records.push((x.map(|x| format!("{}\u{0}{}", tag, x)), r.clone()));
    }

    let mut out = String::new();
    let subm = records.iter().find_map(|(k, _)| k.as_ref().and_then(|k| k.strip_prefix("SUBM\u{0}")).map(|s| s.to_string()));
    let _ = write!(out, "0 HEAD\n1 SOUR GildedRoots\n2 NAME Gilded Roots\n2 VERS {}\n1 GEDC\n2 VERS 5.5.1\n2 FORM LINEAGE-LINKED\n1 CHAR UTF-8\n", env!("CARGO_PKG_VERSION"));
    if let Some(h) = home {
        let _ = writeln!(out, "1 _HOME @I{}@", h);
    }
    let subm_x = subm.clone().unwrap_or_else(|| "SUBM".to_string());
    let _ = writeln!(out, "1 SUBM @{}@", subm_x);
    if subm.is_none() {
        let _ = writeln!(out, "0 @SUBM@ SUBM\n1 NAME Gilded Roots user");
    }
    out.push_str(&body);

    // Decide which source and note records go out.
    let mut emit_keys: Vec<String> = Vec::new();
    if opts.prune {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut work: Vec<String> = pointers(&body);
        while let Some(x) = work.pop() {
            if !seen.insert(x.clone()) {
                continue;
            }
            if let Some(b) = blocks.get(&x) {
                work.extend(pointers(b));
            }
        }
        emit_keys.extend(blocks.keys().filter(|k| seen.contains(*k)).cloned());
    } else {
        emit_keys.extend(blocks.keys().cloned());
    }
    for k in &emit_keys {
        out.push_str(&blocks[k]);
    }
    for (k, r) in &records {
        let is_subm = k.as_ref().map_or(false, |k| k.starts_with("SUBM\u{0}"));
        if opts.prune && !is_subm {
            continue;
        }
        out.push_str(r);
        if !r.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str("0 TRLR\n");
    Export { text: out, media: media_files }
}

fn has_detail(e: &Event) -> bool {
    !e.typ.is_empty() || !e.cause.is_empty() || !e.age.is_empty() || !e.agency.is_empty() || !e.note.is_empty() || !e.note_refs.is_empty() || !e.cites.is_empty() || !e.extra.is_empty() || !e.val.is_empty()
}

fn putp_rec(o: &mut String, canon: &str, tag: &str) {
    let _ = writeln!(o, "0 @{}@ {}", canon, tag);
}

pub fn to_ged(db: &Db) -> String {
    to_ged_full(db, &ExportOpts::default()).text
}

/* ------------------------------------------------------------------ */
/* Merging another tree into this one                                  */
/* ------------------------------------------------------------------ */

/// Replace every `@old@` that has an entry in the map.
fn remap_text(text: &str, xm: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('@') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('@') {
            Some(j) if j > 0 && !after[..j].contains(char::is_whitespace) && !after.starts_with('#') => {
                match xm.get(&after[..j]) {
                    Some(n) => {
                        out.push('@');
                        out.push_str(n);
                        out.push('@');
                    }
                    None => out.push_str(&rest[i..i + 2 + j]),
                }
                rest = &after[j + 1..];
            }
            _ => {
                out.push('@');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

struct Remap<'a> {
    xm: &'a HashMap<String, String>,
    nmap: &'a HashMap<u32, u32>,
    smap: &'a HashMap<u32, u32>,
    rmap: &'a HashMap<u32, u32>,
}
impl Vis for Remap<'_> {
    fn raw(&mut self, r: &mut Raw) {
        r.text = remap_text(&r.text, self.xm);
        if let Some((head, tail)) = r.at.split_once('@') {
            if let Some(n) = self.xm.get(tail) {
                r.at = format!("{}@{}", head, n);
            }
        }
    }
    fn text(&mut self, t: &mut String) {
        *t = remap_text(t, self.xm);
    }
    fn note(&mut self, n: &mut u32) {
        if let Some(x) = self.nmap.get(n) {
            *n = *x;
        }
    }
    fn src(&mut self, s: &mut Option<u32>) {
        if let Some(Some(x)) = s.as_ref().map(|i| self.smap.get(i)) {
            *s = Some(*x);
        }
    }
    fn repo(&mut self, s: &mut Option<u32>) {
        if let Some(Some(x)) = s.as_ref().map(|i| self.rmap.get(i)) {
            *s = Some(*x);
        }
    }
}


struct RepPtr<'a> {
    from: &'a str,
    xm: HashMap<String, String>,
    to: bool,
}
impl Vis for RepPtr<'_> {
    fn raw(&mut self, r: &mut Raw) {
        if let Some((head, tail)) = r.at.split_once('@') {
            if tail == self.from {
                match self.xm.get(self.from) {
                    Some(n) => r.at = format!("{}@{}", head, n),
                    None => r.text.clear(),
                }
            }
        }
        if pointers(&r.text).iter().any(|p| p == self.from) {
            if self.to {
                r.text = remap_text(&r.text, &self.xm);
            } else {
                r.text.clear();
            }
        }
    }
}

/// A person or family is gone (deleted, or merged into another): stored GEDCOM blocks that point at it
/// are pointed at `to` instead, or dropped when `to` is None, so an export never has a link to nothing.
pub fn replace_pointer(db: &mut Db, from: &str, to: Option<&str>) {
    let mut xm = HashMap::new();
    if let Some(t) = to {
        xm.insert(from.to_string(), t.to_string());
    }
    walk(db, &mut RepPtr { from, xm, to: to.is_some() });
    // Links kept as fields rather than as stored GEDCOM text.
    let num = |s: &str, letter: char| s.strip_prefix(letter).and_then(|n| n.parse::<u32>().ok());
    if let Some(n) = num(from, 'I') {
        let to_id = to.and_then(|t| num(t, 'I'));
        for b in db.bookmarks.iter_mut() {
            if *b == n {
                *b = to_id.unwrap_or(0);
            }
        }
        let mut seen_b = HashSet::new();
        db.bookmarks.retain(|b| *b != 0 && seen_b.insert(*b));
        for (pid, p) in db.people.iter_mut() {
            for a in p.assoc.iter_mut() {
                if a.other == n {
                    a.other = to_id.unwrap_or(0);
                }
            }
            let me = *pid;
            let mut seen: HashSet<(u32, String)> = HashSet::new();
            p.assoc.retain(|a| a.other != 0 && a.other != me && seen.insert((a.other, a.role.clone())));
            for m in p.media.iter_mut() {
                for f in m.faces.iter_mut() {
                    if f.who == n {
                        f.who = to_id.unwrap_or(0);
                    }
                }
                m.faces.retain(|f| f.who != 0);
            }
            if let Some(fp) = p.face_portrait.as_mut() {
                if fp.owner == n {
                    match to_id {
                        Some(t) => fp.owner = t,
                        None => p.face_portrait = None,
                    }
                }
            }
        }
    }
    if let Some(n) = num(from, 'F') {
        let to_id = to.and_then(|t| num(t, 'F'));
        for p in db.people.values_mut() {
            if p.pref_parents == Some(n) {
                p.pref_parents = to_id;
            }
            if p.fam_order.contains(&n) {
                p.fam_order = p.fam_order.iter().filter_map(|f| if *f == n { to_id } else { Some(*f) }).collect();
                let mut seen = HashSet::new();
                p.fam_order.retain(|f| seen.insert(*f));
            }
        }
    }
}

/// How "Renumber people" puts people in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenumberOrder {
    /// The home person first, then their family outwards: parents and grandparents first,
    /// then spouses and children, then everyone else in their present order.
    Family,
    /// Everyone keeps their present order; only the gaps between numbers close up.
    Present,
    /// Alphabetical by surname, then given names, then birth year.
    Alphabetical,
}

/// The order people would get, as a list of their present numbers.
pub fn renumber_order(db: &Db, how: RenumberOrder) -> Vec<u32> {
    let all: Vec<u32> = db.people.keys().copied().collect();
    match how {
        RenumberOrder::Present => all,
        RenumberOrder::Alphabetical => {
            let mut v = all;
            let key = |id: &u32| {
                let p = &db.people[id];
                let (g, s) = if p.given.is_empty() && p.surname.is_empty() { split_name(&p.name) } else { (p.given.clone(), p.surname.clone()) };
                // people with no name at all go at the end
                (s.trim().is_empty() && g.trim().is_empty(), s.to_lowercase(), g.to_lowercase(), year(&p.bd).parse::<i32>().unwrap_or(i32::MAX), *id)
            };
            v.sort_by_cached_key(key);
            v
        }
        RenumberOrder::Family => {
            // parents of each person (preferred family first), partners and children
            let mut parents: HashMap<u32, Vec<u32>> = HashMap::new();
            let mut partners: HashMap<u32, Vec<u32>> = HashMap::new();
            let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
            for (fid, f) in &db.fams {
                let couple: Vec<u32> = f.h.iter().chain(f.w.iter()).copied().collect();
                for c in &f.kids {
                    let pref = db.people.get(c).and_then(|p| p.pref_parents) == Some(*fid);
                    let v = parents.entry(*c).or_default();
                    if pref {
                        for (i, x) in couple.iter().enumerate() {
                            v.insert(i, *x);
                        }
                    } else {
                        v.extend(couple.iter().copied());
                    }
                }
                for x in &couple {
                    partners.entry(*x).or_default().extend(couple.iter().filter(|y| *y != x));
                    kids.entry(*x).or_default().extend(f.kids.iter().copied());
                }
            }
            let mut seen: HashSet<u32> = HashSet::new();
            let mut out: Vec<u32> = Vec::with_capacity(all.len());
            let start = db.home.filter(|h| db.people.contains_key(h)).or_else(|| all.first().copied());
            if let Some(h) = start {
                // ancestors first, a generation at a time (father before mother), as on a pedigree
                let mut gen = vec![h];
                seen.insert(h);
                out.push(h);
                while !gen.is_empty() {
                    let mut next = Vec::new();
                    for x in &gen {
                        for p in parents.get(x).into_iter().flatten() {
                            if db.people.contains_key(p) && seen.insert(*p) {
                                out.push(*p);
                                next.push(*p);
                            }
                        }
                    }
                    gen = next;
                }
                // then everyone joined to them, nearest first: partners, children, and onwards
                let mut q: std::collections::VecDeque<u32> = out.iter().copied().collect();
                while let Some(x) = q.pop_front() {
                    for y in partners.get(&x).into_iter().flatten().chain(kids.get(&x).into_iter().flatten()).chain(parents.get(&x).into_iter().flatten()) {
                        if db.people.contains_key(y) && seen.insert(*y) {
                            out.push(*y);
                            q.push_back(*y);
                        }
                    }
                }
            }
            out.extend(all.into_iter().filter(|x| !seen.contains(x)));
            out
        }
    }
}

/// Give people new numbers 1, 2, 3... in `order` (from renumber_order), and families new
/// numbers 1, 2, 3... in the order their people now come. Every link is updated: families,
/// preferred parents, associations, face tags, profile pictures, bookmarks, the journal, DNA,
/// the home person and the GEDCOM details kept as text. Each person keeps their old number as a
/// reference (REFN, type "Old ID"), so older printouts and notes can still be matched.
/// Returns the new number of each person (old -> new).
pub fn renumber(db: &mut Db, order: &[u32]) -> HashMap<u32, u32> {
    let mut pm: HashMap<u32, u32> = HashMap::new();
    for id in order.iter().chain(db.people.keys()) {
        if db.people.contains_key(id) && !pm.contains_key(id) {
            let n = pm.len() as u32 + 1;
            pm.insert(*id, n);
        }
    }
    // families in the order their first member now comes
    let mut fams: Vec<(u32, u32)> = db
        .fams
        .iter()
        .map(|(fid, f)| {
            let first = f.h.iter().chain(f.w.iter()).chain(f.kids.iter()).chain(f.more.iter().map(|(_, x)| x)).filter_map(|x| pm.get(x)).min().copied().unwrap_or(u32::MAX);
            (first, *fid)
        })
        .collect();
    fams.sort();
    let fm: HashMap<u32, u32> = fams.iter().enumerate().map(|(i, (_, f))| (*f, i as u32 + 1)).collect();
    let p = |x: u32| pm.get(&x).copied().unwrap_or(x);
    let f = |x: u32| fm.get(&x).copied().unwrap_or(x);
    // GEDCOM details kept as text
    let mut xm: HashMap<String, String> = HashMap::new();
    for (o, n) in &pm {
        xm.insert(format!("I{}", o), format!("I{}", n));
    }
    for (o, n) in &fm {
        xm.insert(format!("F{}", o), format!("F{}", n));
    }
    let none: HashMap<u32, u32> = HashMap::new();
    walk(db, &mut Remap { xm: &xm, nmap: &none, smap: &none, rmap: &none });
    for r in db.records.iter_mut() {
        if r.contains('@') {
            *r = remap_text(r, &xm);
        }
    }
    // people
    let old_people = std::mem::take(&mut db.people);
    for (old, mut person) in old_people {
        person.pref_parents = person.pref_parents.map(f);
        person.fam_order = person.fam_order.iter().map(|x| f(*x)).collect();
        for a in person.assoc.iter_mut() {
            a.other = p(a.other);
        }
        for m in person.media.iter_mut() {
            for fc in m.faces.iter_mut() {
                fc.who = p(fc.who);
            }
        }
        if let Some(fp) = person.face_portrait.as_mut() {
            fp.owner = p(fp.owner);
        }
        let new = p(old);
        if new != old {
            // keep one "Old ID": the number before this renumbering
            person.extra.retain(|r| !(r.at.is_empty() && r.text.starts_with("0 REFN ") && r.text.contains("\n1 TYPE Old ID")));
            person.extra.push(Raw { at: String::new(), text: format!("0 REFN {}\n1 TYPE Old ID\n", old) });
        }
        db.people.insert(new, person);
    }
    // families
    let old_fams = std::mem::take(&mut db.fams);
    for (old, mut fam) in old_fams {
        fam.h = fam.h.map(p);
        fam.w = fam.w.map(p);
        for k in fam.kids.iter_mut() {
            *k = p(*k);
        }
        for (_, x) in fam.more.iter_mut() {
            *x = p(*x);
        }
        fam.pedi = std::mem::take(&mut fam.pedi).into_iter().map(|(k, v)| (p(k), v)).collect();
        fam.frel = std::mem::take(&mut fam.frel).into_iter().map(|(k, v)| (p(k), v)).collect();
        fam.mrel = std::mem::take(&mut fam.mrel).into_iter().map(|(k, v)| (p(k), v)).collect();
        db.fams.insert(f(old), fam);
    }
    // everything else that names a person
    for b in db.bookmarks.iter_mut() {
        *b = p(*b);
    }
    for j in db.journal.iter_mut() {
        for x in j.people.iter_mut() {
            *x = p(*x);
        }
    }
    for t in db.dna.iter_mut() {
        t.person = t.person.map(p);
        for m in t.matches.iter_mut() {
            m.link = m.link.map(p);
        }
    }
    db.home = db.home.map(p);
    // new records continue after the highest number in use (people, families, sources...)
    let top = [db.people.keys().max(), db.fams.keys().max(), db.sources.keys().max(), db.repos.keys().max(), db.notes.keys().max()].into_iter().flatten().max().copied().unwrap_or(0);
    db.next = db.next.max(top);
    pm
}

fn add_unique<T: Clone>(dst: &mut Vec<T>, src: &[T], same: impl Fn(&T, &T) -> bool) {
    for x in src {
        if !dst.iter().any(|y| same(y, x)) {
            dst.push(x.clone());
        }
    }
}

fn fill(a: &mut String, b: &str) {
    if a.is_empty() {
        *a = b.to_string();
    }
}

fn merge_event_details(a: &mut Event, b: &Event) {
    fill(&mut a.val, &b.val);
    fill(&mut a.typ, &b.typ);
    fill(&mut a.cause, &b.cause);
    fill(&mut a.age, &b.age);
    fill(&mut a.agency, &b.agency);
    fill(&mut a.temp, &b.temp);
    fill(&mut a.stat, &b.stat);
    if !b.note.is_empty() && !a.note.contains(&b.note) {
        push_note(&mut a.note, &b.note);
    }
    add_unique(&mut a.note_refs, &b.note_refs, |x, y| x == y);
    add_unique(&mut a.cites, &b.cites, same_cite);
}

fn same_cite(a: &Source, b: &Source) -> bool {
    a.sid == b.sid && a.text == b.text && a.page == b.page && a.about == b.about
}

fn same_event(a: &Event, b: &Event) -> bool {
    a.kind == b.kind && a.date == b.date && a.place == b.place && a.val == b.val
}

/// Add what `f` knows to family `e` (the same couple recorded twice).
pub fn absorb_family(e: &mut Family, f: Family) {
    if e.h.is_none() {
        e.h = f.h;
    }
    if e.w.is_none() {
        e.w = f.w;
    }
    for c in f.kids {
        if !e.kids.contains(&c) {
            e.kids.push(c);
        }
    }
    for (c, v) in f.pedi {
        e.pedi.entry(c).or_insert(v);
    }
    add_unique(&mut e.more, &f.more, |a, b| a == b);
    fill(&mut e.kind, &f.kind);
    for (c, v) in f.frel {
        e.frel.entry(c).or_insert(v);
    }
    for (c, v) in f.mrel {
        e.mrel.entry(c).or_insert(v);
    }
    fill(&mut e.md, &f.md);
    fill(&mut e.mp, &f.mp);
    merge_event_details(&mut e.mx, &f.mx);
    add_unique(&mut e.events, &f.events, same_event);
    add_unique(&mut e.cites, &f.cites, same_cite);
    add_unique(&mut e.note_refs, &f.note_refs, |a, b| a == b);
    add_unique(&mut e.media, &f.media, |a, b| a.file == b.file && a.path == b.path && a.name == b.name);
    add_unique(&mut e.extra, &f.extra, |a, b| a.at == b.at && a.text == b.text);
    for (k, v) in f.fvals {
        e.fvals.entry(k).or_insert(v);
    }
    if !f.notes.is_empty() && !e.notes.contains(&f.notes) {
        push_note(&mut e.notes, &f.notes);
    }
}

/// Lower-case names of a person's parents, spouses and children.
#[cfg(test)]
fn relative_names(db: &Db, id: u32) -> HashSet<String> {
    let mut out = HashSet::new();
    for f in db.fams.values() {
        let in_family = f.h == Some(id) || f.w == Some(id) || f.more.iter().any(|(_, x)| *x == id);
        let is_child = f.kids.contains(&id);
        if in_family {
            for x in f.h.iter().chain(f.w.iter()).chain(f.kids.iter()).chain(f.more.iter().map(|(_, x)| x)) {
                if *x != id {
                    if let Some(p) = db.people.get(x) {
                        out.insert(p.name.to_lowercase());
                    }
                }
            }
        }
        if is_child {
            for x in f.h.iter().chain(f.w.iter()) {
                if let Some(p) = db.people.get(x) {
                    out.insert(p.name.to_lowercase());
                }
            }
        }
    }
    out.remove("");
    out
}

/// Which incoming people look like people already in the tree, by the old automatic rule
/// (same name and birth year, or same name and a shared relative). Only used by tests now:
/// the app never merges two people without the user confirming each pair.
#[cfg(test)]
pub fn auto_pairs(dst: &Db, inc: &Db) -> HashMap<u32, u32> {
    let mut pairs: HashMap<u32, u32> = HashMap::new();
    let inc_rels: HashMap<u32, HashSet<String>> = inc.people.keys().map(|i| (*i, relative_names(inc, *i))).collect();
    for (id, p) in &inc.people {
        let name = p.name.to_lowercase();
        let (by, dy) = (year(&p.bd), year(&p.dd));
        if name.is_empty() {
            continue;
        }
        let found = dst
            .people
            .iter()
            .find(|(k, q)| {
                if q.name.to_lowercase() != name || pairs.values().any(|v| v == *k) {
                    return false;
                }
                let (qb, qd) = (year(&q.bd), year(&q.dd));
                if !by.is_empty() && !qb.is_empty() {
                    return by == qb;
                }
                if !dy.is_empty() && !qd.is_empty() && dy != qd {
                    return false;
                }
                !inc_rels[id].is_disjoint(&relative_names(dst, **k))
            })
            .map(|(k, _)| *k);
        if let Some(k) = found {
            pairs.insert(*id, k);
        }
    }
    pairs
}

#[cfg(test)]
pub fn merge_into(dst: &mut Db, inc: Db) -> (usize, usize) {
    let pairs = auto_pairs(dst, &inc);
    merge_into_pairs(dst, inc, &pairs)
}

/// Facts the incoming record gives that differ from what the tree already says.
fn differing(q: &Person, p: &Person) -> Vec<String> {
    let mut out = Vec::new();
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    for (label, a, b) in [
        ("Given name", &q.given, &p.given),
        ("Surname", &q.surname, &p.surname),
        ("Maiden name", &q.maiden, &p.maiden),
        ("Sex", &q.sex, &p.sex),
        ("Born", &q.bd, &p.bd),
        ("Birthplace", &q.bp, &p.bp),
        ("Christened", &q.chd, &p.chd),
        ("Christening place", &q.chp, &p.chp),
        ("Died", &q.dd, &p.dd),
        ("Place of death", &q.dp, &p.dp),
        ("Buried", &q.bud, &p.bud),
        ("Burial place", &q.bup, &p.bup),
    ] {
        if !a.is_empty() && !b.is_empty() && norm(a) != norm(b) {
            // A fuller version of the same name is not a disagreement.
            let (wa, wb): (HashSet<String>, HashSet<String>) = (norm(a).split(' ').map(String::from).collect(), norm(b).split(' ').map(String::from).collect());
            if matches!(label, "Given name" | "Surname" | "Maiden name") && (wa.is_subset(&wb) || wb.is_subset(&wa)) {
                continue;
            }
            out.push(format!("{}: {}", label, b));
        }
    }
    out
}

/// Add the people, families, sources and notes of `inc` to `dst`. `pairs` says which incoming people
/// are the same person as someone already in the tree (incoming number -> tree number); everyone else
/// is added as a new person. Nothing is matched by guesswork here: the caller decides.
/// A matched person only gets what they were missing; where the incoming record disagrees, the tree's
/// value is kept and the other value is written into the person's notes so nothing is lost.
/// Returns (people added, people matched).
pub fn merge_into_pairs(dst: &mut Db, inc: Db, pairs: &HashMap<u32, u32>) -> (usize, usize) {
    let mut inc = inc;
    let (mut added, mut matched) = (0usize, 0usize);
    // People
    let mut pmap: HashMap<u32, u32> = HashMap::new();
    let mut is_match: HashSet<u32> = HashSet::new();
    let mut used: HashSet<u32> = HashSet::new();
    for id in inc.people.keys() {
        match pairs.get(id).filter(|k| dst.people.contains_key(k) && !used.contains(k)) {
            Some(k) => {
                used.insert(*k);
                pmap.insert(*id, *k);
                is_match.insert(*id);
                matched += 1;
            }
            None => {
                pmap.insert(*id, dst.alloc());
                added += 1;
            }
        }
    }
    // Families
    let mut fmap: HashMap<u32, u32> = HashMap::new();
    let mut fam_match: HashSet<u32> = HashSet::new();
    for (fid, f) in &inc.fams {
        let h = f.h.and_then(|x| pmap.get(&x).copied());
        let w = f.w.and_then(|x| pmap.get(&x).copied());
        // The same couple, or one that only lacked a partner in one of the two trees.
        let compat = |a: Option<u32>, b: Option<u32>| a.is_none() || b.is_none() || a == b;
        let ex = if h.is_none() && w.is_none() {
            None
        } else {
            dst.fams
                .iter()
                .find(|(k, e)| compat(e.h, h) && compat(e.w, w) && ((h.is_some() && e.h == h) || (w.is_some() && e.w == w)) && !fmap.values().any(|v| v == *k))
                .map(|(k, _)| *k)
        };
        match ex {
            Some(k) => {
                fmap.insert(*fid, k);
                fam_match.insert(*fid);
            }
            None => {
                fmap.insert(*fid, dst.alloc());
            }
        }
    }
    // Sources and notes
    let mut smap: HashMap<u32, u32> = HashMap::new();
    let mut new_src: HashSet<u32> = HashSet::new();
    for (id, s) in &inc.sources {
        let found = dst.sources.iter().find(|(_, e)| e.title == s.title && e.author == s.author && e.publ == s.publ && e.abbr == s.abbr).map(|(k, _)| *k);
        match found {
            Some(k) => {
                smap.insert(*id, k);
            }
            None => {
                smap.insert(*id, dst.alloc());
                new_src.insert(*id);
            }
        }
    }
    let mut rmap: HashMap<u32, u32> = HashMap::new();
    let mut new_repo: HashSet<u32> = HashSet::new();
    for (id, r) in &inc.repos {
        let found = dst.repos.iter().find(|(_, e)| e.name.to_lowercase() == r.name.to_lowercase() && e.addr == r.addr).map(|(k, _)| *k);
        match found {
            Some(k) => {
                rmap.insert(*id, k);
            }
            None => {
                rmap.insert(*id, dst.alloc());
                new_repo.insert(*id);
            }
        }
    }
    let mut nmap: HashMap<u32, u32> = HashMap::new();
    let mut new_note: HashSet<u32> = HashSet::new();
    for (id, n) in &inc.notes {
        let found = dst.notes.iter().find(|(_, e)| e.text == n.text && e.extra.is_empty() && n.extra.is_empty()).map(|(k, _)| *k);
        match found {
            Some(k) => {
                nmap.insert(*id, k);
            }
            None => {
                nmap.insert(*id, dst.alloc());
                new_note.insert(*id);
            }
        }
    }
    // Whole records
    let mut xm: HashMap<String, String> = HashMap::new();
    for (o, n) in &pmap {
        xm.insert(format!("I{}", o), format!("I{}", n));
    }
    for (o, n) in &fmap {
        xm.insert(format!("F{}", o), format!("F{}", n));
    }
    for (o, n) in &smap {
        xm.insert(format!("S{}", o), format!("S{}", n));
    }
    for (o, n) in &nmap {
        xm.insert(format!("N{}", o), format!("N{}", n));
    }
    for (o, n) in &rmap {
        xm.insert(format!("R{}", o), format!("R{}", n));
    }
    let has_subm = dst.records.iter().any(|r| r.contains(" SUBM"));
    let mut new_records: Vec<String> = Vec::new();
    for r in std::mem::take(&mut inc.records) {
        let first = r.lines().next().unwrap_or("").to_string();
        let mut it = first.split_whitespace();
        let _ = it.next();
        let (x, tag) = match (it.next(), it.next()) {
            (Some(a), Some(b)) if a.starts_with('@') => (a.trim_matches('@').to_string(), b.to_string()),
            _ => (String::new(), String::new()),
        };
        if tag == "SUBM" && has_subm {
            continue;
        }
        if x.is_empty() {
            new_records.push(r);
            continue;
        }
        let nx = format!("X{}", dst.alloc());
        xm.insert(x.clone(), nx.clone());
        new_records.push(r.replacen(&format!("@{}@", x), &format!("@{}@", nx), 1));
    }
    inc.records = new_records;
    walk(&mut inc, &mut Remap { xm: &xm, nmap: &nmap, smap: &smap, rmap: &rmap });

    for (id, s) in std::mem::take(&mut inc.sources) {
        if new_src.contains(&id) {
            dst.sources.insert(smap[&id], s);
        }
    }
    for (id, r) in std::mem::take(&mut inc.repos) {
        if new_repo.contains(&id) {
            dst.repos.insert(rmap[&id], r);
        }
    }
    for (id, n) in std::mem::take(&mut inc.notes) {
        if new_note.contains(&id) {
            dst.notes.insert(nmap[&id], n);
        }
    }
    dst.records.extend(std::mem::take(&mut inc.records));

    for (id, mut p) in std::mem::take(&mut inc.people) {
        let k = pmap[&id];
        for a in p.assoc.iter_mut() {
            a.other = pmap.get(&a.other).copied().unwrap_or(0);
        }
        p.assoc.retain(|a| a.other != 0 && a.other != k);
        p.pref_parents = p.pref_parents.and_then(|f| fmap.get(&f).copied());
        p.fam_order = p.fam_order.iter().filter_map(|f| fmap.get(f).copied()).collect();
        for m in p.media.iter_mut() {
            for f in m.faces.iter_mut() {
                f.who = pmap.get(&f.who).copied().unwrap_or(0);
            }
            m.faces.retain(|f| f.who != 0);
        }
        if let Some(fp) = p.face_portrait.take() {
            p.face_portrait = pmap.get(&fp.owner).map(|o| FacePortrait { owner: *o, ..fp });
        }
        if !is_match.contains(&id) {
            dst.people.insert(k, p);
            continue;
        }
        let q = dst.people.get_mut(&k).unwrap();
        let diffs = differing(q, &p);
        if !diffs.is_empty() {
            push_note(&mut q.notes, &format!("A merged record of this person disagreed on: {}. (The values above were kept.)", diffs.join("; ")));
        }
        fill(&mut q.given, &p.given);
        fill(&mut q.surname, &p.surname);
        fill(&mut q.maiden, &p.maiden);
        fill(&mut q.prefix, &p.prefix);
        fill(&mut q.suffix, &p.suffix);
        fill(&mut q.nick, &p.nick);
        fill(&mut q.sex, &p.sex);
        for (a, b) in [(&mut q.bd, &p.bd), (&mut q.bp, &p.bp), (&mut q.dd, &p.dd), (&mut q.dp, &p.dp), (&mut q.chd, &p.chd), (&mut q.chp, &p.chp), (&mut q.bud, &p.bud), (&mut q.bup, &p.bup)] {
            fill(a, b);
        }
        fill(&mut q.chtag, &p.chtag);
        for (key, e) in &p.fx {
            match q.fx.get_mut(key) {
                Some(mine) => merge_event_details(mine, e),
                None => {
                    q.fx.insert(key.clone(), e.clone());
                }
            }
        }
        for (key, v) in &p.fvals {
            q.fvals.entry(key.clone()).or_insert_with(|| v.clone());
        }
        add_unique(&mut q.assoc, &p.assoc, |a, b| a.other == b.other && a.role == b.role);
        if q.pref_parents.is_none() {
            q.pref_parents = p.pref_parents;
        }
        if q.fam_order.is_empty() {
            q.fam_order = p.fam_order.clone();
        }
        if q.contact.is_empty() {
            q.contact = p.contact.clone();
        }
        add_unique(&mut q.alt_names, &p.alt_names, |a, b| a.kind == b.kind && a.given == b.given && a.surname == b.surname);
        add_unique(&mut q.events, &p.events, same_event);
        add_unique(&mut q.sources, &p.sources, same_cite);
        add_unique(&mut q.media, &p.media, |a, b| (!a.file.is_empty() && a.file == b.file) || (a.file.is_empty() && b.file.is_empty() && a.path == b.path && a.name == b.name));
        add_unique(&mut q.note_refs, &p.note_refs, |a, b| a == b);
        add_unique(&mut q.extra, &p.extra, |a, b| a.at == b.at && a.text == b.text);
        if !p.notes.is_empty() && !q.notes.contains(&p.notes) {
            push_note(&mut q.notes, &p.notes);
        }
        q.name = compose_name(&q.given, &q.surname);
    }
    for (fid, f) in std::mem::take(&mut inc.fams) {
        let m = |x: Option<u32>| x.and_then(|i| pmap.get(&i).copied());
        let frel: BTreeMap<u32, String> = f.frel.iter().filter_map(|(c, v)| pmap.get(c).map(|n| (*n, v.clone()))).collect();
        let mrel: BTreeMap<u32, String> = f.mrel.iter().filter_map(|(c, v)| pmap.get(c).map(|n| (*n, v.clone()))).collect();
        let kids: Vec<u32> = f.kids.iter().filter_map(|k| pmap.get(k).copied()).collect();
        let more: Vec<(String, u32)> = f.more.iter().filter_map(|(r, i)| pmap.get(i).map(|n| (r.clone(), *n))).collect();
        let pedi: BTreeMap<u32, String> = f.pedi.iter().filter_map(|(c, v)| pmap.get(c).map(|n| (*n, v.clone()))).collect();
        let k = fmap[&fid];
        if !fam_match.contains(&fid) {
            dst.fams.insert(k, Family { h: m(f.h), w: m(f.w), kids, more, pedi, frel, mrel, ..f });
            continue;
        }
        let e = dst.fams.get_mut(&k).unwrap();
        absorb_family(e, Family { h: m(f.h), w: m(f.w), kids, more, pedi, frel, mrel, ..f });
    }
    dst.sample = false;
    (added, matched)
}

/* ------------------------------------------------------------------ */
/* Checking that nothing was lost                                      */
/* ------------------------------------------------------------------ */

fn norm_date_cmp(v: &str) -> String {
    let mut s = v.trim().to_uppercase().replace(['(', ')'], " ").replace('.', "");
    for (i, m) in MON_FULL.iter().enumerate() {
        s = s
            .split_whitespace()
            .map(|t| if t == *m || (t.len() >= 4 && m.starts_with(t) && t != "MAY") { MON[i].to_string() } else { t.to_string() })
            .collect::<Vec<_>>()
            .join(" ");
    }
    let s = s
        .split_whitespace()
        .map(|t| match t {
            "CIRCA" | "CA" | "C" | "ABOUT" => "ABT",
            "BEFORE" => "BEF",
            "AFTER" => "AFT",
            "BETWEEN" => "BET",
            "ESTIMATED" => "EST",
            "CALCULATED" => "CAL",
            "INTERPRETED" => "INT",
            x => x,
        })
        .collect::<Vec<_>>()
        .join(" ");
    s
}

#[derive(Default)]
struct Flatten {
    exact: HashMap<(String, String), usize>,
    words: HashMap<(String, String), usize>,
    names: Vec<(String, Vec<String>)>,
}

fn flatten(n: &Node, path: &str, parent_val: &str, f: &mut Flatten) {
    let mut v = collapse(&n.val);
    let tag = n.tag.as_str();
    if is_ptr(&n.val) {
        v = "@".to_string();
    }
    let here = if path.is_empty() { tag.to_string() } else { format!("{}.{}", path, tag) };
    match tag {
        "NOTE" => {
            for w in v.split_whitespace() {
                *f.words.entry((here.clone(), w.to_lowercase())).or_insert(0) += 1;
            }
        }
        "NAME" if n.xref.is_none() && !path.is_empty() => {
            let words: Vec<String> = v.replace('/', " ").split_whitespace().map(|w| w.to_lowercase()).collect();
            if !words.is_empty() {
                f.names.push((here.clone(), words));
            }
        }
        "GIVN" | "SURN" | "SPFX" | "UNKNOWN" if n.kids.is_empty() && contains_ci(parent_val, &n.val) => {}
        _ => {
            if !(n.val.is_empty() && !n.kids.is_empty()) && !(n.val.is_empty() && n.kids.is_empty()) {
                let val = match tag {
                    "DATE" => norm_date_cmp(&ged_date(&v)),
                    "SEX" => v.chars().next().map(|c| c.to_ascii_uppercase().to_string()).unwrap_or_default(),
                    "FILE" => v.replace('\\', "/").rsplit('/').next().unwrap_or("").to_lowercase(),
                    "TITL" | "FORM" | "PAGE" | "TEXT" | "AUTH" | "ABBR" | "PUBL" | "PLAC" | "TYPE" | "CAUS" | "AGNC" | "AGE" | "TEMP" | "STAT" | "QUAY" => v.clone(),
                    _ => v.clone(),
                };
                *f.exact.entry((here.clone(), val)).or_insert(0) += 1;
            }
        }
    }
    let pv = collapse(&n.val);
    for k in &n.kids {
        flatten(k, &here, &pv, f);
    }
}


/// Bring the two spellings of a media object (5.5 and 5.5.1) and pointers-versus-inline
/// to one shape, so that the comparison is about content only.
fn norm_obje(n: &mut Node) {
    if n.tag == "OBJE" {
        let (mut form, mut titl): (Option<Node>, Option<Node>) = (None, None);
        let mut kids: Vec<Node> = Vec::new();
        for k in std::mem::take(&mut n.kids) {
            match k.tag.as_str() {
                "FORM" if form.is_none() => form = Some(k),
                "TITL" if titl.is_none() => titl = Some(k),
                _ => kids.push(k),
            }
        }
        if let Some(file) = kids.iter_mut().find(|k| k.tag == "FILE") {
            if let Some(f) = form {
                if !file.kids.iter().any(|k| k.tag == "FORM") {
                    file.kids.push(f);
                }
            }
            if let Some(t) = titl {
                if !file.kids.iter().any(|k| k.tag == "TITL") {
                    file.kids.push(t);
                }
            }
        } else {
            kids.extend(form);
            kids.extend(titl);
        }
        n.kids = kids;
    }
}

fn inline_objs(n: &mut Node, objs: &HashMap<String, Node>) {
    if n.tag == "OBJE" && is_ptr(&n.val) {
        if let Some(o) = objs.get(n.val.trim_matches('@')) {
            n.val.clear();
            n.kids = o.kids.clone();
        }
    }
    norm_obje(n);
    for k in n.kids.iter_mut() {
        inline_objs(k, objs);
    }
}

fn prep(roots: Vec<Node>) -> Vec<Node> {
    let mut objs: HashMap<String, Node> = HashMap::new();
    for r in &roots {
        if r.tag == "OBJE" {
            if let Some(x) = &r.xref {
                objs.insert(x.clone(), r.clone());
            }
        }
    }
    fn refs(n: &Node, out: &mut HashSet<String>) {
        if n.tag == "OBJE" && is_ptr(&n.val) {
            out.insert(n.val.trim_matches('@').to_string());
        }
        for k in &n.kids {
            refs(k, out);
        }
    }
    let mut used: HashSet<String> = HashSet::new();
    for r in &roots {
        refs(r, &mut used);
    }
    let mut out = Vec::new();
    for mut r in roots {
        if r.tag == "OBJE" && r.xref.as_ref().map_or(false, |x| used.contains(x)) {
            continue;
        }
        inline_objs(&mut r, &objs);
        out.push(r);
    }
    out
}

fn flatten_all(roots: &[Node]) -> Flatten {
    let mut f = Flatten::default();
    for r in roots {
        if r.tag == "HEAD" || r.tag == "TRLR" {
            continue;
        }
        // The record's own name is not data. A note record's text is.
        let mut top = r.clone();
        top.xref = None;
        let only_children = !matches!(r.tag.as_str(), "NOTE");
        if only_children {
            top.val.clear();
        }
        // keep the record type in the path
        let kids = std::mem::take(&mut top.kids);
        let mut head = top;
        head.kids = Vec::new();
        if !head.val.is_empty() {
            flatten(&head, "", "", &mut f);
        }
        for k in &kids {
            flatten(k, &r.tag, "", &mut f);
        }
    }
    f
}

/// Compare an original GEDCOM file with an export of what was read from it.
/// Returns a line for each piece of the original that is missing from the export.
pub fn loss_check(original: &str, exported: &str) -> Vec<String> {
    let mut w = Vec::new();
    let a = flatten_all(&prep(build_tree(original, &mut w)));
    let b = flatten_all(&prep(build_tree(exported, &mut w)));
    let mut lost: Vec<String> = Vec::new();
    let mut keys: Vec<_> = a.exact.iter().collect();
    keys.sort();
    for ((path, val), n) in keys {
        let have = b.exact.get(&(path.clone(), val.clone())).copied().unwrap_or(0);
        if have < *n {
            let miss = n - have;
            let shown = if val.len() > 90 { format!("{}...", val.chars().take(90).collect::<String>()) } else { val.clone() };
            lost.push(if miss > 1 { format!("{} = {}  (x{})", path, shown, miss) } else { format!("{} = {}", path, shown) });
        }
    }
    let mut wk: Vec<_> = a.words.iter().collect();
    wk.sort();
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for ((path, word), n) in wk {
        let have = b.words.get(&(path.clone(), word.clone())).copied().unwrap_or(0);
        if have < *n {
            by_path.entry(path.clone()).or_default().push(word.clone());
        }
    }
    for (path, words) in by_path {
        let shown: Vec<String> = words.into_iter().take(12).collect();
        lost.push(format!("{} (words missing: {})", path, shown.join(" ")));
    }
    // Names: each original name must be found inside some exported name.
    let mut index: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, (bp, bw)) in b.names.iter().enumerate() {
        for w in bw {
            index.entry((bp.as_str(), w.as_str())).or_default().push(i);
        }
    }
    let mut used = vec![false; b.names.len()];
    for (path, words) in &a.names {
        let empty: Vec<usize> = Vec::new();
        let cands = words.iter().map(|w| index.get(&(path.as_str(), w.as_str())).unwrap_or(&empty)).min_by_key(|v| v.len()).unwrap_or(&empty);
        let mut found = false;
        for &i in cands {
            if used[i] {
                continue;
            }
            let mut pool = b.names[i].1.clone();
            let ok = words.iter().all(|w| match pool.iter().position(|x| x == w) {
                Some(j) => {
                    pool.remove(j);
                    true
                }
                None => false,
            });
            if ok {
                used[i] = true;
                found = true;
                break;
            }
        }
        if !found {
            lost.push(format!("{} = {}", path, words.join(" ")));
        }
    }
    lost
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Everything about each person that does not depend on their number: name, parents,
    /// partners, children, associations, face tags, and the whole exported record with the
    /// numbers taken out.
    fn shape(db: &Db) -> Vec<String> {
        let name = |x: &u32| db.people.get(x).map(|p| p.name.clone()).unwrap_or_else(|| format!("?{}", x));
        let mut v: Vec<String> = db
            .people
            .iter()
            .map(|(id, p)| {
                let mut par: Vec<String> = db.fams.values().filter(|f| f.kids.contains(id)).map(|f| format!("{}+{}", f.h.map(|x| name(&x)).unwrap_or_default(), f.w.map(|x| name(&x)).unwrap_or_default())).collect();
                par.sort();
                let mut kids: Vec<String> = db.fams.values().filter(|f| f.h == Some(*id) || f.w == Some(*id)).flat_map(|f| f.kids.iter().map(name)).collect();
                kids.sort();
                let assoc: Vec<String> = p.assoc.iter().map(|a| format!("{}:{}", name(&a.other), a.role)).collect();
                let faces: Vec<String> = p.media.iter().flat_map(|m| m.faces.iter().map(|f| name(&f.who))).collect();
                let pref = p.pref_parents.and_then(|f| db.fams.get(&f)).map(|f| f.h.map(|x| name(&x)).unwrap_or_default());
                format!("{} | {:?} | {:?} | {:?} | {:?} | {:?} | {} src", p.name, par, kids, assoc, faces, pref, p.sources.len())
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn renumbering_keeps_every_link() {
        let mut db = crate::sample::sample_family();
        let before = shape(&db);
        let home_name = db.people[&db.home.unwrap()].name.clone();
        let bookmarked: Vec<String> = db.bookmarks.iter().map(|b| db.people[b].name.clone()).collect();
        let journal: Vec<Vec<String>> = db.journal.iter().map(|j| j.people.iter().map(|x| db.people[x].name.clone()).collect()).collect();
        for how in [RenumberOrder::Family, RenumberOrder::Alphabetical, RenumberOrder::Present] {
            let order = renumber_order(&db, how);
            assert_eq!(order.len(), db.people.len());
            renumber(&mut db, &order);
            assert_eq!(shape(&db), before, "{:?}", how);
            let ids: Vec<u32> = db.people.keys().copied().collect();
            assert_eq!(ids, (1..=ids.len() as u32).collect::<Vec<_>>(), "people are numbered 1, 2, 3...");
            let fids: Vec<u32> = db.fams.keys().copied().collect();
            assert_eq!(fids, (1..=fids.len() as u32).collect::<Vec<_>>());
            assert_eq!(db.people[&db.home.unwrap()].name, home_name);
            assert_eq!(db.bookmarks.iter().map(|b| db.people[b].name.clone()).collect::<Vec<_>>(), bookmarked);
            assert_eq!(db.journal.iter().map(|j| j.people.iter().map(|x| db.people[x].name.clone()).collect::<Vec<_>>()).collect::<Vec<_>>(), journal);
            if how == RenumberOrder::Family {
                assert_eq!(db.people[&1].name, home_name, "the home person becomes number 1");
            }
            // the exported file still reads back to the same tree, with no broken links
            // (face tags are Gilded Roots' own and are not part of a GEDCOM file)
            let no_faces = |d: &Db| {
                let mut d = d.clone();
                for p in d.people.values_mut() {
                    for m in p.media.iter_mut() {
                        m.faces.clear();
                    }
                }
                shape(&d)
            };
            let (back, rep) = parse_ged_report(&to_ged(&db));
            assert_eq!(shape(&back), no_faces(&db));
            assert!(!rep.warnings.iter().any(|w| w.contains("no such") || w.contains("missing")), "{:?}", rep.warnings);
            // new people get numbers after everything in use
            let n = db.alloc();
            assert!(!db.people.contains_key(&n) && !db.fams.contains_key(&n) && !db.sources.contains_key(&n));
        }
        // each person carries one "Old ID", not one per renumbering
        let olds = db.people.values().map(|p| p.extra.iter().filter(|r| r.text.contains("TYPE Old ID")).count()).max().unwrap();
        assert!(olds <= 1);
    }

    #[test]
    fn renumbering_updates_links_kept_as_gedcom_text() {
        let t = "0 HEAD\n0 @I50@ INDI\n1 NAME Kid /A/\n1 FAMC @F70@\n2 _FOO bar\n1 ASSO @I60@\n2 RELA friend\n1 _CUSTOM @I60@\n0 @I60@ INDI\n1 NAME Dad /A/\n1 FAMS @F70@\n0 @F70@ FAM\n1 HUSB @I60@\n1 CHIL @I50@\n0 @X1@ _PLAN\n1 _WHO @I60@\n0 TRLR\n";
        let (mut db, _) = parse_ged_report(t);
        let order = renumber_order(&db, RenumberOrder::Alphabetical);
        renumber(&mut db, &order);
        let out = to_ged(&db);
        let (back, rep) = parse_ged_report(&out);
        assert_eq!(back.people.len(), 2);
        let dad = back.people.iter().find(|(_, p)| p.name == "Dad A").map(|(k, _)| *k).unwrap();
        let kid = back.people.iter().find(|(_, p)| p.name == "Kid A").map(|(k, _)| *k).unwrap();
        assert!(back.fams.values().any(|f| f.h == Some(dad) && f.kids.contains(&kid)));
        assert!(out.contains("_CUSTOM @I1@") || out.contains("_CUSTOM @I"), "{}", out);
        // every pointer in the file goes somewhere
        for line in out.lines() {
            if let Some(i) = line.find(" @") {
                let x = line[i + 2..].trim_end_matches('@');
                if line.starts_with("0 ") { continue; }
                assert!(out.contains(&format!("0 @{}@ ", x)), "dangling {} in {}", x, line);
            }
        }
        assert!(rep.warnings.is_empty(), "{:?}", rep.warnings);
    }


    #[test]
    fn home_person_comes_from_the_file_and_survives_export() {
        // like an Ancestry file: the home person is first, and the IDs are big numbers
        let t = "0 HEAD\n1 SOUR Ancestry.com Family Trees\n0 @I262317410388@ INDI\n1 NAME Doyle /McIntosh/\n1 FAMC @F9@\n\
                 0 @I331709105@ INDI\n1 NAME John /Cooper/\n0 @I_1005789154@ INDI\n1 NAME Gene /McIntosh/\n1 FAMS @F9@\n\
                 0 @F9@ FAM\n1 HUSB @I_1005789154@\n1 CHIL @I262317410388@\n0 TRLR\n";
        let (mut db, _) = parse_ged_report(t);
        let home = db.home.expect("a home person");
        assert_eq!(db.people[&home].name, "Doyle McIntosh");
        // exported: home person first and named in the header
        let out = to_ged(&db);
        let first = out.lines().find(|l| l.ends_with(" INDI")).unwrap().to_string();
        assert_eq!(first, format!("0 @I{}@ INDI", home));
        assert!(out.contains(&format!("1 _HOME @I{}@", home)));
        let (back, _) = parse_ged_report(&out);
        assert_eq!(back.people[&back.home.unwrap()].name, "Doyle McIntosh");
        // a home person chosen later is kept too, even though they are not first in the tree
        let john = *db.people.iter().find(|(_, p)| p.name == "John Cooper").unwrap().0;
        db.home = Some(john);
        let (again, _) = parse_ged_report(&to_ged(&db));
        assert_eq!(again.people[&again.home.unwrap()].name, "John Cooper");
    }

    #[test]
    fn a_file_without_people_has_no_home() {
        let (db, _) = parse_ged_report("0 HEAD\n0 @S1@ SOUR\n1 TITL x\n0 TRLR\n");
        assert!(db.home.is_none());
        let (db, _) = parse_ged_report("0 HEAD\n1 _HOME @I77@\n0 @I1@ INDI\n1 NAME A /B/\n0 TRLR\n");
        assert_eq!(db.people[&db.home.unwrap()].name, "A B", "a _HOME that points nowhere falls back to the first person");
    }


    fn rt(text: &str) -> (Db, ImportReport, String) {
        let (db, mut rep) = parse_ged_report(text);
        let out = to_ged(&db);
        rep.lost = loss_check(text, &out);
        (db, rep, out)
    }

    fn assert_nothing_lost(text: &str) -> (Db, String) {
        let (db, rep, out) = rt(text);
        assert!(rep.lost.is_empty(), "lost: {:#?}\n---\n{}", rep.lost, out);
        // and the export itself reads back to the same thing (second generation is stable)
        let (db2, rep2) = parse_ged_report(&out);
        let out2 = to_ged(&db2);
        assert!(loss_check(&out, &out2).is_empty(), "second generation lost: {:#?}", loss_check(&out, &out2));
        assert_eq!(db.people.len(), db2.people.len());
        assert_eq!(db.fams.len(), db2.fams.len());
        assert_eq!(rep.sources, rep2.sources);
        (db, out)
    }

    #[test]
    fn dates_round_trip() {
        for d in [
            "ABT 1850", "BEF 1900", "AFT 1 JAN 1900", "BET 1850 AND 1860", "FROM 1900 TO 1910", "FROM 1900", "TO 1910", "EST 1850", "CAL 1850",
            "INT 1850 (about the time of the war)", "(Spring of 1850)", "1850/51", "12 MAR 1850", "MAR 1850", "1850", "500 BC", "@#DJULIAN@ 12 JAN 1700",
            "BET 12 JAN 1850 AND 3 FEB 1851",
        ] {
            let nice = nice_date(d);
            let back = ged_date(&nice);
            assert_eq!(norm_date_cmp(&back), norm_date_cmp(d), "{} -> {} -> {}", d, nice, back);
        }
        assert_eq!(nice_date("ABT 1850"), "about 1850");
        assert_eq!(ged_date("about 1850"), "ABT 1850");
        assert_eq!(ged_date("March 12, 1850"), "12 MAR 1850");
        assert_eq!(ged_date("1850-03-12"), "12 MAR 1850");
        assert_eq!(ged_date("between 1850 and 1860"), "BET 1850 AND 1860");
        assert_eq!(ged_date("sometime in the spring"), "(sometime in the spring)");
        assert_eq!(ged_date("circa 1850"), "ABT 1850");
    }

    #[test]
    fn line_endings_cr_crlf_lf_all_read() {
        let base = "0 HEAD\n0 @I1@ INDI\n1 NAME A /B/\n1 BIRT\n2 DATE 1 JAN 1900\n0 TRLR\n";
        for t in [base.to_string(), base.replace('\n', "\r\n"), base.replace('\n', "\r")] {
            let db = parse_ged(&t);
            assert_eq!(db.people.len(), 1);
            assert_eq!(db.people.values().next().unwrap().bd, "1 JAN 1900");
        }
    }

    #[test]
    fn long_text_and_line_breaks_survive() {
        let long = "word ".repeat(200);
        let t = format!("0 HEAD\n0 @I1@ INDI\n1 NAME A /B/\n1 NOTE first line\n2 CONT second line\n2 CONC  and more\n1 OCCU {}\n0 TRLR\n", long.trim());
        let (db, out) = assert_nothing_lost(&t);
        let p = db.people.values().next().unwrap();
        assert_eq!(p.notes, "first line\nsecond line and more");
        assert_eq!(p.events[0].val, long.trim());
        assert!(out.lines().all(|l| l.len() < 255));
    }

    #[test]
    fn leading_at_sign_is_escaped() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME A /B/\n1 NOTE @@home is where the heart is\n0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        assert_eq!(db.people.values().next().unwrap().notes, "@home is where the heart is");
        assert!(out.contains("1 NOTE @@home"));
    }

    #[test]
    fn source_records_keep_page_data_quality_and_are_shared() {
        let t = "0 HEAD\n\
0 @S1@ SOUR\n1 TITL 1940 US Census\n1 AUTH US Government\n1 PUBL Washington\n1 ABBR Census40\n1 TEXT Whole census\n1 REPO @R1@\n2 CALN T627\n1 NOTE About this source\n\
0 @R1@ REPO\n1 NAME National Archives\n\
0 @I1@ INDI\n1 NAME John /Smith/\n1 BIRT\n2 DATE ABT 1900\n2 SOUR @S1@\n3 PAGE Sheet 12, line 4\n3 QUAY 2\n3 DATA\n4 DATE 1 APR 1940\n4 TEXT John Smith, age 40\n3 NOTE Found by cousin\n1 DEAT\n2 SOUR @S1@\n3 PAGE Sheet 99\n1 SOUR Family Bible\n2 PAGE inside cover\n\
0 @I2@ INDI\n1 NAME Mary /Smith/\n1 SOUR @S1@\n2 PAGE Sheet 12, line 5\n\
0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        assert_eq!(db.sources.len(), 1);
        let s = db.sources.values().next().unwrap();
        assert_eq!(s.title, "1940 US Census");
        assert_eq!(s.author, "US Government");
        assert_eq!(s.abbr, "Census40");
        let john = db.people.values().find(|p| p.given == "John").unwrap();
        let birth = john.sources.iter().find(|c| c.about == "Birth").unwrap();
        assert_eq!(birth.page, "Sheet 12, line 4");
        assert_eq!(birth.quality, "2");
        assert_eq!(birth.data, "John Smith, age 40");
        assert_eq!(birth.ddate, "1 APR 1940");
        assert_eq!(birth.note, "Found by cousin");
        assert!(birth.sid.is_some());
        let death = john.sources.iter().find(|c| c.about == "Death").unwrap();
        assert_eq!(death.page, "Sheet 99");
        assert_eq!(death.sid, birth.sid);
        assert!(john.sources.iter().any(|c| c.sid.is_none() && c.text == "Family Bible" && c.page == "inside cover"));
        assert_eq!(out.lines().filter(|l| l.starts_with("0 @") && l.ends_with(" SOUR")).count(), 1, "one shared source record");
        assert!(out.contains("0 @R"), "repository record kept");
    }

    #[test]
    fn linked_notes_are_shared_and_kept() {
        let t = "0 HEAD\n0 @N1@ NOTE The family moved west\n1 CONC  in 1850.\n1 CONT Second paragraph.\n\
0 @I1@ INDI\n1 NAME A /B/\n1 NOTE @N1@\n1 NOTE own note\n\
0 @I2@ INDI\n1 NAME C /B/\n1 NOTE @N1@\n1 RESI\n2 NOTE @N1@\n2 DATE 1850\n\
0 @F1@ FAM\n1 HUSB @I1@\n1 NOTE @N1@\n0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        assert_eq!(db.notes.len(), 1);
        let n = db.notes.values().next().unwrap();
        assert_eq!(n.text, "The family moved west in 1850.\nSecond paragraph.");
        assert_eq!(db.people.values().filter(|p| !p.note_refs.is_empty()).count(), 2);
        assert_eq!(out.matches("0 @N").count(), 1);
    }

    #[test]
    fn alternate_names_kinds_prefixes_and_suffixes() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME Dr. Mary Ann /Jones/ Jr.\n2 NPFX Dr.\n2 GIVN Mary Ann\n2 SURN Jones\n2 NSFX Jr.\n2 NICK Annie\n\
1 NAME Mary Ann /Smith/\n2 TYPE married\n2 SOUR Marriage license\n1 NAME Mary /Brown/\n2 TYPE birth\n1 NAME Maria /Jones/\n2 TYPE aka\n2 NOTE Spanish form\n1 NAME Sister Agnes //\n2 TYPE religious\n0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        let p = db.people.values().next().unwrap();
        assert_eq!(p.given, "Dr. Mary Ann".replace("Dr. ", ""));
        assert_eq!(p.surname, "Jones");
        assert_eq!(p.prefix, "Dr.");
        assert_eq!(p.suffix, "Jr.");
        assert_eq!(p.nick, "Annie");
        // birth name with a different given name stays an alternate name
        assert_eq!(p.alt_names.len(), 4, "{:?}", p.alt_names.iter().map(|a| (&a.kind, &a.given, &a.surname)).collect::<Vec<_>>());
        assert!(p.alt_names.iter().any(|a| a.kind == "married" && a.surname == "Smith" && a.cites.len() == 1));
        assert!(p.alt_names.iter().any(|a| a.kind == "aka" && a.note == "Spanish form"));
        assert!(out.contains("2 TYPE religious"));
    }

    #[test]
    fn birth_name_with_same_given_name_becomes_maiden_name() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME Mary /Smith/\n1 NAME Mary /Brown/\n2 TYPE birth\n0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        let p = db.people.values().next().unwrap();
        assert_eq!(p.maiden, "Brown");
        assert!(p.alt_names.is_empty());
        assert!(out.contains("2 TYPE birth"));
    }

    #[test]
    fn many_event_types_and_details_are_kept() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME A /B/\n1 SEX U\n\
1 BIRT\n2 DATE ABT 1850\n2 PLAC Springfield, Illinois\n3 MAP\n4 LATI N39.8\n4 LONG W89.6\n2 CAUS none\n2 AGE 0y\n2 ADDR 1 Main St\n3 CITY Springfield\n2 NOTE born at home\n\
1 DEAT Y\n1 BAPM\n2 DATE 1851\n1 CHR\n2 DATE 1852\n\
1 ADOP\n2 DATE 1860\n2 FAMC @F1@\n3 ADOP BOTH\n1 EDUC Harvard\n2 DATE 1870\n2 PLAC Cambridge\n1 OCCU Farmer\n2 AGNC Self\n1 RELI Lutheran\n1 RESI\n2 DATE 1880\n2 ADDR 5 Elm\n1 CENS\n2 DATE 1900\n1 IMMI\n2 DATE 1849\n2 TYPE ship\n1 NATU\n2 DATE 1855\n1 PROB\n2 DATE 1910\n1 WILL\n1 EVEN Captain in the guard\n2 TYPE Military service\n2 DATE 1861\n1 EVEN\n2 TYPE Custom thing\n2 DATE 1862\n1 _MILT Some military\n2 DATE 1863\n1 FACT blue eyes\n2 TYPE Eyes\n1 BAPL\n2 DATE 5 MAY 1990\n2 TEMP SLAKE\n2 STAT COMPLETED\n1 SLGC\n2 FAMC @F1@\n2 DATE 1 JAN 1991\n2 TEMP LOGAN\n1 ENDL\n2 TEMP PROVO\n2 STAT STILLBORN\n1 SSN 123-45-6789\n1 _UID 12345-ABCDE\n1 RIN 77\n1 REFN abc\n2 TYPE mine\n1 AFN 1234-56\n1 RESN privacy\n1 CHAN\n2 DATE 1 JAN 2020\n3 TIME 12:00\n1 ASSO @I2@\n2 RELA godfather\n\
0 @I2@ INDI\n1 NAME G /H/\n0 @F1@ FAM\n1 HUSB @I2@\n1 CHIL @I1@\n0 TRLR\n";
        let (db, _out) = assert_nothing_lost(t);
        let p = db.people.values().find(|p| p.given == "A").unwrap();
        assert_eq!(p.fvals.get("SEX").map(|s| s.as_str()), Some("U"));
        assert_eq!(p.bp, "Springfield, Illinois");
        assert_eq!(p.fx["BIRT"].cause, "none");
        assert_eq!(p.fx["BIRT"].note, "born at home");
        assert_eq!(p.chd, "1851");
        assert_eq!(p.chtag, "BAPM");
        assert!(p.events.iter().any(|e| e.kind == "Christening" && e.date == "1852"));
        let occ = p.events.iter().find(|e| e.kind == "Occupation").unwrap();
        assert_eq!(occ.val, "Farmer");
        assert_eq!(occ.agency, "Self");
        let mil = p.events.iter().find(|e| e.kind == "Military service").unwrap();
        assert_eq!(mil.val, "Captain in the guard");
        let bapl = p.events.iter().find(|e| e.kind == "LDS baptism").unwrap();
        assert_eq!((bapl.temp.as_str(), bapl.stat.as_str()), ("SLAKE", "COMPLETED"));
        assert!(p.events.iter().any(|e| e.tag == "_MILT" && e.date == "1863"));
        assert!(p.events.iter().any(|e| e.kind == "Custom thing"));
    }

    #[test]
    fn complex_families_multiple_marriages_adoption_and_same_sex_couples() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME Pat /Lee/\n1 SEX M\n1 FAMS @F1@\n1 FAMS @F2@\n\
0 @I2@ INDI\n1 NAME Ann /Fox/\n1 SEX F\n1 FAMS @F1@\n\
0 @I3@ INDI\n1 NAME Bea /Roe/\n1 SEX F\n1 FAMS @F2@\n\
0 @I4@ INDI\n1 NAME Kid /Lee/\n1 FAMC @F1@\n1 FAMC @F2@\n2 PEDI adopted\n2 NOTE note on link\n2 STAT proven\n\
0 @I5@ INDI\n1 NAME Foster /Lee/\n1 FAMC @F2@\n2 PEDI foster\n\
0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 CHIL @I4@\n1 MARR\n2 DATE 1 JAN 1950\n2 PLAC Reno\n2 TYPE civil\n2 SOUR Court book\n3 PAGE 4\n2 NOTE a quiet wedding\n2 HUSB\n3 AGE 22y\n1 DIV\n2 DATE 1960\n1 ENGA\n2 DATE 1949\n1 MARB\n1 SLGS\n2 TEMP SLAKE\n2 DATE 1 JAN 1960\n2 STAT COMPLETED\n1 NCHI 1\n1 RESN locked\n\
0 @F2@ FAM\n1 HUSB @I1@\n1 WIFE @I3@\n1 WIFE @I2@\n1 CHIL @I4@\n2 _FREL Adopted\n1 CHIL @I5@\n1 MARR Y\n1 EVEN\n2 TYPE Commitment\n2 DATE 1970\n1 NOTE Same-sex style family\n0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        assert_eq!(db.fams.len(), 2);
        let f2 = db.fams.values().nth(1).unwrap();
        assert_eq!(f2.more.len(), 1, "second wife kept");
        let kid = db.people.values().find(|p| p.given == "Kid").unwrap();
        let kid_id = db.people.iter().find(|(_, p)| p.given == "Kid").map(|(i, _)| *i).unwrap();
        assert_eq!(db.fams.values().filter(|f| f.kids.contains(&kid_id)).count(), 2, "child of two families");
        assert_eq!(db.fams.values().nth(1).unwrap().pedi.get(&kid_id).map(|s| s.as_str()), Some("adopted"));
        assert!(kid.extra.iter().any(|r| r.at.starts_with("FAMC@")));
        assert_eq!(out.matches("1 FAMC").count(), 3);
        let f1 = db.fams.values().next().unwrap();
        assert_eq!(f1.md, "1 JAN 1950");
        assert_eq!(f1.mx.typ, "civil");
        assert_eq!(f1.mx.cites.len(), 1);
        assert!(f1.events.iter().any(|e| e.kind == "Divorce"));
        assert!(f1.events.iter().any(|e| e.kind == "LDS spouse sealing"));
        // Pat is in two families
        let pat = db.people.iter().find(|(_, p)| p.given == "Pat").map(|(i, _)| *i).unwrap();
        assert_eq!(db.fams_of(pat).len(), 2);
        // Ann appears as a second wife of family 2, so her FAMS list must include it
        let ann = db.people.iter().find(|(_, p)| p.given == "Ann").map(|(i, _)| *i).unwrap();
        assert_eq!(db.fams_of(ann).len(), 2);
    }

    #[test]
    fn research_tasks_round_trip() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME A /B/\n1 _TODO Find 1880 census\n2 DATE 2026-10-29\n2 STAT OPEN\n2 NOTE Try Oregon first\n1 _TODO Order death certificate\n2 STAT DONE\n0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        let p = db.people.values().next().unwrap();
        assert_eq!(p.todos.len(), 2);
        assert!(!p.todos[0].done && p.todos[1].done);
        assert_eq!(p.todos[0].note, "Try Oregon first");
        assert!(out.contains("1 _TODO Find 1880 census"));
    }

    #[test]
    fn ancestry_media_and_odd_dates() {
        let t = "0 HEAD\n1 SOUR Ancestry.com Member Trees\n0 @I1@ INDI\n1 NAME A /B/\n1 BIRT\n2 DATE FEB 8 1891\n1 DEAT\n2 DATE 13 SEPTEMBER, 1713\n1 OBJE @M1@\n\
0 @M1@ OBJE\n1 FILE https://mediasvc.ancestry.com/v2/image/namespaces/1093/media/abc.jpg?Client=Trees\n2 FORM jpg\n3 TYPE image\n3 _MTYPE document\n3 _STYPE jpg\n3 _SIZE 1001189\n3 _WDTH 1000\n3 _HGHT 1200\n2 TITL #41 Logging Loco with Crew\n1 DATE 1920\n1 PLAC Oregon\n1 _CREA 2020-01-01\n1 _USER abc\n1 _OID x-y\n1 _ATL Y\n1 _ORIG u\n\
0 @M2@ OBJE\n1 FILE https://mediasvc.ancestry.com/v2/image/x.jpg\n1 FORM jpg\n2 TYPE image\n2 _SIZE 5\n1 TITL Second one\n1 _OID q\n\
0 @I2@ INDI\n1 NAME C /D/\n1 OBJE @M2@\n1 OBJE @M1@\n0 TRLR\n";
        let (_db, rep, out) = rt(t);
        assert!(rep.lost.is_empty(), "lost: {:#?}\n---\n{}", rep.lost, out);
    }

    #[test]
    fn media_references_are_kept_whether_or_not_the_file_exists() {
        let t = "0 HEAD\n0 @M1@ OBJE\n1 FILE photos/grandpa.jpg\n2 FORM jpeg\n3 MEDI photo\n2 TITL Grandpa in 1950\n1 _PRIM Y\n\
0 @I1@ INDI\n1 NAME A /B/\n1 OBJE @M1@\n1 OBJE\n2 FILE C:\\Docs\\will.pdf\n3 FORM pdf\n2 TITL The will\n1 OBJE\n2 FORM gif\n2 FILE old.gif\n2 TITL Old style\n\
0 @F1@ FAM\n1 HUSB @I1@\n1 OBJE\n2 FILE wedding.png\n0 TRLR\n";
        let (db, out) = assert_nothing_lost(t);
        let p = db.people.values().next().unwrap();
        assert_eq!(p.media.len(), 3);
        assert_eq!(p.media[0].path, "photos/grandpa.jpg");
        assert_eq!(p.media[0].caption, "Grandpa in 1950");
        assert_eq!(p.media[1].name, "will.pdf");
        assert_eq!(db.fams.values().next().unwrap().media.len(), 1);
        assert!(out.contains("4 MEDI photo"));
        assert!(out.contains("_PRIM Y"));
    }

    #[test]
    fn unknown_records_and_tags_are_kept_not_dropped() {
        let t = "0 HEAD\n1 SOUR Weird Program\n2 VERS 9\n0 @U1@ SUBM\n1 NAME Me\n1 ADDR 12 Road\n2 CONT Town\n\
0 @I1@ INDI\n1 NAME A /B/\n1 _FSFTID KXYZ-123\n1 _APID 1,2::3\n1 FOO bar\n2 BAZ qux\n3 DEEP down\n\
0 _CUSTOM record\n1 DATA stuff\n0 @X9@ _THING\n1 PTR @I1@\n\
0 TRLR\n";
        let (db, rep, out) = rt(t);
        assert!(rep.lost.is_empty(), "{:#?}", rep.lost);
        assert!(rep.kept_total() >= 4);
        assert!(out.contains("3 DEEP down"));
        assert!(out.contains("0 _CUSTOM record"));
        assert!(out.contains("_FSFTID KXYZ-123"));
        // pointers inside kept records still point at the person after renumbering
        let (pid, _) = db.people.iter().next().unwrap();
        assert!(out.contains(&format!("1 PTR @I{}@", pid)));
    }

    #[test]
    fn damaged_files_do_not_panic_and_report_problems() {
        for t in ["", "garbage\nmore garbage", "0 HEAD\n0 @I1@ INDI\n1 FAMC @F99@\n0 @F1@ FAM\n1 HUSB @I77@\n1 CHIL @I1@\n1 CHIL @I555@\n0 TRLR\n", "1 NAME no record\n2 CONC x\n", "0 @I1@ INDI\n1 NOTE @N1@\n1 SOUR @S1@\n"] {
            let (db, rep) = parse_ged_report(t);
            let _ = to_ged(&db);
            let _ = rep;
        }
        let (_, rep) = parse_ged_report("0 @I1@ INDI\n1 NOTE @N1@\n1 SOUR @S1@\n");
        assert!(!rep.warnings.is_empty());
    }

    #[test]
    fn child_link_from_the_child_side_only_is_repaired_and_reported() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME A /B/\n1 FAMC @F1@\n0 @F1@ FAM\n1 HUSB @I2@\n0 @I2@ INDI\n1 NAME C /B/\n0 TRLR\n";
        let (db, rep) = parse_ged_report(t);
        assert_eq!(db.fams.values().next().unwrap().kids.len(), 1);
        assert!(rep.warnings.iter().any(|w| w.contains("does not list them")));
    }

    #[test]
    fn ansel_accents_are_decoded() {
        // "José Müller" with ANSEL combining accents before the letter: e2 = acute, e8 = diaeresis
        let mut b = b"0 HEAD\r\n1 CHAR ANSEL\r\n0 @I1@ INDI\r\n1 NAME Jos".to_vec();
        b.extend_from_slice(&[0xE2, b'e']);
        b.extend_from_slice(b" M");
        b.extend_from_slice(&[0xE8, b'u']);
        b.extend_from_slice(b"ller //\r\n1 NOTE price \xB9 5, \xA5 \xC3\r\n0 TRLR\r\n");
        let (t, cs) = decode_ged(&b);
        assert_eq!(cs, "ANSEL");
        assert!(t.contains("José Müller"), "{}", t);
        assert!(t.contains("£ 5, Æ ©"), "{}", t);
    }

    #[test]
    fn windows_ansi_and_utf16_are_decoded() {
        let ansi = b"0 HEAD\n1 CHAR ANSI\n0 @I1@ INDI\n1 NAME Ren\xE9e /Fran\xE7ois/\n0 TRLR\n";
        let (t, _) = decode_ged(ansi);
        assert!(t.contains("Renée /François/"));
        let mut u16b = vec![0xFF, 0xFE];
        for c in "0 HEAD\n0 @I1@ INDI\n1 NAME Zoë /Ünal/\n0 TRLR\n".encode_utf16() {
            u16b.extend_from_slice(&c.to_le_bytes());
        }
        let (t, cs) = decode_ged(&u16b);
        assert_eq!(cs, "UTF-16");
        assert!(t.contains("Zoë /Ünal/"));
    }

    #[test]
    fn paf_style_file_with_lds_ordinances_round_trips() {
        let t = "0 HEAD\n1 SOUR PAF\n2 NAME Personal Ancestral File\n2 VERS 5.2.18.0\n2 CORP The Church of Jesus Christ of Latter-day Saints\n1 DEST ANSTFILE\n1 DATE 5 MAR 2001\n1 FILE family.ged\n1 GEDC\n2 VERS 5.5\n2 FORM LINEAGE-LINKED\n1 CHAR ANSEL\n1 SUBM @SUBM@\n\
0 @SUBM@ SUBM\n1 NAME Dennis\n\
0 @I1@ INDI\n1 NAME John /Smith/\n1 SEX M\n1 BIRT\n2 DATE 12 MAR 1850\n2 PLAC Salt Lake City, Salt Lake, Utah, USA\n2 SOUR @S1@\n3 PAGE p. 7\n1 DEAT\n2 DATE 3 FEB 1920\n2 PLAC Ogden, Weber, Utah\n1 BURI\n2 PLAC Ogden City Cemetery\n1 BAPL\n2 DATE 1 JAN 1880\n2 TEMP SLAKE\n2 STAT COMPLETED\n1 ENDL\n2 DATE 2 JAN 1880\n2 TEMP SLAKE\n1 SLGC\n2 FAMC @F1@\n2 DATE 3 JAN 1880\n2 TEMP SLAKE\n1 FAMS @F2@\n1 FAMC @F1@\n1 NOTE He was a pioneer.\n2 CONT Arrived in 1850.\n1 _UID ABCDEF\n1 RFN 99\n1 AFN 1ABC-DE\n\
0 @I2@ INDI\n1 NAME Jane /Doe/\n1 SEX F\n1 FAMS @F2@\n\
0 @I3@ INDI\n1 NAME Old /Smith/\n1 SEX M\n1 FAMS @F1@\n\
0 @F1@ FAM\n1 HUSB @I3@\n1 CHIL @I1@\n\
0 @F2@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 MARR\n2 DATE 4 JUL 1875\n2 PLAC Salt Lake City\n1 SLGS\n2 DATE 5 JUL 1875\n2 TEMP SLAKE\n2 STAT COMPLETED\n\
0 @S1@ SOUR\n1 TITL Family Bible\n1 AUTH Smith family\n\
0 TRLR\n";
        let (db, rep, out) = rt(t);
        assert!(rep.lost.is_empty(), "{:#?}", rep.lost);
        assert!(rep.program.contains("Personal Ancestral File"));
        assert_eq!(rep.gedcom_version, "5.5");
        let john = db.people.values().find(|p| p.given == "John").unwrap();
        assert_eq!(john.events.iter().filter(|e| e.kind.starts_with("LDS")).count(), 3);
        assert!(out.contains("2 TEMP SLAKE"));
        assert!(out.contains("1 CHAR UTF-8"));
    }

    #[test]
    fn private_copy_leaves_out_living_people_and_everything_hanging_off_them() {
        let t = "0 HEAD\n0 @N1@ NOTE secret note\n0 @S1@ SOUR\n1 TITL Secret source\n\
0 @I1@ INDI\n1 NAME Live /Person/\n1 NOTE @N1@\n1 SOUR @S1@\n1 BIRT\n2 DATE 1990\n1 OBJE\n2 FILE me.jpg\n\
0 @I2@ INDI\n1 NAME Dead /Person/\n1 BIRT\n2 DATE 1800\n\
0 @F1@ FAM\n1 HUSB @I1@\n1 MARR\n2 DATE 2015\n2 PLAC Secret Place\n1 NOTE @N1@\n0 TRLR\n";
        let (mut db, _) = parse_ged_report(t);
        for p in db.people.values_mut() {
            p.living = p.given == "Live";
        }
        let out = to_ged_full(&db.sanitized(), &ExportOpts { prune: true, media_dir: None }).text;
        for secret in ["secret note", "Secret source", "1990", "me.jpg", "Secret Place", "2015", "Live /Person/"] {
            assert!(!out.contains(secret), "leaked {}:\n{}", secret, out);
        }
        assert!(out.contains("Dead /Person/"));
        assert!(out.contains("1800"));
    }

    #[test]
    fn migrate_moves_old_event_values() {
        let mut db = Db::default();
        db.ver = 0;
        let id = db.alloc();
        db.people.insert(id, Person { events: vec![Event { kind: "Occupation".into(), note: "Baker".into(), ..Default::default() }, Event { kind: "Residence".into(), note: "near the mill".into(), ..Default::default() }], ..Default::default() });
        migrate(&mut db);
        let p = &db.people[&id];
        assert_eq!(p.events[0].val, "Baker");
        assert_eq!(p.events[1].note, "near the mill");
        migrate(&mut db);
        assert_eq!(db.people[&id].events[0].val, "Baker");
    }

    #[test]
    fn old_tree_json_without_new_fields_still_loads() {
        let old = r#"{"people":{"1":{"name":"A B","given":"A","surname":"B","sources":[{"about":"Birth","text":"Bible"}],"events":[{"kind":"Occupation","date":"","place":"","note":"Baker"}]}},"fams":{"2":{"h":1,"w":null,"md":"1900","mp":"","kids":[]}},"next":2}"#;
        let mut db: Db = serde_json::from_str(old).unwrap();
        migrate(&mut db);
        assert_eq!(db.people[&1].events[0].val, "Baker");
        let out = to_ged(&db);
        assert!(out.contains("1 OCCU Baker"));
        assert!(out.contains("2 DATE 1900"));
    }

    #[test]
    fn the_apps_own_sample_and_json_round_trip_through_gedcom() {
        let db = sample();
        let out = to_ged(&db);
        let (db2, rep) = parse_ged_report(&out);
        assert_eq!(db2.people.len(), db.people.len());
        assert_eq!(db2.fams.len(), db.fams.len());
        assert!(loss_check(&out, &to_ged(&db2)).is_empty());
        let _ = rep;
        // and the model itself survives JSON
        let j = serde_json::to_string(&db2).unwrap();
        let back: Db = serde_json::from_str(&j).unwrap();
        assert_eq!(to_ged(&back), to_ged(&db2));
    }

    #[test]
    fn event_tables_agree() {
        for (_, l) in EVENT_TAGS {
            assert!(EVENT_KINDS.contains(l), "{} missing from EVENT_KINDS", l);
        }
        for (_, l) in FAM_TAGS {
            assert!(FAM_KINDS.contains(l) || *l == "Census" || *l == "Residence", "{} missing from FAM_KINDS", l);
        }
        let mut seen = HashSet::new();
        for (t, _) in EVENT_TAGS {
            assert!(seen.insert(*t), "duplicate tag {}", t);
        }
    }


    #[test]
    fn merging_keeps_sources_notes_and_links_straight() {
        let a = "0 HEAD\n0 @S1@ SOUR\n1 TITL Census\n0 @N1@ NOTE note a\n0 @I1@ INDI\n1 NAME John /Smith/\n1 BIRT\n2 DATE 1900\n2 SOUR @S1@\n3 PAGE 1\n1 NOTE @N1@\n1 ASSO @I2@\n2 RELA friend\n\
0 @I2@ INDI\n1 NAME Bob /Friend/\n0 @F1@ FAM\n1 HUSB @I1@\n0 TRLR\n";
        let b = "0 HEAD\n0 @S7@ SOUR\n1 TITL Census\n0 @S8@ SOUR\n1 TITL Parish book\n0 @N5@ NOTE note b\n0 @N6@ NOTE note a\n0 @R1@ REPO\n1 NAME Archive\n\
0 @I1@ INDI\n1 NAME John /Smith/\n1 BIRT\n2 DATE 1900\n2 PLAC Town\n2 SOUR @S7@\n3 PAGE 2\n1 SOUR @S8@\n1 NOTE @N5@\n1 NOTE @N6@\n\
0 @I2@ INDI\n1 NAME Mary /Jones/\n1 ASSO @I1@\n2 RELA cousin\n1 NOTE @N5@\n0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 CHIL @I3@\n0 @I3@ INDI\n1 NAME Kid /Smith/\n1 FAMC @F1@\n2 PEDI adopted\n0 TRLR\n";
        let (mut da, _) = parse_ged_report(a);
        let (db, _) = parse_ged_report(b);
        let (added, matched) = merge_into(&mut da, db);
        assert_eq!((added, matched), (2, 1));
        assert_eq!(da.people.len(), 4);
        assert_eq!(da.sources.len(), 2, "Census is shared, Parish book is new");
        assert_eq!(da.notes.len(), 2, "note a is shared, note b is new");
        let john = da.people.values().find(|p| p.given == "John").unwrap();
        assert_eq!(john.bp, "Town");
        assert_eq!(john.note_refs.len(), 2);
        assert_eq!(john.sources.len(), 3, "two Census citations (different pages) and the parish book");
        // every link in the result points at something that exists
        let out = to_ged(&da);
        let (re, rep) = parse_ged_report(&out);
        assert!(rep.warnings.is_empty(), "{:?}", rep.warnings);
        assert_eq!(re.people.len(), 4);
        assert!(loss_check(&out, &to_ged(&re)).is_empty());
        let kid = da.people.iter().find(|(_, p)| p.given == "Kid").map(|(i, _)| *i).unwrap();
        assert!(da.fams.values().any(|f| f.kids.contains(&kid) && f.pedi.get(&kid).map(|s| s.as_str()) == Some("adopted")));
        // the ASSO from the second file now points at John, the ASSO from the first at Bob
        assert_eq!(out.matches("ASSO @I").count(), 2);
        assert!(out.contains(" REPO\n"), "repository record carried over");
        // merging the same file again adds nobody and nothing is duplicated
        let (db_again, _) = parse_ged_report(b);
        let (added2, _) = merge_into(&mut da, db_again);
        assert_eq!(added2, 0, "people without a birth year are recognised by their relatives");
        assert_eq!(da.people.len(), 4);
        assert_eq!(da.fams.len(), 1);
        assert_eq!(da.sources.len(), 2);
        assert_eq!(da.notes.len(), 2);
    }

    #[test]
    fn deleting_or_merging_a_person_leaves_no_dangling_links() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME A /B/\n1 ASSO @I2@\n2 RELA friend\n0 @I2@ INDI\n1 NAME C /D/\n0 @I3@ INDI\n1 NAME E /F/\n1 ASSO @I2@\n0 TRLR\n";
        let (db, _) = parse_ged_report(t);
        let ids: Vec<u32> = db.people.keys().copied().collect();
        let mut d = db.clone();
        d.people.remove(&ids[1]);
        replace_pointer(&mut d, &format!("I{}", ids[1]), None);
        let out = to_ged(&d);
        assert!(!out.contains("ASSO"), "{}", out);
        let mut d = db.clone();
        d.people.remove(&ids[1]);
        replace_pointer(&mut d, &format!("I{}", ids[1]), Some(&format!("I{}", ids[2])));
        let out = to_ged(&d);
        // I1 now points at I3; I3's own link to I2 would have pointed at itself, so it is dropped.
        assert_eq!(out.matches(&format!("ASSO @I{}@", ids[2])).count(), 1, "{}", out);
    }

    #[test]
    fn the_same_couple_recorded_twice_is_combined_without_losing_details() {
        let mut a = Family { h: Some(1), w: Some(2), md: "1900".into(), kids: vec![3], ..Default::default() };
        let b = Family { h: Some(1), w: Some(2), mp: "Reno".into(), kids: vec![3, 4], notes: "a note".into(), events: vec![Event { kind: "Divorce".into(), date: "1910".into(), ..Default::default() }], pedi: [(4, "adopted".to_string())].into_iter().collect(), ..Default::default() };
        absorb_family(&mut a, b);
        assert_eq!((a.md.as_str(), a.mp.as_str()), ("1900", "Reno"));
        assert_eq!(a.kids, vec![3, 4]);
        assert_eq!(a.events.len(), 1);
        assert_eq!(a.notes, "a note");
        assert_eq!(a.pedi.get(&4).map(|s| s.as_str()), Some("adopted"));
    }

    #[test]
    fn export_names_media_files_and_lists_them_for_copying() {
        let mut db = Db::default();
        let id = db.alloc();
        db.people.insert(id, Person { name: "A B".into(), given: "A".into(), surname: "B".into(), media: vec![
            Media { file: "111-photo.jpg".into(), name: "photo.jpg".into(), caption: "Front porch".into(), ..Default::default() },
            Media { file: "222-photo.jpg".into(), name: "photo.jpg".into(), ..Default::default() },
            Media { path: "C:\\gone\\old.tif".into(), name: "old.tif".into(), ..Default::default() },
        ], ..Default::default() });
        let ex = to_ged_full(&db, &ExportOpts { prune: false, media_dir: Some("tree-media".into()) });
        assert_eq!(ex.media.len(), 2);
        assert_eq!(ex.media[0].1, "tree-media/photo.jpg");
        assert_eq!(ex.media[1].1, "tree-media/photo-2.jpg");
        assert!(ex.text.contains("2 FILE tree-media/photo.jpg"));
        assert!(ex.text.contains("3 TITL Front porch"));
        assert!(ex.text.contains("2 FILE C:\\gone\\old.tif"), "reference to a file that was not found is kept");
    }

    #[test]
    fn the_loss_check_really_detects_damage() {
        let t = "0 HEAD\n0 @S1@ SOUR\n1 TITL Bible\n0 @N1@ NOTE shared words here\n0 @I1@ INDI\n1 NAME A /B/\n1 BIRT\n2 DATE 1900\n2 PLAC Town\n2 SOUR @S1@\n3 PAGE 9\n1 OCCU Baker\n1 NOTE hello there\n1 NOTE @N1@\n1 _WEIRD thing\n0 TRLR\n";
        let (db, _) = parse_ged_report(t);
        assert!(loss_check(t, &to_ged(&db)).is_empty());
        let mut d = db.clone();
        d.people.values_mut().next().unwrap().events.clear();
        assert!(loss_check(t, &to_ged(&d)).iter().any(|l| l.contains("OCCU")));
        let mut d = db.clone();
        d.people.values_mut().next().unwrap().bp.clear();
        assert!(loss_check(t, &to_ged(&d)).iter().any(|l| l.contains("PLAC")));
        let mut d = db.clone();
        d.notes.clear();
        assert!(!loss_check(t, &to_ged(&d)).is_empty());
        let mut d = db.clone();
        d.people.values_mut().next().unwrap().extra.clear();
        assert!(loss_check(t, &to_ged(&d)).iter().any(|l| l.contains("_WEIRD")));
        let mut d = db.clone();
        d.people.values_mut().next().unwrap().sources[0].page.clear();
        assert!(loss_check(t, &to_ged(&d)).iter().any(|l| l.contains("PAGE")));
        let mut d = db.clone();
        d.people.values_mut().next().unwrap().notes = "hello".into();
        assert!(loss_check(t, &to_ged(&d)).iter().any(|l| l.contains("NOTE")));
        let mut d = db.clone();
        d.people.values_mut().next().unwrap().bd = "1901".into();
        assert!(loss_check(t, &to_ged(&d)).iter().any(|l| l.contains("DATE")));
    }

    #[test]
    #[ignore]
    fn dump_roundtrip() {
        let (Ok(i), Ok(o)) = (std::env::var("FT_DUMP_IN"), std::env::var("FT_DUMP_OUT")) else { return };
        let (text, _) = decode_ged(&std::fs::read(i).unwrap());
        let (db, _) = parse_ged_report(&text);
        std::fs::write(o, to_ged(&db)).unwrap();
    }

    /// Real files from other programs, if a folder of them is given in FT_GED_SAMPLES.
    #[test]
    fn real_world_files_from_other_programs() {
        let Ok(dir) = std::env::var("FT_GED_SAMPLES") else {
            eprintln!("FT_GED_SAMPLES not set; skipping real-world files");
            return;
        };
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().map_or(true, |x| x != "ged") {
                continue;
            }
            let b = std::fs::read(&p).unwrap();
            if b.len() < 100 {
                continue;
            }
            let (text, cs) = decode_ged(&b);
            let (db, mut rep) = parse_ged_report(&text);
            let out = to_ged(&db);
            rep.lost = loss_check(&text, &out);
            eprintln!(
                "{}: {} / {} people, {} families, {} sources, {} notes, {} media, kept-as-is {} ({} kinds), warnings {}, LOST {}",
                p.file_name().unwrap().to_string_lossy(),
                cs,
                db.people.len(),
                db.fams.len(),
                db.sources.len(),
                db.notes.len(),
                rep.media,
                rep.kept_total(),
                rep.kept.len(),
                rep.warnings.len(),
                rep.lost.len()
            );
            for l in rep.lost.iter().take(25) {
                eprintln!("    lost: {}", l);
            }
            if std::env::var("FT_SHOW_KEPT").is_ok() {
                for (k, n) in &rep.kept {
                    eprintln!("    kept {:>4} {}", n, k);
                }
            }
            // second generation must be stable too
            let (db2, _) = parse_ged_report(&out);
            let out2 = to_ged(&db2);
            let again = loss_check(&out, &out2);
            assert!(again.is_empty(), "{}: second generation lost {:#?}", p.display(), again.iter().take(20).collect::<Vec<_>>());
            assert_eq!(db.people.len(), db2.people.len());
            assert_eq!(db.fams.len(), db2.fams.len());
            n += 1;
        }
        assert!(n > 0, "no .ged files found");
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;

    const SRC: &str = "0 HEAD\n1 GEDC\n2 VERS 5.5.1\n1 CHAR UTF-8\n\
0 @I1@ INDI\n1 NAME Anna /Berg/\n1 BIRT\n2 DATE 3 MAR 1880\n2 SOUR @S1@\n3 PAGE p. 12, line 4\n3 QUAY 3\n3 WWW https://example.org/rec/1\n3 _ACCESSED 5 JAN 2024\n3 DATA\n4 DATE 1 JAN 1900\n4 TEXT Anna Berg born 3 March\n1 RESI\n2 PLAC Oslo\n2 SOUR @S1@\n3 PAGE p. 99\n0 @I2@ INDI\n1 NAME Ola /Berg/\n1 SOUR @S1@\n2 PAGE other page\n\
0 @S1@ SOUR\n1 TITL Parish register\n1 AUTH Church of Norway\n1 WWW https://example.org/parish\n1 _TYPE Church record\n1 REPO @R1@\n2 CALN KB 4/2\n3 MEDI Microfilm\n\
0 @R1@ REPO\n1 NAME National Archives\n1 ADDR Oslo\n1 PHON 555-1\n1 EMAIL a@example.org\n1 WWW https://example.org\n1 NOTE Open weekdays\n0 TRLR\n";

    #[test]
    fn repositories_and_details_become_fields_and_round_trip() {
        let (db, rep) = parse_ged_report(SRC);
        assert_eq!(db.repos.len(), 1);
        let r = db.repos.values().next().unwrap();
        assert_eq!((r.name.as_str(), r.addr.as_str(), r.phone.as_str(), r.url.as_str()), ("National Archives", "Oslo", "555-1", "https://example.org"));
        assert_eq!(r.notes, "Open weekdays");
        let s = db.sources.values().next().unwrap();
        assert_eq!(s.url, "https://example.org/parish");
        assert_eq!(s.kind, "Church record");
        assert_eq!(s.call, "KB 4/2");
        assert_eq!(s.medium, "Microfilm");
        assert_eq!(s.repo, db.repos.keys().next().copied());
        assert!(s.extra.is_empty(), "nothing should be left over as raw text");
        let ann = db.people.values().find(|p| p.name.starts_with("Anna")).unwrap();
        let c = ann.sources.iter().find(|c| c.about == "Birth").expect("birth citation");
        assert_eq!((c.page.as_str(), c.quality.as_str(), c.url.as_str(), c.accessed.as_str()), ("p. 12, line 4", "3", "https://example.org/rec/1", "5 JAN 2024"));
        assert_eq!(c.data, "Anna Berg born 3 March");
        assert!(db.records.iter().all(|r| !r.contains(" REPO")), "repositories are no longer kept as raw records");
        assert!(rep.lost.is_empty());
        let out = to_ged(&db);
        let lost = loss_check(SRC, &out);
        assert!(lost.is_empty(), "{:#?}\n{}", lost, out);
        let (db2, _) = parse_ged_report(&out);
        assert_eq!(db2.repos.len(), 1);
        assert_eq!(db2.sources.len(), 1);
        assert!(loss_check(&out, &to_ged(&db2)).is_empty());
    }

    #[test]
    fn one_source_serves_many_people_and_events() {
        let (db, _) = parse_ged_report(SRC);
        let sid = *db.sources.keys().next().unwrap();
        let used: usize = db.people.values().map(|p| p.sources.iter().filter(|c| c.sid == Some(sid)).count() + p.fx.values().flat_map(|e| e.cites.iter()).filter(|c| c.sid == Some(sid)).count() + p.events.iter().flat_map(|e| e.cites.iter()).filter(|c| c.sid == Some(sid)).count()).sum();
        assert_eq!(used, 3);
    }

    #[test]
    fn old_trees_with_raw_repository_records_are_upgraded() {
        // How version 2 stored it: the record kept whole, and the link kept as raw text under the source.
        let mut db = Db::default();
        let id = db.alloc();
        db.sources.insert(id, SourceRec { title: "Parish register".into(), extra: vec![Raw { at: "".into(), text: "0 REPO @R9@\n1 CALN KB 4/2\n2 MEDI Microfilm\n".into() }], ..Default::default() });
        db.records.push("0 @R9@ REPO\n1 NAME National Archives\n1 WWW https://example.org\n".into());
        db.records.push("0 @U1@ SUBM\n1 NAME Me\n".into());
        db.ver = 2;
        migrate(&mut db);
        assert_eq!(db.ver, 4);
        assert_eq!(db.repos.get(&9).map(|r| r.name.as_str()), Some("National Archives"));
        let s = &db.sources[&id];
        assert_eq!((s.repo, s.call.as_str(), s.medium.as_str()), (Some(9), "KB 4/2", "Microfilm"));
        assert!(s.extra.is_empty());
        assert_eq!(db.records.len(), 1, "other records stay");
        assert!(to_ged(&db).contains("0 @R9@ REPO"));
    }

    #[test]
    fn merging_trees_shares_repositories_and_keeps_links() {
        let (mut a, _) = parse_ged_report(SRC);
        let (b, _) = parse_ged_report(&SRC.replace("Anna /Berg/", "Berta /Lund/").replace("Ola /Berg/", "Per /Lund/").replace("Parish register", "Other register"));
        merge_into(&mut a, b);
        assert_eq!(a.repos.len(), 1, "same repository is not duplicated");
        assert_eq!(a.sources.len(), 2);
        let repo = *a.repos.keys().next().unwrap();
        assert!(a.sources.values().all(|s| s.repo == Some(repo)));
        let out = to_ged(&a);
        assert_eq!(out.matches(" REPO\n").count(), 1);
    }

    #[test]
    fn private_copy_keeps_only_repositories_in_use() {
        let (mut db, _) = parse_ged_report(SRC);
        let rid = db.alloc();
        db.repos.insert(rid, RepoRec { name: "Unused library".into(), ..Default::default() });
        let full = to_ged_full(&db, &ExportOpts { prune: true, media_dir: None }).text;
        assert!(full.contains("National Archives"));
        assert!(!full.contains("Unused library"));
    }
}

/* ------------------------------------------------------------------ */
/* Suggesting who might be the same person                             */
/* ------------------------------------------------------------------ */

/// A suggestion that an incoming person is someone already in the tree. It is only a clue.
#[derive(Clone)]
pub struct MatchCand {
    pub inc: u32,
    pub dst: u32,
    pub score: i32,
    pub why: Vec<String>,
    pub against: Vec<String>,
}

impl MatchCand {
    pub fn strength(&self) -> &'static str {
        if !self.against.is_empty() {
            "Doubtful"
        } else if self.score >= 12 {
            "Strong"
        } else if self.score >= 8 {
            "Fair"
        } else {
            "Weak (little to go on)"
        }
    }
}

fn alnum(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

fn name_key(p: &Person) -> (Vec<String>, String) {
    let words: Vec<&str> = p.name.split_whitespace().collect();
    let surname = if !p.surname.is_empty() { alnum(&p.surname) } else { words.last().map(|w| alnum(w)).unwrap_or_default() };
    let given: Vec<String> = if !p.given.is_empty() { p.given.split_whitespace().map(alnum).collect() } else { words.iter().take(words.len().saturating_sub(1)).map(|w| alnum(w)).collect() };
    (given.into_iter().filter(|g| !g.is_empty()).collect(), surname)
}

fn rel_keys(db: &Db, id: u32) -> (Vec<String>, Vec<String>, Vec<String>) {
    // (parents, spouses, children) as lower-case names
    let nm = |i: &u32| db.people.get(i).map(|p| p.name.to_lowercase()).filter(|n| !n.is_empty());
    let (f, m) = db.parents(id);
    let parents: Vec<String> = [f, m].iter().flatten().filter_map(|i| nm(i)).collect();
    let mut spouses = Vec::new();
    let mut kids = Vec::new();
    for fid in db.fams_of(id) {
        let fam = &db.fams[&fid];
        for s in [fam.h, fam.w].iter().flatten().filter(|s| **s != id) {
            if let Some(n) = nm(s) {
                spouses.push(n);
            }
        }
        for k in &fam.kids {
            if let Some(n) = nm(k) {
                kids.push(n);
            }
        }
    }
    (parents, spouses, kids)
}

/// For each incoming person, the people in the tree who might be the same person, best first.
pub fn merge_candidates(dst: &Db, inc: &Db) -> Vec<MatchCand> {
    let mut buckets: HashMap<String, Vec<u32>> = HashMap::new();
    for (id, p) in &dst.people {
        let (_, sn) = name_key(p);
        if !sn.is_empty() {
            buckets.entry(sn).or_default().push(*id);
        }
        let mn = alnum(&p.maiden);
        if !mn.is_empty() {
            buckets.entry(mn).or_default().push(*id);
        }
    }
    let dst_rels: HashMap<u32, (Vec<String>, Vec<String>, Vec<String>)> = dst.people.keys().map(|i| (*i, rel_keys(dst, *i))).collect();
    let mut out: Vec<MatchCand> = Vec::new();
    for (iid, ip) in &inc.people {
        let (ig, isn) = name_key(ip);
        let mut keys = vec![isn.clone()];
        let imn = alnum(&ip.maiden);
        if !imn.is_empty() {
            keys.push(imn);
        }
        let mut seen: HashSet<u32> = HashSet::new();
        let irel = rel_keys(inc, *iid);
        let mut mine: Vec<MatchCand> = Vec::new();
        for k in keys.iter().filter(|k| !k.is_empty()) {
            for did in buckets.get(k).into_iter().flatten() {
                if !seen.insert(*did) {
                    continue;
                }
                let dp = &dst.people[did];
                let (dg, _) = name_key(dp);
                let (mut score, mut why, mut against) = (0i32, Vec::<String>::new(), Vec::<String>::new());
                match (ig.first(), dg.first()) {
                    (Some(a), Some(b)) if a == b => {
                        score += 3;
                        why.push("same first name".into());
                        if ig == dg {
                            score += 2;
                            why.push("same given names".into());
                        } else if ig.iter().all(|x| dg.contains(x)) || dg.iter().all(|x| ig.contains(x)) {
                            score += 2;
                            why.push("one record has a fuller name".into());
                        }
                    }
                    (Some(a), Some(b)) if a.chars().count() <= 2 && b.starts_with(a.as_str()) || b.chars().count() <= 2 && a.starts_with(b.as_str()) => {
                        score += 1;
                        why.push("first name is an initial of the other".into());
                    }
                    (None, _) | (_, None) => {}
                    _ => continue,
                }
                if !ip.sex.is_empty() && !dp.sex.is_empty() && ip.sex != dp.sex {
                    continue;
                }
                let (by, dy) = (year(&ip.bd), year(&dp.bd));
                if let (Ok(a), Ok(b)) = (by.parse::<i32>(), dy.parse::<i32>()) {
                    if a == b {
                        score += 4;
                        why.push("same birth year".into());
                    } else if (a - b).abs() <= 2 {
                        score += 1;
                        why.push("birth years within 2".into());
                    } else {
                        score -= 6;
                        against.push(format!("born {} and {}", a, b));
                    }
                }
                let (iy, dyr) = (year(&ip.dd), year(&dp.dd));
                if let (Ok(a), Ok(b)) = (iy.parse::<i32>(), dyr.parse::<i32>()) {
                    if a == b {
                        score += 2;
                        why.push("same death year".into());
                    } else if (a - b).abs() > 2 {
                        score -= 4;
                        against.push(format!("died {} and {}", a, b));
                    }
                }
                if !ip.bp.is_empty() && alnum(&ip.bp) == alnum(&dp.bp) {
                    score += 1;
                    why.push("same birthplace".into());
                }
                let drel = &dst_rels[did];
                let shared = |a: &Vec<String>, b: &Vec<String>| a.iter().filter(|x| b.contains(x)).count() as i32;
                let sp = shared(&irel.0, &drel.0);
                if sp > 0 {
                    score += 3 * sp;
                    why.push(if sp > 1 { "same parents' names".into() } else { "a parent has the same name".into() });
                } else if !irel.0.is_empty() && !drel.0.is_empty() {
                    score -= 2;
                    against.push("their parents are named differently".into());
                }
                let ss = shared(&irel.1, &drel.1);
                if ss > 0 {
                    score += 2;
                    why.push("a spouse has the same name".into());
                }
                let sk = shared(&irel.2, &drel.2).min(3);
                if sk > 0 {
                    score += sk;
                    why.push("children with the same names".into());
                }
                if score >= 5 || (score >= 3 && !against.is_empty()) {
                    mine.push(MatchCand { inc: *iid, dst: *did, score, why, against });
                }
            }
        }
        mine.sort_by(|a, b| b.score.cmp(&a.score).then(a.dst.cmp(&b.dst)));
        mine.truncate(5);
        out.extend(mine);
    }
    out.sort_by(|a, b| b.score.cmp(&a.score).then(a.inc.cmp(&b.inc)));
    out
}

#[cfg(test)]
mod relationship_tests {
    use super::*;

    const T: &str = "0 HEAD\n1 GEDC\n2 VERS 5.5.1\n1 CHAR UTF-8\n\
0 @I1@ INDI\n1 NAME Anna /Berg/\n1 SEX F\n1 FAMS @F1@\n1 FAMS @F2@\n\
0 @I2@ INDI\n1 NAME Bo /Berg/\n1 SEX M\n1 FAMS @F1@\n\
0 @I3@ INDI\n1 NAME Cal /Dahl/\n1 SEX M\n1 FAMS @F2@\n\
0 @I4@ INDI\n1 NAME Kid /Berg/\n1 SEX M\n1 FAMC @F1@\n2 PEDI birth\n2 _PREF Y\n1 FAMC @F2@\n2 PEDI adopted\n2 _FREL Adopted\n2 _MREL Natural\n\
0 @I5@ INDI\n1 NAME Gus /Moe/\n1 SEX M\n1 ASSO @I4@\n2 TYPE INDI\n2 RELA Guardian\n2 NOTE Appointed 1901\n\
0 @F1@ FAM\n1 HUSB @I2@\n1 WIFE @I1@\n1 CHIL @I4@\n1 _STAT Married\n1 MARR\n2 DATE 1900\n\
0 @F2@ FAM\n1 HUSB @I3@\n1 WIFE @I1@\n1 CHIL @I4@\n1 _STAT Unmarried\n1 MARR\n2 DATE 1910\n\
0 TRLR\n";

    #[test]
    fn complex_family_links_become_fields_and_round_trip() {
        let (db, rep) = parse_ged_report(T);
        assert!(rep.lost.is_empty());
        let kid = db.people.values().position(|p| p.name.starts_with("Kid")).unwrap();
        let kid_id = *db.people.keys().nth(kid).unwrap();
        let fams = db.parent_families(kid_id);
        assert_eq!(fams.len(), 2);
        let f2 = fams.iter().find(|f| db.fams[f].kind == "Unmarried").unwrap();
        assert_eq!(db.fams[f2].rel_to(kid_id, true), "Adopted");
        assert_eq!(db.fams[f2].rel_to(kid_id, false), "Natural", "born to the mother, adopted by her partner");
        assert!(db.people[&kid_id].pref_parents.is_some());
        assert_eq!(db.people[&kid_id].pref_parents, fams.first().copied().filter(|f| db.fams[f].kind == "Married"));
        let guard = db.people.values().find(|p| p.name.starts_with("Gus")).unwrap();
        assert_eq!((guard.assoc[0].role.as_str(), guard.assoc[0].note.as_str()), ("Guardian", "Appointed 1901"));
        assert!(guard.extra.is_empty());
        let out = to_ged(&db);
        let lost = loss_check(T, &out);
        assert!(lost.is_empty(), "{:#?}\n{}", lost, out);
        assert!(out.contains("_FREL Adopted") && out.contains("_MREL Natural") && out.contains("_PREF Y") && out.contains("_STAT Unmarried"));
        let (db2, _) = parse_ged_report(&out);
        assert!(loss_check(&out, &to_ged(&db2)).is_empty());
    }

    #[test]
    fn partners_come_in_the_order_of_their_marriages() {
        let (db, _) = parse_ged_report(T);
        let anna = *db.people.iter().find(|(_, p)| p.name.starts_with("Anna")).unwrap().0;
        let fams = db.fams_of(anna);
        let years: Vec<String> = fams.iter().map(|f| db.fams[f].md.clone()).collect();
        assert_eq!(years, vec!["1900", "1910"]);
    }

    #[test]
    fn old_raw_relationship_text_is_upgraded_to_fields() {
        let mut db = Db::default();
        let (a, b, f) = (db.alloc(), db.alloc(), db.alloc());
        db.people.insert(a, Person { name: "Kid".into(), extra: vec![Raw { at: format!("FAMC@F{}", f), text: "0 _FREL Step\n0 _PREF Y\n0 STAT proven\n".into() }, Raw { at: "".into(), text: format!("0 ASSO @I{}@\n1 RELA Godfather\n", b) }], ..Default::default() });
        db.people.insert(b, Person { name: "Pa".into(), ..Default::default() });
        db.fams.insert(f, Family { h: Some(b), kids: vec![a], ..Default::default() });
        db.ver = 3;
        migrate(&mut db);
        assert_eq!(db.ver, 4);
        assert_eq!(db.fams[&f].frel.get(&a).map(|s| s.as_str()), Some("Step"));
        assert_eq!(db.people[&a].pref_parents, Some(f));
        assert_eq!(db.people[&a].assoc[0].role, "Godfather");
        assert_eq!(db.people[&a].extra.len(), 1, "the unrelated STAT line stays as raw text");
        assert!(db.people[&a].extra[0].text.contains("STAT proven"));
    }

    #[test]
    fn deleting_or_merging_people_cleans_associations_and_choices() {
        let (mut db, _) = parse_ged_report(T);
        let kid = *db.people.iter().find(|(_, p)| p.name.starts_with("Kid")).unwrap().0;
        let gus = *db.people.iter().find(|(_, p)| p.name.starts_with("Gus")).unwrap().0;
        let mut d = db.clone();
        d.people.remove(&kid);
        replace_pointer(&mut d, &format!("I{}", kid), None);
        assert!(d.people[&gus].assoc.is_empty());
        // merging Kid into Gus: Gus cannot be his own guardian
        db.merge_people(gus, kid);
        assert!(db.people[&gus].assoc.is_empty());
        assert!(db.people[&gus].pref_parents.is_some(), "the chosen parents move with the merge");
    }

    #[test]
    fn merging_trees_keeps_relationship_choices_attached_to_the_right_people() {
        let (mut a, _) = parse_ged_report(T);
        let (b, _) = parse_ged_report(&T.replace("Berg", "Lund").replace("Dahl", "Vik").replace("Moe", "Ness"));
        let before = a.people.len();
        merge_into_pairs(&mut a, b, &HashMap::new());
        assert_eq!(a.people.len(), before * 2);
        let kids: Vec<u32> = a.people.iter().filter(|(_, p)| p.name.starts_with("Kid")).map(|(i, _)| *i).collect();
        assert_eq!(kids.len(), 2);
        for k in kids {
            let fams = a.parent_families(k);
            assert_eq!(fams.len(), 2);
            let adopted = fams.iter().filter(|f| a.fams[f].rel_to(k, true) == "Adopted").count();
            assert_eq!(adopted, 1);
            assert!(a.people[&k].pref_parents.map_or(false, |f| a.fams[&f].kids.contains(&k)));
        }
        for p in a.people.values().filter(|p| p.name.starts_with("Gus")) {
            let o = p.assoc[0].other;
            assert!(a.people[&o].name.starts_with("Kid"));
        }
        assert!(loss_check(&to_ged(&a), &to_ged(&parse_ged_report(&to_ged(&a)).0)).is_empty());
    }
}
