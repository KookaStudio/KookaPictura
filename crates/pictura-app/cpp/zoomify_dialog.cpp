#include "zoomify_dialog.h"

#include "api/document_api.h"
#include "api/engine_bridge.h"
#include "dialogs.h"
#include "busy_job.h"

#include <QtCore/QByteArray>
#include <QtCore/QDir>
#include <QtCore/QElapsedTimer>
#include <QtCore/QFileInfo>
#include <QtCore/QPointer>
#include "protected_output.h"
#include <QtCore/QVector>
#include <QtCore/QThread>
#include <QtGui/QImage>
#include <QtGui/QImageWriter>
#include <QtGui/QPainter>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <cstdint>
#include <memory>

namespace pictura {

namespace {

int gTestDelayMs = 0;

QString defaultFolder(PictureView* view)
{
    const QString path = view ? document_api::filePath(*view) : QString();
    if (!path.isEmpty()) {
        return QFileInfo(path).absolutePath();
    }
    return QDir::homePath();
}

struct TileJob {
    int tier = 0;
    int x = 0;
    int y = 0;
    int left = 0;
    int top = 0;
    int width = 0;
    int height = 0;
    int ordinal = 0;
};

struct TierInfo {
    int level = 0;
    int width = 0;
    int height = 0;
};

struct JobState {
    QVector<TileJob> tiles;
    QVector<TierInfo> tiers;
    QImage base;
    QImage tier;
    int tierIndex = -1;
    int next = 0;
    bool prepared = false;
    bool cancelled = false;
    bool failed = false;
    QString error;
};

QString tilePath(const QString& folder, const TileJob& job)
{
    return folder + QStringLiteral("/TileGroup")
        + QString::number(pictura::zoomify_group_for(job.ordinal))
        + QStringLiteral("/") + QString::number(job.tier)
        + QStringLiteral("-") + QString::number(job.x) + QStringLiteral("-")
        + QString::number(job.y) + QStringLiteral(".jpg");
}

bool writeText(const QString& path, const QString& text)
{
    pictura::ProtectedOutput file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    if (file.write(text.toUtf8()) < 0) {
        file.cancelWriting();
        return false;
    }
    return file.commit();
}

} // namespace

void zoomifySetTestDelay(int milliseconds)
{
    gTestDelayMs = qMax(0, milliseconds);
}

ZoomifyDialog::ZoomifyDialog(int documentWidth, int documentHeight, const QString& folder,
                             QWidget* parent)
    : QDialog(parent)
    , documentWidth_(qMax(1, documentWidth))
    , documentHeight_(qMax(1, documentHeight))
{
    setWindowTitle(tr("Zoomify"));
    auto* layout = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    auto* folderRow = new QHBoxLayout;
    folder_ = new QLineEdit(folder, this);
    folder_->setObjectName(QStringLiteral("zoomifyFolder"));
    auto* browseButton = new QPushButton(tr("Browse…"), this);
    folderRow->addWidget(folder_, 1);
    folderRow->addWidget(browseButton);
    form->addRow(tr("Output Location:"), folderRow);

    width_ = new QSpinBox(this);
    width_->setObjectName(QStringLiteral("zoomifyWidth"));
    width_->setRange(1, documentWidth_);
    width_->setValue(documentWidth_);
    height_ = new QSpinBox(this);
    height_->setObjectName(QStringLiteral("zoomifyHeight"));
    height_->setRange(1, documentHeight_);
    height_->setValue(documentHeight_);
    auto* browserRow = new QHBoxLayout;
    browserRow->addWidget(width_);
    browserRow->addWidget(new QLabel(QStringLiteral("×"), this));
    browserRow->addWidget(height_);
    form->addRow(tr("Browser Options:"), browserRow);

    quality_ = new QSpinBox(this);
    quality_->setObjectName(QStringLiteral("zoomifyQuality"));
    quality_->setRange(0, 100);
    quality_->setValue(75);
    form->addRow(tr("Image Tile Options:"), quality_);

    template_ = new QComboBox(this);
    template_->setObjectName(QStringLiteral("zoomifyTemplate"));
    template_->addItem(tr("Default"));
    form->addRow(tr("Template:"), template_);

    layout->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Cancel | QDialogButtonBox::Ok, this);
    buttons->button(QDialogButtonBox::Ok)->setText(tr("Export"));
    layout->addWidget(buttons);

    connect(browseButton, &QPushButton::clicked, this, &ZoomifyDialog::browse);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
}

void ZoomifyDialog::browse()
{
    const QString chosen = QFileDialog::getExistingDirectory(
        this, tr("Choose Zoomify Output Folder"), folder_->text());
    if (!chosen.isEmpty()) {
        folder_->setText(chosen);
    }
}

QString ZoomifyDialog::outputFolder() const
{
    return folder_->text();
}

int ZoomifyDialog::baseWidth() const
{
    return width_->value();
}

int ZoomifyDialog::baseHeight() const
{
    return height_->value();
}

int ZoomifyDialog::quality() const
{
    return quality_->value();
}

QString ZoomifyDialog::templateName() const
{
    return template_->currentText();
}

bool zoomifyExport(PictureView* view, const QString& folder, int baseWidth, int baseHeight,
                   int quality, bool* cancelled, QWidget* parent)
{
    if (!view || !view->has_document() || !pictura::compatibility_reason(*view).isEmpty()) {
        return false;
    }
    const int documentWidth = view->document_width();
    const int documentHeight = view->document_height();
    if (baseWidth <= 0 || baseHeight <= 0 || baseWidth > documentWidth
        || baseHeight > documentHeight) {
        return false;
    }
    const QFileInfo destination(folder);
    if (!destination.isDir() || !destination.isWritable()) {
        return false;
    }

    auto state = std::make_shared<JobState>();

    const ::rust::Vec<std::int32_t> flat = zoomify_plan_flat(baseWidth, baseHeight);
    if (flat.size() < 4) {
        return false;
    }
    const int tierCount = flat[2];
    int at = 4;
    int ordinal = 0;
    for (int tier = 0; tier < tierCount; ++tier) {
        if (at + 4 > int(flat.size())) {
            return false;
        }
        const int level = flat[at];
        const int tierWidth = flat[at + 1];
        const int tierHeight = flat[at + 2];
        const int tileCount = flat[at + 3];
        at += 4;
        state->tiers.append({level, tierWidth, tierHeight});
        for (int i = 0; i < tileCount; ++i) {
            if (at + 6 > int(flat.size())) {
                return false;
            }
            state->tiles.append({tier, flat[at], flat[at + 1], flat[at + 2], flat[at + 3],
                                 flat[at + 4], flat[at + 5], ordinal++});
            at += 6;
        }
    }

    const QString properties = zoomify_properties(baseWidth, baseHeight);
    const QString html = zoomify_html(baseWidth, baseHeight);
    if (properties.isEmpty() || html.isEmpty()
        || output_path_protected(folder + QStringLiteral("/ImageProperties.xml"))
        || output_path_protected(folder + QStringLiteral("/index.html"))) {
        return false;
    }
    for (const TileJob& job : state->tiles) {
        if (output_path_protected(tilePath(folder, job))) {
            return false;
        }
    }

    const QPointer<PictureView> viewGuard(view);
    return runCancellableJob(
        parent, QObject::tr("Zoomify"), QObject::tr("Writing tiles…"),
        [state] { return true; },
        [state, viewGuard, baseWidth, baseHeight, documentWidth, documentHeight, folder, quality] {
            if (state->cancelled || state->failed || state->next >= state->tiles.size()) {
                return false;
            }
            if (!state->prepared) {
                if (!viewGuard) {
                    state->failed = true;
                    return false;
                }
                QImage preview = documentPreviewImage(viewGuard.data(),
                                                      qMax(baseWidth, baseHeight));
                if (preview.isNull()) {
                    const ::rust::Vec<std::uint8_t> rgba = web_flattened(*viewGuard.data());
                    if (rgba.size()
                        != std::size_t(documentWidth) * std::size_t(documentHeight) * 4u) {
                        state->failed = true;
                        return false;
                    }
                    preview = QImage(rgba.data(), documentWidth, documentHeight,
                                     documentWidth * 4, QImage::Format_RGBA8888)
                                  .copy();
                }
                state->base = preview.scaled(baseWidth, baseHeight, Qt::IgnoreAspectRatio,
                                             Qt::SmoothTransformation)
                                  .copy();
                state->prepared = true;
                if (state->cancelled) {
                    return false;
                }
            }
            QElapsedTimer budget;
            budget.start();
            while (state->next < state->tiles.size() && budget.elapsed() < 16) {
                const TileJob& job = state->tiles[state->next];
                if (state->tierIndex != job.tier) {
                    const TierInfo& tier = state->tiers[job.tier];
                    state->tier = state->base.scaled(tier.width, tier.height,
                                                     Qt::IgnoreAspectRatio,
                                                     Qt::SmoothTransformation);
                    state->tierIndex = job.tier;
                }
                QImage tile = state->tier.copy(job.left, job.top, job.width, job.height);
                QImage flattened(tile.size(), QImage::Format_RGB32);
                flattened.fill(Qt::white);
                {
                    QPainter painter(&flattened);
                    painter.drawImage(0, 0, tile);
                }
                const QString path = tilePath(folder, job);
                const QString group = QFileInfo(path).absolutePath();
                if (!QDir().mkpath(group)) {
                    state->failed = true;
                    state->error = group;
                    return false;
                }
                pictura::ProtectedOutput file(path);
                QImageWriter writer(&file, QByteArrayLiteral("JPEG"));
                writer.setQuality(quality);
                if (!file.open(QIODevice::WriteOnly) || !writer.write(flattened) || !file.commit()) {
                    state->failed = true;
                    state->error = path;
                    return false;
                }
                ++state->next;
                if (gTestDelayMs > 0) {
                    QThread::msleep(static_cast<unsigned long>(gTestDelayMs));
                }
            }
            return state->next < state->tiles.size();
        },
        [state] {
            return state->tiles.isEmpty()
                ? 1000
                : state->next * 1000 / int(state->tiles.size());
        },
        [state] { state->cancelled = true; },
        [state, folder, properties, html, cancelled] {
            if (cancelled) {
                *cancelled = state->cancelled;
            }
            if (state->cancelled || state->failed) {
                return false;
            }
            return writeText(folder + QStringLiteral("/ImageProperties.xml"), properties)
                && writeText(folder + QStringLiteral("/index.html"), html);
        });
}

bool zoomifyExportFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document() || !pictura::compatibility_reason(*view).isEmpty()) {
        return false;
    }
    ZoomifyDialog dialog(view->document_width(), view->document_height(), defaultFolder(view),
                         parent);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    bool cancelled = false;
    const bool exported = zoomifyExport(view, dialog.outputFolder(), dialog.baseWidth(),
                                        dialog.baseHeight(), dialog.quality(), &cancelled,
                                        parent);
    if (!exported && !cancelled && parent) {
        QMessageBox::warning(parent, QObject::tr("Zoomify"),
                             QObject::tr("The Zoomify export could not be written."));
    }
    return exported;
}

} // namespace pictura

#include "moc_zoomify_dialog.cpp"
