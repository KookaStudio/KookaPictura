# Qt Test suites for the C++ shell. Explicit list, no globbing (repo convention).
set(PICTURA_QT_TESTS
    tst_smoke
    tst_document_api
    tst_document_session
    tst_selection_api
    tst_paint_api
    tst_transform_api
    tst_fonts
    tst_design_system
    tst_design_system_dialog
    tst_edit_clipboard
    tst_mask_edit
    tst_layers_mask_commands
    tst_last_layer
    tst_image_ops
    tst_image_rotate
    tst_crop_straighten
    tst_crop_shield
    tst_crop_grip
    tst_crop_snap
    tst_crop_session
    tst_crop_options
    tst_crop_preview
    tst_crop_cursor
    tst_session_ports
    tst_perspective_crop
    tst_canvas_pan
    tst_initial_view
    tst_new_document
    tst_image_mode
    tst_layers_panel
    tst_layer_target
    tst_layers_panel_fixes
    tst_layers_preview_coalescing
    tst_layers_snapshot
    tst_align_distribute
    tst_options_bars_parity
    tst_transform_options
    tst_transform_controls
    tst_transform_skew
    tst_transform_distort
    tst_transform_perspective
    tst_transform_lock
    tst_transform_overlay
    tst_transform_warp
    tst_transform_commands
    tst_command_tree
    tst_merge_commands
    tst_layer_commands
    tst_fill_tools
    tst_retouch_tools
    tst_pen_tools
    tst_path_selection_tools
    tst_shape_tools
    tst_rotate_view
    tst_save_for_web
    tst_export_as
    tst_webp
    tst_type_tools
    tst_type_panels
    tst_paragraph_type
    tst_move_feedback
    tst_smart_guides
    tst_tool_options_persistence
    tst_character_paragraph_panels
    tst_paragraph_styles_panel
    tst_shared_widgets
    tst_info_panel
    tst_histogram_panel
    tst_tool_hint_bar
    tst_swatches_panel
    tst_color_picker
    tst_properties_panel
    tst_channels_panel
    tst_paths_panel
    tst_icon_assets
    tst_filter_menu
    tst_filter_gallery
    tst_lens_flare_dialog
    tst_picture_frame_dialog
    tst_reduce_noise_dialog
    tst_lighting_effects_dialog
    tst_shear_dialog
    tst_displace
    tst_custom_filter
    tst_shape_blur
    tst_smart_blur
    tst_lens_blur_dialog
    tst_blur_gallery_dialog
    tst_tilt_shift
    tst_blur_gallery_effects
    tst_path_blur
    tst_spin_blur
    tst_liquify_dialog
    tst_layer_style
    tst_color_range
    tst_refine_edge
    tst_replace_color
    tst_match_color
    tst_print
    tst_save_pdf
    tst_import_pdf
    tst_image_adjustments
    tst_layers_smart_filters
    tst_smart_object_actions
    tst_edit_shortcuts
    tst_workspaces
    tst_floating_workspaces
    tst_floating_workspaces_restart
    tst_zoom_tool
    tst_large_document
    tst_tool_commit
    tst_recovery
    tst_session_store
    tst_rulers_guides
    tst_panels_parity
    tst_selection_alt
    tst_selection_move
    tst_nested_layer_target
    tst_screen_modes
    tst_dialog_chrome
    tst_keyboard_shortcuts
    tst_search_bar
    tst_native_menu_bar
    tst_loading_tab
    tst_pictura_raw
    tst_acr_settings
    tst_acr_dialog
    tst_canvas_context_menus
    tst_brush_tip_preview
)

file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/qt-test-results)

# The slow suites are registered in several entries, so the layer's wall time
# is set by the number of suites and the parallel level rather than by one
# slow suite. Each entry runs a share of the suite's test functions: the
# share comes from the PICTURA_TEST_SHARD environment variable, and the
# functions are distributed over the shares in declaration order.
#
# Only suites whose test functions are independent of each other belong here.
# A suite that shares its window between the test functions keeps one entry:
# its functions build on each other's state.
set(PICTURA_QT_TEST_SHARDS
    tst_floating_workspaces
    tst_import_pdf
    tst_save_pdf
    tst_workspaces
)
set(PICTURA_QT_TEST_SHARD_COUNT 4)

set(PICTURA_QT_TEST_SOURCES "")
foreach(test_name IN LISTS PICTURA_QT_TESTS)
    list(APPEND PICTURA_QT_TEST_SOURCES ${test_name}.cpp)
    add_executable(${test_name} ${test_name}.cpp)
    if(PICTURA_SHELL_SHARED)
        target_link_libraries(${test_name} PRIVATE pictura_shell Qt6::Test)
    else()
        target_link_libraries(${test_name} PRIVATE pictura_shell Qt6::Test pictura_assets)
    endif()
    # Test debug info only needs line tables; the suites are not debugged through
    # full variable info, and this keeps the test objects and their links small.
    target_compile_options(${test_name} PRIVATE "$<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-g1>")

    if(test_name IN_LIST PICTURA_QT_TEST_SHARDS)
        math(EXPR _pictura_last_shard "${PICTURA_QT_TEST_SHARD_COUNT} - 1")
        foreach(_shard RANGE 0 ${_pictura_last_shard})
            set(_entry "${test_name}_${_shard}")
            add_test(NAME ${_entry} COMMAND ${test_name}
                -o ${CMAKE_BINARY_DIR}/qt-test-results/${_entry}.xml,junitxml)
            set_tests_properties(${_entry} PROPERTIES
                ENVIRONMENT "QT_QPA_PLATFORM=offscreen;PICTURA_TEST_SHARD=${_shard}/${PICTURA_QT_TEST_SHARD_COUNT}"
                TIMEOUT 120
            )
        endforeach()
    else()
        add_test(NAME ${test_name} COMMAND ${test_name}
            -o ${CMAKE_BINARY_DIR}/qt-test-results/${test_name}.xml,junitxml)
        set_tests_properties(${test_name} PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
            TIMEOUT 120
        )
    endif()
endforeach()

# The suites include the generated cxx-qt header, so they follow the bridge
# stamp like the shell does.
pictura_depend_on_bridge_stamp(${PICTURA_QT_TEST_SOURCES})
