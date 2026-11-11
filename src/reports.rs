//! Printable pages and reports (HTML).

use super::*;

pub(crate) fn parse_md(d: &str) -> Option<(u32, u32)> {
    const M: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    let toks: Vec<&str> = d.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
    if toks.len() >= 3 && toks[0].len() == 4 && toks[0].chars().all(|c| c.is_ascii_digit()) {
        let m: u32 = toks[1].parse().ok()?;
        let dd: u32 = toks[2].parse().ok()?;
        if (1..=12).contains(&m) && (1..=31).contains(&dd) {
            return Some((m, dd));
        }
    }
    let mut month: Option<u32> = None;
    let mut day: Option<u32> = None;
    for t in &toks {
        let l = t.to_lowercase();
        if month.is_none() && l.len() >= 3 {
            if let Some(i) = M.iter().position(|m| l.starts_with(m)) {
                month = Some(i as u32 + 1);
                continue;
            }
        }
        if day.is_none() && t.len() <= 2 {
            if let Ok(n) = t.parse::<u32>() {
                if (1..=31).contains(&n) {
                    day = Some(n);
                }
            }
        }
    }
    Some((month?, day?))
}

pub(crate) fn ancestor_slots(db: &Db, root: u32, gens: usize) -> Vec<Vec<Option<u32>>> {
    let mut slots: Vec<Vec<Option<u32>>> = vec![vec![Some(root)]];
    for g in 1..gens {
        let cur = {
            let prev = &slots[g - 1];
            let mut c = Vec::new();
            for s in prev {
                let (f, m) = match s {
                    Some(i) => db.parents(*i),
                    None => (None, None),
                };
                c.push(f);
                c.push(m);
            }
            c
        };
        slots.push(cur);
    }
    slots
}

pub(crate) fn desc_layout(db: &Db, ix: &Idx, id: u32, depth: usize, max: usize, parent: Option<usize>, nodes: &mut Vec<(u32, usize, f32, Option<usize>)>, leaf: &mut f32, seen: &mut HashSet<u32>) -> f32 {
    let idx = nodes.len();
    nodes.push((id, depth, 0.0, parent));
    let mut kids: Vec<u32> = Vec::new();
    if depth + 1 < max {
        for f in db.fams_of_ix(ix, id) {
            for k in &db.fams[f].kids {
                if db.people.contains_key(k) && !seen.contains(k) && !kids.contains(k) {
                    kids.push(*k);
                }
            }
        }
    }
    let y = if kids.is_empty() {
        let y = *leaf;
        *leaf += 1.0;
        y
    } else {
        seen.extend(kids.iter().cloned());
        let mut first = 0.0f32;
        let mut last = 0.0f32;
        for (n, k) in kids.iter().enumerate() {
            let ky = desc_layout(db, ix, *k, depth + 1, max, Some(idx), nodes, leaf, seen);
            if n == 0 {
                first = ky;
            }
            last = ky;
        }
        (first + last) / 2.0
    };
    nodes[idx].2 = y;
    y
}

pub(crate) fn desc_ul(db: &Db, id: u32, depth: usize, max: usize, seen: &mut Vec<u32>, out: &mut String) {
    let p = &db.people[&id];
    let mut spouse = String::new();
    for f in db.fams_of(id) {
        let fm = &db.fams[&f];
        let sp = if fm.h == Some(id) { fm.w } else { fm.h };
        if let Some(sid) = sp {
            spouse.push_str(&format!("<br><small>m. {} {}</small>", dp(&fm.md, &fm.mp), esc(&db.people[&sid].name)));
        }
    }
    out.push_str(&format!("<li><b>{}</b> {}{}", esc(&display_name(p)), esc(&life(p)), spouse));
    let mut kids: Vec<u32> = Vec::new();
    if depth + 1 < max {
        for f in db.fams_of(id) {
            for k in &db.fams[&f].kids {
                if !seen.contains(k) && !kids.contains(k) {
                    kids.push(*k);
                }
            }
        }
    }
    if !kids.is_empty() {
        seen.extend(kids.iter().cloned());
        out.push_str("<ul>");
        for k in kids {
            desc_ul(db, k, depth + 1, max, seen, out);
        }
        out.push_str("</ul>");
    }
    out.push_str("</li>");
}

pub(crate) fn current_year() -> i32 {
    today().get(0..4).and_then(|y| y.parse().ok()).unwrap_or(2026)
}

pub(crate) fn parse_date(s: &str) -> (Option<i32>, Option<u32>, Option<u32>) {
    const M: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    let l = s.to_lowercase();
    let (mut y, mut m, mut d): (Option<i32>, Option<u32>, Option<u32>) = (None, None, None);
    for tok in l.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()) {
        if tok.len() == 4 && tok.chars().all(|c| c.is_ascii_digit()) {
            if y.is_none() {
                y = tok.parse().ok();
            }
        } else if tok.len() <= 2 && tok.chars().all(|c| c.is_ascii_digit()) {
            if d.is_none() {
                d = tok.parse().ok();
            }
        } else if tok.len() >= 3 && m.is_none() {
            if let Some(i) = M.iter().position(|mm| tok.starts_with(mm)) {
                m = Some(i as u32 + 1);
            }
        }
    }
    (y, m, d)
}

/// Two lines for a box in the descendants chart: the person, then (below) their partner(s).
pub(crate) fn desc_lines(db: &Db, id: u32) -> (String, Option<String>) {
    let Some(p) = db.people.get(&id) else { return (String::new(), None) };
    let l = life(p);
    let first = if l.is_empty() { display_name(p) } else { format!("{}  {}", display_name(p), l) };
    let mut partners: Vec<String> = Vec::new();
    for f in db.fams_of(id) {
        let fm = &db.fams[&f];
        let sp = if fm.h == Some(id) { fm.w } else { fm.h };
        if let Some(q) = sp.and_then(|sid| db.people.get(&sid)) {
            let ql = life(q);
            partners.push(if ql.is_empty() { format!("+ {}", styled_name(q)) } else { format!("+ {}  {}", styled_name(q), ql) });
        }
    }
    let second = if partners.is_empty() { None } else { Some(partners.join("   ")) };
    (first, second)
}

/// Edit a list of citations. Each one points at a source record (shared by everyone who cites it)
/// or holds typed text, and carries its own page, quality, transcription, web address and access date.
pub(crate) fn cite_list(
    ui: &mut egui::Ui,
    id_key: impl std::hash::Hash + Copy,
    cites: &mut Vec<Source>,
    about: Option<&[&str]>,
    new_about: &str,
    sources: &mut BTreeMap<u32, SourceRec>,
    next: &mut u32,
    open_src: &mut Option<u32>,
) {
    let mut titles: Vec<(u32, String)> = sources.iter().map(|(k, s)| (*k, source_title(s))).collect();
    titles.sort_by_key(|t| t.1.to_lowercase());
    let mut remove: Option<usize> = None;
    let mut new_for: Option<usize> = None;
    for (i, c) in cites.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if let Some(list) = about {
                egui::ComboBox::from_id_salt((id_key, i, "about")).selected_text(if c.about.is_empty() { "General" } else { c.about.as_str() }).width(100.0).show_ui(ui, |ui| {
                    for f in list {
                        ui.selectable_value(&mut c.about, f.to_string(), *f);
                    }
                });
            }
            let shown = match c.sid.and_then(|s| titles.iter().find(|t| t.0 == s)) {
                Some(t) => t.1.clone(),
                None if c.text.is_empty() => "Choose a source...".to_string(),
                None => format!("{} (typed)", c.text),
            };
            egui::ComboBox::from_id_salt((id_key, i, "src")).selected_text(shown).width(300.0).show_ui(ui, |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    if ui.selectable_label(false, "+ New source...").clicked() {
                        new_for = Some(i);
                    }
                    if ui.selectable_label(c.sid.is_none(), "Just type the source name").clicked() {
                        if let Some(t) = c.sid.and_then(|s| titles.iter().find(|t| t.0 == s)) {
                            c.text = t.1.clone();
                        }
                        c.sid = None;
                    }
                    ui.separator();
                    for (id, t) in &titles {
                        if ui.selectable_label(c.sid == Some(*id), t.as_str()).clicked() {
                            c.sid = Some(*id);
                            c.text = t.clone();
                        }
                    }
                });
            });
            if let Some(s) = c.sid.filter(|s| sources.contains_key(s)) {
                if ui.small_button("Open source").on_hover_text("See or edit the source itself; changes apply everywhere it is cited").clicked() {
                    *open_src = Some(s);
                }
            }
            if ui.small_button("Remove").clicked() {
                remove = Some(i);
            }
        });
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            if c.sid.is_none() {
                ui.add(egui::TextEdit::singleline(&mut c.text).hint_text("source name, e.g. 1940 US Census").desired_width(230.0));
            }
            ui.add(egui::TextEdit::singleline(&mut c.page).hint_text("page, line or entry").desired_width(200.0));
            egui::ComboBox::from_id_salt((id_key, i, "quay")).selected_text(quality_label(&c.quality)).width(170.0).show_ui(ui, |ui| {
                for q in ["", "0", "1", "2", "3"] {
                    ui.selectable_value(&mut c.quality, q.to_string(), quality_label(q));
                }
            });
        });
        let has_more = !c.data.is_empty() || !c.url.is_empty() || !c.accessed.is_empty() || !c.note.is_empty();
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            egui::CollapsingHeader::new(if has_more { "Transcription, web address, notes (filled in)" } else { "Transcription, web address, notes" })
                .id_salt((id_key, i, "more"))
                .show(ui, |ui| {
                    ui.label("Text from the source (copy it exactly as written)");
                    ui.add(egui::TextEdit::multiline(&mut c.data).desired_rows(3).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        ui.label("Date of the entry");
                        ui.add(egui::TextEdit::singleline(&mut c.ddate).desired_width(110.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Web address of this record");
                        ui.add(egui::TextEdit::singleline(&mut c.url).hint_text("https://...").desired_width(260.0));
                        if is_url(&c.url) {
                            ui.hyperlink_to("Open", c.url.clone());
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Looked at on");
                        ui.add(egui::TextEdit::singleline(&mut c.accessed).hint_text("e.g. 5 Jan 2024").desired_width(130.0));
                    });
                    ui.label("Your notes on this citation");
                    ui.add(egui::TextEdit::multiline(&mut c.note).desired_rows(2).desired_width(f32::INFINITY));
                });
        });
        ui.add_space(4.0);
    }
    if let Some(i) = remove {
        cites.remove(i);
    }
    if let Some(i) = new_for {
        *next += 1;
        let id = *next;
        sources.insert(id, SourceRec::default());
        if let Some(c) = cites.get_mut(i) {
            c.sid = Some(id);
            c.text = String::new();
        }
        *open_src = Some(id);
    }
    if ui.button("Add source").clicked() {
        cites.push(Source { about: new_about.to_string(), ..Default::default() });
    }
}

/// A collapsible "Sources (n)" block for one fact or event.
pub(crate) fn cites_section(
    ui: &mut egui::Ui,
    id_key: impl std::hash::Hash + Copy,
    cites: &mut Vec<Source>,
    sources: &mut BTreeMap<u32, SourceRec>,
    next: &mut u32,
    open_src: &mut Option<u32>,
) {
    egui::CollapsingHeader::new(format!("Sources ({})", cites.len())).id_salt((id_key, "sec")).show(ui, |ui| {
        cite_list(ui, id_key, cites, None, "", sources, next, open_src);
    });
}

pub(crate) fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub(crate) fn dp(d: &str, p: &str) -> String {
    let d = print_date(d);
    let v: Vec<&str> = [d.as_str(), p.trim()].into_iter().filter(|s| !s.is_empty()).collect();
    esc(&v.join(", "))
}

pub(crate) fn page(title: &str, body: &str, landscape: bool) -> String {
    page_with_css(title, body, landscape, "", true)
}

pub(crate) fn page_with_css(title: &str, body: &str, landscape: bool, extra: &str, show_title: bool) -> String {
    // One look for every printed page. Long text (web addresses in sources) wraps instead of
    // pushing the right edge off the paper; tables have fixed column widths that fit the page.
    let css = "*{box-sizing:border-box}\
        html,body{margin:0;padding:0}\
        body{font-family:Georgia,'Times New Roman',serif;color:#1a1a1a;font-size:12.5px;line-height:1.35;padding:14px 18px;max-width:100%;overflow-wrap:anywhere;word-break:break-word}\
        h1{font-size:21px;margin:0 0 2px;letter-spacing:.2px}\
        .sub{color:#555;font-size:12px;margin:0 0 10px}\
        .rule{border:0;border-top:2px solid #6b5a2e;margin:6px 0 12px}\
        h2{font-size:15px;margin:16px 0 6px;color:#3d3420;border-bottom:1px solid #c9bf9f;padding-bottom:2px}\
        h3{font-size:13px;margin:12px 0 4px;color:#3d3420}\
        table{border-collapse:collapse;width:100%;table-layout:fixed;margin:0 0 10px}\
        th,td{border:1px solid #b9b2a0;padding:3px 6px;text-align:left;vertical-align:top;font-size:12px;overflow-wrap:anywhere}\
        th{background:#efe9d8;font-weight:bold;-webkit-print-color-adjust:exact;print-color-adjust:exact}\
        td.l{color:#555;width:16%}\
        tr.who th{background:#e2d8bb;font-size:13px}\
        td.empty{color:#aaa}\
        svg{width:100%;height:auto;max-height:82vh;display:block;margin:0 auto}\
        ol.src,ul{margin:2px 0 8px 0;padding-left:20px}\
        ol.src li,li{margin:2px 0;font-size:11.5px}\
        .note{white-space:pre-wrap;font-size:12px;margin:2px 0 8px}\
        .sec{break-inside:avoid;page-break-inside:avoid}\
        .foot{margin-top:14px;color:#777;font-size:10.5px;border-top:1px solid #ddd;padding-top:4px}\
        .noprint{margin-bottom:10px;font-family:system-ui,sans-serif}\
        .noprint button{font-size:14px;padding:6px 14px}\
        @media print{.noprint{display:none}body{padding:0}svg{max-height:150mm}.foot{display:none}}\
        @media print{body.port{width:7.25in}body.land{width:9.75in}}\
        @media print and (orientation:portrait){body.land{width:7.25in}}\
        .safari{display:none;background:#fff6d6;border:1px solid #e0c97a;padding:6px 10px;margin:8px 0;font-size:13px}";
    let heading = if show_title { format!("<h1>{}</h1><hr class=\"rule\">", esc(title)) } else { String::new() };
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{t}</title><style>{css}{extra}@page{{size:letter {o};margin:12mm 12mm 14mm 12mm;@bottom-left{{content:'{t2}';font:9px Georgia,serif;color:#888}}@bottom-center{{content:'Prepared {d} with Gilded Roots';font:9px Georgia,serif;color:#888}}@bottom-right{{content:'Page ' counter(page) ' of ' counter(pages);font:9px Georgia,serif;color:#888}}}}</style></head><body class=\"{bc}\"><div class=\"noprint\"><button onclick=\"window.print()\">Print or save as PDF</button><div class=\"safari\" id=\"grSafari\">Printing from Safari? If the right side is cut off, lower <b>Scale</b> in the print window (try 85%){lt}. Chrome and Firefox fit the page by themselves.</div></div>{h}{body}<div class=\"foot\">Prepared {d} with Gilded Roots</div><script>{fit}window.onload=function(){{setTimeout(function(){{window.print()}},500)}}</script></body></html>",
        t = esc(title),
        t2 = esc(title).replace('\'', "\\'"),
        css = css,
        extra = extra,
        o = if landscape { "landscape" } else { "portrait" },
        // No shrink-to-fit script: every page is laid out to the paper's width with CSS (tables and
        // charts are 100% wide), and a script measuring the page while printing misjudged it and
        // shrank pages that already fitted.
        // Safari prints at 100% without shrinking to the paper; the page widths in @media print above
        // keep it on the paper, and Safari users get a tip in case their printer's margins are wider.
        fit = "if(/^((?!chrome|chromium|crios|fxios|edg|android).)*safari/i.test(navigator.userAgent)){var s=document.getElementById('grSafari');if(s)s.style.display='block';}",
        bc = if landscape { "land" } else { "port" },
        lt = if landscape { ", and choose Landscape" } else { "" },
        h = heading,
        body = body,
        d = {
            let t = today();
            let (y, m, dd) = (&t[..4], t[5..7].parse::<usize>().unwrap_or(1), t[8..10].trim_start_matches('0').to_string());
            const MN: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
            format!("{} {} {}", dd, MN[m.clamp(1, 12) - 1], y)
        }
    )
}

/// A date on paper: the chosen date style, or "12 Mar 1921" when dates are shown as typed,
/// so a printed sheet is tidy even when dates were typed in different ways.
pub(crate) fn print_date(s: &str) -> String {
    let st = DATE_STYLE.load(Ordering::Relaxed);
    show_date_as(s, if st == 0 { 1 } else { st })
}

impl App {
pub(crate)     fn open_print(&mut self, html: String) {
        // kept in the program's own folder: a shared temporary folder could be tampered with
        let dir = data_dir().join("print");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("gilded-roots-print.html");
        match std::fs::write(&path, html) {
            Ok(_) => {
                open_path(&path);
                self.status = "Opened a print page in your browser. Use its Print button, or choose Save as PDF.".to_string();
            }
            Err(e) => self.status = format!("Could not make the print page: {}", e),
        }
    }

pub(crate)     fn print_db(&self) -> Db {
        if self.settings.hide_living {
            self.db.sanitized()
        } else {
            self.db.clone()
        }
    }

    /// One spouse's block on a family group sheet: name, the four life events with date and
    /// place in their own columns, and their parents.
pub(crate)     fn person_rows(db: &Db, role: &str, id: Option<u32>) -> String {
        let cell = |s: &str| if s.trim().is_empty() { "<td class=\"empty\"></td>".to_string() } else { format!("<td>{}</td>", esc(s.trim())) };
        match id {
            Some(i) => {
                let p = &db.people[&i];
                let mut r = format!("<tr class=\"who\"><th>{}</th><th colspan=\"2\">{}</th></tr>", role, esc(&display_name(p)));
                for (l, d, pl) in [("Born", &p.bd, &p.bp), (if p.chtag == "BAPM" { "Baptised" } else { "Christened" }, &p.chd, &p.chp), ("Died", &p.dd, &p.dp), ("Buried", &p.bud, &p.bup)] {
                    r.push_str(&format!("<tr><td class=\"l\">{}</td>{}{}</tr>", l, cell(&print_date(d)), cell(pl)));
                }
                let (f, m) = db.parents(i);
                let nm = |x: Option<u32>| x.and_then(|x| db.people.get(&x)).map(display_name).unwrap_or_default();
                r.push_str(&format!("<tr><td class=\"l\">Father</td><td colspan=\"2\">{}</td></tr>", esc(&nm(f))));
                r.push_str(&format!("<tr><td class=\"l\">Mother</td><td colspan=\"2\">{}</td></tr>", esc(&nm(m))));
                r
            }
            None => format!("<tr class=\"who\"><th>{}</th><th colspan=\"2\" style=\"font-weight:normal;color:#777\">not recorded</th></tr>", role),
        }
    }

pub(crate)     fn family_html(&self, id: u32) -> String {
        let db = self.print_db();
        let name = db.people[&id].name.clone();
        page_with_css(&format!("Family Group Sheet: {}", name), &Self::family_body(&db, id), false, "", false)
    }

pub(crate)     fn family_body(db: &Db, id: u32) -> String {
        let mut ids = db.fams_of(id);
        if ids.is_empty() {
            if let Some(f) = db.famc(id) {
                ids.push(f);
            }
        }
        let name = db.people[&id].name.clone();
        if ids.is_empty() {
            return format!("<h1>Family Group Sheet</h1><hr class=\"rule\"><p>{} has no family group yet.</p>", esc(&name));
        }
        let mut body = String::new();
        let colgroup = "<colgroup><col style=\"width:16%\"><col style=\"width:30%\"><col style=\"width:54%\"></colgroup>";
        for (n, fid) in ids.iter().enumerate() {
            let f = &db.fams[fid];
            let couple: Vec<String> = [f.h, f.w].into_iter().flatten().filter_map(|x| db.people.get(&x)).map(display_name).collect();
            if n > 0 {
                body.push_str("<div style=\"break-before:page;page-break-before:always\"></div>");
            }
            body.push_str("<h1>Family Group Sheet</h1>");
            body.push_str(&format!("<p class=\"sub\">{}</p><hr class=\"rule\">", esc(&couple.join("  &  "))));
            body.push_str(&format!("<div class=\"sec\"><table>{}", colgroup));
            body.push_str(&Self::person_rows(db, "Husband", f.h));
            body.push_str("</table></div>");
            body.push_str(&format!("<div class=\"sec\"><table>{}", colgroup));
            body.push_str(&Self::person_rows(db, "Wife", f.w));
            body.push_str("</table></div>");
            let kind = if f.kind.trim().is_empty() { "Marriage".to_string() } else { f.kind.trim().to_string() };
            body.push_str(&format!(
                "<table>{}<tr class=\"who\"><th>{}</th><td>{}</td><td>{}</td></tr></table>",
                colgroup,
                esc(&kind),
                esc(&print_date(&f.md)),
                esc(f.mp.trim())
            ));
            body.push_str("<h2>Children</h2>");
            body.push_str("<table><colgroup><col style=\"width:4%\"><col style=\"width:5%\"><col style=\"width:23%\"><col style=\"width:24%\"><col style=\"width:22%\"><col style=\"width:22%\"></colgroup>");
            body.push_str("<tr><th>#</th><th>Sex</th><th>Name</th><th>Born</th><th>Died</th><th>Spouse</th></tr>");
            for (n, k) in f.kids.iter().enumerate() {
                let Some(c) = db.people.get(k) else { continue };
                let when_where = |d: &str, p: &str| {
                    let (d, p) = (print_date(d), p.trim().to_string());
                    match (d.is_empty(), p.is_empty()) {
                        (true, true) => String::new(),
                        (false, true) => esc(&d),
                        (true, false) => esc(&p),
                        (false, false) => format!("{}<br><span style=\"color:#555\">{}</span>", esc(&d), esc(&p)),
                    }
                };
                let spouses: Vec<String> = db.partners(*k).into_iter().filter_map(|s| db.people.get(&s)).map(display_name).collect();
                body.push_str(&format!(
                    "<tr class=\"sec\"><td>{}</td><td>{}</td><td><b>{}</b>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    n + 1,
                    match c.sex.as_str() {
                        "M" => "M",
                        "F" => "F",
                        _ => "",
                    },
                    esc(&display_name(c)),
                    esc(&db.child_note(*fid, *k)),
                    when_where(&c.bd, &c.bp),
                    when_where(&c.dd, &c.dp),
                    esc(&spouses.join("; "))
                ));
            }
            if f.kids.is_empty() {
                body.push_str("<tr><td colspan=\"6\" class=\"empty\">No children recorded</td></tr>");
            }
            body.push_str("</table>");
            for (who, role) in [(f.h, "Husband"), (f.w, "Wife")] {
                let Some(who) = who else { continue };
                let p = &db.people[&who];
                if !p.notes.trim().is_empty() {
                    body.push_str(&format!("<h3>Notes: {} ({})</h3><div class=\"note\">{}</div>", esc(&p.name), role.to_lowercase(), esc(p.notes.trim())));
                }
                // sources, numbered, without repeats
                let mut seen: Vec<String> = Vec::new();
                let mut items: Vec<String> = Vec::new();
                for sr in &p.sources {
                    let text = db.cite_text(sr);
                    if seen.contains(&text) {
                        continue;
                    }
                    seen.push(text.clone());
                    let about = if sr.about == "General" || sr.about.is_empty() { String::new() } else { format!("<i>{}:</i> ", esc(&sr.about)) };
                    items.push(format!("<li>{}{}</li>", about, esc(&text)));
                }
                if !items.is_empty() {
                    body.push_str(&format!("<h3>Sources: {} ({})</h3><ol class=\"src\">{}</ol>", esc(&p.name), role.to_lowercase(), items.join("")));
                }
            }
        }
        body
    }

pub(crate)     fn pedigree_html(&self, root: u32) -> String {
        let db = self.print_db();
        let svg = self.pedigree_svg(&db, root);
        page(&format!("Pedigree chart of {}", db.people[&root].name), &svg, true)
    }

pub(crate)     fn pedigree_svg(&self, db: &Db, root: u32) -> String {
        let gens = self.settings.gens.clamp(3, 8);
        let rows = 1usize << (gens - 1);
        // more detail when there is room: name, born and died lines for up to 5 generations
        let lines = if gens <= 5 { 3 } else if gens == 6 { 2 } else { 1 };
        let bw = if gens <= 5 { 250.0f32 } else { 200.0 };
        let gap = 40.0f32;
        let bh = 24.0 + 18.0 * (lines as f32 - 1.0);
        let pitch = bh + if gens <= 4 { 22.0 } else { 6.0 };
        let top = 10.0f32;
        let w = gens as f32 * bw + (gens as f32 - 1.0) * gap + 20.0;
        let h = rows as f32 * pitch + top + 10.0;
        let mut slots: Vec<Vec<Option<u32>>> = vec![vec![Some(root)]];
        for g in 1..gens {
            let cur = {
                let prev = &slots[g - 1];
                let mut c = Vec::new();
                for s in prev {
                    let (f, m) = match s {
                        Some(i) => db.parents(*i),
                        None => (None, None),
                    };
                    c.push(f);
                    c.push(m);
                }
                c
            };
            slots.push(cur);
        }
        let cut = |t: String, max: usize| if t.chars().count() > max { format!("{}\u{2026}", t.chars().take(max.saturating_sub(1)).collect::<String>()) } else { t };
        let fact = |label: &str, d: &str, p: &str| -> String {
            let d = print_date(d);
            let p = p.trim();
            match (d.is_empty(), p.is_empty()) {
                (true, true) => String::new(),
                (false, true) => format!("{} {}", label, d),
                (true, false) => format!("{} {}", label, p),
                (false, false) => format!("{} {}, {}", label, d, p),
            }
        };
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" font-family=\"Georgia, serif\">", w, h);
        for g in 0..gens {
            let n = 1usize << g;
            let step = rows as f32 * pitch / n as f32;
            for i in 0..n {
                let cy = top + (i as f32 + 0.5) * step;
                let x = 10.0 + g as f32 * (bw + gap);
                if g > 0 {
                    let ccy = top + ((i / 2) as f32 + 0.5) * step * 2.0;
                    svg.push_str(&format!("<polyline fill=\"none\" stroke=\"#8a7d5a\" stroke-width=\"0.9\" points=\"{},{} {},{} {},{} {},{}\"/>", x, cy, x - gap / 2.0, cy, x - gap / 2.0, ccy, x - gap, ccy));
                }
                let y0 = cy - bh / 2.0;
                match slots[g][i] {
                    Some(id) => {
                        let p = &db.people[&id];
                        let (fill, ink, sub) = if g == 0 { ("#2f3a4f", "#fff", "#d8dde8") } else { ("#fbf8ef", "#111", "#555") };
                        let maxc = (bw / 9.5) as usize;
                        let name = if lines == 1 {
                            let l = life(p);
                            if l.is_empty() { display_name(p) } else { format!("{}  {}", display_name(p), l) }
                        } else {
                            display_name(p)
                        };
                        svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"#8a7d5a\" stroke-width=\"0.9\"/>", x, y0, bw, bh, fill));
                        svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"17\" font-weight=\"bold\" fill=\"{}\">{}</text>", x + 6.0, y0 + 18.0, ink, esc(&cut(name, maxc))));
                        if lines >= 2 {
                            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"14\" fill=\"{}\">{}</text>", x + 6.0, y0 + 36.0, sub, esc(&cut(fact("b.", &p.bd, &p.bp), maxc + 8))));
                        }
                        if lines >= 3 {
                            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"14\" fill=\"{}\">{}</text>", x + 6.0, y0 + 54.0, sub, esc(&cut(fact("d.", &p.dd, &p.dp), maxc + 8))));
                        }
                    }
                    None => svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"#fff\" stroke=\"#cfc7b0\" stroke-dasharray=\"3,2\"/>", x, y0, bw, bh)),
                }
            }
        }
        svg.push_str("</svg>");
        svg
    }

    /// Describe what just changed, for the History list.
pub(crate)     fn change_label(&self) -> String {
        if let Some(l) = &self.x.next_label {
            return l.clone();
        }
        let added: Vec<u32> = self.db.people.keys().filter(|k| !self.x.prev_names.contains_key(k)).copied().collect();
        let gone: Vec<&String> = self.x.prev_names.iter().filter(|(k, _)| !self.db.people.contains_key(k)).map(|(_, v)| v).collect();
        if added.len() == 1 && gone.is_empty() {
            return format!("Added {}", self.db.label(added[0]));
        }
        if added.len() > 1 {
            return format!("Added {} people", added.len());
        }
        if gone.len() == 1 && added.is_empty() {
            return format!("Deleted {}", gone[0]);
        }
        if gone.len() > 1 {
            return format!("Removed {} people", gone.len());
        }
        if let Some((sid, old)) = &self.x.prev_sel {
            if let Some(p) = self.db.people.get(sid) {
                if serde_json::to_string(p).unwrap_or_default() != *old {
                    return format!("Changed {}", self.db.label(*sid));
                }
            }
        }
        if self.db.fams.len() != self.x.prev_fams {
            return "Changed family links".into();
        }
        "Changed the tree".into()
    }
}

impl App {
pub(crate)     fn ancestors_html(&self, root: u32) -> String {
        let db = self.print_db();
        let slots = ancestor_slots(&db, root, 12);
        let mut body = String::new();
        for (g, row) in slots.iter().enumerate() {
            let found: Vec<(usize, u32)> = row.iter().enumerate().filter_map(|(i, s)| s.map(|id| (i, id))).collect();
            if found.is_empty() {
                continue;
            }
            body.push_str(&format!("<h2>Generation {}</h2><table><tr><th>No.</th><th>Name</th><th>Born</th><th>Died</th></tr>", g + 1));
            for (i, id) in found {
                let p = &db.people[&id];
                body.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    (1u64 << g) + i as u64,
                    esc(&display_name(p)),
                    dp(&p.bd, &p.bp),
                    dp(&p.dd, &p.dp)
                ));
            }
            body.push_str("</table>");
        }
        page(&format!("Ancestors of {}", db.people[&root].name), &body, false)
    }

pub(crate)     fn descendants_list_html(&self, root: u32) -> String {
        let db = self.print_db();
        let mut out = String::from("<ul>");
        let mut seen = vec![root];
        desc_ul(&db, root, 0, 12, &mut seen, &mut out);
        out.push_str("</ul>");
        page(&format!("Descendants of {}", db.people[&root].name), &out, false)
    }

pub(crate)     fn sources_html(&self) -> String {
        let db = self.print_db();
        let mut ids: Vec<u32> = db.sources.keys().copied().collect();
        ids.sort_by_key(|i| source_title(&db.sources[i]).to_lowercase());
        let counts = db.source_counts();
        let mut b = String::new();
        b.push_str(&format!("<h1>Sources</h1><p>{} source(s), {} repositor{}.</p>", ids.len(), db.repos.len(), if db.repos.len() == 1 { "y" } else { "ies" }));
        for id in ids {
            let s = &db.sources[&id];
            b.push_str(&format!("<div style=\"margin:0 0 14px;page-break-inside:avoid\"><b>{}</b>", esc(&source_title(s))));
            if !s.author.is_empty() {
                b.push_str(&format!("<br>{}", esc(&s.author)));
            }
            let mut line: Vec<String> = Vec::new();
            if !s.kind.is_empty() {
                line.push(s.kind.clone());
            }
            if !s.publ.is_empty() {
                line.push(s.publ.clone());
            }
            if !line.is_empty() {
                b.push_str(&format!("<br>{}", esc(&line.join(". "))));
            }
            if let Some(r) = s.repo.and_then(|r| db.repos.get(&r)) {
                let mut w = r.name.clone();
                if !s.call.is_empty() {
                    w = format!("{}, call number {}", w, s.call);
                }
                if !s.medium.is_empty() {
                    w = format!("{} ({})", w, s.medium);
                }
                b.push_str(&format!("<br>Kept at: {}", esc(&w)));
                if !r.addr.is_empty() {
                    b.push_str(&format!("<br><span style=\"white-space:pre-wrap\">{}</span>", esc(&r.addr)));
                }
            } else if !s.call.is_empty() {
                b.push_str(&format!("<br>Call number: {}", esc(&s.call)));
            }
            if !s.url.is_empty() {
                b.push_str(&format!("<br>{}", esc(&s.url)));
            }
            b.push_str(&format!("<br><small>Cited {} time(s)</small></div>", counts.get(&id).copied().unwrap_or(0)));
        }
        page("Sources", &b, false)
    }

pub(crate)     fn birthdays_html(&self) -> String {
        let db = self.print_db();
        let mut items: Vec<(u32, u32, String)> = Vec::new();
        for p in db.people.values() {
            if let Some((m, d)) = parse_md(&p.bd) {
                let born = parse_date(&p.bd).0.map(|y| format!(" (born {})", y)).unwrap_or_default();
                items.push((m, d, format!("Birthday: {}{}", esc(&display_name(p)), born)));
            }
            if let Some((m, d)) = parse_md(&p.dd) {
                let yy = parse_date(&p.dd).0.map(|y| format!(" ({})", y)).unwrap_or_default();
                items.push((m, d, format!("Remembered: {}{}", esc(&display_name(p)), yy)));
            }
        }
        for f in db.fams.values() {
            if let Some((m, d)) = parse_md(&f.md) {
                let a = f.h.map(|i| display_name(&db.people[&i])).unwrap_or_else(|| "?".to_string());
                let b = f.w.map(|i| display_name(&db.people[&i])).unwrap_or_else(|| "?".to_string());
                let yy = parse_date(&f.md).0.map(|y| format!(" (married {})", y)).unwrap_or_default();
                items.push((m, d, format!("Wedding anniversary: {} and {}{}", esc(&a), esc(&b), yy)));
            }
        }
        items.sort();
        let names = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let mut body = String::new();
        let mut cur = 0u32;
        for (m, d, t) in &items {
            if *m != cur {
                if cur != 0 {
                    body.push_str("</table>");
                }
                body.push_str(&format!("<h2>{}</h2><table>", names[(*m as usize - 1).min(11)]));
                cur = *m;
            }
            body.push_str(&format!("<tr><td class=\"l\">{}</td><td>{}</td></tr>", d, t));
        }
        if cur != 0 {
            body.push_str("</table>");
        }
        if items.is_empty() {
            body.push_str("<p>No full birthdays or anniversaries recorded yet. Add dates that include a day and month, like 12 Mar 1921.</p>");
        }
        page("Birthdays and anniversaries", &body, false)
    }

pub(crate)     fn descendant_svg(&self, db: &Db, root: u32) -> String {
        let maxd = self.settings.gens.clamp(3, 8);
        let mut nodes: Vec<(u32, usize, f32, Option<usize>)> = Vec::new();
        let mut leaf = 0.0f32;
        let mut seen: HashSet<u32> = HashSet::from([root]);
        let ix = db.index();
        desc_layout(db, &ix, root, 0, maxd, None, &mut nodes, &mut leaf, &mut seen);
        let rows = leaf.max(1.0);
        let (bw, bh, pitch, gap, top) = (270.0f32, 38.0f32, 44.0f32, 30.0f32, 10.0f32);
        let w = maxd as f32 * (bw + gap) + 20.0;
        let h = rows * pitch + top + 20.0;
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" font-family=\"Georgia, serif\" font-size=\"12\">", w, h);
        for n in &nodes {
            if let Some(pi) = n.3 {
                let pn = &nodes[pi];
                let px = 10.0 + pn.1 as f32 * (bw + gap) + bw;
                let py = top + (pn.2 + 0.5) * pitch;
                let cx = 10.0 + n.1 as f32 * (bw + gap);
                let cy = top + (n.2 + 0.5) * pitch;
                svg.push_str(&format!(
                    "<polyline fill=\"none\" stroke=\"#888\" points=\"{},{} {},{} {},{} {},{}\"/>",
                    px, py, px + gap / 2.0, py, px + gap / 2.0, cy, cx, cy
                ));
            }
        }
        for (i, n) in nodes.iter().enumerate() {
            let x = 10.0 + n.1 as f32 * (bw + gap);
            let cy = top + (n.2 + 0.5) * pitch;
            let cut = |t: String| if t.chars().count() > 44 { format!("{}...", t.chars().take(42).collect::<String>()) } else { t };
            let (l1, l2) = desc_lines(db, n.0);
            let (l1, l2) = (cut(l1), l2.map(cut));
            let (fill, ink) = if i == 0 { ("#2f3a4f", "#fff") } else { ("#fff", "#111") };
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"#444\"/>",
                x, cy - bh / 2.0, bw, bh, fill
            ));
            match l2 {
                Some(l2) => svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text><text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"10.5\" opacity=\"0.8\">{}</text>",
                    x + 6.0, cy - 4.0, ink, esc(&l1), x + 6.0, cy + 11.0, ink, esc(&l2)
                )),
                None => svg.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>", x + 6.0, cy + 4.0, ink, esc(&l1))),
            }
        }
        svg.push_str("</svg>");
        svg
    }

pub(crate)     fn descendant_html(&self, root: u32) -> String {
        let db = self.print_db();
        let svg = self.descendant_svg(&db, root);
        page(&format!("Descendant chart of {}", db.people[&root].name), &svg, true)
    }

pub(crate)     fn book_html(&self, root: u32) -> String {
        let db = self.print_db();
        let p = &db.people[&root];
        let mut b = String::new();
        b.push_str(&format!("<h2>{}</h2><table>", esc(&display_name(p))));
        let mut rows: Vec<(String, String)> = vec![
            ("Born".to_string(), dp(&p.bd, &p.bp)),
            ("Christened".to_string(), dp(&p.chd, &p.chp)),
            ("Died".to_string(), dp(&p.dd, &p.dp)),
            ("Buried".to_string(), dp(&p.bud, &p.bup)),
        ];
        for e in &p.events {
            let detail = [e.val.as_str(), e.note.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(". ");
            let note = if detail.is_empty() { String::new() } else { format!(" {}", esc(&detail)) };
            rows.push((e.kind.clone(), format!("{}{}", dp(&e.date, &e.place), note)));
        }
        for (l, v) in rows {
            if !v.trim().is_empty() {
                b.push_str(&format!("<tr><td class=\"l\">{}</td><td>{}</td></tr>", esc(&l), v));
            }
        }
        b.push_str("</table>");
        if !p.notes.is_empty() {
            b.push_str(&format!("<h2>Notes</h2><p style=\"white-space:pre-wrap\">{}</p>", esc(&p.notes)));
        }
        if !p.sources.is_empty() {
            b.push_str("<h2>Sources</h2><ul>");
            for sr in &p.sources {
                let label = if sr.about == "General" || sr.about.is_empty() { String::new() } else { format!("{}: ", sr.about) };
                b.push_str(&format!("<li>{}{}</li>", esc(&label), esc(&db.cite_text(sr))));
            }
            b.push_str("</ul>");
        }
        let imgs: Vec<&Media> = p.media.iter().filter(|m| is_image(&m.file)).collect();
        if !imgs.is_empty() {
            b.push_str("<h2>Photos</h2><div style=\"display:flex;flex-wrap:wrap;gap:12px\">");
            for m in imgs {
                let path = if !m.mid.is_empty() && m.mid != "-" { mprev(&m.mid) } else { mfile(&m.file) };
                let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                b.push_str(&format!(
                    "<figure style=\"margin:0;width:230px\"><img src=\"{}\" style=\"width:100%\"><figcaption style=\"font-size:12px\">{}</figcaption></figure>",
                    file_uri(&path),
                    esc(cap)
                ));
            }
            b.push_str("</div>");
        }
        let docs: Vec<&Media> = p.media.iter().filter(|m| !is_image(&m.file)).collect();
        if !docs.is_empty() {
            b.push_str("<h2>Documents and recorded stories</h2><ul>");
            for m in docs {
                let cap = if m.caption.is_empty() { m.name.as_str() } else { m.caption.as_str() };
                b.push_str(&format!("<li><a href=\"{}\">{}</a></li>", file_uri(&mfile(&m.file)), esc(cap)));
            }
            b.push_str("</ul>");
        }
        let tl = db.timeline(root);
        if !tl.is_empty() {
            b.push_str("<h2>Timeline</h2><table>");
            for (d, t) in &tl {
                b.push_str(&format!("<tr><td class=\"l\">{}</td><td>{}</td></tr>", esc(d), esc(t)));
            }
            b.push_str("</table>");
        }
        b.push_str("<div style=\"page-break-before:always\"></div><h2>Family</h2>");
        b.push_str(&Self::family_body(&db, root));
        let (f, m) = db.parents(root);
        if let Some(x) = f.or(m) {
            b.push_str("<h2>Parents' family</h2>");
            b.push_str(&Self::family_body(&db, x));
        }
        b.push_str("<div style=\"page-break-before:always\"></div><h2>Pedigree chart</h2>");
        b.push_str(&self.pedigree_svg(&db, root));
        if db.fams_of(root).iter().any(|f| !db.fams[f].kids.is_empty()) {
            b.push_str("<h2>Descendant chart</h2>");
            b.push_str(&self.descendant_svg(&db, root));
        }
        page(&format!("Family book: {}", p.name), &b, false)
    }
}
