//! Groups: named sets of people, saved with the tree (PAF calls this Focus/Filter). A group is
//! built by relationship (ancestors, descendants...) and by what is recorded (born in Idaho,
//! no sources...), and can then be shown in the People tab, printed, exported or split off as
//! a tree of its own.

use super::*;

/// Who to take, counting out from one person.
pub(crate) const REL_KINDS: &[&str] = &["Ancestors", "Descendants", "Ancestors and descendants", "Everyone connected", "Just this person"];

/// What a field rule looks at, and the kind of test it takes.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FieldKind {
    Text,
    Year,
    Flag,
}

pub(crate) const FIELDS: &[(&str, FieldKind)] = &[
    ("Name", FieldKind::Text),
    ("Surname", FieldKind::Text),
    ("Given names", FieldKind::Text),
    ("Sex", FieldKind::Text),
    ("Born (year)", FieldKind::Year),
    ("Birthplace", FieldKind::Text),
    ("Died (year)", FieldKind::Year),
    ("Place of death", FieldKind::Text),
    ("Any place", FieldKind::Text),
    ("Notes", FieldKind::Text),
    ("Living", FieldKind::Flag),
    ("Has no sources", FieldKind::Flag),
    ("Has photos or documents", FieldKind::Flag),
    ("Has no parents", FieldKind::Flag),
    ("Has no spouse", FieldKind::Flag),
    ("Has no children", FieldKind::Flag),
    ("Has open research tasks", FieldKind::Flag),
];

pub(crate) const TEXT_TESTS: &[&str] = &["contains", "is", "starts with", "is empty", "is not empty"];
pub(crate) const YEAR_TESTS: &[&str] = &["before", "in or before", "is", "in or after", "after", "is empty", "is not empty"];

/// Does a test need a value typed in?
pub(crate) fn needs_value(kind: FieldKind, test: usize) -> bool {
    match kind {
        FieldKind::Text => test < 3,
        FieldKind::Year => test < 5,
        FieldKind::Flag => false,
    }
}

/// The people reached from `id` by `kind` (an index into REL_KINDS), up to `gens` generations
/// (0 = all), with the spouses of each and the brothers and sisters of each when asked.
pub(crate) fn related(db: &Db, ix: &Idx, id: u32, kind: usize, gens: u32, spouses: bool, siblings: bool) -> BTreeSet<u32> {
    let mut out: BTreeSet<u32> = BTreeSet::new();
    if !db.people.contains_key(&id) {
        return out;
    }
    let parents = |n: u32| -> Vec<u32> {
        let mut v = vec![];
        for f in ix.child_of.get(&n).into_iter().flatten() {
            if let Some(fm) = db.fams.get(f) {
                v.extend(fm.h.iter().chain(fm.w.iter()).copied());
            }
        }
        v
    };
    let children = |n: u32| -> Vec<u32> {
        let mut v = vec![];
        for f in ix.partner_in.get(&n).into_iter().flatten() {
            if let Some(fm) = db.fams.get(f) {
                v.extend(fm.kids.iter().copied());
            }
        }
        v
    };
    let partners = |n: u32| -> Vec<u32> {
        let mut v = vec![];
        for f in ix.partner_in.get(&n).into_iter().flatten() {
            if let Some(fm) = db.fams.get(f) {
                v.extend(fm.h.iter().chain(fm.w.iter()).chain(fm.more.iter().map(|(_, x)| x)).copied().filter(|x| *x != n));
            }
        }
        v
    };
    let within = |d: u32| gens == 0 || d <= gens;
    let walk = |start: u32, step: &dyn Fn(u32) -> Vec<u32>, out: &mut BTreeSet<u32>| {
        let mut seen: HashMap<u32, u32> = HashMap::new();
        seen.insert(start, 0);
        let mut q = std::collections::VecDeque::from([start]);
        while let Some(n) = q.pop_front() {
            let d = seen[&n];
            out.insert(n);
            if !within(d + 1) {
                continue;
            }
            for m in step(n) {
                if db.people.contains_key(&m) && !seen.contains_key(&m) {
                    seen.insert(m, d + 1);
                    q.push_back(m);
                }
            }
        }
    };
    match kind {
        0 => walk(id, &parents, &mut out),
        1 => walk(id, &children, &mut out),
        2 => {
            walk(id, &parents, &mut out);
            walk(id, &children, &mut out);
        }
        3 => {
            let all = |n: u32| -> Vec<u32> {
                let mut v = parents(n);
                v.extend(children(n));
                v.extend(partners(n));
                v
            };
            walk(id, &all, &mut out);
        }
        _ => {
            out.insert(id);
        }
    }
    let found: Vec<u32> = out.iter().copied().collect();
    if siblings {
        for n in &found {
            for f in ix.child_of.get(n).into_iter().flatten() {
                if let Some(fm) = db.fams.get(f) {
                    out.extend(fm.kids.iter().copied().filter(|k| db.people.contains_key(k)));
                }
            }
        }
    }
    if spouses {
        let now: Vec<u32> = out.iter().copied().collect();
        for n in now {
            out.extend(partners(n).into_iter().filter(|k| db.people.contains_key(k)));
        }
    }
    out
}

fn year_of(s: &str) -> Option<i32> {
    parse_date(s).0
}

/// Does person `id` pass the rule: field (index into FIELDS), test (index into TEXT_TESTS or
/// YEAR_TESTS) and value?
pub(crate) fn matches(db: &Db, ix: &Idx, id: u32, field: usize, test: usize, value: &str) -> bool {
    let Some(p) = db.people.get(&id) else { return false };
    let Some((_, kind)) = FIELDS.get(field) else { return false };
    let text = |s: &str| -> bool {
        let s = s.trim().to_lowercase();
        let v = value.trim().to_lowercase();
        match test {
            0 => !v.is_empty() && s.contains(&v),
            1 => s == v,
            2 => !v.is_empty() && s.starts_with(&v),
            3 => s.is_empty(),
            _ => !s.is_empty(),
        }
    };
    let year = |s: &str| -> bool {
        let y = year_of(s);
        let v: Option<i32> = value.trim().parse().ok();
        match (test, y, v) {
            (5, None, _) => true,
            (5, Some(_), _) => false,
            (6, y, _) => y.is_some(),
            (_, Some(y), Some(v)) => match test {
                0 => y < v,
                1 => y <= v,
                2 => y == v,
                3 => y >= v,
                _ => y > v,
            },
            _ => false,
        }
    };
    match (field, kind) {
        (0, _) => {
            // any of the person's names
            let names = format!("{}\n{}\n{}", p.name, p.maiden, p.nick);
            if test >= 3 {
                text(&p.name)
            } else {
                names.lines().any(|n| text(n))
            }
        }
        (1, _) => text(&p.surname),
        (2, _) => text(&p.given),
        (3, _) => {
            let s = match p.sex.as_str() {
                "M" => "male",
                "F" => "female",
                _ => "",
            };
            // "is M" and "is male" both work
            text(s) || (test == 1 && p.sex.eq_ignore_ascii_case(value.trim()))
        }
        (4, _) => year(if p.bd.trim().is_empty() { &p.chd } else { &p.bd }),
        (5, _) => text(&p.bp),
        (6, _) => year(if p.dd.trim().is_empty() { &p.bud } else { &p.dd }),
        (7, _) => text(&p.dp),
        (8, _) => {
            let mut places: Vec<&str> = vec![&p.bp, &p.chp, &p.dp, &p.bup];
            places.extend(p.events.iter().map(|e| e.place.as_str()));
            places.extend(p.fx.values().map(|e| e.place.as_str()));
            for f in ix.partner_in.get(&id).into_iter().flatten() {
                if let Some(fm) = db.fams.get(f) {
                    places.push(&fm.mp);
                    places.extend(fm.events.iter().map(|e| e.place.as_str()));
                }
            }
            if test >= 3 {
                let any = places.iter().any(|s| !s.trim().is_empty());
                if test == 3 { !any } else { any }
            } else {
                places.iter().any(|s| !s.trim().is_empty() && text(s))
            }
        }
        (9, _) => text(&p.notes),
        (10, _) => p.living,
        (11, _) => p.sources.is_empty() && p.events.iter().all(|e| e.cites.is_empty()) && p.fx.values().all(|e| e.cites.is_empty()),
        (12, _) => !p.media.is_empty(),
        (13, _) => ix.child_of.get(&id).map_or(true, |v| v.iter().all(|f| db.fams.get(f).map_or(true, |fm| fm.h.is_none() && fm.w.is_none()))),
        (14, _) => ix.partner_in.get(&id).map_or(true, |v| v.iter().all(|f| db.fams.get(f).map_or(true, |fm| fm.h.iter().chain(fm.w.iter()).all(|x| *x == id)))),
        (15, _) => ix.partner_in.get(&id).map_or(true, |v| v.iter().all(|f| db.fams.get(f).map_or(true, |fm| fm.kids.is_empty()))),
        (16, _) => p.todos.iter().any(|t| !t.done),
        _ => false,
    }
}

/// Everyone who passes a field rule.
pub(crate) fn matching(db: &Db, field: usize, test: usize, value: &str) -> BTreeSet<u32> {
    let ix = db.index();
    db.people.keys().copied().filter(|id| matches(db, &ix, *id, field, test, value)).collect()
}

/// A plain description of a field rule, e.g. "Birthplace contains Idaho".
pub(crate) fn rule_text(field: usize, test: usize, value: &str) -> String {
    let Some((name, kind)) = FIELDS.get(field) else { return String::new() };
    match kind {
        FieldKind::Flag => name.to_string(),
        FieldKind::Text => format!("{} {}{}", name, TEXT_TESTS.get(test).unwrap_or(&""), if needs_value(*kind, test) { format!(" \"{}\"", value.trim()) } else { String::new() }),
        FieldKind::Year => format!("{} {}{}", name, YEAR_TESTS.get(test).unwrap_or(&""), if needs_value(*kind, test) { format!(" {}", value.trim()) } else { String::new() }),
    }
}

/// A copy of the tree with only the people in `keep`: families keep only those members (and go
/// when nobody is left), and every link to someone left out is taken away, so the copy can be
/// exported or opened as a tree of its own.
pub(crate) fn keep_only(db: &Db, keep: &BTreeSet<u32>) -> Db {
    let mut d = db.clone();
    d.people.retain(|k, _| keep.contains(k));
    let gone_fams: Vec<u32> = d
        .fams
        .iter_mut()
        .filter_map(|(fid, f)| {
            f.h = f.h.filter(|x| keep.contains(x));
            f.w = f.w.filter(|x| keep.contains(x));
            f.kids.retain(|x| keep.contains(x));
            f.more.retain(|(_, x)| keep.contains(x));
            f.pedi.retain(|k, _| keep.contains(k));
            f.frel.retain(|k, _| keep.contains(k));
            f.mrel.retain(|k, _| keep.contains(k));
            let members = f.h.iter().count() + f.w.iter().count() + f.kids.len() + f.more.len();
            // A family left with only one person in it links nobody, so it goes too.
            let empty = members <= 1;
            if empty { Some(*fid) } else { None }
        })
        .collect();
    for f in &gone_fams {
        d.fams.remove(f);
    }
    for p in d.people.values_mut() {
        p.assoc.retain(|a| keep.contains(&a.other));
        p.pref_parents = p.pref_parents.filter(|f| d.fams.contains_key(f));
        p.fam_order.retain(|f| d.fams.contains_key(f));
        for m in p.media.iter_mut() {
            m.faces.retain(|fc| keep.contains(&fc.who));
        }
        if p.face_portrait.as_ref().map_or(false, |fp| !keep.contains(&fp.owner)) {
            p.face_portrait = None;
        }
    }
    // Links kept as GEDCOM text (from imported files) to anyone left out.
    let kept_fams: HashSet<u32> = d.fams.keys().copied().collect();
    let gone_ptr = |t: &str| -> bool {
        let mut rest = t;
        while let Some(i) = rest.find('@') {
            let after = &rest[i + 1..];
            let Some(j) = after.find('@') else { break };
            let x = &after[..j];
            if let Some(n) = x.strip_prefix('I').and_then(|n| n.parse::<u32>().ok()) {
                if !keep.contains(&n) {
                    return true;
                }
            }
            if let Some(n) = x.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
                if !kept_fams.contains(&n) && db.fams.contains_key(&n) {
                    return true;
                }
            }
            rest = &after[j + 1..];
        }
        false
    };
    for p in d.people.values_mut() {
        p.extra.retain(|r| !gone_ptr(&r.text));
        for e in p.events.iter_mut() {
            e.extra.retain(|r| !gone_ptr(&r.text));
        }
    }
    for f in d.fams.values_mut() {
        f.extra.retain(|r| !gone_ptr(&r.text));
    }
    d.records.retain(|r| !gone_ptr(r));
    d.bookmarks.retain(|b| keep.contains(b));
    for j in d.journal.iter_mut() {
        j.people.retain(|x| keep.contains(x));
    }
    d.dna.retain(|t| t.person.map_or(true, |p| keep.contains(&p)));
    for t in d.dna.iter_mut() {
        for m in t.matches.iter_mut() {
            m.link = m.link.filter(|x| keep.contains(x));
        }
    }
    d.home = d.home.filter(|h| keep.contains(h));
    for g in d.groups.iter_mut() {
        g.ids.retain(|x| keep.contains(x));
    }
    d
}

impl App {
    /// The open group, if any.
    fn cur_group(&self) -> Option<usize> {
        let i = self.x.group_cur;
        (i < self.db.groups.len()).then_some(i)
    }

    pub(crate) fn groups_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.groups_open {
            return;
        }
        let mut open = true;
        let mut change: Option<Box<dyn FnOnce(&mut App)>> = None;
        egui::Window::new("Groups").open(&mut open).default_size([760.0, 600.0]).show(ctx, |ui| {
            ui.label(
                RichText::new("A group is a set of people you pick out, by relationship or by what is recorded about them. Groups are kept with the tree. Use one to show those people in the People tab, print a list, export a GEDCOM file of just them, or start a new tree from them.")
                    .small()
                    .weak(),
            );
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Group");
                let names: Vec<String> = self.db.groups.iter().map(|g| format!("{} ({})", g.name, g.ids.iter().filter(|i| self.db.people.contains_key(i)).count())).collect();
                let cur = self.cur_group();
                egui::ComboBox::from_id_salt("group_pick")
                    .width(240.0)
                    .selected_text(cur.and_then(|i| names.get(i).cloned()).unwrap_or_else(|| "(no groups yet)".into()))
                    .show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut self.x.group_cur, i, n);
                        }
                    });
                ui.add(egui::TextEdit::singleline(&mut self.x.group_name).hint_text("name for a new group").desired_width(170.0));
                let name = self.x.group_name.trim().to_string();
                if ui.add_enabled(!name.is_empty(), egui::Button::new("New group")).clicked() {
                    change = Some(Box::new(move |a: &mut App| {
                        a.db.groups.push(Group { name, ..Default::default() });
                        a.x.group_cur = a.db.groups.len() - 1;
                        a.x.group_name.clear();
                    }));
                }
                if let Some(i) = cur {
                    ui.menu_button("More", |ui| {
                        let name = self.x.group_name.trim().to_string();
                        if ui.add_enabled(!name.is_empty(), egui::Button::new("Rename to the name typed")).clicked() {
                            change = Some(Box::new(move |a: &mut App| {
                                a.db.groups[i].name = name;
                                a.x.group_name.clear();
                            }));
                            ui.close_menu();
                        }
                        if ui.button("Copy this group").clicked() {
                            change = Some(Box::new(move |a: &mut App| {
                                let mut g = a.db.groups[i].clone();
                                g.name = format!("{} (copy)", g.name);
                                a.db.groups.push(g);
                                a.x.group_cur = a.db.groups.len() - 1;
                            }));
                            ui.close_menu();
                        }
                        if ui.button("Delete this group").on_hover_text("Only the group goes; nobody is deleted from the tree. Undo brings it back.").clicked() {
                            change = Some(Box::new(move |a: &mut App| {
                                a.db.groups.remove(i);
                                a.x.group_cur = 0;
                                if a.x.table_only == crate::table::Only::Group {
                                    a.x.table_only = crate::table::Only::All;
                                }
                            }));
                            ui.close_menu();
                        }
                    });
                }
            });
            let Some(gi) = self.cur_group() else {
                ui.add_space(8.0);
                ui.label("Type a name, such as \"Harwood line\" or \"Born in Idaho\", and press New group.");
                return;
            };
            let members: Vec<u32> = self.db.groups[gi].ids.iter().copied().filter(|i| self.db.people.contains_key(i)).collect();
            ui.separator();
            ui.columns(2, |cols| {
                let ui = &mut cols[0];
                // ---- by relationship
                ui.label(RichText::new("By relationship").strong());
                let start = self.x.group_who.filter(|i| self.db.people.contains_key(i)).or(self.sel);
                ui.horizontal(|ui| {
                    ui.label("Start from");
                    ui.label(RichText::new(start.map(|i| self.db.label(i)).unwrap_or_else(|| "(nobody chosen)".into())).strong());
                });
                ui.horizontal(|ui| {
                    if ui.small_button("Use the person selected now").clicked() {
                        self.x.group_who = self.sel;
                    }
                    if let Some(h) = self.db.home {
                        if ui.small_button("Use the home person").clicked() {
                            self.x.group_who = Some(h);
                        }
                    }
                });
                egui::ComboBox::from_id_salt("group_rel").width(220.0).selected_text(REL_KINDS[self.x.group_rel.min(REL_KINDS.len() - 1)]).show_ui(ui, |ui| {
                    for (i, l) in REL_KINDS.iter().enumerate() {
                        ui.selectable_value(&mut self.x.group_rel, i, *l);
                    }
                });
                if self.x.group_rel <= 2 {
                    ui.horizontal(|ui| {
                        ui.label("Generations");
                        ui.add(egui::DragValue::new(&mut self.x.group_gens).range(0..=99));
                        ui.label(RichText::new(if self.x.group_gens == 0 { "(all)" } else { "" }).weak());
                    });
                }
                ui.checkbox(&mut self.x.group_sibs, "With their brothers and sisters");
                ui.checkbox(&mut self.x.group_spouses, "With their spouses");
                let found = start.map(|s| related(&self.db, &self.db.index(), s, self.x.group_rel, self.x.group_gens, self.x.group_spouses, self.x.group_sibs)).unwrap_or_default();
                ui.label(RichText::new(format!("{} people", found.len())).weak());
                ui.horizontal(|ui| {
                    let what = start.map(|s| format!("{} of {}", REL_KINDS[self.x.group_rel.min(REL_KINDS.len() - 1)].to_lowercase(), self.db.label(s))).unwrap_or_default();
                    let f1 = found.clone();
                    let w1 = what.clone();
                    if ui.add_enabled(!found.is_empty(), egui::Button::new("Add to group")).clicked() {
                        change = Some(Box::new(move |a: &mut App| {
                            let g = &mut a.db.groups[gi];
                            g.ids.extend(f1);
                            g.how.push(format!("Added {}", w1));
                        }));
                    }
                    let f2 = found.clone();
                    if ui.add_enabled(!found.is_empty(), egui::Button::new("Take out of group")).clicked() {
                        change = Some(Box::new(move |a: &mut App| {
                            let g = &mut a.db.groups[gi];
                            g.ids.retain(|x| !f2.contains(x));
                            g.how.push(format!("Took out {}", what));
                        }));
                    }
                });
                ui.add_space(10.0);
                // ---- by what is recorded
                ui.label(RichText::new("By what is recorded").strong());
                let fi = self.x.group_field.min(FIELDS.len() - 1);
                let (fname, kind) = FIELDS[fi];
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("group_field").width(170.0).selected_text(fname).show_ui(ui, |ui| {
                        for (i, (l, _)) in FIELDS.iter().enumerate() {
                            ui.selectable_value(&mut self.x.group_field, i, *l);
                        }
                    });
                    let tests: &[&str] = match kind {
                        FieldKind::Text => TEXT_TESTS,
                        FieldKind::Year => YEAR_TESTS,
                        FieldKind::Flag => &[],
                    };
                    if !tests.is_empty() {
                        let t = self.x.group_test.min(tests.len() - 1);
                        egui::ComboBox::from_id_salt("group_test").width(110.0).selected_text(tests[t]).show_ui(ui, |ui| {
                            for (i, l) in tests.iter().enumerate() {
                                ui.selectable_value(&mut self.x.group_test, i, *l);
                            }
                        });
                    }
                });
                let test = match kind {
                    FieldKind::Text => self.x.group_test.min(TEXT_TESTS.len() - 1),
                    FieldKind::Year => self.x.group_test.min(YEAR_TESTS.len() - 1),
                    FieldKind::Flag => 0,
                };
                if needs_value(kind, test) {
                    ui.add(egui::TextEdit::singleline(&mut self.x.group_val).hint_text(if kind == FieldKind::Year { "a year, e.g. 1900" } else { "e.g. Idaho" }).desired_width(220.0));
                }
                let ready = !needs_value(kind, test) || !self.x.group_val.trim().is_empty();
                let hits = if ready { matching(&self.db, fi, test, &self.x.group_val) } else { BTreeSet::new() };
                let in_group = hits.iter().filter(|x| self.db.groups[gi].ids.contains(x)).count();
                ui.label(RichText::new(format!("{} people match ({} of them already in the group)", hits.len(), in_group)).weak());
                let rule = rule_text(fi, test, &self.x.group_val);
                ui.horizontal(|ui| {
                    let (h1, r1) = (hits.clone(), rule.clone());
                    if ui.add_enabled(ready && !hits.is_empty(), egui::Button::new("Add matches")).clicked() {
                        change = Some(Box::new(move |a: &mut App| {
                            let g = &mut a.db.groups[gi];
                            g.ids.extend(h1);
                            g.how.push(format!("Added everyone: {}", r1));
                        }));
                    }
                    let (h2, r2) = (hits.clone(), rule.clone());
                    if ui.add_enabled(ready, egui::Button::new("Keep only matches")).on_hover_text("Narrow the group down to the people in it who match").clicked() {
                        change = Some(Box::new(move |a: &mut App| {
                            let g = &mut a.db.groups[gi];
                            g.ids.retain(|x| h2.contains(x));
                            g.how.push(format!("Kept only: {}", r2));
                        }));
                    }
                    let (h3, r3) = (hits, rule);
                    if ui.add_enabled(ready, egui::Button::new("Take out matches")).clicked() {
                        change = Some(Box::new(move |a: &mut App| {
                            let g = &mut a.db.groups[gi];
                            g.ids.retain(|x| !h3.contains(x));
                            g.how.push(format!("Took out: {}", r3));
                        }));
                    }
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Add everyone").clicked() {
                        change = Some(Box::new(move |a: &mut App| {
                            let all: Vec<u32> = a.db.people.keys().copied().collect();
                            let g = &mut a.db.groups[gi];
                            g.ids.extend(all);
                            g.how.push("Added everyone".into());
                        }));
                    }
                    if ui.add_enabled(!members.is_empty(), egui::Button::new("Empty the group")).clicked() {
                        change = Some(Box::new(move |a: &mut App| {
                            let g = &mut a.db.groups[gi];
                            g.ids.clear();
                            g.how.clear();
                        }));
                    }
                });
                if !self.db.groups[gi].how.is_empty() {
                    ui.add_space(6.0);
                    egui::CollapsingHeader::new(RichText::new("How this group was made").small()).id_salt("group_how").show(ui, |ui| {
                        for h in &self.db.groups[gi].how {
                            ui.label(RichText::new(h).small().weak());
                        }
                    });
                }

                // ---- the people in it, and what to do with them
                let ui = &mut cols[1];
                ui.label(RichText::new(format!("In this group: {} people", members.len())).strong());
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(!members.is_empty(), egui::Button::new("Show in People tab")).on_hover_text("List them in the People tab, where they can be sorted, edited and saved as a spreadsheet").clicked() {
                        self.x.table_only = crate::table::Only::Group;
                        self.x.table_group = gi;
                        self.view = View::Table;
                    }
                    if ui.add_enabled(!members.is_empty(), egui::Button::new("Print list")).clicked() {
                        self.x.group_print = Some(gi);
                    }
                    if ui.add_enabled(!members.is_empty(), egui::Button::new("Export GEDCOM...")).on_hover_text("A GEDCOM file with only these people, with their photos and sources").clicked() {
                        self.x.group_export = Some(gi);
                    }
                    if ui.add_enabled(!members.is_empty() && self.x.practice.is_none(), egui::Button::new("Start a new tree from them...")).on_hover_text("Copy these people into a new tree of their own. This tree is not changed.").clicked() {
                        self.x.group_split = Some((gi, self.db.groups[gi].name.clone()));
                    }
                });
                ui.separator();
                let mut sorted: Vec<(String, u32)> = members.iter().map(|i| (self.db.people[i].name.to_lowercase(), *i)).collect();
                sorted.sort();
                egui::ScrollArea::vertical().id_salt("group_members").auto_shrink([false, false]).show(ui, |ui| {
                    for (_, id) in sorted.iter().take(2000) {
                        ui.horizontal(|ui| {
                            if ui.small_button("x").on_hover_text("Take out of the group").clicked() {
                                let id = *id;
                                change = Some(Box::new(move |a: &mut App| {
                                    a.db.groups[gi].ids.remove(&id);
                                }));
                            }
                            if ui.link(self.db.label(*id)).clicked() {
                                *act = Some(Action::Select(*id));
                            }
                        });
                    }
                    if sorted.len() > 2000 {
                        ui.weak(format!("...and {} more (see them all with Show in People tab)", sorted.len() - 2000));
                    }
                });
            });
        });
        if let Some(c) = change {
            c(self);
            self.x.next_label = Some("Changed a group".into());
            self.dirty = true;
        }
        self.x.groups_open = open;
        self.group_jobs(ctx);
    }

    /// Printing, exporting and splitting, started from the window (they open file dialogs).
    fn group_jobs(&mut self, ctx: &egui::Context) {
        if let Some(gi) = self.x.group_print.take() {
            if let Some(g) = self.db.groups.get(gi) {
                let html = self.group_list_html(g);
                self.open_print(html);
            }
        }
        if let Some(gi) = self.x.group_export.take() {
            if let Some(g) = self.db.groups.get(gi) {
                let part = keep_only(&self.db, &g.ids);
                let name = format!("{}.ged", sanitize(&g.name));
                if let Some(path) = rfd::FileDialog::new().set_title("Save the group as a GEDCOM file").set_directory(start_dir()).set_file_name(&name).save_file() {
                    remember_dir(&path);
                    self.status = match write_gedcom(&part, &path, false) {
                        Ok(s) | Err(s) => s,
                    };
                }
            }
        }
        if let Some((gi, name)) = self.x.group_split.clone() {
            let mut open = true;
            let mut go = false;
            let tree = sanitize_tree_name(&name);
            let exists = !tree.is_empty() && base_dir().join("More trees").join(&tree).exists();
            egui::Window::new("Start a new tree from a group").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
                let n = self.db.groups.get(gi).map_or(0, |g| g.ids.iter().filter(|i| self.db.people.contains_key(i)).count());
                ui.label(format!("Copy the {} people in this group, with their families, sources, notes and photos, into a new tree.", n));
                ui.label(RichText::new("This tree stays as it is. The new tree opens when it is made; switch back any time with File > Trees.").small().weak());
                ui.horizontal(|ui| {
                    ui.label("Name for the new tree");
                    let mut s = name.clone();
                    if ui.text_edit_singleline(&mut s).changed() {
                        self.x.group_split = Some((gi, s));
                    }
                });
                if exists {
                    ui.colored_label(egui::Color32::from_rgb(200, 120, 40), "A tree with that name already exists. Choose another name.");
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(!tree.is_empty() && !exists, egui::Button::new("Make the new tree")).clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.x.group_split = None;
                    }
                });
            });
            if !open {
                self.x.group_split = None;
            }
            if go {
                self.x.group_split = None;
                if let Some(g) = self.db.groups.get(gi).cloned() {
                    match split_off(&self.db, &g, &tree) {
                        Ok(n) => {
                            self.switch_tree(&tree);
                            self.x.groups_open = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("Gilded Roots - {}", tree_title(&tree))));
                            self.status = format!("Made the tree \"{}\" with {} people, and opened it.", tree, n);
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
        }
    }

    fn group_list_html(&self, g: &Group) -> String {
        let mut rows: Vec<(String, u32)> = g.ids.iter().filter_map(|i| self.db.people.get(i).map(|p| (format!("{} {}", p.surname.to_lowercase(), p.given.to_lowercase()), *i))).collect();
        rows.sort();
        let mut b = String::new();
        b.push_str(&format!("<h1>{}</h1><p>{} people &middot; {}</p>", esc(&g.name), rows.len(), esc(&print_date(&today()))));
        if !g.how.is_empty() {
            b.push_str("<p class=\"small\">");
            b.push_str(&g.how.iter().map(|h| esc(h)).collect::<Vec<_>>().join("; "));
            b.push_str("</p>");
        }
        b.push_str("<table><tr><th>ID</th><th>Name</th><th>Born</th><th>Died</th><th>Parents</th></tr>");
        for (_, id) in rows {
            let p = &self.db.people[&id];
            let (f, m) = self.db.parents(id);
            let par: Vec<String> = [f, m].into_iter().flatten().filter_map(|x| self.db.people.get(&x)).map(|x| esc(&x.name)).collect();
            b.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                id,
                esc(&p.name),
                esc(&dp(&p.bd, &p.bp)),
                esc(&dp(&p.dd, &p.dp)),
                par.join(" &amp; ")
            ));
        }
        b.push_str("</table>");
        page(&g.name, &b, false)
    }
}

/// Write a new tree in "More trees" holding only the group, with the photo and document files
/// it uses copied across. Returns how many people went in.
pub(crate) fn split_off(db: &Db, g: &Group, tree: &str) -> Result<usize, String> {
    let dir = base_dir().join("More trees").join(tree);
    if dir.exists() {
        return Err("A tree with that name already exists.".into());
    }
    let mut part = keep_only(db, &g.ids);
    part.groups.clear();
    part.sample = false;
    let n = part.people.len();
    std::fs::create_dir_all(dir.join("media")).map_err(|e| format!("Could not make the new tree: {}", io_msg(&e)))?;
    // copy the media files the new tree uses
    let json = serde_json::to_string(&part).unwrap_or_default();
    if let Ok(rd) = std::fs::read_dir(media_dir()) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_file() && json.contains(&format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))) {
                let _ = std::fs::copy(e.path(), dir.join("media").join(&name));
            }
        }
    }
    save_db_to(&dir, &part).map_err(|e| format!("Could not save the new tree: {}", e))?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ann + Bob -> Cal (b. 1901, Boise, Idaho) who married Dee; Cal + Dee -> Eve (living).
    // Fay is unrelated. Bob has an ASSO link to Fay kept as GEDCOM text.
    const T: &str = "0 HEAD\n0 @I1@ INDI\n1 NAME Ann /Lee/\n1 SEX F\n1 FAMS @F1@\n0 @I2@ INDI\n1 NAME Bob /Lee/\n1 SEX M\n1 FAMS @F1@\n1 _CUSTOM @I6@\n1 SOUR @S1@\n0 @I3@ INDI\n1 NAME Cal /Lee/\n1 SEX M\n1 BIRT\n2 DATE 1901\n2 PLAC Boise, Ada, Idaho, USA\n1 FAMC @F1@\n1 FAMS @F2@\n0 @I4@ INDI\n1 NAME Dee /Fox/\n1 SEX F\n1 FAMS @F2@\n0 @I5@ INDI\n1 NAME Eve /Lee/\n1 SEX F\n1 FAMC @F2@\n0 @I6@ INDI\n1 NAME Fay /Roe/\n1 BIRT\n2 DATE 1950\n2 PLAC Paris, France\n0 @F1@ FAM\n1 HUSB @I2@\n1 WIFE @I1@\n1 CHIL @I3@\n0 @F2@ FAM\n1 HUSB @I3@\n1 WIFE @I4@\n1 MARR\n2 PLAC Twin Falls, Idaho\n1 CHIL @I5@\n0 @S1@ SOUR\n1 TITL Lee family Bible\n0 TRLR\n";

    fn db() -> Db {
        gedcom::parse_ged(T)
    }
    fn id(db: &Db, given: &str) -> u32 {
        *db.people.iter().find(|(_, p)| p.given == given).unwrap().0
    }
    fn names(db: &Db, s: &BTreeSet<u32>) -> Vec<String> {
        let mut v: Vec<String> = s.iter().map(|i| db.people[i].given.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn groups_by_relationship() {
        let db = db();
        let ix = db.index();
        let eve = id(&db, "Eve");
        let ann = id(&db, "Ann");
        assert_eq!(names(&db, &related(&db, &ix, eve, 0, 0, false, false)), ["Ann", "Bob", "Cal", "Dee", "Eve"]);
        assert_eq!(names(&db, &related(&db, &ix, eve, 0, 1, false, false)), ["Cal", "Dee", "Eve"], "one generation");
        assert_eq!(names(&db, &related(&db, &ix, ann, 1, 0, false, false)), ["Ann", "Cal", "Eve"]);
        assert_eq!(names(&db, &related(&db, &ix, ann, 1, 0, true, false)), ["Ann", "Bob", "Cal", "Dee", "Eve"], "with spouses");
        assert_eq!(names(&db, &related(&db, &ix, ann, 3, 0, false, false)), ["Ann", "Bob", "Cal", "Dee", "Eve"], "everyone connected");
        assert_eq!(names(&db, &related(&db, &ix, ann, 4, 0, false, false)), ["Ann"]);
        assert!(related(&db, &ix, 999, 0, 0, true, true).is_empty());
    }

    #[test]
    fn groups_by_what_is_recorded() {
        let db = db();
        let ix = db.index();
        let m = |f: usize, t: usize, v: &str| names(&db, &matching(&db, f, t, v));
        assert_eq!(m(5, 0, "idaho"), ["Cal"], "birthplace contains");
        assert_eq!(m(8, 0, "idaho"), ["Cal", "Dee"], "any place, including where they married");
        assert_eq!(m(4, 0, "1925"), ["Cal"], "born before");
        assert_eq!(m(4, 3, "1950"), ["Fay"], "born in or after");
        assert_eq!(m(4, 5, ""), ["Ann", "Bob", "Dee", "Eve"], "no birth year");
        assert_eq!(m(1, 1, "lee"), ["Ann", "Bob", "Cal", "Eve"], "surname is");
        assert_eq!(m(3, 1, "female"), ["Ann", "Dee", "Eve"]);
        assert_eq!(m(3, 1, "F"), ["Ann", "Dee", "Eve"]);
        assert_eq!(m(13, 0, ""), ["Ann", "Bob", "Dee", "Fay"], "no parents");
        assert_eq!(m(14, 0, ""), ["Eve", "Fay"], "no spouse");
        assert_eq!(m(15, 0, ""), ["Eve", "Fay"], "no children");
        assert!(!m(11, 0, "").contains(&"Bob".to_string()), "Bob has a source");
        assert!(m(5, 0, "").is_empty(), "contains nothing matches nobody");
        assert!(matches(&db, &ix, id(&db, "Cal"), 0, 0, "cal lee"));
        assert_eq!(rule_text(5, 0, " Idaho "), "Birthplace contains \"Idaho\"");
        assert_eq!(rule_text(10, 0, ""), "Living");
    }

    #[test]
    fn a_group_cut_out_of_the_tree_has_no_loose_ends() {
        let db = db();
        let ix = db.index();
        // Bob's line upward from Cal, without Dee, Eve or Fay
        let keep = related(&db, &ix, id(&db, "Cal"), 0, 0, false, false);
        let part = keep_only(&db, &keep);
        assert_eq!(names(&part, &part.people.keys().copied().collect()), ["Ann", "Bob", "Cal"]);
        // Cal's marriage to Dee had only Cal left, so it went
        assert_eq!(part.fams.len(), 1);
        let out = gedcom::to_ged(&part);
        for x in ["@I4@", "@I5@", "@I6@", "@F2@"] {
            assert!(!out.contains(x), "{x} left behind:\n{out}");
        }
        assert!(out.contains("Lee family Bible"), "Bob's source goes with him");
        // and it reads back cleanly
        let back = gedcom::parse_ged(&out);
        assert_eq!(back.people.len(), 3);
        assert_eq!(back.fams.len(), 1);
    }

    #[test]
    fn groups_follow_renumbering_and_merging() {
        let mut db = db();
        let (cal, eve) = (id(&db, "Cal"), id(&db, "Eve"));
        db.groups.push(Group { name: "Two".into(), ids: [cal, eve].into_iter().collect(), how: vec![] });
        let order = gedcom::renumber_order(&db, gedcom::RenumberOrder::Alphabetical);
        gedcom::renumber(&mut db, &order);
        assert_eq!(names(&db, &db.groups[0].ids), ["Cal", "Eve"]);
        let (eve, fay) = (id(&db, "Eve"), id(&db, "Fay"));
        db.merge_people(fay, eve);
        assert!(db.groups[0].ids.contains(&fay) && !db.groups[0].ids.contains(&eve));
        // groups are kept with the tree
        let json = serde_json::to_string(&db).unwrap();
        let back: Db = serde_json::from_str(&json).unwrap();
        assert_eq!(back.groups[0].name, "Two");
    }
}
