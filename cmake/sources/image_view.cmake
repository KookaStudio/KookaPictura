# image_view sources for pictura_shell.
# Included from the root CMakeLists.txt; keep paths explicit (no globbing).
list(APPEND PICTURA_SHELL_SOURCES
    crates/pictura-app/cpp/canvas_range.h
    crates/pictura-app/cpp/canvas_ruler.cpp
    crates/pictura-app/cpp/canvas_ruler.h
    crates/pictura-app/cpp/canvas_scrollbars.cpp
    crates/pictura-app/cpp/canvas_scrollbars.h
    crates/pictura-app/cpp/image_view.cpp
    crates/pictura-app/cpp/image_view.h
    crates/pictura-app/cpp/image_view_annotations.cpp
    crates/pictura-app/cpp/image_view_channels.cpp
    crates/pictura-app/cpp/image_view_document.cpp
    crates/pictura-app/cpp/image_view_guides.cpp
    crates/pictura-app/cpp/image_view_move.cpp
    crates/pictura-app/cpp/image_view_overlays.cpp
    crates/pictura-app/cpp/image_view_paths.cpp
    crates/pictura-app/cpp/image_view_present.cpp
    crates/pictura-app/cpp/image_view_smart_guides.cpp
    crates/pictura-app/cpp/image_view_transform_warp.cpp
    crates/pictura-app/cpp/image_view_type.cpp
    crates/pictura-app/cpp/screen_mode_message.cpp
    crates/pictura-app/cpp/screen_mode_message.h
    crates/pictura-app/cpp/vp_canvas.cpp
    crates/pictura-app/cpp/vp_canvas.h
    crates/pictura-app/cpp/vp_options.cpp
    crates/pictura-app/cpp/vp_options.h
)
