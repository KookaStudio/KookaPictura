#include "menu_registrar.h"

#include "commands.h"
#include "api/layer_comps_bridge.h"
#include "variables_dialog.h"

namespace pictura {

void MenuRegistrar::wireVariablesMenu()
{
    using namespace Qt::StringLiterals;
    const auto count = [this](const QString& group) {
        auto* view = activeView();
        return view && variables_available(*view, group);
    };

    commands_->setHandler("image.variables.define", [this] {
        if (auto* view = activeView()) {
            variable_dialogs::define(*view, imageView(), dialogParent(), false);
        }
    });
    commands_->setEnabledProvider("image.variables.define", [count] { return count(QString()); });
    commands_->setHandler("image.variables.data.sets", [this] {
        if (auto* view = activeView()) {
            variable_dialogs::define(*view, imageView(), dialogParent(), true);
        }
    });
    commands_->setEnabledProvider("image.variables.data.sets", [count] { return count(u"variables"_s); });
    commands_->setHandler("file.import.variable.data.sets", [this] {
        if (auto* view = activeView()) {
            variable_dialogs::importSets(*view, dialogParent());
        }
    });
    commands_->setEnabledProvider("file.import.variable.data.sets", [count] { return count(u"variables"_s); });
    commands_->setHandler("image.apply.data.set", [this] {
        if (auto* view = activeView()) {
            variable_dialogs::apply(*view, imageView(), dialogParent());
        }
    });
    commands_->setEnabledProvider("image.apply.data.set", [count] { return count(u"sets"_s); });
    commands_->setHandler("file.export.data.sets.as.files", [this] {
        if (auto* view = activeView()) {
            variable_dialogs::exportSets(*view, dialogParent(), activeFilePath(), activeDocumentName());
        }
    });
    commands_->setEnabledProvider("file.export.data.sets.as.files", [count] { return count(u"sets"_s); });
}

} // namespace pictura
