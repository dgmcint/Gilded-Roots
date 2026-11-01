# Releasing Gilded Roots

How to put a new version on GitHub with installers for Windows, Mac and Linux. The installers are built by
GitHub (the **Build installers** workflow in `.github/workflows/installers.yml`), not on your own computer.

## 1. Before uploading

- [ ] `Cargo.toml` has the right `version` (1.0.0 for the first release). Help > About shows this number.
- [ ] `CHANGELOG.md` has a section for this version.
- [ ] In Finder, delete the stray file `.gitignore copy` from `ft-rust` (the real `.gitignore` already has
      everything in it).
- [ ] Never upload the `target` folder (it is huge and is rebuilt by GitHub). `.gitignore` keeps it out when
      using git.

Hidden files: `.github` and `.gitignore` start with a dot, so Finder hides them. Press **Cmd + Shift + .**
in Finder to show them. They must be uploaded: `.github` is what builds the installers.

## 2. Upload the code to GitHub

**On the website:** open the repository, choose **Add file > Upload files**, and drag in everything inside
`ft-rust` except `target` (including `.github`, `.gitignore`, `packaging`, `assets`, `icons`, `docs`,
`src`, `tools`, `tests_data` and the files at the top level). Write a message such as "Version 1.0.0" and
choose **Commit changes** to `main`.

**Or with git** (Terminal, in the `ft-rust` folder):

```
git add -A
git commit -m "Version 1.0.0"
git push
```

Files that must be on GitHub for the installers to build: `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/`,
`assets/`, `icons/`, `packaging/gilded-roots.iss`, `packaging/gilded-roots.desktop`,
`packaging/PORTABLE-README.txt`, `LICENSE`, `README.txt`, `THIRD_PARTY_LICENSES.txt` and
`.github/workflows/installers.yml`.

## 3. Test build (recommended)

1. On GitHub open **Actions > Build installers > Run workflow**, keep `main`, and choose **Run workflow**.
2. Wait for all four jobs (Windows, Mac Apple Silicon, Mac Intel, Linux) to turn green. About 15 to 30 minutes.
3. Open the finished run and download the artifacts at the bottom. Try at least the Mac `.dmg` on your Mac:
   open it, import a GEDCOM and a `.paf`, print a blank form, open the DNA tab.
4. If a job is red, open it, copy the red error lines, and bring them to Claude.

A test build does not publish anything.

## 4. Publish the release

1. On GitHub open **Releases > Draft a new release**.
2. **Choose a tag:** type `v1.0.0` and choose **Create new tag: v1.0.0 on publish**. Target: `main`.
3. **Release title:** `Gilded Roots 1.0.0`.
4. **Description:** paste the release notes below.
5. Choose **Publish release**.

Publishing creates the tag, the tag starts the **Build installers** workflow, and when it finishes it attaches
the installers to the release by itself. After about 20 to 30 minutes the release should list these 8 files:

| File | For |
| --- | --- |
| `GildedRoots-Setup.exe` | Windows installer |
| `GildedRoots-portable-windows.zip` | Windows, USB drive |
| `GildedRoots-apple-silicon.dmg` | Mac with an M1 or later chip |
| `GildedRoots-intel.dmg` | Older Intel Mac |
| `GildedRoots-portable-mac-apple-silicon.zip` | Mac (M chip), USB drive |
| `GildedRoots-portable-mac-intel.zip` | Mac (Intel), USB drive |
| `GildedRoots-linux-amd64.deb` | Linux (Ubuntu, Debian, Mint) |
| `GildedRoots-portable-linux-x86_64.tar.gz` | Linux, USB drive |

If no run appears under **Actions** a few minutes after publishing: **Actions > Build installers > Run
workflow**, set **Use workflow from** to **Tags > v1.0.0**, and run it. Started from a tag, it still attaches
the files to the release.

## 5. After publishing

- [ ] Download one installer from the release page itself and open it, to check the public link works.
- [ ] Repository **About** (gear icon on the main page): a short description, for example "Free, offline family
      history program for Windows, Mac and Linux. A modern replacement for PAF.", and topics such as
      `genealogy`, `family-tree`, `gedcom`, `paf`, `rust`.
- [ ] **Settings > General > Features:** Issues ticked (the README sends bug reports there). The Sponsor
      button comes from `.github/FUNDING.yml`.

## Later versions

1. Change `version` in `Cargo.toml` (for example 1.0.1 for fixes, 1.1.0 for new features).
2. Add a section at the top of `CHANGELOG.md`.
3. Upload as in step 2, then publish a release with the matching tag (`v1.0.1`).

The Windows installer keeps the same AppId in every version, so a new version installs over the old one.
Nobody's family tree is touched: it lives in `Documents/Gilded Roots`, which installing or uninstalling never
removes.

## Release notes for 1.0.0 (paste into the release description)

```
Gilded Roots 1.0.0, the first public release.

A free, offline family history program for Windows, Mac and Linux, made as a modern replacement for
PAF (Personal Ancestral File). No account, no subscription, no advertising, nothing uploaded.

Highlights
- Import your tree from a GEDCOM file (PAF, Ancestry, FamilySearch, MyHeritage, RootsMagic, Legacy and
  others), or open a PAF 5 .paf file directly. Every import comes with a report of what came across.
- Pedigree, family, individual and descendant views; fan, hourglass and lifespan charts; reports and
  printable family group sheets.
- Sources and citations, research journal, to-do lists, places explorer, duplicate finder and data checker.
- Photos with face tagging, documents, a PDF viewer and recorded family stories.
- Offline map of where your family lived.
- DNA: ethnicity, match lists with a "How related?" guide, and Y-DNA / mtDNA with the direct line on the map.
- Printable blank forms: family group sheets, 4- and 5-generation pedigrees (plain or LDS) and census
  worksheets for the US, Canada and England and Wales.
- Tombstone calculator: a birth date from the age on a gravestone.
- Safe saving, daily backups, Family Tree Archives, and portable versions for a USB drive.

Which file do I download?
- Windows: GildedRoots-Setup.exe
- Mac: GildedRoots-apple-silicon.dmg (M1 or newer) or GildedRoots-intel.dmg (older Macs)
- Linux: GildedRoots-linux-amd64.deb
- To run from a USB drive: the matching "portable" file.

The installers are not signed. Windows: choose More info > Run anyway. Mac: open the app once, click Done at
the warning, then choose System Settings > Privacy & Security > Open Anyway (on macOS 14 or older,
right-click the app and choose Open).

Coming from PAF? A .paf file brings in names, births, deaths, marriages and family links. For christenings,
burials, LDS ordinances, notes and sources, export a GEDCOM from PAF (File > Export) and import that instead.

Full list of changes: CHANGELOG.md. How to use it: docs/USER_GUIDE.md.
```
