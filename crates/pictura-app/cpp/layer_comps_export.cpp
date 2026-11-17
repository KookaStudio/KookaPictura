#include "layer_comps_export.h"

#include "api/document_api.h"
#include "api/layer_comps_api.h"
#include "api/layers_bridge.h"
#include "busy_job.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QPointer>
#include <QtCore/QStringList>
#include <QtCore/QVector>
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

#include <memory>

namespace pictura {

namespace {
QString tr(const char* text)
{
    return QObject::tr(text);
}

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
        setWindowTitle(tr("Layer Comps to Files"));
        auto* layout = new QVBoxLayout(this);
        auto* form = new QFormLayout;
        auto* row = new QHBoxLayout;
        folder_ = new QLineEdit(folder, this);
        folder_->setObjectName(QStringLiteral("compsDestination"));
        auto* browse = new QPushButton(tr("Browse…"), this);
        row->addWidget(folder_, 1);
        row->addWidget(browse);
        form->addRow(tr("Destination:"), row);
        prefix_ = new QLineEdit(prefix, this);
        prefix_->setObjectName(QStringLiteral("compsPrefix"));
        form->addRow(tr("File Name Prefix:"), prefix_);
        format_ = new QComboBox(this);
        format_->setObjectName(QStringLiteral("compsFormat"));
        for (const char* name : {"PSD", "JPEG", "TIFF", "BMP", "PDF"}) {
            format_->addItem(QString::fromLatin1(name), QString::fromLatin1(name));
        }
        form->addRow(tr("File Type:"), format_);
        icc_ = new QCheckBox(tr("Include ICC Profile"), this);
        icc_->setObjectName(QStringLiteral("compsIcc"));
        form->addRow(QString(), icc_);
        layout->addLayout(form);
        auto* buttons = new QDialogButtonBox(QDialogButtonBox::Cancel | QDialogButtonBox::Ok, this);
        buttons->button(QDialogButtonBox::Ok)->setText(tr("Export"));
        layout->addWidget(buttons);
        connect(browse, &QPushButton::clicked, this, [this] {
            const QString chosen = QFileDialog::getExistingDirectory(
                this, tr("Choose Export Folder"), folder_->text());
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

namespace {

struct Target {
    int id = -1;
    QString path;
};

struct BatchState {
    QVector<Target> targets;
    int next = 0;
    bool cancelled = false;
    bool failed = false;
    QString failing;
};

} // namespace

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

bool exportLayerComps(PictureView* view, const QString& folder, const QString& prefix,
                      const QString& format, bool includeIcc, QWidget* parent)
{
    if (!view || !view->has_document()) {
        return false;
    }
    const QFileInfo destination(folder);
    if (!destination.isDir() || !destination.isWritable()) {
        if (parent) {
            QMessageBox::warning(parent, tr("Layer Comps to Files"),
                                 tr("The destination folder is not writable."));
        }
        return false;
    }
    const QString targetsText = layer_comps_api::exportTargets(*view, prefix);
    if (targetsText.isEmpty()) {
        if (parent) {
            QMessageBox::information(parent, tr("Layer Comps to Files"),
                                     tr("This document has no layer comps."));
        }
        return false;
    }
    const QString suffix = layerCompsSuffix(format);
    auto state = std::make_shared<BatchState>();
    for (const QString& line : targetsText.split(QLatin1Char('\n'), Qt::SkipEmptyParts)) {
        const int tab = line.indexOf(QLatin1Char('\t'));
        if (tab <= 0) {
            continue;
        }
        const int id = line.left(tab).toInt();
        const QString output = folder + QLatin1Char('/') + line.mid(tab + 1) + suffix;
        if (QFileInfo(output).isDir()) {
            state->failed = true;
            state->failing = output;
            break;
        }
        state->targets.append({id, output});
    }
    if (state->failed || state->targets.isEmpty()) {
        if (parent && state->failed) {
            QMessageBox::warning(parent, tr("Layer Comps to Files"),
                                 tr("Could not write %1.").arg(state->failing));
        }
        return false;
    }

    const QPointer<PictureView> viewGuard(view);
    const QPointer<QWidget> parentGuard(parent);
    return runCancellableJob(
        parent, tr("Layer Comps to Files"), tr("Writing files…"), [state] { return true; },
        [state, viewGuard, format, includeIcc] {
            if (state->cancelled || state->failed || state->next >= state->targets.size()) {
                return false;
            }
            if (!viewGuard) {
                state->failed = true;
                return false;
            }
            const Target& target = state->targets[state->next];
            if (!layer_comps_api::exportFile(*viewGuard.data(), target.id, format, includeIcc,
                                             target.path)) {
                state->failed = true;
                state->failing = target.path;
                return false;
            }
            ++state->next;
            return state->next < state->targets.size();
        },
        [state] {
            return state->targets.isEmpty() ? 1000
                                            : state->next * 1000 / int(state->targets.size());
        },
        [state] { state->cancelled = true; },
        [state, parentGuard] {
            if (!state->cancelled && !state->failed) {
                return true;
            }
            if (state->failed && parentGuard) {
                QMessageBox::warning(parentGuard.data(), tr("Layer Comps to Files"),
                                     tr("Could not write %1.").arg(state->failing));
            }
            return false;
        });
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
            QMessageBox::information(parent, tr("Layer Comps to PDF"),
                                     tr("This document has no layer comps."));
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
        parent, tr("Layer Comps to PDF"), defaultFolder(view), tr("PDF (*.pdf)"));
    return !path.isEmpty() && exportLayerCompsPdf(view, path, parent);
}

} // namespace pictura

#include "layer_comps_export.moc"
