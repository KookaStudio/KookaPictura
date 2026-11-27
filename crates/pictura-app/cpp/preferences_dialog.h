#pragma once

#include <QtCore/QHash>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QDialog>

#include "preferences.h"

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QFormLayout;
class QPushButton;
class QSpinBox;
class QLabel;
class QRadioButton;
class QPushButton;
class QListWidget;
class QSlider;
class QStackedWidget;

namespace pictura {

class ColorSwatchWidget;
class NumericField;

// The M41 CS6 Preferences dialog: a left pane list plus a QStackedWidget.
// General, Interface, File Handling, Performance, Units & Rulers, Guides,
// Grid, & Slices, and Type are real pages; the other CS6 panes are listed but disabled. Modeless so it never blocks the headless
// self-test.
class PreferencesDialog : public QDialog {
    Q_OBJECT

public:
    static const QString kGeneral;
    static const QString kInterface;
    static const QString kFileHandling;
    static const QString kPerformance;
    static const QString kUnits;
    static const QString kGuides;
    static const QString kType;
    static const QString kAgentControl;
    static const QString kUpdates;

    explicit PreferencesDialog(QWidget* parent = nullptr);

    // Add a page after the CS6 panes, for what only some windows have.
    void addPage(const QString& name, QWidget* page);

    // Select `page` (a no-op when unknown) and show non-modally.
    void openOn(const QString& page);

    void setShiftKeyForToolSwitch(bool on);
    void setOpenDocumentsAsTabs(bool on);
    void setFloatingDocumentDocking(bool on);
    // General: Zoom Resizes Windows.
    void setZoomResizesWindows(bool on);
    void setHistoryLogSettings(const HistoryLogSettings& settings);
    void setAutoCollapseIconic(bool on);
    void setAutoShowHidden(bool on);
    void setBrightnessLevel(int level);
    void setThemeFamily(int family);
    void setUiScalePercent(int percent);
    int uiScalePercentForTest() const;
    void setLanguageChoices(const QString& savedChoice);
    void setLanguageRestartPending(bool pending);
    void showLanguageStatus(const QString& message);
    void setPickerChoice(int choice);
    int pickerChoice() const;
    void setGpuCompute(bool on);
    // Memory Usage, in percent of the host's memory (#1211).
    void setMemoryUsagePercent(int percent);
    void setGpuComputeEnabled(bool on);
    // Performance (#636): "Let Pictura Use" in bytes, negative when unknown.
    void setMemoryCeiling(qint64 bytes);
    void setHistoryStates(int states);
    void setCacheLevels(int levels);
    // The configured scratch volumes in order; empty for the default.
    void setScratchVolumes(const QStringList& volumes);
    // Append `path` to the scratch volumes (Add's chosen directory); ignored
    // when listed or at four.
    void addScratchVolume(const QString& path);
    // The detected adapter's name; empty when there is none.
    void setGpuAdapter(const QString& name);
    // File Handling: Automatically Save Recovery Information Every `minutes`.
    void setSaveInBackground(bool on);
    void setAutoSaveRecovery(bool on, int minutes);
    void setNativeSaveCompatibility(int choice);
    // Save Camera Raw Settings In: 0 Camera Raw Database, 1 sidecar files.
    void setRawSettingsStore(int store);
    // Guides, Grid, & Slices: the guide colour and Lines / Dashed Lines style.
    void setGuideAppearance(const QColor& color, bool dashed);
    // Guides, Grid, & Slices: the live Grid group.
    void setGridSettings(const QColor& color, int style, int unit, double every, int subdivisions);
    // Units & Rulers: the ruler unit (a RulerUnit index) and Traditional
    // (72.27) rather than PostScript (72) points per inch.
    void setRulerUnit(int unit);
    void setTraditionalPoints(bool on);
    void setColumnSize(double widthInches, double gutterInches);
    // Type: Fill New Type Layers With Placeholder Text (not CS6; Photoshop CC).
    void setTypePlaceholderText(bool on);
    // Type: Show Asian Text Options, Middle Eastern, Missing Glyph Protection.
    void setTypeShowAsianOptions(bool on);
    void setTypeMiddleEastern(bool on);
    void setTypeMissingGlyphProtection(bool on);

    // Test hooks.
    QStringList pagesForTest() const;
    QString currentPageForTest() const;
    bool setCheckboxForTest(const QString& key, bool on);
    bool checkboxForTest(const QString& key) const;

signals:
    void historyLogChanged(const HistoryLogSettings& settings);
    void exportSettingsRequested();
    void importSettingsRequested();
    void importTranslationRequested();
    void languageSelected(const QString& selection);
    void languageRestartRequested();
    void useShiftKeyForToolSwitchChanged(bool on);
    void openDocumentsAsTabsChanged(bool on);
    void floatingDocumentDockingChanged(bool on);
    void zoomResizesWindowsChanged(bool on);
    void autoCollapseIconicChanged(bool on);
    void autoShowHiddenChanged(bool on);
    void brightnessLevelChanged(int level);
    void themeFamilyChanged(int family);
    void uiScalePercentChanged(int percent);
    void pickerChoiceChanged(int choice);
    void gpuComputeChanged(bool on);
    void memoryUsagePercentChanged(int percent);
    void historyStatesChanged(int states);
    void cacheLevelsChanged(int levels);
    void scratchVolumesChanged(const QStringList& volumes);
    void saveInBackgroundChanged(bool on);
    void autoSaveRecoveryChanged(bool on, int minutes);
    void nativeSaveCompatibilityChanged(int choice);
    void rawSettingsStoreChanged(int store);
    void guideAppearanceChanged(const QColor& color, bool dashed);
    void gridSettingsChanged(const QColor& color, int style, int unit, double every,
                             int subdivisions);
    void rulerUnitChanged(int unit);
    void traditionalPointsChanged(bool on);
    void columnSizeChanged(double widthInches, double gutterInches);
    void typePlaceholderTextChanged(bool on);
    void typeShowAsianOptionsChanged(bool on);
    void typeMiddleEasternChanged(bool on);
    void typeMissingGlyphProtectionChanged(bool on);

private:
    void addHistoryLogControls(QFormLayout* form, QWidget* page);
    void updateHistoryLog();
    bool chooseHistoryLogPath();

    HistoryLogSettings historyLogSettings_;
    QCheckBox* historyLogEnabled_ = nullptr;
    QComboBox* historyLogDestination_ = nullptr;
    QComboBox* historyLogVerbosity_ = nullptr;
    QPushButton* historyLogChoose_ = nullptr;
    void showPage(const QString& page);
    QCheckBox* makeCheckbox(const QString& key, const QString& label, QWidget* page);
    QCheckBox* checkbox(const QString& key) const;
    QWidget* buildPerformancePage();
    void showScratchVolumes();
    void moveScratchVolume(int delta);
    QWidget* buildUnitsPage();
    void syncColumnSizeFields();
    QWidget* buildGuidesPage();
    void showGuideColor(const QColor& color);

    QListWidget* pageList_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QStringList realPages_;
    QHash<QString, QCheckBox*> checkboxes_;
    NumericField* brightness_ = nullptr;
    QComboBox* themeFamily_ = nullptr;
    QComboBox* uiScale_ = nullptr;
    QComboBox* language_ = nullptr;
    QLabel* languageStatus_ = nullptr;
    QPushButton* languageRestart_ = nullptr;
    QComboBox* pickerChoice_ = nullptr;
    QComboBox* imageInterpolation_ = nullptr;
    QComboBox* autoSaveMinutes_ = nullptr;
    QComboBox* nativeSaveCompatibility_ = nullptr;
    QComboBox* rawSettingsStore_ = nullptr;
    QSpinBox* memoryUsage_ = nullptr;
    QSlider* memorySlider_ = nullptr;
    QLabel* memoryCeiling_ = nullptr;
    QSpinBox* historyStates_ = nullptr;
    QSpinBox* cacheLevels_ = nullptr;
    QListWidget* scratchList_ = nullptr;
    QPushButton* scratchAdd_ = nullptr;
    QPushButton* scratchRemove_ = nullptr;
    QPushButton* scratchUp_ = nullptr;
    QPushButton* scratchDown_ = nullptr;
    QStringList scratchVolumes_;
    QLabel* gpuAdapter_ = nullptr;
    QComboBox* guideColor_ = nullptr;
    QComboBox* guideStyle_ = nullptr;
    ColorSwatchWidget* guideSwatch_ = nullptr;
    QColor guideColorValue_;
    QComboBox* gridColor_ = nullptr;
    QComboBox* gridStyle_ = nullptr;
    QComboBox* gridUnit_ = nullptr;
    QDoubleSpinBox* gridEvery_ = nullptr;
    QSpinBox* gridSubdivisions_ = nullptr;
    ColorSwatchWidget* gridSwatch_ = nullptr;
    QColor gridColorValue_{0xc0, 0xc0, 0xc0};
    double columnWidthInches_ = 2.5;
    double columnGutterInches_ = 1.0 / 6.0;
    QDoubleSpinBox* columnWidth_ = nullptr;
    QDoubleSpinBox* columnGutter_ = nullptr;
    QComboBox* columnWidthUnit_ = nullptr;
    QComboBox* columnGutterUnit_ = nullptr;
    QComboBox* rulerUnit_ = nullptr;
    QRadioButton* postScriptPoints_ = nullptr;
    QRadioButton* traditionalPoints_ = nullptr;
};

} // namespace pictura
