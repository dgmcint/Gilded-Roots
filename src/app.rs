//! The App itself: its state, the dialogs it can show, actions and undo.

use super::*;

#[derive(PartialEq, Clone, Copy)]
pub(crate) enum View {
    Pedigree,
    Family,
    Descendants,
    Person,
    Gallery,
    Charts,
    Table,
    Map,
    Memories,
    Dna,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) enum Wipe {
    Sample,
    Empty,
    Import,
    Restore,
}


pub(crate) struct LinkReq {
    pub(crate) child: u32,
    pub(crate) fid: Option<u32>,
    pub(crate) father: bool,
    pub(crate) q: String,
}

/// Drop-down for how a child is related to one parent. Returns the new GEDCOM word when changed.
pub(crate) fn rel_combo(ui: &mut egui::Ui, id_key: impl std::hash::Hash, cur: &str) -> Option<String> {
    let mut out = None;
    egui::ComboBox::from_id_salt(id_key).selected_text(rel_name(cur)).width(120.0).show_ui(ui, |ui| {
        for (w, lbl) in PARENT_RELS {
            if ui.selectable_label(rel_name(cur) == *lbl, *lbl).clicked() {
                out = Some(if *w == "Natural" { String::new() } else { w.to_string() });
            }
        }
    });
    out
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Father,
    Mother,
    Spouse,
    Child(Option<u32>),
    Person,
    ParentIn(u32, bool),
    NewParent(bool),
    Sibling,
}

pub(crate) struct Recorder {
    pub(crate) _stream: cpal::Stream,
    pub(crate) buf: Arc<Mutex<Vec<i16>>>,
    pub(crate) channels: u16,
    pub(crate) rate: u32,
    pub(crate) started: std::time::Instant,
    pub(crate) person: u32,
    pub(crate) peak: Arc<AtomicU32>,
}

pub(crate) struct Dlg {
    pub(crate) kind: Kind,
    pub(crate) name: String,
    pub(crate) sex: String,
    pub(crate) target: Option<u32>,
    pub(crate) fresh: u8,
    pub(crate) bd: String,
    pub(crate) bp: String,
    pub(crate) dd: String,
    pub(crate) dp: String,
    pub(crate) bud: String,
    pub(crate) bup: String,
    pub(crate) living: bool,
    pub(crate) notes: String,
}

impl Dlg {
    pub(crate) fn new(kind: Kind, sex: &str, target: Option<u32>) -> Dlg {
        Dlg { kind, name: String::new(), sex: sex.to_string(), target, fresh: 2, bd: String::new(), bp: String::new(), dd: String::new(), dp: String::new(), bud: String::new(), bup: String::new(), living: false, notes: String::new() }
    }
}

/// The small "Quick edit" window opened from a chart box.
pub(crate) struct QuickEdit {
    pub(crate) id: u32,
    pub(crate) given: String,
    pub(crate) surname: String,
    pub(crate) sex: String,
    pub(crate) bd: String,
    pub(crate) bp: String,
    pub(crate) dd: String,
    pub(crate) dp: String,
    pub(crate) living: bool,
}

/// Screen state for the newer tools (kept together so App's long constructor stays readable).
#[derive(Default)]
pub(crate) struct Extra {
    /// A fingerprint of the tree as last noted for Undo, so checking for a change after a click
    /// does not need a whole new copy of the tree in memory.
    pub(crate) last_hash: u64,
    pub(crate) map_cache: Option<std::sync::Arc<mapview::MapShown>>,
    pub(crate) mem_cache: Option<(media::MemKey, std::sync::Arc<Vec<media::MemItem>>)>,
    /// Goes up whenever the tree is noted as changed, so long lists worked out from it (the
    /// People table, the map's pins, Memories) are kept until then instead of rebuilt each frame.
    pub(crate) data_ver: u64,
    pub(crate) table_cache: Option<(table::TableKey, std::sync::Arc<Vec<table::Row>>)>,
    pub(crate) sorted_cache: HashMap<String, (u64, std::sync::Arc<Vec<u32>>)>,
    /// When to check whether the last clicks or typing changed the tree (for Undo and saving).
    /// Checking waits until the typing pauses, so a big tree does not slow every keystroke.
    pub(crate) track_at: Option<std::time::Instant>,
    /// A save running in the background, so the window does not freeze while a big tree is written.
    pub(crate) save_job: Option<std::thread::JoinHandle<Result<usize, String>>>,
    /// Tools > Renumber people: open, the order chosen (0 family, 1 present, 2 alphabetical),
    /// and the preview for that order (first few people: old number, new number, name)
    pub(crate) renum_open: bool,
    /// LDS ordinances panel: the text typed into a temple list's search box
    pub(crate) temple_search: String,
    /// Find Individual: how to search (0 name or place, 1 ID number, 2 a couple), and the
    /// ID number and the two names typed for the other ways
    pub(crate) find_mode: u8,
    /// Ditto: the last date and the last place typed, for Ctrl+D (Cmd+D) in an empty box
    pub(crate) ditto_date: String,
    /// Special letters window: open, capitals, the text box last typed in, and a letter to
    /// type into it next frame
    pub(crate) letters_open: bool,
    /// Groups window: open, which group, the person to count from, the relationship rule
    /// (kind, generations, spouses, brothers and sisters), the field rule (field, test, value),
    /// a name being typed, and jobs to do (print, export, split off as a tree with its name)
    pub(crate) groups_open: bool,
    /// Date calculator (which kind, the two dates, an age) and the Soundex window
    pub(crate) calc_open: bool,
    /// Find and replace in notes: open, the words, options, and notes unticked (by place)
    pub(crate) nfr_open: bool,
    /// Source templates: which one, and what is typed in its boxes
    pub(crate) tpl_which: usize,
    pub(crate) tpl_vals: Vec<String>,
    pub(crate) nfr_find: String,
    pub(crate) nfr_with: String,
    pub(crate) nfr_case: bool,
    pub(crate) nfr_whole: bool,
    pub(crate) nfr_skip: HashSet<String>,
    pub(crate) calc_mode: u8,
    pub(crate) calc_a: String,
    pub(crate) calc_b: String,
    pub(crate) calc_age: String,
    pub(crate) soundex_open: bool,
    pub(crate) soundex_q: String,
    pub(crate) group_cur: usize,
    pub(crate) group_who: Option<u32>,
    pub(crate) group_rel: usize,
    pub(crate) group_gens: u32,
    pub(crate) group_spouses: bool,
    pub(crate) group_sibs: bool,
    pub(crate) group_field: usize,
    pub(crate) group_test: usize,
    pub(crate) group_val: String,
    pub(crate) group_name: String,
    pub(crate) group_print: Option<usize>,
    pub(crate) group_export: Option<usize>,
    pub(crate) group_split: Option<(usize, String)>,
    /// The People tab's "Group" filter: which group
    pub(crate) table_group: usize,
    pub(crate) letters_caps: bool,
    pub(crate) last_text_focus: Option<egui::Id>,
    pub(crate) pending_letter: Option<(String, egui::Id)>,
    pub(crate) ditto_place: String,
    pub(crate) find_id: String,
    pub(crate) find_a: String,
    pub(crate) find_b: String,
    pub(crate) renum_how: u8,
    pub(crate) renum_preview: Option<(u8, Vec<(u32, u32, String)>, usize)>,
    /// Gallery/Memories: choosing several items to remove at once
    pub(crate) gal_pick: bool,
    pub(crate) gal_sel: HashSet<(u32, usize)>,
    pub(crate) gal_confirm: bool,
    /// The remove box: also move the files that nobody else uses out of the media folder
    pub(crate) gal_del_files: bool,
    /// Files that removing the ticked items would leave unused (worked out once, when asked).
    pub(crate) gal_freed: Vec<String>,
    /// Asking before removing every link to a photo stored on a website
    pub(crate) gal_online_confirm: bool,
    pub(crate) jump_open: bool,
    pub(crate) jump_q: String,
    pub(crate) jump_sel: usize,
    pub(crate) todo_open: bool,
    pub(crate) todo_find: String,
    pub(crate) todo_done: bool,
    pub(crate) todo_new: String,
    pub(crate) places_open: bool,
    pub(crate) places_find: String,
    pub(crate) places_sel: Option<String>,
    pub(crate) places_rename: String,
    pub(crate) places_by_count: bool,
    pub(crate) history_open: bool,
    /// What the tree looked like at the last change, to describe the next one in History.
    pub(crate) prev_names: HashMap<u32, String>,
    pub(crate) prev_sel: Option<(u32, String)>,
    pub(crate) prev_fams: usize,
    /// A description an action leaves for the History list ("Changed place spelling...").
    pub(crate) next_label: Option<String>,
    /// Face tagging in the photo viewer: on/off, the box being drawn (0..1 of the picture), who to tag.
    pub(crate) tag_mode: bool,
    pub(crate) tag_drag: Option<(egui::Pos2, egui::Pos2)>,
    pub(crate) tag_box: Option<[f32; 4]>,
    pub(crate) tag_q: String,
    pub(crate) crop_mode: bool,
    pub(crate) pdf: Option<pdfview::PdfView>,
    pub(crate) pdf_req: Option<(PathBuf, String)>,
    pub(crate) licenses_open: bool,
    pub(crate) sized: bool,
    pub(crate) backup_open: bool,
    pub(crate) backup_check: f64,
    pub(crate) arch_job: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    pub(crate) arch_confirm: Option<(PathBuf, archive::ArchiveInfo)>,
    pub(crate) paf_help: bool,
    pub(crate) designer_open: bool,
    pub(crate) design: designer::Design,
    pub(crate) trees_open: bool,
    pub(crate) tree_new: String,
    pub(crate) tree_rename: Option<(String, String)>,
    pub(crate) stats_open: bool,
    pub(crate) stats: Option<((usize, usize), stats::Stats)>,
    pub(crate) map_surname: String,
    pub(crate) journal_open: bool,
    pub(crate) journal_new: Option<Journal>,
    pub(crate) journal_edit: Option<usize>,
    pub(crate) journal_find: String,
    pub(crate) hist_open: bool,
    pub(crate) hist_new: (i32, i32, String),
    /// While exploring the sample family: the real tree and selection, put back afterwards.
    pub(crate) practice: Option<(Db, Option<u32>, View)>,
    pub(crate) gal_kind: u8,
    pub(crate) map_who: u8,
    pub(crate) map_types: [bool; 5],
    pub(crate) map_year: i32,
    pub(crate) map_play: bool,
    pub(crate) map_tick: f64,
    pub(crate) map_undated: bool,
    pub(crate) map_zoom: f32,
    pub(crate) map_center: [f32; 2],
    pub(crate) map_fit: bool,
    pub(crate) map_pick: Option<(i32, i32)>,
    pub(crate) map_setting: Option<String>,
    pub(crate) geo_cache: HashMap<String, Option<([f32; 2], mapview::Prec)>>,
    pub(crate) story: Option<StoryEdit>,
    pub(crate) chart: ChartKind,
    pub(crate) fan_gens: usize,
    pub(crate) fan_deg: f32,
    pub(crate) fan_menu: Option<u32>,
    /// Data-checker results for the person on screen, and what they were worked out from.
    pub(crate) flags: Vec<String>,
    pub(crate) flags_key: (u32, usize, usize),
    pub(crate) table_find: String,
    pub(crate) table_col: table::Col,
    pub(crate) table_desc: bool,
    pub(crate) table_only: table::Only,
    pub(crate) layout_name: String,
    pub(crate) ver_open: bool,
    pub(crate) ver: Option<(String, Db, (Vec<u32>, Vec<u32>, Vec<u32>))>,
    /// Tools > Tombstone calculator.
    pub(crate) tomb: tombstone::Tomb,
    /// Reports > Forms > Blank census forms.
    pub(crate) census: census::CensusUi,
    /// The DNA tab.
    pub(crate) dna: dna::DnaUi,
    /// Y-DNA and mtDNA in the DNA tab.
    pub(crate) line: dnaline::LineUi,
}

/// One step of undo history: the whole tree as it was (packed small), what the next change was,
/// and when it happened.
pub(crate) struct Snap {
    pub(crate) z: Vec<u8>,
    pub(crate) label: String,
    pub(crate) at: std::time::Instant,
}

pub(crate) fn pack(json: &str) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(json.as_bytes(), 1)
}

pub(crate) fn unpack(z: &[u8]) -> String {
    miniz_oxide::inflate::decompress_to_vec(z).ok().and_then(|b| String::from_utf8(b).ok()).unwrap_or_default()
}

pub(crate) fn when_ago(t: std::time::Instant) -> String {
    let s = t.elapsed().as_secs();
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        _ => format!("{} h {} min ago", s / 3600, (s % 3600) / 60),
    }
}

/// What a quick-jump line does.
pub(crate) enum Jump {
    Act(Action),
    View(View),
    Window(&'static str),
}

/// The person being edited on the Individual page, and how they looked before Edit was pressed.
pub(crate) struct EditSession {
    pub(crate) id: u32,
    pub(crate) snapshot: String,
}

pub(crate) enum Action {
    Select(u32),
    Fit,
    ViewTree(u32, u8),
    QuickEdit(u32),
    EditStart,
    EditSave,
    EditCancel,
    SetHome(u32),
    ShowParents(u32),
    ShowParentsFor(u32, u32),
    GoHome,
    Add(Kind),
    Delete,
    Fx(usize),
    Open(u32),
    Highlight(u32),
    AddFor(u32, Kind),
    SetGens(usize),
    /// Turn the hand tool on the charts on or off
    SetHand(bool),
    Back,
    Forward,
    PrintPedigree,
    PrintFamily,
    AttachFiles,
    StartRec,
    StopRec,
    SetZoom(f32),
    ReportAncestors,
    ReportDescendants,
    ReportBirthdays,
    ReportSources,
    ReportAncestorBook,
    ReportDescendantBook,
    ReportBook,
    PrintDescendants,
    PrintChart,
    Bookmark(u32),
    /// Move a child earlier (true) or later in a family's list: (family, child, earlier)
    MoveChild(u32, u32, bool),
    /// Put a family's children in order of birth
    SortChildren(u32),
    /// Move one of a person's families (spouses) earlier or later: (person, family, earlier)
    MoveFamily(u32, u32, bool),
    /// Add a person to a group, or take them out: (group, person)
    GroupToggle(usize, u32),
    /// Open the Groups window counting from this person
    GroupsOpen(u32),
}

pub(crate) struct App {
    pub(crate) db: Db,
    pub(crate) sel: Option<u32>,
    pub(crate) view: View,
    pub(crate) search: String,
    pub(crate) fx: usize,
    pub(crate) dlg: Option<Dlg>,
    pub(crate) confirm: bool,
    pub(crate) status: String,
    pub(crate) dirty: bool,
    pub(crate) find_open: bool,
    pub(crate) settings: Settings,
    pub(crate) show_look: bool,
    pub(crate) show_loc: bool,
    pub(crate) review: Option<PersonReview>,
    pub(crate) imerge: Option<ImportMerge>,
    pub(crate) src_open: bool,
    pub(crate) src_sel: Option<u32>,
    pub(crate) src_tab: u8,
    pub(crate) src_filter: String,
    pub(crate) repo_sel: Option<u32>,
    pub(crate) src_pick: Option<u32>,
    pub(crate) src_merge: Option<u32>,
    pub(crate) src_confirm: bool,
    pub(crate) hl: Option<u32>,
    pub(crate) back: Vec<u32>,
    pub(crate) fwd: Vec<u32>,
    pub(crate) wipe: Option<Wipe>,
    pub(crate) show_about: bool,
    pub(crate) help_open: bool,
    pub(crate) help_find: String,
    pub(crate) move_data: bool,
    pub(crate) imp_open: bool,
    pub(crate) imp_src: usize,
    pub(crate) imp_replace: bool,
    pub(crate) rec: Option<Recorder>,
    pub(crate) rel_open: bool,
    pub(crate) rel_a: Option<u32>,
    pub(crate) rel_b: Option<u32>,
    pub(crate) viewing: Option<(u32, usize)>,
    pub(crate) undo: Vec<Snap>,
    pub(crate) redo: Vec<Snap>,
    pub(crate) last_json: String,
    pub(crate) last_snap: std::time::Instant,
    pub(crate) last_backup_day: String,
    pub(crate) check_open: bool,
    pub(crate) dup_open: bool,
    pub(crate) merge_open: bool,
    pub(crate) merge_id: String,
    pub(crate) merge_other: Option<u32>,
    pub(crate) dups: Vec<(u32, u32, String)>,
    pub(crate) dup_ignore: HashSet<(u32, u32)>,
    pub(crate) save_error: Option<String>,
    pub(crate) last_saved: Option<std::time::Instant>,
    pub(crate) next_save: std::time::Instant,
    pub(crate) notice: Option<String>,
    pub(crate) media_job: Option<std::sync::mpsc::Receiver<(bool, usize, usize, Option<String>)>>,
    pub(crate) startup_done: bool,
    pub(crate) verify_open: bool,
    pub(crate) verify_results: Vec<(String, Result<usize, String>)>,
    pub(crate) media_gap: String,
    pub(crate) close_warned: bool,
    pub(crate) report: Option<ImportReport>,
    pub(crate) report_open: bool,
    pub(crate) link: Option<LinkReq>,
    pub(crate) fit_req: bool,
    pub(crate) editing: Option<EditSession>,
    pub(crate) quick: Option<QuickEdit>,
    pub(crate) gal_only: bool,
    pub(crate) home_pick: Option<String>,
    pub(crate) gal_find: String,
    pub(crate) assoc_q: String,
    pub(crate) assoc_role: String,
    pub(crate) x: Extra,
}

impl App {
    pub(crate) fn new() -> Self {
        let (mut db, load_notice) = load_from(&data_dir());
        fix_names(&mut db);
        // Older versions kept one home person for the whole program; move it into the tree it
        // was chosen in (the tree that is open), where each tree now keeps its own.
        let mut settings_home_moved = false;
        {
            let st = load_settings();
            if db.home.is_none() {
                if let Some(h) = st.home.filter(|h| db.people.contains_key(h)) {
                    db.home = Some(h);
                }
            }
            if st.home.is_some() {
                settings_home_moved = true;
            }
        }
        db.home = db.home.filter(|h| db.people.contains_key(h));
        let notice = if load_notice.is_empty() { None } else { Some(load_notice) };
        let recovered = notice.is_some();
        let mut status = if db.sample {
            "Showing sample data. Use File > Import GEDCOM to load your own tree.".to_string()
        } else if db.people.is_empty() {
            "Welcome! Add your first person, or import a tree with File > Import tree.".to_string()
        } else {
            String::new()
        };
        let sel = db.home.or_else(|| db.people.keys().next().copied());
        let last_json = serde_json::to_string(&db).unwrap_or_default();
        if let Some(n) = &notice {
            status = n.clone();
        }
        if let Some(svc) = synced_service(&data_dir()) {
            if status.is_empty() {
                status = format!("Your tree folder looks like it is synced by {}. Avoid opening it on two computers at once. See Help > About for options.", svc);
            }
        }
        if let Err(e) = dated_backup(&db) {
            status = format!("Today's backup could not be made: {}", e);
        }
        if mac_translocated() {
            status = "macOS is running this copy from a hidden temporary folder, so a FamilyTreeData folder beside it (portable mode) cannot be used. See Help > About for the one-time fix.".to_string();
        }
        let mut app = App { db, sel, view: View::Pedigree, search: String::new(), fx: 0, dlg: None, confirm: false, status, dirty: recovered, find_open: false, settings: load_settings(), show_look: false, show_loc: false, review: None, imerge: None, src_open: false, src_sel: None, src_tab: 0, src_filter: String::new(), repo_sel: None, src_pick: None, src_merge: None, src_confirm: false, hl: None, back: Vec::new(), fwd: Vec::new(), wipe: None, show_about: false, help_open: false, help_find: String::new(), move_data: false, imp_open: false, imp_src: 0, imp_replace: false, rec: None, rel_open: false, rel_a: None, rel_b: None, viewing: None, undo: Vec::new(), redo: Vec::new(), last_json, last_snap: std::time::Instant::now(), last_backup_day: today(), check_open: false, dup_open: false, merge_open: false, merge_id: String::new(), merge_other: None, dups: Vec::new(), dup_ignore: HashSet::new(), save_error: None, last_saved: None, next_save: std::time::Instant::now(), notice, media_job: None, startup_done: false, verify_open: false, verify_results: Vec::new(), media_gap: String::new(), close_warned: false, report: None, report_open: false, link: None, fit_req: false, editing: None, quick: None, gal_only: false, home_pick: None, gal_find: String::new(), assoc_q: String::new(), assoc_role: "Guardian".to_string(), x: Extra::default() }.started();
        if settings_home_moved {
            // the home person now lives in the tree; the old program-wide setting is cleared
            app.settings.home = None;
            save_settings(&app.settings);
            app.dirty = true;
        }
        app
    }

    pub(crate) fn persist(&mut self) -> bool {
        // the sample family is for practice: it is never written over the real tree
        if self.x.practice.is_some() {
            return true;
        }
        self.wait_for_save();
        self.track();
        self.x.track_at = None;
        match save_json_to(&data_dir(), self.last_json.as_bytes()) {
            Ok(_) => {
                self.last_saved = Some(std::time::Instant::now());
                self.save_error = None;
                true
            }
            Err(e) => {
                self.save_error = Some(e);
                self.next_save = std::time::Instant::now() + std::time::Duration::from_secs(3);
                false
            }
        }
    }
}

impl App {
    pub(crate) fn started(mut self) -> Self {
        self.remember_state();
        self
    }

    /// After the tree changed or was swapped for another: lists worked out from it are redone.
    pub(crate) fn tree_replaced(&mut self) {
        self.x.data_ver += 1;
        self.x.last_hash = 0;
    }

    pub(crate) fn remember_state(&mut self) {
        self.x.prev_names = self.db.people.iter().map(|(k, p)| (*k, p.name.clone())).collect();
        self.x.prev_sel = self.sel.and_then(|s| self.db.people.get(&s).map(|p| (s, serde_json::to_string(p).unwrap_or_default())));
        self.x.prev_fams = self.db.fams.len();
        self.x.next_label = None;
    }

    /// Starts saving in the background (the copy of the tree saved is the one Undo last noted).
    pub(crate) fn autosave(&mut self) {
        if self.x.practice.is_some() {
            self.dirty = false;
            return;
        }
        if self.x.save_job.is_some() {
            return;
        }
        self.track();
        self.x.track_at = None;
        let dir = data_dir();
        let json = self.last_json.clone().into_bytes();
        self.dirty = false;
        self.next_save = std::time::Instant::now() + std::time::Duration::from_secs(2);
        self.x.save_job = Some(std::thread::spawn(move || save_json_to(&dir, &json)));
    }

    /// Picks up the result of a background save once it has finished.
    pub(crate) fn poll_save(&mut self) {
        if self.x.save_job.as_ref().map_or(false, |j| j.is_finished()) {
            self.finish_save();
        }
    }

    /// Waits for a background save to finish (before closing, or saving again straight away).
    pub(crate) fn wait_for_save(&mut self) {
        if self.x.save_job.is_some() {
            self.finish_save();
        }
    }

    fn finish_save(&mut self) {
        let Some(job) = self.x.save_job.take() else { return };
        match job.join().unwrap_or_else(|_| Err("the save stopped unexpectedly".to_string())) {
            Ok(_) => {
                self.last_saved = Some(std::time::Instant::now());
                self.save_error = None;
            }
            Err(e) => {
                self.save_error = Some(e);
                self.dirty = true;
                self.next_save = std::time::Instant::now() + std::time::Duration::from_secs(3);
            }
        }
    }

    /// Notes a change for Undo. Returns whether anything in the tree changed.
    pub(crate) fn track(&mut self) -> bool {
        let mut h = Fingerprint::default();
        if self.x.last_hash == 0 {
            self.x.last_hash = fingerprint(self.last_json.as_bytes());
        }
        if serde_json::to_writer(&mut h, &self.db).is_ok() && h.0 == self.x.last_hash {
            if self.x.prev_sel.as_ref().map(|x| x.0) != self.sel {
                self.remember_state();
            }
            return false;
        }
        let cur = serde_json::to_string(&self.db).unwrap_or_default();
        if cur != self.last_json {
            let label = self.change_label();
            if self.last_snap.elapsed() > std::time::Duration::from_millis(800) || self.undo.is_empty() {
                self.undo.push(Snap { z: pack(&self.last_json), label, at: std::time::Instant::now() });
                if self.undo.len() > 60 {
                    self.undo.remove(0);
                }
            } else if let Some(last) = self.undo.last_mut() {
                // quick follow-on edits are one step; keep the clearer description
                if last.label != label && label != "Changed the tree" && last.label == "Changed the tree" {
                    last.label = label;
                }
                last.at = std::time::Instant::now();
            }
            self.redo.clear();
            self.last_json = cur;
            self.tree_replaced();
            self.last_snap = std::time::Instant::now();
            self.remember_state();
            true
        } else {
            if self.x.prev_sel.as_ref().map(|x| x.0) != self.sel {
                self.remember_state();
            }
            false
        }
    }

    pub(crate) fn load_snapshot(&mut self, json: String) {
        if let Ok(mut db) = serde_json::from_str::<Db>(&json) {
            fix_names(&mut db);
            self.db = db;
            self.last_json = serde_json::to_string(&self.db).unwrap_or(json);
            self.tree_replaced();
            self.last_snap = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(5)).unwrap_or_else(std::time::Instant::now);
            if !self.sel.map(|s| self.db.people.contains_key(&s)).unwrap_or(false) {
                self.sel = self.db.home.or_else(|| self.db.people.keys().next().copied());
            }
            self.viewing = None;
            self.hl = None;
            self.dirty = true;
            self.remember_state();
        }
    }

    pub(crate) fn undo_action(&mut self) {
        self.track();
        if let Some(prev) = self.undo.pop() {
            self.redo.push(Snap { z: pack(&self.last_json), label: prev.label.clone(), at: prev.at });
            self.load_snapshot(unpack(&prev.z));
            self.status = format!("Undone: {}.", prev.label);
        }
    }

    pub(crate) fn redo_action(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(Snap { z: pack(&self.last_json), label: next.label.clone(), at: next.at });
            self.load_snapshot(unpack(&next.z));
            self.status = format!("Redone: {}.", next.label);
        }
    }
}

#[cfg(test)]
impl App {
    /// An app with the given tree that never touches the user's real data or settings.
    pub(crate) fn for_test(db: Db) -> App {
        let sel = db.people.keys().next().copied();
        let status = String::new();
        let recovered = false;
        let notice: Option<String> = None;
        let last_json = serde_json::to_string(&db).unwrap_or_default();
        App { db, sel, view: View::Pedigree, search: String::new(), fx: 0, dlg: None, confirm: false, status, dirty: recovered, find_open: false, settings: Settings::default(), show_look: false, show_loc: false, review: None, imerge: None, src_open: false, src_sel: None, src_tab: 0, src_filter: String::new(), repo_sel: None, src_pick: None, src_merge: None, src_confirm: false, hl: None, back: Vec::new(), fwd: Vec::new(), wipe: None, show_about: false, help_open: false, help_find: String::new(), move_data: false, imp_open: false, imp_src: 0, imp_replace: false, rec: None, rel_open: false, rel_a: None, rel_b: None, viewing: None, undo: Vec::new(), redo: Vec::new(), last_json, last_snap: std::time::Instant::now(), last_backup_day: today(), check_open: false, dup_open: false, merge_open: false, merge_id: String::new(), merge_other: None, dups: Vec::new(), dup_ignore: HashSet::new(), save_error: None, last_saved: None, next_save: std::time::Instant::now(), notice, media_job: None, startup_done: true, verify_open: false, verify_results: Vec::new(), media_gap: String::new(), close_warned: false, report: None, report_open: false, link: None, fit_req: false, editing: None, quick: None, gal_only: false, home_pick: None, gal_find: String::new(), assoc_q: String::new(), assoc_role: "Guardian".to_string(), x: Extra::default() }.started()
    }
}

/// A quick fingerprint of some bytes (FNV-1a). The same bytes, however they are split up when
/// written, always give the same fingerprint.
pub(crate) struct Fingerprint(pub(crate) u64);

impl Default for Fingerprint {
    fn default() -> Self {
        Fingerprint(0xcbf2_9ce4_8422_2325)
    }
}

impl std::io::Write for Fingerprint {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut h = self.0;
        for b in buf {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = h;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) fn fingerprint(b: &[u8]) -> u64 {
    let mut h = Fingerprint::default();
    let _ = std::io::Write::write(&mut h, b);
    h.0
}
