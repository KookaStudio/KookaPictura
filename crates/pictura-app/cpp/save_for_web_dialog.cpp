#include "save_for_web_dialog.h"

#include "api/engine_bridge.h"
#include "dialogs.h"
#include "save_for_web_color_table.h"
#include "save_optimized_dialog.h"

#include <QtCore/QDir>
#include <QtCore/QEvent>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include "protected_output.h"
#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QVBoxLayout>

#include <array>
#include <cstddef>
#include <iterator>
#include <string>
#include <utility>

namespace pictura {

namespace {

constexpr int kPreviewSize = 320;

struct Remembered {
    bool valid = false;
    WebSettings settings;
    int metadataLevel = 0;
};

Remembered& remembered()
{
    static Remembered value;
    return value;
}

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

::rust::Slice<const std::uint8_t> bytesOf(const QByteArray& data)
{
    return {reinterpret_cast<const std::uint8_t*>(data.constData()),
            std::size_t(data.size())};
}

QByteArray fromRust(const ::rust::Vec<std::uint8_t>& data)
{
    return QByteArray(reinterpret_cast<const char*>(data.data()), qsizetype(data.size()));
}

} // namespace

SaveForWebDialog::SaveForWebDialog(const QImage& image, QWidget* parent)
    : SaveForWebDialog(image, WebMetadata{}, parent)
{
}

SaveForWebDialog::SaveForWebDialog(const QImage& image, WebMetadata metadata,
                                   QWidget* parent)
    : QDialog(parent)
    , original_(image.convertToFormat(QImage::Format_RGBA8888))
    , rawMetadata_(std::move(metadata))
{
    setObjectName(QStringLiteral("saveForWebDialog"));
    setWindowTitle(QStringLiteral("Save for Web & Devices"));
    buildUi();
    restore();
    refresh();
    for (int i = 1; i < views_.size(); ++i) {
        views_[i]->installEventFilter(this);
    }
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
    tabs_->setCurrentIndex(1);
    left->addWidget(tabs_);

    auto* panes = new QGridLayout;
    for (int i = 0; i < 4; ++i) {
        auto* column = new QVBoxLayout;
        auto* view = new QLabel(this);
        view->setObjectName(i == 0 ? QStringLiteral("saveForWebOriginal")
                                   : i == 1 ? QStringLiteral("saveForWebOptimized")
                                            : QStringLiteral("saveForWebPane%1").arg(i + 1));
        view->setAlignment(Qt::AlignCenter);
        view->setMinimumSize(kPreviewSize / 2, kPreviewSize / 2);
        auto* info = new QLabel(this);
        info->setObjectName(i == 0 ? QStringLiteral("saveForWebOriginalInfo")
                                   : i == 1 ? QStringLiteral("saveForWebOptimizedInfo")
                                            : QStringLiteral("saveForWebPane%1Info").arg(i + 1));
        column->addWidget(view, 1);
        column->addWidget(info);
        panes->addLayout(column, i / 2, i % 2);
        views_.append(view);
        infos_.append(info);
    }
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

    colorTable_ = new ColorTableWidget(this);
    right->addWidget(colorTable_);

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

    metadata_ = new QComboBox(this);
    metadata_->setObjectName(QStringLiteral("saveForWebMetadata"));
    metadata_->addItems({QStringLiteral("None"), QStringLiteral("Copyright"),
                         QStringLiteral("Copyright and Contact Info"),
                         QStringLiteral("All Except Camera Info"), QStringLiteral("All")});
    auto* metadataRow = new QFormLayout;
    metadataRow->addRow(QStringLiteral("Metadata:"), metadata_);
    right->addLayout(metadataRow);
    repopulate_ = new QPushButton(QStringLiteral("Repopulate Views"), this);
    repopulate_->setObjectName(QStringLiteral("saveForWebRepopulate"));
    right->addWidget(repopulate_);
    right->addStretch(1);

    auto* buttons = new QDialogButtonBox(this);
    QPushButton* save = buttons->addButton(QStringLiteral("Save..."),
                                           QDialogButtonBox::AcceptRole);
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

    const auto changedEncoding = [this]() {
        {
            const QSignalBlocker block(preset_);
            preset_->setCurrentIndex(0);
        }
        storeActive();
        remember();
        refresh();
    };
    const auto changed = [this, changedEncoding]() {
        panes_[activePane_].packed.clear();
        panes_[activePane_].packedWidth = 0;
        panes_[activePane_].packedHeight = 0;
        changedEncoding();
    };
    const auto changedReduction = [this, changed]() {
        panes_[activePane_].custom.clear();
        panes_[activePane_].mappedTransparency = false;
        changed();
    };
    connect(format_, &QComboBox::currentIndexChanged, this, changedEncoding);
    for (QComboBox* combo : {matte_, resample_}) {
        connect(combo, &QComboBox::currentIndexChanged, this, changed);
    }
    connect(wbmpDither_, &QComboBox::currentIndexChanged, this, changedEncoding);
    connect(reduction_, &QComboBox::currentIndexChanged, this, changedReduction);
    connect(dither_, &QComboBox::currentIndexChanged, this, changedReduction);
    for (QSpinBox* box : {quality_, wbmpAmount_}) {
        connect(box, &QSpinBox::valueChanged, this, changedEncoding);
    }
    connect(colors_, &QSpinBox::valueChanged, this, changedReduction);
    connect(ditherAmount_, &QSpinBox::valueChanged, this, changedReduction);
    for (QCheckBox* box : {interlaced_, progressive_, optimized_, pngTransparency_}) {
        connect(box, &QCheckBox::toggled, this, changedEncoding);
    }
    connect(transparency_, &QCheckBox::toggled, this, changedReduction);
    connect(tabs_, &QTabBar::currentChanged, this, [this]() {
        if (tabs_->currentIndex() != 3 && activePane_ != 1) {
            activePane_ = 1;
            applySettingsToWidgets(panes_[1]);
        }
        refresh();
    });
    connect(preset_, &QComboBox::currentIndexChanged, this,
            [this](int index) { applyPreset(index); });
    connect(qualityPreset_, &QComboBox::currentIndexChanged, this,
            [this](int index) { quality_->setValue(std::array{10, 30, 60, 80, 100}[index]); });
    connect(metadata_, &QComboBox::currentIndexChanged, this, [this]() {
        storeMetadataLevel();
        remember();
        refresh();
    });
    connect(repopulate_, &QPushButton::clicked, this, [this]() { repopulate(); });
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
    connect(colorTable_, &ColorTableWidget::colorsEdited, this,
            [this](const QVector<QRgb>& colors) { onColorsEdited(colors); });
    connect(colorTable_, &ColorTableWidget::addRequested, this,
            [this](QRgb color) { onAddColor(color); });
    connect(colorTable_, &ColorTableWidget::deleteRequested, this,
            [this](int index) { onDeleteColor(index); });
    connect(colorTable_, &ColorTableWidget::sortRequested, this,
            [this](int sort) { onSortColors(sort); });
    connect(colorTable_, &ColorTableWidget::transparencyRequested, this,
            [this](int index, bool transparent) { onTransparency(index, transparent); });
    connect(colorTable_, &ColorTableWidget::loadRequested, this,
            [this]() { onLoadColorTable(); });
    connect(colorTable_, &ColorTableWidget::saveRequested, this,
            [this]() { onSaveColorTable(); });
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
    panes_[activePane_].custom.clear();
    panes_[activePane_].packed.clear();
    panes_[activePane_].packedWidth = 0;
    panes_[activePane_].packedHeight = 0;
    panes_[activePane_].mappedTransparency = false;
    storeActive();
    remember();
    refresh();
}

void SaveForWebDialog::storeActive()
{
    WebSettings s = panes_[activePane_];
    s.format = format_->currentIndex();
    s.reduction = reduction_->currentIndex();
    s.colors = colors_->value();
    s.dither = dither_->currentIndex();
    s.ditherAmount = ditherAmount_->value();
    s.transparency = transparency_->isChecked();
    s.interlaced = interlaced_->isChecked();
    s.matte = matte_->currentIndex();
    s.quality = quality_->value();
    s.progressive = progressive_->isChecked();
    s.optimizedJpeg = optimized_->isChecked();
    s.pngTransparency = pngTransparency_->isChecked();
    s.wbmpDither = wbmpDither_->currentIndex();
    s.wbmpAmount = wbmpAmount_->value();
    panes_[activePane_] = s;
}

void SaveForWebDialog::storeMetadataLevel()
{
    metadataLevel_ = metadata_->currentIndex();
}

void SaveForWebDialog::applySettingsToWidgets(const WebSettings& settings)
{
    const QList<QWidget*> fields = findChildren<QWidget*>();
    for (QWidget* w : fields) {
        w->blockSignals(true);
    }
    format_->setCurrentIndex(settings.format);
    reduction_->setCurrentIndex(settings.reduction);
    colors_->setValue(settings.colors);
    dither_->setCurrentIndex(settings.dither);
    ditherAmount_->setValue(settings.ditherAmount);
    transparency_->setChecked(settings.transparency);
    interlaced_->setChecked(settings.interlaced);
    matte_->setCurrentIndex(settings.matte);
    quality_->setValue(settings.quality);
    qualityPreset_->setCurrentIndex(settings.quality >= 60 ? 2 : settings.quality >= 30 ? 1 : 0);
    progressive_->setChecked(settings.progressive);
    optimized_->setChecked(settings.optimizedJpeg);
    pngTransparency_->setChecked(settings.pngTransparency);
    wbmpDither_->setCurrentIndex(settings.wbmpDither);
    wbmpAmount_->setValue(settings.wbmpAmount);
    const int page = std::array{0, 1, 0, 2, 3}[qBound(0, settings.format, 4)];
    settings_->setCurrentIndex(page);
    for (QWidget* w : fields) {
        w->blockSignals(false);
    }
}

void SaveForWebDialog::remember() const
{
    Remembered& r = remembered();
    const int pane = activePane_ == 0 ? 1 : activePane_;
    r.valid = true;
    r.settings = panes_[pane];
    r.metadataLevel = metadataLevel_;
}

void SaveForWebDialog::restore()
{
    const Remembered r = remembered();
    WebSettings base;
    if (r.valid) {
        base = r.settings;
    } else {
        base.format = Gif;
        base.reduction = 1;
        base.colors = 128;
        base.dither = 1;
        base.ditherAmount = 88;
        base.transparency = true;
        base.quality = 60;
        base.optimizedJpeg = true;
        base.pngTransparency = true;
        base.wbmpDither = 1;
        base.wbmpAmount = 88;
    }
    base.packed.clear();
    base.packedWidth = 0;
    base.packedHeight = 0;
    panes_[0] = base;
    panes_[1] = base;
    panes_[2] = loweredSettings(base, 1);
    panes_[3] = loweredSettings(base, 2);
    activePane_ = 1;
    metadataLevel_ = r.valid ? r.metadataLevel : 0;
    {
        const QSignalBlocker block(metadata_);
        metadata_->setCurrentIndex(metadataLevel_);
    }
    applySettingsToWidgets(base);
    {
        const QSignalBlocker block(preset_);
        preset_->setCurrentIndex(r.valid ? 0 : 1);
    }
}

QImage SaveForWebDialog::sized() const
{
    if (width_->value() == original_.width() && height_->value() == original_.height()) {
        return original_;
    }
    return original_
        .scaled(width_->value(), height_->value(), Qt::IgnoreAspectRatio,
                resample_->currentIndex() == 0 ? Qt::SmoothTransformation
                                               : Qt::FastTransformation)
        .convertToFormat(QImage::Format_RGBA8888);
}

QByteArray SaveForWebDialog::encodedForPane(int pane) const
{
    const QImage image = sized();
    QByteArray data = encodeWebImage(image, panes_[pane]);
    return embedWebMetadata(data, webSuffix(panes_[pane]), rawMetadata_, metadataLevel_);
}

QByteArray SaveForWebDialog::encoded() const
{
    return encodedForPane(activePane_);
}

bool SaveForWebDialog::historyLogOmitted() const
{
    return web_history_omitted(suffix(), rawMetadata_, metadataLevel_);
}

QString SaveForWebDialog::suffix() const
{
    return webSuffix(panes_[activePane_]);
}

QString SaveForWebDialog::formatName(int format) const
{
    switch (format) {
    case Jpeg:
        return QStringLiteral("JPEG");
    case Png8:
        return QStringLiteral("PNG-8");
    case Png24:
        return QStringLiteral("PNG-24");
    case Wbmp:
        return QStringLiteral("WBMP");
    default:
        return QStringLiteral("GIF");
    }
}

void SaveForWebDialog::refresh()
{
    const QImage image = sized();
    const int tab = tabs_->currentIndex();
    for (int i = 0; i < 4; ++i) {
        const bool visible = tab == 0 ? i == 0 : tab == 1 ? i == 1 : tab == 2 ? i < 2 : true;
        views_[i]->setVisible(visible);
        infos_[i]->setVisible(visible);
        if (!visible) {
            continue;
        }
        if (i == 0) {
            views_[0]->setPixmap(QPixmap::fromImage(image.scaled(
                kPreviewSize, kPreviewSize, Qt::KeepAspectRatio, Qt::SmoothTransformation)));
            infos_[0]->setText(QStringLiteral("Original\n%1 x %2")
                                   .arg(image.width())
                                   .arg(image.height()));
            continue;
        }
        const QByteArray data = encodedForPane(i);
        const QImage shown = panes_[i].format == Wbmp
            ? decodeWbmp(data, image.width(), image.height())
            : QImage::fromData(data);
        views_[i]->setPixmap(QPixmap::fromImage(shown.scaled(
            kPreviewSize, kPreviewSize, Qt::KeepAspectRatio, Qt::SmoothTransformation)));
        const double seconds = data.size() * 8.0 / 56600.0;
        infos_[i]->setText(QStringLiteral("%1\n%2\n%3 sec @ 56.6 Kbps")
                               .arg(formatName(panes_[i].format), sizeText(data.size()))
                               .arg(qMax(1, qRound(seconds))));
        views_[i]->setStyleSheet(i == activePane_
                                     ? QStringLiteral("QLabel { border: 2px solid #d08a00; }")
                                     : QString());
    }
    {
        const QSignalBlocker block(settings_);
        settings_->setCurrentIndex(
            std::array{0, 1, 0, 2, 3}[qBound(0, panes_[activePane_].format, 4)]);
    }
    interlaced_->setEnabled(panes_[activePane_].format == Gif);
    const bool table = panes_[activePane_].format == Gif || panes_[activePane_].format == Png8;
    colorTable_->setVisible(table);
    if (table) {
        syncColorTable();
    }
    repopulate_->setEnabled(tab == 3);
}

void SaveForWebDialog::syncColorTable()
{
    const QByteArray packed = quantizeWebImage(sized(), panes_[activePane_]);
    colorTable_->setColors(paletteFromPacked(packed));
}

void SaveForWebDialog::setActivePane(int pane)
{
    if (pane <= 0 || pane >= 4 || pane == activePane_) {
        return;
    }
    storeActive();
    activePane_ = pane;
    applySettingsToWidgets(panes_[pane]);
    refresh();
}

bool SaveForWebDialog::eventFilter(QObject* watched, QEvent* event)
{
    if (event->type() == QEvent::MouseButtonPress) {
        for (int i = 1; i < views_.size(); ++i) {
            if (watched == views_[i]) {
                setActivePane(i);
                break;
            }
        }
    }
    return QDialog::eventFilter(watched, event);
}

void SaveForWebDialog::repopulate()
{
    storeActive();
    const int active = activePane_ == 0 ? 1 : activePane_;
    const WebSettings base = panes_[active];
    for (int pane = 1; pane < 4; ++pane) {
        panes_[pane] = pane == active ? base : loweredSettings(base, pane);
    }
    applySettingsToWidgets(panes_[activePane_]);
    refresh();
}

void SaveForWebDialog::onColorsEdited(const QVector<QRgb>& colors)
{
    WebSettings& settings = panes_[activePane_];
    const QImage image = sized();
    QByteArray packed = quantizeWebImage(image, settings);
    if (packed.size() < 2) {
        return;
    }
    const int n = (int(quint8(packed[0])) << 8) | int(quint8(packed[1]));
    if (n < 1 || packed.size() < 2 + n * 4 || colors.size() < n) {
        return;
    }
    for (int i = 0; i < n; ++i) {
        const QRgb color = colors[i];
        const int at = 2 + i * 4;
        packed[at] = char(qRed(color));
        packed[at + 1] = char(qGreen(color));
        packed[at + 2] = char(qBlue(color));
        packed[at + 3] = char(qAlpha(color));
    }
    settings.packed = packed;
    settings.packedWidth = image.width();
    settings.packedHeight = image.height();
    settings.custom = colors;
    settings.locked.clear();
    refresh();
}

void SaveForWebDialog::onAddColor(QRgb color)
{
    if (panes_[activePane_].locked.size() >= 256) {
        return;
    }
    panes_[activePane_].locked.append(color);
    panes_[activePane_].custom.clear();
    panes_[activePane_].packed.clear();
    panes_[activePane_].packedWidth = 0;
    panes_[activePane_].packedHeight = 0;
    panes_[activePane_].mappedTransparency = false;
    refresh();
}

void SaveForWebDialog::onDeleteColor(int index)
{
    const QVector<QRgb> colors = colorTable_->colors();
    if (index < 0 || index >= colors.size()) {
        return;
    }
    const QImage image = sized();
    const QByteArray packed = quantizeWebImage(image, panes_[activePane_]);
    const QByteArray deleted = fromRust(
        web_palette_delete(bytesOf(packed), image.width(), image.height(), index));
    if (deleted.isEmpty()) {
        return;
    }
    panes_[activePane_].packed = deleted;
    panes_[activePane_].packedWidth = image.width();
    panes_[activePane_].packedHeight = image.height();
    panes_[activePane_].custom = paletteFromPacked(deleted);
    panes_[activePane_].locked.clear();
    refresh();
}

void SaveForWebDialog::onSortColors(int sort)
{
    const QImage image = sized();
    const QByteArray packed = quantizeWebImage(image, panes_[activePane_]);
    const QByteArray sorted = fromRust(
        web_palette_sort(bytesOf(packed), image.width(), image.height(), sort));
    if (sorted.isEmpty()) {
        return;
    }
    panes_[activePane_].packed = sorted;
    panes_[activePane_].packedWidth = image.width();
    panes_[activePane_].packedHeight = image.height();
    panes_[activePane_].custom = paletteFromPacked(sorted);
    panes_[activePane_].locked.clear();
    refresh();
}

void SaveForWebDialog::onTransparency(int index, bool transparent)
{
    QVector<QRgb> colors = colorTable_->colors();
    if (index < 0 || index >= colors.size()) {
        return;
    }
    for (int i = 0; i < colors.size(); ++i) {
        const QRgb color = colors[i];
        if (transparent) {
            colors[i] = qRgba(qRed(color), qGreen(color), qBlue(color),
                              i == index ? 0 : 255);
        } else if (i == index) {
            colors[i] = qRgba(qRed(color), qGreen(color), qBlue(color), 255);
        }
    }
    panes_[activePane_].mappedTransparency = transparent;
    onColorsEdited(colors);
}

void SaveForWebDialog::onLoadColorTable()
{
    const QString path = QFileDialog::getOpenFileName(
        this, QStringLiteral("Load Color Table"), defaultFileDialogDirectory(),
        QStringLiteral("Adobe Color Table (*.act)"));
    if (!path.isEmpty()) {
        loadColorTableForTest(path);
    }
}

void SaveForWebDialog::onSaveColorTable()
{
    const QString path = QFileDialog::getSaveFileName(
        this, QStringLiteral("Save Color Table"), defaultFileDialogDirectory(),
        QStringLiteral("Adobe Color Table (*.act)"));
    if (!path.isEmpty()) {
        saveColorTableForTest(path);
    }
}

QVector<SlicePlacement> SaveForWebDialog::buildSliceCells(
    const QVector<ExportSlice>& slices, const QSize& documentSize, const QSize& targetSize,
    const QString& base, const QString& suffix)
{
    QVector<SlicePlacement> cells;
    const double sx = documentSize.width() > 0
        ? double(targetSize.width()) / documentSize.width()
        : 1.0;
    const double sy = documentSize.height() > 0
        ? double(targetSize.height()) / documentSize.height()
        : 1.0;
    for (const ExportSlice& slice : slices) {
        const int left = qRound(slice.x * sx);
        const int top = qRound(slice.y * sy);
        const int right = qRound((slice.x + slice.width) * sx);
        const int bottom = qRound((slice.y + slice.height) * sy);
        if (right <= left || bottom <= top) {
            continue;
        }
        SlicePlacement cell;
        cell.name = QStringLiteral("%1_%2.%3")
                        .arg(base)
                        .arg(slice.number, 2, 10, QLatin1Char('0'))
                        .arg(suffix);
        cell.x = left;
        cell.y = top;
        cell.width = right - left;
        cell.height = bottom - top;
        cells.append(cell);
    }
    return cells;
}

bool SaveForWebDialog::writeSlices(const QString& path, int format, int layout,
                                   const QVector<SlicePlacement>& cells,
                                   const QString& title) const
{
    const QImage image = sized();
    const QDir dir = QFileInfo(path).absolutePath();
    const WebSettings& settings = panes_[activePane_];
    const bool wantImages = format != 2;
    const bool wantHtml = format != 1;
    if (wantHtml && output_path_protected(path)) {
        return false;
    }
    if (wantImages) {
        for (const SlicePlacement& cell : cells) {
            if (output_path_protected(dir.filePath(QStringLiteral("images/") + cell.name))) {
                return false;
            }
        }
    }
    const bool indexed = settings.format == Gif || settings.format == Png8;
    QByteArray packed;
    if (wantImages && indexed) {
        packed = quantizeWebImage(image, settings);
        if (packed.isEmpty()) {
            return false;
        }
    }
    if (wantImages && !QDir().mkpath(dir.filePath(QStringLiteral("images")))) {
        return false;
    }
    if (wantImages) {
        for (const SlicePlacement& cell : cells) {
            QByteArray data;
            if (indexed) {
                const QByteArray slice = cropPacked(
                    packed, image.size(), cell.x, cell.y, cell.width, cell.height);
                data = encodeIndexedWebImage(slice, QSize(cell.width, cell.height), settings);
            } else {
                const QImage crop = image.copy(cell.x, cell.y, cell.width, cell.height);
                data = encodeWebImage(crop, settings);
            }
            data = embedWebMetadata(data, webSuffix(settings), rawMetadata_, metadataLevel_);
            if (data.isEmpty()) {
                return false;
            }
            pictura::ProtectedOutput file(dir.filePath(QStringLiteral("images/") + cell.name));
            if (!file.open(QIODevice::WriteOnly) || file.write(data) != data.size()
                || !file.commit()) {
                return false;
            }
        }
    }
    if (wantHtml) {
        ::rust::Vec<WebSliceCell> rustCells;
        for (const SlicePlacement& cell : cells) {
            WebSliceCell rustCell;
            rustCell.name = ::rust::String(cell.name.toStdString());
            rustCell.x = cell.x;
            rustCell.y = cell.y;
            rustCell.width = cell.width;
            rustCell.height = cell.height;
            rustCells.push_back(std::move(rustCell));
        }
        const QString html = web_slice_html(std::move(rustCells), image.width(), image.height(),
                                            title, QStringLiteral("images"), layout);
        pictura::ProtectedOutput file(path);
        const QByteArray data = html.toUtf8();
        if (!file.open(QIODevice::WriteOnly) || file.write(data) != data.size()
            || !file.commit()) {
            return false;
        }
    }
    return true;
}

bool SaveForWebDialog::saveTo(const QString& path) const
{
    const QByteArray data = encoded();
    pictura::ProtectedOutput file(path);
    return !data.isEmpty() && file.open(QIODevice::WriteOnly)
        && file.write(data) == data.size() && file.commit();
}

WebSettings SaveForWebDialog::paneSettingsForTest(int pane) const
{
    return pane >= 0 && pane < 4 ? panes_[pane] : WebSettings{};
}

QVector<QRgb> SaveForWebDialog::colorTableForTest() const
{
    return colorTable_->colors();
}

void SaveForWebDialog::editSwatchForTest(int index, const QColor& color)
{
    colorTable_->selectIndex(index);
    colorTable_->editSelected(color);
}

void SaveForWebDialog::deleteSwatchForTest(int index)
{
    onDeleteColor(index);
}

void SaveForWebDialog::mapSwatchForTest(int index, bool transparent)
{
    onTransparency(index, transparent);
}

void SaveForWebDialog::sortSwatchesForTest(int sort)
{
    onSortColors(sort);
}

void SaveForWebDialog::addSwatchForTest(const QColor& color)
{
    onAddColor(color.rgba());
}

bool SaveForWebDialog::saveColorTableForTest(const QString& path) const
{
    const QVector<QRgb> colors = colorTable_->colors();
    if (colors.isEmpty() || colors.size() > 256) {
        return false;
    }
    QByteArray packed;
    packed.append(char((colors.size() >> 8) & 0xff));
    packed.append(char(colors.size() & 0xff));
    int clear = -1;
    for (int i = 0; i < colors.size(); ++i) {
        const QRgb color = colors[i];
        packed.append(char(qRed(color)));
        packed.append(char(qGreen(color)));
        packed.append(char(qBlue(color)));
        packed.append(char(qAlpha(color)));
        if (clear < 0 && qAlpha(color) == 0) {
            clear = i;
        }
    }
    packed.append(char(0));
    packed.append(char(0));
    if (clear >= 0) {
        const int value = clear + 1;
        packed[packed.size() - 2] = char((value >> 8) & 0xff);
        packed[packed.size() - 1] = char(value & 0xff);
    }
    const QByteArray data = fromRust(web_act_encode(bytesOf(packed)));
    if (data.isEmpty()) {
        return false;
    }
    pictura::ProtectedOutput file(path);
    return file.open(QIODevice::WriteOnly) && file.write(data) == data.size() && file.commit();
}

bool SaveForWebDialog::loadColorTableForTest(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return false;
    }
    const QByteArray data = file.readAll();
    const QByteArray packed = fromRust(web_act_parse(bytesOf(data)));
    if (packed.size() < 4) {
        return false;
    }
    panes_[activePane_].custom = paletteFromPacked(packed);
    panes_[activePane_].locked.clear();
    panes_[activePane_].packed.clear();
    panes_[activePane_].packedWidth = 0;
    panes_[activePane_].packedHeight = 0;
    panes_[activePane_].mappedTransparency = true;
    refresh();
    return true;
}

bool saveForWebFromView(QWidget* parent, PictureView* view, const QString& documentName,
                        QString* notice)
{
    if (!view || !view->has_document() || !pictura::compatibility_reason(*view).isEmpty()) {
        return false;
    }
    const int w = view->document_width();
    const int h = view->document_height();
    const ::rust::Vec<std::uint8_t> rgba = web_flattened(*view);
    if (rgba.size() != std::size_t(w) * std::size_t(h) * 4) {
        return false;
    }
    const QImage image = QImage(rgba.data(), w, h, w * 4, QImage::Format_RGBA8888).copy();
    WebMetadata raw = web_metadata(*view);
    SaveForWebDialog dialog(image, std::move(raw), parent);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    const auto report = [&](bool saved) {
        if (saved && notice && dialog.historyLogOmitted()) {
            *notice = QObject::tr("History Log: the JPEG was exported without new history entries.");
        }
        return saved;
    };
    const QString documentBase = QFileInfo(documentName).completeBaseName();
    const ::rust::Vec<std::int32_t> flat = web_export_slices(*view);
    QVector<ExportSlice> slices;
    bool hasUser = false;
    for (int i = 0; i + 5 < flat.size(); i += 6) {
        slices.append({flat[i], flat[i + 1], flat[i + 2], flat[i + 3], flat[i + 4],
                       flat[i + 5]});
        hasUser = hasUser || flat[i + 5] >= 0;
    }
    if (!hasUser) {
        const QString suffix = dialog.suffix();
        QString filter;
        const QString path = getSaveFileName(
            parent, QStringLiteral("Save Optimized As"),
            defaultFileDialogDirectory() + QLatin1Char('/') + documentBase + QLatin1Char('.')
                + suffix,
            {QStringLiteral("Images (*.%1)").arg(suffix)}, &filter);
        return report(!path.isEmpty() && dialog.saveTo(path));
    }
    SaveOptimizedDialog saveDialog(
        defaultFileDialogDirectory() + QLatin1Char('/') + documentBase + QStringLiteral(".html"),
        parent);
    if (saveDialog.exec() != QDialog::Accepted) {
        return false;
    }
    const QString base = QFileInfo(saveDialog.path()).completeBaseName();
    const QVector<SlicePlacement> cells = SaveForWebDialog::buildSliceCells(
        slices, QSize(w, h), dialog.sized().size(), base, dialog.suffix());
    return report(dialog.writeSlices(saveDialog.path(), saveDialog.format(), saveDialog.layout(),
                                     cells, documentBase));
}

} // namespace pictura

#include "moc_save_for_web_dialog.cpp"
