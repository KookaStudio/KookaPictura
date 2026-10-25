#include "panel_column.h"

#include "panel_column_internal.h"

#include "frame.h"
#include "icons.h"
#include "panel_group.h"
#include "theme.h"

#include <QtCore/QEvent>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtGui/QAction>
#include <QtGui/QCursor>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QHideEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPaintEvent>
#include <QtGui/QPalette>
#include <QtGui/QResizeEvent>
#include <QtGui/QScreen>
#include <QtGui/QShowEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QBoxLayout>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGraphicsOpacityEffect>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QMenu>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

bool gForceChildOverlayForTest = false;

// A small bottom-right grip that resizes its owning `PanelFloat`. Unlike
// `QSizeGrip` it never touches the top-level window (the main window); it only
// grows or shrinks the overlay, clamped to the overlay's minimum size.
class PanelResizeGrip : public QWidget {
public:
    explicit PanelResizeGrip(PanelFloat* overlay)
        : QWidget(overlay), overlay_(overlay)
    {
        setObjectName(QStringLiteral("panelFloatSizeGrip"));
        setAttribute(Qt::WA_StyledBackground, true);
        setCursor(Qt::SizeFDiagCursor);
        setFixedSize(14, 14);
        setToolTip(QObject::tr("Resize"));
    }
    QSize sizeHint() const override { return QSize(14, 14); }

protected:
    void paintEvent(QPaintEvent*) override
    {
        // The classic diagonal grip lines, mirroring the native QSizeGrip.
        QPainter painter(this);
        painter.setPen(palette().color(QPalette::WindowText));
        for (int i = 0; i < 3; ++i) {
            const int offset = 3 + i * 4;
            painter.drawLine(width() - offset, height() - 2, width() - 2, height() - offset);
        }
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        if (event->button() != Qt::LeftButton) {
            return;
        }
        pressGlobal_ = event->globalPosition().toPoint();
        startSize_ = overlay_->size();
        event->accept();
    }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (!(event->buttons() & Qt::LeftButton)) {
            return;
        }
        const QPoint delta = event->globalPosition().toPoint() - pressGlobal_;
        const QSize next = (startSize_ + QSize(delta.x(), delta.y()))
                               .expandedTo(overlay_->minimumSize());
        overlay_->resize(next);
        event->accept();
    }

    void mouseReleaseEvent(QMouseEvent* event) override { event->accept(); }

private:
    PanelFloat* overlay_;
    QPoint pressGlobal_;
    QSize startSize_;
};

// A width-only edge grip for the persistent flyout. It resizes the flyout from
// its canvas-facing edge and keeps the opposite edge anchored.
class PanelEdgeGrip : public QWidget {
public:
    PanelEdgeGrip(PanelFlyout* flyout, bool leftEdge, QWidget* parent);
    void setLeftEdge(bool left) { leftEdge_ = left; }

protected:
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;

private:
    PanelFlyout* flyout_;
    bool leftEdge_;
    QPoint pressGlobal_;
    QRect startGeom_;
};

} // namespace

bool PanelFloat::overlayUsesTopLevel()
{
    if (gForceChildOverlayForTest) {
        return false;
    }
    return !QGuiApplication::platformName().contains(QStringLiteral("wayland"),
                                                     Qt::CaseInsensitive);
}

void PanelFloat::setForceChildOverlayForTest(bool force)
{
    gForceChildOverlayForTest = force;
}

bool PanelFloat::childOverlayForcedForTest()
{
    return gForceChildOverlayForTest;
}

PanelFlyout::PanelFlyout(QWidget* parent)
    : QWidget(parent)
{
    // The flyout is a persistent overlay, not a popup: outside presses must not
    // dismiss it. Mirror PanelFloat's platform-adaptive hosting so the top-level
    // window can be positioned freely and the child overlay stays in the frame.
    if (PanelFloat::overlayUsesTopLevel()) {
        setWindowFlags(Qt::Tool | Qt::FramelessWindowHint);
    } else {
        setWindowFlags(Qt::Widget);
    }
    setAttribute(Qt::WA_StyledBackground, true);
    // A panel flyout is a widget-column presentation; use the same width floor
    // as a docked column.
    setMinimumWidth(PanelFloat::kFloatMinWidth);
    resizeGrip_ = new PanelEdgeGrip(this, true, this);
    resizeGrip_->setObjectName(QStringLiteral("panelIconFlyoutResizeGrip"));
    resizeGrip_->hide();
}

void PanelFlyout::setResizeEdge(bool left)
{
    resizeLeft_ = left;
    if (auto* grip = static_cast<PanelEdgeGrip*>(resizeGrip_)) {
        grip->setLeftEdge(left);
    }
    updateResizeGrip();
}

void PanelFlyout::updateResizeGrip()
{
    if (!resizeGrip_) {
        return;
    }
    const int gripWidth = 4;
    resizeGrip_->setGeometry(resizeLeft_ ? QRect(0, 0, gripWidth, height())
                                         : QRect(width() - gripWidth, 0, gripWidth, height()));
    resizeGrip_->show();
    resizeGrip_->raise();
}

void PanelFlyout::hideEvent(QHideEvent* event)
{
    QWidget::hideEvent(event);
    if (onHidden) {
        onHidden();
    }
}

void PanelFlyout::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    updateResizeGrip();
}

PanelFloat::PanelFloat(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("panelFloat"));
    // Wayland ignores a client's `move()` of a top-level window, so an overlay
    // there must be an in-window child the parent can reposition. Everywhere
    // else it is a frameless `Qt::Tool` top-level parented to (transient for)
    // the main window: no title bar, no decorations, no taskbar entry, and it
    // may move anywhere on the screen. The parent keeps either kind above the
    // frame and hidden with it.
    if (overlayUsesTopLevel()) {
        setWindowFlags(Qt::Tool | Qt::FramelessWindowHint);
    } else {
        setWindowFlags(Qt::Widget);
    }
    setAttribute(Qt::WA_StyledBackground, true);
    setMinimumWidth(kFloatMinWidth);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(1, 1, 1, 1);
    layout->setSpacing(0);
    // M47: one shared effect dims the whole overlay while it hovers a valid drop
    // target; `setWindowOpacity` is a no-op on a child widget.
    opacityEffect_ = new QGraphicsOpacityEffect(this);
    opacityEffect_->setOpacity(1.0);
    setGraphicsEffect(opacityEffect_);
    // The grip resizes this overlay, never the main window.
    sizeGrip_ = new PanelResizeGrip(this);
}

void PanelFloat::setContent(QWidget* content)
{
    if (!content) {
        return;
    }
    content_ = content;
    if (auto* layout = qobject_cast<QBoxLayout*>(this->layout())) {
        layout->addWidget(content);
    }
}

void PanelFloat::setGroup(PanelGroup* group)
{
    group_ = group;
    if (group_) {
        setContent(group_);
        group_->setFloating(true);
        if (QToolButton* close = group_->floatCloseButton()) {
            QObject::connect(close, &QToolButton::clicked, this, [this]() {
                if (onClose) {
                    onClose();
                }
            });
        }
        QObject::connect(group_, &PanelGroup::collapsedToIconsChanged, this,
                         [this](bool) { syncToContent(); });
    }
}

void PanelFloat::setResizable(bool on)
{
    resizable_ = on;
    if (sizeGrip_) {
        sizeGrip_->setVisible(on);
    }
    if (on) {
        return;
    }
    // A non-resizable overlay takes the minimum its content needs. The tools
    // column drops the widget-column width floor so it can hug the tool grid.
    setMinimumWidth(0);
    setMinimumHeight(0);
    if (content_) {
        const int w = qMax(content_->minimumWidth(), content_->minimumSizeHint().width());
        const int h = qMax(content_->minimumHeight(), content_->minimumSizeHint().height());
        resize(qMax(1, w), qMax(1, h));
    }
}

void PanelFloat::syncToContent()
{
    if (group_) {
        if (group_->isCollapsedToIcons()) {
            // Icon mode hugs the icon row on both axes: hide the resize grip and
            // drop the normal-width floor, otherwise a body-wide residue is left
            // around the icons.
            if (sizeGrip_) {
                sizeGrip_->setVisible(false);
            }
            const int w = qMax(1, group_->sizeHint().width());
            const int h = qMax(group_->sizeHint().height(), kFloatIconMinHeight);
            setMinimumWidth(w);
            setMinimumHeight(h);
            resize(w, h);
            return;
        }
        // Expanded: restore the grip and the shared normal minimum width, keep a
        // top-bar + tab-bar height floor, and grow to fit the group.
        setMinimumWidth(kFloatMinWidth);
        setMinimumHeight(kFloatMinHeight);
        if (sizeGrip_) {
            sizeGrip_->setVisible(resizable_);
        }
        const int target = group_->sizeHint().height();
        if (target > height()) {
            resize(qMax(width(), kFloatMinWidth), target);
        }
        return;
    }
    // A whole-column overlay has no group; snap to the hosted column's content.
    auto* column = qobject_cast<PanelColumn*>(content_);
    if (!column) {
        return;
    }
    if (column->isToolsColumn()) {
        // A non-resizable tools overlay hugs its content: the 1<->2 column flip
        // changes the grid's width and height, so re-fit both axes. The re-fit
        // runs after the layout pass (deferred by the caller), so the hint is
        // the flipped arrangement's, not the previous mode's.
        if (content_->layout()) {
            content_->layout()->invalidate();
            content_->layout()->activate();
        }
        content_->updateGeometry();
        // The tools column wraps its content in a scroll area for the docked
        // frame; a floating overlay still hugs the full tool grid, so use the
        // column's size hint rather than the scroll area's small minimum.
        const int w = qMax(content_->minimumWidth(), content_->minimumSizeHint().width());
        const int h = qMax(content_->minimumHeight(), content_->sizeHint().height());
        setMinimumWidth(0);
        setMinimumHeight(0);
        resize(qMax(1, w), qMax(1, h));
        return;
    }
    if (!column->railMode()) {
        return;
    }
    setMinimumHeight(kFloatIconMinHeight);
    resize(qMax(kFloatMinWidth, column->minimumWidth()),
           qMax(column->minimumSizeHint().height(), kFloatIconMinHeight));
}

void PanelFloat::setDragDimmed(bool dimmed)
{
    if (opacityEffect_) {
        opacityEffect_->setOpacity(dimmed ? 0.6 : 1.0);
    }
}

qreal PanelFloat::dragOpacityForTest() const
{
    return opacityEffect_ ? opacityEffect_->opacity() : 1.0;
}

void PanelFloat::showTabIndicator(PanelGroup* group, int index)
{
    QTabBar* bar = group ? group->tabBar() : nullptr;
    if (!bar) {
        return;
    }
    if (!indicator_) {
        // Same `#2a7fff` mark as the docked column's indicator, owned by the
        // float because the docked indicator cannot reach outside its viewport.
        indicator_ = new QWidget(this);
        indicator_->setObjectName(QStringLiteral("panelFloatDropIndicator"));
        indicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    }
    const int x = group->tabInsertionX(index);
    const QPoint origin = bar->mapTo(this, QPoint(x, 0));
    indicator_->setGeometry(QRect(origin.x() - 1, origin.y(), 3, bar->height()));
    indicator_->show();
    indicator_->raise();
}

void PanelFloat::hideTabIndicator()
{
    if (indicator_) {
        indicator_->hide();
    }
}

void PanelFloat::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    if (!sizeGrip_) {
        return;
    }
    sizeGrip_->resize(sizeGrip_->sizeHint());
    sizeGrip_->move(width() - sizeGrip_->width() - 2, height() - sizeGrip_->height() - 2);
    sizeGrip_->raise();
}

PanelEdgeGrip::PanelEdgeGrip(PanelFlyout* flyout, bool leftEdge, QWidget* parent)
    : QWidget(parent)
    , flyout_(flyout)
    , leftEdge_(leftEdge)
{
    setCursor(Qt::SizeHorCursor);
    setAttribute(Qt::WA_NoSystemBackground, true);
    setFixedWidth(4);
}

void PanelEdgeGrip::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton || !flyout_) {
        return;
    }
    pressGlobal_ = event->globalPosition().toPoint();
    startGeom_ = flyout_->geometry();
    event->accept();
}

void PanelEdgeGrip::mouseMoveEvent(QMouseEvent* event)
{
    if (!flyout_ || !(event->buttons() & Qt::LeftButton)) {
        return;
    }
    const int delta = event->globalPosition().toPoint().x() - pressGlobal_.x();
    int maxWidth = 0;
    if (PanelFloat::overlayUsesTopLevel()) {
        QScreen* screen = flyout_->screen();
        if (!screen) {
            screen = QGuiApplication::primaryScreen();
        }
        const QRect bounds = screen ? screen->availableGeometry() : QRect();
        maxWidth = leftEdge_ ? startGeom_.right() - bounds.left() + 1
                             : bounds.right() - startGeom_.left() + 1;
    } else {
        QWidget* host = flyout_->parentWidget();
        const QRect bounds = host ? host->rect() : QRect();
        maxWidth = leftEdge_ ? startGeom_.right() - bounds.left() + 1
                             : bounds.right() - startGeom_.left() + 1;
    }
    const int minimum = qMax(1, flyout_->minimumWidth());
    const int width = qBound(minimum,
                             startGeom_.width() + (leftEdge_ ? -delta : delta),
                             qMax(minimum, maxWidth));
    flyout_->resize(width, startGeom_.height());
    if (leftEdge_) {
        flyout_->move(startGeom_.right() - width + 1, startGeom_.top());
    }
    event->accept();
}

void PanelEdgeGrip::mouseReleaseEvent(QMouseEvent* event)
{
    event->accept();
}

} // namespace pictura
