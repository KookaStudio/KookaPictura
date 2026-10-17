#pragma once

#include <QtCore/QList>
#include <QtCore/QRectF>
#include <QtCore/QSize>
#include <QtCore/QVector>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include <array>

class QCheckBox;
class QComboBox;
class QGroupBox;
class QLabel;
class QLineEdit;
class QListWidget;
class QRadioButton;

namespace pictura {

class RasterSizeFields;

// What `pdf_import_probe` reports about a PDF: each page's boxes (Bounding,
// Media, Crop, Bleed, Trim, Art) in view-space points, and its embedded
// images (`imagesAvailable` is false for an encrypted or unreadable file).
struct PdfImportProbe {
    struct Page {
        double viewHeight = 0;
        std::array<QRectF, 6> boxes;
    };
    struct Image {
        int width = 0;
        int height = 0;
        int page = 0;
    };
    QVector<Page> pages;
    QVector<Image> images;
    bool imagesAvailable = false;

    // Parse the probe text after its `ok` line.
    static PdfImportProbe parse(const QString& text);
};

// Split the packed thumbnails `pdf_import_thumbnails` returns.
QVector<QImage> unpackThumbnails(const QByteArray& packed);

// File > Open on a generic PDF (docs/10-workflow-io/open-and-new.md, *Open
// PDF*): choose pages or embedded images, the page box, and the raster size,
// mode, and depth. Layout and defaults follow the owner's CS6 screenshots on
// #627.
class ImportPdfDialog : public QDialog {
    Q_OBJECT

public:
    ImportPdfDialog(const QString& path, const QString& password, const PdfImportProbe& probe,
                    QWidget* parent = nullptr);

    bool imagesMode() const;
    // The chosen pages or images, 0-based, ascending.
    QList<int> selection() const;
    QString name() const;
    bool antiAliased() const;
    bool suppressWarnings() const;
    RasterSizeFields* sizeFields() const { return size_; }
    // Page `page`'s pixel size: every selected page scales by the factor that
    // maps the largest selected box to the Width × Height fields.
    QSize pixelSizeFor(int page) const;
    // Page `page`'s chosen box as `x0 y0 x1 y1 view_height` for the bridge.
    QString areaFor(int page) const;

    // Test seams. Names: "select" (comma-separated 1-based numbers), "images"
    // ("1" for Images mode), "name", "cropTo" (menu text), "antiAliased",
    // "suppress", "thumbnailSize" (menu text); anything else goes to the Image
    // Size fields. `fieldForTest` also answers "selectedLabel", "pagesChecked",
    // "imagesEnabled", "threeDEnabled", "iconSize", and "count".
    void setFieldForTest(const QString& name, const QString& value);
    QString fieldForTest(const QString& name) const;

protected:
    void resizeEvent(QResizeEvent* event) override;

private:
    void buildUi();
    void showPages();
    void showImages();
    void selectionChanged();
    void applyThumbnailSize();
    QRectF boxOf(int page) const;
    // The largest selected box (widest width, tallest height).
    QSizeF largestBox() const;

    QString path_;
    QString password_;
    PdfImportProbe probe_;
    QVector<QImage> pageThumbs_;
    QVector<QImage> imageThumbs_;
    bool imageThumbsLoaded_ = false;

    QRadioButton* pages_ = nullptr;
    QRadioButton* images_ = nullptr;
    QRadioButton* threeD_ = nullptr;
    QListWidget* grid_ = nullptr;
    QLabel* selectedLabel_ = nullptr;
    QComboBox* thumbSize_ = nullptr;
    QGroupBox* pageOptions_ = nullptr;
    QLineEdit* name_ = nullptr;
    QComboBox* cropTo_ = nullptr;
    QCheckBox* antiAliased_ = nullptr;
    RasterSizeFields* size_ = nullptr;
    QCheckBox* suppress_ = nullptr;
};

} // namespace pictura
