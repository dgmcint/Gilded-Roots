#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, RichText, Rounding, Sense, Stroke};
use serde::{Deserialize, Serialize};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

mod archive;
mod census;
mod forms;
mod charts;
mod designer;
mod dna;
mod dnaline;
mod stats;
mod journal;
mod mapview;
mod pdfview;
mod table;
mod tombstone;
mod paf;
mod gedcom;
mod review;
mod sample;
use sample::sample_family;
use charts::ChartKind;
use gedcom::*;
use review::{ImportMerge, PersonReview};

/// A piece of GEDCOM the app has no field for. It is kept exactly as read
/// (as GEDCOM lines, levels relative to where it belongs) and written back on export.
/// `at` names the sub-block it sits under ("" = directly under its parent).
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Raw {
    at: String,
    text: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Media {
    file: String,
    name: String,
    caption: String,
    thumb: String,
    mid: String,
    /// The file reference as it appeared in a GEDCOM file (kept when the file itself was not found).
    path: String,
    form: String,
    /// People tagged in this picture.
    faces: Vec<Face>,
    extra: Vec<Raw>,
}

/// A box drawn on a photo around one person's face. `r` is x, y, width, height as parts of the
/// picture (0 to 1), so it stays right whatever size the photo is shown at.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
struct Face {
    who: u32,
    r: [f32; 4],
}

/// A profile picture cut from a tagged face in someone's photo.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
struct FacePortrait {
    owner: u32,
    file: String,
    r: [f32; 4],
}

/// One event or fact. Used for life events, attributes, family events and the
/// extra details (notes, causes, ages...) that hang off birth, death, marriage and so on.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Event {
    kind: String,
    date: String,
    place: String,
    /// Free-text note.
    note: String,
    /// The value on the event line itself (an occupation, a religion, "Y"...).
    val: String,
    typ: String,
    cause: String,
    age: String,
    agency: String,
    /// LDS temple code and ordinance status.
    temp: String,
    stat: String,
    /// The original GEDCOM tag, when it is not one this app knows by name.
    tag: String,
    /// How sure the researcher is of this fact: confirmed, supported, probable, uncertain, disputed.
    conf: String,
    cites: Vec<Source>,
    note_refs: Vec<u32>,
    extra: Vec<Raw>,
}

/// The research confidence scale, strongest first: (stored word, label, colour).
const CONFIDENCE: [(&str, &str, [u8; 3]); 5] = [
    ("confirmed", "Confirmed by original records", [56, 142, 60]),
    ("supported", "Supported by several sources", [40, 120, 170]),
    ("probable", "Probable", [196, 152, 40]),
    ("uncertain", "Uncertain", [220, 120, 40]),
    ("disputed", "Disputed", [200, 60, 60]),
];

/// A sound-alike key for a surname (American Soundex), so spelling variants group together.
fn sound_key(s: &str) -> String {
    let letters: Vec<char> = s.chars().filter(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_uppercase()).collect();
    let Some(first) = letters.first().copied() else { return String::new() };
    let code = |c: char| match c {
        'B' | 'F' | 'P' | 'V' => '1',
        'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
        'D' | 'T' => '3',
        'L' => '4',
        'M' | 'N' => '5',
        'R' => '6',
        'H' | 'W' => '-',
        _ => '0',
    };
    let mut out = String::from(first);
    let mut last = code(first);
    for &c in &letters[1..] {
        let k = code(c);
        if k == '-' {
            continue;
        }
        if k != '0' && k != last {
            out.push(k);
            if out.len() == 4 {
                break;
            }
        }
        last = k;
    }
    while out.len() < 4 {
        out.push('0');
    }
    out
}

fn first_given(g: &str) -> String {
    g.split_whitespace().next().unwrap_or("").to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// The formal name behind a common nickname ("Bill" -> "william").
fn canon_given(g: &str) -> String {
    const NICK: &[(&str, &[&str])] = &[
        ("william", &["bill", "billy", "will", "willie", "willy", "wm"]),
        ("elizabeth", &["eliza", "beth", "betty", "betsy", "bess", "bessie", "liz", "lizzie", "libby", "elisabeth"]),
        ("margaret", &["maggie", "peggy", "meg", "madge", "marge", "margie", "greta", "daisy"]),
        ("john", &["jack", "johnny", "jno"]),
        ("mary", &["polly", "molly", "mae", "mamie", "maria", "marie"]),
        ("james", &["jim", "jimmy", "jas", "jamie"]),
        ("robert", &["bob", "bobby", "rob", "robbie", "bert"]),
        ("thomas", &["tom", "tommy", "thos"]),
        ("richard", &["dick", "rick", "rich"]),
        ("catherine", &["kate", "katie", "kathy", "katherine", "kathryn", "kitty", "cathy"]),
        ("ann", &["anne", "anna", "annie", "nancy", "nan"]),
        ("sarah", &["sally", "sadie", "sara"]),
        ("henry", &["harry", "hank", "hal"]),
        ("charles", &["charlie", "chas", "chuck"]),
        ("edward", &["ed", "eddie", "ted", "ned"]),
        ("frederick", &["fred", "freddie", "fritz"]),
        ("joseph", &["joe", "joey", "jos"]),
        ("benjamin", &["ben", "benny"]),
        ("samuel", &["sam", "sammy"]),
        ("alexander", &["alex", "sandy", "alec"]),
        ("susan", &["sue", "susie", "susanna", "susannah"]),
        ("dorothy", &["dot", "dolly", "dottie"]),
        ("helen", &["nell", "nellie", "ellen"]),
        ("frances", &["fanny", "frankie"]),
        ("francis", &["frank"]),
        ("lawrence", &["larry", "laurence"]),
        ("patrick", &["pat", "paddy"]),
        ("daniel", &["dan", "danny"]),
        ("matilda", &["tilly", "mattie"]),
        ("christopher", &["chris", "kit"]),
    ];
    let g = g.to_lowercase();
    for (full, nicks) in NICK {
        if g == *full || nicks.contains(&g.as_str()) {
            return full.to_string();
        }
    }
    g
}

/// How many single-letter changes turn one word into the other.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + if a[i - 1] == b[j - 1] { 0 } else { 1 });
        }
        prev = cur;
    }
    prev[b.len()]
}

/// A small coloured label showing how sure a fact is (nothing when it is not rated).
fn conf_badge(ui: &mut egui::Ui, v: &str) {
    if let Some(c) = CONFIDENCE.iter().find(|c| c.0 == v.trim()) {
        let col = Color32::from_rgb(c.2[0], c.2[1], c.2[2]);
        egui::Frame::none().fill(col.gamma_multiply(0.18)).stroke(Stroke::new(1.0_f32, col)).rounding(8.0).inner_margin(egui::Margin::symmetric(6.0, 1.0)).show(ui, |ui| {
            ui.label(RichText::new(c.1).small().color(col));
        });
    } else {
        ui.label("");
    }
}

/// A drop-down to choose how sure a fact is. Returns true when it changed.
fn conf_combo(ui: &mut egui::Ui, id: impl std::hash::Hash, v: &mut String) -> bool {
    let mut changed = false;
    let shown = conf_label(v).unwrap_or("How sure? (not rated)");
    egui::ComboBox::from_id_salt(id).selected_text(RichText::new(shown).small()).width(190.0).show_ui(ui, |ui| {
        if ui.selectable_label(v.is_empty(), "Not rated").clicked() {
            v.clear();
            changed = true;
        }
        for c in CONFIDENCE {
            if ui.selectable_label(v == c.0, c.1).clicked() {
                *v = c.0.to_string();
                changed = true;
            }
        }
    });
    changed
}

fn conf_label(v: &str) -> Option<&'static str> {
    let v = v.trim().to_lowercase();
    CONFIDENCE.iter().find(|c| c.0 == v).map(|c| c.1)
}

/// One entry in the research journal: what was searched, where, and what came of it.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Journal {
    /// The day of the search, as typed.
    date: String,
    /// What was looked for.
    goal: String,
    /// Where: an archive, a website, a book, a person.
    place: String,
    /// The source searched, when it is one in the tree.
    sid: Option<u32>,
    /// "found", "nothing", "partial" or "" (not yet).
    result: String,
    /// What was discovered (or not).
    notes: String,
    /// People this search was about.
    people: Vec<u32>,
}

const EVENT_KINDS: &[&str] = &[
    "Residence", "Occupation", "Education", "Graduation", "Religion", "Title", "Nationality", "Physical description", "Possessions", "Fact",
    "Immigration", "Emigration", "Naturalization", "Census", "Retirement", "Adoption", "Baptism", "Adult christening", "Blessing", "Confirmation",
    "First communion", "Ordination", "Bar Mitzvah", "Bas Mitzvah", "Cremation", "Probate", "Will", "Caste", "ID number", "Social security number",
    "Number of children", "Number of marriages", "Christening", "Birth (additional)", "Death (additional)", "Burial (additional)",
    "LDS baptism", "LDS confirmation", "LDS initiatory", "LDS endowment", "LDS child sealing",
    "Military service", "Divorce", "Other",
];
/// GEDCOM tag and the label shown in the app, for events and facts of a person.
const EVENT_TAGS: &[(&str, &str)] = &[
    ("RESI", "Residence"), ("OCCU", "Occupation"), ("EDUC", "Education"), ("GRAD", "Graduation"), ("RELI", "Religion"), ("TITL", "Title"),
    ("NATI", "Nationality"), ("DSCR", "Physical description"), ("PROP", "Possessions"), ("FACT", "Fact"),
    ("IMMI", "Immigration"), ("EMIG", "Emigration"), ("NATU", "Naturalization"), ("CENS", "Census"), ("RETI", "Retirement"),
    ("ADOP", "Adoption"), ("BAPM", "Baptism"), ("CHRA", "Adult christening"), ("BLES", "Blessing"), ("CONF", "Confirmation"),
    ("FCOM", "First communion"), ("ORDN", "Ordination"), ("BARM", "Bar Mitzvah"), ("BASM", "Bas Mitzvah"), ("CREM", "Cremation"),
    ("PROB", "Probate"), ("WILL", "Will"), ("CAST", "Caste"), ("IDNO", "ID number"), ("SSN", "Social security number"),
    ("NCHI", "Number of children"), ("NMR", "Number of marriages"), ("CHR", "Christening"), ("BIRT", "Birth (additional)"),
    ("DEAT", "Death (additional)"), ("BURI", "Burial (additional)"),
    ("BAPL", "LDS baptism"), ("CONL", "LDS confirmation"), ("INIT", "LDS initiatory"), ("ENDL", "LDS endowment"), ("SLGC", "LDS child sealing"),
];
/// The same for events of a family.
const FAM_TAGS: &[(&str, &str)] = &[
    ("DIV", "Divorce"), ("DIVF", "Divorce filed"), ("ENGA", "Engagement"), ("MARB", "Marriage banns"), ("MARC", "Marriage contract"),
    ("MARL", "Marriage license"), ("MARS", "Marriage settlement"), ("ANUL", "Annulment"), ("MARR", "Marriage (additional)"),
    ("CENS", "Census"), ("RESI", "Residence"), ("SLGS", "LDS spouse sealing"), ("NCHI", "Number of children"),
];
const FAM_KINDS: &[&str] = &[
    "Divorce", "Divorce filed", "Engagement", "Marriage banns", "Marriage contract", "Marriage license", "Marriage settlement", "Annulment",
    "Marriage (additional)", "Census", "Residence", "LDS spouse sealing", "Number of children", "Other",
];

/// A citation: a reference to a source, with the page and details that go with it.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Source {
    about: String,
    /// Shown text. For a citation of a source record this is that record's title.
    text: String,
    /// The source record cited, if any.
    sid: Option<u32>,
    page: String,
    /// Text copied from the source (a transcription), and its date.
    data: String,
    ddate: String,
    /// A web address for this particular record, and the day it was looked at.
    url: String,
    accessed: String,
    quality: String,
    note: String,
    note_refs: Vec<u32>,
    extra: Vec<Raw>,
}

/// A source record (a book, a census, a website...) that citations point to.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct SourceRec {
    title: String,
    author: String,
    abbr: String,
    publ: String,
    /// What kind of source it is: Book, Census, Vital record, Church record, Website...
    kind: String,
    /// Web address of the source as a whole.
    url: String,
    /// Where the source is kept, and its call number there.
    repo: Option<u32>,
    call: String,
    medium: String,
    text: String,
    notes: String,
    note_refs: Vec<u32>,
    extra: Vec<Raw>,
}

/// A repository: the library, archive, website or person that holds sources.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct RepoRec {
    name: String,
    addr: String,
    url: String,
    email: String,
    phone: String,
    notes: String,
    note_refs: Vec<u32>,
    extra: Vec<Raw>,
}

const SOURCE_KINDS: &[&str] = &[
    "Book", "Census", "Vital record", "Church record", "Military record", "Land or probate record", "Immigration record", "Newspaper", "Website",
    "Database", "Family papers", "Interview", "Photograph", "Cemetery record", "Letter or diary", "Other",
];

/// A note that is shared between several people, families or sources.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct NoteRec {
    text: String,
    extra: Vec<Raw>,
}

/// A second (or third...) name: birth name, married name, nickname, religious name.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct AltName {
    kind: String,
    given: String,
    surname: String,
    prefix: String,
    suffix: String,
    nick: String,
    note: String,
    note_refs: Vec<u32>,
    cites: Vec<Source>,
    extra: Vec<Raw>,
}

const FACTS: [&str; 6] = ["General", "Birth", "Christening", "Death", "Burial", "Marriage"];

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Person {
    name: String,
    given: String,
    surname: String,
    maiden: String,
    living: bool,
    sex: String,
    bd: String,
    bp: String,
    chd: String,
    chp: String,
    dd: String,
    dp: String,
    bud: String,
    bup: String,
    notes: String,
    media: Vec<Media>,
    sources: Vec<Source>,
    events: Vec<Event>,
    portrait: String,
    prefix: String,
    suffix: String,
    nick: String,
    alt_names: Vec<AltName>,
    /// Shared notes this person points to.
    note_refs: Vec<u32>,
    /// "BAPM" when the christening above was recorded as a baptism.
    chtag: String,
    /// Original values of single-line facts that carry one ("SEX" other than M/F, "BIRT" = "Y").
    fvals: BTreeMap<String, String>,
    /// Extra details of NAME, BIRT, CHR, DEAT and BURI (notes, causes, ages...).
    fx: BTreeMap<String, Event>,
    /// The family whose parents the chart shows, when the person belongs to more than one.
    pref_parents: Option<u32>,
    /// Ties to other people outside the family structure: guardian, godparent, witness...
    assoc: Vec<Assoc>,
    /// Research tasks for this person.
    todos: Vec<Todo>,
    /// Profile picture cut from a face tag (takes the place of `portrait` when set).
    face_portrait: Option<FacePortrait>,
    /// The part of the profile photo to show (x, y, width, height as parts of the picture).
    portrait_crop: Option<[f32; 4]>,
    extra: Vec<Raw>,
}

/// A research task: something still to find out or do for a person.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Todo {
    text: String,
    done: bool,
    /// The day it was written down, as "2026-10-29".
    added: String,
    note: String,
}

/// "`other` is this person's `role`": a guardian, godparent, mentor, witness...
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Assoc {
    other: u32,
    role: String,
    note: String,
}

/// How a child can be related to a parent. The stored word is the one other programs use.
/// Optional donation page (opened in the browser; the app never handles payments).
const DONATE_URL: &str = "https://www.paypal.me/doylemcintosh";
/// Where people report problems and ask questions: the GitHub repository's Issues page.
/// Left empty until the repository's address is filled in here.
const PROJECT_URL: &str = "";
const NOT_AFFILIATED: &str = "Gilded Roots is an independent project. It is not affiliated with, endorsed by or connected to FamilySearch, The Church of Jesus Christ of Latter-day Saints, Ancestry, MyHeritage, RootsMagic or any other company named in it. Personal Ancestral File (PAF) and the other product names are trademarks of their owners and are used only to describe compatibility.";
/// The licences of every library the program is built from (generated from Cargo.lock).
const THIRD_PARTY: &str = include_str!("../THIRD_PARTY_LICENSES.txt");
const PARENT_RELS: &[(&str, &str)] = &[
    ("Natural", "Biological"),
    ("Adopted", "Adopted"),
    ("Step", "Step-parent"),
    ("Foster", "Foster"),
    ("Guardian", "Guardian"),
    ("Sealed", "Sealed (LDS)"),
];

/// What kind of partnership a family record is (Gramps-style words).
const PARTNER_KINDS: &[&str] = &["Married", "Unmarried", "Civil Union", "Unknown"];

const ASSOC_ROLES: &[&str] = &["Guardian", "Godparent", "Foster parent", "Step-parent", "Mentor", "Employer", "Witness", "Neighbor", "Friend", "Associate"];

fn rel_is_bio(s: &str) -> bool {
    matches!(s.trim().to_lowercase().as_str(), "" | "natural" | "biological" | "birth")
}

fn rel_name(s: &str) -> String {
    match PARENT_RELS.iter().find(|(k, _)| k.eq_ignore_ascii_case(s.trim())) {
        Some((_, v)) => v.to_string(),
        None if rel_is_bio(s) => "Biological".to_string(),
        None => s.to_string(),
    }
}

#[derive(Default)]
struct Idx {
    child_of: HashMap<u32, Vec<u32>>,
    partner_in: HashMap<u32, Vec<u32>>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Family {
    h: Option<u32>,
    w: Option<u32>,
    md: String,
    mp: String,
    kids: Vec<u32>,
    /// More partners than the usual one husband and one wife, with their GEDCOM role.
    more: Vec<(String, u32)>,
    /// How each child is related to this family: "adopted", "foster", "sealing", "birth".
    pedi: BTreeMap<u32, String>,
    /// What kind of partnership this is (Married, Unmarried, Civil Union...). Empty when not said.
    kind: String,
    /// How a child is related to the father / mother slot when that differs from `pedi`
    /// (a child born to the mother and adopted by her husband, for instance).
    frel: BTreeMap<u32, String>,
    mrel: BTreeMap<u32, String>,
    /// Details of the marriage other than its date and place.
    mx: Event,
    events: Vec<Event>,
    cites: Vec<Source>,
    notes: String,
    note_refs: Vec<u32>,
    media: Vec<Media>,
    fvals: BTreeMap<String, String>,
    extra: Vec<Raw>,
}

#[derive(Default)]
struct Kin {
    step_parents: Vec<u32>,
    half_siblings: Vec<u32>,
    step_siblings: Vec<u32>,
    step_children: Vec<u32>,
    /// People who list this person as their guardian, godparent...: (who, role).
    assoc_of: Vec<(u32, String)>,
}

impl Family {
    /// How `child` is related to the father slot (`father`) or the mother slot: Natural, Adopted, Step...
    /// An empty answer means nothing was recorded, which is read as biological.
    fn rel_to(&self, child: u32, father: bool) -> String {
        let own = if father { self.frel.get(&child) } else { self.mrel.get(&child) };
        if let Some(s) = own {
            if !s.is_empty() {
                return s.clone();
            }
        }
        match self.pedi.get(&child).map(|s| s.to_lowercase()).as_deref() {
            Some("adopted") => "Adopted".into(),
            Some("foster") => "Foster".into(),
            Some("sealing") | Some("sealed") => "Sealed".into(),
            _ => String::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Db {
    #[serde(default)]
    people: BTreeMap<u32, Person>,
    #[serde(default)]
    fams: BTreeMap<u32, Family>,
    #[serde(default)]
    next: u32,
    #[serde(default)]
    sample: bool,
    #[serde(default)]
    sources: BTreeMap<u32, SourceRec>,
    #[serde(default)]
    repos: BTreeMap<u32, RepoRec>,
    #[serde(default)]
    notes: BTreeMap<u32, NoteRec>,
    /// Whole GEDCOM records with no place in the model (repositories, submitter, custom records).
    #[serde(default)]
    records: Vec<String>,
    /// People the user bookmarked, in the order they were added.
    #[serde(default)]
    bookmarks: Vec<u32>,
    /// The research journal, newest last.
    #[serde(default)]
    journal: Vec<Journal>,
    /// Historical events the user added or changed for timelines (the built-in list is used otherwise).
    #[serde(default)]
    history: Vec<(i32, i32, String, String)>,
    /// Map positions (longitude, latitude) the user set for places the map could not find.
    #[serde(default)]
    place_coords: BTreeMap<String, [f32; 2]>,
    /// Autosomal DNA tests: ethnicity estimates and match lists (the DNA tab).
    #[serde(default)]
    dna: Vec<dna::DnaTest>,
    /// Data layout version, so files saved by older versions are upgraded once.
    /// A file with no version is an old one (0); new trees start at the current version.
    #[serde(default)]
    ver: u32,
}

impl Default for Db {
    fn default() -> Self {
        Db { people: BTreeMap::new(), fams: BTreeMap::new(), next: 0, sample: false, sources: BTreeMap::new(), repos: BTreeMap::new(), notes: BTreeMap::new(), records: Vec::new(), bookmarks: Vec::new(), journal: Vec::new(), history: Vec::new(), place_coords: BTreeMap::new(), dna: Vec::new(), ver: 4 }
    }
}

/// Who a citation belongs to.
#[derive(Clone, Copy, PartialEq)]
enum Owner {
    Person(u32),
    Family(u32),
}

/// What a citation backs up: a named fact ("Birth") or an event (kind and date).
struct Claim<'a> {
    what: &'a str,
    when: &'a str,
}

fn source_title(s: &SourceRec) -> String {
    if !s.title.is_empty() {
        s.title.clone()
    } else if !s.abbr.is_empty() {
        s.abbr.clone()
    } else {
        "(untitled source)".to_string()
    }
}

fn quality_label(q: &str) -> &'static str {
    match q.trim() {
        "0" => "0 - unreliable",
        "1" => "1 - questionable",
        "2" => "2 - secondary evidence",
        "3" => "3 - direct, primary evidence",
        _ => "not rated",
    }
}

impl Db {
    /// Visit every citation in the tree with its owner and what it supports.
    fn each_cite(&self, f: &mut dyn FnMut(Owner, Claim, &Source)) {
        for (id, p) in &self.people {
            let o = Owner::Person(*id);
            for c in &p.sources {
                f(o, Claim { what: if c.about.is_empty() { "General" } else { c.about.as_str() }, when: "" }, c);
            }
            for (k, e) in &p.fx {
                for c in &e.cites {
                    f(o, Claim { what: k.as_str(), when: e.date.as_str() }, c);
                }
            }
            for e in &p.events {
                for c in &e.cites {
                    f(o, Claim { what: e.kind.as_str(), when: e.date.as_str() }, c);
                }
            }
            for a in &p.alt_names {
                for c in &a.cites {
                    f(o, Claim { what: "Other name", when: "" }, c);
                }
            }
        }
        for (id, fm) in &self.fams {
            let o = Owner::Family(*id);
            for c in &fm.cites {
                f(o, Claim { what: "Family", when: "" }, c);
            }
            for c in &fm.mx.cites {
                f(o, Claim { what: "Marriage", when: fm.md.as_str() }, c);
            }
            for e in &fm.events {
                for c in &e.cites {
                    f(o, Claim { what: e.kind.as_str(), when: e.date.as_str() }, c);
                }
            }
        }
    }

    fn each_cite_mut(&mut self, f: &mut dyn FnMut(&mut Source)) {
        for p in self.people.values_mut() {
            p.sources.iter_mut().for_each(&mut *f);
            for e in p.fx.values_mut() {
                e.cites.iter_mut().for_each(&mut *f);
            }
            for e in p.events.iter_mut() {
                e.cites.iter_mut().for_each(&mut *f);
            }
            for a in p.alt_names.iter_mut() {
                a.cites.iter_mut().for_each(&mut *f);
            }
        }
        for fm in self.fams.values_mut() {
            fm.cites.iter_mut().for_each(&mut *f);
            fm.mx.cites.iter_mut().for_each(&mut *f);
            for e in fm.events.iter_mut() {
                e.cites.iter_mut().for_each(&mut *f);
            }
        }
    }

    fn source_counts(&self) -> HashMap<u32, usize> {
        let mut n: HashMap<u32, usize> = HashMap::new();
        self.each_cite(&mut |_, _, c| {
            if let Some(s) = c.sid {
                *n.entry(s).or_insert(0) += 1;
            }
        });
        n
    }

    /// Everything that cites a source: (who, what it supports, date, page).
    fn source_uses(&self, sid: u32) -> Vec<(Owner, String, String, String)> {
        let mut out = Vec::new();
        self.each_cite(&mut |o, cl, c| {
            if c.sid == Some(sid) {
                out.push((o, cl.what.to_string(), cl.when.to_string(), c.page.clone()));
            }
        });
        out
    }

    fn owner_label(&self, o: Owner) -> String {
        match o {
            Owner::Person(i) => self.people.get(&i).map(|_| self.label(i)).unwrap_or_default(),
            Owner::Family(i) => match self.fams.get(&i) {
                Some(f) => {
                    let n = |x: Option<u32>| x.and_then(|i| self.people.get(&i)).map(display_name).unwrap_or_else(|| "?".to_string());
                    format!("Family: {} and {}", n(f.h), n(f.w))
                }
                None => String::new(),
            },
        }
    }

    /// A citation written out in full: source, page, where it is kept, web address and access date.
    fn cite_text(&self, c: &Source) -> String {
        let mut parts: Vec<String> = Vec::new();
        match c.sid.and_then(|i| self.sources.get(&i)) {
            Some(s) => {
                let mut head = source_title(s);
                if !s.author.is_empty() {
                    head = format!("{}, {}", s.author, head);
                }
                if !s.publ.is_empty() {
                    head = format!("{} ({})", head, s.publ);
                }
                parts.push(head);
                if !c.page.is_empty() {
                    parts.push(c.page.clone());
                }
                if let Some(r) = s.repo.and_then(|r| self.repos.get(&r)) {
                    let mut w = r.name.clone();
                    if !s.call.is_empty() {
                        w = format!("{}, {}", w, s.call);
                    }
                    parts.push(w);
                } else if !s.call.is_empty() {
                    parts.push(s.call.clone());
                }
                let url = if !c.url.is_empty() { &c.url } else { &s.url };
                if !url.is_empty() {
                    parts.push(url.clone());
                }
            }
            None => {
                parts.push(c.text.clone());
                if !c.page.is_empty() {
                    parts.push(c.page.clone());
                }
                if !c.url.is_empty() {
                    parts.push(c.url.clone());
                }
            }
        }
        let mut s = parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(". ");
        if !c.accessed.is_empty() {
            s.push_str(&format!(" (accessed {})", c.accessed));
        }
        s
    }

    fn delete_source(&mut self, id: u32) {
        let title = self.sources.get(&id).map(source_title).unwrap_or_default();
        self.each_cite_mut(&mut |c| {
            if c.sid == Some(id) {
                c.sid = None;
                c.text = title.clone();
            }
        });
        self.sources.remove(&id);
        replace_pointer(self, &format!("S{}", id), None);
    }

    /// Fold source `gone` into `keep`: every citation of `gone` now cites `keep`.
    fn merge_sources(&mut self, keep: u32, gone: u32) {
        if keep == gone {
            return;
        }
        let Some(g) = self.sources.remove(&gone) else { return };
        let title = self.sources.get(&keep).map(source_title).unwrap_or_default();
        self.each_cite_mut(&mut |c| {
            if c.sid == Some(gone) {
                c.sid = Some(keep);
                c.text = title.clone();
            }
        });
        if let Some(k) = self.sources.get_mut(&keep) {
            let fill = |a: &mut String, b: String| {
                if a.is_empty() {
                    *a = b;
                }
            };
            fill(&mut k.title, g.title);
            fill(&mut k.author, g.author);
            fill(&mut k.abbr, g.abbr);
            fill(&mut k.publ, g.publ);
            fill(&mut k.kind, g.kind);
            fill(&mut k.url, g.url);
            fill(&mut k.call, g.call);
            fill(&mut k.medium, g.medium);
            fill(&mut k.text, g.text);
            if k.repo.is_none() {
                k.repo = g.repo;
            }
            if !g.notes.is_empty() && !k.notes.contains(&g.notes) {
                if !k.notes.is_empty() {
                    k.notes.push('\n');
                }
                k.notes.push_str(&g.notes);
            }
            for n in g.note_refs {
                if !k.note_refs.contains(&n) {
                    k.note_refs.push(n);
                }
            }
            k.extra.extend(g.extra);
        }
        replace_pointer(self, &format!("S{}", gone), Some(&format!("S{}", keep)));
    }

    fn delete_repo(&mut self, id: u32) {
        for s in self.sources.values_mut() {
            if s.repo == Some(id) {
                s.repo = None;
            }
        }
        self.repos.remove(&id);
        replace_pointer(self, &format!("R{}", id), None);
    }
}

fn split_name(name: &str) -> (String, String) {
    let w: Vec<&str> = name.split_whitespace().collect();
    if w.len() > 1 {
        (w[..w.len() - 1].join(" "), w[w.len() - 1].to_string())
    } else {
        (name.trim().to_string(), String::new())
    }
}

fn compose_name(given: &str, surname: &str) -> String {
    format!("{} {}", given.trim(), surname.trim()).trim().to_string()
}

fn fix_names(db: &mut Db) {
    for p in db.people.values_mut() {
        if p.given.is_empty() && p.surname.is_empty() && !p.name.is_empty() {
            let (g, s) = split_name(&p.name);
            p.given = g;
            p.surname = s;
        }
    }
}

/// How names and dates are shown on screen (View > Names and dates). Display only: what is
/// stored and exported never changes.
static NAME_STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
static DATE_STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// The map colours chosen under Map style (see mapview::MAP_STYLES).
static MAP_STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// The app's background and accent colours (0xRRGGBB), so "Match the app" maps follow them.
static APP_BG: AtomicU32 = AtomicU32::new(0x1e1f22);
static APP_ACCENT: AtomicU32 = AtomicU32::new(0x3b6fd4);

/// A name in the chosen style: "John Henry Smith", "SMITH, John Henry" or "Smith, John Henry".
fn styled_name(p: &Person) -> String {
    styled_name_as(p, NAME_STYLE.load(Ordering::Relaxed))
}

fn styled_name_as(p: &Person, style: u8) -> String {
    if style == 0 || p.surname.trim().is_empty() {
        return if p.name.is_empty() { "Unnamed".to_string() } else { p.name.clone() };
    }
    let sur = if style == 1 { p.surname.trim().to_uppercase() } else { p.surname.trim().to_string() };
    if p.given.trim().is_empty() {
        sur
    } else {
        format!("{}, {}", sur, p.given.trim())
    }
}

/// A date in the chosen style. Dates with words in them ("about 1850", "between...") are left
/// as written; only plain day-month-year dates are re-written.
fn show_date(s: &str) -> String {
    show_date_as(s, DATE_STYLE.load(Ordering::Relaxed))
}

fn show_date_as(s: &str, style: u8) -> String {
    let t = s.trim();
    if style == 0 || t.is_empty() {
        return t.to_string();
    }
    const FULL: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const SHORT: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let words = t.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).count();
    let (y, m, d) = parse_date(t);
    match (y, m, d) {
        (Some(y), Some(m), Some(d)) if words == 3 && (1..=12).contains(&m) => match style {
            1 => format!("{} {} {}", d, SHORT[m as usize - 1], y),
            2 => format!("{} {}, {}", FULL[m as usize - 1], d, y),
            _ => format!("{:04}-{:02}-{:02}", y, m, d),
        },
        (Some(y), Some(m), None) if words == 2 && (1..=12).contains(&m) => match style {
            1 => format!("{} {}", SHORT[m as usize - 1], y),
            2 => format!("{} {}", FULL[m as usize - 1], y),
            _ => format!("{:04}-{:02}", y, m),
        },
        _ => t.to_string(),
    }
}

fn display_name(p: &Person) -> String {
    let n = styled_name(p);
    if p.maiden.is_empty() { n } else { format!("{} (née {})", n, p.maiden) }
}

fn pick_sex<'a>(sex: &str, neutral: &'a str, male: &'a str, female: &'a str) -> &'a str {
    match sex {
        "M" => male,
        "F" => female,
        _ => neutral,
    }
}

fn legal_parent_word(rel: &str, sx: &str) -> String {
    match rel.to_lowercase().as_str() {
        "adopted" => format!("adoptive {}", pick_sex(sx, "parent", "father", "mother")),
        "step" => pick_sex(sx, "step-parent", "stepfather", "stepmother").to_string(),
        "foster" => format!("foster {}", pick_sex(sx, "parent", "father", "mother")),
        "guardian" => "legal guardian".to_string(),
        "sealed" => format!("sealed {}", pick_sex(sx, "parent", "father", "mother")),
        other => format!("{} ({})", pick_sex(sx, "parent", "father", "mother"), other),
    }
}

fn legal_child_word(rel: &str, sx: &str) -> String {
    match rel.to_lowercase().as_str() {
        "adopted" => format!("adopted {}", pick_sex(sx, "child", "son", "daughter")),
        "step" => pick_sex(sx, "stepchild", "stepson", "stepdaughter").to_string(),
        "foster" => format!("foster {}", pick_sex(sx, "child", "son", "daughter")),
        "guardian" => "ward".to_string(),
        "sealed" => format!("sealed {}", pick_sex(sx, "child", "son", "daughter")),
        other => format!("{} ({})", pick_sex(sx, "child", "son", "daughter"), other),
    }
}

/// Name a relationship from distances to the nearest common ancestor.
fn kin_term(da: i32, db: i32, sx: &str, half: bool) -> String {
    if da == 0 {
        if db == 1 {
            pick_sex(sx, "child", "son", "daughter").to_string()
        } else {
            format!("{}{}", greats(db - 2), pick_sex(sx, "grandchild", "grandson", "granddaughter"))
        }
    } else if db == 0 {
        if da == 1 {
            pick_sex(sx, "parent", "father", "mother").to_string()
        } else {
            format!("{}{}", greats(da - 2), pick_sex(sx, "grandparent", "grandfather", "grandmother"))
        }
    } else if da == 1 && db == 1 {
        format!("{}{}", if half { "half-" } else { "" }, pick_sex(sx, "sibling", "brother", "sister"))
    } else if da == 1 {
        format!("{}{}", greats(db - 2), pick_sex(sx, "niece or nephew", "nephew", "niece"))
    } else if db == 1 {
        format!("{}{}", greats(da - 2), pick_sex(sx, "aunt or uncle", "uncle", "aunt"))
    } else {
        let k = da.min(db) - 1;
        let removed = (da - db).abs();
        let rm = match removed {
            0 => String::new(),
            1 => " once removed".to_string(),
            2 => " twice removed".to_string(),
            n => format!(" {} times removed", n),
        };
        format!("{} cousin{}", ordinal(k as usize), rm)
    }
}

fn greats(n: i32) -> String {
    match n {
        n if n <= 0 => String::new(),
        1 => "great-".to_string(),
        n => format!("{} great-", ordinal(n as usize)),
    }
}

fn year(s: &str) -> String {
    s.split(|c: char| !c.is_ascii_digit())
        .find(|t| t.len() == 4)
        .unwrap_or("")
        .to_string()
}

fn life(p: &Person) -> String {
    let (b, d) = (year(&p.bd), year(&p.dd));
    if b.is_empty() && d.is_empty() {
        String::new()
    } else {
        format!("{}-{}", if b.is_empty() { "?" } else { b.as_str() }, d)
    }
}

impl Db {
    fn families_count(&self) -> usize {
        self.fams.len()
    }
    fn alloc(&mut self) -> u32 {
        self.next += 1;
        self.next
    }
    /// Every family the person is a child of, best first: the one chosen for the chart, then
    /// families where both parents are biological, then the rest.
    /// Lookup tables so big trees do not rescan every family for every question.
    /// Build once for a whole chart, report or check; rebuild after the data changes.
    fn index(&self) -> Idx {
        let mut ix = Idx::default();
        for (fid, f) in &self.fams {
            for k in &f.kids {
                ix.child_of.entry(*k).or_default().push(*fid);
            }
            for p in f.h.iter().chain(f.w.iter()).chain(f.more.iter().map(|(_, x)| x)) {
                let e = ix.partner_in.entry(*p).or_default();
                if !e.contains(fid) {
                    e.push(*fid);
                }
            }
        }
        for v in ix.partner_in.values_mut() {
            v.sort_by_key(|f| (parse_date(&self.fams[f].md).0.unwrap_or(i32::MAX), *f));
        }
        ix
    }
    fn parent_families_ix(&self, ix: &Idx, id: u32) -> Vec<u32> {
        let cands: &[u32] = ix.child_of.get(&id).map(|v| v.as_slice()).unwrap_or(&[]);
        self.rank_parent_fams(id, cands.iter().copied())
    }
    fn fams_of_ix<'a>(&self, ix: &'a Idx, id: u32) -> &'a [u32] {
        ix.partner_in.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    fn parents_ix(&self, ix: &Idx, id: u32) -> (Option<u32>, Option<u32>) {
        match self.parent_families_ix(ix, id).first() {
            Some(f) => (self.fams[f].h, self.fams[f].w),
            None => (None, None),
        }
    }
    fn parent_families(&self, id: u32) -> Vec<u32> {
        self.rank_parent_fams(id, self.fams.iter().filter(|(_, f)| f.kids.contains(&id)).map(|(k, _)| *k))
    }
    fn rank_parent_fams(&self, id: u32, cands: impl Iterator<Item = u32>) -> Vec<u32> {
        let pref = self.people.get(&id).and_then(|p| p.pref_parents);
        let mut v: Vec<(u8, u32)> = cands
            .filter_map(|k| self.fams.get(&k).map(|f| (k, f)))
            .map(|(k, f)| {
                let bio = rel_is_bio(&f.rel_to(id, true)) as u8 + rel_is_bio(&f.rel_to(id, false)) as u8;
                let rank = if pref == Some(k) { 0 } else { 3 - bio };
                (rank, k)
            })
            .collect();
        v.sort();
        v.into_iter().map(|(_, k)| k).collect()
    }
    fn famc(&self, id: u32) -> Option<u32> {
        self.parent_families(id).first().copied()
    }
    /// Families the person is a partner in, in the order of the partnerships (by marriage year,
    /// those without a date last).
    fn fams_of(&self, id: u32) -> Vec<u32> {
        let mut v: Vec<(i32, u32)> = self
            .fams
            .iter()
            .filter(|(_, f)| f.h == Some(id) || f.w == Some(id) || f.more.iter().any(|(_, x)| *x == id))
            .map(|(k, f)| (parse_date(&f.md).0.unwrap_or(i32::MAX), *k))
            .collect();
        v.sort();
        v.into_iter().map(|(_, k)| k).collect()
    }
    /// Everyone who is a parent of `id` through any family, with how: (parent, relationship word).
    /// Brothers and sisters (full and half) in the order of the parents' family lists.
    fn siblings(&self, id: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for f in self.parent_families(id) {
            for k in &self.fams[&f].kids {
                if *k != id && !out.contains(k) && self.people.contains_key(k) {
                    out.push(*k);
                }
            }
        }
        // half-brothers and sisters: other children of any of the parents
        for (p, _) in self.parent_links(id) {
            for k in self.children_of(p) {
                if k != id && !out.contains(&k) {
                    out.push(k);
                }
            }
        }
        out
    }
    fn partners(&self, id: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for f in self.fams_of(id) {
            let fm = &self.fams[&f];
            for p in fm.h.iter().chain(fm.w.iter()) {
                if *p != id && !out.contains(p) && self.people.contains_key(p) {
                    out.push(*p);
                }
            }
        }
        out
    }
    fn children_of(&self, id: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for f in self.fams_of(id) {
            for k in &self.fams[&f].kids {
                if !out.contains(k) && self.people.contains_key(k) {
                    out.push(*k);
                }
            }
        }
        out
    }
    fn parent_links(&self, id: u32) -> Vec<(u32, String)> {
        let mut out: Vec<(u32, String)> = Vec::new();
        for fid in self.parent_families(id) {
            let f = &self.fams[&fid];
            for (p, father) in [(f.h, true), (f.w, false)] {
                if let Some(p) = p {
                    if !out.iter().any(|(q, _)| *q == p) {
                        out.push((p, f.rel_to(id, father)));
                    }
                }
            }
        }
        out
    }
    /// Step-parents (a parent's partner who is not a parent), half-siblings (one parent in common),
    /// step-siblings (children of a step-parent) and step-children (a partner's children). Worked out
    /// from the tree; nothing is stored.
    fn kin_extras(&self, id: u32) -> Kin {
        let parents: Vec<u32> = self.parent_links(id).into_iter().map(|(p, _)| p).collect();
        let mut k = Kin::default();
        let kids_of = |p: u32| -> Vec<u32> { self.fams_of(p).into_iter().flat_map(|f| self.fams[&f].kids.clone()).collect() };
        let partners_of = |p: u32| -> Vec<u32> {
            self.fams_of(p)
                .into_iter()
                .flat_map(|f| {
                    let fm = &self.fams[&f];
                    [fm.h, fm.w].into_iter().flatten().filter(|x| *x != p).collect::<Vec<_>>()
                })
                .collect()
        };
        let push = |v: &mut Vec<u32>, x: u32| {
            if x != id && !v.contains(&x) {
                v.push(x);
            }
        };
        for p in &parents {
            for s in kids_of(*p) {
                let theirs: Vec<u32> = self.parent_links(s).into_iter().map(|(q, _)| q).collect();
                let shared = theirs.iter().filter(|q| parents.contains(q)).count();
                if shared == 1 {
                    push(&mut k.half_siblings, s);
                }
            }
            for q in partners_of(*p) {
                if !parents.contains(&q) && q != id {
                    push(&mut k.step_parents, q);
                    for s in kids_of(q) {
                        let theirs: Vec<u32> = self.parent_links(s).into_iter().map(|(x, _)| x).collect();
                        if !theirs.iter().any(|x| parents.contains(x)) {
                            push(&mut k.step_siblings, s);
                        }
                    }
                }
            }
        }
        let mine = kids_of(id);
        for q in partners_of(id) {
            for c in kids_of(q) {
                if !mine.contains(&c) && !self.parent_links(c).iter().any(|(x, _)| *x == id) {
                    push(&mut k.step_children, c);
                }
            }
        }
        // People who have this person as a guardian, godparent and so on.
        for (oid, op) in &self.people {
            for a in &op.assoc {
                if a.other == id && *oid != id {
                    k.assoc_of.push((*oid, a.role.clone()));
                }
            }
        }
        k
    }
    /// Put `pid` into a parent slot for `child`. `fid` = an existing family the child
    /// already belongs to; None starts a new, separate set of parents (adoptive, step...).
    fn add_parent(&mut self, child: u32, fid: Option<u32>, father: bool, pid: u32) {
        if pid == child {
            return;
        }
        match fid.filter(|f| self.fams.contains_key(f)) {
            Some(f) => {
                let fm = self.fams.get_mut(&f).unwrap();
                if !fm.kids.contains(&child) {
                    fm.kids.push(child);
                }
                if father { fm.h = Some(pid) } else { fm.w = Some(pid) }
            }
            None => {
                let nf = self.alloc();
                let mut fm = Family { kids: vec![child], ..Default::default() };
                if father { fm.h = Some(pid) } else { fm.w = Some(pid) }
                // anything but the first set of parents defaults to "not biological" only when the user says so
                self.fams.insert(nf, fm);
            }
        }
    }
    /// Take a child out of one parent family (the family and the people stay).
    /// " (adopted)", " (step-child of mother)" ... or empty for an ordinary biological child.
    fn child_note(&self, fid: u32, child: u32) -> String {
        let Some(f) = self.fams.get(&fid) else { return String::new() };
        let fr = f.rel_to(child, true);
        let mr = f.rel_to(child, false);
        if rel_is_bio(&fr) && rel_is_bio(&mr) {
            return String::new();
        }
        if rel_name(&fr) == rel_name(&mr) {
            return format!(" ({})", rel_name(&fr).to_lowercase());
        }
        let mut parts = Vec::new();
        if f.h.is_some() { parts.push(format!("{} to father", rel_name(&fr).to_lowercase())); }
        if f.w.is_some() { parts.push(format!("{} to mother", rel_name(&mr).to_lowercase())); }
        format!(" ({})", parts.join(", "))
    }
    fn remove_from_family(&mut self, child: u32, fid: u32) {
        if let Some(f) = self.fams.get_mut(&fid) {
            f.kids.retain(|k| *k != child);
            f.pedi.remove(&child);
            f.frel.remove(&child);
            f.mrel.remove(&child);
        }
        if let Some(p) = self.people.get_mut(&child) {
            if p.pref_parents == Some(fid) {
                p.pref_parents = None;
            }
        }
    }
    fn parents(&self, id: u32) -> (Option<u32>, Option<u32>) {
        match self.famc(id) {
            Some(f) => {
                let f = &self.fams[&f];
                (f.h, f.w)
            }
            None => (None, None),
        }
    }
    fn timeline(&self, id: u32) -> Vec<(String, String)> {
        let p = &self.people[&id];
        let by = parse_date(&p.bd).0;
        let mut items: Vec<(i32, String, String)> = Vec::new();
        let mut add = |date: &str, place: &str, what: String, is_birth: bool| {
            if date.trim().is_empty() && place.trim().is_empty() {
                return;
            }
            let (y, m, d) = parse_date(date);
            let key = y.map(|y| y * 400 + m.unwrap_or(0) as i32 * 32 + d.unwrap_or(0) as i32).unwrap_or(i32::MAX);
            let age = match (by, y) {
                (Some(b), Some(y)) if !is_birth && y >= b => format!(" (age {})", y - b),
                _ => String::new(),
            };
            let pl = if place.is_empty() { String::new() } else { format!(" - {}", place) };
            items.push((key, date.to_string(), format!("{}{}{}", what, pl, age)));
        };
        add(&p.bd, &p.bp, "Born".to_string(), true);
        add(&p.chd, &p.chp, "Christened".to_string(), false);
        add(&p.dd, &p.dp, "Died".to_string(), false);
        add(&p.bud, &p.bup, "Buried".to_string(), false);
        for fid in self.fams_of(id) {
            let f = &self.fams[&fid];
            let sp = if f.h == Some(id) { f.w } else { f.h };
            let who = sp.map(|s| self.people[&s].name.clone()).unwrap_or_default();
            add(&f.md, &f.mp, format!("Married {}", who).trim().to_string(), false);
            for k in &f.kids {
                let c = &self.people[k];
                add(&c.bd, &c.bp, format!("Child born: {}", c.name), false);
            }
        }
        for e in &p.events {
            let detail = [e.val.as_str(), e.note.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(". ");
            let note = if detail.is_empty() { String::new() } else { format!(": {}", detail) };
            add(&e.date, &e.place, format!("{}{}", e.kind, note), false);
        }
        items.sort_by_key(|x| x.0);
        items.into_iter().map(|(_, d, t)| (d, t)).collect()
    }

    fn check(&self) -> Vec<(u32, String)> {
        let cy = current_year();
        let mut v: Vec<(u32, String)> = Vec::new();
        let ix = self.index();
        for (id, p) in &self.people {
            let by = parse_date(&p.bd).0;
            let dy = parse_date(&p.dd).0;
            if p.bd.trim().is_empty() {
                v.push((*id, "No birth date recorded".to_string()));
            }
            match (by, dy) {
                (Some(b), Some(d)) => {
                    if d < b {
                        v.push((*id, "Died before they were born".to_string()));
                    } else if d - b > 110 {
                        v.push((*id, format!("Age at death would be {}", d - b)));
                    }
                }
                (Some(b), None) => {
                    if !p.living && cy - b > 110 {
                        v.push((*id, "Born over 110 years ago with no death date (add one, or tick Living)".to_string()));
                    }
                }
                _ => {}
            }
            if let Some(b) = by {
                if b > cy {
                    v.push((*id, "Birth year is in the future".to_string()));
                }
            }
            let (f, m) = self.parents_ix(&ix, *id);
            for (pid, role) in [(f, "father"), (m, "mother")] {
                let (Some(pid), Some(b)) = (pid, by) else { continue };
                let par = &self.people[&pid];
                if let Some(pb) = parse_date(&par.bd).0 {
                    if b < pb {
                        v.push((*id, format!("Born before their {} was born", role)));
                    } else if b - pb < 13 {
                        v.push((*id, format!("Their {} would have been only {} at the birth", role, b - pb)));
                    } else if role == "mother" && b - pb > 55 {
                        v.push((*id, format!("Their mother would have been {} at the birth", b - pb)));
                    } else if role == "father" && b - pb > 85 {
                        v.push((*id, format!("Their father would have been {} at the birth", b - pb)));
                    }
                }
                if let Some(pd) = parse_date(&par.dd).0 {
                    if role == "mother" && b > pd {
                        v.push((*id, "Born after their mother died".to_string()));
                    }
                    if role == "father" && b > pd + 1 {
                        v.push((*id, "Born more than a year after their father died".to_string()));
                    }
                }
            }
        }
        for f in self.fams.values() {
            if let Some(my) = parse_date(&f.md).0 {
                for sp in [f.h, f.w].into_iter().flatten() {
                    if let Some(sb) = parse_date(&self.people[&sp].bd).0 {
                        if my < sb + 12 {
                            v.push((sp, format!("Married in {} when only {}", my, my - sb)));
                        }
                    }
                }
            }
        }
        // Complex-family checks
        for id in self.people.keys() {
            let pf = self.parent_families_ix(&ix, *id);
            let bio_sets = pf.iter().filter(|f| { let fm = &self.fams[*f]; rel_is_bio(&fm.rel_to(*id, true)) && rel_is_bio(&fm.rel_to(*id, false)) && (fm.h.is_some() || fm.w.is_some()) }).count();
            if bio_sets > 1 {
                v.push((*id, "Has more than one set of biological parents (mark the others as adopted, step, foster or guardian)".to_string()));
            }
        }
        // Loops: someone who is their own ancestor. One pass over the whole parent graph.
        let parents_of = |n: u32| -> Vec<u32> {
            let mut out = Vec::new();
            for f in self.parent_families_ix(&ix, n) {
                let fm = &self.fams[&f];
                for p in fm.h.iter().chain(fm.w.iter()) {
                    if !out.contains(p) && self.people.contains_key(p) {
                        out.push(*p);
                    }
                }
            }
            out
        };
        let mut state: HashMap<u32, u8> = HashMap::new(); // 1 = on the current path, 2 = finished
        let mut looped: HashSet<u32> = HashSet::new();
        for start in self.people.keys() {
            if state.contains_key(start) {
                continue;
            }
            let mut stack: Vec<(u32, Vec<u32>, usize)> = vec![(*start, parents_of(*start), 0)];
            state.insert(*start, 1);
            while let Some(top) = stack.last_mut() {
                if top.2 < top.1.len() {
                    let nx = top.1[top.2];
                    top.2 += 1;
                    match state.get(&nx).copied() {
                        Some(1) => {
                            let pos = stack.iter().position(|(n, _, _)| *n == nx).unwrap_or(0);
                            for (n, _, _) in &stack[pos..] {
                                looped.insert(*n);
                            }
                        }
                        Some(_) => {}
                        None => {
                            state.insert(nx, 1);
                            let ps = parents_of(nx);
                            stack.push((nx, ps, 0));
                        }
                    }
                } else {
                    let n = top.0;
                    state.insert(n, 2);
                    stack.pop();
                }
            }
        }
        let mut looped: Vec<u32> = looped.into_iter().collect();
        looped.sort();
        for id in looped {
            v.push((id, "Is recorded as their own ancestor (a loop in the family links)".to_string()));
        }
        let mut seen_pairs: HashMap<(u32, u32), u32> = HashMap::new();
        for f in self.fams.values() {
            if let (Some(a), Some(b)) = (f.h, f.w) {
                let key = (a.min(b), a.max(b));
                let n = seen_pairs.entry(key).or_insert(0);
                *n += 1;
                if *n == 2 {
                    v.push((a, format!("Has two separate family groups with the same partner, {} (merge them if it is one partnership)", self.people[&b].name)));
                }
            }
            for k in &f.kids {
                if Some(*k) == f.h || Some(*k) == f.w {
                    v.push((*k, "Is recorded as their own parent".to_string()));
                }
            }
        }
        v.sort_by(|a, b| self.people[&a.0].name.to_lowercase().cmp(&self.people[&b.0].name.to_lowercase()));
        v
    }

    fn rel_names_from(&self, f: &Family, id: u32, out: &mut HashSet<String>) {
        let mine = f.h == Some(id) || f.w == Some(id);
        let kid = f.kids.contains(&id);
        if mine || kid {
            for x in f.h.iter().chain(f.w.iter()).chain(if mine { f.kids.iter() } else { [].iter() }) {
                if *x != id {
                    if let Some(p) = self.people.get(x) {
                        let g = p.given.to_lowercase().split_whitespace().next().unwrap_or("").to_string();
                        if !g.is_empty() {
                            out.insert(g);
                        }
                    }
                }
            }
        }
    }
    fn rel_names_ix(&self, ix: &Idx, id: u32) -> HashSet<String> {
        let mut out = HashSet::new();
        let mut fs: Vec<u32> = ix.child_of.get(&id).cloned().unwrap_or_default();
        fs.extend(ix.partner_in.get(&id).cloned().unwrap_or_default());
        fs.sort();
        fs.dedup();
        for f in fs {
            if let Some(fm) = self.fams.get(&f) {
                self.rel_names_from(fm, id, &mut out);
            }
        }
        out
    }

    fn duplicate_pairs(&self) -> Vec<(u32, u32, String)> {
        let norm = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
        let ix = self.index();
        let mut cache: HashMap<u32, HashSet<String>> = HashMap::new();
        // People are only compared within a group whose surnames sound alike (so "Smith" and
        // "Smyth", or a typo, still meet). A woman is also grouped under her maiden name.
        let mut buckets: HashMap<String, Vec<u32>> = HashMap::new();
        for (id, p) in &self.people {
            let sn = if p.surname.is_empty() { p.name.split_whitespace().last().unwrap_or("").to_string() } else { p.surname.clone() };
            let mut keys: Vec<String> = vec![sound_key(&sn)];
            if !p.maiden.trim().is_empty() {
                keys.push(sound_key(&p.maiden));
            }
            keys.retain(|k| !k.is_empty());
            keys.dedup();
            if norm(&p.given).is_empty() {
                continue;
            }
            for k in keys {
                buckets.entry(k).or_default().push(*id);
            }
        }
        let mut seen_pairs: HashSet<(u32, u32)> = HashSet::new();
        let mut out: Vec<(i32, u32, u32, String)> = Vec::new();
        for ids in buckets.values() {
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let pair = (ids[i].min(ids[j]), ids[i].max(ids[j]));
                    if !seen_pairs.insert(pair) {
                        continue;
                    }
                    let (a, b) = (&self.people[&ids[i]], &self.people[&ids[j]]);
                    let (ga, gb) = (norm(&a.given), norm(&b.given));
                    let mut score: i32 = 0;
                    let mut why: Vec<&str> = Vec::new();
                    let (fa, fb) = (first_given(&a.given), first_given(&b.given));
                    if !ga.is_empty() && ga == gb {
                        score += 2;
                        why.push("same name");
                    } else if !fa.is_empty() && canon_given(&fa) == canon_given(&fb) {
                        score += 2;
                        why.push(if fa == fb { "same first name" } else { "nickname of the same name" });
                    } else if !ga.is_empty() && !gb.is_empty() && (ga.starts_with(&gb) || gb.starts_with(&ga)) {
                        score += 1;
                        why.push("similar name");
                    } else if fa.len() >= 4 && fb.len() >= 4 && edit_distance(&fa, &fb) <= 1 {
                        score += 1;
                        why.push("names differ by one letter");
                    } else {
                        continue;
                    }
                    let (sa, sb) = (norm(&a.surname), norm(&b.surname));
                    if !sa.is_empty() && !sb.is_empty() && sa != sb {
                        why.push("surnames sound alike");
                    }
                    if let (Some(x), Some(y)) = (parse_date(&a.bd).0, parse_date(&b.bd).0) {
                        let d = (x - y).abs();
                        if d == 0 {
                            score += 2;
                            why.push("same birth year");
                        } else if d <= 2 {
                            score += 1;
                            why.push("birth years close");
                        } else {
                            continue;
                        }
                    }
                    if !a.bp.is_empty() && norm(&a.bp) == norm(&b.bp) {
                        score += 1;
                        why.push("same birthplace");
                    }
                    if let (Some(x), Some(y)) = (parse_date(&a.dd).0, parse_date(&b.dd).0) {
                        if x == y {
                            score += 1;
                            why.push("same death year");
                        }
                    }
                    let exact = !ga.is_empty() && ga == gb;
                    if exact && !a.bp.is_empty() != !b.bp.is_empty() {
                        why.push("one copy has less detail");
                    }
                    for k in [ids[i], ids[j]] {
                        if !cache.contains_key(&k) {
                            cache.insert(k, self.rel_names_ix(&ix, k));
                        }
                    }
                    let (ra, rb) = (&cache[&ids[i]], &cache[&ids[j]]);
                    if !ra.is_disjoint(rb) {
                        score += 2;
                        why.push("same relatives");
                    }
                    // Same full name and nothing contradicts it (checked above): worth a look even when
                    // one copy has little else recorded.
                    if score >= 3 || exact {
                        let (lo, hi) = if ids[i] < ids[j] { (ids[i], ids[j]) } else { (ids[j], ids[i]) };
                        out.push((score, lo, hi, why.join(", ")));
                    }
                }
            }
        }
        out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        out.into_iter().map(|(_, a, b, w)| (a, b, w)).collect()
    }

    fn sanitized(&self) -> Db {
        let mut d = self.clone();
        for p in d.people.values_mut() {
            if p.living {
                let sex = p.sex.clone();
                *p = Person { name: "Living".to_string(), given: "Living".to_string(), sex, living: true, ..Default::default() };
            }
        }
        // Marriages and other family details of a living person are private too.
        let living: HashSet<u32> = d.people.iter().filter(|(_, p)| p.living).map(|(k, _)| *k).collect();
        for f in d.fams.values_mut() {
            let touches = f.h.map_or(false, |x| living.contains(&x)) || f.w.map_or(false, |x| living.contains(&x)) || f.more.iter().any(|(_, x)| living.contains(x));
            if touches {
                *f = Family { h: f.h, w: f.w, kids: f.kids.clone(), more: f.more.clone(), ..Default::default() };
            }
        }
        d
    }

    fn ancestors(&self, id: u32) -> HashMap<u32, u32> {
        let mut m: HashMap<u32, u32> = HashMap::new();
        m.insert(id, 0);
        let mut q = std::collections::VecDeque::new();
        q.push_back(id);
        while let Some(n) = q.pop_front() {
            let d = m[&n];
            let (f, mo) = self.parents(n);
            for p in [f, mo].into_iter().flatten() {
                if !m.contains_key(&p) {
                    m.insert(p, d + 1);
                    q.push_back(p);
                }
            }
        }
        m
    }

    /// Ancestors with their distance. With `bio_only`, only biological (or unrecorded) links are followed.
    fn ancestors_by(&self, id: u32, bio_only: bool) -> HashMap<u32, u32> {
        let mut m: HashMap<u32, u32> = HashMap::new();
        m.insert(id, 0);
        let mut q = std::collections::VecDeque::new();
        q.push_back(id);
        while let Some(n) = q.pop_front() {
            let d = m[&n];
            for fid in self.parent_families(n) {
                let f = &self.fams[&fid];
                for (p, father) in [(f.h, true), (f.w, false)] {
                    if let Some(p) = p {
                        if (!bio_only || rel_is_bio(&f.rel_to(n, father))) && !m.contains_key(&p) {
                            m.insert(p, d + 1);
                            q.push_back(p);
                        }
                    }
                }
            }
        }
        m
    }

    fn relationship(&self, a: u32, b: u32) -> String {
        let (pa, pb) = (&self.people[&a], &self.people[&b]);
        let (na, nb) = (if pa.name.is_empty() { "Unnamed" } else { pa.name.as_str() }, if pb.name.is_empty() { "Unnamed" } else { pb.name.as_str() });
        if a == b {
            return format!("{} and {} are the same person.", na, nb);
        }
        let sx = pb.sex.as_str();
        // Partners
        if let Some(f) = self.fams.values().find(|f| (f.h == Some(a) && f.w == Some(b)) || (f.h == Some(b) && f.w == Some(a))) {
            let ended = f.events.iter().any(|e| e.kind == "Divorce" || e.kind == "Annulment");
            let word = match f.kind.as_str() {
                "Unmarried" => "partner".to_string(),
                "Civil Union" => format!("civil {}", pick_sex(sx, "partner", "partner", "partner")),
                _ => pick_sex(sx, "spouse", "husband", "wife").to_string(),
            };
            return format!("{} is {}'s {}{}.", nb, na, if ended { "former " } else { "" }, word);
        }
        // A direct parent or child by adoption, fostering, step or guardianship
        if let Some((_, rel)) = self.parent_links(a).into_iter().find(|(p, _)| *p == b) {
            if !rel_is_bio(&rel) {
                return format!("{} is {}'s {}.", nb, na, legal_parent_word(&rel, sx));
            }
        }
        if let Some((_, rel)) = self.parent_links(b).into_iter().find(|(p, _)| *p == a) {
            if !rel_is_bio(&rel) {
                return format!("{} is {}'s {}.", nb, na, legal_child_word(&rel, sx));
            }
        }
        // Blood relatives (biological links only)
        let (ma, mb) = (self.ancestors_by(a, true), self.ancestors_by(b, true));
        let mut commons: Vec<(u32, u32)> = ma.iter().filter_map(|(k, da)| mb.get(k).map(|db| (*da, *db))).collect();
        if let Some(sum) = commons.iter().map(|(x, y)| x + y).min() {
            commons.retain(|(x, y)| x + y == sum);
            commons.sort();
            let (da, db) = (commons[0].0 as i32, commons[0].1 as i32);
            let half = da == 1 && db == 1 && commons.len() < 2;
            return format!("{} is {}'s {}.", nb, na, kin_term(da, db, sx, half));
        }
        // Step and half relatives
        let kin = self.kin_extras(a);
        if kin.step_parents.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "step-parent", "stepfather", "stepmother"));
        }
        if kin.step_children.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "stepchild", "stepson", "stepdaughter"));
        }
        if kin.step_siblings.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "step-sibling", "stepbrother", "stepsister"));
        }
        if kin.half_siblings.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "half-sibling", "half-brother", "half-sister"));
        }
        // Relatives through adoption, fostering or guardianship links
        let (la, lb) = (self.ancestors_by(a, false), self.ancestors_by(b, false));
        let lc: Vec<(u32, u32)> = la.iter().filter_map(|(k, da)| lb.get(k).map(|db| (*da, *db))).collect();
        if let Some((da, db)) = lc.iter().copied().min_by_key(|(x, y)| (x + y, *x)) {
            return format!("{} is {}'s {}, counting adoptive, foster or step links (not a blood relationship).", nb, na, kin_term(da as i32, db as i32, sx, false));
        }
        if let Some(r) = pa.assoc.iter().find(|x| x.other == b) {
            return format!("{} is {}'s {} (a recorded tie, not a family relationship).", nb, na, if r.role.is_empty() { "associate" } else { r.role.as_str() });
        }
        if let Some(r) = pb.assoc.iter().find(|x| x.other == a) {
            return format!("{} is the {} of {} (a recorded tie, not a family relationship).", na, if r.role.is_empty() { "associate" } else { r.role.as_str() }, nb);
        }
        format!("No blood relationship found between {} and {} in your tree. (In-laws and other marriage relationships are not included.)", na, nb)
    }

    fn label(&self, id: u32) -> String {
        let p = &self.people[&id];
        let name = display_name(p);
        let l = life(p);
        if l.is_empty() { name } else { format!("{} ({})", name, l) }
    }
    fn sorted(&self, filter: &str) -> Vec<u32> {
        let f = filter.to_lowercase();
        let mut v: Vec<u32> = self
            .people
            .iter()
            .filter(|(_, p)| f.is_empty() || format!("{} {} {} {}", p.name, p.maiden, p.bp, p.dp).to_lowercase().contains(&f))
            .map(|(i, _)| *i)
            .collect();
        v.sort_by_cached_key(|i| {
            let p = &self.people[i];
            let n = p.name.to_lowercase();
            let sn = if p.surname.is_empty() { n.split_whitespace().last().unwrap_or("").to_string() } else { p.surname.to_lowercase() };
            (sn, n)
        });
        v
    }
}

fn add(db: &mut Db, n: &str, s: &str, bd: &str, bp: &str, dd: &str) -> u32 {
    let id = db.alloc();
    db.people.insert(
        id,
        Person { name: n.into(), sex: s.into(), bd: bd.into(), bp: bp.into(), dd: dd.into(), ..Default::default() },
    );
    id
}

fn fam(db: &mut Db, h: u32, w: u32, md: &str, mp: &str, kids: &[u32]) {
    let id = db.alloc();
    db.fams.insert(id, Family { h: Some(h), w: Some(w), md: md.into(), mp: mp.into(), kids: kids.to_vec(), ..Default::default() });
}

fn sample() -> Db {
    let mut db = Db { sample: true, ..Default::default() };
    let a = add(&mut db, "Henry Walker", "M", "12 Mar 1921", "Provo, Utah", "1994");
    let b = add(&mut db, "Ruth Miller", "F", "1924", "Ogden, Utah", "2009");
    let c = add(&mut db, "Robert Walker", "M", "1948", "Salt Lake City, Utah", "");
    let d = add(&mut db, "Anne Walker", "F", "1951", "Salt Lake City, Utah", "");
    let e = add(&mut db, "Linda Hayes", "F", "1950", "Boise, Idaho", "");
    let g = add(&mut db, "Susan Walker", "F", "1975", "Denver, Colorado", "");
    let h = add(&mut db, "George Hayes", "M", "1925", "Twin Falls, Idaho", "2001");
    let i = add(&mut db, "Edith Cole", "F", "1927", "Boise, Idaho", "2015");
    fam(&mut db, a, b, "4 Jun 1946", "Salt Lake City, Utah", &[c, d]);
    fam(&mut db, c, e, "1972", "Denver, Colorado", &[g]);
    fam(&mut db, h, i, "1948", "Boise, Idaho", &[e]);
    fix_names(&mut db);
    db
}

/* ---------- App ---------- */

#[derive(PartialEq, Clone, Copy)]
enum View {
    Pedigree,
    Family,
    Descendants,
    Person,
    Gallery,
    Charts,
    Table,
    Map,
    Memories,
    Dna,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum Wipe {
    Sample,
    Empty,
    Import,
    Restore,
}


struct LinkReq {
    child: u32,
    fid: Option<u32>,
    father: bool,
    q: String,
}

/// Drop-down for how a child is related to one parent. Returns the new GEDCOM word when changed.
fn rel_combo(ui: &mut egui::Ui, salt: impl std::hash::Hash, cur: &str) -> Option<String> {
    let mut out = None;
    egui::ComboBox::from_id_salt(salt).selected_text(rel_name(cur)).width(120.0).show_ui(ui, |ui| {
        for (w, lbl) in PARENT_RELS {
            if ui.selectable_label(rel_name(cur) == *lbl, *lbl).clicked() {
                out = Some(if *w == "Natural" { String::new() } else { w.to_string() });
            }
        }
    });
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Father,
    Mother,
    Spouse,
    Child(Option<u32>),
    Person,
    ParentIn(u32, bool),
    NewParent(bool),
    Sibling,
}

#[derive(Clone, Serialize, Deserialize)]
struct Settings {
    dark: bool,
    accent: [u8; 3],
    #[serde(default = "default_bg")]
    bg: [u8; 3],
    #[serde(default = "default_gens")]
    gens: usize,
    #[serde(default = "default_true")]
    hide_living: bool,
    #[serde(default)]
    mic: String,
    #[serde(default = "default_zoom")]
    zoom: f32,
    #[serde(default)]
    home: Option<u32>,
    #[serde(default = "default_dock")]
    dock: Vec<DockItem>,
    #[serde(default = "default_dock_w")]
    dock_w: f32,
    /// Size of all text and buttons (1.0 = normal).
    #[serde(default = "default_zoom")]
    ui_scale: f32,
    /// 0 "John Smith", 1 "SMITH, John", 2 "Smith, John"
    #[serde(default)]
    name_style: u8,
    /// 0 as typed, 1 "12 Mar 1921", 2 "March 12, 1921", 3 "1921-03-12"
    #[serde(default)]
    date_style: u8,
    #[serde(default = "default_true")]
    box_photos: bool,
    #[serde(default)]
    box_places: bool,
    #[serde(default)]
    box_ids: bool,
    /// Pedigree and descendant charts: the hand tool is on (drag to move the chart around)
    #[serde(default)]
    hand: bool,
    /// Profile photo size on the Individual page: 0 small, 1 normal, 2 large, 3 extra large.
    #[serde(default = "default_photo_size")]
    photo_size: u8,
    /// Backups: 0 daily, 1 every hour while open, 2 weekly. How many to keep. A second folder
    /// (USB stick, external drive) that also receives a Family Tree Archive.
    #[serde(default = "default_true")]
    show_history: bool,
    /// The tree that was open last ("" = the main tree).
    #[serde(default)]
    current_tree: String,
    #[serde(default)]
    backup_every: u8,
    #[serde(default = "default_keep")]
    backup_keep: usize,
    #[serde(default)]
    backup_extra: String,
    /// Window size when the program was last closed (points), and whether it was maximised.
    #[serde(default)]
    win: Option<[f32; 2]>,
    #[serde(default)]
    win_max: bool,
    /// Report designs the user saved (Reports > Report designer).
    #[serde(default)]
    reports: Vec<designer::Design>,
    /// Panel layouts the user saved by name.
    #[serde(default)]
    layouts: Vec<Layout>,
    /// Map colours: 0 matches the app (charcoal or paper), then the styles in mapview::MAP_STYLES.
    #[serde(default)]
    map_style: u8,
    /// Background for the light theme. Colour schemes change it; the default is the original grey.
    #[serde(default = "default_light_bg")]
    light_bg: [u8; 3],
    /// Text colour set by a colour scheme. None keeps the theme's own text colour.
    #[serde(default)]
    ink: Option<[u8; 3]>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Layout {
    name: String,
    dock: Vec<DockItem>,
    width: f32,
}

/// Ready-made layouts: (name, panels to show).
const LAYOUT_PRESETS: &[(&str, &[&str])] = &[
    ("Research", &["relationships", "todo", "sources", "notes"]),
    ("Photos and stories", &["media", "relationships", "notes"]),
    ("Family", &["relationships", "events", "names"]),
    ("Everything", &["relationships", "todo", "events", "sources", "notes", "names", "media", "more"]),
];

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct DockItem {
    sec: String,
    on: bool,
    float: bool,
}

fn default_dock() -> Vec<DockItem> {
    let d = |sec: &str, on: bool| DockItem { sec: sec.to_string(), on, float: false };
    vec![d("relationships", true), d("todo", true), d("journal", false), d("events", false), d("sources", false), d("notes", true), d("names", false), d("media", false), d("more", false)]
}

fn default_keep() -> usize {
    14
}

fn default_photo_size() -> u8 {
    1
}

fn default_dock_w() -> f32 {
    380.0
}

fn dock_title(sec: &str) -> &'static str {
    match sec {
        "relationships" => "Relationships",
        "events" => "Events",
        "sources" => "Sources",
        "notes" => "Notes",
        "names" => "Other names",
        "media" => "Photos and recordings",
        "more" => "Merge and delete",
        "todo" => "Research to-do",
        "journal" => "Research journal",
        _ => "",
    }
}

fn default_zoom() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_gens() -> usize {
    5
}

fn default_bg() -> [u8; 3] {
    [30, 31, 34]
}

impl Default for Settings {
    fn default() -> Self {
        Settings { hand: false, dark: true, accent: [59, 111, 212], bg: default_bg(), gens: default_gens(), hide_living: true, mic: String::new(), zoom: 1.0, home: None, dock: default_dock(), dock_w: default_dock_w(), ui_scale: 1.0, name_style: 0, date_style: 0, box_photos: true, box_places: false, box_ids: false, photo_size: 1, show_history: true, current_tree: String::new(), backup_every: 0, backup_keep: 14, backup_extra: String::new(), win: None, win_max: false, layouts: Vec::new(), reports: Vec::new(), map_style: 0, light_bg: default_light_bg(), ink: None }
    }
}

const PRESETS: [(&str, [u8; 3]); 24] = [
    ("Blue", [59, 111, 212]),
    ("Royal blue", [45, 80, 170]),
    ("Sky", [56, 150, 220]),
    ("Navy", [40, 60, 120]),
    ("Teal", [20, 150, 150]),
    ("Turquoise", [30, 175, 170]),
    ("Green", [56, 142, 60]),
    ("Forest", [40, 105, 60]),
    ("Olive", [120, 130, 50]),
    ("Sage", [120, 150, 110]),
    ("Gold", [196, 152, 40]),
    ("Amber", [220, 150, 30]),
    ("Orange", [230, 126, 34]),
    ("Copper", [184, 100, 60]),
    ("Brown", [130, 90, 60]),
    ("Red", [200, 70, 70]),
    ("Crimson", [170, 30, 60]),
    ("Rose", [210, 90, 130]),
    ("Pink", [225, 120, 170]),
    ("Plum", [130, 60, 120]),
    ("Purple", [140, 90, 200]),
    ("Lavender", [150, 130, 210]),
    ("Slate", [110, 120, 135]),
    ("Graphite", [80, 85, 95]),
];

const BG_PRESETS: [(&str, [u8; 3]); 8] = [
    ("Black", [14, 14, 16]),
    ("Charcoal", [30, 31, 34]),
    ("Grey", [50, 52, 56]),
    ("Light grey", [74, 77, 83]),
    ("Midnight blue", [22, 28, 44]),
    ("Deep green", [22, 36, 30]),
    ("Espresso", [38, 30, 26]),
    ("Aubergine", [36, 26, 40]),
];

/// A full colour scheme: whether it is dark, the background, the accent and the text colour.
/// Picking one under View > Colours sets all four; the swatches below still change them one by one.
struct Scheme {
    name: &'static str,
    group: &'static str,
    dark: bool,
    bg: [u8; 3],
    accent: [u8; 3],
    ink: [u8; 3],
}

const SCHEMES: &[Scheme] = &[
    Scheme { name: "Slate Blue", group: "Tree pictures", dark: true, bg: [67, 82, 106], accent: [135, 141, 151], ink: [246, 238, 226] },
    Scheme { name: "Navy", group: "Tree pictures", dark: true, bg: [30, 52, 99], accent: [115, 129, 158], ink: [255, 255, 255] },
    Scheme { name: "Parchment", group: "Tree pictures", dark: false, bg: [226, 201, 162], accent: [140, 88, 42], ink: [66, 40, 18] },
    Scheme { name: "Blue Linen", group: "Tree pictures", dark: true, bg: [88, 110, 142], accent: [144, 155, 171], ink: [236, 231, 220] },
    Scheme { name: "Leather", group: "Tree pictures", dark: true, bg: [104, 52, 24], accent: [156, 117, 86], ink: [242, 224, 188] },
    Scheme { name: "Leaf Green", group: "Genealogy sites", dark: false, bg: [243, 240, 232], accent: [104, 134, 28], ink: [52, 48, 44] },
    Scheme { name: "Fresh Green", group: "Genealogy sites", dark: false, bg: [242, 243, 240], accent: [80, 132, 34], ink: [51, 51, 51] },
    Scheme { name: "Deep Green", group: "Genealogy sites", dark: true, bg: [48, 88, 38], accent: [124, 150, 117], ink: [250, 252, 246] },
    Scheme { name: "Sage", group: "Earth tones", dark: false, bg: [199, 208, 184], accent: [88, 110, 74], ink: [40, 52, 36] },
    Scheme { name: "Olive", group: "Earth tones", dark: true, bg: [85, 90, 52], accent: [143, 145, 113], ink: [240, 236, 214] },
    Scheme { name: "Moss", group: "Earth tones", dark: true, bg: [66, 82, 50], accent: [131, 142, 114], ink: [238, 240, 220] },
    Scheme { name: "Forest Green", group: "Earth tones", dark: true, bg: [36, 66, 48], accent: [111, 132, 115], ink: [234, 240, 226] },
    Scheme { name: "Terracotta", group: "Earth tones", dark: true, bg: [150, 78, 52], accent: [188, 138, 116], ink: [250, 236, 222] },
    Scheme { name: "Clay", group: "Earth tones", dark: false, bg: [214, 180, 156], accent: [150, 80, 50], ink: [70, 40, 28] },
    Scheme { name: "Burgundy", group: "Popular colors", dark: true, bg: [110, 28, 40], accent: [163, 107, 114], ink: [250, 236, 236] },
    Scheme { name: "Teal", group: "Popular colors", dark: true, bg: [22, 92, 100], accent: [101, 150, 154], ink: [232, 246, 244] },
    Scheme { name: "Plum", group: "Popular colors", dark: true, bg: [82, 46, 88], accent: [143, 117, 148], ink: [244, 234, 246] },
    Scheme { name: "Charcoal", group: "Popular colors", dark: true, bg: [52, 54, 58], accent: [121, 123, 124], ink: [236, 236, 232] },
    Scheme { name: "Midnight", group: "Popular colors", dark: true, bg: [20, 24, 40], accent: [98, 102, 116], ink: [226, 230, 240] },
    Scheme { name: "Sky", group: "Popular colors", dark: false, bg: [200, 222, 240], accent: [40, 100, 170], ink: [20, 44, 70] },
    Scheme { name: "Rose", group: "Popular colors", dark: false, bg: [236, 210, 212], accent: [160, 60, 80], ink: [80, 30, 40] },
    Scheme { name: "Ivory", group: "Neutrals", dark: false, bg: [246, 242, 228], accent: [110, 104, 80], ink: [40, 38, 30] },
    Scheme { name: "Linen", group: "Neutrals", dark: false, bg: [236, 230, 218], accent: [120, 100, 70], ink: [50, 44, 36] },
    Scheme { name: "Stone", group: "Neutrals", dark: false, bg: [214, 212, 206], accent: [90, 94, 100], ink: [40, 40, 38] },
    Scheme { name: "Taupe", group: "Neutrals", dark: false, bg: [200, 190, 178], accent: [110, 92, 76], ink: [50, 42, 36] },
];

/// The light background used before colour schemes existed.
fn default_light_bg() -> [u8; 3] {
    [233, 234, 236]
}

impl Settings {
    fn use_scheme(&mut self, s: &Scheme) {
        self.dark = s.dark;
        self.accent = s.accent;
        self.ink = Some(s.ink);
        if s.dark {
            self.bg = s.bg;
        } else {
            self.light_bg = s.bg;
        }
    }

    fn is_scheme(&self, s: &Scheme) -> bool {
        self.dark == s.dark && self.accent == s.accent && self.ink == Some(s.ink) && (if s.dark { self.bg } else { self.light_bg }) == s.bg
    }
}

/// The schemes in their groups; returns true when one was picked.
fn scheme_list(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut picked = false;
    for (i, s) in SCHEMES.iter().enumerate() {
        if i == 0 || SCHEMES[i - 1].group != s.group {
            if i > 0 {
                ui.separator();
            }
            ui.label(RichText::new(s.group).small().weak());
        }
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(28.0, 16.0), Sense::hover());
            let bg = Color32::from_rgb(s.bg[0], s.bg[1], s.bg[2]);
            ui.painter().rect(rect, 3.0_f32, bg, Stroke::new(1.0_f32, Color32::GRAY));
            let dot = Rect::from_center_size(rect.center(), vec2(10.0, 8.0));
            ui.painter().rect_filled(dot, 2.0_f32, Color32::from_rgb(s.ink[0], s.ink[1], s.ink[2]));
            if ui.selectable_label(settings.is_scheme(s), s.name).clicked() {
                settings.use_scheme(s);
                picked = true;
            }
        });
    }
    picked
}

/// A grid of large colour swatches with their names; returns true when one was picked.
fn swatch_grid(ui: &mut egui::Ui, list: &[(&str, [u8; 3])], cur: &mut [u8; 3], outline: bool) -> bool {
    let mut picked = false;
    egui::Grid::new(ui.next_auto_id()).spacing([8.0, 8.0]).show(ui, |ui| {
        for (i, (name, c)) in list.iter().enumerate() {
            ui.vertical(|ui| {
                ui.set_width(64.0);
                let (rect, resp) = ui.allocate_exact_size(vec2(64.0, 40.0), Sense::click());
                let col = Color32::from_rgb(c[0], c[1], c[2]);
                ui.painter().rect_filled(rect, 8.0_f32, col);
                if outline || resp.hovered() {
                    ui.painter().rect_stroke(rect, 8.0_f32, Stroke::new(1.0_f32, Color32::GRAY));
                }
                if *cur == *c {
                    ui.painter().rect_stroke(rect.expand(3.0), 10.0_f32, Stroke::new(2.5_f32, ui.visuals().text_color()));
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, "\u{2713}", FontId::proportional(20.0), on_accent(col));
                }
                ui.label(RichText::new(*name).small());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    *cur = *c;
                    picked = true;
                }
            });
            if (i + 1) % 6 == 0 {
                ui.end_row();
            }
        }
    });
    picked
}

struct Recorder {
    _stream: cpal::Stream,
    buf: Arc<Mutex<Vec<i16>>>,
    channels: u16,
    rate: u32,
    started: std::time::Instant,
    person: u32,
    peak: Arc<AtomicU32>,
}

struct Dlg {
    kind: Kind,
    name: String,
    sex: String,
    target: Option<u32>,
    fresh: u8,
    bd: String,
    bp: String,
    dd: String,
    dp: String,
    bud: String,
    bup: String,
    living: bool,
    notes: String,
}

impl Dlg {
    fn new(kind: Kind, sex: &str, target: Option<u32>) -> Dlg {
        Dlg { kind, name: String::new(), sex: sex.to_string(), target, fresh: 2, bd: String::new(), bp: String::new(), dd: String::new(), dp: String::new(), bud: String::new(), bup: String::new(), living: false, notes: String::new() }
    }
}

/// The small "Quick edit" window opened from a chart box.
struct QuickEdit {
    id: u32,
    given: String,
    surname: String,
    sex: String,
    bd: String,
    bp: String,
    dd: String,
    dp: String,
    living: bool,
}

/// Screen state for the newer tools (kept together so App's long constructor stays readable).
#[derive(Default)]
struct Extra {
    /// Gallery/Memories: choosing several items to remove at once
    gal_pick: bool,
    gal_sel: HashSet<(u32, usize)>,
    gal_confirm: bool,
    jump_open: bool,
    jump_q: String,
    jump_sel: usize,
    todo_open: bool,
    todo_find: String,
    todo_done: bool,
    todo_new: String,
    places_open: bool,
    places_find: String,
    places_sel: Option<String>,
    places_rename: String,
    places_by_count: bool,
    history_open: bool,
    /// What the tree looked like at the last change, to describe the next one in History.
    prev_names: HashMap<u32, String>,
    prev_sel: Option<(u32, String)>,
    prev_fams: usize,
    /// A description an action leaves for the History list ("Changed place spelling...").
    next_label: Option<String>,
    /// Face tagging in the photo viewer: on/off, the box being drawn (0..1 of the picture), who to tag.
    tag_mode: bool,
    tag_drag: Option<(egui::Pos2, egui::Pos2)>,
    tag_box: Option<[f32; 4]>,
    tag_q: String,
    crop_mode: bool,
    pdf: Option<pdfview::PdfView>,
    pdf_req: Option<(PathBuf, String)>,
    licenses_open: bool,
    sized: bool,
    backup_open: bool,
    backup_check: f64,
    arch_job: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    arch_confirm: Option<(PathBuf, archive::ArchiveInfo)>,
    paf_help: bool,
    designer_open: bool,
    design: designer::Design,
    trees_open: bool,
    tree_new: String,
    tree_rename: Option<(String, String)>,
    stats_open: bool,
    stats: Option<((usize, usize), stats::Stats)>,
    map_surname: String,
    journal_open: bool,
    journal_new: Option<Journal>,
    journal_edit: Option<usize>,
    journal_find: String,
    hist_open: bool,
    hist_new: (i32, i32, String),
    /// While exploring the sample family: the real tree and selection, put back afterwards.
    practice: Option<(Db, Option<u32>, View)>,
    gal_kind: u8,
    map_who: u8,
    map_types: [bool; 5],
    map_year: i32,
    map_play: bool,
    map_tick: f64,
    map_undated: bool,
    map_zoom: f32,
    map_center: [f32; 2],
    map_fit: bool,
    map_pick: Option<(i32, i32)>,
    map_setting: Option<String>,
    geo_cache: HashMap<String, Option<([f32; 2], mapview::Prec)>>,
    story: Option<StoryEdit>,
    chart: ChartKind,
    fan_gens: usize,
    fan_deg: f32,
    fan_menu: Option<u32>,
    /// Data-checker results for the person on screen, and what they were worked out from.
    flags: Vec<String>,
    flags_key: (u32, usize, usize),
    table_find: String,
    table_col: table::Col,
    table_desc: bool,
    table_only: table::Only,
    layout_name: String,
    ver_open: bool,
    ver: Option<(String, Db, (Vec<u32>, Vec<u32>, Vec<u32>))>,
    /// Tools > Tombstone calculator.
    tomb: tombstone::Tomb,
    /// Reports > Forms > Blank census forms.
    census: census::CensusUi,
    /// The DNA tab.
    dna: dna::DnaUi,
    /// Y-DNA and mtDNA in the DNA tab.
    line: dnaline::LineUi,
}

/// One step of undo history: the whole tree as it was (packed small), what the next change was,
/// and when it happened.
struct Snap {
    z: Vec<u8>,
    label: String,
    at: std::time::Instant,
}

fn pack(json: &str) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(json.as_bytes(), 1)
}

fn unpack(z: &[u8]) -> String {
    miniz_oxide::inflate::decompress_to_vec(z).ok().and_then(|b| String::from_utf8(b).ok()).unwrap_or_default()
}

fn when_ago(t: std::time::Instant) -> String {
    let s = t.elapsed().as_secs();
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        _ => format!("{} h {} min ago", s / 3600, (s % 3600) / 60),
    }
}

/// What a quick-jump line does.
enum Jump {
    Act(Action),
    View(View),
    Window(&'static str),
}

/// The person being edited on the Individual page, and how they looked before Edit was pressed.
struct EditSession {
    id: u32,
    snapshot: String,
}

enum Action {
    Select(u32),
    Fit,
    ViewTree(u32, u8),
    QuickEdit(u32),
    EditStart,
    EditSave,
    EditCancel,
    SetHome(u32),
    ShowParents(u32),
    ShowParentsFor(u32, u32),
    GoHome,
    Add(Kind),
    Delete,
    Fx(usize),
    Open(u32),
    Highlight(u32),
    AddFor(u32, Kind),
    SetGens(usize),
    /// Turn the hand tool on the charts on or off
    SetHand(bool),
    Back,
    Forward,
    PrintPedigree,
    PrintFamily,
    AttachFiles,
    StartRec,
    StopRec,
    SetZoom(f32),
    ReportAncestors,
    ReportDescendants,
    ReportBirthdays,
    ReportSources,
    ReportBook,
    PrintDescendants,
    PrintChart,
    Bookmark(u32),
}

struct App {
    db: Db,
    sel: Option<u32>,
    view: View,
    search: String,
    fx: usize,
    dlg: Option<Dlg>,
    confirm: bool,
    status: String,
    dirty: bool,
    find_open: bool,
    settings: Settings,
    show_look: bool,
    show_loc: bool,
    review: Option<PersonReview>,
    imerge: Option<ImportMerge>,
    src_open: bool,
    src_sel: Option<u32>,
    src_tab: u8,
    src_filter: String,
    repo_sel: Option<u32>,
    src_pick: Option<u32>,
    src_merge: Option<u32>,
    src_confirm: bool,
    hl: Option<u32>,
    back: Vec<u32>,
    fwd: Vec<u32>,
    wipe: Option<Wipe>,
    show_about: bool,
    help_open: bool,
    help_find: String,
    move_data: bool,
    imp_open: bool,
    imp_src: usize,
    imp_replace: bool,
    rec: Option<Recorder>,
    rel_open: bool,
    rel_a: Option<u32>,
    rel_b: Option<u32>,
    viewing: Option<(u32, usize)>,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    last_json: String,
    last_snap: std::time::Instant,
    last_backup_day: String,
    check_open: bool,
    dup_open: bool,
    merge_open: bool,
    merge_id: String,
    merge_other: Option<u32>,
    dups: Vec<(u32, u32, String)>,
    dup_ignore: HashSet<(u32, u32)>,
    save_error: Option<String>,
    last_saved: Option<std::time::Instant>,
    next_save: std::time::Instant,
    notice: Option<String>,
    media_job: Option<std::sync::mpsc::Receiver<(bool, usize, usize, Option<String>)>>,
    startup_done: bool,
    verify_open: bool,
    verify_results: Vec<(String, Result<usize, String>)>,
    media_gap: String,
    close_warned: bool,
    report: Option<ImportReport>,
    report_open: bool,
    link: Option<LinkReq>,
    fit_req: bool,
    editing: Option<EditSession>,
    quick: Option<QuickEdit>,
    gal_only: bool,
    home_pick: Option<String>,
    gal_find: String,
    assoc_q: String,
    assoc_role: String,
    x: Extra,
}

fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf();
    if exe.to_string_lossy().contains(".app/Contents/MacOS") {
        for _ in 0..3 {
            dir = dir.parent()?.to_path_buf();
        }
    }
    Some(dir.join("FamilyTreeData"))
}

/// macOS runs a downloaded, unsigned app from a hidden temporary copy ("App Translocation"),
/// away from the FamilyTreeData folder beside it. Spot that, to explain the one-time fix.
fn mac_translocated() -> bool {
    cfg!(target_os = "macos") && std::env::current_exe().map(|e| e.to_string_lossy().contains("/AppTranslocation/")).unwrap_or(false)
}

fn is_portable() -> bool {
    portable_dir().map(|p| p.is_dir()).unwrap_or(false)
}

/// The hidden per-user folder: settings, and the pointer to a custom data folder.
fn config_dir() -> PathBuf {
    if let Some(p) = portable_dir() {
        if p.is_dir() {
            return p;
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let base = (if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library").join("Application Support"))
    } else {
        home.map(|h| h.join(".local").join("share"))
    })
    .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("FamilyTree");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Is this folder probably kept in sync by iCloud, OneDrive or Dropbox? (A best guess from the path.)
fn synced_service(dir: &std::path::Path) -> Option<&'static str> {
    let canon = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let s = format!("{} {}", dir.to_string_lossy(), canon.to_string_lossy()).to_lowercase();
    if s.contains("mobile documents") || s.contains("clouddocs") || s.contains("icloud") {
        return Some("iCloud Drive");
    }
    if s.contains("onedrive") {
        return Some("OneDrive");
    }
    if s.contains("dropbox") {
        return Some("Dropbox");
    }
    if cfg!(target_os = "macos") {
        // With "Desktop & Documents Folders" turned on, iCloud keeps a Documents folder of its own.
        if let Some(h) = std::env::var_os("HOME").map(PathBuf::from) {
            let icd = h.join("Library").join("Mobile Documents").join("com~apple~CloudDocs").join("Documents");
            if icd.is_dir() && dir.starts_with(h.join("Documents")) {
                return Some("iCloud Drive");
            }
        }
    }
    None
}

fn documents_dir() -> PathBuf {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let docs = home.join("Documents");
    if docs.is_dir() { docs } else { home }
}

/// Count files under a folder (used to check a copy is complete).
fn count_files(d: &std::path::Path) -> usize {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.flatten() {
            let p = e.path();
            n += if p.is_dir() { count_files(&p) } else { 1 };
        }
    }
    n
}

/// One-time move of the tree from the old hidden folder to the visible one. The old copy is left in
/// place (marked) until the new copy has been checked, and is never deleted by the app.
fn migrate_old_data(old: &std::path::Path, new: &std::path::Path) -> bool {
    if new.join("tree.json").exists() || !old.join("tree.json").exists() || old.join("MOVED.txt").exists() || old == new {
        return true;
    }
    let existed = new.exists();
    let copy = (|| -> std::io::Result<bool> {
        std::fs::create_dir_all(new)?;
        for e in std::fs::read_dir(old)? {
            let e = e?;
            let name = e.file_name();
            if name == "settings.json" || name == "location.txt" || name == "MOVED.txt" {
                continue;
            }
            let (p, t) = (e.path(), new.join(&name));
            if p.is_dir() {
                copy_dir(&p, &t)?;
            } else {
                std::fs::copy(&p, &t)?;
            }
        }
        let same = std::fs::read(old.join("tree.json"))? == std::fs::read(new.join("tree.json"))?;
        let files = |d: &std::path::Path| count_files(&d.join("media")) + count_files(&d.join("backups"));
        Ok(same && files(old) == files(new))
    })();
    match copy {
        Ok(true) => {
            let _ = std::fs::write(
                old.join("MOVED.txt"),
                format!("Your family tree now lives in:\n{}\nThis old copy is no longer used. You can delete this folder once you are happy with the new one.\n", new.display()),
            );
            true
        }
        _ => {
            if !existed {
                let _ = std::fs::remove_dir_all(new);
            } else {
                let _ = std::fs::remove_file(new.join("tree.json"));
            }
            false
        }
    }
}

/// The tree being worked on when it is not the main one (File > Family trees).
static CURRENT_TREE: Mutex<Option<String>> = Mutex::new(None);

/// The folder of the tree that is open: the main folder, or a folder under "More trees".
fn data_dir() -> PathBuf {
    let base = base_dir();
    match CURRENT_TREE.lock().ok().and_then(|g| g.clone()) {
        Some(name) if !name.is_empty() => base.join("More trees").join(name),
        _ => base,
    }
}

fn tree_title(name: &str) -> String {
    if name.is_empty() { "My family tree".to_string() } else { name.to_string() }
}

/// A tree name that is safe as a folder name.
fn sanitize_tree_name(s: &str) -> String {
    let t: String = s.trim().chars().filter(|c| c.is_alphanumeric() || " -_'&().".contains(*c)).collect();
    let t = t.trim().trim_matches('.').to_string();
    if t.len() > 60 { t.chars().take(60).collect() } else { t }
}

/// Names of the extra trees (folders under "More trees" that hold a tree).
fn other_trees() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(base_dir().join("More trees"))
        .map(|r| r.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    v.sort_by_key(|n| n.to_lowercase());
    v
}

/// The main folder (Documents/Gilded Roots, the portable folder, or the one chosen in settings).
fn base_dir() -> PathBuf {
    if let Some(p) = portable_dir() {
        if p.is_dir() {
            return p;
        }
    }
    let cfg = config_dir();
    if let Ok(s) = std::fs::read_to_string(cfg.join("location.txt")) {
        let p = PathBuf::from(s.trim());
        if p.is_dir() || std::fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    let dir = documents_dir().join("Gilded Roots");
    if !migrate_old_data(&cfg, &dir) {
        // Could not move safely: keep using the old folder rather than risk the data.
        return cfg;
    }
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn store_path() -> PathBuf {
    data_dir().join("tree.json")
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let p = entry.path();
        let t = dst.join(entry.file_name());
        if p.is_dir() {
            copy_dir(&p, &t)?;
        } else {
            std::fs::copy(&p, &t)?;
        }
    }
    Ok(())
}

fn stamp() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    format!("{}-{}", ms, N.fetch_add(1, Ordering::Relaxed))
}

fn media_dir() -> PathBuf {
    let d = store_path().with_file_name("media");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn ext_of(f: &str) -> String {
    f.rsplit('.').next().unwrap_or("").to_lowercase()
}

fn is_image(f: &str) -> bool {
    matches!(ext_of(f).as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp")
}

/// File types the program keeps as photos, documents and recordings.
fn is_media_name(f: &str) -> bool {
    matches!(
        ext_of(f).as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "heif" | "tif" | "tiff" | "pdf" | "doc" | "docx" | "rtf" | "txt" | "odt" | "htm" | "html" | "wav" | "mp3" | "m4a" | "ogg" | "flac" | "aac" | "mp4" | "mov" | "avi"
    )
}

/// A stored media file name that stays inside the media folder (no folders, no "..").
fn safe_media_path(file: &str) -> Option<PathBuf> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file.contains("..") || file.contains(':') {
        return None;
    }
    Some(media_dir().join(file))
}

/// The full path of a stored media file, kept inside the media folder whatever the stored name
/// says (a tree from someone else could hold a name like "C:\\Windows\\..." or "../../x").
/// An unsafe name gives a path that does not exist, so nothing outside the folder is ever read.
fn mfile(file: &str) -> PathBuf {
    safe_media_path(file).unwrap_or_else(|| media_dir().join("previews").join("-missing-"))
}

/// The same for a preview picture (thumbnail or mid-size) in the previews folder.
fn mprev(file: &str) -> PathBuf {
    match safe_media_path(file) {
        Some(_) => media_dir().join("previews").join(file),
        None => media_dir().join("previews").join("-missing-"),
    }
}

/// Open a stored media file in the computer's own app, but only a media file inside the media
/// folder (never a program or a file somewhere else on the computer).
fn open_media(file: &str) {
    if is_media_name(file) {
        if let Some(p) = safe_media_path(file) {
            if p.is_file() {
                open_path(&p);
            }
        }
    }
}

fn is_pdf(f: &str) -> bool {
    ext_of(f) == "pdf"
}

fn is_audio(f: &str) -> bool {
    matches!(ext_of(f).as_str(), "wav" | "mp3" | "m4a" | "ogg" | "flac" | "aac")
}

fn sanitize(n: &str) -> String {
    n.chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }).collect()
}

fn import_one(p: &std::path::Path, name: &str) -> Option<(String, String)> {
    let ext = ext_of(name);
    if cfg!(target_os = "macos") && (ext == "heic" || ext == "heif") {
        let stem = std::path::Path::new(name).file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| "photo".to_string());
        let file = format!("{}-{}.jpg", stamp(), sanitize(&stem));
        let ok = std::process::Command::new("sips")
            .args(["-s", "format", "jpeg"])
            .arg(p)
            .arg("--out")
            .arg(media_dir().join(&file))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some((file, format!("{}.jpg", stem)));
        }
    }
    let file = format!("{}-{}", stamp(), sanitize(name));
    std::fs::copy(p, media_dir().join(&file)).ok()?;
    Some((file, name.to_string()))
}

/// Where file pickers start: the folder last used, otherwise the person's Documents or home folder
/// (never the app's own hidden data folder).
static LAST_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

fn start_dir() -> PathBuf {
    if let Some(d) = LAST_DIR.lock().ok().and_then(|g| g.clone()) {
        if d.is_dir() {
            return d;
        }
    }
    documents_dir()
}

fn remember_dir(file: &Path) {
    if let (Some(d), Ok(mut g)) = (file.parent(), LAST_DIR.lock()) {
        *g = Some(d.to_path_buf());
    }
}

/// Read a GEDCOM file: decode it, parse it, copy in the photos it refers to, and check what an export would lose.
fn read_ged_file(path: &Path) -> Result<(Db, ImportReport), String> {
    let b = std::fs::read(path).map_err(|e| format!("Could not read file: {}", e))?;
    if b.is_empty() {
        return Err("That file is empty.".to_string());
    }
    // One of this app's own tree files (tree.json or a backup)?
    let trimmed = b.iter().skip_while(|c| c.is_ascii_whitespace() || **c == 0xEF || **c == 0xBB || **c == 0xBF).next();
    if trimmed == Some(&b'{') {
        return match serde_json::from_slice::<Db>(&b) {
            Ok(mut db) if !db.people.is_empty() => {
                migrate(&mut db);
                let rep = ImportReport {
                    file: path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default(),
                    program: "a saved tree file (JSON)".to_string(),
                    charset: "UTF-8".to_string(),
                    people: db.people.len(),
                    families: db.fams.len(),
                    sources: db.sources.len(),
                    notes: db.notes.len(),
                    ..Default::default()
                };
                Ok((db, rep))
            }
            Ok(_) => Err("That JSON file has no people in it.".to_string()),
            Err(_) => Err("That is a JSON file, but not a family tree saved by this app, so it can't be read. If it came from another program, export a GEDCOM (.ged) file from that program instead.".to_string()),
        };
    }
    // A PAF 5 database: read it directly.
    if paf::is_paf(&b) {
        let (text, notes) = paf::to_gedcom(&b)?;
        let (db, mut rep) = parse_ged_report(&text);
        if db.people.is_empty() {
            return Err("This PAF file has no people in it.".to_string());
        }
        rep.file = path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        rep.program = "PAF 5 database (.paf), read directly".to_string();
        rep.charset = "ANSI".to_string();
        if notes.odd_dates > 0 {
            rep.warnings.push(format!("{} dates were stored in a way this reader doesn't fully understand yet. They came in as full dates; please check them against PAF.", notes.odd_dates));
        }
        if notes.unread > 0 {
            rep.warnings.push(format!("{} people or families have extra details (such as christenings, burials, LDS ordinances, notes or sources) this reader can't read from a .paf file yet. To bring those in, export a GEDCOM from PAF and import that instead.", notes.unread));
        }
        if notes.odd_links > 0 {
            rep.warnings.push(format!("{} children are linked to parents as something other than a birth child (adopted, sealed, guardian and so on). They came in as ordinary children; please check those links.", notes.odd_links));
        }
        rep.lost = loss_check(&text, &to_ged(&db));
        return Ok((db, rep));
    }
    // A program's own data file (PAF's .paf, for example) rather than a GEDCOM export?
    let nuls = b.iter().take(4096).filter(|c| **c == 0).count();
    let head: String = b.iter().take(4000).map(|&c| c as char).collect();
    let looks_text = nuls < 20 && (head.contains("0 HEAD") || head.contains("\n0 @") || head.contains("\r0 @") || head.starts_with("0 "));
    let is_utf16 = b.starts_with(&[0xFF, 0xFE]) || b.starts_with(&[0xFE, 0xFF]);
    if !looks_text && !is_utf16 && nuls >= 20 {
        return Err("This is a program's own data file, not a GEDCOM file, so it can't be read directly. Open it in the program that made it (for PAF: File > Export, choose GEDCOM), save the .ged file, then import that here.".to_string());
    }
    let (text, charset) = decode_ged(&b);
    let (mut db, mut rep) = parse_ged_report(&text);
    if db.people.is_empty() {
        return Err("This file is text, but it has no people in a GEDCOM layout. Make sure it is the GEDCOM (.ged) file the other program exported, not a report or a different kind of export.".to_string());
    }
    rep.file = path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
    rep.charset = charset;
    resolve_media(&mut db, path.parent().unwrap_or(Path::new(".")), &mut rep);
    rep.lost = loss_check(&text, &to_ged(&db));
    Ok((db, rep))
}

/// Write the tree as a GEDCOM file. Photos and documents are copied into a folder next to it and the file
/// points to them. The file is read back and compared with the tree, so a problem is never silent.
/// Ok and Err both carry the message to show.
fn write_gedcom(db: &Db, path: &Path, private: bool) -> Result<String, String> {
    let out = if private { db.sanitized() } else { db.clone() };
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "gilded-roots".to_string());
    let has_media = out.people.values().any(|p| p.media.iter().any(|m| !m.file.is_empty())) || out.fams.values().any(|f| f.media.iter().any(|m| !m.file.is_empty()));
    let folder = format!("{}-media", sanitize(&stem));
    let ex = to_ged_full(&out, &ExportOpts { prune: private, media_dir: if has_media { Some(folder.clone()) } else { None } });
    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let (mut copied, mut missing) = (0usize, 0usize);
    let mut failed: Option<String> = None;
    if has_media {
        let _ = std::fs::create_dir_all(dir.join(&folder));
        for (file, rel) in &ex.media {
            let src = mfile(file);
            let dst = dir.join(rel);
            if !src.exists() {
                missing += 1;
            } else if let Err(e) = std::fs::copy(&src, &dst) {
                failed = Some(io_msg(&e));
            } else {
                copied += 1;
            }
        }
    }
    atomic_write(path, ex.text.as_bytes()).map_err(|e| format!("Could not save: {}", e))?;
    let mut s = format!("Saved {}", path.display());
    let mut problem = false;
    if has_media {
        s.push_str(&format!(" with {} photo/document file(s) in the folder {}", copied, folder));
        if missing > 0 {
            s.push_str(&format!(" ({} file(s) were missing and could not be copied)", missing));
            problem = true;
        }
        if let Some(e) = failed {
            s.push_str(&format!(" (some files could not be copied: {})", e));
            problem = true;
        }
    }
    match std::fs::read(path) {
        Ok(b) => {
            let (back, _) = decode_ged(&b);
            let (db2, _) = parse_ged_report(&back);
            let lost = loss_check(&to_ged_full(&out, &ExportOpts { prune: private, media_dir: None }).text, &back);
            if db2.people.len() != out.people.len() || db2.fams.len() != out.families_count() || !lost.is_empty() {
                s.push_str(&format!(". WARNING: reading the saved file back found {} people and {} families (your tree has {} and {}){}.", db2.people.len(), db2.fams.len(), out.people.len(), out.families_count(), if lost.is_empty() { String::new() } else { format!(", and {} piece(s) differ", lost.len()) }));
                problem = true;
            }
        }
        Err(e) => {
            s.push_str(&format!(". WARNING: could not read the file back to check it: {}", e));
            problem = true;
        }
    }
    if problem { Err(s) } else { Ok(s) }
}

/// Photos and documents that a GEDCOM file refers to: copy in the ones that can be found
/// (next to the file, or at the path it gives) and keep the reference for the rest.
fn resolve_media(db: &mut Db, base: &Path, rep: &mut ImportReport) {
    let mut fix = |m: &mut Media| {
        if !m.file.is_empty() || m.path.is_empty() {
            return;
        }
        if m.path.starts_with("http://") || m.path.starts_with("https://") {
            rep.media_missing += 1;
            return;
        }
        let norm = m.path.replace('\\', "/");
        let base_name = norm.rsplit('/').next().unwrap_or("").to_string();
        // A GEDCOM file can name any file on the computer. Only photos, documents and
        // recordings are copied in, so a shared file cannot pull in other private files.
        if !is_media_name(&base_name) {
            rep.media_missing += 1;
            return;
        }
        let candidates = [PathBuf::from(&m.path), base.join(&norm), base.join(&base_name)];
        for c in candidates.iter() {
            if c.is_file() {
                if let Some((file, shown)) = import_one(c, &base_name) {
                    m.file = file;
                    m.name = shown;
                    m.path.clear();
                    rep.media_found += 1;
                    return;
                }
            }
        }
        rep.media_missing += 1;
    };
    for p in db.people.values_mut() {
        for m in p.media.iter_mut() {
            fix(m);
        }
    }
    for f in db.fams.values_mut() {
        for m in f.media.iter_mut() {
            fix(m);
        }
    }
    let online = db.people.values().flat_map(|p| p.media.iter()).chain(db.fams.values().flat_map(|f| f.media.iter())).filter(|m| is_url(&m.path)).count();
    if online > 0 {
        rep.warnings.push(format!(
            "{} photos/documents are links to a website (for example Ancestry), not files. The links are kept and can be opened in your browser, but the pictures themselves are not in the GEDCOM file. To bring them in, download them and add them with Add photos or documents.",
            online
        ));
    }
}

const USER_GUIDE: &str = include_str!("../docs/USER_GUIDE.md");

/// Draw the user guide (simple Markdown: headings, bullets, **bold**), showing only the
/// sections that mention `find` when it is not empty.
fn help_text(ui: &mut egui::Ui, find: &str) {
    let q = find.trim().to_lowercase();
    let mut sections: Vec<(String, Vec<&str>)> = vec![(String::new(), Vec::new())];
    for line in USER_GUIDE.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            sections.push((h.to_string(), Vec::new()));
        } else if line.starts_with("# ") {
            continue;
        } else {
            sections.last_mut().unwrap().1.push(line);
        }
    }
    let mut shown = 0;
    for (head, body) in &sections {
        if !q.is_empty() && !head.to_lowercase().contains(&q) && !body.iter().any(|l| l.to_lowercase().contains(&q)) {
            continue;
        }
        if head.is_empty() && body.iter().all(|l| l.trim().is_empty()) {
            continue;
        }
        shown += 1;
        if !head.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(head).size(18.0).strong());
        }
        let mut para = String::new();
        let flush = |ui: &mut egui::Ui, para: &mut String| {
            if !para.trim().is_empty() {
                ui.add(egui::Label::new(RichText::new(para.replace("**", "").replace('`', "")).size(14.0)).wrap());
                ui.add_space(4.0);
            }
            para.clear();
        };
        for l in body {
            let t = l.trim();
            if t.is_empty() {
                flush(ui, &mut para);
            } else if let Some(b) = t.strip_prefix("- ") {
                flush(ui, &mut para);
                ui.horizontal_wrapped(|ui| {
                    ui.label("  \u{2022}");
                    ui.add(egui::Label::new(RichText::new(b.replace("**", "").replace('`', "")).size(14.0)).wrap());
                });
            } else {
                if !para.is_empty() {
                    para.push(' ');
                }
                para.push_str(t);
            }
        }
        flush(ui, &mut para);
    }
    if shown == 0 {
        ui.label(RichText::new("Nothing in the guide matches that.").italics().weak());
    }
}

impl Db {
    /// Every place written anywhere in the tree, with who and what it is attached to.
    fn place_uses(&self) -> BTreeMap<String, Vec<(u32, String, String)>> {
        let mut m: BTreeMap<String, Vec<(u32, String, String)>> = BTreeMap::new();
        let mut add = |pl: &str, id: u32, what: &str, date: &str| {
            let pl = pl.trim();
            if !pl.is_empty() {
                m.entry(pl.to_string()).or_default().push((id, what.to_string(), date.trim().to_string()));
            }
        };
        for (id, p) in &self.people {
            add(&p.bp, *id, "Born", &p.bd);
            add(&p.chp, *id, "Christened", &p.chd);
            add(&p.dp, *id, "Died", &p.dd);
            add(&p.bup, *id, "Buried", &p.bud);
            for e in &p.events {
                add(&e.place, *id, if e.kind.is_empty() { "Event" } else { e.kind.as_str() }, &e.date);
            }
        }
        for f in self.fams.values() {
            for who in [f.h, f.w].into_iter().flatten() {
                add(&f.mp, who, "Married", &f.md);
                for e in &f.events {
                    add(&e.place, who, if e.kind.is_empty() { "Family event" } else { e.kind.as_str() }, &e.date);
                }
            }
        }
        m
    }

    /// Change one spelling of a place to another everywhere. Returns how many were changed.
    fn rename_place(&mut self, from: &str, to: &str) -> usize {
        let (from, to) = (from.trim(), to.trim());
        if from.is_empty() || to.is_empty() || from == to {
            return 0;
        }
        let mut n = 0;
        let mut fix = |s: &mut String| {
            if s.trim() == from {
                *s = to.to_string();
                n += 1;
            }
        };
        for p in self.people.values_mut() {
            fix(&mut p.bp);
            fix(&mut p.chp);
            fix(&mut p.dp);
            fix(&mut p.bup);
            for e in p.events.iter_mut() {
                fix(&mut e.place);
            }
        }
        for f in self.fams.values_mut() {
            fix(&mut f.mp);
            for e in f.events.iter_mut() {
                fix(&mut e.place);
            }
        }
        n
    }
}

/// A place spelling reduced to letters and digits, so "Portland, Oregon" and "portland,oregon."
/// are seen as the same. "USA", "U.S.A." and "United States" all end the same way.
fn place_key(s: &str) -> String {
    let mut t: String = s.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
    t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    for tail in [" united states of america", " united states", " u s a", " usa", " us"] {
        if let Some(x) = t.strip_suffix(tail) {
            t = x.to_string();
            break;
        }
    }
    t.replace(' ', "")
}

/// The part of a picture to show in a frame of aspect `target` (width / height) without
/// stretching: the box `r` grown just enough, around its centre, to match.
fn crop_uv(r: [f32; 4], img_aspect: f32, target: f32) -> Rect {
    let [x, y, mut w, mut h] = r;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let ia = if img_aspect > 0.0 { img_aspect } else { 1.0 };
    let cur = (w * ia) / h.max(1e-6);
    if cur > target {
        h = (w * ia / target).min(1.0);
    } else {
        w = (h * target / ia).min(1.0);
    }
    let x0 = (cx - w / 2.0).clamp(0.0, 1.0 - w);
    let y0 = (cy - h / 2.0).clamp(0.0, 1.0 - h);
    Rect::from_min_size(pos2(x0, y0), vec2(w, h))
}

/// Draw a picture (or the face part of it) filling `rect`, cropped rather than squashed.
fn put_photo(ui: &mut egui::Ui, rect: Rect, uri: &str, r: [f32; 4], rounding: f32) {
    let img = egui::Image::new(uri.to_string());
    let aspect = match img.load_for_size(ui.ctx(), rect.size()) {
        Ok(egui::load::TexturePoll::Ready { texture }) => texture.size.x / texture.size.y.max(1.0),
        _ => 1.0,
    };
    let uv = crop_uv(r, aspect, rect.width() / rect.height().max(1.0));
    ui.put(rect, egui::Image::new(uri.to_string()).uv(uv).fit_to_exact_size(rect.size()).rounding(rounding));
}

/// Which Gallery group a media item belongs to: 1 photo, 2 document or PDF, 3 recording, 4 story.
fn media_kind(m: &Media) -> u8 {
    let form = m.form.to_lowercase();
    if form == "story" || m.extra.iter().any(|r| r.text.contains("TYPE story")) {
        4
    } else if is_image(&m.file) || online_photo(m) {
        1
    } else if is_audio(&m.file) {
        3
    } else {
        2
    }
}

/// A story being written or read in the story window.
struct StoryEdit {
    who: u32,
    idx: Option<usize>,
    title: String,
    text: String,
}

/// A picture that lives on a website (Ancestry and others put links, not files, in their GEDCOM).
fn online_photo(m: &Media) -> bool {
    m.file.is_empty() && is_url(&m.path) && matches!(m.form.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tif" | "tiff" | "webp")
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Take several photos, documents or recordings off the people they belong to, in one go.
/// The files are left in the media folder (and backups); a profile photo that was removed is
/// cleared so no box points at it. Returns how many were removed.
fn remove_media_items(db: &mut Db, sel: &HashSet<(u32, usize)>) -> usize {
    let mut by: HashMap<u32, Vec<usize>> = HashMap::new();
    for (who, i) in sel {
        by.entry(*who).or_default().push(*i);
    }
    let mut n = 0;
    for (who, mut idx) in by {
        let Some(p) = db.people.get_mut(&who) else { continue };
        idx.sort_unstable_by(|a, b| b.cmp(a));
        idx.dedup();
        for i in idx {
            if i < p.media.len() {
                let m = p.media.remove(i);
                if !m.file.is_empty() && p.portrait == m.file {
                    p.portrait.clear();
                    p.face_portrait = None;
                    p.portrait_crop = None;
                }
                n += 1;
            }
        }
    }
    n
}

fn make_previews(src: &std::path::Path) -> Option<(String, String)> {
    use image::ImageDecoder;
    let reader = image::ImageReader::open(src).ok()?.with_guessed_format().ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let orient = decoder.orientation().ok();
    let mut img = image::DynamicImage::from_decoder(decoder).ok()?;
    if let Some(o) = orient {
        img.apply_orientation(o);
    }
    let dir = media_dir().join("previews");
    let _ = std::fs::create_dir_all(&dir);
    let id = stamp();
    let thumb = format!("{}-t.jpg", id);
    let mid = format!("{}-m.jpg", id);
    img.thumbnail(320, 320).to_rgb8().save(dir.join(&thumb)).ok()?;
    img.thumbnail(1600, 1600).to_rgb8().save(dir.join(&mid)).ok()?;
    Some((thumb, mid))
}

fn input_device_names() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|d| d.filter_map(|x| x.name().ok()).collect())
        .unwrap_or_default()
}

fn try_device(dev: &cpal::Device, buf: &Arc<Mutex<Vec<i16>>>, peak: &Arc<AtomicU32>) -> Result<(cpal::Stream, u16, u32), String> {
    let cfg = match dev.default_input_config() {
        Ok(c) => c,
        Err(e1) => match dev.supported_input_configs() {
            Ok(mut it) => match it.next() {
                Some(r) => r.with_max_sample_rate(),
                None => return Err(e1.to_string()),
            },
            Err(e2) => return Err(format!("{} / {}", e1, e2)),
        },
    };
    let (sf, channels, rate) = (cfg.sample_format(), cfg.channels(), cfg.sample_rate().0);
    let config: cpal::StreamConfig = cfg.into();
    let b = buf.clone();
    let pk = peak.clone();
    let err_fn = |e: cpal::StreamError| eprintln!("audio error: {}", e);
    let stream = match sf {
        cpal::SampleFormat::F32 => dev.build_input_stream(
            &config,
            move |d: &[f32], _: &cpal::InputCallbackInfo| {
                let m = d.iter().fold(0f32, |a, x| a.max(x.abs()));
                pk.fetch_max((m * 32767.0) as u32, Ordering::Relaxed);
                if let Ok(mut v) = b.lock() {
                    v.extend(d.iter().map(|x| (x.clamp(-1.0, 1.0) * 32767.0) as i16));
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => dev.build_input_stream(
            &config,
            move |d: &[i16], _: &cpal::InputCallbackInfo| {
                let m = d.iter().map(|x| x.unsigned_abs() as u32).max().unwrap_or(0);
                pk.fetch_max(m, Ordering::Relaxed);
                if let Ok(mut v) = b.lock() {
                    v.extend_from_slice(d);
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => dev.build_input_stream(
            &config,
            move |d: &[u16], _: &cpal::InputCallbackInfo| {
                let m = d.iter().map(|x| (*x as i32 - 32768).unsigned_abs()).max().unwrap_or(0);
                pk.fetch_max(m, Ordering::Relaxed);
                if let Ok(mut v) = b.lock() {
                    v.extend(d.iter().map(|x| (*x as i32 - 32768) as i16));
                }
            },
            err_fn,
            None,
        ),
        _ => return Err("unsupported audio format".to_string()),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, channels, rate))
}

fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let z = secs / 86400 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/* ---------- safe storage: atomic saves, verified backups, recovery ---------- */

fn fnv64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for x in b {
        h ^= *x as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn is_disk_full(e: &std::io::Error) -> bool {
    if cfg!(windows) {
        matches!(e.raw_os_error(), Some(112) | Some(39))
    } else {
        e.raw_os_error() == Some(28)
    }
}

fn io_msg(e: &std::io::Error) -> String {
    if is_disk_full(e) {
        "the disk is full".to_string()
    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
        "permission denied (the data folder may be read-only or locked by another program)".to_string()
    } else {
        e.to_string()
    }
}

fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut t = p.as_os_str().to_owned();
    t.push(suffix);
    PathBuf::from(t)
}

/// Writes `data` to a temporary file next to `path`, flushes it to disk, re-reads it to
/// confirm it is complete, and only then swaps it into place. The old file is never
/// touched unless the new one is fully written and verified.
fn atomic_write_with(path: &Path, data: &[u8], writer: &dyn Fn(&mut std::fs::File, &[u8]) -> std::io::Result<()>) -> std::io::Result<()> {
    let tmp = with_suffix(path, ".tmp");
    let written = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        writer(&mut f, data)?;
        f.flush()?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    match std::fs::read(&tmp) {
        Ok(back) if back.len() == data.len() && fnv64(&back) == fnv64(data) => {}
        Ok(_) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "the file on disk did not match what was written"));
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if cfg!(unix) {
        if let Some(dir) = path.parent() {
            if let Ok(d) = std::fs::File::open(dir) {
                let _ = d.sync_all();
            }
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    atomic_write_with(path, data, &|f: &mut std::fs::File, d: &[u8]| f.write_all(d))
}

fn sidecar(p: &Path) -> PathBuf {
    with_suffix(p, ".fnv")
}

fn read_db(path: &Path) -> Result<Db, String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    if b.is_empty() {
        return Err("the file is empty".to_string());
    }
    let mut db = serde_json::from_slice::<Db>(&b).map_err(|e| format!("not valid tree data: {}", e))?;
    migrate(&mut db);
    Ok(db)
}

/// Saves the tree. Keeps the previous good version as tree.json.prev.
fn save_db_to(dir: &Path, db: &Db) -> Result<usize, String> {
    let json = serde_json::to_vec(db).map_err(|e| e.to_string())?;
    serde_json::from_slice::<Db>(&json).map_err(|e| format!("internal check failed: {}", e))?;
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join("tree.json");
    if let Ok(old) = std::fs::read(&path) {
        if serde_json::from_slice::<Db>(&old).is_ok() {
            let _ = atomic_write(&dir.join("tree.json.prev"), &old);
        }
    }
    atomic_write(&path, &json).map_err(|e| io_msg(&e))?;
    Ok(json.len())
}

fn list_backups(bdir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(bdir)
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("tree-") && n.ends_with(".json")).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Re-reads a backup, compares its checksum (when one was stored) and parses it.
fn verify_backup(p: &Path) -> Result<usize, String> {
    let b = std::fs::read(p).map_err(|e| e.to_string())?;
    if let Ok(want) = std::fs::read_to_string(sidecar(p)) {
        let got = format!("{:016x}", fnv64(&b));
        if want.trim() != got {
            return Err("checksum does not match (the file changed or is damaged)".to_string());
        }
    }
    if b.is_empty() {
        return Err("the file is empty".to_string());
    }
    serde_json::from_slice::<Db>(&b).map(|d| d.people.len()).map_err(|e| format!("not valid tree data: {}", e))
}

fn write_backup_in(bdir: &Path, name: &str, json: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(bdir).map_err(|e| io_msg(&e))?;
    let p = bdir.join(name);
    atomic_write(&p, json).map_err(|e| io_msg(&e))?;
    atomic_write(&sidecar(&p), format!("{:016x}\n", fnv64(json)).as_bytes()).map_err(|e| io_msg(&e))?;
    verify_backup(&p).map(|_| ())
}

static BACKUP_HOURLY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// How many backups to keep (Tools > Backup settings).
static BACKUP_KEEP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(14);

fn prune_backups(bdir: &Path) {
    let mut names = list_backups(bdir);
    let keep = BACKUP_KEEP.load(Ordering::Relaxed).clamp(3, 365);
    while names.len() > keep {
        let old = names.remove(0);
        let p = bdir.join(old);
        let _ = std::fs::remove_file(sidecar(&p));
        let _ = std::fs::remove_file(&p);
    }
}

/// One verified backup per day (or on demand with force). Returns Ok(true) if a new one was written.
fn dated_backup_in(bdir: &Path, db: &Db, force: bool) -> Result<bool, String> {
    if db.people.is_empty() {
        return Ok(false);
    }
    let name = if BACKUP_HOURLY.load(Ordering::Relaxed) {
        let h = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| (d.as_secs() / 3600) % 24).unwrap_or(0);
        format!("tree-{}-{:02}h.json", today(), h)
    } else {
        format!("tree-{}.json", today())
    };
    let p = bdir.join(&name);
    if !force && p.exists() && verify_backup(&p).is_ok() {
        prune_backups(bdir);
        return Ok(false);
    }
    let json = serde_json::to_vec(db).map_err(|e| e.to_string())?;
    write_backup_in(bdir, &name, &json)?;
    prune_backups(bdir);
    Ok(true)
}

fn dated_backup(db: &Db) -> Result<bool, String> {
    dated_backup_in(&data_dir().join("backups"), db, false)
}

fn verify_all_backups() -> Vec<(String, Result<usize, String>)> {
    let bdir = data_dir().join("backups");
    list_backups(&bdir).into_iter().rev().map(|n| {
        let r = verify_backup(&bdir.join(&n));
        (n, r)
    }).collect()
}

/// Loads the tree. If the main file is damaged it is set aside (never overwritten) and the
/// newest readable copy is used instead: interrupted save, previous version, dated backups.
fn load_from(dir: &Path) -> (Db, String) {
    let main = dir.join("tree.json");
    let tmp = with_suffix(&main, ".tmp");
    let prev = dir.join("tree.json.prev");
    let bdir = dir.join("backups");
    let mut why = String::new();
    let mut kept = String::new();
    if main.exists() {
        match read_db(&main) {
            Ok(db) => {
                let _ = std::fs::remove_file(&tmp);
                return (db, String::new());
            }
            Err(e) => {
                why = e;
                let dest = dir.join(format!("tree.json.damaged-{}", stamp()));
                if std::fs::rename(&main, &dest).is_ok() {
                    kept = dest.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                }
            }
        }
    }
    let mut cands: Vec<PathBuf> = vec![tmp.clone(), prev];
    for n in list_backups(&bdir).into_iter().rev() {
        cands.push(bdir.join(n));
    }
    cands.push(dir.join("tree-backup.json"));
    for c in cands {
        if !c.exists() || verify_backup(&c).is_err() {
            continue;
        }
        if let Ok(db) = read_db(&c) {
            if db.people.is_empty() {
                continue;
            }
            let name = c.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let _ = std::fs::remove_file(&tmp);
            let msg = if why.is_empty() {
                format!("Your main data file was missing. Your tree was restored from {}.", name)
            } else {
                format!("Your data file was damaged ({}). Your tree was recovered from {}. The damaged file was kept as {}. Please check your most recent changes.", why, name, kept)
            };
            return (db, msg);
        }
    }
    if why.is_empty() {
        (Db::default(), String::new())
    } else {
        (Db::default(), format!("Your data file was damaged ({}) and no backup could be read. The damaged file was kept as {} in your data folder. Please do not delete it.", why, kept))
    }
}

/// Copies photos and recordings between the live media folder and the backup copy.
/// Never deletes. With only_missing it will not overwrite files that already exist.
/// With dry it only counts what would be copied. Returns (copied, failed, first error).
fn mirror_dir(src: &Path, dst: &Path, only_missing: bool, dry: bool) -> (usize, usize, Option<String>) {
    fn walk(src: &Path, dst: &Path, only_missing: bool, dry: bool, copied: &mut usize, failed: &mut usize, first: &mut Option<String>) {
        let Ok(rd) = std::fs::read_dir(src) else { return };
        if !dry {
            let _ = std::fs::create_dir_all(dst);
        }
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            let t = dst.join(e.file_name());
            if p.is_dir() {
                if e.file_name() == "previews" {
                    continue;
                }
                walk(&p, &t, only_missing, dry, copied, failed, first);
            } else {
                let need = match (std::fs::metadata(&p), std::fs::metadata(&t)) {
                    (Ok(_), Err(_)) => true,
                    (Ok(a), Ok(b)) => !only_missing && a.len() != b.len(),
                    _ => false,
                };
                if !need {
                    continue;
                }
                if dry {
                    *copied += 1;
                    continue;
                }
                let part = with_suffix(&t, ".part");
                match std::fs::copy(&p, &part).and_then(|_| std::fs::rename(&part, &t)) {
                    Ok(_) => *copied += 1,
                    Err(err) => {
                        *failed += 1;
                        if first.is_none() {
                            *first = Some(io_msg(&err));
                        }
                        let _ = std::fs::remove_file(&part);
                    }
                }
            }
        }
    }
    let (mut copied, mut failed, mut first): (usize, usize, Option<String>) = (0, 0, None);
    walk(src, dst, only_missing, dry, &mut copied, &mut failed, &mut first);
    (copied, failed, first)
}

fn ago(d: std::time::Duration) -> String {
    let s = d.as_secs();
    if s < 5 {
        "just now".to_string()
    } else if s < 60 {
        format!("{} s ago", s)
    } else if s < 3600 {
        format!("{} min ago", s / 60)
    } else {
        format!("{} h ago", s / 3600)
    }
}

/// A picture address for the program's own image loader. Unlike `file_uri` (used in web pages),
/// spaces are left as they are: the loader reads the path literally, so "%20" would break
/// folders like "Gilded Roots".
fn img_uri(p: &std::path::Path) -> String {
    format!("file://{}", p.display())
}

fn file_uri(p: &std::path::Path) -> String {
    let s = p.display().to_string().replace('\\', "/").replace(' ', "%20");
    if s.starts_with('/') { format!("file://{}", s) } else { format!("file:///{}", s) }
}

fn parse_md(d: &str) -> Option<(u32, u32)> {
    const M: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    let toks: Vec<&str> = d.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
    if toks.len() >= 3 && toks[0].len() == 4 && toks[0].chars().all(|c| c.is_ascii_digit()) {
        let m: u32 = toks[1].parse().ok()?;
        let dd: u32 = toks[2].parse().ok()?;
        if (1..=12).contains(&m) && (1..=31).contains(&dd) {
            return Some((m, dd));
        }
    }
    let mut month: Option<u32> = None;
    let mut day: Option<u32> = None;
    for t in &toks {
        let l = t.to_lowercase();
        if month.is_none() && l.len() >= 3 {
            if let Some(i) = M.iter().position(|m| l.starts_with(m)) {
                month = Some(i as u32 + 1);
                continue;
            }
        }
        if day.is_none() && t.len() <= 2 {
            if let Ok(n) = t.parse::<u32>() {
                if (1..=31).contains(&n) {
                    day = Some(n);
                }
            }
        }
    }
    Some((month?, day?))
}

fn ancestor_slots(db: &Db, root: u32, gens: usize) -> Vec<Vec<Option<u32>>> {
    let mut slots: Vec<Vec<Option<u32>>> = vec![vec![Some(root)]];
    for g in 1..gens {
        let cur = {
            let prev = &slots[g - 1];
            let mut c = Vec::new();
            for s in prev {
                let (f, m) = match s {
                    Some(i) => db.parents(*i),
                    None => (None, None),
                };
                c.push(f);
                c.push(m);
            }
            c
        };
        slots.push(cur);
    }
    slots
}

fn desc_layout(db: &Db, ix: &Idx, id: u32, depth: usize, max: usize, parent: Option<usize>, nodes: &mut Vec<(u32, usize, f32, Option<usize>)>, leaf: &mut f32, seen: &mut HashSet<u32>) -> f32 {
    let idx = nodes.len();
    nodes.push((id, depth, 0.0, parent));
    let mut kids: Vec<u32> = Vec::new();
    if depth + 1 < max {
        for f in db.fams_of_ix(ix, id) {
            for k in &db.fams[f].kids {
                if db.people.contains_key(k) && !seen.contains(k) && !kids.contains(k) {
                    kids.push(*k);
                }
            }
        }
    }
    let y = if kids.is_empty() {
        let y = *leaf;
        *leaf += 1.0;
        y
    } else {
        seen.extend(kids.iter().cloned());
        let mut first = 0.0f32;
        let mut last = 0.0f32;
        for (n, k) in kids.iter().enumerate() {
            let ky = desc_layout(db, ix, *k, depth + 1, max, Some(idx), nodes, leaf, seen);
            if n == 0 {
                first = ky;
            }
            last = ky;
        }
        (first + last) / 2.0
    };
    nodes[idx].2 = y;
    y
}

fn desc_ul(db: &Db, id: u32, depth: usize, max: usize, seen: &mut Vec<u32>, out: &mut String) {
    let p = &db.people[&id];
    let mut spouse = String::new();
    for f in db.fams_of(id) {
        let fm = &db.fams[&f];
        let sp = if fm.h == Some(id) { fm.w } else { fm.h };
        if let Some(sid) = sp {
            spouse.push_str(&format!("<br><small>m. {} {}</small>", dp(&fm.md, &fm.mp), esc(&db.people[&sid].name)));
        }
    }
    out.push_str(&format!("<li><b>{}</b> {}{}", esc(&display_name(p)), esc(&life(p)), spouse));
    let mut kids: Vec<u32> = Vec::new();
    if depth + 1 < max {
        for f in db.fams_of(id) {
            for k in &db.fams[&f].kids {
                if !seen.contains(k) && !kids.contains(k) {
                    kids.push(*k);
                }
            }
        }
    }
    if !kids.is_empty() {
        seen.extend(kids.iter().cloned());
        out.push_str("<ul>");
        for k in kids {
            desc_ul(db, k, depth + 1, max, seen, out);
        }
        out.push_str("</ul>");
    }
    out.push_str("</li>");
}

/// The Select / Hand choice on the chart toolbars. Select: click people as usual. Hand: drag to
/// move the chart around (the chart can also be moved by its scroll bars or the mouse wheel).
fn hand_switch(ui: &mut egui::Ui, hand: bool, act: &mut Option<Action>) {
    if ui.selectable_label(!hand, "Select").on_hover_text("Click a person to go to them (H switches to the hand)").clicked() && hand {
        *act = Some(Action::SetHand(false));
    }
    if ui.selectable_label(hand, "Hand").on_hover_text("Drag the chart to move around it (H switches back to Select)").clicked() && !hand {
        *act = Some(Action::SetHand(true));
    }
    // H flips between the two, unless someone is typing
    if !ui.ctx().wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::H) && i.modifiers.is_none()) {
        *act = Some(Action::SetHand(!hand));
    }
}

/// The hand tool itself: everything under `rect` becomes a surface to grab and drag.
fn hand_overlay(ui: &mut egui::Ui, rect: Rect, id: &str) {
    let r = ui.interact(rect, egui::Id::new(("hand_tool", id)), Sense::click_and_drag());
    if r.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        ui.scroll_with_delta(r.drag_delta());
    } else if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
}

fn zoom_controls(ui: &mut egui::Ui, z: f32, act: &mut Option<Action>) {
    ui.label("Zoom:");
    if ui.add_enabled(z > 0.26, egui::Button::new("-")).clicked() {
        *act = Some(Action::SetZoom((z - if z > 0.65 { 0.1 } else { 0.05 }).max(0.25)));
    }
    if ui.button(format!("{}%", (z * 100.0).round() as i32)).clicked() {
        *act = Some(Action::SetZoom(1.0));
    }
    if ui.add_enabled(z < 1.79, egui::Button::new("+")).clicked() {
        *act = Some(Action::SetZoom((z + if z >= 0.6 { 0.1 } else { 0.05 }).min(1.8)));
    }
    if ui.button("Fit").on_hover_text("Fit the whole chart in the window").clicked() {
        *act = Some(Action::Fit);
    }
}

fn current_year() -> i32 {
    today().get(0..4).and_then(|y| y.parse().ok()).unwrap_or(2026)
}

fn parse_date(s: &str) -> (Option<i32>, Option<u32>, Option<u32>) {
    const M: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    let l = s.to_lowercase();
    let (mut y, mut m, mut d): (Option<i32>, Option<u32>, Option<u32>) = (None, None, None);
    for tok in l.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()) {
        if tok.len() == 4 && tok.chars().all(|c| c.is_ascii_digit()) {
            if y.is_none() {
                y = tok.parse().ok();
            }
        } else if tok.len() <= 2 && tok.chars().all(|c| c.is_ascii_digit()) {
            if d.is_none() {
                d = tok.parse().ok();
            }
        } else if tok.len() >= 3 && m.is_none() {
            if let Some(i) = M.iter().position(|mm| tok.starts_with(mm)) {
                m = Some(i as u32 + 1);
            }
        }
    }
    (y, m, d)
}

/// Two lines for a box in the descendants chart: the person, then (below) their partner(s).
fn desc_lines(db: &Db, id: u32) -> (String, Option<String>) {
    let Some(p) = db.people.get(&id) else { return (String::new(), None) };
    let l = life(p);
    let first = if l.is_empty() { display_name(p) } else { format!("{}  {}", display_name(p), l) };
    let mut partners: Vec<String> = Vec::new();
    for f in db.fams_of(id) {
        let fm = &db.fams[&f];
        let sp = if fm.h == Some(id) { fm.w } else { fm.h };
        if let Some(q) = sp.and_then(|sid| db.people.get(&sid)) {
            let ql = life(q);
            partners.push(if ql.is_empty() { format!("+ {}", styled_name(q)) } else { format!("+ {}  {}", styled_name(q), ql) });
        }
    }
    let second = if partners.is_empty() { None } else { Some(partners.join("   ")) };
    (first, second)
}

/// Edit a list of citations. Each one points at a source record (shared by everyone who cites it)
/// or holds typed text, and carries its own page, quality, transcription, web address and access date.
fn cite_list(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + Copy,
    cites: &mut Vec<Source>,
    about: Option<&[&str]>,
    new_about: &str,
    sources: &mut BTreeMap<u32, SourceRec>,
    next: &mut u32,
    open_src: &mut Option<u32>,
) {
    let mut titles: Vec<(u32, String)> = sources.iter().map(|(k, s)| (*k, source_title(s))).collect();
    titles.sort_by_key(|t| t.1.to_lowercase());
    let mut remove: Option<usize> = None;
    let mut new_for: Option<usize> = None;
    for (i, c) in cites.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if let Some(list) = about {
                egui::ComboBox::from_id_salt((salt, i, "about")).selected_text(if c.about.is_empty() { "General" } else { c.about.as_str() }).width(100.0).show_ui(ui, |ui| {
                    for f in list {
                        ui.selectable_value(&mut c.about, f.to_string(), *f);
                    }
                });
            }
            let shown = match c.sid.and_then(|s| titles.iter().find(|t| t.0 == s)) {
                Some(t) => t.1.clone(),
                None if c.text.is_empty() => "Choose a source...".to_string(),
                None => format!("{} (typed)", c.text),
            };
            egui::ComboBox::from_id_salt((salt, i, "src")).selected_text(shown).width(300.0).show_ui(ui, |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    if ui.selectable_label(false, "+ New source...").clicked() {
                        new_for = Some(i);
                    }
                    if ui.selectable_label(c.sid.is_none(), "Just type the source name").clicked() {
                        if let Some(t) = c.sid.and_then(|s| titles.iter().find(|t| t.0 == s)) {
                            c.text = t.1.clone();
                        }
                        c.sid = None;
                    }
                    ui.separator();
                    for (id, t) in &titles {
                        if ui.selectable_label(c.sid == Some(*id), t.as_str()).clicked() {
                            c.sid = Some(*id);
                            c.text = t.clone();
                        }
                    }
                });
            });
            if let Some(s) = c.sid.filter(|s| sources.contains_key(s)) {
                if ui.small_button("Open source").on_hover_text("See or edit the source itself; changes apply everywhere it is cited").clicked() {
                    *open_src = Some(s);
                }
            }
            if ui.small_button("Remove").clicked() {
                remove = Some(i);
            }
        });
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            if c.sid.is_none() {
                ui.add(egui::TextEdit::singleline(&mut c.text).hint_text("source name, e.g. 1940 US Census").desired_width(230.0));
            }
            ui.add(egui::TextEdit::singleline(&mut c.page).hint_text("page, line or entry").desired_width(200.0));
            egui::ComboBox::from_id_salt((salt, i, "quay")).selected_text(quality_label(&c.quality)).width(170.0).show_ui(ui, |ui| {
                for q in ["", "0", "1", "2", "3"] {
                    ui.selectable_value(&mut c.quality, q.to_string(), quality_label(q));
                }
            });
        });
        let has_more = !c.data.is_empty() || !c.url.is_empty() || !c.accessed.is_empty() || !c.note.is_empty();
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            egui::CollapsingHeader::new(if has_more { "Transcription, web address, notes (filled in)" } else { "Transcription, web address, notes" })
                .id_salt((salt, i, "more"))
                .show(ui, |ui| {
                    ui.label("Text from the source (copy it exactly as written)");
                    ui.add(egui::TextEdit::multiline(&mut c.data).desired_rows(3).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        ui.label("Date of the entry");
                        ui.add(egui::TextEdit::singleline(&mut c.ddate).desired_width(110.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Web address of this record");
                        ui.add(egui::TextEdit::singleline(&mut c.url).hint_text("https://...").desired_width(260.0));
                        if is_url(&c.url) {
                            ui.hyperlink_to("Open", c.url.clone());
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Looked at on");
                        ui.add(egui::TextEdit::singleline(&mut c.accessed).hint_text("e.g. 5 Jan 2024").desired_width(130.0));
                    });
                    ui.label("Your notes on this citation");
                    ui.add(egui::TextEdit::multiline(&mut c.note).desired_rows(2).desired_width(f32::INFINITY));
                });
        });
        ui.add_space(4.0);
    }
    if let Some(i) = remove {
        cites.remove(i);
    }
    if let Some(i) = new_for {
        *next += 1;
        let id = *next;
        sources.insert(id, SourceRec::default());
        if let Some(c) = cites.get_mut(i) {
            c.sid = Some(id);
            c.text = String::new();
        }
        *open_src = Some(id);
    }
    if ui.button("Add source").clicked() {
        cites.push(Source { about: new_about.to_string(), ..Default::default() });
    }
}

/// A collapsible "Sources (n)" block for one fact or event.
fn cites_section(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + Copy,
    cites: &mut Vec<Source>,
    sources: &mut BTreeMap<u32, SourceRec>,
    next: &mut u32,
    open_src: &mut Option<u32>,
) {
    egui::CollapsingHeader::new(format!("Sources ({})", cites.len())).id_salt((salt, "sec")).show(ui, |ui| {
        cite_list(ui, salt, cites, None, "", sources, next, open_src);
    });
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn dp(d: &str, p: &str) -> String {
    let d = print_date(d);
    let v: Vec<&str> = [d.as_str(), p.trim()].into_iter().filter(|s| !s.is_empty()).collect();
    esc(&v.join(", "))
}

#[cfg(windows)]
fn open_path(p: &std::path::Path) {
    // Explorer opens the file with its usual program. Unlike "cmd /C start", it does not treat
    // characters such as & or | in a file name as commands.
    let _ = std::process::Command::new("explorer").arg(p).spawn();
}

#[cfg(not(windows))]
fn open_path(p: &std::path::Path) {
    let cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(cmd).arg(p).spawn();
}

fn page(title: &str, body: &str, landscape: bool) -> String {
    page_with_css(title, body, landscape, "", true)
}

fn page_with_css(title: &str, body: &str, landscape: bool, extra: &str, show_title: bool) -> String {
    // One look for every printed page. Long text (web addresses in sources) wraps instead of
    // pushing the right edge off the paper; tables have fixed column widths that fit the page.
    let css = "*{box-sizing:border-box}\
        html,body{margin:0;padding:0}\
        body{font-family:Georgia,'Times New Roman',serif;color:#1a1a1a;font-size:12.5px;line-height:1.35;padding:14px 18px;max-width:100%;overflow-wrap:anywhere;word-break:break-word}\
        h1{font-size:21px;margin:0 0 2px;letter-spacing:.2px}\
        .sub{color:#555;font-size:12px;margin:0 0 10px}\
        .rule{border:0;border-top:2px solid #6b5a2e;margin:6px 0 12px}\
        h2{font-size:15px;margin:16px 0 6px;color:#3d3420;border-bottom:1px solid #c9bf9f;padding-bottom:2px}\
        h3{font-size:13px;margin:12px 0 4px;color:#3d3420}\
        table{border-collapse:collapse;width:100%;table-layout:fixed;margin:0 0 10px}\
        th,td{border:1px solid #b9b2a0;padding:3px 6px;text-align:left;vertical-align:top;font-size:12px;overflow-wrap:anywhere}\
        th{background:#efe9d8;font-weight:bold;-webkit-print-color-adjust:exact;print-color-adjust:exact}\
        td.l{color:#555;width:16%}\
        tr.who th{background:#e2d8bb;font-size:13px}\
        td.empty{color:#aaa}\
        svg{width:100%;height:auto;max-height:82vh;display:block;margin:0 auto}\
        ol.src,ul{margin:2px 0 8px 0;padding-left:20px}\
        ol.src li,li{margin:2px 0;font-size:11.5px}\
        .note{white-space:pre-wrap;font-size:12px;margin:2px 0 8px}\
        .sec{break-inside:avoid;page-break-inside:avoid}\
        .foot{margin-top:14px;color:#777;font-size:10.5px;border-top:1px solid #ddd;padding-top:4px}\
        .noprint{margin-bottom:10px;font-family:system-ui,sans-serif}\
        .noprint button{font-size:14px;padding:6px 14px}\
        @media print{.noprint{display:none}body{padding:0}svg{max-height:150mm}.foot{display:none}}\
        @media print{body.port{width:7.25in}body.land{width:9.75in}}\
        @media print and (orientation:portrait){body.land{width:7.25in}}\
        .safari{display:none;background:#fff6d6;border:1px solid #e0c97a;padding:6px 10px;margin:8px 0;font-size:13px}";
    let heading = if show_title { format!("<h1>{}</h1><hr class=\"rule\">", esc(title)) } else { String::new() };
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{t}</title><style>{css}{extra}@page{{size:letter {o};margin:12mm 12mm 14mm 12mm;@bottom-left{{content:'{t2}';font:9px Georgia,serif;color:#888}}@bottom-center{{content:'Prepared {d} with Gilded Roots';font:9px Georgia,serif;color:#888}}@bottom-right{{content:'Page ' counter(page) ' of ' counter(pages);font:9px Georgia,serif;color:#888}}}}</style></head><body class=\"{bc}\"><div class=\"noprint\"><button onclick=\"window.print()\">Print or save as PDF</button><div class=\"safari\" id=\"grSafari\">Printing from Safari? If the right side is cut off, lower <b>Scale</b> in the print window (try 85%){lt}. Chrome and Firefox fit the page by themselves.</div></div>{h}{body}<div class=\"foot\">Prepared {d} with Gilded Roots</div><script>{fit}window.onload=function(){{setTimeout(function(){{window.print()}},500)}}</script></body></html>",
        t = esc(title),
        t2 = esc(title).replace('\'', "\\'"),
        css = css,
        extra = extra,
        o = if landscape { "landscape" } else { "portrait" },
        // No shrink-to-fit script: every page is laid out to the paper's width with CSS (tables and
        // charts are 100% wide), and a script measuring the page while printing misjudged it and
        // shrank pages that already fitted.
        // Safari prints at 100% without shrinking to the paper; the page widths in @media print above
        // keep it on the paper, and Safari users get a tip in case their printer's margins are wider.
        fit = "if(/^((?!chrome|chromium|crios|fxios|edg|android).)*safari/i.test(navigator.userAgent)){var s=document.getElementById('grSafari');if(s)s.style.display='block';}",
        bc = if landscape { "land" } else { "port" },
        lt = if landscape { ", and choose Landscape" } else { "" },
        h = heading,
        body = body,
        d = {
            let t = today();
            let (y, m, dd) = (&t[..4], t[5..7].parse::<usize>().unwrap_or(1), t[8..10].trim_start_matches('0').to_string());
            const MN: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
            format!("{} {} {}", dd, MN[m.clamp(1, 12) - 1], y)
        }
    )
}

/// A date on paper: the chosen date style, or "12 Mar 1921" when dates are shown as typed,
/// so a printed sheet is tidy even when dates were typed in different ways.
fn print_date(s: &str) -> String {
    let st = DATE_STYLE.load(Ordering::Relaxed);
    show_date_as(s, if st == 0 { 1 } else { st })
}

fn open_folder(p: &std::path::Path) {
    let dir = p.parent().unwrap_or(p);
    let cmd = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(cmd).arg(dir).spawn();
}

fn backup_path() -> PathBuf {
    store_path().with_file_name("tree-backup.json")
}

impl Db {
    pub fn merge_people(&mut self, keep: u32, gone: u32) {
        let Some(d) = self.people.remove(&gone) else { return };
        if let Some(k) = self.people.get_mut(&keep) {
            let fill = |a: &mut String, b: &String| {
                if a.is_empty() {
                    *a = b.clone();
                }
            };
            fill(&mut k.given, &d.given);
            fill(&mut k.surname, &d.surname);
            fill(&mut k.maiden, &d.maiden);
            fill(&mut k.sex, &d.sex);
            fill(&mut k.bd, &d.bd);
            fill(&mut k.bp, &d.bp);
            fill(&mut k.chd, &d.chd);
            fill(&mut k.chp, &d.chp);
            fill(&mut k.dd, &d.dd);
            fill(&mut k.dp, &d.dp);
            fill(&mut k.bud, &d.bud);
            fill(&mut k.bup, &d.bup);
            fill(&mut k.portrait, &d.portrait);
            if !d.notes.is_empty() && !k.notes.contains(&d.notes) {
                if !k.notes.is_empty() {
                    k.notes.push('\n');
                }
                k.notes.push_str(&d.notes);
            }
            for sr in d.sources {
                if !k.sources.iter().any(|x| x.text == sr.text) {
                    k.sources.push(sr);
                }
            }
            for e in d.events {
                if !k.events.iter().any(|x| x.kind == e.kind && x.date == e.date && x.place == e.place) {
                    k.events.push(e);
                }
            }
            k.media.extend(d.media);
            fill(&mut k.prefix, &d.prefix);
            fill(&mut k.suffix, &d.suffix);
            fill(&mut k.nick, &d.nick);
            fill(&mut k.chtag, &d.chtag);
            for a in d.assoc {
                if !k.assoc.iter().any(|x| x.other == a.other && x.role == a.role) {
                    k.assoc.push(a);
                }
            }
            if k.pref_parents.is_none() {
                k.pref_parents = d.pref_parents;
            }
            for a in d.alt_names {
                if !k.alt_names.iter().any(|x| x.kind == a.kind && x.given == a.given && x.surname == a.surname) {
                    k.alt_names.push(a);
                }
            }
            for n in d.note_refs {
                if !k.note_refs.contains(&n) {
                    k.note_refs.push(n);
                }
            }
            for (key, e) in d.fx {
                k.fx.entry(key).or_insert(e);
            }
            for (key, v) in d.fvals {
                k.fvals.entry(key).or_insert(v);
            }
            for r in d.extra {
                if !k.extra.iter().any(|x| x.at == r.at && x.text == r.text) {
                    k.extra.push(r);
                }
            }
            k.name = compose_name(&k.given, &k.surname);
            if k.living || d.living {
                k.living = true;
            }
        }
        for f in self.fams.values_mut() {
            if f.h == Some(gone) {
                f.h = Some(keep);
            }
            if f.w == Some(gone) {
                f.w = Some(keep);
            }
            for kd in f.kids.iter_mut() {
                if *kd == gone {
                    *kd = keep;
                }
            }
            let mut seen: HashSet<u32> = HashSet::new();
            f.kids.retain(|x| seen.insert(*x));
            for (_, x) in f.more.iter_mut() {
                if *x == gone {
                    *x = keep;
                }
            }
            if let Some(v) = f.pedi.remove(&gone) {
                f.pedi.entry(keep).or_insert(v);
            }
            if let Some(v) = f.frel.remove(&gone) {
                f.frel.entry(keep).or_insert(v);
            }
            if let Some(v) = f.mrel.remove(&gone) {
                f.mrel.entry(keep).or_insert(v);
            }
        }
        replace_pointer(self, &format!("I{}", gone), Some(&format!("I{}", keep)));
        let ids: Vec<u32> = self.fams.keys().copied().collect();
        let mut seen: HashMap<(Option<u32>, Option<u32>), u32> = HashMap::new();
        for fid in ids {
            let key = {
                let f = &self.fams[&fid];
                (f.h, f.w)
            };
            if key.0.is_none() && key.1.is_none() {
                continue;
            }
            if let Some(&first) = seen.get(&key) {
                if let Some(f) = self.fams.remove(&fid) {
                    if let Some(t) = self.fams.get_mut(&first) {
                        absorb_family(t, f);
                    }
                    replace_pointer(self, &format!("F{}", fid), Some(&format!("F{}", first)));
                }
            } else {
                seen.insert(key, fid);
            }
        }
    }
}

impl App {
    fn new() -> Self {
        let (mut db, load_notice) = load_from(&data_dir());
        fix_names(&mut db);
        let notice = if load_notice.is_empty() { None } else { Some(load_notice) };
        let recovered = notice.is_some();
        let mut status = if db.sample {
            "Showing sample data. Use File > Import GEDCOM to load your own tree.".to_string()
        } else if db.people.is_empty() {
            "Welcome! Add your first person, or import a tree with File > Import tree.".to_string()
        } else {
            String::new()
        };
        let sel = db.people.keys().next().copied();
        let last_json = serde_json::to_string(&db).unwrap_or_default();
        if let Some(n) = &notice {
            status = n.clone();
        }
        if let Some(svc) = synced_service(&data_dir()) {
            if status.is_empty() {
                status = format!("Your tree folder looks like it is synced by {}. Avoid opening it on two computers at once. See Help > About for options.", svc);
            }
        }
        if let Err(e) = dated_backup(&db) {
            status = format!("Today's backup could not be made: {}", e);
        }
        if mac_translocated() {
            status = "macOS is running this copy from a hidden temporary folder, so a FamilyTreeData folder beside it (portable mode) cannot be used. See Help > About for the one-time fix.".to_string();
        }
        App { db, sel, view: View::Pedigree, search: String::new(), fx: 0, dlg: None, confirm: false, status, dirty: recovered, find_open: false, settings: load_settings(), show_look: false, show_loc: false, review: None, imerge: None, src_open: false, src_sel: None, src_tab: 0, src_filter: String::new(), repo_sel: None, src_pick: None, src_merge: None, src_confirm: false, hl: None, back: Vec::new(), fwd: Vec::new(), wipe: None, show_about: false, help_open: false, help_find: String::new(), move_data: false, imp_open: false, imp_src: 0, imp_replace: false, rec: None, rel_open: false, rel_a: None, rel_b: None, viewing: None, undo: Vec::new(), redo: Vec::new(), last_json, last_snap: std::time::Instant::now(), last_backup_day: today(), check_open: false, dup_open: false, merge_open: false, merge_id: String::new(), merge_other: None, dups: Vec::new(), dup_ignore: HashSet::new(), save_error: None, last_saved: None, next_save: std::time::Instant::now(), notice, media_job: None, startup_done: false, verify_open: false, verify_results: Vec::new(), media_gap: String::new(), close_warned: false, report: None, report_open: false, link: None, fit_req: false, editing: None, quick: None, gal_only: false, home_pick: None, gal_find: String::new(), assoc_q: String::new(), assoc_role: "Guardian".to_string(), x: Extra::default() }.started()
    }

    fn persist(&mut self) -> bool {
        // the sample family is for practice: it is never written over the real tree
        if self.x.practice.is_some() {
            return true;
        }
        match save_db_to(&data_dir(), &self.db) {
            Ok(_) => {
                self.last_saved = Some(std::time::Instant::now());
                self.save_error = None;
                true
            }
            Err(e) => {
                self.save_error = Some(e);
                self.next_save = std::time::Instant::now() + std::time::Duration::from_secs(3);
                false
            }
        }
    }

    fn spawn_media_job(&mut self, restore: bool) {
        if self.media_job.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel::<(bool, usize, usize, Option<String>)>();
        let live = media_dir();
        let copy = data_dir().join("backups").join("media");
        std::thread::spawn(move || {
            let r = if restore { mirror_dir(&copy, &live, true, false) } else { mirror_dir(&live, &copy, false, false) };
            let _ = tx.send((restore, r.0, r.1, r.2));
        });
        self.media_job = Some(rx);
    }

    fn poll_media_job(&mut self) {
        let mut fin: Option<Option<(bool, usize, usize, Option<String>)>> = None;
        if let Some(rx) = &self.media_job {
            match rx.try_recv() {
                Ok(r) => fin = Some(Some(r)),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => fin = Some(None),
                Err(_) => {}
            }
        }
        if let Some(res) = fin {
            self.media_job = None;
            if let Some((restore, copied, failed, err)) = res {
                if failed > 0 {
                    let msg = format!("{} photo or recording file(s) could not be {}: {}", failed, if restore { "restored" } else { "backed up" }, err.unwrap_or_default());
                    self.status = msg.clone();
                    self.notice = Some(msg);
                } else if restore {
                    self.status = format!("Restored {} missing photo or recording file(s) from the backup.", copied);
                } else if copied > 0 {
                    self.status = format!("Backed up {} new photo or recording file(s).", copied);
                }
            }
        }
    }

    fn run_verify(&mut self) {
        self.verify_results = verify_all_backups();
        let gap = mirror_dir(&media_dir(), &data_dir().join("backups").join("media"), false, true).0;
        self.media_gap = if gap == 0 {
            "Photos and recordings: every file is in the backup.".to_string()
        } else {
            format!("Photos and recordings: {} file(s) are not in the backup yet. Use Tools > Back up now.", gap)
        };
        self.verify_open = true;
    }

    fn read_ged(&mut self) -> Option<Db> {
        // No file-type filter: some programs and drives give GEDCOM files odd or missing extensions, and a filter greys them out.
        let path = rfd::FileDialog::new().set_title("Choose a GEDCOM file (.ged), a PAF file (.paf) or a saved tree (.json)").set_directory(start_dir()).pick_file()?;
        remember_dir(&path);
        match read_ged_file(&path) {
            Ok((db, rep)) => {
                self.report = Some(rep);
                self.report_open = true;
                Some(db)
            }
            Err(e) => {
                self.status = e;
                None
            }
        }
    }

    fn import(&mut self) {
        if let Some(db) = self.read_ged() {
            let n = db.people.len();
            self.sel = db.people.keys().next().copied();
            self.db = db;
            self.status = format!("Imported {} people.", n);
            self.dirty = true;
        }
    }

    fn import_merge(&mut self) {
        if let Some(inc) = self.read_ged() {
            let file = self.report.as_ref().map(|r| r.file.clone()).unwrap_or_else(|| "the file".to_string());
            let im = ImportMerge::new(&self.db, inc, file);
            if im.order.is_empty() {
                // Nobody in the file looks like anyone in the tree, so there is nothing to decide.
                let ImportMerge { inc, .. } = im;
                if !self.backup() {
                    self.report_open = false;
                    return;
                }
                let (added, _) = merge_into_pairs(&mut self.db, inc, &HashMap::new());
                if self.sel.is_none() {
                    self.sel = self.db.people.keys().next().copied();
                }
                self.status = format!("Added {} people. None of them looked like anyone already in your tree.", added);
                self.dirty = true;
            } else {
                self.imerge = Some(im);
                self.report_open = false;
            }
        }
    }

    fn export(&mut self, private: bool) {
        if let Some(path) = rfd::FileDialog::new().set_title("Save GEDCOM file").set_directory(start_dir()).set_file_name("gilded-roots.ged").save_file() {
            remember_dir(&path);
            self.status = match write_gedcom(&self.db, &path, private) {
                Ok(s) | Err(s) => s,
            };
        }
    }

    fn create(&mut self, d: Dlg) {
        let nid = self.db.alloc();
        let full = d.name.trim().to_string();
        let (gn, sn) = split_name(&full);
        self.db.people.insert(nid, Person { name: full, given: gn, surname: sn, sex: d.sex.clone(), bd: d.bd.trim().to_string(), bp: d.bp.trim().to_string(), dd: d.dd.trim().to_string(), dp: d.dp.trim().to_string(), bud: d.bud.trim().to_string(), bup: d.bup.trim().to_string(), living: d.living && d.dd.trim().is_empty(), notes: d.notes.trim().to_string(), ..Default::default() });
        let me = d.target.or(self.sel);
        match d.kind {
            Kind::Person => {
                self.sel = Some(nid);
                self.view = View::Person;
            }
            Kind::Father | Kind::Mother => {
                let Some(me) = me else { return };
                let father = d.kind == Kind::Father;
                match self.db.famc(me) {
                    Some(f) => {
                        let fm = self.db.fams.get_mut(&f).unwrap();
                        if father { fm.h = Some(nid) } else { fm.w = Some(nid) }
                    }
                    None => {
                        let fid = self.db.alloc();
                        let mut fm = Family { kids: vec![me], ..Default::default() };
                        if father { fm.h = Some(nid) } else { fm.w = Some(nid) }
                        self.db.fams.insert(fid, fm);
                    }
                }
            }
            Kind::ParentIn(f, father) => {
                let Some(me) = me else { return };
                self.db.add_parent(me, Some(f), father, nid);
            }
            Kind::NewParent(father) => {
                let Some(me) = me else { return };
                self.db.add_parent(me, None, father, nid);
            }
            Kind::Sibling => {
                let Some(me) = me else { return };
                match self.db.famc(me) {
                    Some(f) => self.db.fams.get_mut(&f).unwrap().kids.push(nid),
                    None => {
                        let fid = self.db.alloc();
                        self.db.fams.insert(fid, Family { kids: vec![me, nid], ..Default::default() });
                    }
                }
            }
            Kind::Spouse => {
                let Some(me) = me else { return };
                let me_f = self.db.people[&me].sex == "F";
                let fid = self.db.alloc();
                let (h, w) = if me_f { (nid, me) } else { (me, nid) };
                self.db.fams.insert(fid, Family { h: Some(h), w: Some(w), ..Default::default() });
            }
            Kind::Child(f) => {
                let Some(me) = me else { return };
                let fid = match f {
                    Some(f) => f,
                    None => {
                        let fid = self.db.alloc();
                        let me_f = self.db.people[&me].sex == "F";
                        let mut fm = Family::default();
                        if me_f { fm.w = Some(me) } else { fm.h = Some(me) }
                        self.db.fams.insert(fid, fm);
                        fid
                    }
                };
                self.db.fams.get_mut(&fid).unwrap().kids.push(nid);
            }
        }
    }

    fn is_pristine_sample(&self) -> bool {
        if !self.db.sample {
            return false;
        }
        let s = sample();
        s.people.len() == self.db.people.len()
            && s.fams.len() == self.db.fams.len()
            && s.people.values().all(|sp| {
                self.db.people.values().any(|p| p.name == sp.name && p.bd == sp.bd && p.bp == sp.bp && p.dd == sp.dd && p.notes.is_empty() && p.media.is_empty())
            })
    }

    fn backup(&mut self) -> bool {
        if self.db.people.is_empty() || self.x.practice.is_some() {
            return true;
        }
        let json = match serde_json::to_vec(&self.db) {
            Ok(j) => j,
            Err(e) => {
                self.status = format!("Could not make a safety backup, so nothing was changed: {}", e);
                return false;
            }
        };
        match atomic_write(&backup_path(), &json) {
            Ok(_) => true,
            Err(e) => {
                let m = format!("Could not save a safety backup ({}), so nothing was changed.", io_msg(&e));
                self.status = m.clone();
                self.notice = Some(m);
                false
            }
        }
    }

    fn restore(&mut self) {
        match read_db(&backup_path()) {
            Ok(mut old) => {
                fix_names(&mut old);
                if !self.backup() {
                    return;
                }
                self.sel = old.people.keys().next().copied();
                self.db = old;
                self.back.clear();
                self.fwd.clear();
                self.dirty = true;
                self.status = "Restored your previous tree. Your last tree was saved as the new backup.".to_string();
            }
            Err(_) => self.status = "No usable backup found yet.".to_string(),
        }
    }

    fn request_wipe(&mut self, w: Wipe) {
        if w == Wipe::Restore || self.db.people.is_empty() || self.is_pristine_sample() {
            self.run_wipe(w);
        } else {
            self.wipe = Some(w);
        }
    }

    fn run_wipe(&mut self, w: Wipe) {
        if w == Wipe::Restore {
            self.restore();
            return;
        }
        if !self.backup() {
            return;
        }
        match w {
            Wipe::Sample => {
                self.db = sample();
                self.sel = self.db.people.keys().next().copied();
            }
            Wipe::Empty => {
                self.db = Db::default();
                self.sel = None;
            }
            Wipe::Import => self.import(),
            Wipe::Restore => {}
        }
        self.back.clear();
        self.fwd.clear();
        self.dirty = true;
    }

    fn attach_files(&mut self, id: u32) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Add photos or documents")
            .add_filter("Photos, PDFs, documents and recordings", &["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "tif", "tiff", "pdf", "doc", "docx", "rtf", "txt", "odt", "htm", "html", "wav", "mp3", "m4a", "ogg", "flac", "aac", "mp4", "mov"])
            .add_filter("All files", &["*"])
            .pick_files()
        {
            let mut n = 0;
            for p in paths {
                let name = p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                if let Some((file, shown)) = import_one(&p, &name) {
                    if let Some(person) = self.db.people.get_mut(&id) {
                        person.media.push(Media { file, name: shown, ..Default::default() });
                        n += 1;
                    }
                }
            }
            self.status = format!("Added {} file(s).", n);
            self.dirty = true;
        }
    }

    fn ensure_previews(&mut self, id: u32) {
        let todo: Vec<(usize, String)> = match self.db.people.get(&id) {
            Some(p) => p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file) && m.thumb.is_empty()).map(|(i, m)| (i, m.file.clone())).collect(),
            None => return,
        };
        if todo.is_empty() {
            return;
        }
        for (i, f) in todo {
            let r = make_previews(&mfile(&f));
            if let Some(m) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(i)) {
                match r {
                    Some((t, mid)) => {
                        m.thumb = t;
                        m.mid = mid;
                    }
                    None => {
                        m.thumb = "-".to_string();
                    }
                }
            }
        }
        self.dirty = true;
    }

    /// Make previews for up to `max` photos (anywhere in the tree) that do not have one yet.
    /// Returns true if any were made (the screen should be drawn again).
    fn missing_previews(&mut self, max: usize) -> bool {
        let todo: Vec<(u32, usize, String)> = self
            .db
            .people
            .iter()
            .flat_map(|(id, p)| p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file) && m.thumb.is_empty()).map(move |(i, m)| (*id, i, m.file.clone())))
            .take(max)
            .collect();
        if todo.is_empty() {
            return false;
        }
        for (id, i, f) in todo {
            let r = make_previews(&mfile(&f));
            if let Some(m) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(i)) {
                match r {
                    Some((t, mid)) => {
                        m.thumb = t;
                        m.mid = mid;
                    }
                    None => m.thumb = "-".to_string(),
                }
            }
        }
        self.dirty = true;
        // ask for another frame either way, so the previews just made are drawn straight away
        true
    }

    fn viewer(&mut self, ctx: &egui::Context) {
        let Some((pid, idx)) = self.viewing else { return };
        let valid = self.db.people.get(&pid).map(|p| idx < p.media.len()).unwrap_or(false);
        if !valid {
            self.viewing = None;
            return;
        }
        let mut open = true;
        let mut nav: Option<usize> = None;
        let mut remove = false;
        let mut portrait: Option<String> = None;
        let (mut tag_cancel, mut tag_pick, mut face_remove, mut face_portrait, mut face_go): (bool, Option<u32>, Option<usize>, Option<usize>, Option<usize>) = (false, None, None, None, None);
        let mut crop_pick: Option<[f32; 4]> = None;
        let tag_hits: Vec<(u32, String)> = if self.x.tag_box.is_some() {
            let q = self.x.tag_q.trim().to_lowercase();
            let words: Vec<&str> = q.split_whitespace().collect();
            let mut v: Vec<(u32, String)> = self
                .db
                .people
                .iter()
                .filter(|(_, p)| !words.is_empty() && words.iter().all(|w| p.name.to_lowercase().contains(w)))
                .map(|(i, _)| (*i, self.db.label(*i)))
                .take(8)
                .collect();
            if words.is_empty() {
                // the photo's owner and their close family are the likely ones
                let mut near = vec![pid];
                let (f, mo) = self.db.parents(pid);
                near.extend(f);
                near.extend(mo);
                near.extend(self.db.partners(pid));
                near.extend(self.db.children_of(pid));
                near.extend(self.db.siblings(pid));
                v = near.into_iter().filter(|i| self.db.people.contains_key(i)).take(12).map(|i| (i, self.db.label(i))).collect();
            }
            v
        } else {
            Vec::new()
        };
        let label_cache: HashMap<u32, String> = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).map(|m| m.faces.iter().map(|f| (f.who, self.db.label(f.who))).collect()).unwrap_or_default();
        let lc = label_cache;
        egui::Window::new("Photo").open(&mut open).default_size([720.0, 640.0]).show(ctx, |ui| {
            let p = self.db.people.get_mut(&pid).unwrap();
            let imgs: Vec<usize> = p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file)).map(|(i, _)| i).collect();
            let pos = imgs.iter().position(|&i| i == idx).unwrap_or(0);
            ui.horizontal(|ui| {
                if ui.add_enabled(pos > 0, egui::Button::new("< Previous")).clicked() {
                    nav = Some(imgs[pos - 1]);
                }
                ui.label(format!("{} of {}", pos + 1, imgs.len()));
                if ui.add_enabled(pos + 1 < imgs.len(), egui::Button::new("Next >")).clicked() {
                    nav = Some(imgs[pos + 1]);
                }
            });
            let full = mfile(&p.media[idx].file);
            let m = &mut p.media[idx];
            let uri = if m.mid.is_empty() || m.mid == "-" {
                img_uri(&full)
            } else {
                img_uri(&mprev(&m.mid))
            };
            let file_name = m.file.clone();
            let sense = if self.x.tag_mode || self.x.crop_mode { Sense::click_and_drag() } else { Sense::hover() };
            let resp = ui.add(egui::Image::new(uri).max_size(vec2(ui.available_width(), 480.0)).sense(sense));
            let ir = resp.rect;
            let to_img = |p: egui::Pos2| pos2(((p.x - ir.min.x) / ir.width()).clamp(0.0, 1.0), ((p.y - ir.min.y) / ir.height()).clamp(0.0, 1.0));
            let to_scr = |r: [f32; 4]| Rect::from_min_size(pos2(ir.min.x + r[0] * ir.width(), ir.min.y + r[1] * ir.height()), vec2(r[2] * ir.width(), r[3] * ir.height()));
            // drawing a new box
            if self.x.tag_mode || self.x.crop_mode {
                if resp.drag_started() {
                    if let Some(p) = resp.interact_pointer_pos() {
                        self.x.tag_drag = Some((p, p));
                        self.x.tag_box = None;
                    }
                }
                if resp.dragged() {
                    if let (Some(p), Some((a, _))) = (resp.interact_pointer_pos(), self.x.tag_drag) {
                        self.x.tag_drag = Some((a, p));
                    }
                }
                if resp.drag_stopped() {
                    if let Some((a, b)) = self.x.tag_drag.take() {
                        let (a, b) = (to_img(a), to_img(b));
                        let r = [a.x.min(b.x), a.y.min(b.y), (a.x - b.x).abs(), (a.y - b.y).abs()];
                        if r[2] > 0.015 && r[3] > 0.015 {
                            self.x.tag_box = Some(r);
                            self.x.tag_q.clear();
                        }
                    }
                }
            }
            let accent = ui.visuals().selection.bg_fill;
            let names: Vec<(usize, Rect, String)> = m.faces.iter().enumerate().map(|(k, f)| (k, to_scr(f.r), lc.get(&f.who).cloned().unwrap_or_else(|| "(someone removed from the tree)".into()))).collect();
            let painter = ui.painter_at(ir);
            for (_, r, name) in &names {
                painter.rect_stroke(*r, 2.0_f32, Stroke::new(2.0_f32, Color32::WHITE));
                painter.rect_stroke(r.expand(1.0), 2.0_f32, Stroke::new(1.0_f32, Color32::BLACK));
                let g = painter.layout_no_wrap(name.clone(), FontId::proportional(13.0), Color32::WHITE);
                let lr = Rect::from_min_size(pos2(r.min.x, r.max.y + 2.0), g.size() + vec2(8.0, 4.0));
                painter.rect_filled(lr, 3.0_f32, Color32::from_black_alpha(170));
                painter.galley(lr.min + vec2(4.0, 2.0), g, Color32::WHITE);
            }
            if let Some((a, b)) = self.x.tag_drag {
                painter.rect_stroke(Rect::from_two_pos(a, b), 0.0_f32, Stroke::new(2.0_f32, accent));
            }
            if let Some(r) = self.x.tag_box {
                painter.rect_stroke(to_scr(r), 2.0_f32, Stroke::new(2.5_f32, accent));
            }
            ui.add(egui::TextEdit::singleline(&mut m.caption).hint_text("Caption").desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                if ui.button("Open full size").clicked() {
                    open_media(&m.file);
                }
                if ui.button("Use as profile photo").clicked() {
                    portrait = Some(file_name.clone());
                }
                if ui.selectable_label(self.x.crop_mode, "Crop for profile photo").on_hover_text("Drag a box around the part of the picture to show as the profile photo").clicked() {
                    self.x.crop_mode = !self.x.crop_mode;
                    self.x.tag_mode = false;
                    self.x.tag_box = None;
                    self.x.tag_drag = None;
                }
                let lbl = if self.x.tag_mode { "Stop tagging faces" } else { "Tag faces" };
                if ui.selectable_label(self.x.tag_mode, lbl).on_hover_text("Drag a box around someone's face, then choose who it is").clicked() {
                    self.x.tag_mode = !self.x.tag_mode;
                    self.x.crop_mode = false;
                    self.x.tag_box = None;
                    self.x.tag_drag = None;
                }
                if ui.button("Remove photo").clicked() {
                    remove = true;
                }
            });
            if self.x.tag_mode && self.x.tag_box.is_none() {
                ui.label(RichText::new("Drag a box around a face on the picture.").italics().weak());
            }
            if self.x.crop_mode {
                if self.x.tag_box.is_none() {
                    ui.label(RichText::new("Drag a box around the part to show as the profile photo.").italics().weak());
                } else {
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Use this part as the profile photo").strong()).clicked() {
                            crop_pick = self.x.tag_box;
                        }
                        if ui.button("Cancel").clicked() {
                            tag_cancel = true;
                        }
                    });
                }
            }
            if self.x.tag_mode && self.x.tag_box.is_some() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Who is this?").strong());
                        ui.add(egui::TextEdit::singleline(&mut self.x.tag_q).hint_text("Type a name").desired_width(220.0)).request_focus();
                        if ui.button("Cancel").clicked() {
                            tag_cancel = true;
                        }
                    });
                    for (who, label) in &tag_hits {
                        if ui.button(label.as_str()).clicked() {
                            tag_pick = Some(*who);
                        }
                    }
                });
            }
            if !names.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new("Tagged in this photo").strong());
                for (k, _, name) in &names {
                    ui.horizontal(|ui| {
                        ui.label(name.as_str());
                        if ui.small_button("Go to").clicked() {
                            face_go = Some(*k);
                        }
                        if ui.small_button("Use as their profile picture").clicked() {
                            face_portrait = Some(*k);
                        }
                        if ui.small_button("Remove tag").clicked() {
                            face_remove = Some(*k);
                        }
                    });
                }
            }
        });
        if tag_cancel {
            self.x.tag_box = None;
        }
        if let Some(r) = crop_pick {
            let file = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).map(|m| m.file.clone());
            if let (Some(f), Some(p)) = (file, self.db.people.get_mut(&pid)) {
                p.portrait = f;
                p.portrait_crop = Some(r);
                p.face_portrait = None;
            }
            self.x.tag_box = None;
            self.x.crop_mode = false;
            self.dirty = true;
            self.status = "Profile photo cropped.".to_string();
        }
        if let (Some(who), Some(r)) = (tag_pick, self.x.tag_box) {
            if let Some(m) = self.db.people.get_mut(&pid).and_then(|p| p.media.get_mut(idx)) {
                m.faces.push(Face { who, r });
            }
            self.x.tag_box = None;
            self.dirty = true;
            self.status = format!("Tagged {}. The photo now shows on their page too.", self.db.label(who));
        }
        if let Some(k) = face_remove {
            if let Some(m) = self.db.people.get_mut(&pid).and_then(|p| p.media.get_mut(idx)) {
                if k < m.faces.len() {
                    m.faces.remove(k);
                }
            }
            self.dirty = true;
        }
        if let Some(k) = face_portrait {
            let info = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).and_then(|m| m.faces.get(k).map(|f| (f.who, m.file.clone(), f.r)));
            if let Some((who, file, r)) = info {
                if let Some(q) = self.db.people.get_mut(&who) {
                    q.face_portrait = Some(FacePortrait { owner: pid, file, r });
                    q.portrait_crop = None;
                }
                self.dirty = true;
                self.status = format!("Profile picture set for {}.", self.db.label(who));
            }
        }
        if let Some(k) = face_go {
            if let Some(w) = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).and_then(|m| m.faces.get(k)).map(|f| f.who) {
                self.viewing = None;
                self.goto(w);
                self.view = View::Person;
                return;
            }
        }
        if !open {
            self.viewing = None;
        }
        if let Some(n) = nav {
            self.viewing = Some((pid, n));
        }
        if let Some(f) = portrait {
            if let Some(p) = self.db.people.get_mut(&pid) {
                p.portrait = f;
                p.face_portrait = None;
                p.portrait_crop = None;
            }
            self.dirty = true;
            self.status = "Profile photo set. It now shows in the pedigree boxes.".to_string();
        }
        if remove {
            if let Some(p) = self.db.people.get_mut(&pid) {
                if idx < p.media.len() {
                    p.media.remove(idx);
                }
            }
            self.viewing = None;
            self.dirty = true;
        }
    }

    fn start_recording(&mut self, person: u32) {
        let host = cpal::default_host();
        let mut cands: Vec<cpal::Device> = Vec::new();
        if !self.settings.mic.is_empty() {
            if let Ok(ds) = host.input_devices() {
                for d in ds {
                    if d.name().map(|n| n == self.settings.mic).unwrap_or(false) {
                        cands.push(d);
                    }
                }
            }
        }
        if let Some(d) = host.default_input_device() {
            cands.push(d);
        }
        if let Ok(ds) = host.input_devices() {
            for d in ds {
                cands.push(d);
            }
        }
        if cands.is_empty() {
            self.status = "No microphone found. Check System Settings > Sound > Input, and that Gilded Roots is allowed under Privacy & Security > Microphone.".to_string();
            return;
        }
        let buf = Arc::new(Mutex::new(Vec::<i16>::new()));
        let peak = Arc::new(AtomicU32::new(0));
        let mut last = String::new();
        for dev in cands {
            let name = dev.name().unwrap_or_else(|_| "a microphone".to_string());
            match try_device(&dev, &buf, &peak) {
                Ok((stream, channels, rate)) => {
                    self.rec = Some(Recorder { _stream: stream, buf, channels, rate, started: std::time::Instant::now(), person, peak });
                    self.status = format!("Recording from {}. Watch the level bar: if it stays empty when you talk, pick a different microphone.", name);
                    return;
                }
                Err(e) => last = format!("{}: {}", name, e),
            }
        }
        self.status = format!(
            "Could not start any microphone (last error - {}). Check System Settings > Sound > Input for a working microphone, close other apps that use it, and check Privacy & Security > Microphone.",
            last
        );
    }

    fn stop_recording(&mut self) {
        let Some(r) = self.rec.take() else { return };
        let Recorder { _stream, buf, channels, rate, person, .. } = r;
        drop(_stream);
        let samples = buf.lock().map(|v| v.clone()).unwrap_or_default();
        if samples.is_empty() {
            self.status = "Nothing was recorded. Check your microphone settings.".to_string();
            return;
        }
        let top = samples.iter().map(|x| x.unsigned_abs()).max().unwrap_or(0);
        if top < 8 {
            self.status = "The recording was completely silent, so it was not saved. On a Mac, open System Settings > Privacy & Security > Microphone and switch on Gilded Roots, then try again.".to_string();
            return;
        }
        let file = format!("{}-story.wav", stamp());
        let spec = hound::WavSpec { channels, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let path = media_dir().join(&file);
        let ok = (|| -> Result<(), hound::Error> {
            let mut w = hound::WavWriter::create(&path, spec)?;
            for x in &samples {
                w.write_sample(*x)?;
            }
            w.finalize()
        })();
        match ok {
            Ok(_) => {
                if let Some(p) = self.db.people.get_mut(&person) {
                    p.media.push(Media { file, name: "Recorded story.wav".to_string(), caption: "Recorded story".to_string(), ..Default::default() });
                }
                self.status = "Story saved. You'll find it under Photos, documents and stories.".to_string();
                self.dirty = true;
            }
            Err(e) => self.status = format!("Could not save the recording: {}", e),
        }
    }

    fn change_data_folder(&mut self) {
        let Some(parent) = rfd::FileDialog::new().set_title("Choose where to keep your tree (a folder named Gilded Roots is made inside it)").set_directory(std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).unwrap_or_else(start_dir)).pick_folder() else {
            return;
        };
        let target = parent.join("Gilded Roots");
        let src = data_dir();
        if target == src {
            return;
        }
        if target.join("tree.json").exists() {
            self.status = "That folder already holds a tree. Choose a different place.".to_string();
            return;
        }
        self.persist();
        match copy_dir(&src, &target) {
            Ok(_) if std::fs::read(src.join("tree.json")).ok() == std::fs::read(target.join("tree.json")).ok() => {
                if std::fs::write(config_dir().join("location.txt"), target.to_string_lossy().as_bytes()).is_ok() {
                    self.status = format!("Your tree is now kept in {}. The old folder was left untouched.", target.display());
                } else {
                    self.status = "Copied, but could not save the new location, so the old folder is still in use.".to_string();
                }
            }
            _ => {
                self.status = "Could not copy your data to that folder, so nothing was changed.".to_string();
            }
        }
    }

    fn make_portable(&mut self) {
        let Some(target) = portable_dir() else {
            self.status = "Could not find where the app is stored.".to_string();
            return;
        };
        if target.is_dir() {
            self.status = "This copy is already portable.".to_string();
            return;
        }
        self.persist();
        // every tree comes along (the main one and those under More trees)
        let src = base_dir();
        match copy_dir(&src, &target) {
            Ok(_) => {
                self.status = "Portable mode is on. Your data is now kept in the FamilyTreeData folder next to the app.".to_string();
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&target);
                self.status = format!(
                    "Could not create a folder next to the app ({}). Try moving the app to a folder you can write to, such as your Desktop or a USB stick.",
                    e
                );
            }
        }
    }

    fn open_print(&mut self, html: String) {
        // kept in the program's own folder: a shared temporary folder could be tampered with
        let dir = data_dir().join("print");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("gilded-roots-print.html");
        match std::fs::write(&path, html) {
            Ok(_) => {
                open_path(&path);
                self.status = "Opened a print page in your browser. Use its Print button, or choose Save as PDF.".to_string();
            }
            Err(e) => self.status = format!("Could not make the print page: {}", e),
        }
    }

    fn print_db(&self) -> Db {
        if self.settings.hide_living {
            self.db.sanitized()
        } else {
            self.db.clone()
        }
    }

    /// One spouse's block on a family group sheet: name, the four life events with date and
    /// place in their own columns, and their parents.
    fn person_rows(db: &Db, role: &str, id: Option<u32>) -> String {
        let cell = |s: &str| if s.trim().is_empty() { "<td class=\"empty\"></td>".to_string() } else { format!("<td>{}</td>", esc(s.trim())) };
        match id {
            Some(i) => {
                let p = &db.people[&i];
                let mut r = format!("<tr class=\"who\"><th>{}</th><th colspan=\"2\">{}</th></tr>", role, esc(&display_name(p)));
                for (l, d, pl) in [("Born", &p.bd, &p.bp), (if p.chtag == "BAPM" { "Baptised" } else { "Christened" }, &p.chd, &p.chp), ("Died", &p.dd, &p.dp), ("Buried", &p.bud, &p.bup)] {
                    r.push_str(&format!("<tr><td class=\"l\">{}</td>{}{}</tr>", l, cell(&print_date(d)), cell(pl)));
                }
                let (f, m) = db.parents(i);
                let nm = |x: Option<u32>| x.and_then(|x| db.people.get(&x)).map(display_name).unwrap_or_default();
                r.push_str(&format!("<tr><td class=\"l\">Father</td><td colspan=\"2\">{}</td></tr>", esc(&nm(f))));
                r.push_str(&format!("<tr><td class=\"l\">Mother</td><td colspan=\"2\">{}</td></tr>", esc(&nm(m))));
                r
            }
            None => format!("<tr class=\"who\"><th>{}</th><th colspan=\"2\" style=\"font-weight:normal;color:#777\">not recorded</th></tr>", role),
        }
    }

    fn family_html(&self, id: u32) -> String {
        let db = self.print_db();
        let name = db.people[&id].name.clone();
        page_with_css(&format!("Family Group Sheet: {}", name), &Self::family_body(&db, id), false, "", false)
    }

    fn family_body(db: &Db, id: u32) -> String {
        let mut ids = db.fams_of(id);
        if ids.is_empty() {
            if let Some(f) = db.famc(id) {
                ids.push(f);
            }
        }
        let name = db.people[&id].name.clone();
        if ids.is_empty() {
            return format!("<h1>Family Group Sheet</h1><hr class=\"rule\"><p>{} has no family group yet.</p>", esc(&name));
        }
        let mut body = String::new();
        let colgroup = "<colgroup><col style=\"width:16%\"><col style=\"width:30%\"><col style=\"width:54%\"></colgroup>";
        for (n, fid) in ids.iter().enumerate() {
            let f = &db.fams[fid];
            let couple: Vec<String> = [f.h, f.w].into_iter().flatten().filter_map(|x| db.people.get(&x)).map(display_name).collect();
            if n > 0 {
                body.push_str("<div style=\"break-before:page;page-break-before:always\"></div>");
            }
            body.push_str("<h1>Family Group Sheet</h1>");
            body.push_str(&format!("<p class=\"sub\">{}</p><hr class=\"rule\">", esc(&couple.join("  &  "))));
            body.push_str(&format!("<div class=\"sec\"><table>{}", colgroup));
            body.push_str(&Self::person_rows(db, "Husband", f.h));
            body.push_str("</table></div>");
            body.push_str(&format!("<div class=\"sec\"><table>{}", colgroup));
            body.push_str(&Self::person_rows(db, "Wife", f.w));
            body.push_str("</table></div>");
            let kind = if f.kind.trim().is_empty() { "Marriage".to_string() } else { f.kind.trim().to_string() };
            body.push_str(&format!(
                "<table>{}<tr class=\"who\"><th>{}</th><td>{}</td><td>{}</td></tr></table>",
                colgroup,
                esc(&kind),
                esc(&print_date(&f.md)),
                esc(f.mp.trim())
            ));
            body.push_str("<h2>Children</h2>");
            body.push_str("<table><colgroup><col style=\"width:4%\"><col style=\"width:5%\"><col style=\"width:23%\"><col style=\"width:24%\"><col style=\"width:22%\"><col style=\"width:22%\"></colgroup>");
            body.push_str("<tr><th>#</th><th>Sex</th><th>Name</th><th>Born</th><th>Died</th><th>Spouse</th></tr>");
            for (n, k) in f.kids.iter().enumerate() {
                let Some(c) = db.people.get(k) else { continue };
                let when_where = |d: &str, p: &str| {
                    let (d, p) = (print_date(d), p.trim().to_string());
                    match (d.is_empty(), p.is_empty()) {
                        (true, true) => String::new(),
                        (false, true) => esc(&d),
                        (true, false) => esc(&p),
                        (false, false) => format!("{}<br><span style=\"color:#555\">{}</span>", esc(&d), esc(&p)),
                    }
                };
                let spouses: Vec<String> = db.partners(*k).into_iter().filter_map(|s| db.people.get(&s)).map(display_name).collect();
                body.push_str(&format!(
                    "<tr class=\"sec\"><td>{}</td><td>{}</td><td><b>{}</b>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    n + 1,
                    match c.sex.as_str() {
                        "M" => "M",
                        "F" => "F",
                        _ => "",
                    },
                    esc(&display_name(c)),
                    esc(&db.child_note(*fid, *k)),
                    when_where(&c.bd, &c.bp),
                    when_where(&c.dd, &c.dp),
                    esc(&spouses.join("; "))
                ));
            }
            if f.kids.is_empty() {
                body.push_str("<tr><td colspan=\"6\" class=\"empty\">No children recorded</td></tr>");
            }
            body.push_str("</table>");
            for (who, role) in [(f.h, "Husband"), (f.w, "Wife")] {
                let Some(who) = who else { continue };
                let p = &db.people[&who];
                if !p.notes.trim().is_empty() {
                    body.push_str(&format!("<h3>Notes: {} ({})</h3><div class=\"note\">{}</div>", esc(&p.name), role.to_lowercase(), esc(p.notes.trim())));
                }
                // sources, numbered, without repeats
                let mut seen: Vec<String> = Vec::new();
                let mut items: Vec<String> = Vec::new();
                for sr in &p.sources {
                    let text = db.cite_text(sr);
                    if seen.contains(&text) {
                        continue;
                    }
                    seen.push(text.clone());
                    let about = if sr.about == "General" || sr.about.is_empty() { String::new() } else { format!("<i>{}:</i> ", esc(&sr.about)) };
                    items.push(format!("<li>{}{}</li>", about, esc(&text)));
                }
                if !items.is_empty() {
                    body.push_str(&format!("<h3>Sources: {} ({})</h3><ol class=\"src\">{}</ol>", esc(&p.name), role.to_lowercase(), items.join("")));
                }
            }
        }
        body
    }

    fn pedigree_html(&self, root: u32) -> String {
        let db = self.print_db();
        let svg = self.pedigree_svg(&db, root);
        page(&format!("Pedigree chart of {}", db.people[&root].name), &svg, true)
    }

    fn pedigree_svg(&self, db: &Db, root: u32) -> String {
        let gens = self.settings.gens.clamp(3, 8);
        let rows = 1usize << (gens - 1);
        // more detail when there is room: name, born and died lines for up to 5 generations
        let lines = if gens <= 5 { 3 } else if gens == 6 { 2 } else { 1 };
        let bw = if gens <= 5 { 250.0f32 } else { 200.0 };
        let gap = 40.0f32;
        let bh = 24.0 + 18.0 * (lines as f32 - 1.0);
        let pitch = bh + if gens <= 4 { 22.0 } else { 6.0 };
        let top = 10.0f32;
        let w = gens as f32 * bw + (gens as f32 - 1.0) * gap + 20.0;
        let h = rows as f32 * pitch + top + 10.0;
        let mut slots: Vec<Vec<Option<u32>>> = vec![vec![Some(root)]];
        for g in 1..gens {
            let cur = {
                let prev = &slots[g - 1];
                let mut c = Vec::new();
                for s in prev {
                    let (f, m) = match s {
                        Some(i) => db.parents(*i),
                        None => (None, None),
                    };
                    c.push(f);
                    c.push(m);
                }
                c
            };
            slots.push(cur);
        }
        let cut = |t: String, max: usize| if t.chars().count() > max { format!("{}\u{2026}", t.chars().take(max.saturating_sub(1)).collect::<String>()) } else { t };
        let fact = |label: &str, d: &str, p: &str| -> String {
            let d = print_date(d);
            let p = p.trim();
            match (d.is_empty(), p.is_empty()) {
                (true, true) => String::new(),
                (false, true) => format!("{} {}", label, d),
                (true, false) => format!("{} {}", label, p),
                (false, false) => format!("{} {}, {}", label, d, p),
            }
        };
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" font-family=\"Georgia, serif\">", w, h);
        for g in 0..gens {
            let n = 1usize << g;
            let step = rows as f32 * pitch / n as f32;
            for i in 0..n {
                let cy = top + (i as f32 + 0.5) * step;
                let x = 10.0 + g as f32 * (bw + gap);
                if g > 0 {
                    let ccy = top + ((i / 2) as f32 + 0.5) * step * 2.0;
                    svg.push_str(&format!("<polyline fill=\"none\" stroke=\"#8a7d5a\" stroke-width=\"0.9\" points=\"{},{} {},{} {},{} {},{}\"/>", x, cy, x - gap / 2.0, cy, x - gap / 2.0, ccy, x - gap, ccy));
                }
                let y0 = cy - bh / 2.0;
                match slots[g][i] {
                    Some(id) => {
                        let p = &db.people[&id];
                        let (fill, ink, sub) = if g == 0 { ("#2f3a4f", "#fff", "#d8dde8") } else { ("#fbf8ef", "#111", "#555") };
                        let maxc = (bw / 9.5) as usize;
                        let name = if lines == 1 {
                            let l = life(p);
                            if l.is_empty() { display_name(p) } else { format!("{}  {}", display_name(p), l) }
                        } else {
                            display_name(p)
                        };
                        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"#8a7d5a\" stroke-width=\"0.9\"/>", x, y0, bw, bh, fill));
                        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"17\" font-weight=\"bold\" fill=\"{}\">{}</text>", x + 6.0, y0 + 18.0, ink, esc(&cut(name, maxc))));
                        if lines >= 2 {
                            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"14\" fill=\"{}\">{}</text>", x + 6.0, y0 + 36.0, sub, esc(&cut(fact("b.", &p.bd, &p.bp), maxc + 8))));
                        }
                        if lines >= 3 {
                            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"14\" fill=\"{}\">{}</text>", x + 6.0, y0 + 54.0, sub, esc(&cut(fact("d.", &p.dd, &p.dp), maxc + 8))));
                        }
                    }
                    None => svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"#fff\" stroke=\"#cfc7b0\" stroke-dasharray=\"3,2\"/>", x, y0, bw, bh)),
                }
            }
        }
        svg.push_str("</svg>");
        svg
    }

    /// Describe what just changed, for the History list.
    fn change_label(&self) -> String {
        if let Some(l) = &self.x.next_label {
            return l.clone();
        }
        let added: Vec<u32> = self.db.people.keys().filter(|k| !self.x.prev_names.contains_key(k)).copied().collect();
        let gone: Vec<&String> = self.x.prev_names.iter().filter(|(k, _)| !self.db.people.contains_key(k)).map(|(_, v)| v).collect();
        if added.len() == 1 && gone.is_empty() {
            return format!("Added {}", self.db.label(added[0]));
        }
        if added.len() > 1 {
            return format!("Added {} people", added.len());
        }
        if gone.len() == 1 && added.is_empty() {
            return format!("Deleted {}", gone[0]);
        }
        if gone.len() > 1 {
            return format!("Removed {} people", gone.len());
        }
        if let Some((sid, old)) = &self.x.prev_sel {
            if let Some(p) = self.db.people.get(sid) {
                if serde_json::to_string(p).unwrap_or_default() != *old {
                    return format!("Changed {}", self.db.label(*sid));
                }
            }
        }
        if self.db.fams.len() != self.x.prev_fams {
            return "Changed family links".into();
        }
        "Changed the tree".into()
    }

    fn started(mut self) -> Self {
        self.remember_state();
        self
    }

    fn remember_state(&mut self) {
        self.x.prev_names = self.db.people.iter().map(|(k, p)| (*k, p.name.clone())).collect();
        self.x.prev_sel = self.sel.and_then(|s| self.db.people.get(&s).map(|p| (s, serde_json::to_string(p).unwrap_or_default())));
        self.x.prev_fams = self.db.fams.len();
        self.x.next_label = None;
    }

    fn track(&mut self) {
        let cur = serde_json::to_string(&self.db).unwrap_or_default();
        if cur != self.last_json {
            let label = self.change_label();
            if self.last_snap.elapsed() > std::time::Duration::from_millis(800) || self.undo.is_empty() {
                self.undo.push(Snap { z: pack(&self.last_json), label, at: std::time::Instant::now() });
                if self.undo.len() > 60 {
                    self.undo.remove(0);
                }
            } else if let Some(last) = self.undo.last_mut() {
                // quick follow-on edits are one step; keep the clearer description
                if last.label != label && label != "Changed the tree" && last.label == "Changed the tree" {
                    last.label = label;
                }
                last.at = std::time::Instant::now();
            }
            self.redo.clear();
            self.last_json = cur;
            self.last_snap = std::time::Instant::now();
            self.remember_state();
        } else if self.x.prev_sel.as_ref().map(|x| x.0) != self.sel {
            self.remember_state();
        }
    }

    fn load_snapshot(&mut self, json: String) {
        if let Ok(mut db) = serde_json::from_str::<Db>(&json) {
            fix_names(&mut db);
            self.db = db;
            self.last_json = serde_json::to_string(&self.db).unwrap_or(json);
            self.last_snap = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(5)).unwrap_or_else(std::time::Instant::now);
            if !self.sel.map(|s| self.db.people.contains_key(&s)).unwrap_or(false) {
                self.sel = self.db.people.keys().next().copied();
            }
            self.viewing = None;
            self.hl = None;
            self.dirty = true;
            self.remember_state();
        }
    }

    fn undo_action(&mut self) {
        self.track();
        if let Some(prev) = self.undo.pop() {
            self.redo.push(Snap { z: pack(&self.last_json), label: prev.label.clone(), at: prev.at });
            self.load_snapshot(unpack(&prev.z));
            self.status = format!("Undone: {}.", prev.label);
        }
    }

    fn redo_action(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(Snap { z: pack(&self.last_json), label: next.label.clone(), at: next.at });
            self.load_snapshot(unpack(&next.z));
            self.status = format!("Redone: {}.", next.label);
        }
    }

    /// Tools > Saved versions: the daily copies of the tree, what is different in each, and
    /// putting back one person or the whole tree.
    fn versions_window(&mut self, ctx: &egui::Context) {
        if !self.x.ver_open {
            return;
        }
        let bdir = data_dir().join("backups");
        let names: Vec<String> = list_backups(&bdir).into_iter().rev().collect();
        let mut open = true;
        let mut load: Option<String> = None;
        let mut restore_all: Option<String> = None;
        let mut bring_back: Option<u32> = None;
        egui::Window::new("Saved versions").open(&mut open).default_size([760.0, 540.0]).show(ctx, |ui| {
            ui.label(RichText::new("A copy of your tree is kept each day you use the program (the last 14 days). Pick a day to see what was different then.").small().weak());
            ui.separator();
            egui::SidePanel::left("ver_list").resizable(false).exact_width(200.0).show_inside(ui, |ui| {
                if names.is_empty() {
                    ui.label(RichText::new("No saved versions yet.").italics().weak());
                }
                for n in &names {
                    let day = n.trim_start_matches("tree-").trim_end_matches(".json").to_string();
                    let on = self.x.ver.as_ref().map(|v| v.0 == *n).unwrap_or(false);
                    if ui.selectable_label(on, RichText::new(day).size(15.0)).clicked() {
                        load = Some(n.clone());
                    }
                }
            });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let Some((name, old, diff)) = &self.x.ver else {
                    ui.label(RichText::new("Pick a day on the left.").italics().weak());
                    return;
                };
                ui.label(RichText::new(name.trim_start_matches("tree-").trim_end_matches(".json")).size(20.0).strong());
                ui.label(format!("{} people then, {} now.", old.people.len(), self.db.people.len()));
                ui.horizontal(|ui| {
                    if ui.button("Restore the whole tree to this day...").on_hover_text("Your tree as it is now is kept as a backup first").clicked() {
                        restore_all = Some(name.clone());
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let (gone, changed, added) = diff;
                    ui.label(RichText::new(format!("Removed since then ({})", gone.len())).strong());
                    for id in gone.iter().take(300) {
                        ui.horizontal(|ui| {
                            ui.label(old.label(*id));
                            if ui.small_button("Bring back").on_hover_text("Puts this person back as they were that day (their family links need adding again)").clicked() {
                                bring_back = Some(*id);
                            }
                        });
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Changed since then ({})", changed.len())).strong());
                    for id in changed.iter().take(300) {
                        ui.horizontal(|ui| {
                            ui.label(old.label(*id));
                            if ui.small_button("Put back as they were").on_hover_text("Replaces this person's facts, notes, sources and photos with that day's copy").clicked() {
                                bring_back = Some(*id);
                            }
                        });
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Added since then ({})", added.len())).strong());
                    for id in added.iter().take(300) {
                        if self.db.people.contains_key(id) {
                            ui.label(self.db.label(*id));
                        }
                    }
                });
            });
        });
        if let Some(n) = load {
            match read_db(&bdir.join(&n)) {
                Ok(old) => {
                    let mut gone = Vec::new();
                    let mut changed = Vec::new();
                    for (id, p) in &old.people {
                        match self.db.people.get(id) {
                            None => gone.push(*id),
                            Some(q) => {
                                if serde_json::to_string(p).ok() != serde_json::to_string(q).ok() {
                                    changed.push(*id);
                                }
                            }
                        }
                    }
                    let added: Vec<u32> = self.db.people.keys().filter(|k| !old.people.contains_key(k)).copied().collect();
                    self.x.ver = Some((n, old, (gone, changed, added)));
                }
                Err(e) => self.status = format!("That saved version could not be read: {}", e),
            }
        }
        if let Some(id) = bring_back {
            if let Some((_, old, diff)) = self.x.ver.as_mut() {
                if let Some(p) = old.people.get(&id).cloned() {
                    let name = p.name.clone();
                    self.db.people.insert(id, p);
                    if self.db.next <= id {
                        self.db.next = id + 1;
                    }
                    diff.0.retain(|x| *x != id);
                    diff.1.retain(|x| *x != id);
                    self.x.next_label = Some(format!("Brought back {} from a saved version", name));
                    self.dirty = true;
                    self.status = format!("{} is back as they were that day. Edit > Undo takes it away again.", name);
                }
            }
        }
        if let Some(n) = restore_all {
            self.restore_path(&bdir.join(&n));
            self.x.ver = None;
        }
        if !open {
            self.x.ver = None;
        }
        self.x.ver_open = open;
    }

    /// Tools > History of changes (F12): every recent change, newest first. Clicking one puts the
    /// tree back to just before it; the steps after it stay available to redo.
    fn history_window(&mut self, ctx: &egui::Context) {
        if !self.x.history_open {
            return;
        }
        let mut open = true;
        let mut go_undo = 0usize;
        let mut go_redo = 0usize;
        egui::Window::new("History of changes").open(&mut open).default_size([460.0, 520.0]).show(ctx, |ui| {
            ui.label(RichText::new("Newest first. Click a change to go back to just before it. Nothing is lost: you can redo forward again.").small().weak());
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if !self.redo.is_empty() {
                    ui.label(RichText::new("Undone (click to redo up to here)").small().weak());
                    for (k, s) in self.redo.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(false, RichText::new(s.label.as_str()).italics().weak()).clicked() {
                                go_redo = self.redo.len() - k;
                            }
                            ui.label(RichText::new(when_ago(s.at)).small().weak());
                        });
                    }
                    ui.separator();
                }
                ui.label(RichText::new("Now").strong());
                if self.undo.is_empty() {
                    ui.label(RichText::new("No changes yet in this session.").italics().weak());
                }
                let n = self.undo.len();
                for (k, s) in self.undo.iter().enumerate().rev() {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(false, RichText::new(s.label.as_str()).size(14.0)).on_hover_text("Go back to just before this change").clicked() {
                            go_undo = n - k;
                        }
                        ui.label(RichText::new(when_ago(s.at)).small().weak());
                    });
                }
            });
            ui.separator();
            ui.label(RichText::new("History covers this session (the last 60 changes). Older versions are in the daily backups: Tools > Restore previous tree.").small().weak());
        });
        for _ in 0..go_undo {
            self.undo_action();
        }
        for _ in 0..go_redo {
            self.redo_action();
        }
        self.x.history_open = open;
    }

    fn restore_from_file(&mut self) {
        let dir = data_dir().join("backups");
        let Some(path) = rfd::FileDialog::new().set_directory(dir).add_filter("Backup", &["json"]).pick_file() else { return };
        self.restore_path(&path);
    }

    fn restore_path(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if let Err(e) = verify_backup(&path) {
            self.status = format!("That backup is damaged and was not used: {}", e);
            self.notice = Some(self.status.clone());
            return;
        }
        match read_db(&path) {
            Ok(mut old) => {
                fix_names(&mut old);
                if !self.backup() {
                    return;
                }
                self.sel = old.people.keys().next().copied();
                self.db = old;
                self.dirty = true;
                self.status = "Restored from that backup. Your previous tree was saved as the last backup.".to_string();
                self.spawn_media_job(true);
            }
            Err(e) => self.status = format!("That file could not be read as a backup: {}", e),
        }
    }


    /// Where a person's profile picture comes from: the picture's address and the part of it
    /// to show (the whole picture, or a tagged face).
    fn portrait_src(&self, p: &Person) -> Option<(String, [f32; 4])> {
        if let Some(fp) = &p.face_portrait {
            if let Some(m) = self.db.people.get(&fp.owner).and_then(|o| o.media.iter().find(|m| m.file == fp.file)) {
                // a face is a small part of the picture, so use the larger preview
                for f in [&m.mid, &m.thumb] {
                    if !f.is_empty() && f != "-" {
                        let t = mprev(f);
                        if t.is_file() {
                            return Some((img_uri(&t), fp.r));
                        }
                    }
                }
                let full = mfile(&m.file);
                if full.is_file() {
                    return Some((img_uri(&full), fp.r));
                }
            }
        }
        if p.portrait.is_empty() {
            return None;
        }
        let m = p.media.iter().find(|m| m.file == p.portrait)?;
        let crop = p.portrait_crop.unwrap_or([0.0, 0.0, 1.0, 1.0]);
        // a cropped photo needs the larger preview to stay sharp
        let order = if p.portrait_crop.is_some() { [&m.mid, &m.thumb] } else { [&m.thumb, &m.mid] };
        for f in order {
            if !f.is_empty() && f != "-" {
                let t = mprev(f);
                if t.is_file() {
                    return Some((img_uri(&t), crop));
                }
            }
        }
        // no small copy yet (or it was lost): use the photo itself
        let full = mfile(&m.file);
        if full.is_file() && is_image(&m.file) {
            Some((img_uri(&full), crop))
        } else {
            None
        }
    }

    /// Photos in other people's albums where this person is tagged, shown cropped to their face.
    fn tagged_strip(&mut self, ui: &mut egui::Ui, id: u32) {
        let tags = self.tagged_in(id);
        if tags.is_empty() {
            return;
        }
        ui.add_space(6.0);
        ui.label(RichText::new(format!("Tagged in {} other photo(s)", tags.len())).strong());
        let mut open: Option<(u32, usize)> = None;
        ui.horizontal_wrapped(|ui| {
            for (owner, i, r) in &tags {
                let Some(m) = self.db.people.get(owner).and_then(|o| o.media.get(*i)) else { continue };
                let src = [&m.mid, &m.thumb].into_iter().find(|f| !f.is_empty() && *f != "-").map(|f| mprev(f)).unwrap_or_else(|| mfile(&m.file));
                let uri = img_uri(&src);
                let who = self.db.label(*owner);
                ui.vertical(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(vec2(90.0, 90.0), Sense::click());
                    put_photo(ui, rect, &uri, *r, 6.0);
                    if resp.on_hover_text(format!("From {}'s photos", who)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some((*owner, *i));
                    }
                });
            }
        });
        if let Some(v) = open {
            self.viewing = Some(v);
        }
    }

    /// Photos in other people's albums where this person is tagged: (owner, index, face box).
    fn tagged_in(&self, id: u32) -> Vec<(u32, usize, [f32; 4])> {
        let mut v = Vec::new();
        for (oid, o) in &self.db.people {
            for (i, m) in o.media.iter().enumerate() {
                for f in &m.faces {
                    if f.who == id && *oid != id {
                        v.push((*oid, i, f.r));
                    }
                }
            }
        }
        v
    }

    fn chart_colors(&self, ui: &egui::Ui) -> (Stroke, Stroke, Color32, Color32, Color32, Color32, Color32) {
        let dark = ui.visuals().dark_mode;
        let bg = self.settings.bg;
        let rgb = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
        let (line_c, edge_c, box_c, hover_c, ink_c, head_c) = if dark {
            (shift(bg, 60), shift(bg, 38), shift(bg, 12), shift(bg, 24), rgb(230, 232, 235), shift(bg, 120))
        } else {
            (rgb(150, 155, 162), rgb(184, 188, 194), Color32::WHITE, rgb(245, 247, 250), rgb(31, 35, 40), rgb(90, 96, 105))
        };
        let accent = Color32::from_rgb(self.settings.accent[0], self.settings.accent[1], self.settings.accent[2]);
        (Stroke::new(1.0_f32, line_c), Stroke::new(1.0_f32, edge_c), box_c, hover_c, ink_c, head_c, accent)
    }

    fn descendants(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let sel = self.sel.unwrap();
        let z = self.settings.zoom;
        let maxd = self.settings.gens.clamp(3, 8);
        self.nav_bar(ui, act);
        ui.horizontal(|ui| {
            ui.label("Generations:");
            if ui.add_enabled(maxd > 3, egui::Button::new("-")).clicked() {
                *act = Some(Action::SetGens(maxd - 1));
            }
            ui.label(maxd.to_string());
            if ui.add_enabled(maxd < 8, egui::Button::new("+")).clicked() {
                *act = Some(Action::SetGens(maxd + 1));
            }
            ui.separator();
            zoom_controls(ui, z, act);
            ui.separator();
            hand_switch(ui, self.settings.hand, act);
            ui.separator();
            if ui.button("Print").clicked() {
                *act = Some(Action::PrintDescendants);
            }
        });
        let mut nodes: Vec<(u32, usize, f32, Option<usize>)> = Vec::new();
        let mut leaf = 0.0f32;
        let mut seen: HashSet<u32> = HashSet::from([sel]);
        let ix = self.db.index();
        desc_layout(&self.db, &ix, sel, 0, maxd, None, &mut nodes, &mut leaf, &mut seen);
        let rows = leaf.max(1.0);
        if self.fit_req {
            let av = ui.available_size();
            let zw = (av.x - 24.0) / (maxd as f32 * (250.0 + 34.0));
            let zh = (av.y - 20.0) / (rows * 46.0 + 20.0);
            *act = Some(Action::SetZoom(zw.min(zh).clamp(0.25, 1.8)));
        }
        if nodes.len() == 1 {
            ui.label("No children recorded yet. Open the Individual tab and use Add child.");
        }
        let (bw, bh, pitch, gap, top) = (250.0f32 * z, 40.0f32 * z, 46.0f32 * z, 34.0f32 * z, 10.0f32 * z);
        let tw = maxd as f32 * (bw + gap);
        let th = rows * pitch + top + 10.0;
        let (line, edge, box_c, hover_c, ink_c, _head_c, accent) = self.chart_colors(ui);
        egui::ScrollArea::both().show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(vec2(tw + 12.0, th), Sense::hover());
            let o = resp.rect.min + vec2(6.0, 0.0);
            if resp.hovered() {
                let zd = ui.input(|i| i.zoom_delta());
                if (zd - 1.0).abs() > 0.001 {
                    *act = Some(Action::SetZoom((z * zd).clamp(0.25, 1.8)));
                }
            }
            let vis = ui.clip_rect().expand(40.0);
            for n in &nodes {
                if let Some(pi) = n.3 {
                    let pn = &nodes[pi];
                    let cy0 = o.y + top + (n.2 + 0.5) * pitch;
                    let py0 = o.y + top + (pn.2 + 0.5) * pitch;
                    if cy0.max(py0) < vis.min.y || cy0.min(py0) > vis.max.y {
                        continue;
                    }
                    let px = o.x + pn.1 as f32 * (bw + gap) + bw;
                    let py = o.y + top + (pn.2 + 0.5) * pitch;
                    let cx = o.x + n.1 as f32 * (bw + gap);
                    let cy = o.y + top + (n.2 + 0.5) * pitch;
                    painter.add(egui::Shape::line(vec![pos2(px, py), pos2(px + gap / 2.0, py), pos2(px + gap / 2.0, cy), pos2(cx, cy)], line));
                }
            }
            for (i, n) in nodes.iter().enumerate() {
                let x = o.x + n.1 as f32 * (bw + gap);
                let cy = o.y + top + (n.2 + 0.5) * pitch;
                let rect = Rect::from_min_size(pos2(x, cy - bh / 2.0), vec2(bw, bh));
                if !rect.intersects(vis) {
                    continue;
                }
                let r2 = ui.interact(rect, egui::Id::new(("desc", i)), Sense::click());
                let is_root = i == 0;
                let is_hl = self.hl == Some(n.0) && !is_root;
                let fill = if is_root {
                    accent
                } else if is_hl {
                    accent.gamma_multiply(0.35)
                } else if r2.hovered() {
                    hover_c
                } else {
                    box_c
                };
                let ink = if is_root { on_accent(accent) } else { ink_c };
                painter.rect(rect, 5.0_f32, fill, edge);
                if is_root {
                    painter.rect_stroke(rect.expand(2.5), 7.0_f32, Stroke::new(2.5_f32, ink_c));
                }
                if is_hl {
                    painter.rect_stroke(rect, 5.0_f32, Stroke::new(3.0_f32, accent));
                }
                let (l1, l2) = desc_lines(&self.db, n.0);
                let clip = painter.with_clip_rect(rect.shrink(2.0));
                match &l2 {
                    Some(l2) => {
                        clip.text(pos2(rect.min.x + 6.0, rect.min.y + bh * 0.30), Align2::LEFT_CENTER, l1, FontId::proportional(13.0 * z), ink);
                        clip.text(pos2(rect.min.x + 6.0, rect.min.y + bh * 0.74), Align2::LEFT_CENTER, l2, FontId::proportional(11.5 * z), ink.gamma_multiply(0.72));
                    }
                    None => {
                        clip.text(pos2(rect.min.x + 6.0, rect.center().y), Align2::LEFT_CENTER, l1, FontId::proportional(13.0 * z), ink);
                    }
                }
                let r2 = r2.on_hover_ui(|ui| {
                    let (a, b) = desc_lines(&self.db, n.0);
                    ui.label(a);
                    if let Some(b) = b {
                        ui.label(b);
                    }
                });
                let nid = n.0;
                r2.context_menu(|ui| self.person_menu(ui, nid, act));
                if r2.double_clicked() {
                    *act = Some(Action::Open(nid));
                } else if r2.clicked() {
                    *act = Some(Action::Highlight(nid));
                }
            }
            // hand tool: a layer over the whole chart that takes every click and drag, so dragging
            // anywhere moves the chart and nothing is selected by accident
            if self.settings.hand {
                hand_overlay(ui, resp.rect, "desc");
            }
        });
    }

    fn ancestors_html(&self, root: u32) -> String {
        let db = self.print_db();
        let slots = ancestor_slots(&db, root, 12);
        let mut body = String::new();
        for (g, row) in slots.iter().enumerate() {
            let found: Vec<(usize, u32)> = row.iter().enumerate().filter_map(|(i, s)| s.map(|id| (i, id))).collect();
            if found.is_empty() {
                continue;
            }
            body.push_str(&format!("<h2>Generation {}</h2><table><tr><th>No.</th><th>Name</th><th>Born</th><th>Died</th></tr>", g + 1));
            for (i, id) in found {
                let p = &db.people[&id];
                body.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    (1u64 << g) + i as u64,
                    esc(&display_name(p)),
                    dp(&p.bd, &p.bp),
                    dp(&p.dd, &p.dp)
                ));
            }
            body.push_str("</table>");
        }
        page(&format!("Ancestors of {}", db.people[&root].name), &body, false)
    }

    fn descendants_list_html(&self, root: u32) -> String {
        let db = self.print_db();
        let mut out = String::from("<ul>");
        let mut seen = vec![root];
        desc_ul(&db, root, 0, 12, &mut seen, &mut out);
        out.push_str("</ul>");
        page(&format!("Descendants of {}", db.people[&root].name), &out, false)
    }

    fn sources_html(&self) -> String {
        let db = self.print_db();
        let mut ids: Vec<u32> = db.sources.keys().copied().collect();
        ids.sort_by_key(|i| source_title(&db.sources[i]).to_lowercase());
        let counts = db.source_counts();
        let mut b = String::new();
        b.push_str(&format!("<h1>Sources</h1><p>{} source(s), {} repositor{}.</p>", ids.len(), db.repos.len(), if db.repos.len() == 1 { "y" } else { "ies" }));
        for id in ids {
            let s = &db.sources[&id];
            b.push_str(&format!("<div style=\"margin:0 0 14px;page-break-inside:avoid\"><b>{}</b>", esc(&source_title(s))));
            if !s.author.is_empty() {
                b.push_str(&format!("<br>{}", esc(&s.author)));
            }
            let mut line: Vec<String> = Vec::new();
            if !s.kind.is_empty() {
                line.push(s.kind.clone());
            }
            if !s.publ.is_empty() {
                line.push(s.publ.clone());
            }
            if !line.is_empty() {
                b.push_str(&format!("<br>{}", esc(&line.join(". "))));
            }
            if let Some(r) = s.repo.and_then(|r| db.repos.get(&r)) {
                let mut w = r.name.clone();
                if !s.call.is_empty() {
                    w = format!("{}, call number {}", w, s.call);
                }
                if !s.medium.is_empty() {
                    w = format!("{} ({})", w, s.medium);
                }
                b.push_str(&format!("<br>Kept at: {}", esc(&w)));
                if !r.addr.is_empty() {
                    b.push_str(&format!("<br><span style=\"white-space:pre-wrap\">{}</span>", esc(&r.addr)));
                }
            } else if !s.call.is_empty() {
                b.push_str(&format!("<br>Call number: {}", esc(&s.call)));
            }
            if !s.url.is_empty() {
                b.push_str(&format!("<br>{}", esc(&s.url)));
            }
            b.push_str(&format!("<br><small>Cited {} time(s)</small></div>", counts.get(&id).copied().unwrap_or(0)));
        }
        page("Sources", &b, false)
    }

    fn birthdays_html(&self) -> String {
        let db = self.print_db();
        let mut items: Vec<(u32, u32, String)> = Vec::new();
        for p in db.people.values() {
            if let Some((m, d)) = parse_md(&p.bd) {
                let born = parse_date(&p.bd).0.map(|y| format!(" (born {})", y)).unwrap_or_default();
                items.push((m, d, format!("Birthday: {}{}", esc(&display_name(p)), born)));
            }
            if let Some((m, d)) = parse_md(&p.dd) {
                let yy = parse_date(&p.dd).0.map(|y| format!(" ({})", y)).unwrap_or_default();
                items.push((m, d, format!("Remembered: {}{}", esc(&display_name(p)), yy)));
            }
        }
        for f in db.fams.values() {
            if let Some((m, d)) = parse_md(&f.md) {
                let a = f.h.map(|i| display_name(&db.people[&i])).unwrap_or_else(|| "?".to_string());
                let b = f.w.map(|i| display_name(&db.people[&i])).unwrap_or_else(|| "?".to_string());
                let yy = parse_date(&f.md).0.map(|y| format!(" (married {})", y)).unwrap_or_default();
                items.push((m, d, format!("Wedding anniversary: {} and {}{}", esc(&a), esc(&b), yy)));
            }
        }
        items.sort();
        let names = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let mut body = String::new();
        let mut cur = 0u32;
        for (m, d, t) in &items {
            if *m != cur {
                if cur != 0 {
                    body.push_str("</table>");
                }
                body.push_str(&format!("<h2>{}</h2><table>", names[(*m as usize - 1).min(11)]));
                cur = *m;
            }
            body.push_str(&format!("<tr><td class=\"l\">{}</td><td>{}</td></tr>", d, t));
        }
        if cur != 0 {
            body.push_str("</table>");
        }
        if items.is_empty() {
            body.push_str("<p>No full birthdays or anniversaries recorded yet. Add dates that include a day and month, like 12 Mar 1921.</p>");
        }
        page("Birthdays and anniversaries", &body, false)
    }

    fn descendant_svg(&self, db: &Db, root: u32) -> String {
        let maxd = self.settings.gens.clamp(3, 8);
        let mut nodes: Vec<(u32, usize, f32, Option<usize>)> = Vec::new();
        let mut leaf = 0.0f32;
        let mut seen: HashSet<u32> = HashSet::from([root]);
        let ix = db.index();
        desc_layout(db, &ix, root, 0, maxd, None, &mut nodes, &mut leaf, &mut seen);
        let rows = leaf.max(1.0);
        let (bw, bh, pitch, gap, top) = (270.0f32, 38.0f32, 44.0f32, 30.0f32, 10.0f32);
        let w = maxd as f32 * (bw + gap) + 20.0;
        let h = rows * pitch + top + 20.0;
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" font-family=\"Georgia, serif\" font-size=\"12\">", w, h);
        for n in &nodes {
            if let Some(pi) = n.3 {
                let pn = &nodes[pi];
                let px = 10.0 + pn.1 as f32 * (bw + gap) + bw;
                let py = top + (pn.2 + 0.5) * pitch;
                let cx = 10.0 + n.1 as f32 * (bw + gap);
                let cy = top + (n.2 + 0.5) * pitch;
                svg.push_str(&format!(
                    "<polyline fill=\"none\" stroke=\"#888\" points=\"{},{} {},{} {},{} {},{}\"/>",
                    px, py, px + gap / 2.0, py, px + gap / 2.0, cy, cx, cy
                ));
            }
        }
        for (i, n) in nodes.iter().enumerate() {
            let x = 10.0 + n.1 as f32 * (bw + gap);
            let cy = top + (n.2 + 0.5) * pitch;
            let cut = |t: String| if t.chars().count() > 44 { format!("{}...", t.chars().take(42).collect::<String>()) } else { t };
            let (l1, l2) = desc_lines(db, n.0);
            let (l1, l2) = (cut(l1), l2.map(cut));
            let (fill, ink) = if i == 0 { ("#2f3a4f", "#fff") } else { ("#fff", "#111") };
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"#444\"/>",
                x, cy - bh / 2.0, bw, bh, fill
            ));
            match l2 {
                Some(l2) => svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text><text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"10.5\" opacity=\"0.8\">{}</text>",
                    x + 6.0, cy - 4.0, ink, esc(&l1), x + 6.0, cy + 11.0, ink, esc(&l2)
                )),
                None => svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>", x + 6.0, cy + 4.0, ink, esc(&l1))),
            }
        }
        svg.push_str("</svg>");
        svg
    }

    fn descendant_html(&self, root: u32) -> String {
        let db = self.print_db();
        let svg = self.descendant_svg(&db, root);
        page(&format!("Descendant chart of {}", db.people[&root].name), &svg, true)
    }

    fn book_html(&self, root: u32) -> String {
        let db = self.print_db();
        let p = &db.people[&root];
        let mut b = String::new();
        b.push_str(&format!("<h2>{}</h2><table>", esc(&display_name(p))));
        let mut rows: Vec<(String, String)> = vec![
            ("Born".to_string(), dp(&p.bd, &p.bp)),
            ("Christened".to_string(), dp(&p.chd, &p.chp)),
            ("Died".to_string(), dp(&p.dd, &p.dp)),
            ("Buried".to_string(), dp(&p.bud, &p.bup)),
        ];
        for e in &p.events {
            let detail = [e.val.as_str(), e.note.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(". ");
            let note = if detail.is_empty() { String::new() } else { format!(" {}", esc(&detail)) };
            rows.push((e.kind.clone(), format!("{}{}", dp(&e.date, &e.place), note)));
        }
        for (l, v) in rows {
            if !v.trim().is_empty() {
                b.push_str(&format!("<tr><td class=\"l\">{}</td><td>{}</td></tr>", esc(&l), v));
            }
        }
        b.push_str("</table>");
        if !p.notes.is_empty() {
            b.push_str(&format!("<h2>Notes</h2><p style=\"white-space:pre-wrap\">{}</p>", esc(&p.notes)));
        }
        if !p.sources.is_empty() {
            b.push_str("<h2>Sources</h2><ul>");
            for sr in &p.sources {
                let label = if sr.about == "General" || sr.about.is_empty() { String::new() } else { format!("{}: ", sr.about) };
                b.push_str(&format!("<li>{}{}</li>", esc(&label), esc(&db.cite_text(sr))));
            }
            b.push_str("</ul>");
        }
        let imgs: Vec<&Media> = p.media.iter().filter(|m| is_image(&m.file)).collect();
        if !imgs.is_empty() {
            b.push_str("<h2>Photos</h2><div style=\"display:flex;flex-wrap:wrap;gap:12px\">");
            for m in imgs {
                let path = if !m.mid.is_empty() && m.mid != "-" { mprev(&m.mid) } else { mfile(&m.file) };
                let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                b.push_str(&format!(
                    "<figure style=\"margin:0;width:230px\"><img src=\"{}\" style=\"width:100%\"><figcaption style=\"font-size:12px\">{}</figcaption></figure>",
                    file_uri(&path),
                    esc(cap)
                ));
            }
            b.push_str("</div>");
        }
        let docs: Vec<&Media> = p.media.iter().filter(|m| !is_image(&m.file)).collect();
        if !docs.is_empty() {
            b.push_str("<h2>Documents and recorded stories</h2><ul>");
            for m in docs {
                let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                b.push_str(&format!("<li><a href=\"{}\">{}</a></li>", file_uri(&mfile(&m.file)), esc(cap)));
            }
            b.push_str("</ul>");
        }
        let tl = db.timeline(root);
        if !tl.is_empty() {
            b.push_str("<h2>Timeline</h2><table>");
            for (d, t) in &tl {
                b.push_str(&format!("<tr><td class=\"l\">{}</td><td>{}</td></tr>", esc(d), esc(t)));
            }
            b.push_str("</table>");
        }
        b.push_str("<div style=\"page-break-before:always\"></div><h2>Family</h2>");
        b.push_str(&Self::family_body(&db, root));
        let (f, m) = db.parents(root);
        if let Some(x) = f.or(m) {
            b.push_str("<h2>Parents' family</h2>");
            b.push_str(&Self::family_body(&db, x));
        }
        b.push_str("<div style=\"page-break-before:always\"></div><h2>Pedigree chart</h2>");
        b.push_str(&self.pedigree_svg(&db, root));
        if db.fams_of(root).iter().any(|f| !db.fams[f].kids.is_empty()) {
            b.push_str("<h2>Descendant chart</h2>");
            b.push_str(&self.descendant_svg(&db, root));
        }
        page(&format!("Family book: {}", p.name), &b, false)
    }

    /// Things worth checking about this person (from the data checker), shown on their page.
    fn problem_flags(&mut self, ui: &mut egui::Ui, id: u32) {
        let key = (id, self.last_json.len(), self.undo.len());
        if self.x.flags_key != key {
            let mut f: Vec<String> = self.db.check().into_iter().filter(|(who, _)| *who == id).map(|(_, t)| t).collect();
            if let Some(p) = self.db.people.get(&id) {
                let cited = !p.sources.is_empty() || p.events.iter().any(|e| !e.cites.is_empty()) || p.fx.values().any(|e| !e.cites.is_empty());
                if !cited {
                    f.push("No sources yet: nothing backs up these facts".to_string());
                }
            }
            self.x.flags = f;
            self.x.flags_key = key;
        }
        if self.x.flags.is_empty() {
            return;
        }
        let amber = Color32::from_rgb(222, 166, 60);
        egui::Frame::group(ui.style()).stroke(Stroke::new(1.0_f32, amber)).show(ui, |ui| {
            egui::CollapsingHeader::new(RichText::new(format!("{} thing(s) to check", self.x.flags.len())).color(amber)).id_salt("flags").default_open(false).show(ui, |ui| {
                for t in &self.x.flags {
                    ui.label(format!("\u{2022} {}", t));
                }
                ui.label(RichText::new("Tools > Data checker lists these for the whole tree.").small().weak());
            });
        });
    }

    /// One person's research tasks. Works whether or not the page is locked: ticking off a
    /// task does not change any facts about the person.
    fn todo_panel(&mut self, ui: &mut egui::Ui, id: u32) {
        let mut remove: Option<usize> = None;
        let mut changed = false;
        if let Some(p) = self.db.people.get_mut(&id) {
            if p.todos.is_empty() {
                ui.label(RichText::new("Nothing to do yet. Add what you still want to find, like \"1880 census\" or \"marriage record\".").italics().weak());
            }
            for (i, t) in p.todos.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut t.done, "").on_hover_text("Done").changed() {
                        changed = true;
                    }
                    let mut txt = RichText::new(t.text.as_str()).size(15.0);
                    if t.done {
                        txt = txt.strikethrough().weak();
                    }
                    ui.add(egui::Label::new(txt).wrap());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").on_hover_text("Remove this task").clicked() {
                            remove = Some(i);
                        }
                        if !t.added.is_empty() {
                            ui.label(RichText::new(t.added.as_str()).small().weak());
                        }
                    });
                });
            }
            if let Some(i) = remove {
                p.todos.remove(i);
                changed = true;
            }
        }
        let mut add = false;
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.x.todo_new).hint_text("New task").desired_width(220.0));
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                add = true;
            }
            if ui.button("Add task").clicked() {
                add = true;
            }
        });
        if add && !self.x.todo_new.trim().is_empty() {
            if let Some(p) = self.db.people.get_mut(&id) {
                p.todos.push(Todo { text: self.x.todo_new.trim().to_string(), added: today(), ..Default::default() });
                changed = true;
            }
            self.x.todo_new.clear();
        }
        if changed {
            self.dirty = true;
        }
    }

    /// The first screen when the tree is empty: import (from PAF or anything else), start new,
    /// open an archive, or explore the sample family without touching anything.
    fn welcome(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.add(egui::Image::new(egui::include_image!("../icons/icon-256.png")).max_width(96.0));
                ui.label(RichText::new("Welcome to Gilded Roots").size(30.0).strong());
                ui.label(RichText::new("Your family history, kept on your own computer.").size(16.0));
                ui.label(RichText::new("No account. No subscription. No advertising. Nothing is uploaded anywhere.").weak());
                ui.add_space(18.0);
                ui.label(RichText::new("How would you like to get started?").size(18.0));
                ui.add_space(10.0);
            });
            let card = |ui: &mut egui::Ui, title: &str, body: &str| -> bool {
                let mut clicked = false;
                egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(14.0)).rounding(10.0).show(ui, |ui| {
                    ui.set_width(ui.available_width().min(620.0));
                    let r = ui.add(egui::Button::new(RichText::new(title).size(18.0).strong()).frame(false));
                    ui.label(body);
                    clicked = r.clicked();
                    ui.add_space(2.0);
                    if ui.button("Choose").clicked() {
                        clicked = true;
                    }
                });
                clicked
            };
            ui.vertical_centered(|ui| {
                ui.set_max_width(640.0);
                if card(ui, "Import an existing family tree", "Bring in your research from Ancestry, FamilySearch, MyHeritage, RootsMagic, Legacy, PAF or any program that saves GEDCOM (.ged) files. PAF 5 .paf files can be opened directly. You get a full report of what came across.") {
                    self.imp_open = true;
                }
                ui.horizontal(|ui| {
                    if ui.link(if self.x.paf_help { "Hide the PAF help" } else { "Coming from PAF? Show me how" }).clicked() {
                        self.x.paf_help = !self.x.paf_help;
                    }
                });
                if self.x.paf_help {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(ui.available_width().min(620.0));
                        ui.label(RichText::new("The quick way").strong());
                        ui.label("Choose Import an existing family tree and pick your PAF 5 file (the one ending in .paf). Gilded Roots reads names, sex, births, deaths, marriages and all the family links straight from it, without PAF.");
                        ui.label(RichText::new("Have christenings, burials, LDS ordinances, notes, sources or photos in PAF? Use the full way below for now, so those come across too. The import report tells you if anything was left behind.").small().weak());
                        ui.add_space(6.0);
                        ui.label(RichText::new("The full way: export a GEDCOM from PAF").strong());
                        ui.label("1. Open your family file in PAF.");
                        ui.label("2. Choose File > Export. Pick GEDCOM 5.5, character set ANSEL or UTF-8, and tick 'Include notes', 'Include sources' and 'Include multimedia links'.");
                        ui.label("3. Save the .ged file somewhere easy, such as your Documents folder. If your PAF photos live in a folder, keep that folder next to the .ged file.");
                        ui.label("4. Here, choose Import an existing family tree and pick the .ged file. Gilded Roots copies in the photos it can find and shows a report.");
                        ui.label(RichText::new("Your PAF file is not changed. You can import again at any time.").small().weak());
                    });
                }
                ui.add_space(8.0);
                if card(ui, "Create a new family tree", "Start from scratch. Add yourself first, then parents, grandparents and so on.") {
                    *act = Some(Action::Add(Kind::Person));
                }
                ui.add_space(8.0);
                if card(ui, "Open a Family Tree Archive", "Continue from an archive file made with File > Save Family Tree Archive, for example from another computer. It brings back the tree, photos, documents and stories.") {
                    self.open_archive_pick();
                }
                ui.add_space(8.0);
                if card(ui, "Explore the sample family first", "A made-up family over five generations, with photos, sources, marriage records, documents, stories and a map. Try anything: nothing you do there is saved, and your own tree is never touched.") {
                    self.start_practice();
                }
                ui.add_space(16.0);
                ui.label(RichText::new("Press F1 at any time for the user guide.").weak());
            });
        });
    }

    /// Open the sample family in practice mode. The real tree waits untouched.
    fn start_practice(&mut self) {
        if self.x.practice.is_some() {
            return;
        }
        if self.editing.is_some() {
            self.editing = None;
        }
        let real = std::mem::replace(&mut self.db, sample_family());
        self.x.practice = Some((real, self.sel, self.view));
        self.sel = self.db.people.iter().find(|(_, p)| p.name == "Emily Rose Harwood").map(|(k, _)| *k).or_else(|| self.db.people.keys().next().copied());
        self.view = View::Pedigree;
        self.back.clear();
        self.fwd.clear();
        self.undo.clear();
        self.redo.clear();
        self.last_json = serde_json::to_string(&self.db).unwrap_or_default();
        self.remember_state();
        self.x.geo_cache.clear();
        self.status = "You are exploring the sample family. Nothing here is saved. Use 'Back to my tree' at the top when you are done.".into();
    }

    fn end_practice(&mut self) {
        if let Some((real, sel, view)) = self.x.practice.take() {
            self.db = real;
            self.sel = sel;
            self.view = view;
            self.editing = None;
            self.back.clear();
            self.fwd.clear();
            self.undo.clear();
            self.redo.clear();
            self.viewing = None;
            self.last_json = serde_json::to_string(&self.db).unwrap_or_default();
            self.remember_state();
            self.x.geo_cache.clear();
            self.status = "Back to your own tree.".into();
        }
    }

    fn save_archive_pick(&mut self) {
        if self.x.practice.is_some() {
            self.status = "This is the sample family; go back to your tree to archive it.".into();
            return;
        }
        let name = format!("Gilded Roots archive {}.zip", today());
        if let Some(path) = rfd::FileDialog::new().set_title("Save Family Tree Archive").set_directory(start_dir()).set_file_name(&name).add_filter("Family Tree Archive", &["zip"]).save_file() {
            remember_dir(&path);
            let db = self.db.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let r = archive::write_archive(&db, &path).map(|i| format!("Saved the Family Tree Archive {} ({} people, {} families, {} photos and documents). Keep a copy somewhere safe.", path.display(), i.people, i.families, i.media));
                let _ = tx.send(r);
            });
            self.x.arch_job = Some(rx);
            self.status = "Making the Family Tree Archive...".into();
        }
    }

    fn open_archive_pick(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title("Open Family Tree Archive").set_directory(start_dir()).add_filter("Family Tree Archive", &["zip"]).pick_file() {
            remember_dir(&path);
            match archive::read_info(&path) {
                Ok(info) => self.x.arch_confirm = Some((path, info)),
                Err(e) => {
                    self.status = e.clone();
                    self.notice = Some(e);
                }
            }
        }
    }

    /// Windows for the archive (confirm restore, progress) and backup settings.
    fn archive_windows(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.x.arch_job {
            if let Ok(r) = rx.try_recv() {
                match r {
                    Ok(s) => self.status = s,
                    Err(e) => {
                        self.status = format!("The archive could not be made: {}", e);
                        self.notice = Some(self.status.clone());
                    }
                }
                self.x.arch_job = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(300));
            }
        }
        if let Some((path, info)) = &self.x.arch_confirm {
            let mut go = false;
            let mut cancel = false;
            let has_tree = !self.db.people.is_empty() && self.x.practice.is_none();
            egui::Window::new("Open Family Tree Archive").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.set_max_width(440.0);
                ui.label(RichText::new(path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()).strong());
                ui.label(format!("Made {}. {} people, {} families, {} photos and documents.", if info.made.is_empty() { "on an unknown date".to_string() } else { print_date(&info.made) }, info.people, info.families, info.media));
                ui.add_space(6.0);
                if has_tree {
                    ui.colored_label(Color32::from_rgb(222, 166, 60), format!("This replaces the tree you have open ({} people). Your current tree is kept as a backup first (Tools > Restore previous tree brings it back).", self.db.people.len()));
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Restore this archive").strong()).clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if go {
                let path = path.clone();
                self.x.arch_confirm = None;
                if self.x.practice.is_some() {
                    self.end_practice();
                }
                if self.backup() {
                    match archive::read_archive(&path) {
                        Ok((mut db, n)) => {
                            fix_names(&mut db);
                            // previews are made again on this computer
                            for p in db.people.values_mut() {
                                for m in p.media.iter_mut() {
                                    for f in [&mut m.thumb, &mut m.mid] {
                                        if !f.is_empty() && f != "-" && !mprev(f.as_str()).is_file() {
                                            f.clear();
                                        }
                                    }
                                }
                            }
                            self.db = db;
                            self.sel = self.db.people.keys().next().copied();
                            self.dirty = true;
                            self.undo.clear();
                            self.redo.clear();
                            self.status = format!("Restored the archive: {} people and {} photos and documents.", self.db.people.len(), n);
                        }
                        Err(e) => {
                            self.status = e.clone();
                            self.notice = Some(e);
                        }
                    }
                }
            } else if cancel {
                self.x.arch_confirm = None;
            }
        }
        if self.x.backup_open {
            let mut open = true;
            let mut changed = false;
            egui::Window::new("Backup settings").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.set_max_width(480.0);
                ui.label(RichText::new("Automatic backups").strong());
                ui.horizontal(|ui| {
                    for (v, l) in [(1u8, "Every hour"), (0, "Every day"), (2, "Every week")] {
                        if ui.selectable_label(self.settings.backup_every == v, l).clicked() {
                            self.settings.backup_every = v;
                            changed = true;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Keep the last");
                    if ui.add(egui::DragValue::new(&mut self.settings.backup_keep).range(3..=365)).changed() {
                        changed = true;
                    }
                    ui.label("backups");
                });
                ui.label(RichText::new(format!("Backups are checked after they are written, and kept in {}", data_dir().join("backups").display())).small().weak());
                ui.add_space(8.0);
                ui.label(RichText::new("Second copy somewhere else (recommended)").strong());
                ui.label("Choose a USB stick, external drive or another folder. Each backup also saves a complete Family Tree Archive there, with every photo and document.");
                ui.horizontal(|ui| {
                    ui.label(if self.settings.backup_extra.is_empty() { "Not set".to_string() } else { self.settings.backup_extra.clone() });
                    if ui.button("Choose folder...").clicked() {
                        if let Some(d) = rfd::FileDialog::new().set_title("Choose where to keep a second copy").pick_folder() {
                            self.settings.backup_extra = d.display().to_string();
                            changed = true;
                        }
                    }
                    if !self.settings.backup_extra.is_empty() && ui.button("Stop").clicked() {
                        self.settings.backup_extra.clear();
                        changed = true;
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Back up now").clicked() {
                        self.x.backup_check = -1.0e9;
                        self.backup_now(true);
                    }
                    if ui.button("Check backups...").clicked() {
                        self.run_verify();
                    }
                    if ui.button("Saved versions...").clicked() {
                        self.x.ver_open = true;
                    }
                });
            });
            if changed {
                save_settings(&self.settings);
            }
            self.x.backup_open = open;
        }
    }

    /// Make a backup now: the checked copy in the backups folder, the photos mirror, and (when a
    /// second folder is chosen) a Family Tree Archive there.
    fn backup_now(&mut self, force: bool) {
        if self.x.practice.is_some() || self.db.people.is_empty() {
            return;
        }
        BACKUP_KEEP.store(self.settings.backup_keep, Ordering::Relaxed);
        BACKUP_HOURLY.store(self.settings.backup_every == 1, Ordering::Relaxed);
        match dated_backup_in(&data_dir().join("backups"), &self.db, force) {
            Ok(made) => {
                if force {
                    self.status = "Backup saved and checked.".to_string();
                }
                if made || force {
                    self.spawn_media_job(false);
                    let extra = self.settings.backup_extra.trim().to_string();
                    if !extra.is_empty() && self.x.arch_job.is_none() {
                        let dir = PathBuf::from(&extra);
                        if dir.is_dir() {
                            let db = self.db.clone();
                            let keep = self.settings.backup_keep;
                            let (tx, rx) = std::sync::mpsc::channel();
                            std::thread::spawn(move || {
                                let path = dir.join(format!("Gilded Roots archive {}.zip", today()));
                                let r = archive::write_archive(&db, &path).map(|_| format!("Backup saved, and an archive copy was saved in {}", dir.display()));
                                // keep only the newest archives there too
                                if let Ok(rd) = std::fs::read_dir(&dir) {
                                    let mut v: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("Gilded Roots archive ") && n.to_string_lossy().ends_with(".zip")).unwrap_or(false)).collect();
                                    v.sort();
                                    while v.len() > keep.max(3) {
                                        let _ = std::fs::remove_file(v.remove(0));
                                    }
                                }
                                let _ = tx.send(r);
                            });
                            self.x.arch_job = Some(rx);
                        } else {
                            self.status = format!("Backup saved, but the second-copy folder {} is not available (is the drive plugged in?).", extra);
                        }
                    }
                }
            }
            Err(e) => {
                self.status = format!("Backup failed: {}", e);
                self.notice = Some(self.status.clone());
            }
        }
    }

    /// Called after changes: makes a backup when one is due by the chosen frequency.
    fn backup_if_due(&mut self, now: f64) {
        if now - self.x.backup_check < 60.0 {
            return;
        }
        self.x.backup_check = now;
        let bdir = data_dir().join("backups");
        let newest = list_backups(&bdir).into_iter().filter_map(|n| std::fs::metadata(bdir.join(n)).and_then(|m| m.modified()).ok()).max();
        let age = newest.and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()).unwrap_or(u64::MAX);
        let due = match self.settings.backup_every {
            1 => age >= 3600,
            2 => age >= 7 * 86400,
            _ => today() != self.last_backup_day || age >= 86400,
        };
        if due {
            self.backup_now(false);
            self.last_backup_day = today();
        }
    }

    /// Switch to another tree ("" = the main tree). The open one is saved first.
    fn switch_tree(&mut self, name: &str) {
        if self.x.practice.is_some() {
            self.end_practice();
        }
        if self.dirty && !self.persist() {
            self.status = "Your changes could not be saved, so the tree was not switched.".into();
            return;
        }
        self.editing = None;
        if let Ok(mut g) = CURRENT_TREE.lock() {
            *g = if name.is_empty() { None } else { Some(name.to_string()) };
        }
        let _ = std::fs::create_dir_all(data_dir());
        let (mut db, notice) = load_from(&data_dir());
        fix_names(&mut db);
        self.db = db;
        self.sel = self.db.people.keys().next().copied();
        self.settings.current_tree = name.to_string();
        if self.settings.home.map(|h| !self.db.people.contains_key(&h)).unwrap_or(false) {
            self.settings.home = None;
        }
        save_settings(&self.settings);
        self.back.clear();
        self.fwd.clear();
        self.undo.clear();
        self.redo.clear();
        self.viewing = None;
        self.hl = None;
        self.dirty = false;
        self.last_json = serde_json::to_string(&self.db).unwrap_or_default();
        self.remember_state();
        self.x.geo_cache.clear();
        self.x.stats = None;
        self.last_backup_day = String::new();
        self.status = if notice.is_empty() { format!("Opened {}.", tree_title(name)) } else { notice };
    }

    /// File > Family trees: keep unrelated research in separate trees.
    fn trees_window(&mut self, ctx: &egui::Context) {
        if !self.x.trees_open {
            return;
        }
        let mut open = true;
        let mut go: Option<String> = None;
        let cur = self.settings.current_tree.clone();
        let trees = other_trees();
        egui::Window::new("Family trees").open(&mut open).resizable(false).show(ctx, |ui| {
            ui.set_max_width(460.0);
            ui.label("Keep separate research projects apart: your own family, your spouse's, a client's, a one-name study. Each tree has its own people, photos and backups.");
            ui.add_space(6.0);
            for name in std::iter::once(String::new()).chain(trees.iter().cloned()) {
                ui.horizontal(|ui| {
                    let here = name == cur;
                    let label = tree_title(&name);
                    ui.label(if here { RichText::new(format!("\u{2022} {}", label)).strong() } else { RichText::new(label) });
                    if here {
                        ui.label(RichText::new("(open)").small().weak());
                    } else if ui.button("Open").clicked() {
                        go = Some(name.clone());
                    }
                    if !name.is_empty() && ui.small_button("Rename...").clicked() {
                        self.x.tree_rename = Some((name.clone(), name.clone()));
                    }
                });
            }
            if let Some((old, new)) = self.x.tree_rename.as_mut() {
                ui.horizontal(|ui| {
                    ui.label(format!("New name for {}:", old));
                    ui.add(egui::TextEdit::singleline(new).desired_width(160.0));
                });
                let (old, new) = (old.clone(), sanitize_tree_name(new));
                ui.horizontal(|ui| {
                    if ui.add_enabled(!new.is_empty() && new != old, egui::Button::new("Rename")).clicked() {
                        let root = base_dir().join("More trees");
                        if root.join(&new).exists() {
                            self.status = "A tree with that name already exists.".into();
                        } else if std::fs::rename(root.join(&old), root.join(&new)).is_ok() {
                            if cur == old {
                                if let Ok(mut g) = CURRENT_TREE.lock() {
                                    *g = Some(new.clone());
                                }
                                self.settings.current_tree = new.clone();
                                save_settings(&self.settings);
                            }
                            self.status = format!("Renamed to {}.", new);
                        } else {
                            self.status = "That tree could not be renamed (is a file in it open in another program?).".into();
                        }
                        self.x.tree_rename = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.x.tree_rename = None;
                    }
                });
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.x.tree_new).hint_text("Name for a new tree").desired_width(200.0));
                let name = sanitize_tree_name(&self.x.tree_new);
                if ui.add_enabled(!name.is_empty(), egui::Button::new("Create and open")).clicked() {
                    let dir = base_dir().join("More trees").join(&name);
                    if dir.exists() {
                        self.status = "A tree with that name already exists.".into();
                    } else if std::fs::create_dir_all(&dir).is_ok() {
                        go = Some(name.clone());
                        self.x.tree_new.clear();
                    }
                }
            });
            ui.label(RichText::new(format!("Extra trees are kept in {}", base_dir().join("More trees").display())).small().weak());
        });
        if let Some(n) = go {
            self.switch_tree(&n);
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("Gilded Roots - {}", tree_title(&n))));
        }
        self.x.trees_open = open;
    }

    /// Every research task in the tree in one list.
    fn todo_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.todo_open {
            return;
        }
        let mut open = true;
        let mut go: Option<u32> = None;
        let mut changed = false;
        let q = self.x.todo_find.trim().to_lowercase();
        let show_done = self.x.todo_done;
        let total_open: usize = self.db.people.values().map(|p| p.todos.iter().filter(|t| !t.done).count()).sum();
        let mut find = self.x.todo_find.clone();
        let mut sd = show_done;
        let mut ids: Vec<u32> = self.db.people.iter().filter(|(_, p)| !p.todos.is_empty()).map(|(i, _)| *i).collect();
        ids.sort_by(|a, b| self.db.people[a].name.cmp(&self.db.people[b].name));
        egui::Window::new("Research to-do list").open(&mut open).default_size([560.0, 520.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} open task(s)", total_open)).strong());
                ui.add(egui::TextEdit::singleline(&mut find).hint_text("Filter by task or name").desired_width(200.0));
                ui.checkbox(&mut sd, "Show finished");
            });
            ui.label(RichText::new("Add tasks from a person's Individual page (Research to-do panel).").small().weak());
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let mut any = false;
                for id in &ids {
                    let Some(p) = self.db.people.get_mut(id) else { continue };
                    let name_hit = p.name.to_lowercase().contains(&q);
                    let rows: Vec<usize> = p.todos.iter().enumerate().filter(|(_, t)| (sd || !t.done) && (q.is_empty() || name_hit || t.text.to_lowercase().contains(&q))).map(|(i, _)| i).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    any = true;
                    let label = p.name.clone();
                    ui.add_space(4.0);
                    if ui.link(RichText::new(label).size(15.0).strong()).clicked() {
                        go = Some(*id);
                    }
                    for i in rows {
                        let t = &mut p.todos[i];
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            if ui.checkbox(&mut t.done, "").changed() {
                                changed = true;
                            }
                            let mut txt = RichText::new(t.text.as_str());
                            if t.done {
                                txt = txt.strikethrough().weak();
                            }
                            ui.label(txt);
                            if !t.added.is_empty() {
                                ui.label(RichText::new(t.added.as_str()).small().weak());
                            }
                        });
                    }
                }
                if !any {
                    ui.label(RichText::new("No tasks match.").italics().weak());
                }
            });
        });
        self.x.todo_find = find;
        self.x.todo_done = sd;
        if changed {
            self.dirty = true;
        }
        if let Some(i) = go {
            *act = Some(Action::Open(i));
        }
        self.x.todo_open = open;
    }

    /// Tools > Places: every place in the tree, who was there, and fixing spellings.
    fn places_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.places_open {
            return;
        }
        let uses = self.db.place_uses();
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for k in uses.keys() {
            groups.entry(place_key(k)).or_default().push(k.clone());
        }
        let mut dupes: Vec<Vec<String>> = groups.into_values().filter(|v| v.len() > 1).collect();
        dupes.sort();
        let mut open = true;
        let mut find = self.x.places_find.clone();
        let mut sel = self.x.places_sel.clone();
        let mut by_count = self.x.places_by_count;
        let mut rename = self.x.places_rename.clone();
        let mut do_rename: Option<(String, String)> = None;
        let mut go: Option<u32> = None;
        egui::Window::new("Places").open(&mut open).default_size([860.0, 560.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} different places", uses.len())).strong());
                ui.add(egui::TextEdit::singleline(&mut find).hint_text("Search places").desired_width(220.0));
                ui.checkbox(&mut by_count, "Most used first");
                if !dupes.is_empty() {
                    ui.label(RichText::new(format!("{} place(s) spelled more than one way", dupes.len())).color(Color32::from_rgb(220, 160, 60)));
                }
            });
            ui.separator();
            egui::SidePanel::left("places_list").resizable(true).default_width(360.0).show_inside(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("pl_scroll").auto_shrink([false, false]).show(ui, |ui| {
                    if !dupes.is_empty() && find.trim().is_empty() {
                        egui::CollapsingHeader::new(RichText::new("Spelled more than one way").strong()).default_open(true).show(ui, |ui| {
                            for g in &dupes {
                                ui.horizontal_wrapped(|ui| {
                                    for v in g {
                                        let n = uses.get(v).map(|x| x.len()).unwrap_or(0);
                                        if ui.selectable_label(sel.as_deref() == Some(v.as_str()), format!("{} ({})", v, n)).clicked() {
                                            sel = Some(v.clone());
                                            rename = g.iter().max_by_key(|x| uses.get(*x).map(|u| u.len()).unwrap_or(0)).cloned().unwrap_or_default();
                                        }
                                    }
                                });
                                ui.separator();
                            }
                        });
                        ui.add_space(6.0);
                    }
                    let q = find.trim().to_lowercase();
                    let mut list: Vec<(&String, usize)> = uses.iter().filter(|(k, _)| q.is_empty() || k.to_lowercase().contains(&q)).map(|(k, v)| (k, v.len())).collect();
                    if by_count {
                        list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
                    }
                    for (k, n) in list.into_iter().take(3000) {
                        if ui.selectable_label(sel.as_deref() == Some(k.as_str()), format!("{}   ({})", k, n)).clicked() {
                            sel = Some(k.clone());
                            rename = k.clone();
                        }
                    }
                });
            });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let Some(pl) = sel.clone() else {
                    ui.label(RichText::new("Pick a place on the left to see who was there.").italics().weak());
                    return;
                };
                ui.label(RichText::new(pl.as_str()).size(20.0).strong());
                let list = uses.get(&pl).cloned().unwrap_or_default();
                ui.label(format!("Used {} time(s)", list.len()));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Change this spelling to");
                    ui.add(egui::TextEdit::singleline(&mut rename).desired_width(260.0));
                    if ui.add_enabled(!rename.trim().is_empty() && rename.trim() != pl, egui::Button::new("Change everywhere")).on_hover_text("Every fact that uses this exact spelling is changed. Edit > Undo puts it back.").clicked() {
                        do_rename = Some((pl.clone(), rename.clone()));
                    }
                });
                ui.separator();
                let mut rows = list.clone();
                rows.sort_by(|a, b| parse_date(&a.2).0.cmp(&parse_date(&b.2).0));
                egui::ScrollArea::vertical().id_salt("pl_people").auto_shrink([false, false]).show(ui, |ui| {
                    egui::Grid::new("pl_grid").num_columns(3).striped(true).spacing([16.0, 4.0]).show(ui, |ui| {
                        for (id, what, date) in &rows {
                            if ui.link(self.db.label(*id)).clicked() {
                                go = Some(*id);
                            }
                            ui.label(what.as_str());
                            ui.label(RichText::new(if date.is_empty() { "-" } else { date.as_str() }).weak());
                            ui.end_row();
                        }
                    });
                });
            });
        });
        if let Some((from, to)) = do_rename {
            let n = self.db.rename_place(&from, &to);
            self.x.next_label = Some(format!("Changed place \"{}\" to \"{}\"", from, to.trim()));
            self.dirty = true;
            self.status = format!("Changed {} fact(s) from \"{}\" to \"{}\". Edit > Undo puts them back.", n, from, to.trim());
            sel = Some(to.trim().to_string());
            rename = to.trim().to_string();
        }
        if let Some(i) = go {
            *act = Some(Action::Open(i));
        }
        self.x.places_find = find;
        self.x.places_sel = sel;
        self.x.places_by_count = by_count;
        self.x.places_rename = rename;
        self.x.places_open = open;
    }

    /// Keys that work anywhere. On a Mac the top-row keys are brightness and volume unless fn is
    /// held, so help and full screen also have the usual Mac shortcuts.
    ///   F1 help   F2 edit person   F3 find   F4 quick jump   F5 fit chart   F6 next tab
    ///   F7 data checker   F8 research to-do   F9 places   F11 full screen   F12 history
    fn global_keys(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        use egui::{Key, Modifiers as M};
        let typing = ctx.wants_keyboard_input();
        let pressed = ctx.input_mut(|i| {
            let mut v: Vec<u8> = Vec::new();
            for (n, k) in [(1u8, Key::F1), (2, Key::F2), (3, Key::F3), (4, Key::F4), (5, Key::F5), (6, Key::F6), (7, Key::F7), (8, Key::F8), (9, Key::F9), (11, Key::F11), (12, Key::F12)] {
                if i.consume_key(M::NONE, k) {
                    v.push(n);
                }
            }
            if i.consume_key(M::COMMAND | M::SHIFT, Key::Slash) || i.consume_key(M::COMMAND, Key::Questionmark) || i.consume_key(M::COMMAND | M::SHIFT, Key::Questionmark) {
                v.push(1);
            }
            if i.consume_key(M::COMMAND | M::CTRL, Key::F) {
                v.push(11);
            }
            v
        });
        for n in pressed {
            match n {
                1 => self.help_open = !self.help_open,
                2 if self.sel.is_some() && self.editing.is_none() => *act = Some(Action::EditStart),
                3 => self.find_open = true,
                4 => {
                    self.x.jump_open = !self.x.jump_open;
                    self.x.jump_q.clear();
                    self.x.jump_sel = 0;
                }
                5 if !typing => *act = Some(Action::Fit),
                6 => {
                    self.view = match self.view {
                        View::Family => View::Pedigree,
                        View::Pedigree => View::Descendants,
                        View::Descendants => View::Person,
                        View::Person => View::Gallery,
                        View::Gallery => View::Memories,
                        View::Memories => View::Charts,
                        View::Charts => View::Table,
                        View::Table => View::Map,
                        View::Map => View::Dna,
                        View::Dna => View::Family,
                    }
                }
                7 => self.check_open = true,
                8 => self.x.todo_open = !self.x.todo_open,
                9 => self.x.places_open = !self.x.places_open,
                11 => {
                    let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                }
                12 => self.x.history_open = !self.x.history_open,
                _ => {}
            }
        }
    }

    /// Everything quick jump can offer for the text typed so far: people first, then commands.
    fn jump_results(&self, q: &str) -> Vec<(String, String, Jump)> {
        let q = q.trim().to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();
        let mut out: Vec<(String, String, Jump)> = Vec::new();
        let sel = self.sel;
        let mut cmds: Vec<(&str, Jump)> = vec![
            ("Go to home person", Jump::Act(Action::GoHome)),
            ("Add a new person", Jump::Act(Action::Add(Kind::Person))),
            ("Family view", Jump::View(View::Family)),
            ("Pedigree view", Jump::View(View::Pedigree)),
            ("Descendants view", Jump::View(View::Descendants)),
            ("Individual page", Jump::View(View::Person)),
            ("Gallery", Jump::View(View::Gallery)),
            ("Charts: fan chart, hourglass, lifespans", Jump::View(View::Charts)),
            ("People table (everyone in a list)", Jump::View(View::Table)),
            ("Map of places", Jump::View(View::Map)),
            ("Memories: stories, recordings, documents, notes", Jump::View(View::Memories)),
            ("DNA: ethnicity estimate and matches", Jump::View(View::Dna)),
            ("Write a story", Jump::Window("story")),
            ("Research to-do list", Jump::Window("todo")),
            ("Places", Jump::Window("places")),
            ("History of changes", Jump::Window("history")),
            ("Research journal", Jump::Window("journal")),
            ("Family statistics", Jump::Window("stats")),
            ("Tombstone calculator (birth from age at death)", Jump::Window("tomb")),
            ("Blank census forms", Jump::Window("census")),
            ("Report designer", Jump::Window("designer")),
            ("Family trees (switch or create)", Jump::Window("trees")),
            ("Historical events", Jump::Window("hist")),
            ("Save Family Tree Archive", Jump::Window("archive_save")),
            ("Open Family Tree Archive", Jump::Window("archive_open")),
            ("Backup settings", Jump::Window("backup")),
            ("Explore the sample family (practice)", Jump::Window("practice")),
            ("Saved versions (daily copies)", Jump::Window("versions")),
            ("Sources and repositories", Jump::Window("sources")),
            ("Data checker", Jump::Window("check")),
            ("Find duplicates", Jump::Window("dups")),
            ("Relationship calculator", Jump::Window("rel")),
            ("Home person (choose)", Jump::Window("home")),
            ("Import tree", Jump::Window("import")),
            ("Export GEDCOM", Jump::Window("export")),
            ("User guide / help", Jump::Window("help")),
            ("Fit chart to window", Jump::Act(Action::Fit)),
            ("Undo", Jump::Window("undo")),
            ("Redo", Jump::Window("redo")),
        ];
        if let Some(i) = sel {
            cmds.push(("Edit this person", Jump::Act(Action::EditStart)));
            cmds.push(("Quick edit this person", Jump::Act(Action::QuickEdit(i))));
            cmds.push(("Add father", Jump::Act(Action::AddFor(i, Kind::Father))));
            cmds.push(("Add mother", Jump::Act(Action::AddFor(i, Kind::Mother))));
            cmds.push(("Add spouse or partner", Jump::Act(Action::AddFor(i, Kind::Spouse))));
            cmds.push(("Add child", Jump::Act(Action::AddFor(i, Kind::Child(self.db.fams_of(i).first().copied())))));
            cmds.push(("Add brother or sister", Jump::Act(Action::AddFor(i, Kind::Sibling))));
            cmds.push(("Add a research task for this person", Jump::Window("todo_add")));
            cmds.push(("Add photos or documents", Jump::Act(Action::AttachFiles)));
        }
        let hit = |hay: &str| words.iter().all(|w| hay.contains(w));
        if words.is_empty() {
            for b in self.db.bookmarks.iter().filter(|b| self.db.people.contains_key(b)) {
                out.push((format!("\u{2605} {}", self.db.label(*b)), "person".into(), Jump::Act(Action::Select(*b))));
            }
        }
        if !words.is_empty() {
            let mut people: Vec<(i32, u32, String)> = Vec::new();
            for (id, p) in &self.db.people {
                let l = life(p);
                let hay = format!("{} {} #{} {}", p.name.to_lowercase(), l, id, p.nick.to_lowercase());
                if hit(&hay) {
                    // names that start with what was typed come first
                    let score = if p.name.to_lowercase().starts_with(words[0]) { 0 } else { 1 };
                    people.push((score, *id, if l.is_empty() { p.name.clone() } else { format!("{}  ({})", p.name, l) }));
                }
            }
            people.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
            for (_, id, label) in people.into_iter().take(30) {
                out.push((label, "person".into(), Jump::Act(Action::Select(id))));
            }
        }
        for (label, j) in cmds {
            if words.is_empty() || hit(&label.to_lowercase()) {
                out.push((label.to_string(), "command".into(), j));
            }
        }
        out
    }

    fn run_jump(&mut self, j: Jump, act: &mut Option<Action>) {
        match j {
            Jump::Act(a) => *act = Some(a),
            Jump::View(v) => self.view = v,
            Jump::Window(w) => match w {
                "todo" => self.x.todo_open = true,
                "todo_add" => {
                    self.x.todo_open = true;
                    self.view = View::Person;
                }
                "places" => self.x.places_open = true,
                "history" => self.x.history_open = true,
                "versions" => self.x.ver_open = true,
                "journal" => self.x.journal_open = true,
                "stats" => self.x.stats_open = true,
                "tomb" => self.open_tombstone(self.sel),
                "census" => self.x.census.open = true,
                "designer" => self.x.designer_open = true,
                "trees" => self.x.trees_open = true,
                "hist" => self.x.hist_open = true,
                "archive_save" => self.save_archive_pick(),
                "archive_open" => self.open_archive_pick(),
                "backup" => self.x.backup_open = true,
                "practice" => self.start_practice(),
                "story" => {
                    if let Some(s) = self.sel {
                        self.x.story = Some(StoryEdit { who: s, idx: None, title: String::new(), text: String::new() });
                    }
                }
                "sources" => self.src_open = true,
                "check" => self.check_open = true,
                "dups" => {
                    self.dups = self.db.duplicate_pairs().into_iter().filter(|x| !self.dup_ignore.contains(&(x.0, x.1))).collect();
                    self.dup_open = true;
                }
                "rel" => self.rel_open = true,
                "home" => self.home_pick = Some(String::new()),
                "import" => self.imp_open = true,
                "export" => self.export(false),
                "help" => self.help_open = true,
                "undo" => self.undo_action(),
                "redo" => self.redo_action(),
                _ => {}
            },
        }
    }

    /// Ctrl/Cmd+K: type a name or a command, arrows to choose, Enter to go.
    fn jump_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            self.x.jump_open = !self.x.jump_open;
            self.x.jump_q.clear();
            self.x.jump_sel = 0;
        }
        if !self.x.jump_open {
            return;
        }
        let mut q = self.x.jump_q.clone();
        let results = self.jump_results(&q);
        let n = results.len();
        let (down, up, enter, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        if down && n > 0 {
            self.x.jump_sel = (self.x.jump_sel + 1).min(n - 1);
        }
        if up {
            self.x.jump_sel = self.x.jump_sel.saturating_sub(1);
        }
        let mut pick: Option<usize> = if enter && n > 0 { Some(self.x.jump_sel.min(n - 1)) } else { None };
        let mut close = esc;
        let cur = self.x.jump_sel;
        egui::Window::new("Quick jump")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size([520.0, 0.0])
            .anchor(Align2::CENTER_TOP, [0.0, 70.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut q).hint_text("Type a name, a year, an ID, or what you want to do...").desired_width(f32::INFINITY).font(egui::TextStyle::Heading));
                r.request_focus();
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    let mut last_kind = String::new();
                    for (k, (label, kind, _)) in results.iter().enumerate() {
                        if *kind != last_kind {
                            ui.label(RichText::new(if kind == "person" { "People" } else { "Commands" }).small().weak());
                            last_kind = kind.clone();
                        }
                        let rr = ui.selectable_label(k == cur, RichText::new(label.as_str()).size(15.0));
                        if k == cur {
                            rr.scroll_to_me(None);
                        }
                        if rr.clicked() {
                            pick = Some(k);
                        }
                    }
                    if results.is_empty() {
                        ui.label(RichText::new("Nothing matches.").italics().weak());
                    }
                });
                ui.label(RichText::new("Up/Down to choose, Enter to go, Esc to close").small().weak());
            });
        if q != self.x.jump_q {
            self.x.jump_sel = 0;
        }
        self.x.jump_q = q;
        if let Some(k) = pick {
            let mut results = results;
            let (_, _, j) = results.swap_remove(k);
            self.x.jump_open = false;
            self.run_jump(j, act);
        } else if close {
            self.x.jump_open = false;
        }
        let _ = &mut close;
    }

    /// The Gallery: every photo, document, PDF, recording, story and note in the tree, with
    /// filters by kind and an Add menu.
    fn gallery_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        self.media_hub(ui, act, true);
    }

    fn memories_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        if self.x.gal_kind == 1 {
            self.x.gal_kind = 0;
        }
        self.media_hub(ui, act, false);
    }

    fn media_hub(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, photos: bool) {
        let sel_name = self.sel.map(|s| self.db.label(s)).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.heading(if photos { "Gallery" } else { "Memories" });
            ui.add_space(8.0);
            ui.add_enabled_ui(self.sel.is_some(), |ui| {
                ui.menu_button(RichText::new("+ Add").strong(), |ui| {
                    ui.label(RichText::new(format!("For {}", sel_name)).small().weak());
                    if ui.button(if photos { "Photos..." } else { "PDFs, documents or recordings..." }).clicked() {
                        *act = Some(Action::AttachFiles);
                        ui.close_menu();
                    }
                    if ui.button("Write a story...").clicked() {
                        if let Some(s) = self.sel {
                            self.x.story = Some(StoryEdit { who: s, idx: None, title: String::new(), text: String::new() });
                        }
                        ui.close_menu();
                    }
                    if ui.button("Record a spoken story...").on_hover_text("Recording is done from the person's Photos and recordings panel").clicked() {
                        self.view = View::Person;
                        if let Some(d) = self.settings.dock.iter_mut().find(|d| d.sec == "media") {
                            d.on = true;
                        }
                        ui.close_menu();
                    }
                    if ui.button("Write a note...").clicked() {
                        self.view = View::Person;
                        if let Some(d) = self.settings.dock.iter_mut().find(|d| d.sec == "notes") {
                            d.on = true;
                        }
                        *act = Some(Action::EditStart);
                        ui.close_menu();
                    }
                });
            });
            ui.separator();
            if !photos {
                for (k, l) in [(0u8, "All memories"), (4, "Stories"), (3, "Recordings"), (2, "Documents and PDFs"), (5, "Notes")] {
                    if ui.selectable_label(self.x.gal_kind == k, l).clicked() {
                        self.x.gal_kind = k;
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.gal_only, "Only the selected person");
            ui.add(egui::TextEdit::singleline(&mut self.gal_find).hint_text("Filter by name or caption").desired_width(220.0));
            ui.separator();
            if !self.x.gal_pick {
                if ui.button("Select...").on_hover_text("Click several items to tick them, then remove them in one go").clicked() {
                    self.x.gal_pick = true;
                    self.x.gal_sel.clear();
                }
            }
        });
        let q = self.gal_find.trim().to_lowercase();
        let want = if photos { 1 } else { self.x.gal_kind };
        // (owner, media index or usize::MAX for a note, kind, title, owner name)
        let mut items: Vec<(u32, usize, u8, String, String)> = Vec::new();
        for (id, p) in &self.db.people {
            for (i, m) in p.media.iter().enumerate() {
                if self.gal_only && self.sel != Some(*id) && !m.faces.iter().any(|f| Some(f.who) == self.sel) {
                    continue;
                }
                let kind = media_kind(m);
                if (want != 0 && want != kind) || (!photos && kind == 1) {
                    continue;
                }
                let title = if !m.caption.is_empty() { m.caption.clone() } else if !m.name.is_empty() { m.name.clone() } else { m.path.rsplit('/').next().unwrap_or("").to_string() };
                if !q.is_empty() && !p.name.to_lowercase().contains(&q) && !title.to_lowercase().contains(&q) {
                    continue;
                }
                items.push((*id, i, kind, title, p.name.clone()));
            }
            if !photos && (want == 0 || want == 5) && !p.notes.trim().is_empty() && (!self.gal_only || self.sel == Some(*id)) {
                let first: String = p.notes.trim().chars().take(140).collect();
                if q.is_empty() || p.name.to_lowercase().contains(&q) || p.notes.to_lowercase().contains(&q) {
                    items.push((*id, usize::MAX, 5, first, p.name.clone()));
                }
            }
        }
        items.sort_by(|a, b| a.4.cmp(&b.4).then(a.2.cmp(&b.2)).then(a.1.cmp(&b.1)));
        let counts = |k: u8| items.iter().filter(|x| x.2 == k).count();
        if photos {
            ui.label(RichText::new(format!("{} photo(s)", counts(1))).weak());
        } else {
            ui.label(RichText::new(format!("{} stor(ies), {} recording(s), {} document(s), {} note(s)", counts(4), counts(3), counts(2), counts(5))).weak());
        }
        ui.separator();
        if items.is_empty() {
            ui.label("Nothing here yet. Select a person and use + Add.");
            return;
        }
        if self.x.gal_pick {
            // only keep ticks for items still listed
            let shown: HashSet<(u32, usize)> = items.iter().filter(|x| x.1 != usize::MAX).map(|x| (x.0, x.1)).collect();
            self.x.gal_sel.retain(|k| shown.contains(k));
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} selected", self.x.gal_sel.len())).strong());
                if ui.button("Select all").clicked() {
                    self.x.gal_sel = shown.clone();
                }
                if ui.button("Select none").clicked() {
                    self.x.gal_sel.clear();
                }
                if ui.add_enabled(!self.x.gal_sel.is_empty(), egui::Button::new("Remove selected...")).clicked() {
                    self.x.gal_confirm = true;
                }
                if ui.button("Done").clicked() {
                    self.x.gal_pick = false;
                    self.x.gal_sel.clear();
                }
            });
            ui.add_space(4.0);
        }
        let mut pdf_req: Option<(PathBuf, String)> = None;
        let mut story_open: Option<(u32, usize)> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // A grid that fits the window: as many columns as fit, with the cards stretched to
            // fill each row, so nothing runs off the right edge.
            let gap = 12.0;
            let avail = (ui.available_width() - 4.0).max(130.0);
            let cols = (((avail + gap) / (170.0 + gap)).floor() as usize).max(1);
            let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).clamp(120.0, 260.0);
            for row in items.chunks(cols) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (id, i, kind, title, name) in row {
                        ui.vertical(|ui| {
                            ui.set_width(cw);
                            ui.set_max_width(cw);
                            let size = vec2(cw, (cw * 0.76).round());
                            if *i == usize::MAX {
                                // a note: show its first lines on a card
                                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                                ui.painter().rect(rect, 6.0_f32, ui.visuals().faint_bg_color, ui.visuals().widgets.noninteractive.bg_stroke);
                                let g = ui.painter().layout(title.clone(), FontId::proportional(12.0), ui.visuals().text_color(), rect.width() - 16.0);
                                ui.painter().with_clip_rect(rect.shrink(6.0)).galley(rect.min + vec2(8.0, 8.0), g, ui.visuals().text_color());
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    *act = Some(Action::Open(*id));
                                }
                                ui.label(RichText::new("Note").small().weak());
                            } else {
                                let m = &self.db.people[id].media[*i];
                                if *kind == 1 && online_photo(m) {
                                    let picked = self.x.gal_sel.contains(&(*id, *i));
                                    if ui.add_sized(size, egui::Button::new("Photo stored online\nClick to open in browser").selected(picked)).clicked() {
                                        if self.x.gal_pick {
                                            if !self.x.gal_sel.remove(&(*id, *i)) {
                                                self.x.gal_sel.insert((*id, *i));
                                            }
                                        } else {
                                            ui.ctx().open_url(egui::OpenUrl::new_tab(m.path.clone()));
                                        }
                                    }
                                } else if *kind == 1 && !m.thumb.is_empty() && m.thumb != "-" {
                                    let uri = img_uri(&mprev(&m.thumb));
                                    let r = ui.add(egui::Image::new(uri).fit_to_exact_size(size).rounding(4.0).sense(Sense::click()));
                                    if self.x.gal_sel.contains(&(*id, *i)) {
                                        ui.painter().rect_stroke(r.rect.expand(2.0), 5.0_f32, Stroke::new(3.0_f32, ui.visuals().selection.bg_fill));
                                    }
                                    if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        if self.x.gal_pick {
                                            // choosing several: a click ticks or unticks the photo
                                            if !self.x.gal_sel.remove(&(*id, *i)) {
                                                self.x.gal_sel.insert((*id, *i));
                                            }
                                        } else {
                                            self.viewing = Some((*id, *i));
                                        }
                                    }
                                } else {
                                    let (badge, hint) = match *kind {
                                        1 => ("PHOTO", "Open"),
                                        3 => ("\u{266A}", "Play"),
                                        4 => ("STORY", "Read"),
                                        _ if is_pdf(&m.file) => ("PDF", "Read"),
                                        _ => ("DOC", "Open"),
                                    };
                                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                                    let hov = resp.hovered();
                                    ui.painter().rect(rect, 6.0_f32, if hov { ui.visuals().widgets.hovered.bg_fill } else { ui.visuals().faint_bg_color }, ui.visuals().widgets.noninteractive.bg_stroke);
                                    ui.painter().text(rect.center() - vec2(0.0, 12.0), Align2::CENTER_CENTER, badge, FontId::proportional(28.0), ui.visuals().selection.bg_fill);
                                    let online = m.file.is_empty() && is_url(&m.path);
                                    ui.painter().text(rect.center() + vec2(0.0, 26.0), Align2::CENTER_CENTER, if online { "Open online" } else { hint }, FontId::proportional(12.0), ui.visuals().weak_text_color());
                                    if self.x.gal_sel.contains(&(*id, *i)) {
                                        ui.painter().rect_stroke(rect.expand(2.0), 7.0_f32, Stroke::new(3.0_f32, ui.visuals().selection.bg_fill));
                                    }
                                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        if self.x.gal_pick {
                                            if !self.x.gal_sel.remove(&(*id, *i)) {
                                                self.x.gal_sel.insert((*id, *i));
                                            }
                                        } else if online {
                                            ui.ctx().open_url(egui::OpenUrl::new_tab(m.path.clone()));
                                        } else if *kind == 4 && m.file.ends_with(".txt") {
                                            story_open = Some((*id, *i));
                                        } else if is_pdf(&m.file) {
                                            pdf_req = Some((mfile(&m.file), title.clone()));
                                        } else if *kind == 1 && m.thumb.is_empty() && !m.file.is_empty() {
                                            self.viewing = Some((*id, *i));
                                        } else if !m.file.is_empty() {
                                            open_media(&m.file);
                                        }
                                    }
                                }
                                if self.x.gal_pick {
                                    let key = (*id, *i);
                                    let mut on = self.x.gal_sel.contains(&key);
                                    if ui.checkbox(&mut on, RichText::new(title.as_str()).small()).changed() {
                                        if on {
                                            self.x.gal_sel.insert(key);
                                        } else {
                                            self.x.gal_sel.remove(&key);
                                        }
                                    }
                                } else {
                                    ui.add(egui::Label::new(RichText::new(title.as_str()).small()).truncate());
                                }
                            }
                            if ui.link(RichText::new(name.as_str()).small()).clicked() {
                                *act = Some(Action::Open(*id));
                            }
                        });
                    }
                });
                ui.add_space(10.0);
            }
        });
        if pdf_req.is_some() {
            self.x.pdf_req = pdf_req;
        }
        if self.x.gal_confirm {
            let n = self.x.gal_sel.len();
            let mut go = false;
            let mut cancel = false;
            egui::Window::new("Remove selected items").id(egui::Id::new("gal_remove")).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)).show(ui.ctx(), |ui| {
                ui.label(format!("Remove {} item(s) from the people they belong to?", n));
                ui.label(RichText::new("The files themselves stay in your media folder and backups. Undo (Ctrl+Z) brings them back.").small().weak());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Remove").strong()).clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if cancel {
                self.x.gal_confirm = false;
            }
            if go {
                self.x.gal_confirm = false;
                let removed = remove_media_items(&mut self.db, &self.x.gal_sel);
                self.x.gal_sel.clear();
                self.x.gal_pick = false;
                self.viewing = None;
                self.dirty = true;
                self.status = format!("Removed {} item(s). Undo (Ctrl+Z) brings them back.", removed);
            }
        }
        if let Some((who, i)) = story_open {
            if let Some(m) = self.db.people.get(&who).and_then(|p| p.media.get(i)) {
                let text = safe_media_path(&m.file).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
                self.x.story = Some(StoryEdit { who, idx: Some(i), title: m.caption.clone(), text });
            }
        }
    }

    /// Write or read a story. Stories are kept as plain text files in the media folder, so they
    /// are backed up and exported like any other document.
    fn story_window(&mut self, ctx: &egui::Context) {
        let Some(st) = self.x.story.as_mut() else { return };
        let mut open = true;
        let mut save = false;
        let who = self.db.people.get(&st.who).map(|p| p.name.clone()).unwrap_or_default();
        egui::Window::new(format!("Story about {}", who)).id(egui::Id::new("story_win")).open(&mut open).default_size([620.0, 520.0]).show(ctx, |ui| {
            ui.add(egui::TextEdit::singleline(&mut st.title).hint_text("Title, for example \"How Grandpa met Grandma\"").font(egui::TextStyle::Heading).desired_width(f32::INFINITY));
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut st.text).desired_rows(18).desired_width(f32::INFINITY).font(egui::TextStyle::Body));
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(!st.text.trim().is_empty(), egui::Button::new("Save story")).clicked() {
                    save = true;
                }
                ui.label(RichText::new("Saved as a text file with this person's photos and documents.").small().weak());
            });
        });
        if save {
            let st = self.x.story.take().unwrap();
            let title = if st.title.trim().is_empty() { "Story".to_string() } else { st.title.trim().to_string() };
            let file = match st.idx.and_then(|i| self.db.people.get(&st.who).and_then(|p| p.media.get(i)).map(|m| m.file.clone())) {
                Some(f) if !f.is_empty() => f,
                _ => format!("{}-{}.txt", stamp(), sanitize(&title)),
            };
            let Some(target) = safe_media_path(&file) else {
                self.status = "Could not save the story: its file name is not allowed.".to_string();
                return;
            };
            match atomic_write(&target, st.text.as_bytes()) {
                Ok(_) => {
                    if let Some(p) = self.db.people.get_mut(&st.who) {
                        match st.idx.and_then(|i| p.media.get_mut(i)) {
                            Some(m) => m.caption = title.clone(),
                            None => p.media.push(Media { file, name: format!("{}.txt", title), caption: title.clone(), form: "story".into(), ..Default::default() }),
                        }
                    }
                    self.dirty = true;
                    self.status = format!("Saved the story \"{}\".", title);
                }
                Err(e) => self.status = format!("Could not save the story: {}", e),
            }
        } else if !open {
            self.x.story = None;
        }
    }


    /// The actions for one person in a chart: open, view tree, quick edit, add a relative.
    fn person_menu(&self, ui: &mut egui::Ui, id: u32, act: &mut Option<Action>) {
        if ui.button("Open individual page").clicked() {
            *act = Some(Action::Open(id));
            ui.close_menu();
        }
        ui.menu_button("View their tree", |ui| {
            for (lbl, w) in [("Ancestors (pedigree)", 0u8), ("Descendants", 1), ("Family", 2)] {
                if ui.button(lbl).clicked() {
                    *act = Some(Action::ViewTree(id, w));
                    ui.close_menu();
                }
            }
        });
        if ui.button("Quick edit...").clicked() {
            *act = Some(Action::QuickEdit(id));
            ui.close_menu();
        }
        ui.menu_button("Add a relative", |ui| {
            let (f, m) = self.db.parents(id);
            let first_fam = self.db.fams_of(id).first().copied();
            let items: [(&str, Kind, bool); 6] = [
                ("Father...", Kind::Father, f.is_none()),
                ("Mother...", Kind::Mother, m.is_none()),
                ("Another set of parents...", Kind::NewParent(true), f.is_some() || m.is_some()),
                ("Brother or sister...", Kind::Sibling, true),
                ("Spouse or partner...", Kind::Spouse, true),
                ("Child...", Kind::Child(first_fam), true),
            ];
            for (lbl, k, on) in items {
                if ui.add_enabled(on, egui::Button::new(lbl)).clicked() {
                    *act = Some(Action::AddFor(id, k));
                    ui.close_menu();
                }
            }
        });
        let sets = self.db.parent_families(id);
        if sets.len() > 1 {
            ui.menu_button("Show which parents", |ui| {
                let shown = self.db.famc(id);
                for f in &sets {
                    let fm = &self.db.fams[f];
                    let bio = rel_is_bio(&fm.rel_to(id, true)) && rel_is_bio(&fm.rel_to(id, false));
                    let names: Vec<String> = [fm.h, fm.w].into_iter().flatten().filter(|x| self.db.people.contains_key(x)).map(|x| self.db.label(x)).collect();
                    let label = format!("{}{}: {}", if shown == Some(*f) { "* " } else { "" }, if bio { "Biological" } else { "Other" }, if names.is_empty() { "(no names)".to_string() } else { names.join(" & ") });
                    if ui.selectable_label(shown == Some(*f), label).clicked() {
                        *act = Some(Action::ShowParentsFor(id, *f));
                        ui.close_menu();
                    }
                }
            });
        }
        ui.separator();
        if ui.button("Centre chart on this person").clicked() {
            *act = Some(Action::Select(id));
            ui.close_menu();
        }
        if ui.button("Make this the home person").clicked() {
            *act = Some(Action::SetHome(id));
            ui.close_menu();
        }
        let marked = self.db.bookmarks.contains(&id);
        if ui.button(if marked { "Remove bookmark" } else { "Bookmark this person" }).clicked() {
            *act = Some(Action::Bookmark(id));
            ui.close_menu();
        }
    }

    /// Spouse(s) and children of one person, as a drop-down list.
    fn spouse_children_menu(&self, ui: &mut egui::Ui, id: u32, act: &mut Option<Action>) {
        let fams = self.db.fams_of(id);
        for fid in &fams {
            let fm = &self.db.fams[fid];
            let other = if fm.h == Some(id) { fm.w } else { fm.h };
            if let Some(o) = other.filter(|o| self.db.people.contains_key(o)) {
                if ui.button(format!("+ {}", self.db.label(o))).clicked() {
                    *act = Some(Action::Select(o));
                    ui.close_menu();
                }
            }
            for k in &fm.kids {
                if !self.db.people.contains_key(k) {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    if ui.button(format!("{}{}", self.db.label(*k), self.db.child_note(*fid, *k))).clicked() {
                        *act = Some(Action::Select(*k));
                        ui.close_menu();
                    }
                });
            }
        }
        if fams.is_empty() {
            ui.weak("No spouse or children recorded");
        }
        ui.separator();
        if ui.button("Add spouse...").clicked() {
            *act = Some(Action::AddFor(id, Kind::Spouse));
            ui.close_menu();
        }
        if ui.button("Add child...").clicked() {
            *act = Some(Action::AddFor(id, Kind::Child(fams.first().copied())));
            ui.close_menu();
        }
    }

    fn siblings_menu(&self, ui: &mut egui::Ui, id: u32, act: &mut Option<Action>) {
        let sibs = self.db.siblings(id);
        for k in &sibs {
            if ui.button(self.db.label(*k)).clicked() {
                *act = Some(Action::Select(*k));
                ui.close_menu();
            }
        }
        if sibs.is_empty() {
            ui.weak("No brothers or sisters recorded");
        }
        ui.separator();
        if ui.button("Add brother or sister...").clicked() {
            *act = Some(Action::AddFor(id, Kind::Sibling));
            ui.close_menu();
        }
    }

    /// Apply the Quick edit window to the tree.
    fn apply_quick(&mut self, q: &QuickEdit) {
        if let Some(p) = self.db.people.get_mut(&q.id) {
            p.given = q.given.trim().to_string();
            p.surname = q.surname.trim().to_string();
            p.name = compose_name(&p.given, &p.surname);
            p.sex = q.sex.clone();
            p.bd = q.bd.trim().to_string();
            p.bp = q.bp.trim().to_string();
            p.dd = q.dd.trim().to_string();
            p.dp = q.dp.trim().to_string();
            p.living = q.living;
        }
    }

    /// Where an arrow key goes from the selected person: right = father (Shift: mother),
    /// left = a child (the one you just came from, if any), up/down = previous/next sibling.
    fn nav_target(&self, key: egui::Key, shift: bool) -> Option<u32> {
        let sel = self.sel?;
        match key {
            egui::Key::ArrowRight => {
                let (f, m) = self.db.parents(sel);
                if shift { m.or(f) } else { f.or(m) }
            }
            egui::Key::ArrowLeft => {
                let kids = self.db.children_of(sel);
                match self.back.last() {
                    Some(b) if kids.contains(b) => Some(*b),
                    _ => kids.first().copied(),
                }
            }
            egui::Key::ArrowUp | egui::Key::ArrowDown => {
                let sibs = self.db.siblings(sel);
                if sibs.is_empty() {
                    return None;
                }
                let mut all = sibs;
                if let Some(f) = self.db.famc(sel) {
                    // keep the family's own order, with the selected person in place
                    let own = &self.db.fams[&f].kids;
                    all.retain(|x| !own.contains(x));
                    let mut ordered: Vec<u32> = own.iter().copied().filter(|x| self.db.people.contains_key(x)).collect();
                    ordered.extend(all);
                    all = ordered;
                } else {
                    all.push(sel);
                }
                let pos = all.iter().position(|x| *x == sel)?;
                if key == egui::Key::ArrowUp {
                    pos.checked_sub(1).map(|p| all[p])
                } else {
                    all.get(pos + 1).copied()
                }
            }
            _ => None,
        }
    }

    /// The people you have been looking at, oldest first, ending with the current one.
    fn trail(&self) -> Vec<u32> {
        let mut seen: Vec<u32> = Vec::new();
        let cur = self.sel;
        for p in self.back.iter().rev() {
            if Some(*p) != cur && self.db.people.contains_key(p) && !seen.contains(p) {
                seen.push(*p);
                if seen.len() >= 6 {
                    break;
                }
            }
        }
        seen.reverse();
        if let Some(c) = cur {
            seen.push(c);
        }
        seen
    }

    fn trail_bar(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let t = self.trail();
        if t.len() < 2 {
            return;
        }
        let accent = Color32::from_rgb(self.settings.accent[0], self.settings.accent[1], self.settings.accent[2]);
        let short = |s: &str| -> String {
            let c: Vec<char> = s.chars().collect();
            if c.len() > 22 { format!("{}...", c[..21].iter().collect::<String>()) } else { s.to_string() }
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Trail").small().weak());
            let last = t.len() - 1;
            for (n, id) in t.iter().enumerate() {
                let name = short(&display_name(&self.db.people[id]));
                if n == last {
                    ui.label(RichText::new(name).strong().color(accent));
                } else {
                    if ui.link(name).on_hover_text("Go back to this person").clicked() {
                        *act = Some(Action::Select(*id));
                    }
                    ui.label(RichText::new(">").weak());
                }
            }
        });
    }

    /// Back, forward, home, and one-click moves to parents, siblings, partners and children.
    fn nav_bar(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let Some(sel) = self.sel else { return };
        let pick = |ui: &mut egui::Ui, act: &mut Option<Action>, ids: &[u32]| {
            for k in ids {
                if ui.button(self.db.label(*k)).clicked() {
                    *act = Some(Action::Select(*k));
                    ui.close_menu();
                }
            }
        };
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!self.back.is_empty(), egui::Button::new("< Back")).clicked() {
                *act = Some(Action::Back);
            }
            if ui.add_enabled(!self.fwd.is_empty(), egui::Button::new("Forward >")).clicked() {
                *act = Some(Action::Forward);
            }
            let hb = ui.button("Home").on_hover_text("Go to the home person. Right-click to make the current person the home person.");
            if hb.clicked() {
                *act = Some(Action::GoHome);
            }
            hb.context_menu(|ui| {
                if ui.button("Make the current person the home person").clicked() {
                    *act = Some(Action::SetHome(sel));
                    ui.close_menu();
                }
            });
            let mut recent: Vec<u32> = Vec::new();
            for k in self.back.iter().rev() {
                if *k != sel && self.db.people.contains_key(k) && !recent.contains(k) {
                    recent.push(*k);
                }
                if recent.len() >= 15 {
                    break;
                }
            }
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                ui.menu_button("Recent", |ui| pick(ui, act, &recent));
            });
            let marked = self.db.bookmarks.contains(&sel);
            ui.menu_button(if marked { "Bookmarks \u{2605}" } else { "Bookmarks" }, |ui| {
                if ui.button(if marked { "Remove bookmark for this person" } else { "Bookmark this person" }).clicked() {
                    *act = Some(Action::Bookmark(sel));
                    ui.close_menu();
                }
                let list: Vec<u32> = self.db.bookmarks.iter().copied().filter(|k| self.db.people.contains_key(k)).collect();
                if !list.is_empty() {
                    ui.separator();
                    pick(ui, act, &list);
                }
            });
            ui.separator();
            let sets = self.db.parent_families(sel);
            ui.add_enabled_ui(!sets.is_empty(), |ui| {
                ui.menu_button("Parents", |ui| {
                    for f in &sets {
                        let fm = &self.db.fams[f];
                        let mut ids: Vec<u32> = Vec::new();
                        for p in fm.h.iter().chain(fm.w.iter()) {
                            if self.db.people.contains_key(p) {
                                ids.push(*p);
                            }
                        }
                        if sets.len() > 1 {
                            let shown = self.db.famc(sel) == Some(*f);
                            let bio = rel_is_bio(&fm.rel_to(sel, true)) && rel_is_bio(&fm.rel_to(sel, false));
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(if bio { "Biological parents" } else { "Other parents" }).small().weak());
                                if shown {
                                    ui.label(RichText::new("shown on chart").small().weak());
                                } else if ui.small_button("Show on chart").clicked() {
                                    *act = Some(Action::ShowParents(*f));
                                    ui.close_menu();
                                }
                            });
                        }
                        pick(ui, act, &ids);
                    }
                });
            });
            let sibs = self.db.siblings(sel);
            ui.add_enabled_ui(!sibs.is_empty(), |ui| {
                ui.menu_button("Siblings", |ui| pick(ui, act, &sibs));
            });
            let pt = self.db.partners(sel);
            ui.add_enabled_ui(!pt.is_empty(), |ui| {
                ui.menu_button("Partners", |ui| pick(ui, act, &pt));
            });
            let kids = self.db.children_of(sel);
            ui.add_enabled_ui(!kids.is_empty(), |ui| {
                ui.menu_button("Children", |ui| pick(ui, act, &kids));
            });
        });
    }

    fn goto(&mut self, i: u32) {
        if self.sel != Some(i) {
            if let Some(c) = self.sel {
                self.back.push(c);
            }
            self.fwd.clear();
        }
        self.sel = Some(i);
    }

    fn delete(&mut self) {
        let Some(id) = self.sel else { return };
        self.db.people.remove(&id);
        for f in self.db.fams.values_mut() {
            if f.h == Some(id) { f.h = None; }
            if f.w == Some(id) { f.w = None; }
            f.kids.retain(|k| *k != id);
            f.more.retain(|(_, x)| *x != id);
            f.pedi.remove(&id);
            f.frel.remove(&id);
            f.mrel.remove(&id);
        }
        replace_pointer(&mut self.db, &format!("I{}", id), None);
        self.db.fams.retain(|_, f| f.h.is_some() || f.w.is_some() || !f.kids.is_empty() || !f.more.is_empty());
        self.sel = self.db.people.keys().next().copied();
        self.confirm = false;
    }

    fn pedigree(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let sel = self.sel.unwrap();
        let gens = self.settings.gens.clamp(3, 8);
        let rows = 1usize << (gens - 1);
        let z = self.settings.zoom;
        let (bh, pitch, gap, top) = (28.0f32 * z, 36.0f32 * z, 28.0f32 * z, 30.0f32 * z);
        let th = rows as f32 * pitch + top;
        let bw = ((ui.available_width() - (gens as f32 - 1.0) * gap - 24.0) / gens as f32).clamp(150.0, 340.0) * z;
        let tw = gens as f32 * bw + (gens as f32 - 1.0) * gap;
        if self.fit_req {
            let av = ui.available_size();
            let bw0 = bw / z;
            let zw = (av.x - 24.0) / (gens as f32 * bw0 + (gens as f32 - 1.0) * 28.0);
            let zh = (av.y - 100.0) / (rows as f32 * 36.0 + 36.0);
            *act = Some(Action::SetZoom(zw.min(zh).clamp(0.25, 1.8)));
        }
        let ix = self.db.index();
        let mut slots: Vec<Vec<Option<u32>>> = vec![vec![Some(sel)]];
        for g in 1..gens {
            let cur = {
                let prev = &slots[g - 1];
                let mut c = Vec::new();
                for s in prev {
                    let (f, m) = match s {
                        Some(i) => self.db.parents_ix(&ix, *i),
                        None => (None, None),
                    };
                    c.push(f);
                    c.push(m);
                }
                c
            };
            slots.push(cur);
        }
        let kids: Vec<(u32, String)> = self
            .db
            .fams_of(sel)
            .iter()
            .flat_map(|f| self.db.fams[f].kids.iter().map(|k| (*k, self.db.label(*k))).collect::<Vec<_>>())
            .collect();
        self.nav_bar(ui, act);
        ui.horizontal(|ui| {
            ui.label("Generations:");
            if ui.add_enabled(gens > 3, egui::Button::new("-")).clicked() {
                *act = Some(Action::SetGens(gens - 1));
            }
            ui.label(gens.to_string());
            if ui.add_enabled(gens < 8, egui::Button::new("+")).clicked() {
                *act = Some(Action::SetGens(gens + 1));
            }
            ui.separator();
            zoom_controls(ui, z, act);
            ui.separator();
            hand_switch(ui, self.settings.hand, act);
            ui.separator();
            if ui.button("Print").clicked() {
                *act = Some(Action::PrintPedigree);
            }
        });
        egui::ScrollArea::both().show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(vec2(tw + 12.0, th + 6.0), Sense::hover());
            let o = resp.rect.min + vec2(6.0, 0.0);
            if resp.hovered() {
                let zd = ui.input(|i| i.zoom_delta());
                if (zd - 1.0).abs() > 0.001 {
                    *act = Some(Action::SetZoom((z * zd).clamp(0.25, 1.8)));
                }
            }
            let dark = ui.visuals().dark_mode;
            let bg = self.settings.bg;
            let rgb = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
            let (line_c, edge_c, box_c, hover_c, ink_c, head_c) = if dark {
                (shift(bg, 60), shift(bg, 38), shift(bg, 12), shift(bg, 24), rgb(230, 232, 235), shift(bg, 120))
            } else {
                (rgb(150, 155, 162), rgb(184, 188, 194), Color32::WHITE, rgb(245, 247, 250), rgb(31, 35, 40), rgb(90, 96, 105))
            };
            let accent = Color32::from_rgb(self.settings.accent[0], self.settings.accent[1], self.settings.accent[2]);
            let black = Stroke::new(1.0_f32, line_c);
            let edge = Stroke::new(1.0_f32, edge_c);
            for g in 0..gens {
                painter.text(
                    pos2(o.x + g as f32 * (bw + gap), o.y + 4.0),
                    Align2::LEFT_TOP,
                    ordinal(g + 1),
                    FontId::proportional(17.0 * z),
                    head_c,
                );
            }
            for g in 0..gens {
                let n = 1usize << g;
                let step = rows as f32 * pitch / n as f32;
                for i in 0..n {
                    let cy = o.y + top + (i as f32 + 0.5) * step;
                    let x = o.x + g as f32 * (bw + gap);
                    let rect = Rect::from_min_size(pos2(x, cy - bh / 2.0), vec2(bw, bh));
                    if g > 0 {
                        let ccy = o.y + top + ((i / 2) as f32 + 0.5) * step * 2.0;
                        painter.add(egui::Shape::line(
                            vec![pos2(x, cy), pos2(x - gap / 2.0, cy), pos2(x - gap / 2.0, ccy), pos2(x - gap, ccy)],
                            black,
                        ));
                    }
                    match slots[g][i] {
                        Some(id) => {
                            let p = &self.db.people[&id];
                            let r2 = ui.interact(rect, egui::Id::new(("ped", g, i)), Sense::click());
                            let is_sel = g == 0;
                            let is_hl = self.hl == Some(id) && !is_sel;
                            let fill = if is_sel {
                                accent
                            } else if is_hl {
                                accent.gamma_multiply(0.35)
                            } else if r2.hovered() {
                                hover_c
                            } else {
                                box_c
                            };
                            let ink = if is_sel { on_accent(accent) } else { ink_c };
                            painter.rect(rect, 5.0_f32, fill, edge);
                            if is_sel {
                                painter.rect_stroke(rect.expand(2.5), 7.0_f32, Stroke::new(2.5_f32, ink_c));
                            }
                            let l = life(p);
                            let mut txt = if l.is_empty() { styled_name(p) } else { format!("{}  {}", styled_name(p), l) };
                            if self.settings.box_places {
                                let pl = if !p.bp.trim().is_empty() { p.bp.trim() } else { p.dp.trim() };
                                if let Some(first) = pl.split(',').next().filter(|x| !x.trim().is_empty()) {
                                    txt.push_str(&format!("  \u{b7} {}", first.trim()));
                                }
                            }
                            if self.settings.box_ids {
                                txt.push_str(&format!("  #{}", id));
                            }
                            let ir = Rect::from_min_size(pos2(rect.min.x + 3.0, rect.min.y + 2.0), vec2(bh - 4.0, bh - 4.0));
                            match self.portrait_src(p).filter(|_| self.settings.box_photos) {
                                Some((uri, fr)) => put_photo(ui, ir, &uri, fr, 3.0),
                                None => {
                                    painter.rect_filled(ir, 3.0_f32, if is_sel { on_accent(accent).gamma_multiply(0.22) } else { hover_c });
                                    let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
                                    painter.text(ir.center(), Align2::CENTER_CENTER, initial, FontId::proportional(14.0 * z), if is_sel { ink } else { head_c });
                                }
                            }
                            let (pf0, pm0) = self.db.parents_ix(&ix, id);
                            let has_arrow = (g == gens - 1 && (pf0.is_some() || pm0.is_some())) || (g == 0 && !kids.is_empty());
                            let right_pad = 34.0 + if has_arrow { 24.0 } else { 0.0 };
                            let clip = Rect::from_min_max(rect.min + vec2(2.0, 2.0), pos2((rect.max.x - right_pad).max(rect.min.x + bh + 20.0), rect.max.y - 2.0));
                            painter.with_clip_rect(clip).text(
                                pos2(rect.min.x + bh + 4.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                txt,
                                FontId::proportional(14.0 * z),
                                ink,
                            );
                            let mz = Rect::from_min_size(pos2(rect.max.x - 32.0, rect.min.y + 3.0), vec2(30.0, bh - 6.0));
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(mz), |ui| {
                                ui.menu_button(RichText::new("...").small(), |ui| self.person_menu(ui, id, act));
                            });
                            if g == gens - 1 {
                                let (pf, pm) = (pf0, pm0);
                                if pf.is_some() || pm.is_some() {
                                    let zone = Rect::from_min_size(pos2(rect.max.x - 56.0, rect.min.y), vec2(24.0, bh));
                                    let ra = ui.interact(zone, egui::Id::new(("arrow", i)), Sense::click());
                                    let c = if ra.hovered() { accent } else { head_c };
                                    let (cx, cyy) = (zone.center().x, zone.center().y);
                                    painter.add(egui::Shape::convex_polygon(
                                        vec![pos2(cx - 3.0, cyy - 6.0), pos2(cx - 3.0, cyy + 6.0), pos2(cx + 5.0, cyy)],
                                        c,
                                        Stroke::NONE,
                                    ));
                                    if ra.on_hover_text("Show this person's ancestors").clicked() {
                                        *act = Some(Action::Select(id));
                                    }
                                }
                            }
                            if g == 0 && !kids.is_empty() {
                                let zone = Rect::from_min_size(pos2(rect.max.x - 56.0, rect.min.y), vec2(24.0, bh));
                                let ra = ui.interact(zone, egui::Id::new("arrow_down"), Sense::click());
                                let c = if ra.hovered() { ink } else { on_accent(accent).gamma_multiply(0.75) };
                                let (cx, cyy) = (zone.center().x, zone.center().y);
                                painter.add(egui::Shape::convex_polygon(
                                    vec![pos2(cx + 3.0, cyy - 6.0), pos2(cx + 3.0, cyy + 6.0), pos2(cx - 5.0, cyy)],
                                    c,
                                    Stroke::NONE,
                                ));
                                let pid = egui::Id::new("kids_popup");
                                let ra = ra.on_hover_text(if kids.len() == 1 { "Go down to this person's child" } else { "Go down to one of this person's children" });
                                if ra.clicked() {
                                    if kids.len() == 1 {
                                        *act = Some(Action::Select(kids[0].0));
                                    } else {
                                        ui.memory_mut(|mm| mm.toggle_popup(pid));
                                    }
                                }
                                egui::popup_below_widget(ui, pid, &ra, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                                    ui.set_min_width(220.0);
                                    for (k, l) in &kids {
                                        if ui.button(l.as_str()).clicked() {
                                            *act = Some(Action::Select(*k));
                                        }
                                    }
                                });
                            }
                            if is_hl {
                                painter.rect_stroke(rect, 5.0_f32, Stroke::new(3.0_f32, accent));
                            }
                            r2.context_menu(|ui| self.person_menu(ui, id, act));
                            if is_sel {
                                let w = rect.width().min(230.0);
                                let n_sc = self.db.partners(id).len() + kids.len();
                                let n_sib = self.db.siblings(id).len();
                                let r_a = Rect::from_min_size(pos2(rect.min.x, rect.max.y + 8.0), vec2(w, 22.0));
                                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r_a), |ui| {
                                    ui.menu_button(format!("Spouse & children ({})", n_sc), |ui| self.spouse_children_menu(ui, id, act));
                                });
                                let r_b = Rect::from_min_size(pos2(rect.min.x, rect.max.y + 36.0), vec2(w, 22.0));
                                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r_b), |ui| {
                                    ui.menu_button(format!("Siblings ({})", n_sib), |ui| self.siblings_menu(ui, id, act));
                                });
                            }
                            if r2.double_clicked() {
                                *act = Some(Action::Open(id));
                            } else if r2.clicked() {
                                *act = Some(Action::Highlight(id));
                            }
                        }
                        None => {
                            let child = if g > 0 { slots[g - 1][i / 2] } else { None };
                            if let Some(child) = child {
                                let r2 = ui.interact(rect, egui::Id::new(("ped", g, i)), Sense::click());
                                let fill = if r2.hovered() { hover_c } else { box_c.gamma_multiply(0.45) };
                                painter.rect(rect, 5.0_f32, fill, edge);
                                if r2.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    let label = if i % 2 == 0 { "+ Add father" } else { "+ Add mother" };
                                    painter.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(13.0), head_c);
                                }
                                if r2.clicked() {
                                    let k = if i % 2 == 0 { Kind::Father } else { Kind::Mother };
                                    *act = Some(Action::AddFor(child, k));
                                }
                            } else {
                                painter.rect(rect, 5.0_f32, box_c.gamma_multiply(0.45), edge);
                            }
                        }
                    }
                }
            }
            // hand tool: a layer over the whole chart that takes every click and drag, so dragging
            // anywhere moves the chart and nothing is selected by accident
            if self.settings.hand {
                hand_overlay(ui, resp.rect, "ped");
            }
        });
    }

    /// A compact, clickable person card: portrait (or initial), name, years and birthplace.
    fn mini_card(&self, ui: &mut egui::Ui, id: u32, tag: &str, current: bool, act: &mut Option<Action>) {
        let Some(p) = self.db.people.get(&id) else { return };
        let (_, edge, box_c, hover_c, ink_c, head_c, accent) = self.chart_colors(ui);
        let w = ui.available_width();
        let frame = egui::Frame::none().fill(box_c).stroke(edge).rounding(6.0).inner_margin(egui::Margin::symmetric(8.0, 6.0));
        let out = frame.show(ui, |ui| {
            ui.set_width((w - 18.0).max(40.0));
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                match self.portrait_src(p) {
                    Some((uri, fr)) => put_photo(ui, r, &uri, fr, 5.0),
                    None => {
                        ui.painter().rect_filled(r, 5.0_f32, hover_c);
                        let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, initial, FontId::proportional(18.0), head_c);
                    }
                }
                ui.vertical(|ui| {
                    if !tag.is_empty() {
                        ui.label(RichText::new(tag).small().weak());
                    }
                    ui.add(egui::Label::new(RichText::new(display_name(p)).strong().color(ink_c)).truncate());
                    let l = life(p);
                    if !l.is_empty() {
                        ui.label(RichText::new(l).weak());
                    }
                    if !p.bp.is_empty() {
                        ui.add(egui::Label::new(RichText::new(p.bp.as_str()).small().weak()).truncate());
                    }
                });
            });
        });
        let rect = out.response.rect;
        let r = ui.interact(rect, out.response.id.with("click"), Sense::click());
        if current {
            ui.painter().rect_stroke(rect, 6.0_f32, Stroke::new(2.0_f32, accent));
        } else if r.hovered() {
            ui.painter().rect_stroke(rect, 6.0_f32, Stroke::new(1.5_f32, accent.gamma_multiply(0.8)));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if r.clicked() && !current {
            *act = Some(Action::Select(id));
        }
    }

    /// The couple, the marriage and the children of one family: the left side of the Family tab.
    fn family_main(&self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, ids: &[u32], fx: usize) {
        let fid = ids[fx];
        let f = &self.db.fams[&fid];
        if ids.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                for (i, fid2) in ids.iter().enumerate() {
                    let fm = &self.db.fams[fid2];
                    let other = if fm.h == Some(id) { fm.w } else { fm.h };
                    let mut label = other.and_then(|o| self.db.people.get(&o)).map(|p| p.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Family {}", i + 1));
                    let y = year(&fm.md);
                    if !y.is_empty() {
                        label.push_str(&format!(" ({})", y));
                    }
                    if ui.selectable_label(i == fx, label).clicked() {
                        *act = Some(Action::Fx(i));
                    }
                }
            });
            ui.add_space(4.0);
        }
        ui.columns(2, |cols| {
            for (n, (slot, tag)) in [(f.h, "Husband"), (f.w, "Wife")].into_iter().enumerate() {
                match slot {
                    Some(pid) => self.mini_card(&mut cols[n], pid, tag, pid == id, act),
                    None => {
                        cols[n].group(|ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(tag).small().weak());
                            ui.weak("not recorded");
                        });
                    }
                }
            }
        });
        ui.add_space(4.0);
        let mut m = Vec::new();
        if !f.kind.is_empty() {
            m.push(f.kind.clone());
        }
        if !f.md.is_empty() {
            m.push(f.md.clone());
        }
        if !f.mp.is_empty() {
            m.push(f.mp.clone());
        }
        ui.vertical_centered(|ui| {
            if m.is_empty() {
                ui.weak("Marriage details not recorded");
            } else {
                ui.label(m.join("  -  "));
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new(format!("Children ({})", f.kids.len())).strong());
        if f.kids.is_empty() {
            ui.weak("No children recorded");
        } else {
            egui::Grid::new("kids").num_columns(4).spacing([22.0, 5.0]).show(ui, |ui| {
                ui.label(RichText::new("Name").small().weak());
                ui.label(RichText::new("Born").small().weak());
                ui.label(RichText::new("Died").small().weak());
                ui.label(RichText::new("Parent link").small().weak());
                ui.end_row();
                for k in &f.kids {
                    let Some(c) = self.db.people.get(k) else { continue };
                    ui.horizontal(|ui| {
                        if ui.link(display_name(c)).clicked() {
                            *act = Some(Action::Select(*k));
                        }
                        let n = self.db.children_of(*k).len();
                        if n > 0 {
                            ui.weak(format!("{} {}", n, if n == 1 { "child" } else { "children" }));
                        }
                    });
                    ui.label(if c.bp.is_empty() { c.bd.clone() } else { format!("{}  {}", c.bd, c.bp).trim().to_string() });
                    ui.label(c.dd.as_str());
                    let note = self.db.child_note(fid, *k);
                    ui.label(if note.is_empty() { "Birth".to_string() } else { note.trim().trim_matches(|c| c == '(' || c == ')').to_string() });
                    ui.end_row();
                }
            });
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Add child").clicked() {
                *act = Some(Action::Add(Kind::Child(Some(fid))));
            }
            if ui.button("Print family group sheet").clicked() {
                *act = Some(Action::PrintFamily);
            }
        });
    }

    /// Parents, siblings, partners and other relationships of the selected person: the right side.
    fn family_side(&self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, fx: usize) {
        ui.label(RichText::new("Relationships").strong().size(16.0));
        ui.add_space(4.0);
        let sets = self.db.parent_families(id);
        if sets.is_empty() {
            ui.weak("No parents recorded");
            ui.horizontal(|ui| {
                if ui.small_button("Add father").clicked() {
                    *act = Some(Action::Add(Kind::Father));
                }
                if ui.small_button("Add mother").clicked() {
                    *act = Some(Action::Add(Kind::Mother));
                }
            });
        }
        for f in &sets {
            let fm = &self.db.fams[f];
            for (slot, father) in [(fm.h, true), (fm.w, false)] {
                let Some(pid) = slot else { continue };
                let rel = fm.rel_to(id, father);
                let role = if father { "father" } else { "mother" };
                let tag = if sets.len() == 1 && rel_is_bio(&rel) {
                    if father { "Father".to_string() } else { "Mother".to_string() }
                } else if rel_is_bio(&rel) {
                    format!("Biological {}", role)
                } else {
                    format!("{} {}", rel_name(&rel), role)
                };
                self.mini_card(ui, pid, &tag, false, act);
                ui.add_space(3.0);
            }
        }
        let sibs = self.db.siblings(id);
        if !sibs.is_empty() {
            egui::CollapsingHeader::new(format!("Siblings ({})", sibs.len())).id_salt("fam_sibs").default_open(false).show(ui, |ui| {
                let mine = self.db.parent_families(id);
                for sid in &sibs {
                    let half = !mine.iter().any(|f| self.db.fams[f].kids.contains(sid) && self.db.fams[f].h.is_some() && self.db.fams[f].w.is_some());
                    let Some(p) = self.db.people.get(sid) else { continue };
                    ui.horizontal(|ui| {
                        if ui.link(display_name(p)).clicked() {
                            *act = Some(Action::Select(*sid));
                        }
                        let mut t = life(p);
                        if half {
                            t = format!("{} half", t).trim().to_string();
                        }
                        ui.weak(t);
                    });
                }
            });
        }
        ui.add_space(6.0);
        ui.label(RichText::new("Partners and children").strong());
        let fams = self.db.fams_of(id);
        if fams.is_empty() {
            ui.weak("No partner recorded");
            if ui.small_button("Add spouse").clicked() {
                *act = Some(Action::Add(Kind::Spouse));
            }
        }
        for (i, fid) in fams.iter().enumerate() {
            let fm = &self.db.fams[fid];
            let other = if fm.h == Some(id) { fm.w } else { fm.h };
            let y = year(&fm.md);
            let mut tag = if fm.kind.is_empty() { "Partner".to_string() } else { fm.kind.clone() };
            if !y.is_empty() {
                tag.push_str(&format!(" {}", y));
            }
            match other {
                Some(o) => self.mini_card(ui, o, &tag, false, act),
                None => {
                    ui.weak("Partner not recorded");
                }
            }
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    for k in &fm.kids {
                        let Some(c) = self.db.people.get(k) else { continue };
                        ui.horizontal(|ui| {
                            if ui.link(display_name(c)).clicked() {
                                *act = Some(Action::Select(*k));
                            }
                            ui.weak(life(c));
                        });
                    }
                    if fams.len() > 1 && i != fx && ui.small_button("Show this family").clicked() {
                        *act = Some(Action::Fx(i));
                    }
                });
            });
            ui.add_space(4.0);
        }
        let assoc: Vec<(u32, String)> = self.db.people.get(&id).map(|p| p.assoc.iter().map(|a| (a.other, a.role.clone())).collect()).unwrap_or_default();
        let assoc_of = self.db.kin_extras(id).assoc_of;
        if !assoc.is_empty() || !assoc_of.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new("Other relationships").strong());
            for (o, role) in &assoc {
                if let Some(p) = self.db.people.get(o) {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(role.as_str());
                        if ui.link(display_name(p)).clicked() {
                            *act = Some(Action::Select(*o));
                        }
                    });
                }
            }
            for (o, role) in &assoc_of {
                if let Some(p) = self.db.people.get(o) {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(format!("is the {} of", role));
                        if ui.link(display_name(p)).clicked() {
                            *act = Some(Action::Select(*o));
                        }
                    });
                }
            }
        }
    }

    fn family_view(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let id = self.sel.unwrap();
        self.nav_bar(ui, act);
        let mut ids = self.db.fams_of(id);
        if ids.is_empty() {
            if let Some(f) = self.db.famc(id) {
                ids.push(f);
            }
        }
        let fx = self.fx.min(ids.len().saturating_sub(1));
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let avail = ui.available_width();
            let side_w = 300.0f32;
            if avail > 860.0 {
                ui.horizontal_top(|ui| {
                    let main_w = avail - side_w - 24.0;
                    ui.allocate_ui_with_layout(vec2(main_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(main_w);
                        if ids.is_empty() {
                            self.family_empty(ui, act);
                        } else {
                            self.family_main(ui, act, id, &ids, fx);
                        }
                    });
                    ui.add_space(8.0);
                    ui.allocate_ui_with_layout(vec2(side_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(side_w);
                        self.family_side(ui, act, id, fx);
                    });
                });
            } else {
                if ids.is_empty() {
                    self.family_empty(ui, act);
                } else {
                    self.family_main(ui, act, id, &ids, fx);
                }
                ui.add_space(10.0);
                ui.separator();
                self.family_side(ui, act, id, fx);
            }
        });
    }

    fn family_empty(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        ui.label("No family group yet.");
        ui.horizontal(|ui| {
            if ui.button("Add spouse").clicked() {
                *act = Some(Action::Add(Kind::Spouse));
            }
            if ui.button("Add child").clicked() {
                *act = Some(Action::Add(Kind::Child(None)));
            }
        });
    }

    /// One block of the Individual page. The big blocks live in the dock on the right.
    /// The Individual page while it is locked: plain, readable text instead of greyed-out boxes.
    fn person_read(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, sec: &str) {
        let Some(p) = self.db.people.get(&id).cloned() else { return };
        let mut pdf_req: Option<(PathBuf, String)> = None;
        // A value laid out at a set width (wrapping only text longer than that), so inside a grid
        // it keeps its width; a plain wrapping label there shrinks to one word per line.
        let val = |ui: &mut egui::Ui, t: &str| {
            let w = (ui.ctx().screen_rect().width() * 0.42).clamp(260.0, 720.0);
            let g = ui.fonts(|f| f.layout(t.to_string(), FontId::proportional(15.0), ui.visuals().text_color(), w));
            ui.add(egui::Label::new(g));
        };
        let lab = |ui: &mut egui::Ui, t: &str| {
            ui.label(RichText::new(t).size(14.0).weak());
        };
        let both = |d: &str, pl: &str| -> String {
            match (d.trim().is_empty(), pl.trim().is_empty()) {
                (true, true) => String::new(),
                (false, true) => show_date(d),
                (true, false) => format!("in {}", pl.trim()),
                (false, false) => format!("{}  -  {}", show_date(d), pl.trim()),
            }
        };
        let none = |ui: &mut egui::Ui, t: &str| {
            ui.label(RichText::new(t).italics().weak());
        };
        match sec {
            "facts" => {
                ui.horizontal(|ui| {
                    let k = match self.settings.photo_size { 0 => 0.75, 2 => 1.45, 3 => 2.0, _ => 1.0 };
                    let sz = vec2(110.0 * k, 130.0 * k);
                    match self.portrait_src(&p) {
                        Some((uri, fr)) => {
                            let (r, _) = ui.allocate_exact_size(sz, Sense::hover());
                            put_photo(ui, r, &uri, fr, 6.0);
                        }
                        None => {
                            let (r, _) = ui.allocate_exact_size(sz, Sense::hover());
                            ui.painter().rect_filled(r, 6.0, ui.visuals().faint_bg_color);
                            ui.painter().rect_stroke(r, 6.0, ui.visuals().widgets.noninteractive.bg_stroke);
                            let ini = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, ini, FontId::proportional(48.0), ui.visuals().weak_text_color());
                        }
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        let mut full = String::new();
                        for part in [p.prefix.trim(), p.name.trim(), p.suffix.trim()] {
                            if !part.is_empty() {
                                if !full.is_empty() {
                                    full.push(' ');
                                }
                                full.push_str(part);
                            }
                        }
                        if full.is_empty() {
                            full = "(no name)".into();
                        }
                        ui.label(RichText::new(full).size(26.0).strong());
                        let l = life(&p);
                        let sex = match p.sex.as_str() {
                            "M" => "Male",
                            "F" => "Female",
                            _ => "Sex not recorded",
                        };
                        let mut sub = sex.to_string();
                        if !l.is_empty() {
                            sub = format!("{}   |   {}", l, sub);
                        }
                        if p.living {
                            sub.push_str("   |   Living");
                        }
                        ui.label(RichText::new(sub).size(16.0));
                        if !p.nick.trim().is_empty() {
                            ui.label(RichText::new(format!("Known as \"{}\"", p.nick.trim())).size(15.0).italics());
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Person ID #{}", id)).small().weak());
                            if self.settings.home == Some(id) {
                                ui.label(RichText::new("Home person").small().weak());
                            }
                        });
                    });
                });
                ui.add_space(10.0);
                ui.label(RichText::new("Vital facts").size(17.0).strong());
                ui.separator();
                let cf = |t: &str| p.fx.get(t).map(|e| e.conf.clone()).unwrap_or_default();
                let rows: Vec<(&str, String, String)> = vec![
                    ("Born", both(&p.bd, &p.bp), cf("BIRT")),
                    (if p.chtag == "BAPM" { "Baptised" } else { "Christened" }, both(&p.chd, &p.chp), cf("CHR")),
                    ("Died", both(&p.dd, &p.dp), cf("DEAT")),
                    ("Buried", both(&p.bud, &p.bup), cf("BURI")),
                    ("Maiden name", p.maiden.trim().to_string(), String::new()),
                ];
                let shown: Vec<_> = rows.into_iter().filter(|r| !r.1.is_empty()).collect();
                if shown.is_empty() {
                    none(ui, "No dates or places yet. Press Edit to add them.");
                } else {
                    egui::Grid::new("read_facts").num_columns(3).spacing([24.0, 8.0]).show(ui, |ui| {
                        for (k, v, c) in &shown {
                            lab(ui, k);
                            val(ui, v);
                            conf_badge(ui, c);
                            ui.end_row();
                        }
                    });
                }
            }
            "timeline" => {
                let tl = self.db.timeline(id);
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Timeline").size(17.0).strong());
                    if ui.checkbox(&mut self.settings.show_history, "with historical events").changed() {
                        save_settings(&self.settings);
                    }
                });
                ui.separator();
                if tl.is_empty() {
                    none(ui, "Add dates to see a timeline.");
                } else {
                    // the person's own events, with historical events slotted in by year
                    let mut rows: Vec<(i32, bool, String, String)> = tl.iter().enumerate().map(|(i, (d, t))| (parse_date(d).0.unwrap_or(i32::MIN + i as i32), false, d.clone(), t.clone())).collect();
                    if self.settings.show_history {
                        for (y, t) in self.history_for(&p) {
                            rows.push((y, true, String::new(), t));
                        }
                    }
                    rows.sort_by_key(|r| (r.0, r.1));
                    egui::Grid::new("read_tl").num_columns(2).striped(true).spacing([24.0, 6.0]).show(ui, |ui| {
                        for (_, hist, d, t) in &rows {
                            if *hist {
                                ui.label("");
                                ui.add(egui::Label::new(RichText::new(t.as_str()).italics().weak()).wrap());
                            } else {
                                ui.label(RichText::new(if d.is_empty() { "-".to_string() } else { show_date(d) }).size(14.0).strong());
                                val(ui, t);
                            }
                            ui.end_row();
                        }
                    });
                }
            }
            "relationships" => {
                let link = |ui: &mut egui::Ui, act: &mut Option<Action>, who: u32, extra: &str| {
                    ui.horizontal(|ui| {
                        if ui.link(RichText::new(self.db.label(who)).size(15.0)).clicked() {
                            *act = Some(Action::Select(who));
                        }
                        if !extra.is_empty() {
                            ui.label(RichText::new(extra).small().weak());
                        }
                    });
                };
                let pf = self.db.parent_families(id);
                lab(ui, if pf.len() > 1 { "Parents (more than one set)" } else { "Parents" });
                if pf.is_empty() {
                    none(ui, "None recorded");
                }
                for (n, fid) in pf.iter().enumerate() {
                    let f = self.db.fams[fid].clone();
                    if pf.len() > 1 {
                        ui.label(RichText::new(format!("Set {}", n + 1)).small().weak());
                    }
                    for (who, father) in [(f.h, true), (f.w, false)] {
                        if let Some(w) = who.filter(|w| self.db.people.contains_key(w)) {
                            let r = f.rel_to(id, father);
                            let extra = if rel_is_bio(&r) { String::new() } else { format!("({})", r.to_lowercase()) };
                            link(ui, act, w, &extra);
                        }
                    }
                }
                ui.add_space(6.0);
                let fams = self.db.fams_of(id);
                lab(ui, "Spouses and children");
                if fams.is_empty() {
                    none(ui, "None recorded");
                }
                for fid in &fams {
                    let f = self.db.fams[fid].clone();
                    let other = if f.h == Some(id) { f.w } else { f.h };
                    let m = both(&f.md, &f.mp);
                    match other.filter(|o| self.db.people.contains_key(o)) {
                        Some(o) => link(ui, act, o, &if m.is_empty() { String::new() } else { format!("married {}", m) }),
                        None => {
                            ui.label(RichText::new("Unknown partner").italics());
                        }
                    }
                    for k in &f.kids {
                        if self.db.people.contains_key(k) {
                            ui.horizontal(|ui| {
                                ui.add_space(18.0);
                                link(ui, act, *k, self.db.child_note(*fid, *k).trim());
                            });
                        }
                    }
                }
                let sibs = self.db.siblings(id);
                if !sibs.is_empty() {
                    ui.add_space(6.0);
                    lab(ui, "Brothers and sisters");
                    for s2 in sibs {
                        link(ui, act, s2, "");
                    }
                }
                if !p.assoc.is_empty() {
                    ui.add_space(6.0);
                    lab(ui, "Other ties");
                    for a in &p.assoc {
                        if self.db.people.contains_key(&a.other) {
                            link(ui, act, a.other, &format!("({})", a.role));
                        }
                    }
                }
            }
            "events" => {
                if p.events.is_empty() {
                    none(ui, "No other events.");
                } else {
                    egui::Grid::new("read_ev").num_columns(2).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                        for e in &p.events {
                            ui.label(RichText::new(if e.kind.is_empty() { "Other" } else { e.kind.as_str() }).size(14.0).strong());
                            ui.vertical(|ui| {
                                let mut line = both(&e.date, &e.place);
                                if !e.val.trim().is_empty() {
                                    line = if line.is_empty() { e.val.trim().to_string() } else { format!("{}  -  {}", e.val.trim(), line) };
                                }
                                val(ui, if line.is_empty() { "-" } else { line.as_str() });
                                if !e.note.trim().is_empty() {
                                    ui.add(egui::Label::new(RichText::new(e.note.trim()).small().weak()).wrap());
                                }
                                if !e.cites.is_empty() {
                                    ui.label(RichText::new(format!("{} source(s)", e.cites.len())).small().weak());
                                }
                                conf_badge(ui, &e.conf);
                            });
                            ui.end_row();
                        }
                    });
                }
            }
            "sources" => {
                if p.sources.is_empty() {
                    none(ui, "No sources yet.");
                }
                for c in &p.sources {
                    ui.vertical(|ui| {
                        let title = if c.text.trim().is_empty() { "(untitled source)" } else { c.text.trim() };
                        val(ui, title);
                        let mut bits: Vec<String> = Vec::new();
                        if !c.about.trim().is_empty() {
                            bits.push(format!("for {}", c.about.trim()));
                        }
                        if !c.page.trim().is_empty() {
                            bits.push(format!("page {}", c.page.trim()));
                        }
                        if !c.quality.trim().is_empty() {
                            bits.push(format!("quality {}", c.quality.trim()));
                        }
                        if !bits.is_empty() {
                            ui.label(RichText::new(bits.join("  |  ")).small().weak());
                        }
                    });
                    ui.add_space(4.0);
                }
            }
            "notes" => {
                let mut any = false;
                if !p.notes.trim().is_empty() {
                    any = true;
                    val(ui, p.notes.trim());
                }
                for nid in &p.note_refs {
                    if let Some(n) = self.db.notes.get(nid) {
                        if !n.text.trim().is_empty() {
                            any = true;
                            ui.add_space(4.0);
                            val(ui, n.text.trim());
                        }
                    }
                }
                if !any {
                    none(ui, "No notes.");
                }
            }
            "names" => {
                if p.alt_names.is_empty() {
                    none(ui, "No other names.");
                }
                egui::Grid::new("read_names").num_columns(2).spacing([18.0, 6.0]).show(ui, |ui| {
                    for a in &p.alt_names {
                        lab(ui, if a.kind.is_empty() { "Other name" } else { a.kind.as_str() });
                        val(ui, &compose_name(&a.given, &a.surname));
                        ui.end_row();
                    }
                });
            }
            "media" => {
                let photos: Vec<(usize, &Media)> = p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file)).collect();
                let others: Vec<(usize, &Media)> = p.media.iter().enumerate().filter(|(_, m)| !is_image(&m.file)).collect();
                if photos.is_empty() && others.is_empty() {
                    none(ui, "No photos, documents or recordings.");
                }
                ui.horizontal_wrapped(|ui| {
                    for (i, m) in &photos {
                        ui.vertical(|ui| {
                            ui.set_width(120.0);
                            if !m.thumb.is_empty() && m.thumb != "-" {
                                let uri = img_uri(&mprev(&m.thumb));
                                let r = ui.add(egui::Image::new(uri).fit_to_exact_size(vec2(120.0, 90.0)).rounding(4.0).sense(Sense::click()));
                                if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    self.viewing = Some((id, *i));
                                }
                            } else if ui.add_sized([120.0, 90.0], egui::Button::new("Open")).clicked() {
                                if m.thumb.is_empty() {
                                    self.viewing = Some((id, *i));
                                } else {
                                    open_media(&m.file);
                                }
                            }
                            let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                            ui.label(RichText::new(cap).small());
                        });
                    }
                });
                self.tagged_strip(ui, id);
                for (_, m) in &others {
                    ui.horizontal(|ui| {
                        let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                        val(ui, cap);
                        if !m.file.is_empty() && ui.small_button(if is_audio(&m.file) { "Play" } else if is_pdf(&m.file) { "Read" } else { "Open" }).clicked() {
                            if is_pdf(&m.file) {
                                pdf_req = Some((mfile(&m.file), cap.to_string()));
                            } else {
                                open_media(&m.file);
                            }
                        }
                        if m.file.is_empty() && is_url(&m.path) {
                            ui.hyperlink_to("Open online", m.path.as_str());
                        }
                    });
                }
            }
            "more" => {
                none(ui, "Press Edit to merge or delete this person.");
            }
            "todo" => self.todo_panel(ui, id),
            "journal" => self.journal_panel(ui, id),
            other => {
                ui.add_enabled_ui(false, |ui| self.person_section(ui, act, id, other));
            }
        }
        if pdf_req.is_some() {
            self.x.pdf_req = pdf_req;
        }
    }

    fn person_section(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, sec: &str) {
        let mut pdf_req: Option<(PathBuf, String)> = None;
        self.person_section_inner(ui, act, id, sec, &mut pdf_req);
        if pdf_req.is_some() {
            self.x.pdf_req = pdf_req;
        }
    }

    fn person_section_inner(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, sec: &str, pdf_req: &mut Option<(PathBuf, String)>) {
        match sec {
            "facts" => {
                let home = self.settings.home;
                let mut tomb_req = false;
            {
                let p = self.db.people.get_mut(&id).unwrap();
                egui::Grid::new("pg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Given name(s)");
                    let r1 = ui.add(egui::TextEdit::singleline(&mut p.given).desired_width(320.0));
                    ui.end_row();
                    ui.label("Surname");
                    let r2 = ui.add(egui::TextEdit::singleline(&mut p.surname).desired_width(320.0));
                    ui.end_row();
                    if r1.changed() || r2.changed() {
                        p.name = compose_name(&p.given, &p.surname);
                    }
                    ui.label("Maiden name");
                    ui.add(egui::TextEdit::singleline(&mut p.maiden).hint_text("if different from surname").desired_width(320.0));
                    ui.end_row();
                    ui.label("Title / suffix / nickname");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut p.prefix).hint_text("Dr., Rev.").desired_width(90.0));
                        ui.add(egui::TextEdit::singleline(&mut p.suffix).hint_text("Jr., III").desired_width(90.0));
                        ui.add(egui::TextEdit::singleline(&mut p.nick).hint_text("nickname").desired_width(120.0));
                    });
                    ui.end_row();
                    ui.label("Sex");
                    let st = match p.sex.as_str() {
                        "M" => "Male",
                        "F" => "Female",
                        _ => "Unknown",
                    };
                    egui::ComboBox::from_id_salt("sex").selected_text(st).show_ui(ui, |ui| {
                        ui.selectable_value(&mut p.sex, "M".to_string(), "Male");
                        ui.selectable_value(&mut p.sex, "F".to_string(), "Female");
                        ui.selectable_value(&mut p.sex, String::new(), "Unknown");
                    });
                    ui.end_row();
                    ui.label("Person ID");
                    ui.label(RichText::new(format!("#{}", id)).weak());
                    ui.end_row();
                    if home == Some(id) {
                        ui.label("Home person");
                        ui.label("This is the home person (change it in Tools > Home person...)");
                        ui.end_row();
                    }
                    ui.label("Living");
                    ui.checkbox(&mut p.living, "Living (kept private when printing or exporting)");
                    ui.end_row();
                    ui.label("Born");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut p.bd).hint_text("e.g. 12 Mar 1921, about 1850").desired_width(320.0));
                        let mut c = p.fx.get("BIRT").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "BIRT"), &mut c) {
                            p.fx.entry("BIRT".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Birthplace");
                    ui.add(egui::TextEdit::singleline(&mut p.bp).desired_width(320.0));
                    ui.end_row();
                    ui.label("Christened");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut p.chd).desired_width(320.0));
                        let mut c = p.fx.get("CHR").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "CHR"), &mut c) {
                            p.fx.entry("CHR".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Christening place");
                    ui.add(egui::TextEdit::singleline(&mut p.chp).desired_width(320.0));
                    ui.end_row();
                    ui.label("Died");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut p.dd).desired_width(320.0));
                        let mut c = p.fx.get("DEAT").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "DEAT"), &mut c) {
                            p.fx.entry("DEAT".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Place of death");
                    ui.add(egui::TextEdit::singleline(&mut p.dp).desired_width(320.0));
                    ui.end_row();
                    ui.label("Buried");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut p.bud).desired_width(320.0));
                        let mut c = p.fx.get("BURI").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "BURI"), &mut c) {
                            p.fx.entry("BURI".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Burial place");
                    ui.add(egui::TextEdit::singleline(&mut p.bup).desired_width(320.0));
                    ui.end_row();
                    if !p.dd.trim().is_empty() || !p.bud.trim().is_empty() {
                        ui.label("");
                        if ui.small_button("Calculate birth from gravestone age...").on_hover_text("Tombstone calculator: work out the birth date from the age at death").clicked() {
                            tomb_req = true;
                        }
                        ui.end_row();
                    }
                });
            }
            if tomb_req {
                self.open_tombstone(Some(id));
            }
            }
            "relationships" => {
        let (fa, mo) = self.db.parents(id);
        let fl = fa.map(|i| (i, self.db.label(i)));
        let ml = mo.map(|i| (i, self.db.label(i)));
        let fams: Vec<(u32, Option<(u32, String)>, Vec<(u32, String)>)> = self
            .db
            .fams_of(id)
            .into_iter()
            .map(|fid| {
                let f = &self.db.fams[&fid];
                let sp = if f.h == Some(id) { f.w } else { f.h };
                (fid, sp.map(|s| (s, self.db.label(s))), f.kids.iter().map(|k| (*k, format!("{}{}", self.db.label(*k), self.db.child_note(fid, *k)))).collect())
            })
            .collect();
            ui.heading("Parents");
            let pfams: Vec<(u32, Option<(u32, String)>, Option<(u32, String)>, String, String)> = self
                .db
                .parent_families(id)
                .into_iter()
                .map(|fid| {
                    let f = &self.db.fams[&fid];
                    (fid, f.h.map(|i| (i, self.db.label(i))), f.w.map(|i| (i, self.db.label(i))), f.rel_to(id, true), f.rel_to(id, false))
                })
                .collect();
            let shown_fam = self.db.famc(id);
            if pfams.is_empty() {
                for (role, link, kind) in [("Father", &fl, Kind::Father), ("Mother", &ml, Kind::Mother)] {
                    ui.horizontal(|ui| {
                        ui.label(role);
                        match link {
                            Some((i, l)) => {
                                if ui.link(l.as_str()).clicked() {
                                    *act = Some(Action::Select(*i));
                                }
                            }
                            None => {
                                if ui.button(format!("Add {}", role.to_lowercase())).clicked() {
                                    *act = Some(Action::Add(kind));
                                }
                                if ui.button("Link existing person").clicked() {
                                    self.link = Some(LinkReq { child: id, fid: None, father: kind == Kind::Father, q: String::new() });
                                }
                            }
                        }
                    });
                }
            }
            let many = pfams.len() > 1;
            for (n, (fid, h, w, frel, mrel)) in pfams.iter().enumerate() {
                ui.group(|ui| {
                    let bio = rel_is_bio(frel) && rel_is_bio(mrel);
                    let mut title = if many { format!("Set of parents {}", n + 1) } else { "Parents".to_string() };
                    title.push_str(if bio { " - biological" } else { " - not both biological" });
                    if many && shown_fam == Some(*fid) {
                        title.push_str(" (shown on the chart)");
                    }
                    ui.label(RichText::new(title).strong());
                    for (role, slot, cur, father) in [("Father", h, frel, true), ("Mother", w, mrel, false)] {
                        ui.horizontal(|ui| {
                            ui.label(role);
                            match slot {
                                Some((i, l)) => {
                                    if ui.link(l.as_str()).clicked() {
                                        *act = Some(Action::Select(*i));
                                    }
                                    if let Some(newv) = rel_combo(ui, ("prel", *fid, father), cur) {
                                        if let Some(f) = self.db.fams.get_mut(fid) {
                                            let m = if father { &mut f.frel } else { &mut f.mrel };
                                            if newv.is_empty() { m.remove(&id); } else { m.insert(id, newv); }
                                        }
                                    }
                                }
                                None => {
                                    if ui.button(format!("Add {}", role.to_lowercase())).clicked() {
                                        *act = Some(Action::Add(Kind::ParentIn(*fid, father)));
                                    }
                                    if ui.button("Link existing person").clicked() {
                                        self.link = Some(LinkReq { child: id, fid: Some(*fid), father, q: String::new() });
                                    }
                                }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        if many && shown_fam != Some(*fid) && ui.button("Show these parents on the chart").clicked() {
                            if let Some(p) = self.db.people.get_mut(&id) {
                                p.pref_parents = Some(*fid);
                            }
                        }
                        if many && ui.button("Remove from this family").on_hover_text("The people stay; only this person's link to this set of parents is removed.").clicked() {
                            self.db.remove_from_family(id, *fid);
                        }
                    });
                });
            }
            if !pfams.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Another set of parents (adoptive, step, foster, guardian...):");
                    if ui.button("Add father").clicked() {
                        *act = Some(Action::Add(Kind::NewParent(true)));
                    }
                    if ui.button("Add mother").clicked() {
                        *act = Some(Action::Add(Kind::NewParent(false)));
                    }
                    if ui.button("Link existing person").clicked() {
                        self.link = Some(LinkReq { child: id, fid: None, father: true, q: String::new() });
                    }
                });
                ui.label(RichText::new("Choose how each parent is related (biological, adopted, step, foster, guardian) in the drop-downs above. Biological and legal parents are kept as separate sets.").small().weak());
            }
            ui.add_space(10.0);
            ui.heading("Family groups");
            for (fid, sp, kids) in &fams {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Spouse");
                        match sp {
                            Some((s, l)) => {
                                if ui.link(l.as_str()).clicked() {
                                    *act = Some(Action::Select(*s));
                                }
                            }
                            None => {
                                ui.weak("not recorded");
                            }
                        }
                    });
                    if let Some(f) = self.db.fams.get_mut(fid) {
                        egui::Grid::new(("mg", *fid)).num_columns(2).show(ui, |ui| {
                            ui.label("Married");
                            ui.text_edit_singleline(&mut f.md);
                            ui.end_row();
                            ui.label("Place");
                            ui.text_edit_singleline(&mut f.mp);
                            ui.end_row();
                            ui.label("Partnership");
                            let shown = if f.kind.is_empty() { "Not stated" } else { f.kind.as_str() };
                            egui::ComboBox::from_id_salt(("pkind", *fid)).selected_text(shown).show_ui(ui, |ui| {
                                if ui.selectable_label(f.kind.is_empty(), "Not stated").clicked() {
                                    f.kind.clear();
                                }
                                for k in PARTNER_KINDS {
                                    if ui.selectable_label(f.kind == *k, *k).clicked() {
                                        f.kind = k.to_string();
                                    }
                                }
                            });
                            ui.end_row();
                        });
                    }
                    ui.label("Children");
                    for (k, l) in kids {
                        ui.horizontal(|ui| {
                            if ui.link(l.as_str()).clicked() {
                                *act = Some(Action::Select(*k));
                            }
                            if let Some(f) = self.db.fams.get_mut(fid) {
                                let cur = f.pedi.get(k).cloned().unwrap_or_default();
                                let shown = if cur.is_empty() { "birth child".to_string() } else { format!("{} child", cur) };
                                egui::ComboBox::from_id_salt(("pedi", *fid, *k)).selected_text(shown).width(110.0).show_ui(ui, |ui| {
                                    for (v, lbl) in [("", "birth child"), ("adopted", "adopted child"), ("foster", "foster child"), ("sealing", "sealed child")] {
                                        if ui.selectable_label(cur == v, lbl).clicked() {
                                            if v.is_empty() {
                                                f.pedi.remove(k);
                                            } else {
                                                f.pedi.insert(*k, v.to_string());
                                            }
                                        }
                                    }
                                });
                                for (father, who) in [(true, if f.h.is_some() { "father" } else { "" }), (false, if f.w.is_some() { "mother" } else { "" })] {
                                    if who.is_empty() {
                                        continue;
                                    }
                                    let cur = f.rel_to(*k, father);
                                    ui.label(RichText::new(who).small().weak());
                                    if let Some(nv) = rel_combo(ui, ("crel", *fid, *k, father), &cur) {
                                        let m = if father { &mut f.frel } else { &mut f.mrel };
                                        if nv.is_empty() { m.remove(k); } else { m.insert(*k, nv); }
                                    }
                                }
                            }
                        });
                    }
                    if ui.button("Add child").clicked() {
                        *act = Some(Action::Add(Kind::Child(Some(*fid))));
                    }
                    if let Some(f) = self.db.fams.get_mut(fid) {
                        egui::CollapsingHeader::new("Marriage details, family events and notes").id_salt(("fmore", *fid)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Kind of marriage");
                                ui.add(egui::TextEdit::singleline(&mut f.mx.typ).hint_text("civil, religious...").desired_width(160.0));
                            });
                            ui.label("Note about the marriage");
                            ui.add(egui::TextEdit::multiline(&mut f.mx.note).desired_rows(2).desired_width(f32::INFINITY));
                            ui.label("Sources for the marriage");
                            cites_section(ui, ("msrc", *fid), &mut f.mx.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                            ui.label("Sources for this family as a whole");
                            cites_section(ui, ("fsrc", *fid), &mut f.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                            let mut remove: Option<usize> = None;
                            for (i, e) in f.events.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt(("fev", *fid, i)).selected_text(if e.kind.is_empty() { "Other" } else { e.kind.as_str() }).width(140.0).show_ui(ui, |ui| {
                                        for k in FAM_KINDS.iter() {
                                            ui.selectable_value(&mut e.kind, k.to_string(), *k);
                                        }
                                    });
                                    ui.add(egui::TextEdit::singleline(&mut e.date).hint_text("date").desired_width(100.0));
                                    ui.add(egui::TextEdit::singleline(&mut e.place).hint_text("place").desired_width(130.0));
                                    ui.add(egui::TextEdit::singleline(&mut e.note).hint_text("note").desired_width(130.0));
                                    if ui.button("Remove").clicked() {
                                        remove = Some(i);
                                    }
                                });
                                cites_section(ui, ("fevsrc", *fid, i), &mut e.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                            }
                            if let Some(i) = remove {
                                f.events.remove(i);
                            }
                            if ui.button("Add family event").clicked() {
                                f.events.push(Event { kind: "Divorce".to_string(), ..Default::default() });
                            }
                            ui.label("Notes about this family");
                            ui.add(egui::TextEdit::multiline(&mut f.notes).desired_rows(3).desired_width(f32::INFINITY));
                            for nid in f.note_refs.clone() {
                                if let Some(n) = self.db.notes.get_mut(&nid) {
                                    ui.label(RichText::new("Shared note").small().weak());
                                    ui.add(egui::TextEdit::multiline(&mut n.text).desired_rows(2).desired_width(f32::INFINITY));
                                }
                            }
                        });
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button("Add spouse").clicked() {
                    *act = Some(Action::Add(Kind::Spouse));
                }
                if fams.is_empty() && ui.button("Add child").clicked() {
                    *act = Some(Action::Add(Kind::Child(None)));
                }
            });
            ui.add_space(10.0);
            ui.heading("Step and half family");
            {
                let kin = self.db.kin_extras(id);
                let lines: [(&str, &Vec<u32>); 4] = [("Step-parents", &kin.step_parents), ("Half-siblings", &kin.half_siblings), ("Step-siblings", &kin.step_siblings), ("Step-children", &kin.step_children)];
                let mut any = false;
                for (lbl, ids) in lines {
                    if ids.is_empty() {
                        continue;
                    }
                    any = true;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(lbl).strong());
                        for i in ids {
                            if let Some(p) = self.db.people.get(i) {
                                if ui.link(display_name(p)).clicked() {
                                    *act = Some(Action::Select(*i));
                                }
                            }
                        }
                    });
                }
                if !any {
                    ui.weak("None found. These are worked out from the families above, so nothing needs to be entered.");
                }
            }
            ui.add_space(10.0);
            ui.heading("Other relationships");
            ui.label(RichText::new("Guardians, godparents, foster carers and anyone else connected to this person who is not a parent or spouse.").small().weak());
            {
                let kin_of = self.db.kin_extras(id).assoc_of;
                for (o, role) in &kin_of {
                    if let Some(p) = self.db.people.get(o) {
                        ui.horizontal(|ui| {
                            ui.label("Is the");
                            ui.label(RichText::new(role.as_str()).strong());
                            ui.label("of");
                            if ui.link(display_name(p)).clicked() {
                                *act = Some(Action::Select(*o));
                            }
                        });
                    }
                }
                let names: BTreeMap<u32, String> = self.db.people.iter().map(|(k, p)| (*k, display_name(p))).collect();
                let mut remove: Option<usize> = None;
                if let Some(p) = self.db.people.get_mut(&id) {
                    for (i, a) in p.assoc.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label("Has");
                            if ui.link(names.get(&a.other).cloned().unwrap_or_else(|| "(missing)".into())).clicked() {
                                *act = Some(Action::Select(a.other));
                            }
                            ui.label("as");
                            egui::ComboBox::from_id_salt(("arole", i)).selected_text(a.role.clone()).width(120.0).show_ui(ui, |ui| {
                                for r in ASSOC_ROLES {
                                    ui.selectable_value(&mut a.role, r.to_string(), *r);
                                }
                            });
                            ui.add(egui::TextEdit::singleline(&mut a.role).desired_width(100.0));
                            ui.add(egui::TextEdit::singleline(&mut a.note).hint_text("note").desired_width(160.0));
                            if ui.button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        p.assoc.remove(i);
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Add:");
                    egui::ComboBox::from_id_salt("newarole").selected_text(self.assoc_role.clone()).width(120.0).show_ui(ui, |ui| {
                        for r in ASSOC_ROLES {
                            ui.selectable_value(&mut self.assoc_role, r.to_string(), *r);
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut self.assoc_q).hint_text("search for the person").desired_width(180.0));
                });
                if !self.assoc_q.trim().is_empty() {
                    let cands: Vec<(u32, String)> = self.db.sorted(&self.assoc_q).into_iter().filter(|i| *i != id).take(6).map(|i| (i, self.db.label(i))).collect();
                    for (i, lbl) in cands {
                        if ui.link(format!("+ {}", lbl)).clicked() {
                            let role = self.assoc_role.clone();
                            if let Some(p) = self.db.people.get_mut(&id) {
                                p.assoc.push(Assoc { other: i, role, note: String::new() });
                            }
                            self.assoc_q.clear();
                        }
                    }
                }
            }
            }
            "names" => {
            {
                let p = self.db.people.get_mut(&id).unwrap();
                let mut remove: Option<usize> = None;
                for (i, a) in p.alt_names.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let kinds = ["", "birth", "married", "aka", "religious", "immigrant"];
                        let shown = if a.kind.is_empty() { "other name".to_string() } else { a.kind.clone() };
                        egui::ComboBox::from_id_salt(("alt", i)).selected_text(shown).width(100.0).show_ui(ui, |ui| {
                            for k in kinds {
                                ui.selectable_value(&mut a.kind, k.to_string(), if k.is_empty() { "other name" } else { k });
                            }
                        });
                        ui.add(egui::TextEdit::singleline(&mut a.given).hint_text("given names").desired_width(150.0));
                        ui.add(egui::TextEdit::singleline(&mut a.surname).hint_text("surname").desired_width(130.0));
                        if ui.button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    p.alt_names.remove(i);
                }
                if ui.button("Add another name").clicked() {
                    p.alt_names.push(AltName { kind: "aka".to_string(), ..Default::default() });
                }
            }
            }
            "notes" => {
            {
                let p = self.db.people.get_mut(&id).unwrap();
                ui.add(egui::TextEdit::multiline(&mut p.notes).desired_rows(6).desired_width(f32::INFINITY));
            }
            {
                let refs: Vec<u32> = self.db.people.get(&id).map(|p| p.note_refs.clone()).unwrap_or_default();
                for nid in refs {
                    let uses = self.db.people.values().filter(|q| q.note_refs.contains(&nid)).count() + self.db.fams.values().filter(|f| f.note_refs.contains(&nid)).count();
                    if let Some(n) = self.db.notes.get_mut(&nid) {
                        ui.label(RichText::new(if uses > 1 { format!("Shared note (also used by {} other record(s); editing it changes all of them)", uses - 1) } else { "Linked note".to_string() }).small().weak());
                        ui.add(egui::TextEdit::multiline(&mut n.text).desired_rows(3).desired_width(f32::INFINITY));
                    }
                }
            }
            }
            "events" => {
            {
                let p = self.db.people.get_mut(&id).unwrap();
                let mut remove: Option<usize> = None;
                for (i, e) in p.events.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(("ev", i)).selected_text(if e.kind.is_empty() { "Other" } else { e.kind.as_str() }).width(140.0).show_ui(ui, |ui| {
                            for k in EVENT_KINDS.iter() {
                                ui.selectable_value(&mut e.kind, k.to_string(), *k);
                            }
                        });
                        ui.add(egui::TextEdit::singleline(&mut e.date).hint_text("date").desired_width(110.0));
                        ui.add(egui::TextEdit::singleline(&mut e.place).hint_text("place").desired_width(150.0));
                        ui.add(egui::TextEdit::singleline(&mut e.val).hint_text("details").desired_width(150.0));
                        conf_combo(ui, ("evcf", i), &mut e.conf);
                        if ui.button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.add(egui::TextEdit::singleline(&mut e.note).hint_text("note").desired_width(260.0));
                        if e.kind.starts_with("LDS") {
                            ui.add(egui::TextEdit::singleline(&mut e.temp).hint_text("temple").desired_width(70.0));
                            ui.add(egui::TextEdit::singleline(&mut e.stat).hint_text("status").desired_width(100.0));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            cites_section(ui, ("evsrc", i), &mut e.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                        });
                    });
                }
                if let Some(i) = remove {
                    p.events.remove(i);
                }
                if ui.button("Add event").clicked() {
                    p.events.push(Event { kind: "Residence".to_string(), ..Default::default() });
                }
            }
            }
            "timeline" => {
                let tl = self.db.timeline(id);
            egui::CollapsingHeader::new("Timeline").default_open(false).show(ui, |ui| {
                if tl.is_empty() {
                    ui.label("Add dates to see a timeline.");
                }
                for (d, t) in &tl {
                    ui.label(format!("{}   {}", if d.is_empty() { "-" } else { d.as_str() }, t));
                }
            });
            }
            "sources" => {
            ui.label(RichText::new("Pick a source you have used before, or add a new one. A source can back up many people and facts.").small().weak());
            {
                let p = self.db.people.get_mut(&id).unwrap();
                cite_list(ui, "psrc", &mut p.sources, Some(&FACTS), "General", &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
            }
            }
            "media" => {
            ui.heading("Photos");
            ui.horizontal(|ui| {
                if ui.button("Add photos or documents...").clicked() {
                    *act = Some(Action::AttachFiles);
                }
                if ui.button("Open media folder").on_hover_text(format!("{}", media_dir().display())).clicked() {
                    open_path(&media_dir());
                }
                match &self.rec {
                    Some(r) => {
                        let secs = r.started.elapsed().as_secs();
                        if ui.button(format!("Stop recording ({}:{:02})", secs / 60, secs % 60)).clicked() {
                            *act = Some(Action::StopRec);
                        }
                        let level = (r.peak.swap(0, Ordering::Relaxed) as f32 / 32768.0).sqrt().clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(level).desired_width(140.0).text("mic level"));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    None => {
                        if ui.button("Record a story").clicked() {
                            *act = Some(Action::StartRec);
                        }
                        let cur = if self.settings.mic.is_empty() { "System default".to_string() } else { self.settings.mic.clone() };
                        egui::ComboBox::from_id_salt("mic").selected_text(cur).show_ui(ui, |ui| {
                            if ui.selectable_label(self.settings.mic.is_empty(), "System default").clicked() {
                                self.settings.mic.clear();
                                save_settings(&self.settings);
                            }
                            for n in input_device_names() {
                                if ui.selectable_label(self.settings.mic == n, n.as_str()).clicked() {
                                    self.settings.mic = n.clone();
                                    save_settings(&self.settings);
                                }
                            }
                        });
                    }
                }
            });
            let photos: Vec<(usize, Media)> = match self.db.people.get(&id) {
                Some(p) => p.media.iter().cloned().enumerate().filter(|(_, m)| is_image(&m.file)).collect(),
                None => Vec::new(),
            };
            if photos.is_empty() {
                ui.label(RichText::new("No photos yet. Click Add photos or documents.").weak());
            }
            ui.horizontal_wrapped(|ui| {
                for (i, m) in &photos {
                    ui.vertical(|ui| {
                        ui.set_width(150.0);
                        if m.thumb.is_empty() || m.thumb == "-" {
                            let label = if m.thumb == "-" { "No preview\nClick to open" } else { "Loading..." };
                            if ui.add_sized([150.0, 110.0], egui::Button::new(label)).clicked() {
                                // still making its preview: open it in our own viewer like any other photo;
                                // only a photo that cannot be previewed at all goes to the computer's own app
                                if m.thumb.is_empty() {
                                    self.viewing = Some((id, *i));
                                } else {
                                    open_media(&m.file);
                                }
                            }
                        } else {
                            let uri = img_uri(&mprev(&m.thumb));
                            let r = ui.add(egui::Image::new(uri).fit_to_exact_size(vec2(150.0, 110.0)).rounding(4.0).sense(Sense::click()));
                            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                self.viewing = Some((id, *i));
                            }
                        }
                        let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                        ui.label(RichText::new(cap).small());
                    });
                }
            });
            self.tagged_strip(ui, id);
            let others: Vec<(usize, Media)> = match self.db.people.get(&id) {
                Some(p) => p.media.iter().cloned().enumerate().filter(|(_, m)| !is_image(&m.file)).collect(),
                None => Vec::new(),
            };
            if !others.is_empty() {
                ui.add_space(8.0);
                ui.heading("Documents and recordings");
                let mut remove: Option<usize> = None;
                for (i, m) in &others {
                    ui.group(|ui| {
                        let full = mfile(&m.file);
                        ui.vertical(|ui| {
                            if let Some(mm) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(*i)) {
                                ui.add(egui::TextEdit::singleline(&mut mm.caption).hint_text("Caption").desired_width(280.0));
                            }
                            ui.label(RichText::new(m.name.as_str()).small().weak());
                            if m.file.is_empty() && is_url(&m.path) {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Stored on a website, not on this computer.").small().weak());
                                    ui.hyperlink_to("Open online", m.path.as_str());
                                });
                            } else if m.file.is_empty() {
                                ui.label(RichText::new(format!("File not found. The GEDCOM file named it: {}", if m.path.is_empty() { "(no file name)" } else { m.path.as_str() })).small().weak());
                            }
                            ui.horizontal(|ui| {
                                let label = if is_audio(&m.file) { "Play" } else if is_pdf(&m.file) { "Read" } else { "Open" };
                                if !m.file.is_empty() && ui.button(label).clicked() {
                                    if is_pdf(&m.file) {
                                        *pdf_req = Some((full.clone(), if m.caption.is_empty() { m.name.clone() } else { m.caption.clone() }));
                                    } else {
                                        open_media(&m.file);
                                    }
                                }
                                if ui.button("Remove").clicked() {
                                    remove = Some(*i);
                                }
                            });
                        });
                    });
                }
                if let Some(i) = remove {
                    if let Some(p) = self.db.people.get_mut(&id) {
                        if i < p.media.len() {
                            p.media.remove(i);
                        }
                    }
                }
            }
            ui.label(RichText::new("Photos, documents and recordings are stored on this computer only.").small().weak());
            }
            "todo" => self.todo_panel(ui, id),
            "journal" => self.journal_panel(ui, id),
            "more" => {
            if ui.button("Merge with another person...").on_hover_text("Use this when the same person is in your tree twice").clicked() {
                self.merge_open = true;
                self.merge_other = None;
                self.merge_id.clear();
            }
            ui.add_space(4.0);
            let label = if self.confirm { "Click again to delete" } else { "Delete person" };
            if ui.button(label).clicked() {
                if self.confirm {
                    *act = Some(Action::Delete);
                } else {
                    self.confirm = true;
                }
            }
            }
            _ => {}
        }
    }

    /// The panels that are shown and docked, one after another, each with its own title bar.
    fn dock_panels(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, editing: bool) {
        let items = self.settings.dock.clone();
        let mut cmd: Option<(usize, u8)> = None; // 0 up, 1 down, 2 pop out, 3 hide
        for (i, it) in items.iter().enumerate() {
            if !it.on || it.float {
                continue;
            }
            let cid = ui.make_persistent_id(("dock", it.sec.as_str()));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), cid, true)
                .show_header(ui, |ui| {
                    ui.label(RichText::new(dock_title(&it.sec)).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").on_hover_text("Hide this panel").clicked() {
                            cmd = Some((i, 3));
                        }
                        if ui.small_button("Float").on_hover_text("Pop this panel out into its own window").clicked() {
                            cmd = Some((i, 2));
                        }
                        if ui.small_button("v").on_hover_text("Move down").clicked() {
                            cmd = Some((i, 1));
                        }
                        if ui.small_button("^").on_hover_text("Move up").clicked() {
                            cmd = Some((i, 0));
                        }
                    });
                })
                .body(|ui| {
                    if editing { self.person_section(ui, act, id, &it.sec) } else { self.person_read(ui, act, id, &it.sec) }
                });
            ui.add_space(4.0);
        }
        if let Some((i, c)) = cmd {
            let d = &mut self.settings.dock;
            match c {
                0 if i > 0 => d.swap(i, i - 1),
                1 if i + 1 < d.len() => d.swap(i, i + 1),
                2 => d[i].float = true,
                3 => d[i].on = false,
                _ => {}
            }
            save_settings(&self.settings);
        }
    }

    fn person_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let id = self.sel.unwrap();
        let editing = matches!(&self.editing, Some(e) if e.id == id);
        self.nav_bar(ui, act);
        ui.add_space(2.0);
        if editing {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Editing. Changes are only kept when you press Save.").strong());
                    if ui.button("Save").clicked() {
                        *act = Some(Action::EditSave);
                    }
                    if ui.button("Cancel").on_hover_text("Put this person back exactly as they were before you pressed Edit").clicked() {
                        *act = Some(Action::EditCancel);
                    }
                });
            });
        } else {
            ui.horizontal(|ui| {
                if ui.button("Edit").on_hover_text("Unlock this page to make changes").clicked() {
                    *act = Some(Action::EditStart);
                }
                ui.label(RichText::new("Locked. Press Edit to change anything.").weak());
            });
        }
        self.problem_flags(ui, id);
        ui.add_space(4.0);
        self.dock_bar(ui);
        ui.add_space(4.0);
        let docked = self.settings.dock.iter().any(|d| d.on && !d.float);
        let wide = ui.available_width() > 820.0;
        if wide && docked {
            egui::SidePanel::right("person_dock").resizable(true).default_width(self.settings.dock_w).width_range(280.0..=720.0).show_inside(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("person_dock_scroll").auto_shrink([false, false]).show(ui, |ui| {
                    self.dock_panels(ui, act, id, editing);
                });
            });
        }
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("person_main").auto_shrink([false, false]).show(ui, |ui| {
                if editing {
                    self.person_section(ui, act, id, "facts");
                    self.person_section(ui, act, id, "timeline");
                } else {
                    self.person_read(ui, act, id, "facts");
                    self.person_read(ui, act, id, "timeline");
                }
                if !wide && docked {
                    ui.add_space(10.0);
                    self.dock_panels(ui, act, id, editing);
                }
            });
        });
        // panels popped out into their own windows
        let ctx = ui.ctx().clone();
        let items = self.settings.dock.clone();
        for (i, it) in items.iter().enumerate() {
            if !(it.on && it.float) {
                continue;
            }
            let mut open = true;
            let mut dock_back = false;
            egui::Window::new(dock_title(&it.sec)).id(egui::Id::new(("float", it.sec.as_str()))).open(&mut open).default_size([420.0, 420.0]).show(&ctx, |ui| {
                if ui.small_button("Dock back on the right").clicked() {
                    dock_back = true;
                }
                egui::ScrollArea::vertical().id_salt(("float_scroll", it.sec.as_str())).show(ui, |ui| {
                    if editing { self.person_section(ui, act, id, &it.sec) } else { self.person_read(ui, act, id, &it.sec) }
                });
            });
            if dock_back {
                self.settings.dock[i].float = false;
                save_settings(&self.settings);
            } else if !open {
                self.settings.dock[i].on = false;
                self.settings.dock[i].float = false;
                save_settings(&self.settings);
            }
        }
    }

    /// Row of buttons that show or hide each panel.
    fn dock_bar(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let mut reset = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Panels").small().weak());
            for it in self.settings.dock.iter_mut() {
                if ui.selectable_label(it.on, dock_title(&it.sec)).on_hover_text("Show or hide this panel on the right").clicked() {
                    it.on = !it.on;
                    if it.on {
                        it.float = false;
                    }
                    changed = true;
                }
            }
            let mut apply: Option<Vec<DockItem>> = None;
            let mut apply_w: Option<f32> = None;
            ui.menu_button("Layouts", |ui| {
                ui.label(RichText::new("Ready-made").small().weak());
                for (name, secs) in LAYOUT_PRESETS {
                    if ui.button(*name).clicked() {
                        let mut d: Vec<DockItem> = secs.iter().map(|s| DockItem { sec: s.to_string(), on: true, float: false }).collect();
                        for x in default_dock() {
                            if !d.iter().any(|y| y.sec == x.sec) {
                                d.push(DockItem { on: false, ..x });
                            }
                        }
                        apply = Some(d);
                        ui.close_menu();
                    }
                }
                let mut del: Option<usize> = None;
                if !self.settings.layouts.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Yours").small().weak());
                    for (i, l) in self.settings.layouts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.button(l.name.as_str()).clicked() {
                                apply = Some(l.dock.clone());
                                apply_w = Some(l.width);
                                ui.close_menu();
                            }
                            if ui.small_button("x").on_hover_text("Forget this layout").clicked() {
                                del = Some(i);
                            }
                        });
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.x.layout_name).hint_text("Name for this layout").desired_width(150.0));
                    if ui.add_enabled(!self.x.layout_name.trim().is_empty(), egui::Button::new("Save current")).clicked() {
                        let name = self.x.layout_name.trim().to_string();
                        self.settings.layouts.retain(|l| l.name != name);
                        self.settings.layouts.push(Layout { name, dock: self.settings.dock.clone(), width: self.settings.dock_w });
                        self.x.layout_name.clear();
                        changed = true;
                        ui.close_menu();
                    }
                });
                if let Some(i) = del {
                    self.settings.layouts.remove(i);
                    changed = true;
                }
            });
            if let Some(d) = apply {
                self.settings.dock = d;
                if let Some(w) = apply_w {
                    self.settings.dock_w = w;
                }
                changed = true;
            }
            if ui.small_button("Reset layout").clicked() {
                reset = true;
            }
        });
        if reset {
            self.settings.dock = default_dock();
            self.settings.dock_w = 380.0;
            changed = true;
        }
        if changed {
            save_settings(&self.settings);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut act: Option<Action> = None;
        NAME_STYLE.store(self.settings.name_style, Ordering::Relaxed);
        DATE_STYLE.store(self.settings.date_style, Ordering::Relaxed);
        MAP_STYLE.store(self.settings.map_style, Ordering::Relaxed);
        let rgb = |c: [u8; 3]| (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
        APP_BG.store(rgb(self.settings.bg), Ordering::Relaxed);
        APP_ACCENT.store(rgb(self.settings.accent), Ordering::Relaxed);
        let scale = self.settings.ui_scale.clamp(0.7, 2.0);
        if (ctx.zoom_factor() - scale).abs() > 0.001 {
            ctx.set_zoom_factor(scale);
        }
        // egui's default (0.3 s) is quicker than most people double-click, especially on a Mac.
        ctx.options_mut(|o| {
            if o.input_options.max_double_click_delay < 0.5 {
                o.input_options.max_double_click_delay = 0.5;
            }
        });

        let (has_f, has_m) = self.sel.map(|s| self.db.parents(s)).map(|(f, m)| (f.is_some(), m.is_some())).unwrap_or((false, false));
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(format!("Family trees... (open: {})", tree_title(&self.settings.current_tree))).on_hover_text("Switch between separate trees, or start a new one").clicked() {
                        self.x.trees_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Open Family Tree Archive...").on_hover_text("Restore everything from an archive file (tree, photos, documents, stories)").clicked() {
                        self.open_archive_pick();
                        ui.close_menu();
                    }
                    if ui.button("Save Family Tree Archive...").on_hover_text("One file with your whole tree, every photo, document, story and recording, and a GEDCOM copy").clicked() {
                        self.save_archive_pick();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Import tree...").clicked() {
                        self.imp_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Export GEDCOM (everything)...").clicked() {
                        self.export(false);
                        ui.close_menu();
                    }
                    if ui.button("Export GEDCOM (living people private)...").clicked() {
                        self.export(true);
                        ui.close_menu();
                    }
                    {
                        let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                        if ui.button(if fs { "Leave full screen        F11" } else { "Full screen        F11 (Mac: Ctrl+Cmd+F)" }).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.add_enabled(self.sel.is_some(), egui::Button::new("Print pedigree chart...")).clicked() {
                        act = Some(Action::PrintPedigree);
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.sel.is_some(), egui::Button::new("Print family group sheet...")).clicked() {
                        act = Some(Action::PrintFamily);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo")).clicked() {
                        self.undo_action();
                        ui.close_menu();
                    }
                    if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo")).clicked() {
                        self.redo_action();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Edit individual").clicked() {
                        self.view = View::Person;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Add", |ui| {
                    if ui.button("Individual...").clicked() {
                        act = Some(Action::Add(Kind::Person));
                        ui.close_menu();
                    }
                    if ui.button("Spouse...").clicked() {
                        act = Some(Action::Add(Kind::Spouse));
                        ui.close_menu();
                    }
                    if ui.add_enabled(!has_f, egui::Button::new("Father...")).clicked() {
                        act = Some(Action::Add(Kind::Father));
                        ui.close_menu();
                    }
                    if ui.add_enabled(!has_m, egui::Button::new("Mother...")).clicked() {
                        act = Some(Action::Add(Kind::Mother));
                        ui.close_menu();
                    }
                    if ui.button("Child...").clicked() {
                        let fid = self.sel.and_then(|s| self.db.fams_of(s).first().copied());
                        act = Some(Action::Add(Kind::Child(fid)));
                        ui.close_menu();
                    }
                    if ui.button("Another set of parents (adoptive, step...)...").clicked() {
                        act = Some(Action::Add(Kind::NewParent(true)));
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Go to home person").clicked() {
                        act = Some(Action::GoHome);
                        ui.close_menu();
                    }
                    if ui.button("Fit chart to window").clicked() {
                        act = Some(Action::Fit);
                        ui.close_menu();
                    }
                    ui.separator();
                    for (v, l) in [(View::Family, "Family"), (View::Pedigree, "Pedigree"), (View::Descendants, "Descendants"), (View::Person, "Individual"), (View::Gallery, "Gallery"), (View::Memories, "Memories"), (View::Charts, "Charts"), (View::Table, "People"), (View::Map, "Map"), (View::Dna, "DNA")] {
                        if ui.button(l).clicked() {
                            self.view = v;
                            ui.close_menu();
                        }
                    }
                    ui.menu_button("Generations", |ui| {
                        for n in 3..=8usize {
                            if ui.selectable_label(self.settings.gens == n, format!("{} generations", n)).clicked() {
                                act = Some(Action::SetGens(n));
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    let mut changed = false;
                    ui.menu_button("Colours", |ui| {
                        ui.menu_button("Colour schemes", |ui| {
                            egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                                if scheme_list(ui, &mut self.settings) {
                                    changed = true;
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.separator();
                        ui.label(RichText::new("Accent colour").strong());
                        if swatch_grid(ui, &PRESETS, &mut self.settings.accent, false) {
                            changed = true;
                        }
                        if self.settings.dark {
                            ui.separator();
                            ui.label(RichText::new("Dark background").strong());
                            if swatch_grid(ui, &BG_PRESETS, &mut self.settings.bg, true) {
                                changed = true;
                            }
                        }
                        ui.separator();
                        if ui.button("More: theme and your own colours...").clicked() {
                            self.show_look = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Text size", |ui| {
                        for (v, l) in [(0.85f32, "Small"), (1.0, "Normal"), (1.15, "Large"), (1.3, "Larger"), (1.5, "Largest")] {
                            if ui.selectable_label((self.settings.ui_scale - v).abs() < 0.01, l).clicked() {
                                self.settings.ui_scale = v;
                                changed = true;
                            }
                        }
                        ui.label(RichText::new("Makes all text and buttons bigger or smaller.").small().weak());
                    });
                    ui.menu_button("Names", |ui| {
                        for (v, l) in [(0u8, "John Henry Smith"), (1, "SMITH, John Henry"), (2, "Smith, John Henry")] {
                            if ui.selectable_label(self.settings.name_style == v, l).clicked() {
                                self.settings.name_style = v;
                                changed = true;
                            }
                        }
                    });
                    ui.menu_button("Dates", |ui| {
                        for (v, l) in [(0u8, "As typed"), (1, "12 Mar 1921"), (2, "March 12, 1921"), (3, "1921-03-12")] {
                            if ui.selectable_label(self.settings.date_style == v, l).clicked() {
                                self.settings.date_style = v;
                                changed = true;
                            }
                        }
                        ui.label(RichText::new("Only changes how dates look. What you typed is kept.").small().weak());
                    });
                    ui.menu_button("Profile photo size", |ui| {
                        for (v, l) in [(0u8, "Small"), (1, "Normal"), (2, "Large"), (3, "Extra large")] {
                            if ui.selectable_label(self.settings.photo_size == v, l).clicked() {
                                self.settings.photo_size = v;
                                changed = true;
                            }
                        }
                        ui.label(RichText::new("The photo at the top of the Individual page.").small().weak());
                    });
                    ui.menu_button("Chart boxes", |ui| {
                        changed |= ui.checkbox(&mut self.settings.box_photos, "Show photos").changed();
                        changed |= ui.checkbox(&mut self.settings.box_places, "Show birthplace").changed();
                        changed |= ui.checkbox(&mut self.settings.box_ids, "Show person ID").changed();
                    });
                    {
                        let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                        if ui.button(if fs { "Leave full screen" } else { "Full screen" }).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                            ui.close_menu();
                        }
                    }
                    if changed {
                        save_settings(&self.settings);
                        apply_theme(ctx, &self.settings);
                    }
                    ui.separator();
                    if ui.button("Appearance...").clicked() {
                        self.show_look = true;
                        ui.close_menu();
                    }
                    if ui.button("Where my tree is kept...").clicked() {
                        self.show_loc = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Search", |ui| {
                    if ui.button("Find individual...").clicked() {
                        self.find_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Relationship finder...").clicked() {
                        self.rel_open = true;
                        if self.rel_a.is_none() {
                            self.rel_a = self.sel;
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("Reports", |ui| {
                    if ui.checkbox(&mut self.settings.hide_living, "Keep living people private").on_hover_text("Printed charts, sheets and reports show living people as \"Living\" with no dates or places").changed() {
                        save_settings(&self.settings);
                    }
                    ui.separator();
                    if ui.button("Report designer (make your own)...").clicked() {
                        self.x.designer_open = true;
                        ui.close_menu();
                    }
                    ui.menu_button("Forms (blank, to fill in by hand)", |ui| {
                        if ui.button("Family group sheet").clicked() {
                            self.open_print(forms::blank_family_group());
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart, 4 generations").clicked() {
                            self.open_print(forms::blank_pedigree(4));
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart, 5 generations").clicked() {
                            self.open_print(forms::blank_pedigree(5));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Family group sheet with LDS ordinances").clicked() {
                            self.open_print(forms::blank_family_group_lds());
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart with LDS ordinances, 4 generations").clicked() {
                            self.open_print(forms::blank_pedigree_lds(4));
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart with LDS ordinances, 5 generations").clicked() {
                            self.open_print(forms::blank_pedigree_lds(5));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Census forms...").on_hover_text("United States 1790-1950, Canada 1851-1931, United Kingdom 1841-1921").clicked() {
                            self.x.census.open = true;
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    let has = self.sel.is_some();
                    for (label, k) in [
                        ("Ancestor list...", Action::ReportAncestors),
                        ("Descendant list...", Action::ReportDescendants),
                        ("Descendant chart...", Action::PrintDescendants),
                        ("Birthdays and anniversaries...", Action::ReportBirthdays),
                        ("Family book...", Action::ReportBook),
                        ("Source list (bibliography)...", Action::ReportSources),
                    ] {
                        if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                            act = Some(k);
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Sources and repositories...").clicked() {
                        self.src_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Home person...").on_hover_text("Choose who the Home button and Home key go back to").clicked() {
                        self.home_pick = Some(String::new());
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add_enabled(backup_path().exists(), egui::Button::new("Restore previous tree")).clicked() {
                        self.request_wipe(Wipe::Restore);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Research to-do list...").clicked() {
                        self.x.todo_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Research journal...").clicked() {
                        self.x.journal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Family statistics...").clicked() {
                        self.x.stats_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Historical events...").clicked() {
                        self.x.hist_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Places...").clicked() {
                        self.x.places_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Tombstone calculator...").on_hover_text("Work out a birth date from the death date and the age on a gravestone").clicked() {
                        self.open_tombstone(self.sel);
                        ui.close_menu();
                    }
                    if ui.button("History of changes...").clicked() {
                        self.x.history_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Saved versions (daily copies)...").clicked() {
                        self.x.ver_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Data checker...").clicked() {
                        self.check_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Find duplicates...").clicked() {
                        self.dups = self.db.duplicate_pairs().into_iter().filter(|x| !self.dup_ignore.contains(&(x.0, x.1))).collect();
                        self.dup_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Backup settings...").clicked() {
                        self.x.backup_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Restore from a dated backup...").clicked() {
                        self.restore_from_file();
                        ui.close_menu();
                    }
                    if !is_portable() && self.settings.current_tree.is_empty() && ui.button("Move my data folder...").on_hover_text("Keep your tree somewhere other than Documents").clicked() {
                        self.move_data = true;
                        ui.close_menu();
                    }
                    if ui.button("Open backups folder").clicked() {
                        open_folder(&data_dir().join("backups").join("x"));
                        ui.close_menu();
                    }
                    if ui.button("Back up now (data and photos)").clicked() {
                        self.backup_now(true);
                        ui.close_menu();
                    }
                    if ui.button("Check backups...").clicked() {
                        self.run_verify();
                        ui.close_menu();
                    }
                    if ui.button("Restore missing photos and recordings").clicked() {
                        self.spawn_media_job(true);
                        ui.close_menu();
                    }
                    ui.separator();
                    if is_portable() {
                        ui.add_enabled(false, egui::Button::new("Portable mode is on"));
                    } else if ui.button("Make portable (keep data next to the app)").clicked() {
                        self.make_portable();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About Gilded Roots").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.x.practice.is_none(), egui::Button::new("Explore the sample family (practice)")).clicked() {
                        self.start_practice();
                        ui.close_menu();
                    }
                    if ui.button("Open-source licences").clicked() {
                        self.x.licenses_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("User guide        F1 (Mac: Cmd+?)").clicked() {
                        self.help_open = true;
                        ui.close_menu();
                    }
                    ui.menu_button("Keyboard shortcuts", |ui| {
                        for (k, d) in [
                            ("F1  (Mac: Cmd+?)", "User guide"),
                            ("F2", "Edit the selected person"),
                            ("F3", "Find a person"),
                            ("F4  or  Ctrl/Cmd+K", "Quick jump"),
                            ("F5", "Fit the chart to the window"),
                            ("F6", "Next tab"),
                            ("F7", "Data checker"),
                            ("F8", "Research to-do list"),
                            ("F9", "Places"),
                            ("F11  (Mac: Ctrl+Cmd+F)", "Full screen"),
                            ("F12", "History of changes"),
                            ("Arrow keys", "Move around the chart"),
                            ("Home", "Go to the home person"),
                            ("Ctrl/Cmd+1..9", "Family, Pedigree, Descendants, Individual, Gallery, Charts, People, Map, Memories"),
                            ("Ctrl/Cmd + / -", "Zoom the chart"),
                            ("H", "Pedigree and descendants: switch between Select and the hand"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.add_sized([170.0, 18.0], egui::Label::new(RichText::new(k).strong()));
                                ui.label(d);
                            });
                        }
                        ui.label(RichText::new("On a Mac, hold fn to use the F keys.").small().weak());
                    });
                    if ui.button("Open data folder").clicked() {
                        open_folder(&store_path());
                        ui.close_menu();
                    }
                });
            });
        });

        if self.x.practice.is_some() {
            let mut leave = false;
            egui::TopBottomPanel::top("practice").frame(egui::Frame::default().fill(Color32::from_rgb(196, 152, 40)).inner_margin(egui::Margin::symmetric(10.0, 6.0))).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Practice mode: you are exploring the made-up sample family. Nothing here is saved, and your own tree is safe.").color(Color32::BLACK).strong());
                    if ui.button("Back to my tree").clicked() {
                        leave = true;
                    }
                });
            });
            if leave {
                self.end_practice();
            }
        }
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (v, l) in [(View::Family, "Family"), (View::Pedigree, "Pedigree"), (View::Descendants, "Descendants"), (View::Person, "Individual"), (View::Gallery, "Gallery"), (View::Memories, "Memories"), (View::Charts, "Charts"), (View::Table, "People"), (View::Map, "Map"), (View::Dna, "DNA")] {
                    let mut t = RichText::new(l).size(16.0);
                    if self.view == v {
                        t = t.strong();
                    }
                    if ui.selectable_label(self.view == v, t).clicked() {
                        self.view = v;
                    }
                }
            });
        });

        if let Some(err) = self.save_error.clone() {
            egui::TopBottomPanel::top("save_err").show(ctx, |ui| {
                ui.colored_label(
                    Color32::from_rgb(225, 70, 70),
                    format!("Your latest changes could not be saved: {}. Your tree is still open in the app and it will keep trying. Free up space or fix the folder, and use File > Export GEDCOM now to keep a copy.", err),
                );
            });
        }
        let mut dismiss = false;
        if let Some(n) = self.notice.clone() {
            egui::TopBottomPanel::top("notice").show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(Color32::from_rgb(230, 170, 40), n.as_str());
                    if ui.button("Dismiss").clicked() {
                        dismiss = true;
                    }
                });
            });
        }
        if dismiss {
            self.notice = None;
        }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let msg = if self.status.is_empty() {
                    "For help, press F1. Double-click a person to open their page."
                } else {
                    self.status.as_str()
                };
                ui.label(msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.save_error.is_some() {
                        ui.colored_label(Color32::from_rgb(225, 70, 70), "NOT SAVED");
                    } else if self.dirty {
                        ui.label("Saving...");
                    } else if let Some(t) = self.last_saved {
                        ui.label(format!("Saved {}", ago(t.elapsed())));
                    } else {
                        ui.label("All changes saved");
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let ok = self.sel.map(|s| self.db.people.contains_key(&s)).unwrap_or(false);
            if !ok {
                self.welcome(ui, &mut act);
                return;
            }
            if self.sel.is_some() {
                self.trail_bar(ui, &mut act);
            }
            match self.view {
                View::Pedigree => self.pedigree(ui, &mut act),
                View::Family => self.family_view(ui, &mut act),
                View::Descendants => self.descendants(ui, &mut act),
                View::Person => self.person_view(ui, &mut act),
                View::Gallery => self.gallery_view(ui, &mut act),
                View::Charts => self.charts_view(ui, &mut act),
                View::Table => self.table_view(ui, &mut act),
                View::Map => self.map_view(ui, &mut act),
                View::Memories => self.memories_view(ui, &mut act),
                View::Dna => self.dna_view(ui, &mut act),
            }
        });

        // Leaving a person (or the Individual tab) while editing: keep, discard or stay.
        if let Some(e) = &self.editing {
            let leaving = self.view != View::Person || self.sel != Some(e.id);
            if leaving {
                let changed = serde_json::to_string(&self.db).map(|c| c != e.snapshot).unwrap_or(true);
                if !changed {
                    self.editing = None;
                } else {
                    let name = self.db.people.get(&e.id).map(display_name).unwrap_or_default();
                    let mut choice = 0u8;
                    egui::Area::new(egui::Id::new("leave_edit_block")).order(egui::Order::Foreground).fixed_pos(pos2(0.0, 0.0)).show(ctx, |ui| {
                        let r = ctx.screen_rect();
                        ui.allocate_rect(r, Sense::click_and_drag());
                        ui.painter().rect_filled(r, 0.0_f32, Color32::from_black_alpha(110));
                    });
                    egui::Window::new("Unsaved changes").collapsible(false).resizable(false).order(egui::Order::Tooltip).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                        ui.set_max_width(380.0);
                        ui.label(format!("You have changed {} and not pressed Save.", name));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                choice = 1;
                            }
                            if ui.button("Discard changes").clicked() {
                                choice = 2;
                            }
                            if ui.button("Keep editing").clicked() {
                                choice = 3;
                            }
                        });
                    });
                    match choice {
                        1 => {
                            self.editing = None;
                            self.dirty = true;
                        }
                        2 => {
                            if let Some(e) = self.editing.take() {
                                self.load_snapshot(e.snapshot);
                                self.dirty = true;
                            }
                        }
                        3 => {
                            if let Some(e) = &self.editing {
                                self.sel = Some(e.id);
                            }
                            self.view = View::Person;
                        }
                        _ => {}
                    }
                }
            }
        }

        self.global_keys(ctx, &mut act);
        if self.help_open {
            let mut open = true;
            let mut find = self.help_find.clone();
            egui::Window::new("User guide").open(&mut open).default_size([640.0, 600.0]).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search");
                    ui.add(egui::TextEdit::singleline(&mut find).hint_text("e.g. merge, sources, home person").desired_width(260.0));
                    if !find.is_empty() && ui.small_button("Clear").clicked() {
                        find.clear();
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| help_text(ui, &find));
            });
            self.help_find = find;
            self.help_open = open;
        }

        self.jump_window(ctx, &mut act);
        if let Some((p, t)) = self.x.pdf_req.take() {
            match pdfview::PdfView::open(&p, &t) {
                Ok(v) => self.x.pdf = Some(v),
                Err(e) => {
                    self.status = format!("{} Opening it in your PDF app instead.", e);
                    open_path(&p);
                }
            }
        }
        if let Some(v) = self.x.pdf.as_mut() {
            if !v.show(ctx) {
                self.x.pdf = None;
            }
        }
        self.todo_window(ctx, &mut act);
        self.places_window(ctx, &mut act);
        self.tombstone_window(ctx);
        self.census_window(ctx);
        self.history_window(ctx);
        self.archive_windows(ctx);
        self.journal_window(ctx, &mut act);
        self.history_events_window(ctx);
        self.stats_window(ctx, &mut act);
        self.trees_window(ctx);
        self.designer_window(ctx);
        self.story_window(ctx);
        self.versions_window(ctx);
        if self.home_pick.is_some() {
            let mut open = true;
            let mut chosen: Option<u32> = None;
            let cur = self.settings.home.filter(|h| self.db.people.contains_key(h)).map(|h| self.db.label(h));
            let sel = self.sel;
            let sel_label = sel.map(|i| self.db.label(i));
            let q = self.home_pick.clone().unwrap_or_default();
            let ql = q.trim().to_lowercase();
            let mut hits: Vec<(u32, String)> = Vec::new();
            if !ql.is_empty() {
                for (i, p) in &self.db.people {
                    if p.name.to_lowercase().contains(&ql) || i.to_string() == ql {
                        hits.push((*i, self.db.label(*i)));
                        if hits.len() >= 200 {
                            break;
                        }
                    }
                }
                hits.sort_by(|a, b| a.1.cmp(&b.1));
            }
            let mut q2 = q.clone();
            egui::Window::new("Home person").open(&mut open).collapsible(false).default_width(420.0).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("The Home button and the Home key always come back to this person, and the tree opens on them.");
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Current home person: {}", cur.clone().unwrap_or_else(|| "none chosen".into()))).strong());
                ui.add_space(6.0);
                if let (Some(i), Some(l)) = (sel, sel_label.clone()) {
                    if ui.button(format!("Use the person selected now: {}", l)).clicked() {
                        chosen = Some(i);
                    }
                    ui.add_space(4.0);
                }
                ui.add(egui::TextEdit::singleline(&mut q2).hint_text("Type a name or ID to search").desired_width(f32::INFINITY));
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (i, l) in &hits {
                        if ui.selectable_label(false, l.as_str()).clicked() {
                            chosen = Some(*i);
                        }
                    }
                    if !ql.is_empty() && hits.is_empty() {
                        ui.weak("No one matches.");
                    }
                });
            });
            self.home_pick = Some(q2);
            if let Some(i) = chosen {
                self.settings.home = Some(i);
                save_settings(&self.settings);
                self.status = format!("Home person is now {}.", self.db.label(i));
                self.home_pick = None;
            } else if !open {
                self.home_pick = None;
            }
        }

        if self.quick.is_some() {
            let mut save = false;
            let mut close = false;
            let mut full: Option<u32> = None;
            if let Some(q) = self.quick.as_mut() {
                egui::Window::new("Quick edit").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                    egui::Grid::new("quick_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("Given name(s)");
                        ui.add(egui::TextEdit::singleline(&mut q.given).desired_width(260.0));
                        ui.end_row();
                        ui.label("Surname");
                        ui.add(egui::TextEdit::singleline(&mut q.surname).desired_width(260.0));
                        ui.end_row();
                        ui.label("Sex");
                        let st = match q.sex.as_str() {
                            "M" => "Male",
                            "F" => "Female",
                            _ => "Unknown",
                        };
                        egui::ComboBox::from_id_salt("qsex").selected_text(st).show_ui(ui, |ui| {
                            ui.selectable_value(&mut q.sex, "M".to_string(), "Male");
                            ui.selectable_value(&mut q.sex, "F".to_string(), "Female");
                            ui.selectable_value(&mut q.sex, String::new(), "Unknown");
                        });
                        ui.end_row();
                        ui.label("Born");
                        ui.add(egui::TextEdit::singleline(&mut q.bd).desired_width(260.0));
                        ui.end_row();
                        ui.label("Birthplace");
                        ui.add(egui::TextEdit::singleline(&mut q.bp).desired_width(260.0));
                        ui.end_row();
                        ui.label("Died");
                        ui.add(egui::TextEdit::singleline(&mut q.dd).desired_width(260.0));
                        ui.end_row();
                        ui.label("Place of death");
                        ui.add(egui::TextEdit::singleline(&mut q.dp).desired_width(260.0));
                        ui.end_row();
                        ui.label("Living");
                        ui.checkbox(&mut q.living, "Living (kept private when printing or exporting)");
                        ui.end_row();
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                        if ui.button("Open full page").clicked() {
                            full = Some(q.id);
                            close = true;
                        }
                    });
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        close = true;
                    }
                });
            }
            if save {
                if let Some(q) = self.quick.take() {
                    self.apply_quick(&q);
                    self.dirty = true;
                    self.status = "Saved.".to_string();
                }
            } else if close {
                self.quick = None;
            }
            if let Some(i) = full {
                self.goto(i);
                self.view = View::Person;
            }
        }

        if self.find_open {
            let mut open = true;
            egui::Window::new("Find Individual").open(&mut open).default_width(300.0).show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Name or place").desired_width(f32::INFINITY));
                ui.label(format!("{} people", self.db.people.len()));
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for id in self.db.sorted(&self.search) {
                        let txt = self.db.label(id);
                        if ui.selectable_label(self.sel == Some(id), txt).clicked() {
                            act = Some(Action::Select(id));
                        }
                    }
                });
            });
            self.find_open = open;
        }

        self.review_window(ctx);
        self.import_merge_window(ctx);
        if let Some(id) = self.src_pick.take() {
            if self.db.sources.contains_key(&id) {
                self.src_open = true;
                self.src_tab = 0;
                self.src_sel = Some(id);
                self.src_confirm = false;
            }
        }
        if self.src_open {
            let mut open = true;
            let mut go: Option<u32> = None;
            let mut del_src: Option<u32> = None;
            let mut merge: Option<(u32, u32)> = None;
            let mut del_repo: Option<u32> = None;
            let mut new_repo_for: Option<u32> = None;
            let counts = self.db.source_counts();
            let mut by_title: Vec<(u32, String)> = self.db.sources.iter().map(|(k, s)| (*k, source_title(s))).collect();
            by_title.sort_by_key(|t| t.1.to_lowercase());
            let mut repo_list: Vec<(u32, String)> = self.db.repos.iter().map(|(k, r)| (*k, if r.name.is_empty() { "(unnamed repository)".to_string() } else { r.name.clone() })).collect();
            repo_list.sort_by_key(|t| t.1.to_lowercase());
            let repo_use: HashMap<u32, usize> = {
                let mut n = HashMap::new();
                for s in self.db.sources.values() {
                    if let Some(r) = s.repo {
                        *n.entry(r).or_insert(0) += 1;
                    }
                }
                n
            };
            egui::Window::new("Sources and repositories").open(&mut open).default_size([900.0, 560.0]).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.src_tab == 0, format!("Sources ({})", self.db.sources.len())).clicked() {
                        self.src_tab = 0;
                        self.src_confirm = false;
                    }
                    if ui.selectable_label(self.src_tab == 1, format!("Repositories ({})", self.db.repos.len())).clicked() {
                        self.src_tab = 1;
                        self.src_confirm = false;
                    }
                });
                ui.separator();
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(270.0);
                        if self.src_tab == 0 {
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.src_filter).hint_text("search sources").desired_width(160.0));
                                if ui.button("New source").clicked() {
                                    let id = self.db.alloc();
                                    self.db.sources.insert(id, SourceRec::default());
                                    self.src_sel = Some(id);
                                    self.src_filter.clear();
                                }
                            });
                            let f = self.src_filter.to_lowercase();
                            egui::ScrollArea::vertical().id_salt("srclist").max_height(440.0).show(ui, |ui| {
                                for (id, t) in &by_title {
                                    let s = &self.db.sources[id];
                                    if !f.is_empty() && !format!("{} {} {}", t, s.author, s.abbr).to_lowercase().contains(&f) {
                                        continue;
                                    }
                                    let n = counts.get(id).copied().unwrap_or(0);
                                    let label = format!("{}  ({})", t, n);
                                    if ui.selectable_label(self.src_sel == Some(*id), label).clicked() {
                                        self.src_sel = Some(*id);
                                        self.src_confirm = false;
                                        self.src_merge = None;
                                    }
                                }
                            });
                        } else {
                            if ui.button("New repository").clicked() {
                                let id = self.db.alloc();
                                self.db.repos.insert(id, RepoRec::default());
                                self.repo_sel = Some(id);
                            }
                            egui::ScrollArea::vertical().id_salt("repolist").max_height(440.0).show(ui, |ui| {
                                for (id, t) in &repo_list {
                                    let n = repo_use.get(id).copied().unwrap_or(0);
                                    if ui.selectable_label(self.repo_sel == Some(*id), format!("{}  ({})", t, n)).clicked() {
                                        self.repo_sel = Some(*id);
                                        self.src_confirm = false;
                                    }
                                }
                            });
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_min_width(560.0);
                        egui::ScrollArea::vertical().id_salt("srcedit").max_height(470.0).show(ui, |ui| {
                            if self.src_tab == 0 {
                                let Some(sel) = self.src_sel.filter(|s| self.db.sources.contains_key(s)) else {
                                    ui.label("Choose a source on the left, or make a new one.");
                                    ui.label(RichText::new("A source is a book, census, record set, website or person. Write it down once here, then cite it from any number of people, facts and events (each citation adds its own page, transcription and quality).").weak());
                                    return;
                                };
                                let uses = self.db.source_uses(sel);
                                {
                                    let s = self.db.sources.get_mut(&sel).unwrap();
                                    egui::Grid::new("srcgrid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                        ui.label("Title");
                                        ui.add(egui::TextEdit::singleline(&mut s.title).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Author or creator");
                                        ui.add(egui::TextEdit::singleline(&mut s.author).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Short title");
                                        ui.add(egui::TextEdit::singleline(&mut s.abbr).hint_text("used in short lists").desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Type of source");
                                        egui::ComboBox::from_id_salt("srckind").selected_text(if s.kind.is_empty() { "not set" } else { s.kind.as_str() }).width(200.0).show_ui(ui, |ui| {
                                            ui.selectable_value(&mut s.kind, String::new(), "not set");
                                            for k in SOURCE_KINDS {
                                                ui.selectable_value(&mut s.kind, k.to_string(), *k);
                                            }
                                        });
                                        ui.end_row();
                                        ui.label("Publication details");
                                        ui.add(egui::TextEdit::singleline(&mut s.publ).hint_text("publisher, place, year, volume").desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Web address");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(&mut s.url).hint_text("https://...").desired_width(310.0));
                                            if is_url(&s.url) {
                                                ui.hyperlink_to("Open", s.url.clone());
                                            }
                                        });
                                        ui.end_row();
                                        ui.label("Kept at");
                                        ui.horizontal(|ui| {
                                            let shown = s.repo.and_then(|r| repo_list.iter().find(|t| t.0 == r)).map(|t| t.1.clone()).unwrap_or_else(|| "no repository".to_string());
                                            egui::ComboBox::from_id_salt("srcrepo").selected_text(shown).width(260.0).show_ui(ui, |ui| {
                                                if ui.selectable_label(s.repo.is_none(), "no repository").clicked() {
                                                    s.repo = None;
                                                }
                                                for (rid, t) in &repo_list {
                                                    if ui.selectable_label(s.repo == Some(*rid), t.as_str()).clicked() {
                                                        s.repo = Some(*rid);
                                                    }
                                                }
                                                ui.separator();
                                                if ui.selectable_label(false, "+ New repository...").clicked() {
                                                    new_repo_for = Some(sel);
                                                }
                                            });
                                            if let Some(r) = s.repo {
                                                if ui.small_button("Open repository").clicked() {
                                                    self.repo_sel = Some(r);
                                                    self.src_tab = 1;
                                                }
                                            }
                                        });
                                        ui.end_row();
                                        ui.label("Call number");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(&mut s.call).hint_text("shelf, film or reference number").desired_width(220.0));
                                            ui.add(egui::TextEdit::singleline(&mut s.medium).hint_text("book, microfilm, digital...").desired_width(150.0));
                                        });
                                        ui.end_row();
                                    });
                                    ui.label("Description, or text of the whole source");
                                    ui.add(egui::TextEdit::multiline(&mut s.text).desired_rows(3).desired_width(f32::INFINITY));
                                    ui.label("Your notes on this source (how reliable it is, where you found it)");
                                    ui.add(egui::TextEdit::multiline(&mut s.notes).desired_rows(3).desired_width(f32::INFINITY));
                                }
                                ui.add_space(6.0);
                                ui.separator();
                                ui.label(RichText::new(format!("Cited {} time(s)", uses.len())).strong());
                                egui::ScrollArea::vertical().id_salt("srcuses").max_height(150.0).show(ui, |ui| {
                                    for (o, what, when, page) in uses.iter().take(300) {
                                        ui.horizontal(|ui| {
                                            let who = self.db.owner_label(*o);
                                            if ui.link(who).clicked() {
                                                go = match o {
                                                    Owner::Person(i) => Some(*i),
                                                    Owner::Family(i) => self.db.fams.get(i).and_then(|f| f.h.or(f.w)),
                                                };
                                            }
                                            let mut t = what.clone();
                                            if !when.is_empty() {
                                                t = format!("{} {}", t, when);
                                            }
                                            if !page.is_empty() {
                                                t = format!("{}  -  {}", t, page);
                                            }
                                            ui.label(RichText::new(t).weak());
                                        });
                                    }
                                    if uses.len() > 300 {
                                        ui.label(format!("...and {} more", uses.len() - 300));
                                    }
                                });
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    let shown = self.src_merge.filter(|m| *m != sel).and_then(|m| by_title.iter().find(|t| t.0 == m)).map(|t| t.1.clone()).unwrap_or_else(|| "Merge into...".to_string());
                                    egui::ComboBox::from_id_salt("srcmerge").selected_text(shown).width(230.0).show_ui(ui, |ui| {
                                        for (id, t) in &by_title {
                                            if *id != sel && ui.selectable_label(self.src_merge == Some(*id), t.as_str()).clicked() {
                                                self.src_merge = Some(*id);
                                            }
                                        }
                                    });
                                    if let Some(target) = self.src_merge.filter(|m| *m != sel && self.db.sources.contains_key(m)) {
                                        if ui.button("Merge (same source, entered twice)").clicked() {
                                            merge = Some((target, sel));
                                        }
                                    }
                                });
                                let label = if self.src_confirm { "Click again to delete this source" } else { "Delete source" };
                                if ui.button(label).on_hover_text("Citations that used it keep its name as typed text").clicked() {
                                    if self.src_confirm {
                                        del_src = Some(sel);
                                    } else {
                                        self.src_confirm = true;
                                    }
                                }
                            } else {
                                let Some(sel) = self.repo_sel.filter(|s| self.db.repos.contains_key(s)) else {
                                    ui.label("Choose a repository on the left, or make a new one.");
                                    ui.label(RichText::new("A repository is where sources are kept: a library, archive, courthouse, church, website or a relative's attic.").weak());
                                    return;
                                };
                                {
                                    let r = self.db.repos.get_mut(&sel).unwrap();
                                    egui::Grid::new("repogrid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                        ui.label("Name");
                                        ui.add(egui::TextEdit::singleline(&mut r.name).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Address");
                                        ui.add(egui::TextEdit::multiline(&mut r.addr).desired_rows(3).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Phone");
                                        ui.add(egui::TextEdit::singleline(&mut r.phone).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Email");
                                        ui.add(egui::TextEdit::singleline(&mut r.email).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Web address");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(&mut r.url).hint_text("https://...").desired_width(310.0));
                                            if is_url(&r.url) {
                                                ui.hyperlink_to("Open", r.url.clone());
                                            }
                                        });
                                        ui.end_row();
                                    });
                                    ui.label("Notes (opening hours, how to request records)");
                                    ui.add(egui::TextEdit::multiline(&mut r.notes).desired_rows(3).desired_width(f32::INFINITY));
                                }
                                ui.add_space(6.0);
                                ui.separator();
                                ui.label(RichText::new("Sources kept here").strong());
                                let here: Vec<(u32, String)> = by_title.iter().filter(|(id, _)| self.db.sources[id].repo == Some(sel)).cloned().collect();
                                if here.is_empty() {
                                    ui.label(RichText::new("None yet. Choose this repository in a source's \"Kept at\" box.").weak());
                                }
                                for (id, t) in here {
                                    if ui.link(t).clicked() {
                                        self.src_sel = Some(id);
                                        self.src_tab = 0;
                                    }
                                }
                                ui.add_space(6.0);
                                let label = if self.src_confirm { "Click again to delete this repository" } else { "Delete repository" };
                                if ui.button(label).on_hover_text("Its sources stay, they just no longer say where they are kept").clicked() {
                                    if self.src_confirm {
                                        del_repo = Some(sel);
                                    } else {
                                        self.src_confirm = true;
                                    }
                                }
                            }
                        });
                    });
                });
            });
            self.src_open = open;
            if let Some(src) = new_repo_for {
                let rid = self.db.alloc();
                self.db.repos.insert(rid, RepoRec::default());
                if let Some(s) = self.db.sources.get_mut(&src) {
                    s.repo = Some(rid);
                }
                self.repo_sel = Some(rid);
                self.src_tab = 1;
            }
            if let Some((keep, gone)) = merge {
                self.db.merge_sources(keep, gone);
                self.src_sel = Some(keep);
                self.src_merge = None;
                self.dirty = true;
                self.status = "Merged the two sources. Everything that cited either now cites the one source.".to_string();
            }
            if let Some(id) = del_src {
                self.db.delete_source(id);
                self.src_sel = None;
                self.src_confirm = false;
                self.dirty = true;
            }
            if let Some(id) = del_repo {
                self.db.delete_repo(id);
                self.repo_sel = None;
                self.src_confirm = false;
                self.dirty = true;
            }
            if let Some(p) = go {
                act = Some(Action::Open(p));
            }
        }

        if self.show_loc {
            let mut open = true;
            let mut reset = false;
            egui::Window::new("Where my tree is kept").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label("Your tree, photos, recordings and backups are kept in this folder:");
                ui.label(RichText::new(data_dir().display().to_string()).strong());
                if let Some(svc) = synced_service(&data_dir()) {
                    ui.label(RichText::new(format!("This folder looks like it is synced by {}. Avoid opening the same tree on two computers at once.", svc)).color(Color32::from_rgb(214, 160, 60)));
                }
                if is_portable() {
                    ui.label("Portable mode is on, so everything stays next to the app.");
                } else {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Choose a different folder...").clicked() {
                            self.move_data = true;
                        }
                        if config_dir().join("location.txt").exists() && ui.button("Back to the default (Documents)").clicked() {
                            reset = true;
                        }
                        if ui.button("Open folder").clicked() {
                            open_folder(&store_path());
                        }
                    });
                    ui.label(RichText::new("Your data is copied to the new place first and checked. The old folder is left as it was.").small().weak());
                }
            });
            self.show_loc = open;
            if reset {
                self.persist();
                let src = data_dir();
                let target = documents_dir().join("Gilded Roots");
                if target.join("tree.json").exists() || src == target {
                    let _ = std::fs::remove_file(config_dir().join("location.txt"));
                    self.status = format!("Using {} again.", target.display());
                } else {
                    match copy_dir(&src, &target) {
                        Ok(_) if std::fs::read(src.join("tree.json")).ok() == std::fs::read(target.join("tree.json")).ok() => {
                            let _ = std::fs::remove_file(config_dir().join("location.txt"));
                            self.status = format!("Your tree is back in {}. The other folder was left untouched.", target.display());
                        }
                        _ => self.status = "Could not copy your data, so nothing was changed.".to_string(),
                    }
                }
            }
        }

        if self.show_look {
            let mut open = true;
            let mut changed = false;
            egui::Window::new("Appearance").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label("Theme");
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.settings.dark, "Dark").clicked() {
                        self.settings.dark = true;
                        self.settings.ink = None;
                        changed = true;
                    }
                    if ui.selectable_label(!self.settings.dark, "Light").clicked() {
                        self.settings.dark = false;
                        self.settings.ink = None;
                        self.settings.light_bg = default_light_bg();
                        changed = true;
                    }
                });
                ui.add_space(8.0);
                ui.collapsing(RichText::new("Colour schemes").strong(), |ui| {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        if scheme_list(ui, &mut self.settings) {
                            changed = true;
                        }
                    });
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Accent colour").strong());
                if swatch_grid(ui, &PRESETS, &mut self.settings.accent, false) {
                    changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Custom");
                    if ui.color_edit_button_srgb(&mut self.settings.accent).changed() {
                        changed = true;
                    }
                });
                if self.settings.dark {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Dark background").strong());
                    if swatch_grid(ui, &BG_PRESETS, &mut self.settings.bg, true) {
                        changed = true;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Custom");
                        if ui.color_edit_button_srgb(&mut self.settings.bg).changed() {
                            changed = true;
                        }
                    });
                }
                ui.add_space(6.0);
                if ui.button("Reset to default").clicked() {
                    let keep = self.settings.gens;
                    self.settings = Settings::default();
                    self.settings.gens = keep;
                    changed = true;
                }
            });
            self.show_look = open;
            if changed {
                apply_theme(ctx, &self.settings);
                save_settings(&self.settings);
            }
        }

        if let Some(w) = self.wipe {
            let (title, msg) = match w {
                Wipe::Sample => ("Load sample data?", "This replaces your tree with the sample family."),
                Wipe::Empty => ("Start with an empty tree?", "This removes everyone from your tree."),
                Wipe::Import => ("Import a GEDCOM file?", "Importing replaces your tree with the contents of the file."),
                Wipe::Restore => ("", ""),
            };
            let mut choice: Option<bool> = None;
            egui::Window::new(title).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label(format!("You have {} people in your tree. {}", self.db.people.len(), msg));
                ui.label("A backup of your current tree is saved first. You can bring it back with Tools > Restore previous tree.");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        choice = Some(false);
                    }
                    if ui.button("Replace my tree").clicked() {
                        choice = Some(true);
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    choice = Some(false);
                }
            });
            if let Some(c) = choice {
                self.wipe = None;
                if c {
                    self.run_wipe(w);
                }
            }
        }

        if self.move_data {
            self.move_data = false;
            self.change_data_folder();
        }
        if self.show_about {
            let mut open = true;
            egui::Window::new("About Gilded Roots").open(&mut open).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new(egui::include_image!("../icons/icon-256.png")).max_width(110.0));
                    ui.add_space(4.0);
                    ui.heading("Gilded Roots");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label("A free, offline home for your family history.");
                    ui.label(RichText::new("Created by Doyle McIntosh (DGMCINT)").italics());
                });
                ui.separator();
                ui.label(RichText::new("Quick tips").strong());
                ui.label("- Click a box in the pedigree to highlight it. Double-click to open that person's page.");
                ui.label("- Click a blank box to add a father or mother.");
                ui.label("- Right-click a box (or use its ... button) for more actions.");
                ui.label("- Press Ctrl/Cmd+K to jump to anyone or any command. Press F1 for the user guide.");
                ui.label("- Use the arrow on the last column to see older generations.");
                ui.label("- Use File > Export GEDCOM to make a backup you can keep anywhere.");
                ui.separator();
                ui.label(RichText::new("Support").strong());
                ui.label("Gilded Roots is free. If you enjoy it and would like to help with its development, a small donation, even $5, is very much appreciated. It is entirely optional.");
                ui.hyperlink_to("Donate with PayPal", DONATE_URL);
                ui.separator();
                ui.label(RichText::new("Your data").strong());
                ui.label("Everything is stored on this computer only.");
                ui.label(format!("Folder: {}", data_dir().display()));
                if is_portable() {
                    ui.label("Portable mode: your data is kept in the FamilyTreeData folder next to the app.");
                }
                if mac_translocated() {
                    ui.colored_label(Color32::from_rgb(222, 166, 60), "macOS is running this copy from a hidden temporary folder, so portable mode cannot work. Open Terminal and run once:  xattr -dr com.apple.quarantine  followed by a space, then drag Gilded Roots.app onto the Terminal window and press Return. Then open the app again.");
                }
                if let Some(svc) = synced_service(&data_dir()) {
                    ui.label(RichText::new(format!("This folder looks like it is synced by {}. That is fine on one computer, but avoid opening the same tree on two computers at once, and keep files downloaded (turn off storage optimising for this folder). \"Change folder...\" moves it somewhere that does not sync.", svc)).color(Color32::from_rgb(214, 160, 60)));
                }
                ui.horizontal(|ui| {
                    if ui.button("Open data folder").clicked() {
                        open_folder(&store_path());
                    }
                    if !is_portable() && ui.button("Change folder...").clicked() {
                        self.move_data = true;
                    }
                });
                ui.separator();
                ui.label(RichText::new("Licence and notices").strong());
                ui.label("Gilded Roots is free, open-source software under the MIT licence. It comes as is, without any warranty.");
                ui.label(RichText::new(NOT_AFFILIATED).small().weak());
                ui.label(RichText::new("Map data: Natural Earth (public domain). Town names: GeoNames, CC BY 4.0.").small().weak());
                ui.horizontal(|ui| {
                    if ui.button("Open-source licences...").clicked() {
                        self.x.licenses_open = true;
                    }
                    if PROJECT_URL.is_empty() {
                        ui.label(RichText::new("Questions and bug reports: the Issues tab of the Gilded Roots page on GitHub.").small());
                    } else {
                        ui.hyperlink_to("Questions and bug reports", PROJECT_URL);
                    }
                });
            });
            self.show_about = open;
        }
        if self.x.licenses_open {
            let mut open = true;
            egui::Window::new("Open-source licences").open(&mut open).default_size([640.0, 560.0]).show(ctx, |ui| {
                ui.label("Gilded Roots is built with these open-source libraries. Thank you to their authors.");
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(THIRD_PARTY).monospace().small()).wrap());
                });
            });
            self.x.licenses_open = open;
        }

        if self.imp_open {
            let mut open = true;
            let mut pick = false;
            egui::Window::new("Import a family tree").open(&mut open).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label("Ancestry, MyHeritage, FamilySearch and most programs share trees as a GEDCOM (.ged) file. Export one from your account, then choose it here.");
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for (i, n) in ["Any GEDCOM file", "Ancestry", "MyHeritage", "FamilySearch"].iter().enumerate() {
                        if ui.selectable_label(self.imp_src == i, *n).clicked() {
                            self.imp_src = i;
                        }
                    }
                });
                ui.separator();
                match self.imp_src {
                    1 => {
                        ui.label("1. Sign in to Ancestry and open your tree.");
                        ui.label("2. Click the tree name menu and choose Tree Settings.");
                        ui.label("3. Under Manage Your Tree, click Export tree and wait for it to be prepared.");
                        ui.label("4. Click Download your GEDCOM file.");
                        ui.weak("You usually need to be the tree owner. Photos and documents are not included in the file.");
                        ui.hyperlink_to("Open Ancestry", "https://www.ancestry.com");
                    }
                    2 => {
                        ui.label("1. Sign in to MyHeritage.");
                        ui.label("2. Open the Family Tree menu and choose Manage trees.");
                        ui.label("3. Click Export to GEDCOM, then Begin the export.");
                        ui.label("4. Download the file (you may get an email with a link).");
                        ui.weak("Photos are not included in the file.");
                        ui.hyperlink_to("Open MyHeritage", "https://www.myheritage.com");
                    }
                    3 => {
                        ui.label("1. Sign in to FamilySearch and open the Download your family tree page.");
                        ui.label("2. Click the export button to download a GEDCOM file.");
                        ui.weak("It includes up to 8 generations of your ancestors, without photos. FamilySearch's shared tree has no full-tree download. Programs like RootsMagic can export more.");
                        ui.hyperlink_to("Open the FamilySearch export page", "https://familysearch.org/innovate/export");
                    }
                    _ => {
                        ui.label("Any program that can export GEDCOM works, such as RootsMagic, Legacy, Gramps, Family Tree Maker, or PAF itself.");
                    }
                }
                ui.separator();
                ui.radio_value(&mut self.imp_replace, false, "Add to my tree (you check and confirm every possible match)");
                ui.radio_value(&mut self.imp_replace, true, "Replace my tree with this file");
                ui.add_space(6.0);
                if ui.button("Choose GEDCOM file...").clicked() {
                    pick = true;
                }
            });
            if pick {
                open = false;
                if self.imp_replace || self.is_pristine_sample() || self.db.people.is_empty() {
                    self.request_wipe(Wipe::Import);
                } else {
                    self.import_merge();
                }
            }
            self.imp_open = open;
        }

        if self.rel_open {
            let mut open = true;
            egui::Window::new("Relationship finder").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label("Choose two people to see how they are related.");
                let ids = self.db.sorted("");
                for (title, which) in [("Person A", 0), ("Person B", 1)] {
                    ui.label(title);
                    let cur = if which == 0 { &mut self.rel_a } else { &mut self.rel_b };
                    let shown = (*cur).filter(|i| self.db.people.contains_key(i)).map(|i| self.db.label(i)).unwrap_or_else(|| "Choose...".to_string());
                    egui::ComboBox::from_id_salt(("rel", which)).selected_text(shown).width(340.0).show_ui(ui, |ui| {
                        for id in &ids {
                            if ui.selectable_label(*cur == Some(*id), self.db.label(*id)).clicked() {
                                *cur = Some(*id);
                            }
                        }
                    });
                }
                if let (Some(a), Some(b)) = (self.rel_a, self.rel_b) {
                    if self.db.people.contains_key(&a) && self.db.people.contains_key(&b) {
                        ui.separator();
                        ui.label(RichText::new(self.db.relationship(a, b)).strong());
                    }
                }
            });
            self.rel_open = open;
        }

        if self.view == View::Person {
            if let Some(id) = self.sel {
                self.ensure_previews(id);
            }
        }
        if matches!(self.view, View::Gallery | View::Memories | View::Person) && self.missing_previews(4) {
            // new previews were made: draw again straight away (and make the next few), so new
            // photos appear without having to leave the tab and come back
            ctx.request_repaint();
        }
        self.viewer(ctx);

        if self.check_open {
            let mut open = true;
            let issues = self.db.check();
            let mut go_to: Option<u32> = None;
            egui::Window::new("Data checker").open(&mut open).default_size([560.0, 420.0]).show(ctx, |ui| {
                ui.label(format!("{} thing(s) worth a look. Click a name to open that person.", issues.len()));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (id, msg) in &issues {
                        ui.horizontal(|ui| {
                            if ui.link(self.db.label(*id)).clicked() {
                                go_to = Some(*id);
                            }
                            ui.label(msg.as_str());
                        });
                    }
                });
            });
            self.check_open = open;
            if let Some(id) = go_to {
                act = Some(Action::Open(id));
            }
        }

        if self.merge_open {
            let mut open = true;
            let mut do_merge: Option<(u32, u32)> = None;
            match self.sel.filter(|i| self.db.people.contains_key(i)) {
                None => open = false,
                Some(me) => {
                    let mut others: Vec<u32> = self.db.people.keys().copied().filter(|i| *i != me).collect();
                    others.sort_by_key(|i| self.db.people[i].name.to_lowercase());
                    egui::Window::new("Merge with another person").open(&mut open).default_size([460.0, 220.0]).show(ctx, |ui| {
                        ui.label(format!("Keep: {}", self.db.label(me)));
                        ui.label("Choose the other record of this person. You will see both side by side and confirm before anything is merged.");
                        let shown = self.merge_other.filter(|i| self.db.people.contains_key(i)).map(|i| self.db.label(i)).unwrap_or_else(|| "Choose...".to_string());
                        egui::ComboBox::from_id_salt("mergeother").selected_text(shown).width(380.0).show_ui(ui, |ui| {
                            for id in &others {
                                if ui.selectable_label(self.merge_other == Some(*id), format!("{}  #{}", self.db.label(*id), id)).clicked() {
                                    self.merge_other = Some(*id);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("or type the person's ID (#):");
                            ui.add(egui::TextEdit::singleline(&mut self.merge_id).desired_width(70.0));
                        });
                        if let Ok(n) = self.merge_id.trim().trim_start_matches('#').parse::<u32>() {
                            if n != me && self.db.people.contains_key(&n) {
                                self.merge_other = Some(n);
                            }
                        }
                        if let Some(o) = self.merge_other {
                            if ui.button("Compare and merge...").clicked() {
                                do_merge = Some((me, o));
                            }
                        }
                    });
                }
            }
            if let Some((a, b)) = do_merge {
                self.start_review(a, b);
                open = false;
            }
            self.merge_open = open;
        }

        if self.dup_open {
            let mut open = true;
            let mut merge: Option<(u32, u32)> = None;
            let mut ignore: Option<(u32, u32)> = None;
            let mut rescan = false;
            egui::Window::new("Possible duplicates").open(&mut open).default_size([580.0, 440.0]).show(ctx, |ui| {
                ui.label(format!("{} possible duplicate pair(s).", self.dups.len()));
                if ui.button("Scan again").clicked() {
                    rescan = true;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (a, b, why) in &self.dups {
                        if !self.db.people.contains_key(a) || !self.db.people.contains_key(b) {
                            continue;
                        }
                        ui.group(|ui| {
                            ui.label(RichText::new(why.as_str()).small().weak());
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(format!("{}  #{}", self.db.label(*a), a)).strong());
                                    ui.label(self.db.people[a].bp.as_str());
                                });
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(format!("{}  #{}", self.db.label(*b), b)).strong());
                                    ui.label(self.db.people[b].bp.as_str());
                                });
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Compare and merge...").clicked() {
                                    merge = Some((*a, *b));
                                }
                                if ui.button("Not the same").clicked() {
                                    ignore = Some((*a, *b));
                                }
                            });
                        });
                    }
                });
            });
            self.dup_open = open;
            if let Some((a, b)) = merge {
                self.start_review(a, b);
            }
            if let Some(pair) = ignore {
                self.dup_ignore.insert(pair);
                self.dups.retain(|x| (x.0, x.1) != pair);
            }
            if rescan {
                self.dups = self.db.duplicate_pairs().into_iter().filter(|x| !self.dup_ignore.contains(&(x.0, x.1))).collect();
            }
        }

        if self.report_open {
            let mut open = true;
            let mut save_text: Option<String> = None;
            if let Some(r) = &self.report {
                egui::Window::new("Import report").open(&mut open).default_size([620.0, 520.0]).show(ctx, |ui| {
                    ui.label(RichText::new(format!("{}", r.file)).strong());
                    let mut from = String::new();
                    if !r.program.is_empty() {
                        from.push_str(&format!("Written by {}. ", r.program));
                    }
                    if !r.gedcom_version.is_empty() {
                        from.push_str(&format!("GEDCOM {}. ", r.gedcom_version));
                    }
                    from.push_str(&format!("Character set: {}.", r.charset));
                    ui.label(from);
                    ui.add_space(6.0);
                    ui.label(format!(
                        "{} people, {} families, {} sources ({} citations), {} shared notes, {} other names, {} events and facts, {} photos/documents.",
                        r.people, r.families, r.sources, r.citations, r.notes, r.alt_names, r.events, r.media
                    ));
                    if r.media > 0 {
                        ui.label(format!("Photos and documents: {} found and copied in, {} not found next to the file (their names are kept).", r.media_found, r.media_missing));
                    }
                    ui.add_space(6.0);
                    if r.lost.is_empty() {
                        ui.colored_label(Color32::from_rgb(90, 190, 110), "Nothing was lost: every line of the file will appear again when you export.");
                    } else {
                        ui.colored_label(Color32::from_rgb(225, 70, 70), format!("{} piece(s) of the file would NOT appear in an export:", r.lost.len()));
                        egui::ScrollArea::vertical().id_salt("lostlist").max_height(150.0).show(ui, |ui| {
                            for l in &r.lost {
                                ui.label(RichText::new(l).monospace());
                            }
                        });
                    }
                    if !r.kept.is_empty() {
                        ui.add_space(6.0);
                        egui::CollapsingHeader::new(format!("{} item(s) kept exactly as read (this app has no field for them, but they are saved and written back)", r.kept_total())).show(ui, |ui| {
                            egui::ScrollArea::vertical().id_salt("keptlist").max_height(150.0).show(ui, |ui| {
                                for (k, n) in &r.kept {
                                    ui.label(RichText::new(format!("{:>5}  {}", n, k)).monospace());
                                }
                            });
                        });
                    }
                    if !r.warnings.is_empty() {
                        ui.add_space(6.0);
                        egui::CollapsingHeader::new(format!("{} note(s) about the file", r.warnings.len())).default_open(true).show(ui, |ui| {
                            egui::ScrollArea::vertical().id_salt("warnlist").max_height(120.0).show(ui, |ui| {
                                for w in r.warnings.iter().take(200) {
                                    ui.label(w);
                                }
                            });
                        });
                    }
                    ui.add_space(8.0);
                    if ui.button("Save this report as a text file...").clicked() {
                        let mut t = format!("Import report for {}\n{}\nCharacter set: {}\n\n", r.file, r.program, r.charset);
                        t.push_str(&format!("{} people, {} families, {} sources, {} citations, {} shared notes, {} other names, {} events, {} media\n\n", r.people, r.families, r.sources, r.citations, r.notes, r.alt_names, r.events, r.media));
                        t.push_str(&format!("Would not appear in an export ({}):\n", r.lost.len()));
                        for l in &r.lost {
                            t.push_str(&format!("  {}\n", l));
                        }
                        t.push_str("\nKept exactly as read:\n");
                        for (k, n) in &r.kept {
                            t.push_str(&format!("  {:>5}  {}\n", n, k));
                        }
                        t.push_str("\nNotes:\n");
                        for w in &r.warnings {
                            t.push_str(&format!("  {}\n", w));
                        }
                        save_text = Some(t);
                    }
                });
            }
            self.report_open = open;
            if let Some(t) = save_text {
                if let Some(p) = rfd::FileDialog::new().set_file_name("import-report.txt").save_file() {
                    self.status = match atomic_write(&p, t.as_bytes()) {
                        Ok(_) => format!("Saved {}", p.display()),
                        Err(e) => format!("Could not save: {}", e),
                    };
                }
            }
        }

        if self.verify_open {
            let mut open = true;
            let mut again = false;
            egui::Window::new("Backup check").open(&mut open).default_size([540.0, 380.0]).show(ctx, |ui| {
                ui.label("Each backup is read back and compared with its checksum.");
                if self.verify_results.is_empty() {
                    ui.label("No dated backups yet.");
                }
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    for (name, r) in &self.verify_results {
                        match r {
                            Ok(n) => {
                                ui.label(format!("OK        {}   ({} people)", name, n));
                            }
                            Err(e) => {
                                ui.colored_label(Color32::from_rgb(225, 70, 70), format!("DAMAGED   {}   {}", name, e));
                            }
                        }
                    }
                });
                ui.separator();
                ui.label(self.media_gap.as_str());
                if ui.button("Check again").clicked() {
                    again = true;
                }
            });
            self.verify_open = open;
            if again {
                self.run_verify();
            }
        }

        if self.link.is_some() {
            let mut close = false;
            let mut chosen: Option<(u32, Option<u32>, bool, u32)> = None;
            let mut cands: Vec<(u32, String)> = Vec::new();
            if let Some(l) = self.link.as_ref() {
                cands = self.db.sorted(&l.q).into_iter().filter(|i| *i != l.child).take(12).map(|i| (i, self.db.label(i))).collect();
            }
            if let Some(l) = self.link.as_mut() {
                egui::Window::new("Link an existing person as a parent").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Search");
                        ui.text_edit_singleline(&mut l.q);
                    });
                    if l.fid.is_none() {
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut l.father, true, "as father / first parent");
                            ui.radio_value(&mut l.father, false, "as mother / second parent");
                        });
                    }
                    ui.separator();
                    for (i, lbl) in &cands {
                        if ui.link(lbl.as_str()).clicked() {
                            chosen = Some((l.child, l.fid, l.father, *i));
                        }
                    }
                    if cands.is_empty() {
                        ui.weak("No matching people.");
                    }
                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            }
            if let Some((c, f, fa, p)) = chosen {
                self.db.add_parent(c, f, fa, p);
                self.dirty = true;
                close = true;
            }
            if close {
                self.link = None;
            }
        }

        let mut done: Option<bool> = None;
        if let Some(d) = self.dlg.as_mut() {
            let title = match d.kind {
                Kind::Father => "Add father",
                Kind::Mother => "Add mother",
                Kind::Spouse => "Add spouse",
                Kind::Child(_) => "Add child",
                Kind::Person => "Add person",
                Kind::ParentIn(_, true) => "Add father to this family",
                Kind::ParentIn(_, false) => "Add mother to this family",
                Kind::NewParent(true) => "Add another father (new set of parents)",
                Kind::NewParent(false) => "Add another mother (new set of parents)",
                Kind::Sibling => "Add brother or sister",
            };
            let lock = matches!(d.kind, Kind::Father | Kind::Mother | Kind::Spouse);
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(440.0);
                    let mut name_resp = None;
                    egui::Grid::new("add_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("Full name");
                        let te = ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(300.0));
                        if d.fresh > 0 {
                            te.request_focus();
                            d.fresh -= 1;
                            ctx.request_repaint();
                        }
                        name_resp = Some(te);
                        ui.end_row();
                        ui.label("Sex");
                        ui.add_enabled_ui(!lock, |ui| {
                            let st = match d.sex.as_str() {
                                "F" => "Female",
                                "M" => "Male",
                                _ => "Unknown",
                            };
                            egui::ComboBox::from_id_salt("dsex").selected_text(st).show_ui(ui, |ui| {
                                ui.selectable_value(&mut d.sex, "M".to_string(), "Male");
                                ui.selectable_value(&mut d.sex, "F".to_string(), "Female");
                                ui.selectable_value(&mut d.sex, String::new(), "Unknown");
                            });
                        });
                        ui.end_row();
                        ui.label("Born");
                        ui.add(egui::TextEdit::singleline(&mut d.bd).hint_text("e.g. 12 Mar 1921, about 1850").desired_width(300.0));
                        ui.end_row();
                        ui.label("Birthplace");
                        ui.add(egui::TextEdit::singleline(&mut d.bp).desired_width(300.0));
                        ui.end_row();
                        ui.label("Died");
                        ui.add(egui::TextEdit::singleline(&mut d.dd).desired_width(300.0));
                        ui.end_row();
                        ui.label("Place of death");
                        ui.add(egui::TextEdit::singleline(&mut d.dp).desired_width(300.0));
                        ui.end_row();
                        ui.label("Buried");
                        ui.add(egui::TextEdit::singleline(&mut d.bud).desired_width(300.0));
                        ui.end_row();
                        ui.label("Burial place");
                        ui.add(egui::TextEdit::singleline(&mut d.bup).desired_width(300.0));
                        ui.end_row();
                        ui.label("Living");
                        ui.checkbox(&mut d.living, "Living (kept private when printing or exporting)");
                        ui.end_row();
                        ui.label("Notes");
                        ui.add(egui::TextEdit::multiline(&mut d.notes).desired_rows(3).desired_width(300.0));
                        ui.end_row();
                    });
                    ui.label(RichText::new("Only the name is required. Everything else can be added later.").small().weak());
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() && !d.name.trim().is_empty() {
                            done = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            done = Some(false);
                        }
                    });
                    let enter = name_resp.map(|t| t.lost_focus()).unwrap_or(false) && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if enter && !d.name.trim().is_empty() {
                        done = Some(true);
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        done = Some(false);
                    }
                });
        }
        if let Some(ok) = done {
            let d = self.dlg.take().unwrap();
            if ok {
                self.create(d);
                self.dirty = true;
            }
        }

        if !ctx.wants_keyboard_input() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z)) {
                self.redo_action();
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.undo_action();
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)) {
                self.redo_action();
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.find_open = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            act = Some(Action::Add(Kind::Person));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals)) {
            act = Some(Action::SetZoom(self.settings.zoom + 0.1));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus)) {
            act = Some(Action::SetZoom(self.settings.zoom - 0.1));
        }
        for (k, v) in [(egui::Key::Num1, View::Family), (egui::Key::Num2, View::Pedigree), (egui::Key::Num3, View::Descendants), (egui::Key::Num4, View::Person), (egui::Key::Num5, View::Gallery), (egui::Key::Num6, View::Charts), (egui::Key::Num7, View::Table), (egui::Key::Num8, View::Map), (egui::Key::Num9, View::Memories)] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, k)) {
                self.view = v;
            }
        }

        // Keyboard: arrows move around the tree (not while typing or while a dialog is open).
        if act.is_none()
            && self.dlg.is_none()
            && self.link.is_none()
            && self.review.is_none()
            && self.imerge.is_none()
            && !ctx.wants_keyboard_input()
            && matches!(self.view, View::Pedigree | View::Descendants | View::Family)
        {
            let (key, shift, home) = ctx.input(|i| {
                let plain = !i.modifiers.command && !i.modifiers.alt;
                let k = [egui::Key::ArrowRight, egui::Key::ArrowLeft, egui::Key::ArrowUp, egui::Key::ArrowDown].into_iter().find(|k| plain && i.key_pressed(*k));
                (k, i.modifiers.shift, plain && i.key_pressed(egui::Key::Home))
            });
            if home {
                act = Some(Action::GoHome);
            } else if let Some(k) = key {
                if let Some(t) = self.nav_target(k, shift) {
                    act = Some(Action::Select(t));
                }
            }
        }

        if let Some(a) = act {
            match a {
                Action::Select(i) => {
                    self.goto(i);
                    self.hl = None;
                    self.fx = 0;
                    self.confirm = false;
                }
                Action::Fx(i) => self.fx = i,
                Action::Highlight(i) => self.hl = Some(i),
                Action::AttachFiles => {
                    if let Some(id) = self.sel {
                        self.attach_files(id);
                    }
                }
                Action::StartRec => {
                    if let Some(id) = self.sel {
                        if self.rec.is_none() {
                            self.start_recording(id);
                        }
                    }
                }
                Action::StopRec => self.stop_recording(),
                Action::PrintPedigree => {
                    if let Some(r) = self.sel {
                        let h = self.pedigree_html(r);
                        self.open_print(h);
                    }
                }
                Action::Bookmark(i) => {
                    if let Some(k) = self.db.bookmarks.iter().position(|x| *x == i) {
                        self.db.bookmarks.remove(k);
                        self.status = format!("Bookmark removed for {}.", self.db.label(i));
                    } else {
                        self.db.bookmarks.push(i);
                        self.status = format!("Bookmarked {}. Find them under Bookmarks above the chart.", self.db.label(i));
                    }
                    self.dirty = true;
                }
                Action::PrintChart => {
                    if let Some((title, body)) = self.chart_page() {
                        let landscape = self.x.chart != ChartKind::Fan || self.x.fan_deg < 359.0;
                        self.open_print(page(&title, &body, landscape));
                    }
                }
                Action::PrintFamily => {
                    if let Some(r) = self.sel {
                        let h = self.family_html(r);
                        self.open_print(h);
                    }
                }
                Action::SetZoom(z) => {
                    self.settings.zoom = z.clamp(0.25, 1.8);
                    self.fit_req = false;
                    save_settings(&self.settings);
                }
                Action::Fit => self.fit_req = true,
                Action::EditStart => {
                    if let Some(id) = self.sel {
                        self.view = View::Person;
                        self.editing = Some(EditSession { id, snapshot: serde_json::to_string(&self.db).unwrap_or_default() });
                    }
                }
                Action::EditSave => {
                    self.editing = None;
                    self.status = "Changes saved.".to_string();
                    self.dirty = true;
                }
                Action::EditCancel => {
                    if let Some(e) = self.editing.take() {
                        self.load_snapshot(e.snapshot);
                        self.status = "Changes cancelled. The person is back as they were.".to_string();
                        self.dirty = true;
                    }
                }
                Action::SetHome(i) => {
                    self.settings.home = Some(i);
                    save_settings(&self.settings);
                    let who = self.db.people.get(&i).map(display_name).unwrap_or_default();
                    self.status = format!("{} is now the home person. Use the Home button, View > Go to home person, or the Home key to return.", who);
                }
                Action::ShowParentsFor(who, f) => {
                    if let Some(p) = self.db.people.get_mut(&who) {
                        p.pref_parents = Some(f);
                    }
                    self.dirty = true;
                }
                Action::ShowParents(f) => {
                    if let Some(sid) = self.sel {
                        if let Some(p) = self.db.people.get_mut(&sid) {
                            p.pref_parents = Some(f);
                        }
                    }
                }
                Action::GoHome => {
                    if let Some(h) = self.settings.home.filter(|h| self.db.people.contains_key(h)) {
                        self.goto(h);
                        self.hl = None;
                        self.fx = 0;
                    } else {
                        self.status = "No home person set yet. Right-click a person and choose \"Make this the home person\".".to_string();
                    }
                }
                Action::PrintDescendants => {
                    if let Some(r) = self.sel {
                        let h = self.descendant_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportAncestors => {
                    if let Some(r) = self.sel {
                        let h = self.ancestors_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportDescendants => {
                    if let Some(r) = self.sel {
                        let h = self.descendants_list_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportBirthdays => {
                    let h = self.birthdays_html();
                    self.open_print(h);
                }
                Action::ReportSources => {
                    let h = self.sources_html();
                    self.open_print(h);
                }
                Action::ReportBook => {
                    if let Some(r) = self.sel {
                        let h = self.book_html(r);
                        self.open_print(h);
                    }
                }
                Action::Back => {
                    while let Some(p) = self.back.pop() {
                        if self.db.people.contains_key(&p) {
                            if let Some(c) = self.sel {
                                self.fwd.push(c);
                            }
                            self.sel = Some(p);
                            self.hl = None;
                            break;
                        }
                    }
                }
                Action::Forward => {
                    while let Some(p) = self.fwd.pop() {
                        if self.db.people.contains_key(&p) {
                            if let Some(c) = self.sel {
                                self.back.push(c);
                            }
                            self.sel = Some(p);
                            self.hl = None;
                            break;
                        }
                    }
                }
                Action::SetGens(n) => {
                    self.settings.gens = n.clamp(3, 8);
                    save_settings(&self.settings);
                }
                Action::SetHand(on) => {
                    self.settings.hand = on;
                    save_settings(&self.settings);
                }
                Action::AddFor(target, k) => {
                    let t_f = self.db.people.get(&target).map(|p| p.sex == "F").unwrap_or(false);
                    let sex = match k {
                        Kind::Father | Kind::ParentIn(_, true) | Kind::NewParent(true) => "M",
                        Kind::Mother | Kind::ParentIn(_, false) | Kind::NewParent(false) => "F",
                        Kind::Spouse => {
                            if t_f { "M" } else { "F" }
                        }
                        _ => "M",
                    };
                    self.dlg = Some(Dlg::new(k, sex, Some(target)));
                }
                Action::ViewTree(i, which) => {
                    self.goto(i);
                    self.hl = None;
                    self.fx = 0;
                    self.view = match which {
                        0 => View::Pedigree,
                        1 => View::Descendants,
                        _ => View::Family,
                    };
                }
                Action::QuickEdit(i) => {
                    if let Some(p) = self.db.people.get(&i) {
                        self.quick = Some(QuickEdit { id: i, given: p.given.clone(), surname: p.surname.clone(), sex: p.sex.clone(), bd: p.bd.clone(), bp: p.bp.clone(), dd: p.dd.clone(), dp: p.dp.clone(), living: p.living });
                    }
                }
                Action::Open(i) => {
                    self.hl = None;
                    self.goto(i);
                    self.view = View::Person;
                    self.fx = 0;
                    self.confirm = false;
                }
                Action::Add(k) => {
                    let me_f = self.sel.and_then(|i| self.db.people.get(&i)).map(|p| p.sex == "F").unwrap_or(false);
                    let sex = match k {
                        Kind::Father | Kind::ParentIn(_, true) | Kind::NewParent(true) => "M",
                        Kind::Mother | Kind::ParentIn(_, false) | Kind::NewParent(false) => "F",
                        Kind::Spouse => {
                            if me_f { "M" } else { "F" }
                        }
                        _ => "M",
                    };
                    self.dlg = Some(Dlg::new(k, sex, None));
                }
                Action::Delete => {
                    self.delete();
                    self.dirty = true;
                }
            }
        }

        let changed = ctx.input(|i| {
            i.pointer.any_released()
                || i.events.iter().any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Key { pressed: true, .. }))
        });
        if changed {
            self.dirty = true;
            self.track();
            self.backup_if_due(ctx.input(|i| i.time));
        }
        if !self.x.sized {
            self.x.sized = true;
            if self.settings.win.is_none() {
                if let Some(m) = ctx.input(|i| i.viewport().monitor_size) {
                    if m.x > 100.0 && m.y > 100.0 {
                        let w = (m.x * 0.85).clamp(800.0, 1800.0);
                        let h = (m.y * 0.85).clamp(560.0, 1200.0);
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(w, h)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos2((m.x - w) / 2.0, ((m.y - h) / 2.0).max(24.0))));
                    }
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            let (inner, maxed, full) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
            if !full {
                self.settings.win_max = maxed;
                if let (Some(r), false) = (inner, maxed) {
                    self.settings.win = Some([r.width(), r.height()]);
                }
                save_settings(&self.settings);
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty {
            if self.persist() {
                self.dirty = false;
            } else if !self.close_warned {
                self.close_warned = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.status = "Your latest changes could not be saved, so the app was not closed. Fix the problem shown in red, or export a GEDCOM copy first. Close again to quit anyway.".to_string();
            }
        }
        if self.dirty && std::time::Instant::now() >= self.next_save {
            if self.persist() {
                self.dirty = false;
            }
        }
        if !self.startup_done {
            self.startup_done = true;
            self.spawn_media_job(false);
        }
        self.poll_media_job();
        if self.media_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(if self.save_error.is_some() { 3 } else { 10 }));
    }
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

fn load_settings() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .map(|mut st| {
            // panels added in later versions show up for people with older settings
            for d in default_dock() {
                if !st.dock.iter().any(|x| x.sec == d.sec) {
                    st.dock.push(d);
                }
            }
            st
        })
        .unwrap_or_default()
}

fn save_settings(s: &Settings) {
    if let Ok(j) = serde_json::to_string(s) {
        let _ = atomic_write(&settings_path(), j.as_bytes());
    }
}

fn ordinal(n: usize) -> String {
    match n {
        1 => "1st".to_string(),
        2 => "2nd".to_string(),
        3 => "3rd".to_string(),
        _ => format!("{}th", n),
    }
}

fn lighten(c: Color32) -> Color32 {
    let f = |v: u8| v.saturating_add(((255u16 - v as u16) * 40 / 100) as u8);
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

fn on_accent(c: Color32) -> Color32 {
    let l = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if l > 160.0 { Color32::BLACK } else { Color32::WHITE }
}

fn shift(bg: [u8; 3], d: i16) -> Color32 {
    let f = |v: u8| (v as i16 + d).clamp(0, 255) as u8;
    Color32::from_rgb(f(bg[0]), f(bg[1]), f(bg[2]))
}

fn apply_theme(ctx: &egui::Context, settings: &Settings) {
    let (dark, acc, bg) = (settings.dark, settings.accent, settings.bg);
    let rgb = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let accent = Color32::from_rgb(acc[0], acc[1], acc[2]);
    if dark {
        v.panel_fill = shift(bg, 0);
        v.window_fill = shift(bg, 7);
        v.extreme_bg_color = shift(bg, -6);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, shift(bg, 25));
        v.widgets.inactive.bg_fill = shift(bg, 15);
        v.widgets.inactive.weak_bg_fill = shift(bg, 15);
        v.widgets.hovered.bg_fill = shift(bg, 28);
        v.widgets.hovered.weak_bg_fill = shift(bg, 28);
        v.widgets.active.bg_fill = shift(bg, 40);
        v.widgets.active.weak_bg_fill = shift(bg, 40);
        v.hyperlink_color = lighten(accent);
    } else if settings.light_bg == default_light_bg() {
        v.panel_fill = rgb(233, 234, 236);
        v.window_fill = rgb(245, 245, 246);
        v.extreme_bg_color = Color32::WHITE;
        v.hyperlink_color = accent;
    } else {
        // A light colour scheme: the same steps as the dark theme, going darker instead of lighter.
        let lb = settings.light_bg;
        v.panel_fill = shift(lb, 0);
        v.window_fill = shift(lb, 10);
        v.extreme_bg_color = shift(lb, 22);
        v.faint_bg_color = shift(lb, 6);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, shift(lb, -40));
        v.widgets.inactive.bg_fill = shift(lb, -12);
        v.widgets.inactive.weak_bg_fill = shift(lb, -12);
        v.widgets.hovered.bg_fill = shift(lb, -22);
        v.widgets.hovered.weak_bg_fill = shift(lb, -22);
        v.widgets.active.bg_fill = shift(lb, -32);
        v.widgets.active.weak_bg_fill = shift(lb, -32);
        v.hyperlink_color = accent;
    }
    if let Some(ink) = settings.ink {
        let ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.fg_stroke.color = ink;
        }
    }
    v.selection.bg_fill = accent;
    let r = Rounding::same(6.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.window_rounding = Rounding::same(8.0);
    ctx.set_visuals(v);
}

fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../icons/icon-256.png")).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData { rgba: img.into_raw(), width, height })
}

#[cfg(test)]
impl App {
    /// An app with the given tree that never touches the user's real data or settings.
    fn for_test(db: Db) -> App {
        let sel = db.people.keys().next().copied();
        let status = String::new();
        let recovered = false;
        let notice: Option<String> = None;
        let last_json = serde_json::to_string(&db).unwrap_or_default();
        App { db, sel, view: View::Pedigree, search: String::new(), fx: 0, dlg: None, confirm: false, status, dirty: recovered, find_open: false, settings: Settings::default(), show_look: false, show_loc: false, review: None, imerge: None, src_open: false, src_sel: None, src_tab: 0, src_filter: String::new(), repo_sel: None, src_pick: None, src_merge: None, src_confirm: false, hl: None, back: Vec::new(), fwd: Vec::new(), wipe: None, show_about: false, help_open: false, help_find: String::new(), move_data: false, imp_open: false, imp_src: 0, imp_replace: false, rec: None, rel_open: false, rel_a: None, rel_b: None, viewing: None, undo: Vec::new(), redo: Vec::new(), last_json, last_snap: std::time::Instant::now(), last_backup_day: today(), check_open: false, dup_open: false, merge_open: false, merge_id: String::new(), merge_other: None, dups: Vec::new(), dup_ignore: HashSet::new(), save_error: None, last_saved: None, next_save: std::time::Instant::now(), notice, media_job: None, startup_done: false, verify_open: false, verify_results: Vec::new(), media_gap: String::new(), close_warned: false, report: None, report_open: false, link: None, fit_req: false, editing: None, quick: None, gal_only: false, home_pick: None, gal_find: String::new(), assoc_q: String::new(), assoc_role: "Guardian".to_string(), x: Extra::default() }.started()
    }
}

fn main() -> eframe::Result<()> {
    // Open at the size it was last closed at; the first time, a good size that is then fitted
    // to the screen on the first frame (most of the screen, centred, not full screen).
    let st = load_settings();
    if !st.current_tree.is_empty() && base_dir().join("More trees").join(&st.current_tree).is_dir() {
        if let Ok(mut g) = CURRENT_TREE.lock() {
            *g = Some(st.current_tree.clone());
        }
    }
    let size = st.win.map(|w| [w[0].max(800.0), w[1].max(560.0)]).unwrap_or([1400.0, 880.0]);
    let mut vb = egui::ViewportBuilder::default().with_inner_size(size).with_min_inner_size([800.0, 560.0]).with_title(if st.current_tree.is_empty() { "Gilded Roots".to_string() } else { format!("Gilded Roots - {}", st.current_tree) });
    if st.win_max {
        vb = vb.with_maximized(true);
    }
    if let Some(icon) = window_icon() {
        vb = vb.with_icon(icon);
    }
    let opts = eframe::NativeOptions { viewport: vb, ..Default::default() };
    eframe::run_native(
        "Gilded Roots",
        opts,
        Box::new(|cc| {
            let app = App::new();
            egui_extras::install_image_loaders(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, &app.settings);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("familytree-test-{}-{}", tag, stamp()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn good_db() -> Db {
        sample()
    }

    fn damaged_files(d: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(d)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("tree.json.damaged-")).unwrap_or(false))
            .collect()
    }


    /// Whole path: a tree with photos is written as a GEDCOM file, then read back as if it came from another program.
    #[test]
    fn export_then_import_with_photos_end_to_end() {
        let home = tdir("home");
        std::env::set_var("HOME", &home);
        std::env::set_var("APPDATA", &home);
        let mut db = Db::default();
        let a = db.alloc();
        let b = db.alloc();
        let photo = media_dir().join("111-grandpa.jpg");
        std::fs::write(&photo, b"not really a jpeg but bytes are bytes").unwrap();
        db.people.insert(a, Person { name: "Grand Pa".into(), given: "Grand".into(), surname: "Pa".into(), bd: "about 1850".into(), media: vec![Media { file: "111-grandpa.jpg".into(), name: "grandpa.jpg".into(), caption: "On the farm".into(), ..Default::default() }], ..Default::default() });
        db.people.insert(b, Person { name: "Kid Pa".into(), given: "Kid".into(), surname: "Pa".into(), living: true, bd: "2001".into(), ..Default::default() });
        let f = db.alloc();
        db.fams.insert(f, Family { h: Some(a), kids: vec![b], ..Default::default() });

        let out = tdir("out");
        let ged = out.join("tree.ged");
        let msg = write_gedcom(&db, &ged, false).expect("export ok");
        assert!(msg.contains("1 photo/document file"), "{}", msg);
        assert_eq!(std::fs::read(out.join("tree-media").join("grandpa.jpg")).unwrap(), b"not really a jpeg but bytes are bytes");
        let text = std::fs::read_to_string(&ged).unwrap();
        assert!(text.contains("2 FILE tree-media/grandpa.jpg") && text.contains("ABT 1850"));

        // a private copy has no living person and no photos of them
        let ged2 = out.join("private.ged");
        write_gedcom(&db, &ged2, true).expect("private export ok");
        let t2 = std::fs::read_to_string(&ged2).unwrap();
        assert!(!t2.contains("2001") && !t2.contains("Kid Pa") && t2.contains("Living"));

        // another computer: empty data folder
        let home2 = tdir("home2");
        std::env::set_var("HOME", &home2);
        std::env::set_var("APPDATA", &home2);
        let (db2, rep) = read_ged_file(&ged).expect("import ok");
        assert_eq!(db2.people.len(), 2);
        assert!(rep.lost.is_empty(), "{:?}", rep.lost);
        assert_eq!((rep.media_found, rep.media_missing), (1, 0));
        let m = &db2.people.values().find(|p| p.given == "Grand").unwrap().media[0];
        assert!(!m.file.is_empty() && m.path.is_empty());
        assert_eq!(m.caption, "On the farm");
        assert_eq!(std::fs::read(mfile(&m.file)).unwrap(), b"not really a jpeg but bytes are bytes");

        // a photo that cannot be found is reported, and its name is kept
        std::fs::remove_file(out.join("tree-media").join("grandpa.jpg")).unwrap();
        let (db3, rep3) = read_ged_file(&ged).expect("import ok");
        assert_eq!((rep3.media_found, rep3.media_missing), (0, 1));
        assert_eq!(db3.people.values().find(|p| p.given == "Grand").unwrap().media[0].path, "tree-media/grandpa.jpg");
        assert!(to_ged(&db3).contains("2 FILE tree-media/grandpa.jpg"));
    }

    #[test]
    fn saved_tree_json_round_trips_every_new_field() {
        let text = "0 HEAD\n0 @S1@ SOUR\n1 TITL T\n0 @N1@ NOTE n\n0 @I1@ INDI\n1 NAME A /B/\n1 NAME C /D/\n2 TYPE aka\n1 BIRT\n2 DATE ABT 1900\n2 SOUR @S1@\n3 PAGE 4\n2 CAUS x\n1 NOTE @N1@\n1 _X y\n1 OBJE\n2 FILE a.jpg\n0 @F1@ FAM\n1 HUSB @I1@\n1 MARR\n2 TYPE civil\n1 DIV\n0 TRLR\n";
        let (db, _) = parse_ged_report(text);
        let dir = tdir("json");
        save_db_to(&dir, &db).unwrap();
        let back = read_db(&dir.join("tree.json")).unwrap();
        assert_eq!(to_ged(&back), to_ged(&db));
        assert!(loss_check(text, &to_ged(&back)).is_empty());
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let d = tdir("aw");
        let p = d.join("f.json");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"two");
        assert!(!with_suffix(&p, ".tmp").exists());
    }

    #[test]
    fn disk_full_during_write_keeps_old_file() {
        let d = tdir("full");
        let p = d.join("f.json");
        atomic_write(&p, b"good").unwrap();
        let code = if cfg!(windows) { 112 } else { 28 };
        let r = atomic_write_with(&p, b"brand new data", &|f: &mut std::fs::File, data: &[u8]| {
            f.write_all(&data[..4])?;
            Err(std::io::Error::from_raw_os_error(code))
        });
        let e = r.unwrap_err();
        assert!(is_disk_full(&e));
        assert_eq!(io_msg(&e), "the disk is full");
        assert_eq!(std::fs::read(&p).unwrap(), b"good");
        assert!(!with_suffix(&p, ".tmp").exists());
    }

    #[test]
    fn silently_short_write_is_caught_and_old_file_kept() {
        let d = tdir("short");
        let p = d.join("f.json");
        atomic_write(&p, b"good").unwrap();
        let r = atomic_write_with(&p, b"brand new data", &|f: &mut std::fs::File, data: &[u8]| f.write_all(&data[..data.len() / 2]));
        assert!(r.is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"good");
        assert!(!with_suffix(&p, ".tmp").exists());
    }

    #[test]
    fn crash_mid_save_leaves_partial_temp_that_is_ignored() {
        let d = tdir("crash");
        save_db_to(&d, &good_db()).unwrap();
        std::fs::write(d.join("tree.json.tmp"), br#"{"people":{"1":{"na"#).unwrap();
        let (db, note) = load_from(&d);
        assert_eq!(db.people.len(), good_db().people.len());
        assert!(note.is_empty());
        assert!(!d.join("tree.json.tmp").exists());
    }

    #[test]
    fn corrupt_main_recovers_from_previous_version() {
        let d = tdir("prev");
        save_db_to(&d, &good_db()).unwrap();
        save_db_to(&d, &good_db()).unwrap();
        std::fs::write(d.join("tree.json"), b"{ this is not json").unwrap();
        let (db, note) = load_from(&d);
        assert_eq!(db.people.len(), good_db().people.len());
        assert!(note.contains("recovered"));
        assert_eq!(damaged_files(&d).len(), 1);
    }

    #[test]
    fn corrupt_main_and_previous_recovers_from_dated_backup() {
        let d = tdir("dated");
        let db = good_db();
        save_db_to(&d, &db).unwrap();
        dated_backup_in(&d.join("backups"), &db, true).unwrap();
        std::fs::write(d.join("tree.json"), b"garbage").unwrap();
        std::fs::write(d.join("tree.json.prev"), b"").unwrap();
        let (got, note) = load_from(&d);
        assert_eq!(got.people.len(), db.people.len());
        assert!(note.contains("tree-"));
    }

    #[test]
    fn everything_corrupt_starts_empty_but_keeps_the_damaged_file() {
        let d = tdir("allbad");
        std::fs::write(d.join("tree.json"), b"garbage").unwrap();
        let (got, note) = load_from(&d);
        assert!(got.people.is_empty());
        assert!(note.contains("no backup"));
        assert!(!d.join("tree.json").exists());
        let kept = damaged_files(&d);
        assert_eq!(kept.len(), 1);
        assert_eq!(std::fs::read(&kept[0]).unwrap(), b"garbage");
    }

    #[test]
    fn fresh_install_is_empty_without_any_warning() {
        let d = tdir("fresh");
        let (got, note) = load_from(&d);
        assert!(got.people.is_empty());
        assert!(note.is_empty());
    }

    #[test]
    fn truncated_and_empty_files_are_rejected() {
        let json = serde_json::to_vec(&good_db()).unwrap();
        let d = tdir("trunc");
        let p = d.join("t.json");
        std::fs::write(&p, &json[..json.len() / 2]).unwrap();
        assert!(read_db(&p).is_err());
        std::fs::write(&p, b"").unwrap();
        assert!(read_db(&p).is_err());
    }

    #[test]
    fn a_damaged_main_file_is_never_copied_over_the_good_previous_version() {
        let d = tdir("keepprev");
        save_db_to(&d, &good_db()).unwrap();
        save_db_to(&d, &good_db()).unwrap();
        std::fs::write(d.join("tree.json"), b"junk").unwrap();
        save_db_to(&d, &good_db()).unwrap();
        assert!(read_db(&d.join("tree.json.prev")).is_ok());
        assert!(read_db(&d.join("tree.json")).is_ok());
    }

    #[test]
    fn backup_checksum_detects_silent_damage() {
        let d = tdir("sum");
        let name = "tree-2026-01-01.json";
        let json = serde_json::to_vec(&good_db()).unwrap();
        write_backup_in(&d, name, &json).unwrap();
        assert_eq!(verify_backup(&d.join(name)).unwrap(), good_db().people.len());
        let mut bytes = std::fs::read(d.join(name)).unwrap();
        let pos = bytes.windows(5).position(|w| w == b"Henry").unwrap();
        bytes[pos + 4] = b'i';
        std::fs::write(d.join(name), &bytes).unwrap();
        assert!(verify_backup(&d.join(name)).unwrap_err().contains("checksum"));
    }

    #[test]
    fn todays_damaged_backup_is_replaced() {
        let d = tdir("fix");
        let name = format!("tree-{}.json", today());
        std::fs::write(d.join(&name), b"junk").unwrap();
        assert!(dated_backup_in(&d, &good_db(), false).unwrap());
        assert!(verify_backup(&d.join(&name)).is_ok());
    }

    #[test]
    fn old_backups_are_pruned_to_fourteen() {
        let d = tdir("prune");
        let json = serde_json::to_vec(&good_db()).unwrap();
        for i in 1..=20 {
            write_backup_in(&d, &format!("tree-2026-01-{:02}.json", i), &json).unwrap();
        }
        prune_backups(&d);
        let names = list_backups(&d);
        assert_eq!(names.len(), 14);
        assert_eq!(names[0], "tree-2026-01-07.json");
        assert!(!d.join("tree-2026-01-01.json.fnv").exists());
    }

    #[test]
    fn media_mirror_copies_only_new_or_changed_files_and_skips_previews() {
        let src = tdir("msrc");
        let dst = tdir("mdst");
        std::fs::write(src.join("a.txt"), b"aaa").unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub").join("b.txt"), b"bbb").unwrap();
        std::fs::create_dir_all(src.join("previews")).unwrap();
        std::fs::write(src.join("previews").join("skip.jpg"), b"x").unwrap();
        assert_eq!(mirror_dir(&src, &dst, false, true).0, 2);
        assert!(!dst.join("a.txt").exists());
        let r = mirror_dir(&src, &dst, false, false);
        assert_eq!((r.0, r.1), (2, 0));
        assert!(dst.join("sub").join("b.txt").exists());
        assert!(!dst.join("previews").exists());
        assert_eq!(mirror_dir(&src, &dst, false, false).0, 0);
        std::fs::write(src.join("a.txt"), b"aaaa-changed").unwrap();
        assert_eq!(mirror_dir(&src, &dst, false, false).0, 1);
        std::fs::write(dst.join("a.txt"), b"changed in backup").unwrap();
        assert_eq!(mirror_dir(&dst, &src, true, false).0, 0);
    }
}

#[cfg(test)]
mod import_kinds {
    use super::*;
    fn tmp(name: &str, data: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("gr-{}-{}", std::process::id(), name));
        std::fs::write(&p, data).unwrap();
        p
    }
    #[test]
    fn json_tree_and_binary_files_are_recognised() {
        let mut db = Db::default();
        db.people.insert(1, Person { name: "Ann Test".into(), ..Default::default() });
        let p = tmp("t.json", serde_json::to_string(&db).unwrap().as_bytes());
        let (d, rep) = read_ged_file(&p).expect("json tree should load");
        assert_eq!(d.people.len(), 1);
        assert_eq!(rep.people, 1);
        let b = tmp("t.paf", &[0u8; 600]);
        let e = match read_ged_file(&b) { Err(e) => e, Ok(_) => panic!("binary accepted") };
        assert!(e.contains("Export"), "{}", e);
        let o = tmp("o.json", b"{\"a\":1}");
        assert!(read_ged_file(&o).is_err());
    }
}

#[cfg(test)]
mod data_move {
    use super::*;
    #[test]
    fn old_folder_is_copied_checked_and_marked() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("gr-move-{}-{}", std::process::id(), nanos));
        let (old, new) = (base.join("old"), base.join("Documents").join("Gilded Roots"));
        std::fs::create_dir_all(old.join("media")).unwrap();
        std::fs::write(old.join("tree.json"), b"{\"x\":1}").unwrap();
        std::fs::write(old.join("settings.json"), b"{}").unwrap();
        std::fs::write(old.join("media").join("a.jpg"), b"img").unwrap();
        assert!(migrate_old_data(&old, &new));
        assert_eq!(std::fs::read(new.join("tree.json")).unwrap(), b"{\"x\":1}");
        assert!(new.join("media").join("a.jpg").exists());
        assert!(!new.join("settings.json").exists());
        assert!(old.join("tree.json").exists(), "old copy must stay");
        assert!(old.join("MOVED.txt").exists());
        // Second call does nothing and does not fail.
        std::fs::remove_file(new.join("tree.json")).unwrap();
        assert!(migrate_old_data(&old, &new));
        assert!(!new.join("tree.json").exists(), "marked folders are not moved twice");
    }
}

#[cfg(test)]
mod sync_detect {
    use super::*;
    #[test]
    fn cloud_folders_are_flagged() {
        assert_eq!(synced_service(std::path::Path::new("/Users/a/Library/Mobile Documents/com~apple~CloudDocs/Gilded Roots")), Some("iCloud Drive"));
        assert_eq!(synced_service(std::path::Path::new("C:\\Users\\a\\OneDrive\\Documents\\Gilded Roots")), Some("OneDrive"));
        assert_eq!(synced_service(std::path::Path::new("/tmp/nothing-special/Gilded Roots")), None);
    }
}

#[cfg(test)]
mod citation_tests {
    use super::*;

    fn tree() -> (Db, u32, u32, u32) {
        let mut db = Db::default();
        let (a, b) = (db.alloc(), db.alloc());
        db.people.insert(a, Person { name: "Ann Test".into(), ..Default::default() });
        db.people.insert(b, Person { name: "Bob Test".into(), ..Default::default() });
        let repo = db.alloc();
        db.repos.insert(repo, RepoRec { name: "County Archive".into(), url: "https://archive.example".into(), ..Default::default() });
        let s = db.alloc();
        db.sources.insert(s, SourceRec { title: "Parish register".into(), author: "Church".into(), repo: Some(repo), call: "KB 4/2".into(), ..Default::default() });
        db.people.get_mut(&a).unwrap().sources.push(Source { sid: Some(s), about: "Birth".into(), page: "p. 3".into(), accessed: "5 Jan 2024".into(), ..Default::default() });
        db.people.get_mut(&b).unwrap().events.push(Event { kind: "Residence".into(), date: "1900".into(), cites: vec![Source { sid: Some(s), page: "p. 9".into(), ..Default::default() }], ..Default::default() });
        (db, a, s, repo)
    }

    #[test]
    fn one_source_is_used_by_people_and_events() {
        let (db, a, s, _) = tree();
        let uses = db.source_uses(s);
        assert_eq!(uses.len(), 2);
        assert_eq!(db.source_counts()[&s], 2);
        assert!(uses.iter().any(|u| u.0 == Owner::Person(a) && u.1 == "Birth" && u.3 == "p. 3"));
        assert!(uses.iter().any(|u| u.1 == "Residence" && u.2 == "1900"));
    }

    #[test]
    fn citation_text_gathers_source_page_repository_and_access_date() {
        let (db, a, _, _) = tree();
        let t = db.cite_text(&db.people[&a].sources[0]);
        assert_eq!(t, "Church, Parish register. p. 3. County Archive, KB 4/2 (accessed 5 Jan 2024)");
    }

    #[test]
    fn merging_sources_moves_every_citation_and_fills_gaps() {
        let (mut db, a, s, _) = tree();
        let dup = db.alloc();
        db.sources.insert(dup, SourceRec { title: "Parish register (copy)".into(), url: "https://x.example".into(), publ: "Oslo 1880".into(), ..Default::default() });
        db.people.get_mut(&a).unwrap().sources.push(Source { sid: Some(dup), page: "p. 7".into(), ..Default::default() });
        db.merge_sources(s, dup);
        assert!(!db.sources.contains_key(&dup));
        assert_eq!(db.source_counts()[&s], 3);
        let k = &db.sources[&s];
        assert_eq!((k.url.as_str(), k.publ.as_str(), k.title.as_str()), ("https://x.example", "Oslo 1880", "Parish register"));
        assert!(db.people[&a].sources.iter().all(|c| c.sid == Some(s)));
        assert_eq!(db.people[&a].sources[1].page, "p. 7", "page stays with the citation");
    }

    #[test]
    fn deleting_a_source_keeps_its_name_in_the_citations() {
        let (mut db, a, s, _) = tree();
        db.delete_source(s);
        let c = &db.people[&a].sources[0];
        assert_eq!((c.sid, c.text.as_str(), c.page.as_str()), (None, "Parish register", "p. 3"));
        assert!(to_ged(&db).contains("SOUR Parish register"));
    }

    #[test]
    fn deleting_a_repository_leaves_the_source() {
        let (mut db, _, s, repo) = tree();
        db.delete_repo(repo);
        assert!(db.sources[&s].repo.is_none());
        assert!(!to_ged(&db).contains("County Archive"));
    }

    #[test]
    fn everything_survives_save_and_reload_and_gedcom() {
        let (db, a, s, repo) = tree();
        let json = serde_json::to_string(&db).unwrap();
        let back: Db = serde_json::from_str(&json).unwrap();
        assert_eq!(back.repos[&repo].url, "https://archive.example");
        assert_eq!(back.people[&a].sources[0].accessed, "5 Jan 2024");
        let out = to_ged(&back);
        let (re, _) = parse_ged_report(&out);
        assert_eq!(re.repos.len(), 1);
        let rs = re.sources.values().next().unwrap();
        assert_eq!((rs.call.as_str(), rs.author.as_str()), ("KB 4/2", "Church"));
        assert_eq!(re.source_counts().values().sum::<usize>(), 2);
        let _ = s;
    }
}

#[cfg(test)]
mod relation_tests {
    use super::*;

    fn pers(db: &mut Db, name: &str, sex: &str) -> u32 {
        let id = db.alloc();
        db.people.insert(id, Person { name: name.into(), sex: sex.into(), ..Default::default() });
        id
    }
    fn fam(db: &mut Db, h: Option<u32>, w: Option<u32>, kids: &[u32], md: &str) -> u32 {
        let id = db.alloc();
        db.fams.insert(id, Family { h, w, kids: kids.to_vec(), md: md.into(), ..Default::default() });
        id
    }

    /// Kid is born to Ann and Bob, then adopted by Cal and Dee. Cal and Dee have their own child Eve.
    fn adopted_tree() -> (Db, u32, u32, u32, u32, u32, u32, u32) {
        let mut db = Db::default();
        let (bob, ann) = (pers(&mut db, "Bob Birth", "M"), pers(&mut db, "Ann Birth", "F"));
        let (cal, dee) = (pers(&mut db, "Cal Adopt", "M"), pers(&mut db, "Dee Adopt", "F"));
        let kid = pers(&mut db, "Kid", "M");
        let eve = pers(&mut db, "Eve", "F");
        let f1 = fam(&mut db, Some(bob), Some(ann), &[kid], "1950");
        let f2 = fam(&mut db, Some(cal), Some(dee), &[kid, eve], "1955");
        db.fams.get_mut(&f2).unwrap().pedi.insert(kid, "adopted".into());
        (db, bob, ann, cal, dee, kid, eve, f1.max(f2))
    }

    #[test]
    fn biological_and_legal_parents_are_kept_apart() {
        let (db, bob, _, cal, _, kid, _, _) = adopted_tree();
        assert_eq!(db.parent_families(kid).len(), 2);
        assert_eq!(db.parents(kid).0, Some(bob), "the chart shows the birth parents by default");
        assert_eq!(db.relationship(kid, bob), "Bob Birth is Kid's father.");
        assert_eq!(db.relationship(kid, cal), "Cal Adopt is Kid's adoptive father.");
        assert_eq!(db.relationship(cal, kid), "Kid is Cal Adopt's adopted son.");
    }

    #[test]
    fn a_preferred_family_changes_what_the_chart_shows_without_changing_relationships() {
        let (mut db, bob, _, cal, _, kid, _, _) = adopted_tree();
        let adoptive = db.parent_families(kid).into_iter().find(|f| db.fams[f].h == Some(cal)).unwrap();
        db.people.get_mut(&kid).unwrap().pref_parents = Some(adoptive);
        assert_eq!(db.parents(kid).0, Some(cal));
        assert_eq!(db.relationship(kid, bob), "Bob Birth is Kid's father.");
        assert_eq!(db.parent_links(kid).len(), 4);
    }

    #[test]
    fn an_adopted_sibling_is_not_called_a_blood_relative() {
        let (db, _, _, _, _, kid, eve, _) = adopted_tree();
        let r = db.relationship(kid, eve);
        assert!(r.contains("sister") && r.contains("not a blood relationship"), "{}", r);
    }

    #[test]
    fn step_and_half_relatives_are_worked_out_from_partnerships() {
        let mut db = Db::default();
        let (mum, dad) = (pers(&mut db, "Mum", "F"), pers(&mut db, "Dad", "M"));
        let kid = pers(&mut db, "Kid", "M");
        let (sue, tim) = (pers(&mut db, "Stepmum Sue", "F"), pers(&mut db, "Tim", "M"));
        let half = pers(&mut db, "Half", "F");
        fam(&mut db, Some(dad), Some(mum), &[kid], "1980");
        fam(&mut db, Some(dad), Some(sue), &[half], "1990");
        let sue_kid = pers(&mut db, "Sue's other child", "M");
        let sue_prev = pers(&mut db, "Sue's first partner", "M");
        fam(&mut db, Some(sue_prev), Some(sue), &[sue_kid], "1985");
        let _ = tim;
        let k = db.kin_extras(kid);
        assert_eq!(k.step_parents, vec![sue]);
        assert_eq!(k.half_siblings, vec![half]);
        assert_eq!(k.step_siblings, vec![sue_kid]);
        assert_eq!(db.relationship(kid, sue), "Stepmum Sue is Kid's stepmother.");
        assert_eq!(db.relationship(kid, half), "Half is Kid's half-sister.");
        assert_eq!(db.relationship(kid, sue_kid), "Sue's other child is Kid's stepbrother.");
        let d = db.kin_extras(dad);
        assert_eq!(d.step_children, vec![sue_kid]);
        assert_eq!(db.relationship(sue_kid, kid), "Kid is Sue's other child's stepbrother.");
    }

    #[test]
    fn partners_are_ordered_by_marriage_and_named_by_partnership_type() {
        let mut db = Db::default();
        let (a, b, c) = (pers(&mut db, "A", "M"), pers(&mut db, "B", "F"), pers(&mut db, "C", "F"));
        let late = fam(&mut db, Some(a), Some(c), &[], "1990");
        let early = fam(&mut db, Some(a), Some(b), &[], "1970");
        let none = fam(&mut db, Some(a), None, &[], "");
        assert_eq!(db.fams_of(a), vec![early, late, none]);
        db.fams.get_mut(&early).unwrap().kind = "Unmarried".into();
        assert_eq!(db.relationship(a, b), "B is A's partner.");
        db.fams.get_mut(&early).unwrap().events.push(Event { kind: "Divorce".into(), ..Default::default() });
        assert!(db.relationship(a, b).contains("former"));
        assert_eq!(db.relationship(a, c), "C is A's wife.");
    }

    #[test]
    fn a_guardian_is_found_when_there_is_no_family_link() {
        let mut db = Db::default();
        let (ward, g) = (pers(&mut db, "Ward", "F"), pers(&mut db, "Gus", "M"));
        db.people.get_mut(&ward).unwrap().assoc.push(Assoc { other: g, role: "Guardian".into(), note: String::new() });
        assert!(db.relationship(ward, g).contains("Guardian"));
        assert!(db.relationship(g, ward).contains("Guardian of") || db.relationship(g, ward).contains("the Guardian of"));
        assert_eq!(db.kin_extras(g).assoc_of, vec![(ward, "Guardian".to_string())]);
    }

    #[test]
    fn same_sex_parents_need_no_husband_and_wife() {
        let mut db = Db::default();
        let (a, b, kid) = (pers(&mut db, "Al", "M"), pers(&mut db, "Bo", "M"), pers(&mut db, "Kid", "F"));
        fam(&mut db, Some(a), Some(b), &[kid], "2010");
        assert_eq!(db.parent_links(kid).len(), 2);
        assert_eq!(db.relationship(kid, b), "Bo is Kid's father.");
        assert_eq!(db.relationship(a, b), "Bo is Al's husband.");
    }

    #[test]
    fn add_parent_and_remove() {
        let mut db = Db::default();
        let kid = pers(&mut db, "Kid", "M");
        let a = pers(&mut db, "A", "F");
        let b = pers(&mut db, "B", "F");
        db.add_parent(kid, None, false, a);
        let f1 = db.famc(kid).unwrap();
        db.add_parent(kid, Some(f1), false, b); // replaces slot in same family
        assert_eq!(db.fams[&f1].w, Some(b));
        db.add_parent(kid, None, true, a); // separate second set
        assert_eq!(db.parent_families(kid).len(), 2);
        let second = *db.parent_families(kid).iter().find(|f| **f != f1).unwrap();
        db.fams.get_mut(&second).unwrap().frel.insert(kid, "Adopted".into());
        assert!(db.child_note(second, kid).contains("adopted"));
        db.remove_from_family(kid, second);
        assert_eq!(db.parent_families(kid), vec![f1]);
        assert!(db.fams[&second].frel.is_empty());
    }

    #[test]
    fn checker_flags_complex_family_problems() {
        let mut db = Db::default();
        let kid = pers(&mut db, "Kid", "M");
        let (a, b, c, d) = (pers(&mut db, "A", "M"), pers(&mut db, "B", "F"), pers(&mut db, "C", "M"), pers(&mut db, "D", "F"));
        fam(&mut db, Some(a), Some(b), &[kid], "");
        assert!(!db.check().iter().any(|(_, m)| m.contains("biological parents")));
        let f2 = fam(&mut db, Some(c), Some(d), &[kid], "");
        assert!(db.check().iter().any(|(_, m)| m.contains("more than one set of biological")));
        db.fams.get_mut(&f2).unwrap().pedi.insert(kid, "adopted".into());
        assert!(!db.check().iter().any(|(_, m)| m.contains("more than one set of biological")));
        // loop: A becomes child of Kid
        fam(&mut db, Some(kid), None, &[a], "");
        assert!(db.check().iter().any(|(_, m)| m.contains("own ancestor")));
        // duplicate partnership
        fam(&mut db, Some(a), Some(b), &[], "");
        assert!(db.check().iter().any(|(_, m)| m.contains("two separate family groups")));
    }
}

#[cfg(test)]
mod scale_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn media_names_cannot_reach_outside_the_media_folder() {
        for bad in ["../../etc/passwd", "C:\\Windows\\notepad.exe", "/Applications/Calculator.app", "a/b.jpg", "x:y.jpg", ""] {
            assert!(!mfile(bad).starts_with(std::path::Path::new("/etc")));
            assert!(mfile(bad).starts_with(media_dir()), "{bad}");
            assert!(mprev(bad).starts_with(media_dir()), "{bad}");
            assert!(mfile(bad).ends_with("-missing-"), "{bad}");
        }
        assert_eq!(mfile("photo.jpg"), media_dir().join("photo.jpg"));
        assert!(!is_media_name("setup.exe") && !is_media_name("run.bat") && !is_media_name("x.app"));
    }

    #[test]
    fn removing_several_media_at_once() {
        let mut db = Db::default();
        let mut p = Person { name: "A".into(), ..Default::default() };
        for f in ["a.jpg", "b.jpg", "c.jpg", "d.pdf"] {
            p.media.push(Media { file: f.into(), ..Default::default() });
        }
        p.portrait = "c.jpg".into();
        db.people.insert(1, p);
        let sel: HashSet<(u32, usize)> = [(1, 0), (1, 2), (1, 9), (7, 0)].into_iter().collect();
        assert_eq!(remove_media_items(&mut db, &sel), 2);
        let left: Vec<&str> = db.people[&1].media.iter().map(|m| m.file.as_str()).collect();
        assert_eq!(left, vec!["b.jpg", "d.pdf"]);
        assert!(db.people[&1].portrait.is_empty(), "a removed profile photo is cleared");
    }

    #[test]
    #[ignore]
    fn snapshot_cost_like_users_tree() {
        let (mut db, _) = big_tree(4400);
        // about 5 citations, 1 event and 1 photo link per person, like an Ancestry import
        for p in db.people.values_mut() {
            for k in 0..5 {
                p.sources.push(Source { text: format!("1900 United States Federal Census, record {}", k), page: "Year: 1900; Census Place: Somewhere, Some County, Oregon; Roll: 1234; Page: 5; Enumeration District: 0067".into(), url: "https://www.ancestry.com/discoveryui-content/view/12345:7602".into(), extra: vec![Raw { at: String::new(), text: "1 _APID 1,7602::12345678\n".into() }], ..Default::default() });
            }
            p.events.push(Event { kind: "Residence".into(), date: "1910".into(), place: "Portland, Multnomah, Oregon, USA".into(), ..Default::default() });
            p.media.push(Media { path: "https://mediasvc.ancestry.com/v2/image/namespaces/1093/media/abc.jpg?Client=Trees".into(), form: "jpg".into(), caption: "Photo".into(), ..Default::default() });
        }
        let t = Instant::now();
        let j = serde_json::to_string(&db).unwrap();
        println!("json {} MB, serialise {:?}", j.len() / 1_000_000, t.elapsed());
        let t = Instant::now();
        let _ = j == j.clone();
        println!("compare {:?}", t.elapsed());
        let t = Instant::now();
        let z = pack(&j);
        println!("pack {:?} -> {} KB", t.elapsed(), z.len() / 1000);
        let t = Instant::now();
        assert_eq!(unpack(&z), j);
        println!("unpack {:?}", t.elapsed());
    }

    /// A realistic big tree: deep lines, large sibling groups, remarriages, adoptions.
    pub fn big_tree(n: usize) -> (Db, u32) {
        let mut db = Db::default();
        let surnames = ["Smith", "Jones", "Brown", "Taylor", "Wilson", "Davies", "Evans", "Thomas", "Johnson", "Roberts", "Walker", "Wright", "Robinson", "Thompson", "White", "Hughes", "Edwards", "Green", "Hall", "Wood"];
        let given = ["John", "Mary", "William", "Elizabeth", "James", "Sarah", "George", "Ann", "Thomas", "Jane", "Henry", "Margaret", "Charles", "Alice", "Robert", "Emma"];
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        let mut next = move |m: usize| -> usize {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng % m as u64) as usize
        };
        let mk = |db: &mut Db, sex: &str, year: i32, sn: &str, next: &mut dyn FnMut(usize) -> usize| -> u32 {
            let id = db.alloc();
            let g = given[next(given.len())];
            let name = format!("{} {}", g, sn);
            db.people.insert(id, Person { name: name.clone(), given: g.into(), surname: sn.into(), sex: sex.into(), bd: format!("{}", year), bp: "Somewhere".into(), ..Default::default() });
            id
        };
        // oldest generation: a few couples, then breed downward until n people exist
        let mut frontier: Vec<(u32, i32)> = Vec::new();
        let root = mk(&mut db, "M", 1700, "Smith", &mut next);
        frontier.push((root, 1700));
        let mut qi = 0;
        while db.people.len() < n && qi < frontier.len() {
            let (p, y) = frontier[qi];
            qi += 1;
            let sex = db.people[&p].sex.clone();
            let marriages = if next(6) == 0 { 2 } else { 1 };
            for _ in 0..marriages {
                if db.people.len() >= n {
                    break;
                }
                let sn = surnames[next(surnames.len())];
                let sp = mk(&mut db, if sex == "M" { "F" } else { "M" }, y + next(6) as i32 - 2, sn, &mut next);
                let fid = db.alloc();
                let (h, w) = if sex == "M" { (Some(p), Some(sp)) } else { (Some(sp), Some(p)) };
                let mut fam = Family { h, w, md: format!("{}", y + 22), ..Default::default() };
                let kids = 1 + next(6);
                for _ in 0..kids {
                    if db.people.len() >= n {
                        break;
                    }
                    let ksex = if next(2) == 0 { "M" } else { "F" };
                    let ky = y + 24 + next(12) as i32;
                    let hs = db.people[&h.unwrap()].surname.clone();
                    let k = mk(&mut db, ksex, ky, &hs, &mut next);
                    fam.kids.push(k);
                    if next(25) == 0 {
                        fam.pedi.insert(k, "adopted".into());
                    }
                    frontier.push((k, ky));
                }
                db.fams.insert(fid, fam);
            }
        }
        (db, root)
    }

    fn t<R>(label: &str, f: impl FnOnce() -> R) -> (R, f64) {
        let s = Instant::now();
        let r = f();
        let ms = s.elapsed().as_secs_f64() * 1000.0;
        println!("  {:<34} {:>9.1} ms", label, ms);
        (r, ms)
    }

    #[test]
    #[ignore]
    fn timings() {
        for n in [1000usize, 5000, 20000] {
            let (db, root) = big_tree(n);
            println!("{} people, {} families", db.people.len(), db.fams.len());
            let ids: Vec<u32> = db.people.keys().copied().step_by((db.people.len() / 200).max(1)).collect();
            t("index build (per frame)", || db.index().child_of.len());
            t("famc x200", || ids.iter().for_each(|i| { db.famc(*i); }));
            t("fams_of x200", || ids.iter().for_each(|i| { db.fams_of(*i); }));
            let deep = *db.people.keys().last().unwrap();
            t("ancestors_by (deepest)", || db.ancestors_by(deep, false).len());
            t("kin_extras x200", || ids.iter().for_each(|i| { db.kin_extras(*i); }));
            t("relationship (root, deepest)", || db.relationship(root, deep).len());
            t("descendants layout (8 gens)", || {
                let mut nodes = Vec::new();
                let mut leaf = 0.0;
                let mut seen: HashSet<u32> = HashSet::from([root]);
                let ix = db.index();
                desc_layout(&db, &ix, root, 0, 8, None, &mut nodes, &mut leaf, &mut seen);
                nodes.len()
            });
            t("sorted(\"\") list", || db.sorted("").len());
            t("sorted(\"smith\") search", || db.sorted("smith").len());
            t("data checker", || db.check().len());
            t("duplicate finder", || db.duplicate_pairs().len());
            let (json, _) = t("save to JSON", || serde_json::to_string(&db).unwrap());
            t("load from JSON", || serde_json::from_str::<Db>(&json).unwrap().people.len());
            let (ged, _) = t("export GEDCOM", || gedcom::to_ged(&db));
            t("import GEDCOM", || gedcom::parse_ged_report(&ged).0.people.len());
        }
    }

    #[test]
    fn index_answers_match_the_slow_scans() {
        let (db, _) = big_tree(3000);
        let ix = db.index();
        for id in db.people.keys().step_by(7) {
            assert_eq!(db.parent_families_ix(&ix, *id), db.parent_families(*id));
            assert_eq!(db.fams_of_ix(&ix, *id).to_vec(), db.fams_of(*id));
            assert_eq!(db.parents_ix(&ix, *id), db.parents(*id));
        }
    }

    #[test]
    fn big_trees_stay_fast() {
        // Generous limits so slow machines and debug builds still pass; the old code took minutes.
        let (db, root) = big_tree(5000);
        let limit = 15.0;
        let (_, ms) = t("checker", || db.check().len());
        assert!(ms < limit * 1000.0, "data checker took {} ms", ms);
        let (_, ms) = t("duplicates", || db.duplicate_pairs().len());
        assert!(ms < limit * 1000.0, "duplicate finder took {} ms", ms);
        let (n, ms) = t("descendants", || {
            let mut nodes = Vec::new();
            let mut leaf = 0.0;
            let mut seen: HashSet<u32> = HashSet::from([root]);
            let ix = db.index();
            desc_layout(&db, &ix, root, 0, 8, None, &mut nodes, &mut leaf, &mut seen);
            nodes.len()
        });
        assert!(n > 100 && ms < limit * 1000.0);
        let (_, ms) = t("search", || db.sorted("smith").len());
        assert!(ms < limit * 1000.0);
    }

    #[test]
    fn navigation_helpers_find_family_members() {
        let mut db = Db::default();
        let mk = |db: &mut Db, n: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), ..Default::default() });
            id
        };
        let (dad, mum, step) = (mk(&mut db, "Dad"), mk(&mut db, "Mum"), mk(&mut db, "Stepmum"));
        let (a, b, c, half) = (mk(&mut db, "A"), mk(&mut db, "B"), mk(&mut db, "C"), mk(&mut db, "Half"));
        let f1 = db.alloc();
        db.fams.insert(f1, Family { h: Some(dad), w: Some(mum), kids: vec![a, b, c], ..Default::default() });
        let f2 = db.alloc();
        db.fams.insert(f2, Family { h: Some(dad), w: Some(step), kids: vec![half], ..Default::default() });
        assert_eq!(db.siblings(b), vec![a, c, half]);
        assert_eq!(db.partners(dad), vec![mum, step]);
        assert_eq!(db.children_of(dad), vec![a, b, c, half]);
        assert!(db.children_of(a).is_empty());
        assert!(db.siblings(dad).is_empty());
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;

    fn tiny() -> Db {
        let mut db = Db::default();
        for n in ["Ann Lee", "Bob Lee"] {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), given: n.split(' ').next().unwrap().into(), surname: "Lee".into(), ..Default::default() });
        }
        let f = db.alloc();
        let ids: Vec<u32> = db.people.keys().copied().collect();
        db.fams.insert(f, Family { h: Some(ids[1]), w: Some(ids[0]), ..Default::default() });
        db
    }

    #[test]
    fn pedigree_double_click_opens_person() {
        let mut app = App::for_test(tiny());
        let root = *app.db.people.keys().next().unwrap();
        app.sel = Some(root);
        app.view = View::Pedigree;
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0));
        let frame = |app: &mut App, events: Vec<egui::Event>, t: f64| -> Option<Action> {
            let mut out = None;
            let input = egui::RawInput { screen_rect: Some(screen), events, time: Some(t), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut act = None;
                    app.pedigree(ui, &mut act);
                    out = act;
                });
            });
            out
        };
        // give the root parents and grandparents so outer boxes exist too
        let mk = |db: &mut Db, n: &str, sx: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), sex: sx.into(), portrait: String::new(), ..Default::default() });
            id
        };
        let (f, m) = (mk(&mut app.db, "Dad Lee", "M"), mk(&mut app.db, "Mum Lee", "F"));
        let (gf, gm) = (mk(&mut app.db, "Grandad Lee", "M"), mk(&mut app.db, "Gran Lee", "F"));
        let f1 = app.db.alloc();
        app.db.fams.insert(f1, Family { h: Some(f), w: Some(m), kids: vec![root], ..Default::default() });
        let f2 = app.db.alloc();
        app.db.fams.insert(f2, Family { h: Some(gf), w: Some(gm), kids: vec![f], ..Default::default() });
        frame(&mut app, vec![], 0.0);
        let mut t = 1.0;
        for (g, i, who) in [(1usize, 0usize, f), (1, 1, m), (2, 0, gf), (2, 1, gm)] {
            let r = ctx.read_response(egui::Id::new(("ped", g, i))).expect("box drawn");
            let p = pos2(r.rect.min.x + r.rect.height() + 30.0, r.rect.center().y);
            let press = |down: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
            let mut got = Vec::new();
            for (ev, dt) in [(vec![egui::Event::PointerMoved(p)], 0.0), (vec![press(true)], 0.1), (vec![press(false)], 0.15), (vec![press(true)], 0.2), (vec![press(false)], 0.25)] {
                got.push(frame(&mut app, ev, t + dt));
            }
            t += 2.0;
            assert!(got.iter().any(|a| matches!(a, Some(Action::Open(x)) if *x == who)), "box {:?} did not open", (g, i));
        }
        let r = ctx.read_response(egui::Id::new(("ped", 0usize, 0usize))).expect("root box drawn");
        // click on the name, a little right of the portrait
        let p = pos2(r.rect.min.x + r.rect.height() + 30.0, r.rect.center().y);
        let press = |down: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
        frame(&mut app, vec![egui::Event::PointerMoved(p)], t);
        let a1 = frame(&mut app, vec![press(true)], t + 0.2);
        let a2 = frame(&mut app, vec![press(false)], t + 0.25);
        let a3 = frame(&mut app, vec![press(true)], t + 0.3);
        let a4 = frame(&mut app, vec![press(false)], t + 0.35);
        let all = [a1, a2, a3, a4];
        let names: Vec<String> = all.iter().map(|a| match a { Some(Action::Open(_)) => "open".into(), Some(Action::Highlight(_)) => "hl".into(), Some(_) => "other".into(), None => "-".into() }).collect();
        assert!(all.iter().any(|a| matches!(a, Some(Action::Open(i)) if *i == root)), "got {:?}", names);
    }

    #[test]
    fn image_addresses_keep_spaces() {
        let p = std::path::Path::new("/Users/me/Documents/Gilded Roots/media/previews/a b.jpg");
        assert_eq!(img_uri(p), "file:///Users/me/Documents/Gilded Roots/media/previews/a b.jpg");
    }

    #[test]
    fn f1_and_quick_jump_keys_work() {
        let mut app = App::for_test(tiny());
        let ctx = egui::Context::default();
        let key = |k: egui::Key, m: egui::Modifiers| egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m };
        for (ev, want) in [(key(egui::Key::F1, egui::Modifiers::NONE), true), (key(egui::Key::F1, egui::Modifiers::NONE), false), (key(egui::Key::Slash, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT), true)] {
            let input = egui::RawInput { events: vec![ev], modifiers: Default::default(), ..Default::default() };
            let _ = ctx.run(input, |ctx| app.global_keys(ctx, &mut None));
            assert_eq!(app.help_open, want);
        }
        // Ctrl/Cmd+K opens quick jump, typing a name finds the person, Enter selects them
        let target = *app.db.people.iter().find(|(_, p)| p.name == "Bob Lee").unwrap().0;
        let mut act = None;
        let input = egui::RawInput { events: vec![key(egui::Key::K, egui::Modifiers::COMMAND)], ..Default::default() };
        let _ = ctx.run(input, |ctx| app.jump_window(ctx, &mut act));
        assert!(app.x.jump_open);
        let r = app.jump_results("bob");
        assert!(matches!(r.first(), Some((_, k, Jump::Act(Action::Select(i)))) if k == "person" && *i == target));
        assert!(app.jump_results("pedigree").iter().any(|x| x.0 == "Pedigree view"));
    }

    #[test]
    fn places_are_listed_and_renamed_everywhere() {
        let mut db = tiny();
        let ids: Vec<u32> = db.people.keys().copied().collect();
        db.people.get_mut(&ids[0]).unwrap().bp = "Portland, Oregon, USA".into();
        db.people.get_mut(&ids[1]).unwrap().dp = "portland, oregon".into();
        let f = *db.fams.keys().next().unwrap();
        db.fams.get_mut(&f).unwrap().mp = "Portland, Oregon, USA".into();
        let uses = db.place_uses();
        assert_eq!(uses["Portland, Oregon, USA"].len(), 3); // birth + marriage for both partners
        assert_eq!(place_key("Portland, Oregon, USA"), place_key("portland, oregon"));
        assert_eq!(db.rename_place("portland, oregon", "Portland, Oregon, USA"), 1);
        assert_eq!(db.place_uses().len(), 1);
    }

    #[test]
    fn face_tags_follow_people_and_crop_without_stretching() {
        // a tall box on a wide picture grows sideways to fill a square frame
        let uv = crop_uv([0.4, 0.2, 0.1, 0.3], 2.0, 1.0);
        assert!((uv.width() * 2.0 - uv.height()).abs() < 1e-4, "{:?}", uv);
        assert!(uv.min.x >= 0.0 && uv.max.x <= 1.0 && uv.min.y >= 0.0 && uv.max.y <= 1.0);
        let mut app = App::for_test(tiny());
        let ids: Vec<u32> = app.db.people.keys().copied().collect();
        let (owner, tagged) = (ids[0], ids[1]);
        app.db.people.get_mut(&owner).unwrap().media.push(Media { file: "x.jpg".into(), faces: vec![Face { who: tagged, r: [0.1, 0.1, 0.2, 0.2] }], ..Default::default() });
        app.db.people.get_mut(&tagged).unwrap().face_portrait = Some(FacePortrait { owner, file: "x.jpg".into(), r: [0.1, 0.1, 0.2, 0.2] });
        assert_eq!(app.tagged_in(tagged).len(), 1);
        // deleting the tagged person removes the tag
        replace_pointer(&mut app.db, &format!("I{}", tagged), None);
        assert!(app.db.people[&owner].media[0].faces.is_empty());
        // tags survive save and reload
        app.db.people.get_mut(&owner).unwrap().media[0].faces.push(Face { who: owner, r: [0.0, 0.0, 0.5, 0.5] });
        let j = serde_json::to_string(&app.db).unwrap();
        let back: Db = serde_json::from_str(&j).unwrap();
        assert_eq!(back.people[&owner].media[0].faces.len(), 1);
    }

    #[test]
    fn every_view_and_window_draws_on_a_big_tree() {
        let (db, root) = scale_tests::big_tree(3000);
        let mut app = App::for_test(db);
        app.sel = Some(root);
        // a deeper person, so the fan has ancestors
        let deep = *app.db.people.keys().max().unwrap();
        app.db.people.get_mut(&deep).unwrap().todos.push(Todo { text: "Find census".into(), ..Default::default() });
        app.db.bookmarks.push(deep);
        app.x.todo_open = true;
        app.x.places_open = true;
        app.x.history_open = true;
        app.x.ver_open = true;
        app.x.jump_open = true;
        app.help_open = true;
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        for sel in [root, deep] {
            app.sel = Some(sel);
            for v in [View::Pedigree, View::Family, View::Descendants, View::Person, View::Gallery, View::Charts, View::Table, View::Map, View::Memories, View::Dna] {
                for chart in [ChartKind::Fan, ChartKind::Hourglass, ChartKind::Lifespans] {
                    app.view = v;
                    app.x.chart = chart;
                    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
                    let _ = ctx.run(input, |ctx| {
                        let mut act = None;
                        egui::CentralPanel::default().show(ctx, |ui| match v {
                            View::Pedigree => app.pedigree(ui, &mut act),
                            View::Family => app.family_view(ui, &mut act),
                            View::Descendants => app.descendants(ui, &mut act),
                            View::Person => app.person_view(ui, &mut act),
                            View::Gallery => app.gallery_view(ui, &mut act),
                            View::Charts => app.charts_view(ui, &mut act),
                            View::Table => app.table_view(ui, &mut act),
                            View::Map => app.map_view(ui, &mut act),
                            View::Memories => app.memories_view(ui, &mut act),
                            View::Dna => app.dna_view(ui, &mut act),
                        });
                        app.jump_window(ctx, &mut act);
                        app.todo_window(ctx, &mut act);
                        app.places_window(ctx, &mut act);
                        app.history_window(ctx);
                        app.versions_window(ctx);
                    });
                    if v != View::Charts {
                        break;
                    }
                }
            }
        }
        assert!(app.chart_page().is_some());
        // the table sorts and filters
        app.x.table_col = table::Col::Born;
        let rows = app.table_rows();
        assert_eq!(rows.len(), app.db.people.len());
        app.x.table_only = table::Only::OpenTasks;
        assert_eq!(app.table_rows().len(), 1);
        app.x.table_only = table::Only::Bookmarked;
        assert_eq!(app.table_rows()[0].id, deep);
    }

    #[test]
    fn history_describes_changes_and_undoes_them() {
        let mut app = App::for_test(tiny());
        let id = *app.db.people.keys().next().unwrap();
        app.sel = Some(id);
        app.remember_state();
        app.db.people.get_mut(&id).unwrap().bd = "1900".into();
        app.last_snap = std::time::Instant::now() - std::time::Duration::from_secs(5);
        app.track();
        assert_eq!(app.undo.len(), 1);
        assert!(app.undo[0].label.starts_with("Changed "), "{}", app.undo[0].label);
        let nid = app.db.alloc();
        app.db.people.insert(nid, Person { name: "New One".into(), ..Default::default() });
        app.last_snap = std::time::Instant::now() - std::time::Duration::from_secs(5);
        app.track();
        assert!(app.undo[1].label.starts_with("Added New One"), "{}", app.undo[1].label);
        app.undo_action();
        app.undo_action();
        assert!(app.db.people[&id].bd.is_empty());
        assert!(!app.db.people.contains_key(&nid));
        app.redo_action();
        assert_eq!(app.db.people[&id].bd, "1900");
    }

    #[test]
    fn name_and_date_styles_only_change_the_display() {
        let p = Person { name: "John Henry Smith".into(), given: "John Henry".into(), surname: "Smith".into(), bd: "12 Mar 1921".into(), ..Default::default() };
        assert_eq!(styled_name_as(&p, 1), "SMITH, John Henry");
        assert_eq!(styled_name_as(&p, 2), "Smith, John Henry");
        assert_eq!(styled_name_as(&p, 0), "John Henry Smith");
        assert_eq!(show_date_as("12 Mar 1921", 2), "March 12, 1921");
        assert_eq!(show_date_as("about 1850", 2), "about 1850");
        assert_eq!(show_date_as("12 MAR 1921", 3), "1921-03-12");
        assert_eq!(show_date_as("12 Mar 1921", 0), "12 Mar 1921");
        assert_eq!(p.bd, "12 Mar 1921");
    }

    #[test]
    fn pdf_viewer_reads_and_draws_pages() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests_data/two_pages.pdf");
        let v = pdfview::PdfView::open(&path, "Certificate").expect("opens");
        assert_eq!(v.pages, 2);
        let bytes = std::sync::Arc::new(std::fs::read(&path).unwrap());
        let (_, img) = pdfview::render_page(&bytes, 0, 1.0).expect("draws");
        assert_eq!(img.size, [612, 792]);
        // white paper with the filled black rectangle (300 x 200 points) and the text drawn on it
        let dark = img.pixels.iter().filter(|c| c.r() < 40 && c.g() < 40 && c.b() < 40).count();
        let white = img.pixels.iter().filter(|c| c.r() > 245 && c.g() > 245 && c.b() > 245).count();
        assert!((60_000..80_000).contains(&dark), "{} dark pixels", dark);
        assert!(white > 350_000, "only {} white pixels", white);
        let img_bytes: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
        let _ = image::RgbImage::from_raw(612, 792, img_bytes).unwrap().save("/tmp/pdf_page1.png");
        // a file that is not a PDF gives a plain message instead of a crash
        let bad = std::env::temp_dir().join("not_a.pdf");
        std::fs::write(&bad, b"hello").unwrap();
        assert!(pdfview::PdfView::open(&bad, "x").is_err());
        // drawing the viewer window does not panic
        let mut v = v;
        let ctx = egui::Context::default();
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                v.show(ctx);
            });
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }

    #[test]
    #[ignore]
    fn write_fan_chart_svg_for_a_look() {
        let (db, _) = scale_tests::big_tree(800);
        let mut app = App::for_test(db);
        let deep = *app.db.people.keys().max().unwrap();
        app.sel = Some(deep);
        app.x.chart = ChartKind::Fan;
        app.x.fan_gens = 6;
        for deg in [240.0, 360.0] {
            app.x.fan_deg = deg;
            let (title, body) = app.chart_page().unwrap();
            std::fs::write(format!("/tmp/fan{}.html", deg as u32), page(&title, &body, true)).unwrap();
        }
    }

    #[test]
    #[ignore]
    fn write_print_pages_for_a_look() {
        let (mut db, _) = scale_tests::big_tree(300);
        let id = *db.people.keys().find(|k| !db.fams_of(**k).is_empty() && db.parents(**k).0.is_some()).unwrap();
        {
            let p = db.people.get_mut(&id).unwrap();
            p.bd = "Jan 25 1975".into();
            p.bp = "Salt Lake City, Salt Lake, Utah, USA".into();
            p.notes = "A long note about this person that goes on for a while to see how notes wrap on paper.".into();
            for k in 0..3 {
                p.sources.push(Source { about: "Birth".into(), text: "Ancestro.example, U.S., Newspapers Obituary Index, 1800s-current. The Marysville Advocate; Publication Date: 28 May 1981; URL: https://www.newspapers.example/image/345662540/?article=f03a0f47-8903-45c1-a57e-d43e1619f152&focus=0.34905595,0.03336283,0.49968433,0.2853241&xid=3355".into(), ..Default::default() });
                let _ = k;
            }
        }
        let mut app = App::for_test(db);
        app.settings.hide_living = false;
        app.sel = Some(id);
        std::fs::write("/tmp/p_family.html", app.family_html(id)).unwrap();
        std::fs::write("/tmp/p_pedigree.html", app.pedigree_html(id)).unwrap();
        std::fs::write("/tmp/p_ancestors.html", app.ancestors_html(id)).unwrap();
        std::fs::write("/tmp/p_book.html", app.book_html(id)).unwrap();
    }

    #[test]
    fn map_finds_places_the_way_people_write_them() {
        let g = mapview::geo();
        let near = |p: &str, lon: f32, lat: f32| {
            let (ll, pr) = g.locate(p).unwrap_or_else(|| panic!("{} not found", p));
            assert!((ll[0] - lon).abs() < 0.6 && (ll[1] - lat).abs() < 0.6, "{} -> {:?}", p, ll);
            pr
        };
        assert!(near("Eugene, Lane, Oregon, USA", -123.09, 44.05) == mapview::Prec::Town);
        near("North Platte, Lincoln, Nebraska, USA", -100.77, 41.12);
        near("Salt Lake City, Salt Lake, Utah, USA", -111.89, 40.76);
        near("Springfield, Lane, Oregon", -123.02, 44.05);
        near("Portland, Oregon", -122.68, 45.52);
        near("Portland, Maine, United States", -70.26, 43.66);
        near("Paris, France", 2.35, 48.85);
        near("London, England", -0.13, 51.51);
        near("St. Louis, Missouri", -90.2, 38.63);
        // a farm that is not a town: the state's middle is used
        assert!(near("Old Smith Farm, Utah, USA", -111.7, 39.3) == mapview::Prec::State);
        assert!(g.locate("Nowhereville").is_none());
    }

    #[test]
    fn map_and_memories_draw() {
        let (db, root) = scale_tests::big_tree(500);
        let mut app = App::for_test(db);
        app.sel = Some(root);
        for p in app.db.people.values_mut() {
            p.bp = "Eugene, Lane, Oregon, USA".into();
            p.notes = "A note".into();
        }
        let ctx = egui::Context::default();
        for who in 0..4u8 {
            app.x.map_who = who;
            for v in [View::Map, View::Memories, View::Gallery] {
                let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
                let _ = ctx.run(input, |ctx| {
                    let mut act = None;
                    egui::CentralPanel::default().show(ctx, |ui| match v {
                        View::Map => app.map_view(ui, &mut act),
                        View::Memories => app.memories_view(ui, &mut act),
                        _ => app.gallery_view(ui, &mut act),
                    });
                });
            }
        }
        assert!(!app.map_pins().is_empty());
    }

    #[test]
    fn damaged_gedcom_files_never_crash_the_import() {
        let base = "0 HEAD\n1 CHAR UTF-8\n0 @I1@ INDI\n1 NAME John /Smith/\n1 BIRT\n2 DATE 12 MAR 1900\n2 PLAC Eugene, Oregon\n1 FAMS @F1@\n1 OBJE\n2 FILE ../../etc/passwd\n0 @I2@ INDI\n1 NAME Mary /Jones/\n1 FAMS @F1@\n0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 CHIL @I3@\n1 MARR\n2 DATE ABT 1925\n0 @S1@ SOUR\n1 TITL Census\n0 TRLR\n";
        let bytes = base.as_bytes();
        let mut rng: u64 = 0x2545F4914F6CDD1D;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for _ in 0..3000 {
            let mut b = bytes.to_vec();
            for _ in 0..(next() % 12 + 1) {
                let i = (next() as usize) % b.len();
                match next() % 4 {
                    0 => b[i] = (next() % 256) as u8,
                    1 => {
                        b.remove(i);
                    }
                    2 => {
                        let set = b"@0123 \n\r\t/:";
                        b.insert(i, set[(next() as usize) % set.len()])
                    }
                    _ => {
                        let j = (next() as usize) % b.len();
                        let chunk: Vec<u8> = b[i.min(j)..i.max(j)].to_vec();
                        b.splice(i..i, chunk);
                    }
                }
                if b.is_empty() {
                    b.push(b'0');
                }
            }
            let (text, _) = gedcom::decode_ged(&b);
            let (db, _) = gedcom::parse_ged_report(&text);
            let _ = gedcom::to_ged(&db);
            let _ = db.check();
        }
        // a GEDCOM that names a private file is not copied in
        let (mut db, mut rep) = gedcom::parse_ged_report(base);
        resolve_media(&mut db, std::path::Path::new("/tmp"), &mut rep);
        assert!(db.people.values().all(|p| p.media.iter().all(|m| m.file.is_empty())));
        assert!(!is_media_name("passwd") && is_media_name("grandma.JPG"));
        assert!(safe_media_path("../x.txt").is_none() && safe_media_path("a/b.txt").is_none() && safe_media_path("123-story.txt").is_some());
    }

    #[test]
    fn archive_keeps_everything_and_rejects_bad_files() {
        let mut db = sample_family();
        db.journal.push(Journal { goal: "x".into(), ..Default::default() });
        let dir = std::env::temp_dir().join(format!("gr-arch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test archive.zip");
        let info = archive::write_archive(&db, &path).expect("archive written");
        assert_eq!(info.people, db.people.len());
        assert!(info.media >= 8, "media {}", info.media);
        let back = archive::read_info(&path).unwrap();
        assert_eq!(back.people, db.people.len());
        let (db2, n) = archive::read_archive(&path).unwrap();
        assert_eq!(serde_json::to_string(&db2).unwrap(), serde_json::to_string(&db).unwrap());
        assert!(n >= 8);
        // the GEDCOM inside reads back with everyone in it
        let mut z = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut ged = String::new();
        std::io::Read::read_to_string(&mut z.by_name("family.ged").unwrap(), &mut ged).unwrap();
        assert_eq!(gedcom::parse_ged(&ged).people.len(), db.people.len());
        // not a zip, and a zip with a damaged tree, are refused with a message
        std::fs::write(dir.join("bad.zip"), b"hello").unwrap();
        assert!(archive::read_info(&dir.join("bad.zip")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn practice_mode_never_touches_the_real_tree() {
        let mut app = App::for_test(tiny());
        let before = serde_json::to_string(&app.db).unwrap();
        app.start_practice();
        assert!(app.db.people.len() >= 20);
        assert!(app.db.people.values().any(|p| !p.media.is_empty()));
        assert!(!app.db.journal.is_empty());
        // edits in practice are thrown away
        let some = *app.db.people.keys().next().unwrap();
        app.db.people.get_mut(&some).unwrap().name = "Changed".into();
        assert!(app.persist());
        app.end_practice();
        assert_eq!(serde_json::to_string(&app.db).unwrap(), before);
        // and the welcome screen draws
        let mut app = App::for_test(Db::default());
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut act = None;
                app.welcome(ui, &mut act);
            });
        });
    }

    #[test]
    fn confidence_journal_and_history_round_trip() {
        let mut db = sample_family();
        let t = *db.people.iter().find(|(_, p)| p.name == "Thomas James Harwood").unwrap().0;
        assert_eq!(db.people[&t].fx["DEAT"].conf, "confirmed");
        let ged = gedcom::to_ged(&db);
        assert!(ged.contains("_CONF confirmed"), "confidence is exported");
        let back = gedcom::parse_ged(&ged);
        let t2 = back.people.values().find(|p| p.name == "Thomas James Harwood").unwrap();
        assert_eq!(t2.fx["DEAT"].conf, "confirmed");
        assert!(t2.events.iter().any(|e| e.conf == "confirmed"));
        // journal and history survive saving
        db.history.push((1911, 1911, "Mine fire".into(), String::new()));
        let j: Db = serde_json::from_str(&serde_json::to_string(&db).unwrap()).unwrap();
        assert_eq!(j.journal.len(), db.journal.len());
        // timeline history: a US-born man of 1898-1975 sees both world wars and the depression
        let app = App::for_test(j);
        let h = app.history_for(&app.db.people[&t]);
        assert!(h.iter().any(|x| x.1.contains("First World War")));
        assert!(h.iter().any(|x| x.1.contains("Great Depression")));
        assert!(!h.iter().any(|x| x.1.contains("Irish")));
        assert!(!h.iter().any(|x| x.1.contains("Mayflower")));
    }

    #[test]
    fn duplicates_found_through_nicknames_and_spellings() {
        let mut db = Db::default();
        let mk = |db: &mut Db, g: &str, s: &str, bd: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: format!("{} {}", g, s), given: g.into(), surname: s.into(), bd: bd.into(), bp: "Leeds, England".into(), ..Default::default() });
            id
        };
        let a = mk(&mut db, "William", "Smith", "1850");
        let b = mk(&mut db, "Bill", "Smyth", "1850");
        let c = mk(&mut db, "Margaret", "Jones", "1901");
        let d = mk(&mut db, "Peggy", "Jones", "1902");
        let e = mk(&mut db, "Jonathan", "Brown", "1900");
        let f = mk(&mut db, "Jonathon", "Brown", "1900");
        let _other = mk(&mut db, "Mary", "Smith", "1850");
        let pairs = db.duplicate_pairs();
        let has = |x: u32, y: u32| pairs.iter().any(|p| (p.0 == x && p.1 == y) || (p.0 == y && p.1 == x));
        assert!(has(a, b), "{:?}", pairs);
        assert!(has(c, d));
        assert!(has(e, f));
        assert_eq!(sound_key("Smith"), sound_key("Smyth"));
        assert_eq!(canon_given("Bill"), "william");
    }

    #[test]
    fn statistics_designer_and_trees() {
        let db = sample_family();
        let st = stats::compute(&db);
        assert!(st.life_men.1 > 0 && st.surnames.iter().any(|s| s.0 == "Harwood"));
        assert!(st.moved.1 > 0);
        let mut app = App::for_test(db);
        app.sel = app.db.people.iter().find(|(_, p)| p.name == "Emily Rose Harwood").map(|(k, _)| *k);
        let mut d = designer::Design { who: 1, ..Default::default() };
        d.cols.push("father".into());
        d.hide_living = false;
        let html = app.design_html(&d);
        assert!(html.contains("Thomas James Harwood") && html.contains("James Harwood"));
        assert!(!html.contains("Ruth Ellen Harwood"), "ancestors only");
        let csv = app.design_csv(&d);
        assert!(csv.lines().count() >= 10);
        assert_eq!(sanitize_tree_name("../Mom's side: Jensen"), "Mom's side Jensen");
        assert_eq!(sanitize_tree_name("  .. "), "");
    }

    #[test]
    fn sibling_and_quick_edit() {
        let mut app = App::for_test(tiny());
        let ids: Vec<u32> = app.db.people.keys().copied().collect();
        let me = ids[0];
        let mut d = Dlg::new(Kind::Sibling, "M", Some(me));
        d.name = "Sib Lee".into();
        app.create(d);
        let sib = *app.db.people.iter().find(|(_, p)| p.name == "Sib Lee").unwrap().0;
        assert!(app.db.siblings(me).contains(&sib));
        let q = QuickEdit { id: sib, given: "Sam".into(), surname: "Lee".into(), sex: "M".into(), bd: "1 Jan 1900".into(), bp: String::new(), dd: String::new(), dp: String::new(), living: false };
        app.apply_quick(&q);
        assert_eq!(app.db.people[&sib].name, "Sam Lee");
        assert_eq!(app.db.people[&sib].bd, "1 Jan 1900");
    }

    #[test]
    fn pedigree_and_gallery_draw_headless() {
        let mut app = App::for_test(tiny());
        app.sel = app.db.people.keys().next().copied();
        for v in [View::Pedigree, View::Gallery, View::Descendants] {
            app.view = v;
            let ctx = egui::Context::default();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut act = None;
                    match v {
                        View::Pedigree => app.pedigree(ui, &mut act),
                        View::Gallery => app.gallery_view(ui, &mut act),
                        _ => app.descendants(ui, &mut act),
                    }
                });
            });
        }
    }

    #[test]
    fn add_window_details_land_on_the_new_person() {
        let mut app = App::for_test(tiny());
        let mut d = Dlg::new(Kind::Person, "F", None);
        d.name = "Cara May Lee".into();
        d.bd = " 4 Jun 1950 ".into();
        d.bp = "Leeds".into();
        d.dd = "1 Jan 2010".into();
        d.living = true; // ignored because a death date was given
        d.notes = "note".into();
        app.create(d);
        let p = app.db.people.values().find(|p| p.name == "Cara May Lee").unwrap();
        assert_eq!((p.bd.as_str(), p.bp.as_str(), p.dd.as_str(), p.sex.as_str()), ("4 Jun 1950", "Leeds", "1 Jan 2010", "F"));
        assert!(!p.living);
        assert_eq!(p.surname, "Lee");
        assert_eq!(p.notes, "note");
        // the new person opens on their (locked) page
        assert!(app.editing.is_none());
    }

    #[test]
    fn cancel_puts_the_person_back_exactly_as_they_were() {
        let mut app = App::for_test(tiny());
        let id = app.sel.unwrap();
        let before = serde_json::to_string(&app.db).unwrap();
        app.editing = Some(EditSession { id, snapshot: before.clone() });
        // a cat walks on the keyboard
        app.db.people.get_mut(&id).unwrap().bd = "99999".into();
        app.db.people.get_mut(&id).unwrap().notes = "asdfgh".into();
        assert_ne!(serde_json::to_string(&app.db).unwrap(), before);
        let e = app.editing.take().unwrap();
        app.load_snapshot(e.snapshot);
        assert_eq!(serde_json::to_string(&app.db).unwrap(), before);
        assert_eq!(app.sel, Some(id));
    }

    /// Draw the Individual page headless, locked and unlocked, to make sure nothing panics.
    #[test]
    fn individual_page_draws_locked_and_unlocked() {
        let mut app = App::for_test(tiny());
        let id = app.sel.unwrap();
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        for editing in [false, true] {
            app.editing = if editing { Some(EditSession { id, snapshot: String::new() }) } else { None };
            let mut act: Option<Action> = None;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.person_view(ui, &mut act));
            });
            // nothing is changed just by drawing
            assert!(act.is_none());
        }
    }

    #[test]
    fn locked_page_ignores_typing() {
        let mut app = App::for_test(tiny());
        let id = app.sel.unwrap();
        let before = serde_json::to_string(&app.db).unwrap();
        let ctx = egui::Context::default();
        let mut act: Option<Action> = None;
        for _ in 0..3 {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::Text("987654".into()));
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.person_view(ui, &mut act));
            });
        }
        assert_eq!(serde_json::to_string(&app.db).unwrap(), before, "typing changed a locked person {}", id);
    }

    /// Draw the Family tab headless, wide and narrow, for a person with parents, step-parents,
    /// two partnerships, an adopted child and an association.
    #[test]
    fn family_tab_draws_for_a_complex_family() {
        let mut db = tiny();
        let mk = |db: &mut Db, n: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), bd: "1950".into(), bp: "Leeds".into(), ..Default::default() });
            id
        };
        let me = mk(&mut db, "Me Lee");
        let (dad, mum, step, partner, partner2, kid, adopted) = (mk(&mut db, "Dad"), mk(&mut db, "Mum"), mk(&mut db, "Step Mum"), mk(&mut db, "Pat"), mk(&mut db, "Quinn"), mk(&mut db, "Kid"), mk(&mut db, "Adoptee"));
        let f1 = db.alloc();
        db.fams.insert(f1, Family { h: Some(dad), w: Some(mum), kids: vec![me], ..Default::default() });
        let f2 = db.alloc();
        let mut fam2 = Family { h: Some(dad), w: Some(step), kids: vec![me], ..Default::default() };
        fam2.frel.insert(me, "Step".into());
        fam2.mrel.insert(me, "Step".into());
        db.fams.insert(f2, fam2);
        let f3 = db.alloc();
        let mut fam3 = Family { h: Some(me), w: Some(partner), kids: vec![kid, adopted], md: "1975".into(), kind: "Married".into(), ..Default::default() };
        fam3.pedi.insert(adopted, "adopted".into());
        db.fams.insert(f3, fam3);
        let f4 = db.alloc();
        db.fams.insert(f4, Family { h: Some(me), w: Some(partner2), ..Default::default() });
        db.people.get_mut(&me).unwrap().assoc.push(Assoc { other: dad, role: "Guardian".into(), note: String::new() });
        let mut app = App::for_test(db);
        app.sel = Some(me);
        let ctx = egui::Context::default();
        for (w, fx) in [(1300.0f32, 0usize), (1300.0, 1), (600.0, 0)] {
            app.fx = fx;
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, 800.0)));
            let mut act: Option<Action> = None;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.family_view(ui, &mut act));
            });
            assert!(act.is_none());
        }
        // a person with no families at all
        app.sel = Some(partner2);
        app.db.fams.remove(&f4);
        let mut act = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.family_view(ui, &mut act));
        });
    }

    #[test]
    fn dock_panels_draw_wide_narrow_and_floating() {
        let mut app = App::for_test(tiny());
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        for d in app.settings.dock.iter_mut() {
            d.on = true;
        }
        let run = |app: &mut App, w: f32| {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, 900.0)));
            let mut act: Option<Action> = None;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.person_view(ui, &mut act));
            });
        };
        run(&mut app, 1400.0);
        run(&mut app, 600.0);
        app.settings.dock[0].float = true; // pop the first panel out
        run(&mut app, 1400.0);
        let id = app.sel.unwrap();
        app.editing = Some(EditSession { id, snapshot: String::new() });
        run(&mut app, 1400.0);
    }

    #[test]
    fn old_settings_files_get_the_default_dock() {
        let old = r#"{"dark":true,"accent":[59,111,212],"bg":[30,31,34],"gens":5,"hide_living":true,"mic":"","zoom":1.0}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.dock.len(), default_dock().len());
        assert!(s.dock.iter().any(|d| d.sec == "relationships" && d.on));
        assert!(s.dock_w > 200.0);
    }

    #[test]
    fn descendants_boxes_put_the_partner_on_a_second_line() {
        let mut db = tiny();
        let ids: Vec<u32> = db.people.keys().copied().collect();
        let (a, b) = (ids[0], ids[1]);
        db.people.get_mut(&a).unwrap().bd = "1900".into();
        let (l1, l2) = desc_lines(&db, a);
        assert!(l1.contains("Ann Lee") && l1.contains("1900"));
        let l2 = l2.expect("partner line");
        assert!(l2.starts_with("+ Bob Lee"), "{}", l2);
        // someone with no partner has a single line
        let c = db.alloc();
        db.people.insert(c, Person { name: "Solo".into(), ..Default::default() });
        assert!(desc_lines(&db, c).1.is_none());
        // the printed chart carries both lines
        let app = App::for_test(db.clone());
        let svg = app.descendant_svg(&db, a);
        assert!(svg.contains("Ann Lee") && svg.contains("+ Bob Lee"));
        let _ = b;
    }
}
