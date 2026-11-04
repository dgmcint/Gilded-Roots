; Inno Setup script for the Windows installer. Built by .github/workflows/installers.yml:
;   cargo build --release
;   ISCC.exe packaging\gilded-roots.iss
; Paths are relative to this file. The result is dist\GildedRoots-Setup.exe.

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
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=GildedRoots-Setup
SetupIconFile=..\icons\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
LicenseFile=..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\gilded-roots.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

; The family tree lives in Documents\Gilded Roots and is never removed by uninstalling.
