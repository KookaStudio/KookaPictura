#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QListWidget;
class QPushButton;

namespace pictura {

// File > Scripts > Statistics…: the CS6 Image Statistics dialog. It collects
// the source images, the stack mode, and the Auto-Align choice; the caller
// merges the accepted sources through the image-stack pipeline.
class StatisticsDialog : public QDialog {
    Q_OBJECT

public:
    StatisticsDialog(const QStringList& openPaths, bool hasUnsavedOpenDocuments,
                     int initialMode, bool initialAlign, const QString& startFolder,
                     QWidget* parent = nullptr);

    QStringList paths() const;
    bool alignImages() const;
    int stackModeCode() const;

    // Add paths in order, ignoring duplicates and non-files.
    void addFiles(const QStringList& paths);
    // Folder Browse: add the folder's supported image files, not its subfolders.
    void addFolder(const QString& folder);
    // Add the saved open documents; warn when an open document is unsaved.
    void addOpenFiles();

private:
    void browse();
    void removeSelected();
    void refreshButtons();

    QStringList openPaths_;
    QString startFolder_;
    bool hasUnsavedOpenDocuments_ = false;
    QComboBox* use_ = nullptr;
    QComboBox* stackMode_ = nullptr;
    QListWidget* files_ = nullptr;
    QPushButton* browse_ = nullptr;
    QPushButton* remove_ = nullptr;
    QPushButton* addOpen_ = nullptr;
    QCheckBox* align_ = nullptr;
    QPushButton* ok_ = nullptr;
};

} // namespace pictura
