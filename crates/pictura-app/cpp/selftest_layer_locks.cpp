#include "selftest_layer_locks.h"
#include "selftest_report.h"

#include "api/transform_api.h"
#include "frame.h"
#include "image_view.h"
#include "panels/layers_panel.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/Qt>
#include <QtGui/QColor>
#include <QtGui/QImage>

int pictura::runLayerLocksChecks(pictura::PicturaMainWindow& frame)
{
    // llk_move_refusal (313): a Move-tool drag over a position-locked layer is
    // refused at the shared translate entry, so the rect, the composite, and
    // the history are all unchanged.
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockMove"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        pictura::ImageView* canvas = frame.imageView();
        auto* tools = frame.findChild<pictura::ToolController*>();
        if (!created || !view || !canvas || !tools) {
            return pictura::selfTest().fail(313, "move refusal fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString locked = view->add_layer_in(QString());
        view->set_active_layer(locked);
        view->set_layer_name_path(locked, QStringLiteral("LockedMove"));
        view->set_layers_lock(QStringList{locked}, QStringLiteral("position"), true);
        const QString rectBefore = transform_api::layerRect(*view, locked).toString();
        const unsigned int pixelBefore = view->composite_argb(2, 2);
        const int historyBefore = view->history_count();
        frame.setActiveTool(pictura::ToolId::Move);
        canvas->mousePressed(QPointF(2, 2), Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(QPointF(7, 7));
        canvas->mouseReleased(QPointF(7, 7));
        const bool rectSame = transform_api::layerRect(*view, locked).toString() == rectBefore;
        const bool compositeSame = view->composite_argb(2, 2) == pixelBefore;
        const bool noHistory = view->history_count() == historyBefore;
        ST_BEGIN("llk_move_refusal");
        ST_PASS("llk_move_refusal rect=%d composite=%d history=%d", rectSame ? 1 : 0,
                compositeSame ? 1 : 0, view->history_count() - historyBefore);
        if (!rectSame || !compositeSame || !noHistory) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(313, "move-tool lock refusal");
        }
        frame.closeDocument(doc, false);
    }

    // llk_filter_refusal (314): a filter on a pixel-locked layer returns false
    // and leaves the composite and history untouched.
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockFilter"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(314, "filter refusal fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString locked = view->add_layer_in(QString());
        view->set_active_layer(locked);
        view->set_layer_name_path(locked, QStringLiteral("LockedFilter"));
        view->set_layers_lock(QStringList{locked}, QStringLiteral("pixels"), true);
        const unsigned int pixelBefore = view->composite_argb(4, 4);
        const int historyBefore = view->history_count();
        const bool filtered = view->apply_filter(QStringLiteral("add-noise"));
        const bool compositeSame = view->composite_argb(4, 4) == pixelBefore;
        const bool noHistory = view->history_count() == historyBefore;
        ST_BEGIN("llk_filter_refusal");
        ST_PASS("llk_filter_refusal applied=%d composite=%d history=%d", filtered ? 1 : 0,
                compositeSame ? 1 : 0, view->history_count() - historyBefore);
        if (filtered || !compositeSame || !noHistory) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(314, "filter lock refusal");
        }
        frame.closeDocument(doc, false);
    }

    // llk_locked_cursor (315): a paint tool over a pixel-locked layer shows the
    // forbidden cursor; unlocking restores the normal tool cursor.
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockCursor"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* tools = frame.findChild<pictura::ToolController*>();
        if (!created || !view || !tools) {
            return pictura::selfTest().fail(315, "locked cursor fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString locked = view->add_layer_in(QString());
        view->set_active_layer(locked);
        view->set_layer_name_path(locked, QStringLiteral("LockedCursor"));
        view->set_layers_lock(QStringList{locked}, QStringLiteral("pixels"), true);
        frame.setActiveTool(pictura::ToolId::Brush);
        tools->refreshCursor();
        const bool forbidden =
            frame.imageView()->cursor().shape() == Qt::ForbiddenCursor;
        view->set_layers_lock(QStringList{locked}, QStringLiteral("pixels"), false);
        tools->refreshCursor();
        const bool restored =
            frame.imageView()->cursor().shape() != Qt::ForbiddenCursor;
        ST_BEGIN("llk_locked_cursor");
        ST_PASS("llk_locked_cursor locked=%d unlocked=%d", forbidden ? 1 : 0,
                restored ? 1 : 0);
        if (!forbidden || !restored) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(315, "locked pixel cursor");
        }
        frame.closeDocument(doc, false);
    }

    // llk_paint_refusal (316): a brush stroke on a pixel-locked layer begins no
    // stroke, writes no pixels, adds no history, and does not dirty the
    // document.
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockPaint"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(316, "paint refusal fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString locked = view->add_layer_in(QString());
        view->set_active_layer(locked);
        view->set_layer_name_path(locked, QStringLiteral("LockedPaint"));
        view->set_layers_lock(QStringList{locked}, QStringLiteral("pixels"), true);
        const unsigned int pixelBefore = view->composite_argb(4, 4);
        const int historyBefore = view->history_count();
        const bool begun = view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100,
                                             25, QStringLiteral("normal"), false, false);
        const bool noStroke = !view->is_painting();
        const bool compositeSame = view->composite_argb(4, 4) == pixelBefore;
        const bool noHistory = view->history_count() == historyBefore;
        ST_BEGIN("llk_paint_refusal");
        ST_PASS("llk_paint_refusal begun=%d stroke=%d composite=%d history=%d", begun ? 1 : 0,
                noStroke ? 1 : 0, compositeSame ? 1 : 0, view->history_count() - historyBefore);
        if (begun || !noStroke || !compositeSame || !noHistory) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(316, "paint lock refusal");
        }
        frame.closeDocument(doc, false);
    }

    // llk_active_lock_cursor (395): the refusal cursor follows the ACTIVE layer,
    // not the topmost. A pixel-locked active layer with an unlocked topmost must
    // still show Forbidden and refuse the stroke.
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockActive"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* tools = frame.findChild<pictura::ToolController*>();
        if (!created || !view || !tools) {
            return pictura::selfTest().fail(395, "active lock cursor fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString active = view->add_layer_in(QString());
        view->add_layer_in(QString());
        view->set_active_layer(active);
        view->set_layer_name_path(active, QStringLiteral("LockedActive"));
        view->set_layers_lock(QStringList{active}, QStringLiteral("pixels"), true);
        frame.setActiveTool(pictura::ToolId::Brush);
        tools->refreshCursor();
        const bool forbidden = frame.imageView()->cursor().shape() == Qt::ForbiddenCursor;
        const bool refused = !view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 4, 100, 100, 0, 100,
                                                100, 25, QStringLiteral("normal"), false, false);
        const bool noStroke = !view->is_painting();
        ST_BEGIN("llk_active_lock_cursor");
        ST_PASS("llk_active_lock_cursor forbidden=%d refused=%d stroke=%d", forbidden ? 1 : 0,
                refused ? 1 : 0, noStroke ? 1 : 0);
        if (!forbidden || !refused || !noStroke) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(395, "active-layer lock cursor");
        }
        frame.closeDocument(doc, false);
    }

    // llk_topmost_lock_paints (396): a locked topmost layer must not blank the
    // cursor or refuse an edit that targets a lower, editable active layer.
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockTop"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* tools = frame.findChild<pictura::ToolController*>();
        if (!created || !view || !tools) {
            return pictura::selfTest().fail(396, "topmost lock fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString active = view->add_layer_in(QString());
        const QString top = view->add_layer_in(QString());
        view->set_layers_lock(QStringList{top}, QStringLiteral("pixels"), true);
        view->set_active_layer(active);
        frame.setActiveTool(pictura::ToolId::Brush);
        tools->refreshCursor();
        const bool blank = frame.imageView()->cursor().shape() == Qt::BlankCursor;
        const unsigned int before = view->composite_argb(4, 4);
        const bool painted = view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 4, 100, 100, 0, 100,
                                               100, 25, QStringLiteral("normal"), false, false)
            && view->paint_dab(4, 4, 1.0) && view->end_paint();
        const bool changed = before != view->composite_argb(4, 4);
        ST_BEGIN("llk_topmost_lock_paints");
        ST_PASS("llk_topmost_lock_paints blank=%d painted=%d changed=%d", blank ? 1 : 0,
                painted ? 1 : 0, changed ? 1 : 0);
        if (!blank || !painted || !changed) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(396, "topmost lock must not refuse a lower edit");
        }
        frame.closeDocument(doc, false);
    }

    // llk_panel_multi_refusal (397): a panel multi-selection (and a cleared
    // selection) resolves to no single active layer, so the brush refuses.
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockPanel"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(397, "panel multi fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString a = view->add_layer_in(QString());
        const QString b = view->add_layer_in(QString());
        panel->setView(view);
        panel->refresh();
        panel->selectPaths(QStringList{a, b}, a);
        const bool noSingle = view->active_layer_path().isEmpty();
        const bool multiRefused = !view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 4, 100, 100, 0,
                                                     100, 100, 25, QStringLiteral("normal"),
                                                     false, false);
        panel->selectPaths(QStringList{}, QString());
        const bool nothingActive = view->active_layer_path().isEmpty();
        const bool noneRefused = !view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 4, 100, 100, 0,
                                                    100, 100, 25, QStringLiteral("normal"),
                                                    false, false);
        const bool noStroke = !view->is_painting();
        ST_BEGIN("llk_panel_multi_refusal");
        ST_PASS("llk_panel_multi_refusal noSingle=%d multi=%d none=%d noneRefused=%d",
                noSingle ? 1 : 0, multiRefused ? 1 : 0, nothingActive ? 1 : 0,
                noneRefused ? 1 : 0);
        if (!noSingle || !multiRefused || !nothingActive || !noneRefused || !noStroke) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(397, "panel multi/none selection must refuse");
        }
        frame.closeDocument(doc, false);
    }

    // llk_transparency_180 (398): a transparency lock through the bridge keeps an
    // A=180 pixel's alpha exactly while recolouring its RGB, and leaves an A=0
    // pixel untouched. Documented divergence: content moves keep their existing
    // transparency refusal, which this paint-path lock does not model.
    const QString alphaPath =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-alpha180.png");
    {
        const bool created = frame.newDocument(QStringLiteral("LayerLockAlpha"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(398, "transparency alpha fixture");
        }
        const int doc = frame.activeDocumentIndex();
        QImage seeded(8, 8, QImage::Format_ARGB32);
        seeded.fill(Qt::transparent);
        seeded.setPixel(6, 6, qRgba(0, 0, 0, 180));
        const bool opened = seeded.save(alphaPath, "PNG") && view->open_image(alphaPath);
        view->set_active_layer(QStringLiteral("0"));
        const unsigned int before = view->sample_argb(6, 6);
        view->set_layers_lock(QStringList{QStringLiteral("0")}, QStringLiteral("transparency"),
                              true);
        const bool painted = view->begin_paint(0xFF0000FFu, 0xFFFFFFFFu, 4, 100, 100, 0, 100, 100,
                                               25, QStringLiteral("normal"), false, false)
            && view->paint_dab(6, 6, 1.0) && view->end_paint();
        const unsigned int after = view->sample_argb(6, 6);
        const bool alphaKept = qAlpha(before) == 180 && qAlpha(after) == 180;
        const bool rgbChanged = (after & 0x00FFFFFFu) != (before & 0x00FFFFFFu);
        const bool clearUntouched = qAlpha(view->sample_argb(0, 0)) == 0;
        ST_BEGIN("llk_transparency_180");
        ST_PASS("llk_transparency_180 opened=%d a0=%d a1=%d rgb=%d clear=%d", opened ? 1 : 0,
                qAlpha(before), qAlpha(after), rgbChanged ? 1 : 0, clearUntouched ? 1 : 0);
        if (!opened || !alphaKept || !rgbChanged || !clearUntouched || !painted) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(398, "transparency lock must keep alpha 180");
        }
        frame.closeDocument(doc, false);
    }
    QFile::remove(alphaPath);

    return 0;
}
