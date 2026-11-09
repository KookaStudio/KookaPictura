# Kooka Pictura — M0 walking skeleton (M0-A).
#
# Rust owns the QObject and the image; CMake builds the Rust crate as a static
# library through Corrosion (via CXX-Qt-CMake) and links it into the C++ shell.

cmake_minimum_required(VERSION 3.24)

project(kooka_pictura VERSION 0.3.0 LANGUAGES CXX) # x-release-please-version

# Route local C++ compilation through sccache when it is installed. An explicit
# -DCMAKE_CXX_COMPILER_LAUNCHER or the same-named environment variable wins: the
# cache variable is seeded from the environment during project(), so this runs
# after a user override is already visible. The nested guard keeps a missing
# sccache from setting the launcher to SCCACHE_EXECUTABLE-NOTFOUND.
if(NOT CMAKE_CXX_COMPILER_LAUNCHER)
    find_program(SCCACHE_EXECUTABLE sccache)
    if(SCCACHE_EXECUTABLE)
        set(CMAKE_CXX_COMPILER_LAUNCHER "${SCCACHE_EXECUTABLE}")
        message(STATUS "C++ compiler launcher: ${SCCACHE_EXECUTABLE}")
    endif()
endif()

# clangd (and therefore Serena's C++ symbol tools) reads compile_commands.json
# from the repo root. CMake writes it into the build dir, so link it back. An
# in-source build (flatpak-builder's cmake buildsystem) has no separate build
# dir, so there is nothing to link.
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(_compile_db "${CMAKE_SOURCE_DIR}/compile_commands.json")
if(NOT CMAKE_SOURCE_DIR STREQUAL CMAKE_BINARY_DIR)
    if(IS_SYMLINK "${_compile_db}")
        file(REMOVE "${_compile_db}")
    endif()
    if(NOT EXISTS "${_compile_db}")
        file(CREATE_LINK "${CMAKE_BINARY_DIR}/compile_commands.json" "${_compile_db}" SYMBOLIC)
    endif()
endif()

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE RelWithDebInfo CACHE STRING "Build type" FORCE)
    message(STATUS "No CMAKE_BUILD_TYPE set; defaulting to RelWithDebInfo")
endif()

# Prefer mold on ELF platforms, then lld, then the compiler's default linker.
# check_linker_flag proves the compiler accepts the flag, so an installed mold
# with a GCC older than 12.1 falls through instead of failing at link time. An
# explicit -fuse-ld= in the user's linker flags wins. macOS and Windows keep
# their native linker.
include(CheckLinkerFlag)
if(UNIX AND NOT APPLE AND NOT MSVC
   AND NOT CMAKE_LINKER_TYPE
   AND NOT CMAKE_EXE_LINKER_FLAGS MATCHES "-fuse-ld="
   AND NOT CMAKE_SHARED_LINKER_FLAGS MATCHES "-fuse-ld=")
    find_program(MOLD_EXECUTABLE mold)
    if(MOLD_EXECUTABLE)
        check_linker_flag(CXX "-fuse-ld=mold" PICTURA_LINKER_ACCEPTS_MOLD)
    endif()
    if(PICTURA_LINKER_ACCEPTS_MOLD)
        set(_pictura_linker_flag "-fuse-ld=mold")
    else()
        check_linker_flag(CXX "-fuse-ld=lld" PICTURA_LINKER_ACCEPTS_LLD)
        if(PICTURA_LINKER_ACCEPTS_LLD)
            set(_pictura_linker_flag "-fuse-ld=lld")
        endif()
    endif()
    if(_pictura_linker_flag)
        add_link_options("${_pictura_linker_flag}")
        message(STATUS "C++ linker flag: ${_pictura_linker_flag}")
    endif()
elseif(UNIX AND NOT APPLE AND NOT MSVC AND NOT CMAKE_LINKER_TYPE)
    # The cached -fuse-ld= wins, but a build tree configured before the mold
    # preference keeps an old -fuse-ld=lld without saying so. Say so once per
    # build tree and flag value, so a deliberate choice is not nagged. The
    # first non-mold -fuse-ld= in either the EXE or the SHARED flags counts.
    string(REGEX MATCHALL "-fuse-ld=[^ ]+" _pictura_cached_lds
        "${CMAKE_EXE_LINKER_FLAGS} ${CMAKE_SHARED_LINKER_FLAGS}")
    set(_pictura_cached_ld "")
    foreach(_ld IN LISTS _pictura_cached_lds)
        if(NOT _ld STREQUAL "-fuse-ld=mold")
            set(_pictura_cached_ld "${_ld}")
            break()
        endif()
    endforeach()
    if(_pictura_cached_ld
       AND NOT _pictura_cached_ld STREQUAL "$CACHE{PICTURA_LINKER_MASK_WARNED}")
        find_program(MOLD_EXECUTABLE mold)
        if(MOLD_EXECUTABLE)
            check_linker_flag(CXX "-fuse-ld=mold" PICTURA_LINKER_ACCEPTS_MOLD)
        endif()
        if(PICTURA_LINKER_ACCEPTS_MOLD)
            message(WARNING
                "The cached linker flags carry ${_pictura_cached_ld}, so mold is "
                "not used although it is installed. To let the build pick mold, "
                "reconfigure with: cmake -U CMAKE_EXE_LINKER_FLAGS "
                "-U CMAKE_SHARED_LINKER_FLAGS ${CMAKE_BINARY_DIR}")
            set(PICTURA_LINKER_MASK_WARNED "${_pictura_cached_ld}" CACHE INTERNAL
                "The cached -fuse-ld= flag the mold warning was printed for")
        endif()
    endif()
endif()

# Bound concurrent link steps: a highly parallel compile can otherwise start
# several large links at once and thrash memory. These must be set before the
# first target is created.
if(CMAKE_GENERATOR MATCHES "Ninja")
    set(CMAKE_JOB_POOLS link=4)
    set(CMAKE_JOB_POOL_LINK link)
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

# MSVC's <cmath> only exposes M_PI (used by the crop/overlay geometry) when
# _USE_MATH_DEFINES is defined before the include. A command-line define covers
# every translation unit regardless of include order; other compilers ignore it.
add_compile_definitions(_USE_MATH_DEFINES)

# Qt6::GuiPrivate supplies the RHI headers used by interop.cpp. The private
# dependency is deliberate, so mute Qt's configure-time warning at the boundary
# instead of filtering build logs.
set(QT_NO_PRIVATE_MODULE_WARNING ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Svg Network PrintSupport GuiPrivate)

# Enables BUILD_TESTING (default ON); the Qt Test suites live under cpp/tests.
include(CTest)

find_package(CxxQt QUIET)
if(NOT CxxQt_FOUND)
    include(FetchContent)
    FetchContent_Declare(
        CxxQt
        GIT_REPOSITORY https://github.com/kdab/cxx-qt-cmake.git
        GIT_TAG 0.10.0
    )
    FetchContent_MakeAvailable(CxxQt)
endif()

# cxx-qt reaches Qt through qmake. Debian/Ubuntu ship qmake6 for Qt 6; the bare
# `qmake` is Qt 5, so look for qmake6 first. The value the build uses is the
# same in every cargo invocation: no script and no build system exports a
# QMAKE, so the cxx-qt build script finds qmake on the PATH itself. The
# lookup here is the early error for a machine without one.
find_program(QMAKE_EXECUTABLE NAMES qmake6 qmake)
if(NOT QMAKE_EXECUTABLE)
    message(FATAL_ERROR "qmake not found — install qt6-base-dev")
endif()

# An optimized development build compiles the Rust library with the incremental
# `dev-app` profile (see Cargo.toml). Packaging (BUILD_TESTING off), Debug, and
# multi-config generators keep Corrosion's default release/dev mapping.
set(PICTURA_DEV_OPTIMIZED OFF)
if(BUILD_TESTING AND NOT CMAKE_CONFIGURATION_TYPES
   AND CMAKE_BUILD_TYPE MATCHES "^(Release|RelWithDebInfo)$")
    set(PICTURA_DEV_OPTIMIZED ON)
endif()
set(_pictura_cargo_profile "")
if(PICTURA_DEV_OPTIMIZED)
    set(_pictura_cargo_profile PROFILE dev-app)
endif()

# The optimized development build compiles C++ at -Og -g1 behind a precompiled
# header: measured cold and warm, both pay (openspec change
# qt-fixture-and-compile-cost, design section 7). Packaging and Debug configures
# keep their settings. Split DWARF was measured slower, and GCC rejects a
# precompiled header for a unit built with it.
set(PICTURA_DEV_COMPILE_OPTIONS "")
if(PICTURA_DEV_OPTIMIZED)
    set(PICTURA_DEV_COMPILE_OPTIONS "$<$<CXX_COMPILER_ID:GNU,Clang>:-Og;-g1>")
endif()
# The precompiled header was measured with GCC only; it follows the flags'
# compiler scope, so AppleClang and MSVC builds keep compiling without one.
set(PICTURA_DEV_PCH OFF)
if(PICTURA_DEV_OPTIMIZED AND CMAKE_CXX_COMPILER_ID MATCHES "^(GNU|Clang)$")
    set(PICTURA_DEV_PCH ON)
endif()
set(PICTURA_PCH_HEADER "${CMAKE_CURRENT_SOURCE_DIR}/crates/pictura-app/cpp/pictura_pch.h")

cxx_qt_import_crate(
    MANIFEST_PATH crates/pictura-app/Cargo.toml
    ${_pictura_cargo_profile}
    CRATES pictura_app
    QT_MODULES Qt::Core Qt::Gui Qt::Widgets
)

# A generator clean (`ninja -t clean`) deletes the exported initializer object,
# whose real producer is the crate's build script, and cargo still calls that
# script fresh, so the next link fails. The build script watches a stamp in the
# export directory; before each cargo step, the check touches the stamp when
# the object is missing, so cargo reruns the script and writes it again.
get_target_property(_pictura_cxx_qt_export_root pictura_app CXX_QT_EXPORT_DIR)
set(_pictura_cxxqt_regenerate "${_pictura_cxx_qt_export_root}/pictura_app.regenerate")
if(NOT EXISTS "${_pictura_cxxqt_regenerate}")
    file(TOUCH "${_pictura_cxxqt_regenerate}")
endif()
add_custom_target(pictura_cxxqt_export_check
    COMMAND ${CMAKE_COMMAND}
        "-DOBJECT=${_pictura_cxx_qt_export_root}/crates/pictura_app/initializers.o"
        "-DSTAMP=${_pictura_cxxqt_regenerate}"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/cmake/cxxqt_export_check.cmake"
    VERBATIM)
add_dependencies(cargo-prebuild_pictura_app pictura_cxxqt_export_check)

# The bridge stamp.
#
# The bridge Rust sources define the generated C++ interface. A change to one
# of them has to rebuild the C++ that includes the generated headers in the
# same build, but Ninja decides what to compile before the cargo edge has
# regenerated those headers, so a header rewritten during a build is only seen
# by the next one. This stamp records the bridge content at configure time --
# before the build plans -- and the translation units that include a generated
# header depend on it.
set(PICTURA_BRIDGE_STAMP "${CMAKE_BINARY_DIR}/pictura_bridge.stamp")

# The text of a bridge file that the generated C++ depends on: the
# `#[cxx_qt::bridge]` module, with the outer attributes and doc comments
# directly above it (cxx-gen attaches those to the module too). Everything else
# in the file is implementation, so an edit there leaves the stamp alone. The
# module ends at the first column-0 `}`, which rustfmt guarantees. A string or
# block comment inside the module could hold such a line and end the slice
# early, so the slice must have balanced braces, an even number of quotes, and
# no `/*`; when it does not, or the attribute or the closing brace is missing,
# the whole file is hashed, which rebuilds too much rather than too little. The
# pictura_app test `bridge_modules_end_at_column_zero` pins the same shape from
# the Rust side.
function(pictura_bridge_hash_text path out_var)
    file(READ "${path}" _src)
    set(${out_var} "${_src}" PARENT_SCOPE)
    string(FIND "${_src}" "#[cxx_qt::bridge" _attr)
    if(_attr EQUAL -1)
        return()
    endif()

    set(_start ${_attr})
    while(_start GREATER 0)
        math(EXPR _newline "${_start} - 1")
        string(SUBSTRING "${_src}" ${_newline} 1 _char)
        if(NOT _char STREQUAL "\n")
            break()
        endif()
        string(SUBSTRING "${_src}" 0 ${_newline} _head)
        string(FIND "${_head}" "\n" _previous REVERSE)
        math(EXPR _line_start "${_previous} + 1")
        math(EXPR _line_length "${_newline} - ${_line_start}")
        string(SUBSTRING "${_src}" ${_line_start} ${_line_length} _line)
        if(NOT _line MATCHES "^(#\\[|///)")
            break()
        endif()
        set(_start ${_line_start})
    endwhile()

    string(SUBSTRING "${_src}" ${_attr} -1 _tail)
    string(FIND "${_tail}" "\n}\n" _close)
    if(_close EQUAL -1)
        return()
    endif()
    math(EXPR _length "${_attr} + ${_close} + 2 - ${_start}")
    string(SUBSTRING "${_src}" ${_start} ${_length} _module)

    string(REGEX MATCHALL "{" _opens "${_module}")
    string(REGEX MATCHALL "}" _closes "${_module}")
    list(LENGTH _opens _open_count)
    list(LENGTH _closes _close_count)
    string(REPLACE "\\\\" "" _unescaped "${_module}")
    string(REPLACE "\\\"" "" _unescaped "${_unescaped}")
    string(REGEX MATCHALL "\"" _quotes "${_unescaped}")
    list(LENGTH _quotes _quote_count)
    math(EXPR _odd_quotes "${_quote_count} % 2")
    string(FIND "${_module}" "/*" _block_comment)
    if(_open_count EQUAL _close_count AND _odd_quotes EQUAL 0
       AND _block_comment EQUAL -1)
        set(${out_var} "${_module}" PARENT_SCOPE)
    endif()
endfunction()

# The bridge sources are the Rust files carrying the cxx-qt bridge macro, plus
# every file the build script's `.files([...])` list names. The scan finds a
# bridge wherever it is; the list catches a listed file before it has one, so
# the bridge it gains later is a configure dependency from the start. A listed
# file without a bridge is hashed whole. CONFIGURE_DEPENDS re-runs the configure
# step when a Rust file is added or removed.
file(GLOB_RECURSE _pictura_rs_files RELATIVE "${CMAKE_SOURCE_DIR}" CONFIGURE_DEPENDS
    "crates/pictura-app/src/*.rs")
set(_pictura_bridge_files "")
foreach(_file IN LISTS _pictura_rs_files)
    file(READ "${CMAKE_SOURCE_DIR}/${_file}" _content)
    string(FIND "${_content}" "cxx_qt::bridge" _bridge_at)
    if(NOT _bridge_at EQUAL -1)
        list(APPEND _pictura_bridge_files "${_file}")
    endif()
endforeach()
file(STRINGS "${CMAKE_SOURCE_DIR}/crates/pictura-app/build.rs" _pictura_listed
    REGEX "^[ \t]*\"src/[^\"]+\\.rs\",?[ \t]*$")
foreach(_line IN LISTS _pictura_listed)
    string(REGEX REPLACE "^[ \t]*\"([^\"]+)\".*$" "crates/pictura-app/\\1" _file "${_line}")
    if(EXISTS "${CMAKE_SOURCE_DIR}/${_file}")
        list(APPEND _pictura_bridge_files "${_file}")
    endif()
endforeach()
list(REMOVE_DUPLICATES _pictura_bridge_files)
list(SORT _pictura_bridge_files)
set(_pictura_bridge_digest "")
foreach(_file IN LISTS _pictura_bridge_files)
    pictura_bridge_hash_text("${CMAKE_SOURCE_DIR}/${_file}" _bridge_text)
    string(SHA256 _file_digest "${_bridge_text}")
    string(APPEND _pictura_bridge_digest "${_file_digest} ${_file}\n")
endforeach()
string(SHA256 _pictura_bridge_hash "${_pictura_bridge_digest}")

# Only a bridge file, or build.rs, re-runs the configure step on an edit, so the
# stamp is refreshed when a bridge changes. A file named in build.rs's list is
# a configure dependency even before it carries a bridge.
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS
    ${_pictura_bridge_files}
    crates/pictura-app/build.rs)

# Rewrite the stamp only when the bridge content changed: a re-configure that
# finds the same bridge must not dirty the whole C++ tree.
if(EXISTS "${PICTURA_BRIDGE_STAMP}")
    file(READ "${PICTURA_BRIDGE_STAMP}" _pictura_stamp_text)
else()
    set(_pictura_stamp_text "")
endif()
if(NOT _pictura_stamp_text STREQUAL _pictura_bridge_hash)
    file(WRITE "${PICTURA_BRIDGE_STAMP}" "${_pictura_bridge_hash}")
endif()

# Make every C++ translation unit depend on the stamp, so a bridge change
# rebuilds its consumers in the same build. The stamp is global, and after the
# membrane migration a translation unit can reach a generated header through a
# header (frame_includes.h, paint_tip.h, ...), so a per-source string scan
# would miss some consumers. Rebuilding too much is the accepted fallback.
function(pictura_depend_on_bridge_stamp)
    foreach(_source IN LISTS ARGN)
        if(_source MATCHES "\\.(c|cc|cpp|cxx)$")
            set_property(SOURCE "${_source}" APPEND PROPERTY
                OBJECT_DEPENDS "${PICTURA_BRIDGE_STAMP}")
        endif()
    endforeach()
endfunction()

# Compile the Qt resource once: the shared shell carries it, and the static
# configuration links this object library into the executable and each test.
# POSITION_INDEPENDENT_CODE: object libraries default to non-PIC, but they are
# linked into PIE executables and the shared shell; lld rejects the qrc's
# references without it.
add_library(pictura_assets OBJECT assets/pictura.qrc)
set_target_properties(pictura_assets PROPERTIES POSITION_INDEPENDENT_CODE ON)

include(cmake/sources/api.cmake)
include(cmake/sources/acr.cmake)
include(cmake/sources/panels.cmake)
include(cmake/sources/frame.cmake)
include(cmake/sources/tools.cmake)
include(cmake/sources/options_bar.cmake)
include(cmake/sources/control_server.cmake)
include(cmake/sources/update.cmake)
include(cmake/sources/documents.cmake)
include(cmake/sources/filters.cmake)
include(cmake/sources/adjustments.cmake)
include(cmake/sources/dialogs.cmake)
include(cmake/sources/image_view.cmake)
include(cmake/sources/shell.cmake)

# A shared shell lets each Qt Test executable link one small library instead of
# two large static archives. Release, packaging, and Windows keep the static
# shell so the shipped binary stays self-contained and no symbol exports are
# needed.
if(BUILD_TESTING AND NOT WIN32)
    set(PICTURA_SHELL_SHARED ON)
    add_library(pictura_shell SHARED ${PICTURA_SHELL_SOURCES})
else()
    set(PICTURA_SHELL_SHARED OFF)
    add_library(pictura_shell STATIC ${PICTURA_SHELL_SOURCES})
endif()

# The translation units that include a generated cxx-qt header follow the
# bridge stamp.
pictura_depend_on_bridge_stamp(${PICTURA_SHELL_SOURCES})

# Target-level, so the per-source -O1 below still comes after them.
target_compile_options(pictura_shell PRIVATE ${PICTURA_DEV_COMPILE_OPTIONS})
if(PICTURA_DEV_PCH)
    target_precompile_headers(pictura_shell PRIVATE "${PICTURA_PCH_HEADER}")
endif()

target_include_directories(pictura_shell PUBLIC
    crates/pictura-app/cpp
)
target_compile_definitions(pictura_shell PUBLIC
    PICTURA_VERSION="${PROJECT_VERSION}"
    PICTURA_BUILD_TYPE="$<CONFIG>"
)
# In the shared configuration pictura_app and the resource object are private to
# the library, so the generated cxx-qt headers have to be republished for the
# test executables.
if(PICTURA_SHELL_SHARED)
    get_target_property(_pictura_cxx_qt_export_dir pictura_app CXX_QT_EXPORT_DIR)
    if(_pictura_cxx_qt_export_dir)
        target_include_directories(pictura_shell PUBLIC
            "${_pictura_cxx_qt_export_dir}/crates/pictura_app/include")
    endif()
endif()

# The Rust static lib embeds `pictura-color` (Little CMS 2). On Linux the
# `lcms2-sys` crate links the system library dynamically, so the C++ link needs
# `-llcms2`. On Windows pkg-config finds no system lcms2 and `lcms2-sys`'s static
# fallback compiles the vendored copy into `pictura_app`; macOS does the same by
# enabling lcms2-sys's `static` feature (pictura-color), so neither platform
# links anything external.
if(PICTURA_SHELL_SHARED)
    target_link_libraries(pictura_shell PUBLIC Qt6::Widgets Qt6::Svg Qt6::Network Qt6::PrintSupport)
    target_link_libraries(pictura_shell PRIVATE pictura_app pictura_assets)
else()
    target_link_libraries(pictura_shell PUBLIC pictura_app Qt6::Svg Qt6::Network Qt6::PrintSupport)
endif()
# RHI/private headers: interop.cpp includes <rhi/qrhi.h>. The exported private
# target carries the platform-correct include dir (Debian, macOS framework,
# Windows), so no hand-built qmake-derived path is needed.
target_link_libraries(pictura_shell PRIVATE Qt6::GuiPrivate)
if(UNIX AND NOT APPLE)
    find_library(LCMS2_LIBRARY NAMES lcms2 REQUIRED)
    if(PICTURA_SHELL_SHARED)
        target_link_libraries(pictura_shell PRIVATE "${LCMS2_LIBRARY}")
    else()
        target_link_libraries(pictura_shell PUBLIC "${LCMS2_LIBRARY}")
    endif()
endif()

# The shell is a library so the Qt Test suites under cpp/tests can link the
# UI without a second main. Only main.cpp and the self-test sources stay here.
include(cmake/sources/selftest.cmake)

add_executable(pictura ${PICTURA_SELF_TEST_SOURCES})
# No precompiled header here: the self-test sources do not see Qt's keyword
# macros (selftest_report.h declares an `emit` function).
target_compile_options(pictura PRIVATE ${PICTURA_DEV_COMPILE_OPTIONS})
# Menu construction is not on a hot path but compiles slowly; a lighter
# optimization level keeps it off the development critical path.
if(PICTURA_DEV_OPTIMIZED)
    # Its -O1 does not match the precompiled header, which GCC would then read
    # as plain text.
    set_source_files_properties(
            crates/pictura-app/cpp/filter_commands.cpp
        PROPERTIES COMPILE_OPTIONS "$<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-O1>"
                   SKIP_PRECOMPILE_HEADERS ON)
endif()

target_link_libraries(pictura PRIVATE pictura_shell)
if(NOT PICTURA_SHELL_SHARED)
    target_link_libraries(pictura PRIVATE pictura_assets)
endif()

# The self-test sources include the generated cxx-qt header too.
pictura_depend_on_bridge_stamp(${PICTURA_SELF_TEST_SOURCES})

# Windows-only presentation: a GUI-subsystem executable (no console window) with
# an embedded icon and manifest. A plain `add_executable` never runs Qt's
# target-finalization hook, so `qt6_finalize_target` is what makes
# QT_WINDOWS_APP_PROJECT_IDENTIFIER — and Qt's generated manifest, which
# declares Windows 10/11 compatibility — actually apply.
if(WIN32)
    enable_language(RC)
    configure_file(packaging/windows/pictura.rc.in
        "${CMAKE_CURRENT_BINARY_DIR}/pictura.rc" @ONLY)
    target_sources(pictura PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/pictura.rc")
    set_target_properties(pictura PROPERTIES
        WIN32_EXECUTABLE ON
        QT_WINDOWS_APP_PROJECT_IDENTIFIER "io.github.KookaPictura.KookaPictura")
    qt6_finalize_target(pictura)
endif()

# macOS presentation: an application bundle, the form macdeployqt and create-dmg
# operate on. The Info.plist template substitutes the project version, so the
# bundle tracks the x-release-please-version marker.
if(APPLE)
    set_target_properties(pictura PROPERTIES
        MACOSX_BUNDLE TRUE
        MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_SOURCE_DIR}/packaging/macos/Info.plist.in"
        MACOSX_BUNDLE_BUNDLE_NAME "Kooka Pictura"
        MACOSX_BUNDLE_GUI_IDENTIFIER "io.github.KookaPictura.KookaPictura"
        MACOSX_BUNDLE_ICON_FILE app.icns
        MACOSX_BUNDLE_SHORT_VERSION_STRING "${PROJECT_VERSION}"
        MACOSX_BUNDLE_BUNDLE_VERSION "${PROJECT_VERSION}")
    set_source_files_properties(assets/icons/app.icns PROPERTIES
        MACOSX_PACKAGE_LOCATION Resources)
    target_sources(pictura PRIVATE assets/icons/app.icns)
endif()

if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
    add_subdirectory(crates/pictura-app/cpp/tests)
endif()

# The AppImage job (.github/workflows/appimage.yml) stages this tree into an
# AppDir; the macOS job (.github/workflows/macos.yml) stages the .app bundle.
# The desktop file and icon share the binary's name so Wayland's default app_id
# ("pictura") resolves to them.
include(GNUInstallDirs)
install(TARGETS pictura
    BUNDLE DESTINATION ${CMAKE_INSTALL_BINDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
if(NOT WIN32)
    install(FILES packaging/pictura.desktop DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
    install(FILES assets/icons/app.png
        DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/512x512/apps
        RENAME pictura.png)
    install(FILES packaging/pictura.metainfo.xml
        DESTINATION ${CMAKE_INSTALL_DATADIR}/metainfo)
endif()
