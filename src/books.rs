//! Family history books: a person's ancestors or descendants written out as paragraphs,
//! generation by generation, with every person numbered and an index of names at the end.
//! Ancestors are numbered father = 2 x child and mother = 2 x child + 1; descendants are
//! numbered in the order they are written, with each child who has their own paragraph marked.

use super::*;

/// "on 12 March 1921", "in 1921", or "about 1921" as the date reads.
fn when(d: &str) -> String {
    let t = print_date(d);
    let t = t.trim();
    if t.is_empty() {
        return String::new();
    }
    let first = t.split_whitespace().next().unwrap_or("");
    let words = t.split_whitespace().count();
    if words == 1 && first.chars().all(|c| c.is_ascii_digit()) {
        format!("in {}", t)
    } else if first.chars().next().map_or(false, |c| c.is_ascii_digit()) || ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"].iter().any(|m| first.starts_with(m)) {
        if words == 2 && !first.chars().next().unwrap().is_ascii_digit() {
            format!("in {}", t)
        } else {
            format!("on {}", t)
        }
    } else {
        t.to_lowercase()
    }
}

/// "born on 1 May 1900 in Boise, Idaho" (the parts there are), escaped.
fn event_words(verb: &str, d: &str, p: &str) -> Option<String> {
    let w = when(d);
    let p = p.trim();
    if w.is_empty() && p.is_empty() {
        return None;
    }
    let mut s = verb.to_string();
    if !w.is_empty() {
        s.push(' ');
        s.push_str(&w);
    }
    if !p.is_empty() {
        s.push_str(" in ");
        s.push_str(p);
    }
    Some(esc(&s))
}

fn pronoun(p: &Person) -> &'static str {
    match p.sex.as_str() {
        "M" => "He",
        "F" => "She",
        _ => "They",
    }
}

/// The sentences about one person's own life: birth (or christening), death, burial.
fn life_sentences(p: &Person) -> String {
    let mut parts: Vec<String> = vec![];
    if let Some(b) = event_words("was born", &p.bd, &p.bp).or_else(|| event_words("was christened", &p.chd, &p.chp)) {
        parts.push(b);
    }
    let mut out = String::new();
    if !parts.is_empty() {
        out.push_str(&parts.join(" and "));
        out.push('.');
    }
    let mut later: Vec<String> = vec![];
    if let Some(d) = event_words("died", &p.dd, &p.dp) {
        later.push(d);
    }
    if let Some(b) = event_words("was buried", &p.bud, &p.bup) {
        later.push(b);
    }
    if !later.is_empty() {
        if !out.is_empty() {
            out.push(' ');
            out.push_str(pronoun(p));
            out.push(' ');
        }
        out.push_str(&later.join(" and "));
        out.push('.');
    }
    // Occupations and residences say a lot in a few words.
    for e in &p.events {
        let k = e.kind.to_lowercase();
        if (k == "occupation" || k == "occu") && !e.val.trim().is_empty() {
            out.push_str(&format!(" {} worked as {}{}.", pronoun(p), esc(e.val.trim()), event_words("", &e.date, &e.place).map(|s| s.to_string()).unwrap_or_default()));
        }
    }
    out
}

/// "Emily Rose Harwood" in bold, with her life in a few words after it.
fn short_life(p: &Person) -> String {
    let b = parse_date(&p.bd).0.map(|y| format!("b. {}", y));
    let d = parse_date(&p.dd).0.map(|y| format!("d. {}", y));
    let v: Vec<String> = [b, d].into_iter().flatten().collect();
    if v.is_empty() {
        esc(&display_name(p))
    } else {
        format!("{} ({})", esc(&display_name(p)), v.join(", "))
    }
}

const BOOK_CSS: &str = "p.who{margin:10px 0 2px}p.who b.n{display:inline-block;min-width:2.6em}\
    ol.kids{list-style:lower-roman;margin:2px 0 6px 28px;padding-left:12px}ol.kids li{margin:1px 0}\
    .idx{column-count:2;column-gap:24px;font-size:11.5px}.idx div{break-inside:avoid}\
    h2.gen{margin-top:18px;border-bottom:1px solid #ccc}";

impl App {
    /// Ancestors of `root`, written out generation by generation.
    pub(crate) fn ancestor_book_html(&self, root: u32) -> String {
        let db = self.print_db();
        let Some(rp) = db.people.get(&root) else { return String::new() };
        // number everyone who exists; someone on two lines keeps the lowest number
        let mut num: BTreeMap<u128, u32> = BTreeMap::new();
        let mut first: HashMap<u32, u128> = HashMap::new();
        let mut q = std::collections::VecDeque::from([(root, 1u128, 0u32)]);
        while let Some((id, n, g)) = q.pop_front() {
            if g > 100 {
                continue;
            }
            if first.contains_key(&id) {
                num.insert(n, id); // written as "see number ..."
                continue;
            }
            first.insert(id, n);
            num.insert(n, id);
            let (f, m) = db.parents(id);
            if let Some(f) = f {
                q.push_back((f, n.saturating_mul(2), g + 1));
            }
            if let Some(m) = m {
                q.push_back((m, n.saturating_mul(2).saturating_add(1), g + 1));
            }
        }
        let gen_of = |n: u128| 127 - n.leading_zeros();
        let mut b = format!("<p>The ancestors of {}, generation by generation. Each person has a number: a father is twice his child's number and a mother twice plus one, so the parents of number 5 are 10 and 11. The index at the end lists everyone by name.</p>", esc(&display_name(rp)));
        let mut last_gen = u32::MAX;
        let mut index: Vec<(String, String)> = vec![];
        for (n, id) in &num {
            let g = gen_of(*n);
            if g != last_gen {
                last_gen = g;
                let title = match g {
                    0 => "The first generation".to_string(),
                    1 => "Parents".to_string(),
                    2 => "Grandparents".to_string(),
                    3 => "Great-grandparents".to_string(),
                    k => format!("{} great-grandparents", ordinal(k as usize - 2)),
                };
                b.push_str(&format!("<h2 class=\"gen\">Generation {}: {}</h2>", g + 1, esc(&title)));
            }
            let p = &db.people[id];
            let fn_ = first[id];
            if fn_ != *n {
                b.push_str(&format!("<p class=\"who\"><b class=\"n\">{}.</b> {}: see number {}.</p>", n, esc(&display_name(p)), fn_));
                continue;
            }
            index.push((format!("{} {}", p.surname.to_lowercase(), p.given.to_lowercase()), format!("{} &mdash; {}", short_life(p), n)));
            let mut s = format!("<p class=\"who\"><b class=\"n\">{}.</b> <b>{}</b> {}", n, esc(&display_name(p)), life_sentences(p));
            // parents by number
            let (f, m) = db.parents(*id);
            let pn: Vec<String> = [f.map(|f| (f, n.saturating_mul(2))), m.map(|m| (m, n.saturating_mul(2).saturating_add(1)))]
                .into_iter()
                .flatten()
                .map(|(pid, k)| format!("{} ({})", esc(&display_name(&db.people[&pid])), first.get(&pid).copied().unwrap_or(k)))
                .collect();
            if !pn.is_empty() {
                s.push_str(&format!(" {} was the child of {}.", pronoun(p), pn.join(" and ")));
            }
            // marriages
            for f in db.fams_of(*id) {
                let fam = &db.fams[&f];
                let sp = if fam.h == Some(*id) { fam.w } else { fam.h };
                let Some(sp) = sp.and_then(|x| db.people.get(&x).map(|q| (x, q))) else { continue };
                let tag = first.get(&sp.0).map(|k| format!(" ({})", k)).unwrap_or_default();
                let w = event_words("", &fam.md, &fam.mp).unwrap_or_default();
                s.push_str(&format!(" {} married {}{}{}.", pronoun(p), esc(&display_name(sp.1)), tag, w));
            }
            s.push_str("</p>");
            if !p.notes.trim().is_empty() {
                s.push_str(&format!("<p class=\"note\">{}</p>", esc(p.notes.trim())));
            }
            b.push_str(&s);
        }
        b.push_str(&index_html(index));
        page_with_css(&format!("Ancestors of {}", display_name(rp)), &b, false, BOOK_CSS, true)
    }

    /// Descendants of `root`, written out generation by generation.
    pub(crate) fn descendant_book_html(&self, root: u32) -> String {
        let db = self.print_db();
        let Some(rp) = db.people.get(&root) else { return String::new() };
        // number people in the order they are written: generation by generation
        let mut order: Vec<(u32, u32)> = vec![(root, 0)];
        let mut numbered: HashMap<u32, usize> = HashMap::new();
        numbered.insert(root, 1);
        let mut i = 0;
        while i < order.len() {
            let (id, g) = order[i];
            if g < 50 {
                for f in db.fams_of(id) {
                    for k in &db.fams[&f].kids {
                        if db.people.contains_key(k) && !numbered.contains_key(k) {
                            numbered.insert(*k, numbered.len() + 1);
                            order.push((*k, g + 1));
                        }
                    }
                }
            }
            i += 1;
        }
        // only people with a family of their own (or the first person) get a paragraph
        let has_own = |id: u32| -> bool { db.fams_of(id).iter().any(|f| !db.fams[f].kids.is_empty() || db.fams[f].h.is_some() && db.fams[f].w.is_some()) };
        let mut para: HashMap<u32, usize> = HashMap::new();
        for (id, _) in &order {
            if *id == root || has_own(*id) {
                para.insert(*id, para.len() + 1);
            }
        }
        let mut b = format!("<p>The descendants of {}, generation by generation. Everyone with a family of their own has a numbered paragraph; in each list of children, a number after a name points to that child's paragraph. The index at the end lists everyone by name.</p>", esc(&display_name(rp)));
        let mut last_gen = u32::MAX;
        let mut index: Vec<(String, String)> = vec![];
        for (id, g) in &order {
            let p = &db.people[id];
            let Some(n) = para.get(id).copied() else {
                // indexed under their parents' paragraph
                let (f, m) = db.parents(*id);
                let at = [f, m].into_iter().flatten().find_map(|x| para.get(&x)).map(|k| k.to_string()).unwrap_or_default();
                index.push((format!("{} {}", p.surname.to_lowercase(), p.given.to_lowercase()), format!("{} &mdash; child in {}", short_life(p), at)));
                continue;
            };
            if *g != last_gen {
                last_gen = *g;
                b.push_str(&format!("<h2 class=\"gen\">Generation {}</h2>", g + 1));
            }
            index.push((format!("{} {}", p.surname.to_lowercase(), p.given.to_lowercase()), format!("{} &mdash; {}", short_life(p), n)));
            let mut s = format!("<p class=\"who\"><b class=\"n\">{}.</b> <b>{}</b> {}", n, esc(&display_name(p)), life_sentences(p));
            let (f, m) = db.parents(*id);
            if *id != root {
                let pn: Vec<String> = [f, m].into_iter().flatten().map(|x| {
                    let k = para.get(&x).map(|k| format!(" ({})", k)).unwrap_or_default();
                    format!("{}{}", esc(&display_name(&db.people[&x])), k)
                }).collect();
                if !pn.is_empty() {
                    s.push_str(&format!(" {} was the child of {}.", pronoun(p), pn.join(" and ")));
                }
            }
            s.push_str("</p>");
            if !p.notes.trim().is_empty() {
                s.push_str(&format!("<p class=\"note\">{}</p>", esc(p.notes.trim())));
            }
            for f in db.fams_of(*id) {
                let fam = &db.fams[&f];
                let sp = if fam.h == Some(*id) { fam.w } else { fam.h };
                let spouse = sp.and_then(|x| db.people.get(&x));
                if let Some(q) = spouse {
                    let w = event_words("", &fam.md, &fam.mp).unwrap_or_default();
                    let life = life_sentences(q);
                    s.push_str(&format!("<p>{} married <b>{}</b>{}.{}</p>", pronoun(p), esc(&display_name(q)), w, if life.is_empty() { String::new() } else { format!(" {} {}", esc(&display_name(q)), life) }));
                    index.push((format!("{} {}", q.surname.to_lowercase(), q.given.to_lowercase()), format!("{} &mdash; spouse in {}", short_life(q), n)));
                }
                let kids: Vec<&u32> = fam.kids.iter().filter(|k| db.people.contains_key(k)).collect();
                if !kids.is_empty() {
                    let with = spouse.map(|q| format!(" and {}", esc(&display_name(q)))).unwrap_or_default();
                    s.push_str(&format!("<p>Children of {}{}:</p><ol class=\"kids\">", esc(&display_name(p)), with));
                    for k in kids {
                        let c = &db.people[k];
                        let more = para.get(k).map(|n| format!(" &nbsp;<b>&rarr; {}</b>", n)).unwrap_or_default();
                        s.push_str(&format!("<li>{}{}</li>", short_life(c), more));
                    }
                    s.push_str("</ol>");
                }
            }
            b.push_str(&s);
        }
        b.push_str(&index_html(index));
        page_with_css(&format!("Descendants of {}", display_name(rp)), &b, false, BOOK_CSS, true)
    }
}

fn index_html(mut index: Vec<(String, String)>) -> String {
    index.sort();
    index.dedup();
    let mut b = String::from("<h2 class=\"gen\">Index of names</h2><div class=\"idx\">");
    for (_, line) in index {
        b.push_str(&format!("<div>{}</div>", line));
    }
    b.push_str("</div>");
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_read_naturally() {
        assert_eq!(when("1921"), "in 1921");
        assert!(when("12 MAR 1921").starts_with("on "), "{}", when("12 MAR 1921"));
        assert!(when("MAR 1921").starts_with("in "), "{}", when("MAR 1921"));
        assert!(when("ABT 1900").starts_with("ab"), "{}", when("ABT 1900"));
        assert_eq!(when(""), "");
    }

    #[test]
    fn books_number_everyone_and_index_them() {
        let db = crate::sample::sample_family();
        let mut app = App::for_test(db);
        app.settings.hide_living = false;
        let emily = *app.db.people.iter().find(|(_, p)| p.given.starts_with("Emily")).unwrap().0;
        let a = app.ancestor_book_html(emily);
        assert!(a.contains("<b class=\"n\">1.</b> <b>Emily Rose Harwood</b>"), "Emily is number 1");
        assert!(a.contains("<b class=\"n\">2.</b>") && a.contains("<b class=\"n\">3.</b>"), "her parents are 2 and 3");
        assert!(a.contains("Index of names"));
        let anc = app.db.ancestors(emily).len();
        assert!(a.matches("<div>").count() >= anc, "everyone indexed");
        // descendants of the oldest Harwood reach Emily
        let william = *app.db.people.iter().find(|(_, p)| p.name == "William Harwood").unwrap().0;
        let d = app.descendant_book_html(william);
        assert!(d.contains("<b class=\"n\">1.</b> <b>William Harwood</b>"));
        assert!(d.contains("Emily Rose Harwood"));
        assert!(d.contains("Children of William Harwood"));
        assert!(d.contains("&rarr;"), "children with their own paragraph are pointed to");
    }
}
