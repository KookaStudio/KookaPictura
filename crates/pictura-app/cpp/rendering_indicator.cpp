#include "rendering_indicator.h"

#include <QtCore/QTimer>
#include <QtGui/QFontMetrics>
#include <QtGui/QPainter>
#include <QtGui/QPaintEvent>

#include <algorithm>

namespace pictura {

namespace {

constexpr int kMargin = 10;
constexpr int kPadding = 8;
constexpr int kSpinner = 14;

} // namespace

RenderingIndicator::RenderingIndicator(QWidget* canvas)
    : QWidget(canvas)
    , delay_(new QTimer(this))
    , animation_(new QTimer(this))
{
    setObjectName(QStringLiteral("renderingIndicator"));
    setAttribute(Qt::WA_TransparentForMouseEvents);
    setAttribute(Qt::WA_TranslucentBackground);
    setFocusPolicy(Qt::NoFocus);
    delay_->setSingleShot(true);
    delay_->setInterval(kDelayMs);
    animation_->setInterval(kTickMs);
    connect(delay_, &QTimer::timeout, this, [this] {
        if (pending()) {
            showNow();
        }
    });
    connect(animation_, &QTimer::timeout, this, &RenderingIndicator::tick);
    const QString label = tr("Rendering…");
    const QFontMetrics metrics(font());
    resize(kPadding * 3 + kSpinner + metrics.horizontalAdvance(label),
           std::max(kSpinner, metrics.height()) + kPadding * 2);
    hide();
}

void RenderingIndicator::setPendingSource(std::function<bool()> pending)
{
    pending_ = std::move(pending);
    poke();
}

bool RenderingIndicator::pending() const
{
    return pending_ && pending_();
}

void RenderingIndicator::poke()
{
    if (!pending()) {
        stop();
        return;
    }
    if (!isVisible() && !delay_->isActive()) {
        delay_->start();
    }
}

void RenderingIndicator::place()
{
    const QWidget* canvas = parentWidget();
    if (canvas) {
        move(canvas->width() - width() - kMargin, kMargin);
    }
}

void RenderingIndicator::showNow()
{
    place();
    raise();
    show();
    animation_->start();
}

void RenderingIndicator::stop()
{
    delay_->stop();
    animation_->stop();
    hide();
}

void RenderingIndicator::tick()
{
    if (!pending()) {
        stop();
        return;
    }
    angle_ = (angle_ + 30) % 360;
    update();
}

bool RenderingIndicator::delayActiveForTest() const
{
    return delay_->isActive();
}

bool RenderingIndicator::animationActiveForTest() const
{
    return animation_->isActive();
}

void RenderingIndicator::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(0, 0, 0, 160));
    painter.drawRoundedRect(QRectF(rect()), 6, 6);

    const QRectF spinner(kPadding, (height() - kSpinner) / 2.0, kSpinner, kSpinner);
    QPen track(QColor(255, 255, 255, 60), 2.0);
    painter.setPen(track);
    painter.setBrush(Qt::NoBrush);
    painter.drawEllipse(spinner);
    QPen arc(Qt::white, 2.0);
    arc.setCapStyle(Qt::RoundCap);
    painter.setPen(arc);
    // Qt angles are in 1/16 degree, counter-clockwise; turn clockwise.
    painter.drawArc(spinner, -angle_ * 16, 100 * 16);

    painter.setPen(Qt::white);
    const QRect text(kPadding * 2 + kSpinner, 0, width() - kPadding * 3 - kSpinner, height());
    painter.drawText(text, Qt::AlignVCenter | Qt::AlignLeft, tr("Rendering…"));
}

} // namespace pictura

#include "moc_rendering_indicator.cpp"
