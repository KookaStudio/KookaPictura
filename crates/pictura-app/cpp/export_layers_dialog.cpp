#include "export_layers_dialog.h"

#include "api/document_api.h"
#include "api/engine_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QString defaultFolder(PictureView* view)
{
    const QString path = view ? document_api::filePath(*view) : QString();
    return path.isEmpty() ? QDir::homePath() : QFileInfo(path).absolutePath();
}

QString defaultPrefix(PictureView* view)
{
    const QString path = view ? document_api::filePath(*view) : QString();
    const QString stem = QFileInfo(path).completeBaseName();
    return stem.isEmpty() ? QStringLiteral("Untitled") : stem;
}

} // namespace

QString exportLayersSuffix(const QString& format)
{
    if (format == QLatin1String("JPEG")) {
        return QStringLiteral(".jpg");
    }
    if (format == QLatin1String("TIFF")) {
        return QStringLiteral(".tif");
    }
    if (format == QLatin1String("BMP")) {
        return QStringLiteral(".bmp");
    }
    if (format == QLatin1String("PDF")) {
        return QStringLiteral(".pdf");
    }
    return QStringLiteral(".psd");
}

ExportLayersDialog::ExportLayersDialog(const QString& folder, const QString& prefix,
                                       QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Export Layers to Files"));
    auto* layout = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    auto* folderRow = new QHBoxLayout;
    folder_ = new QLineEdit(folder, this);
    folder_->setObjectName(QStringLiteral("layersDestination"));
    auto* browseButton = new QPushButton(tr("Browse…"), this);
    folderRow->addWidget(folder_, 1);
    folderRow->addWidget(browseButton);
    form->addRow(tr("Destination:"), folderRow);

    prefix_ = new QLineEdit(prefix, this);
    prefix_->setObjectName(QStringLiteral("layersPrefix"));
    form->addRow(tr("File Name Prefix:"), prefix_);

    visible_ = new QCheckBox(tr("Visible Layers Only"), this);
    visible_->setObjectName(QStringLiteral("layersVisible"));
    form->addRow(QString(), visible_);

    format_ = new QComboBox(this);
    format_->setObjectName(QStringLiteral("layersFormat"));
    for (const char* name : {"PSD", "JPEG", "TIFF", "BMP", "PDF"}) {
        format_->addItem(QString::fromLatin1(name), QString::fromLatin1(name));
    }
    form->addRow(tr("File Type:"), format_);

    icc_ = new QCheckBox(tr("Include ICC Profile"), this);
    icc_->setObjectName(QStringLiteral("layersIcc"));
    form->addRow(QString(), icc_);

    layout->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Cancel | QDialogButtonBox::Ok, this);
    buttons->button(QDialogButtonBox::Ok)->setText(tr("Export"));
    layout->addWidget(buttons);

    connect(browseButton, &QPushButton::clicked, this, &ExportLayersDialog::browse);
    connect(format_, &QComboBox::currentIndexChanged, this, &ExportLayersDialog::refreshIcc);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    refreshIcc();
}

void ExportLayersDialog::browse()
{
    const QString chosen = QFileDialog::getExistingDirectory(
        this, tr("Choose Export Folder"), folder_->text());
    if (!chosen.isEmpty()) {
        folder_->setText(chosen);
    }
}

void ExportLayersDialog::refreshIcc()
{
    icc_->setEnabled(format() != QLatin1String("BMP"));
}

QString ExportLayersDialog::destination() const
{
    return folder_->text();
}

QString ExportLayersDialog::prefix() const
{
    return prefix_->text();
}

bool ExportLayersDialog::visibleOnly() const
{
    return visible_->isChecked();
}

QString ExportLayersDialog::format() const
{
    return format_->currentData().toString();
}

bool ExportLayersDialog::includeIcc() const
{
    return icc_->isChecked();
}

ExportBatchSpec layersExportBatchSpec(PictureView& view, QWidget* parent,
                                      const QString& folder, const QString& prefix,
                                      bool visibleOnly, const QString& format, bool includeIcc)
{
    ExportBatchSpec spec;
    spec.parent = parent;
    spec.view = &view;
    spec.title = QObject::tr("Export Layers to Files");
    spec.destination = folder;
    spec.suffix = exportLayersSuffix(format);
    spec.targetsText = export_layer_targets(view, visibleOnly, prefix);
    spec.destinationFailureText = QObject::tr("The destination folder is not writable.");
    spec.write = [format, includeIcc](PictureView& targetView, const ExportTarget& target) {
        return export_layer_file(targetView, target.key, format, includeIcc, target.path);
    };
    spec.failureText = [](const ExportTarget& target) {
        return QObject::tr("Could not write layer %1 to %2.").arg(target.key, target.path);
    };
    return spec;
}

bool exportLayers(PictureView* view, const QString& folder, const QString& prefix,
                  bool visibleOnly, const QString& format, bool includeIcc, QWidget* parent)
{
    if (!view || !view->has_document()) {
        return false;
    }
    return exportBatch(layersExportBatchSpec(*view, parent, folder, prefix, visibleOnly, format,
                                             includeIcc));
}

bool exportLayersFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return false;
    }
    ExportLayersDialog dialog(defaultFolder(view), defaultPrefix(view), parent);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    return exportLayers(view, dialog.destination(), dialog.prefix(), dialog.visibleOnly(),
                        dialog.format(), dialog.includeIcc(), parent);
}

} // namespace pictura

#include "moc_export_layers_dialog.cpp"
