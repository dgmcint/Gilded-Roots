# Contributing

## Build and test

```
cargo build --release
cargo test --release
```

To test against your own GEDCOM files (they are never uploaded or stored):

```
FT_GED_SAMPLES=/path/to/folder cargo test --release real_world -- --nocapture
```

Each file should report `LOST 0`.

## Layout

| Path | Purpose |
| --- | --- |
| `src/main.rs` | Start-up, and the list of the files below |
| `src/model.rs` | The data model: `Db`, `Person`, `Family`, events, sources, media |
| `src/genealogy.rs` | Names, dates, relationship words, merging people, places |
| `src/app.rs` | The `App` struct, dialogs, actions, undo and redo |
| `src/settings.rs` | Settings, colour schemes, the theme |
| `src/storage.rs` | Folders, saving and loading, atomic writes, backups, reading and writing GEDCOM files |
| `src/files.rs` | Import, export, archives, backups and switching trees (the menu actions) |
| `src/media.rs` | Photos, recordings, the Gallery and Memories |
| `src/reports.rs` | Printable reports (HTML) |
| `src/views.rs` | Menus, navigation, pedigree and family views |
| `src/person.rs` | The Individual page |
| `src/windows.rs` | Pop-up windows (renumber, versions, history, to-do, welcome, trees, places) |
| `src/jump.rs` | Keyboard shortcuts and quick jump |
| `src/update.rs` | The frame loop that draws the program |
| `src/help.rs` | The built-in User Guide |
| `src/tests.rs` | Tests for the files above |
| `src/gedcom.rs` | GEDCOM reader/writer, lossless raw blocks, merge engine, data migrations |
| `src/review.rs` | Merge review screens and import-merge confirmation |
| `packaging/` | Windows installer script and Linux `.desktop` file |
| `.github/workflows/installers.yml` | Cloud build of all installers |

## Guidelines

- Never lose user data: keep saves atomic and add a test for any new stored field's GEDCOM round trip.
- Bump `Db.ver` and add a `migrate` step when changing the saved format.
- Never decide automatically that two people are the same; always go through a review screen.
- Do not use the old program's name in the product name.

## Maintainer

Gilded Roots is created and maintained by Doyle McIntosh (DGMCINT). Contributions are welcome through pull requests.

## Third-party notices

When dependencies change, regenerate `THIRD_PARTY_LICENSES.txt` with `python3 tools/make_licenses.py` (it is built into the program and shown under
Help > Open-source licences). Only permissive licences (MIT, Apache-2.0, BSD, Zlib, ISC and similar) should be added;
check a new dependency's licence before using it.
