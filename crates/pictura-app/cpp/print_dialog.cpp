#include "print_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/web_export.cxxqt.h"

#include <QtGui/QPainter>
#include <QtGui/QStandardItemModel>
#include <QtPrintSupport/QPrintDialog>
#include <QtPrintSupport/QPrinter>
#include <QtPrintSupport/QPrinterInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <cstdint>

namespace pictura {

namespace {

constexpr int kPreviewWidth = 280;
constexpr int kPreviewHeight = 360;

QIcon orientationIcon(bool landscape)
{
    QPixmap pixmap(20, 20);
    pixmap.fill(Qt::transparent);
    QPainter p(&pixmap);
    p.setPen(QColor(0xd4, 0xd4, 0xd4));
    p.drawRect(landscape ? QRect(2, 4, 16, 12) : QRect(4, 2, 12, 16));
    return QIcon(pixmap);
}

// A combo entry CS6 offers that is not implemented: listed, not selectable.
void addDisabledItem(QComboBox* combo, const QString& text)
{
    combo->addItem(text);
    if (auto* model = qobject_cast<QStandardItemModel*>(combo->model())) {
        model->item(combo->count() - 1)->setEnabled(false);
    }
}

} // namespace

PrintSettings& lastPrintSettings()
{
    static PrintSettings settings{QPrinterInfo::defaultPrinter().printerName(), 1, false};
    return settings;
}

void configurePrinter(QPrinter& printer, const PrintSettings& settings, const QString& pdfPath)
{
    if (settings.printer.isEmpty()) {
        printer.setOutputFormat(QPrinter::PdfFormat);
        printer.setOutputFileName(pdfPath);
        printer.setPageSize(QPageSize(QPageSize::Letter));
    } else {
        printer.setPrinterName(settings.printer);
    }
    printer.setCopyCount(settings.copies);
    printer.setPageOrientation(settings.landscape ? QPageLayout::Landscape
                                                  : QPageLayout::Portrait);
}

bool printImage(QPrinter& printer, const QImage& image)
{
    if (image.isNull()) {
        return false;
    }
    QPainter painter(&printer);
    if (!painter.isActive()) {
        return false;
    }
    const QRect page = painter.viewport();
    QSize fitted = image.size();
    fitted.scale(page.size(), Qt::KeepAspectRatio);
    painter.setRenderHint(QPainter::SmoothPixmapTransform);
    painter.drawImage(QRect((page.width() - fitted.width()) / 2,
                            (page.height() - fitted.height()) / 2, fitted.width(),
                            fitted.height()),
                      image);
    return painter.end();
}

PrintDialog::PrintDialog(const QImage& image, QWidget* parent)
    : QDialog(parent)
    , image_(image)
{
    setWindowTitle(QStringLiteral("Kooka Pictura Print Settings"));
    setObjectName(QStringLiteral("printDialog"));
    // CS6 lets the dialog and its preview be resized.
    setSizeGripEnabled(true);
    auto* outer = new QVBoxLayout(this);
    auto* body = new QHBoxLayout();

    // Left: the page preview and the preview aids.
    auto* left = new QVBoxLayout();
    pageSize_ = new QLabel(this);
    pageSize_->setObjectName(QStringLiteral("printPageSize"));
    left->addWidget(pageSize_);
    previewLabel_ = new QLabel(this);
    previewLabel_->setFixedSize(kPreviewWidth, kPreviewHeight);
    previewLabel_->setAlignment(Qt::AlignCenter);
    left->addWidget(previewLabel_);
    left->addSpacing(12);
    for (const char* aid : {"Match Print Colors", "Gamut Warning"}) {
        auto* check = new QCheckBox(QLatin1String(aid), this);
        check->setEnabled(false);
        check->setToolTip(QLatin1String(aid) + QStringLiteral(" — needs soft proofing, not "
                                                               "implemented yet"));
        left->addWidget(check);
    }
    paperWhite_ = new QCheckBox(QStringLiteral("Show Paper White"), this);
    paperWhite_->setObjectName(QStringLiteral("printPaperWhite"));
    left->addWidget(paperWhite_);
    left->addStretch();
    body->addLayout(left);

    // Right: Printer Setup and Color Management.
    auto* right = new QVBoxLayout();
    auto* setup = new QGroupBox(QStringLiteral("Printer Setup"), this);
    auto* setupLayout = new QVBoxLayout(setup);
    auto* printerRow = new QHBoxLayout();
    printerRow->addWidget(new QLabel(QStringLiteral("Printer:"), setup));
    printer_ = new QComboBox(setup);
    printer_->setObjectName(QStringLiteral("printPrinter"));
    printer_->setMinimumWidth(200);
    printerRow->addWidget(printer_, 1);
    setupLayout->addLayout(printerRow);

    auto* copiesRow = new QHBoxLayout();
    copiesRow->addWidget(new QLabel(QStringLiteral("Copies:"), setup));
    copies_ = new QSpinBox(setup);
    copies_->setObjectName(QStringLiteral("printCopies"));
    copies_->setRange(1, 999);
    copiesRow->addWidget(copies_);
    copiesRow->addSpacing(12);
    printSettings_ = new QPushButton(QStringLiteral("Print Settings…"), setup);
    copiesRow->addWidget(printSettings_);
    copiesRow->addStretch();
    setupLayout->addLayout(copiesRow);

    auto* layoutRow = new QHBoxLayout();
    layoutRow->addWidget(new QLabel(QStringLiteral("Layout:"), setup));
    portrait_ = new QPushButton(setup);
    landscape_ = new QPushButton(setup);
    portrait_->setObjectName(QStringLiteral("printPortrait"));
    landscape_->setObjectName(QStringLiteral("printLandscape"));
    for (QPushButton* button : {portrait_, landscape_}) {
        const bool wide = button == landscape_;
        button->setIcon(orientationIcon(wide));
        button->setCheckable(true);
        button->setAutoExclusive(true);
        button->setFixedSize(30, 26);
        button->setToolTip(wide ? QStringLiteral("Landscape") : QStringLiteral("Portrait"));
        layoutRow->addWidget(button);
    }
    layoutRow->addStretch();
    setupLayout->addLayout(layoutRow);
    right->addWidget(setup);

    auto* color = new QGroupBox(QStringLiteral("Color Management"), this);
    auto* colorLayout = new QVBoxLayout(color);
    auto* hint = new QLabel(QStringLiteral("Remember to enable the printer's color management "
                                           "in the print settings dialog box."),
                            color);
    hint->setWordWrap(true);
    colorLayout->addWidget(hint);
    // The printed image is the flattened document converted to sRGB.
    colorLayout->addWidget(new QLabel(QStringLiteral("Document Profile: sRGB IEC61966-2.1"), color));
    auto* handlingRow = new QHBoxLayout();
    handlingRow->addWidget(new QLabel(QStringLiteral("Color Handling:"), color));
    auto* handling = new QComboBox(color);
    handling->addItem(QStringLiteral("Printer Manages Colors"));
    addDisabledItem(handling, QStringLiteral("Photoshop Manages Colors"));
    addDisabledItem(handling, QStringLiteral("Separations"));
    handlingRow->addWidget(handling, 1);
    colorLayout->addLayout(handlingRow);
    const auto disabledRow = [&](const QString& label, const QStringList& items) {
        auto* row = new QHBoxLayout();
        row->addWidget(new QLabel(label, color));
        auto* combo = new QComboBox(color);
        combo->addItems(items);
        combo->setEnabled(false);
        row->addWidget(combo, 1);
        colorLayout->addLayout(row);
    };
    disabledRow(QStringLiteral("Printer Profile:"), {QStringLiteral("Working RGB - sRGB")});
    disabledRow(QStringLiteral("Rendering Intent:"),
                {QStringLiteral("Relative Colorimetric"), QStringLiteral("Perceptual"),
                 QStringLiteral("Saturation"), QStringLiteral("Absolute Colorimetric")});
    auto* blackPoint = new QCheckBox(QStringLiteral("Black Point Compensation"), color);
    blackPoint->setEnabled(false);
    colorLayout->addWidget(blackPoint);
    right->addWidget(color);
    right->addStretch();
    body->addLayout(right, 1);
    outer->addLayout(body, 1);

    auto* buttons = new QHBoxLayout();
    buttons->addStretch();
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    auto* done = new QPushButton(QStringLiteral("Done"), this);
    auto* printButton = new QPushButton(QStringLiteral("Print"), this);
    done->setObjectName(QStringLiteral("printDone"));
    printButton->setObjectName(QStringLiteral("printPrint"));
    printButton->setDefault(true);
    buttons->addWidget(cancel);
    buttons->addWidget(done);
    buttons->addWidget(printButton);
    outer->addLayout(buttons);

    pdfPath_ = [this]() {
        return QFileDialog::getSaveFileName(this, QStringLiteral("Save Print Output As"),
                                            QString(), QStringLiteral("PDF Document (*.pdf)"));
    };
    const PrintSettings& last = lastPrintSettings();
    populatePrinters();
    printer_->setCurrentIndex(qMax(0, printer_->findData(last.printer)));
    copies_->setValue(last.copies);
    (last.landscape ? landscape_ : portrait_)->setChecked(true);

    connect(printer_, &QComboBox::currentIndexChanged, this, [this]() {
        printSettings_->setEnabled(!printer_->currentData().toString().isEmpty());
        updatePreview();
    });
    connect(printSettings_, &QPushButton::clicked, this, [this]() {
        QPrinter printer;
        configurePrinter(printer, settings(), QString());
        QPrintDialog(&printer, this).exec();
    });
    connect(portrait_, &QPushButton::toggled, this, [this]() { updatePreview(); });
    connect(paperWhite_, &QCheckBox::toggled, this, [this]() { updatePreview(); });
    // Done and Print keep the settings as the new defaults; Cancel does not.
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(done, &QPushButton::clicked, this, [this]() {
        lastPrintSettings() = settings();
        accept();
    });
    connect(printButton, &QPushButton::clicked, this, &PrintDialog::print);
    printSettings_->setEnabled(!printer_->currentData().toString().isEmpty());
    updatePreview();
}

void PrintDialog::populatePrinters()
{
    for (const QPrinterInfo& info : QPrinterInfo::availablePrinters()) {
        printer_->addItem(info.printerName(), info.printerName());
    }
    // Linux / CUPS: with or without a queue, the job can be written as a PDF.
    printer_->addItem(QStringLiteral("Save as PDF"), QString());
}

PrintSettings PrintDialog::settings() const
{
    return {printer_->currentData().toString(), copies_->value(), landscape_->isChecked()};
}

void PrintDialog::setPdfPathProvider(std::function<QString()> provider)
{
    pdfPath_ = std::move(provider);
}

QSizeF PrintDialog::paperInches() const
{
    const QString name = printer_->currentData().toString();
    const QPageSize size = name.isEmpty() ? QPageSize(QPageSize::Letter)
                                          : QPrinterInfo::printerInfo(name).defaultPageSize();
    const QSizeF inches = size.size(QPageSize::Inch);
    return inches.isEmpty() ? QSizeF(8.5, 11.0) : inches;
}

void PrintDialog::updatePreview()
{
    QSizeF paper = paperInches();
    if (landscape_->isChecked()) {
        paper.transpose();
    }
    pageSize_->setText(QStringLiteral("%1 in x %2 in")
                           .arg(paper.width(), 0, 'g', 3)
                           .arg(paper.height(), 0, 'g', 3));
    const QSize page =
        paper.scaled(kPreviewWidth - 20, kPreviewHeight - 20, Qt::KeepAspectRatio).toSize();
    QImage sheet(page, QImage::Format_RGB32);
    sheet.fill(paperWhite_->isChecked() ? QColor(0xf5, 0xf5, 0xf0) : QColor(Qt::white));
    QPainter p(&sheet);
    if (!image_.isNull()) {
        const int margin = 10;
        QSize fitted = image_.size();
        fitted.scale(page.width() - 2 * margin, page.height() - 2 * margin, Qt::KeepAspectRatio);
        p.setRenderHint(QPainter::SmoothPixmapTransform);
        p.drawImage(QRect((page.width() - fitted.width()) / 2,
                          (page.height() - fitted.height()) / 2, fitted.width(), fitted.height()),
                    image_);
    }
    p.setPen(QColor(0x99, 0x99, 0x99));
    p.drawRect(0, 0, page.width() - 1, page.height() - 1);
    p.end();

    preview_ = QImage(kPreviewWidth, kPreviewHeight, QImage::Format_RGB32);
    preview_.fill(QColor(0x80, 0x80, 0x80));
    QPainter rp(&preview_);
    const QPoint at((kPreviewWidth - page.width()) / 2, (kPreviewHeight - page.height()) / 2);
    rp.fillRect(QRect(at + QPoint(2, 2), page), QColor(0x50, 0x50, 0x50));
    rp.drawImage(at, sheet);
    rp.end();
    previewLabel_->setPixmap(QPixmap::fromImage(preview_));
}

void PrintDialog::print()
{
    const PrintSettings chosen = settings();
    QString pdf;
    if (chosen.printer.isEmpty()) {
        pdf = pdfPath_ ? pdfPath_() : QString();
        if (pdf.isEmpty()) {
            return;
        }
    }
    QPrinter printer(QPrinter::HighResolution);
    configurePrinter(printer, chosen, pdf);
    if (!printImage(printer, image_)) {
        QMessageBox::warning(this, QStringLiteral("Print"),
                             QStringLiteral("The image could not be printed."));
        return;
    }
    lastPrintSettings() = chosen;
    accept();
}

QImage printableImage(PictureView* view)
{
    if (!view || !view->has_document()) {
        return QImage();
    }
    const int w = view->document_width();
    const int h = view->document_height();
    const ::rust::Vec<std::uint8_t> rgba = web_flattened(*view);
    if (rgba.size() != std::size_t(w) * std::size_t(h) * 4) {
        return QImage();
    }
    return QImage(rgba.data(), w, h, w * 4, QImage::Format_RGBA8888).copy();
}

void printFromView(QWidget* parent, PictureView* view)
{
    const QImage image = printableImage(view);
    if (!image.isNull()) {
        PrintDialog(image, parent).exec();
    }
}

void printOneCopyFromView(QWidget* parent, PictureView* view)
{
    const QImage image = printableImage(view);
    PrintSettings settings = lastPrintSettings();
    if (image.isNull()) {
        return;
    }
    // One copy to the last printer; Save as PDF (or none) needs the dialog.
    if (settings.printer.isEmpty() || QPrinterInfo::printerInfo(settings.printer).isNull()) {
        printFromView(parent, view);
        return;
    }
    settings.copies = 1;
    QPrinter printer(QPrinter::HighResolution);
    configurePrinter(printer, settings, QString());
    if (!printImage(printer, image)) {
        QMessageBox::warning(parent, QStringLiteral("Print One Copy"),
                             QStringLiteral("The image could not be printed."));
    }
}

} // namespace pictura

#include "moc_print_dialog.cpp"
