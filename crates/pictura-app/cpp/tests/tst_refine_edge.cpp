// Select > Refine Edge (#297): the dialog previews the refined mask, OK applies
// once to the selection, Decontaminate forbids the in-place outputs, and a
// Layer Mask output writes the active layer's mask.

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtWidgets/QApplication>

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
    QVERIFY(!view_->layer_row_has_mask(layer));
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
    QVERIFY2(view_->layer_row_has_mask(layer), "the active layer gained a mask");
}

void RefineEdgeTest::optionsBarButtonOpensIt()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(6, 6, 26, 26, QStringLiteral("new"), 0.0));
    auto* bar = window_->findChild<QToolBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* button = bar->findChild<QToolButton*>(QStringLiteral("optionsRefineEdge"));
    QVERIFY2(button, "the options bar carries a Select and Mask button");
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
