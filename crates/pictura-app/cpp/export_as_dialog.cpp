#include "export_as_dialog.h"

#include "api/document_api.h"
#include "dialogs.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/export.cxxqt.h"
#include "pictura_app/src/cxxqt_object/svg_export.cxxqt.h"
#include "pictura_app/src/cxxqt_object/web_export.cxxqt.h"

#include <QtCore/QBuffer>
#include <QtCore/QFileInfo>
#include <QtCore/QSaveFile>
#include <QtCore/QSet>
#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtGui/QImageWriter>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <cstring>

namespace pictura {

namespace {

constexpr int kPreviewSide = 320;
// A full-resolution encode runs on every change; above this many pixels the
// size estimate is omitted rather than stalling the dialog.
constexpr qint64 kSizeEstimateCap = 4000000;
constexpr int kSizeDebounceMs = 300;

struct FormatEntry {
    const char* label;
    const char* value;
    const char* suffix;
};

constexpr FormatEntry kFormats[] = {
    {"PNG", "PNG", "png"},
    {"JPG", "JPG", "jpg"},
    {"PNG-8", "PNG8", "png"},
    {"GIF", "GIF", "gif"},
    {"SVG", "SVG", "svg"},
    {"WebP", "WEBP", "webp"},
};

QString suffixForFormat(const QString& format)
{
    for (const FormatEntry& entry : kFormats) {
        if (format == QLatin1String(entry.value)) {
            return QString::fromLatin1(entry.suffix);
        }
    }
    return QStringLiteral("png");
}

QString filterForFormat(const QString& format)
{
    for (const FormatEntry& entry : kFormats) {
        if (format == QLatin1String(entry.value)) {
            return QStringLiteral("%1 (*.%2)")
                .arg(QString::fromLatin1(entry.label),
                     QString::fromLatin1(entry.suffix).toUpper());
        }
    }
    return QStringLiteral("PNG (*.PNG)");
}

// The format a path's suffix names, or `fallback` when it names none. A `.png`
// keeps the fallback's PNG vs PNG-8 distinction; anything else wins so a typed
// `.jpg` writes JPEG even when the combo said PNG.
ExportAsDialog::Format formatForPath(const QString& path, ExportAsDialog::Format fallback)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    if (suffix == QLatin1String("jpg") || suffix == QLatin1String("jpeg")) {
        return ExportAsDialog::Jpg;
    }
    if (suffix == QLatin1String("gif")) {
        return ExportAsDialog::Gif;
    }
    if (suffix == QLatin1String("svg")) {
        return ExportAsDialog::Svg;
    }
    if (suffix == QLatin1String("webp")) {
        return ExportAsDialog::Webp;
    }
    if (suffix == QLatin1String("png")) {
        return fallback == ExportAsDialog::Png8 ? ExportAsDialog::Png8 : ExportAsDialog::Png;
    }
    return fallback;
}

QImage rgbaOf(const QImage& image) { return image.convertToFormat(QImage::Format_RGBA8888); }

::rust::Slice<const std::uint8_t> bytesOf(const QImage& rgba)
{
    return {rgba.constBits(), std::size_t(rgba.sizeInBytes())};
}

QImage over(const QImage& image, const QColor& matte)
{
    QImage flat(image.size(), QImage::Format_RGB32);
    flat.fill(matte);
    QPainter painter(&flat);
    painter.drawImage(0, 0, image);
    return flat;
}

QString sizeText(qsizetype bytes)
{
    if (bytes < 1024) {
        return QStringLiteral("%1 bytes").arg(bytes);
    }
    if (bytes < 1024 * 1024) {
        return QStringLiteral("%1 KB").arg(double(bytes) / 1024.0, 0, 'f', 1);
    }
    return QStringLiteral("%1 MB").arg(double(bytes) / (1024.0 * 1024.0), 0, 'f', 1);
}

// A filesystem-safe name: replace separators and control characters.
QString sanitizeName(const QString& name)
{
    QString out;
    out.reserve(name.size());
    for (const QChar c : name) {
        out.append(c.isLetterOrNumber() || c == QLatin1Char('-') || c == QLatin1Char('_')
                       || c == QLatin1Char('.') || c == QLatin1Char(' ')
                   ? c
                   : QLatin1Char('_'));
    }
    const QString trimmed = out.trimmed();
    return trimmed.isEmpty() ? QStringLiteral("export") : trimmed;
}

// Reduce `rgba` with the PNG-8 / GIF settings (Selective, 256 colours, no
// dither); with `gif`, also encode it.
QImage indexedImage(const QImage& rgba, bool transparency, QByteArray* gif)
{
    WebPalette palette{};
    palette.reduction = 1; // Selective
    palette.colors = 256;
    palette.dither = 0; // No Dither
    palette.amount = 0;
    palette.transparency = transparency;
    palette.matte = 0xffffffffu;
    const ::rust::Vec<std::uint8_t> packed =
        web_quantize(bytesOf(rgba), rgba.width(), rgba.height(), palette);
    if (packed.size() < 2) {
        return QImage();
    }
    const int n = (int(packed[0]) << 8) | int(packed[1]);
    QImage indexed(rgba.size(), QImage::Format_Indexed8);
    QList<QRgb> table;
    for (int i = 0; i < n; ++i) {
        const std::uint8_t* c = packed.data() + 2 + i * 4;
        table.append(qRgba(c[0], c[1], c[2], c[3]));
    }
    indexed.setColorTable(table);
    const std::uint8_t* indices = packed.data() + 2 + n * 4;
    for (int y = 0; y < rgba.height(); ++y) {
        std::memcpy(indexed.scanLine(y), indices + std::size_t(y) * rgba.width(),
                    std::size_t(rgba.width()));
    }
    if (gif) {
        const ::rust::Slice<const std::uint8_t> view(packed.data(), packed.size());
        const ::rust::Vec<std::uint8_t> bytes =
            web_encode_gif(view, rgba.width(), rgba.height(), false);
        *gif = QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
    }
    return indexed;
}

// Keep a path whose suffix names a format `saveTo` honors; otherwise replace an
// unknown suffix (or add one) with the chosen format's suffix.
QString retargetPath(const QString& path, const QString& chosenFormat)
{
    const QString lower = QFileInfo(path).suffix().toLower();
    if (lower == QLatin1String("png") || lower == QLatin1String("jpg")
        || lower == QLatin1String("jpeg") || lower == QLatin1String("gif")
        || lower == QLatin1String("svg") || lower == QLatin1String("webp")) {
        return path;
    }
    QString base = path;
    if (!lower.isEmpty()) {
        base.chop(lower.size() + 1);
    }
    return base + QLatin1Char('.') + suffixForFormat(chosenFormat);
}

constexpr const char* kResampleOptions[] = {
    "Bicubic", "Bicubic Automatic", "Bicubic Sharper", "Bicubic Smoother",
    "Bilinear", "Nearest Neighbor", "Preserve Details",
};

} // namespace

ExportAsDialog::ExportAsDialog(const QVector<ExportEntry>& entries, QWidget* parent)
    : QDialog(parent)
    , entries_(entries)
{
    setObjectName(QStringLiteral("exportAsDialog"));
    setWindowTitle(tr("Export As"));
    buildUi();
    if (!entries_.isEmpty()) {
        list_->setCurrentRow(0);
        resetSizingFor(0);
    }
    refreshFormatControls();
    refreshPreview();
}

void ExportAsDialog::buildUi()
{
    auto* root = new QHBoxLayout(this);

    // Left: the layer list.
    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("exportAsList"));
    for (int i = 0; i < entries_.size(); ++i) {
        auto* item = new QListWidgetItem(entries_[i].name, list_);
        item->setFlags(Qt::ItemIsUserCheckable | Qt::ItemIsEnabled | Qt::ItemIsSelectable);
        item->setCheckState(i == 0 ? Qt::Checked : Qt::Unchecked);
        if (!entries_[i].image.isNull()) {
            item->setIcon(QPixmap::fromImage(entries_[i].image.scaled(
                32, 32, Qt::KeepAspectRatio, Qt::SmoothTransformation)));
        }
    }
    list_->setMinimumWidth(150);
    root->addWidget(list_);

    // Center: the preview and its zoom controls.
    auto* center = new QVBoxLayout;
    previewArea_ = new QScrollArea(this);
    previewArea_->setObjectName(QStringLiteral("exportAsPreviewArea"));
    previewArea_->setAlignment(Qt::AlignCenter);
    previewArea_->setMinimumSize(kPreviewSide, kPreviewSide);
    preview_ = new QLabel(this);
    preview_->setObjectName(QStringLiteral("exportAsPreview"));
    preview_->setAlignment(Qt::AlignCenter);
    previewArea_->setWidget(preview_);
    center->addWidget(previewArea_, 1);

    auto* zoomRow = new QHBoxLayout;
    const auto zoomButton = [this, zoomRow](const char* name, const char* text) {
        auto* button = new QPushButton(QString::fromLatin1(text), this);
        button->setObjectName(QString::fromLatin1(name));
        zoomRow->addWidget(button);
        return button;
    };
    connect(zoomButton("exportAsZoomFit", "Fit"), &QPushButton::clicked, this,
            [this] { fitZoom(); });
    connect(zoomButton("exportAsZoomActual", "1:1"), &QPushButton::clicked, this,
            [this] { applyZoom(1.0); });
    connect(zoomButton("exportAsZoomOut", "-"), &QPushButton::clicked, this,
            [this] { applyZoom(qMax(0.05, zoom_ / 1.25)); });
    connect(zoomButton("exportAsZoomIn", "+"), &QPushButton::clicked, this,
            [this] { applyZoom(qMin(8.0, zoom_ * 1.25)); });
    zoomLabel_ = new QLabel(this);
    zoomLabel_->setObjectName(QStringLiteral("exportAsZoomLabel"));
    zoomRow->addWidget(zoomLabel_);
    zoomRow->addStretch(1);
    sizeLabel_ = new QLabel(this);
    sizeLabel_->setObjectName(QStringLiteral("exportAsSizeLabel"));
    zoomRow->addWidget(sizeLabel_);
    center->addLayout(zoomRow);
    root->addLayout(center, 1);

    // Right: the settings column.
    auto* right = new QVBoxLayout;

    auto* fileBox = new QGroupBox(tr("File Settings"), this);
    auto* fileForm = new QFormLayout(fileBox);
    format_ = new QComboBox(fileBox);
    format_->setObjectName(QStringLiteral("exportAsFormat"));
    for (const FormatEntry& entry : kFormats) {
        format_->addItem(QString::fromLatin1(entry.label), QString::fromLatin1(entry.value));
    }
    fileForm->addRow(tr("Format:"), format_);
    transparency_ = new QCheckBox(tr("Transparency"), fileBox);
    transparency_->setObjectName(QStringLiteral("exportAsTransparency"));
    transparency_->setChecked(true);
    fileForm->addRow(QString(), transparency_);
    qualityLabel_ = new QLabel(tr("Quality:"), fileBox);
    quality_ = new QSpinBox(fileBox);
    quality_->setObjectName(QStringLiteral("exportAsQuality"));
    quality_->setRange(1, 100);
    quality_->setValue(90);
    fileForm->addRow(qualityLabel_, quality_);
    right->addWidget(fileBox);

    auto* sizeBox = new QGroupBox(tr("Image Size"), this);
    auto* sizeForm = new QFormLayout(sizeBox);
    width_ = new QSpinBox(sizeBox);
    width_->setObjectName(QStringLiteral("exportAsWidth"));
    height_ = new QSpinBox(sizeBox);
    height_->setObjectName(QStringLiteral("exportAsHeight"));
    for (QSpinBox* box : {width_, height_}) {
        box->setRange(1, 30000);
        box->setSuffix(QStringLiteral(" px"));
    }
    scale_ = new QSpinBox(sizeBox);
    scale_->setObjectName(QStringLiteral("exportAsScale"));
    scale_->setRange(1, 1000);
    scale_->setSuffix(QStringLiteral("%"));
    scale_->setValue(100);
    sizeForm->addRow(tr("Width:"), width_);
    sizeForm->addRow(tr("Height:"), height_);
    sizeForm->addRow(tr("Scale:"), scale_);
    constrain_ = new QCheckBox(tr("Constrain Proportions"), sizeBox);
    constrain_->setObjectName(QStringLiteral("exportAsConstrain"));
    constrain_->setChecked(true);
    sizeForm->addRow(constrain_);
    resample_ = new QComboBox(sizeBox);
    resample_->setObjectName(QStringLiteral("exportAsResample"));
    for (const char* option : kResampleOptions) {
        resample_->addItem(QString::fromLatin1(option));
    }
    sizeForm->addRow(tr("Resample:"), resample_);
    right->addWidget(sizeBox);

    auto* canvasBox = new QGroupBox(tr("Canvas Size"), this);
    auto* canvasForm = new QFormLayout(canvasBox);
    canvasWidth_ = new QSpinBox(canvasBox);
    canvasWidth_->setObjectName(QStringLiteral("exportAsCanvasWidth"));
    canvasHeight_ = new QSpinBox(canvasBox);
    canvasHeight_->setObjectName(QStringLiteral("exportAsCanvasHeight"));
    for (QSpinBox* box : {canvasWidth_, canvasHeight_}) {
        box->setRange(1, 30000);
        box->setSuffix(QStringLiteral(" px"));
    }
    canvasReset_ = new QPushButton(tr("Reset"), canvasBox);
    canvasReset_->setObjectName(QStringLiteral("exportAsCanvasReset"));
    canvasForm->addRow(tr("Width:"), canvasWidth_);
    canvasForm->addRow(tr("Height:"), canvasHeight_);
    canvasForm->addRow(QString(), canvasReset_);
    right->addWidget(canvasBox);
    right->addStretch(1);

    auto* buttons = new QDialogButtonBox(this);
    QPushButton* exportButton =
        buttons->addButton(tr("Export..."), QDialogButtonBox::AcceptRole);
    exportButton->setObjectName(QStringLiteral("exportAsExport"));
    buttons->addButton(QDialogButtonBox::Cancel);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    right->addWidget(buttons);
    root->addLayout(right);

    sizeTimer_ = new QTimer(this);
    sizeTimer_->setSingleShot(true);
    sizeTimer_->setInterval(kSizeDebounceMs);
    connect(sizeTimer_, &QTimer::timeout, this, [this] {
        const int entry = currentEntry();
        if (entry < 0) {
            return;
        }
        const QImage image = sized(entry);
        if (image.isNull()) {
            return;
        }
        const QString dimensions =
            QStringLiteral("%1 x %2").arg(image.width()).arg(image.height());
        if (qint64(image.width()) * image.height() > kSizeEstimateCap) {
            sizeLabel_->setText(dimensions);
            return;
        }
        const QByteArray data = encoded(entry);
        sizeLabel_->setText(data.isEmpty()
                                ? dimensions
                                : QStringLiteral("%1  %2").arg(sizeText(data.size()), dimensions));
    });

    // A setting change re-previews (and re-estimates); a selection change resets
    // the sizing to that entry's own size.
    const auto changed = [this] {
        refreshFormatControls();
        refreshPreview();
    };
    connect(format_, &QComboBox::currentIndexChanged, this, changed);
    connect(resample_, &QComboBox::currentIndexChanged, this, changed);
    connect(transparency_, &QCheckBox::toggled, this, changed);
    connect(quality_, &QSpinBox::valueChanged, this, changed);
    connect(width_, &QSpinBox::valueChanged, this, [this](int w) {
        const int entry = currentEntry();
        if (constrain_->isChecked() && entry >= 0) {
            const QImage& base = entries_[entry].image;
            const QSignalBlocker blockH(height_);
            const QSignalBlocker blockS(scale_);
            height_->setValue(qMax(1, qRound(double(w) * base.height() / base.width())));
            scale_->setValue(qRound(100.0 * w / base.width()));
        }
        syncCanvasToImage();
        refreshPreview();
    });
    connect(height_, &QSpinBox::valueChanged, this, [this](int h) {
        const int entry = currentEntry();
        if (constrain_->isChecked() && entry >= 0) {
            const QImage& base = entries_[entry].image;
            const QSignalBlocker blockW(width_);
            const QSignalBlocker blockS(scale_);
            width_->setValue(qMax(1, qRound(double(h) * base.width() / base.height())));
            scale_->setValue(qRound(100.0 * h / base.height()));
        }
        syncCanvasToImage();
        refreshPreview();
    });
    connect(scale_, &QSpinBox::valueChanged, this, [this](int percent) {
        const int entry = currentEntry();
        if (entry >= 0) {
            const QImage& base = entries_[entry].image;
            const QSignalBlocker blockW(width_);
            const QSignalBlocker blockH(height_);
            width_->setValue(qMax(1, qRound(base.width() * percent / 100.0)));
            height_->setValue(qMax(1, qRound(base.height() * percent / 100.0)));
        }
        syncCanvasToImage();
        refreshPreview();
    });
    connect(canvasWidth_, &QSpinBox::valueChanged, this, [this] {
        if (!updatingCanvas_) {
            canvasLinked_ = false;
        }
        refreshPreview();
    });
    connect(canvasHeight_, &QSpinBox::valueChanged, this, [this] {
        if (!updatingCanvas_) {
            canvasLinked_ = false;
        }
        refreshPreview();
    });
    connect(canvasReset_, &QPushButton::clicked, this, [this] {
        canvasLinked_ = true;
        syncCanvasToImage();
        refreshPreview();
    });
    connect(list_, &QListWidget::currentRowChanged, this, [this](int row) {
        if (row >= 0) {
            resetSizingFor(row);
        }
        refreshPreview();
    });
    connect(list_, &QListWidget::itemChanged, this, [this] { refreshPreview(); });
}

void ExportAsDialog::resetSizingFor(int entry)
{
    if (entry < 0 || entry >= entries_.size()) {
        return;
    }
    const QImage& image = entries_[entry].image;
    const QSignalBlocker blockW(width_);
    const QSignalBlocker blockH(height_);
    const QSignalBlocker blockS(scale_);
    const QSignalBlocker blockCW(canvasWidth_);
    const QSignalBlocker blockCH(canvasHeight_);
    canvasLinked_ = true;
    width_->setValue(qMax(1, image.width()));
    height_->setValue(qMax(1, image.height()));
    scale_->setValue(100);
    canvasWidth_->setValue(qMax(1, image.width()));
    canvasHeight_->setValue(qMax(1, image.height()));
}

void ExportAsDialog::syncCanvasToImage()
{
    if (!canvasLinked_) {
        return;
    }
    updatingCanvas_ = true;
    canvasWidth_->setValue(width_->value());
    canvasHeight_->setValue(height_->value());
    updatingCanvas_ = false;
}

void ExportAsDialog::refreshFormatControls()
{
    const Format format = Format(format_->currentIndex());
    // WebP export is lossless (the engine has no lossy encoder), so its quality
    // control stays off even though the format carries alpha.
    const bool lossy = format == Jpg;
    const bool alpha = format == Png || format == Png8 || format == Gif || format == Webp;
    quality_->setEnabled(lossy);
    qualityLabel_->setEnabled(lossy);
    transparency_->setEnabled(alpha);
}

int ExportAsDialog::currentEntry() const
{
    const int row = list_->currentRow();
    return row >= 0 && row < entries_.size() ? row : (entries_.isEmpty() ? -1 : 0);
}

QImage ExportAsDialog::sized(int entry) const
{
    if (entry < 0 || entry >= entries_.size()) {
        return QImage();
    }
    QImage image = entries_[entry].image;
    const int w = width_->value();
    const int h = height_->value();
    if (w != image.width() || h != image.height()) {
        image = image.scaled(w, h, Qt::IgnoreAspectRatio,
                             resample_->currentIndex() == 5 ? Qt::FastTransformation
                                                            : Qt::SmoothTransformation);
    }
    const int cw = canvasWidth_->value();
    const int ch = canvasHeight_->value();
    if (cw == image.width() && ch == image.height()) {
        return image;
    }
    QImage canvas(cw, ch, QImage::Format_RGBA8888);
    canvas.fill(Qt::transparent);
    QPainter painter(&canvas);
    painter.drawImage(0, 0, image);
    painter.end();
    return canvas;
}

QByteArray ExportAsDialog::encodedAs(int entry, Format format) const
{
    const QImage image = sized(entry);
    if (image.isNull()) {
        return QByteArray();
    }
    QByteArray out;
    QBuffer buffer(&out);
    buffer.open(QIODevice::WriteOnly);
    const QColor matte(Qt::white);
    switch (format) {
    case Png:
        QImageWriter(&buffer, "png").write(transparency_->isChecked() ? image : over(image, matte));
        break;
    case Jpg: {
        QImageWriter writer(&buffer, "jpeg");
        writer.setQuality(quality_->value());
        writer.write(over(image, matte));
        break;
    }
    case Png8:
        QImageWriter(&buffer, "png").write(indexedImage(image, transparency_->isChecked(), nullptr));
        break;
    case Gif:
        indexedImage(image, transparency_->isChecked(), &out);
        break;
    case Svg:
        // SVG is document-level text, written by `exportAsFromView` through the
        // `svg_from_view` bridge; it has no per-entry `QImage` encoding.
        break;
    case Webp: {
        // Qt's WebP writer is an optional plugin, so encode through the engine.
        const QImage source = transparency_->isChecked() ? image : over(image, matte);
        const QImage rgba = rgbaOf(source);
        const ::rust::Vec<std::uint8_t> bytes =
            webp_encode(bytesOf(rgba), rgba.width(), rgba.height());
        out = QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
        break;
    }
    }
    return out;
}

QByteArray ExportAsDialog::encoded(int entry) const
{
    return encodedAs(entry, Format(format_->currentIndex()));
}

QString ExportAsDialog::format() const
{
    return format_->currentData().toString();
}

int ExportAsDialog::quality() const
{
    return quality_->value();
}

int ExportAsDialog::scale() const
{
    return scale_->value();
}

QString ExportAsDialog::suffix() const
{
    return suffixForFormat(format());
}

bool ExportAsDialog::saveTo(int entry, const QString& path) const
{
    const QByteArray data = encodedAs(entry, formatForPath(path, Format(format_->currentIndex())));
    QSaveFile file(path);
    return !data.isEmpty() && file.open(QIODevice::WriteOnly) && file.write(data) == data.size()
        && file.commit();
}

QVector<int> ExportAsDialog::checkedEntries() const
{
    QVector<int> checked;
    for (int row = 0; row < list_->count(); ++row) {
        if (list_->item(row)->checkState() == Qt::Checked) {
            checked.append(row);
        }
    }
    return checked;
}

void ExportAsDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    // The viewport has a real size only once shown; fit then.
    QTimer::singleShot(0, this, [this] { fitZoom(); });
}

void ExportAsDialog::applyZoom(double factor)
{
    fit_ = false;
    zoom_ = factor;
    refreshPreview();
}

void ExportAsDialog::fitZoom()
{
    fit_ = true;
    refreshPreview();
}

void ExportAsDialog::scheduleSizeEstimate()
{
    sizeTimer_->start();
}

void ExportAsDialog::refreshPreview()
{
    const int entry = currentEntry();
    if (entry < 0) {
        preview_->clear();
        sizeLabel_->clear();
        return;
    }
    const QImage image = sized(entry);
    if (image.isNull()) {
        preview_->clear();
        return;
    }
    if (fit_) {
        const QSize available = previewArea_->viewport()->size();
        const double factor = qMin(double(available.width()) / image.width(),
                                   double(available.height()) / image.height());
        zoom_ = qBound(0.01, factor, 8.0);
    }
    zoomLabel_->setText(QStringLiteral("%1%").arg(qRound(zoom_ * 100)));
    const QSize shown(qMax(1, qRound(image.width() * zoom_)),
                      qMax(1, qRound(image.height() * zoom_)));
    preview_->setPixmap(QPixmap::fromImage(
        image.scaled(shown, Qt::IgnoreAspectRatio, Qt::SmoothTransformation)));
    preview_->resize(shown);
    scheduleSizeEstimate();
}

bool exportAsFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return false;
    }
    const int width = view->document_width();
    const int height = view->document_height();
    QVector<ExportEntry> entries;
    {
        const ::rust::Vec<std::uint8_t> rgba = web_flattened(*view);
        if (rgba.size() != std::size_t(width) * std::size_t(height) * 4) {
            return false;
        }
        QString name = QFileInfo(document_api::filePath(*view)).completeBaseName();
        if (name.isEmpty()) {
            name = QStringLiteral("Untitled");
        }
        entries.append({name, QStringLiteral("document"),
                        QImage(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888)
                            .copy()});
    }
    const int layerCount = view->layer_count();
    for (int i = 0; i < layerCount; ++i) {
        if (view->layer_kind(i) == QStringLiteral("adjustment")) {
            continue;
        }
        const ::rust::Vec<std::uint8_t> packed = layer_flattened(*view, i);
        if (packed.size() < 8) {
            continue;
        }
        const auto u32 = [&packed](int at) {
            return int(packed[at]) | (int(packed[at + 1]) << 8) | (int(packed[at + 2]) << 16)
                | (int(packed[at + 3]) << 24);
        };
        const int lw = u32(0);
        const int lh = u32(4);
        if (lw <= 0 || lh <= 0
            || packed.size() != std::size_t(8) + std::size_t(lw) * std::size_t(lh) * 4) {
            continue;
        }
        entries.append({view->layer_name(i), view->layer_kind(i),
                        QImage(packed.data() + 8, lw, lh, lw * 4, QImage::Format_RGBA8888).copy()});
    }

    ExportAsDialog dialog(entries, parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    const QVector<int> checked = dialog.checkedEntries();
    if (checked.isEmpty()) {
        return false;
    }
    const QString documentPath = document_api::filePath(*view);

    // SVG is document-level text, not a per-entry `QImage`: one `.svg` holds the
    // whole document, so the checked entries do not select targets.
    if (dialog.format() == QLatin1String("SVG")) {
        const QString svg = svg_from_view(*view);
        if (svg.isEmpty()) {
            return false;
        }
        QString directory = QFileInfo(documentPath).absolutePath();
        if (directory.isEmpty()) {
            directory = defaultFileDialogDirectory();
        }
        const QString path = getSaveFileName(
            parent, QObject::tr("Export As"),
            directory + QLatin1Char('/') + entries[0].name + QStringLiteral(".svg"),
            {QStringLiteral("SVG (*.SVG)")}, nullptr);
        if (path.isEmpty()) {
            return false;
        }
        const QByteArray data = svg.toUtf8();
        QSaveFile file(retargetPath(path, QStringLiteral("SVG")));
        return file.open(QIODevice::WriteOnly) && file.write(data) == data.size()
            && file.commit();
    }

    if (checked.size() == 1) {
        const int entry = checked.first();
        const QString name = entries[entry].name;
        const QString suffix = dialog.suffix();
        QString directory = QFileInfo(documentPath).absolutePath();
        if (directory.isEmpty()) {
            directory = defaultFileDialogDirectory();
        }
        const QString path = getSaveFileName(
            parent, QObject::tr("Export As"),
            directory + QLatin1Char('/') + name + QLatin1Char('.') + suffix,
            {filterForFormat(dialog.format())}, nullptr);
        return !path.isEmpty() && dialog.saveTo(entry, retargetPath(path, dialog.format()));
    }

    QString directory = QFileInfo(documentPath).absolutePath();
    if (directory.isEmpty()) {
        directory = defaultFileDialogDirectory();
    }
    directory = QFileDialog::getExistingDirectory(parent, QObject::tr("Export As"), directory);
    if (directory.isEmpty()) {
        return false;
    }
    const QString suffix = dialog.suffix();
    QSet<QString> used;
    bool ok = true;
    for (const int entry : checked) {
        QString base = sanitizeName(entries[entry].name);
        QString candidate = base;
        int counter = 2;
        while (used.contains(candidate.toLower())) {
            candidate = QStringLiteral("%1-%2").arg(base).arg(counter++);
        }
        used.insert(candidate.toLower());
        const QString path = directory + QLatin1Char('/') + candidate + QLatin1Char('.') + suffix;
        ok = dialog.saveTo(entry, path) && ok;
    }
    return ok;
}

bool quickExportPngFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return false;
    }
    const QString source = document_api::filePath(*view);
    QString path;
    bool prompt = source.isEmpty();
    if (!source.isEmpty()) {
        const QFileInfo info(source);
        path = info.absolutePath() + QLatin1Char('/') + info.completeBaseName()
            + QStringLiteral(".png");
        // A PNG-backed document's target equals its own file; prompt for another
        // name rather than overwriting the source with the flattened composite.
        prompt = QFileInfo(path).absoluteFilePath() == QFileInfo(source).absoluteFilePath();
    }
    if (prompt) {
        path = getSaveFileName(parent, QObject::tr("Quick Export as PNG"),
                               source.isEmpty() ? QString() : path, {QStringLiteral("PNG (*.PNG)")},
                               nullptr);
        if (path.isEmpty()) {
            return false;
        }
    }
    // Quick Export is always PNG, so force the extension regardless of a typed one.
    const QString suffix = QFileInfo(path).suffix();
    QString base = path;
    if (!suffix.isEmpty()) {
        base.chop(suffix.size() + 1);
    }
    const QString target = lowercasedImageSuffix(base + QStringLiteral(".png"));
    return export_image(*view, target, QStringLiteral("PNG"), -1, 100);
}

} // namespace pictura
