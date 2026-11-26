//! The guided tour (Help > Tutorials): short lessons on the sample family. Each step says
//! what to do, outlines the button to press, and moves on by itself once it has been done.
//!
//! Buttons the tour can point at note where they were drawn with `mark` each frame; the tour
//! card is drawn last and outlines the ones the current step names.

use super::*;
use std::cell::RefCell;

thread_local! {
    static MARKS: RefCell<HashMap<&'static str, Rect>> = RefCell::new(HashMap::new());
}

/// Notes where a button the tour may point at was drawn this frame.
pub(crate) fn mark(key: &'static str, r: Rect) {
    MARKS.with(|m| {
        m.borrow_mut().insert(key, r);
    });
}

fn marked(key: &str) -> Option<Rect> {
    MARKS.with(|m| m.borrow().get(key).copied())
}

pub(crate) fn clear_marks() {
    MARKS.with(|m| m.borrow_mut().clear());
}

/// How things stood when a step began, so a step can tell when it has been done.
#[derive(Clone, Default)]
pub(crate) struct Base {
    pub(crate) people: usize,
    pub(crate) redo: usize,
    pub(crate) person_json: String,
    pub(crate) saves: u32,
    pub(crate) prints: u32,
}

/// A tour in progress.
pub(crate) struct Tour {
    pub(crate) lesson: usize,
    pub(crate) step: usize,
    pub(crate) base: Base,
    /// When the current step was seen to be done (the card says "Done" for a moment).
    pub(crate) done_at: Option<f64>,
    /// Counts of things the steps watch for.
    pub(crate) saves: u32,
    pub(crate) prints: u32,
    /// The card is folded down to its title bar.
    pub(crate) small: bool,
}

type Check = fn(&App, &Base) -> bool;

pub(crate) struct Step {
    pub(crate) text: &'static str,
    /// Buttons to outline (the first one drawn is used for the arrow).
    pub(crate) targets: &'static [&'static str],
    /// None: the step only explains, and Next moves on.
    pub(crate) check: Option<Check>,
}

pub(crate) struct Lesson {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) about: &'static str,
    pub(crate) steps: &'static [Step],
}

fn sel_named(app: &App, name: &str) -> bool {
    app.sel.and_then(|s| app.db.people.get(&s)).map_or(false, |p| p.name == name)
}

fn sel_json(app: &App) -> String {
    app.sel.and_then(|s| app.db.people.get(&s)).and_then(|p| serde_json::to_string(p).ok()).unwrap_or_default()
}

pub(crate) const LESSONS: &[Lesson] = &[
    Lesson {
        id: "around",
        title: "Finding your way around",
        about: "The pedigree chart, a person's page, the Family tab, Home and Quick jump.",
        steps: &[
            Step {
                text: "This is the pedigree chart. Emily Rose Harwood, on the left, is the home person; her parents and grandparents spread out to the right. Click her father, Robert Walter Harwood, once to highlight him.",
                targets: &["ped_father"],
                check: Some(|a, _| sel_named(a, "Robert Walter Harwood") || a.hl.and_then(|h| a.db.people.get(&h)).map_or(false, |p| p.name == "Robert Walter Harwood")),
            },
            Step {
                text: "Now double-click Robert to open his Individual page, with everything recorded about him. (Clicking the Individual tab does the same.)",
                targets: &["tab:Individual"],
                check: Some(|a, _| a.view == View::Person),
            },
            Step {
                text: "The page is locked, so nothing can change by accident. Click the Family tab to see Robert with his wife and children.",
                targets: &["tab:Family"],
                check: Some(|a, _| a.view == View::Family),
            },
            Step {
                text: "Press Home on the toolbar to go back to Emily, the home person. The Home key on the keyboard does the same.",
                targets: &["home"],
                check: Some(|a, _| a.sel.is_some() && a.sel == a.db.home),
            },
            Step {
                text: "Quick jump finds anyone, or any command, by typing. Press Quick jump (or Ctrl+K; Cmd+K on a Mac), type Thomas and press Enter.",
                targets: &["jump"],
                check: Some(|a, _| sel_named(a, "Thomas James Harwood")),
            },
        ],
    },
    Lesson {
        id: "add",
        title: "Adding relatives",
        about: "Add a husband for Emily and a child for them both.",
        steps: &[
            Step {
                text: "Press Home to go back to Emily, then open the Family tab.",
                targets: &["home", "tab:Family"],
                check: Some(|a, _| a.view == View::Family && a.sel.is_some() && a.sel == a.db.home),
            },
            Step {
                text: "The Family tab's toolbar adds relatives to the selected person. Press Add spouse.",
                targets: &["fam_spouse"],
                check: Some(|a, _| a.dlg.as_ref().map_or(false, |d| d.kind == Kind::Spouse)),
            },
            Step {
                text: "Type a name, such as John Carter, and check that Sex says Male. Add a birth year if you like (only the name is needed), then press Add.",
                targets: &["dlg_add"],
                check: Some(|a, b| a.db.people.len() > b.people),
            },
            Step {
                text: "John is now Emily's husband. Press Add child to give them a child: type a first name and press Add.",
                targets: &["fam_child", "dlg_add"],
                check: Some(|a, b| a.db.people.len() > b.people),
            },
            Step {
                text: "The child is listed under the couple. Click any name or card to go to that person. Brothers, sisters and parents are added the same way.",
                targets: &[],
                check: None,
            },
        ],
    },
    Lesson {
        id: "edit",
        title: "Changing facts",
        about: "Unlock a page, change something, save it, and undo it.",
        steps: &[
            Step {
                text: "Press Home to go to Emily, then open the Individual tab.",
                targets: &["home", "tab:Individual"],
                check: Some(|a, _| a.view == View::Person && a.sel.is_some() && a.sel == a.db.home),
            },
            Step {
                text: "Press Edit to unlock the page.",
                targets: &["page_edit", "ind_edit"],
                check: Some(|a, _| a.editing.is_some()),
            },
            Step {
                text: "Change something: type a nickname, or change her birthplace. In an empty date or place box, Ctrl+D (Cmd+D on a Mac) fills in the last one you typed.",
                targets: &[],
                check: Some(|a, b| a.editing.is_some() && sel_json(a) != b.person_json),
            },
            Step {
                text: "Press Save to keep the change. (Cancel would put Emily back exactly as she was.)",
                targets: &["page_save", "ind_done"],
                check: Some(|a, b| a.editing.is_none() && tour_count(a, |t| t.saves) > b.saves),
            },
            Step {
                text: "Changed your mind? Press Undo on the toolbar (or Ctrl+Z; Cmd+Z on a Mac) to take the change back.",
                targets: &["undo"],
                check: Some(|a, b| a.redo.len() > b.redo),
            },
        ],
    },
    Lesson {
        id: "memories",
        title: "Photos, documents and stories",
        about: "Look at a photo with tagged faces and read a document.",
        steps: &[
            Step {
                text: "Open the Memories tab. It holds every photo, document, story and recording in the tree.",
                targets: &["tab:Memories"],
                check: Some(|a, _| a.view == View::Memories),
            },
            Step {
                text: "Click the wedding photo of Thomas and Margaret to look at it.",
                targets: &[],
                check: Some(|a, _| a.viewing.is_some()),
            },
            Step {
                text: "Both faces are tagged: Tag faces in the photo window shows the boxes. Close the photo when you have had a look.",
                targets: &[],
                check: Some(|a, _| a.viewing.is_none()),
            },
            Step {
                text: "Now click the 1930 census extract (a PDF) to read it in the built-in viewer. Close the viewer when you are done.",
                targets: &[],
                check: Some(|a, _| a.x.pdf.is_some()),
            },
            Step {
                text: "To add your own, select a person and press Add files on the toolbar: photos, PDFs, letters and sound recordings are all welcome. Write a story and Record a story are beside it.",
                targets: &["mem_files", "mem_story", "mem_rec"],
                check: None,
            },
        ],
    },
    Lesson {
        id: "charts",
        title: "Charts and printing",
        about: "See the family as a fan or a timeline, and print a chart.",
        steps: &[
            Step {
                text: "On the Pedigree tab, choose Fan chart. It puts the selected person in the middle; each ring is one generation further back.",
                targets: &["tab:Pedigree", "ped:Fan"],
                check: Some(|a, _| a.view == View::Pedigree && a.settings.ped_mode == 1),
            },
            Step {
                text: "On a person's page, the Timeline tab has Family lifespans: who was alive at the same time.",
                targets: &["tab:Individual", "tl:Lifespans"],
                check: Some(|a, _| a.view == View::Person && a.settings.tl_mode == 1),
            },
            Step {
                text: "Press Print chart. The printout opens in your web browser, where you can print it or save it as a PDF.",
                targets: &["chart_print"],
                check: Some(|a, b| tour_count(a, |t| t.prints) > b.prints),
            },
            Step {
                text: "Every tab prints what it shows with Print on the main toolbar. The Reports menu has family group sheets, books and lists, and Reports > Forms prints blank forms to fill in by hand.",
                targets: &["print", "menu:Reports"],
                check: None,
            },
        ],
    },
    Lesson {
        id: "map",
        title: "The map",
        about: "Watch the family move over the years.",
        steps: &[
            Step {
                text: "Open the Map tab. Each pin is a place where something happened: green for births, red for deaths, purple for marriages.",
                targets: &["tab:Map"],
                check: Some(|a, _| a.view == View::Map),
            },
            Step {
                text: "Press Play under the map to watch the family move from England and Denmark to the American West.",
                targets: &["map_play"],
                check: Some(|a, _| a.x.map_play),
            },
            Step {
                text: "Click any pin to see who was born, married or died there, and click a name to go to them.",
                targets: &[],
                check: Some(|a, _| a.x.map_pick.is_some()),
            },
        ],
    },
    Lesson {
        id: "yours",
        title: "Starting your own tree",
        about: "Leave the sample family and begin with yours.",
        steps: &[
            Step {
                text: "Nothing you did in the sample family was saved. When you are ready, press Back to my tree in the gold bar at the top.",
                targets: &["practice_back"],
                check: Some(|a, _| a.x.practice.is_none()),
            },
            Step {
                text: "To begin, press Add person and add yourself, then your parents and grandparents from the Family tab. Or bring in a tree from another program with File > Import tree. The User guide (F1) has a step-by-step chapter: Your first family tree.",
                targets: &["add", "menu:File"],
                check: None,
            },
        ],
    },
];

/// Reads one of the tour's counters (0 when no tour is running).
fn tour_count(a: &App, f: fn(&Tour) -> u32) -> u32 {
    a.x.tour.as_ref().map_or(0, f)
}

impl App {
    fn tour_base(&self) -> Base {
        Base {
            people: self.db.people.len(),
            redo: self.redo.len(),
            person_json: sel_json(self),
            saves: tour_count(self, |t| t.saves),
            prints: tour_count(self, |t| t.prints),
        }
    }

    /// Starts a lesson (in the sample family; practice mode is started if needed).
    pub(crate) fn tour_start(&mut self, lesson: usize) {
        if self.x.practice.is_none() && lesson + 1 < LESSONS.len() {
            self.start_practice();
        }
        self.x.tour = Some(Tour { lesson, step: 0, base: Base::default(), done_at: None, saves: 0, prints: 0, small: false });
        self.x.tour_list = false;
        let b = self.tour_base();
        if let Some(t) = self.x.tour.as_mut() {
            t.base = b;
        }
    }

    /// Notes actions a step may be waiting for (Save, printing).
    pub(crate) fn tour_saw(&mut self, a: &Action) {
        let Some(t) = self.x.tour.as_mut() else { return };
        match a {
            Action::EditSave => t.saves += 1,
            Action::PrintPedigree | Action::PrintFamily | Action::PrintDescendants | Action::PrintChart | Action::ReportBook | Action::ReportAncestors | Action::ReportDescendants => t.prints += 1,
            _ => {}
        }
    }

    fn tour_goto(&mut self, lesson: usize, step: usize) {
        let b = self.tour_base();
        if let Some(t) = self.x.tour.as_mut() {
            t.lesson = lesson;
            t.step = step;
            t.base = b;
            t.done_at = None;
        }
    }

    fn tour_finish_lesson(&mut self) {
        let Some(t) = self.x.tour.as_ref() else { return };
        let id = LESSONS[t.lesson].id.to_string();
        if !self.settings.tour_done.contains(&id) {
            self.settings.tour_done.push(id);
            if !cfg!(test) {
                save_settings(&self.settings);
            }
        }
        self.x.tour = None;
        self.x.tour_list = true;
    }

    /// Draws the tour card, outlines the button the step is about, and moves on when a step
    /// has been done. Called once a frame, after everything else is drawn.
    pub(crate) fn tour_frame(&mut self, ctx: &egui::Context) {
        if self.x.tour_list {
            self.tour_lessons(ctx);
        }
        let Some(t) = self.x.tour.as_ref() else {
            clear_marks();
            return;
        };
        let (li, si) = (t.lesson.min(LESSONS.len() - 1), t.step);
        let lesson = &LESSONS[li];
        let Some(step) = lesson.steps.get(si) else {
            self.tour_finish_lesson();
            clear_marks();
            return;
        };
        let now = ctx.input(|i| i.time);
        // The sample family is needed for every lesson but the last; if it was left, offer to go back.
        let lost = self.x.practice.is_none() && li + 1 < LESSONS.len();
        let done = !lost && step.check.map_or(false, |c| c(self, &t.base));
        if done && t.done_at.is_none() {
            self.x.tour.as_mut().unwrap().done_at = Some(now);
        }
        let done_at = self.x.tour.as_ref().and_then(|t| t.done_at);
        if let Some(d) = done_at {
            if now - d > 1.2 {
                self.tour_goto(li, si + 1);
            }
            ctx.request_repaint();
        }

        // Outline the buttons this step is about, gently pulsing.
        if !lost && done_at.is_none() {
            let rects: Vec<Rect> = step.targets.iter().filter_map(|k| marked(k)).collect();
            if !rects.is_empty() {
                let pulse = ((now * 3.0).sin() * 0.5 + 0.5) as f32;
                let gold = Color32::from_rgb(232, 178, 40);
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("tour_marks")));
                for r in rects {
                    let r = r.expand(3.0 + pulse * 3.0);
                    painter.rect_stroke(r, 6.0, Stroke::new(3.0_f32, gold.gamma_multiply(0.55 + 0.45 * pulse)));
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(40));
            }
        }
        clear_marks();

        let small = self.x.tour.as_ref().map_or(false, |t| t.small);
        let mut go: Option<(usize, usize)> = None;
        let mut end = false;
        let mut list = false;
        let mut back_to_sample = false;
        let mut fold = None;
        let n = lesson.steps.len();
        crate::look::window(crate::look::wtitle(crate::i18n::tr("Guided tour")))
            .id(egui::Id::new("tour_card"))
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, [-16.0, -40.0])
            .frame(egui::Frame::window(&ctx.style()).stroke(Stroke::new(2.0_f32, Color32::from_rgb(232, 178, 40))))
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(crate::i18n::tr("Guided tour")).strong().color(Color32::from_rgb(232, 178, 40)));
                    ui.label(RichText::new(crate::i18n::trv(lesson.title)).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").on_hover_text(crate::i18n::tr("End the tour")).clicked() {
                            end = true;
                        }
                        if ui.small_button(if small { "^" } else { "v" }).on_hover_text(if small { crate::i18n::tr("Show the tour") } else { crate::i18n::tr("Fold the tour away") }).clicked() {
                            fold = Some(!small);
                        }
                    });
                });
                if small {
                    return;
                }
                ui.add(egui::ProgressBar::new((si as f32 + if done_at.is_some() { 1.0 } else { 0.0 }) / n as f32).desired_height(4.0));
                ui.label(RichText::new(crate::i18n::trf("Step {} of {}", &["Step ", " of ", ""], &[(si + 1).to_string(), n.to_string()])).small().weak());
                ui.add_space(4.0);
                if lost {
                    ui.label(RichText::new(crate::i18n::tr("The tour uses the sample family, and you have gone back to your own tree.")).size(15.0));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::tr("Go back to the sample family")).clicked() {
                            back_to_sample = true;
                        }
                        if ui.button(crate::i18n::tr("End the tour")).clicked() {
                            end = true;
                        }
                    });
                    return;
                }
                if done_at.is_some() {
                    ui.label(RichText::new(crate::i18n::tr("Well done!")).size(17.0).strong().color(Color32::from_rgb(90, 180, 100)));
                } else {
                    ui.add(egui::Label::new(RichText::new(crate::i18n::trv(step.text)).size(15.0).color(ui.visuals().strong_text_color())).wrap());
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(si > 0, egui::Button::new(crate::i18n::tr("< Back"))).clicked() {
                        go = Some((li, si - 1));
                    }
                    let last = si + 1 == n;
                    let next = if step.check.is_none() {
                        if last { crate::i18n::tr("Finish") } else { crate::i18n::tr("Next >") }
                    } else {
                        crate::i18n::tr("Skip this step")
                    };
                    let b = if step.check.is_none() { egui::Button::new(RichText::new(next).strong()) } else { egui::Button::new(next) };
                    if ui.add_enabled(done_at.is_none(), b).clicked() {
                        go = Some((li, si + 1));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(crate::i18n::tr("Lessons")).clicked() {
                            list = true;
                        }
                    });
                });
            });
        if let Some(f) = fold {
            if let Some(t) = self.x.tour.as_mut() {
                t.small = f;
            }
        }
        if back_to_sample {
            self.tour_start(li);
            return;
        }
        if let Some((l, s)) = go {
            self.tour_goto(l, s);
        }
        if list {
            self.x.tour_list = true;
        }
        if end {
            self.x.tour = None;
        }
    }

    /// The list of lessons (Help > Tutorials).
    fn tour_lessons(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut start = None;
        crate::look::window(crate::look::wtitle(crate::i18n::tr("Guided tour")))
            .id(egui::Id::new("tour_lessons"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.label(crate::i18n::tr("Short lessons on the made-up sample family. Each step shows you what to press and moves on when you have done it. Nothing you do is saved, and your own tree is not touched."));
                ui.add_space(8.0);
                let first_open = LESSONS.iter().position(|l| !self.settings.tour_done.iter().any(|d| d == l.id));
                for (i, l) in LESSONS.iter().enumerate() {
                    let done = self.settings.tour_done.iter().any(|d| d == l.id);
                    ui.horizontal(|ui| {
                        ui.add_sized([16.0, 18.0], egui::Label::new(RichText::new(if done { "✔" } else { "" }).color(Color32::from_rgb(90, 180, 100))));
                        let w = ui.available_width() - 80.0;
                        ui.vertical(|ui| {
                            ui.set_width(w);
                            ui.label(RichText::new(crate::i18n::trf("{}. {}", &["", ". ", ""], &[(i + 1).to_string(), crate::i18n::trv(l.title).to_string()])).strong());
                            ui.add(egui::Label::new(RichText::new(crate::i18n::trv(l.about)).size(12.5).weak()).wrap());
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = if done { crate::i18n::tr("Again") } else { crate::i18n::tr("Start") };
                            let b = if Some(i) == first_open { egui::Button::new(RichText::new(label).strong()).stroke(Stroke::new(1.5_f32, Color32::from_rgb(232, 178, 40))) } else { egui::Button::new(label) };
                            if ui.add(b).clicked() {
                                start = Some(i);
                            }
                        });
                    });
                    ui.add_space(2.0);
                }
                ui.add_space(6.0);
                ui.label(RichText::new(crate::i18n::tr("The User guide (F1) explains everything in more detail.")).small().weak());
            });
        if let Some(i) = start {
            self.tour_start(i);
        } else if !open {
            self.x.tour_list = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(app: &mut App, ctx: &egui::Context, t: f64) {
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), time: Some(t), ..Default::default() };
        let _ = ctx.run(input, |c| app.frame(c));
    }

    #[test]
    fn every_lesson_draws_and_steps_move_on_when_done() {
        let mut app = App::for_test(Db::default());
        let ctx = egui::Context::default();
        app.x.tour_list = true;
        run(&mut app, &ctx, 0.0);
        app.tour_start(0);
        assert!(app.x.practice.is_some(), "the tour opens the sample family");
        let home = app.db.home;
        assert!(home.is_some());
        // every step of every lesson draws
        for (li, l) in LESSONS.iter().enumerate() {
            for si in 0..l.steps.len() {
                app.tour_goto(li, si);
                run(&mut app, &ctx, 1.0);
            }
        }
        // step 1: selecting Robert is noticed, and after a moment the tour moves on
        app.tour_goto(0, 0);
        let robert = app.db.people.iter().find(|(_, p)| p.name == "Robert Walter Harwood").map(|(k, _)| *k).unwrap();
        app.hl = Some(robert);
        run(&mut app, &ctx, 10.0);
        assert!(app.x.tour.as_ref().unwrap().done_at.is_some());
        run(&mut app, &ctx, 12.0);
        assert_eq!(app.x.tour.as_ref().unwrap().step, 1);
        // the toolbar's Home button is marked, so the step pointing at it can outline it
        app.tour_goto(0, 3);
        run(&mut app, &ctx, 13.0);
        // the last step of a lesson finishes it and remembers it
        app.tour_goto(0, LESSONS[0].steps.len());
        run(&mut app, &ctx, 14.0);
        assert!(app.x.tour.is_none());
        assert!(app.settings.tour_done.iter().any(|d| d == "around"));
        // leaving the sample family during a lesson offers to go back
        app.tour_start(2);
        app.end_practice();
        run(&mut app, &ctx, 15.0);
        assert!(app.x.tour.is_some());
        app.tour_start(2);
        assert!(app.x.practice.is_some());
    }

    #[test]
    fn saving_and_undo_are_noticed() {
        let mut app = App::for_test(Db::default());
        let ctx = egui::Context::default();
        app.tour_start(2);
        app.sel = app.db.home;
        app.view = View::Person;
        run(&mut app, &ctx, 1.0);
        run(&mut app, &ctx, 3.0);
        assert_eq!(app.x.tour.as_ref().unwrap().step, 1);
        app.editing = Some(EditSession { id: app.sel.unwrap(), snapshot: serde_json::to_string(&app.db).unwrap() });
        run(&mut app, &ctx, 4.0);
        run(&mut app, &ctx, 6.0);
        assert_eq!(app.x.tour.as_ref().unwrap().step, 2);
        let s = app.sel.unwrap();
        app.db.people.get_mut(&s).unwrap().name.push_str(" Test");
        run(&mut app, &ctx, 7.0);
        run(&mut app, &ctx, 9.0);
        assert_eq!(app.x.tour.as_ref().unwrap().step, 3);
        app.tour_saw(&Action::EditSave);
        app.editing = None;
        run(&mut app, &ctx, 10.0);
        run(&mut app, &ctx, 12.0);
        assert_eq!(app.x.tour.as_ref().unwrap().step, 4);
    }
}
