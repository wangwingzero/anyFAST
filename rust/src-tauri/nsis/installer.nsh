; anyFAST NSIS Installer Header
; ============================================================================
; This file contains NSIS script definitions for auto-start functionality.
; 
; NOTE: Tauri uses hooks.nsi (configured in tauri.conf.json) for the actual
; installer hooks. This file serves as a reference implementation and can be
; used for custom NSIS builds or future enhancements.
;
; Requirements implemented:
; - Requirement 1.1: Register auto-start in Windows installer
; - Requirement 1.2: Start minimized to tray on system startup
; ============================================================================

!include "MUI2.nsh"
!include "nsDialogs.nsh"
!include "LogicLib.nsh"
!include "WinMessages.nsh"

; ============================================================================
; Constants
; ============================================================================

; Service constants
!define SERVICE_NAME "anyfast-service"
!define SERVICE_DISPLAY_NAME "anyFAST Hosts Service"
!define SERVICE_DESCRIPTION "Manages hosts file for anyFAST network optimization tool"

; Auto-start registry constants
; Registry path: HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Run
; This ensures the app starts automatically when the current user logs in
!define AUTOSTART_REG_KEY "SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
!define AUTOSTART_REG_NAME "anyFAST"

; ============================================================================
; Variables
; ============================================================================

; Variable to store checkbox control handle
Var AutoStartCheckbox
; Variable to store checkbox state (BST_CHECKED or BST_UNCHECKED)
Var AutoStartState

; ============================================================================
; Auto-start Registration Macros
; ============================================================================

; Macro: RegisterAutoStart
; Purpose: Write auto-start entry to Windows registry
; Requirement 1.1: Register auto-start in Windows installer
; Requirement 1.2: Start minimized to tray on system startup
;
; Registry entry format:
;   Key:   HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run
;   Name:  anyFAST
;   Value: "C:\Program Files\anyFAST\anyFAST.exe" --minimized
;
; The --minimized flag tells the application to start minimized to system tray
!macro RegisterAutoStart
  DetailPrint "Registering auto-start..."
  ; Write to HKEY_CURRENT_USER for per-user auto-start
  ; Using HKCU instead of HKLM to avoid requiring admin privileges for this setting
  WriteRegStr HKCU "${AUTOSTART_REG_KEY}" "${AUTOSTART_REG_NAME}" '"$INSTDIR\anyFAST.exe" --minimized'
  DetailPrint "Auto-start registered: $INSTDIR\anyFAST.exe --minimized"
!macroend

; Macro: UnregisterAutoStart
; Purpose: Remove auto-start entry from Windows registry
; Called during uninstallation to clean up registry entries
!macro UnregisterAutoStart
  DetailPrint "Removing auto-start registration..."
  DeleteRegValue HKCU "${AUTOSTART_REG_KEY}" "${AUTOSTART_REG_NAME}"
  DetailPrint "Auto-start registration removed."
!macroend

; ============================================================================
; Custom Installer Page: Auto-start Option
; ============================================================================

; Function: AutoStartPage
; Purpose: Display a custom page with checkbox for auto-start option
; This page allows users to choose whether to enable auto-start during installation
Function AutoStartPage
  !insertmacro MUI_HEADER_TEXT "开机自启动设置" "选择是否在系统启动时自动运行 anyFAST"
  
  nsDialogs::Create 1018
  Pop $0
  
  ${If} $0 == error
    Abort
  ${EndIf}
  
  ; Create checkbox for auto-start option
  ; Position: x=0, y=0, width=100%, height=12 units
  ${NSD_CreateCheckbox} 0 0 100% 12u "开机时自动启动 anyFAST (最小化到系统托盘)"
  Pop $AutoStartCheckbox
  
  ; Default to checked (enabled) - most users want auto-start
  ${NSD_Check} $AutoStartCheckbox
  
  ; Add description text explaining the feature
  ${NSD_CreateLabel} 0 20u 100% 36u "启用此选项后，anyFAST 将在 Windows 启动时自动运行，并最小化到系统托盘。$\r$\n$\r$\n您可以随时在应用设置中更改此选项。"
  Pop $0
  
  nsDialogs::Show
FunctionEnd

; Function: AutoStartPageLeave
; Purpose: Save the checkbox state when user leaves the page
Function AutoStartPageLeave
  ; Get checkbox state: BST_CHECKED (1) or BST_UNCHECKED (0)
  ${NSD_GetState} $AutoStartCheckbox $AutoStartState
FunctionEnd

; ============================================================================
; Service Installation Macros
; ============================================================================

; Macro: InstallService
; Purpose: Install and start the anyFAST background service
!macro InstallService
  ; Stop existing service if running
  DetailPrint "Stopping existing service..."
  nsExec::ExecToLog 'sc stop "${SERVICE_NAME}"'
  Pop $0

  ; Wait for service to stop
  Sleep 2000

  ; Delete existing service if present
  DetailPrint "Removing old service registration..."
  nsExec::ExecToLog 'sc delete "${SERVICE_NAME}"'
  Pop $0
  Sleep 1000

  ; Install new service with auto-start
  DetailPrint "Installing ${SERVICE_DISPLAY_NAME}..."
  nsExec::ExecToLog 'sc create "${SERVICE_NAME}" binPath= "$INSTDIR\anyfast-service.exe" start= auto DisplayName= "${SERVICE_DISPLAY_NAME}"'
  Pop $0

  ; Set service description
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

; Macro: UninstallService
; Purpose: Stop and remove the anyFAST background service
!macro UninstallService
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

; ============================================================================
; Installation Callbacks
; ============================================================================

; Function: .onInstSuccess
; Purpose: Called after successful installation
; Installs service and registers auto-start based on user choice
Function .onInstSuccess
  !insertmacro InstallService
  
  ; Register auto-start only if checkbox was checked
  ; Requirement 1.1: Register auto-start in Windows installer
  ${If} $AutoStartState == ${BST_CHECKED}
    !insertmacro RegisterAutoStart
  ${EndIf}
FunctionEnd

; Function: un.onUninstSuccess
; Purpose: Called after successful uninstallation
; Removes service and auto-start registry entry
Function un.onUninstSuccess
  !insertmacro UninstallService
  
  ; Always remove auto-start registry entry on uninstall
  ; This ensures clean removal even if user disabled auto-start in settings
  !insertmacro UnregisterAutoStart
FunctionEnd

; ============================================================================
; Page Registration (for custom NSIS builds)
; ============================================================================
; To use the custom auto-start page, add this to your NSIS script:
;
; !insertmacro MUI_PAGE_WELCOME
; !insertmacro MUI_PAGE_DIRECTORY
; Page custom AutoStartPage AutoStartPageLeave  ; Custom auto-start page
; !insertmacro MUI_PAGE_INSTFILES
; !insertmacro MUI_PAGE_FINISH
;
; Note: Tauri's NSIS integration uses hooks.nsi instead of custom pages.
; The auto-start functionality is implemented in hooks.nsi using
; NSIS_HOOK_POSTINSTALL and NSIS_HOOK_PREUNINSTALL macros.
; ============================================================================
