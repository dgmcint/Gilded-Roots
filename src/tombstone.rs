//! Tools > Tombstone calculator: works out when someone was born from the death date and the age
//! carved on a gravestone ("died 14 March 1872, aged 72 y 3 m 14 d"), and can save the answer as
//! an estimated birth date with the gravestone as its source.

use super::*;

/// A calendar day (proleptic Gregorian).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Day {
    pub y: i32,
    pub m: u32,
    pub d: u32,
}

const MON_UP: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
const MON_FULL: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

fn leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if leap(y) {
                29
            } else {
                28
            }
        }
    }
}

impl Day {
    pub fn new(y: i32, m: u32, d: u32) -> Option<Day> {
        if (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m) {
            Some(Day { y, m, d })
        } else {
            None
        }
    }

    /// Days since 1 March of year 0, for adding and taking away days.
    fn number(self) -> i64 {
        let (y, m) = if self.m <= 2 { (self.y as i64 - 1, self.m as i64 + 9) } else { (self.y as i64, self.m as i64 - 3) };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let doy = (153 * m + 2) / 5 + self.d as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe
    }

    fn from_number(n: i64) -> Day {
        let era = n.div_euclid(146097);
        let doe = n.rem_euclid(146097);
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = (yoe + era * 400 + if m <= 2 { 1 } else { 0 }) as i32;
        Day { y, m, d }
    }

    pub fn add_days(self, n: i64) -> Day {
        Day::from_number(self.number() + n)
    }

    /// Go back whole years, then whole months, then days, the way genealogists count an age
    /// back from a death date. A day that does not exist in the month reached (31 April) becomes
    /// the last day of that month.
    pub fn minus(self, years: u32, months: u32, days: u32) -> Day {
        let total = self.y as i64 * 12 + (self.m as i64 - 1) - years as i64 * 12 - months as i64;
        let y = total.div_euclid(12) as i32;
        let m = total.rem_euclid(12) as u32 + 1;
        let d = self.d.min(days_in_month(y, m));
        Day { y, m, d }.add_days(-(days as i64))
    }

    pub fn ged(self) -> String {
        format!("{} {} {}", self.d, MON_UP[self.m as usize - 1], self.y)
    }

    pub fn words(self) -> String {
        format!("{} {} {}", self.d, MON_FULL[self.m as usize - 1], self.y)
    }
}

/// What the stone says about age.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Age {
    pub years: u32,
    pub months: Option<u32>,
    pub days: Option<u32>,
    /// "In his 73rd year": he had passed his 72nd birthday. `years` then holds the 73.
    pub nth_year: bool,
}

impl Age {
    /// Completed years, months and days, after turning "in his 73rd year" into "aged 72".
    fn parts(&self) -> (u32, u32, u32) {
        let y = if self.nth_year { self.years.saturating_sub(1) } else { self.years };
        (y, self.months.unwrap_or(0), self.days.unwrap_or(0))
    }

    /// The age in words, as it would be read off the stone.
    pub fn words(&self) -> String {
        if self.nth_year {
            return format!("in the {} year of life", ordinal(self.years));
        }
        let mut v = Vec::new();
        let unit = |n: u32, one: &str, many: &str| format!("{} {}", n, if n == 1 { one } else { many });
        if self.years > 0 || (self.months.is_none() && self.days.is_none()) {
            v.push(unit(self.years, "year", "years"));
        }
        if let Some(m) = self.months {
            v.push(unit(m, "month", "months"));
        }
        if let Some(d) = self.days {
            v.push(unit(d, "day", "days"));
        }
        format!("aged {}", v.join(", "))
    }

    /// The age in GEDCOM form ("72y 3m 14d").
    pub fn ged(&self) -> String {
        let (y, m, d) = self.parts();
        let mut v = Vec::new();
        if y > 0 || (self.months.is_none() && self.days.is_none()) {
            v.push(format!("{}y", y));
        }
        if self.months.is_some() {
            v.push(format!("{}m", m));
        }
        if self.days.is_some() {
            v.push(format!("{}d", d));
        }
        v.join(" ")
    }
}

pub fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{}{}", n, suffix)
}

/// Read an age written the way stones, records and GEDCOM files write it:
/// "72y 3m 14d", "72 years, 3 months and 14 days", "aged 72", "in his 73rd year", "6 months".
/// Returns None for text with no age in it ("infant", "stillborn", "<1").
pub fn parse_age(s: &str) -> Option<Age> {
    let l = s.to_lowercase().replace(',', " ").replace(" and ", " ");
    if l.contains('<') || l.contains('>') {
        return None;
    }
    let mut age = Age::default();
    let mut found = false;
    if l.contains("year") && (l.contains("st year") || l.contains("nd year") || l.contains("rd year") || l.contains("th year")) {
        // "in his 73rd year"
        let n: String = l.split_whitespace().find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()) && (w.ends_with("st") || w.ends_with("nd") || w.ends_with("rd") || w.ends_with("th"))).unwrap_or("").chars().take_while(|c| c.is_ascii_digit()).collect();
        let n: u32 = n.parse().ok()?;
        if n == 0 {
            return None;
        }
        return Some(Age { years: n, nth_year: true, ..Default::default() });
    }
    // Split "72y" into "72 y" so numbers and units can be read as pairs.
    let mut spaced = String::new();
    let mut prev_digit = false;
    for c in l.chars() {
        let digit = c.is_ascii_digit();
        if prev_digit && c.is_alphabetic() {
            spaced.push(' ');
        }
        spaced.push(c);
        prev_digit = digit;
    }
    let toks: Vec<&str> = spaced.split_whitespace().collect();
    let mut i = 0;
    while i < toks.len() {
        if let Ok(n) = toks[i].parse::<u32>() {
            let unit = toks.get(i + 1).copied().unwrap_or("");
            if unit == "m" || unit.starts_with("mo") {
                age.months = Some(n);
                i += 2;
            } else if unit.starts_with('d') {
                age.days = Some(n);
                i += 2;
            } else if unit.starts_with('w') {
                age.days = Some(age.days.unwrap_or(0) + n * 7);
                i += 2;
            } else if unit.starts_with('y') {
                age.years = n;
                i += 2;
            } else {
                // A bare number is years ("aged 72").
                age.years = n;
                i += 1;
            }
            found = true;
        } else {
            i += 1;
        }
    }
    if found {
        Some(age)
    } else {
        None
    }
}

/// How exactly the death date is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    Day,
    Month,
    Year,
}

/// The death date as the first and last day it could be. "1872" is all of 1872.
pub fn death_span(s: &str) -> Option<(Day, Day, Known, bool)> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let up = t.to_uppercase();
    // "ABT", "BEF", "EST" and similar make the date approximate.
    let rough = up.contains('~') || up.split_whitespace().any(|x| ["ABT", "ABT.", "ABOUT", "EST", "CAL", "BEF", "AFT", "BET", "CIRCA", "C.", "CA."].contains(&x));
    let (y, m, d) = parse_date(t);
    let y = y?;
    match (m, d) {
        (Some(m), Some(d)) => {
            let day = Day::new(y, m, d)?;
            Some((day, day, Known::Day, rough))
        }
        (Some(m), None) => {
            let a = Day::new(y, m, 1)?;
            let b = Day::new(y, m, days_in_month(y, m))?;
            Some((a, b, Known::Month, rough))
        }
        _ => Some((Day { y, m: 1, d: 1 }, Day { y, m: 12, d: 31 }, Known::Year, rough)),
    }
}

/// The answer: the earliest and latest possible birth days.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Born {
    pub first: Day,
    pub last: Day,
    pub known: Known,
}

impl Born {
    pub fn exact(&self) -> bool {
        self.first == self.last
    }

    /// The birth date for the tree, in GEDCOM form: one date, or "BET ... AND ...".
    pub fn ged(&self) -> String {
        if self.exact() {
            return self.first.ged();
        }
        let (a, b) = (self.first, self.last);
        match self.known {
            Known::Year => {
                if a.y == b.y {
                    format!("{}", a.y)
                } else {
                    format!("BET {} AND {}", a.y, b.y)
                }
            }
            Known::Month => {
                if (a.y, a.m) == (b.y, b.m) {
                    format!("{} {}", MON_UP[a.m as usize - 1], a.y)
                } else {
                    format!("BET {} {} AND {} {}", MON_UP[a.m as usize - 1], a.y, MON_UP[b.m as usize - 1], b.y)
                }
            }
            Known::Day => format!("BET {} AND {}", a.ged(), b.ged()),
        }
    }

    pub fn words(&self) -> String {
        if self.exact() {
            return self.first.words();
        }
        let (a, b) = (self.first, self.last);
        match self.known {
            Known::Year if a.y == b.y => format!("sometime in {}", a.y),
            Known::Year => format!("in {} or {}", a.y, b.y),
            _ => format!("between {} and {}", a.words(), b.words()),
        }
    }
}

/// Work out the birth from the death date and the age at death.
///
/// An exact age (years, months and days) counts straight back. An age in whole years means the
/// person had had that many birthdays but not one more, so the birth falls in a window one year
/// wide (one month wide for an age given in years and months).
pub fn birth(death: &str, age: &Age) -> Option<Born> {
    let (d1, d2, known, _) = death_span(death)?;
    let (y, m, d) = age.parts();
    let (first, last) = if age.days.is_some() {
        (d1.minus(y, m, d), d2.minus(y, m, d))
    } else if age.months.is_some() {
        (d1.minus(y, m + 1, 0).add_days(1), d2.minus(y, m, 0))
    } else {
        (d1.minus(y + 1, 0, 0).add_days(1), d2.minus(y, 0, 0))
    };
    let known = match known {
        Known::Day if age.days.is_none() => Known::Day,
        k => k,
    };
    Some(Born { first, last, known })
}

/// Warnings to show beside an answer.
pub fn cautions(death: &str, from_burial: bool) -> Vec<String> {
    let mut v = Vec::new();
    if let Some((d1, _, known, rough)) = death_span(death) {
        if rough {
            v.push("The death date is approximate, so the birth date is too.".to_string());
        }
        if known != Known::Day {
            v.push("The death date has no exact day, so the answer is a range.".to_string());
        }
        if d1.y < 1753 {
            v.push("Before 1752 Britain and its colonies used the old (Julian) calendar, and the year began on 25 March. Dates from then can be off by up to 11 days, and a January-to-March date may belong to the next year. Check the original record.".to_string());
        }
    }
    if from_burial {
        v.push("This uses the burial date, which is usually a few days after death, so the birth date may be a few days late.".to_string());
    }
    v
}

/// The calculator window's state.
#[derive(Default)]
pub struct Tomb {
    pub open: bool,
    pub who: Option<u32>,
    pub death: String,
    pub from_burial: bool,
    pub years: String,
    pub months: String,
    pub days: String,
    pub nth: bool,
    pub source: String,
    pub replace: bool,
    pub done: Option<String>,
}

impl Tomb {
    /// Fill the form from a person's death (or burial) date and any age recorded with it.
    pub fn load(&mut self, db: &Db, id: u32) {
        let Some(p) = db.people.get(&id) else { return };
        self.who = Some(id);
        self.done = None;
        self.replace = false;
        self.from_burial = p.dd.trim().is_empty() && !p.bud.trim().is_empty();
        self.death = if self.from_burial { p.bud.clone() } else { p.dd.clone() };
        let recorded = ["DEAT", "BURI"].iter().filter_map(|t| p.fx.get(*t)).map(|e| e.age.clone()).find(|a| !a.trim().is_empty());
        self.years.clear();
        self.months.clear();
        self.days.clear();
        self.nth = false;
        if let Some(a) = recorded.and_then(|a| parse_age(&a)) {
            self.years = if a.years > 0 || (a.months.is_none() && a.days.is_none()) { a.years.to_string() } else { String::new() };
            self.months = a.months.map(|n| n.to_string()).unwrap_or_default();
            self.days = a.days.map(|n| n.to_string()).unwrap_or_default();
            self.nth = a.nth_year;
        }
        self.source = if p.bup.trim().is_empty() { "Gravestone".to_string() } else { format!("Gravestone, {}", p.bup.trim()) };
    }

    pub fn age(&self) -> Option<Age> {
        let num = |s: &str| -> Result<Option<u32>, ()> {
            let t = s.trim();
            if t.is_empty() {
                Ok(None)
            } else {
                t.parse::<u32>().map(Some).map_err(|_| ())
            }
        };
        let y = num(&self.years).ok()?;
        let (m, d) = if self.nth { (None, None) } else { (num(&self.months).ok()?, num(&self.days).ok()?) };
        if y.is_none() && m.is_none() && d.is_none() {
            return None;
        }
        let a = Age { years: y.unwrap_or(0), months: m, days: d, nth_year: self.nth };
        if a.nth_year && a.years == 0 {
            return None;
        }
        Some(a)
    }
}

/// Put a calculated birth on a person: the date (marked as an estimate), a note saying how it was
/// worked out, the age with the death or burial when none was recorded, and the gravestone as the
/// source. Returns the person's name, or None when the person is not in the tree.
pub fn save_birth(db: &mut Db, id: u32, t: &Tomb, b: &Born, a: &Age) -> Option<String> {
    let p = db.people.get_mut(&id)?;
    let src = if t.source.trim().is_empty() { "Gravestone".to_string() } else { t.source.trim().to_string() };
    let when = if t.from_burial { "buried" } else { "died" };
    let inscription = format!("{} {}, {}", when, t.death.trim(), a.words());
    p.bd = b.ged();
    // Keep the age from the stone with the death (or burial) when none was recorded.
    let tag = if t.from_burial { "BURI" } else { "DEAT" };
    let de = p.fx.entry(tag.to_string()).or_default();
    if de.age.trim().is_empty() {
        de.age = a.ged();
    }
    let e = p.fx.entry("BIRT".to_string()).or_default();
    e.conf = if b.exact() { "probable".to_string() } else { "uncertain".to_string() };
    let line = format!("Calculated from the gravestone: {}.", inscription);
    e.note = if e.note.trim().is_empty() { line } else { format!("{}\n{}", e.note.trim_end(), line) };
    p.sources.push(Source { about: "Birth".to_string(), text: src, page: format!("Inscription: {}", inscription), note: "Birth date worked out from the age at death on the gravestone.".to_string(), ..Default::default() });
    Some(display_name(p))
}

impl App {
    /// Open the calculator for a person (or empty when there is none).
    pub(crate) fn open_tombstone(&mut self, who: Option<u32>) {
        self.x.tomb = Tomb { open: true, ..Default::default() };
        if let Some(id) = who {
            self.x.tomb.load(&self.db, id);
        }
    }

    pub(crate) fn tombstone_window(&mut self, ctx: &egui::Context) {
        if !self.x.tomb.open {
            return;
        }
        let mut open = true;
        let mut save: Option<(u32, Born, Age)> = None;
        let mut load: Option<u32> = None;
        let sel = self.sel;
        egui::Window::new("Tombstone calculator").open(&mut open).resizable(false).default_width(470.0).show(ctx, |ui| {
            let t = &mut self.x.tomb;
            ui.label("Works out a birth date from the death date and the age carved on a gravestone.");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let name = t.who.and_then(|i| self.db.people.get(&i)).map(display_name).unwrap_or_else(|| "Nobody chosen (the answer can't be saved)".to_string());
                ui.label(RichText::new("Person:").strong());
                ui.label(name);
                if let Some(s) = sel {
                    if t.who != Some(s) && ui.small_button("Use the person I'm looking at").clicked() {
                        load = Some(s);
                    }
                }
            });
            ui.separator();
            egui::Grid::new("tomb_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                ui.label(if t.from_burial { "Burial date" } else { "Death date" });
                ui.add(egui::TextEdit::singleline(&mut t.death).hint_text("e.g. 14 Mar 1872").desired_width(200.0));
                ui.end_row();
                ui.label("The stone says");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut t.nth, false, "aged...");
                    ui.radio_value(&mut t.nth, true, "in his or her ...th year");
                });
                ui.end_row();
                if t.nth {
                    ui.label("Year of life");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut t.years).hint_text("e.g. 73").desired_width(64.0));
                        if let Ok(n) = t.years.trim().parse::<u32>() {
                            if n > 0 {
                                ui.label(RichText::new(format!("\"in the {} year\" means aged {}", ordinal(n), n - 1)).weak());
                            }
                        }
                    });
                } else {
                    ui.label("Age");
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut t.years).hint_text("e.g. 72").desired_width(64.0));
                        ui.label("years");
                        ui.add(egui::TextEdit::singleline(&mut t.months).hint_text("e.g. 3").desired_width(56.0));
                        ui.label("months");
                        ui.add(egui::TextEdit::singleline(&mut t.days).hint_text("e.g. 14").desired_width(56.0));
                        ui.label("days");
                    });
                }
                ui.end_row();
            });
            if !t.nth {
                ui.label(RichText::new("Leave months and days empty if the stone gives years only.").small().weak());
            }
            ui.separator();
            let age = t.age();
            let ans = age.as_ref().and_then(|a| birth(&t.death, a));
            match (&age, &ans) {
                (None, _) if !t.years.trim().is_empty() || !t.months.trim().is_empty() || !t.days.trim().is_empty() => {
                    ui.colored_label(egui::Color32::from_rgb(220, 120, 40), "Type the age as whole numbers.");
                }
                (None, _) => {
                    ui.label(RichText::new("Type the age from the stone to see the birth date.").italics().weak());
                }
                (Some(_), None) => {
                    ui.colored_label(egui::Color32::from_rgb(220, 120, 40), "Type a death date with at least a year (for example 14 Mar 1872).");
                }
                (Some(a), Some(b)) => {
                    ui.label(RichText::new(format!("Born {}", b.words())).size(18.0).strong());
                    ui.label(RichText::new(format!("Saved in the tree as: {}", b.ged())).weak());
                    if !b.exact() {
                        ui.label(RichText::new(if a.days.is_none() && a.months.is_none() {
                            "An age in whole years means the person had passed that birthday but not the next one, so the birth falls in a one-year window."
                        } else {
                            "Without the days, the birth falls in a one-month window."
                        }).small());
                    }
                    for c in cautions(&t.death, t.from_burial) {
                        ui.colored_label(egui::Color32::from_rgb(220, 120, 40), c);
                    }
                    if let Some(id) = t.who {
                        if let Some(p) = self.db.people.get(&id) {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Source");
                                ui.add(egui::TextEdit::singleline(&mut t.source).desired_width(330.0));
                            });
                            let has = !p.bd.trim().is_empty();
                            if has {
                                ui.checkbox(&mut t.replace, format!("Replace the birth date already there ({})", p.bd.trim()));
                            }
                            let label = format!("Save as {}'s birth date", display_name(p));
                            if ui.add_enabled(!has || t.replace, egui::Button::new(label)).clicked() {
                                save = Some((id, b.clone(), *a));
                            }
                        }
                    }
                }
            }
            if let Some(m) = &t.done {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(56, 142, 60), m);
            }
        });
        if let Some(id) = load {
            self.x.tomb.load(&self.db, id);
        }
        if let Some((id, b, a)) = save {
            if let Some(name) = save_birth(&mut self.db, id, &self.x.tomb, &b, &a) {
                self.x.next_label = Some(format!("Calculated {}'s birth date from a gravestone", name));
                self.x.tomb.done = Some(format!("Saved. {}'s birth date is now {}.", name, b.ged()));
                self.x.tomb.replace = false;
                self.dirty = true;
            }
        }
        if !open {
            self.x.tomb.open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn age(y: u32, m: Option<u32>, d: Option<u32>) -> Age {
        Age { years: y, months: m, days: d, nth_year: false }
    }

    #[test]
    fn exact_age_counts_back_years_months_days() {
        let b = birth("14 Mar 1872", &age(72, Some(3), Some(14))).unwrap();
        assert!(b.exact());
        assert_eq!(b.ged(), "30 NOV 1799");
    }

    #[test]
    fn day_lands_past_end_of_month() {
        // 31 May minus 3 months is 28 (or 29) February, then the days.
        let b = birth("31 May 1900", &age(10, Some(3), Some(0))).unwrap();
        assert_eq!(b.ged(), "28 FEB 1890");
        let b = birth("31 May 1904", &age(0, Some(3), Some(0))).unwrap();
        assert_eq!(b.ged(), "29 FEB 1904");
    }

    #[test]
    fn days_cross_a_leap_day() {
        let b = birth("2 Mar 1896", &age(0, Some(0), Some(2))).unwrap();
        assert_eq!(b.ged(), "29 FEB 1896");
    }

    #[test]
    fn whole_years_give_a_one_year_window() {
        let b = birth("14 Mar 1872", &age(72, None, None)).unwrap();
        assert_eq!((b.first, b.last), (Day { y: 1799, m: 3, d: 15 }, Day { y: 1800, m: 3, d: 14 }));
        assert_eq!(b.ged(), "BET 15 MAR 1799 AND 14 MAR 1800");
    }

    #[test]
    fn years_and_months_give_a_one_month_window() {
        let b = birth("14 Mar 1872", &age(72, Some(3), None)).unwrap();
        assert_eq!((b.first, b.last), (Day { y: 1799, m: 11, d: 15 }, Day { y: 1799, m: 12, d: 14 }));
    }

    #[test]
    fn nth_year_is_one_less() {
        let a = parse_age("in his 73rd year").unwrap();
        assert!(a.nth_year);
        let b = birth("14 Mar 1872", &a).unwrap();
        assert_eq!(b.ged(), "BET 15 MAR 1799 AND 14 MAR 1800");
    }

    #[test]
    fn year_only_death() {
        let b = birth("1872", &age(72, None, None)).unwrap();
        assert_eq!(b.ged(), "BET 1799 AND 1800");
        assert_eq!(b.words(), "in 1799 or 1800");
    }

    #[test]
    fn month_only_death_with_exact_age() {
        let b = birth("Mar 1872", &age(72, Some(3), Some(14))).unwrap();
        assert_eq!(b.ged(), "BET NOV 1799 AND DEC 1799");
    }

    #[test]
    fn infant_ages() {
        let b = birth("10 Jan 1880", &age(0, None, Some(12))).unwrap();
        assert_eq!(b.ged(), "29 DEC 1879");
        let a = parse_age("6 months").unwrap();
        let b = birth("10 Jan 1880", &a).unwrap();
        assert_eq!(b.ged(), "BET 11 JUN 1879 AND 10 JUL 1879");
    }

    #[test]
    fn reads_common_age_wordings() {
        assert_eq!(parse_age("72y 3m 14d"), Some(age(72, Some(3), Some(14))));
        assert_eq!(parse_age("72 years, 3 months and 14 days"), Some(age(72, Some(3), Some(14))));
        assert_eq!(parse_age("aged 72"), Some(age(72, None, None)));
        assert_eq!(parse_age("Aged 72 yrs 3 mos"), Some(age(72, Some(3), None)));
        assert_eq!(parse_age("2w"), Some(age(0, None, Some(14))));
        assert_eq!(parse_age("<1"), None);
        assert_eq!(parse_age("STILLBORN"), None);
        assert_eq!(parse_age("in her 21st year").map(|a| (a.years, a.nth_year)), Some((21, true)));
    }

    #[test]
    fn old_calendar_and_burial_warnings() {
        assert!(cautions("3 Feb 1740", false).iter().any(|c| c.contains("Julian")));
        assert!(cautions("3 Feb 1840", true).iter().any(|c| c.contains("burial")));
        assert!(cautions("ABT 1840", false).iter().any(|c| c.contains("approximate")));
        assert!(cautions("3 Feb 1840", false).is_empty());
    }

    #[test]
    fn day_number_round_trips() {
        let mut d = Day { y: 1599, m: 12, d: 25 };
        for _ in 0..200_000 {
            let n = d.add_days(1);
            assert_eq!(n.add_days(-1), d);
            d = n;
        }
        assert_eq!(Day { y: 2000, m: 2, d: 28 }.add_days(1), Day { y: 2000, m: 2, d: 29 });
        assert_eq!(Day { y: 1900, m: 2, d: 28 }.add_days(1), Day { y: 1900, m: 3, d: 1 });
    }

    #[test]
    fn saving_sets_birth_note_and_source() {
        let mut db = Db::default();
        let id = add(&mut db, "Thomas Harwood", "M", "", "", "14 Mar 1872");
        db.people.get_mut(&id).unwrap().bup = "Hillside Cemetery".into();
        db.people.get_mut(&id).unwrap().fx.entry("DEAT".into()).or_default().age = "72y 3m 14d".into();
        let mut t = Tomb::default();
        t.load(&db, id);
        assert_eq!((t.years.as_str(), t.months.as_str(), t.days.as_str()), ("72", "3", "14"));
        assert_eq!(t.source, "Gravestone, Hillside Cemetery");
        let a = t.age().unwrap();
        let b = birth(&t.death, &a).unwrap();
        assert_eq!(b.ged(), "30 NOV 1799");
        assert_eq!(save_birth(&mut db, id, &t, &b, &a).as_deref(), Some("Thomas Harwood"));
        let p = &db.people[&id];
        assert_eq!(p.bd, "30 NOV 1799");
        assert_eq!(p.fx["BIRT"].conf, "probable");
        assert!(p.fx["BIRT"].note.contains("died 14 Mar 1872, aged 72 years, 3 months, 14 days"));
        let s = p.sources.last().unwrap();
        assert_eq!((s.about.as_str(), s.text.as_str()), ("Birth", "Gravestone, Hillside Cemetery"));
        // The saved range survives a GEDCOM export.
        let ged = gedcom::to_ged(&db);
        assert!(ged.contains("2 DATE 30 NOV 1799"), "{}", ged);
    }

    #[test]
    fn age_is_kept_with_the_death_when_missing() {
        let mut db = Db::default();
        let id = add(&mut db, "Ann Lee", "F", "", "", "1872");
        let mut t = Tomb::default();
        t.load(&db, id);
        t.years = "72".into();
        let a = t.age().unwrap();
        let b = birth(&t.death, &a).unwrap();
        save_birth(&mut db, id, &t, &b, &a);
        let p = &db.people[&id];
        assert_eq!(p.bd, "BET 1799 AND 1800");
        assert_eq!(p.fx["DEAT"].age, "72y");
        assert_eq!(p.fx["BIRT"].conf, "uncertain");
        let ged = gedcom::to_ged(&db);
        assert!(ged.contains("2 DATE BET 1799 AND 1800"), "{}", ged);
        assert!(ged.contains("2 AGE 72y"), "{}", ged);
    }
}
