//! Media folders sorted by person. Each file in the tree's media folder can sit in a folder
//! named after the person it belongs to ("Harwood, Thomas James (9)"), in "Shared" when it
//! belongs to several people, or in "Sources" when only a source uses it.
//!
//! The tree itself only stores each file's name (always unique: the program adds a number when
//! a file comes in), never its folder. The program finds a file wherever it is in the media
//! folder, so moving files never changes the tree: Undo, backups and older versions of the
//! program keep working, and a person's folder can follow their name.

use super::*;

/// Folders in the media folder that are not person folders.
const NOT_FOLDERS: [&str; 1] = ["previews"];

/// Which subfolder each file is in (files at the top of the media folder are not listed), for
/// one media folder.
static INDEX: Mutex<Option<(PathBuf, HashMap<String, String>)>> = Mutex::new(None);

fn scan(dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return map };
    for e in rd.flatten() {
        let folder = e.file_name().to_string_lossy().to_string();
        if folder.starts_with('.') || NOT_FOLDERS.contains(&folder.as_str()) || !e.path().is_dir() {
            continue;
        }
        if let Ok(inner) = std::fs::read_dir(e.path()) {
            for f in inner.flatten() {
                let name = f.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && f.path().is_file() {
                    map.insert(name, folder.clone());
                }
            }
        }
    }
    map
}

fn with_index<R>(dir: &Path, f: impl FnOnce(&mut HashMap<String, String>) -> R) -> R {
    let mut g = match INDEX.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    if g.as_ref().map_or(true, |x| x.0 != dir) {
        *g = Some((dir.to_path_buf(), scan(dir)));
    }
    f(&mut g.as_mut().unwrap().1)
}

/// The subfolder a media file is in, if any.
pub(crate) fn media_folder_of(dir: &Path, file: &str) -> Option<String> {
    with_index(dir, |m| m.get(file).cloned())
}

/// The file is no longer in a person folder (moved out of the media folder, say).
pub(crate) fn forget_media_file(dir: &Path, file: &str) {
    with_index(dir, |m| m.remove(file));
}

/// Read the media folder again (after files were moved outside the program).
pub(crate) fn forget_media_index() {
    if let Ok(mut g) = INDEX.lock() {
        *g = None;
    }
}

/// Every media file in the media folder and its person folders: (name, size, last changed).
pub(crate) fn all_media_files(dir: &Path) -> Vec<(String, u64, std::time::SystemTime)> {
    let mut out = Vec::new();
    let mut add = |p: &Path| {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let Ok(md) = e.metadata() else { continue };
                let name = e.file_name().to_string_lossy().to_string();
                if md.is_file() && !name.starts_with('.') && is_media_name(&name) {
                    out.push((name, md.len(), md.modified().unwrap_or(std::time::UNIX_EPOCH)));
                }
            }
        }
    };
    add(dir);
    let folders: HashSet<String> = with_index(dir, |m| m.values().cloned().collect());
    for f in folders {
        add(&dir.join(f));
    }
    out
}

/// A folder name made from a person's name and ID, safe on every computer.
pub(crate) fn person_folder_name(id: u32, p: &Person) -> String {
    let (s, g) = (p.surname.trim(), p.given.trim());
    let base = match (s.is_empty(), g.is_empty()) {
        (false, false) => format!("{}, {}", s, g),
        (false, true) => s.to_string(),
        (true, false) => g.to_string(),
        (true, true) => {
            let n = p.name.trim();
            if n.is_empty() { "Person".to_string() } else { n.to_string() }
        }
    };
    let mut clean: String = base.chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '-' } else { c }).collect();
    clean = clean.trim().trim_matches('.').trim().to_string();
    if clean.chars().count() > 80 {
        clean = clean.chars().take(80).collect::<String>().trim().to_string();
    }
    format!("{} ({})", clean, id)
}

/// One file to move: its name, the folder it is in now (None = the top of the media folder)
/// and the folder it belongs in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MediaMove {
    pub(crate) file: String,
    pub(crate) from: Option<String>,
    pub(crate) to: String,
}

/// Where every file the tree uses belongs: the folder of the one person it is on, "Shared"
/// when it is on several people (or on a couple), "Sources" when only a source uses it.
pub(crate) fn media_homes(db: &Db) -> HashMap<String, String> {
    let mut owners: HashMap<String, BTreeSet<u32>> = HashMap::new();
    for (id, p) in &db.people {
        let portrait = p.face_portrait.as_ref().map(|f| f.file.as_str()).into_iter().chain([p.portrait.as_str()]);
        for f in p.media.iter().map(|m| m.file.as_str()).chain(portrait) {
            if !f.is_empty() {
                owners.entry(f.to_string()).or_default().insert(*id);
            }
        }
    }
    for f in db.fams.values() {
        for m in &f.media {
            if !m.file.is_empty() {
                let e = owners.entry(m.file.clone()).or_default();
                e.extend(f.h.iter().chain(f.w.iter()));
                if e.len() < 2 {
                    // a couple's photo with only one partner recorded still counts as shared
                    e.insert(u32::MAX);
                }
            }
        }
    }
    let mut homes = HashMap::new();
    for f in media_in_use(db) {
        let home = match owners.get(&f) {
            Some(o) if o.len() == 1 => {
                let id = *o.iter().next().unwrap();
                match db.people.get(&id) {
                    Some(p) => person_folder_name(id, p),
                    None => "Shared".to_string(),
                }
            }
            Some(_) => "Shared".to_string(),
            None => "Sources".to_string(),
        };
        homes.insert(f, home);
    }
    homes
}

/// The files that are not in the folder they belong in. Files that are missing are left out.
pub(crate) fn media_plan(db: &Db) -> Vec<MediaMove> {
    let dir = media_dir();
    let mut plan: Vec<MediaMove> = Vec::new();
    for (file, to) in media_homes(db) {
        if safe_media_path(&file).is_none() {
            continue;
        }
        let from = media_folder_of(&dir, &file);
        if from.as_deref() == Some(to.as_str()) {
            continue;
        }
        let src = match &from {
            Some(f) => dir.join(f).join(&file),
            None => dir.join(&file),
        };
        if src.is_file() {
            plan.push(MediaMove { file, from, to });
        }
    }
    plan.sort_by(|a, b| a.to.cmp(&b.to).then(a.file.cmp(&b.file)));
    plan
}

/// Moves one file from one place to another in the same folder tree: renamed when possible,
/// otherwise copied, checked and then the old copy removed.
fn move_file(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(p) = dst.parent() {
        std::fs::create_dir_all(p).map_err(|e| io_msg(&e))?;
    }
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    let part = with_suffix(dst, ".part");
    std::fs::copy(src, &part).map_err(|e| io_msg(&e))?;
    let same = std::fs::metadata(src).map(|m| m.len()).ok() == std::fs::metadata(&part).map(|m| m.len()).ok();
    if !same {
        let _ = std::fs::remove_file(&part);
        return Err("the copy did not match".to_string());
    }
    std::fs::rename(&part, dst).map_err(|e| io_msg(&e))?;
    std::fs::remove_file(src).map_err(|e| io_msg(&e))
}

/// Moves the files as planned, in the media folder and in the backup copy of it. Returns how
/// many moved and what went wrong with any that did not. Person folders left empty are removed.
pub(crate) fn apply_media_plan(plan: &[MediaMove]) -> (usize, Vec<String>) {
    let dir = media_dir();
    let backup = data_dir().join("backups").join("media");
    let mut moved = 0;
    let mut problems = Vec::new();
    let mut emptied: HashSet<String> = HashSet::new();
    for mv in plan {
        let at = |base: &Path, folder: &Option<String>| match folder {
            Some(f) => base.join(f).join(&mv.file),
            None => base.join(&mv.file),
        };
        let (src, dst) = (at(&dir, &mv.from), dir.join(&mv.to).join(&mv.file));
        if !src.is_file() {
            continue;
        }
        if dst.exists() {
            // the same file is already where it belongs (put back from a backup, say)
            let same = std::fs::metadata(&src).map(|m| m.len()).ok() == std::fs::metadata(&dst).map(|m| m.len()).ok();
            if same {
                with_index(&dir, |m| m.insert(mv.file.clone(), mv.to.clone()));
                continue;
            }
            problems.push(format!("{}: a different file with that name is already in \"{}\"", mv.file, mv.to));
            continue;
        }
        match move_file(&src, &dst) {
            Ok(()) => {
                moved += 1;
                with_index(&dir, |m| m.insert(mv.file.clone(), mv.to.clone()));
                if let Some(f) = &mv.from {
                    emptied.insert(f.clone());
                }
                // the backup copy follows, so it does not end up with two copies of every file
                let (bsrc, bdst) = (at(&backup, &mv.from), backup.join(&mv.to).join(&mv.file));
                if bsrc.is_file() && !bdst.exists() {
                    let _ = move_file(&bsrc, &bdst);
                }
            }
            Err(e) => problems.push(format!("{}: {}", mv.file, e)),
        }
    }
    for f in emptied {
        for base in [&dir, &backup] {
            let p = base.join(&f);
            if std::fs::read_dir(&p).map(|mut r| r.next().is_none()).unwrap_or(false) {
                let _ = std::fs::remove_dir(&p);
            }
        }
    }
    (moved, problems)
}

impl App {
    /// Keeps the media folders sorted, when the tree is set to: run after changes are saved
    /// (a renamed person's files move to their new folder, a photo added to a second person moves
    /// to Shared).
    pub(crate) fn tidy_media(&mut self) {
        if !self.db.media_folders || self.x.practice.is_some() {
            return;
        }
        if self.media_job.is_some() {
            // a backup is copying the media folder: wait until it is done
            self.x.tidy_pending = true;
            return;
        }
        self.x.tidy_pending = false;
        let plan = media_plan(&self.db);
        if plan.is_empty() {
            return;
        }
        let (moved, problems) = apply_media_plan(&plan);
        self.x.unlinked = None;
        if moved > 0 {
            self.status = format!("Moved {} file(s) to their person folders in the media folder.", moved);
        }
        if let Some(p) = problems.first() {
            self.status = format!("Could not move {} file(s) to their folder: {}", problems.len(), p);
        }
    }

    /// Tools > Sort media into person folders.
    pub(crate) fn media_folders_window(&mut self, ctx: &egui::Context) {
        if !self.x.folders_open {
            return;
        }
        let mut open = true;
        let mut go = false;
        let mut stop = false;
        if self.x.folders_plan.is_none() {
            forget_media_index();
            self.x.folders_plan = Some(media_plan(&self.db));
        }
        let plan = self.x.folders_plan.clone().unwrap_or_default();
        let on = self.db.media_folders;
        egui::Window::new("Sort media into person folders").id(egui::Id::new("media_folders")).open(&mut open).collapsible(false).default_width(560.0).show(ctx, |ui| {
            ui.label("Puts each photo, document and recording in a folder named after the person it belongs to, inside the media folder, so you can find a person's files on your computer.");
            ui.add_space(4.0);
            ui.label(RichText::new("- One person's files go in a folder like \"Harwood, Thomas James (9)\" (the ID number keeps two people with the same name apart).\n- A file on several people, or on a couple, goes in \"Shared\".\n- A file only a source uses goes in \"Sources\".\n- After this, it stays sorted: new files go to their folder, a person's folder follows their name, and a file added to a second person moves to Shared.").small());
            ui.label(RichText::new("Nothing is deleted and your tree is not changed: files are only moved, and the backup copy of the media folder is sorted the same way. Undo does not need to move anything back.").small().weak());
            ui.separator();
            if plan.is_empty() {
                ui.label(if on { "Everything is already in its folder." } else { "No files need moving (there may be no files yet)." });
            } else {
                let folders: HashSet<&str> = plan.iter().map(|m| m.to.as_str()).collect();
                ui.label(RichText::new(format!("{} file(s) will move into {} folder(s):", plan.len(), folders.len())).strong());
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for m in plan.iter().take(200) {
                        ui.label(RichText::new(format!("{}  ->  {}", m.file, m.to)).small().monospace());
                    }
                    if plan.len() > 200 {
                        ui.label(RichText::new(format!("... and {} more", plan.len() - 200)).small().weak());
                    }
                });
            }
            if self.media_job.is_some() {
                ui.label(RichText::new("A backup is copying the media folder. Wait a moment for it to finish.").small().weak());
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let label = if on { "Sort now" } else { "Sort the files and keep them sorted" };
                if ui.add_enabled(self.media_job.is_none() && (!plan.is_empty() || !on), egui::Button::new(RichText::new(label).strong())).clicked() {
                    go = true;
                }
                if on && ui.button("Stop keeping them sorted").on_hover_text("Files stay where they are now; new files go in the top of the media folder").clicked() {
                    stop = true;
                }
                if ui.button("Open media folder").clicked() {
                    open_path(&media_dir());
                }
            });
        });
        if go {
            self.db.media_folders = true;
            self.dirty = true;
            let (moved, problems) = apply_media_plan(&plan);
            self.x.unlinked = None;
            self.status = if problems.is_empty() {
                format!("Moved {} file(s) into person folders. New files will be sorted too.", moved)
            } else {
                format!("Moved {} file(s); {} could not be moved: {}", moved, problems.len(), problems[0])
            };
            open = false;
        }
        if stop {
            self.db.media_folders = false;
            self.dirty = true;
            self.status = "Media files will no longer be sorted into person folders. They stay where they are.".to_string();
            open = false;
        }
        if !open {
            self.x.folders_open = false;
            self.x.folders_plan = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_are_safe_and_tell_people_apart() {
        let p = Person { given: "Thomas James".into(), surname: "Harwood".into(), ..Default::default() };
        assert_eq!(person_folder_name(9, &p), "Harwood, Thomas James (9)");
        let odd = Person { given: "A/B: \"C\"".into(), surname: "".into(), ..Default::default() };
        let n = person_folder_name(3, &odd);
        assert!(!n.contains('/') && !n.contains(':') && !n.contains('"') && n.ends_with("(3)"), "{n}");
        assert_eq!(person_folder_name(4, &Person::default()), "Person (4)");
    }

    #[test]
    fn files_belong_with_their_person_or_in_shared() {
        let mut db = Db::default();
        let m = |f: &str| Media { file: f.into(), ..Default::default() };
        db.people.insert(1, Person { given: "Ann".into(), surname: "Lee".into(), media: vec![m("a.jpg"), m("both.jpg")], ..Default::default() });
        db.people.insert(2, Person { given: "Bob".into(), surname: "Lee".into(), media: vec![m("both.jpg")], ..Default::default() });
        db.fams.insert(5, Family { h: Some(2), w: Some(1), media: vec![m("wedding.jpg")], ..Default::default() });
        let h = media_homes(&db);
        assert_eq!(h["a.jpg"], "Lee, Ann (1)");
        assert_eq!(h["both.jpg"], "Shared");
        assert_eq!(h["wedding.jpg"], "Shared");
    }
}
