// Color Settings, Convert to Profile, and the settings the frame applies to
// every view. Split from frame.cpp to stay under the file-size cap.

#include "frame_includes.h"
#include "api/document_api.h"

#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"

namespace pictura {

void PicturaMainWindow::showProfileCommand(bool convert)
{
    PictureView* view = activeView();
    if (!view || !view->has_document() || document_api::documentMode(*view) != QStringLiteral("rgb")) {
        return;
    }
    ProfileDialog dialog(convert, pictura::document_profile_name(*view), this);
    if (convert) {
        dialog.setConversionOptions(renderIntent_, blackPointCompensation_, useDither_);
        connect(&dialog, &ProfileDialog::optionsChanged, this, [&dialog, view]() {
            if (dialog.preview()) {
                pictura::preview_convert_profile(
                    *view, dialog.profileIndex(), dialog.renderIntent(),
                    dialog.blackPointCompensation(), dialog.useDither(), dialog.flatten());
            } else {
                pictura::cancel_convert_preview(*view);
            }
        });
    }
    if (runDialog(dialog, this) != QDialog::Accepted) {
        if (convert) {
            pictura::cancel_convert_preview(*view);
        }
        return;
    }
    const bool ok = convert
        ? pictura::convert_profile_with_options(
              *view, dialog.profileIndex(), dialog.renderIntent(),
              dialog.blackPointCompensation(), dialog.useDither(), dialog.flatten())
        : view->assign_profile(dialog.profileIndex());
    if (ok) {
        refresh();
    }
}

void PicturaMainWindow::applyColorSettings(PictureView* view)
{
    if (!view) {
        return;
    }
    view->set_color_policy(colorPolicy_);
    pictura::set_color_settings(*view, cmykPolicy_, workingCmyk_, renderIntent_,
                                blackPointCompensation_, useDither_, askMismatchOpen_,
                                askMissingOpen_);
    pictura::set_picker_choice(*view, pickerChoice_);
}

void PicturaMainWindow::showColorSettings()
{
    ColorSettingsValues values;
    values.rgbPolicy = colorPolicy_;
    values.cmykPolicy = cmykPolicy_;
    values.workingCmyk = workingCmyk_;
    values.renderIntent = renderIntent_;
    values.blackPointCompensation = blackPointCompensation_;
    values.useDither = useDither_;
    values.askMismatchOpen = askMismatchOpen_;
    values.askMissingOpen = askMissingOpen_;
    values.textGammaEnabled = textGammaEnabled_;
    values.textGamma = textGamma_;
    ColorSettingsDialog dialog(values, this);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return;
    }
    const ColorSettingsValues out = dialog.values();
    colorPolicy_ = out.rgbPolicy;
    cmykPolicy_ = out.cmykPolicy;
    workingCmyk_ = out.workingCmyk;
    renderIntent_ = out.renderIntent;
    blackPointCompensation_ = out.blackPointCompensation;
    useDither_ = out.useDither;
    askMismatchOpen_ = out.askMismatchOpen;
    askMissingOpen_ = out.askMissingOpen;
    applyTextGamma(out.textGammaEnabled, out.textGamma);
    for (const DocEntry* entry : docs_) {
        if (entry) {
            applyColorSettings(entry->view);
        }
    }
    saveSession();
}

} // namespace pictura
