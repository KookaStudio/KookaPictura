#include "gradient_noise_page.h"

#include "gradient_editor_dialog.h"
#include "panels/percent_field.h"

#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int kHandle = 9;

// Each model's component labels and the ramp under each range.
struct Component {
    const char* label;
    QList<QColor> ramp;
};

QList<Component> components(int model)
{
    switch (model) {
    case NoiseSettings::Hsb: {
        QList<QColor> hues;
        for (int h = 0; h <= 360; h += 60) {
            hues << QColor::fromHsv(h % 360, 255, 255);
        }
        return {{"H:", hues},
                {"S:", {Qt::white, QColor(255, 0, 0)}},
                {"B:", {Qt::black, Qt::white}}};
    }
    case NoiseSettings::Lab:
        return {{"L:", {Qt::black, Qt::white}},
                {"a:", {QColor(0, 160, 120), QColor(220, 0, 140)}},
                {"b:", {QColor(0, 80, 220), QColor(240, 210, 0)}}};
    default:
        return {{"R:", {Qt::black, QColor(255, 0, 0)}},
                {"G:", {Qt::black, QColor(0, 255, 0)}},
                {"B:", {Qt::black, QColor(0, 0, 255)}}};
    }
}

} // namespace

// --- ChannelRangeSlider -----------------------------------------------------

ChannelRangeSlider::ChannelRangeSlider(QWidget* parent)
    : QWidget(parent)
{
    setMinimumHeight(10 + kHandle + 4);
}

QSize ChannelRangeSlider::sizeHint() const
{
    return {200, 10 + kHandle + 4};
}

void ChannelRangeSlider::setRange(int low, int high)
{
    low_ = qBound(0, low, 100);
    high_ = qBound(low_, high, 100);
    update();
}

void ChannelRangeSlider::setRamp(const QList<QColor>& colors)
{
    ramp_ = colors;
    update();
}

QRect ChannelRangeSlider::grooveRect() const
{
    return QRect(kHandle / 2 + 1, 1, width() - kHandle - 2, 8);
}

int ChannelRangeSlider::handleX(int value) const
{
    const QRect g = grooveRect();
    return g.left() + qRound(value * (g.width() - 1) / 100.0);
}

int ChannelRangeSlider::valueAt(int x) const
{
    const QRect g = grooveRect();
    return qBound(0, qRound((x - g.left()) * 100.0 / qMax(1, g.width() - 1)), 100);
}

void ChannelRangeSlider::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    const QRect g = grooveRect();
    QLinearGradient ramp(g.left(), 0, g.right() + 1, 0);
    for (int i = 0; i < ramp_.size(); ++i) {
        ramp.setColorAt(ramp_.size() > 1 ? double(i) / (ramp_.size() - 1) : 0.0, ramp_.at(i));
    }
    painter.fillRect(g, ramp);
    painter.setPen(palette().color(QPalette::Dark));
    painter.drawRect(g.adjusted(0, 0, -1, -1));
    painter.setRenderHint(QPainter::Antialiasing);
    const auto handle = [&painter, g, this](int value, const QColor& fill) {
        const double x = handleX(value) + 0.5;
        const double top = g.bottom() + 1.5;
        QPainterPath path;
        path.moveTo(x, top);
        path.lineTo(x + kHandle / 2.0, top + kHandle);
        path.lineTo(x - kHandle / 2.0, top + kHandle);
        path.closeSubpath();
        painter.setPen(QColor(60, 60, 60));
        painter.setBrush(fill);
        painter.drawPath(path);
    };
    handle(low_, Qt::black);
    handle(high_, Qt::white);
}

void ChannelRangeSlider::mousePressEvent(QMouseEvent* event)
{
    const int value = valueAt(qRound(event->position().x()));
    // The nearer handle; on a tie, the one that can move that way.
    const int toLow = std::abs(value - low_);
    const int toHigh = std::abs(value - high_);
    dragging_ = toLow < toHigh || (toLow == toHigh && value < low_) ? 0 : 1;
    mouseMoveEvent(event);
}

void ChannelRangeSlider::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_ < 0) {
        return;
    }
    const int value = valueAt(qRound(event->position().x()));
    if (dragging_ == 0) {
        low_ = qMin(value, high_);
    } else {
        high_ = qMax(value, low_);
    }
    update();
    emit rangeChanged(low_, high_);
}

// --- GradientNoisePage ------------------------------------------------------

GradientNoisePage::GradientNoisePage(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("gradientNoisePage"));
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);
    roughness_ = new PercentField(QStringLiteral("Roughness:"), this);
    roughness_->setObjectName(QStringLiteral("gradientRoughness"));
    auto* roughRow = new QHBoxLayout;
    roughRow->addWidget(roughness_);
    roughRow->addStretch(1);
    column->addLayout(roughRow);
    strip_ = new GradientSwatch(this);
    strip_->setObjectName(QStringLiteral("gradientNoiseStrip"));
    column->addWidget(strip_);

    auto* lower = new QHBoxLayout;
    auto* modelBox = new QGroupBox(this);
    auto* modelGrid = new QGridLayout(modelBox);
    auto* modelRow = new QHBoxLayout;
    modelRow->addWidget(new QLabel(QStringLiteral("Color Model:"), modelBox));
    model_ = new QComboBox(modelBox);
    model_->setObjectName(QStringLiteral("gradientNoiseModel"));
    model_->addItems({QStringLiteral("RGB"), QStringLiteral("HSB"), QStringLiteral("LAB")});
    modelRow->addWidget(model_);
    modelRow->addStretch(1);
    modelGrid->addLayout(modelRow, 0, 0, 1, 2);
    for (int i = 0; i < 3; ++i) {
        labels_[i] = new QLabel(modelBox);
        ranges_[i] = new ChannelRangeSlider(modelBox);
        ranges_[i]->setObjectName(QStringLiteral("gradientNoiseRange%1").arg(i));
        modelGrid->addWidget(labels_[i], i + 1, 0);
        modelGrid->addWidget(ranges_[i], i + 1, 1);
        connect(ranges_[i], &ChannelRangeSlider::rangeChanged, this, [this, i](int low, int high) {
            settings_.low[i] = low;
            settings_.high[i] = high;
            edited();
        });
    }
    modelGrid->setColumnStretch(1, 1);
    lower->addWidget(modelBox, 1);

    auto* optionsBox = new QGroupBox(QStringLiteral("Options:"), this);
    auto* options = new QVBoxLayout(optionsBox);
    restrict_ = new QCheckBox(QStringLiteral("Restrict Colors"), optionsBox);
    restrict_->setObjectName(QStringLiteral("gradientNoiseRestrict"));
    transparency_ = new QCheckBox(QStringLiteral("Add Transparency"), optionsBox);
    transparency_->setObjectName(QStringLiteral("gradientNoiseTransparency"));
    auto* randomize = new QPushButton(QStringLiteral("Randomize"), optionsBox);
    randomize->setObjectName(QStringLiteral("gradientNoiseRandomize"));
    randomize->setAutoDefault(false);
    options->addWidget(restrict_);
    options->addWidget(transparency_);
    options->addWidget(randomize, 0, Qt::AlignLeft);
    options->addStretch(1);
    lower->addWidget(optionsBox);
    column->addLayout(lower);
    column->addStretch(1);

    connect(roughness_, &PercentField::valueChanged, this, [this](double v) {
        settings_.roughness = qRound(v);
        edited();
    });
    connect(model_, &QComboBox::currentIndexChanged, this, [this](int model) {
        settings_.model = model;
        showSettings();
        edited();
    });
    connect(restrict_, &QCheckBox::toggled, this, [this](bool on) {
        settings_.restrictColors = on;
        edited();
    });
    connect(transparency_, &QCheckBox::toggled, this, [this](bool on) {
        settings_.transparency = on;
        edited();
    });
    connect(randomize, &QPushButton::clicked, this, &GradientNoisePage::randomize);
    showSettings();
}

void GradientNoisePage::setSettings(const NoiseSettings& settings)
{
    settings_ = settings;
    showSettings();
}

void GradientNoisePage::randomize()
{
    // A linear congruential step: each press a new gradient, the same
    // sequence every run.
    settings_.seed = settings_.seed * 1664525u + 1013904223u;
    edited();
}

void GradientNoisePage::edited()
{
    if (showing_) {
        return;
    }
    strip_->setStops(noiseStops(settings_));
    emit changed();
}

void GradientNoisePage::showSettings()
{
    showing_ = true;
    roughness_->setValue(settings_.roughness);
    model_->setCurrentIndex(settings_.model);
    const QList<Component> parts = components(settings_.model);
    for (int i = 0; i < 3; ++i) {
        labels_[i]->setText(QLatin1String(parts.at(i).label));
        ranges_[i]->setRamp(parts.at(i).ramp);
        ranges_[i]->setRange(settings_.low[i], settings_.high[i]);
    }
    restrict_->setChecked(settings_.restrictColors);
    transparency_->setChecked(settings_.transparency);
    showing_ = false;
    strip_->setStops(noiseStops(settings_));
}

} // namespace pictura

#include "moc_gradient_noise_page.cpp"
