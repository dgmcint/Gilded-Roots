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
mod gedcom;
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
#[cfg(test)]
mod bigtree;
pub(crate) use model::*;
pub(crate) use genealogy::*;
pub(crate) use settings::*;
pub(crate) use storage::*;
pub(crate) use help::*;
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
    if !st.current_tree.is_empty() && base_dir().join("More trees").join(&st.current_tree).is_dir() {
        if let Ok(mut g) = CURRENT_TREE.lock() {
            *g = Some(st.current_tree.clone());
        }
    }
    let size = st.win.map(|w| [w[0].max(800.0), w[1].max(560.0)]).unwrap_or([1400.0, 880.0]);
    let mut vb = egui::ViewportBuilder::default().with_inner_size(size).with_min_inner_size([800.0, 560.0]).with_title(if st.current_tree.is_empty() { "Gilded Roots".to_string() } else { format!("Gilded Roots - {}", st.current_tree) });
    if st.win_max {
        vb = vb.with_maximized(true);
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
