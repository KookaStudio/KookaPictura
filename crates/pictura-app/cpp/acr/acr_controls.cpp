#include "acr_controls.h"

#include "acr_settings.h"
#include "theme.h"

#include <QtCore/QtGlobal>
#include <QtGui/QLinearGradient>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPolygonF>
#include <QtGui/QResizeEvent>
#include <QtWidgets/QAbstractSpinBox>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

#include <cmath>

namespace pictura {

namespace {

constexpr int kRowHeight = 20;
constexpr int kHandleHalf = 6;
constexpr int kFieldWidth = 54;

// The pointer: a small house — a roof over a square body — centred on the
// handle x with its base on `base`.
QPolygonF housePolygon(int cx, int base)
{
    QPolygonF house;
    house << QPointF(cx, base - 15.0) << QPointF(cx + kHandleHalf, base - 9.0)
          << QPointF(cx + kHandleHalf, base) << QPointF(cx - kHandleHalf, base)
          << QPointF(cx - kHandleHalf, base - 9.0);
    return house;
}

void drawHouse(QPainter& painter, int cx, int base)
{
    painter.setPen(Theme::tokenColor(QStringLiteral("border")));
    painter.setBrush(Theme::tokenColor(QStringLiteral("content-control")));
    painter.drawPolygon(housePolygon(cx, base));
}

QList<QColor> effectiveRamp(const QList<QColor>& ramp)
{
    if (ramp.size() >= 2) {
        return ramp;
    }
    return {Theme::panelColor().darker(135), Theme::panelColor().lighter(118)};
}

void fillGroove(QPainter& painter, const QRect& groove, const QList<QColor>& ramp)
{
    const QList<QColor> stops = effectiveRamp(ramp);
    QLinearGradient gradient(groove.topLeft(), groove.topRight());
    for (int i = 0; i < stops.size(); ++i) {
        gradient.setColorAt(double(i) / double(stops.size() - 1), stops.at(i));
    }
    painter.fillRect(groove, gradient);
}

QString formatValue(double value, int decimals)
{
    return QString::number(value, 'f', decimals);
}

} // namespace

AcrSlider::AcrSlider(const QString& label, double minimum, double maximum, double value,
                     int decimals, QWidget* parent)
    : QWidget(parent), min_(minimum), max_(maximum), value_(qBound(minimum, value, maximum)),
      decimals_(decimals)
{
    setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
    label_ = new QLabel(label, this);
    label_->installEventFilter(this);
    field_ = new QDoubleSpinBox(this);
    field_->setDecimals(decimals);
    field_->setRange(minimum, maximum);
    field_->setSingleStep(decimals > 0 ? 0.05 : 1.0);
    field_->setValue(value_);
    field_->setButtonSymbols(QAbstractSpinBox::NoButtons);
    field_->setFrame(false);
    field_->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    connect(field_, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this, [this](double v) {
        if (updating_ || qFuzzyCompare(v + 1.0, value_ + 1.0)) {
            return;
        }
        value_ = v;
        update();
        emit valueChanged();
    });
}

double AcrSlider::value() const
{
    return value_;
}

void AcrSlider::setValue(double value)
{
    value_ = qBound(min_, value, max_);
    updating_ = true;
    field_->setValue(value_);
    updating_ = false;
    update();
}

void AcrSlider::setRamp(const QList<QColor>& stops)
{
    ramp_ = stops;
    update();
}

double AcrSlider::originFraction() const
{
    if (min_ < 0.0 && max_ > 0.0) {
        return (0.0 - min_) / (max_ - min_);
    }
    return 0.0;
}

int AcrSlider::xForValue(double value) const
{
    const int span = qMax(1, width() - 2 * kHandleHalf);
    const double t = max_ > min_ ? (value - min_) / (max_ - min_) : 0.0;
    return kHandleHalf + qRound(t * span);
}

double AcrSlider::valueForX(int x) const
{
    const int span = qMax(1, width() - 2 * kHandleHalf);
    const double t = qBound(0.0, double(x - kHandleHalf) / double(span), 1.0);
    const double raw = min_ + t * (max_ - min_);
    const double step = std::pow(10.0, -decimals_);
    return qBound(min_, qRound(raw / step) * step, max_);
}

void AcrSlider::setFromX(int x)
{
    const double next = valueForX(x);
    if (qFuzzyCompare(next + 1.0, value_ + 1.0)) {
        return;
    }
    setValue(next);
    emit valueChanged();
}

void AcrSlider::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    label_->setGeometry(0, 0, qMax(0, width() - kFieldWidth), kRowHeight);
    field_->setGeometry(width() - kFieldWidth, 0, kFieldWidth, kRowHeight);
}

QSize AcrSlider::sizeHint() const
{
    return QSize(200, kRowHeight + 20);
}

void AcrSlider::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    const int base = height() - 4;
    const int cy = base - 4;
    const QRect groove(kHandleHalf, cy - 3, qMax(1, width() - 2 * kHandleHalf), 6);

    QPainterPath clip;
    clip.addRoundedRect(groove, 3, 3);
    painter.save();
    painter.setClipPath(clip);
    fillGroove(painter, groove, ramp_);
    const int originX = kHandleHalf + qRound(originFraction() * qMax(1, width() - 2 * kHandleHalf));
    const int handleX = xForValue(value_);
    QColor fill = palette().color(QPalette::Highlight);
    fill.setAlpha(90);
    painter.fillRect(QRect(qMin(originX, handleX), groove.top(), qAbs(handleX - originX),
                           groove.height()),
                     fill);
    painter.restore();

    painter.setPen(QColor(0, 0, 0, 130));
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(groove, 3, 3);
    drawHouse(painter, handleX, base);
}

void AcrSlider::mousePressEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        dragging_ = true;
        setFromX(event->position().toPoint().x());
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void AcrSlider::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_) {
        setFromX(event->position().toPoint().x());
        event->accept();
        return;
    }
    QWidget::mouseMoveEvent(event);
}

void AcrSlider::mouseReleaseEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton && dragging_) {
        dragging_ = false;
        event->accept();
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

bool AcrSlider::eventFilter(QObject* watched, QEvent* event)
{
    if (watched != label_) {
        return QWidget::eventFilter(watched, event);
    }
    if (event->type() == QEvent::MouseButtonPress) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton) {
            scrubbing_ = true;
            scrubStartX_ = mouse->globalPosition().toPoint().x();
            scrubStartValue_ = value_;
            return true;
        }
    } else if (event->type() == QEvent::MouseMove && scrubbing_) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        const int dx = mouse->globalPosition().toPoint().x() - scrubStartX_;
        const double span = max_ - min_;
        const double next = qBound(min_, scrubStartValue_ + dx * span / 200.0, max_);
        setValue(next);
        emit valueChanged();
        return true;
    } else if (event->type() == QEvent::MouseButtonRelease) {
        scrubbing_ = false;
        return true;
    }
    return QWidget::eventFilter(watched, event);
}

AcrRangeSlider::AcrRangeSlider(const QString& label, double minimum, double maximum, double lo,
                               double hi, int decimals, QWidget* parent)
    : QWidget(parent), min_(minimum), max_(maximum), lo_(qBound(minimum, lo, maximum)),
      hi_(qBound(minimum, hi, maximum)), decimals_(decimals)
{
    setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
    label_ = new QLabel(label, this);
    value_ = new QLabel(this);
    value_->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    value_->setText(QStringLiteral("%1 / %2").arg(formatValue(lo_, decimals_),
                                                  formatValue(hi_, decimals_)));
}

double AcrRangeSlider::low() const
{
    return lo_;
}

double AcrRangeSlider::high() const
{
    return hi_;
}

void AcrRangeSlider::setValues(double lo, double hi)
{
    lo_ = qBound(min_, lo, max_);
    hi_ = qBound(min_, hi, max_);
    value_->setText(
        QStringLiteral("%1 / %2").arg(formatValue(lo_, decimals_), formatValue(hi_, decimals_)));
    update();
}

void AcrRangeSlider::setRamp(const QList<QColor>& stops)
{
    ramp_ = stops;
    update();
}

int AcrRangeSlider::xForValue(double value) const
{
    const int span = qMax(1, width() - 2 * kHandleHalf);
    const double t = max_ > min_ ? (value - min_) / (max_ - min_) : 0.0;
    return kHandleHalf + qRound(t * span);
}

double AcrRangeSlider::valueForX(int x) const
{
    const int span = qMax(1, width() - 2 * kHandleHalf);
    const double t = qBound(0.0, double(x - kHandleHalf) / double(span), 1.0);
    const double step = std::pow(10.0, -decimals_);
    return qBound(min_, qRound((min_ + t * (max_ - min_)) / step) * step, max_);
}

void AcrRangeSlider::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    label_->setGeometry(0, 0, qMax(0, width() - kFieldWidth), kRowHeight);
    value_->setGeometry(width() - kFieldWidth, 0, kFieldWidth, kRowHeight);
}

QSize AcrRangeSlider::sizeHint() const
{
    return QSize(200, kRowHeight + 20);
}

void AcrRangeSlider::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    const int base = height() - 4;
    const int cy = base - 4;
    const QRect groove(kHandleHalf, cy - 3, qMax(1, width() - 2 * kHandleHalf), 6);

    QPainterPath clip;
    clip.addRoundedRect(groove, 3, 3);
    painter.save();
    painter.setClipPath(clip);
    fillGroove(painter, groove, ramp_);
    painter.restore();

    painter.setPen(QColor(0, 0, 0, 130));
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(groove, 3, 3);
    drawHouse(painter, xForValue(lo_), base);
    drawHouse(painter, xForValue(hi_), base);
}

void AcrRangeSlider::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton) {
        QWidget::mousePressEvent(event);
        return;
    }
    const int x = event->position().toPoint().x();
    dragging_ = qAbs(x - xForValue(lo_)) <= qAbs(x - xForValue(hi_)) ? 0 : 1;
    mouseMoveEvent(event);
}

void AcrRangeSlider::mouseMoveEvent(QMouseEvent* event)
{
    if (dragging_ < 0) {
        QWidget::mouseMoveEvent(event);
        return;
    }
    const double next = valueForX(event->position().toPoint().x());
    if (dragging_ == 0) {
        lo_ = qMin(next, hi_);
    } else {
        hi_ = qMax(next, lo_);
    }
    setValues(lo_, hi_);
    emit valueChanged();
    event->accept();
}

void AcrRangeSlider::mouseReleaseEvent(QMouseEvent* event)
{
    if (dragging_ >= 0) {
        dragging_ = -1;
        event->accept();
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

AcrCombo::AcrCombo(const QString& label, const QStringList& items, const QString& value,
                   QWidget* parent)
    : QWidget(parent)
{
    auto* row = new QHBoxLayout(this);
    row->setContentsMargins(0, 0, 0, 0);
    row->addWidget(new QLabel(label, this));
    row->addStretch(1);
    combo_ = new QComboBox(this);
    combo_->addItems(items);
    setValue(value);
    row->addWidget(combo_);
    connect(combo_, &QComboBox::currentTextChanged, this, [this](const QString&) {
        emit valueChanged();
    });
}

QString AcrCombo::value() const
{
    return combo_->currentText();
}

void AcrCombo::setValue(const QString& value)
{
    int index = combo_->findText(value);
    if (index < 0) {
        index = combo_->findText(QString(value).remove(QChar('\0')));
    }
    if (index >= 0) {
        combo_->setCurrentIndex(index);
    }
}

AcrCheck::AcrCheck(const QString& label, bool checked, QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    box_ = new QCheckBox(label, this);
    box_->setChecked(checked);
    layout->addWidget(box_);
    connect(box_, &QCheckBox::toggled, this, &AcrCheck::toggled);
}

void AcrCheck::setChecked(bool checked)
{
    box_->setChecked(checked);
}

QLabel* acrSection(const QString& text, QWidget* parent)
{
    auto* label = new QLabel(text, parent);
    QFont font = label->font();
    font.setBold(true);
    label->setFont(font);
    label->setAlignment(Qt::AlignHCenter);
    return label;
}

AcrPanel::AcrPanel(AcrSettings* settings, QWidget* parent)
    : QWidget(parent), settings_(settings)
{
}

AcrSlider* AcrPanel::addSlider(QVBoxLayout* column, const QByteArray& key, const QString& label,
                               double minimum, double maximum, int decimals, double defaultValue,
                               const QList<QColor>& ramp)
{
    const QByteArray stored = settings_->raw(key);
    const double value = stored.isEmpty() ? defaultValue : stored.toDouble();
    auto* slider = new AcrSlider(label, minimum, maximum, value, decimals, this);
    slider->setObjectName(QString::fromLatin1(key));
    slider->setRamp(ramp);
    connect(slider, &AcrSlider::valueChanged, this, [this, key, slider, decimals]() {
        if (decimals == 0) {
            settings_->setInt(key, static_cast<int>(std::lround(slider->value())));
        } else {
            settings_->setDouble(key, slider->value());
        }
        emit changed();
    });
    column->addWidget(slider);
    return slider;
}

AcrRangeSlider* AcrPanel::addRangeSlider(QVBoxLayout* column, const QByteArray& keyLo,
                                         const QByteArray& keyHi, const QString& label,
                                         double minimum, double maximum, int decimals,
                                         double defaultLo, double defaultHi,
                                         const QList<QColor>& ramp)
{
    const QByteArray storedLo = settings_->raw(keyLo);
    const QByteArray storedHi = settings_->raw(keyHi);
    const double lo = storedLo.isEmpty() ? defaultLo : storedLo.toDouble();
    const double hi = storedHi.isEmpty() ? defaultHi : storedHi.toDouble();
    auto* slider = new AcrRangeSlider(label, minimum, maximum, lo, hi, decimals, this);
    slider->setObjectName(QString::fromLatin1(keyLo));
    slider->setRamp(ramp);
    connect(slider, &AcrRangeSlider::valueChanged, this, [this, keyLo, keyHi, slider, decimals]() {
        if (decimals == 0) {
            settings_->setInt(keyLo, static_cast<int>(std::lround(slider->low())));
            settings_->setInt(keyHi, static_cast<int>(std::lround(slider->high())));
        } else {
            settings_->setDouble(keyLo, slider->low());
            settings_->setDouble(keyHi, slider->high());
        }
        emit changed();
    });
    column->addWidget(slider);
    return slider;
}

AcrCombo* AcrPanel::addCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                             const QStringList& items, const QString& defaultValue)
{
    const QString value = settings_->textValue(key, defaultValue);
    auto* combo = new AcrCombo(label, items, value, this);
    combo->setObjectName(QString::fromLatin1(key));
    connect(combo, &AcrCombo::valueChanged, this, [this, key, combo]() {
        settings_->setText(key, combo->value());
        emit changed();
    });
    column->addWidget(combo);
    return combo;
}

AcrCombo* AcrPanel::addEnumCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                                 const QList<AcrEnumItem>& items)
{
    QStringList labels;
    labels.reserve(items.size());
    for (const AcrEnumItem& item : items) {
        labels.append(item.label);
    }
    const QByteArray stored = settings_->raw(key);
    QString value;
    for (const AcrEnumItem& item : items) {
        if (item.code == stored) {
            value = item.label;
            break;
        }
    }
    auto* combo = new AcrCombo(label, labels, value, this);
    combo->setObjectName(QString::fromLatin1(key));
    connect(combo, &AcrCombo::valueChanged, this, [this, key, combo, items]() {
        for (const AcrEnumItem& item : items) {
            if (item.label == combo->value()) {
                settings_->setRaw(key, item.code);
                break;
            }
        }
        emit changed();
    });
    column->addWidget(combo);
    return combo;
}

AcrCheck* AcrPanel::addCheck(QVBoxLayout* column, const QByteArray& key, const QString& label,
                             bool defaultValue)
{
    const bool checked = settings_->boolValue(key, defaultValue);
    auto* check = new AcrCheck(label, checked, this);
    check->setObjectName(QString::fromLatin1(key));
    connect(check, &AcrCheck::toggled, this, [this, key](bool on) {
        settings_->setBool(key, on);
        emit changed();
    });
    column->addWidget(check);
    return check;
}

void AcrPanel::addSection(QVBoxLayout* column, const QString& text)
{
    addSeparator(column);
    column->addWidget(acrSection(text, this));
}

void AcrPanel::addSeparator(QVBoxLayout* column)
{
    auto* line = new QFrame(this);
    line->setFrameShape(QFrame::HLine);
    line->setFrameShadow(QFrame::Plain);
    column->addWidget(line);
}

} // namespace pictura

#include "moc_acr_controls.cpp"
