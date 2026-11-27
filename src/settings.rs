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
    /// The pedigree's style (0 Classic, 1 Photo cards, 2 Couple cards, 3 Upright tree, 4 Compact) and colours
    /// (0 plain, 1 men and women, 2 father's and mother's side).
    #[serde(default)]
    pub(crate) ped_style: u8,
    #[serde(default)]
    pub(crate) ped_colours: u8,
    /// A thin coloured band across the top of each pedigree card (men and women, or the two sides).
    #[serde(default)]
    pub(crate) ped_band: bool,
    /// Stronger outlines and text, for people who find the usual look hard to see.
    #[serde(default)]
    pub(crate) high_contrast: bool,
    /// Chart colours that people with colour blindness can tell apart (blue and orange).
    #[serde(default)]
    pub(crate) cb_colours: bool,
    /// The glass look in use (its scheme's name), or "" for none.
    #[serde(default)]
    pub(crate) glass: String,
    /// An outline around whatever the keyboard is on, after Tab is pressed.
    #[serde(default = "default_true")]
    pub(crate) focus_ring: bool,
    /// Open on the Home tab when the program starts.
    #[serde(default = "default_true")]
    pub(crate) start_home: bool,
    /// Settings saved before the default colour became gold have this false (see load_settings).
    #[serde(default)]
    pub(crate) accent_gold: bool,
    /// Light or dark following the computer's own setting (Automatic on a Mac).
    #[serde(default)]
    pub(crate) auto_dark: bool,
    /// A splash of the accent colour in the glass looks' backdrop (they are neutral otherwise).
    #[serde(default)]
    pub(crate) glass_splash: bool,
    /// A see-through window: the desktop shows behind the glass looks (from the next start).
    #[serde(default)]
    pub(crate) see_through: bool,
    /// How frosted the glass looks are: 0 clear, 1 milky and nearly solid.
    #[serde(default = "default_frost")]
    pub(crate) glass_frost: f32,
    /// The floating toolbar down the left of the charts.
    #[serde(default = "default_true")]
    pub(crate) float_tb: bool,
    /// The tab open on the Individual page.
    #[serde(default = "default_person_tab")]
    pub(crate) person_tab: String,
    /// LDS ordinances shown under Facts on the Individual page (they show anyway when recorded).
    #[serde(default)]
    pub(crate) show_lds: bool,
    /// What the Pedigree tab shows: 0 the pedigree, 1 the fan chart, 2 the hourglass.
    #[serde(default)]
    pub(crate) ped_mode: u8,
    /// The Timeline tab of the Individual page: 0 the person's life, 1 the family's lifespans.
    #[serde(default)]
    pub(crate) tl_mode: u8,
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
/// The tabs of the Individual page, in order (Relationships stay on the right).
pub(crate) const PERSON_TABS: [&str; 6] = ["facts", "timeline", "media", "sources", "notes", "research"];

pub(crate) fn person_tab_title(sec: &str) -> &'static str {
    match sec {
        "facts" => "Facts",
        "timeline" => "Timeline",
        "research" => "Research",
        other => dock_title(other),
    }
}

pub(crate) fn default_person_tab() -> String {
    "facts".to_string()
}

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
        Settings { dark: true, accent: DEFAULT_ACCENT, bg: default_bg(), gens: default_gens(), hide_living: true, mic: String::new(), zoom: 1.0, home: None, dock: default_dock(), dock_w: default_dock_w(), ui_scale: 1.0, name_style: 0, date_style: 0, paper_a4: false, box_photos: true, box_places: false, box_ids: false, photo_size: 1, show_history: true, current_tree: String::new(), backup_every: 0, backup_keep: 14, backup_extra: String::new(), win: None, win_max: false, layouts: Vec::new(), reports: Vec::new(), map_style: 0, light_bg: default_light_bg(), ink: None, mem_kind: 0, mem_by_person: false, pmem_kind: 0, import: ImportChoices::default(), tb_main: true, tb_tab: true, tb_words: true, tb_large: false, tb_hidden: Vec::new(), language: String::new(), tour_done: Vec::new(), system_font: true, auto_log: true, ped_style: 1, ped_colours: 0, ped_band: false, high_contrast: false, cb_colours: false, focus_ring: true, glass: String::new(), start_home: true, accent_gold: true, auto_dark: false, glass_splash: false, see_through: false, glass_frost: default_frost(), float_tb: true, person_tab: default_person_tab(), show_lds: false, ped_mode: 0, tl_mode: 0 }
    }
}

/// The colour the program starts with: an old gold, for Gilded Roots.
pub(crate) const DEFAULT_ACCENT: [u8; 3] = [160, 112, 24];
/// The colour it started with before 1.0.9.
const OLD_DEFAULT_ACCENT: [u8; 3] = [59, 111, 212];

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
    /// A glass look: see-through panels and frosted windows over a soft coloured backdrop.
    pub(crate) glass: bool,
}

/// The group holding Gilded Roots' own look, as it comes (the computer's text colour, no glass).
pub(crate) const DEFAULT_GROUP: &str = "Gilded Roots";

pub(crate) const SCHEMES: &[Scheme] = &[
    Scheme { name: "Gilded Roots dark", group: DEFAULT_GROUP, dark: true, bg: [30, 31, 34], accent: DEFAULT_ACCENT, ink: [0, 0, 0], glass: false },
    Scheme { name: "Gilded Roots light", group: DEFAULT_GROUP, dark: false, bg: [233, 234, 236], accent: DEFAULT_ACCENT, ink: [0, 0, 0], glass: false },
    Scheme { name: "Slate Blue", group: "Earth tones", dark: true, bg: [67, 82, 106], accent: [135, 141, 151], ink: [246, 238, 226], glass: false },
    Scheme { name: "Navy", group: "Earth tones", dark: true, bg: [30, 52, 99], accent: [115, 129, 158], ink: [255, 255, 255], glass: false },
    Scheme { name: "Tan", group: "Earth tones", dark: false, bg: [226, 201, 162], accent: [140, 88, 42], ink: [66, 40, 18], glass: false },
    Scheme { name: "Brown", group: "Earth tones", dark: true, bg: [104, 52, 24], accent: [156, 117, 86], ink: [242, 224, 188], glass: false },
    Scheme { name: "Leaf Green", group: "Earth tones", dark: false, bg: [243, 240, 232], accent: [104, 134, 28], ink: [52, 48, 44], glass: false },
    Scheme { name: "Sage", group: "Earth tones", dark: false, bg: [199, 208, 184], accent: [88, 110, 74], ink: [40, 52, 36], glass: false },
    Scheme { name: "Forest Green", group: "Earth tones", dark: true, bg: [36, 66, 48], accent: [111, 132, 115], ink: [234, 240, 226], glass: false },
    Scheme { name: "Terracotta", group: "Earth tones", dark: true, bg: [150, 78, 52], accent: [188, 138, 116], ink: [250, 236, 222], glass: false },
    Scheme { name: "Burgundy", group: "Popular colors", dark: true, bg: [110, 28, 40], accent: [163, 107, 114], ink: [250, 236, 236], glass: false },
    Scheme { name: "Teal", group: "Popular colors", dark: true, bg: [22, 92, 100], accent: [101, 150, 154], ink: [232, 246, 244], glass: false },
    Scheme { name: "Plum", group: "Popular colors", dark: true, bg: [82, 46, 88], accent: [143, 117, 148], ink: [244, 234, 246], glass: false },
    Scheme { name: "Charcoal", group: "Popular colors", dark: true, bg: [52, 54, 58], accent: [121, 123, 124], ink: [236, 236, 232], glass: false },
    Scheme { name: "Sky Blue", group: "Popular colors", dark: false, bg: [200, 222, 240], accent: [40, 100, 170], ink: [20, 44, 70], glass: false },
    Scheme { name: "Rose", group: "Popular colors", dark: false, bg: [236, 210, 212], accent: [160, 60, 80], ink: [80, 30, 40], glass: false },
    Scheme { name: "Ivory", group: "Neutrals and heritage", dark: false, bg: [246, 242, 228], accent: [110, 104, 80], ink: [40, 38, 30], glass: false },
    Scheme { name: "Light Grey", group: "Neutrals and heritage", dark: false, bg: [214, 212, 206], accent: [90, 94, 100], ink: [40, 40, 38], glass: false },
    Scheme { name: "Sepia", group: "Neutrals and heritage", dark: false, bg: [234, 222, 199], accent: [122, 82, 44], ink: [52, 36, 20], glass: false },
    Scheme { name: "High contrast light", group: "Easy to see", dark: false, bg: [255, 255, 255], accent: [0, 64, 160], ink: [0, 0, 0], glass: false },
    Scheme { name: "High contrast dark", group: "Easy to see", dark: true, bg: [0, 0, 0], accent: [255, 214, 0], ink: [255, 255, 255], glass: false },
    // Glass looks
    Scheme { name: "Light Glass", group: "Glass", dark: false, bg: [226, 234, 246], accent: [10, 99, 204], ink: [11, 37, 64], glass: true },
    Scheme { name: "Dark Glass", group: "Glass", dark: true, bg: [26, 14, 14], accent: [79, 163, 184], ink: [255, 255, 255], glass: true },
    Scheme { name: "Blue Glass", group: "Glass", dark: true, bg: [7, 26, 69], accent: [25, 230, 255], ink: [242, 251, 255], glass: true },
    Scheme { name: "Black and Amber", group: "Retro screens", dark: true, bg: [14, 14, 20], accent: [247, 181, 56], ink: [246, 231, 200], glass: false },
];

/// The soft coloured backdrop behind a glass look: the four corners, then soft glows
/// (centre as a share of the window, radius as a share of its larger side, colour).
pub(crate) struct GlassBackdrop {
    pub(crate) corners: [[u8; 3]; 4],
    pub(crate) glows: &'static [([f32; 2], f32, [u8; 4])],
}

pub(crate) fn glass_backdrop(name: &str) -> GlassBackdrop {
    match name {
        // neutral, as glass over a plain desk: no colour of its own beyond a faint cool tint
        "Dark Glass" => GlassBackdrop { corners: [[38, 40, 45], [26, 28, 32], [20, 22, 25], [32, 34, 39]], glows: &[([0.75, 0.25], 0.45, [255, 255, 255, 16]), ([0.2, 0.8], 0.4, [150, 170, 200, 14])] },
        "Blue Glass" => GlassBackdrop { corners: [[12, 44, 108], [7, 26, 69], [4, 18, 52], [10, 30, 80]], glows: &[([0.8, 0.2], 0.42, [25, 230, 255, 70]), ([0.2, 0.75], 0.38, [60, 140, 255, 50])] },
        // "Light Glass": light and neutral, a bright window over a pale grey
        _ => GlassBackdrop { corners: [[226, 229, 235], [238, 239, 243], [232, 234, 238], [222, 226, 233]], glows: &[([0.25, 0.25], 0.45, [255, 255, 255, 110]), ([0.8, 0.75], 0.4, [205, 214, 228, 70])] },
    }
}

/// The light background used before colour schemes existed.
pub(crate) fn default_light_bg() -> [u8; 3] {
    [233, 234, 236]
}

impl Settings {
    pub(crate) fn use_scheme(&mut self, s: &Scheme) {
        self.glass = if s.glass { s.name.to_string() } else { String::new() };
        self.dark = s.dark;
        self.accent = s.accent;
        self.ink = if s.group == DEFAULT_GROUP { None } else { Some(s.ink) };
        if s.dark {
            self.bg = s.bg;
        } else {
            self.light_bg = s.bg;
        }
    }

    pub(crate) fn is_scheme(&self, s: &Scheme) -> bool {
        (self.glass == if s.glass { s.name } else { "" }) && self.dark == s.dark && self.accent == s.accent && self.ink == (if s.group == DEFAULT_GROUP { None } else { Some(s.ink) }) && (if s.dark { self.bg } else { self.light_bg }) == s.bg
    }
}

/// The schemes in their groups; returns true when one was picked.
pub(crate) fn scheme_list(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut picked = false;
    // every scheme on show at once, as swatches in its own colours, under three headings
    let (w, h) = (72.0, 38.0);
    let cols = ((ui.available_width() + 8.0) / (w + 8.0)).floor().max(3.0) as usize;
    let heads: [(&str, fn(&Scheme) -> bool); 3] = [("Light", |s| !s.glass && !s.dark), ("Dark", |s| !s.glass && s.dark), ("Glass", |s| s.glass)];
    for (k, (head, keep)) in heads.iter().enumerate() {
        ui.label(RichText::new(crate::i18n::trv(head)).small().weak());
        egui::Grid::new(("scheme_grid", k)).spacing([8.0, 6.0]).show(ui, |ui| {
            for (i, s) in SCHEMES.iter().filter(|s| keep(s)).enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(w);
                    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::click());
                    let bg = Color32::from_rgb(s.bg[0], s.bg[1], s.bg[2]);
                    let accent = Color32::from_rgb(s.accent[0], s.accent[1], s.accent[2]);
                    // the Gilded Roots looks use the theme's own text colour
                    let ink = if s.group == DEFAULT_GROUP { if s.dark { Color32::from_gray(230) } else { Color32::from_gray(30) } } else { Color32::from_rgb(s.ink[0], s.ink[1], s.ink[2]) };
                    let p = ui.painter();
                    p.rect_filled(rect, 5.0_f32, bg);
                    // a band of the accent colour along the bottom, and a little text
                    let band = Rect::from_min_max(pos2(rect.min.x, rect.max.y - 9.0), rect.max);
                    p.rect_filled(band, egui::Rounding { nw: 0.0, ne: 0.0, sw: 5.0, se: 5.0 }, accent);
                    p.text(pos2(rect.min.x + 8.0, rect.min.y + 5.0), Align2::LEFT_TOP, "Aa", FontId::proportional(14.0), ink);
                    p.rect_stroke(rect, 5.0_f32, Stroke::new(1.0_f32, if resp.hovered() { ui.visuals().text_color() } else { Color32::from_gray(128) }));
                    let on = settings.is_scheme(s);
                    if on {
                        p.rect_stroke(rect.expand(2.5), 7.0_f32, Stroke::new(2.0_f32, ui.visuals().selection.bg_fill));
                        p.text(pos2(rect.max.x - 7.0, rect.min.y + 4.0), Align2::RIGHT_TOP, "\u{2713}", FontId::proportional(14.0), ink);
                    }
                    let name = crate::i18n::trv(s.name);
                    ui.add(egui::Label::new(RichText::new(name).small()).wrap());
                    if resp.on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        settings.use_scheme(s);
                        settings.auto_dark = false;
                        picked = true;
                    }
                });
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
        ui.add_space(4.0);
    }
    picked
}

/// A grid of large colour swatches with their names; returns true when one was picked.
pub(crate) fn swatch_grid(ui: &mut egui::Ui, list: &[(&str, [u8; 3])], cur: &mut [u8; 3], outline: bool) -> bool {
    let mut picked = false;
    egui::Grid::new(ui.next_auto_id()).spacing([8.0, 8.0]).show(ui, |ui| {
        for (i, (name, c)) in list.iter().enumerate() {
            ui.vertical(|ui| {
                ui.set_width(52.0);
                let (rect, resp) = ui.allocate_exact_size(vec2(52.0, 26.0), Sense::click());
                let col = Color32::from_rgb(c[0], c[1], c[2]);
                ui.painter().rect_filled(rect, 5.0_f32, col);
                if outline || resp.hovered() {
                    ui.painter().rect_stroke(rect, 5.0_f32, Stroke::new(1.0_f32, Color32::GRAY));
                }
                if *cur == *c {
                    ui.painter().rect_stroke(rect.expand(2.5), 7.0_f32, Stroke::new(2.0_f32, ui.visuals().text_color()));
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, "\u{2713}", FontId::proportional(15.0), on_accent(col));
                }
                ui.label(RichText::new(*name).small());
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    *cur = *c;
                    picked = true;
                }
            });
            if (i + 1) % 8 == 0 {
                ui.end_row();
            }
        }
    });
    picked
}

pub(crate) fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

impl Settings {
    /// Switch to the plain light or dark theme, as the Light and Dark buttons do.
    pub(crate) fn set_dark(&mut self, dark: bool) {
        self.dark = dark;
        self.ink = None;
        if !dark {
            self.light_bg = default_light_bg();
        }
    }
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
            // the old default blue becomes the new default gold, once; a colour someone chose stays
            if !st.accent_gold {
                if st.accent == OLD_DEFAULT_ACCENT {
                    st.accent = DEFAULT_ACCENT;
                }
                st.accent_gold = true;
            }
            // the glass looks were renamed (plain colour names); the others are known by their colours
            st.glass = match st.glass.as_str() {
                "Glass" => "Light Glass",
                "Ocean Glass" => "Blue Glass",
                "Steel Glass" => "Blue Glass",
                g => g,
            }
            .to_string();
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
    // the program picks light or dark itself (or follows the computer through auto_dark), so egui
    // keeps one style and does not swap in its own when the computer changes
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
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
    // Glass: see-through panels over the soft backdrop (painted by paint_glass_backdrop), frosted
    // windows and menus, light edges, rounder corners.
    let glass = !settings.glass.is_empty();
    if glass {
        let a = glass_rgba;
        // Frosting (Settings > Appearance): from clear glass to milky, nearly solid panels
        let up = |lo: u8, hi: u8| frost_between(settings, lo, hi);
        if dark {
            v.panel_fill = a(30, 32, 38, up(12, 215));
            v.window_fill = a(24, 26, 34, up(220, 245));
            v.extreme_bg_color = a(0, 0, 0, up(90, 200));
            v.faint_bg_color = a(255, 255, 255, 10);
            v.widgets.noninteractive.bg_fill = a(255, 255, 255, 16);
            v.widgets.noninteractive.weak_bg_fill = a(255, 255, 255, 16);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, a(255, 255, 255, 50));
            for (w, al) in [(&mut v.widgets.inactive, 30u8), (&mut v.widgets.hovered, 55), (&mut v.widgets.active, 80), (&mut v.widgets.open, 55)] {
                w.bg_fill = a(255, 255, 255, al);
                w.weak_bg_fill = a(255, 255, 255, al);
                w.bg_stroke = Stroke::new(1.0_f32, a(255, 255, 255, al + 30));
            }
            v.window_stroke = Stroke::new(1.0_f32, a(255, 255, 255, 80));
        } else {
            v.panel_fill = a(255, 255, 255, up(70, 235));
            v.window_fill = a(250, 251, 253, up(228, 250));
            v.extreme_bg_color = a(255, 255, 255, up(200, 245));
            v.faint_bg_color = a(255, 255, 255, up(60, 150));
            v.widgets.noninteractive.bg_fill = a(255, 255, 255, up(110, 210));
            v.widgets.noninteractive.weak_bg_fill = a(255, 255, 255, up(110, 210));
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, a(20, 40, 70, 40));
            for (w, al) in [(&mut v.widgets.inactive, 150u8), (&mut v.widgets.hovered, 200), (&mut v.widgets.active, 235), (&mut v.widgets.open, 200)] {
                w.bg_fill = a(255, 255, 255, al);
                w.weak_bg_fill = a(255, 255, 255, al);
                w.bg_stroke = Stroke::new(1.0_f32, a(20, 40, 70, 36));
            }
            v.window_stroke = Stroke::new(1.0_f32, a(255, 255, 255, 230));
        }
        v.window_shadow = egui::epaint::Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: Color32::from_black_alpha(if dark { 110 } else { 45 }) };
        v.popup_shadow = egui::epaint::Shadow { offset: vec2(0.0, 6.0), blur: 18.0, spread: 0.0, color: Color32::from_black_alpha(if dark { 90 } else { 35 }) };
    }
    // High contrast: every button and box gets a clear outline in the text colour, and the text is
    // the theme's own colour everywhere.
    if settings.high_contrast {
        let ink = v.widgets.noninteractive.fg_stroke.color;
        let edge = mix(ink, v.panel_fill, 0.75);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.5_f32, edge);
        v.widgets.inactive.bg_stroke = Stroke::new(1.5_f32, edge);
        v.widgets.hovered.bg_stroke = Stroke::new(2.0_f32, ink);
        v.widgets.active.bg_stroke = Stroke::new(2.0_f32, ink);
        v.widgets.open.bg_stroke = Stroke::new(2.0_f32, ink);
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
            w.fg_stroke = Stroke::new(w.fg_stroke.width.max(1.5), ink);
        }
        v.window_stroke = Stroke::new(2.0_f32, ink);
    }
    crate::pedstyles::set_cb_colours(settings.cb_colours);
    let texts = [v.widgets.noninteractive.fg_stroke.color, v.widgets.inactive.fg_stroke.color, v.widgets.active.fg_stroke.color];
    // (with glass the panels are see-through, so judge against the backdrop's own colour)
    let behind = if glass { Color32::from_rgb(if dark { bg[0] } else { settings.light_bg[0] }, if dark { bg[1] } else { settings.light_bg[1] }, if dark { bg[2] } else { settings.light_bg[2] }) } else { v.panel_fill };
    let (sel_bg, sel_fg) = readable_selection(accent, behind, &texts);
    v.selection.bg_fill = sel_bg;
    v.selection.stroke = Stroke::new(if settings.high_contrast { 2.0_f32 } else { 1.0_f32 }, sel_fg);
    let r = Rounding::same(if glass { 10.0 } else { 6.0 });
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;
    v.window_rounding = Rounding::same(if glass { 16.0 } else { 8.0 });
    v.menu_rounding = Rounding::same(if glass { 12.0 } else { 6.0 });
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

    /// The easy-to-see schemes and the high contrast switch really give strong contrast.
    #[test]
    fn high_contrast_is_high() {
        let ctx = egui::Context::default();
        for name in ["High contrast light", "High contrast dark"] {
            let sc = SCHEMES.iter().find(|s| s.name == name).unwrap();
            let mut s = Settings::default();
            s.use_scheme(sc);
            s.high_contrast = true;
            apply_theme(&ctx, &s);
            let v = ctx.style().visuals.clone();
            assert!(contrast(v.widgets.noninteractive.fg_stroke.color, v.panel_fill) >= 15.0, "{name}: text");
            assert!(contrast(v.widgets.inactive.fg_stroke.color, v.widgets.inactive.bg_fill) >= 7.0, "{name}: button text");
            assert!(v.widgets.inactive.bg_stroke.width >= 1.5 && contrast(v.widgets.inactive.bg_stroke.color, v.panel_fill) >= 3.0, "{name}: buttons have a clear outline");
        }
        // every scheme with high contrast on still keeps selected things readable
        for sc in SCHEMES {
            let mut s = Settings::default();
            s.use_scheme(sc);
            s.high_contrast = true;
            apply_theme(&ctx, &s);
            let v = ctx.style().visuals.clone();
            assert!(contrast(v.selection.stroke.color, v.selection.bg_fill) >= 4.5, "{}", sc.name);
        }
    }

    /// The colour-blind friendly colours stay apart for people with red-green colour blindness
    /// (checked with the usual simulation of deuteranopia and protanopia), better than the usual ones.
    #[test]
    fn colour_blind_colours_stay_apart() {
        // Machado, Oliveira and Fernandes (2009), full severity
        let deut = [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]];
        let prot = [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]];
        let lin = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        let seen = |c: Color32, m: [[f32; 3]; 3]| {
            let x = [lin(c.r()), lin(c.g()), lin(c.b())];
            [0, 1, 2].map(|i| (m[i][0] * x[0] + m[i][1] * x[1] + m[i][2] * x[2]).clamp(0.0, 1.0).powf(1.0 / 2.2))
        };
        let apart = |a: Color32, b: Color32, m| {
            let (x, y) = (seen(a, m), seen(b, m));
            ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2) + (x[2] - y[2]).powi(2)).sqrt()
        };
        use crate::pedstyles::{father_side, man, mother_side, set_cb_colours, woman};
        for m in [deut, prot] {
            set_cb_colours(false);
            let (usual_mw, usual_sides) = (apart(man(), woman(), m), apart(father_side(), mother_side(), m));
            set_cb_colours(true);
            let (cb_mw, cb_sides) = (apart(man(), woman(), m), apart(father_side(), mother_side(), m));
            assert!(cb_mw > 0.35 && cb_mw > usual_mw, "men and women {cb_mw:.2} (usual {usual_mw:.2})");
            assert!(cb_sides > 0.35, "the two sides {cb_sides:.2} (usual {usual_sides:.2})");
        }
        set_cb_colours(false);
    }
}

/// A see-through colour that blends the way its numbers say: `al` out of 255 of the colour over
/// what is behind. (Color32::from_rgba_unmultiplied goes through linear light, which makes faint
/// white far stronger than asked for.)
pub(crate) fn glass_rgba(r: u8, g: u8, b: u8, al: u8) -> Color32 {
    let m = |c: u8| ((c as u16 * al as u16) / 255) as u8;
    Color32::from_rgba_premultiplied(m(r), m(g), m(b), al)
}

/// Paint a glass look's backdrop behind everything: a soft four-colour gradient with glows.
/// Called at the start of each frame, before the panels (which are see-through with glass).
/// A glass look's opacity between `lo` (clear glass) and `hi` (fully frosted), by the Frosting slider.
pub(crate) fn frost_between(settings: &Settings, lo: u8, hi: u8) -> u8 {
    let f = settings.glass_frost.clamp(0.0, 1.0);
    (lo as f32 + (hi as f32 - lo as f32) * f).round() as u8
}

pub(crate) fn default_frost() -> f32 {
    0.35
}

/// Whether this run's window was made see-through (it can only be chosen when the window opens).
pub(crate) static WINDOW_SEE_THROUGH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The window is see-through now: made so at the start, with a glass look in use.
pub(crate) fn see_through_now(settings: &Settings) -> bool {
    WINDOW_SEE_THROUGH.load(std::sync::atomic::Ordering::Relaxed) && settings.see_through && !settings.glass.is_empty()
}

pub(crate) fn paint_glass_backdrop(ctx: &egui::Context, settings: &Settings) {
    if settings.glass.is_empty() {
        return;
    }
    // see-through: only a faint wash of the backdrop, so the desktop shows behind
    let wash = if see_through_now(settings) { 0.3f32 + 0.65 * settings.glass_frost.clamp(0.0, 1.0) } else { 1.0 };
    let g = glass_backdrop(&settings.glass);
    let r = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let c = |x: [u8; 3]| if wash < 1.0 { glass_rgba(x[0], x[1], x[2], (255.0 * wash) as u8) } else { Color32::from_rgb(x[0], x[1], x[2]) };
    let mut mesh = egui::epaint::Mesh::default();
    // a 2 x 2 grid of the four corner colours, with a centre point for a smoother mix
    let pts = [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()];
    for (i, p) in pts.iter().enumerate() {
        mesh.colored_vertex(*p, c(g.corners[i]));
    }
    let mid = |a: [u8; 3], b: [u8; 3], cc: [u8; 3], d: [u8; 3]| [0, 1, 2].map(|i| ((a[i] as u16 + b[i] as u16 + cc[i] as u16 + d[i] as u16) / 4) as u8);
    mesh.colored_vertex(r.center(), c(mid(g.corners[0], g.corners[1], g.corners[2], g.corners[3])));
    for (a, b) in [(0u32, 1u32), (1, 3), (3, 2), (2, 0)] {
        mesh.add_triangle(a, b, 4);
    }
    painter.add(egui::Shape::mesh(mesh));
    // soft glows: a fan of triangles from a coloured centre out to a see-through edge
    let side = r.width().max(r.height());
    // a splash of colour, when chosen: two soft glows of the accent colour, one lighter
    let a = settings.accent;
    let strong = if settings.dark { 70 } else { 95 };
    let light = [0, 1, 2].map(|i| ((a[i] as u16 * 2 + 255) / 3) as u8);
    let splash = [([0.2f32, 0.3f32], 0.45f32, [a[0], a[1], a[2], strong]), ([0.85, 0.75], 0.4, [light[0], light[1], light[2], strong - 20])];
    let extra: &[([f32; 2], f32, [u8; 4])] = if settings.glass_splash { &splash } else { &[] };
    for (centre, rad, col) in g.glows.iter().chain(extra.iter()) {
        let ctr = egui::pos2(r.left() + centre[0] * r.width(), r.top() + centre[1] * r.height());
        let radius = rad * side;
        let mut m = egui::epaint::Mesh::default();
        m.colored_vertex(ctr, glass_rgba(col[0], col[1], col[2], col[3]));
        let n = 48;
        for k in 0..n {
            let t = k as f32 / n as f32 * std::f32::consts::TAU;
            m.colored_vertex(ctr + vec2(t.cos(), t.sin()) * radius, Color32::TRANSPARENT);
        }
        for k in 0..n {
            m.add_triangle(0, 1 + k, 1 + (k + 1) % n);
        }
        painter.add(egui::Shape::mesh(m));
    }
}
