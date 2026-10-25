#pragma once

#include <QtCore/QString>
#include <QtCore/QVector>

namespace pictura {

// One adjustment or fill kind as the engine catalogue lists it
// (`pictura_render::ADJUSTMENT_CATALOGUE`).
struct AdjustmentKind {
    QString kind;
    QString title;
    // "dialog" / "direct" under Image > Adjustments, "root" at the Image menu
    // root (the Auto commands), or empty.
    QString image;
    // Offered by Layer > New Adjustment Layer / New Fill Layer.
    bool layer = false;
    bool layerEllipsis = false;
    // The Adjustments panel tile icon id and flyout row text; empty without.
    QString panelIcon;
    QString panelMenu;
    bool fill = false;
};

// Every kind in CS6 order, read once from the engine. Image > Adjustments,
// New Fill/Adjustment Layer, the Layers panel's New Fill/Adjustment menu and
// the Adjustments panel all build from it.
const QVector<AdjustmentKind>& adjustmentCatalogue();

} // namespace pictura
