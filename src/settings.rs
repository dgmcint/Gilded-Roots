//! Settings, colour schemes and the look of the program.

use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Settings {
    pub(crate) dark: bool,
    pub(crate) accent: [u8; 3],
    #[serde(default = "default_bg")]
    pub(crate) bg: [u8; 3],
    #[serde(default = "default_gens")]
    pub(crate) gens: usize,
    #[serde(default = "default_true")]
    pub(crate) hide_living: bool,
    #[serde(default)]
    pub(crate) mic: String,
    #[serde(default = "default_zoom")]
    pub(crate) zoom: f32,
    /// Old versions kept one home person for the whole program here. It is now kept with each
    /// tree (Db::home); this is only read once, to move an old choice into the tree.
    #[serde(default)]
    pub(crate) home: Option<u32>,
    #[serde(default = "default_dock")]
    pub(crate) dock: Vec<DockItem>,
    #[serde(default = "default_dock_w")]
    pub(crate) dock_w: f32,
    /// Size of all text and buttons (1.0 = normal).
    #[serde(default = "default_zoom")]
    pub(crate) ui_scale: f32,
    /// 0 "John Smith", 1 "SMITH, John", 2 "Smith, John"
    #[serde(default)]
    pub(crate) name_style: u8,
    /// 0 as typed, 1 "12 Mar 1921", 2 "March 12, 1921", 3 "1921-03-12"
    #[serde(default)]
    pub(crate) date_style: u8,
    /// Printouts on A4 paper instead of US Letter
    #[serde(default)]
    pub(crate) paper_a4: bool,
    #[serde(default = "default_true")]
    pub(crate) box_photos: bool,
    #[serde(default)]
    pub(crate) box_places: bool,
    #[serde(default)]
    pub(crate) box_ids: bool,
    /// Pedigree and descendant charts: the hand tool is on (drag to move the chart around)
    #[serde(default)]
    pub(crate) hand: bool,
    /// Profile photo size on the Individual page: 0 small, 1 normal, 2 large, 3 extra large.
    #[serde(default = "default_photo_size")]
    pub(crate) photo_size: u8,
    /// Backups: 0 daily, 1 every hour while open, 2 weekly. How many to keep. A second folder
    /// (USB stick, external drive) that also receives a Family Tree Archive.
    #[serde(default = "default_true")]
    pub(crate) show_history: bool,
    /// The Memories tab's last filter (0 all, 1 photos, 2 documents, 3 recordings, 4 stories,
    /// 5 notes, 6 not linked to anyone) and whether it is grouped by person.
    #[serde(default)]
    pub(crate) mem_kind: u8,
    #[serde(default)]
    pub(crate) mem_by_person: bool,
    /// Toolbars: the main toolbar and the tab toolbars shown, words beside the pictures, large
    /// buttons, and buttons hidden (by their names).
    #[serde(default = "default_true")]
    pub(crate) tb_main: bool,
    #[serde(default = "default_true")]
    pub(crate) tb_tab: bool,
    #[serde(default = "default_true")]
    pub(crate) tb_words: bool,
    #[serde(default)]
    pub(crate) tb_large: bool,
    #[serde(default)]
    pub(crate) tb_hidden: Vec<String>,
    /// Use the computer's own font (on by default) rather than the one built into the program.
    #[serde(default = "default_true")]
    pub(crate) system_font: bool,
    /// Note in the research journal, by itself, what was done (research files added, sources cited).
    #[serde(default = "default_true")]
    pub(crate) auto_log: bool,
    /// The last choices in the Import options window.
    #[serde(default)]
    pub(crate) import: ImportChoices,
    /// The filter in the Memories section of the Individual page.
    #[serde(default)]
    pub(crate) pmem_kind: u8,
    /// The tree that was open last ("" = the main tree).
    #[serde(default)]
    pub(crate) current_tree: String,
    #[serde(default)]
    pub(crate) backup_every: u8,
    #[serde(default = "default_keep")]
    pub(crate) backup_keep: usize,
    #[serde(default)]
    pub(crate) backup_extra: String,
    /// Window size when the program was last closed (points), and whether it was maximised.
    #[serde(default)]
    pub(crate) win: Option<[f32; 2]>,
    #[serde(default)]
    pub(crate) win_max: bool,
    /// Report designs the user saved (Reports > Report designer).
    #[serde(default)]
    pub(crate) reports: Vec<designer::Design>,
    /// Panel layouts the user saved by name.
    #[serde(default)]
    pub(crate) layouts: Vec<Layout>,
    /// Map colours: 0 matches the app (charcoal or paper), then the styles in mapview::MAP_STYLES.
    #[serde(default)]
    pub(crate) map_style: u8,
    /// Background for the light theme. Colour schemes change it; the default is the original grey.
    #[serde(default = "default_light_bg")]
    pub(crate) light_bg: [u8; 3],
    /// Text colour set by a colour scheme. None keeps the theme's own text colour.
    #[serde(default)]
    pub(crate) ink: Option<[u8; 3]>,
    /// The language of the program, by its code (i18n::LANGUAGES); "" is English.
    #[serde(default)]
    pub(crate) language: String,
    /// Guided tour lessons finished (by their id).
    #[serde(default)]
    pub(crate) tour_done: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Layout {
    pub(crate) name: String,
    pub(crate) dock: Vec<DockItem>,
    pub(crate) width: f32,
}

/// Ready-made layouts: (name, panels to show).
pub(crate) const LAYOUT_PRESETS: &[(&str, &[&str])] = &[
    ("Research", &["relationships", "todo", "sources", "notes"]),
    ("Photos and stories", &["media", "relationships", "notes"]),
    ("Family", &["relationships", "events", "names"]),
    ("Everything", &["relationships", "todo", "events", "sources", "notes", "names", "media", "more"]),
];

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DockItem {
    pub(crate) sec: String,
    pub(crate) on: bool,
    pub(crate) float: bool,
}

pub(crate) fn default_dock() -> Vec<DockItem> {
    let d = |sec: &str, on: bool| DockItem { sec: sec.to_string(), on, float: false };
    vec![d("relationships", true), d("todo", true), d("journal", false), d("events", false), d("sources", false), d("notes", true), d("names", false), d("media", false), d("lds", false), d("contact", false), d("more", false)]
}

pub(crate) fn default_keep() -> usize {
    14
}

pub(crate) fn default_photo_size() -> u8 {
    1
}

pub(crate) fn default_dock_w() -> f32 {
    380.0
}

pub(crate) fn dock_title(sec: &str) -> &'static str {
    match sec {
        "relationships" => "Relationships",
        "events" => "Events",
        "sources" => "Sources",
        "notes" => "Notes",
        "names" => "Other names",
        "media" => "Memories",
        "more" => "Merge and delete",
        "todo" => "Research to-do",
        "journal" => "Research journal",
        "lds" => "LDS ordinances",
        "contact" => "Contact details",
        _ => "",
    }
}

pub(crate) fn default_zoom() -> f32 {
    1.0
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_gens() -> usize {
    5
}

pub(crate) fn default_bg() -> [u8; 3] {
    [30, 31, 34]
}

impl Default for Settings {
    fn default() -> Self {
        Settings { hand: false, dark: true, accent: [59, 111, 212], bg: default_bg(), gens: default_gens(), hide_living: true, mic: String::new(), zoom: 1.0, home: None, dock: default_dock(), dock_w: default_dock_w(), ui_scale: 1.0, name_style: 0, date_style: 0, paper_a4: false, box_photos: true, box_places: false, box_ids: false, photo_size: 1, show_history: true, current_tree: String::new(), backup_every: 0, backup_keep: 14, backup_extra: String::new(), win: None, win_max: false, layouts: Vec::new(), reports: Vec::new(), map_style: 0, light_bg: default_light_bg(), ink: None, mem_kind: 0, mem_by_person: false, pmem_kind: 0, import: ImportChoices::default(), tb_main: true, tb_tab: true, tb_words: true, tb_large: false, tb_hidden: Vec::new(), language: String::new(), tour_done: Vec::new(), system_font: true, auto_log: true }
    }
}

pub(crate) const PRESETS: [(&str, [u8; 3]); 24] = [
    ("Blue", [59, 111, 212]),
    ("Royal blue", [45, 80, 170]),
    ("Sky", [56, 150, 220]),
    ("Navy", [40, 60, 120]),
    ("Teal", [20, 150, 150]),
    ("Turquoise", [30, 175, 170]),
    ("Green", [56, 142, 60]),
    ("Forest", [40, 105, 60]),
    ("Olive", [120, 130, 50]),
    ("Sage", [120, 150, 110]),
    ("Gold", [196, 152, 40]),
    ("Amber", [220, 150, 30]),
    ("Orange", [230, 126, 34]),
    ("Copper", [184, 100, 60]),
    ("Brown", [130, 90, 60]),
    ("Red", [200, 70, 70]),
    ("Crimson", [170, 30, 60]),
    ("Rose", [210, 90, 130]),
    ("Pink", [225, 120, 170]),
    ("Plum", [130, 60, 120]),
    ("Purple", [140, 90, 200]),
    ("Lavender", [150, 130, 210]),
    ("Slate", [110, 120, 135]),
    ("Graphite", [80, 85, 95]),
];

pub(crate) const BG_PRESETS: [(&str, [u8; 3]); 8] = [
    ("Black", [14, 14, 16]),
    ("Charcoal", [30, 31, 34]),
    ("Grey", [50, 52, 56]),
    ("Light grey", [74, 77, 83]),
    ("Midnight blue", [22, 28, 44]),
    ("Deep green", [22, 36, 30]),
    ("Espresso", [38, 30, 26]),
    ("Aubergine", [36, 26, 40]),
];

/// A full colour scheme: whether it is dark, the background, the accent and the text colour.
/// Picking one under View > Colours sets all four; the swatches below still change them one by one.
pub(crate) struct Scheme {
    pub(crate) name: &'static str,
    pub(crate) group: &'static str,
    pub(crate) dark: bool,
    pub(crate) bg: [u8; 3],
    pub(crate) accent: [u8; 3],
    pub(crate) ink: [u8; 3],
}

pub(crate) const SCHEMES: &[Scheme] = &[
    Scheme { name: "Slate Blue", group: "Tree pictures", dark: true, bg: [67, 82, 106], accent: [135, 141, 151], ink: [246, 238, 226] },
    Scheme { name: "Navy", group: "Tree pictures", dark: true, bg: [30, 52, 99], accent: [115, 129, 158], ink: [255, 255, 255] },
    Scheme { name: "Parchment", group: "Tree pictures", dark: false, bg: [226, 201, 162], accent: [140, 88, 42], ink: [66, 40, 18] },
    Scheme { name: "Blue Linen", group: "Tree pictures", dark: true, bg: [88, 110, 142], accent: [144, 155, 171], ink: [236, 231, 220] },
    Scheme { name: "Leather", group: "Tree pictures", dark: true, bg: [104, 52, 24], accent: [156, 117, 86], ink: [242, 224, 188] },
    Scheme { name: "Leaf Green", group: "Genealogy sites", dark: false, bg: [243, 240, 232], accent: [104, 134, 28], ink: [52, 48, 44] },
    Scheme { name: "Fresh Green", group: "Genealogy sites", dark: false, bg: [242, 243, 240], accent: [80, 132, 34], ink: [51, 51, 51] },
    Scheme { name: "Deep Green", group: "Genealogy sites", dark: true, bg: [48, 88, 38], accent: [124, 150, 117], ink: [250, 252, 246] },
    Scheme { name: "Sage", group: "Earth tones", dark: false, bg: [199, 208, 184], accent: [88, 110, 74], ink: [40, 52, 36] },
    Scheme { name: "Olive", group: "Earth tones", dark: true, bg: [85, 90, 52], accent: [143, 145, 113], ink: [240, 236, 214] },
    Scheme { name: "Moss", group: "Earth tones", dark: true, bg: [66, 82, 50], accent: [131, 142, 114], ink: [238, 240, 220] },
    Scheme { name: "Forest Green", group: "Earth tones", dark: true, bg: [36, 66, 48], accent: [111, 132, 115], ink: [234, 240, 226] },
    Scheme { name: "Terracotta", group: "Earth tones", dark: true, bg: [150, 78, 52], accent: [188, 138, 116], ink: [250, 236, 222] },
    Scheme { name: "Clay", group: "Earth tones", dark: false, bg: [214, 180, 156], accent: [150, 80, 50], ink: [70, 40, 28] },
    Scheme { name: "Burgundy", group: "Popular colors", dark: true, bg: [110, 28, 40], accent: [163, 107, 114], ink: [250, 236, 236] },
    Scheme { name: "Teal", group: "Popular colors", dark: true, bg: [22, 92, 100], accent: [101, 150, 154], ink: [232, 246, 244] },
    Scheme { name: "Plum", group: "Popular colors", dark: true, bg: [82, 46, 88], accent: [143, 117, 148], ink: [244, 234, 246] },
    Scheme { name: "Charcoal", group: "Popular colors", dark: true, bg: [52, 54, 58], accent: [121, 123, 124], ink: [236, 236, 232] },
    Scheme { name: "Midnight", group: "Popular colors", dark: true, bg: [20, 24, 40], accent: [98, 102, 116], ink: [226, 230, 240] },
    Scheme { name: "Sky", group: "Popular colors", dark: false, bg: [200, 222, 240], accent: [40, 100, 170], ink: [20, 44, 70] },
    Scheme { name: "Rose", group: "Popular colors", dark: false, bg: [236, 210, 212], accent: [160, 60, 80], ink: [80, 30, 40] },
    Scheme { name: "Ivory", group: "Neutrals", dark: false, bg: [246, 242, 228], accent: [110, 104, 80], ink: [40, 38, 30] },
    Scheme { name: "Linen", group: "Neutrals", dark: false, bg: [236, 230, 218], accent: [120, 100, 70], ink: [50, 44, 36] },
    Scheme { name: "Stone", group: "Neutrals", dark: false, bg: [214, 212, 206], accent: [90, 94, 100], ink: [40, 40, 38] },
    Scheme { name: "Taupe", group: "Neutrals", dark: false, bg: [200, 190, 178], accent: [110, 92, 76], ink: [50, 42, 36] },
];

/// The light background used before colour schemes existed.
pub(crate) fn default_light_bg() -> [u8; 3] {
    [233, 234, 236]
}

impl Settings {
    pub(crate) fn use_scheme(&mut self, s: &Scheme) {
        self.dark = s.dark;
        self.accent = s.accent;
        self.ink = Some(s.ink);
        if s.dark {
            self.bg = s.bg;
        } else {
            self.light_bg = s.bg;
        }
    }

    pub(crate) fn is_scheme(&self, s: &Scheme) -> bool {
        self.dark == s.dark && self.accent == s.accent && self.ink == Some(s.ink) && (if s.dark { self.bg } else { self.light_bg }) == s.bg
    }
}

/// The schemes in their groups; returns true when one was picked.
pub(crate) fn scheme_list(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut picked = false;
    for (i, s) in SCHEMES.iter().enumerate() {
        if i == 0 || SCHEMES[i - 1].group != s.group {
            if i > 0 {
                ui.separator();
            }
            ui.label(RichText::new(crate::i18n::trv(s.group)).small().weak());
        }
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(28.0, 16.0), Sense::hover());
            let bg = Color32::from_rgb(s.bg[0], s.bg[1], s.bg[2]);
            ui.painter().rect(rect, 3.0_f32, bg, Stroke::new(1.0_f32, Color32::GRAY));
            let dot = Rect::from_center_size(rect.center(), vec2(10.0, 8.0));
            ui.painter().rect_filled(dot, 2.0_f32, Color32::from_rgb(s.ink[0], s.ink[1], s.ink[2]));
            if ui.selectable_label(settings.is_scheme(s), crate::i18n::trv(s.name)).clicked() {
                settings.use_scheme(s);
                picked = true;
            }
        });
    }
    picked
}

/// A grid of large colour swatches with their names; returns true when one was picked.
pub(crate) fn swatch_grid(ui: &mut egui::Ui, list: &[(&str, [u8; 3])], cur: &mut [u8; 3], outline: bool) -> bool {
    let mut picked = false;
    egui::Grid::new(ui.next_auto_id()).spacing([8.0, 8.0]).show(ui, |ui| {
        for (i, (name, c)) in list.iter().enumerate() {
            ui.vertical(|ui| {
                ui.set_width(64.0);
                let (rect, resp) = ui.allocate_exact_size(vec2(64.0, 40.0), Sense::click());
                let col = Color32::from_rgb(c[0], c[1], c[2]);
                ui.painter().rect_filled(rect, 8.0_f32, col);
                if outline || resp.hovered() {
                    ui.painter().rect_stroke(rect, 8.0_f32, Stroke::new(1.0_f32, Color32::GRAY));
                }
                if *cur == *c {
                    ui.painter().rect_stroke(rect.expand(3.0), 10.0_f32, Stroke::new(2.5_f32, ui.visuals().text_color()));
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, "\u{2713}", FontId::proportional(20.0), on_accent(col));
                }
                ui.label(RichText::new(*name).small());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    *cur = *c;
                    picked = true;
                }
            });
            if (i + 1) % 6 == 0 {
                ui.end_row();
            }
        }
    });
    picked
}

pub(crate) fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub(crate) fn load_settings() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .map(|mut st| {
            // panels added in later versions show up for people with older settings
            for d in default_dock() {
                if !st.dock.iter().any(|x| x.sec == d.sec) {
                    st.dock.push(d);
                }
            }
            st
        })
        .unwrap_or_default()
}

pub(crate) fn save_settings(s: &Settings) {
    if let Ok(j) = serde_json::to_string(s) {
        let _ = atomic_write(&settings_path(), j.as_bytes());
    }
}

pub(crate) fn lighten(c: Color32) -> Color32 {
    let f = |v: u8| v.saturating_add(((255u16 - v as u16) * 40 / 100) as u8);
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

pub(crate) fn on_accent(c: Color32) -> Color32 {
    let l = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if l > 160.0 { Color32::BLACK } else { Color32::WHITE }
}

/// How light a colour looks (0 black to 1 white), as used for contrast checks.
fn luminance(c: Color32) -> f32 {
    let f = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.039_28 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
}

/// The contrast between two colours, from 1 (the same) to 21 (black on white).
pub(crate) fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `a` mixed with `b`: t = 1 gives `a`, t = 0 gives `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// The fill and text colours for selected things: the accent, softened toward the background until
/// every text colour given reads clearly on it (selected text in a box keeps its own colour). The text
/// on selected buttons and tabs is the one that stands out most. Gives (fill, text).
pub(crate) fn readable_selection(accent: Color32, bg: Color32, texts: &[Color32]) -> (Color32, Color32) {
    let ok = |fill: Color32| texts.iter().all(|c| contrast(*c, fill) >= 4.5);
    // if even the background is not enough (a mid-tone scheme), go on toward black under light text
    // or toward white under dark text
    let light_text = texts.iter().map(|c| luminance(*c)).sum::<f32>() / texts.len().max(1) as f32 > 0.4;
    let end = if light_text { Color32::BLACK } else { Color32::WHITE };
    let steps = (0..=20).map(|i| mix(accent, bg, 1.0 - i as f32 * 0.05)).chain((1..=20).map(|i| mix(end, bg, i as f32 * 0.05)));
    let mut fill = bg;
    for f in steps {
        fill = f;
        if ok(f) {
            break;
        }
    }
    let best = texts.iter().copied().max_by(|a, b| contrast(*a, fill).total_cmp(&contrast(*b, fill))).unwrap_or(Color32::BLACK);
    (fill, best)
}

pub(crate) fn shift(bg: [u8; 3], d: i16) -> Color32 {
    let f = |v: u8| (v as i16 + d).clamp(0, 255) as u8;
    Color32::from_rgb(f(bg[0]), f(bg[1]), f(bg[2]))
}

pub(crate) fn apply_theme(ctx: &egui::Context, settings: &Settings) {
    let (dark, acc, bg) = (settings.dark, settings.accent, settings.bg);
    let rgb = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let accent = Color32::from_rgb(acc[0], acc[1], acc[2]);
    if dark {
        v.panel_fill = shift(bg, 0);
        v.window_fill = shift(bg, 7);
        v.extreme_bg_color = shift(bg, -6);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, shift(bg, 25));
        v.widgets.inactive.bg_fill = shift(bg, 15);
        v.widgets.inactive.weak_bg_fill = shift(bg, 15);
        v.widgets.hovered.bg_fill = shift(bg, 28);
        v.widgets.hovered.weak_bg_fill = shift(bg, 28);
        v.widgets.active.bg_fill = shift(bg, 40);
        v.widgets.active.weak_bg_fill = shift(bg, 40);
        v.hyperlink_color = lighten(accent);
    } else if settings.light_bg == default_light_bg() {
        v.panel_fill = rgb(233, 234, 236);
        v.window_fill = rgb(245, 245, 246);
        v.extreme_bg_color = Color32::WHITE;
        v.hyperlink_color = accent;
    } else {
        // A light colour scheme: the same steps as the dark theme, going darker instead of lighter.
        let lb = settings.light_bg;
        v.panel_fill = shift(lb, 0);
        v.window_fill = shift(lb, 10);
        v.extreme_bg_color = shift(lb, 22);
        v.faint_bg_color = shift(lb, 6);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, shift(lb, -40));
        v.widgets.inactive.bg_fill = shift(lb, -12);
        v.widgets.inactive.weak_bg_fill = shift(lb, -12);
        v.widgets.hovered.bg_fill = shift(lb, -22);
        v.widgets.hovered.weak_bg_fill = shift(lb, -22);
        v.widgets.active.bg_fill = shift(lb, -32);
        v.widgets.active.weak_bg_fill = shift(lb, -32);
        v.hyperlink_color = accent;
    }
    if let Some(ink) = settings.ink {
        let ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.fg_stroke.color = ink;
        }
    }
    // What is selected (a tab, a chosen button, a highlighted row, selected text) must stay easy to read:
    // the fill is the accent, softened toward the background until the text on it has a contrast of
    // at least 4.5 to 1 (the usual guideline for small text), and the text on it is the theme's ink.
    let texts = [v.widgets.noninteractive.fg_stroke.color, v.widgets.inactive.fg_stroke.color, v.widgets.active.fg_stroke.color];
    let (sel_bg, sel_fg) = readable_selection(accent, v.panel_fill, &texts);
    v.selection.bg_fill = sel_bg;
    v.selection.stroke = Stroke::new(1.0_f32, sel_fg);
    let r = Rounding::same(6.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.window_rounding = Rounding::same(8.0);
    v.menu_rounding = Rounding::same(6.0);
    ctx.set_visuals(v);
    ctx.style_mut(apply_spacing);
}

#[cfg(test)]
mod readable_tests {
    use super::*;

    /// Every colour scheme, and the plain light and dark themes, keep selected things easy to read.
    #[test]
    fn selected_things_are_easy_to_read_in_every_scheme() {
        let mut all: Vec<Settings> = SCHEMES
            .iter()
            .map(|sc| {
                let mut s = Settings::default();
                s.use_scheme(sc);
                s
            })
            .collect();
        for dark in [true, false] {
            for accent in [[59, 111, 212], [90, 94, 100], [67, 82, 106], [200, 200, 60], [30, 30, 30]] {
                all.push(Settings { dark, accent, ink: None, ..Settings::default() });
            }
        }
        let ctx = egui::Context::default();
        for s in all {
            apply_theme(&ctx, &s);
            let v = ctx.style().visuals.clone();
            let c = contrast(v.selection.stroke.color, v.selection.bg_fill);
            assert!(c >= 4.5, "selected text {:?} on {:?} only {c:.1} (dark {}, accent {:?})", v.selection.stroke.color, v.selection.bg_fill, s.dark, s.accent);
            let t = contrast(v.widgets.noninteractive.fg_stroke.color, v.selection.bg_fill);
            assert!(t >= 4.5, "text over the selection only {t:.1} (dark {}, accent {:?})", s.dark, s.accent);
        }
    }
}
