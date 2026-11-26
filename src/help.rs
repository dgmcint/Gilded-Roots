//! The built-in User Guide (Help > User guide, F1): the guide's parts and chapters listed on the
//! left, the text on the right, a search box that keeps only the chapters that mention a word,
//! and the guide's pictures.
//!
//! The guide is docs/USER_GUIDE.md, a simple Markdown file that also reads well on GitHub:
//! `# Part`, `## Chapter`, paragraphs, `- ` bullets, `1. ` numbered steps, `**bold**`,
//! `![caption](images/guide/file.png)` pictures, and `See also:` lines whose `[name](#chapter)`
//! links jump to another chapter.

use super::*;
use std::sync::OnceLock;

pub(crate) const USER_GUIDE: &str = include_str!("../docs/USER_GUIDE.md");

/// The guide's pictures, built into the program so the guide works offline.
const PICTURES: &[(&str, &[u8])] = &[
    ("images/guide/welcome.png", include_bytes!("../docs/images/guide/welcome.png")),
    ("images/guide/screen.png", include_bytes!("../docs/images/guide/screen.png")),
    ("images/guide/tour.png", include_bytes!("../docs/images/guide/tour.png")),
    ("images/guide/add-person.png", include_bytes!("../docs/images/guide/add-person.png")),
    ("images/guide/family.png", include_bytes!("../docs/images/guide/family.png")),
    ("images/guide/editing.png", include_bytes!("../docs/images/guide/editing.png")),
    ("images/guide/memories.png", include_bytes!("../docs/images/guide/memories.png")),
];

pub(crate) struct Chapter {
    pub(crate) part: usize,
    pub(crate) title: String,
    pub(crate) lines: Vec<&'static str>,
}

pub(crate) struct Guide {
    pub(crate) parts: Vec<String>,
    pub(crate) chapters: Vec<Chapter>,
}

/// The guide, split into parts and chapters (read once).
pub(crate) fn guide() -> &'static Guide {
    static G: OnceLock<Guide> = OnceLock::new();
    G.get_or_init(|| {
        let mut parts: Vec<String> = Vec::new();
        let mut chapters: Vec<Chapter> = Vec::new();
        let mut first_heading = true;
        for line in USER_GUIDE.lines() {
            if let Some(h) = line.strip_prefix("## ") {
                chapters.push(Chapter { part: parts.len().saturating_sub(1), title: h.trim().to_string(), lines: Vec::new() });
            } else if let Some(h) = line.strip_prefix("# ") {
                // the first heading is the guide's own title
                if first_heading {
                    first_heading = false;
                    continue;
                }
                parts.push(h.trim().to_string());
            } else if let Some(c) = chapters.last_mut() {
                c.lines.push(line);
            }
        }
        if parts.is_empty() {
            parts.push(String::new());
        }
        Guide { parts, chapters }
    })
}

/// A chapter's name as GitHub makes it into a link (`#your-first-family-tree`).
pub(crate) fn slug(title: &str) -> String {
    let mut s = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_alphanumeric() {
            s.push(c);
        } else if c == ' ' || c == '-' {
            s.push('-');
        }
    }
    s
}

/// The chapter a `#link` points at.
pub(crate) fn chapter_for(link: &str) -> Option<usize> {
    let want = link.trim_start_matches('#');
    guide().chapters.iter().position(|c| slug(&c.title) == want)
}

fn matches(c: &Chapter, q: &str) -> bool {
    q.is_empty() || c.title.to_lowercase().contains(q) || c.lines.iter().any(|l| l.to_lowercase().contains(q))
}

/// Text with `**bold**`, `code` and `[links](...)` turned into one wrapped label.
fn rich(ui: &egui::Ui, text: &str, size: f32) -> egui::text::LayoutJob {
    // the program's font has no arrow, so menu paths are written the way the menus show them
    let text = text.replace(" → ", " > ").replace('→', ">");
    let text = text.as_str();
    let mut job = egui::text::LayoutJob::default();
    let normal = ui.visuals().text_color();
    let strong = ui.visuals().strong_text_color();
    let link = ui.visuals().hyperlink_color;
    let mut bold = false;
    let mut rest = text;
    let fmt = |c: Color32, mono: bool| egui::TextFormat { font_id: if mono { FontId::monospace(size - 1.0) } else { FontId::proportional(size) }, color: c, ..Default::default() };
    while !rest.is_empty() {
        // the next marker: ** (bold), ` (code) or [ (link)
        let next = ["**", "`", "["].iter().filter_map(|m| rest.find(m).map(|i| (i, *m))).min_by_key(|x| x.0);
        let Some((i, m)) = next else {
            job.append(rest, 0.0, fmt(if bold { strong } else { normal }, false));
            break;
        };
        if i > 0 {
            job.append(&rest[..i], 0.0, fmt(if bold { strong } else { normal }, false));
        }
        rest = &rest[i..];
        match m {
            "**" => {
                bold = !bold;
                rest = &rest[2..];
            }
            "`" => {
                let end = rest[1..].find('`').map(|e| e + 1).unwrap_or(rest.len());
                job.append(&rest[1..end.min(rest.len())], 0.0, fmt(strong, true));
                rest = if end < rest.len() { &rest[end + 1..] } else { "" };
            }
            _ => {
                // [text](target): the text, in the link colour
                let close = rest.find("](").and_then(|a| rest[a..].find(')').map(|b| (a, a + b)));
                match close {
                    Some((a, b)) => {
                        job.append(&rest[1..a], 0.0, fmt(link, false));
                        rest = &rest[b + 1..];
                    }
                    _ => {
                        job.append("[", 0.0, fmt(normal, false));
                        rest = &rest[1..];
                    }
                }
            }
        }
    }
    job.wrap.max_width = ui.available_width();
    job
}

/// The `[text](#chapter)` links in a line: (text, chapter).
fn links(line: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(a) = rest.find('[') {
        let Some(b) = rest[a..].find("](").map(|b| a + b) else { break };
        let Some(c) = rest[b..].find(')').map(|c| b + c) else { break };
        let text = &rest[a + 1..b];
        let target = &rest[b + 2..c];
        if target.starts_with('#') {
            if let Some(ch) = chapter_for(target) {
                out.push((text.to_string(), ch));
            }
        }
        rest = &rest[c + 1..];
    }
    out
}

fn picture(ui: &mut egui::Ui, path: &str, caption: &str) {
    let Some((_, bytes)) = PICTURES.iter().find(|(p, _)| *p == path) else { return };
    // the picture's own size, from the PNG header
    let dim = |i: usize| bytes.get(i..i + 4).map_or(1.0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]).max(1) as f32);
    let (pw, ph) = (dim(16), dim(20));
    let w = ui.available_width().min(760.0).min(pw);
    ui.add_space(4.0);
    ui.add(egui::Image::from_bytes(format!("bytes://guide/{}", path), *bytes).fit_to_exact_size(vec2(w, w * ph / pw)).rounding(6.0));
    if !caption.is_empty() {
        ui.label(RichText::new(caption).size(12.5).italics().weak());
    }
    ui.add_space(6.0);
}

impl App {
    /// The User guide window.
    pub(crate) fn help_window(&mut self, ctx: &egui::Context) {
        if !self.help_open {
            return;
        }
        let g = guide();
        let mut open = true;
        let mut find = self.help_find.clone();
        let mut goto = self.x.help_goto.take();
        let mut tour = false;
        crate::look::window(crate::look::wtitle(crate::i18n::tr("User guide"))).open(&mut open).default_size([880.0, 640.0]).min_width(520.0).show(ctx, |ui| {
            let q = find.trim().to_lowercase();
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr("Search"));
                ui.add(egui::TextEdit::singleline(&mut find).hint_text(crate::i18n::tr("e.g. merge, sources, home person")).desired_width(240.0));
                if !find.is_empty() && ui.small_button(crate::i18n::tr("Clear")).clicked() {
                    find.clear();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::tr("Guided tour")).on_hover_text(crate::i18n::tr("Step-by-step lessons on the sample family")).clicked() {
                        tour = true;
                    }
                });
            });
            ui.separator();
            let wide = ui.available_width() > 680.0;
            let h = ui.available_height();
            ui.horizontal_top(|ui| {
                // Contents
                if wide {
                    ui.vertical(|ui| {
                        ui.set_width(210.0);
                        egui::ScrollArea::vertical().id_salt("guide_toc").max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                            for (pi, part) in g.parts.iter().enumerate() {
                                let chs: Vec<usize> = (0..g.chapters.len()).filter(|&i| g.chapters[i].part == pi && matches(&g.chapters[i], &q)).collect();
                                if chs.is_empty() {
                                    continue;
                                }
                                if !part.is_empty() {
                                    ui.add_space(6.0);
                                    ui.label(RichText::new(part.as_str()).size(13.0).strong());
                                }
                                for i in chs {
                                    if ui.add(egui::Label::new(RichText::new(g.chapters[i].title.as_str()).size(13.0).color(ui.visuals().hyperlink_color)).sense(Sense::click()).wrap()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        goto = Some(i);
                                    }
                                }
                            }
                        });
                    });
                    ui.separator();
                }
                // The text
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical().id_salt("guide_text").max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                        let mut shown = 0;
                        let mut last_part = usize::MAX;
                        for (ci, c) in g.chapters.iter().enumerate() {
                            if !matches(c, &q) {
                                continue;
                            }
                            shown += 1;
                            if c.part != last_part {
                                last_part = c.part;
                                if let Some(p) = g.parts.get(c.part).filter(|p| !p.is_empty()) {
                                    ui.add_space(14.0);
                                    ui.label(RichText::new(p.as_str()).size(22.0).strong().color(Color32::from_rgb(232, 178, 40)));
                                }
                            }
                            ui.add_space(10.0);
                            let head = ui.label(RichText::new(c.title.as_str()).size(18.0).strong());
                            if goto == Some(ci) {
                                head.scroll_to_me(Some(egui::Align::TOP));
                                goto = None;
                            }
                            chapter_body(ui, c, &mut goto);
                        }
                        if shown == 0 {
                            ui.label(RichText::new(crate::i18n::tr("Nothing in the guide matches that.")).italics().weak());
                        }
                    });
                });
            });
        });
        // a link was clicked: show that chapter (next frame, once the search no longer hides it)
        if let Some(gt) = goto {
            if !matches(&g.chapters[gt], &find.trim().to_lowercase()) {
                find.clear();
            }
            self.x.help_goto = Some(gt);
            ctx.request_repaint();
        }
        self.help_find = find;
        self.help_open = open;
        if tour {
            self.x.tour_list = true;
        }
    }
}

/// One block of a chapter: a paragraph, a bullet, a numbered step, a picture or a See also line.
enum Block {
    Para(String),
    Bullet(String),
    Step(String, String),
    Picture(String, String),
    SeeAlso(String),
}

/// Splits a chapter into blocks. A list item carries on over the indented lines under it.
fn blocks(lines: &[&str]) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut open = false; // the last block can take more lines
    for l in lines {
        let t = l.trim();
        if t.is_empty() {
            open = false;
            continue;
        }
        let indented = l.starts_with("  ");
        if let Some(img) = t.strip_prefix("![") {
            if let (Some(a), Some(b)) = (img.find("]("), img.rfind(')')) {
                out.push(Block::Picture(img[a + 2..b].to_string(), img[..a].to_string()));
            }
            open = false;
        } else if let Some(rest) = t.strip_prefix("See also:") {
            out.push(Block::SeeAlso(rest.to_string()));
            open = false;
        } else if let Some(b) = t.strip_prefix("- ") {
            out.push(Block::Bullet(b.to_string()));
            open = true;
        } else if let Some((n, b)) = t.split_once(". ").filter(|(n, _)| !n.is_empty() && n.len() <= 2 && n.chars().all(|c| c.is_ascii_digit())) {
            out.push(Block::Step(n.to_string(), b.to_string()));
            open = true;
        } else {
            match out.last_mut() {
                Some(Block::Bullet(x)) | Some(Block::Step(_, x)) if open && indented => {
                    x.push(' ');
                    x.push_str(t);
                }
                Some(Block::Para(x)) if open => {
                    x.push(' ');
                    x.push_str(t);
                }
                _ => {
                    out.push(Block::Para(t.to_string()));
                    open = true;
                }
            }
        }
    }
    out
}

/// Draws one chapter's text.
fn chapter_body(ui: &mut egui::Ui, c: &Chapter, goto: &mut Option<usize>) {
    for b in blocks(&c.lines) {
        match b {
            Block::Para(t) => {
                ui.add_space(2.0);
                let job = rich(ui, &t, 14.0);
                ui.add(egui::Label::new(job).wrap());
                ui.add_space(4.0);
            }
            Block::Bullet(t) => {
                ui.horizontal_top(|ui| {
                    ui.add_sized([18.0, 18.0], egui::Label::new(RichText::new("\u{2022}").size(14.0)));
                    let job = rich(ui, &t, 14.0);
                    ui.add(egui::Label::new(job).wrap());
                });
            }
            Block::Step(n, t) => {
                ui.add_space(2.0);
                ui.horizontal_top(|ui| {
                    ui.add_sized([24.0, 20.0], egui::Label::new(RichText::new(format!("{}.", n)).size(15.0).strong().color(Color32::from_rgb(232, 178, 40))));
                    let job = rich(ui, &t, 14.0);
                    ui.add(egui::Label::new(job).wrap());
                });
            }
            Block::Picture(path, caption) => picture(ui, &path, &caption),
            Block::SeeAlso(rest) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(crate::i18n::tr("See also:")).size(13.0).weak());
                    for (text, ch) in links(&rest) {
                        if ui.link(RichText::new(text).size(13.0)).clicked() {
                            *goto = Some(ch);
                        }
                    }
                });
                ui.add_space(4.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_has_parts_chapters_working_links_and_its_pictures() {
        let g = guide();
        assert!(g.parts.len() >= 5, "parts: {:?}", g.parts);
        assert!(g.chapters.len() > 40);
        // every #link names a real chapter, and every picture is built in
        for c in &g.chapters {
            for l in &c.lines {
                let mut rest = *l;
                while let Some(i) = rest.find("](#") {
                    let end = rest[i..].find(')').map(|e| i + e).unwrap();
                    let target = &rest[i + 2..end];
                    assert!(chapter_for(target).is_some(), "broken link {} in {}", target, c.title);
                    rest = &rest[end..];
                }
                if let Some(img) = l.trim().strip_prefix("![") {
                    let a = img.find("](").unwrap();
                    let path = &img[a + 2..img.rfind(')').unwrap()];
                    assert!(PICTURES.iter().any(|(p, _)| *p == path), "picture {} is not built in", path);
                }
            }
        }
        // every built-in picture is used
        for (p, _) in PICTURES {
            assert!(USER_GUIDE.contains(p), "{} is never shown", p);
        }
        assert_eq!(slug("Your first family tree, step by step"), "your-first-family-tree-step-by-step");
        // list items carry on over their indented lines
        let b = blocks(&["- one", "  two", "next para", "", "1. step", "   more", "plain"]);
        assert!(matches!(&b[0], Block::Bullet(t) if t == "one two"));
        assert!(matches!(&b[1], Block::Para(t) if t == "next para"));
        assert!(matches!(&b[2], Block::Step(n, t) if n == "1" && t == "step more"));
        assert!(matches!(&b[3], Block::Para(t) if t == "plain"));
    }

    #[test]
    fn the_guide_window_draws_searches_and_jumps() {
        let mut app = App::for_test(crate::sample::sample_family());
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
        app.help_open = true;
        for (find, go) in [("", None), ("merge", None), ("zzzz no such words", None), ("", Some(10))] {
            app.help_find = find.to_string();
            app.x.help_goto = go;
            let _ = ctx.run(input(), |c| app.frame(c));
            let _ = ctx.run(input(), |c| app.frame(c));
        }
        assert!(app.help_open);
    }
}
