#include <QtTest/QtTest>

#include <QtCore/QElapsedTimer>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QMessageBox>

#include "blur_gallery_canvas.h"
#include "blur_gallery_dialog.h"
#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

// Filter > Blur Gallery > Tilt-Shift, and the empty-target refusal every
// Blur and Blur Gallery command shares.

namespace {

const QStringList kTiltShiftPath = {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"),
                                    QStringLiteral("Tilt-Shift")};

const pictura::FilterCommandSpec& tiltShiftSpec()
{
    return *pictura::filterCommandForPath(kTiltShiftPath);
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

QImage pattern(int w = 96, int h = 64)
{
    QImage image(w, h, QImage::Format_RGB32);
    for (int y = 0; y < h; ++y) {
        for (int x = 0; x < w; ++x) {
            image.setPixelColor(x, y, QColor((x * 4) % 256, (y * 4) % 256, (x * 9) % 256));
        }
    }
    return image;
}

QList<double> defaultTail()
{
    return {1.0, 14.0, 40.0, 40.0, 118.0, 1.0, 0.0, 50.0, 50.0, 0.0, 0.0, 0.0, 1.0};
}

// One centred band: angle 0, focus 0.1 and fade 0.15 of the height each way,
// no distortion, blur 15.
QList<double> defaultTiltValues()
{
    QList<double> values = {1.0, 0.5, 0.5, 0.0, 0.1, 0.1, 0.15, 0.15, 0.0, 0.0, 15.0};
    values += defaultTail();
    return values;
}

QRectF pictureRect(const QWidget* canvas)
{
    const QRectF room = QRectF(canvas->rect()).adjusted(24, 24, -24, -24);
    const QSizeF fitted = QSizeF(96, 64).scaled(room.size(), Qt::KeepAspectRatio);
    return QRectF(room.center() - QPointF(fitted.width(), fitted.height()) / 2.0, fitted);
}

void drag(QWidget* widget, const QPoint& from, const QPoint& to)
{
    QTest::mousePress(widget, Qt::LeftButton, Qt::NoModifier, from);
    QTest::mouseMove(widget, (from + to) / 2);
    QTest::mouseMove(widget, to);
    QTest::mouseRelease(widget, Qt::LeftButton, Qt::NoModifier, to);
}

// Polls for the empty-target error while a command runs: records its text and
// OKs it. Any other dialog that opens means the command was not refused; it is
// rejected so the test cannot hang, and counted in `*others`.
std::unique_ptr<QTimer> answerEmptyError(QStringList* errors, int* others)
{
    auto timer = std::make_unique<QTimer>();
    timer->setInterval(5);
    QObject::connect(timer.get(), &QTimer::timeout, [errors, others]() {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            auto* dialog = qobject_cast<QDialog*>(widget);
            if (!dialog || !dialog->isVisible()) {
                continue;
            }
            auto* box = qobject_cast<QMessageBox*>(dialog);
            if (box && box->objectName() == QStringLiteral("emptyFilterTargetError")) {
                errors->append(box->text());
                box->button(QMessageBox::Ok)->click();
            } else {
                ++*others;
                dialog->reject();
            }
        }
    });
    timer->start();
    return timer;
}

} // namespace

class TiltShiftTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void workspaceOpensOnTheDefaultBand();
    void bandListRoundTrips();
    void linesAndDotsReshapeTheBand();
    void distortionEditsTheSelectedBand();
    void hoverShowsTheDragCursors();
    void aDragPreviewsADraftThenTheFullPicture();
    void previewKeepsTheBandSharp();
    void menuCommitsOnce();
    void onlyTheActiveLayerBlurs();
    void everyBlurRefusesAnEmptyLayer();
    void aSelectionOverTransparencyIsEmpty();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TiltShiftTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern().save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kTiltShiftPath) != nullptr);
}

void TiltShiftTest::workspaceOpensOnTheDefaultBand()
{
    window_->registry()->refresh();
    QAction* entry = window_->registry()->action(pictura::commandIdForPath(kTiltShiftPath));
    QVERIFY(entry != nullptr && entry->isEnabled());
    QCOMPARE(entry->text(), QStringLiteral("Tilt-Shift…"));

    pictura::BlurGalleryDialog dialog(window_->activeView(), tiltShiftSpec());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Tilt-Shift"));
    QCOMPARE(dialog.canvas()->mode(), pictura::BlurGalleryMode::TiltShift);
    QCOMPARE(dialog.values(), defaultTiltValues());
    auto* row = dialog.findChild<QCheckBox*>(QStringLiteral("tiltShiftEnabled"));
    QVERIFY(row != nullptr && row->isChecked() && row->isEnabled());
    auto* blur = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("tiltShiftSpin"));
    QVERIFY(blur != nullptr && blur->value() == 15.0 && blur->maximum() == 500.0);
    auto* distortion = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("tiltDistortionSpin"));
    QVERIFY(distortion != nullptr && distortion->value() == 0.0);
    QCOMPARE(distortion->minimum(), -100.0);
    QCOMPARE(distortion->maximum(), 100.0);
    auto* symmetric = dialog.findChild<QCheckBox*>(QStringLiteral("tiltSymmetric"));
    QVERIFY(symmetric != nullptr && !symmetric->isChecked());
    for (const char* name : {"fieldBlurEnabled", "irisBlurEnabled", "pathBlurEnabled"}) {
        auto* other = dialog.findChild<QCheckBox*>(QLatin1String(name));
        QVERIFY2(other != nullptr, name);
        QVERIFY2(!other->isChecked() && other->isEnabled(), name);
    }
    for (const char* name : {"spinBlurEnabled"}) {
        auto* other = dialog.findChild<QCheckBox*>(QLatin1String(name));
        QVERIFY2(other != nullptr, name);
        QVERIFY2(!other->isChecked() && !other->isEnabled(), name);
    }
    dialog.reject();
}

void TiltShiftTest::bandListRoundTrips()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), tiltShiftSpec());
    QList<double> bands = {2.0, 0.5, 0.4, 15.0, 0.1, 0.05, 0.2, 0.3, 40.0, 1.0, 20.0,
                           0.2, 0.8, -30.0, 0.0, 0.0, 0.1, 0.1, -60.0, 0.0, 8.0};
    bands += defaultTail();
    dialog.setValues(bands);
    QCOMPARE(dialog.values(), bands);
    // An Iris stride does not fit the band record and keeps the default.
    QList<double> iris = {1.0, 0.5, 0.5, 0.35, 0.35, 0.0, 50.0, 0.65, 15.0};
    iris += defaultTail();
    dialog.setValues(iris);
    QCOMPARE(dialog.values(), defaultTiltValues());
    dialog.reject();
}

void TiltShiftTest::linesAndDotsReshapeTheBand()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), tiltShiftSpec());
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();
    const QRectF picture = pictureRect(canvas);
    const QPointF centre = picture.center();
    const double h = picture.height();
    const double side = picture.width() * 0.3;
    const auto at = [&](double dx, double dy) { return (centre + QPointF(dx, dy)).toPoint(); };

    // The lower solid line, grabbed away from the pin, widens the sharp area;
    // its dashed line stays put.
    drag(canvas, at(side, 0.1 * h), at(side, 0.18 * h));
    QList<double> v = dialog.values();
    QVERIFY2(qAbs(v[5] - 0.18) < 0.02, qPrintable(QString::number(v[5])));
    QVERIFY2(qAbs(v[5] + v[7] - 0.25) < 0.02, "the dashed line moved with the solid one");
    QCOMPARE(v[4], 0.1);

    // The upper dashed line widens the fade above.
    drag(canvas, at(-side, -0.25 * h), at(-side, -0.4 * h));
    v = dialog.values();
    QVERIFY2(qAbs(v[6] - 0.3) < 0.02, qPrintable(QString::number(v[6])));

    // The dot on the upper solid line turns the band.
    const double up = 0.1 * h;
    drag(canvas, at(0, -up), at(up, -up));
    v = dialog.values();
    QVERIFY2(qAbs(v[3] - 45.0) < 3.0, qPrintable(QString::number(v[3])));
    QCOMPARE(int(v[0]), 1);
    dialog.reject();
}

void TiltShiftTest::distortionEditsTheSelectedBand()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), tiltShiftSpec());
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("tiltDistortionSpin"))->setValue(-35.0);
    dialog.findChild<QCheckBox*>(QStringLiteral("tiltSymmetric"))->setChecked(true);
    const QList<double> v = dialog.values();
    QCOMPARE(v[8], -35.0);
    QCOMPARE(v[9], 1.0);
    dialog.reject();
}

void TiltShiftTest::hoverShowsTheDragCursors()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), tiltShiftSpec());
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();
    const QRectF picture = pictureRect(canvas);
    const QPointF centre = picture.center();
    const double h = picture.height();
    const double side = picture.width() * 0.3;
    const auto hover = [&](double dx, double dy) {
        QTest::mouseMove(canvas, (centre + QPointF(dx, dy)).toPoint());
        return canvas->cursor().shape();
    };
    // Lines of a horizontal band move vertically, solid and dashed alike.
    QCOMPARE(hover(side, 0.1 * h), Qt::SizeVerCursor);
    QCOMPARE(hover(-side, -0.25 * h), Qt::SizeVerCursor);
    // A dot shows the rotate arrow (a rendered cursor).
    QCOMPARE(hover(0, -0.1 * h), Qt::BitmapCursor);
    // Off the lines, and on the pin, the cursor is plain.
    QCOMPARE(hover(side, 0.05 * h), Qt::ArrowCursor);
    QCOMPARE(hover(0, 0), Qt::ArrowCursor);

    // A band turned 45° moves along the other diagonal.
    QList<double> turned = {1.0, 0.5, 0.5, 45.0, 0.1, 0.1, 0.15, 0.15, 0.0, 0.0, 15.0};
    turned += defaultTail();
    dialog.setValues(turned);
    const double r = 0.1 * h / std::sqrt(2.0);
    QCOMPARE(hover(-r + side / 2, r + side / 2), Qt::SizeBDiagCursor);
    dialog.reject();
}

void TiltShiftTest::aDragPreviewsADraftThenTheFullPicture()
{
    pictura::PicturaMainWindow& frame = *window_;
    const int previous = frame.activeDocumentIndex();
    const QString path = dir_.filePath(QStringLiteral("large.png"));
    QVERIFY(pattern(1000, 700).save(path) && frame.openDocumentAtPath(path));
    {
        pictura::BlurGalleryDialog dialog(frame.activeView(), tiltShiftSpec());
        QList<double> wide = {1.0, 0.5, 0.5, 0.0, 0.1, 0.1, 0.15, 0.15, 0.0, 0.0, 176.0};
        wide += defaultTail();
        dialog.setValues(wide);
        dialog.resize(900, 600);
        dialog.show();
        QVERIFY(QTest::qWaitForWindowExposed(&dialog));
        QTRY_COMPARE(dialog.canvas()->image().width(), 1000);

        auto* canvas = dialog.canvas();
        const QPoint pin = canvas->rect().center();
        QTest::mousePress(canvas, Qt::LeftButton, Qt::NoModifier, pin);
        QVERIFY(canvas->dragging());
        QElapsedTimer draft;
        draft.start();
        QTest::mouseMove(canvas, pin + QPoint(0, 20));
        QTRY_VERIFY(canvas->image().width() <= 360);
        const qint64 draftMs = draft.elapsed();

        QElapsedTimer full;
        full.start();
        QTest::mouseRelease(canvas, Qt::LeftButton, Qt::NoModifier, pin + QPoint(0, 20));
        QVERIFY(!canvas->dragging());
        QTRY_COMPARE(canvas->image().width(), 1000);
        qInfo().noquote() << "tilt-shift preview: draft" << draftMs << "ms, full"
                          << full.elapsed() << "ms";
        QVERIFY(dialog.values().at(2) > 0.5);
        dialog.reject();
    }
    frame.closeDocument(frame.activeDocumentIndex(), false);
    QCOMPARE(frame.activeDocumentIndex(), previous);
}

void TiltShiftTest::previewKeepsTheBandSharp()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), tiltShiftSpec());
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(false);
    QCoreApplication::processEvents();
    const QImage unfiltered = dialog.canvas()->image();
    QVERIFY(!unfiltered.isNull());
    dialog.findChild<QCheckBox*>(QStringLiteral("bokehEnabled"))->setChecked(false);
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("tiltShiftSpin"))->setValue(30.0);
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(true);
    QTRY_VERIFY2(dialog.canvas()->image() != unfiltered, "the band did not blur the preview");
    const QImage blurred = dialog.canvas()->image();
    for (int x = 4; x < 92; x += 8) {
        QCOMPARE(blurred.pixel(x, 32), unfiltered.pixel(x, 32));
    }
    QVERIFY2(blurred.pixel(48, 1) != unfiltered.pixel(48, 1), "the top stayed sharp");
    QVERIFY2(blurred.pixel(48, 62) != unfiltered.pixel(48, 62), "the bottom stayed sharp");
    dialog.reject();
}

void TiltShiftTest::menuCommitsOnce()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int before = view->history_count();
    QList<double> committed;
    QTimer::singleShot(0, [&committed] {
        if (auto* dialog = openWorkspace()) {
            dialog->findChild<QDoubleSpinBox*>(QStringLiteral("tiltShiftSpin"))->setValue(22.0);
            committed = dialog->values();
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kTiltShiftPath)));
    QCOMPARE(committed.size(), 24);
    QCOMPARE(committed.at(10), 22.0);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Tilt-Shift"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("tilt-shift"));
    QCOMPARE(filter_last_params(*view), committed);
}

void TiltShiftTest::onlyTheActiveLayerBlurs()
{
    // A black box on its own layer over the patterned Background, in the
    // band's upper blur zone: the box feathers, the Background stays sharp.
    pictura::PicturaMainWindow& frame = *window_;
    pictura::PictureView* view = frame.activeView();
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    frame.selectLayerPath(QString::number(added));
    QVERIFY(view->select_rect(4, 4, 16, 12, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    const QRgb farBefore = view->composite_argb(80, 4);
    const QRgb edgeBefore = view->composite_argb(21, 8);

    QList<double> values = {1.0, 0.5, 0.5, 0.0, 0.1, 0.1, 0.15, 0.15, 0.0, 0.0, 10.0};
    values += {0.0, 14.0, 40.0, 40.0, 118.0, 0.0, 0.0, 50.0, 50.0, 0.0, 0.0, 0.0, 1.0};
    {
        pictura::BlurGalleryDialog dialog(view, tiltShiftSpec());
        dialog.setValues(values);
        dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(false);
        QCoreApplication::processEvents();
        const QImage unfiltered = dialog.canvas()->image();
        dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(true);
        QTRY_VERIFY(dialog.canvas()->image() != unfiltered);
        const QImage preview = dialog.canvas()->image();
        QCOMPARE(preview.pixel(80, 4), unfiltered.pixel(80, 4));
        QCOMPARE(preview.pixel(60, 60), unfiltered.pixel(60, 60));
        QVERIFY2(preview.pixel(21, 8) != unfiltered.pixel(21, 8), "the box edge did not feather");
        dialog.reject();
    }

    QTimer::singleShot(0, [&values] {
        if (auto* dialog = openWorkspace()) {
            dialog->setValues(values);
            dialog->accept();
        }
    });
    QVERIFY(frame.registry()->dispatch(pictura::commandIdForPath(kTiltShiftPath)));
    QCOMPARE(view->composite_argb(80, 4), farBefore);
    QVERIFY2(view->composite_argb(21, 8) != edgeBefore, "the commit kept the box edge hard");
    QCOMPARE(filter_last_kind(*view), QStringLiteral("tilt-shift"));
}

void TiltShiftTest::everyBlurRefusesAnEmptyLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view != nullptr);
    QVERIFY(filter_has_last(*view));
    const int added = view->add_layer(0);
    QVERIFY(added > 0);
    frame.selectLayerPath(QString::number(added));
    QVERIFY(filter_target_ready(*view));
    QVERIFY(filter_target_empty(*view));
    const int before = view->history_count();

    int blurs = 0;
    for (const pictura::FilterCommandSpec& spec : pictura::filterCommands()) {
        const QString family = spec.path.value(1);
        const bool gallery = family == QStringLiteral("Blur Gallery");
        if ((!gallery && family != QStringLiteral("Blur")) || !pictura::filter_kind_supported(spec.kind)) {
            continue;
        }
        ++blurs;
        QStringList errors;
        int others = 0;
        {
            const auto answer = answerEmptyError(&errors, &others);
            QVERIFY(frame.registry()->dispatch(pictura::commandIdForPath(spec.path)));
        }
        QCOMPARE(others, 0);
        QCOMPARE(errors.size(), 1);
        const QString reason = gallery ? QStringLiteral("the initial bounding rectangle is empty")
                                       : QStringLiteral("the selected area is empty");
        QCOMPARE(errors.first(), QStringLiteral("Could not complete the %1 command because %2.")
                                     .arg(spec.label, reason));
    }
    QVERIFY2(blurs >= 13, qPrintable(QString::number(blurs)));

    // Last Filter (Tilt-Shift, from the commit above) refuses the same way.
    QStringList errors;
    int others = 0;
    {
        const auto answer = answerEmptyError(&errors, &others);
        QVERIFY(frame.registry()->dispatch(
            QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    }
    QCOMPARE(others, 0);
    QCOMPARE(errors, QStringList{QStringLiteral(
                         "Could not complete the Tilt-Shift command because the initial "
                         "bounding rectangle is empty.")});
    QCOMPARE(view->history_count(), before);
}

void TiltShiftTest::aSelectionOverTransparencyIsEmpty()
{
    pictura::PicturaMainWindow& frame = *window_;
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY(!filter_target_empty(*view));
    QVERIFY(view->select_rect(40, 30, 10, 10, QStringLiteral("new"), 0.0));
    QVERIFY(filter_target_empty(*view));

    const QStringList gaussian = {QStringLiteral("Filter"), QStringLiteral("Blur"),
                                  QStringLiteral("Gaussian Blur")};
    QStringList errors;
    int others = 0;
    {
        const auto answer = answerEmptyError(&errors, &others);
        QVERIFY(frame.registry()->dispatch(pictura::commandIdForPath(gaussian)));
    }
    QCOMPARE(others, 0);
    QCOMPARE(errors, QStringList{QStringLiteral(
                         "Could not complete the Gaussian Blur command because the selected "
                         "area is empty.")});

    // Over the painted square the blur runs (its dialog opens).
    QVERIFY(view->select_rect(2, 2, 12, 12, QStringLiteral("new"), 0.0));
    QVERIFY(!filter_target_empty(*view));
    errors.clear();
    {
        const auto answer = answerEmptyError(&errors, &others);
        QVERIFY(frame.registry()->dispatch(pictura::commandIdForPath(gaussian)));
    }
    QVERIFY(errors.isEmpty());
    QCOMPARE(others, 1);
    view->deselect();
}

QTEST_MAIN(TiltShiftTest)

#include "tst_tilt_shift.moc"
