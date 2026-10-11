#include "frame_includes.h"

#include <QtCore/QDebug>
#include <QtWidgets/QStatusBar>

#include "filter_commands.h"
#include "filter_commit.h"
#include "filter_gallery_dialog.h"
#include "filter_preview_dialog.h"
#include "lens_flare_dialog.h"
#include "lighting_effects_dialog.h"

#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/jobs.cxxqt.h"

namespace pictura {

namespace {

// Preview against the section and scale the user is currently looking at.
FilterPreviewView previewViewFor(ImageView* canvas)
{
    FilterPreviewView view;
    if (canvas) {
        view.visible = canvas->visibleDocumentRect();
        view.canvasZoom = canvas->zoom();
        view.canvas = canvas;
        view.devicePixelRatio = canvas->devicePixelRatioF();
    }
    return view;
}

// Lens Flare's parameters are one struct and Lighting Effects' a list of lights,
// each edited on its own dialog; every other filter edits independent slots on
// the shared one.
bool runFilterDialog(PictureView* view, const FilterCommandSpec& spec,
                     const FilterPreviewView& previewView, const QList<double>& initial,
                     QList<double>* out, QWidget* parent)
{
    if (spec.kind == QStringLiteral("lens-flare")) {
        return LensFlareDialog::get(view, spec, initial, out, parent);
    }
    if (spec.kind == QStringLiteral("lighting-effects")) {
        return LightingEffectsDialog::get(view, spec, initial, out, parent);
    }
    return FilterPreviewDialog::get(view, spec, previewView, initial, out, parent);
}

// Commit `kind` on a worker with a busy indicator, recorded under its menu
// name. A zoomed-out dialog leaves its canvas preview up until the commit lands.
bool commitFilter(QWidget* parent, PictureView* view, ImageView* canvas, const QString& kind,
                  const QList<double>& values)
{
    const FilterCommandSpec* spec = filterCommandForKind(kind);
    const QString name = spec ? spec->label : kind;
    const bool applied = runFilterCommit(parent, view, name, [&] {
        return filter_commit_start(*view, kind, values, filterHistoryLabel(name));
    });
    if (canvas) {
        canvas->clearFilterPreview();
    }
    return applied;
}

} // namespace

void PicturaMainWindow::reportFilterRefusal(PictureView* view)
{
    const QString reason = view ? filter_last_error(*view) : QString();
    const QString message =
        reason.isEmpty() ? tr("The filter could not be applied.") : reason;
    qWarning().noquote() << "filter refused:" << message;
    statusBar()->showMessage(message, 6000);
}

void PicturaMainWindow::applyFilterCommand(const FilterCommandSpec& spec)
{
    PictureView* view = activeView();
    if (!view || !filter_target_ready(*view)) {
        return;
    }
    if (spec.params.isEmpty()) {
        // Nothing to edit: apply the engine defaults directly.
        if (commitFilter(this, view, imageView(), spec.kind, QList<double>())) {
            refresh();
        } else {
            reportFilterRefusal(view);
        }
        return;
    }
    QList<double> values;
    // The dialog previews on the canvas as controls change; OK commits one
    // state on a worker, Cancel discards the preview bit-identically.
    if (runFilterDialog(view, spec, previewViewFor(imageView()), QList<double>(), &values, this)) {
        if (commitFilter(this, view, imageView(), spec.kind, values)) {
            refresh();
        } else {
            reportFilterRefusal(view);
        }
    }
}

void PicturaMainWindow::wireFilterMenu()
{
    for (const FilterCommandSpec& spec : filterCommands()) {
        if (!filter_kind_supported(spec.kind)) {
            continue; // a row with no Rust mapping stays a disabled stub
        }
        const QString id = commandIdForPath(spec.path);
        registry_->setImplemented(id, true);
        const FilterCommandSpec captured = spec;
        registry_->setHandler(id, [this, captured]() { applyFilterCommand(captured); });
        // CS6 marks every dialog-opening entry with an ellipsis; a parameterless
        // entry applies straight away and stays bare.
        registry_->setLabelProvider(id, [captured]() {
            return captured.params.isEmpty() ? captured.label
                                             : captured.label + QStringLiteral("…");
        });
        registry_->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && filter_target_ready(*view);
        });
    }

    // Filter > Filter Gallery: the stacked gallery dialog; OK commits the
    // visible effects as one history state.
    const QString gallery =
        commandIdForPath({QStringLiteral("Filter"), QStringLiteral("Filter Gallery…")});
    registry_->setImplemented(gallery, true);
    registry_->setHandler(gallery, [this]() {
        PictureView* view = activeView();
        if (!view || !filter_target_ready(*view)) {
            return;
        }
        FilterGalleryDialog dialog(view, this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return;
        }
        switch (dialog.commit()) {
        case FilterGalleryDialog::CommitResult::Applied:
            refresh();
            break;
        case FilterGalleryDialog::CommitResult::Refused:
            reportFilterRefusal(view);
            break;
        case FilterGalleryDialog::CommitResult::NothingVisible:
            break;
        }
    });
    registry_->setEnabledProvider(gallery, [this]() {
        PictureView* view = activeView();
        return view && filter_target_ready(*view);
    });

    // Filter > Last Filter: re-apply the last committed filter with no dialog.
    registry_->setHandler(command_ids::FilterLastFilter, [this]() {
        PictureView* view = activeView();
        if (!view || !filter_has_last(*view)) {
            return;
        }
        if (commitFilter(this, view, imageView(), filter_last_kind(*view), filter_last_params(*view))) {
            refresh();
        } else {
            reportFilterRefusal(view);
        }
    });
    registry_->setEnabledProvider(command_ids::FilterLastFilter, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && filter_has_last(*view) && filter_target_ready(*view);
    });
    registry_->setLabelProvider(command_ids::FilterLastFilter, [this]() {
        PictureView* view = activeView();
        const FilterCommandSpec* spec =
            view ? filterCommandForKind(filter_last_kind(*view)) : nullptr;
        return spec ? spec->label : QStringLiteral("Last Filter");
    });

    // Filter > Last Filter Settings: reopen the last filter's dialog prefilled.
    registry_->setHandler(command_ids::FilterLastFilterSettings, [this]() {
        PictureView* view = activeView();
        if (!view || !filter_has_last(*view)) {
            return;
        }
        const FilterCommandSpec* spec = filterCommandForKind(filter_last_kind(*view));
        if (!spec || spec->params.isEmpty()) {
            return;
        }
        QList<double> values;
        if (runFilterDialog(view, *spec, previewViewFor(imageView()), filter_last_params(*view),
                            &values, this)) {
            if (commitFilter(this, view, imageView(), spec->kind, values)) {
                refresh();
            } else {
                reportFilterRefusal(view);
            }
        }
    });
    registry_->setEnabledProvider(command_ids::FilterLastFilterSettings, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document() || !filter_has_last(*view) || !filter_target_ready(*view)) {
            return false;
        }
        const FilterCommandSpec* spec = filterCommandForKind(filter_last_kind(*view));
        return spec && !spec->params.isEmpty();
    });
}

} // namespace pictura
