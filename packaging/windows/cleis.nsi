; Cleis installer for Windows (D-75), built by NSIS in release.yml:
;   makensis /DVERSION=0.2.1 /DSRC=dist\files /DOUT=dist\Cleis-0.2.1-abc1234-windows-x64-setup.exe cleis.nsi
; Per-user install (no administrator): %LOCALAPPDATA%\Programs\Cleis. The vault and settings stay in
; the user's profile; the uninstaller leaves them.
; Silent update from Cleis (D-74): `setup.exe /S /UPDATE` closes the running copy, installs, starts
; Cleis again.

Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel user

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !error "VERSION is required"
!endif
!ifndef SRC
  !error "SRC (folder of the files to install) is required"
!endif
!ifndef OUT
  !define OUT "Cleis-${VERSION}-setup.exe"
!endif

!define APP "Cleis"
!define PUBLISHER "MATV Studio"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Cleis"

Name "${APP}"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\Cleis"
InstallDirRegKey HKCU "Software\Cleis" "InstallDir"
BrandingText "${APP} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "${APP} installer"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

!define MUI_ICON "cleis.ico"
!define MUI_UNICON "cleis.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\Cleis.exe"
!define MUI_FINISHPAGE_RUN_TEXT "$(RunCleis)"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "French"

LangString RunCleis ${LANG_ENGLISH} "Start Cleis"
LangString RunCleis ${LANG_FRENCH} "Lancer Cleis"
LangString StillOpen ${LANG_ENGLISH} "Cleis is still open. Quit it from its icon in the notification area, then retry."
LangString StillOpen ${LANG_FRENCH} "Cleis est encore ouvert. Quittez-le depuis son icône dans la zone de notification, puis réessayez."

Var Update

; Asks the running copy to quit (it stops the server it started), then waits for it. Labels are
; local to the section that inserts the macro.
!macro QuitRunning
  IfFileExists "$INSTDIR\Cleis.exe" 0 +2
    ExecWait '"$INSTDIR\Cleis.exe" --quit'
  StrCpy $0 0
  ${Do}
    ClearErrors
    ; A running program cannot be opened for writing.
    FileOpen $1 "$INSTDIR\Cleis.exe" a
    ${IfNot} ${Errors}
      FileClose $1
      ${Break}
    ${EndIf}
    IfFileExists "$INSTDIR\Cleis.exe" 0 qr_done
    IntOp $0 $0 + 1
    ${If} $0 > 50
      ${If} ${Silent}
        Abort
      ${EndIf}
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "$(StillOpen)" IDRETRY qr_retry
      Abort
      qr_retry:
      StrCpy $0 0
    ${EndIf}
    Sleep 200
  ${Loop}
  qr_done:
!macroend

Function .onInit
  ${GetParameters} $0
  ClearErrors
  ${GetOptions} $0 "/UPDATE" $1
  ${IfNot} ${Errors}
    StrCpy $Update 1
  ${EndIf}
FunctionEnd

Section "Cleis" SecMain
  SectionIn RO
  !insertmacro QuitRunning
  SetOutPath "$INSTDIR"
  File "${SRC}\Cleis.exe"
  File "${SRC}\cleis-server.exe"
  File "${SRC}\README.md"
  File "${SRC}\SERVER.md"
  File "${SRC}\THIRD_PARTY.md"
  File "${SRC}\THIRD_PARTY_LICENSES.txt"
  File "${SRC}\OFL.txt"
  File "cleis.ico"
  ; D-78: Cleis Server for a NAS (QNAP packages, static Linux servers), sent there by Cleis.
  SetOutPath "$INSTDIR\server"
  File /nonfatal "${SRC}\server\*.*"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall Cleis.exe"

  CreateShortcut "$SMPROGRAMS\Cleis.lnk" "$INSTDIR\Cleis.exe" "" "$INSTDIR\cleis.ico"
  ${IfNot} ${Silent}
    CreateShortcut "$DESKTOP\Cleis.lnk" "$INSTDIR\Cleis.exe" "" "$INSTDIR\cleis.ico"
  ${EndIf}

  WriteRegStr HKCU "Software\Cleis" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\cleis.ico"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall Cleis.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall Cleis.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"

  ; An update started from Cleis: Cleis comes back by itself.
  ${If} $Update == 1
    Exec '"$INSTDIR\Cleis.exe"'
  ${EndIf}
SectionEnd

Section "Uninstall"
  !insertmacro QuitRunning
  Delete "$INSTDIR\Cleis.exe"
  Delete "$INSTDIR\cleis-server.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\SERVER.md"
  Delete "$INSTDIR\THIRD_PARTY.md"
  Delete "$INSTDIR\THIRD_PARTY_LICENSES.txt"
  Delete "$INSTDIR\OFL.txt"
  Delete "$INSTDIR\cleis.ico"
  Delete "$INSTDIR\*.old"
  RMDir /r "$INSTDIR\server"
  Delete "$INSTDIR\Uninstall Cleis.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Cleis.lnk"
  Delete "$DESKTOP\Cleis.lnk"
  ; Open at login (Preferences), written by Cleis itself.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Cleis"
  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "Software\Cleis"
SectionEnd
