//! Importing, exporting, backups, archives and switching trees.

use super::*;

impl App {
pub(crate)     fn spawn_media_job(&mut self, restore: bool) {
        if self.media_job.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel::<(bool, usize, usize, Option<String>)>();
        let live = media_dir();
        let copy = data_dir().join("backups").join("media");
        std::thread::spawn(move || {
            let r = if restore { mirror_dir(&copy, &live, true, false) } else { mirror_dir(&live, &copy, false, false) };
            let _ = tx.send((restore, r.0, r.1, r.2));
        });
        self.media_job = Some(rx);
    }

pub(crate)     fn poll_media_job(&mut self) {
        let mut fin: Option<Option<(bool, usize, usize, Option<String>)>> = None;
        if let Some(rx) = &self.media_job {
            match rx.try_recv() {
                Ok(r) => fin = Some(Some(r)),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => fin = Some(None),
                Err(_) => {}
            }
        }
        if let Some(res) = fin {
            self.media_job = None;
            if let Some((restore, copied, failed, err)) = res {
                if failed > 0 {
                    let msg = format!("{} photo or recording file(s) could not be {}: {}", failed, if restore { "restored" } else { "backed up" }, err.unwrap_or_default());
                    self.status = msg.clone();
                    self.notice = Some(msg);
                } else if restore {
                    self.status = format!("Restored {} missing photo or recording file(s) from the backup.", copied);
                } else if copied > 0 {
                    self.status = format!("Backed up {} new photo or recording file(s).", copied);
                }
            }
        }
    }

pub(crate)     fn run_verify(&mut self) {
        self.verify_results = verify_all_backups();
        let gap = mirror_dir(&media_dir(), &data_dir().join("backups").join("media"), false, true).0;
        self.media_gap = if gap == 0 {
            "Photos and recordings: every file is in the backup.".to_string()
        } else {
            format!("Photos and recordings: {} file(s) are not in the backup yet. Use Tools > Back up now.", gap)
        };
        self.verify_open = true;
    }

pub(crate)     fn read_ged(&mut self) -> Option<Db> {
        // No file-type filter: some programs and drives give GEDCOM files odd or missing extensions, and a filter greys them out.
        let path = rfd::FileDialog::new().set_title("Choose a GEDCOM file (.ged), a PAF file (.paf) or a saved tree (.json)").set_directory(start_dir()).pick_file()?;
        remember_dir(&path);
        match read_ged_file(&path) {
            Ok((db, rep)) => {
                self.report = Some(rep);
                self.report_open = true;
                Some(db)
            }
            Err(e) => {
                self.status = e;
                None
            }
        }
    }

pub(crate)     fn import(&mut self) {
        if let Some(db) = self.read_ged() {
            let n = db.people.len();
            self.sel = db.home.or_else(|| db.people.keys().next().copied());
            self.db = db;
            self.status = format!("Imported {} people.", n);
            self.dirty = true;
        }
    }

pub(crate)     fn import_merge(&mut self) {
        if let Some(inc) = self.read_ged() {
            let file = self.report.as_ref().map(|r| r.file.clone()).unwrap_or_else(|| "the file".to_string());
            let im = ImportMerge::new(&self.db, inc, file);
            if im.order.is_empty() {
                // Nobody in the file looks like anyone in the tree, so there is nothing to decide.
                let ImportMerge { inc, .. } = im;
                if !self.backup() {
                    self.report_open = false;
                    return;
                }
                let (added, _) = merge_into_pairs(&mut self.db, inc, &HashMap::new());
                if self.sel.is_none() {
                    self.sel = self.db.people.keys().next().copied();
                }
                self.status = format!("Added {} people. None of them looked like anyone already in your tree.", added);
                self.dirty = true;
            } else {
                self.imerge = Some(im);
                self.report_open = false;
            }
        }
    }

pub(crate)     fn export(&mut self, private: bool) {
        if let Some(path) = rfd::FileDialog::new().set_title("Save GEDCOM file").set_directory(start_dir()).set_file_name("gilded-roots.ged").save_file() {
            remember_dir(&path);
            self.status = match write_gedcom(&self.db, &path, private) {
                Ok(s) | Err(s) => s,
            };
        }
    }

pub(crate)     fn create(&mut self, d: Dlg) {
        let nid = self.db.alloc();
        let full = d.name.trim().to_string();
        let (gn, sn) = split_name(&full);
        self.db.people.insert(nid, Person { name: full, given: gn, surname: sn, sex: d.sex.clone(), bd: d.bd.trim().to_string(), bp: d.bp.trim().to_string(), dd: d.dd.trim().to_string(), dp: d.dp.trim().to_string(), bud: d.bud.trim().to_string(), bup: d.bup.trim().to_string(), living: d.living && d.dd.trim().is_empty(), notes: d.notes.trim().to_string(), ..Default::default() });
        let me = d.target.or(self.sel);
        match d.kind {
            Kind::Person => {
                self.sel = Some(nid);
                self.view = View::Person;
            }
            Kind::Father | Kind::Mother => {
                let Some(me) = me else { return };
                let father = d.kind == Kind::Father;
                match self.db.famc(me) {
                    Some(f) => {
                        let fm = self.db.fams.get_mut(&f).unwrap();
                        if father { fm.h = Some(nid) } else { fm.w = Some(nid) }
                    }
                    None => {
                        let fid = self.db.alloc();
                        let mut fm = Family { kids: vec![me], ..Default::default() };
                        if father { fm.h = Some(nid) } else { fm.w = Some(nid) }
                        self.db.fams.insert(fid, fm);
                    }
                }
            }
            Kind::ParentIn(f, father) => {
                let Some(me) = me else { return };
                self.db.add_parent(me, Some(f), father, nid);
            }
            Kind::NewParent(father) => {
                let Some(me) = me else { return };
                self.db.add_parent(me, None, father, nid);
            }
            Kind::Sibling => {
                let Some(me) = me else { return };
                match self.db.famc(me) {
                    Some(f) => self.db.fams.get_mut(&f).unwrap().kids.push(nid),
                    None => {
                        let fid = self.db.alloc();
                        self.db.fams.insert(fid, Family { kids: vec![me, nid], ..Default::default() });
                    }
                }
            }
            Kind::Spouse => {
                let Some(me) = me else { return };
                let me_f = self.db.people[&me].sex == "F";
                let fid = self.db.alloc();
                let (h, w) = if me_f { (nid, me) } else { (me, nid) };
                self.db.fams.insert(fid, Family { h: Some(h), w: Some(w), ..Default::default() });
            }
            Kind::Child(f) => {
                let Some(me) = me else { return };
                let fid = match f {
                    Some(f) => f,
                    None => {
                        let fid = self.db.alloc();
                        let me_f = self.db.people[&me].sex == "F";
                        let mut fm = Family::default();
                        if me_f { fm.w = Some(me) } else { fm.h = Some(me) }
                        self.db.fams.insert(fid, fm);
                        fid
                    }
                };
                self.db.fams.get_mut(&fid).unwrap().kids.push(nid);
            }
        }
    }

pub(crate)     fn is_pristine_sample(&self) -> bool {
        if !self.db.sample {
            return false;
        }
        let s = sample();
        s.people.len() == self.db.people.len()
            && s.fams.len() == self.db.fams.len()
            && s.people.values().all(|sp| {
                self.db.people.values().any(|p| p.name == sp.name && p.bd == sp.bd && p.bp == sp.bp && p.dd == sp.dd && p.notes.is_empty() && p.media.is_empty())
            })
    }

pub(crate)     fn backup(&mut self) -> bool {
        if self.db.people.is_empty() || self.x.practice.is_some() {
            return true;
        }
        let json = match serde_json::to_vec(&self.db) {
            Ok(j) => j,
            Err(e) => {
                self.status = format!("Could not make a safety backup, so nothing was changed: {}", e);
                return false;
            }
        };
        match atomic_write(&backup_path(), &json) {
            Ok(_) => true,
            Err(e) => {
                let m = format!("Could not save a safety backup ({}), so nothing was changed.", io_msg(&e));
                self.status = m.clone();
                self.notice = Some(m);
                false
            }
        }
    }

pub(crate)     fn restore(&mut self) {
        match read_db(&backup_path()) {
            Ok(mut old) => {
                fix_names(&mut old);
                if !self.backup() {
                    return;
                }
                self.sel = old.people.keys().next().copied();
                self.db = old;
                self.back.clear();
                self.fwd.clear();
                self.dirty = true;
                self.status = "Restored your previous tree. Your last tree was saved as the new backup.".to_string();
            }
            Err(_) => self.status = "No usable backup found yet.".to_string(),
        }
    }

pub(crate)     fn request_wipe(&mut self, w: Wipe) {
        if w == Wipe::Restore || self.db.people.is_empty() || self.is_pristine_sample() {
            self.run_wipe(w);
        } else {
            self.wipe = Some(w);
        }
    }

pub(crate)     fn run_wipe(&mut self, w: Wipe) {
        if w == Wipe::Restore {
            self.restore();
            return;
        }
        if !self.backup() {
            return;
        }
        match w {
            Wipe::Sample => {
                self.db = sample();
                self.sel = self.db.home.or_else(|| self.db.people.keys().next().copied());
            }
            Wipe::Empty => {
                self.db = Db::default();
                self.sel = None;
            }
            Wipe::Import => self.import(),
            Wipe::Restore => {}
        }
        self.back.clear();
        self.fwd.clear();
        self.dirty = true;
    }
}

impl App {
pub(crate)     fn change_data_folder(&mut self) {
        let Some(parent) = rfd::FileDialog::new().set_title("Choose where to keep your tree (a folder named Gilded Roots is made inside it)").set_directory(std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from).unwrap_or_else(start_dir)).pick_folder() else {
            return;
        };
        let target = parent.join("Gilded Roots");
        let src = data_dir();
        if target == src {
            return;
        }
        if target.join("tree.json").exists() {
            self.status = "That folder already holds a tree. Choose a different place.".to_string();
            return;
        }
        self.persist();
        match copy_dir(&src, &target) {
            Ok(_) if std::fs::read(src.join("tree.json")).ok() == std::fs::read(target.join("tree.json")).ok() => {
                if std::fs::write(config_dir().join("location.txt"), target.to_string_lossy().as_bytes()).is_ok() {
                    self.status = format!("Your tree is now kept in {}. The old folder was left untouched.", target.display());
                } else {
                    self.status = "Copied, but could not save the new location, so the old folder is still in use.".to_string();
                }
            }
            _ => {
                self.status = "Could not copy your data to that folder, so nothing was changed.".to_string();
            }
        }
    }

pub(crate)     fn make_portable(&mut self) {
        let Some(target) = portable_dir() else {
            self.status = "Could not find where the app is stored.".to_string();
            return;
        };
        if target.is_dir() {
            self.status = "This copy is already portable.".to_string();
            return;
        }
        self.persist();
        // every tree comes along (the main one and those under More trees)
        let src = base_dir();
        match copy_dir(&src, &target) {
            Ok(_) => {
                self.status = "Portable mode is on. Your data is now kept in the FamilyTreeData folder next to the app.".to_string();
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&target);
                self.status = format!(
                    "Could not create a folder next to the app ({}). Try moving the app to a folder you can write to, such as your Desktop or a USB stick.",
                    e
                );
            }
        }
    }
}

impl App {
pub(crate)     fn restore_from_file(&mut self) {
        let dir = data_dir().join("backups");
        let Some(path) = rfd::FileDialog::new().set_directory(dir).add_filter("Backup", &["json"]).pick_file() else { return };
        self.restore_path(&path);
    }

pub(crate)     fn restore_path(&mut self, path: &Path) {
        let path = path.to_path_buf();
        if let Err(e) = verify_backup(&path) {
            self.status = format!("That backup is damaged and was not used: {}", e);
            self.notice = Some(self.status.clone());
            return;
        }
        match read_db(&path) {
            Ok(mut old) => {
                fix_names(&mut old);
                if !self.backup() {
                    return;
                }
                self.sel = old.people.keys().next().copied();
                self.db = old;
                self.dirty = true;
                self.status = "Restored from that backup. Your previous tree was saved as the last backup.".to_string();
                self.spawn_media_job(true);
            }
            Err(e) => self.status = format!("That file could not be read as a backup: {}", e),
        }
    }
}

impl App {
pub(crate)     fn save_archive_pick(&mut self) {
        if self.x.practice.is_some() {
            self.status = "This is the sample family; go back to your tree to archive it.".into();
            return;
        }
        let name = format!("Gilded Roots archive {}.zip", today());
        if let Some(path) = rfd::FileDialog::new().set_title("Save Family Tree Archive").set_directory(start_dir()).set_file_name(&name).add_filter("Family Tree Archive", &["zip"]).save_file() {
            remember_dir(&path);
            let db = self.db.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let r = archive::write_archive(&db, &path).map(|i| format!("Saved the Family Tree Archive {} ({} people, {} families, {} photos and documents). Keep a copy somewhere safe.", path.display(), i.people, i.families, i.media));
                let _ = tx.send(r);
            });
            self.x.arch_job = Some(rx);
            self.status = "Making the Family Tree Archive...".into();
        }
    }

pub(crate)     fn open_archive_pick(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_title("Open Family Tree Archive").set_directory(start_dir()).add_filter("Family Tree Archive", &["zip"]).pick_file() {
            remember_dir(&path);
            match archive::read_info(&path) {
                Ok(info) => self.x.arch_confirm = Some((path, info)),
                Err(e) => {
                    self.status = e.clone();
                    self.notice = Some(e);
                }
            }
        }
    }
}

impl App {
    /// Make a backup now: the checked copy in the backups folder, the photos mirror, and (when a
    /// second folder is chosen) a Family Tree Archive there.
pub(crate)     fn backup_now(&mut self, force: bool) {
        if self.x.practice.is_some() || self.db.people.is_empty() {
            return;
        }
        BACKUP_KEEP.store(self.settings.backup_keep, Ordering::Relaxed);
        BACKUP_HOURLY.store(self.settings.backup_every == 1, Ordering::Relaxed);
        match dated_backup_in(&data_dir().join("backups"), &self.db, force) {
            Ok(made) => {
                if force {
                    self.status = "Backup saved and checked.".to_string();
                }
                if made || force {
                    self.spawn_media_job(false);
                    let extra = self.settings.backup_extra.trim().to_string();
                    if !extra.is_empty() && self.x.arch_job.is_none() {
                        let dir = PathBuf::from(&extra);
                        if dir.is_dir() {
                            let db = self.db.clone();
                            let keep = self.settings.backup_keep;
                            let (tx, rx) = std::sync::mpsc::channel();
                            std::thread::spawn(move || {
                                let path = dir.join(format!("Gilded Roots archive {}.zip", today()));
                                let r = archive::write_archive(&db, &path).map(|_| format!("Backup saved, and an archive copy was saved in {}", dir.display()));
                                // keep only the newest archives there too
                                if let Ok(rd) = std::fs::read_dir(&dir) {
                                    let mut v: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("Gilded Roots archive ") && n.to_string_lossy().ends_with(".zip")).unwrap_or(false)).collect();
                                    v.sort();
                                    while v.len() > keep.max(3) {
                                        let _ = std::fs::remove_file(v.remove(0));
                                    }
                                }
                                let _ = tx.send(r);
                            });
                            self.x.arch_job = Some(rx);
                        } else {
                            self.status = format!("Backup saved, but the second-copy folder {} is not available (is the drive plugged in?).", extra);
                        }
                    }
                }
            }
            Err(e) => {
                self.status = format!("Backup failed: {}", e);
                self.notice = Some(self.status.clone());
            }
        }
    }

    /// Called after changes: makes a backup when one is due by the chosen frequency.
pub(crate)     fn backup_if_due(&mut self, now: f64) {
        if now - self.x.backup_check < 60.0 {
            return;
        }
        self.x.backup_check = now;
        let bdir = data_dir().join("backups");
        let newest = list_backups(&bdir).into_iter().filter_map(|n| std::fs::metadata(bdir.join(n)).and_then(|m| m.modified()).ok()).max();
        let age = newest.and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()).unwrap_or(u64::MAX);
        let due = match self.settings.backup_every {
            1 => age >= 3600,
            2 => age >= 7 * 86400,
            _ => today() != self.last_backup_day || age >= 86400,
        };
        if due {
            self.backup_now(false);
            self.last_backup_day = today();
        }
    }

    /// Switch to another tree ("" = the main tree). The open one is saved first.
pub(crate)     fn switch_tree(&mut self, name: &str) {
        if self.x.practice.is_some() {
            self.end_practice();
        }
        if self.dirty && !self.persist() {
            self.status = "Your changes could not be saved, so the tree was not switched.".into();
            return;
        }
        self.editing = None;
        if let Ok(mut g) = CURRENT_TREE.lock() {
            *g = if name.is_empty() { None } else { Some(name.to_string()) };
        }
        let _ = std::fs::create_dir_all(data_dir());
        let (mut db, notice) = load_from(&data_dir());
        fix_names(&mut db);
        self.db = db;
        self.db.home = self.db.home.filter(|h| self.db.people.contains_key(h));
        self.sel = self.db.home.or_else(|| self.db.people.keys().next().copied());
        self.settings.current_tree = name.to_string();
        save_settings(&self.settings);
        self.back.clear();
        self.fwd.clear();
        self.undo.clear();
        self.redo.clear();
        self.viewing = None;
        self.hl = None;
        self.dirty = false;
        self.last_json = serde_json::to_string(&self.db).unwrap_or_default();
        self.remember_state();
        self.x.geo_cache.clear();
        self.x.stats = None;
        self.last_backup_day = String::new();
        self.status = if notice.is_empty() { format!("Opened {}.", tree_title(name)) } else { notice };
    }
}
