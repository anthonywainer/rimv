!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\resources\runtime-dlls.txt" rimv_runtime_manifest_present rimv_runtime_manifest_missing
  rimv_runtime_manifest_present:
    CopyFiles /SILENT "$INSTDIR\resources\*.dll" "$INSTDIR"
    IfErrors rimv_runtime_copy_failed
    Goto rimv_runtime_copy_done
  rimv_runtime_manifest_missing:
    MessageBox MB_ICONSTOP "RimV's speech runtime files are missing. Setup cannot finish."
    Abort
  rimv_runtime_copy_failed:
    MessageBox MB_ICONSTOP "RimV could not install its speech runtime files. Setup cannot finish."
    Abort
  rimv_runtime_copy_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\*.dll"
!macroend
