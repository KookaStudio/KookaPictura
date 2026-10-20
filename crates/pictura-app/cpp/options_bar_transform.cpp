// The session-keyed Free Transform options page (#296): reference point
// locator, numeric X/Y/W/H/angle/H/V, the interpolation menu, and the warp /
// Cancel / Apply controls. In Warp mode the box fields give way to the warp
// style, orientation, and Bend/H/V fields. Every write goes through ToolController so the
// canvas overlay and the fields stay in step.

#include "options_bar.h"

#include "icons.h"
#include "image_view.h"
#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/warp.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QIcon>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QPolygonF>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

namespace {

// The interpolation menu, in CS6 order: display name and bridge id.
struct InterpolationEntry {
    const char* name;
    const char* id;
};

const InterpolationEntry kInterpolations[] = {
    {"Nearest Neighbor", "nearest"},
    {"Bilinear", "bilinear"},
    {"Bicubic", "bicubic"},
    {"Bicubic Smoother", "bicubic-smoother"},
    {"Bicubic Sharper", "bicubic-sharper"},
    {"Bicubic Automatic", "bicubic-automatic"},
};

// The Warp style menu, in CS6 order: display name and the bridge id
// (`pictura_core::warp::WarpStyle::id`; keep the two lists in step).
struct WarpStyleEntry {
    const char* name;
    const char* id;
};

const WarpStyleEntry kWarpStyles[] = {
    {"None", "none"},           {"Custom", "custom"},        {"Arc", "arc"},
    {"Arc Lower", "arcLower"},  {"Arc Upper", "arcUpper"},   {"Arch", "arch"},
    {"Bulge", "bulge"},         {"Shell Lower", "shellLower"}, {"Shell Upper", "shellUpper"},
    {"Flag", "flag"},           {"Wave", "wave"},            {"Fish", "fish"},
    {"Rise", "rise"},           {"Fisheye", "fisheye"},      {"Inflate", "inflate"},
    {"Squeeze", "squeeze"},     {"Twist", "twist"},
};

// The Free Transform fields Warp mode hides.
const char* const kBoxFields[] = {
    "optionsTransformShowPivot", "optionsTransformPivotGrid", "optionsTransformX",
    "optionsTransformRelative",  "optionsTransformY",         "optionsTransformW",
    "optionsTransformLink",      "optionsTransformH",         "optionsTransformAngle",
    "optionsTransformSkewH",     "optionsTransformSkewV",
};

// A small right-pointing triangle for the relative-positioning toggle.
QIcon relativeIcon(const QWidget* owner)
{
    QPixmap pixmap(8, 8);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setPen(Qt::NoPen);
    painter.setBrush(owner->palette().color(QPalette::WindowText));
    QPolygonF triangle;
    triangle << QPointF(1.5, 0.5) << QPointF(7.0, 4.0) << QPointF(1.5, 7.5);
    painter.drawPolygon(triangle);
    return QIcon(pixmap);
}

// One small square of the 3x3 reference point locator (the centre is filled).
QIcon pivotCellIcon(const QWidget* owner, bool centre)
{
    QPixmap pixmap(9, 9);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    const QColor ink = owner->palette().color(QPalette::WindowText);
    painter.setPen(QPen(ink, 1));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(1, 1, 6, 6);
    if (centre) {
        painter.fillRect(3, 3, 3, 3, ink);
    }
    return QIcon(pixmap);
}

QToolButton* iconButton(const QString& name, const QString& iconId, const QString& tip,
                        QWidget* parent)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(name);
    button->setIcon(icon(iconId));
    button->setIconSize(QSize(18, 18));
    button->setToolButtonStyle(Qt::ToolButtonIconOnly);
    button->setAutoRaise(true);
    button->setFocusPolicy(Qt::NoFocus);
    button->setToolTip(tip);
    return button;
}

} // namespace

QWidget* OptionsBar::buildTransformPage()
{
    auto* page = new QWidget(stack_);
    page->setObjectName(QStringLiteral("optionsTransformPage"));
    transformPage_ = page;
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);

    layout->addWidget(iconButton(QStringLiteral("optionsTransformIcon"),
                                 QStringLiteral("tool.move"), QStringLiteral("Free Transform"),
                                 page));
    auto* separator = new QFrame(page);
    separator->setObjectName(QStringLiteral("optionsToolSeparator"));
    separator->setFrameShape(QFrame::VLine);
    separator->setFrameShadow(QFrame::Sunken);
    separator->setFixedHeight(18);
    separator->setAttribute(Qt::WA_TransparentForMouseEvents);
    layout->addWidget(separator);

    auto* showPivot = iconButton(QStringLiteral("optionsTransformShowPivot"),
                                 QStringLiteral("info.crosshair"),
                                 QStringLiteral("Show Reference Point"), page);
    showPivot->setCheckable(true);
    showPivot->setChecked(true);
    layout->addWidget(showPivot);
    connect(showPivot, &QToolButton::toggled, this, [this](bool on) {
        if (controller_) {
            controller_->setTransformShowPivot(on);
        }
    });

    // The 3x3 reference point locator: nine square cells, row-major.
    auto* locator = new QWidget(page);
    locator->setObjectName(QStringLiteral("optionsTransformPivotGrid"));
    auto* grid = new QGridLayout(locator);
    grid->setContentsMargins(2, 0, 2, 0);
    grid->setSpacing(1);
    for (int i = 0; i < 9; ++i) {
        auto* cell = new QToolButton(locator);
        cell->setObjectName(QStringLiteral("optionsPivot%1").arg(i));
        cell->setIcon(pivotCellIcon(page, i == 4));
        cell->setIconSize(QSize(9, 9));
        cell->setFixedSize(13, 13);
        cell->setAutoRaise(true);
        cell->setFocusPolicy(Qt::NoFocus);
        cell->setToolTip(QStringLiteral("Reference point preset"));
        grid->addWidget(cell, i / 3, i % 3);
        connect(cell, &QToolButton::clicked, this, [this, i]() {
            if (controller_) {
                controller_->setTransformPivotPreset(i);
            }
        });
    }
    layout->addWidget(locator);

    auto* xField = new NumericField(
        QStringLiteral("X"),
        numericConfig(-1000000.0, 1000000.0, 1.0, 1, QStringLiteral("px"), true,
                      QStringLiteral("optionsTransformX")),
        page);
    layout->addWidget(xField);

    auto* relative = new QToolButton(page);
    relative->setObjectName(QStringLiteral("optionsTransformRelative"));
    relative->setIcon(relativeIcon(page));
    relative->setIconSize(QSize(8, 8));
    relative->setCheckable(true);
    relative->setAutoRaise(true);
    relative->setFocusPolicy(Qt::NoFocus);
    relative->setToolTip(QStringLiteral("Use Relative Positioning For Reference Point"));
    layout->addWidget(relative);
    connect(relative, &QToolButton::toggled, this, [this](bool on) {
        transformRelative_ = on;
        refreshTransformPage();
    });

    auto* yField = new NumericField(
        QStringLiteral("Y"),
        numericConfig(-1000000.0, 1000000.0, 1.0, 1, QStringLiteral("px"), true,
                      QStringLiteral("optionsTransformY")),
        page);
    layout->addWidget(yField);

    // X/Y move the reference point only; the relative toggle makes them offsets
    // from the session origin.
    connect(xField, &NumericField::valueChanged, this, [this, yField](double value) {
        if (!controller_) {
            return;
        }
        PictureView* v = controller_->view();
        if (!v) {
            return;
        }
        const double x =
            transformRelative_ ? transform_pivot_origin_x(*v) + value : value;
        const double y = transformRelative_ ? transform_pivot_origin_y(*v) + yField->value()
                                            : yField->value();
        controller_->setTransformPivot(x, y);
    });
    connect(yField, &NumericField::valueChanged, this, [this, xField](double value) {
        if (!controller_) {
            return;
        }
        PictureView* v = controller_->view();
        if (!v) {
            return;
        }
        const double x = transformRelative_ ? transform_pivot_origin_x(*v) + xField->value()
                                            : xField->value();
        const double y =
            transformRelative_ ? transform_pivot_origin_y(*v) + value : value;
        controller_->setTransformPivot(x, y);
    });

    auto* wField = new NumericField(
        QStringLiteral("W"),
        numericConfig(0.1, 10000.0, 1.0, 1, QStringLiteral("%"), true,
                      QStringLiteral("optionsTransformW")),
        page);
    layout->addWidget(wField);
    connect(wField, &NumericField::valueChanged, this, [this](double value) {
        if (controller_) {
            controller_->setTransformWidth(value, transformLink_);
        }
    });

    auto* link = iconButton(QStringLiteral("optionsTransformLink"),
                            QStringLiteral("layers.link"),
                            QStringLiteral("Maintain Aspect Ratio"), page);
    link->setCheckable(true);
    link->setChecked(transformLink_);
    layout->addWidget(link);
    connect(link, &QToolButton::toggled, this, [this, link](bool on) {
        transformLink_ = on;
        link->setIcon(icon(on ? QStringLiteral("layers.link") : QStringLiteral("layers.unlink2")));
    });

    auto* hField = new NumericField(
        QStringLiteral("H"),
        numericConfig(0.1, 10000.0, 1.0, 1, QStringLiteral("%"), true,
                      QStringLiteral("optionsTransformH")),
        page);
    layout->addWidget(hField);
    connect(hField, &NumericField::valueChanged, this, [this](double value) {
        if (controller_) {
            controller_->setTransformHeight(value, transformLink_);
        }
    });

    auto* angleField = new NumericField(
        QStringLiteral("Angle"),
        numericConfig(-360.0, 360.0, 1.0, 1, QStringLiteral("\u00B0"), true,
                      QStringLiteral("optionsTransformAngle")),
        page);
    layout->addWidget(angleField);
    connect(angleField, &NumericField::valueChanged, this, [this](double value) {
        if (controller_) {
            controller_->setTransformAngle(value);
        }
    });

    auto* skewHField = new NumericField(
        QStringLiteral("H"),
        numericConfig(-89.0, 89.0, 1.0, 1, QStringLiteral("\u00B0"), true,
                      QStringLiteral("optionsTransformSkewH")),
        page);
    layout->addWidget(skewHField);
    auto* skewVField = new NumericField(
        QStringLiteral("V"),
        numericConfig(-89.0, 89.0, 1.0, 1, QStringLiteral("\u00B0"), true,
                      QStringLiteral("optionsTransformSkewV")),
        page);
    layout->addWidget(skewVField);
    connect(skewHField, &NumericField::valueChanged, this, [this, skewVField](double value) {
        if (controller_) {
            controller_->setTransformSkew(value, skewVField->value());
        }
    });
    connect(skewVField, &NumericField::valueChanged, this, [this, skewHField](double value) {
        if (controller_) {
            controller_->setTransformSkew(skewHField->value(), value);
        }
    });

    layout->addWidget(buildWarpFields(page));

    auto* interpolation = new QComboBox(page);
    interpolation->setObjectName(QStringLiteral("optionsTransformInterpolation"));
    for (const InterpolationEntry& entry : kInterpolations) {
        interpolation->addItem(QString::fromLatin1(entry.name), QString::fromLatin1(entry.id));
    }
    interpolation->setToolTip(QStringLiteral("Interpolation"));
    layout->addWidget(interpolation);
    connect(interpolation, &QComboBox::currentIndexChanged, this,
            [this, interpolation](int) {
                if (controller_) {
                    controller_->setTransformInterpolation(
                        interpolation->currentData().toString());
                }
            });

    auto* warp = iconButton(QStringLiteral("optionsTransformWarp"),
                            QStringLiteral("options.warp"),
                            QStringLiteral("Switch Between Free Transform And Warp Modes"),
                            page);
    warp->setCheckable(true);
    layout->addWidget(warp);
    connect(warp, &QToolButton::clicked, this, [this](bool on) {
        if (!controller_) {
            return;
        }
        if (on) {
            controller_->enterTransformWarp();
        } else {
            controller_->leaveTransformWarp();
        }
        refreshTransformPage();
    });

    auto* cancel = iconButton(QStringLiteral("optionsTransformCancel"),
                              QStringLiteral("crop.cancel"), QStringLiteral("Cancel (Esc)"),
                              page);
    layout->addWidget(cancel);
    connect(cancel, &QToolButton::clicked, this, [this]() {
        if (controller_) {
            controller_->cancelFreeTransform();
        }
    });
    auto* apply = iconButton(QStringLiteral("optionsTransformApply"),
                             QStringLiteral("crop.apply"), QStringLiteral("Apply (Enter)"),
                             page);
    layout->addWidget(apply);
    connect(apply, &QToolButton::clicked, this, [this]() {
        if (controller_) {
            controller_->commitFreeTransform();
        }
    });

    layout->addStretch(1);
    return page;
}

// Warp mode's fields: the style menu, the orientation toggle, and the
// Bend/H/V percentages, which act only on a preset style.
QWidget* OptionsBar::buildWarpFields(QWidget* page)
{
    auto* fields = new QWidget(page);
    fields->setObjectName(QStringLiteral("optionsWarpFields"));
    fields->setVisible(false);
    auto* layout = new QHBoxLayout(fields);
    layout->setContentsMargins(0, 0, 0, 0);

    auto* style = new QComboBox(fields);
    style->setObjectName(QStringLiteral("optionsWarpStyle"));
    for (const WarpStyleEntry& entry : kWarpStyles) {
        style->addItem(QString::fromLatin1(entry.name), QString::fromLatin1(entry.id));
    }
    style->setToolTip(QStringLiteral("Warp"));
    layout->addWidget(style);
    connect(style, &QComboBox::currentIndexChanged, this, [this, style](int) {
        if (controller_) {
            controller_->setTransformWarpStyle(style->currentData().toString());
            refreshTransformPage();
        }
    });

    auto* orientation = iconButton(QStringLiteral("optionsWarpOrientation"),
                                   QStringLiteral("options.warp"),
                                   QStringLiteral("Change The Warp Orientation"), fields);
    orientation->setCheckable(true);
    layout->addWidget(orientation);
    connect(orientation, &QToolButton::toggled, this, [this](bool on) {
        if (controller_) {
            controller_->setTransformWarpVertical(on);
        }
    });

    NumericField* values[3] = {};
    const char* const names[3] = {"optionsWarpBend", "optionsWarpH", "optionsWarpV"};
    const char* const labels[3] = {"Bend", "H", "V"};
    for (int i = 0; i < 3; ++i) {
        values[i] = new NumericField(
            QString::fromLatin1(labels[i]),
            numericConfig(-100.0, 100.0, 1.0, 1, QStringLiteral("%"), true,
                          QString::fromLatin1(names[i])),
            fields);
        layout->addWidget(values[i]);
    }
    for (int i = 0; i < 3; ++i) {
        connect(values[i], &NumericField::valueChanged, this, [this, values](double) {
            if (controller_) {
                controller_->setTransformWarpValues(values[0]->value(), values[1]->value(),
                                                    values[2]->value());
            }
        });
    }
    return fields;
}

void OptionsBar::refreshTransformPage()
{
    if (!transformPage_ || !controller_) {
        return;
    }
    PictureView* v = controller_->view();
    if (!v || !v->transform_session_active()) {
        return;
    }
    const auto field = [this](const char* name) {
        return transformPage_->findChild<NumericField*>(QString::fromLatin1(name));
    };
    NumericField* xField = field("optionsTransformX");
    NumericField* yField = field("optionsTransformY");
    NumericField* wField = field("optionsTransformW");
    NumericField* hField = field("optionsTransformH");
    NumericField* angleField = field("optionsTransformAngle");
    NumericField* skewHField = field("optionsTransformSkewH");
    NumericField* skewVField = field("optionsTransformSkewV");
    auto* showPivot = transformPage_->findChild<QToolButton*>(
        QStringLiteral("optionsTransformShowPivot"));
    auto* relative = transformPage_->findChild<QToolButton*>(
        QStringLiteral("optionsTransformRelative"));
    auto* warp = transformPage_->findChild<QToolButton*>(QStringLiteral("optionsTransformWarp"));
    auto* interpolation = transformPage_->findChild<QComboBox*>(
        QStringLiteral("optionsTransformInterpolation"));
    if (!xField || !yField || !wField || !hField || !angleField || !skewHField || !skewVField
        || !showPivot || !relative || !warp || !interpolation) {
        return;
    }

    const bool freeMode = transform_session_mode(*v) == QStringLiteral("free");
    const QString blockedTip =
        QStringLiteral("Not editable in Skew, Distort, or Perspective; drag a handle instead.");
    wField->setEnabled(freeMode);
    hField->setEnabled(freeMode);
    wField->setToolTip(freeMode ? QString() : blockedTip);
    hField->setToolTip(freeMode ? QString() : blockedTip);

    {
        const QSignalBlocker block(showPivot);
        showPivot->setChecked(transform_show_pivot(*v));
    }
    {
        const QSignalBlocker block(relative);
        relative->setChecked(transformRelative_);
    }
    const double originX = transform_pivot_origin_x(*v);
    const double originY = transform_pivot_origin_y(*v);
    {
        const QSignalBlocker bx(xField);
        const QSignalBlocker by(yField);
        xField->setValue(transformRelative_ ? transform_pivot_x(*v) - originX
                                            : transform_pivot_x(*v));
        yField->setValue(transformRelative_ ? transform_pivot_y(*v) - originY
                                            : transform_pivot_y(*v));
    }
    {
        const QSignalBlocker bw(wField);
        const QSignalBlocker bh(hField);
        wField->setValue(transform_width_pct(*v));
        hField->setValue(transform_height_pct(*v));
    }
    {
        const QSignalBlocker ba(angleField);
        const QSignalBlocker bsh(skewHField);
        const QSignalBlocker bsv(skewVField);
        angleField->setValue(transform_angle_degrees(*v));
        skewHField->setValue(transform_skew_h_degrees(*v));
        skewVField->setValue(transform_skew_v_degrees(*v));
    }
    {
        const QSignalBlocker block(interpolation);
        const int index =
            interpolation->findData(QString::fromUtf8(transform_interpolation_id(*v).toUtf8()));
        if (index >= 0) {
            interpolation->setCurrentIndex(index);
        }
    }
    const bool warping = transform_warp_active(*v);
    {
        const QSignalBlocker block(warp);
        warp->setChecked(warping);
    }
    for (const char* name : kBoxFields) {
        if (QWidget* w = transformPage_->findChild<QWidget*>(QString::fromLatin1(name))) {
            w->setVisible(!warping);
        }
    }
    refreshWarpFields(v, warping);
}

void OptionsBar::refreshWarpFields(PictureView* v, bool warping)
{
    auto* fields = transformPage_->findChild<QWidget*>(QStringLiteral("optionsWarpFields"));
    auto* style = transformPage_->findChild<QComboBox*>(QStringLiteral("optionsWarpStyle"));
    auto* orientation =
        transformPage_->findChild<QToolButton*>(QStringLiteral("optionsWarpOrientation"));
    auto* bend = transformPage_->findChild<NumericField*>(QStringLiteral("optionsWarpBend"));
    auto* h = transformPage_->findChild<NumericField*>(QStringLiteral("optionsWarpH"));
    auto* vField = transformPage_->findChild<NumericField*>(QStringLiteral("optionsWarpV"));
    if (!fields || !style || !orientation || !bend || !h || !vField) {
        return;
    }
    fields->setVisible(warping);
    if (!warping) {
        return;
    }
    const QString id = QString::fromUtf8(transform_warp_style_id(*v).toUtf8());
    const bool preset = id != QStringLiteral("none") && id != QStringLiteral("custom");
    {
        const QSignalBlocker block(style);
        style->setCurrentIndex(style->findData(id));
    }
    const QSignalBlocker bo(orientation);
    const QSignalBlocker bb(bend);
    const QSignalBlocker bh(h);
    const QSignalBlocker bv(vField);
    orientation->setChecked(transform_warp_vertical(*v));
    bend->setValue(transform_warp_bend(*v));
    h->setValue(transform_warp_h_distort(*v));
    vField->setValue(transform_warp_v_distort(*v));
    for (QWidget* w : {static_cast<QWidget*>(orientation), static_cast<QWidget*>(bend),
                       static_cast<QWidget*>(h), static_cast<QWidget*>(vField)}) {
        w->setEnabled(preset);
    }
}

} // namespace pictura
