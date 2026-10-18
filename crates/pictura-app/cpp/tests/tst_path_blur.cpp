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
#include <QtWidgets/QToolButton>

#include "blur_gallery_canvas.h"
#include "blur_gallery_dialog.h"
#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

// Filter > Blur Gallery > Path Blur (a post-CS6 effect, ported on the
// owner's request).

namespace {

const QStringList kPathBlurPath = {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"),
                                   QStringLiteral("Path Blur")};

const pictura::FilterCommandSpec& pathBlurSpec()
{
    return *pictura::filterCommandForPath(kPathBlurPath);
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

// Red and blue vary along rows and green down columns, so a blur along rows
// changes red and leaves green alone.
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

QList<double> defaultTail()
{
    return {1.0, 14.0, 40.0, 40.0, 118.0, 1.0, 0.0, 50.0, 50.0, 0.0, 0.0, 0.0, 1.0};
}

// Speed 50, Taper 0, Centered Blur, Basic Blur, no strobe, four flashes.
QList<double> defaultMotion()
{
    return {50.0, 0.0, 1.0, 0.0, 0.0, 4.0};
}

// One path right from the centre, 0.08 of the width; both end speeds
// 0.04 of the 96 px document, rounded.
QList<double> defaultPathValues()
{
    QList<double> values = {1.0, 2.0, 0.5, 0.5, 0.58, 0.5, 4.0, 4.0};
    values += defaultMotion();
    values += defaultTail();
    return values;
}

QRectF pictureRect(const QWidget* canvas)
{
    const QRectF room = QRectF(canvas->rect()).adjusted(24, 24, -24, -24);
    const QSizeF fitted = QSizeF(96, 64).scaled(room.size(), Qt::KeepAspectRatio);
    return QRectF(room.center() - QPointF(fitted.width(), fitted.height()) / 2.0, fitted);
}

QPoint at(const QWidget* canvas, double x, double y)
{
    const QRectF area = pictureRect(canvas);
    return QPointF(area.x() + x * area.width(), area.y() + y * area.height()).toPoint();
}

void drag(QWidget* widget, const QPoint& from, const QPoint& to,
          Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QTest::mousePress(widget, Qt::LeftButton, modifiers, from);
    QTest::mouseMove(widget, (from + to) / 2);
    QTest::mouseMove(widget, to);
    QTest::mouseRelease(widget, Qt::LeftButton, modifiers, to);
}

// The path count and each path's point count, from a single-effect list.
QList<int> shape(const QList<double>& values)
{
    QList<int> out = {int(values.at(0))};
    int at = 1;
    for (int i = 0; i < out.first(); ++i) {
        const int points = int(values.at(at));
        out << points;
        at += 1 + 2 * points + 2;
    }
    return out;
}

// Path `index`'s points, from a single-effect list.
QList<QPointF> pathPoints(const QList<double>& values, int index)
{
    int at = 1;
    for (int i = 0; i < index; ++i) {
        at += 1 + 2 * int(values.at(at)) + 2;
    }
    QList<QPointF> points;
    for (int k = 0; k < int(values.at(at)); ++k) {
        points << QPointF(values.at(at + 1 + 2 * k), values.at(at + 2 + 2 * k));
    }
    return points;
}

bool near(const QPointF& a, const QPointF& b)
{
    return qAbs(a.x() - b.x()) < 0.02 && qAbs(a.y() - b.y()) < 0.02;
}

} // namespace

class PathBlurTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void workspaceOpensOnTheDefaultPath();
    void pathListRoundTrips();
    void canvasEditsPointsAndPaths();
    void hoverShowsThePinCursor();
    void effectsTabFollowsTheSelectedEffect();
    void noiseGrainsThePreview();
    void endPointSpeedEditsTheSelectedEnd();
    void strobeMakesThePresetCustom();
    void previewBlursAlongThePath();
    void menuCommitsOnce();
    void combinesWithFieldBlur();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PathBlurTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern().save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kPathBlurPath) != nullptr);
}

void PathBlurTest::workspaceOpensOnTheDefaultPath()
{
    window_->registry()->refresh();
    QAction* entry = window_->registry()->action(pictura::commandIdForPath(kPathBlurPath));
    QVERIFY(entry != nullptr && entry->isEnabled());
    QCOMPARE(entry->text(), QStringLiteral("Path Blur…"));

    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    QCOMPARE(dialog.canvas()->mode(), pictura::BlurGalleryMode::Path);
    QCOMPARE(dialog.kind(), QStringLiteral("path-blur"));
    QCOMPARE(dialog.values(), defaultPathValues());
    auto* row = dialog.findChild<QCheckBox*>(QStringLiteral("pathBlurEnabled"));
    QVERIFY(row != nullptr && row->isChecked() && row->isEnabled());
    auto* spin = dialog.findChild<QCheckBox*>(QStringLiteral("spinBlurEnabled"));
    QVERIFY(spin != nullptr && spin->isEnabled() && !spin->isChecked());
    auto* preset = dialog.findChild<QComboBox*>(QStringLiteral("pathBlurPreset"));
    QVERIFY(preset != nullptr);
    QCOMPARE(preset->currentText(), QStringLiteral("Basic Blur"));
    QCOMPARE(preset->count(), 2);
    auto* speed = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("pathSpeedSpin"));
    QVERIFY(speed != nullptr && speed->value() == 50.0 && speed->maximum() == 500.0);
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("pathCentered"))->isChecked());
    QVERIFY(!dialog.findChild<QDoubleSpinBox*>(QStringLiteral("pathEndSpeedSpin"))->isEnabled());
    QVERIFY(!dialog.findChild<QCheckBox*>(QStringLiteral("pathEditShapes"))->isEnabled());
    QCOMPARE(dialog.findChild<QDoubleSpinBox*>(QStringLiteral("strobeFlashesSpin"))->value(), 4.0);
    dialog.reject();
}

void PathBlurTest::pathListRoundTrips()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    QList<double> list = {2.0, 2.0, 0.1, 0.2, 0.3, 0.4, 5.0, 25.0,
                          3.0, 0.5, 0.5, 0.6, 0.4, 0.9, 0.5, 40.0, 0.0};
    list += {120.0, 30.0, 0.0, 1.0, 40.0, 6.0};
    list += defaultTail();
    dialog.setValues(list);
    QCOMPARE(dialog.values(), list);
    QCOMPARE(dialog.findChild<QComboBox*>(QStringLiteral("pathBlurPreset"))->currentText(),
             QStringLiteral("Custom"));
    QVERIFY(!dialog.findChild<QCheckBox*>(QStringLiteral("pathCentered"))->isChecked());

    dialog.reject();

    // A malformed list keeps the defaults.
    pictura::BlurGalleryDialog fresh(window_->activeView(), pathBlurSpec());
    fresh.setValues(list.mid(0, list.size() - 1));
    QCOMPARE(fresh.values(), defaultPathValues());
    fresh.reject();
}

void PathBlurTest::canvasEditsPointsAndPaths()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();

    // Dragging the end point moves it.
    drag(canvas, at(canvas, 0.58, 0.5), at(canvas, 0.8, 0.5));
    QList<double> v = dialog.values();
    QVERIFY2(qAbs(v.at(4) - 0.8) < 0.02, qPrintable(QString::number(v.at(4))));
    QCOMPARE(canvas->selectedPoint(), 1);

    // A press on the curve inserts a point there.
    drag(canvas, at(canvas, 0.65, 0.5), at(canvas, 0.65, 0.3));
    QCOMPARE(shape(dialog.values()), QList<int>({1, 3}));
    QCOMPARE(canvas->selectedPoint(), 1);
    QVERIFY(dialog.values().at(5) < 0.4);

    // Ctrl-dragging a point moves the whole path.
    const QList<double> before = dialog.values();
    drag(canvas, at(canvas, 0.5, 0.5), at(canvas, 0.5, 0.6), Qt::ControlModifier);
    const QList<double> moved = dialog.values();
    for (const int y : {3, 5, 7}) {
        QVERIFY2(qAbs(moved.at(y) - before.at(y) - 0.1) < 0.02, qPrintable(QString::number(y)));
    }

    // Clicks on empty picture pin a new path's stops while its last point
    // follows the pointer; a double-click ends it on the last stop.
    canvas->setFocus();
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.15, 0.85));
    QVERIFY(canvas->building() && canvas->dragging());
    QCOMPARE(canvas->selected(), 1);
    QTest::mouseMove(canvas, at(canvas, 0.3, 0.7));
    QVERIFY(near(pathPoints(dialog.values(), 1).last(), {0.3, 0.7}));
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.3, 0.7));
    QTest::mouseMove(canvas, at(canvas, 0.45, 0.85));
    QCOMPARE(shape(dialog.values()), QList<int>({2, 3, 3}));
    // Qt delivers a double-click as a click, then the double-click event.
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.45, 0.85));
    QTest::mouseDClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.45, 0.85));
    QVERIFY(!canvas->building() && !canvas->dragging());
    QCOMPARE(shape(dialog.values()), QList<int>({2, 3, 3}));
    const QList<QPointF> curve = pathPoints(dialog.values(), 1);
    QVERIFY(near(curve[0], {0.15, 0.85}) && near(curve[1], {0.3, 0.7})
            && near(curve[2], {0.45, 0.85}));

    // Enter ends a path on its first stop at the default length; Esc ends a
    // path without closing the workspace.
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.1, 0.15));
    QTest::mouseMove(canvas, at(canvas, 0.2, 0.3));
    QTest::keyClick(canvas, Qt::Key_Return);
    v = dialog.values();
    QCOMPARE(shape(v), QList<int>({3, 3, 3, 2}));
    const QList<QPointF> stub = pathPoints(v, 2);
    QVERIFY2(qAbs(stub[1].x() - stub[0].x() - 0.08) < 1e-9 && stub[1].y() == stub[0].y(),
             "the default length");
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.6, 0.15));
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.75, 0.2));
    QTest::keyClick(canvas, Qt::Key_Escape);
    QVERIFY(!canvas->building() && dialog.isVisible());
    QCOMPARE(shape(dialog.values()), QList<int>({4, 3, 3, 2, 2}));

    // Delete removes the selected path (its points cannot drop below two),
    // then a point of the curved one.
    QTest::keyClick(canvas, Qt::Key_Delete);
    QCOMPARE(shape(dialog.values()), QList<int>({3, 3, 3, 2}));
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier,
                      at(canvas, moved.at(4), moved.at(5)));
    QCOMPARE(canvas->selected(), 0);
    QCOMPARE(canvas->selectedPoint(), 1);
    QTest::keyClick(canvas, Qt::Key_Delete);
    QCOMPARE(shape(dialog.values()), QList<int>({3, 2, 3, 2}));
    dialog.reject();
}

void PathBlurTest::hoverShowsThePinCursor()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();
    QTest::mouseMove(canvas, at(canvas, 0.2, 0.2));
    QCOMPARE(canvas->cursor().shape(), Qt::BitmapCursor);
    QTest::mouseMove(canvas, at(canvas, 0.5, 0.5));
    QCOMPARE(canvas->cursor().shape(), Qt::ArrowCursor);
    dialog.reject();
}

void PathBlurTest::effectsTabFollowsTheSelectedEffect()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("blurEffectsTabs"));
    QVERIFY(tabs != nullptr);
    // Bokeh does not apply to Path Blur; the Motion Effects do.
    QVERIFY(!tabs->isTabEnabled(0));
    QVERIFY(tabs->isTabEnabled(1));
    QCOMPARE(tabs->currentIndex(), 1);
    QVERIFY(tabs->isTabEnabled(2));
    dialog.findChild<QToolButton*>(QStringLiteral("fieldBlurSelect"))->click();
    QVERIFY(tabs->isTabEnabled(0));
    QVERIFY(!tabs->isTabEnabled(1));
    QCOMPARE(tabs->currentIndex(), 0);
    dialog.reject();
}

void PathBlurTest::noiseGrainsThePreview()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("noiseEnabled"))->isChecked());
    QCoreApplication::processEvents();
    const QImage clean = dialog.canvas()->image();
    QVERIFY(!clean.isNull());
    // Amount alone turns the grain on; green, constant along each row, only
    // changes through the grain.
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("noiseAmountSpin"))->setValue(30.0);
    QTRY_VERIFY2(dialog.canvas()->image() != clean, "the grain did not reach the preview");
    const QImage grainy = dialog.canvas()->image();
    int changed = 0;
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 96; ++x) {
            changed += qGreen(grainy.pixel(x, y)) != qGreen(clean.pixel(x, y)) ? 1 : 0;
        }
    }
    QVERIFY2(changed > 96 * 64 / 2, qPrintable(QString::number(changed)));
    dialog.reject();
}

void PathBlurTest::endPointSpeedEditsTheSelectedEnd()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();
    auto* endSpeed = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("pathEndSpeedSpin"));
    QVERIFY(endSpeed != nullptr && !endSpeed->isEnabled());

    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.58, 0.5));
    QVERIFY(endSpeed->isEnabled());
    QCOMPARE(endSpeed->value(), 4.0);
    endSpeed->setValue(20.0);
    QList<double> v = dialog.values();
    QCOMPARE(v.at(6), 4.0);
    QCOMPARE(v.at(7), 20.0);

    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.5, 0.5));
    QCOMPARE(endSpeed->value(), 4.0);
    endSpeed->setValue(9.0);
    v = dialog.values();
    QCOMPARE(v.at(6), 9.0);
    QCOMPARE(v.at(7), 20.0);

    // A middle point is no end point.
    drag(canvas, at(canvas, 0.54, 0.5), at(canvas, 0.54, 0.4));
    QCOMPARE(shape(dialog.values()), QList<int>({1, 3}));
    QVERIFY(!endSpeed->isEnabled());
    dialog.reject();
}

void PathBlurTest::strobeMakesThePresetCustom()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    auto* preset = dialog.findChild<QComboBox*>(QStringLiteral("pathBlurPreset"));
    auto* strength = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("strobeStrengthSpin"));
    QVERIFY(preset != nullptr && strength != nullptr);

    strength->setValue(30.0);
    QCOMPARE(preset->currentText(), QStringLiteral("Custom"));
    QCOMPARE(dialog.values().at(12), 30.0);

    preset->setCurrentIndex(1);
    QCOMPARE(preset->currentText(), QStringLiteral("Rear Sync Flash"));
    QCOMPARE(preset->count(), 2);
    QCOMPARE(strength->value(), 0.0);
    const QList<double> v = dialog.values();
    QCOMPARE(v.at(11), 1.0); // rear sync
    QCOMPARE(v.at(12), 0.0);
    dialog.reject();
}

void PathBlurTest::previewBlursAlongThePath()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), pathBlurSpec());
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(false);
    QCoreApplication::processEvents();
    const QImage unfiltered = dialog.canvas()->image();
    QVERIFY(!unfiltered.isNull());
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("pathSpeedSpin"))->setValue(500.0);
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(true);
    QTRY_VERIFY2(dialog.canvas()->image() != unfiltered, "the path did not blur the preview");
    const QImage blurred = dialog.canvas()->image();
    // A horizontal path smears along rows: green (constant along each row)
    // is untouched everywhere, red changes.
    bool redMoved = false;
    for (int y = 0; y < 64; y += 3) {
        for (int x = 0; x < 96; x += 3) {
            QCOMPARE(qGreen(blurred.pixel(x, y)), qGreen(unfiltered.pixel(x, y)));
            redMoved = redMoved || qRed(blurred.pixel(x, y)) != qRed(unfiltered.pixel(x, y));
        }
    }
    QVERIFY(redMoved);
    dialog.reject();
}

void PathBlurTest::menuCommitsOnce()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int before = view->history_count();
    QList<double> committed;
    QTimer::singleShot(0, [&committed] {
        if (auto* dialog = openWorkspace()) {
            dialog->findChild<QDoubleSpinBox*>(QStringLiteral("pathSpeedSpin"))->setValue(80.0);
            committed = dialog->values();
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kPathBlurPath)));
    QCOMPARE(committed.size(), 27);
    QCOMPARE(committed.at(8), 80.0);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Path Blur"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("path-blur"));
    QCOMPARE(filter_last_params(*view), committed);
}

void PathBlurTest::combinesWithFieldBlur()
{
    const QStringList fieldPath = {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"),
                                   QStringLiteral("Field Blur")};
    pictura::BlurGalleryDialog dialog(window_->activeView(),
                                      *pictura::filterCommandForPath(fieldPath));
    dialog.findChild<QCheckBox*>(QStringLiteral("pathBlurEnabled"))->setChecked(true);
    QCOMPARE(dialog.selectedEffect(), pictura::BlurGalleryMode::Path);
    QCOMPARE(dialog.kind(), QStringLiteral("blur-gallery"));
    const QList<double> v = dialog.values();
    // One Field pin, no Iris or Tilt-Shift, one path, no spins, the motion,
    // the flash duration, the tail.
    QCOMPARE(v.size(), 1 + 3 + 1 + 1 + 1 + 7 + 1 + 6 + 1 + 13);
    QCOMPARE(v.at(0), 1.0);
    QCOMPARE(v.at(4), 0.0);
    QCOMPARE(v.at(5), 0.0);
    QCOMPARE(v.mid(6, 8), QList<double>({1.0, 2.0, 0.5, 0.5, 0.58, 0.5, 4.0, 4.0}));
    QCOMPARE(v.at(14), 0.0);
    QCOMPARE(v.mid(15, 6), defaultMotion());
    QCOMPARE(v.at(21), 1.0);

    pictura::BlurGalleryDialog reopened(
        window_->activeView(), *pictura::filterCommandForKind(QStringLiteral("blur-gallery")));
    reopened.setValues(v);
    QCOMPARE(reopened.values(), v);
    QVERIFY(reopened.effectEnabled(pictura::BlurGalleryMode::Path));
    dialog.reject();
}

QTEST_MAIN(PathBlurTest)

#include "tst_path_blur.moc"
