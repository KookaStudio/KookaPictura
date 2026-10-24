// Window > Arrange: floating and consolidating document windows, Cascade,
// Tile, and the Match commands. The ids are referenced as literals, so the generated
// command id header stays unchanged.

#include "frame_includes.h"

namespace pictura {

namespace {

const QString kArrangeCascade = QStringLiteral("window.arrange.cascade");
const QString kArrangeTile = QStringLiteral("window.arrange.tile");
const QString kArrangeConsolidate = QStringLiteral("window.arrange.consolidate.all.to.tabs");
const QString kArrangeFloat = QStringLiteral("window.arrange.float.in.window");
const QString kArrangeFloatAll = QStringLiteral("window.arrange.float.all.in.windows");
const QString kArrangeMatchZoom = QStringLiteral("window.arrange.match.zoom");
const QString kArrangeMatchLocation = QStringLiteral("window.arrange.match.location");
const QString kArrangeMatchAll = QStringLiteral("window.arrange.match.all");

// A cascaded window takes this share of the document area, and each one sits
// a header plus this gap below and right of the previous one.
constexpr double kCascadeAreaFraction = 0.6;
constexpr int kCascadeGap = 8;

} // namespace

void PicturaMainWindow::wireWindowArrange()
{
    const auto hasDocuments = [this]() { return !docs_.isEmpty(); };
    registry_->setHandler(kArrangeFloat, [this]() { floatWindow(activeWindow_); });
    registry_->setEnabledProvider(kArrangeFloat, [this]() {
        return activeWindow_ && windows_.contains(activeWindow_)
               && activeWindow_->kind != DocWindow::Host::Float && !activeWindow_->doc->loading;
    });
    registry_->setHandler(kArrangeFloatAll, [this]() { floatAllWindows(); });
    registry_->setEnabledProvider(kArrangeFloatAll, hasDocuments);
    registry_->setHandler(kArrangeConsolidate, [this]() { consolidateAllToTabs(nullptr); });
    registry_->setEnabledProvider(kArrangeConsolidate, hasDocuments);
    registry_->setHandler(kArrangeCascade, [this]() { cascadeWindows(); });
    registry_->setEnabledProvider(kArrangeCascade, hasDocuments);
    registry_->setHandler(kArrangeTile, [this]() { tileWindows(); });
    registry_->setEnabledProvider(kArrangeTile, [this]() { return windows_.size() > 1; });

    const auto canMatch = [this]() {
        return windows_.size() > 1 && activeWindow_ && activeWindow_->canvas;
    };
    registry_->setHandler(kArrangeMatchZoom, [this]() { matchWindows(true, false); });
    registry_->setEnabledProvider(kArrangeMatchZoom, canMatch);
    registry_->setHandler(kArrangeMatchLocation, [this]() { matchWindows(false, true); });
    registry_->setEnabledProvider(kArrangeMatchLocation, canMatch);
    registry_->setHandler(kArrangeMatchAll, [this]() { matchWindows(true, true); });
    registry_->setEnabledProvider(kArrangeMatchAll, canMatch);
}

QRect PicturaMainWindow::cascadeGeometry(int slot) const
{
    const QRect area = documentAreaGlobalRect();
    const QSize size = (QSizeF(area.size()) * kCascadeAreaFraction).toSize();
    const int step = DocumentFloatWindow::kHeaderHeight + kCascadeGap;
    // How many steps keep the whole header, inside the float's 1 px border,
    // within the area; the cascade restarts at the top-left after the last.
    const int acrossX = (area.width() - size.width()) / step + 1;
    const int acrossY = (area.height() - DocumentFloatWindow::kHeaderHeight - 2) / step + 1;
    const int offset = (qMax(0, slot) % qMax(1, qMin(acrossX, acrossY))) * step;
    return QRect(area.topLeft() + QPoint(offset, offset), size);
}

void PicturaMainWindow::floatAllWindows()
{
    DocWindow* active = activeWindow_;
    const QList<DocWindow*> windows = windows_;
    int slot = 0;
    for (DocWindow* window : windows) {
        if (window->kind != DocWindow::Host::Float
            && floatWindow(window, clampFloatGeometry(cascadeGeometry(slot)), false)) {
            ++slot;
        }
    }
    activateDocWindow(active);
}

void PicturaMainWindow::cascadeWindows()
{
    DocWindow* active = activeWindow_;
    const QList<DocWindow*> windows = windows_;
    QList<DocumentFloatWindow*> placed;
    for (DocWindow* window : windows) {
        const QRect geometry = clampFloatGeometry(cascadeGeometry(int(placed.size())));
        if (window->kind == DocWindow::Host::Float && window->floating) {
            window->floating->setGlobalGeometry(geometry);
        } else if (!floatWindow(window, geometry, false)) {
            // A loading document keeps its tab.
            continue;
        }
        placed << window->floating;
    }
    // Later windows overlap earlier ones, and the active window ends on top.
    for (DocumentFloatWindow* floating : placed) {
        raiseDocumentFloat(floating);
    }
    activateDocWindow(active);
}

void PicturaMainWindow::tileWindows()
{
    DocWindow* active = activeWindow_;
    const QList<DocWindow*> windows = windows_;
    // In cycle order, so the grid fills as Ctrl+Tab steps. A loading document
    // keeps its tab and joins the tiles once it is ready.
    for (DocWindow* window : windows) {
        tileWindow(window, false);
    }
    activateDocWindow(active);
}

void PicturaMainWindow::matchWindows(bool zoom, bool location)
{
    ImageView* source = activeWindow_ ? activeWindow_->canvas : nullptr;
    if (!source || source->documentSize().isEmpty()) {
        return;
    }
    // CS6's Match Location is unverified: the centre is matched as a fraction
    // of each document's size.
    const QSize sourceSize = source->documentSize();
    const QPointF centre = source->widgetToImage(QRectF(source->rect()).center());
    const QPointF fraction(centre.x() / sourceSize.width(), centre.y() / sourceSize.height());
    for (DocWindow* window : windows_) {
        ImageView* canvas = window->canvas;
        if (window == activeWindow_ || !canvas || window->doc->loading
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

} // namespace pictura
