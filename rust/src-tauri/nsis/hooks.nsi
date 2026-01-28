; anyFAST NSIS Installer Hooks
; This file is loaded by the Tauri NSIS installer to add
; custom service installation and uninstallation logic.

; Service constants
!define SERVICE_NAME "anyfast-service"
!define SERVICE_DISPLAY_NAME "anyFAST Hosts Service"
!define SERVICE_DESCRIPTION "Manages hosts file for anyFAST network optimization tool"

; Pre-install hook - stop service BEFORE files are copied
!macro NSIS_HOOK_PREINSTALL
  ; Stop existing service if running
  DetailPrint "Stopping existing service..."
  nsExec::ExecToLog 'sc stop "${SERVICE_NAME}"'
  Pop $0

  ; Also kill any running processes directly
  nsExec::ExecToLog 'taskkill /F /IM anyfast-service.exe'
  Pop $0
  nsExec::ExecToLog 'taskkill /F /IM anyfast.exe'
  Pop $0

  ; Wait for processes to fully terminate
  Sleep 3000

  ; Delete existing service registration
  DetailPrint "Removing old service registration..."
  nsExec::ExecToLog 'sc delete "${SERVICE_NAME}"'
  Pop $0
  Sleep 1000
!macroend

; Post-install hook - install and start service after files are copied
!macro NSIS_HOOK_POSTINSTALL
  ; Install new service
  DetailPrint "Installing ${SERVICE_DISPLAY_NAME}..."
  nsExec::ExecToLog 'sc create "${SERVICE_NAME}" binPath= "$INSTDIR\anyfast-service.exe" start= auto DisplayName= "${SERVICE_DISPLAY_NAME}"'
  Pop $0

  ; Set description
  nsExec::ExecToLog 'sc description "${SERVICE_NAME}" "${SERVICE_DESCRIPTION}"'
  Pop $0

  ; Configure service recovery options (restart on failure)
  nsExec::ExecToLog 'sc failure "${SERVICE_NAME}" reset= 60 actions= restart/5000/restart/5000/restart/5000'
  Pop $0

  ; Start the service
  DetailPrint "Starting ${SERVICE_DISPLAY_NAME}..."
  nsExec::ExecToLog 'sc start "${SERVICE_NAME}"'
  Pop $0

  DetailPrint "Service installation completed."
!macroend

; Custom uninstallation - remove service before files are deleted
!macro NSIS_HOOK_PREUNINSTALL
  ; Stop the service
  DetailPrint "Stopping ${SERVICE_DISPLAY_NAME}..."
  nsExec::ExecToLog 'sc stop "${SERVICE_NAME}"'
  Pop $0

  ; Wait for service to stop
  Sleep 3000

  ; Delete the service
  DetailPrint "Removing ${SERVICE_DISPLAY_NAME}..."
  nsExec::ExecToLog 'sc delete "${SERVICE_NAME}"'
  Pop $0

  ; Wait for service to be fully removed
  Sleep 1000

  DetailPrint "Service removal completed."
!macroend
