#include "selftest_large_doc.h"
#include "selftest_report.h"

#include "frame.h"
#include "image_view.h"

#include "api/view_bridge.h"

#include <QtCore/QString>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>

namespace {

// Byte-for-byte widget-image equality (both grabs share the same format).
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

} // namespace

namespace pictura {

int runLargeDocumentChecks(pictura::PicturaMainWindow& frame)
{
    // ldoc_selection_no_recomposite (542): a selection-only edit must not run
    // the compositor. `canvas_revision` only bumps when the composite/pyramid is
    // rebuilt, so it must be unchanged across select/invert/deselect/select-all,
    // while the selection state actually changes.
    {
        const bool created = frame.newDocument(QStringLiteral("SelectRefresh"), 64, 64,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(542, "selection refresh fixture");
        }
        const int doc = frame.activeDocumentIndex();
        const quint64 before = view->canvas_revision();

        const bool selected = view->select_rect(0, 0, 16, 16, QStringLiteral("new"), 0.0);
        const bool hasRect = view->has_selection();
        const bool inverted = view->invert_selection();
        view->deselect();
        const bool gone = !view->has_selection();
        view->select_all();
        const bool all = view->has_selection();
        const quint64 after = view->canvas_revision();
        const bool noRecomposite = before == after;

        ST_BEGIN("ldoc_selection_no_recomposite");
        ST_PASS("ldoc_selection_no_recomposite before=%llu after=%llu rect=%d inverted=%d "
                "deselected=%d select_all=%d",
                static_cast<unsigned long long>(before), static_cast<unsigned long long>(after),
                hasRect ? 1 : 0, inverted ? 1 : 0, gone ? 1 : 0, all ? 1 : 0);
        if (!selected || !hasRect || !inverted || !gone || !all || !noRecomposite) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(542, "selection edit ran the compositor");
        }
        frame.closeDocument(doc, false);
    }

    // ldoc_paint_presents_from_crop (543): while a stroke is in progress the
    // canvas must present the view-pyramid level crop, not the full-resolution
    // image. Compare the crop render against the crop-disabled full-resolution
    // draw mid-stroke; they must be identical, with the stroke still active.
    {
        const bool created = frame.newDocument(QStringLiteral("PaintCrop"), 512, 512,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        ImageView* canvas = frame.imageView();
        if (!created || !view || !canvas) {
            return pictura::selfTest().fail(543, "paint crop fixture");
        }
        const int doc = frame.activeDocumentIndex();
        view->set_active_layer(QStringLiteral("0"));
        frame.resize(480, 420);
        QApplication::processEvents();
        canvas->actualPixels();
        QApplication::processEvents();
        QImage warm(canvas->size(), QImage::Format_ARGB32);
        canvas->render(&warm);
        canvas->render(&warm);

        const bool begun = view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 24, 100, 100, 0, 100, 100,
                                             25, QStringLiteral("normal"), false, false);
        const bool dabbed = view->paint_dab(256.0, 256.0, 1.0);
        const bool painting = view->is_painting();
        QImage crop(canvas->size(), QImage::Format_ARGB32);
        canvas->render(&crop);
        canvas->setPresentLevelCropForTest(false);
        QImage full(canvas->size(), QImage::Format_ARGB32);
        canvas->render(&full);
        canvas->setPresentLevelCropForTest(true);
        view->cancel_paint();
        const bool identical = sameImage(crop, full);

        ST_BEGIN("ldoc_paint_presents_from_crop");
        ST_PASS("ldoc_paint_presents_from_crop begun=%d dab=%d painting=%d identical=%d",
                begun ? 1 : 0, dabbed ? 1 : 0, painting ? 1 : 0, identical ? 1 : 0);
        if (!begun || !dabbed || !painting || !identical) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(543, "mid-stroke present did not use the crop");
        }
        frame.closeDocument(doc, false);
    }

    return 0;
}

} // namespace pictura
