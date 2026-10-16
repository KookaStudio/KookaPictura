// The Layers panel's mask commands (#668): the row menu enables only the mask
// rows that apply, the mask button walks Add layer mask → Add vector mask →
// Add a mask, and the mask thumbnail's menu (whose selection commands
// tst_mask_edit covers) toggles, deletes and applies the mask, refines it, and
// sets the rubylith overlay's display options. The overlay is display only. The
// vector-mask thumbnail has its own menu, and the trash deletes a doubly-masked
// layer's vector mask after a "Delete Vector Mask?" prompt (#480).

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "mask_display_options_dialog.h"
#include "panels/layers_panel.h"
#include "refine_edge_dialog.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/mask_commands.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"

#include "qt_test_support.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QTreeView>

class LayersMaskCommandsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void rowMenuFollowsTheMaskState();
    void maskButtonFollowsTheMasks();
    void maskMenuRows();
    void maskMenuActions();
    void applyNeedsAPixelLayer();
    void refineMask();
    void refineMaskOnAFillLayer();
    void rubylithOverlay();
    void rubylithEndsWithItsLayer();
    void maskOptions();
    void vectorMaskMenu();
    void trashDeletesTheVectorMask();
    void trashActsOnTheSelectedRow();

private:
    // A white `side`² document with a rasterized fill layer selected; returns
    // the layer's path, empty on failure.
    QString fillLayer(int side);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::LayersPanel* panel_ = nullptr;
};

void LayersMaskCommandsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // Shown and wide, so a row lays out the layer, mask and vector thumbnails
    // inside the tree's viewport.
    window_->resize(1800, 1000);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel_ != nullptr);
}

void LayersMaskCommandsTest::cleanup()
{
    panel_->setDialogDriverForTest({});
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

QString LayersMaskCommandsTest::fillLayer(int side)
{
    pictura::PicturaMainWindow& frame = *window_;
    if (!frame.newDocument(QStringLiteral("Masks"), side, side, QStringLiteral("rgb"), 8,
                           QStringLiteral("white"))) {
        return {};
    }
    pictura::PictureView* view = frame.activeView();
    const QString path = view ? view->add_solid_fill(0xff2244aau) : QString();
    if (path.isEmpty() || !view->rasterize_fill_content(path)) {
        return {};
    }
    frame.selectLayerPath(path);
    QCoreApplication::processEvents();
    return path;
}

// lmc_row_menu_enabled_mask, lmc_row_menu_states
void LayersMaskCommandsTest::rowMenuFollowsTheMaskState()
{
    const QString path = fillLayer(16);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    const auto expect = [&](bool add, bool del, bool enable, bool disable) {
        QCOMPARE(panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Add Layer Mask")), add);
        QCOMPARE(panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Delete Layer Mask")),
                 del);
        QCOMPARE(panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Enable Layer Mask")),
                 enable);
        QCOMPARE(panel_->rowMenuEnabledForPathForTest(path, QStringLiteral("Disable Layer Mask")),
                 disable);
    };
    expect(true, false, false, false);
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-all")));
    expect(false, true, false, true);
    QVERIFY(pictura::layer_mask_set_enabled_path(*view, path, false));
    expect(false, true, true, false);
    QVERIFY(pictura::layer_mask_delete_path(*view, path));
    expect(true, false, false, false);
}

// lmc_mask_button
void LayersMaskCommandsTest::maskButtonFollowsTheMasks()
{
    const QString path = fillLayer(16);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QCOMPARE(panel_->maskButtonToolTipForTest(), QStringLiteral("Add layer mask"));
    QVERIFY(panel_->maskButtonEnabledForTest());

    panel_->clickMaskButtonForTest();
    QCoreApplication::processEvents();
    QVERIFY2(pictura::layer_mask_present(*view), "the first click adds a layer mask");
    QVERIFY(!pictura::vector_mask_present(*view));
    QCOMPARE(panel_->maskButtonToolTipForTest(), QStringLiteral("Add vector mask"));

    panel_->clickMaskButtonForTest();
    QCoreApplication::processEvents();
    QVERIFY2(pictura::vector_mask_present(*view), "the second click adds a vector mask");
    QCOMPARE(panel_->maskButtonToolTipForTest(), QStringLiteral("Add a mask"));
    QVERIFY2(!panel_->maskButtonEnabledForTest(), "nothing left to add");

    QVERIFY(pictura::vector_mask_delete(*view));
    QVERIFY(pictura::layer_mask_delete(*view));
    QCoreApplication::processEvents();
    panel_->refresh();
    QCOMPARE(panel_->maskButtonToolTipForTest(), QStringLiteral("Add layer mask"));
    QVERIFY(panel_->maskButtonEnabledForTest());
}

// lmc_mask_menu_rows
void LayersMaskCommandsTest::maskMenuRows()
{
    const QString path = fillLayer(16);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-all")));
    QCoreApplication::processEvents();
    panel_->refresh();

    const QStringList expected = {
        QStringLiteral("Disable Layer Mask"),
        QStringLiteral("Delete Layer Mask"),
        QStringLiteral("Apply Layer Mask"),
        QStringLiteral("Add Mask To Selection"),
        QStringLiteral("Subtract Mask From Selection"),
        QStringLiteral("Intersect Mask With Selection"),
        QStringLiteral("Refine Mask…"),
        QStringLiteral("Mask Options…")};
    QCOMPARE(panel_->maskMenuTextsForPathForTest(path), expected);
    for (const QString& text : expected) {
        QVERIFY2(panel_->maskMenuEnabledForPathForTest(path, text), qPrintable(text));
    }

    QVERIFY(pictura::layer_mask_set_enabled_path(*view, path, false));
    QCoreApplication::processEvents();
    panel_->refresh();
    QCOMPARE(panel_->maskMenuTextsForPathForTest(path).first(),
             QStringLiteral("Enable Layer Mask"));
}

// lmc_mask_menu_actions
void LayersMaskCommandsTest::maskMenuActions()
{
    const QString path = fillLayer(16);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("hide-all")));
    const quint32 hidden = view->sample_argb(8, 8);
    const int base = view->history_count();

    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("enableLayerMask"), path));
    QVERIFY(pictura::layer_mask_disabled_path(*view, path));
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("enableLayerMask"), path));
    QVERIFY(!pictura::layer_mask_disabled_path(*view, path));
    QCOMPARE(view->history_count(), base + 2);

    // Apply bakes the hide-all mask into the layer's alpha.
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("applyLayerMask"), path));
    QVERIFY(!pictura::layer_mask_present_path(*view, path));
    QCOMPARE(view->history_label(base + 2), QStringLiteral("Apply Layer Mask"));
    QCOMPARE(view->sample_argb(8, 8), hidden);

    QVERIFY(view->undo());
    QVERIFY(pictura::layer_mask_present_path(*view, path));
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("deleteLayerMask"), path));
    QVERIFY(!pictura::layer_mask_present_path(*view, path));
    QVERIFY2(view->sample_argb(8, 8) != hidden, "delete discards the mask: the layer shows");
}

// lmc_refine_mask
void LayersMaskCommandsTest::refineMask()
{
    const QString path = fillLayer(64);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view->select_rect(0, 0, 32, 64, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-selection")));
    view->deselect();
    const quint32 shown = view->sample_argb(8, 32);
    const quint32 hidden = view->sample_argb(56, 32);
    const int base = view->history_count();

    QString title;
    int output = -1;
    panel_->setDialogDriverForTest([&](QDialog& dialog) {
        title = dialog.windowTitle();
        auto* refine = qobject_cast<pictura::RefineEdgeDialog*>(&dialog);
        output = refine ? refine->outputTo() : -1;
        auto* feather = dialog.findChild<QSpinBox*>(QStringLiteral("refineFeather"));
        if (feather) {
            feather->setValue(4);
        }
        return feather != nullptr;
    });
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("refineMask"), path));
    QCOMPARE(title, QStringLiteral("Refine Mask"));
    QCOMPARE(output, 1);
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Refine Mask"));
    const quint32 edge = view->sample_argb(32, 32);
    QVERIFY2(edge != shown && edge != hidden, "the feathered edge is partial coverage");
    QCOMPARE(view->sample_argb(4, 32), shown);
    QVERIFY2(pictura::layer_mask_linked(*view), "the mask stays linked");
}

// lmc_refine_mask_any_layer: Refine Mask replaces the mask of a layer that
// does not render from its own pixels.
void LayersMaskCommandsTest::refineMaskOnAFillLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("FillMask"), 64, 64, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    const QString path = view->add_solid_fill(0xff2244aau);
    QVERIFY(!path.isEmpty());
    frame.selectLayerPath(path);
    QVERIFY(view->select_rect(0, 0, 32, 64, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-selection")));
    view->deselect();
    const int base = view->history_count();
    panel_->setDialogDriverForTest([](QDialog& dialog) {
        auto* feather = dialog.findChild<QSpinBox*>(QStringLiteral("refineFeather"));
        if (feather) {
            feather->setValue(4);
        }
        return feather != nullptr;
    });
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("refineMask"), path));
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Refine Mask"));
    QVERIFY(pictura::layer_mask_present_path(*view, path));
}

// lmc_apply_pixel_only: Apply would drop the mask of a layer that renders
// from its content or children, revealing what it hid.
void LayersMaskCommandsTest::applyNeedsAPixelLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("ApplyKinds"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    const QString fill = view->add_solid_fill(0xff2244aau);
    QVERIFY(!fill.isEmpty());
    frame.selectLayerPath(fill);
    QVERIFY(pictura::layer_mask_add_path(*view, fill, QStringLiteral("hide-all")));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY(!panel_->maskMenuEnabledForPathForTest(fill, QStringLiteral("Apply Layer Mask")));
    QVERIFY(panel_->maskMenuEnabledForPathForTest(fill, QStringLiteral("Delete Layer Mask")));
    const int history = view->history_count();
    QVERIFY2(!panel_->performMaskActionForTest(QStringLiteral("applyLayerMask"), fill),
             "the bridge refuses too");
    QVERIFY(pictura::layer_mask_present_path(*view, fill));
    QCOMPARE(view->history_count(), history);
}

// lmc_rubylith_toggle
void LayersMaskCommandsTest::rubylithOverlay()
{
    const QString path = fillLayer(32);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("hide-all")));
    QCoreApplication::processEvents();
    panel_->refresh();
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    const quint32 before = view->sample_argb(16, 16);
    const int history = view->history_count();

    QVERIFY(panel_->altShiftClickMaskThumbnailForTest(path));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_overlay_path(*view), path);
    QVERIFY2(!canvas->maskOverlayForTest().isNull(), "the canvas draws the overlay");
    QCOMPARE(view->sample_argb(16, 16), before);
    QCOMPARE(view->history_count(), history);

    QVERIFY(panel_->altShiftClickMaskThumbnailForTest(path));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_overlay_path(*view).isEmpty());
    QVERIFY(canvas->maskOverlayForTest().isNull());
    QCOMPARE(view->sample_argb(16, 16), before);
}

// lmc_rubylith_follows_the_layer: the overlay's path is positional, so it
// ends when its layer stops being the current one or the stack changes,
// instead of landing on another layer's mask.
void LayersMaskCommandsTest::rubylithEndsWithItsLayer()
{
    const QString path = fillLayer(32);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("hide-all")));
    QCoreApplication::processEvents();
    panel_->refresh();

    QVERIFY(panel_->altShiftClickMaskThumbnailForTest(path));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_overlay_path(*view), path);
    QVERIFY(panel_->performRowActionForTest(QStringLiteral("moveDown"), path));
    QCoreApplication::processEvents();
    QVERIFY2(pictura::mask_overlay_path(*view).isEmpty(), "a move ends the overlay");
    QVERIFY(window_->imageView()->maskOverlayForTest().isNull());
    panel_->refresh();
    QCoreApplication::processEvents();

    // The masked layer now sits at the other index; the old path names the
    // Background, whose rows have no mask to show.
    const QString moved = pictura::layer_mask_present_path(*view, QStringLiteral("0"))
                              ? QStringLiteral("0")
                              : QStringLiteral("1");
    QVERIFY(moved != path);
    QVERIFY(panel_->altShiftClickMaskThumbnailForTest(moved));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_overlay_path(*view), moved);
    window_->selectLayerPath(moved == QStringLiteral("0") ? QStringLiteral("1")
                                                          : QStringLiteral("0"));
    QCoreApplication::processEvents();
    QVERIFY2(pictura::mask_overlay_path(*view).isEmpty(), "another current layer ends it");
}

// lmc_rubylith_options
void LayersMaskCommandsTest::maskOptions()
{
    const QString path = fillLayer(32);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("hide-all")));
    QCOMPARE(pictura::mask_display_color(*view), 0xFFFF0000u);
    QCOMPARE(pictura::mask_display_opacity(*view), 50);

    QColor shownColor;
    int shownOpacity = -1;
    panel_->setDialogDriverForTest([&](QDialog& dialog) {
        auto* options = qobject_cast<pictura::MaskDisplayOptionsDialog*>(&dialog);
        if (!options) {
            return false;
        }
        shownColor = options->color();
        shownOpacity = options->opacity();
        options->setColor(QColor(0, 0, 255));
        options->setOpacity(80);
        return true;
    });
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("maskOptions"), path));
    QCOMPARE(shownColor, QColor(255, 0, 0));
    QCOMPARE(shownOpacity, 50);
    QCOMPARE(pictura::mask_display_color(*view), 0xFF0000FFu);
    QCOMPARE(pictura::mask_display_opacity(*view), 80);

    QVERIFY(pictura::mask_overlay_toggle(*view, path));
    window_->refresh();
    const QImage overlay = window_->imageView()->maskOverlayForTest();
    QVERIFY(!overlay.isNull());
    QCOMPARE(overlay.pixelColor(0, 0), QColor(0, 0, 255, 204));
}

// lmc_vector_menu
void LayersMaskCommandsTest::vectorMaskMenu()
{
    const QString path = fillLayer(16);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")));
    QVERIFY(pictura::vector_mask_add(*view, QStringLiteral("reveal-all")));
    QCoreApplication::processEvents();
    panel_->refresh();

    QCOMPARE(panel_->maskMenuTextsForPathForTest(path).first(),
             QStringLiteral("Disable Layer Mask"));
    // The vector thumbnail sits right of the layer-mask thumbnail.
    QCOMPARE(panel_->vectorMenuTextsForTest(path),
             (QStringList{QStringLiteral("Disable Vector Mask"),
                          QStringLiteral("Delete Vector Mask"),
                          QStringLiteral("Rasterize Vector Mask")}));

    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("vectorToggleEnabled"), path));
    QVERIFY(pictura::vector_mask_disabled(*view));
    panel_->refresh();
    QCOMPARE(panel_->vectorMenuTextsForTest(path).first(), QStringLiteral("Enable Vector Mask"));
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("vectorToggleEnabled"), path));
    QVERIFY(!pictura::vector_mask_disabled(*view));

    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("vectorDelete"), path));
    QVERIFY(!pictura::vector_mask_present(*view));
    QVERIFY(view->undo());
    QVERIFY(pictura::vector_mask_present(*view));
    QVERIFY(panel_->performMaskActionForTest(QStringLiteral("vectorRasterize"), path));
    QVERIFY(!pictura::vector_mask_present(*view));
    QCOMPARE(view->history_label(view->history_count() - 1),
             QStringLiteral("Rasterize Vector Mask"));
}

// lmc_trash_vector_prompt
void LayersMaskCommandsTest::trashDeletesTheVectorMask()
{
    const QString path = fillLayer(16);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")));
    QVERIFY(pictura::vector_mask_add(*view, QStringLiteral("reveal-all")));
    QCoreApplication::processEvents();
    panel_->refresh();
    const int layers = view->layer_count();

    QString prompt;
    bool accept = false;
    bool dontShow = false;
    int prompts = 0;
    panel_->setDialogDriverForTest([&](QDialog& dialog) {
        ++prompts;
        auto* box = qobject_cast<QMessageBox*>(&dialog);
        prompt = box ? box->text() : QString();
        if (box && box->checkBox()) {
            box->checkBox()->setChecked(dontShow);
        }
        return accept;
    });

    // Cancel keeps both masks and the layer.
    panel_->clickDeleteButtonForTest();
    QCOMPARE(prompt, QStringLiteral("Delete Vector Mask?"));
    QVERIFY(pictura::vector_mask_present(*view));
    QVERIFY(pictura::layer_mask_present(*view));

    // OK deletes only the vector mask.
    accept = true;
    dontShow = true;
    panel_->clickDeleteButtonForTest();
    QCoreApplication::processEvents();
    QVERIFY(!pictura::vector_mask_present(*view));
    QVERIFY(pictura::layer_mask_present(*view));
    QCOMPARE(view->layer_count(), layers);
    QVERIFY(!pictura::loadSession().confirmDeleteVectorMask);

    // "Don't show again": the next one goes without asking.
    QVERIFY(pictura::vector_mask_add(*view, QStringLiteral("reveal-all")));
    QCoreApplication::processEvents();
    panel_->refresh();
    panel_->clickDeleteButtonForTest();
    QCOMPARE(prompts, 2);
    QVERIFY(!pictura::vector_mask_present(*view));
    QCOMPARE(view->layer_count(), layers);

    pictura::SessionState session = pictura::loadSession();
    session.confirmDeleteVectorMask = true;
    pictura::saveSession(session);
}

// lmc_trash_selected_row: in the extended-selection tree the current row can
// sit outside the selection (Ctrl+arrow); the trash still acts on the
// selected layer, prompting for its vector mask rather than deleting it.
void LayersMaskCommandsTest::trashActsOnTheSelectedRow()
{
    const QString path = fillLayer(16);
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")));
    QVERIFY(pictura::vector_mask_add(*view, QStringLiteral("reveal-all")));
    QCoreApplication::processEvents();
    panel_->refresh();
    panel_->selectPaths({path}, path);
    const int layers = view->layer_count();

    auto* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    tree->setFocus();
    QTest::keyClick(tree, Qt::Key_Down, Qt::ControlModifier);
    QCoreApplication::processEvents();
    QCOMPARE(panel_->selectedPaths(), QStringList{path});
    QVERIFY2(panel_->currentPath() != path, "the current row left the selection");

    int prompts = 0;
    panel_->setDialogDriverForTest([&](QDialog&) {
        ++prompts;
        return true;
    });
    QVERIFY(panel_->deleteButtonEnabledForTest());
    panel_->clickDeleteButtonForTest();
    QCoreApplication::processEvents();
    QCOMPARE(prompts, 1);
    QCOMPARE(view->layer_count(), layers);
    QVERIFY2(!pictura::vector_mask_present_path(*view, path), "the selected layer's vector mask");
    QVERIFY(pictura::layer_mask_present_path(*view, path));
}

QTEST_MAIN(LayersMaskCommandsTest)
#include "tst_layers_mask_commands.moc"
