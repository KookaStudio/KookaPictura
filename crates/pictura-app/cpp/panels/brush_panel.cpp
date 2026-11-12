#include "brush_panel.h"

#include "numeric_field.h"

#include "paint_tip.h"
#include "tools.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QRadialGradient>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QStyle>
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

// A small padlock drawn into the set row: closed when the set is locked.
QIcon padlockIcon(bool locked)
{
    QPixmap pm(12, 12);
    pm.fill(Qt::transparent);
    QPainter p(&pm);
    p.setRenderHint(QPainter::Antialiasing, true);
    const QPalette palette = QApplication::palette();
    p.setPen(QPen(palette.color(QPalette::Text), 1.2));
    if (locked) {
        p.drawArc(QRectF(3, 1, 6, 6), 0, 180 * 16);
    } else {
        p.drawArc(QRectF(4, 1, 6, 6), 0, 180 * 16);
    }
    p.setBrush(palette.color(QPalette::ButtonText));
    p.drawRoundedRect(QRectF(2.5, 5.5, 7, 5.5), 1.0, 1.0);
    return QIcon(pm);
}

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
    body->setObjectName(QStringLiteral("brushPanelBody"));
    sets_ = new QListWidget(this);
    sets_->setObjectName(QStringLiteral("brushOptionSets"));
    // Narrow enough that the docked panel never widens the column it shares.
    sets_->setMaximumWidth(110);
    auto* tipShape = new QListWidgetItem(tr("Brush Tip Shape"), sets_);
    tipShape->setData(Qt::UserRole, QStringLiteral("tip"));
    sets_->setCurrentItem(tipShape);
    for (const char* name : kDynamics) {
        const QString label = QString::fromLatin1(name);
        auto* item = new QListWidgetItem(label, sets_);
        item->setData(Qt::UserRole, label);
        if (label == QLatin1String("Build-up")) {
            // The standalone Airbrush/Build-up toggle: the options-bar Airbrush
            // button is the other view of this one flag.
            buildUp_ = item;
            item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsUserCheckable);
            item->setToolTip(tr("Build up paint while the mouse button is held "
                                "(the options-bar Airbrush option)"));
        } else if (label == QLatin1String("Shape Dynamics")
                   || label == QLatin1String("Scattering")
                   || label == QLatin1String("Color Dynamics")
                   || label == QLatin1String("Transfer")
                   || label == QLatin1String("Brush Pose")) {
            item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsUserCheckable);
            item->setCheckState(Qt::Unchecked);
            item->setToolTip(tr("%1: turn the set on or off").arg(label));
        } else {
            item->setFlags(Qt::NoItemFlags);
            item->setCheckState(Qt::Unchecked);
            item->setToolTip(tr("%1: not implemented yet").arg(label));
        }
    }
    connect(sets_, &QListWidget::itemChanged, this, [this](QListWidgetItem* item) {
        if (!paint_) {
            return;
        }
        if (item == buildUp_) {
            paint_->setAirbrush(item->checkState() == Qt::Checked);
            return;
        }
        const QString name = item->data(Qt::UserRole).toString();
        const bool on = item->checkState() == Qt::Checked;
        BrushDynamics d = paint_->dynamics();
        if (name == QLatin1String("Shape Dynamics")) {
            d.shapeEnabled = on;
        } else if (name == QLatin1String("Scattering")) {
            d.scatterEnabled = on;
        } else if (name == QLatin1String("Color Dynamics")) {
            d.colorEnabled = on;
        } else if (name == QLatin1String("Transfer")) {
            d.transferEnabled = on;
        } else if (name == QLatin1String("Brush Pose")) {
            d.poseEnabled = on;
        } else {
            return;
        }
        paint_->setDynamics(d);
        refreshDynamics();
    });
    connect(sets_, &QListWidget::currentItemChanged, this,
            [this](QListWidgetItem* current, QListWidgetItem*) {
                if (!pages_ || !current) {
                    return;
                }
                // One page per row, in row order.
                pages_->setCurrentIndex(sets_->row(current));
                refreshDynamics();
            });
    // Clicking the already-current row still shows its page: a row can keep
    // the current flag while the stack was left on another page.
    connect(sets_, &QListWidget::itemClicked, this, [this](QListWidgetItem* item) {
        if (pages_ && item) {
            pages_->setCurrentIndex(sets_->row(item));
            refreshDynamics();
        }
    });

    auto* page = new QWidget(this);
    auto* pageLayout = new QVBoxLayout(page);
    auto* grid = new QGridLayout();
    grid->setSpacing(2);
    for (size_t i = 0; i < kStandardTips.size(); ++i) {
        const StandardTip tip = kStandardTips[i];
        auto* button = new QToolButton(page);
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
    pageLayout->addLayout(grid);

    size_ = new NumericField(tr("Size"), field(1, 5000, tr(" px"), QStringLiteral("brushPanelSize")),
                             page);
    pageLayout->addWidget(size_);

    auto* shapeRow = new QHBoxLayout();
    auto* shapeFields = new QVBoxLayout();
    auto* flips = new QVBoxLayout();
    flipX_ = new QCheckBox(tr("Flip X"), page);
    flipX_->setObjectName(QStringLiteral("brushPanelFlipX"));
    flipY_ = new QCheckBox(tr("Flip Y"), page);
    flipY_->setObjectName(QStringLiteral("brushPanelFlipY"));
    flips->addWidget(flipX_);
    flips->addWidget(flipY_);
    shapeFields->addLayout(flips);
    angle_ = new NumericField(tr("Angle:"),
                              field(-180, 180, QStringLiteral("°"), QStringLiteral("brushPanelAngle")),
                              page);
    roundness_ = new NumericField(
        tr("Roundness:"), field(1, 100, QStringLiteral("%"), QStringLiteral("brushPanelRoundness")),
        page);
    shapeFields->addWidget(angle_);
    shapeFields->addWidget(roundness_);
    shapeRow->addLayout(shapeFields);
    shape_ = new TipShapeView(page);
    shapeRow->addWidget(shape_);
    pageLayout->addLayout(shapeRow);

    hardness_ = new NumericField(
        tr("Hardness"), field(0, 100, QStringLiteral("%"), QStringLiteral("brushPanelHardness")),
        page);
    pageLayout->addWidget(hardness_);

    auto* bristleLine = new QFrame(page);
    bristleLine->setFrameShape(QFrame::HLine);
    bristleLine->setFrameShadow(QFrame::Sunken);
    pageLayout->addWidget(bristleLine);
    bristleEnabled_ = new QCheckBox(tr("Bristle Tip"), page);
    bristleEnabled_->setObjectName(QStringLiteral("brushBristleEnabled"));
    bristleEnabled_->setToolTip(tr("Draw the dab as a bundle of individual bristle tracks"));
    pageLayout->addWidget(bristleEnabled_);

    auto* bristleShapeRow = new QHBoxLayout();
    bristleShapeRow->addWidget(new QLabel(tr("Shape:"), page));
    bristleShape_ = new QComboBox(page);
    bristleShape_->setObjectName(QStringLiteral("brushBristleShape"));
    bristleShape_->addItems({tr("Round"), tr("Round Fan"), tr("Round Curve"), tr("Flat"),
                             tr("Flat Fan"), tr("Flat Curve")});
    bristleShapeRow->addWidget(bristleShape_, 1);
    pageLayout->addLayout(bristleShapeRow);

    bristleDensity_ = new NumericField(
        tr("Bristles"), field(0, 100, QStringLiteral("%"), QStringLiteral("brushBristleDensity")),
        page);
    pageLayout->addWidget(bristleDensity_);
    bristleLength_ = new NumericField(
        tr("Length"), field(0, 100, QStringLiteral("%"), QStringLiteral("brushBristleLength")),
        page);
    pageLayout->addWidget(bristleLength_);
    bristleThickness_ = new NumericField(
        tr("Thickness"),
        field(0, 100, QStringLiteral("%"), QStringLiteral("brushBristleThickness")), page);
    pageLayout->addWidget(bristleThickness_);
    bristleStiffness_ = new NumericField(
        tr("Stiffness"),
        field(0, 100, QStringLiteral("%"), QStringLiteral("brushBristleStiffness")), page);
    pageLayout->addWidget(bristleStiffness_);

    auto* spacingRow = new QHBoxLayout();
    auto* spacingOn = new QCheckBox(tr("Spacing"), page);
    spacingOn->setObjectName(QStringLiteral("brushPanelSpacingOn"));
    spacingOn->setChecked(true);
    spacingOn->setEnabled(false);
    spacingOn->setToolTip(tr("Spacing off (spacing by drag speed): not implemented yet"));
    spacing_ = new NumericField(QString(),
                                field(1, 1000, QStringLiteral("%"), QStringLiteral("brushPanelSpacing")),
                                page);
    spacingRow->addWidget(spacingOn);
    spacingRow->addWidget(spacing_, 1);
    pageLayout->addLayout(spacingRow);
    pageLayout->addStretch(1);

    pages_ = new QStackedWidget(this);
    pages_->setObjectName(QStringLiteral("brushDynamicsPages"));
    pages_->addWidget(page);
    buildDynamicsPages();

    lockButton_ = new QToolButton(this);
    lockButton_->setObjectName(QStringLiteral("brushSetLock"));
    lockButton_->setCheckable(true);
    lockButton_->setText(tr("Lock"));
    lockButton_->setToolTip(tr("Keep this set's values when a brush preset is chosen"));
    connect(lockButton_, &QToolButton::clicked, this,
            [this](bool checked) { setSelectedSetLock(checked); });
    auto* setColumn = new QVBoxLayout();
    setColumn->addWidget(sets_);
    setColumn->addWidget(lockButton_, 0, Qt::AlignLeft);
    body->addLayout(setColumn);
    body->addWidget(pages_, 1);

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
    connect(bristleEnabled_, &QCheckBox::toggled, this, [this](bool on) {
        if (paint_) {
            paint_->setBristleEnabled(on);
        }
    });
    connect(bristleShape_, QOverload<int>::of(&QComboBox::currentIndexChanged), this,
            [this](int index) {
                if (paint_) {
                    paint_->setBristleShape(index);
                }
            });
    connect(bristleDensity_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setBristleDensity(qRound(v));
        }
    });
    connect(bristleLength_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setBristleLength(qRound(v));
        }
    });
    connect(bristleThickness_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setBristleThickness(qRound(v));
        }
    });
    connect(bristleStiffness_, &NumericField::valueChanged, this, [this](double v) {
        if (paint_) {
            paint_->setBristleStiffness(qRound(v));
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

void BrushPanel::buildDynamicsPages()
{
    auto addInt = [this](QBoxLayout* layout, const QString& key, const QString& label,
                         int lo, int hi, const QString& suffix,
                         int BrushDynamics::* member) {
        auto* f = new NumericField(label, field(lo, hi, suffix, key), this);
        f->setObjectName(key);
        layout->addWidget(f);
        dynWidgets_.insert(key, f);
        connect(f, &NumericField::valueChanged, this, [this, member](double v) {
            updateDynamics([member, v](BrushDynamics& d) { d.*member = qRound(v); });
        });
        dynRefreshers_.push_back([f, member](const BrushDynamics& d) {
            const QSignalBlocker block(f);
            f->setValue(d.*member);
        });
    };
    auto addBool = [this](QBoxLayout* layout, const QString& key, const QString& label,
                          bool BrushDynamics::* member) {
        auto* c = new QCheckBox(label, this);
        c->setObjectName(key);
        layout->addWidget(c);
        dynWidgets_.insert(key, c);
        connect(c, &QCheckBox::toggled, this, [this, member](bool on) {
            updateDynamics([member, on](BrushDynamics& d) { d.*member = on; });
        });
        dynRefreshers_.push_back([c, member](const BrushDynamics& d) {
            const QSignalBlocker block(c);
            c->setChecked(d.*member);
        });
    };
    auto addControl = [this](QBoxLayout* layout, const QString& key, const QString& label,
                             int BrushDynamics::* control, int BrushDynamics::* fade) {
        auto* row = new QHBoxLayout();
        row->addWidget(new QLabel(label, this));
        auto* combo = new QComboBox(this);
        combo->setObjectName(key + QStringLiteral("Control"));
        combo->addItems({tr("Off"), tr("Fade"), tr("Pen Pressure"), tr("Pen Tilt"),
                         tr("Stylus Wheel"), tr("Rotation"), tr("Initial Direction"),
                         tr("Direction")});
        auto* spin = new QSpinBox(this);
        spin->setObjectName(key + QStringLiteral("Fade"));
        spin->setRange(1, 9999);
        spin->setSuffix(tr(" steps"));
        spin->setVisible(false);
        row->addWidget(combo);
        row->addWidget(spin);
        layout->addLayout(row);
        dynWidgets_.insert(combo->objectName(), combo);
        dynWidgets_.insert(spin->objectName(), spin);
        connect(combo, QOverload<int>::of(&QComboBox::currentIndexChanged), this,
                [this, control, spin](int index) {
                    spin->setVisible(index == 1);
                    updateDynamics([control, index](BrushDynamics& d) { d.*control = index; });
                });
        connect(spin, QOverload<int>::of(&QSpinBox::valueChanged), this,
                [this, fade](int value) {
                    updateDynamics([fade, value](BrushDynamics& d) { d.*fade = value; });
                });
        dynRefreshers_.push_back([combo, spin, control, fade](const BrushDynamics& d) {
            const QSignalBlocker blockCombo(combo);
            const QSignalBlocker blockSpin(spin);
            combo->setCurrentIndex(d.*control);
            spin->setValue(d.*fade);
            spin->setVisible(d.*control == 1);
        });
    };

    for (const char* name : kDynamics) {
        const QString label = QString::fromLatin1(name);
        auto* page = new QWidget(this);
        auto* layout = new QVBoxLayout(page);
        if (label == QLatin1String("Shape Dynamics")) {
            addInt(layout, QStringLiteral("dynShapeSizeJitter"), tr("Size Jitter"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::sizeJitter);
            addInt(layout, QStringLiteral("dynShapeMinDiameter"), tr("Minimum Diameter"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::minDiameter);
            addControl(layout, QStringLiteral("dynShapeSize"), tr("Size Control"),
                       &BrushDynamics::sizeControl, &BrushDynamics::sizeFade);
            addInt(layout, QStringLiteral("dynShapeAngleJitter"), tr("Angle Jitter"), 0, 180,
                   QStringLiteral("°"), &BrushDynamics::angleJitter);
            addControl(layout, QStringLiteral("dynShapeAngle"), tr("Angle Control"),
                       &BrushDynamics::angleControl, &BrushDynamics::angleFade);
            addInt(layout, QStringLiteral("dynShapeRoundnessJitter"), tr("Roundness Jitter"), 0,
                   100, QStringLiteral("%"), &BrushDynamics::roundnessJitter);
            addInt(layout, QStringLiteral("dynShapeMinRoundness"), tr("Minimum Roundness"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::minRoundness);
            addControl(layout, QStringLiteral("dynShapeRoundness"), tr("Roundness Control"),
                       &BrushDynamics::roundnessControl, &BrushDynamics::roundnessFade);
            addInt(layout, QStringLiteral("dynShapeTiltScale"), tr("Tilt Scale"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::tiltScale);
            addBool(layout, QStringLiteral("dynShapeProjection"), tr("Brush Projection"),
                    &BrushDynamics::brushProjection);
        } else if (label == QLatin1String("Scattering")) {
            addInt(layout, QStringLiteral("dynScatterAmount"), tr("Scatter"), 0, 1000,
                   QStringLiteral("%"), &BrushDynamics::scatter);
            addControl(layout, QStringLiteral("dynScatter"), tr("Scatter Control"),
                       &BrushDynamics::scatterControl, &BrushDynamics::scatterFade);
            addBool(layout, QStringLiteral("dynScatterBothAxes"), tr("Both Axes"),
                    &BrushDynamics::bothAxes);
            addInt(layout, QStringLiteral("dynScatterCount"), tr("Count"), 1, 16, QString(),
                   &BrushDynamics::count);
            addInt(layout, QStringLiteral("dynScatterCountJitter"), tr("Count Jitter"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::countJitter);
            addControl(layout, QStringLiteral("dynScatterCount"), tr("Count Control"),
                       &BrushDynamics::countControl, &BrushDynamics::countFade);
        } else if (label == QLatin1String("Color Dynamics")) {
            addBool(layout, QStringLiteral("dynColorApplyPerTip"), tr("Apply Per Tip"),
                    &BrushDynamics::applyPerTip);
            addInt(layout, QStringLiteral("dynColorFgBgJitter"),
                   tr("Foreground/Background Jitter"), 0, 100, QStringLiteral("%"),
                   &BrushDynamics::fgBgJitter);
            addControl(layout, QStringLiteral("dynColorFgBg"), tr("FG/BG Control"),
                       &BrushDynamics::fgBgControl, &BrushDynamics::fgBgFade);
            addInt(layout, QStringLiteral("dynColorHueJitter"), tr("Hue Jitter"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::hueJitter);
            addInt(layout, QStringLiteral("dynColorSaturationJitter"), tr("Saturation Jitter"),
                   0, 100, QStringLiteral("%"), &BrushDynamics::saturationJitter);
            addInt(layout, QStringLiteral("dynColorBrightnessJitter"), tr("Brightness Jitter"),
                   0, 100, QStringLiteral("%"), &BrushDynamics::brightnessJitter);
            addInt(layout, QStringLiteral("dynColorPurity"), tr("Purity"), -100, 100,
                   QStringLiteral("%"), &BrushDynamics::purity);
        } else if (label == QLatin1String("Transfer")) {
            addInt(layout, QStringLiteral("dynTransferOpacityJitter"), tr("Opacity Jitter"), 0,
                   100, QStringLiteral("%"), &BrushDynamics::opacityJitter);
            addControl(layout, QStringLiteral("dynTransferOpacity"), tr("Opacity Control"),
                       &BrushDynamics::opacityControl, &BrushDynamics::opacityFade);
            addInt(layout, QStringLiteral("dynTransferFlowJitter"), tr("Flow Jitter"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::flowJitter);
            addControl(layout, QStringLiteral("dynTransferFlow"), tr("Flow Control"),
                       &BrushDynamics::flowControl, &BrushDynamics::flowFade);
        } else if (label == QLatin1String("Brush Pose")) {
            addInt(layout, QStringLiteral("dynPoseTiltX"), tr("Tilt X"), -90, 90,
                   QStringLiteral("°"), &BrushDynamics::poseTiltX);
            addBool(layout, QStringLiteral("dynPoseOverrideTiltX"), tr("Tilt X Override"),
                    &BrushDynamics::overrideTiltX);
            addInt(layout, QStringLiteral("dynPoseTiltY"), tr("Tilt Y"), -90, 90,
                   QStringLiteral("°"), &BrushDynamics::poseTiltY);
            addBool(layout, QStringLiteral("dynPoseOverrideTiltY"), tr("Tilt Y Override"),
                    &BrushDynamics::overrideTiltY);
            addInt(layout, QStringLiteral("dynPoseRotation"), tr("Rotation"), 0, 360,
                   QStringLiteral("°"), &BrushDynamics::poseRotation);
            addBool(layout, QStringLiteral("dynPoseOverrideRotation"), tr("Rotation Override"),
                    &BrushDynamics::overrideRotation);
            addInt(layout, QStringLiteral("dynPosePressure"), tr("Pressure"), 0, 100,
                   QStringLiteral("%"), &BrushDynamics::posePressure);
            addBool(layout, QStringLiteral("dynPoseOverridePressure"), tr("Pressure Override"),
                    &BrushDynamics::overridePressure);
        } else {
            auto* note = new QLabel(tr("%1 is not implemented yet").arg(label), page);
            note->setEnabled(false);
            layout->addWidget(note);
        }
        layout->addStretch(1);
        pages_->addWidget(page);
    }
}

void BrushPanel::updateDynamics(const std::function<void(BrushDynamics&)>& edit)
{
    if (!paint_) {
        return;
    }
    BrushDynamics d = paint_->dynamics();
    edit(d);
    paint_->setDynamics(d);
    refreshDynamics();
}

QListWidgetItem* BrushPanel::selectedSetItem() const
{
    if (!sets_) {
        return nullptr;
    }
    QListWidgetItem* item = sets_->currentItem();
    if (!item || item == buildUp_) {
        return nullptr;
    }
    const QString name = item->data(Qt::UserRole).toString();
    if (name == QLatin1String("tip") || name == QLatin1String("Texture")
        || name == QLatin1String("Dual Brush") || name == QLatin1String("Noise")
        || name == QLatin1String("Wet Edges") || name == QLatin1String("Smoothing")
        || name == QLatin1String("Protect Texture")) {
        return nullptr;
    }
    return item;
}

void BrushPanel::setSelectedSetLock(bool locked)
{
    QListWidgetItem* item = selectedSetItem();
    if (!paint_ || !item) {
        return;
    }
    const QString name = item->data(Qt::UserRole).toString();
    BrushDynamics d = paint_->dynamics();
    if (name == QLatin1String("Shape Dynamics")) {
        d.lockShape = locked;
    } else if (name == QLatin1String("Scattering")) {
        d.lockScatter = locked;
    } else if (name == QLatin1String("Color Dynamics")) {
        d.lockColor = locked;
    } else if (name == QLatin1String("Transfer")) {
        d.lockTransfer = locked;
    } else if (name == QLatin1String("Brush Pose")) {
        d.lockPose = locked;
    } else {
        return;
    }
    paint_->setDynamics(d);
    refreshDynamics();
}

void BrushPanel::refreshDynamics()
{
    if (!paint_ || !sets_) {
        return;
    }
    const BrushDynamics d = paint_->dynamics();
    for (auto& refresh : dynRefreshers_) {
        refresh(d);
    }
    for (int i = 0; i < sets_->count(); ++i) {
        QListWidgetItem* item = sets_->item(i);
        if (!item || item == buildUp_) {
            continue;
        }
        const QString name = item->data(Qt::UserRole).toString();
        bool enabled = false;
        bool locked = false;
        if (name == QLatin1String("Shape Dynamics")) {
            enabled = d.shapeEnabled;
            locked = d.lockShape;
        } else if (name == QLatin1String("Scattering")) {
            enabled = d.scatterEnabled;
            locked = d.lockScatter;
        } else if (name == QLatin1String("Color Dynamics")) {
            enabled = d.colorEnabled;
            locked = d.lockColor;
        } else if (name == QLatin1String("Transfer")) {
            enabled = d.transferEnabled;
            locked = d.lockTransfer;
        } else if (name == QLatin1String("Brush Pose")) {
            enabled = d.poseEnabled;
            locked = d.lockPose;
        } else {
            continue;
        }
        const QSignalBlocker block(sets_);
        item->setCheckState(enabled ? Qt::Checked : Qt::Unchecked);
        item->setIcon(locked ? padlockIcon(true) : QIcon());
    }
    if (lockButton_) {
        QListWidgetItem* item = selectedSetItem();
        lockButton_->setEnabled(item != nullptr);
        bool locked = false;
        if (item) {
            const QString name = item->data(Qt::UserRole).toString();
            locked = name == QLatin1String("Shape Dynamics")   ? d.lockShape
                : name == QLatin1String("Scattering")          ? d.lockScatter
                : name == QLatin1String("Color Dynamics")      ? d.lockColor
                : name == QLatin1String("Transfer")            ? d.lockTransfer
                                                               : d.lockPose;
        }
        lockButton_->setChecked(locked);
    }
}

void BrushPanel::setController(ToolController* controller)
{
    controller_ = controller;
    paint_ = controller ? controller->optionsFor<PaintOptions>(ToolId::Brush) : nullptr;
    if (controller_) {
        connect(controller_, &ToolController::brushSizeChanged, this, [this] { refresh(); });
        connect(controller_, &ToolController::brushTipChanged, this, [this] { refresh(); });
        connect(controller_, &ToolController::paintSettingsChanged, this,
                [this] { refreshAirbrush(); });
    }
    refresh();
}

// The Build-up row follows the shared flag when the options bar or
// `Shift+Alt+P` changes it. The list's signals are blocked, or setting the
// check state would write the same value back through `itemChanged`.
void BrushPanel::refreshAirbrush()
{
    if (!paint_ || !buildUp_) {
        return;
    }
    if (QWidget* list = buildUp_->listWidget()) {
        const QSignalBlocker block(list);
        buildUp_->setCheckState(paint_->airbrush() ? Qt::Checked : Qt::Unchecked);
    }
}

void BrushPanel::refresh()
{
    if (!paint_) {
        return;
    }
    refreshAirbrush();
    refreshDynamics();
    size_->setValue(paint_->size());
    hardness_->setValue(paint_->hardness());
    angle_->setValue(paint_->tipAngle());
    roundness_->setValue(paint_->roundness());
    spacing_->setValue(paint_->spacing());
    {
        const QSignalBlocker be(bristleEnabled_);
        const QSignalBlocker bs(bristleShape_);
        const QSignalBlocker bd(bristleDensity_);
        const QSignalBlocker bl(bristleLength_);
        const QSignalBlocker bt(bristleThickness_);
        const QSignalBlocker bk(bristleStiffness_);
        bristleEnabled_->setChecked(paint_->bristleEnabled());
        bristleShape_->setCurrentIndex(paint_->bristleShape());
        bristleDensity_->setValue(paint_->bristleDensity());
        bristleLength_->setValue(paint_->bristleLength());
        bristleThickness_->setValue(paint_->bristleThickness());
        bristleStiffness_->setValue(paint_->bristleStiffness());
    }
    const bool bristle = paint_->bristleEnabled();
    bristleShape_->setEnabled(bristle);
    bristleDensity_->setEnabled(bristle);
    bristleLength_->setEnabled(bristle);
    bristleThickness_->setEnabled(bristle);
    bristleStiffness_->setEnabled(bristle);
    hardness_->setEnabled(!bristle);
    roundness_->setEnabled(!bristle);
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
