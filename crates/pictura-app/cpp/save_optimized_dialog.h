#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QComboBox;
class QLineEdit;

namespace pictura {

// Save Optimized As: the output file name, Format (HTML and Images / Images
// Only / HTML Only), and the Table/CSS layout. Slices are always All Slices.
class SaveOptimizedDialog : public QDialog {
    Q_OBJECT

public:
    enum Format { HtmlAndImages, ImagesOnly, HtmlOnly };
    enum Layout { Table, Css };

    explicit SaveOptimizedDialog(const QString& defaultPath, QWidget* parent = nullptr);

    QString path() const;
    int format() const;
    int layout() const;

private:
    QLineEdit* path_ = nullptr;
    QComboBox* format_ = nullptr;
    QComboBox* layout_ = nullptr;
};

} // namespace pictura
