#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kShearPath = {QStringLiteral("Filter"), QStringLiteral("Distort"),
                                QStringLiteral("Shear")};

const pictura::FilterCommandSpec& shearSpec()
{
    return *pictura::filterCommandForPath(kShearPath);
}

pictura::FilterPreviewDialog* activeFilterDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::FilterPreviewDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

QWidget* curveBox(pictura::FilterPreviewDialog& dialog)
{
    return dialog.findChild<QWidget*>(QStringLiteral("shearCurveWidget"));
}

// The bottom end of the curve box, dragged to the right edge: offset +1.
void bendTheCurve(QWidget* curve)
{
    const QPoint bottom(curve->width() / 2, curve->height() - 1);
    const QPoint right(curve->width() - 1, curve->height() - 1);
    QTest::mousePress(curve, Qt::LeftButton, Qt::NoModifier, bottom);
    QTest::mouseMove(curve, right);
    QTest::mouseRelease(curve, Qt::LeftButton, Qt::NoModifier, right);
}

// The `shear` slots: a point count, then up to `kShearMaxPoints`
// `(position, offset)` pairs, then the fill.
QList<double> controlPointSlots()
{
    QList<double> values = {3.0, -1.0, -0.5, 0.0, 0.0, 1.0, 0.5};
    while (values.size() < 1 + 2 * pictura::kShearMaxPoints) {
        values.append(0.0);
    }
    values.append(1.0);
    return values;
}

} // namespace

class ShearDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void layoutFollowsCs6();
    void curveBoxEditsTheControlPoints();
    void theCurveKeepsAFewControlPoints();
    void dragUpdatesThePaneBeforeTheCanvas();
    void controlPointsRoundTripThroughSlots();
    void menuOpensItAndCommits();
    void rejectRestoresPixels();
    void lastFilterSettingsReopensPrefilled();
    void paneKeepsTheOtherLayersUnsheared();
    void paneShowsTheBlendMode();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ShearDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // A patterned picture, so a shear visibly moves something.
    QImage pattern(64, 64, QImage::Format_RGB32);
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 64; ++x) {
            pattern.setPixelColor(x, y, QColor((x * 4) % 256, (y * 4) % 256, (x + y) % 256));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kShearPath) != nullptr);
}

void ShearDialogTest::layoutFollowsCs6()
{
    const pictura::FilterCommandSpec& spec = shearSpec();
    QVERIFY(spec.previewBelow);
    pictura::FilterPreviewDialog dialog(window_->activeView(), spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Shear"));

    QWidget* curve = curveBox(dialog);
    QVERIFY(curve != nullptr);
    QCOMPARE(curve->size(), QSize(140, 140));

    // The straight default is two end points, not a lattice of samples.
    const QList<double> values = dialog.values();
    QCOMPARE(values.size(), 1 + 2 * pictura::kShearMaxPoints + 1);
    QCOMPARE(values.at(0), 2.0);
    QCOMPARE(values.at(1), -1.0);
    QCOMPARE(values.at(2), 0.0);
    QCOMPARE(values.at(3), 1.0);
    QCOMPARE(values.at(4), 0.0);
    QCOMPARE(values.last(), 0.0);

    const QList<QRadioButton*> radios = dialog.findChildren<QRadioButton*>();
    QStringList labels;
    for (QRadioButton* radio : radios) {
        labels.append(radio->text());
    }
    QCOMPARE(labels, QStringList({QStringLiteral("Wrap Around"),
                                  QStringLiteral("Repeat Edge Pixels")}));
    QVERIFY(radios.first()->isChecked());

    // CS6's Shear has no Preview checkbox and no zoom controls; the picture
    // sits under the curve box.
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview")) == nullptr);
    QVERIFY(dialog.findChild<QToolButton*>(QStringLiteral("filterZoomIn")) == nullptr);
    auto* preview = dialog.findChild<QLabel*>(QStringLiteral("filterThumbnail"));
    QVERIFY(preview != nullptr);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    const QRect curveRect(curve->mapTo(&dialog, QPoint(0, 0)), curve->size());
    const QRect previewRect(preview->mapTo(&dialog, QPoint(0, 0)), preview->size());
    QVERIFY2(previewRect.top() >= curveRect.bottom(), "the preview must sit below the curve box");
}

void ShearDialogTest::curveBoxEditsTheControlPoints()
{
    pictura::FilterPreviewDialog dialog(window_->activeView(), shearSpec());
    QWidget* curve = curveBox(dialog);
    QVERIFY(curve != nullptr);

    // Drag the bottom end to the right: only its offset changes.
    bendTheCurve(curve);
    const QList<double> bent = dialog.values();
    QCOMPARE(bent.at(0), 2.0);
    QCOMPARE(bent.at(1), -1.0);
    QCOMPARE(bent.at(2), 0.0);
    QCOMPARE(bent.at(3), 1.0);
    QCOMPARE(bent.at(4), 1.0);

    // Click the line to add a point, then drag it out of the box: the curve
    // goes back to what it was. The bent line at mid-height sits at 3/4 width.
    const QPoint midway((curve->width() - 1) * 3 / 4, curve->height() / 2);
    QTest::mouseClick(curve, Qt::LeftButton, Qt::NoModifier, midway);
    const QList<double> withPoint = dialog.values();
    QCOMPARE(withPoint.at(0), 3.0);
    QVERIFY(withPoint.at(1) < withPoint.at(5));

    QTest::mousePress(curve, Qt::LeftButton, Qt::NoModifier, midway);
    QTest::mouseMove(curve, QPoint(-30, -30));
    QTest::mouseRelease(curve, Qt::LeftButton, Qt::NoModifier, QPoint(-30, -30));
    QCOMPARE(dialog.values(), bent);
}

void ShearDialogTest::theCurveKeepsAFewControlPoints()
{
    pictura::FilterPreviewDialog dialog(window_->activeView(), shearSpec());
    QWidget* curve = curveBox(dialog);
    QVERIFY(curve != nullptr);

    // A clicked point is stored as one point, not sampled into a lattice: the
    // curve box must show the handful the user placed, like CS6's. The bend
    // puts the line's midpoint at 3/4 width, where the click lands on it.
    bendTheCurve(curve);
    const QPoint middle((curve->width() - 1) * 3 / 4, curve->height() / 2);
    QTest::mouseClick(curve, Qt::LeftButton, Qt::NoModifier, middle);
    const QList<double> values = dialog.values();
    QCOMPARE(values.at(0), 3.0);
    QVERIFY(values.at(4) > 0.4); // the new point's offset, on the bent line
    for (int i = 7; i < 1 + 2 * pictura::kShearMaxPoints; ++i) {
        QCOMPARE(values.at(i), 0.0);
    }

    // A click away from the rendered line is ignored, as CS6's is: only the
    // line adds a point.
    QTest::mouseClick(curve, Qt::LeftButton, Qt::NoModifier, QPoint(5, curve->height() / 2));
    QCOMPARE(dialog.values().at(0), 3.0);

    // The cap refuses further points instead of overflowing the slots. Each
    // click lands on the line, which the points on it keep straight.
    for (int i = 0; i < 12; ++i) {
        const int y = 20 + i * 8;
        const int x = static_cast<int>((static_cast<double>(y) / (curve->height() - 1) + 1.0)
                                       / 2.0 * (curve->width() - 1));
        QTest::mouseClick(curve, Qt::LeftButton, Qt::NoModifier, QPoint(x, y));
    }
    QCOMPARE(dialog.values().at(0), static_cast<double>(pictura::kShearMaxPoints));
}

void ShearDialogTest::dragUpdatesThePaneBeforeTheCanvas()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    pictura::FilterPreviewDialog dialog(view, shearSpec());
    dialog.applyInitial(QList<double>());
    QWidget* curve = curveBox(dialog);
    auto* pane = dialog.findChild<QLabel*>(QStringLiteral("filterThumbnail"));
    QVERIFY(curve != nullptr && pane != nullptr);

    const QImage paneBefore = pane->pixmap().toImage();
    const QImage canvasBefore = view->image();

    // Mid-drag: the dialog's own pane follows the proxy render, but the
    // whole-layer canvas preview must wait for the release.
    const QPoint bottom(curve->width() / 2, curve->height() - 1);
    const QPoint right(curve->width() - 1, curve->height() - 1);
    QTest::mousePress(curve, Qt::LeftButton, Qt::NoModifier, bottom);
    QTest::mouseMove(curve, right);
    QVERIFY2(pane->pixmap().toImage() != paneBefore, "the pane did not follow the drag");
    QCOMPARE(view->image(), canvasBefore);

    QTest::mouseRelease(curve, Qt::LeftButton, Qt::NoModifier, right);
    QTRY_VERIFY2(view->image() != canvasBefore, "the canvas preview did not run on release");
}

void ShearDialogTest::controlPointsRoundTripThroughSlots()
{
    pictura::FilterPreviewDialog dialog(window_->activeView(), shearSpec());
    const QList<double> expected = controlPointSlots();
    dialog.applyInitial(expected);

    const QList<double> got = dialog.values();
    QCOMPARE(got.size(), expected.size());
    for (int i = 0; i < expected.size(); ++i) {
        QVERIFY2(qAbs(got.at(i) - expected.at(i)) < 1e-9,
                 qPrintable(QStringLiteral("slot %1: %2 vs %3")
                                .arg(i)
                                .arg(got.at(i))
                                .arg(expected.at(i))));
    }
    const QList<QRadioButton*> radios = dialog.findChildren<QRadioButton*>();
    QCOMPARE(radios.size(), 2);
    QVERIFY(radios.at(1)->isChecked());
}

void ShearDialogTest::menuOpensItAndCommits()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = activeFilterDialog()) {
            if (QWidget* curve = curveBox(*dialog)) {
                bendTheCurve(curve);
            }
            seen = dialog->values();
            dialog->accept();
        }
    });
    const int before = view->history_count();
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kShearPath)));
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("shear"));
    const QList<double> last = filter_last_params(*view);
    QCOMPARE(last, seen);
    QCOMPARE(last.size(), 1 + 2 * pictura::kShearMaxPoints + 1);
    QCOMPARE(last.at(4), 1.0);
}

void ShearDialogTest::rejectRestoresPixels()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QImage before = view->image();
    pictura::FilterPreviewDialog dialog(view, shearSpec());
    QWidget* curve = curveBox(dialog);
    QVERIFY(curve != nullptr);
    bendTheCurve(curve);
    QTRY_VERIFY2(view->image() != before, "the canvas preview did not run");
    dialog.reject();
    QCOMPARE(view->image(), before);
}

void ShearDialogTest::lastFilterSettingsReopensPrefilled()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QList<double> expected = controlPointSlots();
    QVERIFY(apply_filter_params(*view, QStringLiteral("shear"), expected));

    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = activeFilterDialog()) {
            seen = dialog->values();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(seen.size(), expected.size());
    for (int i = 0; i < expected.size(); ++i) {
        QVERIFY2(qAbs(seen.at(i) - expected.at(i)) < 1e-9,
                 qPrintable(QStringLiteral("slot %1: %2 vs %3")
                                .arg(i)
                                .arg(seen.at(i))
                                .arg(expected.at(i))));
    }
}

// The pane shows the picture with the filter applied to the active layer
// alone. A partial copy makes the stack a full background layer plus a
// selected-region top layer, so the flattened composite the pane used to
// re-filter is not the picture: only the active top layer may shear, and the
// background must stay put under it.
void ShearDialogTest::paneKeepsTheOtherLayersUnsheared()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);

    // Single layer: the pane is the sheared picture.
    pictura::FilterPreviewDialog one(view, shearSpec());
    QWidget* oneCurve = curveBox(one);
    auto* onePane = one.findChild<QLabel*>(QStringLiteral("filterThumbnail"));
    QVERIFY(oneCurve != nullptr && onePane != nullptr);
    bendTheCurve(oneCurve);
    const QImage single = onePane->pixmap().toImage();
    QVERIFY(!single.isNull());
    one.reject();

    view->select_rect(0, 0, 32, 32, QStringLiteral("new"), 0.0);
    const QString top = view->layer_via_copy(QStringLiteral("0"));
    QVERIFY(!top.isEmpty());
    view->set_active_layer(top);
    view->deselect();

    pictura::FilterPreviewDialog two(view, shearSpec());
    QWidget* twoCurve = curveBox(two);
    auto* twoPane = two.findChild<QLabel*>(QStringLiteral("filterThumbnail"));
    QVERIFY(twoCurve != nullptr && twoPane != nullptr);
    bendTheCurve(twoCurve);
    const QImage stacked = twoPane->pixmap().toImage();
    two.reject();

    QVERIFY2(single != stacked, "the pane filtered every layer, not just the active one");
}

void ShearDialogTest::paneShowsTheBlendMode()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QImage gray(64, 64, QImage::Format_RGB32);
    gray.fill(QColor(128, 128, 128));
    const QString path = dir_.filePath(QStringLiteral("gray.png"));
    QVERIFY(gray.save(path) && window->openDocumentAtPath(path));
    pictura::PictureView* view = window->activeView();
    QVERIFY(view != nullptr);
    view->select_rect(0, 0, 64, 64, QStringLiteral("new"), 0.0);
    const QString top = view->layer_via_copy(QStringLiteral("0"));
    QVERIFY(!top.isEmpty());
    view->set_active_layer(top);
    view->deselect();
    QVERIFY(view->set_layer_blend(top.toInt(), QStringLiteral("mul ")));

    pictura::FilterPreviewDialog dialog(view, shearSpec());
    QWidget* curve = curveBox(dialog);
    auto* pane = dialog.findChild<QLabel*>(QStringLiteral("filterThumbnail"));
    QVERIFY(curve != nullptr && pane != nullptr);
    bendTheCurve(curve);
    const QImage shown = pane->pixmap().toImage();
    QVERIFY(!shown.isNull());
    const int red = shown.pixelColor(shown.rect().center()).red();
    dialog.reject();
    QVERIFY2(red < 80, qPrintable(QStringLiteral("gray multiplied by itself, got %1").arg(red)));
}

QTEST_MAIN(ShearDialogTest)

#include "tst_shear_dialog.moc"
