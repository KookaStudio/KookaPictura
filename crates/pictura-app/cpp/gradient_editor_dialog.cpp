#include "gradient_editor_dialog.h"
#include <QtCore/QCoreApplication>
#include "preset_store.h"
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMap>
#include <QtCore/QSet>
#include <QtCore/QStringList>

#include "color_picker_dialog.h"
#include "panels/color_swatch_widget.h"
#include "panels/percent_field.h"

#include "api/paint_bridge.h"
#include "api/filters_bridge.h"

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
#include <iterator>

namespace pictura {

namespace {

constexpr int kMaxLocation = 4096;

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

GradientStops libraryRamp(const PresetRow& row, const QColor& foreground, const QColor& background)
{
    const int index = row.values.value(QStringLiteral("index")).toInt(-1);
    if (index >= 0) {
        return gradientPreset(index, foreground, background);
    }
    struct Point { QColor color; double midpoint; };
    QMap<int, Point> colors;
    QMap<int, QPair<double, double>> opacity;
    for (const auto& value : row.values.value(QStringLiteral("stops")).toArray()) {
        const auto stop = value.toObject();
        const auto color = stop.value(QStringLiteral("color"));
        const auto rgb = color.toArray();
        const auto rgba = color.toString() == QStringLiteral("foreground") ? foreground
            : color.toString() == QStringLiteral("background") ? background
            : rgb.size() == 3 ? QColor(qBound(0, qRound(rgb[0].toDouble()), 255),
                                       qBound(0, qRound(rgb[1].toDouble()), 255),
                                       qBound(0, qRound(rgb[2].toDouble()), 255)) : QColor();
        if (rgba.isValid()) {
            colors.insert(qBound(0, stop.value(QStringLiteral("at")).toInt(), kMaxLocation),
                          {rgba, qBound(.01, stop.value(QStringLiteral("mid")).toDouble(50) / 100.0, .99)});
        }
    }
    for (const auto& value : row.values.value(QStringLiteral("opacity")).toArray()) {
        const auto stop = value.toObject();
        opacity.insert(qBound(0, stop.value(QStringLiteral("at")).toInt(), kMaxLocation),
                       {qBound(0.0, stop.value(QStringLiteral("value")).toDouble() / 100.0, 1.0),
                        qBound(.01, stop.value(QStringLiteral("mid")).toDouble(50) / 100.0, .99)});
    }
    if (colors.isEmpty()) { return {}; }
    auto tAt = [](int at, int left, int right, double middle) {
        const double t = double(at - left) / double(right - left);
        return t <= middle ? .5 * t / middle : .5 + .5 * (t - middle) / (1 - middle);
    };
    auto positionsFor = [](const auto& source, auto midpoint) {
        QSet<int> positions;
        for (auto it = source.cbegin(); it != source.cend(); ++it) {
            positions.insert(it.key());
            const auto next = std::next(it);
            if (next != source.cend()) {
                positions.insert(it.key() + qRound((next.key() - it.key()) * midpoint(it.value())));
            }
        }
        return positions;
    };
    auto positions = positionsFor(colors, [](const Point& point) { return point.midpoint; });
    positions.unite(positionsFor(opacity, [](const auto& point) { return point.second; }));
    positions.insert(0);
    positions.insert(kMaxLocation);
    auto sorted = positions.values();
    std::sort(sorted.begin(), sorted.end());
    GradientStops result;
    for (const int at : sorted) {
        const auto high = colors.upperBound(at);
        QColor color;
        int midpoint = 50;
        if (high == colors.cbegin()) { color = colors.first().color; }
        else if (high == colors.cend()) { color = colors.last().color; }
        else {
            const auto low = std::prev(high);
            const double t = tAt(at, low.key(), high.key(), low->midpoint);
            const auto mix = [t](double x, double y) { return x + (y - x) * t; };
            color = QColor::fromRgbF(mix(low->color.redF(), high->color.redF()),
                                     mix(low->color.greenF(), high->color.greenF()),
                                     mix(low->color.blueF(), high->color.blueF()));
            midpoint = qRound(low->midpoint * 100.0);
        }
        if (!opacity.isEmpty()) {
            const auto hi = opacity.upperBound(at);
            double alpha;
            if (hi == opacity.cbegin()) { alpha = opacity.first().first; }
            else if (hi == opacity.cend()) { alpha = opacity.last().first; }
            else {
                const auto lo = std::prev(hi);
                const auto t = tAt(at, lo.key(), hi.key(), lo->second);
                alpha = lo->first + (hi->first - lo->first) * t;
            }
            color.setAlphaF(qBound(0.0, alpha, 1.0));
        }
        result.push_back({at, color, midpoint});
    }
    return result;
}

GradientStops noiseStops(const NoiseSettings& noise)
{
    return parseGradientStops(QString(gradient_noise_stops(
        noise.seed, noise.roughness, noise.model, noise.low[0], noise.high[0], noise.low[1],
        noise.high[1], noise.low[2], noise.high[2], noise.restrictColors, noise.transparency)));
}

GradientStops bakedSolidStops(const GradientSpec& spec)
{
    if (spec.stops.isEmpty()) {
        return {};
    }
    GradientStops colors = spec.stops;
    std::stable_sort(colors.begin(), colors.end(),
                     [](const auto& a, const auto& b) { return a.location < b.location; });
    GradientOpacityStops opacity = spec.opacityStops;
    if (opacity.isEmpty()) {
        opacity = gradientOpacityFromColors(colors);
    }
    std::stable_sort(opacity.begin(), opacity.end(),
                     [](const auto& a, const auto& b) { return a.location < b.location; });
    // A midpoint has no effect at the endpoints alone, so sample inside the
    // segments when one is off-centre.
    const bool dense =
        std::any_of(colors.begin(), colors.end(),
                    [](const GradientColorStop& s) { return s.midpoint != 50; })
        || std::any_of(opacity.begin(), opacity.end(),
                       [](const GradientOpacityStop& s) { return s.midpoint != 50; });
    QSet<int> positions;
    if (dense) {
        for (int at = 0; at <= kMaxLocation; at += 32) {
            positions.insert(at);
        }
        positions.insert(kMaxLocation);
    }
    for (const GradientColorStop& stop : colors) {
        positions.insert(stop.location);
    }
    for (const GradientOpacityStop& stop : opacity) {
        positions.insert(stop.location);
    }
    positions.insert(0);
    positions.insert(kMaxLocation);
    auto sorted = positions.values();
    std::sort(sorted.begin(), sorted.end());
    GradientStops result;
    for (const int at : sorted) {
        QColor color = gradientColorAt(colors, at);
        if (!opacity.isEmpty()) {
            color.setAlpha(qRound(gradientOpacityAt(opacity, at) * 255.0 / 100.0));
        }
        result.push_back({at, color, 50});
    }
    return result;
}

GradientStops resolvedStops(const GradientSpec& spec)
{
    if (spec.type == GradientSpec::Noise) {
        return noiseStops(spec.noise);
    }
    return parseGradientStops(
        QString(gradient_smoothed_stops(formatGradientStops(bakedSolidStops(spec)),
                                        spec.smoothness)));
}

QString formatGradientSpec(const GradientSpec& spec)
{
    QJsonObject object;
    object.insert(QStringLiteral("name"), spec.name);
    object.insert(QStringLiteral("type"), spec.type);
    object.insert(QStringLiteral("smoothness"), spec.smoothness);
    QJsonArray colors;
    for (const GradientColorStop& stop : spec.stops) {
        QJsonObject item;
        item.insert(QStringLiteral("at"), stop.location);
        item.insert(QStringLiteral("mid"), stop.midpoint);
        item.insert(QStringLiteral("color"), double(stop.color.rgba()));
        colors.append(item);
    }
    object.insert(QStringLiteral("stops"), colors);
    QJsonArray opacity;
    for (const GradientOpacityStop& stop : spec.opacityStops) {
        QJsonObject item;
        item.insert(QStringLiteral("at"), stop.location);
        item.insert(QStringLiteral("mid"), stop.midpoint);
        item.insert(QStringLiteral("value"), stop.opacity);
        opacity.append(item);
    }
    object.insert(QStringLiteral("opacity"), opacity);
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

QString formatGradientSpecStops(const GradientSpec& spec)
{
    QStringList colors;
    for (const GradientColorStop& stop : spec.stops) {
        QString hex = QStringLiteral("%1").arg(stop.color.rgb() & 0xffffffu, 6, 16,
                                               QLatin1Char('0'));
        if (stop.color.alpha() < 255) {
            hex += QStringLiteral("%1").arg(stop.color.alpha(), 2, 16, QLatin1Char('0'));
        }
        if (stop.midpoint != 50) {
            hex += QStringLiteral(":%1").arg(stop.midpoint);
        }
        colors << QStringLiteral("%1:%2").arg(stop.location).arg(hex);
    }
    QStringList opacity;
    for (const GradientOpacityStop& stop : spec.opacityStops) {
        opacity << QStringLiteral("%1:%2%3")
                       .arg(stop.location)
                       .arg(stop.opacity)
                       .arg(stop.midpoint != 50 ? QStringLiteral(":%1").arg(stop.midpoint)
                                                : QString());
    }
    QString text = colors.join(QLatin1Char(' '));
    if (!opacity.isEmpty()) {
        text += QLatin1Char('|') + opacity.join(QLatin1Char(' '));
    }
    return text;
}

GradientSpec parseGradientSpec(const QString& text)
{
    GradientSpec spec;
    const QJsonDocument document = QJsonDocument::fromJson(text.toUtf8());
    if (!document.isObject()) {
        return spec;
    }
    const QJsonObject object = document.object();
    spec.name = object.value(QStringLiteral("name")).toString();
    spec.type = qBound(0, object.value(QStringLiteral("type")).toInt(), 1);
    spec.smoothness = qBound(0, object.value(QStringLiteral("smoothness")).toInt(100), 100);
    for (const auto& value : object.value(QStringLiteral("stops")).toArray()) {
        const QJsonObject item = value.toObject();
        const int at = item.value(QStringLiteral("at")).toInt(-1);
        if (at < 0 || at > kMaxLocation) {
            continue;
        }
        const QRgb rgba =
            static_cast<QRgb>(item.value(QStringLiteral("color")).toDouble());
        spec.stops.push_back({at, QColor::fromRgba(rgba),
                              qBound(1, item.value(QStringLiteral("mid")).toInt(50), 99)});
    }
    for (const auto& value : object.value(QStringLiteral("opacity")).toArray()) {
        const QJsonObject item = value.toObject();
        const int at = item.value(QStringLiteral("at")).toInt(-1);
        if (at < 0 || at > kMaxLocation) {
            continue;
        }
        spec.opacityStops.push_back(
            {at, qBound(0, item.value(QStringLiteral("value")).toInt(100), 100),
             qBound(1, item.value(QStringLiteral("mid")).toInt(50), 99)});
    }
    if (spec.stops.size() < 2) {
        return {};
    }
    return spec;
}

// --- GradientSwatch ---------------------------------------------------------

GradientSwatch::GradientSwatch(QWidget* parent)
    : QWidget(parent)
{
    setCursor(Qt::PointingHandCursor);
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

void GradientSwatch::mousePressEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        emit clicked();
    }
    QWidget::mousePressEvent(event);
}

void GradientSwatch::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    const QRect r = rect().adjusted(0, 0, -1, -1);
    paintGradient(painter, r, stops_);
    painter.setPen(palette().color(QPalette::Dark));
    painter.drawRect(r);
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

    auto* presetsBox = new QGroupBox(QCoreApplication::translate("PicturaUi", "Presets"), this);
    auto* presetsGrid = new QGridLayout(presetsBox);
    presetsGrid->setSpacing(4);
    constexpr int kColumns = 10;
    constexpr int kSwatch = 30;
    auto& store = PresetStore::current();
    const auto rebuildPresets = [this, presetsBox, presetsGrid, foreground, background, &store] {
      while (auto* item = presetsGrid->takeAt(0)) {
          delete item->widget();
          delete item;
      }
      const auto& rows = store.rows(PresetType::Gradients);
      for (int i = 0; i < rows.size(); ++i) {
        QPixmap pixmap(kSwatch, kSwatch);
        {
            QPainter painter(&pixmap);
            paintGradient(painter, pixmap.rect(), libraryRamp(rows.at(i), foreground, background));
        }
        auto* swatch = new QToolButton(presetsBox);
        swatch->setObjectName(QStringLiteral("gradientPreset%1").arg(i));
        swatch->setIcon(QIcon(pixmap));
        swatch->setIconSize(QSize(kSwatch, kSwatch));
        swatch->setToolTip(rows.at(i).name);
        swatch->setAutoRaise(true);
        presetsGrid->addWidget(swatch, i / kColumns, i % kColumns);
        connect(swatch, &QToolButton::clicked, this, [this, i]() { choosePreset(i); });
      }
    };
    rebuildPresets();
    connect(&store, &PresetStore::changed, this, [rebuildPresets](int type) {
        if (type == int(PresetType::Gradients)) { rebuildPresets(); }
    });
    presetsGrid->setColumnStretch(kColumns, 1);
    left->addWidget(presetsBox);

    auto* nameRow = new QHBoxLayout;
    nameRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Name:"), this));
    name_ = new QLineEdit(spec.name, this);
    name_->setObjectName(QStringLiteral("gradientName"));
    nameRow->addWidget(name_, 1);
    left->addLayout(nameRow);

    auto* rampBox = new QGroupBox(this);
    auto* ramp = new QVBoxLayout(rampBox);
    auto* typeRow = new QHBoxLayout;
    typeRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Gradient Type:"), this));
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
    smoothness_ = new PercentField(QCoreApplication::translate("PicturaUi", "Smoothness:"), solid);
    smoothness_->setObjectName(QStringLiteral("gradientSmoothness"));
    auto* smoothRow = new QHBoxLayout;
    smoothRow->addWidget(smoothness_);
    smoothRow->addStretch(1);
    solidColumn->addLayout(smoothRow);
    bar_ = new GradientStopBar(solid);
    solidColumn->addWidget(bar_);

    auto* stopsBox = new QGroupBox(QCoreApplication::translate("PicturaUi", "Stops"), solid);
    auto* stopsGrid = new QGridLayout(stopsBox);
    const auto locationField = [this]() {
        auto* field = new QSpinBox(this);
        field->setRange(0, 100);
        field->setSuffix(QStringLiteral(" %"));
        field->setFixedWidth(70);
        return field;
    };
    opacity_ = new QSpinBox(this);
    opacity_->setObjectName(QStringLiteral("gradientOpacity"));
    opacity_->setRange(0, 100);
    opacity_->setSuffix(QStringLiteral(" %"));
    opacity_->setFixedWidth(70);
    opacityLocation_ = locationField();
    opacityLocation_->setObjectName(QStringLiteral("gradientOpacityLocation"));
    opacityLocation_->setKeyboardTracking(false);
    opacityDelete_ = new QPushButton(QCoreApplication::translate("PicturaUi", "Delete"), this);
    opacityDelete_->setObjectName(QStringLiteral("gradientOpacityDelete"));
    opacityDelete_->setAutoDefault(false);
    stopsGrid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Opacity:"), this), 0, 0);
    stopsGrid->addWidget(opacity_, 0, 1);
    stopsGrid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Location:"), this), 0, 2);
    stopsGrid->addWidget(opacityLocation_, 0, 3);
    stopsGrid->addWidget(opacityDelete_, 0, 4);
    color_ = new ColorSwatchWidget(this);
    color_->setObjectName(QStringLiteral("gradientStopColor"));
    color_->setFixedSize(60, 22);
    location_ = locationField();
    location_->setObjectName(QStringLiteral("gradientStopLocation"));
    location_->setKeyboardTracking(false);
    delete_ = new QPushButton(QCoreApplication::translate("PicturaUi", "Delete"), this);
    delete_->setObjectName(QStringLiteral("gradientStopDelete"));
    delete_->setAutoDefault(false);
    stopsGrid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Color:"), this), 1, 0);
    stopsGrid->addWidget(color_, 1, 1);
    stopsGrid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Location:"), this), 1, 2);
    stopsGrid->addWidget(location_, 1, 3);
    stopsGrid->addWidget(delete_, 1, 4);
    midpoint_ = new QSpinBox(this);
    midpoint_->setObjectName(QStringLiteral("gradientStopMidpoint"));
    midpoint_->setRange(1, 99);
    midpoint_->setSuffix(QStringLiteral(" %"));
    midpoint_->setFixedWidth(70);
    midpoint_->setKeyboardTracking(false);
    stopsGrid->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Midpoint:"), this), 2, 2);
    stopsGrid->addWidget(midpoint_, 2, 3);
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
    bar_->setGradient(spec.stops, spec.opacityStops);
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
    connect(bar_, &GradientStopBar::selectionChanged, this,
            [this](int) { showSelection(); });
    connect(bar_, &GradientStopBar::opacitySelectionChanged, this,
            [this](int) { showSelection(); });
    connect(bar_, &GradientStopBar::stopsChanged, this, [this]() {
        showSelection();
        edited();
    });
    connect(bar_, &GradientStopBar::opacityStopsChanged, this, [this]() {
        showSelection();
        edited();
    });
    connect(location_, &QSpinBox::valueChanged, this, [this](int percent) {
        if (!loading_) {
            bar_->setStopLocation(bar_->selected(), qRound(percent * kMaxLocation / 100.0));
        }
    });
    connect(opacityLocation_, &QSpinBox::valueChanged, this, [this](int percent) {
        if (!loading_) {
            bar_->setOpacityStopLocation(bar_->selectedOpacity(),
                                         qRound(percent * kMaxLocation / 100.0));
        }
    });
    connect(opacity_, &QSpinBox::valueChanged, this, [this](int percent) {
        if (!loading_) {
            bar_->setOpacityStopValue(bar_->selectedOpacity(), percent);
        }
    });
    connect(midpoint_, &QSpinBox::valueChanged, this, [this](int percent) {
        if (loading_) {
            return;
        }
        if (bar_->selectedKind() == GradientStopBar::StopKind::Opacity) {
            bar_->setOpacityStopMidpoint(bar_->selectedOpacity(), percent);
        } else {
            bar_->setStopMidpoint(bar_->selected(), percent);
        }
    });
    connect(delete_, &QPushButton::clicked, this, [this]() { bar_->removeStop(bar_->selected()); });
    connect(opacityDelete_, &QPushButton::clicked, this,
            [this]() { bar_->removeOpacityStop(bar_->selectedOpacity()); });
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
    showSelection();
    setMinimumWidth(560);
}

void GradientEditorDialog::setSolidOnly(bool solidOnly)
{
    type_->setEnabled(!solidOnly);
    smoothness_->setEnabled(!solidOnly);
    if (solidOnly) {
        type_->setCurrentIndex(GradientSpec::Solid);
    }
}

void GradientEditorDialog::choosePreset(int index)
{
    const auto& rows = PresetStore::current().rows(PresetType::Gradients);
    if (index < 0 || index >= rows.size()) { return; }
    const auto& row = rows.at(index);
    const GradientStops stops = libraryRamp(row, foreground_, background_);
    if (stops.isEmpty()) {
        return;
    }
    loading_ = true;
    type_->setCurrentIndex(GradientSpec::Solid);
    bar_->setGradient(stops, {});
    bar_->select(0);
    name_->setText(row.name);
    smoothness_->setValue(qBound(0, row.values.value(QStringLiteral("smoothness")).toInt(100), 100));
    loading_ = false;
    emit gradientChanged();
}

void GradientEditorDialog::showSelection()
{
    const bool color = bar_->selectedKind() == GradientStopBar::StopKind::Color;
    const int colorIndex = bar_->selected();
    const int opacityIndex = bar_->selectedOpacity();
    const bool hasColor = colorIndex >= 0 && colorIndex < bar_->stops().size();
    const bool hasOpacity = opacityIndex >= 0 && opacityIndex < bar_->opacityStops().size();
    color_->setEnabled(color && hasColor);
    location_->setEnabled(color && hasColor);
    delete_->setEnabled(color && hasColor && bar_->stops().size() > 2);
    opacity_->setEnabled(!color && hasOpacity);
    opacityLocation_->setEnabled(!color && hasOpacity);
    opacityDelete_->setEnabled(!color && hasOpacity && bar_->opacityStops().size() > 2);
    midpoint_->setEnabled(color ? hasColor : hasOpacity);
    const bool wasLoading = loading_;
    loading_ = true;
    if (color && hasColor) {
        const GradientColorStop& stop = bar_->stops().at(colorIndex);
        location_->setValue(qRound(stop.location * 100.0 / kMaxLocation));
        color_->setColor(stop.color);
        midpoint_->setValue(stop.midpoint);
        opacityLocation_->clear();
        opacity_->setValue(100);
    } else if (!color && hasOpacity) {
        const GradientOpacityStop& stop = bar_->opacityStops().at(opacityIndex);
        opacityLocation_->setValue(qRound(stop.location * 100.0 / kMaxLocation));
        opacity_->setValue(stop.opacity);
        midpoint_->setValue(stop.midpoint);
        location_->clear();
        color_->setColor(QColor());
    } else {
        location_->clear();
        opacityLocation_->clear();
        color_->setColor(QColor());
        midpoint_->setValue(50);
    }
    loading_ = wasLoading;
}

// An edit makes the gradient the user's own and previews it.
void GradientEditorDialog::edited()
{
    if (loading_) {
        return;
    }
    name_->setText(QCoreApplication::translate("PicturaUi", "Custom"));
    emit gradientChanged();
}

GradientSpec GradientEditorDialog::spec() const
{
    GradientSpec spec;
    spec.name = name_->text();
    spec.type = type_->currentIndex();
    spec.stops = bar_->stops();
    spec.opacityStops = bar_->opacityStops();
    spec.smoothness = qRound(smoothness_->value());
    spec.noise = noise_->settings();
    return spec;
}

} // namespace pictura

#include "moc_gradient_editor_dialog.cpp"
