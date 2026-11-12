//! The data the program keeps: people, families, events, sources, media and the database.

use super::*;

/// A piece of GEDCOM the app has no field for. It is kept exactly as read
/// (as GEDCOM lines, levels relative to where it belongs) and written back on export.
/// `at` names the sub-block it sits under ("" = directly under its parent).
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Raw {
    pub(crate) at: String,
    pub(crate) text: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Media {
    pub(crate) file: String,
    pub(crate) name: String,
    pub(crate) caption: String,
    pub(crate) thumb: String,
    pub(crate) mid: String,
    /// The file reference as it appeared in a GEDCOM file (kept when the file itself was not found).
    pub(crate) path: String,
    pub(crate) form: String,
    /// People tagged in this picture.
    pub(crate) faces: Vec<Face>,
    pub(crate) extra: Vec<Raw>,
}

/// A box drawn on a photo around one person's face. `r` is x, y, width, height as parts of the
/// picture (0 to 1), so it stays right whatever size the photo is shown at.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct Face {
    pub(crate) who: u32,
    pub(crate) r: [f32; 4],
}

/// A profile picture cut from a tagged face in someone's photo.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct FacePortrait {
    pub(crate) owner: u32,
    pub(crate) file: String,
    pub(crate) r: [f32; 4],
}

/// One event or fact. Used for life events, attributes, family events and the
/// extra details (notes, causes, ages...) that hang off birth, death, marriage and so on.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Event {
    pub(crate) kind: String,
    pub(crate) date: String,
    pub(crate) place: String,
    /// Free-text note.
    pub(crate) note: String,
    /// The value on the event line itself (an occupation, a religion, "Y"...).
    pub(crate) val: String,
    pub(crate) typ: String,
    pub(crate) cause: String,
    pub(crate) age: String,
    pub(crate) agency: String,
    /// LDS temple code and ordinance status.
    pub(crate) temp: String,
    pub(crate) stat: String,
    /// The original GEDCOM tag, when it is not one this app knows by name.
    pub(crate) tag: String,
    /// How sure the researcher is of this fact: confirmed, supported, probable, uncertain, disputed.
    pub(crate) conf: String,
    pub(crate) cites: Vec<Source>,
    pub(crate) note_refs: Vec<u32>,
    pub(crate) extra: Vec<Raw>,
}

/// The research confidence scale, strongest first: (stored word, label, colour).
pub(crate) const CONFIDENCE: [(&str, &str, [u8; 3]); 5] = [
    ("confirmed", "Confirmed by original records", [56, 142, 60]),
    ("supported", "Supported by several sources", [40, 120, 170]),
    ("probable", "Probable", [196, 152, 40]),
    ("uncertain", "Uncertain", [220, 120, 40]),
    ("disputed", "Disputed", [200, 60, 60]),
];

/// A sound-alike key for a surname (American Soundex), so spelling variants group together.
pub(crate) fn sound_key(s: &str) -> String {
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

pub(crate) fn first_given(g: &str) -> String {
    g.split_whitespace().next().unwrap_or("").to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// The formal name behind a common nickname ("Bill" -> "william").
pub(crate) fn canon_given(g: &str) -> String {
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
pub(crate) fn edit_distance(a: &str, b: &str) -> usize {
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
pub(crate) fn conf_badge(ui: &mut egui::Ui, v: &str) {
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
pub(crate) fn conf_combo(ui: &mut egui::Ui, id: impl std::hash::Hash, v: &mut String) -> bool {
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

pub(crate) fn conf_label(v: &str) -> Option<&'static str> {
    let v = v.trim().to_lowercase();
    CONFIDENCE.iter().find(|c| c.0 == v).map(|c| c.1)
}

/// One entry in the research journal: what was searched, where, and what came of it.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Journal {
    /// The day of the search, as typed.
    pub(crate) date: String,
    /// What was looked for.
    pub(crate) goal: String,
    /// Where: an archive, a website, a book, a person.
    pub(crate) place: String,
    /// The source searched, when it is one in the tree.
    pub(crate) sid: Option<u32>,
    /// "found", "nothing", "partial" or "" (not yet).
    pub(crate) result: String,
    /// What was discovered (or not).
    pub(crate) notes: String,
    /// People this search was about.
    pub(crate) people: Vec<u32>,
}

pub(crate) const EVENT_KINDS: &[&str] = &[
    "Residence", "Occupation", "Education", "Graduation", "Religion", "Title", "Nationality", "Physical description", "Possessions", "Fact",
    "Immigration", "Emigration", "Naturalization", "Census", "Retirement", "Adoption", "Baptism", "Adult christening", "Blessing", "Confirmation",
    "First communion", "Ordination", "Bar Mitzvah", "Bas Mitzvah", "Cremation", "Probate", "Will", "Caste", "ID number", "Social security number",
    "Number of children", "Number of marriages", "Christening", "Birth (additional)", "Death (additional)", "Burial (additional)",
    "LDS baptism", "LDS confirmation", "LDS initiatory", "LDS endowment", "LDS child sealing",
    "Military service", "Divorce", "Other",
];
/// GEDCOM tag and the label shown in the app, for events and facts of a person.
pub(crate) const EVENT_TAGS: &[(&str, &str)] = &[
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
pub(crate) const FAM_TAGS: &[(&str, &str)] = &[
    ("DIV", "Divorce"), ("DIVF", "Divorce filed"), ("ENGA", "Engagement"), ("MARB", "Marriage banns"), ("MARC", "Marriage contract"),
    ("MARL", "Marriage license"), ("MARS", "Marriage settlement"), ("ANUL", "Annulment"), ("MARR", "Marriage (additional)"),
    ("CENS", "Census"), ("RESI", "Residence"), ("SLGS", "LDS spouse sealing"), ("NCHI", "Number of children"),
];
pub(crate) const FAM_KINDS: &[&str] = &[
    "Divorce", "Divorce filed", "Engagement", "Marriage banns", "Marriage contract", "Marriage license", "Marriage settlement", "Annulment",
    "Marriage (additional)", "Census", "Residence", "LDS spouse sealing", "Number of children", "Other",
];

/// A citation: a reference to a source, with the page and details that go with it.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Source {
    pub(crate) about: String,
    /// Shown text. For a citation of a source record this is that record's title.
    pub(crate) text: String,
    /// The source record cited, if any.
    pub(crate) sid: Option<u32>,
    pub(crate) page: String,
    /// Text copied from the source (a transcription), and its date.
    pub(crate) data: String,
    pub(crate) ddate: String,
    /// A web address for this particular record, and the day it was looked at.
    pub(crate) url: String,
    pub(crate) accessed: String,
    pub(crate) quality: String,
    pub(crate) note: String,
    pub(crate) note_refs: Vec<u32>,
    pub(crate) extra: Vec<Raw>,
}

/// A source record (a book, a census, a website...) that citations point to.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SourceRec {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) abbr: String,
    pub(crate) publ: String,
    /// What kind of source it is: Book, Census, Vital record, Church record, Website...
    pub(crate) kind: String,
    /// Web address of the source as a whole.
    pub(crate) url: String,
    /// Where the source is kept, and its call number there.
    pub(crate) repo: Option<u32>,
    pub(crate) call: String,
    pub(crate) medium: String,
    pub(crate) text: String,
    pub(crate) notes: String,
    pub(crate) note_refs: Vec<u32>,
    pub(crate) extra: Vec<Raw>,
}

/// A repository: the library, archive, website or person that holds sources.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct RepoRec {
    pub(crate) name: String,
    pub(crate) addr: String,
    pub(crate) url: String,
    pub(crate) email: String,
    pub(crate) phone: String,
    pub(crate) notes: String,
    pub(crate) note_refs: Vec<u32>,
    pub(crate) extra: Vec<Raw>,
}

pub(crate) const SOURCE_KINDS: &[&str] = &[
    "Book", "Census", "Vital record", "Church record", "Military record", "Land or probate record", "Immigration record", "Newspaper", "Website",
    "Database", "Family papers", "Interview", "Photograph", "Cemetery record", "Letter or diary", "Other",
];

/// A note that is shared between several people, families or sources.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct NoteRec {
    pub(crate) text: String,
    pub(crate) extra: Vec<Raw>,
}

/// A second (or third...) name: birth name, married name, nickname, religious name.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AltName {
    pub(crate) kind: String,
    pub(crate) given: String,
    pub(crate) surname: String,
    pub(crate) prefix: String,
    pub(crate) suffix: String,
    pub(crate) nick: String,
    pub(crate) note: String,
    pub(crate) note_refs: Vec<u32>,
    pub(crate) cites: Vec<Source>,
    pub(crate) extra: Vec<Raw>,
}

pub(crate) const FACTS: [&str; 6] = ["General", "Birth", "Christening", "Death", "Burial", "Marriage"];

/// How to reach a living relative.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct Contact {
    pub(crate) addr: String,
    pub(crate) phone: String,
    pub(crate) email: String,
    pub(crate) url: String,
}

impl Contact {
    pub(crate) fn is_empty(&self) -> bool {
        [&self.addr, &self.phone, &self.email, &self.url].iter().all(|s| s.trim().is_empty())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Person {
    pub(crate) name: String,
    pub(crate) given: String,
    pub(crate) surname: String,
    pub(crate) maiden: String,
    pub(crate) living: bool,
    pub(crate) sex: String,
    pub(crate) bd: String,
    pub(crate) bp: String,
    pub(crate) chd: String,
    pub(crate) chp: String,
    pub(crate) dd: String,
    pub(crate) dp: String,
    pub(crate) bud: String,
    pub(crate) bup: String,
    pub(crate) notes: String,
    pub(crate) media: Vec<Media>,
    pub(crate) sources: Vec<Source>,
    pub(crate) events: Vec<Event>,
    pub(crate) portrait: String,
    pub(crate) prefix: String,
    pub(crate) suffix: String,
    pub(crate) nick: String,
    pub(crate) alt_names: Vec<AltName>,
    /// Shared notes this person points to.
    pub(crate) note_refs: Vec<u32>,
    /// "BAPM" when the christening above was recorded as a baptism.
    pub(crate) chtag: String,
    /// Original values of single-line facts that carry one ("SEX" other than M/F, "BIRT" = "Y").
    pub(crate) fvals: BTreeMap<String, String>,
    /// Extra details of NAME, BIRT, CHR, DEAT and BURI (notes, causes, ages...).
    pub(crate) fx: BTreeMap<String, Event>,
    /// The family whose parents the chart shows, when the person belongs to more than one.
    pub(crate) pref_parents: Option<u32>,
    /// The order of this person's own families (spouses), when it has been chosen by hand or came
    /// from a file. Empty means by marriage date. Families not listed come after, by date.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) fam_order: Vec<u32>,
    /// Address, phone, email and web page, for living relatives (GEDCOM ADDR, PHON, EMAIL, WWW).
    #[serde(skip_serializing_if = "Contact::is_empty")]
    pub(crate) contact: Contact,
    /// Ties to other people outside the family structure: guardian, godparent, witness...
    pub(crate) assoc: Vec<Assoc>,
    /// Research tasks for this person.
    pub(crate) todos: Vec<Todo>,
    /// Profile picture cut from a face tag (takes the place of `portrait` when set).
    pub(crate) face_portrait: Option<FacePortrait>,
    /// The part of the profile photo to show (x, y, width, height as parts of the picture).
    pub(crate) portrait_crop: Option<[f32; 4]>,
    pub(crate) extra: Vec<Raw>,
}

/// A research task: something still to find out or do for a person.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Todo {
    pub(crate) text: String,
    pub(crate) done: bool,
    /// The day it was written down, as "2026-10-29".
    pub(crate) added: String,
    pub(crate) note: String,
}

/// "`other` is this person's `role`": a guardian, godparent, mentor, witness...
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Assoc {
    pub(crate) other: u32,
    pub(crate) role: String,
    pub(crate) note: String,
}

/// How a child can be related to a parent. The stored word is the one other programs use.
/// Optional donation page (opened in the browser; the app never handles payments).
pub(crate) const DONATE_URL: &str = "https://www.paypal.me/doylemcintosh";
/// Where people report problems and ask questions: the GitHub repository's Issues page.
/// Left empty until the repository's address is filled in here.
pub(crate) const PROJECT_URL: &str = "";
pub(crate) const NOT_AFFILIATED: &str = "Gilded Roots is an independent project. It is not affiliated with, endorsed by or connected to FamilySearch, The Church of Jesus Christ of Latter-day Saints, Ancestry, MyHeritage, RootsMagic or any other company named in it. Personal Ancestral File (PAF) and the other product names are trademarks of their owners and are used only to describe compatibility.";
/// The licences of every library the program is built from (generated from Cargo.lock).
pub(crate) const THIRD_PARTY: &str = include_str!("../THIRD_PARTY_LICENSES.txt");
pub(crate) const PARENT_RELS: &[(&str, &str)] = &[
    ("Natural", "Biological"),
    ("Adopted", "Adopted"),
    ("Step", "Step-parent"),
    ("Foster", "Foster"),
    ("Guardian", "Guardian"),
    ("Sealed", "Sealed (LDS)"),
];

/// What kind of partnership a family record is (Gramps-style words).
pub(crate) const PARTNER_KINDS: &[&str] = &["Married", "Unmarried", "Civil Union", "Unknown"];

pub(crate) const ASSOC_ROLES: &[&str] = &["Guardian", "Godparent", "Foster parent", "Step-parent", "Mentor", "Employer", "Witness", "Neighbor", "Friend", "Associate"];

pub(crate) fn rel_is_bio(s: &str) -> bool {
    matches!(s.trim().to_lowercase().as_str(), "" | "natural" | "biological" | "birth")
}

pub(crate) fn rel_name(s: &str) -> String {
    match PARENT_RELS.iter().find(|(k, _)| k.eq_ignore_ascii_case(s.trim())) {
        Some((_, v)) => v.to_string(),
        None if rel_is_bio(s) => "Biological".to_string(),
        None => s.to_string(),
    }
}

#[derive(Default)]
pub(crate) struct Idx {
    pub(crate) child_of: HashMap<u32, Vec<u32>>,
    pub(crate) partner_in: HashMap<u32, Vec<u32>>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Family {
    pub(crate) h: Option<u32>,
    pub(crate) w: Option<u32>,
    pub(crate) md: String,
    pub(crate) mp: String,
    pub(crate) kids: Vec<u32>,
    /// More partners than the usual one husband and one wife, with their GEDCOM role.
    pub(crate) more: Vec<(String, u32)>,
    /// How each child is related to this family: "adopted", "foster", "sealing", "birth".
    pub(crate) pedi: BTreeMap<u32, String>,
    /// What kind of partnership this is (Married, Unmarried, Civil Union...). Empty when not said.
    pub(crate) kind: String,
    /// How a child is related to the father / mother slot when that differs from `pedi`
    /// (a child born to the mother and adopted by her husband, for instance).
    pub(crate) frel: BTreeMap<u32, String>,
    pub(crate) mrel: BTreeMap<u32, String>,
    /// Details of the marriage other than its date and place.
    pub(crate) mx: Event,
    pub(crate) events: Vec<Event>,
    pub(crate) cites: Vec<Source>,
    pub(crate) notes: String,
    pub(crate) note_refs: Vec<u32>,
    pub(crate) media: Vec<Media>,
    pub(crate) fvals: BTreeMap<String, String>,
    pub(crate) extra: Vec<Raw>,
}

#[derive(Default)]
pub(crate) struct Kin {
    pub(crate) step_parents: Vec<u32>,
    pub(crate) half_siblings: Vec<u32>,
    pub(crate) step_siblings: Vec<u32>,
    pub(crate) step_children: Vec<u32>,
    /// People who list this person as their guardian, godparent...: (who, role).
    pub(crate) assoc_of: Vec<(u32, String)>,
}

impl Family {
    /// How `child` is related to the father slot (`father`) or the mother slot: Natural, Adopted, Step...
    /// An empty answer means nothing was recorded, which is read as biological.
    pub(crate) fn rel_to(&self, child: u32, father: bool) -> String {
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
pub(crate) struct Db {
    #[serde(default)]
    pub(crate) people: BTreeMap<u32, Person>,
    #[serde(default)]
    pub(crate) fams: BTreeMap<u32, Family>,
    #[serde(default)]
    pub(crate) next: u32,
    #[serde(default)]
    pub(crate) sample: bool,
    #[serde(default)]
    pub(crate) sources: BTreeMap<u32, SourceRec>,
    #[serde(default)]
    pub(crate) repos: BTreeMap<u32, RepoRec>,
    #[serde(default)]
    pub(crate) notes: BTreeMap<u32, NoteRec>,
    /// Whole GEDCOM records with no place in the model (repositories, submitter, custom records).
    #[serde(default)]
    pub(crate) records: Vec<String>,
    /// People the user bookmarked, in the order they were added.
    #[serde(default)]
    pub(crate) bookmarks: Vec<u32>,
    /// The research journal, newest last.
    #[serde(default)]
    pub(crate) journal: Vec<Journal>,
    /// Historical events the user added or changed for timelines (the built-in list is used otherwise).
    #[serde(default)]
    pub(crate) history: Vec<(i32, i32, String, String)>,
    /// Map positions (longitude, latitude) the user set for places the map could not find.
    #[serde(default)]
    pub(crate) place_coords: BTreeMap<String, [f32; 2]>,
    /// Autosomal DNA tests: ethnicity estimates and match lists (the DNA tab).
    #[serde(default)]
    pub(crate) dna: Vec<dna::DnaTest>,
    /// Data layout version, so files saved by older versions are upgraded once.
    /// A file with no version is an old one (0); new trees start at the current version.
    #[serde(default)]
    pub(crate) ver: u32,
    /// The home person of this tree (each tree keeps its own). Taken from the file when a tree is
    /// imported, and written back into exported GEDCOM files.
    #[serde(default)]
    pub(crate) home: Option<u32>,
    /// Saved groups of people (Tools > Groups), kept with the tree.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) groups: Vec<Group>,
}

/// A named set of people, and the steps it was made with.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Group {
    pub(crate) name: String,
    pub(crate) ids: BTreeSet<u32>,
    pub(crate) how: Vec<String>,
}

impl Default for Db {
    fn default() -> Self {
        Db { people: BTreeMap::new(), fams: BTreeMap::new(), next: 0, sample: false, sources: BTreeMap::new(), repos: BTreeMap::new(), notes: BTreeMap::new(), records: Vec::new(), bookmarks: Vec::new(), journal: Vec::new(), history: Vec::new(), place_coords: BTreeMap::new(), dna: Vec::new(), ver: 4, home: None, groups: Vec::new() }
    }
}

/// Who a citation belongs to.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Owner {
    Person(u32),
    Family(u32),
}

/// What a citation backs up: a named fact ("Birth") or an event (kind and date).
pub(crate) struct Claim<'a> {
    pub(crate) what: &'a str,
    pub(crate) when: &'a str,
}

pub(crate) fn source_title(s: &SourceRec) -> String {
    if !s.title.is_empty() {
        s.title.clone()
    } else if !s.abbr.is_empty() {
        s.abbr.clone()
    } else {
        "(untitled source)".to_string()
    }
}

pub(crate) fn quality_label(q: &str) -> &'static str {
    match q.trim() {
        "0" => "0 - unreliable",
        "1" => "1 - questionable",
        "2" => "2 - secondary evidence",
        "3" => "3 - direct, primary evidence",
        _ => "not rated",
    }
}
