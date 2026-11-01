//! The Charts tab: fan chart, hourglass chart and lifespan timeline.
//! Drawn straight onto the screen with the same colours as the other charts; every chart can be
//! printed (or saved as PDF) through the browser, like the pedigree chart.

use super::*;
use std::f32::consts::PI;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ChartKind {
    #[default]
    Fan,
    Hourglass,
    Lifespans,
}

/// Fan colours: father's side cool, mother's side warm, deeper further back.
fn fan_fill(dark: bool, paternal: bool, g: usize) -> Color32 {
    let t = (g as f32 / 8.0).min(1.0);
    let (a, b) = if paternal { ([74, 110, 160], [40, 70, 120]) } else { ([170, 92, 110], [120, 52, 70]) };
    let mix = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8;
    let c = Color32::from_rgb(mix(0), mix(1), mix(2));
    if dark {
        c
    } else {
        // lighter for paper-like light mode
        Color32::from_rgb(255 - (255 - c.r()) / 2, 255 - (255 - c.g()) / 2, 255 - (255 - c.b()) / 2)
    }
}

/// Ancestors by Ahnentafel number: 1 = the person, 2 = father, 3 = mother, 4 = father's father...
pub fn ahnen(db: &Db, root: u32, gens: usize) -> Vec<Option<u32>> {
    let n = (1usize << gens) - 1;
    let mut v: Vec<Option<u32>> = vec![None; n + 1];
    v[1] = Some(root);
    for k in 1..=n {
        if 2 * k + 1 > n {
            break;
        }
        if let Some(id) = v[k] {
            let (f, m) = db.parents(id);
            v[2 * k] = f;
            v[2 * k + 1] = m;
        }
    }
    v
}

fn years(p: &Person) -> (Option<i32>, Option<i32>) {
    (parse_date(&p.bd).0, parse_date(&p.dd).0)
}

impl App {
    pub(crate) fn charts_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        self.nav_bar(ui, act);
        ui.horizontal(|ui| {
            for (k, l) in [(ChartKind::Fan, "Fan chart"), (ChartKind::Hourglass, "Hourglass"), (ChartKind::Lifespans, "Lifespans")] {
                if ui.selectable_label(self.x.chart == k, RichText::new(l).size(15.0)).clicked() {
                    self.x.chart = k;
                }
            }
            ui.separator();
            if self.x.chart != ChartKind::Lifespans {
                ui.label("Generations:");
                let g = self.x.fan_gens.clamp(3, 8);
                if ui.add_enabled(g > 3, egui::Button::new("-")).clicked() {
                    self.x.fan_gens = g - 1;
                }
                ui.label(g.to_string());
                if ui.add_enabled(g < 8, egui::Button::new("+")).clicked() {
                    self.x.fan_gens = g + 1;
                }
            }
            if self.x.chart == ChartKind::Fan {
                ui.separator();
                for (deg, l) in [(180.0, "Half"), (240.0, "240\u{b0}"), (360.0, "Full circle")] {
                    if ui.selectable_label((self.x.fan_deg - deg).abs() < 1.0, l).clicked() {
                        self.x.fan_deg = deg;
                    }
                }
            }
            ui.separator();
            if ui.button("Print or save as PDF...").clicked() {
                *act = Some(Action::PrintChart);
            }
        });
        if self.x.fan_gens == 0 {
            self.x.fan_gens = 5;
        }
        if self.x.fan_deg == 0.0 {
            self.x.fan_deg = 240.0;
        }
        ui.separator();
        match self.x.chart {
            ChartKind::Fan => self.fan_chart(ui, act),
            ChartKind::Hourglass => self.hourglass(ui, act),
            ChartKind::Lifespans => self.lifespans(ui, act),
        }
    }

    /// Angles (radians, screen coordinates: 0 = right, growing clockwise) of slot `i` of ring `g`.
    fn fan_span(&self, g: usize, i: usize) -> (f32, f32) {
        let total = self.x.fan_deg.to_radians();
        let start = -PI / 2.0 - total / 2.0; // centred on straight up
        let n = (1usize << g) as f32;
        (start + total * i as f32 / n, start + total * (i as f32 + 1.0) / n)
    }

    fn fan_chart(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let Some(root) = self.sel else { return };
        let gens = self.x.fan_gens.clamp(3, 8);
        let anc = ahnen(&self.db, root, gens);
        let avail = ui.available_size();
        let (resp, painter) = ui.allocate_painter(avail, Sense::click());
        let rect = resp.rect;
        let full = self.x.fan_deg >= 359.0;
        let half = self.x.fan_deg <= 181.0;
        // the fan's size: fit the width and the height it needs
        let r_w = rect.width() / 2.0 - 10.0;
        let r_h = if full { rect.height() / 2.0 - 10.0 } else if half { rect.height() - 30.0 } else { (rect.height() - 20.0) / 1.55 };
        let r_max = r_w.min(r_h).max(120.0);
        let centre = if full { rect.center() } else if half { pos2(rect.center().x, rect.max.y - 20.0) } else { pos2(rect.center().x, rect.min.y + 10.0 + r_max) };
        let r0 = r_max * 0.16; // centre circle
        let ring = (r_max - r0) / (gens - 1) as f32;
        let dark = ui.visuals().dark_mode;
        let (_, edge, _, _, ink_c, head_c, accent) = self.chart_colors(ui);
        let ink = if dark { Color32::from_rgb(240, 240, 240) } else { Color32::from_rgb(25, 28, 32) };
        let pointer = resp.hover_pos();
        let mut hovered: Option<(usize, usize)> = None; // (g, i)
        if let Some(p) = pointer {
            let d = p - centre;
            let r = d.length();
            if r <= r0 {
                hovered = Some((0, 0));
            } else if r < r0 + ring * (gens - 1) as f32 {
                let g = 1 + ((r - r0) / ring) as usize;
                let mut a = d.y.atan2(d.x);
                let (s0, _) = self.fan_span(g, 0);
                while a < s0 {
                    a += 2.0 * PI;
                }
                let frac = (a - s0) / self.x.fan_deg.to_radians();
                if (0.0..1.0).contains(&frac) {
                    hovered = Some((g, (frac * (1usize << g) as f32) as usize));
                }
            }
        }
        // centre
        let root_p = &self.db.people[&root];
        painter.circle(centre, r0, if hovered == Some((0, 0)) { accent.gamma_multiply(0.85) } else { accent }, Stroke::new(1.5_f32, ink_c));
        let rl = life(root_p);
        painter.text(centre - vec2(0.0, 7.0), Align2::CENTER_CENTER, short_name(&styled_name(root_p), 18), FontId::proportional((r0 / 5.0).clamp(10.0, 15.0)), on_accent(accent));
        painter.text(centre + vec2(0.0, 9.0), Align2::CENTER_CENTER, rl, FontId::proportional((r0 / 7.0).clamp(9.0, 12.0)), on_accent(accent));
        for g in 1..gens {
            let n = 1usize << g;
            let (ri, ro) = (r0 + ring * (g - 1) as f32, r0 + ring * g as f32);
            for i in 0..n {
                let k = n + i;
                let who = anc.get(k).copied().flatten();
                let (a0, a1) = self.fan_span(g, i);
                let paternal = i < n / 2;
                let is_h = hovered == Some((g, i));
                let fill = match who {
                    Some(_) if is_h => fan_fill(dark, paternal, g).gamma_multiply(1.25),
                    Some(_) => fan_fill(dark, paternal, g),
                    None => {
                        if is_h {
                            head_c.gamma_multiply(0.35)
                        } else {
                            Color32::TRANSPARENT
                        }
                    }
                };
                // filled in thin convex slices
                let steps = ((a1 - a0).abs() / 0.05).ceil().max(2.0) as usize;
                let mut outline: Vec<egui::Pos2> = Vec::new();
                for s in 0..steps {
                    let t0 = a0 + (a1 - a0) * s as f32 / steps as f32;
                    let t1 = a0 + (a1 - a0) * (s + 1) as f32 / steps as f32;
                    let p = |r: f32, t: f32| centre + vec2(t.cos(), t.sin()) * r;
                    if fill != Color32::TRANSPARENT {
                        painter.add(egui::Shape::convex_polygon(vec![p(ri, t0), p(ro, t0), p(ro, t1), p(ri, t1)], fill, Stroke::NONE));
                    }
                    outline.push(p(ro, t0));
                    if s + 1 == steps {
                        outline.push(p(ro, t1));
                    }
                }
                let mut inner: Vec<egui::Pos2> = (0..=steps).rev().map(|s| {
                    let t = a0 + (a1 - a0) * s as f32 / steps as f32;
                    centre + vec2(t.cos(), t.sin()) * ri
                }).collect();
                outline.append(&mut inner);
                outline.push(outline[0]);
                painter.add(egui::Shape::line(outline, edge));
                // label
                let Some(id) = who else { continue };
                let p = &self.db.people[&id];
                let mid_a = (a0 + a1) / 2.0;
                let mid_r = (ri + ro) / 2.0;
                let arc_len = (a1 - a0).abs() * mid_r;
                let size = (ring / 6.5).clamp(8.0, 14.0);
                let (lines, along): (Vec<String>, bool) = if arc_len > ring * 1.3 {
                    // wide slot: text across the slot
                    (vec![short_name(&styled_name(p), ((arc_len / (size * 0.55)) as usize).max(6)), life(p)], true)
                } else {
                    // narrow slot: text along the radius
                    (vec![short_name(&styled_name(p), ((ring / (size * 0.55)) as usize).max(6)), life(p)], false)
                };
                let angle = if along {
                    let t = mid_a + PI / 2.0;
                    // keep text upright
                    if mid_a.sin() > 0.2 { t + PI } else { t }
                } else if mid_a.cos() < 0.0 {
                    mid_a + PI
                } else {
                    mid_a
                };
                let c = centre + vec2(mid_a.cos(), mid_a.sin()) * mid_r;
                let gap = size * 1.15;
                for (li, t) in lines.iter().enumerate() {
                    if t.is_empty() {
                        continue;
                    }
                    let fs = if li == 0 { size } else { size * 0.85 };
                    let galley = painter.layout_no_wrap(t.clone(), FontId::proportional(fs), ink);
                    let off_y = (li as f32 - (lines.len() as f32 - 1.0) / 2.0) * gap;
                    // centre the galley on c, shifted perpendicular to the text direction
                    let dir = vec2(angle.cos(), angle.sin());
                    let nrm = vec2(-angle.sin(), angle.cos());
                    let half = galley.size() / 2.0;
                    let pos = c - dir * half.x - nrm * half.y + nrm * off_y;
                    painter.add(egui::epaint::TextShape::new(pos, galley, ink).with_angle(angle));
                }
            }
        }
        let who_at = |h: Option<(usize, usize)>| -> Option<(usize, Option<u32>)> {
            let (g, i) = h?;
            let k = (1usize << g) + i;
            Some((k, anc.get(k).copied().flatten()))
        };
        if let Some((k, w)) = who_at(hovered) {
            match w {
                Some(id) => {
                    resp.clone().on_hover_text(format!("{}\nDouble-click to open, right-click for more", self.db.label(id)));
                }
                None if k > 1 => {
                    let child = anc.get(k / 2).copied().flatten();
                    if child.is_some() {
                        resp.clone().on_hover_text(if k % 2 == 0 { "Click to add a father" } else { "Click to add a mother" });
                    }
                }
                _ => {}
            }
        }
        if resp.secondary_clicked() {
            self.x.fan_menu = who_at(hovered).and_then(|x| x.1);
        }
        if let Some(id) = self.x.fan_menu {
            let mut keep = false;
            resp.context_menu(|ui| {
                keep = true;
                self.person_menu(ui, id, act);
            });
            if !keep {
                self.x.fan_menu = None;
            }
        }
        if resp.double_clicked() {
            if let Some((_, Some(id))) = who_at(hovered) {
                *act = Some(Action::Open(id));
            }
        } else if resp.clicked() {
            match who_at(hovered) {
                Some((_, Some(id))) => *act = Some(Action::Highlight(id)),
                Some((k, None)) if k > 1 => {
                    if let Some(child) = anc.get(k / 2).copied().flatten() {
                        *act = Some(Action::AddFor(child, if k % 2 == 0 { Kind::Father } else { Kind::Mother }));
                    }
                }
                _ => {}
            }
        }
        if let Some(h) = self.hl {
            if let Some(k) = anc.iter().position(|x| *x == Some(h)) {
                if k > 1 {
                    let g = (usize::BITS - 1 - k.leading_zeros()) as usize;
                    let i = k - (1 << g);
                    let (a0, a1) = self.fan_span(g, i);
                    let (ri, ro) = (r0 + ring * (g - 1) as f32, r0 + ring * g as f32);
                    let pts: Vec<egui::Pos2> = (0..=12).map(|s| a0 + (a1 - a0) * s as f32 / 12.0).map(|t| centre + vec2(t.cos(), t.sin()) * ro).chain((0..=12).rev().map(|s| a0 + (a1 - a0) * s as f32 / 12.0).map(|t| centre + vec2(t.cos(), t.sin()) * ri)).collect();
                    let mut closed = pts.clone();
                    closed.push(pts[0]);
                    painter.add(egui::Shape::line(closed, Stroke::new(3.0_f32, accent)));
                }
            }
        }
        let found = anc.iter().skip(2).filter(|x| x.is_some()).count();
        let slots = anc.len() - 2;
        painter.text(rect.min + vec2(6.0, 4.0), Align2::LEFT_TOP, format!("{} of {} ancestors found in {} generations", found, slots, gens), FontId::proportional(13.0), head_c);
    }

    /// Ancestors above, the person in the middle, children and grandchildren below.
    fn hourglass(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let Some(root) = self.sel else { return };
        let up = (self.x.fan_gens.clamp(3, 8) - 1).min(4);
        let anc = ahnen(&self.db, root, up + 1);
        let (line, edge, box_c, hover_c, ink_c, head_c, accent) = self.chart_colors(ui);
        let z = self.settings.zoom;
        let (bw, bh, vgap) = (150.0 * z, 40.0 * z, 34.0 * z);
        // rows below: children, then grandchildren
        let kids: Vec<u32> = self.db.children_of(root);
        let gkids: Vec<(u32, Vec<u32>)> = kids.iter().map(|k| (*k, self.db.children_of(*k))).collect();
        let top_slots = 1usize << up;
        let n_g = gkids.iter().map(|x| x.1.len().max(1)).sum::<usize>().max(kids.len()).max(1);
        let width = (top_slots.max(n_g) as f32) * (bw + 10.0) + 40.0;
        let height = (up as f32 + 3.0) * (bh + vgap) + 40.0;
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(vec2(width.max(ui.available_width()), height), Sense::hover());
            let o = resp.rect.min + vec2(20.0, 20.0);
            let cx = resp.rect.center().x;
            let mut boxes: Vec<(Rect, u32, bool)> = Vec::new();
            let row_y = |r: usize| o.y + r as f32 * (bh + vgap);
            // ancestors: row 0 is the oldest generation shown
            let mut pos_of: HashMap<usize, Rect> = HashMap::new();
            for g in (0..=up).rev() {
                let n = 1usize << g;
                let row = up - g;
                let span = top_slots as f32 * (bw + 10.0);
                for i in 0..n {
                    let k = n + i;
                    let x = cx - span / 2.0 + (i as f32 + 0.5) * span / n as f32 - bw / 2.0;
                    let r = Rect::from_min_size(pos2(x, row_y(row)), vec2(bw, bh));
                    pos_of.insert(k, r);
                    if let Some(id) = anc.get(k).copied().flatten() {
                        boxes.push((r, id, k == 1));
                    }
                }
            }
            // lines between generations
            for (k, r) in &pos_of {
                if *k == 1 || anc.get(*k).copied().flatten().is_none() {
                    continue;
                }
                if let Some(c) = pos_of.get(&(k / 2)) {
                    let a = pos2(r.center().x, r.max.y);
                    let b = pos2(c.center().x, c.min.y);
                    let my = (a.y + b.y) / 2.0;
                    painter.line_segment([a, pos2(a.x, my)], line);
                    painter.line_segment([pos2(a.x, my), pos2(b.x, my)], line);
                    painter.line_segment([pos2(b.x, my), b], line);
                }
            }
            // descendants
            let root_r = pos_of[&1];
            let kid_row = up + 1;
            let gk_row = up + 2;
            let total_w = n_g as f32 * (bw + 10.0);
            let mut x = cx - total_w / 2.0;
            for (k, gks) in &gkids {
                let w = gks.len().max(1) as f32 * (bw + 10.0);
                let kr = Rect::from_min_size(pos2(x + w / 2.0 - bw / 2.0, row_y(kid_row)), vec2(bw, bh));
                boxes.push((kr, *k, false));
                let a = pos2(root_r.center().x, root_r.max.y);
                let b = pos2(kr.center().x, kr.min.y);
                let my = (a.y + b.y) / 2.0;
                painter.line_segment([a, pos2(a.x, my)], line);
                painter.line_segment([pos2(a.x, my), pos2(b.x, my)], line);
                painter.line_segment([pos2(b.x, my), b], line);
                for (j, g2) in gks.iter().enumerate() {
                    let gr = Rect::from_min_size(pos2(x + j as f32 * (bw + 10.0) + 5.0, row_y(gk_row)), vec2(bw, bh));
                    boxes.push((gr, *g2, false));
                    let a = pos2(kr.center().x, kr.max.y);
                    let b = pos2(gr.center().x, gr.min.y);
                    let my = (a.y + b.y) / 2.0;
                    painter.line_segment([a, pos2(a.x, my)], line);
                    painter.line_segment([pos2(a.x, my), pos2(b.x, my)], line);
                    painter.line_segment([pos2(b.x, my), b], line);
                }
                x += w;
            }
            for (i, (r, id, is_root)) in boxes.iter().enumerate() {
                let rr = ui.interact(*r, egui::Id::new(("hg", i, *id)), Sense::click());
                let fill = if *is_root { accent } else if rr.hovered() { hover_c } else { box_c };
                painter.rect(*r, 5.0_f32, fill, edge);
                if self.hl == Some(*id) && !*is_root {
                    painter.rect_stroke(*r, 5.0_f32, Stroke::new(3.0_f32, accent));
                }
                let p = &self.db.people[id];
                let ink = if *is_root { on_accent(accent) } else { ink_c };
                let clip = r.shrink(3.0);
                painter.with_clip_rect(clip).text(pos2(r.min.x + 6.0, r.min.y + bh * 0.32), Align2::LEFT_CENTER, styled_name(p), FontId::proportional(13.0 * z), ink);
                painter.with_clip_rect(clip).text(pos2(r.min.x + 6.0, r.min.y + bh * 0.72), Align2::LEFT_CENTER, life(p), FontId::proportional(11.0 * z), if *is_root { ink } else { head_c });
                rr.context_menu(|ui| self.person_menu(ui, *id, act));
                if rr.double_clicked() {
                    *act = Some(Action::Open(*id));
                } else if rr.clicked() {
                    *act = Some(if *is_root { Action::Highlight(*id) } else { Action::Select(*id) });
                }
            }
            if kids.is_empty() {
                painter.text(pos2(cx, row_y(kid_row) + bh / 2.0), Align2::CENTER_CENTER, "No children recorded", FontId::proportional(13.0), head_c);
            }
        });
    }

    /// The person's close family as bars along a line of years.
    fn lifespans(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let Some(root) = self.sel else { return };
        let mut rows: Vec<(u32, &'static str)> = Vec::new();
        let (f, m) = self.db.parents(root);
        for (who, rel) in [(f, "Father"), (m, "Mother")] {
            if let Some(w) = who {
                let (gf, gm) = self.db.parents(w);
                if let Some(x) = gf {
                    rows.push((x, "Grandfather"));
                }
                if let Some(x) = gm {
                    rows.push((x, "Grandmother"));
                }
                rows.push((w, rel));
            }
        }
        rows.push((root, "This person"));
        for s in self.db.siblings(root) {
            rows.push((s, "Brother or sister"));
        }
        for s in self.db.partners(root) {
            rows.push((s, "Spouse or partner"));
        }
        for c in self.db.children_of(root) {
            rows.push((c, "Child"));
        }
        let mut seen = HashSet::new();
        rows.retain(|r| seen.insert(r.0));
        let this_year = today()[..4].parse::<i32>().unwrap_or(2026);
        let spans: Vec<(u32, &str, Option<i32>, Option<i32>, bool)> = rows
            .iter()
            .map(|(id, rel)| {
                let p = &self.db.people[id];
                let (b, d) = years(p);
                (*id, *rel, b, d, p.living)
            })
            .collect();
        let known: Vec<i32> = spans.iter().flat_map(|s| [s.2, s.3]).flatten().collect();
        if known.is_empty() {
            ui.label(RichText::new("Add birth or death dates to see lifespans.").italics().weak());
            return;
        }
        let y0 = (known.iter().min().unwrap() - 5) / 10 * 10;
        let y1 = ((known.iter().max().unwrap().max(&this_year).min(&(this_year)) + 9) / 10 * 10).max(y0 + 20);
        let (line, edge, _box_c, hover_c, ink_c, head_c, accent) = self.chart_colors(ui);
        let dark = ui.visuals().dark_mode;
        let row_h = 30.0;
        let label_w = 250.0;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let w = ui.available_width().max(600.0);
            let h = 40.0 + spans.len() as f32 * row_h + 20.0;
            let (resp, painter) = ui.allocate_painter(vec2(w, h), Sense::hover());
            let r = resp.rect;
            let x_of = |y: f32| r.min.x + label_w + (y - y0 as f32) / (y1 - y0) as f32 * (r.width() - label_w - 20.0);
            // year grid
            let step = if y1 - y0 > 200 { 50 } else if y1 - y0 > 80 { 20 } else { 10 };
            let mut y = y0;
            while y <= y1 {
                let x = x_of(y as f32);
                painter.line_segment([pos2(x, r.min.y + 24.0), pos2(x, r.max.y - 10.0)], Stroke::new(1.0_f32, line.color.gamma_multiply(0.5)));
                painter.text(pos2(x, r.min.y + 4.0), Align2::CENTER_TOP, y.to_string(), FontId::proportional(12.0), head_c);
                y += step;
            }
            for (i, (id, rel, b, d, living)) in spans.iter().enumerate() {
                let top = r.min.y + 30.0 + i as f32 * row_h;
                let lr = Rect::from_min_size(pos2(r.min.x, top), vec2(label_w - 10.0, row_h - 6.0));
                let rr = ui.interact(Rect::from_min_size(pos2(r.min.x, top), vec2(r.width(), row_h - 4.0)), egui::Id::new(("ls", *id)), Sense::click());
                if rr.hovered() {
                    painter.rect_filled(rr.rect, 3.0_f32, hover_c);
                }
                let p = &self.db.people[id];
                painter.with_clip_rect(lr).text(pos2(lr.min.x + 4.0, lr.center().y - 6.0), Align2::LEFT_CENTER, styled_name(p), FontId::proportional(14.0), ink_c);
                painter.with_clip_rect(lr).text(pos2(lr.min.x + 4.0, lr.center().y + 8.0), Align2::LEFT_CENTER, *rel, FontId::proportional(11.0), head_c);
                let col = if *id == root { accent } else if p.sex == "F" { fan_fill(dark, false, 2) } else { fan_fill(dark, true, 2) };
                let bar_y = top + (row_h - 6.0) / 2.0;
                match (b, d) {
                    (Some(b), Some(d)) => {
                        painter.rect(Rect::from_min_max(pos2(x_of(*b as f32), bar_y - 6.0), pos2(x_of(*d as f32).max(x_of(*b as f32) + 3.0), bar_y + 6.0)), 4.0, col, edge);
                        painter.text(pos2(x_of(*d as f32) + 6.0, bar_y), Align2::LEFT_CENTER, format!("{} years", d - b), FontId::proportional(11.0), head_c);
                    }
                    (Some(b), None) => {
                        let end = if *living { this_year } else { (*b + 70).min(this_year) };
                        let br = Rect::from_min_max(pos2(x_of(*b as f32), bar_y - 6.0), pos2(x_of(end as f32), bar_y + 6.0));
                        painter.rect(br, 4.0_f32, if *living { col } else { col.gamma_multiply(0.45) }, edge);
                        if !*living {
                            painter.text(pos2(br.max.x + 6.0, bar_y), Align2::LEFT_CENTER, "death not recorded", FontId::proportional(11.0), head_c);
                        }
                    }
                    (None, Some(d)) => {
                        let br = Rect::from_min_max(pos2(x_of((*d - 60) as f32), bar_y - 6.0), pos2(x_of(*d as f32), bar_y + 6.0));
                        painter.rect(br, 4.0_f32, col.gamma_multiply(0.45), edge);
                        painter.text(pos2(br.min.x - 6.0, bar_y), Align2::RIGHT_CENTER, "birth not recorded", FontId::proportional(11.0), head_c);
                    }
                    (None, None) => {
                        painter.text(pos2(x_of(y0 as f32) + 4.0, bar_y), Align2::LEFT_CENTER, "no dates", FontId::proportional(11.0), head_c);
                    }
                }
                rr.context_menu(|ui| self.person_menu(ui, *id, act));
                if rr.double_clicked() {
                    *act = Some(Action::Open(*id));
                } else if rr.clicked() {
                    *act = Some(Action::Select(*id));
                }
            }
            // a line for today
            let tx = x_of(this_year as f32);
            painter.line_segment([pos2(tx, r.min.y + 24.0), pos2(tx, r.max.y - 10.0)], Stroke::new(1.5_f32, accent.gamma_multiply(0.7)));
        });
    }

    /// A printable page (SVG in the browser) of the chart on screen.
    pub(crate) fn chart_page(&self) -> Option<(String, String)> {
        let root = self.sel?;
        let rp = &self.db.people[&root];
        match self.x.chart {
            ChartKind::Fan => {
                let gens = self.x.fan_gens.clamp(3, 8);
                let anc = ahnen(&self.db, root, gens);
                let r_max = 480.0;
                let r0 = r_max * 0.16;
                let ring = (r_max - r0) / (gens - 1) as f32;
                let full = self.x.fan_deg >= 359.0;
                let (w, h) = (2.0 * r_max + 40.0, if full { 2.0 * r_max + 40.0 } else { r_max * 1.6 + 40.0 });
                let (cx, cy) = (w / 2.0, if full { h / 2.0 } else { 20.0 + r_max });
                let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" font-family=\"Georgia,serif\">");
                s.push_str(&format!("<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r0}\" fill=\"#eef\" stroke=\"#555\"/>"));
                s.push_str(&format!("<text x=\"{cx}\" y=\"{}\" font-size=\"14\" text-anchor=\"middle\">{}</text>", cy - 4.0, esc(&short_name(&rp.name, 22))));
                s.push_str(&format!("<text x=\"{cx}\" y=\"{}\" font-size=\"11\" text-anchor=\"middle\">{}</text>", cy + 12.0, esc(&life(rp))));
                for g in 1..gens {
                    let n = 1usize << g;
                    let (ri, ro) = (r0 + ring * (g - 1) as f32, r0 + ring * g as f32);
                    for i in 0..n {
                        let (a0, a1) = self.fan_span(g, i);
                        let pt = |r: f32, t: f32| (cx + r * t.cos(), cy + r * t.sin());
                        let (x1, y1) = pt(ro, a0);
                        let (x2, y2) = pt(ro, a1);
                        let (x3, y3) = pt(ri, a1);
                        let (x4, y4) = pt(ri, a0);
                        let large = if (a1 - a0) > PI { 1 } else { 0 };
                        let who = anc.get(n + i).copied().flatten();
                        let fill = if who.is_none() { "#fff" } else if i < n / 2 { "#dde6f3" } else { "#f3dde3" };
                        s.push_str(&format!("<path d=\"M{x1:.1},{y1:.1} A{ro:.1},{ro:.1} 0 {large} 1 {x2:.1},{y2:.1} L{x3:.1},{y3:.1} A{ri:.1},{ri:.1} 0 {large} 0 {x4:.1},{y4:.1} Z\" fill=\"{fill}\" stroke=\"#666\" stroke-width=\"0.8\"/>"));
                        let Some(id) = who else { continue };
                        let p = &self.db.people[&id];
                        let mid = (a0 + a1) / 2.0;
                        let mr = (ri + ro) / 2.0;
                        let (tx, ty) = pt(mr, mid);
                        let arc = (a1 - a0) * mr;
                        let fs = (ring / 7.0).clamp(7.0, 13.0);
                        let rot = if arc > ring * 1.3 {
                            let t = (mid + PI / 2.0).to_degrees();
                            if mid.sin() > 0.2 { t + 180.0 } else { t }
                        } else if mid.cos() < 0.0 {
                            (mid + PI).to_degrees()
                        } else {
                            mid.to_degrees()
                        };
                        let max_chars = if arc > ring * 1.3 { (arc / (fs * 0.55)) as usize } else { (ring / (fs * 0.55)) as usize };
                        s.push_str(&format!(
                            "<g transform=\"translate({tx:.1},{ty:.1}) rotate({rot:.1})\"><text y=\"-2\" font-size=\"{fs:.1}\" text-anchor=\"middle\">{}</text><text y=\"{:.1}\" font-size=\"{:.1}\" text-anchor=\"middle\" fill=\"#444\">{}</text></g>",
                            esc(&short_name(&p.name, max_chars.max(6))),
                            fs,
                            fs * 0.85,
                            esc(&life(p))
                        ));
                    }
                }
                s.push_str("</svg>");
                Some((format!("Fan chart of {}", rp.name), s))
            }
            ChartKind::Hourglass | ChartKind::Lifespans => {
                // a simple, clean list layout that prints well on any paper size
                let mut b = String::new();
                let row = |b: &mut String, rel: &str, id: u32| {
                    let p = &self.db.people[&id];
                    b.push_str(&format!("<tr><td class=\"l\">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", esc(rel), esc(&p.name), dp(&p.bd, &p.bp), dp(&p.dd, &p.dp)));
                };
                b.push_str("<table><tr><th>Relationship</th><th>Name</th><th>Born</th><th>Died</th></tr>");
                let anc = ahnen(&self.db, root, 4);
                let names = ["", "This person", "Father", "Mother", "Father's father", "Father's mother", "Mother's father", "Mother's mother", "", "", "", "", "", "", "", ""];
                for k in (2..8).rev() {
                    if let Some(id) = anc[k] {
                        row(&mut b, names[k], id);
                    }
                }
                row(&mut b, "This person", root);
                for s in self.db.partners(root) {
                    row(&mut b, "Spouse or partner", s);
                }
                for s in self.db.siblings(root) {
                    row(&mut b, "Brother or sister", s);
                }
                for c in self.db.children_of(root) {
                    row(&mut b, "Child", c);
                    for g in self.db.children_of(c) {
                        row(&mut b, "Grandchild", g);
                    }
                }
                b.push_str("</table>");
                Some((format!("Family of {}", rp.name), b))
            }
        }
    }
}

/// A name cut to fit `max` characters, keeping the surname.
pub fn short_name(n: &str, max: usize) -> String {
    let n = n.trim();
    if n.chars().count() <= max {
        return n.to_string();
    }
    let parts: Vec<&str> = n.split_whitespace().collect();
    if parts.len() >= 2 {
        // initials for the given names
        let sur = parts[parts.len() - 1];
        let ini: String = parts[..parts.len() - 1].iter().filter_map(|p| p.chars().next()).map(|c| format!("{}.", c)).collect::<Vec<_>>().join(" ");
        let s = format!("{} {}", ini, sur);
        if s.chars().count() <= max {
            return s;
        }
    }
    let mut s: String = n.chars().take(max.saturating_sub(1)).collect();
    s.push('\u{2026}');
    s
}
