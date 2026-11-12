//! The frame loop: draws the whole program each frame.

use super::*;


impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut act: Option<Action> = None;
        self.type_pending_letter(ctx);
        NAME_STYLE.store(self.settings.name_style, Ordering::Relaxed);
        DATE_STYLE.store(self.settings.date_style, Ordering::Relaxed);
        PAPER_A4.store(self.settings.paper_a4, Ordering::Relaxed);
        MAP_STYLE.store(self.settings.map_style, Ordering::Relaxed);
        let rgb = |c: [u8; 3]| (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
        APP_BG.store(rgb(self.settings.bg), Ordering::Relaxed);
        APP_ACCENT.store(rgb(self.settings.accent), Ordering::Relaxed);
        let scale = self.settings.ui_scale.clamp(0.7, 2.0);
        if (ctx.zoom_factor() - scale).abs() > 0.001 {
            ctx.set_zoom_factor(scale);
        }
        // egui's default (0.3 s) is quicker than most people double-click, especially on a Mac.
        ctx.options_mut(|o| {
            if o.input_options.max_double_click_delay < 0.5 {
                o.input_options.max_double_click_delay = 0.5;
            }
        });

        let (has_f, has_m) = self.sel.map(|s| self.db.parents(s)).map(|(f, m)| (f.is_some(), m.is_some())).unwrap_or((false, false));
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(format!("Family trees... (open: {})", tree_title(&self.settings.current_tree))).on_hover_text("Switch between separate trees, or start a new one").clicked() {
                        self.x.trees_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Open Family Tree Archive...").on_hover_text("Restore everything from an archive file (tree, photos, documents, stories)").clicked() {
                        self.open_archive_pick();
                        ui.close_menu();
                    }
                    if ui.button("Save Family Tree Archive...").on_hover_text("One file with your whole tree, every photo, document, story and recording, and a GEDCOM copy").clicked() {
                        self.save_archive_pick();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Import tree...").clicked() {
                        self.imp_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Export GEDCOM (everything)...").clicked() {
                        self.export(false);
                        ui.close_menu();
                    }
                    if ui.button("Export GEDCOM (living people private)...").clicked() {
                        self.export(true);
                        ui.close_menu();
                    }
                    {
                        let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                        if ui.button(if fs { "Leave full screen        F11" } else { "Full screen        F11 (Mac: Ctrl+Cmd+F)" }).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.add_enabled(self.sel.is_some(), egui::Button::new("Print pedigree chart...")).clicked() {
                        act = Some(Action::PrintPedigree);
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.sel.is_some(), egui::Button::new("Print family group sheet...")).clicked() {
                        act = Some(Action::PrintFamily);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo")).clicked() {
                        self.undo_action();
                        ui.close_menu();
                    }
                    if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo")).clicked() {
                        self.redo_action();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Find and replace in notes...").on_hover_text("Change words in every note at once, seeing each one first").clicked() {
                        self.x.nfr_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Special letters (é, ñ, ß...)...").on_hover_text("Accented letters and symbols to put into names and places").clicked() {
                        self.x.letters_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Edit individual").clicked() {
                        self.view = View::Person;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Add", |ui| {
                    if ui.button("Individual...").clicked() {
                        act = Some(Action::Add(Kind::Person));
                        ui.close_menu();
                    }
                    if ui.button("Spouse...").clicked() {
                        act = Some(Action::Add(Kind::Spouse));
                        ui.close_menu();
                    }
                    if ui.add_enabled(!has_f, egui::Button::new("Father...")).clicked() {
                        act = Some(Action::Add(Kind::Father));
                        ui.close_menu();
                    }
                    if ui.add_enabled(!has_m, egui::Button::new("Mother...")).clicked() {
                        act = Some(Action::Add(Kind::Mother));
                        ui.close_menu();
                    }
                    if ui.button("Child...").clicked() {
                        let fid = self.sel.and_then(|s| self.db.fams_of(s).first().copied());
                        act = Some(Action::Add(Kind::Child(fid)));
                        ui.close_menu();
                    }
                    if ui.button("Another set of parents (adoptive, step...)...").clicked() {
                        act = Some(Action::Add(Kind::NewParent(true)));
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Go to home person").clicked() {
                        act = Some(Action::GoHome);
                        ui.close_menu();
                    }
                    if ui.button("Fit chart to window").clicked() {
                        act = Some(Action::Fit);
                        ui.close_menu();
                    }
                    ui.separator();
                    for (v, l) in [(View::Family, "Family"), (View::Pedigree, "Pedigree"), (View::Descendants, "Descendants"), (View::Person, "Individual"), (View::Gallery, "Gallery"), (View::Memories, "Memories"), (View::Charts, "Charts"), (View::Table, "People"), (View::Map, "Map"), (View::Dna, "DNA")] {
                        if ui.button(l).clicked() {
                            self.view = v;
                            ui.close_menu();
                        }
                    }
                    ui.menu_button("Generations", |ui| {
                        for n in 3..=8usize {
                            if ui.selectable_label(self.settings.gens == n, format!("{} generations", n)).clicked() {
                                act = Some(Action::SetGens(n));
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    let mut changed = false;
                    ui.menu_button("Colours", |ui| {
                        ui.menu_button("Colour schemes", |ui| {
                            egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                                if scheme_list(ui, &mut self.settings) {
                                    changed = true;
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.separator();
                        ui.label(RichText::new("Accent colour").strong());
                        if swatch_grid(ui, &PRESETS, &mut self.settings.accent, false) {
                            changed = true;
                        }
                        if self.settings.dark {
                            ui.separator();
                            ui.label(RichText::new("Dark background").strong());
                            if swatch_grid(ui, &BG_PRESETS, &mut self.settings.bg, true) {
                                changed = true;
                            }
                        }
                        ui.separator();
                        if ui.button("More: theme and your own colours...").clicked() {
                            self.show_look = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Text size", |ui| {
                        for (v, l) in [(0.85f32, "Small"), (1.0, "Normal"), (1.15, "Large"), (1.3, "Larger"), (1.5, "Largest")] {
                            if ui.selectable_label((self.settings.ui_scale - v).abs() < 0.01, l).clicked() {
                                self.settings.ui_scale = v;
                                changed = true;
                            }
                        }
                        ui.label(RichText::new("Makes all text and buttons bigger or smaller.").small().weak());
                    });
                    ui.menu_button("Names", |ui| {
                        for (v, l) in [(0u8, "John Henry Smith"), (1, "SMITH, John Henry"), (2, "Smith, John Henry")] {
                            if ui.selectable_label(self.settings.name_style == v, l).clicked() {
                                self.settings.name_style = v;
                                changed = true;
                            }
                        }
                    });
                    ui.menu_button("Dates", |ui| {
                        for (v, l) in [(0u8, "As typed"), (1, "12 Mar 1921"), (2, "March 12, 1921"), (3, "1921-03-12")] {
                            if ui.selectable_label(self.settings.date_style == v, l).clicked() {
                                self.settings.date_style = v;
                                changed = true;
                            }
                        }
                        ui.label(RichText::new("Only changes how dates look. What you typed is kept.").small().weak());
                    });
                    ui.menu_button("Profile photo size", |ui| {
                        for (v, l) in [(0u8, "Small"), (1, "Normal"), (2, "Large"), (3, "Extra large")] {
                            if ui.selectable_label(self.settings.photo_size == v, l).clicked() {
                                self.settings.photo_size = v;
                                changed = true;
                            }
                        }
                        ui.label(RichText::new("The photo at the top of the Individual page.").small().weak());
                    });
                    ui.menu_button("Chart boxes", |ui| {
                        changed |= ui.checkbox(&mut self.settings.box_photos, "Show photos").changed();
                        changed |= ui.checkbox(&mut self.settings.box_places, "Show birthplace").changed();
                        changed |= ui.checkbox(&mut self.settings.box_ids, "Show person ID").changed();
                    });
                    {
                        let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                        if ui.button(if fs { "Leave full screen" } else { "Full screen" }).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                            ui.close_menu();
                        }
                    }
                    if changed {
                        save_settings(&self.settings);
                        apply_theme(ctx, &self.settings);
                    }
                    ui.separator();
                    if ui.button("Appearance...").clicked() {
                        self.show_look = true;
                        ui.close_menu();
                    }
                    if ui.button("Where my tree is kept...").clicked() {
                        self.show_loc = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Search", |ui| {
                    if ui.button("Find individual...").clicked() {
                        self.find_open = true;
                        self.x.find_mode = 0;
                        ui.close_menu();
                    }
                    if ui.button("Find by ID number...").clicked() {
                        self.find_open = true;
                        self.x.find_mode = 1;
                        ui.close_menu();
                    }
                    if ui.button("Find a couple...").clicked() {
                        self.find_open = true;
                        self.x.find_mode = 2;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Groups (focus and filter)...").on_hover_text("Pick out a set of people by relationship or by what is recorded, then show, print, export or split them off").clicked() {
                        self.x.groups_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Relationship finder...").clicked() {
                        self.rel_open = true;
                        if self.rel_a.is_none() {
                            self.rel_a = self.sel;
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("Reports", |ui| {
                    if ui.checkbox(&mut self.settings.hide_living, "Keep living people private").on_hover_text("Printed charts, sheets and reports show living people as \"Living\" with no dates or places").changed() {
                        save_settings(&self.settings);
                    }
                    ui.menu_button(if self.settings.paper_a4 { "Paper: A4" } else { "Paper: US Letter" }, |ui| {
                        for (v, l) in [(false, "US Letter (8.5 x 11 in)"), (true, "A4 (210 x 297 mm)")] {
                            if ui.selectable_label(self.settings.paper_a4 == v, l).clicked() {
                                self.settings.paper_a4 = v;
                                PAPER_A4.store(v, Ordering::Relaxed);
                                save_settings(&self.settings);
                                ui.close_menu();
                            }
                        }
                        ui.label(RichText::new("Also choose the same paper in the print window.").small().weak());
                    });
                    ui.separator();
                    if ui.button("Report designer (make your own)...").clicked() {
                        self.x.designer_open = true;
                        ui.close_menu();
                    }
                    ui.menu_button("Forms (blank, to fill in by hand)", |ui| {
                        if ui.button("Family group sheet").clicked() {
                            self.open_print(forms::blank_family_group());
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart, 4 generations").clicked() {
                            self.open_print(forms::blank_pedigree(4));
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart, 5 generations").clicked() {
                            self.open_print(forms::blank_pedigree(5));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Family group sheet with LDS ordinances").clicked() {
                            self.open_print(forms::blank_family_group_lds());
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart with LDS ordinances, 4 generations").clicked() {
                            self.open_print(forms::blank_pedigree_lds(4));
                            ui.close_menu();
                        }
                        if ui.button("Pedigree chart with LDS ordinances, 5 generations").clicked() {
                            self.open_print(forms::blank_pedigree_lds(5));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Census forms...").on_hover_text("United States 1790-1950, Canada 1851-1931, United Kingdom 1841-1921").clicked() {
                            self.x.census.open = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Lists", |ui| {
                        for (i, (l, tip)) in lists::LISTS.iter().enumerate() {
                            if ui.button(*l).on_hover_text(*tip).clicked() {
                                let html = self.list_html(i);
                                self.open_print(html);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    let has = self.sel.is_some();
                    for (label, k) in [
                        ("Ancestor list...", Action::ReportAncestors),
                        ("Descendant list...", Action::ReportDescendants),
                        ("Descendant chart...", Action::PrintDescendants),
                        ("Birthdays and anniversaries...", Action::ReportBirthdays),
                        ("Family book...", Action::ReportBook),
                        ("Ancestor book (with index)...", Action::ReportAncestorBook),
                        ("Descendant book (with index)...", Action::ReportDescendantBook),
                        ("Source list (bibliography)...", Action::ReportSources),
                    ] {
                        if ui.add_enabled(has, egui::Button::new(label)).clicked() {
                            act = Some(k);
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Sources and repositories...").clicked() {
                        self.src_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Date calculator...").on_hover_text("Age on a date, birth year from an age, days between dates, day of the week").clicked() {
                        self.x.calc_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Soundex...").on_hover_text("The sound-alike code of a surname, and the surnames in this tree that share it").clicked() {
                        self.x.soundex_open = true;
                        if self.x.soundex_q.is_empty() {
                            if let Some(p) = self.sel.and_then(|i| self.db.people.get(&i)) {
                                self.x.soundex_q = p.surname.clone();
                            }
                        }
                        ui.close_menu();
                    }
                    let who = self.sel.and_then(|i| self.db.people.get(&i)).cloned();
                    ui.add_enabled_ui(who.is_some(), |ui| {
                        ui.menu_button("Search websites for this person", |ui| {
                            if let Some(p) = &who {
                                ui.label(RichText::new(format!("{} — opens in your web browser", display_name(p))).small().weak());
                                for (site, url) in tools::search_links(p) {
                                    if ui.button(site).clicked() {
                                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                        ui.close_menu();
                                    }
                                }
                            }
                        });
                    });
                    ui.separator();
                    if ui.button("Home person...").on_hover_text("Choose who the Home button and Home key go back to").clicked() {
                        self.home_pick = Some(String::new());
                        ui.close_menu();
                    }
                    if ui.add_enabled(!self.db.people.is_empty(), egui::Button::new("Renumber people...")).on_hover_text("Give everyone new ID numbers 1, 2, 3..., starting with the home person").clicked() {
                        self.x.renum_open = true;
                        self.x.renum_preview = None;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add_enabled(backup_path().exists(), egui::Button::new("Restore previous tree")).clicked() {
                        self.request_wipe(Wipe::Restore);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Research to-do list...").clicked() {
                        self.x.todo_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Research journal...").clicked() {
                        self.x.journal_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Family statistics...").clicked() {
                        self.x.stats_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Historical events...").clicked() {
                        self.x.hist_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Places...").clicked() {
                        self.x.places_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Tombstone calculator...").on_hover_text("Work out a birth date from the death date and the age on a gravestone").clicked() {
                        self.open_tombstone(self.sel);
                        ui.close_menu();
                    }
                    if ui.button("History of changes...").clicked() {
                        self.x.history_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Saved versions (daily copies)...").clicked() {
                        self.x.ver_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Data checker...").clicked() {
                        self.check_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Find duplicates...").clicked() {
                        self.dups = self.db.duplicate_pairs().into_iter().filter(|x| !self.dup_ignore.contains(&(x.0, x.1))).collect();
                        self.dup_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Backup settings...").clicked() {
                        self.x.backup_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Restore from a dated backup...").clicked() {
                        self.restore_from_file();
                        ui.close_menu();
                    }
                    if !is_portable() && self.settings.current_tree.is_empty() && ui.button("Move my data folder...").on_hover_text("Keep your tree somewhere other than Documents").clicked() {
                        self.move_data = true;
                        ui.close_menu();
                    }
                    if ui.button("Open backups folder").clicked() {
                        open_folder(&data_dir().join("backups").join("x"));
                        ui.close_menu();
                    }
                    if ui.button("Back up now (data and photos)").clicked() {
                        self.backup_now(true);
                        ui.close_menu();
                    }
                    if ui.button("Check backups...").clicked() {
                        self.run_verify();
                        ui.close_menu();
                    }
                    if ui.button("Restore missing photos and recordings").clicked() {
                        self.spawn_media_job(true);
                        ui.close_menu();
                    }
                    ui.separator();
                    if is_portable() {
                        ui.add_enabled(false, egui::Button::new("Portable mode is on"));
                    } else if ui.button("Make portable (keep data next to the app)").clicked() {
                        self.make_portable();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About Gilded Roots").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.x.practice.is_none(), egui::Button::new("Explore the sample family (practice)")).clicked() {
                        self.start_practice();
                        ui.close_menu();
                    }
                    if ui.button("Open-source licences").clicked() {
                        self.x.licenses_open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("User guide        F1 (Mac: Cmd+?)").clicked() {
                        self.help_open = true;
                        ui.close_menu();
                    }
                    ui.menu_button("Keyboard shortcuts", |ui| {
                        for (k, d) in [
                            ("F1  (Mac: Cmd+?)", "User guide"),
                            ("F2", "Edit the selected person"),
                            ("F3", "Find a person"),
                            ("F4  or  Ctrl/Cmd+K", "Quick jump"),
                            ("F5", "Fit the chart to the window"),
                            ("F6", "Next tab"),
                            ("F7", "Data checker"),
                            ("F8", "Research to-do list"),
                            ("F9", "Places"),
                            ("F11  (Mac: Ctrl+Cmd+F)", "Full screen"),
                            ("F12", "History of changes"),
                            ("Arrow keys", "Move around the chart"),
                            ("Home", "Go to the home person"),
                            ("Ctrl/Cmd+1..9", "Family, Pedigree, Descendants, Individual, Gallery, Charts, People, Map, Memories"),
                            ("Ctrl/Cmd + / -", "Zoom the chart"),
                            ("H", "Pedigree and descendants: switch between Select and the hand"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.add_sized([170.0, 18.0], egui::Label::new(RichText::new(k).strong()));
                                ui.label(d);
                            });
                        }
                        ui.label(RichText::new("On a Mac, hold fn to use the F keys.").small().weak());
                    });
                    if ui.button("Open data folder").clicked() {
                        open_folder(&store_path());
                        ui.close_menu();
                    }
                });
            });
        });

        if self.x.practice.is_some() {
            let mut leave = false;
            egui::TopBottomPanel::top("practice").frame(egui::Frame::default().fill(Color32::from_rgb(196, 152, 40)).inner_margin(egui::Margin::symmetric(10.0, 6.0))).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Practice mode: you are exploring the made-up sample family. Nothing here is saved, and your own tree is safe.").color(Color32::BLACK).strong());
                    if ui.button("Back to my tree").clicked() {
                        leave = true;
                    }
                });
            });
            if leave {
                self.end_practice();
            }
        }
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (v, l) in [(View::Family, "Family"), (View::Pedigree, "Pedigree"), (View::Descendants, "Descendants"), (View::Person, "Individual"), (View::Gallery, "Gallery"), (View::Memories, "Memories"), (View::Charts, "Charts"), (View::Table, "People"), (View::Map, "Map"), (View::Dna, "DNA")] {
                    let mut t = RichText::new(l).size(16.0);
                    if self.view == v {
                        t = t.strong();
                    }
                    if ui.selectable_label(self.view == v, t).clicked() {
                        self.view = v;
                    }
                }
            });
        });

        if let Some(err) = self.save_error.clone() {
            egui::TopBottomPanel::top("save_err").show(ctx, |ui| {
                ui.colored_label(
                    Color32::from_rgb(225, 70, 70),
                    format!("Your latest changes could not be saved: {}. Your tree is still open in the app and it will keep trying. Free up space or fix the folder, and use File > Export GEDCOM now to keep a copy.", err),
                );
            });
        }
        let mut dismiss = false;
        if let Some(n) = self.notice.clone() {
            egui::TopBottomPanel::top("notice").show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(Color32::from_rgb(230, 170, 40), n.as_str());
                    if ui.button("Dismiss").clicked() {
                        dismiss = true;
                    }
                });
            });
        }
        if dismiss {
            self.notice = None;
        }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let msg = if self.status.is_empty() {
                    "For help, press F1. Double-click a person to open their page."
                } else {
                    self.status.as_str()
                };
                ui.label(msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.save_error.is_some() {
                        ui.colored_label(Color32::from_rgb(225, 70, 70), "NOT SAVED");
                    } else if self.dirty {
                        ui.label("Saving...");
                    } else if let Some(t) = self.last_saved {
                        ui.label(format!("Saved {}", ago(t.elapsed())));
                    } else {
                        ui.label("All changes saved");
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let ok = self.sel.map(|s| self.db.people.contains_key(&s)).unwrap_or(false);
            if !ok {
                self.welcome(ui, &mut act);
                return;
            }
            if self.sel.is_some() {
                self.trail_bar(ui, &mut act);
            }
            match self.view {
                View::Pedigree => self.pedigree(ui, &mut act),
                View::Family => self.family_view(ui, &mut act),
                View::Descendants => self.descendants(ui, &mut act),
                View::Person => self.person_view(ui, &mut act),
                View::Gallery => self.gallery_view(ui, &mut act),
                View::Charts => self.charts_view(ui, &mut act),
                View::Table => self.table_view(ui, &mut act),
                View::Map => self.map_view(ui, &mut act),
                View::Memories => self.memories_view(ui, &mut act),
                View::Dna => self.dna_view(ui, &mut act),
            }
        });

        // Leaving a person (or the Individual tab) while editing: keep, discard or stay.
        if let Some(e) = &self.editing {
            let leaving = self.view != View::Person || self.sel != Some(e.id);
            if leaving {
                let changed = serde_json::to_string(&self.db).map(|c| c != e.snapshot).unwrap_or(true);
                if !changed {
                    self.editing = None;
                } else {
                    let name = self.db.people.get(&e.id).map(display_name).unwrap_or_default();
                    let mut choice = 0u8;
                    egui::Area::new(egui::Id::new("leave_edit_block")).order(egui::Order::Foreground).fixed_pos(pos2(0.0, 0.0)).show(ctx, |ui| {
                        let r = ctx.screen_rect();
                        ui.allocate_rect(r, Sense::click_and_drag());
                        ui.painter().rect_filled(r, 0.0_f32, Color32::from_black_alpha(110));
                    });
                    egui::Window::new("Unsaved changes").collapsible(false).resizable(false).order(egui::Order::Tooltip).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                        ui.set_max_width(380.0);
                        ui.label(format!("You have changed {} and not pressed Save.", name));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                choice = 1;
                            }
                            if ui.button("Discard changes").clicked() {
                                choice = 2;
                            }
                            if ui.button("Keep editing").clicked() {
                                choice = 3;
                            }
                        });
                    });
                    match choice {
                        1 => {
                            self.editing = None;
                            self.dirty = true;
                        }
                        2 => {
                            if let Some(e) = self.editing.take() {
                                self.load_snapshot(e.snapshot);
                                self.dirty = true;
                            }
                        }
                        3 => {
                            if let Some(e) = &self.editing {
                                self.sel = Some(e.id);
                            }
                            self.view = View::Person;
                        }
                        _ => {}
                    }
                }
            }
        }

        self.global_keys(ctx, &mut act);
        if self.help_open {
            let mut open = true;
            let mut find = self.help_find.clone();
            egui::Window::new("User guide").open(&mut open).default_size([640.0, 600.0]).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search");
                    ui.add(egui::TextEdit::singleline(&mut find).hint_text("e.g. merge, sources, home person").desired_width(260.0));
                    if !find.is_empty() && ui.small_button("Clear").clicked() {
                        find.clear();
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| help_text(ui, &find));
            });
            self.help_find = find;
            self.help_open = open;
        }

        self.jump_window(ctx, &mut act);
        if let Some((p, t)) = self.x.pdf_req.take() {
            match pdfview::PdfView::open(&p, &t) {
                Ok(v) => self.x.pdf = Some(v),
                Err(e) => {
                    self.status = format!("{} Opening it in your PDF app instead.", e);
                    open_path(&p);
                }
            }
        }
        if let Some(v) = self.x.pdf.as_mut() {
            if !v.show(ctx) {
                self.x.pdf = None;
            }
        }
        self.todo_window(ctx, &mut act);
        self.places_window(ctx, &mut act);
        self.tombstone_window(ctx);
        self.census_window(ctx);
        self.history_window(ctx);
        self.archive_windows(ctx);
        self.journal_window(ctx, &mut act);
        self.history_events_window(ctx);
        self.stats_window(ctx, &mut act);
        self.trees_window(ctx);
        self.designer_window(ctx);
        self.story_window(ctx);
        self.versions_window(ctx);
        self.renumber_window(ctx);
        if self.home_pick.is_some() {
            let mut open = true;
            let mut chosen: Option<u32> = None;
            let cur = self.db.home.filter(|h| self.db.people.contains_key(h)).map(|h| self.db.label(h));
            let sel = self.sel;
            let sel_label = sel.map(|i| self.db.label(i));
            let q = self.home_pick.clone().unwrap_or_default();
            let ql = q.trim().to_lowercase();
            let mut hits: Vec<(u32, String)> = Vec::new();
            if !ql.is_empty() {
                for (i, p) in &self.db.people {
                    if p.name.to_lowercase().contains(&ql) || i.to_string() == ql {
                        hits.push((*i, self.db.label(*i)));
                        if hits.len() >= 200 {
                            break;
                        }
                    }
                }
                hits.sort_by(|a, b| a.1.cmp(&b.1));
            }
            let mut q2 = q.clone();
            egui::Window::new("Home person").open(&mut open).collapsible(false).default_width(420.0).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("The Home button and the Home key always come back to this person, and the tree opens on them.");
                ui.add_space(4.0);
                ui.label(RichText::new(format!("Current home person: {}", cur.clone().unwrap_or_else(|| "none chosen".into()))).strong());
                ui.add_space(6.0);
                if let (Some(i), Some(l)) = (sel, sel_label.clone()) {
                    if ui.button(format!("Use the person selected now: {}", l)).clicked() {
                        chosen = Some(i);
                    }
                    ui.add_space(4.0);
                }
                ui.add(egui::TextEdit::singleline(&mut q2).hint_text("Type a name or ID to search").desired_width(f32::INFINITY));
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (i, l) in &hits {
                        if ui.selectable_label(false, l.as_str()).clicked() {
                            chosen = Some(*i);
                        }
                    }
                    if !ql.is_empty() && hits.is_empty() {
                        ui.weak("No one matches.");
                    }
                });
            });
            self.home_pick = Some(q2);
            if let Some(i) = chosen {
                self.db.home = Some(i);
                self.dirty = true;
                self.status = format!("Home person is now {}.", self.db.label(i));
                self.home_pick = None;
            } else if !open {
                self.home_pick = None;
            }
        }

        if self.quick.is_some() {
            let mut save = false;
            let mut close = false;
            let mut full: Option<u32> = None;
            if let Some(q) = self.quick.as_mut() {
                egui::Window::new("Quick edit").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                    egui::Grid::new("quick_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("Given name(s)");
                        ui.add(egui::TextEdit::singleline(&mut q.given).desired_width(260.0));
                        ui.end_row();
                        ui.label("Surname");
                        ui.add(egui::TextEdit::singleline(&mut q.surname).desired_width(260.0));
                        ui.end_row();
                        ui.label("Sex");
                        let st = match q.sex.as_str() {
                            "M" => "Male",
                            "F" => "Female",
                            _ => "Unknown",
                        };
                        egui::ComboBox::from_id_salt("qsex").selected_text(st).show_ui(ui, |ui| {
                            ui.selectable_value(&mut q.sex, "M".to_string(), "Male");
                            ui.selectable_value(&mut q.sex, "F".to_string(), "Female");
                            ui.selectable_value(&mut q.sex, String::new(), "Unknown");
                        });
                        ui.end_row();
                        ui.label("Born");
                        ditto(ui, &mut q.bd, |t| t.desired_width(260.0), &mut self.x.ditto_date);
                        ui.end_row();
                        ui.label("Birthplace");
                        ditto(ui, &mut q.bp, |t| t.desired_width(260.0), &mut self.x.ditto_place);
                        ui.end_row();
                        ui.label("Died");
                        ditto(ui, &mut q.dd, |t| t.desired_width(260.0), &mut self.x.ditto_date);
                        ui.end_row();
                        ui.label("Place of death");
                        ditto(ui, &mut q.dp, |t| t.desired_width(260.0), &mut self.x.ditto_place);
                        ui.end_row();
                        ui.label("Living");
                        ui.checkbox(&mut q.living, "Living (kept private when printing or exporting)");
                        ui.end_row();
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            save = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                        if ui.button("Open full page").clicked() {
                            full = Some(q.id);
                            close = true;
                        }
                    });
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        close = true;
                    }
                });
            }
            if save {
                if let Some(q) = self.quick.take() {
                    self.apply_quick(&q);
                    self.dirty = true;
                    self.status = "Saved.".to_string();
                }
            } else if close {
                self.quick = None;
            }
            if let Some(i) = full {
                self.goto(i);
                self.view = View::Person;
            }
        }

        if self.find_open {
            let mut open = true;
            let mut couple: Option<(u32, u32)> = None;
            egui::Window::new("Find Individual").open(&mut open).default_width(320.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.x.find_mode, 0, "Name or place");
                    ui.selectable_value(&mut self.x.find_mode, 1, "ID number");
                    ui.selectable_value(&mut self.x.find_mode, 2, "A couple");
                });
                ui.separator();
                match self.x.find_mode {
                    1 => {
                        let r = ui.add(egui::TextEdit::singleline(&mut self.x.find_id).hint_text("ID number, e.g. 12").desired_width(f32::INFINITY));
                        if r.changed() {
                            self.x.find_id.retain(|c| c.is_ascii_alphanumeric() || c == '#' || c == '@');
                        }
                        ui.label(RichText::new("Also finds the old number of someone renumbered with Tools > Renumber people.").small().weak());
                        let found = self.db.find_by_number(&self.x.find_id);
                        if found.is_empty() && !self.x.find_id.trim().is_empty() {
                            ui.weak("Nobody has that number.");
                        }
                        for (id, old) in found {
                            let txt = if old { format!("{}   (Old ID; now #{})", self.db.label(id), id) } else { format!("#{}  {}", id, self.db.label(id)) };
                            if ui.selectable_label(self.sel == Some(id), txt).clicked() {
                                act = Some(Action::Select(id));
                            }
                        }
                    }
                    2 => {
                        ui.add(egui::TextEdit::singleline(&mut self.x.find_a).hint_text("One partner's name, e.g. John Smith").desired_width(f32::INFINITY));
                        ui.add(egui::TextEdit::singleline(&mut self.x.find_b).hint_text("The other partner's name, e.g. Mary").desired_width(f32::INFINITY));
                        ui.label(RichText::new("Part of a name is enough; either order works. Leave one box empty to see every spouse of the other person.").small().weak());
                        let fams = self.db.find_couples(&self.x.find_a, &self.x.find_b);
                        if fams.is_empty() && !(self.x.find_a.trim().is_empty() && self.x.find_b.trim().is_empty()) {
                            ui.weak("No couple matches.");
                        }
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for f in fams.iter().take(200) {
                                let fm = &self.db.fams[f];
                                let nm = |x: Option<u32>| x.and_then(|i| self.db.people.get(&i)).map(display_name).unwrap_or_else(|| "(not recorded)".into());
                                let y = year(&fm.md);
                                let txt = format!("{}  &  {}{}", nm(fm.h), nm(fm.w), if y.is_empty() { String::new() } else { format!("   (married {y})") });
                                if ui.selectable_label(false, txt).clicked() {
                                    if let Some(p) = fm.h.or(fm.w) {
                                        couple = Some((p, *f));
                                    }
                                }
                            }
                        });
                    }
                    _ => {
                        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Name or place").desired_width(f32::INFINITY));
                        ui.label(format!("{} people", self.db.people.len()));
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for id in self.db.sorted(&self.search) {
                                let txt = self.db.label(id);
                                if ui.selectable_label(self.sel == Some(id), txt).clicked() {
                                    act = Some(Action::Select(id));
                                }
                            }
                        });
                    }
                }
            });
            if let Some((p, f)) = couple {
                self.goto(p);
                self.view = View::Family;
                self.fx = self.db.fams_of(p).iter().position(|x| *x == f).unwrap_or(0);
            }
            self.find_open = open;
        }

        self.review_window(ctx);
        self.import_merge_window(ctx);
        self.letters_window(ctx);
        self.groups_window(ctx, &mut act);
        self.tools_windows(ctx);
        self.notes_replace_window(ctx, &mut act);
        if let Some(id) = self.src_pick.take() {
            if self.db.sources.contains_key(&id) {
                self.src_open = true;
                self.src_tab = 0;
                self.src_sel = Some(id);
                self.src_confirm = false;
            }
        }
        if self.src_open {
            let mut open = true;
            let mut go: Option<u32> = None;
            let mut del_src: Option<u32> = None;
            let mut merge: Option<(u32, u32)> = None;
            let mut del_repo: Option<u32> = None;
            let mut new_repo_for: Option<u32> = None;
            let counts = self.db.source_counts();
            let mut by_title: Vec<(u32, String)> = self.db.sources.iter().map(|(k, s)| (*k, source_title(s))).collect();
            by_title.sort_by_key(|t| t.1.to_lowercase());
            let mut repo_list: Vec<(u32, String)> = self.db.repos.iter().map(|(k, r)| (*k, if r.name.is_empty() { "(unnamed repository)".to_string() } else { r.name.clone() })).collect();
            repo_list.sort_by_key(|t| t.1.to_lowercase());
            let repo_use: HashMap<u32, usize> = {
                let mut n = HashMap::new();
                for s in self.db.sources.values() {
                    if let Some(r) = s.repo {
                        *n.entry(r).or_insert(0) += 1;
                    }
                }
                n
            };
            egui::Window::new("Sources and repositories").open(&mut open).default_size([900.0, 560.0]).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.src_tab == 0, format!("Sources ({})", self.db.sources.len())).clicked() {
                        self.src_tab = 0;
                        self.src_confirm = false;
                    }
                    if ui.selectable_label(self.src_tab == 1, format!("Repositories ({})", self.db.repos.len())).clicked() {
                        self.src_tab = 1;
                        self.src_confirm = false;
                    }
                });
                ui.separator();
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(270.0);
                        if self.src_tab == 0 {
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.src_filter).hint_text("search sources").desired_width(160.0));
                                if ui.button("New source").clicked() {
                                    let id = self.db.alloc();
                                    self.db.sources.insert(id, SourceRec::default());
                                    self.src_sel = Some(id);
                                    self.src_filter.clear();
                                }
                            });
                            let f = self.src_filter.to_lowercase();
                            egui::ScrollArea::vertical().id_salt("srclist").max_height(440.0).show(ui, |ui| {
                                for (id, t) in &by_title {
                                    let s = &self.db.sources[id];
                                    if !f.is_empty() && !format!("{} {} {}", t, s.author, s.abbr).to_lowercase().contains(&f) {
                                        continue;
                                    }
                                    let n = counts.get(id).copied().unwrap_or(0);
                                    let label = format!("{}  ({})", t, n);
                                    if ui.selectable_label(self.src_sel == Some(*id), label).clicked() {
                                        self.src_sel = Some(*id);
                                        self.src_confirm = false;
                                        self.src_merge = None;
                                    }
                                }
                            });
                        } else {
                            if ui.button("New repository").clicked() {
                                let id = self.db.alloc();
                                self.db.repos.insert(id, RepoRec::default());
                                self.repo_sel = Some(id);
                            }
                            egui::ScrollArea::vertical().id_salt("repolist").max_height(440.0).show(ui, |ui| {
                                for (id, t) in &repo_list {
                                    let n = repo_use.get(id).copied().unwrap_or(0);
                                    if ui.selectable_label(self.repo_sel == Some(*id), format!("{}  ({})", t, n)).clicked() {
                                        self.repo_sel = Some(*id);
                                        self.src_confirm = false;
                                    }
                                }
                            });
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_min_width(560.0);
                        egui::ScrollArea::vertical().id_salt("srcedit").max_height(470.0).show(ui, |ui| {
                            if self.src_tab == 0 {
                                let Some(sel) = self.src_sel.filter(|s| self.db.sources.contains_key(s)) else {
                                    ui.label("Choose a source on the left, or make a new one.");
                                    ui.label(RichText::new("A source is a book, census, record set, website or person. Write it down once here, then cite it from any number of people, facts and events (each citation adds its own page, transcription and quality).").weak());
                                    return;
                                };
                                let uses = self.db.source_uses(sel);
                                {
                                    let s = self.db.sources.get_mut(&sel).unwrap();
                                    egui::Grid::new("srcgrid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                        ui.label("Title");
                                        ui.add(egui::TextEdit::singleline(&mut s.title).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Author or creator");
                                        ui.add(egui::TextEdit::singleline(&mut s.author).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Short title");
                                        ui.add(egui::TextEdit::singleline(&mut s.abbr).hint_text("used in short lists").desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Type of source");
                                        egui::ComboBox::from_id_salt("srckind").selected_text(if s.kind.is_empty() { "not set" } else { s.kind.as_str() }).width(200.0).show_ui(ui, |ui| {
                                            ui.selectable_value(&mut s.kind, String::new(), "not set");
                                            for k in SOURCE_KINDS {
                                                ui.selectable_value(&mut s.kind, k.to_string(), *k);
                                            }
                                        });
                                        ui.end_row();
                                        ui.label("Publication details");
                                        ui.add(egui::TextEdit::singleline(&mut s.publ).hint_text("publisher, place, year, volume").desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Web address");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(&mut s.url).hint_text("https://...").desired_width(310.0));
                                            if is_url(&s.url) {
                                                ui.hyperlink_to("Open", s.url.clone());
                                            }
                                        });
                                        ui.end_row();
                                        ui.label("Kept at");
                                        ui.horizontal(|ui| {
                                            let shown = s.repo.and_then(|r| repo_list.iter().find(|t| t.0 == r)).map(|t| t.1.clone()).unwrap_or_else(|| "no repository".to_string());
                                            egui::ComboBox::from_id_salt("srcrepo").selected_text(shown).width(260.0).show_ui(ui, |ui| {
                                                if ui.selectable_label(s.repo.is_none(), "no repository").clicked() {
                                                    s.repo = None;
                                                }
                                                for (rid, t) in &repo_list {
                                                    if ui.selectable_label(s.repo == Some(*rid), t.as_str()).clicked() {
                                                        s.repo = Some(*rid);
                                                    }
                                                }
                                                ui.separator();
                                                if ui.selectable_label(false, "+ New repository...").clicked() {
                                                    new_repo_for = Some(sel);
                                                }
                                            });
                                            if let Some(r) = s.repo {
                                                if ui.small_button("Open repository").clicked() {
                                                    self.repo_sel = Some(r);
                                                    self.src_tab = 1;
                                                }
                                            }
                                        });
                                        ui.end_row();
                                        ui.label("Call number");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(&mut s.call).hint_text("shelf, film or reference number").desired_width(220.0));
                                            ui.add(egui::TextEdit::singleline(&mut s.medium).hint_text("book, microfilm, digital...").desired_width(150.0));
                                        });
                                        ui.end_row();
                                    });
                                    templates::template_ui(ui, s, &mut self.x.tpl_which, &mut self.x.tpl_vals);
                                    ui.label("Description, or text of the whole source");
                                    ui.add(egui::TextEdit::multiline(&mut s.text).desired_rows(3).desired_width(f32::INFINITY));
                                    ui.label("Your notes on this source (how reliable it is, where you found it)");
                                    ui.add(egui::TextEdit::multiline(&mut s.notes).desired_rows(3).desired_width(f32::INFINITY));
                                }
                                ui.add_space(6.0);
                                ui.separator();
                                ui.label(RichText::new(format!("Cited {} time(s)", uses.len())).strong());
                                egui::ScrollArea::vertical().id_salt("srcuses").max_height(150.0).show(ui, |ui| {
                                    for (o, what, when, page) in uses.iter().take(300) {
                                        ui.horizontal(|ui| {
                                            let who = self.db.owner_label(*o);
                                            if ui.link(who).clicked() {
                                                go = match o {
                                                    Owner::Person(i) => Some(*i),
                                                    Owner::Family(i) => self.db.fams.get(i).and_then(|f| f.h.or(f.w)),
                                                };
                                            }
                                            let mut t = what.clone();
                                            if !when.is_empty() {
                                                t = format!("{} {}", t, when);
                                            }
                                            if !page.is_empty() {
                                                t = format!("{}  -  {}", t, page);
                                            }
                                            ui.label(RichText::new(t).weak());
                                        });
                                    }
                                    if uses.len() > 300 {
                                        ui.label(format!("...and {} more", uses.len() - 300));
                                    }
                                });
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    let shown = self.src_merge.filter(|m| *m != sel).and_then(|m| by_title.iter().find(|t| t.0 == m)).map(|t| t.1.clone()).unwrap_or_else(|| "Merge into...".to_string());
                                    egui::ComboBox::from_id_salt("srcmerge").selected_text(shown).width(230.0).show_ui(ui, |ui| {
                                        for (id, t) in &by_title {
                                            if *id != sel && ui.selectable_label(self.src_merge == Some(*id), t.as_str()).clicked() {
                                                self.src_merge = Some(*id);
                                            }
                                        }
                                    });
                                    if let Some(target) = self.src_merge.filter(|m| *m != sel && self.db.sources.contains_key(m)) {
                                        if ui.button("Merge (same source, entered twice)").clicked() {
                                            merge = Some((target, sel));
                                        }
                                    }
                                });
                                let label = if self.src_confirm { "Click again to delete this source" } else { "Delete source" };
                                if ui.button(label).on_hover_text("Citations that used it keep its name as typed text").clicked() {
                                    if self.src_confirm {
                                        del_src = Some(sel);
                                    } else {
                                        self.src_confirm = true;
                                    }
                                }
                            } else {
                                let Some(sel) = self.repo_sel.filter(|s| self.db.repos.contains_key(s)) else {
                                    ui.label("Choose a repository on the left, or make a new one.");
                                    ui.label(RichText::new("A repository is where sources are kept: a library, archive, courthouse, church, website or a relative's attic.").weak());
                                    return;
                                };
                                {
                                    let r = self.db.repos.get_mut(&sel).unwrap();
                                    egui::Grid::new("repogrid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                                        ui.label("Name");
                                        ui.add(egui::TextEdit::singleline(&mut r.name).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Address");
                                        ui.add(egui::TextEdit::multiline(&mut r.addr).desired_rows(3).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Phone");
                                        ui.add(egui::TextEdit::singleline(&mut r.phone).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Email");
                                        ui.add(egui::TextEdit::singleline(&mut r.email).desired_width(380.0));
                                        ui.end_row();
                                        ui.label("Web address");
                                        ui.horizontal(|ui| {
                                            ui.add(egui::TextEdit::singleline(&mut r.url).hint_text("https://...").desired_width(310.0));
                                            if is_url(&r.url) {
                                                ui.hyperlink_to("Open", r.url.clone());
                                            }
                                        });
                                        ui.end_row();
                                    });
                                    ui.label("Notes (opening hours, how to request records)");
                                    ui.add(egui::TextEdit::multiline(&mut r.notes).desired_rows(3).desired_width(f32::INFINITY));
                                }
                                ui.add_space(6.0);
                                ui.separator();
                                ui.label(RichText::new("Sources kept here").strong());
                                let here: Vec<(u32, String)> = by_title.iter().filter(|(id, _)| self.db.sources[id].repo == Some(sel)).cloned().collect();
                                if here.is_empty() {
                                    ui.label(RichText::new("None yet. Choose this repository in a source's \"Kept at\" box.").weak());
                                }
                                for (id, t) in here {
                                    if ui.link(t).clicked() {
                                        self.src_sel = Some(id);
                                        self.src_tab = 0;
                                    }
                                }
                                ui.add_space(6.0);
                                let label = if self.src_confirm { "Click again to delete this repository" } else { "Delete repository" };
                                if ui.button(label).on_hover_text("Its sources stay, they just no longer say where they are kept").clicked() {
                                    if self.src_confirm {
                                        del_repo = Some(sel);
                                    } else {
                                        self.src_confirm = true;
                                    }
                                }
                            }
                        });
                    });
                });
            });
            self.src_open = open;
            if let Some(src) = new_repo_for {
                let rid = self.db.alloc();
                self.db.repos.insert(rid, RepoRec::default());
                if let Some(s) = self.db.sources.get_mut(&src) {
                    s.repo = Some(rid);
                }
                self.repo_sel = Some(rid);
                self.src_tab = 1;
            }
            if let Some((keep, gone)) = merge {
                self.db.merge_sources(keep, gone);
                self.src_sel = Some(keep);
                self.src_merge = None;
                self.dirty = true;
                self.status = "Merged the two sources. Everything that cited either now cites the one source.".to_string();
            }
            if let Some(id) = del_src {
                self.db.delete_source(id);
                self.src_sel = None;
                self.src_confirm = false;
                self.dirty = true;
            }
            if let Some(id) = del_repo {
                self.db.delete_repo(id);
                self.repo_sel = None;
                self.src_confirm = false;
                self.dirty = true;
            }
            if let Some(p) = go {
                act = Some(Action::Open(p));
            }
        }

        if self.show_loc {
            let mut open = true;
            let mut reset = false;
            egui::Window::new("Where my tree is kept").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label("Your tree, photos, recordings and backups are kept in this folder:");
                ui.label(RichText::new(data_dir().display().to_string()).strong());
                if let Some(svc) = synced_service(&data_dir()) {
                    ui.label(RichText::new(format!("This folder looks like it is synced by {}. Avoid opening the same tree on two computers at once.", svc)).color(Color32::from_rgb(214, 160, 60)));
                }
                if is_portable() {
                    ui.label("Portable mode is on, so everything stays next to the app.");
                } else {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Choose a different folder...").clicked() {
                            self.move_data = true;
                        }
                        if config_dir().join("location.txt").exists() && ui.button("Back to the default (Documents)").clicked() {
                            reset = true;
                        }
                        if ui.button("Open folder").clicked() {
                            open_folder(&store_path());
                        }
                    });
                    ui.label(RichText::new("Your data is copied to the new place first and checked. The old folder is left as it was.").small().weak());
                }
            });
            self.show_loc = open;
            if reset {
                self.persist();
                let src = data_dir();
                let target = documents_dir().join("Gilded Roots");
                if target.join("tree.json").exists() || src == target {
                    let _ = std::fs::remove_file(config_dir().join("location.txt"));
                    self.status = format!("Using {} again.", target.display());
                } else {
                    match copy_dir(&src, &target) {
                        Ok(_) if std::fs::read(src.join("tree.json")).ok() == std::fs::read(target.join("tree.json")).ok() => {
                            let _ = std::fs::remove_file(config_dir().join("location.txt"));
                            self.status = format!("Your tree is back in {}. The other folder was left untouched.", target.display());
                        }
                        _ => self.status = "Could not copy your data, so nothing was changed.".to_string(),
                    }
                }
            }
        }

        if self.show_look {
            let mut open = true;
            let mut changed = false;
            egui::Window::new("Appearance").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label("Theme");
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.settings.dark, "Dark").clicked() {
                        self.settings.dark = true;
                        self.settings.ink = None;
                        changed = true;
                    }
                    if ui.selectable_label(!self.settings.dark, "Light").clicked() {
                        self.settings.dark = false;
                        self.settings.ink = None;
                        self.settings.light_bg = default_light_bg();
                        changed = true;
                    }
                });
                ui.add_space(8.0);
                ui.collapsing(RichText::new("Colour schemes").strong(), |ui| {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        if scheme_list(ui, &mut self.settings) {
                            changed = true;
                        }
                    });
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Accent colour").strong());
                if swatch_grid(ui, &PRESETS, &mut self.settings.accent, false) {
                    changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Custom");
                    if ui.color_edit_button_srgb(&mut self.settings.accent).changed() {
                        changed = true;
                    }
                });
                if self.settings.dark {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Dark background").strong());
                    if swatch_grid(ui, &BG_PRESETS, &mut self.settings.bg, true) {
                        changed = true;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Custom");
                        if ui.color_edit_button_srgb(&mut self.settings.bg).changed() {
                            changed = true;
                        }
                    });
                }
                ui.add_space(6.0);
                if ui.button("Reset to default").clicked() {
                    let keep = self.settings.gens;
                    self.settings = Settings::default();
                    self.settings.gens = keep;
                    changed = true;
                }
            });
            self.show_look = open;
            if changed {
                apply_theme(ctx, &self.settings);
                save_settings(&self.settings);
            }
        }

        if let Some(w) = self.wipe {
            let (title, msg) = match w {
                Wipe::Sample => ("Load sample data?", "This replaces your tree with the sample family."),
                Wipe::Empty => ("Start with an empty tree?", "This removes everyone from your tree."),
                Wipe::Import => ("Import a GEDCOM file?", "Importing replaces your tree with the contents of the file."),
                Wipe::Restore => ("", ""),
            };
            let mut choice: Option<bool> = None;
            egui::Window::new(title).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label(format!("You have {} people in your tree. {}", self.db.people.len(), msg));
                ui.label("A backup of your current tree is saved first. You can bring it back with Tools > Restore previous tree.");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        choice = Some(false);
                    }
                    if ui.button("Replace my tree").clicked() {
                        choice = Some(true);
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    choice = Some(false);
                }
            });
            if let Some(c) = choice {
                self.wipe = None;
                if c {
                    self.run_wipe(w);
                }
            }
        }

        if self.move_data {
            self.move_data = false;
            self.change_data_folder();
        }
        if self.show_about {
            let mut open = true;
            egui::Window::new("About Gilded Roots").open(&mut open).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new(egui::include_image!("../icons/icon-256.png")).max_width(110.0));
                    ui.add_space(4.0);
                    ui.heading("Gilded Roots");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label("A free, offline home for your family history.");
                    ui.label(RichText::new("Created by Doyle McIntosh (DGMCINT)").italics());
                });
                ui.separator();
                ui.label(RichText::new("Quick tips").strong());
                ui.label("- Click a box in the pedigree to highlight it. Double-click to open that person's page.");
                ui.label("- Click a blank box to add a father or mother.");
                ui.label("- Right-click a box (or use its ... button) for more actions.");
                ui.label("- Press Ctrl/Cmd+K to jump to anyone or any command. Press F1 for the user guide.");
                ui.label("- Use the arrow on the last column to see older generations.");
                ui.label("- Use File > Export GEDCOM to make a backup you can keep anywhere.");
                ui.separator();
                ui.label(RichText::new("Support").strong());
                ui.label("Gilded Roots is free. If you enjoy it and would like to help with its development, a small donation, even $5, is very much appreciated. It is entirely optional.");
                ui.hyperlink_to("Donate with PayPal", DONATE_URL);
                ui.separator();
                ui.label(RichText::new("Your data").strong());
                ui.label("Everything is stored on this computer only.");
                ui.label(format!("Folder: {}", data_dir().display()));
                if is_portable() {
                    ui.label("Portable mode: your data is kept in the FamilyTreeData folder next to the app.");
                }
                if mac_translocated() {
                    ui.colored_label(Color32::from_rgb(222, 166, 60), "macOS is running this copy from a hidden temporary folder, so portable mode cannot work. Open Terminal and run once:  xattr -dr com.apple.quarantine  followed by a space, then drag Gilded Roots.app onto the Terminal window and press Return. Then open the app again.");
                }
                if let Some(svc) = synced_service(&data_dir()) {
                    ui.label(RichText::new(format!("This folder looks like it is synced by {}. That is fine on one computer, but avoid opening the same tree on two computers at once, and keep files downloaded (turn off storage optimising for this folder). \"Change folder...\" moves it somewhere that does not sync.", svc)).color(Color32::from_rgb(214, 160, 60)));
                }
                ui.horizontal(|ui| {
                    if ui.button("Open data folder").clicked() {
                        open_folder(&store_path());
                    }
                    if !is_portable() && ui.button("Change folder...").clicked() {
                        self.move_data = true;
                    }
                });
                ui.separator();
                ui.label(RichText::new("Licence and notices").strong());
                ui.label("Gilded Roots is free, open-source software under the MIT licence. It comes as is, without any warranty.");
                ui.label(RichText::new(NOT_AFFILIATED).small().weak());
                ui.label(RichText::new("Map data: Natural Earth (public domain). Town names: GeoNames, CC BY 4.0.").small().weak());
                ui.horizontal(|ui| {
                    if ui.button("Open-source licences...").clicked() {
                        self.x.licenses_open = true;
                    }
                    if PROJECT_URL.is_empty() {
                        ui.label(RichText::new("Questions and bug reports: the Issues tab of the Gilded Roots page on GitHub.").small());
                    } else {
                        ui.hyperlink_to("Questions and bug reports", PROJECT_URL);
                    }
                });
            });
            self.show_about = open;
        }
        if self.x.licenses_open {
            let mut open = true;
            egui::Window::new("Open-source licences").open(&mut open).default_size([640.0, 560.0]).show(ctx, |ui| {
                ui.label("Gilded Roots is built with these open-source libraries. Thank you to their authors.");
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(THIRD_PARTY).monospace().small()).wrap());
                });
            });
            self.x.licenses_open = open;
        }

        if self.imp_open {
            let mut open = true;
            let mut pick = false;
            egui::Window::new("Import a family tree").open(&mut open).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label("Ancestry, MyHeritage, FamilySearch and most programs share trees as a GEDCOM (.ged) file. Export one from your account, then choose it here.");
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for (i, n) in ["Any GEDCOM file", "Ancestry", "MyHeritage", "FamilySearch"].iter().enumerate() {
                        if ui.selectable_label(self.imp_src == i, *n).clicked() {
                            self.imp_src = i;
                        }
                    }
                });
                ui.separator();
                match self.imp_src {
                    1 => {
                        ui.label("1. Sign in to Ancestry and open your tree.");
                        ui.label("2. Click the tree name menu and choose Tree Settings.");
                        ui.label("3. Under Manage Your Tree, click Export tree and wait for it to be prepared.");
                        ui.label("4. Click Download your GEDCOM file.");
                        ui.weak("You usually need to be the tree owner. Photos and documents are not included in the file.");
                        ui.hyperlink_to("Open Ancestry", "https://www.ancestry.com");
                    }
                    2 => {
                        ui.label("1. Sign in to MyHeritage.");
                        ui.label("2. Open the Family Tree menu and choose Manage trees.");
                        ui.label("3. Click Export to GEDCOM, then Begin the export.");
                        ui.label("4. Download the file (you may get an email with a link).");
                        ui.weak("Photos are not included in the file.");
                        ui.hyperlink_to("Open MyHeritage", "https://www.myheritage.com");
                    }
                    3 => {
                        ui.label("1. Sign in to FamilySearch and open the Download your family tree page.");
                        ui.label("2. Click the export button to download a GEDCOM file.");
                        ui.weak("It includes up to 8 generations of your ancestors, without photos. FamilySearch's shared tree has no full-tree download. Programs like RootsMagic can export more.");
                        ui.hyperlink_to("Open the FamilySearch export page", "https://familysearch.org/innovate/export");
                    }
                    _ => {
                        ui.label("Any program that can export GEDCOM works, such as RootsMagic, Legacy, Gramps, Family Tree Maker, or PAF itself.");
                    }
                }
                ui.separator();
                ui.radio_value(&mut self.imp_replace, false, "Add to my tree (you check and confirm every possible match)");
                ui.radio_value(&mut self.imp_replace, true, "Replace my tree with this file");
                ui.add_space(6.0);
                if ui.button("Choose GEDCOM file...").clicked() {
                    pick = true;
                }
            });
            if pick {
                open = false;
                if self.imp_replace || self.is_pristine_sample() || self.db.people.is_empty() {
                    self.request_wipe(Wipe::Import);
                } else {
                    self.import_merge();
                }
            }
            self.imp_open = open;
        }

        if self.rel_open {
            let mut open = true;
            egui::Window::new("Relationship finder").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label("Choose two people to see how they are related.");
                let ids = self.db.sorted("");
                for (title, which) in [("Person A", 0), ("Person B", 1)] {
                    ui.label(title);
                    let cur = if which == 0 { &mut self.rel_a } else { &mut self.rel_b };
                    let shown = (*cur).filter(|i| self.db.people.contains_key(i)).map(|i| self.db.label(i)).unwrap_or_else(|| "Choose...".to_string());
                    egui::ComboBox::from_id_salt(("rel", which)).selected_text(shown).width(340.0).show_ui(ui, |ui| {
                        for id in &ids {
                            if ui.selectable_label(*cur == Some(*id), self.db.label(*id)).clicked() {
                                *cur = Some(*id);
                            }
                        }
                    });
                }
                if let (Some(a), Some(b)) = (self.rel_a, self.rel_b) {
                    if self.db.people.contains_key(&a) && self.db.people.contains_key(&b) {
                        ui.separator();
                        ui.label(RichText::new(self.db.relationship(a, b)).strong());
                    }
                }
            });
            self.rel_open = open;
        }

        if self.view == View::Person {
            if let Some(id) = self.sel {
                self.ensure_previews(id);
            }
        }
        if matches!(self.view, View::Gallery | View::Memories | View::Person) && self.missing_previews(4) {
            // new previews were made: draw again straight away (and make the next few), so new
            // photos appear without having to leave the tab and come back
            ctx.request_repaint();
        }
        self.viewer(ctx);

        if self.check_open {
            let mut open = true;
            let issues = self.db.check();
            let mut go_to: Option<u32> = None;
            egui::Window::new("Data checker").open(&mut open).default_size([560.0, 420.0]).show(ctx, |ui| {
                ui.label(format!("{} thing(s) worth a look. Click a name to open that person.", issues.len()));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (id, msg) in &issues {
                        ui.horizontal(|ui| {
                            if ui.link(self.db.label(*id)).clicked() {
                                go_to = Some(*id);
                            }
                            ui.label(msg.as_str());
                        });
                    }
                });
            });
            self.check_open = open;
            if let Some(id) = go_to {
                act = Some(Action::Open(id));
            }
        }

        if self.merge_open {
            let mut open = true;
            let mut do_merge: Option<(u32, u32)> = None;
            match self.sel.filter(|i| self.db.people.contains_key(i)) {
                None => open = false,
                Some(me) => {
                    let mut others: Vec<u32> = self.db.people.keys().copied().filter(|i| *i != me).collect();
                    others.sort_by_key(|i| self.db.people[i].name.to_lowercase());
                    egui::Window::new("Merge with another person").open(&mut open).default_size([460.0, 220.0]).show(ctx, |ui| {
                        ui.label(format!("Keep: {}", self.db.label(me)));
                        ui.label("Choose the other record of this person. You will see both side by side and confirm before anything is merged.");
                        let shown = self.merge_other.filter(|i| self.db.people.contains_key(i)).map(|i| self.db.label(i)).unwrap_or_else(|| "Choose...".to_string());
                        egui::ComboBox::from_id_salt("mergeother").selected_text(shown).width(380.0).show_ui(ui, |ui| {
                            for id in &others {
                                if ui.selectable_label(self.merge_other == Some(*id), format!("{}  #{}", self.db.label(*id), id)).clicked() {
                                    self.merge_other = Some(*id);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("or type the person's ID (#):");
                            ui.add(egui::TextEdit::singleline(&mut self.merge_id).desired_width(70.0));
                        });
                        if let Ok(n) = self.merge_id.trim().trim_start_matches('#').parse::<u32>() {
                            if n != me && self.db.people.contains_key(&n) {
                                self.merge_other = Some(n);
                            }
                        }
                        if let Some(o) = self.merge_other {
                            if ui.button("Compare and merge...").clicked() {
                                do_merge = Some((me, o));
                            }
                        }
                    });
                }
            }
            if let Some((a, b)) = do_merge {
                self.start_review(a, b);
                open = false;
            }
            self.merge_open = open;
        }

        if self.dup_open {
            let mut open = true;
            let mut merge: Option<(u32, u32)> = None;
            let mut ignore: Option<(u32, u32)> = None;
            let mut rescan = false;
            egui::Window::new("Possible duplicates").open(&mut open).default_size([580.0, 440.0]).show(ctx, |ui| {
                ui.label(format!("{} possible duplicate pair(s).", self.dups.len()));
                if ui.button("Scan again").clicked() {
                    rescan = true;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (a, b, why) in &self.dups {
                        if !self.db.people.contains_key(a) || !self.db.people.contains_key(b) {
                            continue;
                        }
                        ui.group(|ui| {
                            ui.label(RichText::new(why.as_str()).small().weak());
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(format!("{}  #{}", self.db.label(*a), a)).strong());
                                    ui.label(self.db.people[a].bp.as_str());
                                });
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(format!("{}  #{}", self.db.label(*b), b)).strong());
                                    ui.label(self.db.people[b].bp.as_str());
                                });
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Compare and merge...").clicked() {
                                    merge = Some((*a, *b));
                                }
                                if ui.button("Not the same").clicked() {
                                    ignore = Some((*a, *b));
                                }
                            });
                        });
                    }
                });
            });
            self.dup_open = open;
            if let Some((a, b)) = merge {
                self.start_review(a, b);
            }
            if let Some(pair) = ignore {
                self.dup_ignore.insert(pair);
                self.dups.retain(|x| (x.0, x.1) != pair);
            }
            if rescan {
                self.dups = self.db.duplicate_pairs().into_iter().filter(|x| !self.dup_ignore.contains(&(x.0, x.1))).collect();
            }
        }

        if self.report_open {
            let mut open = true;
            let mut save_text: Option<String> = None;
            if let Some(r) = &self.report {
                egui::Window::new("Import report").open(&mut open).default_size([620.0, 520.0]).show(ctx, |ui| {
                    ui.label(RichText::new(format!("{}", r.file)).strong());
                    let mut from = String::new();
                    if !r.program.is_empty() {
                        from.push_str(&format!("Written by {}. ", r.program));
                    }
                    if !r.gedcom_version.is_empty() {
                        from.push_str(&format!("GEDCOM {}. ", r.gedcom_version));
                    }
                    from.push_str(&format!("Character set: {}.", r.charset));
                    ui.label(from);
                    ui.add_space(6.0);
                    ui.label(format!(
                        "{} people, {} families, {} sources ({} citations), {} shared notes, {} other names, {} events and facts, {} photos/documents.",
                        r.people, r.families, r.sources, r.citations, r.notes, r.alt_names, r.events, r.media
                    ));
                    if r.media > 0 {
                        ui.label(format!("Photos and documents: {} found and copied in, {} not found next to the file (their names are kept).", r.media_found, r.media_missing));
                    }
                    ui.add_space(6.0);
                    if r.lost.is_empty() {
                        ui.colored_label(Color32::from_rgb(90, 190, 110), "Nothing was lost: every line of the file will appear again when you export.");
                    } else {
                        ui.colored_label(Color32::from_rgb(225, 70, 70), format!("{} piece(s) of the file would NOT appear in an export:", r.lost.len()));
                        egui::ScrollArea::vertical().id_salt("lostlist").max_height(150.0).show(ui, |ui| {
                            for l in &r.lost {
                                ui.label(RichText::new(l).monospace());
                            }
                        });
                    }
                    if !r.kept.is_empty() {
                        ui.add_space(6.0);
                        egui::CollapsingHeader::new(format!("{} item(s) kept exactly as read (this app has no field for them, but they are saved and written back)", r.kept_total())).show(ui, |ui| {
                            egui::ScrollArea::vertical().id_salt("keptlist").max_height(150.0).show(ui, |ui| {
                                for (k, n) in &r.kept {
                                    ui.label(RichText::new(format!("{:>5}  {}", n, k)).monospace());
                                }
                            });
                        });
                    }
                    if !r.warnings.is_empty() {
                        ui.add_space(6.0);
                        egui::CollapsingHeader::new(format!("{} note(s) about the file", r.warnings.len())).default_open(true).show(ui, |ui| {
                            egui::ScrollArea::vertical().id_salt("warnlist").max_height(120.0).show(ui, |ui| {
                                for w in r.warnings.iter().take(200) {
                                    ui.label(w);
                                }
                            });
                        });
                    }
                    ui.add_space(8.0);
                    if ui.button("Save this report as a text file...").clicked() {
                        let mut t = format!("Import report for {}\n{}\nCharacter set: {}\n\n", r.file, r.program, r.charset);
                        t.push_str(&format!("{} people, {} families, {} sources, {} citations, {} shared notes, {} other names, {} events, {} media\n\n", r.people, r.families, r.sources, r.citations, r.notes, r.alt_names, r.events, r.media));
                        t.push_str(&format!("Would not appear in an export ({}):\n", r.lost.len()));
                        for l in &r.lost {
                            t.push_str(&format!("  {}\n", l));
                        }
                        t.push_str("\nKept exactly as read:\n");
                        for (k, n) in &r.kept {
                            t.push_str(&format!("  {:>5}  {}\n", n, k));
                        }
                        t.push_str("\nNotes:\n");
                        for w in &r.warnings {
                            t.push_str(&format!("  {}\n", w));
                        }
                        save_text = Some(t);
                    }
                });
            }
            self.report_open = open;
            if let Some(t) = save_text {
                if let Some(p) = rfd::FileDialog::new().set_file_name("import-report.txt").save_file() {
                    self.status = match atomic_write(&p, t.as_bytes()) {
                        Ok(_) => format!("Saved {}", p.display()),
                        Err(e) => format!("Could not save: {}", e),
                    };
                }
            }
        }

        if self.verify_open {
            let mut open = true;
            let mut again = false;
            egui::Window::new("Backup check").open(&mut open).default_size([540.0, 380.0]).show(ctx, |ui| {
                ui.label("Each backup is read back and compared with its checksum.");
                if self.verify_results.is_empty() {
                    ui.label("No dated backups yet.");
                }
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    for (name, r) in &self.verify_results {
                        match r {
                            Ok(n) => {
                                ui.label(format!("OK        {}   ({} people)", name, n));
                            }
                            Err(e) => {
                                ui.colored_label(Color32::from_rgb(225, 70, 70), format!("DAMAGED   {}   {}", name, e));
                            }
                        }
                    }
                });
                ui.separator();
                ui.label(self.media_gap.as_str());
                if ui.button("Check again").clicked() {
                    again = true;
                }
            });
            self.verify_open = open;
            if again {
                self.run_verify();
            }
        }

        if self.link.is_some() {
            let mut close = false;
            let mut chosen: Option<(u32, Option<u32>, bool, u32)> = None;
            let mut cands: Vec<(u32, String)> = Vec::new();
            if let Some(l) = self.link.as_ref() {
                cands = self.db.sorted(&l.q).into_iter().filter(|i| *i != l.child).take(12).map(|i| (i, self.db.label(i))).collect();
            }
            if let Some(l) = self.link.as_mut() {
                egui::Window::new("Link an existing person as a parent").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Search");
                        ui.text_edit_singleline(&mut l.q);
                    });
                    if l.fid.is_none() {
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut l.father, true, "as father / first parent");
                            ui.radio_value(&mut l.father, false, "as mother / second parent");
                        });
                    }
                    ui.separator();
                    for (i, lbl) in &cands {
                        if ui.link(lbl.as_str()).clicked() {
                            chosen = Some((l.child, l.fid, l.father, *i));
                        }
                    }
                    if cands.is_empty() {
                        ui.weak("No matching people.");
                    }
                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            }
            if let Some((c, f, fa, p)) = chosen {
                self.db.add_parent(c, f, fa, p);
                self.dirty = true;
                close = true;
            }
            if close {
                self.link = None;
            }
        }

        let mut done: Option<bool> = None;
        if let Some(d) = self.dlg.as_mut() {
            let title = match d.kind {
                Kind::Father => "Add father",
                Kind::Mother => "Add mother",
                Kind::Spouse => "Add spouse",
                Kind::Child(_) => "Add child",
                Kind::Person => "Add person",
                Kind::ParentIn(_, true) => "Add father to this family",
                Kind::ParentIn(_, false) => "Add mother to this family",
                Kind::NewParent(true) => "Add another father (new set of parents)",
                Kind::NewParent(false) => "Add another mother (new set of parents)",
                Kind::Sibling => "Add brother or sister",
            };
            let lock = matches!(d.kind, Kind::Father | Kind::Mother | Kind::Spouse);
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(440.0);
                    let mut name_resp = None;
                    egui::Grid::new("add_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("Full name");
                        let te = ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(300.0));
                        if d.fresh > 0 {
                            te.request_focus();
                            d.fresh -= 1;
                            ctx.request_repaint();
                        }
                        name_resp = Some(te);
                        ui.end_row();
                        ui.label("Sex");
                        ui.add_enabled_ui(!lock, |ui| {
                            let st = match d.sex.as_str() {
                                "F" => "Female",
                                "M" => "Male",
                                _ => "Unknown",
                            };
                            egui::ComboBox::from_id_salt("dsex").selected_text(st).show_ui(ui, |ui| {
                                ui.selectable_value(&mut d.sex, "M".to_string(), "Male");
                                ui.selectable_value(&mut d.sex, "F".to_string(), "Female");
                                ui.selectable_value(&mut d.sex, String::new(), "Unknown");
                            });
                        });
                        ui.end_row();
                        ui.label("Born");
                        ditto(ui, &mut d.bd, |t| t.hint_text("e.g. 12 Mar 1921, about 1850").desired_width(300.0), &mut self.x.ditto_date);
                        ui.end_row();
                        ui.label("Birthplace");
                        ditto(ui, &mut d.bp, |t| t.desired_width(300.0), &mut self.x.ditto_place);
                        ui.end_row();
                        ui.label("Died");
                        ditto(ui, &mut d.dd, |t| t.desired_width(300.0), &mut self.x.ditto_date);
                        ui.end_row();
                        ui.label("Place of death");
                        ditto(ui, &mut d.dp, |t| t.desired_width(300.0), &mut self.x.ditto_place);
                        ui.end_row();
                        ui.label("Buried");
                        ditto(ui, &mut d.bud, |t| t.desired_width(300.0), &mut self.x.ditto_date);
                        ui.end_row();
                        ui.label("Burial place");
                        ditto(ui, &mut d.bup, |t| t.desired_width(300.0), &mut self.x.ditto_place);
                        ui.end_row();
                        ui.label("Living");
                        ui.checkbox(&mut d.living, "Living (kept private when printing or exporting)");
                        ui.end_row();
                        ui.label("Notes");
                        ui.add(egui::TextEdit::multiline(&mut d.notes).desired_rows(3).desired_width(300.0));
                        ui.end_row();
                    });
                    ui.label(RichText::new("Only the name is required. Everything else can be added later.").small().weak());
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() && !d.name.trim().is_empty() {
                            done = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            done = Some(false);
                        }
                    });
                    let enter = name_resp.map(|t| t.lost_focus()).unwrap_or(false) && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if enter && !d.name.trim().is_empty() {
                        done = Some(true);
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        done = Some(false);
                    }
                });
        }
        if let Some(ok) = done {
            let d = self.dlg.take().unwrap();
            if ok {
                self.create(d);
                self.dirty = true;
            }
        }

        if !ctx.wants_keyboard_input() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z)) {
                self.redo_action();
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.undo_action();
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)) {
                self.redo_action();
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.find_open = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            act = Some(Action::Add(Kind::Person));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals)) {
            act = Some(Action::SetZoom(self.settings.zoom + 0.1));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus)) {
            act = Some(Action::SetZoom(self.settings.zoom - 0.1));
        }
        for (k, v) in [(egui::Key::Num1, View::Family), (egui::Key::Num2, View::Pedigree), (egui::Key::Num3, View::Descendants), (egui::Key::Num4, View::Person), (egui::Key::Num5, View::Gallery), (egui::Key::Num6, View::Charts), (egui::Key::Num7, View::Table), (egui::Key::Num8, View::Map), (egui::Key::Num9, View::Memories)] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, k)) {
                self.view = v;
            }
        }

        // Keyboard: arrows move around the tree (not while typing or while a dialog is open).
        if act.is_none()
            && self.dlg.is_none()
            && self.link.is_none()
            && self.review.is_none()
            && self.imerge.is_none()
            && !ctx.wants_keyboard_input()
            && matches!(self.view, View::Pedigree | View::Descendants | View::Family)
        {
            let (key, shift, home) = ctx.input(|i| {
                let plain = !i.modifiers.command && !i.modifiers.alt;
                let k = [egui::Key::ArrowRight, egui::Key::ArrowLeft, egui::Key::ArrowUp, egui::Key::ArrowDown].into_iter().find(|k| plain && i.key_pressed(*k));
                (k, i.modifiers.shift, plain && i.key_pressed(egui::Key::Home))
            });
            if home {
                act = Some(Action::GoHome);
            } else if let Some(k) = key {
                if let Some(t) = self.nav_target(k, shift) {
                    act = Some(Action::Select(t));
                }
            }
        }

        if let Some(a) = act {
            match a {
                Action::Select(i) => {
                    self.goto(i);
                    self.hl = None;
                    self.fx = 0;
                    self.confirm = false;
                }
                Action::Fx(i) => self.fx = i,
                Action::MoveChild(f, k, up) => {
                    if self.db.move_child(f, k, up) {
                        self.x.next_label = Some("Changed the order of children".into());
                        self.dirty = true;
                    }
                }
                Action::GroupToggle(g, id) => {
                    if let Some(gr) = self.db.groups.get_mut(g) {
                        if !gr.ids.remove(&id) {
                            gr.ids.insert(id);
                        }
                        self.x.next_label = Some("Changed a group".into());
                        self.dirty = true;
                    }
                }
                Action::GroupsOpen(id) => {
                    self.x.group_who = Some(id);
                    self.x.groups_open = true;
                }
                Action::SortChildren(f) => {
                    if self.db.sort_children(f) {
                        self.x.next_label = Some("Put children in order of birth".into());
                        self.dirty = true;
                        self.status = "Children put in order of birth.".into();
                    } else {
                        self.status = "The children are already in order of birth.".into();
                    }
                }
                Action::MoveFamily(who, f, up) => {
                    if self.db.move_family(who, f, up) {
                        self.fx = self.db.fams_of(who).iter().position(|x| *x == f).unwrap_or(0);
                        self.x.next_label = Some("Changed the order of spouses".into());
                        self.dirty = true;
                    }
                }
                Action::Highlight(i) => self.hl = Some(i),
                Action::AttachFiles => {
                    if let Some(id) = self.sel {
                        self.attach_files(id);
                    }
                }
                Action::StartRec => {
                    if let Some(id) = self.sel {
                        if self.rec.is_none() {
                            self.start_recording(id);
                        }
                    }
                }
                Action::StopRec => self.stop_recording(),
                Action::PrintPedigree => {
                    if let Some(r) = self.sel {
                        let h = self.pedigree_html(r);
                        self.open_print(h);
                    }
                }
                Action::Bookmark(i) => {
                    if let Some(k) = self.db.bookmarks.iter().position(|x| *x == i) {
                        self.db.bookmarks.remove(k);
                        self.status = format!("Bookmark removed for {}.", self.db.label(i));
                    } else {
                        self.db.bookmarks.push(i);
                        self.status = format!("Bookmarked {}. Find them under Bookmarks above the chart.", self.db.label(i));
                    }
                    self.dirty = true;
                }
                Action::PrintChart => {
                    if let Some((title, body)) = self.chart_page() {
                        let landscape = self.x.chart != ChartKind::Fan || self.x.fan_deg < 359.0;
                        self.open_print(page(&title, &body, landscape));
                    }
                }
                Action::PrintFamily => {
                    if let Some(r) = self.sel {
                        let h = self.family_html(r);
                        self.open_print(h);
                    }
                }
                Action::SetZoom(z) => {
                    self.settings.zoom = z.clamp(0.25, 1.8);
                    self.fit_req = false;
                    save_settings(&self.settings);
                }
                Action::Fit => self.fit_req = true,
                Action::EditStart => {
                    if let Some(id) = self.sel {
                        self.view = View::Person;
                        self.editing = Some(EditSession { id, snapshot: serde_json::to_string(&self.db).unwrap_or_default() });
                    }
                }
                Action::EditSave => {
                    self.editing = None;
                    self.status = "Changes saved.".to_string();
                    self.dirty = true;
                }
                Action::EditCancel => {
                    if let Some(e) = self.editing.take() {
                        self.load_snapshot(e.snapshot);
                        self.status = "Changes cancelled. The person is back as they were.".to_string();
                        self.dirty = true;
                    }
                }
                Action::SetHome(i) => {
                    self.db.home = Some(i);
                    self.dirty = true;
                    let who = self.db.people.get(&i).map(display_name).unwrap_or_default();
                    self.status = format!("{} is now the home person. Use the Home button, View > Go to home person, or the Home key to return.", who);
                }
                Action::ShowParentsFor(who, f) => {
                    if let Some(p) = self.db.people.get_mut(&who) {
                        p.pref_parents = Some(f);
                    }
                    self.dirty = true;
                }
                Action::ShowParents(f) => {
                    if let Some(sid) = self.sel {
                        if let Some(p) = self.db.people.get_mut(&sid) {
                            p.pref_parents = Some(f);
                        }
                    }
                }
                Action::GoHome => {
                    if let Some(h) = self.db.home.filter(|h| self.db.people.contains_key(h)) {
                        self.goto(h);
                        self.hl = None;
                        self.fx = 0;
                    } else {
                        self.status = "No home person set yet. Right-click a person and choose \"Make this the home person\".".to_string();
                    }
                }
                Action::PrintDescendants => {
                    if let Some(r) = self.sel {
                        let h = self.descendant_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportAncestors => {
                    if let Some(r) = self.sel {
                        let h = self.ancestors_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportDescendants => {
                    if let Some(r) = self.sel {
                        let h = self.descendants_list_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportBirthdays => {
                    let h = self.birthdays_html();
                    self.open_print(h);
                }
                Action::ReportSources => {
                    let h = self.sources_html();
                    self.open_print(h);
                }
                Action::ReportAncestorBook => {
                    if let Some(r) = self.sel {
                        let h = self.ancestor_book_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportDescendantBook => {
                    if let Some(r) = self.sel {
                        let h = self.descendant_book_html(r);
                        self.open_print(h);
                    }
                }
                Action::ReportBook => {
                    if let Some(r) = self.sel {
                        let h = self.book_html(r);
                        self.open_print(h);
                    }
                }
                Action::Back => {
                    while let Some(p) = self.back.pop() {
                        if self.db.people.contains_key(&p) {
                            if let Some(c) = self.sel {
                                self.fwd.push(c);
                            }
                            self.sel = Some(p);
                            self.hl = None;
                            break;
                        }
                    }
                }
                Action::Forward => {
                    while let Some(p) = self.fwd.pop() {
                        if self.db.people.contains_key(&p) {
                            if let Some(c) = self.sel {
                                self.back.push(c);
                            }
                            self.sel = Some(p);
                            self.hl = None;
                            break;
                        }
                    }
                }
                Action::SetGens(n) => {
                    self.settings.gens = n.clamp(3, 8);
                    save_settings(&self.settings);
                }
                Action::SetHand(on) => {
                    self.settings.hand = on;
                    save_settings(&self.settings);
                }
                Action::AddFor(target, k) => {
                    let t_f = self.db.people.get(&target).map(|p| p.sex == "F").unwrap_or(false);
                    let sex = match k {
                        Kind::Father | Kind::ParentIn(_, true) | Kind::NewParent(true) => "M",
                        Kind::Mother | Kind::ParentIn(_, false) | Kind::NewParent(false) => "F",
                        Kind::Spouse => {
                            if t_f { "M" } else { "F" }
                        }
                        _ => "M",
                    };
                    self.dlg = Some(Dlg::new(k, sex, Some(target)));
                }
                Action::ViewTree(i, which) => {
                    self.goto(i);
                    self.hl = None;
                    self.fx = 0;
                    self.view = match which {
                        0 => View::Pedigree,
                        1 => View::Descendants,
                        _ => View::Family,
                    };
                }
                Action::QuickEdit(i) => {
                    if let Some(p) = self.db.people.get(&i) {
                        self.quick = Some(QuickEdit { id: i, given: p.given.clone(), surname: p.surname.clone(), sex: p.sex.clone(), bd: p.bd.clone(), bp: p.bp.clone(), dd: p.dd.clone(), dp: p.dp.clone(), living: p.living });
                    }
                }
                Action::Open(i) => {
                    self.hl = None;
                    self.goto(i);
                    self.view = View::Person;
                    self.fx = 0;
                    self.confirm = false;
                }
                Action::Add(k) => {
                    let me_f = self.sel.and_then(|i| self.db.people.get(&i)).map(|p| p.sex == "F").unwrap_or(false);
                    let sex = match k {
                        Kind::Father | Kind::ParentIn(_, true) | Kind::NewParent(true) => "M",
                        Kind::Mother | Kind::ParentIn(_, false) | Kind::NewParent(false) => "F",
                        Kind::Spouse => {
                            if me_f { "M" } else { "F" }
                        }
                        _ => "M",
                    };
                    self.dlg = Some(Dlg::new(k, sex, None));
                }
                Action::Delete => {
                    self.delete();
                    self.dirty = true;
                }
            }
        }

        let changed = ctx.input(|i| {
            i.pointer.any_released()
                || i.events.iter().any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Key { pressed: true, .. }))
        });
        if changed {
            self.dirty = true;
            self.track();
            self.backup_if_due(ctx.input(|i| i.time));
        }
        if !self.x.sized {
            self.x.sized = true;
            if self.settings.win.is_none() {
                if let Some(m) = ctx.input(|i| i.viewport().monitor_size) {
                    if m.x > 100.0 && m.y > 100.0 {
                        let w = (m.x * 0.85).clamp(800.0, 1800.0);
                        let h = (m.y * 0.85).clamp(560.0, 1200.0);
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(w, h)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos2((m.x - w) / 2.0, ((m.y - h) / 2.0).max(24.0))));
                    }
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            let (inner, maxed, full) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
            if !full {
                self.settings.win_max = maxed;
                if let (Some(r), false) = (inner, maxed) {
                    self.settings.win = Some([r.width(), r.height()]);
                }
                save_settings(&self.settings);
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty {
            if self.persist() {
                self.dirty = false;
            } else if !self.close_warned {
                self.close_warned = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.status = "Your latest changes could not be saved, so the app was not closed. Fix the problem shown in red, or export a GEDCOM copy first. Close again to quit anyway.".to_string();
            }
        }
        if self.dirty && std::time::Instant::now() >= self.next_save {
            if self.persist() {
                self.dirty = false;
            }
        }
        if !self.startup_done {
            self.startup_done = true;
            self.spawn_media_job(false);
        }
        self.poll_media_job();
        if self.media_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(if self.save_error.is_some() { 3 } else { 10 }));
        self.note_text_focus(ctx);
    }
}
