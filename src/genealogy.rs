//! Working with the family data: names, dates, relationships, merging and places.

use super::*;

impl Db {
    /// Visit every citation in the tree with its owner and what it supports.
    pub(crate) fn each_cite(&self, f: &mut dyn FnMut(Owner, Claim, &Source)) {
        for (id, p) in &self.people {
            let o = Owner::Person(*id);
            for c in &p.sources {
                f(o, Claim { what: if c.about.is_empty() { "General" } else { c.about.as_str() }, when: "" }, c);
            }
            for (k, e) in &p.fx {
                for c in &e.cites {
                    f(o, Claim { what: k.as_str(), when: e.date.as_str() }, c);
                }
            }
            for e in &p.events {
                for c in &e.cites {
                    f(o, Claim { what: e.kind.as_str(), when: e.date.as_str() }, c);
                }
            }
            for a in &p.alt_names {
                for c in &a.cites {
                    f(o, Claim { what: "Other name", when: "" }, c);
                }
            }
        }
        for (id, fm) in &self.fams {
            let o = Owner::Family(*id);
            for c in &fm.cites {
                f(o, Claim { what: "Family", when: "" }, c);
            }
            for c in &fm.mx.cites {
                f(o, Claim { what: "Marriage", when: fm.md.as_str() }, c);
            }
            for e in &fm.events {
                for c in &e.cites {
                    f(o, Claim { what: e.kind.as_str(), when: e.date.as_str() }, c);
                }
            }
        }
    }

    pub(crate) fn each_cite_mut(&mut self, f: &mut dyn FnMut(&mut Source)) {
        for p in self.people.values_mut() {
            p.sources.iter_mut().for_each(&mut *f);
            for e in p.fx.values_mut() {
                e.cites.iter_mut().for_each(&mut *f);
            }
            for e in p.events.iter_mut() {
                e.cites.iter_mut().for_each(&mut *f);
            }
            for a in p.alt_names.iter_mut() {
                a.cites.iter_mut().for_each(&mut *f);
            }
        }
        for fm in self.fams.values_mut() {
            fm.cites.iter_mut().for_each(&mut *f);
            fm.mx.cites.iter_mut().for_each(&mut *f);
            for e in fm.events.iter_mut() {
                e.cites.iter_mut().for_each(&mut *f);
            }
        }
    }

    pub(crate) fn source_counts(&self) -> HashMap<u32, usize> {
        let mut n: HashMap<u32, usize> = HashMap::new();
        self.each_cite(&mut |_, _, c| {
            if let Some(s) = c.sid {
                *n.entry(s).or_insert(0) += 1;
            }
        });
        n
    }

    /// Everything that cites a source: (who, what it supports, date, page).
    pub(crate) fn source_uses(&self, sid: u32) -> Vec<(Owner, String, String, String)> {
        let mut out = Vec::new();
        self.each_cite(&mut |o, cl, c| {
            if c.sid == Some(sid) {
                out.push((o, cl.what.to_string(), cl.when.to_string(), c.page.clone()));
            }
        });
        out
    }

    pub(crate) fn owner_label(&self, o: Owner) -> String {
        match o {
            Owner::Person(i) => self.people.get(&i).map(|_| self.label(i)).unwrap_or_default(),
            Owner::Family(i) => match self.fams.get(&i) {
                Some(f) => {
                    let n = |x: Option<u32>| x.and_then(|i| self.people.get(&i)).map(display_name).unwrap_or_else(|| "?".to_string());
                    format!("Family: {} and {}", n(f.h), n(f.w))
                }
                None => String::new(),
            },
        }
    }

    /// A citation written out in full: source, page, where it is kept, web address and access date.
    pub(crate) fn cite_text(&self, c: &Source) -> String {
        let mut parts: Vec<String> = Vec::new();
        match c.sid.and_then(|i| self.sources.get(&i)) {
            Some(s) => {
                let mut head = source_title(s);
                if !s.author.is_empty() {
                    head = format!("{}, {}", s.author, head);
                }
                if !s.publ.is_empty() {
                    head = format!("{} ({})", head, s.publ);
                }
                parts.push(head);
                if !c.page.is_empty() {
                    parts.push(c.page.clone());
                }
                if let Some(r) = s.repo.and_then(|r| self.repos.get(&r)) {
                    let mut w = r.name.clone();
                    if !s.call.is_empty() {
                        w = format!("{}, {}", w, s.call);
                    }
                    parts.push(w);
                } else if !s.call.is_empty() {
                    parts.push(s.call.clone());
                }
                let url = if !c.url.is_empty() { &c.url } else { &s.url };
                if !url.is_empty() {
                    parts.push(url.clone());
                }
            }
            None => {
                parts.push(c.text.clone());
                if !c.page.is_empty() {
                    parts.push(c.page.clone());
                }
                if !c.url.is_empty() {
                    parts.push(c.url.clone());
                }
            }
        }
        let mut s = parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(". ");
        if !c.accessed.is_empty() {
            s.push_str(&format!(" (accessed {})", c.accessed));
        }
        s
    }

    pub(crate) fn delete_source(&mut self, id: u32) {
        let title = self.sources.get(&id).map(source_title).unwrap_or_default();
        self.each_cite_mut(&mut |c| {
            if c.sid == Some(id) {
                c.sid = None;
                c.text = title.clone();
            }
        });
        self.sources.remove(&id);
        replace_pointer(self, &format!("S{}", id), None);
    }

    /// Fold source `gone` into `keep`: every citation of `gone` now cites `keep`.
    pub(crate) fn merge_sources(&mut self, keep: u32, gone: u32) {
        if keep == gone {
            return;
        }
        let Some(g) = self.sources.remove(&gone) else { return };
        let title = self.sources.get(&keep).map(source_title).unwrap_or_default();
        self.each_cite_mut(&mut |c| {
            if c.sid == Some(gone) {
                c.sid = Some(keep);
                c.text = title.clone();
            }
        });
        if let Some(k) = self.sources.get_mut(&keep) {
            let fill = |a: &mut String, b: String| {
                if a.is_empty() {
                    *a = b;
                }
            };
            fill(&mut k.title, g.title);
            fill(&mut k.author, g.author);
            fill(&mut k.abbr, g.abbr);
            fill(&mut k.publ, g.publ);
            fill(&mut k.kind, g.kind);
            fill(&mut k.url, g.url);
            fill(&mut k.call, g.call);
            fill(&mut k.medium, g.medium);
            fill(&mut k.text, g.text);
            if k.repo.is_none() {
                k.repo = g.repo;
            }
            if !g.notes.is_empty() && !k.notes.contains(&g.notes) {
                if !k.notes.is_empty() {
                    k.notes.push('\n');
                }
                k.notes.push_str(&g.notes);
            }
            for n in g.note_refs {
                if !k.note_refs.contains(&n) {
                    k.note_refs.push(n);
                }
            }
            k.extra.extend(g.extra);
        }
        replace_pointer(self, &format!("S{}", gone), Some(&format!("S{}", keep)));
    }

    pub(crate) fn delete_repo(&mut self, id: u32) {
        for s in self.sources.values_mut() {
            if s.repo == Some(id) {
                s.repo = None;
            }
        }
        self.repos.remove(&id);
        replace_pointer(self, &format!("R{}", id), None);
    }
}

pub(crate) fn split_name(name: &str) -> (String, String) {
    let w: Vec<&str> = name.split_whitespace().collect();
    if w.len() > 1 {
        (w[..w.len() - 1].join(" "), w[w.len() - 1].to_string())
    } else {
        (name.trim().to_string(), String::new())
    }
}

pub(crate) fn compose_name(given: &str, surname: &str) -> String {
    format!("{} {}", given.trim(), surname.trim()).trim().to_string()
}

pub(crate) fn fix_names(db: &mut Db) {
    for p in db.people.values_mut() {
        if p.given.is_empty() && p.surname.is_empty() && !p.name.is_empty() {
            let (g, s) = split_name(&p.name);
            p.given = g;
            p.surname = s;
        }
    }
}

/// How names and dates are shown on screen (View > Names and dates). Display only: what is
/// stored and exported never changes.
pub(crate) static NAME_STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub(crate) static DATE_STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// Printouts on A4 paper (from the settings, read when a page is made)
pub(crate) static PAPER_A4: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The map colours chosen under Map style (see mapview::MAP_STYLES).
pub(crate) static MAP_STYLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// The app's background and accent colours (0xRRGGBB), so "Match the app" maps follow them.
pub(crate) static APP_BG: AtomicU32 = AtomicU32::new(0x1e1f22);
pub(crate) static APP_ACCENT: AtomicU32 = AtomicU32::new(0x3b6fd4);

/// A name in the chosen style: "John Henry Smith", "SMITH, John Henry" or "Smith, John Henry".
pub(crate) fn styled_name(p: &Person) -> String {
    styled_name_as(p, NAME_STYLE.load(Ordering::Relaxed))
}

pub(crate) fn styled_name_as(p: &Person, style: u8) -> String {
    if style == 0 || p.surname.trim().is_empty() {
        return if p.name.is_empty() { "Unnamed".to_string() } else { p.name.clone() };
    }
    let sur = if style == 1 { p.surname.trim().to_uppercase() } else { p.surname.trim().to_string() };
    if p.given.trim().is_empty() {
        sur
    } else {
        format!("{}, {}", sur, p.given.trim())
    }
}

/// A date in the chosen style. Dates with words in them ("about 1850", "between...") are left
/// as written; only plain day-month-year dates are re-written.
pub(crate) fn show_date(s: &str) -> String {
    show_date_as(s, DATE_STYLE.load(Ordering::Relaxed))
}

pub(crate) fn show_date_as(s: &str, style: u8) -> String {
    let t = s.trim();
    if style == 0 || t.is_empty() {
        return t.to_string();
    }
    const FULL: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const SHORT: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let words = t.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).count();
    let (y, m, d) = parse_date(t);
    match (y, m, d) {
        (Some(y), Some(m), Some(d)) if words == 3 && (1..=12).contains(&m) => match style {
            1 => format!("{} {} {}", d, SHORT[m as usize - 1], y),
            2 => format!("{} {}, {}", FULL[m as usize - 1], d, y),
            _ => format!("{:04}-{:02}-{:02}", y, m, d),
        },
        (Some(y), Some(m), None) if words == 2 && (1..=12).contains(&m) => match style {
            1 => format!("{} {}", SHORT[m as usize - 1], y),
            2 => format!("{} {}", FULL[m as usize - 1], y),
            _ => format!("{:04}-{:02}", y, m),
        },
        _ => t.to_string(),
    }
}

pub(crate) fn display_name(p: &Person) -> String {
    let n = styled_name(p);
    if p.maiden.is_empty() { n } else { format!("{} (née {})", n, p.maiden) }
}

pub(crate) fn pick_sex<'a>(sex: &str, neutral: &'a str, male: &'a str, female: &'a str) -> &'a str {
    match sex {
        "M" => male,
        "F" => female,
        _ => neutral,
    }
}

pub(crate) fn legal_parent_word(rel: &str, sx: &str) -> String {
    match rel.to_lowercase().as_str() {
        "adopted" => format!("adoptive {}", pick_sex(sx, "parent", "father", "mother")),
        "step" => pick_sex(sx, "step-parent", "stepfather", "stepmother").to_string(),
        "foster" => format!("foster {}", pick_sex(sx, "parent", "father", "mother")),
        "guardian" => "legal guardian".to_string(),
        "sealed" => format!("sealed {}", pick_sex(sx, "parent", "father", "mother")),
        other => format!("{} ({})", pick_sex(sx, "parent", "father", "mother"), other),
    }
}

pub(crate) fn legal_child_word(rel: &str, sx: &str) -> String {
    match rel.to_lowercase().as_str() {
        "adopted" => format!("adopted {}", pick_sex(sx, "child", "son", "daughter")),
        "step" => pick_sex(sx, "stepchild", "stepson", "stepdaughter").to_string(),
        "foster" => format!("foster {}", pick_sex(sx, "child", "son", "daughter")),
        "guardian" => "ward".to_string(),
        "sealed" => format!("sealed {}", pick_sex(sx, "child", "son", "daughter")),
        other => format!("{} ({})", pick_sex(sx, "child", "son", "daughter"), other),
    }
}

/// Name a relationship from distances to the nearest common ancestor.
pub(crate) fn kin_term(da: i32, db: i32, sx: &str, half: bool) -> String {
    if da == 0 {
        if db == 1 {
            pick_sex(sx, "child", "son", "daughter").to_string()
        } else {
            format!("{}{}", greats(db - 2), pick_sex(sx, "grandchild", "grandson", "granddaughter"))
        }
    } else if db == 0 {
        if da == 1 {
            pick_sex(sx, "parent", "father", "mother").to_string()
        } else {
            format!("{}{}", greats(da - 2), pick_sex(sx, "grandparent", "grandfather", "grandmother"))
        }
    } else if da == 1 && db == 1 {
        format!("{}{}", if half { "half-" } else { "" }, pick_sex(sx, "sibling", "brother", "sister"))
    } else if da == 1 {
        format!("{}{}", greats(db - 2), pick_sex(sx, "niece or nephew", "nephew", "niece"))
    } else if db == 1 {
        format!("{}{}", greats(da - 2), pick_sex(sx, "aunt or uncle", "uncle", "aunt"))
    } else {
        let k = da.min(db) - 1;
        let removed = (da - db).abs();
        let rm = match removed {
            0 => String::new(),
            1 => " once removed".to_string(),
            2 => " twice removed".to_string(),
            n => format!(" {} times removed", n),
        };
        format!("{} cousin{}", ordinal(k as usize), rm)
    }
}

pub(crate) fn greats(n: i32) -> String {
    match n {
        n if n <= 0 => String::new(),
        1 => "great-".to_string(),
        n => format!("{} great-", ordinal(n as usize)),
    }
}

pub(crate) fn year(s: &str) -> String {
    s.split(|c: char| !c.is_ascii_digit())
        .find(|t| t.len() == 4)
        .unwrap_or("")
        .to_string()
}

pub(crate) fn life(p: &Person) -> String {
    let (b, d) = (year(&p.bd), year(&p.dd));
    if b.is_empty() && d.is_empty() {
        String::new()
    } else {
        format!("{}-{}", if b.is_empty() { "?" } else { b.as_str() }, d)
    }
}

impl Db {
    pub(crate) fn families_count(&self) -> usize {
        self.fams.len()
    }
    pub(crate) fn alloc(&mut self) -> u32 {
        self.next += 1;
        self.next
    }
    /// Every family the person is a child of, best first: the one chosen for the chart, then
    /// families where both parents are biological, then the rest.
    /// Lookup tables so big trees do not rescan every family for every question.
    /// Build once for a whole chart, report or check; rebuild after the data changes.
    pub(crate) fn index(&self) -> Idx {
        let mut ix = Idx::default();
        for (fid, f) in &self.fams {
            for k in &f.kids {
                ix.child_of.entry(*k).or_default().push(*fid);
            }
            for p in f.h.iter().chain(f.w.iter()).chain(f.more.iter().map(|(_, x)| x)) {
                let e = ix.partner_in.entry(*p).or_default();
                if !e.contains(fid) {
                    e.push(*fid);
                }
            }
        }
        for (p, v) in ix.partner_in.iter_mut() {
            v.sort_by_key(|f| self.fam_rank(*p, *f));
        }
        ix
    }
    pub(crate) fn parent_families_ix(&self, ix: &Idx, id: u32) -> Vec<u32> {
        let cands: &[u32] = ix.child_of.get(&id).map(|v| v.as_slice()).unwrap_or(&[]);
        self.rank_parent_fams(id, cands.iter().copied())
    }
    pub(crate) fn fams_of_ix<'a>(&self, ix: &'a Idx, id: u32) -> &'a [u32] {
        ix.partner_in.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub(crate) fn parents_ix(&self, ix: &Idx, id: u32) -> (Option<u32>, Option<u32>) {
        match self.parent_families_ix(ix, id).first() {
            Some(f) => (self.fams[f].h, self.fams[f].w),
            None => (None, None),
        }
    }
    pub(crate) fn parent_families(&self, id: u32) -> Vec<u32> {
        self.rank_parent_fams(id, self.fams.iter().filter(|(_, f)| f.kids.contains(&id)).map(|(k, _)| *k))
    }
    pub(crate) fn rank_parent_fams(&self, id: u32, cands: impl Iterator<Item = u32>) -> Vec<u32> {
        let pref = self.people.get(&id).and_then(|p| p.pref_parents);
        let mut v: Vec<(u8, u32)> = cands
            .filter_map(|k| self.fams.get(&k).map(|f| (k, f)))
            .map(|(k, f)| {
                let bio = rel_is_bio(&f.rel_to(id, true)) as u8 + rel_is_bio(&f.rel_to(id, false)) as u8;
                let rank = if pref == Some(k) { 0 } else { 3 - bio };
                (rank, k)
            })
            .collect();
        v.sort();
        v.into_iter().map(|(_, k)| k).collect()
    }
    pub(crate) fn famc(&self, id: u32) -> Option<u32> {
        self.parent_families(id).first().copied()
    }
    /// Families the person is a partner in, in the order of the partnerships (by marriage year,
    /// those without a date last).
    pub(crate) fn fams_of(&self, id: u32) -> Vec<u32> {
        let mut v: Vec<((usize, i32, u32), u32)> = self
            .fams
            .iter()
            .filter(|(_, f)| f.h == Some(id) || f.w == Some(id) || f.more.iter().any(|(_, x)| *x == id))
            .map(|(k, _)| (self.fam_rank(id, *k), *k))
            .collect();
        v.sort();
        v.into_iter().map(|(_, k)| k).collect()
    }
    /// Where a family comes in a person's list of spouses: the order chosen by hand (or from the
    /// file) first, then by marriage date, then by number.
    pub(crate) fn fam_rank(&self, person: u32, fid: u32) -> (usize, i32, u32) {
        let pos = self.people.get(&person).and_then(|p| p.fam_order.iter().position(|f| *f == fid)).unwrap_or(usize::MAX);
        let y = self.fams.get(&fid).and_then(|f| parse_date(&f.md).0).unwrap_or(i32::MAX);
        (pos, y, fid)
    }
    /// Move a person's family (spouse) one place earlier or later in their list.
    pub(crate) fn move_family(&mut self, person: u32, fid: u32, earlier: bool) -> bool {
        let mut v = self.fams_of(person);
        let Some(i) = v.iter().position(|f| *f == fid) else { return false };
        let j = if earlier { i.checked_sub(1) } else { Some(i + 1).filter(|j| *j < v.len()) };
        let Some(j) = j else { return false };
        v.swap(i, j);
        match self.people.get_mut(&person) {
            Some(p) => {
                p.fam_order = v;
                true
            }
            None => false,
        }
    }
    /// Move a child one place earlier or later among the family's children.
    pub(crate) fn move_child(&mut self, fid: u32, kid: u32, earlier: bool) -> bool {
        let Some(f) = self.fams.get_mut(&fid) else { return false };
        let Some(i) = f.kids.iter().position(|k| *k == kid) else { return false };
        let j = if earlier { i.checked_sub(1) } else { Some(i + 1).filter(|j| *j < f.kids.len()) };
        let Some(j) = j else { return false };
        f.kids.swap(i, j);
        true
    }
    /// Put a family's children in order of birth. Children with no birth date keep their place
    /// after the ones before them.
    pub(crate) fn sort_children(&mut self, fid: u32) -> bool {
        let Some(f) = self.fams.get(&fid) else { return false };
        let key = |k: &u32| -> (i32, u32, u32) {
            let Some(p) = self.people.get(k) else { return (i32::MAX, 13, 32) };
            let d = if p.bd.trim().is_empty() { &p.chd } else { &p.bd };
            let (y, m, dd) = parse_date(d);
            (y.unwrap_or(i32::MAX), m.unwrap_or(13), dd.unwrap_or(32))
        };
        // Undated children stay just after the dated child they followed.
        let mut last = (i32::MIN, 0, 0);
        let mut keyed: Vec<((i32, u32, u32), usize, u32)> = Vec::new();
        for (i, k) in f.kids.iter().enumerate() {
            let mut kk = key(k);
            if kk.0 == i32::MAX {
                kk = last;
            } else {
                last = kk;
            }
            keyed.push((kk, i, *k));
        }
        keyed.sort();
        let new: Vec<u32> = keyed.into_iter().map(|(_, _, k)| k).collect();
        let f = self.fams.get_mut(&fid).unwrap();
        if f.kids == new {
            return false;
        }
        f.kids = new;
        true
    }
    /// Everyone who is a parent of `id` through any family, with how: (parent, relationship word).
    /// Brothers and sisters (full and half) in the order of the parents' family lists.
    pub(crate) fn siblings(&self, id: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for f in self.parent_families(id) {
            for k in &self.fams[&f].kids {
                if *k != id && !out.contains(k) && self.people.contains_key(k) {
                    out.push(*k);
                }
            }
        }
        // half-brothers and sisters: other children of any of the parents
        for (p, _) in self.parent_links(id) {
            for k in self.children_of(p) {
                if k != id && !out.contains(&k) {
                    out.push(k);
                }
            }
        }
        out
    }
    pub(crate) fn partners(&self, id: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for f in self.fams_of(id) {
            let fm = &self.fams[&f];
            for p in fm.h.iter().chain(fm.w.iter()) {
                if *p != id && !out.contains(p) && self.people.contains_key(p) {
                    out.push(*p);
                }
            }
        }
        out
    }
    pub(crate) fn children_of(&self, id: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for f in self.fams_of(id) {
            for k in &self.fams[&f].kids {
                if !out.contains(k) && self.people.contains_key(k) {
                    out.push(*k);
                }
            }
        }
        out
    }
    pub(crate) fn parent_links(&self, id: u32) -> Vec<(u32, String)> {
        let mut out: Vec<(u32, String)> = Vec::new();
        for fid in self.parent_families(id) {
            let f = &self.fams[&fid];
            for (p, father) in [(f.h, true), (f.w, false)] {
                if let Some(p) = p {
                    if !out.iter().any(|(q, _)| *q == p) {
                        out.push((p, f.rel_to(id, father)));
                    }
                }
            }
        }
        out
    }
    /// Step-parents (a parent's partner who is not a parent), half-siblings (one parent in common),
    /// step-siblings (children of a step-parent) and step-children (a partner's children). Worked out
    /// from the tree; nothing is stored.
    pub(crate) fn kin_extras(&self, id: u32) -> Kin {
        let parents: Vec<u32> = self.parent_links(id).into_iter().map(|(p, _)| p).collect();
        let mut k = Kin::default();
        let kids_of = |p: u32| -> Vec<u32> { self.fams_of(p).into_iter().flat_map(|f| self.fams[&f].kids.clone()).collect() };
        let partners_of = |p: u32| -> Vec<u32> {
            self.fams_of(p)
                .into_iter()
                .flat_map(|f| {
                    let fm = &self.fams[&f];
                    [fm.h, fm.w].into_iter().flatten().filter(|x| *x != p).collect::<Vec<_>>()
                })
                .collect()
        };
        let push = |v: &mut Vec<u32>, x: u32| {
            if x != id && !v.contains(&x) {
                v.push(x);
            }
        };
        for p in &parents {
            for s in kids_of(*p) {
                let theirs: Vec<u32> = self.parent_links(s).into_iter().map(|(q, _)| q).collect();
                let shared = theirs.iter().filter(|q| parents.contains(q)).count();
                if shared == 1 {
                    push(&mut k.half_siblings, s);
                }
            }
            for q in partners_of(*p) {
                if !parents.contains(&q) && q != id {
                    push(&mut k.step_parents, q);
                    for s in kids_of(q) {
                        let theirs: Vec<u32> = self.parent_links(s).into_iter().map(|(x, _)| x).collect();
                        if !theirs.iter().any(|x| parents.contains(x)) {
                            push(&mut k.step_siblings, s);
                        }
                    }
                }
            }
        }
        let mine = kids_of(id);
        for q in partners_of(id) {
            for c in kids_of(q) {
                if !mine.contains(&c) && !self.parent_links(c).iter().any(|(x, _)| *x == id) {
                    push(&mut k.step_children, c);
                }
            }
        }
        // People who have this person as a guardian, godparent and so on.
        for (oid, op) in &self.people {
            for a in &op.assoc {
                if a.other == id && *oid != id {
                    k.assoc_of.push((*oid, a.role.clone()));
                }
            }
        }
        k
    }
    /// Put `pid` into a parent slot for `child`. `fid` = an existing family the child
    /// already belongs to; None starts a new, separate set of parents (adoptive, step...).
    pub(crate) fn add_parent(&mut self, child: u32, fid: Option<u32>, father: bool, pid: u32) {
        if pid == child {
            return;
        }
        match fid.filter(|f| self.fams.contains_key(f)) {
            Some(f) => {
                let fm = self.fams.get_mut(&f).unwrap();
                if !fm.kids.contains(&child) {
                    fm.kids.push(child);
                }
                if father { fm.h = Some(pid) } else { fm.w = Some(pid) }
            }
            None => {
                let nf = self.alloc();
                let mut fm = Family { kids: vec![child], ..Default::default() };
                if father { fm.h = Some(pid) } else { fm.w = Some(pid) }
                // anything but the first set of parents defaults to "not biological" only when the user says so
                self.fams.insert(nf, fm);
            }
        }
    }
    /// Take a child out of one parent family (the family and the people stay).
    /// " (adopted)", " (step-child of mother)" ... or empty for an ordinary biological child.
    pub(crate) fn child_note(&self, fid: u32, child: u32) -> String {
        let Some(f) = self.fams.get(&fid) else { return String::new() };
        let fr = f.rel_to(child, true);
        let mr = f.rel_to(child, false);
        if rel_is_bio(&fr) && rel_is_bio(&mr) {
            return String::new();
        }
        if rel_name(&fr) == rel_name(&mr) {
            return format!(" ({})", rel_name(&fr).to_lowercase());
        }
        let mut parts = Vec::new();
        if f.h.is_some() { parts.push(format!("{} to father", rel_name(&fr).to_lowercase())); }
        if f.w.is_some() { parts.push(format!("{} to mother", rel_name(&mr).to_lowercase())); }
        format!(" ({})", parts.join(", "))
    }
    pub(crate) fn remove_from_family(&mut self, child: u32, fid: u32) {
        if let Some(f) = self.fams.get_mut(&fid) {
            f.kids.retain(|k| *k != child);
            f.pedi.remove(&child);
            f.frel.remove(&child);
            f.mrel.remove(&child);
        }
        if let Some(p) = self.people.get_mut(&child) {
            if p.pref_parents == Some(fid) {
                p.pref_parents = None;
            }
        }
    }
    pub(crate) fn parents(&self, id: u32) -> (Option<u32>, Option<u32>) {
        match self.famc(id) {
            Some(f) => {
                let f = &self.fams[&f];
                (f.h, f.w)
            }
            None => (None, None),
        }
    }
    pub(crate) fn timeline(&self, id: u32) -> Vec<(String, String)> {
        let p = &self.people[&id];
        let by = parse_date(&p.bd).0;
        let mut items: Vec<(i32, String, String)> = Vec::new();
        let mut add = |date: &str, place: &str, what: String, is_birth: bool| {
            if date.trim().is_empty() && place.trim().is_empty() {
                return;
            }
            let (y, m, d) = parse_date(date);
            let key = y.map(|y| y * 400 + m.unwrap_or(0) as i32 * 32 + d.unwrap_or(0) as i32).unwrap_or(i32::MAX);
            let age = match (by, y) {
                (Some(b), Some(y)) if !is_birth && y >= b => format!(" (age {})", y - b),
                _ => String::new(),
            };
            let pl = if place.is_empty() { String::new() } else { format!(" - {}", place) };
            items.push((key, date.to_string(), format!("{}{}{}", what, pl, age)));
        };
        add(&p.bd, &p.bp, "Born".to_string(), true);
        add(&p.chd, &p.chp, "Christened".to_string(), false);
        add(&p.dd, &p.dp, "Died".to_string(), false);
        add(&p.bud, &p.bup, "Buried".to_string(), false);
        for fid in self.fams_of(id) {
            let f = &self.fams[&fid];
            let sp = if f.h == Some(id) { f.w } else { f.h };
            let who = sp.map(|s| self.people[&s].name.clone()).unwrap_or_default();
            add(&f.md, &f.mp, format!("Married {}", who).trim().to_string(), false);
            for k in &f.kids {
                let c = &self.people[k];
                add(&c.bd, &c.bp, format!("Child born: {}", c.name), false);
            }
        }
        for e in &p.events {
            let detail = [e.val.as_str(), e.note.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(". ");
            let note = if detail.is_empty() { String::new() } else { format!(": {}", detail) };
            add(&e.date, &e.place, format!("{}{}", e.kind, note), false);
        }
        items.sort_by_key(|x| x.0);
        items.into_iter().map(|(_, d, t)| (d, t)).collect()
    }

    pub(crate) fn check(&self) -> Vec<(u32, String)> {
        let cy = current_year();
        let mut v: Vec<(u32, String)> = Vec::new();
        let ix = self.index();
        for (id, p) in &self.people {
            let by = parse_date(&p.bd).0;
            let dy = parse_date(&p.dd).0;
            if p.bd.trim().is_empty() {
                v.push((*id, "No birth date recorded".to_string()));
            }
            match (by, dy) {
                (Some(b), Some(d)) => {
                    if d < b {
                        v.push((*id, "Died before they were born".to_string()));
                    } else if d - b > 110 {
                        v.push((*id, format!("Age at death would be {}", d - b)));
                    }
                }
                (Some(b), None) => {
                    if !p.living && cy - b > 110 {
                        v.push((*id, "Born over 110 years ago with no death date (add one, or tick Living)".to_string()));
                    }
                }
                _ => {}
            }
            if let Some(b) = by {
                if b > cy {
                    v.push((*id, "Birth year is in the future".to_string()));
                }
            }
            let (f, m) = self.parents_ix(&ix, *id);
            for (pid, role) in [(f, "father"), (m, "mother")] {
                let (Some(pid), Some(b)) = (pid, by) else { continue };
                let par = &self.people[&pid];
                if let Some(pb) = parse_date(&par.bd).0 {
                    if b < pb {
                        v.push((*id, format!("Born before their {} was born", role)));
                    } else if b - pb < 13 {
                        v.push((*id, format!("Their {} would have been only {} at the birth", role, b - pb)));
                    } else if role == "mother" && b - pb > 55 {
                        v.push((*id, format!("Their mother would have been {} at the birth", b - pb)));
                    } else if role == "father" && b - pb > 85 {
                        v.push((*id, format!("Their father would have been {} at the birth", b - pb)));
                    }
                }
                if let Some(pd) = parse_date(&par.dd).0 {
                    if role == "mother" && b > pd {
                        v.push((*id, "Born after their mother died".to_string()));
                    }
                    if role == "father" && b > pd + 1 {
                        v.push((*id, "Born more than a year after their father died".to_string()));
                    }
                }
            }
        }
        for f in self.fams.values() {
            if let Some(my) = parse_date(&f.md).0 {
                for sp in [f.h, f.w].into_iter().flatten() {
                    if let Some(sb) = parse_date(&self.people[&sp].bd).0 {
                        if my < sb + 12 {
                            v.push((sp, format!("Married in {} when only {}", my, my - sb)));
                        }
                    }
                }
            }
        }
        // Complex-family checks
        for id in self.people.keys() {
            let pf = self.parent_families_ix(&ix, *id);
            let bio_sets = pf.iter().filter(|f| { let fm = &self.fams[*f]; rel_is_bio(&fm.rel_to(*id, true)) && rel_is_bio(&fm.rel_to(*id, false)) && (fm.h.is_some() || fm.w.is_some()) }).count();
            if bio_sets > 1 {
                v.push((*id, "Has more than one set of biological parents (mark the others as adopted, step, foster or guardian)".to_string()));
            }
        }
        // Loops: someone who is their own ancestor. One pass over the whole parent graph.
        let parents_of = |n: u32| -> Vec<u32> {
            let mut out = Vec::new();
            for f in self.parent_families_ix(&ix, n) {
                let fm = &self.fams[&f];
                for p in fm.h.iter().chain(fm.w.iter()) {
                    if !out.contains(p) && self.people.contains_key(p) {
                        out.push(*p);
                    }
                }
            }
            out
        };
        let mut state: HashMap<u32, u8> = HashMap::new(); // 1 = on the current path, 2 = finished
        let mut looped: HashSet<u32> = HashSet::new();
        for start in self.people.keys() {
            if state.contains_key(start) {
                continue;
            }
            let mut stack: Vec<(u32, Vec<u32>, usize)> = vec![(*start, parents_of(*start), 0)];
            state.insert(*start, 1);
            while let Some(top) = stack.last_mut() {
                if top.2 < top.1.len() {
                    let nx = top.1[top.2];
                    top.2 += 1;
                    match state.get(&nx).copied() {
                        Some(1) => {
                            let pos = stack.iter().position(|(n, _, _)| *n == nx).unwrap_or(0);
                            for (n, _, _) in &stack[pos..] {
                                looped.insert(*n);
                            }
                        }
                        Some(_) => {}
                        None => {
                            state.insert(nx, 1);
                            let ps = parents_of(nx);
                            stack.push((nx, ps, 0));
                        }
                    }
                } else {
                    let n = top.0;
                    state.insert(n, 2);
                    stack.pop();
                }
            }
        }
        let mut looped: Vec<u32> = looped.into_iter().collect();
        looped.sort();
        for id in looped {
            v.push((id, "Is recorded as their own ancestor (a loop in the family links)".to_string()));
        }
        let mut seen_pairs: HashMap<(u32, u32), u32> = HashMap::new();
        for f in self.fams.values() {
            if let (Some(a), Some(b)) = (f.h, f.w) {
                let key = (a.min(b), a.max(b));
                let n = seen_pairs.entry(key).or_insert(0);
                *n += 1;
                if *n == 2 {
                    v.push((a, format!("Has two separate family groups with the same partner, {} (merge them if it is one partnership)", self.people[&b].name)));
                }
            }
            for k in &f.kids {
                if Some(*k) == f.h || Some(*k) == f.w {
                    v.push((*k, "Is recorded as their own parent".to_string()));
                }
            }
        }
        v.sort_by(|a, b| self.people[&a.0].name.to_lowercase().cmp(&self.people[&b.0].name.to_lowercase()));
        v
    }

    pub(crate) fn rel_names_from(&self, f: &Family, id: u32, out: &mut HashSet<String>) {
        let mine = f.h == Some(id) || f.w == Some(id);
        let kid = f.kids.contains(&id);
        if mine || kid {
            for x in f.h.iter().chain(f.w.iter()).chain(if mine { f.kids.iter() } else { [].iter() }) {
                if *x != id {
                    if let Some(p) = self.people.get(x) {
                        let g = p.given.to_lowercase().split_whitespace().next().unwrap_or("").to_string();
                        if !g.is_empty() {
                            out.insert(g);
                        }
                    }
                }
            }
        }
    }
    pub(crate) fn rel_names_ix(&self, ix: &Idx, id: u32) -> HashSet<String> {
        let mut out = HashSet::new();
        let mut fs: Vec<u32> = ix.child_of.get(&id).cloned().unwrap_or_default();
        fs.extend(ix.partner_in.get(&id).cloned().unwrap_or_default());
        fs.sort();
        fs.dedup();
        for f in fs {
            if let Some(fm) = self.fams.get(&f) {
                self.rel_names_from(fm, id, &mut out);
            }
        }
        out
    }

    pub(crate) fn duplicate_pairs(&self) -> Vec<(u32, u32, String)> {
        let norm = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
        let ix = self.index();
        let mut cache: HashMap<u32, HashSet<String>> = HashMap::new();
        // People are only compared within a group whose surnames sound alike (so "Smith" and
        // "Smyth", or a typo, still meet). A woman is also grouped under her maiden name.
        let mut buckets: HashMap<String, Vec<u32>> = HashMap::new();
        for (id, p) in &self.people {
            let sn = if p.surname.is_empty() { p.name.split_whitespace().last().unwrap_or("").to_string() } else { p.surname.clone() };
            let mut keys: Vec<String> = vec![sound_key(&sn)];
            if !p.maiden.trim().is_empty() {
                keys.push(sound_key(&p.maiden));
            }
            keys.retain(|k| !k.is_empty());
            keys.dedup();
            if norm(&p.given).is_empty() {
                continue;
            }
            for k in keys {
                buckets.entry(k).or_default().push(*id);
            }
        }
        let mut seen_pairs: HashSet<(u32, u32)> = HashSet::new();
        let mut out: Vec<(i32, u32, u32, String)> = Vec::new();
        for ids in buckets.values() {
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let pair = (ids[i].min(ids[j]), ids[i].max(ids[j]));
                    if !seen_pairs.insert(pair) {
                        continue;
                    }
                    let (a, b) = (&self.people[&ids[i]], &self.people[&ids[j]]);
                    let (ga, gb) = (norm(&a.given), norm(&b.given));
                    let mut score: i32 = 0;
                    let mut why: Vec<&str> = Vec::new();
                    let (fa, fb) = (first_given(&a.given), first_given(&b.given));
                    if !ga.is_empty() && ga == gb {
                        score += 2;
                        why.push("same name");
                    } else if !fa.is_empty() && canon_given(&fa) == canon_given(&fb) {
                        score += 2;
                        why.push(if fa == fb { "same first name" } else { "nickname of the same name" });
                    } else if !ga.is_empty() && !gb.is_empty() && (ga.starts_with(&gb) || gb.starts_with(&ga)) {
                        score += 1;
                        why.push("similar name");
                    } else if fa.len() >= 4 && fb.len() >= 4 && edit_distance(&fa, &fb) <= 1 {
                        score += 1;
                        why.push("names differ by one letter");
                    } else {
                        continue;
                    }
                    let (sa, sb) = (norm(&a.surname), norm(&b.surname));
                    if !sa.is_empty() && !sb.is_empty() && sa != sb {
                        why.push("surnames sound alike");
                    }
                    if let (Some(x), Some(y)) = (parse_date(&a.bd).0, parse_date(&b.bd).0) {
                        let d = (x - y).abs();
                        if d == 0 {
                            score += 2;
                            why.push("same birth year");
                        } else if d <= 2 {
                            score += 1;
                            why.push("birth years close");
                        } else {
                            continue;
                        }
                    }
                    if !a.bp.is_empty() && norm(&a.bp) == norm(&b.bp) {
                        score += 1;
                        why.push("same birthplace");
                    }
                    if let (Some(x), Some(y)) = (parse_date(&a.dd).0, parse_date(&b.dd).0) {
                        if x == y {
                            score += 1;
                            why.push("same death year");
                        }
                    }
                    let exact = !ga.is_empty() && ga == gb;
                    if exact && !a.bp.is_empty() != !b.bp.is_empty() {
                        why.push("one copy has less detail");
                    }
                    for k in [ids[i], ids[j]] {
                        if !cache.contains_key(&k) {
                            cache.insert(k, self.rel_names_ix(&ix, k));
                        }
                    }
                    let (ra, rb) = (&cache[&ids[i]], &cache[&ids[j]]);
                    if !ra.is_disjoint(rb) {
                        score += 2;
                        why.push("same relatives");
                    }
                    // Same full name and nothing contradicts it (checked above): worth a look even when
                    // one copy has little else recorded.
                    if score >= 3 || exact {
                        let (lo, hi) = if ids[i] < ids[j] { (ids[i], ids[j]) } else { (ids[j], ids[i]) };
                        out.push((score, lo, hi, why.join(", ")));
                    }
                }
            }
        }
        out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        out.into_iter().map(|(_, a, b, w)| (a, b, w)).collect()
    }

    pub(crate) fn sanitized(&self) -> Db {
        let mut d = self.clone();
        for p in d.people.values_mut() {
            if p.living {
                let sex = p.sex.clone();
                *p = Person { name: "Living".to_string(), given: "Living".to_string(), sex, living: true, ..Default::default() };
            }
        }
        // Marriages and other family details of a living person are private too.
        let living: HashSet<u32> = d.people.iter().filter(|(_, p)| p.living).map(|(k, _)| *k).collect();
        for f in d.fams.values_mut() {
            let touches = f.h.map_or(false, |x| living.contains(&x)) || f.w.map_or(false, |x| living.contains(&x)) || f.more.iter().any(|(_, x)| living.contains(x));
            if touches {
                *f = Family { h: f.h, w: f.w, kids: f.kids.clone(), more: f.more.clone(), ..Default::default() };
            }
        }
        notes::without_private_notes(d)
    }

    pub(crate) fn ancestors(&self, id: u32) -> HashMap<u32, u32> {
        let mut m: HashMap<u32, u32> = HashMap::new();
        m.insert(id, 0);
        let mut q = std::collections::VecDeque::new();
        q.push_back(id);
        while let Some(n) = q.pop_front() {
            let d = m[&n];
            let (f, mo) = self.parents(n);
            for p in [f, mo].into_iter().flatten() {
                if !m.contains_key(&p) {
                    m.insert(p, d + 1);
                    q.push_back(p);
                }
            }
        }
        m
    }

    /// Ancestors with their distance. With `bio_only`, only biological (or unrecorded) links are followed.
    pub(crate) fn ancestors_by(&self, id: u32, bio_only: bool) -> HashMap<u32, u32> {
        let mut m: HashMap<u32, u32> = HashMap::new();
        m.insert(id, 0);
        let mut q = std::collections::VecDeque::new();
        q.push_back(id);
        while let Some(n) = q.pop_front() {
            let d = m[&n];
            for fid in self.parent_families(n) {
                let f = &self.fams[&fid];
                for (p, father) in [(f.h, true), (f.w, false)] {
                    if let Some(p) = p {
                        if (!bio_only || rel_is_bio(&f.rel_to(n, father))) && !m.contains_key(&p) {
                            m.insert(p, d + 1);
                            q.push_back(p);
                        }
                    }
                }
            }
        }
        m
    }

    pub(crate) fn relationship(&self, a: u32, b: u32) -> String {
        let (pa, pb) = (&self.people[&a], &self.people[&b]);
        let (na, nb) = (if pa.name.is_empty() { "Unnamed" } else { pa.name.as_str() }, if pb.name.is_empty() { "Unnamed" } else { pb.name.as_str() });
        if a == b {
            return format!("{} and {} are the same person.", na, nb);
        }
        let sx = pb.sex.as_str();
        // Partners
        if let Some(f) = self.fams.values().find(|f| (f.h == Some(a) && f.w == Some(b)) || (f.h == Some(b) && f.w == Some(a))) {
            let ended = f.events.iter().any(|e| e.kind == "Divorce" || e.kind == "Annulment");
            let word = match f.kind.as_str() {
                "Unmarried" => "partner".to_string(),
                "Civil Union" => format!("civil {}", pick_sex(sx, "partner", "partner", "partner")),
                _ => pick_sex(sx, "spouse", "husband", "wife").to_string(),
            };
            return format!("{} is {}'s {}{}.", nb, na, if ended { "former " } else { "" }, word);
        }
        // A direct parent or child by adoption, fostering, step or guardianship
        if let Some((_, rel)) = self.parent_links(a).into_iter().find(|(p, _)| *p == b) {
            if !rel_is_bio(&rel) {
                return format!("{} is {}'s {}.", nb, na, legal_parent_word(&rel, sx));
            }
        }
        if let Some((_, rel)) = self.parent_links(b).into_iter().find(|(p, _)| *p == a) {
            if !rel_is_bio(&rel) {
                return format!("{} is {}'s {}.", nb, na, legal_child_word(&rel, sx));
            }
        }
        // Blood relatives (biological links only)
        let (ma, mb) = (self.ancestors_by(a, true), self.ancestors_by(b, true));
        let mut commons: Vec<(u32, u32)> = ma.iter().filter_map(|(k, da)| mb.get(k).map(|db| (*da, *db))).collect();
        if let Some(sum) = commons.iter().map(|(x, y)| x + y).min() {
            commons.retain(|(x, y)| x + y == sum);
            commons.sort();
            let (da, db) = (commons[0].0 as i32, commons[0].1 as i32);
            let half = da == 1 && db == 1 && commons.len() < 2;
            return format!("{} is {}'s {}.", nb, na, kin_term(da, db, sx, half));
        }
        // Step and half relatives
        let kin = self.kin_extras(a);
        if kin.step_parents.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "step-parent", "stepfather", "stepmother"));
        }
        if kin.step_children.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "stepchild", "stepson", "stepdaughter"));
        }
        if kin.step_siblings.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "step-sibling", "stepbrother", "stepsister"));
        }
        if kin.half_siblings.contains(&b) {
            return format!("{} is {}'s {}.", nb, na, pick_sex(sx, "half-sibling", "half-brother", "half-sister"));
        }
        // Relatives through adoption, fostering or guardianship links
        let (la, lb) = (self.ancestors_by(a, false), self.ancestors_by(b, false));
        let lc: Vec<(u32, u32)> = la.iter().filter_map(|(k, da)| lb.get(k).map(|db| (*da, *db))).collect();
        if let Some((da, db)) = lc.iter().copied().min_by_key(|(x, y)| (x + y, *x)) {
            return format!("{} is {}'s {}, counting adoptive, foster or step links (not a blood relationship).", nb, na, kin_term(da as i32, db as i32, sx, false));
        }
        if let Some(r) = pa.assoc.iter().find(|x| x.other == b) {
            return format!("{} is {}'s {} (a recorded tie, not a family relationship).", nb, na, if r.role.is_empty() { "associate" } else { r.role.as_str() });
        }
        if let Some(r) = pb.assoc.iter().find(|x| x.other == a) {
            return format!("{} is the {} of {} (a recorded tie, not a family relationship).", na, if r.role.is_empty() { "associate" } else { r.role.as_str() }, nb);
        }
        format!("No blood relationship found between {} and {} in your tree. (In-laws and other marriage relationships are not included.)", na, nb)
    }

    pub(crate) fn label(&self, id: u32) -> String {
        let p = &self.people[&id];
        let name = display_name(p);
        let l = life(p);
        if l.is_empty() { name } else { format!("{} ({})", name, l) }
    }
    /// People whose ID number (or Old ID from before renumbering) is the number typed.
    /// "#12", "12" and "I12" all work.
    pub(crate) fn find_by_number(&self, q: &str) -> Vec<(u32, bool)> {
        let n = q.trim().trim_start_matches('#').trim_start_matches(['I', 'i']).trim_start_matches('@').trim_end_matches('@').trim();
        if n.is_empty() {
            return vec![];
        }
        let mut out = vec![];
        if let Ok(v) = n.parse::<u32>() {
            if self.people.contains_key(&v) {
                out.push((v, false));
            }
        }
        for (id, p) in &self.people {
            if old_id(p).map_or(false, |o| o.trim_start_matches('I').trim_matches('@') == n || o == n) && !out.iter().any(|x| x.0 == *id) {
                out.push((*id, true));
            }
        }
        out
    }
    /// Families whose two partners match the two names typed, in either order. Each name may
    /// be part of a name ("john", "smi"); an empty box matches anyone.
    pub(crate) fn find_couples(&self, a: &str, b: &str) -> Vec<u32> {
        let words = |s: &str| s.to_lowercase().split_whitespace().map(|w| w.to_string()).collect::<Vec<_>>();
        let (wa, wb) = (words(a), words(b));
        if wa.is_empty() && wb.is_empty() {
            return vec![];
        }
        let hit = |id: Option<u32>, w: &[String]| -> bool {
            if w.is_empty() {
                return true;
            }
            let Some(p) = id.and_then(|i| self.people.get(&i)) else { return false };
            let hay = format!("{} {} {}", p.name, p.maiden, p.nick).to_lowercase();
            w.iter().all(|x| hay.contains(x.as_str()))
        };
        let mut out: Vec<(String, u32)> = self
            .fams
            .iter()
            .filter(|(_, f)| f.h.is_some() || f.w.is_some())
            .filter(|(_, f)| (hit(f.h, &wa) && hit(f.w, &wb)) || (hit(f.w, &wa) && hit(f.h, &wb)))
            .map(|(k, f)| (f.h.or(f.w).and_then(|x| self.people.get(&x)).map(|p| p.name.to_lowercase()).unwrap_or_default(), *k))
            .collect();
        out.sort();
        out.into_iter().map(|(_, k)| k).collect()
    }
    pub(crate) fn sorted(&self, filter: &str) -> Vec<u32> {
        let f = filter.to_lowercase();
        let mut v: Vec<u32> = self
            .people
            .iter()
            .filter(|(_, p)| f.is_empty() || format!("{} {} {} {}", p.name, p.maiden, p.bp, p.dp).to_lowercase().contains(&f))
            .map(|(i, _)| *i)
            .collect();
        v.sort_by_cached_key(|i| {
            let p = &self.people[i];
            let n = p.name.to_lowercase();
            let sn = if p.surname.is_empty() { n.split_whitespace().last().unwrap_or("").to_string() } else { p.surname.to_lowercase() };
            (sn, n)
        });
        v
    }
}

pub(crate) fn add(db: &mut Db, n: &str, s: &str, bd: &str, bp: &str, dd: &str) -> u32 {
    let id = db.alloc();
    db.people.insert(
        id,
        Person { name: n.into(), sex: s.into(), bd: bd.into(), bp: bp.into(), dd: dd.into(), ..Default::default() },
    );
    id
}

pub(crate) fn fam(db: &mut Db, h: u32, w: u32, md: &str, mp: &str, kids: &[u32]) {
    let id = db.alloc();
    db.fams.insert(id, Family { h: Some(h), w: Some(w), md: md.into(), mp: mp.into(), kids: kids.to_vec(), ..Default::default() });
}

pub(crate) fn sample() -> Db {
    let mut db = Db { sample: true, ..Default::default() };
    let a = add(&mut db, "Henry Walker", "M", "12 Mar 1921", "Provo, Utah", "1994");
    let b = add(&mut db, "Ruth Miller", "F", "1924", "Ogden, Utah", "2009");
    let c = add(&mut db, "Robert Walker", "M", "1948", "Salt Lake City, Utah", "");
    let d = add(&mut db, "Anne Walker", "F", "1951", "Salt Lake City, Utah", "");
    let e = add(&mut db, "Linda Hayes", "F", "1950", "Boise, Idaho", "");
    let g = add(&mut db, "Susan Walker", "F", "1975", "Denver, Colorado", "");
    let h = add(&mut db, "George Hayes", "M", "1925", "Twin Falls, Idaho", "2001");
    let i = add(&mut db, "Edith Cole", "F", "1927", "Boise, Idaho", "2015");
    fam(&mut db, a, b, "4 Jun 1946", "Salt Lake City, Utah", &[c, d]);
    fam(&mut db, c, e, "1972", "Denver, Colorado", &[g]);
    fam(&mut db, h, i, "1948", "Boise, Idaho", &[e]);
    fix_names(&mut db);
    db
}

/* ---------- App ---------- */

impl Db {
    /// Every place written anywhere in the tree, with who and what it is attached to.
    pub(crate) fn place_uses(&self) -> BTreeMap<String, Vec<(u32, String, String)>> {
        let mut m: BTreeMap<String, Vec<(u32, String, String)>> = BTreeMap::new();
        let mut add = |pl: &str, id: u32, what: &str, date: &str| {
            let pl = pl.trim();
            if !pl.is_empty() {
                m.entry(pl.to_string()).or_default().push((id, what.to_string(), date.trim().to_string()));
            }
        };
        for (id, p) in &self.people {
            add(&p.bp, *id, "Born", &p.bd);
            add(&p.chp, *id, "Christened", &p.chd);
            add(&p.dp, *id, "Died", &p.dd);
            add(&p.bup, *id, "Buried", &p.bud);
            for e in &p.events {
                add(&e.place, *id, if e.kind.is_empty() { "Event" } else { e.kind.as_str() }, &e.date);
            }
        }
        for f in self.fams.values() {
            for who in [f.h, f.w].into_iter().flatten() {
                add(&f.mp, who, "Married", &f.md);
                for e in &f.events {
                    add(&e.place, who, if e.kind.is_empty() { "Family event" } else { e.kind.as_str() }, &e.date);
                }
            }
        }
        m
    }

    /// Change one spelling of a place to another everywhere. Returns how many were changed.
    pub(crate) fn rename_place(&mut self, from: &str, to: &str) -> usize {
        let (from, to) = (from.trim(), to.trim());
        if from.is_empty() || to.is_empty() || from == to {
            return 0;
        }
        let mut n = 0;
        let mut fix = |s: &mut String| {
            if s.trim() == from {
                *s = to.to_string();
                n += 1;
            }
        };
        for p in self.people.values_mut() {
            fix(&mut p.bp);
            fix(&mut p.chp);
            fix(&mut p.dp);
            fix(&mut p.bup);
            for e in p.events.iter_mut() {
                fix(&mut e.place);
            }
        }
        for f in self.fams.values_mut() {
            fix(&mut f.mp);
            for e in f.events.iter_mut() {
                fix(&mut e.place);
            }
        }
        n
    }
}

/// A place spelling reduced to letters and digits, so "Portland, Oregon" and "portland,oregon."
/// are seen as the same. "USA", "U.S.A." and "United States" all end the same way.
pub(crate) fn place_key(s: &str) -> String {
    let mut t: String = s.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
    t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    for tail in [" united states of america", " united states", " u s a", " usa", " us"] {
        if let Some(x) = t.strip_suffix(tail) {
            t = x.to_string();
            break;
        }
    }
    t.replace(' ', "")
}

impl Db {
    pub fn merge_people(&mut self, keep: u32, gone: u32) {
        let Some(d) = self.people.remove(&gone) else { return };
        if self.home == Some(gone) {
            self.home = Some(keep);
        }
        for g in self.groups.iter_mut() {
            if g.ids.remove(&gone) {
                g.ids.insert(keep);
            }
        }
        if let Some(k) = self.people.get_mut(&keep) {
            let fill = |a: &mut String, b: &String| {
                if a.is_empty() {
                    *a = b.clone();
                }
            };
            fill(&mut k.given, &d.given);
            fill(&mut k.surname, &d.surname);
            fill(&mut k.maiden, &d.maiden);
            fill(&mut k.sex, &d.sex);
            fill(&mut k.bd, &d.bd);
            fill(&mut k.bp, &d.bp);
            fill(&mut k.chd, &d.chd);
            fill(&mut k.chp, &d.chp);
            fill(&mut k.dd, &d.dd);
            fill(&mut k.dp, &d.dp);
            fill(&mut k.bud, &d.bud);
            fill(&mut k.bup, &d.bup);
            fill(&mut k.portrait, &d.portrait);
            if !d.notes.is_empty() && !k.notes.contains(&d.notes) {
                if !k.notes.is_empty() {
                    k.notes.push('\n');
                }
                k.notes.push_str(&d.notes);
            }
            for sr in d.sources {
                if !k.sources.iter().any(|x| x.text == sr.text) {
                    k.sources.push(sr);
                }
            }
            for e in d.events {
                if !k.events.iter().any(|x| x.kind == e.kind && x.date == e.date && x.place == e.place) {
                    k.events.push(e);
                }
            }
            k.media.extend(d.media);
            fill(&mut k.prefix, &d.prefix);
            fill(&mut k.suffix, &d.suffix);
            fill(&mut k.nick, &d.nick);
            fill(&mut k.chtag, &d.chtag);
            for a in d.assoc {
                if !k.assoc.iter().any(|x| x.other == a.other && x.role == a.role) {
                    k.assoc.push(a);
                }
            }
            if k.pref_parents.is_none() {
                k.pref_parents = d.pref_parents;
            }
            if k.fam_order.is_empty() {
                k.fam_order = d.fam_order;
            }
            fill(&mut k.contact.addr, &d.contact.addr);
            fill(&mut k.contact.phone, &d.contact.phone);
            fill(&mut k.contact.email, &d.contact.email);
            fill(&mut k.contact.url, &d.contact.url);
            for a in d.alt_names {
                if !k.alt_names.iter().any(|x| x.kind == a.kind && x.given == a.given && x.surname == a.surname) {
                    k.alt_names.push(a);
                }
            }
            for n in d.note_refs {
                if !k.note_refs.contains(&n) {
                    k.note_refs.push(n);
                }
            }
            for (key, e) in d.fx {
                k.fx.entry(key).or_insert(e);
            }
            for (key, v) in d.fvals {
                k.fvals.entry(key).or_insert(v);
            }
            for r in d.extra {
                if !k.extra.iter().any(|x| x.at == r.at && x.text == r.text) {
                    k.extra.push(r);
                }
            }
            k.name = compose_name(&k.given, &k.surname);
            if k.living || d.living {
                k.living = true;
            }
        }
        for f in self.fams.values_mut() {
            if f.h == Some(gone) {
                f.h = Some(keep);
            }
            if f.w == Some(gone) {
                f.w = Some(keep);
            }
            for kd in f.kids.iter_mut() {
                if *kd == gone {
                    *kd = keep;
                }
            }
            let mut seen: HashSet<u32> = HashSet::new();
            f.kids.retain(|x| seen.insert(*x));
            for (_, x) in f.more.iter_mut() {
                if *x == gone {
                    *x = keep;
                }
            }
            if let Some(v) = f.pedi.remove(&gone) {
                f.pedi.entry(keep).or_insert(v);
            }
            if let Some(v) = f.frel.remove(&gone) {
                f.frel.entry(keep).or_insert(v);
            }
            if let Some(v) = f.mrel.remove(&gone) {
                f.mrel.entry(keep).or_insert(v);
            }
        }
        replace_pointer(self, &format!("I{}", gone), Some(&format!("I{}", keep)));
        let ids: Vec<u32> = self.fams.keys().copied().collect();
        let mut seen: HashMap<(Option<u32>, Option<u32>), u32> = HashMap::new();
        for fid in ids {
            let key = {
                let f = &self.fams[&fid];
                (f.h, f.w)
            };
            if key.0.is_none() && key.1.is_none() {
                continue;
            }
            if let Some(&first) = seen.get(&key) {
                if let Some(f) = self.fams.remove(&fid) {
                    if let Some(t) = self.fams.get_mut(&first) {
                        absorb_family(t, f);
                    }
                    replace_pointer(self, &format!("F{}", fid), Some(&format!("F{}", first)));
                }
            } else {
                seen.insert(key, fid);
            }
        }
    }
}

pub(crate) fn ordinal(n: usize) -> String {
    match n {
        1 => "1st".to_string(),
        2 => "2nd".to_string(),
        3 => "3rd".to_string(),
        _ => format!("{}th", n),
    }
}
