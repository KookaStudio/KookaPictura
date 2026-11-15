// Window > Arrange and the Hand/Zoom tools' multi-window options.

#include "document_registry.h"

#include "document_float_window.h"
#include "image_view.h"

#include <QtCore/QtMath>
#include <QtCore/QSizeF>

namespace pictura {

namespace {

constexpr double kCascadeAreaFraction = 0.6;
constexpr int kCascadeGap = 8;

} // namespace

QRect DocumentRegistry::cascadeGeometry(int slot) const
{
    const QRect area = documentAreaGlobalRect();
    const QSize size = (QSizeF(area.size()) * kCascadeAreaFraction).toSize();
    const int step = DocumentFloatWindow::kHeaderHeight + kCascadeGap;
    const int acrossX = (area.width() - size.width()) / step + 1;
    const int acrossY = (area.height() - DocumentFloatWindow::kHeaderHeight - 2) / step + 1;
    const int offset = (qMax(0, slot) % qMax(1, qMin(acrossX, acrossY))) * step;
    return QRect(area.topLeft() + QPoint(offset, offset), size);
}

void DocumentRegistry::floatAllWindows()
{
    DocWindow* active = activeWindow();
    const QList<DocWindow*> windows = this->windows();
    int slot = 0;
    for (DocWindow* window : windows) {
        if (window->kind != DocWindow::Host::Float
            && floatWindow(window, clampFloatGeometry(cascadeGeometry(slot)), false)) {
            ++slot;
        }
    }
    activateDocWindow(active);
}

void DocumentRegistry::cascadeWindows()
{
    DocWindow* active = activeWindow();
    const QList<DocWindow*> windows = this->windows();
    QList<DocumentFloatWindow*> placed;
    for (DocWindow* window : windows) {
        const QRect geometry = clampFloatGeometry(cascadeGeometry(int(placed.size())));
        if (window->kind == DocWindow::Host::Float && window->floating) {
            window->floating->setGlobalGeometry(geometry);
        } else if (!floatWindow(window, geometry, false)) {
            continue;
        }
        placed << window->floating;
    }
    for (DocumentFloatWindow* floating : placed) {
        raiseDocumentFloat(floating);
    }
    activateDocWindow(active);
}

void DocumentRegistry::tileWindows()
{
    DocWindow* active = activeWindow();
    const QList<DocWindow*> windows = this->windows();
    for (DocWindow* window : windows) {
        tileWindow(window, false);
    }
    activateDocWindow(active);
}

void DocumentRegistry::matchWindows(bool zoom, bool location)
{
    DocWindow* active = activeWindow();
    ImageView* source = active ? active->canvas : nullptr;
    if (!source || source->documentSize().isEmpty()) {
        return;
    }
    const QSize sourceSize = source->documentSize();
    const QPointF centre = source->widgetToImage(QRectF(source->rect()).center());
    const QPointF fraction(centre.x() / sourceSize.width(), centre.y() / sourceSize.height());
    for (DocWindow* window : windows()) {
        ImageView* canvas = window->canvas;
        if (window == active || !canvas || window->doc->loading
            || canvas->documentSize().isEmpty()) {
            continue;
        }
        if (zoom) {
            canvas->setZoom(source->zoom(), QRectF(canvas->rect()).center());
        }
        if (location) {
            const QSize size = canvas->documentSize();
            const QPointF target(fraction.x() * size.width(), fraction.y() * size.height());
            canvas->centerOnImageRect(QRectF(target - QPointF(0.5, 0.5), QSizeF(1.0, 1.0)));
        }
    }
}

void DocumentRegistry::scrollOtherWindows(const ImageView* source, const QPointF& delta,
                                          Qt::KeyboardModifiers mods)
{
    if (scrollingAllWindows_) {
        return;
    }
    if (!mods.testFlag(Qt::ShiftModifier) && !(hooks_.scrollAllWindows && hooks_.scrollAllWindows())) {
        return;
    }
    scrollingAllWindows_ = true;
    for (DocWindow* window : windows()) {
        ImageView* canvas = window->canvas;
        if (canvas && canvas != source && canvas->isVisible() && !window->doc->loading) {
            canvas->panBy(delta);
        }
    }
    scrollingAllWindows_ = false;
}

void DocumentRegistry::onToolZoomed(double factor, bool allWindows, bool resizeToFit)
{
    DocWindow* zoomed = windowOfCanvas(hooks_.toolCanvas ? hooks_.toolCanvas() : nullptr);
    if (!zoomed) {
        return;
    }
    if (allWindows) {
        for (DocWindow* window : windows()) {
            ImageView* canvas = window->canvas;
            if (window == zoomed || !canvas || window->doc->loading
                || canvas->documentSize().isEmpty()) {
                continue;
            }
            canvas->setZoom(canvas->zoom() * factor, QRectF(canvas->rect()).center());
        }
    }
    if (resizeToFit) {
        resizeWindowToFit(zoomed);
    }
}

void DocumentRegistry::resizeWindowToFit(DocWindow* window)
{
    if (!window || window->kind != DocWindow::Host::Float || !window->floating
        || !window->canvas) {
        return;
    }
    ImageView* canvas = window->canvas;
    const QSize document = canvas->documentSize();
    const QRect area = documentAreaGlobalRect();
    if (document.isEmpty() || area.isEmpty()) {
        return;
    }
    QRect geometry = window->floating->globalGeometry();
    const QSize minimum = window->floating->minimumSizeHint();
    const QSize chrome = (minimum - canvas->minimumSize()).expandedTo(QSize(0, 0));
    const QSize image(qCeil(document.width() * canvas->zoom()),
                      qCeil(document.height() * canvas->zoom()));
    geometry.setSize((image + chrome).expandedTo(minimum).boundedTo(area.size()));
    if (geometry.right() > area.right()) {
        geometry.moveRight(area.right());
    }
    if (geometry.bottom() > area.bottom()) {
        geometry.moveBottom(area.bottom());
    }
    window->floating->setGlobalGeometry(clampFloatGeometry(geometry));
}

} // namespace pictura
