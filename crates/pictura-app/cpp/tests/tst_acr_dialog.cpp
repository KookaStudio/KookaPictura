#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtCore/QPointF>
#include <QtCore/QSize>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSizeGrip>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>

#include "acr/acr_controls.h"
#include "acr/acr_histogram.h"
#include "acr/acr_panels.h"
#include "acr/acr_preview_pane.h"
#include "acr/acr_settings.h"
#include "frame.h"
#include "panels/curve_widget.h"
#include "pictura_raw_dialog.h"
#include "theme.h"
#include "api/view_bridge.h"

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
    void splitToningSaturationRampsFollowHue();
    void splitToningHighlightsChangeThePreview();
    void lensPanelWritesKeys();
    void effectsPanelWritesKeysAndStyleRule();
    void calibrationPanelWritesKeys();
    void previewPaneAndHistogram();
    void clippingTintsOutOfRange();
    void applyAcrossTabsRecordsOneState();
    void cancelAppliesNothing();
    void wheelStepsPreviewZoom();
    void tabStripIsEvenIconStrip();
    void controlsClearPreview();
    void panelContentIsInset();
    void hslPanelColorsMatchRanges();
    void effectsTabUsesFxIcon();
    void bottomControlsShareABaseline();
    void sizeGripSitsBelowOk();
    void photoInfoForDocument();
    void pickerSamplesHoveredPixel();
    void pickerRestoresAverage();

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
    QCOMPARE(tabs->count(), 8);
    QCOMPARE(tabs->currentIndex(), 0);
    QVERIFY2(tabs->tabText(0).isEmpty(), "tabs are icon-only");
    QCOMPARE(tabs->tabToolTip(0), QStringLiteral("Basic"));
    QCOMPARE(tabs->tabToolTip(6), QStringLiteral("Effects"));
    QCOMPARE(tabs->tabToolTip(7), QStringLiteral("Camera Calibration"));
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

void AcrDialogTest::splitToningSaturationRampsFollowHue()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrSplitToningPanel panel(&settings);
    auto* highlightHue = panel.findChild<AcrSlider*>(QString::fromLatin1(acr::ToningHighlightHue));
    auto* highlightSat = panel.findChild<AcrSlider*>(QString::fromLatin1(acr::ToningHighlightSat));
    auto* shadowHue = panel.findChild<AcrSlider*>(QString::fromLatin1(acr::ToningShadowHue));
    auto* shadowSat = panel.findChild<AcrSlider*>(QString::fromLatin1(acr::ToningShadowSat));
    QVERIFY(highlightHue && highlightSat && shadowHue && shadowSat);
    QCOMPARE(highlightSat->rampForTest().first(), Theme::panelColor().darker(135));
    QVERIFY(setSlider(&panel, acr::ToningHighlightHue, 167));
    QCOMPARE(highlightSat->rampForTest().last(), QColor::fromHsv(167, 255, 255));
    QVERIFY(setSlider(&panel, acr::ToningShadowHue, 291));
    QCOMPARE(shadowSat->rampForTest().last(), QColor::fromHsv(291, 255, 255));
    QCOMPARE(highlightHue->rampForTest().size(), 13);
}

void AcrDialogTest::splitToningHighlightsChangeThePreview()
{
    QVERIFY(window_->newDocument(QStringLiteral("AcrSplitToning"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    PictureView* view = window_->activeView();
    QVERIFY(view);
    const QString path = view->add_solid_fill(0xffa0a0a0u);
    QVERIFY(!path.isEmpty());
    QVERIFY(view->rasterize_fill_content(path));

    PicturaRawDialog dialog(view, path, QString(), nullptr);
    const QImage initial = dialog.previewPaneForTest()->image();
    QVERIFY(!initial.isNull());
    QVERIFY(setSlider(&dialog, acr::ToningHighlightHue, 230));
    QVERIFY(setSlider(&dialog, acr::ToningHighlightSat, 52));
    QTRY_VERIFY(dialog.previewPaneForTest()->image() != initial);

    const QColor before = initial.pixelColor(initial.width() / 2, initial.height() / 2);
    const QImage tinted = dialog.previewPaneForTest()->image();
    const QColor after = tinted.pixelColor(tinted.width() / 2, tinted.height() / 2);
    QVERIFY2(after.blue() >= before.blue() + 20 && after.red() < before.red(),
             qPrintable(QStringLiteral("%1,%2,%3 -> %4,%5,%6")
                            .arg(before.red())
                            .arg(before.green())
                            .arg(before.blue())
                            .arg(after.red())
                            .arg(after.green())
                            .arg(after.blue())));
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

void AcrDialogTest::previewPaneAndHistogram()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    AcrPreviewPane* pane = dialog.previewPaneForTest();
    AcrHistogram* hist = dialog.histogramForTest();
    QCOMPARE(hist->seriesCountForTest(), 3);

    const QImage initial = pane->image();
    QVERIFY(setSlider(&dialog, acr::Exposure, 1.5));
    QTRY_VERIFY(pane->image() != initial);
    const QImage bright = pane->image();
    QVERIFY(!bright.isNull());
    const QString brightReadout = hist->readoutForTest();
    QVERIFY2(brightReadout.contains(QStringLiteral("R:")), qPrintable(brightReadout));

    QVERIFY(setSlider(&dialog, acr::Exposure, -2.0));
    QTRY_VERIFY(pane->image() != bright);
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
    // A smart object retains the settings across apply and readback.
    QVERIFY(view->convert_to_smart_object(path));
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

void AcrDialogTest::wheelStepsPreviewZoom()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    AcrPreviewPane* pane = dialog.previewPaneForTest();
    QVERIFY(pane);
    QLineEdit* edit = pane->zoomEditForTest();
    QVERIFY(edit);
    QCOMPARE(edit->text(), QStringLiteral("Fit"));

    const auto wheel = [pane](int delta) {
        QWheelEvent event(QPointF(pane->width() / 2.0, pane->height() / 2.0),
                          pane->mapToGlobal(QPoint(pane->width() / 2, pane->height() / 2)),
                          QPoint(), QPoint(0, delta), Qt::NoButton, Qt::NoModifier,
                          Qt::NoScrollPhase, false);
        QApplication::sendEvent(pane, &event);
    };
    wheel(120);
    const QString first = edit->text();
    QVERIFY2(first != QStringLiteral("Fit"), qPrintable(first));
    wheel(120);
    QVERIFY2(edit->text() != first, qPrintable(edit->text()));
    wheel(-120);
    QCOMPARE(edit->text(), first);

    // The wheel uses the same ladder as the + button.
    PicturaRawDialog viaButtons(window_->activeView(), path, QString(), nullptr);
    auto* plus = viaButtons.findChild<QToolButton*>(QStringLiteral("acrZoomIn"));
    QVERIFY(plus);
    QTest::mouseClick(plus, Qt::LeftButton);
    QCOMPARE(viaButtons.previewPaneForTest()->zoomEditForTest()->text(), first);
}

void AcrDialogTest::tabStripIsEvenIconStrip()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    QTabBar* tabs = dialog.tabBarForTest();
    QVERIFY(tabs);
    QVERIFY(!tabs->drawBase());
    QVERIFY(tabs->expanding());
    QCOMPARE(tabs->iconSize(), QSize(16, 16));
    QCOMPARE(tabs->count(), 8);
}

void AcrDialogTest::controlsClearPreview()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    dialog.resize(1000, 640);
    dialog.show();
    QCoreApplication::processEvents();
    AcrPreviewPane* preview = dialog.previewPaneForTest();
    AcrHistogram* histogram = dialog.histogramForTest();
    QVERIFY(preview);
    QVERIFY(histogram);
    const int previewRight = preview->mapTo(&dialog, QPoint(preview->width(), 0)).x();
    const int controlsLeft = histogram->mapTo(&dialog, QPoint(0, 0)).x();
    QVERIFY2(controlsLeft - previewRight >= 12,
             qPrintable(QStringLiteral("%1 - %2").arg(controlsLeft).arg(previewRight)));
    dialog.hide();
}

void AcrDialogTest::panelContentIsInset()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    auto* basic = dialog.findChild<AcrBasicPanel*>();
    QVERIFY(basic);
    QVERIFY(basic->layout());
    QVERIFY2(basic->layout()->contentsMargins().left() >= 12,
             qPrintable(QString::number(basic->layout()->contentsMargins().left())));
    QVERIFY2(basic->layout()->contentsMargins().right() >= 8,
             qPrintable(QString::number(basic->layout()->contentsMargins().right())));
}

void AcrDialogTest::hslPanelColorsMatchRanges()
{
    AcrSettings settings = AcrSettings::decode(QString());
    AcrHslPanel panel(&settings);
    const auto middle = [&panel](const char* key) {
        auto* slider = panel.findChild<AcrSlider*>(QString::fromLatin1(key));
        if (!slider) {
            return QColor();
        }
        const QList<QColor> ramp = slider->rampForTest();
        return ramp.size() == 3 ? ramp.at(1) : QColor();
    };
    QCOMPARE(middle("HA_R"), QColor(255, 0, 0));
    QCOMPARE(middle("HA_O"), QColor(255, 128, 0));
    QCOMPARE(middle("HA_Y"), QColor(255, 255, 0));
    QCOMPARE(middle("HA_G"), QColor(0, 255, 0));
    QCOMPARE(middle("HA_A"), QColor(0, 255, 255));
    QCOMPARE(middle("HA_B"), QColor(0, 0, 255));
    QCOMPARE(middle("HA_P"), QColor(128, 0, 255));
    QCOMPARE(middle("HA_M"), QColor(255, 0, 255));
    QCOMPARE(middle("SA_R"), QColor(255, 0, 0));
    QCOMPARE(middle("LA_B"), QColor(0, 0, 255));
    auto* gray = panel.findChild<AcrSlider*>(QStringLiteral("GM_R"));
    QVERIFY(gray);
    QCOMPARE(gray->rampForTest(), (QList<QColor>{QColor(0x33, 0x33, 0x33), QColor(0xdd, 0xdd, 0xdd)}));
}

void AcrDialogTest::effectsTabUsesFxIcon()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    QTabBar* tabs = dialog.tabBarForTest();
    QVERIFY(tabs);
    QCOMPARE(tabs->count(), 8);
    QCOMPARE(tabs->tabToolTip(6), QStringLiteral("Effects"));
    QVERIFY(!tabs->tabIcon(6).isNull());
    QVERIFY(!dialog.findChild<QToolButton*>(QStringLiteral("acrSettingsButton")));
}

void AcrDialogTest::bottomControlsShareABaseline()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    dialog.resize(1000, 640);
    dialog.show();
    QCoreApplication::processEvents();

    auto* shadows = dialog.previewPaneForTest()->shadowToggleForTest();
    auto* box = dialog.findChild<QDialogButtonBox*>();
    QVERIFY(shadows);
    QVERIFY(box);
    auto* ok = box->button(QDialogButtonBox::Ok);
    QVERIFY(ok);
    const int shadowsBottom = shadows->mapTo(&dialog, QPoint(0, shadows->height())).y();
    const int okBottom = ok->mapTo(&dialog, QPoint(0, ok->height())).y();
    QCOMPARE(shadowsBottom, okBottom);
    dialog.hide();
}

void AcrDialogTest::sizeGripSitsBelowOk()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    dialog.resize(1000, 640);
    dialog.show();
    QCoreApplication::processEvents();

    auto* grip = dialog.findChild<QSizeGrip*>(QStringLiteral("rawSizeGrip"));
    auto* box = dialog.findChild<QDialogButtonBox*>();
    QVERIFY(grip);
    QVERIFY(box);
    auto* ok = box->button(QDialogButtonBox::Ok);
    QVERIFY(ok);
    QVERIFY(grip->isVisible());
    const int okBottom = ok->mapTo(&dialog, QPoint(0, ok->height())).y();
    const int gripTop = grip->mapTo(&dialog, QPoint(0, 0)).y();
    QVERIFY2(gripTop >= okBottom, qPrintable(QStringLiteral("%1 < %2").arg(gripTop).arg(okBottom)));
    dialog.hide();
}

void AcrDialogTest::photoInfoForDocument()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);

    auto* name = dialog.findChild<QLabel*>(QStringLiteral("rawPhotoName"));
    auto* info = dialog.findChild<QLabel*>(QStringLiteral("rawPhotoInfo"));
    QVERIFY(name);
    QVERIFY(info);
    QCOMPARE(name->text(), QStringLiteral("Untitled"));
    const QString text = info->text();
    QVERIFY2(text.contains(QStringLiteral("16 by 16")), qPrintable(text));
    QVERIFY2(text.contains(QStringLiteral("8 bits")), qPrintable(text));
    QVERIFY2(text.contains(QStringLiteral("sRGB")), qPrintable(text));
    QVERIFY2(text.contains(QStringLiteral("72 ppi")), qPrintable(text));

    auto* top = dialog.findChild<QLabel*>(QStringLiteral("acrExposureTop"));
    auto* bottom = dialog.findChild<QLabel*>(QStringLiteral("acrExposureBottom"));
    QVERIFY(top);
    QVERIFY(bottom);
    QVERIFY2(top->text().isEmpty(), qPrintable(top->text()));
    QVERIFY2(bottom->text().isEmpty(), qPrintable(bottom->text()));
}

void AcrDialogTest::pickerSamplesHoveredPixel()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    dialog.resize(900, 640);
    dialog.show();
    QCoreApplication::processEvents();

    AcrPreviewPane* pane = dialog.previewPaneForTest();
    QVERIFY(pane);
    QImage image(2, 2, QImage::Format_ARGB32);
    image.setPixelColor(0, 0, QColor(255, 0, 0));
    image.setPixelColor(1, 0, QColor(0, 255, 0));
    image.setPixelColor(0, 1, QColor(0, 0, 255));
    image.setPixelColor(1, 1, QColor(255, 255, 0));
    pane->setPreviewImageForTest(image);
    QCoreApplication::processEvents();

    const QRect area = pane->rect().adjusted(0, 0, 0, -28);
    const QSize scaled = image.size().scaled(area.size(), Qt::KeepAspectRatio);
    const QRect target(area.center().x() - scaled.width() / 2,
                       area.center().y() - scaled.height() / 2, scaled.width(), scaled.height());
    const QPoint topLeft = target.topLeft() + QPoint(target.width() / 4, target.height() / 4);
    const QPoint topRight = target.topLeft() + QPoint(target.width() * 3 / 4, target.height() / 4);

    AcrHistogram* histogram = dialog.histogramForTest();
    QVERIFY(histogram);
    const auto move = [pane](const QPoint& pos) {
        QMouseEvent event(QEvent::MouseMove, QPointF(pos), QPointF(pane->mapToGlobal(pos)),
                          Qt::NoButton, Qt::NoButton, Qt::NoModifier);
        QApplication::sendEvent(pane, &event);
    };
    QVERIFY(!pane->imageRectForTest().isEmpty());
    move(topLeft);
    QCOMPARE(histogram->readoutForTest(), QStringLiteral("R: 255  G: 0  B: 0"));
    move(topRight);
    QCOMPARE(histogram->readoutForTest(), QStringLiteral("R: 0  G: 255  B: 0"));
    dialog.hide();
}

void AcrDialogTest::pickerRestoresAverage()
{
    const QString path = makeRasterLayer();
    QVERIFY(!path.isEmpty());
    PicturaRawDialog dialog(window_->activeView(), path, QString(), nullptr);
    dialog.resize(900, 640);
    dialog.show();
    QCoreApplication::processEvents();

    AcrPreviewPane* pane = dialog.previewPaneForTest();
    AcrHistogram* histogram = dialog.histogramForTest();
    QVERIFY(pane);
    QVERIFY(histogram);
    const QString average = histogram->readoutForTest();

    QImage image(2, 2, QImage::Format_ARGB32);
    image.fill(QColor(255, 0, 0));
    pane->setPreviewImageForTest(image);
    QCoreApplication::processEvents();
    const QRect area = pane->rect().adjusted(0, 0, 0, -28);
    QMouseEvent move(QEvent::MouseMove, QPointF(area.center()),
                     QPointF(pane->mapToGlobal(area.center())), Qt::NoButton, Qt::NoButton,
                     Qt::NoModifier);
    QApplication::sendEvent(pane, &move);
    QCOMPARE(histogram->readoutForTest(), QStringLiteral("R: 255  G: 0  B: 0"));

    QEvent leave(QEvent::Leave);
    QApplication::sendEvent(pane, &leave);
    QCOMPARE(histogram->readoutForTest(), average);
    dialog.hide();
}

PICTURA_QT_TEST_MAIN(AcrDialogTest)
#include "tst_acr_dialog.moc"
