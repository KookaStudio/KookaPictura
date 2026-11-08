# acr sources for pictura_shell.
# Included from the root CMakeLists.txt; keep paths explicit (no globbing).
list(APPEND PICTURA_SHELL_SOURCES
    crates/pictura-app/cpp/acr/acr_basic_panel.cpp
    crates/pictura-app/cpp/acr/acr_calibration_panel.cpp
    crates/pictura-app/cpp/acr/acr_controls.cpp
    crates/pictura-app/cpp/acr/acr_controls.h
    crates/pictura-app/cpp/acr/acr_detail_panel.cpp
    crates/pictura-app/cpp/acr/acr_effects_panel.cpp
    crates/pictura-app/cpp/acr/acr_histogram.cpp
    crates/pictura-app/cpp/acr/acr_histogram.h
    crates/pictura-app/cpp/acr/acr_hsl_panel.cpp
    crates/pictura-app/cpp/acr/acr_lens_panel.cpp
    crates/pictura-app/cpp/acr/acr_panels.h
    crates/pictura-app/cpp/acr/acr_presets_panel.cpp
    crates/pictura-app/cpp/acr/acr_preview_pane.cpp
    crates/pictura-app/cpp/acr/acr_preview_pane.h
    crates/pictura-app/cpp/acr/acr_settings.cpp
    crates/pictura-app/cpp/acr/acr_settings.h
    crates/pictura-app/cpp/acr/acr_split_toning_panel.cpp
    crates/pictura-app/cpp/acr/acr_tone_curve_panel.cpp
)
