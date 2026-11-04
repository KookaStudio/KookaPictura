#include "selftest_active_layer.h"
#include "selftest_report.h"

#include "frame.h"
#include "tools.h"

#include "api/view_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/Qt>
#include <QtGui/QColor>
#include <QtGui/QImage>

// Active-layer gating (item 9) and the per-pixel transparency lock (item 10)
// through the bridge. A panel path of "" is how zero/multiple selection is
// represented, so the same empty path stands in for both refusals.
int pictura::runActiveLayerChecks(pictura::PicturaMainWindow& frame)
{
    constexpr unsigned kRed = 0xFFFF0000u;
    constexpr unsigned kWhite = 0xFFFFFFFFu;

    // lal_no_active_refuses (334): with no active layer a paint and a filter are
    // both refused, no stroke starts, and no history state is added.
    {
        const bool created = frame.newDocument(QStringLiteral("ActiveNone"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(334, "no-active fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_active_layer(QString());
        const int historyBefore = view->history_count();
        const bool painted = view->begin_paint(kRed, kWhite, 8, 100, 100, 0, 100, 100, 25,
                                               QStringLiteral("normal"), false, false);
        const bool noStroke = !view->is_painting();
        const bool filtered = view->apply_filter(QStringLiteral("add-noise"));
        const bool moved = view->begin_move_preview() || view->commit_move(1, 1);
        const bool noHistory = view->history_count() == historyBefore;
        ST_BEGIN("lal_no_active_refuses");
        ST_PASS("lal_no_active painted=%d stroke=%d filtered=%d moved=%d hist=%d", painted ? 1 : 0,
                noStroke ? 1 : 0, filtered ? 1 : 0, moved ? 1 : 0,
                view->history_count() - historyBefore);
        if (painted || !noStroke || filtered || moved || !noHistory) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(334, "no active layer must refuse");
        }
        frame.closeDocument(doc, false);
    }

    // lal_active_target (335): a single active layer is the edit target. A green
    // stroke on the lower layer is hidden behind the opaque blue top layer, so
    // the top layer was not touched; hiding it reveals the green edit.
    {
        const bool created = frame.newDocument(QStringLiteral("ActiveTarget"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(335, "active-target fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const QString top = view->add_layer_in(QString());
        view->set_active_layer(top);
        const bool blue = view->begin_paint(0xFF0000FFu, kWhite, 4, 100, 100, 0, 100, 100, 25,
                                            QStringLiteral("normal"), false, false)
            && view->paint_dab(5, 5, 1.0) && view->end_paint();
        view->set_active_layer(QStringLiteral("0"));
        const bool green = view->begin_paint(0xFF00FF00u, kWhite, 4, 100, 100, 0, 100, 100, 25,
                                             QStringLiteral("normal"), false, false)
            && view->paint_dab(5, 5, 1.0) && view->end_paint();
        const unsigned int covered = view->sample_argb(5, 5);
        const bool coveredBlue = qBlue(covered) > 200 && qGreen(covered) < 60;
        view->set_layer_visible(1, false);
        const unsigned int revealed = view->sample_argb(5, 5);
        const bool revealedGreen = qGreen(revealed) > 200 && qBlue(revealed) < 60;
        view->set_layer_visible(1, true);
        ST_BEGIN("lal_active_target");
        ST_PASS("lal_active_target blue=%d green=%d covered_blue=%d revealed_green=%d",
                blue ? 1 : 0, green ? 1 : 0, coveredBlue ? 1 : 0, revealedGreen ? 1 : 0);
        if (!blue || !green || !coveredBlue || !revealedGreen) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(335, "active layer must receive the edit");
        }
        frame.closeDocument(doc, false);
    }

    // lal_multi_refuses (336): more than one selected layer (an empty panel path)
    // refuses the edit and leaves the document unchanged.
    {
        const bool created = frame.newDocument(QStringLiteral("ActiveMulti"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(336, "multi fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->add_layer_in(QString());
        view->set_active_layer(QString());
        const int historyBefore = view->history_count();
        const bool painted = view->begin_paint(kRed, kWhite, 4, 100, 100, 0, 100, 100, 25,
                                               QStringLiteral("normal"), false, false);
        const bool noHistory = view->history_count() == historyBefore;
        ST_BEGIN("lal_multi_refuses");
        ST_PASS("lal_multi painted=%d hist=%d", painted ? 1 : 0,
                view->history_count() - historyBefore);
        if (painted || view->is_painting() || !noHistory) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(336, "multi selection must refuse");
        }
        frame.closeDocument(doc, false);
    }

    // lal_transparency_alpha (338): a transparency-locked stroke keeps each
    // pixel's alpha and leaves a fully transparent pixel untouched.
    {
        const bool created = frame.newDocument(QStringLiteral("ActiveAlpha"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(338, "alpha fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_active_layer(QStringLiteral("0"));
        view->begin_paint(kRed, kWhite, 4, 100, 100, 0, 70, 100, 25,
                          QStringLiteral("normal"), false, false);
        view->paint_dab(6, 6, 1.0);
        view->end_paint();
        const unsigned int before = view->sample_argb(6, 6);
        const int alphaBefore = qAlpha(before);
        view->set_layers_lock(QStringList{QStringLiteral("0")}, QStringLiteral("transparency"),
                              true);
        view->begin_paint(0xFF0000FFu, kWhite, 4, 100, 100, 0, 100, 100, 25,
                          QStringLiteral("normal"), false, false);
        view->paint_dab(6, 6, 1.0);
        view->end_paint();
        const unsigned int after = view->sample_argb(6, 6);
        const bool alphaKept = qAlpha(after) == alphaBefore && alphaBefore > 0
            && alphaBefore < 255;
        const bool colourChanged = (after & 0x00FFFFFFu) != (before & 0x00FFFFFFu);
        const bool clearPixelUntouched = qAlpha(view->sample_argb(0, 0)) == 0;
        ST_BEGIN("lal_transparency_alpha");
        ST_PASS("lal_transparency a0=%d a1=%d rgb=%d clear=%d", alphaBefore, qAlpha(after),
                colourChanged ? 1 : 0, clearPixelUntouched ? 1 : 0);
        if (!alphaKept || !colourChanged || !clearPixelUntouched) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(338, "transparency lock must keep alpha");
        }
        frame.closeDocument(doc, false);
    }

    // lal_import_activates (339): an imported opaque raster becomes the locked
    // Background and is made active so a following tool edit targets it.
    {
        const bool created = frame.newDocument(QStringLiteral("ActiveImport"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(339, "import fixture");
        }
        const int doc = frame.activeDocumentIndex();
        QImage image(8, 8, QImage::Format_RGB32);
        image.fill(QColor(200, 30, 30));
        const QString path = QDir::tempPath() + QStringLiteral("/kooka-pictura-active-layer.png");
        const bool opened = image.save(path) && view->open_image(path);
        const bool active = view->active_layer_path() == QStringLiteral("0");
        const bool background = view->layer_kind(0) == QStringLiteral("background");
        const bool paintable = view->begin_paint(kRed, kWhite, 4, 100, 100, 0, 100, 100, 25,
                                                 QStringLiteral("normal"), false, false);
        if (paintable) {
            view->cancel_paint();
        }
        ST_BEGIN("lal_import_activates");
        ST_PASS("lal_import opened=%d active=%d background=%d paintable=%d", opened ? 1 : 0,
                active ? 1 : 0, background ? 1 : 0, paintable ? 1 : 0);
        if (!opened || !active || !background || !paintable) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(339, "import must activate the Background");
        }
        frame.closeDocument(doc, false);
    }

    return 0;
}
