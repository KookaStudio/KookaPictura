#include "selftest_warp_preset.h"
#include "selftest_report.h"

#include "api/transform_api.h"
#include "frame.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

int pictura::runWarpPresetChecks(pictura::PicturaMainWindow& frame)
{
    const bool created = frame.newDocument(QStringLiteral("WarpPresetCtl"), 16, 16,
                                           QStringLiteral("rgb"), 8,
                                           QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    if (!created || !view) {
        return pictura::selfTest().fail(527, "warp preset fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const QString path = view->add_solid_fill(0xff2244aau);
    const bool raster = view->rasterize_fill_content(path);
    const bool can = view->layer_can_free_transform(path);
    const QString before = transform_api::layerRect(*view, path).toString();

    const int base = view->history_count();
    const bool applied =
        view->apply_warp_preset(path, QStringLiteral("warpArc"), 50.0, 0.0, 0.0, false);
    const bool oneState = view->history_count() == base + 1
        && view->history_label(base) == QStringLiteral("Warp");
    const bool changed = !before.isEmpty() && transform_api::layerRect(*view, path).toString() != before;

    const int noneBase = view->history_count();
    const bool noneRefused =
        !view->apply_warp_preset(path, QStringLiteral("warpNone"), 50.0, 0.0, 0.0, false)
        && view->history_count() == noneBase;
    const int unknownBase = view->history_count();
    const bool unknownRefused =
        !view->apply_warp_preset(path, QStringLiteral("warpNope"), 50.0, 0.0, 0.0, false)
        && view->history_count() == unknownBase;

    ST_BEGIN("lpr_warp_preset");
    ST_PASS("lpr_warp_preset raster=%d can=%d apply=%d states=%d rect=%s->%s none=%d unknown=%d",
            raster ? 1 : 0, can ? 1 : 0, applied ? 1 : 0, view->history_count() - base,
            qPrintable(before), qPrintable(transform_api::layerRect(*view, path).toString()), noneRefused ? 1 : 0,
            unknownRefused ? 1 : 0);
    if (!raster || !can || !applied || !oneState || !changed || !noneRefused || !unknownRefused) {
        return pictura::selfTest().fail(527, "warp preset");
    }
    frame.closeDocument(doc, false);
    return 0;
}
