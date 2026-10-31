#pragma once

#include <QtCore/QHash>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QLabel;
class QRadioButton;
class QListWidget;
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
    void setAutoCollapseIconic(bool on);
    void setAutoShowHidden(bool on);
    void setBrightnessLevel(int level);
    void setThemeFamily(int family);
    void setUiScalePercent(int percent);
    int uiScalePercentForTest() const;
    void setPickerChoice(int choice);
    int pickerChoice() const;
    void setGpuCompute(bool on);
    void setGpuComputeEnabled(bool on);
    // File Handling: Automatically Save Recovery Information Every `minutes`.
    void setAutoSaveRecovery(bool on, int minutes);
    // Guides, Grid, & Slices: the guide colour and Lines / Dashed Lines style.
    void setGuideAppearance(const QColor& color, bool dashed);
    // Units & Rulers: the ruler unit (a RulerUnit index) and Traditional
    // (72.27) rather than PostScript (72) points per inch.
    void setRulerUnit(int unit);
    void setTraditionalPoints(bool on);
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
    void autoSaveRecoveryChanged(bool on, int minutes);
    void guideAppearanceChanged(const QColor& color, bool dashed);
    void rulerUnitChanged(int unit);
    void traditionalPointsChanged(bool on);
    void typePlaceholderTextChanged(bool on);
    void typeShowAsianOptionsChanged(bool on);
    void typeMiddleEasternChanged(bool on);
    void typeMissingGlyphProtectionChanged(bool on);

private:
    void showPage(const QString& page);
    QCheckBox* makeCheckbox(const QString& key, const QString& label, QWidget* page);
    QCheckBox* checkbox(const QString& key) const;
    QWidget* buildUnitsPage();
    QWidget* buildGuidesPage();
    void showGuideColor(const QColor& color);

    QListWidget* pageList_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QStringList realPages_;
    QHash<QString, QCheckBox*> checkboxes_;
    NumericField* brightness_ = nullptr;
    QComboBox* themeFamily_ = nullptr;
    QComboBox* uiScale_ = nullptr;
    QComboBox* pickerChoice_ = nullptr;
    QComboBox* autoSaveMinutes_ = nullptr;
    QComboBox* guideColor_ = nullptr;
    QComboBox* guideStyle_ = nullptr;
    ColorSwatchWidget* guideSwatch_ = nullptr;
    QColor guideColorValue_;
    QComboBox* rulerUnit_ = nullptr;
    QRadioButton* postScriptPoints_ = nullptr;
    QRadioButton* traditionalPoints_ = nullptr;
};

} // namespace pictura
