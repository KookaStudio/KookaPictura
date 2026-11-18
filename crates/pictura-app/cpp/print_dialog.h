#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QPointer>
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
class QVBoxLayout;

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
    QJsonObject options{};
};
PrintSettings& lastPrintSettings();
PrintSettings readPrintSettings();

// Point `printer` at `settings`: a CUPS queue, or a PDF at `pdfPath` when the
// settings name no printer.
void configurePrinter(QPrinter& printer, const PrintSettings& settings, const QString& pdfPath);

// Draw `image` centred and scaled to fit the printable area (CS6's Scale To
// Fit Media). False when the printer cannot be painted on.
bool printImage(QPrinter& printer, const QImage& image);

class PrintDialog : public QDialog {
    Q_OBJECT

public:
    explicit PrintDialog(const QImage& image, QWidget* parent = nullptr);
    explicit PrintDialog(PictureView& view, QWidget* parent = nullptr);

    PrintSettings settings() const;
    // Where Save as PDF writes; defaults to a file dialog (tests replace it).
    void setPdfPathProvider(std::function<QString()> provider);
    // The page-setup dialog; tests replace it to pick a size without a printer.
    void setPageSetupProvider(std::function<bool(QPrinter&)> provider);
    QImage pagePreviewForTest() const { return preview_; }
    void setStatus(const QString& message);

private:
    PrintDialog(const QImage& image, PictureView* view, QWidget* parent);
    void populatePrinters();
    void addColorControls(QVBoxLayout* layout);
    void addOutputControls(QVBoxLayout* layout);
    void populateProfiles();
    void updateOptions();
    void updatePreview();
    void print();
    bool acceptSettings(const PrintSettings& chosen, bool commit);

    QPointer<PictureView> view_;
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
    QLabel* status_ = nullptr;
    QLabel* colorHint_ = nullptr;
    QComboBox* handling_ = nullptr;
    QComboBox* profile_ = nullptr;
    QComboBox* intent_ = nullptr;
    QPushButton* openProfile_ = nullptr;
    QCheckBox* blackPoint_ = nullptr;
    QCheckBox* hardProof_ = nullptr;
    QPushButton* proofSetup_ = nullptr;
    QPushButton* cupsSetup_ = nullptr;
    QCheckBox* matchColors_ = nullptr;
    QCheckBox* gamut_ = nullptr;
    QCheckBox* paperWhite_ = nullptr;
    QJsonObject options_;
    QJsonObject initialOptions_;
    QString filename_;
    QString description_;
};

// The active document's flattened sRGB image, or a null image.
QImage printableImage(PictureView* view);
// File > Print: run the dialog over `view`.
void printFromView(QWidget* parent, PictureView* view);
// File > Print One Copy: print once with the last settings, without the
// dialog; with no usable printer remembered, open the dialog instead.
void printOneCopyFromView(QWidget* parent, PictureView* view);

} // namespace pictura
