// Painting and filling the active layer mask (#108): with the mask edit target
// active, the Brush and Edit > Fill write the mask's coverage (black hides,
// white reveals) and record their normal history state, leaving the layer's
// pixels untouched. The canvas tools' own entry points (`begin_brush`,
// `begin_eraser`) take the same route (#314), and the image and the mask keep
// separate FG/BG pairs.

#include <QtTest/QtTest>
#include <QtWidgets/QToolButton>
#include "api/layers_api.h"

#include "frame.h"
#include "panels/color_panel.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

class MaskEditTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void maskEditTarget();
    void paintAndFillWriteCoverage();
    void canvasToolsWriteCoverage();
    void colorsFollowTheEditTarget();
    void maskSoloView();
    void maskSoloFollowsLayerChange();
    void addLayerMaskStripInverts();
    void maskContextMenu();
    void maskStrokePresentsWhileDragging();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void MaskEditTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void MaskEditTest::maskEditTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskTarget"), 16, 16,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    QVERIFY2(created && view, "mask target fixture");
    const int doc = frame.activeDocumentIndex();
    const QString path = view->add_solid_fill(0xff2244aau);
    QVERIFY(!path.isEmpty());
    QVERIFY(view->rasterize_fill_content(path));
    frame.selectLayerPath(path);

    // Without a mask the target refuses; with one it round-trips and clears.
    QVERIFY2(!pictura::mask_edit_target_set(*view, path), "no mask yet");
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "add mask");
    QVERIFY2(pictura::mask_edit_target_set(*view, path), "target the mask");
    QCOMPARE(pictura::mask_edit_target_get(*view), path);
    QVERIFY2(pictura::mask_edit_target_set(*view, QString()), "clear the target");
    QVERIFY(pictura::mask_edit_target_get(*view).isEmpty());
    QVERIFY2(!pictura::mask_edit_target_set(*view, QStringLiteral("9")), "unknown path");

    // Deleting the mask empties a live target.
    QVERIFY(pictura::mask_edit_target_set(*view, path));
    QVERIFY(pictura::layer_mask_delete(*view));
    QVERIFY(pictura::mask_edit_target_get(*view).isEmpty());
    frame.closeDocument(doc, false);
}

void MaskEditTest::paintAndFillWriteCoverage()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskPaint"), 32, 32,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    QVERIFY2(created && view, "mask paint fixture");
    const int doc = frame.activeDocumentIndex();
    const QString path = view->add_solid_fill(0xff2244aau);
    QVERIFY(!path.isEmpty());
    QVERIFY(view->rasterize_fill_content(path));
    frame.selectLayerPath(path);
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "add mask");
    QVERIFY2(pictura::mask_edit_target_set(*view, path), "target the mask");

    const quint32 fill = view->sample_argb(16, 16);
    const int base = view->history_count();

    // Black on the mask hides the layer; the white background shows through.
    QVERIFY(view->begin_paint(0xff000000u, 0xffffffffu, 20, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Brush"));
    QVERIFY2(view->sample_argb(16, 16) != fill, "the mask hid the layer");

    // Undo restores the coverage bit-for-bit; the layer shows again.
    QVERIFY(view->undo());
    QCOMPARE(view->sample_argb(16, 16), fill);

    // White on the mask reveals a layer hidden by a black mask.
    QVERIFY(pictura::layer_mask_delete(*view));
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("hide-all")), "hide all");
    QVERIFY(pictura::mask_edit_target_set(*view, path));
    QVERIFY(view->begin_paint(0xffffffffu, 0xff000000u, 20, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->sample_argb(16, 16), fill);

    // Cancelling a mask stroke puts the coverage back.
    QVERIFY(view->begin_paint(0xff000000u, 0xffffffffu, 20, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    view->cancel_paint();
    QCOMPARE(view->sample_argb(16, 16), fill);

    // Edit > Fill on the mask writes coverage through the selection.
    QVERIFY(pictura::layer_mask_delete(*view));
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "reveal all");
    QVERIFY(pictura::mask_edit_target_set(*view, path));
    view->select_rect(0, 0, 16, 32, QStringLiteral("new"), 0.0);
    const int fillBase = view->history_count();
    QVERIFY(pictura::edit_fill(*view, 0xff000000u, -1, QStringLiteral("normal"), 100, false));
    QCOMPARE(view->history_count(), fillBase + 1);
    QCOMPARE(view->history_label(fillBase), QStringLiteral("Fill"));
    QVERIFY2(view->sample_argb(8, 16) != fill, "the filled mask hid the layer");
    QCOMPARE(view->sample_argb(24, 16), fill);

    frame.closeDocument(doc, false);
}

void MaskEditTest::canvasToolsWriteCoverage()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskTools"), 32, 32,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    QVERIFY2(created && view, "mask tools fixture");
    const int doc = frame.activeDocumentIndex();
    const QString path = view->add_solid_fill(0xff2244aau);
    QVERIFY(!path.isEmpty());
    QVERIFY(view->rasterize_fill_content(path));
    frame.selectLayerPath(path);
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "add mask");
    QVERIFY2(pictura::mask_edit_target_set(*view, path), "target the mask");

    const quint32 fill = view->sample_argb(16, 16);
    const pictura::PaintTip tip{20, 100, 100, 0, 25, 0, 1, 0, 0, 0};

    // The Brush tool paints black coverage: the layer hides and its own
    // pixels are left as they were.
    const int base = view->history_count();
    QVERIFY(pictura::begin_brush(*view, 0xff000000u, 0xffffffffu, tip, 100, 100,
                                 QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->history_label(base), QStringLiteral("Brush"));
    QVERIFY2(view->sample_argb(16, 16) != fill, "the brush hid the layer through its mask");
    QVERIFY(pictura::mask_edit_target_set(*view, QString()));
    QVERIFY(pictura::layer_mask_delete(*view));
    QCOMPARE(view->sample_argb(16, 16), fill);
    QVERIFY(view->undo());
    QVERIFY(pictura::mask_edit_target_set(*view, path));

    // The Eraser paints the background colour: white reveals again.
    QVERIFY(pictura::begin_eraser(*view, 0xffffffffu, tip, 0, 100, 100, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->history_label(base + 1), QStringLiteral("Eraser"));
    QCOMPARE(view->sample_argb(16, 16), fill);

    // Erase To History has no mask source and is refused.
    QVERIFY2(!pictura::begin_eraser(*view, 0xffffffffu, tip, 0, 100, 100, true),
             "erase to history refuses a mask target");
    QCOMPARE(view->history_count(), base + 2);

    // Tools that cannot paint a mask refuse instead of editing the pixels.
    QVERIFY2(!pictura::begin_pattern_stamp(*view, tip, 100, 100, QStringLiteral("normal"), 0, 0,
                                           0),
             "pattern stamp refuses a mask target");
    QVERIFY2(!pictura::bucket_fill_at(*view, 16, 16, 0xff000000u, -1, QStringLiteral("normal"),
                                      100, 32, false, true, false),
             "paint bucket refuses a mask target");
    QCOMPARE(view->history_count(), base + 2);

    frame.closeDocument(doc, false);
}

void MaskEditTest::colorsFollowTheEditTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskColors"), 16, 16,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* colors = frame.findChild<pictura::ColorState*>();
    QVERIFY2(created && view && panel && colors, "mask colours fixture");
    const int doc = frame.activeDocumentIndex();
    const QString layer = pictura::layers_api::rowAt(*view, 0).path;
    frame.selectLayerPath(layer);
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "add mask");
    frame.show();
    panel->refresh();
    QTest::qWait(50);

    // Adding a mask makes it the edit target, so the colour state is already the
    // mask pair (black on white).
    QCOMPARE(pictura::mask_edit_target_get(*view), layer);
    QCOMPARE(colors->foreground(), QColor(Qt::black));
    QCOMPARE(colors->background(), QColor(Qt::white));

    // Switch to the layer's pixels and set the image pair.
    QVERIFY(panel->clickLayerThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_edit_target_get(*view).isEmpty());
    const QColor imageFg(200, 40, 40);
    const QColor imageBg(20, 90, 200);
    colors->setForeground(imageFg);
    colors->setBackground(imageBg);

    // Back to the mask: it remembers its own pair (black/white).
    QVERIFY(panel->clickMaskThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_edit_target_get(*view), layer);
    QCOMPARE(colors->foreground(), QColor(Qt::black));
    QCOMPARE(colors->background(), QColor(Qt::white));

    // The mask remembers its own pair; the image gets its pair back.
    const QColor maskFg(128, 128, 128);
    colors->setForeground(maskFg);
    QVERIFY(panel->clickLayerThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_edit_target_get(*view).isEmpty());
    QCOMPARE(colors->foreground(), imageFg);
    QCOMPARE(colors->background(), imageBg);

    QVERIFY(panel->clickMaskThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QCOMPARE(colors->foreground(), maskFg);
    QCOMPARE(colors->background(), QColor(Qt::white));

    // Closing the document leaves the mask, which restores the image pair.
    frame.closeDocument(doc, false);
    QCoreApplication::processEvents();
    QCOMPARE(colors->foreground(), imageFg);
}

// Alt-clicking a mask thumbnail shows the mask alone (view-only, no history);
// a second Alt-click, another thumbnail click, or a shared reset restores the
// composite, and painting the mask while soloed keeps the mode.
void MaskEditTest::maskSoloView()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskSolo"), 32, 32,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view && panel, "mask solo fixture");
    const int doc = frame.activeDocumentIndex();
    const QString layer = view->add_solid_fill(0xff2244aau);
    QVERIFY(!layer.isEmpty());
    QVERIFY(view->rasterize_fill_content(layer));
    frame.selectLayerPath(layer);
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "add mask");

    // The bridge is view-only: set/clear round-trips with no history change.
    const int base = view->history_count();
    const int baseIndex = view->history_index();
    QVERIFY(pictura::mask_solo_set(*view, layer));
    QCOMPARE(pictura::mask_solo_get(*view), layer);
    QVERIFY(pictura::mask_solo_set(*view, QString()));
    QVERIFY(pictura::mask_solo_get(*view).isEmpty());
    QCOMPARE(view->history_count(), base);
    QCOMPARE(view->history_index(), baseIndex);
    QVERIFY2(!pictura::mask_solo_set(*view, QStringLiteral("99")), "no mask at path");

    // The panel gesture: Alt-click solos and keeps the mask the edit target.
    panel->setView(view);
    panel->selectPaths({layer}, layer);
    panel->refresh();
    frame.show();
    QTest::qWait(50);
    QVERIFY2(panel->altClickMaskThumbnailForTest(layer), "mask thumbnail exists");
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_solo_get(*view), layer);
    QVERIFY2(panel->rowMaskTargetForTest(layer), "the mask stays the edit target");
    QCOMPARE(view->history_count(), base);

    // A second Alt-click restores; Alt then a plain click restores too.
    QVERIFY(panel->altClickMaskThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_solo_get(*view).isEmpty());
    QVERIFY(panel->altClickMaskThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_solo_get(*view), layer);
    QVERIFY(panel->clickMaskThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_solo_get(*view).isEmpty());

    // Alt+Shift is reserved for rubylith: neither solos nor disables.
    QVERIFY(panel->altShiftClickMaskThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_solo_get(*view).isEmpty());
    QVERIFY2(!panel->rowMaskDisabledForTest(layer), "Alt+Shift did not disable the mask");

    // Painting the soloed mask keeps the mode and records one edit.
    QVERIFY(pictura::mask_solo_set(*view, layer));
    QVERIFY(pictura::mask_edit_target_set(*view, layer));
    const int paintBase = view->history_count();
    QVERIFY(view->begin_paint(0xff000000u, 0xffffffffu, 20, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->history_count(), paintBase + 1);
    QCOMPARE(pictura::mask_solo_get(*view), layer);

    // Deleting the mask ends solo.
    QVERIFY(pictura::layer_mask_delete(*view));
    QVERIFY(pictura::mask_solo_get(*view).isEmpty());

    frame.closeDocument(doc, false);
}

// The mask thumbnail's right-click menu (#436): the CS6 item set and order, the
// enable/disable label, Apply/Delete, the selection combine commands, and the
// Delete key removing the mask.
void MaskEditTest::maskContextMenu()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskMenu"), 32, 32,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view && panel, "mask menu fixture");
    const int doc = frame.activeDocumentIndex();
    const QString layer = view->add_solid_fill(0xff2244aau);
    QVERIFY(!layer.isEmpty());
    QVERIFY(view->rasterize_fill_content(layer));
    frame.selectLayerPath(layer);
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "add mask");
    panel->setView(view);
    panel->selectPaths({layer}, layer);
    panel->refresh();
    frame.show();
    QTest::qWait(50);

    // The menu lists the CS6 items and order.
    QCOMPARE(panel->maskMenuTextsForPathForTest(layer),
             QStringList({QStringLiteral("Disable Layer Mask"),
                          QStringLiteral("Delete Layer Mask"), QStringLiteral("Apply Layer Mask"),
                          QStringLiteral("Add Mask To Selection"),
                          QStringLiteral("Subtract Mask From Selection"),
                          QStringLiteral("Intersect Mask With Selection"),
                          QStringLiteral("Select and Mask…"), QStringLiteral("Mask Options…")}));

    // Select and Mask is wired (it opens Refine Edge); Mask Options stays a
    // disabled placeholder. Apply and Delete are enabled.
    QVERIFY(panel->maskMenuEnabledForPathForTest(layer, QStringLiteral("Select and Mask…")));
    QVERIFY(!panel->maskMenuEnabledForPathForTest(layer, QStringLiteral("Mask Options…")));
    QVERIFY(panel->maskMenuEnabledForPathForTest(layer, QStringLiteral("Apply Layer Mask")));
    QVERIFY(panel->maskMenuEnabledForPathForTest(layer, QStringLiteral("Delete Layer Mask")));

    // The enable/disable label follows the state.
    QVERIFY(pictura::layer_mask_set_enabled_path(*view, layer, false));
    panel->refresh();
    const QStringList off = panel->maskMenuTextsForPathForTest(layer);
    QVERIFY(off.contains(QStringLiteral("Enable Layer Mask")));
    QVERIFY(!off.contains(QStringLiteral("Disable Layer Mask")));
    QVERIFY(pictura::layer_mask_set_enabled_path(*view, layer, true));
    panel->refresh();

    // Apply folds the coverage into the layer and clears the mask, one state.
    const int base = view->history_count();
    QVERIFY(panel->performMaskActionForTest(QStringLiteral("applyLayerMask"), layer));
    panel->refresh();
    QVERIFY2(!panel->rowHasMaskForTest(layer), "apply removed the mask");
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Apply Layer Mask"));

    // Apply is disabled on a smart object (a mask can be added, not applied).
    view->add_layer(-1);
    panel->refresh();
    const QString smart = pictura::layers_api::rowAt(*view, 0).path;
    QVERIFY(view->convert_to_smart_object(smart));
    QVERIFY(pictura::layer_mask_add_path(*view, smart, QStringLiteral("reveal-all")));
    panel->refresh();
    QVERIFY(panel->rowSmartObjectForTest(smart));
    QVERIFY2(!panel->maskMenuEnabledForPathForTest(smart, QStringLiteral("Apply Layer Mask")),
             "apply disabled for a smart object");

    // The selection commands load the mask coverage (white = selected).
    QVERIFY(view->select_rect(0, 0, 16, 32, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-selection")));
    view->deselect();
    QVERIFY(panel->performMaskActionForTest(QStringLiteral("maskAdd"), layer));
    QVERIFY2(view->selection_coverage(4, 16) > 0, "the mask's left half is selected");
    QCOMPARE(view->selection_coverage(28, 16), 0);

    // With a pixel selection, Delete fills that selection with the background
    // colour (the mask survives); with no selection it removes the mask.
    frame.selectLayerPath(layer);
    QVERIFY(pictura::mask_edit_target_set(*view, layer));
    panel->refresh();
    QVERIFY(panel->maskEditing());
    panel->setFocus();
    QCoreApplication::processEvents();
    // The mask's right half is black (hidden) and the edit target's background
    // is white, so a Delete-fill reveals it. Select that half.
    QVERIFY(view->select_rect(16, 0, 16, 32, QStringLiteral("new"), 0.0));
    panel->refresh();
    QVERIFY2(view->selection_coverage(24, 16) > 0, "a selection is active");
    const quint32 revealed = view->sample_argb(8, 16);
    QVERIFY2(view->sample_argb(24, 16) != revealed, "the mask hides the right half");
    const int fillBase = view->history_count();
    QTest::keyClick(panel, Qt::Key_Delete);
    panel->refresh();
    QVERIFY2(panel->rowHasMaskForTest(layer), "Delete filled the selection, mask survives");
    QCOMPARE(view->history_label(fillBase), QStringLiteral("Fill"));
    QCOMPARE(view->history_count(), fillBase + 1);
    QCOMPARE(view->sample_argb(24, 16), revealed);

    // No selection: Delete removes the mask.
    view->deselect();
    panel->refresh();
    const int delBase = view->history_count();
    QTest::keyClick(panel, Qt::Key_Delete);
    panel->refresh();
    QVERIFY2(!panel->rowHasMaskForTest(layer), "Delete removed the mask");
    QCOMPARE(view->history_count(), delBase + 1);

    frame.closeDocument(doc, false);
}

// A live mask stroke must count as painting, so the shell's `regionBlitted`
// slot schedules `flush_present` and the drag renders frame by frame instead of
// only on release. Guards `PictureView::is_painting` including `mask_stroke`.
void MaskEditTest::maskStrokePresentsWhileDragging()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskDrag"), 64, 64,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    QVERIFY2(created && view, "mask drag fixture");
    const int doc = frame.activeDocumentIndex();
    const QString layer = view->add_solid_fill(0xff2244aau);
    QVERIFY(!layer.isEmpty());
    QVERIFY(view->rasterize_fill_content(layer));
    frame.selectLayerPath(layer);
    QVERIFY2(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")), "add mask");
    QVERIFY(pictura::mask_edit_target_set(*view, layer));

    QSignalSpy blit(view, &pictura::PictureView::regionBlitted);
    QVERIFY2(blit.isValid(), "regionBlitted spy");
    const int history = view->history_count();
    // A small brush stays on the exact CPU mask path.
    QVERIFY2(view->begin_paint(0xff000000u, 0xffffffffu, 24, 100, 100, 0, 100, 100, 25,
                               QStringLiteral("normal"), false, false),
             "begin a mask stroke");
    QVERIFY2(view->is_painting(), "a mask stroke is a paint stroke");

    // The frame-opening dab presents itself.
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    const int afterOpen = blit.count();
    QVERIFY2(afterOpen >= 1, "the first mask dab presents");

    // A later dab accumulates; the shell's deferred flush presents the union.
    QVERIFY(view->paint_dab(44.0, 16.0, 1.0));
    QVERIFY(view->flush_present());
    QVERIFY2(blit.count() > afterOpen, "the flush presents the accumulated drag");

    QVERIFY(view->end_paint());
    QVERIFY2(!view->is_painting(), "the mask stroke ended");
    QCOMPARE(view->history_count(), history + 1);
    frame.closeDocument(doc, false);
}

// Mask solo is tied to the current layer: selecting a different layer ends it,
// while a Shift-click that only toggles the mask's enabled bit keeps it.
void MaskEditTest::maskSoloFollowsLayerChange()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskSoloFollow"), 32, 32,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view && panel, "mask solo follow fixture");
    const int doc = frame.activeDocumentIndex();

    const QString masked = view->add_solid_fill(0xff2244aau);
    QVERIFY(!masked.isEmpty());
    QVERIFY(view->rasterize_fill_content(masked));
    QVERIFY2(pictura::layer_mask_add_path(*view, masked, QStringLiteral("reveal-all")), "mask");

    // A different layer: the document's own Background (it carries no mask).
    QString other;
    for (int i = 0; i < 4 && other.isEmpty(); ++i) {
        const auto row = pictura::layers_api::rowAt(*view, i);
        if (row.path.isEmpty()) {
            break;
        }
        if (row.path != masked && !row.hasMask) {
            other = row.path;
        }
    }
    QVERIFY2(!other.isEmpty(), "a second layer exists");

    panel->setView(view);
    frame.selectLayerPath(masked);
    panel->selectPaths({masked}, masked);
    panel->refresh();
    frame.show();
    QTest::qWait(50);
    QVERIFY(pictura::mask_solo_set(*view, masked));

    // Selecting another layer ends the solo.
    panel->selectPaths({other}, other);
    QCoreApplication::processEvents();
    QVERIFY2(pictura::mask_solo_get(*view).isEmpty(), "another layer ends the solo");

    // A Shift-click that toggles the enabled bit keeps the solo.
    frame.selectLayerPath(masked);
    panel->selectPaths({masked}, masked);
    panel->refresh();
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_solo_set(*view, masked));
    QVERIFY(panel->shiftClickMaskThumbnailForTest(masked));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_solo_get(*view), masked);
    QVERIFY2(panel->rowMaskDisabledForTest(masked), "Shift-click disabled the mask");

    frame.closeDocument(doc, false);
}

// The Layers action-strip Add Layer Mask button: an Alt-click adds a Hide All
// (black) mask, inverting rather than revealing.
void MaskEditTest::addLayerMaskStripInverts()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskStripInvert"), 32, 32,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view && panel, "mask strip invert fixture");
    const int doc = frame.activeDocumentIndex();
    const QString layer = view->add_solid_fill(0xff2244aau);
    QVERIFY(!layer.isEmpty());
    QVERIFY(view->rasterize_fill_content(layer));
    frame.selectLayerPath(layer);
    panel->setView(view);
    panel->selectPaths({layer}, layer);
    panel->refresh();
    frame.show();
    QTest::qWait(50);

    auto* button = frame.findChild<QToolButton*>(QStringLiteral("layersStripMask"));
    QVERIFY(button != nullptr);
    const quint32 fill = view->sample_argb(16, 16);
    QTest::mouseClick(button, Qt::LeftButton, Qt::AltModifier);
    QCoreApplication::processEvents();
    QVERIFY2(panel->rowHasMaskForTest(layer), "Alt-click added a mask");
    QVERIFY2(view->sample_argb(16, 16) != fill, "Alt-click added a Hide All (black) mask");

    frame.closeDocument(doc, false);
}

QTEST_MAIN(MaskEditTest)
#include "tst_mask_edit.moc"