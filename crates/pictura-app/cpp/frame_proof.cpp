// View > Proof Setup and Proof Colors (`color/proof-setup`): application view
// state, pushed to every open view and persisted in the session.

#include "frame_includes.h"

#include "proof_setup_dialog.h"

#include "api/engine_bridge.h"
#include "api/proofing_bridge.h"

namespace pictura {

namespace {

const char* const kProofDeviceNames[] = {
    "Working CMYK", "Working Gray", "sRGB IEC61966-2.1", "Adobe RGB (1998)",
    "ProPhoto RGB",
};

} // namespace

void PicturaMainWindow::registerProofHandlers()
{
    const auto select = [this](int device) {
        proofDevice_ = qBound(0, device, 4);
        for (const DocEntry* entry : documents_->docs()) {
            if (entry && entry->view) {
                applyProofState(entry->view);
            }
        }
        if (commandRegistry()) {
            commandRegistry()->refresh();
        }
        refresh();
        updateWindowTitle();
        saveSession();
    };
    commandRegistry()->setHandler(command_ids::ViewProofSetupWorkingCmyk,
                                  [select]() { select(0); });
    commandRegistry()->setHandler(command_ids::ViewProofSetupWorkingGray,
                                  [select]() { select(1); });
    commandRegistry()->setHandler(command_ids::ViewProofSetupCustom,
                                  [this]() { showProofSetupCustom(); });
    commandRegistry()->setHandler(command_ids::ViewProofColors,
                                  [this]() { setProofColors(!proofColors_); });

    commandRegistry()->setCheckedProvider(command_ids::ViewProofSetupWorkingCmyk,
                                          [this]() { return proofDevice_ == 0; });
    commandRegistry()->setCheckedProvider(command_ids::ViewProofSetupWorkingGray,
                                          [this]() { return proofDevice_ == 1; });
    commandRegistry()->setCheckedProvider(command_ids::ViewProofColors,
                                          [this]() { return proofColors_; });
    for (const char* id : {command_ids::ViewProofSetupWorkingCmyk,
                           command_ids::ViewProofSetupWorkingGray,
                           command_ids::ViewProofSetupCustom, command_ids::ViewProofColors}) {
        commandRegistry()->setEnabledProvider(id, [this]() {
            PictureView* view = activeView();
            return view && view->has_document();
        });
    }
}

void PicturaMainWindow::applyProofState(PictureView* view)
{
    if (!view) {
        return;
    }
    pictura::set_proof_setup(*view, proofDevice_, proofIntent_, proofBpc_,
                             proofSimulatePaper_, proofSimulateBlack_);
    pictura::set_proof_colors(*view, proofColors_);
}

void PicturaMainWindow::showProofSetupCustom()
{
    ProofSetupValues values;
    values.device = proofDevice_;
    values.intent = proofIntent_;
    values.bpc = proofBpc_;
    values.simulatePaper = proofSimulatePaper_;
    values.simulateBlack = proofSimulateBlack_;
    ProofSetupDialog dialog(values, this);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return;
    }
    const ProofSetupValues out = dialog.values();
    proofDevice_ = out.device;
    proofIntent_ = out.intent;
    proofBpc_ = out.bpc;
    proofSimulatePaper_ = out.simulatePaper;
    proofSimulateBlack_ = out.simulateBlack;
    for (const DocEntry* entry : documents_->docs()) {
        if (entry && entry->view) {
            applyProofState(entry->view);
        }
    }
    if (commandRegistry()) {
        commandRegistry()->refresh();
    }
    refresh();
    updateWindowTitle();
    saveSession();
}

void PicturaMainWindow::setProofColors(bool on)
{
    proofColors_ = on;
    for (const DocEntry* entry : documents_->docs()) {
        if (entry && entry->view) {
            applyProofState(entry->view);
        }
    }
    if (commandRegistry()) {
        commandRegistry()->refresh();
    }
    refresh();
    updateWindowTitle();
    saveSession();
}

QString PicturaMainWindow::proofSetupName() const
{
    return QString::fromLatin1(kProofDeviceNames[qBound(0, proofDevice_, 4)]);
}

} // namespace pictura
