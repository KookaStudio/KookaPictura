#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QTabWidget>

#include "blur_gallery_canvas.h"
#include "blur_gallery_dialog.h"
#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kFieldBlurPath = {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"),
                                    QStringLiteral("Field Blur")};

const pictura::FilterCommandSpec& fieldBlurSpec()
{
    return *pictura::filterCommandForPath(kFieldBlurPath);
}

pictura::BlurGalleryDialog* openWorkspace()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::BlurGalleryDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

QImage pattern()
{
    QImage image(96, 64, QImage::Format_RGB32);
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 96; ++x) {
            image.setPixelColor(x, y, QColor((x * 4) % 256, (y * 4) % 256, (x * 9) % 256));
        }
    }
    return image;
}

// The effects tail: bokeh enabled 14/40, range 40..118; noise off, 0/50/50/0/0
// Grain, seed 1.
QList<double> defaultTail()
{
    return {1.0, 14.0, 40.0, 40.0, 118.0, 0.0, 0.0, 50.0, 50.0, 0.0, 0.0, 0.0, 1.0};
}

QList<double> defaultValues()
{
    QList<double> values = {1.0, 0.5, 0.5, 0.0};
    values += defaultTail();
    return values;
}

int pinCount(const QList<double>& values)
{
    return values.isEmpty() ? 0 : int(values.first());
}

void drag(QWidget* widget, const QPoint& from, const QPoint& to)
{
    QTest::mousePress(widget, Qt::LeftButton, Qt::NoModifier, from);
    QTest::mouseMove(widget, (from + to) / 2);
    QTest::mouseMove(widget, to);
    QTest::mouseRelease(widget, Qt::LeftButton, Qt::NoModifier, to);
}

} // namespace

class BlurGalleryDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void workspaceFollowsCs6();
    void pinListRoundTrips();
    void pinsAreAddedMovedAndRemoved();
    void blurAmountEditsTheSelectedPinAndPreviews();
    void effectsTabCarriesTheBokehControls();
    void noiseTabCarriesTheGrainControls();
    void hideUiTogglesThePinOverlay();
    void menuCommitsAndCancelRestores();
    void lastFilterReusesThePins();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QString patternPath_;
};

void BlurGalleryDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    patternPath_ = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern().save(patternPath_) && window_->openDocumentAtPath(patternPath_));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kFieldBlurPath) != nullptr);
}

void BlurGalleryDialogTest::workspaceFollowsCs6()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), fieldBlurSpec());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Field Blur"));
    QVERIFY(dialog.canvas() != nullptr);
    // CS6 opens on one centred pin with blur 0 and the effects tail's defaults.
    QCOMPARE(dialog.values(), defaultValues());

    auto* spin = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("fieldBlurSpin"));
    QVERIFY(spin != nullptr);
    QCOMPARE(spin->minimum(), 0.0);
    QCOMPARE(spin->maximum(), 500.0);
    auto* enabled = dialog.findChild<QCheckBox*>(QStringLiteral("fieldBlurEnabled"));
    QVERIFY(enabled != nullptr && enabled->isChecked() && enabled->isEnabled());
    auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"));
    QVERIFY(preview != nullptr && preview->isChecked());
    // Iris Blur, Tilt-Shift, Path Blur, and Spin Blur list as disabled rows.
    for (const char* name : {"irisBlurEnabled", "tiltShiftEnabled", "pathBlurEnabled",
                             "spinBlurEnabled"}) {
        auto* row = dialog.findChild<QCheckBox*>(QLatin1String(name));
        QVERIFY2(row != nullptr, name);
        QVERIFY2(!row->isChecked() && !row->isEnabled(), name);
    }
    dialog.reject();
}

void BlurGalleryDialogTest::pinListRoundTrips()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), fieldBlurSpec());
    QList<double> pins = {2.0, 0.2, 0.3, 10.0, 0.8, 0.7, 30.0};
    pins += defaultTail();
    dialog.setValues(pins);
    QCOMPARE(dialog.values(), pins);
    // A malformed or empty list keeps the defaults.
    dialog.setValues({2.0, 0.2, 0.3});
    QCOMPARE(dialog.values(), defaultValues());
    dialog.reject();
}

void BlurGalleryDialogTest::pinsAreAddedMovedAndRemoved()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), fieldBlurSpec());
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();
    const QPoint center = canvas->rect().center();
    // The default pin sits at the centre, so click beside it to add one.
    const QPoint at = center + QPoint(100, 60);

    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at);
    QCOMPARE(pinCount(dialog.values()), 2);
    const QList<double> two = dialog.values();
    QVERIFY2(qAbs(two[4] - 0.5) > 0.05 && qAbs(two[5] - 0.5) > 0.05,
             "the new pin is not where it was clicked");

    drag(canvas, at, at + QPoint(80, 40));
    const QList<double> moved = dialog.values();
    QVERIFY2(moved[4] > two[4] + 0.05, "the pin did not move right");
    QVERIFY2(moved[5] > two[5] + 0.02, "the pin did not move down");

    QTest::keyClick(canvas, Qt::Key_Delete);
    QCOMPARE(pinCount(dialog.values()), 1);
    // The last pin is never removed.
    QTest::keyClick(canvas, Qt::Key_Delete);
    QCOMPARE(pinCount(dialog.values()), 1);
    dialog.reject();
}

void BlurGalleryDialogTest::blurAmountEditsTheSelectedPinAndPreviews()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), fieldBlurSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    const QImage unfiltered = dialog.canvas()->image();
    QVERIFY(!unfiltered.isNull());

    auto* spin = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("fieldBlurSpin"));
    QVERIFY(spin != nullptr);
    spin->setValue(24.0);
    QTRY_COMPARE(dialog.values().at(3), 24.0);
    // The scheduled re-render blurs the preview.
    QTRY_VERIFY2(dialog.canvas()->image() != unfiltered, "the preview did not blur");

    auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"));
    preview->setChecked(false);
    QTRY_COMPARE(dialog.canvas()->image(), unfiltered);
    preview->setChecked(true);
    QTRY_VERIFY(dialog.canvas()->image() != unfiltered);
    dialog.reject();
}

void BlurGalleryDialogTest::effectsTabCarriesTheBokehControls()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), fieldBlurSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("blurEffectsTabs"));
    QVERIFY(tabs != nullptr);
    QStringList names;
    for (int i = 0; i < tabs->count(); ++i) {
        names.append(tabs->tabText(i));
    }
    QCOMPARE(names, QStringList({QStringLiteral("Effects"), QStringLiteral("Motion Effects"),
                                 QStringLiteral("Noise")}));

    auto* bokeh = dialog.findChild<QCheckBox*>(QStringLiteral("bokehEnabled"));
    auto* light = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bokehLightSpin"));
    auto* color = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bokehColorSpin"));
    auto* low = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bokehRangeLow"));
    auto* high = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bokehRangeHigh"));
    QVERIFY(bokeh != nullptr && bokeh->isChecked());
    QVERIFY(light != nullptr && light->value() == 14.0);
    QVERIFY(color != nullptr && color->value() == 40.0);
    QVERIFY(low != nullptr && low->value() == 40.0);
    QVERIFY(high != nullptr && high->value() == 118.0);

    // Blur the picture with Bokeh off, then raise Light Bokeh: the preview
    // lifts.
    const QImage unfiltered = dialog.canvas()->image();
    bokeh->setChecked(false);
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("fieldBlurSpin"))->setValue(30.0);
    QTRY_VERIFY(dialog.canvas()->image() != unfiltered);
    const QImage blurred = dialog.canvas()->image();
    bokeh->setChecked(true);
    light->setValue(100.0);
    QTRY_VERIFY2(dialog.canvas()->image() != blurred, "Light Bokeh did not lift the preview");

    // Unchecking Bokeh turns the effect off but leaves the controls editable.
    bokeh->setChecked(false);
    QVERIFY(light->isEnabled());
    QVERIFY(high->isEnabled());
    QTRY_COMPARE(dialog.canvas()->image(), blurred);
    dialog.reject();
}

void BlurGalleryDialogTest::noiseTabCarriesTheGrainControls()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), fieldBlurSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* kind = dialog.findChild<QComboBox*>(QStringLiteral("noiseKind"));
    auto* enabled = dialog.findChild<QCheckBox*>(QStringLiteral("noiseEnabled"));
    auto* amount = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("noiseAmountSpin"));
    QVERIFY(kind != nullptr && kind->count() == 3
            && kind->currentText() == QStringLiteral("Gaussian"));
    QCOMPARE(kind->itemText(0), QStringLiteral("Gaussian"));
    QCOMPARE(kind->itemText(1), QStringLiteral("Uniform"));
    QCOMPARE(kind->itemText(2), QStringLiteral("Grain"));
    QVERIFY(enabled != nullptr && !enabled->isChecked());
    QVERIFY(amount != nullptr && amount->value() == 0.0);
    // The controls stay available while the effect is switched off.
    QVERIFY(kind->isEnabled());
    QVERIFY(amount->isEnabled());
    for (const char* name : {"noiseSizeSpin", "noiseRoughnessSpin", "noiseColorSpin",
                             "noiseHighlightsSpin"}) {
        QVERIFY2(dialog.findChild<QDoubleSpinBox*>(QLatin1String(name)) != nullptr, name);
    }

    // Blur the picture, then add grain: the preview roughens.
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("fieldBlurSpin"))->setValue(30.0);
    QTRY_VERIFY(dialog.canvas()->image() != QImage());
    const QImage blurred = dialog.canvas()->image();
    enabled->setChecked(true);
    amount->setValue(100.0);
    QTRY_VERIFY2(dialog.canvas()->image() != blurred, "the grain did not show");
    dialog.reject();
}

void BlurGalleryDialogTest::hideUiTogglesThePinOverlay()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), fieldBlurSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QVERIFY(dialog.canvas()->uiVisible());
    QTest::keyClick(&dialog, Qt::Key_H);
    QVERIFY(!dialog.canvas()->uiVisible());
    QTest::keyClick(&dialog, Qt::Key_H);
    QVERIFY(dialog.canvas()->uiVisible());
    dialog.reject();
}

void BlurGalleryDialogTest::menuCommitsAndCancelRestores()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int before = view->history_count();
    QList<double> committed;
    QTimer::singleShot(0, [&committed] {
        if (auto* dialog = openWorkspace()) {
            dialog->findChild<QDoubleSpinBox*>(QStringLiteral("fieldBlurSpin"))->setValue(18.0);
            committed = dialog->values();
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kFieldBlurPath)));
    QCOMPARE(committed.size(), 17);
    QCOMPARE(committed.at(3), 18.0);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Field Blur"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("field-blur"));
    QCOMPARE(filter_last_params(*view), committed);

    const QImage committedPixels = view->image();
    QTimer::singleShot(0, [] {
        if (auto* dialog = openWorkspace()) {
            dialog->findChild<QDoubleSpinBox*>(QStringLiteral("fieldBlurSpin"))->setValue(60.0);
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kFieldBlurPath)));
    QCOMPARE(view->image(), committedPixels);
    QCOMPARE(view->history_count(), before + 1);
}

void BlurGalleryDialogTest::lastFilterReusesThePins()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    QList<double> pins = {2.0, 0.25, 0.5, 0.0, 0.75, 0.5, 40.0};
    pins += defaultTail();
    QVERIFY(apply_filter_params(*view, QStringLiteral("field-blur"), pins));

    pictura::CommandRegistry* registry = window_->registry();
    registry->refresh();
    QAction* last = registry->action(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QVERIFY(last != nullptr && last->isEnabled());
    QCOMPARE(last->text(), QStringLiteral("Field Blur"));
    const int states = view->history_count();
    const QImage once = view->image();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    QCOMPARE(view->history_count(), states + 1);
    QVERIFY2(view->image() != once, "Last Filter did not blur again");

    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = openWorkspace()) {
            seen = dialog->values();
            dialog->reject();
        }
    });
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(seen, pins);
}

QTEST_MAIN(BlurGalleryDialogTest)

#include "tst_blur_gallery_dialog.moc"
