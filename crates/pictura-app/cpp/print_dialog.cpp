#include "print_dialog.h"

#include "api/engine_bridge.h"
#include "api/printing_bridge.h"
#include "print_job.h"
#include "proof_setup_dialog.h"

#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QSignalBlocker>
#include <QtGui/QPainter>
#include <QtPrintSupport/QPageSetupDialog>
#include <QtPrintSupport/QPrintDialog>
#include <QtPrintSupport/QPrinter>
#include <QtPrintSupport/QPrinterInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <cstdint>
#include <algorithm>
#include <initializer_list>

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

QString optionsJson(const QJsonObject& options)
{
    return QString::fromUtf8(QJsonDocument(options).toJson(QJsonDocument::Compact));
}

QString printFilename(PictureView* view)
{
    const QString name = view ? QFileInfo(view->file_path()).fileName() : QString();
    return name.isEmpty() ? QStringLiteral("Untitled") : name;
}

} // namespace

static QPageSize destinationPageSize(const QString& printerName)
{
    if (printerName.isEmpty()) {
        return QPageSize(QPageSize::Letter);
    }
    const QPageSize size = QPrinterInfo::printerInfo(printerName).defaultPageSize();
    return size.isValid() ? size : QPageSize(QPageSize::Letter);
}

void configurePrinter(QPrinter& printer, const PrintSettings& settings, const QString& pdfPath)
{
    if (settings.printer.isEmpty()) {
        printer.setOutputFormat(QPrinter::PdfFormat);
        printer.setOutputFileName(pdfPath);
        printer.setPageSize(settings.pageSize.isValid()
                                ? settings.pageSize
                                : QPageSize(QPageSize::Letter));
    } else {
        printer.setPrinterName(settings.printer);
        if (settings.pageSize.isValid()) {
            printer.setPageSize(settings.pageSize);
        }
    }
    printer.setPaperSource(settings.paperSource);
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
    : PrintDialog(image, nullptr, parent)
{
}

PrintDialog::PrintDialog(PictureView& view, QWidget* parent)
    : PrintDialog(printPreviewSource(view, kPreviewHeight), &view, parent)
{
}

PrintDialog::PrintDialog(const QImage& image, PictureView* view, QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , image_(image)
    , options_(resolvedPrintSettings(view).options)
    , filename_(printFilename(view))
    , description_(view ? print_document_description(*view) : QString())
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
    matchColors_ = new QCheckBox(QStringLiteral("Match Print Colors"), this);
    matchColors_->setObjectName(QStringLiteral("printMatchColors"));
    matchColors_->setChecked(options_.value(QStringLiteral("match")).toBool());
    left->addWidget(matchColors_);
    gamut_ = new QCheckBox(QStringLiteral("Gamut Warning"), this);
    gamut_->setObjectName(QStringLiteral("printGamutWarning"));
    gamut_->setChecked(options_.value(QStringLiteral("gamut")).toBool());
    left->addWidget(gamut_);
    paperWhite_ = new QCheckBox(QStringLiteral("Show Paper White"), this);
    paperWhite_->setObjectName(QStringLiteral("printPaperWhite"));
    paperWhite_->setChecked(options_.value(QStringLiteral("paperWhite")).toBool());
    left->addWidget(paperWhite_);
    left->addStretch();
    body->addLayout(left);

    // Right: Printer Setup and Color Management.
    auto* rightContent = new QWidget(this);
    auto* right = new QVBoxLayout(rightContent);
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
    printSettings_->setObjectName(QStringLiteral("printPageSetup"));
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
    cupsSetup_ = new QPushButton(QStringLiteral("Configure Managed CUPS Route…"), setup);
    setupLayout->addWidget(cupsSetup_);
    right->addWidget(setup);

    addColorControls(right);
    addOutputControls(right);
    right->addStretch();
    auto* scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    scroll->setWidget(rightContent);
    scroll->setMinimumWidth(400);
    body->addWidget(scroll, 1);
    outer->addLayout(body, 1);

    status_ = new QLabel(this);
    status_->setObjectName(QStringLiteral("printStatus"));
    status_->setWordWrap(true);
    status_->hide();
    outer->addWidget(status_);

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
    pageSetup_ = [this](QPrinter& printer) {
        if (printer.outputFormat() == QPrinter::PdfFormat) {
            QPageSetupDialog dialog(&printer, this);
            return dialog.exec() == QDialog::Accepted;
        }
        QPrintDialog dialog(&printer, this);
        return dialog.exec() == QDialog::Accepted;
    };
    const PrintSettings& last = lastPrintSettings();
    populatePrinters();
    const int printerIndex = printer_->findData(last.printer);
    printer_->setCurrentIndex(printerIndex >= 0 ? printerIndex : printer_->count() - 1);
    copies_->setValue(last.copies);
    (last.landscape ? landscape_ : portrait_)->setChecked(true);
    chosenPageSize_ = last.pageSize;
    paperSource_ = last.paperSource;

    connect(printer_, &QComboBox::currentIndexChanged, this, [this]() { updateOptions(); });
    connect(printSettings_, &QPushButton::clicked, this, [this]() {
        if (!pageSetup_) {
            return;
        }
        QPrinter printer;
        configurePrinter(printer, settings(), QString());
        if (!pageSetup_(printer)) {
            return;
        }
        const QPageSize size = printer.pageLayout().pageSize();
        if (size.isValid()) {
            chosenPageSize_ = size;
        }
        paperSource_ = printer.paperSource();
        updatePreview();
    });
    connect(portrait_, &QPushButton::toggled, this, [this]() { updatePreview(); });
    for (QCheckBox* check : {matchColors_, gamut_, paperWhite_}) {
        connect(check, &QCheckBox::toggled, this, [this]() { updateOptions(); });
    }
    connect(cupsSetup_, &QPushButton::clicked, this, [this]() {
        const auto chosen = settings();
        const auto profile = resolvePrintProfile(chosen.options);
        bool accepted = false;
        const QString input = QInputDialog::getMultiLineText(
            this, QStringLiteral("Validated CUPS Route"),
            QStringLiteral("Enter the device, filter chain, bypass evidence and CUPS options for "
                           "this route. Validate the route independently with the selected output "
                           "profile. Set orientation to pdf when the queue respects the oriented PDF, "
                           "or pdf-with-portrait-option when it requires a portrait job option. "
                           "Clear the form to revoke it after external changes."),
            QString::fromUtf8(QJsonDocument(managedPrintEvidence(chosen, profile)).toJson()), &accepted);
        if (accepted) {
            if (input.trimmed().isEmpty()) {
                setStatus(revokeManagedPrint(chosen, profile));
            } else {
                setStatus(enrollManagedPrint(chosen, profile,
                    QJsonDocument::fromJson(input.toUtf8()).object()));
            }
        }
    });
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(done, &QPushButton::clicked, this, [this]() {
        if (acceptSettings(settings(), true)) {
            accept();
        }
    });
    connect(printButton, &QPushButton::clicked, this, &PrintDialog::print);
    updateOptions();
    initialOptions_ = options_;
}

void PrintDialog::populatePrinters()
{
    for (const QPrinterInfo& info : QPrinterInfo::availablePrinters()) {
        printer_->addItem(info.printerName(), info.printerName());
    }
    // Linux / CUPS: with or without a queue, the job can be written as a PDF.
    printer_->addItem(QStringLiteral("Save as PDF"), QString());
}

void PrintDialog::populateProfiles()
{
    const QString path = options_.value(QStringLiteral("profilePath")).toString();
    const QString name = options_.value(QStringLiteral("profile")).toString();
    const QSignalBlocker blocker(profile_);
    profile_->clear();
    profile_->addItem(QStringLiteral("Choose an ICC profile"), QString());
    auto profiles = installedPrintProfiles();
    if (!path.isEmpty() && std::none_of(profiles.begin(), profiles.end(),
            [&](const PrintProfile& candidate) { return candidate.path == path; })) {
        const auto custom = describePrintProfile(path);
        if (custom.components != 0) {
            profiles.append(custom);
        }
    }
    for (const auto& candidate : profiles) {
        if (candidate.components != 0) {
            profile_->addItem(candidate.name, candidate.path);
        }
    }
    const auto selected = resolvePrintProfile(options_);
    if (selected.components != 0) {
        const int current = profile_->findData(selected.path);
        if (current >= 0) {
            profile_->setCurrentIndex(current);
            options_.insert(QStringLiteral("profilePath"), selected.path);
            options_.insert(QStringLiteral("components"), selected.components);
            return;
        }
    }
    if (!name.isEmpty()) {
        profile_->addItem(QStringLiteral("%1 (Unavailable or ambiguous)").arg(name), QString());
        profile_->setCurrentIndex(profile_->count() - 1);
    }
}

void PrintDialog::addColorControls(QVBoxLayout* layout)
{
    auto* color = new QGroupBox(QStringLiteral("Color Management"), this);
    auto* form = new QFormLayout(color);
    const QString name = view_ ? document_profile_name(*view_) : QString();
    form->addRow(QStringLiteral("Document Profile:"), new QLabel(
        name.isEmpty() ? QStringLiteral("sRGB IEC61966-2.1") : name, color));
    handling_ = new QComboBox(color);
    handling_->setObjectName(QStringLiteral("printColorHandling"));
    handling_->addItems({QStringLiteral("Printer Manages Colors"), QStringLiteral("Pictura Manages Colors")});
    handling_->setCurrentIndex(options_.value(QStringLiteral("managed")).toBool() ? 1 : 0);
    form->addRow(QStringLiteral("Color Handling:"), handling_);
    auto* profiles = new QWidget(color);
    auto* profileLayout = new QHBoxLayout(profiles);
    profileLayout->setContentsMargins(0, 0, 0, 0);
    profile_ = new QComboBox(profiles);
    profile_->setObjectName(QStringLiteral("printProfile"));
    profile_->setSizeAdjustPolicy(QComboBox::AdjustToMinimumContentsLengthWithIcon);
    profile_->setMinimumContentsLength(18);
    profileLayout->addWidget(profile_, 1);
    openProfile_ = new QPushButton(QStringLiteral("Open ICC…"), profiles);
    profileLayout->addWidget(openProfile_);
    form->addRow(QStringLiteral("Printer Profile:"), profiles);
    intent_ = new QComboBox(color);
    intent_->setObjectName(QStringLiteral("printIntent"));
    intent_->addItems({QStringLiteral("Perceptual"), QStringLiteral("Relative Colorimetric"),
                       QStringLiteral("Saturation"), QStringLiteral("Absolute Colorimetric")});
    intent_->setCurrentIndex(qBound(0, options_.value(QStringLiteral("intent")).toInt(1), 3));
    form->addRow(QStringLiteral("Rendering Intent:"), intent_);
    blackPoint_ = new QCheckBox(QStringLiteral("Black Point Compensation"), color);
    blackPoint_->setChecked(options_.value(QStringLiteral("bpc")).toBool(true));
    form->addRow(blackPoint_);
    hardProof_ = new QCheckBox(QStringLiteral("Hard Proofing"), color);
    hardProof_->setObjectName(QStringLiteral("printHardProof"));
    hardProof_->setChecked(options_.value(QStringLiteral("hardProof")).toBool());
    hardProof_->setToolTip(QStringLiteral("Requires Pictura Manages Colors and a valid printer profile."));
    proofSetup_ = new QPushButton(QStringLiteral("Proof Setup…"), color);
    proofSetup_->setObjectName(QStringLiteral("printProofSetup"));
    form->addRow(hardProof_, proofSetup_);
    colorHint_ = new QLabel(color);
    colorHint_->setWordWrap(true);
    form->addRow(colorHint_);
    layout->addWidget(color);
    populateProfiles();
    connect(profile_, &QComboBox::currentIndexChanged, this, [this]() {
        const auto candidate = describePrintProfile(profile_->currentData().toString());
        options_.insert(QStringLiteral("profile"), candidate.name);
        options_.insert(QStringLiteral("profilePath"), candidate.path);
        options_.insert(QStringLiteral("components"), candidate.components);
        updateOptions();
    });
    connect(openProfile_, &QPushButton::clicked, this, [this]() {
        const QString path = QFileDialog::getOpenFileName(this, QStringLiteral("Open Printer ICC Profile"),
            {}, QStringLiteral("ICC Profiles (*.icc *.icm)"));
        if (path.isEmpty()) {
            return;
        }
        const auto candidate = describePrintProfile(path);
        options_.insert(QStringLiteral("profile"), candidate.name.isEmpty() ? QFileInfo(path).fileName() : candidate.name);
        options_.insert(QStringLiteral("profilePath"), path);
        options_.insert(QStringLiteral("components"), candidate.components);
        populateProfiles();
        updateOptions();
        if (candidate.components == 0) {
            setStatus(QStringLiteral("The selected ICC profile cannot be used for printing."));
        }
    });
    connect(handling_, &QComboBox::currentIndexChanged, this, [this]() { updateOptions(); });
    connect(intent_, &QComboBox::currentIndexChanged, this, [this]() { updateOptions(); });
    connect(blackPoint_, &QCheckBox::toggled, this, [this]() { updateOptions(); });
    connect(hardProof_, &QCheckBox::toggled, this, [this]() { updateOptions(); });
    connect(proofSetup_, &QPushButton::clicked, this, [this]() {
        QStringList names{QStringLiteral("proofCMYK"), QStringLiteral("Working Gray"),
                          QStringLiteral("sRGB IEC61966-2.1"), QStringLiteral("Adobe RGB (1998)"),
                          QStringLiteral("ProPhoto RGB")};
        QStringList paths{QStringLiteral("builtin:0"), QStringLiteral("builtin:1"),
                          QStringLiteral("builtin:2"), QStringLiteral("builtin:3"),
                          QStringLiteral("builtin:4")};
        QStringList extra;
        for (const auto& profile : installedPrintProfiles()) {
            if (!profile.path.startsWith(QStringLiteral("builtin:"))) {
                names.append(profile.name);
                paths.append(profile.path);
                extra.append(profile.name);
            }
        }
        const auto current = options_.value(QStringLiteral("proof")).toObject();
        const QString currentName = current.value(QStringLiteral("name")).toString();
        const QString currentPath = current.value(QStringLiteral("path")).toString();
        int selectedDevice = int(paths.indexOf(currentPath));
        if (selectedDevice < 0) {
            selectedDevice = int(names.indexOf(currentName));
        }
        if (selectedDevice < 0 && !currentName.isEmpty()) {
            selectedDevice = int(names.size());
            names.append(currentName);
            paths.append(currentPath);
            extra.append(currentName);
        }
        ProofSetupValues values;
        values.device = qMax(0, selectedDevice);
        values.intent = current.value(QStringLiteral("intent")).toInt(1);
        values.bpc = current.value(QStringLiteral("bpc")).toBool(true);
        values.simulatePaper = current.value(QStringLiteral("paper")).toBool();
        values.simulateBlack = current.value(QStringLiteral("black")).toBool();
        ProofSetupDialog dialog(values, this);
        dialog.addPrintProfiles(extra, values.device);
        if (dialog.exec() == QDialog::Accepted) {
            const auto selected = dialog.values();
            const bool builtinCmyk = selected.device == 0 && !selected.simulatePaper
                && selected.intent == 1 && selected.bpc;
            const QString path = paths.value(selected.device);
            const QString name = selected.device == 0 && !builtinCmyk
                ? describePrintProfile(path).name : names.value(selected.device);
            options_.insert(QStringLiteral("proof"), QJsonObject{
                {QStringLiteral("name"), name}, {QStringLiteral("path"), path},
                {QStringLiteral("kind"), builtinCmyk ? QStringLiteral("builtin") : QStringLiteral("named")},
                {QStringLiteral("intent"), selected.intent}, {QStringLiteral("bpc"), selected.bpc},
                {QStringLiteral("paper"), selected.simulatePaper}, {QStringLiteral("black"), selected.simulateBlack}});
            updateOptions();
        }
    });
}

void PrintDialog::addOutputControls(QVBoxLayout* layout)
{
    auto* output = new QGroupBox(QStringLiteral("Output"), this);
    auto* form = new QFormLayout(output);
    const auto addToggle = [&](const char* key, const QString& label) {
        auto* control = new QCheckBox(label, output);
        const QString field = QLatin1String(key);
        control->setObjectName(QStringLiteral("print_") + field);
        const bool checked = options_.value(field).toBool();
        control->setChecked(checked);
        options_.insert(field, checked);
        form->addRow(control);
        connect(control, &QCheckBox::toggled, this, [this, field](bool enabled) {
            options_.insert(field, enabled);
            updatePreview();
        });
    };
    addToggle("calibration", QStringLiteral("Calibration Bars"));
    addToggle("registration", QStringLiteral("Registration Marks"));
    addToggle("cornerCrop", QStringLiteral("Corner Crop Marks"));
    addToggle("centerCrop", QStringLiteral("Center Crop Marks"));
    addToggle("filename", QStringLiteral("Labels"));
    addToggle("description", QStringLiteral("Description"));
    auto* background = new QPushButton(QStringLiteral("Choose…"), output);
    background->setObjectName(QStringLiteral("printBackground"));
    form->addRow(QStringLiteral("Background:"), background);
    connect(background, &QPushButton::clicked, this, [this]() {
        const auto rgb = options_.value(QStringLiteral("background")).toArray();
        const QColor previous = rgb.size() == 3
            ? QColor(rgb.at(0).toInt(), rgb.at(1).toInt(), rgb.at(2).toInt()) : QColor(Qt::white);
        const QColor selected = QColorDialog::getColor(previous, this, QStringLiteral("Print Background"));
        if (selected.isValid()) {
            options_.insert(QStringLiteral("background"), QJsonArray{selected.red(), selected.green(), selected.blue()});
            updatePreview();
        }
    });
    const auto addWidth = [&](const QString& field, const QString& label) {
        auto* control = new QDoubleSpinBox(output);
        control->setObjectName(QStringLiteral("print_") + field);
        control->setRange(0, 144);
        control->setSuffix(QStringLiteral(" pt"));
        control->setValue(options_.value(field).toDouble());
        options_.insert(field, control->value());
        form->addRow(label, control);
        connect(control, &QDoubleSpinBox::valueChanged, this, [this, field](double value) {
            options_.insert(field, value);
            updatePreview();
        });
    };
    addWidth(QStringLiteral("border"), QStringLiteral("Border:"));
    addWidth(QStringLiteral("bleed"), QStringLiteral("Bleed:"));
    layout->addWidget(output);
}

void PrintDialog::updateOptions()
{
    const bool managed = handling_->currentIndex() == 1;
    const bool valid = profile_->currentIndex() > 0 && !profile_->currentData().toString().isEmpty()
        && options_.value(QStringLiteral("components")).toInt() != 0;
    for (QWidget* control : std::initializer_list<QWidget*>{profile_, openProfile_, intent_, blackPoint_}) {
        control->setEnabled(managed);
    }
    hardProof_->setEnabled(managed && valid);
    proofSetup_->setEnabled(managed && valid && hardProof_->isChecked());
    matchColors_->setEnabled(managed && valid);
    gamut_->setEnabled(managed && valid && matchColors_->isChecked());
    cupsSetup_->setEnabled(managed && valid && !printer_->currentData().toString().isEmpty());
    colorHint_->setText(managed
        ? QStringLiteral("Disable printer-driver color management for application-managed output.")
        : QStringLiteral("The printer handles color. Hard Proofing requires Pictura Manages Colors."));
    options_.insert(QStringLiteral("managed"), managed);
    options_.insert(QStringLiteral("intent"), intent_->currentIndex());
    options_.insert(QStringLiteral("bpc"), blackPoint_->isChecked());
    options_.insert(QStringLiteral("hardProof"), hardProof_->isChecked());
    options_.insert(QStringLiteral("match"), matchColors_->isChecked());
    options_.insert(QStringLiteral("gamut"), gamut_->isChecked() && gamut_->isEnabled());
    options_.insert(QStringLiteral("paperWhite"), paperWhite_->isChecked());
    updatePreview();
}

PrintSettings PrintDialog::settings() const
{
    return {printer_->currentData().toString(), copies_->value(), landscape_->isChecked(),
            chosenPageSize_, paperSource_, options_};
}

void PrintDialog::setPdfPathProvider(std::function<QString()> provider)
{
    pdfPath_ = std::move(provider);
}

void PrintDialog::setPageSetupProvider(std::function<bool(QPrinter&)> provider)
{
    pageSetup_ = std::move(provider);
}

void PrintDialog::updatePreview()
{
    const auto chosen = settings();
    const auto paper = physicalPrintPage(chosen);
    const QSizeF inches = paper.fullRect(QPageLayout::Inch).size();
    pageSize_->setText(QStringLiteral("%1 in x %2 in")
                           .arg(inches.width(), 0, 'g', 3)
                           .arg(inches.height(), 0, 'g', 3));
    preview_ = QImage(kPreviewWidth, kPreviewHeight, QImage::Format_RGB32);
    preview_.fill(QColor(0x80, 0x80, 0x80));
    const QSizeF imageSize = view_ ? QSizeF(view_->document_width(), view_->document_height()) : QSizeF(image_.size());
    const auto page = layoutPrintPage(paper, imageSize, chosen.options, filename_, description_);
    if (!page.paper.isEmpty()) {
        const QSizeF previewPage = page.paper.scaled(kPreviewWidth - 20, kPreviewHeight - 20, Qt::KeepAspectRatio);
        const QPointF at((kPreviewWidth - previewPage.width()) / 2,
                         (kPreviewHeight - previewPage.height()) / 2);
        QString warning;
        const auto image = view_ ? proofDocumentPrintPreview(*view_, image_, chosen, warning)
                                 : proofPrintPreview(image_, chosen, warning);
        QPainter painter(&preview_);
        painter.fillRect(QRectF(at + QPointF(2, 2), previewPage), QColor(0x50, 0x50, 0x50));
        painter.translate(at);
        painter.scale(previewPage.width() / page.paper.width(), previewPage.height() / page.paper.height());
        paintPrintPage(painter, page, image, printPaperAppearance(chosen));
        setStatus(warning);
    }
    previewLabel_->setPixmap(QPixmap::fromImage(preview_));
}

void PrintDialog::setStatus(const QString& message)
{
    status_->setText(message);
    status_->setVisible(!message.isEmpty());
}

bool PrintDialog::acceptSettings(const PrintSettings& chosen, bool commit)
{
    if (view_) {
        QJsonObject before = initialOptions_;
        if (!before.value(QStringLiteral("managed")).toBool()
            && chosen.options.value(QStringLiteral("managed")).toBool()) {
            for (const char* field : {"profile", "components", "intent", "bpc"}) {
                before.remove(QLatin1String(field));
            }
        }
        if (!before.value(QStringLiteral("hardProof")).toBool()
            && chosen.options.value(QStringLiteral("hardProof")).toBool()) {
            before.remove(QStringLiteral("proof"));
        }
        const QString error = print_document_settings_update(
            *view_, optionsJson(before), optionsJson(chosen.options), commit);
        if (!error.isEmpty()) {
            setStatus(error);
            return false;
        }
    }
    if (commit && !storePrintSettings(chosen)) {
        QMessageBox::warning(this, QStringLiteral("Print"),
                             QStringLiteral("The print defaults could not be saved for the next launch."));
    }
    return true;
}

void PrintDialog::print()
{
    const PrintSettings chosen = settings();
    if (!acceptSettings(chosen, false)) {
        return;
    }
    const QString preflight = printJobPreflight(chosen);
    if (!preflight.isEmpty()) {
        setStatus(preflight);
        return;
    }
    QString pdf;
    if (chosen.printer.isEmpty()) {
        pdf = pdfPath_ ? pdfPath_() : QString();
        if (pdf.isEmpty()) {
            return;
        }
    }
    const QString error = submitPrintJob(chosen, view_, image_, pdf, filename_, description_);
    if (!error.isEmpty()) {
        setStatus(error);
        return;
    }
    acceptSettings(chosen, true);
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
    if (view && view->has_document()) {
        PrintDialog(*view, parent).exec();
    }
}

void printOneCopyFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return;
    }
    const PrintSettings settings = resolvedPrintSettings(view);
    const bool available = !settings.printer.isEmpty() && !QPrinterInfo::printerInfo(settings.printer).isNull();
    routePrintOneCopy(settings, available, printJobPreflight, [parent, view](const PrintSettings& chosen) {
        const QString error = submitPrintJob(chosen, view, {}, {}, printFilename(view),
                                             print_document_description(*view));
        if (!error.isEmpty()) {
            QMessageBox::warning(parent, QStringLiteral("Print One Copy"), error);
        }
    }, [parent, view](const QString& reason) {
        PrintDialog dialog(*view, parent);
        dialog.setStatus(reason);
        dialog.exec();
    });
}

void routePrintOneCopy(PrintSettings settings, bool available,
                       const std::function<QString(const PrintSettings&)>& check,
                       const std::function<void(const PrintSettings&)>& submit,
                       const std::function<void(const QString&)>& openDialog)
{
    if (settings.printer.isEmpty() || !available) {
        openDialog({});
        return;
    }
    settings.copies = 1;
    const QString preflight = check(settings);
    if (!preflight.isEmpty()) {
        openDialog(preflight);
        return;
    }
    submit(settings);
}

} // namespace pictura

#include "moc_print_dialog.cpp"
