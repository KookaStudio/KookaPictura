// View > Show, Extras, grid, pixel grid, layer edges, the persistent target
// path and the View zoom presets. The state is application-wide and applied to
// every canvas; the tool that owns an overlay only overrides its Show flag,
// never Extras.

#include "frame_includes.h"

#include "api/paint_bridge.h"
#include "api/selection_api.h"
#include "api/transform_api.h"
#include "path_overlay.h"
#include "show_extras_dialog.h"

namespace pictura {

namespace {

QStringList defaultExtrasMembers()
{
    return {
        QString::fromLatin1(command_ids::ViewShowSelectionEdges),
        QString::fromLatin1(command_ids::ViewShowTargetPath),
        QString::fromLatin1(command_ids::ViewShowLayerEdges),
        QString::fromLatin1(command_ids::ViewShowGuides),
        QString::fromLatin1(command_ids::ViewShowGrid),
        QString::fromLatin1(command_ids::ViewShowCount),
        QString::fromLatin1(command_ids::ViewShowSmartGuides),
        QString::fromLatin1(command_ids::ViewShowSlices),
        QString::fromLatin1(command_ids::ViewShowNotes),
        QString::fromLatin1(command_ids::ViewShowPixelGrid),
    };
}

} // namespace

void PicturaMainWindow::loadViewSession(const SessionState& state)
{
    selectionEdges_ = state.selectionEdges;
    showTargetPath_ = state.showTargetPath;
    showLayerEdges_ = state.showLayerEdges;
    showGrid_ = state.showGrid;
    showCount_ = state.showCount;
    showSlices_ = state.showSlices;
    showNotes_ = state.showNotes;
    showPixelGrid_ = state.showPixelGrid;
    showColorSamplers_ = state.showColorSamplers;
    extras_ = state.extras;
    extrasMembers_ = state.extrasMembersSet ? state.extrasMembers : defaultExtrasMembers();
    lockSlices_ = state.lockSlices;
    gridColor_ = state.gridColor;
    gridStyle_ = state.gridStyle;
    gridUnit_ = state.gridUnit;
    gridEvery_ = state.gridEvery;
    gridSubdivisions_ = state.gridSubdivisions;
    applyViewState();
}

bool PicturaMainWindow::extraMember(const QString& id) const
{
    return extrasMembers_.contains(id);
}

bool PicturaMainWindow::effectiveExtra(const QString& id, bool enabled, bool toolOwns) const
{
    return (enabled || toolOwns) && (extras_ || !extraMember(id));
}

bool PicturaMainWindow::pathToolActive() const
{
    if (!tools_) {
        return false;
    }
    switch (tools_->activeTool()) {
    case ToolId::Pen:
    case ToolId::FreeformPen:
    case ToolId::AddAnchorPoint:
    case ToolId::DeleteAnchorPoint:
    case ToolId::ConvertPoint:
    case ToolId::PathSelection:
    case ToolId::DirectSelection:
    case ToolId::Rectangle:
    case ToolId::RoundedRectangle:
    case ToolId::Ellipse:
    case ToolId::Polygon:
    case ToolId::Line:
    case ToolId::CustomShape:
        return true;
    default:
        return false;
    }
}

void PicturaMainWindow::applyViewState()
{
    if (tools_) {
        tools_->setSlicesLocked(lockSlices_);
    }
    if (!documents_) {
        return;
    }
    for (const DocWindow* window : documents_->windows()) {
        if (!window || !window->canvas) {
            continue;
        }
        applyCanvasViewState(*window);
    }
    for (DocEntry* doc : documents_->docs()) {
        refreshLayerEdges(*doc);
    }
    if (commandRegistry()) {
        commandRegistry()->refresh();
    }
}

void PicturaMainWindow::applyCanvasViewState(const DocWindow& window)
{
    ImageView* canvas = window.canvas;
    const ToolId tool = tools_ ? tools_->activeTool() : ToolId::Move;
    const bool sliceTool = tool == ToolId::Slice || tool == ToolId::SliceSelect;
    const bool noteTool = tool == ToolId::Note;
    const bool countTool = tool == ToolId::Count;
    canvas->setSelectionEdgesVisible(
        effectiveExtra(command_ids::ViewShowSelectionEdges, selectionEdges_));
    canvas->setGuidesVisible(effectiveExtra(command_ids::ViewShowGuides, guidesVisible_));
    canvas->setSmartGuidesEnabled(
        effectiveExtra(command_ids::ViewShowSmartGuides, smartGuidesVisible_));
    canvas->setLayerEdgesVisible(
        effectiveExtra(command_ids::ViewShowLayerEdges, showLayerEdges_));
    canvas->setPixelGridVisible(
        effectiveExtra(command_ids::ViewShowPixelGrid, showPixelGrid_));
    canvas->setSlicesVisible(
        effectiveExtra(command_ids::ViewShowSlices, showSlices_, sliceTool));
    canvas->setNotesVisible(
        effectiveExtra(command_ids::ViewShowNotes, showNotes_, noteTool));
    canvas->setCountVisible(
        edition_is_extended()
        && effectiveExtra(command_ids::ViewShowCount, showCount_, countTool));
    canvas->setSamplersVisible(showColorSamplers_ && extras_);

    applyGridState(window);
    refreshTargetPathOverlay(window);
}

void PicturaMainWindow::applyGridState(const DocWindow& window)
{
    PictureView* view = window.doc ? window.doc->view : nullptr;
    double everyX = gridEvery_;
    double everyY = gridEvery_;
    if (view && view->has_document()) {
        const double ppi = document_ppi(*view);
        switch (gridUnit_) {
        case 2:
            everyX = everyY = gridEvery_ * ppi / 2.54;
            break;
        case 3:
            everyX = everyY = gridEvery_ * ppi / 25.4;
            break;
        case 4:
            everyX = everyY = gridEvery_ * ppi / 72.0;
            break;
        case 5:
            everyX = everyY = gridEvery_ * ppi / 6.0;
            break;
        case 6:
            everyX = view->document_width() * gridEvery_ / 100.0;
            everyY = view->document_height() * gridEvery_ / 100.0;
            break;
        case 0:
            break;
        default:
            everyX = everyY = gridEvery_ * ppi;
            break;
        }
    }
    window.canvas->setGrid(effectiveExtra(command_ids::ViewShowGrid, showGrid_),
                           QColor(gridColor_), gridStyle_, everyX, everyY, gridSubdivisions_);
}

void PicturaMainWindow::refreshViewOverlays()
{
    if (!documents_) {
        return;
    }
    const ToolId tool = tools_ ? tools_->activeTool() : ToolId::Move;
    const bool sliceTool = tool == ToolId::Slice || tool == ToolId::SliceSelect;
    const bool noteTool = tool == ToolId::Note;
    const bool countTool = tool == ToolId::Count;
    for (const DocWindow* window : documents_->windows()) {
        if (!window || !window->canvas || !window->doc || !window->doc->view) {
            continue;
        }
        PictureView* view = window->doc->view;
        ImageView* canvas = window->canvas;
        applyGridState(*window);
        if (!view->has_document()) {
            canvas->clearSliceOverlay();
            canvas->setAnnotationOverlay({}, {}, -1);
            canvas->setCountOverlay({});
            canvas->setTargetPathOverlay({});
            canvas->setLayerEdges(QRectF());
            continue;
        }

        if (!(sliceTool && tools_ && tools_->canvas() == canvas)) {
            canvas->setDocumentSliceOverlay(*view);
        }

        QList<QPointF> samplers;
        QList<QPointF> notes;
        for (int kind = 0; kind < 2; ++kind) {
            const int count = marker_count(*view, kind);
            for (int i = 0; i < count; ++i) {
                const ::rust::Vec<std::int32_t> point = marker_at(*view, kind, i);
                if (point.size() == 2) {
                    (kind == 0 ? samplers : notes).append(QPointF(point[0], point[1]));
                }
            }
        }
        const bool currentDocument = tools_ && tools_->view() == view;
        canvas->setAnnotationOverlay(samplers, notes, currentDocument ? tools_->currentNote() : -1);

        QList<ImageView::CountOverlayMark> counts;
        const int groups = count_group_count(*view);
        for (int group = 0; group < groups; ++group) {
            if (!count_group_visible(*view, group)) {
                continue;
            }
            const QColor color(QRgb(count_group_color(*view, group)));
            const int markerSize = count_group_marker_size(*view, group);
            const int labelSize = count_group_label_size(*view, group);
            const int total = count_group_total(*view, group);
            for (int i = 0; i < total; ++i) {
                const ::rust::Vec<std::int32_t> point = count_group_mark_at(*view, group, i);
                if (point.size() == 2) {
                    counts.append({QPointF(point[0], point[1]), i + 1, color, markerSize,
                                   labelSize});
                }
            }
        }
        canvas->setCountOverlay(counts);

        refreshTargetPathOverlay(*window);
        canvas->setSlicesVisible(
            effectiveExtra(command_ids::ViewShowSlices, showSlices_, sliceTool));
        canvas->setNotesVisible(
            effectiveExtra(command_ids::ViewShowNotes, showNotes_, noteTool));
        canvas->setCountVisible(
            edition_is_extended()
            && effectiveExtra(command_ids::ViewShowCount, showCount_, countTool));
    }
    for (DocEntry* doc : documents_->docs()) {
        refreshLayerEdges(*doc);
    }
}

void PicturaMainWindow::refreshTargetPathOverlay(const DocWindow& window)
{
    ImageView* canvas = window.canvas;
    if (!canvas) {
        return;
    }
    const bool editingHere = pathToolActive() && tools_->canvas() == canvas;
    canvas->setPathOverlayVisible(effectiveExtra(command_ids::ViewShowTargetPath, true));
    const bool visible = effectiveExtra(command_ids::ViewShowTargetPath, showTargetPath_)
        && !editingHere;
    canvas->setTargetPathVisible(visible);
    PictureView* view = window.doc ? window.doc->view : nullptr;
    if (!visible || !view || !view->has_document()) {
        return;
    }
    const bool shapeTarget = paths_active(*view) == -2;
    const bool hadLayerTarget = path_layer_target(*view);
    const QString target = view->active_layer_path();
    canvas->setTargetPathFollowsMove(
        (shapeTarget || hadLayerTarget) && !target.isEmpty()
        && shape_is_shape(*view, target));
    if (shapeTarget != hadLayerTarget) {
        path_set_layer_target(*view, shapeTarget);
    }
    canvas->setTargetPathOverlay(workPathOverlay(*view, -2));
    if (shapeTarget != hadLayerTarget) {
        path_set_layer_target(*view, hadLayerTarget);
    }
}

void PicturaMainWindow::refreshLayerEdges(DocEntry& doc)
{
    PictureView* view = doc.view;
    if (!view || !view->has_document()
        || !effectiveExtra(command_ids::ViewShowLayerEdges, showLayerEdges_)) {
        return;
    }
    const QString active = view->active_layer_path();
    const quint64 revision = view->canvas_revision();
    // A Move changes the layer's rect before (or without) a canvas revision on
    // the deferred paths; key on the rect too, so the outline follows the text
    // instead of staying at the previous place.
    const QString geometry =
        active.isEmpty() ? QString() : transform_api::layerRect(*view, active).toString();
    if (!doc.layerEdges || doc.layerEdges->path != active
        || doc.layerEdges->revision != revision || doc.layerEdges->geometry != geometry) {
        QRectF edges;
        if (!active.isEmpty()) {
            const auto rect = transform_api::contentRect(*view, active);
            if (rect.valid) {
                edges = QRectF(rect.left, rect.top, rect.width(), rect.height());
            }
        }
        doc.layerEdges = DocEntry::LayerEdgeCache{active, revision, geometry, edges};
    }
    forEachCanvas(doc, [&doc](ImageView* canvas) { canvas->setLayerEdges(doc.layerEdges->rect); });
}

void PicturaMainWindow::showExtrasOptions()
{
    QList<ShowExtrasDialog::Entry> entries = {
        {QString::fromLatin1(command_ids::ViewShowSelectionEdges), tr("Selection Edges")},
        {QString::fromLatin1(command_ids::ViewShowTargetPath), tr("Target Path")},
        {QString::fromLatin1(command_ids::ViewShowLayerEdges), tr("Layer Edges")},
        {QString::fromLatin1(command_ids::ViewShowGuides), tr("Guides")},
        {QString::fromLatin1(command_ids::ViewShowGrid), tr("Grid")},
        {QString::fromLatin1(command_ids::ViewShowSmartGuides), tr("Smart Guides")},
        {QString::fromLatin1(command_ids::ViewShowSlices), tr("Slices")},
        {QString::fromLatin1(command_ids::ViewShowNotes), tr("Notes")},
        {QString::fromLatin1(command_ids::ViewShowPixelGrid), tr("Pixel Grid")},
    };
    if (edition_is_extended()) {
        entries.append(
            {QString::fromLatin1(command_ids::ViewShowCount), tr("Count")});
    }
    ShowExtrasDialog dialog(entries, extrasMembers_, this);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return;
    }
    QStringList selected = dialog.selectedIds();
    // Count is Extended-only; a Standard edit keeps its stored membership
    // instead of silently dropping it from the set.
    if (!edition_is_extended()) {
        const QString count = QString::fromLatin1(command_ids::ViewShowCount);
        if (extrasMembers_.contains(count) && !selected.contains(count)) {
            selected.append(count);
        }
    }
    extrasMembers_ = selected;
    applyViewState();
    saveSession();
}

void PicturaMainWindow::wireViewStateSignals()
{
    if (viewStateSignalsWired_ || !tools_) {
        return;
    }
    connect(tools_, &ToolController::activeToolChanged, this,
            [this](ToolId) { refreshViewOverlays(); });
    viewStateSignalsWired_ = true;
    applyViewState();
}

void PicturaMainWindow::refreshDocumentSelection(DocEntry& doc)
{
    PictureView* view = doc.view;
    if (!view || !view->has_document()) {
        return;
    }
    const quint64 generation = view->selection_generation();
    if (doc.selectionGeneration != generation) {
        doc.selectionGeneration = generation;
        if (!selectionEdges_) {
            selectionEdges_ = true;
            saveSession();
            applyViewState();
        }
    }
    // A live border transform supplies its own preview on the active canvas.
    if (view->has_selection()
        && !(tools_ && tools_->view() == view && tools_->transformSelectionTarget())) {
        const auto contour = selection_api::contour(*view);
        forEachCanvas(doc, [&contour](ImageView* canvas) { canvas->setSelectionContour(contour); });
    } else {
        forEachCanvas(doc, [](ImageView* canvas) { canvas->clearSelectionContour(); });
    }
}

void PicturaMainWindow::registerViewHandlers()
{
    registerHdrPreviewHandler();
    CommandRegistry* registry = commandRegistry();
    if (!registry) {
        return;
    }
    const auto hasDocument = [this]() { return documentCount() > 0; };
    struct Toggle {
        const char* command;
        bool PicturaMainWindow::*flag;
        bool documentScoped;
    };
    static const Toggle kToggles[] = {
        {command_ids::ViewExtras, &PicturaMainWindow::extras_, true},
        {command_ids::ViewShowSelectionEdges, &PicturaMainWindow::selectionEdges_, true},
        {command_ids::ViewShowTargetPath, &PicturaMainWindow::showTargetPath_, true},
        {command_ids::ViewShowLayerEdges, &PicturaMainWindow::showLayerEdges_, true},
        {command_ids::ViewShowGrid, &PicturaMainWindow::showGrid_, true},
        {command_ids::ViewShowCount, &PicturaMainWindow::showCount_, true},
        {command_ids::ViewShowSlices, &PicturaMainWindow::showSlices_, true},
        {command_ids::ViewShowNotes, &PicturaMainWindow::showNotes_, true},
        {command_ids::ViewShowPixelGrid, &PicturaMainWindow::showPixelGrid_, true},
        {command_ids::ViewLockSlices, &PicturaMainWindow::lockSlices_, false},
    };
    for (const Toggle& toggle : kToggles) {
        const char* command = toggle.command;
        if (toggle.flag == &PicturaMainWindow::showCount_ && !edition_is_extended()) {
            continue;
        }
        bool PicturaMainWindow::*flag = toggle.flag;
        const bool scoped = toggle.documentScoped;
        registry->setHandler(command, [this, flag, scoped, hasDocument]() {
            if (scoped && !hasDocument()) {
                return;
            }
            this->*flag = !(this->*flag);
            applyViewState();
            saveSession();
        });
        registry->setCheckedProvider(command, [this, flag]() { return this->*flag; });
        if (scoped) {
            registry->setEnabledProvider(command, hasDocument);
        }
    }

    registry->setHandler(command_ids::ViewShowExtraOptions, [this]() {
        showExtrasOptions();
    });

    registry->setHandler(command_ids::ViewShowAll, [this, hasDocument]() {
        if (!hasDocument()) {
            return;
        }
        selectionEdges_ = true;
        showTargetPath_ = true;
        showLayerEdges_ = true;
        showGrid_ = true;
        showCount_ = true;
        showSlices_ = true;
        showNotes_ = true;
        showPixelGrid_ = true;
        showColorSamplers_ = true;
        guidesVisible_ = true;
        smartGuidesVisible_ = true;
        extras_ = true;
        applyViewState();
        saveSession();
    });
    registry->setEnabledProvider(command_ids::ViewShowAll, hasDocument);
    registry->setHandler(command_ids::ViewShowNone, [this, hasDocument]() {
        if (!hasDocument()) {
            return;
        }
        selectionEdges_ = false;
        showTargetPath_ = false;
        showLayerEdges_ = false;
        showGrid_ = false;
        showCount_ = false;
        showSlices_ = false;
        showNotes_ = false;
        showPixelGrid_ = false;
        showColorSamplers_ = false;
        guidesVisible_ = false;
        smartGuidesVisible_ = false;
        applyViewState();
        saveSession();
    });
    registry->setEnabledProvider(command_ids::ViewShowNone, hasDocument);

    registry->setHandler(command_ids::ViewClearSlices, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document() || user_slice_count(*view) <= 0) {
            return;
        }
        if (clear_user_slices(*view)) {
            refreshViewOverlays();
        }
    });
    registry->setEnabledProvider(command_ids::ViewClearSlices, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && user_slice_count(*view) > 0;
    });

    const auto hasReadyCanvas = [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && imageView();
    };
    registry->setHandler(command_ids::View200, [this, hasReadyCanvas]() {
        if (!hasReadyCanvas()) {
            return;
        }
        if (ImageView* canvas = imageView()) {
            canvas->setZoom(2.0, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
        }
    });
    registry->setHandler(command_ids::ViewPrintSize, [this, hasReadyCanvas]() {
        if (!hasReadyCanvas()) {
            return;
        }
        PictureView* view = activeView();
        if (ImageView* canvas = imageView(); view && canvas) {
            canvas->printSize(document_ppi(*view));
        }
    });
    registry->setEnabledProvider(command_ids::View200, hasReadyCanvas);
    registry->setEnabledProvider(command_ids::ViewPrintSize, hasReadyCanvas);
    wireViewStateSignals();
    applyViewState();
}

} // namespace pictura
