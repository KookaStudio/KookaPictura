#include "document_float_window.h"

#include "panels/panel_column.h"
#include "tab_spinner.h"

#include <QtCore/QCoreApplication>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QResizeEvent>
#include <QtGui/QScreen>
#include <QtGui/QWindow>
#include <QtWidgets/QApplication>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int kGripSize = 14;
constexpr int kMinWidth = 160;
constexpr int kMinHeight = 120;

bool gSystemMoveForTest = false;

// The bounds a float may be moved or grown in: the frame's rect for a child
// overlay, the screen's available area for a top-level window.
QRect floatBounds(const QWidget* floating)
{
    if (!floating->isWindow()) {
        const QWidget* host = floating->parentWidget();
        return host ? QRect(host->mapToGlobal(QPoint(0, 0)), host->size()) : QRect();
    }
    QScreen* screen = floating->screen();
    if (!screen) {
        screen = QGuiApplication::primaryScreen();
    }
    return screen ? screen->availableGeometry() : QRect();
}

// The header: press activates the window and starts a drag (with Ctrl, a
// dock drag), a double-click asks to dock, a right-click asks for the header
// menu.
class DocumentFloatHeader : public QWidget {
public:
    explicit DocumentFloatHeader(DocumentFloatWindow* floating)
        : QWidget(floating), floating_(floating)
    {
        setObjectName(QStringLiteral("documentFloatHeader"));
        setAttribute(Qt::WA_StyledBackground, true);
        setFixedHeight(DocumentFloatWindow::kHeaderHeight);
    }

protected:
    void mousePressEvent(QMouseEvent* event) override
    {
        if (floating_->onActivated) {
            floating_->onActivated();
        }
        dockDragArmed_ = false;
        if (event->button() == Qt::LeftButton) {
            if (event->modifiers() & Qt::ControlModifier) {
                dockDragArmed_ = true;
                dockDragStart_ = event->globalPosition().toPoint();
            } else {
                floating_->beginDrag(event->globalPosition().toPoint());
            }
        }
        event->accept();
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (!(event->buttons() & Qt::LeftButton)) {
            event->accept();
            return;
        }
        const QPoint globalPos = event->globalPosition().toPoint();
        if (!dockDragArmed_) {
            floating_->dragTo(globalPos);
        } else if ((globalPos - dockDragStart_).manhattanLength()
                   >= QApplication::startDragDistance()) {
            dockDragArmed_ = false;
            if (floating_->onDockDragRequested) {
                floating_->onDockDragRequested();
            }
        }
        event->accept();
    }

    void mouseReleaseEvent(QMouseEvent* event) override
    {
        if (event->button() == Qt::LeftButton) {
            dockDragArmed_ = false;
            floating_->endDrag(event->globalPosition().toPoint());
        }
        event->accept();
    }

    void mouseDoubleClickEvent(QMouseEvent* event) override
    {
        // The frame docks the float, which hides it and deletes it later, so
        // nothing here touches it afterwards.
        if (event->button() == Qt::LeftButton && floating_->onDockRequested) {
            floating_->onDockRequested();
        }
        event->accept();
    }

    void contextMenuEvent(QContextMenuEvent* event) override
    {
        if (floating_->onContextMenu) {
            floating_->onContextMenu(event->globalPos());
        }
        event->accept();
    }

private:
    DocumentFloatWindow* floating_;
    bool dockDragArmed_ = false;
    QPoint dockDragStart_;
};

// A bottom-right grip that grows or shrinks the float within its bounds,
// following `PanelEdgeGrip`: the press keeps the start geometry and each move
// maps from it.
class DocumentFloatGrip : public QWidget {
public:
    explicit DocumentFloatGrip(DocumentFloatWindow* floating)
        : QWidget(floating), floating_(floating)
    {
        setObjectName(QStringLiteral("documentFloatSizeGrip"));
        setCursor(Qt::SizeFDiagCursor);
        setFixedSize(kGripSize, kGripSize);
    }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.setPen(palette().color(QPalette::WindowText));
        for (int i = 0; i < 3; ++i) {
            const int offset = 3 + i * 4;
            painter.drawLine(width() - offset, height() - 2, width() - 2, height() - offset);
        }
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        if (event->button() == Qt::LeftButton) {
            pressGlobal_ = event->globalPosition().toPoint();
            startGeometry_ = floating_->globalGeometry();
        }
        event->accept();
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (!(event->buttons() & Qt::LeftButton)) {
            return;
        }
        const QPoint delta = event->globalPosition().toPoint() - pressGlobal_;
        const QRect bounds = floatBounds(floating_);
        int width = startGeometry_.width() + delta.x();
        int height = startGeometry_.height() + delta.y();
        if (bounds.isValid()) {
            width = qMin(width, bounds.right() - startGeometry_.left() + 1);
            height = qMin(height, bounds.bottom() - startGeometry_.top() + 1);
        }
        floating_->resize(QSize(width, height).expandedTo(floating_->minimumSize()));
        event->accept();
    }

    void mouseReleaseEvent(QMouseEvent* event) override { event->accept(); }

private:
    DocumentFloatWindow* floating_;
    QPoint pressGlobal_;
    QRect startGeometry_;
};

} // namespace

DocumentWindowTitle::DocumentWindowTitle(QWidget* parent)
    : QLabel(parent)
{
    setObjectName(QStringLiteral("documentFloatTitle"));
    setAttribute(Qt::WA_TransparentForMouseEvents, true);
    setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
}

void DocumentWindowTitle::setFullText(const QString& label, bool dirty)
{
    label_ = label;
    marker_ = dirty ? QStringLiteral(" *") : QString();
    setToolTip(label);
    elide();
}

void DocumentWindowTitle::resizeEvent(QResizeEvent* event)
{
    QLabel::resizeEvent(event);
    elide();
}

void DocumentWindowTitle::elide()
{
    const QFontMetrics metrics(font());
    const int available = qMax(0, width() - metrics.horizontalAdvance(marker_));
    setText(metrics.elidedText(label_, Qt::ElideRight, available) + marker_);
}

DocumentFloatWindow::DocumentFloatWindow(QWidget* frame)
    : QWidget(frame)
{
    setObjectName(QStringLiteral("documentFloatWindow"));
    if (PanelFloat::childOverlayForcedForTest()) {
        setWindowFlags(Qt::Widget);
    } else {
        setWindowFlags(Qt::Tool | Qt::FramelessWindowHint);
    }
    setAttribute(Qt::WA_StyledBackground, true);
    setMinimumSize(kMinWidth, kMinHeight);

    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(1, 1, 1, 1);
    layout->setSpacing(0);

    header_ = new DocumentFloatHeader(this);
    auto* headerLayout = new QHBoxLayout(header_);
    headerLayout->setContentsMargins(8, 0, 4, 0);
    headerLayout->setSpacing(4);
    title_ = new DocumentWindowTitle(header_);
    headerLayout->addWidget(title_, 1);
    auto* close = new TabCloseButton(header_);
    close->setObjectName(QStringLiteral("documentFloatClose"));
    close_ = close;
    // The frame closes the window on the next event-loop turn, so the button's
    // own click handler has returned before the float goes away.
    connect(close, &TabCloseButton::closeRequested, this, [this]() {
        if (onCloseRequested) {
            onCloseRequested();
        }
    });
    headerLayout->addWidget(close);
    layout->addWidget(header_);

    body_ = new QWidget(this);
    body_->setObjectName(QStringLiteral("documentFloatBody"));
    bodyLayout_ = new QVBoxLayout(body_);
    bodyLayout_->setContentsMargins(0, 0, 0, 0);
    bodyLayout_->setSpacing(0);
    layout->addWidget(body_, 1);

    grip_ = new DocumentFloatGrip(this);
}

void DocumentFloatWindow::setPage(QWidget* page)
{
    if (!page) {
        return;
    }
    page_ = page;
    bodyLayout_->addWidget(page);
    page->show();
    grip_->raise();
}

void DocumentFloatWindow::setLabel(const QString& label, bool dirty)
{
    title_->setFullText(label, dirty);
}

QString DocumentFloatWindow::headerText() const
{
    return title_->fullText();
}

QAbstractButton* DocumentFloatWindow::closeButton() const
{
    return close_;
}

QRect DocumentFloatWindow::globalGeometry() const
{
    if (isWindow()) {
        return geometry();
    }
    return QRect(mapToGlobal(QPoint(0, 0)), size());
}

void DocumentFloatWindow::setGlobalGeometry(const QRect& geometry)
{
    resize(geometry.size().expandedTo(minimumSize()));
    if (isWindow() || !parentWidget()) {
        move(geometry.topLeft());
    } else {
        move(parentWidget()->mapFromGlobal(geometry.topLeft()));
    }
}

void DocumentFloatWindow::beginDrag(const QPoint& globalPos)
{
    dragging_ = true;
    dragMoved_ = false;
    dragStart_ = globalPos;
    grabOffset_ = globalPos - globalGeometry().topLeft();
}

void DocumentFloatWindow::dragTo(const QPoint& globalPos)
{
    if (!dragging_) {
        return;
    }
    if (!dragMoved_
        && (globalPos - dragStart_).manhattanLength() < QApplication::startDragDistance()) {
        return;
    }
    if (!dragMoved_ && startSystemMove()) {
        return;
    }
    dragMoved_ = true;
    const QPoint topLeft = globalPos - grabOffset_;
    if (isWindow() || !parentWidget()) {
        move(topLeft);
        return;
    }
    // A child overlay stays inside the frame, so its header is never lost.
    QWidget* host = parentWidget();
    const QPoint local = host->mapFromGlobal(topLeft);
    const int maxX = qMax(0, host->width() - width());
    const int maxY = qMax(0, host->height() - height());
    move(qBound(0, local.x(), maxX), qBound(0, local.y(), maxY));
}

void DocumentFloatWindow::endDrag(const QPoint& globalPos)
{
    if (!dragging_) {
        return;
    }
    dragging_ = false;
    if (dragMoved_ && onDragReleased) {
        onDragReleased(globalPos);
    }
    dragMoved_ = false;
}

bool DocumentFloatWindow::movesBySystem()
{
    return gSystemMoveForTest
           || QGuiApplication::platformName().contains(QStringLiteral("wayland"),
                                                       Qt::CaseInsensitive);
}

void DocumentFloatWindow::setSystemMoveForTest(bool on)
{
    gSystemMoveForTest = on;
}

bool DocumentFloatWindow::startSystemMove()
{
    if (!isWindow() || !movesBySystem()) {
        return false;
    }
    QWindow* handle = windowHandle();
    if (!handle || !handle->startSystemMove()) {
        return false;
    }
    dragging_ = false;
    dragMoved_ = false;
    return true;
}

bool DocumentFloatWindow::event(QEvent* event)
{
    if (event->type() == QEvent::WindowActivate && isWindow() && onActivated) {
        onActivated();
    }
    return QWidget::event(event);
}

void DocumentFloatWindow::keyPressEvent(QKeyEvent* event)
{
    // A key event stops at a window boundary. A top-level float hands the keys
    // its children left unhandled to the frame, whose own key handling (Space
    // pan, Enter, the crop and nudge keys) then sees them as it does from a
    // tab. A child overlay's keys reach the frame by plain propagation.
    if (isWindow() && parentWidget()) {
        QCoreApplication::sendEvent(parentWidget(), event);
        return;
    }
    QWidget::keyPressEvent(event);
}

void DocumentFloatWindow::keyReleaseEvent(QKeyEvent* event)
{
    if (isWindow() && parentWidget()) {
        QCoreApplication::sendEvent(parentWidget(), event);
        return;
    }
    QWidget::keyReleaseEvent(event);
}

void DocumentFloatWindow::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    grip_->move(width() - grip_->width() - 1, height() - grip_->height() - 1);
    grip_->raise();
}

} // namespace pictura

#include "moc_document_float_window.cpp"
