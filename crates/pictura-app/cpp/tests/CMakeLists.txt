# Qt Test suites for the C++ shell. Explicit list, no globbing (repo convention).
set(PICTURA_QT_TESTS
    tst_smoke
    tst_fonts
    tst_edit_clipboard
    tst_mask_edit
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
    tst_perspective_crop
    tst_canvas_pan
    tst_initial_view
    tst_new_document
    tst_image_mode
    tst_layers_panel
    tst_align_distribute
    tst_options_bars_parity
    tst_transform_options
    tst_transform_controls
    tst_command_tree
    tst_fill_tools
    tst_retouch_tools
    tst_pen_tools
    tst_path_selection_tools
    tst_shape_tools
    tst_rotate_view
    tst_save_for_web
    tst_type_tools
    tst_type_panels
    tst_character_paragraph_panels
    tst_paragraph_styles_panel
    tst_shared_widgets
    tst_info_panel
    tst_histogram_panel
    tst_tool_hint_bar
    tst_swatches_panel
    tst_properties_panel
    tst_channels_panel
    tst_paths_panel
    tst_icon_assets
    tst_filter_menu
    tst_filter_gallery
    tst_lens_flare_dialog
    tst_lighting_effects_dialog
    tst_shear_dialog
    tst_layer_style
    tst_color_range
    tst_refine_edge
    tst_replace_color
    tst_match_color
    tst_print
    tst_image_adjustments
    tst_layers_smart_filters
    tst_smart_object_actions
    tst_edit_shortcuts
    tst_workspaces
    tst_zoom_tool
    tst_large_document
    tst_recovery
    tst_rulers_guides
    tst_panels_parity
    tst_selection_alt
    tst_nested_layer_target
    tst_screen_modes
)

file(MAKE_DIRECTORY ${CMAKE_BINARY_DIR}/qt-test-results)

foreach(test_name IN LISTS PICTURA_QT_TESTS)
    add_executable(${test_name} ${test_name}.cpp)
    target_link_libraries(${test_name} PRIVATE pictura_shell Qt6::Test pictura_assets)
    add_test(NAME ${test_name} COMMAND ${test_name}
        -o ${CMAKE_BINARY_DIR}/qt-test-results/${test_name}.xml,junitxml)
    set_tests_properties(${test_name} PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
        TIMEOUT 120
    )
endforeach()
