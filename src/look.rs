//! Making the program look and behave like the computer it runs on: the computer's own font,
//! tidy spacing, keyboard shortcuts written the way each system writes them, and one Settings
//! window (Cmd+, on a Mac, Ctrl+, elsewhere) for the choices that were spread over the menus.

use super::*;
use eframe::egui::{FontData, FontDefinitions, FontFamily};
use std::sync::{Mutex, OnceLock};

/// True on a Mac (decided when the program is built, so no Mac-only code is needed).
pub(crate) const MAC: bool = cfg!(target_os = "macos");

/// A keyboard shortcut written the way the computer writes it. `keys` is written the cross-
/// platform way: "Cmd+Z", "Cmd+Shift+Z", "F1", "Ctrl+Cmd+F". On a Mac that becomes ⌘Z, ⇧⌘Z;
/// elsewhere Cmd means Ctrl: Ctrl+Z, Ctrl+Shift+Z.
pub(crate) fn keys(k: &str) -> String {
    if MAC {
        let mut mods = String::new();
        let mut rest = String::new();
        for part in k.split('+') {
            match part {
                "Ctrl" => mods.push('⌃'),
                "Alt" => mods.push('⌥'),
                "Shift" => mods.push('⇧'),
                "Cmd" => mods.push('⌘'),
                p => rest.push_str(p),
            }
        }
        // Apple's order: Control, Option, Shift, Command
        let order = ['⌃', '⌥', '⇧', '⌘'];
        let mods: String = order.iter().filter(|c| mods.contains(**c)).collect();
        format!("{mods}{rest}")
    } else {
        // "Ctrl+Cmd+F" is a Mac-only shortcut; elsewhere the plain key does it
        let parts: Vec<&str> = k.split('+').collect();
        if parts.contains(&"Ctrl") && parts.contains(&"Cmd") {
            return String::new();
        }
        // the key's name as printed on the keyboard (German keyboards say Strg)
        k.replace("Cmd", "Ctrl").replace("Ctrl", crate::i18n::tr("Ctrl"))
    }
}

/// A menu item with its shortcut shown on the right, the way menus show them.
pub(crate) fn menu_item(text: &str, shortcut: &str) -> egui::Button<'static> {
    let b = egui::Button::new(text.to_string());
    let s = keys(shortcut);
    if s.is_empty() { b } else { b.shortcut_text(s) }
}

/// The word for leaving the program: "Quit Gilded Roots" on a Mac and Linux, "Exit" on Windows.
pub(crate) fn quit_label() -> &'static str {
    if cfg!(target_os = "windows") { crate::i18n::tr("Exit") } else { crate::i18n::tr("Quit Gilded Roots") }
}

/// The shortcut for leaving: Cmd+Q on a Mac, Ctrl+Q on Linux, Alt+F4 on Windows.
pub(crate) fn quit_keys() -> String {
    if cfg!(target_os = "windows") { "Alt+F4".to_string() } else { keys("Cmd+Q") }
}

// ---------------------------------------------------------------------------------------------
// Fonts

static SYSTEM_FONT: OnceLock<Option<(String, Vec<u8>, u32)>> = OnceLock::new();
static EXTRA_FONTS: Mutex<Vec<(String, FontData)>> = Mutex::new(Vec::new());
static USE_SYSTEM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Where the computer keeps its usual text font: (path, index in a font collection).
fn system_font_candidates() -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = Vec::new();
    if MAC {
        // San Francisco, the Mac's own font, then Helvetica Neue
        v.push(("/System/Library/Fonts/SFNS.ttf".into(), 0));
        v.push(("/System/Library/Fonts/HelveticaNeue.ttc".into(), 0));
        v.push(("/System/Library/Fonts/Helvetica.ttc".into(), 0));
    } else if cfg!(target_os = "windows") {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        v.push((format!("{windir}\\Fonts\\segoeui.ttf"), 0));
        v.push((format!("{windir}\\Fonts\\arial.ttf"), 0));
    } else {
        // the font the desktop is set to use, as fontconfig reports it
        for pat in ["system-ui", "sans-serif"] {
            if let Ok(out) = std::process::Command::new("fc-match").args(["-f", "%{file}|%{index}", pat]).output() {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                if let Some((f, i)) = s.split_once('|') {
                    if !f.is_empty() {
                        v.push((f.to_string(), i.trim().parse().unwrap_or(0)));
                    }
                }
            }
        }
        for p in [
            "/usr/share/fonts/cantarell/Cantarell-VF.otf",
            "/usr/share/fonts/opentype/cantarell/Cantarell-Regular.otf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ] {
            v.push((p.to_string(), 0));
        }
    }
    v
}

/// The computer's own text font, read once (None when none of the usual ones is there, or it
/// is a kind of font file that cannot be used).
fn system_font() -> Option<&'static (String, Vec<u8>, u32)> {
    SYSTEM_FONT
        .get_or_init(|| {
            for (path, index) in system_font_candidates() {
                let lower = path.to_lowercase();
                if !(lower.ends_with(".ttf") || lower.ends_with(".otf") || lower.ends_with(".ttc")) {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(&path) {
                    // only a font that can actually be read and has ordinary letters
                    if let Ok(f) = ab_glyph::FontRef::try_from_slice_and_index(&bytes, index) {
                        use ab_glyph::Font;
                        if f.glyph_id('a').0 != 0 && f.glyph_id('A').0 != 0 && f.glyph_id('é').0 != 0 {
                            return Some((path, bytes, index));
                        }
                    }
                }
            }
            None
        })
        .as_ref()
}

/// The name of the computer's own font, for the Settings window.
pub(crate) fn system_font_name() -> Option<String> {
    system_font().map(|(p, _, _)| Path::new(p).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
}

/// More fonts to fall back on for letters the main fonts lack (Japanese, Chinese, Korean).
pub(crate) fn add_fallback_fonts(list: Vec<(String, FontData)>) {
    if let Ok(mut g) = EXTRA_FONTS.lock() {
        g.extend(list);
    }
}

/// Use the computer's own font or the one built into the program.
pub(crate) fn set_use_system_font(on: bool) {
    USE_SYSTEM.store(on, Ordering::Relaxed);
}

/// The fonts as chosen: the computer's own (when wanted and found) first, then the program's
/// built-in fonts for anything it lacks (symbols, arrows), then the extra ones.
pub(crate) fn font_definitions() -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    if USE_SYSTEM.load(Ordering::Relaxed) {
        if let Some((_, bytes, index)) = system_font() {
            let mut fd = FontData::from_owned(bytes.clone());
            fd.index = *index;
            defs.font_data.insert("system-ui".into(), fd);
            if let Some(list) = defs.families.get_mut(&FontFamily::Proportional) {
                list.insert(0, "system-ui".into());
            }
        }
    }
    if let Ok(extra) = EXTRA_FONTS.lock() {
        for (name, fd) in extra.iter() {
            defs.font_data.insert(name.clone(), fd.clone());
            for fam in [FontFamily::Proportional, FontFamily::Monospace] {
                if let Some(list) = defs.families.get_mut(&fam) {
                    list.push(name.clone());
                }
            }
        }
    }
    defs
}

/// Put the chosen fonts to use.
pub(crate) fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

// ---------------------------------------------------------------------------------------------
// Spacing

/// Light, Dark or Automatic (following the computer). Returns whether anything changed.
pub(crate) fn light_dark_choice(ui: &mut egui::Ui, ctx: &egui::Context, settings: &mut Settings) -> bool {
    let tr = crate::i18n::tr;
    let mut changed = false;
    let auto = settings.auto_dark;
    if ui.selectable_label(!auto && !settings.dark, tr("Light")).clicked() {
        settings.auto_dark = false;
        settings.set_dark(false);
        changed = true;
    }
    if ui.selectable_label(!auto && settings.dark, tr("Dark")).clicked() {
        settings.auto_dark = false;
        settings.set_dark(true);
        changed = true;
    }
    let r = ui.selectable_label(auto, tr("Automatic")).on_hover_text(tr("Light or dark as your computer is set, changing when it changes"));
    if r.clicked() && !auto {
        settings.auto_dark = true;
        if let Some(t) = ctx.system_theme() {
            settings.set_dark(t == egui::Theme::Dark);
        }
        changed = true;
    }
    changed
}

/// The middle of the program's window, kept each frame so new windows can open there.
static SCREEN_MIDDLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn note_screen(ctx: &egui::Context) {
    let c = ctx.screen_rect().center();
    SCREEN_MIDDLE.store(((c.x.to_bits() as u64) << 32) | c.y.to_bits() as u64, std::sync::atomic::Ordering::Relaxed);
}

/// A Gilded Roots window: it opens in the middle of the program's window (and can then be moved).
pub(crate) fn window(title: impl Into<egui::WidgetText>) -> egui::Window<'static> {
    let v = SCREEN_MIDDLE.load(std::sync::atomic::Ordering::Relaxed);
    let mid = egui::pos2(f32::from_bits((v >> 32) as u32), f32::from_bits(v as u32));
    let w = egui::Window::new(title);
    if v == 0 { w } else { w.pivot(egui::Align2::CENTER_CENTER).default_pos(mid) }
}

/// A window's title, at the size a Mac or desktop window's title bar uses, so the bar stays slim.
pub(crate) fn wtitle(t: impl Into<String>) -> egui::RichText {
    egui::RichText::new(t).size(13.5)
}

/// Even spacing and comfortable buttons, closer to what people expect on a Mac or Linux desktop.
pub(crate) fn apply_spacing(style: &mut egui::Style) {
    let s = &mut style.spacing;
    s.item_spacing = vec2(8.0, 6.0);
    s.button_padding = vec2(8.0, 4.0);
    s.menu_margin = egui::Margin::symmetric(6.0, 6.0);
    // slim title bars (the margin above and below the title sets their height), as on a Mac
    s.window_margin = egui::Margin { left: 12.0, right: 12.0, top: 6.0, bottom: 10.0 };
    s.interact_size.y = s.interact_size.y.max(22.0);
    s.combo_width = s.combo_width.max(120.0);
    s.icon_spacing = 6.0;
    // menus a little wider, so the shortcuts on the right have room
    s.menu_width = s.menu_width.max(220.0);
}

// ---------------------------------------------------------------------------------------------
// The Settings window

pub(crate) const SETTINGS_PAGES: [&str; 9] = ["Appearance", "Colours", "Text and readability", "Language", "Names and dates", "Charts", "Toolbars", "Printing and privacy", "Tree, backups and microphone"];

impl App {
    /// A clear outline around whatever the keyboard is on. It shows once Tab is pressed (so people
    /// moving around with the keyboard can see where they are) and goes away at the next click.
    pub(crate) fn focus_ring(&mut self, ctx: &egui::Context) {
        if !self.settings.focus_ring {
            return;
        }
        let (tab, pointer) = ctx.input(|i| (i.key_pressed(egui::Key::Tab), i.pointer.any_pressed()));
        if tab {
            self.x.kbd_nav = true;
        } else if pointer {
            self.x.kbd_nav = false;
        }
        if !self.x.kbd_nav {
            return;
        }
        let Some(id) = ctx.memory(|m| m.focused()) else { return };
        let Some(r) = ctx.read_response(id) else { return };
        let v = ctx.style().visuals.clone();
        let col = if v.dark_mode { lighten(v.hyperlink_color) } else { v.hyperlink_color };
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("focus_ring")));
        painter.rect_stroke(r.rect.expand(3.0), 6.0_f32, Stroke::new(2.5_f32, col));
    }

    /// Settings, laid out like a desktop program's preferences: the pages down the left, and each
    /// page in titled panels that fit without scrolling.
    pub(crate) fn settings_window(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        if !self.x.settings_open {
            return;
        }
        let mut open = true;
        let mut changed = false;
        let (w, h) = (860.0, 540.0);
        crate::look::window(crate::look::wtitle(crate::i18n::tr("Settings")))
            .id(egui::Id::new("settings_window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size([w, h])
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // the pages, down the left, on a slightly darker strip
                    egui::Frame::none().fill(ui.visuals().faint_bg_color).rounding(6.0).inner_margin(6.0).show(ui, |ui| {
                        ui.allocate_ui_with_layout(vec2(196.0, h - 12.0), egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            ui.set_width(196.0);
                            ui.set_min_height(h - 12.0);
                            ui.spacing_mut().button_padding = vec2(10.0, 6.0);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for (i, p) in SETTINGS_PAGES.iter().enumerate() {
                                if ui.selectable_label(self.x.settings_page == i, crate::i18n::trv(p)).clicked() {
                                    self.x.settings_page = i;
                                }
                            }
                        });
                    });
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.set_width(w - 222.0);
                        ui.set_height(h);
                        let page = SETTINGS_PAGES.get(self.x.settings_page).copied().unwrap_or("");
                        ui.label(RichText::new(crate::i18n::trv(page)).size(17.0).strong());
                        ui.add_space(6.0);
                        // only when the text is made very large does a page ever need to scroll
                        egui::ScrollArea::vertical().max_height(h - 70.0).auto_shrink([false, true]).show(ui, |ui| {
                            changed |= self.settings_page(ui, ctx, act);
                        });
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            if ui.button(RichText::new(crate::i18n::tr("Done")).strong()).clicked() {
                                self.x.settings_open = false;
                            }
                        });
                    });
                });
            });
        if changed {
            save_settings(&self.settings);
            apply_theme(ctx, &self.settings);
        }
        if !open {
            self.x.settings_open = false;
        }
    }

    /// One page of the Settings window. Returns whether anything changed.
    fn settings_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, act: &mut Option<Action>) -> bool {
        let tr = crate::i18n::tr;
        let mut changed = false;
        // a titled panel, as in a desktop program's preferences
        let section = |ui: &mut egui::Ui, t: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
            egui::Frame::group(ui.style()).rounding(6.0).inner_margin(egui::Margin::symmetric(12.0, 8.0)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(t).strong());
                ui.add_space(2.0);
                add(ui);
            });
            ui.add_space(6.0);
        };
        match self.x.settings_page {
            0 => {
                section(ui, tr("Light or dark"), &mut |ui| {
                    ui.horizontal(|ui| {
                        changed |= light_dark_choice(ui, ctx, &mut self.settings);
                    });
                });
                section(ui, tr("Colour scheme"), &mut |ui| {
                    changed |= scheme_list(ui, &mut self.settings);
                    if !self.settings.glass.is_empty() {
                        ui.add_space(4.0);
                        changed |= ui.checkbox(&mut self.settings.glass_splash, tr("A splash of colour behind the glass")).on_hover_text(tr("Soft glows of the accent colour (Colours page) behind the see-through panels")).changed();
                        changed |= ui.checkbox(&mut self.settings.see_through, tr("Transparent window: your desktop shows behind the glass")).on_hover_text(tr("Takes effect the next time Gilded Roots starts")).changed();
                    }
                });
                if ui.button(tr("Back to the Gilded Roots look")).on_hover_text(tr("The colours Gilded Roots comes with, light or dark as now")).clicked() {
                    let d = Settings::default();
                    let dark = self.settings.dark;
                    self.settings.accent = d.accent;
                    self.settings.bg = d.bg;
                    self.settings.light_bg = d.light_bg;
                    self.settings.ink = d.ink;
                    self.settings.glass = d.glass;
                    self.settings.dark = dark;
                    changed = true;
                }
            }
            1 => {
                section(ui, tr("Accent colour"), &mut |ui| {
                    changed |= swatch_grid(ui, &PRESETS, &mut self.settings.accent, false);
                    ui.horizontal(|ui| {
                        ui.label(tr("Custom"));
                        changed |= ui.color_edit_button_srgb(&mut self.settings.accent).changed();
                    });
                });
                if self.settings.dark {
                    section(ui, tr("Dark background"), &mut |ui| {
                        changed |= swatch_grid(ui, &BG_PRESETS, &mut self.settings.bg, true);
                        ui.horizontal(|ui| {
                            ui.label(tr("Custom"));
                            changed |= ui.color_edit_button_srgb(&mut self.settings.bg).changed();
                        });
                    });
                } else {
                    ui.label(RichText::new(tr("The background colours are for the dark look.")).small().weak());
                }
            }
            2 => {
                section(ui, tr("Text size"), &mut |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (v, l) in [(0.85f32, "Small"), (1.0, "Normal"), (1.15, "Large"), (1.3, "Larger"), (1.5, "Largest")] {
                            if ui.selectable_label((self.settings.ui_scale - v).abs() < 0.01, crate::i18n::trv(l)).clicked() {
                                self.settings.ui_scale = v;
                                changed = true;
                            }
                        }
                    });
                });
                section(ui, tr("Font"), &mut |ui| {
                    let own = system_font_name();
                    let mut sys = self.settings.system_font;
                    ui.add_enabled_ui(own.is_some(), |ui| {
                        if ui.radio_value(&mut sys, true, match &own {
                            Some(n) => crate::i18n::trf("The computer's own font ({})", &["The computer's own font (", ")"], &[n.clone()]),
                            None => tr("The computer's own font (not found on this computer)").to_string(),
                        }).changed() {
                            changed = true;
                        }
                    });
                    if ui.radio_value(&mut sys, false, tr("The font built into Gilded Roots")).changed() {
                        changed = true;
                    }
                    if sys != self.settings.system_font {
                        self.settings.system_font = sys;
                        set_use_system_font(sys);
                        install_fonts(ctx);
                    }
                });
                section(ui, tr("Easier to see"), &mut |ui| {
                    if ui.checkbox(&mut self.settings.high_contrast, tr("High contrast: clear outlines on every button, box and chart")).changed() {
                        changed = true;
                    }
                    if ui.checkbox(&mut self.settings.cb_colours, tr("Colour-blind friendly chart colours (blue and orange)")).on_hover_text(tr("Men and women, and the father's and mother's sides, in colours people with red-green colour blindness can tell apart")).changed() {
                        changed = true;
                    }
                    if ui.checkbox(&mut self.settings.focus_ring, tr("Show where the keyboard is (an outline after pressing Tab)")).changed() {
                        changed = true;
                    }
                    ui.label(RichText::new(tr("The \"Easy to see\" and \"Heritage\" colour schemes (Appearance) include high-contrast and sepia looks.")).small().weak());
                });
            }
            3 => {
                if crate::i18n::offered().any(|l| matches!(l.0, "ja" | "zh" | "ko")) {
                    crate::i18n::add_asian_fonts(ctx);
                }
                let cur = if self.settings.language.is_empty() { "en" } else { self.settings.language.as_str() };
                let mut pick = None;
                section(ui, tr("Language"), &mut |ui| {
                    egui::Grid::new("lang_grid").num_columns(3).spacing([16.0, 4.0]).min_col_width(170.0).show(ui, |ui| {
                        for (i, &(code, own, english, _)) in crate::i18n::offered().enumerate() {
                            let text = if own == english { own.to_string() } else { format!("{own}  ({english})") };
                            if ui.selectable_label(cur == code, text).clicked() {
                                pick = Some(code);
                            }
                            if i % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                    ui.label(RichText::new(tr("Anything not translated yet stays in English.")).small().weak());
                });
                if let Some(code) = pick {
                    self.settings.language = if code == "en" { String::new() } else { code.to_string() };
                    crate::i18n::set_language(&self.settings.language);
                    changed = true;
                }
            }
            4 => {
                section(ui, tr("Names"), &mut |ui| {
                    for (v, l) in [(0u8, "John Henry Smith"), (1, "SMITH, John Henry"), (2, "Smith, John Henry")] {
                        if ui.radio(self.settings.name_style == v, crate::i18n::trv(l)).clicked() {
                            self.settings.name_style = v;
                            changed = true;
                        }
                    }
                });
                section(ui, tr("Dates"), &mut |ui| {
                    for (v, l) in [(0u8, "As typed"), (1, "12 Mar 1921"), (2, "March 12, 1921"), (3, "1921-03-12")] {
                        if ui.radio(self.settings.date_style == v, crate::i18n::trv(l)).clicked() {
                            self.settings.date_style = v;
                            changed = true;
                        }
                    }
                    ui.label(RichText::new(tr("Only changes how dates look. What you typed is kept.")).small().weak());
                });
            }
            5 => {
                section(ui, tr("Generations"), &mut |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for n in 3..=8usize {
                            if ui.selectable_label(self.settings.gens == n, n.to_string()).clicked() {
                                *act = Some(Action::SetGens(n));
                            }
                        }
                    });
                });
                section(ui, tr("Chart boxes"), &mut |ui| {
                    changed |= ui.checkbox(&mut self.settings.box_photos, tr("Show photos")).changed();
                    changed |= ui.checkbox(&mut self.settings.box_places, tr("Show birthplace")).changed();
                    changed |= ui.checkbox(&mut self.settings.box_ids, tr("Show person ID")).changed();
                });
                section(ui, tr("Profile photo size"), &mut |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (v, l) in [(0u8, "Small"), (1, "Normal"), (2, "Large"), (3, "Extra large")] {
                            if ui.selectable_label(self.settings.photo_size == v, crate::i18n::trv(l)).clicked() {
                                self.settings.photo_size = v;
                                changed = true;
                            }
                        }
                    });
                    ui.label(RichText::new(tr("The photo at the top of the Individual page.")).small().weak());
                });
            }
            6 => {
                section(ui, tr("Toolbars"), &mut |ui| {
                    changed |= ui.checkbox(&mut self.settings.tb_main, tr("Show main toolbar")).changed();
                    changed |= ui.checkbox(&mut self.settings.tb_tab, tr("Show tab toolbars")).changed();
                    changed |= ui.checkbox(&mut self.settings.tb_words, tr("Toolbar words beside the pictures")).changed();
                    changed |= ui.checkbox(&mut self.settings.tb_large, tr("Large toolbar buttons")).changed();
                    changed |= ui.checkbox(&mut self.settings.float_tb, tr("Floating toolbar beside the charts")).on_hover_text(tr("Select or hand, centre, home person, zoom and print, down the left side of the pedigree and descendant charts")).changed();
                    if !self.settings.tb_hidden.is_empty() && ui.button(tr("Show every toolbar button again")).clicked() {
                        self.settings.tb_hidden.clear();
                        changed = true;
                    }
                    ui.label(RichText::new(tr("Right-click a toolbar to choose its buttons.")).small().weak());
                });
            }
            7 => {
                section(ui, tr("Privacy"), &mut |ui| {
                    changed |= ui.checkbox(&mut self.settings.hide_living, tr("Keep living people private")).on_hover_text(tr("Printed charts, sheets and reports show living people as \"Living\" with no dates or places")).changed();
                });
                section(ui, tr("Paper"), &mut |ui| {
                    for (v, l) in [(false, "US Letter (8.5 x 11 in)"), (true, "A4 (210 x 297 mm)")] {
                        if ui.radio(self.settings.paper_a4 == v, crate::i18n::trv(l)).clicked() {
                            self.settings.paper_a4 = v;
                            PAPER_A4.store(v, Ordering::Relaxed);
                            changed = true;
                        }
                    }
                });
            }
            _ => {
                section(ui, tr("Your tree and its backups"), &mut |ui| {
                    ui.label(RichText::new(tr("Where your tree is kept, how often it is backed up, how many backups to keep, and a second backup folder (a USB drive, say).")).small());
                    if ui.button(tr("Where my tree is kept...")).clicked() {
                        self.show_loc = true;
                    }
                });
                section(ui, tr("Research journal"), &mut |ui| {
                    changed |= ui.checkbox(&mut self.settings.auto_log, tr("Note what I do in the research journal")).changed();
                    ui.label(RichText::new(tr("Adding research files is written in the journal by itself, with the day and the person.")).small().weak());
                });
                section(ui, tr("Microphone"), &mut |ui| {
                    let cur = if self.settings.mic.is_empty() { tr("System default").to_string() } else { self.settings.mic.clone() };
                    egui::ComboBox::from_id_salt("settings_mic").selected_text(cur).width(300.0).show_ui(ui, |ui| {
                        if ui.selectable_label(self.settings.mic.is_empty(), tr("System default")).clicked() {
                            self.settings.mic.clear();
                            changed = true;
                        }
                        for n in input_device_names() {
                            if ui.selectable_label(self.settings.mic == n, n.as_str()).clicked() {
                                self.settings.mic = n.clone();
                                changed = true;
                            }
                        }
                    });
                    ui.label(RichText::new(tr("Used for recording spoken stories.")).small().weak());
                });
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_are_written_the_computer_s_way() {
        if MAC {
            assert_eq!(keys("Cmd+Z"), "⌘Z");
            assert_eq!(keys("Cmd+Shift+Z"), "⇧⌘Z");
            assert_eq!(keys("Ctrl+Cmd+F"), "⌃⌘F");
            assert_eq!(keys("F1"), "F1");
        } else {
            assert_eq!(keys("Cmd+Z"), "Ctrl+Z");
            assert_eq!(keys("Cmd+Shift+Z"), "Ctrl+Shift+Z");
            assert_eq!(keys("Ctrl+Cmd+F"), "", "Mac-only shortcut not shown");
            assert_eq!(keys("F11"), "F11");
        }
    }

    #[test]
    fn fonts_always_include_the_built_in_ones() {
        set_use_system_font(true);
        let d = font_definitions();
        let prop = &d.families[&FontFamily::Proportional];
        // the built-in fonts stay as fallbacks for symbols and arrows
        for name in FontDefinitions::default().families[&FontFamily::Proportional].iter() {
            assert!(prop.contains(name), "{name} kept");
        }
        if system_font().is_some() {
            assert_eq!(prop[0], "system-ui", "the computer's font comes first");
        }
        set_use_system_font(false);
        let d = font_definitions();
        assert!(!d.families[&FontFamily::Proportional].contains(&"system-ui".to_string()));
        set_use_system_font(true);
    }

    #[test]
    fn every_settings_page_draws() {
        let mut app = App::for_test(crate::sample::sample_family());
        app.x.settings_open = true;
        let ctx = egui::Context::default();
        for p in 0..SETTINGS_PAGES.len() {
            app.x.settings_page = p;
            let _ = ctx.run(Default::default(), |c| app.frame(c));
            assert!(app.x.settings_open);
        }
    }
}
