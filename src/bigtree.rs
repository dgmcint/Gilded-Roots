//! A made-up family tree of any size, for testing that big trees stay fast. It is written as
//! a GEDCOM file, so reading it also tests the importer. The same size always gives the
//! same tree.

#![cfg(test)]

use super::*;

const MALE: &[&str] = &[
    "John", "William", "James", "George", "Charles", "Thomas", "Joseph", "Henry", "Robert", "Edward", "Samuel", "David", "Frank", "Walter", "Arthur", "Albert", "Harry", "Richard", "Fred", "Peter", "Hans", "Lars", "Olaf",
    "Erik", "Pierre", "Jean", "Giovanni", "Josef", "Jan", "Patrick", "Michael", "Daniel", "Benjamin", "Isaac", "Jacob", "Andrew", "Hugh", "Alexander", "Ernest", "Herbert", "Leonard", "Stanley", "Raymond", "Howard", "Ralph",
];
const FEMALE: &[&str] = &[
    "Mary", "Anna", "Elizabeth", "Margaret", "Sarah", "Emma", "Alice", "Martha", "Ellen", "Clara", "Ida", "Bertha", "Minnie", "Annie", "Florence", "Ruth", "Helen", "Edith", "Grace", "Rose", "Jane", "Catherine", "Louise",
    "Ingrid", "Marie", "Sophie", "Maria", "Johanna", "Kirsten", "Bridget", "Agnes", "Hannah", "Rebecca", "Lydia", "Esther", "Harriet", "Mabel", "Ethel", "Lillian", "Dorothy", "Mildred", "Frances", "Eleanor", "Hazel", "Irene",
];
const SURNAMES: &[&str] = &[
    "Smith", "Johnson", "Brown", "Taylor", "Miller", "Wilson", "Moore", "Anderson", "Thomas", "Jackson", "White", "Harris", "Martin", "Thompson", "Young", "Allen", "King", "Wright", "Scott", "Green", "Baker", "Adams", "Nelson",
    "Hill", "Campbell", "Mitchell", "Roberts", "Carter", "Phillips", "Evans", "Turner", "Parker", "Collins", "Edwards", "Stewart", "Morris", "Murphy", "Cook", "Rogers", "Morgan", "Cooper", "Peterson", "Bailey", "Reed",
    "Kelly", "Howard", "Cox", "Ward", "Richardson", "Watson", "Brooks", "Wood", "James", "Bennett", "Gray", "Hughes", "Price", "Sanders", "Myers", "Long", "Ross", "Foster", "Jensen", "Larsen", "Nielsen", "Hansen",
    "Olsen", "Schmidt", "Schneider", "Fischer", "Weber", "Becker", "Dubois", "Moreau", "Rossi", "Russo", "Novak", "Kowalski", "Harwood", "Ellis", "McIntosh", "Fraser", "MacLeod", "O'Brien", "Walsh", "Byrne",
];
const PLACES: &[&str] = &[
    "Boise, Ada, Idaho, USA", "Twin Falls, Twin Falls, Idaho, USA", "Salt Lake City, Salt Lake, Utah, USA", "Logan, Cache, Utah, USA", "Ogden, Weber, Utah, USA", "Provo, Utah, Utah, USA", "Elko, Elko, Nevada, USA",
    "Portland, Multnomah, Oregon, USA", "Eugene, Lane, Oregon, USA", "Seattle, King, Washington, USA", "Spokane, Spokane, Washington, USA", "Denver, Denver, Colorado, USA", "Omaha, Douglas, Nebraska, USA",
    "Des Moines, Polk, Iowa, USA", "Chicago, Cook, Illinois, USA", "Nauvoo, Hancock, Illinois, USA", "Columbus, Franklin, Ohio, USA", "Pittsburgh, Allegheny, Pennsylvania, USA", "Boston, Suffolk, Massachusetts, USA",
    "New York, New York, New York, USA", "Richmond, Henrico, Virginia, USA", "Nashville, Davidson, Tennessee, USA", "St. Louis, St. Louis, Missouri, USA", "Minneapolis, Hennepin, Minnesota, USA",
    "Leeds, Yorkshire, England", "York, Yorkshire, England", "Manchester, Lancashire, England", "London, Middlesex, England", "Bristol, Gloucestershire, England", "Glasgow, Lanarkshire, Scotland", "Inverness, Inverness-shire, Scotland",
    "Cork, Cork, Ireland", "Dublin, Dublin, Ireland", "Oslo, Akershus, Norway", "Bergen, Hordaland, Norway", "Copenhagen, Denmark", "Aarhus, Denmark", "Stockholm, Sweden", "Gothenburg, Sweden", "Hamburg, Germany",
    "Bremen, Germany", "Munich, Bavaria, Germany", "Zurich, Switzerland", "Lyon, France", "Turin, Piedmont, Italy", "Krakow, Poland", "Prague, Bohemia", "Toronto, Ontario, Canada", "Cardston, Alberta, Canada",
];
const MONTHS: &[&str] = &["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
const JOBS: &[&str] = &["Farmer", "Laborer", "Miner", "Teacher", "Carpenter", "Blacksmith", "Merchant", "Clerk", "Nurse", "Dressmaker", "Railroad worker", "Minister", "Doctor", "Sheep rancher", "Housekeeper"];

/// A tiny fixed random-number source, so the same size always makes the same tree.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.next() % 100 < pct
    }
    fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        v[self.below(v.len())]
    }
}

struct P {
    given: String,
    sur: String,
    male: bool,
    born: i32,
    died: Option<i32>,
    famc: Option<usize>,
    fams: Vec<usize>,
}

/// A GEDCOM file with about `n` people in families over many generations: births,
/// christenings, deaths and burials with places, marriages, occupations and residences,
/// notes, and citations of a few thousand sources.
pub(crate) fn big_gedcom(n: usize) -> String {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15 ^ n as u64);
    let mut people: Vec<P> = Vec::with_capacity(n + 16);
    let mut fams: Vec<(usize, usize, i32, Vec<usize>)> = vec![]; // husband, wife, year, children
    let new_person = |r: &mut Rng, people: &mut Vec<P>, male: bool, sur: Option<String>, born: i32| -> usize {
        let given = if r.chance(40) {
            format!("{} {}", r.pick(if male { MALE } else { FEMALE }), r.pick(if male { MALE } else { FEMALE }))
        } else {
            r.pick(if male { MALE } else { FEMALE }).to_string()
        };
        let sur = sur.unwrap_or_else(|| r.pick(SURNAMES).to_string());
        let life = 1 + r.below(95) as i32;
        let died = if born + life < 2025 { Some(born + life) } else { None };
        people.push(P { given, sur, male, born, died, famc: None, fams: vec![] });
        people.len() - 1
    };
    // founders, born 1700-1760
    let mut singles: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for _ in 0..(n / 200).max(4) {
        let b = 1700 + r.below(60) as i32;
        let m = r.chance(50);
        let i = new_person(&mut r, &mut people, m, None, b);
        singles.push_back(i);
    }
    let mut cursor = 0;
    while people.len() < n {
        // marry the next adult to a newcomer (or a cousin from the pool) and give them children
        if cursor >= people.len() {
            let b = 1700 + r.below(60) as i32;
            let m = r.chance(50);
            new_person(&mut r, &mut people, m, None, b);
        }
        let a = cursor;
        cursor += 1;
        if people[a].born > 2000 || r.chance(12) {
            continue; // stays single
        }
        let marriages = if r.chance(8) { 2 } else { 1 };
        for k in 0..marriages {
            let (am, ab) = (people[a].male, people[a].born);
            let sb = ab + r.below(9) as i32 - 4;
            let s = new_person(&mut r, &mut people, !am, None, sb);
            let (h, w) = if am { (a, s) } else { (s, a) };
            let year = ab.max(sb) + 19 + r.below(10) as i32 + 12 * k as i32;
            let fi = fams.len();
            people[h].fams.push(fi);
            people[w].fams.push(fi);
            let nk = r.below(9);
            let mut kids = vec![];
            let sur = people[h].sur.clone();
            for j in 0..nk {
                let kb = year + 1 + 2 * j as i32 + r.below(2) as i32;
                if kb > 2024 || people.len() >= n {
                    break;
                }
                let m = r.chance(50);
                let c = new_person(&mut r, &mut people, m, Some(sur.clone()), kb);
                people[c].famc = Some(fi);
                kids.push(c);
            }
            fams.push((h, w, year, kids));
        }
    }
    // write it out
    let mut o = String::with_capacity(n * 400);
    o.push_str("0 HEAD\n1 SOUR BIGTREE\n1 GEDC\n2 VERS 5.5.1\n1 CHAR UTF-8\n");
    let nsrc = (n / 50).max(10);
    let date = |r: &mut Rng, y: i32| -> String {
        match r.below(10) {
            0 => format!("ABT {}", y),
            1 | 2 => format!("{}", y),
            3 => format!("{} {}", MONTHS[r.below(12)], y),
            _ => format!("{} {} {}", 1 + r.below(28), MONTHS[r.below(12)], y),
        }
    };
    for (i, p) in people.iter().enumerate() {
        o.push_str(&format!("0 @I{}@ INDI\n1 NAME {} /{}/\n2 GIVN {}\n2 SURN {}\n1 SEX {}\n", i + 1, p.given, p.sur, p.given, p.sur, if p.male { "M" } else { "F" }));
        let bp = r.pick(PLACES);
        o.push_str(&format!("1 BIRT\n2 DATE {}\n2 PLAC {}\n", date(&mut r, p.born), bp));
        if r.chance(40) {
            o.push_str(&format!("2 SOUR @S{}@\n3 PAGE p. {}\n3 QUAY {}\n", 1 + r.below(nsrc), 1 + r.below(400), r.below(4)));
        }
        if r.chance(25) {
            o.push_str(&format!("1 CHR\n2 DATE {}\n2 PLAC {}\n", date(&mut r, p.born), bp));
        }
        if let Some(d) = p.died {
            let dp = r.pick(PLACES);
            o.push_str(&format!("1 DEAT\n2 DATE {}\n2 PLAC {}\n", date(&mut r, d), dp));
            if r.chance(50) {
                o.push_str(&format!("1 BURI\n2 DATE {}\n2 PLAC {}\n", date(&mut r, d), dp));
            }
        }
        if r.chance(30) && p.born < 1990 {
            o.push_str(&format!("1 OCCU {}\n2 DATE {}\n", r.pick(JOBS), p.born + 25));
        }
        if r.chance(20) {
            o.push_str(&format!("1 RESI\n2 DATE {}\n2 PLAC {}\n", p.born + 30, r.pick(PLACES)));
        }
        if r.chance(30) {
            o.push_str(&format!("1 NOTE {} was known in the family for {}. Letters from {} mention the farm near {}.\n", p.given, r.pick(&["music", "hard work", "a quick temper", "kindness", "stories", "carpentry"]), p.born + 20, r.pick(PLACES)));
        }
        if let Some(f) = p.famc {
            o.push_str(&format!("1 FAMC @F{}@\n", f + 1));
        }
        for f in &p.fams {
            o.push_str(&format!("1 FAMS @F{}@\n", f + 1));
        }
    }
    for (i, (h, w, y, kids)) in fams.iter().enumerate() {
        o.push_str(&format!("0 @F{}@ FAM\n1 HUSB @I{}@\n1 WIFE @I{}@\n1 MARR\n2 DATE {}\n2 PLAC {}\n", i + 1, h + 1, w + 1, date(&mut r, *y), r.pick(PLACES)));
        for k in kids {
            o.push_str(&format!("1 CHIL @I{}@\n", k + 1));
        }
    }
    for s in 0..nsrc {
        o.push_str(&format!(
            "0 @S{}@ SOUR\n1 TITL {} {} census, {}\n1 AUTH {} census office\n",
            s + 1,
            1850 + 10 * (s % 8),
            if s % 3 == 0 { "United States" } else { "county" },
            PLACES[s % PLACES.len()],
            if s % 2 == 0 { "US" } else { "State" }
        ));
    }
    o.push_str("0 TRLR\n");
    o
}

#[test]
fn the_big_tree_is_sensible() {
    let g = big_gedcom(2000);
    let db = gedcom::parse_ged(&g);
    assert!((1990..=2010).contains(&db.people.len()), "{}", db.people.len());
    assert!(db.fams.len() > 300);
    assert_eq!(big_gedcom(2000), g, "the same size makes the same tree");
    // a good share of people have parents and the tree is many generations deep
    let with_parents = db.people.keys().filter(|id| db.parents(**id) != (None, None)).count();
    assert!(with_parents * 2 > db.people.len(), "{with_parents}");
}

/// Times everything a big tree touches and prints a table. Run with
/// `BIG_N=100000 cargo test --release time_big_tree -- --ignored --nocapture`.
#[test]
#[ignore]
fn time_big_tree() {
    use std::time::Instant;
    let n: usize = std::env::var("BIG_N").ok().and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let tmp = std::env::temp_dir().join(format!("gr-big-{}", std::process::id()));
    std::fs::create_dir_all(tmp.join("Documents")).unwrap();
    std::env::set_var("HOME", &tmp);
    std::env::set_var("XDG_DOCUMENTS_DIR", tmp.join("Documents"));
    let rows: std::cell::RefCell<Vec<(String, f64)>> = Default::default();
    let t = |name: &str, f: &mut dyn FnMut()| {
        let s = Instant::now();
        f();
        let ms = s.elapsed().as_secs_f64() * 1000.0;
        println!("{:>10.1} ms  {}", ms, name);
        rows.borrow_mut().push((name.to_string(), ms));
    };
    let mut ged = String::new();
    t("make the test GEDCOM file", &mut || ged = big_gedcom(n));
    println!("           ({} MB of GEDCOM)", ged.len() / 1_000_000);
    let mut db = Db::default();
    t("import GEDCOM", &mut || db = gedcom::parse_ged(&ged));
    println!("           ({} people, {} families, {} sources)", db.people.len(), db.fams.len(), db.sources.len());
    let mut json = vec![];
    t("save the tree (to JSON)", &mut || json = serde_json::to_vec(&db).unwrap());
    println!("           ({} MB saved)", json.len() / 1_000_000);
    t("open the tree (from JSON)", &mut || db = serde_json::from_slice(&json).unwrap());
    let mut ix = db.index();
    t("index of families", &mut || ix = db.index());
    let some = *db.people.keys().nth(db.people.len() * 9 / 10).unwrap();
    let other = *db.people.keys().nth(db.people.len() / 3).unwrap();
    t("Find: search 'john smith'", &mut || { std::hint::black_box(db.sorted("john smith")); });
    t("Find: list everyone (empty search)", &mut || { std::hint::black_box(db.sorted("")); });
    t("ancestors of one person", &mut || { std::hint::black_box(db.ancestors(some)); });
    t("relationship between two people", &mut || { std::hint::black_box(db.relationship(some, other)); });
    t("export GEDCOM", &mut || { std::hint::black_box(gedcom::to_ged(&db)); });
    t("private copy (living left out)", &mut || { std::hint::black_box(db.sanitized()); });
    t("renumber: work out family order", &mut || { std::hint::black_box(gedcom::renumber_order(&db, gedcom::RenumberOrder::Family)); });
    t("group: birthplace contains Idaho", &mut || { std::hint::black_box(groups::matching(&db, 5, 0, "idaho")); });
    t("group: everyone connected", &mut || { std::hint::black_box(groups::related(&db, &ix, some, 3, 0, false, false)); });
    t("find duplicates", &mut || { std::hint::black_box(db.duplicate_pairs()); });
    let mut app = App::for_test(db.clone());
    app.sel = Some(some);
    t("list: everyone alphabetical", &mut || { std::hint::black_box(app.list_html(0)); });
    t("list: end-of-line ancestors", &mut || { std::hint::black_box(app.list_html(2)); });
    t("ancestor book", &mut || { std::hint::black_box(app.ancestor_book_html(some)); });
    t("map pins", &mut || { std::hint::black_box(app.map_pins()); });
    t("People table rows", &mut || { std::hint::black_box(app.table_rows()); });
    t("quick jump: 'john'", &mut || { std::hint::black_box(app.jump_results("john")); });
    t("copy the whole tree in memory", &mut || { std::hint::black_box(app.db.clone()); });
    t("undo check after a click (nothing changed)", &mut || { app.track(); });
    app.db.people.get_mut(&some).unwrap().notes.push_str(" Cooper.");
    t("undo check after an edit", &mut || { app.track(); });
    app.dirty = true;
    t("autosave: start (window carries on)", &mut || app.autosave());
    t("autosave: rest, in the background", &mut || app.wait_for_save());
    if let Some(e) = &app.save_error { panic!("save failed: {e}"); }
    // drawing each tab, as the program does every frame
    let ctx = egui::Context::default();
    let input = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0))), ..Default::default() };
    for (name, v) in [("Family", View::Family), ("Pedigree", View::Pedigree), ("Descendants", View::Descendants), ("Individual", View::Person), ("Gallery", View::Gallery), ("Charts", View::Charts), ("People", View::Table), ("Map", View::Map), ("Memories", View::Memories), ("DNA", View::Dna)] {
        app.view = v;
        let out = ctx.run(input(), |c| app.frame(c)); // first frame (sets things up)
        println!("           ({} shapes drawn)", out.shapes.len());
        let click = || { let mut i = input(); i.events.push(egui::Event::PointerButton { pos: egui::pos2(5.0, 1070.0), button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }); i };
        let s = Instant::now();
        let _ = ctx.run(click(), |c| app.frame(c));
        let ms = s.elapsed().as_secs_f64() * 1000.0;
        println!("{:>10.1} ms  draw the {} tab after a click", ms, name);
        rows.borrow_mut().push((format!("draw the {} tab after a click", name), ms));
        t(&format!("draw the {} tab, first frame", name), &mut || { let _ = ctx.run(input(), |c| app.frame(c)); });
        let s = Instant::now();
        for _ in 0..5 { let _ = ctx.run(input(), |c| app.frame(c)); }
        let ms = s.elapsed().as_secs_f64() * 1000.0 / 5.0;
        println!("{:>10.1} ms  draw the {} tab, each frame after", ms, name);
        rows.borrow_mut().push((format!("draw the {} tab, each frame after", name), ms));
    }
    println!("\nslowest:");
    let mut r = rows.borrow().clone();
    r.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (n, ms) in r.iter().take(12) {
        println!("{:>10.1} ms  {}", ms, n);
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Writes the possible duplicates in a made-up tree to a file, to compare one version of the
/// check with another.
#[test]
#[ignore]
fn dump_duplicates() {
    let n: usize = std::env::var("BIG_N").ok().and_then(|s| s.parse().ok()).unwrap_or(8000);
    let mut ged = big_gedcom(n);
    // some nicknames, spelling slips and copies, so every kind of match is tried
    let extra = ["Bill", "Wm", "Jno", "Polly", "Elisabeth", "Jhon", "Willam", "Mary Ann", "Jo", "Marie"];
    let mut add = String::new();
    for (k, g) in extra.iter().cycle().take(600).enumerate() {
        add.push_str(&format!("0 @X{k}@ INDI\n1 NAME {g} /{}/\n1 BIRT\n2 DATE {}\n2 PLAC Boise, Ada, Idaho, USA\n", ["Smith", "Smyth", "Jones", "Brown", "Browne"][k % 5], 1750 + (k % 60)));
    }
    ged = ged.replace("0 TRLR", &format!("{add}0 TRLR"));
    let db = gedcom::parse_ged(&ged);
    let t = std::time::Instant::now();
    let pairs = db.duplicate_pairs();
    eprintln!("{} pairs in {:?}", pairs.len(), t.elapsed());
    let out: Vec<String> = pairs.iter().map(|(a, b, w)| format!("{a} {b} {w}")).collect();
    std::fs::write(std::env::var("DUP_OUT").unwrap_or("/tmp/dups.txt".into()), out.join("\n")).unwrap();
}

#[test]
fn family_index_follows_every_change() {
    let mut db = gedcom::parse_ged(&big_gedcom(300));
    let (fid, kid) = {
        let (fid, f) = db.fams.iter().find(|(_, f)| f.h.is_some() && f.w.is_some()).unwrap();
        (*fid, *db.people.keys().find(|k| !f.kids.contains(k) && db.parents(**k) == (None, None)).unwrap())
    };
    let before = db.index();
    assert!(db.parent_families(kid).is_empty());
    // changed through get_mut: the remembered index must be thrown away
    db.fams.get_mut(&fid).unwrap().kids.push(kid);
    assert_eq!(db.parent_families(kid), vec![fid]);
    assert!(!std::sync::Arc::ptr_eq(&before, &db.index()));
    // and through a loop over every family
    for (_, f) in &mut db.fams {
        f.kids.retain(|k| *k != kid);
    }
    assert!(db.parent_families(kid).is_empty());
    // a copy answers the same and keeps its own index
    let copy = db.clone();
    db.fams.get_mut(&fid).unwrap().kids.push(kid);
    assert!(copy.parent_families(kid).is_empty());
    assert_eq!(db.parent_families(kid), vec![fid]);
    // taken out whole and put back
    let all = std::mem::take(&mut db.fams);
    assert!(db.parent_families(kid).is_empty());
    db.fams = all;
    assert_eq!(db.parent_families(kid), vec![fid]);
    // asking again without a change gives the remembered index
    assert!(std::sync::Arc::ptr_eq(&db.index(), &db.index()));
}

#[test]
fn saved_tree_leaves_out_empty_fields_and_reads_back_the_same() {
    let db = gedcom::parse_ged(&big_gedcom(300));
    let j = serde_json::to_string(&db).unwrap();
    assert!(!j.contains("\"chd\":\"\"") && !j.contains("\"media\":[]"), "empty fields are left out");
    let back: Db = serde_json::from_str(&j).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), j);
    // a file from an older version, with every empty field written out, still opens
    let p = db.people.values().next().unwrap();
    let mut v = serde_json::to_value(p).unwrap();
    for (k, e) in [("chd", serde_json::json!("")), ("media", serde_json::json!([])), ("living", serde_json::json!(false)), ("pref_parents", serde_json::Value::Null)] {
        if v.get(k).is_none() {
            v[k] = e;
        }
    }
    assert!(v.get("media").is_some());
    let old: Person = serde_json::from_value(v).unwrap();
    assert_eq!(serde_json::to_string(&old).unwrap(), serde_json::to_string(p).unwrap());
}

#[test]
fn long_lists_are_kept_until_the_tree_changes() {
    let mut app = App::for_test(gedcom::parse_ged(&big_gedcom(500)));
    let a = app.table_rows_kept();
    assert!(std::sync::Arc::ptr_eq(&a, &app.table_rows_kept()), "kept while nothing changes");
    assert!(!app.track(), "a click that changes nothing is not a change");
    assert!(std::sync::Arc::ptr_eq(&a, &app.table_rows_kept()));
    let id = a[0].id;
    app.db.people.get_mut(&id).unwrap().name = "Zebedee Zzyzx".into();
    assert!(app.track(), "an edit is noted");
    let b = app.table_rows_kept();
    assert!(!std::sync::Arc::ptr_eq(&a, &b), "redone after a change");
    assert!(b.iter().any(|r| r.name.contains("Zzyzx")));
    // a different filter is worked out straight away
    app.x.table_find = "zzyzx".into();
    assert_eq!(app.table_rows_kept().len(), 1);
    // undo puts the old name back, and the lists follow
    app.undo_action();
    assert!(app.table_rows_kept().is_empty());
}

/// A tree the size of a big real one stays quick. The limits are generous (a slow test build on
/// a busy machine passes easily); before this work, several of these took many times longer.
#[test]
fn big_tree_stays_quick() {
    use std::time::Instant;
    let n = if cfg!(debug_assertions) { 6_000 } else { 30_000 };
    let mut app = App::for_test(gedcom::parse_ged(&big_gedcom(n)));
    let ctx = egui::Context::default();
    let input = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
    let t = Instant::now();
    let dups = app.db.duplicate_pairs();
    assert!(t.elapsed().as_secs_f64() < 8.0, "duplicates took {:?}", t.elapsed());
    assert!(!dups.is_empty());
    let t = Instant::now();
    app.track();
    assert!(t.elapsed().as_secs_f64() < 3.0, "checking for a change took {:?}", t.elapsed());
    for v in [View::Family, View::Pedigree, View::Descendants, View::Person, View::Table, View::Map, View::Memories, View::Charts] {
        app.view = v;
        let _ = ctx.run(input(), |c| app.frame(c));
        let t = Instant::now();
        for _ in 0..3 {
            let _ = ctx.run(input(), |c| app.frame(c));
        }
        let each = t.elapsed().as_secs_f64() / 3.0;
        assert!(each < 0.25, "drawing {:?} took {each:.3}s a frame", v as u8);
    }
}

/// Writes a made-up tree as the program's own file, to try the program on a big tree:
/// `BIG_N=100000 SEED_OUT=/tmp/tree.json cargo test --release write_big_tree -- --ignored`.
#[test]
#[ignore]
fn write_big_tree() {
    let n: usize = std::env::var("BIG_N").ok().and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let db = gedcom::parse_ged(&big_gedcom(n));
    std::fs::write(std::env::var("SEED_OUT").unwrap_or("/tmp/big-tree.json".into()), serde_json::to_vec(&db).unwrap()).unwrap();
}
