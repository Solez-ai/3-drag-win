#define MyAppName "3-win-drag"
#define MyAppPublisher "Samin Yeasar"
#define MyAppURL "https://github.com/Solez-ai/3-drag-win"
#define MyAppExeName "3-win-drag.exe"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{8D8B784A-5E30-4775-85B4-D82D2F0BEF0C}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppVerName={#MyAppName} {#AppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppCopyright=Made by Samin Yeasar
AppComments=Open source MIT licensed Windows touchpad drag utility
VersionInfoVersion={#AppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=3-win-drag installer
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\3-win-drag
DefaultGroupName=3-win-drag
DisableProgramGroupPage=no
LicenseFile=..\LICENSE
InfoBeforeFile=..\dist\installer-assets\INSTALLER_README.txt
OutputDir=..\dist\installer
OutputBaseFilename=3-win-drag-setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
WizardImageFile=..\dist\installer-assets\wizard-side.bmp
WizardSmallImageFile=..\dist\installer-assets\wizard-top.bmp
SetupIconFile=..\dist\installer-assets\3-win-drag.ico
UninstallDisplayIcon={app}\3-win-drag.exe
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "..\target\release\3-win-drag.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\logo.png"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\3-win-drag"; Filename: "{app}\3-win-drag.exe"
Name: "{group}\README"; Filename: "{app}\README.md"
Name: "{group}\Uninstall 3-win-drag"; Filename: "{uninstallexe}"
Name: "{autodesktop}\3-win-drag"; Filename: "{app}\3-win-drag.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\3-win-drag.exe"; Description: "Launch 3-win-drag now"; Flags: nowait postinstall skipifsilent

[Code]
var
  GesturePage: TWizardPage;
  GestureHeadline: TNewStaticText;
  GestureBody: TNewStaticText;
  GestureCheck: TNewCheckBox;
  OpenTouchpadSettingsButton: TNewButton;

procedure OpenTouchpadSettingsButtonClick(Sender: TObject);
var
  ResultCode: Integer;
begin
  ShellExec('open', 'ms-settings:devices-touchpad', '', '', SW_SHOWNORMAL, ewNoWait, ResultCode);
end;

procedure InitializeWizard;
begin
  GesturePage :=
    CreateCustomPage(
      wpLicense,
      'Disable Conflicting Windows Gestures',
      'Turn off the built-in three-finger touchpad actions before installation continues.'
    );

  GestureHeadline := TNewStaticText.Create(GesturePage);
  GestureHeadline.Parent := GesturePage.Surface;
  GestureHeadline.Caption :=
    '3-win-drag works best when Windows is not still using three-finger gestures for task switching, search, or desktop actions.';
  GestureHeadline.Left := ScaleX(0);
  GestureHeadline.Top := ScaleY(0);
  GestureHeadline.Width := ScaleX(418);
  GestureHeadline.Height := ScaleY(36);
  GestureHeadline.WordWrap := True;

  GestureBody := TNewStaticText.Create(GesturePage);
  GestureBody.Parent := GesturePage.Surface;
  GestureBody.Caption :=
    'Before clicking Next:' + #13#10 + #13#10 +
    '1. Open Settings.' + #13#10 +
    '2. Go to Bluetooth & devices > Touchpad.' + #13#10 +
    '3. Open Three-finger gestures.' + #13#10 +
    '4. Set the built-in three-finger actions to Nothing or disable them.' + #13#10 +
    '5. Return to this installer and continue.';
  GestureBody.Left := ScaleX(0);
  GestureBody.Top := ScaleY(46);
  GestureBody.Width := ScaleX(418);
  GestureBody.Height := ScaleY(96);
  GestureBody.WordWrap := True;

  OpenTouchpadSettingsButton := TNewButton.Create(GesturePage);
  OpenTouchpadSettingsButton.Parent := GesturePage.Surface;
  OpenTouchpadSettingsButton.Caption := 'Open Windows Touchpad Settings';
  OpenTouchpadSettingsButton.Left := ScaleX(0);
  OpenTouchpadSettingsButton.Top := ScaleY(154);
  OpenTouchpadSettingsButton.Width := ScaleX(210);
  OpenTouchpadSettingsButton.Height := ScaleY(26);
  OpenTouchpadSettingsButton.OnClick := @OpenTouchpadSettingsButtonClick;

  GestureCheck := TNewCheckBox.Create(GesturePage);
  GestureCheck.Parent := GesturePage.Surface;
  GestureCheck.Caption :=
    'I understand that Windows three-finger gestures should be disabled before I continue.';
  GestureCheck.Left := ScaleX(0);
  GestureCheck.Top := ScaleY(198);
  GestureCheck.Width := ScaleX(418);
  GestureCheck.Height := ScaleY(36);
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;

  if CurPageID = GesturePage.ID then
  begin
    if not GestureCheck.Checked then
    begin
      MsgBox(
        'Disable the built-in Windows three-finger touchpad gestures before continuing with the installation.',
        mbError,
        MB_OK
      );
      Result := False;
    end;
  end;
end;
