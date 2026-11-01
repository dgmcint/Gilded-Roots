//! Y-DNA and mtDNA in the DNA tab. These tests follow one line only: Y-DNA passes from father to
//! son, mtDNA from a mother to all her children. Each test keeps a haplogroup, the test level,
//! the markers (Y-STR values) or mutations (mtDNA), a list of matches, and a view of that line
//! in the tree with a map of where each generation was born.

use super::*;
use dna::{first_number, parse_csv, LineMatch};

/// Read Y-STR markers pasted from a results page or a downloaded file. Accepts two rows (marker
/// names, then values), or pairs on one or many lines: "DYS393 13", "DYS393=13", "DYS393: 13".
/// Values can be multi-copy ("11-14", "15-16-17-17").
pub fn parse_markers(text: &str) -> Vec<(String, String)> {
    let is_marker = |w: &str| {
        let u = w.to_uppercase();
        (u.starts_with("DYS") || u.starts_with("DYF") || u.starts_with("Y-GATA") || u.starts_with("YGATA") || u.starts_with("GATA") || u.starts_with("CDY") || u.starts_with("YCA") || u.starts_with("Y-GGAAT") || u.starts_with("DXYS"))
            && u.len() >= 3
    };
    let is_value = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.' || c == ',') && w.chars().any(|c| c.is_ascii_digit());
    // two rows: names, then values
    let rows = parse_csv(text);
    if rows.len() >= 2 {
        let (a, b) = (&rows[0], &rows[1]);
        let names = a.iter().filter(|w| is_marker(w)).count();
        if names >= 3 && a.len() == b.len() {
            return a.iter().zip(b.iter()).filter(|(n, v)| is_marker(n) && is_value(v)).map(|(n, v)| (n.trim().to_string(), v.trim().to_string())).collect();
        }
    }
    // pairs anywhere in the text
    let cleaned: String = text.chars().map(|c| if c == '=' || c == ':' || c == ',' || c == ';' || c == '\t' || c == '|' { ' ' } else { c }).collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if is_marker(words[i]) && i + 1 < words.len() && is_value(words[i + 1]) {
            out.push((words[i].to_string(), words[i + 1].to_string()));
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

/// A plain-words guide to a Y-DNA genetic distance at a test level. Rough on purpose: the
/// number of mutations varies a lot from family to family.
pub fn ydna_guide(distance: u32, markers: u32) -> &'static str {
    let per37 = if markers == 0 { distance as f32 } else { distance as f32 * 37.0 / markers as f32 };
    if markers > 0 && markers < 25 {
        return "Too few markers to say much; a match at this level can be very distant.";
    }
    match per37 {
        x if x < 0.75 => "Very close: a shared paternal ancestor is likely within the last several generations.",
        x if x < 1.75 => "Close: likely related within the time of surnames and written records.",
        x if x < 3.25 => "Possibly related within genealogical times; check surnames and places.",
        _ => "Probably a more distant paternal relationship, before most records.",
    }
}

/// The same for mtDNA, by how much of the sequence was compared.
pub fn mtdna_guide(distance: u32, compared: &str) -> &'static str {
    let full = compared.to_lowercase().contains("full") || compared.to_lowercase().contains("fms") || compared.to_lowercase().contains("coding");
    match (full, distance) {
        (true, 0) => "Exact full-sequence match: the shared maternal ancestor may be within genealogical times, though often still several centuries back.",
        (true, 1..=2) => "Close full-sequence match: a shared maternal ancestor perhaps several centuries back.",
        (true, _) => "A more distant maternal relationship.",
        (false, 0) => "Exact match on part of the sequence only: the shared ancestor can be hundreds or thousands of years back. A full-sequence test tells more.",
        _ => "A distant maternal relationship.",
    }
}

/// Guess the columns of a Y-DNA or mtDNA match file: name, genetic distance, markers or level
/// compared, haplogroup, earliest known ancestor, notes.
pub fn guess_line_columns(headers: &[String]) -> [Option<usize>; 6] {
    let h: Vec<String> = headers.iter().map(|x| x.to_lowercase()).collect();
    let find = |t: &dyn Fn(&str) -> bool| h.iter().position(|x| t(x));
    let name = find(&|x| x == "name" || x.contains("full name") || x.contains("match name") || x == "display name")
        .or_else(|| find(&|x| x.contains("name") && !["first", "last", "middle", "sur", "ancestor", "kit", "email"].iter().any(|w| x.contains(w))));
    [
        name,
        find(&|x| x.contains("distance") || x == "gd"),
        find(&|x| x.contains("marker") || x.contains("level") || x.contains("compared") || x.contains("test")),
        find(&|x| x.contains("haplogroup") || x.contains("haplo")),
        find(&|x| x.contains("ancestor")),
        find(&|x| x.contains("note")),
    ]
}

pub fn rows_to_line_matches(rows: &[Vec<String>], cols: &[Option<usize>; 6]) -> Vec<LineMatch> {
    let cell = |r: &Vec<String>, i: Option<usize>| -> String { i.and_then(|i| r.get(i)).map(|s| s.trim().to_string()).unwrap_or_default() };
    rows.iter()
        .skip(1)
        .filter_map(|r| {
            let name = cell(r, cols[0]);
            if name.is_empty() {
                return None;
            }
            Some(LineMatch {
                name,
                distance: first_number(&cell(r, cols[1])).map(|n| n as u32),
                compared: cell(r, cols[2]),
                haplogroup: cell(r, cols[3]),
                ancestor: cell(r, cols[4]),
                link: None,
                notes: cell(r, cols[5]),
            })
        })
        .collect()
}

pub const LINE_FIELDS: [&str; 6] = ["Name", "Genetic distance", "Markers or level compared", "Haplogroup", "Earliest known ancestor", "Notes"];
pub const Y_LEVELS: &[&str] = &["Y-12", "Y-25", "Y-37", "Y-67", "Y-111", "Big Y-500", "Big Y-700", "SNP pack or single SNP", "Whole genome", "Other"];
pub const MT_LEVELS: &[&str] = &["HVR1", "HVR1 and HVR2", "Full sequence (mtFull)", "Haplogroup only (from an autosomal test)", "Other"];

/// The people on the tested line, the tester first: father, father's father... (Y-DNA) or
/// mother, mother's mother... (mtDNA). Stops at a gap or a loop.
pub fn direct_line(db: &Db, start: u32, paternal: bool) -> Vec<u32> {
    let mut v = vec![start];
    let mut seen: HashSet<u32> = HashSet::from([start]);
    let mut cur = start;
    loop {
        let (fa, mo) = db.parents(cur);
        let next = if paternal { fa } else { mo };
        match next {
            Some(n) if seen.insert(n) && db.people.contains_key(&n) => {
                v.push(n);
                cur = n;
            }
            _ => break,
        }
    }
    v
}

/// A match file being imported: its name, its rows, and which column holds each detail.
pub type LineImport = (String, Vec<Vec<String>>, [Option<usize>; 6]);

/// The line-test window state lives in DnaUi; these fields are extra for it.
#[derive(Default)]
pub struct LineUi {
    pub paste: Option<String>,
    pub edit: Option<(usize, LineMatch)>,
    pub link_q: String,
    pub import: Option<LineImport>,
    pub find: String,
    pub new_marker: (String, String),
}

impl App {
    pub(crate) fn line_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let si = self.x.dna.sel;
        let y = self.db.dna[si].kind == 1;
        ui.horizontal(|ui| {
            let tabs: [(u8, &str); 3] = if y { [(0, "Haplogroup and markers"), (1, "Matches"), (2, "Father's line")] } else { [(0, "Haplogroup and mutations"), (1, "Matches"), (2, "Mother's line")] };
            for (k, l) in tabs {
                if ui.selectable_label(self.x.dna.tab == k, l).clicked() {
                    self.x.dna.tab = k;
                }
            }
        });
        ui.separator();
        match self.x.dna.tab {
            0 => self.line_results(ui, y),
            1 => self.line_matches(ui, act, y),
            _ => self.line_map(ui, act, y),
        }
        self.line_windows(ui.ctx(), y);
    }

    fn line_results(&mut self, ui: &mut egui::Ui, y: bool) {
        let si = self.x.dna.sel;
        let mut changed = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let t = &mut self.db.dna[si].line;
            egui::Grid::new("line_top").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label("Haplogroup");
                changed |= ui.add(egui::TextEdit::singleline(&mut t.haplogroup).hint_text(if y { "e.g. R-M269 or R-FGC11134" } else { "e.g. H1a1" }).font(egui::TextStyle::Heading).desired_width(320.0)).changed();
                ui.end_row();
                ui.label("Test level");
                egui::ComboBox::from_id_salt("line_level").selected_text(if t.level.is_empty() { "Choose".to_string() } else { t.level.clone() }).width(220.0).show_ui(ui, |ui| {
                    for l in if y { Y_LEVELS } else { MT_LEVELS } {
                        if ui.selectable_label(t.level == *l, *l).clicked() {
                            t.level = l.to_string();
                            changed = true;
                        }
                    }
                });
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.label(RichText::new(if y {
                "A haplogroup is the branch of humanity's paternal family tree your line belongs to. The letter is the old, broad branch; the longer the name after it, the newer and more specific the branch."
            } else {
                "A haplogroup is the branch of humanity's maternal family tree your line belongs to. The letter is the old, broad branch; the letters and numbers after it narrow it down."
            }).small().weak());
            ui.separator();
            if y {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Y-STR markers ({})", t.markers.len())).strong());
                    if ui.button("Paste markers...").on_hover_text("Copy the marker table from the results page and paste it in").clicked() {
                        self.x.line.paste = Some(String::new());
                    }
                    if ui.button("Import markers file...").on_hover_text("The Y-STR results file downloaded from the testing company").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Y-STR results", &["csv", "txt", "tsv", "xlsx", "xls"]).set_directory(start_dir()).pick_file() {
                            remember_dir(&path);
                            // shown in the paste window first, so the markers can be checked before they replace the old ones
                            self.x.line.paste = Some(dna::read_table_text(&path).unwrap_or_default());
                        }
                    }
                });
                ui.add_space(4.0);
                // the markers in a grid of cards, ten to a row like most results pages
                let mut remove: Option<usize> = None;
                let w = ui.available_width();
                let per_row = ((w / 92.0).floor() as usize).clamp(4, 12);
                for (ci, chunk) in t.markers.chunks_mut(per_row).enumerate() {
                    ui.horizontal(|ui| {
                        for (j, (name, val)) in chunk.iter_mut().enumerate() {
                            ui.vertical(|ui| {
                                ui.set_width(84.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(name.as_str()).small().weak());
                                    if ui.add(egui::Label::new(RichText::new("x").small().weak()).sense(Sense::click())).on_hover_text("Remove").clicked() {
                                        remove = Some(ci * per_row + j);
                                    }
                                });
                                changed |= ui.add(egui::TextEdit::singleline(val).desired_width(70.0)).changed();
                            });
                        }
                    });
                }
                if let Some(i) = remove {
                    t.markers.remove(i);
                    changed = true;
                }
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.x.line.new_marker.0).hint_text("Marker, e.g. DYS393").desired_width(130.0));
                    ui.add(egui::TextEdit::singleline(&mut self.x.line.new_marker.1).hint_text("Value").desired_width(70.0));
                    if ui.add_enabled(!self.x.line.new_marker.0.trim().is_empty(), egui::Button::new("Add")).clicked() {
                        let (n, v) = std::mem::take(&mut self.x.line.new_marker);
                        t.markers.push((n.trim().to_uppercase(), v.trim().to_string()));
                        changed = true;
                    }
                });
            } else {
                ui.label(RichText::new("Mutations (differences from the reference sequence)").strong());
                ui.label(RichText::new("Paste them as your results page lists them, e.g. \"16129A 16519C 263G 315.1C\".").small().weak());
                changed |= ui.add(egui::TextEdit::multiline(&mut t.mutations).desired_rows(6).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace)).changed();
                let n = t.mutations.split(|c: char| c.is_whitespace() || c == ',').filter(|w| !w.is_empty()).count();
                if n > 0 {
                    ui.label(RichText::new(format!("{} mutation(s)", n)).weak());
                }
            }
        });
        if changed {
            self.x.next_label = Some(format!("Changed {} results", if y { "Y-DNA" } else { "mtDNA" }));
            self.dirty = true;
        }
    }

    fn line_matches(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, y: bool) {
        let si = self.x.dna.sel;
        ui.horizontal_wrapped(|ui| {
            if ui.button("+ Add a match").clicked() {
                self.x.line.edit = Some((usize::MAX, LineMatch::default()));
                self.x.line.link_q.clear();
            }
            if ui.button("Import a match list...").on_hover_text("The match file you can download from the testing company").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Match list (CSV, text or Excel)", &["csv", "txt", "tsv", "xlsx", "xls"]).set_directory(start_dir()).pick_file() {
                    remember_dir(&path);
                    match dna::read_table_text(&path) {
                        Ok(text) => {
                            let rows = parse_csv(&text);
                            if dna::is_raw_dna(&text) {
                                self.status = "That is a raw DNA data file (your own DNA readings), not a match list.".to_string();
                            } else if rows.len() < 2 {
                                self.status = "That file has no rows to import.".to_string();
                            } else {
                                let cols = guess_line_columns(&rows[0]);
                                self.x.line.import = Some((path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(), rows, cols));
                            }
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
            ui.separator();
            ui.add(egui::TextEdit::singleline(&mut self.x.line.find).hint_text("Filter by name, haplogroup or ancestor").desired_width(240.0));
        });
        dna::where_from_help(ui);
        let t = &self.db.dna[si].line;
        let q = self.x.line.find.to_lowercase();
        let mut idx: Vec<usize> = (0..t.matches.len())
            .filter(|&i| {
                let m = &t.matches[i];
                q.is_empty() || [&m.name, &m.haplogroup, &m.ancestor, &m.notes].iter().any(|s| s.to_lowercase().contains(&q))
            })
            .collect();
        idx.sort_by_key(|&i| (t.matches[i].distance.unwrap_or(u32::MAX), t.matches[i].name.to_lowercase()));
        ui.label(RichText::new(format!("{} of {} match(es), closest first", idx.len(), t.matches.len())).weak());
        let mut open_edit: Option<usize> = None;
        let mut go: Option<u32> = None;
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("line_grid").striped(true).num_columns(8).spacing([14.0, 5.0]).show(ui, |ui| {
                for h in ["Name", "Distance", if y { "Markers" } else { "Compared" }, "Haplogroup", "Earliest known ancestor", "What it suggests", "In my tree", ""] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for &i in &idx {
                    let m = &t.matches[i];
                    ui.label(m.name.as_str());
                    ui.label(m.distance.map(|d| d.to_string()).unwrap_or_default());
                    ui.label(m.compared.as_str());
                    ui.label(m.haplogroup.as_str());
                    ui.label(m.ancestor.as_str());
                    let guide = match m.distance {
                        Some(d) if y => ydna_guide(d, first_number(&m.compared).unwrap_or(0.0) as u32),
                        Some(d) => mtdna_guide(d, &m.compared),
                        None => "",
                    };
                    ui.add_sized([300.0, 18.0], egui::Label::new(RichText::new(guide).small().weak()).truncate()).on_hover_text(guide);
                    match m.link.filter(|p| self.db.people.contains_key(p)) {
                        Some(p) => {
                            if ui.link(display_name(&self.db.people[&p])).clicked() {
                                go = Some(p);
                            }
                        }
                        None => {
                            ui.label("");
                        }
                    }
                    if ui.small_button("Edit").clicked() {
                        open_edit = Some(i);
                    }
                    ui.end_row();
                }
            });
            if t.matches.is_empty() {
                ui.add_space(10.0);
                ui.label(RichText::new(if y {
                    "No matches yet. Y-DNA matches share your direct paternal line, and often your surname. The genetic distance is how many marker steps you differ by: the smaller it is, at the more markers, the closer the relationship."
                } else {
                    "No matches yet. mtDNA matches share your direct maternal line. Because mtDNA changes very slowly, even exact matches can be related many generations back; a full-sequence test narrows that down."
                }).weak());
            }
        });
        if let Some(i) = open_edit {
            self.x.line.edit = Some((i, self.db.dna[si].line.matches[i].clone()));
            self.x.line.link_q.clear();
        }
        if let Some(p) = go {
            *act = Some(Action::Open(p));
        }
    }

    /// The tested line through the tree: a list of the generations and a map of where each was
    /// born, joined in order from the oldest ancestor to the tester.
    fn line_map(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, y: bool) {
        let si = self.x.dna.sel;
        let Some(start) = self.db.dna[si].person.filter(|p| self.db.people.contains_key(p)) else {
            ui.label("Choose who took the test (Test details...) to see their line.");
            return;
        };
        let line = direct_line(&self.db, start, y);
        let hap = self.db.dna[si].line.haplogroup.clone();
        let pts: Vec<(u32, Option<[f32; 2]>, String, String)> = line
            .iter()
            .map(|id| {
                let p = &self.db.people[id];
                let (place, date) = if !p.bp.trim().is_empty() { (p.bp.clone(), p.bd.clone()) } else if !p.chp.trim().is_empty() { (p.chp.clone(), p.chd.clone()) } else { (String::new(), String::new()) };
                let ll = if place.trim().is_empty() { None } else { self.locate_place(&place).map(|(ll, _)| ll) };
                (*id, ll, place, date)
            })
            .collect();
        let mut go: Option<u32> = None;
        egui::SidePanel::left("line_list").resizable(true).default_width(320.0).show_inside(ui, |ui| {
            ui.label(RichText::new(if y { "Father's father's line" } else { "Mother's mother's line" }).size(16.0).strong());
            if !hap.trim().is_empty() {
                ui.label(format!("Haplogroup {}", hap.trim()));
            }
            ui.label(RichText::new(format!("{} generation(s) in the tree", line.len())).weak());
            ui.add_space(6.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (n, (id, ll, place, date)) in pts.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 10.0, gen_colour(n, pts.len()));
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, (n + 1).to_string(), FontId::proportional(11.0), Color32::WHITE);
                        ui.vertical(|ui| {
                            if ui.link(self.db.label(*id)).clicked() {
                                go = Some(*id);
                            }
                            let w = if place.is_empty() { "Birthplace not recorded".to_string() } else { format!("{} {}", place, show_date(date)).trim().to_string() };
                            ui.label(RichText::new(w).small().weak());
                            if !place.is_empty() && ll.is_none() {
                                ui.label(RichText::new("Not found on the map").small().weak());
                            }
                        });
                    });
                }
                ui.add_space(8.0);
                let last = &self.db.people[line.last().unwrap()];
                ui.label(RichText::new(format!(
                    "The line stops at {} because no {} is recorded. Adding them extends the line.",
                    display_name(last),
                    if y { "father" } else { "mother" }
                ))
                .small()
                .weak());
            });
        });
        // the map
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let placed: Vec<(usize, [f32; 2])> = pts.iter().enumerate().filter_map(|(i, p)| p.1.map(|ll| (i, ll))).collect();
        let (mut lo, mut hi) = ([180.0f32, 85.0f32], [-180.0f32, -85.0f32]);
        for (_, ll) in &placed {
            lo = [lo[0].min(ll[0]), lo[1].min(ll[1])];
            hi = [hi[0].max(ll[0]), hi[1].max(ll[1])];
        }
        if placed.is_empty() {
            lo = [-160.0, -45.0];
            hi = [160.0, 70.0];
        }
        // always show at least a region (about 50 by 25 degrees), so a family that stayed in one
        // area is shown among its neighbouring states and countries
        let pad_lon = ((hi[0] - lo[0]) * 0.3).max((50.0 - (hi[0] - lo[0])) / 2.0).max(4.0);
        let pad_lat = ((hi[1] - lo[1]) * 0.3).max((25.0 - (hi[1] - lo[1])) / 2.0).max(3.0);
        let (lo, hi) = ([lo[0] - pad_lon, (lo[1] - pad_lat).max(-70.0)], [hi[0] + pad_lon, (hi[1] + pad_lat).min(80.0)]);
        let (m0, m1) = (mapview::merc_lat(lo[1]), mapview::merc_lat(hi[1]));
        let k = (rect.width() / (hi[0] - lo[0])).min(rect.height() / (m1 - m0).max(0.5));
        let c = [(lo[0] + hi[0]) / 2.0, (m0 + m1) / 2.0];
        let painter = ui.painter_at(rect);
        mapview::paint_world(&painter, rect, c, k, ui.visuals().dark_mode);
        // the path, oldest ancestor to the tester, with an arrow on each step
        let mut ordered: Vec<(usize, egui::Pos2)> = placed.iter().map(|(i, ll)| (*i, mapview::world_point(rect, c, k, ll[0], ll[1]))).collect();
        ordered.sort_by_key(|x| std::cmp::Reverse(x.0));
        for w in ordered.windows(2) {
            let (a, b) = (w[0].1, w[1].1);
            if a.distance(b) < 2.0 {
                continue;
            }
            let col = gen_colour(w[1].0, pts.len());
            // a gentle curve, like a flight path
            let mid = pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let n = vec2(-(b.y - a.y), b.x - a.x).normalized() * (a.distance(b) * 0.15);
            let ctrl = mid + n;
            let pts_curve: Vec<egui::Pos2> = (0..=24).map(|s| {
                let t = s as f32 / 24.0;
                let u = 1.0 - t;
                pos2(u * u * a.x + 2.0 * u * t * ctrl.x + t * t * b.x, u * u * a.y + 2.0 * u * t * ctrl.y + t * t * b.y)
            }).collect();
            painter.add(egui::Shape::line(pts_curve.clone(), Stroke::new(2.0_f32, col.gamma_multiply(0.85))));
            // arrow head near the end
            let tip = pts_curve[21];
            let back = pts_curve[18];
            let dir = (tip - back).normalized();
            let side = vec2(-dir.y, dir.x);
            painter.add(egui::Shape::convex_polygon(vec![tip + dir * 5.0, tip - dir * 4.0 + side * 4.5, tip - dir * 4.0 - side * 4.5], col, Stroke::NONE));
        }
        let mut hover: Option<String> = None;
        let mut used: HashMap<(i32, i32), usize> = HashMap::new();
        for (i, p) in &ordered {
            // generations born in the same place fan out a little so each number shows
            let key = ((p.x / 6.0) as i32, (p.y / 6.0) as i32);
            let shift = used.entry(key).or_insert(0);
            let q = *p + vec2(*shift as f32 * 14.0, 0.0);
            *shift += 1;
            let col = gen_colour(*i, pts.len());
            painter.circle_filled(q, 15.0, col.gamma_multiply(0.18));
            painter.circle(q, 9.0, col, Stroke::new(1.2_f32, Color32::from_white_alpha(210)));
            painter.text(q, Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(10.5), Color32::WHITE);
            if resp.hover_pos().is_some_and(|m| m.distance(q) <= 10.0) {
                let (id, _, place, date) = &pts[*i];
                hover = Some(format!("{}. {}\nBorn {} {}", i + 1, self.db.label(*id), place, show_date(date)));
            }
        }
        if placed.is_empty() {
            painter.text(rect.center(), Align2::CENTER_CENTER, "No birthplaces on this line could be found on the map yet.", FontId::proportional(14.0), Color32::from_white_alpha(200));
        }
        painter.text(rect.left_bottom() + vec2(10.0, -10.0), Align2::LEFT_BOTTOM, "1 = the tester. Arrows run from the oldest ancestor down to the tester.", FontId::proportional(11.0), Color32::from_white_alpha(170));
        if let Some(h) = hover {
            resp.on_hover_text(h);
        }
        if let Some(p) = go {
            *act = Some(Action::Open(p));
        }
    }

    fn line_windows(&mut self, ctx: &egui::Context, y: bool) {
        let si = self.x.dna.sel;
        if let Some(mut text) = self.x.line.paste.take() {
            let mut open = true;
            let mut keep = true;
            egui::Window::new("Paste Y-DNA markers").open(&mut open).default_width(520.0).show(ctx, |ui| {
                ui.label("Copy the marker table from your results page and paste it here. Two rows (marker names, then values) or pairs such as \"DYS393 13\" both work.");
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut text).desired_rows(8).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                });
                let found = parse_markers(&text);
                ui.label(format!("Found {} marker(s)", found.len()));
                ui.label(RichText::new(found.iter().take(14).map(|(n, v)| format!("{} {}", n, v)).collect::<Vec<_>>().join(" \u{00B7} ")).small());
                ui.horizontal(|ui| {
                    if ui.add_enabled(!found.is_empty(), egui::Button::new("Replace the markers with these")).clicked() {
                        self.db.dna[si].line.markers = found.clone();
                        self.x.next_label = Some("Pasted Y-DNA markers".to_string());
                        self.dirty = true;
                        keep = false;
                    }
                });
            });
            if open && keep {
                self.x.line.paste = Some(text);
            }
        }
        if let Some((i, mut m)) = self.x.line.edit.take() {
            let mut open = true;
            let (mut done, mut delete) = (false, false);
            egui::Window::new(if i == usize::MAX { format!("Add a {} match", if y { "Y-DNA" } else { "mtDNA" }) } else { format!("{} match", if y { "Y-DNA" } else { "mtDNA" }) }).open(&mut open).resizable(false).show(ctx, |ui| {
                egui::Grid::new("line_edit").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(260.0));
                    ui.end_row();
                    ui.label("Genetic distance");
                    let mut d = m.distance.map(|x| x.to_string()).unwrap_or_default();
                    ui.horizontal(|ui| {
                        if ui.add(egui::TextEdit::singleline(&mut d).hint_text("e.g. 1").desired_width(60.0)).changed() {
                            m.distance = first_number(&d).map(|x| x as u32);
                        }
                        if let Some(dd) = m.distance {
                            let g = if y { ydna_guide(dd, first_number(&m.compared).unwrap_or(0.0) as u32) } else { mtdna_guide(dd, &m.compared) };
                            ui.add(egui::Label::new(RichText::new(g).small().weak()).wrap());
                        }
                    });
                    ui.end_row();
                    ui.label(if y { "Markers compared" } else { "Compared" });
                    egui::ComboBox::from_id_salt("line_cmp").selected_text(if m.compared.is_empty() { "Choose".to_string() } else { m.compared.clone() }).show_ui(ui, |ui| {
                        let opts: &[&str] = if y { &["12", "25", "37", "67", "111", "Big Y"] } else { &["HVR1", "HVR1 and HVR2", "Full sequence"] };
                        for o in opts {
                            if ui.selectable_label(m.compared == *o, *o).clicked() {
                                m.compared = o.to_string();
                            }
                        }
                    });
                    ui.end_row();
                    ui.label("Haplogroup");
                    ui.add(egui::TextEdit::singleline(&mut m.haplogroup).desired_width(260.0));
                    ui.end_row();
                    ui.label("Earliest known ancestor");
                    ui.add(egui::TextEdit::singleline(&mut m.ancestor).hint_text("e.g. John Walker, b. 1702 Ayrshire").desired_width(260.0));
                    ui.end_row();
                    ui.label("In my tree");
                    let cur = m.link.filter(|p| self.db.people.contains_key(p)).map(|p| self.db.label(p)).unwrap_or_else(|| "Not linked".to_string());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(cur).strong());
                        if m.link.is_some() && ui.small_button("Unlink").clicked() {
                            m.link = None;
                        }
                    });
                    ui.end_row();
                    ui.label("");
                    ui.add(egui::TextEdit::singleline(&mut self.x.line.link_q).hint_text("Type a name to link this match").desired_width(260.0));
                    ui.end_row();
                    let q = self.x.line.link_q.trim().to_string();
                    if !q.is_empty() {
                        for id in self.db.sorted(&q).into_iter().take(6) {
                            ui.label("");
                            if ui.button(self.db.label(id)).clicked() {
                                m.link = Some(id);
                                self.x.line.link_q.clear();
                            }
                            ui.end_row();
                        }
                    }
                    ui.label("Notes");
                    ui.add(egui::TextEdit::multiline(&mut m.notes).desired_rows(3).desired_width(260.0));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.add_enabled(!m.name.trim().is_empty(), egui::Button::new("Save")).clicked() {
                        done = true;
                    }
                    if i != usize::MAX && ui.button("Delete this match").clicked() {
                        delete = true;
                    }
                });
            });
            let list = &mut self.db.dna[si].line.matches;
            if done {
                if i == usize::MAX {
                    list.push(m);
                } else if i < list.len() {
                    list[i] = m;
                }
                self.x.next_label = Some("Saved a DNA match".to_string());
                self.dirty = true;
            } else if delete {
                if i < list.len() {
                    list.remove(i);
                }
                self.x.next_label = Some("Deleted a DNA match".to_string());
                self.dirty = true;
            } else if open {
                self.x.line.edit = Some((i, m));
            }
        }
        if let Some((file, rows, mut cols)) = self.x.line.import.take() {
            let mut open = true;
            let mut add: Option<Vec<LineMatch>> = None;
            egui::Window::new("Import matches").open(&mut open).default_width(560.0).show(ctx, |ui| {
                ui.label(format!("{}: {} row(s)", file, rows.len().saturating_sub(1)));
                let headers = rows[0].clone();
                egui::Grid::new("line_imp").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                    for (fi, f) in LINE_FIELDS.iter().enumerate() {
                        ui.label(*f);
                        let shown = cols[fi].and_then(|c| headers.get(c)).cloned().unwrap_or_else(|| "(not in this file)".to_string());
                        egui::ComboBox::from_id_salt(("line_imp_col", fi)).selected_text(shown).width(240.0).show_ui(ui, |ui| {
                            if ui.selectable_label(cols[fi].is_none(), "(not in this file)").clicked() {
                                cols[fi] = None;
                            }
                            for (ci, h) in headers.iter().enumerate() {
                                if ui.selectable_label(cols[fi] == Some(ci), h.as_str()).clicked() {
                                    cols[fi] = Some(ci);
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
                let mut found = rows_to_line_matches(&rows, &cols);
                let level = self.db.dna[si].line.level.clone();
                if cols[2].is_none() && !level.is_empty() {
                    let lv = level.trim_start_matches("Y-").to_string();
                    for m in found.iter_mut() {
                        m.compared = lv.clone();
                    }
                    ui.label(RichText::new(format!("The file has no markers column, so each match is marked as compared at your test level ({}).", level)).small().weak());
                }
                let have: HashSet<String> = self.db.dna[si].line.matches.iter().map(|m| m.name.to_lowercase()).collect();
                let new: Vec<LineMatch> = found.into_iter().filter(|m| !have.contains(&m.name.to_lowercase())).collect();
                for m in new.iter().take(5) {
                    ui.label(RichText::new(format!("{} \u{00B7} distance {} \u{00B7} {} \u{00B7} {}", m.name, m.distance.map(|d| d.to_string()).unwrap_or_default(), m.haplogroup, m.ancestor)).small());
                }
                if ui.add_enabled(!new.is_empty(), egui::Button::new(format!("Add {} match(es)", new.len()))).clicked() {
                    add = Some(new);
                }
            });
            if let Some(list) = add {
                let n = list.len();
                self.db.dna[si].line.matches.extend(list);
                self.x.next_label = Some(format!("Imported {} DNA match(es)", n));
                self.dirty = true;
            } else if open {
                self.x.line.import = Some((file, rows, cols));
            }
        }
    }
}

/// Colours along the line: the oldest generation warm gold, the tester cool blue.
fn gen_colour(i: usize, n: usize) -> Color32 {
    let t = if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 };
    let a = [70.0, 140.0, 220.0];
    let b = [226.0, 170.0, 70.0];
    Color32::from_rgb((a[0] + (b[0] - a[0]) * t) as u8, (a[1] + (b[1] - a[1]) * t) as u8, (a[2] + (b[2] - a[2]) * t) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dna::{DnaTest, LineTest};

    #[test]
    fn markers_from_two_rows_or_pairs() {
        let m = parse_markers("DYS393,DYS390,DYS19,DYS391,DYS385\n13,24,14,11,11-14\n");
        assert_eq!(m.len(), 5);
        assert_eq!(m[4], ("DYS385".to_string(), "11-14".to_string()));
        let m = parse_markers("DYS393 13  DYS390=24\nDYS19: 14\tCDY 36-37\nnot a marker 5");
        assert_eq!(m.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), vec!["DYS393", "DYS390", "DYS19", "CDY"]);
        assert_eq!(m[3].1, "36-37");
    }

    #[test]
    fn guides_read_sensibly() {
        assert!(ydna_guide(0, 37).starts_with("Very close"));
        assert!(ydna_guide(2, 111).starts_with("Very close") || ydna_guide(2, 111).starts_with("Close"));
        assert!(ydna_guide(5, 37).starts_with("Probably"));
        assert!(ydna_guide(0, 12).starts_with("Too few"));
        assert!(mtdna_guide(0, "Full sequence").starts_with("Exact full-sequence"));
        assert!(mtdna_guide(0, "HVR1").contains("part of the sequence"));
    }

    #[test]
    fn match_files_for_line_tests() {
        let rows = parse_csv("Full Name,Genetic Distance,Markers,Haplogroup,Earliest Known Ancestor\nAl Walker,1,37,R-M269,John Walker b.1702\n,2,37,,\n");
        let cols = guess_line_columns(&rows[0]);
        let got = rows_to_line_matches(&rows, &cols);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].distance, got[0].compared.as_str(), got[0].haplogroup.as_str(), got[0].ancestor.as_str()), (Some(1), "37", "R-M269", "John Walker b.1702"));
    }

    #[test]
    fn the_line_through_the_tree() {
        let db = sample();
        // someone in the sample with a father and a father's father
        let start = db.people.keys().copied().max_by_key(|id| direct_line(&db, *id, true).len()).unwrap();
        let line = direct_line(&db, start, true);
        assert!(line.len() >= 3, "sample has three generations of fathers");
        for w in line.windows(2) {
            assert_eq!(db.parents(w[0]).0, Some(w[1]));
        }
        let mline = direct_line(&db, start, false);
        for w in mline.windows(2) {
            assert_eq!(db.parents(w[0]).1, Some(w[1]));
        }
    }

    #[test]
    fn line_tests_draw() {
        let mut app = App::for_test(sample());
        let me = app.db.people.keys().copied().max_by_key(|id| direct_line(&app.db, *id, true).len()).unwrap();
        for kind in [1u8, 2] {
            app.db.dna.push(DnaTest {
                person: Some(me),
                kind,
                line: LineTest { haplogroup: "R-M269".into(), level: "Y-37".into(), markers: parse_markers("DYS393 13 DYS390 24"), mutations: "16519C 263G".into(), matches: vec![LineMatch { name: "Al".into(), distance: Some(1), compared: "37".into(), ..Default::default() }] },
                ..Default::default()
            });
        }
        let ctx = egui::Context::default();
        for sel in [0usize, 1] {
            for tab in 0..3u8 {
                app.x.dna.sel = sel;
                app.x.dna.tab = tab;
                app.x.line.paste = if tab == 0 { Some("DYS19 14".into()) } else { None };
                app.x.line.edit = if tab == 1 { Some((0, app.db.dna[sel].line.matches[0].clone())) } else { None };
                let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0))), ..Default::default() };
                let _ = ctx.run(input, |ctx| {
                    let mut act = None;
                    egui::CentralPanel::default().show(ctx, |ui| app.dna_view(ui, &mut act));
                });
            }
        }
        assert_eq!(app.db.dna.len(), 2);
    }
}
