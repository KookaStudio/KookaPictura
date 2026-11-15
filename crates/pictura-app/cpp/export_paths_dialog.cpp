#include "export_paths_dialog.h"

#include "api/document_api.h"
#include "api/engine_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QSaveFile>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include <cstdint>

namespace pictura {

namespace {

QString withAiSuffix(const QString& path)
{
    if (path.endsWith(QStringLiteral(".ai"), Qt::CaseInsensitive)) {
        return path;
    }
    const QFileInfo info(path);
    return info.path() + QLatin1Char('/') + info.completeBaseName() + QStringLiteral(".ai");
}

} // namespace

ExportPathsDialog::ExportPathsDialog(const QString& defaultPath, QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Paths to Illustrator"));
    auto* layout = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    auto* pathRow = new QHBoxLayout;
    path_ = new QLineEdit(defaultPath, this);
    path_->setObjectName(QStringLiteral("pathsDestination"));
    auto* browseButton = new QPushButton(tr("Browse…"), this);
    pathRow->addWidget(path_, 1);
    pathRow->addWidget(browseButton);
    form->addRow(tr("Destination:"), pathRow);

    format_ = new QComboBox(this);
    format_->setObjectName(QStringLiteral("pathsFormat"));
    format_->addItem(tr("Illustrator 8 (EPS-based)"), QStringLiteral("EPS"));
    format_->addItem(tr("Illustrator (PDF-based)"), QStringLiteral("PDF"));
    form->addRow(tr("Format:"), format_);

    layout->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Cancel | QDialogButtonBox::Ok, this);
    buttons->button(QDialogButtonBox::Ok)->setText(tr("Export"));
    layout->addWidget(buttons);

    connect(browseButton, &QPushButton::clicked, this, &ExportPathsDialog::browse);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
}

void ExportPathsDialog::browse()
{
    const QString chosen = QFileDialog::getSaveFileName(
        this, tr("Export Paths to Illustrator"), path_->text(),
        tr("Illustrator (*.ai)"));
    if (!chosen.isEmpty()) {
        path_->setText(chosen);
    }
}

QString ExportPathsDialog::destination() const
{
    return path_->text();
}

QString ExportPathsDialog::format() const
{
    return format_->currentData().toString();
}

bool exportPaths(PictureView* view, const QString& path, const QString& format)
{
    if (!view || !view->has_document() || !illustrator_has_paths(*view)) {
        return false;
    }
    const ::rust::Vec<std::uint8_t> bytes = format == QLatin1String("PDF")
        ? illustrator_paths_pdf(*view)
        : illustrator_paths_eps(*view);
    if (bytes.size() == 0) {
        return false;
    }
    QSaveFile file(withAiSuffix(path));
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    if (file.write(reinterpret_cast<const char*>(bytes.data()), qint64(bytes.size()))
        != qint64(bytes.size())) {
        file.cancelWriting();
        return false;
    }
    return file.commit();
}

bool exportPathsFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return false;
    }
    QString defaultPath = document_api::filePath(*view);
    if (defaultPath.isEmpty()) {
        defaultPath = QDir::homePath() + QStringLiteral("/paths.ai");
    } else {
        defaultPath = QFileInfo(defaultPath).absolutePath() + QStringLiteral("/")
            + QFileInfo(defaultPath).completeBaseName() + QStringLiteral(".ai");
    }
    ExportPathsDialog dialog(defaultPath, parent);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    if (!illustrator_has_paths(*view)) {
        if (parent) {
            QMessageBox::information(parent, QObject::tr("Paths to Illustrator"),
                                     QObject::tr("No paths to export."));
        }
        return false;
    }
    return exportPaths(view, dialog.destination(), dialog.format());
}

} // namespace pictura

#include "moc_export_paths_dialog.cpp"
