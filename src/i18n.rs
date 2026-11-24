//! Languages. Every piece of text on screen is written in English in the code, and that English
//! is also the key used to look up its translation. `tr("Save")` gives the translation of "Save"
//! in the chosen language, or "Save" itself when there is none, so anything not yet translated
//! simply stays in English. In English nothing is looked up at all.
//!
//! The translations are in `lang/<code>.json`, one file per language, each a list of
//! `"English": "translation"` pairs. `tools/lang_extract.py` collects every English text into
//! `lang/en.json` (the list to translate) and shows how much of each language is done.
//!
//! Text with numbers or names in it ("Found {n} people") goes through `trf`: the translation
//! keeps the same `{...}` parts, in any order the language needs, and they are filled in.

use std::collections::HashMap;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock};

/// Every language the program knows, as (code, name in that language, English name, finished).
/// Only finished ones are offered in View > Language; the rest are kept for a later version
/// (their translation files are in lang/ and can be filled in at any time).
pub(crate) const LANGUAGES: &[(&str, &str, &str, bool)] = &[
    ("en", "English", "English", true),
    ("cs", "Čeština", "Czech", false),
    ("cy", "Cymraeg", "Welsh", false),
    ("da", "Dansk", "Danish", false),
    ("de", "Deutsch", "German", true),
    ("es", "Español", "Spanish", true),
    ("fr", "Français", "French", true),
    ("it", "Italiano", "Italian", true),
    ("hu", "Magyar", "Hungarian", false),
    ("nl", "Nederlands", "Dutch", true),
    ("nb", "Norsk", "Norwegian", true),
    ("pl", "Polski", "Polish", false),
    ("pt", "Português", "Portuguese (Brazil)", true),
    ("fi", "Suomi", "Finnish", false),
    ("sv", "Svenska", "Swedish", true),
    ("tl", "Tagalog", "Tagalog", false),
    ("ru", "Русский", "Russian", false),
    ("uk", "Українська", "Ukrainian", false),
    ("ja", "日本語", "Japanese", false),
    ("zh", "中文（简体）", "Chinese (Simplified)", false),
    ("ko", "한국어", "Korean", false),
];

/// The languages shown in View > Language.
pub(crate) fn offered() -> impl Iterator<Item = &'static (&'static str, &'static str, &'static str, bool)> {
    LANGUAGES.iter().filter(|l| l.3)
}

/// The translation files, built into the program.
fn file_for(code: &str) -> Option<&'static str> {
    Some(match code {
        "cs" => include_str!("../lang/cs.json"),
        "cy" => include_str!("../lang/cy.json"),
        "da" => include_str!("../lang/da.json"),
        "de" => include_str!("../lang/de.json"),
        "es" => include_str!("../lang/es.json"),
        "fr" => include_str!("../lang/fr.json"),
        "it" => include_str!("../lang/it.json"),
        "hu" => include_str!("../lang/hu.json"),
        "nl" => include_str!("../lang/nl.json"),
        "nb" => include_str!("../lang/nb.json"),
        "pl" => include_str!("../lang/pl.json"),
        "pt" => include_str!("../lang/pt.json"),
        "fi" => include_str!("../lang/fi.json"),
        "sv" => include_str!("../lang/sv.json"),
        "tl" => include_str!("../lang/tl.json"),
        "ru" => include_str!("../lang/ru.json"),
        "uk" => include_str!("../lang/uk.json"),
        "ja" => include_str!("../lang/ja.json"),
        "zh" => include_str!("../lang/zh.json"),
        "ko" => include_str!("../lang/ko.json"),
        _ => return None,
    })
}

type Table = HashMap<&'static str, &'static str>;

/// The table for the chosen language; null for English.
static TABLE: AtomicPtr<Table> = AtomicPtr::new(std::ptr::null_mut());
/// Tables already loaded, so switching back and forth does not load a file twice.
static LOADED: OnceLock<Mutex<HashMap<String, &'static Table>>> = OnceLock::new();
static CODE: Mutex<String> = Mutex::new(String::new());

fn table() -> Option<&'static Table> {
    let p = TABLE.load(Ordering::Acquire);
    // Safety: tables are leaked when loaded and never freed, so the pointer stays valid.
    if p.is_null() { None } else { Some(unsafe { &*p }) }
}

/// Reads a translation file. Empty translations and the file's own notes ("_" keys) are skipped.
fn parse(json: &str) -> Table {
    let map: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
    map.into_iter()
        .filter(|(k, v)| !k.starts_with('_') && !v.trim().is_empty())
        .map(|(k, v)| (&*Box::leak(k.into_boxed_str()), &*Box::leak(v.into_boxed_str())))
        .collect()
}

/// Chooses the language by its code ("" or "en" for English). Unknown codes give English.
pub(crate) fn set_language(code: &str) {
    if let Ok(mut c) = CODE.lock() {
        *c = code.to_string();
    }
    let Some(json) = file_for(code) else {
        TABLE.store(std::ptr::null_mut(), Ordering::Release);
        return;
    };
    let loaded = LOADED.get_or_init(|| Mutex::new(HashMap::new()));
    let t: &'static Table = {
        let mut g = loaded.lock().unwrap_or_else(|e| e.into_inner());
        *g.entry(code.to_string()).or_insert_with(|| Box::leak(Box::new(parse(json))))
    };
    TABLE.store(if t.is_empty() { std::ptr::null_mut() } else { t as *const Table as *mut Table }, Ordering::Release);
}

/// The code of the chosen language ("" for English).
pub(crate) fn language() -> String {
    CODE.lock().map(|c| c.clone()).unwrap_or_default()
}

/// The translation of text that is not built into the program as such (a fact's name kept in
/// the tree, "Birth"), or the text itself.
pub(crate) fn tr_dyn(s: &str) -> String {
    match table() {
        None => s.to_string(),
        Some(t) => t.get(s).copied().unwrap_or(s).to_string(),
    }
}

/// The translation of a piece of text, or the text itself.
#[inline]
pub(crate) fn tr(s: &'static str) -> &'static str {
    match table() {
        None => s,
        Some(t) => t.get(s).copied().unwrap_or(s),
    }
}

/// The same as `tr`, for text that comes from a list or table in the code (a menu's entries, a
/// toolbar's buttons). Only text built into the program can be passed (`&'static str`), so
/// names and places typed by the user are never looked up.
#[inline]
pub(crate) fn trv(s: &'static str) -> &'static str {
    tr(s)
}

/// Text with parts filled in. `key` is the English with its `{...}` parts as written in the
/// code; `segs` is the English cut at those parts (one more than there are parts) and `parts`
/// what goes in each, already formatted. In English this simply joins them.
pub(crate) fn trf(key: &'static str, segs: &[&'static str], parts: &[String]) -> String {
    let join = || {
        let mut out = String::new();
        for (i, s) in segs.iter().enumerate() {
            out.push_str(s);
            if let Some(p) = parts.get(i) {
                out.push_str(p);
            }
        }
        out
    };
    let Some(t) = table() else { return join() };
    let Some(tpl) = t.get(key) else { return join() };
    fill(key, tpl, parts).unwrap_or_else(join)
}

/// The `{...}` parts of a template, in order, as (start, end) byte ranges. `{{` and `}}` are
/// left as they are; they stand for single braces.
fn holes(s: &str) -> Vec<(usize, usize)> {
    let b = s.as_bytes();
    let (mut i, mut out) = (0, Vec::new());
    while i < b.len() {
        if b[i] == b'{' {
            if b.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            if let Some(j) = s[i..].find('}') {
                out.push((i, i + j + 1));
                i += j + 1;
                continue;
            }
        } else if b[i] == b'}' && b.get(i + 1) == Some(&b'}') {
            i += 2;
            continue;
        }
        i += 1;
    }
    out
}

/// Puts the parts into a translation. Each `{...}` in the translation is matched to the same
/// `{...}` in the English; when one appears several times (`{}`), they are taken in order.
/// Gives None if the translation's parts do not match the English (it is then left in English).
fn fill(key: &str, tpl: &str, parts: &[String]) -> Option<String> {
    let kh: Vec<&str> = holes(key).into_iter().map(|(a, b)| &key[a..b]).collect();
    if kh.len() != parts.len() {
        return None;
    }
    let th = holes(tpl);
    if th.len() != kh.len() {
        return None;
    }
    let mut used = vec![false; kh.len()];
    let (mut out, mut last) = (String::new(), 0);
    for (a, b) in th {
        out.push_str(&unescape(&tpl[last..a]));
        let name = &tpl[a..b];
        let k = (0..kh.len()).find(|&k| !used[k] && kh[k] == name)?;
        used[k] = true;
        out.push_str(&parts[k]);
        last = b;
    }
    out.push_str(&unescape(&tpl[last..]));
    Some(out)
}

fn unescape(s: &str) -> String {
    s.replace("{{", "{").replace("}}", "}")
}

/// Japanese, Chinese and Korean letters are not in the program's own fonts, so they are taken
/// from the fonts that come with the computer, when it has them. They are added after the
/// program's fonts, so they are only used for those letters. Done once, when first needed.
static ASIAN_DONE: OnceLock<()> = OnceLock::new();

pub(crate) fn add_asian_fonts(ctx: &eframe::egui::Context) {
    if ASIAN_DONE.get().is_some() {
        return;
    }
    let _ = ASIAN_DONE.set(());
    use eframe::egui::FontData;
    // (path, index in a font collection). The first one found for each language is used.
    let groups: &[&[(&str, u32)]] = &[
        // Japanese
        &[
            ("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", 0),
            ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
            ("C:\\Windows\\Fonts\\YuGothM.ttc", 0),
            ("C:\\Windows\\Fonts\\meiryo.ttc", 0),
            ("C:\\Windows\\Fonts\\msgothic.ttc", 0),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
            ("/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc", 0),
        ],
        // Chinese
        &[
            ("/System/Library/Fonts/STHeiti Light.ttc", 0),
            ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
            ("C:\\Windows\\Fonts\\msyh.ttc", 0),
            ("C:\\Windows\\Fonts\\simsun.ttc", 0),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2),
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 2),
            ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
        ],
        // Korean
        &[
            ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0),
            ("C:\\Windows\\Fonts\\malgun.ttf", 0),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 1),
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 1),
            ("/usr/share/fonts/truetype/nanum/NanumGothic.ttf", 0),
        ],
    ];
    let mut found: Vec<(String, FontData)> = Vec::new();
    let mut added = Vec::new();
    let mut seen: Vec<(&str, u32)> = Vec::new();
    for (gi, group) in groups.iter().enumerate() {
        for &(path, index) in group.iter() {
            if seen.contains(&(path, index)) {
                break;
            }
            if let Ok(bytes) = std::fs::read(path) {
                let name = format!("system-asian-{gi}");
                let mut fd = FontData::from_owned(bytes);
                fd.index = index;
                found.push((name.clone(), fd));
                added.push(name);
                seen.push((path, index));
                break;
            }
        }
    }
    if added.is_empty() {
        return;
    }
    // Japanese first so its own way of drawing shared letters is used in Japanese.
    if language() == "zh" {
        added.rotate_left(1);
    } else if language() == "ko" {
        added.rotate_right(1);
    }
    // in that order, after the main fonts
    let ordered: Vec<(String, FontData)> = added.iter().filter_map(|n| found.iter().find(|f| &f.0 == n).cloned()).collect();
    crate::look::add_fallback_fonts(ordered);
    crate::look::install_fonts(ctx);
}

/// True every few seconds until the Asian fonts have been added (to look for such names then).
pub(crate) fn asian_check_due(now: f64) -> bool {
    use std::sync::atomic::AtomicU64;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    if ASIAN_DONE.get().is_some() {
        return false;
    }
    let next = f64::from_bits(NEXT.load(Ordering::Relaxed));
    if now < next {
        return false;
    }
    NEXT.store((now + 3.0).to_bits(), Ordering::Relaxed);
    true
}

/// True if the text has Japanese, Chinese or Korean letters.
pub(crate) fn has_asian(s: &str) -> bool {
    s.chars().any(|c| {
        let u = c as u32;
        (0x3040..=0x30FF).contains(&u) // kana
            || (0x3400..=0x9FFF).contains(&u) // Chinese characters
            || (0xAC00..=0xD7AF).contains(&u) // Korean
            || (0x1100..=0x11FF).contains(&u)
            || (0xF900..=0xFAFF).contains(&u)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_reorders_and_keeps_braces() {
        let parts = vec!["3".to_string(), "Smith".to_string()];
        assert_eq!(fill("{n} people named {name}", "{name}: {n} Personen", &parts).as_deref(), Some("Smith: 3 Personen"));
        let parts = vec!["1".to_string(), "2".to_string()];
        assert_eq!(fill("{} of {}", "{} von {} {{x}}", &parts).as_deref(), Some("1 von 2 {x}"));
        assert_eq!(fill("{} of {}", "{} von", &parts), None);
        assert_eq!(fill("{a} {b}", "{a} {c}", &["1".into(), "2".into()]), None);
    }

    #[test]
    fn english_joins() {
        assert_eq!(trf("Found {n} people", &["Found ", " people"], &["5".into()]), "Found 5 people");
    }
}
