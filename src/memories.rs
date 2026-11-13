//! Memories: each person's photos, documents, stories and recordings on their Individual page,
//! and the family-wide Memories tab (which took over from the old Gallery tab).

use super::*;

/// The filters on a person's Memories section (kind numbers as in `media_kind`).
pub(crate) const PERSON_KINDS: [(u8, &str); 5] = [(0, "All"), (1, "Photos"), (2, "Documents"), (4, "Stories"), (3, "Recordings")];

/// The filters on the Memories tab: the person filters plus notes and files nobody uses.
pub(crate) const TAB_KINDS: [(u8, &str); 7] = [(0, "All"), (1, "Photos"), (2, "Documents"), (4, "Stories"), (3, "Recordings"), (5, "Notes"), (6, "Not linked to anyone")];

/// Files in the media folder that nobody in the tree uses (for "Not linked to anyone"),
/// newest first. Previews, removed files and backups are not included.
pub(crate) fn unlinked_files(db: &Db) -> Vec<(String, u64)> {
    let used = media_in_use(db);
    let mut out: Vec<(String, u64, std::time::SystemTime)> = all_media_files(&media_dir()).into_iter().filter(|(n, _, _)| !used.contains(n)).collect();
    out.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    out.into_iter().map(|(n, s, _)| (n, s)).collect()
}

/// A label in a box of fixed width, lined up on the left (cut short with ... if too long), so
/// lists line up in columns.
pub(crate) fn fixed_label(ui: &mut egui::Ui, w: f32, text: impl Into<egui::WidgetText>) {
    ui.allocate_ui_with_layout(vec2(w, ui.spacing().interact_size.y), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_width(w);
        ui.add(egui::Label::new(text).truncate());
    });
}

/// A file size in words people use.
pub(crate) fn size_words(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1} MB", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{} KB", n / 1_000)
    } else {
        format!("{} bytes", n)
    }
}

impl App {
    /// The Memories section of the Individual page: everything kept for one person, with a
    /// filter, one Add button that takes any kind of file, and photos shown as pictures.
    /// `editing` adds captions to type into and Remove buttons.
    pub(crate) fn person_memories(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, editing: bool, pdf_req: &mut Option<(PathBuf, String)>) {
        let Some(p) = self.db.people.get(&id) else { return };
        let media: Vec<(usize, Media, u8)> = p.media.iter().cloned().enumerate().map(|(i, m)| {
            let k = media_kind(&m);
            (i, m, k)
        }).collect();
        let tagged = self.tagged_in(id).len();
        let count = |k: u8| if k == 0 { media.len() } else { media.iter().filter(|x| x.2 == k).count() };
        let mut kind = self.settings.pmem_kind;
        if kind != 0 && count(kind) == 0 && !(kind == 1 && tagged > 0) {
            kind = 0;
        }
        ui.horizontal_wrapped(|ui| {
            for (k, l) in PERSON_KINDS {
                let n = count(k);
                if k != 0 && n == 0 && !(k == 1 && tagged > 0) {
                    continue;
                }
                if ui.selectable_label(kind == k, format!("{} ({})", l, n)).clicked() {
                    self.settings.pmem_kind = k;
                    save_settings(&self.settings);
                }
            }
            ui.separator();
            ui.menu_button(RichText::new("+ Add").strong(), |ui| {
                if ui.button("Photos, documents or recordings...").on_hover_text("Any file: photos, PDFs, letters, certificates, sound or video").clicked() {
                    *act = Some(Action::AttachFiles);
                    ui.close_menu();
                }
                if ui.button("Write a story...").clicked() {
                    self.x.story = Some(StoryEdit { who: id, idx: None, title: String::new(), text: String::new() });
                    ui.close_menu();
                }
                if self.rec.is_none() && ui.button("Record a spoken story").clicked() {
                    *act = Some(Action::StartRec);
                    ui.close_menu();
                }
            });
            if let Some(r) = &self.rec {
                let secs = r.started.elapsed().as_secs();
                if ui.button(format!("Stop recording ({}:{:02})", secs / 60, secs % 60)).clicked() {
                    *act = Some(Action::StopRec);
                }
                let level = (r.peak.swap(0, Ordering::Relaxed) as f32 / 32768.0).sqrt().clamp(0.0, 1.0);
                ui.add(egui::ProgressBar::new(level).desired_width(120.0).text("mic level"));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            }
            if editing {
                // their own folder, once the media folder is sorted by person
                let own = self.db.people.get(&id).map(|p| media_dir().join(person_folder_name(id, p))).filter(|d| self.db.media_folders && d.is_dir());
                let target = own.unwrap_or_else(media_dir);
                if ui.button(if self.db.media_folders { "Open their folder" } else { "Open media folder" }).on_hover_text(format!("{}", target.display())).clicked() {
                    open_path(&target);
                }
                if self.rec.is_none() {
                    let cur = if self.settings.mic.is_empty() { "Microphone: system default".to_string() } else { format!("Microphone: {}", self.settings.mic) };
                    egui::ComboBox::from_id_salt("mic").selected_text(cur).show_ui(ui, |ui| {
                        if ui.selectable_label(self.settings.mic.is_empty(), "System default").clicked() {
                            self.settings.mic.clear();
                            save_settings(&self.settings);
                        }
                        for n in input_device_names() {
                            if ui.selectable_label(self.settings.mic == n, n.as_str()).clicked() {
                                self.settings.mic = n.clone();
                                save_settings(&self.settings);
                            }
                        }
                    });
                }
            }
        });
        if media.is_empty() && tagged == 0 {
            ui.label(RichText::new("Nothing here yet. Use + Add for photos, documents, stories and recordings.").italics().weak());
            return;
        }
        let show = |k: u8| kind == 0 || kind == k;
        // photos as pictures
        let photos: Vec<&(usize, Media, u8)> = media.iter().filter(|x| x.2 == 1).collect();
        if show(1) && !photos.is_empty() {
            let w = if editing { 150.0 } else { 120.0 };
            ui.horizontal_wrapped(|ui| {
                for (i, m, _) in photos {
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        let size = vec2(w, (w * 0.74).round());
                        if online_photo(m) {
                            if ui.add_sized(size, egui::Button::new("Photo stored online\nClick to open")).clicked() {
                                ui.ctx().open_url(egui::OpenUrl::new_tab(m.path.clone()));
                            }
                        } else if m.thumb.is_empty() || m.thumb == "-" {
                            let label = if m.thumb == "-" { "No preview\nClick to open" } else { "Loading..." };
                            if ui.add_sized(size, egui::Button::new(label)).clicked() {
                                // still making its preview: open it in our own viewer; only a photo
                                // that cannot be previewed at all goes to the computer's own app
                                if m.thumb.is_empty() {
                                    self.viewing = Some((id, *i));
                                } else {
                                    open_media(&m.file);
                                }
                            }
                        } else {
                            let r = ui.add(egui::Image::new(img_uri(&mprev(&m.thumb))).fit_to_exact_size(size).rounding(4.0).sense(Sense::click()));
                            if r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Open; tag faces and choose the profile photo there").clicked() {
                                self.viewing = Some((id, *i));
                            }
                        }
                        if editing {
                            if let Some(mm) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(*i)) {
                                ui.add(egui::TextEdit::singleline(&mut mm.caption).hint_text("Caption").desired_width(w));
                            }
                        } else {
                            let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                            ui.add(egui::Label::new(RichText::new(cap).small()).truncate());
                        }
                    });
                }
            });
        }
        if show(1) {
            self.tagged_strip(ui, id);
        }
        // everything else as a list
        let mut remove: Option<usize> = None;
        let mut story_open: Option<usize> = None;
        let others: Vec<&(usize, Media, u8)> = media.iter().filter(|x| x.2 != 1 && show(x.2)).collect();
        if !others.is_empty() {
            ui.add_space(4.0);
        }
        for (i, m, k) in others {
            let cap = if m.caption.is_empty() { m.name.clone() } else { m.caption.clone() };
            let online = m.file.is_empty() && is_url(&m.path);
            ui.horizontal(|ui| {
                let badge = match k {
                    3 => "Recording",
                    4 => "Story",
                    _ if is_pdf(&m.file) => "PDF",
                    _ => "Document",
                };
                fixed_label(ui, 70.0, RichText::new(badge).small().color(ui.visuals().selection.bg_fill));
                if editing {
                    if let Some(mm) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(*i)) {
                        ui.add(egui::TextEdit::singleline(&mut mm.caption).hint_text(m.name.as_str()).desired_width(260.0));
                    }
                } else {
                    ui.label(cap.as_str());
                }
                if online {
                    ui.hyperlink_to("Open online", m.path.as_str());
                } else if m.file.is_empty() {
                    ui.label(RichText::new(format!("File not found (the GEDCOM file named it {})", if m.path.is_empty() { "nothing" } else { m.path.as_str() })).small().weak());
                } else {
                    let label = if *k == 3 { "Play" } else if *k == 4 || is_pdf(&m.file) { "Read" } else { "Open" };
                    if ui.small_button(label).clicked() {
                        if *k == 4 && m.file.ends_with(".txt") {
                            story_open = Some(*i);
                        } else if is_pdf(&m.file) {
                            *pdf_req = Some((mfile(&m.file), cap.clone()));
                        } else {
                            open_media(&m.file);
                        }
                    }
                }
                if editing && ui.small_button("Remove").on_hover_text("Take it off this person. The file stays in the media folder; Undo (Ctrl+Z) puts it back.").clicked() {
                    remove = Some(*i);
                }
            });
        }
        if let Some(i) = remove {
            if let Some(p) = self.db.people.get_mut(&id) {
                if i < p.media.len() {
                    p.media.remove(i);
                    self.dirty = true;
                }
            }
        }
        if let Some(i) = story_open {
            if let Some(m) = self.db.people.get(&id).and_then(|p| p.media.get(i)) {
                let text = safe_media_path(&m.file).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
                self.x.story = Some(StoryEdit { who: id, idx: Some(i), title: m.caption.clone(), text });
            }
        }
        if editing {
            ui.label(RichText::new("Photos, documents and recordings are kept on this computer only, in the media folder.").small().weak());
        }
    }
}
