//! Small research tools: a date calculator, a Soundex window and searching genealogy websites
//! for a person (opened in the web browser; nothing is sent until you choose a site).

use super::*;

/// Days since 1 March of year 0, for whole dates (proleptic Gregorian calendar).
pub(crate) fn day_number(y: i32, m: u32, d: u32) -> i64 {
    let (y, m) = if m <= 2 { (y as i64 - 1, m as i64 + 12) } else { (y as i64, m as i64) };
    365 * y + y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400) + (153 * (m - 3) + 2) / 5 + d as i64
}

pub(crate) fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A whole date, or None (with what is missing) when a part is not there or not real.
pub(crate) fn whole_date(s: &str) -> Result<(i32, u32, u32), &'static str> {
    match parse_date(s) {
        (Some(y), Some(m), Some(d)) if d >= 1 && d <= days_in_month(y, m) => Ok((y, m, d)),
        (Some(_), Some(_), Some(_)) => Err("that day is not in that month"),
        (None, _, _) => Err("type a date with a year, e.g. 12 Mar 1921"),
        _ => Err("type the day and month too, e.g. 12 Mar 1921"),
    }
}

/// Years, months and days from one date to a later one: whole months first (a month after
/// 31 January is the last day of February), then the days left over.
pub(crate) fn span(a: (i32, u32, u32), b: (i32, u32, u32)) -> (i32, i32, i32) {
    let mut months = (b.0 - a.0) * 12 + (b.1 as i32 - a.1 as i32);
    if b.2 < a.2 {
        months -= 1;
    }
    let total = (a.1 as i32 - 1) + months;
    let (ay, am) = (a.0 + total.div_euclid(12), (total.rem_euclid(12) + 1) as u32);
    let ad = a.2.min(days_in_month(ay, am));
    let days = day_number(b.0, b.1, b.2) - day_number(ay, am, ad);
    (months.div_euclid(12), months.rem_euclid(12), days as i32)
}

pub(crate) fn weekday(y: i32, m: u32, d: u32) -> &'static str {
    // 1 March 2000 was a Wednesday
    const DAYS: [&str; 7] = ["Wednesday", "Thursday", "Friday", "Saturday", "Sunday", "Monday", "Tuesday"];
    DAYS[(day_number(y, m, d) - day_number(2000, 3, 1)).rem_euclid(7) as usize]
}

fn plural(n: i64, w: &str) -> String {
    format!("{} {}{}", n, w, if n == 1 { "" } else { "s" })
}

/// The years someone could have been born in, given their age on a date: an age of 34 on
/// 1 June 1880 means born between 2 June 1845 and 1 June 1846.
pub(crate) fn born_from_age(age: u32, on: &str) -> Option<String> {
    let (y, m, d) = parse_date(on);
    let y = y?;
    match (m, d) {
        (Some(m), Some(d)) => {
            let latest_y = y - age as i32;
            let (ey, em, ed) = if d >= days_in_month(y - age as i32 - 1, m) {
                // the day after the date, a year earlier
                if m == 12 { (latest_y, 1, 1) } else { (latest_y - 1, m + 1, 1) }
            } else {
                (latest_y - 1, m, d + 1)
            };
            Some(format!("between {} and {}", fmt_date(ey, em, ed), fmt_date(latest_y, m, d)))
        }
        _ => Some(format!("in {} or {}", y - age as i32 - 1, y - age as i32)),
    }
}

fn fmt_date(y: i32, m: u32, d: u32) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} {}", d, M[(m - 1) as usize], y)
}

/// Percent-encode text for a web address.
pub(crate) fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.trim().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => o.push(b as char),
            b' ' => o.push('+'),
            _ => o.push_str(&format!("%{:02X}", b)),
        }
    }
    o
}

/// The genealogy websites a person can be searched on, with the web address for each.
pub(crate) fn search_links(p: &Person) -> Vec<(&'static str, String)> {
    let given = p.given.split_whitespace().next().unwrap_or("").to_string();
    let sur = p.surname.trim().to_string();
    let by = parse_date(if p.bd.trim().is_empty() { &p.chd } else { &p.bd }).0;
    let dy = parse_date(if p.dd.trim().is_empty() { &p.bud } else { &p.dd }).0;
    let place = p.bp.trim().to_string();
    let mut fs = format!("https://www.familysearch.org/search/record/results?q.givenName={}&q.surname={}", enc(&given), enc(&sur));
    if let Some(y) = by {
        fs.push_str(&format!("&q.birthLikeDate.from={}&q.birthLikeDate.to={}", y - 2, y + 2));
    }
    if !place.is_empty() {
        fs.push_str(&format!("&q.birthLikePlace={}", enc(&place)));
    }
    if let Some(y) = dy {
        fs.push_str(&format!("&q.deathLikeDate.from={}&q.deathLikeDate.to={}", y - 2, y + 2));
    }
    let mut fg = format!("https://www.findagrave.com/memorial/search?firstname={}&lastname={}", enc(&given), enc(&sur));
    if let Some(y) = by {
        fg.push_str(&format!("&birthyear={}&birthyearfilter=2", y));
    }
    if let Some(y) = dy {
        fg.push_str(&format!("&deathyear={}&deathyearfilter=2", y));
    }
    let mut an = format!("https://www.ancestry.com/search/?name={}_{}", enc(&given), enc(&sur));
    if let Some(y) = by {
        an.push_str(&format!("&birth={}", y));
    }
    if let Some(y) = dy {
        an.push_str(&format!("&death={}", y));
    }
    let wt = format!("https://www.wikitree.com/index.php?title=Special:SearchPerson&wpFirst={}&wpLast={}", enc(&given), enc(&sur));
    let years: Vec<String> = [by, dy].into_iter().flatten().map(|y| y.to_string()).collect();
    let g = format!("https://www.google.com/search?q={}", enc(&format!("\"{} {}\" {} genealogy", given, sur, years.join(" "))));
    vec![("FamilySearch (free)", fs), ("Find a Grave (free)", fg), ("WikiTree (free)", wt), ("Ancestry", an), ("Google", g)]
}

impl App {
    pub(crate) fn tools_windows(&mut self, ctx: &egui::Context) {
        self.date_calc_window(ctx);
        self.soundex_window(ctx);
    }

    fn date_calc_window(&mut self, ctx: &egui::Context) {
        if !self.x.calc_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Date calculator").open(&mut open).default_width(420.0).resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.x.calc_mode, 0, "Age on a date");
                ui.selectable_value(&mut self.x.calc_mode, 1, "Born from an age");
                ui.selectable_value(&mut self.x.calc_mode, 2, "Between two dates");
            });
            ui.separator();
            let sel_birth = self.sel.and_then(|i| self.db.people.get(&i)).map(|p| (display_name(p), p.bd.clone()));
            match self.x.calc_mode {
                0 => {
                    ui.label("How old was someone on a given day?");
                    egui::Grid::new("calc0").num_columns(2).show(ui, |ui| {
                        ui.label("Born");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.x.calc_a).hint_text("e.g. 3 Feb 1838").desired_width(150.0));
                            if let Some((n, bd)) = &sel_birth {
                                if !bd.is_empty() && ui.small_button("Use the selected person").on_hover_text(format!("{n}, born {bd}")).clicked() {
                                    self.x.calc_a = bd.clone();
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("On");
                        ui.add(egui::TextEdit::singleline(&mut self.x.calc_b).hint_text("e.g. 1 Jun 1880").desired_width(150.0));
                        ui.end_row();
                    });
                    let r = match (whole_date(&self.x.calc_a), whole_date(&self.x.calc_b)) {
                        (Ok(a), Ok(b)) if day_number(b.0, b.1, b.2) >= day_number(a.0, a.1, a.2) => {
                            let (y, m, d) = span(a, b);
                            format!("{}, {} and {} old.", plural(y as i64, "year"), plural(m as i64, "month"), plural(d as i64, "day"))
                        }
                        (Ok(_), Ok(_)) => "The second date is before the birth.".to_string(),
                        _ => match (parse_date(&self.x.calc_a).0, parse_date(&self.x.calc_b).0) {
                            (Some(a), Some(b)) if b >= a => format!("About {} or {} years old (whole dates give an exact answer).", (b - a - 1).max(0), b - a),
                            _ => String::new(),
                        },
                    };
                    ui.add_space(6.0);
                    ui.label(RichText::new(r).strong());
                }
                1 => {
                    ui.label("A census or record gives an age: when could the person have been born?");
                    egui::Grid::new("calc1").num_columns(2).show(ui, |ui| {
                        ui.label("Age");
                        ui.add(egui::TextEdit::singleline(&mut self.x.calc_age).hint_text("e.g. 34").desired_width(60.0));
                        ui.end_row();
                        ui.label("On (date of the record)");
                        ui.add(egui::TextEdit::singleline(&mut self.x.calc_b).hint_text("e.g. 1 Jun 1880, or 1880").desired_width(150.0));
                        ui.end_row();
                    });
                    let r = match self.x.calc_age.trim().parse::<u32>().ok().and_then(|a| born_from_age(a, &self.x.calc_b)) {
                        Some(s) => format!("Born {}.", s),
                        None => String::new(),
                    };
                    ui.add_space(6.0);
                    ui.label(RichText::new(r).strong());
                }
                _ => {
                    ui.label("How long between two dates, and what day of the week each was.");
                    egui::Grid::new("calc2").num_columns(2).show(ui, |ui| {
                        ui.label("From");
                        ui.add(egui::TextEdit::singleline(&mut self.x.calc_a).hint_text("e.g. 14 Jun 1924").desired_width(150.0));
                        ui.end_row();
                        ui.label("To");
                        ui.add(egui::TextEdit::singleline(&mut self.x.calc_b).hint_text("e.g. 11 Dec 1975").desired_width(150.0));
                        ui.end_row();
                    });
                    ui.add_space(6.0);
                    for (l, s) in [("From", &self.x.calc_a), ("To", &self.x.calc_b)] {
                        if let Ok((y, m, d)) = whole_date(s) {
                            ui.label(format!("{}: {} was a {}.", l, fmt_date(y, m, d), weekday(y, m, d)));
                        }
                    }
                    match (whole_date(&self.x.calc_a), whole_date(&self.x.calc_b)) {
                        (Ok(a), Ok(b)) => {
                            let (a, b) = if day_number(b.0, b.1, b.2) < day_number(a.0, a.1, a.2) { (b, a) } else { (a, b) };
                            let days = day_number(b.0, b.1, b.2) - day_number(a.0, a.1, a.2);
                            let (y, m, d) = span(a, b);
                            ui.label(RichText::new(format!("{}, {} and {} ({} in all).", plural(y as i64, "year"), plural(m as i64, "month"), plural(d as i64, "day"), plural(days, "day"))).strong());
                        }
                        (Err(e), _) if !self.x.calc_a.trim().is_empty() => {
                            ui.weak(format!("From: {e}."));
                        }
                        (_, Err(e)) if !self.x.calc_b.trim().is_empty() => {
                            ui.weak(format!("To: {e}."));
                        }
                        _ => {}
                    }
                }
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Dates use today's calendar (Gregorian) throughout; records before a country changed calendars may be 10 to 13 days different.").small().weak());
        });
        self.x.calc_open = open;
    }

    fn soundex_window(&mut self, ctx: &egui::Context) {
        if !self.x.soundex_open {
            return;
        }
        let mut open = true;
        let mut go: Option<u32> = None;
        egui::Window::new("Soundex").open(&mut open).default_width(380.0).show(ctx, |ui| {
            ui.label(RichText::new("Soundex gives surnames that sound alike the same code (Smith, Smyth and Smithe are all S530), as used in US census indexes.").small().weak());
            ui.add(egui::TextEdit::singleline(&mut self.x.soundex_q).hint_text("a surname, e.g. Harwood").desired_width(220.0));
            let code = sound_key(&self.x.soundex_q);
            if code.is_empty() {
                return;
            }
            ui.label(RichText::new(format!("Soundex code: {}", code)).strong().size(16.0));
            let mut same: BTreeMap<String, Vec<u32>> = BTreeMap::new();
            for (id, p) in &self.db.people {
                for s in [&p.surname, &p.maiden] {
                    if !s.trim().is_empty() && sound_key(s) == code {
                        same.entry(s.trim().to_string()).or_default().push(*id);
                    }
                }
            }
            ui.separator();
            if same.is_empty() {
                ui.weak("No surname in this tree has that code.");
                return;
            }
            ui.label(format!("Surnames in this tree with the code {}:", code));
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (s, ids) in &same {
                    egui::CollapsingHeader::new(format!("{} ({})", s, ids.len())).id_salt(("sx", s)).show(ui, |ui| {
                        for id in ids.iter().take(200) {
                            if ui.link(self.db.label(*id)).clicked() {
                                go = Some(*id);
                            }
                        }
                    });
                }
            });
        });
        self.x.soundex_open = open;
        if let Some(id) = go {
            self.goto(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages_and_spans() {
        assert_eq!(span((1838, 2, 3), (1880, 6, 1)), (42, 3, 29));
        assert_eq!(span((1900, 1, 31), (1900, 3, 1)), (0, 1, 1));
        assert_eq!(span((2000, 2, 29), (2001, 2, 28)), (0, 11, 30));
        assert_eq!(weekday(1924, 6, 14), "Saturday");
        assert_eq!(weekday(2026, 11, 11), "Wednesday");
        assert_eq!(day_number(1975, 12, 11) - day_number(1924, 6, 14), 18807);
        assert!(whole_date("30 Feb 1900").is_err());
        assert!(whole_date("29 Feb 2000").is_ok());
        assert!(whole_date("1900").is_err());
    }

    #[test]
    fn born_from_an_age() {
        assert_eq!(born_from_age(34, "1 Jun 1880").unwrap(), "between 2 Jun 1845 and 1 Jun 1846");
        assert_eq!(born_from_age(34, "1880").unwrap(), "in 1845 or 1846");
        assert_eq!(born_from_age(10, "31 Dec 1900").unwrap(), "between 1 Jan 1890 and 31 Dec 1890");
    }

    #[test]
    fn search_addresses() {
        let p = Person { given: "Mary Ann".into(), surname: "O'Neil".into(), bd: "12 Mar 1850".into(), bp: "Cork, Ireland".into(), dd: "1920".into(), ..Default::default() };
        let l = search_links(&p);
        let fs = &l[0].1;
        assert!(fs.contains("q.givenName=Mary&q.surname=O%27Neil"), "{fs}");
        assert!(fs.contains("q.birthLikeDate.from=1848&q.birthLikeDate.to=1852"));
        assert!(fs.contains("q.birthLikePlace=Cork%2C+Ireland"));
        assert!(l.iter().all(|(_, u)| u.starts_with("https://")));
        assert!(l[1].1.contains("birthyear=1850") && l[1].1.contains("deathyear=1920"));
    }
}
