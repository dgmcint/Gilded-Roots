//! The sample family: the made-up Harwood family over six generations, used for practice
//! (Help > Explore the sample family). Every person, record and picture here is fictional.

use super::*;

static FILES: &[(&str, &[u8])] = &[
    ("sample-thomas.jpg", include_bytes!("../assets/sample/sample-thomas.jpg")),
    ("sample-margaret.jpg", include_bytes!("../assets/sample/sample-margaret.jpg")),
    ("sample-walter.jpg", include_bytes!("../assets/sample/sample-walter.jpg")),
    ("sample-emily.jpg", include_bytes!("../assets/sample/sample-emily.jpg")),
    ("sample-wedding.jpg", include_bytes!("../assets/sample/sample-wedding.jpg")),
    ("sample-farm.jpg", include_bytes!("../assets/sample/sample-farm.jpg")),
    ("sample-marriage.pdf", include_bytes!("../assets/sample/sample-marriage.pdf")),
    ("sample-census.pdf", include_bytes!("../assets/sample/sample-census.pdf")),
];

const STORY: &str = "The farm near Twin Falls

Grandpa Thomas used to say the farm was built on stubbornness and sugar beets. He and Grandma Margaret \
moved there in 1925, the year after they married, with one cow, a borrowed plough and a house that let \
the wind in through every wall.

The first winter the well froze and Thomas carried water from the canal in milk cans. Margaret kept a \
ledger of every egg sold; it is still in the family, with her small neat handwriting and a pressed \
wildflower between the pages for June 1926, the month Walter was born.

(A made-up story for the Gilded Roots sample family.)";

/// Put the sample pictures and documents in the media folder (once) and make their previews.
fn place_files() {
    for (name, bytes) in FILES {
        if let Some(p) = safe_media_path(name) {
            if !p.exists() {
                let _ = atomic_write(&p, bytes);
            }
        }
    }
    if let Some(p) = safe_media_path("sample-story-farm.txt") {
        if !p.exists() {
            let _ = atomic_write(&p, STORY.as_bytes());
        }
    }
}

fn media(file: &str, caption: &str) -> Media {
    let mut m = Media { file: file.into(), name: file.into(), caption: caption.into(), ..Default::default() };
    if is_image(file) {
        if let Some((t, mid)) = safe_media_path(file).and_then(|p| make_previews(&p)) {
            m.thumb = t;
            m.mid = mid;
        }
    }
    m
}

pub fn sample_family() -> Db {
    place_files();
    let mut db = Db { sample: true, ..Default::default() };
    let p = |db: &mut Db, name: &str, sex: &str, bd: &str, bp: &str, dd: &str, dp: &str| -> u32 {
        let id = db.alloc();
        let (given, surname) = split_name(name);
        db.people.insert(id, Person { name: name.into(), given, surname, sex: sex.into(), bd: bd.into(), bp: bp.into(), dd: dd.into(), dp: dp.into(), living: dd.is_empty() && parse_date(bd).0.map(|y| y > 1925).unwrap_or(false), ..Default::default() });
        id
    };
    // generation 1 (oldest)
    let james = p(&mut db, "James Harwood", "M", "3 Feb 1838", "Leeds, Yorkshire, England", "17 Nov 1901", "Salt Lake City, Salt Lake, Utah, USA");
    let maryann = p(&mut db, "Mary Ann Pickering", "F", "1842", "Leeds, Yorkshire, England", "9 Jan 1909", "Salt Lake City, Salt Lake, Utah, USA");
    let niels = p(&mut db, "Niels Jensen", "M", "1840", "Aalborg, Denmark", "1902", "Twin Falls, Twin Falls, Idaho, USA");
    let karen = p(&mut db, "Karen Sorensen", "F", "12 May 1844", "Aalborg, Denmark", "1910", "Twin Falls, Twin Falls, Idaho, USA");
    let samuel = p(&mut db, "Samuel Ellis", "M", "1870", "Columbus, Ohio, USA", "1945", "Boise, Ada, Idaho, USA");
    let ida = p(&mut db, "Ida Mae Carter", "F", "22 Aug 1874", "Springfield, Missouri, USA", "1950", "Boise, Ada, Idaho, USA");
    // generation 2
    let william = p(&mut db, "William Harwood", "M", "14 Oct 1866", "Leeds, Yorkshire, England", "2 Mar 1931", "Twin Falls, Twin Falls, Idaho, USA");
    let clara = p(&mut db, "Clara Jensen", "F", "1870", "Aalborg, Denmark", "1940", "Twin Falls, Twin Falls, Idaho, USA");
    // generation 3
    let thomas = p(&mut db, "Thomas James Harwood", "M", "4 Jul 1898", "Twin Falls, Twin Falls, Idaho, USA", "11 Dec 1975", "Eugene, Lane, Oregon, USA");
    let claraj = p(&mut db, "Clara Harwood", "F", "1900", "Twin Falls, Twin Falls, Idaho, USA", "1980", "Boise, Ada, Idaho, USA");
    let margaret = p(&mut db, "Margaret Anne Ellis", "F", "19 Mar 1902", "Boise, Ada, Idaho, USA", "6 Apr 1988", "Eugene, Lane, Oregon, USA");
    // generation 4
    let walter = p(&mut db, "Walter Thomas Harwood", "M", "21 Jun 1926", "Twin Falls, Twin Falls, Idaho, USA", "30 Sep 2004", "Eugene, Lane, Oregon, USA");
    let ruth = p(&mut db, "Ruth Ellen Harwood", "F", "1929", "Twin Falls, Twin Falls, Idaho, USA", "2018", "Portland, Multnomah, Oregon, USA");
    let helen = p(&mut db, "Helen Louise Brooks", "F", "8 Jan 1929", "Portland, Multnomah, Oregon, USA", "2015", "Eugene, Lane, Oregon, USA");
    let george = p(&mut db, "George Patterson", "M", "1927", "Seattle, King, Washington, USA", "2010", "Portland, Multnomah, Oregon, USA");
    // generation 5
    let robert = p(&mut db, "Robert Walter Harwood", "M", "15 May 1952", "Eugene, Lane, Oregon, USA", "", "");
    let susan = p(&mut db, "Susan Harwood", "F", "1955", "Eugene, Lane, Oregon, USA", "", "");
    let linda = p(&mut db, "Linda Morales", "F", "1955", "Sacramento, California, USA", "", "");
    // generation 6
    let emily = p(&mut db, "Emily Rose Harwood", "F", "3 Sep 1998", "Eugene, Lane, Oregon, USA", "", "");
    let daniel = p(&mut db, "Daniel Harwood", "M", "2001", "Springfield, Lane, Oregon, USA", "", "");

    let f = |db: &mut Db, h: u32, w: u32, md: &str, mp: &str, kids: &[u32]| -> u32 {
        let id = db.alloc();
        db.fams.insert(id, Family { h: Some(h), w: Some(w), md: md.into(), mp: mp.into(), kids: kids.to_vec(), kind: "Married".into(), ..Default::default() });
        id
    };
    f(&mut db, james, maryann, "1864", "Leeds, Yorkshire, England", &[william]);
    f(&mut db, niels, karen, "1866", "Aalborg, Denmark", &[clara]);
    f(&mut db, samuel, ida, "1896", "Boise, Ada, Idaho, USA", &[margaret]);
    f(&mut db, william, clara, "1895", "Salt Lake City, Salt Lake, Utah, USA", &[thomas, claraj]);
    let tm = f(&mut db, thomas, margaret, "14 Jun 1924", "Boise, Ada, Idaho, USA", &[walter, ruth]);
    f(&mut db, walter, helen, "1950", "Portland, Multnomah, Oregon, USA", &[robert, susan]);
    f(&mut db, george, ruth, "1951", "Portland, Multnomah, Oregon, USA", &[]);
    f(&mut db, robert, linda, "1996", "Sacramento, California, USA", &[emily, daniel]);

    // sources and repositories (all made up)
    let rid = db.alloc();
    db.repos.insert(rid, RepoRec { name: "Sample County Archives".into(), addr: "1 Records Lane, Twin Falls, Idaho".into(), ..Default::default() });
    let src = |db: &mut Db, title: &str, kind: &str| -> u32 {
        let id = db.alloc();
        db.sources.insert(id, SourceRec { title: title.into(), kind: kind.into(), repo: Some(rid), ..Default::default() });
        id
    };
    let s_census = src(&mut db, "1930 Census, Twin Falls County, Idaho (sample)", "Census");
    let s_marr = src(&mut db, "Ada County Marriage Register (sample)", "Vital record");
    let s_bible = src(&mut db, "Harwood Family Bible (sample)", "Family record");
    let s_ship = src(&mut db, "Passenger List, ship Minnesota, 1868 (sample)", "Immigration record");
    let s_obit = src(&mut db, "Eugene Daily Register, obituary of Thomas Harwood, Dec 1975 (sample)", "Newspaper");
    let s_church = src(&mut db, "St Peter's Church baptism register, Leeds (sample)", "Church record");
    let cite = |sid: u32, about: &str, page: &str, q: &str| Source { sid: Some(sid), about: about.into(), page: page.into(), quality: q.into(), ..Default::default() };

    let conf = |p: &mut Person, tag: &str, c: &str| {
        p.fx.entry(tag.to_string()).or_default().conf = c.to_string();
    };
    if let Some(x) = db.people.get_mut(&thomas) {
        x.sources.push(cite(s_bible, "Birth", "Family record page", "2"));
        x.sources.push(cite(s_census, "General", "Sheet 4A, line 12", "3"));
        x.sources.push(cite(s_marr, "Marriage", "Volume 7, page 112", "3"));
        x.sources.push(cite(s_obit, "Death", "Page 6", "2"));
        conf(x, "BIRT", "supported");
        conf(x, "DEAT", "confirmed");
        x.events.push(Event { kind: "Census".into(), date: "1930".into(), place: "Twin Falls, Twin Falls, Idaho, USA".into(), val: "Farmer, own farm".into(), conf: "confirmed".into(), cites: vec![cite(s_census, "", "Sheet 4A, line 12", "3")], ..Default::default() });
        x.events.push(Event { kind: "Residence".into(), date: "1955".into(), place: "Eugene, Lane, Oregon, USA".into(), conf: "probable".into(), ..Default::default() });
        x.notes = "Known as Tom. Farmed sugar beets near Twin Falls for thirty years, then moved to Eugene to be near Walter.".into();
        x.media.push(media("sample-thomas.jpg", "Thomas Harwood, about 1898"));
        x.media.push(media("sample-wedding.jpg", "Wedding of Thomas and Margaret, 1924"));
        x.media.push(media("sample-farm.jpg", "The Harwood farm near Twin Falls"));
        x.media.push(media("sample-marriage.pdf", "Certificate of marriage, 1924"));
        x.media.push(media("sample-census.pdf", "1930 census extract"));
        x.media.push(Media { file: "sample-story-farm.txt".into(), name: "The farm near Twin Falls.txt".into(), caption: "The farm near Twin Falls".into(), form: "story".into(), ..Default::default() });
        x.portrait = "sample-thomas.jpg".into();
        // the wedding photo has both faces tagged
        if let Some(m) = x.media.iter_mut().find(|m| m.file == "sample-wedding.jpg") {
            m.faces.push(Face { who: thomas, r: [0.29, 0.16, 0.17, 0.27] });
            m.faces.push(Face { who: margaret, r: [0.54, 0.18, 0.17, 0.27] });
        }
        x.todos.push(Todo { text: "Find Thomas's WWI draft card (1918)".into(), added: "2026-01-12".into(), ..Default::default() });
        x.todos.push(Todo { text: "Order the 1975 death certificate from Oregon".into(), added: "2026-01-12".into(), done: true, ..Default::default() });
    }
    if let Some(x) = db.people.get_mut(&margaret) {
        x.sources.push(cite(s_marr, "Marriage", "Volume 7, page 112", "3"));
        x.sources.push(cite(s_census, "General", "Sheet 4A, line 13", "3"));
        conf(x, "BIRT", "probable");
        x.media.push(media("sample-margaret.jpg", "Margaret Ellis, about 1902"));
        x.portrait = "sample-margaret.jpg".into();
        x.notes = "Kept the farm ledger for forty years.".into();
        x.todos.push(Todo { text: "Confirm Margaret's birth date: family Bible says 19 Mar, a census says 1903".into(), added: "2026-02-03".into(), ..Default::default() });
    }
    if let Some(x) = db.people.get_mut(&walter) {
        x.media.push(media("sample-walter.jpg", "Walter Harwood, 1946"));
        x.portrait = "sample-walter.jpg".into();
        x.events.push(Event { kind: "Military service".into(), date: "1944".into(), place: "Fort Lewis, Washington, USA".into(), val: "U.S. Army".into(), conf: "uncertain".into(), ..Default::default() });
        x.sources.push(cite(s_census, "Birth", "Sheet 4A, line 14 (age 4)", "2"));
        conf(x, "BIRT", "supported");
    }
    if let Some(x) = db.people.get_mut(&emily) {
        x.media.push(media("sample-emily.jpg", "Emily Rose Harwood"));
        x.portrait = "sample-emily.jpg".into();
    }
    if let Some(x) = db.people.get_mut(&james) {
        x.sources.push(cite(s_church, "Birth", "Entry 214", "3"));
        x.sources.push(cite(s_ship, "General", "Line 33", "2"));
        conf(x, "BIRT", "confirmed");
        conf(x, "DEAT", "disputed");
        x.events.push(Event { kind: "Immigration".into(), date: "Aug 1868".into(), place: "New York, New York, USA".into(), val: "Arrived on the Minnesota from Liverpool".into(), conf: "supported".into(), cites: vec![cite(s_ship, "", "Line 33", "2")], ..Default::default() });
        x.notes = "Death year is disputed: a cemetery index says 1901, a family letter says 1902.".into();
    }
    if let Some(x) = db.people.get_mut(&clara) {
        x.events.push(Event { kind: "Immigration".into(), date: "1885".into(), place: "Salt Lake City, Salt Lake, Utah, USA".into(), conf: "probable".into(), ..Default::default() });
    }
    if let Some(fm) = db.fams.get_mut(&tm) {
        fm.cites.push(cite(s_marr, "Marriage", "Volume 7, page 112", "3"));
        fm.mx.conf = "confirmed".into();
    }
    // research journal
    db.journal.push(Journal { date: "12 Jan 2026".into(), goal: "Thomas Harwood's marriage record".into(), place: "Sample County Archives, marriage registers 1920-1926".into(), sid: Some(s_marr), result: "found".into(), notes: "Found in volume 7, page 112. Witnesses Samuel Ellis and Clara Harwood.".into(), people: vec![thomas, margaret] });
    db.journal.push(Journal { date: "3 Feb 2026".into(), goal: "Margaret Ellis's birth record".into(), place: "Idaho state birth index, 1900-1905 (online)".into(), sid: None, result: "nothing".into(), notes: "Not indexed under Ellis or Elis. Try the Ada County delayed birth registers next.".into(), people: vec![margaret] });
    db.journal.push(Journal { date: "20 Feb 2026".into(), goal: "James Harwood's arrival in America".into(), place: "Passenger lists, New York, 1868".into(), sid: Some(s_ship), result: "partial".into(), notes: "A James Harwood, 30, labourer, on the Minnesota. Wife and son listed. Age fits; need a second source.".into(), people: vec![james] });
    db.bookmarks.push(thomas);
    db.home = Some(emily);
    let _ = (claraj, niels, karen, samuel, ida, george, susan, linda, daniel, helen);
    fix_names(&mut db);
    db
}
