#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QComboBox;
class QLineEdit;

namespace pictura {

class PictureView;

// File > Export > Paths to Illustrator: a destination and the EPS- or
// PDF-based `.ai` encoding.
class ExportPathsDialog : public QDialog {
    Q_OBJECT

public:
    explicit ExportPathsDialog(const QString& defaultPath, QWidget* parent = nullptr);

    QString destination() const;
    // "EPS" or "PDF".
    QString format() const;

private:
    void browse();

    QLineEdit* path_ = nullptr;
    QComboBox* format_ = nullptr;
};

// Run the dialog and write the paths. False without a document, without paths,
// or when the write fails.
bool exportPathsFromView(QWidget* parent, PictureView* view);

// The dialog's work without the dialog, for the Qt test suite.
bool exportPaths(PictureView* view, const QString& path, const QString& format);

} // namespace pictura
