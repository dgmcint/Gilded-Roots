GILDED ROOTS - family history

Needs Rust 1.92 or newer. If Rust is already installed, run:  rustup update

WINDOWS
1. Install Rust from https://rustup.rs (and Visual Studio Build Tools, C++ workload, if asked).
2. In this folder run:  cargo run --release
3. Your app:  target\release\gilded-roots.exe
   Data is saved to Documents\Gilded Roots\tree.json (older versions used %APPDATA%\FamilyTree; it is moved over once, and the old copy is left in place)

MAC
1. Open Terminal and run:  xcode-select --install
2. Install Rust:  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   then close and reopen Terminal.
3. cd into this folder and run:  cargo run --release
4. To make a double-clickable app (this also adds microphone permission), run:
     bash build-mac.sh
   (or by hand: cargo install cargo-bundle, cargo bundle --release, then add NSMicrophoneUsageDescription
   to the app's Info.plist and run codesign --force --deep -s - on it)
   Your app:  target/release/bundle/osx/Gilded Roots.app  (drag it to Applications)
   The first time, right-click it and choose Open, because it is not signed by Apple.
   Data is saved to ~/Documents/Gilded Roots/tree.json (older versions used ~/Library/Application Support/FamilyTree; it is moved over once, and the old copy is left in place)

Build on the Mac you will use it on (Apple Silicon or Intel).
Use File > Import GEDCOM to load your data and File > Export GEDCOM for backups.

LINUX
1. Install build tools and libraries:
   Debian/Ubuntu:  sudo apt install build-essential pkg-config libgtk-3-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libasound2-dev
   Fedora:         sudo dnf install gcc pkg-config gtk3-devel libxkbcommon-devel wayland-devel mesa-libGL-devel alsa-lib-devel
   Arch:           sudo pacman -S base-devel gtk3 libxkbcommon wayland alsa-lib
2. Install Rust:  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   then close and reopen the terminal.
3. In this folder run:  cargo run --release
4. Your program:  target/release/gilded-roots
5. To add it to your app menu:
     mkdir -p ~/.local/bin ~/.local/share/applications
     cp target/release/gilded-roots ~/.local/bin/
     mkdir -p ~/.local/share/icons/hicolor/256x256/apps
     cp icons/256x256.png ~/.local/share/icons/hicolor/256x256/apps/gilded-roots.png
     sed "s|HOME|$HOME|" gilded-roots.desktop > ~/.local/share/applications/gilded-roots.desktop
   Data is saved to ~/Documents/Gilded Roots/tree.json (older versions used ~/.local/share/FamilyTree; it is moved over once, and the old copy is left in place)

INSTALLERS FOR ALL THREE SYSTEMS (automatic)
Installers cannot be built on one computer for all systems, so this project
includes a GitHub workflow that builds them in the cloud for free.
1. Make a free account at https://github.com and create a new repository.
2. Upload everything in this folder (keep the .github folder).
3. Open the repository's Actions tab, pick "Build installers", click Run workflow.
4. After about 10 minutes, open the finished run and download:
     windows-installer      -> GildedRoots-Setup.exe
     mac-installer-apple-silicon / mac-installer-intel -> .dmg files
     linux-installer        -> GildedRoots-linux-amd64.deb (sudo apt install ./file.deb)
To publish a downloadable release page instead, push a tag such as v1.0.0.
The installers are unsigned: Windows may show a SmartScreen warning (More info > Run anyway)
and on Mac, open the app once, then choose System Settings > Privacy & Security > Open Anyway
(on macOS 14 or older, right-click the app and choose Open instead).

PHOTOS, DOCUMENTS AND RECORDED STORIES
Files you add are copied into a "media" folder next to your tree data, and recordings are saved there as .wav files.
Mac: the first time you record, macOS asks for microphone permission. If you build the app yourself (not from the
GitHub installer), add NSMicrophoneUsageDescription to its Info.plist, as the workflow does.

APP ICON
The logo is in the icons folder. To change it, replace the files there (keep the same names) and rebuild.

PORTABLE MODE (keep your tree with the app, for example on a USB stick)
Tools > Make portable copies your data into a FamilyTreeData folder next to the app (next to the .app on Mac)
and uses it from then on. Copy the app AND that folder together to move your tree. To share only the app,
copy just the app, without the FamilyTreeData folder. To stop using portable mode, delete or rename the
FamilyTreeData folder; the app goes back to the data folder on the computer.

DATA SAFETY
- Saves are atomic: the tree is written to a temporary file, flushed to disk, read back and checked,
  and only then swapped in. The previous good version is kept as tree.json.prev.
- If a save fails, a red banner and "NOT SAVED" in the status bar appear and the app keeps retrying.
- If tree.json is ever damaged, the app never overwrites it: it is renamed tree.json.damaged-... and the
  newest readable copy (interrupted save, previous version, dated backup) is loaded instead.
- One backup per day is kept for 14 days, each with a checksum. Photos and recordings are copied to
  backups/media (never deleted). Tools > Check backups re-verifies everything.
- Run the recovery tests with:  cargo test

GEDCOM (importing and exporting)
--------------------------------
* Opens PAF 5 .paf files directly (names, births, deaths, marriages and family links).
* Reads GEDCOM 5.5 / 5.5.1 from PAF, Ancestry, FamilySearch, MyHeritage, RootsMagic, Gramps, webtrees and others.
  Character sets: UTF-8, UTF-16, ANSEL (PAF) and Windows ANSI. Line endings: LF, CR LF or CR.
* Kept: source records with page, quality and quoted text; shared (linked) notes; photos and documents
  (copied in when found next to the file); other names (birth, married, aka...), titles, suffixes, nicknames;
  all standard event types including LDS ordinances; family events, adoption/foster links, several marriages;
  approximate and uncertain dates ("about", "between", "from..to", date phrases).
* Anything the app has no field for (addresses, map coordinates, change dates, ID numbers, custom tags, whole custom
  records) is stored exactly as read and written back on export.
* After every import the app compares the original file with what an export would contain and shows an
  Import report. Every export is read back and checked the same way.
* Export puts photos and documents in a folder named <file>-media next to the .ged file. Keep the two together.
* Tests: `cargo test`. To also test against your own GEDCOM files, put them in a folder and run
  FT_GED_SAMPLES=/path/to/folder cargo test real_world -- --nocapture

SOURCES AND CITATIONS
---------------------
Tools > Sources and repositories... opens the source manager.
- A SOURCE is written down once: title, author, type (book, census, church record, website...), publication
  details, web address, where it is kept (a repository), call number and notes.
- A REPOSITORY is where sources are kept: name, address, phone, email, web address, notes.
- A CITATION says where one fact came from. On a person you can add a source for the person, for Birth,
  Christening, Death, Burial or Marriage; every "More events" row and every family (marriage and family
  events) has its own Sources block. Each citation picks a source from the list (or makes a new one) and adds
  its own page or entry, quality rating, a transcription of the text, a web address for that record and the
  date you looked at it.
- One source can be cited any number of times. The source manager shows every person, fact and event that
  cites it, and "Merge" joins two entries of the same source. Deleting a source keeps its name in each citation.
- Reports > Source list (bibliography)... prints all sources with where they are kept.
- GEDCOM: sources, repositories, call numbers (CALN/MEDI), pages, quality (QUAY), transcriptions (DATA/TEXT)
  and entry dates are written in the standard way. Web addresses are written as WWW, the source type as _TYPE and
  the date you looked at a record as _ACCESSED; other programs may keep these as custom data or ignore them.

MERGING SAFELY
--------------
The program never decides that two records are the same person on its own. A matching name and birth year
is only a clue, because different people can share both.
- Tools > Find duplicates, or "Merge with another person..." on a person's page, opens a REVIEW screen:
  both records side by side, their parents, spouses and children compared, every fact that differs marked
  DIFFERENT, and warnings (different sex, birth years far apart, different parents, one is the other's
  ancestor, they are husband and wife...). Where facts differ you click the value to keep; the other value is
  saved in the person's notes. You must tick a box to confirm, and a second box if there are serious warnings.
- File > Import tree, then "Add to my tree": after you pick a file, every person who might already be in your tree is
  listed with the reasons ("same birth year", "a parent has the same name") and the reasons against. You
  confirm "same person" or "different people" for each one. Anyone you do not confirm is added as a separate
  person. A safety backup is made before anything changes, and Undo works afterwards.


COMPLEX FAMILIES
----------------
Gilded Roots does not assume every person has one father, one mother and one marriage.

* More than one set of parents. A person can belong to several parent families, for example the birth
  parents and an adoptive couple. On the person's page, each set is shown separately. Use
  "Another set of parents" to add one, and "Show these parents on the chart" to choose which set the
  pedigree follows (also in the Parents button above the chart).
* Biological and legal relationships are kept apart. For each child and each parent you can choose
  Biological, Adopted, Step-parent, Foster, Guardian or Sealed (LDS). Nothing is overwritten:
  the biological family stays recorded while the adoptive one is added beside it.
* Multiple partnerships. Any number of family groups per person, each with its own partner, dates,
  children, and a partnership kind (Married, Unmarried, Civil Union, Unknown). Partners are not
  assumed to be male and female.
* Step and half relatives are worked out automatically (step-parents, half-siblings, step-siblings,
  step-children) and shown on the person's page. Nothing needs to be entered twice.
* Other relationships (guardian, godparent, foster carer, mentor, witness...) are recorded under
  "Other relationships" and are saved in GEDCOM as ASSO records.
* The Relationship finder names blood and legal links separately, and the Data Checker flags more
  than one biological set of parents, loops (someone as their own ancestor), and duplicate
  partnerships.
* GEDCOM: everything above is written to and read from GEDCOM (PEDI, _FREL, _MREL, _PREF, _STAT,
  ASSO/RELA). Programs that ignore the custom tags still see the standard FAMC/FAMS links.


FINDING YOUR WAY AROUND A LARGE TREE
------------------------------------
* Fit: the "Fit" button next to the zoom controls (or View > Fit chart to window) sizes the chart to
  show everything in the window. Zoom now goes down to 25%.
* Centre on a person: click a box to highlight it, double-click to edit, right-click > "Centre chart
  on this person" to make them the middle of the chart.
* Home person: choose Tools > Home person... and search for anyone (or right-click a person in a chart
  and choose "Make this the home person"), then the "Home" button, View > Go to home person, or the
  Home key takes you straight back.
* Trail: a line above the chart lists the people you have visited. Click any name to go back to it.
* Selected person: the person at the centre has a bold outline; a highlighted person has a thick
  coloured border and a tinted box.
* Moving between generations: the Parents, Siblings, Partners and Children buttons above the chart
  list the relatives of the selected person (with separate groups for adoptive, step or foster
  parents and for half-brothers and sisters). The arrow on a box at the end of the chart goes further back.
* Keyboard: Right arrow = father (Shift+Right = mother), Left arrow = a child (the one you just came
  from, if you went up), Up/Down = previous/next brother or sister, Home = home person.
  Arrow keys do nothing while you are typing in a box.
* Speed: the data checker, duplicate finder, descendants chart, search and lists were reworked and are
  tested with generated trees of 5,000 people (and timed at 20,000). The descendants chart draws only
  what is on screen.


EDITING A PERSON SAFELY
-----------------------
* A person's Individual page opens LOCKED: nothing can be typed or changed until you press Edit.
  This protects your tree from stray keystrokes (a cat on the keyboard, a click in the wrong box).
* While editing, a bar shows Save and Cancel. Cancel puts the person back exactly as they were when
  you pressed Edit. If you click another person or tab with unsaved changes, the program asks
  whether to Save, Discard or Keep editing.
* Adding a person (Add menu, or Ctrl/Cmd+N) opens a larger window with name, sex, birth, death, burial,
  living and notes, so a new person can be filled in at once. Only the name is required.


THE FAMILY TAB
--------------
* Left: the selected person's family. Husband and wife as compact cards (portrait or initial, name, years,
  birthplace), the marriage line (kind, date, place) and a children table with Born, Died and Parent link
  (Birth, Adopted, Step, Foster...). Someone with several partnerships gets one button per partner above the cards.
* Right: Relationships. Parents (each labelled Biological, Adopted, Step...), Siblings (half-siblings marked),
  Partners and children for every partnership, and Other relationships (guardian, godparent...).
* Click any card or name to go to that person. In a narrow window the Relationships panel moves below the family.


THE INDIVIDUAL PAGE AND ITS PANELS
----------------------------------
* The main area (left) holds the person's core facts and a timeline. The rest lives in panels on the
  right: Relationships (parents, family groups, step and half relatives, other relationships), Events,
  Sources, Notes, Other names, Photos and recordings, and Merge and delete.
* The "Panels" row above the page shows or hides each panel. Each panel has a title bar with ^ and v to
  move it up or down, Float to pop it out into its own window (drag it anywhere, even onto a second screen),
  and x to hide it. Click a title to fold a panel up. Drag the edge of the dock to make it wider.
* Which panels are open, their order, and which are floating are remembered. "Reset layout" restores the defaults.
* In a narrow window the panels move below the main area. The page is still locked until you press Edit.

PEDIGREE BOX ACTIONS
Each pedigree box has a portrait (or initial) and a "..." button; right-click gives the same menu:
Open individual page, View their tree (ancestors / descendants / family), Quick edit, Add a relative
(father, mother, another set of parents, brother or sister, spouse, child), Centre chart, Make home person.
Below the main person are "Spouse & children" and "Siblings" drop-downs.

GALLERY
The Gallery tab (Ctrl/Cmd+5) shows every photo in the tree. Filter by name or caption, or show only the
selected person. Click a photo to view it, or the name to open that person.

READING A LOCKED PAGE
When the Individual page is locked it shows a clean profile instead of greyed-out boxes: portrait, name,
years and sex at the top, then only the facts that are filled in. Relatives are links you can click.
Press Edit to get the full set of boxes back.

IMPORTING FROM ANCESTRY
Ancestry GEDCOM files hold links to photos on Ancestry's website, not the photos. The links are kept and
exported again, and each shows "Open online". Download the pictures from Ancestry and add them with
"Add photos or documents" to keep them on this computer.

NEW TOOLS (see Help > User guide, F1, for details)
* Quick jump: Ctrl/Cmd+K or F4, type a name or a command.
* Keyboard: F1 help, F2 edit, F3 find, F4 quick jump, F5 fit chart, F6 next tab, F7 data checker,
  F8 to-do list, F9 places, F11 full screen, F12 history. On a Mac hold fn for F keys.
* Research to-do panel on each person, and Tools > Research to-do list.
* Tools > Places: who was where, and fixing places spelled more than one way.
* Photos: Tag faces, then "Use as their profile picture".
* Tools > History of changes and Tools > Saved versions (daily copies).
* Charts tab: fan chart, hourglass, lifespans. People tab: everyone in a sortable table (save as CSV).
* View: text size, name and date styles, chart box contents, full screen.
* Recent people and Bookmarks above the chart.

MAP, MEMORIES AND PRINTING
* Map tab: pins for where people were born, married, died, were buried and lived. Works offline.
  Places it cannot find are listed: pick one and click where it is on the map.
* Memories tab: stories, recordings, documents and PDFs, notes. Gallery: photos. Both have + Add.
* Printouts fit the page, with page numbers and tidy dates. Reports > Keep living people private.
* View > Colours: large colour swatches. The window remembers its size.
* Map data: Natural Earth (public domain). Town names: GeoNames (CC BY 4.0). Rebuild with tools/make_mapdata.py.

ARCHIVE, RESEARCH AND PRACTICE
* Welcome screen: import (PAF steps included), create new, open an archive, or explore the sample family.
* Help > Explore the sample family: practice mode, nothing is saved.
* File > Save / Open Family Tree Archive: one zip with everything. File > Family trees: separate trees.
* Tools > Backup settings: how often, how many, and a second copy on a USB stick or drive.
* Confidence ratings on every fact; Tools > Research journal; historical events in timelines;
  Tools > Family statistics; Reports > Report designer.

DNA, FORMS AND TOOLS (added 31 October 2026)
* DNA tab: ethnicity (table, bars, map), match lists with a "How related?" guide, and Y-DNA / mtDNA with the
  direct line drawn on the map. Match lists import from CSV, tab-separated and Excel (.xlsx) files.
* Reports > Forms: blank family group sheet, 4- and 5-generation pedigree charts (plain or LDS), and census
  worksheets for the US, Canada and England & Wales.
* Tools > Tombstone calculator: works out a birth date from a gravestone's death date and age.
* Import opens PAF 5 .paf files directly. For christenings, burials, ordinances, notes and sources, export a
  GEDCOM from PAF and import that instead.
* Map style (on the Map tab and the DNA map): Match the app, Charcoal, Warm grey, Slate blue or Paper.
