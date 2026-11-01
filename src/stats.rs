//! Tools > Family statistics: lifespans, family sizes, marriage ages, where people were born and
//! moved, the most common names, and where each surname is found.

use super::*;

#[derive(Default, Clone)]
pub struct Stats {
    pub people: usize,
    pub families: usize,
    pub living: usize,
    pub with_birth: usize,
    pub with_sources: usize,
    /// (birth century, average age at death, how many)
    pub life_by_century: Vec<(i32, f32, usize)>,
    pub life_men: (f32, usize),
    pub life_women: (f32, usize),
    pub oldest: Option<(u32, i32)>,
    /// (decade the family began, average children, families)
    pub kids_by_decade: Vec<(i32, f32, usize)>,
    pub biggest: Option<(u32, usize)>,
    pub marry_men: (f32, usize),
    pub marry_women: (f32, usize),
    /// born somewhere other than the parents' birthplaces (state or country level)
    pub moved: (usize, usize),
    pub birth_regions: Vec<(String, usize)>,
    pub surnames: Vec<(String, usize, Vec<(String, usize)>)>,
    pub given_m: Vec<(String, usize)>,
    pub given_f: Vec<(String, usize)>,
}

/// The broad region of a place: the last part ("Oregon, USA" -> "USA"), with the state for
/// places in the United States, Canada and Australia ("Oregon, USA").
fn region(place: &str) -> String {
    let parts: Vec<&str> = place.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        n => {
            let last = parts[n - 1];
            let l = last.to_lowercase();
            if ["usa", "united states", "us", "canada", "australia"].contains(&l.as_str()) {
                format!("{}, {}", parts[n - 2], last)
            } else {
                last.to_string()
            }
        }
    }
}

fn top(m: HashMap<String, usize>, n: usize) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

pub fn compute(db: &Db) -> Stats {
    let mut s = Stats { people: db.people.len(), families: db.families_count(), ..Default::default() };
    let mut by_c: BTreeMap<i32, (f32, usize)> = BTreeMap::new();
    let mut regions: HashMap<String, usize> = HashMap::new();
    let mut sur: HashMap<String, (usize, HashMap<String, usize>)> = HashMap::new();
    let (mut gm, mut gf): (HashMap<String, usize>, HashMap<String, usize>) = (HashMap::new(), HashMap::new());
    for (id, p) in &db.people {
        if p.living {
            s.living += 1;
        }
        let (b, d) = (parse_date(&p.bd).0, parse_date(&p.dd).0);
        if b.is_some() {
            s.with_birth += 1;
        }
        if !p.sources.is_empty() || p.events.iter().any(|e| !e.cites.is_empty()) {
            s.with_sources += 1;
        }
        if let (Some(b), Some(d)) = (b, d) {
            let age = d - b;
            if (0..=120).contains(&age) {
                let e = by_c.entry(b / 100 * 100).or_insert((0.0, 0));
                e.0 += age as f32;
                e.1 += 1;
                let t = if p.sex == "F" { &mut s.life_women } else { &mut s.life_men };
                if p.sex == "M" || p.sex == "F" {
                    t.0 += age as f32;
                    t.1 += 1;
                }
                if s.oldest.map(|o| age > o.1).unwrap_or(true) {
                    s.oldest = Some((*id, age));
                }
            }
        }
        if !p.bp.trim().is_empty() {
            *regions.entry(region(&p.bp)).or_default() += 1;
        }
        let sn = p.surname.trim();
        if !sn.is_empty() {
            let e = sur.entry(sn.to_string()).or_default();
            e.0 += 1;
            if !p.bp.trim().is_empty() {
                *e.1.entry(region(&p.bp)).or_default() += 1;
            }
        }
        if let Some(g) = p.given.split_whitespace().next() {
            match p.sex.as_str() {
                "M" => *gm.entry(g.to_string()).or_default() += 1,
                "F" => *gf.entry(g.to_string()).or_default() += 1,
                _ => {}
            }
        }
        // moved: born somewhere other than both parents
        let (f, m) = db.parents(*id);
        if !p.bp.trim().is_empty() {
            let mine = region(&p.bp);
            let theirs: Vec<String> = [f, m].into_iter().flatten().filter_map(|x| db.people.get(&x)).map(|q| q.bp.trim()).filter(|b| !b.is_empty()).map(region).collect();
            if !theirs.is_empty() {
                s.moved.1 += 1;
                if !theirs.contains(&mine) {
                    s.moved.0 += 1;
                }
            }
        }
    }
    s.life_by_century = by_c.into_iter().map(|(c, (sum, n))| (c, sum / n as f32, n)).collect();
    s.life_men.0 /= s.life_men.1.max(1) as f32;
    s.life_women.0 /= s.life_women.1.max(1) as f32;
    let mut by_d: BTreeMap<i32, (f32, usize)> = BTreeMap::new();
    let (mut mm, mut mw) = ((0.0f32, 0usize), (0.0f32, 0usize));
    for (fid, f) in &db.fams {
        let n = f.kids.len();
        if s.biggest.map(|b| n > b.1).unwrap_or(n > 0) {
            s.biggest = Some((*fid, n));
        }
        let year = parse_date(&f.md).0.or_else(|| f.kids.iter().filter_map(|k| db.people.get(k)).filter_map(|k| parse_date(&k.bd).0).min());
        if let Some(y) = year {
            let e = by_d.entry(y / 10 * 10).or_insert((0.0, 0));
            e.0 += n as f32;
            e.1 += 1;
        }
        if let Some(my) = parse_date(&f.md).0 {
            for (who, acc) in [(f.h, &mut mm), (f.w, &mut mw)] {
                if let Some(b) = who.and_then(|w| db.people.get(&w)).and_then(|p| parse_date(&p.bd).0) {
                    let age = my - b;
                    if (12..=90).contains(&age) {
                        acc.0 += age as f32;
                        acc.1 += 1;
                    }
                }
            }
        }
    }
    s.kids_by_decade = by_d.into_iter().map(|(d, (sum, n))| (d, sum / n as f32, n)).collect();
    s.marry_men = (mm.0 / mm.1.max(1) as f32, mm.1);
    s.marry_women = (mw.0 / mw.1.max(1) as f32, mw.1);
    s.birth_regions = top(regions, 15);
    let mut sv: Vec<(String, usize, Vec<(String, usize)>)> = sur.into_iter().map(|(k, (n, m))| (k, n, top(m, 4))).collect();
    sv.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sv.truncate(30);
    s.surnames = sv;
    s.given_m = top(gm, 10);
    s.given_f = top(gf, 10);
    s
}

fn bar(ui: &mut egui::Ui, label: &str, value: f32, max: f32, text: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([150.0, 18.0], egui::Label::new(RichText::new(label).small()).truncate());
        let (r, _) = ui.allocate_exact_size(vec2(260.0, 14.0), Sense::hover());
        ui.painter().rect_filled(r, 3.0_f32, ui.visuals().faint_bg_color);
        let w = if max > 0.0 { (value / max).clamp(0.0, 1.0) * r.width() } else { 0.0 };
        ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(w, r.height())), 3.0_f32, ui.visuals().selection.bg_fill);
        ui.label(RichText::new(text).small());
    });
}

impl App {
    pub(crate) fn stats_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.stats_open {
            return;
        }
        let key = (self.db.people.len(), self.last_json.len());
        if self.x.stats.as_ref().map(|s| s.0) != Some(key) {
            self.x.stats = Some((key, compute(&self.db)));
        }
        let st = self.x.stats.as_ref().unwrap().1.clone();
        let mut open = true;
        let mut map_surname: Option<String> = None;
        egui::Window::new("Family statistics").open(&mut open).default_size([620.0, 640.0]).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new(format!("{} people in {} families. {} living. {} with a birth date, {} with at least one source.", st.people, st.families, st.living, st.with_birth, st.with_sources)).strong());
                ui.add_space(8.0);
                ui.label(RichText::new("How long people lived").size(16.0).strong());
                if st.life_men.1 + st.life_women.1 > 0 {
                    ui.label(format!("Men lived {:.0} years on average ({} with both dates); women {:.0} years ({}).", st.life_men.0, st.life_men.1, st.life_women.0, st.life_women.1));
                }
                let mx = st.life_by_century.iter().map(|x| x.1).fold(0.0, f32::max);
                for (c, avg, n) in &st.life_by_century {
                    bar(ui, &format!("Born in the {}s", c), *avg, mx, &format!("{:.0} years ({} people)", avg, n));
                }
                if let Some((id, age)) = st.oldest {
                    ui.horizontal(|ui| {
                        ui.label("Longest life:");
                        if ui.link(self.db.label(id)).clicked() {
                            *act = Some(Action::Open(id));
                        }
                        ui.label(format!("{} years", age));
                    });
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Family size").size(16.0).strong());
                let mx = st.kids_by_decade.iter().map(|x| x.1).fold(0.0, f32::max);
                for (d, avg, n) in &st.kids_by_decade {
                    bar(ui, &format!("Families from the {}s", d), *avg, mx, &format!("{:.1} children ({} families)", avg, n));
                }
                if let Some((fid, n)) = st.biggest {
                    let f = &self.db.fams[&fid];
                    let names: Vec<String> = [f.h, f.w].into_iter().flatten().filter_map(|x| self.db.people.get(&x)).map(|p| p.name.clone()).collect();
                    ui.label(format!("Largest family: {} with {} children.", names.join(" and "), n));
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Marriage").size(16.0).strong());
                ui.label(format!("Average age at marriage: men {:.0} ({}), women {:.0} ({}).", st.marry_men.0, st.marry_men.1, st.marry_women.0, st.marry_women.1));
                ui.add_space(8.0);
                ui.label(RichText::new("Where people were born, and moving").size(16.0).strong());
                if st.moved.1 > 0 {
                    ui.label(format!("{} of {} people ({:.0}%) were born in a different state or country from their parents.", st.moved.0, st.moved.1, 100.0 * st.moved.0 as f32 / st.moved.1 as f32));
                }
                let mx = st.birth_regions.first().map(|x| x.1 as f32).unwrap_or(1.0);
                for (r, n) in &st.birth_regions {
                    bar(ui, r, *n as f32, mx, &n.to_string());
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Surnames and where they are found").size(16.0).strong());
                egui::Grid::new("sur").num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                    for (name, n, places) in &st.surnames {
                        ui.label(RichText::new(format!("{} ({})", name, n)).strong());
                        ui.label(RichText::new(places.iter().map(|(p, k)| format!("{} {}", p, k)).collect::<Vec<_>>().join(", ")).small());
                        if ui.small_button("Show on map").clicked() {
                            map_surname = Some(name.clone());
                        }
                        ui.end_row();
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Most common first names").size(16.0).strong());
                ui.columns(2, |c| {
                    c[0].label(RichText::new("Men").strong());
                    for (g, n) in &st.given_m {
                        c[0].label(format!("{} ({})", g, n));
                    }
                    c[1].label(RichText::new("Women").strong());
                    for (g, n) in &st.given_f {
                        c[1].label(format!("{} ({})", g, n));
                    }
                });
            });
        });
        if let Some(s) = map_surname {
            self.x.map_surname = s;
            self.x.map_who = 3;
            self.x.map_fit = true;
            self.x.map_pick = None;
            self.view = View::Map;
        }
        self.x.stats_open = open;
    }
}
