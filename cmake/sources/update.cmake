# update sources for pictura_shell.
# Included from the root CMakeLists.txt; keep paths explicit (no globbing).
list(APPEND PICTURA_SHELL_SOURCES
    crates/pictura-app/cpp/update_banner.cpp
    crates/pictura-app/cpp/update_banner.h
    crates/pictura-app/cpp/update_checker.cpp
    crates/pictura-app/cpp/update_checker.h
    crates/pictura-app/cpp/update_installer.cpp
    crates/pictura-app/cpp/update_installer.h
    crates/pictura-app/cpp/update_package.cpp
    crates/pictura-app/cpp/update_package.h
    crates/pictura-app/cpp/update_preferences_page.cpp
    crates/pictura-app/cpp/update_preferences_page.h
    crates/pictura-app/cpp/update_release.cpp
    crates/pictura-app/cpp/update_release.h
    crates/pictura-app/cpp/update_version.cpp
    crates/pictura-app/cpp/update_version.h
)
