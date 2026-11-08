# options_bar sources for pictura_shell.
# Included from the root CMakeLists.txt; keep paths explicit (no globbing).
list(APPEND PICTURA_SHELL_SOURCES
    crates/pictura-app/cpp/options_bar.cpp
    crates/pictura-app/cpp/options_bar.h
    crates/pictura-app/cpp/options_bar_crop.cpp
    crates/pictura-app/cpp/options_bar_erase.cpp
    crates/pictura-app/cpp/options_bar_eyedropper.cpp
    crates/pictura-app/cpp/options_bar_fill.cpp
    crates/pictura-app/cpp/options_bar_move.cpp
    crates/pictura-app/cpp/options_bar_navigate.cpp
    crates/pictura-app/cpp/options_bar_paint.cpp
    crates/pictura-app/cpp/options_bar_pen.cpp
    crates/pictura-app/cpp/options_bar_retouch.cpp
    crates/pictura-app/cpp/options_bar_rotate.cpp
    crates/pictura-app/cpp/options_bar_shape.cpp
    crates/pictura-app/cpp/options_bar_stamp.cpp
    crates/pictura-app/cpp/options_bar_transform.cpp
    crates/pictura-app/cpp/options_bar_type.cpp
)
