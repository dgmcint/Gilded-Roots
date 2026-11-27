//! Other ways to show the pedigree, picked from the Style list above the chart: Photo cards and Compact
//! (drawn by the classic chart with other box sizes), Couple cards (one card per couple, with their
//! marriage and a list of their children) and Upright tree (ancestors rising above the chosen person,
//! children underneath). Our own layouts, colours and wording.

use super::*;

/// The styles, in the order of the Style list.
pub(crate) const PED_STYLES: [&str; 5] = ["Classic", "Photo cards", "Couple cards", "Upright tree", "Compact"];
/// How boxes are coloured.
pub(crate) const PED_COLOURS: [&str; 3] = ["Plain", "Men and women", "Father's and mother's side"];

/// The most generations each style shows.
pub(crate) fn ped_max_gens(style: u8) -> usize {
    match style {
        1 => 6,
        2 => 6,
        3 => 5,
        4 => 10,
        _ => 8,
    }
}

/// `a` mixed into `b` by t (0 = all b).
fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Colour-blind friendly chart colours (Settings > Appearance): blue and orange, which people with
/// red-green colour blindness (the common kinds) can tell apart, instead of blue and pink.
static CB_COLOURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn set_cb_colours(on: bool) {
    CB_COLOURS.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn cb_colours() -> bool {
    CB_COLOURS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The colour for men on charts.
pub(crate) fn man() -> Color32 {
    if cb_colours() { Color32::from_rgb(0, 114, 178) } else { Color32::from_rgb(70, 120, 200) }
}

/// The colour for women on charts.
pub(crate) fn woman() -> Color32 {
    if cb_colours() { Color32::from_rgb(230, 159, 0) } else { Color32::from_rgb(200, 80, 130) }
}

/// The colour for the father's side.
pub(crate) fn father_side() -> Color32 {
    if cb_colours() { Color32::from_rgb(86, 180, 233) } else { Color32::from_rgb(60, 150, 120) }
}

/// The colour for the mother's side.
pub(crate) fn mother_side() -> Color32 {
    if cb_colours() { Color32::from_rgb(213, 94, 0) } else { Color32::from_rgb(200, 140, 50) }
}

impl App {
    /// The colour a box is tinted with: by man or woman, or by the father's or mother's side.
    /// `side` is Some(true) for the father's side of the chosen person, Some(false) for the mother's.
    pub(crate) fn ped_tint(&self, sex: &str, side: Option<bool>) -> Option<Color32> {
        match self.settings.ped_colours {
            1 => match sex {
                "M" => Some(man()),
                "F" => Some(woman()),
                _ => None,
            },
            2 => side.map(|f| if f { father_side() } else { mother_side() }),
            _ => None,
        }
    }

    /// The colour of the band across the top of a card, when the band is turned on: the two sides
    /// when Colours is set to them, otherwise men and women.
    pub(crate) fn ped_band_colour(&self, sex: &str, side: Option<bool>) -> Option<Color32> {
        if !self.settings.ped_band {
            return None;
        }
        if self.settings.ped_colours == 2 {
            return side.map(|f| if f { father_side() } else { mother_side() });
        }
        match sex {
            "M" => Some(man()),
            "F" => Some(woman()),
            _ => None,
        }
    }

    /// A box's fill with its tint, if any.
    pub(crate) fn ped_fill(&self, base: Color32, sex: &str, side: Option<bool>) -> Color32 {
        match self.ped_tint(sex, side) {
            Some(t) => blend(t, base, 0.22),
            None => base,
        }
    }

    /// Pedigree, Fan chart or Hourglass: the views of the Pedigree tab (as on the genealogy websites).
    pub(crate) fn ped_mode_switch(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let m = self.settings.ped_mode;
        // 0 pedigree, 3 descendants, 1 fan chart, 2 hourglass (the numbers kept from before)
        for (k, l) in [(0u8, crate::i18n::tr("Pedigree")), (3, crate::i18n::tr("Descendants")), (1, crate::i18n::tr("Fan chart")), (2, crate::i18n::tr("Hourglass"))] {
            let r = ui.selectable_label(m == k, RichText::new(l).size(15.0).strong());
            crate::tour::mark(["ped:Pedigree", "ped:Fan", "ped:Hourglass", "ped:Descendants"][k as usize], r.rect);
            if r.clicked() && m != k {
                *act = Some(Action::PedMode(k));
            }
        }
        ui.separator();
    }

    /// The row above every pedigree style: style, colours, generations, zoom and print (when no toolbar has them).
    pub(crate) fn ped_controls(&self, ui: &mut egui::Ui, act: &mut Option<Action>, gens: usize, z: f32) {
        let style = self.settings.ped_style.min(4);
        let gmax = ped_max_gens(style);
        ui.horizontal_wrapped(|ui| {
            self.ped_mode_switch(ui, act);
            ui.label(crate::i18n::tr("Style:"));
            egui::ComboBox::from_id_salt("ped_style").selected_text(crate::i18n::trv(PED_STYLES[style as usize])).show_ui(ui, |ui| {
                for (i, s) in PED_STYLES.iter().enumerate() {
                    if ui.selectable_label(style as usize == i, crate::i18n::trv(s)).clicked() {
                        *act = Some(Action::PedStyle(i as u8, self.settings.ped_colours));
                    }
                }
            });
            ui.label(crate::i18n::tr("Colours:"));
            let c = self.settings.ped_colours.min(2);
            egui::ComboBox::from_id_salt("ped_colours").selected_text(crate::i18n::trv(PED_COLOURS[c as usize])).show_ui(ui, |ui| {
                for (i, s) in PED_COLOURS.iter().enumerate() {
                    if ui.selectable_label(c as usize == i, crate::i18n::trv(s)).clicked() {
                        *act = Some(Action::PedStyle(style, i as u8));
                    }
                }
            });
            let mut band = self.settings.ped_band;
            if ui.checkbox(&mut band, crate::i18n::tr("Colour band")).on_hover_text(crate::i18n::tr("A thin coloured band across the top of each card: blue for men and pink for women, or the father's and mother's sides when Colours is set to them")).changed() {
                *act = Some(Action::PedBand(band));
            }
            ui.separator();
            ui.label(crate::i18n::tr("Generations:"));
            if ui.add_enabled(gens > 3, egui::Button::new("-")).clicked() {
                *act = Some(Action::SetGens(gens - 1));
            }
            ui.label(gens.to_string());
            if ui.add_enabled(gens < gmax, egui::Button::new("+")).clicked() {
                *act = Some(Action::SetGens(gens + 1));
            }
            self.row_zoom_and_print(ui, z, act);
        });
    }

    /// The chosen person's ancestors, generation by generation: slot i of generation g has the parents
    /// of slot i/2 of generation g-1 (father at even slots, mother at odd).
    pub(crate) fn ancestor_slots(&self, sel: u32, gens: usize) -> Vec<Vec<Option<u32>>> {
        let ix = self.db.index();
        let mut slots: Vec<Vec<Option<u32>>> = vec![vec![Some(sel)]];
        for g in 1..gens {
            let mut c = Vec::new();
            for s in &slots[g - 1] {
                let (f, m) = match s {
                    Some(i) => self.db.parents_ix(&ix, *i),
                    None => (None, None),
                };
                c.push(f);
                c.push(m);
            }
            slots.push(c);
        }
        slots
    }

    /// The family in which `child` is the child of this father and mother.
    fn couple_family(&self, child: u32, f: Option<u32>, m: Option<u32>) -> Option<u32> {
        self.db.fams.iter().find(|(_, fam)| fam.kids.contains(&child) && fam.h == f && fam.w == m).map(|(k, _)| *k)
    }

    /// One person's photo, or their initial, in a rectangle (`round` for a circle).
    fn ped_face(&self, ui: &mut egui::Ui, r: Rect, p: &Person, round: bool, back: Color32, ink: Color32) {
        let rounding = if round { r.width() / 2.0 } else { 4.0 };
        match self.portrait_src(p).filter(|_| self.settings.box_photos) {
            Some((uri, fr)) => put_photo(ui, r, &uri, fr, rounding),
            None => {
                ui.painter().rect_filled(r, rounding, back);
                let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
                ui.painter().text(r.center(), Align2::CENTER_CENTER, initial, FontId::proportional(r.height() * 0.42), ink);
            }
        }
    }

    /// A person line inside a card: clicking chooses them, double-clicking opens their page, right-click
    /// gives the usual menu. Returns the response.
    fn ped_person_hit(&self, ui: &mut egui::Ui, r: Rect, id: u32, key: impl std::hash::Hash, act: &mut Option<Action>) -> egui::Response {
        let resp = ui.interact(r, egui::Id::new(("pedp", key)), Sense::click());
        crate::views::speak_person(&resp, &self.db, id);
        if resp.double_clicked() {
            *act = Some(Action::Open(id));
        } else if resp.clicked() {
            *act = Some(Action::Select(id));
        }
        resp.context_menu(|ui| self.person_menu(ui, id, act));
        resp
    }

    /// Couple cards: the chosen person, then one card per couple going back, each with both names, a
    /// coloured edge for each, their marriage, and a list of their children to jump to.
    pub(crate) fn couple_pedigree(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let Some(sel) = self.sel.filter(|s| self.db.people.contains_key(s)) else { return };
        let gens = self.settings.gens.clamp(3, ped_max_gens(2));
        let z = self.settings.zoom;
        let (line, edge, box_c, hover_c, ink, head, accent) = self.chart_colors(ui);
        let (cw, ch, gap, pitch, top) = (300.0 * z, 78.0 * z, 34.0 * z, 92.0 * z, 30.0 * z);
        let slots = self.ancestor_slots(sel, gens);
        // columns: 0 the chosen person, then generation g's couples (g = 1..gens-1)
        let rows = (1usize << (gens - 2)).max(1);
        let tw = cw + (gens - 1) as f32 * (cw + gap);
        let th = top + rows as f32 * pitch + 60.0 * z;
        if self.fit_req {
            let av = ui.available_size();
            let zw = (av.x - 24.0) / (300.0 + (gens - 1) as f32 * 334.0);
            let zh = (av.y - 100.0) / (rows as f32 * 92.0 + 90.0);
            *act = Some(Action::SetZoom(zw.min(zh).clamp(0.25, 1.8)));
        }
        self.nav_bar(ui, act);
        self.ped_controls(ui, act, gens, z);
        self.paint_stage(ui);
        let view = chart_canvas(ui, ("ped_cc", sel, z.to_bits()), vec2(tw + 12.0, th), |ui| {
            let (resp, mut painter) = ui.allocate_painter(vec2(tw + 12.0, th), Sense::hover());
            let o = resp.rect.min + vec2(6.0, 0.0);
            // the chart can be dragged past where it was laid out, so paint anywhere in sight
            painter.set_clip_rect(ui.clip_rect());
            if resp.hovered() {
                let zd = ui.input(|i| i.zoom_delta());
                if (zd - 1.0).abs() > 0.001 {
                    *act = Some(Action::SetZoom((z * zd).clamp(0.25, 1.8)));
                }
            }
            painter.text(pos2(o.x, o.y + 4.0), Align2::LEFT_TOP, ordinal(1), FontId::proportional(17.0 * z), head);
            for g in 1..gens {
                painter.text(pos2(o.x + g as f32 * (cw + gap), o.y + 4.0), Align2::LEFT_TOP, ordinal(g + 1), FontId::proportional(17.0 * z), head);
            }
            let centre_y = |col: usize, i: usize| -> f32 {
                // column col (>= 1) has 2^(col-1) cards; column 0 has one
                let n = if col == 0 { 1 } else { 1usize << (col - 1) };
                let step = rows as f32 * pitch / n as f32;
                o.y + top + (i as f32 + 0.5) * step
            };
            // the chosen person
            let p = &self.db.people[&sel];
            let cy0 = centre_y(0, 0);
            let r0 = Rect::from_min_size(pos2(o.x, cy0 - 26.0 * z), vec2(cw, 52.0 * z));
            crate::views::card(&painter, r0, 8.0, accent, Stroke::new(2.0_f32, ink), ui.visuals().dark_mode);
            self.ped_face(ui, Rect::from_min_size(r0.min + vec2(5.0, 5.0) * z, vec2(42.0, 42.0) * z), p, false, on_accent(accent).gamma_multiply(0.22), on_accent(accent));
            painter.with_clip_rect(r0.shrink(2.0)).text(pos2(r0.min.x + 54.0 * z, r0.center().y - 8.0 * z), Align2::LEFT_CENTER, styled_name(p), FontId::proportional(15.0 * z), on_accent(accent));
            painter.text(pos2(r0.min.x + 54.0 * z, r0.center().y + 10.0 * z), Align2::LEFT_CENTER, life(p), FontId::proportional(12.5 * z), on_accent(accent));
            let h0 = self.ped_person_hit(ui, r0, sel, ("c0", sel), act);
            let _ = h0;
            let w = r0.width().min(230.0);
            let n = self.db.partners(sel).len() + self.db.children_of(sel).len();
            let n_sib = self.db.siblings(sel).len();
            self.family_panel(ui, r0, w, sel, crate::i18n::trf("Spouse & children ({})", &["Spouse & children (", ")"], &[n.to_string()]), crate::i18n::trf("Siblings ({})", &["Siblings (", ")"], &[n_sib.to_string()]), act);
            // couples
            for col in 1..gens {
                let n = 1usize << (col - 1);
                for i in 0..n {
                    let (f, m) = (slots[col][2 * i], slots[col][2 * i + 1]);
                    let child = slots[col - 1][i];
                    let x = o.x + col as f32 * (cw + gap);
                    let cy = centre_y(col, i);
                    let r = Rect::from_min_size(pos2(x, cy - ch / 2.0), vec2(cw, ch));
                    // line back to the child
                    // the child is the father (top line) or mother (second line) of the card to the left
                    let line_y = |k: f32| -ch / 2.0 + 2.0 + (k + 0.5) * ch * 0.34;
                    let child_y = if col == 1 { cy0 } else { centre_y(col - 1, i / 2) + if i % 2 == 0 { line_y(0.0) } else { line_y(1.0) } };
                    painter.add(egui::Shape::line(vec![pos2(x, cy), pos2(x - gap / 2.0, cy), pos2(x - gap / 2.0, child_y), pos2(x - gap, child_y)], line));
                    let Some(child) = child else {
                        painter.rect(r, 6.0, box_c.gamma_multiply(0.45), edge);
                        continue;
                    };
                    if f.is_none() && m.is_none() {
                        let hit = ui.interact(r, egui::Id::new(("cc_add", col, i)), Sense::click());
                        painter.rect(r, 6.0, if hit.hovered() { hover_c } else { box_c.gamma_multiply(0.45) }, edge);
                        if hit.hovered() {
                            painter.text(r.center(), Align2::CENTER_CENTER, crate::i18n::tr("+ Add parents"), FontId::proportional(13.0), head);
                        }
                        if hit.clicked() {
                            *act = Some(Action::AddFor(child, Kind::Father));
                        }
                        continue;
                    }
                    crate::views::card(&painter, r, 8.0, box_c, edge, ui.visuals().dark_mode);
                    let half = ch * 0.34;
                    for (k, who) in [(0usize, f), (1usize, m)] {
                        let lr = Rect::from_min_size(pos2(r.min.x, r.min.y + 2.0 + k as f32 * half), vec2(cw, half));
                        let side = if col == 1 { Some(k == 0) } else { Some(i < n / 2) };
                        match who.and_then(|id| self.db.people.get(&id).map(|p| (id, p))) {
                            Some((id, p)) => {
                                let hit = self.ped_person_hit(ui, lr, id, ("cc", col, i, k), act);
                                // the husband's and wife's places say which, when the record does not
                                let sex = match p.sex.as_str() {
                                    "M" | "F" => p.sex.as_str(),
                                    _ => if k == 0 { "M" } else { "F" },
                                };
                                let fill = self.ped_fill(if hit.hovered() { hover_c } else { box_c }, sex, side);
                                painter.rect_filled(lr.shrink2(vec2(1.0, 1.0)), 5.0, fill);
                                let bar = if sex == "F" { woman() } else { man() };
                                painter.rect_filled(Rect::from_min_size(lr.min + vec2(1.0, 3.0), vec2(4.0 * z, half - 6.0)), 2.0, bar);
                                let face = Rect::from_min_size(lr.min + vec2(9.0 * z, 3.0), vec2(half - 6.0, half - 6.0));
                                self.ped_face(ui, face, p, true, hover_c, head);
                                let t = format!("{}  {}", styled_name(p), life(p));
                                painter.with_clip_rect(lr.shrink(2.0)).text(pos2(face.max.x + 6.0, lr.center().y), Align2::LEFT_CENTER, t, FontId::proportional(13.5 * z), ink);
                                if col == gens - 1 {
                                    let (pf, pm) = self.db.parents(id);
                                    if pf.is_some() || pm.is_some() {
                                        let zone = Rect::from_min_size(pos2(lr.max.x - 22.0, lr.min.y), vec2(20.0, half));
                                        let ra = ui.interact(zone, egui::Id::new(("cc_more", col, i, k)), Sense::click());
                                        let (cx, cyy) = (zone.center().x, zone.center().y);
                                        painter.add(egui::Shape::convex_polygon(vec![pos2(cx - 3.0, cyy - 5.0), pos2(cx - 3.0, cyy + 5.0), pos2(cx + 4.0, cyy)], if ra.hovered() { accent } else { head }, Stroke::NONE));
                                        if ra.on_hover_text(crate::i18n::tr("Show this person's ancestors")).clicked() {
                                            *act = Some(Action::Select(id));
                                        }
                                    }
                                }
                            }
                            None => {
                                let hit = ui.interact(lr, egui::Id::new(("cc_none", col, i, k)), Sense::click());
                                if hit.hovered() {
                                    painter.text(lr.center(), Align2::CENTER_CENTER, if k == 0 { crate::i18n::tr("+ Add father") } else { crate::i18n::tr("+ Add mother") }, FontId::proportional(12.5), head);
                                }
                                if hit.clicked() {
                                    *act = Some(Action::AddFor(child, if k == 0 { Kind::Father } else { Kind::Mother }));
                                }
                            }
                        }
                    }
                    // marriage and children
                    let fam = self.couple_family(child, f, m);
                    let row = Rect::from_min_max(pos2(r.min.x + 8.0, r.min.y + 2.0 + 2.0 * half), pos2(r.max.x - 4.0, r.max.y - 2.0));
                    if let Some(fid) = fam {
                        let fm = &self.db.fams[&fid];
                        let wed = [fm.md.trim(), fm.mp.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(", ");
                        let wed = if wed.is_empty() { String::new() } else { crate::i18n::trf("Married {}", &["Married ", ""], &[wed]) };
                        let wed_clip = Rect::from_min_max(row.min, pos2(row.max.x - if fm.kids.is_empty() { 0.0 } else { 114.0 }, row.max.y));
                        painter.with_clip_rect(wed_clip).text(pos2(row.min.x, row.center().y), Align2::LEFT_CENTER, wed, FontId::proportional(12.0 * z), head);
                        let kids: Vec<u32> = fm.kids.clone();
                        if !kids.is_empty() {
                            let br = Rect::from_min_size(pos2(row.max.x - 110.0, row.min.y), vec2(108.0, row.height()));
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(br), |ui| {
                                ui.menu_button(RichText::new(crate::i18n::trf("Children ({})", &["Children (", ")"], &[kids.len().to_string()])).small(), |ui| {
                                    for k in &kids {
                                        if ui.button(self.db.label(*k)).clicked() {
                                            *act = Some(Action::Select(*k));
                                            ui.close_menu();
                                        }
                                    }
                                });
                            });
                        }
                    }
                }
            }
        });
        self.float_bar(ui, view, act);
    }

    /// Upright tree: the chosen person (with their spouse) near the bottom, parents and grandparents rising
    /// above as couple tiles with round photos, and the children in a row underneath.
    pub(crate) fn upright_pedigree(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let Some(sel) = self.sel.filter(|s| self.db.people.contains_key(s)) else { return };
        let gens = self.settings.gens.clamp(3, ped_max_gens(3));
        let z = self.settings.zoom;
        let (line, edge, box_c, hover_c, ink, head, accent) = self.chart_colors(ui);
        let (tw, th, gx, gy) = (250.0 * z, 112.0 * z, 18.0 * z, 46.0 * z);
        let slots = self.ancestor_slots(sel, gens);
        // tiles: level 0 is the chosen person (and spouse); level k >= 1 has 2^(k-1) couples
        let top_n = 1usize << (gens - 2);
        let width = (top_n as f32 * (tw + gx)).max(tw * 2.0 + gx) + 40.0;
        let kids = self.db.children_of(sel);
        let kid_w = 150.0 * z;
        let width = width.max((kids.len() + 1) as f32 * (kid_w + 10.0) + 40.0);
        let height = (gens as f32) * (th + gy) + 150.0 * z;
        if self.fit_req {
            let av = ui.available_size();
            let zw = (av.x - 24.0) / (width / z);
            let zh = (av.y - 100.0) / (height / z);
            *act = Some(Action::SetZoom(zw.min(zh).clamp(0.25, 1.8)));
        }
        self.nav_bar(ui, act);
        self.ped_controls(ui, act, gens, z);
        self.paint_stage(ui);
        let view = chart_canvas(ui, ("ped_up", sel, z.to_bits()), vec2(width, height), |ui| {
            let (resp, mut painter) = ui.allocate_painter(vec2(width, height), Sense::hover());
            let o = resp.rect.min;
            // the chart can be dragged past where it was laid out, so paint anywhere in sight
            painter.set_clip_rect(ui.clip_rect());
            if resp.hovered() {
                let zd = ui.input(|i| i.zoom_delta());
                if (zd - 1.0).abs() > 0.001 {
                    *act = Some(Action::SetZoom((z * zd).clamp(0.25, 1.8)));
                }
            }
            // x centres of the tiles, from the top level down
            let mut cx: Vec<Vec<f32>> = vec![Vec::new(); gens];
            let lvl_top = gens - 1;
            cx[lvl_top] = (0..top_n).map(|i| o.x + 20.0 + tw / 2.0 + i as f32 * (tw + gx) + (width - 40.0 - top_n as f32 * (tw + gx) + gx) / 2.0).collect();
            for k in (1..lvl_top).rev() {
                let n = 1usize << (k - 1);
                cx[k] = (0..n).map(|i| (cx[k + 1][2 * i] + cx[k + 1][2 * i + 1]) / 2.0).collect();
            }
            // level 1's husband half sits over the chosen person, so level 0 is shifted to match
            let lvl1 = cx.get(1).and_then(|v| v.first()).copied().unwrap_or(o.x + width / 2.0);
            cx[0] = vec![lvl1 + tw / 4.0];
            let y_of = |k: usize| o.y + 14.0 + (lvl_top - k) as f32 * (th + gy);
            let couple = |k: usize, i: usize| -> (Option<u32>, Option<u32>) {
                if k == 0 {
                    (Some(sel), self.db.partners(sel).first().copied())
                } else {
                    (slots[k][2 * i], slots[k][2 * i + 1])
                }
            };
            for k in (0..gens).rev() {
                let n = if k == 0 { 1 } else { 1usize << (k - 1) };
                for i in 0..n {
                    let c = cx[k][i];
                    let r = Rect::from_center_size(pos2(c, y_of(k) + th / 2.0), vec2(tw, th));
                    let (a, b) = couple(k, i);
                    // the child below this couple: which tile and which half
                    if k >= 1 {
                        let (bx, by) = if k == 1 {
                            (cx[0][0] - tw / 4.0, y_of(0))
                        } else {
                            (cx[k - 1][i / 2] + if i % 2 == 0 { -tw / 4.0 } else { tw / 4.0 }, y_of(k - 1))
                        };
                        let mid = r.max.y + gy / 2.0;
                        painter.add(egui::Shape::line(vec![pos2(c, r.max.y), pos2(c, mid), pos2(bx, mid), pos2(bx, by)], line));
                    }
                    let child = if k == 0 { None } else if k == 1 { Some(sel) } else { slots[k - 1][i] };
                    if k >= 1 && child.is_none() {
                        painter.rect(r, 10.0, box_c.gamma_multiply(0.4), edge);
                        continue;
                    }
                    let is_sel = k == 0;
                    crate::views::card(&painter, r, 10.0, if is_sel { blend(accent, box_c, 0.25) } else { box_c }, if is_sel { Stroke::new(2.0_f32, accent) } else { edge }, ui.visuals().dark_mode);
                    for (h, who) in [(0usize, a), (1usize, b)] {
                        let half = Rect::from_min_size(pos2(r.min.x + h as f32 * tw / 2.0, r.min.y), vec2(tw / 2.0, th));
                        let side = if k == 0 { None } else if k == 1 { Some(h == 0) } else { Some(i < n / 2) };
                        match who.and_then(|id| self.db.people.get(&id).map(|p| (id, p))) {
                            Some((id, p)) => {
                                let hit = self.ped_person_hit(ui, half, id, ("up", k, i, h), act);
                                if let Some(t) = self.ped_tint(&p.sex, side) {
                                    painter.rect_filled(half.shrink(3.0), 8.0, blend(t, box_c, 0.14));
                                }
                                if hit.hovered() {
                                    painter.rect_filled(half.shrink(3.0), 8.0, hover_c.gamma_multiply(0.6));
                                }
                                let d = 50.0 * z;
                                let face = Rect::from_center_size(pos2(half.center().x, half.min.y + 8.0 * z + d / 2.0), vec2(d, d));
                                self.ped_face(ui, face, p, true, hover_c, head);
                                let tr = Rect::from_min_max(pos2(half.min.x + 4.0, face.max.y + 2.0), pos2(half.max.x - 4.0, half.max.y - 2.0));
                                let pc = painter.with_clip_rect(tr);
                                // first name and surname, shortened to fit; the years underneath
                                let first = p.given.split_whitespace().next().unwrap_or("");
                                let short = if first.is_empty() { styled_name(p) } else { format!("{first} {}", p.surname) };
                                let fit = ((tr.width() / (6.8 * z)) as usize).max(6);
                                pc.text(pos2(tr.center().x, tr.min.y + 9.0 * z), Align2::CENTER_CENTER, crate::charts::short_name(&short, fit), FontId::proportional(12.5 * z), ink);
                                pc.text(pos2(tr.center().x, tr.min.y + 25.0 * z), Align2::CENTER_CENTER, life(p), FontId::proportional(11.0 * z), head);
                            }
                            None => {
                                let hit = ui.interact(half, egui::Id::new(("up_add", k, i, h)), Sense::click());
                                let label = if k == 0 {
                                    crate::i18n::tr("+ Add spouse")
                                } else if h == 0 {
                                    crate::i18n::tr("+ Add father")
                                } else {
                                    crate::i18n::tr("+ Add mother")
                                };
                                painter.text(half.center(), Align2::CENTER_CENTER, label, FontId::proportional(12.0 * z), if hit.hovered() { ink } else { head });
                                if hit.clicked() {
                                    *act = Some(match (k, h, child) {
                                        (0, _, _) => Action::AddFor(sel, Kind::Spouse),
                                        (_, 0, Some(c)) => Action::AddFor(c, Kind::Father),
                                        (_, _, Some(c)) => Action::AddFor(c, Kind::Mother),
                                        _ => Action::AddFor(sel, Kind::Spouse),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            // children underneath
            let y0 = y_of(0) + th + gy * 0.8;
            let n = kids.len() + 1;
            let row_w = n as f32 * (kid_w + 10.0) - 10.0;
            let x0 = (cx[0][0] - row_w / 2.0).max(o.x + 10.0);
            if !kids.is_empty() {
                let mid = y0 - gy * 0.4;
                painter.add(egui::Shape::line_segment([pos2(cx[0][0], y_of(0) + th), pos2(cx[0][0], mid)], line));
                painter.add(egui::Shape::line_segment([pos2(x0 + kid_w / 2.0, mid), pos2(x0 + (kids.len() - 1) as f32 * (kid_w + 10.0) + kid_w / 2.0, mid)], line));
            }
            for (j, k) in kids.iter().enumerate() {
                let Some(p) = self.db.people.get(k) else { continue };
                let r = Rect::from_min_size(pos2(x0 + j as f32 * (kid_w + 10.0), y0), vec2(kid_w, 46.0 * z));
                painter.add(egui::Shape::line_segment([pos2(r.center().x, y0 - gy * 0.4), pos2(r.center().x, y0)], line));
                let hit = self.ped_person_hit(ui, r, *k, ("upk", j), act);
                crate::views::card(&painter, r, 8.0, self.ped_fill(if hit.hovered() { hover_c } else { box_c }, &p.sex, None), edge, ui.visuals().dark_mode);
                let face = Rect::from_min_size(r.min + vec2(5.0, 5.0) * z, vec2(36.0, 36.0) * z);
                self.ped_face(ui, face, p, true, hover_c, head);
                let pc = painter.with_clip_rect(r.shrink(2.0));
                pc.text(pos2(face.max.x + 5.0, r.center().y - 7.0 * z), Align2::LEFT_CENTER, styled_name(p), FontId::proportional(12.0 * z), ink);
                pc.text(pos2(face.max.x + 5.0, r.center().y + 9.0 * z), Align2::LEFT_CENTER, life(p), FontId::proportional(10.5 * z), head);
            }
            let r = Rect::from_min_size(pos2(x0 + kids.len() as f32 * (kid_w + 10.0), y0), vec2(kid_w, 46.0 * z));
            let hit = ui.interact(r, egui::Id::new("up_add_child"), Sense::click());
            painter.rect(r, 8.0, if hit.hovered() { hover_c } else { box_c.gamma_multiply(0.45) }, edge);
            painter.text(r.center(), Align2::CENTER_CENTER, crate::i18n::tr("+ Add child"), FontId::proportional(12.0 * z), if hit.hovered() { ink } else { head });
            if hit.clicked() {
                *act = Some(Action::AddFor(sel, Kind::Child(None)));
            }
        });
        self.float_bar(ui, view, act);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pedigree_style_draws() {
        let db = crate::sample::sample_family();
        let mut app = App::for_test(db);
        app.sel = app.db.home.or_else(|| app.db.people.keys().next().copied());
        let ctx = egui::Context::default();
        for style in 0..5u8 {
            for colours in 0..3u8 {
                for gens in [3usize, 5, 10] {
                    app.settings.ped_band = gens == 5;
                    app.settings.dark = gens != 10;
                    app.settings.ped_style = style;
                    app.settings.ped_colours = colours;
                    app.settings.gens = gens;
                    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
                    ctx.set_visuals(if app.settings.dark { egui::Visuals::dark() } else { egui::Visuals::light() });
                    let _ = ctx.run(input, |ctx| {
                        let mut act = None;
                        egui::CentralPanel::default().show(ctx, |ui| app.pedigree(ui, &mut act));
                    });
                }
            }
        }
        // the backdrop and the cards are clearly different, in both modes
        for dark in [true, false] {
            let ui_ctx = egui::Context::default();
            ui_ctx.set_visuals(if dark { egui::Visuals::dark() } else { egui::Visuals::light() });
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
            let _ = ui_ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let (_, _, card, _, ink, _, _) = app.chart_colors(ui);
                    let stage = app.chart_stage(ui);
                    let lum = |c: Color32| 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
                    assert!((lum(card) - lum(stage)).abs() > 18.0, "cards stand out from the backdrop (dark {})", dark);
                    assert!((lum(ink) - lum(card)).abs() > 120.0, "names are easy to read on the cards (dark {})", dark);
                });
            });
        }
    }
}
