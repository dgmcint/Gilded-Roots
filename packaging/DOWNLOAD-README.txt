GILDED ROOTS - free family history software
============================================

Free and open source (MIT licence). No account, no subscription, no adverts. Your family tree
stays on your computer and the program never connects to a server.

There is one download for each kind of computer. Each one can either install Gilded Roots on
the computer or set up a portable copy that runs from a USB drive.

  GildedRoots-Windows-Setup.exe   Windows 10 or 11, 64-bit
  GildedRoots-Mac.dmg             Any Mac (Apple chip or Intel), macOS 11 or newer
  GildedRoots-Linux.tar.gz        Linux, 64-bit (Ubuntu, Debian, Mint, Fedora and similar)


WINDOWS
-------
1. Run GildedRoots-Windows-Setup.exe.
   If Windows says "Windows protected your PC", click More info, then Run anyway. (The
   installer isn't signed with a paid certificate, so Windows warns about it.)
2. Choose how you want to use it:
   - Install on this computer: adds Gilded Roots to the Start menu. Uninstall from Settings.
   - Portable: pick a folder, for example your USB drive. Everything is kept in a
     FamilyTreeData folder beside the program, and nothing is added to the computer.


MAC
---
1. Open GildedRoots-Mac.dmg. It works on every Mac, with an Apple chip or Intel.
2. To install: drag Gilded Roots onto the Applications folder.
   For a portable copy: drag the "Portable (USB drive)" folder onto your USB drive.
3. The first time you open it, macOS says it can't check the developer (the app isn't signed
   by Apple). Click Done, then open System Settings > Privacy & Security, scroll down and
   click Open Anyway next to Gilded Roots. On macOS 14 or older, right-click the app and
   choose Open instead. You only need to do this once.
4. Portable copy only: if the program says it is running from a hidden temporary folder, open
   Terminal and run this once (drag the app onto the Terminal window to fill in its location):
       xattr -dr com.apple.quarantine "/Volumes/YOUR DRIVE/Portable (USB drive)/Gilded Roots.app"


LINUX
-----
1. Unpack GildedRoots-Linux.tar.gz (double-click it, or: tar -xzf GildedRoots-Linux.tar.gz).
2. To install: open the "Gilded Roots" folder in a terminal and run  ./install.sh
   It installs for you only, with a menu entry; no administrator password is needed.
   To remove it later:  ./install.sh --uninstall
   For a portable copy: copy the "Portable (USB drive)" folder onto your USB drive and run
   gilded-roots from there.
The computer needs the usual desktop libraries (GTK 3), which most desktops already have.


YOUR FAMILY TREE
----------------
- Installed: kept in Documents/Gilded Roots (your tree, photos, documents and backups).
  Help > Open data folder shows it. Uninstalling never removes it.
- Portable: kept in the FamilyTreeData folder next to the program. Keep the two together.
- To move to another computer: File > Save Family Tree Archive makes one file with everything;
  File > Open Family Tree Archive on the new computer.
- To bring a tree from PAF, Ancestry, FamilySearch or another program: choose "Import an
  existing family tree" on the welcome screen.

Help: press F1 in the program for the user guide.
Questions and problems: https://github.com/dgmcint/Gilded-Roots/issues

Gilded Roots is an independent project by DGMCINT. It is not affiliated with FamilySearch,
The Church of Jesus Christ of Latter-day Saints, Ancestry or any other company.
