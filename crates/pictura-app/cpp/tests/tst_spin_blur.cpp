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

// Filter > Blur Gallery > Spin Blur (a post-CS6 effect, ported on the
// owner's request).

namespace {

using Mode = pictura::BlurGalleryMode;

const QStringList kSpinBlurPath = {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"),
                                   QStringLiteral("Spin Blur")};

const pictura::FilterCommandSpec& spinBlurSpec()
{
    return *pictura::filterCommandForPath(kSpinBlurPath);
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

// Red and blue vary along rows and green down columns, so turning any part
// of the picture changes it.
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

// A centred circle 0.13 of the width across, feather 0.5, at 15°, no pivot.
QList<double> defaultSpin()
{
    return {0.5, 0.5, 0.13, 0.13, 0.0, 0.5, 15.0, 0.0, 0.0};
}

QList<double> defaultSpinValues()
{
    QList<double> values = {1.0};
    values += defaultSpin();
    values += {0.0, 4.0, 1.0}; // no strobe, four flashes, 1°
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

bool close(double a, double b)
{
    return qAbs(a - b) < 0.02;
}

bool shown(const QWidget* dialog, const QString& name)
{
    return dialog->findChild<QWidget*>(name)->isVisibleTo(dialog);
}

} // namespace

class SpinBlurTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void workspaceOpensOnTheDefaultSpin();
    void spinListRoundTrips();
    void canvasEditsSpins();
    void durationShowsOnlyForSpinBlur();
    void previewSpinsThePicture();
    void menuCommitsOnce();
    void combinesWithFieldBlur();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SpinBlurTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern().save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kSpinBlurPath) != nullptr);
}

void SpinBlurTest::workspaceOpensOnTheDefaultSpin()
{
    window_->registry()->refresh();
    QAction* entry = window_->registry()->action(pictura::commandIdForPath(kSpinBlurPath));
    QVERIFY(entry != nullptr && entry->isEnabled());
    QCOMPARE(entry->text(), QStringLiteral("Spin Blur…"));

    pictura::BlurGalleryDialog dialog(window_->activeView(), spinBlurSpec());
    QCOMPARE(dialog.canvas()->mode(), Mode::Spin);
    QCOMPARE(dialog.selectedEffect(), Mode::Spin);
    QCOMPARE(dialog.kind(), QStringLiteral("spin-blur"));
    QCOMPARE(dialog.values(), defaultSpinValues());
    auto* row = dialog.findChild<QCheckBox*>(QStringLiteral("spinBlurEnabled"));
    QVERIFY(row != nullptr && row->isChecked() && row->isEnabled());
    auto* angle = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("spinBlurSpin"));
    QVERIFY(angle != nullptr && angle->isVisibleTo(&dialog));
    QCOMPARE(angle->value(), 15.0);
    QCOMPARE(angle->maximum(), 360.0);
    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("blurEffectsTabs"));
    QVERIFY(tabs != nullptr);
    QVERIFY(!tabs->isTabEnabled(0));
    QVERIFY(tabs->isTabEnabled(1));
    QCOMPARE(tabs->currentIndex(), 1);
    auto* duration = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("strobeDurationSpin"));
    QVERIFY(duration != nullptr && duration->isVisibleTo(&dialog));
    QCOMPARE(duration->value(), 1.0);
    dialog.reject();
}

void SpinBlurTest::spinListRoundTrips()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spinBlurSpec());
    QList<double> list = {2.0, 0.3, 0.6, 0.2, 0.1, 25.0, 0.4, 90.0, 0.05, -0.02,
                          0.8, 0.2, 0.1, 0.1, 0.0, 1.0, 300.0, 0.0, 0.0};
    list += {30.0, 6.0, 12.0};
    list += defaultTail();
    dialog.setValues(list);
    QCOMPARE(dialog.values(), list);
    // The strobe is shared with Path Blur, whose preset reads Custom.
    QCOMPARE(dialog.findChild<QComboBox*>(QStringLiteral("pathBlurPreset"))->currentText(),
             QStringLiteral("Custom"));
    dialog.reject();

    // A malformed list keeps the defaults.
    pictura::BlurGalleryDialog fresh(window_->activeView(), spinBlurSpec());
    fresh.setValues(list.mid(0, list.size() - 1));
    QCOMPARE(fresh.values(), defaultSpinValues());
    fresh.reject();
}

void SpinBlurTest::canvasEditsSpins()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spinBlurSpec());
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();
    // Radii are fractions of the width; the picture is 1.5 times as wide as
    // tall, so a width fraction down the picture is 1.5 times as much.
    const double aspect = 96.0 / 64.0;

    // The lower feather dot sits half the y radius below the centre.
    drag(canvas, at(canvas, 0.5, 0.5 + 0.13 * 0.5 * aspect),
         at(canvas, 0.5, 0.5 + 0.13 * 0.8 * aspect));
    QList<double> v = dialog.values();
    QVERIFY2(close(v.at(6), 0.8), qPrintable(QString::number(v.at(6))));

    // The right radius handle resizes the x semi-axis and keeps the angle.
    drag(canvas, at(canvas, 0.63, 0.5), at(canvas, 0.75, 0.5));
    v = dialog.values();
    QVERIFY2(close(v.at(3), 0.25), qPrintable(QString::number(v.at(3))));
    QVERIFY2(qAbs(v.at(5)) < 2.0, qPrintable(QString::number(v.at(5))));

    // Alt-dragging the marker moves only the rotation centre...
    drag(canvas, at(canvas, 0.5, 0.5), at(canvas, 0.55, 0.45), Qt::AltModifier);
    v = dialog.values();
    QVERIFY(close(v.at(1), 0.5) && close(v.at(2), 0.5));
    QVERIFY2(close(v.at(8), 0.05) && close(v.at(9), -0.05),
             qPrintable(QStringLiteral("%1 %2").arg(v.at(8)).arg(v.at(9))));
    // ...and dragging it moves the spin, the rotation centre with it.
    drag(canvas, at(canvas, 0.55, 0.45), at(canvas, 0.6, 0.5));
    v = dialog.values();
    QVERIFY2(close(v.at(1), 0.55) && close(v.at(2), 0.55),
             qPrintable(QStringLiteral("%1 %2").arg(v.at(1)).arg(v.at(2))));
    QVERIFY(close(v.at(8), 0.05) && close(v.at(9), -0.05));

    // The ring sets the Blur Angle, a degree per pixel.
    const QPoint ring = at(canvas, 0.6, 0.5) + QPoint(13, 0);
    drag(canvas, ring, ring + QPoint(40, 0));
    v = dialog.values();
    QVERIFY2(qAbs(v.at(7) - 55.0) <= 1.0, qPrintable(QString::number(v.at(7))));

    // A click on empty picture adds a spin; Delete removes it.
    canvas->setFocus();
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, at(canvas, 0.1, 0.15));
    v = dialog.values();
    QCOMPARE(v.at(0), 2.0);
    QCOMPARE(canvas->selected(), 1);
    QVERIFY(close(v.at(10), 0.1) && close(v.at(11), 0.15));
    QCOMPARE(v.at(17), 0.0); // a new spin turns round its own centre
    QCOMPARE(v.at(18), 0.0);
    QTest::keyClick(canvas, Qt::Key_Delete);
    QCOMPARE(dialog.values().at(0), 1.0);
    dialog.reject();
}

void SpinBlurTest::durationShowsOnlyForSpinBlur()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spinBlurSpec());
    QVERIFY(shown(&dialog, QStringLiteral("strobeDurationSpin")));
    QVERIFY(shown(&dialog, QStringLiteral("strobeStrengthSpin")));
    QVERIFY(shown(&dialog, QStringLiteral("strobeFlashesSpin")));
    dialog.findChild<QToolButton*>(QStringLiteral("pathBlurSelect"))->click();
    QCOMPARE(dialog.selectedEffect(), Mode::Path);
    QVERIFY(!shown(&dialog, QStringLiteral("strobeDurationSpin")));
    QVERIFY(shown(&dialog, QStringLiteral("strobeStrengthSpin")));
    QVERIFY(shown(&dialog, QStringLiteral("strobeFlashesSpin")));
    dialog.findChild<QToolButton*>(QStringLiteral("fieldBlurSelect"))->click();
    auto* tabs = dialog.findChild<QTabWidget*>(QStringLiteral("blurEffectsTabs"));
    QVERIFY(tabs->isTabEnabled(0) && !tabs->isTabEnabled(1));
    dialog.reject();
}

void SpinBlurTest::previewSpinsThePicture()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spinBlurSpec());
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(false);
    QCoreApplication::processEvents();
    const QImage unfiltered = dialog.canvas()->image();
    QVERIFY(!unfiltered.isNull());
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("spinBlurSpin"))->setValue(180.0);
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(true);
    QTRY_VERIFY2(dialog.canvas()->image() != unfiltered, "the spin did not reach the preview");
    const QImage spun = dialog.canvas()->image();
    bool centreMoved = false;
    for (int y = 26; y < 38; ++y) {
        for (int x = 42; x < 54; ++x) {
            centreMoved = centreMoved || spun.pixel(x, y) != unfiltered.pixel(x, y);
        }
    }
    QVERIFY(centreMoved);
    // Outside the spin the picture stays sharp.
    for (const QPoint& corner : {QPoint(2, 2), QPoint(93, 2), QPoint(2, 61), QPoint(93, 61)}) {
        QCOMPARE(spun.pixel(corner), unfiltered.pixel(corner));
    }
    dialog.reject();
}

void SpinBlurTest::menuCommitsOnce()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int before = view->history_count();
    QList<double> committed;
    QTimer::singleShot(0, [&committed] {
        if (auto* dialog = openWorkspace()) {
            dialog->findChild<QDoubleSpinBox*>(QStringLiteral("spinBlurSpin"))->setValue(40.0);
            committed = dialog->values();
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kSpinBlurPath)));
    QCOMPARE(committed.size(), 26);
    QCOMPARE(committed.at(7), 40.0);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Spin Blur"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("spin-blur"));
    QCOMPARE(filter_last_params(*view), committed);
}

void SpinBlurTest::combinesWithFieldBlur()
{
    const QStringList fieldPath = {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"),
                                   QStringLiteral("Field Blur")};
    pictura::BlurGalleryDialog dialog(window_->activeView(),
                                      *pictura::filterCommandForPath(fieldPath));
    dialog.findChild<QCheckBox*>(QStringLiteral("spinBlurEnabled"))->setChecked(true);
    QCOMPARE(dialog.selectedEffect(), Mode::Spin);
    QCOMPARE(dialog.kind(), QStringLiteral("blur-gallery"));
    const QList<double> v = dialog.values();
    // One Field pin, no Iris, Tilt-Shift, or paths, one spin, the motion,
    // the flash duration, the tail.
    QCOMPARE(v.size(), 1 + 3 + 1 + 1 + 1 + 1 + 9 + 6 + 1 + 13);
    QCOMPARE(v.at(0), 1.0);
    QCOMPARE(v.mid(4, 3), QList<double>({0.0, 0.0, 0.0}));
    QCOMPARE(v.at(7), 1.0);
    QCOMPARE(v.mid(8, 9), defaultSpin());
    QCOMPARE(v.mid(17, 6), QList<double>({50.0, 0.0, 1.0, 0.0, 0.0, 4.0}));
    QCOMPARE(v.at(23), 1.0);

    pictura::BlurGalleryDialog reopened(
        window_->activeView(), *pictura::filterCommandForKind(QStringLiteral("blur-gallery")));
    reopened.setValues(v);
    QCOMPARE(reopened.values(), v);
    QVERIFY(reopened.effectEnabled(Mode::Spin));
    QVERIFY(reopened.effectEnabled(Mode::Field));
    dialog.reject();
}

QTEST_MAIN(SpinBlurTest)

#include "tst_spin_blur.moc"
