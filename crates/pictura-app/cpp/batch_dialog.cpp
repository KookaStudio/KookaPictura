#include "batch_dialog.h"

#include "dialogs.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {
namespace {

QCheckBox* checkBox(const QString& text, const char* name, QWidget* parent)
{
    auto* box = new QCheckBox(text, parent);
    box->setObjectName(QLatin1String(name));
    return box;
}

QLabel* pathLabel(const char* name, QWidget* parent)
{
    auto* label = new QLabel(parent);
    label->setObjectName(QLatin1String(name));
    label->setTextInteractionFlags(Qt::TextSelectableByMouse);
    label->setWordWrap(true);
    return label;
}

} // namespace

FileNamingWidget::FileNamingWidget(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    example_ = new QLabel(this);
    example_->setObjectName(QStringLiteral("namingExample"));
    layout->addWidget(example_);

    auto* grid = new QGridLayout;
    QStringList choices{QString()};
    choices << batchTokenLabels();
    for (int i = 0; i < kFieldCount; ++i) {
        auto* combo = new QComboBox(this);
        combo->setObjectName(QStringLiteral("namingField%1").arg(i));
        combo->setEditable(true);
        combo->setInsertPolicy(QComboBox::NoInsert);
        combo->addItems(choices);
        connect(combo, &QComboBox::currentTextChanged, this, [this] {
            updateExample();
            emit changed();
        });
        // CS6 reads the fields left to right, then top to bottom.
        grid->addWidget(combo, i / 2, i % 2);
        fields_ << combo;
    }
    layout->addLayout(grid);

    auto* row = new QHBoxLayout;
    row->addWidget(new QLabel(tr("Starting serial#:"), this));
    serial_ = new QSpinBox(this);
    serial_->setObjectName(QStringLiteral("startingSerial"));
    serial_->setRange(1, 999999);
    connect(serial_, &QSpinBox::valueChanged, this, [this] {
        updateExample();
        emit changed();
    });
    row->addWidget(serial_);
    row->addStretch(1);
    layout->addLayout(row);

    auto* compat = new QHBoxLayout;
    compat->addWidget(new QLabel(tr("Compatibility:"), this));
    windows_ = checkBox(tr("Windows"), "compatWindows", this);
    mac_ = checkBox(tr("Mac OS"), "compatMac", this);
    unix_ = checkBox(tr("Unix"), "compatUnix", this);
    // Unix rules always apply on Linux.
    unix_->setChecked(true);
    unix_->setEnabled(false);
    for (QCheckBox* box : {windows_, mac_, unix_}) {
        compat->addWidget(box);
        connect(box, &QCheckBox::toggled, this, [this] {
            updateExample();
            emit changed();
        });
    }
    compat->addStretch(1);
    layout->addLayout(compat);

    setNaming(BatchOptions().namingFields, 1, false, false);
}

void FileNamingWidget::setNaming(const QStringList& fields, int startingSerial, bool windows,
                                 bool mac)
{
    for (int i = 0; i < kFieldCount; ++i) {
        fields_.at(i)->setCurrentText(fields.value(i));
    }
    serial_->setValue(qMax(1, startingSerial));
    windows_->setChecked(windows);
    mac_->setChecked(mac);
    updateExample();
}

QStringList FileNamingWidget::fields() const
{
    QStringList out;
    for (const QComboBox* combo : fields_) {
        out << combo->currentText();
    }
    while (!out.isEmpty() && out.last().isEmpty()) {
        out.removeLast();
    }
    return out;
}

int FileNamingWidget::startingSerial() const
{
    return serial_->value();
}

bool FileNamingWidget::windowsCompatible() const
{
    return windows_->isChecked();
}

bool FileNamingWidget::macCompatible() const
{
    return mac_->isChecked();
}

QString FileNamingWidget::exampleText() const
{
    return example_->text();
}

void FileNamingWidget::updateExample()
{
    if (!serial_ || !windows_ || !mac_) {
        return;
    }
    BatchOptions naming;
    naming.namingFields = fields();
    naming.startingSerial = startingSerial();
    naming.windowsCompatible = windowsCompatible();
    naming.macCompatible = macCompatible();
    const QString name = batchRenderName(naming.namingJson(), QStringLiteral("MyFile"),
                                         QStringLiteral("gif"), 0, QDate::currentDate());
    example_->setText(tr("Example: %1").arg(name));
}

BatchOptionsWidget::BatchOptionsWidget(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);

    auto* play = new QGroupBox(tr("Play"), this);
    auto* playForm = new QFormLayout(play);
    set_ = new QComboBox(play);
    set_->setObjectName(QStringLiteral("batchSet"));
    action_ = new QComboBox(play);
    action_->setObjectName(QStringLiteral("batchAction"));
    playForm->addRow(tr("Set:"), set_);
    playForm->addRow(tr("Action:"), action_);
    connect(set_, &QComboBox::currentIndexChanged, this, [this] { fillActions(); });
    layout->addWidget(play);

    auto* source = new QGroupBox(tr("Source"), this);
    auto* sourceLayout = new QVBoxLayout(source);
    source_ = new QComboBox(source);
    source_->setObjectName(QStringLiteral("batchSource"));
    source_->addItems({tr("Folder"), tr("Opened Files")});
    sourceChoose_ = new QPushButton(tr("Choose…"), source);
    sourceChoose_->setObjectName(QStringLiteral("batchSourceChoose"));
    sourcePath_ = pathLabel("batchSourcePath", source);
    auto* sourceRow = new QHBoxLayout;
    sourceRow->addWidget(source_);
    sourceRow->addWidget(sourceChoose_);
    sourceRow->addWidget(sourcePath_, 1);
    sourceLayout->addLayout(sourceRow);
    overrideOpen_ = checkBox(tr("Override Action “Open” Commands"),
                             "batchOverrideOpen", source);
    includeSubfolders_ = checkBox(tr("Include All Subfolders"), "batchIncludeSubfolders", source);
    suppressOpen_ = checkBox(tr("Suppress File Open Options Dialogs"), "batchSuppressOpen", source);
    suppressProfile_ = checkBox(tr("Suppress Color Profile Warnings"), "batchSuppressProfile",
                                source);
    for (QCheckBox* box : {overrideOpen_, includeSubfolders_, suppressOpen_, suppressProfile_}) {
        sourceLayout->addWidget(box);
    }
    layout->addWidget(source);

    auto* destination = new QGroupBox(tr("Destination"), this);
    auto* destinationLayout = new QVBoxLayout(destination);
    destination_ = new QComboBox(destination);
    destination_->setObjectName(QStringLiteral("batchDestination"));
    destination_->addItems({tr("None"), tr("Save and Close"), tr("Folder")});
    destinationChoose_ = new QPushButton(tr("Choose…"), destination);
    destinationChoose_->setObjectName(QStringLiteral("batchDestinationChoose"));
    destinationPath_ = pathLabel("batchDestinationPath", destination);
    auto* destinationRow = new QHBoxLayout;
    destinationRow->addWidget(destination_);
    destinationRow->addWidget(destinationChoose_);
    destinationRow->addWidget(destinationPath_, 1);
    destinationLayout->addLayout(destinationRow);
    overrideSave_ = checkBox(tr("Override Action “Save As” Commands"),
                             "batchOverrideSave", destination);
    destinationLayout->addWidget(overrideSave_);
    auto* namingGroup = new QGroupBox(tr("File Naming"), destination);
    namingGroup->setObjectName(QStringLiteral("batchNamingGroup"));
    auto* namingLayout = new QVBoxLayout(namingGroup);
    naming_ = new FileNamingWidget(namingGroup);
    naming_->setObjectName(QStringLiteral("batchNaming"));
    namingLayout->addWidget(naming_);
    destinationLayout->addWidget(namingGroup);
    layout->addWidget(destination);

    auto* errors = new QGroupBox(tr("Errors"), this);
    auto* errorsRow = new QHBoxLayout(errors);
    errors_ = new QComboBox(errors);
    errors_->setObjectName(QStringLiteral("batchErrors"));
    errors_->addItems({tr("Stop For Errors"), tr("Log Errors To File")});
    errorLogChoose_ = new QPushButton(tr("Save As…"), errors);
    errorLogChoose_->setObjectName(QStringLiteral("batchErrorLogChoose"));
    errorLogPath_ = pathLabel("batchErrorLogPath", errors);
    errorsRow->addWidget(errors_);
    errorsRow->addWidget(errorLogChoose_);
    errorsRow->addWidget(errorLogPath_, 1);
    layout->addWidget(errors);

    connect(source_, &QComboBox::currentIndexChanged, this, [this] { updateEnablement(); });
    connect(destination_, &QComboBox::currentIndexChanged, this,
            [this] { updateEnablement(); });
    connect(errors_, &QComboBox::currentIndexChanged, this, [this] { updateEnablement(); });
    connect(sourceChoose_, &QPushButton::clicked, this, [this] { chooseSourceFolder(); });
    connect(destinationChoose_, &QPushButton::clicked, this,
            [this] { chooseDestinationFolder(); });
    connect(errorLogChoose_, &QPushButton::clicked, this, [this] { chooseErrorLog(); });
    updateEnablement();
}

void BatchOptionsWidget::setLibrary(const ActionLibrary& library)
{
    library_ = library;
    const QSignalBlocker blocker(set_);
    set_->clear();
    for (const auto& entry : library_) {
        set_->addItem(entry.first);
    }
    fillActions();
}

void BatchOptionsWidget::fillActions()
{
    action_->clear();
    const int index = set_->currentIndex();
    if (index >= 0 && index < library_.size()) {
        action_->addItems(library_.at(index).second);
    }
}

void BatchOptionsWidget::setOptions(const BatchOptions& options)
{
    const int setIndex = set_->findText(options.set);
    if (setIndex >= 0) {
        set_->setCurrentIndex(setIndex);
    }
    const int actionIndex = action_->findText(options.action);
    if (actionIndex >= 0) {
        action_->setCurrentIndex(actionIndex);
    }
    source_->setCurrentIndex(options.source == BatchOptions::Source::OpenedFiles ? 1 : 0);
    sourceFolder_ = options.sourceFolder;
    sourcePath_->setText(QDir::toNativeSeparators(sourceFolder_));
    includeSubfolders_->setChecked(options.includeSubfolders);
    overrideOpen_->setChecked(options.overrideOpen);
    suppressOpen_->setChecked(options.suppressOpen);
    suppressProfile_->setChecked(options.suppressProfile);
    destination_->setCurrentIndex(static_cast<int>(options.destination));
    destinationFolder_ = options.destinationFolder;
    destinationPath_->setText(QDir::toNativeSeparators(destinationFolder_));
    overrideSave_->setChecked(options.overrideSave);
    naming_->setNaming(options.namingFields, options.startingSerial, options.windowsCompatible,
                       options.macCompatible);
    errors_->setCurrentIndex(options.errors == BatchOptions::Errors::Log ? 1 : 0);
    errorLog_ = options.errorLog;
    errorLogPath_->setText(QDir::toNativeSeparators(errorLog_));
    updateEnablement();
}

BatchOptions BatchOptionsWidget::options() const
{
    BatchOptions options;
    options.set = set_->currentText();
    options.action = action_->currentText();
    options.source = source_->currentIndex() == 1 ? BatchOptions::Source::OpenedFiles
                                                   : BatchOptions::Source::Folder;
    options.sourceFolder = sourceFolder_;
    options.includeSubfolders = includeSubfolders_->isChecked();
    options.overrideOpen = overrideOpen_->isChecked();
    options.suppressOpen = suppressOpen_->isChecked();
    options.suppressProfile = suppressProfile_->isChecked();
    options.destination = static_cast<BatchOptions::Destination>(destination_->currentIndex());
    options.destinationFolder = destinationFolder_;
    // A disabled override does not apply (destination None).
    options.overrideSave = overrideSave_->isEnabled() && overrideSave_->isChecked();
    options.namingFields = naming_->fields();
    options.startingSerial = naming_->startingSerial();
    options.windowsCompatible = naming_->windowsCompatible();
    options.macCompatible = naming_->macCompatible();
    options.errors = errors_->currentIndex() == 1 ? BatchOptions::Errors::Log
                                                  : BatchOptions::Errors::Stop;
    options.errorLog = errorLog_;
    return options;
}

void BatchOptionsWidget::updateEnablement()
{
    const bool folderSource = source_->currentIndex() == 0;
    sourceChoose_->setEnabled(folderSource);
    includeSubfolders_->setEnabled(folderSource);
    const int destination = destination_->currentIndex();
    const bool folderDestination = destination == 2;
    destinationChoose_->setEnabled(folderDestination);
    overrideSave_->setEnabled(destination != 0);
    naming_->setEnabled(folderDestination);
    errorLogChoose_->setEnabled(errors_->currentIndex() == 1);
}

void BatchOptionsWidget::chooseSourceFolder()
{
    const QString title = tr("Choose a batch folder");
    const QString chosen = folderProvider_
        ? folderProvider_(title, sourceFolder_)
        : QFileDialog::getExistingDirectory(this, title, sourceFolder_);
    if (!chosen.isEmpty()) {
        sourceFolder_ = chosen;
        sourcePath_->setText(QDir::toNativeSeparators(chosen));
    }
}

void BatchOptionsWidget::chooseDestinationFolder()
{
    const QString title = tr("Choose a destination folder");
    const QString chosen = folderProvider_
        ? folderProvider_(title, destinationFolder_)
        : QFileDialog::getExistingDirectory(this, title, destinationFolder_);
    if (!chosen.isEmpty()) {
        destinationFolder_ = chosen;
        destinationPath_->setText(QDir::toNativeSeparators(chosen));
    }
}

void BatchOptionsWidget::chooseErrorLog()
{
    const QString title = tr("Save error log as");
    const QString chosen = fileProvider_
        ? fileProvider_(title, errorLog_)
        : QFileDialog::getSaveFileName(this, title, errorLog_, tr("Text (*.txt)"));
    if (!chosen.isEmpty()) {
        errorLog_ = chosen;
        errorLogPath_->setText(QDir::toNativeSeparators(chosen));
    }
}

BatchDialog::BatchDialog(QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("batchDialog"));
    setWindowTitle(tr("Batch"));
    auto* layout = new QVBoxLayout(this);
    options_ = new BatchOptionsWidget(this);
    layout->addWidget(options_);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &BatchDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &BatchDialog::reject);
    layout->addWidget(buttons);
}

BatchOptions BatchDialog::options() const
{
    return options_->options();
}

void BatchDialog::accept()
{
    const BatchOptions chosen = options();
    const QString error = validator_ ? validator_(chosen) : QString();
    if (!error.isEmpty()) {
        if (messageHandler_) {
            messageHandler_(error);
        } else {
            QMessageBox::warning(this, windowTitle(), error);
        }
        return;
    }
    if (!chosen.overrideOpen && openProbe_ && openProbe_(chosen)) {
        const QString warning =
            tr("The action contains an Open command and Override Action “Open” "
               "Commands is off. The batch will open the action's recorded file instead of "
               "processing the selected files. Continue?");
        const bool proceed = confirmHandler_
            ? confirmHandler_(warning)
            : QMessageBox::warning(this, windowTitle(), warning,
                                   QMessageBox::Ok | QMessageBox::Cancel)
                == QMessageBox::Ok;
        if (!proceed) {
            return;
        }
    }
    QDialog::accept();
}

} // namespace pictura

#include "moc_batch_dialog.cpp"
