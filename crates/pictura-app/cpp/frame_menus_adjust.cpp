// Image > Adjustments and Image > Auto Tone / Auto Contrast / Auto Color:
// destructive adjustments of the active pixel layer, within the selection.

#include "frame_includes.h"

#include "api/layers_api.h"

#include "adjustment_dialog.h"
#include "api/document_api.h"
#include "hdr_toning_dialog.h"
#include "layer_adjustments.h"
#include "match_color_dialog.h"
#include "replace_color_dialog.h"

#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust.cxxqt.h"

namespace pictura {

namespace {

// The dialog adjustments: engine kind and menu leaf.
struct Entry {
    const char* kind;
    const char* leaf;
};

constexpr Entry kDialogs[] = {
    {"brightness-contrast", "Brightness/Contrast"},
    {"levels", "Levels"},
    {"curves", "Curves"},
    {"exposure", "Exposure"},
    {"vibrance", "Vibrance"},
    {"hue-saturation", "Hue/Saturation"},
    {"color-balance", "Color Balance"},
    {"black-white", "Black & White"},
    {"photo-filter", "Photo Filter"},
    {"channel-mixer", "Channel Mixer"},
    {"posterize", "Posterize"},
    {"threshold", "Threshold"},
    {"gradient-map", "Gradient Map"},
    {"selective-color", "Selective Color"},
    {"shadows-highlights", "Shadows/Highlights"},
    {"color-lookup", "Color Lookup"},
};

// The commands that apply at once.
constexpr Entry kDirect[] = {
    {"invert", "Invert"},           {"desaturate", "Desaturate"},
    {"equalize", "Equalize"},       {"auto-tone", "Auto Tone"},
    {"auto-contrast", "Auto Contrast"}, {"auto-color", "Auto Color"},
};

} // namespace

void PicturaMainWindow::wireImageAdjustments()
{
    const auto ready = [this]() {
        PictureView* view = activeView();
        return view && filter_target_ready(*view);
    };
    for (const Entry& entry : kDialogs) {
        const QString leaf = QString::fromUtf8(entry.leaf);
        const QString id =
            commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), leaf});
        const QString kind = QString::fromLatin1(entry.kind);
        registry_->setImplemented(id, true);
        // CS6 marks every dialog-opening entry with an ellipsis.
        registry_->setLabelProvider(id, [leaf]() { return leaf + QStringLiteral("…"); });
        registry_->setHandler(id, [this, kind]() {
            PictureView* view = activeView();
            if (view && filter_target_ready(*view)
                && runAdjustmentDialog(this, view, kind, tools_->foreground(),
                                       tools_->background(), imageView())) {
                refresh();
            } else if (view) {
                reportFilterRefusal(view);
            }
        });
        registry_->setEnabledProvider(id, ready);
    }
    for (const Entry& entry : kDirect) {
        const QString leaf = QString::fromUtf8(entry.leaf);
        const QString kind = QString::fromLatin1(entry.kind);
        // The Auto commands live at the top of the Image menu, not under
        // Adjustments.
        const QString id = kind.startsWith(QLatin1String("auto-"))
                               ? commandIdForPath({QStringLiteral("Image"), leaf})
                               : commandIdForPath({QStringLiteral("Image"),
                                                   QStringLiteral("Adjustments"), leaf});
        registry_->setImplemented(id, true);
        registry_->setHandler(id, [this, kind]() {
            PictureView* view = activeView();
            if (view && image_adjust_direct(*view, kind)) {
                refresh();
            } else if (view) {
                reportFilterRefusal(view);
            }
        });
        registry_->setEnabledProvider(id, ready);
    }

    // HDR Toning is a neighborhood operator, not a pointwise adjustment, so it
    // runs through the filter preview/apply path via its own dialog.
    {
        const QString leaf = QStringLiteral("HDR Toning");
        const QString id =
            commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), leaf});
        registry_->setImplemented(id, true);
        registry_->setLabelProvider(id, [leaf]() { return leaf + QStringLiteral("…"); });
        registry_->setHandler(id, [this]() {
            PictureView* view = activeView();
            ImageView* canvas = imageView();
            const QRect visible = canvas ? canvas->visibleDocumentRect().toAlignedRect() : QRect();
            if (view && filter_target_ready(*view)
                && HdrToningDialog::get(this, view, visible)) {
                refresh();
            } else if (view) {
                reportFilterRefusal(view);
            }
        });
        registry_->setEnabledProvider(id, ready);
    }

    // Replace Color is dialog-only and non-modal, so its eyedropper can reach
    // the canvas; OK commits one "Replace Color" state.
    {
        const QString leaf = QStringLiteral("Replace Color");
        const QString id =
            commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), leaf});
        registry_->setImplemented(id, true);
        registry_->setLabelProvider(id, [leaf]() { return leaf + QStringLiteral("…"); });
        registry_->setHandler(id, [this]() {
            PictureView* view = activeView();
            if (!view || !filter_target_ready(*view)) {
                if (view) {
                    reportFilterRefusal(view);
                }
                return;
            }
            if (replaceColorDialog_) {
                replaceColorDialog_->raise();
                replaceColorDialog_->activateWindow();
                return;
            }
            auto* dialog = new ReplaceColorDialog(view, tools_, imageView(), this);
            dialog->setAttribute(Qt::WA_DeleteOnClose);
            replaceColorDialog_ = dialog;
            connect(dialog, &QDialog::finished, this, [this](int result) {
                if (result == QDialog::Accepted) {
                    refresh();
                }
            });
            dialog->show();
        });
        registry_->setEnabledProvider(id, ready);
    }

    // Match Color matches the active layer's statistics to another open
    // document, one of its layers, or the loaded statistics; it is RGB-only
    // (CS6), so Grayscale documents disable the row. Modal, like the other
    // adjustment dialogs.
    {
        const QString leaf = QStringLiteral("Match Color");
        const QString id =
            commandIdForPath({QStringLiteral("Image"), QStringLiteral("Adjustments"), leaf});
        registry_->setImplemented(id, true);
        registry_->setLabelProvider(id, [leaf]() { return leaf + QStringLiteral("…"); });
        registry_->setHandler(id, [this]() {
            PictureView* view = activeView();
            if (!view || !filter_target_ready(*view)) {
                if (view) {
                    reportFilterRefusal(view);
                }
                return;
            }
            QList<MatchColorDialog::Source> sources;
            for (int i = 0; i < documentCount(); ++i) {
                if (viewAt(i) && viewAt(i) != view) {
                    sources.append({documentName(i), viewAt(i)});
                }
            }
            MatchColorDialog dialog(view, imageView(), sources, this);
            if (runDialog(dialog, this) == QDialog::Accepted) {
                refresh();
            }
        });
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && filter_target_ready(*view)
                && document_api::documentMode(*view) == QStringLiteral("rgb");
        });
    }
}

// Layer > New Adjustment Layer: the same add_adjustment bridge the Adjustments
// panel uses, one entry per CS6 kind over the active document.
void PicturaMainWindow::wireLayerAdjustments()
{
    const auto ready = [this]() {
        return activeView() && activeView()->has_document();
    };
    for (const LayerAdjustment& entry : kLayerAdjustments) {
        const QString leaf = QString::fromUtf8(entry.leaf);
        const QString id = commandIdForPath(
            {QStringLiteral("Layer"), QStringLiteral("New Adjustment Layer"), leaf});
        const QString kind = QString::fromLatin1(entry.kind);
        registry_->setImplemented(id, true);
        if (entry.ellipsis) {
            registry_->setLabelProvider(id, [leaf]() { return leaf + QStringLiteral("…"); });
        }
        registry_->setHandler(id, [this, kind]() {
            PictureView* view = activeView();
            if (view && view->add_adjustment(kind)) {
                refresh();
            }
        });
        registry_->setEnabledProvider(id, ready);
    }
}

// Layer > Layer Content Options opens the active fill/adjustment layer's
// Properties page. It is enabled only for a layer carrying an adjustment block,
// which covers both adjustment and fill layers.
void PicturaMainWindow::wireLayerContentOptions()
{
    const auto currentAdjustmentPath = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layersPanel_ ? layersPanel_->currentPath() : QString();
        if (!view || path.isEmpty()) {
            return QString();
        }
        const auto row = pictura::layers_api::rowForPath(*view, path);
        return row.hasAdjustment ? path : QString();
    };
    const QString id = QString::fromLatin1(command_ids::LayerContentOptions);
    registry_->setImplemented(id, true);
    registry_->setHandler(id, [this, currentAdjustmentPath]() {
        if (currentAdjustmentPath().isEmpty()) {
            return;
        }
        if (panelColumn_) {
            panelColumn_->showPanel(QStringLiteral("propertiesPanel"), true);
        }
        if (propertiesPanel_) {
            propertiesPanel_->refresh();
        }
    });
    registry_->setEnabledProvider(
        id, [currentAdjustmentPath]() { return !currentAdjustmentPath().isEmpty(); });
}

} // namespace pictura
