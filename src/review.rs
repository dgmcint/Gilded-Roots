// Safer merging: comparing two records side by side, and the review windows that
// make the user confirm every merge. Nothing here merges people on a hunch.

use crate::gedcom::*;
use crate::*;
use eframe::egui::{self, Color32, RichText};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq)]
pub enum St {
    Same,
    /// One is a fuller version of the other ("Laura" and "Laura Jane").
    Compatible,
    Differ,
    OnlyLeft,
    OnlyRight,
}

pub struct Row {
    pub key: &'static str,
    pub label: &'static str,
    pub left: String,
    pub right: String,
    pub st: St,
}

pub struct Rel {
    pub label: String,
    pub left: String,
    pub right: String,
    pub st: St,
}

pub struct Warn {
    pub text: String,
    /// 1 = look at this, 2 = serious: these may well be two different people.
    pub level: u8,
}

pub struct Compare {
    pub rows: Vec<Row>,
    pub rels: Vec<Rel>,
    pub warns: Vec<Warn>,
    pub counts: Vec<(&'static str, usize, usize)>,
}

impl Compare {
    pub fn serious(&self) -> bool {
        self.warns.iter().any(|w| w.level >= 2)
    }
    pub fn conflicts(&self) -> usize {
        self.rows.iter().filter(|r| r.st == St::Differ).count()
    }
}

const FIELDS: &[(&str, &str)] = &[
    ("given", "Given names"),
    ("surname", "Surname"),
    ("maiden", "Maiden name"),
    ("sex", "Sex"),
    ("bd", "Born"),
    ("bp", "Birthplace"),
    ("chd", "Christened"),
    ("chp", "Christening place"),
    ("dd", "Died"),
    ("dp", "Place of death"),
    ("bud", "Buried"),
    ("bup", "Burial place"),
];

pub fn field(p: &Person, key: &str) -> String {
    match key {
        "given" => p.given.clone(),
        "surname" => p.surname.clone(),
        "maiden" => p.maiden.clone(),
        "sex" => p.sex.clone(),
        "bd" => p.bd.clone(),
        "bp" => p.bp.clone(),
        "chd" => p.chd.clone(),
        "chp" => p.chp.clone(),
        "dd" => p.dd.clone(),
        "dp" => p.dp.clone(),
        "bud" => p.bud.clone(),
        "bup" => p.bup.clone(),
        _ => String::new(),
    }
}

pub fn set_field(p: &mut Person, key: &str, v: String) {
    match key {
        "given" => p.given = v,
        "surname" => p.surname = v,
        "maiden" => p.maiden = v,
        "sex" => p.sex = v,
        "bd" => p.bd = v,
        "bp" => p.bp = v,
        "chd" => p.chd = v,
        "chp" => p.chp = v,
        "dd" => p.dd = v,
        "dp" => p.dp = v,
        "bud" => p.bud = v,
        "bup" => p.bup = v,
        _ => {}
    }
}

fn shown(key: &str, v: &str) -> String {
    if key == "sex" {
        match v {
            "M" => "Male".into(),
            "F" => "Female".into(),
            _ => v.to_string(),
        }
    } else {
        v.to_string()
    }
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn words(s: &str) -> HashSet<String> {
    norm(s).split(' ').map(String::from).filter(|w| !w.is_empty()).collect()
}

fn same_date(a: &str, b: &str) -> bool {
    norm(&ged_date(a)) == norm(&ged_date(b))
}

fn yr(s: &str) -> Option<i32> {
    year(s).parse().ok()
}

fn parent_pairs(db: &Db, id: u32) -> Vec<(Option<u32>, Option<u32>)> {
    db.fams.values().filter(|f| f.kids.contains(&id)).map(|f| (f.h, f.w)).collect()
}

fn rel_label(db: &Db, id: u32) -> String {
    db.people.get(&id).map(|_| db.label(id)).unwrap_or_default()
}

fn same_person_guess(da: &Db, a: u32, db: &Db, b: u32) -> bool {
    match (da.people.get(&a), db.people.get(&b)) {
        (Some(x), Some(y)) => {
            norm(&x.name) == norm(&y.name) && match (yr(&x.bd), yr(&y.bd)) {
                (Some(p), Some(q)) => p == q,
                _ => true,
            }
        }
        _ => false,
    }
}

/// Compare two people. `left` and `right` may be in different trees (importing) or the same one.
pub fn compare(ldb: &Db, l: u32, rdb: &Db, r: u32, same_tree: bool) -> Compare {
    let (lp, rp) = (&ldb.people[&l], &rdb.people[&r]);
    let mut rows = Vec::new();
    for (key, label) in FIELDS {
        let (a, b) = (field(lp, key), field(rp, key));
        if a.is_empty() && b.is_empty() {
            continue;
        }
        let st = if a.is_empty() {
            St::OnlyRight
        } else if b.is_empty() {
            St::OnlyLeft
        } else if norm(&a) == norm(&b) || (matches!(*key, "bd" | "dd" | "chd" | "bud") && same_date(&a, &b)) {
            St::Same
        } else if matches!(*key, "given" | "surname" | "maiden") && (words(&a).is_subset(&words(&b)) || words(&b).is_subset(&words(&a))) {
            St::Compatible
        } else {
            St::Differ
        };
        rows.push(Row { key, label, left: shown(key, &a), right: shown(key, &b), st });
    }

    // Parents, spouses and children
    let mut rels = Vec::new();
    let (lpp, rpp) = (parent_pairs(ldb, l), parent_pairs(rdb, r));
    for (label, pick) in [("Father", 0usize), ("Mother", 1usize)] {
        let get = |pp: &Vec<(Option<u32>, Option<u32>)>| -> Vec<u32> { pp.iter().filter_map(|p| if pick == 0 { p.0 } else { p.1 }).collect() };
        let (la, ra) = (get(&lpp), get(&rpp));
        if la.is_empty() && ra.is_empty() {
            continue;
        }
        let st = if la.is_empty() {
            St::OnlyRight
        } else if ra.is_empty() {
            St::OnlyLeft
        } else if la.iter().any(|a| ra.iter().any(|b| same_person_guess(ldb, *a, rdb, *b))) {
            St::Same
        } else {
            St::Differ
        };
        let j = |db: &Db, v: &Vec<u32>| v.iter().map(|i| rel_label(db, *i)).collect::<Vec<_>>().join(" / ");
        rels.push(Rel { label: label.to_string(), left: j(ldb, &la), right: j(rdb, &ra), st });
    }
    let side = |db: &Db, id: u32| -> (Vec<u32>, Vec<u32>) {
        let mut sp = Vec::new();
        let mut kids = Vec::new();
        for fid in db.fams_of(id) {
            let f = &db.fams[&fid];
            for s in [f.h, f.w].iter().flatten().filter(|s| **s != id) {
                sp.push(*s);
            }
            kids.extend(f.kids.iter().copied());
        }
        (sp, kids)
    };
    let ((ls, lk), (rs, rk)) = (side(ldb, l), side(rdb, r));
    for (label, la, ra) in [("Spouse", ls, rs), ("Child", lk, rk)] {
        let mut used = vec![false; ra.len()];
        for a in &la {
            let hit = ra.iter().enumerate().position(|(i, b)| !used[i] && same_person_guess(ldb, *a, rdb, *b));
            match hit {
                Some(i) => {
                    used[i] = true;
                    rels.push(Rel { label: label.into(), left: rel_label(ldb, *a), right: rel_label(rdb, ra[i]), st: St::Same });
                }
                None => rels.push(Rel { label: label.into(), left: rel_label(ldb, *a), right: String::new(), st: St::OnlyLeft }),
            }
        }
        for (i, b) in ra.iter().enumerate() {
            if !used[i] {
                rels.push(Rel { label: label.into(), left: String::new(), right: rel_label(rdb, *b), st: St::OnlyRight });
            }
        }
    }

    // Warnings
    let mut warns = vec![Warn { text: "A matching name and birth year does not prove these are the same person. Check each line below before you confirm.".into(), level: 0 }];
    if !lp.sex.is_empty() && !rp.sex.is_empty() && lp.sex != rp.sex {
        warns.push(Warn { text: "One is recorded as male and the other as female.".into(), level: 2 });
    }
    if let (Some(a), Some(b)) = (yr(&lp.bd), yr(&rp.bd)) {
        if (a - b).abs() > 2 {
            warns.push(Warn { text: format!("Birth years are {} years apart ({} and {}).", (a - b).abs(), a, b), level: 2 });
        }
    }
    if let (Some(a), Some(b)) = (yr(&lp.dd), yr(&rp.dd)) {
        if (a - b).abs() > 2 {
            warns.push(Warn { text: format!("Death years are {} years apart ({} and {}).", (a - b).abs(), a, b), level: 2 });
        }
    }
    for pick in ["Father", "Mother"] {
        if rels.iter().any(|r| r.label == pick && r.st == St::Differ) {
            warns.push(Warn { text: format!("Their {} is a different person in each record.", pick.to_lowercase()), level: 2 });
        }
    }
    if !rels.iter().any(|r| r.label == "Spouse" && r.st == St::Same) && rels.iter().any(|r| r.label == "Spouse" && r.st == St::OnlyLeft) && rels.iter().any(|r| r.label == "Spouse" && r.st == St::OnlyRight) {
        warns.push(Warn { text: "They have different spouses.".into(), level: 1 });
    }
    if !lp.bp.is_empty() && !rp.bp.is_empty() && norm(&lp.bp) != norm(&rp.bp) {
        warns.push(Warn { text: "The birthplaces differ.".into(), level: 1 });
    }
    if lp.living != rp.living {
        warns.push(Warn { text: "One is marked living and the other is not.".into(), level: 1 });
    }
    if same_tree {
        let anc_r = ldb.ancestors(r);
        let anc_l = ldb.ancestors(l);
        if anc_r.contains_key(&l) && l != r {
            warns.push(Warn { text: format!("{} is an ancestor of {}. One person cannot be their own ancestor.", display_name(lp), display_name(rp)), level: 2 });
        } else if anc_l.contains_key(&r) && l != r {
            warns.push(Warn { text: format!("{} is an ancestor of {}. One person cannot be their own ancestor.", display_name(rp), display_name(lp)), level: 2 });
        }
        if ldb.fams.values().any(|f| (f.h == Some(l) && f.w == Some(r)) || (f.h == Some(r) && f.w == Some(l))) {
            warns.push(Warn { text: "They are recorded as husband and wife.".into(), level: 2 });
        }
        if ldb.fams.values().any(|f| f.kids.contains(&l) && f.kids.contains(&r)) {
            warns.push(Warn { text: "Both are listed as children of the same parents. They may be brothers or sisters (even twins), or one entry may be a duplicate.".into(), level: 1 });
        }
    }
    let counts = vec![
        ("Events and facts", lp.events.len() + lp.fx.len(), rp.events.len() + rp.fx.len()),
        ("Source citations", lp.sources.len(), rp.sources.len()),
        ("Photos and documents", lp.media.len(), rp.media.len()),
        ("Other names", lp.alt_names.len(), rp.alt_names.len()),
        ("Linked notes", lp.note_refs.len() + usize::from(!lp.notes.is_empty()), rp.note_refs.len() + usize::from(!rp.notes.is_empty())),
    ];
    Compare { rows, rels, warns, counts }
}

fn st_text(st: St) -> (&'static str, Color32) {
    match st {
        St::Same => ("same", Color32::from_rgb(110, 170, 120)),
        St::Compatible => ("same, one is fuller", Color32::from_rgb(110, 170, 120)),
        St::Differ => ("DIFFERENT", Color32::from_rgb(225, 100, 90)),
        St::OnlyLeft => ("only on the left", Color32::from_rgb(150, 150, 160)),
        St::OnlyRight => ("only on the right", Color32::from_rgb(150, 150, 160)),
    }
}

/// Draw the comparison. With `picks`, a different or fuller value can be chosen by clicking it.
pub fn show_compare(ui: &mut egui::Ui, c: &Compare, picks: Option<&mut HashMap<&'static str, bool>>, ltitle: &str, rtitle: &str) {
    for w in &c.warns {
        let col = match w.level {
            2 => Color32::from_rgb(225, 100, 90),
            1 => Color32::from_rgb(214, 160, 60),
            _ => ui.visuals().weak_text_color(),
        };
        ui.label(RichText::new(if w.level >= 1 { format!("! {}", w.text) } else { w.text.clone() }).color(col));
    }
    ui.add_space(6.0);
    let mut picks = picks;
    egui::Grid::new("cmpgrid").num_columns(4).spacing([14.0, 5.0]).striped(true).show(ui, |ui| {
        ui.label("");
        ui.label(RichText::new(ltitle).strong());
        ui.label(RichText::new(rtitle).strong());
        ui.label("");
        ui.end_row();
        for r in &c.rows {
            ui.label(r.label);
            let choosable = matches!(r.st, St::Differ | St::Compatible);
            match (&mut picks, choosable) {
                (Some(p), true) => {
                    let right = p.get(r.key).copied().unwrap_or(false);
                    if ui.selectable_label(!right, &r.left).clicked() {
                        p.insert(r.key, false);
                    }
                    if ui.selectable_label(right, &r.right).clicked() {
                        p.insert(r.key, true);
                    }
                }
                _ => {
                    ui.label(&r.left);
                    ui.label(&r.right);
                }
            }
            let (t, col) = st_text(r.st);
            ui.label(RichText::new(t).small().color(col));
            ui.end_row();
        }
        for r in &c.rels {
            ui.label(&r.label);
            ui.label(&r.left);
            ui.label(&r.right);
            let (t, col) = st_text(match r.st {
                St::Differ => St::Differ,
                s => s,
            });
            ui.label(RichText::new(t).small().color(col));
            ui.end_row();
        }
        for (label, a, b) in &c.counts {
            if *a == 0 && *b == 0 {
                continue;
            }
            ui.label(RichText::new(*label).weak());
            ui.label(RichText::new(a.to_string()).weak());
            ui.label(RichText::new(b.to_string()).weak());
            ui.label(RichText::new("combined").small().weak());
            ui.end_row();
        }
    });
    if picks.is_some() && c.rows.iter().any(|r| matches!(r.st, St::Differ | St::Compatible)) {
        ui.label(RichText::new("Click the value to keep where the records differ. The other value is saved in the person's notes, so nothing is lost.").small().weak());
    }
}

/* ------------------------------------------------------------------ */
/* Merging two people already in the tree                               */
/* ------------------------------------------------------------------ */

pub struct PersonReview {
    pub left: u32,
    pub right: u32,
    pub picks: HashMap<&'static str, bool>,
    pub sure: bool,
    pub sure2: bool,
}

impl PersonReview {
    pub fn new(db: &Db, left: u32, right: u32) -> PersonReview {
        let mut r = PersonReview { left, right, picks: HashMap::new(), sure: false, sure2: false };
        r.defaults(db);
        r
    }
    /// Start with the left record's values, except that a fuller name wins over a shorter one.
    fn defaults(&mut self, db: &Db) {
        self.picks.clear();
        if db.people.contains_key(&self.left) && db.people.contains_key(&self.right) {
            let c = compare(db, self.left, db, self.right, true);
            for r in &c.rows {
                if r.st == St::Compatible && r.right.len() > r.left.len() {
                    self.picks.insert(r.key, true);
                }
            }
        }
    }
    pub fn swap(&mut self, db: &Db) {
        std::mem::swap(&mut self.left, &mut self.right);
        self.sure = false;
        self.sure2 = false;
        self.defaults(db);
    }
}

impl App {
    pub fn start_review(&mut self, a: u32, b: u32) {
        if a != b && self.db.people.contains_key(&a) && self.db.people.contains_key(&b) {
            self.review = Some(PersonReview::new(&self.db, a, b));
        }
    }

    pub fn merge_reviewed(&mut self, keep: u32, gone: u32, picks: &HashMap<&'static str, bool>) {
        self.db.merge_reviewed(keep, gone, picks);
        if self.sel == Some(gone) {
            self.sel = Some(keep);
        }
        self.dirty = true;
    }

    pub fn review_window(&mut self, ctx: &egui::Context) {
        let Some(mut rv) = self.review.take() else { return };
        if !self.db.people.contains_key(&rv.left) || !self.db.people.contains_key(&rv.right) {
            return;
        }
        let mut open = true;
        let mut do_merge = false;
        let mut swap = false;
        let mut go: Option<u32> = None;
        let c = compare(&self.db, rv.left, &self.db, rv.right, true);
        egui::Window::new("Review before merging").open(&mut open).default_size([760.0, 600.0]).show(ctx, |ui| {
            egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("The LEFT record is kept. The RIGHT record is folded into it and removed.").strong());
                    if ui.button("Swap sides").clicked() {
                        swap = true;
                    }
                });
                ui.horizontal(|ui| {
                    if ui.link(format!("Open left: {}  #{}", self.db.label(rv.left), rv.left)).clicked() {
                        go = Some(rv.left);
                    }
                    if ui.link(format!("Open right: {}  #{}", self.db.label(rv.right), rv.right)).clicked() {
                        go = Some(rv.right);
                    }
                });
                ui.separator();
                let (lt, rt) = (format!("Keep: {}", display_name(&self.db.people[&rv.left])), format!("Fold in: {}", display_name(&self.db.people[&rv.right])));
                show_compare(ui, &c, Some(&mut rv.picks), &lt, &rt);
                ui.add_space(8.0);
                ui.separator();
                ui.label("When merged, the person keeps every fact, event, source, photo and note from both records, and both sets of family links (a person with two sets of parents keeps both).");
                ui.add_space(4.0);
                ui.checkbox(&mut rv.sure, format!("I have compared these records{} and they are the same person.", if c.conflicts() > 0 { ", including the differences above," } else { "" }));
                if c.serious() {
                    ui.checkbox(&mut rv.sure2, "I understand the warnings marked ! above and still want to merge them.");
                }
                let ready = rv.sure && (!c.serious() || rv.sure2);
                ui.horizontal(|ui| {
                    if ui.add_enabled(ready, egui::Button::new("Merge these two people")).clicked() {
                        do_merge = true;
                    }
                    if !ready {
                        ui.label(RichText::new("Tick the box to enable merging.").small().weak());
                    }
                });
            });
        });
        if swap {
            rv.swap(&self.db);
        }
        if let Some(p) = go {
            self.sel = Some(p);
        }
        if do_merge {
            if self.backup() {
                let name = display_name(&self.db.people[&rv.left]);
                self.merge_reviewed(rv.left, rv.right, &rv.picks);
                self.sel = Some(rv.left);
                self.status = format!("Merged the two records into {}. Use Undo if this was a mistake.", name);
                self.dirty = true;
                self.dups.retain(|d| self.db.people.contains_key(&d.0) && self.db.people.contains_key(&d.1));
            }
            return;
        }
        if open {
            self.review = Some(rv);
        }
    }
}

/* ------------------------------------------------------------------ */
/* Adding another tree: every possible match is decided by the user    */
/* ------------------------------------------------------------------ */

pub struct ImportMerge {
    pub inc: Db,
    pub file: String,
    pub cands: Vec<MatchCand>,
    /// Incoming people who have at least one suggestion, best first.
    pub order: Vec<u32>,
    /// Incoming person -> person in the tree, confirmed as the same.
    pub same: HashMap<u32, u32>,
    /// Incoming people the user said are different from every suggestion.
    pub different: HashSet<u32>,
    pub sel: Option<u32>,
    pub opt: usize,
    pub bulk_ok: bool,
}

impl ImportMerge {
    pub fn new(dst: &Db, inc: Db, file: String) -> ImportMerge {
        let cands = merge_candidates(dst, &inc);
        let mut order: Vec<u32> = Vec::new();
        for c in &cands {
            if !order.contains(&c.inc) {
                order.push(c.inc);
            }
        }
        let sel = order.first().copied();
        ImportMerge { inc, file, cands, order, same: HashMap::new(), different: HashSet::new(), sel, opt: 0, bulk_ok: false }
    }
    fn options(&self, inc: u32) -> Vec<&MatchCand> {
        self.cands.iter().filter(|c| c.inc == inc).collect()
    }
    fn taken(&self, dst: u32, except: u32) -> bool {
        self.same.iter().any(|(i, d)| *d == dst && *i != except)
    }
    fn strong(&self) -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = Vec::new();
        let mut used: HashSet<u32> = HashSet::new();
        for i in &self.order {
            if let Some(c) = self.options(*i).first() {
                let second_close = self.options(*i).get(1).map_or(false, |n| n.score + 3 >= c.score);
                if c.strength() == "Strong" && !second_close && used.insert(c.dst) {
                    out.push((*i, c.dst));
                }
            }
        }
        out
    }
}

impl App {
    pub fn import_merge_window(&mut self, ctx: &egui::Context) {
        let Some(mut im) = self.imerge.take() else { return };
        let mut open = true;
        let mut finish = false;
        egui::Window::new("Add to my tree: check possible matches").open(&mut open).default_size([980.0, 640.0]).show(ctx, |ui| {
            let total = im.inc.people.len();
            let n_same = im.same.len();
            let undecided = im.order.iter().filter(|i| !im.same.contains_key(i) && !im.different.contains(i)).count();
            ui.label(format!(
                "{} contains {} people. {} of them look like someone already in your tree. Nothing is merged unless you confirm that pair.",
                im.file, total, im.order.len()
            ));
            ui.label(RichText::new("People you do not confirm are added as new, separate people. You can still merge them later from Tools > Find duplicates.").small().weak());
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    egui::ScrollArea::vertical().id_salt("imlist").max_height(430.0).show(ui, |ui| {
                        for i in im.order.clone() {
                            let p = &im.inc.people[&i];
                            let state = if im.same.contains_key(&i) {
                                "MERGE"
                            } else if im.different.contains(&i) {
                                "kept apart"
                            } else {
                                "undecided"
                            };
                            let label = format!("{}  [{}]", im.inc.label(i), state);
                            let _ = p;
                            if ui.selectable_label(im.sel == Some(i), label).clicked() {
                                im.sel = Some(i);
                                im.opt = 0;
                            }
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(600.0);
                    egui::ScrollArea::vertical().id_salt("imcmp").max_height(430.0).show(ui, |ui| {
                        let Some(inc_id) = im.sel.filter(|s| im.inc.people.contains_key(s)) else {
                            ui.label("No possible matches to check.");
                            return;
                        };
                        let opts: Vec<MatchCand> = im.options(inc_id).into_iter().cloned().collect();
                        if opts.is_empty() {
                            return;
                        }
                        if im.opt >= opts.len() {
                            im.opt = 0;
                        }
                        if opts.len() > 1 {
                            ui.label(RichText::new(format!("{} people in your tree could be this person. Choose which one to compare:", opts.len())).strong());
                            for (k, o) in opts.iter().enumerate() {
                                if ui.selectable_label(im.opt == k, format!("{}  #{}   {}", self.db.label(o.dst), o.dst, o.strength())).clicked() {
                                    im.opt = k;
                                }
                            }
                            ui.separator();
                        }
                        let cand = opts[im.opt].clone();
                        ui.label(RichText::new(format!("Why it looks like a match: {}", if cand.why.is_empty() { "nothing in particular".to_string() } else { cand.why.join(", ") })).small());
                        if !cand.against.is_empty() {
                            ui.label(RichText::new(format!("Against: {}", cand.against.join(", "))).small().color(Color32::from_rgb(225, 100, 90)));
                        }
                        ui.label(RichText::new(format!("Match strength: {}", cand.strength())).small().weak());
                        let c = compare(&self.db, cand.dst, &im.inc, inc_id, false);
                        show_compare(ui, &c, None, "In your tree", &format!("In {}", im.file));
                        if c.conflicts() > 0 {
                            ui.label(RichText::new("Where they differ, your tree's value is kept and the file's value is written into the person's notes.").small().weak());
                        }
                        ui.add_space(8.0);
                        let taken = im.taken(cand.dst, inc_id);
                        let is_same = im.same.get(&inc_id) == Some(&cand.dst);
                        ui.horizontal(|ui| {
                            if is_same {
                                ui.label(RichText::new("Confirmed: these will be merged.").strong());
                                if ui.button("Undo").clicked() {
                                    im.same.remove(&inc_id);
                                }
                            } else {
                                let can = !taken;
                                if ui.add_enabled(can, egui::Button::new("Yes, they are the same person: merge")).clicked() {
                                    im.same.insert(inc_id, cand.dst);
                                    im.different.remove(&inc_id);
                                }
                                if ui.button("No, different people: keep both").clicked() {
                                    im.same.remove(&inc_id);
                                    im.different.insert(inc_id);
                                }
                            }
                        });
                        if taken {
                            ui.label(RichText::new("Another person from the file is already confirmed as this person in your tree.").small().color(Color32::from_rgb(214, 160, 60)));
                        }
                        if c.serious() {
                            ui.label(RichText::new("Read the warnings marked ! before you confirm.").small().color(Color32::from_rgb(225, 100, 90)));
                        }
                    });
                });
            });
            ui.separator();
            let strong = im.strong();
            if !strong.is_empty() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut im.bulk_ok, format!("I have looked through the {} strong matches (same name, birth year and relatives)", strong.len()));
                    if ui.add_enabled(im.bulk_ok, egui::Button::new("Confirm all of them")).clicked() {
                        for (i, d) in &strong {
                            if !im.taken(*d, *i) {
                                im.same.insert(*i, *d);
                                im.different.remove(i);
                            }
                        }
                    }
                });
            }
            ui.label(format!(
                "Summary: {} will be merged into people already in your tree, {} possible matches you decided are different, {} possible matches undecided (added as new people), and {} people with no match added as new.",
                n_same,
                im.different.len(),
                undecided,
                total.saturating_sub(im.order.len())
            ));
            ui.horizontal(|ui| {
                if ui.button("Add to my tree").clicked() {
                    finish = true;
                }
                ui.label(RichText::new("A safety backup of your tree is made first.").small().weak());
            });
        });
        if finish {
            if self.backup() {
                let pairs = im.same.clone();
                let inc = std::mem::take(&mut im.inc);
                let (added, matched) = merge_into_pairs(&mut self.db, inc, &pairs);
                if self.sel.is_none() {
                    self.sel = self.db.people.keys().next().copied();
                }
                self.status = format!("Added {} new people and merged {} that you confirmed were already in your tree.", added, matched);
                self.dirty = true;
            }
            return;
        }
        if open {
            self.imerge = Some(im);
        } else {
            self.status = "Nothing was added.".to_string();
        }
    }
}

impl Db {
    /// Merge `gone` into `keep` after the user went through the review. Where the two records
    /// disagree, the chosen value is kept and the other is written into the notes.
    pub fn merge_reviewed(&mut self, keep: u32, gone: u32, picks: &HashMap<&'static str, bool>) {
        let (kp, gp) = match (self.people.get(&keep), self.people.get(&gone)) {
            (Some(k), Some(g)) => (k.clone(), g.clone()),
            _ => return,
        };
        let mut other: Vec<String> = Vec::new();
        let mut newvals: Vec<(&'static str, String)> = Vec::new();
        for (key, label) in FIELDS {
            let (a, b) = (field(&kp, key), field(&gp, key));
            if a.is_empty() || b.is_empty() || norm(&a) == norm(&b) {
                continue;
            }
            if words(&a).is_subset(&words(&b)) || words(&b).is_subset(&words(&a)) {
                if matches!(*key, "given" | "surname" | "maiden") {
                    if picks.get(key).copied().unwrap_or(false) {
                        newvals.push((key, b));
                    }
                    continue;
                }
            }
            if picks.get(key).copied().unwrap_or(false) {
                other.push(format!("{}: {}", label, shown(key, &a)));
                newvals.push((key, b));
            } else {
                other.push(format!("{}: {}", label, shown(key, &b)));
            }
        }
        if let Some(k) = self.people.get_mut(&keep) {
            for (key, v) in newvals {
                set_field(k, key, v);
            }
            if !other.is_empty() {
                let line = format!("Merged from a second record of this person. The other record said: {}.", other.join("; "));
                if !k.notes.is_empty() {
                    k.notes.push('\n');
                }
                k.notes.push_str(&line);
            }
        }
        self.merge_people(keep, gone);
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(db: &mut Db, name: &str, sex: &str, bd: &str, bp: &str) -> u32 {
        let id = db.alloc();
        let (g, s) = split_name(name);
        db.people.insert(id, Person { name: name.into(), given: g, surname: s, sex: sex.into(), bd: bd.into(), bp: bp.into(), ..Default::default() });
        id
    }

    fn couple(db: &mut Db, h: u32, w: u32, kids: &[u32]) -> u32 {
        let id = db.alloc();
        db.fams.insert(id, Family { h: Some(h), w: Some(w), kids: kids.to_vec(), ..Default::default() });
        id
    }

    /// Two different men, same name, same birth year, different families.
    fn two_johns() -> (Db, Db, u32, u32) {
        let mut a = Db::default();
        let john_a = person(&mut a, "John Smith", "M", "1850", "Leeds");
        let f = person(&mut a, "William Smith", "M", "1820", "");
        let m = person(&mut a, "Ann Smith", "F", "1825", "");
        couple(&mut a, f, m, &[john_a]);
        let mut b = Db::default();
        let john_b = person(&mut b, "John Smith", "M", "1850", "York");
        let f2 = person(&mut b, "Henry Smith", "M", "1822", "");
        let m2 = person(&mut b, "Mary Smith", "F", "1826", "");
        couple(&mut b, f2, m2, &[john_b]);
        (a, b, john_a, john_b)
    }

    #[test]
    fn same_name_and_birth_year_is_only_a_doubtful_suggestion() {
        let (a, b, ja, jb) = two_johns();
        let cands = merge_candidates(&a, &b);
        let c = cands.iter().find(|c| c.inc == jb && c.dst == ja).expect("suggested");
        assert!(c.why.iter().any(|w| w.contains("birth year")));
        assert!(c.against.iter().any(|w| w.contains("parents")), "{:?}", c.against);
        assert_eq!(c.strength(), "Doubtful");
    }

    #[test]
    fn nothing_is_merged_unless_the_user_confirms_the_pair() {
        let (mut a, b, ja, _) = two_johns();
        let before = a.people.len();
        let (added, matched) = merge_into_pairs(&mut a, b, &HashMap::new());
        assert_eq!((added, matched), (3, 0));
        assert_eq!(a.people.len(), before + 3);
        assert_eq!(a.people.values().filter(|p| p.name == "John Smith").count(), 2);
        assert!(a.people.contains_key(&ja));
    }

    #[test]
    fn a_confirmed_pair_merges_and_keeps_the_disagreeing_value_in_notes() {
        let (mut a, b, ja, jb) = two_johns();
        let mut pairs = HashMap::new();
        pairs.insert(jb, ja);
        merge_into_pairs(&mut a, b, &pairs);
        assert_eq!(a.people.values().filter(|p| p.name == "John Smith").count(), 1);
        let j = &a.people[&ja];
        assert_eq!(j.bp, "Leeds", "the tree's value stays");
        assert!(j.notes.contains("York"), "the other value is kept in the notes: {}", j.notes);
        // He is now the child of both sets of parents rather than one being thrown away.
        assert_eq!(a.fams.values().filter(|f| f.kids.contains(&ja)).count(), 2);
    }

    #[test]
    fn a_pair_cannot_be_confirmed_twice_onto_one_person() {
        let mut a = Db::default();
        let x = person(&mut a, "Eva Roe", "F", "1900", "");
        let mut b = Db::default();
        let y1 = person(&mut b, "Eva Roe", "F", "1900", "");
        let y2 = person(&mut b, "Eva Roe", "F", "1900", "");
        let mut pairs = HashMap::new();
        pairs.insert(y1, x);
        pairs.insert(y2, x);
        let (added, matched) = merge_into_pairs(&mut a, b, &pairs);
        assert_eq!((added, matched), (1, 1));
        assert_eq!(a.people.len(), 2);
    }

    #[test]
    fn suggestions_catch_fuller_names_and_skip_other_sexes() {
        let mut a = Db::default();
        let laura = person(&mut a, "Laura Jane Smith", "F", "", "");
        person(&mut a, "Laura Smith", "M", "", "");
        let mut b = Db::default();
        let l2 = person(&mut b, "Laura Smith", "F", "", "");
        let cands = merge_candidates(&a, &b);
        assert_eq!(cands.len(), 1, "the man is not suggested");
        assert_eq!((cands[0].inc, cands[0].dst), (l2, laura));
        assert_eq!(cands[0].strength(), "Weak (little to go on)");
    }

    #[test]
    fn comparison_shows_conflicts_and_serious_warnings() {
        let (a, b, ja, jb) = two_johns();
        let c = compare(&a, ja, &b, jb, false);
        let bp = c.rows.iter().find(|r| r.key == "bp").unwrap();
        assert!(bp.st == St::Differ);
        assert!(c.rows.iter().find(|r| r.key == "bd").unwrap().st == St::Same);
        assert!(c.rels.iter().any(|r| r.label == "Father" && r.st == St::Differ));
        assert!(c.serious());
        assert_eq!(c.conflicts(), 1);
    }

    #[test]
    fn comparison_refuses_to_pretend_relatives_are_one_person() {
        let mut db = Db::default();
        let dad = person(&mut db, "Tom Lee", "M", "1900", "");
        let mum = person(&mut db, "Sue Lee", "F", "1902", "");
        let son = person(&mut db, "Tom Lee", "M", "1900", "");
        couple(&mut db, dad, mum, &[son]);
        let c = compare(&db, dad, &db, son, true);
        assert!(c.warns.iter().any(|w| w.level == 2 && w.text.contains("ancestor")), "father and son with the same name and year");
        let c2 = compare(&db, dad, &db, mum, true);
        assert!(c2.warns.iter().any(|w| w.text.contains("husband and wife")));
        assert!(c2.warns.iter().any(|w| w.text.contains("male and the other as female")));
    }

    #[test]
    fn reviewed_merge_in_the_tree_uses_the_chosen_values_and_notes_the_rest() {
        let mut db = Db::default();
        let l = person(&mut db, "Laura Smith", "F", "1950", "Leeds");
        let r = person(&mut db, "Laura Jane Smith", "F", "1950", "York");
        let kid = person(&mut db, "Kid", "M", "1980", "");
        let hub = person(&mut db, "Hub", "M", "1948", "");
        couple(&mut db, hub, r, &[kid]);
        let mut picks: HashMap<&'static str, bool> = HashMap::new();
        picks.insert("given", true); // the fuller name
        picks.insert("bp", false); // keep Leeds
        db.merge_reviewed(l, r, &picks);
        assert!(!db.people.contains_key(&r));
        let p = &db.people[&l];
        assert_eq!((p.given.as_str(), p.bp.as_str(), p.name.as_str()), ("Laura Jane", "Leeds", "Laura Jane Smith"));
        assert!(p.notes.contains("York"));
        assert!(db.fams.values().any(|f| f.w == Some(l) && f.kids.contains(&kid)), "the family now points at the kept person");
        // choosing the right-hand value instead
        let mut db2 = Db::default();
        let l = person(&mut db2, "Laura Smith", "F", "1950", "Leeds");
        let r = person(&mut db2, "Laura Smith", "F", "1950", "York");
        let mut picks = HashMap::new();
        picks.insert("bp", true);
        db2.merge_reviewed(l, r, &picks);
        assert_eq!(db2.people[&l].bp, "York");
        assert!(db2.people[&l].notes.contains("Leeds"));
    }

    #[test]
    fn strong_matches_need_real_corroboration() {
        let mut a = Db::default();
        let f = person(&mut a, "William Smith", "M", "1820", "");
        let m = person(&mut a, "Ann Smith", "F", "1825", "");
        let j = person(&mut a, "John Smith", "M", "1850", "Leeds");
        couple(&mut a, f, m, &[j]);
        let mut b = Db::default();
        let f2 = person(&mut b, "William Smith", "M", "1820", "");
        let m2 = person(&mut b, "Ann Smith", "F", "1825", "");
        let j2 = person(&mut b, "John Smith", "M", "1850", "Leeds");
        couple(&mut b, f2, m2, &[j2]);
        let cands = merge_candidates(&a, &b);
        let top = cands.iter().find(|c| c.inc == j2 && c.dst == j).unwrap();
        assert_eq!(top.strength(), "Strong");
        let im = ImportMerge::new(&a, b, "x.ged".into());
        assert!(im.strong().iter().any(|p| *p == (j2, j)));
    }
}
