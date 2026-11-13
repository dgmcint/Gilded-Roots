//! Keyboard shortcuts and the quick-jump search.

use super::*;

impl App {
    /// Keys that work anywhere. On a Mac the top-row keys are brightness and volume unless fn is
    /// held, so help and full screen also have the usual Mac shortcuts.
    ///   F1 help   F2 edit person   F3 find   F4 quick jump   F5 fit chart   F6 next tab
    ///   F7 data checker   F8 research to-do   F9 places   F11 full screen   F12 history
    pub(crate) fn global_keys(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
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
                        View::Person | View::Gallery => View::Memories,
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
    pub(crate) fn jump_results(&self, q: &str) -> Vec<(String, String, Jump)> {
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
            ("Photos (Memories tab)", Jump::View(View::Gallery)),
            ("Charts: fan chart, hourglass, lifespans", Jump::View(View::Charts)),
            ("People table (everyone in a list)", Jump::View(View::Table)),
            ("Map of places", Jump::View(View::Map)),
            ("Memories: photos, stories, recordings, documents, notes", Jump::View(View::Memories)),
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
            ("Renumber people", Jump::Window("renum")),
            ("Special letters: accents and symbols (é, ñ, ß, †)", Jump::Window("letters")),
            ("Find by ID number", Jump::Window("find_id")),
            ("Find a couple", Jump::Window("find_couple")),
            ("Groups: focus and filter", Jump::Window("groups")),
            ("Date calculator", Jump::Window("calc")),
            ("Find and replace in notes", Jump::Window("nfr")),
            ("Soundex", Jump::Window("soundex")),
            ("List: everyone, alphabetical", Jump::Window("list0")),
            ("List: marriages", Jump::Window("list1")),
            ("List: end-of-line ancestors", Jump::Window("list2")),
            ("List: people linked to nobody", Jump::Window("list3")),
            ("List: missing LDS ordinances", Jump::Window("list4")),
            ("List: contact details", Jump::Window("list5")),
            ("List: possible duplicates", Jump::Window("list6")),
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
            // A number typed on its own finds that ID number first.
            if words.len() == 1 {
                for (id, old) in self.db.find_by_number(words[0]).into_iter().rev() {
                    people.retain(|x| x.1 != id);
                    let tag = if old { format!("Old ID {}", words[0].trim_start_matches('#')) } else { format!("#{id}") };
                    people.insert(0, (0, id, format!("{}   {}", self.db.label(id), tag)));
                }
            }
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

    pub(crate) fn run_jump(&mut self, j: Jump, act: &mut Option<Action>) {
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
                "letters" => self.x.letters_open = true,
                "groups" => self.x.groups_open = true,
                "calc" => self.x.calc_open = true,
                "nfr" => self.x.nfr_open = true,
                "soundex" => self.x.soundex_open = true,
                w if w.starts_with("list") => {
                    if let Ok(i) = w[4..].parse::<usize>() {
                        let html = self.list_html(i);
                        self.open_print(html);
                    }
                }
                "find_id" => {
                    self.find_open = true;
                    self.x.find_mode = 1;
                }
                "find_couple" => {
                    self.find_open = true;
                    self.x.find_mode = 2;
                }
                "renum" => {
                    self.x.renum_open = true;
                    self.x.renum_preview = None;
                }
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
    pub(crate) fn jump_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
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
}
