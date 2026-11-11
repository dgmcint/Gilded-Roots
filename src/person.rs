//! The Individual page and its sections.

use super::*;

impl App {
    /// One block of the Individual page. The big blocks live in the dock on the right.
    /// The Individual page while it is locked: plain, readable text instead of greyed-out boxes.
pub(crate)     fn person_read(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, sec: &str) {
        let Some(p) = self.db.people.get(&id).cloned() else { return };
        let mut pdf_req: Option<(PathBuf, String)> = None;
        // A value laid out at a set width (wrapping only text longer than that), so inside a grid
        // it keeps its width; a plain wrapping label there shrinks to one word per line.
        let val = |ui: &mut egui::Ui, t: &str| {
            let w = (ui.ctx().screen_rect().width() * 0.42).clamp(260.0, 720.0);
            let g = ui.fonts(|f| f.layout(t.to_string(), FontId::proportional(15.0), ui.visuals().text_color(), w));
            ui.add(egui::Label::new(g));
        };
        let lab = |ui: &mut egui::Ui, t: &str| {
            ui.label(RichText::new(t).size(14.0).weak());
        };
        let both = |d: &str, pl: &str| -> String {
            match (d.trim().is_empty(), pl.trim().is_empty()) {
                (true, true) => String::new(),
                (false, true) => show_date(d),
                (true, false) => format!("in {}", pl.trim()),
                (false, false) => format!("{}  -  {}", show_date(d), pl.trim()),
            }
        };
        let none = |ui: &mut egui::Ui, t: &str| {
            ui.label(RichText::new(t).italics().weak());
        };
        match sec {
            "facts" => {
                ui.horizontal(|ui| {
                    let k = match self.settings.photo_size { 0 => 0.75, 2 => 1.45, 3 => 2.0, _ => 1.0 };
                    let sz = vec2(110.0 * k, 130.0 * k);
                    match self.portrait_src(&p) {
                        Some((uri, fr)) => {
                            let (r, _) = ui.allocate_exact_size(sz, Sense::hover());
                            put_photo(ui, r, &uri, fr, 6.0);
                        }
                        None => {
                            let (r, _) = ui.allocate_exact_size(sz, Sense::hover());
                            ui.painter().rect_filled(r, 6.0, ui.visuals().faint_bg_color);
                            ui.painter().rect_stroke(r, 6.0, ui.visuals().widgets.noninteractive.bg_stroke);
                            let ini = p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, ini, FontId::proportional(48.0), ui.visuals().weak_text_color());
                        }
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        let mut full = String::new();
                        for part in [p.prefix.trim(), p.name.trim(), p.suffix.trim()] {
                            if !part.is_empty() {
                                if !full.is_empty() {
                                    full.push(' ');
                                }
                                full.push_str(part);
                            }
                        }
                        if full.is_empty() {
                            full = "(no name)".into();
                        }
                        ui.label(RichText::new(full).size(26.0).strong());
                        let l = life(&p);
                        let sex = match p.sex.as_str() {
                            "M" => "Male",
                            "F" => "Female",
                            _ => "Sex not recorded",
                        };
                        let mut sub = sex.to_string();
                        if !l.is_empty() {
                            sub = format!("{}   |   {}", l, sub);
                        }
                        if p.living {
                            sub.push_str("   |   Living");
                        }
                        ui.label(RichText::new(sub).size(16.0));
                        if !p.nick.trim().is_empty() {
                            ui.label(RichText::new(format!("Known as \"{}\"", p.nick.trim())).size(15.0).italics());
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Person ID #{}", id)).small().weak());
                            if let Some(old) = old_id(&self.db.people[&id]) {
                                ui.label(RichText::new(format!("(Old ID #{})", old)).small().weak()).on_hover_text("This person's number before the tree was renumbered (Tools > Renumber people)");
                            }
                            if self.db.home == Some(id) {
                                ui.label(RichText::new("Home person").small().weak());
                            }
                        });
                    });
                });
                ui.add_space(10.0);
                ui.label(RichText::new("Vital facts").size(17.0).strong());
                ui.separator();
                let cf = |t: &str| p.fx.get(t).map(|e| e.conf.clone()).unwrap_or_default();
                let rows: Vec<(&str, String, String)> = vec![
                    ("Born", both(&p.bd, &p.bp), cf("BIRT")),
                    (if p.chtag == "BAPM" { "Baptised" } else { "Christened" }, both(&p.chd, &p.chp), cf("CHR")),
                    ("Died", both(&p.dd, &p.dp), cf("DEAT")),
                    ("Buried", both(&p.bud, &p.bup), cf("BURI")),
                    ("Maiden name", p.maiden.trim().to_string(), String::new()),
                ];
                let shown: Vec<_> = rows.into_iter().filter(|r| !r.1.is_empty()).collect();
                if shown.is_empty() {
                    none(ui, "No dates or places yet. Press Edit to add them.");
                } else {
                    egui::Grid::new("read_facts").num_columns(3).spacing([24.0, 8.0]).show(ui, |ui| {
                        for (k, v, c) in &shown {
                            lab(ui, k);
                            val(ui, v);
                            conf_badge(ui, c);
                            ui.end_row();
                        }
                    });
                }
            }
            "timeline" => {
                let tl = self.db.timeline(id);
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Timeline").size(17.0).strong());
                    if ui.checkbox(&mut self.settings.show_history, "with historical events").changed() {
                        save_settings(&self.settings);
                    }
                });
                ui.separator();
                if tl.is_empty() {
                    none(ui, "Add dates to see a timeline.");
                } else {
                    // the person's own events, with historical events slotted in by year
                    let mut rows: Vec<(i32, bool, String, String)> = tl.iter().enumerate().map(|(i, (d, t))| (parse_date(d).0.unwrap_or(i32::MIN + i as i32), false, d.clone(), t.clone())).collect();
                    if self.settings.show_history {
                        for (y, t) in self.history_for(&p) {
                            rows.push((y, true, String::new(), t));
                        }
                    }
                    rows.sort_by_key(|r| (r.0, r.1));
                    egui::Grid::new("read_tl").num_columns(2).striped(true).spacing([24.0, 6.0]).show(ui, |ui| {
                        for (_, hist, d, t) in &rows {
                            if *hist {
                                ui.label("");
                                ui.add(egui::Label::new(RichText::new(t.as_str()).italics().weak()).wrap());
                            } else {
                                ui.label(RichText::new(if d.is_empty() { "-".to_string() } else { show_date(d) }).size(14.0).strong());
                                val(ui, t);
                            }
                            ui.end_row();
                        }
                    });
                }
            }
            "relationships" => {
                let link = |ui: &mut egui::Ui, act: &mut Option<Action>, who: u32, extra: &str| {
                    ui.horizontal(|ui| {
                        if ui.link(RichText::new(self.db.label(who)).size(15.0)).clicked() {
                            *act = Some(Action::Select(who));
                        }
                        if !extra.is_empty() {
                            ui.label(RichText::new(extra).small().weak());
                        }
                    });
                };
                let pf = self.db.parent_families(id);
                lab(ui, if pf.len() > 1 { "Parents (more than one set)" } else { "Parents" });
                if pf.is_empty() {
                    none(ui, "None recorded");
                }
                for (n, fid) in pf.iter().enumerate() {
                    let f = self.db.fams[fid].clone();
                    if pf.len() > 1 {
                        ui.label(RichText::new(format!("Set {}", n + 1)).small().weak());
                    }
                    for (who, father) in [(f.h, true), (f.w, false)] {
                        if let Some(w) = who.filter(|w| self.db.people.contains_key(w)) {
                            let r = f.rel_to(id, father);
                            let extra = if rel_is_bio(&r) { String::new() } else { format!("({})", r.to_lowercase()) };
                            link(ui, act, w, &extra);
                        }
                    }
                }
                ui.add_space(6.0);
                let fams = self.db.fams_of(id);
                lab(ui, "Spouses and children");
                if fams.is_empty() {
                    none(ui, "None recorded");
                }
                for fid in &fams {
                    let f = self.db.fams[fid].clone();
                    let other = if f.h == Some(id) { f.w } else { f.h };
                    let m = both(&f.md, &f.mp);
                    match other.filter(|o| self.db.people.contains_key(o)) {
                        Some(o) => link(ui, act, o, &if m.is_empty() { String::new() } else { format!("married {}", m) }),
                        None => {
                            ui.label(RichText::new("Unknown partner").italics());
                        }
                    }
                    for k in &f.kids {
                        if self.db.people.contains_key(k) {
                            ui.horizontal(|ui| {
                                ui.add_space(18.0);
                                link(ui, act, *k, self.db.child_note(*fid, *k).trim());
                            });
                        }
                    }
                }
                let sibs = self.db.siblings(id);
                if !sibs.is_empty() {
                    ui.add_space(6.0);
                    lab(ui, "Brothers and sisters");
                    for s2 in sibs {
                        link(ui, act, s2, "");
                    }
                }
                if !p.assoc.is_empty() {
                    ui.add_space(6.0);
                    lab(ui, "Other ties");
                    for a in &p.assoc {
                        if self.db.people.contains_key(&a.other) {
                            link(ui, act, a.other, &format!("({})", a.role));
                        }
                    }
                }
            }
            "contact" => {
                let c = &p.contact;
                if c.is_empty() {
                    none(ui, "No contact details. Press Edit to add an address, phone or email.");
                } else {
                    egui::Grid::new("read_contact").num_columns(2).spacing([18.0, 4.0]).show(ui, |ui| {
                        for (label, v) in [("Address", &c.addr), ("Phone", &c.phone), ("Email", &c.email), ("Web page", &c.url)] {
                            if v.trim().is_empty() {
                                continue;
                            }
                            ui.label(RichText::new(label).weak());
                            if label == "Email" {
                                ui.hyperlink_to(v.trim(), format!("mailto:{}", v.trim()));
                            } else if label == "Web page" && (v.starts_with("http://") || v.starts_with("https://")) {
                                ui.hyperlink_to(v.trim(), v.trim());
                            } else {
                                val(ui, v.trim());
                            }
                            ui.end_row();
                        }
                    });
                    if !p.living {
                        ui.label(RichText::new("This person is not marked as living.").small().weak());
                    }
                }
            }
            "lds" => {
                let rows = self.lds_rows(id);
                if rows.iter().all(|r| r.event.is_none()) {
                    none(ui, "No ordinances recorded. Press Edit to add them.");
                }
                egui::Grid::new("read_lds").num_columns(2).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                    for r in &rows {
                        let Some(e) = &r.event else { continue };
                        ui.label(RichText::new(&r.label).size(14.0).strong());
                        ui.vertical(|ui| {
                            let mut parts = vec![];
                            if !e.date.trim().is_empty() {
                                parts.push(show_date(&e.date));
                            }
                            if !e.temp.trim().is_empty() {
                                parts.push(temples::temple_label(&e.temp));
                            }
                            val(ui, if parts.is_empty() { "-".to_string() } else { parts.join("  -  ") }.as_str());
                            if !e.stat.trim().is_empty() {
                                ui.label(RichText::new(temples::status_label(&e.stat)).small().weak());
                            }
                        });
                        ui.end_row();
                    }
                });
            }
            "events" => {
                if p.events.is_empty() {
                    none(ui, "No other events.");
                } else {
                    egui::Grid::new("read_ev").num_columns(2).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                        for e in &p.events {
                            ui.label(RichText::new(if e.kind.is_empty() { "Other" } else { e.kind.as_str() }).size(14.0).strong());
                            ui.vertical(|ui| {
                                let mut line = both(&e.date, &e.place);
                                if !e.val.trim().is_empty() {
                                    line = if line.is_empty() { e.val.trim().to_string() } else { format!("{}  -  {}", e.val.trim(), line) };
                                }
                                val(ui, if line.is_empty() { "-" } else { line.as_str() });
                                if !e.note.trim().is_empty() {
                                    ui.add(egui::Label::new(RichText::new(e.note.trim()).small().weak()).wrap());
                                }
                                if !e.cites.is_empty() {
                                    ui.label(RichText::new(format!("{} source(s)", e.cites.len())).small().weak());
                                }
                                conf_badge(ui, &e.conf);
                            });
                            ui.end_row();
                        }
                    });
                }
            }
            "sources" => {
                if p.sources.is_empty() {
                    none(ui, "No sources yet.");
                }
                for c in &p.sources {
                    ui.vertical(|ui| {
                        let title = if c.text.trim().is_empty() { "(untitled source)" } else { c.text.trim() };
                        val(ui, title);
                        let mut bits: Vec<String> = Vec::new();
                        if !c.about.trim().is_empty() {
                            bits.push(format!("for {}", c.about.trim()));
                        }
                        if !c.page.trim().is_empty() {
                            bits.push(format!("page {}", c.page.trim()));
                        }
                        if !c.quality.trim().is_empty() {
                            bits.push(format!("quality {}", c.quality.trim()));
                        }
                        if !bits.is_empty() {
                            ui.label(RichText::new(bits.join("  |  ")).small().weak());
                        }
                    });
                    ui.add_space(4.0);
                }
            }
            "notes" => {
                let mut any = false;
                if !p.notes.trim().is_empty() {
                    any = true;
                    val(ui, p.notes.trim());
                }
                for nid in &p.note_refs {
                    if let Some(n) = self.db.notes.get(nid) {
                        if !n.text.trim().is_empty() {
                            any = true;
                            ui.add_space(4.0);
                            val(ui, n.text.trim());
                        }
                    }
                }
                if !any {
                    none(ui, "No notes.");
                }
            }
            "names" => {
                if p.alt_names.is_empty() {
                    none(ui, "No other names.");
                }
                egui::Grid::new("read_names").num_columns(2).spacing([18.0, 6.0]).show(ui, |ui| {
                    for a in &p.alt_names {
                        lab(ui, if a.kind.is_empty() { "Other name" } else { a.kind.as_str() });
                        val(ui, &compose_name(&a.given, &a.surname));
                        ui.end_row();
                    }
                });
            }
            "media" => {
                let photos: Vec<(usize, &Media)> = p.media.iter().enumerate().filter(|(_, m)| is_image(&m.file)).collect();
                let others: Vec<(usize, &Media)> = p.media.iter().enumerate().filter(|(_, m)| !is_image(&m.file)).collect();
                if photos.is_empty() && others.is_empty() {
                    none(ui, "No photos, documents or recordings.");
                }
                ui.horizontal_wrapped(|ui| {
                    for (i, m) in &photos {
                        ui.vertical(|ui| {
                            ui.set_width(120.0);
                            if !m.thumb.is_empty() && m.thumb != "-" {
                                let uri = img_uri(&mprev(&m.thumb));
                                let r = ui.add(egui::Image::new(uri).fit_to_exact_size(vec2(120.0, 90.0)).rounding(4.0).sense(Sense::click()));
                                if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    self.viewing = Some((id, *i));
                                }
                            } else if ui.add_sized([120.0, 90.0], egui::Button::new("Open")).clicked() {
                                if m.thumb.is_empty() {
                                    self.viewing = Some((id, *i));
                                } else {
                                    open_media(&m.file);
                                }
                            }
                            let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                            ui.label(RichText::new(cap).small());
                        });
                    }
                });
                self.tagged_strip(ui, id);
                for (_, m) in &others {
                    ui.horizontal(|ui| {
                        let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                        val(ui, cap);
                        if !m.file.is_empty() && ui.small_button(if is_audio(&m.file) { "Play" } else if is_pdf(&m.file) { "Read" } else { "Open" }).clicked() {
                            if is_pdf(&m.file) {
                                pdf_req = Some((mfile(&m.file), cap.to_string()));
                            } else {
                                open_media(&m.file);
                            }
                        }
                        if m.file.is_empty() && is_url(&m.path) {
                            ui.hyperlink_to("Open online", m.path.as_str());
                        }
                    });
                }
            }
            "more" => {
                none(ui, "Press Edit to merge or delete this person.");
            }
            "todo" => self.todo_panel(ui, id),
            "journal" => self.journal_panel(ui, id),
            other => {
                ui.add_enabled_ui(false, |ui| self.person_section(ui, act, id, other));
            }
        }
        if pdf_req.is_some() {
            self.x.pdf_req = pdf_req;
        }
    }

pub(crate)     fn person_section(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, sec: &str) {
        let mut pdf_req: Option<(PathBuf, String)> = None;
        self.person_section_inner(ui, act, id, sec, &mut pdf_req);
        if pdf_req.is_some() {
            self.x.pdf_req = pdf_req;
        }
    }

pub(crate)     fn person_section_inner(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, sec: &str, pdf_req: &mut Option<(PathBuf, String)>) {
        match sec {
            "facts" => {
                let home = self.db.home;
                let mut tomb_req = false;
            {
                let p = self.db.people.get_mut(&id).unwrap();
                egui::Grid::new("pg").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Given name(s)");
                    let r1 = ui.add(egui::TextEdit::singleline(&mut p.given).desired_width(320.0));
                    ui.end_row();
                    ui.label("Surname");
                    let r2 = ui.add(egui::TextEdit::singleline(&mut p.surname).desired_width(320.0));
                    ui.end_row();
                    if r1.changed() || r2.changed() {
                        p.name = compose_name(&p.given, &p.surname);
                    }
                    ui.label("Maiden name");
                    ui.add(egui::TextEdit::singleline(&mut p.maiden).hint_text("if different from surname").desired_width(320.0));
                    ui.end_row();
                    ui.label("Title / suffix / nickname");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut p.prefix).hint_text("Dr., Rev.").desired_width(90.0));
                        ui.add(egui::TextEdit::singleline(&mut p.suffix).hint_text("Jr., III").desired_width(90.0));
                        ui.add(egui::TextEdit::singleline(&mut p.nick).hint_text("nickname").desired_width(120.0));
                    });
                    ui.end_row();
                    ui.label("Sex");
                    let st = match p.sex.as_str() {
                        "M" => "Male",
                        "F" => "Female",
                        _ => "Unknown",
                    };
                    egui::ComboBox::from_id_salt("sex").selected_text(st).show_ui(ui, |ui| {
                        ui.selectable_value(&mut p.sex, "M".to_string(), "Male");
                        ui.selectable_value(&mut p.sex, "F".to_string(), "Female");
                        ui.selectable_value(&mut p.sex, String::new(), "Unknown");
                    });
                    ui.end_row();
                    ui.label("Person ID");
                    ui.label(RichText::new(format!("#{}", id)).weak());
                    ui.end_row();
                    if home == Some(id) {
                        ui.label("Home person");
                        ui.label("This is the home person (change it in Tools > Home person...)");
                        ui.end_row();
                    }
                    ui.label("Living");
                    ui.checkbox(&mut p.living, "Living (kept private when printing or exporting)");
                    ui.end_row();
                    ui.label("Born");
                    ui.horizontal(|ui| {
                        ditto(ui, &mut p.bd, |t| t.hint_text("e.g. 12 Mar 1921, about 1850").desired_width(320.0), &mut self.x.ditto_date);
                        let mut c = p.fx.get("BIRT").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "BIRT"), &mut c) {
                            p.fx.entry("BIRT".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Birthplace");
                    ditto(ui, &mut p.bp, |t| t.desired_width(320.0), &mut self.x.ditto_place);
                    ui.end_row();
                    ui.label("Christened");
                    ui.horizontal(|ui| {
                        ditto(ui, &mut p.chd, |t| t.desired_width(320.0), &mut self.x.ditto_date);
                        let mut c = p.fx.get("CHR").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "CHR"), &mut c) {
                            p.fx.entry("CHR".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Christening place");
                    ditto(ui, &mut p.chp, |t| t.desired_width(320.0), &mut self.x.ditto_place);
                    ui.end_row();
                    ui.label("Died");
                    ui.horizontal(|ui| {
                        ditto(ui, &mut p.dd, |t| t.desired_width(320.0), &mut self.x.ditto_date);
                        let mut c = p.fx.get("DEAT").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "DEAT"), &mut c) {
                            p.fx.entry("DEAT".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Place of death");
                    ditto(ui, &mut p.dp, |t| t.desired_width(320.0), &mut self.x.ditto_place);
                    ui.end_row();
                    ui.label("Buried");
                    ui.horizontal(|ui| {
                        ditto(ui, &mut p.bud, |t| t.desired_width(320.0), &mut self.x.ditto_date);
                        let mut c = p.fx.get("BURI").map(|e| e.conf.clone()).unwrap_or_default();
                        if conf_combo(ui, ("cf", "BURI"), &mut c) {
                            p.fx.entry("BURI".to_string()).or_default().conf = c;
                        }
                    });
                    ui.end_row();
                    ui.label("Burial place");
                    ditto(ui, &mut p.bup, |t| t.desired_width(320.0), &mut self.x.ditto_place);
                    ui.end_row();
                    if !p.dd.trim().is_empty() || !p.bud.trim().is_empty() {
                        ui.label("");
                        if ui.small_button("Calculate birth from gravestone age...").on_hover_text("Tombstone calculator: work out the birth date from the age at death").clicked() {
                            tomb_req = true;
                        }
                        ui.end_row();
                    }
                });
            }
            if tomb_req {
                self.open_tombstone(Some(id));
            }
            }
            "relationships" => {
        let (fa, mo) = self.db.parents(id);
        let fl = fa.map(|i| (i, self.db.label(i)));
        let ml = mo.map(|i| (i, self.db.label(i)));
        let fams: Vec<(u32, Option<(u32, String)>, Vec<(u32, String)>)> = self
            .db
            .fams_of(id)
            .into_iter()
            .map(|fid| {
                let f = &self.db.fams[&fid];
                let sp = if f.h == Some(id) { f.w } else { f.h };
                (fid, sp.map(|s| (s, self.db.label(s))), f.kids.iter().map(|k| (*k, format!("{}{}", self.db.label(*k), self.db.child_note(fid, *k)))).collect())
            })
            .collect();
            ui.heading("Parents");
            let pfams: Vec<(u32, Option<(u32, String)>, Option<(u32, String)>, String, String)> = self
                .db
                .parent_families(id)
                .into_iter()
                .map(|fid| {
                    let f = &self.db.fams[&fid];
                    (fid, f.h.map(|i| (i, self.db.label(i))), f.w.map(|i| (i, self.db.label(i))), f.rel_to(id, true), f.rel_to(id, false))
                })
                .collect();
            let shown_fam = self.db.famc(id);
            if pfams.is_empty() {
                for (role, link, kind) in [("Father", &fl, Kind::Father), ("Mother", &ml, Kind::Mother)] {
                    ui.horizontal(|ui| {
                        ui.label(role);
                        match link {
                            Some((i, l)) => {
                                if ui.link(l.as_str()).clicked() {
                                    *act = Some(Action::Select(*i));
                                }
                            }
                            None => {
                                if ui.button(format!("Add {}", role.to_lowercase())).clicked() {
                                    *act = Some(Action::Add(kind));
                                }
                                if ui.button("Link existing person").clicked() {
                                    self.link = Some(LinkReq { child: id, fid: None, father: kind == Kind::Father, q: String::new() });
                                }
                            }
                        }
                    });
                }
            }
            let many = pfams.len() > 1;
            for (n, (fid, h, w, frel, mrel)) in pfams.iter().enumerate() {
                ui.group(|ui| {
                    let bio = rel_is_bio(frel) && rel_is_bio(mrel);
                    let mut title = if many { format!("Set of parents {}", n + 1) } else { "Parents".to_string() };
                    title.push_str(if bio { " - biological" } else { " - not both biological" });
                    if many && shown_fam == Some(*fid) {
                        title.push_str(" (shown on the chart)");
                    }
                    ui.label(RichText::new(title).strong());
                    for (role, slot, cur, father) in [("Father", h, frel, true), ("Mother", w, mrel, false)] {
                        ui.horizontal(|ui| {
                            ui.label(role);
                            match slot {
                                Some((i, l)) => {
                                    if ui.link(l.as_str()).clicked() {
                                        *act = Some(Action::Select(*i));
                                    }
                                    if let Some(newv) = rel_combo(ui, ("prel", *fid, father), cur) {
                                        if let Some(f) = self.db.fams.get_mut(fid) {
                                            let m = if father { &mut f.frel } else { &mut f.mrel };
                                            if newv.is_empty() { m.remove(&id); } else { m.insert(id, newv); }
                                        }
                                    }
                                }
                                None => {
                                    if ui.button(format!("Add {}", role.to_lowercase())).clicked() {
                                        *act = Some(Action::Add(Kind::ParentIn(*fid, father)));
                                    }
                                    if ui.button("Link existing person").clicked() {
                                        self.link = Some(LinkReq { child: id, fid: Some(*fid), father, q: String::new() });
                                    }
                                }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        if many && shown_fam != Some(*fid) && ui.button("Show these parents on the chart").clicked() {
                            if let Some(p) = self.db.people.get_mut(&id) {
                                p.pref_parents = Some(*fid);
                            }
                        }
                        if many && ui.button("Remove from this family").on_hover_text("The people stay; only this person's link to this set of parents is removed.").clicked() {
                            self.db.remove_from_family(id, *fid);
                        }
                    });
                });
            }
            if !pfams.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Another set of parents (adoptive, step, foster, guardian...):");
                    if ui.button("Add father").clicked() {
                        *act = Some(Action::Add(Kind::NewParent(true)));
                    }
                    if ui.button("Add mother").clicked() {
                        *act = Some(Action::Add(Kind::NewParent(false)));
                    }
                    if ui.button("Link existing person").clicked() {
                        self.link = Some(LinkReq { child: id, fid: None, father: true, q: String::new() });
                    }
                });
                ui.label(RichText::new("Choose how each parent is related (biological, adopted, step, foster, guardian) in the drop-downs above. Biological and legal parents are kept as separate sets.").small().weak());
            }
            ui.add_space(10.0);
            ui.heading("Family groups");
            for (fid, sp, kids) in &fams {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Spouse");
                        match sp {
                            Some((s, l)) => {
                                if ui.link(l.as_str()).clicked() {
                                    *act = Some(Action::Select(*s));
                                }
                            }
                            None => {
                                ui.weak("not recorded");
                            }
                        }
                    });
                    if let Some(f) = self.db.fams.get_mut(fid) {
                        egui::Grid::new(("mg", *fid)).num_columns(2).show(ui, |ui| {
                            ui.label("Married");
                            ditto(ui, &mut f.md, |t| t, &mut self.x.ditto_date);
                            ui.end_row();
                            ui.label("Place");
                            ditto(ui, &mut f.mp, |t| t, &mut self.x.ditto_place);
                            ui.end_row();
                            ui.label("Partnership");
                            let shown = if f.kind.is_empty() { "Not stated" } else { f.kind.as_str() };
                            egui::ComboBox::from_id_salt(("pkind", *fid)).selected_text(shown).show_ui(ui, |ui| {
                                if ui.selectable_label(f.kind.is_empty(), "Not stated").clicked() {
                                    f.kind.clear();
                                }
                                for k in PARTNER_KINDS {
                                    if ui.selectable_label(f.kind == *k, *k).clicked() {
                                        f.kind = k.to_string();
                                    }
                                }
                            });
                            ui.end_row();
                        });
                    }
                    ui.label("Children");
                    for (k, l) in kids {
                        ui.horizontal(|ui| {
                            if ui.link(l.as_str()).clicked() {
                                *act = Some(Action::Select(*k));
                            }
                            if let Some(f) = self.db.fams.get_mut(fid) {
                                let cur = f.pedi.get(k).cloned().unwrap_or_default();
                                let shown = if cur.is_empty() { "birth child".to_string() } else { format!("{} child", cur) };
                                egui::ComboBox::from_id_salt(("pedi", *fid, *k)).selected_text(shown).width(110.0).show_ui(ui, |ui| {
                                    for (v, lbl) in [("", "birth child"), ("adopted", "adopted child"), ("foster", "foster child"), ("sealing", "sealed child")] {
                                        if ui.selectable_label(cur == v, lbl).clicked() {
                                            if v.is_empty() {
                                                f.pedi.remove(k);
                                            } else {
                                                f.pedi.insert(*k, v.to_string());
                                            }
                                        }
                                    }
                                });
                                for (father, who) in [(true, if f.h.is_some() { "father" } else { "" }), (false, if f.w.is_some() { "mother" } else { "" })] {
                                    if who.is_empty() {
                                        continue;
                                    }
                                    let cur = f.rel_to(*k, father);
                                    ui.label(RichText::new(who).small().weak());
                                    if let Some(nv) = rel_combo(ui, ("crel", *fid, *k, father), &cur) {
                                        let m = if father { &mut f.frel } else { &mut f.mrel };
                                        if nv.is_empty() { m.remove(k); } else { m.insert(*k, nv); }
                                    }
                                }
                            }
                        });
                    }
                    if ui.button("Add child").clicked() {
                        *act = Some(Action::Add(Kind::Child(Some(*fid))));
                    }
                    if let Some(f) = self.db.fams.get_mut(fid) {
                        egui::CollapsingHeader::new("Marriage details, family events and notes").id_salt(("fmore", *fid)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Kind of marriage");
                                ui.add(egui::TextEdit::singleline(&mut f.mx.typ).hint_text("civil, religious...").desired_width(160.0));
                            });
                            ui.label("Note about the marriage");
                            ui.add(egui::TextEdit::multiline(&mut f.mx.note).desired_rows(2).desired_width(f32::INFINITY));
                            ui.label("Sources for the marriage");
                            cites_section(ui, ("msrc", *fid), &mut f.mx.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                            ui.label("Sources for this family as a whole");
                            cites_section(ui, ("fsrc", *fid), &mut f.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                            let mut remove: Option<usize> = None;
                            for (i, e) in f.events.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_salt(("fev", *fid, i)).selected_text(if e.kind.is_empty() { "Other" } else { e.kind.as_str() }).width(140.0).show_ui(ui, |ui| {
                                        for k in FAM_KINDS.iter() {
                                            ui.selectable_value(&mut e.kind, k.to_string(), *k);
                                        }
                                    });
                                    ditto(ui, &mut e.date, |t| t.hint_text("date").desired_width(100.0), &mut self.x.ditto_date);
                                    ditto(ui, &mut e.place, |t| t.hint_text("place").desired_width(130.0), &mut self.x.ditto_place);
                                    ui.add(egui::TextEdit::singleline(&mut e.note).hint_text("note").desired_width(130.0));
                                    if ui.button("Remove").clicked() {
                                        remove = Some(i);
                                    }
                                });
                                cites_section(ui, ("fevsrc", *fid, i), &mut e.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                            }
                            if let Some(i) = remove {
                                f.events.remove(i);
                            }
                            if ui.button("Add family event").clicked() {
                                f.events.push(Event { kind: "Divorce".to_string(), ..Default::default() });
                            }
                            ui.label("Notes about this family");
                            ui.add(egui::TextEdit::multiline(&mut f.notes).desired_rows(3).desired_width(f32::INFINITY));
                            for nid in f.note_refs.clone() {
                                if let Some(n) = self.db.notes.get_mut(&nid) {
                                    ui.label(RichText::new("Shared note").small().weak());
                                    ui.add(egui::TextEdit::multiline(&mut n.text).desired_rows(2).desired_width(f32::INFINITY));
                                }
                            }
                        });
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button("Add spouse").clicked() {
                    *act = Some(Action::Add(Kind::Spouse));
                }
                if fams.is_empty() && ui.button("Add child").clicked() {
                    *act = Some(Action::Add(Kind::Child(None)));
                }
            });
            ui.add_space(10.0);
            ui.heading("Step and half family");
            {
                let kin = self.db.kin_extras(id);
                let lines: [(&str, &Vec<u32>); 4] = [("Step-parents", &kin.step_parents), ("Half-siblings", &kin.half_siblings), ("Step-siblings", &kin.step_siblings), ("Step-children", &kin.step_children)];
                let mut any = false;
                for (lbl, ids) in lines {
                    if ids.is_empty() {
                        continue;
                    }
                    any = true;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(lbl).strong());
                        for i in ids {
                            if let Some(p) = self.db.people.get(i) {
                                if ui.link(display_name(p)).clicked() {
                                    *act = Some(Action::Select(*i));
                                }
                            }
                        }
                    });
                }
                if !any {
                    ui.weak("None found. These are worked out from the families above, so nothing needs to be entered.");
                }
            }
            ui.add_space(10.0);
            ui.heading("Other relationships");
            ui.label(RichText::new("Guardians, godparents, foster carers and anyone else connected to this person who is not a parent or spouse.").small().weak());
            {
                let kin_of = self.db.kin_extras(id).assoc_of;
                for (o, role) in &kin_of {
                    if let Some(p) = self.db.people.get(o) {
                        ui.horizontal(|ui| {
                            ui.label("Is the");
                            ui.label(RichText::new(role.as_str()).strong());
                            ui.label("of");
                            if ui.link(display_name(p)).clicked() {
                                *act = Some(Action::Select(*o));
                            }
                        });
                    }
                }
                let names: BTreeMap<u32, String> = self.db.people.iter().map(|(k, p)| (*k, display_name(p))).collect();
                let mut remove: Option<usize> = None;
                if let Some(p) = self.db.people.get_mut(&id) {
                    for (i, a) in p.assoc.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label("Has");
                            if ui.link(names.get(&a.other).cloned().unwrap_or_else(|| "(missing)".into())).clicked() {
                                *act = Some(Action::Select(a.other));
                            }
                            ui.label("as");
                            egui::ComboBox::from_id_salt(("arole", i)).selected_text(a.role.clone()).width(120.0).show_ui(ui, |ui| {
                                for r in ASSOC_ROLES {
                                    ui.selectable_value(&mut a.role, r.to_string(), *r);
                                }
                            });
                            ui.add(egui::TextEdit::singleline(&mut a.role).desired_width(100.0));
                            ui.add(egui::TextEdit::singleline(&mut a.note).hint_text("note").desired_width(160.0));
                            if ui.button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        p.assoc.remove(i);
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Add:");
                    egui::ComboBox::from_id_salt("newarole").selected_text(self.assoc_role.clone()).width(120.0).show_ui(ui, |ui| {
                        for r in ASSOC_ROLES {
                            ui.selectable_value(&mut self.assoc_role, r.to_string(), *r);
                        }
                    });
                    ui.add(egui::TextEdit::singleline(&mut self.assoc_q).hint_text("search for the person").desired_width(180.0));
                });
                if !self.assoc_q.trim().is_empty() {
                    let cands: Vec<(u32, String)> = self.db.sorted(&self.assoc_q).into_iter().filter(|i| *i != id).take(6).map(|i| (i, self.db.label(i))).collect();
                    for (i, lbl) in cands {
                        if ui.link(format!("+ {}", lbl)).clicked() {
                            let role = self.assoc_role.clone();
                            if let Some(p) = self.db.people.get_mut(&id) {
                                p.assoc.push(Assoc { other: i, role, note: String::new() });
                            }
                            self.assoc_q.clear();
                        }
                    }
                }
            }
            }
            "names" => {
            {
                let p = self.db.people.get_mut(&id).unwrap();
                let mut remove: Option<usize> = None;
                for (i, a) in p.alt_names.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let kinds = ["", "birth", "married", "aka", "religious", "immigrant"];
                        let shown = if a.kind.is_empty() { "other name".to_string() } else { a.kind.clone() };
                        egui::ComboBox::from_id_salt(("alt", i)).selected_text(shown).width(100.0).show_ui(ui, |ui| {
                            for k in kinds {
                                ui.selectable_value(&mut a.kind, k.to_string(), if k.is_empty() { "other name" } else { k });
                            }
                        });
                        ui.add(egui::TextEdit::singleline(&mut a.given).hint_text("given names").desired_width(150.0));
                        ui.add(egui::TextEdit::singleline(&mut a.surname).hint_text("surname").desired_width(130.0));
                        if ui.button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    p.alt_names.remove(i);
                }
                if ui.button("Add another name").clicked() {
                    p.alt_names.push(AltName { kind: "aka".to_string(), ..Default::default() });
                }
            }
            }
            "notes" => {
            {
                let p = self.db.people.get_mut(&id).unwrap();
                ui.add(egui::TextEdit::multiline(&mut p.notes).desired_rows(6).desired_width(f32::INFINITY));
            }
            {
                let refs: Vec<u32> = self.db.people.get(&id).map(|p| p.note_refs.clone()).unwrap_or_default();
                for nid in refs {
                    let uses = self.db.people.values().filter(|q| q.note_refs.contains(&nid)).count() + self.db.fams.values().filter(|f| f.note_refs.contains(&nid)).count();
                    if let Some(n) = self.db.notes.get_mut(&nid) {
                        ui.label(RichText::new(if uses > 1 { format!("Shared note (also used by {} other record(s); editing it changes all of them)", uses - 1) } else { "Linked note".to_string() }).small().weak());
                        ui.add(egui::TextEdit::multiline(&mut n.text).desired_rows(3).desired_width(f32::INFINITY));
                    }
                }
            }
            }
            "contact" => {
                ui.label(RichText::new("How to reach a living relative. Living people, and these details, are left out of private copies and printouts that keep the living private.").small().weak());
                let p = self.db.people.get_mut(&id).unwrap();
                egui::Grid::new("edit_contact").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("Address");
                    ui.add(egui::TextEdit::multiline(&mut p.contact.addr).desired_rows(2).desired_width(260.0));
                    ui.end_row();
                    ui.label("Phone");
                    ui.add(egui::TextEdit::singleline(&mut p.contact.phone).desired_width(260.0));
                    ui.end_row();
                    ui.label("Email");
                    ui.add(egui::TextEdit::singleline(&mut p.contact.email).desired_width(260.0));
                    ui.end_row();
                    ui.label("Web page");
                    ui.add(egui::TextEdit::singleline(&mut p.contact.url).hint_text("https://").desired_width(260.0));
                    ui.end_row();
                });
            }
            "lds" => {
                ui.label(RichText::new("Baptism, confirmation, initiatory, endowment and sealings, with the temple and status, as in PAF. Choose a temple from the list, or search it by place or code.").small().weak());
                let rows = self.lds_rows(id);
                let mut add: Option<LdsRow> = None;
                let mut remove: Option<LdsRow> = None;
                {
                    for (i, r) in rows.iter().enumerate() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(&r.label).strong());
                        let mut search = std::mem::take(&mut self.x.temple_search);
                        let mut last_date = std::mem::take(&mut self.x.ditto_date);
                        ui.horizontal_wrapped(|ui| match self.lds_event_mut(id, r) {
                            Some(e) => {
                                ditto(ui, &mut e.date, |t| t.hint_text("date").desired_width(110.0), &mut last_date);
                                temples::temple_picker(ui, ("lds_t", i), &mut e.temp, &mut search);
                                temples::status_picker(ui, ("lds_s", i), &mut e.stat);
                                if ui.small_button("Remove").clicked() {
                                    remove = Some(r.clone());
                                }
                            }
                            None => {
                                if ui.button("Add").clicked() {
                                    add = Some(r.clone());
                                }
                            }
                        });
                        self.x.temple_search = search;
                        self.x.ditto_date = last_date;
                    }
                }
                if let Some(r) = add {
                    self.lds_add(id, &r);
                }
                if let Some(r) = remove {
                    self.lds_remove(id, &r);
                }
            }
            "events" => {
            {
                let p = self.db.people.get_mut(&id).unwrap();
                let mut remove: Option<usize> = None;
                for (i, e) in p.events.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(("ev", i)).selected_text(if e.kind.is_empty() { "Other" } else { e.kind.as_str() }).width(140.0).show_ui(ui, |ui| {
                            for k in EVENT_KINDS.iter() {
                                ui.selectable_value(&mut e.kind, k.to_string(), *k);
                            }
                        });
                        ditto(ui, &mut e.date, |t| t.hint_text("date").desired_width(110.0), &mut self.x.ditto_date);
                        ditto(ui, &mut e.place, |t| t.hint_text("place").desired_width(150.0), &mut self.x.ditto_place);
                        ui.add(egui::TextEdit::singleline(&mut e.val).hint_text("details").desired_width(150.0));
                        conf_combo(ui, ("evcf", i), &mut e.conf);
                        if ui.button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.add(egui::TextEdit::singleline(&mut e.note).hint_text("note").desired_width(260.0));
                        if e.kind.starts_with("LDS") {
                            ui.add(egui::TextEdit::singleline(&mut e.temp).hint_text("temple").desired_width(70.0));
                            ui.add(egui::TextEdit::singleline(&mut e.stat).hint_text("status").desired_width(100.0));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            cites_section(ui, ("evsrc", i), &mut e.cites, &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
                        });
                    });
                }
                if let Some(i) = remove {
                    p.events.remove(i);
                }
                if ui.button("Add event").clicked() {
                    p.events.push(Event { kind: "Residence".to_string(), ..Default::default() });
                }
            }
            }
            "timeline" => {
                let tl = self.db.timeline(id);
            egui::CollapsingHeader::new("Timeline").default_open(false).show(ui, |ui| {
                if tl.is_empty() {
                    ui.label("Add dates to see a timeline.");
                }
                for (d, t) in &tl {
                    ui.label(format!("{}   {}", if d.is_empty() { "-" } else { d.as_str() }, t));
                }
            });
            }
            "sources" => {
            ui.label(RichText::new("Pick a source you have used before, or add a new one. A source can back up many people and facts.").small().weak());
            {
                let p = self.db.people.get_mut(&id).unwrap();
                cite_list(ui, "psrc", &mut p.sources, Some(&FACTS), "General", &mut self.db.sources, &mut self.db.next, &mut self.src_pick);
            }
            }
            "media" => {
            ui.heading("Photos");
            ui.horizontal(|ui| {
                if ui.button("Add photos or documents...").clicked() {
                    *act = Some(Action::AttachFiles);
                }
                if ui.button("Open media folder").on_hover_text(format!("{}", media_dir().display())).clicked() {
                    open_path(&media_dir());
                }
                match &self.rec {
                    Some(r) => {
                        let secs = r.started.elapsed().as_secs();
                        if ui.button(format!("Stop recording ({}:{:02})", secs / 60, secs % 60)).clicked() {
                            *act = Some(Action::StopRec);
                        }
                        let level = (r.peak.swap(0, Ordering::Relaxed) as f32 / 32768.0).sqrt().clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(level).desired_width(140.0).text("mic level"));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    None => {
                        if ui.button("Record a story").clicked() {
                            *act = Some(Action::StartRec);
                        }
                        let cur = if self.settings.mic.is_empty() { "System default".to_string() } else { self.settings.mic.clone() };
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
            let photos: Vec<(usize, Media)> = match self.db.people.get(&id) {
                Some(p) => p.media.iter().cloned().enumerate().filter(|(_, m)| is_image(&m.file)).collect(),
                None => Vec::new(),
            };
            if photos.is_empty() {
                ui.label(RichText::new("No photos yet. Click Add photos or documents.").weak());
            }
            ui.horizontal_wrapped(|ui| {
                for (i, m) in &photos {
                    ui.vertical(|ui| {
                        ui.set_width(150.0);
                        if m.thumb.is_empty() || m.thumb == "-" {
                            let label = if m.thumb == "-" { "No preview\nClick to open" } else { "Loading..." };
                            if ui.add_sized([150.0, 110.0], egui::Button::new(label)).clicked() {
                                // still making its preview: open it in our own viewer like any other photo;
                                // only a photo that cannot be previewed at all goes to the computer's own app
                                if m.thumb.is_empty() {
                                    self.viewing = Some((id, *i));
                                } else {
                                    open_media(&m.file);
                                }
                            }
                        } else {
                            let uri = img_uri(&mprev(&m.thumb));
                            let r = ui.add(egui::Image::new(uri).fit_to_exact_size(vec2(150.0, 110.0)).rounding(4.0).sense(Sense::click()));
                            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                self.viewing = Some((id, *i));
                            }
                        }
                        let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                        ui.label(RichText::new(cap).small());
                    });
                }
            });
            self.tagged_strip(ui, id);
            let others: Vec<(usize, Media)> = match self.db.people.get(&id) {
                Some(p) => p.media.iter().cloned().enumerate().filter(|(_, m)| !is_image(&m.file)).collect(),
                None => Vec::new(),
            };
            if !others.is_empty() {
                ui.add_space(8.0);
                ui.heading("Documents and recordings");
                let mut remove: Option<usize> = None;
                for (i, m) in &others {
                    ui.group(|ui| {
                        let full = mfile(&m.file);
                        ui.vertical(|ui| {
                            if let Some(mm) = self.db.people.get_mut(&id).and_then(|p| p.media.get_mut(*i)) {
                                ui.add(egui::TextEdit::singleline(&mut mm.caption).hint_text("Caption").desired_width(280.0));
                            }
                            ui.label(RichText::new(m.name.as_str()).small().weak());
                            if m.file.is_empty() && is_url(&m.path) {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Stored on a website, not on this computer.").small().weak());
                                    ui.hyperlink_to("Open online", m.path.as_str());
                                });
                            } else if m.file.is_empty() {
                                ui.label(RichText::new(format!("File not found. The GEDCOM file named it: {}", if m.path.is_empty() { "(no file name)" } else { m.path.as_str() })).small().weak());
                            }
                            ui.horizontal(|ui| {
                                let label = if is_audio(&m.file) { "Play" } else if is_pdf(&m.file) { "Read" } else { "Open" };
                                if !m.file.is_empty() && ui.button(label).clicked() {
                                    if is_pdf(&m.file) {
                                        *pdf_req = Some((full.clone(), if m.caption.is_empty() { m.name.clone() } else { m.caption.clone() }));
                                    } else {
                                        open_media(&m.file);
                                    }
                                }
                                if ui.button("Remove").clicked() {
                                    remove = Some(*i);
                                }
                            });
                        });
                    });
                }
                if let Some(i) = remove {
                    if let Some(p) = self.db.people.get_mut(&id) {
                        if i < p.media.len() {
                            p.media.remove(i);
                        }
                    }
                }
            }
            ui.label(RichText::new("Photos, documents and recordings are stored on this computer only.").small().weak());
            }
            "todo" => self.todo_panel(ui, id),
            "journal" => self.journal_panel(ui, id),
            "more" => {
            if ui.button("Merge with another person...").on_hover_text("Use this when the same person is in your tree twice").clicked() {
                self.merge_open = true;
                self.merge_other = None;
                self.merge_id.clear();
            }
            ui.add_space(4.0);
            let label = if self.confirm { "Click again to delete" } else { "Delete person" };
            if ui.button(label).clicked() {
                if self.confirm {
                    *act = Some(Action::Delete);
                } else {
                    self.confirm = true;
                }
            }
            }
            _ => {}
        }
    }

    /// The panels that are shown and docked, one after another, each with its own title bar.
pub(crate)     fn dock_panels(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>, id: u32, editing: bool) {
        let items = self.settings.dock.clone();
        let mut cmd: Option<(usize, u8)> = None; // 0 up, 1 down, 2 pop out, 3 hide
        for (i, it) in items.iter().enumerate() {
            if !it.on || it.float {
                continue;
            }
            let cid = ui.make_persistent_id(("dock", it.sec.as_str()));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), cid, true)
                .show_header(ui, |ui| {
                    ui.label(RichText::new(dock_title(&it.sec)).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").on_hover_text("Hide this panel").clicked() {
                            cmd = Some((i, 3));
                        }
                        if ui.small_button("Float").on_hover_text("Pop this panel out into its own window").clicked() {
                            cmd = Some((i, 2));
                        }
                        if ui.small_button("v").on_hover_text("Move down").clicked() {
                            cmd = Some((i, 1));
                        }
                        if ui.small_button("^").on_hover_text("Move up").clicked() {
                            cmd = Some((i, 0));
                        }
                    });
                })
                .body(|ui| {
                    if editing { self.person_section(ui, act, id, &it.sec) } else { self.person_read(ui, act, id, &it.sec) }
                });
            ui.add_space(4.0);
        }
        if let Some((i, c)) = cmd {
            let d = &mut self.settings.dock;
            match c {
                0 if i > 0 => d.swap(i, i - 1),
                1 if i + 1 < d.len() => d.swap(i, i + 1),
                2 => d[i].float = true,
                3 => d[i].on = false,
                _ => {}
            }
            save_settings(&self.settings);
        }
    }

pub(crate)     fn person_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let id = self.sel.unwrap();
        let editing = matches!(&self.editing, Some(e) if e.id == id);
        self.nav_bar(ui, act);
        ui.add_space(2.0);
        if editing {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Editing. Changes are only kept when you press Save.").strong());
                    if ui.button("Save").clicked() {
                        *act = Some(Action::EditSave);
                    }
                    if ui.button("Cancel").on_hover_text("Put this person back exactly as they were before you pressed Edit").clicked() {
                        *act = Some(Action::EditCancel);
                    }
                });
            });
        } else {
            ui.horizontal(|ui| {
                if ui.button("Edit").on_hover_text("Unlock this page to make changes").clicked() {
                    *act = Some(Action::EditStart);
                }
                ui.label(RichText::new("Locked. Press Edit to change anything.").weak());
            });
        }
        self.problem_flags(ui, id);
        ui.add_space(4.0);
        self.dock_bar(ui);
        ui.add_space(4.0);
        let docked = self.settings.dock.iter().any(|d| d.on && !d.float);
        let wide = ui.available_width() > 820.0;
        if wide && docked {
            egui::SidePanel::right("person_dock").resizable(true).default_width(self.settings.dock_w).width_range(280.0..=720.0).show_inside(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("person_dock_scroll").auto_shrink([false, false]).show(ui, |ui| {
                    self.dock_panels(ui, act, id, editing);
                });
            });
        }
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("person_main").auto_shrink([false, false]).show(ui, |ui| {
                if editing {
                    self.person_section(ui, act, id, "facts");
                    self.person_section(ui, act, id, "timeline");
                } else {
                    self.person_read(ui, act, id, "facts");
                    self.person_read(ui, act, id, "timeline");
                }
                if !wide && docked {
                    ui.add_space(10.0);
                    self.dock_panels(ui, act, id, editing);
                }
            });
        });
        // panels popped out into their own windows
        let ctx = ui.ctx().clone();
        let items = self.settings.dock.clone();
        for (i, it) in items.iter().enumerate() {
            if !(it.on && it.float) {
                continue;
            }
            let mut open = true;
            let mut dock_back = false;
            egui::Window::new(dock_title(&it.sec)).id(egui::Id::new(("float", it.sec.as_str()))).open(&mut open).default_size([420.0, 420.0]).show(&ctx, |ui| {
                if ui.small_button("Dock back on the right").clicked() {
                    dock_back = true;
                }
                egui::ScrollArea::vertical().id_salt(("float_scroll", it.sec.as_str())).show(ui, |ui| {
                    if editing { self.person_section(ui, act, id, &it.sec) } else { self.person_read(ui, act, id, &it.sec) }
                });
            });
            if dock_back {
                self.settings.dock[i].float = false;
                save_settings(&self.settings);
            } else if !open {
                self.settings.dock[i].on = false;
                self.settings.dock[i].float = false;
                save_settings(&self.settings);
            }
        }
    }

    /// Row of buttons that show or hide each panel.
pub(crate)     fn dock_bar(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let mut reset = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Panels").small().weak());
            for it in self.settings.dock.iter_mut() {
                if ui.selectable_label(it.on, dock_title(&it.sec)).on_hover_text("Show or hide this panel on the right").clicked() {
                    it.on = !it.on;
                    if it.on {
                        it.float = false;
                    }
                    changed = true;
                }
            }
            let mut apply: Option<Vec<DockItem>> = None;
            let mut apply_w: Option<f32> = None;
            ui.menu_button("Layouts", |ui| {
                ui.label(RichText::new("Ready-made").small().weak());
                for (name, secs) in LAYOUT_PRESETS {
                    if ui.button(*name).clicked() {
                        let mut d: Vec<DockItem> = secs.iter().map(|s| DockItem { sec: s.to_string(), on: true, float: false }).collect();
                        for x in default_dock() {
                            if !d.iter().any(|y| y.sec == x.sec) {
                                d.push(DockItem { on: false, ..x });
                            }
                        }
                        apply = Some(d);
                        ui.close_menu();
                    }
                }
                let mut del: Option<usize> = None;
                if !self.settings.layouts.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Yours").small().weak());
                    for (i, l) in self.settings.layouts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.button(l.name.as_str()).clicked() {
                                apply = Some(l.dock.clone());
                                apply_w = Some(l.width);
                                ui.close_menu();
                            }
                            if ui.small_button("x").on_hover_text("Forget this layout").clicked() {
                                del = Some(i);
                            }
                        });
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.x.layout_name).hint_text("Name for this layout").desired_width(150.0));
                    if ui.add_enabled(!self.x.layout_name.trim().is_empty(), egui::Button::new("Save current")).clicked() {
                        let name = self.x.layout_name.trim().to_string();
                        self.settings.layouts.retain(|l| l.name != name);
                        self.settings.layouts.push(Layout { name, dock: self.settings.dock.clone(), width: self.settings.dock_w });
                        self.x.layout_name.clear();
                        changed = true;
                        ui.close_menu();
                    }
                });
                if let Some(i) = del {
                    self.settings.layouts.remove(i);
                    changed = true;
                }
            });
            if let Some(d) = apply {
                self.settings.dock = d;
                if let Some(w) = apply_w {
                    self.settings.dock_w = w;
                }
                changed = true;
            }
            if ui.small_button("Reset layout").clicked() {
                reset = true;
            }
        });
        if reset {
            self.settings.dock = default_dock();
            self.settings.dock_w = 380.0;
            changed = true;
        }
        if changed {
            save_settings(&self.settings);
        }
    }
}

/// One line of the LDS ordinances panel: which ordinance, and where its event is kept.
#[derive(Clone)]
pub(crate) struct LdsRow {
    pub(crate) label: String,
    /// The event kind ("LDS baptism", "LDS child sealing", "LDS spouse sealing"...).
    pub(crate) kind: &'static str,
    /// For a sealing: the family (the parents' family, or the couple's family).
    pub(crate) fam: Option<u32>,
    /// Where the event is now: its place in the person's events, or the family's for a spouse sealing.
    pub(crate) at: Option<usize>,
    pub(crate) event: Option<Event>,
}

/// The family a "sealed to parents" event is for, from its FAMC line.
pub(crate) fn sealing_family(e: &Event) -> Option<u32> {
    e.extra.iter().flat_map(|r| r.text.lines()).find_map(|l| {
        let rest = l.trim().strip_prefix("0 FAMC @F")?;
        rest.trim_end_matches('@').parse().ok()
    })
}

impl App {
    /// The ordinances shown for a person: the four personal ones, a sealing to each set of
    /// parents, and a sealing to each spouse.
    pub(crate) fn lds_rows(&self, id: u32) -> Vec<LdsRow> {
        let Some(p) = self.db.people.get(&id) else { return vec![] };
        let mut rows = vec![];
        for (kind, label) in [("LDS baptism", "Baptism"), ("LDS confirmation", "Confirmation"), ("LDS initiatory", "Initiatory"), ("LDS endowment", "Endowment")] {
            let at = p.events.iter().position(|e| e.kind == kind);
            rows.push(LdsRow { label: label.into(), kind, fam: None, at, event: at.map(|i| p.events[i].clone()) });
        }
        let name = |x: Option<u32>| x.and_then(|x| self.db.people.get(&x)).map(display_name);
        let parents = self.db.parent_families(id);
        let sealings: Vec<usize> = p.events.iter().enumerate().filter(|(_, e)| e.kind == "LDS child sealing").map(|(i, _)| i).collect();
        for f in &parents {
            let fam = &self.db.fams[f];
            let who: Vec<String> = [name(fam.h), name(fam.w)].into_iter().flatten().collect();
            let label = if who.is_empty() { "Sealed to parents".to_string() } else { format!("Sealed to parents: {}", who.join(" and ")) };
            // The sealing that names this family, or the only unnamed one when there is one set of parents.
            let at = sealings
                .iter()
                .copied()
                .find(|i| sealing_family(&p.events[*i]) == Some(*f))
                .or_else(|| if parents.len() == 1 { sealings.iter().copied().find(|i| sealing_family(&p.events[*i]).is_none()) } else { None });
            rows.push(LdsRow { label, kind: "LDS child sealing", fam: Some(*f), at, event: at.map(|i| p.events[i].clone()) });
        }
        if parents.is_empty() {
            let at = sealings.first().copied();
            rows.push(LdsRow { label: "Sealed to parents".into(), kind: "LDS child sealing", fam: None, at, event: at.map(|i| p.events[i].clone()) });
        }
        for f in self.db.fams_of(id) {
            let fam = &self.db.fams[&f];
            let spouse = if fam.h == Some(id) { fam.w } else { fam.h };
            let label = match name(spouse) {
                Some(n) => format!("Sealed to spouse: {n}"),
                None => "Sealed to spouse".to_string(),
            };
            let at = fam.events.iter().position(|e| e.kind == "LDS spouse sealing");
            rows.push(LdsRow { label, kind: "LDS spouse sealing", fam: Some(f), at, event: at.map(|i| fam.events[i].clone()) });
        }
        rows
    }

    pub(crate) fn lds_event_mut(&mut self, id: u32, r: &LdsRow) -> Option<&mut Event> {
        let i = r.at?;
        if r.kind == "LDS spouse sealing" {
            self.db.fams.get_mut(&r.fam?)?.events.get_mut(i)
        } else {
            self.db.people.get_mut(&id)?.events.get_mut(i)
        }
    }

    pub(crate) fn lds_add(&mut self, id: u32, r: &LdsRow) {
        let mut e = Event { kind: r.kind.to_string(), ..Default::default() };
        if r.kind == "LDS spouse sealing" {
            if let Some(f) = r.fam.and_then(|f| self.db.fams.get_mut(&f)) {
                f.events.push(e);
            }
            return;
        }
        if r.kind == "LDS child sealing" {
            if let Some(f) = r.fam {
                e.extra.push(Raw { at: String::new(), text: format!("0 FAMC @F{f}@\n") });
            }
        }
        if let Some(p) = self.db.people.get_mut(&id) {
            p.events.push(e);
        }
    }

    pub(crate) fn lds_remove(&mut self, id: u32, r: &LdsRow) {
        let Some(i) = r.at else { return };
        if r.kind == "LDS spouse sealing" {
            if let Some(f) = r.fam.and_then(|f| self.db.fams.get_mut(&f)) {
                if i < f.events.len() {
                    f.events.remove(i);
                }
            }
        } else if let Some(p) = self.db.people.get_mut(&id) {
            if i < p.events.len() {
                p.events.remove(i);
            }
        }
    }
}
