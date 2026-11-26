//! A family website: a folder of plain web pages made from the tree, to put on a USB stick, burn to
//! a disc, send to a relative or upload anywhere. It needs no server and no internet: open
//! index.html in any web browser. Living people are always left out (they show only as "Living",
//! with no page of their own), and so are private notes.
//!
//! The folder holds:
//!   index.html          the front page: title, introduction, search box, surname list
//!   search.js           the list of everyone, for the search box
//!   style.css           the look (light, and dark when the computer is set to dark)
//!   s/<surname>.html    everyone with one surname
//!   p/<id>.html         one page for each person
//!   media/              the photos and documents shown on the pages

use crate::reports::{dp, esc, print_date};
use crate::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// What to put on the website.
#[derive(Clone, Debug)]
pub(crate) struct WebOpts {
    pub title: String,
    pub intro: String,
    pub photos: bool,
    pub notes: bool,
    pub sources: bool,
}

impl Default for WebOpts {
    fn default() -> Self {
        WebOpts { title: String::new(), intro: String::new(), photos: true, notes: true, sources: true }
    }
}

/// What was made.
#[derive(Clone, Debug, Default)]
pub(crate) struct WebInfo {
    pub folder: PathBuf,
    pub people: usize,
    pub left_out: usize,
    pub files: usize,
}

const CSS: &str = r#":root{--stage:#efeeea;--card:#fff;--ink:#1f2328;--soft:#5f6670;--line:#dcd9d3;--accent:#3d5a80;--band-m:#4a7fc1;--band-f:#c8607e}
@media (prefers-color-scheme:dark){:root{--stage:#0e0f11;--card:#26282c;--ink:#eceef0;--soft:#a5abb3;--line:#3a3d42;--accent:#8fb3e8}}
*{box-sizing:border-box}
body{margin:0;background:var(--stage);color:var(--ink);font:16px/1.5 -apple-system,"Segoe UI",Roboto,"Helvetica Neue",Arial,sans-serif}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
header.top{background:var(--card);border-bottom:1px solid var(--line);padding:10px 20px;display:flex;gap:16px;align-items:center;flex-wrap:wrap}
header.top .site{font-weight:700;color:var(--ink)}
main{max-width:980px;margin:0 auto;padding:20px}
.card{background:var(--card);border-radius:12px;box-shadow:0 2px 8px rgba(0,0,0,.08);padding:18px 20px;margin:0 0 18px}
h1{font-size:1.9em;margin:.1em 0 .2em}h2{font-size:1.2em;margin:0 0 .6em}
.life{color:var(--soft);font-size:1.05em}
.hero{display:flex;gap:20px;align-items:center;flex-wrap:wrap}
.portrait{width:132px;height:132px;border-radius:50%;background:var(--line) center/cover no-repeat;flex:none}
.facts{border-collapse:collapse;width:100%}.facts td{padding:6px 8px;border-top:1px solid var(--line);vertical-align:top}
.facts tr:first-child td{border-top:0}.facts td.l{color:var(--soft);width:9em;white-space:nowrap}
.people{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:12px}
.pc{display:flex;gap:10px;align-items:center;background:var(--card);border-radius:10px;box-shadow:0 1px 5px rgba(0,0,0,.08);padding:10px 12px;border-top:4px solid transparent;color:var(--ink)}
.pc.M{border-top-color:var(--band-m)}.pc.F{border-top-color:var(--band-f)}
.pc .mini{width:44px;height:44px;border-radius:50%;background:var(--line) center/cover no-repeat;flex:none}
.pc .n{font-weight:600}.pc .y{color:var(--soft);font-size:.9em}
a.pc:hover{text-decoration:none;outline:2px solid var(--accent)}
.living{color:var(--soft);font-style:italic}
.gallery{display:grid;grid-template-columns:repeat(auto-fill,minmax(160px,1fr));gap:12px}
.gallery figure{margin:0}.gallery img{width:100%;height:150px;object-fit:cover;border-radius:8px;display:block}
.gallery figcaption{font-size:.85em;color:var(--soft);margin-top:4px}
.notes{white-space:pre-wrap}
ol.src{padding-left:1.3em}ol.src li{margin:.3em 0}
.letters a{display:inline-block;min-width:1.8em;text-align:center;padding:2px 4px;border-radius:6px;margin:2px;background:var(--stage)}
.surnames{columns:4 180px}.surnames div{break-inside:avoid;padding:2px 0}.surnames .c{color:var(--soft);font-size:.85em}
input.search{width:100%;font:inherit;padding:10px 12px;border-radius:10px;border:1px solid var(--line);background:var(--stage);color:var(--ink)}
#results{margin-top:12px}
.tree{display:grid;grid-template-columns:1fr 1fr 1fr;gap:10px;align-items:center}
.tree .col{display:flex;flex-direction:column;gap:10px;justify-content:center}
footer{color:var(--soft);font-size:.85em;text-align:center;padding:20px}
@media (max-width:640px){.tree{grid-template-columns:1fr}.facts td.l{width:auto}}
@media print{body{background:#fff}.card,.pc{box-shadow:none;border:1px solid #ccc}header.top input{display:none}}
"#;

const SEARCH_JS: &str = r#"(function(){var box=document.getElementById('q'),out=document.getElementById('results');if(!box||!window.PEOPLE)return;
function norm(s){return s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g,'')}
var all=PEOPLE.map(function(p){return [norm(p[0]+' '+p[1]),p]});
box.addEventListener('input',function(){var q=norm(box.value.trim());out.innerHTML='';if(q.length<2)return;var words=q.split(/\s+/),n=0,frag=document.createElement('div');frag.className='people';
for(var i=0;i<all.length&&n<200;i++){var t=all[i][0],ok=true;for(var w=0;w<words.length;w++){if(t.indexOf(words[w])<0){ok=false;break}}if(!ok)continue;n++;var p=all[i][1],a=document.createElement('a');a.className='pc '+p[3];a.href=p[2];
var d=document.createElement('div');var nm=document.createElement('div');nm.className='n';nm.textContent=p[0];var y=document.createElement('div');y.className='y';y.textContent=p[1];d.appendChild(nm);d.appendChild(y);a.appendChild(d);frag.appendChild(a)}
if(!n){out.textContent=box.getAttribute('data-none')}else{out.appendChild(frag)}});})();
"#;

/// A file name for a surname page: letters and digits only.
fn slug(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            o.extend(c.to_lowercase());
        } else if !o.ends_with('-') && !o.is_empty() {
            o.push('-');
        }
    }
    let o = o.trim_matches('-').to_string();
    // keep names from other alphabets usable as file names everywhere
    let ascii: String = o.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    if ascii.is_empty() || ascii.len() < o.chars().count() / 2 {
        format!("n{:x}", fnv64(s.as_bytes()) & 0xffff_ffff)
    } else {
        ascii
    }
}

fn page(title: &str, site: &str, up: &str, body: &str, made: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{t}</title><link rel=\"stylesheet\" href=\"{up}style.css\"></head><body>\n<header class=\"top\"><a class=\"site\" href=\"{up}index.html\">{s}</a></header>\n<main>\n{body}\n</main>\n<footer>{made}</footer>\n</body></html>\n",
        lang = { let l = crate::i18n::language(); if l.is_empty() { "en".to_string() } else { esc(&l) } },
        t = esc(title),
        s = esc(site),
        up = up,
        body = body,
        made = esc(made)
    )
}

/// The picture for a person's portrait: (file in the media folder, crop).
fn portrait_of(db: &Db, p: &Person) -> Option<(String, [f32; 4])> {
    if let Some(fp) = &p.face_portrait {
        if let Some(o) = db.people.get(&fp.owner) {
            if !o.living && o.media.iter().any(|m| m.file == fp.file) && is_image(&fp.file) {
                return Some((fp.file.clone(), fp.r));
            }
        }
    }
    if !p.portrait.is_empty() && is_image(&p.portrait) && p.media.iter().any(|m| m.file == p.portrait) {
        return Some((p.portrait.clone(), p.portrait_crop.unwrap_or([0.0, 0.0, 1.0, 1.0])));
    }
    p.media.iter().find(|m| is_image(&m.file) && !m.file.is_empty()).map(|m| (m.file.clone(), [0.0, 0.0, 1.0, 1.0]))
}

/// CSS that shows the cropped part of a picture in a round frame.
fn crop_style(file: &str, r: [f32; 4], up: &str) -> String {
    let url = format!("{}media/{}", up, url_part(file));
    let w = (r[2] - r[0]).clamp(0.02, 1.0);
    let h = (r[3] - r[1]).clamp(0.02, 1.0);
    if w >= 0.999 && h >= 0.999 {
        return format!("background-image:url('{}')", url);
    }
    let px = if w < 1.0 { r[0] / (1.0 - w) * 100.0 } else { 0.0 };
    let py = if h < 1.0 { r[1] / (1.0 - h) * 100.0 } else { 0.0 };
    format!("background-image:url('{}');background-size:{:.1}% {:.1}%;background-position:{:.1}% {:.1}%", url, 100.0 / w, 100.0 / h, px, py)
}

/// A file name as part of a link.
fn url_part(f: &str) -> String {
    let mut o = String::new();
    for b in f.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            o.push(b as char);
        } else {
            let _ = write!(o, "%{:02X}", b);
        }
    }
    o
}

struct Site<'a> {
    db: &'a Db,
    opts: &'a WebOpts,
    /// people who get a page
    shown: HashSet<u32>,
    /// media files the pages use
    media: HashSet<String>,
    made: String,
}

impl<'a> Site<'a> {
    fn name(&self, id: u32) -> String {
        self.db.people.get(&id).map(display_name).unwrap_or_default()
    }

    /// A small card linking to a person (or "Living", with no link).
    fn card(&mut self, id: u32, up: &str) -> String {
        let Some(p) = self.db.people.get(&id) else { return String::new() };
        let sex = match p.sex.as_str() {
            "M" => "M",
            "F" => "F",
            _ => "",
        };
        if !self.shown.contains(&id) {
            return format!("<div class=\"pc {}\"><div><div class=\"n living\">{}</div></div></div>", sex, esc(&tr("Living")));
        }
        let mini = match portrait_of(self.db, p).filter(|_| self.opts.photos) {
            Some((f, r)) => {
                self.media.insert(f.clone());
                format!("<div class=\"mini\" style=\"{}\"></div>", crop_style(&f, r, up))
            }
            None => String::new(),
        };
        format!(
            "<a class=\"pc {}\" href=\"{}p/{}.html\">{}<div><div class=\"n\">{}</div><div class=\"y\">{}</div></div></a>",
            sex,
            up,
            id,
            mini,
            esc(&display_name(p)),
            esc(&life(p))
        )
    }

    fn person_page(&mut self, id: u32) -> String {
        let db = self.db;
        let p = &db.people[&id];
        let up = "../";
        let mut b = String::new();
        // the top: portrait, name, years
        b.push_str("<section class=\"card hero\">");
        if self.opts.photos {
            if let Some((f, r)) = portrait_of(db, p) {
                self.media.insert(f.clone());
                let _ = write!(b, "<div class=\"portrait\" style=\"{}\"></div>", crop_style(&f, r, up));
            }
        }
        let _ = write!(b, "<div><h1>{}</h1><div class=\"life\">{}</div>", esc(&display_name(p)), esc(&life(p)));
        if !p.surname.is_empty() {
            let _ = write!(b, "<div><a href=\"../s/{}.html\">{}</a></div>", slug(&p.surname), esc(&trf("Everyone named {}", &[&p.surname])));
        }
        b.push_str("</div></section>");

        // facts
        let mut rows: Vec<(String, String)> = Vec::new();
        for (label, d, pl) in [(tr("Birth"), &p.bd, &p.bp), (tr("Christening"), &p.chd, &p.chp), (tr("Death"), &p.dd, &p.dp), (tr("Burial"), &p.bud, &p.bup)] {
            if !d.is_empty() || !pl.is_empty() {
                rows.push((label.to_string(), dp(d, pl)));
            }
        }
        for e in &p.events {
            let mut v = dp(&e.date, &e.place);
            let detail = [e.val.as_str(), e.typ.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(", ");
            if !detail.is_empty() {
                v = if v.is_empty() { esc(&detail) } else { format!("{} &middot; {}", esc(&detail), v) };
            }
            if !v.is_empty() {
                rows.push((e.kind.clone(), v));
            }
        }
        if !rows.is_empty() {
            let _ = write!(b, "<section class=\"card\"><h2>{}</h2><table class=\"facts\">", esc(&tr("Facts")));
            for (l, v) in rows {
                let _ = write!(b, "<tr><td class=\"l\">{}</td><td>{}</td></tr>", esc(&l), v);
            }
            b.push_str("</table></section>");
        }

        // parents and grandparents
        let (fa, mo) = db.parents(id);
        if fa.is_some() || mo.is_some() {
            let _ = write!(b, "<section class=\"card\"><h2>{}</h2><div class=\"tree\">", esc(&tr("Parents and grandparents")));
            let _ = write!(b, "<div class=\"col\">{}</div>", self.card(id, up));
            let mut col2 = String::new();
            let mut col3 = String::new();
            for par in [fa, mo].into_iter().flatten() {
                col2.push_str(&self.card(par, up));
                let (g1, g2) = db.parents(par);
                for g in [g1, g2].into_iter().flatten() {
                    col3.push_str(&self.card(g, up));
                }
            }
            let _ = write!(b, "<div class=\"col\">{}</div><div class=\"col\">{}</div></div></section>", col2, col3);
        }

        // families
        for fid in db.fams_of(id) {
            let Some(f) = db.fams.get(&fid) else { continue };
            let spouse = if f.h == Some(id) { f.w } else { f.h };
            let title = match spouse {
                Some(s) if self.shown.contains(&s) => trf("Family with {}", &[&self.name(s)]),
                Some(_) => trf("Family with {}", &[&tr("Living")]),
                None => tr("Family").to_string(),
            };
            let _ = write!(b, "<section class=\"card\"><h2>{}</h2>", esc(&title));
            if let Some(s) = spouse {
                let _ = write!(b, "<div class=\"people\">{}</div>", self.card(s, up));
            }
            if !f.md.is_empty() || !f.mp.is_empty() {
                let _ = write!(b, "<p>{}: {}</p>", esc(&tr("Married")), dp(&f.md, &f.mp));
            }
            if !f.kids.is_empty() {
                let _ = write!(b, "<h2>{}</h2><div class=\"people\">", esc(&tr("Children")));
                for k in f.kids.clone() {
                    b.push_str(&self.card(k, up));
                }
                b.push_str("</div>");
            }
            b.push_str("</section>");
        }

        // notes
        if self.opts.notes && !p.notes.trim().is_empty() {
            let _ = write!(b, "<section class=\"card\"><h2>{}</h2><div class=\"notes\">{}</div></section>", esc(&tr("Notes")), esc(p.notes.trim()));
        }

        // photos and documents
        if self.opts.photos {
            let items: Vec<&Media> = p.media.iter().filter(|m| !m.file.is_empty() && is_media_name(&m.file) && !m.file.ends_with(".txt")).collect();
            if !items.is_empty() {
                let _ = write!(b, "<section class=\"card\"><h2>{}</h2><div class=\"gallery\">", esc(&tr("Photos and documents")));
                for m in items {
                    self.media.insert(m.file.clone());
                    let cap = if m.caption.is_empty() { &m.name } else { &m.caption };
                    let href = format!("{}media/{}", up, url_part(&m.file));
                    if is_image(&m.file) {
                        let _ = write!(b, "<figure><a href=\"{h}\"><img src=\"{h}\" alt=\"{c}\" loading=\"lazy\"></a><figcaption>{c}</figcaption></figure>", h = href, c = esc(cap));
                    } else {
                        let _ = write!(b, "<figure><a href=\"{}\">{}</a><figcaption>{}</figcaption></figure>", href, esc(if cap.is_empty() { &m.file } else { cap }), esc(&m.form));
                    }
                }
                b.push_str("</div></section>");
            }
        }

        // sources
        if self.opts.sources {
            let mut cites: Vec<String> = Vec::new();
            for c in p.sources.iter().chain(p.events.iter().flat_map(|e| e.cites.iter())) {
                let t = db.cite_text(c);
                if !t.trim().is_empty() && !cites.contains(&t) {
                    cites.push(t);
                }
            }
            if !cites.is_empty() {
                let _ = write!(b, "<section class=\"card\"><h2>{}</h2><ol class=\"src\">", esc(&tr("Sources")));
                for c in cites {
                    let _ = write!(b, "<li>{}</li>", esc(&c));
                }
                b.push_str("</ol></section>");
            }
        }
        page(&display_name(p), &self.opts.title, up, &b, &self.made)
    }
}

fn tr(s: &'static str) -> String {
    crate::i18n::tr(s).to_string()
}

/// A translated text with {} filled in, in order.
fn trf(s: &'static str, args: &[&str]) -> String {
    let t = crate::i18n::tr(s).to_string();
    let mut out = t.clone();
    for a in args {
        if let Some(i) = out.find("{}") {
            out.replace_range(i..i + 2, a);
        }
    }
    out
}

/// Today as "25 Nov 2026".
fn nice_today() -> String {
    let t = today();
    let parts: Vec<&str> = t.split('-').collect();
    if let [y, m, d] = parts[..] {
        const M: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
        if let (Ok(mi), Ok(di)) = (m.parse::<usize>(), d.parse::<u32>()) {
            if (1..=12).contains(&mi) {
                return print_date(&format!("{} {} {}", di, M[mi - 1], y));
            }
        }
    }
    t
}

/// Make the website in a new folder inside `parent`. Living people and private notes are left out.
pub(crate) fn write_website(db: &Db, parent: &Path, opts: &WebOpts) -> Result<WebInfo, String> {
    let mut opts = opts.clone();
    if opts.title.trim().is_empty() {
        opts.title = tr("Our Family History");
    }
    let clean = db.sanitized();
    // the folder is named after the title, keeping spaces but nothing a file system could object to
    let mut name: String = opts.title.trim().chars().map(|c| if c.is_alphanumeric() || " -_'&,.()".contains(c) { c } else { '-' }).collect();
    name = name.trim_matches(|c: char| c == '.' || c == ' ' || c == '-').chars().take(80).collect();
    if name.is_empty() {
        name = "Family website".into();
    }
    let dir = parent.join(name);
    let io = |e: std::io::Error| io_msg(&e);
    std::fs::create_dir_all(dir.join("p")).map_err(io)?;
    std::fs::create_dir_all(dir.join("s")).map_err(io)?;
    let shown: HashSet<u32> = clean.people.iter().filter(|(_, p)| !p.living).map(|(k, _)| *k).collect();
    let made = trf("Made {} with Gilded Roots. Living people are not shown.", &[&nice_today()]);
    let mut site = Site { db: &clean, opts: &opts, shown: shown.clone(), media: HashSet::new(), made: made.clone() };
    let mut files = 0usize;
    let write = |path: PathBuf, text: &str| -> Result<(), String> { std::fs::write(&path, text).map_err(|e| format!("{}: {}", path.display(), io_msg(&e))) };

    // one page for each person
    let mut ids: Vec<u32> = shown.iter().copied().collect();
    ids.sort();
    for id in &ids {
        let html = site.person_page(*id);
        write(dir.join("p").join(format!("{}.html", id)), &html)?;
        files += 1;
    }

    // surname pages
    let mut by_surname: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for id in &ids {
        let p = &clean.people[id];
        let s = if p.surname.trim().is_empty() { tr("(no surname)") } else { p.surname.trim().to_string() };
        by_surname.entry(s).or_default().push(*id);
    }
    let mut slugs: HashMap<String, String> = HashMap::new();
    for (s, list) in by_surname.iter_mut() {
        list.sort_by_key(|id| {
            let p = &clean.people[id];
            (year(&p.bd).parse::<i32>().unwrap_or(9999), p.given.clone())
        });
        let sl = slug(s);
        slugs.insert(s.clone(), sl.clone());
        let mut b = format!("<section class=\"card\"><h1>{}</h1><div class=\"life\">{}</div></section><div class=\"people\">", esc(s), esc(&trf("{} people", &[&list.len().to_string()])));
        for id in list.clone() {
            b.push_str(&site.card(id, "../"));
        }
        b.push_str("</div>");
        write(dir.join("s").join(format!("{}.html", sl)), &page(s, &opts.title, "../", &b, &made))?;
        files += 1;
    }

    // the front page
    let mut b = String::new();
    let _ = write!(b, "<section class=\"card\"><h1>{}</h1>", esc(&opts.title));
    if !opts.intro.trim().is_empty() {
        let _ = write!(b, "<p class=\"notes\">{}</p>", esc(opts.intro.trim()));
    }
    let _ = write!(b, "<div class=\"life\">{}</div>", esc(&trf("{} people and {} surnames.", &[&ids.len().to_string(), &by_surname.len().to_string()])));
    if let Some(h) = clean.home.filter(|h| shown.contains(h)) {
        let _ = write!(b, "<p>{}</p><div class=\"people\">{}</div>", esc(&tr("Start here:")), site.card(h, ""));
    }
    b.push_str("</section>");
    let _ = write!(
        b,
        "<section class=\"card\"><h2>{}</h2><input class=\"search\" id=\"q\" type=\"search\" placeholder=\"{}\" data-none=\"{}\" autofocus><div id=\"results\"></div></section>",
        esc(&tr("Find a person")),
        esc(&tr("Type a name, for example Mary Smith")),
        esc(&tr("No one by that name."))
    );
    let mut letters: Vec<char> = Vec::new();
    for s in by_surname.keys() {
        let c = s.chars().next().map(|c| c.to_uppercase().next().unwrap_or(c)).unwrap_or('?');
        if !letters.contains(&c) {
            letters.push(c);
        }
    }
    let _ = write!(b, "<section class=\"card\"><h2>{}</h2><div class=\"letters\">", esc(&tr("Surnames")));
    for c in &letters {
        let _ = write!(b, "<a href=\"#l{}\">{}</a>", *c as u32, esc(&c.to_string()));
    }
    b.push_str("</div><div class=\"surnames\">");
    let mut last: Option<char> = None;
    for (s, list) in &by_surname {
        let c = s.chars().next().map(|c| c.to_uppercase().next().unwrap_or(c)).unwrap_or('?');
        let anchor = if last != Some(c) { format!(" id=\"l{}\"", c as u32) } else { String::new() };
        last = Some(c);
        let _ = write!(b, "<div{}><a href=\"s/{}.html\">{}</a> <span class=\"c\">{}</span></div>", anchor, slugs[s], esc(s), list.len());
    }
    b.push_str("</div></section><script src=\"search.js\"></script>");
    let index = page(&opts.title, &opts.title, "", &b, &made).replace("</main>", "</main>");
    write(dir.join("index.html"), &index)?;
    files += 1;

    // everyone, for the search box
    let mut js = String::from("var PEOPLE=[\n");
    for id in &ids {
        let p = &clean.people[id];
        let sex = match p.sex.as_str() {
            "M" => "M",
            "F" => "F",
            _ => "",
        };
        let _ = writeln!(js, "[{},{},{},{}],", serde_json::to_string(&display_name(p)).unwrap_or_default(), serde_json::to_string(&life(p)).unwrap_or_default(), serde_json::to_string(&format!("p/{}.html", id)).unwrap_or_default(), serde_json::to_string(sex).unwrap_or_default());
    }
    js.push_str("];\n");
    js.push_str(SEARCH_JS);
    write(dir.join("search.js"), &js)?;
    write(dir.join("style.css"), CSS)?;
    files += 2;

    // the photos and documents the pages show
    if opts.photos && !site.media.is_empty() {
        std::fs::create_dir_all(dir.join("media")).map_err(io)?;
        let mut list: Vec<&String> = site.media.iter().collect();
        list.sort();
        for f in list {
            let Some(src) = safe_media_path(f) else { continue };
            let dst = dir.join("media").join(f);
            if dst.exists() && std::fs::metadata(&dst).map(|m| m.len()).ok() == std::fs::metadata(&src).map(|m| m.len()).ok() {
                files += 1;
                continue;
            }
            if std::fs::copy(&src, &dst).is_ok() {
                files += 1;
            }
        }
    }
    Ok(WebInfo { folder: dir, people: ids.len(), left_out: clean.people.len() - ids.len(), files })
}

impl App {
    /// File > Make a family website.
    pub(crate) fn web_open(&mut self) {
        if self.x.web.is_none() {
            let title = self.db.home.and_then(|h| self.db.people.get(&h)).filter(|p| !p.surname.is_empty()).map(|p| trf("The {} Family", &[&p.surname])).unwrap_or_default();
            self.x.web = Some(WebOpts { title, ..Default::default() });
        }
    }

    /// The family website window, and the job that makes it.
    pub(crate) fn web_window(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.x.web_job {
            match rx.try_recv() {
                Ok(Ok(info)) => {
                    self.x.web_job = None;
                    self.status = trf("The family website is ready: {} people ({} living people left out), {} files, in {}. Open index.html in it with any web browser.", &[&info.people.to_string(), &info.left_out.to_string(), &info.files.to_string(), &info.folder.display().to_string()]);
                    open_path(&info.folder.join("index.html"));
                }
                Ok(Err(e)) => {
                    self.x.web_job = None;
                    self.status = trf("The website could not be made: {}", &[&e]);
                    self.notice = Some(self.status.clone());
                }
                Err(_) => ctx.request_repaint_after(std::time::Duration::from_millis(300)),
            }
        }
        let Some(mut o) = self.x.web.take() else { return };
        let mut keep = true;
        let mut go = false;
        egui::Window::new(crate::look::wtitle(crate::i18n::tr("Make a family website"))).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).open(&mut keep).show(ctx, |ui| {
            ui.set_max_width(480.0);
            ui.label(crate::i18n::tr("Makes a folder of web pages: a front page with a search box and the surnames, and a page for every person with their facts, family, photos and sources. It needs no internet: open index.html in any web browser, from your computer, a USB stick or a disc."));
            ui.add_space(6.0);
            egui::Grid::new("web_opts").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label(crate::i18n::tr("Title:"));
                ui.add(egui::TextEdit::singleline(&mut o.title).hint_text(crate::i18n::tr("Our Family History")).desired_width(300.0));
                ui.end_row();
                ui.label(crate::i18n::tr("Welcome text:"));
                ui.add(egui::TextEdit::multiline(&mut o.intro).hint_text(crate::i18n::tr("A few words for the front page (optional)")).desired_rows(3).desired_width(300.0));
                ui.end_row();
            });
            ui.checkbox(&mut o.photos, crate::i18n::tr("Photos and documents"));
            ui.checkbox(&mut o.notes, crate::i18n::tr("Notes (private notes are always left out)"));
            ui.checkbox(&mut o.sources, crate::i18n::tr("Sources"));
            ui.add_space(4.0);
            let living = self.db.people.values().filter(|p| p.living).count();
            ui.label(RichText::new(trf("Living people are always left out: {} people will show only as \"Living\".", &[&living.to_string()])).weak());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let busy = self.x.web_job.is_some();
                if ui.add_enabled(!busy && !self.db.people.is_empty(), egui::Button::new(RichText::new(crate::i18n::tr("Choose a folder and make it...")).strong())).clicked() {
                    go = true;
                }
                if busy {
                    ui.spinner();
                    ui.label(crate::i18n::tr("Making the website..."));
                }
            });
        });
        if go {
            if let Some(parent) = rfd::FileDialog::new().set_title(crate::i18n::tr("Where to put the website folder")).set_directory(start_dir()).pick_folder() {
                remember_dir(&parent);
                let db = self.db.clone();
                let opts = o.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let _ = tx.send(write_website(&db, &parent, &opts));
                });
                self.x.web_job = Some(rx);
                self.status = crate::i18n::tr("Making the website...").into();
                keep = false;
            }
        }
        if keep {
            self.x.web = Some(o);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Makes a website from the tree at $GR_WEB_TREE into $GR_WEB_OUT (HOME must hold its media), to look at.
    #[test]
    #[ignore]
    fn write_sample_website() {
        let tree = std::env::var("GR_WEB_TREE").expect("GR_WEB_TREE");
        let out = std::env::var("GR_WEB_OUT").expect("GR_WEB_OUT");
        let db: Db = serde_json::from_slice(&std::fs::read(tree).unwrap()).unwrap();
        let info = write_website(&db, std::path::Path::new(&out), &WebOpts { title: "The Harwood Family".into(), intro: "Our family, from the farms of Kent to Oregon.".into(), ..Default::default() }).unwrap();
        println!("{:?}", info);
    }

    /// Every link on every page goes to a file that is there, and living people appear nowhere.
    #[test]
    fn website_has_working_links_and_no_living_people() {
        // the sample takes the home lock itself, so it comes first
        let mut db = crate::sample::sample_family();
        let _home = crate::home_lock();
        let home = std::env::temp_dir().join(format!("gr-web-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("HOME", &home);
        std::env::set_var("APPDATA", &home);
        // the sample's photos are in the media folder
        for p in db.people.values() {
            for m in &p.media {
                if !m.file.is_empty() {
                    let _ = std::fs::write(mfile(&m.file), b"x");
                }
            }
        }
        // a living grandchild with a secret note and a photo
        let kid = db.alloc();
        let some_fam = *db.fams.keys().next().unwrap();
        db.people.insert(kid, Person { name: "Zebedee Quintrell".into(), given: "Zebedee".into(), surname: "Quintrell".into(), living: true, bd: "2011".into(), notes: "secret note".into(), media: vec![Media { file: "999-zeb.jpg".into(), ..Default::default() }], ..Default::default() });
        std::fs::write(mfile("999-zeb.jpg"), b"zeb").unwrap();
        db.fams.get_mut(&some_fam).unwrap().kids.push(kid);
        let out = home.join("site");
        let info = write_website(&db, &out, &WebOpts { title: "The Test Family".into(), intro: "Hello <b>cousins</b>".into(), ..Default::default() }).unwrap();
        let living = db.people.values().filter(|p| p.living).count();
        assert_eq!(info.people, db.people.len() - living);
        assert_eq!(info.left_out, living);
        let dir = info.folder.clone();
        assert!(dir.ends_with("The Test Family"));
        let mut pages = 0;
        let mut all = String::new();
        for sub in ["", "p", "s"] {
            for e in std::fs::read_dir(dir.join(sub)).unwrap().flatten() {
                let path = e.path();
                if path.extension().map_or(false, |x| x == "html") {
                    pages += 1;
                    let html = std::fs::read_to_string(&path).unwrap();
                    assert!(html.starts_with("<!DOCTYPE html>"));
                    // every relative link and picture is there
                    for attr in ["href=\"", "src=\"", "url('"] {
                        for part in html.split(attr).skip(1) {
                            let end = part.find(|c| c == '"' || c == '\'').unwrap();
                            let link = &part[..end];
                            if link.starts_with('#') || link.starts_with("http") {
                                continue;
                            }
                            let mut decoded = String::new();
                            let b = link.as_bytes();
                            let mut i = 0;
                            while i < b.len() {
                                if b[i] == b'%' {
                                    decoded.push(u8::from_str_radix(&link[i + 1..i + 3], 16).unwrap() as char);
                                    i += 3;
                                } else {
                                    decoded.push(b[i] as char);
                                    i += 1;
                                }
                            }
                            let target = path.parent().unwrap().join(&decoded);
                            assert!(target.exists(), "{} links to {} which is missing", path.display(), link);
                        }
                    }
                    all.push_str(&html);
                }
            }
        }
        assert_eq!(pages, 1 + info.people + std::fs::read_dir(dir.join("s")).unwrap().count());
        let js = std::fs::read_to_string(dir.join("search.js")).unwrap();
        all.push_str(&js);
        assert!(!all.contains("Zebedee") && !all.contains("Quintrell") && !all.contains("secret note") && !all.contains("2011"), "living people are left out");
        assert!(!dir.join("media").join("999-zeb.jpg").exists(), "a living person's photo is not copied");
        assert!(all.contains("Hello &lt;b&gt;cousins&lt;/b&gt;"), "text is shown as text");
        assert!(dir.join("style.css").is_file());
        assert!(js.matches("p/").count() == info.people);
        // made again into the same folder: the same result
        let again = write_website(&db, &out, &WebOpts { title: "The Test Family".into(), ..Default::default() }).unwrap();
        assert_eq!(again.people, info.people);
        let _ = std::fs::remove_dir_all(&home);
    }
}
