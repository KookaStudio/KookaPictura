// Select > Refine Edge (#297): the dialog previews the refined mask, OK applies
// once to the selection, Decontaminate forbids the in-place outputs, and a
// Layer Mask output writes the active layer's mask.

#include <QtTest/QtTest>
#include "api/layers_api.h"

#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QShortcut>
#include <QtWidgets/QApplication>
#include <cmath>
#include <cstddef>
#include <vector>

#include <rust/cxx.h>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "refine_edge_dialog.h"

#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolBar>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

namespace {

const QStringList kRefinePath = {QStringLiteral("Select"), QStringLiteral("Refine Edge…")};

::rust::Slice<const double> noDabs()
{
    static const double none[1] = {0.0};
    return ::rust::Slice<const double>(none, 0);
}

pictura::RefineEdgeDialog* activeRefineDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::RefineEdgeDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

} // namespace

class RefineEdgeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void previewsAndApplies();
    void cancelChangesNothing();
    void decontaminateForbidsInPlaceOutput();
    void layerMaskOutputWritesTheMask();
    void previewMatchesCommitWithinProxyTolerance();
    void brushDabsApplyThroughTheBridge();
    void brushBandPreviewMatchesTheEngine();
    void canvasDragAddsBrushDabs();
    void brushKeysToggleResizeAndShowEdge();
    void brushAltRestoresMomentarily();
    void cancelDiscardsBrushStrokes();
    void optionsBarButtonOpensIt();

private:
    // A white 40x40 document with a red square over (8, 8)-(30, 30).
    bool setupRedSquare();
    QString commandId() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
};

void RefineEdgeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void RefineEdgeTest::cleanup()
{
    QCoreApplication::processEvents();
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool RefineEdgeTest::setupRedSquare()
{
    if (!window_->newDocument(QStringLiteral("Refine"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    if (!view_) {
        return false;
    }
    // A red square on the white background: the selection gives the fill its
    // default mask, which is baked into the raster layer's alpha.
    if (!view_->select_rect(8, 8, 22, 22, QStringLiteral("new"), 0.0)) {
        return false;
    }
    const QString path = view_->add_solid_fill(0xffff0000u);
    if (path.isEmpty() || !view_->rasterize_fill_content(path)
        || !pictura::layer_mask_apply_path(*view_, path)) {
        return false;
    }
    view_->deselect();
    window_->selectLayerPath(path);
    return view_->sample_argb(20, 20) == 0xffff0000u
        && view_->sample_argb(35, 20) == 0xffffffffu;
}

QString RefineEdgeTest::commandId() const
{
    return pictura::commandIdForPath(kRefinePath);
}

void RefineEdgeTest::previewsAndApplies()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    const int before = view_->history_count();
    int previewWidth = 0;
    QTimer::singleShot(0, [&previewWidth] {
        pictura::RefineEdgeDialog* dialog = activeRefineDialog();
        if (!dialog) {
            return;
        }
        previewWidth = dialog->previewForTest().width();
        auto* radius = dialog->findChild<QSpinBox*>(QStringLiteral("refineRadius"));
        if (radius) {
            radius->setValue(4);
        }
        dialog->findChild<QDialogButtonBox*>()->button(QDialogButtonBox::Ok)->click();
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QCOMPARE(previewWidth, 40);
    QCOMPARE(view_->history_count(), before + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Refine Edge"));
    QVERIFY(view_->has_selection());
}

void RefineEdgeTest::cancelChangesNothing()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    const int before = view_->history_count();
    const int coverage = view_->selection_coverage(6, 6);
    QTimer::singleShot(0, [] {
        if (auto* dialog = activeRefineDialog()) {
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QCOMPARE(view_->history_count(), before);
    QCOMPARE(view_->selection_coverage(6, 6), coverage);
}

void RefineEdgeTest::decontaminateForbidsInPlaceOutput()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    int outputIndex = -1;
    QTimer::singleShot(0, [&outputIndex] {
        auto* dialog = activeRefineDialog();
        if (!dialog) {
            return;
        }
        auto* decon = dialog->findChild<QCheckBox*>(QStringLiteral("refineDecontaminate"));
        auto* output = dialog->findChild<QComboBox*>(QStringLiteral("refineOutput"));
        if (decon) {
            decon->setChecked(true);
        }
        if (output) {
            outputIndex = output->currentIndex();
        }
        dialog->reject();
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QVERIFY2(outputIndex >= 2, "Decontaminate forces a colour output");
    // The engine refuses an in-place target with decontamination.
    QVERIFY(!pictura::refine_edge_apply(*view_, false, 2, 0, 0, 0, 0, noDabs(), true, 50, 0));
}

void RefineEdgeTest::layerMaskOutputWritesTheMask()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    const int layer = 0;
    QVERIFY(!pictura::layers_api::rowAt(*view_, layer).hasMask);
    QTimer::singleShot(0, [] {
        auto* dialog = activeRefineDialog();
        if (!dialog) {
            return;
        }
        auto* output = dialog->findChild<QComboBox*>(QStringLiteral("refineOutput"));
        if (output) {
            output->setCurrentIndex(1);
        }
        dialog->findChild<QDialogButtonBox*>()->button(QDialogButtonBox::Ok)->click();
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QVERIFY2(pictura::layers_api::rowAt(*view_, layer).hasMask, "the active layer gained a mask");
}

void RefineEdgeTest::previewMatchesCommitWithinProxyTolerance()
{
    QVERIFY(window_->newDocument(QStringLiteral("PreviewScale"), 400, 400,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    const QString path = view_->add_solid_fill(0xffff0000u);
    if (!path.isEmpty()) {
        pictura::layer_mask_delete_path(*view_, path);
    }
    QVERIFY(!path.isEmpty());
    QVERIFY(view_->rasterize_fill_content(path));
    window_->selectLayerPath(path);
    QVERIFY(view_->select_rect(50, 50, 300, 300, QStringLiteral("new"), 0.0));

    // Radius alone is invisible on a flat composite (the band re-estimation
    // keeps the input), so add a Feather ramp: its width follows the proxy
    // scale and exposes a preview that forgot to scale the pixel distances.
    const int radius = 10;
    const int feather = 8;
    int pw = 0;
    int ph = 0;
    const ::rust::Vec<uint8_t> preview = pictura::refine_edge_preview(
        *view_, false, radius, 0, feather, 0, 0, noDabs(), 220, pw, ph);
    QVERIFY(pw > 0 && ph > 0);
    QCOMPARE(int(preview.size()), pw * ph);

    const int before = view_->history_count();
    QVERIFY(pictura::refine_edge_apply(*view_, false, radius, 0, feather, 0, 0, noDabs(), false, 0,
                                       0));
    QCOMPARE(view_->history_count(), before + 1);

    const auto firstHalf = [](auto sample, int width, int y) {
        for (int x = 0; x < width; ++x) {
            if (sample(x, y) >= 128) {
                return x;
            }
        }
        return -1;
    };
    // Width of the left transition: first visible coverage to full coverage.
    const auto transition = [](auto sample, int width, int y) {
        int lo = -1;
        for (int x = 0; x < width; ++x) {
            const int coverage = sample(x, y);
            if (coverage > 4 && lo < 0) {
                lo = x;
            }
            if (coverage >= 251) {
                return lo >= 0 ? x - lo : -1;
            }
        }
        return -1;
    };
    const auto previewAt = [&](int x, int y) {
        return int(preview.data()[size_t(y) * size_t(pw) + size_t(x)]);
    };
    const auto committedAt = [&](int x, int y) {
        return view_->selection_coverage(x * 400 / pw, y * 400 / ph);
    };
    int maxEdgeDifference = 0;
    int maxWidthDifference = 0;
    int compared = 0;
    for (int y = 10; y + 10 < ph; ++y) {
        const int previewEdge = firstHalf(previewAt, pw, y);
        const int committedEdge = firstHalf(committedAt, pw, y);
        const int previewWidth = transition(previewAt, pw, y);
        const int committedWidth = transition(committedAt, pw, y);
        if (previewEdge >= 0 && committedEdge >= 0 && previewWidth > 0 && committedWidth > 0) {
            maxEdgeDifference = qMax(maxEdgeDifference, qAbs(previewEdge - committedEdge));
            maxWidthDifference = qMax(maxWidthDifference, qAbs(previewWidth - committedWidth));
            ++compared;
        }
    }
    QVERIFY2(compared > 0, "the preview and the commit both have an edge to compare");
    QVERIFY2(maxEdgeDifference <= 2,
             qPrintable(QStringLiteral("edge differs by %1 proxy px").arg(maxEdgeDifference)));
    QVERIFY2(maxWidthDifference <= 2,
             qPrintable(QStringLiteral("feather width differs by %1 proxy px")
                            .arg(maxWidthDifference)));
}


void RefineEdgeTest::brushDabsApplyThroughTheBridge()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 20, 20, QStringLiteral("new"), 0.0));
    // The image edge at x=30 is outside the initial selection (which ends at
    // x=26), so only a dab can put coverage there.
    const std::vector<double> values = {30.0, 20.0, 8.0, 0.0};
    const ::rust::Slice<const double> dabs(values.data(), values.size());
    const auto at = [](const ::rust::Vec<uint8_t>& data, int w, int x, int y) {
        return int(data.data()[size_t(y) * size_t(w) + size_t(x)]);
    };

    int width = 0;
    int height = 0;
    const ::rust::Vec<uint8_t> plain = pictura::refine_edge_preview(
        *view_, false, 0, 0, 0, 0, 0, noDabs(), 40, width, height);
    QVERIFY2(width == 40 && height == 40, "the full-size preview");
    QVERIFY2(at(plain, width, 30, 20) == 0, "the unrefined selection stops before x=30");

    const ::rust::Vec<uint8_t> preview = pictura::refine_edge_preview(
        *view_, false, 0, 0, 0, 0, 0, dabs, 40, width, height);
    QVERIFY2(at(preview, width, 30, 20) > 0, "the dab refines the image edge");

    // The same stroke at half size, before the commit replaces the selection.
    int pw = 0;
    int ph = 0;
    const ::rust::Vec<uint8_t> proxy = pictura::refine_edge_preview(
        *view_, false, 0, 0, 0, 0, 0, dabs, 20, pw, ph);
    QVERIFY(pw == 20 && ph == 20);
    int proxyRight = -1;
    for (int x = 0; x < pw; ++x) {
        if (proxy.data()[size_t(10) * size_t(pw) + size_t(x)] > 0) {
            proxyRight = x;
        }
    }
    QVERIFY2(proxyRight >= 0, "the proxy sees the refined edge");

    const int before = view_->history_count();
    QVERIFY(pictura::refine_edge_apply(*view_, false, 0, 0, 0, 0, 0, dabs, false, 0, 0));
    QCOMPARE(view_->history_count(), before + 1);
    QVERIFY2(view_->selection_coverage(30, 20) > 0, "the commit applies the dab");
    int committedRight = -1;
    for (int x = 0; x < 40; ++x) {
        if (view_->selection_coverage(x, 20) > 0) {
            committedRight = x;
        }
    }
    QVERIFY2(committedRight >= 0, "the commit refines the same area");
    QVERIFY2(std::abs(proxyRight - committedRight / 2) <= 2,
             "the proxy and commit agree within two proxy pixels");
}

void RefineEdgeTest::brushBandPreviewMatchesTheEngine()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    const std::vector<double> values = {30.0, 20.0, 8.0, 0.0};
    const ::rust::Slice<const double> dabs(values.data(), values.size());
    int plainWidth = 0;
    int plainHeight = 0;
    const ::rust::Vec<uint8_t> plain = pictura::refine_edge_band_preview(
        *view_, false, 0, noDabs(), 40, plainWidth, plainHeight);
    QVERIFY(plainWidth == 40 && plainHeight == 40);
    QVERIFY2(int(plain.data()[size_t(20) * size_t(plainWidth) + size_t(30)]) == 0,
             "a zero Radius with no dabs has no band at the image edge");

    int width = 0;
    int height = 0;
    const ::rust::Vec<uint8_t> band =
        pictura::refine_edge_band_preview(*view_, false, 0, dabs, 40, width, height);
    QVERIFY(width == 40 && height == 40);
    const auto bandAt = [&](int x, int y) {
        return int(band.data()[size_t(y) * size_t(width) + size_t(x)]);
    };
    QVERIFY2(bandAt(30, 20) == 255, "the brush-expanded band reaches the image edge");
    QVERIFY2(bandAt(0, 0) == 0, "far from the band");
}

void RefineEdgeTest::canvasDragAddsBrushDabs()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    const int before = view_->history_count();
    const int edgeBefore = view_->selection_coverage(30, 20);
    const int farBefore = view_->selection_coverage(5, 5);
    QCOMPARE(edgeBefore, 255);
    QCOMPARE(farBefore, 0);
    bool ringShown = false;
    bool dabsRecorded = false;
    bool previewChanged = false;
    bool historyBeforeOk = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeRefineDialog();
        auto* canvas = window_->imageView();
        if (!dialog || !canvas) {
            return;
        }
        if (auto* size = dialog->findChild<QSpinBox*>(QStringLiteral("refineBrushSize"))) {
            size->setValue(20);
        }
        const QPoint hover = canvas->imageToWidget(QPointF(30.0, 20.0)).toPoint();
        QTest::mouseMove(canvas, hover);
        ringShown = canvas->hasBrushOutline()
            && std::abs(canvas->brushOutlineDiameterForTest() - 20.0) < 0.001;
        const QImage beforePreview = dialog->previewForTest();
        QTest::mousePress(canvas, Qt::LeftButton, Qt::NoModifier, hover);
        QTest::mouseMove(canvas, canvas->imageToWidget(QPointF(30.0, 24.0)).toPoint());
        QTest::mouseRelease(canvas, Qt::LeftButton, Qt::NoModifier,
                            canvas->imageToWidget(QPointF(30.0, 26.0)).toPoint());
        dabsRecorded = !dialog->dabsForTest().isEmpty();
        previewChanged = dialog->previewForTest() != beforePreview;
        historyBeforeOk = view_->history_count() == before;
        dialog->findChild<QDialogButtonBox*>()->button(QDialogButtonBox::Ok)->click();
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QVERIFY2(ringShown, "hovering shows the brush ring at the brush size");
    QVERIFY2(dabsRecorded, "the drag records refinement dabs");
    QVERIFY2(previewChanged, "the stroke updates the refined preview");
    QVERIFY2(historyBeforeOk, "strokes are preview-only before OK");
    QCOMPARE(view_->history_count(), before + 1);
    QVERIFY2(view_->selection_coverage(30, 20) < edgeBefore,
             "the committed dab lowers the coverage under the stroke");
    QCOMPARE(view_->selection_coverage(5, 5), farBefore);
}

void RefineEdgeTest::brushKeysToggleResizeAndShowEdge()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    bool latched = false;
    bool unlatched = false;
    bool resized = false;
    bool ringShown = false;
    bool edgeShown = false;
    bool greenSeen = false;
    bool greenCleared = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeRefineDialog();
        auto* canvas = window_->imageView();
        if (!dialog || !canvas) {
            return;
        }
        auto* restore = dialog->findChild<QCheckBox*>(QStringLiteral("refineBrushRestore"));
        auto* size = dialog->findChild<QSpinBox*>(QStringLiteral("refineBrushSize"));
        auto* show = dialog->findChild<QCheckBox*>(QStringLiteral("refineShowRadius"));
        auto* radius = dialog->findChild<QSpinBox*>(QStringLiteral("refineRadius"));
        if (!restore || !size || !show || !radius) {
            return;
        }
        radius->setValue(4);
        const int sizeBefore = size->value();
        // Invoke the installed shortcuts directly: the offscreen platform does
        // not activate a second window, so QTest::keyClick never reaches the
        // application-scoped shortcut map. The key sequence itself is asserted
        // by finding the shortcut whose key matches.
        const auto fire = [&](const QKeySequence& key) {
            for (QShortcut* shortcut : dialog->findChildren<QShortcut*>()) {
                if (shortcut->key() == key) {
                    QMetaObject::invokeMethod(shortcut, "activated", Qt::DirectConnection);
                    return true;
                }
            }
            return false;
        };
        const QKeySequence latchKey(QStringLiteral("Shift+E"));
        latched = fire(latchKey) && restore->isChecked();
        unlatched = fire(latchKey) && !restore->isChecked();
        QTest::mouseMove(canvas, canvas->imageToWidget(QPointF(30.0, 20.0)).toPoint());
        resized = fire(QKeySequence(Qt::Key_BracketRight)) && size->value() > sizeBefore;
        // Re-assert the hover at the new size; the ring is drawn on hover.
        QTest::mouseMove(canvas, canvas->imageToWidget(QPointF(30.0, 24.0)).toPoint());
        ringShown = canvas->hasBrushOutline()
            && std::abs(canvas->brushOutlineDiameterForTest() - double(size->value())) < 0.001;
        edgeShown = fire(QKeySequence(Qt::Key_J)) && show->isChecked();
        const QImage overlay = canvas->refinePreviewForTest();
        for (int y = 0; y < overlay.height() && !greenSeen; ++y) {
            for (int x = 0; x < overlay.width(); ++x) {
                const QColor pixel = overlay.pixelColor(x, y);
                if (pixel.green() > 0 && pixel.red() == 0) {
                    greenSeen = true;
                    break;
                }
            }
        }
        fire(QKeySequence(Qt::Key_J));
        const QImage cleared = canvas->refinePreviewForTest();
        bool anyGreen = false;
        for (int y = 0; y < cleared.height() && !anyGreen; ++y) {
            for (int x = 0; x < cleared.width(); ++x) {
                const QColor pixel = cleared.pixelColor(x, y);
                if (pixel.green() > 0 && pixel.red() == 0) {
                    anyGreen = true;
                    break;
                }
            }
        }
        greenCleared = !show->isChecked() && !anyGreen;
        dialog->reject();
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QVERIFY2(latched && unlatched, "Shift+E latches and unlatches restore");
    QVERIFY2(resized, "] increases the brush size");
    QVERIFY2(ringShown, "the hover ring follows the brush size");
    QVERIFY2(edgeShown, "J toggles Show Edge");
    QVERIFY2(greenSeen, "Show Edge paints the band on the canvas overlay");
    QVERIFY2(greenCleared, "J clears the band again");
}

void RefineEdgeTest::brushAltRestoresMomentarily()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    bool altErase = false;
    bool plainRefine = false;
    bool latchErase = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeRefineDialog();
        auto* canvas = window_->imageView();
        if (!dialog || !canvas) {
            return;
        }
        const QPoint p1 = canvas->imageToWidget(QPointF(28.0, 18.0)).toPoint();
        QTest::mousePress(canvas, Qt::LeftButton, Qt::AltModifier, p1);
        QTest::mouseRelease(canvas, Qt::LeftButton, Qt::AltModifier, p1);
        const QVector<pictura::RefineBrushDab> first = dialog->dabsForTest();
        altErase = !first.isEmpty() && first.last().erase;

        const QPoint p2 = canvas->imageToWidget(QPointF(28.0, 22.0)).toPoint();
        QTest::mousePress(canvas, Qt::LeftButton, Qt::NoModifier, p2);
        QTest::mouseRelease(canvas, Qt::LeftButton, Qt::NoModifier, p2);
        const QVector<pictura::RefineBrushDab> second = dialog->dabsForTest();
        plainRefine = second.size() > first.size() && !second.last().erase;

        if (auto* restore = dialog->findChild<QCheckBox*>(QStringLiteral("refineBrushRestore"))) {
            restore->setChecked(true);
        }
        const QPoint p3 = canvas->imageToWidget(QPointF(28.0, 26.0)).toPoint();
        QTest::mousePress(canvas, Qt::LeftButton, Qt::NoModifier, p3);
        QTest::mouseRelease(canvas, Qt::LeftButton, Qt::NoModifier, p3);
        const QVector<pictura::RefineBrushDab> third = dialog->dabsForTest();
        latchErase = third.size() > second.size() && third.last().erase;
        dialog->reject();
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QVERIFY2(altErase, "Alt makes the stroke restore");
    QVERIFY2(plainRefine, "without Alt the next stroke refines");
    QVERIFY2(latchErase, "a latched restore applies without Alt");
}

void RefineEdgeTest::cancelDiscardsBrushStrokes()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    const int before = view_->history_count();
    const int coverage = view_->selection_coverage(6, 6);
    QImage beforePreview;
    bool previewChanged = false;
    bool previewCleared = false;
    bool ringCleared = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeRefineDialog();
        auto* canvas = window_->imageView();
        if (!dialog || !canvas) {
            return;
        }
        beforePreview = dialog->previewForTest();
        const QPoint p = canvas->imageToWidget(QPointF(30.0, 20.0)).toPoint();
        QTest::mousePress(canvas, Qt::LeftButton, Qt::NoModifier, p);
        QTest::mouseMove(canvas, canvas->imageToWidget(QPointF(30.0, 24.0)).toPoint());
        QTest::mouseRelease(canvas, Qt::LeftButton, Qt::NoModifier,
                            canvas->imageToWidget(QPointF(30.0, 26.0)).toPoint());
        previewChanged = dialog->previewForTest() != beforePreview;
        dialog->reject();
        previewCleared = !canvas->hasRefinePreviewForTest();
        ringCleared = !canvas->hasBrushOutline();
    });
    QVERIFY(window_->registry()->dispatch(commandId()));
    QCOMPARE(view_->history_count(), before);
    QCOMPARE(view_->selection_coverage(6, 6), coverage);
    QVERIFY2(previewChanged, "the stroke updates the refined preview");
    QVERIFY2(previewCleared, "Cancel clears the refine overlay");
    QVERIFY2(ringCleared, "Cancel clears the brush ring");
}

void RefineEdgeTest::optionsBarButtonOpensIt()
{
    QVERIFY(setupRedSquare());
    auto* bar = window_->findChild<QToolBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* button = bar->findChild<QToolButton*>(QStringLiteral("optionsRefineEdge"));
    QVERIFY2(button, "the options bar carries a Select and Mask button");
    const auto menuEnabled = [&]() {
        for (const pictura::CommandInfo& info : window_->registry()->describe()) {
            if (info.id == commandId()) {
                return info.enabled;
            }
        }
        return false;
    };
    // The button mirrors the Refine Edge command: disabled with no selection,
    // enabled once one exists.
    QVERIFY2(!button->isEnabled(), "Select and Mask is disabled without a selection");
    QVERIFY2(!menuEnabled(), "the menu command is disabled without a selection");
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    QTRY_VERIFY2(button->isEnabled(), "Select and Mask enables with a selection");
    QTRY_VERIFY2(menuEnabled(), "the menu command enables with a selection");
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (auto* dialog = activeRefineDialog()) {
            opened = true;
            dialog->reject();
        }
    });
    button->click();
    QVERIFY2(opened, "clicking Select and Mask opens the Refine Edge dialog");
}

PICTURA_QT_TEST_MAIN(RefineEdgeTest)
#include "tst_refine_edge.moc"
