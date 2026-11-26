//! The research notepad (F10): a floating window of dated pages for jotting things down while
//! researching. It is kept with the tree and saved as you type. A line (or the text selected)
//! can be sent to the selected person's notes or to-do list.

use super::*;

/// One page of the notepad.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub(crate) struct NotePage {
    pub(crate) id: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) title: String,
    /// The day the page was started, as "2026-11-24".
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) day: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
}

impl Db {
    /// Adds a new page, named after today, and gives its id.
    pub(crate) fn notepad_new_page(&mut self) -> u32 {
        let id = self.notepad.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let day = today();
        self.notepad.push(NotePage { id, title: nice_day(&day), day, text: String::new() });
        id
    }
}

/// "2026-11-24" as "24 Nov 2026".
pub(crate) fn nice_day(d: &str) -> String {
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let p: Vec<&str> = d.split('-').collect();
    match (p.first().and_then(|y| y.parse::<u32>().ok()), p.get(1).and_then(|m| m.parse::<usize>().ok()), p.get(2).and_then(|d| d.parse::<u32>().ok())) {
        (Some(y), Some(m), Some(dd)) if (1..=12).contains(&m) => format!("{dd} {} {y}", months[m - 1]),
        _ => d.to_string(),
    }
}

/// The text to send: what is selected, or else the whole line the cursor is on.
pub(crate) fn line_at(text: &str, sel: Option<(usize, usize)>) -> String {
    let Some((a, b)) = sel else { return String::new() };
    let chars: Vec<char> = text.chars().collect();
    let (a, b) = (a.min(b).min(chars.len()), a.max(b).min(chars.len()));
    if a != b {
        return chars[a..b].iter().collect::<String>().trim().to_string();
    }
    let start = chars[..a].iter().rposition(|c| *c == '\n').map_or(0, |i| i + 1);
    let end = chars[a..].iter().position(|c| *c == '\n').map_or(chars.len(), |i| a + i);
    chars[start..end].iter().collect::<String>().trim().to_string()
}

impl App {
    pub(crate) fn notepad_window(&mut self, ctx: &egui::Context) {
        if !self.x.notepad_open {
            return;
        }
        let tr = crate::i18n::tr;
        if self.db.notepad.is_empty() {
            self.x.notepad_page = self.db.notepad_new_page();
            self.dirty = true;
        }
        if !self.db.notepad.iter().any(|p| p.id == self.x.notepad_page) {
            self.x.notepad_page = self.db.notepad.last().map(|p| p.id).unwrap_or(0);
        }
        let mut open = true;
        let mut changed = false;
        let sel = self.sel.filter(|s| self.db.people.contains_key(s));
        egui::Window::new(crate::look::wtitle(tr("Research notepad"))).id(egui::Id::new("notepad")).open(&mut open).default_size([520.0, 460.0]).min_width(360.0).show(ctx, |ui| {
            // the pages, as tabs
            ui.horizontal_wrapped(|ui| {
                let pages: Vec<(u32, String)> = self.db.notepad.iter().map(|p| (p.id, p.title.clone())).collect();
                for (id, t) in pages {
                    let t = if t.trim().is_empty() { tr("(untitled)").to_string() } else { t };
                    if ui.selectable_label(self.x.notepad_page == id, t).clicked() {
                        self.x.notepad_page = id;
                        self.x.notepad_confirm = false;
                    }
                }
                if ui.button(tr("+ New page")).on_hover_text(tr("Start a page for today")).clicked() {
                    self.x.notepad_page = self.db.notepad_new_page();
                    changed = true;
                }
            });
            ui.separator();
            let Some(ix) = self.db.notepad.iter().position(|p| p.id == self.x.notepad_page) else { return };
            ui.horizontal(|ui| {
                ui.label(tr("Title:"));
                changed |= ui.add(egui::TextEdit::singleline(&mut self.db.notepad[ix].title).desired_width(220.0)).changed();
                ui.label(RichText::new(crate::i18n::trf("Started {}", &["Started ", ""], &[nice_day(&self.db.notepad[ix].day)])).small().weak());
            });
            let out = egui::TextEdit::multiline(&mut self.db.notepad[ix].text)
                .id(egui::Id::new(("notepad_text", self.x.notepad_page)))
                .hint_text(tr("Jot things down as you research: what you looked at, names to follow up, call numbers..."))
                .desired_width(f32::INFINITY)
                .desired_rows(14)
                .show(ui);
            changed |= out.response.changed();
            if let Some(r) = out.cursor_range {
                self.x.notepad_cursor = Some((r.primary.ccursor.index, r.secondary.ccursor.index));
            }
            let text = self.db.notepad[ix].text.clone();
            let line = line_at(&text, self.x.notepad_cursor);
            ui.horizontal_wrapped(|ui| {
                if ui.small_button(tr("Insert today's date")).clicked() {
                    let at = self.x.notepad_cursor.map(|c| c.0.max(c.1)).unwrap_or(text.chars().count()).min(text.chars().count());
                    let byte = text.char_indices().nth(at).map_or(text.len(), |(b, _)| b);
                    self.db.notepad[ix].text.insert_str(byte, &format!("{} ", nice_day(&today())));
                    changed = true;
                }
                ui.separator();
                let who = sel.map(|s| self.db.label(s));
                let can = sel.is_some() && !line.is_empty();
                let hint = if sel.is_none() { tr("Select a person first") } else if line.is_empty() { tr("Put the cursor on a line, or select some text") } else { tr("Sends the line the cursor is on, or the text selected") };
                if ui.add_enabled(can, egui::Button::new(tr("Line to their notes")).small()).on_hover_text(hint).on_disabled_hover_text(hint).clicked() {
                    if let Some(p) = sel.and_then(|s| self.db.people.get_mut(&s)) {
                        if !p.notes.is_empty() && !p.notes.ends_with('\n') {
                            p.notes.push('\n');
                        }
                        p.notes.push_str(&line);
                        self.status = crate::i18n::trf("Added to {}'s notes.", &["Added to ", "'s notes."], &[who.clone().unwrap_or_default()]);
                        changed = true;
                    }
                }
                if ui.add_enabled(can, egui::Button::new(tr("Line to their to-do list")).small()).on_hover_text(hint).on_disabled_hover_text(hint).clicked() {
                    if let Some(p) = sel.and_then(|s| self.db.people.get_mut(&s)) {
                        p.todos.push(Todo { text: line.clone(), added: today(), ..Default::default() });
                        self.status = crate::i18n::trf("Added to {}'s to-do list.", &["Added to ", "'s to-do list."], &[who.clone().unwrap_or_default()]);
                        changed = true;
                    }
                }
                if let Some(w) = who {
                    ui.label(RichText::new(w).small().weak());
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Saved as you type, with your tree.")).small().weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.x.notepad_confirm {
                        if ui.small_button(tr("Cancel")).clicked() {
                            self.x.notepad_confirm = false;
                        }
                        if ui.small_button(tr("Delete")).clicked() {
                            let id = self.x.notepad_page;
                            self.db.notepad.retain(|p| p.id != id);
                            self.x.notepad_confirm = false;
                            changed = true;
                        }
                        ui.label(RichText::new(tr("Delete this page? Undo brings it back.")).small());
                    } else if ui.small_button(tr("Delete page...")).clicked() {
                        self.x.notepad_confirm = true;
                    }
                });
            });
        });
        if changed {
            self.dirty = true;
        }
        if !open {
            self.x.notepad_open = false;
            // a page left empty is not kept
            let before = self.db.notepad.len();
            self.db.notepad.retain(|p| !(p.text.trim().is_empty() && p.title == nice_day(&p.day)));
            if self.db.notepad.len() != before {
                self.dirty = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_days() {
        let t = "first line\nsecond line here\nthird";
        assert_eq!(line_at(t, Some((14, 14))), "second line here");
        assert_eq!(line_at(t, Some((0, 5))), "first");
        assert_eq!(line_at(t, Some((t.chars().count(), t.chars().count()))), "third");
        assert_eq!(line_at(t, None), "");
        assert_eq!(nice_day("2026-11-24"), "24 Nov 2026");
        assert_eq!(nice_day("someday"), "someday");
    }

    #[test]
    fn the_notepad_keeps_pages_with_the_tree_and_sends_lines() {
        let mut db = crate::sample::sample_family();
        let id = db.notepad_new_page();
        db.notepad[0].text = "Check the 1910 census for Buhl".into();
        let back: Db = serde_json::from_str(&serde_json::to_string(&db).unwrap()).unwrap();
        assert_eq!(back.notepad, db.notepad);
        let p = *db.people.keys().next().unwrap();
        let mut app = App::for_test(db);
        app.sel = Some(p);
        app.x.notepad_open = true;
        app.x.notepad_page = id;
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
        let _ = ctx.run(input, |ctx| app.notepad_window(ctx));
        assert_eq!(app.db.notepad.len(), 1);
    }
}
