//! Toolbars: the main toolbar under the menus (the same on every tab) and a toolbar for each
//! tab under the tab names. Buttons have small pictures drawn here (our own simple drawings);
//! hovering shows the name and shortcut. Each can show words or not, be small or large, and
//! have buttons hidden (right-click a toolbar).

use super::*;

/// The little pictures on toolbar buttons.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Icon {
    Back,
    Forward,
    Home,
    AddPerson,
    Edit,
    Done,
    Find,
    Jump,
    Undo,
    Redo,
    Print,
    Backup,
    Father,
    Mother,
    Spouse,
    Child,
    Sibling,
    Sort,
    Star,
    ZoomIn,
    ZoomOut,
    Fit,
    Story,
    Mic,
    Clip,
    Select,
    Sheet,
    Globe,
    Pin,
}

/// Draws an icon in `r` with lines of colour `c`.
pub(crate) fn paint_icon(p: &egui::Painter, r: Rect, icon: Icon, c: Color32) {
    let s = r.width().min(r.height());
    let o = r.center();
    let w = (s / 11.0).max(1.2);
    let st = Stroke::new(w, c);
    // a point on a grid from -1..1 in both directions
    let at = |x: f32, y: f32| o + vec2(x * s * 0.5, y * s * 0.5);
    let line = |pts: &[(f32, f32)]| {
        p.add(egui::Shape::line(pts.iter().map(|(x, y)| at(*x, *y)).collect(), st));
    };
    let head = |x: f32, y: f32, rad: f32| {
        p.circle_stroke(at(x, y), rad * s * 0.5, st);
    };
    let body = |x: f32, y: f32, wd: f32| {
        // shoulders: a half circle under the head
        let n = 10;
        let pts: Vec<(f32, f32)> = (0..=n).map(|i| {
            let a = std::f32::consts::PI * (i as f32 / n as f32);
            (x - wd * a.cos(), y - wd * 0.9 * a.sin())
        }).collect();
        line(&pts);
    };
    let plus = |x: f32, y: f32, k: f32| {
        line(&[(x - k, y), (x + k, y)]);
        line(&[(x, y - k), (x, y + k)]);
    };
    let arc = |cx: f32, cy: f32, rad: f32, a0: f32, a1: f32| {
        let n = 14;
        let pts: Vec<(f32, f32)> = (0..=n).map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            (cx + rad * a.cos(), cy + rad * a.sin())
        }).collect();
        line(&pts);
    };
    use std::f32::consts::PI;
    match icon {
        Icon::Back => {
            line(&[(0.7, 0.0), (-0.7, 0.0)]);
            line(&[(-0.2, -0.5), (-0.7, 0.0), (-0.2, 0.5)]);
        }
        Icon::Forward => {
            line(&[(-0.7, 0.0), (0.7, 0.0)]);
            line(&[(0.2, -0.5), (0.7, 0.0), (0.2, 0.5)]);
        }
        Icon::Home => {
            line(&[(-0.8, 0.0), (0.0, -0.75), (0.8, 0.0)]);
            line(&[(-0.55, -0.2), (-0.55, 0.75), (0.55, 0.75), (0.55, -0.2)]);
            line(&[(-0.15, 0.75), (-0.15, 0.3), (0.15, 0.3), (0.15, 0.75)]);
        }
        Icon::AddPerson => {
            head(-0.2, -0.35, 0.28);
            body(-0.2, 0.75, 0.5);
            plus(0.6, -0.35, 0.25);
        }
        Icon::Edit => {
            line(&[(-0.7, 0.7), (-0.55, 0.25), (0.45, -0.75), (0.75, -0.45), (-0.25, 0.55), (-0.7, 0.7)]);
            line(&[(0.25, -0.55), (0.55, -0.25)]);
        }
        Icon::Done => {
            line(&[(-0.7, 0.0), (-0.2, 0.5), (0.75, -0.55)]);
        }
        Icon::Find => {
            head(-0.15, -0.15, 0.5);
            line(&[(0.22, 0.22), (0.75, 0.75)]);
        }
        Icon::Jump => {
            line(&[(0.15, -0.85), (-0.45, 0.1), (0.05, 0.1), (-0.15, 0.85), (0.45, -0.1), (-0.05, -0.1), (0.15, -0.85)]);
        }
        Icon::Undo => {
            arc(0.05, 0.1, 0.55, -PI * 0.95, PI * 0.45);
            line(&[(-0.75, -0.35), (-0.5, 0.15), (-0.1, -0.15)]);
        }
        Icon::Redo => {
            arc(-0.05, 0.1, 0.55, -PI * 0.05, -PI * 1.45);
            line(&[(0.75, -0.35), (0.5, 0.15), (0.1, -0.15)]);
        }
        Icon::Print => {
            line(&[(-0.45, -0.25), (-0.45, -0.8), (0.45, -0.8), (0.45, -0.25)]);
            line(&[(-0.45, 0.4), (-0.8, 0.4), (-0.8, -0.25), (0.8, -0.25), (0.8, 0.4), (0.45, 0.4)]);
            line(&[(-0.45, 0.1), (-0.45, 0.8), (0.45, 0.8), (0.45, 0.1), (-0.45, 0.1)]);
        }
        Icon::Backup => {
            // a box being filled, with an arrow going in
            line(&[(-0.75, 0.0), (-0.75, 0.75), (0.75, 0.75), (0.75, 0.0)]);
            line(&[(0.0, -0.85), (0.0, 0.35)]);
            line(&[(-0.35, 0.0), (0.0, 0.35), (0.35, 0.0)]);
        }
        Icon::Father | Icon::Mother => {
            head(0.0, 0.05, 0.25);
            body(0.0, 0.8, 0.45);
            line(&[(0.0, -0.45), (0.0, -0.9)]);
            line(&[(-0.25, -0.65), (0.0, -0.9), (0.25, -0.65)]);
            if icon == Icon::Mother {
                // a skirt line to tell them apart
                line(&[(-0.55, 0.8), (0.55, 0.8)]);
            }
        }
        Icon::Spouse => {
            head(-0.32, 0.1, 0.42);
            head(0.32, 0.1, 0.42);
        }
        Icon::Child => {
            line(&[(0.0, -0.9), (0.0, -0.45)]);
            line(&[(-0.25, -0.7), (0.0, -0.45), (0.25, -0.7)]);
            head(0.0, -0.05, 0.22);
            body(0.0, 0.75, 0.38);
        }
        Icon::Sibling => {
            head(-0.4, -0.25, 0.22);
            body(-0.4, 0.6, 0.35);
            head(0.4, -0.25, 0.22);
            body(0.4, 0.6, 0.35);
        }
        Icon::Sort => {
            line(&[(-0.75, -0.6), (0.1, -0.6)]);
            line(&[(-0.75, -0.1), (-0.1, -0.1)]);
            line(&[(-0.75, 0.4), (-0.3, 0.4)]);
            line(&[(0.55, -0.75), (0.55, 0.75)]);
            line(&[(0.3, 0.5), (0.55, 0.75), (0.8, 0.5)]);
        }
        Icon::Star => {
            let pts: Vec<(f32, f32)> = (0..=10).map(|i| {
                let a = -PI / 2.0 + i as f32 * PI / 5.0;
                let rr = if i % 2 == 0 { 0.85 } else { 0.38 };
                (rr * a.cos(), rr * a.sin() + 0.05)
            }).collect();
            line(&pts);
        }
        Icon::ZoomIn | Icon::ZoomOut => {
            head(-0.15, -0.15, 0.5);
            line(&[(0.22, 0.22), (0.75, 0.75)]);
            line(&[(-0.38, -0.15), (0.08, -0.15)]);
            if icon == Icon::ZoomIn {
                line(&[(-0.15, -0.38), (-0.15, 0.08)]);
            }
        }
        Icon::Fit => {
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                line(&[(0.75 * sx, 0.3 * sy), (0.75 * sx, 0.75 * sy), (0.3 * sx, 0.75 * sy)]);
            }
        }
        Icon::Story => {
            line(&[(0.0, -0.55), (0.0, 0.7)]);
            line(&[(0.0, -0.55), (-0.35, -0.7), (-0.8, -0.6), (-0.8, 0.6), (-0.35, 0.5), (0.0, 0.7), (0.35, 0.5), (0.8, 0.6), (0.8, -0.6), (0.35, -0.7), (0.0, -0.55)]);
        }
        Icon::Mic => {
            line(&[(-0.2, -0.3), (-0.2, -0.65), (0.2, -0.65), (0.2, -0.3)]);
            arc(0.0, -0.3, 0.2, 0.0, PI);
            arc(0.0, -0.25, 0.45, 0.0, PI);
            line(&[(0.0, 0.2), (0.0, 0.75)]);
            line(&[(-0.3, 0.75), (0.3, 0.75)]);
        }
        Icon::Clip => {
            line(&[(0.35, -0.3), (0.35, 0.45)]);
            arc(0.05, 0.45, 0.3, 0.0, PI);
            line(&[(-0.25, 0.45), (-0.25, -0.55)]);
            arc(0.0, -0.55, 0.25, PI, 2.0 * PI);
            line(&[(0.25, -0.55), (0.25, 0.35)]);
            arc(0.12, 0.35, 0.13, 0.0, PI);
            line(&[(-0.01, 0.35), (-0.01, -0.2)]);
        }
        Icon::Select => {
            line(&[(-0.75, -0.75), (0.75, -0.75), (0.75, 0.75), (-0.75, 0.75), (-0.75, -0.75)]);
            line(&[(-0.4, 0.0), (-0.1, 0.35), (0.45, -0.35)]);
        }
        Icon::Sheet => {
            line(&[(-0.75, -0.7), (0.75, -0.7), (0.75, 0.7), (-0.75, 0.7), (-0.75, -0.7)]);
            line(&[(-0.75, -0.25), (0.75, -0.25)]);
            line(&[(-0.75, 0.22), (0.75, 0.22)]);
            line(&[(-0.2, -0.7), (-0.2, 0.7)]);
        }
        Icon::Globe => {
            head(0.0, 0.0, 0.8);
            line(&[(-0.8, 0.0), (0.8, 0.0)]);
            arc(0.0, 0.0, 0.8, -PI / 2.0, PI / 2.0);
            line(&[(0.0, -0.8), (0.0, 0.8)]);
            let pts: Vec<(f32, f32)> = (0..=14).map(|i| {
                let a = -PI / 2.0 + PI * i as f32 / 14.0;
                (-0.35 * a.cos(), 0.8 * a.sin())
            }).collect();
            line(&pts);
            let pts: Vec<(f32, f32)> = (0..=14).map(|i| {
                let a = -PI / 2.0 + PI * i as f32 / 14.0;
                (0.35 * a.cos(), 0.8 * a.sin())
            }).collect();
            line(&pts);
        }
        Icon::Pin => {
            arc(0.0, -0.3, 0.45, PI * 0.8, PI * 2.2);
            line(&[(-0.42, -0.15), (0.0, 0.8), (0.42, -0.15)]);
            head(0.0, -0.3, 0.15);
        }
    }
}

/// One button on a toolbar.
pub(crate) struct Tool {
    /// A name kept in the settings when the button is hidden.
    pub(crate) id: &'static str,
    pub(crate) icon: Icon,
    pub(crate) label: &'static str,
    pub(crate) tip: String,
    pub(crate) enabled: bool,
    /// Draw it as switched on (the hand tool, say).
    pub(crate) on: bool,
}

impl Tool {
    pub(crate) fn new(id: &'static str, icon: Icon, label: &'static str, tip: impl Into<String>) -> Self {
        Tool { id, icon, label, tip: tip.into(), enabled: true, on: false }
    }
    pub(crate) fn enabled(mut self, e: bool) -> Self {
        self.enabled = e;
        self
    }
}

/// A gap between groups of buttons.
pub(crate) const GAP: &str = "|";

/// Draws a row of tool buttons and returns the id of the one clicked. Right-click opens the
/// menu for showing words, button size and which buttons appear.
pub(crate) fn tool_row(ui: &mut egui::Ui, which: &str, tools: &[Tool], settings: &mut Settings) -> Option<&'static str> {
    let mut clicked = None;
    let big = settings.tb_large;
    let words = settings.tb_words;
    let isz = if big { 22.0 } else { 16.0 };
    let pad = if big { 6.0 } else { 4.0 };
    let mut changed = false;
    let resp = ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let mut last_gap = true;
        for t in tools {
            if t.id == GAP {
                if !last_gap {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    last_gap = true;
                }
                continue;
            }
            if settings.tb_hidden.iter().any(|h| h == t.id) {
                continue;
            }
            last_gap = false;
            let font = FontId::proportional(if big { 14.0 } else { 13.0 });
            let galley = if words { Some(ui.painter().layout_no_wrap(t.label.to_string(), font, ui.visuals().text_color())) } else { None };
            let tw = galley.as_ref().map_or(0.0, |g| g.size().x + 6.0);
            let size = vec2(isz + pad * 2.0 + tw, isz + pad * 2.0);
            let (rect, resp) = ui.allocate_exact_size(size, if t.enabled { Sense::click() } else { Sense::hover() });
            let vis = ui.visuals();
            let fill = if t.on {
                vis.selection.bg_fill
            } else if resp.hovered() && t.enabled {
                vis.widgets.hovered.weak_bg_fill
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, 4.0, fill);
            let col = if !t.enabled {
                vis.weak_text_color().gamma_multiply(0.6)
            } else if t.on {
                vis.selection.stroke.color
            } else {
                vis.text_color()
            };
            let ir = Rect::from_min_size(rect.min + vec2(pad, pad), vec2(isz, isz));
            paint_icon(ui.painter(), ir, t.icon, col);
            if let Some(g) = galley {
                ui.painter().galley(pos2(ir.max.x + 5.0, rect.center().y - g.size().y / 2.0), g, col);
            }
            let tip = if t.tip.is_empty() { t.label.to_string() } else { t.tip.clone() };
            let resp = if t.enabled { resp.on_hover_text(tip) } else { resp.on_hover_text(format!("{} (not available here)", tip)) };
            if resp.clicked() {
                clicked = Some(t.id);
            }
        }
    });
    resp.response.interact(Sense::click()).context_menu(|ui| {
        ui.label(RichText::new(format!("{} toolbar", which)).strong());
        if ui.checkbox(&mut settings.tb_words, "Show words beside the pictures").changed() {
            changed = true;
        }
        if ui.checkbox(&mut settings.tb_large, "Large buttons").changed() {
            changed = true;
        }
        ui.separator();
        ui.label(RichText::new("Buttons to show").small().weak());
        for t in tools.iter().filter(|t| t.id != GAP) {
            let mut on = !settings.tb_hidden.iter().any(|h| h == t.id);
            if ui.checkbox(&mut on, t.label).changed() {
                if on {
                    settings.tb_hidden.retain(|h| h != t.id);
                } else {
                    settings.tb_hidden.push(t.id.to_string());
                }
                changed = true;
            }
        }
        ui.separator();
        if ui.button("Show every button").clicked() {
            let ids: Vec<&str> = tools.iter().map(|t| t.id).collect();
            settings.tb_hidden.retain(|h| !ids.contains(&h.as_str()));
            changed = true;
        }
        if ui.button(format!("Hide the {} toolbar", which.to_lowercase())).clicked() {
            if which == "Main" {
                settings.tb_main = false;
            } else {
                settings.tb_tab = false;
            }
            changed = true;
            ui.close_menu();
        }
    });
    if changed {
        save_settings(settings);
    }
    clicked
}

impl App {
    /// The main toolbar: the same on every tab.
    pub(crate) fn main_toolbar(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let has = self.sel.map_or(false, |s| self.db.people.contains_key(&s));
        let print_what = match self.view {
            View::Pedigree => "Print the pedigree chart",
            View::Family => "Print the family group sheet",
            View::Descendants => "Print the descendant chart",
            View::Charts => "Print this chart",
            View::Person => "Print a family book for this person",
            _ => "",
        };
        let tools = vec![
            Tool::new("back", Icon::Back, "Back", "Back to the person before").enabled(!self.back.is_empty()),
            Tool::new("forward", Icon::Forward, "Forward", "Forward again").enabled(!self.fwd.is_empty()),
            Tool::new("home", Icon::Home, "Home", "Go to the home person (Home key)").enabled(self.db.home.is_some() || has),
            Tool::new(GAP, Icon::Back, "", ""),
            Tool::new("add", Icon::AddPerson, "Add person", "Add a new person to the tree"),
            Tool::new("edit", Icon::Edit, "Edit", "Edit the selected person (opens their page)").enabled(has && self.editing.is_none()),
            Tool::new(GAP, Icon::Back, "", ""),
            Tool::new("find", Icon::Find, "Find", "Find a person by name, place or ID (F3)"),
            Tool::new("jump", Icon::Jump, "Quick jump", "Jump to any person, tab or command by typing (Ctrl/Cmd+K)"),
            Tool::new(GAP, Icon::Back, "", ""),
            Tool::new("undo", Icon::Undo, "Undo", "Undo the last change (Ctrl/Cmd+Z)").enabled(!self.undo.is_empty() || self.x.track_at.is_some()),
            Tool::new("redo", Icon::Redo, "Redo", "Redo what was undone (Ctrl/Cmd+Y)").enabled(!self.redo.is_empty()),
            Tool::new(GAP, Icon::Back, "", ""),
            Tool::new("print", Icon::Print, "Print", if print_what.is_empty() { "Print (on the Pedigree, Family, Descendants, Individual and Charts tabs)".to_string() } else { print_what.to_string() }).enabled(has && !print_what.is_empty()),
            Tool::new("backup", Icon::Backup, "Back up now", "Make a checked backup of your tree and photos now").enabled(self.x.practice.is_none()),
        ];
        match tool_row(ui, "Main", &tools, &mut self.settings) {
            Some("back") => *act = Some(Action::Back),
            Some("forward") => *act = Some(Action::Forward),
            Some("home") => *act = Some(Action::GoHome),
            Some("add") => *act = Some(Action::Add(Kind::Person)),
            Some("edit") => *act = Some(Action::EditStart),
            Some("find") => self.find_open = true,
            Some("jump") => self.x.jump_open = true,
            Some("undo") => self.undo_action(),
            Some("redo") => self.redo_action(),
            Some("print") => {
                *act = Some(match self.view {
                    View::Pedigree => Action::PrintPedigree,
                    View::Family => Action::PrintFamily,
                    View::Descendants => Action::PrintDescendants,
                    View::Charts => Action::PrintChart,
                    _ => Action::ReportBook,
                })
            }
            Some("backup") => self.backup_now(true),
            _ => {}
        }
    }

    /// The toolbar for the tab that is open.
    pub(crate) fn tab_toolbar(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let sel = self.sel.filter(|s| self.db.people.contains_key(s));
        let has = sel.is_some();
        let is_home = sel.is_some() && self.db.home == sel;
        let z = self.settings.zoom;
        let (which, tools): (&str, Vec<Tool>) = match self.view {
            View::Family => {
                let (f, m) = sel.map(|s| self.db.parents(s)).unwrap_or((None, None));
                let first_fam = sel.and_then(|s| self.db.fams_of(s).first().copied());
                let kids = first_fam.and_then(|f| self.db.fams.get(&f)).map_or(0, |f| f.kids.len());
                ("Family", vec![
                    Tool::new("fam_father", Icon::Father, "Add father", "Add a father for the selected person").enabled(has && f.is_none()),
                    Tool::new("fam_mother", Icon::Mother, "Add mother", "Add a mother for the selected person").enabled(has && m.is_none()),
                    Tool::new("fam_spouse", Icon::Spouse, "Add spouse", "Add a husband, wife or partner").enabled(has),
                    Tool::new("fam_child", Icon::Child, "Add child", "Add a child (to their first family)").enabled(has),
                    Tool::new("fam_sibling", Icon::Sibling, "Add brother or sister", "").enabled(has),
                    Tool::new(GAP, Icon::Back, "", ""),
                    Tool::new("fam_sort", Icon::Sort, "Sort children", "Put the children of their first family in order of birth").enabled(kids > 1),
                    Tool::new("fam_print", Icon::Print, "Family group sheet", "Print the family group sheet").enabled(has),
                ])
            }
            View::Pedigree | View::Descendants => {
                let ped = self.view == View::Pedigree;
                let mut v = vec![
                    Tool::new("ch_out", Icon::ZoomOut, "Smaller", "Zoom out (Ctrl/Cmd -)").enabled(z > 0.3),
                    Tool::new("ch_in", Icon::ZoomIn, "Bigger", "Zoom in (Ctrl/Cmd +)").enabled(z < 1.75),
                    Tool::new("ch_fit", Icon::Fit, "Fit", "Fit the chart to the window (F5)"),
                    Tool::new(GAP, Icon::Back, "", ""),
                    Tool::new("ch_home", Icon::Star, "Make home person", "Make the selected person the home person").enabled(has && !is_home),
                ];
                v.push(Tool::new(GAP, Icon::Back, "", ""));
                v.push(if ped { Tool::new("ch_print_ped", Icon::Print, "Print pedigree", "Print the pedigree chart") } else { Tool::new("ch_print_desc", Icon::Print, "Print descendants", "Print the descendant chart") }.enabled(has));
                (if ped { "Pedigree" } else { "Descendants" }, v)
            }
            View::Person => {
                let editing = self.editing.is_some();
                ("Individual", vec![
                    if editing { Tool::new("ind_done", Icon::Done, "Done", "Save and finish editing") } else { Tool::new("ind_edit", Icon::Edit, "Edit", "Unlock this page to change it") }.enabled(has),
                    Tool::new(GAP, Icon::Back, "", ""),
                    Tool::new("ind_files", Icon::Clip, "Add files", "Add photos, documents or recordings to this person").enabled(has),
                    Tool::new("ind_story", Icon::Story, "Write a story", "").enabled(has),
                    Tool::new("ind_rec", Icon::Mic, "Record a story", "Record a spoken story with the microphone").enabled(has && self.rec.is_none()),
                    Tool::new(GAP, Icon::Back, "", ""),
                    Tool::new("ind_print", Icon::Print, "Family book", "Print a family book (individual summary) for this person").enabled(has),
                ])
            }
            View::Memories | View::Gallery => ("Memories", vec![
                Tool::new("mem_files", Icon::Clip, "Add files", "Add photos, documents or recordings to the selected person").enabled(has),
                Tool::new("mem_story", Icon::Story, "Write a story", "").enabled(has),
                Tool::new("mem_rec", Icon::Mic, "Record a story", "Record a spoken story with the microphone").enabled(has && self.rec.is_none()),
                Tool::new(GAP, Icon::Back, "", ""),
                Tool::new("mem_select", Icon::Select, "Select...", "Tick several items to remove them in one go").enabled(self.settings.mem_kind != 6),
            ]),
            View::Charts => ("Charts", vec![Tool::new("chart_print", Icon::Print, "Print chart", "Print this chart").enabled(has)]),
            View::Table => ("People", vec![
                Tool::new("tbl_find", Icon::Find, "Find", "Find a person (F3)"),
                Tool::new("tbl_csv", Icon::Sheet, "Save as spreadsheet", "Save the rows shown as a CSV file for Excel, Numbers or Google Sheets"),
            ]),
            View::Map => ("Map", vec![
                Tool::new("map_fit", Icon::Fit, "Fit to pins", "Zoom to show every pin"),
                Tool::new("map_world", Icon::Globe, "World", "Show the whole world"),
                Tool::new(GAP, Icon::Back, "", ""),
                Tool::new("map_me", Icon::Pin, "This person", "Show only the selected person's places").enabled(has),
            ]),
            View::Dna => return,
        };
        match tool_row(ui, which, &tools, &mut self.settings) {
            Some("fam_father") => *act = Some(Action::Add(Kind::Father)),
            Some("fam_mother") => *act = Some(Action::Add(Kind::Mother)),
            Some("fam_spouse") => *act = Some(Action::Add(Kind::Spouse)),
            Some("fam_child") => *act = Some(Action::Add(Kind::Child(sel.and_then(|s| self.db.fams_of(s).first().copied())))),
            Some("fam_sibling") => *act = Some(Action::Add(Kind::Sibling)),
            Some("fam_sort") => {
                if let Some(f) = sel.and_then(|s| self.db.fams_of(s).first().copied()) {
                    *act = Some(Action::SortChildren(f));
                }
            }
            Some("fam_print") => *act = Some(Action::PrintFamily),
            Some("ch_out") => *act = Some(Action::SetZoom(z - 0.1)),
            Some("ch_in") => *act = Some(Action::SetZoom(z + 0.1)),
            Some("ch_fit") => *act = Some(Action::Fit),
            Some("ch_home") => {
                if let Some(s) = sel {
                    *act = Some(Action::SetHome(s));
                }
            }
            Some("ch_print_ped") => *act = Some(Action::PrintPedigree),
            Some("ch_print_desc") => *act = Some(Action::PrintDescendants),
            Some("ind_edit") => *act = Some(Action::EditStart),
            Some("ind_done") => *act = Some(Action::EditSave),
            Some("ind_files") | Some("mem_files") => *act = Some(Action::AttachFiles),
            Some("ind_story") | Some("mem_story") => {
                if let Some(s) = sel {
                    self.x.story = Some(StoryEdit { who: s, idx: None, title: String::new(), text: String::new() });
                }
            }
            Some("ind_rec") | Some("mem_rec") => *act = Some(Action::StartRec),
            Some("ind_print") => *act = Some(Action::ReportBook),
            Some("mem_select") => {
                self.x.gal_pick = true;
                self.x.gal_sel.clear();
            }
            Some("chart_print") => *act = Some(Action::PrintChart),
            Some("tbl_find") => self.find_open = true,
            Some("tbl_csv") => self.x.csv_req = true,
            Some("map_fit") => self.x.map_fit = true,
            Some("map_world") => {
                self.x.map_center = [0.0, 20.0];
                self.x.map_zoom = 2.5;
            }
            Some("map_me") => {
                self.x.map_who = 0;
                self.x.map_fit = true;
                self.x.map_pick = None;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_toolbar_draws_in_every_style() {
        let mut app = App::for_test(crate::sample::sample_family());
        let ctx = egui::Context::default();
        let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
        for (words, large) in [(true, false), (false, true)] {
            app.settings.tb_words = words;
            app.settings.tb_large = large;
            for v in [View::Family, View::Pedigree, View::Descendants, View::Person, View::Memories, View::Charts, View::Table, View::Map, View::Dna] {
                app.view = v;
                let out = ctx.run(input(), |c| app.frame(c));
                assert!(!out.shapes.is_empty());
            }
        }
        // a hidden button is left out, and the toolbars can be turned off
        app.settings.tb_hidden.push("backup".into());
        app.settings.tb_main = false;
        app.settings.tb_tab = false;
        let _ = ctx.run(input(), |c| app.frame(c));
        // every picture draws at any size
        let _ = ctx.run(input(), |c| {
            egui::CentralPanel::default().show(c, |ui| {
                let all = [Icon::Back, Icon::Forward, Icon::Home, Icon::AddPerson, Icon::Edit, Icon::Done, Icon::Find, Icon::Jump, Icon::Undo, Icon::Redo, Icon::Print, Icon::Backup, Icon::Father, Icon::Mother, Icon::Spouse, Icon::Child, Icon::Sibling, Icon::Sort, Icon::Star, Icon::ZoomIn, Icon::ZoomOut, Icon::Fit, Icon::Story, Icon::Mic, Icon::Clip, Icon::Select, Icon::Sheet, Icon::Globe, Icon::Pin];
                for (n, i) in all.iter().enumerate() {
                    for s in [12.0, 22.0, 48.0] {
                        paint_icon(ui.painter(), Rect::from_min_size(pos2(n as f32 * 50.0, s * 2.0), vec2(s, s)), *i, Color32::WHITE);
                    }
                }
            });
        });
    }
}
