// Rulers and guides (#294): View > Rulers / Show > Guides / Lock Guides /
// Clear Guides / New Guide…, each canvas's rulers and guide look, and the
// Guides, Grid, & Slices preferences. The guides themselves are document data
// (cxxqt_object/guides.rs); ToolController shows and drags them.

#include "frame_includes.h"

#include "new_guide_dialog.h"

#include "pictura_app/src/cxxqt_object/guides.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

namespace pictura {

void PicturaMainWindow::loadGuideSession(const SessionState& state)
{
    rulersVisible_ = state.rulersVisible;
    freePan_ = state.freePan;
    snapCanvasToWorkspace_ = state.snapCanvasToWorkspace;
    if (state.rulerUnit >= 0 && state.rulerUnit < kRulerUnitCount) {
        rulerUnit_ = RulerUnit(state.rulerUnit);
    }
    traditionalPoints_ = state.traditionalPoints;
    guidesVisible_ = state.guidesVisible;
    guidesLocked_ = state.guidesLocked;
    smartGuidesVisible_ = state.smartGuidesVisible;
    const QColor color(state.guideColor);
    if (color.isValid()) {
        guideColor_ = color;
    }
    guideDashed_ = state.guideDashed;
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
                if (!guidesVisible_) {
                    guidesVisible_ = true;
                    applyGuideViewState();
                    saveSession();
                }
                add_guide(*view, vertical, position);
            });
}

void PicturaMainWindow::applyGuideViewState()
{
    for (const DocWindow* window : windows_) {
        // A document still loading has no canvas yet; setupCanvas() calls
        // prepareCanvasGuides() with the current state when it completes.
        if (!window->host || !window->canvas) {
            continue;
        }
        window->host->setRulersVisible(rulersVisible_);
        window->host->setRulerUnit(rulerUnit_, traditionalPoints_);
        window->canvas->setGuidesVisible(guidesVisible_);
        window->canvas->setSmartGuidesEnabled(smartGuidesVisible_);
        window->canvas->setGuideAppearance(guideColor_, guideDashed_);
    }
    if (tools_) {
        tools_->setGuidesLocked(guidesLocked_);
    }
    if (registry_) {
        registry_->refresh();
    }
}

void PicturaMainWindow::registerGuideHandlers()
{
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
        registry_->setHandler(command, [this, flag]() {
            this->*flag = !(this->*flag);
            applyGuideViewState();
            saveSession();
        });
        registry_->setCheckedProvider(command, [this, flag]() { return this->*flag; });
    }

    registry_->setHandler(command_ids::ViewClearGuides, [this]() {
        if (PictureView* view = activeView()) {
            clear_guides(*view);
        }
    });
    registry_->setEnabledProvider(command_ids::ViewClearGuides, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && guide_count(*view) > 0;
    });

    registry_->setHandler(command_ids::ViewNewGuide, [this]() {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        NewGuideDialog dialog(this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return;
        }
        if (!guidesVisible_) {
            guidesVisible_ = true;
            applyGuideViewState();
            saveSession();
        }
        add_guide(*view, dialog.vertical(), dialog.position());
    });
    registry_->setEnabledProvider(command_ids::ViewNewGuide, [this]() {
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
}

} // namespace pictura
