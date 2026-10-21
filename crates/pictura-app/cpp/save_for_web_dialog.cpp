#include "save_for_web_dialog.h"

#include "dialogs.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/web_export.cxxqt.h"

#include <QtCore/QBuffer>
#include <QtCore/QFileInfo>
#include <QtCore/QSaveFile>
#include <QtCore/QSignalBlocker>
#include <QtGui/QImageWriter>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QVBoxLayout>

#include <array>
#include <cstring>

namespace pictura {

namespace {

constexpr int kPreviewSize = 320;

// The settings Done keeps for the next time the dialog opens (session only).
struct Remembered {
    bool valid = false;
    int format = 0;
    int reduction = 1;
    int colors = 128;
    int dither = 1;
    int ditherAmount = 88;
    bool transparency = true;
    bool interlaced = false;
    int matte = 0;
    int quality = 60;
    bool progressive = false;
    bool optimized = true;
    bool pngTransparency = true;
    int wbmpDither = 1;
    int wbmpAmount = 88;
};

Remembered& remembered()
{
    static Remembered settings;
    return settings;
}

// CS6's stock presets: name, format, reduction, colours, dither, JPEG quality.
struct Preset {
    const char* name;
    int format;
    int colors;
    int dither;
    int quality;
};

constexpr Preset kPresets[] = {
    {"GIF 128 Dithered", SaveForWebDialog::Gif, 128, 1, 0},
    {"GIF 64 No Dither", SaveForWebDialog::Gif, 64, 0, 0},
    {"GIF 32 Dithered", SaveForWebDialog::Gif, 32, 1, 0},
    {"JPEG High", SaveForWebDialog::Jpeg, 0, 0, 60},
    {"JPEG Medium", SaveForWebDialog::Jpeg, 0, 0, 30},
    {"JPEG Low", SaveForWebDialog::Jpeg, 0, 0, 10},
    {"PNG-8 128 Dithered", SaveForWebDialog::Png8, 128, 1, 0},
    {"PNG-24", SaveForWebDialog::Png24, 0, 0, 0},
};

QString sizeText(qsizetype bytes)
{
    if (bytes < 1024) {
        return QStringLiteral("%1 bytes").arg(bytes);
    }
    return QStringLiteral("%1K").arg(double(bytes) / 1024.0, 0, 'f', 1);
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

// A WBMP preview: Qt has no WBMP reader, and the layout is trivial.
QImage decodeWbmp(const QByteArray& data, int width, int height)
{
    QImage out(width, height, QImage::Format_RGB32);
    int pos = 2;
    for (int skip = 0; skip < 2; ++skip) {
        while (pos < data.size() && (quint8(data[pos]) & 0x80)) {
            ++pos;
        }
        ++pos;
    }
    const int row = (width + 7) / 8;
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            const int at = pos + y * row + x / 8;
            const bool white = at < data.size() && (quint8(data[at]) & (0x80 >> (x % 8)));
            out.setPixel(x, y, white ? qRgb(255, 255, 255) : qRgb(0, 0, 0));
        }
    }
    return out;
}

} // namespace

SaveForWebDialog::SaveForWebDialog(const QImage& image, QWidget* parent)
    : QDialog(parent)
    , original_(rgbaOf(image))
{
    setObjectName(QStringLiteral("saveForWebDialog"));
    setWindowTitle(QStringLiteral("Save for Web & Devices"));
    buildUi();
    restore();
    refresh();
}

void SaveForWebDialog::buildUi()
{
    auto* root = new QHBoxLayout(this);

    auto* left = new QVBoxLayout;
    tabs_ = new QTabBar(this);
    tabs_->setObjectName(QStringLiteral("saveForWebTabs"));
    for (const char* tab : {"Original", "Optimized", "2-Up", "4-Up"}) {
        tabs_->addTab(QString::fromLatin1(tab));
    }
    tabs_->setTabEnabled(3, false);
    tabs_->setTabToolTip(3, QStringLiteral("4-Up: not implemented yet"));
    tabs_->setCurrentIndex(1);
    left->addWidget(tabs_);
    auto* panes = new QHBoxLayout;
    const auto pane = [this, panes](QLabel*& view, QLabel*& info, const char* name) {
        auto* column = new QVBoxLayout;
        view = new QLabel(this);
        view->setObjectName(QString::fromLatin1(name));
        view->setAlignment(Qt::AlignCenter);
        view->setMinimumSize(kPreviewSize / 2, kPreviewSize / 2);
        info = new QLabel(this);
        info->setObjectName(QString::fromLatin1(name) + QStringLiteral("Info"));
        column->addWidget(view, 1);
        column->addWidget(info);
        panes->addLayout(column);
    };
    pane(originalView_, originalInfo_, "saveForWebOriginal");
    pane(optimizedView_, optimizedInfo_, "saveForWebOptimized");
    left->addLayout(panes, 1);
    root->addLayout(left, 1);

    auto* right = new QVBoxLayout;
    auto* top = new QFormLayout;
    preset_ = new QComboBox(this);
    preset_->setObjectName(QStringLiteral("saveForWebPreset"));
    preset_->addItem(QStringLiteral("[Unnamed]"));
    for (const Preset& p : kPresets) {
        preset_->addItem(QString::fromLatin1(p.name));
    }
    format_ = new QComboBox(this);
    format_->setObjectName(QStringLiteral("saveForWebFormat"));
    format_->addItems({QStringLiteral("GIF"), QStringLiteral("JPEG"), QStringLiteral("PNG-8"),
                       QStringLiteral("PNG-24"), QStringLiteral("WBMP")});
    top->addRow(QStringLiteral("Preset:"), preset_);
    top->addRow(QStringLiteral("Format:"), format_);
    right->addLayout(top);

    settings_ = new QStackedWidget(this);
    // GIF and PNG-8 share one page.
    auto* palette = new QWidget(settings_);
    auto* pf = new QFormLayout(palette);
    reduction_ = new QComboBox(palette);
    reduction_->setObjectName(QStringLiteral("saveForWebReduction"));
    reduction_->addItems({QStringLiteral("Perceptual"), QStringLiteral("Selective"),
                          QStringLiteral("Adaptive"), QStringLiteral("Restrictive"),
                          QStringLiteral("Black & White"), QStringLiteral("Grayscale")});
    colors_ = new QSpinBox(palette);
    colors_->setObjectName(QStringLiteral("saveForWebColors"));
    colors_->setRange(2, 256);
    dither_ = new QComboBox(palette);
    dither_->setObjectName(QStringLiteral("saveForWebDither"));
    dither_->addItems({QStringLiteral("No Dither"), QStringLiteral("Diffusion"),
                       QStringLiteral("Pattern"), QStringLiteral("Noise")});
    ditherAmount_ = new QSpinBox(palette);
    ditherAmount_->setObjectName(QStringLiteral("saveForWebDitherAmount"));
    ditherAmount_->setRange(0, 100);
    ditherAmount_->setSuffix(QStringLiteral("%"));
    transparency_ = new QCheckBox(QStringLiteral("Transparency"), palette);
    transparency_->setObjectName(QStringLiteral("saveForWebTransparency"));
    interlaced_ = new QCheckBox(QStringLiteral("Interlaced"), palette);
    interlaced_->setObjectName(QStringLiteral("saveForWebInterlaced"));
    matte_ = new QComboBox(palette);
    matte_->setObjectName(QStringLiteral("saveForWebMatte"));
    matte_->addItems({QStringLiteral("White"), QStringLiteral("Black")});
    pf->addRow(QStringLiteral("Color Reduction:"), reduction_);
    pf->addRow(QStringLiteral("Colors:"), colors_);
    pf->addRow(QStringLiteral("Dither:"), dither_);
    pf->addRow(QStringLiteral("Amount:"), ditherAmount_);
    pf->addRow(transparency_);
    pf->addRow(QStringLiteral("Matte:"), matte_);
    pf->addRow(interlaced_);
    settings_->addWidget(palette);

    auto* jpeg = new QWidget(settings_);
    auto* jf = new QFormLayout(jpeg);
    qualityPreset_ = new QComboBox(jpeg);
    qualityPreset_->setObjectName(QStringLiteral("saveForWebQualityPreset"));
    qualityPreset_->addItems({QStringLiteral("Low"), QStringLiteral("Medium"),
                              QStringLiteral("High"), QStringLiteral("Very High"),
                              QStringLiteral("Maximum")});
    quality_ = new QSpinBox(jpeg);
    quality_->setObjectName(QStringLiteral("saveForWebQuality"));
    quality_->setRange(0, 100);
    progressive_ = new QCheckBox(QStringLiteral("Progressive"), jpeg);
    progressive_->setObjectName(QStringLiteral("saveForWebProgressive"));
    optimized_ = new QCheckBox(QStringLiteral("Optimized"), jpeg);
    optimized_->setObjectName(QStringLiteral("saveForWebOptimizedJpeg"));
    jf->addRow(qualityPreset_);
    jf->addRow(QStringLiteral("Quality:"), quality_);
    jf->addRow(optimized_);
    jf->addRow(progressive_);
    settings_->addWidget(jpeg);

    auto* png24 = new QWidget(settings_);
    auto* p24 = new QFormLayout(png24);
    pngTransparency_ = new QCheckBox(QStringLiteral("Transparency"), png24);
    pngTransparency_->setObjectName(QStringLiteral("saveForWebPngTransparency"));
    p24->addRow(pngTransparency_);
    settings_->addWidget(png24);

    auto* wbmp = new QWidget(settings_);
    auto* wf = new QFormLayout(wbmp);
    wbmpDither_ = new QComboBox(wbmp);
    wbmpDither_->setObjectName(QStringLiteral("saveForWebWbmpDither"));
    wbmpDither_->addItems({QStringLiteral("No Dither"), QStringLiteral("Diffusion"),
                           QStringLiteral("Pattern"), QStringLiteral("Noise")});
    wbmpAmount_ = new QSpinBox(wbmp);
    wbmpAmount_->setObjectName(QStringLiteral("saveForWebWbmpAmount"));
    wbmpAmount_->setRange(0, 100);
    wbmpAmount_->setSuffix(QStringLiteral("%"));
    wf->addRow(QStringLiteral("Dither:"), wbmpDither_);
    wf->addRow(QStringLiteral("Amount:"), wbmpAmount_);
    settings_->addWidget(wbmp);
    right->addWidget(settings_);

    auto* sizeBox = new QGroupBox(QStringLiteral("Image Size"), this);
    auto* sf = new QFormLayout(sizeBox);
    width_ = new QSpinBox(sizeBox);
    width_->setObjectName(QStringLiteral("saveForWebWidth"));
    height_ = new QSpinBox(sizeBox);
    height_->setObjectName(QStringLiteral("saveForWebHeight"));
    percent_ = new QSpinBox(sizeBox);
    percent_->setObjectName(QStringLiteral("saveForWebPercent"));
    for (QSpinBox* box : {width_, height_}) {
        box->setRange(1, 30000);
        box->setSuffix(QStringLiteral(" px"));
    }
    percent_->setRange(1, 1000);
    percent_->setSuffix(QStringLiteral("%"));
    width_->setValue(original_.width());
    height_->setValue(original_.height());
    percent_->setValue(100);
    constrain_ = new QCheckBox(QStringLiteral("Constrain Proportions"), sizeBox);
    constrain_->setChecked(true);
    resample_ = new QComboBox(sizeBox);
    resample_->setObjectName(QStringLiteral("saveForWebResample"));
    resample_->addItems({QStringLiteral("Bicubic"), QStringLiteral("Nearest Neighbor")});
    sf->addRow(QStringLiteral("W:"), width_);
    sf->addRow(QStringLiteral("H:"), height_);
    sf->addRow(QStringLiteral("Percent:"), percent_);
    sf->addRow(constrain_);
    sf->addRow(QStringLiteral("Quality:"), resample_);
    right->addWidget(sizeBox);
    right->addStretch(1);

    auto* buttons = new QDialogButtonBox(this);
    QPushButton* save = buttons->addButton(QStringLiteral("Save..."), QDialogButtonBox::AcceptRole);
    save->setObjectName(QStringLiteral("saveForWebSave"));
    buttons->addButton(QDialogButtonBox::Cancel);
    QPushButton* doneButton =
        buttons->addButton(QStringLiteral("Done"), QDialogButtonBox::ActionRole);
    doneButton->setObjectName(QStringLiteral("saveForWebDone"));
    connect(buttons, &QDialogButtonBox::accepted, this, [this]() {
        remember();
        accept();
    });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(doneButton, &QPushButton::clicked, this, [this]() {
        remember();
        done(QDialog::Rejected);
    });
    right->addWidget(buttons);
    root->addLayout(right);

    // Any setting change re-optimises; the preset falls back to [Unnamed].
    const auto changed = [this]() {
        const QSignalBlocker block(preset_);
        preset_->setCurrentIndex(0);
        refresh();
    };
    for (QComboBox* combo : {format_, reduction_, dither_, matte_, wbmpDither_, resample_}) {
        connect(combo, &QComboBox::currentIndexChanged, this, changed);
    }
    for (QSpinBox* box : {colors_, ditherAmount_, quality_, wbmpAmount_}) {
        connect(box, &QSpinBox::valueChanged, this, changed);
    }
    for (QCheckBox* box : {transparency_, interlaced_, progressive_, optimized_, pngTransparency_}) {
        connect(box, &QCheckBox::toggled, this, changed);
    }
    connect(tabs_, &QTabBar::currentChanged, this, [this]() { refresh(); });
    connect(preset_, &QComboBox::currentIndexChanged, this, [this](int i) { applyPreset(i); });
    connect(qualityPreset_, &QComboBox::currentIndexChanged, this,
            [this](int i) { quality_->setValue(std::array{10, 30, 60, 80, 100}[i]); });
    connect(width_, &QSpinBox::valueChanged, this, [this](int w) {
        if (constrain_->isChecked()) {
            const QSignalBlocker blockH(height_);
            const QSignalBlocker blockP(percent_);
            height_->setValue(qMax(1, qRound(double(w) * original_.height() / original_.width())));
            percent_->setValue(qRound(100.0 * w / original_.width()));
        }
        refresh();
    });
    connect(height_, &QSpinBox::valueChanged, this, [this](int h) {
        if (constrain_->isChecked()) {
            const QSignalBlocker blockW(width_);
            const QSignalBlocker blockP(percent_);
            width_->setValue(qMax(1, qRound(double(h) * original_.width() / original_.height())));
            percent_->setValue(qRound(100.0 * h / original_.height()));
        }
        refresh();
    });
    connect(percent_, &QSpinBox::valueChanged, this, [this](int p) {
        const QSignalBlocker blockW(width_);
        const QSignalBlocker blockH(height_);
        width_->setValue(qMax(1, qRound(original_.width() * p / 100.0)));
        height_->setValue(qMax(1, qRound(original_.height() * p / 100.0)));
        refresh();
    });
}

void SaveForWebDialog::applyPreset(int index)
{
    if (index <= 0 || index > int(std::size(kPresets))) {
        return;
    }
    const Preset& p = kPresets[index - 1];
    const QList<QWidget*> fields = {format_, colors_, dither_, quality_};
    for (QWidget* w : fields) {
        w->blockSignals(true);
    }
    format_->setCurrentIndex(p.format);
    if (p.colors > 0) {
        colors_->setValue(p.colors);
        dither_->setCurrentIndex(p.dither);
    }
    if (p.quality > 0) {
        quality_->setValue(p.quality);
    }
    for (QWidget* w : fields) {
        w->blockSignals(false);
    }
    refresh();
}

void SaveForWebDialog::remember() const
{
    Remembered& r = remembered();
    r.valid = true;
    r.format = format_->currentIndex();
    r.reduction = reduction_->currentIndex();
    r.colors = colors_->value();
    r.dither = dither_->currentIndex();
    r.ditherAmount = ditherAmount_->value();
    r.transparency = transparency_->isChecked();
    r.interlaced = interlaced_->isChecked();
    r.matte = matte_->currentIndex();
    r.quality = quality_->value();
    r.progressive = progressive_->isChecked();
    r.optimized = optimized_->isChecked();
    r.pngTransparency = pngTransparency_->isChecked();
    r.wbmpDither = wbmpDither_->currentIndex();
    r.wbmpAmount = wbmpAmount_->value();
}

void SaveForWebDialog::restore()
{
    // The first open is CS6's default, GIF 128 Dithered with Selective.
    const Remembered r = remembered();
    const QList<QWidget*> fields = findChildren<QWidget*>();
    for (QWidget* w : fields) {
        w->blockSignals(true);
    }
    format_->setCurrentIndex(r.format);
    reduction_->setCurrentIndex(r.reduction);
    colors_->setValue(r.colors);
    dither_->setCurrentIndex(r.dither);
    ditherAmount_->setValue(r.ditherAmount);
    transparency_->setChecked(r.transparency);
    interlaced_->setChecked(r.interlaced);
    matte_->setCurrentIndex(r.matte);
    quality_->setValue(r.quality);
    qualityPreset_->setCurrentIndex(2);
    progressive_->setChecked(r.progressive);
    optimized_->setChecked(r.optimized);
    pngTransparency_->setChecked(r.pngTransparency);
    wbmpDither_->setCurrentIndex(r.wbmpDither);
    wbmpAmount_->setValue(r.wbmpAmount);
    preset_->setCurrentIndex(r.valid ? 0 : 1);
    for (QWidget* w : fields) {
        w->blockSignals(false);
    }
}

QImage SaveForWebDialog::sized() const
{
    if (width_->value() == original_.width() && height_->value() == original_.height()) {
        return original_;
    }
    return rgbaOf(original_.scaled(width_->value(), height_->value(), Qt::IgnoreAspectRatio,
                                   resample_->currentIndex() == 0 ? Qt::SmoothTransformation
                                                                  : Qt::FastTransformation));
}

// Reduce `rgba` with the GIF / PNG-8 settings; with `gif`, also encode it.
QImage SaveForWebDialog::indexedImage(const QImage& rgba, QByteArray* gif) const
{
    WebPalette palette{};
    palette.reduction = reduction_->currentIndex();
    palette.colors = colors_->value();
    palette.dither = dither_->currentIndex();
    palette.amount = ditherAmount_->value();
    palette.transparency = transparency_->isChecked();
    palette.matte = matte_->currentIndex() == 1 ? 0xff000000u : 0xffffffffu;
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
            web_encode_gif(view, rgba.width(), rgba.height(), interlaced_->isChecked());
        *gif = QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
    }
    return indexed;
}

QByteArray SaveForWebDialog::encoded() const
{
    const QImage image = sized();
    QByteArray out;
    QBuffer buffer(&out);
    buffer.open(QIODevice::WriteOnly);
    const QColor matte = matte_->currentIndex() == 1 ? QColor(Qt::black) : QColor(Qt::white);
    switch (format_->currentIndex()) {
    case Gif:
        indexedImage(image, &out);
        break;
    case Jpeg: {
        QImageWriter writer(&buffer, "jpeg");
        writer.setQuality(quality_->value());
        writer.setOptimizedWrite(optimized_->isChecked());
        writer.setProgressiveScanWrite(progressive_->isChecked());
        writer.write(over(image, matte));
        break;
    }
    case Png8:
        QImageWriter(&buffer, "png").write(indexedImage(image, nullptr));
        break;
    case Png24:
        QImageWriter(&buffer, "png")
            .write(pngTransparency_->isChecked() ? image : over(image, matte));
        break;
    case Wbmp: {
        const ::rust::Vec<std::uint8_t> bytes =
            web_encode_wbmp(bytesOf(image), image.width(), image.height(),
                            wbmpDither_->currentIndex(), wbmpAmount_->value());
        out = QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
        break;
    }
    }
    return out;
}

QString SaveForWebDialog::suffix() const
{
    switch (format_->currentIndex()) {
    case Jpeg:
        return QStringLiteral("jpg");
    case Png8:
    case Png24:
        return QStringLiteral("png");
    case Wbmp:
        return QStringLiteral("wbmp");
    default:
        return QStringLiteral("gif");
    }
}

bool SaveForWebDialog::saveTo(const QString& path) const
{
    const QByteArray data = encoded();
    QSaveFile file(path);
    return !data.isEmpty() && file.open(QIODevice::WriteOnly) && file.write(data) == data.size()
        && file.commit();
}

// Re-optimise and redraw the panes: the original, the optimised file decoded
// back (so the preview is what will be written), or both for 2-Up.
void SaveForWebDialog::refresh()
{
    settings_->setCurrentIndex(std::array{0, 1, 0, 2, 3}[format_->currentIndex()]);
    interlaced_->setEnabled(format_->currentIndex() == Gif);
    const QImage image = sized();
    const QByteArray data = encoded();
    QImage optimized = format_->currentIndex() == Wbmp
        ? decodeWbmp(data, image.width(), image.height())
        : QImage::fromData(data);
    const int tab = tabs_->currentIndex();
    const auto show = [](QLabel* view, const QImage& shown) {
        view->setPixmap(QPixmap::fromImage(shown.scaled(kPreviewSize, kPreviewSize,
                                                        Qt::KeepAspectRatio,
                                                        Qt::SmoothTransformation)));
    };
    originalView_->setVisible(tab != 1);
    originalInfo_->setVisible(tab != 1);
    optimizedView_->setVisible(tab != 0);
    optimizedInfo_->setVisible(tab != 0);
    show(originalView_, image);
    show(optimizedView_, optimized);
    originalInfo_->setText(QStringLiteral("Original\n%1 x %2")
                               .arg(image.width())
                               .arg(image.height()));
    const double seconds = data.size() * 8.0 / 56600.0;
    optimizedInfo_->setText(QStringLiteral("%1\n%2\n%3 sec @ 56.6 Kbps")
                                .arg(format_->currentText(), sizeText(data.size()))
                                .arg(qMax(1, qRound(seconds))));
}

bool saveForWebFromView(QWidget* parent, PictureView* view, const QString& documentName)
{
    if (!view || !view->has_document()) {
        return false;
    }
    const int w = view->document_width();
    const int h = view->document_height();
    const ::rust::Vec<std::uint8_t> rgba = web_flattened(*view);
    if (rgba.size() != std::size_t(w) * std::size_t(h) * 4) {
        return false;
    }
    const QImage image =
        QImage(rgba.data(), w, h, w * 4, QImage::Format_RGBA8888).copy();
    SaveForWebDialog dialog(image, parent);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    const QString base = QFileInfo(documentName).completeBaseName();
    const QString suffix = dialog.suffix();
    QString filter;
    const QString path = getSaveFileName(
        parent, QStringLiteral("Save Optimized As"),
        defaultFileDialogDirectory() + QLatin1Char('/') + base + QLatin1Char('.') + suffix,
        {QStringLiteral("Images (*.%1)").arg(suffix)}, &filter);
    return !path.isEmpty() && dialog.saveTo(path);
}

} // namespace pictura

#include "moc_save_for_web_dialog.cpp"
