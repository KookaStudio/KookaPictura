#include "layer_style_dialog.h"

#include "blend_modes.h"
#include "color_picker_dialog.h"
#include "image_view.h"
#include "panels/angle_dial.h"
#include "panels/color_swatch_widget.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QVBoxLayout>

#include <QtCore/QtMath>

#include <cmath>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"

namespace pictura {

namespace {

QColor unpack(double packed)
{
    const int v = std::isfinite(packed) ? qBound(0, int(packed + 0.5), 0xffffff) : 0;
    return QColor((v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff);
}

double pack(const QColor& c)
{
    return double((c.red() << 16) | (c.green() << 8) | c.blue());
}

// CS6's Distance scale: v = a·(e^(k·t) − 1) over t in [0, 1], fitted so t = ½
// is 61 px and t = 1 is 30000 px. With r = e^(k/2), r + 1 = 30000 / 61.
constexpr int kDistanceSteps = 1000;
const double kDistanceR = 30000.0 / 61.0 - 1.0;
const double kDistanceK = 2.0 * std::log(kDistanceR);
const double kDistanceA = 61.0 / (kDistanceR - 1.0);

ImageView* canvasUnder(QObject* object)
{
    for (auto* w = qobject_cast<QWidget*>(object); w; w = w->parentWidget()) {
        if (auto* view = qobject_cast<ImageView*>(w)) {
            return view;
        }
    }
    return nullptr;
}

void paintSwatch(ColorSwatchWidget* button, const QColor& color)
{
    button->setColor(color);
}

} // namespace

LayerStyleDialog::LayerStyleDialog(PictureView* view, const QString& path, const QString& effect,
                                   QWidget* parent)
    : QDialog(parent), view_(view), path_(path)
{
    setObjectName(QStringLiteral("layerStyleDialog"));
    setWindowTitle(tr("Layer Style"));
    resize(720, 520);

    auto* outer = new QHBoxLayout(this);
    list_ = new QListWidget;
    list_->setObjectName(QStringLiteral("layerStyleList"));
    list_->setFixedWidth(190);
    outer->addWidget(list_);

    pages_ = new QStackedWidget;
    outer->addWidget(pages_, 1);

    auto* buttons = new QVBoxLayout;
    auto* ok = new QPushButton(tr("OK"));
    ok->setObjectName(QStringLiteral("layerStyleOk"));
    ok->setDefault(true);
    auto* cancel = new QPushButton(tr("Cancel"));
    cancel->setObjectName(QStringLiteral("layerStyleCancel"));
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    buttons->addStretch();
    outer->addLayout(buttons);

    // CS6's order down the list, which is the order the effects apply.
    addFixedPage(tr("Blending Options"), buildBlendingOptionsPage());
    addEffect(QStringLiteral("bevel"), tr("Bevel && Emboss"), buildBevelPage());
    addEffect(QStringLiteral("stroke"), tr("Stroke"), buildStrokePage());
    addEffect(QStringLiteral("innerShadow"), tr("Inner Shadow"),
              buildShadowPage(QStringLiteral("innerShadow"), true));
    addEffect(QStringLiteral("innerGlow"), tr("Inner Glow"),
              buildGlowPage(QStringLiteral("innerGlow"), true));
    addEffect(QStringLiteral("satin"), tr("Satin"), buildSatinPage());
    addEffect(QStringLiteral("colorOverlay"), tr("Color Overlay"), buildColorOverlayPage());
    addEffect(QStringLiteral("gradientOverlay"), tr("Gradient Overlay"),
              buildGradientOverlayPage());
    addEffect(QStringLiteral("patternOverlay"), tr("Pattern Overlay"),
              buildPatternOverlayPage());
    addEffect(QStringLiteral("outerGlow"), tr("Outer Glow"),
              buildGlowPage(QStringLiteral("outerGlow"), false));
    addEffect(QStringLiteral("dropShadow"), tr("Drop Shadow"),
              buildShadowPage(QStringLiteral("dropShadow"), false));

    connect(list_, &QListWidget::currentRowChanged, pages_, &QStackedWidget::setCurrentIndex);
    connect(list_, &QListWidget::itemChanged, this, [this] { onListChanged(); });
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);

    const int row = qMax(0, keys_.indexOf(effect));
    list_->setCurrentRow(row);
    if (!effect.isEmpty() && value(effect + QStringLiteral(".on")) < 0.5
        && (list_->item(row)->flags() & Qt::ItemIsUserCheckable)) {
        list_->item(row)->setCheckState(Qt::Checked);
    }
}

int LayerStyleDialog::currentPage() const
{
    return pages_->currentIndex();
}

int LayerStyleDialog::distanceToSlider(double px)
{
    const double t = std::log(qMax(0.0, px) / kDistanceA + 1.0) / kDistanceK;
    return qBound(0, int(std::lround(t * kDistanceSteps)), kDistanceSteps);
}

double LayerStyleDialog::sliderToDistance(int position)
{
    const double t = double(position) / kDistanceSteps;
    return qBound(0.0, kDistanceA * (std::exp(kDistanceK * t) - 1.0), 30000.0);
}

QString LayerStyleDialog::shadowKey() const
{
    const QString key = keys_.value(currentPage());
    return key == QLatin1String("dropShadow") || key == QLatin1String("innerShadow") ? key
                                                                                      : QString();
}

bool LayerStyleDialog::setShadowOffset(const QPointF& offset)
{
    const QString key = shadowKey();
    if (key.isEmpty()) {
        return false;
    }
    // The renderer offsets by (−d·cos θ, d·sin θ): the light comes from θ.
    const double distance = std::hypot(offset.x(), offset.y());
    const QString angleKey = key + QStringLiteral(".angle");
    const QString distanceKey = key + QStringLiteral(".distance");
    if (distance >= 0.5) {
        setValue(angleKey, double(std::lround(qRadiansToDegrees(
                               std::atan2(offset.y(), -offset.x())))));
    }
    setValue(distanceKey, double(std::lround(distance)));
    refreshControls(angleKey);
    refreshControls(distanceKey);
    return true;
}

void LayerStyleDialog::refreshControls(const QString& key)
{
    const double v = value(key);
    if (!std::isfinite(v)) {
        return;
    }
    if (auto* spin = findChild<QSpinBox*>(key)) {
        const QSignalBlocker block(spin);
        spin->setValue(int(std::lround(v)));
    }
    if (auto* slider = findChild<QSlider*>(key + QStringLiteral(".slider"))) {
        const QSignalBlocker block(slider);
        slider->setValue(slider->property("exponential").toBool() ? distanceToSlider(v)
                                                                  : int(std::lround(v)));
    }
    if (auto* dial = findChild<AngleDial*>(key + QStringLiteral(".dial"))) {
        const QSignalBlocker block(dial);
        dial->setAngle(v);
    }
}

void LayerStyleDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    // Installed after runDialog's input blocker, so this filter runs first and
    // sees the canvas drag the blocker would otherwise swallow.
    qApp->installEventFilter(this);
}

void LayerStyleDialog::hideEvent(QHideEvent* event)
{
    qApp->removeEventFilter(this);
    dragging_ = false;
    QDialog::hideEvent(event);
}

bool LayerStyleDialog::eventFilter(QObject* watched, QEvent* event)
{
    const QEvent::Type type = event->type();
    if (type != QEvent::MouseButtonPress && type != QEvent::MouseMove
        && type != QEvent::MouseButtonRelease) {
        return QDialog::eventFilter(watched, event);
    }
    auto* mouse = static_cast<QMouseEvent*>(event);
    ImageView* canvas = canvasUnder(watched);
    if (!canvas || shadowKey().isEmpty()) {
        return QDialog::eventFilter(watched, event);
    }
    if (type == QEvent::MouseButtonPress && mouse->button() == Qt::LeftButton) {
        const QString key = shadowKey();
        const double distance = value(key + QStringLiteral(".distance"));
        const double angle = qDegreesToRadians(value(key + QStringLiteral(".angle")));
        dragOffset_ = QPointF(-distance * std::cos(angle), distance * std::sin(angle));
        dragStart_ = mouse->globalPosition();
        dragging_ = true;
        return true;
    }
    if (!dragging_) {
        return QDialog::eventFilter(watched, event);
    }
    if (type == QEvent::MouseButtonRelease) {
        dragging_ = false;
        return true;
    }
    const double zoom = canvas->zoom() > 0.0 ? canvas->zoom() : 1.0;
    setShadowOffset(dragOffset_ + (mouse->globalPosition() - dragStart_) / zoom);
    return true;
}

void LayerStyleDialog::addFixedPage(const QString& title, QWidget* page)
{
    // Blending Options is not an effect: always there, nothing to switch on.
    auto* item = new QListWidgetItem(title, list_);
    item->setFlags(item->flags() & ~Qt::ItemIsUserCheckable);
    pages_->addWidget(page);
    keys_.append(QString());
}

void LayerStyleDialog::addEffect(const QString& key, const QString& title, QWidget* page)
{
    auto* item = new QListWidgetItem(title, list_);
    item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
    const QSignalBlocker block(list_);
    item->setCheckState(value(key + QStringLiteral(".on")) >= 0.5 ? Qt::Checked : Qt::Unchecked);
    pages_->addWidget(page);
    keys_.append(key);
}

void LayerStyleDialog::onListChanged()
{
    for (int row = 0; row < list_->count(); ++row) {
        const QString& key = keys_.at(row);
        if (!key.isEmpty()) {
            const bool on = list_->item(row)->checkState() == Qt::Checked;
            setValue(key + QStringLiteral(".on"), on ? 1.0 : 0.0);
        }
    }
}

void LayerStyleDialog::accept()
{
    // One state for the whole visit, however many controls moved.
    if (dirty_ && view_) {
        layer_style_commit(*view_, tr("Layer Style"));
    }
    QDialog::accept();
}

void LayerStyleDialog::reject()
{
    if (dirty_ && view_) {
        layer_style_cancel(*view_);
    }
    dirty_ = false;
    QDialog::reject();
}

double LayerStyleDialog::value(const QString& key) const
{
    return view_ ? layer_style_value(*view_, path_, key) : 0.0;
}

void LayerStyleDialog::setValue(const QString& key, double v)
{
    if (view_ && layer_style_set(*view_, path_, key, v)) {
        dirty_ = true;
    }
}

// ------------------------------------------------------------------ binding --

void LayerStyleDialog::bindCheck(QCheckBox* box, const QString& key)
{
    box->setChecked(value(key) >= 0.5);
    connect(box, &QCheckBox::toggled, this,
            [this, key](bool on) { setValue(key, on ? 1.0 : 0.0); });
}

void LayerStyleDialog::bindChoice(QComboBox* combo, const QString& key)
{
    const double current = value(key);
    // A negative value (a file's own pattern, not a built-in one) selects none.
    if (std::isfinite(current)) {
        combo->setCurrentIndex(current < 0 ? -1
                                           : qBound(0, int(current + 0.5), combo->count() - 1));
    }
    connect(combo, &QComboBox::currentIndexChanged, this,
            [this, key](int index) { setValue(key, double(index)); });
}

void LayerStyleDialog::bindBlendMode(QComboBox* combo, const QString& key)
{
    for (const BlendEntry& entry : kBlends) {
        combo->addItem(tr(entry.name));
    }
    // CS6 groups the modes with separators; the combo keeps one index per
    // mode instead so the index is the engine's.
    bindChoice(combo, key);
}

void LayerStyleDialog::bindColor(ColorSwatchWidget* button, const QString& key)
{
    button->setFixedSize(40, 20);
    paintSwatch(button, unpack(value(key)));
    connect(button, &QPushButton::clicked, this, [this, key, button] {
        const QColor picked = ColorPickerDialog::getColor(unpack(value(key)), this, tr("Color"));
        if (picked.isValid()) {
            setValue(key, pack(picked));
            paintSwatch(button, picked);
        }
    });
}

QWidget* LayerStyleDialog::colorButton(const QString& key)
{
    auto* button = new ColorSwatchWidget;
    button->setObjectName(key);
    bindColor(button, key);
    return button;
}

QWidget* LayerStyleDialog::sliderRow(const QString& key, int min, int max, const QString& suffix)
{
    auto* row = new QWidget;
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    auto* slider = new QSlider(Qt::Horizontal);
    slider->setObjectName(key + QStringLiteral(".slider"));
    slider->setRange(min, max);
    auto* spin = new QSpinBox;
    spin->setObjectName(key);
    spin->setRange(min, max);
    spin->setSuffix(suffix);
    spin->setFixedWidth(80);
    const double start = value(key);
    const int v = std::isfinite(start) ? int(std::lround(start)) : min;
    slider->setValue(v);
    spin->setValue(v);
    // Two views of one number: each moves the other without re-writing it.
    connect(slider, &QSlider::valueChanged, this, [this, key, spin](int x) {
        const QSignalBlocker block(spin);
        spin->setValue(x);
        setValue(key, double(x));
    });
    connect(spin, &QSpinBox::valueChanged, this, [this, key, slider](int x) {
        const QSignalBlocker block(slider);
        slider->setValue(x);
        setValue(key, double(x));
    });
    layout->addWidget(slider, 1);
    layout->addWidget(spin);
    return row;
}

QWidget* LayerStyleDialog::distanceRow(const QString& key)
{
    auto* row = new QWidget;
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    auto* slider = new QSlider(Qt::Horizontal);
    slider->setObjectName(key + QStringLiteral(".slider"));
    slider->setProperty("exponential", true);
    slider->setRange(0, kDistanceSteps);
    auto* spin = new QSpinBox;
    spin->setObjectName(key);
    spin->setRange(0, 30000);
    spin->setSuffix(tr(" px"));
    spin->setFixedWidth(80);
    const double start = value(key);
    const int v = std::isfinite(start) ? int(std::lround(start)) : 0;
    slider->setValue(distanceToSlider(v));
    spin->setValue(v);
    connect(slider, &QSlider::valueChanged, this, [this, key, spin](int position) {
        const int px = int(std::lround(sliderToDistance(position)));
        const QSignalBlocker block(spin);
        spin->setValue(px);
        setValue(key, double(px));
    });
    connect(spin, &QSpinBox::valueChanged, this, [this, key, slider](int px) {
        const QSignalBlocker block(slider);
        slider->setValue(distanceToSlider(px));
        setValue(key, double(px));
    });
    layout->addWidget(slider, 1);
    layout->addWidget(spin);
    return row;
}

QWidget* LayerStyleDialog::angleRow(const QString& key)
{
    auto* row = new QWidget;
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    auto* dial = new AngleDial;
    dial->setObjectName(key + QStringLiteral(".dial"));
    auto* spin = new QSpinBox;
    spin->setObjectName(key);
    spin->setRange(-180, 180);
    spin->setSuffix(QStringLiteral("°"));
    spin->setFixedWidth(80);
    const double start = value(key);
    const int v = std::isfinite(start) ? int(std::lround(start)) : 0;
    dial->setAngle(v);
    spin->setValue(v);
    // The dial wraps into [0, 360); CS6 shows the angle in (-180, 180].
    connect(dial, &AngleDial::angleChanged, this, [this, key, spin](double degrees) {
        const int x = int(std::lround(degrees > 180.0 ? degrees - 360.0 : degrees));
        const QSignalBlocker block(spin);
        spin->setValue(x);
        setValue(key, double(x));
    });
    connect(spin, &QSpinBox::valueChanged, this, [this, key, dial](int x) {
        const QSignalBlocker block(dial);
        dial->setAngle(x);
        setValue(key, double(x));
    });
    layout->addWidget(dial);
    layout->addWidget(spin);
    layout->addStretch();
    return row;
}

} // namespace pictura
