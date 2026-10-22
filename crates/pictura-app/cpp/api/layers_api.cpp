#include "api/layers_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"

#include <rust/cxx.h>

namespace pictura::layers_api {

namespace {

QStringList splitEffects(const QString& joined)
{
    return joined.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
}

LayerRow convert(const ::pictura::LayerRow& row)
{
    LayerRow out;
    out.path = row.path;
    out.depth = row.depth;
    out.name = row.name;
    out.kind = row.kind;
    out.visible = row.visible;
    out.blend = row.blend;
    out.opacity = row.opacity;
    out.fill = row.fill;
    out.lockBits = row.lock;
    out.color = row.color;
    out.clipping = row.clipping;
    out.hasMask = row.has_mask;
    out.maskLinked = row.mask_linked;
    out.maskDisabled = row.mask_disabled;
    out.hasVectorMask = row.has_vector_mask;
    out.vectorMaskLinked = row.vector_mask_linked;
    out.vectorMaskDisabled = row.vector_mask_disabled;
    out.hasAdjustment = row.has_adjustment;
    out.expandable = row.expandable;
    out.childCount = row.child_count;
    out.linked = row.linked;
    out.placed = row.placed;
    out.shape = row.shape;
    out.warped = row.warped;
    out.smartObject = row.smart_object;
    out.hasStyle = row.has_style;
    out.styleEffects = splitEffects(row.style_effects);
    out.hasBlendIf = row.has_blend_if;
    out.thumbnail = row.thumbnail;
    out.maskThumbnail = row.mask_thumbnail;
    out.vectorMaskThumbnail = row.vector_mask_thumbnail;
    return out;
}

} // namespace

Snapshot snapshot(PictureView& view, int thumbSize, bool entireDocument)
{
    Snapshot snap;
    snap.documentWidth = view.document_width();
    snap.documentHeight = view.document_height();
    snap.thumbSize = thumbSize;
    snap.maskTargetPath = mask_edit_target_get(view);
    const ::rust::Vec<::pictura::LayerRow> rows = view.layers_snapshot(thumbSize, entireDocument);
    snap.rows.reserve(static_cast<int>(rows.size()));
    for (const ::pictura::LayerRow& row : rows) {
        snap.rows.append(convert(row));
    }
    return snap;
}

} // namespace pictura::layers_api
