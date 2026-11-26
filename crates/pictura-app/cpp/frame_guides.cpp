// Rulers and guides (#294): View > Rulers / Show > Guides / Lock Guides /
// Clear Guides / New Guide…, each canvas's rulers and guide look, and the
// Guides, Grid, & Slices preferences. The guides themselves are document data
// (cxxqt_object/guides.rs); ToolController shows and drags them.

#include "frame_includes.h"

#include "new_guide_dialog.h"

#include "api/document_bridge.h"

namespace pictura {

void PicturaMainWindow::loadGuideSession(const SessionState& state, const Preferences& prefs)
{
    rulersVisible_ = state.rulersVisible;
    freePan_ = state.freePan;
    snapCanvasToWorkspace_ = state.snapCanvasToWorkspace;
    if (prefs.rulerUnit >= 0 && prefs.rulerUnit < kRulerUnitCount) {
        rulerUnit_ = RulerUnit(prefs.rulerUnit);
    }
    traditionalPoints_ = prefs.traditionalPoints;
    columnWidthInches_ = prefs.columnWidthInches;
    columnGutterInches_ = prefs.columnGutterInches;
    guidesVisible_ = state.guidesVisible;
    guidesLocked_ = state.guidesLocked;
    smartGuidesVisible_ = state.smartGuidesVisible;
    const QColor color(prefs.guideColor);
    if (color.isValid()) {
        guideColor_ = color;
    }
    guideDashed_ = prefs.guideDashed;
}

void PicturaMainWindow::prepareCanvasGuides(const DocWindow& window)
{
    window.host->setRulersVisible(rulersVisible_);
    window.host->setRulerUnit(rulerUnit_, traditionalPoints_);
    window.host->setPpiProvider([view = QPointer<PictureView>(window.doc->view)]() {
        return view && view->has_document() ? document_ppi(*view) : 72.0;
    });
    window.canvas->setGuidesVisible(guidesVisible_);
    window.canvas->setSmartGuidesEnabled(smartGuidesVisible_);
    window.canvas->setGuideAppearance(guideColor_, guideDashed_);
    connect(window.host, &CanvasScrollBars::rulerUnitChosen, this,
            &PicturaMainWindow::setRulerUnit);
    connect(window.host, &CanvasScrollBars::rulerPreferencesRequested, this,
            [this]() { showPreferences(PreferencesDialog::kUnits); });
    if (tools_) {
        tools_->setGuidesLocked(guidesLocked_);
    }
    connect(window.host, &CanvasScrollBars::guideDropped, this,
            [this, view = QPointer<PictureView>(window.doc->view)](bool vertical, double position) {
                if (!view) {
                    return;
                }
                // Placing a guide shows hidden guides, as in CS6.
                if (!effectiveExtra(command_ids::ViewShowGuides, guidesVisible_)) {
                    guidesVisible_ = true;
                    extras_ = extras_ || extraMember(command_ids::ViewShowGuides);
                    applyViewState();
                    saveSession();
                }
                add_guide(*view, vertical, position);
            });
}

void PicturaMainWindow::syncSiblingGuides()
{
    if (!documents_->activeWindow() || documents_->activeWindow()->doc->windows.size() < 2) {
        return;
    }
    PictureView* view = documents_->activeWindow()->doc->view;
    QList<ImageView::GuideLine> lines;
    const int count = view && view->has_document() ? guide_count(*view) : 0;
    for (int i = 0; i < count; ++i) {
        const ::rust::Vec<double> guide = guide_at(*view, i);
        if (guide.size() == 2) {
            lines.append({guide[0] != 0.0, guide[1]});
        }
    }
    ImageView* active = documents_->activeWindow()->canvas;
    forEachCanvas(*documents_->activeWindow()->doc, [active, &lines](ImageView* canvas) {
        if (canvas != active) {
            canvas->setGuides(lines);
        }
    });
}

void PicturaMainWindow::applyGuideViewState()
{
    for (const DocWindow* window : documents_->windows()) {
        // A document still loading has no canvas yet; setupCanvas() calls
        // prepareCanvasGuides() with the current state when it completes.
        if (!window->host || !window->canvas) {
            continue;
        }
        window->host->setRulersVisible(rulersVisible_);
        window->host->setRulerUnit(rulerUnit_, traditionalPoints_);
        window->canvas->setGuidesVisible(effectiveExtra(command_ids::ViewShowGuides, guidesVisible_));
        window->canvas->setSmartGuidesEnabled(
            effectiveExtra(command_ids::ViewShowSmartGuides, smartGuidesVisible_));
        window->canvas->setGuideAppearance(guideColor_, guideDashed_);
    }
    if (tools_) {
        tools_->setGuidesLocked(guidesLocked_);
    }
    if (commandRegistry()) {
        commandRegistry()->refresh();
    }
}

void PicturaMainWindow::registerGuideHandlers()
{
    // Show toggles also set the state that an opening document will inherit.
    const auto hasDocument = [this]() { return documentCount() > 0; };
    struct Toggle {
        const char* command;
        bool PicturaMainWindow::*flag;
    };
    static const Toggle kToggles[] = {
        {command_ids::ViewRulers, &PicturaMainWindow::rulersVisible_},
        {command_ids::ViewShowGuides, &PicturaMainWindow::guidesVisible_},
        {command_ids::ViewShowSmartGuides, &PicturaMainWindow::smartGuidesVisible_},
        {command_ids::ViewLockGuides, &PicturaMainWindow::guidesLocked_},
    };
    for (const Toggle& toggle : kToggles) {
        const char* command = toggle.command;
        bool PicturaMainWindow::*flag = toggle.flag;
        // Flip the flag rather than read the action, so a control-server
        // dispatch (which runs the handler without toggling the action)
        // toggles too; the checked provider then re-syncs the action.
        const bool showToggle = command == command_ids::ViewShowGuides
            || command == command_ids::ViewShowSmartGuides;
        commandRegistry()->setHandler(command, [this, flag, showToggle, hasDocument]() {
            if (showToggle && !hasDocument()) {
                return;
            }
            this->*flag = !(this->*flag);
            applyGuideViewState();
            saveSession();
        });
        commandRegistry()->setCheckedProvider(command, [this, flag]() { return this->*flag; });
        if (showToggle) {
            commandRegistry()->setEnabledProvider(command, hasDocument);
        }
    }

    commandRegistry()->setHandler(command_ids::ViewClearGuides, [this]() {
        if (PictureView* view = activeView()) {
            clear_guides(*view);
        }
    });
    commandRegistry()->setEnabledProvider(command_ids::ViewClearGuides, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && guide_count(*view) > 0;
    });

    commandRegistry()->setHandler(command_ids::ViewNewGuide, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        NewGuideDialog dialog(this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return;
        }
        if (!effectiveExtra(command_ids::ViewShowGuides, guidesVisible_)) {
            guidesVisible_ = true;
            extras_ = extras_ || extraMember(command_ids::ViewShowGuides);
            applyViewState();
            saveSession();
        }
        add_guide(*view, dialog.vertical(), dialog.position());
    });
    commandRegistry()->setEnabledProvider(command_ids::ViewNewGuide, [this]() {
        PictureView* view = activeView();
        return view && view->has_document();
    });
}

void PicturaMainWindow::setRulerUnit(RulerUnit unit)
{
    rulerUnit_ = unit;
    if (preferencesDialog_) {
        preferencesDialog_->setRulerUnit(int(unit));
    }
    applyGuideViewState();
    saveSession();
}

void PicturaMainWindow::connectGuidePreferences(PreferencesDialog* dialog)
{
    dialog->setRulerUnit(int(rulerUnit_));
    dialog->setTraditionalPoints(traditionalPoints_);
    dialog->setColumnSize(columnWidthInches_, columnGutterInches_);
    connect(dialog, &PreferencesDialog::columnSizeChanged, this,
            [this](double width, double gutter) {
                columnWidthInches_ = width;
                columnGutterInches_ = gutter;
                saveSession();
            });
    connect(dialog, &PreferencesDialog::rulerUnitChanged, this, [this](int unit) {
        if (unit >= 0 && unit < kRulerUnitCount) {
            setRulerUnit(RulerUnit(unit));
        }
    });
    connect(dialog, &PreferencesDialog::traditionalPointsChanged, this, [this](bool on) {
        traditionalPoints_ = on;
        applyGuideViewState();
        saveSession();
    });
    dialog->setGuideAppearance(guideColor_, guideDashed_);
    connect(dialog, &PreferencesDialog::guideAppearanceChanged, this,
            [this](const QColor& color, bool dashed) {
                guideColor_ = color;
                guideDashed_ = dashed;
                applyGuideViewState();
                saveSession();
            });
    dialog->setGridSettings(QColor(gridColor_), gridStyle_, gridUnit_, gridEvery_,
                            gridSubdivisions_);
    connect(dialog, &PreferencesDialog::gridSettingsChanged, this,
            [this](const QColor& color, int style, int unit, double every, int subdivisions) {
                gridColor_ = color.name();
                gridStyle_ = qBound(0, style, 2);
                gridUnit_ = qBound(0, unit, 6);
                gridEvery_ = every > 0.0 ? every : 1.0;
                gridSubdivisions_ = qMax(1, subdivisions);
                applyViewState();
                saveSession();
            });
}

} // namespace pictura
