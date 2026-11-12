//! Notes across the tree: find and replace in every note (showing each match first, with a
//! tick box), topic tags for notes, and private lines (starting with ~) that stay off paper.

use super::*;

/// Where a note is kept.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum NoteAt {
    Person(u32),
    PersonFact(u32, String),
    PersonEvent(u32, usize),
    Family(u32),
    FamilyEvent(u32, usize),
    Shared(u32),
    Source(u32),
    Repo(u32),
}

/// Topic tags a note line can start with, so notes read in order and can be found by topic.
pub(crate) const NOTE_TAGS: &[&str] = &[
    "Birth", "Christening", "Marriage", "Death", "Burial", "Military", "Occupation", "Residence", "Immigration", "Religion", "Education", "Medical", "Research", "Story",
];

/// Every note in the tree, with where it is.
pub(crate) fn all_notes(db: &Db) -> Vec<(NoteAt, &String)> {
    let mut v: Vec<(NoteAt, &String)> = vec![];
    for (id, p) in &db.people {
        v.push((NoteAt::Person(*id), &p.notes));
        for (k, e) in &p.fx {
            v.push((NoteAt::PersonFact(*id, k.clone()), &e.note));
        }
        for (i, e) in p.events.iter().enumerate() {
            v.push((NoteAt::PersonEvent(*id, i), &e.note));
        }
    }
    for (id, f) in &db.fams {
        v.push((NoteAt::Family(*id), &f.notes));
        for (i, e) in f.events.iter().enumerate() {
            v.push((NoteAt::FamilyEvent(*id, i), &e.note));
        }
    }
    for (id, n) in &db.notes {
        v.push((NoteAt::Shared(*id), &n.text));
    }
    for (id, s) in &db.sources {
        v.push((NoteAt::Source(*id), &s.notes));
    }
    for (id, r) in &db.repos {
        v.push((NoteAt::Repo(*id), &r.notes));
    }
    v.retain(|(_, t)| !t.is_empty());
    v
}

pub(crate) fn note_mut<'a>(db: &'a mut Db, at: &NoteAt) -> Option<&'a mut String> {
    match at {
        NoteAt::Person(id) => db.people.get_mut(id).map(|p| &mut p.notes),
        NoteAt::PersonFact(id, k) => db.people.get_mut(id).and_then(|p| p.fx.get_mut(k)).map(|e| &mut e.note),
        NoteAt::PersonEvent(id, i) => db.people.get_mut(id).and_then(|p| p.events.get_mut(*i)).map(|e| &mut e.note),
        NoteAt::Family(id) => db.fams.get_mut(id).map(|f| &mut f.notes),
        NoteAt::FamilyEvent(id, i) => db.fams.get_mut(id).and_then(|f| f.events.get_mut(*i)).map(|e| &mut e.note),
        NoteAt::Shared(id) => db.notes.get_mut(id).map(|n| &mut n.text),
        NoteAt::Source(id) => db.sources.get_mut(id).map(|s| &mut s.notes),
        NoteAt::Repo(id) => db.repos.get_mut(id).map(|r| &mut r.notes),
    }
}

/// Who or what a note belongs to, in words.
pub(crate) fn note_owner(db: &Db, at: &NoteAt) -> String {
    let who = |id: &u32| db.people.get(id).map(display_name).unwrap_or_default();
    let fam = |id: &u32| {
        db.fams
            .get(id)
            .map(|f| [f.h, f.w].into_iter().flatten().filter_map(|x| db.people.get(&x)).map(display_name).collect::<Vec<_>>().join(" & "))
            .unwrap_or_default()
    };
    match at {
        NoteAt::Person(id) => who(id),
        NoteAt::PersonFact(id, k) => format!("{} ({})", who(id), fact_name(k)),
        NoteAt::PersonEvent(id, i) => format!("{} ({})", who(id), db.people.get(id).and_then(|p| p.events.get(*i)).map(|e| e.kind.clone()).unwrap_or_default()),
        NoteAt::Family(id) => format!("Family: {}", fam(id)),
        NoteAt::FamilyEvent(id, i) => format!("Family: {} ({})", fam(id), db.fams.get(id).and_then(|f| f.events.get(*i)).map(|e| e.kind.clone()).unwrap_or_default()),
        NoteAt::Shared(id) => format!("Shared note #{}", id),
        NoteAt::Source(id) => format!("Source: {}", db.sources.get(id).map(source_title).unwrap_or_default()),
        NoteAt::Repo(id) => format!("Repository: {}", db.repos.get(id).map(|r| r.name.clone()).unwrap_or_default()),
    }
}

fn fact_name(k: &str) -> String {
    match k {
        "BIRT" => "birth".into(),
        "CHR" => "christening".into(),
        "DEAT" => "death".into(),
        "BURI" => "burial".into(),
        _ => k.to_lowercase(),
    }
}

/// Where `find` occurs in `text` (byte ranges), ignoring case unless `case` is set. With
/// `whole`, only where it is a whole word.
pub(crate) fn find_in(text: &str, find: &str, case: bool, whole: bool) -> Vec<(usize, usize)> {
    if find.is_empty() {
        return vec![];
    }
    let mut out = vec![];
    let (hay, needle) = if case { (text.to_string(), find.to_string()) } else { (text.to_lowercase(), find.to_lowercase()) };
    // Lower-casing can change byte lengths for a few letters; then only exact-case matching is safe.
    if hay.len() != text.len() || needle.len() != find.len() {
        return if case { vec![] } else { find_in(text, find, true, whole) };
    }
    let mut from = 0;
    while let Some(i) = hay[from..].find(&needle) {
        let s = from + i;
        let e = s + needle.len();
        let word = |c: Option<char>| c.map_or(false, |c| c.is_alphanumeric() || c == '_');
        if !whole || (!word(text[..s].chars().next_back()) && !word(text[e..].chars().next())) {
            out.push((s, e));
        }
        from = e.max(s + 1);
        while from < text.len() && !text.is_char_boundary(from) {
            from += 1;
        }
    }
    out
}

/// `text` with every match of `find` replaced.
pub(crate) fn replace_in(text: &str, find: &str, with: &str, case: bool, whole: bool) -> String {
    let mut out = String::new();
    let mut last = 0;
    for (s, e) in find_in(text, find, case, whole) {
        out.push_str(&text[last..s]);
        out.push_str(with);
        last = e;
    }
    out.push_str(&text[last..]);
    out
}

/// A note without its private lines (lines starting with ~), for printing and private copies.
pub(crate) fn public_text(t: &str) -> String {
    if !t.contains('~') {
        return t.to_string();
    }
    t.lines().filter(|l| !l.trim_start().starts_with('~')).collect::<Vec<_>>().join("\n")
}

/// The tree with private note lines taken out.
pub(crate) fn without_private_notes(mut db: Db) -> Db {
    for p in db.people.values_mut() {
        p.notes = public_text(&p.notes);
        for e in p.fx.values_mut().chain(p.events.iter_mut()) {
            e.note = public_text(&e.note);
        }
    }
    for f in db.fams.values_mut() {
        f.notes = public_text(&f.notes);
        for e in f.events.iter_mut() {
            e.note = public_text(&e.note);
        }
    }
    for n in db.notes.values_mut() {
        n.text = public_text(&n.text);
    }
    for s in db.sources.values_mut() {
        s.notes = public_text(&s.notes);
    }
    db.each_cite_mut(&mut |c| c.note = public_text(&c.note));
    db
}

/// A short bit of the note around a match, for the list of matches.
fn context(text: &str, s: usize, e: usize) -> (String, String, String) {
    let mut a = s.saturating_sub(40);
    while !text.is_char_boundary(a) {
        a -= 1;
    }
    let mut b = (e + 40).min(text.len());
    while !text.is_char_boundary(b) {
        b += 1;
    }
    let pre = format!("{}{}", if a > 0 { "..." } else { "" }, &text[a..s]).replace('\n', " ");
    let post = format!("{}{}", &text[e..b], if b < text.len() { "..." } else { "" }).replace('\n', " ");
    (pre, text[s..e].to_string(), post)
}

impl App {
    pub(crate) fn notes_replace_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.nfr_open {
            return;
        }
        let mut open = true;
        let mut apply = false;
        egui::Window::new("Find and replace in notes").open(&mut open).default_size([640.0, 520.0]).show(ctx, |ui| {
            egui::Grid::new("nfr").num_columns(2).show(ui, |ui| {
                ui.label("Find");
                if ui.add(egui::TextEdit::singleline(&mut self.x.nfr_find).desired_width(320.0)).changed() {
                    self.x.nfr_skip.clear();
                }
                ui.end_row();
                ui.label("Replace with");
                ui.add(egui::TextEdit::singleline(&mut self.x.nfr_with).hint_text("leave empty to delete the words").desired_width(320.0));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.checkbox(&mut self.x.nfr_case, "Match capitals").changed() || ui.checkbox(&mut self.x.nfr_whole, "Whole words only").changed() {
                    self.x.nfr_skip.clear();
                }
            });
            ui.separator();
            let hits: Vec<(NoteAt, usize, (String, String, String))> = all_notes(&self.db)
                .into_iter()
                .flat_map(|(at, t)| {
                    find_in(t, &self.x.nfr_find, self.x.nfr_case, self.x.nfr_whole).into_iter().map(move |(s, e)| (at.clone(), s, context(t, s, e)))
                })
                .collect();
            if self.x.nfr_find.is_empty() {
                ui.weak("Type the words to find. Every note is searched: people, families, events, shared notes, sources and repositories.");
                return;
            }
            let notes_hit: Vec<&NoteAt> = {
                let mut v: Vec<&NoteAt> = hits.iter().map(|h| &h.0).collect();
                v.dedup();
                v
            };
            ui.label(format!("{} matches in {} notes. Untick any note to leave it as it is.", hits.len(), notes_hit.len()));
            let ticked = notes_hit.iter().filter(|a| !self.x.nfr_skip.contains(&format!("{:?}", a))).count();
            ui.horizontal(|ui| {
                if ui.add_enabled(ticked > 0, egui::Button::new(format!("Replace in {} ticked notes", ticked))).clicked() {
                    apply = true;
                }
                ui.label(RichText::new("Undo (Ctrl+Z) puts them back.").small().weak());
            });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let mut last: Option<&NoteAt> = None;
                for (at, _, (pre, m, post)) in hits.iter().take(1000) {
                    if last != Some(at) {
                        last = Some(at);
                        let key = format!("{:?}", at);
                        ui.horizontal(|ui| {
                            let mut on = !self.x.nfr_skip.contains(&key);
                            if ui.checkbox(&mut on, "").changed() {
                                if on {
                                    self.x.nfr_skip.remove(&key);
                                } else {
                                    self.x.nfr_skip.insert(key.clone());
                                }
                            }
                            let owner = note_owner(&self.db, at);
                            let person = match at {
                                NoteAt::Person(i) | NoteAt::PersonFact(i, _) | NoteAt::PersonEvent(i, _) => Some(*i),
                                NoteAt::Family(f) | NoteAt::FamilyEvent(f, _) => self.db.fams.get(f).and_then(|f| f.h.or(f.w)),
                                _ => None,
                            };
                            match person {
                                Some(p) => {
                                    if ui.link(RichText::new(owner).strong()).clicked() {
                                        *act = Some(Action::Select(p));
                                    }
                                }
                                None => {
                                    ui.label(RichText::new(owner).strong());
                                }
                            }
                        });
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(28.0);
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label(RichText::new(pre).weak());
                        ui.label(RichText::new(m).strong().background_color(ui.visuals().selection.bg_fill));
                        ui.label(RichText::new(format!("  ->  {}  ", self.x.nfr_with)).italics());
                        ui.label(RichText::new(post).weak());
                    });
                }
                if hits.len() > 1000 {
                    ui.weak(format!("...and {} more matches (all are replaced).", hits.len() - 1000));
                }
            });
        });
        if apply {
            let targets: Vec<NoteAt> = {
                let mut v: Vec<NoteAt> = all_notes(&self.db)
                    .into_iter()
                    .filter(|(at, t)| !find_in(t, &self.x.nfr_find, self.x.nfr_case, self.x.nfr_whole).is_empty() && !self.x.nfr_skip.contains(&format!("{:?}", at)))
                    .map(|(at, _)| at)
                    .collect();
                v.dedup();
                v
            };
            let (f, w, c, wh) = (self.x.nfr_find.clone(), self.x.nfr_with.clone(), self.x.nfr_case, self.x.nfr_whole);
            let mut n = 0;
            for at in &targets {
                if let Some(t) = note_mut(&mut self.db, at) {
                    *t = replace_in(t, &f, &w, c, wh);
                    n += 1;
                }
            }
            self.x.next_label = Some(format!("Replaced \"{}\" in {} notes", f, n));
            self.dirty = true;
            self.status = format!("Replaced \"{}\" with \"{}\" in {} notes.", f, w, n);
        }
        self.x.nfr_open = open;
    }
}

/// A menu of topic tags for a note: picking one starts a new line "Tag: " at the end.
pub(crate) fn note_tag_menu(ui: &mut egui::Ui, text: &mut String) {
    ui.menu_button("Add a tag", |ui| {
        ui.label(RichText::new("Starts a new line with the topic, e.g. \"Military: \"").small().weak());
        for t in NOTE_TAGS {
            if ui.button(*t).clicked() {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&format!("{}: ", t));
                ui.close_menu();
            }
        }
        ui.separator();
        if ui.button("Private line (~)").on_hover_text("A line starting with ~ is kept in the tree but left off printouts and private copies").clicked() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str("~ ");
            ui.close_menu();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_and_replacing_words() {
        let t = "Twin Falls, ID. Moved from twin falls in 1950. Twin Fallsville is elsewhere.";
        assert_eq!(find_in(t, "twin falls", false, false).len(), 3);
        assert_eq!(find_in(t, "twin falls", false, true).len(), 2, "not inside Twin Fallsville");
        assert_eq!(find_in(t, "Twin Falls", true, false).len(), 2);
        assert_eq!(replace_in(t, "twin falls", "Twin Falls, Idaho", false, true), "Twin Falls, Idaho, ID. Moved from Twin Falls, Idaho in 1950. Twin Fallsville is elsewhere.");
        assert_eq!(replace_in("aaa", "a", "b", true, false), "bbb");
        assert!(find_in("x", "", false, false).is_empty());
        // letters outside plain ASCII
        assert_eq!(find_in("Müller und müller", "MÜLLER", false, false).len(), 2);
    }

    #[test]
    fn replacing_across_the_tree_reaches_every_kind_of_note() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME Ann /Lee/\n1 NOTE Lived in Twin Falls.\n1 BIRT\n2 DATE 1900\n2 NOTE Born in Twin Falls\n0 @F1@ FAM\n1 HUSB @I1@\n1 NOTE Farm near Twin Falls\n0 @S1@ SOUR\n1 TITL Twin Falls Times\n1 NOTE Twin Falls paper\n0 @N1@ NOTE Shared: Twin Falls\n0 TRLR\n";
        let mut db = gedcom::parse_ged(t);
        let all = all_notes(&db);
        let hits: usize = all.iter().map(|(_, t)| find_in(t, "twin falls", false, false).len()).sum();
        assert_eq!(hits, 5, "{:?}", all.iter().map(|(a, t)| (a, t.to_string())).collect::<Vec<_>>());
        let ats: Vec<NoteAt> = all.into_iter().map(|(a, _)| a).collect();
        for at in &ats {
            let t = note_mut(&mut db, at).unwrap();
            *t = replace_in(t, "twin falls", "Twin Falls, Idaho", false, true);
        }
        let after: usize = all_notes(&db).iter().map(|(_, t)| find_in(t, "Twin Falls, Idaho", true, false).len()).sum();
        assert_eq!(after, 5);
        assert!(ats.iter().all(|a| !note_owner(&db, a).is_empty()));
    }

    #[test]
    fn private_lines_stay_off_paper() {
        assert_eq!(public_text("Kept.\n~ Not for printing.\nAlso kept."), "Kept.\nAlso kept.");
        assert_eq!(public_text("No tilde"), "No tilde");
        let mut db = Db::default();
        db.people.insert(1, Person { name: "A".into(), notes: "Open\n~secret".into(), ..Default::default() });
        let d = without_private_notes(db);
        assert_eq!(d.people[&1].notes, "Open");
    }
}
