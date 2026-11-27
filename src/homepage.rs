//! The Home tab, shown when the program opens (View > Open on the Home tab turns that off): the home person,
//! recent people, the research to-do list, "On this day" in the family, what to look for next, the changes
//! made today, and when the last backup was made with a Back up now button.

use super::*;

/// Someone's day and month from a date as typed ("13 NOV 1843", "Nov 13, 1843"), when it has both.
fn day_month(s: &str) -> Option<(u32, u32, Option<i32>)> {
    let up = s.to_uppercase();
    if ["BEF", "AFT", "ABT", "BET", "EST", "CAL", "FROM"].iter().any(|q| up.trim_start().starts_with(q)) {
        return None;
    }
    let (y, m, d) = crate::reports::parse_date(s);
    Some((d?, m?, y))
}

/// Today's day and month, and the year.
fn today_dmy() -> (u32, u32, i32) {
    let t = today();
    let p: Vec<i32> = t.split('-').filter_map(|x| x.parse().ok()).collect();
    if p.len() == 3 {
        (p[2] as u32, p[1] as u32, p[0])
    } else {
        (1, 1, 2000)
    }
}

/// One "On this day" line: who, what happened, the year and how long ago.
pub(crate) struct Anniversary {
    pub(crate) who: u32,
    pub(crate) what: &'static str,
    pub(crate) year: Option<i32>,
}

/// Births, marriages and deaths in the family on a day of the year.
pub(crate) fn on_this_day(db: &Db, day: u32, month: u32) -> Vec<Anniversary> {
    let mut out = Vec::new();
    for (id, p) in &db.people {
        for (date, what) in [(&p.bd, "born"), (&p.dd, "died")] {
            if let Some((d, m, y)) = day_month(date) {
                if d == day && m == month {
                    out.push(Anniversary { who: *id, what, year: y });
                }
            }
        }
    }
    for f in db.fams.values() {
        if let Some((d, m, y)) = day_month(&f.md) {
            if d == day && m == month {
                if let Some(h) = f.h.or(f.w) {
                    out.push(Anniversary { who: h, what: "married", year: y });
                }
            }
        }
    }
    out.sort_by_key(|a| a.year.unwrap_or(9999));
    out
}

/// Things worth looking for: direct ancestors of the home person with no parents entered (the ends of
/// the line), and people with facts but no sources. Each list is short.
pub(crate) fn suggestions(db: &Db, home: Option<u32>) -> (Vec<u32>, Vec<u32>) {
    let mut ends = Vec::new();
    if let Some(h) = home.filter(|h| db.people.contains_key(h)) {
        let ix = db.index();
        let mut todo = vec![(h, 0usize)];
        let mut seen = HashSet::new();
        while let Some((p, g)) = todo.pop() {
            if !seen.insert(p) || g > 12 {
                continue;
            }
            let (f, m) = db.parents_ix(&ix, p);
            if f.is_none() && m.is_none() {
                if p != h {
                    ends.push(p);
                }
            } else {
                todo.extend(f.map(|x| (x, g + 1)));
                todo.extend(m.map(|x| (x, g + 1)));
            }
        }
        ends.sort_by_key(|p| db.people.get(p).map(|x| crate::reports::parse_date(&x.bd).0.unwrap_or(9999)).unwrap_or(9999));
    }
    let mut unsourced: Vec<u32> = db.people.iter().filter(|(_, p)| p.sources.is_empty() && (!p.bd.is_empty() || !p.dd.is_empty())).map(|(k, _)| *k).collect();
    unsourced.sort_by_key(|k| db.people.get(k).map(|p| p.name.to_lowercase()).unwrap_or_default());
    (ends, unsourced)
}

impl App {
    /// How long ago the newest backup was made (looked up at most once a minute).
    fn last_backup_age(&mut self, now: f64) -> Option<u64> {
        if self.x.home_backup.0 == 0.0 || now - self.x.home_backup.0 > 60.0 {
            let bdir = data_dir().join("backups");
            let newest = list_backups(&bdir).into_iter().filter_map(|n| std::fs::metadata(bdir.join(n)).and_then(|m| m.modified()).ok()).max();
            self.x.home_backup = (now.max(0.001), newest);
        }
        self.x.home_backup.1.and_then(|t| t.elapsed().ok()).map(|d| d.as_secs())
    }

    pub(crate) fn home_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let tr = crate::i18n::tr;
        let now = ui.input(|i| i.time);
        let (_, edge, box_c, _, _, head, _) = self.chart_colors(ui);
        let card = |ui: &mut egui::Ui, title: String, body: &mut dyn FnMut(&mut egui::Ui)| {
            egui::Frame::none().fill(box_c).stroke(edge).rounding(8.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(title).strong().size(16.0));
                ui.add_space(4.0);
                body(ui);
            });
            ui.add_space(10.0);
        };
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(6.0);
            let name = if self.settings.current_tree.is_empty() { tr("My family tree").to_string() } else { self.settings.current_tree.clone() };
            ui.horizontal(|ui| {
                ui.heading(RichText::new(name).size(24.0));
                ui.add_space(12.0);
                ui.label(
                    RichText::new(crate::i18n::trf(
                        "{} people, {} families, {} sources",
                        &["", " people, ", " families, ", " sources"],
                        &[self.db.people.len().to_string(), self.db.fams.len().to_string(), self.db.sources.len().to_string()],
                    ))
                    .color(head),
                );
            });
            ui.add_space(8.0);
            let wide = ui.available_width() > 900.0;
            let cols = if wide { 2 } else { 1 };
            ui.columns(cols, |c| {
                // left column (or the only one)
                {
                    let ui = &mut c[0];
                    let home = self.db.home.filter(|h| self.db.people.contains_key(h));
                    card(ui, tr("Home person").to_string(), &mut |ui| match home {
                        Some(h) => {
                            self.mini_card(ui, h, "", false, act);
                            ui.horizontal_wrapped(|ui| {
                                for (v, l) in [(View::Pedigree, "Pedigree"), (View::Family, "Family"), (View::Descendants, "Descendants"), (View::Person, "Individual")] {
                                    if ui.button(crate::i18n::trv(l)).clicked() {
                                        *act = Some(Action::Select(h));
                                        self.view = v;
                                        if v == View::Pedigree {
                                            self.settings.ped_mode = 0;
                                        }
                                    }
                                }
                            });
                        }
                        None => {
                            ui.label(RichText::new(tr("No home person yet. Choose one with File > Home person...")).weak());
                        }
                    });
                    // recent people
                    let mut recent: Vec<u32> = Vec::new();
                    for k in self.sel.iter().chain(self.back.iter().rev()) {
                        if self.db.people.contains_key(k) && !recent.contains(k) && Some(*k) != home {
                            recent.push(*k);
                        }
                        if recent.len() >= 6 {
                            break;
                        }
                    }
                    if !recent.is_empty() {
                        card(ui, tr("Recent people").to_string(), &mut |ui| {
                            for k in &recent {
                                self.mini_card(ui, *k, "", false, act);
                                ui.add_space(3.0);
                            }
                        });
                    }
                    // on this day
                    let (d, m, y) = today_dmy();
                    let days = on_this_day(&self.db, d, m);
                    card(ui, tr("On this day").to_string(), &mut |ui| {
                        if days.is_empty() {
                            ui.label(RichText::new(tr("Nobody in the family was born, married or died on this day.")).weak());
                        }
                        for a in days.iter().take(10) {
                            let Some(p) = self.db.people.get(&a.who) else { continue };
                            ui.horizontal_wrapped(|ui| {
                                if ui.link(styled_name(p)).clicked() {
                                    *act = Some(Action::Select(a.who));
                                }
                                let what = match a.what {
                                    "born" => tr("was born"),
                                    "married" => tr("was married"),
                                    _ => tr("died"),
                                };
                                match a.year {
                                    Some(yr) if yr <= y => {
                                        ui.label(crate::i18n::trf("{} in {}, {} years ago", &["", " in ", ", ", " years ago"], &[what.to_string(), yr.to_string(), (y - yr).to_string()]));
                                    }
                                    _ => {
                                        ui.label(what);
                                    }
                                }
                            });
                        }
                    });
                }
                // right column (or below)
                {
                    let ui = &mut c[cols - 1];
                    // research to-do list
                    let mut open: Vec<(u32, String, String)> = Vec::new();
                    for (id, p) in &self.db.people {
                        for t in p.todos.iter().filter(|t| !t.done) {
                            open.push((*id, t.text.clone(), t.added.clone()));
                        }
                    }
                    open.sort_by(|a, b| b.2.cmp(&a.2));
                    card(ui, crate::i18n::trf("Research to-do list ({})", &["Research to-do list (", ")"], &[open.len().to_string()]), &mut |ui| {
                        if open.is_empty() {
                            ui.label(RichText::new(tr("Nothing on the to-do list. Add one from a person's page, or send a line from the notepad (F10).")).weak());
                        }
                        for (id, text, _) in open.iter().take(8) {
                            ui.horizontal_wrapped(|ui| {
                                if ui.link(self.db.label(*id)).clicked() {
                                    *act = Some(Action::Select(*id));
                                }
                                ui.label(RichText::new(text).weak());
                            });
                        }
                        if ui.button(tr("Open the to-do list (F8)")).clicked() {
                            self.x.todo_open = true;
                        }
                    });
                    // what to look for next
                    let (ends, unsourced) = suggestions(&self.db, self.db.home);
                    card(ui, tr("What to look for next").to_string(), &mut |ui| {
                        if !ends.is_empty() {
                            ui.label(crate::i18n::trf("Ends of the line: {} ancestors have no parents entered yet", &["Ends of the line: ", " ancestors have no parents entered yet"], &[ends.len().to_string()]));
                            ui.horizontal_wrapped(|ui| {
                                for k in ends.iter().take(6) {
                                    if ui.link(self.db.label(*k)).clicked() {
                                        *act = Some(Action::Select(*k));
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        }
                        if !unsourced.is_empty() {
                            ui.label(crate::i18n::trf("{} people have dates but no sources", &["", " people have dates but no sources"], &[unsourced.len().to_string()]));
                            ui.horizontal_wrapped(|ui| {
                                for k in unsourced.iter().take(6) {
                                    if ui.link(self.db.label(*k)).clicked() {
                                        *act = Some(Action::Select(*k));
                                    }
                                }
                            });
                        }
                        if ends.is_empty() && unsourced.is_empty() {
                            ui.label(RichText::new(tr("Nothing stands out. Try the data checker (F7) for a closer look.")).weak());
                        }
                    });
                    // changes made today
                    let labels: Vec<(String, std::time::Instant)> = self.undo.iter().rev().take(6).map(|s| (s.label.clone(), s.at)).collect();
                    card(ui, tr("Changes since the program opened").to_string(), &mut |ui| {
                        if labels.is_empty() {
                            ui.label(RichText::new(tr("No changes yet.")).weak());
                        }
                        for (l, at) in &labels {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(when_ago(*at)).small().color(head));
                                ui.label(if l.is_empty() { tr("A change") } else { l.as_str() });
                            });
                        }
                    });
                    // backup
                    let age = self.last_backup_age(now);
                    let mut back_up = false;
                    card(ui, tr("Backup").to_string(), &mut |ui| {
                        let text = match age {
                            None => tr("No backup made yet.").to_string(),
                            Some(s) if s < 60 => tr("Last backup just now.").to_string(),
                            Some(s) if s < 3600 => crate::i18n::trf("Last backup {} minutes ago.", &["Last backup ", " minutes ago."], &[(s / 60).to_string()]),
                            Some(s) if s < 86400 * 2 => crate::i18n::trf("Last backup {} hours ago.", &["Last backup ", " hours ago."], &[(s / 3600).to_string()]),
                            Some(s) => crate::i18n::trf("Last backup {} days ago.", &["Last backup ", " days ago."], &[(s / 86400).to_string()]),
                        };
                        ui.label(text);
                        if ui.button(tr("Back up now")).clicked() {
                            back_up = true;
                        }
                    });
                    if back_up {
                        self.backup_now(true);
                        self.x.home_backup.0 = 0.0;
                    }
                }
            });
            ui.add_space(4.0);
            if ui.checkbox(&mut self.settings.start_home, tr("Open on this Home tab when the program starts")).changed() {
                save_settings(&self.settings);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anniversaries_and_suggestions() {
        let mut db = crate::sample::sample_family();
        let id = *db.people.keys().next().unwrap();
        db.people.get_mut(&id).unwrap().bd = "13 NOV 1843".into();
        let hits = on_this_day(&db, 13, 11);
        assert!(hits.iter().any(|a| a.who == id && a.what == "born" && a.year == Some(1843)));
        db.people.get_mut(&id).unwrap().bd = "ABT 13 NOV 1843".into();
        assert!(!on_this_day(&db, 13, 11).iter().any(|a| a.who == id));
        let (ends, _) = suggestions(&db, db.home.or(Some(id)));
        assert!(ends.iter().all(|e| db.people.contains_key(e)));
        let mut app = App::for_test(db);
        app.view = View::Home;
        let ctx = egui::Context::default();
        for w in [700.0, 1400.0] {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(w, 900.0))), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                let mut act = None;
                egui::CentralPanel::default().show(ctx, |ui| app.home_view(ui, &mut act));
            });
        }
    }
}
