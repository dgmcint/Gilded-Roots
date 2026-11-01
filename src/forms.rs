//! Reports > Forms: blank paper forms to take to a library, archive or relative's house and fill
//! in by hand: a family group sheet and a 4- or 5-generation pedigree chart. (Blank census forms
//! are in census.rs.)

use super::*;

/// The line across the top of every blank form: who filled it in, when, and where from.
fn prepared_by() -> String {
    "<div class=\"bf\"><div><span>Prepared by</span></div><div><span>Date</span></div><div class=\"w2\"><span>Address, phone or email</span></div></div>".to_string()
}

const BLANK_CSS: &str = ".bf{display:flex;border:1px solid #b9b2a0;margin:0 0 6px}table.fg{margin:0 0 6px}\
    .bf div{flex:1;height:30px;padding:1px 5px;border-left:1px solid #b9b2a0}.bf div:first-child{border-left:0}\
    .bf div.w2{flex:2}\
    .bf span,.lbl{font-size:8px;color:#6b5a2e;text-transform:uppercase;letter-spacing:.3px}\
    table.fg td,table.fg th{height:19px;font-size:10px;padding:1px 4px;vertical-align:middle}\
    table.fg th{font-size:10.5px;text-align:left}\
    table.fg td.k{color:#555;font-size:10px;width:13%}\
    table.ch td{height:31px;font-size:9.5px;vertical-align:top;padding:1px 3px}\
    table.ch th{font-size:9.5px;text-align:center;vertical-align:bottom}\
    table.ch td.n{text-align:center;color:#888}\
    .box{border:1px solid #b9b2a0;height:56px;padding:1px 5px;margin:0 0 6px}";

/// The ordinances a Latter-day Saint family group sheet or pedigree records, with the short
/// letters used on the pedigree's tick boxes.
pub const ORDINANCES: [(&str, &str); 6] = [("B", "Baptized"), ("C", "Confirmed"), ("I", "Initiatory"), ("E", "Endowed"), ("SP", "Sealed to parents"), ("SS", "Sealed to spouse")];

/// A blank family group sheet: husband, wife, their marriage, up to ten children, and room
/// for notes and sources. Portrait, one page.
pub fn blank_family_group() -> String {
    family_group(false)
}

/// The same sheet with room for Latter-day Saint ordinance dates and temples for the couple and
/// each child (eight children, so it stays on one page). Portrait, one page.
pub fn blank_family_group_lds() -> String {
    family_group(true)
}

/// The couple's ordinances side by side: one row per ordinance, a date and temple for each.
fn couple_ordinances() -> String {
    let mut r = String::from("<table class=\"fg ord\"><colgroup><col style=\"width:16%\"><col style=\"width:14%\"><col style=\"width:28%\"><col style=\"width:14%\"><col style=\"width:28%\"></colgroup><tr><th>LDS ordinances</th><th colspan=\"2\">Husband <span class=\"lbl\">date &middot; temple</span></th><th colspan=\"2\">Wife <span class=\"lbl\">date &middot; temple</span></th></tr>");
    for (_, name) in ORDINANCES {
        if name == "Sealed to spouse" {
            r.push_str("<tr><td class=\"k\">Sealed to spouse</td><td></td><td></td><td colspan=\"2\" class=\"same\">(this couple: one date, written for the husband)</td></tr>");
        } else {
            r.push_str(&format!("<tr><td class=\"k\">{}</td><td></td><td></td><td></td><td></td></tr>", name));
        }
    }
    r.push_str("</table>");
    r
}

fn family_group(lds: bool) -> String {
    let mut b = String::new();
    b.push_str(&prepared_by());
    let person = |role: &str, married: bool| -> String {
        let mut r = format!("<table class=\"fg\"><colgroup><col style=\"width:13%\"><col style=\"width:27%\"><col style=\"width:60%\"></colgroup><tr><th colspan=\"3\">{} <span class=\"lbl\">full name</span></th></tr><tr><td class=\"k\"></td><td class=\"lbl\">Date</td><td class=\"lbl\">Place (town, county, state or province, country)</td></tr>", role);
        let mut rows = vec!["Born", "Christened", "Died", "Buried"];
        if married {
            rows.insert(2, "Married");
        }
        for l in rows {
            r.push_str(&format!("<tr><td class=\"k\">{}</td><td></td><td></td></tr>", l));
        }
        if lds {
            // two to a row, to keep the LDS sheet on one page
            for (a, b) in [("Father", "Mother"), ("Occupation", "Other spouses")] {
                r.push_str(&format!("<tr><td colspan=\"3\" class=\"pair\"><div><span class=\"k\">{}</span><span></span><span class=\"k\">{}</span><span></span></div></td></tr>", a, b));
            }
        } else {
            for l in ["Father", "Mother", "Other spouses", "Occupation"] {
                r.push_str(&format!("<tr><td class=\"k\">{}</td><td colspan=\"2\"></td></tr>", l));
            }
        }
        r.push_str("</table>");
        r
    };
    b.push_str(&person("Husband", true));
    b.push_str(&person("Wife", false).replace("full name", "full name before marriage"));
    if lds {
        b.push_str(&couple_ordinances());
    }
    b.push_str("<table class=\"ch\"><colgroup><col style=\"width:4%\"><col style=\"width:5%\"><col style=\"width:23%\"><col style=\"width:22%\"><col style=\"width:22%\"><col style=\"width:24%\"></colgroup>");
    b.push_str("<tr><th>#</th><th>Sex</th><th>Children, in order of birth<br><span class=\"lbl\">full name</span></th><th>Born<br><span class=\"lbl\">date and place</span></th><th>Died<br><span class=\"lbl\">date and place</span></th><th>Married<br><span class=\"lbl\">spouse, date and place</span></th></tr>");
    // ten children; eight on the LDS sheet, so the ordinance lines still fit on one page
    let kids = if lds { 8 } else { 10 };
    for i in 1..=kids {
        b.push_str(&format!("<tr class=\"kid\"><td class=\"n\">{}</td><td></td><td></td><td></td><td></td><td></td></tr>", i));
        if lds {
            let blanks: String = ORDINANCES.iter().map(|(k, n)| format!("<span title=\"{}\"><b>{}</b> ________</span>", n, k)).collect::<Vec<_>>().join(" ");
            b.push_str(&format!("<tr class=\"ordrow\"><td></td><td colspan=\"5\">{} <span><b>Temple</b> ______________</span></td></tr>", blanks));
        }
    }
    b.push_str("</table>");
    if lds {
        b.push_str("<p class=\"key\">Children's ordinances: B baptized, C confirmed, I initiatory, E endowed, SP sealed to parents, SS sealed to spouse. Write the date on each line and the temple at the end. More than eight children? Continue on a second sheet.</p>");
    }
    b.push_str("<div class=\"box\"><span class=\"lbl\">Sources (where each fact came from) and notes</span></div>");
    let title = if lds { "Family group sheet with LDS ordinances" } else { "Family group sheet" };
    // the LDS sheet has more to fit on one page, so its rows are a little shorter
    let tight = if lds { "table.fg td,table.fg th{height:16px}table.ch td{height:22px}table.ch th{font-size:9px}.box{height:34px}td.pair{padding:0}td.pair div{display:flex;height:100%}td.pair span{flex:37;padding:1px 4px}td.pair span.k{flex:13;color:#555;font-size:10px;border-right:1px solid #b9b2a0}td.pair span.k~span.k{border-left:1px solid #b9b2a0}h1{font-size:18px}.rule{margin:4px 0 8px}" } else { "" };
    let css = format!("{}{}table.ord td{{height:16px}}table.ord td.same{{font-size:8.5px;color:#888}}tr.ordrow td{{height:16px;font-size:8.5px;color:#555;vertical-align:middle;border-top:0}}tr.ordrow span{{margin-right:5px;display:inline-block}}tr.kid td{{border-bottom:0}}.key{{font-size:9px;color:#555;margin:2px 0 5px}}table{{break-inside:auto}}tr{{break-inside:avoid}}", BLANK_CSS, tight);
    page_with_css(title, &b, false, &css, true)
}

/// Which details to write under each name in a generation of a blank pedigree.
fn pedigree_fields(gen: u32, gens: u32) -> &'static [&'static str] {
    match (gen, gens) {
        (1, _) | (2, _) => &["b.", "p.", "m.", "p.", "d.", "p."],
        (3, 4) => &["b.", "p.", "m.", "d.", "p."],
        (3, _) => &["b.", "p.", "m.", "d."],
        (4, 4) => &["b.", "p.", "d."],
        (4, _) => &["b.", "m.", "d."],
        _ => &["b.", "d."],
    }
}

/// A blank pedigree chart for 4 or 5 generations: numbered boxes (1 is the person the chart
/// starts with, 2 and 3 the parents, 4 to 7 the grandparents...) joined by brackets, with
/// lines for chart numbers so several charts can be linked. Landscape, one page.
pub fn blank_pedigree(gens: u32) -> String {
    pedigree(gens, false)
}

/// The same chart with tick boxes in each person's box for Latter-day Saint ordinances.
pub fn blank_pedigree_lds(gens: u32) -> String {
    pedigree(gens, true)
}

fn pedigree(gens: u32, lds: bool) -> String {
    let gens = gens.clamp(4, 5);
    let (w, top, h) = (1000.0f32, 6.0f32, 640.0f32);
    let colw: Vec<f32> = if gens == 4 { vec![230.0, 230.0, 240.0, 270.0] } else { vec![190.0, 190.0, 190.0, 200.0, 210.0] };
    let gap = (w - colw.iter().sum::<f32>()) / (gens as f32 - 1.0 + 0.0001).max(1.0);
    let mut xs = Vec::new();
    let mut x = 0.0;
    for cw in &colw {
        xs.push(x);
        x += cw + gap;
    }
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" font-family=\"Georgia,serif\">", w, top + h + 4.0);
    let line = 12.5f32;
    // boxes, generation by generation
    let mut centres: Vec<Vec<(f32, f32, f32)>> = Vec::new();
    for g in 1..=gens {
        let n = 1u32 << (g - 1);
        let slot = h / n as f32;
        let fields = pedigree_fields(g, gens);
        let last = g == gens;
        let ticks = lds && !(last && gens == 5);
        let bh = if last && gens == 5 { 30.0 } else { 17.0 + line * fields.len() as f32 + 3.0 + if ticks { 13.0 } else { 0.0 } };
        let (bx, bw) = (xs[g as usize - 1], colw[g as usize - 1]);
        let mut row = Vec::new();
        for i in 0..n {
            let cy = top + slot * (i as f32 + 0.5);
            let by = cy - bh / 2.0;
            let num = n + i;
            svg.push_str(&format!("<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" rx=\"3\" fill=\"#fffdf6\" stroke=\"#8a8169\" stroke-width=\"1\"/>"));
            svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"9\" font-weight=\"bold\" fill=\"#6b5a2e\">{}</text>", bx + 4.0, by + 11.0, num));
            // name line
            svg.push_str(&format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#b9b2a0\" stroke-width=\"0.8\"/>", bx + 18.0, by + 14.0, bx + bw - 5.0, by + 14.0));
            if last && gens == 5 {
                // one line: born and died side by side
                let half = (bw - 10.0) / 2.0;
                for (k, lab) in ["b.", "d."].iter().enumerate() {
                    let lx = bx + 5.0 + half * k as f32;
                    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"7.5\" fill=\"#777\">{}</text><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ddd5bf\" stroke-width=\"0.7\"/>", lx, by + 25.0, lab, lx + 10.0, by + 26.0, lx + half - 4.0, by + 26.0));
                }
            } else {
                for (k, lab) in fields.iter().enumerate() {
                    let ly = by + 17.0 + line * (k as f32 + 1.0) - 2.0;
                    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"7.5\" fill=\"#777\">{}</text><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ddd5bf\" stroke-width=\"0.7\"/>", bx + 5.0, ly, lab, bx + 17.0, ly + 1.0, bx + bw - 5.0, ly + 1.0));
                }
            }
            if ticks {
                // a row of small boxes to tick as each ordinance is done
                let ty = by + bh - 12.0;
                let mut tx = bx + 5.0;
                for (k, _) in ORDINANCES {
                    svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"8\" height=\"8\" fill=\"none\" stroke=\"#8a8169\" stroke-width=\"0.7\"/><text x=\"{}\" y=\"{}\" font-size=\"7\" fill=\"#6b5a2e\">{}</text>", tx, ty, tx + 10.0, ty + 7.0, k));
                    tx += if k.len() > 1 { 26.0 } else { 21.0 };
                }
            }
            if last {
                svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"7\" fill=\"#999\" text-anchor=\"end\">cont. on chart ____</text>", bx + bw - 4.0, by + 10.0));
            }
            row.push((bx, bx + bw, cy));
        }
        centres.push(row);
    }
    // brackets from each child to its two parents
    for g in 0..(gens as usize - 1) {
        for (i, &(_, right, cy)) in centres[g].iter().enumerate() {
            let (fx, _, fy) = centres[g + 1][2 * i];
            let (_, _, my) = centres[g + 1][2 * i + 1];
            let mid = right + (fx - right) / 2.0;
            svg.push_str(&format!("<path d=\"M{right} {cy} H{mid} M{mid} {fy} V{my} M{mid} {fy} H{fx} M{mid} {my} H{fx}\" fill=\"none\" stroke=\"#8a8169\" stroke-width=\"1\"/>"));
        }
    }
    // the spouse of number 1, under its box
    let (l1, r1, c1) = centres[0][0];
    let sp_y = c1 + (17.0 + line * 6.0 + 3.0 + if lds { 13.0 } else { 0.0 }) / 2.0 + 22.0;
    svg.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"8\" fill=\"#6b5a2e\">SPOUSE OF NO. 1</text><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#b9b2a0\" stroke-width=\"0.8\"/>", l1 + 2.0, sp_y, l1 + 2.0, sp_y + 16.0, r1, sp_y + 16.0));
    svg.push_str("</svg>");
    let mut b = String::new();
    b.push_str(&format!(
        "<div class=\"bf\"><div><span>Chart number</span></div><div class=\"w2\"><span>No. 1 on this chart is the same as no. ____ on chart ____</span></div><div><span>Prepared by</span></div><div><span>Date</span></div></div>{}<p style=\"margin:4px 0 0;font-size:9.5px;color:#555\">b. born &middot; p. place &middot; m. married &middot; d. died. Fathers have even numbers and mothers odd: the father of no. n is no. 2n and the mother no. 2n+1 (the father of no. 3 is no. 6).{}</p>",
        svg,
        if lds { " Tick a box when the ordinance is done: B baptized, C confirmed, I initiatory, E endowed, SP sealed to parents, SS sealed to spouse." } else { "" }
    ));
    let b = format!("{}{}", census::LANDSCAPE_TIP, b);
    let title = if lds { format!("Pedigree chart with LDS ordinances ({} generations)", gens) } else { format!("Pedigree chart ({} generations)", gens) };
    page_with_css(&title, &b, true, &format!("{}svg{{max-height:none}}@media print{{svg{{max-height:158mm}}}}", BLANK_CSS), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_group_sheet_has_both_parents_and_ten_children() {
        let h = blank_family_group();
        assert!(h.contains("Husband") && h.contains("Wife") && h.contains("full name before marriage"));
        assert_eq!(h.matches("<td class=\"n\">").count(), 10);
        assert_eq!(h.matches("Married</td>").count(), 1);
    }

    #[test]
    fn pedigree_numbers_every_box() {
        for (gens, boxes) in [(4u32, 15usize), (5, 31)] {
            let h = blank_pedigree(gens);
            assert_eq!(h.matches("<rect ").count(), boxes, "{} generations", gens);
            assert!(h.contains(&format!(">{}</text>", boxes)));
            assert_eq!(h.matches("cont. on chart").count(), 1 << (gens - 1));
        }
        // out-of-range requests are kept to 4 or 5
        assert_eq!(blank_pedigree(9).matches("<rect ").count(), 31);
    }

    #[test]
    fn lds_versions_add_ordinances() {
        let h = blank_family_group_lds();
        assert_eq!(h.matches("tr class=\"ordrow\"").count(), 8);
        assert_eq!(h.matches("Sealed to spouse</td>").count(), 1);
        assert!(h.contains("LDS ordinances</th>"));
        assert!(!blank_family_group().contains("class=\"ordrow\""));
        for (gens, ticked) in [(4u32, 15usize), (5, 15)] {
            let h = blank_pedigree_lds(gens);
            // six tick boxes in every box that has room (all but a 5-generation chart's last column)
            assert_eq!(h.matches("width=\"8\" height=\"8\"").count(), ticked * 6, "{} generations", gens);
            assert!(h.contains("SP sealed to parents"));
        }
    }

    #[test]
    #[ignore]
    fn blank_forms_for_a_look() {
        let dir = std::env::temp_dir().join("gr-census");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("family-group.html"), blank_family_group()).unwrap();
        std::fs::write(dir.join("pedigree-4.html"), blank_pedigree(4)).unwrap();
        std::fs::write(dir.join("pedigree-5.html"), blank_pedigree(5)).unwrap();
        std::fs::write(dir.join("family-group-lds.html"), blank_family_group_lds()).unwrap();
        std::fs::write(dir.join("pedigree-4-lds.html"), blank_pedigree_lds(4)).unwrap();
        std::fs::write(dir.join("pedigree-5-lds.html"), blank_pedigree_lds(5)).unwrap();
    }
}
