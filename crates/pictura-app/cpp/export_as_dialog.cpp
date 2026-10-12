#include "export_as_dialog.h"

#include "api/document_api.h"
#include "dialogs.h"
#include "panels/jump_slider.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/export.cxxqt.h"

#include <QtCore/QFileInfo>
#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>

namespace pictura {

namespace {

struct FormatEntry {
    const char* label;
    const char* value;
    const char* suffix;
};

constexpr FormatEntry kFormats[] = {
    {"PNG", "PNG", "png"},
    {"JPEG", "JPG", "jpg"},
    {"TIFF", "TIF", "tif"},
    {"WebP", "WEBP", "webp"},
    {"BMP", "BMP", "bmp"},
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

// The combo's human label plus its uppercase pattern, e.g. "JPEG (*.JPG)".
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

// Map a path suffix to the export format value, or empty if it is not one of
// the formats Export As can write.
QString formatForSuffix(const QString& suffix)
{
    const QString lower = suffix.toLower();
    for (const FormatEntry& entry : kFormats) {
        if (lower == QLatin1String(entry.suffix)) {
            return QString::fromLatin1(entry.value);
        }
    }
    return QString();
}

// Export As honors a supported raster suffix; an empty or unknown suffix is
// replaced with the chosen format's suffix so the path always names a format we
// can write.
QString retargetExportPath(const QString& path, const QString& format)
{
    const QString suffix = QFileInfo(path).suffix();
    if (!formatForSuffix(suffix).isEmpty()) {
        return path;
    }
    QString base = path;
    if (!suffix.isEmpty()) {
        base.chop(suffix.size() + 1);
    }
    return base + QLatin1Char('.') + suffixForFormat(format);
}

// Map the view's remembered source format to one of the dialog's values.
QString canonicalOutputFormat(const QString& sourceFormat)
{
    if (sourceFormat == QLatin1String("jpg") || sourceFormat == QLatin1String("jpeg")) {
        return QStringLiteral("JPG");
    }
    if (sourceFormat == QLatin1String("tif") || sourceFormat == QLatin1String("tiff")) {
        return QStringLiteral("TIF");
    }
    if (sourceFormat == QLatin1String("webp")) {
        return QStringLiteral("WEBP");
    }
    if (sourceFormat == QLatin1String("bmp")) {
        return QStringLiteral("BMP");
    }
    return QStringLiteral("PNG");
}

} // namespace

ExportAsDialog::ExportAsDialog(const QString& initialFormat, QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Export As"));

    auto* form = new QFormLayout(this);

    format_ = new QComboBox(this);
    for (const FormatEntry& entry : kFormats) {
        format_->addItem(QString::fromLatin1(entry.label), QString::fromLatin1(entry.value));
    }
    const int index = format_->findData(initialFormat);
    format_->setCurrentIndex(index >= 0 ? index : 0);
    form->addRow(tr("Format:"), format_);

    qualityLabel_ = new QLabel(tr("Quality:"), this);
    quality_ = new JumpSlider(Qt::Horizontal, this);
    quality_->setRange(1, 100);
    quality_->setValue(90);
    form->addRow(qualityLabel_, quality_);

    scale_ = new QSpinBox(this);
    scale_->setRange(1, 1000);
    scale_->setValue(100);
    scale_->setSuffix(QStringLiteral("%"));
    form->addRow(tr("Scale:"), scale_);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    form->addRow(buttons);

    connect(format_, &QComboBox::currentIndexChanged, this,
            [this] { updateQualityEnabled(); });
    installRollingTypeAhead(format_);
    updateQualityEnabled();
}

void ExportAsDialog::updateQualityEnabled()
{
    const bool lossy = format() == QLatin1String("JPG") || format() == QLatin1String("WEBP");
    quality_->setEnabled(lossy);
    qualityLabel_->setEnabled(lossy);
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

bool exportAsFromView(QWidget* parent, PictureView* view)
{
    if (!view || !view->has_document()) {
        return false;
    }
    ExportAsDialog dialog(canonicalOutputFormat(output_format(*view)), parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    const QString chosen = dialog.format();
    QString path = getSaveFileName(parent, QObject::tr("Export As"), document_api::filePath(*view),
                                   {filterForFormat(chosen)}, nullptr);
    if (path.isEmpty()) {
        return false;
    }
    // The suffix is authoritative, exactly as in Save: a typed `.jpg` writes
    // JPEG even when the combo said PNG, and an unknown suffix falls back to the
    // chosen format.
    const QString target = lowercasedImageSuffix(retargetExportPath(path, chosen));
    const QString format = formatForSuffix(QFileInfo(target).suffix());
    return export_image(*view, target, format, dialog.quality(), dialog.scale());
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
        prompt = QFileInfo(path).absoluteFilePath()
            == QFileInfo(source).absoluteFilePath();
    }
    if (prompt) {
        path = getSaveFileName(parent, QObject::tr("Quick Export as PNG"),
                               source.isEmpty() ? QString() : path,
                               {QStringLiteral("PNG (*.PNG)")}, nullptr);
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
