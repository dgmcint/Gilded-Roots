//! Special letters: a small window of accented letters and symbols that puts the one clicked
//! into the box you were typing in (for names and places in other languages).

use super::*;

/// Letters by group. Capitals are made from these when "Capitals" is ticked.
pub(crate) const LETTER_GROUPS: &[(&str, &str)] = &[
    ("Western Europe", "áàâäãåāæçéèêëēíìîïīñóòôöõøōœúùûüūýÿß"),
    ("Central and Eastern Europe", "ąćčďęěğıłńňőřśšşţțșťůűźżž"),
    ("Nordic and Old English", "åæøðþ"),
    ("Symbols used in records", "†*∞°~’“”–—…½¼¾№§"),
];

/// The capital form of a letter, where it has one.
pub(crate) fn capital(c: char) -> String {
    match c {
        // The capital ẞ is rare and missing from the screen font, so ß stays as it is.
        'ß' => "ß".to_string(),
        'ı' => "I".to_string(),
        _ => c.to_uppercase().collect(),
    }
}

impl App {
    /// Remember the text box being typed in, so a letter clicked in the window goes there.
    /// Called at the end of each frame.
    pub(crate) fn note_text_focus(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            if let Some(id) = ctx.memory(|m| m.focused()) {
                self.x.last_text_focus = Some(id);
            }
        }
    }

    /// Put a letter chosen last frame into the box that had the cursor. Called at the start of
    /// each frame, before anything is drawn.
    pub(crate) fn type_pending_letter(&mut self, ctx: &egui::Context) {
        if let Some((s, id)) = self.x.pending_letter.take() {
            ctx.memory_mut(|m| m.request_focus(id));
            ctx.input_mut(|i| i.events.push(egui::Event::Text(s)));
        }
    }

    pub(crate) fn letters_window(&mut self, ctx: &egui::Context) {
        if !self.x.letters_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Special letters").open(&mut open).default_width(330.0).resizable(false).show(ctx, |ui| {
            let target = self.x.last_text_focus;
            ui.label(
                RichText::new(if target.is_some() {
                    "Click a letter to put it where you were typing. It is also copied, so you can paste it elsewhere."
                } else {
                    "Click in a box first (a name or place), then click a letter. Letters are also copied, so you can paste them."
                })
                .small()
                .weak(),
            );
            ui.checkbox(&mut self.x.letters_caps, "Capitals");
            for (group, letters) in LETTER_GROUPS {
                ui.add_space(4.0);
                ui.label(RichText::new(*group).small().strong());
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                    for c in letters.chars() {
                        let s = if self.x.letters_caps && c.is_alphabetic() { capital(c) } else { c.to_string() };
                        let b = ui.add(egui::Button::new(RichText::new(&s).size(18.0)).min_size(egui::vec2(30.0, 30.0)));
                        if b.clicked() {
                            ui.ctx().copy_text(s.clone());
                            if let Some(id) = target {
                                self.x.pending_letter = Some((s.clone(), id));
                                ui.ctx().request_repaint();
                            }
                        }
                    }
                });
            }
        });
        self.x.letters_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_letter_has_a_capital_or_is_a_symbol() {
        for (_, letters) in LETTER_GROUPS {
            for c in letters.chars() {
                let cap = capital(c);
                assert!(!cap.is_empty());
                if c.is_lowercase() && c != 'ß' {
                    assert_ne!(cap, c.to_string(), "{c} should have a capital");
                }
            }
        }
        assert_eq!(capital('ñ'), "Ñ");
        assert_eq!(capital('ß'), "ß");
    }
}
