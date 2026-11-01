//! The Family Tree Archive: one ordinary .zip file holding everything needed to rebuild a
//! family history: the tree, every photo, document, PDF, story and recording, a GEDCOM copy that
//! other programs can read, and a note explaining what is inside. Because it is a plain zip, it
//! can still be opened in decades even without this program.

use super::*;
use std::io::{Read, Write};
use zip::write::SimpleFileOptions;

pub const README: &str = "GILDED ROOTS FAMILY TREE ARCHIVE

This file holds a complete family history:

  tree.json        the whole tree as Gilded Roots keeps it (people, families, sources,
                   notes, research tasks, journal, face tags, map positions...)
  family.ged       the same tree as GEDCOM 5.5.1, which almost every family history
                   program can read (PAF, RootsMagic, Legacy, Ancestry, FamilySearch...)
  media/           every photo, document, PDF, story and recording
  manifest.txt     when the archive was made, what it holds, and checksums

To restore it in Gilded Roots: File > Open Family Tree Archive.
In another program: import family.ged, and keep the media folder next to it.
Any zip tool can open this file.
";

/// What an archive holds, read from its manifest (or worked out when there is none).
#[derive(Default, Debug)]
pub struct ArchiveInfo {
    pub people: usize,
    pub families: usize,
    pub media: usize,
    pub made: String,
}

/// Write the archive to `path` (through a temporary file, so a failure never leaves half a file).
pub fn write_archive(db: &Db, path: &Path) -> Result<ArchiveInfo, String> {
    let json = serde_json::to_vec(db).map_err(|e| e.to_string())?;
    let ex = to_ged_full(db, &ExportOpts { prune: false, media_dir: Some("media".into()) });
    let tmp = with_suffix(path, ".partial");
    let f = std::fs::File::create(&tmp).map_err(|e| io_msg(&e))?;
    let mut z = zip::ZipWriter::new(std::io::BufWriter::new(f));
    let packed = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored).large_file(true);
    let mut info = ArchiveInfo { people: db.people.len(), families: db.families_count(), media: 0, made: today() };
    let mut sums = String::new();
    let put = |z: &mut zip::ZipWriter<std::io::BufWriter<std::fs::File>>, name: &str, data: &[u8], opt: SimpleFileOptions| -> Result<(), String> {
        z.start_file(name, opt).map_err(|e| e.to_string())?;
        z.write_all(data).map_err(|e| io_msg(&e))
    };
    put(&mut z, "README.txt", README.as_bytes(), packed)?;
    put(&mut z, "tree.json", &json, packed)?;
    sums.push_str(&format!("tree.json {:016x}\n", fnv64(&json)));
    put(&mut z, "family.ged", ex.text.as_bytes(), packed)?;
    // every media file the tree uses (the GEDCOM names them media/<name>)
    let mut done: HashSet<String> = HashSet::new();
    let mut missing = 0usize;
    for (file, rel) in &ex.media {
        if !done.insert(rel.clone()) {
            continue;
        }
        let Some(src) = safe_media_path(file) else { continue };
        match std::fs::read(&src) {
            Ok(bytes) => {
                let opt = if is_image(file) || is_audio(file) || is_pdf(file) { stored } else { packed };
                put(&mut z, rel, &bytes, opt)?;
                info.media += 1;
            }
            Err(_) => missing += 1,
        }
    }
    // media the GEDCOM does not list (stories saved as text, recordings) are kept too
    let mut extra: Vec<String> = Vec::new();
    for p in db.people.values() {
        for m in &p.media {
            if !m.file.is_empty() && !ex.media.iter().any(|(f, _)| f == &m.file) {
                extra.push(m.file.clone());
            }
        }
    }
    for f in extra {
        let rel = format!("media/{}", f);
        if !done.insert(rel.clone()) {
            continue;
        }
        if let Some(src) = safe_media_path(&f) {
            if let Ok(bytes) = std::fs::read(&src) {
                put(&mut z, &rel, &bytes, if f.ends_with(".txt") { packed } else { stored })?;
                info.media += 1;
            }
        }
    }
    let manifest = format!(
        "Gilded Roots Family Tree Archive\nformat 1\nmade {}\nprogram {}\npeople {}\nfamilies {}\nmedia {}\nmissing media {}\n{}",
        info.made,
        env!("CARGO_PKG_VERSION"),
        info.people,
        info.families,
        info.media,
        missing,
        sums
    );
    put(&mut z, "manifest.txt", manifest.as_bytes(), packed)?;
    let w = z.finish().map_err(|e| e.to_string())?;
    w.into_inner().map_err(|e| e.to_string())?.sync_all().map_err(|e| io_msg(&e))?;
    // check it before putting it in place
    let check = read_info(&tmp)?;
    if check.people != info.people {
        let _ = std::fs::remove_file(&tmp);
        return Err("the archive did not read back correctly".into());
    }
    std::fs::rename(&tmp, path).map_err(|e| io_msg(&e))?;
    Ok(info)
}

fn open_zip(path: &Path) -> Result<zip::ZipArchive<std::fs::File>, String> {
    let f = std::fs::File::open(path).map_err(|e| io_msg(&e))?;
    zip::ZipArchive::new(f).map_err(|_| "This is not a Family Tree Archive (it could not be opened as a zip file).".to_string())
}

/// Read the tree inside an archive and check it against the manifest's checksum.
fn read_tree(z: &mut zip::ZipArchive<std::fs::File>) -> Result<(Db, Vec<u8>), String> {
    let mut json = Vec::new();
    {
        let mut e = z.by_name("tree.json").map_err(|_| "This zip file has no tree.json, so it is not a Family Tree Archive.".to_string())?;
        if e.size() > 2_000_000_000 {
            return Err("The tree in this archive is too large to be real.".into());
        }
        e.read_to_end(&mut json).map_err(|e| io_msg(&e))?;
    }
    let mut manifest = String::new();
    if let Ok(mut m) = z.by_name("manifest.txt") {
        let _ = m.read_to_string(&mut manifest);
    }
    if let Some(want) = manifest.lines().find_map(|l| l.strip_prefix("tree.json ")) {
        if want.trim() != format!("{:016x}", fnv64(&json)) {
            return Err("The tree inside this archive is damaged (its checksum does not match).".into());
        }
    }
    let db: Db = serde_json::from_slice(&json).map_err(|e| format!("The tree inside this archive could not be read: {}", e))?;
    Ok((db, json))
}

/// What an archive holds, without restoring it.
pub fn read_info(path: &Path) -> Result<ArchiveInfo, String> {
    let mut z = open_zip(path)?;
    let (db, _) = read_tree(&mut z)?;
    let media = (0..z.len()).filter(|i| z.by_index(*i).map(|e| e.name().map(|n| n.starts_with("media/")).unwrap_or(false) && !e.is_dir()).unwrap_or(false)).count();
    let mut made = String::new();
    if let Ok(mut m) = z.by_name("manifest.txt") {
        let mut s = String::new();
        let _ = m.read_to_string(&mut s);
        made = s.lines().find_map(|l| l.strip_prefix("made ")).unwrap_or("").to_string();
    }
    Ok(ArchiveInfo { people: db.people.len(), families: db.families_count(), media, made })
}

/// Unpack an archive: its media go into the media folder (files already there are kept) and the
/// tree is returned for the caller to put in place.
pub fn read_archive(path: &Path) -> Result<(Db, usize), String> {
    let mut z = open_zip(path)?;
    let (db, _) = read_tree(&mut z)?;
    let mut n = 0usize;
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        let Ok(name) = e.name().map(|n| n.to_string()) else { continue };
        let Some(file) = name.strip_prefix("media/") else { continue };
        // only plain file names, never folders or ".." (a damaged or hostile zip cannot write elsewhere)
        let Some(dst) = safe_media_path(file) else { continue };
        if e.is_dir() {
            continue;
        }
        if dst.exists() && std::fs::metadata(&dst).map(|m| m.len() == e.size()).unwrap_or(false) {
            n += 1;
            continue;
        }
        let mut bytes = Vec::with_capacity(e.size().min(200_000_000) as usize);
        e.read_to_end(&mut bytes).map_err(|e| io_msg(&e))?;
        atomic_write(&dst, &bytes).map_err(|e| io_msg(&e))?;
        n += 1;
    }
    Ok((db, n))
}
