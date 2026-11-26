#include "preferences_dialog.h"
#include "localization.h"

#include <QtCore/QCoreApplication>
#include "preset_store.h"
#include <QtWidgets/QMessageBox>

#include "canvas_ruler.h"
#include "panels/color_swatch_widget.h"
#include "panels/numeric_field.h"
#include "preferences.h"
#include "theme.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

const QString PreferencesDialog::kGeneral = QStringLiteral("General");
const QString PreferencesDialog::kInterface = QStringLiteral("Interface");
const QString PreferencesDialog::kFileHandling = QStringLiteral("File Handling");
const QString PreferencesDialog::kPerformance = QStringLiteral("Performance");
const QString PreferencesDialog::kUnits = QStringLiteral("Units & Rulers");
const QString PreferencesDialog::kGuides = QStringLiteral("Guides, Grid, & Slices");
const QString PreferencesDialog::kType = QStringLiteral("Type");
const QString PreferencesDialog::kAgentControl =
    QString::fromLatin1(QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Agent Control"));
const QString PreferencesDialog::kUpdates =
    QString::fromLatin1(QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Updates"));

namespace {

// The CS6 pane list; the real pages are the ones present in `realPages_`.
const char* const kPaneNames[] = {
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "General"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Interface"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "File Handling"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Performance"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Cursors"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Transparency & Gamut"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Units & Rulers"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Guides, Grid, & Slices"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Plug-ins"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Type"),
    QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "3D"),
};

struct NamedColor {
    const char* name;
    QColor color;
};

// CS6's guide colour presets. Cyan and Magenta were sampled from a CS6
// Preferences screenshot (#294) and Light Blue from its slice swatch.
// ponytail: the other presets are approximated in the same 74/255 family.
const NamedColor kGuideColors[] = {
    {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Light Blue"), QColor(74, 113, 255)}, {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Light Red"), QColor(255, 74, 74)},
    {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Green"), QColor(74, 255, 74)},       {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Medium Blue"), QColor(74, 74, 255)},
    {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Yellow"), QColor(255, 255, 74)},     {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Magenta"), QColor(255, 74, 255)},
    {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Cyan"), QColor(74, 255, 255)},       {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Light Gray"), QColor(192, 192, 192)},
    {QT_TRANSLATE_NOOP("pictura::PreferencesDialog", "Black"), QColor(0, 0, 0)},
};

QIcon swatchIcon(const QColor& color)
{
    QPixmap pixmap(12, 12);
    pixmap.fill(color);
    return QIcon(pixmap);
}

QComboBox* colorCombo(const QString& objectName, QWidget* parent)
{
    auto* combo = new QComboBox(parent);
    combo->setObjectName(objectName);
    for (const NamedColor& preset : kGuideColors) {
        combo->addItem(swatchIcon(preset.color),
                       QCoreApplication::translate("pictura::PreferencesDialog", preset.name),
                       preset.color);
    }
    combo->addItem(QCoreApplication::translate("pictura::PreferencesDialog", "Custom…"));
    return combo;
}

ColorSwatchWidget* swatch(const QColor& color, QWidget* parent)
{
    auto* widget = new ColorSwatchWidget(parent);
    widget->setFixedSize(40, 40);
    widget->setFocusPolicy(Qt::NoFocus);
    widget->setColor(color);
    return widget;
}

} // namespace

PreferencesDialog::PreferencesDialog(QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("preferencesDialog"));
    setWindowTitle(tr("Preferences"));
    setWindowModality(Qt::NonModal);

    auto* layout = new QHBoxLayout(this);
    pageList_ = new QListWidget(this);
    pageList_->setObjectName(QStringLiteral("preferencesPages"));
    pageList_->setFixedWidth(160);
    layout->addWidget(pageList_);

    stack_ = new QStackedWidget(this);
    stack_->setObjectName(QStringLiteral("preferencesStack"));
    layout->addWidget(stack_, 1);

    realPages_ = {kGeneral, kInterface, kFileHandling, kPerformance, kUnits, kGuides, kType};

    // General: the app's theme brightness and family, and Zoom Resizes Windows.
    auto* generalPage = new QWidget(this);
    generalPage->setObjectName(QStringLiteral("preferencesGeneralPage"));
    auto* generalForm = new QFormLayout(generalPage);
    NumericFieldConfig brightnessConfig;
    brightnessConfig.minimum = 0;
    brightnessConfig.maximum = Theme::kLevelCount - 1;
    brightnessConfig.decimals = 0;
    brightnessConfig.objectName = QStringLiteral("preferencesBrightness");
    brightness_ = new NumericField(QString(), brightnessConfig, generalPage);
    brightness_->setValue(Theme::kDefaultLevel);
    generalForm->addRow(tr("Brightness"), brightness_);

    themeFamily_ = new QComboBox(generalPage);
    themeFamily_->setObjectName(QStringLiteral("preferencesThemeFamily"));
    themeFamily_->addItem(tr("Dark"), 0);
    themeFamily_->addItem(tr("Light"), 1);
    generalForm->addRow(tr("Theme"), themeFamily_);
    makeCheckbox(QStringLiteral("zoomResizesWindows"), tr("Zoom Resizes Windows"),
                 generalPage);
    addHistoryLogControls(generalForm, generalPage);

    pickerChoice_ = new QComboBox(generalPage);
    pickerChoice_->setObjectName(QStringLiteral("preferencesPickerChoice"));
    pickerChoice_->addItem(tr("Adobe"), 0);
    pickerChoice_->addItem(tr("System"), 1);
    generalForm->addRow(tr("Color Picker"), pickerChoice_);
    connect(pickerChoice_, &QComboBox::currentIndexChanged, this, [this](int index) {
        emit pickerChoiceChanged(pickerChoice_->itemData(index).toInt());
    });

    auto* settingsRow = new QWidget(generalPage);
    auto* settingsLayout = new QHBoxLayout(settingsRow);
    settingsLayout->setContentsMargins(0, 0, 0, 0);
    auto* exportSettings = new QPushButton(tr("Export Settings…"), settingsRow);
    exportSettings->setObjectName(QStringLiteral("preferencesExportSettings"));
    auto* importSettings = new QPushButton(tr("Import Settings…"), settingsRow);
    importSettings->setObjectName(QStringLiteral("preferencesImportSettings"));
    settingsLayout->addWidget(exportSettings);
    settingsLayout->addWidget(importSettings);
    auto* resetPresets = new QPushButton(tr("Reset Preset Lists…"), settingsRow);
    resetPresets->setObjectName(QStringLiteral("preferencesResetPresetLists"));
    settingsLayout->addWidget(resetPresets);
    connect(resetPresets, &QPushButton::clicked, this, [this] {
        QMessageBox confirm(QMessageBox::Question, tr("Reset Preset Lists"),
            tr("Restore the default preset lists? This removes edits to current lists. Saved libraries are kept."),
            QMessageBox::Yes | QMessageBox::No, this);
        confirm.setTextFormat(Qt::PlainText);
        if (confirm.exec() != QMessageBox::Yes) {
            return;
        }
        QString error;
        if (!PresetStore::current().resetAll(&error)) { QMessageBox::warning(this, tr("Reset Preset Lists"), error); }
    });
    settingsLayout->addStretch(1);
    generalForm->addRow(tr("Settings"), settingsRow);
    connect(exportSettings, &QPushButton::clicked, this,
            &PreferencesDialog::exportSettingsRequested);
    connect(importSettings, &QPushButton::clicked, this,
            &PreferencesDialog::importSettingsRequested);
    stack_->addWidget(generalPage);

    auto* interfacePage = new QWidget(this);
    interfacePage->setObjectName(QStringLiteral("preferencesInterfacePage"));
    auto* interfaceLayout = new QVBoxLayout(interfacePage);
    makeCheckbox(QStringLiteral("useShiftKeyForToolSwitch"),
                 tr("Use Shift Key For Tool Switch"), interfacePage);
    makeCheckbox(QStringLiteral("autoCollapseIconic"), tr("Auto-Collapse Iconic Panels"),
                 interfacePage);
    makeCheckbox(QStringLiteral("autoShowHidden"), tr("Auto-Show Hidden Panels"),
                 interfacePage);
    makeCheckbox(QStringLiteral("openDocumentsAsTabs"), tr("Open Documents As Tabs"),
                 interfacePage)
        ->setChecked(true);
    makeCheckbox(QStringLiteral("enableFloatingDocumentDocking"),
                 tr("Enable Floating Document Window Docking"), interfacePage)
        ->setChecked(true);
    auto* uiScaleRow = new QWidget(interfacePage);
    auto* uiScaleLayout = new QHBoxLayout(uiScaleRow);
    uiScaleLayout->addWidget(new QLabel(tr("UI Size"), uiScaleRow));
    uiScale_ = new QComboBox(uiScaleRow);
    uiScale_->setObjectName(QStringLiteral("preferencesUiScale"));
    for (int percent : Theme::uiScaleChoices()) {
        uiScale_->addItem(tr("%1%").arg(percent), percent);
    }
    uiScale_->setCurrentIndex(uiScale_->findData(Theme::kDefaultUiScalePercent));
    uiScaleLayout->addWidget(uiScale_);
    uiScaleLayout->addStretch(1);
    interfaceLayout->addWidget(uiScaleRow);

    auto* languageRow = new QWidget(interfacePage);
    auto* languageLayout = new QHBoxLayout(languageRow);
    auto* languageLabel = new QLabel(tr("Language"), languageRow);
    language_ = new QComboBox(languageRow);
    language_->setObjectName(QStringLiteral("preferencesLanguage"));
    languageLabel->setBuddy(language_);
    auto* importTranslation = new QPushButton(tr("Import Translation…"), languageRow);
    importTranslation->setObjectName(QStringLiteral("preferencesImportTranslation"));
    languageRestart_ = new QPushButton(tr("Restart Pictura"), languageRow);
    languageRestart_->setObjectName(QStringLiteral("preferencesRestartLanguage"));
    languageRestart_->hide();
    languageLayout->addWidget(languageLabel);
    languageLayout->addWidget(language_);
    languageLayout->addWidget(languageRestart_);
    languageLayout->addWidget(importTranslation);
    languageLayout->addStretch(1);
    interfaceLayout->addWidget(languageRow);
    languageStatus_ = new QLabel(interfacePage);
    languageStatus_->setObjectName(QStringLiteral("preferencesLanguageStatus"));
    languageStatus_->setWordWrap(true);
    languageStatus_->hide();
    interfaceLayout->addWidget(languageStatus_);
    connect(language_, &QComboBox::activated, this, [this](int index) {
        const QString selection = language_->itemData(index).toString();
        emit languageSelected(selection);
    });
    connect(importTranslation, &QPushButton::clicked, this,
            &PreferencesDialog::importTranslationRequested);
    connect(languageRestart_, &QPushButton::clicked, this,
            &PreferencesDialog::languageRestartRequested);
    interfaceLayout->addStretch(1);
    stack_->addWidget(interfacePage);

    // File Handling: CS6's crash-recovery autosave, a checkbox and its interval
    // on one row.
    auto* fileHandlingPage = new QWidget(this);
    fileHandlingPage->setObjectName(QStringLiteral("preferencesFileHandlingPage"));
    auto* fileHandlingLayout = new QVBoxLayout(fileHandlingPage);
    makeCheckbox(QStringLiteral("saveInBackground"), tr("Save in Background"), fileHandlingPage)
        ->setChecked(true);
    auto* autoSaveRow = new QWidget(fileHandlingPage);
    auto* autoSaveLayout = new QHBoxLayout(autoSaveRow);
    autoSaveLayout->setContentsMargins(0, 0, 0, 0);
    makeCheckbox(QStringLiteral("autoSaveRecovery"),
                 tr("Automatically Save Recovery Information Every:"), autoSaveRow);
    autoSaveMinutes_ = new QComboBox(autoSaveRow);
    autoSaveMinutes_->setObjectName(QStringLiteral("preferencesAutoSaveMinutes"));
    for (int minutes : kAutoSaveMinuteChoices) {
        autoSaveMinutes_->addItem(minutes == 60 ? tr("1 Hour") : tr("%1 Minutes").arg(minutes),
                                  minutes);
    }
    autoSaveLayout->addWidget(autoSaveMinutes_);
    autoSaveLayout->addStretch(1);
    fileHandlingLayout->addWidget(autoSaveRow);
    auto* compatibilityRow = new QHBoxLayout;
    auto* compatibilityLabel = new QLabel(tr("Maximize PSD and PSB File Compatibility:"),
                                          fileHandlingPage);
    nativeSaveCompatibility_ = new QComboBox(fileHandlingPage);
    nativeSaveCompatibility_->setObjectName(QStringLiteral("preferencesNativeSaveCompatibility"));
    nativeSaveCompatibility_->addItems({tr("Always"), tr("Ask"), tr("Never")});
    compatibilityLabel->setBuddy(nativeSaveCompatibility_);
    compatibilityRow->addWidget(compatibilityLabel);
    compatibilityRow->addWidget(nativeSaveCompatibility_);
    compatibilityRow->addStretch();
    fileHandlingLayout->addLayout(compatibilityRow);
    connect(nativeSaveCompatibility_, &QComboBox::currentIndexChanged,
            this, &PreferencesDialog::nativeSaveCompatibilityChanged);
    // ponytail: CS6 keeps this in the Camera Raw Preferences dialog (#633);
    // until that exists it sits with the other file-handling choices.
    auto* rawStoreRow = new QHBoxLayout;
    auto* rawStoreLabel = new QLabel(tr("Save Camera Raw Settings In:"), fileHandlingPage);
    rawSettingsStore_ = new QComboBox(fileHandlingPage);
    rawSettingsStore_->setObjectName(QStringLiteral("preferencesRawSettingsStore"));
    rawSettingsStore_->addItems({tr("Camera Raw Database"), tr("Sidecar \".XMP\" Files")});
    rawStoreLabel->setBuddy(rawSettingsStore_);
    rawStoreRow->addWidget(rawStoreLabel);
    rawStoreRow->addWidget(rawSettingsStore_);
    rawStoreRow->addStretch();
    fileHandlingLayout->addLayout(rawStoreRow);
    connect(rawSettingsStore_, &QComboBox::currentIndexChanged,
            this, &PreferencesDialog::rawSettingsStoreChanged);
    fileHandlingLayout->addStretch(1);
    stack_->addWidget(fileHandlingPage);
    setAutoSaveRecovery(true, 10);
    connect(autoSaveMinutes_, &QComboBox::currentIndexChanged, this, [this] {
        emit autoSaveRecoveryChanged(checkbox(QStringLiteral("autoSaveRecovery"))->isChecked(),
                                     autoSaveMinutes_->currentData().toInt());
    });

    stack_->addWidget(buildPerformancePage());
    stack_->addWidget(buildUnitsPage());
    stack_->addWidget(buildGuidesPage());

    // Type: the CS6 type options and glyph protection, plus the Photoshop CC
    // placeholder fill.
    auto* typePage = new QWidget(this);
    typePage->setObjectName(QStringLiteral("preferencesTypePage"));
    auto* typeLayout = new QVBoxLayout(typePage);
    makeCheckbox(QStringLiteral("showAsianTextOptions"),
                 tr("Show Asian Text Options"), typePage);
    makeCheckbox(QStringLiteral("middleEastern"), tr("Middle Eastern"), typePage);
    makeCheckbox(QStringLiteral("missingGlyphProtection"),
                 tr("Enable Missing Glyph Protection"), typePage)
        ->setChecked(true);
    makeCheckbox(QStringLiteral("fillPlaceholderText"),
                 tr("Fill New Type Layers With Placeholder Text"), typePage)
        ->setChecked(true);
    typeLayout->addStretch(1);
    stack_->addWidget(typePage);

    // List rows carry their pane name in Qt::UserRole: the CS6 pane order does
    // not match the stack's real-page order, so the row maps by name.
    for (const char* name : kPaneNames) {
        const QString pageName = QString::fromUtf8(name);
        auto* item = new QListWidgetItem(tr(name), pageList_);
        item->setData(Qt::UserRole, pageName);
        if (!realPages_.contains(pageName)) {
            item->setFlags(item->flags() & ~Qt::ItemIsEnabled);
        }
    }
    connect(pageList_, &QListWidget::currentRowChanged, this, [this](int row) {
        const QListWidgetItem* item = pageList_->item(row);
        if (!item) {
            return;
        }
        const int index = realPages_.indexOf(item->data(Qt::UserRole).toString());
        if (index >= 0) {
            stack_->setCurrentIndex(index);
        }
    });
    connect(brightness_, &NumericField::valueChanged, this,
            [this](double level) { emit brightnessLevelChanged(qRound(level)); });
    connect(themeFamily_, &QComboBox::currentIndexChanged, this,
            [this](int index) { emit themeFamilyChanged(themeFamily_->itemData(index).toInt()); });
    connect(uiScale_, &QComboBox::currentIndexChanged, this,
            [this]() { emit uiScalePercentChanged(uiScale_->currentData().toInt()); });
    pageList_->setCurrentRow(0);
    for (auto* label : findChildren<QLabel*>()) {
        if (label->textFormat() == Qt::AutoText) {
            label->setTextFormat(Qt::PlainText);
        }
    }
}

void PreferencesDialog::setLanguageChoices(const QString& savedChoice)
{
    language_->clear();
    for (const auto& choice : availableUiLanguages()) {
        language_->addItem(choice.label, choice.selection);
    }
    language_->setCurrentIndex(qMax(0, language_->findData(savedChoice)));
}

void PreferencesDialog::showLanguageStatus(const QString& message)
{
    languageStatus_->setText(message);
    languageStatus_->setVisible(!message.isEmpty());
}

void PreferencesDialog::setLanguageRestartPending(bool pending)
{
    languageRestart_->setVisible(pending);
}

void PreferencesDialog::addPage(const QString& name, QWidget* page)
{
    realPages_.append(name);
    stack_->addWidget(page);
    const QString title = tr(name.toUtf8().constData());
    (new QListWidgetItem(title, pageList_))->setData(Qt::UserRole, name);
}

QCheckBox* PreferencesDialog::makeCheckbox(const QString& key, const QString& label,
                                           QWidget* page)
{
    auto* box = new QCheckBox(label, page);
    box->setObjectName(QStringLiteral("preferences_") + key);
    page->layout()->addWidget(box);
    checkboxes_.insert(key, box);
    connect(box, &QCheckBox::toggled, this, [this, key](bool on) {
        if (key == QStringLiteral("useShiftKeyForToolSwitch")) {
            emit useShiftKeyForToolSwitchChanged(on);
        } else if (key == QStringLiteral("openDocumentsAsTabs")) {
            emit openDocumentsAsTabsChanged(on);
        } else if (key == QStringLiteral("enableFloatingDocumentDocking")) {
            emit floatingDocumentDockingChanged(on);
        } else if (key == QStringLiteral("zoomResizesWindows")) {
            emit zoomResizesWindowsChanged(on);
        } else if (key == QStringLiteral("autoCollapseIconic")) {
            emit autoCollapseIconicChanged(on);
        } else if (key == QStringLiteral("autoShowHidden")) {
            emit autoShowHiddenChanged(on);
        } else if (key == QStringLiteral("useGpuCompute")) {
            emit gpuComputeChanged(on);
        } else if (key == QStringLiteral("fillPlaceholderText")) {
            emit typePlaceholderTextChanged(on);
        } else if (key == QStringLiteral("showAsianTextOptions")) {
            emit typeShowAsianOptionsChanged(on);
        } else if (key == QStringLiteral("middleEastern")) {
            emit typeMiddleEasternChanged(on);
        } else if (key == QStringLiteral("missingGlyphProtection")) {
            emit typeMissingGlyphProtectionChanged(on);
        } else if (key == QStringLiteral("saveInBackground")) {
            emit saveInBackgroundChanged(on);
        } else if (key == QStringLiteral("autoSaveRecovery")) {
            autoSaveMinutes_->setEnabled(on);
            emit autoSaveRecoveryChanged(on, autoSaveMinutes_->currentData().toInt());
        }
    });
    return box;
}

QCheckBox* PreferencesDialog::checkbox(const QString& key) const
{
    return checkboxes_.value(key, nullptr);
}

void PreferencesDialog::setShiftKeyForToolSwitch(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("useShiftKeyForToolSwitch"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setOpenDocumentsAsTabs(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("openDocumentsAsTabs"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setFloatingDocumentDocking(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("enableFloatingDocumentDocking"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setZoomResizesWindows(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("zoomResizesWindows"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setAutoCollapseIconic(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("autoCollapseIconic"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setAutoShowHidden(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("autoShowHidden"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setBrightnessLevel(int level)
{
    brightness_->setValue(level);
}

void PreferencesDialog::setPickerChoice(int choice)
{
    if (!pickerChoice_) {
        return;
    }
    const int index = pickerChoice_->findData(choice == 1 ? 1 : 0);
    if (index >= 0) {
        pickerChoice_->setCurrentIndex(index);
    }
}

int PreferencesDialog::pickerChoice() const
{
    return pickerChoice_ ? pickerChoice_->currentData().toInt() : 0;
}

void PreferencesDialog::setThemeFamily(int family)
{
    if (themeFamily_) {
        const int index = themeFamily_->findData(family == 1 ? 1 : 0);
        if (index >= 0) {
            themeFamily_->setCurrentIndex(index);
        }
    }
}

void PreferencesDialog::setUiScalePercent(int percent)
{
    if (!uiScale_) {
        return;
    }
    const int index = uiScale_->findData(percent);
    if (index >= 0) {
        const QSignalBlocker blocker(uiScale_);
        uiScale_->setCurrentIndex(index);
    }
}

int PreferencesDialog::uiScalePercentForTest() const
{
    return uiScale_ ? uiScale_->currentData().toInt() : 0;
}

void PreferencesDialog::setTypePlaceholderText(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("fillPlaceholderText"))) {
        box->setChecked(on);
    }
}

void PreferencesDialog::setTypeShowAsianOptions(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("showAsianTextOptions"))) {
        const QSignalBlocker block(box);
        box->setChecked(on);
    }
}

void PreferencesDialog::setTypeMiddleEastern(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("middleEastern"))) {
        const QSignalBlocker block(box);
        box->setChecked(on);
    }
}

void PreferencesDialog::setTypeMissingGlyphProtection(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("missingGlyphProtection"))) {
        const QSignalBlocker block(box);
        box->setChecked(on);
    }
}

void PreferencesDialog::setGpuCompute(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("useGpuCompute"))) {
        const QSignalBlocker block(box);
        box->setChecked(on);
    }
}

void PreferencesDialog::setGpuComputeEnabled(bool on)
{
    if (QCheckBox* box = checkbox(QStringLiteral("useGpuCompute"))) {
        box->setEnabled(on);
    }
}

void PreferencesDialog::setNativeSaveCompatibility(int choice)
{
    const QSignalBlocker blocker(nativeSaveCompatibility_);
    nativeSaveCompatibility_->setCurrentIndex(choice);
}

void PreferencesDialog::setRawSettingsStore(int store)
{
    const QSignalBlocker blocker(rawSettingsStore_);
    rawSettingsStore_->setCurrentIndex(store == 1 ? 1 : 0);
}

void PreferencesDialog::setSaveInBackground(bool on)
{
    QCheckBox* box = checkbox(QStringLiteral("saveInBackground"));
    const QSignalBlocker blocker(box);
    box->setChecked(on);
}

void PreferencesDialog::setAutoSaveRecovery(bool on, int minutes)
{
    QCheckBox* box = checkbox(QStringLiteral("autoSaveRecovery"));
    const QSignalBlocker blockBox(box);
    const QSignalBlocker blockMinutes(autoSaveMinutes_);
    box->setChecked(on);
    autoSaveMinutes_->setEnabled(on);
    const int index = autoSaveMinutes_->findData(minutes);
    autoSaveMinutes_->setCurrentIndex(index >= 0 ? index : autoSaveMinutes_->findData(10));
}

void PreferencesDialog::showPage(const QString& page)
{
    const int index = realPages_.indexOf(page);
    if (index < 0) {
        return;
    }
    for (int row = 0; row < pageList_->count(); ++row) {
        if (pageList_->item(row)->data(Qt::UserRole).toString() == page) {
            pageList_->setCurrentRow(row);
            break;
        }
    }
    stack_->setCurrentIndex(index);
}

void PreferencesDialog::openOn(const QString& page)
{
    showPage(page);
    show();
    raise();
    activateWindow();
}

QStringList PreferencesDialog::pagesForTest() const
{
    return realPages_;
}

QString PreferencesDialog::currentPageForTest() const
{
    return realPages_.value(stack_->currentIndex());
}

bool PreferencesDialog::setCheckboxForTest(const QString& key, bool on)
{
    QCheckBox* box = checkbox(key);
    if (!box) {
        return false;
    }
    box->setChecked(on);
    return true;
}

bool PreferencesDialog::checkboxForTest(const QString& key) const
{
    QCheckBox* box = checkbox(key);
    return box && box->isChecked();
}

// Units & Rulers: ruler units, Point/Pica Size, and Column Size are live.
// Type and New Document Preset Resolutions show disabled defaults.
QWidget* PreferencesDialog::buildUnitsPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("preferencesUnitsPage"));
    auto* layout = new QGridLayout(page);

    auto* units = new QGroupBox(tr("Units"), page);
    auto* unitsForm = new QFormLayout(units);
    rulerUnit_ = new QComboBox(units);
    rulerUnit_->setObjectName(QStringLiteral("preferencesRulerUnits"));
    for (int i = 0; i < kRulerUnitCount; ++i) {
        rulerUnit_->addItem(tr(rulerUnitName(RulerUnit(i)).toUtf8().constData()));
    }
    rulerUnit_->setCurrentIndex(int(RulerUnit::Inches));
    auto* typeUnit = new QComboBox(units);
    typeUnit->setObjectName(QStringLiteral("preferencesTypeUnit"));
    typeUnit->addItems({tr("Pixels"), tr("Points"), tr("Millimeters")});
    typeUnit->setCurrentIndex(1);
    typeUnit->setEnabled(false);
    unitsForm->addRow(tr("Rulers:"), rulerUnit_);
    unitsForm->addRow(tr("Type:"), typeUnit);
    layout->addWidget(units, 0, 0);

    const auto valueAndUnit = [](QWidget* parent, const QString& value,
                                 const QStringList& units) {
        auto* row = new QWidget(parent);
        auto* rowLayout = new QHBoxLayout(row);
        rowLayout->setContentsMargins(0, 0, 0, 0);
        rowLayout->addWidget(new QLabel(value, row));
        auto* unit = new QComboBox(row);
        unit->addItems(units);
        rowLayout->addWidget(unit);
        return row;
    };
    auto* columns = new QGroupBox(tr("Column Size"), page);
    auto* columnsForm = new QFormLayout(columns);
    const auto columnField = [this, columns, columnsForm](
                                 const QString& label, const QString& name, double* inches,
                                 QDoubleSpinBox*& value, QComboBox*& unit) {
        auto* row = new QHBoxLayout;
        value = new QDoubleSpinBox(columns);
        value->setObjectName(name);
        value->setDecimals(6);
        value->setKeyboardTracking(false);
        unit = new QComboBox(columns);
        unit->setObjectName(name + QStringLiteral("Unit"));
        for (const auto choice : {RulerUnit::Points, RulerUnit::Inches, RulerUnit::Centimeters,
                                  RulerUnit::Millimeters, RulerUnit::Picas}) {
            unit->addItem(tr(rulerUnitName(choice).toUtf8().constData()), int(choice));
        }
        row->addWidget(value);
        row->addWidget(unit);
        columnsForm->addRow(label, row);
        connect(unit, &QComboBox::currentIndexChanged,
                this, &PreferencesDialog::syncColumnSizeFields);
        connect(value, &QDoubleSpinBox::valueChanged, this, [this, inches, unit](double amount) {
            const auto choice = RulerUnit(unit->currentData().toInt());
            *inches = amount * pixelsPerRulerUnit(choice, 1.0, traditionalPoints_->isChecked(), 0);
            emit columnSizeChanged(columnWidthInches_, columnGutterInches_);
        });
    };
    columnField(tr("Width:"), QStringLiteral("preferencesColumnWidth"), &columnWidthInches_,
                columnWidth_, columnWidthUnit_);
    columnField(tr("Gutter:"), QStringLiteral("preferencesColumnGutter"), &columnGutterInches_,
                columnGutter_, columnGutterUnit_);
    layout->addWidget(columns, 1, 0);

    const QStringList resolutionUnits = {tr("Pixels/Inch"), tr("Pixels/Centimeter")};
    auto* resolutions = new QGroupBox(tr("New Document Preset Resolutions"), page);
    resolutions->setObjectName(QStringLiteral("preferencesNewDocumentPresetResolutions"));
    auto* resolutionsForm = new QFormLayout(resolutions);
    resolutionsForm->addRow(tr("Print Resolution:"),
                            valueAndUnit(resolutions, QStringLiteral("300"), resolutionUnits));
    resolutionsForm->addRow(tr("Screen Resolution:"),
                            valueAndUnit(resolutions, QStringLiteral("72"), resolutionUnits));
    resolutions->setEnabled(false);
    layout->addWidget(resolutions, 0, 1);

    auto* pointSize = new QGroupBox(tr("Point/Pica Size"), page);
    auto* pointLayout = new QVBoxLayout(pointSize);
    postScriptPoints_ = new QRadioButton(tr("PostScript (72 points/inch)"), pointSize);
    postScriptPoints_->setObjectName(QStringLiteral("preferencesPostScriptPoints"));
    traditionalPoints_ = new QRadioButton(tr("Traditional (72.27 points/inch)"), pointSize);
    traditionalPoints_->setObjectName(QStringLiteral("preferencesTraditionalPoints"));
    postScriptPoints_->setChecked(true);
    pointLayout->addWidget(postScriptPoints_);
    pointLayout->addWidget(traditionalPoints_);
    layout->addWidget(pointSize, 1, 1);
    layout->setRowStretch(2, 1);

    connect(rulerUnit_, &QComboBox::activated, this,
            [this](int index) { emit rulerUnitChanged(index); });
    connect(traditionalPoints_, &QRadioButton::toggled, this, [this](bool on) {
        syncColumnSizeFields();
        emit traditionalPointsChanged(on);
    });
    syncColumnSizeFields();
    return page;
}

void PreferencesDialog::setRulerUnit(int unit)
{
    const QSignalBlocker block(rulerUnit_);
    rulerUnit_->setCurrentIndex(unit);
}

void PreferencesDialog::setTraditionalPoints(bool on)
{
    const QSignalBlocker blockTraditional(traditionalPoints_);
    const QSignalBlocker blockPostScript(postScriptPoints_);
    (on ? traditionalPoints_ : postScriptPoints_)->setChecked(true);
    syncColumnSizeFields();
}

void PreferencesDialog::setColumnSize(double widthInches, double gutterInches)
{
    columnWidthInches_ = widthInches;
    columnGutterInches_ = gutterInches;
    syncColumnSizeFields();
}

void PreferencesDialog::syncColumnSizeFields()
{
    const bool traditional = traditionalPoints_->isChecked();
    const auto sync = [traditional](QDoubleSpinBox* field, QComboBox* unit,
                                    double inches, double minInches) {
        const auto choice = RulerUnit(unit->currentData().toInt());
        const double inchesPerUnit = pixelsPerRulerUnit(choice, 1.0, traditional, 0);
        const QSignalBlocker blocked(field);
        field->setRange(minInches / inchesPerUnit, 100000.0 / inchesPerUnit);
        field->setValue(inches / inchesPerUnit);
    };
    sync(columnWidth_, columnWidthUnit_, columnWidthInches_, 0.0001);
    sync(columnGutter_, columnGutterUnit_, columnGutterInches_, 0.0);
}

// Guides and Grid are editable. Smart Guides and Slices show their defaults.
QWidget* PreferencesDialog::buildGuidesPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("preferencesGuidesPage"));
    auto* layout = new QVBoxLayout(page);

    auto* guides = new QGroupBox(tr("Guides"), page);
    auto* guidesGrid = new QGridLayout(guides);
    guideColor_ = colorCombo(QStringLiteral("preferencesGuideColor"), guides);
    guideStyle_ = new QComboBox(guides);
    guideStyle_->setObjectName(QStringLiteral("preferencesGuideStyle"));
    guideStyle_->addItems({tr("Lines"), tr("Dashed Lines")});
    guideSwatch_ = swatch(QColor(), guides);
    guidesGrid->addWidget(new QLabel(tr("Color:"), guides), 0, 0, Qt::AlignRight);
    guidesGrid->addWidget(guideColor_, 0, 1);
    guidesGrid->addWidget(new QLabel(tr("Style:"), guides), 1, 0, Qt::AlignRight);
    guidesGrid->addWidget(guideStyle_, 1, 1);
    guidesGrid->setColumnStretch(2, 1);
    guidesGrid->addWidget(guideSwatch_, 0, 3, 2, 1);
    layout->addWidget(guides);

    auto* smart = new QGroupBox(tr("Smart Guides"), page);
    auto* smartGrid = new QGridLayout(smart);
    auto* smartColor = colorCombo(QStringLiteral("preferencesSmartGuideColor"), smart);
    smartColor->setCurrentIndex(smartColor->findData(QColor(255, 74, 255)));
    smartGrid->addWidget(new QLabel(tr("Color:"), smart), 0, 0, Qt::AlignRight);
    smartGrid->addWidget(smartColor, 0, 1);
    smartGrid->setColumnStretch(2, 1);
    smartGrid->addWidget(swatch(QColor(255, 74, 255), smart), 0, 3);
    smart->setEnabled(false);
    layout->addWidget(smart);

    auto* grid = new QGroupBox(tr("Grid"), page);
    auto* gridGrid = new QGridLayout(grid);
    gridColor_ = colorCombo(QStringLiteral("preferencesGridColor"), grid);
    gridColor_->setCurrentIndex(7); // Light Gray
    gridColorValue_ = gridColor_->itemData(7).value<QColor>();
    gridStyle_ = new QComboBox(grid);
    gridStyle_->setObjectName(QStringLiteral("preferencesGridStyle"));
    gridStyle_->addItems({tr("Lines"), tr("Dashed Lines"), tr("Dots")});
    gridUnit_ = new QComboBox(grid);
    gridUnit_->setObjectName(QStringLiteral("preferencesGridUnit"));
    gridUnit_->addItems({tr("Pixels"), tr("Inches"), tr("Centimeters"), tr("Millimeters"),
                         tr("Points"), tr("Picas"), tr("Percent")});
    gridUnit_->setCurrentIndex(1);
    gridEvery_ = new QDoubleSpinBox(grid);
    gridEvery_->setObjectName(QStringLiteral("preferencesGridEvery"));
    gridEvery_->setRange(0.01, 10000.0);
    gridEvery_->setDecimals(2);
    gridEvery_->setValue(1.0);
    gridSubdivisions_ = new QSpinBox(grid);
    gridSubdivisions_->setObjectName(QStringLiteral("preferencesGridSubdivisions"));
    gridSubdivisions_->setRange(1, 100);
    gridSubdivisions_->setValue(4);
    auto* everyRow = new QHBoxLayout;
    everyRow->addWidget(gridEvery_);
    everyRow->addWidget(gridUnit_);
    gridGrid->addWidget(new QLabel(tr("Color:"), grid), 0, 0, Qt::AlignRight);
    gridGrid->addWidget(gridColor_, 0, 1);
    gridGrid->addWidget(new QLabel(tr("Style:"), grid), 1, 0, Qt::AlignRight);
    gridGrid->addWidget(gridStyle_, 1, 1);
    gridGrid->addWidget(new QLabel(tr("Gridline Every:"), grid), 2, 0, Qt::AlignRight);
    gridGrid->addLayout(everyRow, 2, 1);
    gridGrid->addWidget(new QLabel(tr("Subdivisions:"), grid), 3, 0, Qt::AlignRight);
    gridGrid->addWidget(gridSubdivisions_, 3, 1);
    gridGrid->setColumnStretch(2, 1);
    gridSwatch_ = swatch(QColor(192, 192, 192), grid);
    gridGrid->addWidget(gridSwatch_, 0, 3, 2, 1);
    const auto emitGrid = [this]() {
        if (gridSwatch_) {
            gridSwatch_->setColor(gridColorValue_);
        }
        emit gridSettingsChanged(gridColorValue_, gridStyle_->currentIndex(),
                                 gridUnit_->currentIndex(), gridEvery_->value(),
                                 gridSubdivisions_->value());
    };
    connect(gridColor_, &QComboBox::activated, this, [this, emitGrid](int index) {
        QColor color = gridColor_->itemData(index).value<QColor>();
        if (!color.isValid()) {
            color = QColorDialog::getColor(gridColorValue_, this, tr("Custom Grid Color"));
            if (!color.isValid()) {
                const int previous = gridColor_->findData(gridColorValue_);
                gridColor_->setCurrentIndex(previous >= 0 ? previous : gridColor_->count() - 1);
                return;
            }
        }
        gridColorValue_ = color;
        emitGrid();
    });
    connect(gridStyle_, &QComboBox::activated, this, [emitGrid](int) { emitGrid(); });
    connect(gridUnit_, &QComboBox::activated, this, [emitGrid](int) { emitGrid(); });
    connect(gridEvery_, &QDoubleSpinBox::valueChanged, this, [emitGrid](double) { emitGrid(); });
    connect(gridSubdivisions_, &QSpinBox::valueChanged, this, [emitGrid](int) { emitGrid(); });
    layout->addWidget(grid);

    auto* slices = new QGroupBox(tr("Slices"), page);
    auto* slicesRow = new QHBoxLayout(slices);
    auto* sliceColor = colorCombo(QStringLiteral("preferencesSliceColor"), slices);
    auto* sliceNumbers = new QCheckBox(tr("Show Slice Numbers"), slices);
    sliceNumbers->setChecked(true);
    slicesRow->addWidget(new QLabel(tr("Line Color:"), slices));
    slicesRow->addWidget(sliceColor);
    slicesRow->addWidget(sliceNumbers);
    slicesRow->addStretch(1);
    slices->setEnabled(false);
    layout->addWidget(slices);
    layout->addStretch(1);

    setGuideAppearance(QColor(74, 255, 255), false);
    connect(guideColor_, &QComboBox::activated, this, [this](int index) {
        QColor color = guideColor_->itemData(index).value<QColor>();
        if (!color.isValid()) {
            color = QColorDialog::getColor(guideColorValue_, this, tr("Custom Guide Color"));
            if (!color.isValid()) {
                showGuideColor(guideColorValue_);
                return;
            }
        }
        showGuideColor(color);
        emit guideAppearanceChanged(color, guideStyle_->currentIndex() == 1);
    });
    connect(guideStyle_, &QComboBox::activated, this, [this](int index) {
        emit guideAppearanceChanged(guideColorValue_, index == 1);
    });
    return page;
}

void PreferencesDialog::showGuideColor(const QColor& color)
{
    guideColorValue_ = color;
    int index = guideColor_->count() - 1;
    for (int i = 0; i < guideColor_->count() - 1; ++i) {
        if (guideColor_->itemData(i).value<QColor>() == color) {
            index = i;
            break;
        }
    }
    const QSignalBlocker block(guideColor_);
    guideColor_->setCurrentIndex(index);
    guideSwatch_->setColor(color);
}

void PreferencesDialog::setGuideAppearance(const QColor& color, bool dashed)
{
    showGuideColor(color);
    const QSignalBlocker block(guideStyle_);
    guideStyle_->setCurrentIndex(dashed ? 1 : 0);
}

void PreferencesDialog::setGridSettings(const QColor& color, int style, int unit, double every,
                                        int subdivisions)
{
    const QSignalBlocker blockColor(gridColor_);
    const QSignalBlocker blockStyle(gridStyle_);
    const QSignalBlocker blockUnit(gridUnit_);
    const QSignalBlocker blockEvery(gridEvery_);
    const QSignalBlocker blockSubs(gridSubdivisions_);
    gridColorValue_ = color.isValid() ? color : QColor(192, 192, 192);
    const int index = gridColor_->findData(gridColorValue_);
    gridColor_->setCurrentIndex(index >= 0 ? index : gridColor_->count() - 1);
    gridStyle_->setCurrentIndex(qBound(0, style, 2));
    gridUnit_->setCurrentIndex(qBound(0, unit, 6));
    gridEvery_->setValue(every > 0.0 ? every : 1.0);
    gridSubdivisions_->setValue(qMax(1, subdivisions));
    if (gridSwatch_) {
        gridSwatch_->setColor(gridColorValue_);
    }
}

} // namespace pictura

#include "moc_preferences_dialog.cpp"
