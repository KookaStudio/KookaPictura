#include "fill_layer_editor.h"

#include "color_picker_dialog.h"
#include "gradient_fill_dialog.h"
#include "layer_new_dialog.h"
#include "pattern_fill_dialog.h"

#include "pictura_app/src/cxxqt_object/impl_layers/fill_content.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"

#include <QtCore/QStringList>

namespace pictura {

namespace {

QString solidPayload(const QColor& color)
{
    return QString::number(color.rgb() & 0xffffff, 16).rightJustified(6, QLatin1Char('0'));
}

const char* kDefaultGradient = "linear|0|100|0|0:000000,4096:ffffff";

} // namespace

bool openFillLayerEditor(PictureView& view, const QString& path, QWidget* parent)
{
    const QString kind = layer_fill_kind(view, path);
    if (kind == QLatin1String("solid-color")) {
        const QColor color =
            ColorPickerDialog::getColor(QColor::fromRgba(layer_fill_color(view, path)), parent,
                                        QStringLiteral("Solid Color Fill"));
        return color.isValid() && set_layer_fill_color(view, path, color.rgba());
    }
    if (kind == QLatin1String("gradient")) {
        QString payload;
        if (!GradientFillDialog::getFill(layer_fill_gradient(view, path), parent, &payload)) {
            return false;
        }
        return set_layer_fill_gradient(view, path, payload);
    }
    if (kind == QLatin1String("pattern")) {
        QString payload;
        if (!PatternFillDialog::getFill(view, layer_fill_pattern(view, path), parent, &payload)) {
            return false;
        }
        return set_layer_fill_pattern(view, path, payload);
    }
    return false;
}

bool createFillLayerFromDialog(PictureView& view, const QString& kind, const QColor& solid,
                               QWidget* parent, bool showNewLayerDialog)
{
    if (kind != QLatin1String("solid-color") && kind != QLatin1String("gradient")
        && kind != QLatin1String("pattern")) {
        return false;
    }
    if (kind == QLatin1String("pattern") && !layer_has_patterns(view)) {
        return false;
    }
    const QStringList patterns = document_pattern_ids(view);

    QString payload;
    QString name;
    int color = 0;
    QString blend = QStringLiteral("norm");
    int opacity = 255;
    if (showNewLayerDialog) {
        LayerNewSpec spec;
        const QString defaultName = kind == QLatin1String("solid-color") ? QStringLiteral("Color Fill 1")
                                    : kind == QLatin1String("gradient") ? QStringLiteral("Gradient Fill 1")
                                                                        : QStringLiteral("Pattern Fill 1");
        if (!LayerNewDialog::get(false, parent, &spec, defaultName)) {
            return false;
        }
        name = spec.name == defaultName ? QString() : spec.name;
        color = spec.color;
        blend = spec.blend;
        opacity = spec.opacity;
        if (kind == QLatin1String("solid-color")) {
            const QColor chosen =
                ColorPickerDialog::getColor(solid, parent, QStringLiteral("Solid Color Fill"));
            if (!chosen.isValid()) {
                return false;
            }
            payload = solidPayload(chosen);
        } else if (kind == QLatin1String("gradient")) {
            if (!GradientFillDialog::getFill(QString::fromLatin1(kDefaultGradient), parent,
                                             &payload)) {
                return false;
            }
        } else {
            if (patterns.isEmpty()
                || !PatternFillDialog::getFill(view, patterns.first() + QStringLiteral("|100|1|0|0"),
                                               parent, &payload)) {
                return false;
            }
        }
        return !add_fill_layer(view, kind, payload, layer_selection_anchor(view), name, color,
                              blend, opacity, spec.clipping)
                    .isEmpty();
    }

    // Quick-add: create with the default content, then open the editor on it.
    if (kind == QLatin1String("solid-color")) {
        payload = solidPayload(solid);
    } else if (kind == QLatin1String("gradient")) {
        payload = QString::fromLatin1(kDefaultGradient);
    } else {
        if (patterns.isEmpty()) {
            return false;
        }
        payload = patterns.first() + QStringLiteral("|100|1|0|0");
    }
    const QString created = add_fill_layer(view, kind, payload, layer_selection_anchor(view),
                                          QString(), 0, QStringLiteral("norm"), 255, false);
    if (created.isEmpty()) {
        return false;
    }
    openFillLayerEditor(view, created, parent);
    return true;
}

} // namespace pictura
