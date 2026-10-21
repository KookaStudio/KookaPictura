#include "pdf_presets_dialog.h"

#include "dialogs.h"
#include "pdf_save_dialog.h"

#include "pictura_app/src/cxxqt_object/pdf_export.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

bool writeText(const QString& path, const QString& text)
{
    QFile file(path);
    return file.open(QIODevice::WriteOnly | QIODevice::Truncate)
           && file.write(text.toUtf8()) >= 0;
}

bool isBuiltin(const QString& name)
{
    return name.startsWith(QLatin1Char('[')) && name.endsWith(QLatin1Char(']'));
}

} // namespace

PdfPresetsDialog::PdfPresetsDialog(const QString& presetsDir,
                                   std::function<bool(QDialog&)> runEditor, QWidget* parent)
    : QDialog(parent)
    , presetsDir_(presetsDir)
    , runEditor_(std::move(runEditor))
{
    setObjectName(QStringLiteral("pdfPresetsDialog"));
    setWindowTitle(tr("Adobe PDF Presets"));
    auto* layout = new QHBoxLayout(this);
    auto* left = new QVBoxLayout();
    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("pdfPresetList"));
    left->addWidget(list_, 1);
    description_ = new QLabel(this);
    description_->setObjectName(QStringLiteral("pdfPresetDescription"));
    description_->setWordWrap(true);
    left->addWidget(new QLabel(tr("Preset Description:"), this));
    left->addWidget(description_);
    layout->addLayout(left, 1);

    auto* buttons = new QVBoxLayout();
    auto* done = new QPushButton(tr("Done"), this);
    auto* newButton = new QPushButton(tr("New…"), this);
    editButton_ = new QPushButton(tr("Edit…"), this);
    deleteButton_ = new QPushButton(tr("Delete"), this);
    auto* loadButton = new QPushButton(tr("Load…"), this);
    saveAsButton_ = new QPushButton(tr("Save As…"), this);
    for (QPushButton* button :
         {done, newButton, editButton_, deleteButton_, loadButton, saveAsButton_}) {
        buttons->addWidget(button);
    }
    buttons->addStretch(1);
    layout->addLayout(buttons);

    connect(done, &QPushButton::clicked, this, &QDialog::accept);
    connect(list_, &QListWidget::currentRowChanged, this, [this]() { updateButtons(); });
    connect(newButton, &QPushButton::clicked, this, [this]() {
        bool ok = false;
        const QString name = QInputDialog::getText(this, tr("New PDF Preset"), tr("Name:"),
                                                   QLineEdit::Normal, QString(), &ok);
        if (ok) {
            newPreset(name);
        }
    });
    connect(editButton_, &QPushButton::clicked, this, [this]() { editSelected(); });
    connect(deleteButton_, &QPushButton::clicked, this, [this]() { deleteSelected(); });
    connect(loadButton, &QPushButton::clicked, this, [this]() {
        const QString file = getOpenFileName(this, tr("Load Adobe PDF Preset"), QString(),
                                             {tr("Adobe PDF Preset (*.joboptions)")});
        if (!file.isEmpty()) {
            load(file);
        }
    });
    connect(saveAsButton_, &QPushButton::clicked, this, [this]() {
        QString selected;
        const QString file =
            getSaveFileName(this, tr("Save Adobe PDF Preset As"), selectedName() + ".joboptions",
                            {tr("Adobe PDF Preset (*.joboptions)")}, &selected);
        if (!file.isEmpty()) {
            saveSelectedAs(file);
        }
    });
    reload(QString());
}

void PdfPresetsDialog::reload(const QString& select)
{
    list_->clear();
    list_->addItems(pdf_builtin_presets().split(QLatin1Char('\n'), Qt::SkipEmptyParts));
    list_->addItems(PdfSaveDialog::userPresetNames(presetsDir_));
    const auto found = list_->findItems(select, Qt::MatchExactly);
    list_->setCurrentRow(found.isEmpty() ? 0 : list_->row(found.first()));
    updateButtons();
}

void PdfPresetsDialog::updateButtons()
{
    const bool user = !selectedName().isEmpty() && !isBuiltin(selectedName());
    editButton_->setEnabled(user);
    deleteButton_->setEnabled(user);
    saveAsButton_->setEnabled(!selectedName().isEmpty());
    description_->setText(description());
}

QString PdfPresetsDialog::selectedName() const
{
    return list_->currentItem() ? list_->currentItem()->text() : QString();
}

QString PdfPresetsDialog::filePath(const QString& name) const
{
    return presetsDir_ + QLatin1Char('/') + name + QStringLiteral(".joboptions");
}

QStringList PdfPresetsDialog::presetNames() const
{
    QStringList names;
    for (int i = 0; i < list_->count(); ++i) {
        names << list_->item(i)->text();
    }
    return names;
}

bool PdfPresetsDialog::select(const QString& name)
{
    const auto found = list_->findItems(name, Qt::MatchExactly);
    if (found.isEmpty()) {
        return false;
    }
    list_->setCurrentItem(found.first());
    return true;
}

bool PdfPresetsDialog::canEdit() const
{
    return editButton_->isEnabled();
}

QString PdfPresetsDialog::description() const
{
    const QString options = PdfSaveDialog::presetOptions(selectedName(), presetsDir_);
    return options.isEmpty() ? QString() : pdf_option(options, QStringLiteral("description"));
}

// Run the editor over `options`; on OK write them as preset `name`.
bool PdfPresetsDialog::edit(const QString& name, const QString& options)
{
    PdfSaveDialog editor(options, presetsDir_, PdfSaveDialog::Mode::PresetEditor, this);
    if (!runEditor_(editor)) {
        return false;
    }
    if (!QDir().mkpath(presetsDir_)
        || !writeText(filePath(name), pdf_preset_text(editor.options()))) {
        return false;
    }
    reload(name);
    return true;
}

bool PdfPresetsDialog::newPreset(const QString& name)
{
    const QString trimmed = name.trimmed();
    if (trimmed.isEmpty() || isBuiltin(trimmed) || trimmed.contains(QLatin1Char('/'))
        || presetNames().contains(trimmed)) {
        return false;
    }
    const QString base = PdfSaveDialog::presetOptions(selectedName(), presetsDir_);
    return edit(trimmed, base.isEmpty() ? pdf_default_options() : base);
}

bool PdfPresetsDialog::editSelected()
{
    const QString name = selectedName();
    return canEdit() && edit(name, PdfSaveDialog::presetOptions(name, presetsDir_));
}

bool PdfPresetsDialog::deleteSelected()
{
    const QString name = selectedName();
    if (!canEdit() || !QFile::remove(filePath(name))) {
        return false;
    }
    reload(QString());
    return true;
}

bool PdfPresetsDialog::load(const QString& file)
{
    QFile source(file);
    if (!source.open(QIODevice::ReadOnly)) {
        return false;
    }
    const QString options = pdf_parse_joboptions(QString::fromUtf8(source.readAll()));
    const QString name = QFileInfo(file).completeBaseName();
    if (options.isEmpty() || isBuiltin(name) || !QDir().mkpath(presetsDir_)
        || !writeText(filePath(name), pdf_preset_text(options))) {
        return false;
    }
    reload(name);
    return true;
}

bool PdfPresetsDialog::saveSelectedAs(const QString& file)
{
    const QString options = PdfSaveDialog::presetOptions(selectedName(), presetsDir_);
    return !options.isEmpty() && writeText(file, pdf_preset_text(options));
}

} // namespace pictura

#include "moc_pdf_presets_dialog.cpp"
