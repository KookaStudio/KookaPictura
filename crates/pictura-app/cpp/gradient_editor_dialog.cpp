#include "gradient_editor_dialog.h"

#include "color_picker_dialog.h"
#include "panels/color_swatch_widget.h"
#include "panels/percent_field.h"

#include "pictura_app/src/cxxqt_object/image_adjust/gradient_map.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

constexpr int kMaxLocation = 4096;
constexpr int kStopWidth = 11;
constexpr int kStopHeight = 14;
constexpr int kInset = kStopWidth / 2 + 2;
// How far below the bar a dragged stop must go to be deleted.
constexpr int kDragOffDistance = 30;

QColor colorAt(const GradientStops& stops, int location)
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
                                 : double(location - a.location) / (b.location - a.location);
            const auto mix = [t](int x, int y) { return qRound(x + (y - x) * t); };
            return QColor(mix(a.color.red(), b.color.red()), mix(a.color.green(), b.color.green()),
                          mix(a.color.blue(), b.color.blue()),
                          mix(a.color.alpha(), b.color.alpha()));
        }
    }
    return stops.last().color;
}

} // namespace

QString formatGradientStops(const GradientStops& stops)
{
    QStringList items;
    for (const GradientColorStop& stop : stops) {
        const QColor& c = stop.color;
        QString hex = QStringLiteral("%1").arg(c.rgb() & 0xffffffu, 6, 16, QLatin1Char('0'));
        if (c.alpha() < 255) {
            hex += QStringLiteral("%1").arg(c.alpha(), 2, 16, QLatin1Char('0'));
        }
        items << QStringLiteral("%1:%2").arg(stop.location).arg(hex);
    }
    return items.join(QLatin1Char(' '));
}

GradientStops parseGradientStops(const QString& text)
{
    GradientStops stops;
    for (const QString& item : text.split(QLatin1Char(' '), Qt::SkipEmptyParts)) {
        const QStringList parts = item.split(QLatin1Char(':'));
        const QString hex = parts.value(1);
        bool locationOk = false;
        bool colorOk = false;
        const int location = parts.value(0).toInt(&locationOk);
        const uint value = hex.toUInt(&colorOk, 16);
        if (parts.size() != 2 || !locationOk || !colorOk
            || (hex.size() != 6 && hex.size() != 8)) {
            return {};
        }
        QColor color = QColor::fromRgb(hex.size() == 8 ? value >> 8 : value);
        if (hex.size() == 8) {
            color.setAlpha(int(value & 0xffu));
        }
        stops.append({location, color});
    }
    return stops;
}

void paintGradient(QPainter& painter, const QRect& rect, const GradientStops& stops)
{
    const bool clear = std::any_of(stops.begin(), stops.end(),
                                   [](const GradientColorStop& s) { return s.color.alpha() < 255; });
    if (clear) {
        constexpr int kCell = 4;
        painter.fillRect(rect, Qt::white);
        for (int y = rect.top(); y <= rect.bottom(); y += kCell) {
            for (int x = rect.left() + ((y - rect.top()) / kCell % 2) * kCell; x <= rect.right();
                 x += 2 * kCell) {
                painter.fillRect(QRect(x, y, kCell, kCell).intersected(rect), QColor(204, 204, 204));
            }
        }
    }
    QLinearGradient ramp(rect.left(), 0, rect.right() + 1, 0);
    for (const GradientColorStop& stop : stops) {
        ramp.setColorAt(qBound(0.0, double(stop.location) / kMaxLocation, 1.0), stop.color);
    }
    painter.fillRect(rect, ramp);
}

GradientStops gradientPreset(int index, const QColor& foreground, const QColor& background)
{
    return parseGradientStops(
        QString(gradient_preset_map_stops(index, foreground.rgba(), background.rgba())));
}

GradientStops noiseStops(const NoiseSettings& noise)
{
    return parseGradientStops(QString(gradient_noise_stops(
        noise.seed, noise.roughness, noise.model, noise.low[0], noise.high[0], noise.low[1],
        noise.high[1], noise.low[2], noise.high[2], noise.restrictColors, noise.transparency)));
}

GradientStops resolvedStops(const GradientSpec& spec)
{
    if (spec.type == GradientSpec::Noise) {
        return noiseStops(spec.noise);
    }
    return parseGradientStops(
        QString(gradient_smoothed_stops(formatGradientStops(spec.stops), spec.smoothness)));
}

// --- GradientSwatch ---------------------------------------------------------

GradientSwatch::GradientSwatch(QWidget* parent)
    : QWidget(parent)
{
    setMinimumHeight(24);
}

void GradientSwatch::setStops(const GradientStops& stops)
{
    stops_ = stops;
    update();
}

QSize GradientSwatch::sizeHint() const
{
    return {380, 26};
}

void GradientSwatch::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    const QRect r = rect().adjusted(0, 0, -1, -1);
    paintGradient(painter, r, stops_);
    painter.setPen(palette().color(QPalette::Dark));
    painter.drawRect(r);
}

// --- GradientStopBar --------------------------------------------------------

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

void GradientStopBar::setStops(const GradientStops& stops)
{
    stops_ = stops;
    std::stable_sort(stops_.begin(), stops_.end(),
                     [](const auto& a, const auto& b) { return a.location < b.location; });
    selected_ = stops_.isEmpty() ? -1 : qBound(-1, selected_, int(stops_.size()) - 1);
    update();
    emit selectionChanged(selected_);
}

void GradientStopBar::select(int index)
{
    selected_ = qBound(-1, index, int(stops_.size()) - 1);
    update();
    emit selectionChanged(selected_);
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
    stops_[index].location = qBound(0, location, kMaxLocation);
    selected_ = index;
    sortKeepingSelection();
    update();
    emit stopsChanged();
}

void GradientStopBar::removeStop(int index)
{
    if (index < 0 || index >= stops_.size() || stops_.size() <= 2) {
        return;
    }
    stops_.remove(index);
    selected_ = qMin(index, int(stops_.size()) - 1);
    update();
    emit stopsChanged();
    emit selectionChanged(selected_);
}

int GradientStopBar::addStop(int location)
{
    location = qBound(0, location, kMaxLocation);
    stops_.append({location, colorAt(stops_, location)});
    selected_ = int(stops_.size()) - 1;
    sortKeepingSelection();
    update();
    emit stopsChanged();
    emit selectionChanged(selected_);
    return selected_;
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

QRect GradientStopBar::barRect() const
{
    return QRect(kInset, kStopHeight + 2, width() - 2 * kInset, height() - 2 * kStopHeight - 4);
}

int GradientStopBar::stopX(int index) const
{
    const QRect bar = barRect();
    return bar.left() + qRound(double(stops_.at(index).location) * (bar.width() - 1) / kMaxLocation);
}

int GradientStopBar::stopAt(const QPoint& pos) const
{
    const QRect bar = barRect();
    if (pos.y() <= bar.bottom()) {
        return -1;
    }
    // The selected stop wins a tie, then the nearest.
    int best = -1;
    int bestDistance = kStopWidth / 2 + 1;
    for (int i = 0; i < stops_.size(); ++i) {
        const int distance = std::abs(stopX(i) - pos.x());
        if (distance < bestDistance || (distance == bestDistance && i == selected_)) {
            best = i;
            bestDistance = distance;
        }
    }
    return best;
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
    paintGradient(painter, bar, stops_);
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
    // The opacity stops: CS6 draws one at each end of a gradient map's ramp.
    marker(bar.left(), bar.top() - 1, -1, Qt::black, false);
    marker(bar.right(), bar.top() - 1, -1, Qt::black, false);
    for (int i = 0; i < stops_.size(); ++i) {
        if (dragOff_ && i == selected_) {
            continue;
        }
        marker(stopX(i), bar.bottom() + 1, 1, stops_.at(i).color.toRgb(), i == selected_);
    }
}

void GradientStopBar::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton) {
        return;
    }
    const QPoint pos = event->position().toPoint();
    const int hit = stopAt(pos);
    if (hit >= 0) {
        select(hit);
        dragging_ = true;
    } else if (pos.y() > barRect().bottom()) {
        addStop(locationAt(pos.x()));
        dragging_ = true;
    }
    dragOff_ = false;
}

void GradientStopBar::mouseMoveEvent(QMouseEvent* event)
{
    if (!dragging_ || selected_ < 0) {
        return;
    }
    const QPoint pos = event->position().toPoint();
    dragOff_ = stops_.size() > 2 && pos.y() > barRect().bottom() + kStopHeight + kDragOffDistance;
    setStopLocation(selected_, locationAt(pos.x()));
}

void GradientStopBar::mouseReleaseEvent(QMouseEvent*)
{
    if (dragging_ && dragOff_) {
        dragOff_ = false;
        removeStop(selected_);
    }
    dragging_ = false;
    dragOff_ = false;
    update();
}

void GradientStopBar::mouseDoubleClickEvent(QMouseEvent* event)
{
    const int hit = stopAt(event->position().toPoint());
    if (hit < 0) {
        return;
    }
    const QColor picked =
        ColorPickerDialog::getColor(stops_.at(hit).color, this, QStringLiteral("Select stop color:"));
    if (picked.isValid()) {
        setStopColor(hit, picked);
    }
}

// --- GradientEditorDialog ---------------------------------------------------

GradientEditorDialog::GradientEditorDialog(const GradientSpec& spec, const QColor& foreground,
                                           const QColor& background, QWidget* parent)
    : QDialog(parent)
    , foreground_(foreground)
    , background_(background)
{
    setObjectName(QStringLiteral("gradientEditor"));
    setWindowTitle(QStringLiteral("Gradient Editor"));
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(14);
    auto* left = new QVBoxLayout;
    left->setSpacing(8);

    auto* presetsBox = new QGroupBox(QStringLiteral("Presets"), this);
    auto* presetsGrid = new QGridLayout(presetsBox);
    presetsGrid->setSpacing(4);
    constexpr int kColumns = 10;
    constexpr int kSwatch = 30;
    for (int i = 0; i < gradient_preset_count(); ++i) {
        QPixmap pixmap(kSwatch, kSwatch);
        {
            QPainter painter(&pixmap);
            paintGradient(painter, pixmap.rect(), gradientPreset(i, foreground, background));
        }
        auto* swatch = new QToolButton(presetsBox);
        swatch->setObjectName(QStringLiteral("gradientPreset%1").arg(i));
        swatch->setIcon(QIcon(pixmap));
        swatch->setIconSize(QSize(kSwatch, kSwatch));
        swatch->setToolTip(QString(gradient_preset_name(i)));
        swatch->setAutoRaise(true);
        presetsGrid->addWidget(swatch, i / kColumns, i % kColumns);
        connect(swatch, &QToolButton::clicked, this, [this, i]() { choosePreset(i); });
    }
    presetsGrid->setColumnStretch(kColumns, 1);
    left->addWidget(presetsBox);

    auto* nameRow = new QHBoxLayout;
    nameRow->addWidget(new QLabel(QStringLiteral("Name:"), this));
    name_ = new QLineEdit(spec.name, this);
    name_->setObjectName(QStringLiteral("gradientName"));
    nameRow->addWidget(name_, 1);
    left->addLayout(nameRow);

    auto* rampBox = new QGroupBox(this);
    auto* ramp = new QVBoxLayout(rampBox);
    auto* typeRow = new QHBoxLayout;
    typeRow->addWidget(new QLabel(QStringLiteral("Gradient Type:"), this));
    type_ = new QComboBox(this);
    type_->setObjectName(QStringLiteral("gradientType"));
    type_->addItems({QStringLiteral("Solid"), QStringLiteral("Noise")});
    typeRow->addWidget(type_);
    typeRow->addStretch(1);
    ramp->addLayout(typeRow);
    pages_ = new QStackedWidget(this);
    ramp->addWidget(pages_);

    auto* solid = new QWidget(pages_);
    auto* solidColumn = new QVBoxLayout(solid);
    solidColumn->setContentsMargins(0, 0, 0, 0);
    smoothness_ = new PercentField(QStringLiteral("Smoothness:"), solid);
    smoothness_->setObjectName(QStringLiteral("gradientSmoothness"));
    auto* smoothRow = new QHBoxLayout;
    smoothRow->addWidget(smoothness_);
    smoothRow->addStretch(1);
    solidColumn->addLayout(smoothRow);
    bar_ = new GradientStopBar(solid);
    solidColumn->addWidget(bar_);

    auto* stopsBox = new QGroupBox(QStringLiteral("Stops"), solid);
    auto* stopsGrid = new QGridLayout(stopsBox);
    const auto locationField = [this]() {
        auto* field = new QSpinBox(this);
        field->setRange(0, 100);
        field->setSuffix(QStringLiteral(" %"));
        field->setFixedWidth(70);
        return field;
    };
    auto* opacity = new QSpinBox(this);
    opacity->setRange(0, 100);
    opacity->setSuffix(QStringLiteral(" %"));
    opacity->setFixedWidth(70);
    QSpinBox* opacityLocation = locationField();
    auto* opacityDelete = new QPushButton(QStringLiteral("Delete"), this);
    for (QWidget* w : {static_cast<QWidget*>(opacity), static_cast<QWidget*>(opacityLocation),
                       static_cast<QWidget*>(opacityDelete)}) {
        w->setEnabled(false);
    }
    stopsGrid->addWidget(new QLabel(QStringLiteral("Opacity:"), this), 0, 0);
    stopsGrid->addWidget(opacity, 0, 1);
    stopsGrid->addWidget(new QLabel(QStringLiteral("Location:"), this), 0, 2);
    stopsGrid->addWidget(opacityLocation, 0, 3);
    stopsGrid->addWidget(opacityDelete, 0, 4);
    color_ = new ColorSwatchWidget(this);
    color_->setObjectName(QStringLiteral("gradientStopColor"));
    color_->setFixedSize(60, 22);
    location_ = locationField();
    location_->setObjectName(QStringLiteral("gradientStopLocation"));
    location_->setKeyboardTracking(false);
    delete_ = new QPushButton(QStringLiteral("Delete"), this);
    delete_->setObjectName(QStringLiteral("gradientStopDelete"));
    delete_->setAutoDefault(false);
    stopsGrid->addWidget(new QLabel(QStringLiteral("Color:"), this), 1, 0);
    stopsGrid->addWidget(color_, 1, 1);
    stopsGrid->addWidget(new QLabel(QStringLiteral("Location:"), this), 1, 2);
    stopsGrid->addWidget(location_, 1, 3);
    stopsGrid->addWidget(delete_, 1, 4);
    stopsGrid->setColumnStretch(5, 1);
    solidColumn->addWidget(stopsBox);
    pages_->addWidget(solid);
    noise_ = new GradientNoisePage(pages_);
    pages_->addWidget(noise_);
    left->addWidget(rampBox);
    outer->addLayout(left, 1);

    auto* buttons = new QVBoxLayout;
    buttons->setSpacing(6);
    const auto addButton = [this, buttons](const QString& text, bool enabled) {
        auto* b = new QPushButton(text, this);
        b->setMinimumWidth(84);
        b->setAutoDefault(false);
        b->setEnabled(enabled);
        buttons->addWidget(b);
        return b;
    };
    QPushButton* ok = addButton(QStringLiteral("OK"), true);
    ok->setDefault(true);
    QPushButton* cancel = addButton(QStringLiteral("Cancel"), true);
    addButton(QStringLiteral("Load…"), false);
    addButton(QStringLiteral("Save…"), false);
    buttons->addSpacing(24);
    addButton(QStringLiteral("New"), false);
    buttons->addStretch(1);
    outer->addLayout(buttons);

    loading_ = true;
    bar_->setStops(spec.stops);
    bar_->select(0);
    smoothness_->setValue(spec.smoothness);
    noise_->setSettings(spec.noise);
    type_->setCurrentIndex(spec.type);
    pages_->setCurrentIndex(spec.type);
    loading_ = false;

    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(type_, &QComboBox::currentIndexChanged, this, [this](int index) {
        pages_->setCurrentIndex(index);
        edited();
    });
    connect(smoothness_, &PercentField::valueChanged, this, &GradientEditorDialog::edited);
    connect(noise_, &GradientNoisePage::changed, this, &GradientEditorDialog::edited);
    connect(bar_, &GradientStopBar::selectionChanged, this, &GradientEditorDialog::showSelection);
    connect(bar_, &GradientStopBar::stopsChanged, this, [this]() {
        showSelection(bar_->selected());
        edited();
    });
    connect(location_, &QSpinBox::valueChanged, this, [this](int percent) {
        if (!loading_) {
            bar_->setStopLocation(bar_->selected(), qRound(percent * kMaxLocation / 100.0));
        }
    });
    connect(delete_, &QPushButton::clicked, this, [this]() { bar_->removeStop(bar_->selected()); });
    connect(color_, &QPushButton::clicked, this, [this]() {
        const int index = bar_->selected();
        if (index < 0) {
            return;
        }
        const QColor picked = ColorPickerDialog::getColor(
            bar_->stops().at(index).color.toRgb(), this, QStringLiteral("Select stop color:"));
        if (picked.isValid()) {
            QColor color = picked;
            color.setAlpha(bar_->stops().at(index).color.alpha());
            bar_->setStopColor(index, color);
        }
    });
    showSelection(bar_->selected());
    setMinimumWidth(560);
}

void GradientEditorDialog::choosePreset(int index)
{
    const GradientStops stops = gradientPreset(index, foreground_, background_);
    if (stops.isEmpty()) {
        return;
    }
    loading_ = true;
    type_->setCurrentIndex(GradientSpec::Solid);
    bar_->setStops(stops);
    bar_->select(0);
    name_->setText(QString(gradient_preset_name(index)));
    loading_ = false;
    emit gradientChanged();
}

void GradientEditorDialog::showSelection(int index)
{
    const bool has = index >= 0 && index < bar_->stops().size();
    color_->setEnabled(has);
    location_->setEnabled(has);
    delete_->setEnabled(has && bar_->stops().size() > 2);
    const bool wasLoading = loading_;
    loading_ = true;
    if (has) {
        const GradientColorStop& stop = bar_->stops().at(index);
        location_->setValue(qRound(stop.location * 100.0 / kMaxLocation));
        color_->setColor(stop.color);
    } else {
        location_->clear();
        color_->setColor(QColor());
    }
    loading_ = wasLoading;
}

// An edit makes the gradient the user's own and previews it.
void GradientEditorDialog::edited()
{
    if (loading_) {
        return;
    }
    name_->setText(QStringLiteral("Custom"));
    emit gradientChanged();
}

GradientSpec GradientEditorDialog::spec() const
{
    GradientSpec spec;
    spec.name = name_->text();
    spec.type = type_->currentIndex();
    spec.stops = bar_->stops();
    spec.smoothness = qRound(smoothness_->value());
    spec.noise = noise_->settings();
    return spec;
}

} // namespace pictura
