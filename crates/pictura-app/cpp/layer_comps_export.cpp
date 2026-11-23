#include "layer_comps_export.h"
#include <QtCore/QCoreApplication>

#include "api/document_api.h"
#include "api/layer_comps_api.h"
#include "api/layers_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QDialog>

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

class CompsDialog : public QDialog {
    Q_OBJECT

public:
    CompsDialog(const QString& folder, const QString& prefix, QWidget* parent)
        : QDialog(parent)
    {
        setWindowTitle(QCoreApplication::translate("PicturaUi", "Layer Comps to Files"));
        auto* layout = new QVBoxLayout(this);
        auto* form = new QFormLayout;
        auto* row = new QHBoxLayout;
        folder_ = new QLineEdit(folder, this);
        folder_->setObjectName(QStringLiteral("compsDestination"));
        auto* browse = new QPushButton(QCoreApplication::translate("PicturaUi", "Browse…"), this);
        row->addWidget(folder_, 1);
        row->addWidget(browse);
        form->addRow(QCoreApplication::translate("PicturaUi", "Destination:"), row);
        prefix_ = new QLineEdit(prefix, this);
        prefix_->setObjectName(QStringLiteral("compsPrefix"));
        form->addRow(QCoreApplication::translate("PicturaUi", "File Name Prefix:"), prefix_);
        format_ = new QComboBox(this);
        format_->setObjectName(QStringLiteral("compsFormat"));
        for (const char* name : {"PSD", "JPEG", "TIFF", "BMP", "PDF"}) {
            format_->addItem(QString::fromLatin1(name), QString::fromLatin1(name));
        }
        form->addRow(QCoreApplication::translate("PicturaUi", "File Type:"), format_);
        icc_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Include ICC Profile"), this);
        icc_->setObjectName(QStringLiteral("compsIcc"));
        form->addRow(QString(), icc_);
        layout->addLayout(form);
        auto* buttons = new QDialogButtonBox(QDialogButtonBox::Cancel | QDialogButtonBox::Ok, this);
        buttons->button(QDialogButtonBox::Ok)->setText(QCoreApplication::translate("PicturaUi", "Export"));
        layout->addWidget(buttons);
        connect(browse, &QPushButton::clicked, this, [this] {
            const QString chosen = QFileDialog::getExistingDirectory(
                this, QCoreApplication::translate("PicturaUi", "Choose Export Folder"), folder_->text());
            if (!chosen.isEmpty()) {
                folder_->setText(chosen);
            }
        });
        connect(format_, &QComboBox::currentIndexChanged, this, [this] {
            icc_->setEnabled(format_->currentData().toString() != QLatin1String("BMP"));
        });
        connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        icc_->setEnabled(true);
    }

    QString destination() const { return folder_->text(); }
    QString prefix() const { return prefix_->text(); }
    QString format() const { return format_->currentData().toString(); }
    bool includeIcc() const { return icc_->isChecked(); }

private:
    QLineEdit* folder_ = nullptr;
    QLineEdit* prefix_ = nullptr;
    QComboBox* format_ = nullptr;
    QCheckBox* icc_ = nullptr;
};

QString layerCompsSuffix(const QString& format)
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

ExportBatchSpec layerCompsExportBatchSpec(PictureView& view, QWidget* parent,
                                          const QString& folder, const QString& prefix,
                                          const QString& format, bool includeIcc)
{
    ExportBatchSpec spec;
    spec.parent = parent;
    spec.view = &view;
    spec.title = QCoreApplication::translate("PicturaUi", "Layer Comps to Files");
    spec.destination = folder;
    spec.suffix = layerCompsSuffix(format);
    spec.targetsText = layer_comps_api::exportTargets(view, prefix);
    spec.destinationFailureText =
        QCoreApplication::translate("PicturaUi", "The destination folder is not writable.");
    spec.emptyInfoText = QCoreApplication::translate("PicturaUi", "This document has no layer comps.");
    spec.write = [format, includeIcc](PictureView& targetView, const ExportTarget& target) {
        return layer_comps_api::exportFile(targetView, target.key.toInt(), format, includeIcc,
                                           target.path);
    };
    spec.failureText = [](const ExportTarget& target) {
        return QCoreApplication::translate("PicturaUi", "Could not write %1.")
            .arg(target.path);
    };
    return spec;
}

bool exportLayerComps(PictureView* view, const QString& folder, const QString& prefix,
                      const QString& format, bool includeIcc, QWidget* parent)
{
    if (!view || !view->has_document()) {
        return false;
    }
    return exportBatch(
        layerCompsExportBatchSpec(*view, parent, folder, prefix, format, includeIcc));
}

bool exportLayerCompsFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return false;
    }
    CompsDialog dialog(defaultFolder(view), defaultPrefix(view), parent);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    return exportLayerComps(view, dialog.destination(), dialog.prefix(), dialog.format(),
                            dialog.includeIcc(), parent);
}

bool exportLayerCompsPdf(PictureView* view, const QString& path, QWidget* parent)
{
    if (!view || !view->has_document()) {
        return false;
    }
    if (path.isEmpty()) {
        return false;
    }
    if (layer_comps_api::count(*view) == 0) {
        if (parent) {
            QMessageBox::information(parent, QCoreApplication::translate("PicturaUi", "Layer Comps to PDF"),
                                     QCoreApplication::translate("PicturaUi", "This document has no layer comps."));
        }
        return false;
    }
    return layer_comps_api::exportPdf(*view, path);
}

bool exportLayerCompsPdfFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return false;
    }
    const QString path = QFileDialog::getSaveFileName(
        parent, QCoreApplication::translate("PicturaUi", "Layer Comps to PDF"), defaultFolder(view), QCoreApplication::translate("PicturaUi", "PDF (*.pdf)"));
    return !path.isEmpty() && exportLayerCompsPdf(view, path, parent);
}

} // namespace pictura

#include "layer_comps_export.moc"
