//! The data the program keeps: people, families, events, sources, media and the database.

use super::*;

/// A piece of GEDCOM the app has no field for. It is kept exactly as read
/// (as GEDCOM lines, levels relative to where it belongs) and written back on export.
/// `at` names the sub-block it sits under ("" = directly under its parent).
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Raw {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Media {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) file: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) caption: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) thumb: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) mid: String,
    /// The file reference as it appeared in a GEDCOM file (kept when the file itself was not found).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) form: String,
    /// People tagged in this picture.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) faces: Vec<Face>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) file: String,
    pub(crate) r: [f32; 4],
}

/// One event or fact. Used for life events, attributes, family events and the
/// extra details (notes, causes, ages...) that hang off birth, death, marriage and so on.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Event {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) date: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) place: String,
    /// Free-text note.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) note: String,
    /// The value on the event line itself (an occupation, a religion, "Y"...).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) val: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) typ: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) cause: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) age: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) agency: String,
    /// LDS temple code and ordinance status.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) temp: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) stat: String,
    /// The original GEDCOM tag, when it is not one this app knows by name.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) tag: String,
    /// How sure the researcher is of this fact: confirmed, supported, probable, uncertain, disputed.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) conf: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) cites: Vec<Source>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) note_refs: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) date: String,
    /// What was looked for.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) goal: String,
    /// Where: an archive, a website, a book, a person.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) place: String,
    /// The source searched, when it is one in the tree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sid: Option<u32>,
    /// "found", "nothing", "partial" or "" (not yet).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) result: String,
    /// What was discovered (or not).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) notes: String,
    /// People this search was about.
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) about: String,
    /// Shown text. For a citation of a source record this is that record's title.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    /// The source record cited, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sid: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) page: String,
    /// Text copied from the source (a transcription), and its date.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) data: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) ddate: String,
    /// A web address for this particular record, and the day it was looked at.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) accessed: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) quality: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) note: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) note_refs: Vec<u32>,
    /// Pictures of the record itself (a census page, a certificate), kept with this citation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) media: Vec<Media>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra: Vec<Raw>,
}

/// A source record (a book, a census, a website...) that citations point to.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SourceRec {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) author: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) abbr: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) publ: String,
    /// What kind of source it is: Book, Census, Vital record, Church record, Website...
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) kind: String,
    /// Web address of the source as a whole.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) url: String,
    /// Where the source is kept, and its call number there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repo: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) call: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) medium: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) notes: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) note_refs: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra: Vec<Raw>,
}

/// A repository: the library, archive, website or person that holds sources.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct RepoRec {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) addr: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) email: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) phone: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) notes: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) note_refs: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra: Vec<Raw>,
}

/// A second (or third...) name: birth name, married name, nickname, religious name.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AltName {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) given: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) surname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) prefix: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) suffix: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) nick: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) note: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) note_refs: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) cites: Vec<Source>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra: Vec<Raw>,
}

pub(crate) const FACTS: [&str; 6] = ["General", "Birth", "Christening", "Death", "Burial", "Marriage"];

/// How to reach a living relative.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct Contact {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) addr: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) phone: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) email: String,
    #[serde(skip_serializing_if = "String::is_empty")]
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) given: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) surname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) maiden: String,
    #[serde(skip_serializing_if = "crate::is_false")]
    pub(crate) living: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) sex: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) bd: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) bp: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) chd: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) chp: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) dd: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) dp: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) bud: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) bup: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) notes: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) media: Vec<Media>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) sources: Vec<Source>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) events: Vec<Event>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) portrait: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) prefix: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) suffix: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) nick: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) alt_names: Vec<AltName>,
    /// Shared notes this person points to.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) note_refs: Vec<u32>,
    /// "BAPM" when the christening above was recorded as a baptism.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) chtag: String,
    /// Original values of single-line facts that carry one ("SEX" other than M/F, "BIRT" = "Y").
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) fvals: BTreeMap<String, String>,
    /// Extra details of NAME, BIRT, CHR, DEAT and BURI (notes, causes, ages...).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) fx: BTreeMap<String, Event>,
    /// The family whose parents the chart shows, when the person belongs to more than one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pref_parents: Option<u32>,
    /// The order of this person's own families (spouses), when it has been chosen by hand or came
    /// from a file. Empty means by marriage date. Families not listed come after, by date.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) fam_order: Vec<u32>,
    /// Address, phone, email and web page, for living relatives (GEDCOM ADDR, PHON, EMAIL, WWW).
    #[serde(skip_serializing_if = "Contact::is_empty")]
    pub(crate) contact: Contact,
    /// Ties to other people outside the family structure: guardian, godparent, witness...
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) assoc: Vec<Assoc>,
    /// Research tasks for this person.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) todos: Vec<Todo>,
    /// Profile picture cut from a face tag (takes the place of `portrait` when set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) face_portrait: Option<FacePortrait>,
    /// The part of the profile photo to show (x, y, width, height as parts of the picture).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) portrait_crop: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra: Vec<Raw>,
}

/// A research task: something still to find out or do for a person.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Todo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    #[serde(skip_serializing_if = "crate::is_false")]
    pub(crate) done: bool,
    /// The day it was written down, as "2026-10-29".
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) added: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) note: String,
}

/// "`other` is this person's `role`": a guardian, godparent, mentor, witness...
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Assoc {
    pub(crate) other: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
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

/// Who belongs to which family, so finding someone's parents or partners does not mean
/// reading every family in the tree. Partners are in no particular order here; `fams_of_ix`
/// puts them in order.
#[derive(Default)]
pub(crate) struct Idx {
    pub(crate) child_of: HashMap<u32, Vec<u32>>,
    pub(crate) partner_in: HashMap<u32, Vec<u32>>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Family {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) h: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) w: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) md: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) mp: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) kids: Vec<u32>,
    /// More partners than the usual one husband and one wife, with their GEDCOM role.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) more: Vec<(String, u32)>,
    /// How each child is related to this family: "adopted", "foster", "sealing", "birth".
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) pedi: BTreeMap<u32, String>,
    /// What kind of partnership this is (Married, Unmarried, Civil Union...). Empty when not said.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) kind: String,
    /// How a child is related to the father / mother slot when that differs from `pedi`
    /// (a child born to the mother and adopted by her husband, for instance).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) frel: BTreeMap<u32, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) mrel: BTreeMap<u32, String>,
    /// Details of the marriage other than its date and place.
    pub(crate) mx: Event,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) events: Vec<Event>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) cites: Vec<Source>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) notes: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) note_refs: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) media: Vec<Media>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) fvals: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    pub(crate) fams: Fams,
    #[serde(default)]
    pub(crate) next: u32,
    /// The media folder is kept sorted into person folders (Tools > Sort media into person folders).
    #[serde(default, skip_serializing_if = "crate::is_false")]
    pub(crate) media_folders: bool,
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
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub(crate) ids: BTreeSet<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) how: Vec<String>,
}

impl Default for Db {
    fn default() -> Self {
        Db { people: BTreeMap::new(), fams: Fams::default(), next: 0, media_folders: false, sample: false, sources: BTreeMap::new(), repos: BTreeMap::new(), notes: BTreeMap::new(), records: Vec::new(), bookmarks: Vec::new(), journal: Vec::new(), history: Vec::new(), place_coords: BTreeMap::new(), dna: Vec::new(), ver: 4, home: None, groups: Vec::new() }
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


/// The tree's families. It reads and changes like the plain list of families it holds, and it
/// remembers the family index (who is whose child and partner) until something changes a
/// family: any change, through any route, throws the remembered index away.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Fams {
    map: BTreeMap<u32, Family>,
    #[serde(skip)]
    ix: std::sync::Mutex<Option<std::sync::Arc<Idx>>>,
}

impl Default for Fams {
    fn default() -> Self {
        BTreeMap::new().into()
    }
}

impl Clone for Fams {
    fn clone(&self) -> Self {
        let ix = self.ix.lock().map(|g| g.clone()).unwrap_or(None);
        Fams { map: self.map.clone(), ix: std::sync::Mutex::new(ix) }
    }
}

impl From<BTreeMap<u32, Family>> for Fams {
    fn from(map: BTreeMap<u32, Family>) -> Self {
        Fams { map, ix: Default::default() }
    }
}

impl std::ops::Deref for Fams {
    type Target = BTreeMap<u32, Family>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl std::ops::DerefMut for Fams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Ok(g) = self.ix.get_mut() {
            *g = None;
        }
        &mut self.map
    }
}

impl<'a> IntoIterator for &'a Fams {
    type Item = (&'a u32, &'a Family);
    type IntoIter = std::collections::btree_map::Iter<'a, u32, Family>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<'a> IntoIterator for &'a mut Fams {
    type Item = (&'a u32, &'a mut Family);
    type IntoIter = std::collections::btree_map::IterMut<'a, u32, Family>;
    fn into_iter(self) -> Self::IntoIter {
        std::ops::DerefMut::deref_mut(self).iter_mut()
    }
}

impl IntoIterator for Fams {
    type Item = (u32, Family);
    type IntoIter = std::collections::btree_map::IntoIter<u32, Family>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl FromIterator<(u32, Family)> for Fams {
    fn from_iter<I: IntoIterator<Item = (u32, Family)>>(it: I) -> Self {
        it.into_iter().collect::<BTreeMap<_, _>>().into()
    }
}

impl Fams {
    /// The family index, built the first time it is asked for after a change.
    pub(crate) fn index(&self) -> std::sync::Arc<Idx> {
        let mut g = match self.ix.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if let Some(ix) = g.as_ref() {
            return ix.clone();
        }
        let mut ix = Idx::default();
        for (fid, f) in &self.map {
            for k in &f.kids {
                let e = ix.child_of.entry(*k).or_default();
                if !e.contains(fid) {
                    e.push(*fid);
                }
            }
            for p in f.h.iter().chain(f.w.iter()).chain(f.more.iter().map(|(_, x)| x)) {
                let e = ix.partner_in.entry(*p).or_default();
                if !e.contains(fid) {
                    e.push(*fid);
                }
            }
        }
        let ix = std::sync::Arc::new(ix);
        *g = Some(ix.clone());
        ix
    }
}
