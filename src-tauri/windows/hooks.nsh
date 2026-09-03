!macro NSIS_HOOK_PREUNINSTALL
  ldtg_retry_firewall_cleanup:
  ClearErrors
  ExecShellWait "runas" "$INSTDIR\ldtg-firewall-cleanup.exe" "" SW_HIDE
  IfErrors ldtg_firewall_cleanup_failed
  ClearErrors
  ExecWait '"$INSTDIR\ldtg-firewall-cleanup.exe"' $0
  IfErrors ldtg_firewall_cleanup_failed
  StrCmp $0 "0" ldtg_firewall_cleanup_done ldtg_firewall_cleanup_failed
  ldtg_firewall_cleanup_failed:
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Die LDTG-Firewallregel konnte nicht entfernt werden. Die Deinstallation wurde vor dem Löschen der Programmdateien angehalten." IDRETRY ldtg_retry_firewall_cleanup
  SetErrorLevel 1
  Abort
  ldtg_firewall_cleanup_done:
!macroend

; LDTG intentionally retains per-user configuration and logs during uninstall.
; These directories may contain a user-selected share, so recursive deletion is unsafe.
