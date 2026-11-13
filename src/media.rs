//! Photos, sound, documents and the Gallery.

use super::*;

/// The part of a picture to show in a frame of aspect `target` (width / height) without
/// stretching: the box `r` grown just enough, around its centre, to match.
pub(crate) fn crop_uv(r: [f32; 4], img_aspect: f32, target: f32) -> Rect {
    let [x, y, mut w, mut h] = r;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let ia = if img_aspect > 0.0 { img_aspect } else { 1.0 };
    let cur = (w * ia) / h.max(1e-6);
    if cur > target {
        h = (w * ia / target).min(1.0);
    } else {
        w = (h * target / ia).min(1.0);
    }
    let x0 = (cx - w / 2.0).clamp(0.0, 1.0 - w);
    let y0 = (cy - h / 2.0).clamp(0.0, 1.0 - h);
    Rect::from_min_size(pos2(x0, y0), vec2(w, h))
}

/// Draw a picture (or the face part of it) filling `rect`, cropped rather than squashed.
pub(crate) fn put_photo(ui: &mut egui::Ui, rect: Rect, uri: &str, r: [f32; 4], rounding: f32) {
    let img = egui::Image::new(uri.to_string());
    let aspect = match img.load_for_size(ui.ctx(), rect.size()) {
        Ok(egui::load::TexturePoll::Ready { texture }) => texture.size.x / texture.size.y.max(1.0),
        _ => 1.0,
    };
    let uv = crop_uv(r, aspect, rect.width() / rect.height().max(1.0));
    ui.put(rect, egui::Image::new(uri.to_string()).uv(uv).fit_to_exact_size(rect.size()).rounding(rounding));
}

/// Which Gallery group a media item belongs to: 1 photo, 2 document or PDF, 3 recording, 4 story.
pub(crate) fn media_kind(m: &Media) -> u8 {
    let form = m.form.to_lowercase();
    if form == "story" || m.extra.iter().any(|r| r.text.contains("TYPE story")) {
        4
    } else if is_image(&m.file) || online_photo(m) {
        1
    } else if is_audio(&m.file) {
        3
    } else {
        2
    }
}

/// A story being written or read in the story window.
pub(crate) struct StoryEdit {
    pub(crate) who: u32,
    pub(crate) idx: Option<usize>,
    pub(crate) title: String,
    pub(crate) text: String,
}

/// A picture that lives on a website (Ancestry and others put links, not files, in their GEDCOM).
pub(crate) fn online_photo(m: &Media) -> bool {
    m.file.is_empty() && is_url(&m.path) && matches!(m.form.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tif" | "tiff" | "webp")
}

pub(crate) fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Take several photos, documents or recordings off the people they belong to, in one go.
/// The files are left in the media folder (and backups); a profile photo that was removed is
/// cleared so no box points at it. Returns how many were removed.
pub(crate) fn remove_media_items(db: &mut Db, sel: &HashSet<(u32, usize)>) -> usize {
    let mut by: HashMap<u32, Vec<usize>> = HashMap::new();
    for (who, i) in sel {
        by.entry(*who).or_default().push(*i);
    }
    let mut n = 0;
    for (who, mut idx) in by {
        let Some(p) = db.people.get_mut(&who) else { continue };
        idx.sort_unstable_by(|a, b| b.cmp(a));
        idx.dedup();
        for i in idx {
            if i == NOTE_ITEM {
                if !p.notes.trim().is_empty() {
                    p.notes.clear();
                    n += 1;
                }
            } else if i < p.media.len() {
                let m = p.media.remove(i);
                if !m.file.is_empty() && p.portrait == m.file {
                    p.portrait.clear();
                    p.face_portrait = None;
                    p.portrait_crop = None;
                }
                n += 1;
            }
        }
    }
    n
}

/// One card in Memories or the Gallery: (owner, media index or NOTE_ITEM, kind, title, owner name).
pub(crate) type MemItem = (u32, usize, u8, String, String);
/// What the list of cards depends on, besides the tree: (tree version, photos, kind, only the
/// selected person, selected person, filter).
pub(crate) type MemKey = (u64, bool, u8, bool, Option<u32>, String);

/// The key used for a person's note in the Memories selection (in place of a media index).
pub(crate) const NOTE_ITEM: usize = usize::MAX;

/// Every media file the tree still uses (photos, documents, recordings and profile pictures).
pub(crate) fn media_in_use(db: &Db) -> HashSet<String> {
    let mut used = HashSet::new();
    for p in db.people.values() {
        for m in &p.media {
            if !m.file.is_empty() {
                used.insert(m.file.clone());
            }
        }
        if !p.portrait.is_empty() {
            used.insert(p.portrait.clone());
        }
        if let Some(fp) = &p.face_portrait {
            if !fp.file.is_empty() {
                used.insert(fp.file.clone());
            }
        }
    }
    for f in db.fams.values() {
        used.extend(f.media.iter().filter(|m| !m.file.is_empty()).map(|m| m.file.clone()));
    }
    db.each_cite(&mut |_, _, c| used.extend(c.media.iter().filter(|m| !m.file.is_empty()).map(|m| m.file.clone())));
    used
}

/// The files that removing `sel` would leave unused (so they could be taken out of the media folder).
pub(crate) fn files_freed_by(db: &Db, sel: &HashSet<(u32, usize)>) -> Vec<String> {
    let mut after = db.clone();
    remove_media_items(&mut after, sel);
    let still = media_in_use(&after);
    let mut out: Vec<String> = sel
        .iter()
        .filter_map(|(who, i)| db.people.get(who).and_then(|p| p.media.get(*i)))
        .map(|m| m.file.clone())
        .filter(|f| !f.is_empty() && !still.contains(f) && safe_media_path(f).map(|p| p.is_file()).unwrap_or(false))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Move media files out of the media folder into "Removed files" in the data folder, rather than
/// deleting them, so nothing is lost by a slip. Returns how many were moved.
pub(crate) fn move_to_removed(files: &[String]) -> usize {
    let dir = data_dir().join("Removed files");
    if std::fs::create_dir_all(&dir).is_err() {
        return 0;
    }
    let mut n = 0;
    for f in files {
        let Some(src) = safe_media_path(f) else { continue };
        if !src.is_file() {
            continue;
        }
        let mut dst = dir.join(f);
        let mut k = 2;
        while dst.exists() {
            dst = dir.join(format!("{} ({})", f, k));
            k += 1;
        }
        if std::fs::rename(&src, &dst).is_ok() || (std::fs::copy(&src, &dst).is_ok() && std::fs::remove_file(&src).is_ok()) {
            n += 1;
        }
    }
    n
}

/// Take every link to a photo stored on a website (Ancestry and MyHeritage put these in their
/// GEDCOM files instead of the pictures) off everyone. Returns how many were removed.
pub(crate) fn remove_online_links(db: &mut Db) -> usize {
    let mut n = 0;
    for p in db.people.values_mut() {
        let before = p.media.len();
        p.media.retain(|m| !online_photo(m));
        n += before - p.media.len();
    }
    n
}

pub(crate) fn make_previews(src: &std::path::Path) -> Option<(String, String)> {
    use image::ImageDecoder;
    let reader = image::ImageReader::open(src).ok()?.with_guessed_format().ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let orient = decoder.orientation().ok();
    let mut img = image::DynamicImage::from_decoder(decoder).ok()?;
    if let Some(o) = orient {
        img.apply_orientation(o);
    }
    let dir = media_dir().join("previews");
    let _ = std::fs::create_dir_all(&dir);
    let id = stamp();
    let thumb = format!("{}-t.jpg", id);
    let mid = format!("{}-m.jpg", id);
    img.thumbnail(320, 320).to_rgb8().save(dir.join(&thumb)).ok()?;
    img.thumbnail(1600, 1600).to_rgb8().save(dir.join(&mid)).ok()?;
    Some((thumb, mid))
}

pub(crate) fn input_device_names() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|d| d.filter_map(|x| x.name().ok()).collect())
        .unwrap_or_default()
}

pub(crate) fn try_device(dev: &cpal::Device, buf: &Arc<Mutex<Vec<i16>>>, peak: &Arc<AtomicU32>) -> Result<(cpal::Stream, u16, u32), String> {
    let cfg = match dev.default_input_config() {
        Ok(c) => c,
        Err(e1) => match dev.supported_input_configs() {
            Ok(mut it) => match it.next() {
                Some(r) => r.with_max_sample_rate(),
                None => return Err(e1.to_string()),
            },
            Err(e2) => return Err(format!("{} / {}", e1, e2)),
        },
    };
    let (sf, channels, rate) = (cfg.sample_format(), cfg.channels(), cfg.sample_rate().0);
    let config: cpal::StreamConfig = cfg.into();
    let b = buf.clone();
    let pk = peak.clone();
    let err_fn = |e: cpal::StreamError| eprintln!("audio error: {}", e);
    let stream = match sf {
        cpal::SampleFormat::F32 => dev.build_input_stream(
            &config,
            move |d: &[f32], _: &cpal::InputCallbackInfo| {
                let m = d.iter().fold(0f32, |a, x| a.max(x.abs()));
                pk.fetch_max((m * 32767.0) as u32, Ordering::Relaxed);
                if let Ok(mut v) = b.lock() {
                    v.extend(d.iter().map(|x| (x.clamp(-1.0, 1.0) * 32767.0) as i16));
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => dev.build_input_stream(
            &config,
            move |d: &[i16], _: &cpal::InputCallbackInfo| {
                let m = d.iter().map(|x| x.unsigned_abs() as u32).max().unwrap_or(0);
                pk.fetch_max(m, Ordering::Relaxed);
                if let Ok(mut v) = b.lock() {
                    v.extend_from_slice(d);
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => dev.build_input_stream(
            &config,
            move |d: &[u16], _: &cpal::InputCallbackInfo| {
                let m = d.iter().map(|x| (*x as i32 - 32768).unsigned_abs()).max().unwrap_or(0);
                pk.fetch_max(m, Ordering::Relaxed);
                if let Ok(mut v) = b.lock() {
                    v.extend(d.iter().map(|x| (*x as i32 - 32768) as i16));
                }
            },
            err_fn,
            None,
        ),
        _ => return Err("unsupported audio format".to_string()),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, channels, rate))
}

impl App {
    pub(crate) fn attach_files(&mut self, id: u32) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Add photos or documents")
            .add_filter("Photos, PDFs, documents and recordings", &["jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "tif", "tiff", "pdf", "doc", "docx", "rtf", "txt", "odt", "htm", "html", "wav", "mp3", "m4a", "ogg", "flac", "aac", "mp4", "mov"])
            .add_filter("All files", &["*"])
            .pick_files()
        {
            let mut n = 0;
            for p in paths {
                let name = p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                if let Some((file, shown)) = import_one(&p, &name) {
                    if let Some(person) = self.db.people.get_mut(&id) {
                        person.media.push(Media { file, name: shown, ..Default::default() });
                        n += 1;
                    }
                }
            }
            self.status = format!("Added {} file(s).", n);
            self.dirty = true;
        }
    }

    pub(crate) fn ensure_previews(&mut self, id: u32) {
        let todo: Vec<(usize, String)> = match self.db.people.get(&id) {
            Some(p) => p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file) && m.thumb.is_empty()).map(|(i, m)| (i, m.file.clone())).collect(),
            None => return,
        };
        if todo.is_empty() {
            return;
        }
        for (i, f) in todo {
            let r = make_previews(&mfile(&f));
            if let Some(m) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(i)) {
                match r {
                    Some((t, mid)) => {
                        m.thumb = t;
                        m.mid = mid;
                    }
                    None => {
                        m.thumb = "-".to_string();
                    }
                }
            }
        }
        self.dirty = true;
    }

    /// Make previews for up to `max` photos (anywhere in the tree) that do not have one yet.
    /// Returns true if any were made (the screen should be drawn again).
    pub(crate) fn missing_previews(&mut self, max: usize) -> bool {
        let todo: Vec<(u32, usize, String)> = self
            .db
            .people
            .iter()
            .flat_map(|(id, p)| p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file) && m.thumb.is_empty()).map(move |(i, m)| (*id, i, m.file.clone())))
            .take(max)
            .collect();
        if todo.is_empty() {
            return false;
        }
        for (id, i, f) in todo {
            let r = make_previews(&mfile(&f));
            if let Some(m) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(i)) {
                match r {
                    Some((t, mid)) => {
                        m.thumb = t;
                        m.mid = mid;
                    }
                    None => m.thumb = "-".to_string(),
                }
            }
        }
        self.dirty = true;
        // ask for another frame either way, so the previews just made are drawn straight away
        true
    }

    pub(crate) fn viewer(&mut self, ctx: &egui::Context) {
        let Some((pid, idx)) = self.viewing else { return };
        let valid = self.db.people.get(&pid).map(|p| idx < p.media.len()).unwrap_or(false);
        if !valid {
            self.viewing = None;
            return;
        }
        let mut open = true;
        let mut nav: Option<usize> = None;
        let mut remove = false;
        let mut portrait: Option<String> = None;
        let (mut tag_cancel, mut tag_pick, mut face_remove, mut face_portrait, mut face_go): (bool, Option<u32>, Option<usize>, Option<usize>, Option<usize>) = (false, None, None, None, None);
        let mut crop_pick: Option<[f32; 4]> = None;
        let tag_hits: Vec<(u32, String)> = if self.x.tag_box.is_some() {
            let q = self.x.tag_q.trim().to_lowercase();
            let words: Vec<&str> = q.split_whitespace().collect();
            let mut v: Vec<(u32, String)> = self
                .db
                .people
                .iter()
                .filter(|(_, p)| !words.is_empty() && words.iter().all(|w| p.name.to_lowercase().contains(w)))
                .map(|(i, _)| (*i, self.db.label(*i)))
                .take(8)
                .collect();
            if words.is_empty() {
                // the photo's owner and their close family are the likely ones
                let mut near = vec![pid];
                let (f, mo) = self.db.parents(pid);
                near.extend(f);
                near.extend(mo);
                near.extend(self.db.partners(pid));
                near.extend(self.db.children_of(pid));
                near.extend(self.db.siblings(pid));
                v = near.into_iter().filter(|i| self.db.people.contains_key(i)).take(12).map(|i| (i, self.db.label(i))).collect();
            }
            v
        } else {
            Vec::new()
        };
        let label_cache: HashMap<u32, String> = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).map(|m| m.faces.iter().map(|f| (f.who, self.db.label(f.who))).collect()).unwrap_or_default();
        let lc = label_cache;
        egui::Window::new("Photo").open(&mut open).default_size([720.0, 640.0]).show(ctx, |ui| {
            let p = self.db.people.get_mut(&pid).unwrap();
            let imgs: Vec<usize> = p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file)).map(|(i, _)| i).collect();
            let pos = imgs.iter().position(|&i| i == idx).unwrap_or(0);
            ui.horizontal(|ui| {
                if ui.add_enabled(pos > 0, egui::Button::new("< Previous")).clicked() {
                    nav = Some(imgs[pos - 1]);
                }
                ui.label(format!("{} of {}", pos + 1, imgs.len()));
                if ui.add_enabled(pos + 1 < imgs.len(), egui::Button::new("Next >")).clicked() {
                    nav = Some(imgs[pos + 1]);
                }
            });
            let full = mfile(&p.media[idx].file);
            let m = &mut p.media[idx];
            let uri = if m.mid.is_empty() || m.mid == "-" {
                img_uri(&full)
            } else {
                img_uri(&mprev(&m.mid))
            };
            let file_name = m.file.clone();
            let sense = if self.x.tag_mode || self.x.crop_mode { Sense::click_and_drag() } else { Sense::hover() };
            let resp = ui.add(egui::Image::new(uri).max_size(vec2(ui.available_width(), 480.0)).sense(sense));
            let ir = resp.rect;
            let to_img = |p: egui::Pos2| pos2(((p.x - ir.min.x) / ir.width()).clamp(0.0, 1.0), ((p.y - ir.min.y) / ir.height()).clamp(0.0, 1.0));
            let to_scr = |r: [f32; 4]| Rect::from_min_size(pos2(ir.min.x + r[0] * ir.width(), ir.min.y + r[1] * ir.height()), vec2(r[2] * ir.width(), r[3] * ir.height()));
            // drawing a new box
            if self.x.tag_mode || self.x.crop_mode {
                if resp.drag_started() {
                    if let Some(p) = resp.interact_pointer_pos() {
                        self.x.tag_drag = Some((p, p));
                        self.x.tag_box = None;
                    }
                }
                if resp.dragged() {
                    if let (Some(p), Some((a, _))) = (resp.interact_pointer_pos(), self.x.tag_drag) {
                        self.x.tag_drag = Some((a, p));
                    }
                }
                if resp.drag_stopped() {
                    if let Some((a, b)) = self.x.tag_drag.take() {
                        let (a, b) = (to_img(a), to_img(b));
                        let r = [a.x.min(b.x), a.y.min(b.y), (a.x - b.x).abs(), (a.y - b.y).abs()];
                        if r[2] > 0.015 && r[3] > 0.015 {
                            self.x.tag_box = Some(r);
                            self.x.tag_q.clear();
                        }
                    }
                }
            }
            let accent = ui.visuals().selection.bg_fill;
            let names: Vec<(usize, Rect, String)> = m.faces.iter().enumerate().map(|(k, f)| (k, to_scr(f.r), lc.get(&f.who).cloned().unwrap_or_else(|| "(someone removed from the tree)".into()))).collect();
            let painter = ui.painter_at(ir);
            for (_, r, name) in &names {
                painter.rect_stroke(*r, 2.0_f32, Stroke::new(2.0_f32, Color32::WHITE));
                painter.rect_stroke(r.expand(1.0), 2.0_f32, Stroke::new(1.0_f32, Color32::BLACK));
                let g = painter.layout_no_wrap(name.clone(), FontId::proportional(13.0), Color32::WHITE);
                let lr = Rect::from_min_size(pos2(r.min.x, r.max.y + 2.0), g.size() + vec2(8.0, 4.0));
                painter.rect_filled(lr, 3.0_f32, Color32::from_black_alpha(170));
                painter.galley(lr.min + vec2(4.0, 2.0), g, Color32::WHITE);
            }
            if let Some((a, b)) = self.x.tag_drag {
                painter.rect_stroke(Rect::from_two_pos(a, b), 0.0_f32, Stroke::new(2.0_f32, accent));
            }
            if let Some(r) = self.x.tag_box {
                painter.rect_stroke(to_scr(r), 2.0_f32, Stroke::new(2.5_f32, accent));
            }
            ui.add(egui::TextEdit::singleline(&mut m.caption).hint_text("Caption").desired_width(f32::INFINITY));
            ui.horizontal(|ui| {
                if ui.button("Open full size").clicked() {
                    open_media(&m.file);
                }
                if ui.button("Use as profile photo").clicked() {
                    portrait = Some(file_name.clone());
                }
                if ui.selectable_label(self.x.crop_mode, "Crop for profile photo").on_hover_text("Drag a box around the part of the picture to show as the profile photo").clicked() {
                    self.x.crop_mode = !self.x.crop_mode;
                    self.x.tag_mode = false;
                    self.x.tag_box = None;
                    self.x.tag_drag = None;
                }
                let lbl = if self.x.tag_mode { "Stop tagging faces" } else { "Tag faces" };
                if ui.selectable_label(self.x.tag_mode, lbl).on_hover_text("Drag a box around someone's face, then choose who it is").clicked() {
                    self.x.tag_mode = !self.x.tag_mode;
                    self.x.crop_mode = false;
                    self.x.tag_box = None;
                    self.x.tag_drag = None;
                }
                if ui.button("Remove photo").clicked() {
                    remove = true;
                }
            });
            if self.x.tag_mode && self.x.tag_box.is_none() {
                ui.label(RichText::new("Drag a box around a face on the picture.").italics().weak());
            }
            if self.x.crop_mode {
                if self.x.tag_box.is_none() {
                    ui.label(RichText::new("Drag a box around the part to show as the profile photo.").italics().weak());
                } else {
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Use this part as the profile photo").strong()).clicked() {
                            crop_pick = self.x.tag_box;
                        }
                        if ui.button("Cancel").clicked() {
                            tag_cancel = true;
                        }
                    });
                }
            }
            if self.x.tag_mode && self.x.tag_box.is_some() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Who is this?").strong());
                        ui.add(egui::TextEdit::singleline(&mut self.x.tag_q).hint_text("Type a name").desired_width(220.0)).request_focus();
                        if ui.button("Cancel").clicked() {
                            tag_cancel = true;
                        }
                    });
                    for (who, label) in &tag_hits {
                        if ui.button(label.as_str()).clicked() {
                            tag_pick = Some(*who);
                        }
                    }
                });
            }
            if !names.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new("Tagged in this photo").strong());
                for (k, _, name) in &names {
                    ui.horizontal(|ui| {
                        ui.label(name.as_str());
                        if ui.small_button("Go to").clicked() {
                            face_go = Some(*k);
                        }
                        if ui.small_button("Use as their profile picture").clicked() {
                            face_portrait = Some(*k);
                        }
                        if ui.small_button("Remove tag").clicked() {
                            face_remove = Some(*k);
                        }
                    });
                }
            }
        });
        if tag_cancel {
            self.x.tag_box = None;
        }
        if let Some(r) = crop_pick {
            let file = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).map(|m| m.file.clone());
            if let (Some(f), Some(p)) = (file, self.db.people.get_mut(&pid)) {
                p.portrait = f;
                p.portrait_crop = Some(r);
                p.face_portrait = None;
            }
            self.x.tag_box = None;
            self.x.crop_mode = false;
            self.dirty = true;
            self.status = "Profile photo cropped.".to_string();
        }
        if let (Some(who), Some(r)) = (tag_pick, self.x.tag_box) {
            if let Some(m) = self.db.people.get_mut(&pid).and_then(|p| p.media.get_mut(idx)) {
                m.faces.push(Face { who, r });
            }
            self.x.tag_box = None;
            self.dirty = true;
            self.status = format!("Tagged {}. The photo now shows on their page too.", self.db.label(who));
        }
        if let Some(k) = face_remove {
            if let Some(m) = self.db.people.get_mut(&pid).and_then(|p| p.media.get_mut(idx)) {
                if k < m.faces.len() {
                    m.faces.remove(k);
                }
            }
            self.dirty = true;
        }
        if let Some(k) = face_portrait {
            let info = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).and_then(|m| m.faces.get(k).map(|f| (f.who, m.file.clone(), f.r)));
            if let Some((who, file, r)) = info {
                if let Some(q) = self.db.people.get_mut(&who) {
                    q.face_portrait = Some(FacePortrait { owner: pid, file, r });
                    q.portrait_crop = None;
                }
                self.dirty = true;
                self.status = format!("Profile picture set for {}.", self.db.label(who));
            }
        }
        if let Some(k) = face_go {
            if let Some(w) = self.db.people.get(&pid).and_then(|p| p.media.get(idx)).and_then(|m| m.faces.get(k)).map(|f| f.who) {
                self.viewing = None;
                self.goto(w);
                self.view = View::Person;
                return;
            }
        }
        if !open {
            self.viewing = None;
        }
        if let Some(n) = nav {
            self.viewing = Some((pid, n));
        }
        if let Some(f) = portrait {
            if let Some(p) = self.db.people.get_mut(&pid) {
                p.portrait = f;
                p.face_portrait = None;
                p.portrait_crop = None;
            }
            self.dirty = true;
            self.status = "Profile photo set. It now shows in the pedigree boxes.".to_string();
        }
        if remove {
            if let Some(p) = self.db.people.get_mut(&pid) {
                if idx < p.media.len() {
                    p.media.remove(idx);
                }
            }
            self.viewing = None;
            self.dirty = true;
        }
    }

    pub(crate) fn start_recording(&mut self, person: u32) {
        let host = cpal::default_host();
        let mut cands: Vec<cpal::Device> = Vec::new();
        if !self.settings.mic.is_empty() {
            if let Ok(ds) = host.input_devices() {
                for d in ds {
                    if d.name().map(|n| n == self.settings.mic).unwrap_or(false) {
                        cands.push(d);
                    }
                }
            }
        }
        if let Some(d) = host.default_input_device() {
            cands.push(d);
        }
        if let Ok(ds) = host.input_devices() {
            for d in ds {
                cands.push(d);
            }
        }
        if cands.is_empty() {
            self.status = "No microphone found. Check System Settings > Sound > Input, and that Gilded Roots is allowed under Privacy & Security > Microphone.".to_string();
            return;
        }
        let buf = Arc::new(Mutex::new(Vec::<i16>::new()));
        let peak = Arc::new(AtomicU32::new(0));
        let mut last = String::new();
        for dev in cands {
            let name = dev.name().unwrap_or_else(|_| "a microphone".to_string());
            match try_device(&dev, &buf, &peak) {
                Ok((stream, channels, rate)) => {
                    self.rec = Some(Recorder { _stream: stream, buf, channels, rate, started: std::time::Instant::now(), person, peak });
                    self.status = format!("Recording from {}. Watch the level bar: if it stays empty when you talk, pick a different microphone.", name);
                    return;
                }
                Err(e) => last = format!("{}: {}", name, e),
            }
        }
        self.status = format!(
            "Could not start any microphone (last error - {}). Check System Settings > Sound > Input for a working microphone, close other apps that use it, and check Privacy & Security > Microphone.",
            last
        );
    }

    pub(crate) fn stop_recording(&mut self) {
        let Some(r) = self.rec.take() else { return };
        let Recorder { _stream, buf, channels, rate, person, .. } = r;
        drop(_stream);
        let samples = buf.lock().map(|v| v.clone()).unwrap_or_default();
        if samples.is_empty() {
            self.status = "Nothing was recorded. Check your microphone settings.".to_string();
            return;
        }
        let top = samples.iter().map(|x| x.unsigned_abs()).max().unwrap_or(0);
        if top < 8 {
            self.status = "The recording was completely silent, so it was not saved. On a Mac, open System Settings > Privacy & Security > Microphone and switch on Gilded Roots, then try again.".to_string();
            return;
        }
        let file = format!("{}-story.wav", stamp());
        let spec = hound::WavSpec { channels, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let path = media_dir().join(&file);
        let ok = (|| -> Result<(), hound::Error> {
            let mut w = hound::WavWriter::create(&path, spec)?;
            for x in &samples {
                w.write_sample(*x)?;
            }
            w.finalize()
        })();
        match ok {
            Ok(_) => {
                if let Some(p) = self.db.people.get_mut(&person) {
                    p.media.push(Media { file, name: "Recorded story.wav".to_string(), caption: "Recorded story".to_string(), ..Default::default() });
                }
                self.status = "Story saved. You'll find it under Photos, documents and stories.".to_string();
                self.dirty = true;
            }
            Err(e) => self.status = format!("Could not save the recording: {}", e),
        }
    }
}

impl App {
    /// Where a person's profile picture comes from: the picture's address and the part of it
    /// to show (the whole picture, or a tagged face).
    pub(crate) fn portrait_src(&self, p: &Person) -> Option<(String, [f32; 4])> {
        if let Some(fp) = &p.face_portrait {
            if let Some(m) = self.db.people.get(&fp.owner).and_then(|o| o.media.iter().find(|m| m.file == fp.file)) {
                // a face is a small part of the picture, so use the larger preview
                for f in [&m.mid, &m.thumb] {
                    if !f.is_empty() && f != "-" {
                        let t = mprev(f);
                        if t.is_file() {
                            return Some((img_uri(&t), fp.r));
                        }
                    }
                }
                let full = mfile(&m.file);
                if full.is_file() {
                    return Some((img_uri(&full), fp.r));
                }
            }
        }
        if p.portrait.is_empty() {
            return None;
        }
        let m = p.media.iter().find(|m| m.file == p.portrait)?;
        let crop = p.portrait_crop.unwrap_or([0.0, 0.0, 1.0, 1.0]);
        // a cropped photo needs the larger preview to stay sharp
        let order = if p.portrait_crop.is_some() { [&m.mid, &m.thumb] } else { [&m.thumb, &m.mid] };
        for f in order {
            if !f.is_empty() && f != "-" {
                let t = mprev(f);
                if t.is_file() {
                    return Some((img_uri(&t), crop));
                }
            }
        }
        // no small copy yet (or it was lost): use the photo itself
        let full = mfile(&m.file);
        if full.is_file() && is_image(&m.file) {
            Some((img_uri(&full), crop))
        } else {
            None
        }
    }

    /// Photos in other people's albums where this person is tagged, shown cropped to their face.
    pub(crate) fn tagged_strip(&mut self, ui: &mut egui::Ui, id: u32) {
        let tags = self.tagged_in(id);
        if tags.is_empty() {
            return;
        }
        ui.add_space(6.0);
        ui.label(RichText::new(format!("Tagged in {} other photo(s)", tags.len())).strong());
        let mut open: Option<(u32, usize)> = None;
        ui.horizontal_wrapped(|ui| {
            for (owner, i, r) in &tags {
                let Some(m) = self.db.people.get(owner).and_then(|o| o.media.get(*i)) else { continue };
                let src = [&m.mid, &m.thumb].into_iter().find(|f| !f.is_empty() && *f != "-").map(|f| mprev(f)).unwrap_or_else(|| mfile(&m.file));
                let uri = img_uri(&src);
                let who = self.db.label(*owner);
                ui.vertical(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(vec2(90.0, 90.0), Sense::click());
                    put_photo(ui, rect, &uri, *r, 6.0);
                    if resp.on_hover_text(format!("From {}'s photos", who)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        open = Some((*owner, *i));
                    }
                });
            }
        });
        if let Some(v) = open {
            self.viewing = Some(v);
        }
    }

    /// Photos in other people's albums where this person is tagged: (owner, index, face box).
    pub(crate) fn tagged_in(&self, id: u32) -> Vec<(u32, usize, [f32; 4])> {
        let mut v = Vec::new();
        for (oid, o) in &self.db.people {
            for (i, m) in o.media.iter().enumerate() {
                for f in &m.faces {
                    if f.who == id && *oid != id {
                        v.push((*oid, i, f.r));
                    }
                }
            }
        }
        v
    }
}

impl App {
    /// The Gallery: every photo, document, PDF, recording, story and note in the tree, with
    /// filters by kind and an Add menu.
    pub(crate) fn gallery_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        self.media_hub(ui, act, true);
    }

    pub(crate) fn memories_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        if self.x.gal_kind == 1 {
            self.x.gal_kind = 0;
        }
        self.media_hub(ui, act, false);
    }

    pub(crate) fn media_hub(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, photos: bool) {
        let sel_name = self.sel.map(|s| self.db.label(s)).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.heading(if photos { "Gallery" } else { "Memories" });
            ui.add_space(8.0);
            ui.add_enabled_ui(self.sel.is_some(), |ui| {
                ui.menu_button(RichText::new("+ Add").strong(), |ui| {
                    ui.label(RichText::new(format!("For {}", sel_name)).small().weak());
                    if ui.button(if photos { "Photos..." } else { "PDFs, documents or recordings..." }).clicked() {
                        *act = Some(Action::AttachFiles);
                        ui.close_menu();
                    }
                    if ui.button("Write a story...").clicked() {
                        if let Some(s) = self.sel {
                            self.x.story = Some(StoryEdit { who: s, idx: None, title: String::new(), text: String::new() });
                        }
                        ui.close_menu();
                    }
                    if ui.button("Record a spoken story...").on_hover_text("Recording is done from the person's Photos and recordings panel").clicked() {
                        self.view = View::Person;
                        if let Some(d) = self.settings.dock.iter_mut().find(|d| d.sec == "media") {
                            d.on = true;
                        }
                        ui.close_menu();
                    }
                    if ui.button("Write a note...").clicked() {
                        self.view = View::Person;
                        if let Some(d) = self.settings.dock.iter_mut().find(|d| d.sec == "notes") {
                            d.on = true;
                        }
                        *act = Some(Action::EditStart);
                        ui.close_menu();
                    }
                });
            });
            ui.separator();
            if !photos {
                for (k, l) in [(0u8, "All memories"), (4, "Stories"), (3, "Recordings"), (2, "Documents and PDFs"), (5, "Notes")] {
                    if ui.selectable_label(self.x.gal_kind == k, l).clicked() {
                        self.x.gal_kind = k;
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.gal_only, "Only the selected person");
            ui.add(egui::TextEdit::singleline(&mut self.gal_find).hint_text("Filter by name or caption").desired_width(220.0));
            ui.separator();
            if !self.x.gal_pick {
                if ui.button("Select...").on_hover_text("Click several items to tick them, then remove them in one go").clicked() {
                    self.x.gal_pick = true;
                    self.x.gal_sel.clear();
                }
            }
            let online = self.db.people.values().map(|p| p.media.iter().filter(|m| online_photo(m)).count()).sum::<usize>();
            if online > 0 && ui.button(format!("Remove online links ({})...", online)).on_hover_text("Take off every link to a photo stored on a website (Ancestry, MyHeritage...). Nothing on your computer is deleted.").clicked() {
                self.x.gal_online_confirm = true;
            }
        });
        let q = self.gal_find.trim().to_lowercase();
        let want = if photos { 1 } else { self.x.gal_kind };
        let key: MemKey = (self.x.data_ver, photos, want, self.gal_only, self.sel, q.clone());
        let items = match &self.x.mem_cache {
            Some((k, items)) if *k == key => items.clone(),
            _ => {
                let items = std::sync::Arc::new(self.memory_items(photos, want, &q));
                self.x.mem_cache = Some((key, items.clone()));
                items
            }
        };
        self.memory_cards(ui, act, photos, &items);
    }

    /// Every card to show, in order of the owner's name.
    fn memory_items(&self, photos: bool, want: u8, q: &str) -> Vec<MemItem> {
        let mut items: Vec<MemItem> = Vec::new();
        for (id, p) in &self.db.people {
            for (i, m) in p.media.iter().enumerate() {
                if self.gal_only && self.sel != Some(*id) && !m.faces.iter().any(|f| Some(f.who) == self.sel) {
                    continue;
                }
                let kind = media_kind(m);
                if (want != 0 && want != kind) || (!photos && kind == 1) {
                    continue;
                }
                let title = if !m.caption.is_empty() { m.caption.clone() } else if !m.name.is_empty() { m.name.clone() } else { m.path.rsplit('/').next().unwrap_or("").to_string() };
                if !q.is_empty() && !p.name.to_lowercase().contains(q) && !title.to_lowercase().contains(q) {
                    continue;
                }
                items.push((*id, i, kind, title, p.name.clone()));
            }
            if !photos && (want == 0 || want == 5) && !p.notes.trim().is_empty() && (!self.gal_only || self.sel == Some(*id)) {
                let first: String = p.notes.trim().chars().take(140).collect();
                if q.is_empty() || p.name.to_lowercase().contains(q) || p.notes.to_lowercase().contains(q) {
                    items.push((*id, NOTE_ITEM, 5, first, p.name.clone()));
                }
            }
        }
        items.sort_by(|a, b| a.4.cmp(&b.4).then(a.2.cmp(&b.2)).then(a.1.cmp(&b.1)));
        items
    }

    fn memory_cards(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, photos: bool, items: &[MemItem]) {
        let counts = |k: u8| items.iter().filter(|x| x.2 == k).count();
        if photos {
            ui.label(RichText::new(format!("{} photo(s)", counts(1))).weak());
        } else {
            ui.label(RichText::new(format!("{} stor(ies), {} recording(s), {} document(s), {} note(s)", counts(4), counts(3), counts(2), counts(5))).weak());
        }
        ui.separator();
        if items.is_empty() {
            ui.label("Nothing here yet. Select a person and use + Add.");
            return;
        }
        if self.x.gal_pick {
            // only keep ticks for items still listed
            let shown: HashSet<(u32, usize)> = items.iter().map(|x| (x.0, x.1)).collect();
            self.x.gal_sel.retain(|k| shown.contains(k));
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} selected", self.x.gal_sel.len())).strong());
                if ui.button("Select all").clicked() {
                    self.x.gal_sel = shown.clone();
                }
                if ui.button("Select none").clicked() {
                    self.x.gal_sel.clear();
                }
                if ui.add_enabled(!self.x.gal_sel.is_empty(), egui::Button::new("Remove selected...")).clicked() {
                    self.x.gal_confirm = true;
                    self.x.gal_freed = files_freed_by(&self.db, &self.x.gal_sel);
                }
                if ui.button("Done").clicked() {
                    self.x.gal_pick = false;
                    self.x.gal_sel.clear();
                }
            });
            ui.add_space(4.0);
        }
        let mut pdf_req: Option<(PathBuf, String)> = None;
        let mut story_open: Option<(u32, usize)> = None;
        // A grid that fits the window: as many columns as fit, with the cards stretched to
        // fill each row, so nothing runs off the right edge.
        let gap = 12.0;
        let avail = (ui.available_width() - 4.0 - ui.spacing().scroll.bar_width).max(130.0);
        let cols = (((avail + gap) / (170.0 + gap)).floor() as usize).max(1);
        let cw = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).clamp(120.0, 260.0);
        // every row is the same height (room for a two-line name), so only the rows in view
        // need drawing, however many memories there are
        let row_h = (cw * 0.76).round() + 64.0;
        let n_rows = items.len().div_ceil(cols);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, n_rows, |ui, range| {
            for row in items.chunks(cols).skip(range.start).take(range.len()) {
                let top = ui.cursor().top();
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (id, i, kind, title, name) in row {
                        ui.vertical(|ui| {
                            ui.set_width(cw);
                            ui.set_max_width(cw);
                            let size = vec2(cw, (cw * 0.76).round());
                            if *i == NOTE_ITEM {
                                // a note: show its first lines on a card
                                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                                ui.painter().rect(rect, 6.0_f32, ui.visuals().faint_bg_color, ui.visuals().widgets.noninteractive.bg_stroke);
                                let g = ui.painter().layout(title.clone(), FontId::proportional(12.0), ui.visuals().text_color(), rect.width() - 16.0);
                                ui.painter().with_clip_rect(rect.shrink(6.0)).galley(rect.min + vec2(8.0, 8.0), g, ui.visuals().text_color());
                                let key = (*id, NOTE_ITEM);
                                if self.x.gal_sel.contains(&key) {
                                    ui.painter().rect_stroke(rect.expand(2.0), 7.0_f32, Stroke::new(3.0_f32, ui.visuals().selection.bg_fill));
                                }
                                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    if self.x.gal_pick {
                                        if !self.x.gal_sel.remove(&key) {
                                            self.x.gal_sel.insert(key);
                                        }
                                    } else {
                                        *act = Some(Action::Open(*id));
                                    }
                                }
                                if self.x.gal_pick {
                                    let mut on = self.x.gal_sel.contains(&key);
                                    if ui.checkbox(&mut on, RichText::new("Note").small()).changed() {
                                        if on {
                                            self.x.gal_sel.insert(key);
                                        } else {
                                            self.x.gal_sel.remove(&key);
                                        }
                                    }
                                } else {
                                    ui.label(RichText::new("Note").small().weak());
                                }
                            } else {
                                // gone since the list was made (just removed): skip until it is redone
                                let Some(m) = self.db.people.get(id).and_then(|p| p.media.get(*i)) else { return };
                                if *kind == 1 && online_photo(m) {
                                    let picked = self.x.gal_sel.contains(&(*id, *i));
                                    if ui.add_sized(size, egui::Button::new("Photo stored online\nClick to open in browser").selected(picked)).clicked() {
                                        if self.x.gal_pick {
                                            if !self.x.gal_sel.remove(&(*id, *i)) {
                                                self.x.gal_sel.insert((*id, *i));
                                            }
                                        } else {
                                            ui.ctx().open_url(egui::OpenUrl::new_tab(m.path.clone()));
                                        }
                                    }
                                } else if *kind == 1 && !m.thumb.is_empty() && m.thumb != "-" {
                                    let uri = img_uri(&mprev(&m.thumb));
                                    let r = ui.add(egui::Image::new(uri).fit_to_exact_size(size).rounding(4.0).sense(Sense::click()));
                                    if self.x.gal_sel.contains(&(*id, *i)) {
                                        ui.painter().rect_stroke(r.rect.expand(2.0), 5.0_f32, Stroke::new(3.0_f32, ui.visuals().selection.bg_fill));
                                    }
                                    if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        if self.x.gal_pick {
                                            // choosing several: a click ticks or unticks the photo
                                            if !self.x.gal_sel.remove(&(*id, *i)) {
                                                self.x.gal_sel.insert((*id, *i));
                                            }
                                        } else {
                                            self.viewing = Some((*id, *i));
                                        }
                                    }
                                } else {
                                    let (badge, hint) = match *kind {
                                        1 => ("PHOTO", "Open"),
                                        3 => ("\u{266A}", "Play"),
                                        4 => ("STORY", "Read"),
                                        _ if is_pdf(&m.file) => ("PDF", "Read"),
                                        _ => ("DOC", "Open"),
                                    };
                                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                                    let hov = resp.hovered();
                                    ui.painter().rect(rect, 6.0_f32, if hov { ui.visuals().widgets.hovered.bg_fill } else { ui.visuals().faint_bg_color }, ui.visuals().widgets.noninteractive.bg_stroke);
                                    ui.painter().text(rect.center() - vec2(0.0, 12.0), Align2::CENTER_CENTER, badge, FontId::proportional(28.0), ui.visuals().selection.bg_fill);
                                    let online = m.file.is_empty() && is_url(&m.path);
                                    ui.painter().text(rect.center() + vec2(0.0, 26.0), Align2::CENTER_CENTER, if online { "Open online" } else { hint }, FontId::proportional(12.0), ui.visuals().weak_text_color());
                                    if self.x.gal_sel.contains(&(*id, *i)) {
                                        ui.painter().rect_stroke(rect.expand(2.0), 7.0_f32, Stroke::new(3.0_f32, ui.visuals().selection.bg_fill));
                                    }
                                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        if self.x.gal_pick {
                                            if !self.x.gal_sel.remove(&(*id, *i)) {
                                                self.x.gal_sel.insert((*id, *i));
                                            }
                                        } else if online {
                                            ui.ctx().open_url(egui::OpenUrl::new_tab(m.path.clone()));
                                        } else if *kind == 4 && m.file.ends_with(".txt") {
                                            story_open = Some((*id, *i));
                                        } else if is_pdf(&m.file) {
                                            pdf_req = Some((mfile(&m.file), title.clone()));
                                        } else if *kind == 1 && m.thumb.is_empty() && !m.file.is_empty() {
                                            self.viewing = Some((*id, *i));
                                        } else if !m.file.is_empty() {
                                            open_media(&m.file);
                                        }
                                    }
                                }
                                if self.x.gal_pick {
                                    let key = (*id, *i);
                                    let mut on = self.x.gal_sel.contains(&key);
                                    if ui.checkbox(&mut on, RichText::new(title.as_str()).small()).changed() {
                                        if on {
                                            self.x.gal_sel.insert(key);
                                        } else {
                                            self.x.gal_sel.remove(&key);
                                        }
                                    }
                                } else {
                                    ui.add(egui::Label::new(RichText::new(title.as_str()).small()).truncate());
                                }
                            }
                            if ui.link(RichText::new(name.as_str()).small()).clicked() {
                                *act = Some(Action::Open(*id));
                            }
                        });
                    }
                });
                let used = ui.cursor().top() - top;
                ui.add_space((row_h - used).max(0.0));
            }
        });
        if pdf_req.is_some() {
            self.x.pdf_req = pdf_req;
        }
        if self.x.gal_confirm {
            let n = self.x.gal_sel.len();
            let mut go = false;
            let mut cancel = false;
            let freed = self.x.gal_freed.clone();
            let notes = self.x.gal_sel.iter().filter(|k| k.1 == NOTE_ITEM).count();
            let del = &mut self.x.gal_del_files;
            egui::Window::new("Remove selected items").id(egui::Id::new("gal_remove")).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)).show(ui.ctx(), |ui| {
                ui.label(format!("Remove {} item(s) from the people they belong to?", n));
                if notes > 0 {
                    ui.label(RichText::new(format!("{} of them are notes: the whole of each person's Notes box is cleared.", notes)).small());
                }
                if freed.is_empty() {
                    ui.label(RichText::new("Undo (Ctrl+Z) brings them back. Files that are still used by someone else stay where they are.").small().weak());
                } else {
                    ui.add_space(4.0);
                    ui.checkbox(del, format!("Also take the {} file(s) out of the media folder", freed.len()));
                    ui.label(RichText::new("Only files that no one else in the tree uses. They are moved to the \"Removed files\" folder in your Gilded Roots folder, not deleted, so you can put one back or delete them yourself later.").small().weak());
                    ui.label(RichText::new(if *del { "Undo (Ctrl+Z) brings the items back; moved files go back from the Removed files folder." } else { "Left unticked, the files stay in your media folder and Undo (Ctrl+Z) brings everything back." }).small().weak());
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Remove").strong()).clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if cancel {
                self.x.gal_confirm = false;
            }
            if go {
                self.x.gal_confirm = false;
                let removed = remove_media_items(&mut self.db, &self.x.gal_sel);
                // only files nothing uses any more, checked again after the removal
                let still = media_in_use(&self.db);
                let moved = if self.x.gal_del_files { move_to_removed(&freed.into_iter().filter(|f| !still.contains(f)).collect::<Vec<_>>()) } else { 0 };
                self.x.gal_sel.clear();
                self.x.gal_pick = false;
                self.viewing = None;
                self.dirty = true;
                self.status = if moved > 0 {
                    format!("Removed {} item(s) and moved {} file(s) to the Removed files folder.", removed, moved)
                } else {
                    format!("Removed {} item(s). Undo (Ctrl+Z) brings them back.", removed)
                };
            }
        }
        if self.x.gal_online_confirm {
            let online = self.db.people.values().map(|p| p.media.iter().filter(|m| online_photo(m)).count()).sum::<usize>();
            let people = self.db.people.values().filter(|p| p.media.iter().any(online_photo)).count();
            let (mut go, mut cancel) = (false, false);
            egui::Window::new("Remove online links").id(egui::Id::new("gal_online")).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)).show(ui.ctx(), |ui| {
                ui.label(format!("Remove all {} links to photos stored on websites, from {} people?", online, people));
                ui.label(RichText::new("These came with a tree from Ancestry, MyHeritage or a similar site: the file held a web address instead of the picture. Nothing on your computer is deleted, and Undo (Ctrl+Z) brings the links back.").small().weak());
                ui.label(RichText::new("To keep a picture, download it from the website first and add it with + Add.").small().weak());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Remove links").strong()).clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if cancel || online == 0 {
                self.x.gal_online_confirm = false;
            }
            if go {
                self.x.gal_online_confirm = false;
                let n = remove_online_links(&mut self.db);
                self.x.gal_sel.clear();
                self.viewing = None;
                self.dirty = true;
                self.status = format!("Removed {} online photo link(s). Undo (Ctrl+Z) brings them back.", n);
            }
        }
        if let Some((who, i)) = story_open {
            if let Some(m) = self.db.people.get(&who).and_then(|p| p.media.get(i)) {
                let text = safe_media_path(&m.file).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
                self.x.story = Some(StoryEdit { who, idx: Some(i), title: m.caption.clone(), text });
            }
        }
    }

    /// Write or read a story. Stories are kept as plain text files in the media folder, so they
    /// are backed up and exported like any other document.
    pub(crate) fn story_window(&mut self, ctx: &egui::Context) {
        let Some(st) = self.x.story.as_mut() else { return };
        let mut open = true;
        let mut save = false;
        let who = self.db.people.get(&st.who).map(|p| p.name.clone()).unwrap_or_default();
        egui::Window::new(format!("Story about {}", who)).id(egui::Id::new("story_win")).open(&mut open).default_size([620.0, 520.0]).show(ctx, |ui| {
            ui.add(egui::TextEdit::singleline(&mut st.title).hint_text("Title, for example \"How Grandpa met Grandma\"").font(egui::TextStyle::Heading).desired_width(f32::INFINITY));
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut st.text).desired_rows(18).desired_width(f32::INFINITY).font(egui::TextStyle::Body));
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(!st.text.trim().is_empty(), egui::Button::new("Save story")).clicked() {
                    save = true;
                }
                ui.label(RichText::new("Saved as a text file with this person's photos and documents.").small().weak());
            });
        });
        if save {
            let st = self.x.story.take().unwrap();
            let title = if st.title.trim().is_empty() { "Story".to_string() } else { st.title.trim().to_string() };
            let file = match st.idx.and_then(|i| self.db.people.get(&st.who).and_then(|p| p.media.get(i)).map(|m| m.file.clone())) {
                Some(f) if !f.is_empty() => f,
                _ => format!("{}-{}.txt", stamp(), sanitize(&title)),
            };
            let Some(target) = safe_media_path(&file) else {
                self.status = "Could not save the story: its file name is not allowed.".to_string();
                return;
            };
            match atomic_write(&target, st.text.as_bytes()) {
                Ok(_) => {
                    if let Some(p) = self.db.people.get_mut(&st.who) {
                        match st.idx.and_then(|i| p.media.get_mut(i)) {
                            Some(m) => m.caption = title.clone(),
                            None => p.media.push(Media { file, name: format!("{}.txt", title), caption: title.clone(), form: "story".into(), ..Default::default() }),
                        }
                    }
                    self.dirty = true;
                    self.status = format!("Saved the story \"{}\".", title);
                }
                Err(e) => self.status = format!("Could not save the story: {}", e),
            }
        } else if !open {
            self.x.story = None;
        }
    }
}
