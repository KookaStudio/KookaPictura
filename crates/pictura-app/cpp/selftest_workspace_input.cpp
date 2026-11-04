#include "selftest_workspace_input.h"
#include "selftest_report.h"

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "api/view_bridge.h"

#include <QtCore/QPointF>
#include <QtCore/Qt>
#include <QtGui/QColor>
#include <QtGui/QCursor>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QApplication>

#include <cmath>

namespace {

void sendWheel(pictura::ImageView* canvas, const QPointF& pos, const QPoint& angle,
               Qt::KeyboardModifiers mods)
{
    QWheelEvent event(pos, canvas->mapToGlobal(pos.toPoint()), QPoint(), angle, Qt::NoButton, mods,
                      Qt::NoScrollPhase, false);
    QApplication::sendEvent(canvas, &event);
}

// The document point under a widget point for the canvas's live pan/zoom.
QPointF imageUnder(const pictura::ImageView* canvas, const QPointF& widgetPos)
{
    return (widgetPos - canvas->offset()) / canvas->zoom();
}

double distance(const QPointF& a, const QPointF& b)
{
    return std::hypot(a.x() - b.x(), a.y() - b.y());
}

} // namespace

int pictura::runWorkspaceInputChecks(pictura::PicturaMainWindow& frame)
{
    const bool created = frame.newDocument(QStringLiteral("WorkspaceInput"), 2000, 2000,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    PictureView* view = frame.activeView();
    ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    if (!created || !view || !canvas || !tools) {
        return pictura::selfTest().fail(372, "workspace input fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const bool armedWithDoc = frame.workspaceOpenArmed();
    frame.resize(800, 600);
    QApplication::processEvents();
    canvas->actualPixels();
    QApplication::processEvents();

    const QPointF centre(canvas->width() / 2.0, canvas->height() / 2.0);
    const QPointF probe(canvas->width() * 0.3, canvas->height() * 0.3);

    // ws_wheel_shift (372): Shift doubles the wheel zoom step. A plain wheel
    // from the same base gives the reference ratio in log space.
    canvas->setZoom(1.0, centre);
    const double baseZoom = canvas->zoom();
    sendWheel(canvas, probe, QPoint(0, 120), Qt::NoModifier);
    const double plainRatio = canvas->zoom() / baseZoom;
    canvas->setZoom(1.0, centre);
    sendWheel(canvas, probe, QPoint(0, 120), Qt::ShiftModifier);
    const double shiftRatio = canvas->zoom() / baseZoom;
    const bool doubled = plainRatio > 1.0
        && std::abs(std::log(shiftRatio) - 2.0 * std::log(plainRatio)) < 1e-6;
    ST_BEGIN("ws_wheel_shift");
    ST_PASS("ws_wheel_shift plain=%.6f shift=%.6f doubled=%d", plainRatio, shiftRatio,
            doubled ? 1 : 0);
    if (!doubled) {
        frame.closeDocument(doc, false);
        return pictura::selfTest().fail(372, "shift must double the wheel zoom step");
    }

    // ws_wheel_alt_pan (373): Alt+vertical wheel pans only the X offset.
    canvas->setZoom(1.0, centre);
    const QPointF altBefore = canvas->offset();
    const double altZoom = canvas->zoom();
    sendWheel(canvas, probe, QPoint(0, 120), Qt::AltModifier);
    const bool altX = canvas->offset().x() != altBefore.x();
    const bool altY = canvas->offset().y() == altBefore.y();
    const bool altZ = canvas->zoom() == altZoom;
    ST_BEGIN("ws_wheel_alt_pan");
    ST_PASS("ws_wheel_alt_pan dx=%d dy=%d zoom=%d", altX ? 1 : 0, altY ? 1 : 0, altZ ? 1 : 0);
    if (!altX || !altY || !altZ) {
        frame.closeDocument(doc, false);
        return pictura::selfTest().fail(373, "alt wheel must pan horizontally only");
    }

    // ws_wheel_ctrl_alt_pan (374): Ctrl+Alt+vertical wheel pans only Y.
    canvas->setZoom(1.0, centre);
    const QPointF caBefore = canvas->offset();
    const double caZoom = canvas->zoom();
    sendWheel(canvas, probe, QPoint(0, 120), Qt::ControlModifier | Qt::AltModifier);
    const bool caX = canvas->offset().x() == caBefore.x();
    const bool caY = canvas->offset().y() != caBefore.y();
    const bool caZ = canvas->zoom() == caZoom;
    ST_BEGIN("ws_wheel_ctrl_alt_pan");
    ST_PASS("ws_wheel_ctrl_alt_pan dx=%d dy=%d zoom=%d", caX ? 1 : 0, caY ? 1 : 0, caZ ? 1 : 0);
    if (!caX || !caY || !caZ) {
        frame.closeDocument(doc, false);
        return pictura::selfTest().fail(374, "ctrl+alt wheel must pan vertically only");
    }

    // ws_wheel_side_pan (375): a horizontal side-wheel pans only X.
    canvas->setZoom(1.0, centre);
    const QPointF sideBefore = canvas->offset();
    const double sideZoom = canvas->zoom();
    sendWheel(canvas, probe, QPoint(120, 0), Qt::NoModifier);
    const bool sideX = canvas->offset().x() != sideBefore.x();
    const bool sideY = canvas->offset().y() == sideBefore.y();
    const bool sideZ = canvas->zoom() == sideZoom;
    ST_BEGIN("ws_wheel_side_pan");
    ST_PASS("ws_wheel_side_pan dx=%d dy=%d zoom=%d", sideX ? 1 : 0, sideY ? 1 : 0, sideZ ? 1 : 0);
    if (!sideX || !sideY || !sideZ) {
        frame.closeDocument(doc, false);
        return pictura::selfTest().fail(375, "side wheel must pan horizontally only");
    }

    // ws_wheel_anchor (376): a plain wheel zooms and leaves the image point
    // under the cursor fixed.
    canvas->setZoom(1.0, centre);
    const QPointF anchorBefore = imageUnder(canvas, probe);
    const double anchorZoomBefore = canvas->zoom();
    sendWheel(canvas, probe, QPoint(0, 120), Qt::NoModifier);
    const bool anchorZoomed = canvas->zoom() > anchorZoomBefore;
    const bool anchored = distance(anchorBefore, imageUnder(canvas, probe)) < 1e-6;
    ST_BEGIN("ws_wheel_anchor");
    ST_PASS("ws_wheel_anchor zoomed=%d drift=%.9f", anchorZoomed ? 1 : 0,
            distance(anchorBefore, imageUnder(canvas, probe)));
    if (!anchorZoomed || !anchored) {
        frame.closeDocument(doc, false);
        return pictura::selfTest().fail(376, "wheel zoom must anchor at the cursor");
    }

    // ws_zoom_click_anchor (377): a Zoom-tool click magnifies and anchors at the
    // clicked point rather than the canvas centre.
    frame.setActiveTool(pictura::ToolId::Zoom);
    canvas->setZoom(1.0, centre);
    const QPointF clickWidget(canvas->width() * 0.3, canvas->height() * 0.3);
    const QPointF clickImage = canvas->widgetToImage(clickWidget);
    const double clickZoomBefore = canvas->zoom();
    // A click (press and release with no drag) steps on release; a drag draws a
    // marquee instead, so the step is not committed at press.
    canvas->mousePressed(clickImage, Qt::LeftButton, int(Qt::NoModifier));
    canvas->mouseReleased(clickImage);
    const bool clickZoomed = canvas->zoom() > clickZoomBefore;
    const bool clickAnchored = distance(clickImage, canvas->widgetToImage(clickWidget)) < 1e-6;
    ST_BEGIN("ws_zoom_click_anchor");
    ST_PASS("ws_zoom_click_anchor zoomed=%d drift=%.9f", clickZoomed ? 1 : 0,
            distance(clickImage, canvas->widgetToImage(clickWidget)));
    if (!clickZoomed || !clickAnchored) {
        frame.closeDocument(doc, false);
        return pictura::selfTest().fail(377, "zoom click must anchor at the click");
    }

    // ws_alt_eyedropper (378): an Alt Brush press samples the foreground with no
    // stroke and no history, leaving the Brush active.
    {
        const bool paintCreated = frame.newDocument(QStringLiteral("AltEyedropper"), 16, 16,
                                                    QStringLiteral("rgb"), 8,
                                                    QStringLiteral("white"));
        PictureView* paintView = frame.activeView();
        ImageView* paintCanvas = frame.imageView();
        if (!paintCreated || !paintView || !paintCanvas) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(378, "alt eyedropper fixture");
        }
        const int paintDoc = frame.activeDocumentIndex();
        paintView->set_active_layer(QStringLiteral("0"));
        const bool dabbed =
            paintView->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100, 25,
                                   QStringLiteral("normal"), true, false)
            && paintView->paint_dab(8.0, 8.0, 1.0) && paintView->end_paint();
        frame.setActiveTool(pictura::ToolId::Brush);
        tools->setForeground(QColor(1, 2, 3));
        const int historyBefore = paintView->history_count();
        paintCanvas->mousePressed(QPointF(8, 8), Qt::LeftButton, int(Qt::AltModifier));
        const QColor sampled = frame.foregroundColor();
        const bool redSampled = dabbed && sampled.red() > 200 && sampled.green() < 80
            && sampled.blue() < 80;
        const bool noStroke = !paintView->is_painting();
        const bool noHistory = paintView->history_count() == historyBefore;
        const bool toolKept = frame.activeTool() == pictura::ToolId::Brush;
        ST_BEGIN("ws_alt_eyedropper");
        ST_PASS("ws_alt_eyedropper dabbed=%d sampled=%02x%02x%02x stroke=%d hist=%d tool=%d",
                dabbed ? 1 : 0, sampled.red(), sampled.green(), sampled.blue(), noStroke ? 1 : 0,
                noHistory ? 1 : 0, toolKept ? 1 : 0);
        frame.closeDocument(paintDoc, false);
        if (!redSampled || !noStroke || !noHistory || !toolKept) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(378, "alt brush press must sample without painting");
        }
    }

    // ws_alt_cursor (379): Alt swaps the blank brush cursor for the eyedropper
    // and releasing Alt restores the blank cursor.
    {
        frame.setActiveTool(pictura::ToolId::Brush);
        tools->refreshCursor(Qt::NoModifier);
        const bool blank = canvas->cursor().shape() == Qt::BlankCursor;
        tools->refreshCursor(Qt::AltModifier);
        const Qt::CursorShape altShape = canvas->cursor().shape();
        const bool eyedropper = altShape != Qt::BlankCursor && altShape != Qt::ForbiddenCursor;
        tools->refreshCursor(Qt::NoModifier);
        const bool restored = canvas->cursor().shape() == Qt::BlankCursor;
        ST_BEGIN("ws_alt_cursor");
        ST_PASS("ws_alt_cursor blank=%d eyedropper=%d restored=%d shape=%d", blank ? 1 : 0,
                eyedropper ? 1 : 0, restored ? 1 : 0, int(altShape));
        if (!blank || !eyedropper || !restored) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(379, "alt cursor must be the eyedropper");
        }
    }

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
    // ws_empty_open (533): the empty-workspace Open gesture is disarmed while
    // any document is open, so a canvas double-click is never hijacked into the
    // Open dialog. (The positive path opens a modal file dialog and is not
    // exercised headlessly.)
    const bool disarmedWithDoc = !armedWithDoc && frame.documentCount() > 0;
    ST_BEGIN("ws_empty_open");
    ST_PASS("ws_empty_open with_doc=%d docs=%d", armedWithDoc ? 1 : 0, frame.documentCount());
    if (!disarmedWithDoc) {
        return pictura::selfTest().fail(533, "an open document must disarm the workspace open gesture");
    }
    return 0;
}
