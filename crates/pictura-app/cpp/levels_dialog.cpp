#include "levels_dialog.h"

#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

constexpr int kStripWidth = 256 + 2 * LevelsStrip::kInset;
constexpr int kThumbHeight = 10;

struct Preset {
    const char* name;
    int inBlack;
    int inWhite;
    double gamma;
    int outBlack;
    int outWhite;
};

// photorust's preset table; "Custom" marks a hand edit.
constexpr Preset kPresets[] = {
    {"Default", 0, 255, 1.00, 0, 255},
    {"Darker", 0, 255, 0.75, 0, 255},
    {"Increase Contrast 1", 5, 250, 1.00, 0, 255},
    {"Increase Contrast 2", 10, 245, 1.00, 0, 255},
    {"Increase Contrast 3", 15, 240, 1.00, 0, 255},
    {"Lighten Shadows", 0, 255, 1.50, 0, 255},
    {"Lighter", 0, 255, 1.50, 0, 255},
    {"Midtones Brighter", 0, 255, 1.25, 0, 255},
    {"Midtones Darker", 0, 255, 0.75, 0, 255},
};
const QString kCustom = QStringLiteral("Custom");

const QStringList kChannelPrefixes = {QString(), QStringLiteral("red."),
                                      QStringLiteral("green."), QStringLiteral("blue.")};

// A level field over the engine layout's range for `name`; the input pair's
// limits are then coupled to each other at run time (`loadChannel`).
QSpinBox* levelSpin(QWidget* parent, const QString& name, const AdjustmentDialog::ParamSpec& range)
{
    auto* spin = new QSpinBox(parent);
    spin->setObjectName(name);
    spin->setRange(qRound(range.min), qRound(range.max));
    spin->setFixedWidth(AdjustmentDialog::kFieldWidth);
    spin->setKeyboardTracking(false);
    return spin;
}

} // namespace

LevelsStrip::LevelsStrip(bool ramp, int height, QWidget* parent)
    : QWidget(parent)
    , ramp_(ramp)
{
    setFixedSize(kStripWidth, height);
}

void LevelsStrip::setBins(const std::array<int, 256>& bins)
{
    bins_ = bins;
    update();
}

void LevelsStrip::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    const QRect face(kInset, 0, 256, height());
    if (ramp_) {
        QLinearGradient ramp(face.topLeft(), face.topRight());
        ramp.setColorAt(0.0, Qt::black);
        ramp.setColorAt(1.0, Qt::white);
        p.fillRect(face, ramp);
    } else {
        p.fillRect(face, Qt::white);
        const int peak = std::max(1, *std::max_element(bins_.begin(), bins_.end()));
        p.setPen(Qt::black);
        for (int i = 0; i < 256; ++i) {
            const int bar = int(double(bins_[i]) / peak * (face.height() - 1));
            if (bar > 0) {
                p.drawLine(face.left() + i, face.bottom(), face.left() + i, face.bottom() - bar + 1);
            }
        }
    }
    p.setPen(QColor(150, 150, 150));
    p.drawRect(face.adjusted(0, 0, -1, -1));
}

TriangleSlider::TriangleSlider(int count, QWidget* parent)
    : QWidget(parent)
    , thumbs_(count)
{
    setFixedSize(kStripWidth, kThumbHeight + 2);
}

void TriangleSlider::setRange(int index, int min, int max)
{
    Thumb& thumb = thumbs_[index];
    thumb.min = min;
    thumb.max = std::max(min, max);
    thumb.value = std::clamp(thumb.value, thumb.min, thumb.max);
    update();
}

void TriangleSlider::setValue(int index, int value)
{
    Thumb& thumb = thumbs_[index];
    thumb.value = std::clamp(value, thumb.min, thumb.max);
    update();
}

void TriangleSlider::setColor(int index, const QColor& color)
{
    thumbs_[index].color = color;
    update();
}

int TriangleSlider::value(int index) const { return thumbs_[index].value; }

int TriangleSlider::xForValue(int value) const { return LevelsStrip::kInset + value; }

int TriangleSlider::valueForX(int index, int x) const
{
    return std::clamp(x - LevelsStrip::kInset, thumbs_[index].min, thumbs_[index].max);
}

void TriangleSlider::paintEvent(QPaintEvent*)
{
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    for (const Thumb& thumb : thumbs_) {
        const qreal x = xForValue(thumb.value) + 0.5;
        QPainterPath path;
        path.moveTo(x, 1);
        path.lineTo(x - 5, kThumbHeight + 1);
        path.lineTo(x + 5, kThumbHeight + 1);
        path.closeSubpath();
        p.setBrush(thumb.color);
        p.setPen(QPen(QColor(90, 90, 90), 1));
        p.drawPath(path);
    }
}

void TriangleSlider::mousePressEvent(QMouseEvent* event)
{
    dragging_ = -1;
    int best = 12;
    // Ties go to the last thumb, so white can leave a black parked under it.
    for (int i = 0; i < thumbs_.size(); ++i) {
        const int distance = std::abs(int(event->position().x()) - xForValue(thumbs_[i].value));
        if (distance <= best) {
            best = distance;
            dragging_ = i;
        }
    }
    mouseMoveEvent(event);
}

void TriangleSlider::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_ < 0) {
        return;
    }
    const int value = valueForX(dragging_, int(event->position().x()));
    if (value != thumbs_[dragging_].value) {
        thumbs_[dragging_].value = value;
        update();
        emit valueChanged(dragging_, value);
    }
}

void TriangleSlider::mouseReleaseEvent(QMouseEvent*) { dragging_ = -1; }

LevelsDialog::LevelsDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                           QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Levels"), parent)
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(14);
    auto* left = new QVBoxLayout;
    left->setSpacing(4);

    auto* presetRow = new QHBoxLayout;
    presetRow->addWidget(new QLabel(QStringLiteral("Preset:"), this));
    preset_ = new QComboBox(this);
    preset_->setObjectName(QStringLiteral("levelsPreset"));
    for (const Preset& preset : kPresets) {
        preset_->addItem(QLatin1String(preset.name));
    }
    preset_->insertSeparator(1);
    preset_->addItem(kCustom);
    presetRow->addWidget(preset_, 1);
    left->addLayout(presetRow);

    auto* channelRow = new QHBoxLayout;
    channelRow->addSpacing(14);
    channelRow->addWidget(new QLabel(QStringLiteral("Channel:"), this));
    channel_ = new QComboBox(this);
    channel_->setObjectName(QStringLiteral("levelsChannel"));
    channel_->addItems({QStringLiteral("RGB"), QStringLiteral("Red"), QStringLiteral("Green"),
                        QStringLiteral("Blue")});
    channelRow->addWidget(channel_, 1);
    left->addLayout(channelRow);
    left->addSpacing(6);

    left->addWidget(new QLabel(QStringLiteral("Input Levels:"), this));
    histogram_ = new LevelsStrip(false, 120, this);
    histogram_->setObjectName(QStringLiteral("levelsHistogram"));
    left->addWidget(histogram_);
    input_ = new TriangleSlider(3, this);
    input_->setObjectName(QStringLiteral("levelsInputSlider"));
    input_->setColor(0, Qt::black);
    input_->setColor(1, QColor(128, 128, 128));
    input_->setColor(2, Qt::white);
    left->addWidget(input_);
    auto* inputRow = new QHBoxLayout;
    const auto levelField = [this](const QString& name) { return levelSpin(this, name, spec(name)); };
    inBlack_ = levelField(QStringLiteral("inputBlack"));
    gamma_ = new QDoubleSpinBox(this);
    gamma_->setObjectName(QStringLiteral("gamma"));
    const ParamSpec gamma = spec(QStringLiteral("gamma"));
    gamma_->setRange(gamma.min, gamma.max);
    gamma_->setDecimals(gamma.decimals);
    gamma_->setSingleStep(0.01);
    gamma_->setFixedWidth(kFieldWidth);
    gamma_->setKeyboardTracking(false);
    inWhite_ = levelField(QStringLiteral("inputWhite"));
    inputRow->addWidget(inBlack_);
    inputRow->addStretch(1);
    inputRow->addWidget(gamma_);
    inputRow->addStretch(1);
    inputRow->addWidget(inWhite_);
    left->addLayout(inputRow);
    left->addSpacing(10);

    left->addWidget(new QLabel(QStringLiteral("Output Levels:"), this));
    left->addWidget(new LevelsStrip(true, 16, this));
    output_ = new TriangleSlider(2, this);
    output_->setObjectName(QStringLiteral("levelsOutputSlider"));
    output_->setColor(0, Qt::black);
    output_->setColor(1, Qt::white);
    left->addWidget(output_);
    auto* outputRow = new QHBoxLayout;
    outBlack_ = levelField(QStringLiteral("outputBlack"));
    outWhite_ = levelField(QStringLiteral("outputWhite"));
    outputRow->addWidget(outBlack_);
    outputRow->addStretch(1);
    outputRow->addWidget(outWhite_);
    left->addLayout(outputRow);
    left->addStretch(1);
    // The fields line up under the strips, so the column is the strip's width.
    auto* leftBox = new QWidget(this);
    leftBox->setLayout(left);
    left->setContentsMargins(0, 0, 0, 0);
    leftBox->setFixedWidth(kStripWidth + 12);
    outer->addWidget(leftBox);

    QVBoxLayout* buttons =
        buttonColumn({QStringLiteral("Auto"), QStringLiteral("Options…")});
    button(QStringLiteral("Options…"))->setEnabled(false);
    buttons->addSpacing(10);
    buttons->addWidget(previewCheck());
    buttons->addStretch(1);
    outer->addLayout(buttons);

    connect(inBlack_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("inputBlack"), v); });
    connect(inWhite_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("inputWhite"), v); });
    connect(gamma_, &QDoubleSpinBox::valueChanged, this,
            [this](double v) { edit(QStringLiteral("gamma"), v); });
    connect(outBlack_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("outputBlack"), v); });
    connect(outWhite_, &QSpinBox::valueChanged, this,
            [this](int v) { edit(QStringLiteral("outputWhite"), v); });
    connect(input_, &TriangleSlider::valueChanged, this, [this](int index, int value) {
        if (index == 0) {
            inBlack_->setValue(value);
        } else if (index == 2) {
            inWhite_->setValue(value);
        } else {
            // The gray thumb sits where the curve crosses half output:
            // norm^(1/gamma) = 0.5, so gamma = log(norm) / log(0.5).
            const int lo = inBlack_->value();
            const int hi = inWhite_->value();
            const double norm = std::clamp(double(value - lo) / std::max(1, hi - lo), 0.001, 0.999);
            gamma_->setValue(std::log(norm) / std::log(0.5));
        }
    });
    connect(output_, &TriangleSlider::valueChanged, this, [this](int index, int value) {
        (index == 0 ? outBlack_ : outWhite_)->setValue(value);
    });
    connect(channel_, &QComboBox::currentIndexChanged, this, &LevelsDialog::loadChannel);
    connect(preset_, &QComboBox::activated, this, &LevelsDialog::applyPreset);
    connect(button(QStringLiteral("Auto")), &QPushButton::clicked, this,
            &LevelsDialog::autoLevels);

    loadChannel();
}

QString LevelsDialog::prefix() const { return kChannelPrefixes.value(channel_->currentIndex()); }

int LevelsDialog::gammaPosition(double gamma) const
{
    const int lo = inBlack_->value();
    const int hi = inWhite_->value();
    return lo + qRound(std::pow(0.5, gamma) * (hi - lo));
}

void LevelsDialog::loadChannel()
{
    loading_ = true;
    const QString key = prefix();
    const int ib = qRound(param(key + QStringLiteral("inputBlack"), 0));
    const int iw = qRound(param(key + QStringLiteral("inputWhite"), 255));
    inBlack_->setMaximum(iw - 2);
    inWhite_->setMinimum(ib + 2);
    inBlack_->setValue(ib);
    inWhite_->setValue(iw);
    gamma_->setValue(param(key + QStringLiteral("gamma"), 1.0));
    outBlack_->setValue(qRound(param(key + QStringLiteral("outputBlack"), 0)));
    outWhite_->setValue(qRound(param(key + QStringLiteral("outputWhite"), 255)));

    input_->setRange(0, 0, iw - 2);
    input_->setRange(1, ib + 1, iw - 1);
    input_->setRange(2, ib + 2, 255);
    input_->setValue(0, ib);
    input_->setValue(2, iw);
    input_->setValue(1, gammaPosition(gamma_->value()));
    output_->setRange(0, 0, 255);
    output_->setRange(1, 0, 255);
    output_->setValue(0, outBlack_->value());
    output_->setValue(1, outWhite_->value());
    histogram_->setBins(histogram(channel_->currentIndex()));
    loading_ = false;
}

void LevelsDialog::edit(const QString& name, double value)
{
    if (loading_) {
        return;
    }
    setParam(prefix() + name, value);
    markCustom();
    // Reload even after a good edit: the black / white limits and the gray
    // thumb follow every input change.
    loadChannel();
}

void LevelsDialog::applyPreset(int index)
{
    const QString name = preset_->itemText(index);
    for (const Preset& preset : kPresets) {
        if (name != QLatin1String(preset.name)) {
            continue;
        }
        // Every channel back to identity first, so each preset value lands
        // against a full 0..255 input range.
        QList<QPair<QString, double>> edits;
        for (const QString& key : kChannelPrefixes) {
            edits << qMakePair(key + QStringLiteral("inputBlack"), 0.0)
                  << qMakePair(key + QStringLiteral("inputWhite"), 255.0)
                  << qMakePair(key + QStringLiteral("gamma"), 1.0)
                  << qMakePair(key + QStringLiteral("outputBlack"), 0.0)
                  << qMakePair(key + QStringLiteral("outputWhite"), 255.0);
        }
        edits << qMakePair(QStringLiteral("inputBlack"), double(preset.inBlack))
              << qMakePair(QStringLiteral("inputWhite"), double(preset.inWhite))
              << qMakePair(QStringLiteral("gamma"), preset.gamma)
              << qMakePair(QStringLiteral("outputBlack"), double(preset.outBlack))
              << qMakePair(QStringLiteral("outputWhite"), double(preset.outWhite));
        setParams(edits);
        const QSignalBlocker block(channel_);
        channel_->setCurrentIndex(0);
        loadChannel();
        return;
    }
}

void LevelsDialog::autoLevels()
{
    const std::array<int, 256>& bins = histogram(channel_->currentIndex());
    long long total = 0;
    for (int count : bins) {
        total += count;
    }
    if (total == 0) {
        return;
    }
    // CS6's default clip: 0.1% of the pixels off each end.
    const long long clip = total / 1000;
    int lo = 0;
    for (long long seen = 0; lo < 255 && (seen += bins[lo]) <= clip;) {
        ++lo;
    }
    int hi = 255;
    for (long long seen = 0; hi > 0 && (seen += bins[hi]) <= clip;) {
        --hi;
    }
    if (hi - lo < 2) {
        return;
    }
    const QString key = prefix();
    setParams({{key + QStringLiteral("inputBlack"), 0.0},
               {key + QStringLiteral("inputWhite"), double(hi)},
               {key + QStringLiteral("inputBlack"), double(lo)}});
    markCustom();
    loadChannel();
}

void LevelsDialog::markCustom()
{
    const QSignalBlocker block(preset_);
    preset_->setCurrentIndex(preset_->findText(kCustom));
}

QWidget* LevelsDialog::controlForTest(const QString& key) const
{
    for (QWidget* widget : {static_cast<QWidget*>(preset_), static_cast<QWidget*>(channel_),
                            static_cast<QWidget*>(input_), static_cast<QWidget*>(output_),
                            static_cast<QWidget*>(inBlack_), static_cast<QWidget*>(gamma_),
                            static_cast<QWidget*>(inWhite_), static_cast<QWidget*>(outBlack_),
                            static_cast<QWidget*>(outWhite_)}) {
        if (widget->objectName() == key) {
            return widget;
        }
    }
    return nullptr;
}

} // namespace pictura

#include "moc_levels_dialog.cpp"
