# Kooka Pictura — M0 walking skeleton (M0-A).
#
# Rust owns the QObject and the image; CMake builds the Rust crate as a static
# library through Corrosion (via CXX-Qt-CMake) and links it into the C++ shell.

cmake_minimum_required(VERSION 3.24)

project(kooka_pictura VERSION 0.2.0 LANGUAGES CXX) # x-release-please-version

# clangd (and therefore Serena's C++ symbol tools) reads compile_commands.json
# from the repo root. CMake writes it into the build dir, so link it back.
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(_compile_db "${CMAKE_SOURCE_DIR}/compile_commands.json")
if(IS_SYMLINK "${_compile_db}")
    file(REMOVE "${_compile_db}")
endif()
if(NOT EXISTS "${_compile_db}")
    file(CREATE_LINK "${CMAKE_BINARY_DIR}/compile_commands.json" "${_compile_db}" SYMBOLIC)
endif()

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE RelWithDebInfo CACHE STRING "Build type" FORCE)
    message(STATUS "No CMAKE_BUILD_TYPE set; defaulting to RelWithDebInfo")
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Svg Network PrintSupport)

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
# `qmake` is Qt 5, so pin the executable explicitly.
find_program(QMAKE_EXECUTABLE NAMES qmake6 qmake)
if(NOT QMAKE_EXECUTABLE)
    message(FATAL_ERROR "qmake not found — install qt6-base-dev")
endif()

# The RHI headers live under the versioned QtGui include dir, which the plain
# Qt6::Gui target does not export on Debian (no Qt6::GuiPrivate package).
execute_process(COMMAND "${QMAKE_EXECUTABLE}" -query QT_INSTALL_HEADERS
    OUTPUT_VARIABLE QT_HEADERS_DIR OUTPUT_STRIP_TRAILING_WHITESPACE)
execute_process(COMMAND "${QMAKE_EXECUTABLE}" -query QT_VERSION
    OUTPUT_VARIABLE QT_VERSION_STR OUTPUT_STRIP_TRAILING_WHITESPACE)

cxx_qt_import_crate(
    MANIFEST_PATH crates/pictura-app/Cargo.toml
    CRATES pictura_app
    QT_MODULES Qt::Core Qt::Gui Qt::Widgets
    QMAKE "${QMAKE_EXECUTABLE}"
)

add_library(pictura_shell STATIC
    crates/pictura-app/cpp/decode_image.cpp
    crates/pictura-app/cpp/decode_image.h
    crates/pictura-app/cpp/encode_image.cpp
    crates/pictura-app/cpp/encode_image.h
    crates/pictura-app/cpp/render_text.cpp
    crates/pictura-app/cpp/render_text.h
    crates/pictura-app/cpp/interop.cpp
    crates/pictura-app/cpp/commands.cpp
    crates/pictura-app/cpp/control_server.cpp
    crates/pictura-app/cpp/control_server_actions.cpp
    crates/pictura-app/cpp/control_server_input.cpp
    crates/pictura-app/cpp/command_tree.cpp
    crates/pictura-app/cpp/theme.cpp
    crates/pictura-app/cpp/fonts.cpp
    crates/pictura-app/cpp/recovery_store.cpp
    crates/pictura-app/cpp/session.cpp
    crates/pictura-app/cpp/workspace_store.cpp
    crates/pictura-app/cpp/preferences_dialog.cpp
    crates/pictura-app/cpp/file_info_dialog.cpp
    crates/pictura-app/cpp/profile_dialog.cpp
    crates/pictura-app/cpp/hdr_conversion_dialog.cpp
    crates/pictura-app/cpp/pictura_raw_dialog.cpp
    crates/pictura-app/cpp/warp_preset_dialog.cpp
    crates/pictura-app/cpp/filter_commands.cpp
    crates/pictura-app/cpp/filter_commands.h
    crates/pictura-app/cpp/filter_gallery_dialog.cpp
    crates/pictura-app/cpp/filter_gallery_dialog.h
    crates/pictura-app/cpp/filter_param_controls.cpp
    crates/pictura-app/cpp/filter_param_controls.h
    crates/pictura-app/cpp/filter_preview_dialog.cpp
    crates/pictura-app/cpp/filter_preview_dialog.h
    crates/pictura-app/cpp/lens_flare_dialog.cpp
    crates/pictura-app/cpp/lens_flare_dialog.h
    crates/pictura-app/cpp/lighting_canvas.cpp
    crates/pictura-app/cpp/lighting_canvas.h
    crates/pictura-app/cpp/lighting_effects_dialog.cpp
    crates/pictura-app/cpp/lighting_effects_dialog.h
    crates/pictura-app/cpp/lighting_rig.cpp
    crates/pictura-app/cpp/lighting_rig.h
    crates/pictura-app/cpp/blend_modes.h
    crates/pictura-app/cpp/layer_style_dialog.cpp
    crates/pictura-app/cpp/layer_style_dialog.h
    crates/pictura-app/cpp/layer_style_pages.cpp
    crates/pictura-app/cpp/color_settings_dialog.cpp
    crates/pictura-app/cpp/color_picker_dialog.cpp
    crates/pictura-app/cpp/color_range_dialog.cpp
    crates/pictura-app/cpp/replace_color_dialog.cpp
    crates/pictura-app/cpp/adjustment_dialog.cpp
    crates/pictura-app/cpp/black_white_dialog.cpp
    crates/pictura-app/cpp/channel_mixer_dialog.cpp
    crates/pictura-app/cpp/color_balance_dialog.cpp
    crates/pictura-app/cpp/curves_dialog.cpp
    crates/pictura-app/cpp/canvas_size_dialog.cpp
    crates/pictura-app/cpp/exposure_dialog.cpp
    crates/pictura-app/cpp/gradient_editor_dialog.cpp
    crates/pictura-app/cpp/gradient_map_dialog.cpp
    crates/pictura-app/cpp/gradient_noise_page.cpp
    crates/pictura-app/cpp/hue_saturation_dialog.cpp
    crates/pictura-app/cpp/image_size_dialog.cpp
    crates/pictura-app/cpp/levels_dialog.cpp
    crates/pictura-app/cpp/photo_filter_dialog.cpp
    crates/pictura-app/cpp/slider_dialog.cpp
    crates/pictura-app/cpp/hdr_toning_dialog.cpp
    crates/pictura-app/cpp/print_dialog.cpp
    crates/pictura-app/cpp/export_as_dialog.cpp
    crates/pictura-app/cpp/duplicate_image_dialog.cpp
    crates/pictura-app/cpp/bitmap_mode_dialog.cpp
    crates/pictura-app/cpp/indexed_color_dialog.cpp
    crates/pictura-app/cpp/fill_dialog.cpp
    crates/pictura-app/cpp/defringe_dialog.cpp
    crates/pictura-app/cpp/stroke_dialog.cpp
    crates/pictura-app/cpp/image_view.cpp
    crates/pictura-app/cpp/image_view_overlays.cpp
    crates/pictura-app/cpp/image_view_annotations.cpp
    crates/pictura-app/cpp/image_view_channels.cpp
    crates/pictura-app/cpp/image_view_document.cpp
    crates/pictura-app/cpp/image_view_paths.cpp
    crates/pictura-app/cpp/image_view_type.cpp
    crates/pictura-app/cpp/canvas_scrollbars.cpp
    crates/pictura-app/cpp/icons.cpp
    crates/pictura-app/cpp/svg_icon_engine.cpp
    crates/pictura-app/cpp/file_drop_router.cpp
    crates/pictura-app/cpp/frame.cpp
    crates/pictura-app/cpp/frame_columns.cpp
    crates/pictura-app/cpp/frame_session.cpp
    crates/pictura-app/cpp/frame_menus.cpp
    crates/pictura-app/cpp/frame_menus_select.cpp
    crates/pictura-app/cpp/frame_menus_edit.cpp
    crates/pictura-app/cpp/frame_menus_filter.cpp
    crates/pictura-app/cpp/frame_menus_align.cpp
    crates/pictura-app/cpp/frame_menus_layer_style.cpp
    crates/pictura-app/cpp/frame_menus_adjust.cpp
    crates/pictura-app/cpp/frame_menus_layer_ops.cpp
    crates/pictura-app/cpp/frame_menus_matting.cpp
    crates/pictura-app/cpp/frame_menus_smart_object.cpp
    crates/pictura-app/cpp/frame_menus_image.cpp
    crates/pictura-app/cpp/frame_menus_image_mode.cpp
    crates/pictura-app/cpp/frame_recent.cpp
    crates/pictura-app/cpp/frame_recovery.cpp
    crates/pictura-app/cpp/frame_test.cpp
    crates/pictura-app/cpp/frame_build.cpp
    crates/pictura-app/cpp/frame_workspaces.cpp
    crates/pictura-app/cpp/frame_workspace_controller.cpp
    crates/pictura-app/cpp/frame_canvas.cpp
    crates/pictura-app/cpp/frame_canvas.h
    crates/pictura-app/cpp/panels/layers_panel.cpp
    crates/pictura-app/cpp/panels/paths_panel.cpp
    crates/pictura-app/cpp/panels/layers_filter_proxy.cpp
    crates/pictura-app/cpp/panels/layers_filter_bar.cpp
    crates/pictura-app/cpp/panels/layers_panel_actions.cpp
    crates/pictura-app/cpp/panels/layers_panel_menu.cpp
    crates/pictura-app/cpp/panels/layers_panel_test.cpp
    crates/pictura-app/cpp/panels/percent_field.cpp
    crates/pictura-app/cpp/panels/numeric_field.cpp
    crates/pictura-app/cpp/panels/angle_dial.cpp
    crates/pictura-app/cpp/panels/ramp_slider.cpp
    crates/pictura-app/cpp/panels/spectrum_bar.cpp
    crates/pictura-app/cpp/panels/curve_widget.cpp
    crates/pictura-app/cpp/panels/history_panel.cpp
    crates/pictura-app/cpp/panels/brush_panel.cpp
    crates/pictura-app/cpp/panels/brush_preset_picker.cpp
    crates/pictura-app/cpp/panels/clone_source_panel.cpp
    crates/pictura-app/cpp/panels/adjustment_controls.cpp
    crates/pictura-app/cpp/panels/character_panel.cpp
    crates/pictura-app/cpp/panels/paragraph_panel.cpp
    crates/pictura-app/cpp/panels/paragraph_styles_panel.cpp
    crates/pictura-app/cpp/paragraph_style_dialog.cpp
    crates/pictura-app/cpp/paragraph_style_dialog_pages.cpp
    crates/pictura-app/cpp/font_combo.cpp
    crates/pictura-app/cpp/panels/glyphs_panel.cpp
    crates/pictura-app/cpp/panels/navigator_panel.cpp
    crates/pictura-app/cpp/panels/color_panel.cpp
    crates/pictura-app/cpp/panels/swatches_panel.cpp
    crates/pictura-app/cpp/panels/info_panel.cpp
    crates/pictura-app/cpp/panels/notes_panel.cpp
    crates/pictura-app/cpp/panels/histogram_panel.cpp
    crates/pictura-app/cpp/panels/placeholder_panel.cpp
    crates/pictura-app/cpp/panels/properties_panel.cpp
    crates/pictura-app/cpp/panels/channels_panel.cpp
    crates/pictura-app/cpp/panels/character_panel.cpp
    crates/pictura-app/cpp/panels/paragraph_panel.cpp
    crates/pictura-app/cpp/panels/panel_group.cpp
    crates/pictura-app/cpp/panels/panel_group_menu.cpp
    crates/pictura-app/cpp/panels/panel_group_test.cpp
    crates/pictura-app/cpp/panels/panel_column.cpp
    crates/pictura-app/cpp/panels/panel_float.cpp
    crates/pictura-app/cpp/panels/panel_column_float.cpp
    crates/pictura-app/cpp/panels/panel_column_iconic.cpp
    crates/pictura-app/cpp/panels/panel_column_menu.cpp
    crates/pictura-app/cpp/panels/panel_column_drag.cpp
    crates/pictura-app/cpp/panels/panel_column_indicator.cpp
    crates/pictura-app/cpp/panels/panel_column_test.cpp
    crates/pictura-app/cpp/tools.cpp
    crates/pictura-app/cpp/tool_registry.cpp
    crates/pictura-app/cpp/tool_hand_zoom.cpp
    crates/pictura-app/cpp/tool_eyedropper.cpp
    crates/pictura-app/cpp/tool_brush.cpp
    crates/pictura-app/cpp/tool_colorreplacement.cpp
    crates/pictura-app/cpp/tool_mixerbrush.cpp
    crates/pictura-app/cpp/tool_stamps.cpp
    crates/pictura-app/cpp/tool_eraser.cpp
    crates/pictura-app/cpp/tool_fills.cpp
    crates/pictura-app/cpp/tool_retouch.cpp
    crates/pictura-app/cpp/tool_pen.cpp
    crates/pictura-app/cpp/tool_path_selection.cpp
    crates/pictura-app/cpp/tool_shape.cpp
    crates/pictura-app/cpp/tool_rotate_view.cpp
    crates/pictura-app/cpp/save_for_web_dialog.cpp
    crates/pictura-app/cpp/shape_dialogs.cpp
    crates/pictura-app/cpp/tool_type.cpp
    crates/pictura-app/cpp/type_fonts.cpp
    crates/pictura-app/cpp/type_text_edit.cpp
    crates/pictura-app/cpp/tool_wand.cpp
    crates/pictura-app/cpp/tool_move.cpp
    crates/pictura-app/cpp/tool_crop.cpp
    crates/pictura-app/cpp/tool_perspectivecrop.cpp
    crates/pictura-app/cpp/tool_slice.cpp
    crates/pictura-app/cpp/tool_annotations.cpp
    crates/pictura-app/cpp/tool_count.cpp
    crates/pictura-app/cpp/tool_ruler.cpp
    crates/pictura-app/cpp/tool_healing.cpp
    crates/pictura-app/cpp/tool_region_drag.cpp
    crates/pictura-app/cpp/tool_patch.cpp
    crates/pictura-app/cpp/tool_redeye.cpp
    crates/pictura-app/cpp/tool_contentawaremove.cpp
    crates/pictura-app/cpp/tool_selection.cpp
    crates/pictura-app/cpp/tool_magneticlasso.cpp
    crates/pictura-app/cpp/tool_catalog.cpp
    crates/pictura-app/cpp/tool_transform.cpp
    crates/pictura-app/cpp/selection_geometry.cpp
    crates/pictura-app/cpp/tools_marquee.cpp
    crates/pictura-app/cpp/tools_selection_move.cpp
    crates/pictura-app/cpp/toolbox.cpp
    crates/pictura-app/cpp/options_bar.cpp
    crates/pictura-app/cpp/options_bar_crop.cpp
    crates/pictura-app/cpp/options_bar_paint.cpp
    crates/pictura-app/cpp/options_bar_stamp.cpp
    crates/pictura-app/cpp/options_bar_erase.cpp
    crates/pictura-app/cpp/options_bar_fill.cpp
    crates/pictura-app/cpp/options_bar_retouch.cpp
    crates/pictura-app/cpp/options_bar_pen.cpp
    crates/pictura-app/cpp/options_bar_shape.cpp
    crates/pictura-app/cpp/options_bar_move.cpp
    crates/pictura-app/cpp/options_bar_eyedropper.cpp
    crates/pictura-app/cpp/options_bar_rotate.cpp
    crates/pictura-app/cpp/options_bar_type.cpp
    crates/pictura-app/cpp/tool_hint_bar.cpp
    crates/pictura-app/cpp/new_document_dialog.cpp
    crates/pictura-app/cpp/layer_new_dialog.cpp
    crates/pictura-app/cpp/dialogs.cpp
    crates/pictura-app/cpp/commands.h
    crates/pictura-app/cpp/control_server.h
    crates/pictura-app/cpp/theme.h
    crates/pictura-app/cpp/fonts.h
    crates/pictura-app/cpp/session.h
    crates/pictura-app/cpp/workspace_store.h
    crates/pictura-app/cpp/preferences_dialog.h
    crates/pictura-app/cpp/file_info_dialog.h
    crates/pictura-app/cpp/profile_dialog.h
    crates/pictura-app/cpp/hdr_conversion_dialog.h
    crates/pictura-app/cpp/hdr_toning_dialog.h
    crates/pictura-app/cpp/black_white_dialog.h
    crates/pictura-app/cpp/channel_mixer_dialog.h
    crates/pictura-app/cpp/color_balance_dialog.h
    crates/pictura-app/cpp/curves_dialog.h
    crates/pictura-app/cpp/canvas_size_dialog.h
    crates/pictura-app/cpp/exposure_dialog.h
    crates/pictura-app/cpp/gradient_editor_dialog.h
    crates/pictura-app/cpp/gradient_map_dialog.h
    crates/pictura-app/cpp/gradient_noise_page.h
    crates/pictura-app/cpp/hue_saturation_dialog.h
    crates/pictura-app/cpp/image_size_dialog.h
    crates/pictura-app/cpp/levels_dialog.h
    crates/pictura-app/cpp/photo_filter_dialog.h
    crates/pictura-app/cpp/slider_dialog.h
    crates/pictura-app/cpp/replace_color_dialog.h
    crates/pictura-app/cpp/pictura_raw_dialog.h
    crates/pictura-app/cpp/warp_preset_dialog.h
    crates/pictura-app/cpp/color_settings_dialog.h
    crates/pictura-app/cpp/color_picker_dialog.h
    crates/pictura-app/cpp/export_as_dialog.h
    crates/pictura-app/cpp/duplicate_image_dialog.h
    crates/pictura-app/cpp/bitmap_mode_dialog.h
    crates/pictura-app/cpp/indexed_color_dialog.h
    crates/pictura-app/cpp/fill_dialog.h
    crates/pictura-app/cpp/defringe_dialog.h
    crates/pictura-app/cpp/stroke_dialog.h
    crates/pictura-app/cpp/image_view.h
    crates/pictura-app/cpp/pictura_debug_timing.h
    crates/pictura-app/cpp/canvas_range.h
    crates/pictura-app/cpp/canvas_scrollbars.h
    crates/pictura-app/cpp/icons.h
    crates/pictura-app/cpp/layer_adjustments.h
    crates/pictura-app/cpp/svg_icon_engine.h
    crates/pictura-app/cpp/file_drop_router.h
    crates/pictura-app/cpp/frame.h
    crates/pictura-app/cpp/panels/layers_panel.h
    crates/pictura-app/cpp/panels/layers_panel_model.h
    crates/pictura-app/cpp/panels/layers_filter_proxy.h
    crates/pictura-app/cpp/panels/layers_filter_bar.h
    crates/pictura-app/cpp/panels/percent_field.h
    crates/pictura-app/cpp/panels/numeric_field.h
    crates/pictura-app/cpp/panels/jump_slider.h
    crates/pictura-app/cpp/panels/angle_dial.h
    crates/pictura-app/cpp/panels/ramp_slider.h
    crates/pictura-app/cpp/panels/spectrum_bar.h
    crates/pictura-app/cpp/panels/curve_widget.h
    crates/pictura-app/cpp/panels/history_panel.h
    crates/pictura-app/cpp/panels/navigator_panel.h
    crates/pictura-app/cpp/panels/color_panel.h
    crates/pictura-app/cpp/panels/swatches_panel.h
    crates/pictura-app/cpp/panels/info_panel.h
    crates/pictura-app/cpp/panels/notes_panel.h
    crates/pictura-app/cpp/panels/histogram_panel.h
    crates/pictura-app/cpp/panels/placeholder_panel.h
    crates/pictura-app/cpp/panels/properties_panel.h
    crates/pictura-app/cpp/panels/channels_panel.h
    crates/pictura-app/cpp/panels/character_panel.h
    crates/pictura-app/cpp/panels/paragraph_panel.h
    crates/pictura-app/cpp/panels/paragraph_styles_panel.h
    crates/pictura-app/cpp/paragraph_style_dialog.h
    crates/pictura-app/cpp/panels/panel_group.h
    crates/pictura-app/cpp/panels/panel_column.h
    crates/pictura-app/cpp/tools.h
    crates/pictura-app/cpp/tool_context.h
    crates/pictura-app/cpp/tool_handler.h
    crates/pictura-app/cpp/tool_registry.h
    crates/pictura-app/cpp/selection_geometry.h
    crates/pictura-app/cpp/toolbox.h
    crates/pictura-app/cpp/options_bar.h
    crates/pictura-app/cpp/tool_hint_bar.h
    crates/pictura-app/cpp/new_document_dialog.h
    crates/pictura-app/cpp/layer_new_dialog.h
    crates/pictura-app/cpp/dialogs.h
)
target_include_directories(pictura_shell PUBLIC
    crates/pictura-app/cpp
    "${QT_HEADERS_DIR}/QtGui/${QT_VERSION_STR}/QtGui"
)
# The Rust static lib embeds `pictura-color` (Little CMS 2); a staticlib does not
# propagate its native link dependencies, so the C++ link needs `-llcms2`.
find_library(LCMS2_LIBRARY NAMES lcms2 REQUIRED)
target_link_libraries(pictura_shell PUBLIC pictura_app Qt6::Svg Qt6::Network Qt6::PrintSupport
    "${LCMS2_LIBRARY}")

# Compile the Qt resource once: the executable and every Qt Test binary link
# this object library instead of each embedding its own copy of the .qrc.
# POSITION_INDEPENDENT_CODE: object libraries default to non-PIC, but they are
# linked into PIE executables; lld rejects the qrc's references without it.
add_library(pictura_assets OBJECT assets/pictura.qrc)
set_target_properties(pictura_assets PROPERTIES POSITION_INDEPENDENT_CODE ON)

# The shell is a static library so the Qt Test suites under cpp/tests can link the
# UI without a second main. Only main.cpp and the self-test sources stay here.
add_executable(pictura
    crates/pictura-app/cpp/main.cpp
    crates/pictura-app/cpp/selftest_active_layer.cpp
    crates/pictura-app/cpp/selftest_annotations.cpp
    crates/pictura-app/cpp/selftest_canvas.cpp
    crates/pictura-app/cpp/selftest_canvas_view.cpp
    crates/pictura-app/cpp/selftest_click_deselect.cpp
    crates/pictura-app/cpp/selftest_control.cpp
    crates/pictura-app/cpp/selftest.cpp
    crates/pictura-app/cpp/selftest_crop_group.cpp
    crates/pictura-app/cpp/selftest_crop_tool.cpp
    crates/pictura-app/cpp/selftest_erasers.cpp
    crates/pictura-app/cpp/selftest_healing.cpp
    crates/pictura-app/cpp/selftest_large_doc.cpp
    crates/pictura-app/cpp/selftest_layer_locks.cpp
    crates/pictura-app/cpp/selftest_layers_adjustments.cpp
    crates/pictura-app/cpp/selftest_layers_controls.cpp
    crates/pictura-app/cpp/selftest_layers_drag.cpp
    crates/pictura-app/cpp/selftest_layers_filter.cpp
    crates/pictura-app/cpp/selftest_layers_interactions.cpp
    crates/pictura-app/cpp/selftest_layers_round3.cpp
    crates/pictura-app/cpp/selftest_layers_smart_object.cpp
    crates/pictura-app/cpp/selftest_magnetic_lasso.cpp
    crates/pictura-app/cpp/selftest_numeric.cpp
    crates/pictura-app/cpp/selftest_paint_live.cpp
    crates/pictura-app/cpp/selftest_paint_perf.cpp
    crates/pictura-app/cpp/selftest_paint_tools.cpp
    crates/pictura-app/cpp/selftest_pictura_raw.cpp
    crates/pictura-app/cpp/selftest_ports.cpp
    crates/pictura-app/cpp/selftest_recent_files.cpp
    crates/pictura-app/cpp/selftest_report.cpp
    crates/pictura-app/cpp/selftest_session.cpp
    crates/pictura-app/cpp/selftest_shell_round3.cpp
    crates/pictura-app/cpp/selftest_shell_round4.cpp
    crates/pictura-app/cpp/selftest_shell_round4_float.cpp
    crates/pictura-app/cpp/selftest_tool_canvas.cpp
    crates/pictura-app/cpp/selftest_tools_selection.cpp
    crates/pictura-app/cpp/selftest_ui_persistence.cpp
    crates/pictura-app/cpp/selftest_visibility.cpp
    crates/pictura-app/cpp/selftest_warp_preset.cpp
    crates/pictura-app/cpp/selftest_workspace_input.cpp
    crates/pictura-app/cpp/selftest_active_layer.h
    crates/pictura-app/cpp/selftest_annotations.h
    crates/pictura-app/cpp/selftest_canvas.h
    crates/pictura-app/cpp/selftest_canvas_view.h
    crates/pictura-app/cpp/selftest_click_deselect.h
    crates/pictura-app/cpp/selftest_control.h
    crates/pictura-app/cpp/selftest_crop_group.h
    crates/pictura-app/cpp/selftest_crop_tool.h
    crates/pictura-app/cpp/selftest_erasers.h
    crates/pictura-app/cpp/selftest.h
    crates/pictura-app/cpp/selftest_healing.h
    crates/pictura-app/cpp/selftest_large_doc.h
    crates/pictura-app/cpp/selftest_layer_locks.h
    crates/pictura-app/cpp/selftest_layers_adjustments.h
    crates/pictura-app/cpp/selftest_layers_controls.h
    crates/pictura-app/cpp/selftest_layers_drag.h
    crates/pictura-app/cpp/selftest_layers_filter.h
    crates/pictura-app/cpp/selftest_layers_interactions.h
    crates/pictura-app/cpp/selftest_layers_round3.h
    crates/pictura-app/cpp/selftest_layers_smart_object.h
    crates/pictura-app/cpp/selftest_magnetic_lasso.h
    crates/pictura-app/cpp/selftest_numeric.h
    crates/pictura-app/cpp/selftest_paint_fixture.h
    crates/pictura-app/cpp/selftest_paint_live.h
    crates/pictura-app/cpp/selftest_paint_perf.h
    crates/pictura-app/cpp/selftest_paint_tools.h
    crates/pictura-app/cpp/selftest_pictura_raw.h
    crates/pictura-app/cpp/selftest_ports.h
    crates/pictura-app/cpp/selftest_recent_files.h
    crates/pictura-app/cpp/selftest_report.h
    crates/pictura-app/cpp/selftest_session.h
    crates/pictura-app/cpp/selftest_shell_round3.h
    crates/pictura-app/cpp/selftest_shell_round4_float.h
    crates/pictura-app/cpp/selftest_shell_round4.h
    crates/pictura-app/cpp/selftest_tool_canvas.h
    crates/pictura-app/cpp/selftest_tools_selection.h
    crates/pictura-app/cpp/selftest_ui_persistence.h
    crates/pictura-app/cpp/selftest_visibility.h
    crates/pictura-app/cpp/selftest_warp_preset.h
    crates/pictura-app/cpp/selftest_workspace_input.h
)
target_link_libraries(pictura PRIVATE pictura_shell pictura_assets)

if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
    add_subdirectory(crates/pictura-app/cpp/tests)
endif()

# The AppImage job (.github/workflows/appimage.yml) stages this tree into an
# AppDir. The desktop file and icon share the binary's name so Wayland's default
# app_id ("pictura") resolves to them.
include(GNUInstallDirs)
install(TARGETS pictura RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
install(FILES packaging/pictura.desktop DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
install(FILES assets/icons/app.png
    DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/512x512/apps
    RENAME pictura.png)
