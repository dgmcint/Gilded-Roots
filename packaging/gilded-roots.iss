; Inno Setup script for the Windows installer. Built by .github/workflows/installers.yml:
;   cargo build --release
;   ISCC.exe packaging\gilded-roots.iss
; Paths are relative to this file. The result is dist\GildedRoots-Windows-Setup.exe.
;
; One installer, two ways to use it (chosen on the "How do you want to use Gilded Roots?" page):
;   * Install on this computer: Start menu, optional desktop icon, uninstall from Settings; the
;     family tree is kept in Documents\Gilded Roots.
;   * Portable: the program goes into a folder the user picks (a USB drive, say) with a
;     FamilyTreeData folder beside it, where everything is kept. Nothing is added to the computer:
;     no Start menu entry, no uninstaller, no registry entry.

#define AppName "Gilded Roots"
#define AppExe "gilded-roots.exe"
; The version comes from Cargo.toml (winres stamps it into the .exe).
#define AppVersion GetVersionNumbersString(AddBackslash(SourcePath) + "..\target\release\gilded-roots.exe")
#if AppVersion == ""
  #undef AppVersion
  #define AppVersion "1.0.0"
#endif

[Setup]
; Keep this AppId the same in every release, so a new version installs over the old one.
AppId={{6B1F2C47-8E3A-4D9B-A0C5-3F7E21D84B90}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Doyle McIntosh (DGMCINT)
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; Always show the folder page, so a portable copy can be put on a USB drive.
DisableDirPage=no
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=GildedRoots-Windows-Setup
SetupIconFile=..\icons\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
LicenseFile=..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; A portable copy leaves nothing behind on the computer: no uninstaller and no entry in Settings.
Uninstallable=not IsPortable
CreateUninstallRegKey=not IsPortable

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Check: not IsPortable

[Dirs]
Name: "{app}\FamilyTreeData"; Check: IsPortable

[Files]
Source: "..\target\release\gilded-roots.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.pdf"; DestDir: "{app}"; Flags: ignoreversion; Check: not IsPortable
Source: "PORTABLE-README.pdf"; DestDir: "{app}"; DestName: "README.pdf"; Flags: ignoreversion; Check: IsPortable
Source: "PORTABLE-README.pdf"; DestDir: "{app}\FamilyTreeData"; DestName: "ABOUT-THIS-FOLDER.pdf"; Flags: ignoreversion onlyifdoesntexist; Check: IsPortable

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; Check: not IsPortable
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon; Check: not IsPortable

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

; The family tree lives in Documents\Gilded Roots (or in FamilyTreeData for a portable copy) and is
; never removed by uninstalling.

[Code]
var
  TypePage: TInputOptionWizardPage;
  NormalDir: String;

procedure InitializeWizard;
begin
  TypePage := CreateInputOptionPage(wpLicense,
    'How do you want to use Gilded Roots?',
    'Choose one. You can always run this installer again to set it up the other way.',
    'Install on this computer: adds Gilded Roots to the Start menu, keeps your family tree in ' +
    'Documents\Gilded Roots, and can be uninstalled from Settings.' + #13#10 + #13#10 +
    'Portable: puts Gilded Roots in a folder you choose, such as a USB drive, and keeps your ' +
    'family tree, photos and backups in a FamilyTreeData folder beside it. Nothing is added to ' +
    'this computer, so you can take it to any Windows computer.',
    True, False);
  TypePage.Add('Install on this computer (recommended)');
  TypePage.Add('Portable: on a USB drive or in a folder I choose');
  TypePage.Values[0] := True;
end;

function IsPortable: Boolean;
begin
  Result := (TypePage <> nil) and TypePage.Values[1];
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if (TypePage <> nil) and (CurPageID = TypePage.ID) then
  begin
    // remember the normal folder (or the one a previous version used) the first time round
    if NormalDir = '' then
      NormalDir := WizardForm.DirEdit.Text;
    if IsPortable then
      WizardForm.DirEdit.Text := ExpandConstant('{userdocs}\Gilded Roots Portable')
    else
      WizardForm.DirEdit.Text := NormalDir;
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpSelectDir then
  begin
    if IsPortable then
      WizardForm.SelectDirLabel.Caption :=
        'Choose where to put the portable copy. To use it on a USB drive, click Browse and pick ' +
        'the drive (for example E:\Gilded Roots). Your family tree will be kept in a ' +
        'FamilyTreeData folder inside it.'
    else
      WizardForm.SelectDirLabel.Caption :=
        'Setup will install Gilded Roots into the following folder. Your family tree is kept ' +
        'separately, in Documents\Gilded Roots.';
  end;
end;
