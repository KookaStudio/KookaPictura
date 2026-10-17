#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

#include <functional>

class QLabel;
class QListWidget;
class QPushButton;

namespace pictura {

// Edit > Adobe PDF Presets…: every preset (built-ins in brackets, then the
// user's `.joboptions` files) with its description, and New / Edit / Delete /
// Load / Save As for user presets. Built-in presets are read-only. `runEditor`
// runs the preset editor (a PdfSaveDialog) modally; tests replace it.
class PdfPresetsDialog : public QDialog {
    Q_OBJECT

public:
    PdfPresetsDialog(const QString& presetsDir, std::function<bool(QDialog&)> runEditor,
                     QWidget* parent = nullptr);

    QStringList presetNames() const;
    bool select(const QString& name);
    bool canEdit() const;
    QString description() const;
    // New from the selected preset under `name`, opened in the editor.
    bool newPreset(const QString& name);
    bool editSelected();
    bool deleteSelected();
    // Copy a `.joboptions` file into the presets folder.
    bool load(const QString& file);
    bool saveSelectedAs(const QString& file);

private:
    void reload(const QString& select);
    void updateButtons();
    QString selectedName() const;
    QString filePath(const QString& name) const;
    bool edit(const QString& name, const QString& options);

    QString presetsDir_;
    std::function<bool(QDialog&)> runEditor_;
    QListWidget* list_ = nullptr;
    QLabel* description_ = nullptr;
    QPushButton* editButton_ = nullptr;
    QPushButton* deleteButton_ = nullptr;
    QPushButton* saveAsButton_ = nullptr;
};

} // namespace pictura
