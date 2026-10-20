#include "file_drop_router.h"

#include "frame.h"
#include "image_view.h"
#include "panels/layers_panel_internal.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QEvent>
#include <QtCore/QFileInfo>
#include <QtCore/QMimeData>
#include <QtCore/QUrl>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTabBar>

namespace pictura {

FileDropRouter::FileDropRouter(PicturaMainWindow* frame)
    : QObject(frame), frame_(frame)
{
}

QStringList FileDropRouter::localPaths(const QMimeData* mime)
{
    QStringList paths;
    if (!mime) {
        return paths;
    }
    for (const QUrl& url : mime->urls()) {
        if (!url.isLocalFile()) {
            continue;
        }
        const QString path = url.toLocalFile();
        if (QFileInfo(path).isFile()) {
            paths.append(path);
        }
    }
    return paths;
}

int FileDropRouter::otherDocumentTabAt(const QPoint& globalPos)
{
    for (QWidget* w = QApplication::widgetAt(globalPos); w; w = w->parentWidget()) {
        auto* bar = qobject_cast<QTabBar*>(w);
        if (bar && bar->objectName() == QLatin1String("documentTabBar")) {
            const int tab = bar->tabAt(bar->mapFromGlobal(globalPos));
            return tab != bar->currentIndex() ? tab : -1;
        }
    }
    return -1;
}

bool FileDropRouter::routeLayerDrag(QObject* watched, QEvent* event)
{
    auto* drag = static_cast<QDropEvent*>(event);
    const QMimeData* mime = drag->mimeData();
    const void* source = layerDragSource(mime);
    if (!mime->hasFormat(kLayerMimeType) || !source) {
        return false;
    }
    auto* bar = qobject_cast<QTabBar*>(watched);
    if (!bar && !qobject_cast<ImageView*>(watched)) {
        return false;
    }
    // Hovering a tab brings its document forward, as CS6 does, so the drop can
    // land on its tab or its canvas. The source tab is activated too, so a drag
    // that returns to it targets nothing rather than the last-hovered document.
    if (bar) {
        const int tab = bar->tabAt(drag->position().toPoint());
        if (tab < 0) {
            drag->ignore();
            return true;
        }
        frame_->setActiveDocumentIndex(tab);
    }
    const PictureView* target = frame_->activeView();
    if (!target || target == source) {
        drag->ignore();
        return true;
    }
    if (event->type() == QEvent::Drop) {
        const QStringList paths =
            QString::fromUtf8(mime->data(kLayerMimeType)).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
        // ponytail: only the dragged row crosses, as the in-panel drop moves only it.
        if (paths.isEmpty() || !frame_->copyLayerFromDocument(source, paths.first())) {
            drag->ignore();
            return true;
        }
    }
    drag->setDropAction(Qt::CopyAction);
    drag->accept();
    return true;
}

bool FileDropRouter::eventFilter(QObject* watched, QEvent* event)
{
    const bool dragEvent = event->type() == QEvent::DragEnter
        || event->type() == QEvent::DragMove || event->type() == QEvent::Drop;
    if (dragEvent && routeLayerDrag(watched, event)) {
        return true;
    }
    if (event->type() == QEvent::DragEnter || event->type() == QEvent::DragMove) {
        auto* drag = static_cast<QDragMoveEvent*>(event);
        if (localPaths(drag->mimeData()).isEmpty()) {
            return false;
        }
        drag->acceptProposedAction();
        return true;
    }
    if (event->type() != QEvent::Drop) {
        return QObject::eventFilter(watched, event);
    }

    auto* drop = static_cast<QDropEvent*>(event);
    const QStringList paths = localPaths(drop->mimeData());
    if (paths.isEmpty()) {
        return false;
    }

    PictureView* view = frame_->activeView();
    const bool place = qobject_cast<ImageView*>(watched) != nullptr && view != nullptr;
    bool changed = false;
    QString lastPlaced;
    if (place) {
        for (const QString& path : paths) {
            const QString layer = PicturaMainWindow::isNativeDocumentPath(path)
                ? view->place_smart_object(path)
                : view->place_image(path);
            if (!layer.isEmpty()) {
                lastPlaced = layer;
            }
            changed = changed || !layer.isEmpty();
        }
    } else {
        for (const QString& path : paths) {
            // Start every path; `changed || start(...)` would short-circuit the
            // second and later files.
            const quint64 token = frame_->startOpenDocumentAtPath(path);
            changed = changed || token != 0;
        }
    }
    if (changed) {
        if (place && !lastPlaced.isEmpty()) {
            frame_->beginFreeTransform(lastPlaced);
            frame_->selectLayerPath(lastPlaced);
        }
        frame_->refresh();
    }
    drop->acceptProposedAction();
    return true;
}

} // namespace pictura
