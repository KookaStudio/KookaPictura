#include "selftest_paint_live.h"
#include "selftest_report.h"

#include "frame.h"
#include "image_view.h"

#include "api/view_bridge.h"

#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>

namespace {

bool sameImage(const QImage& a, const QImage& b)
{
    if (a.size() != b.size()) {
        return false;
    }
    for (int y = 0; y < a.height(); ++y) {
        for (int x = 0; x < a.width(); ++x) {
            if (a.pixel(x, y) != b.pixel(x, y)) {
                return false;
            }
        }
    }
    return true;
}

// True when a strongly red pixel sits within `radius` of `widgetPos`.
bool hasRedNear(const QImage& image, const QPointF& widgetPos, int radius)
{
    const int cx = qRound(widgetPos.x());
    const int cy = qRound(widgetPos.y());
    for (int y = cy - radius; y <= cy + radius; ++y) {
        for (int x = cx - radius; x <= cx + radius; ++x) {
            if (x < 0 || y < 0 || x >= image.width() || y >= image.height()) {
                continue;
            }
            const QColor color = image.pixelColor(x, y);
            if (color.red() > 180 && color.green() < 80 && color.blue() < 80) {
                return true;
            }
        }
    }
    return false;
}

} // namespace

int pictura::runPaintLiveChecks(pictura::PicturaMainWindow& frame)
{
    frame.resize(1100, 700);
    QApplication::processEvents();

    // pp_live_visible (346) at zoom < 1 and pp_live_visible_zoom (347) at
    // zoom > 1: a dab must be on the canvas before release. The pre-fix
    // translate-then-scale patch draws the region at document coordinates in
    // the scaled cache, so at zoom < 1 it is clipped off the cache entirely
    // and at zoom > 1 it lands at the wrong place; either way this check fails.
    const double zooms[2] = {0.5, 1.5};
    const int codes[2] = {346, 347};
    const char* names[2] = {"pp_live_visible", "pp_live_visible_zoom"};
    for (int i = 0; i < 2; ++i) {
        const bool created = frame.newDocument(QStringLiteral("PaintLive"), 240, 240,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        ImageView* canvas = frame.imageView();
        if (!created || !view || !canvas) {
            return pictura::selfTest().fail(codes[i], "paint live fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_active_layer(QStringLiteral("0"));
        canvas->setZoom(zooms[i], QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
        QApplication::processEvents();
        // Warm the scaled present cache at this zoom so the dab takes the
        // in-place patch path rather than a rebuild.
        QImage warm(canvas->size(), QImage::Format_ARGB32);
        canvas->render(&warm);
        canvas->render(&warm);

        const bool begun = view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 60, 100, 100, 0, 100, 100,
                                             25, QStringLiteral("normal"), false, false);
        const bool dabA = view->paint_dab(110.0, 120.0, 1.0);
        const bool dabB = view->paint_dab(120.0, 120.0, 1.0);
        const bool dabC = view->paint_dab(130.0, 120.0, 1.0);
        const int dabs = (dabA ? 1 : 0) + (dabB ? 1 : 0) + (dabC ? 1 : 0);
        // Presents are frame-bounded; show the accumulated dabs on demand.
        view->flush_present();
        const QImage before = canvas->grab().toImage();
        const QPointF expected(120.0 * canvas->zoom() + canvas->offset().x(),
                               120.0 * canvas->zoom() + canvas->offset().y());
        const bool visible = hasRedNear(before, expected, 6);
        view->end_paint();

        const bool ok = begun && dabs >= 1 && visible;
        ST_BEGIN(names[i]);
        ST_PASS("%s begun=%d dabs=%d visible=%d zoom=%.2f", names[i], begun ? 1 : 0, dabs,
                visible ? 1 : 0, canvas->zoom());
        frame.closeDocument(doc, false);
        if (!ok) {
            return pictura::selfTest().fail(codes[i], "dab not live at zoom %.2f", zooms[i]);
        }
    }

    // pp_live_commit (348): several dabs record exactly one history state on
    // release, and undo restores the pre-stroke image.
    {
        const bool created = frame.newDocument(QStringLiteral("PaintCommit"), 240, 240,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(348, "paint commit fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_active_layer(QStringLiteral("0"));
        const QImage pre = view->image();
        const int base = view->history_count();
        const bool begun = view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 40, 100, 100, 0, 100, 100,
                                             25, QStringLiteral("normal"), false, false);
        int dabs = 0;
        for (int x = 80; x <= 160; x += 10) {
            if (view->paint_dab(double(x), 120.0, 1.0)) {
                ++dabs;
            }
        }
        const bool ended = view->end_paint();
        const int after = view->history_count();
        const bool oneState = ended && after == base + 1;
        const bool undone = view->undo();
        const QImage restored = view->image();
        const bool restoredImage = sameImage(pre, restored);

        ST_BEGIN("pp_live_commit");
        ST_PASS("pp_live_commit begun=%d dabs=%d base=%d after=%d one=%d undone=%d restored=%d",
                begun ? 1 : 0, dabs, base, after, oneState ? 1 : 0, undone ? 1 : 0,
                restoredImage ? 1 : 0);
        const bool ok = begun && dabs >= 4 && oneState && undone && restoredImage;
        frame.closeDocument(doc, false);
        if (!ok) {
            return pictura::selfTest().fail(348, "stroke did not commit one undoable state");
        }
    }

    return 0;
}
