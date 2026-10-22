// Layer > Layer Style: the Layer Style dialog on each effect, Copy / Paste /
// Clear Layer Style, Scale Effects, and Hide / Show All Effects.
// ponytail: Global Light and Create Layers stay disabled stubs.

#include "frame_includes.h"

#include "layer_style_dialog.h"

#include <QtWidgets/QInputDialog>

#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"

namespace pictura {

namespace {

// Menu leaf and the dialog page it opens; "" is Blending Options.
const std::pair<const char*, const char*> kDialogLeaves[] = {
    {"Blending Options…", ""},          {"Drop Shadow…", "dropShadow"},
    {"Inner Shadow…", "innerShadow"},   {"Outer Glow…", "outerGlow"},
    {"Inner Glow…", "innerGlow"},       {"Bevel & Emboss…", "bevel"},
    {"Satin…", "satin"},                {"Color Overlay…", "colorOverlay"},
    {"Gradient Overlay…", "gradientOverlay"}, {"Pattern Overlay…", "patternOverlay"},
    {"Stroke…", "stroke"},
};

QString styleCommand(const char* leaf)
{
    return commandIdForPath(
        {QStringLiteral("Layer"), QStringLiteral("Layer Style"), QString::fromUtf8(leaf)});
}

} // namespace

void PicturaMainWindow::wireLayerStyleMenu()
{
    const auto current = [this]() {
        return layerAnchor();
    };
    const auto selected = [this]() {
        return selectedLayerPaths();
    };
    const auto wire = [this](const QString& id, std::function<void()> handler,
                             std::function<bool()> enabled) {
        registry_->setImplemented(id, true);
        registry_->setHandler(id, std::move(handler));
        registry_->setEnabledProvider(id, std::move(enabled));
    };
    const auto canEdit = [this, current]() {
        PictureView* view = activeView();
        return view && view->has_document() && layer_style_can_edit(*view, current());
    };

    for (const auto& [leaf, effect] : kDialogLeaves) {
        const QString key = QString::fromLatin1(effect);
        wire(
            styleCommand(leaf),
            [this, current, key]() {
                PictureView* view = activeView();
                const QString path = current();
                if (view && layer_style_can_edit(*view, path)) {
                    LayerStyleDialog dialog(view, path, key, this);
                    runDialog(dialog, this);
                    refresh();
                }
            },
            canEdit);
    }

    wire(
        styleCommand("Copy Layer Style"),
        [this, current]() {
            if (PictureView* view = activeView()) {
                layer_style_copy(*view, current());
                refresh();
            }
        },
        [this, current, canEdit]() {
            return canEdit() && layer_style_has(*activeView(), current());
        });
    wire(
        styleCommand("Paste Layer Style"),
        [this, selected]() {
            PictureView* view = activeView();
            if (view) {
                layer_style_paste(*view, selected());
            }
        },
        [this, selected]() {
            PictureView* view = activeView();
            return view && view->has_document() && layer_style_can_paste()
                && !selected().isEmpty();
        });
    const auto anyStyled = [this, selected]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return false;
        }
        for (const QString& path : selected()) {
            if (layer_style_has(*view, path) && layer_style_can_edit(*view, path)) {
                return true;
            }
        }
        return false;
    };
    wire(
        styleCommand("Clear Layer Style"),
        [this, selected]() {
            PictureView* view = activeView();
            if (view) {
                layer_style_clear(*view, selected());
            }
        },
        anyStyled);
    wire(
        styleCommand("Scale Effects"),
        [this, selected]() {
            PictureView* view = activeView();
            bool ok = false;
            const int percent = QInputDialog::getInt(this, tr("Scale Layer Effects"),
                                                     tr("Scale (%):"), 100, 1, 1000, 1, &ok);
            if (view && ok) {
                layer_style_scale(*view, selected(), percent);
            }
        },
        anyStyled);
    for (const bool visible : {false, true}) {
        wire(
            styleCommand(visible ? "Show All Effects" : "Hide All Effects"),
            [this, visible]() {
                PictureView* view = activeView();
                if (view) {
                    layer_style_set_all_visible(*view, visible);
                }
            },
            [this, visible]() {
                // Show needs something hidden, Hide something shown.
                PictureView* view = activeView();
                return view && view->has_document() && layer_style_any_visible(*view, !visible);
            });
    }
}

} // namespace pictura
