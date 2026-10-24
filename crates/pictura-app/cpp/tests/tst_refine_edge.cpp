// Select > Refine Edge (#297): the dialog previews the refined mask, OK applies
// once to the selection, Decontaminate forbids the in-place outputs, and a
// Layer Mask output writes the active layer's mask.

#include <QtTest/QtTest>
#include "api/layers_api.h"

#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <cstddef>

#include "commands.h"
#include "frame.h"
#include "refine_edge_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_selection/refine.cxxqt.h"

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
    // A raster pixel layer so the Layer Mask output has an active layer to write.
    const QString path = view_->add_solid_fill(0xffff0000u);
    if (path.isEmpty() || !view_->rasterize_fill_content(path)) {
        return false;
    }
    window_->selectLayerPath(path);
    return view_->sample_argb(20, 20) == 0xffff0000u;
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
    QVERIFY(!pictura::refine_edge_apply(*view_, false, 2, 0, 0, 0, 0, true, 50, 0));
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
        *view_, false, radius, 0, feather, 0, 0, 220, pw, ph);
    QVERIFY(pw > 0 && ph > 0);
    QCOMPARE(int(preview.size()), pw * ph);

    const int before = view_->history_count();
    QVERIFY(pictura::refine_edge_apply(*view_, false, radius, 0, feather, 0, 0, false, 0, 0));
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

void RefineEdgeTest::optionsBarButtonOpensIt()
{
    QVERIFY(setupRedSquare());
    auto* bar = window_->findChild<QToolBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* button = bar->findChild<QToolButton*>(QStringLiteral("optionsRefineEdge"));
    QVERIFY2(button, "the options bar carries a Select and Mask button");
    // The button mirrors the Refine Edge command: disabled with no selection,
    // enabled once one exists.
    QVERIFY2(!button->isEnabled(), "Select and Mask is disabled without a selection");
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    QTRY_VERIFY2(button->isEnabled(), "Select and Mask enables with a selection");
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

QTEST_MAIN(RefineEdgeTest)
#include "tst_refine_edge.moc"
