#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QPointF>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QTabBar>

#include "acr/acr_controls.h"
#include "acr/acr_histogram.h"
#include "acr/acr_panels.h"
#include "acr/acr_preview_pane.h"
#include "acr/acr_settings.h"
#include "frame.h"
#include "panels/curve_widget.h"
#include "pictura_raw_dialog.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

bool setSlider(QWidget* panel, const QByteArray& key, double value)
{
    auto* row = panel->findChild<AcrSlider*>(QString::fromLatin1(key));
    if (!row) {
        return false;
    }
    auto* spin = row->findChild<QDoubleSpinBox*>();
    if (!spin) {
        return false;
    }
    spin->setValue(value);
    return true;
}

} // namespace

class AcrDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void dialogShowsEveryTab();
    void basicPanelWritesKeys();
    void basicPanelWhiteBalanceWritesEnumCode();
    void toneCurvePanelWritesKeys();
    void toneCurvePointEditWritesKey();
    void detailPanelWritesKeys();
    void hslPanelWritesKeys();
    void splitToningPanelWritesKeys();
    void lensPanelWritesKeys();
    void effectsPanelWritesKeysAndStyleRule();
    void calibrationPanelWritesKeys();
    void presetsPanelHasItems();
    void previewPaneAndHistogram();
    void clippingTintsOutOfRange();
    void applyAcrossTabsRecordsOneState();
    void cancelAppliesNothing();

private:
    QString makeRasterLayer();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void AcrDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QCoreApplication::processEvents();
}

void AcrDialogTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

QString AcrDialogTest::makeRasterLayer()
{
    if (!window_->newDocument(QStringLiteral("AcrDialog"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return {};
    }
    PictureView* view = window_->activeView();
    if (!view) {
        return {};
    }
    const QString path = view->add_solid_fill(0xff808080u);
    view->rasterize_fill_content(path);
    return path;
}

void AcrDialogTest::dialogShowsEveryTab()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    QTabBar* tabs = dialog.tabBarForTest();
    QVERIFY(tabs);
    QCOMPARE(tabs->count(), 9);
    QCOMPARE(tabs->currentIndex(), 0);
    QVERIFY2(tabs->tabText(0).isEmpty(), "tabs are icon-only");
    QCOMPARE(tabs->tabToolTip(0), QStringLiteral("Basic"));
    QCOMPARE(tabs->tabToolTip(8), QStringLiteral("Presets"));
}

void AcrDialogTest::basicPanelWritesKeys()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrBasicPanel panel(&settings);
    QVERIFY(setSlider(&panel, acr::Temp, 20));
    QVERIFY(setSlider(&panel, acr::Exposure, 1.5));
    QCOMPARE(settings.intValue(acr::Temp), 20);
    QCOMPARE(settings.doubleValue(acr::Exposure), 1.5);
}

void AcrDialogTest::basicPanelWhiteBalanceWritesEnumCode()
{
    // A stored enum code prefills the label.
    AcrSettings pre = AcrSettings::decode(QStringLiteral("WBal=Dayl"));
    AcrBasicPanel prePanel(&pre);
    auto* preCombo = prePanel.findChild<AcrCombo*>(QString::fromLatin1(acr::WBal));
    QVERIFY(preCombo);
    QCOMPARE(preCombo->value(), QStringLiteral("Daylight"));

    // Picking a label stores the 4-byte enum code, not the label text.
    AcrSettings settings = AcrSettings::decode(QString());
    AcrBasicPanel panel(&settings);
    auto* combo = panel.findChild<AcrCombo*>(QString::fromLatin1(acr::WBal));
    QVERIFY(combo);
    combo->setValue(QStringLiteral("Daylight"));
    QCOMPARE(settings.raw(acr::WBal), QByteArrayLiteral("Dayl"));
    QVERIFY2(settings.encode().contains(QStringLiteral("WBal=Dayl")),
             qPrintable(settings.encode()));
}

void AcrDialogTest::toneCurvePanelWritesKeys()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrToneCurvePanel panel(&settings);
    QVERIFY(setSlider(&panel, acr::ParamShadows, -40));
    QCOMPARE(settings.intValue(acr::ParamShadows), -40);
    auto* curve = panel.findChild<QComboBox*>(QStringLiteral("acrCurve"));
    QVERIFY(curve);
    curve->setCurrentText(QStringLiteral("Medium Contrast"));
    QCOMPARE(settings.curveValue(acr::CurveRgb),
             QList<int>({0, 0, 64, 48, 128, 140, 192, 205, 255, 255}));
}

void AcrDialogTest::toneCurvePointEditWritesKey()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrToneCurvePanel panel(&settings);
    auto* editor = panel.findChild<CurveWidget*>(QStringLiteral("acrCurveWidget"));
    QVERIFY(editor);
    editor->setPoints(
        {QPointF(0.0, 0.0), QPointF(128.0 / 255.0, 64.0 / 255.0), QPointF(1.0, 1.0)});
    QCOMPARE(settings.curveValue(acr::CurveRgb), QList<int>({0, 0, 128, 64, 255, 255}));
}

void AcrDialogTest::detailPanelWritesKeys()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrDetailPanel panel(&settings);
    QVERIFY(setSlider(&panel, acr::SharpenAmount, 40));
    QVERIFY(setSlider(&panel, acr::NoiseColor, 15));
    QCOMPARE(settings.intValue(acr::SharpenAmount), 40);
    QCOMPARE(settings.intValue(acr::NoiseColor), 15);
}

void AcrDialogTest::hslPanelWritesKeys()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrHslPanel panel(&settings);
    const QByteArray redHue("HA_R");
    QVERIFY(setSlider(&panel, redHue, 30));
    QCOMPARE(settings.intValue(redHue), 30);
    auto* gray = panel.findChild<AcrCheck*>(QString::fromLatin1(acr::GrayScale));
    QVERIFY(gray);
    gray->setChecked(true);
    QCOMPARE(settings.boolValue(acr::GrayScale), true);
}

void AcrDialogTest::splitToningPanelWritesKeys()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrSplitToningPanel panel(&settings);
    QVERIFY(setSlider(&panel, acr::ToningShadowHue, 210));
    QCOMPARE(settings.intValue(acr::ToningShadowHue), 210);
}

void AcrDialogTest::lensPanelWritesKeys()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrLensPanel panel(&settings);
    QVERIFY(setSlider(&panel, acr::Distortion, -15));
    QVERIFY(setSlider(&panel, acr::LensVignetteAmount, -20));
    QCOMPARE(settings.intValue(acr::Distortion), -15);
    QCOMPARE(settings.intValue(acr::LensVignetteAmount), -20);
}

void AcrDialogTest::effectsPanelWritesKeysAndStyleRule()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrEffectsPanel panel(&settings);
    QVERIFY(setSlider(&panel, acr::Dehaze, 7));
    QCOMPARE(settings.intValue(acr::Dehaze), 7);
    auto* style = panel.findChild<QComboBox*>(QStringLiteral("acrPcvStyle"));
    QVERIFY(style);
    QVERIFY2(!style->isEnabled(), "Style is disabled while Amount is 0");
    QVERIFY(setSlider(&panel, acr::PcvAmount, -20));
    QVERIFY2(style->isEnabled(), "Style enables once Amount leaves 0");
}

void AcrDialogTest::calibrationPanelWritesKeys()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrCalibrationPanel panel(&settings);
    QVERIFY(setSlider(&panel, acr::ShadowTint, -6));
    QVERIFY(setSlider(&panel, acr::RedHue, 5));
    QCOMPARE(settings.intValue(acr::ShadowTint), -6);
    QCOMPARE(settings.intValue(acr::RedHue), 5);
    auto* process = panel.findChild<QComboBox*>(QStringLiteral("acrProcess"));
    QVERIFY(process);
    process->setCurrentIndex(1);
    QCOMPARE(settings.intValue(acr::ProcessVersion), 2010);

    // A stored profile name (with its trailing NUL) prefills the combo.
    AcrSettings pre = AcrSettings::decode(QStringLiteral("CamP=Pictura Standard\\x00"));
    AcrCalibrationPanel prePanel(&pre);
    auto* profile = prePanel.findChild<AcrCombo*>(QString::fromLatin1(acr::CameraProfile));
    QVERIFY(profile);
    QCOMPARE(profile->value(), QStringLiteral("Pictura Standard"));
}

void AcrDialogTest::presetsPanelHasItems()
{
    AcrSettings settings = AcrSettings::decode(QStringLiteral("Temp=20"));
    AcrPresetsPanel panel(&settings);
    auto* list = panel.findChild<QListWidget*>();
    QVERIFY(list);
    QVERIFY(list->count() >= 8);
    list->setCurrentRow(1);
    // Selecting a preset item must not touch the settings map.
    QCOMPARE(settings.intValue(acr::Temp), 20);
}

void AcrDialogTest::previewPaneAndHistogram()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    AcrPreviewPane* pane = dialog.previewPaneForTest();
    AcrHistogram* hist = dialog.histogramForTest();
    QCOMPARE(hist->seriesCountForTest(), 3);

    QVERIFY(setSlider(&dialog, acr::Exposure, 1.5));
    QTest::qWait(80);
    const QImage bright = pane->image();
    QVERIFY(!bright.isNull());
    const QString brightReadout = hist->readoutForTest();
    QVERIFY2(brightReadout.contains(QStringLiteral("R:")), qPrintable(brightReadout));

    QVERIFY(setSlider(&dialog, acr::Exposure, -2.0));
    QTest::qWait(80);
    const QImage dark = pane->image();
    QVERIFY(!dark.isNull());
    QVERIFY2(bright != dark, "editing a control changes the preview image");
    QVERIFY2(hist->readoutForTest() != brightReadout, "the histogram follows the preview");
}

void AcrDialogTest::clippingTintsOutOfRange()
{
    QImage black(2, 2, QImage::Format_ARGB32);
    black.fill(Qt::black);
    QCOMPARE(AcrPreviewPane::applyClipping(black, true, false).pixelColor(0, 0),
             QColor(0, 0, 255));
    QImage white(2, 2, QImage::Format_ARGB32);
    white.fill(Qt::white);
    QCOMPARE(AcrPreviewPane::applyClipping(white, false, true).pixelColor(0, 0),
             QColor(255, 0, 0));
}

void AcrDialogTest::applyAcrossTabsRecordsOneState()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PictureView* view = window_->activeView();
    PicturaRawDialog dialog(view, path, QString(), nullptr);
    QVERIFY(setSlider(&dialog, acr::Temp, 20));
    QVERIFY(setSlider(&dialog, acr::Dehaze, 7));
    const QString text = dialog.settingsText();
    QVERIFY2(text.contains(QStringLiteral("Temp=20")), qPrintable(text));
    QVERIFY2(text.contains(QStringLiteral("Dhze=7")), qPrintable(text));

    const int base = view->history_count();
    QVERIFY(view->apply_pictura_raw_filter(path, text));
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Pictura Raw"));
    QVERIFY(view->layer_pictura_raw_settings(path).contains(QStringLiteral("Dhze=7")));
}

void AcrDialogTest::cancelAppliesNothing()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    QString out = QStringLiteral("unchanged");
    QTimer::singleShot(0, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* dialog = qobject_cast<QDialog*>(widget)) {
                dialog->reject();
            }
        }
    });
    QVERIFY(!PicturaRawDialog::get(window_.get(), window_->activeView(), path, QString(), &out));
    QCOMPARE(out, QStringLiteral("unchanged"));
}

QTEST_MAIN(AcrDialogTest)
#include "tst_acr_dialog.moc"
