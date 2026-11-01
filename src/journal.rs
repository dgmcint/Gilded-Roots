//! The research journal (what was searched, where, and what came of it) and historical events
//! shown beside a person's timeline.

use super::*;

/// Built-in historical events: (from year, to year, what, region). Region "World" always shows;
/// others show when the person's places mention that part of the world.
pub const HISTORY: &[(i32, i32, &str, &str)] = &[
    (1607, 1607, "Jamestown founded", "USA"),
    (1620, 1620, "Mayflower lands at Plymouth", "USA"),
    (1642, 1651, "English Civil War", "UK"),
    (1692, 1692, "Salem witch trials", "USA"),
    (1707, 1707, "Union of England and Scotland", "UK"),
    (1756, 1763, "Seven Years' War (French and Indian War)", "World"),
    (1775, 1783, "American Revolutionary War", "USA"),
    (1789, 1799, "French Revolution", "Europe"),
    (1790, 1790, "First United States census", "USA"),
    (1803, 1815, "Napoleonic Wars", "Europe"),
    (1803, 1803, "Louisiana Purchase", "USA"),
    (1812, 1815, "War of 1812", "USA"),
    (1825, 1825, "Erie Canal opens", "USA"),
    (1837, 1837, "Civil registration of births, marriages and deaths begins in England and Wales", "UK"),
    (1841, 1841, "First UK census to list every person by name", "UK"),
    (1845, 1852, "Great Famine in Ireland", "Ireland"),
    (1846, 1848, "Mexican-American War", "USA"),
    (1847, 1847, "Mormon pioneers reach the Salt Lake Valley", "USA"),
    (1848, 1855, "California Gold Rush", "USA"),
    (1850, 1850, "First US census to name every person in a household", "USA"),
    (1861, 1865, "American Civil War", "USA"),
    (1862, 1862, "Homestead Act", "USA"),
    (1864, 1864, "Second Schleswig War (Denmark and Prussia)", "Scandinavia"),
    (1867, 1867, "Canadian Confederation", "Canada"),
    (1869, 1869, "Transcontinental railroad completed", "USA"),
    (1876, 1876, "Telephone invented", "World"),
    (1890, 1890, "Most of the 1890 US census later lost in a fire (1921)", "USA"),
    (1892, 1954, "Ellis Island immigration station open", "USA"),
    (1898, 1898, "Spanish-American War", "USA"),
    (1903, 1903, "First powered flight", "World"),
    (1906, 1906, "San Francisco earthquake", "USA"),
    (1912, 1912, "Titanic sinks", "World"),
    (1914, 1918, "First World War", "World"),
    (1917, 1918, "US draft registration for the First World War", "USA"),
    (1918, 1920, "Influenza pandemic", "World"),
    (1920, 1920, "Women win the vote in the United States", "USA"),
    (1929, 1939, "Great Depression", "World"),
    (1930, 1936, "Dust Bowl", "USA"),
    (1939, 1945, "Second World War", "World"),
    (1940, 1940, "1940 US census (released to the public in 2012)", "USA"),
    (1945, 1945, "End of the Second World War", "World"),
    (1950, 1953, "Korean War", "World"),
    (1957, 1957, "Sputnik, the first satellite", "World"),
    (1963, 1963, "President Kennedy assassinated", "USA"),
    (1955, 1975, "Vietnam War", "World"),
    (1969, 1969, "First Moon landing", "World"),
    (1989, 1989, "Fall of the Berlin Wall", "World"),
    (1991, 1991, "World Wide Web opens to the public", "World"),
    (2001, 2001, "September 11 attacks", "USA"),
    (2008, 2008, "Global financial crisis", "World"),
    (2020, 2022, "COVID-19 pandemic", "World"),
];

/// Which regions a person's places belong to.
fn regions_of(p: &Person) -> Vec<&'static str> {
    let mut all = String::new();
    for s in [&p.bp, &p.dp, &p.chp, &p.bup] {
        all.push_str(&s.to_lowercase());
        all.push(' ');
    }
    for e in &p.events {
        all.push_str(&e.place.to_lowercase());
        all.push(' ');
    }
    let mut v = vec!["World"];
    let us_words = ["usa", "united states", "america", "utah", "idaho", "oregon", "california", "ohio", "missouri", "washington", "new york", "texas", "nebraska", "kansas", "virginia", "pennsylvania", "illinois", "massachusetts", "michigan", "iowa", "colorado", "arizona", "nevada", "montana", "wyoming"];
    if us_words.iter().any(|w| all.contains(w)) {
        v.push("USA");
    }
    if ["england", "scotland", "wales", "united kingdom", "britain", "yorkshire", "london"].iter().any(|w| all.contains(w)) {
        v.push("UK");
        v.push("Europe");
    }
    if all.contains("ireland") {
        v.push("Ireland");
        v.push("Europe");
    }
    if ["denmark", "norway", "sweden", "finland", "iceland"].iter().any(|w| all.contains(w)) {
        v.push("Scandinavia");
        v.push("Europe");
    }
    if ["france", "germany", "netherlands", "belgium", "italy", "spain", "switzerland", "poland", "austria"].iter().any(|w| all.contains(w)) {
        v.push("Europe");
    }
    if all.contains("canada") || ["ontario", "quebec", "alberta", "manitoba", "nova scotia"].iter().any(|w| all.contains(w)) {
        v.push("Canada");
    }
    v
}

impl App {
    /// Historical events during a person's life: (year, text).
    pub(crate) fn history_for(&self, p: &Person) -> Vec<(i32, String)> {
        let by = parse_date(&p.bd).0;
        let dy = parse_date(&p.dd).0;
        let Some(b) = by.or(dy.map(|d| d - 70)) else { return Vec::new() };
        let d = dy.unwrap_or_else(|| if p.living { current_year() } else { (b + 80).min(current_year()) });
        let regions = regions_of(p);
        let mut out: Vec<(i32, String)> = Vec::new();
        let custom: Vec<(i32, i32, String, String)> = self.db.history.clone();
        let builtin = HISTORY.iter().map(|(a, b, t, r)| (*a, *b, t.to_string(), r.to_string()));
        for (from, to, text, region) in builtin.chain(custom) {
            if to < b || from > d {
                continue;
            }
            if !(region.is_empty() || regions.contains(&region.as_str()) || !HISTORY.iter().any(|h| h.3 == region)) {
                continue;
            }
            let age = (from.max(b) - b).max(0);
            let when = if from == to { from.to_string() } else { format!("{}-{}", from, to) };
            out.push((from.max(b), format!("{}: {} (age {})", when, text, age)));
        }
        out
    }

    /// One person's journal entries, shown on their Individual page.
    pub(crate) fn journal_panel(&mut self, ui: &mut egui::Ui, id: u32) {
        let entries: Vec<usize> = self.db.journal.iter().enumerate().filter(|(_, j)| j.people.contains(&id)).map(|(i, _)| i).collect();
        if entries.is_empty() {
            ui.label(RichText::new("No searches recorded yet. Note each search, even the ones that found nothing: it saves looking twice.").italics().weak());
        }
        for i in entries.iter().rev() {
            self.journal_card(ui, *i, false);
        }
        if ui.button("Add a journal entry...").clicked() {
            self.x.journal_new = Some(Journal { date: print_date(&today()), people: vec![id], ..Default::default() });
            self.x.journal_open = true;
        }
    }

    fn journal_card(&self, ui: &mut egui::Ui, i: usize, show_people: bool) {
        let Some(j) = self.db.journal.get(i) else { return };
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                let (txt, col) = match j.result.as_str() {
                    "found" => ("Found", Color32::from_rgb(56, 142, 60)),
                    "partial" => ("Partly found", Color32::from_rgb(196, 152, 40)),
                    "nothing" => ("Nothing found", Color32::from_rgb(200, 90, 70)),
                    _ => ("Not finished", Color32::GRAY),
                };
                ui.label(RichText::new(j.date.as_str()).strong());
                ui.label(RichText::new(txt).color(col).small());
            });
            if !j.goal.is_empty() {
                ui.label(RichText::new(format!("Looking for: {}", j.goal)).size(14.0));
            }
            if !j.place.is_empty() {
                ui.label(RichText::new(format!("Where: {}", j.place)).weak());
            }
            if let Some(s) = j.sid.and_then(|s| self.db.sources.get(&s)) {
                ui.label(RichText::new(format!("Source: {}", s.title)).weak());
            }
            if !j.notes.is_empty() {
                ui.add(egui::Label::new(j.notes.as_str()).wrap());
            }
            if show_people && !j.people.is_empty() {
                let names: Vec<String> = j.people.iter().filter(|p| self.db.people.contains_key(p)).map(|p| self.db.label(*p)).collect();
                ui.label(RichText::new(format!("About: {}", names.join(", "))).small().weak());
            }
        });
    }

    /// Tools > Research journal: every entry, newest first, and the form to add or edit one.
    pub(crate) fn journal_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.journal_open {
            return;
        }
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        let mut remove: Option<usize> = None;
        let mut edit: Option<usize> = None;
        let mut go: Option<u32> = None;
        let sources: Vec<(u32, String)> = self.db.sources.iter().map(|(k, s)| (*k, s.title.clone())).collect();
        egui::Window::new("Research journal").open(&mut open).default_size([640.0, 600.0]).show(ctx, |ui| {
            if let Some(j) = self.x.journal_new.as_mut() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(if self.x.journal_edit.is_some() { "Edit entry" } else { "New entry" }).strong());
                    egui::Grid::new("jform").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("Date");
                        ui.add(egui::TextEdit::singleline(&mut j.date).desired_width(160.0));
                        ui.end_row();
                        ui.label("Looking for");
                        ui.add(egui::TextEdit::singleline(&mut j.goal).hint_text("e.g. Thomas's marriage record").desired_width(380.0));
                        ui.end_row();
                        ui.label("Where");
                        ui.add(egui::TextEdit::singleline(&mut j.place).hint_text("archive, website, book, relative...").desired_width(380.0));
                        ui.end_row();
                        ui.label("Source");
                        let cur = j.sid.and_then(|s| sources.iter().find(|x| x.0 == s)).map(|x| x.1.clone()).unwrap_or_else(|| "(none)".into());
                        egui::ComboBox::from_id_salt("jsrc").selected_text(cur).width(380.0).show_ui(ui, |ui| {
                            if ui.selectable_label(j.sid.is_none(), "(none)").clicked() {
                                j.sid = None;
                            }
                            for (k, t) in &sources {
                                if ui.selectable_label(j.sid == Some(*k), t.as_str()).clicked() {
                                    j.sid = Some(*k);
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("Result");
                        ui.horizontal(|ui| {
                            for (v, l) in [("found", "Found"), ("partial", "Partly"), ("nothing", "Nothing"), ("", "Not finished")] {
                                if ui.selectable_label(j.result == v, l).clicked() {
                                    j.result = v.to_string();
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("What I learned");
                        ui.add(egui::TextEdit::multiline(&mut j.notes).desired_rows(4).desired_width(380.0));
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Save entry").strong()).clicked() {
                            save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
                ui.add_space(6.0);
            } else if ui.button("+ New entry").clicked() {
                self.x.journal_new = Some(Journal { date: print_date(&today()), people: self.sel.into_iter().collect(), ..Default::default() });
            }
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.x.journal_find).hint_text("Filter by words or names").desired_width(240.0));
                ui.label(RichText::new(format!("{} entries", self.db.journal.len())).weak());
            });
            ui.separator();
            let q = self.x.journal_find.trim().to_lowercase();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for i in (0..self.db.journal.len()).rev() {
                    let j = &self.db.journal[i];
                    let names: String = j.people.iter().filter_map(|p| self.db.people.get(p)).map(|p| p.name.to_lowercase()).collect::<Vec<_>>().join(" ");
                    if !q.is_empty() && !format!("{} {} {} {} {}", j.goal, j.place, j.notes, j.date, names).to_lowercase().contains(&q) {
                        continue;
                    }
                    self.journal_card(ui, i, true);
                    ui.horizontal(|ui| {
                        for p in j.people.iter().filter(|p| self.db.people.contains_key(p)) {
                            if ui.small_button(format!("Go to {}", self.db.people[p].name)).clicked() {
                                go = Some(*p);
                            }
                        }
                        if ui.small_button("Edit").clicked() {
                            edit = Some(i);
                        }
                        if ui.small_button("Delete").clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.add_space(4.0);
                }
            });
        });
        if save {
            if let Some(j) = self.x.journal_new.take() {
                match self.x.journal_edit.take() {
                    Some(i) if i < self.db.journal.len() => self.db.journal[i] = j,
                    _ => self.db.journal.push(j),
                }
                self.x.next_label = Some("Research journal entry saved".into());
                self.dirty = true;
            }
        } else if cancel {
            self.x.journal_new = None;
            self.x.journal_edit = None;
        }
        if let Some(i) = edit {
            self.x.journal_new = self.db.journal.get(i).cloned();
            self.x.journal_edit = Some(i);
        }
        if let Some(i) = remove {
            if i < self.db.journal.len() {
                self.db.journal.remove(i);
                self.x.next_label = Some("Research journal entry deleted".into());
                self.dirty = true;
            }
        }
        if let Some(p) = go {
            *act = Some(Action::Open(p));
        }
        self.x.journal_open = open;
    }

    /// Tools > Historical events: the built-in list, and your own additions.
    pub(crate) fn history_events_window(&mut self, ctx: &egui::Context) {
        if !self.x.hist_open {
            return;
        }
        if self.x.hist_new.0 == 0 {
            self.x.hist_new = (1900, 1900, String::new());
        }
        let mut open = true;
        let mut del: Option<usize> = None;
        egui::Window::new("Historical events").open(&mut open).default_size([560.0, 520.0]).show(ctx, |ui| {
            ui.label("These appear in a person's timeline when they happened during that person's life. World events always show; regional ones show when the person's places are in that region.");
            if ui.checkbox(&mut self.settings.show_history, "Show historical events in timelines").changed() {
                save_settings(&self.settings);
            }
            ui.separator();
            ui.label(RichText::new("Add your own (a local flood, a mine closing, a move of the whole family...)").strong());
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.x.hist_new.0).range(1000..=2100).prefix("from "));
                ui.add(egui::DragValue::new(&mut self.x.hist_new.1).range(1000..=2100).prefix("to "));
                ui.add(egui::TextEdit::singleline(&mut self.x.hist_new.2).hint_text("what happened").desired_width(240.0));
                if ui.add_enabled(!self.x.hist_new.2.trim().is_empty(), egui::Button::new("Add")).clicked() {
                    let (a, b, t) = (self.x.hist_new.0, self.x.hist_new.1.max(self.x.hist_new.0), self.x.hist_new.2.trim().to_string());
                    self.db.history.push((a, b, t, String::new()));
                    self.x.hist_new.2.clear();
                    self.dirty = true;
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if !self.db.history.is_empty() {
                    ui.label(RichText::new("Yours").strong());
                    for (i, (a, b, t, _)) in self.db.history.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(if a == b { a.to_string() } else { format!("{}-{}", a, b) });
                            ui.label(t.as_str());
                            if ui.small_button("x").clicked() {
                                del = Some(i);
                            }
                        });
                    }
                    ui.separator();
                }
                ui.label(RichText::new("Built in").strong());
                for (a, b, t, r) in HISTORY {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(if a == b { a.to_string() } else { format!("{}-{}", a, b) }).monospace());
                        ui.label(*t);
                        ui.label(RichText::new(*r).small().weak());
                    });
                }
            });
        });
        if let Some(i) = del {
            self.db.history.remove(i);
            self.dirty = true;
        }
        self.x.hist_open = open;
    }
}
