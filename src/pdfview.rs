//! The built-in PDF viewer. Pages are drawn by `hayro` (pure Rust, no outside programs) on a
//! background thread, so a large page never freezes the window.

use super::*;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::{render, PixmapSettings, RenderCache, RenderSettings};
use std::sync::mpsc::{channel, Receiver};

type Rendered = Result<(usize, egui::ColorImage), String>;

pub struct PdfView {
    pub path: PathBuf,
    pub title: String,
    bytes: Arc<Vec<u8>>,
    pub pages: usize,
    pub page: usize,
    /// 0 = fit to the window width; otherwise a zoom factor (1.0 = actual size).
    pub zoom: f32,
    /// Size of each page in PDF points, for working out the zoom.
    sizes: Vec<(f32, f32)>,
    tex: Option<(usize, u32, egui::TextureHandle)>,
    job: Option<(usize, u32, Receiver<Rendered>)>,
    pub error: Option<String>,
}

/// Draw one page at `scale` (pixels per PDF point).
pub fn render_page(bytes: &Arc<Vec<u8>>, page: usize, scale: f32) -> Rendered {
    let pdf = Pdf::new(bytes.clone()).map_err(|e| format!("{:?}", e))?;
    let pages = pdf.pages();
    let p = pages.get(page).ok_or_else(|| "no such page".to_string())?;
    let cache = RenderCache::new();
    let px = render(
        p,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &PixmapSettings { x_scale: scale, y_scale: scale, bg_color: hayro::vello_cpu::color::palette::css::WHITE },
    );
    let (w, h) = (px.width() as usize, px.height() as usize);
    Ok((page, egui::ColorImage::from_rgba_premultiplied([w, h], px.data_as_u8_slice())))
}

impl PdfView {
    pub fn open(path: &Path, title: &str) -> Result<PdfView, String> {
        let bytes = Arc::new(std::fs::read(path).map_err(|e| io_msg(&e))?);
        let pdf = Pdf::new(bytes.clone()).map_err(|e| match format!("{:?}", e) {
            s if s.to_lowercase().contains("encrypt") || s.to_lowercase().contains("password") => "This PDF is password-protected, so it can only be opened in your PDF app.".to_string(),
            s => format!("This file could not be read as a PDF ({})", s),
        })?;
        let sizes: Vec<(f32, f32)> = pdf.pages().iter().map(|p| p.render_dimensions()).collect();
        if sizes.is_empty() {
            return Err("This PDF has no pages.".into());
        }
        Ok(PdfView { path: path.to_path_buf(), title: title.to_string(), bytes, pages: sizes.len(), page: 0, zoom: 0.0, sizes, tex: None, job: None, error: None })
    }

    /// Show the viewer. Returns false when the window was closed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let title = format!("{}  (page {} of {})", self.title, self.page + 1, self.pages);
        egui::Window::new(title).id(egui::Id::new("pdf_viewer")).open(&mut open).default_size([760.0, 860.0]).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(self.page > 0, egui::Button::new("< Previous")).clicked() {
                    self.page -= 1;
                }
                let mut shown = (self.page + 1) as u32;
                ui.label("Page");
                if ui.add(egui::DragValue::new(&mut shown).range(1..=self.pages as u32)).changed() {
                    self.page = (shown as usize).saturating_sub(1).min(self.pages - 1);
                }
                ui.label(format!("of {}", self.pages));
                if ui.add_enabled(self.page + 1 < self.pages, egui::Button::new("Next >")).clicked() {
                    self.page += 1;
                }
                ui.separator();
                if ui.selectable_label(self.zoom == 0.0, "Fit width").clicked() {
                    self.zoom = 0.0;
                }
                if ui.selectable_label((self.zoom - 1.0).abs() < 0.01, "Actual size").clicked() {
                    self.zoom = 1.0;
                }
                if ui.button("-").clicked() {
                    self.zoom = (self.current_zoom(ui.available_width()) / 1.25).max(0.25);
                }
                if ui.button("+").clicked() {
                    self.zoom = (self.current_zoom(ui.available_width()) * 1.25).min(5.0);
                }
                ui.label(format!("{:.0}%", self.current_zoom(ui.available_width()) * 100.0));
                ui.separator();
                if ui.button("Open in my PDF app").on_hover_text("To print, search the text or fill in forms").clicked() {
                    open_path(&self.path);
                }
            });
            // keyboard: arrows and Page Up/Down turn pages when nothing else is typing
            if !ui.ctx().wants_keyboard_input() {
                let (next, prev) = ui.input(|i| (i.key_pressed(egui::Key::PageDown) || i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::PageUp) || i.key_pressed(egui::Key::ArrowLeft)));
                if next && self.page + 1 < self.pages {
                    self.page += 1;
                }
                if prev && self.page > 0 {
                    self.page -= 1;
                }
            }
            ui.separator();
            let avail_w = ui.available_width();
            let z = self.current_zoom(avail_w);
            let ppp = ui.ctx().pixels_per_point();
            // render a little sharper than the screen needs, capped so huge pages stay quick
            let scale = (z * ppp).clamp(0.2, 6.0);
            let key = (scale * 100.0) as u32;
            let have = self.tex.as_ref().map(|t| (t.0, t.1)) == Some((self.page, key));
            if !have && self.job.as_ref().map(|j| (j.0, j.1)) != Some((self.page, key)) {
                let (tx, rx) = channel();
                let bytes = self.bytes.clone();
                let page = self.page;
                let ctx2 = ui.ctx().clone();
                std::thread::spawn(move || {
                    let r = std::panic::catch_unwind(|| render_page(&bytes, page, scale)).unwrap_or_else(|_| Err("This page could not be drawn.".into()));
                    let _ = tx.send(r);
                    ctx2.request_repaint();
                });
                self.job = Some((self.page, key, rx));
            }
            if let Some((p, k, rx)) = &self.job {
                if let Ok(r) = rx.try_recv() {
                    match r {
                        Ok((_, img)) => {
                            let t = ui.ctx().load_texture(format!("pdf-{}-{}", p, k), img, egui::TextureOptions::LINEAR);
                            self.tex = Some((*p, *k, t));
                            self.error = None;
                        }
                        Err(e) => self.error = Some(e),
                    }
                    self.job = None;
                }
            }
            if let Some(e) = &self.error {
                ui.colored_label(Color32::from_rgb(220, 120, 90), e);
                if ui.button("Open in my PDF app instead").clicked() {
                    open_path(&self.path);
                }
            }
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                match &self.tex {
                    Some((p, _, t)) if *p == self.page => {
                        let (pw, ph) = self.sizes[self.page];
                        let size = vec2(pw * z, ph * z);
                        ui.add(egui::Image::new((t.id(), size)));
                    }
                    Some((_, _, t)) => {
                        // the old page stays faintly on screen while the new one is drawn
                        let (pw, ph) = self.sizes[self.page];
                        ui.add(egui::Image::new((t.id(), vec2(pw * z, ph * z))).tint(Color32::from_white_alpha(90)));
                        ui.spinner();
                    }
                    None if self.error.is_none() => {
                        ui.spinner();
                        ui.label("Drawing the page...");
                    }
                    None => {}
                }
            });
        });
        open
    }

    fn current_zoom(&self, avail_w: f32) -> f32 {
        if self.zoom > 0.0 {
            return self.zoom;
        }
        let (pw, _) = self.sizes[self.page];
        ((avail_w - 16.0) / pw.max(1.0)).clamp(0.25, 5.0)
    }
}
