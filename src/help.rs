//! The built-in User Guide.

use super::*;

pub(crate) const USER_GUIDE: &str = include_str!("../docs/USER_GUIDE.md");

/// Draw the user guide (simple Markdown: headings, bullets, **bold**), showing only the
/// sections that mention `find` when it is not empty.
pub(crate) fn help_text(ui: &mut egui::Ui, find: &str) {
    let q = find.trim().to_lowercase();
    let mut sections: Vec<(String, Vec<&str>)> = vec![(String::new(), Vec::new())];
    for line in USER_GUIDE.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            sections.push((h.to_string(), Vec::new()));
        } else if line.starts_with("# ") {
            continue;
        } else {
            sections.last_mut().unwrap().1.push(line);
        }
    }
    let mut shown = 0;
    for (head, body) in &sections {
        if !q.is_empty() && !head.to_lowercase().contains(&q) && !body.iter().any(|l| l.to_lowercase().contains(&q)) {
            continue;
        }
        if head.is_empty() && body.iter().all(|l| l.trim().is_empty()) {
            continue;
        }
        shown += 1;
        if !head.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(head).size(18.0).strong());
        }
        let mut para = String::new();
        let flush = |ui: &mut egui::Ui, para: &mut String| {
            if !para.trim().is_empty() {
                ui.add(egui::Label::new(RichText::new(para.replace("**", "").replace('`', "")).size(14.0)).wrap());
                ui.add_space(4.0);
            }
            para.clear();
        };
        for l in body {
            let t = l.trim();
            if t.is_empty() {
                flush(ui, &mut para);
            } else if let Some(b) = t.strip_prefix("- ") {
                flush(ui, &mut para);
                ui.horizontal_wrapped(|ui| {
                    ui.label("  \u{2022}");
                    ui.add(egui::Label::new(RichText::new(b.replace("**", "").replace('`', "")).size(14.0)).wrap());
                });
            } else {
                if !para.is_empty() {
                    para.push(' ');
                }
                para.push_str(t);
            }
        }
        flush(ui, &mut para);
    }
    if shown == 0 {
        ui.label(RichText::new("Nothing in the guide matches that.").italics().weak());
    }
}
