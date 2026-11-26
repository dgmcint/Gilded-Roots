//! Search everything (Search > Search everything, Cmd/Ctrl+Shift+F): one search through names,
//! facts, places, notes, to-do lists, citations, sources, stories, captions, research files,
//! the notepad and the research journal. Results are grouped by person.

use super::*;

/// Where a match was found, so clicking it can go there.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Hit {
    Person(u32),
    Family(u32),
    Source(u32),
    Research(u32),
    Notepad(u32),
    Theory(u32),
    Letter(u32),
    Journal,
}

/// One match: where it is, what part it is in ("Birth place", "Notes") and the text around it.
#[derive(Clone, Debug)]
pub(crate) struct Found {
    pub(crate) at: Hit,
    pub(crate) part: String,
    pub(crate) text: String,
}

/// The words around the first match, on one line.
fn snippet(text: &str, words: &[String]) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let low = flat.to_lowercase();
    let at = words.iter().filter_map(|w| low.find(w.as_str())).min().unwrap_or(0);
    let chars: Vec<char> = flat.chars().collect();
    // the byte position as a char position
    let ci = flat[..at.min(flat.len())].chars().count();
    let start = ci.saturating_sub(40);
    let end = (ci + 80).min(chars.len());
    let mut s: String = chars[start..end].iter().collect();
    if start > 0 {
        s = format!("…{s}");
    }
    if end < chars.len() {
        s.push('…');
    }
    s
}

/// The name of the part a match was found in, in the chosen language.
fn part_label(p: &str) -> String {
    let tr = crate::i18n::tr;
    match p {
        "To-do" => tr("To-do").to_string(),
        "Citation" => tr("Citation").to_string(),
        "Contact" => tr("Contact").to_string(),
        "Family notes" => tr("Family notes").to_string(),
        "Research file" => tr("Research file").to_string(),
        "Notepad" => tr("Notepad").to_string(),
        "Theory" => tr("Theory").to_string(),
        "Letter" => tr("Letter").to_string(),
        _ => crate::i18n::tr_dyn(p),
    }
}

/// Does the text hold every word?
fn has(text: &str, words: &[String]) -> bool {
    if text.is_empty() {
        return false;
    }
    let low = text.to_lowercase();
    words.iter().all(|w| low.contains(w.as_str()))
}

/// A story's text, read from its file (only small text files).
fn story_text(file: &str) -> String {
    let p = mfile(file);
    match std::fs::metadata(&p) {
        Ok(m) if m.len() < 400_000 && matches!(ext_of(file).as_str(), "txt" | "md" | "htm" | "html") => {
            let t = std::fs::read_to_string(&p).unwrap_or_default();
            if ext_of(file).starts_with("htm") { crate::research::html_to_text(&t) } else { t }
        }
        _ => String::new(),
    }
}

/// Everything that matches every word typed (in any order, any case).
pub(crate) fn search_everything(db: &Db, q: &str) -> Vec<Found> {
    let words: Vec<String> = q.split_whitespace().map(|w| w.to_lowercase()).collect();
    let mut out = Vec::new();
    if words.is_empty() || q.trim().chars().count() < 2 {
        return out;
    }
    let add = |out: &mut Vec<Found>, at: Hit, part: &str, text: &str| {
        if has(text, &words) {
            out.push(Found { at, part: part.to_string(), text: snippet(text, &words) });
        }
    };
    for (id, p) in &db.people {
        let at = Hit::Person(*id);
        let mut names = vec![p.name.clone(), p.nick.clone(), p.maiden.clone()];
        names.extend(p.alt_names.iter().map(|a| format!("{} {}", a.given, a.surname)));
        add(&mut out, at.clone(), "Name", &names.join("  ·  ").trim_matches(|c: char| c == ' ' || c == '·').to_string());
        for (k, d, pl) in [("Birth", &p.bd, &p.bp), ("Christening", &p.chd, &p.chp), ("Death", &p.dd, &p.dp), ("Burial", &p.bud, &p.bup)] {
            add(&mut out, at.clone(), k, &format!("{d} {pl}").trim().to_string());
        }
        for e in &p.events {
            add(&mut out, at.clone(), &e.kind, &[e.date.as_str(), e.place.as_str(), e.val.as_str(), e.typ.as_str(), e.cause.as_str(), e.note.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(", "));
        }
        add(&mut out, at.clone(), "Notes", &p.notes);
        for r in &p.note_refs {
            if let Some(n) = db.notes.get(r) {
                add(&mut out, at.clone(), "Notes", &n.text);
            }
        }
        for t in &p.todos {
            add(&mut out, at.clone(), "To-do", &format!("{} {}", t.text, t.note));
        }
        for c in &p.sources {
            let title = c.sid.and_then(|s| db.sources.get(&s)).map(source_title).unwrap_or_default();
            add(&mut out, at.clone(), "Citation", &[title.as_str(), c.page.as_str(), c.text.as_str(), c.data.as_str(), c.note.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · "));
        }
        for m in &p.media {
            if m.form == "story" {
                let t = story_text(&m.file);
                add(&mut out, at.clone(), "Story", &format!("{} {}", m.caption, t));
            } else {
                add(&mut out, at.clone(), "Caption", &format!("{} {}", m.caption, m.name));
            }
        }
        add(&mut out, at.clone(), "Contact", &format!("{} {} {}", p.contact.addr, p.contact.email, p.contact.url));
    }
    for (fid, f) in db.fams.iter() {
        let at = Hit::Family(*fid);
        add(&mut out, at.clone(), "Marriage", &format!("{} {}", f.md, f.mp));
        add(&mut out, at.clone(), "Family notes", &f.notes);
        for e in &f.events {
            add(&mut out, at.clone(), &e.kind, &format!("{} {} {}", e.date, e.place, e.note));
        }
    }
    for (sid, s) in &db.sources {
        add(&mut out, Hit::Source(*sid), "Source", &[s.title.as_str(), s.author.as_str(), s.abbr.as_str(), s.publ.as_str(), s.text.as_str(), s.notes.as_str(), s.call.as_str(), s.url.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · "));
    }
    for r in &db.research {
        let mut t = format!("{} {} {} {}", r.title, r.date, r.tags.join(" "), r.text);
        if let Some(e) = &r.email {
            t = format!("{t} {} {} {} {}", e.from, e.to, e.subject, e.body);
        }
        add(&mut out, Hit::Research(r.id), "Research file", &t);
    }
    for pg in &db.notepad {
        add(&mut out, Hit::Notepad(pg.id), "Notepad", &format!("{} {}", pg.title, pg.text));
    }
    for t in &db.theories {
        let ev: Vec<&str> = t.evidence.iter().map(|e| e.text.as_str()).collect();
        add(&mut out, Hit::Theory(t.id), "Theory", &format!("{} {} {} {}", t.title, t.question, ev.join(" "), t.conclusion));
    }
    for l in &db.letters {
        add(&mut out, Hit::Letter(l.id), "Letter", &format!("{} {} {} {} {}", l.date, l.who, l.addr, l.subject, l.notes));
    }
    for j in &db.journal {
        add(&mut out, Hit::Journal, "Research journal", &format!("{} {} {} {}", j.date, j.goal, j.place, j.notes));
    }
    out
}

impl App {
    pub(crate) fn search_all_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.sall_open {
            return;
        }
        let tr = crate::i18n::tr;
        // search again only when the words or the tree change
        let key = (self.x.sall_q.trim().to_lowercase(), self.x.data_ver, self.undo.len());
        if self.x.sall_key.as_ref() != Some(&key) {
            self.x.sall_found = search_everything(&self.db, &self.x.sall_q);
            self.x.sall_key = Some(key);
        }
        let mut open = true;
        let mut go: Option<Hit> = None;
        egui::Window::new(crate::look::wtitle(tr("Search everything"))).id(egui::Id::new("search_all")).open(&mut open).default_size([640.0, 560.0]).show(ctx, |ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.x.sall_q).hint_text(tr("Words to find anywhere: names, places, notes, sources, stories, emails...")).desired_width(f32::INFINITY));
            if self.x.sall_focus {
                r.request_focus();
                self.x.sall_focus = false;
            }
            let found = &self.x.sall_found;
            if self.x.sall_q.trim().chars().count() < 2 {
                ui.label(RichText::new(tr("Type at least two letters. Every word must be found; their order doesn't matter.")).small().weak());
                return;
            }
            ui.label(RichText::new(crate::i18n::trf("{} found", &["", " found"], &[found.len().to_string()])).small().weak());
            ui.separator();
            // grouped: each person (and their families), then everything else
            let mut groups: Vec<(String, Hit, Vec<&Found>)> = Vec::new();
            let mut by_person: BTreeMap<u32, Vec<&Found>> = BTreeMap::new();
            let mut fams: BTreeMap<u32, Vec<&Found>> = BTreeMap::new();
            let mut other: Vec<&Found> = Vec::new();
            for f in found {
                match &f.at {
                    Hit::Person(p) => by_person.entry(*p).or_default().push(f),
                    Hit::Family(x) => fams.entry(*x).or_default().push(f),
                    Hit::Research(rid) => match self.db.research.iter().find(|r| r.id == *rid).and_then(|r| r.people.first().copied()) {
                        Some(p) => by_person.entry(p).or_default().push(f),
                        None => other.push(f),
                    },
                    Hit::Theory(tid) => match self.db.theories.iter().find(|r| r.id == *tid).and_then(|r| r.people.first().copied()) {
                        Some(p) => by_person.entry(p).or_default().push(f),
                        None => other.push(f),
                    },
                    Hit::Letter(lid) => match self.db.letters.iter().find(|r| r.id == *lid).and_then(|r| r.people.first().copied()) {
                        Some(p) => by_person.entry(p).or_default().push(f),
                        None => other.push(f),
                    },
                    _ => other.push(f),
                }
            }
            let mut ids: Vec<u32> = by_person.keys().copied().collect();
            ids.sort_by_key(|i| self.db.people.get(i).map(|p| p.name.to_lowercase()).unwrap_or_default());
            for i in ids {
                groups.push((self.db.label(i), Hit::Person(i), by_person.remove(&i).unwrap_or_default()));
            }
            for (fid, v) in fams {
                groups.push((crate::research::fam_label(&self.db, fid), Hit::Family(fid), v));
            }
            if !other.is_empty() {
                groups.push((tr("Sources, research files, theories, letters, notepad and journal").to_string(), Hit::Journal, other));
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for (gi, (title, at, items)) in groups.iter().enumerate() {
                    ui.push_id(gi, |ui| {
                        let head = ui.add(egui::Label::new(RichText::new(title).strong().size(15.0)).sense(egui::Sense::click()));
                        if matches!(at, Hit::Person(_) | Hit::Family(_)) && head.on_hover_text(tr("Open")).clicked() {
                            go = Some(at.clone());
                        }
                        for (k, f) in items.iter().enumerate() {
                            ui.push_id(k, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(14.0);
                                    fixed_label(ui, 110.0, RichText::new(part_label(&f.part)).small().color(ui.visuals().hyperlink_color));
                                    if ui.add(egui::Label::new(f.text.as_str()).truncate().sense(egui::Sense::click())).on_hover_text(tr("Go there")).clicked() {
                                        go = Some(f.at.clone());
                                    }
                                });
                            });
                        }
                        ui.add_space(6.0);
                    });
                }
            });
        });
        if !open {
            self.x.sall_open = false;
        }
        match go {
            Some(Hit::Person(p)) => *act = Some(Action::Open(p)),
            Some(Hit::Family(f)) => {
                if let Some(p) = self.db.fams.get(&f).and_then(|x| x.h.or(x.w)) {
                    self.sel = Some(p);
                    self.view = View::Family;
                }
            }
            Some(Hit::Source(s)) => {
                self.src_open = true;
                self.src_sel = Some(s);
            }
            Some(Hit::Theory(t)) => {
                self.x.theory_sel = Some(t);
                self.x.research_tab = 1;
                self.view = View::Research;
            }
            Some(Hit::Letter(l)) => {
                self.x.letter_sel = Some(l);
                self.x.research_tab = 2;
                self.view = View::Research;
            }
            Some(Hit::Research(r)) => {
                self.x.research_tab = 0;
                self.x.research_sel = Some(r);
                self.x.research_kind = None;
                self.x.research_tag.clear();
                self.x.research_q.clear();
                self.x.research_only_sel = false;
                self.view = View::Research;
            }
            Some(Hit::Notepad(n)) => {
                self.x.notepad_page = n;
                self.x.notepad_open = true;
            }
            Some(Hit::Journal) => self.x.journal_open = true,
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_searched_and_grouped() {
        let mut db = crate::sample::sample_family();
        let p = *db.people.keys().next().unwrap();
        db.people.get_mut(&p).unwrap().notes = "Worked at the Buhl creamery for forty years".into();
        db.research.push(crate::research::ResearchItem { id: 1, title: "Letter about the creamery".into(), ..Default::default() });
        let id = db.notepad_new_page();
        db.notepad[0].text = "Ask Ruth about the Buhl CREAMERY photos".into();
        let f = search_everything(&db, "buhl creamery");
        assert!(f.iter().any(|x| x.at == Hit::Person(p) && x.part == "Notes"));
        assert!(f.iter().any(|x| x.at == Hit::Notepad(id)));
        assert!(!f.iter().any(|x| x.at == Hit::Research(1)), "every word must be there");
        assert_eq!(search_everything(&db, "creamery").iter().filter(|x| x.at == Hit::Research(1)).count(), 1);
        assert!(search_everything(&db, "b").is_empty());
        assert_eq!(snippet("a b c needle d e", &["needle".to_string()]), "a b c needle d e");
        // the window draws
        let mut app = App::for_test(db);
        app.x.sall_open = true;
        app.x.sall_q = "creamery".into();
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
        let _ = ctx.run(input, |ctx| {
            let mut act = None;
            app.search_all_window(ctx, &mut act);
        });
        assert!(app.x.sall_found.len() >= 3);
    }
}
