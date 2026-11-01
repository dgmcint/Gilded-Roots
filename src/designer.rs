//! Reports > Report designer: choose who is in a report, which columns it has, how it is sorted
//! and titled; preview it, print it or save it as a spreadsheet; keep designs to use again.

use super::*;

pub const COLUMNS: &[(&str, &str)] = &[
    ("id", "ID"),
    ("name", "Name"),
    ("sex", "Sex"),
    ("born", "Born"),
    ("bplace", "Birthplace"),
    ("died", "Died"),
    ("dplace", "Place of death"),
    ("age", "Age at death"),
    ("father", "Father"),
    ("mother", "Mother"),
    ("spouses", "Spouses"),
    ("kids", "Children"),
    ("occupation", "Occupation"),
    ("buried", "Buried"),
    ("sources", "Sources"),
    ("confidence", "Doubtful facts"),
    ("tasks", "Open tasks"),
    ("notes", "Notes"),
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Design {
    pub name: String,
    pub title: String,
    /// 0 everyone, 1 ancestors, 2 descendants, 3 bookmarked, 4 one surname
    pub who: u8,
    pub surname: String,
    pub cols: Vec<String>,
    /// "name", "surname", "born", "id"
    pub sort: String,
    pub group_surname: bool,
    pub landscape: bool,
    pub hide_living: bool,
}

impl Default for Design {
    fn default() -> Self {
        Design {
            name: String::new(),
            title: "My report".into(),
            who: 0,
            surname: String::new(),
            cols: vec!["name".into(), "born".into(), "bplace".into(), "died".into(), "dplace".into()],
            sort: "surname".into(),
            group_surname: false,
            landscape: true,
            hide_living: true,
        }
    }
}

impl App {
    fn design_people(&self, d: &Design, db: &Db) -> Vec<u32> {
        let sel = self.sel;
        let mut v: Vec<u32> = match (d.who, sel) {
            (1, Some(s)) => {
                let mut out = vec![s];
                let mut i = 0;
                let mut seen = HashSet::from([s]);
                while i < out.len() {
                    let (f, m) = db.parents(out[i]);
                    for p in [f, m].into_iter().flatten() {
                        if seen.insert(p) {
                            out.push(p);
                        }
                    }
                    i += 1;
                }
                out
            }
            (2, Some(s)) => {
                let mut out = vec![s];
                let mut i = 0;
                let mut seen = HashSet::from([s]);
                while i < out.len() {
                    for c in db.children_of(out[i]) {
                        if seen.insert(c) {
                            out.push(c);
                        }
                    }
                    i += 1;
                }
                out
            }
            (3, _) => db.bookmarks.iter().copied().filter(|b| db.people.contains_key(b)).collect(),
            (4, _) => {
                let sn = d.surname.trim().to_lowercase();
                db.people.iter().filter(|(_, p)| p.surname.trim().to_lowercase() == sn).map(|(k, _)| *k).collect()
            }
            _ => db.people.keys().copied().collect(),
        };
        let key = |id: &u32| -> (String, i32, String) {
            let p = &db.people[id];
            (p.surname.to_lowercase(), parse_date(&p.bd).0.unwrap_or(i32::MAX), p.name.to_lowercase())
        };
        match d.sort.as_str() {
            "born" => v.sort_by_key(|id| (key(id).1, key(id).2)),
            "id" => v.sort(),
            "name" => v.sort_by_key(|id| key(id).2),
            _ => v.sort_by_key(|id| (key(id).0, key(id).2, key(id).1)),
        }
        v
    }

    fn cell(db: &Db, id: u32, col: &str) -> String {
        let p = &db.people[&id];
        let nm = |x: Option<u32>| x.and_then(|x| db.people.get(&x)).map(display_name).unwrap_or_default();
        match col {
            "id" => format!("#{}", id),
            "name" => display_name(p),
            "sex" => p.sex.clone(),
            "born" => print_date(&p.bd),
            "bplace" => p.bp.clone(),
            "died" => print_date(&p.dd),
            "dplace" => p.dp.clone(),
            "age" => match (parse_date(&p.bd).0, parse_date(&p.dd).0) {
                (Some(b), Some(d)) if d >= b => (d - b).to_string(),
                _ => String::new(),
            },
            "father" => nm(db.parents(id).0),
            "mother" => nm(db.parents(id).1),
            "spouses" => db.partners(id).into_iter().map(|s| nm(Some(s))).collect::<Vec<_>>().join("; "),
            "kids" => db.children_of(id).len().to_string(),
            "occupation" => p.events.iter().filter(|e| e.kind == "Occupation").map(|e| e.val.clone()).filter(|v| !v.is_empty()).collect::<Vec<_>>().join("; "),
            "buried" => [print_date(&p.bud), p.bup.clone()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "),
            "sources" => (p.sources.len() + p.events.iter().map(|e| e.cites.len()).sum::<usize>()).to_string(),
            "confidence" => {
                let mut v: Vec<String> = Vec::new();
                for (t, l) in [("BIRT", "birth"), ("CHR", "christening"), ("DEAT", "death"), ("BURI", "burial")] {
                    if let Some(c) = p.fx.get(t).map(|e| e.conf.as_str()).filter(|c| *c == "uncertain" || *c == "disputed") {
                        v.push(format!("{} {}", l, c));
                    }
                }
                for e in &p.events {
                    if e.conf == "uncertain" || e.conf == "disputed" {
                        v.push(format!("{} {}", e.kind.to_lowercase(), e.conf));
                    }
                }
                v.join("; ")
            }
            "tasks" => p.todos.iter().filter(|t| !t.done).map(|t| t.text.clone()).collect::<Vec<_>>().join("; "),
            "notes" => {
                let n: String = p.notes.trim().chars().take(300).collect();
                n
            }
            _ => String::new(),
        }
    }

    fn design_rows(&self, d: &Design) -> (Db, Vec<u32>) {
        let db = if d.hide_living { self.db.sanitized() } else { self.db.clone() };
        let ids = self.design_people(d, &db);
        (db, ids)
    }

    pub(crate) fn design_html(&self, d: &Design) -> String {
        let (db, ids) = self.design_rows(d);
        let cols: Vec<(&str, &str)> = COLUMNS.iter().copied().filter(|c| d.cols.iter().any(|x| x == c.0)).collect();
        let head: String = cols.iter().map(|c| format!("<th>{}</th>", c.1)).collect();
        let mut body = format!("<p class=\"sub\">{} people</p>", ids.len());
        let mut cur_group: Option<String> = None;
        body.push_str("<table>");
        body.push_str(&format!("<tr>{}</tr>", head));
        for id in &ids {
            if d.group_surname {
                let g = db.people[id].surname.trim().to_string();
                if cur_group.as_deref() != Some(g.as_str()) {
                    body.push_str(&format!("<tr><th colspan=\"{}\" style=\"background:#e2d8bb\">{}</th></tr>", cols.len(), esc(if g.is_empty() { "(no surname)" } else { &g })));
                    cur_group = Some(g);
                }
            }
            let row: String = cols.iter().map(|c| format!("<td>{}</td>", esc(&Self::cell(&db, *id, c.0)))).collect();
            body.push_str(&format!("<tr>{}</tr>", row));
        }
        body.push_str("</table>");
        page(if d.title.trim().is_empty() { "Report" } else { d.title.trim() }, &body, d.landscape)
    }

    pub(crate) fn design_csv(&self, d: &Design) -> String {
        let (db, ids) = self.design_rows(d);
        let cols: Vec<(&str, &str)> = COLUMNS.iter().copied().filter(|c| d.cols.iter().any(|x| x == c.0)).collect();
        let q = |s: &str| {
            let s = if s.starts_with(['=', '+', '-', '@']) { format!("'{}", s) } else { s.to_string() };
            format!("\"{}\"", s.replace('"', "\"\""))
        };
        let mut out = cols.iter().map(|c| q(c.1)).collect::<Vec<_>>().join(",");
        out.push('\n');
        for id in ids {
            out.push_str(&cols.iter().map(|c| q(&Self::cell(&db, id, c.0))).collect::<Vec<_>>().join(","));
            out.push('\n');
        }
        out
    }

    pub(crate) fn designer_window(&mut self, ctx: &egui::Context) {
        if !self.x.designer_open {
            return;
        }
        let mut open = true;
        let mut print = false;
        let mut csv = false;
        let mut d = self.x.design.clone();
        let saved = self.settings.reports.clone();
        let mut load: Option<Design> = None;
        let mut del: Option<usize> = None;
        let mut save_as = false;
        egui::Window::new("Report designer").open(&mut open).default_size([760.0, 620.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Saved designs:");
                if saved.is_empty() {
                    ui.label(RichText::new("none yet").weak());
                }
                for (i, s) in saved.iter().enumerate() {
                    if ui.button(s.name.as_str()).clicked() {
                        load = Some(s.clone());
                    }
                    if ui.small_button("x").on_hover_text("Forget this design").clicked() {
                        del = Some(i);
                    }
                }
            });
            ui.separator();
            egui::Grid::new("dz").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Title");
                ui.add(egui::TextEdit::singleline(&mut d.title).desired_width(360.0));
                ui.end_row();
                ui.label("Who");
                ui.horizontal_wrapped(|ui| {
                    for (v, l) in [(0u8, "Everyone"), (1, "Ancestors of the selected person"), (2, "Descendants of the selected person"), (3, "Bookmarked"), (4, "One surname")] {
                        ui.selectable_value(&mut d.who, v, l);
                    }
                    if d.who == 4 {
                        ui.add(egui::TextEdit::singleline(&mut d.surname).hint_text("surname").desired_width(120.0));
                    }
                });
                ui.end_row();
                ui.label("Columns");
                ui.horizontal_wrapped(|ui| {
                    for (k, l) in COLUMNS {
                        let mut on = d.cols.iter().any(|c| c == k);
                        if ui.checkbox(&mut on, *l).changed() {
                            if on {
                                d.cols.push(k.to_string());
                            } else {
                                d.cols.retain(|c| c != k);
                            }
                        }
                    }
                });
                ui.end_row();
                ui.label("Sort by");
                ui.horizontal(|ui| {
                    for (v, l) in [("surname", "Surname"), ("name", "Name"), ("born", "Birth date"), ("id", "ID")] {
                        if ui.selectable_label(d.sort == v, l).clicked() {
                            d.sort = v.to_string();
                        }
                    }
                    ui.checkbox(&mut d.group_surname, "Group by surname");
                });
                ui.end_row();
                ui.label("Page");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut d.landscape, true, "Landscape");
                    ui.selectable_value(&mut d.landscape, false, "Portrait");
                    ui.checkbox(&mut d.hide_living, "Keep living people private");
                });
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Print or save as PDF").strong()).clicked() {
                    print = true;
                }
                if ui.button("Save as spreadsheet (CSV)...").clicked() {
                    csv = true;
                }
                ui.separator();
                ui.add(egui::TextEdit::singleline(&mut d.name).hint_text("Name for this design").desired_width(160.0));
                if ui.add_enabled(!d.name.trim().is_empty(), egui::Button::new("Save design")).clicked() {
                    save_as = true;
                }
            });
            ui.separator();
            // preview of the first rows
            let (db, ids) = self.design_rows(&d);
            ui.label(RichText::new(format!("Preview: {} people", ids.len())).strong());
            let cols: Vec<(&str, &str)> = COLUMNS.iter().copied().filter(|c| d.cols.iter().any(|x| x == c.0)).collect();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("dprev").striped(true).num_columns(cols.len().max(1)).spacing([12.0, 4.0]).show(ui, |ui| {
                    for c in &cols {
                        ui.label(RichText::new(c.1).strong());
                    }
                    ui.end_row();
                    for id in ids.iter().take(40) {
                        for c in &cols {
                            ui.label(Self::cell(&db, *id, c.0));
                        }
                        ui.end_row();
                    }
                });
                if ids.len() > 40 {
                    ui.label(RichText::new(format!("... and {} more on the printed report", ids.len() - 40)).weak());
                }
            });
        });
        if let Some(l) = load {
            d = l;
        }
        if let Some(i) = del {
            self.settings.reports.remove(i);
            save_settings(&self.settings);
        }
        if save_as {
            let name = d.name.trim().to_string();
            self.settings.reports.retain(|r| r.name != name);
            self.settings.reports.push(Design { name, ..d.clone() });
            save_settings(&self.settings);
            self.status = "Report design saved.".into();
        }
        if print {
            let h = self.design_html(&d);
            self.open_print(h);
        }
        if csv {
            let text = self.design_csv(&d);
            if let Some(p) = rfd::FileDialog::new().set_file_name(format!("{}.csv", sanitize(&d.title))).add_filter("Spreadsheet (CSV)", &["csv"]).save_file() {
                self.status = match atomic_write(&p, text.as_bytes()) {
                    Ok(_) => format!("Saved {}", p.display()),
                    Err(e) => format!("Could not save: {}", e),
                };
            }
        }
        self.x.design = d;
        self.x.designer_open = open;
    }
}
