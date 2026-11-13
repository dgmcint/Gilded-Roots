//! The Map tab: pins for where people were born, married, died, were buried and lived.
//! Everything is built in and works offline: country and state outlines from Natural Earth
//! (public domain) and about 170,000 town names with their positions from GeoNames (CC BY 4.0).

use super::*;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

static LAND: &[u8] = include_bytes!("../assets/map/land.z");
static STATES: &[u8] = include_bytes!("../assets/map/states.z");
static PLACES: &[u8] = include_bytes!("../assets/map/places.z");

pub struct Geo {
    verts: Vec<[f32; 2]>,
    tris: Vec<u32>,
    rings: Vec<(u32, u32)>,
    states: Vec<Vec<[f32; 2]>>,
    /// normalised country name -> country code
    country_by_name: HashMap<String, String>,
    country_pos: HashMap<String, [f32; 2]>,
    /// country code -> (normalised state name -> state code like "US.OR")
    state_by_name: HashMap<String, HashMap<String, String>>,
    state_pos: HashMap<String, [f32; 2]>,
    /// normalised town name -> (country, state code, lat, lon)
    towns: HashMap<String, Vec<(String, String, f32, f32)>>,
}

static GEO: OnceLock<Geo> = OnceLock::new();

/// The map data, unpacked the first time it is needed (a fraction of a second).
pub fn geo() -> &'static Geo {
    GEO.get_or_init(load_geo)
}

fn rd_u32(b: &[u8], at: &mut usize) -> u32 {
    let v = u32::from_le_bytes([b[*at], b[*at + 1], b[*at + 2], b[*at + 3]]);
    *at += 4;
    v
}
fn rd_i16(b: &[u8], at: &mut usize) -> f32 {
    let v = i16::from_le_bytes([b[*at], b[*at + 1]]);
    *at += 2;
    v as f32 / 100.0
}

/// Lower case, accents and punctuation removed, common short forms spelled one way.
pub fn norm(s: &str) -> String {
    let t: String = s.nfd().filter(|c| c.is_ascii()).collect::<String>().to_lowercase();
    let t: String = t.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' }).collect();
    let words: Vec<String> = t
        .split_whitespace()
        .map(|w| match w {
            "saint" | "ste" | "sainte" => "st".to_string(),
            "mount" => "mt".to_string(),
            "fort" => "ft".to_string(),
            "co" | "cnty" => "county".to_string(),
            "twp" => "township".to_string(),
            _ => w.to_string(),
        })
        .collect();
    words.join(" ")
}

fn load_geo() -> Geo {
    let unz = |b: &[u8]| miniz_oxide::inflate::decompress_to_vec_zlib(b).unwrap_or_default();
    let land = unz(LAND);
    let mut at = 0;
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let mut rings = Vec::new();
    if land.len() > 12 {
        let n = rd_u32(&land, &mut at) as usize;
        verts.reserve(n);
        for _ in 0..n {
            let x = rd_i16(&land, &mut at);
            let y = rd_i16(&land, &mut at);
            verts.push([x, y]);
        }
        let n = rd_u32(&land, &mut at) as usize;
        tris.reserve(n);
        for _ in 0..n {
            tris.push(rd_u32(&land, &mut at));
        }
        let n = rd_u32(&land, &mut at) as usize;
        for _ in 0..n {
            let s = rd_u32(&land, &mut at);
            let l = rd_u32(&land, &mut at);
            rings.push((s, l));
        }
    }
    let st = unz(STATES);
    let mut states = Vec::new();
    let mut at = 0;
    if st.len() > 4 {
        let n = rd_u32(&st, &mut at) as usize;
        for _ in 0..n {
            let l = rd_u32(&st, &mut at) as usize;
            let mut r = Vec::with_capacity(l);
            for _ in 0..l {
                let x = rd_i16(&st, &mut at);
                let y = rd_i16(&st, &mut at);
                r.push([x, y]);
            }
            states.push(r);
        }
    }
    let text = String::from_utf8(unz(PLACES)).unwrap_or_default();
    let mut g = Geo { verts, tris, rings, states, country_by_name: HashMap::new(), country_pos: HashMap::new(), state_by_name: HashMap::new(), state_pos: HashMap::new(), towns: HashMap::new() };
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.first() {
            Some(&"C") if f.len() >= 5 => {
                for n in f[2].split('|') {
                    g.country_by_name.insert(norm(n), f[1].to_string());
                }
                g.country_by_name.insert(norm(f[1]), f[1].to_string());
                g.country_pos.insert(f[1].to_string(), [f[4].parse().unwrap_or(0.0), f[3].parse().unwrap_or(0.0)]);
            }
            Some(&"S") if f.len() >= 5 => {
                let cc = f[1].split('.').next().unwrap_or("").to_string();
                let m = g.state_by_name.entry(cc).or_default();
                for n in f[2].split('|') {
                    m.insert(norm(n), f[1].to_string());
                }
                g.state_pos.insert(f[1].to_string(), [f[4].parse().unwrap_or(0.0), f[3].parse().unwrap_or(0.0)]);
            }
            Some(&"T") if f.len() >= 6 => {
                let lat: f32 = f[4].parse().unwrap_or(0.0);
                let lon: f32 = f[5].parse().unwrap_or(0.0);
                g.towns.entry(norm(f[1])).or_default().push((f[2].to_string(), format!("{}.{}", f[2], f[3]), lat, lon));
            }
            _ => {}
        }
    }
    // names people often use that are not the official ones
    for (a, cc) in [
        ("usa", "US"), ("us", "US"), ("u s a", "US"), ("u s", "US"), ("united states", "US"), ("united states of america", "US"), ("america", "US"),
        ("england", "GB"), ("scotland", "GB"), ("wales", "GB"), ("northern ireland", "GB"), ("great britain", "GB"), ("uk", "GB"), ("britain", "GB"),
        ("holland", "NL"), ("prussia", "DE"), ("bohemia", "CZ"), ("czechia", "CZ"), ("russia", "RU"), ("eire", "IE"), ("canada", "CA"),
    ] {
        g.country_by_name.insert(a.to_string(), cc.to_string());
    }
    g
}

#[derive(Clone, Copy, PartialEq)]
pub enum Prec {
    Town,
    State,
    Country,
    Set,
}

impl Geo {
    /// Where a place is: (lon, lat) and how exact it is.
    pub fn locate(&self, place: &str) -> Option<([f32; 2], Prec)> {
        let parts: Vec<String> = place.split(',').map(norm).filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return None;
        }
        let mut rest = parts.clone();
        let mut cc: Option<String> = None;
        // country: the last part that names one
        for i in (0..rest.len()).rev() {
            if let Some(c) = self.country_by_name.get(&rest[i]) {
                // "Georgia" is a US state far more often than a country in family trees
                if rest[i] == "georgia" && rest.len() > 1 {
                    continue;
                }
                cc = Some(c.clone());
                rest.remove(i);
                break;
            }
        }
        // state or province
        let mut state: Option<String> = None;
        let try_states = |c: &str, rest: &[String]| -> Option<(usize, String)> {
            let m = self.state_by_name.get(c)?;
            for (i, p) in rest.iter().enumerate().rev() {
                if i == 0 && rest.len() > 1 {
                    continue;
                }
                if let Some(s) = m.get(p) {
                    return Some((i, s.clone()));
                }
            }
            None
        };
        match &cc {
            Some(c) => {
                if let Some((i, s)) = try_states(c, &rest) {
                    state = Some(s);
                    if rest.len() > 1 {
                        rest.remove(i);
                    }
                }
            }
            None => {
                // no country written: most trees leave out "USA"
                for c in ["US", "CA", "GB", "AU"] {
                    if let Some((i, s)) = try_states(c, &rest) {
                        cc = Some(c.to_string());
                        state = Some(s);
                        if rest.len() > 1 {
                            rest.remove(i);
                        }
                        break;
                    }
                }
            }
        }
        // town: the first part, else the next one (some places start with a farm or street)
        for cand in rest.iter().take(2) {
            let cand = cand.trim_start_matches("city of ").to_string();
            if let Some(list) = self.towns.get(&cand) {
                let pick = list
                    .iter()
                    .find(|t| Some(&t.0) == cc.as_ref() && state.as_ref().map(|s| *s == t.1).unwrap_or(false))
                    .or_else(|| list.iter().find(|t| Some(&t.0) == cc.as_ref() && state.is_none()))
                    .or_else(|| if cc.is_none() && state.is_none() && list.len() == 1 { list.first() } else { None });
                if let Some(t) = pick {
                    return Some(([t.3, t.2], Prec::Town));
                }
            }
        }
        if let Some(s) = state.and_then(|s| self.state_pos.get(&s).copied()) {
            return Some((s, Prec::State));
        }
        if let Some(c) = cc.and_then(|c| self.country_pos.get(&c).copied()) {
            return Some((c, Prec::Country));
        }
        None
    }
}

/// One thing that happened somewhere.
#[derive(Clone)]
pub struct Pin {
    pub who: u32,
    pub what: u8, // 0 born, 1 married, 2 died, 3 buried, 4 lived / other
    pub label: String,
    pub date: String,
    pub year: Option<i32>,
    pub place: String,
}

/// The map's pins worked out for one choice of people, types and year, kept until any of
/// those (or the tree) changes, so panning and zooming a big family stays smooth.
pub struct MapShown {
    key: (u64, u8, String, Option<u32>, [bool; 5], i32, bool),
    pins: std::sync::Arc<Vec<Pin>>,
    years: (i32, i32),
    names: std::sync::Arc<Vec<String>>,
    placed: Vec<(usize, [f32; 2], Prec)>,
    missing: BTreeMap<String, usize>,
    groups: BTreeMap<(i32, i32), (Vec<usize>, [f32; 2], Prec)>,
}

pub const WHAT: [(&str, [u8; 3]); 5] = [("Born", [70, 170, 90]), ("Married", [160, 100, 200]), ("Died", [200, 70, 70]), ("Buried", [120, 120, 130]), ("Lived / other", [60, 130, 210])];

fn merc(lat: f32) -> f32 {
    let l = lat.clamp(-84.0, 84.0).to_radians();
    (std::f32::consts::FRAC_PI_4 + l / 2.0).tan().ln().to_degrees()
}

/// The map styles offered under "Map style". All are quiet, low-contrast backgrounds, so the
/// coloured pins and DNA regions on top are what stand out.
pub const MAP_STYLES: &[&str] = &["Match the app", "Charcoal", "Warm grey", "Slate blue", "Paper"];

/// The map's flat colours for the chosen style: sea, land, country borders and state borders.
/// "Match the app" builds the colours from the app's background and accent (View > Colours) in
/// the dark theme, and uses Paper in the light one.
pub(crate) fn map_colours(dark: bool) -> (Color32, Color32, Color32, Color32) {
    let c = |r: u8, g: u8, b: u8| Color32::from_rgb(r, g, b);
    let style = MAP_STYLE.load(Ordering::Relaxed);
    if style == 0 && dark {
        // built from the app's own background, with a touch of the accent colour in the sea
        let un = |v: u32| [(v >> 16) as u8 as f32, (v >> 8) as u8 as f32, v as u8 as f32];
        let (bg, ac) = (un(APP_BG.load(Ordering::Relaxed)), un(APP_ACCENT.load(Ordering::Relaxed)));
        let mix = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
        let col = |v: [f32; 3]| Color32::from_rgb(v[0].round() as u8, v[1].round() as u8, v[2].round() as u8);
        let sea = mix(mix(bg, [0.0; 3], 0.18), ac, 0.07);
        let land = mix(bg, [255.0; 3], 0.16);
        return (col(sea), col(land), col(mix(sea, land, 0.35)), col(mix(sea, land, 0.75)));
    }
    let style = if style == 0 { 4 } else { style };
    match style {
        1 => (c(30, 31, 34), c(58, 60, 65), c(40, 42, 46), c(50, 52, 57)),
        2 => (c(36, 34, 32), c(68, 64, 60), c(46, 43, 40), c(58, 55, 51)),
        3 => (c(46, 53, 80), c(83, 91, 117), c(58, 65, 96), c(72, 79, 108)),
        _ => (c(218, 224, 230), c(248, 247, 244), c(198, 203, 211), c(224, 226, 231)),
    }
}

/// A small "Map style" picker; true when the style changed.
pub(crate) fn style_picker(ui: &mut egui::Ui, style: &mut u8) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(("map_style", ui.id())).selected_text(MAP_STYLES.get(*style as usize).copied().unwrap_or(MAP_STYLES[0])).width(120.0).show_ui(ui, |ui| {
        for (i, n) in MAP_STYLES.iter().enumerate() {
            if ui.selectable_label(*style as usize == i, *n).clicked() {
                *style = i as u8;
                changed = true;
            }
        }
    });
    if changed {
        MAP_STYLE.store(*style, Ordering::Relaxed);
    }
    changed
}

/// Draw the world (sea, land, coastlines) into `rect`, centred on `c` (longitude, Mercator latitude)
/// at `k` pixels per degree. Used by the DNA tab's ethnicity map.
pub(crate) fn paint_world(painter: &egui::Painter, rect: Rect, c: [f32; 2], k: f32, dark: bool) {
    let to_scr = |lon: f32, mlat: f32| pos2(rect.center().x + (lon - c[0]) * k, rect.center().y - (mlat - c[1]) * k);
    let (ocean, land, coast, states) = map_colours(dark);
    painter.rect_filled(rect, 0.0_f32, ocean);
    let g = geo();
    for o in world_copies(c[0], rect.width() / 2.0 / k) {
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(g.verts.len());
        for v in &g.verts {
            mesh.vertices.push(egui::epaint::Vertex { pos: to_scr(v[0] + o, merc(v[1])), uv: egui::epaint::WHITE_UV, color: land });
        }
        mesh.indices = g.tris.clone();
        painter.add(egui::Shape::mesh(mesh));
        if k > 6.0 {
            for r in &g.states {
                let v: Vec<egui::Pos2> = r.iter().map(|v| to_scr(v[0] + o, merc(v[1]))).collect();
                if v.iter().any(|p| rect.contains(*p)) {
                    painter.add(egui::Shape::line(v, Stroke::new(0.5_f32, states)));
                }
            }
        }
        for (s, n) in &g.rings {
            let v: Vec<egui::Pos2> = g.verts[*s as usize..(*s + *n) as usize].iter().map(|v| to_scr(v[0] + o, merc(v[1]))).collect();
            if v.iter().any(|p| rect.expand(50.0).contains(*p)) {
                painter.add(egui::Shape::line(v, Stroke::new(0.7_f32, coast)));
            }
        }
    }
}

/// Which copies of the world (0 = the usual one, -1 to the west, 1 to the east...) show in a
/// view centred on longitude `cx` and `half` degrees wide each side, so the map wraps around.
pub(crate) fn world_copies(cx: f32, half: f32) -> Vec<f32> {
    (-2..=2).map(|o| o as f32 * 360.0).filter(|o| cx - half < 180.0 + o && cx + half > -180.0 + o).collect()
}

/// A longitude moved by whole turns so it sits nearest `cx` (for pins on a wrapped map).
pub(crate) fn near_lon(lon: f32, cx: f32) -> f32 {
    lon + 360.0 * ((cx - lon) / 360.0).round()
}

/// Where a longitude and latitude fall on a map drawn by `paint_world`.
pub(crate) fn world_point(rect: Rect, c: [f32; 2], k: f32, lon: f32, lat: f32) -> egui::Pos2 {
    pos2(rect.center().x + (near_lon(lon, c[0]) - c[0]) * k, rect.center().y - (merc(lat) - c[1]) * k)
}

/// Mercator latitude, for centring a map drawn by `paint_world`.
pub(crate) fn merc_lat(lat: f32) -> f32 {
    merc(lat)
}

impl App {
    /// The people the map is showing.
    fn map_people(&self) -> Vec<u32> {
        let mut v = self.map_people_all();
        let sn = self.x.map_surname.trim().to_lowercase();
        if !sn.is_empty() {
            v.retain(|id| self.db.people.get(id).map(|p| p.surname.trim().to_lowercase() == sn || p.maiden.trim().to_lowercase() == sn).unwrap_or(false));
        }
        v
    }

    fn map_people_all(&self) -> Vec<u32> {
        let Some(sel) = self.sel else { return self.db.people.keys().copied().collect() };
        match self.x.map_who {
            0 => vec![sel],
            1 => {
                let mut out = vec![sel];
                let mut seen: HashSet<u32> = HashSet::from([sel]);
                let mut i = 0;
                while i < out.len() && out.len() < 5000 {
                    let (f, m) = self.db.parents(out[i]);
                    for p in [f, m].into_iter().flatten() {
                        if seen.insert(p) {
                            out.push(p);
                        }
                    }
                    i += 1;
                }
                out
            }
            2 => {
                let mut out = vec![sel];
                let mut seen: HashSet<u32> = HashSet::from([sel]);
                let mut i = 0;
                while i < out.len() && out.len() < 5000 {
                    for c in self.db.children_of(out[i]) {
                        if seen.insert(c) {
                            out.push(c);
                        }
                    }
                    i += 1;
                }
                out
            }
            _ => self.db.people.keys().copied().collect(),
        }
    }

    pub(crate) fn map_pins(&self) -> Vec<Pin> {
        let mut pins = Vec::new();
        let people = self.map_people();
        let set: HashSet<u32> = people.iter().copied().collect();
        let mut add = |who: u32, what: u8, label: &str, date: &str, place: &str| {
            if !place.trim().is_empty() {
                pins.push(Pin { who, what, label: label.to_string(), date: date.trim().to_string(), year: parse_date(date).0, place: place.trim().to_string() });
            }
        };
        for id in &people {
            let Some(p) = self.db.people.get(id) else { continue };
            add(*id, 0, "Born", &p.bd, &p.bp);
            add(*id, 0, "Christened", &p.chd, &p.chp);
            add(*id, 2, "Died", &p.dd, &p.dp);
            add(*id, 3, "Buried", &p.bud, &p.bup);
            for e in &p.events {
                add(*id, 4, if e.kind.is_empty() { "Event" } else { e.kind.as_str() }, &e.date, &e.place);
            }
        }
        for f in self.db.fams.values() {
            if let Some(w) = [f.h, f.w].into_iter().flatten().find(|w| set.contains(w)) {
                add(w, 1, "Married", &f.md, &f.mp);
            }
        }
        pins
    }

    /// The pins for the map's current choices, worked out again only when something changes.
    fn map_shown(&mut self) -> std::sync::Arc<MapShown> {
        let key = (self.x.data_ver, self.x.map_who, self.x.map_surname.clone(), self.sel, self.x.map_types, self.x.map_year, self.x.map_undated);
        if let Some(c) = &self.x.map_cache {
            if c.key == key {
                return c.clone();
            }
        }
        // the pins themselves only depend on who is shown; reuse them when only the types or
        // the year changed (the year moves on every step of Play)
        let same_people = self.x.map_cache.as_ref().filter(|c| c.key.0 == key.0 && c.key.1 == key.1 && c.key.2 == key.2 && c.key.3 == key.3);
        let (pins, years, names) = match same_people {
            Some(c) => (c.pins.clone(), c.years, c.names.clone()),
            None => {
                let pins = self.map_pins();
                let years: Vec<i32> = pins.iter().filter_map(|p| p.year).collect();
                let lo = years.iter().min().copied().unwrap_or(1800);
                let hi = years.iter().max().copied().unwrap_or(2000).max(lo + 1);
                let mut names: Vec<String> = self.db.people.values().map(|p| p.surname.trim().to_string()).filter(|s| !s.is_empty()).collect::<HashSet<_>>().into_iter().collect();
                names.sort();
                (std::sync::Arc::new(pins), (lo, hi), std::sync::Arc::new(names))
            }
        };
        let mut placed: Vec<(usize, [f32; 2], Prec)> = Vec::new();
        let mut missing: BTreeMap<String, usize> = BTreeMap::new();
        for (i, p) in pins.iter().enumerate() {
            if !self.x.map_types[p.what as usize] {
                continue;
            }
            match p.year {
                Some(y) if y > self.x.map_year => continue,
                None if !self.x.map_undated => continue,
                _ => {}
            }
            match self.locate_place(&p.place) {
                Some((ll, pr)) => placed.push((i, ll, pr)),
                None => *missing.entry(p.place.clone()).or_default() += 1,
            }
        }
        let mut groups: BTreeMap<(i32, i32), (Vec<usize>, [f32; 2], Prec)> = BTreeMap::new();
        for (i, ll, pr) in &placed {
            let e = groups.entry(key_of(*ll)).or_insert((Vec::new(), *ll, *pr));
            e.0.push(*i);
        }
        let c = std::sync::Arc::new(MapShown { key, pins, years, names, placed, missing, groups });
        self.x.map_cache = Some(c.clone());
        c
    }

    /// Where a place is, using the user's own setting first.
    pub(crate) fn locate_place(&mut self, place: &str) -> Option<([f32; 2], Prec)> {
        if let Some(p) = self.db.place_coords.get(place.trim()) {
            return Some((*p, Prec::Set));
        }
        if let Some(c) = self.x.geo_cache.get(place) {
            return *c;
        }
        let r = geo().locate(place);
        self.x.geo_cache.insert(place.to_string(), r);
        r
    }

    pub(crate) fn map_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        if self.x.map_zoom == 0.0 {
            self.x.map_zoom = 3.0;
            self.x.map_center = [-30.0, 35.0];
            self.x.map_types = [true; 5];
            self.x.map_fit = true;
            // the first time the map opens it shows the whole family's places, not one person's
            self.x.map_who = 3;
        }
        // a fit asked for by a button below happens on the next frame, once the pins match the
        // new choice (these pins were worked out before the buttons were drawn)
        let fit_now = self.x.map_fit;
        let shown = self.map_shown();
        let (y0, y1) = shown.years;
        if self.x.map_year == 0 || self.x.map_year > y1 || self.x.map_year < y0 {
            self.x.map_year = y1;
        }
        let shown = self.map_shown();
        let pins = &shown.pins;
        // controls
        ui.horizontal_wrapped(|ui| {
            ui.label("Show");
            for (k, l) in [(0u8, "This person"), (1, "Ancestors"), (2, "Descendants"), (3, "Everyone")] {
                if ui.selectable_label(self.x.map_who == k, l).clicked() {
                    self.x.map_who = k;
                    self.x.map_fit = true;
                    self.x.map_pick = None;
                }
            }
            ui.separator();
            ui.label("Surname");
            let names: Vec<String> = shown.names.to_vec();
            egui::ComboBox::from_id_salt("map_sn").selected_text(if self.x.map_surname.is_empty() { "All".to_string() } else { self.x.map_surname.clone() }).width(130.0).show_ui(ui, |ui| {
                if ui.selectable_label(self.x.map_surname.is_empty(), "All").clicked() {
                    self.x.map_surname.clear();
                    self.x.map_fit = true;
                }
                for n in names {
                    if ui.selectable_label(self.x.map_surname == n, n.as_str()).clicked() {
                        self.x.map_surname = n.clone();
                        self.x.map_fit = true;
                    }
                }
            });
            ui.separator();
            for (i, (l, c)) in WHAT.iter().enumerate() {
                let col = Color32::from_rgb(c[0], c[1], c[2]);
                let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 5.0, col);
                ui.checkbox(&mut self.x.map_types[i], *l);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Up to the year");
            let r = ui.add(egui::Slider::new(&mut self.x.map_year, y0..=y1).clamping(egui::SliderClamping::Always));
            if r.changed() {
                self.x.map_play = false;
            }
            if ui.button(if self.x.map_play { "Pause" } else { "Play" }).on_hover_text("Watch the family move through the years").clicked() {
                if !self.x.map_play && self.x.map_year >= y1 {
                    self.x.map_year = y0;
                }
                self.x.map_play = !self.x.map_play;
            }
            ui.checkbox(&mut self.x.map_undated, "Include places with no date");
            ui.separator();
            if ui.button("Fit to pins").clicked() {
                self.x.map_fit = true;
            }
            if ui.button("World").clicked() {
                self.x.map_center = [0.0, 20.0];
                self.x.map_zoom = 2.5;
            }
            ui.separator();
            ui.label("Map style");
            if style_picker(ui, &mut self.settings.map_style) {
                save_settings(&self.settings);
            }
        });
        if self.x.map_play {
            let now = ui.input(|i| i.time);
            if now - self.x.map_tick > 0.12 {
                self.x.map_tick = now;
                self.x.map_year += ((y1 - y0) / 150).max(1);
                if self.x.map_year >= y1 {
                    self.x.map_year = y1;
                    self.x.map_play = false;
                }
            }
            ui.ctx().request_repaint();
        }
        let placed = &shown.placed;
        let missing = &shown.missing;
        // side list: what is at the chosen pin, and places not found
        egui::SidePanel::right("map_side").resizable(true).default_width(300.0).show_inside(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if let Some(key) = self.x.map_pick {
                    let here: Vec<(usize, [f32; 2], Prec)> = shown.groups.get(&key).map(|g| g.0.iter().map(|i| (*i, g.1, g.2)).collect()).unwrap_or_default();
                    if let Some(first) = here.first() {
                        ui.label(RichText::new(pins[first.0].place.as_str()).size(16.0).strong());
                        if first.2 == Prec::State || first.2 == Prec::Country {
                            ui.label(RichText::new("Shown in the middle of the state or country: the town was not found.").small().weak());
                        }
                        let mut rows: Vec<&Pin> = here.iter().map(|x| &pins[x.0]).collect();
                        rows.sort_by_key(|p| p.year.unwrap_or(i32::MAX));
                        for p in rows {
                            ui.horizontal_wrapped(|ui| {
                                let c = WHAT[p.what as usize].1;
                                let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 4.0, Color32::from_rgb(c[0], c[1], c[2]));
                                if ui.link(self.db.label(p.who)).clicked() {
                                    *act = Some(Action::Select(p.who));
                                }
                                ui.label(RichText::new(format!("{} {}", p.label, show_date(&p.date))).weak());
                            });
                        }
                        ui.add_space(4.0);
                        if ui.small_button("Move this place on the map...").clicked() {
                            self.x.map_setting = Some(pins[first.0].place.clone());
                        }
                        ui.separator();
                    }
                }
                ui.label(RichText::new(format!("{} pin(s) on the map", placed.len())).strong());
                if !missing.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Not found ({})", missing.len())).strong());
                    ui.label(RichText::new("Pick one, then click where it is on the map.").small().weak());
                    for (pl, n) in missing {
                        let on = self.x.map_setting.as_deref() == Some(pl.as_str());
                        if ui.selectable_label(on, format!("{} ({})", pl, n)).clicked() {
                            self.x.map_setting = if on { None } else { Some(pl.clone()) };
                        }
                    }
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Map data: Natural Earth. Town names: GeoNames (CC BY 4.0).").small().weak());
            });
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
            let rect = resp.rect;
            let dark = ui.visuals().dark_mode;
            if self.x.map_fit && !fit_now {
                ui.ctx().request_repaint();
            }
            if fit_now && self.x.map_fit && !placed.is_empty() {
                self.x.map_fit = false;
                let (mut a, mut b) = ([f32::MAX, f32::MAX], [f32::MIN, f32::MIN]);
                for (_, ll, _) in placed {
                    a = [a[0].min(ll[0]), a[1].min(merc(ll[1]))];
                    b = [b[0].max(ll[0]), b[1].max(merc(ll[1]))];
                }
                // never closer than about a continent (North America or Europe fill the view),
                // so one or two pins don't blow the map up to a blank close-up
                const MIN_W: f32 = 75.0;
                const MIN_H: f32 = 45.0;
                let w = ((b[0] - a[0]) * 1.25).max(MIN_W);
                let h = ((b[1] - a[1]) * 1.25).max(MIN_H);
                self.x.map_zoom = (rect.width() / w).min(rect.height() / h).clamp(1.5, 400.0);
                self.x.map_center = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            }
            // pan and zoom
            if resp.dragged() {
                let d = resp.drag_delta();
                self.x.map_center[0] -= d.x / self.x.map_zoom;
                self.x.map_center[1] += d.y / self.x.map_zoom;
            }
            if resp.hovered() {
                let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                let f = zoom * (scroll / 300.0).exp();
                if (f - 1.0).abs() > 0.001 {
                    if let Some(p) = resp.hover_pos() {
                        let before = self.x.map_center;
                        let k0 = self.x.map_zoom;
                        let k1 = (k0 * f).clamp(1.5, 800.0);
                        let mx = before[0] + (p.x - rect.center().x) / k0;
                        let my = before[1] - (p.y - rect.center().y) / k0;
                        self.x.map_center = [mx - (p.x - rect.center().x) / k1, my + (p.y - rect.center().y) / k1];
                        self.x.map_zoom = k1;
                    }
                }
            }
            // the map wraps around: keep the centre within one turn of the globe
            self.x.map_center[0] = (self.x.map_center[0] + 180.0).rem_euclid(360.0) - 180.0;
            let (cx, cy, k) = (self.x.map_center[0], self.x.map_center[1], self.x.map_zoom);
            let copies = world_copies(cx, rect.width() / 2.0 / k);
            let to_scr = |lon: f32, mlat: f32| pos2(rect.center().x + (lon - cx) * k, rect.center().y - (mlat - cy) * k);
            let (ocean, land, coast, border) = map_colours(dark);
            painter.rect_filled(rect, 0.0_f32, ocean);
            let g = geo();
            // land, as one mesh
            for o in &copies {
                let mut mesh = egui::Mesh::default();
                mesh.vertices.reserve(g.verts.len());
                for v in &g.verts {
                    mesh.vertices.push(egui::epaint::Vertex { pos: to_scr(v[0] + o, merc(v[1])), uv: egui::epaint::WHITE_UV, color: land });
                }
                mesh.indices = g.tris.clone();
                painter.add(egui::Shape::mesh(mesh));
            }
            let clip = rect.expand(50.0);
            let draw_ring = |pts: &mut dyn Iterator<Item = egui::Pos2>, stroke: Stroke| {
                let v: Vec<egui::Pos2> = pts.collect();
                if v.iter().any(|p| clip.contains(*p)) {
                    painter.add(egui::Shape::line(v, stroke));
                }
            };
            for o in &copies {
                if k > 6.0 {
                    for r in &g.states {
                        draw_ring(&mut r.iter().map(|v| to_scr(v[0] + o, merc(v[1]))), Stroke::new(0.5_f32, border));
                    }
                }
                for (s, n) in &g.rings {
                    draw_ring(&mut g.verts[*s as usize..(*s + *n) as usize].iter().map(|v| to_scr(v[0] + o, merc(v[1]))), Stroke::new(0.7_f32, coast));
                }
            }
            // pins, grouped by spot
            let groups = &shown.groups;
            let mut hover_txt: Option<String> = None;
            let mut clicked_group: Option<(i32, i32)> = None;
            let ptr = resp.hover_pos();
            for (key, (idx, ll, pr)) in groups {
                let p = to_scr(near_lon(ll[0], cx), merc(ll[1]));
                if !rect.contains(p) {
                    continue;
                }
                let mut counts = [0usize; 5];
                for i in idx {
                    counts[pins[*i].what as usize] += 1;
                }
                let main = (0..5).max_by_key(|w| counts[*w]).unwrap_or(0);
                let c = WHAT[main].1;
                let col = Color32::from_rgb(c[0], c[1], c[2]);
                let r = 5.0 + (idx.len() as f32).ln() * 2.5;
                let picked = self.x.map_pick == Some(*key);
                if *pr == Prec::State || *pr == Prec::Country {
                    painter.circle_stroke(p, r, Stroke::new(2.5_f32, col));
                } else {
                    painter.circle(p, r, col, Stroke::new(1.5_f32, Color32::WHITE));
                }
                if picked {
                    painter.circle_stroke(p, r + 4.0, Stroke::new(2.0_f32, Color32::WHITE));
                }
                if idx.len() > 1 {
                    painter.text(p, Align2::CENTER_CENTER, idx.len().to_string(), FontId::proportional(10.0), Color32::WHITE);
                }
                if let Some(m) = ptr {
                    if m.distance(p) <= r + 3.0 {
                        hover_txt = Some(format!("{}\n{} event(s). Click to list them.", pins[idx[0]].place, idx.len()));
                        if resp.clicked() {
                            clicked_group = Some(*key);
                        }
                    }
                }
            }
            if let Some(t) = hover_txt {
                resp.clone().on_hover_text(t);
            }
            if let Some(setting) = self.x.map_setting.clone() {
                painter.text(rect.min + vec2(10.0, 10.0), Align2::LEFT_TOP, format!("Click on the map where \"{}\" is (Esc to stop)", setting), FontId::proportional(14.0), if dark { Color32::WHITE } else { Color32::BLACK });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.x.map_setting = None;
                } else if resp.clicked() && clicked_group.is_none() {
                    if let Some(p) = resp.interact_pointer_pos() {
                        let lon = (cx + (p.x - rect.center().x) / k + 180.0).rem_euclid(360.0) - 180.0;
                        let my = cy - (p.y - rect.center().y) / k;
                        // inverse Mercator
                        let lat = (2.0 * (my.to_radians().exp()).atan() - std::f32::consts::FRAC_PI_2).to_degrees();
                        self.db.place_coords.insert(setting.trim().to_string(), [lon, lat]);
                        self.x.next_label = Some(format!("Placed \"{}\" on the map", setting.trim()));
                        self.dirty = true;
                        self.x.map_setting = None;
                        self.status = format!("\"{}\" is now on the map. Edit > Undo takes it back.", setting.trim());
                    }
                }
            } else if let Some(k2) = clicked_group {
                self.x.map_pick = Some(k2);
            } else if resp.clicked() {
                self.x.map_pick = None;
            }
            // zoom buttons
            let zr = Rect::from_min_size(pos2(rect.max.x - 44.0, rect.min.y + 10.0), vec2(34.0, 70.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(zr), |ui| {
                if ui.add_sized([30.0, 28.0], egui::Button::new("+")).clicked() {
                    self.x.map_zoom = (self.x.map_zoom * 1.5).min(800.0);
                }
                if ui.add_sized([30.0, 28.0], egui::Button::new("-")).clicked() {
                    self.x.map_zoom = (self.x.map_zoom / 1.5).max(1.5);
                }
            });
        });
    }
}

#[cfg(test)]
mod wrap_tests {
    use super::*;

    #[test]
    fn the_world_wraps_around() {
        // centred on the Pacific: the usual world and the copy to the east both show
        assert_eq!(world_copies(175.0, 30.0), vec![0.0, 360.0]);
        assert_eq!(world_copies(0.0, 100.0), vec![0.0]);
        assert_eq!(world_copies(0.0, 250.0), vec![-360.0, 0.0, 360.0]);
        // Samoa (-172) seen from New Zealand (174) sits just east, not across the world
        assert!((near_lon(-172.0, 174.0) - 188.0).abs() < 0.01);
        assert!((near_lon(10.0, 0.0) - 10.0).abs() < 0.01);
    }
}

/// Pins closer than about a kilometre share one spot.
fn key_of(ll: [f32; 2]) -> (i32, i32) {
    ((ll[0] * 100.0).round() as i32, (ll[1] * 100.0).round() as i32)
}
