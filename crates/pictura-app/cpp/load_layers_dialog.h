#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QListWidget;
class QPushButton;

namespace pictura {

// File > Scripts > Load Files into Stack: the CS6 Load Layers dialog. The
// source list holds absolute paths; the two options choose Auto-Align and the
// embedded smart-object combination.
class LoadLayersDialog : public QDialog {
    Q_OBJECT

public:
    LoadLayersDialog(const QStringList& openPaths, const QString& startFolder,
                     QWidget* parent = nullptr);

    QStringList paths() const;
    bool alignImages() const;
    bool createSmartObject() const;

    // Add paths in order, ignoring duplicates and non-files.
    void addFiles(const QStringList& paths);
    // Folder mode: replace the list with the folder's supported image files.
    void setFolder(const QString& folder);

private:
    void browse();
    void removeSelected();
    void addOpenFiles();
    void refreshMode();
    void refreshButtons();

    QStringList openPaths_;
    QString startFolder_;
    QComboBox* use_ = nullptr;
    QListWidget* files_ = nullptr;
    QPushButton* browse_ = nullptr;
    QPushButton* remove_ = nullptr;
    QPushButton* addOpen_ = nullptr;
    QCheckBox* align_ = nullptr;
    QCheckBox* smartObject_ = nullptr;
    QPushButton* ok_ = nullptr;
};

} // namespace pictura
