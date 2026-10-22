#pragma once

#include <QtGui/QImage>
#include <QtGui/QPageSize>
#include <QtPrintSupport/QPrinter>
#include <QtWidgets/QDialog>

#include <functional>

class QCheckBox;
class QComboBox;
class QLabel;
class QPrinter;
class QPushButton;
class QSpinBox;

namespace pictura {

class PictureView;

// The settings File > Print keeps as the defaults for the next Print and for
// Print One Copy (CS6 saves them on Done or Print). An empty `printer` is the
// Save as PDF destination.
struct PrintSettings {
    QString printer;
    int copies = 1;
    bool landscape = false;
    // An invalid QPageSize means the destination's default paper.
    QPageSize pageSize;
    QPrinter::PaperSource paperSource = QPrinter::Auto;
};
PrintSettings& lastPrintSettings();

// Point `printer` at `settings`: a CUPS queue, or a PDF at `pdfPath` when the
// settings name no printer.
void configurePrinter(QPrinter& printer, const PrintSettings& settings, const QString& pdfPath);

// Draw `image` centred and scaled to fit the printable area (CS6's Scale To
// Fit Media). False when the printer cannot be painted on.
bool printImage(QPrinter& printer, const QImage& image);

// File > Print (WF-013): a page preview, Printer Setup (printer or Save as
// PDF, copies, Print Settings for the CUPS dialog, portrait / landscape),
// Color Management, and Cancel / Done / Print. Ported from photorust's
// PrintDialog.
// ponytail: printer-managed colour only (Photoshop Manages Colors and
// Separations, Match Print Colors, and Gamut Warning are shown disabled); no
// Position and Size fields, Print Selected Area, output marks, or Description.
class PrintDialog : public QDialog {
    Q_OBJECT

public:
    explicit PrintDialog(const QImage& image, QWidget* parent = nullptr);

    PrintSettings settings() const;
    // Where Save as PDF writes; defaults to a file dialog (tests replace it).
    void setPdfPathProvider(std::function<QString()> provider);
    // The page-setup dialog; tests replace it to pick a size without a printer.
    void setPageSetupProvider(std::function<bool(QPrinter&)> provider);
    QImage pagePreviewForTest() const { return preview_; }

private:
    void populatePrinters();
    void updatePreview();
    void print();
    // The selected destination's paper size in inches, before orientation.
    QSizeF paperInches() const;

    QImage image_;
    QImage preview_;
    std::function<QString()> pdfPath_;
    std::function<bool(QPrinter&)> pageSetup_;
    QPageSize chosenPageSize_;
    QPrinter::PaperSource paperSource_ = QPrinter::Auto;
    QComboBox* printer_ = nullptr;
    QSpinBox* copies_ = nullptr;
    QPushButton* printSettings_ = nullptr;
    QPushButton* portrait_ = nullptr;
    QPushButton* landscape_ = nullptr;
    QLabel* previewLabel_ = nullptr;
    QLabel* pageSize_ = nullptr;
    QCheckBox* paperWhite_ = nullptr;
};

// The active document's flattened sRGB image, or a null image.
QImage printableImage(PictureView* view);
// File > Print: run the dialog over `view`.
void printFromView(QWidget* parent, PictureView* view);
// File > Print One Copy: print once with the last settings, without the
// dialog; with no usable printer remembered, open the dialog instead.
void printOneCopyFromView(QWidget* parent, PictureView* view);

} // namespace pictura
