; anyFAST NSIS Installer Header
; This file is included by the Tauri-generated NSIS script
; to add custom service installation logic.

!include "MUI2.nsh"

; Service constants
!define SERVICE_NAME "anyfast-service"
!define SERVICE_DISPLAY_NAME "anyFAST Hosts Service"
!define SERVICE_DESCRIPTION "Manages hosts file for anyFAST network optimization tool"

; Macros for service installation
!macro InstallService
  ; Stop existing service if running
  nsExec::ExecToLog 'sc stop "${SERVICE_NAME}"'

  ; Wait for service to stop
  Sleep 2000

  ; Delete existing service if present
  nsExec::ExecToLog 'sc delete "${SERVICE_NAME}"'
  Sleep 1000

  ; Install new service
  nsExec::ExecToLog 'sc create "${SERVICE_NAME}" binPath= "$INSTDIR\anyfast-service.exe" start= auto DisplayName= "${SERVICE_DISPLAY_NAME}"'

  ; Set description
  nsExec::ExecToLog 'sc description "${SERVICE_NAME}" "${SERVICE_DESCRIPTION}"'

  ; Start the service
  nsExec::ExecToLog 'sc start "${SERVICE_NAME}"'
!macroend

!macro UninstallService
  ; Stop the service
  nsExec::ExecToLog 'sc stop "${SERVICE_NAME}"'

  ; Wait for service to stop
  Sleep 2000

  ; Delete the service
  nsExec::ExecToLog 'sc delete "${SERVICE_NAME}"'
!macroend

; Function called during installation
Function .onInstSuccess
  !insertmacro InstallService
FunctionEnd

; Function called during uninstallation
Function un.onUninstSuccess
  !insertmacro UninstallService
FunctionEnd
