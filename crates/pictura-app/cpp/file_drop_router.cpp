#include "file_drop_router.h"

#include "frame.h"
#include "channel_drag.h"
#include "document_window.h"
#include "image_view.h"
#include "selection_drag.h"
#include "panels/layers_panel_internal.h"

#include "api/layers_bridge.h"
#include "api/selection_bridge.h"
#include "api/view_bridge.h"

#include <QtCore/QEvent>
#include <algorithm>
#include <cstddef>
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

QStringList layerDragCopyPaths(const QStringList& target, const QString& dragged)
{
    if (dragged.isEmpty()) {
        return {};
    }
    if (!target.contains(dragged)) {
        return {dragged};
    }
    QStringList paths;
    for (const QString& path : target) {
        const bool covered = std::any_of(target.cbegin(), target.cend(), [&path](const QString& other) {
            return other != path && path.startsWith(other + QLatin1Char('/'));
        });
        if (!covered) {
            paths.append(path);
        }
    }
    return paths.isEmpty() ? QStringList{dragged} : paths;
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

bool FileDropRouter::otherDocumentAt(const QPoint& globalPos, const PictureView* source,
                                     QWidget* within)
{
    auto* frame = qobject_cast<PicturaMainWindow*>(within ? within->window() : nullptr);
    if (!frame) {
        for (QWidget* top : QApplication::topLevelWidgets()) {
            if (auto* main = qobject_cast<PicturaMainWindow*>(top)) {
                frame = main;
                break;
            }
        }
    }
    for (QWidget* w = QApplication::widgetAt(globalPos); w; w = w->parentWidget()) {
        if (auto* bar = qobject_cast<QTabBar*>(w)) {
            const int tab = bar->tabAt(bar->mapFromGlobal(globalPos));
            if (tab >= 0 && tab != bar->currentIndex()) {
                return true;
            }
        }
        if (auto* canvas = qobject_cast<ImageView*>(w)) {
            if (!frame) {
                continue;
            }
            PictureView* target = frame->viewForCanvas(canvas);
            if (target && target != source) {
                return true;
            }
        }
    }
    return false;
}

bool FileDropRouter::isOpenView(const void* view) const
{
    if (!view || !frame_) {
        return false;
    }
    for (int i = 0; i < frame_->documentCount(); ++i) {
        if (static_cast<const void*>(frame_->viewAt(i)) == view) {
            return true;
        }
    }
    return false;
}

bool FileDropRouter::routeLayerDrag(QObject* watched, QEvent* event)
{
    auto* drag = static_cast<QDropEvent*>(event);
    const QMimeData* mime = drag->mimeData();
    if (!mime->hasFormat(kLayerMimeType)) {
        return false;
    }
    auto* bar = qobject_cast<QTabBar*>(watched);
    if (!bar && !qobject_cast<ImageView*>(watched)) {
        return false;
    }
    const void* source = layerDragSource(mime);
    if (!isOpenView(source)) {
        drag->ignore();
        return true;
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
        // The frame expands the dragged row to the source document's target;
        // the payload still carries one path.
        if (paths.isEmpty() || !frame_->copyLayersFromDocument(source, paths.first())) {
            drag->ignore();
            return true;
        }
    }
    drag->setDropAction(Qt::CopyAction);
    drag->accept();
    return true;
}

PictureView* FileDropRouter::dragTarget(QObject* watched, QEvent* event)
{
    auto* drag = static_cast<QDropEvent*>(event);
    if (auto* bar = qobject_cast<QTabBar*>(watched)) {
        const int tab = bar->tabAt(drag->position().toPoint());
        if (tab < 0) {
            return nullptr;
        }
        frame_->setActiveDocumentIndex(tab);
        return frame_->activeView();
    }
    if (auto* canvas = qobject_cast<ImageView*>(watched)) {
        return frame_->viewForCanvas(canvas);
    }
    return nullptr;
}

bool FileDropRouter::routeChannelDrag(QObject* watched, QEvent* event)
{
    auto* drag = static_cast<QDropEvent*>(event);
    const QMimeData* mime = drag->mimeData();
    if (!mime->hasFormat(kChannelRowMime) || !mime->hasFormat(kChannelSourceMime)) {
        return false;
    }
    if (!qobject_cast<QTabBar*>(watched) && !qobject_cast<ImageView*>(watched)) {
        return false;
    }
    const auto* source = static_cast<const PictureView*>(channelDragSource(mime));
    if (!isOpenView(source)) {
        drag->ignore();
        return true;
    }
    PictureView* target = dragTarget(watched, event);
    if (!target || source == target || !source->has_document()
        || !target->has_document()) {
        drag->ignore();
        return true;
    }
    if (event->type() == QEvent::Drop) {
        const ChannelDrag payload = channelDrag(mime);
        const int width = source->document_width();
        const int height = source->document_height();
        const auto coverage =
            payload.alpha >= 0
                ? channel_coverage(*source, QStringLiteral("alpha:%1").arg(payload.alpha))
                : color_row_coverage(*source, payload.row, payload.mask);
        if (coverage.size() != std::size_t(width) * std::size_t(height)) {
            drag->ignore();
            return true;
        }
        const QString name = payload.name.isEmpty()
            ? tr("Alpha 1 copy")
            : tr("%1 copy").arg(payload.name);
        const ::rust::Slice<const std::uint8_t> bytes(coverage.data(), coverage.size());
        if (!duplicate_channel_coverage(*target, bytes, width, height, name, false)) {
            drag->ignore();
            return true;
        }
        frame_->activateView(target);
        frame_->refresh();
    }
    drag->setDropAction(Qt::CopyAction);
    drag->accept();
    return true;
}

bool FileDropRouter::routeSelectionDrag(QObject* watched, QEvent* event)
{
    auto* drag = static_cast<QDropEvent*>(event);
    const QMimeData* mime = drag->mimeData();
    if (!mime->hasFormat(kSelectionSourceMime)) {
        return false;
    }
    if (!qobject_cast<QTabBar*>(watched) && !qobject_cast<ImageView*>(watched)) {
        return false;
    }
    const auto* source = static_cast<const PictureView*>(selectionDragSource(mime));
    if (!isOpenView(source)) {
        drag->ignore();
        return true;
    }
    PictureView* target = dragTarget(watched, event);
    if (!target || source == target || !source->has_document()
        || !target->has_document()) {
        drag->ignore();
        return true;
    }
    if (event->type() == QEvent::Drop) {
        const int width = source->document_width();
        const int height = source->document_height();
        const auto coverage = selection_coverage_bytes(*source);
        if (coverage.size() != std::size_t(width) * std::size_t(height)) {
            drag->ignore();
            return true;
        }
        const ::rust::Slice<const std::uint8_t> bytes(coverage.data(), coverage.size());
        if (!load_selection_coverage_fit(*target, bytes, width, height, false,
                                         QStringLiteral("new"))) {
            drag->ignore();
            return true;
        }
        frame_->activateView(target);
        frame_->refresh();
    }
    drag->setDropAction(Qt::CopyAction);
    drag->accept();
    return true;
}

bool FileDropRouter::eventFilter(QObject* watched, QEvent* event)
{
    const bool dragEvent = event->type() == QEvent::DragEnter
        || event->type() == QEvent::DragMove || event->type() == QEvent::Drop;
    if (dragEvent
        && (routeLayerDrag(watched, event) || routeChannelDrag(watched, event)
            || routeSelectionDrag(watched, event))) {
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

#include "moc_file_drop_router.cpp"
