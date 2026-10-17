#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QComboBox;
class QGroupBox;
class QLabel;
class QListWidget;
class QPlainTextEdit;
class QStackedWidget;

namespace pictura {

// The Save Adobe PDF dialog (`interop/pdf-export`): the Save options, Adobe
// PDF Preset, Standard, and Compatibility controls over a General /
// Compression / Output / Security / Summary page list. Every control edits
// one option field of the canonical `.joboptions` text through the bridge,
// and is disabled when the engine reports the field locked. In preset-editor
// mode (Edit > Adobe PDF Presets) the Save options and Security are hidden.
class PdfSaveDialog : public QDialog {
    Q_OBJECT

public:
    enum class Mode { Save, PresetEditor };

    PdfSaveDialog(const QString& options, const QString& presetsDir, Mode mode,
                  QWidget* parent = nullptr);

    // The edited options as canonical `.joboptions` text.
    QString options() const { return options_; }

    // `<state dir>/PDF Presets`, where user presets live as `.joboptions`.
    static QString presetsDirectory();
    // The user presets in `dir`, by file name without the extension, sorted.
    static QStringList userPresetNames(const QString& dir);
    // Options text for a built-in (bracketed) or user preset; empty if unknown.
    static QString presetOptions(const QString& name, const QString& dir);

    QStringList pageNames() const;
    QString currentPage() const;
    void showPage(const QString& name);
    QStringList presetNames() const;
    bool selectPreset(const QString& name);
    // The preset combo's text: the preset name, plus " (Modified)" after an edit.
    QString presetLabel() const;
    // Save Preset… without the name prompt.
    bool savePresetAs(const QString& name);
    // Edit `field` as the user would through its control.
    void setFieldForTest(const QString& field, const QString& value);
    QWidget* fieldWidgetForTest(const QString& field) const;
    QString summaryText() const;
    bool compressionItemEnabled(int index) const;
    QString encryptionLevelText() const;

private:
    struct Binding {
        QString field;
        QWidget* widget;
    };

    void bind(const QString& field, QWidget* widget);
    void setField(const QString& field, const QString& value);
    void refresh();
    void applyPreset(int index);
    void setPreset(int index, const QString& options);
    void promptSavePreset();
    QWidget* buildGeneralPage();
    QWidget* buildCompressionPage();
    QWidget* buildOutputPage();
    QWidget* buildSecurityPage();
    QWidget* buildSummaryPage();
    QComboBox* combo(const QString& field, const QStringList& items, QWidget* parent);

    QString options_;
    QString presetsDir_;
    Mode mode_;
    // The preset the options came from: its combo row, plain name, and options.
    int presetIndex_ = 0;
    QString presetName_;
    QString presetBaseline_;
    QList<Binding> bindings_;
    QComboBox* presetCombo_ = nullptr;
    QListWidget* pageList_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QGroupBox* saveOptions_ = nullptr;
    QLabel* encryptionLevel_ = nullptr;
    QPlainTextEdit* summary_ = nullptr;
    bool refreshing_ = false;
};

} // namespace pictura
