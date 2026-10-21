#include "brush_panel.h"

#include "numeric_field.h"

#include "paint_tip.h"
#include "tools.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QRadialGradient>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <array>
#include <cstdint>

namespace pictura {

// The tip-shape indicator: the tip's ellipse at its angle and roundness.
class TipShapeView : public QWidget {
public:
    explicit TipShapeView(QWidget* parent)
        : QWidget(parent)
    {
        setFixedSize(72, 72);
    }

    void setShape(int angle, int roundness)
    {
        angle_ = angle;
        roundness_ = roundness;
        update();
    }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing, true);
        p.fillRect(rect(), palette().base());
        p.translate(width() / 2.0, height() / 2.0);
        const double r = width() / 2.0 - 6.0;
        p.setPen(QPen(palette().text().color(), 1.0));
        p.drawLine(QPointF(-r, 0), QPointF(r, 0));
        p.rotate(-angle_);
        p.setPen(QPen(palette().highlight().color(), 1.5));
        p.drawEllipse(QPointF(), r, r * roundness_ / 100.0);
        p.drawLine(QPointF(0, 0), QPointF(r, 0));
    }

private:
    int angle_ = 0;
    int roundness_ = 100;
};

namespace {

struct StandardTip {
    int size;
    int hardness;
};

// Hard and soft round tips in the spirit of CS6's default set.
constexpr std::array<StandardTip, 18> kStandardTips = {{
    {1, 100}, {3, 100}, {5, 100}, {9, 100}, {13, 100}, {19, 100},
    {5, 0}, {9, 0}, {13, 0}, {17, 0}, {21, 0}, {27, 0},
    {35, 0}, {45, 0}, {65, 0}, {100, 0}, {200, 0}, {300, 0},
}};

QIcon tipIcon(const StandardTip& tip)
{
    QPixmap pm(24, 24);
    pm.fill(Qt::transparent);
    QPainter p(&pm);
    p.setRenderHint(QPainter::Antialiasing, true);
    const double r = 2.0 + 6.0 * std::min(tip.size, 60) / 60.0;
    QRadialGradient g(QPointF(12, 9), r);
    g.setColorAt(0.0, Qt::white);
    g.setColorAt(tip.hardness / 100.0 * 0.95, Qt::white);
    g.setColorAt(1.0, QColor(255, 255, 255, 0));
    p.setPen(Qt::NoPen);
    p.setBrush(g);
    p.drawEllipse(QPointF(12, 9), r, r);
    p.setPen(Qt::white);
    QFont font = p.font();
    font.setPixelSize(8);
    p.setFont(font);
    p.drawText(QRect(0, 17, 24, 7), Qt::AlignCenter, QString::number(tip.size));
    return QIcon(pm);
}

NumericFieldConfig field(double lo, double hi, const QString& suffix, const QString& name)
{
    NumericFieldConfig config;
    config.minimum = lo;
    config.maximum = hi;
    config.suffix = suffix;
    config.popup = true;
    config.objectName = name;
    return config;
}

// CS6's Brush panel option sets past Brush Tip Shape, in panel order.
constexpr const char* kDynamics[] = {
    "Shape Dynamics", "Scattering", "Texture",     "Dual Brush",  "Color Dynamics",
    "Transfer",       "Brush Pose", "Noise",       "Wet Edges",   "Build-up",
    "Smoothing",      "Protect Texture",
};

} // namespace

BrushPanel::BrushPanel(QWidget* parent)
    : QWidget(parent)
{
    // The content scrolls, so the panel's size never widens or lengthens the
    // column and groups it shares.
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);
    auto* scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    auto* content = new QWidget(scroll);
    scroll->setWidget(content);
    outer->addWidget(scroll);
    auto* layout = new QVBoxLayout(content);
    auto* presets = new QPushButton(tr("Brush Presets"), this);
    presets->setEnabled(false);
    presets->setToolTip(tr("Brush Presets: not implemented yet"));

    auto* body = new QHBoxLayout();
    auto* sets = new QListWidget(this);
    sets->setObjectName(QStringLiteral("brushOptionSets"));
    // Narrow enough that the docked panel never widens the column it shares.
    sets->setMaximumWidth(110);
    auto* tipShape = new QListWidgetItem(tr("Brush Tip Shape"), sets);
    sets->setCurrentItem(tipShape);
    for (const char* name : kDynamics) {
        auto* item = new QListWidgetItem(QString::fromLatin1(name), sets);
        item->setFlags(Qt::NoItemFlags);
        item->setCheckState(Qt::Unchecked);
        item->setToolTip(tr("%1: not implemented yet").arg(QString::fromLatin1(name)));
    }
    body->addWidget(sets);

    auto* page = new QVBoxLayout();
    auto* grid = new QGridLayout();
    grid->setSpacing(2);
    for (size_t i = 0; i < kStandardTips.size(); ++i) {
        const StandardTip tip = kStandardTips[i];
        auto* button = new QToolButton(this);
        button->setObjectName(QStringLiteral("brushTip%1").arg(i));
        button->setIcon(tipIcon(tip));
        button->setIconSize(QSize(24, 24));
        button->setAutoRaise(true);
        button->setToolTip(tip.hardness == 100 ? tr("Hard Round %1").arg(tip.size)
                                               : tr("Soft Round %1").arg(tip.size));
        grid->addWidget(button, int(i) / 6, int(i) % 6);
        connect(button, &QToolButton::clicked, this, [this, tip] {
            if (paint_) {
                paint_->setSize(tip.size);
                paint_->setHardness(tip.hardness);
                paint_->setRoundness(100);
                paint_->setTipAngle(0);
            }
        });
    }
    page->addLayout(grid);

    size_ = new NumericField(tr("Size"), field(1, 5000, tr(" px"), QStringLiteral("brushPanelSize")),
                             this);
    page->addWidget(size_);

    auto* shapeRow = new QHBoxLayout();
    auto* shapeFields = new QVBoxLayout();
    auto* flips = new QHBoxLayout();
    flipX_ = new QCheckBox(tr("Flip X"), this);
    flipX_->setObjectName(QStringLiteral("brushPanelFlipX"));
    flipY_ = new QCheckBox(tr("Flip Y"), this);
    flipY_->setObjectName(QStringLiteral("brushPanelFlipY"));
    flips->addWidget(flipX_);
    flips->addWidget(flipY_);
    shapeFields->addLayout(flips);
    angle_ = new NumericField(tr("Angle:"),
                              field(-180, 180, QStringLiteral("°"), QStringLiteral("brushPanelAngle")),
                              this);
    roundness_ = new NumericField(
        tr("Roundness:"), field(1, 100, QStringLiteral("%"), QStringLiteral("brushPanelRoundness")),
        this);
    shapeFields->addWidget(angle_);
    shapeFields->addWidget(roundness_);
    shapeRow->addLayout(shapeFields);
    shape_ = new TipShapeView(this);
    shapeRow->addWidget(shape_);
    page->addLayout(shapeRow);

    hardness_ = new NumericField(
        tr("Hardness"), field(0, 100, QStringLiteral("%"), QStringLiteral("brushPanelHardness")),
        this);
    page->addWidget(hardness_);
    auto* spacingRow = new QHBoxLayout();
    auto* spacingOn = new QCheckBox(tr("Spacing"), this);
    spacingOn->setChecked(true);
    spacingOn->setEnabled(false);
    spacingOn->setToolTip(tr("Spacing off (spacing by drag speed): not implemented yet"));
    spacing_ = new NumericField(QString(),
                                field(1, 1000, QStringLiteral("%"), QStringLiteral("brushPanelSpacing")),
                                this);
    spacingRow->addWidget(spacingOn);
    spacingRow->addWidget(spacing_, 1);
    page->addLayout(spacingRow);
    page->addStretch(1);
    body->addLayout(page, 1);

    preview_ = new QLabel(this);
    preview_->setObjectName(QStringLiteral("brushPanelPreview"));
    preview_->setFixedHeight(72);
    // The rendered pixmap must not set the label's minimum width.
    preview_->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Fixed);
    preview_->setAlignment(Qt::AlignCenter);
    preview_->setObjectName(QStringLiteral("brushTipPreview"));

    layout->addWidget(presets, 0, Qt::AlignLeft);
    layout->addLayout(body, 1);
    layout->addWidget(preview_);

    connect(size_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setSize(qRound(v));
        }
    });
    connect(hardness_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setHardness(qRound(v));
        }
    });
    connect(angle_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setTipAngle(qRound(v));
        }
    });
    connect(roundness_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setRoundness(qRound(v));
        }
    });
    connect(spacing_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setSpacing(qRound(v));
        }
    });
    auto flip = [this](bool) {
        if (paint_) {
            paint_->setFlip(flipX_->isChecked(), flipY_->isChecked());
        }
    };
    connect(flipX_, &QCheckBox::toggled, this, flip);
    connect(flipY_, &QCheckBox::toggled, this, flip);
}

void BrushPanel::setController(ToolController* controller)
{
    controller_ = controller;
    paint_ = controller ? controller->optionsFor<PaintOptions>(ToolId::Brush) : nullptr;
    if (controller_) {
        connect(controller_, &ToolController::brushSizeChanged, this, [this] { refresh(); });
        connect(controller_, &ToolController::brushTipChanged, this, [this] { refresh(); });
    }
    refresh();
}

void BrushPanel::refresh()
{
    if (!paint_) {
        return;
    }
    size_->setValue(paint_->size());
    hardness_->setValue(paint_->hardness());
    angle_->setValue(paint_->tipAngle());
    roundness_->setValue(paint_->roundness());
    spacing_->setValue(paint_->spacing());
    {
        const QSignalBlocker bx(flipX_);
        const QSignalBlocker by(flipY_);
        flipX_->setChecked(paint_->flipX());
        flipY_->setChecked(paint_->flipY());
    }
    shape_->setShape(paint_->angle(), paint_->roundness());

    // The preview paints the tip through the engine, capped so a huge brush
    // still fits the strip.
    PaintTip tip = paintTip(*paint_);
    tip.diameter = std::min(tip.diameter, preview_->height() - 12);
    const int w = std::max(preview_->width(), 120);
    const int h = preview_->height();
    const ::rust::Vec<std::uint8_t> rgba = brush_tip_preview(tip, w, h);
    if (rgba.size() == static_cast<size_t>(w) * h * 4) {
        const QImage image(rgba.data(), w, h, w * 4, QImage::Format_RGBA8888);
        preview_->setPixmap(QPixmap::fromImage(image.copy()));
    }
}

QImage BrushPanel::previewForTest() const
{
    return preview_->pixmap().toImage();
}

} // namespace pictura

#include "moc_brush_panel.cpp"
