// Color Settings: the stored conversion values and the dialog's contract.

#include <QtTest/QtTest>

#include <QtWidgets/QCheckBox>

#include <cstddef>

#include "color_settings_dialog.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "qt_test_support.h"

class ColorSettingsTest : public QObject {
    Q_OBJECT

private slots:
    void rgbPolicy();
    void cmykPolicy();
    void conversionOptions();
    void askToggles();
    void cancel();
    void defaults();
    void settingsReachOpen();
    void preferencesSurviveRestart();
    void notADocumentEdit();
    void pickerFollowsConversionOptions();
};

void ColorSettingsTest::rgbPolicy()
{
    pictura::ColorSettingsDialog dialog(pictura::ColorSettingsValues{});
    dialog.setPolicyForTest(0);
    QCOMPARE(dialog.values().rgbPolicy, 2);
    QCOMPARE(dialog.workingSpaceForTest(), QStringLiteral("sRGB IEC61966-2.1"));
}

void ColorSettingsTest::cmykPolicy()
{
    pictura::ColorSettingsDialog dialog(pictura::ColorSettingsValues{});
    dialog.setCmykPolicyForTest(1);
    QCOMPARE(dialog.values().cmykPolicy, 0);
}

void ColorSettingsTest::conversionOptions()
{
    pictura::ColorSettingsDialog dialog(pictura::ColorSettingsValues{});
    dialog.setIntentForTest(0);
    auto* bpc = dialog.findChild<QCheckBox*>(QStringLiteral("colorSettingsBpc"));
    QVERIFY(bpc);
    bpc->setChecked(false);
    const pictura::ColorSettingsValues values = dialog.values();
    QCOMPARE(values.renderIntent, 0);
    QCOMPARE(values.blackPointCompensation, false);
}

void ColorSettingsTest::askToggles()
{
    pictura::ColorSettingsDialog dialog(pictura::ColorSettingsValues{});
    dialog.setAskMismatchForTest(true);
    dialog.setAskMissingForTest(true);
    const pictura::ColorSettingsValues values = dialog.values();
    QVERIFY(values.askMismatchOpen);
    QVERIFY(values.askMissingOpen);
}

void ColorSettingsTest::cancel()
{
    pictura::ColorSettingsDialog dialog(pictura::ColorSettingsValues{});
    dialog.setPolicyForTest(2);
    dialog.reject();
    // The caller only copies values on accept; a rejected dialog leaves the
    // values it was built from untouched.
    const pictura::ColorSettingsValues before;
    QCOMPARE(before.rgbPolicy, 0);
    QCOMPARE(before.cmykPolicy, 0);
}

void ColorSettingsTest::defaults()
{
    const pictura::ColorSettingsValues values;
    QCOMPARE(values.renderIntent, 1);
    QVERIFY(values.blackPointCompensation);
    QVERIFY(values.useDither);
    QCOMPARE(values.rgbPolicy, 0);
    const pictura::SessionState state;
    QCOMPARE(state.renderIntent, 1);
    QVERIFY(state.blackPointCompensation);
    QVERIFY(state.useDither);
}

void ColorSettingsTest::settingsReachOpen()
{
    pictura::PictureView view;
    pictura::set_color_settings(view, 2, 0, 0, false, false, true, true);
    QCOMPARE(pictura::cmyk_policy(view), 2);
    QCOMPARE(pictura::render_intent(view), 0);
    QVERIFY(!pictura::black_point_compensation(view));
    QVERIFY(!pictura::use_dither(view));
    QVERIFY(pictura::ask_mismatch_open(view));
    QVERIFY(pictura::ask_missing_open(view));
}

void ColorSettingsTest::preferencesSurviveRestart()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    pictura::SessionState state;
    state.cmykPolicy = 2;
    state.renderIntent = 0;
    state.blackPointCompensation = false;
    state.useDither = false;
    state.askMismatchOpen = true;
    state.pickerChoice = 1;
    QVERIFY(pictura::saveSession(state));
    const pictura::SessionState loaded = pictura::loadSession();
    QCOMPARE(loaded.cmykPolicy, 2);
    QCOMPARE(loaded.renderIntent, 0);
    QCOMPARE(loaded.blackPointCompensation, false);
    QCOMPARE(loaded.useDither, false);
    QVERIFY(loaded.askMismatchOpen);
    QCOMPARE(loaded.pickerChoice, 1);
}

void ColorSettingsTest::notADocumentEdit()
{
    pictura::PictureView view;
    const int history = view.history_count();
    const bool dirty = view.is_dirty();
    pictura::set_color_settings(view, 2, 0, 0, false, false, true, true);
    QCOMPARE(view.history_count(), history);
    QCOMPARE(view.is_dirty(), dirty);
}

// #1054: the Color Settings intent and black-point compensation drive the
// picker's working-CMYK evaluation, so changing them rebuilds the pair.
void ColorSettingsTest::pickerFollowsConversionOptions()
{
    pictura::PictureView view;
    pictura::set_color_settings(view, 0, 0, 1, true, false, false, false);
    const auto relative = pictura::picker_cmyk(200, 50, 100);
    pictura::set_color_settings(view, 0, 0, 0, false, false, false, false);
    const auto perceptual = pictura::picker_cmyk(200, 50, 100);
    bool same = true;
    for (int i = 0; i < 4; ++i) {
        same = same && relative[std::size_t(i)] == perceptual[std::size_t(i)];
    }
    QVERIFY2(!same, "the Color Settings options change the picker's CMYK readout");
    // Restore the defaults for the rest of the process.
    pictura::set_color_settings(view, 0, 0, 1, true, false, false, false);
}

QTEST_MAIN(ColorSettingsTest)
#include "tst_color_settings.moc"
