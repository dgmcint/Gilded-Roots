//! Pop-up windows: renumber, versions, history, to-do, welcome, trees and places.

use super::*;

impl App {
    /// Tools > Saved versions: the daily copies of the tree, what is different in each, and
    /// putting back one person or the whole tree.
    /// Tools > Renumber people: new ID numbers 1, 2, 3... for everyone (and their families).
    pub(crate) fn renumber_window(&mut self, ctx: &egui::Context) {
        if !self.x.renum_open {
            return;
        }
        let how = |n: u8| match n {
            1 => gedcom::RenumberOrder::Present,
            2 => gedcom::RenumberOrder::Alphabetical,
            _ => gedcom::RenumberOrder::Family,
        };
        if self.x.renum_preview.as_ref().map(|p| p.0) != Some(self.x.renum_how) {
            let order = gedcom::renumber_order(&self.db, how(self.x.renum_how));
            let first: Vec<(u32, u32, String)> = order.iter().take(8).enumerate().map(|(i, id)| (*id, i as u32 + 1, self.db.label(*id))).collect();
            let moved = order.iter().enumerate().filter(|(i, id)| **id != *i as u32 + 1).count();
            self.x.renum_preview = Some((self.x.renum_how, first, moved));
        }
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        let home = self.db.home.filter(|h| self.db.people.contains_key(h)).map(|h| self.db.label(h));
        let n = self.db.people.len();
        let nf = self.db.fams.len();
        egui::Window::new("Renumber people").open(&mut open).collapsible(false).resizable(false).default_width(560.0).show(ctx, |ui| {
            ui.label("Give everyone new ID numbers 1, 2, 3... with no gaps. Families get new numbers 1, 2, 3... too.");
            ui.label(RichText::new("Nothing else changes: names, dates, links, sources and photos all stay as they are.").small().weak());
            ui.add_space(6.0);
            ui.label(RichText::new("Order").strong());
            ui.radio_value(&mut self.x.renum_how, 0, "Home person first, then their family (parents and grandparents, then partners and children)");
            ui.radio_value(&mut self.x.renum_how, 1, "Keep the present order, just close the gaps");
            ui.radio_value(&mut self.x.renum_how, 2, "Alphabetical by surname, then given names");
            if self.x.renum_how == 0 {
                match &home {
                    Some(h) => ui.label(RichText::new(format!("{} (the home person) becomes number 1.", h)).small()),
                    None => ui.label(RichText::new("No home person is set, so the first person in the tree becomes number 1. (Tools > Home person... chooses one.)").small()),
                };
            }
            ui.add_space(6.0);
            if let Some((_, first, moved)) = &self.x.renum_preview {
                ui.label(RichText::new("First few, after renumbering").strong());
                egui::Grid::new("renum_preview").striped(true).num_columns(3).show(ui, |ui| {
                    for (old, new, name) in first {
                        ui.label(format!("#{}", new));
                        ui.label(name.as_str());
                        ui.label(RichText::new(format!("was #{}", old)).small().weak());
                        ui.end_row();
                    }
                });
                ui.add_space(4.0);
                ui.label(format!("{} of {} people get a different number; {} families are renumbered.", moved, n, nf));
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Each person keeps their old number as an \"Old ID\" (shown on their Individual page and kept in exported GEDCOM files), so older printouts and notes can still be matched. Undo (Ctrl+Z) puts the old numbers back.").small().weak());
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Renumber").strong()).clicked() {
                    go = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
        if go {
            let order = gedcom::renumber_order(&self.db, how(self.x.renum_how));
            let pm = gedcom::renumber(&mut self.db, &order);
            let p = |x: u32| pm.get(&x).copied().unwrap_or(x);
            // the screen follows the same people
            self.sel = self.sel.map(p);
            self.back.clear();
            self.fwd.clear();
            self.hl = None;
            self.viewing = None;
            self.rel_a = self.rel_a.map(p);
            self.rel_b = self.rel_b.map(p);
            self.merge_other = None;
            self.dups.clear();
            self.x.gal_sel.clear();
            self.dirty = true;
            self.status = format!("Renumbered {} people and {} families. Undo (Ctrl+Z) puts the old numbers back.", n, nf);
            self.x.renum_open = false;
        }
        if cancel || !open {
            self.x.renum_open = false;
        }
    }

    pub(crate) fn versions_window(&mut self, ctx: &egui::Context) {
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
    pub(crate) fn history_window(&mut self, ctx: &egui::Context) {
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
}

impl App {
    /// Things worth checking about this person (from the data checker), shown on their page.
    pub(crate) fn problem_flags(&mut self, ui: &mut egui::Ui, id: u32) {
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
    pub(crate) fn todo_panel(&mut self, ui: &mut egui::Ui, id: u32) {
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
    pub(crate) fn welcome(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
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
    pub(crate) fn start_practice(&mut self) {
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
        self.tree_replaced();
        self.remember_state();
        self.x.geo_cache.clear();
        self.status = "You are exploring the sample family. Nothing here is saved. Use 'Back to my tree' at the top when you are done.".into();
    }

    pub(crate) fn end_practice(&mut self) {
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
            self.tree_replaced();
            self.remember_state();
            self.x.geo_cache.clear();
            self.status = "Back to your own tree.".into();
        }
    }
}

impl App {
    /// Windows for the archive (confirm restore, progress) and backup settings.
    pub(crate) fn archive_windows(&mut self, ctx: &egui::Context) {
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
                            self.sel = self.db.home.or_else(|| self.db.people.keys().next().copied());
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
}

impl App {
    /// File > Family trees: keep unrelated research in separate trees.
    pub(crate) fn trees_window(&mut self, ctx: &egui::Context) {
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
    pub(crate) fn todo_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
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
    pub(crate) fn places_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
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
}
