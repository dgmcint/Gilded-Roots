//! The Mac's own menu bar, at the top of the screen: the Gilded Roots menu (About, Settings, Hide,
//! Quit), File, Edit, View, Window and Help, the way every Mac program has them. The menus inside
//! the window stay as well, since they hold everything (Add, Search, Reports, Tools...).
//!
//! Only keys nothing else uses are given to the menu bar (Cmd+, Cmd+P Cmd+Q Cmd+W Cmd+H Cmd+M and
//! full screen): a Mac menu item with a key takes that key before the program sees it, so Cmd+C,
//! Cmd+V and Cmd+Z stay with the text boxes and the program, as now.
//!
//! On Windows and Linux this does nothing.

use super::*;

/// A command chosen from the Mac menu bar.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn run_command(app: &mut App, ctx: &egui::Context, id: &str, act: &mut Option<Action>) {
    match id {
        "about" => app.show_about = true,
        "settings" => app.x.settings_open = true,
        "quit" | "close" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        "trees" => app.x.trees_open = true,
        "import" => app.imp_open = true,
        "export_all" => app.export(false),
        "export_private" => app.export(true),
        "export7" => app.export7(false, false),
        "website" => app.web_open(),
        "arch_save" => app.save_archive_pick(),
        "arch_open" => app.open_archive_pick(),
        "print" => {
            *act = Some(match app.view {
                View::Pedigree => Action::PrintPedigree,
                View::Family => Action::PrintFamily,
                View::Descendants => Action::PrintDescendants,
                View::Charts => Action::PrintChart,
                _ => Action::ReportBook,
            })
        }
        "undo" => app.undo_action(),
        "redo" => app.redo_action(),
        "find" => app.find_open = true,
        "searchall" => {
            app.x.sall_open = true;
            app.x.sall_focus = true;
        }
        "jump" => {
            app.x.jump_open = true;
            app.x.jump_q.clear();
            app.x.jump_sel = 0;
        }
        "guide" => app.help_open = true,
        "tour" => app.x.tour_list = true,
        "wiki" => ctx.open_url(egui::OpenUrl::new_tab("https://github.com/dgmcint/Gilded-Roots/wiki")),
        v if v.starts_with("view:") => {
            if let Some((_, view)) = TABS.iter().find(|(name, _)| *name == &v[5..]) {
                app.view = *view;
            }
        }
        _ => {}
    }
}

/// The tabs, as the View menu lists them.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const TABS: [(&str, View); 11] = [
    ("Home", View::Home),
    ("Family", View::Family),
    ("Pedigree", View::Pedigree),
    ("Descendants", View::Descendants),
    ("Individual", View::Person),
    ("Memories", View::Memories),
    ("Charts", View::Charts),
    ("People", View::Table),
    ("Map", View::Map),
    ("Research", View::Research),
    ("DNA", View::Dna),
];

#[cfg(target_os = "macos")]
mod mac {
    use super::TABS;
    use eframe::egui;
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use std::cell::RefCell;
    use std::sync::mpsc::{channel, Receiver};
    use std::sync::Mutex;

    struct Bar {
        menu: Menu,
        lang: String,
    }

    thread_local! {
        static BAR: RefCell<Option<Bar>> = const { RefCell::new(None) };
    }
    static EVENTS: Mutex<Option<Receiver<String>>> = Mutex::new(None);

    fn tr(s: &'static str) -> String {
        crate::i18n::tr(s).to_string()
    }

    fn item(id: &str, text: &str, key: Option<(Modifiers, Code)>) -> MenuItem {
        MenuItem::with_id(id, text, true, key.map(|(m, c)| Accelerator::new(Some(m), c)))
    }

    fn build() -> muda::Result<Menu> {
        let cmd = Modifiers::SUPER;
        let sep = PredefinedMenuItem::separator;
        let app = Submenu::with_items(
            "Gilded Roots",
            true,
            &[
                &item("about", &tr("About Gilded Roots"), None),
                &sep(),
                &item("settings", &tr("Settings..."), Some((cmd, Code::Comma))),
                &sep(),
                &PredefinedMenuItem::services(None),
                &sep(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &sep(),
                &item("quit", &tr("Quit Gilded Roots"), Some((cmd, Code::KeyQ))),
            ],
        )?;
        let file = Submenu::with_items(
            tr("File"),
            true,
            &[
                &item("trees", &tr("Family trees..."), None),
                &sep(),
                &item("import", &tr("Import tree..."), None),
                &item("export_all", &tr("Export GEDCOM (everything)..."), None),
                &item("export_private", &tr("Export GEDCOM (living people private)..."), None),
                &item("export7", &tr("Export GEDCOM 7"), None),
                &item("website", &tr("Make a family website..."), None),
                &sep(),
                &item("arch_save", &tr("Save Family Tree Archive..."), None),
                &item("arch_open", &tr("Open Family Tree Archive..."), None),
                &sep(),
                &item("print", &tr("Print..."), Some((cmd, Code::KeyP))),
                &sep(),
                &item("close", &tr("Close Window"), Some((cmd, Code::KeyW))),
            ],
        )?;
        // Undo and Redo without keys here: Cmd+Z and Cmd+Shift+Z keep working as they do now,
        // in text boxes too.
        let edit = Submenu::with_items(
            tr("Edit"),
            true,
            &[
                &item("undo", &tr("Undo"), None),
                &item("redo", &tr("Redo"), None),
                &sep(),
                &item("find", &tr("Find a person..."), None),
                &item("searchall", &tr("Search everything..."), None),
                &item("jump", &tr("Quick jump..."), None),
            ],
        )?;
        let view = Submenu::new(tr("View"), true);
        for (name, _) in TABS {
            view.append(&item(&format!("view:{}", name), crate::i18n::trv(name), None))?;
        }
        view.append(&sep())?;
        view.append(&PredefinedMenuItem::fullscreen(None))?;
        let window = Submenu::with_items(
            tr("Window"),
            true,
            &[&PredefinedMenuItem::minimize(None), &PredefinedMenuItem::maximize(None), &sep(), &PredefinedMenuItem::bring_all_to_front(None)],
        )?;
        let help = Submenu::with_items(
            tr("Help"),
            true,
            &[&item("guide", &tr("Gilded Roots User Guide"), None), &item("tour", &tr("Guided tour (step-by-step lessons)"), None), &sep(), &item("wiki", &tr("Gilded Roots on the web (wiki)"), None)],
        )?;
        // Gilded Roots has one window, so no "Show Tab Bar" or "Show All Tabs" in the View menu.
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            #[allow(unused_unsafe)]
            unsafe {
                objc2_app_kit::NSWindow::setAllowsAutomaticWindowTabbing(false, mtm);
            }
        }
        let menu = Menu::with_items(&[&app, &file, &edit, &view, &window, &help])?;
        menu.init_for_nsapp();
        window.set_as_windows_menu_for_nsapp();
        help.set_as_help_menu_for_nsapp();
        Ok(menu)
    }

    /// Put the menu bar up (again when the language changes), and return the commands chosen.
    pub(crate) fn poll(ctx: &egui::Context) -> Vec<String> {
        let lang = crate::i18n::language();
        BAR.with(|b| {
            let mut b = b.borrow_mut();
            if b.as_ref().map_or(true, |bar| bar.lang != lang) {
                if b.is_none() {
                    // menu clicks wake the program up, so they are seen at once
                    let (tx, rx) = channel();
                    *EVENTS.lock().unwrap() = Some(rx);
                    let ctx = ctx.clone();
                    let tx = Mutex::new(tx);
                    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                        let _ = tx.lock().map(|t| t.send(e.id.0.clone()));
                        ctx.request_repaint();
                    }));
                }
                if let Some(old) = b.take() {
                    old.menu.remove_for_nsapp();
                }
                if let Ok(menu) = build() {
                    *b = Some(Bar { menu, lang: lang.clone() });
                }
            }
        });
        let mut out = Vec::new();
        if let Some(rx) = EVENTS.lock().unwrap().as_ref() {
            while let Ok(id) = rx.try_recv() {
                out.push(id);
            }
        }
        out
    }
}

impl App {
    /// Each frame: the Mac menu bar's chosen commands (nothing elsewhere).
    pub(crate) fn mac_menu(&mut self, ctx: &egui::Context, act: &mut Option<Action>) {
        #[cfg(target_os = "macos")]
        for id in mac::poll(ctx) {
            run_command(self, ctx, &id, act);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (ctx, act);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every Mac menu command does what it says, tested here on any computer.
    #[test]
    fn mac_menu_commands_work() {
        let mut app = App::for_test(crate::sample::sample_family());
        let ctx = egui::Context::default();
        let mut act = None;
        for (id, check) in [
            ("about", (|a: &App| a.show_about) as fn(&App) -> bool),
            ("settings", |a| a.x.settings_open),
            ("trees", |a| a.x.trees_open),
            ("import", |a| a.imp_open),
            ("find", |a| a.find_open),
            ("searchall", |a| a.x.sall_open),
            ("jump", |a| a.x.jump_open),
            ("guide", |a| a.help_open),
            ("tour", |a| a.x.tour_list),
            ("website", |a| a.x.web.is_some()),
        ] {
            run_command(&mut app, &ctx, id, &mut act);
            assert!(check(&app), "{id}");
        }
        for (name, view) in TABS {
            run_command(&mut app, &ctx, &format!("view:{name}"), &mut act);
            assert!(app.view == view, "{name}");
        }
        run_command(&mut app, &ctx, "print", &mut act);
        assert!(act.is_some());
        // the tab names are ones the program already translates
        for (name, _) in TABS {
            assert!(!crate::i18n::trv(name).is_empty());
        }
    }
}
