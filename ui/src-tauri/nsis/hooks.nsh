; Create Companion — NSIS installer hooks (Tauri bundler).
; The engine (create-companion.exe) is a sidecar installed next to the UI.

!macro NSIS_HOOK_PREINSTALL
  ; Let the installer replace both executables.
  nsExec::ExecToLog 'taskkill /F /IM create-companion.exe'
  nsExec::ExecToLog 'taskkill /F /IM create-companion-ui.exe'
  Sleep 500
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Start the engine right away (it also registers start-at-login when the
  ; config asks for it).
  Exec '"$INSTDIR\create-companion.exe"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'taskkill /F /IM create-companion.exe'
  nsExec::ExecToLog 'taskkill /F /IM create-companion-ui.exe'
  Sleep 500
  ; The engine's start-at-login entry points at $INSTDIR; remove it.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CreateCompanion"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Configuration and logs are the user's; they stay in %APPDATA%\CreateCompanion
  ; and %LOCALAPPDATA%\CreateCompanion.
!macroend
