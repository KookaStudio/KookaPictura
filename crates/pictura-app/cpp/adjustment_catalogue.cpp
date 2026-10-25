#include "adjustment_catalogue.h"

#include "pictura_app/src/cxxqt_object/engine_tables/adjustment_catalogue.cxxqt.h"

namespace pictura {

const QVector<AdjustmentKind>& adjustmentCatalogue()
{
    static const QVector<AdjustmentKind> kinds = [] {
        QVector<AdjustmentKind> out;
        for (const AdjustmentRow& row : adjustment_catalogue()) {
            out.append({row.kind, row.title, row.image, row.layer, row.layer_ellipsis,
                        row.panel_icon, row.panel_menu, row.fill});
        }
        return out;
    }();
    return kinds;
}

} // namespace pictura
