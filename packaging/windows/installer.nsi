; NSIS installer for Kooka Pictura (Windows x64).
;
; Built from the deployed staging folder produced by
; .github/actions/windows-bundle. Everything is passed in as defines so the
; script holds no absolute paths:
;   makensis /DSTAGE_DIR=<dir> /DVERSION=<x.y.z> /DOUTFILE=<path> installer.nsi
!ifndef STAGE_DIR
  !error "STAGE_DIR must be defined (the deployed staging folder)"
!endif
!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef OUTFILE
  !define OUTFILE "KookaPictura-${VERSION}-windows-x64-setup.exe"
!endif
!ifndef APP_ICON
  ; makensis cds into the script's directory, so this is relative to
  ; packaging/windows/, not the invocation cwd.
  !define APP_ICON "..\..\assets\icons\app.ico"
!endif
!ifndef VI_VERSION
  ; VIProductVersion wants four numeric parts; the workflow passes this.
  !define VI_VERSION "0.0.0.0"
!endif

Unicode true
Name "Kooka Pictura ${VERSION}"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\Kooka Pictura"
InstallDirRegKey HKLM "Software\KookaPictura" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "FileFunc.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"
!define MUI_FINISHPAGE_RUN "$INSTDIR\bin\pictura.exe"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

VIProductVersion "${VI_VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "Kooka Pictura"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "Kooka Pictura installer"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "GPL-3.0-or-later"

Function .onInit
  SetRegView 64
  SetShellVarContext all
FunctionEnd

; Reject a shared directory unless it is a previous Kooka Pictura install. The
; marker is written by this installer, so an unmarked directory belongs to
; someone else and the uninstaller would not be allowed to clean it anyway.
; Called from .onVerifyInstDir for interactive installs and again from the
; section, because the directory page (and its callback) is skipped in silent
; mode.
Function CheckInstallDir
  IfFileExists "$INSTDIR\.kooka-pictura-install" done
  FindFirst $0 $1 "$INSTDIR\*.*"
  loop:
    StrCmp $1 "" empty
    StrCmp $1 "." next
    StrCmp $1 ".." next
    FindClose $0
    MessageBox MB_OK|MB_ICONSTOP \
      "$INSTDIR is not empty and is not a Kooka Pictura installation. Choose an empty folder or a previous install directory."
    Abort
  next:
    FindNext $0 $1
    Goto loop
  empty:
    FindClose $0
  done:
FunctionEnd

Function .onVerifyInstDir
  Call CheckInstallDir
FunctionEnd

Section "Kooka Pictura" SecMain
  SetRegView 64
  Call CheckInstallDir
  SetOutPath "$INSTDIR"
  ; The staged tree is already self-contained: pictura.exe, the Qt DLLs, the
  ; platforms plugins, and the MSVC runtime that windeployqt copied.
  File /r "${STAGE_DIR}\*"
  FileOpen $0 "$INSTDIR\.kooka-pictura-install" w
  FileWrite $0 "${VERSION}$\r$\n"
  FileClose $0
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\Kooka Pictura"
  CreateShortcut "$SMPROGRAMS\Kooka Pictura\Kooka Pictura.lnk" "$INSTDIR\bin\pictura.exe" \
    "" "$INSTDIR\bin\pictura.exe" 0
  CreateShortcut "$DESKTOP\Kooka Pictura.lnk" "$INSTDIR\bin\pictura.exe" \
    "" "$INSTDIR\bin\pictura.exe" 0

  WriteRegStr HKLM "Software\KookaPictura" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "DisplayName" "Kooka Pictura"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "DisplayIcon" "$INSTDIR\bin\pictura.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "Publisher" "Kooka Pictura"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura" \
    "EstimatedSize" "$0"
SectionEnd

Section "Uninstall"
  SetRegView 64
  SetShellVarContext all
  ; Never wipe a directory that is not our install: the marker is written by
  ; this installer, so a user who pointed the installer at a shared or
  ; non-empty folder is not silently cleaned out. Quit rather than fall
  ; through: leaving the files must also leave the uninstall entry and
  ; shortcuts, otherwise the install is orphaned.
  IfFileExists "$INSTDIR\.kooka-pictura-install" 0 not_ours
    Delete "$SMPROGRAMS\Kooka Pictura\Kooka Pictura.lnk"
    RMDir "$SMPROGRAMS\Kooka Pictura"
    Delete "$DESKTOP\Kooka Pictura.lnk"
    RMDir /r "$INSTDIR"
    Goto remove_reg
  not_ours:
    MessageBox MB_OK|MB_ICONEXCLAMATION "$INSTDIR does not contain a Kooka Pictura install; nothing was removed."
    Quit
  remove_reg:
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura"
  DeleteRegKey HKLM "Software\KookaPictura"
SectionEnd
