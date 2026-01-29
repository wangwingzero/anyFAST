; anyFAST NSIS Installer Hooks
; ============================================================================
; This file is loaded by the Tauri NSIS installer (configured in tauri.conf.json)
; to add custom service installation and auto-start functionality.
;
; Requirements implemented:
; - Requirement 1.1: Register auto-start in Windows installer
; - Requirement 1.2: Start minimized to tray on system startup
;
; Registry entry for auto-start:
;   Key:   HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run
;   Name:  anyFAST
;   Value: "C:\Program Files\anyFAST\anyFAST.exe" --minimized
; ============================================================================

; Service constants
!define SERVICE_NAME "anyfast-service"
!define SERVICE_DISPLAY_NAME "anyFAST Hosts Service"
!define SERVICE_DESCRIPTION "Manages hosts file for anyFAST network optimization tool"

; Auto-start constants
; Requirement 1.1: Register auto-start in Windows installer
; Requirement 1.2: Start minimized to tray on system startup
!define AUTOSTART_REG_KEY "SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
!define AUTOSTART_REG_NAME "anyFAST"

; ============================================================================
; Pre-install hook - stop service BEFORE files are copied
; ============================================================================
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

; ============================================================================
; Post-install hook - install service and register auto-start after files are copied
; ============================================================================
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

  ; ========================================================================
  ; Register auto-start in Windows registry
  ; Requirement 1.1: Register auto-start in Windows installer
  ; Requirement 1.2: Start minimized to tray on system startup
  ; ========================================================================
  ; The --minimized flag ensures the app starts minimized to system tray
  ; Using HKCU (current user) instead of HKLM to avoid requiring admin privileges
  ; for this specific setting, allowing users to modify it in app settings later
  DetailPrint "Registering auto-start..."
  WriteRegStr HKCU "${AUTOSTART_REG_KEY}" "${AUTOSTART_REG_NAME}" '"$INSTDIR\anyFAST.exe" --minimized'
  
  DetailPrint "Auto-start registration completed."
!macroend

; ============================================================================
; Pre-uninstall hook - remove auto-start and service before files are deleted
; ============================================================================
!macro NSIS_HOOK_PREUNINSTALL
  ; Remove auto-start registry entry first
  DetailPrint "Removing auto-start registration..."
  DeleteRegValue HKCU "${AUTOSTART_REG_KEY}" "${AUTOSTART_REG_NAME}"

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
