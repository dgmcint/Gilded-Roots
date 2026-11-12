//! Source templates: for common kinds of record, fill in a few boxes (year, county, certificate
//! number...) and the source's title, author and publication details are written out the same
//! way every time.

use super::*;

pub(crate) struct Template {
    pub(crate) name: &'static str,
    /// The source type it sets (one of SOURCE_KINDS)
    pub(crate) kind: &'static str,
    /// (label, example)
    pub(crate) fields: &'static [(&'static str, &'static str)],
}

pub(crate) const TEMPLATES: &[Template] = &[
    Template { name: "Census", kind: "Census", fields: &[("Year", "1880"), ("Country", "United States"), ("State or region", "Idaho"), ("County", "Twin Falls"), ("Town or district", "Buhl"), ("Enumeration district", "12")] },
    Template { name: "Birth, marriage or death certificate", kind: "Vital record", fields: &[("Kind of record", "death certificate"), ("Person or couple", "Thomas James Harwood"), ("Date", "11 Dec 1975"), ("Place (county, state)", "Lane County, Oregon"), ("Certificate number", "75-12345"), ("Issued by", "Oregon State Board of Health")] },
    Template { name: "Church register", kind: "Church record", fields: &[("Church", "St Peter's Church"), ("Place", "Leeds, Yorkshire, England"), ("Register", "baptisms"), ("Years", "1830-1845")] },
    Template { name: "Newspaper item", kind: "Newspaper", fields: &[("Newspaper", "Times-News"), ("Place published", "Twin Falls, Idaho"), ("Date", "13 Dec 1975"), ("Page", "4"), ("Headline or item", "Obituary of Thomas J. Harwood")] },
    Template { name: "Book", kind: "Book", fields: &[("Author", "Jane Smith"), ("Title", "History of Twin Falls County"), ("Place published", "Boise, Idaho"), ("Publisher", "Caxton Printers"), ("Year", "1950")] },
    Template { name: "Website or online database", kind: "Website", fields: &[("Website", "FamilySearch"), ("Collection or page", "Idaho, Death Certificates, 1911-1937"), ("Web address", "https://www.familysearch.org/..."), ("Looked at on", "11 Nov 2026")] },
    Template { name: "Gravestone or cemetery record", kind: "Cemetery record", fields: &[("Person", "William Harwood"), ("Cemetery", "Twin Falls Cemetery"), ("Place", "Twin Falls, Idaho"), ("Kind", "gravestone"), ("Section, row or plot", "Block 4, lot 12")] },
    Template { name: "Military record", kind: "Military record", fields: &[("Record", "World War I draft registration card"), ("Person", "Thomas James Harwood"), ("Place", "Twin Falls County, Idaho"), ("Date", "1918"), ("Held by", "National Archives")] },
    Template { name: "Passenger list or immigration record", kind: "Immigration record", fields: &[("Ship", "SS Baltic"), ("From", "Liverpool"), ("To", "New York"), ("Arrived", "12 Apr 1880"), ("Held by", "National Archives")] },
    Template { name: "Family Bible or family papers", kind: "Family papers", fields: &[("Family", "Harwood"), ("Item", "family Bible"), ("Held by", "Ruth Patterson, Portland, Oregon"), ("Years covered", "1838-1975")] },
    Template { name: "Interview", kind: "Interview", fields: &[("Person interviewed", "Ruth Ellen Patterson"), ("Interviewed by", "Emily Harwood"), ("Date", "4 Jul 2010"), ("Place", "Portland, Oregon"), ("Recorded as", "audio recording")] },
    Template { name: "Letter or diary", kind: "Letter or diary", fields: &[("Written by", "Margaret Ellis"), ("Written to", "Ida Mae Ellis"), ("Kind", "letter"), ("Date", "3 Mar 1924"), ("Held by", "Harwood family papers")] },
];

fn v<'a>(vals: &'a [String], i: usize) -> &'a str {
    vals.get(i).map(|s| s.trim()).unwrap_or("")
}

/// Join the non-empty parts with `sep`.
fn join(parts: &[&str], sep: &str) -> String {
    parts.iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(sep)
}

/// (title, author, publication details, web address) for a template filled in with `vals`.
pub(crate) fn build(t: usize, vals: &[String]) -> (String, String, String, String) {
    let g = |i: usize| v(vals, i);
    match TEMPLATES.get(t).map(|t| t.name).unwrap_or("") {
        "Census" => {
            let county = if g(3).is_empty() { String::new() } else { format!("{} County", g(3)) };
            let place = join(&[g(4), &county, g(2)], ", ");
            let title = join(&[&join(&[g(0), g(1), "census"], " "), &place], ", ");
            let publ = if g(5).is_empty() { String::new() } else { format!("Enumeration district {}", g(5)) };
            (title, String::new(), publ, String::new())
        }
        "Birth, marriage or death certificate" => {
            let title = join(&[&join(&[g(3), g(0)], " "), g(1), g(2)], ", ");
            let publ = if g(4).is_empty() { String::new() } else { format!("Certificate no. {}", g(4)) };
            (title, g(5).to_string(), publ, String::new())
        }
        "Church register" => {
            let title = format!("{}{}", join(&[g(0), g(1)], ", "), if g(2).is_empty() && g(3).is_empty() { String::new() } else { format!(": {}", join(&[g(2), g(3)], " ")) });
            (title, g(0).to_string(), String::new(), String::new())
        }
        "Newspaper item" => {
            let title = if g(1).is_empty() || g(0).is_empty() { join(&[g(0), g(1)], "") } else { format!("{} ({})", g(0), g(1)) };
            let page = if g(3).is_empty() { String::new() } else { format!("p. {}", g(3)) };
            let publ = join(&[g(2), &page, g(4)], ", ");
            (title, String::new(), publ, String::new())
        }
        "Book" => {
            let place_pub = join(&[g(2), g(3)], ": ");
            (g(1).to_string(), g(0).to_string(), join(&[&place_pub, g(4)], ", "), String::new())
        }
        "Website or online database" => {
            let title = join(&[g(0), g(1)], ": ");
            let publ = if g(3).is_empty() { String::new() } else { format!("Looked at {}", g(3)) };
            (title, g(0).to_string(), publ, g(2).to_string())
        }
        "Gravestone or cemetery record" => {
            let what = if g(3).is_empty() { "Gravestone" } else { g(3) };
            let mut what = what.to_string();
            if let Some(c) = what.get(0..1) {
                what = format!("{}{}", c.to_uppercase(), &what[1..]);
            }
            let title = join(&[&join(&[&what, g(0)], " of "), g(1), g(2)], ", ");
            (title, String::new(), g(4).to_string(), String::new())
        }
        "Military record" => (join(&[g(0), g(1), g(2), g(3)], ", "), String::new(), String::new(), String::new()),
        "Passenger list or immigration record" => {
            let route = join(&[g(1), g(2)], " to ");
            let title = join(&[&format!("Passenger list{}", if g(0).is_empty() { String::new() } else { format!(", {}", g(0)) }), &route, g(3)], ", ");
            (title, String::new(), String::new(), String::new())
        }
        "Family Bible or family papers" => {
            let title = join(&[&join(&[g(0), "family", g(1)], " ").replace("family family", "family"), g(3)], ", ");
            (title, String::new(), if g(2).is_empty() { String::new() } else { format!("Held by {}", g(2)) }, String::new())
        }
        "Interview" => {
            let mut title = format!("Interview with {}", if g(0).is_empty() { "..." } else { g(0) });
            if !g(1).is_empty() {
                title.push_str(&format!(" by {}", g(1)));
            }
            let title = join(&[&title, g(3), g(2)], ", ");
            (title, g(1).to_string(), g(4).to_string(), String::new())
        }
        "Letter or diary" => {
            let kind = if g(2).is_empty() { "Letter" } else { g(2) };
            let mut t = format!("{}{}", kind.get(0..1).unwrap_or("").to_uppercase(), kind.get(1..).unwrap_or(""));
            if !g(0).is_empty() {
                t.push_str(&format!(" from {}", g(0)));
            }
            if !g(1).is_empty() {
                t.push_str(&format!(" to {}", g(1)));
            }
            (join(&[&t, g(3)], ", "), g(0).to_string(), if g(4).is_empty() { String::new() } else { format!("Held by {}", g(4)) }, String::new())
        }
        _ => Default::default(),
    }
}

/// The "Fill in from a template" part of the source editor.
pub(crate) fn template_ui(ui: &mut egui::Ui, s: &mut SourceRec, which: &mut usize, vals: &mut Vec<String>) {
    egui::CollapsingHeader::new("Fill in from a template").id_salt("src_template").show(ui, |ui| {
        ui.label(RichText::new("Choose the kind of record and fill in what you know; the title, author and publication details above are written out for you.").small().weak());
        let t = (*which).min(TEMPLATES.len() - 1);
        egui::ComboBox::from_id_salt("tpl_pick").selected_text(TEMPLATES[t].name).width(280.0).show_ui(ui, |ui| {
            for (i, tp) in TEMPLATES.iter().enumerate() {
                if ui.selectable_value(which, i, tp.name).clicked() {
                    vals.clear();
                }
            }
        });
        let tp = &TEMPLATES[(*which).min(TEMPLATES.len() - 1)];
        vals.resize(tp.fields.len(), String::new());
        egui::Grid::new("tpl_fields").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            for (i, (label, example)) in tp.fields.iter().enumerate() {
                ui.label(*label);
                ui.add(egui::TextEdit::singleline(&mut vals[i]).hint_text(format!("e.g. {}", example)).desired_width(300.0));
                ui.end_row();
            }
        });
        let filled = vals.iter().any(|v| !v.trim().is_empty());
        let (title, author, publ, url) = if filled { build(*which, vals) } else { Default::default() };
        if !title.is_empty() {
            ui.label(RichText::new(format!("Title: {}", title)).small());
            if !author.is_empty() {
                ui.label(RichText::new(format!("Author: {}", author)).small());
            }
            if !publ.is_empty() {
                ui.label(RichText::new(format!("Publication: {}", publ)).small());
            }
        }
        let has_own = !s.title.trim().is_empty() || !s.author.trim().is_empty() || !s.publ.trim().is_empty();
        let b = ui.add_enabled(!title.is_empty(), egui::Button::new("Use these for this source"));
        let b = if has_own { b.on_hover_text("Replaces the title, author and publication details above. Undo puts them back.") } else { b };
        if b.clicked() {
            s.title = title;
            s.author = author;
            s.publ = publ;
            if !url.is_empty() {
                s.url = url;
            }
            s.kind = tp.kind.to_string();
            vals.clear();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(t: &str, vals: &[&str]) -> (String, String, String, String) {
        let i = TEMPLATES.iter().position(|x| x.name == t).unwrap();
        build(i, &vals.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn every_template_has_a_known_kind_and_writes_a_title() {
        for (i, t) in TEMPLATES.iter().enumerate() {
            assert!(SOURCE_KINDS.contains(&t.kind), "{}", t.name);
            let vals: Vec<String> = t.fields.iter().map(|(_, e)| e.to_string()).collect();
            let (title, ..) = build(i, &vals);
            assert!(!title.is_empty() && !title.contains(", ,") && !title.contains("  "), "{}: {title}", t.name);
            // and with nothing filled in, nothing odd comes out
            let (empty, ..) = build(i, &[]);
            assert!(!empty.starts_with(',') && !empty.ends_with(','), "{}: {empty}", t.name);
        }
    }

    #[test]
    fn titles_read_well() {
        assert_eq!(f("Census", &["1880", "United States", "Idaho", "Twin Falls", "Buhl", "12"]).0, "1880 United States census, Buhl, Twin Falls County, Idaho");
        assert_eq!(f("Census", &["1880", "United States", "Idaho", "Twin Falls", "Buhl", "12"]).2, "Enumeration district 12");
        assert_eq!(f("Book", &["Jane Smith", "History of Twin Falls County", "Boise, Idaho", "Caxton Printers", "1950"]), ("History of Twin Falls County".into(), "Jane Smith".into(), "Boise, Idaho: Caxton Printers, 1950".into(), String::new()));
        assert_eq!(f("Birth, marriage or death certificate", &["death certificate", "Thomas James Harwood", "11 Dec 1975", "Lane County, Oregon", "75-12345", "Oregon State Board of Health"]).0, "Lane County, Oregon death certificate, Thomas James Harwood, 11 Dec 1975");
        assert_eq!(f("Interview", &["Ruth Ellen Patterson", "Emily Harwood", "4 Jul 2010", "Portland, Oregon", "audio recording"]).0, "Interview with Ruth Ellen Patterson by Emily Harwood, Portland, Oregon, 4 Jul 2010");
        assert_eq!(f("Newspaper item", &["Times-News", "Twin Falls, Idaho", "13 Dec 1975", "4", "Obituary of Thomas J. Harwood"]), ("Times-News (Twin Falls, Idaho)".into(), String::new(), "13 Dec 1975, p. 4, Obituary of Thomas J. Harwood".into(), String::new()));
        assert_eq!(f("Gravestone or cemetery record", &["William Harwood", "Twin Falls Cemetery", "Twin Falls, Idaho", "gravestone", "Block 4"]).0, "Gravestone of William Harwood, Twin Falls Cemetery, Twin Falls, Idaho");
        assert_eq!(f("Family Bible or family papers", &["Harwood", "family Bible", "Ruth Patterson", "1838-1975"]).0, "Harwood family Bible, 1838-1975");
    }
}

#[cfg(test)]
mod cite_picture_tests {
    use super::*;

    #[test]
    fn pictures_on_citations_go_out_and_come_back() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME Ann /Lee/\n1 BIRT\n2 DATE 1900\n2 SOUR @S1@\n3 PAGE Sheet 4\n3 OBJE\n4 FILE census-1900.jpg\n5 FORM jpg\n0 @S1@ SOUR\n1 TITL 1900 census\n0 TRLR\n";
        let mut db = gedcom::parse_ged(t);
        let mut found = vec![];
        db.each_cite(&mut |_, _, c| found.extend(c.media.iter().map(|m| m.path.clone())));
        assert_eq!(found, ["census-1900.jpg"], "read from the file");
        // as if the picture had been copied into the media folder
        db.each_cite_mut(&mut |c| {
            for m in c.media.iter_mut() {
                m.file = "abc123.jpg".into();
                m.name = "census-1900.jpg".into();
                m.path.clear();
            }
        });
        assert!(media_in_use(&db).contains("abc123.jpg"), "a citation's picture counts as in use");
        let out = gedcom::to_ged(&db);
        assert!(out.contains("3 OBJE\n4 FILE census-1900.jpg"), "{out}");
        let back = gedcom::parse_ged(&out);
        let mut again = vec![];
        back.each_cite(&mut |_, _, c| again.extend(c.media.iter().map(|m| m.path.clone())));
        assert_eq!(again, ["census-1900.jpg"]);
    }
}
