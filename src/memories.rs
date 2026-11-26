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
            if !media.is_empty() && !self.x.gal_pick && ui.button(crate::i18n::tr("Choose several...")).on_hover_text(crate::i18n::tr("Tick several memories to remove them together")).clicked() {
                self.x.gal_pick = true;
                self.x.gal_sel.clear();
            }
            ui.menu_button(RichText::new(crate::i18n::tr("+ Add")).strong(), |ui| {
                if ui.button(crate::i18n::tr("Photos, documents or recordings...")).on_hover_text(crate::i18n::tr("Any file: photos, PDFs, letters, certificates, sound or video")).clicked() {
                    *act = Some(Action::AttachFiles);
                    ui.close_menu();
                }
                if ui.button(crate::i18n::tr("Write a story...")).clicked() {
                    self.x.story = Some(StoryEdit { who: id, idx: None, title: String::new(), text: String::new() });
                    ui.close_menu();
                }
                if self.rec.is_none() && ui.button(crate::i18n::tr("Record a spoken story")).clicked() {
                    *act = Some(Action::StartRec);
                    ui.close_menu();
                }
            });
            if let Some(r) = &self.rec {
                let secs = r.started.elapsed().as_secs();
                if ui.button(crate::i18n::trf("Stop recording ({}:{:02})", &["Stop recording (", ":", ")"], &[(secs / 60).to_string(), format!("{:02}", secs % 60)])).clicked() {
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
                if ui.button(if self.db.media_folders { crate::i18n::tr("Open their folder") } else { crate::i18n::tr("Open media folder") }).on_hover_text(format!("{}", target.display())).clicked() {
                    open_path(&target);
                }
                if self.rec.is_none() {
                    let cur = if self.settings.mic.is_empty() { "Microphone: system default".to_string() } else { format!("Microphone: {}", self.settings.mic) };
                    egui::ComboBox::from_id_salt("mic").selected_text(cur).show_ui(ui, |ui| {
                        if ui.selectable_label(self.settings.mic.is_empty(), crate::i18n::tr("System default")).clicked() {
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
            ui.label(RichText::new(crate::i18n::tr("Nothing here yet. Use + Add for photos, documents, stories and recordings.")).italics().weak());
            return;
        }
        // the same cards as the Memories tab, each with a remove button
        let _ = pdf_req;
        let items = self.person_memory_items(id, kind);
        self.memory_cards(ui, act, &items, false);
        if editing {
            ui.label(RichText::new(crate::i18n::tr("Photos, documents and recordings are kept on this computer only, in the media folder.")).small().weak());
        }
    }
}
