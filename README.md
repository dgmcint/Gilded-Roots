# Gilded Roots

A modern, native family-history program for Windows, macOS and Linux, built as a replacement for
PAF (Personal Ancestral File). Your tree stays on your computer, in a visible folder, in an open format.

Written in Rust with [egui](https://github.com/emilk/egui). No account, no subscription, no cloud.

## Privacy first

Family history is personal. Gilded Roots is built so that it stays yours:

- **No account** and **no subscription**. Install it and start.
- **No advertising** and **no tracking**. The program never contacts a server.
- **Nothing is uploaded.** Your tree, photos and documents stay in a folder on your own computer
  (`Documents/Gilded Roots`), which you can move, back up or copy as you like.
- **You decide what living people's details leave your computer.** Printouts and reports can hide living
  people, and GEDCOM can be exported with living people kept private.
- **Works offline**, including the map and the built-in user guide.

## Moving from PAF

Gilded Roots was made for people coming from **Personal Ancestral File (PAF)**. Choose **Import an existing
family tree** and pick your PAF 5 `.paf` file: names, births, deaths, marriages and every family link are read
directly, with no need for PAF itself. If your file also has christenings, burials, LDS ordinances, notes,
sources or photos, export a GEDCOM from PAF instead (**File > Export**) and import that, so those come across too.
The import report shows exactly how many people, families, sources, citations, notes and photos came across,
and anything that could not be carried over. Your PAF file is never changed.

## Features

- **Safe by design.** Atomic saves with read-back verification, a kept previous version, daily
  checksummed backups for 14 days, automatic recovery from damaged files, and a visible
  `Documents/Gilded Roots` data folder that you can relocate.
- **GEDCOM in and out.** Opens PAF 5 `.paf` files directly. Reads GEDCOM 5.5 / 5.5.1 (UTF-8, UTF-16, ANSEL, Windows ANSI) from PAF, Ancestry,
  FamilySearch, MyHeritage, RootsMagic, Gramps and others. Anything the app has no field for is preserved and
  written back. Every import and export is checked and reported.
- **Real sources and citations.** Reusable source records, repositories, page/entry references, URLs, access
  dates, transcriptions and quality ratings, attached to people, events and families.
- **Safer merging.** Name and birth year are only clues. Merges happen on a side-by-side review screen that
  compares parents, spouses, children and conflicting facts, and needs your confirmation.
- **Complex families.** Several sets of parents, adoption, step, foster and guardian relationships kept
  separate from biological ones, multiple partnerships, derived step/half relatives, and associations such as
  godparent or guardian.
- **Charts, reports and tools.** Pedigree, family and descendant views, fan chart, hourglass and lifespan
  charts, a sortable People table (saves as CSV), relationship finder, data checker, duplicate finder, printable
  charts and group sheets, bibliography, photos, documents and recorded stories.
- **Research tools.** Research to-do lists (exported as `_TODO`), a Places explorer that finds and fixes places
  spelled more than one way, face tagging in photos with face-crop portraits, a built-in PDF viewer, an offline Map of where your family lived, Memories (stories,
  recordings, documents and notes), bookmarks and recent people.
- **Fast to get around.** Quick jump (Ctrl/Cmd+K) to any person or command, F-key shortcuts, a History of changes
  you can step back through, and saved daily versions you can browse and restore one person from.
- **Comfortable to read.** Adjustable text size, name and date styles, dockable panels with saved layouts, full screen.
- **Archive and protect.** One-file Family Tree Archives with every photo and document, backups on your
  schedule with a second copy on a USB stick or drive, saved daily versions, and a sample family to practise on.
- **Research tools.** Research journal, confidence ratings on every fact, historical events in timelines,
  family statistics, a smarter duplicate finder, a report designer, and separate trees for separate projects.
- **DNA.** Autosomal results (ethnicity table, bar chart and map; match lists with shared cM and a "How related?"
  guide), Y-DNA and mtDNA (haplogroup, markers or mutations, matches, and the direct line drawn on the map).
  Match lists import from CSV, tab-separated and Excel files exported by FTDNA, Ancestry, 23andMe, MyHeritage
  and others.
- **Printable forms.** Blank family group sheets and 4- or 5-generation pedigree charts (plain or with LDS
  ordinance boxes), and census worksheets for the US (1790–1950), Canada (1851–1931) and England and Wales
  (1841–1921).
- **Tombstone calculator.** Turns a gravestone's death date and age ("aged 72 years, 3 months, 10 days") into a
  birth date or range and saves it with a gravestone source.
- **Map** in a clean flat style that can match the app's colours, and wraps around the world.
- **Portable versions** for Windows, Mac and Linux that run from a USB drive with the data beside them.

## Install

Download the installer for your system from the
[Releases](../../releases) page (or from the **Actions → Build installers** run artifacts):

| System | File |
| --- | --- |
| Windows | `GildedRoots-Setup.exe` |
| macOS (Apple Silicon / Intel) | `GildedRoots-apple-silicon.dmg` / `GildedRoots-intel.dmg` |
| Linux | `GildedRoots-linux-amd64.deb` |

### Portable (runs from a USB drive)

| System | File |
| --- | --- |
| Windows | `GildedRoots-portable-windows.zip` |
| macOS (Apple Silicon / Intel) | `GildedRoots-portable-mac-apple-silicon.zip` / `GildedRoots-portable-mac-intel.zip` |
| Linux | `GildedRoots-portable-linux-x86_64.tar.gz` |

Unzip the **Gilded Roots** folder onto a USB stick or external drive. It holds the program and a
`FamilyTreeData` folder; everything (trees, photos, backups, settings) is kept there and nothing is written to the
computer it is plugged into. Keep the two side by side. On a Mac, if the program says portable mode cannot be used,
run `xattr -dr com.apple.quarantine` on the app once (details in the README inside the zip). An installed copy can
also be made portable with **Tools > Make portable**.

The installers are unsigned. On Windows choose **More info → Run anyway**. On a Mac, open the app once, click
**Done** when macOS warns about it, then go to **System Settings → Privacy & Security** and click **Open Anyway**
(on macOS 14 or older you can instead right-click the app and choose **Open**). You only need to do this once.

## Build from source

```
cargo run --release
```

Install [Rust](https://rustup.rs) 1.92 or newer (if you already have Rust, run `rustup update`). On Linux you also need GTK and graphics headers:

```
sudo apt install build-essential pkg-config libgtk-3-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libasound2-dev
```

On macOS run `xcode-select --install` first, and `bash build-mac.sh` to make a double-clickable app.
Full platform notes are in [`README.txt`](README.txt).

## Where your data lives

`Documents/Gilded Roots/tree.json`, plus `media/` and `backups/`. Move it with **Tools → Move my data
folder…**. If that folder is inside iCloud, OneDrive or Dropbox, the app warns you, because live syncing can
conflict with saves.

## Documentation

- [User guide](docs/USER_GUIDE.md): the menus and tools, step by step
- [Changelog](CHANGELOG.md): what has been built
- [Contributing](CONTRIBUTING.md): building, testing and project layout

## Support

Donations are entirely optional, go to the author personally, and are not tax-deductible.

Gilded Roots is free. If you enjoy it and would like to help with its development, a small donation, even $5,
is very much appreciated. It is entirely optional, and donations are voluntary gifts that do not buy features
or support. Thank you!

[Donate with PayPal](https://www.paypal.me/doylemcintosh).

## Questions and bug reports

Please use the **Issues** tab of this repository. Include your operating system, the Gilded Roots version
(Help > About) and what you were doing. Never attach your real tree file to a public issue: it contains
names and dates of living people. A small made-up example, or a screenshot, is enough.

## Trademarks

Gilded Roots is an independent project. It is not affiliated with, endorsed by or connected to FamilySearch,
The Church of Jesus Christ of Latter-day Saints, Ancestry, MyHeritage, RootsMagic or any other company named in
this project. Personal Ancestral File (PAF) and the other product names are trademarks of their owners and are
used only to describe compatibility (for example, "imports GEDCOM files exported from PAF").

## Author

Created and maintained by **DGMCINT**

## License

MIT. See [LICENSE](LICENSE). Gilded Roots is built with open-source libraries under permissive licences (MIT,
Apache-2.0, BSD, Zlib and similar); map data comes from Natural Earth (public domain) and GeoNames (CC BY 4.0). Their notices are in [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) and in the
program under **Help > Open-source licences**.
