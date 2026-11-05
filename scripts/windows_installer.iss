; The Windows installer, built with Inno Setup 6.
;
; It installs per user into %LOCALAPPDATA%\Programs so a silent update needs no
; elevation: the app can replace itself without an administrator prompt, which is
; what makes the in-app updater work on Windows the way it does on macOS.
;
; iscc /DAppVersion=0.1.9 /DSourceDir=<staged bundle> /DOutputDir=<dir> \
;      /DOutputBaseFilename=Opencut-Reinforced-0.1.9-windows-x64-setup scripts/windows_installer.iss

#define AppName "Opencut Reinforced"
#define AppExe "or_app.exe"

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "."
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif
#ifndef OutputBaseFilename
  #define OutputBaseFilename "Opencut-Reinforced-windows-x64-setup"
#endif

[Setup]
AppId={{6E2B7C0C-2F6B-4E2E-9A2D-2F1E7B7C1234}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=huou07
AppPublisherURL=https://github.com/huou07/Opencut-Reinforced
AppSupportURL=https://github.com/huou07/Opencut-Reinforced/issues
DefaultDirName={localappdata}\Programs\Opencut Reinforced
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseFilename}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\{#AppExe}
MinVersion=10.0

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[Code]
// The app keeps the bundled FFmpeg DLLs mapped for its whole lifetime, so an
// upgrade over a running copy cannot replace them: Setup retries, then reports
// "DeleteFile failed; code 5" and, with /VERYSILENT, aborts the install with
// exit code 5 and no message at all. Closing the app first makes the upgrade
// behave like the macOS one. Setup cannot prompt in silent mode, so it is done
// here rather than left to CloseApplications.
procedure StopRunningApp();
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM {#AppExe}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM or.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM or-whisper-provider.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  // Give the window server a moment to release the image sections.
  Sleep(400);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopRunningApp();
  Result := '';
end;
