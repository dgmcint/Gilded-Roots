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
| `src/main.rs` | App, data model (`Db`, `Person`, `Family`), views, reports, saving and backups, tests |
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
