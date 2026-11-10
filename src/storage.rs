//! Where files live, reading and writing the database, backups and GEDCOM files.

use super::*;

pub(crate) fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf();
    if exe.to_string_lossy().contains(".app/Contents/MacOS") {
        for _ in 0..3 {
            dir = dir.parent()?.to_path_buf();
        }
    }
    Some(dir.join("FamilyTreeData"))
}

/// macOS runs a downloaded, unsigned app from a hidden temporary copy ("App Translocation"),
/// away from the FamilyTreeData folder beside it. Spot that, to explain the one-time fix.
pub(crate) fn mac_translocated() -> bool {
    cfg!(target_os = "macos") && std::env::current_exe().map(|e| e.to_string_lossy().contains("/AppTranslocation/")).unwrap_or(false)
}

pub(crate) fn is_portable() -> bool {
    portable_dir().map(|p| p.is_dir()).unwrap_or(false)
}

/// The hidden per-user folder: settings, and the pointer to a custom data folder.
pub(crate) fn config_dir() -> PathBuf {
    if let Some(p) = portable_dir() {
        if p.is_dir() {
            return p;
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let base = (if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library").join("Application Support"))
    } else {
        home.map(|h| h.join(".local").join("share"))
    })
    .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("FamilyTree");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Is this folder probably kept in sync by iCloud, OneDrive or Dropbox? (A best guess from the path.)
pub(crate) fn synced_service(dir: &std::path::Path) -> Option<&'static str> {
    let canon = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let s = format!("{} {}", dir.to_string_lossy(), canon.to_string_lossy()).to_lowercase();
    if s.contains("mobile documents") || s.contains("clouddocs") || s.contains("icloud") {
        return Some("iCloud Drive");
    }
    if s.contains("onedrive") {
        return Some("OneDrive");
    }
    if s.contains("dropbox") {
        return Some("Dropbox");
    }
    if cfg!(target_os = "macos") {
        // With "Desktop & Documents Folders" turned on, iCloud keeps a Documents folder of its own.
        if let Some(h) = std::env::var_os("HOME").map(PathBuf::from) {
            let icd = h.join("Library").join("Mobile Documents").join("com~apple~CloudDocs").join("Documents");
            if icd.is_dir() && dir.starts_with(h.join("Documents")) {
                return Some("iCloud Drive");
            }
        }
    }
    None
}

pub(crate) fn documents_dir() -> PathBuf {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let docs = home.join("Documents");
    if docs.is_dir() { docs } else { home }
}

/// Count files under a folder (used to check a copy is complete).
pub(crate) fn count_files(d: &std::path::Path) -> usize {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.flatten() {
            let p = e.path();
            n += if p.is_dir() { count_files(&p) } else { 1 };
        }
    }
    n
}

/// One-time move of the tree from the old hidden folder to the visible one. The old copy is left in
/// place (marked) until the new copy has been checked, and is never deleted by the app.
pub(crate) fn migrate_old_data(old: &std::path::Path, new: &std::path::Path) -> bool {
    if new.join("tree.json").exists() || !old.join("tree.json").exists() || old.join("MOVED.txt").exists() || old == new {
        return true;
    }
    let existed = new.exists();
    let copy = (|| -> std::io::Result<bool> {
        std::fs::create_dir_all(new)?;
        for e in std::fs::read_dir(old)? {
            let e = e?;
            let name = e.file_name();
            if name == "settings.json" || name == "location.txt" || name == "MOVED.txt" {
                continue;
            }
            let (p, t) = (e.path(), new.join(&name));
            if p.is_dir() {
                copy_dir(&p, &t)?;
            } else {
                std::fs::copy(&p, &t)?;
            }
        }
        let same = std::fs::read(old.join("tree.json"))? == std::fs::read(new.join("tree.json"))?;
        let files = |d: &std::path::Path| count_files(&d.join("media")) + count_files(&d.join("backups"));
        Ok(same && files(old) == files(new))
    })();
    match copy {
        Ok(true) => {
            let _ = std::fs::write(
                old.join("MOVED.txt"),
                format!("Your family tree now lives in:\n{}\nThis old copy is no longer used. You can delete this folder once you are happy with the new one.\n", new.display()),
            );
            true
        }
        _ => {
            if !existed {
                let _ = std::fs::remove_dir_all(new);
            } else {
                let _ = std::fs::remove_file(new.join("tree.json"));
            }
            false
        }
    }
}

/// The tree being worked on when it is not the main one (File > Family trees).
pub(crate) static CURRENT_TREE: Mutex<Option<String>> = Mutex::new(None);

/// The folder of the tree that is open: the main folder, or a folder under "More trees".
pub(crate) fn data_dir() -> PathBuf {
    let base = base_dir();
    match CURRENT_TREE.lock().ok().and_then(|g| g.clone()) {
        Some(name) if !name.is_empty() => base.join("More trees").join(name),
        _ => base,
    }
}

pub(crate) fn tree_title(name: &str) -> String {
    if name.is_empty() { "My family tree".to_string() } else { name.to_string() }
}

/// A tree name that is safe as a folder name.
pub(crate) fn sanitize_tree_name(s: &str) -> String {
    let t: String = s.trim().chars().filter(|c| c.is_alphanumeric() || " -_'&().".contains(*c)).collect();
    let t = t.trim().trim_matches('.').to_string();
    if t.len() > 60 { t.chars().take(60).collect() } else { t }
}

/// Names of the extra trees (folders under "More trees" that hold a tree).
pub(crate) fn other_trees() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(base_dir().join("More trees"))
        .map(|r| r.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    v.sort_by_key(|n| n.to_lowercase());
    v
}

/// The main folder (Documents/Gilded Roots, the portable folder, or the one chosen in settings).
pub(crate) fn base_dir() -> PathBuf {
    if let Some(p) = portable_dir() {
        if p.is_dir() {
            return p;
        }
    }
    let cfg = config_dir();
    if let Ok(s) = std::fs::read_to_string(cfg.join("location.txt")) {
        let p = PathBuf::from(s.trim());
        if p.is_dir() || std::fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    let dir = documents_dir().join("Gilded Roots");
    if !migrate_old_data(&cfg, &dir) {
        // Could not move safely: keep using the old folder rather than risk the data.
        return cfg;
    }
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub(crate) fn store_path() -> PathBuf {
    data_dir().join("tree.json")
}

pub(crate) fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let p = entry.path();
        let t = dst.join(entry.file_name());
        if p.is_dir() {
            copy_dir(&p, &t)?;
        } else {
            std::fs::copy(&p, &t)?;
        }
    }
    Ok(())
}

pub(crate) fn stamp() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    format!("{}-{}", ms, N.fetch_add(1, Ordering::Relaxed))
}

pub(crate) fn media_dir() -> PathBuf {
    let d = store_path().with_file_name("media");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub(crate) fn ext_of(f: &str) -> String {
    f.rsplit('.').next().unwrap_or("").to_lowercase()
}

pub(crate) fn is_image(f: &str) -> bool {
    matches!(ext_of(f).as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp")
}

/// File types the program keeps as photos, documents and recordings.
pub(crate) fn is_media_name(f: &str) -> bool {
    matches!(
        ext_of(f).as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "heif" | "tif" | "tiff" | "pdf" | "doc" | "docx" | "rtf" | "txt" | "odt" | "htm" | "html" | "wav" | "mp3" | "m4a" | "ogg" | "flac" | "aac" | "mp4" | "mov" | "avi"
    )
}

/// A stored media file name that stays inside the media folder (no folders, no "..").
pub(crate) fn safe_media_path(file: &str) -> Option<PathBuf> {
    if file.is_empty() || file.contains('/') || file.contains('\\') || file.contains("..") || file.contains(':') {
        return None;
    }
    Some(media_dir().join(file))
}

/// The full path of a stored media file, kept inside the media folder whatever the stored name
/// says (a tree from someone else could hold a name like "C:\\Windows\\..." or "../../x").
/// An unsafe name gives a path that does not exist, so nothing outside the folder is ever read.
pub(crate) fn mfile(file: &str) -> PathBuf {
    safe_media_path(file).unwrap_or_else(|| media_dir().join("previews").join("-missing-"))
}

/// The same for a preview picture (thumbnail or mid-size) in the previews folder.
pub(crate) fn mprev(file: &str) -> PathBuf {
    match safe_media_path(file) {
        Some(_) => media_dir().join("previews").join(file),
        None => media_dir().join("previews").join("-missing-"),
    }
}

/// Open a stored media file in the computer's own app, but only a media file inside the media
/// folder (never a program or a file somewhere else on the computer).
pub(crate) fn open_media(file: &str) {
    if is_media_name(file) {
        if let Some(p) = safe_media_path(file) {
            if p.is_file() {
                open_path(&p);
            }
        }
    }
}

pub(crate) fn is_pdf(f: &str) -> bool {
    ext_of(f) == "pdf"
}

pub(crate) fn is_audio(f: &str) -> bool {
    matches!(ext_of(f).as_str(), "wav" | "mp3" | "m4a" | "ogg" | "flac" | "aac")
}

pub(crate) fn sanitize(n: &str) -> String {
    n.chars().map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' }).collect()
}

pub(crate) fn import_one(p: &std::path::Path, name: &str) -> Option<(String, String)> {
    let ext = ext_of(name);
    if cfg!(target_os = "macos") && (ext == "heic" || ext == "heif") {
        let stem = std::path::Path::new(name).file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_else(|| "photo".to_string());
        let file = format!("{}-{}.jpg", stamp(), sanitize(&stem));
        let ok = std::process::Command::new("sips")
            .args(["-s", "format", "jpeg"])
            .arg(p)
            .arg("--out")
            .arg(media_dir().join(&file))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some((file, format!("{}.jpg", stem)));
        }
    }
    let file = format!("{}-{}", stamp(), sanitize(name));
    std::fs::copy(p, media_dir().join(&file)).ok()?;
    Some((file, name.to_string()))
}

/// Where file pickers start: the folder last used, otherwise the person's Documents or home folder
/// (never the app's own hidden data folder).
pub(crate) static LAST_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

pub(crate) fn start_dir() -> PathBuf {
    if let Some(d) = LAST_DIR.lock().ok().and_then(|g| g.clone()) {
        if d.is_dir() {
            return d;
        }
    }
    documents_dir()
}

pub(crate) fn remember_dir(file: &Path) {
    if let (Some(d), Ok(mut g)) = (file.parent(), LAST_DIR.lock()) {
        *g = Some(d.to_path_buf());
    }
}

/// Read a GEDCOM file: decode it, parse it, copy in the photos it refers to, and check what an export would lose.
pub(crate) fn read_ged_file(path: &Path) -> Result<(Db, ImportReport), String> {
    let b = std::fs::read(path).map_err(|e| format!("Could not read file: {}", e))?;
    if b.is_empty() {
        return Err("That file is empty.".to_string());
    }
    // One of this app's own tree files (tree.json or a backup)?
    let trimmed = b.iter().skip_while(|c| c.is_ascii_whitespace() || **c == 0xEF || **c == 0xBB || **c == 0xBF).next();
    if trimmed == Some(&b'{') {
        return match serde_json::from_slice::<Db>(&b) {
            Ok(mut db) if !db.people.is_empty() => {
                migrate(&mut db);
                let rep = ImportReport {
                    file: path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default(),
                    program: "a saved tree file (JSON)".to_string(),
                    charset: "UTF-8".to_string(),
                    people: db.people.len(),
                    families: db.fams.len(),
                    sources: db.sources.len(),
                    notes: db.notes.len(),
                    ..Default::default()
                };
                Ok((db, rep))
            }
            Ok(_) => Err("That JSON file has no people in it.".to_string()),
            Err(_) => Err("That is a JSON file, but not a family tree saved by this app, so it can't be read. If it came from another program, export a GEDCOM (.ged) file from that program instead.".to_string()),
        };
    }
    // A PAF 5 database: read it directly.
    if paf::is_paf(&b) {
        let (text, notes) = paf::to_gedcom(&b)?;
        let (db, mut rep) = parse_ged_report(&text);
        if db.people.is_empty() {
            return Err("This PAF file has no people in it.".to_string());
        }
        rep.file = path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        rep.program = "PAF 5 database (.paf), read directly".to_string();
        rep.charset = "ANSI".to_string();
        if notes.odd_dates > 0 {
            rep.warnings.push(format!("{} dates were stored in a way this reader doesn't fully understand yet. They came in as full dates; please check them against PAF.", notes.odd_dates));
        }
        if notes.unread > 0 {
            rep.warnings.push(format!("{} people or families have extra details (such as christenings, burials, LDS ordinances, notes or sources) this reader can't read from a .paf file yet. To bring those in, export a GEDCOM from PAF and import that instead.", notes.unread));
        }
        if notes.odd_links > 0 {
            rep.warnings.push(format!("{} children are linked to parents as something other than a birth child (adopted, sealed, guardian and so on). They came in as ordinary children; please check those links.", notes.odd_links));
        }
        rep.lost = loss_check(&text, &to_ged(&db));
        return Ok((db, rep));
    }
    // A program's own data file (PAF's .paf, for example) rather than a GEDCOM export?
    let nuls = b.iter().take(4096).filter(|c| **c == 0).count();
    let head: String = b.iter().take(4000).map(|&c| c as char).collect();
    let looks_text = nuls < 20 && (head.contains("0 HEAD") || head.contains("\n0 @") || head.contains("\r0 @") || head.starts_with("0 "));
    let is_utf16 = b.starts_with(&[0xFF, 0xFE]) || b.starts_with(&[0xFE, 0xFF]);
    if !looks_text && !is_utf16 && nuls >= 20 {
        return Err("This is a program's own data file, not a GEDCOM file, so it can't be read directly. Open it in the program that made it (for PAF: File > Export, choose GEDCOM), save the .ged file, then import that here.".to_string());
    }
    let (text, charset) = decode_ged(&b);
    let (mut db, mut rep) = parse_ged_report(&text);
    if db.people.is_empty() {
        return Err("This file is text, but it has no people in a GEDCOM layout. Make sure it is the GEDCOM (.ged) file the other program exported, not a report or a different kind of export.".to_string());
    }
    rep.file = path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
    rep.charset = charset;
    resolve_media(&mut db, path.parent().unwrap_or(Path::new(".")), &mut rep);
    rep.lost = loss_check(&text, &to_ged(&db));
    Ok((db, rep))
}

/// Write the tree as a GEDCOM file. Photos and documents are copied into a folder next to it and the file
/// points to them. The file is read back and compared with the tree, so a problem is never silent.
/// Ok and Err both carry the message to show.
pub(crate) fn write_gedcom(db: &Db, path: &Path, private: bool) -> Result<String, String> {
    let out = if private { db.sanitized() } else { db.clone() };
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "gilded-roots".to_string());
    let has_media = out.people.values().any(|p| p.media.iter().any(|m| !m.file.is_empty())) || out.fams.values().any(|f| f.media.iter().any(|m| !m.file.is_empty()));
    let folder = format!("{}-media", sanitize(&stem));
    let ex = to_ged_full(&out, &ExportOpts { prune: private, media_dir: if has_media { Some(folder.clone()) } else { None } });
    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let (mut copied, mut missing) = (0usize, 0usize);
    let mut failed: Option<String> = None;
    if has_media {
        let _ = std::fs::create_dir_all(dir.join(&folder));
        for (file, rel) in &ex.media {
            let src = mfile(file);
            let dst = dir.join(rel);
            if !src.exists() {
                missing += 1;
            } else if let Err(e) = std::fs::copy(&src, &dst) {
                failed = Some(io_msg(&e));
            } else {
                copied += 1;
            }
        }
    }
    atomic_write(path, ex.text.as_bytes()).map_err(|e| format!("Could not save: {}", e))?;
    let mut s = format!("Saved {}", path.display());
    let mut problem = false;
    if has_media {
        s.push_str(&format!(" with {} photo/document file(s) in the folder {}", copied, folder));
        if missing > 0 {
            s.push_str(&format!(" ({} file(s) were missing and could not be copied)", missing));
            problem = true;
        }
        if let Some(e) = failed {
            s.push_str(&format!(" (some files could not be copied: {})", e));
            problem = true;
        }
    }
    match std::fs::read(path) {
        Ok(b) => {
            let (back, _) = decode_ged(&b);
            let (db2, _) = parse_ged_report(&back);
            let lost = loss_check(&to_ged_full(&out, &ExportOpts { prune: private, media_dir: None }).text, &back);
            if db2.people.len() != out.people.len() || db2.fams.len() != out.families_count() || !lost.is_empty() {
                s.push_str(&format!(". WARNING: reading the saved file back found {} people and {} families (your tree has {} and {}){}.", db2.people.len(), db2.fams.len(), out.people.len(), out.families_count(), if lost.is_empty() { String::new() } else { format!(", and {} piece(s) differ", lost.len()) }));
                problem = true;
            }
        }
        Err(e) => {
            s.push_str(&format!(". WARNING: could not read the file back to check it: {}", e));
            problem = true;
        }
    }
    if problem { Err(s) } else { Ok(s) }
}

/// Photos and documents that a GEDCOM file refers to: copy in the ones that can be found
/// (next to the file, or at the path it gives) and keep the reference for the rest.
pub(crate) fn resolve_media(db: &mut Db, base: &Path, rep: &mut ImportReport) {
    let mut fix = |m: &mut Media| {
        if !m.file.is_empty() || m.path.is_empty() {
            return;
        }
        if m.path.starts_with("http://") || m.path.starts_with("https://") {
            rep.media_missing += 1;
            return;
        }
        let norm = m.path.replace('\\', "/");
        let base_name = norm.rsplit('/').next().unwrap_or("").to_string();
        // A GEDCOM file can name any file on the computer. Only photos, documents and
        // recordings are copied in, so a shared file cannot pull in other private files.
        if !is_media_name(&base_name) {
            rep.media_missing += 1;
            return;
        }
        let candidates = [PathBuf::from(&m.path), base.join(&norm), base.join(&base_name)];
        for c in candidates.iter() {
            if c.is_file() {
                if let Some((file, shown)) = import_one(c, &base_name) {
                    m.file = file;
                    m.name = shown;
                    m.path.clear();
                    rep.media_found += 1;
                    return;
                }
            }
        }
        rep.media_missing += 1;
    };
    for p in db.people.values_mut() {
        for m in p.media.iter_mut() {
            fix(m);
        }
    }
    for f in db.fams.values_mut() {
        for m in f.media.iter_mut() {
            fix(m);
        }
    }
    let online = db.people.values().flat_map(|p| p.media.iter()).chain(db.fams.values().flat_map(|f| f.media.iter())).filter(|m| is_url(&m.path)).count();
    if online > 0 {
        rep.warnings.push(format!(
            "{} photos/documents are links to a website (for example Ancestry), not files. The links are kept and can be opened in your browser, but the pictures themselves are not in the GEDCOM file. To bring them in, download them and add them with Add photos or documents.",
            online
        ));
    }
}

pub(crate) fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let z = secs / 86400 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/* ---------- safe storage: atomic saves, verified backups, recovery ---------- */

pub(crate) fn fnv64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for x in b {
        h ^= *x as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub(crate) fn is_disk_full(e: &std::io::Error) -> bool {
    if cfg!(windows) {
        matches!(e.raw_os_error(), Some(112) | Some(39))
    } else {
        e.raw_os_error() == Some(28)
    }
}

pub(crate) fn io_msg(e: &std::io::Error) -> String {
    if is_disk_full(e) {
        "the disk is full".to_string()
    } else if e.kind() == std::io::ErrorKind::PermissionDenied {
        "permission denied (the data folder may be read-only or locked by another program)".to_string()
    } else {
        e.to_string()
    }
}

pub(crate) fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut t = p.as_os_str().to_owned();
    t.push(suffix);
    PathBuf::from(t)
}

/// Writes `data` to a temporary file next to `path`, flushes it to disk, re-reads it to
/// confirm it is complete, and only then swaps it into place. The old file is never
/// touched unless the new one is fully written and verified.
pub(crate) fn atomic_write_with(path: &Path, data: &[u8], writer: &dyn Fn(&mut std::fs::File, &[u8]) -> std::io::Result<()>) -> std::io::Result<()> {
    let tmp = with_suffix(path, ".tmp");
    let written = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        writer(&mut f, data)?;
        f.flush()?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    match std::fs::read(&tmp) {
        Ok(back) if back.len() == data.len() && fnv64(&back) == fnv64(data) => {}
        Ok(_) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "the file on disk did not match what was written"));
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if cfg!(unix) {
        if let Some(dir) = path.parent() {
            if let Ok(d) = std::fs::File::open(dir) {
                let _ = d.sync_all();
            }
        }
    }
    Ok(())
}

pub(crate) fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    atomic_write_with(path, data, &|f: &mut std::fs::File, d: &[u8]| f.write_all(d))
}

pub(crate) fn sidecar(p: &Path) -> PathBuf {
    with_suffix(p, ".fnv")
}

pub(crate) fn read_db(path: &Path) -> Result<Db, String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    if b.is_empty() {
        return Err("the file is empty".to_string());
    }
    let mut db = serde_json::from_slice::<Db>(&b).map_err(|e| format!("not valid tree data: {}", e))?;
    migrate(&mut db);
    Ok(db)
}

/// Saves the tree. Keeps the previous good version as tree.json.prev.
pub(crate) fn save_db_to(dir: &Path, db: &Db) -> Result<usize, String> {
    let json = serde_json::to_vec(db).map_err(|e| e.to_string())?;
    serde_json::from_slice::<Db>(&json).map_err(|e| format!("internal check failed: {}", e))?;
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join("tree.json");
    if let Ok(old) = std::fs::read(&path) {
        if serde_json::from_slice::<Db>(&old).is_ok() {
            let _ = atomic_write(&dir.join("tree.json.prev"), &old);
        }
    }
    atomic_write(&path, &json).map_err(|e| io_msg(&e))?;
    Ok(json.len())
}

pub(crate) fn list_backups(bdir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(bdir)
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("tree-") && n.ends_with(".json")).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Re-reads a backup, compares its checksum (when one was stored) and parses it.
pub(crate) fn verify_backup(p: &Path) -> Result<usize, String> {
    let b = std::fs::read(p).map_err(|e| e.to_string())?;
    if let Ok(want) = std::fs::read_to_string(sidecar(p)) {
        let got = format!("{:016x}", fnv64(&b));
        if want.trim() != got {
            return Err("checksum does not match (the file changed or is damaged)".to_string());
        }
    }
    if b.is_empty() {
        return Err("the file is empty".to_string());
    }
    serde_json::from_slice::<Db>(&b).map(|d| d.people.len()).map_err(|e| format!("not valid tree data: {}", e))
}

pub(crate) fn write_backup_in(bdir: &Path, name: &str, json: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(bdir).map_err(|e| io_msg(&e))?;
    let p = bdir.join(name);
    atomic_write(&p, json).map_err(|e| io_msg(&e))?;
    atomic_write(&sidecar(&p), format!("{:016x}\n", fnv64(json)).as_bytes()).map_err(|e| io_msg(&e))?;
    verify_backup(&p).map(|_| ())
}

pub(crate) static BACKUP_HOURLY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// How many backups to keep (Tools > Backup settings).
pub(crate) static BACKUP_KEEP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(14);

pub(crate) fn prune_backups(bdir: &Path) {
    let mut names = list_backups(bdir);
    let keep = BACKUP_KEEP.load(Ordering::Relaxed).clamp(3, 365);
    while names.len() > keep {
        let old = names.remove(0);
        let p = bdir.join(old);
        let _ = std::fs::remove_file(sidecar(&p));
        let _ = std::fs::remove_file(&p);
    }
}

/// One verified backup per day (or on demand with force). Returns Ok(true) if a new one was written.
pub(crate) fn dated_backup_in(bdir: &Path, db: &Db, force: bool) -> Result<bool, String> {
    if db.people.is_empty() {
        return Ok(false);
    }
    let name = if BACKUP_HOURLY.load(Ordering::Relaxed) {
        let h = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| (d.as_secs() / 3600) % 24).unwrap_or(0);
        format!("tree-{}-{:02}h.json", today(), h)
    } else {
        format!("tree-{}.json", today())
    };
    let p = bdir.join(&name);
    if !force && p.exists() && verify_backup(&p).is_ok() {
        prune_backups(bdir);
        return Ok(false);
    }
    let json = serde_json::to_vec(db).map_err(|e| e.to_string())?;
    write_backup_in(bdir, &name, &json)?;
    prune_backups(bdir);
    Ok(true)
}

pub(crate) fn dated_backup(db: &Db) -> Result<bool, String> {
    dated_backup_in(&data_dir().join("backups"), db, false)
}

pub(crate) fn verify_all_backups() -> Vec<(String, Result<usize, String>)> {
    let bdir = data_dir().join("backups");
    list_backups(&bdir).into_iter().rev().map(|n| {
        let r = verify_backup(&bdir.join(&n));
        (n, r)
    }).collect()
}

/// Loads the tree. If the main file is damaged it is set aside (never overwritten) and the
/// newest readable copy is used instead: interrupted save, previous version, dated backups.
pub(crate) fn load_from(dir: &Path) -> (Db, String) {
    let main = dir.join("tree.json");
    let tmp = with_suffix(&main, ".tmp");
    let prev = dir.join("tree.json.prev");
    let bdir = dir.join("backups");
    let mut why = String::new();
    let mut kept = String::new();
    if main.exists() {
        match read_db(&main) {
            Ok(db) => {
                let _ = std::fs::remove_file(&tmp);
                return (db, String::new());
            }
            Err(e) => {
                why = e;
                let dest = dir.join(format!("tree.json.damaged-{}", stamp()));
                if std::fs::rename(&main, &dest).is_ok() {
                    kept = dest.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                }
            }
        }
    }
    let mut cands: Vec<PathBuf> = vec![tmp.clone(), prev];
    for n in list_backups(&bdir).into_iter().rev() {
        cands.push(bdir.join(n));
    }
    cands.push(dir.join("tree-backup.json"));
    for c in cands {
        if !c.exists() || verify_backup(&c).is_err() {
            continue;
        }
        if let Ok(db) = read_db(&c) {
            if db.people.is_empty() {
                continue;
            }
            let name = c.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let _ = std::fs::remove_file(&tmp);
            let msg = if why.is_empty() {
                format!("Your main data file was missing. Your tree was restored from {}.", name)
            } else {
                format!("Your data file was damaged ({}). Your tree was recovered from {}. The damaged file was kept as {}. Please check your most recent changes.", why, name, kept)
            };
            return (db, msg);
        }
    }
    if why.is_empty() {
        (Db::default(), String::new())
    } else {
        (Db::default(), format!("Your data file was damaged ({}) and no backup could be read. The damaged file was kept as {} in your data folder. Please do not delete it.", why, kept))
    }
}

/// Copies photos and recordings between the live media folder and the backup copy.
/// Never deletes. With only_missing it will not overwrite files that already exist.
/// With dry it only counts what would be copied. Returns (copied, failed, first error).
pub(crate) fn mirror_dir(src: &Path, dst: &Path, only_missing: bool, dry: bool) -> (usize, usize, Option<String>) {
    fn walk(src: &Path, dst: &Path, only_missing: bool, dry: bool, copied: &mut usize, failed: &mut usize, first: &mut Option<String>) {
        let Ok(rd) = std::fs::read_dir(src) else { return };
        if !dry {
            let _ = std::fs::create_dir_all(dst);
        }
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            let t = dst.join(e.file_name());
            if p.is_dir() {
                if e.file_name() == "previews" {
                    continue;
                }
                walk(&p, &t, only_missing, dry, copied, failed, first);
            } else {
                let need = match (std::fs::metadata(&p), std::fs::metadata(&t)) {
                    (Ok(_), Err(_)) => true,
                    (Ok(a), Ok(b)) => !only_missing && a.len() != b.len(),
                    _ => false,
                };
                if !need {
                    continue;
                }
                if dry {
                    *copied += 1;
                    continue;
                }
                let part = with_suffix(&t, ".part");
                match std::fs::copy(&p, &part).and_then(|_| std::fs::rename(&part, &t)) {
                    Ok(_) => *copied += 1,
                    Err(err) => {
                        *failed += 1;
                        if first.is_none() {
                            *first = Some(io_msg(&err));
                        }
                        let _ = std::fs::remove_file(&part);
                    }
                }
            }
        }
    }
    let (mut copied, mut failed, mut first): (usize, usize, Option<String>) = (0, 0, None);
    walk(src, dst, only_missing, dry, &mut copied, &mut failed, &mut first);
    (copied, failed, first)
}

pub(crate) fn ago(d: std::time::Duration) -> String {
    let s = d.as_secs();
    if s < 5 {
        "just now".to_string()
    } else if s < 60 {
        format!("{} s ago", s)
    } else if s < 3600 {
        format!("{} min ago", s / 60)
    } else {
        format!("{} h ago", s / 3600)
    }
}

/// A picture address for the program's own image loader. Unlike `file_uri` (used in web pages),
/// spaces are left as they are: the loader reads the path literally, so "%20" would break
/// folders like "Gilded Roots".
pub(crate) fn img_uri(p: &std::path::Path) -> String {
    format!("file://{}", p.display())
}

pub(crate) fn file_uri(p: &std::path::Path) -> String {
    let s = p.display().to_string().replace('\\', "/").replace(' ', "%20");
    if s.starts_with('/') { format!("file://{}", s) } else { format!("file:///{}", s) }
}

#[cfg(windows)]
pub(crate) fn open_path(p: &std::path::Path) {
    // Explorer opens the file with its usual program. Unlike "cmd /C start", it does not treat
    // characters such as & or | in a file name as commands.
    let _ = std::process::Command::new("explorer").arg(p).spawn();
}

#[cfg(not(windows))]
pub(crate) fn open_path(p: &std::path::Path) {
    let cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(cmd).arg(p).spawn();
}

pub(crate) fn open_folder(p: &std::path::Path) {
    let dir = p.parent().unwrap_or(p);
    let cmd = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(cmd).arg(dir).spawn();
}

pub(crate) fn backup_path() -> PathBuf {
    store_path().with_file_name("tree-backup.json")
}
