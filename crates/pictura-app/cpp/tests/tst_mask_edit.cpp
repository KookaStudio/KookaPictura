// Painting and filling the active layer mask (#108): with the mask edit target
// active, the Brush and Edit > Fill write the mask's coverage (black hides,
// white reveals) and record their normal history state, leaving the layer's
// pixels untouched. The canvas tools' own entry points (`begin_brush`,
// `begin_eraser`) take the same route (#314), and the image and the mask keep
// separate FG/BG pairs.

#include <QtTest/QtTest>
#include <QtWidgets/QToolButton>
#include "api/layers_api.h"
#include "api/selection_api.h"

#include "frame.h"
#include "panels/color_panel.h"
#include "panels/layers_panel.h"
#include "selftest_paint_fixture.h"
#include "toolbox.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

namespace {

QImage maskMoveSeed()
{
    QImage seed(32, 32, QImage::Format_RGB32);
    seed.fill(Qt::white);
    return seed;
}

// A blue pixel layer carrying a hide-all mask with a white patch at (8, 8), so
// the patch reveals the layer and the rest hides it. Leaves the mask target
// active. `layer` receives the masked layer's path.
// The fill's automatic default mask is a #895 behavior; these mask-editing
// tests start from an unmasked layer, so remove it right after creation.
QString addUnmaskedFill(pictura::PictureView& view)
{
    const QString path = view.add_solid_fill(0xff2244aau);
    if (!path.isEmpty()) {
        pictura::layer_mask_delete_path(view, path);
    }
    return path;
}

bool setUpMaskedLayer(paint_fixture::Fixture& f, QString& layer)
{
    layer = addUnmaskedFill(*f.view);
    if (layer.isEmpty() || !f.view->rasterize_fill_content(layer)) {
        return false;
    }
    f.frame.selectLayerPath(layer);
    if (!pictura::layer_mask_add(*f.view, QStringLiteral("hide-all"))) {
        return false;
    }
    if (!f.view->begin_paint(0xffffffffu, 0xff000000u, 12, 100, 100, 0, 100, 100, 25,
                             QStringLiteral("normal"), false, false)) {
        return false;
    }
    if (!f.view->paint_dab(8.0, 8.0, 1.0)) {
        return false;
    }
    return f.view->end_paint();
}

} // namespace

class MaskEditTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void maskEditTarget();
    void maskThumbnailOnNonActiveRowKeepsTarget();
    void ctrlClickMaskLoadsSelection();
    void invertOnMaskTarget();
    void paintAndFillWriteCoverage();
    void canvasToolsWriteCoverage();
    void healingRefusesMaskTarget();
    void colorsFollowTheEditTarget();
    void resetColorsFollowTheEditTarget();
    void moveSelectionOnMaskMovesCoverage();
    void altDragCopiesMaskCoverage();
    void ctrlDragOnMaskMovesCoverage();
    void maskSoloView();
    void maskSoloFollowsLayerChange();
    void addLayerMaskStripInverts();
    void maskContextMenu();
    void maskStrokePresentsWhileDragging();
    void largeMaskDabPreviews();
    void previewOnlyPixelsRestoreAtCommit();

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

void MaskEditTest::maskThumbnailOnNonActiveRowKeepsTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MaskNonActive"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const QString base = addUnmaskedFill(*view);
    QVERIFY(!base.isEmpty());
    QVERIFY(view->rasterize_fill_content(base));
    const QString top = view->add_layer_in(QString());
    QVERIFY(!top.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*view, base, QStringLiteral("reveal-all")));
    pictura::LayersPanel* panel =
        frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    frame.selectLayerPath(top);
    panel->refresh();
    QCoreApplication::processEvents();

    QVERIFY(panel->clickMaskThumbnailForTest(base));
    QCoreApplication::processEvents();
    QCOMPARE(view->active_layer_path(), base);
    QCOMPARE(pictura::mask_edit_target_get(*view), base);

    const pictura::PaintTip tip{20, 100, 100, 0, 25, 0, 1, 0, 0, 0};
    QVERIFY(pictura::begin_brush(*view, 0xff000000u, 0xffffffffu, tip, 100, 100,
                                 QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(8.0, 8.0, 1.0));
    QVERIFY(view->end_paint());

    // The black dab must have changed mask coverage, not the layer pixels.
    QVERIFY(view->mask_selection(base, QStringLiteral("new")));
    QCOMPARE(view->selection_coverage(8, 8), 0);
    QVERIFY(view->selection_coverage(1, 1) > 0);
    frame.closeDocument(doc, false);
}

void MaskEditTest::ctrlClickMaskLoadsSelection()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MaskCtrlClick"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const QString path = addUnmaskedFill(*view);
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-all")));
    pictura::LayersPanel* panel =
        frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    frame.selectLayerPath(path);
    panel->refresh();
    QCoreApplication::processEvents();

    QVERIFY(panel->ctrlClickMaskThumbnailForTest(path));
    QCoreApplication::processEvents();
    QVERIFY2(view->selection_coverage(8, 8) > 0, "the mask loaded as a selection");
    QCOMPARE(panel->selectedPaths().size(), 1);
    QCOMPARE(panel->selectedPaths().first(), path);
    frame.closeDocument(doc, false);
}

void MaskEditTest::invertOnMaskTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("MaskInvert"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const QString fill = view->add_solid_fill(0xff2244aau);
    QVERIFY(!fill.isEmpty());
    QVERIFY(pictura::mask_edit_target_set(*view, fill));
    QVERIFY(pictura::filter_target_ready(*view));
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    QVERIFY(view->mask_selection(fill, QStringLiteral("new")));
    QCOMPARE(view->selection_coverage(8, 8), 0);
    frame.closeDocument(doc, false);
}

void MaskEditTest::maskEditTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    const bool created = frame.newDocument(QStringLiteral("MaskTarget"), 16, 16,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    QVERIFY2(created && view, "mask target fixture");
    const int doc = frame.activeDocumentIndex();
    const QString path = addUnmaskedFill(*view);
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
    const QString path = addUnmaskedFill(*view);
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
    const QString path = addUnmaskedFill(*view);
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
    QVERIFY(pictura::begin_eraser(*view, 0xffffffffu, tip, 0, 100, 100, false, 1.0));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(view->history_label(base + 1), QStringLiteral("Eraser"));
    QCOMPARE(view->sample_argb(16, 16), fill);

    // Erase To History has no mask source and is refused.
    QVERIFY2(!pictura::begin_eraser(*view, 0xffffffffu, tip, 0, 100, 100, true, 1.0),
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

void MaskEditTest::healingRefusesMaskTarget()
{
    paint_fixture::Fixture f(*window_, maskMoveSeed(),
                             QStringLiteral("pictura_healing_mask_target"));
    QVERIFY(f.ok());
    QString layer;
    QVERIFY(setUpMaskedLayer(f, layer));
    QVERIFY2(pictura::mask_edit_target_set(*f.view, layer), "target the mask");

    f.tools->setActiveTool(pictura::ToolId::SpotHealingBrush);
    const int base = f.view->history_count();
    const quint32 revealed = f.view->sample_argb(8, 8);
    const quint32 hidden = f.view->sample_argb(16, 8);
    QSignalSpy refused(f.tools, &pictura::ToolController::pixelEditRefused);

    f.drag({QPointF(8, 8)});

    QCOMPARE(refused.count(), 1);
    QCOMPARE(f.view->history_count(), base);
    QCOMPARE(f.view->sample_argb(8, 8), revealed);
    QCOMPARE(f.view->sample_argb(16, 8), hidden);
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

    // The mask pair is global across documents; start from the CS6 defaults.
    auto* toolbox = frame.findChild<pictura::Toolbox*>();
    QVERIFY(toolbox);
    toolbox->resetForegroundBackground();

    // Adding a mask makes it the edit target, so the colour state is already the
    // mask pair (white on black).
    QCOMPARE(pictura::mask_edit_target_get(*view), layer);
    QCOMPARE(colors->foreground(), QColor(Qt::white));
    QCOMPARE(colors->background(), QColor(Qt::black));

    // Switch to the layer's pixels and set the image pair.
    QVERIFY(panel->clickLayerThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_edit_target_get(*view).isEmpty());
    const QColor imageFg(200, 40, 40);
    const QColor imageBg(20, 90, 200);
    colors->setForeground(imageFg);
    colors->setBackground(imageBg);

    // Back to the mask: it remembers its own pair (white/black).
    QVERIFY(panel->clickMaskThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_edit_target_get(*view), layer);
    QCOMPARE(colors->foreground(), QColor(Qt::white));
    QCOMPARE(colors->background(), QColor(Qt::black));

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
    QCOMPARE(colors->background(), QColor(Qt::black));

    // Closing the document leaves the mask, which restores the image pair.
    frame.closeDocument(doc, false);
    QCoreApplication::processEvents();
    QCOMPARE(colors->foreground(), imageFg);
}

void MaskEditTest::resetColorsFollowTheEditTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    paint_fixture::Fixture f(frame, maskMoveSeed(), QStringLiteral("pictura_mask_reset"));
    QVERIFY2(f.ok(), "mask reset fixture");
    QString layer;
    QVERIFY2(setUpMaskedLayer(f, layer), "masked layer");
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* colors = frame.findChild<pictura::ColorState*>();
    auto* toolbox = frame.findChild<pictura::Toolbox*>();
    QVERIFY(panel && colors && toolbox);
    frame.show();
    panel->refresh();
    QTest::qWait(50);

    // The mask pair is global across documents; start from the CS6 defaults.
    toolbox->resetForegroundBackground();
    QCOMPARE(colors->foreground(), QColor(Qt::white));
    QCOMPARE(colors->background(), QColor(Qt::black));

    colors->setForeground(QColor(128, 128, 128));
    toolbox->resetForegroundBackground();
    QCOMPARE(colors->foreground(), QColor(Qt::white));
    QCOMPARE(colors->background(), QColor(Qt::black));

    QVERIFY(panel->clickLayerThumbnailForTest(layer));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_edit_target_get(*f.view).isEmpty());
    colors->setForeground(QColor(200, 40, 40));
    colors->setBackground(QColor(20, 90, 200));
    toolbox->resetForegroundBackground();
    QCOMPARE(colors->foreground(), QColor(Qt::black));
    QCOMPARE(colors->background(), QColor(Qt::white));
}

void MaskEditTest::moveSelectionOnMaskMovesCoverage()
{
    pictura::PicturaMainWindow& frame = *window_;
    paint_fixture::Fixture f(frame, maskMoveSeed(), QStringLiteral("pictura_mask_move"));
    QVERIFY2(f.ok(), "mask move fixture");
    QString layer;
    QVERIFY2(setUpMaskedLayer(f, layer), "masked layer");
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* colors = frame.findChild<pictura::ColorState*>();
    auto* toolbox = frame.findChild<pictura::Toolbox*>();
    QVERIFY(panel && colors && toolbox);
    panel->refresh();
    QTest::qWait(50);
    toolbox->resetForegroundBackground();
    QCOMPARE(colors->foreground(), QColor(Qt::white));
    QCOMPARE(colors->background(), QColor(Qt::black));

    const quint32 blue = f.view->sample_argb(8, 8);
    QCOMPARE(blue, 0xff2244aau);
    frame.setActiveTool(pictura::ToolId::Move);
    f.view->select_all();
    QCOMPARE(pictura::selection_api::bounds(*f.view).toString(), QStringLiteral("0 0 32 32"));
    const int base = f.view->history_index();

    f.canvas->mousePressed(QPointF(8, 8), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(16, 8));
    f.canvas->mouseReleased(QPointF(16, 8));
    QCOMPARE(f.view->history_index(), base + 1);
    QCOMPARE(f.view->history_label(base + 1), QStringLiteral("Move Selection"));
    QCOMPARE(f.view->sample_argb(16, 8), blue);
    QCOMPARE(f.view->sample_argb(8, 8), 0xffffffffu);
    QCOMPARE(pictura::selection_api::bounds(*f.view).toString(), QStringLiteral("8 0 24 32"));
    QVERIFY2(panel->rowHasMaskForTest(layer), "the mask survives the move");

    // Undo restores the mask and the selection.
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::selection_api::bounds(*f.view).toString(), QStringLiteral("0 0 32 32"));
    QCOMPARE(f.view->sample_argb(8, 8), blue);
    QCOMPARE(f.view->sample_argb(16, 8), 0xffffffffu);

    // A zero-offset press/release records nothing.
    const int zeroBase = f.view->history_index();
    f.canvas->mousePressed(QPointF(8, 8), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseReleased(QPointF(8, 8));
    QCOMPARE(f.view->history_index(), zeroBase);

    // A release at a moved point without a final move event still commits the
    // content and leaves the selection at the offset.
    const int releaseBase = f.view->history_index();
    f.canvas->mousePressed(QPointF(8, 8), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseReleased(QPointF(16, 8));
    QCOMPARE(f.view->history_index(), releaseBase + 1);
    QCOMPARE(f.view->history_label(releaseBase + 1), QStringLiteral("Move Selection"));
    QCOMPARE(pictura::selection_api::bounds(*f.view).toString(), QStringLiteral("8 0 24 32"));

    // The layer pixels never moved: deleting the mask reveals the same blue.
    QVERIFY(pictura::layer_mask_delete_path(*f.view, layer));
    QCOMPARE(f.view->sample_argb(8, 8), blue);
    QCOMPARE(f.view->sample_argb(16, 8), blue);
}

void MaskEditTest::altDragCopiesMaskCoverage()
{
    pictura::PicturaMainWindow& frame = *window_;
    paint_fixture::Fixture f(frame, maskMoveSeed(), QStringLiteral("pictura_mask_alt"));
    QVERIFY2(f.ok(), "mask alt fixture");
    QString layer;
    QVERIFY2(setUpMaskedLayer(f, layer), "masked layer");
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* toolbox = frame.findChild<pictura::Toolbox*>();
    QVERIFY(panel && toolbox);
    panel->refresh();
    QTest::qWait(50);
    toolbox->resetForegroundBackground();

    const quint32 blue = f.view->sample_argb(8, 8);
    const int layers = f.view->layer_count();
    const QString active = f.view->active_layer_path();
    frame.setActiveTool(pictura::ToolId::Move);
    // A small selection keeps the copied coverage from landing on the source.
    QVERIFY(f.view->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    const int base = f.view->history_count();

    f.canvas->mousePressed(QPointF(8, 8), Qt::LeftButton, int(Qt::AltModifier));
    f.canvas->mouseMoved(QPointF(20, 8));
    QVERIFY2(!f.canvas->movePreviewActive(), "no pixel-clone preview on a mask");
    f.canvas->mouseReleased(QPointF(20, 8));

    QCOMPARE(f.view->history_count(), base + 1);
    QCOMPARE(f.view->history_label(base), QStringLiteral("Move Selection"));
    QCOMPARE(f.view->layer_count(), layers);
    QCOMPARE(f.view->active_layer_path(), active);
    QVERIFY2(panel->rowHasMaskForTest(layer), "the mask survives the copy");
    QCOMPARE(f.view->sample_argb(8, 8), blue);
    QCOMPARE(f.view->sample_argb(20, 8), blue);
}

void MaskEditTest::ctrlDragOnMaskMovesCoverage()
{
    pictura::PicturaMainWindow& frame = *window_;
    paint_fixture::Fixture f(frame, maskMoveSeed(), QStringLiteral("pictura_mask_ctrl"));
    QVERIFY2(f.ok(), "mask ctrl fixture");
    QString layer;
    QVERIFY2(setUpMaskedLayer(f, layer), "masked layer");
    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* toolbox = frame.findChild<pictura::Toolbox*>();
    QVERIFY(panel && toolbox);
    panel->refresh();
    QTest::qWait(50);
    toolbox->resetForegroundBackground();

    const quint32 blue = f.view->sample_argb(8, 8);
    frame.setActiveTool(pictura::ToolId::Marquee);
    f.view->select_all();
    const int base = f.view->history_count();

    f.canvas->mousePressed(QPointF(8, 8), Qt::LeftButton, int(Qt::ControlModifier));
    f.canvas->mouseMoved(QPointF(16, 8));
    f.canvas->mouseReleased(QPointF(16, 8));

    QCOMPARE(f.view->history_count(), base + 1);
    QCOMPARE(f.view->history_label(base), QStringLiteral("Move Selection"));
    QCOMPARE(f.view->sample_argb(16, 8), blue);
    QCOMPARE(f.view->sample_argb(8, 8), 0xffffffffu);
    QCOMPARE(pictura::selection_api::bounds(*f.view).toString(), QStringLiteral("8 0 24 32"));
    QVERIFY2(panel->rowHasMaskForTest(layer), "the mask survives the Ctrl move");

    // The layer pixels never moved: deleting the mask reveals the same blue.
    QVERIFY(pictura::layer_mask_delete_path(*f.view, layer));
    QCOMPARE(f.view->sample_argb(8, 8), blue);
    QCOMPARE(f.view->sample_argb(16, 8), blue);
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
    const QString layer = addUnmaskedFill(*view);
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
    const QString layer = addUnmaskedFill(*view);
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
                          QStringLiteral("Refine Mask…"), QStringLiteral("Mask Options…")}));

    // Refine Mask and Mask Options open their dialogs (tst_layers_mask_commands);
    // Apply and Delete are enabled.
    QVERIFY(panel->maskMenuEnabledForPathForTest(layer, QStringLiteral("Refine Mask…")));
    QVERIFY(panel->maskMenuEnabledForPathForTest(layer, QStringLiteral("Mask Options…")));
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
    // must be white for a Delete-fill to reveal it. The CS6 mask default is
    // black, so set the mask background explicitly.
    auto* colors = frame.findChild<pictura::ColorState*>();
    QVERIFY(colors);
    colors->setBackground(Qt::white);
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
    const QString layer = addUnmaskedFill(*view);
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

namespace {

// A white document with a rasterized fill layer whose reveal-all mask is the
// edit target; returns the fill layer's path, empty on failure.
QString maskedFillFixture(pictura::PicturaMainWindow& frame, const QString& name, int side)
{
    if (!frame.newDocument(name, side, side, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return {};
    }
    pictura::PictureView* view = frame.activeView();
    const QString path = view ? addUnmaskedFill(*view) : QString();
    if (path.isEmpty() || !view->rasterize_fill_content(path)) {
        return {};
    }
    frame.selectLayerPath(path);
    if (!pictura::layer_mask_add(*view, QStringLiteral("reveal-all"))
        || !pictura::mask_edit_target_set(*view, path)) {
        return {};
    }
    return path;
}

// The stored view-pyramid pixel of `level` over document point `(x, y)`.
QRgb levelPixel(const pictura::PictureView& view, int level, int x, int y)
{
    const QImage pixel = view.display_image(level, x >> level, y >> level, 1, 1);
    return pixel.isNull() ? 0u : pixel.pixel(0, 0);
}

bool beginLargeMaskStroke(pictura::PictureView& view)
{
    return view.begin_paint(0xff000000u, 0xffffffffu, 600, 100, 100, 0, 100, 100, 25,
                            QStringLiteral("normal"), false, false);
}

}

// Without the GPU stroke a large mask brush previews at a pyramid level and
// replays exactly at the commit; a cancel puts everything back.
void MaskEditTest::largeMaskDabPreviews()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY2(!maskedFillFixture(frame, QStringLiteral("MaskPreview"), 2048).isEmpty(),
             "fixture");
    pictura::PictureView* view = frame.activeView();
    const int doc = frame.activeDocumentIndex();
    view->set_gpu_compute(false);
    QCoreApplication::processEvents();
    const quint32 fill = view->sample_argb(1024, 1024);
    const int base = view->history_count();

    // 600 px on 2048² is over the 262 144 px budget, and level 3 exists
    // (2048 → 1024 → 512 → 256).
    QVERIFY(beginLargeMaskStroke(*view));
    QCOMPARE(view->preview_present_level(), 3);
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    QVERIFY(view->paint_dab(1100.0, 1024.0, 1.0));
    view->flush_present();
    QCOMPARE(view->history_count(), base);
    QCOMPARE(view->sample_argb(1024, 1024), fill);

    // The replay hides the layer under the brush and leaves the rest alone.
    QVERIFY(view->end_paint());
    QCOMPARE(view->preview_present_level(), 0);
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->history_label(base), QStringLiteral("Brush"));
    QCOMPARE(view->sample_argb(1024, 1024), 0xffffffffu);
    QCOMPARE(view->sample_argb(1024 - 280, 1024), 0xffffffffu);
    QCOMPARE(view->sample_argb(16, 16), fill);
    QVERIFY(view->undo());
    QCOMPARE(view->sample_argb(1024, 1024), fill);

    QVERIFY(beginLargeMaskStroke(*view));
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    view->cancel_paint();
    QVERIFY(!view->is_painting());
    QCOMPARE(view->history_count(), base + 1);
    QCOMPARE(view->sample_argb(1024, 1024), fill);

    frame.closeDocument(doc, false);
}

// The preview can paint level pixels the exact stroke never touches; the commit
// must put them back to the stroke-start level instead of leaving the preview
// on the canvas over a mask that never changed there. (The other such case, a
// dab crossing a mask smaller than its layer, needs a mask no app command
// makes; `a_dab_past_the_mask_edge_previews_there_and_restores` covers it.)
void MaskEditTest::previewOnlyPixelsRestoreAtCommit()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY2(!maskedFillFixture(frame, QStringLiteral("MaskSpacing"), 2048).isEmpty(),
             "fixture");
    pictura::PictureView* view = frame.activeView();
    const int doc = frame.activeDocumentIndex();
    view->set_gpu_compute(false);
    QCoreApplication::processEvents();

    // A sample the exact spacing skips: at 25% of 600 px the exact stroke dabs
    // every 150 px, so it never dabs the second sample, 76 px on, while the
    // preview does. (1376, 1024) is inside only that second preview dab.
    QVERIFY(beginLargeMaskStroke(*view));
    const int level = view->preview_present_level();
    QVERIFY(level > 0);
    const QRgb before = levelPixel(*view, level, 1376, 1024);
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    QVERIFY(view->paint_dab(1100.0, 1024.0, 1.0));
    view->flush_present();
    QVERIFY2(levelPixel(*view, level, 1376, 1024) != before, "the preview painted there");
    QVERIFY(view->end_paint());
    QCOMPARE(levelPixel(*view, level, 1376, 1024), before);
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

    const QString masked = addUnmaskedFill(*view);
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
    const QString layer = addUnmaskedFill(*view);
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