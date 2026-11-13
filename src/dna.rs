//! The DNA tab: autosomal DNA results for people in the tree. Each test holds an ethnicity
//! estimate (regions and percentages, shown as a table, a bar chart and a map) and a list of
//! DNA matches (shared cM, segments, predicted relationship, which side, and a link to the
//! matching person in the tree). Match lists can be imported from the CSV files that testing
//! companies let you download, and ethnicity can be pasted from the company's web page.

use super::*;

/// One DNA test taken by one person.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DnaTest {
    /// The person in the tree who took the test.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub company: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kit: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<Region>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<DnaMatch>,
    /// 0 autosomal, 1 Y-DNA (father's line), 2 mtDNA (mother's line). Older trees have only 0.
    pub kind: u8,
    /// Y-DNA or mtDNA results.
    pub line: LineTest,
}

pub const KINDS: &[&str] = &["Autosomal", "Y-DNA", "mtDNA"];

/// Results of a Y-DNA or mtDNA test, which follow one line: father to father, or mother to mother.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct LineTest {
    /// "R-M269", "R-FGC11134", "H1a1"...
    #[serde(skip_serializing_if = "String::is_empty")]
    pub haplogroup: String,
    /// "Y-37", "Big Y-700", "HVR1", "Full sequence"...
    #[serde(skip_serializing_if = "String::is_empty")]
    pub level: String,
    /// Y-STR markers and their values, in the company's order ("DYS393", "13").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<(String, String)>,
    /// mtDNA differences from the reference sequence, as the company lists them.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mutations: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<LineMatch>,
}

/// A Y-DNA or mtDNA match.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct LineMatch {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Genetic distance: how many steps the two results differ by.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<u32>,
    /// Markers or region compared ("37", "111", "HVR1", "Full sequence").
    #[serde(skip_serializing_if = "String::is_empty")]
    pub compared: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub haplogroup: String,
    /// The match's earliest known ancestor on this line.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ancestor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

/// One line of an ethnicity estimate.
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Region {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub pct: f32,
}

/// One DNA match.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DnaMatch {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Shared centimorgans.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cm: Option<f32>,
    /// Percent of DNA shared, when the company gives that instead of (or as well as) cM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<u32>,
    /// Longest shared segment, cM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longest: Option<f32>,
    /// The company's own prediction ("1st-2nd cousin").
    #[serde(skip_serializing_if = "String::is_empty")]
    pub predicted: String,
    /// Maternal, paternal, both or blank.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub side: String,
    /// The matching person in the tree, once worked out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

pub const COMPANIES: &[&str] = &["AncestryDNA", "23andMe", "MyHeritage", "FamilyTreeDNA", "Living DNA", "GEDmatch", "YSEQ", "Nebula", "Other"];
pub const SIDES: &[&str] = &["", "Paternal", "Maternal", "Both"];

/* ---------- how much DNA relatives share ---------- */

/// Groups of relationships that share about the same amount of DNA, with the expected average.
///
/// The averages come straight from inheritance: a parent passes on half of the ~6,800 cM a
/// testing company compares, and each further step halves it again. Full siblings share about
/// three quarters of a parent's amount, because they can share both copies of a stretch.
/// Real amounts vary a lot around these figures, especially for distant cousins, who often show
/// more than the average (and many share nothing detectable at all).
pub const GROUPS: &[(f32, &str, &str)] = &[
    (3400.0, "Parent or child", "Parent or child"),
    (2550.0, "Full sibling", "Full sibling"),
    (1700.0, "Grandparent, aunt or uncle, half sibling", "Grandparent / aunt / half sib"),
    (850.0, "1st cousin, great-grandparent, great-aunt or uncle, half aunt or uncle", "1st cousin range"),
    (425.0, "1st cousin once removed, half 1st cousin", "1C1R range"),
    (212.5, "2nd cousin, 1st cousin twice removed, half 1C1R", "2nd cousin range"),
    (106.0, "2nd cousin once removed, half 2nd cousin", "2C1R range"),
    (53.0, "3rd cousin, 2nd cousin twice removed", "3rd cousin range"),
    (26.5, "3rd cousin once removed, half 3rd cousin", "3C1R range"),
    (13.0, "4th cousin or more distant", "4th cousin or further"),
];

/// The groups that best fit an amount of shared DNA, closest first (at most three).
pub fn likely_groups(cm: f32) -> Vec<usize> {
    if cm <= 0.0 {
        return Vec::new();
    }
    let mut v: Vec<(f32, usize)> = GROUPS.iter().enumerate().map(|(i, g)| ((cm / g.0).ln().abs(), i)).collect();
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    // keep the groups within a factor of about 2 of the best fit
    let best = v[0].0;
    v.into_iter().filter(|(d, _)| *d <= best + std::f32::consts::LN_2).take(3).map(|(_, i)| i).collect()
}

/// A short label for the most likely relationship.
pub fn likely(cm: Option<f32>) -> &'static str {
    match cm.and_then(|c| likely_groups(c).first().copied()) {
        Some(i) => GROUPS[i].2,
        None => "",
    }
}

/* ---------- reading text ---------- */

/// The first number in a piece of text: "1,234.5 cM (17.6%)" gives 1234.5.
pub fn first_number(s: &str) -> Option<f32> {
    let mut out = String::new();
    let mut started = false;
    for ch in s.chars() {
        if ch.is_ascii_digit() || (started && ch == '.') {
            out.push(ch);
            started = true;
        } else if started && ch == ',' {
            continue;
        } else if started {
            break;
        }
    }
    out.trim_end_matches('.').parse().ok()
}

/// A percentage in text: "17.6%" or "(17.6%)" or "<1%".
pub fn percent_in(s: &str) -> Option<f32> {
    let i = s.find('%')?;
    let before = &s[..i];
    // start just after the last character that is not part of the number (that character may be
    // several bytes long, e.g. a non-breaking space or a dash pasted from a website)
    let start = before.char_indices().rev().find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == ',' || c.is_whitespace())).map(|(j, c)| j + c.len_utf8()).unwrap_or(0);
    let n = first_number(&before[start..])?;
    if before[..start].trim_end().ends_with('<') && n <= 1.0 {
        return Some(0.5);
    }
    Some(n)
}

/// Rows of a CSV (or tab-separated) file. Handles quoted fields, commas and line breaks inside
/// quotes, doubled quotes, Windows line ends and a byte-order mark.
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let text = text.trim_start_matches('\u{feff}');
    let first = text.lines().next().unwrap_or("");
    let delim = [',', '\t', ';'].into_iter().max_by_key(|d| first.matches(*d).count()).unwrap_or(',');
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && field.is_empty() {
            quoted = true;
        } else if c == delim {
            row.push(std::mem::take(&mut field).trim().to_string());
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut field).trim().to_string());
            if row.iter().any(|f| !f.is_empty()) {
                rows.push(std::mem::take(&mut row));
            } else {
                row.clear();
            }
        } else {
            field.push(c);
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field.trim().to_string());
        if row.iter().any(|f| !f.is_empty()) {
            rows.push(row);
        }
    }
    rows
}

/// Which column of an imported file holds each detail (None = not in the file).
#[derive(Clone, Default, Debug, PartialEq)]
pub struct ColMap {
    pub name: Option<usize>,
    pub first: Option<usize>,
    pub last: Option<usize>,
    pub cm: Option<usize>,
    pub pct: Option<usize>,
    pub segments: Option<usize>,
    pub longest: Option<usize>,
    pub predicted: Option<usize>,
    pub side: Option<usize>,
    pub notes: Option<usize>,
    /// Chromosome number, in files with one row per shared segment (23andMe).
    pub chrom: Option<usize>,
    pub haplogroups: Option<usize>,
}

pub const FIELDS: &[&str] = &[
    "Name",
    "First name",
    "Last name",
    "Shared cM (or segment cM)",
    "Percent shared",
    "Segments",
    "Longest segment",
    "Predicted relationship",
    "Side (maternal / paternal)",
    "Notes",
    "Chromosome (one row per segment)",
    "Haplogroups",
];

impl ColMap {
    pub fn get_mut(&mut self, i: usize) -> &mut Option<usize> {
        match i {
            0 => &mut self.name,
            1 => &mut self.first,
            2 => &mut self.last,
            3 => &mut self.cm,
            4 => &mut self.pct,
            5 => &mut self.segments,
            6 => &mut self.longest,
            7 => &mut self.predicted,
            8 => &mut self.side,
            9 => &mut self.notes,
            10 => &mut self.chrom,
            _ => &mut self.haplogroups,
        }
    }
}

/// Work out the columns from the header row, whichever company the file came from.
pub fn guess_map(headers: &[String]) -> ColMap {
    let h: Vec<String> = headers.iter().map(|x| x.to_lowercase()).collect();
    let find = |test: &dyn Fn(&str) -> bool| h.iter().position(|x| test(x));
    let mut m = ColMap { name: None, ..Default::default() };
    m.name = find(&|x| ["name", "display name", "match name", "full name", "match", "matched name"].contains(&x))
        .or_else(|| find(&|x| (x.contains("display name") || x.contains("full name") || x.contains("match name")) && !x.contains("first") && !x.contains("last")))
        .or_else(|| find(&|x| x.contains("name") && !["first", "last", "middle", "sur", "user", "kit", "email", "file", "tree", "father", "mother", "maiden"].iter().any(|w| x.contains(w))));
    m.first = find(&|x| x.contains("first name") || x == "given name" || x == "given names");
    m.last = find(&|x| x.contains("last name") || x == "surname" || x == "family name");
    let is_longest = |x: &str| x.contains("longest") || x.contains("largest");
    m.longest = find(&|x| is_longest(x));
    m.cm = find(&|x| !is_longest(x) && (x.contains("cm") || x.contains("centimorgan")) && (x.contains("shared") || x.contains("total") || x.contains("dna") || x.contains("centimorgan") || x == "cm"))
        .or_else(|| find(&|x| !is_longest(x) && (x == "shared dna" || x == "total cm" || x == "cm")));
    m.pct = find(&|x| x.contains("percent") || x.contains('%'));
    m.segments = find(&|x| x.contains("segment") && !is_longest(x));
    m.predicted = find(&|x| x.contains("relationship") && (x.contains("predicted") || x.contains("suggested") || x.contains("estimated") || x.contains("range")))
        .or_else(|| find(&|x| x.contains("relationship") && !x.contains("linked") && !x.contains("tree")));
    m.side = find(&|x| x.contains("maternal") || x.contains("paternal") || x == "side" || x.contains("parental side"));
    m.notes = find(&|x| x.contains("note"));
    m.chrom = find(&|x| x.contains("chromosome") && !["start", "end", "point", "position", "browser"].iter().any(|w| x.contains(w)) || x == "chr");
    m.haplogroups = find(&|x| x.contains("haplogroup"));
    if m.chrom.is_some() && m.cm.is_none() {
        // segment files give each segment's length in cM
        m.cm = find(&|x| x.contains("genetic distance") || x.contains("centimorgan") || x == "cm" || x.contains("segment cm"));
    }
    if m.name.is_some() && (m.name == m.first || m.name == m.last) {
        m.name = None;
    }
    m
}

/// Turn the rows of an imported file (header row first) into matches. Rows with no name are
/// skipped. A "Shared DNA" cell such as "1,234 cM (17.6%)" gives both the cM and the percent.
pub fn rows_to_matches(rows: &[Vec<String>], m: &ColMap) -> Vec<DnaMatch> {
    let cell = |r: &Vec<String>, i: Option<usize>| -> String { i.and_then(|i| r.get(i)).map(|s| s.trim().to_string()).unwrap_or_default() };
    let mut out = Vec::new();
    for r in rows.iter().skip(1) {
        let mut name = cell(r, m.name);
        if name.is_empty() {
            name = format!("{} {}", cell(r, m.first), cell(r, m.last)).trim().to_string();
        }
        if name.is_empty() {
            continue;
        }
        let cm_text = cell(r, m.cm);
        let pct_text = cell(r, m.pct);
        let pct = percent_in(&pct_text).or_else(|| first_number(&pct_text)).or_else(|| percent_in(&cm_text));
        let cm = if cm_text.contains('%') && !cm_text.to_lowercase().contains("cm") { None } else { first_number(&cm_text) };
        let side = {
            let s = cell(r, m.side).to_lowercase();
            if s.starts_with('m') {
                "Maternal".to_string()
            } else if s.starts_with('p') && !s.starts_with("pa") || s.starts_with("pat") {
                "Paternal".to_string()
            } else if s.starts_with('b') {
                "Both".to_string()
            } else {
                cell(r, m.side)
            }
        };
        out.push(DnaMatch {
            name,
            cm,
            pct,
            segments: first_number(&cell(r, m.segments)).map(|n| n as u32),
            longest: first_number(&cell(r, m.longest)),
            predicted: cell(r, m.predicted),
            side,
            link: None,
            notes: {
                let (n, h) = (cell(r, m.notes), cell(r, m.haplogroups));
                match (n.is_empty(), h.is_empty()) {
                    (_, true) => n,
                    (true, false) => format!("Haplogroups: {}", h),
                    _ => format!("{}\nHaplogroups: {}", n, h),
                }
            },
        });
    }
    if m.chrom.is_some() {
        // one row per shared segment: add up each person's segments
        let mut merged: Vec<DnaMatch> = Vec::new();
        let mut at: HashMap<String, usize> = HashMap::new();
        for x in out {
            let key = x.name.to_lowercase();
            match at.get(&key) {
                Some(&i) => {
                    let y = &mut merged[i];
                    if m.segments.is_none() {
                        y.segments = Some(y.segments.unwrap_or(1) + 1);
                    }
                    y.longest = match (y.longest, x.cm) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (a, b) => a.or(b),
                    };
                    y.cm = match (y.cm, x.cm) {
                        (Some(a), Some(b)) => Some(a + b),
                        (a, b) => a.or(b),
                    };
                }
                None => {
                    let mut y = x;
                    if m.segments.is_none() {
                        y.segments = Some(1);
                    }
                    if m.longest.is_none() {
                        y.longest = y.cm;
                    }
                    at.insert(key, merged.len());
                    merged.push(y);
                }
            }
        }
        return merged;
    }
    out
}

/// Read a match list or results file as text, whatever form it came in: CSV or tab-separated
/// text (UTF-8, or UTF-16 as some Windows programs save it), or an Excel workbook (.xlsx, whose
/// first sheet is turned into CSV). Old binary Excel files (.xls) are not readable; the error
/// says to save them as .xlsx or CSV.
pub fn read_table_text(path: &Path) -> Result<String, String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "xls" {
        return Err("Old Excel files (.xls) can't be read. Open it in Excel, Numbers or LibreOffice and save it as .xlsx or CSV.".to_string());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read the file: {}", e))?;
    if ext == "xlsx" || bytes.starts_with(b"PK\x03\x04") {
        let rows = xlsx_rows(&bytes)?;
        return Ok(rows.iter().map(|r| r.iter().map(|f| if f.contains(',') || f.contains('"') || f.contains('\n') { format!("\"{}\"", f.replace('"', "\"\"")) } else { f.clone() }).collect::<Vec<_>>().join(",")).collect::<Vec<_>>().join("\n"));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        let le = bytes[0] == 0xFF;
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn xml_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(j) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        let ent = &tail[1..j];
        match ent {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                let code = if let Some(h) = ent.strip_prefix("#x") { u32::from_str_radix(h, 16).ok() } else { ent.strip_prefix('#').and_then(|d| d.parse().ok()) };
                match code.and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    None => out.push_str(&tail[..=j]),
                }
            }
        }
        rest = &tail[j + 1..];
    }
    out.push_str(rest);
    out
}

/// The text inside every <t>...</t> in a piece of XML, joined (rich text keeps its words in
/// several <t> elements).
fn xml_text(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("<t") {
        let tail = &rest[i..];
        // "<t>" or "<t xml:space=...>", but not "<tr" or other tags starting with t
        let after = tail.as_bytes().get(2).copied().unwrap_or(b'>');
        let Some(gt) = tail.find('>') else { break };
        if after != b'>' && after != b' ' || tail[..gt].ends_with('/') {
            rest = &tail[gt + 1..];
            continue;
        }
        let body = &tail[gt + 1..];
        let Some(end) = body.find("</t>") else { break };
        out.push_str(&xml_unescape(&body[..end]));
        rest = &body[end + 4..];
    }
    out
}

/// The first worksheet of an .xlsx workbook as rows of text.
pub fn xlsx_rows(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "This file could not be opened as an Excel workbook.".to_string())?;
    let mut read = |name: &str| -> Option<String> {
        let mut f = z.by_name(name).ok()?;
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        Some(s)
    };
    let shared: Vec<String> = read("xl/sharedStrings.xml")
        .map(|x| x.split("<si").skip(1).map(|si| xml_text(si.split("</si>").next().unwrap_or(""))).collect())
        .unwrap_or_default();
    let sheet = read("xl/worksheets/sheet1.xml").ok_or_else(|| "This workbook has no first sheet to read.".to_string())?;
    let col_of = |r: &str| -> usize {
        let mut n = 0usize;
        for c in r.chars().take_while(|c| c.is_ascii_alphabetic()) {
            n = n * 26 + (c.to_ascii_uppercase() as usize - 'A' as usize + 1);
        }
        n.saturating_sub(1)
    };
    let attr = |tag: &str, name: &str| -> Option<String> {
        let key = format!(" {}=\"", name);
        let i = tag.find(&key)? + key.len();
        let j = tag[i..].find('"')?;
        Some(tag[i..i + j].to_string())
    };
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in sheet.split("<row").skip(1) {
        let row = row.split("</row>").next().unwrap_or("");
        let mut cells: Vec<String> = Vec::new();
        for cell in row.split("<c").skip(1) {
            if !cell.starts_with(' ') && !cell.starts_with('>') && !cell.starts_with('/') {
                continue;
            }
            let Some(gt) = cell.find('>') else { continue };
            let tag = &cell[..gt];
            let col = attr(tag, "r").map(|r| col_of(&r)).unwrap_or(cells.len());
            let kind = attr(tag, "t").unwrap_or_default();
            let body = if tag.ends_with('/') { "" } else { cell[gt + 1..].split("</c>").next().unwrap_or("") };
            let v = body.split("<v>").nth(1).and_then(|x| x.split("</v>").next()).map(xml_unescape).unwrap_or_default();
            let text = match kind.as_str() {
                "s" => v.trim().parse::<usize>().ok().and_then(|i| shared.get(i).cloned()).unwrap_or_default(),
                "inlineStr" => xml_text(body),
                "b" => if v.trim() == "1" { "TRUE".to_string() } else { "FALSE".to_string() },
                _ => v,
            };
            if col >= cells.len() {
                cells.resize(col + 1, String::new());
            }
            cells[col] = text;
        }
        if cells.iter().any(|c| !c.trim().is_empty()) {
            rows.push(cells);
        }
    }
    Ok(rows)
}

/// True for a raw DNA data file (your own readings, as every company lets you download),
/// which is not a match list.
pub fn is_raw_dna(text: &str) -> bool {
    let head: String = text.lines().take(40).collect::<Vec<_>>().join("\n").to_lowercase();
    head.contains("rsid") && (head.contains("genotype") || head.contains("allele") || head.contains("chromosome"))
}

/// Where each company keeps its downloads, in a few words. Menus change, so this says what to
/// look for rather than every click.
pub const WHERE_FROM: &[(&str, &str)] = &[
    ("AncestryDNA", "Ancestry does not offer a match list download. Type in the matches you are working on, and paste the ethnicity estimate from the DNA Story page. Some browser extensions save Ancestry matches to a CSV file, which can be imported here."),
    ("23andMe", "In DNA Relatives, look at the bottom of the list for the option to request a DNA Relatives download; the file arrives by email. It has one row for each shared segment, which Gilded Roots adds up for each person."),
    ("MyHeritage", "In DNA Matches, look in the menu for Export DNA matches; the file is emailed to you. Not every account offers it."),
    ("FamilyTreeDNA", "Family Finder: the download (CSV) button at the top right of the match list. Y-DNA: the download beside the marker headings, and the Big Y match download. mtDNA: the download at the bottom right of the match list. Y-STR values can be downloaded from the Y-STR results page."),
    ("GEDmatch", "One-to-Many matches: use the option to download the list as a CSV file."),
    ("Any company", "Every company also lets you download your raw DNA data. That file is your own DNA readings, not a list of matches, so it is not imported here; upload it to other sites to find more matches."),
];

/// The "where do I get the file?" list, folded away until wanted.
pub fn where_from_help(ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Where do I get the file?").default_open(false).show(ui, |ui| {
        for (co, how) in WHERE_FROM {
            ui.label(RichText::new(*co).strong());
            ui.label(RichText::new(*how).small());
            ui.add_space(3.0);
        }
    });
}

/// Read an ethnicity estimate pasted from a company's web page or typed as a list, such as
/// "England & Northwestern Europe 45%", "Scotland: 22%" or a name with its percent on the next
/// line. Range lines ("Range: 30-56%") are skipped.
pub fn parse_regions(text: &str) -> Vec<Region> {
    let mut out: Vec<Region> = Vec::new();
    let mut pending: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let low = line.to_lowercase();
        if low.starts_with("range") || (line.contains('%') && (line.contains('\u{2013}') || line.matches(|c: char| c.is_ascii_digit()).count() > 0 && line.contains('-') && line.split('-').count() == 2 && line.split('-').all(|p| first_number(p).is_some()))) {
            continue;
        }
        match percent_in(line) {
            Some(p) => {
                // the name is whatever is left once the percent is taken out
                let i = line.find('%').unwrap_or(line.len());
                let before = &line[..i];
                let num_start = before.char_indices().rev().find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == ',' || c.is_whitespace() || *c == '<')).map(|(j, c)| j + c.len_utf8()).unwrap_or(0);
                let mut name = format!("{} {}", &line[..num_start], &line[i + 1..]);
                name = name.trim().trim_matches(|c: char| c == ':' || c == '-' || c == '|' || c == '\t' || c == '\u{2013}' || c == '\u{2014}' || c == '(' || c == ')' || c.is_whitespace()).to_string();
                if name.is_empty() {
                    if let Some(n) = pending.take() {
                        out.push(Region { name: n, pct: p });
                    }
                } else {
                    pending = None;
                    out.push(Region { name, pct: p });
                }
            }
            None => {
                if first_number(line).is_none() || line.chars().any(|c| c.is_alphabetic()) {
                    pending = Some(line.trim_end_matches(':').to_string());
                }
            }
        }
    }
    out
}

/* ---------- where regions are ---------- */

/// Rough map positions (longitude, latitude) for the region names testing companies use. Longer,
/// more specific names are tried first ("Southern Italy" before "Italy"); names not listed here
/// are looked up in the map's own list of countries.
const PLACES: &[(&str, f32, f32)] = &[
    ("england & northwestern europe", -1.5, 52.5),
    ("england and northwestern europe", -1.5, 52.5),
    ("england, wales & northwestern europe", -1.5, 52.5),
    ("british & irish", -3.5, 54.0),
    ("british and irish", -3.5, 54.0),
    ("northwestern europe", -1.0, 53.0),
    ("england", -1.5, 52.5),
    ("scotland", -4.2, 56.8),
    ("wales", -3.7, 52.3),
    ("ireland", -8.0, 53.2),
    ("cornwall", -4.8, 50.4),
    ("germanic europe", 10.0, 51.0),
    ("north and west europe", 3.0, 52.0),
    ("france", 2.5, 46.6),
    ("french & german", 6.5, 48.5),
    ("germany", 10.0, 51.0),
    ("netherlands", 5.3, 52.2),
    ("belgium", 4.6, 50.6),
    ("switzerland", 8.2, 46.8),
    ("austria", 14.5, 47.5),
    ("sweden & denmark", 13.0, 58.0),
    ("scandinavia", 15.0, 62.0),
    ("sweden", 16.0, 62.0),
    ("norway", 9.0, 61.0),
    ("denmark", 10.0, 56.0),
    ("finland", 26.0, 63.0),
    ("iceland", -19.0, 65.0),
    ("baltics", 24.0, 56.5),
    ("eastern europe & russia", 35.0, 55.0),
    ("eastern europe and russia", 35.0, 55.0),
    ("eastern europe", 28.0, 52.0),
    ("poland", 19.0, 52.0),
    ("russia", 40.0, 56.0),
    ("ukraine", 31.0, 49.0),
    ("hungary", 19.5, 47.2),
    ("czech", 15.5, 49.8),
    ("southern italy", 15.5, 40.0),
    ("northern italy", 10.0, 45.5),
    ("italy", 12.5, 42.5),
    ("sardinia", 9.0, 40.0),
    ("iberian peninsula", -4.0, 40.0),
    ("spain", -3.7, 40.2),
    ("portugal", -8.0, 39.6),
    ("basque", -2.5, 43.0),
    ("greece & albania", 21.5, 40.0),
    ("greece and albania", 21.5, 40.0),
    ("greece", 22.0, 39.5),
    ("balkans", 20.0, 44.0),
    ("southern europe", 14.0, 41.0),
    ("cyprus", 33.0, 35.0),
    ("malta", 14.4, 35.9),
    ("ashkenazi jewish", 22.0, 50.0),
    ("european jewish", 22.0, 50.0),
    ("sephardic jewish", -4.0, 37.5),
    ("jewish", 22.0, 50.0),
    ("levant", 36.0, 33.0),
    ("anatolia", 35.0, 39.0),
    ("turkey", 35.0, 39.0),
    ("caucasus", 44.0, 42.0),
    ("arabian peninsula", 45.0, 23.0),
    ("iran", 53.0, 32.0),
    ("persia", 53.0, 32.0),
    ("middle east", 44.0, 33.0),
    ("egypt", 30.0, 26.0),
    ("northern africa", 5.0, 30.0),
    ("north africa", 5.0, 30.0),
    ("nigeria", 8.0, 9.5),
    ("cameroon", 12.5, 5.5),
    ("congo", 15.0, -1.0),
    ("benin & togo", 1.8, 8.5),
    ("benin and togo", 1.8, 8.5),
    ("ghana", -1.0, 7.9),
    ("ivory coast", -5.5, 7.5),
    ("senegal", -14.5, 14.5),
    ("mali", -4.0, 17.0),
    ("sierra leone", -11.8, 8.5),
    ("kenya", 38.0, 0.5),
    ("ethiopia", 40.0, 9.0),
    ("somalia", 46.0, 6.0),
    ("southern bantu", 27.0, -25.0),
    ("south africa", 25.0, -29.0),
    ("eastern bantu", 34.0, -6.0),
    ("western bantu", 15.0, -6.0),
    ("west africa", -3.0, 12.0),
    ("western africa", -3.0, 12.0),
    ("east africa", 37.0, -1.0),
    ("central africa", 20.0, 2.0),
    ("sub-saharan africa", 18.0, 2.0),
    ("africa", 18.0, 2.0),
    ("indigenous arctic", -100.0, 68.0),
    ("indigenous americas-north", -100.0, 50.0),
    ("indigenous americas—north", -100.0, 50.0),
    ("indigenous americas-mexico", -102.0, 23.5),
    ("indigenous americas—mexico", -102.0, 23.5),
    ("indigenous americas-andean", -74.0, -12.0),
    ("indigenous americas—andean", -74.0, -12.0),
    ("indigenous americas-central", -87.0, 14.0),
    ("indigenous americas—central", -87.0, 14.0),
    ("indigenous americas", -95.0, 30.0),
    ("native american", -100.0, 45.0),
    ("mexico", -102.0, 23.5),
    ("central america", -87.0, 14.0),
    ("caribbean", -72.0, 18.0),
    ("andean", -74.0, -12.0),
    ("brazil", -52.0, -10.0),
    ("southern china", 113.0, 25.0),
    ("northern china", 115.0, 38.0),
    ("china", 104.0, 35.0),
    ("japan", 138.0, 36.0),
    ("korea", 127.8, 36.0),
    ("mongolia", 104.0, 46.8),
    ("vietnam", 106.0, 16.0),
    ("philippines", 122.0, 12.0),
    ("indonesia", 115.0, -2.0),
    ("thailand", 101.0, 15.0),
    ("cambodia", 105.0, 12.5),
    ("malaysia", 102.0, 4.0),
    ("southeast asia", 105.0, 10.0),
    ("east asia", 112.0, 34.0),
    ("northern india", 77.0, 28.0),
    ("southern india", 78.0, 12.0),
    ("south asia", 78.0, 22.0),
    ("india", 78.0, 22.0),
    ("pakistan", 70.0, 30.0),
    ("bangladesh", 90.0, 24.0),
    ("sri lanka", 81.0, 7.8),
    ("nepal", 84.0, 28.0),
    ("central asia", 65.0, 42.0),
    ("polynesia", -150.0, -15.0),
    ("hawaii", -157.0, 20.5),
    ("samoa", -172.0, -13.8),
    ("tonga", -175.2, -21.2),
    ("maori", 174.0, -41.0),
    ("melanesia", 150.0, -6.0),
    ("micronesia", 145.0, 13.0),
    ("aboriginal", 134.0, -25.0),
    ("oceania", 150.0, -10.0),
    ("western europe", 4.0, 48.0),
    ("northern europe", 12.0, 58.0),
    ("europe", 15.0, 50.0),
];

/// Where to put a region on the map, if it can be worked out.
pub fn region_place(name: &str) -> Option<[f32; 2]> {
    let n = name.to_lowercase().replace(" and ", " & ");
    let n = n.trim();
    let mut best: Option<(usize, [f32; 2])> = None;
    for (k, lon, lat) in PLACES {
        let k2 = k.replace(" and ", " & ");
        if n.contains(k2.as_str()) && best.is_none_or(|(l, _)| k2.len() > l) {
            best = Some((k2.len(), [*lon, *lat]));
        }
    }
    best.map(|(_, p)| p).or_else(|| mapview::geo().locate(name).map(|(ll, _)| ll))
}

/// Colours for regions, matches by side and the bar charts.
const PALETTE: [[u8; 3]; 12] = [
    [86, 140, 210],
    [214, 120, 60],
    [92, 168, 98],
    [196, 80, 90],
    [150, 110, 200],
    [200, 170, 60],
    [70, 170, 170],
    [220, 120, 170],
    [130, 150, 70],
    [160, 110, 80],
    [110, 130, 160],
    [190, 140, 110],
];

fn pal(i: usize) -> Color32 {
    let c = PALETTE[i % PALETTE.len()];
    Color32::from_rgb(c[0], c[1], c[2])
}

/* ---------- screen ---------- */

/// A file being imported, before the matches are added.
pub struct Import {
    pub file: String,
    pub rows: Vec<Vec<String>>,
    pub map: ColMap,
}

#[derive(Default)]
pub struct DnaUi {
    pub sel: usize,
    /// 0 ethnicity, 1 matches, 2 relationship estimate
    pub tab: u8,
    pub paste: Option<String>,
    pub find: String,
    pub side: String,
    pub min_cm: f32,
    pub sort: u8,
    pub asc: bool,
    pub edit: Option<(usize, DnaMatch)>,
    pub link_q: String,
    pub import: Option<Import>,
    pub est: String,
    pub details: bool,
    pub person_q: String,
    pub confirm_delete: bool,
}

impl App {
    pub(crate) fn dna_label(&self, t: &DnaTest) -> String {
        let who = t.person.filter(|p| self.db.people.contains_key(p)).map(|p| display_name(&self.db.people[&p])).unwrap_or_else(|| "Not linked to a person".to_string());
        let kind = KINDS.get(t.kind as usize).copied().unwrap_or("Autosomal");
        if t.company.is_empty() {
            format!("{} \u{2014} {}", who, kind)
        } else {
            format!("{} \u{2014} {} {}", who, t.company, kind)
        }
    }

    pub(crate) fn dna_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        // which test
        ui.horizontal_wrapped(|ui| {
            ui.heading("DNA");
            if self.db.dna.is_empty() {
                ui.label(RichText::new("No DNA tests yet.").weak());
            } else {
                if self.x.dna.sel >= self.db.dna.len() {
                    self.x.dna.sel = 0;
                }
                let cur = self.dna_label(&self.db.dna[self.x.dna.sel]);
                let labels: Vec<String> = self.db.dna.iter().map(|t| self.dna_label(t)).collect();
                egui::ComboBox::from_id_salt("dna_test").selected_text(cur).width(300.0).show_ui(ui, |ui| {
                    for (i, l) in labels.iter().enumerate() {
                        if ui.selectable_label(self.x.dna.sel == i, l.as_str()).clicked() {
                            self.x.dna.sel = i;
                        }
                    }
                });
                if ui.button("Test details...").clicked() {
                    self.x.dna.details = true;
                }
            }
            let mut add: Option<u8> = None;
            ui.menu_button("+ Add a DNA test", |ui| {
                if ui.button("Autosomal (ethnicity and cousin matches)").on_hover_text("AncestryDNA, 23andMe, MyHeritage, Family Finder, Living DNA").clicked() {
                    add = Some(0);
                    ui.close_menu();
                }
                if ui.button("Y-DNA (father's father's line)").on_hover_text("Taken by men: Y-37, Y-111, Big Y...").clicked() {
                    add = Some(1);
                    ui.close_menu();
                }
                if ui.button("mtDNA (mother's mother's line)").on_hover_text("Taken by anyone: HVR1, HVR2, full sequence").clicked() {
                    add = Some(2);
                    ui.close_menu();
                }
            });
            if let Some(k) = add {
                let company = if k == 0 { "AncestryDNA" } else { "FamilyTreeDNA" };
                self.db.dna.push(DnaTest { person: self.sel, company: company.to_string(), kind: k, ..Default::default() });
                self.x.dna.sel = self.db.dna.len() - 1;
                self.x.dna.tab = 0;
                self.x.dna.details = true;
                self.x.next_label = Some(format!("Added a {} test", KINDS[k as usize]));
                self.dirty = true;
            }
        });
        if self.db.dna.is_empty() {
            ui.add_space(20.0);
            ui.label("Keep DNA test results here. Autosomal tests give an ethnicity estimate (shown as a table, a chart and a map) and a list of cousin matches. Y-DNA follows the father's father's line and mtDNA the mother's mother's line: each has a haplogroup, its own matches, and a map of that line through your tree.");
            ui.add_space(6.0);
            ui.label("Click \"+ Add a DNA test\" to start. Match lists can be imported from the CSV file that 23andMe, MyHeritage, FamilyTreeDNA and GEDmatch let you download, and an ethnicity estimate can be pasted straight from the testing company's web page.");
            ui.add_space(12.0);
            self.dna_estimator(ui);
            return;
        }
        if self.db.dna[self.x.dna.sel].kind != 0 {
            self.line_view(ui, act);
            self.dna_windows(ui.ctx(), act);
            return;
        }
        ui.horizontal(|ui| {
            for (k, l) in [(0u8, "Ethnicity"), (1, "Matches"), (2, "How related?")] {
                if ui.selectable_label(self.x.dna.tab == k, l).clicked() {
                    self.x.dna.tab = k;
                }
            }
        });
        ui.separator();
        match self.x.dna.tab {
            0 => self.dna_ethnicity(ui),
            1 => self.dna_matches(ui, act),
            _ => {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.dna_estimator(ui));
            }
        }
        self.dna_windows(ui.ctx(), act);
    }

    fn dna_ethnicity(&mut self, ui: &mut egui::Ui) {
        let si = self.x.dna.sel;
        let mut changed = false;
        let mut regions = self.db.dna[si].regions.clone();
        regions.sort_by(|a, b| b.pct.partial_cmp(&a.pct).unwrap_or(std::cmp::Ordering::Equal));
        let total: f32 = regions.iter().map(|r| r.pct).sum();
        let avail = ui.available_size();
        let left_w = (avail.x * 0.42).clamp(320.0, 520.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(left_w);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("+ Add a region").clicked() {
                        self.db.dna[si].regions.push(Region { name: String::new(), pct: 0.0 });
                        changed = true;
                    }
                    if ui.button("Paste from the testing site...").on_hover_text("Copy the list of regions and percentages from the company's web page and paste it in").clicked() {
                        self.x.dna.paste = Some(String::new());
                    }
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical().id_salt("dna_reg").max_height((avail.y * 0.5).max(160.0)).show(ui, |ui| {
                    let mut remove: Option<usize> = None;
                    egui::Grid::new("dna_regions").num_columns(4).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new("").small());
                        ui.label(RichText::new("Region").strong());
                        ui.label(RichText::new("%").strong());
                        ui.label("");
                        ui.end_row();
                        let n = self.db.dna[si].regions.len();
                        for i in 0..n {
                            let rank = regions.iter().position(|r| *r == self.db.dna[si].regions[i]).unwrap_or(i);
                            let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                            ui.painter().rect_filled(r, 2.0_f32, pal(rank));
                            let reg = &mut self.db.dna[si].regions[i];
                            if ui.add_sized([(left_w - 150.0).max(150.0), 20.0], egui::TextEdit::singleline(&mut reg.name).hint_text("e.g. Scotland")).changed() {
                                changed = true;
                            }
                            if ui.add(egui::DragValue::new(&mut reg.pct).range(0.0..=100.0).speed(0.5).suffix("%")).changed() {
                                changed = true;
                            }
                            if ui.small_button("x").on_hover_text("Remove this region").clicked() {
                                remove = Some(i);
                            }
                            ui.end_row();
                        }
                    });
                    if let Some(i) = remove {
                        self.db.dna[si].regions.remove(i);
                        changed = true;
                    }
                });
                ui.add_space(4.0);
                let tot_txt = format!("Total {:.0}%", total);
                if regions.is_empty() {
                    ui.label(RichText::new("No regions yet.").weak());
                } else if (total - 100.0).abs() > 2.0 {
                    ui.colored_label(Color32::from_rgb(220, 120, 40), format!("{} (the regions usually add up to about 100%)", tot_txt));
                } else {
                    ui.label(RichText::new(tot_txt).weak());
                }
                ui.add_space(8.0);
                // bar chart, largest first
                let named: Vec<&Region> = regions.iter().filter(|r| r.pct > 0.0).collect();
                if !named.is_empty() {
                    ui.label(RichText::new("Chart").strong());
                    let maxp = named.iter().map(|r| r.pct).fold(1.0f32, f32::max);
                    for (i, r) in named.iter().enumerate() {
                        let (rect, resp) = ui.allocate_exact_size(vec2(left_w, 22.0), Sense::hover());
                        let label_w = (left_w * 0.45).min(210.0);
                        let p = ui.painter();
                        let name = if r.name.trim().is_empty() { "(no name)" } else { r.name.trim() };
                        let g = p.layout_no_wrap(name.to_string(), FontId::proportional(12.0), ui.visuals().text_color());
                        p.with_clip_rect(Rect::from_min_size(rect.min, vec2(label_w - 6.0, rect.height()))).galley(pos2(rect.min.x, rect.center().y - g.size().y / 2.0), g, ui.visuals().text_color());
                        let bar_w = (left_w - label_w - 50.0).max(20.0) * (r.pct / maxp);
                        let bar = Rect::from_min_size(pos2(rect.min.x + label_w, rect.min.y + 4.0), vec2(bar_w.max(2.0), 14.0));
                        p.rect_filled(bar, 3.0_f32, pal(i));
                        p.text(pos2(bar.max.x + 6.0, rect.center().y), Align2::LEFT_CENTER, if r.pct < 1.0 { "<1%".to_string() } else { format!("{:.0}%", r.pct) }, FontId::proportional(12.0), ui.visuals().text_color());
                        resp.on_hover_text(format!("{}: {:.1}%", name, r.pct));
                    }
                }
            });
            ui.separator();
            // map
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Where the regions are").strong());
                    ui.separator();
                    ui.label("Map style");
                    if mapview::style_picker(ui, &mut self.settings.map_style) {
                        save_settings(&self.settings);
                    }
                });
                let size = vec2(ui.available_width().max(200.0), (avail.y - 40.0).clamp(220.0, 900.0));
                let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
                let placed: Vec<(usize, &Region, [f32; 2])> = regions.iter().enumerate().filter(|(_, r)| r.pct > 0.0).filter_map(|(i, r)| region_place(&r.name).map(|ll| (i, r, ll))).collect();
                // fit the map to the regions, with room around them
                let (mut lo, mut hi) = ([180.0f32, 90.0f32], [-180.0f32, -90.0f32]);
                for (_, _, ll) in &placed {
                    lo = [lo[0].min(ll[0]), lo[1].min(ll[1])];
                    hi = [hi[0].max(ll[0]), hi[1].max(ll[1])];
                }
                if placed.is_empty() {
                    lo = [-170.0, -50.0];
                    hi = [170.0, 70.0];
                }
                let pad_lon = ((hi[0] - lo[0]) * 0.25).max(18.0);
                let pad_lat = ((hi[1] - lo[1]) * 0.25).max(10.0);
                let (lo, hi) = ([lo[0] - pad_lon, (lo[1] - pad_lat).max(-70.0)], [hi[0] + pad_lon, (hi[1] + pad_lat).min(80.0)]);
                let (m0, m1) = (mapview::merc_lat(lo[1]), mapview::merc_lat(hi[1]));
                let k = (rect.width() / (hi[0] - lo[0])).min(rect.height() / (m1 - m0).max(1.0));
                let c = [(lo[0] + hi[0]) / 2.0, (m0 + m1) / 2.0];
                let painter = ui.painter_at(rect);
                mapview::paint_world(&painter, rect, c, k, ui.visuals().dark_mode);
                let mut hover: Option<String> = None;
                let mut labels: Vec<(egui::Pos2, f32, String)> = Vec::new();
                for (i, r, ll) in &placed {
                    let p = mapview::world_point(rect, c, k, ll[0], ll[1]);
                    let rad = 7.0 + r.pct.sqrt() * 4.2;
                    let col = pal(*i);
                    // a soft halo, a solid centre with a fine light edge, and the percentage
                    painter.circle_filled(p, rad * 1.7, col.gamma_multiply(0.16));
                    painter.circle_filled(p, rad * 1.3, col.gamma_multiply(0.22));
                    painter.circle(p, rad, col.gamma_multiply(0.92), Stroke::new(1.2_f32, Color32::from_white_alpha(200)));
                    painter.text(p, Align2::CENTER_CENTER, if r.pct < 1.0 { "<1".to_string() } else { format!("{:.0}%", r.pct) }, FontId::proportional(if rad > 18.0 { 12.0 } else { 10.0 }), Color32::WHITE);
                    if r.pct >= 5.0 {
                        labels.push((p, rad, r.name.trim().to_string()));
                    }
                    if resp.hover_pos().is_some_and(|m| m.distance(p) <= rad) {
                        hover = Some(format!("{}: {:.1}%", r.name, r.pct));
                    }
                }
                // names go on top of all the circles, moved down when they would overlap
                let mut taken: Vec<Rect> = Vec::new();
                for (p, rad, name) in labels {
                    let label = painter.layout_no_wrap(name, FontId::proportional(11.0), Color32::WHITE);
                    let size = label.size() + vec2(8.0, 2.0);
                    let mut box_ = Rect::from_min_size(pos2(p.x - size.x / 2.0, p.y + rad + 3.0), size);
                    for _ in 0..12 {
                        match taken.iter().find(|t| t.expand(1.0).intersects(box_)) {
                            Some(t) => box_ = box_.translate(vec2(0.0, t.max.y + 2.0 - box_.min.y)),
                            None => break,
                        }
                    }
                    taken.push(box_);
                    painter.rect_filled(box_, 3.0_f32, Color32::from_black_alpha(140));
                    painter.galley(box_.min + vec2(4.0, 1.0), label, Color32::WHITE);
                }
                if let Some(h) = hover {
                    resp.on_hover_text(h);
                }
                let missing: Vec<&str> = regions.iter().filter(|r| r.pct > 0.0 && !r.name.trim().is_empty() && region_place(&r.name).is_none()).map(|r| r.name.as_str()).collect();
                if !missing.is_empty() {
                    ui.label(RichText::new(format!("Not on the map (name not recognised): {}", missing.join(", "))).small().weak());
                }
                ui.label(RichText::new("Circles are placed in the middle of each region; their size follows the percentage.").small().weak());
            });
        });
        if changed {
            self.x.next_label = Some("Changed a DNA ethnicity estimate".to_string());
            self.dirty = true;
        }
    }

    fn dna_matches(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let si = self.x.dna.sel;
        ui.horizontal_wrapped(|ui| {
            if ui.button("+ Add a match").clicked() {
                self.x.dna.edit = Some((usize::MAX, DnaMatch::default()));
                self.x.dna.link_q.clear();
            }
            if ui.button("Import a match list...").on_hover_text("A CSV, text or Excel file: the match list downloaded from 23andMe, MyHeritage, FamilyTreeDNA or GEDmatch, or your own spreadsheet").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Match list (CSV, text or Excel)", &["csv", "txt", "tsv", "xlsx", "xls"]).set_directory(start_dir()).pick_file() {
                    remember_dir(&path);
                    match read_table_text(&path) {
                        Ok(text) => {
                            let rows = parse_csv(&text);
                            if is_raw_dna(&text) {
                                self.status = "That is a raw DNA data file (your own DNA readings), not a match list. Download the match list instead: see \"Where do I get the file?\".".to_string();
                            } else if rows.len() < 2 {
                                self.status = "That file has no rows to import.".to_string();
                            } else {
                                let map = guess_map(&rows[0]);
                                self.x.dna.import = Some(Import { file: path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(), rows, map });
                            }
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
            ui.separator();
            ui.add(egui::TextEdit::singleline(&mut self.x.dna.find).hint_text("Filter by name or notes").desired_width(180.0));
            egui::ComboBox::from_id_salt("dna_side").selected_text(if self.x.dna.side.is_empty() { "Any side".to_string() } else { self.x.dna.side.clone() }).width(100.0).show_ui(ui, |ui| {
                for s in ["", "Paternal", "Maternal", "Both", "Unknown"] {
                    if ui.selectable_label(self.x.dna.side == s, if s.is_empty() { "Any side" } else { s }).clicked() {
                        self.x.dna.side = s.to_string();
                    }
                }
            });
            ui.label("At least");
            ui.add(egui::DragValue::new(&mut self.x.dna.min_cm).range(0.0..=4000.0).speed(5.0).suffix(" cM"));
        });
        where_from_help(ui);
        let t = &self.db.dna[si];
        let q = self.x.dna.find.to_lowercase();
        let mut idx: Vec<usize> = (0..t.matches.len())
            .filter(|&i| {
                let m = &t.matches[i];
                (q.is_empty() || m.name.to_lowercase().contains(&q) || m.notes.to_lowercase().contains(&q))
                    && (self.x.dna.side.is_empty() || (self.x.dna.side == "Unknown" && m.side.is_empty()) || m.side == self.x.dna.side)
                    && m.cm.unwrap_or(0.0) >= self.x.dna.min_cm
            })
            .collect();
        let key = |m: &DnaMatch, s: u8| -> (i64, String) {
            match s {
                0 => ((m.cm.unwrap_or(-1.0) * 10.0) as i64, String::new()),
                1 => (0, m.name.to_lowercase()),
                2 => (m.segments.map(|x| x as i64).unwrap_or(-1), String::new()),
                3 => ((m.longest.unwrap_or(-1.0) * 10.0) as i64, String::new()),
                _ => (0, m.side.clone()),
            }
        };
        let s = self.x.dna.sort;
        idx.sort_by(|a, b| {
            let o = key(&t.matches[*a], s).cmp(&key(&t.matches[*b], s));
            let o = if s == 0 || s == 2 || s == 3 { o.reverse() } else { o };
            if self.x.dna.asc { o.reverse() } else { o }
        });
        ui.label(RichText::new(format!("{} of {} match(es)", idx.len(), t.matches.len())).weak());
        // closeness summary
        let mut counts = vec![0usize; GROUPS.len()];
        for m in &t.matches {
            if let Some(g) = m.cm.and_then(|c| likely_groups(c).first().copied()) {
                counts[g] += 1;
            }
        }
        let mut open_edit: Option<usize> = None;
        let mut go: Option<u32> = None;
        let mut sort_click: Option<u8> = None;
        egui::SidePanel::right("dna_side_panel").resizable(true).default_width(320.0).show_inside(ui, |ui| {
            ui.label(RichText::new("Matches by closeness").strong());
            ui.label(RichText::new("Grouped by shared cM").small().weak());
            ui.add_space(4.0);
            let maxc = counts.iter().copied().max().unwrap_or(0).max(1);
            let w = ui.available_width();
            for (i, g) in GROUPS.iter().enumerate() {
                let (rect, resp) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
                let p = ui.painter();
                p.text(pos2(rect.min.x, rect.center().y), Align2::LEFT_CENTER, g.2, FontId::proportional(11.5), ui.visuals().text_color());
                let bx = rect.min.x + w * 0.62;
                let bw = (w * 0.28) * counts[i] as f32 / maxc as f32;
                if counts[i] > 0 {
                    p.rect_filled(Rect::from_min_size(pos2(bx, rect.min.y + 4.0), vec2(bw.max(2.0), 12.0)), 2.0_f32, pal(i));
                }
                p.text(pos2(bx + bw + 5.0, rect.center().y), Align2::LEFT_CENTER, counts[i].to_string(), FontId::proportional(11.5), ui.visuals().weak_text_color());
                resp.on_hover_text(format!("{}\nAbout {:.0} cM on average", g.1, g.0));
            }
        });
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("dna_match_grid").striped(true).num_columns(9).spacing([12.0, 5.0]).show(ui, |ui| {
                for (k, l) in [(1u8, "Name"), (0, "Shared cM"), (2, "Segments"), (3, "Longest"), (9, "Company's prediction"), (9, "Likely (from cM)"), (4, "Side"), (9, "In my tree"), (9, "")] {
                    if k == 9 {
                        ui.label(RichText::new(l).strong());
                    } else {
                        let arrow = if self.x.dna.sort == k { if self.x.dna.asc { " ^" } else { " v" } } else { "" };
                        if ui.add(egui::Label::new(RichText::new(format!("{}{}", l, arrow)).strong()).sense(Sense::click())).on_hover_text("Click to sort").clicked() {
                            sort_click = Some(k);
                        }
                    }
                }
                ui.end_row();
                for &i in &idx {
                    let m = &t.matches[i];
                    ui.label(m.name.as_str());
                    ui.label(match (m.cm, m.pct) {
                        (Some(c), _) => format!("{:.0}", c),
                        (None, Some(p)) => format!("{:.2}%", p),
                        _ => String::new(),
                    });
                    ui.label(m.segments.map(|s| s.to_string()).unwrap_or_default());
                    ui.label(m.longest.map(|s| format!("{:.0}", s)).unwrap_or_default());
                    ui.label(m.predicted.as_str());
                    ui.label(RichText::new(likely(m.cm)).weak());
                    ui.label(m.side.as_str());
                    match m.link.filter(|p| self.db.people.contains_key(p)) {
                        Some(p) => {
                            if ui.link(display_name(&self.db.people[&p])).clicked() {
                                go = Some(p);
                            }
                        }
                        None => {
                            ui.label("");
                        }
                    }
                    if ui.small_button("Edit").clicked() {
                        open_edit = Some(i);
                    }
                    ui.end_row();
                }
            });
            if t.matches.is_empty() {
                ui.add_space(10.0);
                ui.label(RichText::new("No matches yet. Add them one at a time, or import the list your testing company lets you download. AncestryDNA does not offer a match download, so type in the matches you are working on.").weak());
            }
        });
        if let Some(k) = sort_click {
            if self.x.dna.sort == k {
                self.x.dna.asc = !self.x.dna.asc;
            } else {
                self.x.dna.sort = k;
                self.x.dna.asc = false;
            }
        }
        if let Some(i) = open_edit {
            self.x.dna.edit = Some((i, self.db.dna[si].matches[i].clone()));
            self.x.dna.link_q.clear();
        }
        if let Some(p) = go {
            *act = Some(Action::Open(p));
        }
    }

    fn dna_estimator(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("How might we be related?").size(16.0).strong());
        ui.label("Type the shared cM for a match to see which relationships usually share about that much DNA.");
        ui.horizontal(|ui| {
            ui.label("Shared cM");
            ui.add(egui::TextEdit::singleline(&mut self.x.dna.est).hint_text("e.g. 850").desired_width(90.0));
            if let Some(c) = first_number(&self.x.dna.est) {
                if ui.button("Open DNA Painter's Shared cM tool").on_hover_text("Detailed probabilities from real results (opens a web page)").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(format!("https://dnapainter.com/tools/sharedcmv4/{}", c.round() as i64)));
                }
            }
        });
        if let Some(c) = first_number(&self.x.dna.est) {
            ui.add_space(4.0);
            let best = likely_groups(c);
            if best.is_empty() {
                ui.label("Type an amount above 0.");
            }
            for (n, g) in best.iter().enumerate() {
                let (avg, long, _) = GROUPS[*g];
                ui.label(RichText::new(format!("{}{} (about {:.0} cM on average)", if n == 0 { "Most likely: " } else { "Also possible: " }, long, avg)).strong());
            }
            if c > 3800.0 {
                ui.label("More than a parent shares: possibly an identical twin, or a second copy of your own test.");
            }
        }
        ui.add_space(10.0);
        ui.label(RichText::new("Average DNA shared by relationship").strong());
        egui::Grid::new("dna_groups").striped(true).spacing([16.0, 4.0]).show(ui, |ui| {
            ui.label(RichText::new("About").strong());
            ui.label(RichText::new("Relationships").strong());
            ui.end_row();
            for (avg, long, _) in GROUPS {
                ui.label(format!("{:.0} cM", avg));
                ui.label(*long);
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("These are averages worked out from how DNA is inherited. Real amounts vary widely around them: a 1st cousin can share anywhere from about half to double the average, and distant cousins often show more than the average or nothing at all. Use them as a guide alongside your tree, dates and places.").small().weak());
    }

    fn dna_windows(&mut self, ctx: &egui::Context, _act: &mut Option<Action>) {
        let si = self.x.dna.sel;
        // test details
        if self.x.dna.details {
            let mut open = true;
            let mut delete = false;
            egui::Window::new("DNA test details").open(&mut open).resizable(false).show(ctx, |ui| {
                let who = self.db.dna[si].person.filter(|p| self.db.people.contains_key(p)).map(|p| self.db.label(p)).unwrap_or_else(|| "Not chosen".to_string());
                egui::Grid::new("dna_details").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Who took the test");
                    ui.label(RichText::new(who).strong());
                    ui.end_row();
                    ui.label("");
                    ui.add(egui::TextEdit::singleline(&mut self.x.dna.person_q).hint_text("Type a name to choose someone").desired_width(260.0));
                    ui.end_row();
                    let q = self.x.dna.person_q.trim().to_string();
                    if !q.is_empty() {
                        for id in self.db.sorted(&q).into_iter().take(6) {
                            ui.label("");
                            if ui.button(self.db.label(id)).clicked() {
                                self.db.dna[si].person = Some(id);
                                self.x.dna.person_q.clear();
                                self.dirty = true;
                            }
                            ui.end_row();
                        }
                    }
                    ui.label("Type of test");
                    let t = &mut self.db.dna[si];
                    egui::ComboBox::from_id_salt("dna_kind").selected_text(KINDS.get(t.kind as usize).copied().unwrap_or("Autosomal")).show_ui(ui, |ui| {
                        for (k, n) in KINDS.iter().enumerate() {
                            if ui.selectable_label(t.kind as usize == k, *n).clicked() {
                                t.kind = k as u8;
                            }
                        }
                    });
                    ui.end_row();
                    if t.kind == 1 && t.person.and_then(|p| self.db.people.get(&p)).is_some_and(|p| p.sex == "F") {
                        ui.label("");
                        ui.colored_label(Color32::from_rgb(220, 120, 40), "Only men carry Y-DNA. A woman's father's line is traced by testing her father, brother or a paternal uncle or cousin.");
                        ui.end_row();
                    }
                    ui.label("Testing company");
                    let t = &mut self.db.dna[si];
                    egui::ComboBox::from_id_salt("dna_co").selected_text(if t.company.is_empty() { "Choose".to_string() } else { t.company.clone() }).show_ui(ui, |ui| {
                        for c in COMPANIES {
                            if ui.selectable_label(t.company == *c, *c).clicked() {
                                t.company = c.to_string();
                            }
                        }
                    });
                    ui.end_row();
                    ui.label("Kit number or user name");
                    ui.add(egui::TextEdit::singleline(&mut t.kit).desired_width(260.0));
                    ui.end_row();
                    ui.label("Results date");
                    ui.add(egui::TextEdit::singleline(&mut t.date).hint_text("e.g. Mar 2024").desired_width(260.0));
                    ui.end_row();
                    ui.label("Notes");
                    ui.add(egui::TextEdit::multiline(&mut t.notes).desired_rows(3).desired_width(260.0));
                    ui.end_row();
                });
                ui.separator();
                if self.x.dna.confirm_delete {
                    ui.colored_label(Color32::from_rgb(220, 120, 40), "Delete this test with its ethnicity estimate and all its matches?");
                    ui.horizontal(|ui| {
                        if ui.button("Yes, delete it").clicked() {
                            delete = true;
                        }
                        if ui.button("Keep it").clicked() {
                            self.x.dna.confirm_delete = false;
                        }
                    });
                } else if ui.button("Delete this test...").clicked() {
                    self.x.dna.confirm_delete = true;
                }
            });
            if delete {
                self.db.dna.remove(si);
                self.x.dna.sel = 0;
                self.x.dna.details = false;
                self.x.dna.confirm_delete = false;
                self.x.next_label = Some("Deleted a DNA test".to_string());
                self.dirty = true;
                return;
            }
            if !open {
                self.x.dna.details = false;
                self.x.dna.confirm_delete = false;
                self.dirty = true;
            }
        }
        // paste ethnicity
        if let Some(mut text) = self.x.dna.paste.take() {
            let mut open = true;
            let mut keep = true;
            egui::Window::new("Paste an ethnicity estimate").open(&mut open).default_width(460.0).show(ctx, |ui| {
                ui.label("On the testing company's site, select the list of regions with their percentages, copy it, and paste it here. You can also type one region per line, such as \"Scotland 32%\".");
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut text).desired_rows(10).desired_width(f32::INFINITY));
                });
                let found = parse_regions(&text);
                ui.label(format!("Found {} region(s){}", found.len(), if found.is_empty() { String::new() } else { format!(", adding up to {:.0}%", found.iter().map(|r| r.pct).sum::<f32>()) }));
                for r in found.iter().take(12) {
                    ui.label(RichText::new(format!("{}  {}%", r.name, r.pct)).small());
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(!found.is_empty(), egui::Button::new("Replace the estimate with these")).clicked() {
                        self.db.dna[si].regions = found.clone();
                        self.x.next_label = Some("Pasted a DNA ethnicity estimate".to_string());
                        self.dirty = true;
                        keep = false;
                    }
                    if ui.add_enabled(!found.is_empty(), egui::Button::new("Add to the estimate")).clicked() {
                        self.db.dna[si].regions.extend(found.clone());
                        self.x.next_label = Some("Added to a DNA ethnicity estimate".to_string());
                        self.dirty = true;
                        keep = false;
                    }
                });
            });
            if open && keep {
                self.x.dna.paste = Some(text);
            }
        }
        // add or edit a match
        if let Some((i, mut m)) = self.x.dna.edit.take() {
            let mut open = true;
            let mut done = false;
            let mut delete = false;
            egui::Window::new(if i == usize::MAX { "Add a DNA match" } else { "DNA match" }).open(&mut open).resizable(false).show(ctx, |ui| {
                let num = |ui: &mut egui::Ui, v: &mut Option<f32>, hint: &str| {
                    let mut s = v.map(|x| if x.fract() == 0.0 { format!("{:.0}", x) } else { format!("{}", x) }).unwrap_or_default();
                    if ui.add(egui::TextEdit::singleline(&mut s).hint_text(hint).desired_width(90.0)).changed() {
                        *v = first_number(&s);
                    }
                };
                egui::Grid::new("dna_edit").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(260.0));
                    ui.end_row();
                    ui.label("Shared cM");
                    ui.horizontal(|ui| {
                        num(ui, &mut m.cm, "e.g. 212");
                        if let Some(c) = m.cm {
                            ui.label(RichText::new(likely(Some(c))).weak());
                        }
                    });
                    ui.end_row();
                    ui.label("Percent shared");
                    num(ui, &mut m.pct, "if no cM");
                    ui.end_row();
                    ui.label("Segments");
                    let mut seg = m.segments.map(|x| x as f32);
                    num(ui, &mut seg, "e.g. 9");
                    m.segments = seg.map(|x| x as u32);
                    ui.end_row();
                    ui.label("Longest segment (cM)");
                    num(ui, &mut m.longest, "e.g. 45");
                    ui.end_row();
                    ui.label("Company's prediction");
                    ui.add(egui::TextEdit::singleline(&mut m.predicted).hint_text("e.g. 2nd-3rd cousin").desired_width(260.0));
                    ui.end_row();
                    ui.label("Side");
                    egui::ComboBox::from_id_salt("dna_m_side").selected_text(if m.side.is_empty() { "Not known".to_string() } else { m.side.clone() }).show_ui(ui, |ui| {
                        for s in SIDES {
                            if ui.selectable_label(m.side == *s, if s.is_empty() { "Not known" } else { s }).clicked() {
                                m.side = s.to_string();
                            }
                        }
                    });
                    ui.end_row();
                    ui.label("In my tree");
                    let cur = m.link.filter(|p| self.db.people.contains_key(p)).map(|p| self.db.label(p)).unwrap_or_else(|| "Not linked".to_string());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(cur).strong());
                        if m.link.is_some() && ui.small_button("Unlink").clicked() {
                            m.link = None;
                        }
                    });
                    ui.end_row();
                    ui.label("");
                    ui.add(egui::TextEdit::singleline(&mut self.x.dna.link_q).hint_text("Type a name to link this match").desired_width(260.0));
                    ui.end_row();
                    let q = self.x.dna.link_q.trim().to_string();
                    if !q.is_empty() {
                        for id in self.db.sorted(&q).into_iter().take(6) {
                            ui.label("");
                            if ui.button(self.db.label(id)).clicked() {
                                m.link = Some(id);
                                self.x.dna.link_q.clear();
                            }
                            ui.end_row();
                        }
                    }
                    ui.label("Notes");
                    ui.add(egui::TextEdit::multiline(&mut m.notes).hint_text("Shared surnames, where their tree goes, messages...").desired_rows(3).desired_width(260.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.add_enabled(!m.name.trim().is_empty(), egui::Button::new("Save")).clicked() {
                        done = true;
                    }
                    if i != usize::MAX && ui.button("Delete this match").clicked() {
                        delete = true;
                    }
                });
            });
            if done {
                let list = &mut self.db.dna[si].matches;
                if i == usize::MAX {
                    list.push(m);
                    self.x.next_label = Some("Added a DNA match".to_string());
                } else if i < list.len() {
                    list[i] = m;
                    self.x.next_label = Some("Changed a DNA match".to_string());
                }
                self.dirty = true;
            } else if delete {
                if i < self.db.dna[si].matches.len() {
                    self.db.dna[si].matches.remove(i);
                }
                self.x.next_label = Some("Deleted a DNA match".to_string());
                self.dirty = true;
            } else if open {
                self.x.dna.edit = Some((i, m));
            }
        }
        // import a match list
        if let Some(mut imp) = self.x.dna.import.take() {
            let mut open = true;
            let mut add: Option<Vec<DnaMatch>> = None;
            egui::Window::new("Import DNA matches").open(&mut open).default_width(620.0).show(ctx, |ui| {
                ui.label(format!("{}: {} row(s)", imp.file, imp.rows.len().saturating_sub(1)));
                ui.label(RichText::new("Check which column holds each detail. Most are found automatically; change any that are wrong.").small());
                let headers = imp.rows[0].clone();
                egui::Grid::new("dna_imp").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    for (fi, f) in FIELDS.iter().enumerate() {
                        ui.label(*f);
                        let slot = imp.map.get_mut(fi);
                        let shown = slot.and_then(|c| headers.get(c)).cloned().unwrap_or_else(|| "(not in this file)".to_string());
                        egui::ComboBox::from_id_salt(("dna_imp_col", fi)).selected_text(shown).width(260.0).show_ui(ui, |ui| {
                            if ui.selectable_label(slot.is_none(), "(not in this file)").clicked() {
                                *slot = None;
                            }
                            for (ci, h) in headers.iter().enumerate() {
                                if ui.selectable_label(*slot == Some(ci), h.as_str()).clicked() {
                                    *slot = Some(ci);
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
                let found = rows_to_matches(&imp.rows, &imp.map);
                let existing: HashSet<(String, i64)> = self.db.dna[si].matches.iter().map(|m| (m.name.to_lowercase(), (m.cm.unwrap_or(-1.0) * 10.0) as i64)).collect();
                let new: Vec<DnaMatch> = found.iter().filter(|m| !existing.contains(&(m.name.to_lowercase(), (m.cm.unwrap_or(-1.0) * 10.0) as i64))).cloned().collect();
                ui.separator();
                ui.label(RichText::new("First rows, as they will be added").strong());
                egui::Grid::new("dna_imp_prev").striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
                    for h in ["Name", "cM", "%", "Segments", "Longest", "Prediction", "Side"] {
                        ui.label(RichText::new(h).small().strong());
                    }
                    ui.end_row();
                    for m in found.iter().take(6) {
                        ui.label(RichText::new(m.name.as_str()).small());
                        ui.label(RichText::new(m.cm.map(|c| format!("{:.0}", c)).unwrap_or_default()).small());
                        ui.label(RichText::new(m.pct.map(|c| format!("{:.2}", c)).unwrap_or_default()).small());
                        ui.label(RichText::new(m.segments.map(|c| c.to_string()).unwrap_or_default()).small());
                        ui.label(RichText::new(m.longest.map(|c| format!("{:.0}", c)).unwrap_or_default()).small());
                        ui.label(RichText::new(m.predicted.as_str()).small());
                        ui.label(RichText::new(m.side.as_str()).small());
                        ui.end_row();
                    }
                });
                if found.iter().all(|m| m.cm.is_none()) && found.iter().any(|m| m.pct.is_some()) {
                    ui.label(RichText::new("This file gives percent shared rather than cM (as 23andMe does). The percent is kept; type the cM in later if you know it.").small().weak());
                }
                ui.separator();
                let skip = found.len() - new.len();
                ui.label(format!("{} new match(es){}", new.len(), if skip > 0 { format!("; {} already in the list will be skipped", skip) } else { String::new() }));
                if ui.add_enabled(!new.is_empty(), egui::Button::new(format!("Add {} match(es)", new.len()))).clicked() {
                    add = Some(new);
                }
            });
            if let Some(list) = add {
                let n = list.len();
                self.db.dna[si].matches.extend(list);
                self.x.next_label = Some(format!("Imported {} DNA match(es)", n));
                self.status = format!("Added {} DNA match(es) from {}.", n, imp.file);
                self.dirty = true;
            } else if open {
                self.x.dna.import = Some(imp);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_after_non_ascii_characters() {
        assert_eq!(percent_in("Sweden\u{2014}45%"), Some(45.0));
        assert_eq!(percent_in("Norway 12\u{a0}%"), Some(12.0));
        assert_eq!(percent_in("\u{e9}<1%"), Some(0.5));
        assert_eq!(percent_in("\u{fffd}%"), None);
        let r = parse_regions("Sweden\u{2014}45%\nNorway 12\u{a0}%");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].name, "Sweden");
        assert_eq!(r[1].pct, 12.0);
    }

    #[test]
    fn relationship_groups_by_shared_cm() {
        assert_eq!(likely(Some(3450.0)), "Parent or child");
        assert_eq!(likely(Some(2600.0)), "Full sibling");
        assert_eq!(likely(Some(1750.0)), "Grandparent / aunt / half sib");
        assert_eq!(likely(Some(880.0)), "1st cousin range");
        assert_eq!(likely(Some(200.0)), "2nd cousin range");
        assert_eq!(likely(Some(8.0)), "4th cousin or further");
        assert_eq!(likely(None), "");
        assert!(likely_groups(600.0).len() >= 2, "600 cM sits between two groups");
    }

    #[test]
    fn numbers_and_percents_in_text() {
        assert_eq!(first_number("1,234.5 cM (17.6%)"), Some(1234.5));
        assert_eq!(percent_in("1,234.5 cM (17.6%)"), Some(17.6));
        assert_eq!(percent_in("<1%"), Some(0.5));
        assert_eq!(first_number("no number"), None);
        assert_eq!(first_number("45 cM."), Some(45.0));
    }

    #[test]
    fn csv_with_quotes_and_line_breaks() {
        let rows = parse_csv("\u{feff}Name,Notes\r\n\"Smith, John\",\"line one\nline two\"\r\nAnn,\"said \"\"hi\"\"\"\n\n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1], vec!["Smith, John".to_string(), "line one\nline two".to_string()]);
        assert_eq!(rows[2][1], "said \"hi\"");
        let tabs = parse_csv("Name\tShared cM\nAnn\t120\n");
        assert_eq!(tabs[1], vec!["Ann".to_string(), "120".to_string()]);
    }

    #[test]
    fn match_files_from_different_companies() {
        // FamilyTreeDNA-style headings
        let rows = parse_csv("Full Name,First Name,Last Name,Relationship Range,Suggested Relationship,Shared DNA,Longest Block,Notes\nJane Doe,Jane,Doe,1st-3rd cousin,2nd cousin,212.5,45.1,met at reunion\n");
        let m = guess_map(&rows[0]);
        assert_eq!((m.name, m.cm, m.longest, m.notes), (Some(0), Some(5), Some(6), Some(7)));
        let got = rows_to_matches(&rows, &m);
        assert_eq!(got[0].name, "Jane Doe");
        assert_eq!(got[0].cm, Some(212.5));
        assert_eq!(got[0].longest, Some(45.1));
        assert_eq!(got[0].predicted, "1st-3rd cousin");
        // a "Shared DNA" cell holding both cM and percent, and first + last names only
        let rows = parse_csv("First name,Last name,Shared DNA,Shared segments,Largest segment\nTom,Lee,\"1,234 cM (17.6%)\",31,98.2\n");
        let m = guess_map(&rows[0]);
        let got = rows_to_matches(&rows, &m);
        assert_eq!(got[0].name, "Tom Lee");
        assert_eq!((got[0].cm, got[0].pct, got[0].segments, got[0].longest), (Some(1234.0), Some(17.6), Some(31), Some(98.2)));
        // percent instead of cM, and a maternal / paternal column
        let rows = parse_csv("Display Name,Sex,Predicted Relationship,Percent DNA Shared,# Segments Shared,Maternal / Paternal\nAl,Male,Second Cousin,3.12%,8,Paternal\nBea,Female,Third Cousin,0.9%,3,M\n");
        let m = guess_map(&rows[0]);
        let got = rows_to_matches(&rows, &m);
        assert_eq!(got.len(), 2);
        assert_eq!((got[0].cm, got[0].pct, got[0].segments), (None, Some(3.12), Some(8)));
        assert_eq!((got[0].side.as_str(), got[1].side.as_str()), ("Paternal", "Maternal"));
        assert_eq!(got[0].predicted, "Second Cousin");
    }

    #[test]
    fn segment_files_add_up_per_person() {
        // 23andMe-style: one row per shared segment, with the match's overall percent on each row
        let rows = parse_csv("Display Name,Chromosome Number,Chromosome Start Point,Chromosome End Point,Genetic Distance,Percent DNA Shared,Predicted Relationship,Haplogroups\nAnn,1,100,900,30.5,1.2%,Third Cousin,H1a\nAnn,7,5,500,12.0,1.2%,Third Cousin,H1a\nBo,2,1,99,8.0,0.1%,Distant Cousin,\n");
        let m = guess_map(&rows[0]);
        assert_eq!((m.chrom, m.cm), (Some(1), Some(4)));
        let got = rows_to_matches(&rows, &m);
        assert_eq!(got.len(), 2);
        assert_eq!((got[0].name.as_str(), got[0].cm, got[0].segments, got[0].longest, got[0].pct), ("Ann", Some(42.5), Some(2), Some(30.5), Some(1.2)));
        assert!(got[0].notes.contains("H1a"));
        assert_eq!((got[1].segments, got[1].cm), (Some(1), Some(8.0)));
    }

    #[test]
    fn excel_workbooks_are_read() {
        // build a tiny .xlsx: shared strings, an inline string, numbers and a gap
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("xl/sharedStrings.xml", o).unwrap();
            z.write_all(br#"<sst><si><t>Name</t></si><si><t>Shared cM</t></si><si><r><t>Smith, </t></r><r><t xml:space="preserve">Ann &amp; Bo</t></r></si></sst>"#).unwrap();
            z.start_file("xl/worksheets/sheet1.xml", o).unwrap();
            z.write_all(br#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c><c r="D1" t="inlineStr"><is><t>Notes</t></is></c></row><row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2"><v>212.5</v></c><c r="D2" t="inlineStr"><is><t>met &lt;2020&gt;</t></is></c></row></sheetData></worksheet>"#).unwrap();
            z.finish().unwrap();
        }
        let rows = xlsx_rows(buf.get_ref()).unwrap();
        assert_eq!(rows[0], vec!["Name", "Shared cM", "", "Notes"]);
        assert_eq!(rows[1], vec!["Smith, Ann & Bo", "212.5", "", "met <2020>"]);
        let dir = std::env::temp_dir().join("gr-xlsx-test");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("m.xlsx");
        std::fs::write(&f, buf.get_ref()).unwrap();
        let text = read_table_text(&f).unwrap();
        let got = rows_to_matches(&parse_csv(&text), &guess_map(&parse_csv(&text)[0]));
        assert_eq!((got[0].name.as_str(), got[0].cm, got[0].notes.as_str()), ("Smith, Ann & Bo", Some(212.5), "met <2020>"));
        // UTF-16 text, as some Windows programs save it
        let mut u16b = vec![0xFF, 0xFE];
        for unit in "Name\tShared cM\nCy\t45\n".encode_utf16() {
            u16b.extend_from_slice(&unit.to_le_bytes());
        }
        let f2 = dir.join("m.txt");
        std::fs::write(&f2, &u16b).unwrap();
        assert!(read_table_text(&f2).unwrap().starts_with("Name\tShared cM"));
        assert!(read_table_text(&dir.join("old.xls")).is_err());
    }

    #[test]
    fn raw_files_are_recognised() {
        assert!(is_raw_dna("# This data file generated by a company\n# rsid\tchromosome\tposition\tgenotype\nrs123\t1\t555\tAG\n"));
        assert!(is_raw_dna("RSID,CHROMOSOME,POSITION,RESULT\n\"rs1\",\"1\",\"2\",\"AA\"\n") );
        assert!(!is_raw_dna("Name,Shared cM\nAnn,45\n"));
    }

    #[test]
    fn myheritage_and_gedmatch_headings() {
        let rows = parse_csv("Name,Age,Country,Estimated relationship,Total cM,Percent shared DNA,Number of shared segments,Largest segment (cM)\nCy Moss,61,USA,2nd cousin,230.4,3.2%,12,48.1\n");
        let got = rows_to_matches(&rows, &guess_map(&rows[0]));
        assert_eq!((got[0].cm, got[0].pct, got[0].segments, got[0].longest), (Some(230.4), Some(3.2), Some(12), Some(48.1)));
        assert_eq!(got[0].predicted, "2nd cousin");
        let rows = parse_csv("Kit,Name,Email,Largest Seg,Total cM,Gen\nT123,Di Ray,x@y.z,22.1,64.0,4.5\n");
        let got = rows_to_matches(&rows, &guess_map(&rows[0]));
        assert_eq!((got[0].name.as_str(), got[0].cm, got[0].longest), ("Di Ray", Some(64.0), Some(22.1)));
    }

    #[test]
    fn pasted_ethnicity() {
        let r = parse_regions("England & Northwestern Europe\n45%\nRange: 30-56%\nScotland 22%\nIreland: 18%\nSweden & Denmark\t<1%\n");
        let names: Vec<&str> = r.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["England & Northwestern Europe", "Scotland", "Ireland", "Sweden & Denmark"]);
        assert_eq!(r.iter().map(|x| x.pct).collect::<Vec<_>>(), vec![45.0, 22.0, 18.0, 0.5]);
        let r = parse_regions("45% Germanic Europe\n12 % Norway");
        assert_eq!(r[0].name, "Germanic Europe");
        assert_eq!(r[1], Region { name: "Norway".into(), pct: 12.0 });
    }

    #[test]
    fn regions_find_their_place() {
        assert_eq!(region_place("Southern Italy"), Some([15.5, 40.0]));
        assert_eq!(region_place("Italy"), Some([12.5, 42.5]));
        assert_eq!(region_place("England and Northwestern Europe"), Some([-1.5, 52.5]));
        assert!(region_place("Scotland").is_some());
        assert!(region_place("Zzzz nowhere").is_none());
    }

    #[test]
    fn dna_tab_draws_with_results() {
        let mut app = App::for_test(sample());
        let me = *app.db.people.keys().next().unwrap();
        app.db.dna.push(DnaTest {
            person: Some(me),
            company: "AncestryDNA".into(),
            regions: parse_regions("England & Northwestern Europe 48%\nScotland 22%\nIreland 18%\nGermanic Europe 9%\nAtlantis 3%"),
            matches: vec![
                DnaMatch { name: "Ann".into(), cm: Some(880.0), side: "Maternal".into(), link: Some(me), ..Default::default() },
                DnaMatch { name: "Bo".into(), pct: Some(1.2), ..Default::default() },
            ],
            ..Default::default()
        });
        app.view = View::Dna;
        let ctx = egui::Context::default();
        for tab in 0..3u8 {
            app.x.dna.tab = tab;
            app.x.dna.details = tab == 0;
            app.x.dna.paste = if tab == 1 { Some("Wales 5%".into()) } else { None };
            app.x.dna.edit = if tab == 2 { Some((0, app.db.dna[0].matches[0].clone())) } else { None };
            let rows = parse_csv("Name,Shared cM\nCy,45\n");
            app.x.dna.import = Some(Import { file: "m.csv".into(), map: guess_map(&rows[0]), rows });
            app.x.dna.est = "300".into();
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                let mut act = None;
                egui::CentralPanel::default().show(ctx, |ui| app.dna_view(ui, &mut act));
            });
        }
        assert_eq!(app.db.dna[0].regions.len(), 5);
    }

    /// Writes the sample family with a DNA test for a look in the app:
    /// cargo test dna_demo_tree -- --ignored
    #[test]
    #[ignore]
    fn dna_demo_tree() {
        let mut db = sample();
        let me = *db.people.keys().next().unwrap();
        db.dna.push(DnaTest {
            person: Some(me),
            company: "AncestryDNA".into(),
            regions: parse_regions("England & Northwestern Europe 41%\nScotland 19%\nIreland 14%\nGermanic Europe 11%\nSweden & Denmark 6%\nNorway 4%\nFrance 3%\nBaltics 2%"),
            matches: vec![
                DnaMatch { name: "Margaret Ellis".into(), cm: Some(1702.0), segments: Some(48), longest: Some(152.0), predicted: "Close family".into(), side: "Maternal".into(), ..Default::default() },
                DnaMatch { name: "R. Walker".into(), cm: Some(866.0), segments: Some(31), longest: Some(98.0), predicted: "1st-2nd cousin".into(), side: "Paternal".into(), ..Default::default() },
                DnaMatch { name: "jbrooks1952".into(), cm: Some(214.0), segments: Some(11), longest: Some(41.0), predicted: "2nd-3rd cousin".into(), ..Default::default() },
                DnaMatch { name: "Karen Lind".into(), cm: Some(58.0), segments: Some(4), longest: Some(22.0), predicted: "4th-6th cousin".into(), side: "Maternal".into(), ..Default::default() },
                DnaMatch { name: "T.M.".into(), cm: Some(21.0), segments: Some(2), longest: Some(13.0), predicted: "5th-8th cousin".into(), ..Default::default() },
            ],
            ..Default::default()
        });
        let deep = db.people.keys().copied().max_by_key(|id| dnaline::direct_line(&db, *id, true).len()).unwrap();
        db.dna.push(DnaTest {
            person: Some(deep),
            company: "FamilyTreeDNA".into(),
            kind: 1,
            line: LineTest {
                haplogroup: "R-FGC11134".into(),
                level: "Y-37".into(),
                markers: dnaline::parse_markers("DYS393,DYS390,DYS19,DYS391,DYS385,DYS426,DYS388,DYS439,DYS389I,DYS392,DYS389II,DYS458\n13,24,14,11,11-14,12,12,12,13,13,29,17"),
                mutations: String::new(),
                matches: vec![
                    LineMatch { name: "D. Harwood".into(), distance: Some(1), compared: "37".into(), haplogroup: "R-FGC11134".into(), ancestor: "John Harwood, b. 1788 Yorkshire".into(), ..Default::default() },
                    LineMatch { name: "M. Harward".into(), distance: Some(3), compared: "37".into(), haplogroup: "R-M269".into(), ancestor: "Thomas Harward, b. 1720".into(), ..Default::default() },
                ],
            },
            ..Default::default()
        });
        db.sample = false;
        std::fs::write(std::env::temp_dir().join("gr-dna-tree.json"), serde_json::to_string(&db).unwrap()).unwrap();
    }

    #[test]
    fn old_trees_load_without_dna() {
        let db: Db = serde_json::from_str("{\"people\":{},\"fams\":{}}").unwrap();
        assert!(db.dna.is_empty());
        let mut db = Db::default();
        db.dna.push(DnaTest { company: "23andMe".into(), regions: vec![Region { name: "Ireland".into(), pct: 40.0 }], matches: vec![DnaMatch { name: "Ann".into(), cm: Some(99.0), ..Default::default() }], ..Default::default() });
        let back: Db = serde_json::from_str(&serde_json::to_string(&db).unwrap()).unwrap();
        assert!(back.dna == db.dna);
    }
}
