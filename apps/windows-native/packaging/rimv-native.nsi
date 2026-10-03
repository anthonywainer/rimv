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
!ifndef PARAKEET_MODEL_SIZE_MB
  !error "PARAKEET_MODEL_SIZE_MB must come from the shared model catalog."
!endif
!ifndef WHISPER_BASE_MODEL_SIZE_MB
  !error "WHISPER_BASE_MODEL_SIZE_MB must come from the shared model catalog."
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
Var IdentityLog

Section "RimV Native Windows (required)" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  DetailPrint "Installing RimV Native Windows ${APP_VERSION}..."
  File /r "${PAYLOADDIR}\*"

  DetailPrint "Registering RimV's Windows AI package identity..."
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\Register-Identity.ps1" -Action CheckPublisherTrust -InstallLocation "$INSTDIR"' $0
  FileOpen $IdentityLog "$INSTDIR\RimV.IdentityInstaller.log" w
  FileWrite $IdentityLog "CheckPublisherTrust exit code: $0$\r$\n"
  FileClose $IdentityLog
  ${If} $0 == 0
    DetailPrint "RimV's identity publisher certificate is already trusted; no certificate-store changes are needed."
    FileOpen $IdentityLog "$INSTDIR\RimV.IdentityInstaller.log" a
    FileWrite $IdentityLog "Publisher trust check passed; attempting package registration.$\r$\n"
    FileClose $IdentityLog
    Goto register_identity
  ${EndIf}

  ; Silent installs do not change certificate trust, but still let AppX
  ; validate the package against the machine's existing trust configuration.
  IfSilent register_identity
  MessageBox MB_YESNO|MB_ICONQUESTION "Enable Windows Native Speech? Windows requires RimV's identity-package signing certificate (CN=RimV) in this PC's Trusted People store. Windows may ask for administrator approval. The certificate is not added to Trusted Root." IDYES trust_identity
  DetailPrint "Publisher certificate trust declined; Windows Native Speech will remain unavailable."
  Goto identity_unavailable

  trust_identity:
  IfFileExists "$INSTDIR\RimV.Identity.cer" 0 identity_trust_failed
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\Register-Identity.ps1" -Action TrustPublisher -InstallLocation "$INSTDIR"' $0
  FileOpen $IdentityLog "$INSTDIR\RimV.IdentityInstaller.log" a
  FileWrite $IdentityLog "TrustPublisher exit code: $0$\r$\n"
  FileClose $IdentityLog
  ${If} $0 != 0
    DetailPrint "Could not add RimV's publisher certificate to this PC's Trusted People store (error $0)."
    Goto identity_unavailable
  ${EndIf}

  register_identity:
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\Register-Identity.ps1" -Action Install -InstallLocation "$INSTDIR"' $0
  FileOpen $IdentityLog "$INSTDIR\RimV.IdentityInstaller.log" a
  FileWrite $IdentityLog "Identity package registration exit code: $0$\r$\n"
  FileClose $IdentityLog
  ${If} $0 != 0
    DetailPrint "Windows AI package identity registration failed (error $0); Native Speech will be unavailable."
    Goto identity_unavailable
  ${EndIf}
  Goto identity_registration_done

  identity_trust_failed:
  DetailPrint "The RimV identity-package publisher certificate is missing from the installation payload."
  identity_unavailable:
  IfSilent identity_registration_done
  MessageBox MB_ICONEXCLAMATION "RimV installed, but Windows Native Speech could not be enabled. Parakeet and Whisper remain available. To enable Native Speech later, trust $INSTDIR\RimV.Identity.cer in Current User > Trusted People, then run setup again."
  identity_registration_done:

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

Section /o "Parakeet TDT 0.6B v3 INT8 (approx. ${PARAKEET_MODEL_SIZE_MB} MB download)" SecParakeet
  DetailPrint "Downloading and verifying Parakeet and its shared VAD prerequisite through RimV Model Manager..."
  ExecWait '"$INSTDIR\RimV.Windows.exe" --install-model parakeet-tdt-0.6b-v3-int8' $0
  ${If} $0 != 0
    DetailPrint "Optional Parakeet setup or its required VAD prerequisite failed (error $0). RimV itself remains installed."
    IfSilent parakeet_failure_reported
    MessageBox MB_ICONEXCLAMATION "RimV installed successfully, but Parakeet setup or its shared VAD prerequisite failed. Any model that passed verification remains installed. Open Model Manager later to retry the failed model."
    parakeet_failure_reported:
  ${Else}
    DetailPrint "Parakeet installed and verified in RimV's model directory."
  ${EndIf}
SectionEnd

Section /o "Whisper Base (recommended, approx. ${WHISPER_BASE_MODEL_SIZE_MB} MB download)" SecWhisperBase
  DetailPrint "Downloading and verifying Whisper Base and its shared VAD prerequisite through RimV Model Manager..."
  ExecWait '"$INSTDIR\RimV.Windows.exe" --install-model whisper-base' $0
  ${If} $0 != 0
    DetailPrint "Optional Whisper Base setup or its required VAD prerequisite failed (error $0). RimV itself remains installed."
    IfSilent whisper_failure_reported
    MessageBox MB_ICONEXCLAMATION "RimV installed successfully, but Whisper Base setup or its shared VAD prerequisite failed. Any model that passed verification remains installed. Open Model Manager later to retry the failed model."
    whisper_failure_reported:
  ${Else}
    DetailPrint "Whisper Base installed and verified in RimV's model directory."
  ${EndIf}
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
    SetErrorLevel 2
    Abort
  ${EndIf}
FunctionEnd

Section "Uninstall"
  DetailPrint "Removing RimV Native Windows application files and shortcuts..."
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\Register-Identity.ps1" -Action Uninstall -InstallLocation "$INSTDIR"' $0
  ${If} $0 != 0
    DetailPrint "Windows did not report successful identity unregistration (error $0). Continuing file removal."
  ${EndIf}
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
