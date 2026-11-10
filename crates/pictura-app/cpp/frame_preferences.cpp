// The Edit > Preferences leaves and the dialog they open, lifted out of
// frame.cpp / frame_menus.cpp to keep them within their size budgets.

#include "frame_includes.h"
#include "agent_control.h"
#include "api/paint_bridge.h"
#include "api/type_api.h"
#include "frame_update.h"
#include "panels/actions_engine_bridge.h"
#include "preferences_agent_page.h"
#include "settings_backup.h"
#include "update_preferences_page.h"

#include <QtCore/QDir>
#include <QtCore/QStandardPaths>
#include <QtWidgets/QMessageBox>

namespace pictura {

// The implemented Preferences leaves; the other CS6 panes stay disabled and
// are no-ops (their command-tree enablement is unchanged).
void PicturaMainWindow::registerPreferenceHandlers()
{
    registry_->setHandler(command_ids::EditPreferencesGeneral,
                          [this]() { showPreferences(PreferencesDialog::kGeneral); });
    registry_->setHandler(command_ids::EditPreferencesInterface,
                          [this]() { showPreferences(PreferencesDialog::kInterface); });
    registry_->setHandler(command_ids::EditPreferencesPerformance,
                          [this]() { showPreferences(PreferencesDialog::kPerformance); });
    registry_->setHandler(command_ids::EditPreferencesUnits,
                          [this]() { showPreferences(PreferencesDialog::kUnits); });
    registry_->setHandler(command_ids::EditPreferencesGuides,
                          [this]() { showPreferences(PreferencesDialog::kGuides); });
    registry_->setHandler(command_ids::EditPreferencesType,
                          [this]() { showPreferences(PreferencesDialog::kType); });
    registry_->setHandler(command_ids::EditPreferencesAgentControl,
                          [this]() { showPreferences(PreferencesDialog::kAgentControl); });
    // Only an interactive window has one, parented to the frame. A recursive
    // search walks every widget on each command refresh (#1151).
    registry_->setEnabledProvider(command_ids::EditPreferencesAgentControl, [this]() {
        return findChild<AgentControl*>(QString(), Qt::FindDirectChildrenOnly) != nullptr;
    });
}

void PicturaMainWindow::showPreferences(const QString& page)
{
    if (!preferencesDialog_) {
        preferencesDialog_ = new PreferencesDialog(this);
        preferencesDialog_->setShiftKeyForToolSwitch(useShiftKeyForToolSwitch_);
        preferencesDialog_->setOpenDocumentsAsTabs(openDocumentsAsTabs_);
        preferencesDialog_->setFloatingDocumentDocking(floatingDocumentDocking_);
        preferencesDialog_->setZoomResizesWindows(zoomResizesWindows_);
        preferencesDialog_->setAutoCollapseIconic(panelColumn_->autoCollapseIconic());
        preferencesDialog_->setAutoShowHidden(panelColumn_->autoShowHidden());
        preferencesDialog_->setBrightnessLevel(brightnessLevel_);
        preferencesDialog_->setThemeFamily(themeFamily_);
        preferencesDialog_->setUiScalePercent(uiScalePercent_);
        preferencesDialog_->setPickerChoice(pickerChoice_);
        preferencesDialog_->setGpuCompute(gpuCompute_);
        preferencesDialog_->setMemoryUsagePercent(memory_usage_percent());
        preferencesDialog_->setGpuComputeEnabled(gpuCandidatesAvailable_);
        preferencesDialog_->setTypePlaceholderText(!tools_ || tools_->typePlaceholderText());
        preferencesDialog_->setTypeShowAsianOptions(typeShowAsianOptions_);
        preferencesDialog_->setTypeMiddleEastern(typeMiddleEastern_);
        preferencesDialog_->setTypeMissingGlyphProtection(typeMissingGlyphProtection_);
        if (auto* agent = findChild<AgentControl*>(QString(), Qt::FindDirectChildrenOnly)) {
            preferencesDialog_->addPage(PreferencesDialog::kAgentControl,
                                        new AgentControlPage(agent));
        }
        preferencesDialog_->addPage(PreferencesDialog::kUpdates,
                                    makeUpdatePreferencesPage(*this, preferencesDialog_));
        connectRecoveryPreferences(preferencesDialog_);
        connectGuidePreferences(preferencesDialog_);
        connect(preferencesDialog_, &PreferencesDialog::exportSettingsRequested, this, [this]() {
            if (actionsEngine_) {
                actionsEngine_->saveLibrary();
            }
            saveSession();
            QString selected;
            const QStringList filters{tr("Pictura Settings (*.json)"), tr("All Files (*)")};
            QString directory = rememberedFileDialogDirectory();
            if (directory.isEmpty()) {
                directory = QStandardPaths::writableLocation(
                    QStandardPaths::DocumentsLocation);
            }
            if (directory.isEmpty()) {
                directory = QDir::homePath();
            }
            QString path = getSaveFileName(this, tr("Export Settings"),
                                           directory + QStringLiteral("/pictura.json"),
                                           filters, &selected);
            if (path.isEmpty()) {
                return;
            }
            if (!path.endsWith(QStringLiteral(".json"), Qt::CaseInsensitive)) {
                path += QStringLiteral(".json");
            }
            QString error;
            if (!pictura::exportSettingsFile(path, &error)) {
                QMessageBox::warning(
                    this, tr("Export Settings"),
                    error.isEmpty() ? tr("The settings could not be exported.") : error);
            }
        });
        connect(preferencesDialog_, &PreferencesDialog::importSettingsRequested, this, [this]() {
            if (actionsEngine_) {
                actionsEngine_->saveLibrary();
            }
            saveSession();
            const QStringList filters{tr("Pictura Settings (*.json)"), tr("All Files (*)")};
            const QString path =
                getOpenFileName(this, tr("Import Settings"), QString(), filters);
            if (path.isEmpty()) {
                return;
            }
            QString error;
            if (!pictura::importSettingsFile(path, this, &error)) {
                QMessageBox::warning(
                    this, tr("Import Settings"),
                    error.isEmpty() ? tr("The settings could not be imported.") : error);
            }
        });
        connect(preferencesDialog_, &PreferencesDialog::useShiftKeyForToolSwitchChanged,
                this, [this](bool on) {
                    useShiftKeyForToolSwitch_ = on;
                    if (toolbox_) {
                        toolbox_->setShiftKeyForToolSwitch(on);
                    }
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::openDocumentsAsTabsChanged, this,
                [this](bool on) {
                    openDocumentsAsTabs_ = on;
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::floatingDocumentDockingChanged, this,
                [this](bool on) {
                    floatingDocumentDocking_ = on;
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::zoomResizesWindowsChanged, this,
                [this](bool on) {
                    zoomResizesWindows_ = on;
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::autoCollapseIconicChanged, this,
                [this](bool on) { panelColumn_->setAutoCollapseIconic(on); });
        connect(preferencesDialog_, &PreferencesDialog::autoShowHiddenChanged, this,
                [this](bool on) { panelColumn_->setAutoShowHidden(on); });
        connect(preferencesDialog_, &PreferencesDialog::brightnessLevelChanged, this,
                [this](int level) {
                    setBrightnessLevel(level);
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::themeFamilyChanged, this,
                [this](int family) { setThemeFamily(family); });
        connect(preferencesDialog_, &PreferencesDialog::uiScalePercentChanged, this,
                [this](int percent) { setUiScalePercent(percent); });
        connect(preferencesDialog_, &PreferencesDialog::pickerChoiceChanged, this,
                [this](int choice) {
                    pickerChoice_ = choice == 1 ? 1 : 0;
                    ColorPickerDialog::setPickerPreference(pickerChoice_);
                    for (const DocEntry* entry : docs_) {
                        if (entry && entry->view) {
                            pictura::set_picker_choice(*entry->view, pickerChoice_);
                        }
                    }
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::gpuComputeChanged, this,
                &PicturaMainWindow::applyGpuComputePreference);
        connect(preferencesDialog_, &PreferencesDialog::memoryUsagePercentChanged, this,
                [this](int percent) {
                    set_memory_usage_percent(percent);
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::typePlaceholderTextChanged, this,
                [this](bool on) {
                    if (tools_) {
                        tools_->setTypePlaceholderText(on);
                    }
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::typeShowAsianOptionsChanged, this,
                [this](bool on) {
                    typeShowAsianOptions_ = on;
                    if (characterPanel_) {
                        characterPanel_->setShowAsianOptions(on);
                    }
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::typeMiddleEasternChanged, this,
                [this](bool on) {
                    typeMiddleEastern_ = on;
                    if (paragraphPanel_) {
                        paragraphPanel_->setMiddleEastern(on);
                    }
                    saveSession();
                });
        connect(preferencesDialog_, &PreferencesDialog::typeMissingGlyphProtectionChanged, this,
                [this](bool on) {
                    typeMissingGlyphProtection_ = on;
                    pictura::type_api::setMissingGlyphProtection(on);
                    for (const DocEntry* entry : docs_) {
                        if (entry && entry->view) {
                            pictura::type_api::refreshMissingGlyphProtection(*entry->view);
                        }
                    }
                    saveSession();
                });
    }
    preferencesDialog_->openOn(page);
}

} // namespace pictura
