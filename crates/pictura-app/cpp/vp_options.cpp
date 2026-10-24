#include "vp_options.h"
#include "vp_dialog.h"

#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include <tuple>
#include <utility>

#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point_retouch.cxxqt.h"

namespace pictura {

namespace {

QHBoxLayout* row(QWidget* page)
{
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(6);
    return layout;
}

QSpinBox* field(QWidget* page, QHBoxLayout* layout, const QString& label, const QString& name,
                int minimum, int maximum, int value, const QString& suffix)
{
    auto* box = new QSpinBox(page);
    box->setObjectName(name);
    box->setRange(minimum, maximum);
    box->setValue(value);
    box->setSuffix(suffix);
    auto* caption = new QLabel(label, page);
    caption->setBuddy(box);
    layout->addWidget(caption);
    layout->addWidget(box);
    layout->addSpacing(6);
    return box;
}

QComboBox* healBox(QWidget* page, QHBoxLayout* layout, const QString& name)
{
    auto* box = new QComboBox(page);
    box->setObjectName(name);
    box->addItems({QStringLiteral("Off"), QStringLiteral("Luminance"), QStringLiteral("On")});
    layout->addWidget(new QLabel(QStringLiteral("Heal:"), page));
    layout->addWidget(box);
    layout->addSpacing(6);
    return box;
}

} // namespace

VpOptionsBar::VpOptionsBar(QWidget* parent) : QStackedWidget(parent)
{
    setObjectName(QStringLiteral("vpOptions"));
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);

    planesPage_ = new QWidget(this);
    auto* planes = row(planesPage_);
    // ponytail: CS6's Grid Size range is not confirmed; 1–1000 is assumed.
    gridSize_ = field(planesPage_, planes, QStringLiteral("Grid Size:"), QStringLiteral("vpGridSize"),
                      1, 1000, 100, QString());
    gridSize_->setToolTip(QStringLiteral("Grid Size: the spacing of the active plane's grid."));
    angle_ = field(planesPage_, planes, QStringLiteral("Angle:"), QStringLiteral("vpAngle"), 0, 360,
                   0, QStringLiteral("°"));
    angle_->setToolTip(QStringLiteral("Angle: a related plane's angle to the plane it tore off."));
    planes->addStretch(1);

    marqueePage_ = new QWidget(this);
    auto* marquee = row(marqueePage_);
    feather_ = field(marqueePage_, marquee, QStringLiteral("Feather:"), QStringLiteral("vpFeather"),
                     0, 250, 0, QStringLiteral(" px"));
    marqueeOpacity_ = field(marqueePage_, marquee, QStringLiteral("Opacity:"),
                            QStringLiteral("vpMarqueeOpacity"), 0, 100, 100, QStringLiteral(" %"));
    marqueeHeal_ = healBox(marqueePage_, marquee, QStringLiteral("vpMarqueeHeal"));
    moveMode_ = new QComboBox(marqueePage_);
    moveMode_->setObjectName(QStringLiteral("vpMoveMode"));
    moveMode_->addItems({QStringLiteral("Destination"), QStringLiteral("Source")});
    marquee->addWidget(new QLabel(QStringLiteral("Move Mode:"), marqueePage_));
    marquee->addWidget(moveMode_);
    marquee->addStretch(1);
    for (QSpinBox* box : {feather_, marqueeOpacity_}) {
        connect(box, &QSpinBox::valueChanged, this, &VpOptionsBar::pushMarquee);
    }
    for (QComboBox* box : {marqueeHeal_, moveMode_}) {
        connect(box, &QComboBox::currentIndexChanged, this, &VpOptionsBar::pushMarquee);
    }

    auto paintPage = [this](const QString& prefix, Paint* paint, bool brush) {
        auto* page = new QWidget(this);
        auto* layout = row(page);
        paint->diameter = field(page, layout, QStringLiteral("Diameter:"), prefix + QStringLiteral("Diameter"),
                                1, 5000, 100, QStringLiteral(" px"));
        paint->hardness = field(page, layout, QStringLiteral("Hardness:"), prefix + QStringLiteral("Hardness"),
                                0, 100, 100, QStringLiteral(" %"));
        paint->opacity = field(page, layout, QStringLiteral("Opacity:"), prefix + QStringLiteral("Opacity"),
                               0, 100, 100, QStringLiteral(" %"));
        paint->heal = healBox(page, layout, prefix + QStringLiteral("Heal"));
        for (QSpinBox* box : {paint->diameter, paint->hardness, paint->opacity}) {
            connect(box, &QSpinBox::valueChanged, this, [this, brush] { pushPaint(brush); });
        }
        connect(paint->heal, &QComboBox::currentIndexChanged, this, [this, brush] { pushPaint(brush); });
        return std::make_pair(page, layout);
    };
    auto [stampPage, stampRow] = paintPage(QStringLiteral("vpStamp"), &stamp_, false);
    stampPage_ = stampPage;
    auto* aligned = new QCheckBox(QStringLiteral("Aligned"), stampPage_);
    aligned->setObjectName(QStringLiteral("vpAligned"));
    aligned->setChecked(true);
    connect(aligned, &QCheckBox::toggled, this, [](bool on) { vp_set_aligned(on); });
    stampRow->addWidget(aligned);
    stampRow->addStretch(1);

    auto swatch = [this](QWidget* page, QHBoxLayout* layout, const QString& name) {
        auto* button = new QToolButton(page);
        button->setObjectName(name);
        button->setToolTip(QStringLiteral("Brush Color"));
        connect(button, &QToolButton::clicked, this, &VpOptionsBar::pickColour);
        layout->addWidget(new QLabel(QStringLiteral("Brush Color:"), page));
        layout->addWidget(button);
        swatches_.append(button);
    };
    auto [brushPage, brushRow] = paintPage(QStringLiteral("vpBrush"), &brush_, true);
    brushPage_ = brushPage;
    swatch(brushPage_, brushRow, QStringLiteral("vpBrushColour"));
    brushRow->addStretch(1);

    transformPage_ = new QWidget(this);
    auto* transform = row(transformPage_);
    for (const auto& [label, name, horizontal] :
         {std::tuple{"Flip", "vpFlip", true}, std::tuple{"Flop", "vpFlop", false}}) {
        auto* button = new QToolButton(transformPage_);
        button->setObjectName(QString::fromLatin1(name));
        button->setText(QString::fromLatin1(label));
        connect(button, &QToolButton::clicked, this, [this, horizontal = horizontal] {
            if (mirrored) {
                mirrored(horizontal);
            }
        });
        transform->addWidget(button);
    }
    transform->addStretch(1);

    eyedropperPage_ = new QWidget(this);
    auto* eyedropper = row(eyedropperPage_);
    swatch(eyedropperPage_, eyedropper, QStringLiteral("vpEyedropperColour"));
    eyedropper->addStretch(1);

    emptyPage_ = new QWidget(this);
    for (QWidget* page : {planesPage_, marqueePage_, stampPage_, brushPage_, transformPage_,
                          eyedropperPage_, emptyPage_}) {
        addWidget(page);
    }
    setColour(colour_);
    pushMarquee();
    pushPaint(false);
    pushPaint(true);
}

void VpOptionsBar::showFor(int tool)
{
    switch (tool) {
    case VanishingPointDialog::EditPlane:
    case VanishingPointDialog::CreatePlane:
        setCurrentWidget(planesPage_);
        break;
    case VanishingPointDialog::Marquee:
        setCurrentWidget(marqueePage_);
        break;
    case VanishingPointDialog::Stamp:
        setCurrentWidget(stampPage_);
        break;
    case VanishingPointDialog::Brush:
        setCurrentWidget(brushPage_);
        break;
    case VanishingPointDialog::Transform:
        setCurrentWidget(transformPage_);
        break;
    case VanishingPointDialog::Eyedropper:
        setCurrentWidget(eyedropperPage_);
        break;
    default:
        setCurrentWidget(emptyPage_);
        break;
    }
}

void VpOptionsBar::setColour(const QColor& colour)
{
    colour_ = colour;
    QPixmap chip(16, 16);
    chip.fill(colour);
    for (QToolButton* button : swatches_) {
        button->setIcon(QIcon(chip));
    }
    vp_set_brush_colour(colour.red(), colour.green(), colour.blue());
}

void VpOptionsBar::stepDiameter(int tool, int direction)
{
    QSpinBox* box = tool == VanishingPointDialog::Stamp   ? stamp_.diameter
                    : tool == VanishingPointDialog::Brush ? brush_.diameter
                                                          : nullptr;
    if (!box) {
        return;
    }
    // CS6's bracket steps: finer for small brushes.
    const int v = box->value();
    const int step = v < 10 ? 1 : v < 100 ? 10 : v < 500 ? 50 : 100;
    box->setValue(v + direction * step);
}

void VpOptionsBar::pushMarquee()
{
    vp_set_marquee_options(feather_->value(), marqueeOpacity_->value(), marqueeHeal_->currentIndex(),
                           moveMode_->currentIndex() == 1);
}

void VpOptionsBar::pushPaint(bool brush)
{
    const Paint& p = brush ? brush_ : stamp_;
    vp_set_paint_options(brush, p.diameter->value(), p.hardness->value(), p.opacity->value(),
                         p.heal->currentIndex());
}

void VpOptionsBar::pickColour()
{
    const QColor chosen = QColorDialog::getColor(colour_, this, QStringLiteral("Brush Color"));
    if (chosen.isValid()) {
        setColour(chosen);
    }
}

} // namespace pictura

#include "moc_vp_options.cpp"
