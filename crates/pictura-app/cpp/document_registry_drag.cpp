// Tab tear-out and floating-window docking: the drag payload, the dock line,
// and the event filter shared by the tab strip and the document area.

#include "document_registry.h"

#include "api/document_bridge.h"
#include "document_float_window.h"

#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QDataStream>
#include <QtCore/QIODevice>
#include <QtCore/QMimeData>
#include <QtCore/QTimer>
#include <QtGui/QDrag>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QGuiApplication>
#include <QtGui/QMouseEvent>
#include <QtGui/QPixmap>
#include <QtGui/QWindow>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QWidget>

#include <utility>

namespace pictura {

namespace {

constexpr int kDockLineWidth = 2;

template <typename T>
QByteArray streamed(const T& value)
{
    QByteArray data;
    QDataStream stream(&data, QIODevice::WriteOnly);
    stream << value;
    return data;
}

std::unique_ptr<QMimeData> windowDockMime(const DocWindow* window, QWindow* carried,
                                          const QPoint& grabOffset)
{
    auto mime = std::make_unique<QMimeData>();
    mime->setData(kDocumentWindowMimeType,
                  QByteArray::number(quint64(reinterpret_cast<quintptr>(window))));
    if (carried) {
        mime->setData(QStringLiteral("application/x-qt-mainwindowdrag-window"),
                      streamed(reinterpret_cast<qintptr>(carried)));
        mime->setData(QStringLiteral("application/x-qt-mainwindowdrag-position"),
                      streamed(grabOffset));
    }
    return mime;
}

} // namespace

void DocumentRegistry::wireDocumentMenus()
{
    if (!tabs_) {
        return;
    }
    tabs_->tabBar()->installEventFilter(this);
    tabs_->installEventFilter(this);
}

bool DocumentRegistry::eventFilter(QObject* watched, QEvent* event)
{
    if (tabs_ && watched == tabs_->tabBar() && tearOutFilter(event)) {
        return true;
    }
    if (tabs_ && (watched == tabs_->tabBar() || watched == tabs_)
        && windowDropFilter(watched, event)) {
        return true;
    }
    if (tabs_ && watched == tabs_->tabBar() && event->type() == QEvent::MouseButtonRelease) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::MiddleButton) {
            DocWindow* window =
                windowAtTab(tabs_->tabBar()->tabAt(mouse->position().toPoint()));
            if (window) {
                const bool loading = window->doc->loading;
                const QPointer<PictureView> guard(window->doc->view);
                QTimer::singleShot(0, this, [this, window, guard, loading] {
                    if (!guard) {
                        return;
                    }
                    if (loading) {
                        emit cancelOpenRequested(guard.data());
                    } else if (windows_.contains(window) && window->doc->view == guard) {
                        requestCloseWindow(window);
                    }
                });
                return true;
            }
        }
    }
    return QObject::eventFilter(watched, event);
}

bool DocumentRegistry::tearOutFilter(QEvent* event)
{
    if (tearSynthesizing_) {
        return false;
    }
    QTabBar* bar = tabs_->tabBar();
    switch (event->type()) {
    case QEvent::MouseButtonPress: {
        auto* mouse = static_cast<QMouseEvent*>(event);
        tearFloat_ = nullptr;
        tearWindow_ = nullptr;
        if (mouse->button() == Qt::LeftButton) {
            tearWindow_ = windowAtTab(bar->tabAt(mouse->position().toPoint()));
            tearPressPos_ = mouse->position().toPoint();
        }
        return false;
    }
    case QEvent::MouseMove: {
        auto* mouse = static_cast<QMouseEvent*>(event);
        const QPoint globalPos = mouse->globalPosition().toPoint();
        if (tearFloat_) {
            tearFloat_->dragTo(globalPos);
            return true;
        }
        if (!tearWindow_ || !(mouse->buttons() & Qt::LeftButton)
            || !windows_.contains(tearWindow_) || tearWindow_->doc->loading) {
            return false;
        }
        const int y = mouse->position().toPoint().y();
        const int beyond = y < 0 ? -y : y - (bar->height() - 1);
        if (beyond <= QApplication::startDragDistance() * 3) {
            return false;
        }
        DocWindow* window = tearWindow_;
        tearWindow_ = nullptr;
        tearSynthesizing_ = true;
        QMouseEvent release(QEvent::MouseButtonRelease, QPointF(tearPressPos_),
                            QPointF(bar->mapToGlobal(tearPressPos_)), Qt::LeftButton,
                            Qt::NoButton, mouse->modifiers());
        QCoreApplication::sendEvent(bar, &release);
        tearSynthesizing_ = false;
        QRect geometry = defaultFloatGeometry();
        geometry.moveTopLeft(globalPos
                             - QPoint(geometry.width() / 2, DocumentFloatWindow::kHeaderHeight / 2));
        if (!floatWindow(window, geometry)) {
            return true;
        }
        DocumentFloatWindow* floating = window->floating;
        if (floating->isWindow() && DocumentFloatWindow::movesByWindowDrag()) {
            startWindowDockDrag(
                window, true,
                QPoint(floating->width() / 2, DocumentFloatWindow::kHeaderHeight / 2));
            return true;
        }
        tearFloat_ = floating;
        tearFloat_->beginDrag(globalPos);
        return true;
    }
    case QEvent::MouseButtonRelease: {
        tearWindow_ = nullptr;
        if (!tearFloat_) {
            return false;
        }
        const QPointer<DocumentFloatWindow> floating = tearFloat_;
        tearFloat_ = nullptr;
        floating->endDrag(static_cast<QMouseEvent*>(event)->globalPosition().toPoint());
        return true;
    }
    default:
        return false;
    }
}

DocWindow* DocumentRegistry::windowOfDockMime(const QMimeData* mime) const
{
    if (!mime || !mime->hasFormat(kDocumentWindowMimeType)) {
        return nullptr;
    }
    const quint64 address = mime->data(kDocumentWindowMimeType).toULongLong();
    for (DocWindow* window : windows_) {
        if (quint64(reinterpret_cast<quintptr>(window)) == address) {
            return window;
        }
    }
    return nullptr;
}

void DocumentRegistry::startWindowDockDrag(DocWindow* window, bool carryWindow,
                                           const QPoint& grabOffset)
{
    if (!windows_.contains(window) || !window->floating) {
        return;
    }
    if (windowDockDragForTest_) {
        windowDockDragForTest_(windowDockMime(
            window, carryWindow ? window->floating->windowHandle() : nullptr, grabOffset));
        return;
    }
    const QPixmap preview = carryWindow ? QPixmap() : window->floating->header()->grab();
    QTimer::singleShot(0, this, [this, window, carryWindow, grabOffset, preview]() {
        if (!windows_.contains(window) || window->kind != DocWindow::Host::Float
            || !window->floating || !(QGuiApplication::mouseButtons() & Qt::LeftButton)) {
            return;
        }
        QWindow* carried = carryWindow ? window->floating->windowHandle() : nullptr;
        auto* drag = new QDrag(this);
        drag->setMimeData(windowDockMime(window, carried, grabOffset).release());
        if (!preview.isNull()) {
            drag->setPixmap(preview);
        }
        windowDockDragRunning_ = true;
        drag->exec(Qt::MoveAction);
        windowDockDragRunning_ = false;
        hideDockIndicator();
        applyPendingDock();
    });
}

bool DocumentRegistry::windowDropFilter(QObject* watched, QEvent* event)
{
    const QEvent::Type type = event->type();
    if (type == QEvent::DragLeave) {
        hideDockIndicator();
        return false;
    }
    if (type != QEvent::DragEnter && type != QEvent::DragMove && type != QEvent::Drop) {
        return false;
    }
    auto* drop = static_cast<QDropEvent*>(event);
    if (!drop->mimeData() || !drop->mimeData()->hasFormat(kDocumentWindowMimeType)) {
        return false;
    }
    DocWindow* window = windowOfDockMime(drop->mimeData());
    const bool dockable = window && window->kind == DocWindow::Host::Float;
    const QPoint globalPos =
        static_cast<QWidget*>(watched)->mapToGlobal(drop->position().toPoint());
    const bool inZone = dockable && updateDockIndicator(globalPos);
    const bool entering =
        type == QEvent::DragEnter && dockable && !dockZoneGlobalRect().isEmpty();
    if (type == QEvent::Drop) {
        hideDockIndicator();
        if (inZone) {
            pendingDockWindow_ = window;
            pendingDockIndex_ = dockIndexAt(globalPos);
            QTimer::singleShot(0, this, [this]() { applyPendingDock(); });
        }
    }
    if (!inZone && !entering) {
        drop->ignore();
        return true;
    }
    drop->setDropAction(Qt::MoveAction);
    drop->accept();
    return true;
}

bool DocumentRegistry::updateDockIndicator(const QPoint& globalPos)
{
    const QRect zone = dockZoneGlobalRect();
    if (!zone.contains(globalPos)) {
        hideDockIndicator();
        return false;
    }
    QWidget* shell = hooks_.shellWidget();
    if (!dockIndicator_) {
        dockIndicator_ = new QWidget(shell);
        dockIndicator_->setObjectName(QStringLiteral("documentDockIndicator"));
        dockIndicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    }
    QTabBar* bar = tabs_->tabBar();
    QRect line;
    if (bar->isVisible() && bar->count() > 0) {
        const int index = dockIndexAt(globalPos);
        const int x = index >= 0 ? bar->tabRect(index).left()
                                 : bar->tabRect(bar->count() - 1).right() + 1;
        line = QRect(bar->mapToGlobal(QPoint(x, 0)).x() - kDockLineWidth / 2, zone.top(),
                     kDockLineWidth, zone.height());
    } else {
        line = QRect(zone.left(), zone.bottom() - kDockLineWidth + 1, zone.width(),
                     kDockLineWidth);
    }
    dockIndicator_->setGeometry(QRect(shell->mapFromGlobal(line.topLeft()), line.size()));
    dockIndicator_->show();
    dockIndicator_->raise();
    return true;
}

void DocumentRegistry::hideDockIndicator()
{
    if (dockIndicator_) {
        dockIndicator_->hide();
    }
}

void DocumentRegistry::applyPendingDock()
{
    if (windowDockDragRunning_ || !pendingDockWindow_) {
        return;
    }
    DocWindow* window = std::exchange(pendingDockWindow_, nullptr);
    if (windows_.contains(window) && window->kind == DocWindow::Host::Float) {
        dockWindow(window, pendingDockIndex_);
    }
}

} // namespace pictura
