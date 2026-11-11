//! Tests for the main program.

use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("familytree-test-{}-{}", tag, stamp()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn good_db() -> Db {
        sample()
    }

    fn damaged_files(d: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(d)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("tree.json.damaged-")).unwrap_or(false))
            .collect()
    }


    /// Whole path: a tree with photos is written as a GEDCOM file, then read back as if it came from another program.
    #[test]
    fn export_then_import_with_photos_end_to_end() {
        let home = tdir("home");
        std::env::set_var("HOME", &home);
        std::env::set_var("APPDATA", &home);
        let mut db = Db::default();
        let a = db.alloc();
        let b = db.alloc();
        let photo = media_dir().join("111-grandpa.jpg");
        std::fs::write(&photo, b"not really a jpeg but bytes are bytes").unwrap();
        db.people.insert(a, Person { name: "Grand Pa".into(), given: "Grand".into(), surname: "Pa".into(), bd: "about 1850".into(), media: vec![Media { file: "111-grandpa.jpg".into(), name: "grandpa.jpg".into(), caption: "On the farm".into(), ..Default::default() }], ..Default::default() });
        db.people.insert(b, Person { name: "Kid Pa".into(), given: "Kid".into(), surname: "Pa".into(), living: true, bd: "2001".into(), ..Default::default() });
        let f = db.alloc();
        db.fams.insert(f, Family { h: Some(a), kids: vec![b], ..Default::default() });

        let out = tdir("out");
        let ged = out.join("tree.ged");
        let msg = write_gedcom(&db, &ged, false).expect("export ok");
        assert!(msg.contains("1 photo/document file"), "{}", msg);
        assert_eq!(std::fs::read(out.join("tree-media").join("grandpa.jpg")).unwrap(), b"not really a jpeg but bytes are bytes");
        let text = std::fs::read_to_string(&ged).unwrap();
        assert!(text.contains("2 FILE tree-media/grandpa.jpg") && text.contains("ABT 1850"));

        // a private copy has no living person and no photos of them
        let ged2 = out.join("private.ged");
        write_gedcom(&db, &ged2, true).expect("private export ok");
        let t2 = std::fs::read_to_string(&ged2).unwrap();
        assert!(!t2.contains("2001") && !t2.contains("Kid Pa") && t2.contains("Living"));

        // another computer: empty data folder
        let home2 = tdir("home2");
        std::env::set_var("HOME", &home2);
        std::env::set_var("APPDATA", &home2);
        let (db2, rep) = read_ged_file(&ged).expect("import ok");
        assert_eq!(db2.people.len(), 2);
        assert!(rep.lost.is_empty(), "{:?}", rep.lost);
        assert_eq!((rep.media_found, rep.media_missing), (1, 0));
        let m = &db2.people.values().find(|p| p.given == "Grand").unwrap().media[0];
        assert!(!m.file.is_empty() && m.path.is_empty());
        assert_eq!(m.caption, "On the farm");
        assert_eq!(std::fs::read(mfile(&m.file)).unwrap(), b"not really a jpeg but bytes are bytes");

        // a photo that cannot be found is reported, and its name is kept
        std::fs::remove_file(out.join("tree-media").join("grandpa.jpg")).unwrap();
        let (db3, rep3) = read_ged_file(&ged).expect("import ok");
        assert_eq!((rep3.media_found, rep3.media_missing), (0, 1));
        assert_eq!(db3.people.values().find(|p| p.given == "Grand").unwrap().media[0].path, "tree-media/grandpa.jpg");
        assert!(to_ged(&db3).contains("2 FILE tree-media/grandpa.jpg"));
    }

    #[test]
    fn saved_tree_json_round_trips_every_new_field() {
        let text = "0 HEAD\n0 @S1@ SOUR\n1 TITL T\n0 @N1@ NOTE n\n0 @I1@ INDI\n1 NAME A /B/\n1 NAME C /D/\n2 TYPE aka\n1 BIRT\n2 DATE ABT 1900\n2 SOUR @S1@\n3 PAGE 4\n2 CAUS x\n1 NOTE @N1@\n1 _X y\n1 OBJE\n2 FILE a.jpg\n0 @F1@ FAM\n1 HUSB @I1@\n1 MARR\n2 TYPE civil\n1 DIV\n0 TRLR\n";
        let (db, _) = parse_ged_report(text);
        let dir = tdir("json");
        save_db_to(&dir, &db).unwrap();
        let back = read_db(&dir.join("tree.json")).unwrap();
        assert_eq!(to_ged(&back), to_ged(&db));
        assert!(loss_check(text, &to_ged(&back)).is_empty());
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let d = tdir("aw");
        let p = d.join("f.json");
        atomic_write(&p, b"one").unwrap();
        atomic_write(&p, b"two").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"two");
        assert!(!with_suffix(&p, ".tmp").exists());
    }

    #[test]
    fn disk_full_during_write_keeps_old_file() {
        let d = tdir("full");
        let p = d.join("f.json");
        atomic_write(&p, b"good").unwrap();
        let code = if cfg!(windows) { 112 } else { 28 };
        let r = atomic_write_with(&p, b"brand new data", &|f: &mut std::fs::File, data: &[u8]| {
            f.write_all(&data[..4])?;
            Err(std::io::Error::from_raw_os_error(code))
        });
        let e = r.unwrap_err();
        assert!(is_disk_full(&e));
        assert_eq!(io_msg(&e), "the disk is full");
        assert_eq!(std::fs::read(&p).unwrap(), b"good");
        assert!(!with_suffix(&p, ".tmp").exists());
    }

    #[test]
    fn silently_short_write_is_caught_and_old_file_kept() {
        let d = tdir("short");
        let p = d.join("f.json");
        atomic_write(&p, b"good").unwrap();
        let r = atomic_write_with(&p, b"brand new data", &|f: &mut std::fs::File, data: &[u8]| f.write_all(&data[..data.len() / 2]));
        assert!(r.is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"good");
        assert!(!with_suffix(&p, ".tmp").exists());
    }

    #[test]
    fn crash_mid_save_leaves_partial_temp_that_is_ignored() {
        let d = tdir("crash");
        save_db_to(&d, &good_db()).unwrap();
        std::fs::write(d.join("tree.json.tmp"), br#"{"people":{"1":{"na"#).unwrap();
        let (db, note) = load_from(&d);
        assert_eq!(db.people.len(), good_db().people.len());
        assert!(note.is_empty());
        assert!(!d.join("tree.json.tmp").exists());
    }

    #[test]
    fn corrupt_main_recovers_from_previous_version() {
        let d = tdir("prev");
        save_db_to(&d, &good_db()).unwrap();
        save_db_to(&d, &good_db()).unwrap();
        std::fs::write(d.join("tree.json"), b"{ this is not json").unwrap();
        let (db, note) = load_from(&d);
        assert_eq!(db.people.len(), good_db().people.len());
        assert!(note.contains("recovered"));
        assert_eq!(damaged_files(&d).len(), 1);
    }

    #[test]
    fn corrupt_main_and_previous_recovers_from_dated_backup() {
        let d = tdir("dated");
        let db = good_db();
        save_db_to(&d, &db).unwrap();
        dated_backup_in(&d.join("backups"), &db, true).unwrap();
        std::fs::write(d.join("tree.json"), b"garbage").unwrap();
        std::fs::write(d.join("tree.json.prev"), b"").unwrap();
        let (got, note) = load_from(&d);
        assert_eq!(got.people.len(), db.people.len());
        assert!(note.contains("tree-"));
    }

    #[test]
    fn everything_corrupt_starts_empty_but_keeps_the_damaged_file() {
        let d = tdir("allbad");
        std::fs::write(d.join("tree.json"), b"garbage").unwrap();
        let (got, note) = load_from(&d);
        assert!(got.people.is_empty());
        assert!(note.contains("no backup"));
        assert!(!d.join("tree.json").exists());
        let kept = damaged_files(&d);
        assert_eq!(kept.len(), 1);
        assert_eq!(std::fs::read(&kept[0]).unwrap(), b"garbage");
    }

    #[test]
    fn fresh_install_is_empty_without_any_warning() {
        let d = tdir("fresh");
        let (got, note) = load_from(&d);
        assert!(got.people.is_empty());
        assert!(note.is_empty());
    }

    #[test]
    fn truncated_and_empty_files_are_rejected() {
        let json = serde_json::to_vec(&good_db()).unwrap();
        let d = tdir("trunc");
        let p = d.join("t.json");
        std::fs::write(&p, &json[..json.len() / 2]).unwrap();
        assert!(read_db(&p).is_err());
        std::fs::write(&p, b"").unwrap();
        assert!(read_db(&p).is_err());
    }

    #[test]
    fn a_damaged_main_file_is_never_copied_over_the_good_previous_version() {
        let d = tdir("keepprev");
        save_db_to(&d, &good_db()).unwrap();
        save_db_to(&d, &good_db()).unwrap();
        std::fs::write(d.join("tree.json"), b"junk").unwrap();
        save_db_to(&d, &good_db()).unwrap();
        assert!(read_db(&d.join("tree.json.prev")).is_ok());
        assert!(read_db(&d.join("tree.json")).is_ok());
    }

    #[test]
    fn backup_checksum_detects_silent_damage() {
        let d = tdir("sum");
        let name = "tree-2026-01-01.json";
        let json = serde_json::to_vec(&good_db()).unwrap();
        write_backup_in(&d, name, &json).unwrap();
        assert_eq!(verify_backup(&d.join(name)).unwrap(), good_db().people.len());
        let mut bytes = std::fs::read(d.join(name)).unwrap();
        let pos = bytes.windows(5).position(|w| w == b"Henry").unwrap();
        bytes[pos + 4] = b'i';
        std::fs::write(d.join(name), &bytes).unwrap();
        assert!(verify_backup(&d.join(name)).unwrap_err().contains("checksum"));
    }

    #[test]
    fn todays_damaged_backup_is_replaced() {
        let d = tdir("fix");
        let name = format!("tree-{}.json", today());
        std::fs::write(d.join(&name), b"junk").unwrap();
        assert!(dated_backup_in(&d, &good_db(), false).unwrap());
        assert!(verify_backup(&d.join(&name)).is_ok());
    }

    #[test]
    fn old_backups_are_pruned_to_fourteen() {
        let d = tdir("prune");
        let json = serde_json::to_vec(&good_db()).unwrap();
        for i in 1..=20 {
            write_backup_in(&d, &format!("tree-2026-01-{:02}.json", i), &json).unwrap();
        }
        prune_backups(&d);
        let names = list_backups(&d);
        assert_eq!(names.len(), 14);
        assert_eq!(names[0], "tree-2026-01-07.json");
        assert!(!d.join("tree-2026-01-01.json.fnv").exists());
    }

    #[test]
    fn media_mirror_copies_only_new_or_changed_files_and_skips_previews() {
        let src = tdir("msrc");
        let dst = tdir("mdst");
        std::fs::write(src.join("a.txt"), b"aaa").unwrap();
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("sub").join("b.txt"), b"bbb").unwrap();
        std::fs::create_dir_all(src.join("previews")).unwrap();
        std::fs::write(src.join("previews").join("skip.jpg"), b"x").unwrap();
        assert_eq!(mirror_dir(&src, &dst, false, true).0, 2);
        assert!(!dst.join("a.txt").exists());
        let r = mirror_dir(&src, &dst, false, false);
        assert_eq!((r.0, r.1), (2, 0));
        assert!(dst.join("sub").join("b.txt").exists());
        assert!(!dst.join("previews").exists());
        assert_eq!(mirror_dir(&src, &dst, false, false).0, 0);
        std::fs::write(src.join("a.txt"), b"aaaa-changed").unwrap();
        assert_eq!(mirror_dir(&src, &dst, false, false).0, 1);
        std::fs::write(dst.join("a.txt"), b"changed in backup").unwrap();
        assert_eq!(mirror_dir(&dst, &src, true, false).0, 0);
    }
}

#[cfg(test)]
mod import_kinds {
    use super::*;
    fn tmp(name: &str, data: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("gr-{}-{}", std::process::id(), name));
        std::fs::write(&p, data).unwrap();
        p
    }
    #[test]
    fn json_tree_and_binary_files_are_recognised() {
        let mut db = Db::default();
        db.people.insert(1, Person { name: "Ann Test".into(), ..Default::default() });
        let p = tmp("t.json", serde_json::to_string(&db).unwrap().as_bytes());
        let (d, rep) = read_ged_file(&p).expect("json tree should load");
        assert_eq!(d.people.len(), 1);
        assert_eq!(rep.people, 1);
        let b = tmp("t.paf", &[0u8; 600]);
        let e = match read_ged_file(&b) { Err(e) => e, Ok(_) => panic!("binary accepted") };
        assert!(e.contains("Export"), "{}", e);
        let o = tmp("o.json", b"{\"a\":1}");
        assert!(read_ged_file(&o).is_err());
    }
}

#[cfg(test)]
mod data_move {
    use super::*;
    #[test]
    fn old_folder_is_copied_checked_and_marked() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let base = std::env::temp_dir().join(format!("gr-move-{}-{}", std::process::id(), nanos));
        let (old, new) = (base.join("old"), base.join("Documents").join("Gilded Roots"));
        std::fs::create_dir_all(old.join("media")).unwrap();
        std::fs::write(old.join("tree.json"), b"{\"x\":1}").unwrap();
        std::fs::write(old.join("settings.json"), b"{}").unwrap();
        std::fs::write(old.join("media").join("a.jpg"), b"img").unwrap();
        assert!(migrate_old_data(&old, &new));
        assert_eq!(std::fs::read(new.join("tree.json")).unwrap(), b"{\"x\":1}");
        assert!(new.join("media").join("a.jpg").exists());
        assert!(!new.join("settings.json").exists());
        assert!(old.join("tree.json").exists(), "old copy must stay");
        assert!(old.join("MOVED.txt").exists());
        // Second call does nothing and does not fail.
        std::fs::remove_file(new.join("tree.json")).unwrap();
        assert!(migrate_old_data(&old, &new));
        assert!(!new.join("tree.json").exists(), "marked folders are not moved twice");
    }
}

#[cfg(test)]
mod sync_detect {
    use super::*;
    #[test]
    fn cloud_folders_are_flagged() {
        assert_eq!(synced_service(std::path::Path::new("/Users/a/Library/Mobile Documents/com~apple~CloudDocs/Gilded Roots")), Some("iCloud Drive"));
        assert_eq!(synced_service(std::path::Path::new("C:\\Users\\a\\OneDrive\\Documents\\Gilded Roots")), Some("OneDrive"));
        assert_eq!(synced_service(std::path::Path::new("/tmp/nothing-special/Gilded Roots")), None);
    }
}

#[cfg(test)]
mod citation_tests {
    use super::*;

    fn tree() -> (Db, u32, u32, u32) {
        let mut db = Db::default();
        let (a, b) = (db.alloc(), db.alloc());
        db.people.insert(a, Person { name: "Ann Test".into(), ..Default::default() });
        db.people.insert(b, Person { name: "Bob Test".into(), ..Default::default() });
        let repo = db.alloc();
        db.repos.insert(repo, RepoRec { name: "County Archive".into(), url: "https://archive.example".into(), ..Default::default() });
        let s = db.alloc();
        db.sources.insert(s, SourceRec { title: "Parish register".into(), author: "Church".into(), repo: Some(repo), call: "KB 4/2".into(), ..Default::default() });
        db.people.get_mut(&a).unwrap().sources.push(Source { sid: Some(s), about: "Birth".into(), page: "p. 3".into(), accessed: "5 Jan 2024".into(), ..Default::default() });
        db.people.get_mut(&b).unwrap().events.push(Event { kind: "Residence".into(), date: "1900".into(), cites: vec![Source { sid: Some(s), page: "p. 9".into(), ..Default::default() }], ..Default::default() });
        (db, a, s, repo)
    }

    #[test]
    fn one_source_is_used_by_people_and_events() {
        let (db, a, s, _) = tree();
        let uses = db.source_uses(s);
        assert_eq!(uses.len(), 2);
        assert_eq!(db.source_counts()[&s], 2);
        assert!(uses.iter().any(|u| u.0 == Owner::Person(a) && u.1 == "Birth" && u.3 == "p. 3"));
        assert!(uses.iter().any(|u| u.1 == "Residence" && u.2 == "1900"));
    }

    #[test]
    fn citation_text_gathers_source_page_repository_and_access_date() {
        let (db, a, _, _) = tree();
        let t = db.cite_text(&db.people[&a].sources[0]);
        assert_eq!(t, "Church, Parish register. p. 3. County Archive, KB 4/2 (accessed 5 Jan 2024)");
    }

    #[test]
    fn merging_sources_moves_every_citation_and_fills_gaps() {
        let (mut db, a, s, _) = tree();
        let dup = db.alloc();
        db.sources.insert(dup, SourceRec { title: "Parish register (copy)".into(), url: "https://x.example".into(), publ: "Oslo 1880".into(), ..Default::default() });
        db.people.get_mut(&a).unwrap().sources.push(Source { sid: Some(dup), page: "p. 7".into(), ..Default::default() });
        db.merge_sources(s, dup);
        assert!(!db.sources.contains_key(&dup));
        assert_eq!(db.source_counts()[&s], 3);
        let k = &db.sources[&s];
        assert_eq!((k.url.as_str(), k.publ.as_str(), k.title.as_str()), ("https://x.example", "Oslo 1880", "Parish register"));
        assert!(db.people[&a].sources.iter().all(|c| c.sid == Some(s)));
        assert_eq!(db.people[&a].sources[1].page, "p. 7", "page stays with the citation");
    }

    #[test]
    fn deleting_a_source_keeps_its_name_in_the_citations() {
        let (mut db, a, s, _) = tree();
        db.delete_source(s);
        let c = &db.people[&a].sources[0];
        assert_eq!((c.sid, c.text.as_str(), c.page.as_str()), (None, "Parish register", "p. 3"));
        assert!(to_ged(&db).contains("SOUR Parish register"));
    }

    #[test]
    fn deleting_a_repository_leaves_the_source() {
        let (mut db, _, s, repo) = tree();
        db.delete_repo(repo);
        assert!(db.sources[&s].repo.is_none());
        assert!(!to_ged(&db).contains("County Archive"));
    }

    #[test]
    fn everything_survives_save_and_reload_and_gedcom() {
        let (db, a, s, repo) = tree();
        let json = serde_json::to_string(&db).unwrap();
        let back: Db = serde_json::from_str(&json).unwrap();
        assert_eq!(back.repos[&repo].url, "https://archive.example");
        assert_eq!(back.people[&a].sources[0].accessed, "5 Jan 2024");
        let out = to_ged(&back);
        let (re, _) = parse_ged_report(&out);
        assert_eq!(re.repos.len(), 1);
        let rs = re.sources.values().next().unwrap();
        assert_eq!((rs.call.as_str(), rs.author.as_str()), ("KB 4/2", "Church"));
        assert_eq!(re.source_counts().values().sum::<usize>(), 2);
        let _ = s;
    }
}

#[cfg(test)]
mod relation_tests {
    use super::*;

    fn pers(db: &mut Db, name: &str, sex: &str) -> u32 {
        let id = db.alloc();
        db.people.insert(id, Person { name: name.into(), sex: sex.into(), ..Default::default() });
        id
    }
    fn fam(db: &mut Db, h: Option<u32>, w: Option<u32>, kids: &[u32], md: &str) -> u32 {
        let id = db.alloc();
        db.fams.insert(id, Family { h, w, kids: kids.to_vec(), md: md.into(), ..Default::default() });
        id
    }

    /// Kid is born to Ann and Bob, then adopted by Cal and Dee. Cal and Dee have their own child Eve.
    fn adopted_tree() -> (Db, u32, u32, u32, u32, u32, u32, u32) {
        let mut db = Db::default();
        let (bob, ann) = (pers(&mut db, "Bob Birth", "M"), pers(&mut db, "Ann Birth", "F"));
        let (cal, dee) = (pers(&mut db, "Cal Adopt", "M"), pers(&mut db, "Dee Adopt", "F"));
        let kid = pers(&mut db, "Kid", "M");
        let eve = pers(&mut db, "Eve", "F");
        let f1 = fam(&mut db, Some(bob), Some(ann), &[kid], "1950");
        let f2 = fam(&mut db, Some(cal), Some(dee), &[kid, eve], "1955");
        db.fams.get_mut(&f2).unwrap().pedi.insert(kid, "adopted".into());
        (db, bob, ann, cal, dee, kid, eve, f1.max(f2))
    }

    #[test]
    fn biological_and_legal_parents_are_kept_apart() {
        let (db, bob, _, cal, _, kid, _, _) = adopted_tree();
        assert_eq!(db.parent_families(kid).len(), 2);
        assert_eq!(db.parents(kid).0, Some(bob), "the chart shows the birth parents by default");
        assert_eq!(db.relationship(kid, bob), "Bob Birth is Kid's father.");
        assert_eq!(db.relationship(kid, cal), "Cal Adopt is Kid's adoptive father.");
        assert_eq!(db.relationship(cal, kid), "Kid is Cal Adopt's adopted son.");
    }

    #[test]
    fn a_preferred_family_changes_what_the_chart_shows_without_changing_relationships() {
        let (mut db, bob, _, cal, _, kid, _, _) = adopted_tree();
        let adoptive = db.parent_families(kid).into_iter().find(|f| db.fams[f].h == Some(cal)).unwrap();
        db.people.get_mut(&kid).unwrap().pref_parents = Some(adoptive);
        assert_eq!(db.parents(kid).0, Some(cal));
        assert_eq!(db.relationship(kid, bob), "Bob Birth is Kid's father.");
        assert_eq!(db.parent_links(kid).len(), 4);
    }

    #[test]
    fn an_adopted_sibling_is_not_called_a_blood_relative() {
        let (db, _, _, _, _, kid, eve, _) = adopted_tree();
        let r = db.relationship(kid, eve);
        assert!(r.contains("sister") && r.contains("not a blood relationship"), "{}", r);
    }

    #[test]
    fn step_and_half_relatives_are_worked_out_from_partnerships() {
        let mut db = Db::default();
        let (mum, dad) = (pers(&mut db, "Mum", "F"), pers(&mut db, "Dad", "M"));
        let kid = pers(&mut db, "Kid", "M");
        let (sue, tim) = (pers(&mut db, "Stepmum Sue", "F"), pers(&mut db, "Tim", "M"));
        let half = pers(&mut db, "Half", "F");
        fam(&mut db, Some(dad), Some(mum), &[kid], "1980");
        fam(&mut db, Some(dad), Some(sue), &[half], "1990");
        let sue_kid = pers(&mut db, "Sue's other child", "M");
        let sue_prev = pers(&mut db, "Sue's first partner", "M");
        fam(&mut db, Some(sue_prev), Some(sue), &[sue_kid], "1985");
        let _ = tim;
        let k = db.kin_extras(kid);
        assert_eq!(k.step_parents, vec![sue]);
        assert_eq!(k.half_siblings, vec![half]);
        assert_eq!(k.step_siblings, vec![sue_kid]);
        assert_eq!(db.relationship(kid, sue), "Stepmum Sue is Kid's stepmother.");
        assert_eq!(db.relationship(kid, half), "Half is Kid's half-sister.");
        assert_eq!(db.relationship(kid, sue_kid), "Sue's other child is Kid's stepbrother.");
        let d = db.kin_extras(dad);
        assert_eq!(d.step_children, vec![sue_kid]);
        assert_eq!(db.relationship(sue_kid, kid), "Kid is Sue's other child's stepbrother.");
    }

    #[test]
    fn partners_are_ordered_by_marriage_and_named_by_partnership_type() {
        let mut db = Db::default();
        let (a, b, c) = (pers(&mut db, "A", "M"), pers(&mut db, "B", "F"), pers(&mut db, "C", "F"));
        let late = fam(&mut db, Some(a), Some(c), &[], "1990");
        let early = fam(&mut db, Some(a), Some(b), &[], "1970");
        let none = fam(&mut db, Some(a), None, &[], "");
        assert_eq!(db.fams_of(a), vec![early, late, none]);
        db.fams.get_mut(&early).unwrap().kind = "Unmarried".into();
        assert_eq!(db.relationship(a, b), "B is A's partner.");
        db.fams.get_mut(&early).unwrap().events.push(Event { kind: "Divorce".into(), ..Default::default() });
        assert!(db.relationship(a, b).contains("former"));
        assert_eq!(db.relationship(a, c), "C is A's wife.");
    }

    #[test]
    fn a_guardian_is_found_when_there_is_no_family_link() {
        let mut db = Db::default();
        let (ward, g) = (pers(&mut db, "Ward", "F"), pers(&mut db, "Gus", "M"));
        db.people.get_mut(&ward).unwrap().assoc.push(Assoc { other: g, role: "Guardian".into(), note: String::new() });
        assert!(db.relationship(ward, g).contains("Guardian"));
        assert!(db.relationship(g, ward).contains("Guardian of") || db.relationship(g, ward).contains("the Guardian of"));
        assert_eq!(db.kin_extras(g).assoc_of, vec![(ward, "Guardian".to_string())]);
    }

    #[test]
    fn same_sex_parents_need_no_husband_and_wife() {
        let mut db = Db::default();
        let (a, b, kid) = (pers(&mut db, "Al", "M"), pers(&mut db, "Bo", "M"), pers(&mut db, "Kid", "F"));
        fam(&mut db, Some(a), Some(b), &[kid], "2010");
        assert_eq!(db.parent_links(kid).len(), 2);
        assert_eq!(db.relationship(kid, b), "Bo is Kid's father.");
        assert_eq!(db.relationship(a, b), "Bo is Al's husband.");
    }

    #[test]
    fn add_parent_and_remove() {
        let mut db = Db::default();
        let kid = pers(&mut db, "Kid", "M");
        let a = pers(&mut db, "A", "F");
        let b = pers(&mut db, "B", "F");
        db.add_parent(kid, None, false, a);
        let f1 = db.famc(kid).unwrap();
        db.add_parent(kid, Some(f1), false, b); // replaces slot in same family
        assert_eq!(db.fams[&f1].w, Some(b));
        db.add_parent(kid, None, true, a); // separate second set
        assert_eq!(db.parent_families(kid).len(), 2);
        let second = *db.parent_families(kid).iter().find(|f| **f != f1).unwrap();
        db.fams.get_mut(&second).unwrap().frel.insert(kid, "Adopted".into());
        assert!(db.child_note(second, kid).contains("adopted"));
        db.remove_from_family(kid, second);
        assert_eq!(db.parent_families(kid), vec![f1]);
        assert!(db.fams[&second].frel.is_empty());
    }

    #[test]
    fn checker_flags_complex_family_problems() {
        let mut db = Db::default();
        let kid = pers(&mut db, "Kid", "M");
        let (a, b, c, d) = (pers(&mut db, "A", "M"), pers(&mut db, "B", "F"), pers(&mut db, "C", "M"), pers(&mut db, "D", "F"));
        fam(&mut db, Some(a), Some(b), &[kid], "");
        assert!(!db.check().iter().any(|(_, m)| m.contains("biological parents")));
        let f2 = fam(&mut db, Some(c), Some(d), &[kid], "");
        assert!(db.check().iter().any(|(_, m)| m.contains("more than one set of biological")));
        db.fams.get_mut(&f2).unwrap().pedi.insert(kid, "adopted".into());
        assert!(!db.check().iter().any(|(_, m)| m.contains("more than one set of biological")));
        // loop: A becomes child of Kid
        fam(&mut db, Some(kid), None, &[a], "");
        assert!(db.check().iter().any(|(_, m)| m.contains("own ancestor")));
        // duplicate partnership
        fam(&mut db, Some(a), Some(b), &[], "");
        assert!(db.check().iter().any(|(_, m)| m.contains("two separate family groups")));
    }
}

#[cfg(test)]
mod scale_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn media_names_cannot_reach_outside_the_media_folder() {
        for bad in ["../../etc/passwd", "C:\\Windows\\notepad.exe", "/Applications/Calculator.app", "a/b.jpg", "x:y.jpg", ""] {
            assert!(!mfile(bad).starts_with(std::path::Path::new("/etc")));
            assert!(mfile(bad).starts_with(media_dir()), "{bad}");
            assert!(mprev(bad).starts_with(media_dir()), "{bad}");
            assert!(mfile(bad).ends_with("-missing-"), "{bad}");
        }
        assert_eq!(mfile("photo.jpg"), media_dir().join("photo.jpg"));
        assert!(!is_media_name("setup.exe") && !is_media_name("run.bat") && !is_media_name("x.app"));
    }

    #[test]
    fn removing_online_links_and_finding_files_nobody_uses() {
        let mut db = Db::default();
        let mut a = Person { name: "A".into(), ..Default::default() };
        a.media.push(Media { path: "https://www.ancestry.com/x.jpg".into(), form: "jpg".into(), ..Default::default() });
        a.media.push(Media { file: "shared.jpg".into(), ..Default::default() });
        a.media.push(Media { file: "only-a.jpg".into(), ..Default::default() });
        let mut b = Person { name: "B".into(), ..Default::default() };
        b.media.push(Media { file: "shared.jpg".into(), ..Default::default() });
        b.media.push(Media { path: "https://www.myheritage.com/y.png".into(), form: "png".into(), ..Default::default() });
        db.people.insert(1, a);
        db.people.insert(2, b);
        // freed files: only ones nobody else uses (and that exist; none do here, so none are offered)
        let sel: HashSet<(u32, usize)> = [(1, 1), (1, 2)].into_iter().collect();
        assert!(files_freed_by(&db, &sel).is_empty());
        let still = {
            let mut d = db.clone();
            remove_media_items(&mut d, &sel);
            media_in_use(&d)
        };
        assert!(still.contains("shared.jpg") && !still.contains("only-a.jpg"), "a file B still uses stays in use");
        assert_eq!(remove_online_links(&mut db), 2);
        assert_eq!(db.people[&1].media.len(), 2);
        assert_eq!(db.people[&2].media.len(), 1);
        assert_eq!(remove_online_links(&mut db), 0);
    }

    #[test]
    fn removing_several_media_at_once() {
        let mut db = Db::default();
        let mut p = Person { name: "A".into(), ..Default::default() };
        for f in ["a.jpg", "b.jpg", "c.jpg", "d.pdf"] {
            p.media.push(Media { file: f.into(), ..Default::default() });
        }
        p.portrait = "c.jpg".into();
        db.people.insert(1, p);
        let sel: HashSet<(u32, usize)> = [(1, 0), (1, 2), (1, 9), (7, 0)].into_iter().collect();
        assert_eq!(remove_media_items(&mut db, &sel), 2);
        let left: Vec<&str> = db.people[&1].media.iter().map(|m| m.file.as_str()).collect();
        assert_eq!(left, vec!["b.jpg", "d.pdf"]);
        assert!(db.people[&1].portrait.is_empty(), "a removed profile photo is cleared");
    }

    #[test]
    #[ignore]
    fn snapshot_cost_like_users_tree() {
        let (mut db, _) = big_tree(4400);
        // about 5 citations, 1 event and 1 photo link per person, like an Ancestry import
        for p in db.people.values_mut() {
            for k in 0..5 {
                p.sources.push(Source { text: format!("1900 United States Federal Census, record {}", k), page: "Year: 1900; Census Place: Somewhere, Some County, Oregon; Roll: 1234; Page: 5; Enumeration District: 0067".into(), url: "https://www.ancestry.com/discoveryui-content/view/12345:7602".into(), extra: vec![Raw { at: String::new(), text: "1 _APID 1,7602::12345678\n".into() }], ..Default::default() });
            }
            p.events.push(Event { kind: "Residence".into(), date: "1910".into(), place: "Portland, Multnomah, Oregon, USA".into(), ..Default::default() });
            p.media.push(Media { path: "https://mediasvc.ancestry.com/v2/image/namespaces/1093/media/abc.jpg?Client=Trees".into(), form: "jpg".into(), caption: "Photo".into(), ..Default::default() });
        }
        let t = Instant::now();
        let j = serde_json::to_string(&db).unwrap();
        println!("json {} MB, serialise {:?}", j.len() / 1_000_000, t.elapsed());
        let t = Instant::now();
        let _ = j == j.clone();
        println!("compare {:?}", t.elapsed());
        let t = Instant::now();
        let z = pack(&j);
        println!("pack {:?} -> {} KB", t.elapsed(), z.len() / 1000);
        let t = Instant::now();
        assert_eq!(unpack(&z), j);
        println!("unpack {:?}", t.elapsed());
    }

    /// A realistic big tree: deep lines, large sibling groups, remarriages, adoptions.
    pub fn big_tree(n: usize) -> (Db, u32) {
        let mut db = Db::default();
        let surnames = ["Smith", "Jones", "Brown", "Taylor", "Wilson", "Davies", "Evans", "Thomas", "Johnson", "Roberts", "Walker", "Wright", "Robinson", "Thompson", "White", "Hughes", "Edwards", "Green", "Hall", "Wood"];
        let given = ["John", "Mary", "William", "Elizabeth", "James", "Sarah", "George", "Ann", "Thomas", "Jane", "Henry", "Margaret", "Charles", "Alice", "Robert", "Emma"];
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        let mut next = move |m: usize| -> usize {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng % m as u64) as usize
        };
        let mk = |db: &mut Db, sex: &str, year: i32, sn: &str, next: &mut dyn FnMut(usize) -> usize| -> u32 {
            let id = db.alloc();
            let g = given[next(given.len())];
            let name = format!("{} {}", g, sn);
            db.people.insert(id, Person { name: name.clone(), given: g.into(), surname: sn.into(), sex: sex.into(), bd: format!("{}", year), bp: "Somewhere".into(), ..Default::default() });
            id
        };
        // oldest generation: a few couples, then breed downward until n people exist
        let mut frontier: Vec<(u32, i32)> = Vec::new();
        let root = mk(&mut db, "M", 1700, "Smith", &mut next);
        frontier.push((root, 1700));
        let mut qi = 0;
        while db.people.len() < n && qi < frontier.len() {
            let (p, y) = frontier[qi];
            qi += 1;
            let sex = db.people[&p].sex.clone();
            let marriages = if next(6) == 0 { 2 } else { 1 };
            for _ in 0..marriages {
                if db.people.len() >= n {
                    break;
                }
                let sn = surnames[next(surnames.len())];
                let sp = mk(&mut db, if sex == "M" { "F" } else { "M" }, y + next(6) as i32 - 2, sn, &mut next);
                let fid = db.alloc();
                let (h, w) = if sex == "M" { (Some(p), Some(sp)) } else { (Some(sp), Some(p)) };
                let mut fam = Family { h, w, md: format!("{}", y + 22), ..Default::default() };
                let kids = 1 + next(6);
                for _ in 0..kids {
                    if db.people.len() >= n {
                        break;
                    }
                    let ksex = if next(2) == 0 { "M" } else { "F" };
                    let ky = y + 24 + next(12) as i32;
                    let hs = db.people[&h.unwrap()].surname.clone();
                    let k = mk(&mut db, ksex, ky, &hs, &mut next);
                    fam.kids.push(k);
                    if next(25) == 0 {
                        fam.pedi.insert(k, "adopted".into());
                    }
                    frontier.push((k, ky));
                }
                db.fams.insert(fid, fam);
            }
        }
        (db, root)
    }

    fn t<R>(label: &str, f: impl FnOnce() -> R) -> (R, f64) {
        let s = Instant::now();
        let r = f();
        let ms = s.elapsed().as_secs_f64() * 1000.0;
        println!("  {:<34} {:>9.1} ms", label, ms);
        (r, ms)
    }

    #[test]
    #[ignore]
    fn timings() {
        for n in [1000usize, 5000, 20000] {
            let (db, root) = big_tree(n);
            println!("{} people, {} families", db.people.len(), db.fams.len());
            let ids: Vec<u32> = db.people.keys().copied().step_by((db.people.len() / 200).max(1)).collect();
            t("index build (per frame)", || db.index().child_of.len());
            t("famc x200", || ids.iter().for_each(|i| { db.famc(*i); }));
            t("fams_of x200", || ids.iter().for_each(|i| { db.fams_of(*i); }));
            let deep = *db.people.keys().last().unwrap();
            t("ancestors_by (deepest)", || db.ancestors_by(deep, false).len());
            t("kin_extras x200", || ids.iter().for_each(|i| { db.kin_extras(*i); }));
            t("relationship (root, deepest)", || db.relationship(root, deep).len());
            t("descendants layout (8 gens)", || {
                let mut nodes = Vec::new();
                let mut leaf = 0.0;
                let mut seen: HashSet<u32> = HashSet::from([root]);
                let ix = db.index();
                desc_layout(&db, &ix, root, 0, 8, None, &mut nodes, &mut leaf, &mut seen);
                nodes.len()
            });
            t("sorted(\"\") list", || db.sorted("").len());
            t("sorted(\"smith\") search", || db.sorted("smith").len());
            t("data checker", || db.check().len());
            t("duplicate finder", || db.duplicate_pairs().len());
            let (json, _) = t("save to JSON", || serde_json::to_string(&db).unwrap());
            t("load from JSON", || serde_json::from_str::<Db>(&json).unwrap().people.len());
            let (ged, _) = t("export GEDCOM", || gedcom::to_ged(&db));
            t("import GEDCOM", || gedcom::parse_ged_report(&ged).0.people.len());
        }
    }

    #[test]
    fn index_answers_match_the_slow_scans() {
        let (db, _) = big_tree(3000);
        let ix = db.index();
        for id in db.people.keys().step_by(7) {
            assert_eq!(db.parent_families_ix(&ix, *id), db.parent_families(*id));
            assert_eq!(db.fams_of_ix(&ix, *id).to_vec(), db.fams_of(*id));
            assert_eq!(db.parents_ix(&ix, *id), db.parents(*id));
        }
    }

    #[test]
    fn big_trees_stay_fast() {
        // Generous limits so slow machines and debug builds still pass; the old code took minutes.
        let (db, root) = big_tree(5000);
        let limit = 15.0;
        let (_, ms) = t("checker", || db.check().len());
        assert!(ms < limit * 1000.0, "data checker took {} ms", ms);
        let (_, ms) = t("duplicates", || db.duplicate_pairs().len());
        assert!(ms < limit * 1000.0, "duplicate finder took {} ms", ms);
        let (n, ms) = t("descendants", || {
            let mut nodes = Vec::new();
            let mut leaf = 0.0;
            let mut seen: HashSet<u32> = HashSet::from([root]);
            let ix = db.index();
            desc_layout(&db, &ix, root, 0, 8, None, &mut nodes, &mut leaf, &mut seen);
            nodes.len()
        });
        assert!(n > 100 && ms < limit * 1000.0);
        let (_, ms) = t("search", || db.sorted("smith").len());
        assert!(ms < limit * 1000.0);
    }

    #[test]
    fn navigation_helpers_find_family_members() {
        let mut db = Db::default();
        let mk = |db: &mut Db, n: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), ..Default::default() });
            id
        };
        let (dad, mum, step) = (mk(&mut db, "Dad"), mk(&mut db, "Mum"), mk(&mut db, "Stepmum"));
        let (a, b, c, half) = (mk(&mut db, "A"), mk(&mut db, "B"), mk(&mut db, "C"), mk(&mut db, "Half"));
        let f1 = db.alloc();
        db.fams.insert(f1, Family { h: Some(dad), w: Some(mum), kids: vec![a, b, c], ..Default::default() });
        let f2 = db.alloc();
        db.fams.insert(f2, Family { h: Some(dad), w: Some(step), kids: vec![half], ..Default::default() });
        assert_eq!(db.siblings(b), vec![a, c, half]);
        assert_eq!(db.partners(dad), vec![mum, step]);
        assert_eq!(db.children_of(dad), vec![a, b, c, half]);
        assert!(db.children_of(a).is_empty());
        assert!(db.siblings(dad).is_empty());
    }
}

#[cfg(test)]
mod edit_tests {
    use super::*;

    fn tiny() -> Db {
        let mut db = Db::default();
        for n in ["Ann Lee", "Bob Lee"] {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), given: n.split(' ').next().unwrap().into(), surname: "Lee".into(), ..Default::default() });
        }
        let f = db.alloc();
        let ids: Vec<u32> = db.people.keys().copied().collect();
        db.fams.insert(f, Family { h: Some(ids[1]), w: Some(ids[0]), ..Default::default() });
        db
    }

    #[test]
    fn pedigree_double_click_opens_person() {
        let mut app = App::for_test(tiny());
        let root = *app.db.people.keys().next().unwrap();
        app.sel = Some(root);
        app.view = View::Pedigree;
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0));
        let frame = |app: &mut App, events: Vec<egui::Event>, t: f64| -> Option<Action> {
            let mut out = None;
            let input = egui::RawInput { screen_rect: Some(screen), events, time: Some(t), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut act = None;
                    app.pedigree(ui, &mut act);
                    out = act;
                });
            });
            out
        };
        // give the root parents and grandparents so outer boxes exist too
        let mk = |db: &mut Db, n: &str, sx: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), sex: sx.into(), portrait: String::new(), ..Default::default() });
            id
        };
        let (f, m) = (mk(&mut app.db, "Dad Lee", "M"), mk(&mut app.db, "Mum Lee", "F"));
        let (gf, gm) = (mk(&mut app.db, "Grandad Lee", "M"), mk(&mut app.db, "Gran Lee", "F"));
        let f1 = app.db.alloc();
        app.db.fams.insert(f1, Family { h: Some(f), w: Some(m), kids: vec![root], ..Default::default() });
        let f2 = app.db.alloc();
        app.db.fams.insert(f2, Family { h: Some(gf), w: Some(gm), kids: vec![f], ..Default::default() });
        frame(&mut app, vec![], 0.0);
        let mut t = 1.0;
        for (g, i, who) in [(1usize, 0usize, f), (1, 1, m), (2, 0, gf), (2, 1, gm)] {
            let r = ctx.read_response(egui::Id::new(("ped", g, i))).expect("box drawn");
            let p = pos2(r.rect.min.x + r.rect.height() + 30.0, r.rect.center().y);
            let press = |down: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
            let mut got = Vec::new();
            for (ev, dt) in [(vec![egui::Event::PointerMoved(p)], 0.0), (vec![press(true)], 0.1), (vec![press(false)], 0.15), (vec![press(true)], 0.2), (vec![press(false)], 0.25)] {
                got.push(frame(&mut app, ev, t + dt));
            }
            t += 2.0;
            assert!(got.iter().any(|a| matches!(a, Some(Action::Open(x)) if *x == who)), "box {:?} did not open", (g, i));
        }
        let r = ctx.read_response(egui::Id::new(("ped", 0usize, 0usize))).expect("root box drawn");
        // click on the name, a little right of the portrait
        let p = pos2(r.rect.min.x + r.rect.height() + 30.0, r.rect.center().y);
        let press = |down: bool| egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
        frame(&mut app, vec![egui::Event::PointerMoved(p)], t);
        let a1 = frame(&mut app, vec![press(true)], t + 0.2);
        let a2 = frame(&mut app, vec![press(false)], t + 0.25);
        let a3 = frame(&mut app, vec![press(true)], t + 0.3);
        let a4 = frame(&mut app, vec![press(false)], t + 0.35);
        let all = [a1, a2, a3, a4];
        let names: Vec<String> = all.iter().map(|a| match a { Some(Action::Open(_)) => "open".into(), Some(Action::Highlight(_)) => "hl".into(), Some(_) => "other".into(), None => "-".into() }).collect();
        assert!(all.iter().any(|a| matches!(a, Some(Action::Open(i)) if *i == root)), "got {:?}", names);
    }

    #[test]
    fn image_addresses_keep_spaces() {
        let p = std::path::Path::new("/Users/me/Documents/Gilded Roots/media/previews/a b.jpg");
        assert_eq!(img_uri(p), "file:///Users/me/Documents/Gilded Roots/media/previews/a b.jpg");
    }

    #[test]
    fn f1_and_quick_jump_keys_work() {
        let mut app = App::for_test(tiny());
        let ctx = egui::Context::default();
        let key = |k: egui::Key, m: egui::Modifiers| egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m };
        for (ev, want) in [(key(egui::Key::F1, egui::Modifiers::NONE), true), (key(egui::Key::F1, egui::Modifiers::NONE), false), (key(egui::Key::Slash, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT), true)] {
            let input = egui::RawInput { events: vec![ev], modifiers: Default::default(), ..Default::default() };
            let _ = ctx.run(input, |ctx| app.global_keys(ctx, &mut None));
            assert_eq!(app.help_open, want);
        }
        // Ctrl/Cmd+K opens quick jump, typing a name finds the person, Enter selects them
        let target = *app.db.people.iter().find(|(_, p)| p.name == "Bob Lee").unwrap().0;
        let mut act = None;
        let input = egui::RawInput { events: vec![key(egui::Key::K, egui::Modifiers::COMMAND)], ..Default::default() };
        let _ = ctx.run(input, |ctx| app.jump_window(ctx, &mut act));
        assert!(app.x.jump_open);
        let r = app.jump_results("bob");
        assert!(matches!(r.first(), Some((_, k, Jump::Act(Action::Select(i)))) if k == "person" && *i == target));
        assert!(app.jump_results("pedigree").iter().any(|x| x.0 == "Pedigree view"));
    }

    #[test]
    fn places_are_listed_and_renamed_everywhere() {
        let mut db = tiny();
        let ids: Vec<u32> = db.people.keys().copied().collect();
        db.people.get_mut(&ids[0]).unwrap().bp = "Portland, Oregon, USA".into();
        db.people.get_mut(&ids[1]).unwrap().dp = "portland, oregon".into();
        let f = *db.fams.keys().next().unwrap();
        db.fams.get_mut(&f).unwrap().mp = "Portland, Oregon, USA".into();
        let uses = db.place_uses();
        assert_eq!(uses["Portland, Oregon, USA"].len(), 3); // birth + marriage for both partners
        assert_eq!(place_key("Portland, Oregon, USA"), place_key("portland, oregon"));
        assert_eq!(db.rename_place("portland, oregon", "Portland, Oregon, USA"), 1);
        assert_eq!(db.place_uses().len(), 1);
    }

    #[test]
    fn face_tags_follow_people_and_crop_without_stretching() {
        // a tall box on a wide picture grows sideways to fill a square frame
        let uv = crop_uv([0.4, 0.2, 0.1, 0.3], 2.0, 1.0);
        assert!((uv.width() * 2.0 - uv.height()).abs() < 1e-4, "{:?}", uv);
        assert!(uv.min.x >= 0.0 && uv.max.x <= 1.0 && uv.min.y >= 0.0 && uv.max.y <= 1.0);
        let mut app = App::for_test(tiny());
        let ids: Vec<u32> = app.db.people.keys().copied().collect();
        let (owner, tagged) = (ids[0], ids[1]);
        app.db.people.get_mut(&owner).unwrap().media.push(Media { file: "x.jpg".into(), faces: vec![Face { who: tagged, r: [0.1, 0.1, 0.2, 0.2] }], ..Default::default() });
        app.db.people.get_mut(&tagged).unwrap().face_portrait = Some(FacePortrait { owner, file: "x.jpg".into(), r: [0.1, 0.1, 0.2, 0.2] });
        assert_eq!(app.tagged_in(tagged).len(), 1);
        // deleting the tagged person removes the tag
        replace_pointer(&mut app.db, &format!("I{}", tagged), None);
        assert!(app.db.people[&owner].media[0].faces.is_empty());
        // tags survive save and reload
        app.db.people.get_mut(&owner).unwrap().media[0].faces.push(Face { who: owner, r: [0.0, 0.0, 0.5, 0.5] });
        let j = serde_json::to_string(&app.db).unwrap();
        let back: Db = serde_json::from_str(&j).unwrap();
        assert_eq!(back.people[&owner].media[0].faces.len(), 1);
    }

    #[test]
    fn every_view_and_window_draws_on_a_big_tree() {
        let (db, root) = scale_tests::big_tree(3000);
        let mut app = App::for_test(db);
        app.sel = Some(root);
        // a deeper person, so the fan has ancestors
        let deep = *app.db.people.keys().max().unwrap();
        app.db.people.get_mut(&deep).unwrap().todos.push(Todo { text: "Find census".into(), ..Default::default() });
        app.db.bookmarks.push(deep);
        app.x.todo_open = true;
        app.x.places_open = true;
        app.x.history_open = true;
        app.x.ver_open = true;
        app.x.jump_open = true;
        app.help_open = true;
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        for sel in [root, deep] {
            app.sel = Some(sel);
            for v in [View::Pedigree, View::Family, View::Descendants, View::Person, View::Gallery, View::Charts, View::Table, View::Map, View::Memories, View::Dna] {
                for chart in [ChartKind::Fan, ChartKind::Hourglass, ChartKind::Lifespans] {
                    app.view = v;
                    app.x.chart = chart;
                    let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
                    let _ = ctx.run(input, |ctx| {
                        let mut act = None;
                        egui::CentralPanel::default().show(ctx, |ui| match v {
                            View::Pedigree => app.pedigree(ui, &mut act),
                            View::Family => app.family_view(ui, &mut act),
                            View::Descendants => app.descendants(ui, &mut act),
                            View::Person => app.person_view(ui, &mut act),
                            View::Gallery => app.gallery_view(ui, &mut act),
                            View::Charts => app.charts_view(ui, &mut act),
                            View::Table => app.table_view(ui, &mut act),
                            View::Map => app.map_view(ui, &mut act),
                            View::Memories => app.memories_view(ui, &mut act),
                            View::Dna => app.dna_view(ui, &mut act),
                        });
                        app.jump_window(ctx, &mut act);
                        app.todo_window(ctx, &mut act);
                        app.places_window(ctx, &mut act);
                        app.history_window(ctx);
                        app.versions_window(ctx);
                    });
                    if v != View::Charts {
                        break;
                    }
                }
            }
        }
        assert!(app.chart_page().is_some());
        // the table sorts and filters
        app.x.table_col = table::Col::Born;
        let rows = app.table_rows();
        assert_eq!(rows.len(), app.db.people.len());
        app.x.table_only = table::Only::OpenTasks;
        assert_eq!(app.table_rows().len(), 1);
        app.x.table_only = table::Only::Bookmarked;
        assert_eq!(app.table_rows()[0].id, deep);
    }

    #[test]
    fn history_describes_changes_and_undoes_them() {
        let mut app = App::for_test(tiny());
        let id = *app.db.people.keys().next().unwrap();
        app.sel = Some(id);
        app.remember_state();
        app.db.people.get_mut(&id).unwrap().bd = "1900".into();
        app.last_snap = std::time::Instant::now() - std::time::Duration::from_secs(5);
        app.track();
        assert_eq!(app.undo.len(), 1);
        assert!(app.undo[0].label.starts_with("Changed "), "{}", app.undo[0].label);
        let nid = app.db.alloc();
        app.db.people.insert(nid, Person { name: "New One".into(), ..Default::default() });
        app.last_snap = std::time::Instant::now() - std::time::Duration::from_secs(5);
        app.track();
        assert!(app.undo[1].label.starts_with("Added New One"), "{}", app.undo[1].label);
        app.undo_action();
        app.undo_action();
        assert!(app.db.people[&id].bd.is_empty());
        assert!(!app.db.people.contains_key(&nid));
        app.redo_action();
        assert_eq!(app.db.people[&id].bd, "1900");
    }

    #[test]
    fn name_and_date_styles_only_change_the_display() {
        let p = Person { name: "John Henry Smith".into(), given: "John Henry".into(), surname: "Smith".into(), bd: "12 Mar 1921".into(), ..Default::default() };
        assert_eq!(styled_name_as(&p, 1), "SMITH, John Henry");
        assert_eq!(styled_name_as(&p, 2), "Smith, John Henry");
        assert_eq!(styled_name_as(&p, 0), "John Henry Smith");
        assert_eq!(show_date_as("12 Mar 1921", 2), "March 12, 1921");
        assert_eq!(show_date_as("about 1850", 2), "about 1850");
        assert_eq!(show_date_as("12 MAR 1921", 3), "1921-03-12");
        assert_eq!(show_date_as("12 Mar 1921", 0), "12 Mar 1921");
        assert_eq!(p.bd, "12 Mar 1921");
    }

    #[test]
    fn pdf_viewer_reads_and_draws_pages() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests_data/two_pages.pdf");
        let v = pdfview::PdfView::open(&path, "Certificate").expect("opens");
        assert_eq!(v.pages, 2);
        let bytes = std::sync::Arc::new(std::fs::read(&path).unwrap());
        let (_, img) = pdfview::render_page(&bytes, 0, 1.0).expect("draws");
        assert_eq!(img.size, [612, 792]);
        // white paper with the filled black rectangle (300 x 200 points) and the text drawn on it
        let dark = img.pixels.iter().filter(|c| c.r() < 40 && c.g() < 40 && c.b() < 40).count();
        let white = img.pixels.iter().filter(|c| c.r() > 245 && c.g() > 245 && c.b() > 245).count();
        assert!((60_000..80_000).contains(&dark), "{} dark pixels", dark);
        assert!(white > 350_000, "only {} white pixels", white);
        let img_bytes: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
        let _ = image::RgbImage::from_raw(612, 792, img_bytes).unwrap().save("/tmp/pdf_page1.png");
        // a file that is not a PDF gives a plain message instead of a crash
        let bad = std::env::temp_dir().join("not_a.pdf");
        std::fs::write(&bad, b"hello").unwrap();
        assert!(pdfview::PdfView::open(&bad, "x").is_err());
        // drawing the viewer window does not panic
        let mut v = v;
        let ctx = egui::Context::default();
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                v.show(ctx);
            });
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }

    #[test]
    #[ignore]
    fn write_fan_chart_svg_for_a_look() {
        let (db, _) = scale_tests::big_tree(800);
        let mut app = App::for_test(db);
        let deep = *app.db.people.keys().max().unwrap();
        app.sel = Some(deep);
        app.x.chart = ChartKind::Fan;
        app.x.fan_gens = 6;
        for deg in [240.0, 360.0] {
            app.x.fan_deg = deg;
            let (title, body) = app.chart_page().unwrap();
            std::fs::write(format!("/tmp/fan{}.html", deg as u32), page(&title, &body, true)).unwrap();
        }
    }

    #[test]
    #[ignore]
    fn write_print_pages_for_a_look() {
        let (mut db, _) = scale_tests::big_tree(300);
        let id = *db.people.keys().find(|k| !db.fams_of(**k).is_empty() && db.parents(**k).0.is_some()).unwrap();
        {
            let p = db.people.get_mut(&id).unwrap();
            p.bd = "Jan 25 1975".into();
            p.bp = "Salt Lake City, Salt Lake, Utah, USA".into();
            p.notes = "A long note about this person that goes on for a while to see how notes wrap on paper.".into();
            for k in 0..3 {
                p.sources.push(Source { about: "Birth".into(), text: "Ancestro.example, U.S., Newspapers Obituary Index, 1800s-current. The Marysville Advocate; Publication Date: 28 May 1981; URL: https://www.newspapers.example/image/345662540/?article=f03a0f47-8903-45c1-a57e-d43e1619f152&focus=0.34905595,0.03336283,0.49968433,0.2853241&xid=3355".into(), ..Default::default() });
                let _ = k;
            }
        }
        let mut app = App::for_test(db);
        app.settings.hide_living = false;
        app.sel = Some(id);
        std::fs::write("/tmp/p_family.html", app.family_html(id)).unwrap();
        std::fs::write("/tmp/p_pedigree.html", app.pedigree_html(id)).unwrap();
        std::fs::write("/tmp/p_ancestors.html", app.ancestors_html(id)).unwrap();
        std::fs::write("/tmp/p_book.html", app.book_html(id)).unwrap();
    }

    #[test]
    fn map_finds_places_the_way_people_write_them() {
        let g = mapview::geo();
        let near = |p: &str, lon: f32, lat: f32| {
            let (ll, pr) = g.locate(p).unwrap_or_else(|| panic!("{} not found", p));
            assert!((ll[0] - lon).abs() < 0.6 && (ll[1] - lat).abs() < 0.6, "{} -> {:?}", p, ll);
            pr
        };
        assert!(near("Eugene, Lane, Oregon, USA", -123.09, 44.05) == mapview::Prec::Town);
        near("North Platte, Lincoln, Nebraska, USA", -100.77, 41.12);
        near("Salt Lake City, Salt Lake, Utah, USA", -111.89, 40.76);
        near("Springfield, Lane, Oregon", -123.02, 44.05);
        near("Portland, Oregon", -122.68, 45.52);
        near("Portland, Maine, United States", -70.26, 43.66);
        near("Paris, France", 2.35, 48.85);
        near("London, England", -0.13, 51.51);
        near("St. Louis, Missouri", -90.2, 38.63);
        // a farm that is not a town: the state's middle is used
        assert!(near("Old Smith Farm, Utah, USA", -111.7, 39.3) == mapview::Prec::State);
        assert!(g.locate("Nowhereville").is_none());
    }

    #[test]
    fn map_and_memories_draw() {
        let (db, root) = scale_tests::big_tree(500);
        let mut app = App::for_test(db);
        app.sel = Some(root);
        for p in app.db.people.values_mut() {
            p.bp = "Eugene, Lane, Oregon, USA".into();
            p.notes = "A note".into();
        }
        let ctx = egui::Context::default();
        for who in 0..4u8 {
            app.x.map_who = who;
            for v in [View::Map, View::Memories, View::Gallery] {
                let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))), ..Default::default() };
                let _ = ctx.run(input, |ctx| {
                    let mut act = None;
                    egui::CentralPanel::default().show(ctx, |ui| match v {
                        View::Map => app.map_view(ui, &mut act),
                        View::Memories => app.memories_view(ui, &mut act),
                        _ => app.gallery_view(ui, &mut act),
                    });
                });
            }
        }
        assert!(!app.map_pins().is_empty());
    }

    #[test]
    fn damaged_gedcom_files_never_crash_the_import() {
        let base = "0 HEAD\n1 CHAR UTF-8\n0 @I1@ INDI\n1 NAME John /Smith/\n1 BIRT\n2 DATE 12 MAR 1900\n2 PLAC Eugene, Oregon\n1 FAMS @F1@\n1 OBJE\n2 FILE ../../etc/passwd\n0 @I2@ INDI\n1 NAME Mary /Jones/\n1 FAMS @F1@\n0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 CHIL @I3@\n1 MARR\n2 DATE ABT 1925\n0 @S1@ SOUR\n1 TITL Census\n0 TRLR\n";
        let bytes = base.as_bytes();
        let mut rng: u64 = 0x2545F4914F6CDD1D;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for _ in 0..3000 {
            let mut b = bytes.to_vec();
            for _ in 0..(next() % 12 + 1) {
                let i = (next() as usize) % b.len();
                match next() % 4 {
                    0 => b[i] = (next() % 256) as u8,
                    1 => {
                        b.remove(i);
                    }
                    2 => {
                        let set = b"@0123 \n\r\t/:";
                        b.insert(i, set[(next() as usize) % set.len()])
                    }
                    _ => {
                        let j = (next() as usize) % b.len();
                        let chunk: Vec<u8> = b[i.min(j)..i.max(j)].to_vec();
                        b.splice(i..i, chunk);
                    }
                }
                if b.is_empty() {
                    b.push(b'0');
                }
            }
            let (text, _) = gedcom::decode_ged(&b);
            let (db, _) = gedcom::parse_ged_report(&text);
            let _ = gedcom::to_ged(&db);
            let _ = db.check();
        }
        // a GEDCOM that names a private file is not copied in
        let (mut db, mut rep) = gedcom::parse_ged_report(base);
        resolve_media(&mut db, std::path::Path::new("/tmp"), &mut rep);
        assert!(db.people.values().all(|p| p.media.iter().all(|m| m.file.is_empty())));
        assert!(!is_media_name("passwd") && is_media_name("grandma.JPG"));
        assert!(safe_media_path("../x.txt").is_none() && safe_media_path("a/b.txt").is_none() && safe_media_path("123-story.txt").is_some());
    }

    #[test]
    fn archive_keeps_everything_and_rejects_bad_files() {
        let mut db = sample_family();
        db.journal.push(Journal { goal: "x".into(), ..Default::default() });
        let dir = std::env::temp_dir().join(format!("gr-arch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test archive.zip");
        let info = archive::write_archive(&db, &path).expect("archive written");
        assert_eq!(info.people, db.people.len());
        assert!(info.media >= 8, "media {}", info.media);
        let back = archive::read_info(&path).unwrap();
        assert_eq!(back.people, db.people.len());
        let (db2, n) = archive::read_archive(&path).unwrap();
        assert_eq!(serde_json::to_string(&db2).unwrap(), serde_json::to_string(&db).unwrap());
        assert!(n >= 8);
        // the GEDCOM inside reads back with everyone in it
        let mut z = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut ged = String::new();
        std::io::Read::read_to_string(&mut z.by_name("family.ged").unwrap(), &mut ged).unwrap();
        assert_eq!(gedcom::parse_ged(&ged).people.len(), db.people.len());
        // not a zip, and a zip with a damaged tree, are refused with a message
        std::fs::write(dir.join("bad.zip"), b"hello").unwrap();
        assert!(archive::read_info(&dir.join("bad.zip")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn practice_mode_never_touches_the_real_tree() {
        let mut app = App::for_test(tiny());
        let before = serde_json::to_string(&app.db).unwrap();
        app.start_practice();
        assert!(app.db.people.len() >= 20);
        assert!(app.db.people.values().any(|p| !p.media.is_empty()));
        assert!(!app.db.journal.is_empty());
        // edits in practice are thrown away
        let some = *app.db.people.keys().next().unwrap();
        app.db.people.get_mut(&some).unwrap().name = "Changed".into();
        assert!(app.persist());
        app.end_practice();
        assert_eq!(serde_json::to_string(&app.db).unwrap(), before);
        // and the welcome screen draws
        let mut app = App::for_test(Db::default());
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut act = None;
                app.welcome(ui, &mut act);
            });
        });
    }

    #[test]
    fn confidence_journal_and_history_round_trip() {
        let mut db = sample_family();
        let t = *db.people.iter().find(|(_, p)| p.name == "Thomas James Harwood").unwrap().0;
        assert_eq!(db.people[&t].fx["DEAT"].conf, "confirmed");
        let ged = gedcom::to_ged(&db);
        assert!(ged.contains("_CONF confirmed"), "confidence is exported");
        let back = gedcom::parse_ged(&ged);
        let t2 = back.people.values().find(|p| p.name == "Thomas James Harwood").unwrap();
        assert_eq!(t2.fx["DEAT"].conf, "confirmed");
        assert!(t2.events.iter().any(|e| e.conf == "confirmed"));
        // journal and history survive saving
        db.history.push((1911, 1911, "Mine fire".into(), String::new()));
        let j: Db = serde_json::from_str(&serde_json::to_string(&db).unwrap()).unwrap();
        assert_eq!(j.journal.len(), db.journal.len());
        // timeline history: a US-born man of 1898-1975 sees both world wars and the depression
        let app = App::for_test(j);
        let h = app.history_for(&app.db.people[&t]);
        assert!(h.iter().any(|x| x.1.contains("First World War")));
        assert!(h.iter().any(|x| x.1.contains("Great Depression")));
        assert!(!h.iter().any(|x| x.1.contains("Irish")));
        assert!(!h.iter().any(|x| x.1.contains("Mayflower")));
    }

    #[test]
    fn duplicates_found_through_nicknames_and_spellings() {
        let mut db = Db::default();
        let mk = |db: &mut Db, g: &str, s: &str, bd: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: format!("{} {}", g, s), given: g.into(), surname: s.into(), bd: bd.into(), bp: "Leeds, England".into(), ..Default::default() });
            id
        };
        let a = mk(&mut db, "William", "Smith", "1850");
        let b = mk(&mut db, "Bill", "Smyth", "1850");
        let c = mk(&mut db, "Margaret", "Jones", "1901");
        let d = mk(&mut db, "Peggy", "Jones", "1902");
        let e = mk(&mut db, "Jonathan", "Brown", "1900");
        let f = mk(&mut db, "Jonathon", "Brown", "1900");
        let _other = mk(&mut db, "Mary", "Smith", "1850");
        let pairs = db.duplicate_pairs();
        let has = |x: u32, y: u32| pairs.iter().any(|p| (p.0 == x && p.1 == y) || (p.0 == y && p.1 == x));
        assert!(has(a, b), "{:?}", pairs);
        assert!(has(c, d));
        assert!(has(e, f));
        assert_eq!(sound_key("Smith"), sound_key("Smyth"));
        assert_eq!(canon_given("Bill"), "william");
    }

    #[test]
    fn statistics_designer_and_trees() {
        let db = sample_family();
        let st = stats::compute(&db);
        assert!(st.life_men.1 > 0 && st.surnames.iter().any(|s| s.0 == "Harwood"));
        assert!(st.moved.1 > 0);
        let mut app = App::for_test(db);
        app.sel = app.db.people.iter().find(|(_, p)| p.name == "Emily Rose Harwood").map(|(k, _)| *k);
        let mut d = designer::Design { who: 1, ..Default::default() };
        d.cols.push("father".into());
        d.hide_living = false;
        let html = app.design_html(&d);
        assert!(html.contains("Thomas James Harwood") && html.contains("James Harwood"));
        assert!(!html.contains("Ruth Ellen Harwood"), "ancestors only");
        let csv = app.design_csv(&d);
        assert!(csv.lines().count() >= 10);
        assert_eq!(sanitize_tree_name("../Mom's side: Jensen"), "Mom's side Jensen");
        assert_eq!(sanitize_tree_name("  .. "), "");
    }

    #[test]
    fn sibling_and_quick_edit() {
        let mut app = App::for_test(tiny());
        let ids: Vec<u32> = app.db.people.keys().copied().collect();
        let me = ids[0];
        let mut d = Dlg::new(Kind::Sibling, "M", Some(me));
        d.name = "Sib Lee".into();
        app.create(d);
        let sib = *app.db.people.iter().find(|(_, p)| p.name == "Sib Lee").unwrap().0;
        assert!(app.db.siblings(me).contains(&sib));
        let q = QuickEdit { id: sib, given: "Sam".into(), surname: "Lee".into(), sex: "M".into(), bd: "1 Jan 1900".into(), bp: String::new(), dd: String::new(), dp: String::new(), living: false };
        app.apply_quick(&q);
        assert_eq!(app.db.people[&sib].name, "Sam Lee");
        assert_eq!(app.db.people[&sib].bd, "1 Jan 1900");
    }

    #[test]
    fn pedigree_and_gallery_draw_headless() {
        let mut app = App::for_test(tiny());
        app.sel = app.db.people.keys().next().copied();
        for v in [View::Pedigree, View::Gallery, View::Descendants] {
            app.view = v;
            let ctx = egui::Context::default();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut act = None;
                    match v {
                        View::Pedigree => app.pedigree(ui, &mut act),
                        View::Gallery => app.gallery_view(ui, &mut act),
                        _ => app.descendants(ui, &mut act),
                    }
                });
            });
        }
    }

    #[test]
    fn add_window_details_land_on_the_new_person() {
        let mut app = App::for_test(tiny());
        let mut d = Dlg::new(Kind::Person, "F", None);
        d.name = "Cara May Lee".into();
        d.bd = " 4 Jun 1950 ".into();
        d.bp = "Leeds".into();
        d.dd = "1 Jan 2010".into();
        d.living = true; // ignored because a death date was given
        d.notes = "note".into();
        app.create(d);
        let p = app.db.people.values().find(|p| p.name == "Cara May Lee").unwrap();
        assert_eq!((p.bd.as_str(), p.bp.as_str(), p.dd.as_str(), p.sex.as_str()), ("4 Jun 1950", "Leeds", "1 Jan 2010", "F"));
        assert!(!p.living);
        assert_eq!(p.surname, "Lee");
        assert_eq!(p.notes, "note");
        // the new person opens on their (locked) page
        assert!(app.editing.is_none());
    }

    #[test]
    fn cancel_puts_the_person_back_exactly_as_they_were() {
        let mut app = App::for_test(tiny());
        let id = app.sel.unwrap();
        let before = serde_json::to_string(&app.db).unwrap();
        app.editing = Some(EditSession { id, snapshot: before.clone() });
        // a cat walks on the keyboard
        app.db.people.get_mut(&id).unwrap().bd = "99999".into();
        app.db.people.get_mut(&id).unwrap().notes = "asdfgh".into();
        assert_ne!(serde_json::to_string(&app.db).unwrap(), before);
        let e = app.editing.take().unwrap();
        app.load_snapshot(e.snapshot);
        assert_eq!(serde_json::to_string(&app.db).unwrap(), before);
        assert_eq!(app.sel, Some(id));
    }

    /// Draw the Individual page headless, locked and unlocked, to make sure nothing panics.
    #[test]
    fn individual_page_draws_locked_and_unlocked() {
        let mut app = App::for_test(tiny());
        let id = app.sel.unwrap();
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        for editing in [false, true] {
            app.editing = if editing { Some(EditSession { id, snapshot: String::new() }) } else { None };
            let mut act: Option<Action> = None;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.person_view(ui, &mut act));
            });
            // nothing is changed just by drawing
            assert!(act.is_none());
        }
    }

    #[test]
    fn locked_page_ignores_typing() {
        let mut app = App::for_test(tiny());
        let id = app.sel.unwrap();
        let before = serde_json::to_string(&app.db).unwrap();
        let ctx = egui::Context::default();
        let mut act: Option<Action> = None;
        for _ in 0..3 {
            let mut input = egui::RawInput::default();
            input.events.push(egui::Event::Text("987654".into()));
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.person_view(ui, &mut act));
            });
        }
        assert_eq!(serde_json::to_string(&app.db).unwrap(), before, "typing changed a locked person {}", id);
    }

    /// Draw the Family tab headless, wide and narrow, for a person with parents, step-parents,
    /// two partnerships, an adopted child and an association.
    #[test]
    fn family_tab_draws_for_a_complex_family() {
        let mut db = tiny();
        let mk = |db: &mut Db, n: &str| {
            let id = db.alloc();
            db.people.insert(id, Person { name: n.into(), bd: "1950".into(), bp: "Leeds".into(), ..Default::default() });
            id
        };
        let me = mk(&mut db, "Me Lee");
        let (dad, mum, step, partner, partner2, kid, adopted) = (mk(&mut db, "Dad"), mk(&mut db, "Mum"), mk(&mut db, "Step Mum"), mk(&mut db, "Pat"), mk(&mut db, "Quinn"), mk(&mut db, "Kid"), mk(&mut db, "Adoptee"));
        let f1 = db.alloc();
        db.fams.insert(f1, Family { h: Some(dad), w: Some(mum), kids: vec![me], ..Default::default() });
        let f2 = db.alloc();
        let mut fam2 = Family { h: Some(dad), w: Some(step), kids: vec![me], ..Default::default() };
        fam2.frel.insert(me, "Step".into());
        fam2.mrel.insert(me, "Step".into());
        db.fams.insert(f2, fam2);
        let f3 = db.alloc();
        let mut fam3 = Family { h: Some(me), w: Some(partner), kids: vec![kid, adopted], md: "1975".into(), kind: "Married".into(), ..Default::default() };
        fam3.pedi.insert(adopted, "adopted".into());
        db.fams.insert(f3, fam3);
        let f4 = db.alloc();
        db.fams.insert(f4, Family { h: Some(me), w: Some(partner2), ..Default::default() });
        db.people.get_mut(&me).unwrap().assoc.push(Assoc { other: dad, role: "Guardian".into(), note: String::new() });
        let mut app = App::for_test(db);
        app.sel = Some(me);
        let ctx = egui::Context::default();
        for (w, fx) in [(1300.0f32, 0usize), (1300.0, 1), (600.0, 0)] {
            app.fx = fx;
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, 800.0)));
            let mut act: Option<Action> = None;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.family_view(ui, &mut act));
            });
            assert!(act.is_none());
        }
        // a person with no families at all
        app.sel = Some(partner2);
        app.db.fams.remove(&f4);
        let mut act = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.family_view(ui, &mut act));
        });
    }

    #[test]
    fn dock_panels_draw_wide_narrow_and_floating() {
        let mut app = App::for_test(tiny());
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        for d in app.settings.dock.iter_mut() {
            d.on = true;
        }
        let run = |app: &mut App, w: f32| {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, 900.0)));
            let mut act: Option<Action> = None;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.person_view(ui, &mut act));
            });
        };
        run(&mut app, 1400.0);
        run(&mut app, 600.0);
        app.settings.dock[0].float = true; // pop the first panel out
        run(&mut app, 1400.0);
        let id = app.sel.unwrap();
        app.editing = Some(EditSession { id, snapshot: String::new() });
        run(&mut app, 1400.0);
    }

    #[test]
    fn old_settings_files_get_the_default_dock() {
        let old = r#"{"dark":true,"accent":[59,111,212],"bg":[30,31,34],"gens":5,"hide_living":true,"mic":"","zoom":1.0}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.dock.len(), default_dock().len());
        assert!(s.dock.iter().any(|d| d.sec == "relationships" && d.on));
        assert!(s.dock_w > 200.0);
    }

    #[test]
    fn descendants_boxes_put_the_partner_on_a_second_line() {
        let mut db = tiny();
        let ids: Vec<u32> = db.people.keys().copied().collect();
        let (a, b) = (ids[0], ids[1]);
        db.people.get_mut(&a).unwrap().bd = "1900".into();
        let (l1, l2) = desc_lines(&db, a);
        assert!(l1.contains("Ann Lee") && l1.contains("1900"));
        let l2 = l2.expect("partner line");
        assert!(l2.starts_with("+ Bob Lee"), "{}", l2);
        // someone with no partner has a single line
        let c = db.alloc();
        db.people.insert(c, Person { name: "Solo".into(), ..Default::default() });
        assert!(desc_lines(&db, c).1.is_none());
        // the printed chart carries both lines
        let app = App::for_test(db.clone());
        let svg = app.descendant_svg(&db, a);
        assert!(svg.contains("Ann Lee") && svg.contains("+ Bob Lee"));
        let _ = b;
    }
}

#[cfg(test)]
mod lds_tests {
    use super::*;

    const PAF: &str = "0 HEAD\n1 SOUR PAF\n1 CHAR ANSEL\n0 @I1@ INDI\n1 NAME John /Smith/\n1 SEX M\n1 BAPL\n2 DATE 1 JAN 1880\n2 TEMP SLAKE\n2 STAT COMPLETED\n1 SLGC\n2 FAMC @F1@\n2 DATE 3 JAN 1880\n2 TEMP LOGAN\n1 FAMS @F2@\n1 FAMC @F1@\n0 @I2@ INDI\n1 NAME Jane /Doe/\n1 SEX F\n1 FAMS @F2@\n0 @I3@ INDI\n1 NAME Old /Smith/\n1 SEX M\n1 FAMS @F1@\n0 @F1@ FAM\n1 HUSB @I3@\n1 CHIL @I1@\n0 @F2@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 SLGS\n2 DATE 5 JUL 1875\n2 TEMP MANTI\n0 TRLR\n";

    #[test]
    fn ordinances_from_a_paf_file_land_on_their_lines() {
        let db = gedcom::parse_ged(PAF);
        let john = *db.people.iter().find(|(_, p)| p.given.starts_with("John")).unwrap().0;
        let app = App::for_test(db);
        let rows = app.lds_rows(john);
        let get = |label: &str| rows.iter().find(|r| r.label.starts_with(label)).unwrap();
        assert_eq!(get("Baptism").event.as_ref().unwrap().temp, "SLAKE");
        assert!(get("Endowment").event.is_none());
        let parents = get("Sealed to parents");
        assert!(parents.label.contains("Old Smith"), "{}", parents.label);
        assert_eq!(parents.event.as_ref().unwrap().temp, "LOGAN");
        let spouse = get("Sealed to spouse");
        assert!(spouse.label.contains("Jane Doe"), "{}", spouse.label);
        assert_eq!(spouse.event.as_ref().unwrap().temp, "MANTI");
        assert_eq!(temples::temple_label("MANTI"), "Manti, Utah, USA (MANTI)");
    }

    #[test]
    fn adding_ordinances_writes_them_to_gedcom() {
        let db = gedcom::parse_ged(PAF);
        let john = *db.people.iter().find(|(_, p)| p.given.starts_with("John")).unwrap().0;
        let mut app = App::for_test(db);
        // Take the sealing to parents away, then add it back from its line, as the panel does.
        let row = app.lds_rows(john).into_iter().find(|r| r.kind == "LDS child sealing").unwrap();
        app.lds_remove(john, &row);
        let row = app.lds_rows(john).into_iter().find(|r| r.kind == "LDS child sealing").unwrap();
        assert!(row.event.is_none());
        app.lds_add(john, &row);
        let row = app.lds_rows(john).into_iter().find(|r| r.kind == "LDS child sealing").unwrap();
        let e = app.lds_event_mut(john, &row).unwrap();
        e.temp = "SGEOR".into();
        e.stat = "COMPLETED".into();
        let endow = app.lds_rows(john).into_iter().find(|r| r.label == "Endowment").unwrap();
        app.lds_add(john, &endow);
        let out = gedcom::to_ged(&app.db);
        let fam = app.lds_rows(john).into_iter().find(|r| r.kind == "LDS child sealing").unwrap().fam.unwrap();
        assert!(out.contains(&format!("1 SLGC\n2 FAMC @F{fam}@\n2 TEMP SGEOR\n2 STAT COMPLETED")) || out.contains(&format!("1 SLGC\n2 TEMP SGEOR\n2 STAT COMPLETED\n2 FAMC @F{fam}@")), "{out}");
        assert!(out.contains("1 ENDL"), "{out}");
        // And it reads back onto the same line.
        let back = App::for_test(gedcom::parse_ged(&out));
        let john = *back.db.people.iter().find(|(_, p)| p.given.starts_with("John")).unwrap().0;
        let r = back.lds_rows(john).into_iter().find(|r| r.kind == "LDS child sealing").unwrap();
        assert_eq!(r.event.unwrap().temp, "SGEOR");
    }
}

#[cfg(test)]
mod order_tests {
    use super::*;

    // Pat married Bea (1960) after Ann (1950), but the file lists Bea first.
    const T: &str = "0 HEAD\n0 @I1@ INDI\n1 NAME Pat /Lee/\n1 SEX M\n1 FAMS @F2@\n1 FAMS @F1@\n0 @I2@ INDI\n1 NAME Ann /Fox/\n1 SEX F\n1 FAMS @F1@\n0 @I3@ INDI\n1 NAME Bea /Roe/\n1 SEX F\n1 FAMS @F2@\n0 @I4@ INDI\n1 NAME Cal /Lee/\n1 BIRT\n2 DATE 1958\n1 FAMC @F1@\n0 @I5@ INDI\n1 NAME Dee /Lee/\n1 FAMC @F1@\n0 @I6@ INDI\n1 NAME Eve /Lee/\n1 BIRT\n2 DATE 12 MAR 1952\n1 FAMC @F1@\n0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 MARR\n2 DATE 1950\n1 CHIL @I4@\n1 CHIL @I5@\n1 CHIL @I6@\n0 @F2@ FAM\n1 HUSB @I1@\n1 WIFE @I3@\n1 MARR\n2 DATE 1960\n0 TRLR\n";

    fn find(db: &Db, given: &str) -> u32 {
        *db.people.iter().find(|(_, p)| p.given == given).unwrap().0
    }
    fn spouse_names(db: &Db, id: u32) -> Vec<String> {
        db.fams_of(id).iter().map(|f| { let fm = &db.fams[f]; db.people[&fm.w.unwrap()].given.clone() }).collect()
    }
    fn kid_names(db: &Db, f: u32) -> Vec<String> {
        db.fams[&f].kids.iter().map(|k| db.people[k].given.clone()).collect()
    }

    #[test]
    fn spouse_order_comes_from_the_file_and_can_be_changed() {
        let mut db = gedcom::parse_ged(T);
        let pat = find(&db, "Pat");
        assert_eq!(spouse_names(&db, pat), ["Bea", "Ann"], "the file's order is kept");
        assert_eq!(db.fams_of_ix(&db.index(), pat).iter().map(|f| db.people[&db.fams[f].w.unwrap()].given.clone()).collect::<Vec<_>>(), ["Bea", "Ann"]);
        let ann_fam = db.fams_of(pat)[1];
        assert!(db.move_family(pat, ann_fam, true));
        assert_eq!(spouse_names(&db, pat), ["Ann", "Bea"]);
        assert!(!db.move_family(pat, ann_fam, true), "already first");
        // The order goes out in the file and comes back.
        let out = gedcom::to_ged(&db);
        let back = gedcom::parse_ged(&out);
        assert_eq!(spouse_names(&back, find(&back, "Pat")), ["Ann", "Bea"]);
        // In date order, nothing extra is kept.
        assert!(back.people[&find(&back, "Pat")].fam_order.is_empty());
    }

    #[test]
    fn children_move_and_sort_by_birth() {
        let mut db = gedcom::parse_ged(T);
        let pat = find(&db, "Pat");
        let f = *db.fams_of(pat).iter().find(|f| db.fams[f].kids.len() == 3).unwrap();
        assert_eq!(kid_names(&db, f), ["Cal", "Dee", "Eve"]);
        let eve = find(&db, "Eve");
        assert!(db.move_child(f, eve, true));
        assert_eq!(kid_names(&db, f), ["Cal", "Eve", "Dee"]);
        assert!(!db.move_child(f, eve, false) || kid_names(&db, f) == ["Cal", "Dee", "Eve"]);
        // Eve (1952) before Cal (1958); Dee has no date and stays after the child she followed.
        db.fams.get_mut(&f).unwrap().kids = vec![find(&db, "Cal"), find(&db, "Dee"), eve];
        assert!(db.sort_children(f));
        assert_eq!(kid_names(&db, f), ["Eve", "Cal", "Dee"]);
        assert!(!db.sort_children(f), "already in order");
        let out = gedcom::to_ged(&db);
        let back = gedcom::parse_ged(&out);
        let bf = *back.fams.iter().find(|(_, x)| x.kids.len() == 3).unwrap().0;
        assert_eq!(kid_names(&back, bf), ["Eve", "Cal", "Dee"]);
    }

    #[test]
    fn renumbering_keeps_the_spouse_order() {
        let mut db = gedcom::parse_ged(T);
        let pat = find(&db, "Pat");
        let before = spouse_names(&db, pat);
        let order = gedcom::renumber_order(&db, gedcom::RenumberOrder::Alphabetical);
        gedcom::renumber(&mut db, &order);
        assert_eq!(spouse_names(&db, find(&db, "Pat")), before);
    }
}

#[cfg(test)]
mod find_tests {
    use super::*;

    const T: &str = "0 HEAD\n0 @I1@ INDI\n1 NAME John /Smith/\n1 SEX M\n1 FAMS @F1@\n0 @I2@ INDI\n1 NAME Mary /Jones/\n1 SEX F\n1 FAMS @F1@\n0 @I3@ INDI\n1 NAME John /Brown/\n1 SEX M\n1 FAMS @F2@\n0 @I4@ INDI\n1 NAME Ann /Jones/\n1 SEX F\n1 FAMS @F2@\n0 @F1@ FAM\n1 HUSB @I1@\n1 WIFE @I2@\n1 MARR\n2 DATE 1900\n0 @F2@ FAM\n1 HUSB @I3@\n1 WIFE @I4@\n0 TRLR\n";

    #[test]
    fn find_a_couple_in_either_order() {
        let db = gedcom::parse_ged(T);
        let who = |f: &u32| db.people[&db.fams[f].h.unwrap()].name.clone();
        let r = db.find_couples("john", "mary");
        assert_eq!(r.iter().map(who).collect::<Vec<_>>(), ["John Smith"]);
        assert_eq!(db.find_couples("mary", "john smith"), r, "either order");
        assert_eq!(db.find_couples("john", "").len(), 2, "one box empty: every spouse");
        assert!(db.find_couples("john", "zelda").is_empty());
        assert!(db.find_couples(" ", "").is_empty());
    }

    #[test]
    fn find_by_id_number_and_old_id() {
        let mut db = gedcom::parse_ged(T);
        let mary = *db.people.iter().find(|(_, p)| p.given == "Mary").unwrap().0;
        assert_eq!(db.find_by_number(&format!("#{mary}")), vec![(mary, false)]);
        assert_eq!(db.find_by_number(&format!("I{mary}")), vec![(mary, false)]);
        assert!(db.find_by_number("999").is_empty());
        // After renumbering alphabetically, Mary's old number still finds her.
        let order = gedcom::renumber_order(&db, gedcom::RenumberOrder::Alphabetical);
        let map = gedcom::renumber(&mut db, &order);
        let now = map[&mary];
        let hits = db.find_by_number(&mary.to_string());
        assert!(hits.contains(&(now, true)), "{hits:?}");
    }
}

#[cfg(test)]
mod contact_tests {
    use super::*;

    #[test]
    fn contact_details_round_trip_and_stay_out_of_private_copies() {
        let t = "0 HEAD\n0 @I1@ INDI\n1 NAME Liv /Ing/\n1 ADDR 12 Elm St\n2 CONT Twin Falls, Idaho\n1 PHON 208-555-0100\n1 EMAIL liv@example.com\n1 WWW https://example.com\n0 TRLR\n";
        let db = gedcom::parse_ged(t);
        let p = db.people.values().next().unwrap();
        assert_eq!(p.contact.addr, "12 Elm St\nTwin Falls, Idaho");
        assert_eq!(p.contact.phone, "208-555-0100");
        assert_eq!(p.contact.email, "liv@example.com");
        assert_eq!(p.contact.url, "https://example.com");
        let out = gedcom::to_ged(&db);
        assert!(out.contains("1 ADDR 12 Elm St\n2 CONT Twin Falls, Idaho\n1 PHON 208-555-0100\n1 EMAIL liv@example.com\n1 WWW https://example.com"), "{out}");
        let back = gedcom::parse_ged(&out);
        assert!(back.people.values().next().unwrap().contact == p.contact);
        assert!(back.people.values().next().unwrap().extra.iter().all(|r| !r.text.contains("PHON")), "not kept twice");
    }
}
