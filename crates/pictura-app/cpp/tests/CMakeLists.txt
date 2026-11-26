# Qt Test suites for the C++ shell. Explicit list, no globbing (repo convention).
set(PICTURA_QT_TEST_DRIVERS
    shell
    document
    io
    selection
    paint
    transform
    layers
    type
    ui
    filters
    imaging
    control
    preferences
    raw
)
set(PICTURA_QT_TEST_INCLUDED_SOURCES
    tst_actions_panel_cases.cpp
    tst_image_adjustments_shadows_highlights_cases.cpp
)
set(PICTURA_QT_TEST_DRIVER_shell
    tst_smoke
    tst_structured_logging
    tst_history_log
    tst_file_info
    tst_file_menu
    tst_gpu_backend
    tst_offthread_jobs
    tst_panel_host
    tst_menu_registrar
    tst_command_tree
    tst_edition_gating
    tst_toolbar_shape
    tst_command_bus
    tst_merge_commands
    tst_system_info
    tst_debug_toolset
    tst_tool_hint_bar
    tst_icon_assets
    tst_selftest_report
    tst_dialog_chrome
    tst_keyboard_shortcuts
    tst_search_bar
    tst_native_menu_bar
    tst_untrusted_input
    tst_theme_apply
    tst_test_support
    tst_app_updater
    tst_notification_status_bar
)
set(PICTURA_QT_TEST_DRIVER_document
    tst_document_version_policy
    tst_multi_file_open
    tst_document_api
    tst_document_registry
    tst_document_session
    tst_session_ports
    tst_edit_clipboard
    tst_history_snapshots
    tst_edit_refresh
    tst_image_ops
    tst_image_rotate
    tst_canvas_pan
    tst_initial_view
    tst_new_document
    tst_image_mode
    tst_hdr_conversion
    tst_align_distribute
    tst_measurement_scale
    tst_memory_governor
    tst_busy_job
    tst_document_jobs
    tst_edit_shortcuts
    tst_workspaces
    tst_floating_workspaces
    tst_floating_workspaces_restart
    tst_zoom_tool
    tst_large_document
    tst_tool_commit
    tst_recovery
    tst_session_store
    tst_settings_backup
    tst_rulers_guides
    tst_view_menu
    tst_panels_parity
    tst_variables
    tst_screen_modes
    tst_loading_tab
    tst_document_windows
    tst_document_float
    tst_document_arrange
    tst_document_navigate
    tst_canvas_context_menus
)
set(PICTURA_QT_TEST_DRIVER_io
    tst_open_save
    tst_place_fit
    tst_place_sequence
    tst_import_save_coordinator
    tst_save_for_web
    tst_export_as
    tst_export_batch
    tst_export_layers
    tst_file_export_tools
    tst_load_files_into_stack
    tst_statistics
    tst_webp
    tst_save_options
    tst_psd_compatibility
    tst_save_format_options
    tst_save_format_catalog
    tst_batch_dialog
    tst_batch_run
    tst_save_format_dialogs
    tst_save_gif_copy
    tst_print
    tst_save_pdf
    tst_import_pdf
    tst_image_import
)
set(PICTURA_QT_TEST_DRIVER_selection
    tst_selection_api
    tst_mask_edit
    tst_quick_mask
    tst_quick_mask_regressions
    tst_canvas_mask_domain
    tst_crop_straighten
    tst_crop_shield
    tst_crop_settings
    tst_crop_grip
    tst_crop_snap
    tst_crop_session
    tst_crop_options
    tst_crop_preview
    tst_crop_cursor
    tst_perspective_crop
    tst_color_range
    tst_refine_edge
    tst_replace_color
    tst_selection_aa
    tst_selection_alt
    tst_selection_move
    tst_selection_preview
    tst_selection_save_load
)
set(PICTURA_QT_TEST_DRIVER_paint
    tst_paint_api
    tst_color_replacement
    tst_options_bars_parity
    tst_fill_tools
    tst_eraser_tools
    tst_retouch_tools
    tst_stamp_hold
    tst_brush_airbrush
    tst_brush_dynamics
    tst_bristle_brushes
    tst_pen_tools
    tst_path_selection_tools
    tst_shape_tools
    tst_shape_transform
    tst_rotate_view
    tst_move_feedback
    tst_smart_guides
    tst_tool_options_persistence
    tst_brush_tip_preview
    tst_brush_outline
)
set(PICTURA_QT_TEST_DRIVER_transform
    tst_transform_api
    tst_transform_options
    tst_transform_controls
    tst_transform_skew
    tst_transform_distort
    tst_transform_perspective
    tst_transform_lock
    tst_transform_overlay
    tst_transform_warp
    tst_transform_commands
    tst_transform_selection
)
set(PICTURA_QT_TEST_DRIVER_layers
    tst_layers_mask_commands
    tst_mask_link_state
    tst_last_layer
    tst_layers_panel
    tst_layer_target
    tst_linked_layers
    tst_layers_panel_fixes
    tst_layers_panel_bundle
    tst_layers_row_menu
    tst_layers_row_model
    tst_layers_preview_coalescing
    tst_layer_content
    tst_layers_snapshot
    tst_canvas_backdrop
    tst_present_cache
    tst_rendering_indicator
    tst_damage_bounds
    tst_fill_layer_editor
    tst_layer_commands
    tst_properties_panel
    tst_properties_preview
    tst_channels_panel
    tst_channel_options
    tst_paths_panel
    tst_layer_style
    tst_blend_mode_list
    tst_auto_align
    tst_auto_blend
    tst_layers_smart_filters
    tst_smart_filter_apply
    tst_layers_effects
    tst_smart_object_actions
    tst_vector_smart_object
    tst_vector_paste
    tst_smart_object_relink
    tst_layer_comps_panel
    tst_layer_comps_export
    tst_nested_layer_target
)
set(PICTURA_QT_TEST_DRIVER_type
    tst_fonts
    tst_type_tools
    tst_type_panels
    tst_type_color_preview
    tst_type_edit_damage
    tst_paragraph_type
    tst_type_on_path
    tst_character_paragraph_panels
    tst_type_session_panels
    tst_paragraph_styles_panel
    tst_character_styles_panel
    tst_type_preferences
    tst_warp_text
    tst_type_menu
    tst_find_replace
    tst_type_placeholder
    tst_text_gamma_fonts
)
set(PICTURA_QT_TEST_DRIVER_ui
    tst_design_system
    tst_design_system_dialog
    tst_ui_scale
    tst_ui_localization
    tst_shared_widgets
    tst_value_input_enter
    tst_info_panel
    tst_histogram_panel
    tst_swatches_panel
    tst_preset_manager
    tst_color_picker
    tst_eyedropper
    tst_color_settings
    tst_proof_setup
    tst_profile_dialog
    tst_profile_warnings
    tst_adjustment_presets
    tst_adjustments_panel
    tst_actions_panel
)
set(PICTURA_QT_TEST_DRIVER_filters
    tst_filter_menu
    tst_filter_gallery
    tst_fade
    tst_lens_flare_dialog
    tst_picture_frame_dialog
    tst_reduce_noise_dialog
    tst_lighting_effects_dialog
    tst_shear_dialog
    tst_displace
    tst_de_interlace
    tst_custom_filter
    tst_maximum_filter
    tst_digimarc_watermark
    tst_shape_blur
    tst_smart_blur
    tst_lens_blur_dialog
    tst_blur_gallery_dialog
    tst_tilt_shift
    tst_blur_gallery_effects
    tst_path_blur
    tst_spin_blur
    tst_liquify_dialog
    tst_adaptive_wide_angle
    tst_lens_correction
    tst_lens_profiles
    tst_vanishing_point
    tst_vanishing_point_retouch
    tst_vanishing_point_files
)
set(PICTURA_QT_TEST_DRIVER_imaging
    tst_apply_image
    tst_calculations
    tst_match_color
    tst_image_adjustments
    tst_levels_canvas_controls
)
set(PICTURA_QT_TEST_DRIVER_control
    tst_client_registration
    tst_control_approval
    tst_control_editing
    tst_control_limits
    tst_control_live
    tst_control_metrics
    tst_control_metrics_off
    tst_control_modal_guard
    tst_control_policy
    tst_control_serial
)
set(PICTURA_QT_TEST_DRIVER_preferences
    tst_preferences_agent
    tst_performance_preferences
    tst_preferences_store
    tst_preferences_reset
    tst_update_preferences
)
set(PICTURA_QT_TEST_DRIVER_raw
    tst_pictura_raw
    tst_acr_settings
    tst_acr_dialog
    tst_raw_open
)

set(PICTURA_QT_TESTS "")
set(PICTURA_QT_TEST_SOURCES "")
foreach(_driver IN LISTS PICTURA_QT_TEST_DRIVERS)
    set(_driver_tests ${PICTURA_QT_TEST_DRIVER_${_driver}})
    list(APPEND PICTURA_QT_TESTS ${_driver_tests})
    foreach(_test_name IN LISTS _driver_tests)
        set(PICTURA_QT_TEST_TARGET_${_test_name} tst_driver_${_driver})
        list(APPEND PICTURA_QT_TEST_SOURCES ${_test_name}.cpp)
    endforeach()
endforeach()

list(LENGTH PICTURA_QT_TESTS _pictura_qt_test_count)
set(_pictura_qt_unique_tests ${PICTURA_QT_TESTS})
list(REMOVE_DUPLICATES _pictura_qt_unique_tests)
list(LENGTH _pictura_qt_unique_tests _pictura_qt_unique_count)
if(NOT _pictura_qt_test_count EQUAL _pictura_qt_unique_count)
    message(FATAL_ERROR "Qt Test suite registry contains duplicate suite names")
endif()

file(GLOB _pictura_qt_suite_sources CONFIGURE_DEPENDS
    "${CMAKE_CURRENT_SOURCE_DIR}/tst_*.cpp")
foreach(_source IN LISTS _pictura_qt_suite_sources)
    get_filename_component(_source_name "${_source}" NAME)
    if(_source_name IN_LIST PICTURA_QT_TEST_INCLUDED_SOURCES)
        continue()
    endif()
    if(NOT _source_name IN_LIST PICTURA_QT_TEST_SOURCES)
        message(FATAL_ERROR "Qt Test source is not registered: ${_source_name}")
    endif()
endforeach()
foreach(_source_name IN LISTS PICTURA_QT_TEST_SOURCES)
    if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/${_source_name}")
        message(FATAL_ERROR "Registered Qt Test source is missing: ${_source_name}")
    endif()
endforeach()

file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/qt-test-results)

# Shard membership and count come from retained duration evidence, not from a
# hand-maintained list: scripts/qt_test_timings.py reads the JUnit/CTest timing
# store (.ci/qt-test-timings.json) and writes cmake/qt-test-shards.json. Each
# plan entry names the suite, its share count, and (when measured) the exact
# functions of each share, so the split is a recorded decision.
#

# The plan is regenerated with:
#   python3 scripts/qt_test_timings.py plan --timings .ci/qt-test-timings.json \
#       --out cmake/qt-test-shards.json
#
# Only suites whose test functions are independent of each other may appear:
# a suite may share one window between functions and stay shardable when its
# per-function reset (init() and cleanup()) makes every function independent,
# so each passes run alone by name and in any share. suites whose functions
# build on each other's state stay out of the plan.
set(PICTURA_QT_SHARD_PLAN "${CMAKE_SOURCE_DIR}/cmake/qt-test-shards.json")
if(EXISTS "${PICTURA_QT_SHARD_PLAN}")
    file(READ "${PICTURA_QT_SHARD_PLAN}" _pictura_shard_json)
    string(JSON _pictura_plan_count ERROR_VARIABLE _pictura_plan_error LENGTH "${_pictura_shard_json}" suites)
    if(_pictura_plan_error)
        message(FATAL_ERROR "qt-test-shards.json is not valid JSON: ${_pictura_plan_error}")
    endif()
else()
    set(_pictura_plan_count 0)
    message(WARNING "cmake/qt-test-shards.json is missing; every suite runs in one entry")
endif()

set(PICTURA_QT_TEST_SHARDS "")
if(_pictura_plan_count GREATER 0)
    math(EXPR _pictura_last_plan "${_pictura_plan_count} - 1")
    foreach(_plan_index RANGE 0 ${_pictura_last_plan})
        string(JSON _plan_name GET "${_pictura_shard_json}" suites ${_plan_index} name)
        list(APPEND PICTURA_QT_TEST_SHARDS ${_plan_name})
        string(JSON ${_plan_name}_SHARD_COUNT GET "${_pictura_shard_json}" suites ${_plan_index} count)
        string(JSON _plan_shards_type ERROR_VARIABLE _plan_shards_error TYPE "${_pictura_shard_json}" suites ${_plan_index} shards)
        if(NOT _plan_shards_error AND _plan_shards_type STREQUAL "ARRAY")
            string(JSON _plan_shard_length LENGTH "${_pictura_shard_json}" suites ${_plan_index} shards)
            math(EXPR _plan_last_shard "${_plan_shard_length} - 1")
            foreach(_shard_index RANGE 0 ${_plan_last_shard})
                # Read each function by index rather than the whole array: an
                # indexed GET has one unambiguous result independent of how a
                # CMake version renders arrays, and the built list becomes one
                # command-line argument per function name.
                string(JSON _plan_fn_count LENGTH "${_pictura_shard_json}" suites ${_plan_index} shards ${_shard_index})
                set(_plan_functions "")
                if(_plan_fn_count GREATER 0)
                    math(EXPR _plan_last_fn "${_plan_fn_count} - 1")
                    foreach(_fn_index RANGE 0 ${_plan_last_fn})
                        string(JSON _plan_function GET "${_pictura_shard_json}" suites ${_plan_index} shards ${_shard_index} ${_fn_index})
                        list(APPEND _plan_functions "${_plan_function}")
                    endforeach()
                endif()
                set(${_plan_name}_SHARD_${_shard_index}_FUNCTIONS "${_plan_functions}")
            endforeach()
        endif()
    endforeach()
endif()

# A plan entry that names no registered suite would be silently ignored, so
# configuration fails instead: the plan and the suite list must agree.
foreach(_plan_suite IN LISTS PICTURA_QT_TEST_SHARDS)
    if(NOT _plan_suite IN_LIST PICTURA_QT_TESTS)
        message(FATAL_ERROR
            "cmake/qt-test-shards.json shards '${_plan_suite}', which is not a registered suite")
    endif()
endforeach()


# The suites share one precompiled header, built by this object library with
# their own flags: they cannot reuse the shell's, which is compiled position
# independent for the shared library and with full debug info.
if(PICTURA_DEV_PCH)
    add_library(pictura_test_pch OBJECT test_pch.cpp)
    set_target_properties(pictura_test_pch PROPERTIES AUTOMOC OFF)
    target_link_libraries(pictura_test_pch PRIVATE pictura_shell Qt6::Test)
    target_compile_options(pictura_test_pch PRIVATE ${PICTURA_DEV_COMPILE_OPTIONS}
        "$<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-g1>")
    target_precompile_headers(pictura_test_pch PRIVATE "${PICTURA_PCH_HEADER}")
endif()

# The shared fixture's bodies compile here, once, instead of in every suite that
# includes qt_test_support.h. AUTOMOC OFF: its translation unit includes frame.h
# to construct the window, and a moc scan of it would emit a second meta-object
# for PicturaMainWindow. PIC for the same reason pictura_assets sets it: the
# suites are PIE executables and reject non-PIC objects.
add_library(pictura_test_support STATIC qt_test_support.cpp)
set_target_properties(pictura_test_support PROPERTIES
    AUTOMOC OFF
    POSITION_INDEPENDENT_CODE ON)
target_link_libraries(pictura_test_support PRIVATE pictura_shell Qt6::Test)
target_compile_options(pictura_test_support PRIVATE ${PICTURA_DEV_COMPILE_OPTIONS}
    "$<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-g1>")

foreach(_driver IN LISTS PICTURA_QT_TEST_DRIVERS)
    set(_driver_tests ${PICTURA_QT_TEST_DRIVER_${_driver}})
    set(_driver_sources "")
    foreach(_test_name IN LISTS _driver_tests)
        list(APPEND _driver_sources ${_test_name}.cpp)
    endforeach()
    add_executable(tst_driver_${_driver} ${_driver_sources})
    if(PICTURA_SHELL_SHARED)
        target_link_libraries(tst_driver_${_driver} PRIVATE pictura_shell Qt6::Test
            pictura_test_support)
    else()
        target_link_libraries(tst_driver_${_driver} PRIVATE pictura_shell Qt6::Test
            pictura_assets pictura_test_support)
    endif()
    # Test debug info only needs line tables; the suites are not debugged through
    # full variable info, and this keeps the test objects and their links small.
    target_compile_options(tst_driver_${_driver} PRIVATE ${PICTURA_DEV_COMPILE_OPTIONS}
        "$<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-g1>")
    if(TARGET pictura_test_pch)
        target_precompile_headers(tst_driver_${_driver} REUSE_FROM pictura_test_pch)
    endif()
endforeach()

foreach(test_name IN LISTS PICTURA_QT_TESTS)
    set(_driver ${PICTURA_QT_TEST_TARGET_${test_name}})
    set(_pictura_suite_shares 1)
    if(DEFINED ${test_name}_SHARD_COUNT)
        set(_pictura_suite_shares ${${test_name}_SHARD_COUNT})
    endif()
    if(test_name IN_LIST PICTURA_QT_TEST_SHARDS AND _pictura_suite_shares GREATER 1)
        math(EXPR _pictura_last_shard "${_pictura_suite_shares} - 1")
        foreach(_shard RANGE 0 ${_pictura_last_shard})
            set(_entry "${test_name}_${_shard}")
            set(_share_functions "")
            if(DEFINED ${test_name}_SHARD_${_shard}_FUNCTIONS)
                set(_share_functions ${${test_name}_SHARD_${_shard}_FUNCTIONS})
            endif()
            add_test(NAME ${_entry} COMMAND ${_driver}
                --suite ${test_name}
                -o ${CMAKE_BINARY_DIR}/qt-test-results/${_entry}.xml,junitxml
                ${_share_functions})

            set_tests_properties(${_entry} PROPERTIES
                ENVIRONMENT "QT_QPA_PLATFORM=offscreen;PICTURA_TEST_SHARD=${_shard}/${_pictura_suite_shares}"
                TIMEOUT 120
            )
        endforeach()
    else()
        add_test(NAME ${test_name} COMMAND ${_driver}
            --suite ${test_name}
            -o ${CMAKE_BINARY_DIR}/qt-test-results/${test_name}.xml,junitxml)
        set_tests_properties(${test_name} PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
            TIMEOUT 120
        )
    endif()
endforeach()

add_dependencies(tst_driver_shell pictura)
add_dependencies(tst_driver_ui pictura)
add_dependencies(tst_driver_preferences pictura)
set_property(TEST tst_update_preferences APPEND PROPERTY ENVIRONMENT
    "PICTURA_TEST_EXECUTABLE=$<TARGET_FILE:pictura>")

set_source_files_properties(
    tst_structured_logging.cpp
    tst_ui_localization.cpp
    tst_preferences_store.cpp
    PROPERTIES SKIP_PRECOMPILE_HEADERS ON)
set_source_files_properties(tst_structured_logging.cpp PROPERTIES
    COMPILE_DEFINITIONS "PICTURA_LOGGING_TEST_EXECUTABLE=\"$<TARGET_FILE:pictura>\"")
set_source_files_properties(tst_ui_localization.cpp PROPERTIES
    COMPILE_DEFINITIONS "PICTURA_TEST_APP=\"$<TARGET_FILE:pictura>\"")
set_source_files_properties(tst_preferences_store.cpp PROPERTIES
    COMPILE_DEFINITIONS "PICTURA_TEST_EXECUTABLE=\"$<TARGET_FILE:pictura>\"")
set_tests_properties(tst_ui_localization PROPERTIES TIMEOUT 240)

# The brush-outline suite maps logical canvas coordinates to QImage device
# indices; pin the device pixel ratio so the two agree.
set_tests_properties(tst_brush_outline PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_SCALE_FACTOR=1")


# Quick Mask runs about 112 s under the 80% CPU budget; give it headroom over
# the default 120 s timeout so a loaded machine does not fail a passing suite.
# The timeout is per entry, so every share the plan registers gets it.
if(tst_quick_mask IN_LIST PICTURA_QT_TEST_SHARDS AND tst_quick_mask_SHARD_COUNT GREATER 1)
    math(EXPR _pictura_quick_mask_last "${tst_quick_mask_SHARD_COUNT} - 1")
    foreach(_quick_mask_shard RANGE 0 ${_pictura_quick_mask_last})
        set_tests_properties(tst_quick_mask_${_quick_mask_shard} PROPERTIES TIMEOUT 300)
    endforeach()
else()
    set_tests_properties(tst_quick_mask PROPERTIES TIMEOUT 300)
endif()


# The suites include the generated cxx-qt header, so they follow the bridge
# stamp like the shell does.
pictura_depend_on_bridge_stamp(${PICTURA_QT_TEST_SOURCES}
    ${CMAKE_SOURCE_DIR}/crates/pictura-app/cpp/selftest_report.cpp
    ${CMAKE_SOURCE_DIR}/crates/pictura-app/cpp/selftest_run.cpp)

# The self-test token harness lives in the selftest sources, which are linked
# only into the pictura executable, so this suite compiles the harness itself.
target_sources(tst_driver_shell PRIVATE
    ${CMAKE_SOURCE_DIR}/crates/pictura-app/cpp/selftest_report.cpp
    ${CMAKE_SOURCE_DIR}/crates/pictura-app/cpp/selftest_run.cpp)
