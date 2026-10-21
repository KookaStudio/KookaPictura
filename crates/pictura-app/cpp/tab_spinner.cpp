#include "tab_spinner.h"

#include "icons.h"
#include "theme.h"

#include <QtCore/QTimer>
#include <QtGui/QEnterEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>

namespace pictura {

namespace {
constexpr int kSpinIntervalMs = 80;
constexpr int kSpinStep = 30;
constexpr int kSpinnerSize = 14;
} // namespace

TabSpinner::TabSpinner(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("tabSpinner"));
    setFixedSize(kSpinnerSize, kSpinnerSize);
    setCursor(Qt::ArrowCursor);
    setAttribute(Qt::WA_Hover, true);
    timer_ = new QTimer(this);
    timer_->setInterval(kSpinIntervalMs);
    connect(timer_, &QTimer::timeout, this, [this] {
        angle_ = (angle_ + kSpinStep) % 360;
        update();
    });
    // A tab button is created after its tab bar is visible; start now so the
    // animation also runs when the button is installed before the first show.
    timer_->start();
}

void TabSpinner::setOpeningName(const QString& name)
{
    setToolTip(tr("Opening %1 — click to cancel").arg(name));
    setAccessibleName(tr("Cancel opening %1").arg(name));
}

void TabSpinner::paintEvent(QPaintEvent* event)
{
    Q_UNUSED(event);
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    if (hovered_) {
        const QIcon close = icon(QStringLiteral("panel.close"));
        const int side = qMin(width(), height());
        painter.drawPixmap((width() - side) / 2, (height() - side) / 2,
                           close.pixmap(side, side));
        return;
    }
    QPen pen(Theme::tokenColor(QStringLiteral("content")));
    pen.setWidthF(1.6);
    pen.setCapStyle(Qt::RoundCap);
    painter.setPen(pen);
    const QRectF arc = QRectF(rect()).adjusted(1.5, 1.5, -1.5, -1.5);
    painter.drawArc(arc, angle_ * 16, 270 * 16);
}

void TabSpinner::enterEvent(QEnterEvent* event)
{
    hovered_ = true;
    update();
    QWidget::enterEvent(event);
}

void TabSpinner::leaveEvent(QEvent* event)
{
    hovered_ = false;
    pressed_ = false;
    update();
    QWidget::leaveEvent(event);
}

void TabSpinner::mousePressEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        pressed_ = true;
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void TabSpinner::mouseReleaseEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton && pressed_) {
        pressed_ = false;
        if (rect().contains(event->position().toPoint())) {
            emit cancelRequested();
        }
        event->accept();
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

void TabSpinner::showEvent(QShowEvent* event)
{
    timer_->start();
    QWidget::showEvent(event);
}

void TabSpinner::hideEvent(QHideEvent* event)
{
    timer_->stop();
    QWidget::hideEvent(event);
}

TabCloseButton::TabCloseButton(QWidget* parent)
    : QToolButton(parent)
{
    setObjectName(QStringLiteral("tabCloseButton"));
    setIcon(pictura::icon(QStringLiteral("panel.close")));
    setIconSize(QSize(12, 12));
    setFixedSize(16, 16);
    setAutoRaise(true);
    setCursor(Qt::ArrowCursor);
    setToolTip(tr("Close"));
    connect(this, &QToolButton::clicked, this, &TabCloseButton::closeRequested);
}

} // namespace pictura

#include "moc_tab_spinner.cpp"
