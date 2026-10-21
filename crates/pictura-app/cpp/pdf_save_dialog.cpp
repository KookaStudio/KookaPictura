#include "pdf_save_dialog.h"

#include "session.h"

#include "pictura_app/src/cxxqt_object/pdf_export.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QSignalBlocker>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

const QString kModified = QStringLiteral(" (Modified)");

// Fields a preset never carries: switching presets keeps them.
const char* const kKeptFields[] = {
    "layers",          "notes",       "alphaChannels",      "iccProfile",
    "openPassword",    "permissionsPassword", "printing",  "changes",
    "enableCopying",   "enableAccessibility", "plaintextMetadata",
};

QString field(const QString& options, const char* name)
{
    return pdf_option(options, QString::fromLatin1(name));
}

QStringList builtinPresets()
{
    return pdf_builtin_presets().split(QLatin1Char('\n'), Qt::SkipEmptyParts);
}

} // namespace

QString PdfSaveDialog::presetsDirectory()
{
    return QFileInfo(sessionFilePath()).absolutePath() + QStringLiteral("/PDF Presets");
}

QStringList PdfSaveDialog::userPresetNames(const QString& dir)
{
    QStringList names;
    const QFileInfoList files =
        QDir(dir).entryInfoList({QStringLiteral("*.joboptions")}, QDir::Files, QDir::Name);
    for (const QFileInfo& file : files) {
        names << file.completeBaseName();
    }
    return names;
}

QString PdfSaveDialog::presetOptions(const QString& name, const QString& dir)
{
    if (builtinPresets().contains(name)) {
        return pdf_builtin_preset(name);
    }
    QFile file(dir + QLatin1Char('/') + name + QStringLiteral(".joboptions"));
    if (!file.open(QIODevice::ReadOnly)) {
        return QString();
    }
    return pdf_parse_joboptions(QString::fromUtf8(file.readAll()));
}

PdfSaveDialog::PdfSaveDialog(const QString& options, const QString& presetsDir, Mode mode,
                             QWidget* parent)
    : QDialog(parent)
    , options_(options.isEmpty() ? pdf_default_options() : options)
    , presetsDir_(presetsDir)
    , mode_(mode)
{
    setObjectName(QStringLiteral("pdfSaveDialog"));
    setWindowTitle(mode == Mode::Save ? tr("Save Adobe PDF") : tr("Edit PDF Preset"));
    auto* layout = new QVBoxLayout(this);

    auto* header = new QFormLayout();
    presetCombo_ = new QComboBox(this);
    presetCombo_->setObjectName(QStringLiteral("pdfPreset"));
    presetCombo_->addItems(builtinPresets());
    presetCombo_->addItems(userPresetNames(presetsDir_));
    header->addRow(tr("Adobe PDF Preset:"), presetCombo_);
    header->addRow(tr("Standard:"),
                   combo(QStringLiteral("standard"),
                         {tr("None"), tr("PDF/X-1a:2001"), tr("PDF/X-3:2002"),
                          tr("PDF/X-4:2008"), tr("PDF/A-1b:2005")},
                         this));
    header->addRow(tr("Compatibility:"),
                   combo(QStringLiteral("compatibility"),
                         {tr("Acrobat 4 (PDF 1.3)"), tr("Acrobat 5 (PDF 1.4)"),
                          tr("Acrobat 6 (PDF 1.5)"), tr("Acrobat 7 (PDF 1.6)"),
                          tr("Acrobat X (PDF 1.7)")},
                         this));
    layout->addLayout(header);

    auto* body = new QHBoxLayout();
    pageList_ = new QListWidget(this);
    pageList_->setObjectName(QStringLiteral("pdfPages"));
    pageList_->setFixedWidth(140);
    stack_ = new QStackedWidget(this);
    stack_->setObjectName(QStringLiteral("pdfStack"));
    const QList<QPair<QString, QWidget*>> pages = {
        {tr("General"), buildGeneralPage()},
        {tr("Compression"), buildCompressionPage()},
        {tr("Output"), buildOutputPage()},
        {tr("Security"), buildSecurityPage()},
        {tr("Summary"), buildSummaryPage()},
    };
    for (const auto& [name, page] : pages) {
        pageList_->addItem(name);
        stack_->addWidget(page);
    }
    connect(pageList_, &QListWidget::currentRowChanged, stack_, &QStackedWidget::setCurrentIndex);
    pageList_->setCurrentRow(0);
    body->addWidget(pageList_);
    body->addWidget(stack_, 1);
    layout->addLayout(body, 1);

    auto* buttons = new QDialogButtonBox(this);
    auto* savePreset = buttons->addButton(tr("Save Preset…"), QDialogButtonBox::ResetRole);
    savePreset->setObjectName(QStringLiteral("pdfSavePreset"));
    buttons->addButton(mode == Mode::Save ? tr("Save PDF") : tr("OK"),
                       QDialogButtonBox::AcceptRole);
    buttons->addButton(QDialogButtonBox::Cancel);
    connect(savePreset, &QPushButton::clicked, this, &PdfSaveDialog::promptSavePreset);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);

    if (mode_ == Mode::PresetEditor) {
        saveOptions_->hide();
        savePreset->hide();
        presetCombo_->setEnabled(false);
    }

    // Name the preset these options came from, else show a Modified
    // [Photoshop Default].
    int origin = 0;
    for (int i = 0; i < presetCombo_->count(); ++i) {
        if (pdf_same_preset(options_, presetOptions(presetCombo_->itemText(i), presetsDir_))) {
            origin = i;
            break;
        }
    }
    setPreset(origin, presetOptions(presetCombo_->itemText(origin), presetsDir_));
    connect(presetCombo_, &QComboBox::activated, this, &PdfSaveDialog::applyPreset);
    refresh();
}

QComboBox* PdfSaveDialog::combo(const QString& name, const QStringList& items, QWidget* parent)
{
    auto* box = new QComboBox(parent);
    box->setObjectName(QStringLiteral("pdf_") + name);
    box->addItems(items);
    bind(name, box);
    connect(box, &QComboBox::activated, this,
            [this, name](int index) { setField(name, QString::number(index)); });
    return box;
}

void PdfSaveDialog::bind(const QString& name, QWidget* widget)
{
    if (widget->objectName().isEmpty()) {
        widget->setObjectName(QStringLiteral("pdf_") + name);
    }
    bindings_.append({name, widget});
    if (auto* check = qobject_cast<QCheckBox*>(widget)) {
        connect(check, &QCheckBox::toggled, this,
                [this, name](bool on) { setField(name, on ? QStringLiteral("1") : QStringLiteral("0")); });
    } else if (auto* line = qobject_cast<QLineEdit*>(widget)) {
        connect(line, &QLineEdit::textEdited, this,
                [this, name](const QString& text) { setField(name, text); });
    } else if (auto* number = qobject_cast<QDoubleSpinBox*>(widget)) {
        connect(number, &QDoubleSpinBox::valueChanged, this,
                [this, name](double value) { setField(name, QString::number(value)); });
    } else if (auto* integer = qobject_cast<QSpinBox*>(widget)) {
        connect(integer, &QSpinBox::valueChanged, this,
                [this, name](int value) { setField(name, QString::number(value)); });
    } else if (auto* text = qobject_cast<QPlainTextEdit*>(widget)) {
        connect(text, &QPlainTextEdit::textChanged, this,
                [this, name, text]() { setField(name, text->toPlainText()); });
    }
}

QWidget* PdfSaveDialog::buildGeneralPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("pdfGeneralPage"));
    auto* layout = new QVBoxLayout(page);
    saveOptions_ = new QGroupBox(tr("Save Options"), page);
    auto* saveLayout = new QVBoxLayout(saveOptions_);
    const QList<QPair<QString, QString>> saveChecks = {
        {QStringLiteral("layers"), tr("Layers")},
        {QStringLiteral("notes"), tr("Notes")},
        {QStringLiteral("alphaChannels"), tr("Alpha Channels")},
        {QStringLiteral("spotColors"), tr("Spot Colors")},
        {QStringLiteral("iccProfile"), tr("ICC Profile")},
    };
    for (const auto& [name, label] : saveChecks) {
        auto* check = new QCheckBox(label, saveOptions_);
        bind(name, check);
        saveLayout->addWidget(check);
    }
    // ponytail: the model has no spot channels and the app no Proof Setup yet.
    fieldWidgetForTest(QStringLiteral("spotColors"))
        ->setToolTip(tr("Spot channels are not modelled yet."));
    auto* proof = new QCheckBox(tr("Use Proof Setup: Working CMYK"), saveOptions_);
    proof->setObjectName(QStringLiteral("pdfUseProofSetup"));
    proof->setEnabled(false);
    proof->setToolTip(tr("Proof Setup is not available yet."));
    saveLayout->addWidget(proof);
    layout->addWidget(saveOptions_);

    layout->addWidget(new QLabel(tr("Description:"), page));
    auto* description = new QPlainTextEdit(page);
    description->setMaximumHeight(80);
    bind(QStringLiteral("description"), description);
    layout->addWidget(description);

    auto* options = new QGroupBox(tr("Options"), page);
    auto* optionsLayout = new QVBoxLayout(options);
    const QList<QPair<QString, QString>> checks = {
        {QStringLiteral("preserveEditing"), tr("Preserve Photoshop Editing Capabilities")},
        {QStringLiteral("embedThumbnails"), tr("Embed Page Thumbnails")},
        {QStringLiteral("fastWebView"), tr("Optimize For Fast Web View")},
        {QStringLiteral("viewAfterSaving"), tr("View PDF After Saving")},
    };
    for (const auto& [name, label] : checks) {
        auto* check = new QCheckBox(label, options);
        bind(name, check);
        optionsLayout->addWidget(check);
    }
    layout->addWidget(options);
    layout->addStretch(1);
    return page;
}

QWidget* PdfSaveDialog::buildCompressionPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("pdfCompressionPage"));
    auto* form = new QFormLayout(page);
    form->addRow(tr("Downsampling:"),
                 combo(QStringLiteral("downsample"),
                       {tr("Do Not Downsample"), tr("Average Downsampling To"),
                        tr("Subsampling To"), tr("Bicubic Downsampling To")},
                       page));
    for (const auto& [name, label] :
         QList<QPair<QString, QString>>{{QStringLiteral("downsamplePpi"), tr("Resolution:")},
                                        {QStringLiteral("downsampleAbovePpi"),
                                         tr("For Images Above:")}}) {
        auto* ppi = new QDoubleSpinBox(page);
        ppi->setRange(9, 4800);
        ppi->setDecimals(0);
        ppi->setSuffix(tr(" ppi"));
        bind(name, ppi);
        form->addRow(label, ppi);
    }
    auto* compression = combo(QStringLiteral("compression"),
                              {tr("None"), tr("ZIP"), tr("JPEG"), tr("JPEG 2000")}, page);
    if (auto* model = qobject_cast<QStandardItemModel*>(compression->model())) {
        // ponytail: no JPEG 2000 encoder in Qt or the dependency tree.
        model->item(3)->setEnabled(false);
        model->item(3)->setToolTip(tr("JPEG 2000 encoding is not available yet."));
    }
    form->addRow(tr("Compression:"), compression);
    form->addRow(tr("Image Quality:"),
                 combo(QStringLiteral("quality"),
                       {tr("Minimum"), tr("Low"), tr("Medium"), tr("High"), tr("Maximum")},
                       page));
    auto* tile = new QSpinBox(page);
    tile->setRange(128, 2048);
    bind(QStringLiteral("tileSize"), tile);
    form->addRow(tr("Tile Size:"), tile);
    auto* convert = new QCheckBox(tr("Convert 16 Bit/Channel Image To 8 Bits/Channel"), page);
    bind(QStringLiteral("convert16To8"), convert);
    form->addRow(convert);
    return page;
}

QWidget* PdfSaveDialog::buildOutputPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("pdfOutputPage"));
    auto* form = new QFormLayout(page);
    form->addRow(tr("Color Conversion:"),
                 combo(QStringLiteral("colorConversion"),
                       {tr("No Conversion"), tr("Convert To Destination")}, page));
    form->addRow(tr("Destination:"),
                 combo(QStringLiteral("destination"),
                       {tr("sRGB IEC61966-2.1"), tr("Adobe RGB (1998)"), tr("ProPhoto RGB"),
                        tr("Gray Gamma 2.2"), tr("Photocraft Coated CMYK")},
                       page));
    form->addRow(tr("Profile Inclusion Policy:"),
                 combo(QStringLiteral("profileInclusion"),
                       {tr("Include Destination Profile"), tr("Don't Include Profiles")}, page));
    for (const auto& [name, label] :
         QList<QPair<QString, QString>>{
             {QStringLiteral("outputIntentProfile"), tr("Output Intent Profile Name:")},
             {QStringLiteral("outputCondition"), tr("Output Condition:")},
             {QStringLiteral("outputConditionIdentifier"), tr("Output Condition Identifier:")},
             {QStringLiteral("registryName"), tr("Registry Name:")}}) {
        auto* line = new QLineEdit(page);
        bind(name, line);
        form->addRow(label, line);
    }
    return page;
}

QWidget* PdfSaveDialog::buildSecurityPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("pdfSecurityPage"));
    auto* form = new QFormLayout(page);
    for (const auto& [name, label] :
         QList<QPair<QString, QString>>{
             {QStringLiteral("openPassword"), tr("Require a password to open the document")},
             {QStringLiteral("permissionsPassword"),
              tr("Use a password to restrict editing security and permissions")}}) {
        auto* require = new QCheckBox(label, page);
        require->setObjectName(QStringLiteral("pdfRequire_") + name);
        auto* password = new QLineEdit(page);
        password->setEchoMode(QLineEdit::Password);
        bind(name, password);
        connect(require, &QCheckBox::toggled, this, [this, name, password](bool on) {
            password->setEnabled(on);
            if (!on) {
                setField(name, QString());
            }
        });
        form->addRow(require);
        form->addRow(tr("Password:"), password);
    }
    form->addRow(tr("Printing Allowed:"),
                 combo(QStringLiteral("printing"),
                       {tr("None"), tr("Low Resolution (150 dpi)"), tr("High Resolution")}, page));
    form->addRow(tr("Changes Allowed:"),
                 combo(QStringLiteral("changes"),
                       {tr("None"), tr("Inserting, deleting, and rotating pages"),
                        tr("Filling in form fields, and signing existing signature fields"),
                        tr("Commenting, filling in form fields, and signing existing signature "
                           "fields"),
                        tr("Any except extracting pages")},
                       page));
    for (const auto& [name, label] :
         QList<QPair<QString, QString>>{
             {QStringLiteral("enableCopying"), tr("Enable copying of text, images, and other content")},
             {QStringLiteral("enableAccessibility"),
              tr("Enable text access for screen reader devices for the visually impaired")},
             {QStringLiteral("plaintextMetadata"), tr("Enable plaintext metadata")}}) {
        auto* check = new QCheckBox(label, page);
        bind(name, check);
        form->addRow(check);
    }
    encryptionLevel_ = new QLabel(page);
    encryptionLevel_->setObjectName(QStringLiteral("pdfEncryptionLevel"));
    form->addRow(encryptionLevel_);
    return page;
}

QWidget* PdfSaveDialog::buildSummaryPage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("pdfSummaryPage"));
    auto* layout = new QVBoxLayout(page);
    summary_ = new QPlainTextEdit(page);
    summary_->setObjectName(QStringLiteral("pdfSummary"));
    summary_->setReadOnly(true);
    layout->addWidget(summary_);
    return page;
}

void PdfSaveDialog::setField(const QString& name, const QString& value)
{
    if (refreshing_) {
        return;
    }
    options_ = pdf_set_option(options_, name, value);
    refresh();
}

void PdfSaveDialog::refresh()
{
    refreshing_ = true;
    for (const Binding& binding : bindings_) {
        const QString value = pdf_option(options_, binding.field);
        const bool locked = pdf_option_locked(options_, binding.field);
        const QSignalBlocker blocker(binding.widget);
        if (auto* check = qobject_cast<QCheckBox*>(binding.widget)) {
            check->setChecked(value == QStringLiteral("1"));
        } else if (auto* box = qobject_cast<QComboBox*>(binding.widget)) {
            box->setCurrentIndex(value.toInt());
        } else if (auto* line = qobject_cast<QLineEdit*>(binding.widget)) {
            if (line->text() != value) {
                line->setText(value);
            }
            // A password field also follows its "Require a password" box.
            if (auto* require = findChild<QCheckBox*>(QStringLiteral("pdfRequire_") + binding.field)) {
                const QSignalBlocker requireBlocker(require);
                require->setEnabled(!locked);
                if (!value.isEmpty()) {
                    require->setChecked(true);
                } else if (locked) {
                    require->setChecked(false);
                }
                line->setEnabled(!locked && require->isChecked());
                continue;
            }
        } else if (auto* number = qobject_cast<QDoubleSpinBox*>(binding.widget)) {
            number->setValue(value.toDouble());
        } else if (auto* integer = qobject_cast<QSpinBox*>(binding.widget)) {
            integer->setValue(value.toInt());
        } else if (auto* text = qobject_cast<QPlainTextEdit*>(binding.widget)) {
            if (text->toPlainText() != value) {
                text->setPlainText(value);
            }
        }
        binding.widget->setEnabled(!locked);
    }
    encryptionLevel_->setText(tr("Encryption Level: %1").arg(field(options_, "encryptionLevel")));
    summary_->setPlainText(pdf_summary(options_));
    const QSignalBlocker blocker(presetCombo_);
    const bool modified = !pdf_same_preset(options_, presetBaseline_);
    presetCombo_->setItemText(presetIndex_, modified ? presetName_ + kModified : presetName_);
    presetCombo_->setCurrentIndex(presetIndex_);
    refreshing_ = false;
}

// Make row `index` the origin preset, restoring the previous row's plain name.
void PdfSaveDialog::setPreset(int index, const QString& options)
{
    if (!presetName_.isEmpty()) {
        presetCombo_->setItemText(presetIndex_, presetName_);
    }
    presetIndex_ = index;
    presetName_ = presetCombo_->itemText(index);
    presetBaseline_ = options;
}

void PdfSaveDialog::applyPreset(int index)
{
    const QString label = presetCombo_->itemText(index);
    const QString name = label.endsWith(kModified) ? label.chopped(kModified.size()) : label;
    const QString preset = presetOptions(name, presetsDir_);
    if (preset.isEmpty()) {
        return;
    }
    QString next = preset;
    for (const char* kept : kKeptFields) {
        const QString key = QString::fromLatin1(kept);
        next = pdf_set_option(next, key, pdf_option(options_, key));
    }
    options_ = next;
    presetCombo_->setItemText(index, name);
    setPreset(index, preset);
    refresh();
}

bool PdfSaveDialog::savePresetAs(const QString& name)
{
    const QString trimmed = name.trimmed();
    if (trimmed.isEmpty() || builtinPresets().contains(trimmed)
        || trimmed.contains(QLatin1Char('/'))) {
        return false;
    }
    if (!QDir().mkpath(presetsDir_)) {
        return false;
    }
    QFile file(presetsDir_ + QLatin1Char('/') + trimmed + QStringLiteral(".joboptions"));
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)
        || file.write(pdf_preset_text(options_).toUtf8()) < 0) {
        return false;
    }
    file.close();
    int index = presetCombo_->findText(trimmed);
    if (index < 0) {
        presetCombo_->addItem(trimmed);
        index = presetCombo_->count() - 1;
    }
    setPreset(index, presetOptions(trimmed, presetsDir_));
    refresh();
    return true;
}

void PdfSaveDialog::promptSavePreset()
{
    bool ok = false;
    const QString name = QInputDialog::getText(this, tr("Save Adobe PDF Preset"),
                                               tr("Save Preset As:"), QLineEdit::Normal,
                                               presetName_, &ok);
    if (ok) {
        savePresetAs(name);
    }
}

QStringList PdfSaveDialog::pageNames() const
{
    QStringList names;
    for (int i = 0; i < pageList_->count(); ++i) {
        names << pageList_->item(i)->text();
    }
    return names;
}

QString PdfSaveDialog::currentPage() const
{
    return pageList_->currentItem() ? pageList_->currentItem()->text() : QString();
}

void PdfSaveDialog::showPage(const QString& name)
{
    const int row = pageNames().indexOf(name);
    if (row >= 0) {
        pageList_->setCurrentRow(row);
    }
}

QStringList PdfSaveDialog::presetNames() const
{
    QStringList names;
    for (int i = 0; i < presetCombo_->count(); ++i) {
        names << presetCombo_->itemText(i);
    }
    return names;
}

bool PdfSaveDialog::selectPreset(const QString& name)
{
    int index = presetCombo_->findText(name);
    if (index < 0) {
        index = presetCombo_->findText(name + kModified);
    }
    if (index < 0) {
        return false;
    }
    applyPreset(index);
    return true;
}

QString PdfSaveDialog::presetLabel() const
{
    return presetCombo_->currentText();
}

void PdfSaveDialog::setFieldForTest(const QString& name, const QString& value)
{
    setField(name, value);
}

QWidget* PdfSaveDialog::fieldWidgetForTest(const QString& name) const
{
    for (const Binding& binding : bindings_) {
        if (binding.field == name) {
            return binding.widget;
        }
    }
    return nullptr;
}

QString PdfSaveDialog::summaryText() const
{
    return summary_->toPlainText();
}

bool PdfSaveDialog::compressionItemEnabled(int index) const
{
    auto* box = qobject_cast<QComboBox*>(fieldWidgetForTest(QStringLiteral("compression")));
    auto* model = box ? qobject_cast<QStandardItemModel*>(box->model()) : nullptr;
    return model && model->item(index) && model->item(index)->isEnabled();
}

QString PdfSaveDialog::encryptionLevelText() const
{
    return encryptionLevel_->text();
}

} // namespace pictura

#include "moc_pdf_save_dialog.cpp"
