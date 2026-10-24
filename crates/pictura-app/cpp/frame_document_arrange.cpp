// Window > Arrange: floating and consolidating document windows, Cascade,
// Tile, the Match commands, and New Window For; and the Hand and Zoom tools'
// multi-window options. The ids are referenced as literals, so the generated
// command id header stays unchanged.

#include "frame_includes.h"

#include <QtCore/QtMath>

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
const QString kArrangeNewWindow = QStringLiteral("window.arrange.new.window.for.file");

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

    registry_->setHandler(kArrangeNewWindow, [this]() {
        if (activeWindow_) {
            addDocumentWindow(activeWindow_->doc);
        }
    });
    registry_->setEnabledProvider(kArrangeNewWindow, [this]() {
        return activeWindow_ && !activeWindow_->doc->loading;
    });
    // CS6 names the active document in the entry; `&` stays literal.
    registry_->setLabelProvider(kArrangeNewWindow, [this]() {
        const QString label = tr("New Window For");
        if (!activeWindow_) {
            return label;
        }
        QString name = documentDisplayName(*activeWindow_->doc);
        return label + QLatin1Char(' ') + name.replace(QLatin1Char('&'), QStringLiteral("&&"));
    });
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

void PicturaMainWindow::scrollOtherWindows(const ImageView* source, const QPointF& delta,
                                           Qt::KeyboardModifiers mods)
{
    if (scrollingAllWindows_) {
        return;
    }
    const HandGroup* hand = tools_ ? tools_->optionsFor<HandGroup>(ToolId::Hand) : nullptr;
    if (!mods.testFlag(Qt::ShiftModifier) && !(hand && hand->get().scrollAll)) {
        return;
    }
    scrollingAllWindows_ = true;
    for (DocWindow* window : windows_) {
        ImageView* canvas = window->canvas;
        if (canvas && canvas != source && canvas->isVisible() && !window->doc->loading) {
            canvas->panBy(delta);
        }
    }
    scrollingAllWindows_ = false;
}

void PicturaMainWindow::onToolZoomed(double factor, bool allWindows, bool resizeToFit)
{
    DocWindow* zoomed = windowOfCanvas(tools_ ? tools_->canvas() : nullptr);
    if (!zoomed) {
        return;
    }
    if (allWindows) {
        for (DocWindow* window : windows_) {
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

void PicturaMainWindow::resizeWindowToFit(DocWindow* window)
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
    // The header, scroll bars, and border around the canvas keep their size.
    const QSize chrome = geometry.size() - canvas->size();
    const QSize image(qCeil(document.width() * canvas->zoom()),
                      qCeil(document.height() * canvas->zoom()));
    geometry.setSize((image + chrome).boundedTo(area.size()));
    if (geometry.right() > area.right()) {
        geometry.moveRight(area.right());
    }
    if (geometry.bottom() > area.bottom()) {
        geometry.moveBottom(area.bottom());
    }
    window->floating->setGlobalGeometry(clampFloatGeometry(geometry));
}

} // namespace pictura
