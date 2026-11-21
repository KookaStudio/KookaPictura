// The Gradient Editor's ramp widget: colour stops, opacity stops, and
// midpoint diamonds. Split from gradient_editor_dialog.cpp along that seam.

#include "gradient_editor_dialog.h"

#include "color_picker_dialog.h"

#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>

#include <algorithm>

namespace pictura {

namespace {

constexpr int kMaxLocation = 4096;
constexpr int kStopWidth = 11;
constexpr int kStopHeight = 14;
constexpr int kInset = kStopWidth / 2 + 2;
constexpr int kDragOffDistance = 30;

double bias(double t, int midpointPercent)
{
    const double m = qBound(0.01, midpointPercent / 100.0, 0.99);
    return t <= m ? 0.5 * t / m : 0.5 + 0.5 * (t - m) / (1.0 - m);
}

} // namespace

QColor gradientColorAt(const GradientStops& stops, int location)
{
    if (stops.isEmpty()) {
        return Qt::black;
    }
    if (location <= stops.first().location) {
        return stops.first().color;
    }
    for (int i = 1; i < stops.size(); ++i) {
        const GradientColorStop& a = stops.at(i - 1);
        const GradientColorStop& b = stops.at(i);
        if (location <= b.location) {
            const double t = b.location == a.location
                                 ? 1.0
                                 : bias(double(location - a.location) / (b.location - a.location),
                                        a.midpoint);
            const auto mix = [t](int x, int y) { return qRound(x + (y - x) * t); };
            return QColor(mix(a.color.red(), b.color.red()), mix(a.color.green(), b.color.green()),
                          mix(a.color.blue(), b.color.blue()),
                          mix(a.color.alpha(), b.color.alpha()));
        }
    }
    return stops.last().color;
}

int gradientOpacityAt(const GradientOpacityStops& stops, int location)
{
    if (stops.isEmpty()) {
        return 100;
    }
    if (location <= stops.first().location) {
        return stops.first().opacity;
    }
    for (int i = 1; i < stops.size(); ++i) {
        const GradientOpacityStop& a = stops.at(i - 1);
        const GradientOpacityStop& b = stops.at(i);
        if (location <= b.location) {
            const double t = b.location == a.location
                                 ? 1.0
                                 : bias(double(location - a.location) / (b.location - a.location),
                                        a.midpoint);
            return qRound(a.opacity + (b.opacity - a.opacity) * t);
        }
    }
    return stops.last().opacity;
}

GradientOpacityStops gradientOpacityFromColors(const GradientStops& stops)
{
    if (stops.isEmpty()
        || std::all_of(stops.begin(), stops.end(),
                       [](const GradientColorStop& s) { return s.color.alpha() == 255; })) {
        return {};
    }
    GradientOpacityStops result;
    for (const GradientColorStop& stop : stops) {
        result.push_back(
            {stop.location, qRound(stop.color.alpha() * 100.0 / 255.0), stop.midpoint});
    }
    if (result.first().location > 0) {
        result.prepend({0, result.first().opacity, 50});
    }
    if (result.last().location < kMaxLocation) {
        result.push_back({kMaxLocation, result.last().opacity, 50});
    }
    return result;
}

GradientStopBar::GradientStopBar(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("gradientStopBar"));
    setMinimumHeight(kStopHeight * 2 + 28);
}

QSize GradientStopBar::sizeHint() const
{
    return {380, kStopHeight * 2 + 28};
}

void GradientStopBar::deriveOpacityStops()
{
    opacityStops_ = gradientOpacityFromColors(stops_);
    if (opacityStops_.isEmpty()) {
        opacityStops_ = {{0, 100, 50}, {kMaxLocation, 100, 50}};
    }
}

void GradientStopBar::setStops(const GradientStops& stops)
{
    setGradient(stops, {});
}

void GradientStopBar::setGradient(const GradientStops& stops, const GradientOpacityStops& opacity)
{
    stops_ = stops;
    opacityStops_ = opacity;
    for (GradientColorStop& stop : stops_) {
        stop.midpoint = qBound(1, stop.midpoint, 99);
    }
    for (GradientOpacityStop& stop : opacityStops_) {
        stop.midpoint = qBound(1, stop.midpoint, 99);
    }
    std::stable_sort(stops_.begin(), stops_.end(),
                     [](const auto& a, const auto& b) { return a.location < b.location; });
    std::stable_sort(opacityStops_.begin(), opacityStops_.end(),
                     [](const auto& a, const auto& b) { return a.location < b.location; });
    if (opacityStops_.isEmpty()) {
        deriveOpacityStops();
    }
    selected_ = stops_.isEmpty() ? -1 : qBound(-1, selected_, int(stops_.size()) - 1);
    selectedOpacity_ =
        opacityStops_.isEmpty() ? -1 : qBound(-1, selectedOpacity_, int(opacityStops_.size()) - 1);
    update();
    emit stopsChanged();
    emit opacityStopsChanged();
    emit selectionChanged(selected_);
    emit opacitySelectionChanged(selectedOpacity_);
}

void GradientStopBar::select(int index)
{
    kind_ = StopKind::Color;
    selected_ = qBound(-1, index, int(stops_.size()) - 1);
    update();
    emit selectionChanged(selected_);
}

void GradientStopBar::selectOpacity(int index)
{
    kind_ = StopKind::Opacity;
    selectedOpacity_ = qBound(-1, index, int(opacityStops_.size()) - 1);
    update();
    emit opacitySelectionChanged(selectedOpacity_);
}

void GradientStopBar::setStopColor(int index, const QColor& color)
{
    if (index < 0 || index >= stops_.size()) {
        return;
    }
    stops_[index].color = color;
    update();
    emit stopsChanged();
}

void GradientStopBar::setStopLocation(int index, int location)
{
    if (index < 0 || index >= stops_.size()) {
        return;
    }
    location = qBound(0, location, kMaxLocation);
    if (strict_) {
        const int lower = index > 0 ? stops_.at(index - 1).location + 1 : 0;
        const int upper =
            index + 1 < stops_.size() ? stops_.at(index + 1).location - 1 : kMaxLocation;
        location = qBound(lower, location, qMax(lower, upper));
    }
    stops_[index].location = location;
    kind_ = StopKind::Color;
    selected_ = index;
    sortKeepingSelection();
    update();
    emit stopsChanged();
}

void GradientStopBar::setStopMidpoint(int index, int percent)
{
    if (index < 0 || index >= stops_.size()) {
        return;
    }
    stops_[index].midpoint = qBound(1, percent, 99);
    kind_ = StopKind::Color;
    selected_ = index;
    update();
    emit stopsChanged();
}

void GradientStopBar::setOpacityStopValue(int index, int percent)
{
    if (index < 0 || index >= opacityStops_.size()) {
        return;
    }
    opacityStops_[index].opacity = qBound(0, percent, 100);
    kind_ = StopKind::Opacity;
    selectedOpacity_ = index;
    update();
    emit opacityStopsChanged();
}

void GradientStopBar::setOpacityStopLocation(int index, int location)
{
    if (index < 0 || index >= opacityStops_.size()) {
        return;
    }
    opacityStops_[index].location = qBound(0, location, kMaxLocation);
    kind_ = StopKind::Opacity;
    selectedOpacity_ = index;
    sortKeepingOpacitySelection();
    update();
    emit opacityStopsChanged();
}

void GradientStopBar::setOpacityStopMidpoint(int index, int percent)
{
    if (index < 0 || index >= opacityStops_.size()) {
        return;
    }
    opacityStops_[index].midpoint = qBound(1, percent, 99);
    kind_ = StopKind::Opacity;
    selectedOpacity_ = index;
    update();
    emit opacityStopsChanged();
}

void GradientStopBar::removeStop(int index)
{
    if (index < 0 || index >= stops_.size() || stops_.size() <= 2) {
        return;
    }
    stops_.remove(index);
    kind_ = StopKind::Color;
    selected_ = qMin(index, int(stops_.size()) - 1);
    update();
    emit stopsChanged();
    emit selectionChanged(selected_);
}

void GradientStopBar::removeOpacityStop(int index)
{
    if (index < 0 || index >= opacityStops_.size() || opacityStops_.size() <= 2) {
        return;
    }
    opacityStops_.remove(index);
    kind_ = StopKind::Opacity;
    selectedOpacity_ = qMin(index, int(opacityStops_.size()) - 1);
    update();
    emit opacityStopsChanged();
    emit opacitySelectionChanged(selectedOpacity_);
}

int GradientStopBar::addStop(int location)
{
    const int requested = qBound(0, location, kMaxLocation);
    location = requested;
    if (strict_) {
        const auto taken = [this](int value) {
            return std::any_of(stops_.begin(), stops_.end(), [value](const GradientColorStop& stop) {
                return stop.location == value;
            });
        };
        if (taken(location)) {
            bool found = false;
            for (int offset = 1; offset <= kMaxLocation && !found; ++offset) {
                const int up = requested + offset;
                const int down = requested - offset;
                if (up <= kMaxLocation && !taken(up)) {
                    location = up;
                    found = true;
                } else if (down >= 0 && !taken(down)) {
                    location = down;
                    found = true;
                }
            }
            if (!found) {
                return -1;
            }
        }
    }
    stops_.append({location, gradientColorAt(stops_, location), 50});
    kind_ = StopKind::Color;
    selected_ = int(stops_.size()) - 1;
    sortKeepingSelection();
    update();
    emit stopsChanged();
    emit selectionChanged(selected_);
    return selected_;
}

int GradientStopBar::addOpacityStop(int location)
{
    location = qBound(0, location, kMaxLocation);
    if (opacityStops_.isEmpty()) {
        deriveOpacityStops();
    }
    opacityStops_.append({location, gradientOpacityAt(opacityStops_, location), 50});
    kind_ = StopKind::Opacity;
    selectedOpacity_ = int(opacityStops_.size()) - 1;
    sortKeepingOpacitySelection();
    update();
    emit opacityStopsChanged();
    emit opacitySelectionChanged(selectedOpacity_);
    return selectedOpacity_;
}

void GradientStopBar::sortKeepingSelection()
{
    if (selected_ < 0) {
        return;
    }
    const GradientColorStop moved = stops_.at(selected_);
    stops_.remove(selected_);
    const auto at = std::upper_bound(
        stops_.begin(), stops_.end(), moved.location,
        [](int location, const GradientColorStop& stop) { return location < stop.location; });
    selected_ = int(at - stops_.begin());
    stops_.insert(selected_, moved);
}

void GradientStopBar::sortKeepingOpacitySelection()
{
    if (selectedOpacity_ < 0) {
        return;
    }
    const GradientOpacityStop moved = opacityStops_.at(selectedOpacity_);
    opacityStops_.remove(selectedOpacity_);
    const auto at = std::upper_bound(
        opacityStops_.begin(), opacityStops_.end(), moved.location,
        [](int location, const GradientOpacityStop& stop) { return location < stop.location; });
    selectedOpacity_ = int(at - opacityStops_.begin());
    opacityStops_.insert(selectedOpacity_, moved);
}

QRect GradientStopBar::barRect() const
{
    return QRect(kInset, kStopHeight + 2, width() - 2 * kInset, height() - 2 * kStopHeight - 4);
}

int GradientStopBar::colorStopX(int index) const
{
    const QRect bar = barRect();
    return bar.left() + qRound(double(stops_.at(index).location) * (bar.width() - 1) / kMaxLocation);
}

int GradientStopBar::opacityStopX(int index) const
{
    const QRect bar = barRect();
    return bar.left()
        + qRound(double(opacityStops_.at(index).location) * (bar.width() - 1) / kMaxLocation);
}

int GradientStopBar::colorStopAt(const QPoint& pos) const
{
    const QRect bar = barRect();
    if (pos.y() <= bar.bottom()) {
        return -1;
    }
    // The selected stop wins a tie, then the nearest.
    int best = -1;
    int bestDistance = kStopWidth / 2 + 1;
    for (int i = 0; i < stops_.size(); ++i) {
        const int distance = std::abs(colorStopX(i) - pos.x());
        if (distance < bestDistance || (distance == bestDistance && i == selected_)) {
            best = i;
            bestDistance = distance;
        }
    }
    return best;
}

int GradientStopBar::opacityStopAt(const QPoint& pos) const
{
    const QRect bar = barRect();
    if (pos.y() >= bar.top()) {
        return -1;
    }
    int best = -1;
    int bestDistance = kStopWidth / 2 + 1;
    for (int i = 0; i < opacityStops_.size(); ++i) {
        const int distance = std::abs(opacityStopX(i) - pos.x());
        if (distance < bestDistance || (distance == bestDistance && i == selectedOpacity_)) {
            best = i;
            bestDistance = distance;
        }
    }
    return best;
}

QPair<GradientStopBar::StopKind, int> GradientStopBar::midpointAt(const QPoint& pos) const
{
    const QRect bar = barRect();
    if (pos.y() < bar.top() || pos.y() > bar.bottom()) {
        return {StopKind::Color, -1};
    }
    const auto near = [&pos](int x, int y) {
        return std::abs(pos.x() - x) <= 6 && std::abs(pos.y() - y) <= 6;
    };
    for (int i = 0; i + 1 < stops_.size(); ++i) {
        const int x = colorStopX(i)
            + qRound(double(colorStopX(i + 1) - colorStopX(i)) * stops_.at(i).midpoint / 100.0);
        if (near(x, bar.bottom() - 6)) {
            return {StopKind::Color, i};
        }
    }
    for (int i = 0; i + 1 < opacityStops_.size(); ++i) {
        const int x = opacityStopX(i)
            + qRound(double(opacityStopX(i + 1) - opacityStopX(i)) * opacityStops_.at(i).midpoint
                     / 100.0);
        if (near(x, bar.top() + 6)) {
            return {StopKind::Opacity, i};
        }
    }
    return {StopKind::Color, -1};
}

int GradientStopBar::locationAt(int x) const
{
    const QRect bar = barRect();
    return qBound(0, qRound(double(x - bar.left()) * kMaxLocation / (bar.width() - 1)),
                  kMaxLocation);
}

void GradientStopBar::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    const QRect bar = barRect();
    GradientSpec spec;
    spec.stops = stops_;
    spec.opacityStops = opacityStops_;
    paintGradient(painter, bar, bakedSolidStops(spec));
    painter.setPen(palette().color(QPalette::Dark));
    painter.drawRect(bar.adjusted(0, 0, -1, -1));
    painter.setRenderHint(QPainter::Antialiasing);

    const auto marker = [&painter](int x, int tipY, int direction, const QColor& fill,
                                   bool selected) {
        // A house shape: the point at the bar, the swatch away from it.
        const int baseY = tipY + direction * kStopHeight;
        const int shoulderY = tipY + direction * 4;
        QPainterPath path;
        path.moveTo(x + 0.5, tipY + 0.5);
        path.lineTo(x + kStopWidth / 2 + 0.5, shoulderY + 0.5);
        path.lineTo(x + kStopWidth / 2 + 0.5, baseY + 0.5);
        path.lineTo(x - kStopWidth / 2 + 0.5, baseY + 0.5);
        path.lineTo(x - kStopWidth / 2 + 0.5, shoulderY + 0.5);
        path.closeSubpath();
        painter.setPen(QPen(selected ? QColor(Qt::black) : QColor(90, 90, 90), 1));
        painter.setBrush(selected ? QColor(Qt::black) : QColor(240, 240, 240));
        painter.drawPath(path);
        const QRectF swatch(x - kStopWidth / 2 + 2.5, std::min(shoulderY, baseY) + 2.5,
                            kStopWidth - 4, std::abs(baseY - shoulderY) - 4);
        painter.setPen(Qt::NoPen);
        QColor opaque = fill;
        opaque.setAlpha(255);
        painter.setBrush(opaque);
        painter.drawRect(swatch);
    };
    const auto diamond = [&painter](int x, int y, bool selected) {
        QPainterPath path;
        path.moveTo(x + 0.5, y - 5.5);
        path.lineTo(x + 5.5, y + 0.5);
        path.lineTo(x + 0.5, y + 5.5);
        path.lineTo(x - 4.5, y + 0.5);
        path.closeSubpath();
        painter.setPen(QPen(QColor(40, 40, 40), 1));
        painter.setBrush(selected ? QColor(Qt::black) : QColor(Qt::white));
        painter.drawPath(path);
    };
    for (int i = 0; i + 1 < stops_.size(); ++i) {
        const int x = colorStopX(i)
            + qRound(double(colorStopX(i + 1) - colorStopX(i)) * stops_.at(i).midpoint / 100.0);
        diamond(x, bar.bottom() - 6,
                kind_ == StopKind::Color && selected_ == i && dragging_ == DragKind::ColorMidpoint);
    }
    for (int i = 0; i + 1 < opacityStops_.size(); ++i) {
        const int x = opacityStopX(i)
            + qRound(double(opacityStopX(i + 1) - opacityStopX(i)) * opacityStops_.at(i).midpoint
                     / 100.0);
        diamond(x, bar.top() + 6,
                kind_ == StopKind::Opacity && selectedOpacity_ == i
                    && dragging_ == DragKind::OpacityMidpoint);
    }
    for (int i = 0; i < opacityStops_.size(); ++i) {
        if (dragOff_ && dragging_ == DragKind::Opacity && i == selectedOpacity_) {
            continue;
        }
        const int v = qRound(opacityStops_.at(i).opacity * 255.0 / 100.0);
        marker(opacityStopX(i), bar.top() - 1, -1, QColor(v, v, v),
               kind_ == StopKind::Opacity && i == selectedOpacity_);
    }
    for (int i = 0; i < stops_.size(); ++i) {
        if (dragOff_ && dragging_ == DragKind::Color && i == selected_) {
            continue;
        }
        marker(colorStopX(i), bar.bottom() + 1, 1, stops_.at(i).color.toRgb(),
               kind_ == StopKind::Color && i == selected_);
    }
}

void GradientStopBar::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton) {
        return;
    }
    const QPoint pos = event->position().toPoint();
    const QPair<StopKind, int> midpoint = midpointAt(pos);
    if (midpoint.second >= 0) {
        kind_ = midpoint.first;
        if (kind_ == StopKind::Color) {
            selected_ = midpoint.second;
            dragging_ = DragKind::ColorMidpoint;
            emit selectionChanged(selected_);
        } else {
            selectedOpacity_ = midpoint.second;
            dragging_ = DragKind::OpacityMidpoint;
            emit opacitySelectionChanged(selectedOpacity_);
        }
    } else if (pos.y() < barRect().top()) {
        const int hit = opacityStopAt(pos);
        if (hit >= 0) {
            selectOpacity(hit);
        } else {
            addOpacityStop(locationAt(pos.x()));
        }
        dragging_ = DragKind::Opacity;
    } else if (pos.y() > barRect().bottom()) {
        const int hit = colorStopAt(pos);
        if (hit >= 0) {
            select(hit);
        } else {
            addStop(locationAt(pos.x()));
        }
        dragging_ = DragKind::Color;
    }
    dragOff_ = false;
    update();
}

void GradientStopBar::mouseMoveEvent(QMouseEvent* event)
{
    const QPoint pos = event->position().toPoint();
    const QRect bar = barRect();
    if (dragging_ == DragKind::ColorMidpoint || dragging_ == DragKind::OpacityMidpoint) {
        const bool color = dragging_ == DragKind::ColorMidpoint;
        const int left = color ? (selected_ >= 0 ? stops_.at(selected_).location : -1)
                               : (selectedOpacity_ >= 0 ? opacityStops_.at(selectedOpacity_).location
                                                        : -1);
        const int right = color
            ? (selected_ + 1 < stops_.size() ? stops_.at(selected_ + 1).location : -1)
            : (selectedOpacity_ + 1 < opacityStops_.size()
                   ? opacityStops_.at(selectedOpacity_ + 1).location
                   : -1);
        if (left < 0 || right <= left) {
            return;
        }
        const int percent =
            qBound(1, qRound(double(locationAt(pos.x()) - left) * 100.0 / (right - left)), 99);
        if (color) {
            setStopMidpoint(selected_, percent);
        } else {
            setOpacityStopMidpoint(selectedOpacity_, percent);
        }
        return;
    }
    if (dragging_ == DragKind::Color) {
        if (selected_ < 0) {
            return;
        }
        dragOff_ = stops_.size() > 2 && pos.y() > bar.bottom() + kStopHeight + kDragOffDistance;
        setStopLocation(selected_, locationAt(pos.x()));
    } else if (dragging_ == DragKind::Opacity) {
        if (selectedOpacity_ < 0) {
            return;
        }
        dragOff_ =
            opacityStops_.size() > 2 && pos.y() < bar.top() - kStopHeight - kDragOffDistance;
        setOpacityStopLocation(selectedOpacity_, locationAt(pos.x()));
    }
}

void GradientStopBar::mouseReleaseEvent(QMouseEvent*)
{
    if (dragging_ == DragKind::Color && dragOff_) {
        dragOff_ = false;
        removeStop(selected_);
    } else if (dragging_ == DragKind::Opacity && dragOff_) {
        dragOff_ = false;
        removeOpacityStop(selectedOpacity_);
    }
    dragging_ = DragKind::None;
    dragOff_ = false;
    update();
}

void GradientStopBar::mouseDoubleClickEvent(QMouseEvent* event)
{
    const int hit = colorStopAt(event->position().toPoint());
    if (hit < 0) {
        return;
    }
    const QColor picked =
        ColorPickerDialog::getColor(stops_.at(hit).color, this, QStringLiteral("Select stop color:"));
    if (picked.isValid()) {
        setStopColor(hit, picked);
    }
}


} // namespace pictura
