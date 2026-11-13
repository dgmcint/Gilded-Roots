# Changelog

Major work so far, in the order it was done.

## Not yet released

- **Notes can be removed from Memories:** in Select mode, notes can now be ticked (click the card or its box) and
  removed along with photos and files; this clears that person's Notes box, and Undo (Ctrl+Z) brings it back.
  Opening the "Remove selected" box is also quicker on a big tree.

## 1.0.4 (12 November 2026)

- **LDS ordinances panel** on the Individual page: baptism, confirmation, initiatory, endowment, sealing to
  each set of parents and to each spouse, with date, temple (chosen from a searchable list of temple codes,
  including PAF's older two-letter codes) and status. Read from and written to GEDCOM files as PAF does.
- **Reorder children and spouses** in the Family view (^ v for children, < > for spouses), and **Sort children
  by birth**. Spouse order is kept in GEDCOM files and read from PAF and other programs' files.
- **Find by ID number** (also finds the old number after renumbering) and **Find a couple** (Search menu and the
  Find window); a number typed in quick jump finds that ID first.
- **Ditto:** Ctrl+D (Cmd+D) in an empty date or place box fills in the last date or place typed.
- **Special letters** window (Edit menu): accented letters and symbols, put straight into the box being typed in.
- **Groups (focus and filter)** (Search menu): named sets of people kept with the tree, built by relationship
  (ancestors, descendants, everyone connected, with spouses and brothers and sisters, any number of generations)
  and by what is recorded (for example "Birthplace contains Idaho", "Born before 1900", "Has no sources"). A group
  can be shown in the People tab, printed as a list, exported as a GEDCOM file of only those people, or copied into
  a new tree. Right-click anyone to add them to a group.
- **Ancestor book** and **Descendant book** (Reports menu): the family written out as numbered paragraphs, generation
  by generation, with an index of names.
- **Ready-made lists** (Reports → Lists): everyone alphabetically, marriages, end-of-line ancestors, people linked to
  nobody, missing LDS ordinances, contact details and possible duplicates.
- **A4 paper** for every printout (Reports → Paper), as well as US Letter.
- **Source templates**: fill in a few boxes for a census, certificate, newspaper item, book, website and more, and
  the source's title, author and publication details are written out for you.
- **Pictures on citations**: attach a scan of the record itself to a citation; kept in GEDCOM files.
- **Find and replace in notes** (Edit menu), with every match shown first; **note tags** ("Military: ",
  "Research: "...); lines starting with **~** are private and left off printouts and private copies.
- **Date calculator**, **Soundex** and **Search websites for this person** (FamilySearch, Find a Grave, WikiTree,
  Ancestry, Google) in the Tools menu.
- **Contact details** for living relatives: address, phone, email and web page, kept in GEDCOM files and left out
  of private copies.

## 1.0.3 (9 November 2026)

- Pedigree and descendant charts: a **Select / Hand** choice on the toolbar. With the hand on, drag anywhere on the
  chart to move it around, without selecting anyone by accident. **H** switches between the two, and the choice is
  remembered.
- **Home person kept with each tree.** Importing a GEDCOM file keeps its home person (the person the file marks as
  home, or else the first person in the file, as PAF, Ancestry and FamilySearch do), and exported files list the home
  person first and name them in the header (`1 _HOME`), so re-importing keeps them. Each tree now has its own home
  person; a home person chosen in an older version moves into the tree that was open. Trees open at their home person.
- **Tools → Renumber people:** new ID numbers 1, 2, 3... for everyone and for families, in family order from the
  home person, the present order, or alphabetical; preview first, Undo afterwards, and each person keeps an **Old ID**.
- **Map:** opens on everyone's places, and never zooms closer than about a continent when fitting the pins (one or two
  pins no longer fill the screen). Switching between This person, Ancestors, Descendants and Everyone now fits the
  newly chosen pins rather than the previous ones.
- **Gallery and Memories:** when removing items, files nobody else uses can also be taken out of the media folder
  (moved to a **Removed files** folder, not deleted), and **Remove online links...** takes off every link to a photo
  stored on Ancestry, MyHeritage or a similar site in one go.
- The sample family's home person is Emily Rose Harwood. Exported GEDCOM headers carry the program's real version.
- Behind the scenes: the program's largest file was split into smaller files by topic, to make it easier to work
  on. Nothing changes in how the program works.

## 1.0.2 (7 November 2026)

- Colours: 25 colour schemes in five groups (tree pictures, genealogy sites, earth tones, popular colours and
  neutrals), under View > Colours > Colour schemes and in the Appearance window. A scheme sets the background,
  accent and text colour together; the swatches still change each one on its own.

## 1.0.1 (6 November 2026)

- Downloads: one per system, plus a README. Windows: one installer that installs on the computer or sets up a
  portable copy on a USB drive. Mac: one app for both Apple chip and Intel Macs, with a portable folder in the
  dmg. Linux: a tar.gz with `install.sh` (for this user, no password) and a portable folder, built to run on
  older Linux versions too. The README is a PDF, so it always opens as a normal white page, even in dark mode.
- Gallery: new photos show their preview straight away, always open in the built-in viewer, and several can be
  selected and removed at once (Undo brings them back).
- Fan chart: no more faint stripes across the coloured sections.
- DNA: pasting an ethnicity estimate with a dash, accent or non-breaking space before the number no longer
  closes the program.
- Safety: a tree or archive from someone else can no longer point the program at files outside the media
  folder; archives unpack only photos, documents and recordings; web links open only for web addresses.

## 1.0.0

### Foundation
- Native Rust/egui app for Windows, macOS and Linux with dark/grey theme, pedigree, family and descendant
  views, search, undo/redo, photos, documents and recorded stories.
- Installers built in the cloud by a GitHub workflow (Windows `.exe`, macOS `.dmg` for both chip types,
  Linux `.deb`).

### Priority 1: Data safety
- Atomic saves (temp file, flush, read back, swap), previous version kept as `tree.json.prev`.
- Damaged files are never overwritten; they are renamed and the newest readable copy is loaded.
- Daily backups kept 14 days with checksums; photos and recordings mirrored to `backups/media`.
- Visible data folder in Documents, relocation setting, portable mode, and a cloud-sync warning.
- Save-failure banner and "NOT SAVED" status; recovery test suite.

### Priority 2: GEDCOM interchange
- Reader for GEDCOM 5.5 / 5.5.1 with UTF-8, UTF-16, ANSEL and Windows ANSI.
- Lossless "raw" storage of anything without a field; export writes it back.
- Import report and read-back check on every export; living-people-private export.
- Non-GEDCOM file detection, JSON tree import, and tested against real-world files from many programs.

### Priority 3: Sources and citations
- Reusable source records, repositories, and per-claim citations (page, quality, transcription, URL,
  access date) on people, events and families.
- Sources manager with usage list and merge; bibliography report; GEDCOM round trip.

### Priority 4: Safer merging
- Side-by-side review screen, conflicting facts, warnings, explicit confirmation; no silent merges.
- Merge by person ID, "Find duplicates", and import with "Add to my tree" confirmation per candidate.
- Safety backup before changes, and undo.

### Priority 5: Family relationship model
- Multiple sets of parents; per-parent relationship (biological, adopted, step, foster, guardian, sealed).
- Preferred parents for the chart; partnership kinds; non-traditional and same-sex couples supported.
- Derived step/half relatives; associations (guardian, godparent, foster carer...); relationship finder.
- Data checker rules for biological-parent conflicts, ancestry loops and duplicate partnerships.
- GEDCOM round trip of all of the above; data format upgraded to version 4 with automatic migration.

### Priority 6: Large-tree navigation and speed
- Fit-to-window, zoom down to 25%, home person, navigation trail, stronger selected/highlighted styling.
- Parents / Siblings / Partners / Children menus (including adoptive, step, foster and half relatives) and
  keyboard navigation (arrow keys, Home).
- Performance work tested with generated trees of 1,000, 5,000 and 20,000 people: the data checker went from
  24 s to 0.06 s, the duplicate finder from 14 s to 0.3 s and the descendants layout from 0.4 s to 7 ms at 20,000.
  Charts use lookup tables and the descendants chart only draws what is on screen.
- New automated tests: index results match the old scans, large trees stay fast, navigation helpers.

### Safer editing and a fuller Add window
- The Individual page opens locked; Edit unlocks it, Save keeps changes and Cancel restores the person exactly.
  Leaving a person or tab with unsaved changes asks to Save, Discard or Keep editing.
- The Add person window now takes sex, birth, death, burial, living status and notes.
- "Home person" and "Show these parents on the chart" moved out of the menu bar to right-click menus, the
  person page and the Parents button above the chart.

### Family tab redesign
- Compact person cards with portraits, a tighter children table with a Parent link column, and a Relationships
  panel (parents, siblings, partners and children, other relationships) beside the family. Less wasted space.

### Dockable panels on the Individual page
- Core facts and timeline on the left; Relationships, Events, Sources, Notes, Other names, Photos and
  recordings, and Merge and delete as panels on the right. Show/hide, reorder, fold, resize, pop out into
  floating windows, and a remembered layout with Reset layout.

### Descendants chart
- A person's partner(s) now appear on a second line inside the box (on screen and in the printed chart), so they no longer run off the edge.

### Pedigree actions and Gallery
- Pedigree boxes show a portrait or initial and a "..." menu: open individual page, view their tree (ancestors,
  descendants, family), quick edit, add a relative (father, mother, other parents, sibling, spouse, child).
- Generation 1 has "Spouse & children" and "Siblings" drop-downs; descendants chart shares the right-click menu.
- New Gallery tab with every photo in the tree, filter by name or caption.
- Locked Individual page now shows a readable profile (portrait, large name, filled-in facts, clickable relatives)
  instead of greyed-out boxes.
- "Load sample data" and "Start empty" removed from Tools (still offered on first run).

### Ancestry import fixes and home person
- Ancestry media details (FORM with TYPE, size, width, height) are now kept on export; web-link photos show
  "Open online" and appear in the Gallery; the import report explains that Ancestry files carry links, not pictures.
- Dates written as "FEB 8 1891", "13 SEPTEMBER, 1713" or "1864-01-30" were already tidied on import but were
  wrongly listed as lost in the report; fixed.
- Home person is now chosen in Tools > Home person... (search the tree), instead of a button on the Individual page.

### Help and clicking
- F1 (and Help > User guide) opens the user guide inside the program, with search.
- Double-click now allows half a second between clicks (was 0.3 s), so it works with a normal Mac mouse speed.

### Photo fixes
- Profile pictures showed an error when the data folder name has a space ("Gilded Roots"); fixed.
- A profile picture now shows even before its small preview is made.
- "Open media folder" button in the Photos panel shows where added photos are copied.

### Research and navigation tools
- Quick jump (Ctrl/Cmd+K, F4): find any person or command by typing.
- F-key shortcuts (F1 to F12) with Mac alternatives, and Help > Keyboard shortcuts.
- Research to-do panel on every person, tree-wide to-do list, GEDCOM `_TODO` import and export.
- Places explorer: every place with who was there; groups places spelled more than one way; change a spelling everywhere.
- Face tagging in photos; tagged photos appear on the tagged person's page and in the Gallery; face-crop profile pictures.
- History of changes (F12) with plain-English descriptions; click to step back; undo history is stored compressed
  (about a tenth of the memory) and holds 60 steps.
- Saved versions browser: compare a daily copy with today and bring back one person or the whole tree.
- Recent people and bookmarks.
- "Things to check" bar on the Individual page from the data checker, plus a "no sources yet" reminder.

### Charts and views
- Charts tab: fan chart (half, 240 degrees or full circle, 3 to 8 generations, click a gap to add a parent), hourglass,
  and lifespans; all printable.
- People tab: sortable, filterable table of everyone; save as CSV.
- View menu: text size, name style, date style, chart box contents, full screen (also in File and F11).
- Ready-made and saved panel layouts on the Individual page.
- "Show which parents" on every chart box (switch between birth and adoptive parents).
- About window centred; created by DGMCINT

### PDFs and profile photos
- Built-in PDF viewer (pure Rust, offline): pages, zoom, fit width; falls back to your PDF app for password-protected files.
- Add photos or documents now offers a filter for photos, PDFs, documents and recordings.
- Crop the profile photo to any part of a picture; photos are trimmed to fit, never stretched.
- View > Profile photo size (Small to Extra large).

### Notices
- Not-affiliated notice in About, README and the user guide.
- THIRD_PARTY_LICENSES.txt generated from Cargo.lock and shown in Help > Open-source licences.
- About moved to the top of the Help menu; credit shown as DGMCINT.

### Map, Memories and printing
- Map tab: offline world map with pins for births, marriages, deaths, burials and other events; this person, ancestors,
  descendants or everyone; year slider with Play; click a pin to list who was there; set positions for places not found.
  Built-in data: Natural Earth (public domain) and GeoNames town names (CC BY 4.0).
- Memories tab: stories (write and read), recordings, documents and PDFs, and notes, with filters and + Add.
  Gallery stays photos only and also has + Add.
- Printing redesigned: fits the page (long web addresses wrap), Letter size, page numbers and date in the footer,
  tidy dates; new family group sheet layout with parents, spouses, sex and numbered sources; pedigree with birth
  and death lines.
- View > Colours with 24 accent colours and 8 dark backgrounds as large swatches.
- The window opens larger (most of the screen, centred) and remembers its size.
- "Keep living people private" moved to the Reports menu.

### Security review
- Checked dependencies with cargo-audit and the code with clippy; fuzz-tested the GEDCOM reader with 3,000 damaged files.
- A GEDCOM file can only bring in photos, documents and recordings, never other files it names.
- Files open on Windows without going through the command shell.
- Print pages are written to the program's own folder, not the shared temporary folder.
- Spreadsheet (CSV) export cannot carry formulas.
- Story files are always kept inside the media folder.

### Archive, research and practice
- Welcome screen: import (with step-by-step PAF help), create new, open an archive, or explore the sample family.
- Practice mode with a made-up six-generation sample family (photos, tagged wedding photo, PDFs, story, sources,
  journal, confidence ratings); nothing is saved and the real tree is untouched.
- Family Tree Archive: one zip with the tree, all media, a GEDCOM copy and a readme; checked before it is used.
- Backup settings: hourly, daily or weekly; how many to keep; a second-copy folder that receives an archive.
- Confidence on every fact (Confirmed, Supported, Probable, Uncertain, Disputed), exported as `_CONF`.
- Research journal, on its own and per person.
- Historical events beside each timeline (world and regional), plus your own.
- Family statistics; surnames and where they are found; surname filter on the Map.
- Duplicate finder: sound-alike surnames, maiden names, nicknames, one-letter typos.
- Separate family trees (File > Family trees).
- Report designer with saved designs, print/PDF and CSV.
- Privacy and "Moving from PAF" sections in the README.
- Security: zip library on version 9 (fixes published advisories); archives are only read file by file into the
  media folder, never unpacked blindly.

### Tombstone calculator
- Tools > Tombstone calculator: a birth date from the death date and the age on a gravestone (years, months and days,
  years only, or "in his 73rd year"); exact dates or a one-year / one-month window; warnings for old-calendar dates,
  approximate dates and burial dates.
- Saves the answer as an estimated birth with a note and the gravestone as its source, and keeps the age with the death.
- Also on the Individual page ("Calculate birth from gravestone age...") and in Quick jump.

### Blank forms
- Reports > Forms: blank family group sheet, blank 4- and 5-generation pedigree charts with chart numbers for linking,
  and blank census worksheets for every United States census from 1790 to 1950 (columns numbered as on the original
  schedules, split over several sheets where the original was very wide).
- Canadian census worksheets for 1851 (1852) to 1931, including the 1906 and 1916 Prairie censuses, with
  province, district and sub-district boxes.
- United Kingdom census worksheets for England and Wales, 1841 to 1921, with notes on the extra Scottish questions.

### DNA, maps and layout
- New DNA tab: ethnicity estimates (table, bar chart and map; paste from the testing site), DNA matches (shared cM,
  segments, longest segment, prediction, side, notes, link to the tree), CSV import of match lists, a "matches by
  closeness" chart and a "How related?" guide from shared cM.
- New flat map look with a Map style choice: Match the app (built from the chosen background and accent colours),
  Charcoal, Warm grey, Slate blue or Paper. Used by the Map tab and the DNA map.
- Gallery and Memories are laid out as a grid that fits the window instead of running off the right edge.
- Y-DNA and mtDNA tests: haplogroup, test level, Y-STR markers (typed, pasted or from a file) or mtDNA mutations,
  matches with genetic distance and earliest known ancestor, and the father's or mother's line through the tree
  drawn as a path on the map.
- DNA imports read CSV, tab-separated, UTF-16 and Excel (.xlsx) files; 23andMe segment files are added up per
  person; raw DNA files are recognised and turned away; "Where do I get the file?" help for each company.
- The map wraps around at the date line.
- Blank family group sheet and pedigree charts with LDS ordinances; UK 1891 census form numbered as the original;
  piece, folio and page boxes on UK forms.
- Blank family group sheets and pedigree charts (plain and LDS) each print on one page, filling the paper
  edge to edge. The LDS family group sheet holds eight children and puts the couple's ordinances side by side.
- Removed the shrink-to-fit print script: it misjudged the page while printing and shrank or shifted pages
  that already fitted. Every printout is laid out to the paper width instead.
- Safari: printouts are sized in inches so they fit at Safari's 100% scale, family group sheets leave room for
  Safari's own header and footer, and a tip on the page (not printed) explains the Scale and Landscape settings.
- Vital facts on the Individual page no longer wrap one word per line.

### PAF files
- PAF 5 `.paf` files open directly through Import: names, sex, birth, death and marriage dates and places (exact
  dates and typed date phrases), husbands, wives, children in PAF's order, second marriages, and PAF's kept extra
  GEDCOM lines. Worked out clean-room from a user's own .paf and its GEDCOM export, and checked against all 2,306
  people and 733 families in that file.
- Details not decoded yet (christenings, burials, ordinances, notes, sources, non-birth child links, unusual date
  forms) are counted in the import report, never guessed.

### Portable versions
- Portable downloads for Windows, Mac (both chips) and Linux: the program plus a FamilyTreeData folder, ready for a USB drive.
- Make portable now carries every tree along; the Mac warns when macOS runs the app from a hidden temporary copy.

### Support
- Optional donation link in Help > About, the README and the GitHub Sponsor button.

### Known gaps
- `.paf` import reads the core facts only; export a GEDCOM from PAF for notes, sources, ordinances and other events.
- GEDCOM 7.0 is untested.
