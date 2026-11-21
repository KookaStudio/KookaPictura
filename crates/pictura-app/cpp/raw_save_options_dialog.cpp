#include "raw_save_options_dialog.h"

#include "busy_job.h"

#include "api/document_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

RawSaveOptionsDialog::RawSaveOptionsDialog(const QString& defaultPath, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("rawSaveOptionsDialog"));
    setWindowTitle(tr("Save Options"));
    const QFileInfo info(defaultPath);

    auto* destination = new QGroupBox(tr("Destination"), this);
    auto* destinationRow = new QHBoxLayout(destination);
    folder_ = new QLineEdit(QDir::toNativeSeparators(info.absolutePath()), destination);
    folder_->setObjectName(QStringLiteral("rawSaveFolder"));
    auto* browse = new QPushButton(tr("Select Folder…"), destination);
    connect(browse, &QPushButton::clicked, this, [this] {
        const QString dir = QFileDialog::getExistingDirectory(this, tr("Select Folder"),
                                                              folder_->text());
        if (!dir.isEmpty()) {
            folder_->setText(QDir::toNativeSeparators(dir));
        }
    });
    destinationRow->addWidget(folder_, 1);
    destinationRow->addWidget(browse);

    auto* naming = new QGroupBox(tr("File Naming"), this);
    auto* namingForm = new QFormLayout(naming);
    name_ = new QLineEdit(info.fileName(), naming);
    name_->setObjectName(QStringLiteral("rawSaveName"));
    namingForm->addRow(tr("File Name:"), name_);

    auto* format = new QGroupBox(tr("Format: Digital Negative"), this);
    auto* formatForm = new QFormLayout(format);
    compatibility_ = new QComboBox(format);
    compatibility_->setObjectName(QStringLiteral("rawSaveCompatibility"));
    compatibility_->addItems({tr("Camera Raw 2.4 and later"), tr("Camera Raw 4.1 and later"),
                              tr("Camera Raw 4.6 and later"), tr("Camera Raw 5.4 and later"),
                              tr("Camera Raw 6.6 and later")});
    compatibility_->setCurrentIndex(4);
    formatForm->addRow(tr("Compatibility:"), compatibility_);
    preview_ = new QComboBox(format);
    preview_->setObjectName(QStringLiteral("rawSavePreview"));
    preview_->addItems({tr("None"), tr("Medium Size"), tr("Full Size")});
    preview_->setCurrentIndex(1);
    formatForm->addRow(tr("JPEG Preview:"), preview_);
    embedOriginal_ = new QCheckBox(tr("Embed Original Raw File"), format);
    embedOriginal_->setObjectName(QStringLiteral("rawSaveEmbedOriginal"));
    formatForm->addRow(embedOriginal_);
    linear_ = new QCheckBox(tr("Convert to Linear Image"), format);
    linear_->setObjectName(QStringLiteral("rawSaveLinear"));
    formatForm->addRow(linear_);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Cancel, this);
    auto* save = buttons->addButton(tr("Save"), QDialogButtonBox::AcceptRole);
    save->setObjectName(QStringLiteral("rawSaveButton"));
    save->setDefault(true);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(name_, &QLineEdit::textChanged, this,
            [save](const QString& text) { save->setEnabled(!text.trimmed().isEmpty()); });

    auto* layout = new QVBoxLayout(this);
    layout->addWidget(destination);
    layout->addWidget(naming);
    layout->addWidget(format);
    layout->addWidget(buttons);
}

QString RawSaveOptionsDialog::targetPath() const
{
    QString name = name_->text().trimmed();
    if (name.isEmpty()) {
        return {};
    }
    if (!name.endsWith(QStringLiteral(".dng"), Qt::CaseInsensitive)) {
        name += QStringLiteral(".dng");
    }
    return QDir(QDir::fromNativeSeparators(folder_->text().trimmed())).filePath(name);
}

int RawSaveOptionsDialog::compatibility() const { return compatibility_->currentIndex(); }
int RawSaveOptionsDialog::preview() const { return preview_->currentIndex(); }
bool RawSaveOptionsDialog::embedOriginal() const { return embedOriginal_->isChecked(); }
bool RawSaveOptionsDialog::linear() const { return linear_->isChecked(); }

QString runRawSaveImage(QWidget* parent, PictureView* view, const QString& settings,
                        const std::function<bool(QDialog&)>& runDialog)
{
    const QString defaultPath = raw_dng_default_path(*view);
    if (defaultPath.isEmpty()) {
        return QObject::tr("The raw file is no longer loaded.");
    }
    RawSaveOptionsDialog options(defaultPath, parent);
    if (!runDialog(options)) {
        return {};
    }
    const QString target = options.targetPath();
    if (target.isEmpty()) {
        return {};
    }
    const QString name = QFileInfo(target).fileName();
    if (QFileInfo::exists(target)) {
        QMessageBox replace(QMessageBox::Question, QObject::tr("Save Image"),
                            QObject::tr("%1 already exists. Do you want to replace it?").arg(name),
                            QMessageBox::Yes | QMessageBox::No, parent);
        replace.setObjectName(QStringLiteral("rawSaveReplace"));
        replace.setDefaultButton(QMessageBox::No);
        runDialog(replace);
        if (replace.clickedButton() != replace.button(QMessageBox::Yes)) {
            return {};
        }
    }
    if (!raw_save_dng_start(*view, target, settings, options.compatibility(), options.preview(),
                            options.embedOriginal(), options.linear())) {
        return QObject::tr("%1 was not saved: a save is already running.").arg(name);
    }
    runCancellableJob(
        parent, QObject::tr("Save Image"), QObject::tr("Saving %1…").arg(name),
        [] { return true; }, [view] { return raw_save_dng_running(*view); },
        [view] { return raw_save_dng_progress(*view); },
        [view] { raw_save_dng_cancel(*view); }, [] { return true; });
    const QString status = raw_save_dng_finish(*view);
    if (status == QStringLiteral("ok")) {
        return QObject::tr("Saved %1.").arg(name);
    }
    if (status == QStringLiteral("cancelled")) {
        return QObject::tr("Save Image was cancelled; %1 was not written.").arg(name);
    }
    return QObject::tr("%1 was not saved: %2").arg(name, status.mid(QStringLiteral("error:").size()));
}

} // namespace pictura

#include "moc_raw_save_options_dialog.cpp"
