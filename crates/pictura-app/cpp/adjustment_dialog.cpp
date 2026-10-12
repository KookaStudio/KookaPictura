#include "adjustment_dialog.h"

#include "api/document_api.h"
#include "dialogs.h"
#include "image_view.h"

#include "panels/adjustment_controls.h"
#include "panels/color_swatch_widget.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust.cxxqt.h"

#include "curves_dialog.h"
#include "exposure_dialog.h"
#include "hue_saturation_dialog.h"
#include "levels_dialog.h"
#include "slider_dialog.h"
#include "black_white_dialog.h"
#include "channel_mixer_dialog.h"
#include "color_balance_dialog.h"
#include "gradient_map_dialog.h"
#include "photo_filter_dialog.h"
#include "panels/ramp_slider.h"

#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <memory>

namespace pictura {

namespace {

::rust::Slice<const std::uint8_t> slice(const QByteArray& bytes)
{
    return {reinterpret_cast<const std::uint8_t*>(bytes.constData()), std::size_t(bytes.size())};
}

QByteArray bytes(const ::rust::Vec<std::uint8_t>& v)
{
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

} // namespace

AdjustmentDialog::AdjustmentDialog(PictureView* view, const QByteArray& block,
                                   const QRect& visible, QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , block_(block)
    , visible_(visible)
{
    setObjectName(QStringLiteral("adjustmentDialog"));
    // The same frame as the hand-laid-out pages: controls with Preview under
    // them on the left, OK / Cancel down the right.
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);
    auto* left = new QVBoxLayout;
    controls_ = new AdjustmentControls(this);
    controls_->setCurveSource(
        [this](int channel) { return QString(image_adjustment_curve(slice(block_), channel)); });
    controls_->setPage(image_adjustment_page(slice(block_)));
    title_ = controls_->title();
    setWindowTitle(title_);
    left->addWidget(controls_);
    left->addStretch(1);
    left->addWidget(previewCheck(), 0, Qt::AlignRight);
    outer->addLayout(left, 1);
    QVBoxLayout* buttons = buttonColumn();
    buttons->addStretch(1);
    outer->addLayout(buttons);
    setMinimumWidth(420);

    connect(controls_, &AdjustmentControls::valueChanged, this,
            [this](const QString& key, double value) {
                edited(bytes(image_adjustment_set(slice(block_), key, value)));
            });
    connect(controls_, &AdjustmentControls::curveChanged, this,
            [this](int channel, const QString& points) {
                edited(bytes(image_adjustment_set_curve(slice(block_), channel, points)));
            });
    preview();
}

AdjustmentDialog::AdjustmentDialog(PictureView* view, const QByteArray& block,
                                   const QRect& visible, const QString& title, QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , block_(block)
    , visible_(visible)
    , title_(title)
{
    setObjectName(QStringLiteral("adjustmentDialog"));
    setWindowTitle(title_);
    takeHistograms();
    // Show the opening block at once, as the generic page does: Black & White
    // and Photo Filter are not the identity at their defaults.
    preview();
}

void AdjustmentDialog::takeHistograms()
{
    if (!view_) {
        return;
    }
    // The largest view-pyramid level at most 1024 px a side: enough samples
    // for a 256-bin shape, cheap on a large document.
    QImage image;
    const QVector<document_api::CanvasSize> levels = document_api::pyramidSizes(*view_);
    for (int level = 0; level < levels.size(); ++level) {
        const document_api::CanvasSize& size = levels.at(level);
        if (size.width <= 1024 && size.height <= 1024) {
            image = view_->display_image(level, 0, 0, size.width, size.height);
            break;
        }
    }
    image = image.convertToFormat(QImage::Format_ARGB32);
    for (int y = 0; y < image.height(); ++y) {
        const auto* line = reinterpret_cast<const QRgb*>(image.constScanLine(y));
        for (int x = 0; x < image.width(); ++x) {
            if (qAlpha(line[x]) == 0) {
                continue;
            }
            const QRgb pixel = qUnpremultiply(line[x]);
            ++histograms_[0][qGray(pixel)];
            ++histograms_[1][qRed(pixel)];
            ++histograms_[2][qGreen(pixel)];
            ++histograms_[3][qBlue(pixel)];
        }
    }
}

const std::array<int, 256>& AdjustmentDialog::histogram(int channel) const
{
    return histograms_[std::clamp(channel, 0, 3)];
}

QStringList AdjustmentDialog::page() const
{
    return image_adjustment_page(slice(block_));
}

double AdjustmentDialog::param(const QString& key, double fallback) const
{
    for (const QString& row : page()) {
        const QStringList cells = row.split(QLatin1Char('\t'));
        if (cells.size() > 4 && cells[1] == key) {
            return cells[4].toDouble();
        }
    }
    return fallback;
}

bool AdjustmentDialog::setParam(const QString& key, double value)
{
    return setParams({{key, value}});
}

bool AdjustmentDialog::setParams(const QList<QPair<QString, double>>& edits)
{
    QByteArray next = block_;
    for (const auto& [key, value] : edits) {
        next = bytes(image_adjustment_set(slice(next), key, value));
        if (next.isEmpty()) {
            return false;
        }
    }
    edited(next);
    return true;
}

QString AdjustmentDialog::curve(int channel) const
{
    return QString(image_adjustment_curve(slice(block_), channel));
}

bool AdjustmentDialog::setCurve(int channel, const QString& points)
{
    const QByteArray next = bytes(image_adjustment_set_curve(slice(block_), channel, points));
    if (next.isEmpty()) {
        return false;
    }
    edited(next);
    return true;
}

bool AdjustmentDialog::setBlock(const QByteArray& next)
{
    if (next.isEmpty()) {
        return false;
    }
    edited(next);
    return true;
}

QCheckBox* AdjustmentDialog::previewCheck()
{
    if (!preview_) {
        preview_ = new QCheckBox(QStringLiteral("Preview"), this);
        preview_->setObjectName(QStringLiteral("adjustmentPreview"));
        preview_->setChecked(true);
        connect(preview_, &QCheckBox::toggled, this, [this](bool on) {
            if (on) {
                preview();
            } else {
                cancelPreview();
            }
        });
    }
    return preview_;
}

QVBoxLayout* AdjustmentDialog::buttonColumn(const QStringList& extra)
{
    auto* column = new QVBoxLayout;
    column->setSpacing(6);
    const auto add = [this, column](const QString& text) {
        auto* b = new QPushButton(text, this);
        b->setObjectName(QStringLiteral("adjustmentButton")
                         + QString(text).remove(QLatin1Char('.')).remove(QChar(0x2026)));
        b->setMinimumWidth(84);
        b->setAutoDefault(false);
        column->addWidget(b);
        return b;
    };
    QPushButton* ok = add(QStringLiteral("OK"));
    ok->setDefault(true);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(add(QStringLiteral("Cancel")), &QPushButton::clicked, this, &QDialog::reject);
    for (const QString& text : extra) {
        add(text);
    }
    return column;
}

QSpinBox* AdjustmentDialog::addStackedRow(QGridLayout* grid, int row, const QString& label,
                                          const QString& key, int min, int max, int indent,
                                          const QList<QColor>& ramp, const QString& suffix)
{
    auto* text = new QLabel(label, this);
    text->setIndent(indent);
    auto* spin = new QSpinBox(this);
    spin->setObjectName(key);
    spin->setRange(min, max);
    spin->setFixedWidth(kFieldWidth);
    spin->setSuffix(suffix);
    spin->setValue(qRound(param(paramKey(key))));
    auto* slider = new RampSlider(this);
    slider->setRamp(ramp);
    slider->setObjectName(key + QStringLiteral("Slider"));
    slider->setRange(min, max);
    slider->setValue(spin->value());
    grid->addWidget(text, row, 0);
    grid->addWidget(spin, row, 2);
    grid->addWidget(slider, row + 1, 0, 1, 3);
    connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
    connect(spin, &QSpinBox::valueChanged, slider, &QSlider::setValue);
    connect(spin, &QSpinBox::valueChanged, this,
            [this, key](int value) { setParam(paramKey(key), value); });
    return spin;
}

QSpinBox* AdjustmentDialog::addRampRow(QVBoxLayout* layout, const QString& label,
                                       const QString& key, int min, int max,
                                       const QList<QColor>& ramp, const QString& suffix)
{
    auto* head = new QHBoxLayout;
    head->addWidget(new QLabel(label, this));
    head->addStretch(1);
    auto* spin = new QSpinBox(this);
    spin->setObjectName(key);
    spin->setRange(min, max);
    spin->setSuffix(suffix);
    spin->setFixedWidth(kFieldWidth);
    spin->setValue(qRound(param(paramKey(key))));
    head->addWidget(spin);
    layout->addLayout(head);
    auto* slider = new RampSlider(this);
    slider->setObjectName(key + QStringLiteral("Slider"));
    slider->setRange(min, max);
    slider->setValue(spin->value());
    slider->setRamp(ramp);
    layout->addWidget(slider);
    connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
    connect(spin, &QSpinBox::valueChanged, slider, &QSlider::setValue);
    connect(spin, &QSpinBox::valueChanged, this,
            [this, key](int value) { setParam(paramKey(key), value); });
    return spin;
}

ColorSwatchWidget* AdjustmentDialog::swatch(const QString& name)
{
    auto* button = new ColorSwatchWidget(this);
    button->setObjectName(name);
    button->setFixedSize(40, 24);
    return button;
}

void AdjustmentDialog::paintSwatch(ColorSwatchWidget* button, const QColor& color)
{
    button->setColor(color);
}

QHBoxLayout* AdjustmentDialog::bottomButtons()
{
    auto* row = new QHBoxLayout;
    row->addStretch(1);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    for (QPushButton* b : {cancel, ok}) {
        b->setObjectName(QStringLiteral("adjustmentButton") + b->text());
        b->setMinimumWidth(84);
        b->setAutoDefault(false);
        row->addWidget(b);
    }
    ok->setDefault(true);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    return row;
}

QPushButton* AdjustmentDialog::button(const QString& text) const
{
    for (QPushButton* b : findChildren<QPushButton*>()) {
        if (b->text() == text) {
            return b;
        }
    }
    return nullptr;
}

AdjustmentDialog::~AdjustmentDialog()
{
    cancelPreview();
}

void AdjustmentDialog::edited(const QByteArray& next)
{
    // A refused edit (Levels' input black past its white) keeps the last
    // good block; reload the controls to match it.
    if (next.isEmpty()) {
        if (controls_) {
            controls_->setPage(image_adjustment_page(slice(block_)));
        }
        return;
    }
    block_ = next;
    if (!preview_ || preview_->isChecked()) {
        preview();
    }
}

void AdjustmentDialog::preview()
{
    if (view_) {
        previewing_ = image_adjust_preview(*view_, slice(block_), visible_.x(), visible_.y(),
                                           visible_.width(), visible_.height())
            || previewing_;
    }
}

void AdjustmentDialog::cancelPreview()
{
    if (view_ && previewing_) {
        filter_preview_cancel(*view_);
    }
    previewing_ = false;
}

void AdjustmentDialog::setPreviewArea(const QRect& visible)
{
    if (visible == visible_) {
        return;
    }
    visible_ = visible;
    if (!preview_ || preview_->isChecked()) {
        preview();
    }
}

void AdjustmentDialog::accept()
{
    // The commit re-applies to the whole layer from the pre-preview pixels.
    if (view_ && image_adjust_apply(*view_, slice(block_), title_)) {
        previewing_ = false;
    } else {
        cancelPreview();
    }
    QDialog::accept();
}

void AdjustmentDialog::reject()
{
    cancelPreview();
    QDialog::reject();
}

QWidget* AdjustmentDialog::controlForTest(const QString& key) const
{
    return controls_ ? controls_->controlForTest(key) : nullptr;
}

std::unique_ptr<AdjustmentDialog> makeAdjustmentDialog(const QString& kind, PictureView* view,
                                                       const QByteArray& block,
                                                       const QRect& visible, QWidget* parent)
{
    std::unique_ptr<AdjustmentDialog> dialog;
    if (kind == QLatin1String("brightness-contrast")) {
        dialog = std::make_unique<SliderDialog>(
            view, block, visible, QStringLiteral("Brightness/Contrast"),
            QVector<SliderDialog::Row>{
                {QStringLiteral("Brightness:"), QStringLiteral("brightness"), -150, 150},
                {QStringLiteral("Contrast:"), QStringLiteral("contrast"), -50, 100}},
            QStringList{QStringLiteral("Auto")}, parent);
    } else if (kind == QLatin1String("vibrance")) {
        dialog = std::make_unique<SliderDialog>(
            view, block, visible, QStringLiteral("Vibrance"),
            QVector<SliderDialog::Row>{
                {QStringLiteral("Vibrance:"), QStringLiteral("vibrance"), -100, 100},
                {QStringLiteral("Saturation:"), QStringLiteral("saturation"), -100, 100}},
            QStringList{}, parent);
    } else if (kind == QLatin1String("exposure")) {
        dialog = std::make_unique<ExposureDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("hue-saturation")) {
        dialog = std::make_unique<HueSaturationDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("color-balance")) {
        dialog = std::make_unique<ColorBalanceDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("black-white")) {
        dialog = std::make_unique<BlackWhiteDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("gradient-map")) {
        dialog = std::make_unique<GradientMapDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("photo-filter")) {
        dialog = std::make_unique<PhotoFilterDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("channel-mixer")) {
        dialog = std::make_unique<ChannelMixerDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("levels")) {
        dialog = std::make_unique<LevelsDialog>(view, block, visible, parent);
    } else if (kind == QLatin1String("curves")) {
        dialog = std::make_unique<CurvesDialog>(view, block, visible, parent);
    } else {
        dialog = std::make_unique<AdjustmentDialog>(view, block, visible, parent);
    }
    return dialog;
}

bool runAdjustmentDialog(QWidget* parent, PictureView* view, const QString& kind,
                         const QColor& foreground, const QColor& background, ImageView* canvas)
{
    if (!view) {
        return false;
    }
    const QByteArray block = bytes(image_adjustment_default(
        kind, foreground.rgb() & 0xffffffu, background.rgb() & 0xffffffu));
    if (block.isEmpty()) {
        return false;
    }
    const QRect visible = canvas ? canvas->visibleDocumentRect().toAlignedRect() : QRect();
    const auto dialog = makeAdjustmentDialog(kind, view, block, visible, parent);
    if (canvas) {
        // A drag reports every step; re-preview once the view settles.
        auto* settle = new QTimer(dialog.get());
        settle->setSingleShot(true);
        settle->setInterval(120);
        QObject::connect(canvas, &ImageView::viewChanged, settle, qOverload<>(&QTimer::start));
        QObject::connect(settle, &QTimer::timeout, dialog.get(), [canvas, d = dialog.get()]() {
            d->setPreviewArea(canvas->visibleDocumentRect().toAlignedRect());
        });
    }
    return runDialog(*dialog, parent) == QDialog::Accepted;
}

} // namespace pictura
