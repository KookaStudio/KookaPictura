#include "export_layers_dialog.h"

#include "api/document_api.h"
#include "api/engine_bridge.h"
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

#include <memory>

namespace pictura {

namespace {

struct Target {
    QString layerPath;
    QString outputPath;
};

struct BatchState {
    QVector<Target> targets;
    int next = 0;
    bool cancelled = false;
    bool failed = false;
    QString failingLayer;
    QString failingPath;
};

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

bool exportLayers(PictureView* view, const QString& folder, const QString& prefix,
                  bool visibleOnly, const QString& format, bool includeIcc, QWidget* parent)
{
    if (!view || !view->has_document()) {
        return false;
    }
    const QFileInfo destination(folder);
    if (!destination.isDir() || !destination.isWritable()) {
        if (parent) {
            QMessageBox::warning(
                parent, QObject::tr("Export Layers to Files"),
                QObject::tr("The destination folder is not writable."));
        }
        return false;
    }
    const QString targetsText = export_layer_targets(*view, visibleOnly, prefix);
    if (targetsText.isEmpty()) {
        return false;
    }
    auto state = std::make_shared<BatchState>();
    const QString suffix = exportLayersSuffix(format);
    for (const QString& line : targetsText.split(QLatin1Char('\n'), Qt::SkipEmptyParts)) {
        const int tab = line.indexOf(QLatin1Char('\t'));
        if (tab <= 0) {
            continue;
        }
        const QString layerPath = line.left(tab);
        const QString name = line.mid(tab + 1);
        const QString output = folder + QLatin1Char('/') + name + suffix;
        if (QFileInfo(output).isDir()) {
            state->failingLayer = layerPath;
            state->failingPath = output;
            state->failed = true;
            break;
        }
        state->targets.append({layerPath, output});
    }
    if (state->failed || state->targets.isEmpty()) {
        if (parent && state->failed) {
            QMessageBox::warning(
                parent, QObject::tr("Export Layers to Files"),
                QObject::tr("Could not write layer %1 to %2.")
                    .arg(state->failingLayer, state->failingPath));
        }
        return false;
    }

    const QPointer<PictureView> viewGuard(view);
    const QPointer<QWidget> parentGuard(parent);
    const bool done = runCancellableJob(
        parent, QObject::tr("Export Layers to Files"), QObject::tr("Writing files…"),
        [state] { return true; },
        [state, viewGuard, format, includeIcc] {
            if (state->cancelled || state->failed || state->next >= state->targets.size()) {
                return false;
            }
            if (!viewGuard) {
                state->failed = true;
                state->failingLayer = state->targets[state->next].layerPath;
                state->failingPath = state->targets[state->next].outputPath;
                return false;
            }
            const Target& target = state->targets[state->next];
            if (!export_layer_file(*viewGuard.data(), target.layerPath, format, includeIcc,
                                   target.outputPath)) {
                state->failed = true;
                state->failingLayer = target.layerPath;
                state->failingPath = target.outputPath;
                return false;
            }
            ++state->next;
            return state->next < state->targets.size();
        },
        [state] {
            return state->targets.isEmpty()
                ? 1000
                : state->next * 1000 / int(state->targets.size());
        },
        [state] { state->cancelled = true; },
        [state, parentGuard] {
            if (!state->cancelled && !state->failed) {
                return true;
            }
            if (state->failed && parentGuard) {
                QMessageBox::warning(
                    parentGuard.data(), QObject::tr("Export Layers to Files"),
                    QObject::tr("Could not write layer %1 to %2.")
                        .arg(state->failingLayer, state->failingPath));
            }
            return false;
        });
    return done;
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
