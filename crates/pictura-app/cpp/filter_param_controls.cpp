#include "filter_param_controls.h"
#include "color_picker_dialog.h"
#include "panels/jump_slider.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QVariant>
#include <QtGui/QColor>
#include <QtGui/QFontMetrics>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtGui/QResizeEvent>
#include <QtMath>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <cmath>
#include <functional>

namespace pictura {

namespace {

int slotCount(FilterControl control)
{
    switch (control) {
    case FilterControl::Color:
        return 3;
    case FilterControl::Placement:
        return 2;
    case FilterControl::BlurCenter:
        return 0;
    case FilterControl::ShearCurve:
        return 1 + 2 * kShearMaxPoints;
    default:
        return 1;
    }
}

// A slider whose value is a distance in document pixels.
bool measuresPixels(const FilterParamSpec& spec)
{
    const QString unit = spec.suffix.trimmed();
    return spec.control == FilterControl::Slider
           && (unit.startsWith(QStringLiteral("pixels")) || unit == QStringLiteral("px"));
}

// A value box only needs room for five digits and a decimal (with a sign); a
// wider box would just waste the row.
int compactSpinWidth(const QWidget* widget)
{
    return widget->fontMetrics().horizontalAdvance(QStringLiteral("-00000.0")) + 34;
}

// Display-only draggable centre pad for Radial Blur: concentric rings for a
// spin, rays for a zoom. It carries no value.
class BlurCenterWidget : public QWidget {
public:
    explicit BlurCenterWidget(QWidget* parent = nullptr) : QWidget(parent)
    {
        setMinimumSize(120, 120);
        setCursor(Qt::CrossCursor);
    }

    void setZoom(bool zoom)
    {
        zoom_ = zoom;
        update();
    }

protected:
    void resizeEvent(QResizeEvent*) override
    {
        if (!moved_) {
            center_ = QPointF(width() / 2.0, height() / 2.0);
        }
    }

    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.fillRect(rect(), QColor(48, 48, 48));
        const double radius = qMin(width(), height()) / 2.0 - 4.0;
        painter.setPen(QPen(QColor(160, 160, 160), 1));
        for (int i = 1; i <= 4; ++i) {
            painter.drawEllipse(center_, radius * i / 4.0, radius * i / 4.0);
        }
        if (zoom_) {
            for (int degrees = 0; degrees < 360; degrees += 30) {
                const double a = qDegreesToRadians(static_cast<double>(degrees));
                painter.drawLine(QPointF(center_.x() + qCos(a) * radius * 0.15,
                                         center_.y() + qSin(a) * radius * 0.15),
                                 QPointF(center_.x() + qCos(a) * radius,
                                         center_.y() + qSin(a) * radius));
            }
        }
        painter.setPen(QPen(Qt::white, 2));
        painter.drawLine(QPointF(center_.x() - 6, center_.y()),
                         QPointF(center_.x() + 6, center_.y()));
        painter.drawLine(QPointF(center_.x(), center_.y() - 6),
                         QPointF(center_.x(), center_.y() + 6));
    }

    void mousePressEvent(QMouseEvent* event) override { moveTo(event->position()); }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (event->buttons() & Qt::LeftButton) {
            moveTo(event->position());
        }
    }

private:
    void moveTo(const QPointF& pos)
    {
        moved_ = true;
        center_ = QPointF(qBound(0.0, pos.x(), static_cast<double>(width())),
                          qBound(0.0, pos.y(), static_cast<double>(height())));
        update();
    }

    QPointF center_{60.0, 60.0};
    bool zoom_ = false;
    bool moved_ = false;
};

// Draggable normalized-placement pad for Lens Flare: shows the image (or a
// plain frame) with a crosshair. `moved` reports normalized (0..1) x/y.
class PlacementPad : public QWidget {
public:
    explicit PlacementPad(QWidget* parent = nullptr) : QWidget(parent)
    {
        setMinimumSize(120, 120);
        setCursor(Qt::CrossCursor);
    }

    std::function<void(double, double)> moved;

    void setImage(const QImage& image)
    {
        image_ = image;
        update();
    }

    // Silent: used by the spin boxes so the two never feed back.
    void setPosition(double x, double y)
    {
        x_ = qBound(0.0, x, 1.0);
        y_ = qBound(0.0, y, 1.0);
        update();
    }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        if (!image_.isNull()) {
            painter.drawImage(rect(), image_);
        } else {
            painter.fillRect(rect(), QColor(48, 48, 48));
            painter.setPen(QPen(QColor(160, 160, 160), 1));
            painter.drawRect(rect().adjusted(0, 0, -1, -1));
        }
        const QPointF p(x_ * width(), y_ * height());
        painter.setPen(QPen(Qt::white, 2));
        painter.drawLine(QPointF(p.x() - 7, p.y()), QPointF(p.x() + 7, p.y()));
        painter.drawLine(QPointF(p.x(), p.y() - 7), QPointF(p.x(), p.y() + 7));
    }

    void mousePressEvent(QMouseEvent* event) override { moveTo(event->position()); }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (event->buttons() & Qt::LeftButton) {
            moveTo(event->position());
        }
    }

private:
    void moveTo(const QPointF& pos)
    {
        x_ = qBound(0.0, pos.x() / width(), 1.0);
        y_ = qBound(0.0, pos.y() / height(), 1.0);
        update();
        if (moved) {
            moved(x_, y_);
        }
    }

    QImage image_;
    double x_ = 0.5;
    double y_ = 0.5;
};

QColor buttonColor(const QPushButton* button)
{
    return button->property("filterColor").value<QColor>();
}

// A colour parameter shows as a swatch, as CS6's does; the hex is only the tooltip.
void setButtonColor(QPushButton* button, const QColor& color)
{
    button->setProperty("filterColor", color);
    button->setToolTip(color.name());
    button->setStyleSheet(
        QStringLiteral("QPushButton { background-color: %1; border: 1px solid #000; }")
            .arg(color.name()));
}

} // namespace

namespace {

constexpr int kShearCurveBoxSize = 140;
constexpr double kShearGrabRadius = 7.0;

// CS6's Shear curve: a box in which a line runs from the top of the image to
// the bottom, and dragging it sideways pushes those rows sideways.
//
// Click on the line to add a point, drag one to move it, and drag one out of
// the box to take it away — as CS6's does. The two ends cannot be removed,
// since a curve with fewer than two points is not a curve.
class ShearCurveWidget : public QWidget {
public:
    explicit ShearCurveWidget(QWidget* parent) : QWidget(parent)
    {
        setObjectName(QStringLiteral("shearCurveWidget"));
        setFixedSize(kShearCurveBoxSize, kShearCurveBoxSize);
        setCursor(Qt::CrossCursor);
        setToolTip(QStringLiteral("Drag the line to bend the image; click it to add a point, "
                                  "drag a point out to remove"));
    }

    std::function<void()> changed;
    std::function<void()> changedLive;

    // The control points, `x` the offset in -1..1 and `y` the height down the
    // image in 0..1, kept sorted by `y`. The first and last are the ends.
    const QList<QPointF>& points() const { return points_; }

    // Replace the curve with the stored control points; fewer than two falls
    // back to the straight line.
    void setPoints(const QList<QPointF>& points)
    {
        if (points.size() >= 2) {
            points_.clear();
            for (const QPointF& p : points) {
                points_.append(QPointF(qBound(-1.0, p.x(), 1.0), qBound(0.0, p.y(), 1.0)));
            }
        }
        update();
    }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.fillRect(rect(), Qt::white);
        painter.setRenderHint(QPainter::Antialiasing, true);

        // The dotted 4x4 grid CS6 rules the box with.
        painter.setPen(QPen(QColor(0xa0, 0xa0, 0xa0), 1.0, Qt::DotLine));
        for (int i = 1; i < 4; ++i) {
            const double t = static_cast<double>(i) / 4.0;
            painter.drawLine(QPointF(t * width(), 0), QPointF(t * width(), height()));
            painter.drawLine(QPointF(0, t * height()), QPointF(width(), t * height()));
        }

        QPolygonF line;
        for (int y = 0; y < height(); ++y) {
            const double ny = height() > 1 ? static_cast<double>(y) / (height() - 1) : 0.0;
            line.append(QPointF((offsetAt(ny) + 1.0) / 2.0 * (width() - 1), y));
        }
        painter.setPen(QPen(QColor(0x20, 0x20, 0x20), 1.4));
        painter.drawPolyline(line);

        painter.setBrush(QColor(0x20, 0x20, 0x20));
        painter.setPen(Qt::NoPen);
        for (int i = 0; i < points_.size(); ++i) {
            const QPointF p = at(i);
            painter.drawRect(QRectF(p.x() - 2.5, p.y() - 2.5, 5, 5));
        }

        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(QColor(0x50, 0x50, 0x50), 1));
        painter.drawRect(rect().adjusted(0, 0, -1, -1));
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        const QPointF pos = event->position();
        for (int i = 0; i < points_.size(); ++i) {
            if (QLineF(pos, at(i)).length() <= kShearGrabRadius) {
                dragging_ = i;
                return;
            }
        }

        // Not on a point: a click on the line adds one there, in curve order.
        // A click off the line is ignored, as CS6's is; at the cap the click
        // is ignored too, as CS6 stops adding.
        if (points_.size() >= kShearMaxPoints) {
            return;
        }
        const double y = qBound(0.0, pos.y() / (height() - 1), 1.0);
        const double lineX = (offsetAt(y) + 1.0) / 2.0 * (width() - 1);
        if (qAbs(pos.x() - lineX) > kShearGrabRadius) {
            return;
        }
        const double x = qBound(-1.0, pos.x() / (width() - 1) * 2.0 - 1.0, 1.0);
        int index = 1;
        while (index < points_.size() - 1 && points_.at(index).y() < y) {
            ++index;
        }
        points_.insert(index, QPointF(x, y));
        dragging_ = index;
        dirty_ = true;
        update();
        if (changedLive) {
            changedLive();
        }
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (dragging_ < 0) {
            return;
        }
        const QPointF pos = event->position();
        QPointF& point = points_[dragging_];
        point.setX(qBound(-1.0, pos.x() / (width() - 1) * 2.0 - 1.0, 1.0));
        // The two ends belong to the top and bottom rows and only slide
        // sideways.
        if (dragging_ > 0 && dragging_ < points_.size() - 1) {
            point.setY(qBound(points_.at(dragging_ - 1).y(),
                              pos.y() / (height() - 1),
                              points_.at(dragging_ + 1).y()));
        }
        dirty_ = true;
        update();
        if (changedLive) {
            changedLive();
        }
    }

    void mouseReleaseEvent(QMouseEvent* event) override
    {
        // Dragged out of the box: take the point away, unless it is an end,
        // which the curve cannot do without.
        if (dragging_ > 0 && dragging_ < points_.size() - 1
            && !rect().adjusted(-2, -2, 2, 2).contains(event->position().toPoint())) {
            points_.removeAt(dragging_);
            dirty_ = true;
            update();
            if (changedLive) {
                changedLive();
            }
        }
        // The canvas preview is a whole-layer render, so it waits for the
        // release; the dialog's own pane has been following `changedLive`.
        if (dirty_) {
            dirty_ = false;
            if (changed) {
                changed();
            }
        }
        dragging_ = -1;
    }

private:
    QPointF at(int index) const
    {
        const QPointF p = points_.at(index);
        // The offset runs the full width of the box, so -1 is the left edge.
        return QPointF((p.x() + 1.0) / 2.0 * (width() - 1), p.y() * (height() - 1));
    }

    // Smooth offset at the normalized height `y` (0 top, 1 bottom).
    //
    // A cubic Hermite through the control points with Catmull-Rom tangents,
    // so the line bends rather than kinks where the points are, the way CS6's
    // curve does. Two points make both tangents the secant, so the line
    // between them is straight.
    double offsetAt(double y) const
    {
        const double first = points_.first().x();
        const double last = points_.last().x();
        if (y <= 0.0) {
            return first;
        }
        if (y >= 1.0) {
            return last;
        }
        int i = 0;
        while (i + 2 < points_.size() && y > points_.at(i + 1).y()) {
            ++i;
        }
        const int before = qMax(0, i - 1);
        const int after = qMin(points_.size() - 1, i + 2);
        const double p1 = points_.at(i).y();
        const double o1 = points_.at(i).x();
        const double p2 = points_.at(i + 1).y();
        const double o2 = points_.at(i + 1).x();
        const double p0 = points_.at(before).y();
        const double o0 = points_.at(before).x();
        const double p3 = points_.at(after).y();
        const double o3 = points_.at(after).x();
        const double span = p2 - p1;
        const double secant = span > 0.0 ? (o2 - o1) / span : 0.0;
        const double slope1 = i == 0 ? secant : (o2 - o0) / (p2 - p0);
        const double slope2 = i + 2 >= points_.size() ? secant : (o3 - o1) / (p3 - p1);
        const double t = span > 0.0 ? (y - p1) / span : 0.0;
        const double t2 = t * t;
        const double t3 = t2 * t;
        return (2.0 * t3 - 3.0 * t2 + 1.0) * o1 + (t3 - 2.0 * t2 + t) * span * slope1
            + (-2.0 * t3 + 3.0 * t2) * o2 + (t3 - t2) * span * slope2;
    }

    QList<QPointF> points_{{0.0, 0.0}, {0.0, 1.0}};
    int dragging_ = -1;
    bool dirty_ = false;
};

} // namespace

FilterParamControls::FilterParamControls(const QList<FilterParamSpec>& params, QWidget* parent,
                                         const QImage& padImage)
    : QObject(parent), parent_(parent), padImage_(padImage)
{
    for (const FilterParamSpec& spec : params) {
        addControl(spec);
    }

    // Radial Blur: the Blur Method choice flips the display-only centre pad.
    BlurCenterWidget* blurCenter = nullptr;
    QComboBox* blurMethod = nullptr;
    for (const Control& control : controls_) {
        if (control.spec.control == FilterControl::BlurCenter) {
            blurCenter = static_cast<BlurCenterWidget*>(control.center);
        } else if (control.spec.control == FilterControl::Choice
                   && control.spec.label == QStringLiteral("Blur Method:")) {
            blurMethod = control.combo;
        }
    }
    if (blurCenter && blurMethod) {
        connect(blurMethod, &QComboBox::currentIndexChanged, blurCenter,
                [blurCenter](int index) { blurCenter->setZoom(index == 1); });
        blurCenter->setZoom(blurMethod->currentIndex() == 1);
    }
}

QList<QWidget*> FilterParamControls::rows() const
{
    QList<QWidget*> result;
    for (const Control& control : controls_) {
        result.append(control.row);
    }
    return result;
}

void FilterParamControls::addControl(const FilterParamSpec& spec)
{
    const QList<double>& initial = spec.initial;
    Control control;
    control.spec = spec;

    const auto fallback = [&initial](int i, double d) {
        return i < initial.size() ? initial.at(i) : d;
    };

    switch (spec.control) {
    case FilterControl::CheckBox: {
        control.box = new QCheckBox(spec.label, parent_);
        control.box->setChecked(fallback(0, spec.value) >= 0.5);
        connect(control.box, &QCheckBox::toggled, this, [this] { emit changed(); });
        break;
    }
    case FilterControl::Choice: {
        control.combo = new QComboBox(parent_);
        control.combo->addItems(spec.choices);
        control.combo->setCurrentIndex(static_cast<int>(fallback(0, spec.value)));
        connect(control.combo, &QComboBox::currentIndexChanged, this,
                [this] { emit changed(); });
        break;
    }
    case FilterControl::Radio: {
        control.group = new QButtonGroup(parent_);
        for (int i = 0; i < spec.choices.size(); ++i) {
            auto* button = new QRadioButton(spec.choices.at(i), parent_);
            control.group->addButton(button, i);
        }
        const int initial = static_cast<int>(fallback(0, spec.value));
        if (QAbstractButton* button = control.group->button(initial)) {
            button->setChecked(true);
        }
        connect(control.group, &QButtonGroup::idToggled, this, [this](int, bool on) {
            if (on) {
                emit changed();
            }
        });
        break;
    }
    case FilterControl::Color: {
        control.colorButton = new QPushButton(parent_);
        control.colorButton->setObjectName(QStringLiteral("filterColorSwatch"));
        control.colorButton->setFixedSize(24, 22);
        const QColor color(static_cast<int>(fallback(0, spec.initial.value(0, 0))),
                           static_cast<int>(fallback(1, spec.initial.value(1, 0))),
                           static_cast<int>(fallback(2, spec.initial.value(2, 0))));
        setButtonColor(control.colorButton, color);
        connect(control.colorButton, &QPushButton::clicked, this, [this, button = control.colorButton,
                                                                    spec] {
            QString title = spec.label;
            title.remove(QLatin1Char(':'));
            const QColor picked = ColorPickerDialog::getColor(buttonColor(button), parent_, title);
            if (picked.isValid()) {
                setButtonColor(button, picked);
                emit changed();
            }
        });
        break;
    }
    case FilterControl::Placement: {
        control.x = new QDoubleSpinBox(parent_);
        control.y = new QDoubleSpinBox(parent_);
        for (QDoubleSpinBox* spin : {control.x, control.y}) {
            spin->setRange(spec.minimum, spec.maximum);
            spin->setDecimals(3);
            spin->setSingleStep(0.01);
            spin->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
            spin->setFixedWidth(compactSpinWidth(spin));
        }
        control.x->setValue(fallback(0, spec.initial.value(0, 0.5)));
        control.y->setValue(fallback(1, spec.initial.value(1, 0.5)));
        auto* pad = new PlacementPad(parent_);
        pad->setImage(padImage_);
        pad->setPosition(control.x->value(), control.y->value());
        control.pad = pad;
        connect(control.x, &QDoubleSpinBox::valueChanged, this,
                [this, pad, y = control.y](double value) {
                    pad->setPosition(value, y->value());
                    emit changed();
                });
        connect(control.y, &QDoubleSpinBox::valueChanged, this,
                [this, pad, x = control.x](double value) {
                    pad->setPosition(x->value(), value);
                    emit changed();
                });
        pad->moved = [this, x = control.x, y = control.y](double px, double py) {
            QSignalBlocker blockX(x);
            QSignalBlocker blockY(y);
            x->setValue(px);
            y->setValue(py);
            emit changed();
        };
        break;
    }
    case FilterControl::BlurCenter: {
        control.center = new BlurCenterWidget(parent_);
        break;
    }
    case FilterControl::Hidden:
        control.hidden = fallback(0, spec.value);
        break;
    case FilterControl::ShearCurve: {
        auto* curve = new ShearCurveWidget(parent_);
        curve->changed = [this] { emit changed(); };
        curve->changedLive = [this] { emit changedLive(); };
        control.shear = curve;
        break;
    }
    default: {
        control.spin = new QDoubleSpinBox(parent_);
        control.spin->setRange(spec.minimum, spec.maximum);
        control.spin->setDecimals(spec.decimals);
        control.spin->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        control.spin->setFixedWidth(compactSpinWidth(control.spin));
        control.spin->setValue(fallback(0, spec.value));
        control.slider = new JumpSlider(Qt::Horizontal, parent_);
        control.slider->setRange(0, 1000);
        const double span = spec.maximum - spec.minimum;
        const double frac = span > 0.0 ? (control.spin->value() - spec.minimum) / span : 0.0;
        control.slider->setValue(static_cast<int>(frac * 1000.0));
        connect(control.slider, &QSlider::sliderPressed, this,
                [this] { sliderDragging_ = true; });
        connect(control.slider, &QSlider::sliderReleased, this, [this] {
            sliderDragging_ = false;
            emit changed();
        });
        // Dragging moves the number but holds the canvas preview until release.
        connect(control.slider, &QSlider::valueChanged, this,
                [this, spin = control.spin, spec](int value) {
                    const double f = static_cast<double>(value) / 1000.0;
                    QSignalBlocker block(spin);
                    spin->setValue(spec.minimum + f * (spec.maximum - spec.minimum));
                    if (!sliderDragging_) {
                        emit changed();
                    }
                });
        connect(control.spin, &QDoubleSpinBox::valueChanged, this, [this, slider = control.slider,
                                                                    spec](double value) {
            const double span = spec.maximum - spec.minimum;
            const double f = span > 0.0 ? (value - spec.minimum) / span : 0.0;
            QSignalBlocker block(slider);
            slider->setValue(static_cast<int>(f * 1000.0));
            emit changed();
        });
        break;
    }
    }

    // Each parameter is a vertical row: its label and value on one line, and
    // (for a slider) the track beneath, starting flush with the label.
    control.row = new QWidget(parent_);
    auto* rowLayout = new QVBoxLayout(control.row);
    rowLayout->setContentsMargins(0, 0, 0, 0);
    rowLayout->setSpacing(2);
    switch (spec.control) {
    case FilterControl::CheckBox:
        rowLayout->addWidget(control.box);
        break;
    case FilterControl::Choice: {
        auto* line = new QHBoxLayout;
        line->addWidget(new QLabel(spec.label, control.row));
        line->addStretch(1);
        line->addWidget(control.combo);
        rowLayout->addLayout(line);
        break;
    }
    case FilterControl::Color: {
        auto* line = new QHBoxLayout;
        line->addWidget(new QLabel(spec.label, control.row));
        line->addStretch(1);
        line->addWidget(control.colorButton);
        rowLayout->addLayout(line);
        break;
    }
    case FilterControl::Placement: {
        rowLayout->addWidget(new QLabel(spec.label, control.row));
        auto* line = new QHBoxLayout;
        line->addWidget(control.pad, 1);
        line->addWidget(new QLabel(QStringLiteral("X"), control.row));
        line->addWidget(control.x);
        line->addWidget(new QLabel(QStringLiteral("Y"), control.row));
        line->addWidget(control.y);
        rowLayout->addLayout(line);
        break;
    }
    case FilterControl::BlurCenter:
        rowLayout->addWidget(new QLabel(spec.label, control.row));
        rowLayout->addWidget(control.center);
        break;
    case FilterControl::ShearCurve:
        rowLayout->addWidget(control.shear);
        break;
    case FilterControl::Hidden:
        control.row->hide();
        break;
    case FilterControl::Radio: {
        rowLayout->addWidget(new QLabel(spec.label, control.row));
        const QList<QAbstractButton*> buttons = control.group->buttons();
        for (QAbstractButton* button : buttons) {
            rowLayout->addWidget(button);
        }
        break;
    }
    default: {
        auto* line = new QHBoxLayout;
        line->addWidget(new QLabel(spec.label, control.row));
        line->addStretch(1);
        line->addWidget(control.spin);
        const QString unit = spec.suffix.trimmed();
        if (!unit.isEmpty()) {
            auto* unitLabel = new QLabel(unit, control.row);
            unitLabel->setObjectName(QStringLiteral("filterUnit"));
            line->addWidget(unitLabel);
        }
        rowLayout->addLayout(line);
        rowLayout->addWidget(control.slider);
        break;
    }
    }
    controls_.append(control);
}

void FilterParamControls::setValues(const QList<double>& initial)
{
    const QSignalBlocker block(this);
    int slot = 0;
    for (Control& control : controls_) {
        switch (control.spec.control) {
        case FilterControl::CheckBox:
            control.box->setChecked(slot < initial.size() ? initial.at(slot) >= 0.5
                                                          : control.box->isChecked());
            break;
        case FilterControl::Choice:
            control.combo->setCurrentIndex(slot < initial.size()
                                               ? static_cast<int>(initial.at(slot))
                                               : control.combo->currentIndex());
            break;
        case FilterControl::Radio:
            if (slot < initial.size()) {
                if (QAbstractButton* button =
                        control.group->button(static_cast<int>(initial.at(slot)))) {
                    button->setChecked(true);
                }
            }
            break;
        case FilterControl::ShearCurve: {
            const int count = slot < initial.size()
                                  ? qBound(0, static_cast<int>(initial.at(slot)), kShearMaxPoints)
                                  : 0;
            QList<QPointF> points;
            for (int i = 0; i < count; ++i) {
                const double position = slot + 1 + 2 * i < initial.size()
                                            ? initial.at(slot + 1 + 2 * i)
                                            : 0.0;
                const double offset = slot + 2 + 2 * i < initial.size()
                                          ? initial.at(slot + 2 + 2 * i)
                                          : 0.0;
                points.append(QPointF(offset, (position + 1.0) / 2.0));
            }
            static_cast<ShearCurveWidget*>(control.shear)->setPoints(points);
            break;
        }
        case FilterControl::Color: {
            const double r = slot < initial.size() ? initial.at(slot) : buttonColor(control.colorButton).red();
            const double g = slot + 1 < initial.size() ? initial.at(slot + 1) : buttonColor(control.colorButton).green();
            const double b = slot + 2 < initial.size() ? initial.at(slot + 2) : buttonColor(control.colorButton).blue();
            setButtonColor(control.colorButton,
                           QColor(static_cast<int>(r), static_cast<int>(g), static_cast<int>(b)));
            break;
        }
        case FilterControl::Placement:
            if (slot < initial.size()) {
                control.x->setValue(initial.at(slot));
            }
            if (slot + 1 < initial.size()) {
                control.y->setValue(initial.at(slot + 1));
            }
            if (control.pad) {
                static_cast<PlacementPad*>(control.pad)->setPosition(control.x->value(),
                                                                     control.y->value());
            }
            break;
        case FilterControl::BlurCenter:
            break;
        case FilterControl::Hidden:
            if (slot < initial.size()) {
                control.hidden = initial.at(slot);
            }
            break;
        default:
            if (slot < initial.size()) {
                control.spin->setValue(initial.at(slot));
            }
            break;
        }
        slot += slotCount(control.spec.control);
    }
}

double FilterParamControls::controlValue(const Control& control, int slot) const
{
    switch (control.spec.control) {
    case FilterControl::CheckBox:
        return control.box->isChecked() ? 1.0 : 0.0;
    case FilterControl::Choice:
        return static_cast<double>(control.combo->currentIndex());
    case FilterControl::Radio:
        return static_cast<double>(qMax(0, control.group->checkedId()));
    case FilterControl::Color: {
        const QColor color = buttonColor(control.colorButton);
        return slot == 0 ? color.red() : slot == 1 ? color.green() : color.blue();
    }
    case FilterControl::Placement:
        return slot == 0 ? control.x->value() : control.y->value();
    case FilterControl::BlurCenter:
    case FilterControl::ShearCurve:
        return 0.0;
    case FilterControl::Hidden:
        return control.hidden;
    default:
        return control.spin->value();
    }
}

QList<double> FilterParamControls::values() const
{
    QList<double> result;
    for (const Control& control : controls_) {
        if (control.spec.control == FilterControl::ShearCurve) {
            const QList<QPointF>& points =
                static_cast<ShearCurveWidget*>(control.shear)->points();
            result.append(static_cast<double>(points.size()));
            for (const QPointF& p : points) {
                result.append(p.y() * 2.0 - 1.0);
                result.append(p.x());
            }
            for (int i = points.size(); i < kShearMaxPoints; ++i) {
                result.append(0.0);
                result.append(0.0);
            }
            continue;
        }
        const int n = slotCount(control.spec.control);
        for (int s = 0; s < n; ++s) {
            result.append(controlValue(control, s));
        }
    }
    return result;
}


QList<double> FilterParamControls::scaledValues(double scale) const
{
    QList<double> result = values();
    int slot = 0;
    for (const Control& control : controls_) {
        const FilterParamSpec& spec = control.spec;
        if (measuresPixels(spec) && slot < result.size()) {
            double value = result.at(slot) * scale;
            if (spec.decimals == 0) {
                value = std::round(value);
            }
            result[slot] = qBound(spec.minimum, value, spec.maximum);
        }
        slot += slotCount(spec.control);
    }
    return result;
}

} // namespace pictura
