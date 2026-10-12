#include "selftest_layers_interactions.h"
#include "selftest_report.h"
#include "api/layers_api.h"

#include "frame.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

namespace {

int rowForPath(PictureView* view, const QString& path)
{
    for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
        if (pictura::layers_api::rowAt(*view, i).path == path) {
            return i;
        }
    }
    return -1;
}

} // namespace

int runLayersInteractionsChecks(PicturaMainWindow& frame)
{
    // lpr_drag_sibling (328): the model advertises drag/drop, a gap above a
    // sibling resolves to an above/below drop (not "into"), and the reorder is
    // one undo step.
    {
        const bool created = frame.newDocument(QStringLiteral("DragReorder"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        auto* panel = frame.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(328, "sibling drag fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString a = view->add_layer_in(QString());
        const QString b = view->add_layer_in(QString());
        view->set_layer_name_path(a, QStringLiteral("A"));
        view->set_layer_name_path(b, QStringLiteral("B"));
        panel->refresh();
        const bool supported = panel->layerRowDragSupportedForTest();
        const int mode = panel->dragMoveModeAtForTest(a, b, true);
        const bool indicator = panel->dropIndicatorShownForTest();
        const int base = view->history_count();
        const bool dropped = panel->dropAtForTest(a, b, true);
        const int rowA = rowForPath(view, a);
        const int rowB = rowForPath(view, b);
        const bool reordered = rowA >= 0 && rowB >= 0
            && pictura::layers_api::rowAt(*view, rowB).name == QStringLiteral("A")
            && pictura::layers_api::rowAt(*view, rowA).name == QStringLiteral("B");
        const bool oneStep = view->history_count() == base + 1;
        ST_BEGIN("lpr_drag_sibling");
        ST_PASS("lpr_drag_sibling support=%d mode=%d indicator=%d drop=%d reorder=%d history=%d",
                supported ? 1 : 0, mode, indicator ? 1 : 0, dropped ? 1 : 0,
                reordered ? 1 : 0, view->history_count() - base);
        if (!supported || (mode != 0 && mode != 1) || !indicator || !dropped || !reordered
            || !oneStep) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(328, "sibling drag reorder");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_drag_invalid (329): the Background cannot be moved; the drop shows no
    // indicator and the document gains no history.
    {
        const bool created = frame.newDocument(QStringLiteral("DragInvalid"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        PictureView* view = frame.activeView();
        auto* panel = frame.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(329, "invalid drag fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->background_from_layer(QStringLiteral("0"));
        const QString target = view->add_layer_in(QString());
        panel->refresh();
        const int base = view->history_count();
        const int mode = panel->dragMoveModeAtForTest(QStringLiteral("0"), target, true);
        const bool indicator = panel->dropIndicatorShownForTest();
        const bool dropped = panel->dropAtForTest(QStringLiteral("0"), target, true);
        const bool unchanged = view->history_count() == base;
        const bool stillBackground = pictura::layers_api::rowAt(*view, rowForPath(view, QStringLiteral("0"))).kind
            == QStringLiteral("background");
        ST_BEGIN("lpr_drag_invalid");
        ST_PASS("lpr_drag_invalid mode=%d indicator=%d drop=%d history=%d background=%d", mode,
                indicator ? 1 : 0, dropped ? 1 : 0, view->history_count() - base,
                stillBackground ? 1 : 0);
        if (indicator || !unchanged || !stillBackground) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(329, "invalid drop target");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_background_dblclick (330): a double-click outside the name converts
    // the Background to a normal, unlocked layer in one undo step.
    {
        const bool created = frame.newDocument(QStringLiteral("BgDbl"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        PictureView* view = frame.activeView();
        auto* panel = frame.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(330, "background dblclick fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->set_layers_lock(QStringList{QStringLiteral("0")}, QStringLiteral("transparency"),
                              true);
        view->background_from_layer(QStringLiteral("0"));
        panel->refresh();
        const QString path = QStringLiteral("0");
        const bool wasBackground =
            pictura::layers_api::rowAt(*view, rowForPath(view, path)).kind == QStringLiteral("background");
        const int base = view->history_count();
        panel->setBackgroundConvertForTest(true, QString(), 0);
        panel->doubleClickAtForTest(path, true);
        const int row = rowForPath(view, path);
        const bool converted = row >= 0 && pictura::layers_api::rowAt(*view, row).kind == QStringLiteral("pixel");
        const bool unlocked = row >= 0 && pictura::layers_api::rowAt(*view, row).lockBits == 0;
        const bool oneStep = view->history_count() == base + 1;
        ST_BEGIN("lpr_background_dblclick");
        ST_PASS("lpr_background_dblclick before=%d after=%s lock=%d history=%d",
                wasBackground ? 1 : 0, qPrintable(pictura::layers_api::rowAt(*view, row).kind),
                pictura::layers_api::rowAt(*view, row).lockBits, view->history_count() - base);
        if (!wasBackground || !converted || !unlocked || !oneStep) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(330, "background double-click convert");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_background_drop (331): dropping the Background on New Layer converts
    // it in place; no `Background copy` is created and one state is recorded.
    {
        const bool created = frame.newDocument(QStringLiteral("BgDrop"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        PictureView* view = frame.activeView();
        auto* panel = frame.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(331, "background drop fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->background_from_layer(QStringLiteral("0"));
        panel->refresh();
        panel->setBackgroundConvertForTest(true, QString(), 0);
        const int base = view->history_count();
        const bool dropped = panel->dropOnStripButtonForTest(
            QStringLiteral("layersStripNewLayer"), QStringList{QStringLiteral("0")});
        bool copyExists = false;
        for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
            if (pictura::layers_api::rowAt(*view, i).name.contains(QStringLiteral("copy"))) {
                copyExists = true;
            }
        }
        const int row = rowForPath(view, QStringLiteral("0"));
        const bool converted = row >= 0 && pictura::layers_api::rowAt(*view, row).kind == QStringLiteral("pixel");
        const bool oneStep = view->history_count() == base + 1;
        ST_BEGIN("lpr_background_drop");
        ST_PASS("lpr_background_drop dropped=%d copy=%d kind=%s history=%d", dropped ? 1 : 0,
                copyExists ? 1 : 0, qPrintable(pictura::layers_api::rowAt(*view, row).kind),
                view->history_count() - base);
        if (!dropped || copyExists || !converted || !oneStep) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(331, "background drop convert");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_thumbnail_select (332): a Ctrl+click on the thumbnail replaces the
    // selection with the layer's opaque pixels, starts no editor, and keeps the
    // row selected.
    {
        const bool created = frame.newDocument(QStringLiteral("ThumbSelect"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        auto* panel = frame.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(332, "thumbnail select fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        // A previous check may have persisted thumbnails off; force them on.
        panel->setOptionsForTest(2, 1, false);
        panel->refresh();
        const QString path = QStringLiteral("0");
        panel->selectPaths(QStringList{path}, path);
        const bool selectedBefore = panel->currentPath() == path;
        const int base = view->history_count();
        const bool consumed = panel->ctrlClickThumbnailForTest(path);
        const int pixels = view->selection_count();
        const bool noEditor = !panel->inlineEditorOpenForTest();
        const bool keptRow = panel->currentPath() == path;
        const bool oneStep = view->history_count() == base + 1;
        ST_BEGIN("lpr_thumbnail_select");
        ST_PASS("lpr_thumbnail_select consumed=%d pixels=%d editor=%d row=%d history=%d",
                consumed ? 1 : 0, pixels, noEditor ? 1 : 0, keptRow ? 1 : 0,
                view->history_count() - base);
        if (!selectedBefore || !consumed || pixels != 64 || !noEditor || !keptRow || !oneStep) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(332, "thumbnail alpha selection");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_drag_nesting (333): a nesting-locked group refuses a reparent drop of
    // its child; no indicator, no history, parent unchanged.
    {
        const bool created = frame.newDocument(QStringLiteral("NestDrop"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        auto* panel = frame.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(333, "nesting drop fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString group = view->add_group_in(QString());
        const QString child = view->add_layer_in(group);
        const QString sibling = view->add_layer_in(QString());
        view->set_layers_lock(QStringList{group}, QStringLiteral("nesting"), true);
        panel->refresh();
        const int base = view->history_count();
        const int mode = panel->dragMoveModeAtForTest(child, sibling, true);
        const bool indicator = panel->dropIndicatorShownForTest();
        const bool dropped = panel->dropAtForTest(child, sibling, true);
        const bool unchanged = view->history_count() == base
            && rowForPath(view, child) >= 0
            && pictura::layers_api::rowAt(*view, rowForPath(view, child)).path == child;
        ST_BEGIN("lpr_drag_nesting");
        ST_PASS("lpr_drag_nesting mode=%d indicator=%d drop=%d history=%d", mode,
                indicator ? 1 : 0, dropped ? 1 : 0, view->history_count() - base);
        if (indicator || !unchanged) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(333, "nesting-locked reparent drop");
        }
        frame.closeDocument(doc, false);
    }

    return 0;
}

} // namespace pictura
