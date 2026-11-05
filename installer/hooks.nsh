; Tauri NSIS hooks: installs the service and extension alongside the app.
; Copied with File here, not Tauri "resources", so a plain `cargo build` doesn't need the
; release service and extension builds to exist.

!define MONK_ROOT "${__FILEDIR__}\.."

!macro NSIS_HOOK_PREINSTALL
  ; A running service can't be replaced. While locked it refuses to stop, and the files are skipped below.
  nsExec::Exec 'sc.exe stop MonkModeService'
  Pop $0
  Sleep 2000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  SetOutPath "$INSTDIR"
  SetOverwrite try
  File "/oname=monk-service.exe" "${MONK_ROOT}\target\release\monk-service.exe"
  SetOverwrite on
  SetOutPath "$INSTDIR\extension"
  File /r "${MONK_ROOT}\extension\dist\*"
  SetOutPath "$INSTDIR"

  nsExec::ExecToStack '"$INSTDIR\monk-service.exe" install'
  Pop $0
  Pop $1
  ${If} $0 != 0
    MessageBox MB_ICONEXCLAMATION|MB_OK "The Monk Mode service couldn't be installed:$\n$\n$1"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; The service's own binary decides, so the rule lives in one place. It refuses while locked.
  ; ponytail: this also blocks upgrades while locked, as Tauri upgrades run the old uninstaller.
  nsExec::ExecToStack '"$INSTDIR\monk-service.exe" uninstall'
  Pop $0
  Pop $1
  ${If} $0 != 0
    MessageBox MB_ICONSTOP|MB_OK "$1"
    SetErrorLevel 2
    Quit
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\monk-service.exe"
  RMDir /r "$INSTDIR\extension"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Monk Mode"
!macroend
