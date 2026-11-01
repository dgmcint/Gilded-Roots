GILDED ROOTS - PORTABLE VERSION

This copy keeps everything in the FamilyTreeData folder next to the program, so it can live
on a USB stick or external drive and be used on any computer of the same kind (Windows, Mac
or Linux). Nothing is written to the computer you plug it into.

  Gilded Roots program      (GildedRoots.exe on Windows, Gilded Roots.app on Mac,
                             gilded-roots on Linux)
  FamilyTreeData/           your tree, photos, documents, backups and settings

Keep the program and the FamilyTreeData folder side by side. Copy the whole folder to back
it up, or use File > Save Family Tree Archive for a single-file copy.

Mac: the first time, right-click Gilded Roots.app and choose Open (it is not signed by Apple).
If the program says it is not running in portable mode, open Terminal and run, once:
    xattr -dr com.apple.quarantine "/Volumes/YOUR DRIVE/Gilded Roots/Gilded Roots.app"
(drag the app onto the Terminal window to fill in its path). macOS otherwise runs downloaded
programs from a hidden temporary place, away from the FamilyTreeData folder.

Linux: the computer needs the usual desktop libraries (GTK 3); most desktops already have them.
Make the program runnable with:  chmod +x gilded-roots

Safely eject the drive before unplugging it, so the last save finishes.
