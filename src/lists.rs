//! Ready-made lists to print, the ones PAF users reach for: everyone in alphabetical order,
//! marriages, end-of-line ancestors, people linked to nobody, missing LDS ordinances, contact
//! details and possible duplicates.

use super::*;

pub(crate) const LISTS: &[(&str, &str)] = &[
    ("Everyone, in alphabetical order", "Each person's ID, name, birth and death"),
    ("Marriages", "Every couple, with when and where they married"),
    ("End-of-line ancestors", "The selected person's ancestors with no parents recorded: where each line stops"),
    ("People linked to nobody", "People with no parents, partners or children in the tree"),
    ("Missing LDS ordinances", "People who have died, with the ordinances not yet recorded"),
    ("Contact details", "Addresses, phones and emails of living relatives"),
    ("Possible duplicates", "Pairs of people who may be the same person"),
];

fn sort_key(p: &Person) -> (String, String, u32) {
    (p.surname.to_lowercase(), p.given.to_lowercase(), 0)
}

fn table(head: &[&str], rows: &[Vec<String>]) -> String {
    let mut b = String::from("<table><tr>");
    for h in head {
        b.push_str(&format!("<th>{}</th>", esc(h)));
    }
    b.push_str("</tr>");
    for r in rows {
        b.push_str("<tr>");
        for c in r {
            b.push_str(&format!("<td>{}</td>", c));
        }
        b.push_str("</tr>");
    }
    b.push_str("</table>");
    b
}

/// An ordinance counts as recorded when it has a date, a temple or a status (such as
/// "Child" or "Completed").
fn recorded(e: &Option<Event>) -> bool {
    e.as_ref().map_or(false, |e| [&e.date, &e.place, &e.temp, &e.stat].iter().any(|s| !s.trim().is_empty()))
}

impl App {
    /// The list `kind` (an index into LISTS) as a page to print.
    pub(crate) fn list_html(&self, kind: usize) -> String {
        let title = LISTS.get(kind).map(|l| l.0).unwrap_or("List");
        let db = self.print_db();
        let name = |id: u32| db.people.get(&id).map(|p| esc(&display_name(p))).unwrap_or_default();
        let mut people: Vec<&u32> = db.people.keys().collect();
        people.sort_by_key(|id| sort_key(&db.people[id]));
        let (intro, body) = match kind {
            0 => {
                let rows: Vec<Vec<String>> = people.iter().map(|id| {
                    let p = &db.people[id];
                    vec![id.to_string(), esc(&display_name(p)), dp(&p.bd, &p.bp), dp(&p.dd, &p.dp)]
                }).collect();
                (format!("{} people.", rows.len()), table(&["ID", "Name", "Born", "Died"], &rows))
            }
            1 => {
                let mut fams: Vec<(&u32, &Family)> = db.fams.iter().filter(|(_, f)| f.h.is_some() && f.w.is_some()).collect();
                fams.sort_by_key(|(_, f)| (f.h.and_then(|h| db.people.get(&h)).map(sort_key), parse_date(&f.md).0));
                let rows: Vec<Vec<String>> = fams.iter().map(|(fid, f)| {
                    vec![fid.to_string(), name(f.h.unwrap()), name(f.w.unwrap()), dp(&f.md, &f.mp)]
                }).collect();
                (format!("{} couples.", rows.len()), table(&["Family", "Husband", "Wife", "Married"], &rows))
            }
            2 => {
                let Some(root) = self.sel.or(self.db.home) else { return page(title, "<p>Choose a person first.</p>", false) };
                // Walk the ancestors that exist, numbering father 2n and mother 2n+1; someone who
                // appears on more than one line is listed once, with their lowest number.
                let mut seen: HashMap<u32, u128> = HashMap::new();
                let mut q = std::collections::VecDeque::from([(root, 1u128, 0usize)]);
                let mut rows = vec![];
                while let Some((id, n, g)) = q.pop_front() {
                    if seen.contains_key(&id) || g > 120 {
                        continue;
                    }
                    seen.insert(id, n);
                    let (f, m) = db.parents(id);
                    if f.is_none() && m.is_none() {
                        if let Some(p) = db.people.get(&id) {
                            rows.push((n, vec![n.to_string(), (g + 1).to_string(), esc(&display_name(p)), dp(&p.bd, &p.bp), dp(&p.dd, &p.dp)]));
                        }
                    }
                    if let Some(f) = f {
                        q.push_back((f, n.saturating_mul(2), g + 1));
                    }
                    if let Some(m) = m {
                        q.push_back((m, n.saturating_mul(2).saturating_add(1), g + 1));
                    }
                }
                rows.sort_by_key(|r| r.0);
                let rows: Vec<Vec<String>> = rows.into_iter().map(|r| r.1).collect();
                (
                    format!("For {}. {} lines stop here; the number is each person's place in the ancestor numbering (father = 2 &times; child, mother = 2 &times; child + 1).", name(root), rows.len()),
                    table(&["No.", "Generation", "Name", "Born", "Died"], &rows),
                )
            }
            3 => {
                let ix = db.index();
                let rows: Vec<Vec<String>> = people.iter().filter(|id| ix.child_of.get(id).map_or(true, |v| v.is_empty()) && ix.partner_in.get(id).map_or(true, |v| v.is_empty())).map(|id| {
                    let p = &db.people[id];
                    vec![id.to_string(), esc(&display_name(p)), dp(&p.bd, &p.bp), dp(&p.dd, &p.dp)]
                }).collect();
                (format!("{} people with no family links.", rows.len()), table(&["ID", "Name", "Born", "Died"], &rows))
            }
            4 => {
                let mut rows = vec![];
                for id in &people {
                    let Some(p) = self.db.people.get(id) else { continue };
                    if p.living {
                        continue;
                    }
                    let missing: Vec<String> = self
                        .lds_rows(**id)
                        .into_iter()
                        .filter(|r| r.kind != "LDS initiatory" && !recorded(&r.event))
                        .map(|r| esc(&r.label))
                        .collect();
                    if !missing.is_empty() {
                        rows.push(vec![id.to_string(), esc(&display_name(p)), dp(&p.bd, &p.bp), dp(&p.dd, &p.dp), missing.join("<br>")]);
                    }
                }
                (
                    format!("{} people who have died with ordinances not recorded. Living people are left out. An ordinance with a date, temple or status counts as recorded.", rows.len()),
                    table(&["ID", "Name", "Born", "Died", "Not recorded"], &rows),
                )
            }
            5 => {
                let rows: Vec<Vec<String>> = people.iter().filter_map(|id| {
                    let p = self.db.people.get(id)?;
                    if p.contact.is_empty() {
                        return None;
                    }
                    let c = &p.contact;
                    Some(vec![esc(&display_name(p)), esc(c.addr.trim()).replace('\n', "<br>"), esc(c.phone.trim()), esc(c.email.trim())])
                }).collect();
                (format!("{} people. Keep this list private: it holds living people's details.", rows.len()), table(&["Name", "Address", "Phone", "Email"], &rows))
            }
            _ => {
                let pairs = self.db.duplicate_pairs();
                let rows: Vec<Vec<String>> = pairs.iter().filter(|(a, b, _)| db.people.contains_key(a) && db.people.contains_key(b)).map(|(a, b, why)| {
                    let (pa, pb) = (&db.people[a], &db.people[b]);
                    vec![
                        format!("#{} {}<br><small>{}</small>", a, esc(&display_name(pa)), dp(&pa.bd, &pa.bp)),
                        format!("#{} {}<br><small>{}</small>", b, esc(&display_name(pb)), dp(&pb.bd, &pb.bp)),
                        esc(why),
                    ]
                }).collect();
                (format!("{} pairs to check. Merge them with Tools > Find duplicates.", rows.len()), table(&["Person", "Maybe the same as", "Why"], &rows))
            }
        };
        page(title, &format!("<p>{}</p>{}", intro, body), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_list_prints() {
        let mut db = crate::sample::sample_family();
        let some = *db.people.keys().next().unwrap();
        db.people.get_mut(&some).unwrap().contact.phone = "555-0100".into();
        let app = App::for_test(db);
        for k in 0..LISTS.len() {
            let h = app.list_html(k);
            assert!(h.contains("<table>"), "list {k}");
            assert!(h.contains(&esc(LISTS[k].0)), "list {k} has its title");
        }
        assert!(app.list_html(5).contains("555-0100"));
    }

    #[test]
    fn missing_ordinances_and_end_of_line() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME Ann /Lee/\n1 DEAT Y\n1 BAPL\n2 DATE 1 JAN 1900\n2 TEMP SLAKE\n1 FAMS @F1@\n0 @I2@ INDI\n1 NAME Bob /Lee/\n1 DEAT Y\n1 FAMS @F1@\n0 @I3@ INDI\n1 NAME Cal /Lee/\n1 FAMC @F1@\n1 BIRT\n2 DATE 1990\n0 @F1@ FAM\n1 HUSB @I2@\n1 WIFE @I1@\n1 CHIL @I3@\n0 TRLR\n";
        let db = gedcom::parse_ged(t);
        let mut app = App::for_test(db);
        let ann = *app.db.people.iter().find(|(_, p)| p.given == "Ann").unwrap().0;
        let cal = *app.db.people.iter().find(|(_, p)| p.given == "Cal").unwrap().0;
        app.db.people.get_mut(&cal).unwrap().living = true;
        app.settings.hide_living = false;
        let h = app.list_html(4);
        // Ann's baptism is recorded; her endowment is not. Cal is living, so left out.
        let ann_row = h.split("<tr>").find(|r| r.contains("Ann Lee")).unwrap();
        assert!(!ann_row.contains("Baptism") && ann_row.contains("Endowment"), "{ann_row}");
        assert!(!h.contains("Cal Lee"));
        app.sel = Some(cal);
        let e = app.list_html(2);
        assert!(e.contains("Ann Lee") && e.contains("Bob Lee") && !e.contains("Cal Lee</td>"), "{e}");
        let _ = ann;
    }
}
