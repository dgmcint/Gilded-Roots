//! GEDCOM 7 and GEDZIP.
//!
//! Gilded Roots keeps its own reading and writing in GEDCOM 5.5.1 terms (gedcom.rs). A GEDCOM 7
//! file is turned into the 5.5.1 shape before it is read, and a 5.5.1 export is turned into
//! GEDCOM 7 when that is asked for. Both steps work on the tree of lines (`Node`), and are built
//! so that a tree written as GEDCOM 7 and read back comes in the same as from 5.5.1.
//!
//! The main differences handled here:
//! - the header (GEDC VERS 7.0, no CHAR), no CONC lines, "@@" at the start of every text line;
//! - shared notes are SNOTE records; inline sources and repositories must be records;
//! - pictures are always OBJE records, FILE is a URL (spaces written %20) and FORM is a media
//!   type (image/jpeg); FORM's TYPE is MEDI;
//! - dates: calendars are words (JULIAN), "BCE", no INT, and other text goes in a PHRASE;
//! - fixed words in capitals with a PHRASE for anything else (PEDI, ROLE, NAME TYPE, MEDI...);
//! - AFN, RFN, RIN, ROMN and FONE were taken out of the standard.
//!
//! A GEDZIP (.gdz) file is a zip with the GEDCOM 7 file as `gedcom.ged` and the pictures it
//! points to, by the same relative paths.

use crate::gedcom::{build_tree, Node};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// Put in the header of a file turned into 5.5.1, so it is not turned again.
const DONE_MARK: &str = "_GR_FROM7";

/// True when the text is a GEDCOM 7 file (its header says GEDC VERS 7...).
pub fn is_v7(text: &str) -> bool {
    let mut in_head = false;
    let mut in_gedc = false;
    for line in text.lines().take(400) {
        let l = line.trim_start_matches('\u{feff}').trim();
        if l.starts_with("0 ") {
            if in_head {
                return false;
            }
            in_head = l == "0 HEAD";
            continue;
        }
        if !in_head {
            continue;
        }
        if l.starts_with("1 ") {
            if l.starts_with(&format!("1 {}", DONE_MARK)) {
                return false;
            }
            in_gedc = l == "1 GEDC";
            continue;
        }
        if in_gedc {
            if let Some(v) = l.strip_prefix("2 VERS") {
                let v = v.trim();
                if v == "7" || v.starts_with("7.") {
                    // a later DONE_MARK line can still turn this off
                    let rest_has_mark = text.lines().take(400).any(|x| x.trim().starts_with(&format!("1 {}", DONE_MARK)));
                    return !rest_has_mark;
                }
                return false;
            }
        }
    }
    false
}

/* ------------------------------------------------------------------ */
/* Writing a tree of lines                                             */
/* ------------------------------------------------------------------ */

fn write_node(o: &mut String, n: &Node, lvl: u32, v7: bool) {
    let mut lines = n.val.split('\n');
    let first = lines.next().unwrap_or("");
    let _ = write!(o, "{}", lvl);
    if let Some(x) = &n.xref {
        let _ = write!(o, " @{}@", x);
    }
    let _ = write!(o, " {}", n.tag);
    if !first.is_empty() {
        let _ = write!(o, " {}", first);
    }
    o.push('\n');
    for l in lines {
        if l.is_empty() {
            let _ = writeln!(o, "{} CONT", lvl + 1);
        } else if v7 && l.starts_with('@') {
            let _ = writeln!(o, "{} CONT @{}", lvl + 1, l);
        } else {
            let _ = writeln!(o, "{} CONT {}", lvl + 1, l);
        }
    }
    for k in &n.kids {
        write_node(o, k, lvl + 1, v7);
    }
}

fn write_all(roots: &[Node], v7: bool) -> String {
    let mut o = String::new();
    for r in roots {
        write_node(&mut o, r, 0, v7);
    }
    o
}

fn node(tag: &str, val: &str) -> Node {
    Node { xref: None, tag: tag.to_string(), val: val.to_string(), kids: Vec::new() }
}

fn is_ptr(v: &str) -> bool {
    v.len() > 2 && v.starts_with('@') && v.ends_with('@') && !v.starts_with("@@") && !v.starts_with("@#") && !v.contains(char::is_whitespace)
}

fn phrase_of(n: &Node) -> Option<String> {
    n.kids.iter().find(|k| k.tag == "PHRASE").map(|k| k.val.clone())
}

fn take_phrase(n: &mut Node) -> Option<String> {
    let i = n.kids.iter().position(|k| k.tag == "PHRASE")?;
    Some(n.kids.remove(i).val)
}

fn set_phrase(n: &mut Node, p: Option<String>) {
    if let Some(p) = p {
        if !n.kids.iter().any(|k| k.tag == "PHRASE") {
            n.kids.insert(0, node("PHRASE", &p));
        }
    }
}

/* ------------------------------------------------------------------ */
/* Fixed words                                                         */
/* ------------------------------------------------------------------ */

/// A list of fixed words: (GEDCOM 7 word, how 5.5.1 writes it).
type Words = &'static [(&'static str, &'static str)];

const PEDI: Words = &[("BIRTH", "birth"), ("ADOPTED", "adopted"), ("FOSTER", "foster"), ("SEALING", "sealing")];
const FAMC_STAT: Words = &[("CHALLENGED", "challenged"), ("DISPROVEN", "disproven"), ("PROVEN", "proven")];
const NAME_TYPE: Words = &[("AKA", "aka"), ("BIRTH", "birth"), ("IMMIGRANT", "immigrant"), ("MAIDEN", "maiden"), ("MARRIED", "married"), ("PROFESSIONAL", "professional")];
const MEDI: Words = &[
    ("AUDIO", "audio"),
    ("BOOK", "book"),
    ("CARD", "card"),
    ("ELECTRONIC", "electronic"),
    ("FICHE", "fiche"),
    ("FILM", "film"),
    ("MAGAZINE", "magazine"),
    ("MANUSCRIPT", "manuscript"),
    ("MAP", "map"),
    ("NEWSPAPER", "newspaper"),
    ("PHOTO", "photo"),
    ("TOMBSTONE", "tombstone"),
    ("VIDEO", "video"),
];
const ROLE: Words = &[
    ("CHIL", "child"),
    ("CLERGY", "clergy"),
    ("FATH", "father"),
    ("FRIEND", "friend"),
    ("GODP", "godparent"),
    ("HUSB", "husband"),
    ("MOTH", "mother"),
    ("MULTIPLE", "multiple"),
    ("NGHBR", "neighbor"),
    ("OFFICIATOR", "officiator"),
    ("PARENT", "parent"),
    ("SPOU", "spouse"),
    ("WIFE", "wife"),
    ("WITN", "witness"),
];

/// A 5.5.1 word as GEDCOM 7 writes it, with a PHRASE when that would not give the same word back.
fn word_to7(v: &str, words: Words, guess: &dyn Fn(&str) -> Option<&'static str>) -> (String, Option<String>) {
    let t = v.trim();
    let hit = words.iter().find(|(a, b)| a.eq_ignore_ascii_case(t) || b.eq_ignore_ascii_case(t)).map(|(a, _)| *a).or_else(|| guess(&t.to_lowercase()));
    let code = hit.unwrap_or("OTHER");
    let back = word_from7(code, None, words);
    (code.to_string(), if back == t { None } else { Some(t.to_string()) })
}

/// A GEDCOM 7 word (and its PHRASE) as 5.5.1 writes it.
fn word_from7(code: &str, phrase: Option<&str>, words: Words) -> String {
    if let Some(p) = phrase.filter(|p| !p.trim().is_empty()) {
        return p.to_string();
    }
    words.iter().find(|(a, _)| a.eq_ignore_ascii_case(code.trim())).map(|(_, b)| b.to_string()).unwrap_or_else(|| code.trim().to_lowercase())
}

fn no_guess(_: &str) -> Option<&'static str> {
    None
}

fn guess_role(r: &str) -> Option<&'static str> {
    let has = |w: &str| r.contains(w);
    Some(if has("godp") || has("godf") || has("godm") || has("sponsor") {
        "GODP"
    } else if has("witn") {
        "WITN"
    } else if has("friend") {
        "FRIEND"
    } else if has("neighb") {
        "NGHBR"
    } else if has("officiat") {
        "OFFICIATOR"
    } else if has("clergy") || has("priest") || has("minister") || has("pastor") || has("rabbi") {
        "CLERGY"
    } else if has("grand") || has("step") || has("in-law") || has("in law") {
        return None;
    } else if has("father") {
        "FATH"
    } else if has("mother") {
        "MOTH"
    } else if has("parent") {
        "PARENT"
    } else if has("husband") {
        "HUSB"
    } else if has("wife") {
        "WIFE"
    } else if has("spouse") {
        "SPOU"
    } else if r == "son" || r == "daughter" || r == "child" {
        "CHIL"
    } else {
        return None;
    })
}

/// Languages: (GEDCOM 7 language tag, the 5.5.1 language name).
const LANGS: Words = &[
    ("en", "English"),
    ("de", "German"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("it", "Italian"),
    ("nl", "Dutch"),
    ("no", "Norwegian"),
    ("nb", "Norwegian"),
    ("pt", "Portuguese"),
    ("sv", "Swedish"),
    ("da", "Danish"),
    ("fi", "Finnish"),
    ("is", "Icelandic"),
    ("pl", "Polish"),
    ("cs", "Czech"),
    ("sk", "Slovak"),
    ("hu", "Hungarian"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("uk", "Ukrainian"),
    ("el", "Greek"),
    ("la", "Latin"),
    ("he", "Hebrew"),
    ("yi", "Yiddish"),
    ("ar", "Arabic"),
    ("ja", "Japanese"),
    ("zh", "Chinese"),
    ("ko", "Korean"),
    ("haw", "Hawaiian"),
    ("cy", "Welsh"),
    ("ga", "Irish"),
    ("gd", "Gaelic"),
];

/// Media types: (GEDCOM 7 FORM, the file ending 5.5.1 uses). The first ending is the one read back.
const MIME: &[(&str, &str)] = &[
    ("image/jpeg", "jpg"),
    ("image/jpeg", "jpeg"),
    ("image/png", "png"),
    ("image/gif", "gif"),
    ("image/bmp", "bmp"),
    ("image/tiff", "tif"),
    ("image/tiff", "tiff"),
    ("image/webp", "webp"),
    ("image/heic", "heic"),
    ("image/svg+xml", "svg"),
    ("application/pdf", "pdf"),
    ("audio/mpeg", "mp3"),
    ("audio/wav", "wav"),
    ("audio/mp4", "m4a"),
    ("audio/ogg", "ogg"),
    ("audio/flac", "flac"),
    ("video/mp4", "mp4"),
    ("video/quicktime", "mov"),
    ("video/x-msvideo", "avi"),
    ("text/plain", "txt"),
    ("text/html", "htm"),
    ("text/html", "html"),
    ("application/rtf", "rtf"),
    ("application/msword", "doc"),
    ("application/vnd.openxmlformats-officedocument.wordprocessingml.document", "docx"),
    ("application/vnd.ms-excel", "xls"),
    ("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "xlsx"),
    ("message/rfc822", "eml"),
];

fn mime_of(form: &str) -> String {
    let f = form.trim().trim_start_matches('.').to_lowercase();
    if f.contains('/') {
        return f;
    }
    if f.is_empty() {
        return "application/octet-stream".to_string();
    }
    MIME.iter().find(|(_, e)| *e == f).map(|(m, _)| m.to_string()).unwrap_or_else(|| format!("application/x-{}", f))
}

fn form_of(mime: &str) -> String {
    let m = mime.trim().to_lowercase();
    if let Some((_, e)) = MIME.iter().find(|(x, _)| *x == m) {
        return e.to_string();
    }
    if m == "application/octet-stream" {
        return String::new();
    }
    match m.strip_prefix("application/x-") {
        Some(e) => e.to_string(),
        None => m,
    }
}

/* ------------------------------------------------------------------ */
/* File names as URLs                                                  */
/* ------------------------------------------------------------------ */

fn pct_encode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/:!$&'()*+,;=@".contains(&b) {
            o.push(b as char);
        } else {
            let _ = write!(o, "%{:02X}", b);
        }
    }
    o
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() && s.is_char_boundary(i + 1) && s.is_char_boundary(i + 3) {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn is_web(p: &str) -> bool {
    let l = p.to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://") || l.starts_with("ftp://")
}

/// A file name as GEDCOM 7 writes it: a URL, relative where it was relative.
pub fn path_to_url(p: &str) -> String {
    let p = p.trim();
    if p.is_empty() || is_web(p) || p.to_ascii_lowercase().starts_with("file:") {
        return p.to_string();
    }
    let fwd = p.replace('\\', "/");
    let b = fwd.as_bytes();
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        // C:/Users/...
        return format!("file:///{}", pct_encode(&fwd));
    }
    if let Some(rest) = fwd.strip_prefix("//") {
        // a shared network folder: \\server\share\...
        return format!("file://{}", pct_encode(rest));
    }
    if fwd.starts_with('/') {
        return format!("file://{}", pct_encode(&fwd));
    }
    pct_encode(&fwd)
}

/// A GEDCOM 7 URL as a file name.
pub fn url_to_path(u: &str) -> String {
    let u = u.trim();
    if is_web(u) {
        return u.to_string();
    }
    let lower = u.to_ascii_lowercase();
    if lower.starts_with("file://") {
        let rest = &u[7..];
        let d = pct_decode(rest);
        if let Some(r) = d.strip_prefix('/') {
            let b = r.as_bytes();
            if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
                return r.to_string();
            }
            return d;
        }
        if d.to_ascii_lowercase().starts_with("localhost/") {
            return d[9..].to_string();
        }
        return format!("//{}", d);
    }
    if lower.starts_with("file:") {
        return pct_decode(&u[5..]);
    }
    pct_decode(u)
}

/* ------------------------------------------------------------------ */
/* Dates and ages                                                      */
/* ------------------------------------------------------------------ */

const MONTHS: &[&str] = &[
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC", // Gregorian and Julian
    "TSH", "CSH", "KSL", "TVT", "SHV", "ADR", "ADS", "NSN", "IYR", "SVN", "TMZ", "AAV", "ELL", // Hebrew
    "VEND", "BRUM", "FRIM", "NIVO", "PLUV", "VENT", "GERM", "FLOR", "PRAI", "MESS", "THER", "FRUC", "COMP", // French
];
const WORDS7: &[&str] = &["ABT", "CAL", "EST", "BEF", "AFT", "BET", "AND", "FROM", "TO", "GREGORIAN", "JULIAN", "HEBREW", "FRENCH_R", "BCE"];

/// Is this a date GEDCOM 7 can hold as it is?
fn valid7(v: &str) -> bool {
    v.split_whitespace().all(|t| WORDS7.contains(&t) || MONTHS.contains(&t) || (t.chars().all(|c| c.is_ascii_digit()) && t.len() <= 4))
}

/// A 5.5.1 date as GEDCOM 7 writes it, and the PHRASE that goes with it.
pub fn date_to7(v: &str) -> (String, Option<String>) {
    let orig = v.trim();
    if orig.is_empty() {
        return (String::new(), None);
    }
    // a date that is only words in brackets: "(Spring of 1850)"
    if orig.starts_with('(') && orig.ends_with(')') {
        return (String::new(), Some(orig[1..orig.len() - 1].to_string()));
    }
    let mut s = orig.to_uppercase();
    let mut phrase: Option<String> = None;
    if s.starts_with("INT ") {
        let rest = &orig[4..];
        match (rest.find('('), rest.rfind(')')) {
            (Some(a), Some(b)) if b > a => {
                s = rest[..a].trim().to_uppercase();
                phrase = Some(rest[a + 1..b].to_string());
            }
            _ => s = rest.trim().to_uppercase(),
        }
    }
    let s = s
        .replace("@#DGREGORIAN@", "")
        .replace("@#DJULIAN@", "JULIAN")
        .replace("@#DHEBREW@", "HEBREW")
        .replace("@#DFRENCH R@", "FRENCH_R")
        .replace("B.C.", "BCE")
        .replace("(B.C.)", "BCE");
    let mut toks: Vec<String> = Vec::new();
    let mut dual = false;
    for t in s.split_whitespace() {
        match t {
            "BC" | "B.C" | "BC." => toks.push("BCE".into()),
            "ABOUT" => toks.push("ABT".into()),
            "BEFORE" => toks.push("BEF".into()),
            "AFTER" => toks.push("AFT".into()),
            "BETWEEN" => toks.push("BET".into()),
            "ESTIMATED" => toks.push("EST".into()),
            "CALCULATED" => toks.push("CAL".into()),
            _ => {
                // dual dating, 1850/51: GEDCOM 7 has the later (new style) year
                if let Some((a, b)) = t.split_once('/') {
                    if !a.is_empty() && !b.is_empty() && a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|c| c.is_ascii_digit()) && b.len() <= a.len() {
                        let keep = &a[..a.len() - b.len()];
                        let mut y: u32 = format!("{}{}", keep, b).parse().unwrap_or(0);
                        let a_n: u32 = a.parse().unwrap_or(0);
                        if y < a_n {
                            y += 10u32.pow(b.len() as u32);
                        }
                        toks.push(y.to_string());
                        dual = true;
                        continue;
                    }
                }
                toks.push(t.to_string());
            }
        }
    }
    let out = toks.join(" ");
    if !valid7(&out) {
        return (String::new(), Some(orig.to_string()));
    }
    if dual && phrase.is_none() {
        phrase = Some(orig.to_string());
    }
    if phrase.as_deref() == Some("") {
        phrase = None;
    }
    (out, phrase)
}

/// A GEDCOM 7 date (and its PHRASE) as 5.5.1 writes it. None: keep the PHRASE as it is.
fn date_from7(v: &str, phrase: Option<&str>) -> (String, bool) {
    // What Gilded Roots itself wrote from a 5.5.1 date comes back exactly.
    if let Some(p) = phrase {
        if !v.trim().is_empty() && date_to7(p) == (v.trim().to_string(), Some(p.to_string())) {
            return (p.to_string(), true);
        }
    }
    let mut toks: Vec<String> = Vec::new();
    for t in v.split_whitespace() {
        match t {
            "GREGORIAN" => {}
            "JULIAN" => toks.push("@#DJULIAN@".into()),
            "HEBREW" => toks.push("@#DHEBREW@".into()),
            "FRENCH_R" => toks.push("@#DFRENCH R@".into()),
            "BCE" => toks.push("B.C.".into()),
            x if x.starts_with('_') => toks.push(format!("@#D{}@", x)),
            x => toks.push(x.to_string()),
        }
    }
    let s = toks.join(" ");
    match phrase {
        None => (s, false),
        Some(p) if s.is_empty() => (format!("({})", p), true),
        Some(p) if !s.split_whitespace().any(|t| ["ABT", "CAL", "EST", "BEF", "AFT", "BET", "FROM", "TO"].contains(&t)) => (format!("INT {} ({})", s, p), true),
        Some(_) => (s, false),
    }
}

/// An age as GEDCOM 7 writes it.
fn age_to7(v: &str) -> (String, Option<String>) {
    let t = v.trim();
    let u = t.to_uppercase();
    match u.as_str() {
        "" => return (String::new(), None),
        "CHILD" => return ("< 8y".into(), Some(t.to_string())),
        "INFANT" => return ("< 1y".into(), Some(t.to_string())),
        "STILLBORN" => return ("0y".into(), Some(t.to_string())),
        _ => {}
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        // the PHRASE keeps how it was written, so it reads back the same
        return (format!("{}y", t), Some(t.to_string()));
    }
    let ok = t.split_whitespace().enumerate().all(|(i, w)| (i == 0 && (w == "<" || w == ">")) || (w.len() > 1 && w[..w.len() - 1].chars().all(|c| c.is_ascii_digit()) && "ymwd".contains(&w[w.len() - 1..])));
    if ok {
        (t.to_string(), None)
    } else {
        (String::new(), Some(t.to_string()))
    }
}

/* ------------------------------------------------------------------ */
/* 5.5.1 to GEDCOM 7                                                   */
/* ------------------------------------------------------------------ */

const RENAMED: &[(&str, &str)] = &[("AFN", "_AFN"), ("RFN", "_RFN"), ("RIN", "_RIN"), ("ROMN", "_ROMN"), ("FONE", "_FONE")];

struct To7 {
    used: HashSet<String>,
    new_records: Vec<Node>,
    obje_by_body: HashMap<String, String>,
    sour_by_text: HashMap<String, String>,
    repo_by_text: HashMap<String, String>,
    next: u32,
}

impl To7 {
    fn fresh(&mut self, letter: &str) -> String {
        loop {
            self.next += 1;
            let x = format!("{}{}", letter, self.next);
            if self.used.insert(x.clone()) {
                return x;
            }
        }
    }
}

/// Make an inline OBJE into a GEDCOM 7 OBJE record, and point to it.
fn obje_record(cx: &mut To7, n: &mut Node) {
    let mut kids = std::mem::take(&mut n.kids);
    // 5.5 style: FORM and TITL beside FILE go under it
    let form = kids.iter().position(|k| k.tag == "FORM").map(|i| kids.remove(i));
    let titl = kids.iter().position(|k| k.tag == "TITL").map(|i| kids.remove(i));
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
        // no file at all: GEDCOM 7 needs one, so this becomes a note on the person
        let what = titl.map(|t| t.val).unwrap_or_default();
        n.tag = "NOTE".into();
        n.val = if what.is_empty() { "A picture with no file.".to_string() } else { format!("A picture with no file: {}", what) };
        n.kids = Vec::new();
        return;
    }
    let body = write_all(&[Node { xref: None, tag: "OBJE".into(), val: String::new(), kids: kids.clone() }], false);
    let x = match cx.obje_by_body.get(&body) {
        Some(x) => x.clone(),
        None => {
            let x = cx.fresh("M");
            cx.obje_by_body.insert(body, x.clone());
            cx.new_records.push(Node { xref: Some(x.clone()), tag: "OBJE".into(), val: String::new(), kids });
            x
        }
    };
    n.val = format!("@{}@", x);
}

fn file_to7(n: &mut Node) {
    n.val = path_to_url(&n.val);
    let ext = n.val.rsplit('/').next().and_then(|f| f.rsplit_once('.')).map(|(_, e)| e.to_string()).unwrap_or_default();
    match n.kids.iter_mut().find(|k| k.tag == "FORM") {
        Some(f) => {
            f.val = mime_of(if f.val.trim().is_empty() { &ext } else { &f.val });
            for k in f.kids.iter_mut() {
                if k.tag == "TYPE" || k.tag == "MEDI" {
                    k.tag = "MEDI".into();
                    let (v, p) = word_to7(&k.val, MEDI, &no_guess);
                    k.val = v;
                    set_phrase(k, p);
                }
            }
        }
        None => n.kids.insert(0, node("FORM", &mime_of(&ext))),
    }
}

fn walk_to7(cx: &mut To7, n: &mut Node, parent: &str, depth: u32, in_ext: bool) {
    let tag = n.tag.clone();
    // inside an extension (_TODO and the like) dates and words are the program's own: left as they are
    let ext = in_ext || tag.starts_with('_');
    if depth > 0 {
        if let Some((_, to)) = RENAMED.iter().find(|(a, _)| *a == tag) {
            n.tag = to.to_string();
        }
    }
    match tag.as_str() {
        "NOTE" if depth > 0 && is_ptr(&n.val) => n.tag = "SNOTE".into(),
        "OBJE" if depth > 0 && !is_ptr(&n.val) => obje_record(cx, n),
        "SOUR" if depth > 0 && !is_ptr(&n.val) => {
            let text = n.val.trim().to_string();
            if text.is_empty() {
                n.val = "@VOID@".into();
            } else {
                let x = match cx.sour_by_text.get(&text) {
                    Some(x) => x.clone(),
                    None => {
                        let x = cx.fresh("S");
                        cx.sour_by_text.insert(text.clone(), x.clone());
                        let mut rec = Node { xref: Some(x.clone()), tag: "SOUR".into(), val: String::new(), kids: vec![node("TITL", &text)] };
                        // text right under an inline source is the source's text
                        let mut keep = Vec::new();
                        for k in std::mem::take(&mut n.kids) {
                            if k.tag == "TEXT" {
                                rec.kids.push(k);
                            } else {
                                keep.push(k);
                            }
                        }
                        n.kids = keep;
                        cx.new_records.push(rec);
                        x
                    }
                };
                n.val = format!("@{}@", x);
            }
        }
        "REPO" if depth > 0 && !is_ptr(&n.val) => {
            let text = n.val.trim().to_string();
            if text.is_empty() {
                n.val = "@VOID@".into();
            } else {
                let x = match cx.repo_by_text.get(&text) {
                    Some(x) => x.clone(),
                    None => {
                        let x = cx.fresh("R");
                        cx.repo_by_text.insert(text.clone(), x.clone());
                        cx.new_records.push(Node { xref: Some(x.clone()), tag: "REPO".into(), val: String::new(), kids: vec![node("NAME", &text)] });
                        x
                    }
                };
                n.val = format!("@{}@", x);
            }
        }
        "FILE" if parent == "OBJE" => file_to7(n),
        "DATE" if parent != "HEAD" && !ext && !n.val.is_empty() => {
            let (v, p) = date_to7(&n.val);
            n.val = v;
            set_phrase(n, p);
        }
        "AGE" if !ext => {
            let (v, p) = age_to7(&n.val);
            n.val = v;
            set_phrase(n, p);
        }
        "PEDI" => {
            let (v, p) = word_to7(&n.val, PEDI, &no_guess);
            n.val = v;
            set_phrase(n, p);
        }
        "STAT" if parent == "FAMC" => {
            let (v, p) = word_to7(&n.val, FAMC_STAT, &no_guess);
            n.val = v;
            set_phrase(n, p);
        }
        "TYPE" if parent == "NAME" => {
            let (v, p) = word_to7(&n.val, NAME_TYPE, &no_guess);
            n.val = v;
            set_phrase(n, p);
        }
        "RELA" if parent == "ASSO" => {
            n.tag = "ROLE".into();
            let (v, p) = word_to7(&n.val, ROLE, &guess_role);
            n.val = v;
            set_phrase(n, p);
        }
        "MEDI" | "TYPE" if parent == "CALN" => {
            n.tag = "MEDI".into();
            let (v, p) = word_to7(&n.val, MEDI, &no_guess);
            n.val = v;
            set_phrase(n, p);
        }
        "RESN" => n.val = n.val.split(',').map(|w| w.trim().to_uppercase()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(", "),
        "LANG" => match LANGS.iter().find(|(_, name)| name.eq_ignore_ascii_case(n.val.trim())) {
            Some((code, _)) => n.val = code.to_string(),
            None => n.tag = "_LANG".into(),
        },
        _ => {}
    }
    let here = n.tag.clone();
    // ASSO needs a ROLE in GEDCOM 7
    if here == "ASSO" && !n.kids.iter().any(|k| k.tag == "RELA" || k.tag == "ROLE") {
        n.kids.insert(0, node("ROLE", "OTHER"));
    }
    for k in n.kids.iter_mut() {
        walk_to7(cx, k, &here, depth + 1, ext);
    }
}

/// Turn a GEDCOM 5.5.1 file written by Gilded Roots (or any 5.5.1 file) into GEDCOM 7.0.
pub fn to_v7(text: &str) -> String {
    let mut w = Vec::new();
    let mut roots = build_tree(text, &mut w);
    // record names GEDCOM 7 can use: capital letters, digits and _
    let mut used: HashSet<String> = HashSet::new();
    let mut rename: HashMap<String, String> = HashMap::new();
    for r in &roots {
        if let Some(x) = &r.xref {
            used.insert(x.clone());
        }
    }
    let mut cx = To7 { used: used.clone(), new_records: Vec::new(), obje_by_body: HashMap::new(), sour_by_text: HashMap::new(), repo_by_text: HashMap::new(), next: 0 };
    for r in &roots {
        if let Some(x) = &r.xref {
            if x == "VOID" || x.is_empty() || !x.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
                let base: String = x.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect::<String>().to_uppercase();
                let mut cand = if base.is_empty() || base == "VOID" { cx.fresh("X") } else { base };
                if cand != x.as_str() && cx.used.contains(&cand) {
                    cand = cx.fresh("X");
                }
                cx.used.insert(cand.clone());
                rename.insert(x.clone(), cand);
            }
        }
    }
    if !rename.is_empty() {
        fn fix(n: &mut Node, m: &HashMap<String, String>) {
            if let Some(x) = &n.xref {
                if let Some(y) = m.get(x) {
                    n.xref = Some(y.clone());
                }
            }
            if is_ptr(&n.val) {
                if let Some(y) = m.get(n.val.trim_matches('@')) {
                    n.val = format!("@{}@", y);
                }
            }
            for k in n.kids.iter_mut() {
                fix(k, m);
            }
        }
        for r in roots.iter_mut() {
            fix(r, &rename);
        }
    }
    let mut out: Vec<Node> = Vec::new();
    for mut r in roots.drain(..) {
        match r.tag.as_str() {
            "HEAD" => {
                let mut h = Node { xref: None, tag: "HEAD".into(), val: String::new(), kids: Vec::new() };
                h.kids.push(Node { xref: None, tag: "GEDC".into(), val: String::new(), kids: vec![node("VERS", "7.0")] });
                for k in r.kids.into_iter() {
                    match k.tag.as_str() {
                        "GEDC" | "CHAR" | "FILE" | DONE_MARK => {}
                        "LANG" => {
                            if let Some((code, _)) = LANGS.iter().find(|(_, n)| n.eq_ignore_ascii_case(k.val.trim())) {
                                h.kids.push(node("LANG", code));
                            }
                        }
                        _ => h.kids.push(k),
                    }
                }
                out.push(h);
            }
            "SUBN" | "TRLR" => {}
            "NOTE" => {
                r.tag = "SNOTE".into();
                for k in r.kids.iter_mut() {
                    walk_to7(&mut cx, k, "SNOTE", 1, false);
                }
                out.push(r);
            }
            _ => {
                let tag = r.tag.clone();
                for k in r.kids.iter_mut() {
                    walk_to7(&mut cx, k, &tag, 1, false);
                }
                // OBJE records: their files
                out.push(r);
            }
        }
    }
    // records made along the way (pictures, sources): their own lines are turned too
    while !cx.new_records.is_empty() {
        let mut made: Vec<Node> = cx.new_records.drain(..).collect();
        for r in made.iter_mut() {
            let tag = r.tag.clone();
            for k in r.kids.iter_mut() {
                walk_to7(&mut cx, k, &tag, 1, false);
            }
        }
        out.extend(made);
    }
    out.push(node("TRLR", ""));
    if !out.first().map_or(false, |h| h.tag == "HEAD") {
        out.insert(0, Node { xref: None, tag: "HEAD".into(), val: String::new(), kids: vec![Node { xref: None, tag: "GEDC".into(), val: String::new(), kids: vec![node("VERS", "7.0")] }] });
    }
    write_all(&out, true)
}

/* ------------------------------------------------------------------ */
/* GEDCOM 7 to 5.5.1                                                   */
/* ------------------------------------------------------------------ */

fn walk_from7(n: &mut Node, parent: &str, depth: u32, in_ext: bool) {
    let ext = in_ext || n.tag.starts_with('_');
    // "@@" at the start of a continued line is one "@"
    if n.val.contains('\n') {
        let mut lines: Vec<String> = Vec::new();
        for (i, l) in n.val.split('\n').enumerate() {
            if i > 0 && l.starts_with("@@") {
                lines.push(l[1..].to_string());
            } else {
                lines.push(l.to_string());
            }
        }
        n.val = lines.join("\n");
    }
    if depth > 0 {
        if let Some((back, _)) = RENAMED.iter().find(|(_, b)| *b == n.tag) {
            n.tag = back.to_string();
        }
    }
    match n.tag.as_str() {
        "SNOTE" => n.tag = "NOTE".into(),
        "FILE" if parent == "OBJE" => {
            n.val = url_to_path(&n.val);
            for f in n.kids.iter_mut().filter(|k| k.tag == "FORM") {
                f.val = form_of(&f.val);
                for m in f.kids.iter_mut().filter(|k| k.tag == "MEDI") {
                    let p = take_phrase(m);
                    m.val = word_from7(&m.val, p.as_deref(), MEDI);
                    m.tag = "TYPE".into();
                }
            }
        }
        "DATE" if parent != "HEAD" && !ext => {
            let p = phrase_of(n);
            let (v, used) = date_from7(&n.val, p.as_deref());
            n.val = v;
            if used {
                take_phrase(n);
            }
        }
        "AGE" if !ext => {
            let p = take_phrase(n);
            let u = p.as_deref().map(|s| s.to_uppercase()).unwrap_or_default();
            n.val = match p {
                Some(p) if n.val.trim().is_empty() || ["CHILD", "INFANT", "STILLBORN"].contains(&u.as_str()) || age_to7(&p) == (n.val.trim().to_string(), Some(p.clone())) => p,
                Some(p) => {
                    // a phrase we can't fold back in stays beside the age
                    n.kids.insert(0, node("PHRASE", &p));
                    n.val.clone()
                }
                None => n.val.clone(),
            };
        }
        "PEDI" => {
            let p = take_phrase(n);
            n.val = word_from7(&n.val, p.as_deref(), PEDI);
        }
        "STAT" if parent == "FAMC" => {
            let p = take_phrase(n);
            n.val = word_from7(&n.val, p.as_deref(), FAMC_STAT);
        }
        "TYPE" if parent == "NAME" => {
            let p = take_phrase(n);
            n.val = word_from7(&n.val, p.as_deref(), NAME_TYPE);
        }
        "ROLE" if parent == "ASSO" => {
            let p = take_phrase(n);
            n.tag = "RELA".into();
            n.val = if n.val.trim() == "OTHER" && p.is_none() { String::new() } else { word_from7(&n.val, p.as_deref(), ROLE) };
        }
        "MEDI" if parent == "CALN" => {
            let p = take_phrase(n);
            n.val = word_from7(&n.val, p.as_deref(), MEDI);
        }
        "RESN" => n.val = n.val.split(',').map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(", "),
        "LANG" => {
            if let Some((_, name)) = LANGS.iter().find(|(code, _)| code.eq_ignore_ascii_case(n.val.trim().split('-').next().unwrap_or(""))) {
                n.val = name.to_string();
            }
        }
        "_LANG" => n.tag = "LANG".into(),
        "UID" => n.tag = "_UID".into(),
        _ => {}
    }
    // links to nothing
    let here = n.tag.clone();
    n.kids.retain(|k| !(k.val == "@VOID@" && matches!(k.tag.as_str(), "FAMC" | "FAMS" | "HUSB" | "WIFE" | "CHIL" | "ASSO" | "OBJE" | "NOTE" | "SNOTE" | "SUBM" | "ALIA" | "ANCI" | "DESI")));
    for k in n.kids.iter_mut() {
        if k.tag == "SOUR" && k.val == "@VOID@" {
            k.val.clear();
        }
        if k.tag == "REPO" && k.val == "@VOID@" {
            k.val.clear();
        }
        walk_from7(k, &here, depth + 1, ext);
    }
    // an empty RELA (a role of OTHER with nothing said) is left out
    n.kids.retain(|k| !(k.tag == "RELA" && k.val.is_empty() && k.kids.is_empty()));
}

/// Turn a GEDCOM 7 file into the GEDCOM 5.5.1 shape Gilded Roots reads.
pub fn to_551(text: &str) -> String {
    let mut w = Vec::new();
    let mut roots = build_tree(text, &mut w);
    for r in roots.iter_mut() {
        if r.tag == "HEAD" {
            r.kids.retain(|k| k.tag != "SCHMA");
            r.kids.push(node(DONE_MARK, "Y"));
            continue;
        }
        if r.tag == "SNOTE" {
            r.tag = "NOTE".into();
        }
        let tag = r.tag.clone();
        walk_from7(r, &tag, 0, false);
    }
    write_all(&roots, false)
}

/* ------------------------------------------------------------------ */
/* GEDZIP                                                              */
/* ------------------------------------------------------------------ */

/// True when the bytes are a zip file with a GEDCOM file in it (a GEDZIP, or a zip someone made).
pub fn ged_in_zip(b: &[u8]) -> Option<String> {
    if !b.starts_with(b"PK\x03\x04") {
        return None;
    }
    let z = zip::ZipArchive::new(std::io::Cursor::new(b)).ok()?;
    let names: Vec<String> = z.file_names().filter_map(|s| s.ok().map(|s| s.to_string())).collect();
    names
        .iter()
        .find(|n| n.eq_ignore_ascii_case("gedcom.ged"))
        .or_else(|| names.iter().find(|n| !n.contains('/') && n.to_lowercase().ends_with(".ged")))
        .or_else(|| names.iter().find(|n| n.to_lowercase().ends_with(".ged")))
        .cloned()
}

/// Unpack a GEDZIP (or a zip with a GEDCOM in it) into `dir`. Only the GEDCOM file and photos,
/// documents and recordings are unpacked, and only inside `dir`. Returns the GEDCOM file's path.
pub fn unpack(b: &[u8], dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    use std::io::Read;
    let ged = ged_in_zip(b).ok_or_else(|| "This zip file has no GEDCOM (.ged) file in it.".to_string())?;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(b)).map_err(|_| "This zip file could not be opened.".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut ged_path = None;
    for i in 0..z.len() {
        let Ok(mut f) = z.by_index(i) else { continue };
        if f.is_dir() {
            continue;
        }
        let Some(rel) = f.enclosed_name() else { continue };
        let name = f.name().map(|s| s.to_string()).unwrap_or_default();
        let base = rel.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if name != ged && !crate::is_media_name(&base) {
            continue;
        }
        let dst = dir.join(&rel);
        if let Some(p) = dst.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let mut data = Vec::new();
        if f.read_to_end(&mut data).is_err() {
            continue;
        }
        if std::fs::write(&dst, &data).is_ok() && name == ged {
            ged_path = Some(dst);
        }
    }
    ged_path.ok_or_else(|| "The GEDCOM file in this zip could not be unpacked.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_both_ways() {
        for (a, b, p) in [
            ("12 MAR 1850", "12 MAR 1850", None),
            ("ABT 1850", "ABT 1850", None),
            ("BET 1850 AND 1860", "BET 1850 AND 1860", None),
            ("@#DJULIAN@ 12 JAN 1700", "JULIAN 12 JAN 1700", None),
            ("500 B.C.", "500 BCE", None),
            ("INT 1850 (about the time of the war)", "1850", Some("about the time of the war")),
            ("(Spring of 1850)", "", Some("Spring of 1850")),
            ("1850/51", "1851", Some("1850/51")),
            ("1699/00", "1700", Some("1699/00")),
            ("sometime in the 1800s", "", Some("sometime in the 1800s")),
        ] {
            let (v, ph) = date_to7(a);
            assert_eq!((v.as_str(), ph.as_deref()), (b, p), "{}", a);
            let (back, used) = date_from7(&v, ph.as_deref());
            assert!(used || ph.is_none());
            let want = if a.starts_with("sometime") { format!("({})", a) } else { a.to_string() };
            assert_eq!(back, want, "{} back", a);
        }
        // from other programs
        assert_eq!(date_from7("GREGORIAN 1 JAN 1900", None).0, "1 JAN 1900");
        assert_eq!(date_from7("BEF 1900", Some("before the war")).0, "BEF 1900");
        assert_eq!(date_from7("1900", Some("New Year's Day")).0, "INT 1900 (New Year's Day)");
    }

    #[test]
    fn file_names_as_urls() {
        for (p, u) in [
            ("media/My Photo.jpg", "media/My%20Photo.jpg"),
            ("C:\\Users\\Ann\\Pictures\\a b.jpg", "file:///C:/Users/Ann/Pictures/a%20b.jpg"),
            ("/home/ann/x.png", "file:///home/ann/x.png"),
            ("https://example.com/a b.jpg", "https://example.com/a b.jpg"),
            ("photos/Jöhann.jpg", "photos/J%C3%B6hann.jpg"),
        ] {
            assert_eq!(path_to_url(p), u);
        }
        assert_eq!(url_to_path("media/My%20Photo.jpg"), "media/My Photo.jpg");
        assert_eq!(url_to_path("file:///C:/Users/a%20b.jpg"), "C:/Users/a b.jpg");
        assert_eq!(url_to_path("file:///home/ann/x.png"), "/home/ann/x.png");
        assert_eq!(url_to_path("photos/J%C3%B6hann.jpg"), "photos/Jöhann.jpg");
        assert_eq!(url_to_path("100%"), "100%");
    }

    #[test]
    fn a_gedcom7_file_reads_in() {
        let t = "0 HEAD\n1 GEDC\n2 VERS 7.0\n1 SCHMA\n2 TAG _SKYPEID http://xmlns.com/foaf/0.1/skypeID\n\
0 @I1@ INDI\n1 NAME John /Smith/\n2 TYPE OTHER\n3 PHRASE Pen name\n1 SEX M\n1 BIRT\n2 DATE JULIAN 1 JAN 1700\n2 SOUR @VOID@\n3 PAGE Family Bible\n\
1 DEAT\n2 DATE\n3 PHRASE During the great flood\n1 FAMC @F1@\n2 PEDI OTHER\n3 PHRASE Raised by aunt\n1 SNOTE @N1@\n1 OBJE @O1@\n1 ASSO @I2@\n2 ROLE GODP\n1 UID 3a1e7f2c-3d5b-4e5a-9c4b-1f2e3d4c5b6a\n\
1 NOTE A note\n2 CONT @@ starts with an at sign\n\
0 @I2@ INDI\n1 NAME Ann /Lee/\n1 FAMS @VOID@\n0 @F1@ FAM\n1 CHIL @I1@\n1 HUSB @VOID@\n\
0 @N1@ SNOTE Shared note text\n0 @O1@ OBJE\n1 FILE photos/John%20Smith.jpg\n2 FORM image/jpeg\n3 MEDI PHOTO\n2 TITL John's portrait\n0 TRLR\n";
        assert!(is_v7(t));
        let s = to_551(t);
        assert!(!is_v7(&s), "turned once only");
        let (db, rep) = crate::parse_ged_report(&s);
        assert_eq!(rep.gedcom_version, "7.0");
        let p = db.people.values().find(|p| p.given == "John").unwrap();
        assert!(p.alt_names.is_empty() || p.alt_names.iter().all(|a| a.kind != "OTHER"));
        assert_eq!(p.bd, "@#DJULIAN@ 1 JAN 1700");
        assert!(p.dd.contains("During the great flood"));
        assert!(p.notes.contains("\n@ starts with an at sign"), "{:?}", p.notes);
        assert_eq!(p.media.len(), 1);
        assert_eq!(p.media[0].path, "photos/John Smith.jpg");
        assert_eq!(p.media[0].caption, "John's portrait");
        assert!(s.contains("2 RELA godparent"), "{}", s);
        assert!(s.contains("2 PEDI Raised by aunt"));
        assert!(s.contains("1 _UID 3a1e7f2c"));
        assert_eq!(db.notes.len(), 1);
        assert_eq!(db.fams.len(), 1);
        assert!(!s.contains("@VOID@"));
    }

    /// Everything the 5.5.1 tests bring in also survives being written as GEDCOM 7 and read back.
    #[test]
    fn writing_gedcom7_and_reading_it_back_loses_nothing() {
        let sample = crate::to_ged(&crate::sample::sample_family());
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME John /Smith/\n2 TYPE aka\n1 NAME Johnny /Smith/\n2 TYPE stage name\n1 SEX M\n\
1 BIRT\n2 DATE 1850/51\n2 PLAC Boston\n2 SOUR Family Bible\n3 PAGE p. 2\n2 AGE 0\n1 DEAT\n2 DATE (in the war)\n2 AGE CHILD\n\
1 CHR\n2 DATE INT 1851 (Easter)\n1 BURI\n2 DATE @#DJULIAN@ 3 MAR 1700\n1 RESN locked\n1 RIN 77\n1 AFN 1ABC-DE\n\
1 ASSO @I2@\n2 RELA godfather\n1 ASSO @I2@\n2 RELA lodger\n1 FAMC @F1@\n2 PEDI adopted\n2 STAT proven\n\
1 OBJE\n2 FILE media/A photo.jpg\n3 FORM jpg\n4 TYPE photo\n3 TITL A photo\n1 NOTE @N1@\n1 NOTE line one\n2 CONT @line two\n\
0 @I2@ INDI\n1 NAME Bob /Smith/\n0 @F1@ FAM\n1 HUSB @I2@\n1 CHIL @I1@\n0 @N1@ NOTE The shared note\n0 TRLR\n";
        for text in [t, sample.as_str()] {
            let (db, _) = crate::parse_ged_report(text);
            let out = crate::to_ged(&db);
            let v7 = to_v7(&out);
            assert!(is_v7(&v7));
            assert!(!v7.contains(" CONC "), "no CONC in GEDCOM 7");
            assert!(!v7.contains("1 CHAR"));
            assert!(!v7.lines().any(|l| l.starts_with("1 OBJE") && !l.contains('@')), "pictures are records");
            assert!(!v7.lines().any(|l| !l.starts_with("0 ") && (l.ends_with(" SOUR") || l.contains(" SOUR Family"))), "sources are records");
            let back = to_551(&v7);
            let (db2, _) = crate::parse_ged_report(&back);
            assert_eq!(db.people.len(), db2.people.len());
            assert_eq!(db.fams.len(), db2.fams.len());
            // compare with the same tree where the inline sources were made into records
            let (db_rec, _) = crate::parse_ged_report(&to_551(&to_v7(&out)));
            let lost = crate::gedcom::loss_check(&crate::to_ged(&db_rec), &crate::to_ged(&db2));
            assert!(lost.is_empty(), "{:#?}", lost);
            // and nothing but the sources' shape changed from the original
            let lost = crate::gedcom::loss_check(&out, &crate::to_ged(&db2));
            assert!(lost.iter().all(|l| l.contains("SOUR")), "{:#?}\n{}", lost, v7);
        }
        let (db, _) = crate::parse_ged_report(t);
        let v7 = to_v7(&crate::to_ged(&db));
        assert!(v7.contains("2 DATE 1851\n3 PHRASE 1850/51"), "{}", v7);
        assert!(v7.contains("2 ROLE GODP\n3 PHRASE godfather"));
        assert!(v7.contains("2 ROLE OTHER\n3 PHRASE lodger"));
        assert!(v7.contains("2 PEDI ADOPTED\n"));
        assert!(v7.contains("1 RESN LOCKED"));
        assert!(v7.contains("1 _RIN 77"));
        assert!(v7.contains("2 FORM image/jpeg\n3 MEDI PHOTO"), "{}", v7);
        assert!(v7.contains("1 FILE media/A%20photo.jpg") || v7.contains("FILE media/A%20photo.jpg"));
        assert!(v7.contains(" SNOTE The shared note"));
        assert!(v7.contains("1 SNOTE @"));
        assert!(v7.contains("2 CONT @@line two"));
        let back = to_551(&v7);
        let (db2, _) = crate::parse_ged_report(&back);
        let j = db2.people.values().find(|p| p.given == "John").unwrap();
        let j0 = db.people.values().find(|p| p.given == "John").unwrap();
        assert_eq!((&j.bd, &j.chd, &j.dd, &j.bud), (&j0.bd, &j0.chd, &j0.dd, &j0.bud));
        assert!(j.dd.contains("in the war"));
        assert!(j.notes.contains("@line two"));
    }

    #[test]
    fn gedzip_unpacks_only_the_tree_and_pictures() {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("gedcom.ged", o).unwrap();
            z.write_all(b"0 HEAD\n1 GEDC\n2 VERS 7.0\n0 @I1@ INDI\n1 NAME A /B/\n1 OBJE @O1@\n0 @O1@ OBJE\n1 FILE media/a%20b.jpg\n2 FORM image/jpeg\n0 TRLR\n").unwrap();
            z.start_file("media/a b.jpg", o).unwrap();
            z.write_all(b"\xff\xd8\xff").unwrap();
            z.start_file("run-me.sh", o).unwrap();
            z.write_all(b"echo hi").unwrap();
            z.finish().unwrap();
        }
        assert_eq!(ged_in_zip(&buf).as_deref(), Some("gedcom.ged"));
        let dir = std::env::temp_dir().join(format!("gr-gdz-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let g = unpack(&buf, &dir).unwrap();
        assert!(g.ends_with("gedcom.ged"));
        assert!(dir.join("media/a b.jpg").is_file());
        assert!(!dir.join("run-me.sh").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
