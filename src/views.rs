//! The main screens: menus, navigation, pedigree and family views, charts.

use super::*;

/// The Select / Hand choice on the chart toolbars. Select: click people as usual. Hand: drag to
/// move the chart around (the chart can also be moved by its scroll bars or the mouse wheel).
pub(crate) fn hand_switch(ui: &mut egui::Ui, hand: bool, act: &mut Option<Action>) {
    if ui.selectable_label(!hand, "Select").on_hover_text("Click a person to go to them (H switches to the hand)").clicked() && hand {
        *act = Some(Action::SetHand(false));
    }
    if ui.selectable_label(hand, "Hand").on_hover_text("Drag the chart to move around it (H switches back to Select)").clicked() && !hand {
        *act = Some(Action::SetHand(true));
    }
    // H flips between the two, unless someone is typing
    if !ui.ctx().wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::H) && i.modifiers.is_none()) {
        *act = Some(Action::SetHand(!hand));
    }
}

/// The hand tool itself: everything under `rect` becomes a surface to grab and drag.
pub(crate) fn hand_overlay(ui: &mut egui::Ui, rect: Rect, id: &str) {
    let r = ui.interact(rect, egui::Id::new(("hand_tool", id)), Sense::click_and_drag());
    if r.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        ui.scroll_with_delta(r.drag_delta());
    } else if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
}

/// The number a person had before the tree was last renumbered, if it was.
pub(crate) fn old_id(p: &Person) -> Option<String> {
    p.extra.iter().filter(|r| r.at.is_empty() && r.text.contains("\n1 TYPE Old ID")).find_map(|r| r.text.lines().next().and_then(|l| l.strip_prefix("0 REFN ")).map(|v| v.trim().to_string()))
}

pub(crate) fn zoom_controls(ui: &mut egui::Ui, z: f32, act: &mut Option<Action>) {
    ui.label("Zoom:");
    if ui.add_enabled(z > 0.26, egui::Button::new("-")).clicked() {
        *act = Some(Action::SetZoom((z - if z > 0.65 { 0.1 } else { 0.05 }).max(0.25)));
    }
    if ui.button(format!("{}%", (z * 100.0).round() as i32)).clicked() {
        *act = Some(Action::SetZoom(1.0));
    }
    if ui.add_enabled(z < 1.79, egui::Button::new("+")).clicked() {
        *act = Some(Action::SetZoom((z + if z >= 0.6 { 0.1 } else { 0.05 }).min(1.8)));
    }
    if ui.button("Fit").on_hover_text("Fit the whole chart in the window").clicked() {
        *act = Some(Action::Fit);
    }
}

impl App {
pub(crate)     fn chart_colors(&self, ui: &egui::Ui) -> (Stroke, Stroke, Color32, Color32, Color32, Color32, Color32) {
        let dark = ui.visuals().dark_mode;
        let bg = self.settings.bg;
        let rgb = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
        let (line_c, edge_c, box_c, hover_c, ink_c, head_c) = if dark {
            (shift(bg, 60), shift(bg, 38), shift(bg, 12), shift(bg, 24), rgb(230, 232, 235), shift(bg, 120))
        } else {
            (rgb(150, 155, 162), rgb(184, 188, 194), Color32::WHITE, rgb(245, 247, 250), rgb(31, 35, 40), rgb(90, 96, 105))
        };
        let accent = Color32::from_rgb(self.settings.accent[0], self.settings.accent[1], self.settings.accent[2]);
        (Stroke::new(1.0_f32, line_c), Stroke::new(1.0_f32, edge_c), box_c, hover_c, ink_c, head_c, accent)
    }

pub(crate)     fn descendants(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let sel = self.sel.unwrap();
        let z = self.settings.zoom;
        let maxd = self.settings.gens.clamp(3, 8);
        self.nav_bar(ui, act);
        ui.horizontal(|ui| {
            ui.label("Generations:");
            if ui.add_enabled(maxd > 3, egui::Button::new("-")).clicked() {
                *act = Some(Action::SetGens(maxd - 1));
            }
            ui.label(maxd.to_string());
            if ui.add_enabled(maxd < 8, egui::Button::new("+")).clicked() {
                *act = Some(Action::SetGens(maxd + 1));
            }
            ui.separator();
            zoom_controls(ui, z, act);
            ui.separator();
            hand_switch(ui, self.settings.hand, act);
            ui.separator();
            if ui.button("Print").clicked() {
                *act = Some(Action::PrintDescendants);
            }
        });
        let mut nodes: Vec<(u32, usize, f32, Option<usize>)> = Vec::new();
        let mut leaf = 0.0f32;
        let mut seen: HashSet<u32> = HashSet::from([sel]);
        let ix = self.db.index();
        desc_layout(&self.db, &ix, sel, 0, maxd, None, &mut nodes, &mut leaf, &mut seen);
        let rows = leaf.max(1.0);
        if self.fit_req {
            let av = ui.available_size();
            let zw = (av.x - 24.0) / (maxd as f32 * (250.0 + 34.0));
            let zh = (av.y - 20.0) / (rows * 46.0 + 20.0);
            *act = Some(Action::SetZoom(zw.min(zh).clamp(0.25, 1.8)));
        }
        if nodes.len() == 1 {
            ui.label("No children recorded yet. Open the Individual tab and use Add child.");
        }
        let (bw, bh, pitch, gap, top) = (250.0f32 * z, 40.0f32 * z, 46.0f32 * z, 34.0f32 * z, 10.0f32 * z);
        let tw = maxd as f32 * (bw + gap);
        let th = rows * pitch + top + 10.0;
        let (line, edge, box_c, hover_c, ink_c, _head_c, accent) = self.chart_colors(ui);
        egui::ScrollArea::both().show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(vec2(tw + 12.0, th), Sense::hover());
            let o = resp.rect.min + vec2(6.0, 0.0);
            if resp.hovered() {
                let zd = ui.input(|i| i.zoom_delta());
                if (zd - 1.0).abs() > 0.001 {
                    *act = Some(Action::SetZoom((z * zd).clamp(0.25, 1.8)));
                }
            }
            let vis = ui.clip_rect().expand(40.0);
            for n in &nodes {
                if let Some(pi) = n.3 {
                    let pn = &nodes[pi];
                    let cy0 = o.y + top + (n.2 + 0.5) * pitch;
                    let py0 = o.y + top + (pn.2 + 0.5) * pitch;
                    if cy0.max(py0) < vis.min.y || cy0.min(py0) > vis.max.y {
                        continue;
                    }
                    let px = o.x + pn.1 as f32 * (bw + gap) + bw;
                    let py = o.y + top + (pn.2 + 0.5) * pitch;
                    let cx = o.x + n.1 as f32 * (bw + gap);
                    let cy = o.y + top + (n.2 + 0.5) * pitch;
                    painter.add(egui::Shape::line(vec![pos2(px, py), pos2(px + gap / 2.0, py), pos2(px + gap / 2.0, cy), pos2(cx, cy)], line));
                }
            }
            for (i, n) in nodes.iter().enumerate() {
                let x = o.x + n.1 as f32 * (bw + gap);
                let cy = o.y + top + (n.2 + 0.5) * pitch;
                let rect = Rect::from_min_size(pos2(x, cy - bh / 2.0), vec2(bw, bh));
                if !rect.intersects(vis) {
                    continue;
                }
                let r2 = ui.interact(rect, egui::Id::new(("desc", i)), Sense::click());
                let is_root = i == 0;
                let is_hl = self.hl == Some(n.0) && !is_root;
                let fill = if is_root {
                    accent
                } else if is_hl {
                    accent.gamma_multiply(0.35)
                } else if r2.hovered() {
                    hover_c
                } else {
                    box_c
                };
                let ink = if is_root { on_accent(accent) } else { ink_c };
                painter.rect(rect, 5.0_f32, fill, edge);
                if is_root {
                    painter.rect_stroke(rect.expand(2.5), 7.0_f32, Stroke::new(2.5_f32, ink_c));
                }
                if is_hl {
                    painter.rect_stroke(rect, 5.0_f32, Stroke::new(3.0_f32, accent));
                }
                let (l1, l2) = desc_lines(&self.db, n.0);
                let clip = painter.with_clip_rect(rect.shrink(2.0));
                match &l2 {
                    Some(l2) => {
                        clip.text(pos2(rect.min.x + 6.0, rect.min.y + bh * 0.30), Align2::LEFT_CENTER, l1, FontId::proportional(13.0 * z), ink);
                        clip.text(pos2(rect.min.x + 6.0, rect.min.y + bh * 0.74), Align2::LEFT_CENTER, l2, FontId::proportional(11.5 * z), ink.gamma_multiply(0.72));
                    }
                    None => {
                        clip.text(pos2(rect.min.x + 6.0, rect.center().y), Align2::LEFT_CENTER, l1, FontId::proportional(13.0 * z), ink);
                    }
                }
                let r2 = r2.on_hover_ui(|ui| {
                    let (a, b) = desc_lines(&self.db, n.0);
                    ui.label(a);
                    if let Some(b) = b {
                        ui.label(b);
                    }
                });
                let nid = n.0;
                r2.context_menu(|ui| self.person_menu(ui, nid, act));
                if r2.double_clicked() {
                    *act = Some(Action::Open(nid));
                } else if r2.clicked() {
                    *act = Some(Action::Highlight(nid));
                }
            }
            // hand tool: a layer over the whole chart that takes every click and drag, so dragging
            // anywhere moves the chart and nothing is selected by accident
            if self.settings.hand {
                hand_overlay(ui, resp.rect, "desc");
            }
        });
    }
}

impl App {
    /// The actions for one person in a chart: open, view tree, quick edit, add a relative.
pub(crate)     fn person_menu(&self, ui: &mut egui::Ui, id: u32, act: &mut Option<Action>) {
        if ui.button("Open individual page").clicked() {
            *act = Some(Action::Open(id));
            ui.close_menu();
        }
        ui.menu_button("View their tree", |ui| {
            for (lbl, w) in [("Ancestors (pedigree)", 0u8), ("Descendants", 1), ("Family", 2)] {
                if ui.button(lbl).clicked() {
                    *act = Some(Action::ViewTree(id, w));
                    ui.close_menu();
                }
            }
        });
        if ui.button("Quick edit...").clicked() {
            *act = Some(Action::QuickEdit(id));
            ui.close_menu();
        }
        ui.menu_button("Add a relative", |ui| {
            let (f, m) = self.db.parents(id);
            let first_fam = self.db.fams_of(id).first().copied();
            let items: [(&str, Kind, bool); 6] = [
                ("Father...", Kind::Father, f.is_none()),
                ("Mother...", Kind::Mother, m.is_none()),
                ("Another set of parents...", Kind::NewParent(true), f.is_some() || m.is_some()),
                ("Brother or sister...", Kind::Sibling, true),
                ("Spouse or partner...", Kind::Spouse, true),
                ("Child...", Kind::Child(first_fam), true),
            ];
            for (lbl, k, on) in items {
                if ui.add_enabled(on, egui::Button::new(lbl)).clicked() {
                    *act = Some(Action::AddFor(id, k));
                    ui.close_menu();
                }
            }
        });
        let sets = self.db.parent_families(id);
        if sets.len() > 1 {
            ui.menu_button("Show which parents", |ui| {
                let shown = self.db.famc(id);
                for f in &sets {
                    let fm = &self.db.fams[f];
                    let bio = rel_is_bio(&fm.rel_to(id, true)) && rel_is_bio(&fm.rel_to(id, false));
                    let names: Vec<String> = [fm.h, fm.w].into_iter().flatten().filter(|x| self.db.people.contains_key(x)).map(|x| self.db.label(x)).collect();
                    let label = format!("{}{}: {}", if shown == Some(*f) { "* " } else { "" }, if bio { "Biological" } else { "Other" }, if names.is_empty() { "(no names)".to_string() } else { names.join(" & ") });
                    if ui.selectable_label(shown == Some(*f), label).clicked() {
                        *act = Some(Action::ShowParentsFor(id, *f));
                        ui.close_menu();
                    }
                }
            });
        }
        ui.separator();
        if ui.button("Centre chart on this person").clicked() {
            *act = Some(Action::Select(id));
            ui.close_menu();
        }
        if ui.button("Make this the home person").clicked() {
            *act = Some(Action::SetHome(id));
            ui.close_menu();
        }
        let marked = self.db.bookmarks.contains(&id);
        if ui.button(if marked { "Remove bookmark" } else { "Bookmark this person" }).clicked() {
            *act = Some(Action::Bookmark(id));
            ui.close_menu();
        }
    }

    /// Spouse(s) and children of one person, as a drop-down list.
pub(crate)     fn spouse_children_menu(&self, ui: &mut egui::Ui, id: u32, act: &mut Option<Action>) {
        let fams = self.db.fams_of(id);
        for fid in &fams {
            let fm = &self.db.fams[fid];
            let other = if fm.h == Some(id) { fm.w } else { fm.h };
            if let Some(o) = other.filter(|o| self.db.people.contains_key(o)) {
                if ui.button(format!("+ {}", self.db.label(o))).clicked() {
                    *act = Some(Action::Select(o));
                    ui.close_menu();
                }
            }
            for k in &fm.kids {
                if !self.db.people.contains_key(k) {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    if ui.button(format!("{}{}", self.db.label(*k), self.db.child_note(*fid, *k))).clicked() {
                        *act = Some(Action::Select(*k));
                        ui.close_menu();
                    }
                });
            }
        }
        if fams.is_empty() {
            ui.weak("No spouse or children recorded");
        }
        ui.separator();
        if ui.button("Add spouse...").clicked() {
            *act = Some(Action::AddFor(id, Kind::Spouse));
            ui.close_menu();
        }
        if ui.button("Add child...").clicked() {
            *act = Some(Action::AddFor(id, Kind::Child(fams.first().copied())));
            ui.close_menu();
        }
    }

pub(crate)     fn siblings_menu(&self, ui: &mut egui::Ui, id: u32, act: &mut Option<Action>) {
        let sibs = self.db.siblings(id);
        for k in &sibs {
            if ui.button(self.db.label(*k)).clicked() {
                *act = Some(Action::Select(*k));
                ui.close_menu();
            }
        }
        if sibs.is_empty() {
            ui.weak("No brothers or sisters recorded");
        }
        ui.separator();
        if ui.button("Add brother or sister...").clicked() {
            *act = Some(Action::AddFor(id, Kind::Sibling));
            ui.close_menu();
        }
    }

    /// Apply the Quick edit window to the tree.
pub(crate)     fn apply_quick(&mut self, q: &QuickEdit) {
        if let Some(p) = self.db.people.get_mut(&q.id) {
            p.given = q.given.trim().to_string();
            p.surname = q.surname.trim().to_string();
            p.name = compose_name(&p.given, &p.surname);
            p.sex = q.sex.clone();
            p.bd = q.bd.trim().to_string();
            p.bp = q.bp.trim().to_string();
            p.dd = q.dd.trim().to_string();
            p.dp = q.dp.trim().to_string();
            p.living = q.living;
        }
    }

    /// Where an arrow key goes from the selected person: right = father (Shift: mother),
    /// left = a child (the one you just came from, if any), up/down = previous/next sibling.
pub(crate)     fn nav_target(&self, key: egui::Key, shift: bool) -> Option<u32> {
        let sel = self.sel?;
        match key {
            egui::Key::ArrowRight => {
                let (f, m) = self.db.parents(sel);
                if shift { m.or(f) } else { f.or(m) }
            }
            egui::Key::ArrowLeft => {
                let kids = self.db.children_of(sel);
                match self.back.last() {
                    Some(b) if kids.contains(b) => Some(*b),
                    _ => kids.first().copied(),
                }
            }
            egui::Key::ArrowUp | egui::Key::ArrowDown => {
                let sibs = self.db.siblings(sel);
                if sibs.is_empty() {
                    return None;
                }
                let mut all = sibs;
                if let Some(f) = self.db.famc(sel) {
                    // keep the family's own order, with the selected person in place
                    let own = &self.db.fams[&f].kids;
                    all.retain(|x| !own.contains(x));
                    let mut ordered: Vec<u32> = own.iter().copied().filter(|x| self.db.people.contains_key(x)).collect();
                    ordered.extend(all);
                    all = ordered;
                } else {
                    all.push(sel);
                }
                let pos = all.iter().position(|x| *x == sel)?;
                if key == egui::Key::ArrowUp {
                    pos.checked_sub(1).map(|p| all[p])
                } else {
                    all.get(pos + 1).copied()
                }
            }
            _ => None,
        }
    }

    /// The people you have been looking at, oldest first, ending with the current one.
pub(crate)     fn trail(&self) -> Vec<u32> {
        let mut seen: Vec<u32> = Vec::new();
        let cur = self.sel;
        for p in self.back.iter().rev() {
            if Some(*p) != cur && self.db.people.contains_key(p) && !seen.contains(p) {
                seen.push(*p);
                if seen.len() >= 6 {
                    break;
                }
            }
        }
        seen.reverse();
        if let Some(c) = cur {
            seen.push(c);
        }
        seen
    }

pub(crate)     fn trail_bar(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let t = self.trail();
        if t.len() < 2 {
            return;
        }
        let accent = Color32::from_rgb(self.settings.accent[0], self.settings.accent[1], self.settings.accent[2]);
        let short = |s: &str| -> String {
            let c: Vec<char> = s.chars().collect();
            if c.len() > 22 { format!("{}...", c[..21].iter().collect::<String>()) } else { s.to_string() }
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Trail").small().weak());
            let last = t.len() - 1;
            for (n, id) in t.iter().enumerate() {
                let name = short(&display_name(&self.db.people[id]));
                if n == last {
                    ui.label(RichText::new(name).strong().color(accent));
                } else {
                    if ui.link(name).on_hover_text("Go back to this person").clicked() {
                        *act = Some(Action::Select(*id));
                    }
                    ui.label(RichText::new(">").weak());
                }
            }
        });
    }

    /// Back, forward, home, and one-click moves to parents, siblings, partners and children.
pub(crate)     fn nav_bar(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let Some(sel) = self.sel else { return };
        let pick = |ui: &mut egui::Ui, act: &mut Option<Action>, ids: &[u32]| {
            for k in ids {
                if ui.button(self.db.label(*k)).clicked() {
                    *act = Some(Action::Select(*k));
                    ui.close_menu();
                }
            }
        };
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!self.back.is_empty(), egui::Button::new("< Back")).clicked() {
                *act = Some(Action::Back);
            }
            if ui.add_enabled(!self.fwd.is_empty(), egui::Button::new("Forward >")).clicked() {
                *act = Some(Action::Forward);
            }
            let hb = ui.button("Home").on_hover_text("Go to the home person. Right-click to make the current person the home person.");
            if hb.clicked() {
                *act = Some(Action::GoHome);
            }
            hb.context_menu(|ui| {
                if ui.button("Make the current person the home person").clicked() {
                    *act = Some(Action::SetHome(sel));
                    ui.close_menu();
                }
            });
            let mut recent: Vec<u32> = Vec::new();
            for k in self.back.iter().rev() {
                if *k != sel && self.db.people.contains_key(k) && !recent.contains(k) {
                    recent.push(*k);
                }
                if recent.len() >= 15 {
                    break;
                }
            }
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                ui.menu_button("Recent", |ui| pick(ui, act, &recent));
            });
            let marked = self.db.bookmarks.contains(&sel);
            ui.menu_button(if marked { "Bookmarks \u{2605}" } else { "Bookmarks" }, |ui| {
                if ui.button(if marked { "Remove bookmark for this person" } else { "Bookmark this person" }).clicked() {
                    *act = Some(Action::Bookmark(sel));
                    ui.close_menu();
                }
                let list: Vec<u32> = self.db.bookmarks.iter().copied().filter(|k| self.db.people.contains_key(k)).collect();
                if !list.is_empty() {
                    ui.separator();
                    pick(ui, act, &list);
                }
            });
            ui.separator();
            let sets = self.db.parent_families(sel);
            ui.add_enabled_ui(!sets.is_empty(), |ui| {
                ui.menu_button("Parents", |ui| {
                    for f in &sets {
                        let fm = &self.db.fams[f];
                        let mut ids: Vec<u32> = Vec::new();
                        for p in fm.h.iter().chain(fm.w.iter()) {
                            if self.db.people.contains_key(p) {
                                ids.push(*p);
                            }
                        }
                        if sets.len() > 1 {
                            let shown = self.db.famc(sel) == Some(*f);
                            let bio = rel_is_bio(&fm.rel_to(sel, true)) && rel_is_bio(&fm.rel_to(sel, false));
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(if bio { "Biological parents" } else { "Other parents" }).small().weak());
                                if shown {
                                    ui.label(RichText::new("shown on chart").small().weak());
                                } else if ui.small_button("Show on chart").clicked() {
                                    *act = Some(Action::ShowParents(*f));
                                    ui.close_menu();
                                }
                            });
                        }
                        pick(ui, act, &ids);
                    }
                });
            });
            let sibs = self.db.siblings(sel);
            ui.add_enabled_ui(!sibs.is_empty(), |ui| {
                ui.menu_button("Siblings", |ui| pick(ui, act, &sibs));
            });
            let pt = self.db.partners(sel);
            ui.add_enabled_ui(!pt.is_empty(), |ui| {
                ui.menu_button("Partners", |ui| pick(ui, act, &pt));
            });
            let kids = self.db.children_of(sel);
            ui.add_enabled_ui(!kids.is_empty(), |ui| {
                ui.menu_button("Children", |ui| pick(ui, act, &kids));
            });
        });
    }

pub(crate)     fn goto(&mut self, i: u32) {
        if self.sel != Some(i) {
            if let Some(c) = self.sel {
                self.back.push(c);
            }
            self.fwd.clear();
        }
        self.sel = Some(i);
    }

pub(crate)     fn delete(&mut self) {
        let Some(id) = self.sel else { return };
        self.db.people.remove(&id);
        if self.db.home == Some(id) {
            self.db.home = None;
        }
        for f in self.db.fams.values_mut() {
            if f.h == Some(id) { f.h = None; }
            if f.w == Some(id) { f.w = None; }
            f.kids.retain(|k| *k != id);
            f.more.retain(|(_, x)| *x != id);
            f.pedi.remove(&id);
            f.frel.remove(&id);
            f.mrel.remove(&id);
        }
        replace_pointer(&mut self.db, &format!("I{}", id), None);
        self.db.fams.retain(|_, f| f.h.is_some() || f.w.is_some() || !f.kids.is_empty() || !f.more.is_empty());
        self.sel = self.db.home.or_else(|| self.db.people.keys().next().copied());
        self.confirm = false;
    }

pub(crate)     fn pedigree(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let sel = self.sel.unwrap();
        let gens = self.settings.gens.clamp(3, 8);
        let rows = 1usize << (gens - 1);
        let z = self.settings.zoom;
        let (bh, pitch, gap, top) = (28.0f32 * z, 36.0f32 * z, 28.0f32 * z, 30.0f32 * z);
        let th = rows as f32 * pitch + top;
        let bw = ((ui.available_width() - (gens as f32 - 1.0) * gap - 24.0) / gens as f32).clamp(150.0, 340.0) * z;
        let tw = gens as f32 * bw + (gens as f32 - 1.0) * gap;
        if self.fit_req {
            let av = ui.available_size();
            let bw0 = bw / z;
            let zw = (av.x - 24.0) / (gens as f32 * bw0 + (gens as f32 - 1.0) * 28.0);
            let zh = (av.y - 100.0) / (rows as f32 * 36.0 + 36.0);
            *act = Some(Action::SetZoom(zw.min(zh).clamp(0.25, 1.8)));
        }
        let ix = self.db.index();
        let mut slots: Vec<Vec<Option<u32>>> = vec![vec![Some(sel)]];
        for g in 1..gens {
            let cur = {
                let prev = &slots[g - 1];
                let mut c = Vec::new();
                for s in prev {
                    let (f, m) = match s {
                        Some(i) => self.db.parents_ix(&ix, *i),
                        None => (None, None),
                    };
                    c.push(f);
                    c.push(m);
                }
                c
            };
            slots.push(cur);
        }
        let kids: Vec<(u32, String)> = self
            .db
            .fams_of(sel)
            .iter()
            .flat_map(|f| self.db.fams[f].kids.iter().map(|k| (*k, self.db.label(*k))).collect::<Vec<_>>())
            .collect();
        self.nav_bar(ui, act);
        ui.horizontal(|ui| {
            ui.label("Generations:");
            if ui.add_enabled(gens > 3, egui::Button::new("-")).clicked() {
                *act = Some(Action::SetGens(gens - 1));
            }
            ui.label(gens.to_string());
            if ui.add_enabled(gens < 8, egui::Button::new("+")).clicked() {
                *act = Some(Action::SetGens(gens + 1));
            }
            ui.separator();
            zoom_controls(ui, z, act);
            ui.separator();
            hand_switch(ui, self.settings.hand, act);
            ui.separator();
            if ui.button("Print").clicked() {
                *act = Some(Action::PrintPedigree);
            }
        });
        egui::ScrollArea::both().show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(vec2(tw + 12.0, th + 6.0), Sense::hover());
            let o = resp.rect.min + vec2(6.0, 0.0);
            if resp.hovered() {
                let zd = ui.input(|i| i.zoom_delta());
                if (zd - 1.0).abs() > 0.001 {
                    *act = Some(Action::SetZoom((z * zd).clamp(0.25, 1.8)));
                }
            }
            let dark = ui.visuals().dark_mode;
            let bg = self.settings.bg;
            let rgb = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
            let (line_c, edge_c, box_c, hover_c, ink_c, head_c) = if dark {
                (shift(bg, 60), shift(bg, 38), shift(bg, 12), shift(bg, 24), rgb(230, 232, 235), shift(bg, 120))
            } else {
                (rgb(150, 155, 162), rgb(184, 188, 194), Color32::WHITE, rgb(245, 247, 250), rgb(31, 35, 40), rgb(90, 96, 105))
            };
            let accent = Color32::from_rgb(self.settings.accent[0], self.settings.accent[1], self.settings.accent[2]);
            let black = Stroke::new(1.0_f32, line_c);
            let edge = Stroke::new(1.0_f32, edge_c);
            for g in 0..gens {
                painter.text(
                    pos2(o.x + g as f32 * (bw + gap), o.y + 4.0),
                    Align2::LEFT_TOP,
                    ordinal(g + 1),
                    FontId::proportional(17.0 * z),
                    head_c,
                );
            }
            for g in 0..gens {
                let n = 1usize << g;
                let step = rows as f32 * pitch / n as f32;
                for i in 0..n {
                    let cy = o.y + top + (i as f32 + 0.5) * step;
                    let x = o.x + g as f32 * (bw + gap);
                    let rect = Rect::from_min_size(pos2(x, cy - bh / 2.0), vec2(bw, bh));
                    if g > 0 {
                        let ccy = o.y + top + ((i / 2) as f32 + 0.5) * step * 2.0;
                        painter.add(egui::Shape::line(
                            vec![pos2(x, cy), pos2(x - gap / 2.0, cy), pos2(x - gap / 2.0, ccy), pos2(x - gap, ccy)],
                            black,
                        ));
                    }
                    match slots[g][i] {
                        Some(id) => {
                            let p = &self.db.people[&id];
                            let r2 = ui.interact(rect, egui::Id::new(("ped", g, i)), Sense::click());
                            let is_sel = g == 0;
                            let is_hl = self.hl == Some(id) && !is_sel;
                            let fill = if is_sel {
                                accent
                            } else if is_hl {
                                accent.gamma_multiply(0.35)
                            } else if r2.hovered() {
                                hover_c
                            } else {
                                box_c
                            };
                            let ink = if is_sel { on_accent(accent) } else { ink_c };
                            painter.rect(rect, 5.0_f32, fill, edge);
                            if is_sel {
                                painter.rect_stroke(rect.expand(2.5), 7.0_f32, Stroke::new(2.5_f32, ink_c));
                            }
                            let l = life(p);
                            let mut txt = if l.is_empty() { styled_name(p) } else { format!("{}  {}", styled_name(p), l) };
                            if self.settings.box_places {
                                let pl = if !p.bp.trim().is_empty() { p.bp.trim() } else { p.dp.trim() };
                                if let Some(first) = pl.split(',').next().filter(|x| !x.trim().is_empty()) {
                                    txt.push_str(&format!("  \u{b7} {}", first.trim()));
                                }
                            }
                            if self.settings.box_ids {
                                txt.push_str(&format!("  #{}", id));
                            }
                            let ir = Rect::from_min_size(pos2(rect.min.x + 3.0, rect.min.y + 2.0), vec2(bh - 4.0, bh - 4.0));
                            match self.portrait_src(p).filter(|_| self.settings.box_photos) {
                                Some((uri, fr)) => put_photo(ui, ir, &uri, fr, 3.0),
                                None => {
                                    painter.rect_filled(ir, 3.0_f32, if is_sel { on_accent(accent).gamma_multiply(0.22) } else { hover_c });
                                    let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
                                    painter.text(ir.center(), Align2::CENTER_CENTER, initial, FontId::proportional(14.0 * z), if is_sel { ink } else { head_c });
                                }
                            }
                            let (pf0, pm0) = self.db.parents_ix(&ix, id);
                            let has_arrow = (g == gens - 1 && (pf0.is_some() || pm0.is_some())) || (g == 0 && !kids.is_empty());
                            let right_pad = 34.0 + if has_arrow { 24.0 } else { 0.0 };
                            let clip = Rect::from_min_max(rect.min + vec2(2.0, 2.0), pos2((rect.max.x - right_pad).max(rect.min.x + bh + 20.0), rect.max.y - 2.0));
                            painter.with_clip_rect(clip).text(
                                pos2(rect.min.x + bh + 4.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                txt,
                                FontId::proportional(14.0 * z),
                                ink,
                            );
                            let mz = Rect::from_min_size(pos2(rect.max.x - 32.0, rect.min.y + 3.0), vec2(30.0, bh - 6.0));
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(mz), |ui| {
                                ui.menu_button(RichText::new("...").small(), |ui| self.person_menu(ui, id, act));
                            });
                            if g == gens - 1 {
                                let (pf, pm) = (pf0, pm0);
                                if pf.is_some() || pm.is_some() {
                                    let zone = Rect::from_min_size(pos2(rect.max.x - 56.0, rect.min.y), vec2(24.0, bh));
                                    let ra = ui.interact(zone, egui::Id::new(("arrow", i)), Sense::click());
                                    let c = if ra.hovered() { accent } else { head_c };
                                    let (cx, cyy) = (zone.center().x, zone.center().y);
                                    painter.add(egui::Shape::convex_polygon(
                                        vec![pos2(cx - 3.0, cyy - 6.0), pos2(cx - 3.0, cyy + 6.0), pos2(cx + 5.0, cyy)],
                                        c,
                                        Stroke::NONE,
                                    ));
                                    if ra.on_hover_text("Show this person's ancestors").clicked() {
                                        *act = Some(Action::Select(id));
                                    }
                                }
                            }
                            if g == 0 && !kids.is_empty() {
                                let zone = Rect::from_min_size(pos2(rect.max.x - 56.0, rect.min.y), vec2(24.0, bh));
                                let ra = ui.interact(zone, egui::Id::new("arrow_down"), Sense::click());
                                let c = if ra.hovered() { ink } else { on_accent(accent).gamma_multiply(0.75) };
                                let (cx, cyy) = (zone.center().x, zone.center().y);
                                painter.add(egui::Shape::convex_polygon(
                                    vec![pos2(cx + 3.0, cyy - 6.0), pos2(cx + 3.0, cyy + 6.0), pos2(cx - 5.0, cyy)],
                                    c,
                                    Stroke::NONE,
                                ));
                                let pid = egui::Id::new("kids_popup");
                                let ra = ra.on_hover_text(if kids.len() == 1 { "Go down to this person's child" } else { "Go down to one of this person's children" });
                                if ra.clicked() {
                                    if kids.len() == 1 {
                                        *act = Some(Action::Select(kids[0].0));
                                    } else {
                                        ui.memory_mut(|mm| mm.toggle_popup(pid));
                                    }
                                }
                                egui::popup_below_widget(ui, pid, &ra, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                                    ui.set_min_width(220.0);
                                    for (k, l) in &kids {
                                        if ui.button(l.as_str()).clicked() {
                                            *act = Some(Action::Select(*k));
                                        }
                                    }
                                });
                            }
                            if is_hl {
                                painter.rect_stroke(rect, 5.0_f32, Stroke::new(3.0_f32, accent));
                            }
                            r2.context_menu(|ui| self.person_menu(ui, id, act));
                            if is_sel {
                                let w = rect.width().min(230.0);
                                let n_sc = self.db.partners(id).len() + kids.len();
                                let n_sib = self.db.siblings(id).len();
                                let r_a = Rect::from_min_size(pos2(rect.min.x, rect.max.y + 8.0), vec2(w, 22.0));
                                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r_a), |ui| {
                                    ui.menu_button(format!("Spouse & children ({})", n_sc), |ui| self.spouse_children_menu(ui, id, act));
                                });
                                let r_b = Rect::from_min_size(pos2(rect.min.x, rect.max.y + 36.0), vec2(w, 22.0));
                                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r_b), |ui| {
                                    ui.menu_button(format!("Siblings ({})", n_sib), |ui| self.siblings_menu(ui, id, act));
                                });
                            }
                            if r2.double_clicked() {
                                *act = Some(Action::Open(id));
                            } else if r2.clicked() {
                                *act = Some(Action::Highlight(id));
                            }
                        }
                        None => {
                            let child = if g > 0 { slots[g - 1][i / 2] } else { None };
                            if let Some(child) = child {
                                let r2 = ui.interact(rect, egui::Id::new(("ped", g, i)), Sense::click());
                                let fill = if r2.hovered() { hover_c } else { box_c.gamma_multiply(0.45) };
                                painter.rect(rect, 5.0_f32, fill, edge);
                                if r2.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    let label = if i % 2 == 0 { "+ Add father" } else { "+ Add mother" };
                                    painter.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(13.0), head_c);
                                }
                                if r2.clicked() {
                                    let k = if i % 2 == 0 { Kind::Father } else { Kind::Mother };
                                    *act = Some(Action::AddFor(child, k));
                                }
                            } else {
                                painter.rect(rect, 5.0_f32, box_c.gamma_multiply(0.45), edge);
                            }
                        }
                    }
                }
            }
            // hand tool: a layer over the whole chart that takes every click and drag, so dragging
            // anywhere moves the chart and nothing is selected by accident
            if self.settings.hand {
                hand_overlay(ui, resp.rect, "ped");
            }
        });
    }

    /// A compact, clickable person card: portrait (or initial), name, years and birthplace.
pub(crate)     fn mini_card(&self, ui: &mut egui::Ui, id: u32, tag: &str, current: bool, act: &mut Option<Action>) {
        let Some(p) = self.db.people.get(&id) else { return };
        let (_, edge, box_c, hover_c, ink_c, head_c, accent) = self.chart_colors(ui);
        let w = ui.available_width();
        let frame = egui::Frame::none().fill(box_c).stroke(edge).rounding(6.0).inner_margin(egui::Margin::symmetric(8.0, 6.0));
        let out = frame.show(ui, |ui| {
            ui.set_width((w - 18.0).max(40.0));
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                match self.portrait_src(p) {
                    Some((uri, fr)) => put_photo(ui, r, &uri, fr, 5.0),
                    None => {
                        ui.painter().rect_filled(r, 5.0_f32, hover_c);
                        let initial = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, initial, FontId::proportional(18.0), head_c);
                    }
                }
                ui.vertical(|ui| {
                    if !tag.is_empty() {
                        ui.label(RichText::new(tag).small().weak());
                    }
                    ui.add(egui::Label::new(RichText::new(display_name(p)).strong().color(ink_c)).truncate());
                    let l = life(p);
                    if !l.is_empty() {
                        ui.label(RichText::new(l).weak());
                    }
                    if !p.bp.is_empty() {
                        ui.add(egui::Label::new(RichText::new(p.bp.as_str()).small().weak()).truncate());
                    }
                });
            });
        });
        let rect = out.response.rect;
        let r = ui.interact(rect, out.response.id.with("click"), Sense::click());
        if current {
            ui.painter().rect_stroke(rect, 6.0_f32, Stroke::new(2.0_f32, accent));
        } else if r.hovered() {
            ui.painter().rect_stroke(rect, 6.0_f32, Stroke::new(1.5_f32, accent.gamma_multiply(0.8)));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if r.clicked() && !current {
            *act = Some(Action::Select(id));
        }
    }

    /// The couple, the marriage and the children of one family: the left side of the Family tab.
pub(crate)     fn family_main(&self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, ids: &[u32], fx: usize) {
        let fid = ids[fx];
        let f = &self.db.fams[&fid];
        if ids.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                for (i, fid2) in ids.iter().enumerate() {
                    let fm = &self.db.fams[fid2];
                    let other = if fm.h == Some(id) { fm.w } else { fm.h };
                    let mut label = other.and_then(|o| self.db.people.get(&o)).map(|p| p.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Family {}", i + 1));
                    let y = year(&fm.md);
                    if !y.is_empty() {
                        label.push_str(&format!(" ({})", y));
                    }
                    if ui.selectable_label(i == fx, label).clicked() {
                        *act = Some(Action::Fx(i));
                    }
                }
            });
            ui.add_space(4.0);
        }
        ui.columns(2, |cols| {
            for (n, (slot, tag)) in [(f.h, "Husband"), (f.w, "Wife")].into_iter().enumerate() {
                match slot {
                    Some(pid) => self.mini_card(&mut cols[n], pid, tag, pid == id, act),
                    None => {
                        cols[n].group(|ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(tag).small().weak());
                            ui.weak("not recorded");
                        });
                    }
                }
            }
        });
        ui.add_space(4.0);
        let mut m = Vec::new();
        if !f.kind.is_empty() {
            m.push(f.kind.clone());
        }
        if !f.md.is_empty() {
            m.push(f.md.clone());
        }
        if !f.mp.is_empty() {
            m.push(f.mp.clone());
        }
        ui.vertical_centered(|ui| {
            if m.is_empty() {
                ui.weak("Marriage details not recorded");
            } else {
                ui.label(m.join("  -  "));
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new(format!("Children ({})", f.kids.len())).strong());
        if f.kids.is_empty() {
            ui.weak("No children recorded");
        } else {
            egui::Grid::new("kids").num_columns(4).spacing([22.0, 5.0]).show(ui, |ui| {
                ui.label(RichText::new("Name").small().weak());
                ui.label(RichText::new("Born").small().weak());
                ui.label(RichText::new("Died").small().weak());
                ui.label(RichText::new("Parent link").small().weak());
                ui.end_row();
                for k in &f.kids {
                    let Some(c) = self.db.people.get(k) else { continue };
                    ui.horizontal(|ui| {
                        if ui.link(display_name(c)).clicked() {
                            *act = Some(Action::Select(*k));
                        }
                        let n = self.db.children_of(*k).len();
                        if n > 0 {
                            ui.weak(format!("{} {}", n, if n == 1 { "child" } else { "children" }));
                        }
                    });
                    ui.label(if c.bp.is_empty() { c.bd.clone() } else { format!("{}  {}", c.bd, c.bp).trim().to_string() });
                    ui.label(c.dd.as_str());
                    let note = self.db.child_note(fid, *k);
                    ui.label(if note.is_empty() { "Birth".to_string() } else { note.trim().trim_matches(|c| c == '(' || c == ')').to_string() });
                    ui.end_row();
                }
            });
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Add child").clicked() {
                *act = Some(Action::Add(Kind::Child(Some(fid))));
            }
            if ui.button("Print family group sheet").clicked() {
                *act = Some(Action::PrintFamily);
            }
        });
    }

    /// Parents, siblings, partners and other relationships of the selected person: the right side.
pub(crate)     fn family_side(&self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, fx: usize) {
        ui.label(RichText::new("Relationships").strong().size(16.0));
        ui.add_space(4.0);
        let sets = self.db.parent_families(id);
        if sets.is_empty() {
            ui.weak("No parents recorded");
            ui.horizontal(|ui| {
                if ui.small_button("Add father").clicked() {
                    *act = Some(Action::Add(Kind::Father));
                }
                if ui.small_button("Add mother").clicked() {
                    *act = Some(Action::Add(Kind::Mother));
                }
            });
        }
        for f in &sets {
            let fm = &self.db.fams[f];
            for (slot, father) in [(fm.h, true), (fm.w, false)] {
                let Some(pid) = slot else { continue };
                let rel = fm.rel_to(id, father);
                let role = if father { "father" } else { "mother" };
                let tag = if sets.len() == 1 && rel_is_bio(&rel) {
                    if father { "Father".to_string() } else { "Mother".to_string() }
                } else if rel_is_bio(&rel) {
                    format!("Biological {}", role)
                } else {
                    format!("{} {}", rel_name(&rel), role)
                };
                self.mini_card(ui, pid, &tag, false, act);
                ui.add_space(3.0);
            }
        }
        let sibs = self.db.siblings(id);
        if !sibs.is_empty() {
            egui::CollapsingHeader::new(format!("Siblings ({})", sibs.len())).id_salt("fam_sibs").default_open(false).show(ui, |ui| {
                let mine = self.db.parent_families(id);
                for sid in &sibs {
                    let half = !mine.iter().any(|f| self.db.fams[f].kids.contains(sid) && self.db.fams[f].h.is_some() && self.db.fams[f].w.is_some());
                    let Some(p) = self.db.people.get(sid) else { continue };
                    ui.horizontal(|ui| {
                        if ui.link(display_name(p)).clicked() {
                            *act = Some(Action::Select(*sid));
                        }
                        let mut t = life(p);
                        if half {
                            t = format!("{} half", t).trim().to_string();
                        }
                        ui.weak(t);
                    });
                }
            });
        }
        ui.add_space(6.0);
        ui.label(RichText::new("Partners and children").strong());
        let fams = self.db.fams_of(id);
        if fams.is_empty() {
            ui.weak("No partner recorded");
            if ui.small_button("Add spouse").clicked() {
                *act = Some(Action::Add(Kind::Spouse));
            }
        }
        for (i, fid) in fams.iter().enumerate() {
            let fm = &self.db.fams[fid];
            let other = if fm.h == Some(id) { fm.w } else { fm.h };
            let y = year(&fm.md);
            let mut tag = if fm.kind.is_empty() { "Partner".to_string() } else { fm.kind.clone() };
            if !y.is_empty() {
                tag.push_str(&format!(" {}", y));
            }
            match other {
                Some(o) => self.mini_card(ui, o, &tag, false, act),
                None => {
                    ui.weak("Partner not recorded");
                }
            }
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    for k in &fm.kids {
                        let Some(c) = self.db.people.get(k) else { continue };
                        ui.horizontal(|ui| {
                            if ui.link(display_name(c)).clicked() {
                                *act = Some(Action::Select(*k));
                            }
                            ui.weak(life(c));
                        });
                    }
                    if fams.len() > 1 && i != fx && ui.small_button("Show this family").clicked() {
                        *act = Some(Action::Fx(i));
                    }
                });
            });
            ui.add_space(4.0);
        }
        let assoc: Vec<(u32, String)> = self.db.people.get(&id).map(|p| p.assoc.iter().map(|a| (a.other, a.role.clone())).collect()).unwrap_or_default();
        let assoc_of = self.db.kin_extras(id).assoc_of;
        if !assoc.is_empty() || !assoc_of.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new("Other relationships").strong());
            for (o, role) in &assoc {
                if let Some(p) = self.db.people.get(o) {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(role.as_str());
                        if ui.link(display_name(p)).clicked() {
                            *act = Some(Action::Select(*o));
                        }
                    });
                }
            }
            for (o, role) in &assoc_of {
                if let Some(p) = self.db.people.get(o) {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(format!("is the {} of", role));
                        if ui.link(display_name(p)).clicked() {
                            *act = Some(Action::Select(*o));
                        }
                    });
                }
            }
        }
    }

pub(crate)     fn family_view(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let id = self.sel.unwrap();
        self.nav_bar(ui, act);
        let mut ids = self.db.fams_of(id);
        if ids.is_empty() {
            if let Some(f) = self.db.famc(id) {
                ids.push(f);
            }
        }
        let fx = self.fx.min(ids.len().saturating_sub(1));
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let avail = ui.available_width();
            let side_w = 300.0f32;
            if avail > 860.0 {
                ui.horizontal_top(|ui| {
                    let main_w = avail - side_w - 24.0;
                    ui.allocate_ui_with_layout(vec2(main_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(main_w);
                        if ids.is_empty() {
                            self.family_empty(ui, act);
                        } else {
                            self.family_main(ui, act, id, &ids, fx);
                        }
                    });
                    ui.add_space(8.0);
                    ui.allocate_ui_with_layout(vec2(side_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(side_w);
                        self.family_side(ui, act, id, fx);
                    });
                });
            } else {
                if ids.is_empty() {
                    self.family_empty(ui, act);
                } else {
                    self.family_main(ui, act, id, &ids, fx);
                }
                ui.add_space(10.0);
                ui.separator();
                self.family_side(ui, act, id, fx);
            }
        });
    }

pub(crate)     fn family_empty(&self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        ui.label("No family group yet.");
        ui.horizontal(|ui| {
            if ui.button("Add spouse").clicked() {
                *act = Some(Action::Add(Kind::Spouse));
            }
            if ui.button("Add child").clicked() {
                *act = Some(Action::Add(Kind::Child(None)));
            }
        });
    }
}
