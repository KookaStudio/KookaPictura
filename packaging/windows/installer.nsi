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

Function .onInit
  SetRegView 64
  SetShellVarContext all
FunctionEnd

Section "Kooka Pictura" SecMain
  SetRegView 64
  SetOutPath "$INSTDIR"
  ; The staged tree is already self-contained: pictura.exe, the Qt DLLs, the
  ; platforms plugins, and the MSVC runtime that windeployqt copied.
  File /r "${STAGE_DIR}\*"
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
  ; Never wipe a directory that is not our install: the sentinel only exists in
  ; a tree this installer created, so a user who pointed the installer at a
  ; shared or non-empty folder is not silently cleaned out.
  IfFileExists "$INSTDIR\bin\pictura.exe" 0 not_ours
    Delete "$SMPROGRAMS\Kooka Pictura\Kooka Pictura.lnk"
    RMDir "$SMPROGRAMS\Kooka Pictura"
    Delete "$DESKTOP\Kooka Pictura.lnk"
    RMDir /r "$INSTDIR"
    Goto remove_reg
  not_ours:
    MessageBox MB_OK|MB_ICONEXCLAMATION "$INSTDIR does not contain a Kooka Pictura install; nothing was removed."
  remove_reg:
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\KookaPictura"
  DeleteRegKey HKLM "Software\KookaPictura"
SectionEnd
