#include "toolbox.h"

#include "color_picker_dialog.h"
#include "icons.h"
#include "image_view.h"
#include "panels/color_panel.h"
#include "theme.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include <QtCore/QEvent>
#include <QtCore/QSize>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QKeySequence>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPolygon>
#include <QtGui/QScreen>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QApplication>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QSizePolicy>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <functional>

namespace pictura {

namespace {

constexpr int kSwatchSize = 22;
constexpr int kWidgetSize = 40;
constexpr int kMinWidgetSize = 20;
constexpr int kResetSize = 12;
constexpr int kFlyoutDelayMs = 300;

// Slot geometry is derived from the one 96-DPI base in Theme; Qt6 applies the
// device-pixel-ratio transform itself, so we scale only by logical DPI.
qreal slotDpiScale(qreal logicalDpi)
{
    return std::clamp(logicalDpi / 96.0, 1.0, 4.0);
}

// A slot button: the base tool button plus the CS6 interactions the stock
// class lacks — a custom lower-right flyout triangle, right-click and
// press-and-hold open the flyout, Alt-click cycles the group's implemented
// members.
class ToolSlotButton : public QToolButton {
public:
    explicit ToolSlotButton(QWidget* parent = nullptr)
        : QToolButton(parent)
    {
        holdTimer_.setSingleShot(true);
        holdTimer_.setInterval(kFlyoutDelayMs);
        connect(&holdTimer_, &QTimer::timeout, this, [this]() {
            held_ = true;
            if (onMenu) {
                onMenu();
            }
            // The popup owns the pending release, so the button would otherwise
            // stay drawn pressed; clear the down state now.
            setDown(false);
        });
    }

    void setHasFlyout(bool hasFlyout)
    {
        hasFlyout_ = hasFlyout;
        update();
    }
    bool hasFlyout() const { return hasFlyout_; }

    std::function<void()> onMenu;
    std::function<void()> onCycle;

protected:
    void paintEvent(QPaintEvent* event) override
    {
        QToolButton::paintEvent(event);
        if (!hasFlyout_) {
            return;
        }
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setOpacity(0.6);
        painter.setPen(Qt::NoPen);
        painter.setBrush(palette().color(QPalette::ButtonText));
        const int size = 4;
        const int margin = 2;
        QPolygon triangle;
        triangle << QPoint(width() - margin - size, height() - margin)
                 << QPoint(width() - margin, height() - margin)
                 << QPoint(width() - margin, height() - margin - size);
        painter.drawPolygon(triangle);
    }

    void mousePressEvent(QMouseEvent* event) override
    {
        if (event->button() == Qt::RightButton) {
            // Open the menu on release, not press: a popup shown during the
            // press can be dismissed by the matching release on some platforms.
            rightPressed_ = true;
            event->accept();
            return;
        }
        if (event->button() == Qt::LeftButton
            && event->modifiers().testFlag(Qt::AltModifier)) {
            if (onCycle) {
                onCycle();
            }
            event->accept();
            return;
        }
        if (event->button() == Qt::LeftButton) {
            held_ = false;
            holdTimer_.start();
        }
        QToolButton::mousePressEvent(event);
    }

    void mouseReleaseEvent(QMouseEvent* event) override
    {
        holdTimer_.stop();
        if (event->button() == Qt::RightButton && rightPressed_) {
            rightPressed_ = false;
            if (onMenu) {
                onMenu();
            }
            event->accept();
            return;
        }
        if (held_) {
            held_ = false;
            event->accept();
            return;
        }
        QToolButton::mouseReleaseEvent(event);
    }

private:
    QTimer holdTimer_;
    bool held_ = false;
    bool rightPressed_ = false;
    bool hasFlyout_ = false;
};

QString enabledTooltip(const ToolInfo& info)
{
    return QStringLiteral("%1 (%2) — %3")
        .arg(QString::fromLatin1(info.label), QString(toolShortcut(info.id)),
             QString::fromLatin1(info.hint));
}

QString disabledTooltip(const ToolInfo& info)
{
    return QStringLiteral("%1 — not implemented yet").arg(QString::fromLatin1(info.label));
}

int groupMemberCount(int group)
{
    int count = 0;
    for (ToolId id : allToolIds()) {
        if (toolInfo(id).group == group) {
            ++count;
        }
    }
    return count;
}

bool groupHasImplemented(int group)
{
    for (ToolId id : allToolIds()) {
        if (toolInfo(id).group == group && toolImplemented(id)) {
            return true;
        }
    }
    return false;
}

} // namespace

ForegroundBackgroundWidget::ForegroundBackgroundWidget(ColorState* state, QWidget* parent)
    : QWidget(parent)
    , state_(state)
{
    setMinimumSize(kMinWidgetSize, kMinWidgetSize);
    setToolTip(tr("Foreground / background colors — click a swatch to make it active"));
    if (state_) {
        connect(state_, &ColorState::foregroundChanged, this, [this](const QColor&) { update(); });
        connect(state_, &ColorState::backgroundChanged, this, [this](const QColor&) { update(); });
        connect(state_, &ColorState::activeChanged, this, [this](bool) { update(); });
    }
}

void ForegroundBackgroundWidget::setSide(int side)
{
    // ponytail: square only; a height-for-width layout is overkill for a pair of
    // swatches. Clamped so the two squares stay readable at the smallest column.
    const int clamped = std::clamp(side, kMinWidgetSize, kWidgetSize);
    if (clamped == width() && clamped == height()) {
        return;
    }
    setFixedSize(clamped, clamped);
}

int ForegroundBackgroundWidget::swatchSize() const
{
    return std::clamp(width() * 55 / 100, 12, kSwatchSize);
}

int ForegroundBackgroundWidget::resetSize() const
{
    return std::clamp(width() * 30 / 100, 7, kResetSize);
}

QRect ForegroundBackgroundWidget::foregroundRect() const
{
    const int size = swatchSize();
    return QRect(1, 1, size, size);
}

QRect ForegroundBackgroundWidget::backgroundRect() const
{
    const int size = swatchSize();
    return QRect(width() - size - 1, height() - size - 1, size, size);
}

QRect ForegroundBackgroundWidget::resetRect() const
{
    const int size = resetSize();
    return QRect(2, height() - size - 2, size, size);
}

QRect ForegroundBackgroundWidget::swapRect() const
{
    // Top-right corner, clear of the default-colour reset at bottom-left.
    const int size = resetSize();
    return QRect(width() - size - 2, 2, size, size);
}

void ForegroundBackgroundWidget::swapForegroundBackground()
{
    if (state_) {
        const QColor fg = state_->foreground();
        state_->setForeground(state_->background());
        state_->setBackground(fg);
    }
    ++swapCount_;
    update();
}

QColor ForegroundBackgroundWidget::foregroundForTest() const
{
    return state_ ? state_->foreground() : QColor(Qt::black);
}

QColor ForegroundBackgroundWidget::backgroundForTest() const
{
    return state_ ? state_->background() : QColor(Qt::white);
}

void ForegroundBackgroundWidget::resetColors()
{
    if (state_) {
        const bool mask = state_->maskActive();
        state_->setForeground(mask ? Qt::white : Qt::black);
        state_->setBackground(mask ? Qt::black : Qt::white);
    }
    emit reset();
}

void ForegroundBackgroundWidget::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    // No background fill: the panel shows through around the swatches.
    const QColor fg = state_ ? state_->foreground() : QColor(Qt::black);
    const QColor bg = state_ ? state_->background() : QColor(Qt::white);
    const bool fgActive = !state_ || state_->foregroundActive();

    auto drawSwatch = [&painter](const QRect& r, const QColor& color, bool active) {
        painter.fillRect(r, color);
        const QColor border =
            active ? (color.lightness() > 128 ? QColor(20, 20, 20) : QColor(240, 240, 240))
                   : QColor(90, 90, 90);
        painter.setPen(QPen(border, active ? 2 : 1));
        painter.drawRect(r);
    };

    drawSwatch(backgroundRect(), bg, !fgActive);
    drawSwatch(foregroundRect(), fg, fgActive);

    const QRect reset = resetRect();
    painter.fillRect(reset, QColor(Qt::white));
    painter.setPen(QColor(40, 40, 40));
    painter.drawRect(reset.adjusted(0, 0, -1, -1));
    painter.fillRect(QRect(reset.topLeft(), QSize(reset.width() / 2, reset.height() / 2)),
                     QColor(Qt::black));

    // CS6 close-out: swap arrows sourced at the top-right corner — one runs
    // right to left, the other up to down. No asset; the theme ink draws them.
    const QRect swap = swapRect();
    painter.setPen(QPen(palette().color(QPalette::ButtonText), 1));
    const int srcX = swap.right() - 1;
    const int srcY = swap.top() + 1;
    const int leftX = swap.left() + 1;
    const int downY = swap.bottom() - 1;
    painter.drawLine(srcX, srcY, leftX, srcY);
    painter.drawLine(leftX, srcY, leftX + 2, srcY - 2);
    painter.drawLine(leftX, srcY, leftX + 2, srcY + 2);
    painter.drawLine(srcX, srcY, srcX, downY);
    painter.drawLine(srcX, downY, srcX - 2, downY - 2);
    painter.drawLine(srcX, downY, srcX + 2, downY - 2);
}

void ForegroundBackgroundWidget::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton || !state_) {
        QWidget::mousePressEvent(event);
        return;
    }
    const QPoint pos = event->position().toPoint();
    if (resetRect().contains(pos)) {
        resetColors();
        return;
    }
    if (swapRect().contains(pos)) {
        swapForegroundBackground();
        return;
    }
    if (foregroundRect().contains(pos)) {
        state_->setForegroundActive(true);
        const QColor picked = ColorPickerDialog::getColor(state_->foreground(), this,
                                                          tr("Foreground Color"));
        if (picked.isValid()) {
            state_->setForeground(picked);
        }
    } else if (backgroundRect().contains(pos)) {
        state_->setForegroundActive(false);
        const QColor picked = ColorPickerDialog::getColor(state_->background(), this,
                                                          tr("Background Color"));
        if (picked.isValid()) {
            state_->setBackground(picked);
        }
    } else {
        event->ignore();
        return;
    }
    emit clicked();
}

Toolbox::Toolbox(ToolController* controller, ColorState* colors, QWidget* parent)
    : QWidget(parent)
    , controller_(controller)
    , colors_(colors)
{
    setObjectName(QStringLiteral("toolsPanel"));
    setAttribute(Qt::WA_StyledBackground, true);

    auto* layout = new QVBoxLayout(this);
    bodyLayout_ = layout;
    // Track the body's content exactly so a 2->1 column change lowers the
    // content's minimum width instead of leaving the two-column floor behind.
    layout->setSizeConstraint(QLayout::SetMinimumSize);
    layout->setContentsMargins(2, 2, 2, 2);
    layout->setSpacing(4);

    gridWidget_ = new QWidget(this);
    grid_ = new QGridLayout(gridWidget_);
    grid_->setContentsMargins(0, 0, 0, 0);
    grid_->setSpacing(1);

    auto* group = new QButtonGroup(this);
    group->setExclusive(true);

    applyMetrics();

    for (int g = 1; g <= 23; ++g) {
        QList<ToolId> members;
        bool anyImplemented = false;
        for (ToolId id : allToolIds()) {
            if (toolInfo(id).group == g) {
                members << id;
                anyImplemented = anyImplemented || toolImplemented(id);
            }
        }
        if (members.isEmpty()) {
            continue;
        }

        // Hidden groups (all-unimplemented: Object/Camera) build no slot button,
        // so the toolbox drops exactly those slots. Their flyout menu is still
        // built for the self-test's disabled-member checks.
        QMenu* menu = nullptr;
        if (members.size() > 1) {
            menu = new QMenu(this);
            menu->setObjectName(QStringLiteral("toolFlyoutMenu"));
            for (ToolId member : members) {
                const ToolInfo& info = toolInfo(member);
                QAction* action =
                    menu->addAction(icon(QStringLiteral("tool.") + toolIdName(member)),
                                    QString::fromLatin1(info.label));
                action->setData(int(member));
                action->setEnabled(info.implemented);
                action->setToolTip(info.implemented ? enabledTooltip(info)
                                                    : disabledTooltip(info));
                if (!toolShortcut(member).isNull()) {
                    action->setShortcut(QKeySequence(QString(toolShortcut(member))));
                    action->setShortcutVisibleInContextMenu(true);
                    action->setShortcutContext(Qt::WidgetWithChildrenShortcut);
                }
            }
            connect(menu, &QMenu::triggered, this, [this, g](QAction* action) {
                selectMember(g, ToolId(action->data().toInt()));
            });
            connect(menu, &QMenu::aboutToHide, this, [this, menu]() {
                if (openSlotMenu_ == menu) {
                    closeOpenSlotMenu();
                }
            });
        }
        // The Zoom slot is a single tool, so its right-click menu is the CS6
        // zoom-preset menu rather than a flyout of members.
        if (members.size() == 1 && members.contains(ToolId::Zoom)) {
            menu = buildZoomMenu();
            zoomMenu_ = menu;
        }
        slotMenuByGroup_[g] = menu;

        if (!anyImplemented) {
            continue;
        }

        currentByGroup_[g] = groupCurrentTool(g);
        auto* button = new ToolSlotButton(gridWidget_);
        button->setObjectName(QStringLiteral("toolSlotButton"));
        button->setIconSize(iconSize_);
        button->setFixedSize(slotSize_);
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setEnabled(anyImplemented);
        button->setHasFlyout(members.size() > 1);

        button->onMenu = [this, g]() { showSlotMenu(g); };
        button->onCycle = [this, g]() { cycleGroup(g); };

        slotButtons_ << button;
        slotGroups_ << g;
        slotButtonByGroup_[g] = button;
        group->addButton(button);

        refreshSlot(g);
        if (controller_) {
            connect(button, &QToolButton::clicked, this, [this, g]() {
                const ToolId current = groupCurrentTool(g);
                if (toolImplemented(current)) {
                    controller_->setActiveTool(current);
                }
            });
        }
    }
    reflow();

    layout->addWidget(gridWidget_, 0, Qt::AlignHCenter);

    fgbg_ = new ForegroundBackgroundWidget(colors, this);
    layout->addWidget(fgbg_, 0, Qt::AlignHCenter);

    paintMask_ = new QToolButton(this);
    paintMask_->setObjectName(QStringLiteral("paintMaskButton"));
    paintMask_->setCheckable(true);
    paintMask_->setAutoRaise(true);
    const QIcon paintMaskIcon = icon(QStringLiteral("layers.mask"));
    if (!paintMaskIcon.isNull()) {
        paintMask_->setIcon(paintMaskIcon);
    } else {
        paintMask_->setText(QStringLiteral("Q"));
    }
    paintMask_->setToolTip(tr("Edit in Quick Mask Mode"));
    // ponytail: button stub, Quick Mask behavior tracked in #172
    connect(paintMask_, &QToolButton::toggled, this,
            [this](bool checked) { emit paintMaskToggled(checked); });

    screenMode_ = new QToolButton(this);
    screenMode_->setObjectName(QStringLiteral("screenModeButton"));
    screenMode_->setAutoRaise(true);
    const QIcon screenModeIcon = icon(QStringLiteral("view.screenMode.standard"));
    if (!screenModeIcon.isNull()) {
        screenMode_->setIcon(screenModeIcon);
    } else {
        screenMode_->setText(QStringLiteral("Screen Mode"));
    }
    screenMode_->setToolTip(tr("Screen Mode"));

    // The Quick Mask toggle sits immediately left of the screen-mode control.
    // Both are compact icon buttons (not full slots); `contentWidth` measures
    // this row so the tools panel is only as wide as the grid or the footer.
    footerRow_ = new QHBoxLayout();
    footerRow_->setContentsMargins(0, 0, 0, 0);
    footerRow_->setSpacing(2);
    footerRow_->addStretch(1);
    footerRow_->addWidget(paintMask_);
    footerRow_->addWidget(screenMode_);
    footerRow_->addStretch(1);
    layout->addLayout(footerRow_);

    layout->addStretch(1);

    applyMetrics();
    updateContentMetrics();

    if (controller_) {
        connect(controller_, &ToolController::activeToolChanged, this, [this](ToolId id) {
            const int g = toolInfo(id).group;
            currentByGroup_[g] = id;
            refreshSlot(g);
        });
    }
}

Toolbox::~Toolbox()
{
    // `showSlotMenu` installs `this` as a qApp event filter; drop it (and any
    // open popup) even if Qt also unregisters on teardown.
    closeOpenSlotMenu();
}

QSize Toolbox::slotSizeForDpi(qreal logicalDpi)
{
    const qreal scale = slotDpiScale(logicalDpi);
    return QSize(qRound(Theme::kSlotBaseW * scale), qRound(Theme::kSlotBaseH * scale));
}

QSize Toolbox::iconSizeForDpi(qreal logicalDpi)
{
    const qreal scale = slotDpiScale(logicalDpi);
    return QSize(qRound(Theme::kSlotIconMaxW * scale), qRound(Theme::kSlotIconMaxH * scale));
}

void Toolbox::setScreenModeActions(const QList<QAction*>& actions)
{
    if (!screenMode_ || actions.isEmpty()) {
        return;
    }
    auto* menu = new QMenu(screenMode_);
    menu->setObjectName(QStringLiteral("toolScreenModeMenu"));
    for (QAction* action : actions) {
        if (action) {
            // Display only: a trailing tab right-aligns the `F` hint Qt renders
            // as an accelerator column. `F` still cycles in the frame; this
            // adds no key binding. Guarded so re-calling does not double it.
            if (!action->text().endsWith(QStringLiteral("\tF"))) {
                action->setText(action->text() + QStringLiteral("\tF"));
            }
            menu->addAction(action);
        }
    }
    screenModeActions_ = actions;
    screenMode_->setMenu(menu);
    screenMode_->setPopupMode(QToolButton::InstantPopup);
    setActiveScreenMode(0);
}

void Toolbox::setActiveScreenMode(int mode)
{
    if (!screenMode_ || mode < 0 || mode >= screenModeActions_.size()) {
        return;
    }
    QAction* action = screenModeActions_.at(mode);
    if (action && !action->icon().isNull()) {
        screenMode_->setIcon(action->icon());
    }
}

void Toolbox::applyMetrics()
{
    const qreal dpi = screen() ? screen()->logicalDotsPerInch() : 96.0;
    slotSize_ = slotSizeForDpi(dpi);
    iconSize_ = iconSizeForDpi(dpi);
    for (QToolButton* button : slotButtons_) {
        button->setFixedSize(slotSize_);
        button->setIconSize(iconSize_);
    }
    layoutFooter();
    updateContentMetrics();
}

void Toolbox::layoutFooter()
{
    if (footerRow_) {
        // A single column of tools stacks the Quick Mask / Screen Mode pair
        // vertically; two columns keep the compact horizontal row. The 2-column
        // gutter matches the grid spacing so the pair fits the grid width.
        footerRow_->setDirection(columns_ == 1 ? QBoxLayout::TopToBottom
                                               : QBoxLayout::LeftToRight);
        footerRow_->setSpacing(columns_ == 1 ? 2 : (grid_ ? grid_->spacing() : 1));
    }
    // Both controls track a tool slot's size in either column mode, so the
    // footer never renders a shrunken button beside full-size slots.
    if (paintMask_) {
        paintMask_->setFixedSize(slotSize_);
        paintMask_->setIconSize(iconSize_);
    }
    if (screenMode_) {
        screenMode_->setFixedSize(slotSize_);
        screenMode_->setIconSize(iconSize_);
    }
}

bool Toolbox::event(QEvent* event)
{
    // A screen change or a re-host (column float/re-place) re-derives the
    // metrics, so the fixed sizes track the destination DPI and no seam lingers.
    if (event->type() == QEvent::ScreenChangeInternal
        || event->type() == QEvent::ParentChange) {
        applyMetrics();
    }
    return QWidget::event(event);
}

void Toolbox::setColumns(int columns)
{
    columns = columns == 2 ? 2 : 1;
    if (columns == columns_) {
        return;
    }
    columns_ = columns;
    reflow();
    layoutFooter();
    updateContentMetrics();
    // The grid was rebuilt synchronously; force the layout chain to measure it
    // now so a floating tools column's deferred re-fit reads the new
    // arrangement's height instead of the previous mode's.
    if (grid_) {
        grid_->invalidate();
        grid_->activate();
    }
    if (gridWidget_) {
        gridWidget_->updateGeometry();
    }
    updateGeometry();
    if (bodyLayout_) {
        bodyLayout_->invalidate();
        bodyLayout_->activate();
    }
    emit columnsChanged(columns_);
}

int Toolbox::contentWidth(int columns) const
{
    const int slot = slotSize_.width();
    const int spacing = grid_ ? grid_->spacing() : 0;
    const int gridWidth = columns * slot + (columns > 1 ? (columns - 1) * spacing : 0);
    // The compact footer (Quick Mask + Screen Mode) shares the body's width, so
    // the content is as wide as the grid or the footer, whichever is wider.
    const int footerWidth = footerRow_ ? footerRow_->minimumSize().width() : 0;
    const int bodyWidth = qMax(gridWidth, footerWidth);
    const int margins = bodyLayout_
        ? bodyLayout_->contentsMargins().left() + bodyLayout_->contentsMargins().right()
        : 0;
    return bodyWidth + margins;
}

void Toolbox::updateContentMetrics()
{
    // The content's minimum width follows the tool grid or the compact footer,
    // whichever is wider; the hosting PanelColumn tracks it so the column hugs.
    const int content = contentWidth(columns_);
    setMinimumWidth(content);
    if (fgbg_) {
        fgbg_->setSide(content - (bodyLayout_ ? bodyLayout_->contentsMargins().left()
                                                   + bodyLayout_->contentsMargins().right()
                                               : 0));
    }
    if (QWidget* body = this) {
        body->updateGeometry();
    }
}

int Toolbox::contentWidthForTest() const
{
    return contentWidth(columns_);
}

int Toolbox::foregroundBackgroundWidthForTest() const
{
    return fgbg_ ? fgbg_->width() : 0;
}

void Toolbox::swapForegroundBackground()
{
    if (fgbg_) {
        fgbg_->swapForegroundBackground();
    }
}

void Toolbox::resetForegroundBackground()
{
    if (fgbg_) {
        fgbg_->resetColors();
    }
}

void Toolbox::reflow()
{
    while (QLayoutItem* item = grid_->takeAt(0)) {
        delete item;
    }
    for (int i = 0; i < slotButtons_.size(); ++i) {
        if (columns_ == 2) {
            grid_->addWidget(slotButtons_.at(i), i / 2, i % 2, Qt::AlignHCenter);
        } else {
            grid_->addWidget(slotButtons_.at(i), i, 0, Qt::AlignHCenter);
        }
    }
}

QMenu* Toolbox::buildZoomMenu()
{
    auto* menu = new QMenu(this);
    menu->setObjectName(QStringLiteral("zoomToolMenu"));

    const auto add = [this, menu](const QString& text, void (ImageView::*fn)()) {
        QAction* action = menu->addAction(text);
        connect(action, &QAction::triggered, this, [this, fn]() {
            if (ImageView* canvas = controller_ ? controller_->canvas() : nullptr) {
                (canvas->*fn)();
            }
        });
        return action;
    };
    const auto addZoom = [this, menu](const QString& text, double zoom) {
        QAction* action = menu->addAction(text);
        connect(action, &QAction::triggered, this, [this, zoom]() {
            if (ImageView* canvas = controller_ ? controller_->canvas() : nullptr) {
                canvas->setZoom(zoom, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
            }
        });
        return action;
    };

    add(tr("Fit on Screen"), &ImageView::fitOnScreen);
    add(tr("100%"), &ImageView::actualPixels);
    addZoom(tr("200%"), 2.0);
    QAction* printSize = menu->addAction(tr("Print Size"));
    connect(printSize, &QAction::triggered, this, [this]() {
        PictureView* view = controller_ ? controller_->view() : nullptr;
        ImageView* canvas = controller_ ? controller_->canvas() : nullptr;
        if (!view || !canvas) {
            return;
        }
        // One image inch (ppi pixels) displays as one logical screen inch.
        // ponytail: a fixed 96 logical DPI, not the monitor's physical DPI.
        const double ppi = document_ppi(*view);
        if (ppi > 0.0) {
            canvas->setZoom(96.0 / ppi, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
        }
    });
    menu->addSeparator();
    add(tr("Zoom In"), &ImageView::zoomIn);
    add(tr("Zoom Out"), &ImageView::zoomOut);

    // Every entry needs a document; grey them out with none open.
    connect(menu, &QMenu::aboutToShow, this, [this, menu]() {
        ImageView* canvas = controller_ ? controller_->canvas() : nullptr;
        const bool enabled = canvas && canvas->hasDocument();
        for (QAction* action : menu->actions()) {
            action->setEnabled(enabled && !action->isSeparator());
        }
    });
    connect(menu, &QMenu::aboutToHide, this, [this, menu]() {
        if (openSlotMenu_ == menu) {
            closeOpenSlotMenu();
        }
    });
    return menu;
}

void Toolbox::showSlotMenu(int group)
{
    QMenu* menu = slotMenuByGroup_.value(group, nullptr);
    QToolButton* button = slotButtonByGroup_.value(group, nullptr);
    if (!menu || !button) {
        return;
    }
    if (openSlotMenu_ && openSlotMenu_ != menu) {
        closeOpenSlotMenu();
    }
    const QSize hint = menu->sizeHint();
    const QPoint buttonTopLeft = button->mapToGlobal(QPoint(0, 0));
    // Open beside the button, never over it: right edge first, flip left when
    // the right side overflows, and clamp only as a fallback.
    QPoint pos = button->mapToGlobal(QPoint(button->width(), 0));
    if (QScreen* screen = button->screen()) {
        const QRect avail = screen->availableGeometry();
        if (pos.x() + hint.width() > avail.right()) {
            pos.setX(buttonTopLeft.x() - hint.width());
        }
        if (pos.x() < avail.left()) {
            pos.setX(avail.left());
        }
        if (pos.y() + hint.height() > avail.bottom()) {
            pos.setY(avail.bottom() - hint.height());
        }
        if (pos.y() < avail.top()) {
            pos.setY(avail.top());
        }
    }
    openSlotMenu_ = menu;
    openSlotButton_ = button;
    qApp->installEventFilter(this);
    menu->popup(pos);
}

void Toolbox::closeOpenSlotMenu()
{
    QMenu* menu = openSlotMenu_;
    openSlotMenu_ = nullptr;
    if (openSlotButton_) {
        // The open popup swallowed the release; drop any lingering pressed look.
        openSlotButton_->setDown(false);
    }
    openSlotButton_ = nullptr;
    qApp->removeEventFilter(this);
    if (menu) {
        menu->close();
    }
}

QToolButton* Toolbox::slotButtonAt(const QPoint& globalPos) const
{
    for (QToolButton* button : slotButtons_) {
        if (button->rect().contains(button->mapFromGlobal(globalPos))) {
            return button;
        }
    }
    return nullptr;
}

// While a slot flyout is open the popup owns the mouse, so a press on another
// slot would only dismiss it (needing a second click). Catch the press here,
// close the flyout, and forward the press to the target slot so its own
// hold/click semantics run: a quick release activates, a hold opens its menu.
bool Toolbox::eventFilter(QObject* watched, QEvent* event)
{
    if (event->type() == QEvent::MouseButtonPress && openSlotMenu_
        && openSlotMenu_->isVisible()) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        const QPoint globalPos = mouse->globalPosition().toPoint();
        if (!openSlotMenu_->geometry().contains(globalPos)) {
            QToolButton* target = slotButtonAt(globalPos);
            if (target && target != openSlotButton_) {
                closeOpenSlotMenu();
                // Forward a full press/release pair: the popup grabbed the real
                // release, so without the synthetic one the target's hold timer
                // would keep running and open its flyout on a plain click.
                QMouseEvent forwardPress(QEvent::MouseButtonPress,
                                         target->mapFromGlobal(globalPos), mouse->globalPosition(),
                                         mouse->button(), mouse->buttons(), mouse->modifiers());
                QApplication::sendEvent(target, &forwardPress);
                QMouseEvent forwardRelease(QEvent::MouseButtonRelease,
                                           target->mapFromGlobal(globalPos),
                                           mouse->globalPosition(), mouse->button(), Qt::NoButton,
                                           mouse->modifiers());
                QApplication::sendEvent(target, &forwardRelease);
                return true;
            }
        }
    }
    return QWidget::eventFilter(watched, event);
}

QMenu* Toolbox::slotMenuForTest(int group) const
{
    return slotMenuByGroup_.value(group, nullptr);
}

QList<QAction*> Toolbox::slotMenuActionsForTest(int group) const
{
    QMenu* menu = slotMenuForTest(group);
    return menu ? menu->actions() : QList<QAction*>();
}

bool Toolbox::hasFlyoutTriangleForTest(int group) const
{
    QToolButton* button = slotButtonByGroup_.value(group, nullptr);
    if (!button) {
        return false;
    }
    return static_cast<ToolSlotButton*>(button)->hasFlyout();
}

void Toolbox::openSlotFlyoutForTest(int group)
{
    showSlotMenu(group);
}

bool Toolbox::handleToolKey(const QChar& key, bool shift)
{
    const int group = toolGroupForKey(key);
    if (group < 1 || !groupHasImplemented(group)) {
        return false;
    }
    if (shift || !shiftKeyForToolSwitch_) {
        cycleGroup(group, key.toUpper());
        return true;
    }
    ToolId current = groupCurrentTool(group);
    if (toolShortcut(current) != key.toUpper()) {
        for (ToolId id : allToolIds()) {
            if (toolShortcut(id) == key.toUpper() && toolImplemented(id)) {
                current = id;
                break;
            }
        }
    }
    if (!toolImplemented(current)) {
        return false;
    }
    selectMember(group, current);
    return true;
}

ToolId Toolbox::groupCurrentTool(int group) const
{
    if (currentByGroup_.contains(group)) {
        return currentByGroup_.value(group);
    }
    ToolId first;
    bool have = false;
    for (ToolId id : allToolIds()) {
        if (toolInfo(id).group != group) {
            continue;
        }
        if (!have) {
            first = id;
            have = true;
        }
        if (toolImplemented(id)) {
            return id;
        }
    }
    return first;
}

void Toolbox::refreshSlot(int group)
{
    QToolButton* button = slotButtonByGroup_.value(group, nullptr);
    if (!button) {
        return;
    }
    const ToolId id = groupCurrentTool(group);
    const ToolInfo& info = toolInfo(id);
    button->setIcon(icon(QStringLiteral("tool.") + toolIdName(id)));
    button->setToolTip(info.implemented ? enabledTooltip(info) : disabledTooltip(info));
    button->setChecked(controller_ && controller_->activeTool() == id);
}

void Toolbox::refreshToolShortcuts()
{
    // A single-tool group has no flyout; its entry is null.
    for (QMenu* menu : std::as_const(slotMenuByGroup_)) {
        if (!menu) {
            continue;
        }
        for (QAction* action : menu->actions()) {
            const ToolId member = ToolId(action->data().toInt());
            const ToolInfo& info = toolInfo(member);
            const QChar key = toolShortcut(member);
            action->setShortcut(key.isNull() ? QKeySequence() : QKeySequence(QString(key)));
            action->setToolTip(info.implemented ? enabledTooltip(info) : disabledTooltip(info));
        }
    }
    for (auto it = slotButtonByGroup_.cbegin(); it != slotButtonByGroup_.cend(); ++it) {
        refreshSlot(it.key());
    }
}

void Toolbox::selectMember(int group, ToolId id)
{
    currentByGroup_[group] = id;
    refreshSlot(group);
    if (controller_ && toolImplemented(id)) {
        controller_->setActiveTool(id);
    }
}

void Toolbox::cycleGroup(int group, QChar key)
{
    QList<ToolId> enabled;
    for (ToolId id : allToolIds()) {
        if (toolInfo(id).group == group && toolImplemented(id)
            && (key.isNull() || toolShortcut(id) == key)) {
            enabled << id;
        }
    }
    if (enabled.isEmpty() || (enabled.size() < 2 && key.isNull())) {
        return;
    }
    // From a member outside the cycle, the indexOf -1 lands on the first.
    const int index = enabled.indexOf(groupCurrentTool(group));
    selectMember(group, enabled.at((index + 1) % enabled.size()));
}

} // namespace pictura

#include "moc_toolbox.cpp"
