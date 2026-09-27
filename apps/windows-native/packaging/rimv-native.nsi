Unicode true
RequestExecutionLevel user
ManifestDPIAware true
SetCompressor /SOLID lzma
SetCompressorDictSize 64
ShowInstDetails show
ShowUninstDetails show

!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef PAYLOADDIR
  !error "PAYLOADDIR must point to the published native Windows application."
!endif
!ifndef ARTIFACTDIR
  !error "ARTIFACTDIR must point to the installer output directory."
!endif
!ifndef APP_VERSION
  !error "APP_VERSION must be supplied from the workspace version."
!endif
!ifndef NUMERIC_VERSION
  !error "NUMERIC_VERSION must be supplied as major.minor.patch.0."
!endif
!ifndef APP_ICON
  !error "APP_ICON must point to the RimV application icon."
!endif

!define PRODUCT_NAME "RimV Native Windows"
!define PRODUCT_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\RimV.Native.Windows"
!define APP_MUTEX "Local\RimV.Native.Windows.v01"

Name "${PRODUCT_NAME} ${APP_VERSION}"
OutFile "${ARTIFACTDIR}\RimV-${APP_VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\RimV Native Windows"
InstallDirRegKey HKCU "${PRODUCT_KEY}" "InstallLocation"
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
VIProductVersion "${NUMERIC_VERSION}"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "CompanyName" "RimV"
VIAddVersionKey "FileDescription" "RimV Native Windows Setup"
VIAddVersionKey "LegalCopyright" "Copyright (c) RimV contributors"

!define MUI_ABORTWARNING
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"
!define MUI_WELCOMEPAGE_TITLE "Welcome to RimV Native Windows Setup"
!define MUI_WELCOMEPAGE_TEXT "This will install RimV Native Windows ${APP_VERSION} for the current Windows user. Your recordings and models are stored separately and are kept when you uninstall."
!define MUI_DIRECTORYPAGE_TEXT_TOP "RimV will be installed in the following folder."
!define MUI_FINISHPAGE_RUN "$INSTDIR\RimV.Windows.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch RimV Native Windows"
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchRimV

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Var OldInstallDir
Var DesktopShortcut

Section "RimV Native Windows (required)" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  DetailPrint "Installing RimV Native Windows ${APP_VERSION}..."
  File /r "${PAYLOADDIR}\*"

  CreateDirectory "$SMPROGRAMS\RimV"
  CreateShortcut "$SMPROGRAMS\RimV\RimV Native Windows.lnk" "$INSTDIR\RimV.Windows.exe" "" "$INSTDIR\Assets\rimv.ico"
  ${If} $DesktopShortcut == "checked"
    CreateShortcut "$DESKTOP\RimV Native Windows.lnk" "$INSTDIR\RimV.Windows.exe" "" "$INSTDIR\Assets\rimv.ico"
  ${EndIf}

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "${PRODUCT_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKCU "${PRODUCT_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${PRODUCT_KEY}" "Publisher" "RimV"
  WriteRegStr HKCU "${PRODUCT_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${PRODUCT_KEY}" "DisplayIcon" "$INSTDIR\Assets\rimv.ico"
  WriteRegStr HKCU "${PRODUCT_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${PRODUCT_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${PRODUCT_KEY}" "NoRepair" 1
SectionEnd

Section /o "Create a desktop shortcut" SecDesktop
  StrCpy $DesktopShortcut "checked"
SectionEnd

Function .onInit
  StrCpy $DesktopShortcut "unchecked"
  System::Call 'kernel32::OpenMutexW(i 0x00100000, i 0, w "${APP_MUTEX}") p.r0'
  ${If} $0 != 0
    System::Call 'kernel32::CloseHandle(p r0)'
    MessageBox MB_OK|MB_ICONEXCLAMATION "RimV is running. Quit RimV from its tray menu, then run setup again."
    Abort
  ${EndIf}

  ReadRegStr $OldInstallDir HKCU "${PRODUCT_KEY}" "InstallLocation"
  ${If} $OldInstallDir != ""
    StrCpy $INSTDIR $OldInstallDir
    IfFileExists "$INSTDIR\Uninstall.exe" 0 done_upgrade
    DetailPrint "Removing the previous RimV application files..."
    ExecWait '"$INSTDIR\Uninstall.exe" /S' $1
    ${If} $1 != 0
      MessageBox MB_OK|MB_ICONSTOP "The previous RimV installation could not be removed (error $1). Close RimV and try again."
      Abort
    ${EndIf}
  ${EndIf}
  done_upgrade:
FunctionEnd

Function LaunchRimV
  ExecShell "open" "$INSTDIR\RimV.Windows.exe" "" SW_SHOWNORMAL
FunctionEnd

Function un.onInit
  ; Silent uninstaller launches may run from a temporary copy and do not
  ; reliably initialize $INSTDIR. Restore the recorded install location.
  ReadRegStr $INSTDIR HKCU "${PRODUCT_KEY}" "InstallLocation"
  ${If} $INSTDIR == ""
    MessageBox MB_OK|MB_ICONSTOP "RimV's install location could not be found. The application files were not removed."
    SetErrorLevel 1
    Abort
  ${EndIf}

  System::Call 'kernel32::OpenMutexW(i 0x00100000, i 0, w "${APP_MUTEX}") p.r0'
  ${If} $0 != 0
    System::Call 'kernel32::CloseHandle(p r0)'
    MessageBox MB_OK|MB_ICONEXCLAMATION "RimV is running. Quit RimV from its tray menu, then run uninstall again."
    Abort
  ${EndIf}
FunctionEnd

Section "Uninstall"
  DetailPrint "Removing RimV Native Windows application files and shortcuts..."
  Delete "$SMPROGRAMS\RimV\RimV Native Windows.lnk"
  RMDir "$SMPROGRAMS\RimV"
  Delete "$DESKTOP\RimV Native Windows.lnk"
  ; RMDir cannot remove the current working directory. Keep it outside $INSTDIR.
  SetOutPath "$TEMP"
  ClearErrors
  RMDir /r "$INSTDIR"
  IfErrors uninstall_failed
  ; User data lives under %LOCALAPPDATA%\RimV, outside $INSTDIR, and is preserved.
  DeleteRegKey HKCU "${PRODUCT_KEY}"
  Goto uninstall_done

  uninstall_failed:
    SetErrorLevel 1
  uninstall_done:
SectionEnd
