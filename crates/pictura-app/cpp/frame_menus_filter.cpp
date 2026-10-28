#include "frame_includes.h"

#include <QtCore/QDebug>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QStatusBar>

#include "awa_dialog.h"
#include "blur_gallery_dialog.h"
#include "filter_commands.h"
#include "filter_commit.h"
#include "filter_gallery_dialog.h"
#include "filter_preview_dialog.h"
#include "lens_correction_dialog.h"
#include "lens_flare_dialog.h"
#include "lighting_effects_dialog.h"
#include "liquify_dialog.h"
#include "picture_frame_dialog.h"
#include "reduce_noise_dialog.h"
#include "session.h"
#include "vp_dialog.h"

#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"

namespace pictura {

namespace {

// Preview against the section and scale the user is currently looking at: the
// first canvas is the active window's; the document's other windows mirror it.
FilterPreviewView previewViewFor(const QList<ImageView*>& canvases)
{
    FilterPreviewView view;
    if (!canvases.isEmpty()) {
        ImageView* canvas = canvases.first();
        view.visible = canvas->visibleDocumentRect();
        view.canvasZoom = canvas->zoom();
        view.canvas = canvas;
        view.devicePixelRatio = canvas->devicePixelRatioF();
        view.mirrors = canvases.mid(1);
    }
    return view;
}

// CS6's Displace asks for its map file after OK. The loaded map's id fills
// the hidden last slot; Cancel or an unusable file abandons the filter.
bool chooseDisplacementMap(QList<double>* values, QWidget* parent)
{
    const QString path =
        getOpenFileName(parent, QObject::tr("Choose a displacement map."), QString(),
                        {QObject::tr("Photoshop (*.psd *.PSD *.pdd *.PDD)")});
    if (path.isEmpty() || values->isEmpty()) {
        return false;
    }
    const int id = filter_displace_map_load(path);
    if (id == 0) {
        QMessageBox::warning(parent, QObject::tr("Displace"),
                             QObject::tr("Could not use \"%1\" as a displacement map. Choose a "
                                         "Photoshop file that is not in Bitmap mode.")
                                 .arg(QFileInfo(path).fileName()));
        return false;
    }
    values->last() = id;
    return true;
}

// Lens Flare's parameters are one struct, Lighting Effects' a list of lights,
// Liquify's a mesh, Adaptive Wide Angle's a constraint set, Vanishing
// Point's a set of planes and Lens Correction's a workspace with its own
// tools, each edited on its own dialog; every other filter edits independent
// slots on the shared one.
// `*kind` is the kind to commit: the spec's, except that the Blur Gallery
// commits whichever of its effects the user switched on.
bool runFilterDialog(PictureView* view, const FilterCommandSpec& spec,
                     const FilterPreviewView& previewView, const QList<double>& initial,
                     QList<double>* out, QString* kind, QWidget* parent)
{
    *kind = spec.kind;
    switch (spec.dialog) {
    case FilterDialogKind::LensFlare:
        return LensFlareDialog::get(view, spec, initial, out, parent);
    case FilterDialogKind::LightingEffects:
        return LightingEffectsDialog::get(view, spec, initial, out, parent);
    case FilterDialogKind::PictureFrame:
        return PictureFrameDialog::get(view, spec, initial, out, parent);
    case FilterDialogKind::ReduceNoise:
        return ReduceNoiseDialog::get(view, spec, initial, out, parent);
    case FilterDialogKind::BlurGallery:
        return BlurGalleryDialog::get(view, spec, initial, out, kind, parent);
    case FilterDialogKind::Liquify: {
        const auto* frame = qobject_cast<PicturaMainWindow*>(parent);
        const QString name = frame ? frame->activeDocumentName() : QString();
        return LiquifyDialog::get(view, name, initial, out, parent);
    }
    case FilterDialogKind::AdaptiveWideAngle: {
        const auto* frame = qobject_cast<PicturaMainWindow*>(parent);
        const QString name = frame ? frame->activeDocumentName() : QString();
        return AdaptiveWideAngleDialog::get(view, name, initial, out, parent);
    }
    case FilterDialogKind::VanishingPoint: {
        const auto* frame = qobject_cast<PicturaMainWindow*>(parent);
        const QString name = frame ? frame->activeDocumentName() : QString();
        return VanishingPointDialog::get(view, name, initial, out, parent);
    }
    case FilterDialogKind::LensCorrection:
        return LensCorrectionDialog::get(view, initial, out, parent);
    case FilterDialogKind::Shared:
        break;
    }
    if (!FilterPreviewDialog::get(view, spec, previewView, initial, out, parent)) {
        return false;
    }
    return !spec.mapAfterOk || chooseDisplacementMap(out, parent);
}

// Photoshop refuses a blur with nothing under it — an empty layer, or a
// selection over only transparent pixels — with an error naming the command.
// The Blur Gallery words it as an empty bounding rectangle.
bool refuseEmptyBlur(PictureView* view, const FilterCommandSpec& spec, QWidget* parent)
{
    const QString family = spec.path.value(1);
    const bool gallery = family == QStringLiteral("Blur Gallery");
    if ((!gallery && family != QStringLiteral("Blur")) || !filter_target_empty(*view)) {
        return false;
    }
    const QString reason = gallery ? QObject::tr("the initial bounding rectangle is empty")
                                   : QObject::tr("the selected area is empty");
    QMessageBox box(QMessageBox::Critical, QStringLiteral("Kooka Pictura"),
                    QObject::tr("Could not complete the %1 command because %2.")
                        .arg(spec.label, reason),
                    QMessageBox::Ok, parent);
    box.setObjectName(QStringLiteral("emptyFilterTargetError"));
    box.exec();
    return true;
}

// Commit `kind` on a worker with a busy indicator, recorded under its menu
// name. A zoomed-out dialog leaves its canvas preview up on `canvases` until
// the commit lands.
bool commitFilter(QWidget* parent, PictureView* view, const QList<ImageView*>& canvases,
                  const QString& kind, const QList<double>& values)
{
    const FilterCommandSpec* spec = filterCommandForKind(kind);
    const QString name = spec ? spec->label : kind;
    const bool applied = runFilterCommit(parent, view, name, [&] {
        return filter_commit_start(*view, kind, values, filterHistoryLabel(name));
    });
    for (ImageView* canvas : canvases) {
        canvas->clearFilterPreview();
    }
    return applied;
}

// CS6's notice before re-editing a Smart Filter with others stacked above
// it, with the "Don't show again" choice persisted in the session store.
void noteStackedSmartFilterEdit(QWidget* parent)
{
    SessionState session = loadSession();
    if (!session.showSmartFilterEditMessage) {
        return;
    }
    QMessageBox box(QMessageBox::Information, QStringLiteral("Kooka Pictura"),
                    QObject::tr("Smart filters stacked on top of this filter will not preview "
                                "while this filter is being edited. They will be applied after "
                                "committing the filter parameters dialog."),
                    QMessageBox::Ok, parent);
    box.setObjectName(QStringLiteral("smartFilterEditNotice"));
    auto* dontShow = new QCheckBox(QObject::tr("Don't show again"), &box);
    dontShow->setObjectName(QStringLiteral("smartFilterEditDontShowAgain"));
    box.setCheckBox(dontShow);
    box.exec();
    if (dontShow->isChecked()) {
        session = loadSession();
        session.showSmartFilterEditMessage = false;
        saveSession(session);
    }
}

CommandParams filterParams(const QString& kind, const QList<double>& values)
{
    QJsonArray params;
    for (const double value : values) {
        params.append(value);
    }
    CommandParams command;
    command.values.insert(QStringLiteral("kind"), kind);
    command.values.insert(QStringLiteral("params"), params);
    if (kind == QLatin1String("gaussian-blur") && !values.isEmpty()) {
        command.values.insert(QStringLiteral("radius"), values.first());
    }
    return command;
}

QList<double> filterValues(const CommandParams& params)
{
    QList<double> values;
    for (const QJsonValue& value : params.values.value(QStringLiteral("params")).toArray()) {
        values.append(value.toDouble());
    }
    return values;
}

CommandParams galleryParams(const FilterGalleryDialog& dialog)
{
    QJsonArray kinds;
    QJsonArray params;
    for (const FilterGalleryDialog::Effect& effect : dialog.effects()) {
        if (!effect.visible) {
            continue;
        }
        kinds.append(effect.kind);
        for (const double value : effect.values) {
            params.append(value);
        }
    }
    CommandParams command;
    command.values.insert(QStringLiteral("kinds"), kinds);
    command.values.insert(QStringLiteral("params"), params);
    return command;
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
    if (!view || !filter_target_ready(*view) || refuseEmptyBlur(view, spec, this)) {
        return;
    }
    if (spec.params.isEmpty()) {
        // Nothing to edit: apply the engine defaults directly.
        if (commitFilter(this, view, activeDocumentCanvases(), spec.kind, QList<double>())) {
            refresh();
            registry_->recordCommand(commandIdForPath(spec.path),
                                     filterParams(spec.kind, QList<double>()));
        } else {
            reportFilterRefusal(view);
        }
        return;
    }
    QList<double> values;
    QString kind;
    // The dialog previews on the canvas as controls change; OK commits one
    // state on a worker, Cancel discards the preview bit-identically.
    if (runFilterDialog(view, spec, previewViewFor(activeDocumentCanvases()), QList<double>(),
                        &values, &kind, this)) {
        if (commitFilter(this, view, activeDocumentCanvases(), kind, values)) {
            refresh();
            registry_->recordCommand(commandIdForPath(spec.path), filterParams(kind, values));
        } else {
            reportFilterRefusal(view);
        }
    }
}

// Re-edit a generic or Filter Gallery Smart Filter: CS6's notice when filters
// are stacked above it, then its dialog prefilled; OK replaces it in place.
void PicturaMainWindow::editSmartFilter(const QString& path, int index)
{
    PictureView* view = activeView();
    if (!view) {
        return;
    }
    const int editKind = layer_smart_filter_edit_kind(*view, path, index);
    const QStringList kinds = layer_smart_filter_kinds(*view, path, index);
    const QList<double> params = layer_smart_filter_params(*view, path, index);
    if ((editKind != 3 && editKind != 4) || kinds.isEmpty()) {
        return;
    }
    if (index < layer_smart_filter_count(*view, path) - 1) {
        noteStackedSmartFilterEdit(this);
    }
    if (!smart_filter_edit_begin(*view, path, index)) {
        return;
    }
    if (editKind == 4) {
        QList<FilterGalleryDialog::Effect> effects;
        int at = 0;
        for (const QString& kind : kinds) {
            const int arity = filter_param_arity(kind);
            effects.append({kind, params.mid(at, arity), true});
            at += arity;
        }
        FilterGalleryDialog dialog(view, this);
        dialog.setEffects(effects);
        if (runDialog(dialog, this) == QDialog::Accepted
            && dialog.commit() == FilterGalleryDialog::CommitResult::Refused) {
            reportFilterRefusal(view);
        }
    } else if (const FilterCommandSpec* spec = filterCommandForKind(kinds.first())) {
        QList<double> values;
        QString kind;
        if (runFilterDialog(view, *spec, previewViewFor(activeDocumentCanvases()), params, &values,
                            &kind, this)
            && !commitFilter(this, view, activeDocumentCanvases(), kind, values)) {
            reportFilterRefusal(view);
        }
    }
    smart_filter_edit_end(*view);
    refresh();
}

void PicturaMainWindow::wireFilterMenu()
{
    for (const FilterCommandSpec& spec : filterCommands()) {
        const QString id = commandIdForPath(spec.path);
        registry_->setImplemented(id, true);
        const FilterCommandSpec captured = spec;
        registry_->setHandler(id, [this, captured]() { applyFilterCommand(captured); });
        registry_->setExecutor(id, [this](const CommandParams& params) {
            PictureView* view = activeView();
            const QString kind = params.values.value(QStringLiteral("kind")).toString();
            if (!view || !filter_target_ready(*view) || kind.isEmpty()
                || !filter_kind_supported(kind)) {
                return false;
            }
            const bool applied = apply_filter_params(*view, kind, filterValues(params));
            if (applied) {
                refresh();
            }
            return applied;
        });
        // CS6 marks every dialog-opening entry with an ellipsis; a parameterless
        // entry applies straight away and stays bare.
        // The workspace filters' labels already carry it.
        registry_->setLabelProvider(id, [captured]() {
            const QString ellipsis = QStringLiteral("…");
            return captured.params.isEmpty() || captured.label.endsWith(ellipsis)
                       ? captured.label
                       : captured.label + ellipsis;
        });
        // Adaptive Wide Angle and Vanishing Point take 8-bit RGB and
        // Grayscale only, as CS6's 16- and 32-bpc filter lists leave them out;
        // Lens Correction takes 8- and 16-bit RGB and Grayscale.
        registry_->setEnabledProvider(id, [this, kind = captured.kind]() {
            PictureView* view = activeView();
            return view && filter_target_ready(*view) && filter_document_takes(*view, kind);
        });
    }

    // Filter > Filter Gallery: the stacked gallery dialog; OK commits the
    // visible effects as one history state.
    const QString gallery =
        commandIdForPath({QStringLiteral("Filter"), QStringLiteral("Filter Gallery…")});
    registry_->setImplemented(gallery, true);
    registry_->setHandler(gallery, [this, gallery]() {
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
            registry_->recordCommand(gallery, galleryParams(dialog));
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
    registry_->setExecutor(gallery, [this](const CommandParams& params) {
        PictureView* view = activeView();
        if (!view || !filter_target_ready(*view)) {
            return false;
        }
        QStringList kinds;
        for (const QJsonValue& value :
             params.values.value(QStringLiteral("kinds")).toArray()) {
            kinds.append(value.toString());
        }
        if (kinds.isEmpty()) {
            return false;
        }
        const bool applied = apply_filter_stack(*view, kinds, filterValues(params));
        if (applied) {
            refresh();
        }
        return applied;
    });

    // Filter > Last Filter: re-apply the last committed filter with no dialog.
    registry_->setHandler(command_ids::FilterLastFilter, [this]() {
        PictureView* view = activeView();
        if (!view || !filter_has_last(*view)) {
            return;
        }
        const FilterCommandSpec* spec = filterCommandForKind(filter_last_kind(*view));
        if (spec && refuseEmptyBlur(view, *spec, this)) {
            return;
        }
        if (commitFilter(this, view, activeDocumentCanvases(), filter_last_kind(*view),
                         filter_last_params(*view))) {
            refresh();
            registry_->recordCommand(command_ids::FilterLastFilter,
                                     filterParams(filter_last_kind(*view),
                                                  filter_last_params(*view)));
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
    registry_->setExecutor(command_ids::FilterLastFilter,
                           [this](const CommandParams& params) {
                               PictureView* view = activeView();
                               if (!view || !filter_target_ready(*view)) {
                                   return false;
                               }
                               const QString kind =
                                   params.values.value(QStringLiteral("kind")).toString();
                               if (kind.isEmpty() || !filter_kind_supported(kind)) {
                                   return false;
                               }
                               const bool applied =
                                   apply_filter_params(*view, kind, filterValues(params));
                               if (applied) {
                                   refresh();
                               }
                               return applied;
                           });

    // Filter > Last Filter Settings: reopen the last filter's dialog prefilled.
    registry_->setHandler(command_ids::FilterLastFilterSettings, [this]() {
        PictureView* view = activeView();
        if (!view || !filter_has_last(*view)) {
            return;
        }
        const FilterCommandSpec* spec = filterCommandForKind(filter_last_kind(*view));
        if (!spec || spec->params.isEmpty() || refuseEmptyBlur(view, *spec, this)) {
            return;
        }
        QList<double> values;
        QString kind;
        if (runFilterDialog(view, *spec, previewViewFor(activeDocumentCanvases()), filter_last_params(*view),
                            &values, &kind, this)) {
            if (commitFilter(this, view, activeDocumentCanvases(), kind, values)) {
                refresh();
                registry_->recordCommand(command_ids::FilterLastFilterSettings,
                                         filterParams(kind, values));
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
    registry_->setExecutor(command_ids::FilterLastFilterSettings,
                           [this](const CommandParams& params) {
                               PictureView* view = activeView();
                               const QString kind =
                                   params.values.value(QStringLiteral("kind")).toString();
                               if (!view || kind.isEmpty() || !filter_target_ready(*view)
                                   || !filter_kind_supported(kind)) {
                                   return false;
                               }
                               const bool applied =
                                   apply_filter_params(*view, kind, filterValues(params));
                               if (applied) {
                                   refresh();
                               }
                               return applied;
                           });
}

} // namespace pictura
