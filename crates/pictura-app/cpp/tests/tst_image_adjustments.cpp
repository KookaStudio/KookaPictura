// Image > Adjustments (#83): the menu is live on an opened image; a dialog
// previews on the canvas, Cancel restores the layer exactly, OK applies one
// state named for the adjustment; Invert / Desaturate / Equalize / Auto apply
// at once, inside the selection.

#include <QtTest/QtTest>

#include "adjustment_dialog.h"
#include "color_picker_dialog.h"
#include "curves_dialog.h"
#include "gradient_map_dialog.h"
#include "panels/percent_field.h"
#include "levels_dialog.h"
#include "panels/curve_widget.h"
#include "commands.h"
#include "frame.h"
#include "panels/ramp_slider.h"
#include "image_view.h"
#include "dialogs.h"
#include "hdr_toning_dialog.h"
#include "tools.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/gradient_map.cxxqt.h"

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QMenu>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

namespace {

QByteArray block(const char* kind)
{
    const ::rust::Vec<std::uint8_t> v =
        pictura::image_adjustment_default(QString::fromLatin1(kind), 0x000000u, 0xffffffu);
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

} // namespace

class ImageAdjustmentsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void menuIsLiveOnAnOpenedImage();
    void dialogPreviewsCancelsAndApplies();
    void colorLookupPresetRebuildsTheLook();
    void brightnessContrastPreviewsAndApplies();
    void levelsEditsEachChannelAndPresets();
    void exposureAndVibranceUseTheWideLayouts();
    void hueSaturationEditsEachRangeAndPresets();
    void colorBalanceEditsEachTone();
    void channelMixerColorsItsSourcesAndTotals();
    void canvasPansWhileADialogIsOpen();
    void blackWhiteMixesTintsAndPresets();
    void gradientMapEditsItsGradient();
    void gradientMapReopensWithItsState();
    void photoFilterUsesAFilterOrAColor();
    void photoFilterColorPickerSamplesTheImage();
    void curvesEditsEachChannelAndTheSelectedPoint();
    void hdrToningPresetsPopulateControls();
    void hdrToningRefusedApplyDoesNotAccept();
    void directCommandsRespectTheSelection();

private:
    // Open a 20x20 opaque image of `color` (it becomes the Background).
    bool openImage(const QColor& color);
    QAction* leaf(const QString& name) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
    pictura::PictureView* view_ = nullptr;
};

void ImageAdjustmentsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid() && dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ImageAdjustmentsTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool ImageAdjustmentsTest::openImage(const QColor& color)
{
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(color);
    const QString path = dir_.filePath(QStringLiteral("photo.png"));
    if (!image.save(path) || !window_->openDocumentAtPath(path)) {
        return false;
    }
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    window_->registry()->refresh();
    return view_ != nullptr;
}

QAction* ImageAdjustmentsTest::leaf(const QString& name) const
{
    return window_->registry()->action(
        pictura::commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), name}));
}

void ImageAdjustmentsTest::menuIsLiveOnAnOpenedImage()
{
    QAction* levels = leaf(QStringLiteral("Levels"));
    QVERIFY(levels);
    window_->registry()->refresh();
    QVERIFY(!levels->isEnabled());
    QVERIFY(openImage(QColor(100, 150, 200)));
    for (const char* name :
         {"Brightness/Contrast", "Levels", "Curves", "Exposure", "Vibrance", "Hue/Saturation",
          "Color Balance", "Black & White", "Photo Filter", "Channel Mixer", "Invert",
          "Posterize", "Threshold", "Gradient Map", "Selective Color", "Shadows/Highlights",
          "Color Lookup", "Desaturate", "Equalize",
          "HDR Toning", "Match Color"}) {
        QAction* action = leaf(QString::fromUtf8(name));
        QVERIFY2(action && action->isEnabled(), name);
    }
    QCOMPARE(levels->text(), QStringLiteral("Levels…"));
    QCOMPARE(levels->shortcut(), QKeySequence(QStringLiteral("Ctrl+L")));
    QCOMPARE(leaf(QStringLiteral("Invert"))->text(), QStringLiteral("Invert"));
    QCOMPARE(leaf(QStringLiteral("Match Color"))->text(), QStringLiteral("Match Color…"));
    QAction* autoTone = window_->registry()->action(
        pictura::commandIdForPath({QStringLiteral("Image"), QStringLiteral("Auto Tone")}));
    QVERIFY(autoTone && autoTone->isEnabled());
    // The Auto commands live only at the top of the Image menu; Adjustments
    // splits into four sections.
    QVERIFY(!leaf(QStringLiteral("Auto Tone")));
    QMenu* adjustments = levels->associatedObjects().isEmpty()
                             ? nullptr
                             : qobject_cast<QMenu*>(levels->associatedObjects().first());
    QVERIFY(adjustments);
    QStringList sections;
    QString section;
    for (QAction* action : adjustments->actions()) {
        if (action->isSeparator()) {
            sections << section;
            section.clear();
        } else {
            section = action->text();
        }
    }
    sections << section;
    QCOMPARE(sections, QStringList({QStringLiteral("Color Lookup…"),
                                    QStringLiteral("Selective Color…"),
                                    QStringLiteral("HDR Toning…"),
                                    QStringLiteral("Equalize")}));
}

void ImageAdjustmentsTest::dialogPreviewsCancelsAndApplies()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();
    {
        pictura::AdjustmentDialog dialog(view_, block("hue-saturation"), QRect());
        auto* lightness =
            qobject_cast<QSlider*>(dialog.controlForTest(QStringLiteral("lightness")));
        QVERIFY(lightness);
        lightness->setValue(-100);
        QCOMPARE(qGray(view_->composite_argb(5, 5)), 0);
        dialog.reject();
    }
    QCOMPARE(view_->composite_argb(5, 5), before);
    QCOMPARE(view_->history_count(), history);

    pictura::AdjustmentDialog dialog(view_, block("hue-saturation"), QRect());
    qobject_cast<QSlider*>(dialog.controlForTest(QStringLiteral("lightness")))->setValue(-100);
    dialog.accept();
    QCOMPARE(qGray(view_->composite_argb(5, 5)), 0);
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Hue/Saturation"));
    QVERIFY(view_->undo());
    QCOMPARE(view_->composite_argb(5, 5), before);
}

void ImageAdjustmentsTest::colorLookupPresetRebuildsTheLook()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    const QRgb before = view_->composite_argb(5, 5);
    {
        pictura::AdjustmentDialog dialog(view_, block("color-lookup"), QRect());
        auto* preset = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("preset")));
        QVERIFY(preset);
        QCOMPARE(preset->count(), 7);
        preset->setCurrentIndex(1); // Warm Contrast, not the identity
        QVERIFY(view_->composite_argb(5, 5) != before);
        dialog.reject();
    }
    QCOMPARE(view_->composite_argb(5, 5), before);
}

void ImageAdjustmentsTest::brightnessContrastPreviewsAndApplies()
{
    QVERIFY(openImage(QColor(100, 100, 100)));
    const int history = view_->history_count();
    const auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("brightness-contrast"), view_,
                                                      block("brightness-contrast"), QRect());
    auto* brightness = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("brightness")));
    auto* slider = dialog->findChild<QSlider*>(QStringLiteral("brightnessSlider"));
    QVERIFY(brightness && slider);
    QCOMPARE(brightness->minimum(), -150);
    QVERIFY(dialog->minimumWidth() >= 420);
    slider->setValue(100);
    QCOMPARE(brightness->value(), 100);
    QVERIFY(qRed(view_->composite_argb(5, 5)) > 100);
    auto* autoButton = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
    QVERIFY(autoButton && !autoButton->isEnabled());
    dialog->accept();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Brightness/Contrast"));
}

// Exposure: Preset, then label / slider / field / eyedropper rows, at least
// 500 px wide; a preset sets the exposure and drops back to Custom on an edit.
// Vibrance is the stacked Brightness/Contrast layout.
void ImageAdjustmentsTest::exposureAndVibranceUseTheWideLayouts()
{
    QVERIFY(openImage(QColor(100, 100, 100)));
    const QRgb before = view_->composite_argb(5, 5);
    {
        const auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("exposure"), view_,
                                                          block("exposure"), QRect());
        auto* preset = qobject_cast<QComboBox*>(dialog->controlForTest(QStringLiteral("preset")));
        auto* exposure =
            qobject_cast<QDoubleSpinBox*>(dialog->controlForTest(QStringLiteral("exposure")));
        auto* offset = qobject_cast<QDoubleSpinBox*>(dialog->controlForTest(QStringLiteral("offset")));
        QVERIFY(preset && exposure && offset);
        QVERIFY(dialog->minimumWidth() >= 500);
        QCOMPARE(offset->decimals(), 4);
        preset->setCurrentIndex(preset->findText(QStringLiteral("Plus 1.0")));
        emit preset->activated(preset->currentIndex());
        QCOMPARE(exposure->value(), 1.0);
        QVERIFY(qRed(view_->composite_argb(5, 5)) > qRed(before));
        // Gamma runs 9.99 at the left to 0.01 at the right, 1.00 centred.
        auto* gammaSlider = dialog->findChild<QSlider*>(QStringLiteral("gammaSlider"));
        auto* gamma = qobject_cast<QDoubleSpinBox*>(dialog->controlForTest(QStringLiteral("gamma")));
        QVERIFY(gammaSlider && gamma);
        QCOMPARE(gammaSlider->value(), (gammaSlider->minimum() + gammaSlider->maximum()) / 2);
        gammaSlider->setValue(gammaSlider->minimum());
        QCOMPARE(gamma->value(), 9.99);
        QCOMPARE(preset->currentText(), QStringLiteral("Custom"));
        gammaSlider->setValue(gammaSlider->maximum());
        QCOMPARE(gamma->value(), 0.01);
        gamma->setValue(1.0);
        QCOMPARE(gammaSlider->value(), (gammaSlider->minimum() + gammaSlider->maximum()) / 2);
        dialog->reject();
    }
    QCOMPARE(view_->composite_argb(5, 5), before);

    const auto vibrance = pictura::makeAdjustmentDialog(QStringLiteral("vibrance"), view_,
                                                        block("vibrance"), QRect());
    auto* saturation = qobject_cast<QSpinBox*>(vibrance->controlForTest(QStringLiteral("saturation")));
    QVERIFY(saturation && vibrance->findChild<QSlider*>(QStringLiteral("vibranceSlider")));
    QVERIFY(vibrance->minimumWidth() >= 420);
    vibrance->reject();
}

// Hue/Saturation: a colour range edits its own record (Reds lightness -100
// blacks a red pixel and leaves a blue one), Master keeps its values, and a
// preset clears every range.
void ImageAdjustmentsTest::hueSaturationEditsEachRangeAndPresets()
{
    QVERIFY(openImage(QColor(200, 40, 40)));
    const QRgb before = view_->composite_argb(5, 5);
    const auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("hue-saturation"), view_,
                                                      block("hue-saturation"), QRect());
    auto* preset = qobject_cast<QComboBox*>(dialog->controlForTest(QStringLiteral("preset")));
    auto* range = qobject_cast<QComboBox*>(dialog->controlForTest(QStringLiteral("range")));
    auto* lightness = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("lightness")));
    auto* colorize = qobject_cast<QCheckBox*>(
        dialog->controlForTest(QStringLiteral("hueSaturationColorize")));
    QVERIFY(preset && range && lightness && colorize);
    QVERIFY(dialog->minimumWidth() >= 500);
    QVERIFY(!colorize->isEnabled());
    QCOMPARE(range->count(), 7);

    range->setCurrentIndex(range->findText(QStringLiteral("Blues")));
    lightness->setValue(-100);
    QCOMPARE(view_->composite_argb(5, 5), before);
    range->setCurrentIndex(range->findText(QStringLiteral("Reds")));
    QCOMPARE(lightness->value(), 0);
    lightness->setValue(-100);
    QCOMPARE(qRed(view_->composite_argb(5, 5)), 0);
    QCOMPARE(preset->currentText(), QStringLiteral("Custom"));
    range->setCurrentIndex(0);
    QCOMPARE(lightness->value(), 0);

    preset->setCurrentIndex(preset->findText(QStringLiteral("Default")));
    emit preset->activated(preset->currentIndex());
    QCOMPARE(view_->composite_argb(5, 5), before);
    dialog->reject();
}

// Color Balance: each Tone radio edits its own triple; the dialog opens on
// Midtones with Preserve Luminosity on.
void ImageAdjustmentsTest::colorBalanceEditsEachTone()
{
    QVERIFY(openImage(QColor(100, 100, 100)));
    const QRgb before = view_->composite_argb(5, 5);
    const auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("color-balance"), view_,
                                                      block("color-balance"), QRect());
    auto* midtones = qobject_cast<QRadioButton*>(dialog->controlForTest(QStringLiteral("midtones")));
    auto* highlights =
        qobject_cast<QRadioButton*>(dialog->controlForTest(QStringLiteral("highlights")));
    auto* cyanRed = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("cyanRed")));
    auto* luminosity =
        qobject_cast<QCheckBox*>(dialog->controlForTest(QStringLiteral("preserveLuminosity")));
    QVERIFY(midtones && highlights && cyanRed && luminosity);
    QVERIFY(midtones->isChecked() && luminosity->isChecked());
    luminosity->setChecked(false);
    cyanRed->setValue(100);
    QVERIFY(qRed(view_->composite_argb(5, 5)) > qRed(before));
    highlights->click();
    QCOMPARE(cyanRed->value(), 0);
    midtones->click();
    QCOMPARE(cyanRed->value(), 100);
    dialog->reject();
    QCOMPARE(view_->composite_argb(5, 5), before);
}

// Channel Mixer: each output channel edits its own sources over colour ramps,
// with their Total beside them.
void ImageAdjustmentsTest::channelMixerColorsItsSourcesAndTotals()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    const auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("channel-mixer"), view_,
                                                      block("channel-mixer"), QRect());
    auto* output = qobject_cast<QComboBox*>(dialog->controlForTest(QStringLiteral("outputChannel")));
    auto* red = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("red")));
    auto* green = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("green")));
    auto* total = qobject_cast<QLabel*>(dialog->controlForTest(QStringLiteral("channelMixerTotal")));
    QVERIFY(output && red && green && total);
    QCOMPARE(output->count(), 3);
    QVERIFY(qobject_cast<pictura::RampSlider*>(dialog->controlForTest(QStringLiteral("redSlider"))));
    QCOMPARE(red->value(), 100);
    QCOMPARE(total->text(), QStringLiteral("+100 %"));
    green->setValue(-15);
    QCOMPARE(total->text(), QStringLiteral("+85 %"));
    QVERIFY(qRed(view_->composite_argb(5, 5)) < 100);
    output->setCurrentIndex(1);
    QCOMPARE(red->value(), 0);
    QCOMPARE(green->value(), 100);
    dialog->reject();
}

// While a dialog runs, the canvas still pans on a middle-button drag; every
// other click on the main window stays blocked.
void ImageAdjustmentsTest::canvasPansWhileADialogIsOpen()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    window_->resize(900, 700);
    window_->show();
    auto* canvas = window_->findChild<pictura::ImageView*>();
    QVERIFY(canvas && QTest::qWaitForWindowExposed(window_.get()));
    canvas->setZoom(32.0, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    const QPointF before = canvas->offset();
    QSignalSpy clicks(canvas, &pictura::ImageView::mousePressed);
    QDialog dialog(window_.get());
    QPointF during;
    // sendEvent runs the application event filters, as real input does; QTest's
    // synthetic move would not carry the held middle button.
    const auto send = [canvas](QEvent::Type type, QPointF at, Qt::MouseButton button,
                               Qt::MouseButtons held) {
        QMouseEvent event(type, at, canvas->mapToGlobal(at), button, held, Qt::NoModifier);
        QApplication::sendEvent(canvas, &event);
    };
    QTimer::singleShot(0, &dialog, [&]() {
        const QPointF at(canvas->width() / 2.0, canvas->height() / 2.0);
        const QPointF to = at + QPointF(120, 120);
        send(QEvent::MouseButtonPress, at, Qt::MiddleButton, Qt::MiddleButton);
        send(QEvent::MouseMove, to, Qt::NoButton, Qt::MiddleButton);
        send(QEvent::MouseButtonRelease, to, Qt::MiddleButton, Qt::NoButton);
        send(QEvent::MouseButtonPress, at, Qt::LeftButton, Qt::LeftButton);
        send(QEvent::MouseButtonRelease, at, Qt::LeftButton, Qt::NoButton);
        during = canvas->offset();
        dialog.reject();
    });
    pictura::runDialog(dialog, window_.get());
    QVERIFY2(during != before, "the middle drag panned the canvas");
    QCOMPARE(clicks.count(), 0);
    window_->hide();
}

// Black & White: a colour's percentage changes its gray and marks the preset
// Custom, a preset sets the six, and Tint colours the result with the swatch,
// whose Hue / Saturation rows are live only with Tint on.
void ImageAdjustmentsTest::blackWhiteMixesTintsAndPresets()
{
    QVERIFY(openImage(QColor(200, 40, 40)));
    const auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("black-white"), view_,
                                                      block("black-white"), QRect());
    auto* reds = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("reds")));
    auto* tint = qobject_cast<QCheckBox*>(dialog->controlForTest(QStringLiteral("tint")));
    auto* preset = qobject_cast<QComboBox*>(dialog->controlForTest(QStringLiteral("preset")));
    auto* hue = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("tintHue")));
    auto* saturation =
        qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("tintSaturation")));
    QVERIFY(reds && tint && preset && hue && saturation);
    for (const char* name :
         {"Default", "Blue Filter", "Darker", "Green Filter", "High Contrast Blue Filter",
          "High Contrast Red Filter", "Infrared", "Lighter", "Maximum Black", "Maximum White",
          "Neutral Density", "Red Filter", "Yellow Filter", "Custom"}) {
        QVERIFY2(preset->findText(QString::fromLatin1(name)) >= 0, name);
    }
    QCOMPARE(preset->currentText(), QStringLiteral("Default"));
    QCOMPARE(reds->value(), 40);
    QRgb gray = view_->composite_argb(5, 5);
    QCOMPARE(qRed(gray), qBlue(gray));
    reds->setValue(-200);
    QCOMPARE(preset->currentText(), QStringLiteral("Custom"));
    QVERIFY(qRed(view_->composite_argb(5, 5)) < qRed(gray));
    preset->setCurrentIndex(preset->findText(QStringLiteral("Red Filter")));
    emit preset->activated(preset->currentIndex());
    QCOMPARE(reds->value(), 300);
    QVERIFY(qRed(view_->composite_argb(5, 5)) > qRed(gray));
    preset->setCurrentIndex(preset->findText(QStringLiteral("Default")));
    emit preset->activated(preset->currentIndex());
    QCOMPARE(reds->value(), 40);
    QCOMPARE(view_->composite_argb(5, 5), gray);

    QVERIFY(!hue->isEnabled() && !saturation->isEnabled());
    tint->setChecked(true);
    QVERIFY(hue->isEnabled() && saturation->isEnabled());
    QCOMPARE(saturation->value(), 20);
    const QRgb tinted = view_->composite_argb(5, 5);
    QVERIFY(qRed(tinted) > qBlue(tinted));
    hue->setValue(220);
    saturation->setValue(60);
    const QRgb blue = view_->composite_argb(5, 5);
    QVERIFY(qBlue(blue) > qRed(blue));
    const QColor swatch =
        dialog->controlForTest(QStringLiteral("tintColor"))->property("color").value<QColor>();
    QCOMPARE(swatch.hsvHue(), 220);
    QCOMPARE(qRound(swatch.hsvSaturation() * 100 / 255.0), 60);
    dialog->reject();
}

// Gradient Map: the dialog opens on foreground to background, a preset from
// the picker and Reverse remap the gray, and the Gradient Editor's edits
// (presets, stops, Smoothness, Noise) preview as they are made, Cancel
// restoring the gradient it opened on.
void ImageAdjustmentsTest::gradientMapEditsItsGradient()
{
    QVERIFY(openImage(QColor(0, 0, 0)));
    const int history = view_->history_count();
    auto made = pictura::makeAdjustmentDialog(QStringLiteral("gradient-map"), view_,
                                              block("gradient-map"), QRect());
    auto* dialog = qobject_cast<pictura::GradientMapDialog*>(made.get());
    QVERIFY(dialog);
    auto* presets = qobject_cast<QToolButton*>(dialog->controlForTest(QStringLiteral("presets")));
    auto* reverse = qobject_cast<QCheckBox*>(dialog->controlForTest(QStringLiteral("reverse")));
    QVERIFY(presets && presets->menu() && reverse && dialog->controlForTest(QStringLiteral("dither"))
            && dialog->controlForTest(QStringLiteral("sample")));
    QCOMPARE(dialog->spec().stops.size(), 2);
    QCOMPARE(dialog->spec().stops.first().color, QColor(Qt::black));
    QCOMPARE(view_->composite_argb(5, 5), qRgb(0, 0, 0));

    QAction* redGreen = nullptr;
    for (QAction* action : presets->menu()->actions()) {
        if (action->text() == QLatin1String("Red, Green")) {
            redGreen = action;
        }
    }
    QVERIFY(redGreen);
    redGreen->trigger();
    QCOMPARE(view_->composite_argb(5, 5), qRgb(255, 0, 0));
    reverse->setChecked(true);
    QCOMPARE(view_->composite_argb(5, 5), qRgb(0, 255, 0));
    reverse->setChecked(false);

    // The editor's edits preview at once; Cancel puts Red, Green back.
    QRgb previewed = 0;
    QRgb edited = 0;
    dialog->openEditor([&](pictura::GradientEditorDialog& editor) {
        QTimer::singleShot(0, &editor, [&]() {
            editor.choosePreset(5); // Blue, Red, Yellow
            previewed = view_->composite_argb(5, 5);
            pictura::GradientStopBar* bar = editor.bar();
            const int added = bar->addStop(2048);
            QCOMPARE(bar->stops().size(), 4);
            QCOMPARE(editor.spec().name, QStringLiteral("Custom"));
            bar->removeStop(added);
            bar->setStopColor(0, QColor(0, 255, 0));
            edited = view_->composite_argb(5, 5);
            editor.smoothnessField()->setValue(0);
            emit editor.smoothnessField()->valueChanged(0);
            QCOMPARE(editor.spec().smoothness, 0);
            editor.reject();
        });
    });
    QCOMPARE(previewed, qRgb(0, 0, 255));
    QCOMPARE(edited, qRgb(0, 255, 0));
    QCOMPARE(view_->composite_argb(5, 5), qRgb(255, 0, 0));
    QCOMPARE(dialog->spec().name, QStringLiteral("Red, Green"));

    // Noise: its own page, previewed, kept on OK; Randomize makes another.
    dialog->openEditor([&](pictura::GradientEditorDialog& editor) {
        QTimer::singleShot(0, &editor, [&]() {
            editor.typeCombo()->setCurrentIndex(pictura::GradientSpec::Noise);
            QVERIFY(editor.noisePage()->isVisibleTo(&editor));
            const quint32 seed = editor.spec().noise.seed;
            editor.noisePage()->randomize();
            QVERIFY(editor.spec().noise.seed != seed);
            editor.accept();
        });
    });
    QCOMPARE(dialog->spec().type, int(pictura::GradientSpec::Noise));
    const QColor first = pictura::resolvedStops(dialog->spec()).first().color;
    QCOMPARE(view_->composite_argb(5, 5), first.rgb());

    dialog->accept();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Gradient Map"));
    QCOMPARE(view_->composite_argb(5, 5), first.rgb());
}

// Gradient Map: reopening a block keeps its opacity, Reverse, and Dither, so
// a rebuild (toggling Reverse) does not drop them; Dither is disabled.
void ImageAdjustmentsTest::gradientMapReopensWithItsState()
{
    QVERIFY(openImage(QColor(0, 0, 0)));
    const ::rust::Vec<std::uint8_t> made = pictura::gradient_map_block(
        QStringLiteral("0:ff000080 4096:0000ff"), 0, true, true);
    const QByteArray saved(reinterpret_cast<const char*>(made.data()), qsizetype(made.size()));
    auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("gradient-map"), view_, saved,
                                                QRect());
    auto* map = qobject_cast<pictura::GradientMapDialog*>(dialog.get());
    QVERIFY(map);
    auto* reverse = qobject_cast<QCheckBox*>(map->controlForTest(QStringLiteral("reverse")));
    auto* dither = qobject_cast<QCheckBox*>(map->controlForTest(QStringLiteral("dither")));
    QVERIFY(reverse && dither);
    QVERIFY(reverse->isChecked());
    QVERIFY(dither->isChecked());
    QVERIFY(!dither->isEnabled());
    QCOMPARE(map->spec().stops.first().color.alpha(), 128);

    reverse->setChecked(false);
    reverse->setChecked(true);
    QCOMPARE(map->block(), saved);
    map->reject();
}

// Photo Filter: the default is the Warming Filter (85) at 25%; a cooling
// filter cools, and a colour the filters do not name reopens as Color.
void ImageAdjustmentsTest::photoFilterUsesAFilterOrAColor()
{
    QVERIFY(openImage(QColor(128, 128, 128)));
    const QRgb before = view_->composite_argb(5, 5);
    const auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("photo-filter"), view_,
                                                      block("photo-filter"), QRect());
    auto* filter = qobject_cast<QComboBox*>(dialog->controlForTest(QStringLiteral("filter")));
    auto* useFilter = qobject_cast<QRadioButton*>(dialog->controlForTest(QStringLiteral("useFilter")));
    auto* density = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("density")));
    QVERIFY(filter && useFilter && density);
    QVERIFY(dialog->minimumWidth() >= 440);
    QCOMPARE(filter->currentText(), QStringLiteral("Warming Filter (85)"));
    QVERIFY(useFilter->isChecked());
    QCOMPARE(density->value(), 25);
    filter->setCurrentIndex(filter->findText(QStringLiteral("Cooling Filter (80)")));
    density->setValue(80);
    const QRgb cooled = view_->composite_argb(5, 5);
    QVERIFY(qBlue(cooled) > qRed(cooled));
    dialog->reject();
    QCOMPARE(view_->composite_argb(5, 5), before);

    const QByteArray custom = QByteArray(block("photo-filter"));
    const ::rust::Vec<std::uint8_t> edited = pictura::image_adjustment_set(
        {reinterpret_cast<const std::uint8_t*>(custom.constData()), std::size_t(custom.size())},
        QStringLiteral("color"), double(0x123456));
    const QByteArray reopened(reinterpret_cast<const char*>(edited.data()), qsizetype(edited.size()));
    const auto again =
        pictura::makeAdjustmentDialog(QStringLiteral("photo-filter"), view_, reopened, QRect());
    QVERIFY(qobject_cast<QRadioButton*>(again->controlForTest(QStringLiteral("useColor")))
                ->isChecked());
    again->reject();
}

// Photo Filter's colour picker samples the image as CS6's does (#413): while
// it is open, a canvas click reaches the canvas past both dialogs' input
// blockers and takes the composite colour instead of running the active tool.
void ImageAdjustmentsTest::photoFilterColorPickerSamplesTheImage()
{
    QVERIFY(openImage(QColor(10, 200, 30)));
    window_->resize(900, 700);
    window_->show();
    auto* canvas = window_->imageView();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(canvas && tools && QTest::qWaitForWindowExposed(window_.get()));
    const int history = view_->history_count();
    auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("photo-filter"), view_,
                                                block("photo-filter"), QRect(), window_.get());
    dialog->setCanvasSampling(tools);
    auto* swatch = qobject_cast<QPushButton*>(dialog->controlForTest(QStringLiteral("color")));
    auto* useColor = qobject_cast<QRadioButton*>(dialog->controlForTest(QStringLiteral("useColor")));
    QVERIFY(swatch && useColor);

    bool armed = false;
    QColor sampled;
    const auto send = [canvas](QEvent::Type type, QPointF at, Qt::MouseButtons held) {
        QMouseEvent event(type, at, canvas->mapToGlobal(at), Qt::LeftButton, held,
                          Qt::NoModifier);
        QApplication::sendEvent(canvas, &event);
    };
    QTimer::singleShot(0, dialog.get(), [&]() {
        QTimer::singleShot(0, dialog.get(), [&]() {
            auto* picker = dialog->findChild<pictura::ColorPickerDialog*>();
            QVERIFY(picker);
            armed = tools->canvasSamplerActive();
            const QPointF at = canvas->imageToWidget(QPointF(5, 5));
            send(QEvent::MouseButtonPress, at, Qt::LeftButton);
            send(QEvent::MouseButtonRelease, at, Qt::NoButton);
            sampled = picker->selectedColor();
            picker->accept();
        });
        swatch->click();
        dialog->reject();
    });
    pictura::runDialog(*dialog, window_.get());

    QVERIFY(armed);
    QCOMPARE(sampled, QColor(10, 200, 30));
    QCOMPARE(swatch->property("color").value<QColor>(), QColor(10, 200, 30));
    QVERIFY(useColor->isChecked());
    QVERIFY(!tools->canvasSamplerActive());
    QCOMPARE(view_->history_count(), history);
    window_->hide();
}

// Levels: a channel edits only its own record (red's output white at 0 zeroes
// red alone), black is held below white, a preset restores every channel, and
// Auto stretches the channel's occupied range.
void ImageAdjustmentsTest::levelsEditsEachChannelAndPresets()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    const QRgb before = view_->composite_argb(5, 5);
    pictura::LevelsDialog dialog(view_, block("levels"), QRect());
    auto* channel = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("levelsChannel")));
    auto* preset = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("levelsPreset")));
    auto* inBlack = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("inputBlack")));
    auto* inWhite = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("inputWhite")));
    auto* outWhite = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("outputWhite")));
    QVERIFY(channel && preset && inBlack && inWhite && outWhite);
    QCOMPARE(preset->currentText(), QStringLiteral("Default"));

    channel->setCurrentIndex(1);
    outWhite->setValue(0);
    QRgb now = view_->composite_argb(5, 5);
    QCOMPARE(qRed(now), 0);
    QCOMPARE(qGreen(now), qGreen(before));
    QCOMPARE(qBlue(now), qBlue(before));
    QCOMPARE(preset->currentText(), QStringLiteral("Custom"));
    channel->setCurrentIndex(0);
    QCOMPARE(outWhite->value(), 255);

    inWhite->setValue(120);
    QCOMPARE(inBlack->maximum(), 118);

    preset->setCurrentIndex(preset->findText(QStringLiteral("Default")));
    emit preset->activated(preset->currentIndex());
    QCOMPARE(view_->composite_argb(5, 5), before);
    QCOMPARE(inWhite->value(), 255);

    // A flat image: Auto pins black and white around its one green level.
    channel->setCurrentIndex(2);
    dialog.findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"))->click();
    QVERIFY(inBlack->value() <= 150 && inWhite->value() >= 150);
    QVERIFY(inWhite->value() - inBlack->value() >= 2);
    dialog.reject();
    QCOMPARE(view_->composite_argb(5, 5), before);
}

// Curves: a channel's curve moves only that channel; the Input / Output fields
// follow and move the selected point; OK records one "Curves" state.
void ImageAdjustmentsTest::curvesEditsEachChannelAndTheSelectedPoint()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();
    pictura::CurvesDialog dialog(view_, block("curves"), QRect());
    auto* channel = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("curvesChannel")));
    auto* curve = qobject_cast<pictura::CurveWidget*>(dialog.controlForTest(QStringLiteral("curve")));
    auto* input = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("curvesInput")));
    auto* output = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("curvesOutput")));
    QVERIFY(channel && curve && input && output);
    QVERIFY(!input->isEnabled());

    channel->setCurrentIndex(3);
    curve->setPoints({QPointF(0.0, 0.0), QPointF(200 / 255.0, 100 / 255.0), QPointF(1.0, 1.0)});
    QRgb now = view_->composite_argb(5, 5);
    QVERIFY(std::abs(qBlue(now) - 100) <= 1);
    QCOMPARE(qRed(now), qRed(before));
    QCOMPARE(qGreen(now), qGreen(before));

    // A click on the face adds a point and selects it; the fields show it.
    const QPointF at(curve->width() / 2, curve->height() / 2);
    QTest::mouseClick(curve, Qt::LeftButton, Qt::NoModifier, at.toPoint());
    QVERIFY(curve->selected() >= 0 && input->isEnabled());
    const int selected = curve->selected();
    input->setValue(input->value());
    output->setValue(60);
    QCOMPARE(qRound(curve->points().at(selected).y() * 255.0), 60);

    // Delete removes a selected interior point but never an endpoint.
    const int points = curve->points().size();
    QTest::keyClick(curve, Qt::Key_Delete);
    QCOMPARE(curve->points().size(), points - 1);

    dialog.accept();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Curves"));
}

void ImageAdjustmentsTest::hdrToningPresetsPopulateControls()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    pictura::HdrToningDialog dialog(view_, QRect());
    auto* preset = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("hdrPreset")));
    auto* radius = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("hdrRadius")));
    auto* strength = qobject_cast<QDoubleSpinBox*>(dialog.controlForTest(QStringLiteral("hdrStrength")));
    auto* detail = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("hdrDetail")));
    auto* saturation = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("hdrSaturation")));
    QVERIFY(preset && radius && strength && detail && saturation);
    // The 17 presets plus Custom, opening on the control defaults (preset 0).
    QCOMPARE(dialog.presetCount(), 18);
    QCOMPARE(preset->currentText(), QStringLiteral("Default"));

    preset->setCurrentIndex(1); // City Twilight
    QCOMPARE(radius->value(), 383);
    QCOMPARE(strength->value(), 1.14);
    QCOMPARE(saturation->value(), -3);

    // Editing a value drops the preset back to Custom.
    detail->setValue(7);
    QCOMPARE(preset->currentText(), QStringLiteral("Custom"));
    dialog.reject();
}

void ImageAdjustmentsTest::hdrToningRefusedApplyDoesNotAccept()
{
    // A refused apply (here: no target view) must reject, so `get()` reports the
    // failure and the frame does not refresh as if the toning landed.
    pictura::HdrToningDialog dialog(nullptr, QRect());
    dialog.accept();
    QCOMPARE(dialog.result(), int(QDialog::Rejected));
}

void ImageAdjustmentsTest::directCommandsRespectTheSelection()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    QVERIFY(view_->select_rect(0, 0, 10, 20, QStringLiteral("new"), 0.0));
    leaf(QStringLiteral("Invert"))->trigger();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Invert"));
    QCOMPARE(QColor(view_->composite_argb(5, 5)), QColor(155, 105, 55));
    QCOMPARE(QColor(view_->composite_argb(15, 5)), QColor(100, 150, 200));

    view_->deselect();
    leaf(QStringLiteral("Desaturate"))->trigger();
    const QColor grey(view_->composite_argb(15, 5));
    QVERIFY(grey.red() == grey.green() && grey.green() == grey.blue());
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Desaturate"));
    leaf(QStringLiteral("Equalize"))->trigger();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Equalize"));
    // Two greys spread to the ends of the range.
    QCOMPARE(qGray(view_->composite_argb(15, 5)), 255);
}

QTEST_MAIN(ImageAdjustmentsTest)
#include "tst_image_adjustments.moc"
