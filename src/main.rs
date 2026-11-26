#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, RichText, Rounding, Sense, Stroke};
use serde::{Deserialize, Serialize};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

mod archive;
mod census;
mod forms;
mod charts;
mod designer;
mod dna;
mod dnaline;
mod stats;
mod journal;
mod mapview;
mod pdfview;
mod table;
mod tombstone;
mod paf;
mod aq;
mod pedstyles;
mod homepage;
mod gedcom;
mod ged7;
mod website;
mod macmenu;
mod review;
mod sample;
mod temples;
use sample::sample_family;
use charts::ChartKind;
use gedcom::*;
use review::{ImportMerge, PersonReview};

// The main program, split by topic. Each file can see everything here.
mod model;
mod genealogy;
mod settings;
mod storage;
mod help;
mod media;
mod memories;
mod mediafolders;
mod importopts;
mod toolbar;
mod look;
pub(crate) use look::*;
mod research;
mod notepad;
mod searchall;
mod theories;
pub(crate) use importopts::*;
pub(crate) use mediafolders::*;
pub(crate) use memories::*;
mod reports;
mod app;
mod files;
mod windows;
mod jump;
mod views;
mod person;
mod update;
mod letters;
mod groups;
mod lists;
mod books;
mod tools;
mod notes;
mod templates;
mod i18n;
mod tour;

/// Tests that point HOME at a folder of their own hold this, so tests running at the same time
/// never write photos or settings into another test's folder.
#[cfg(test)]
pub(crate) static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn home_lock() -> std::sync::MutexGuard<'static, ()> {
    HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
#[cfg(test)]
mod bigtree;
pub(crate) use model::*;
pub(crate) use genealogy::*;
pub(crate) use settings::*;
pub(crate) use storage::*;
pub(crate) use media::*;
pub(crate) use reports::*;
pub(crate) use app::*;
pub(crate) use views::*;

fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../icons/icon-256.png")).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData { rgba: img.into_raw(), width, height })
}

fn main() -> eframe::Result<()> {
    // Open at the size it was last closed at; the first time, a good size that is then fitted
    // to the screen on the first frame (most of the screen, centred, not full screen).
    let st = load_settings();
    i18n::set_language(&st.language);
    if !st.current_tree.is_empty() && base_dir().join("More trees").join(&st.current_tree).is_dir() {
        if let Ok(mut g) = CURRENT_TREE.lock() {
            *g = Some(st.current_tree.clone());
        }
    }
    let size = st.win.map(|w| [w[0].max(800.0), w[1].max(560.0)]).unwrap_or([1400.0, 880.0]);
    let mut vb = egui::ViewportBuilder::default().with_inner_size(size).with_min_inner_size([800.0, 560.0]).with_title(if st.current_tree.is_empty() { "Gilded Roots".to_string() } else { format!("Gilded Roots - {}", st.current_tree) });
    // on Linux, the name the desktop matches with gilded-roots.desktop, so the dock shows our icon
    vb = vb.with_app_id("gilded-roots");
    if st.win_max {
        vb = vb.with_maximized(true);
    }
    // see-through, when chosen with a glass look (the desktop shows behind the window)
    if st.see_through && !st.glass.is_empty() {
        vb = vb.with_transparent(true);
        settings::WINDOW_SEE_THROUGH.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if let Some(icon) = window_icon() {
        vb = vb.with_icon(icon);
    }
    let opts = eframe::NativeOptions { viewport: vb, ..Default::default() };
    eframe::run_native(
        "Gilded Roots",
        opts,
        Box::new(|cc| {
            let app = App::new();
            egui_extras::install_image_loaders(&cc.egui_ctx);
            set_use_system_font(app.settings.system_font);
            install_fonts(&cc.egui_ctx);
            if matches!(i18n::language().as_str(), "ja" | "zh" | "ko") || app.db.people.values().any(|p| i18n::has_asian(&p.name)) {
                i18n::add_asian_fonts(&cc.egui_ctx);
            }
            apply_theme(&cc.egui_ctx, &app.settings);
            Ok(Box::new(app))
        }),
    )
}


#[cfg(test)]
mod tests;

/// For leaving empty yes/no fields out of the saved tree.
pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}
