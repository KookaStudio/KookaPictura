#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QComboBox;
class QLineEdit;
class QSpinBox;

namespace pictura {

class PictureView;

// File > Export > Zoomify: output folder, browser base size, tile quality,
// and the built-in viewer template.
class ZoomifyDialog : public QDialog {
    Q_OBJECT

public:
    ZoomifyDialog(int documentWidth, int documentHeight, const QString& folder,
                  QWidget* parent = nullptr);

    QString outputFolder() const;
    int baseWidth() const;
    int baseHeight() const;
    int quality() const;
    QString templateName() const;

private:
    void browse();

    QLineEdit* folder_ = nullptr;
    QSpinBox* width_ = nullptr;
    QSpinBox* height_ = nullptr;
    QSpinBox* quality_ = nullptr;
    QComboBox* template_ = nullptr;
    int documentWidth_ = 0;
    int documentHeight_ = 0;
};

// Run the dialog and write the Zoomify output. False without a document or
// when the export fails.
bool zoomifyExportFromView(QWidget* parent, PictureView* view);

// The dialog's work without the dialog, for the Qt test suite.
bool zoomifyExport(PictureView* view, const QString& folder, int baseWidth, int baseHeight,
                   int quality, bool* cancelled = nullptr, QWidget* parent = nullptr);

// Hold each tile for `milliseconds`; the test suite's deterministic cancel seam.
void zoomifySetTestDelay(int milliseconds);

} // namespace pictura
