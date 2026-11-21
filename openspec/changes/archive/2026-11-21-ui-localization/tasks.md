# Tasks

Complete the test-writing tasks (1.1, 1.2, 2.1, 3.1, and 3.2) before the feature
code. Build and run the tests once the implementation is complete.

## 1. Bundled catalogs and language selection

- [x] 1.1 Add and register `tst_ui_localization`. Write `bundledCatalogsTranslateUi`, `unsupportedSystemLanguageUsesEnglish`, `partialCatalogPreservesCommandIdentity`, `explicitSelectionAppliesOnRestart`, and `englishAndSystemCanBeSelected` before implementation. Verify the cases cover the agreed surfaces, the original dispatch path, and initial and restored state.
- [x] 1.2 Before implementation, write `tst_ui_localization::dynamicMenusKeepIdentity`, `languageSaveFailurePreservesChoice`, and `selfTestUsesDeterministicLocale`. Exercise actual Open Recent and Workspace population and dispatch in German and Finnish, a refused preferences write, and the existing self-test under non-English UI and numeric environment settings. Have CMake provide the built `pictura` executable path to the test, and launch it in a fresh process with `--headless --self-test` and controlled environment variables. Preserve the existing self-test checks and tokens; cover localized startup in Qt Test.
- [x] 1.3 Configure Qt LinguistTools in the build and ensure CI and packaging supply the build tools. Generate embedded German and Finnish resources for shared-shell and static-release builds, and verify both resources are discoverable through the application startup path.
- [x] 1.4 Extract the command and tool labels from their canonical sources and translate them when the Qt UI displays them. Mark app-owned text in New Document, Open, Save As, and Preferences for extraction. Fill German and then Finnish catalogs, and verify both have finished entries for the agreed surfaces and that the menu and toolbox tests pass. Resolve dynamic Open Recent and Workspace menus by stable identity, and verify their population and dispatch.
- [x] 1.5 Add stable language values in the typed Preferences store, startup selection and fallback, the Interface selector, and its restart notice. Establish English UI and C numeric locale for the existing self-test startup path regardless of the launch environment, retaining all existing checks and tokens. Verify migration from an old session, propagate preference save failures, restore the last saved choice and show an error without restart success guidance on failure, and verify the active and pending choices in the restart and failure tests.

## 2. Community translations

- [x] 2.1 Write `tst_ui_localization::importsAndActivatesLocalCatalog`, `failedImportPreservesSelection`, and `unavailableImportUsesSystem` before implementation. Cover cancelling the picker, retaining the usable import after deleting its source, rejecting malformed or external references, and restoring a missing catalog selection. Write `tst_ui_localization::importedMarkupRendersAsPlainText` using a valid local TS fixture containing rich-text-looking text for a plain Preferences label.
- [x] 2.2 Implement bounded Qt Linguist source import, the context-based translator, and atomic app-data publication. Wire **Import Translation** to the Preferences file-picker and show validation failures there. Verify that the success, rejection, and fallback cases pass and that import leaves the active UI unchanged. Force plain-text presentation at plain sites and escape catalog text inserted into app-owned markup.
- [x] 2.3 Document how contributors produce a compatible `.ts`, which cases use English fallback, and how to install it using Preferences. Verify the instructions with the same source format used by the import test.

## 3. Locale-aware numeric input and readouts

- [x] 3.1 Write `tst_ui_localization::numericLocaleIsIndependent`, `sharedFieldCommitsCommaDecimal`, and `sharedFieldAppliesLocaleToKeys`, plus `tst_new_document::localizedWidthSurvivesPsdRoundTrip`, before implementation. Verify the tests assert numeric values and visible text, including the reopened PSD's width and resolution.
- [x] 3.2 Write `tst_rulers_guides::rulerLabelsAndMeasurementsUseNumericLocale` and `tst_info_panel::colourAndDocumentSizeUseNumericLocale`, `rulerAngleAndLengthUseNumericLocale`, and `positionAndSizeUseNumericLocale`. Extend the existing ruler-mode and shared-unit cases to check selection restoration and position conversion. Write `tst_rulers_guides::calibratedLengthUsesNumericLocale` and `tst_info_panel::customScaleUsesNumericLocale` for fractional values, units, and unchanged canonical data. Pin the existing decimal-period assertions to a period-decimal locale.
- [x] 3.3 Use the system numeric locale for shared-field validation, display, clamping and stepping and for New Document dimensions. Verify the mixed-locale, keyboard, and PSD round-trip tests pass.
- [x] 3.4 Apply the numeric locale to ruler labels, ruler measurement text, and Info values. Format Info custom scales from typed scale data and expose numeric calibrated ruler length to the options bar for locale-aware display. Verify the ruler and Info cases pass with unchanged units, canonical data, and geometry.

## 4. Interface-wide coverage

- [x] 4.1 Add coverage for translated panel tabs and contents, Options bar controls, and standalone dialogs while asserting stable widget and choice identities.
- [x] 4.2 Extract app-owned Qt presentation text across production shell sources. Mark hardcoded panel, Options bar, dialog, and shell text at display sites. Preserve stored values, control behavior, and plain-text handling of community translations.
- [x] 4.3 Refresh the German and Finnish catalogs and check them against the extracted message list.
- [x] 4.4 Add the conditional Restart Pictura control. Reuse document save checks and verify visibility, cancellation, and launch failure. File a follow-up to investigate immediate retranslation.

## 5. Integration

- [x] 5.1 Build with the repository build wrapper and run the named Qt Test suites, test-plan guard, and strict OpenSpec validation. Check both bundled catalogs in an installed release build using the checklist row.
- [x] 5.2 Update the roadmap to reflect shipped #642 work and the open #1488 RTL work. Include the manual test-plan row in the PR body and archive the completed OpenSpec change before the PR ships.
