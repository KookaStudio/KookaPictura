#include "selftest_visibility.h"
#include "selftest_report.h"

#include "api/transform_api.h"
#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/Qt>
#include <QtGui/QCursor>
#include <QtGui/QImage>

int pictura::runVisibilityChecks(pictura::PicturaMainWindow& frame)
{
    constexpr unsigned kRed = 0xFFFF0000u;
    constexpr unsigned kWhite = 0xFFFFFFFFu;

    // vis_toggle_region (363): the eye toggle on a large canvas takes the
    // region fast path (regionBlitted fires, no full changed recomposite) and
    // records exactly one history state. The measured toggle time is recorded;
    // the sub-second target is enforced only on the reference run so a slow
    // shared machine does not fail the check.
    {
        const bool created = frame.newDocument(QStringLiteral("VisRegion"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(363, "visibility region fixture");
        }
        const int doc = frame.activeDocumentIndex();
        // Grow the canvas around the small layer so the toggle's region is the
        // layer, not the whole document.
        const bool resized = view->resize_canvas(QStringLiteral("center"), 4000, 4000);
        if (!resized) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(363, "visibility resize fixture");
        }
        view->set_active_layer(QStringLiteral("0"));

        int blits = 0;
        int changed = 0;
        auto blitConn = QObject::connect(
            view, &PictureView::regionBlitted,
            [&blits](const QImage&, int, int) { ++blits; });
        auto changedConn =
            QObject::connect(view, &PictureView::changed, [&changed]() { ++changed; });
        const int historyBefore = view->history_count();
        QElapsedTimer clock;
        clock.start();
        const int toggled =
            view->set_layers_visible(QStringList{QStringLiteral("0")}, false);
        const qint64 elapsedMs = clock.elapsed();
        QObject::disconnect(blitConn);
        QObject::disconnect(changedConn);

        const bool invisible = !view->layer_visible(0);
        const bool regionFired = blits >= 1;
        const bool noFull = changed == 0;
        const bool oneState = view->history_count() == historyBefore + 1;
        const bool reference = qEnvironmentVariableIsSet("PICTURA_REFERENCE_RUN");
        const bool underTarget = !reference || elapsedMs < 1000;
        ST_BEGIN("vis_toggle_region");
        ST_PASS("vis_toggle_region toggled=%d invisible=%d blits=%d changed=%d hist=%d ms=%lld "
                "ref=%d",
                toggled, invisible ? 1 : 0, blits, changed,
                view->history_count() - historyBefore, static_cast<long long>(elapsedMs),
                reference ? 1 : 0);
        if (toggled != 1 || !invisible || !regionFired || !noFull || !oneState || !underTarget) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(363, "visibility toggle must use the region path");
        }
        frame.closeDocument(doc, false);
    }

    // vis_invisible_paint (364): a paint tool over an invisible active layer
    // shows the Block/Forbidden cursor, begins no stroke, and adds no history.
    {
        const bool created = frame.newDocument(QStringLiteral("VisPaint"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        auto* tools = frame.findChild<pictura::ToolController*>();
        if (!created || !view || !tools || !frame.imageView()) {
            return pictura::selfTest().fail(364, "invisible paint fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_layers_visible(QStringList{QStringLiteral("0")}, false);
        view->set_active_layer(QStringLiteral("0"));
        frame.setActiveTool(pictura::ToolId::Brush);
        tools->refreshCursor();
        const bool forbidden = frame.imageView()->cursor().shape() == Qt::ForbiddenCursor;

        const int historyBefore = view->history_count();
        const bool begun = view->begin_paint(kRed, kWhite, 8, 100, 100, 0, 100, 100, 25,
                                             QStringLiteral("normal"), false, false);
        const bool noStroke = !view->is_painting();
        const bool noHistory = view->history_count() == historyBefore;
        ST_BEGIN("vis_invisible_paint");
        ST_PASS("vis_invisible_paint forbidden=%d begun=%d stroke=%d hist=%d",
                forbidden ? 1 : 0, begun ? 1 : 0, noStroke ? 1 : 0,
                view->history_count() - historyBefore);
        if (!forbidden || begun || !noStroke || !noHistory) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(364, "invisible active layer must refuse paint");
        }
        frame.closeDocument(doc, false);
    }

    // vis_invisible_move (365): a Move drag on an invisible active layer
    // translates it, records exactly one state, and leaves it invisible.
    {
        const bool created = frame.newDocument(QStringLiteral("VisMove"), 32, 32,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        ImageView* canvas = frame.imageView();
        if (!created || !view || !canvas) {
            return pictura::selfTest().fail(365, "invisible move fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_layers_visible(QStringList{QStringLiteral("0")}, false);
        view->set_active_layer(QStringLiteral("0"));
        const QString rectBefore = transform_api::layerRect(*view, QStringLiteral("0")).toString();
        const int historyBefore = view->history_count();

        frame.setActiveTool(pictura::ToolId::Move);
        canvas->mousePressed(QPointF(8, 8), Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(QPointF(16, 16));
        canvas->mouseReleased(QPointF(16, 16));

        const bool moved = transform_api::layerRect(*view, QStringLiteral("0")).toString() != rectBefore;
        const bool stillInvisible = !view->layer_visible(0);
        const bool oneState = view->history_count() == historyBefore + 1;
        ST_BEGIN("vis_invisible_move");
        ST_PASS("vis_invisible_move moved=%d invisible=%d hist=%d", moved ? 1 : 0,
                stillInvisible ? 1 : 0, view->history_count() - historyBefore);
        if (!moved || !stillInvisible || !oneState) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(365, "invisible layer must still move");
        }
        frame.closeDocument(doc, false);
    }

    // vis_invisible_select (366): selection and Layer via Copy are not refused
    // on an invisible active layer.
    {
        const bool created = frame.newDocument(QStringLiteral("VisSelect"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(366, "invisible select fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_layers_visible(QStringList{QStringLiteral("0")}, false);
        view->set_active_layer(QStringLiteral("0"));

        view->select_all();
        const bool selected = view->has_selection();
        const int historyBefore = view->history_count();
        const QString copy = view->layer_via_copy(QStringLiteral("0"));
        const bool copied = !copy.isEmpty();
        const bool oneState = view->history_count() == historyBefore + 1;
        ST_BEGIN("vis_invisible_select");
        ST_PASS("vis_invisible_select selected=%d copied=%d hist=%d", selected ? 1 : 0,
                copied ? 1 : 0, view->history_count() - historyBefore);
        if (!selected || !copied || !oneState) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(366, "invisible layer must allow selection/copy");
        }
        frame.closeDocument(doc, false);
    }

    // vis_cursor_refresh (383): toggling the active layer's visibility updates
    // the canvas cursor without a pointer move: hidden shows the Block cursor,
    // visible restores the tool cursor.
    {
        const bool created = frame.newDocument(QStringLiteral("VisCursor"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        ImageView* canvas = frame.imageView();
        if (!created || !view || !canvas) {
            return pictura::selfTest().fail(383, "visibility cursor fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_active_layer(QStringLiteral("0"));
        frame.setActiveTool(pictura::ToolId::Brush);
        const bool visible = canvas->cursor().shape() != Qt::ForbiddenCursor;
        view->set_layers_visible(QStringList{QStringLiteral("0")}, false);
        const bool hidden = canvas->cursor().shape() == Qt::ForbiddenCursor;
        view->set_layers_visible(QStringList{QStringLiteral("0")}, true);
        const bool restored = canvas->cursor().shape() != Qt::ForbiddenCursor;
        ST_BEGIN("vis_cursor_refresh");
        ST_PASS("vis_cursor_refresh visible=%d hidden=%d restored=%d", visible ? 1 : 0,
                hidden ? 1 : 0, restored ? 1 : 0);
        if (!visible || !hidden || !restored) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(383, "layer visibility cursor refresh");
        }
        frame.closeDocument(doc, false);
    }

    return 0;
}
