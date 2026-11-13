//! The People tab: everyone in one sortable, filterable table, like a spreadsheet.

use super::*;
use egui_extras::{Column, TableBuilder};

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Col {
    #[default]
    Name,
    Id,
    Sex,
    Born,
    BornIn,
    Died,
    DiedIn,
    Sources,
    Photos,
    Tasks,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Only {
    #[default]
    All,
    NoBirth,
    NoSources,
    OpenTasks,
    Living,
    Bookmarked,
    Doubtful,
    /// The people in a saved group (which one is `table_group`)
    Group,
}

pub struct Row {
    pub id: u32,
    pub name: String,
    pub sex: &'static str,
    pub born: String,
    pub born_in: String,
    pub died: String,
    pub died_in: String,
    pub sources: usize,
    pub photos: usize,
    pub tasks: usize,
    sort_born: i32,
    sort_died: i32,
    sort_name: (String, String),
}

/// What the People table's rows depend on, besides the tree itself.
pub(crate) type TableKey = (u64, String, Only, usize, Col, bool);

fn date_key(s: &str) -> i32 {
    let (y, m, d) = parse_date(s);
    match y {
        Some(y) => y * 10000 + m.unwrap_or(0) as i32 * 100 + d.unwrap_or(0) as i32,
        None => i32::MAX,
    }
}

impl App {
    pub(crate) fn table_rows(&self) -> Vec<Row> {
        let q = self.x.table_find.trim().to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();
        let mut rows: Vec<Row> = Vec::new();
        for (id, p) in &self.db.people {
            let cites = p.sources.len() + p.events.iter().map(|e| e.cites.len()).sum::<usize>() + p.fx.values().map(|e| e.cites.len()).sum::<usize>();
            let open_tasks = p.todos.iter().filter(|t| !t.done).count();
            let keep = match self.x.table_only {
                Only::All => true,
                Only::NoBirth => p.bd.trim().is_empty(),
                Only::NoSources => cites == 0,
                Only::OpenTasks => open_tasks > 0,
                Only::Living => p.living,
                Only::Bookmarked => self.db.bookmarks.contains(id),
                Only::Group => self.db.groups.get(self.x.table_group).map_or(false, |g| g.ids.contains(id)),
                Only::Doubtful => {
                    let weak = |c: &str| c == "uncertain" || c == "disputed";
                    p.fx.values().any(|e| weak(&e.conf)) || p.events.iter().any(|e| weak(&e.conf))
                }
            };
            if !keep {
                continue;
            }
            if !words.is_empty() {
                let hay = format!("{} {} {} {} {} {} #{}", p.name, p.maiden, p.bd, p.bp, p.dd, p.dp, id).to_lowercase();
                if !words.iter().all(|w| hay.contains(w)) {
                    continue;
                }
            }
            rows.push(Row {
                id: *id,
                name: styled_name(p),
                sex: match p.sex.as_str() {
                    "M" => "M",
                    "F" => "F",
                    _ => "?",
                },
                born: show_date(&p.bd),
                born_in: p.bp.trim().to_string(),
                died: show_date(&p.dd),
                died_in: p.dp.trim().to_string(),
                sources: cites,
                photos: p.media.len(),
                tasks: open_tasks,
                sort_born: date_key(&p.bd),
                sort_died: date_key(&p.dd),
                sort_name: (p.surname.to_lowercase(), p.name.to_lowercase()),
            });
        }
        let empty_last = |a: &str, b: &str| (a.is_empty(), a.to_lowercase()).cmp(&(b.is_empty(), b.to_lowercase()));
        rows.sort_by(|a, b| {
            let o = match self.x.table_col {
                Col::Name => a.sort_name.cmp(&b.sort_name),
                Col::Id => a.id.cmp(&b.id),
                Col::Sex => a.sex.cmp(b.sex),
                Col::Born => a.sort_born.cmp(&b.sort_born),
                Col::BornIn => empty_last(&a.born_in, &b.born_in),
                Col::Died => a.sort_died.cmp(&b.sort_died),
                Col::DiedIn => empty_last(&a.died_in, &b.died_in),
                Col::Sources => a.sources.cmp(&b.sources),
                Col::Photos => a.photos.cmp(&b.photos),
                Col::Tasks => a.tasks.cmp(&b.tasks),
            };
            if self.x.table_desc { o.reverse() } else { o }
        });
        rows
    }

    /// Everyone matching `filter`, in name order, kept until the tree changes.
    pub(crate) fn sorted_kept(&mut self, filter: &str) -> std::sync::Arc<Vec<u32>> {
        if let Some((v, ids)) = self.x.sorted_cache.get(filter) {
            if *v == self.x.data_ver {
                return ids.clone();
            }
        }
        if self.x.sorted_cache.len() > 8 {
            self.x.sorted_cache.clear();
        }
        let ids = std::sync::Arc::new(self.db.sorted(filter));
        self.x.sorted_cache.insert(filter.to_string(), (self.x.data_ver, ids.clone()));
        ids
    }

    /// The table's rows, kept until the tree or the filter changes.
    pub(crate) fn table_rows_kept(&mut self) -> std::sync::Arc<Vec<Row>> {
        let key: TableKey = (self.x.data_ver, self.x.table_find.clone(), self.x.table_only, self.x.table_group, self.x.table_col, self.x.table_desc);
        if let Some((k, rows)) = &self.x.table_cache {
            if *k == key {
                return rows.clone();
            }
        }
        let rows = std::sync::Arc::new(self.table_rows());
        self.x.table_cache = Some((key, rows.clone()));
        rows
    }

    pub(crate) fn table_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let rows = self.table_rows_kept();
        let mut export = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{} of {} people", rows.len(), self.db.people.len())).strong());
            ui.add(egui::TextEdit::singleline(&mut self.x.table_find).hint_text("Filter: name, year, place or #ID").desired_width(240.0));
            ui.label("Show");
            let group_label = self.db.groups.get(self.x.table_group).map(|g| format!("Group: {}", g.name)).unwrap_or_else(|| "Group".into());
            egui::ComboBox::from_id_salt("table_only")
                .selected_text(if self.x.table_only == Only::Group { group_label } else { match self.x.table_only {
                    Only::All => "Everyone",
                    Only::NoBirth => "No birth date",
                    Only::NoSources => "No sources",
                    Only::OpenTasks => "Open research tasks",
                    Only::Living => "Living",
                    Only::Bookmarked => "Bookmarked",
                    Only::Doubtful => "Uncertain or disputed facts",
                    Only::Group => "",
                }.to_string() })
                .show_ui(ui, |ui| {
                    for (v, l) in [(Only::All, "Everyone"), (Only::NoBirth, "No birth date"), (Only::NoSources, "No sources"), (Only::OpenTasks, "Open research tasks"), (Only::Living, "Living"), (Only::Bookmarked, "Bookmarked"), (Only::Doubtful, "Uncertain or disputed facts")] {
                        ui.selectable_value(&mut self.x.table_only, v, l);
                    }
                    for (i, g) in self.db.groups.iter().enumerate() {
                        if ui.selectable_label(self.x.table_only == Only::Group && self.x.table_group == i, format!("Group: {}", g.name)).clicked() {
                            self.x.table_only = Only::Group;
                            self.x.table_group = i;
                        }
                    }
                });
            if ui.button("Save as spreadsheet (CSV)...").on_hover_text("Opens in Excel, Numbers or Google Sheets").clicked() {
                export = true;
            }
        });
        ui.label(RichText::new("Click a column title to sort. Click a row to select, double-click to open the person.").small().weak());
        ui.separator();
        let mut sort_click: Option<Col> = None;
        let cols = [
            (Col::Id, "ID", 60.0),
            (Col::Name, "Name", 220.0),
            (Col::Sex, "Sex", 40.0),
            (Col::Born, "Born", 110.0),
            (Col::BornIn, "Birthplace", 200.0),
            (Col::Died, "Died", 110.0),
            (Col::DiedIn, "Place of death", 200.0),
            (Col::Sources, "Sources", 64.0),
            (Col::Photos, "Photos", 60.0),
            (Col::Tasks, "Tasks", 50.0),
        ];
        let sel = self.sel;
        let mut tb = TableBuilder::new(ui).striped(true).resizable(true).sense(Sense::click()).cell_layout(egui::Layout::left_to_right(egui::Align::Center));
        for (_, _, w) in &cols {
            tb = tb.column(Column::initial(*w).at_least(30.0).clip(true));
        }
        tb.header(24.0, |mut h| {
            for (c, l, _) in &cols {
                h.col(|ui| {
                    let arrow = if self.x.table_col == *c { if self.x.table_desc { " v" } else { " ^" } } else { "" };
                    if ui.add(egui::Button::new(RichText::new(format!("{}{}", l, arrow)).strong()).frame(false)).clicked() {
                        sort_click = Some(*c);
                    }
                });
            }
        })
        .body(|body| {
            body.rows(22.0, rows.len(), |mut row| {
                let r = &rows[row.index()];
                row.set_selected(sel == Some(r.id));
                row.col(|ui| {
                    ui.label(RichText::new(format!("#{}", r.id)).weak());
                });
                row.col(|ui| {
                    ui.label(r.name.as_str());
                });
                row.col(|ui| {
                    ui.label(r.sex);
                });
                row.col(|ui| {
                    ui.label(r.born.as_str());
                });
                row.col(|ui| {
                    ui.label(r.born_in.as_str());
                });
                row.col(|ui| {
                    ui.label(r.died.as_str());
                });
                row.col(|ui| {
                    ui.label(r.died_in.as_str());
                });
                for n in [r.sources, r.photos, r.tasks] {
                    row.col(|ui| {
                        ui.label(if n == 0 { RichText::new("-").weak() } else { RichText::new(n.to_string()) });
                    });
                }
                let resp = row.response();
                if resp.double_clicked() {
                    *act = Some(Action::Open(r.id));
                } else if resp.clicked() {
                    *act = Some(Action::Select(r.id));
                }
            });
        });
        if let Some(c) = sort_click {
            if self.x.table_col == c {
                self.x.table_desc = !self.x.table_desc;
            } else {
                self.x.table_col = c;
                self.x.table_desc = false;
            }
        }
        if export {
            let mut csv = String::from("ID,Name,Sex,Born,Birthplace,Died,Place of death,Sources,Photos,Open tasks\n");
            // a cell starting with = + - @ would run as a formula in a spreadsheet: make it plain text
            let q = |s: &str| {
                let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{}", s) } else { s.to_string() };
                format!("\"{}\"", s.replace('"', "\"\""))
            };
            for r in rows.iter() {
                csv.push_str(&format!("{},{},{},{},{},{},{},{},{},{}\n", r.id, q(&r.name), r.sex, q(&r.born), q(&r.born_in), q(&r.died), q(&r.died_in), r.sources, r.photos, r.tasks));
            }
            if let Some(p) = rfd::FileDialog::new().set_file_name("people.csv").add_filter("Spreadsheet (CSV)", &["csv"]).save_file() {
                self.status = match atomic_write(&p, csv.as_bytes()) {
                    Ok(_) => format!("Saved {} rows to {}", rows.len(), p.display()),
                    Err(e) => format!("Could not save: {}", e),
                };
            }
        }
    }
}
