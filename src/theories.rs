//! Two pages of the Research tab beside the files: Theories (a question you are working on, the
//! evidence for and against, and how sure you are) and Letters (a log of the letters and emails
//! you send and receive: archives, libraries, relatives, and whether they have answered).

use super::*;

pub(crate) const THEORY_STATUS: [&str; 5] = ["Still open", "Likely", "Proven", "Disproven", "Set aside"];
pub(crate) const LETTER_STATUS: [&str; 4] = ["Waiting for a reply", "Answered", "No reply", "No reply needed"];

/// A piece of evidence for (or against) a theory.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub(crate) struct Evidence {
    /// For the theory (true) or against it.
    pub(crate) for_it: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    /// A research file it comes from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) research: Option<u32>,
    /// A source it comes from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sid: Option<u32>,
}

/// A research question and what is known about it.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub(crate) struct Theory {
    pub(crate) id: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) title: String,
    /// The question in full: "Was Thomas's father the William Harwood in the 1880 census of Leeds?"
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) question: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) people: Vec<u32>,
    /// An index into THEORY_STATUS.
    pub(crate) status: u8,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) evidence: Vec<Evidence>,
    /// What you concluded, and why.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) conclusion: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) added: String,
}

/// A letter or email sent or received while researching.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub(crate) struct Letter {
    pub(crate) id: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) date: String,
    /// Sent by you (true) or received.
    pub(crate) sent: bool,
    /// Who it was to or from: a relative, an archive, a county clerk.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) who: String,
    /// Their address or email.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) addr: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) subject: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) people: Vec<u32>,
    /// An index into LETTER_STATUS.
    pub(crate) status: u8,
    /// When to write again if no answer comes.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) follow_up: String,
    /// The letter itself, kept in Research (a scan, an email).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) research: Option<u32>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) notes: String,
}

/// "2026-12-01" as a number to compare, when the date is written that way.
fn iso_day(d: &str) -> Option<u32> {
    let p: Vec<u32> = d.trim().split('-').filter_map(|x| x.parse().ok()).collect();
    (p.len() == 3 && p[0] > 1000).then(|| p[0] * 10000 + p[1] * 100 + p[2])
}

impl Letter {
    /// Still waiting, and the day to write again has come.
    pub(crate) fn overdue(&self, today: &str) -> bool {
        self.status == 0 && matches!((iso_day(&self.follow_up), iso_day(today)), (Some(f), Some(t)) if f <= t)
    }
}

/// People chips with a link and an x, and "+ Link" for the selected person. Gives what changed.
fn people_row(ui: &mut egui::Ui, db: &Db, people: &mut Vec<u32>, sel: Option<u32>, act: &mut Option<Action>) -> bool {
    let tr = crate::i18n::tr;
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        let mut drop = None;
        for p in people.iter() {
            let name = db.people.get(p).map(|x| x.name.clone()).unwrap_or_else(|| format!("#{p}"));
            if ui.link(name).on_hover_text(tr("Open their page")).clicked() {
                *act = Some(Action::Open(*p));
            }
            if ui.small_button("x").on_hover_text(tr("Unlink")).clicked() {
                drop = Some(*p);
            }
            ui.add_space(6.0);
        }
        if let Some(d) = drop {
            people.retain(|x| *x != d);
            changed = true;
        }
        match sel.filter(|s| !people.contains(s)) {
            Some(s) => {
                if ui.button(crate::i18n::trf("+ Link {}", &["+ Link ", ""], &[db.label(s)])).clicked() {
                    people.push(s);
                    changed = true;
                }
            }
            None if people.is_empty() => {
                ui.label(RichText::new(tr("Select a person (on any tab, or with Find) to link them.")).small().weak());
            }
            None => {}
        }
    });
    changed
}

/// A drop-down of research files (or none). Gives what changed.
fn research_combo(ui: &mut egui::Ui, id: &str, db: &Db, cur: &mut Option<u32>) -> bool {
    let tr = crate::i18n::tr;
    let mut changed = false;
    let shown = cur.and_then(|c| db.research.iter().find(|r| r.id == c)).map(|r| r.title.clone()).unwrap_or_else(|| tr("(none)").to_string());
    egui::ComboBox::from_id_salt(id).selected_text(shown).width(220.0).show_ui(ui, |ui| {
        if ui.selectable_label(cur.is_none(), tr("(none)")).clicked() {
            *cur = None;
            changed = true;
        }
        for r in &db.research {
            if ui.selectable_label(*cur == Some(r.id), format!("{}  {}", r.title, r.date)).clicked() {
                *cur = Some(r.id);
                changed = true;
            }
        }
    });
    changed
}

impl App {
    /// The Theories page of the Research tab.
    pub(crate) fn theories_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let tr = crate::i18n::tr;
        let sel = self.sel.filter(|s| self.db.people.contains_key(s));
        ui.horizontal(|ui| {
            if ui.button(RichText::new(tr("+ New theory")).strong()).clicked() {
                let id = self.db.theories.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                self.db.theories.push(Theory { id, title: tr("New theory").to_string(), people: sel.into_iter().collect(), added: today(), ..Default::default() });
                self.x.theory_sel = Some(id);
                self.dirty = true;
            }
            ui.label(RichText::new(tr("A question you are working on, the evidence for and against it, and how sure you are.")).small().weak());
        });
        ui.separator();
        if self.db.theories.is_empty() {
            ui.label(RichText::new(tr("No theories yet. For example: \"Was Thomas's father the William Harwood in the 1880 census of Leeds?\" Add the clues for and against as you find them, and mark it Proven or Disproven when you know.")).weak());
            return;
        }
        let avail = ui.available_size();
        let mut changed = false;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2((avail.x * 0.34).clamp(240.0, 420.0), avail.y), egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                egui::ScrollArea::vertical().id_salt("theory_list").auto_shrink([false, false]).show(ui, |ui| {
                    for t in &self.db.theories {
                        let st = crate::i18n::trv(THEORY_STATUS.get(t.status as usize).copied().unwrap_or("Still open"));
                        let n_for = t.evidence.iter().filter(|e| e.for_it).count();
                        let n_against = t.evidence.len() - n_for;
                        let label = format!("{}\n{st} · +{n_for} / -{n_against}", t.title);
                        if ui.selectable_label(self.x.theory_sel == Some(t.id), label).clicked() {
                            self.x.theory_sel = Some(t.id);
                        }
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                let Some(ix) = self.x.theory_sel.and_then(|s| self.db.theories.iter().position(|t| t.id == s)) else {
                    ui.label(RichText::new(tr("Choose a theory on the left.")).weak());
                    return;
                };
                egui::ScrollArea::vertical().id_salt("theory_item").auto_shrink([false, false]).show(ui, |ui| {
                    let db = &self.db;
                    let mut t = db.theories[ix].clone();
                    changed |= ui.add(egui::TextEdit::singleline(&mut t.title).font(egui::TextStyle::Heading).desired_width(f32::INFINITY)).changed();
                    ui.horizontal(|ui| {
                        ui.label(tr("How sure:"));
                        for (i, s) in THEORY_STATUS.iter().enumerate() {
                            if ui.selectable_label(t.status as usize == i, crate::i18n::trv(s)).clicked() {
                                t.status = i as u8;
                                changed = true;
                            }
                        }
                    });
                    ui.label(RichText::new(tr("The question")).strong());
                    changed |= ui.add(egui::TextEdit::multiline(&mut t.question).desired_rows(2).desired_width(f32::INFINITY)).changed();
                    ui.label(RichText::new(tr("About")).strong());
                    changed |= people_row(ui, db, &mut t.people, sel, act);
                    ui.add_space(4.0);
                    for (for_it, head) in [(true, tr("Evidence for")), (false, tr("Evidence against"))] {
                        ui.label(RichText::new(head).strong().color(if for_it { Color32::from_rgb(70, 160, 90) } else { Color32::from_rgb(200, 90, 70) }));
                        let mut drop = None;
                        for (i, e) in t.evidence.iter_mut().enumerate().filter(|(_, e)| e.for_it == for_it) {
                            ui.push_id(("ev", i), |ui| {
                                ui.horizontal(|ui| {
                                    changed |= ui.add(egui::TextEdit::multiline(&mut e.text).desired_rows(1).desired_width((ui.available_width() - 400.0).max(200.0))).changed();
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(tr("File:")).small().weak());
                                            changed |= research_combo(ui, "evr", db, &mut e.research);
                                            if let Some(r) = e.research {
                                                if ui.small_button(tr("Open")).clicked() {
                                                    self.x.research_sel = Some(r);
                                                    self.x.research_tab = 0;
                                                }
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(tr("Source:")).small().weak());
                                            let cur = e.sid.and_then(|s| db.sources.get(&s)).map(source_title).unwrap_or_else(|| tr("(none)").to_string());
                                            egui::ComboBox::from_id_salt("evs").selected_text(cur).width(220.0).show_ui(ui, |ui| {
                                                if ui.selectable_label(e.sid.is_none(), tr("(none)")).clicked() {
                                                    e.sid = None;
                                                    changed = true;
                                                }
                                                for (k, s) in &db.sources {
                                                    if ui.selectable_label(e.sid == Some(*k), source_title(s)).clicked() {
                                                        e.sid = Some(*k);
                                                        changed = true;
                                                    }
                                                }
                                            });
                                        });
                                    });
                                    if ui.small_button("x").on_hover_text(tr("Remove")).clicked() {
                                        drop = Some(i);
                                    }
                                });
                            });
                        }
                        if let Some(i) = drop {
                            t.evidence.remove(i);
                            changed = true;
                        }
                        if ui.small_button(if for_it { tr("+ Evidence for") } else { tr("+ Evidence against") }).clicked() {
                            t.evidence.push(Evidence { for_it, ..Default::default() });
                            changed = true;
                        }
                        ui.add_space(4.0);
                    }
                    ui.label(RichText::new(tr("Conclusion")).strong());
                    ui.label(RichText::new(tr("What you decided, and why. This is what someone reading your research later will want.")).small().weak());
                    changed |= ui.add(egui::TextEdit::multiline(&mut t.conclusion).desired_rows(3).desired_width(f32::INFINITY)).changed();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if self.x.theory_confirm {
                            ui.label(tr("Delete this theory? Undo brings it back."));
                            if ui.button(tr("Delete")).clicked() {
                                self.x.theory_confirm = false;
                                self.x.theory_sel = None;
                                t.id = 0;
                                changed = true;
                            }
                            if ui.button(tr("Cancel")).clicked() {
                                self.x.theory_confirm = false;
                            }
                        } else if ui.button(tr("Delete theory...")).clicked() {
                            self.x.theory_confirm = true;
                        }
                    });
                    if changed {
                        if t.id == 0 {
                            self.db.theories.remove(ix);
                        } else {
                            self.db.theories[ix] = t;
                        }
                    }
                });
            });
        });
        if changed {
            self.dirty = true;
        }
    }

    /// The Letters page of the Research tab.
    pub(crate) fn letters_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let tr = crate::i18n::tr;
        let sel = self.sel.filter(|s| self.db.people.contains_key(s));
        let now = today();
        ui.horizontal(|ui| {
            for (sent, label) in [(true, tr("+ Letter sent")), (false, tr("+ Letter received"))] {
                if ui.button(RichText::new(label).strong()).clicked() {
                    let id = self.db.letters.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                    self.db.letters.push(Letter { id, date: now.clone(), sent, status: if sent { 0 } else { 3 }, people: sel.into_iter().collect(), ..Default::default() });
                    self.x.letter_sel = Some(id);
                    self.dirty = true;
                }
            }
            let waiting = self.db.letters.iter().filter(|l| l.status == 0).count();
            let due = self.db.letters.iter().filter(|l| l.overdue(&now)).count();
            ui.label(RichText::new(crate::i18n::trf("{} waiting for a reply, {} to write again", &["", " waiting for a reply, ", " to write again"], &[waiting.to_string(), due.to_string()])).small().weak());
        });
        ui.separator();
        if self.db.letters.is_empty() {
            ui.label(RichText::new(tr("Keep a log of the letters and emails you send to archives, libraries, churches and relatives, and what came back. Letters still waiting for a reply are counted, and a follow-up day (written like 2026-12-01) marks when to write again.")).weak());
            return;
        }
        let avail = ui.available_size();
        let mut changed = false;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2((avail.x * 0.36).clamp(260.0, 440.0), avail.y), egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                egui::ScrollArea::vertical().id_salt("letter_list").auto_shrink([false, false]).show(ui, |ui| {
                    let mut order: Vec<&Letter> = self.db.letters.iter().collect();
                    order.sort_by(|a, b| b.date.cmp(&a.date));
                    for l in order {
                        let dir = if l.sent { tr("To") } else { tr("From") };
                        let st = crate::i18n::trv(LETTER_STATUS.get(l.status as usize).copied().unwrap_or("Answered"));
                        let mut text = RichText::new(format!("{}  {dir} {}\n{} · {st}", l.date, l.who, l.subject));
                        if l.overdue(&now) {
                            text = text.color(Color32::from_rgb(210, 120, 40));
                        }
                        if ui.selectable_label(self.x.letter_sel == Some(l.id), text).clicked() {
                            self.x.letter_sel = Some(l.id);
                        }
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                let Some(ix) = self.x.letter_sel.and_then(|s| self.db.letters.iter().position(|t| t.id == s)) else {
                    ui.label(RichText::new(tr("Choose a letter on the left.")).weak());
                    return;
                };
                let db = &self.db;
                let mut l = db.letters[ix].clone();
                egui::Grid::new("letter_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("Date:"));
                    changed |= ui.add(egui::TextEdit::singleline(&mut l.date).desired_width(140.0)).changed();
                    ui.end_row();
                    ui.label(tr("Direction:"));
                    ui.horizontal(|ui| {
                        changed |= ui.radio_value(&mut l.sent, true, tr("I sent it")).changed();
                        changed |= ui.radio_value(&mut l.sent, false, tr("I received it")).changed();
                    });
                    ui.end_row();
                    ui.label(if l.sent { tr("To:") } else { tr("From:") });
                    changed |= ui.add(egui::TextEdit::singleline(&mut l.who).hint_text(tr("Oregon State Archives, Aunt Ruth...")).desired_width(320.0)).changed();
                    ui.end_row();
                    ui.label(tr("Address or email:"));
                    changed |= ui.add(egui::TextEdit::singleline(&mut l.addr).desired_width(320.0)).changed();
                    ui.end_row();
                    ui.label(tr("Subject:"));
                    changed |= ui.add(egui::TextEdit::singleline(&mut l.subject).hint_text(tr("Request for a death certificate")).desired_width(320.0)).changed();
                    ui.end_row();
                    ui.label(tr("Status:"));
                    egui::ComboBox::from_id_salt("letter_status").selected_text(crate::i18n::trv(LETTER_STATUS.get(l.status as usize).copied().unwrap_or("Answered"))).show_ui(ui, |ui| {
                        for (i, s) in LETTER_STATUS.iter().enumerate() {
                            if ui.selectable_label(l.status as usize == i, crate::i18n::trv(s)).clicked() {
                                l.status = i as u8;
                                changed = true;
                            }
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Write again on:"));
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::TextEdit::singleline(&mut l.follow_up).hint_text("2026-12-01").desired_width(140.0)).changed();
                        if l.overdue(&now) {
                            ui.label(RichText::new(tr("Time to write again")).color(Color32::from_rgb(210, 120, 40)));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("The letter itself:"));
                    ui.horizontal(|ui| {
                        changed |= research_combo(ui, "letter_file", db, &mut l.research);
                        if let Some(r) = l.research {
                            if ui.small_button(tr("Open")).clicked() {
                                self.x.research_sel = Some(r);
                                self.x.research_tab = 0;
                            }
                        }
                    });
                    ui.end_row();
                });
                ui.label(RichText::new(tr("About")).strong());
                changed |= people_row(ui, db, &mut l.people, sel, act);
                ui.label(RichText::new(tr("Notes")).strong());
                changed |= ui.add(egui::TextEdit::multiline(&mut l.notes).desired_rows(5).desired_width(f32::INFINITY)).changed();
                ui.add_space(8.0);
                let mut delete = false;
                ui.horizontal(|ui| {
                    if self.x.letter_confirm {
                        ui.label(tr("Delete this letter from the log? Undo brings it back."));
                        if ui.button(tr("Delete")).clicked() {
                            delete = true;
                        }
                        if ui.button(tr("Cancel")).clicked() {
                            self.x.letter_confirm = false;
                        }
                    } else if ui.button(tr("Delete from the log...")).clicked() {
                        self.x.letter_confirm = true;
                    }
                });
                if delete {
                    self.db.letters.remove(ix);
                    self.x.letter_sel = None;
                    self.x.letter_confirm = false;
                    changed = true;
                } else if changed {
                    self.db.letters[ix] = l;
                }
            });
        });
        if changed {
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_know_when_to_write_again() {
        let l = Letter { status: 0, follow_up: "2026-12-01".into(), ..Default::default() };
        assert!(!l.overdue("2026-11-30"));
        assert!(l.overdue("2026-12-01"));
        assert!(!Letter { status: 1, ..l.clone() }.overdue("2027-01-01"));
        assert!(!Letter { follow_up: "next spring".into(), ..l }.overdue("2027-01-01"));
    }

    #[test]
    fn theories_and_letters_are_kept_and_drawn() {
        let mut db = crate::sample::sample_family();
        let p = *db.people.keys().next().unwrap();
        db.theories.push(Theory { id: 1, title: "Leeds William".into(), people: vec![p], evidence: vec![Evidence { for_it: true, text: "Same age".into(), ..Default::default() }, Evidence { text: "Different wife".into(), ..Default::default() }], ..Default::default() });
        db.letters.push(Letter { id: 1, sent: true, who: "Oregon State Archives".into(), follow_up: "2000-01-01".into(), ..Default::default() });
        let back: Db = serde_json::from_str(&serde_json::to_string(&db).unwrap()).unwrap();
        assert_eq!(back.theories, db.theories);
        assert_eq!(back.letters, db.letters);
        let mut app = App::for_test(db);
        app.sel = Some(p);
        app.x.theory_sel = Some(1);
        app.x.letter_sel = Some(1);
        let ctx = egui::Context::default();
        for tab in [1u8, 2] {
            app.x.research_tab = tab;
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                let mut act = None;
                egui::CentralPanel::default().show(ctx, |ui| app.research_view(ui, &mut act));
            });
        }
        assert_eq!(app.db.theories.len(), 1);
        assert!(crate::searchall::search_everything(&app.db, "oregon archives").iter().any(|f| f.part == "Letter"));
        assert!(crate::searchall::search_everything(&app.db, "different wife").iter().any(|f| f.part == "Theory"));
    }
}
