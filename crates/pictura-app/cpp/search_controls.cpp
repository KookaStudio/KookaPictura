// The curated panel and options-bar controls Help > Search indexes. Grow it by
// adding a row: the container's and the control's object names, the tool whose
// options page holds the control (or null), the breadcrumb, and extra words to
// match. tst_search_bar::curatedControlsExist keeps every row live.

#include "search_index.h"

namespace pictura {

const QList<SearchControl>& searchControls()
{
    static const QList<SearchControl> controls = {
        {"layersPanel", "layersBlendCombo", nullptr, "Layers panel > Blend Mode",
         "blending layer"},
        {"layersPanel", "layersOpacityField", nullptr, "Layers panel > Opacity",
         "layer transparency"},
        {"layersPanel", "layersFillField", nullptr, "Layers panel > Fill", "fill opacity layer"},
        {"layersPanel", "layersLockTransparency", nullptr,
         "Layers panel > Lock Transparent Pixels", "lock alpha layer"},
        {"layersPanel", "layersLockPixels", nullptr, "Layers panel > Lock Image Pixels",
         "lock paint layer"},
        {"layersPanel", "layersLockPosition", nullptr, "Layers panel > Lock Position",
         "lock move layer"},
        {"layersPanel", "layersLockAll", nullptr, "Layers panel > Lock All", "lock layer"},
        {"brushPanel", "brushPanelSize", nullptr, "Brush panel > Size", "brush diameter tip"},
        {"optionsBar", "optionsBrushMode", "brush", "Options bar > Brush > Mode",
         "blend blending paint"},
        {"optionsBar", "optionsBrushOpacity", "brush", "Options bar > Brush > Opacity",
         "paint transparency"},
        {"optionsBar", "optionsBrushFlow", "brush", "Options bar > Brush > Flow",
         "paint airbrush"},
        {"characterPanel", "characterFamily", nullptr, "Character panel > Font Family",
         "font typeface type text"},
        {"characterPanel", "characterSize", nullptr, "Character panel > Font Size",
         "font type text points"},
    };
    return controls;
}

} // namespace pictura
