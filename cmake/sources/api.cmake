# api sources for pictura_shell.
# Included from the root CMakeLists.txt; keep paths explicit (no globbing).
list(APPEND PICTURA_SHELL_SOURCES
    crates/pictura-app/cpp/api/document_api.cpp
    crates/pictura-app/cpp/api/document_api.h
    crates/pictura-app/cpp/api/document_bridge.h
    crates/pictura-app/cpp/api/engine_bridge.h
    crates/pictura-app/cpp/api/filters_bridge.h
    crates/pictura-app/cpp/api/history_api.cpp
    crates/pictura-app/cpp/api/history_api.h
    crates/pictura-app/cpp/api/layers_api.cpp
    crates/pictura-app/cpp/api/layers_api.h
    crates/pictura-app/cpp/api/layers_bridge.h
    crates/pictura-app/cpp/api/paint_api.cpp
    crates/pictura-app/cpp/api/paint_api.h
    crates/pictura-app/cpp/api/paint_bridge.h
    crates/pictura-app/cpp/api/selection_api.cpp
    crates/pictura-app/cpp/api/selection_api.h
    crates/pictura-app/cpp/api/selection_bridge.h
    crates/pictura-app/cpp/api/transform_api.cpp
    crates/pictura-app/cpp/api/transform_api.h
    crates/pictura-app/cpp/api/transform_bridge.h
    crates/pictura-app/cpp/api/type_api.cpp
    crates/pictura-app/cpp/api/type_api.h
    crates/pictura-app/cpp/api/view_bridge.h
)
