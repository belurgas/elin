; Elin NSIS hooks — keep the wizard quiet and leave a clean install.
; currentUser mode already writes to %LOCALAPPDATA%; no admin, no PATH rewrite.

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Elin is ready. Open it from the Start menu."
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
