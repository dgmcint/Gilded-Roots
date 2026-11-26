//! The Research tab: your own research files kept with the tree: archived emails (.eml, .mbox,
//! Outlook .msg), scanned notes and letters, recordings, PDFs, documents and saved web pages.
//! Each can be tagged, searched, linked to the people and facts it is about, given a
//! transcription, and cited as a source's evidence. Files are copied into the media folder;
//! the originals are never changed.

use super::*;

/// What kind of research file it is.
pub(crate) const RKINDS: [&str; 8] = ["Other", "Email", "Recording", "PDF", "Document", "Scan or picture", "Web page", "Video"];

/// One research file.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub(crate) struct ResearchItem {
    pub(crate) id: u32,
    /// The file in the media folder ("" for an email kept only as text).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) file: String,
    /// The file's name as it was.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) name: String,
    /// An index into RKINDS.
    pub(crate) kind: u8,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) title: String,
    /// The date it is from (an email's date, a letter's date), as text.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) date: String,
    /// When it was added.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) added: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tags: Vec<String>,
    /// The people it is about.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) people: Vec<u32>,
    /// The families it is about.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) fams: Vec<u32>,
    /// Facts it is about: (person, fact such as "Birth").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) facts: Vec<(u32, String)>,
    /// The source it is evidence for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sid: Option<u32>,
    /// A transcription or notes.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) email: Option<EmailInfo>,
}

/// What an email says about itself.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub(crate) struct EmailInfo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) from: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) to: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) cc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) subject: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) date: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) body: String,
    /// The attachments kept, as media file names, with their names as they were.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) attachments: Vec<(String, String)>,
    /// Attachments of kinds not kept (a program, a zip), by name.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) skipped: Vec<String>,
}

/// Works out the kind from a file name.
pub(crate) fn rkind_of(name: &str) -> u8 {
    match ext_of(name).as_str() {
        "eml" | "mbox" | "msg" | "emlx" => 1,
        "mp3" | "m4a" | "wav" | "ogg" | "flac" | "aac" => 2,
        "pdf" => 3,
        "doc" | "docx" | "rtf" | "txt" | "odt" | "xls" | "xlsx" | "csv" | "ods" => 4,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "heif" | "tif" | "tiff" => 5,
        "htm" | "html" | "mht" | "mhtml" | "webarchive" => 6,
        "mp4" | "mov" | "avi" => 7,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------------------------
// Reading emails

/// An email read from a file.
#[derive(Default, Debug, Clone)]
pub(crate) struct Email {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) cc: String,
    pub(crate) subject: String,
    pub(crate) date: String,
    pub(crate) body: String,
    pub(crate) attachments: Vec<(String, Vec<u8>)>,
}

fn b64(s: &[u8]) -> Vec<u8> {
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut n) = (0u32, 0);
    for &c in s {
        if c == b'=' {
            break;
        }
        if let Some(v) = val(c) {
            acc = (acc << 6) | v;
            n += 1;
            if n == 4 {
                out.extend_from_slice(&[(acc >> 16) as u8, (acc >> 8) as u8, acc as u8]);
                acc = 0;
                n = 0;
            }
        }
    }
    if n == 3 {
        out.extend_from_slice(&[(acc >> 10) as u8, (acc >> 2) as u8]);
    } else if n == 2 {
        out.push((acc >> 4) as u8);
    }
    out
}

fn qp(s: &[u8], header: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'=' {
            // soft line break
            if s.get(i + 1) == Some(&b'\r') && s.get(i + 2) == Some(&b'\n') {
                i += 3;
                continue;
            }
            if s.get(i + 1) == Some(&b'\n') {
                i += 2;
                continue;
            }
            let hex = |b: u8| (b as char).to_digit(16);
            if let (Some(a), Some(b)) = (s.get(i + 1).and_then(|b| hex(*b)), s.get(i + 2).and_then(|b| hex(*b))) {
                out.push((a * 16 + b) as u8);
                i += 3;
                continue;
            }
            out.push(c);
        } else if header && c == b'_' {
            out.push(b' ');
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Bytes in a character set to text (UTF-8, or the Western European sets older email used).
fn decode_charset(b: &[u8], charset: &str) -> String {
    let cs = charset.trim().trim_matches('"').to_lowercase();
    if cs.contains("8859") || cs.contains("1252") || cs.contains("latin") || cs == "ascii" || cs == "us-ascii" {
        if std::str::from_utf8(b).is_err() {
            return b.iter().map(|&c| c as char).collect();
        }
    }
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) if cs.is_empty() => b.iter().map(|&c| c as char).collect(),
        Err(_) => String::from_utf8_lossy(b).to_string(),
    }
}

/// Header text with its =?charset?B?...?= and =?charset?Q?...?= parts decoded.
pub(crate) fn decode_header(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    let mut last_was_word = false;
    while let Some(start) = rest.find("=?") {
        let before = &rest[..start];
        let after = &rest[start + 2..];
        let parts: Vec<&str> = after.splitn(3, '?').collect();
        if parts.len() < 3 {
            break;
        }
        let Some(end) = parts[2].find("?=") else { break };
        let (cs, enc, text) = (parts[0], parts[1], &parts[2][..end]);
        // spaces between two encoded words are left out
        if !(last_was_word && before.trim().is_empty()) {
            out.push_str(before);
        }
        let bytes = if enc.eq_ignore_ascii_case("b") { b64(text.as_bytes()) } else { qp(text.as_bytes(), true) };
        out.push_str(&decode_charset(&bytes, cs));
        last_was_word = true;
        let used = start + 2 + parts[0].len() + 1 + parts[1].len() + 1 + end + 2;
        rest = &rest[used.min(rest.len())..];
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// Splits a message into its headers (name in lower case, value unfolded) and body.
fn split_message(b: &[u8]) -> (Vec<(String, String)>, &[u8]) {
    let mut cut = b.len();
    let mut body_at = b.len();
    for i in 0..b.len() {
        if b[i] == b'\n' && (b.get(i + 1) == Some(&b'\n') || (b.get(i + 1) == Some(&b'\r') && b.get(i + 2) == Some(&b'\n'))) {
            cut = i;
            body_at = if b.get(i + 1) == Some(&b'\n') { i + 2 } else { i + 3 };
            break;
        }
    }
    let head = String::from_utf8_lossy(&b[..cut]).to_string();
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in head.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(h) = headers.last_mut() {
                h.1.push(' ');
                h.1.push_str(line.trim());
            }
        } else if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_lowercase(), v.trim().to_string()));
        }
    }
    (headers, &b[body_at.min(b.len())..])
}

fn header<'a>(h: &'a [(String, String)], name: &str) -> &'a str {
    h.iter().find(|x| x.0 == name).map(|x| x.1.as_str()).unwrap_or("")
}

/// A parameter of a header value: boundary="abc", charset=utf-8, filename="x.pdf".
fn param(v: &str, name: &str) -> String {
    for p in v.split(';').skip(1) {
        if let Some((k, val)) = p.split_once('=') {
            let k = k.trim().to_lowercase();
            if k == name || k == format!("{name}*") {
                let val = val.trim().trim_matches('"');
                // RFC 2231: utf-8''name%20here
                if k.ends_with('*') {
                    if let Some((_, enc)) = val.split_once("''") {
                        let bytes = qp(enc.replace('%', "=").as_bytes(), false);
                        return String::from_utf8_lossy(&bytes).to_string();
                    }
                }
                return decode_header(val);
            }
        }
    }
    String::new()
}

/// Text from HTML, for emails that only have an HTML version.
pub(crate) fn html_to_text(h: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut tag = String::new();
    let mut skip = false;
    for c in h.chars() {
        if in_tag {
            if c == '>' {
                in_tag = false;
                let t = tag.to_lowercase();
                let name = t.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_string();
                if name == "style" || name == "script" || name == "head" {
                    skip = !t.starts_with('/');
                }
                if matches!(name.as_str(), "br" | "p" | "div" | "tr" | "li" | "h1" | "h2" | "h3" | "blockquote") {
                    out.push('\n');
                }
                tag.clear();
            } else {
                tag.push(c);
            }
        } else if c == '<' {
            in_tag = true;
        } else if !skip {
            out.push(c);
        }
    }
    let out = out.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'");
    // no more than one empty line in a row
    let mut tidy = String::new();
    let mut blank = 0;
    for l in out.lines() {
        let l = l.trim_end();
        if l.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        tidy.push_str(l);
        tidy.push('\n');
    }
    tidy.trim().to_string()
}

/// Decodes one MIME part's body.
fn part_bytes(h: &[(String, String)], body: &[u8]) -> Vec<u8> {
    match header(h, "content-transfer-encoding").to_lowercase().trim() {
        "base64" => b64(body),
        "quoted-printable" => qp(body, false),
        _ => body.to_vec(),
    }
}

fn walk(h: &[(String, String)], body: &[u8], e: &mut Email, plain: &mut Option<String>, html: &mut Option<String>, depth: usize) {
    let ct = header(h, "content-type");
    let ctl = ct.to_lowercase();
    if ctl.starts_with("multipart/") && depth < 10 {
        let b = param(ct, "boundary");
        if b.is_empty() {
            return;
        }
        let marker = format!("--{b}");
        let text = body;
        let mut parts: Vec<&[u8]> = Vec::new();
        let mut starts: Vec<usize> = Vec::new();
        let mb = marker.as_bytes();
        let mut i = 0;
        while i + mb.len() <= text.len() {
            if &text[i..i + mb.len()] == mb && (i == 0 || text[i - 1] == b'\n') {
                starts.push(i);
                i += mb.len();
            } else {
                i += 1;
            }
        }
        for w in starts.windows(2) {
            let mut s = w[0] + mb.len();
            if text.get(s) == Some(&b'\r') {
                s += 1;
            }
            if text.get(s) == Some(&b'\n') {
                s += 1;
            }
            let mut end = w[1];
            if end > 0 && text[end - 1] == b'\n' {
                end -= 1;
            }
            if end > 0 && text[end - 1] == b'\r' {
                end -= 1;
            }
            if s <= end {
                parts.push(&text[s..end]);
            }
        }
        for p in parts {
            let (ph, pb) = split_message(p);
            walk(&ph, pb, e, plain, html, depth + 1);
        }
        return;
    }
    let disp = header(h, "content-disposition");
    let mut fname = param(disp, "filename");
    if fname.is_empty() {
        fname = param(ct, "name");
    }
    let is_text = ctl.is_empty() || ctl.starts_with("text/plain") || ctl.starts_with("text/html");
    if !fname.is_empty() || disp.to_lowercase().starts_with("attachment") || !is_text {
        if ctl.starts_with("message/rfc822") {
            let inner = part_bytes(h, body);
            let (ih, ib) = split_message(&inner);
            walk(&ih, ib, e, plain, html, depth + 1);
            return;
        }
        let name = if fname.is_empty() { "attachment".to_string() } else { fname };
        e.attachments.push((name, part_bytes(h, body)));
        return;
    }
    let text = decode_charset(&part_bytes(h, body), &param(ct, "charset"));
    if ctl.starts_with("text/html") {
        if html.is_none() {
            *html = Some(text);
        }
    } else if plain.is_none() {
        *plain = Some(text);
    }
}

/// Reads a saved email (.eml, or one message from a mailbox).
pub(crate) fn parse_eml(b: &[u8]) -> Email {
    // Apple Mail's .emlx starts with the message's length on its own line
    let mut b = b;
    if let Some(nl) = b.iter().position(|c| *c == b'\n') {
        if nl < 12 && b[..nl].iter().all(|c| c.is_ascii_digit() || *c == b' ' || *c == b'\r') && nl > 0 {
            b = &b[nl + 1..];
        }
    }
    let (h, body) = split_message(b);
    let mut e = Email {
        from: decode_header(header(&h, "from")),
        to: decode_header(header(&h, "to")),
        cc: decode_header(header(&h, "cc")),
        subject: decode_header(header(&h, "subject")),
        date: header(&h, "date").to_string(),
        ..Default::default()
    };
    let (mut plain, mut html) = (None, None);
    walk(&h, body, &mut e, &mut plain, &mut html, 0);
    e.body = plain.map(|p| p.replace("\r\n", "\n").trim().to_string()).or_else(|| html.map(|h| html_to_text(&h))).unwrap_or_default();
    e
}

/// The separate messages in a mailbox (.mbox): each starts with a line "From ...".
pub(crate) fn split_mbox(b: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    let mut started = false;
    for line in b.split_inclusive(|c| *c == b'\n') {
        if line.starts_with(b"From ") {
            if started && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            started = true;
            continue;
        }
        if started {
            // a line in the text that began ">From " was escaped
            if line.starts_with(b">From ") {
                cur.extend_from_slice(&line[1..]);
            } else {
                cur.extend_from_slice(line);
            }
        }
    }
    if started && !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Reads an Outlook message (.msg).
pub(crate) fn parse_msg(b: &[u8]) -> Result<Email, String> {
    use std::io::Read;
    let mut c = cfb::CompoundFile::open(std::io::Cursor::new(b)).map_err(|_| "This .msg file could not be read.".to_string())?;
    let read = |c: &mut cfb::CompoundFile<std::io::Cursor<&[u8]>>, path: &str| -> Option<Vec<u8>> {
        let mut s = c.open_stream(path).ok()?;
        let mut v = Vec::new();
        s.read_to_end(&mut v).ok()?;
        Some(v)
    };
    let utf16 = |v: Vec<u8>| -> String {
        let w: Vec<u16> = v.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
        String::from_utf16_lossy(&w).trim_end_matches('\0').to_string()
    };
    let text = |c: &mut cfb::CompoundFile<std::io::Cursor<&[u8]>>, base: &str, id: &str| -> String {
        if let Some(v) = read(c, &format!("{base}__substg1.0_{id}001F")) {
            return utf16(v);
        }
        if let Some(v) = read(c, &format!("{base}__substg1.0_{id}001E")) {
            return String::from_utf8_lossy(&v).trim_end_matches('\0').to_string();
        }
        String::new()
    };
    let mut e = Email {
        subject: text(&mut c, "/", "0037"),
        to: text(&mut c, "/", "0E04"),
        cc: text(&mut c, "/", "0E03"),
        body: text(&mut c, "/", "1000"),
        ..Default::default()
    };
    let name = text(&mut c, "/", "0C1A");
    let addr = text(&mut c, "/", "0C1F");
    e.from = match (name.is_empty(), addr.is_empty() || addr.starts_with("/O=")) {
        (false, false) => format!("{name} <{addr}>"),
        (false, true) => name,
        _ => addr,
    };
    // the date, from the headers the message was sent with
    let headers = text(&mut c, "/", "007D");
    if let Some(l) = headers.lines().find(|l| l.to_lowercase().starts_with("date:")) {
        e.date = l[5..].trim().to_string();
    }
    if e.body.is_empty() {
        if let Some(h) = read(&mut c, "/__substg1.0_10130102") {
            e.body = html_to_text(&String::from_utf8_lossy(&h));
        }
    }
    let attach: Vec<String> = c.read_root_storage().filter(|en| en.is_storage() && en.name().starts_with("__attach_version1.0_")).map(|en| en.name().to_string()).collect();
    for a in attach {
        let base = format!("/{a}/");
        let mut nm = text(&mut c, &base, "3707");
        if nm.is_empty() {
            nm = text(&mut c, &base, "3704");
        }
        if let Some(data) = read(&mut c, &format!("{base}__substg1.0_37010102")) {
            e.attachments.push((if nm.is_empty() { "attachment".into() } else { nm }, data));
        }
    }
    Ok(e)
}

/// An email's date as people write it ("3 Mar 2015"), from "Tue, 3 Mar 2015 10:12:00 -0800".
pub(crate) fn email_day(d: &str) -> String {
    let months = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    let words: Vec<&str> = d.split(|c: char| c.is_whitespace() || c == ',').filter(|w| !w.is_empty()).collect();
    for i in 0..words.len().saturating_sub(2) {
        let (a, b, c) = (words[i], words[i + 1], words[i + 2]);
        if let (Ok(day), Some(_), Ok(year)) = (a.parse::<u32>(), months.iter().position(|m| b.to_lowercase().starts_with(m)), c.parse::<u32>()) {
            if (1..=31).contains(&day) && year > 1900 {
                let mut m = b[..3.min(b.len())].to_lowercase();
                m[..1].make_ascii_uppercase();
                return format!("{day} {m} {year}");
            }
        }
    }
    d.trim().to_string()
}

/// Keeps a file's bytes in the media folder under a new unique name. Only kinds of file that
/// can be opened safely are kept.
fn keep_bytes(name: &str, data: &[u8]) -> Option<String> {
    if !is_media_name(name) {
        return None;
    }
    let file = format!("{}-{}", stamp(), sanitize(name));
    std::fs::write(media_dir().join(&file), data).ok()?;
    Some(file)
}

/// Makes research items from a file chosen to add. A mailbox gives one item per message.
pub(crate) fn items_from_file(path: &Path) -> Result<Vec<ResearchItem>, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = ext_of(&name);
    let today = today();
    let email_item = |e: Email, file: String, nm: String| -> ResearchItem {
        let mut kept = Vec::new();
        let mut skipped = Vec::new();
        for (an, data) in &e.attachments {
            match keep_bytes(an, data) {
                Some(f) => kept.push((f, an.clone())),
                None => skipped.push(an.clone()),
            }
        }
        ResearchItem {
            file,
            name: nm,
            kind: 1,
            title: if e.subject.is_empty() { "(no subject)".to_string() } else { e.subject.clone() },
            date: email_day(&e.date),
            added: today.clone(),
            email: Some(EmailInfo { from: e.from, to: e.to, cc: e.cc, subject: e.subject, date: e.date, body: e.body, attachments: kept, skipped }),
            ..Default::default()
        }
    };
    // an Apple Mail mailbox export is a folder ending .mbox with a file "mbox" inside
    if path.is_dir() {
        let inner = path.join("mbox");
        if inner.is_file() {
            return items_from_mbox(&std::fs::read(&inner).map_err(|e| io_msg(&e))?, &name, &email_item);
        }
        return Err(format!("{name} is a folder. Choose the files inside it, or use Add a folder."));
    }
    let bytes = std::fs::read(path).map_err(|e| io_msg(&e))?;
    match ext.as_str() {
        "mbox" => items_from_mbox(&bytes, &name, &email_item),
        "eml" | "emlx" => {
            let e = parse_eml(&bytes);
            let file = keep_bytes(&if ext == "emlx" { format!("{}.eml", name.trim_end_matches(".emlx")) } else { name.clone() }, &bytes).unwrap_or_default();
            Ok(vec![email_item(e, file, name)])
        }
        "msg" => {
            let e = parse_msg(&bytes)?;
            let file = keep_bytes(&name, &bytes).unwrap_or_default();
            Ok(vec![email_item(e, file, name)])
        }
        "pst" | "ost" => Err("Outlook data files (.pst) can't be read directly. In Outlook, drag the emails you want to a folder (they become .msg files), or export them, then add those.".to_string()),
        _ => {
            if !is_media_name(&name) {
                return Err(format!("{name}: this kind of file can't be kept. Photos, scans, PDFs, documents, recordings, web pages and emails can."));
            }
            let (file, shown) = import_one(path, &name).ok_or_else(|| format!("Could not copy {name}."))?;
            let stem = Path::new(&shown).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            Ok(vec![ResearchItem { file, name: shown, kind: rkind_of(&name), title: stem.replace(['_', '-'], " "), added: today.clone(), ..Default::default() }])
        }
    }
}

fn items_from_mbox(bytes: &[u8], name: &str, make: &dyn Fn(Email, String, String) -> ResearchItem) -> Result<Vec<ResearchItem>, String> {
    let msgs = split_mbox(bytes);
    if msgs.is_empty() {
        return Err(format!("{name} has no emails in it."));
    }
    let mut out = Vec::new();
    for (i, m) in msgs.iter().enumerate() {
        let e = parse_eml(m);
        let base = sanitize(&if e.subject.is_empty() { format!("email {}", i + 1) } else { e.subject.chars().take(50).collect::<String>() });
        let file = keep_bytes(&format!("{base}.eml"), m).unwrap_or_default();
        let mut it = make(e, file, format!("{name} #{}", i + 1));
        it.tags.push(Path::new(name).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
        out.push(it);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------

impl Db {
    pub(crate) fn research_next_id(&self) -> u32 {
        self.research.iter().map(|r| r.id).max().unwrap_or(0) + 1
    }

    /// Every tag used, in order.
    pub(crate) fn research_tags(&self) -> Vec<String> {
        let mut t: Vec<String> = self.research.iter().flat_map(|r| r.tags.iter().cloned()).filter(|t| !t.trim().is_empty()).collect::<BTreeSet<_>>().into_iter().collect();
        t.sort_by_key(|s| s.to_lowercase());
        t
    }

    /// The research files about a person (linked to them, their facts, or their families).
    pub(crate) fn research_for(&self, id: u32) -> Vec<&ResearchItem> {
        let fams = self.fams_of(id);
        self.research.iter().filter(|r| r.people.contains(&id) || r.facts.iter().any(|f| f.0 == id) || r.fams.iter().any(|f| fams.contains(f))).collect()
    }
}

/// A family's name for a list: "John Smith & Mary Jones".
pub(crate) fn fam_label(db: &Db, f: u32) -> String {
    let Some(fam) = db.fams.get(&f) else { return format!("#{f}") };
    let n = |p: Option<u32>| p.and_then(|p| db.people.get(&p)).map(|p| p.name.clone());
    match (n(fam.h), n(fam.w)) {
        (Some(a), Some(b)) => format!("{a} & {b}"),
        (Some(a), None) | (None, Some(a)) => a,
        _ => format!("#{f}"),
    }
}

/// Does the item match the search words (title, date, tags, transcription, email, names)?
fn matches(db: &Db, r: &ResearchItem, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let mut hay = format!("{} {} {} {} {}", r.title, r.date, r.name, r.tags.join(" "), r.text);
    if let Some(e) = &r.email {
        hay.push_str(&format!(" {} {} {} {} {}", e.from, e.to, e.cc, e.subject, e.body));
    }
    for p in &r.people {
        if let Some(pp) = db.people.get(p) {
            hay.push(' ');
            hay.push_str(&pp.name);
        }
    }
    let hay = hay.to_lowercase();
    q.split_whitespace().all(|w| hay.contains(w))
}

impl App {
    /// Add research files: choose files (or a whole folder), and each becomes an item.
    pub(crate) fn research_add(&mut self, folder: bool) {
        let mut paths: Vec<PathBuf> = Vec::new();
        if folder {
            let Some(dir) = rfd::FileDialog::new().set_title(crate::i18n::tr("Choose a folder of research files (or an exported Mail mailbox)")).set_directory(start_dir()).pick_folder() else { return };
            remember_dir(&dir);
            if dir.join("mbox").is_file() {
                paths.push(dir);
            } else if let Ok(rd) = std::fs::read_dir(&dir) {
                let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_file() || p.join("mbox").is_file()).collect();
                v.sort();
                paths.extend(v.into_iter().filter(|p| {
                    let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    !n.starts_with('.') && (is_media_name(&n) || matches!(ext_of(&n).as_str(), "eml" | "emlx" | "mbox" | "msg") || p.is_dir())
                }));
            }
        } else {
            let Some(v) = rfd::FileDialog::new().set_title(crate::i18n::tr("Choose research files: emails, scans, PDFs, documents, recordings, web pages")).set_directory(start_dir()).pick_files() else { return };
            if let Some(p) = v.first() {
                remember_dir(p);
            }
            paths = v;
        }
        let (mut n, mut errs) = (0, Vec::new());
        let link = if self.x.research_link_sel { self.sel.filter(|s| self.db.people.contains_key(s)) } else { None };
        for p in paths {
            match items_from_file(&p) {
                Ok(items) => {
                    for mut it in items {
                        it.id = self.db.research_next_id();
                        if let Some(s) = link {
                            it.people.push(s);
                        }
                        self.x.research_sel = Some(it.id);
                        self.db.research.push(it);
                        n += 1;
                    }
                }
                Err(e) => errs.push(e),
            }
        }
        if n > 0 {
            self.dirty = true;
            self.log_research(&format!("Added {} research file(s)", n), link);
        }
        self.status = match (n, errs.first()) {
            (_, Some(e)) if n == 0 => e.clone(),
            (_, Some(e)) => format!("Added {} item(s). {} could not be added: {}", n, errs.len(), e),
            _ => format!("Added {} item(s) to Research.", n),
        };
    }

    /// The Research tab.
    pub(crate) fn research_view(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let tr = crate::i18n::tr;
        ui.horizontal(|ui| {
            ui.heading(tr("Research"));
            ui.add_space(12.0);
            let n = [self.db.research.len(), self.db.theories.len(), self.db.letters.len()];
            for (i, l) in ["Files", "Theories", "Letters"].iter().enumerate() {
                if ui.selectable_label(self.x.research_tab as usize == i, RichText::new(format!("{} ({})", crate::i18n::trv(l), n[i])).size(15.0)).clicked() {
                    self.x.research_tab = i as u8;
                }
            }
        });
        ui.separator();
        match self.x.research_tab {
            1 => return self.theories_view(ui, act),
            2 => return self.letters_view(ui, act),
            _ => {}
        }
        ui.horizontal_wrapped(|ui| {
            ui.menu_button(RichText::new(tr("+ Add")).strong(), |ui| {
                if ui.button(tr("Files... (emails, scans, PDFs, documents, recordings)")).clicked() {
                    self.research_add(false);
                    ui.close_menu();
                }
                if ui.button(tr("A whole folder (or an exported Mail mailbox)...")).clicked() {
                    self.research_add(true);
                    ui.close_menu();
                }
                ui.separator();
                let who = self.sel.filter(|s| self.db.people.contains_key(s)).map(|s| self.db.label(s));
                ui.add_enabled_ui(who.is_some(), |ui| {
                    ui.checkbox(&mut self.x.research_link_sel, crate::i18n::trf("Link new files to {}", &["Link new files to ", ""], &[who.unwrap_or_default()]));
                });
            });
            ui.separator();
            for (k, l) in [(None, "All")].into_iter().chain(RKINDS.iter().enumerate().map(|(i, l)| (Some(i as u8), *l))) {
                let n = match k { None => self.db.research.len(), Some(k) => self.db.research.iter().filter(|r| r.kind == k).count() };
                if k.is_some() && n == 0 {
                    continue;
                }
                if ui.selectable_label(self.x.research_kind == k, format!("{} ({})", crate::i18n::trv(l), n)).clicked() {
                    self.x.research_kind = k;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.x.research_q).hint_text(tr("Search titles, emails, transcriptions, tags, names")).desired_width(320.0));
            ui.checkbox(&mut self.x.research_only_sel, tr("Only about the selected person"));
            let tags = self.db.research_tags();
            if !tags.is_empty() {
                ui.separator();
                ui.label(RichText::new(tr("Tags:")).weak());
                for t in tags {
                    let on = self.x.research_tag == t;
                    if ui.selectable_label(on, &t).clicked() {
                        self.x.research_tag = if on { String::new() } else { t };
                    }
                }
            }
        });
        ui.separator();
        if self.db.research.is_empty() {
            ui.add_space(20.0);
            ui.label(RichText::new(tr("Keep your own research here: archived emails from relatives and archives, scanned notes and letters, recordings, PDFs, documents and saved web pages.")).size(15.0));
            ui.label(tr("Use + Add to bring in files. Emails (.eml, .mbox from Gmail or Apple Mail, Outlook .msg) can be read here, with their attachments kept. Each item can be tagged, searched, linked to the people and facts it is about, given a transcription, and cited as evidence on a source."));
            return;
        }
        // the list on the left, the chosen item on the right
        let q = self.x.research_q.trim().to_lowercase();
        let sel = self.sel;
        let shown: Vec<u32> = self
            .db
            .research
            .iter()
            .filter(|r| self.x.research_kind.map_or(true, |k| r.kind == k))
            .filter(|r| self.x.research_tag.is_empty() || r.tags.contains(&self.x.research_tag))
            .filter(|r| !self.x.research_only_sel || sel.map_or(false, |s| r.people.contains(&s) || r.facts.iter().any(|f| f.0 == s)))
            .filter(|r| matches(&self.db, r, &q))
            .map(|r| r.id)
            .collect();
        let avail = ui.available_size();
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2((avail.x * 0.42).clamp(300.0, 560.0), avail.y), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(RichText::new(crate::i18n::trf("{} item(s)", &["", " item(s)"], &[shown.len().to_string()])).small().weak());
                let h = 44.0;
                egui::ScrollArea::vertical().id_salt("research_list").auto_shrink([false, false]).show_rows(ui, h, shown.len(), |ui, range| {
                    for id in &shown[range] {
                        let Some(r) = self.db.research.iter().find(|r| r.id == *id) else { continue };
                        let on = self.x.research_sel == Some(*id);
                        let people: Vec<String> = r.people.iter().filter_map(|p| self.db.people.get(p)).map(|p| p.name.clone()).collect();
                        let resp = ui.add_sized([ui.available_width(), h - 4.0], egui::SelectableLabel::new(on, ""));
                        let rect = resp.rect.shrink2(vec2(8.0, 4.0));
                        let col = ui.visuals().text_color();
                        let weak = ui.visuals().weak_text_color();
                        let badge = crate::i18n::trv(RKINDS.get(r.kind as usize).copied().unwrap_or("Other"));
                        ui.painter().text(rect.left_top(), Align2::LEFT_TOP, format!("{}  {}", r.title, r.date), FontId::proportional(14.0), col);
                        let sub = if people.is_empty() { badge.to_string() } else { format!("{badge} · {}", people.join(", ")) };
                        ui.painter().text(rect.left_bottom(), Align2::LEFT_BOTTOM, sub, FontId::proportional(11.5), weak);
                        if resp.clicked() {
                            self.x.research_sel = Some(*id);
                        }
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                egui::ScrollArea::vertical().id_salt("research_item").auto_shrink([false, false]).show(ui, |ui| {
                    self.research_item(ui, act);
                });
            });
        });
    }

    /// The chosen research item: what it is, the email or file, its links, tags and transcription.
    fn research_item(&mut self, ui: &mut egui::Ui, act: &mut Option<Action>) {
        let tr = crate::i18n::tr;
        let Some(id) = self.x.research_sel else {
            ui.label(RichText::new(tr("Choose an item on the left.")).weak());
            return;
        };
        let Some(ix) = self.db.research.iter().position(|r| r.id == id) else {
            ui.label(RichText::new(tr("Choose an item on the left.")).weak());
            return;
        };
        let mut changed = false;
        let mut remove = false;
        let sel = self.sel.filter(|s| self.db.people.contains_key(s));
        let sel_name = sel.map(|s| self.db.label(s)).unwrap_or_default();
        let r = self.db.research[ix].clone();
        {
            let it = &mut self.db.research[ix];
            changed |= ui.add(egui::TextEdit::singleline(&mut it.title).font(egui::TextStyle::Heading).desired_width(f32::INFINITY)).changed();
            ui.horizontal(|ui| {
                ui.label(tr("Date:"));
                changed |= ui.add(egui::TextEdit::singleline(&mut it.date).desired_width(140.0)).changed();
                ui.label(tr("Kind:"));
                egui::ComboBox::from_id_salt("rkind").selected_text(crate::i18n::trv(RKINDS.get(it.kind as usize).copied().unwrap_or("Other"))).show_ui(ui, |ui| {
                    for (k, l) in RKINDS.iter().enumerate() {
                        if ui.selectable_label(it.kind as usize == k, crate::i18n::trv(l)).clicked() {
                            it.kind = k as u8;
                            changed = true;
                        }
                    }
                });
            });
        }
        // the file itself
        ui.horizontal_wrapped(|ui| {
            if !r.file.is_empty() {
                ui.label(RichText::new(&r.name).small().weak());
                let label = match r.kind {
                    2 | 7 => tr("Play"),
                    3 | 1 if is_pdf(&r.file) => tr("Read"),
                    _ => tr("Open"),
                };
                if ui.small_button(label).clicked() {
                    if is_pdf(&r.file) {
                        self.x.pdf_req = Some((mfile(&r.file), r.title.clone()));
                    } else if r.kind == 5 {
                        self.x.research_view_pic = Some(r.file.clone());
                    } else {
                        open_media(&r.file);
                    }
                }
                if ui.small_button(tr("Show in folder")).clicked() {
                    if let Some(p) = safe_media_path(&r.file).and_then(|p| p.parent().map(|d| d.to_path_buf())) {
                        open_path(&p);
                    }
                }
            }
        });
        // a scan or picture is shown right here
        if r.kind == 5 && !r.file.is_empty() {
            let p = mfile(&r.file);
            if p.is_file() {
                ui.add(egui::Image::new(img_uri(&p)).max_height(320.0).max_width(ui.available_width()).rounding(4.0));
            }
        }
        // an email, to read here
        if let Some(e) = &r.email {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::Grid::new("email_head").num_columns(2).spacing([10.0, 2.0]).show(ui, |ui| {
                    for (k, v) in [("From:", &e.from), ("To:", &e.to), ("Cc:", &e.cc), ("Date:", &e.date), ("Subject:", &e.subject)] {
                        if !v.is_empty() {
                            ui.label(RichText::new(crate::i18n::trv(k)).weak());
                            ui.label(v.as_str());
                            ui.end_row();
                        }
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().id_salt("email_body").max_height(320.0).show(ui, |ui| {
                    ui.add(egui::Label::new(e.body.as_str()).wrap().selectable(true));
                });
                if !e.attachments.is_empty() || !e.skipped.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(tr("Attachments")).strong());
                    for (f, n) in &e.attachments {
                        ui.horizontal(|ui| {
                            ui.label(n.as_str());
                            if ui.small_button(if is_pdf(f) { tr("Read") } else { tr("Open") }).clicked() {
                                if is_pdf(f) {
                                    self.x.pdf_req = Some((mfile(f), n.clone()));
                                } else {
                                    open_media(f);
                                }
                            }
                            if ui.small_button(tr("Add to Research")).on_hover_text(tr("Make this attachment an item of its own (a scan, a certificate...)")).clicked() {
                                let nid = self.db.research_next_id();
                                let stem = Path::new(n).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                self.db.research.push(ResearchItem { id: nid, file: f.clone(), name: n.clone(), kind: rkind_of(n), title: stem, date: r.date.clone(), added: today(), people: r.people.clone(), tags: r.tags.clone(), ..Default::default() });
                                self.dirty = true;
                                self.status = format!("Added {} to Research.", n);
                            }
                        });
                    }
                    for n in &e.skipped {
                        ui.label(RichText::new(crate::i18n::trf("{} (not kept: not a kind of file that can be opened safely)", &["", " (not kept: not a kind of file that can be opened safely)"], &[n.clone()])).small().weak());
                    }
                }
            });
        }
        ui.add_space(6.0);
        // who and what it is about
        ui.label(RichText::new(tr("About")).strong());
        let mut unlink: Option<u32> = None;
        ui.horizontal_wrapped(|ui| {
            for p in &r.people {
                let name = self.db.people.get(p).map(|x| x.name.clone()).unwrap_or_else(|| format!("#{p}"));
                if ui.link(&name).on_hover_text(tr("Open their page")).clicked() {
                    *act = Some(Action::Open(*p));
                }
                if ui.small_button("x").on_hover_text(tr("Unlink")).clicked() {
                    unlink = Some(*p);
                }
                ui.add_space(6.0);
            }
            if let Some(s) = sel.filter(|s| !r.people.contains(s)) {
                if ui.button(crate::i18n::trf("+ Link {}", &["+ Link ", ""], &[sel_name.clone()])).clicked() {
                    self.db.research[ix].people.push(s);
                    changed = true;
                }
            } else if r.people.is_empty() {
                ui.label(RichText::new(tr("Select a person (on any tab, or with Find) to link them.")).small().weak());
            }
        });
        if let Some(p) = unlink {
            let it = &mut self.db.research[ix];
            it.people.retain(|x| *x != p);
            it.facts.retain(|x| x.0 != p);
            changed = true;
        }
        // facts and families of the linked people
        if !r.people.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Facts:")).weak());
                let mut drop: Option<usize> = None;
                for (i, (p, f)) in r.facts.iter().enumerate() {
                    let who = self.db.people.get(p).map(|x| x.given.clone()).unwrap_or_default();
                    ui.label(format!("{who}: {}", crate::i18n::tr_dyn(f)));
                    if ui.small_button("x").clicked() {
                        drop = Some(i);
                    }
                }
                if let Some(i) = drop {
                    self.db.research[ix].facts.remove(i);
                    changed = true;
                }
                ui.menu_button(tr("+ Fact"), |ui| {
                    for p in &r.people {
                        let Some(pp) = self.db.people.get(p) else { continue };
                        let mut facts: Vec<String> = FACTS.iter().skip(1).map(|s| s.to_string()).collect();
                        for e in &pp.events {
                            if !facts.contains(&e.kind) {
                                facts.push(e.kind.clone());
                            }
                        }
                        ui.label(RichText::new(&pp.name).strong());
                        for f in facts {
                            if ui.button(crate::i18n::tr_dyn(&f)).clicked() {
                                let it = &mut self.db.research[ix];
                                if !it.facts.contains(&(*p, f.clone())) {
                                    it.facts.push((*p, f));
                                    changed = true;
                                }
                                ui.close_menu();
                            }
                        }
                    }
                });
                let fams: Vec<u32> = r.people.iter().flat_map(|p| self.db.fams_of(*p)).collect::<BTreeSet<_>>().into_iter().collect();
                if !fams.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(tr("Families:")).weak());
                    for f in fams {
                        let mut on = r.fams.contains(&f);
                        if ui.checkbox(&mut on, fam_label(&self.db, f)).changed() {
                            let it = &mut self.db.research[ix];
                            if on {
                                it.fams.push(f);
                            } else {
                                it.fams.retain(|x| *x != f);
                            }
                            changed = true;
                        }
                    }
                }
            });
        }
        // tags
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Tags:")).strong());
            let mut t = self.db.research[ix].tags.join(", ");
            if ui.add(egui::TextEdit::singleline(&mut t).hint_text(tr("census, letters, Harwood line")).desired_width(320.0)).changed() {
                self.db.research[ix].tags = t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                changed = true;
            }
        });
        // transcription and notes
        ui.label(RichText::new(tr("Transcription and notes")).strong());
        ui.label(RichText::new(tr("Type what a scan or letter says, so it can be searched.")).small().weak());
        changed |= ui.add(egui::TextEdit::multiline(&mut self.db.research[ix].text).desired_rows(6).desired_width(f32::INFINITY)).changed();
        // evidence for a source
        ui.add_space(6.0);
        ui.label(RichText::new(tr("Evidence")).strong());
        ui.horizontal_wrapped(|ui| {
            let cur = r.sid.and_then(|s| self.db.sources.get(&s)).map(|s| s.title.clone()).unwrap_or_else(|| tr("(no source)").to_string());
            ui.label(tr("Source:"));
            egui::ComboBox::from_id_salt("rsrc").selected_text(cur).width(260.0).show_ui(ui, |ui| {
                if ui.selectable_label(r.sid.is_none(), tr("(no source)")).clicked() {
                    self.db.research[ix].sid = None;
                    changed = true;
                }
                let list: Vec<(u32, String)> = self.db.sources.iter().map(|(k, s)| (*k, s.title.clone())).collect();
                for (k, t) in list {
                    if ui.selectable_label(r.sid == Some(k), t).clicked() {
                        self.db.research[ix].sid = Some(k);
                        changed = true;
                    }
                }
            });
            if ui.button(tr("New source from this")).on_hover_text(tr("Make a source with this item's title, and use it")).clicked() {
                let k = self.db.sources.keys().max().copied().unwrap_or(0) + 1;
                let title = if r.title.is_empty() { r.name.clone() } else { r.title.clone() };
                let author = r.email.as_ref().map(|e| e.from.clone()).unwrap_or_default();
                self.db.sources.insert(k, SourceRec { title, author, publ: r.date.clone(), kind: if r.kind == 1 { "Letter or diary".to_string() } else { String::new() }, ..Default::default() });
                self.db.research[ix].sid = Some(k);
                changed = true;
            }
        });
        let facts = self.db.research[ix].facts.clone();
        let mut log: Option<(String, u32)> = None;
        if r.sid.is_some() && !facts.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr("Cite it on:")).weak());
                for (p, f) in &facts {
                    let who = self.db.people.get(p).map(|x| x.given.clone()).unwrap_or_default();
                    let done = self.db.people.get(p).map_or(false, |pp| pp.sources.iter().any(|s| s.sid == r.sid && &s.about == f && s.media.iter().any(|m| m.file == r.file && !r.file.is_empty())));
                    if ui.add_enabled(!done, egui::Button::new(format!("{who}: {}", crate::i18n::tr_dyn(f)))).on_hover_text(if done { tr("Already cited there") } else { tr("Adds a citation to this fact, with the transcription as its text and the file attached") }).clicked() {
                        if let Some(pp) = self.db.people.get_mut(p) {
                            let media = if r.file.is_empty() { vec![] } else { vec![Media { file: r.file.clone(), name: r.name.clone(), caption: r.title.clone(), ..Default::default() }] };
                            pp.sources.push(Source { about: f.clone(), sid: r.sid, text: r.text.clone(), page: r.date.clone(), media, ..Default::default() });
                            self.status = format!("Cited on {}'s {}.", who, f);
                            changed = true;
                            log = Some((format!("Cited \"{}\" on {}'s {}", r.title, who, f), *p));
                        }
                    }
                }
            });
        } else if r.sid.is_some() {
            ui.label(RichText::new(tr("Add a fact under About to cite this item on it.")).small().weak());
        }
        if let Some((what, p)) = log {
            self.auto_log(&what, "Gilded Roots", Some(p), "");
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button(tr("Remove from Research...")).clicked() {
                self.x.research_confirm = Some(id);
            }
            ui.label(RichText::new(crate::i18n::trf("Added {}", &["Added ", ""], &[r.added.clone()])).small().weak());
        });
        if self.x.research_confirm == Some(id) {
            ui.horizontal(|ui| {
                ui.label(tr("Remove this item? The file stays in the media folder, and Undo brings the item back."));
                if ui.button(tr("Remove")).clicked() {
                    remove = true;
                }
                if ui.button(tr("Cancel")).clicked() {
                    self.x.research_confirm = None;
                }
            });
        }
        if remove {
            self.db.research.retain(|x| x.id != id);
            self.x.research_sel = None;
            self.x.research_confirm = None;
            changed = true;
        }
        if changed {
            self.dirty = true;
        }
    }

    /// A picture from Research, shown large in a window of its own.
    pub(crate) fn research_picture(&mut self, ctx: &egui::Context) {
        let Some(f) = self.x.research_view_pic.clone() else { return };
        let mut open = true;
        egui::Window::new(crate::look::wtitle(crate::i18n::tr("Scan"))).id(egui::Id::new("research_pic")).open(&mut open).default_size([800.0, 700.0]).show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(egui::Image::new(img_uri(&mfile(&f))).max_width(ui.available_width()));
            });
        });
        if !open {
            self.x.research_view_pic = None;
        }
    }

    /// The research files about a person, for their Individual page.
    pub(crate) fn person_research(&mut self, ui: &mut egui::Ui, id: u32) {
        let items: Vec<(u32, String, String, u8)> = self.db.research_for(id).iter().map(|r| (r.id, r.title.clone(), r.date.clone(), r.kind)).collect();
        if items.is_empty() {
            return;
        }
        ui.add_space(6.0);
        ui.label(RichText::new(crate::i18n::trf("Research files ({})", &["Research files (", ")"], &[items.len().to_string()])).strong());
        for (rid, t, d, k) in items {
            ui.horizontal(|ui| {
                fixed_label(ui, 70.0, RichText::new(crate::i18n::trv(RKINDS.get(k as usize).copied().unwrap_or("Other"))).small().color(ui.visuals().hyperlink_color));
                if ui.link(format!("{t}  {d}")).on_hover_text(crate::i18n::tr("Open in the Research tab")).clicked() {
                    self.x.research_sel = Some(rid);
                    self.x.research_tab = 0;
                    self.view = View::Research;
                }
            });
        }
    }

    /// Notes what was done in the research journal, when the automatic log is on.
    pub(crate) fn log_research(&mut self, what: &str, who: Option<u32>) {
        self.auto_log(what, "Gilded Roots", who, "");
    }

    /// Writes an entry in the research journal by itself (when that is turned on in Settings):
    /// what was done, where, about whom. Entries written this way say "(noted by Gilded Roots)".
    pub(crate) fn auto_log(&mut self, what: &str, place: &str, who: Option<u32>, notes: &str) {
        if !self.settings.auto_log {
            return;
        }
        let notes = if notes.is_empty() { "(noted by Gilded Roots)".to_string() } else { format!("{notes}\n(noted by Gilded Roots)") };
        self.db.journal.push(Journal { date: today(), goal: what.to_string(), place: place.to_string(), result: String::new(), people: who.into_iter().collect(), notes, ..Default::default() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EML: &str = "From: =?UTF-8?Q?Ren=C3=A9e_Harwood?= <renee@example.com>\r\nTo: Doyle <d@example.com>\r\nSubject: =?utf-8?B?R3JhbmRwYSdzIGxldHRlcnMg4pyJ?=\r\nDate: Tue, 3 Mar 2015 10:12:00 -0800\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"XYZ\"\r\n\r\n--XYZ\r\nContent-Type: multipart/alternative; boundary=\"ALT\"\r\n\r\n--ALT\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nHere are the letters from Caf=C3=A9 Street.=\r\n See you soon.\r\n--ALT\r\nContent-Type: text/html\r\n\r\n<p>Here</p>\r\n--ALT--\r\n--XYZ\r\nContent-Type: application/pdf; name=\"census.pdf\"\r\nContent-Disposition: attachment; filename=\"census.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--XYZ\r\nContent-Type: application/x-msdownload; name=\"virus.exe\"\r\nContent-Transfer-Encoding: base64\r\n\r\nTVo=\r\n--XYZ--\r\n";

    #[test]
    fn emails_are_read() {
        let e = parse_eml(EML.as_bytes());
        assert_eq!(e.from, "Renée Harwood <renee@example.com>");
        assert_eq!(e.subject, "Grandpa's letters ✉");
        assert_eq!(e.body, "Here are the letters from Café Street. See you soon.");
        assert_eq!(email_day(&e.date), "3 Mar 2015");
        assert_eq!(e.attachments.len(), 2);
        assert_eq!(e.attachments[0].0, "census.pdf");
        assert_eq!(e.attachments[0].1, b"%PDF-1.4\n");
    }

    #[test]
    fn html_only_emails_and_old_character_sets() {
        let m = "Subject: =?iso-8859-1?Q?M=FCller?=\nContent-Type: text/html; charset=iso-8859-1\n\n<html><head><style>p{}</style></head><body><p>Gr\u{00fc}\u{00df}e &amp; more</p><br>Line 2</body></html>";
        let e = parse_eml(m.as_bytes());
        assert_eq!(e.subject, "Müller");
        assert!(e.body.contains("& more") && e.body.contains("Line 2") && !e.body.contains("p{}"), "{}", e.body);
    }

    #[test]
    fn mailboxes_split_into_messages() {
        let mbox = "From a@b Tue Mar  3 10:00:00 2015\nSubject: One\n\nfirst\n>From the old days\n\nFrom c@d Wed Mar  4 10:00:00 2015\nSubject: Two\n\nsecond\n";
        let m = split_mbox(mbox.as_bytes());
        assert_eq!(m.len(), 2);
        let one = parse_eml(&m[0]);
        assert_eq!(one.subject, "One");
        assert!(one.body.contains("From the old days"), "escaped From line restored");
        assert_eq!(parse_eml(&m[1]).subject, "Two");
    }

    #[test]
    fn kinds_from_names() {
        assert_eq!(rkind_of("a.eml"), 1);
        assert_eq!(rkind_of("talk.m4a"), 2);
        assert_eq!(rkind_of("x.PDF"), 3);
        assert_eq!(rkind_of("notes.docx"), 4);
        assert_eq!(rkind_of("scan.tif"), 5);
        assert_eq!(rkind_of("page.html"), 6);
    }

    #[test]
    fn research_is_saved_with_the_tree_and_found_by_person() {
        let mut db = crate::sample::sample_family();
        let (p, _) = db.people.iter().next().map(|(k, v)| (*k, v.clone())).unwrap();
        db.research.push(ResearchItem { id: 1, title: "Letter".into(), people: vec![p], tags: vec!["letters".into()], ..Default::default() });
        let j = serde_json::to_string(&db).unwrap();
        let back: Db = serde_json::from_str(&j).unwrap();
        assert_eq!(back.research, db.research);
        assert_eq!(db.research_for(p).len(), 1);
        assert_eq!(db.research_tags(), vec!["letters".to_string()]);
        assert!(matches(&db, &db.research[0], "letter"));
        assert!(!matches(&db, &db.research[0], "census"));
    }

    #[test]
    fn the_research_tab_draws_an_email_and_a_scan() {
        let mut db = crate::sample::sample_family();
        let p = *db.people.keys().next().unwrap();
        let e = parse_eml(EML.as_bytes());
        db.research.push(ResearchItem { id: 1, kind: 1, title: e.subject.clone(), people: vec![p], facts: vec![(p, "Birth".into())], sid: db.sources.keys().next().copied(), email: Some(EmailInfo { from: e.from, body: e.body, skipped: vec!["virus.exe".into()], ..Default::default() }), ..Default::default() });
        db.research.push(ResearchItem { id: 2, kind: 5, title: "Scan".into(), file: "missing.jpg".into(), ..Default::default() });
        let mut app = App::for_test(db);
        app.sel = Some(p);
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        for sel in [Some(1), Some(2), None] {
            app.x.research_sel = sel;
            app.x.research_q = if sel == Some(2) { "scan".into() } else { String::new() };
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                let mut act = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.research_view(ui, &mut act);
                    app.person_research(ui, p);
                });
                app.research_picture(ctx);
            });
        }
        assert_eq!(app.db.research.len(), 2);
    }
}
