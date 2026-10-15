#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QLabel;
class QListWidget;
class QPushButton;
class QScrollArea;
class QSpinBox;
class QTimer;

namespace pictura {

class PictureView;

// One export target: the flattened document or a single top-level layer, its
// name, and its straight-alpha RGBA8888 image at the target's own size.
struct ExportEntry {
    QString name;
    QString kind;
    QImage image;
};

// The CC Export As dialog: a layer list, a preview with zoom, File Settings
// (Format plus Transparency/Quality), Image Size (linked W/H/Scale + Resample),
// Canvas Size (+ Reset), and a bounded output-size estimate.
class ExportAsDialog : public QDialog {
    Q_OBJECT

public:
    enum Format { Png = 0, Jpg = 1, Png8 = 2, Gif = 3, Svg = 4 };

    explicit ExportAsDialog(const QVector<ExportEntry>& entries, QWidget* parent = nullptr);

    QString format() const;
    int quality() const;
    int scale() const;
    // The selected entry resampled to the image size and placed on the canvas.
    QImage sized(int entry) const;
    // `sized(entry)` encoded in the chosen format.
    QByteArray encoded(int entry) const;
    QString suffix() const;
    bool saveTo(int entry, const QString& path) const;
    QVector<int> checkedEntries() const;

protected:
    void showEvent(QShowEvent* event) override;

private:
    void buildUi();
    void refreshFormatControls();
    void refreshPreview();
    void scheduleSizeEstimate();
    void applyZoom(double factor);
    void fitZoom();
    void resetSizingFor(int entry);
    void syncCanvasToImage();
    int currentEntry() const;
    QByteArray encodedAs(int entry, Format format) const;

    QVector<ExportEntry> entries_;
    QListWidget* list_ = nullptr;
    QScrollArea* previewArea_ = nullptr;
    QLabel* preview_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    QLabel* sizeLabel_ = nullptr;
    QComboBox* format_ = nullptr;
    QCheckBox* transparency_ = nullptr;
    QSpinBox* quality_ = nullptr;
    QLabel* qualityLabel_ = nullptr;
    QSpinBox* width_ = nullptr;
    QSpinBox* height_ = nullptr;
    QSpinBox* scale_ = nullptr;
    QCheckBox* constrain_ = nullptr;
    QComboBox* resample_ = nullptr;
    QSpinBox* canvasWidth_ = nullptr;
    QSpinBox* canvasHeight_ = nullptr;
    QPushButton* canvasReset_ = nullptr;
    QTimer* sizeTimer_ = nullptr;
    double zoom_ = 1.0;
    bool fit_ = true;
    // The canvas follows the image size until the user edits Canvas Size; Reset
    // re-links it.
    bool canvasLinked_ = true;
    bool updatingCanvas_ = false;
};

// Run the CC Export As dialog and write the checked targets. False when
// cancelled, when there is no document, or when an encode fails. Never changes
// the document's path, modified state, or history.
bool exportAsFromView(QWidget* parent, PictureView* view);

// Write `<source stem>.png` beside the view's file, prompting once when the
// document is untitled. False when cancelled or the encode fails.
bool quickExportPngFromView(QWidget* parent, PictureView* view);

} // namespace pictura
