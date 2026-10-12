#include "panel_column.h"

#include "frame.h"
#include "panel_group.h"

#include <QtCore/QRect>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

namespace pictura {

void PanelColumn::showWorkspaceEdgeIndicator(bool left)
{
    // A bare workspace edge has no column on that side to anchor the mark, so it
    // is drawn at the central area's own left/right edge. Never suppressed: this
    // is where a dragged Tools column lands, so it may be owned by the tools
    // column.
    auto* frame = owningFrame();
    QWidget* central = frame ? frame->centralWidget() : nullptr;
    if (!frame || !central) {
        return;
    }
    if (!edgeIndicator_) {
        edgeIndicator_ = new QWidget(frame);
        edgeIndicator_->setObjectName(QStringLiteral("panelColumnEdgeIndicator"));
        edgeIndicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    }
    const QRect centralRect(central->mapToGlobal(QPoint(0, 0)), central->size());
    const int globalX = left ? centralRect.left() : centralRect.right() - 2;
    edgeIndicator_->setGeometry(
        QRect(frame->mapFromGlobal(QPoint(globalX, centralRect.top())),
              QSize(3, centralRect.height())));
    edgeIndicator_->show();
    edgeIndicator_->raise();
}

void PanelColumn::showColumnEdgeIndicator(bool left)
{
    // A full-height mark at the workspace edge for a new-column candidate. It is
    // parented to the frame and raised, so an in-window floating overlay (which
    // follows the cursor, above the splitter) cannot hide it.
    auto* frame = owningFrame();
    if (!frame) {
        return;
    }
    if (!edgeIndicator_) {
        edgeIndicator_ = new QWidget(frame);
        edgeIndicator_->setObjectName(QStringLiteral("panelColumnEdgeIndicator"));
        edgeIndicator_->setAttribute(Qt::WA_TransparentForMouseEvents);
    }
    const QRect selfRect(mapToGlobal(QPoint(0, 0)), size());
    const int globalX = left ? selfRect.left() : selfRect.right() - 2;
    edgeIndicator_->setGeometry(
        QRect(frame->mapFromGlobal(QPoint(globalX, selfRect.top())),
              QSize(3, selfRect.height())));
    edgeIndicator_->show();
    edgeIndicator_->raise();
}

void PanelColumn::showIndicatorFor(const DropTarget& target)
{
    // Any non-edge target hides the frame-level edge mark; the new-column branch
    // below is the only one that shows it.
    if (edgeIndicator_) {
        edgeIndicator_->hide();
    }
    // An unresolved (atomic/forbidden) target draws nothing.
    if (!target.valid) {
        clearIndicator();
        return;
    }
    // Phase 3: a float target draws the same indicator over the float's own tab
    // bar, since the docked indicator cannot reach outside its viewport.
    if (target.floatTarget && target.group) {
        if (indicator_) {
            indicator_->hide();
        }
        if (stripIndicator_) {
            stripIndicator_->hide();
        }
        if (outlineIndicator_) {
            outlineIndicator_->hide();
        }
        if (floatIndicator_ && floatIndicator_ != target.floatTarget) {
            floatIndicator_->hideTabIndicator();
        }
        floatIndicator_ = target.floatTarget;
        floatIndicator_->showTabIndicator(target.group, target.tabIndex);
        return;
    }
    if (floatIndicator_) {
        floatIndicator_->hideTabIndicator();
        floatIndicator_ = nullptr;
    }
    // Phase 4: a whole group dropped on another group's tab bar tabifies into the
    // target group. Draw the blue region outline around the whole target group
    // instead of the thin insertion line.
    if (target.groupTabify && target.group && scroll_ && !toolsContent_) {
        QWidget* viewport = scroll_->viewport();
        if (!viewport || !outlineIndicator_) {
            clearIndicator();
            return;
        }
        if (indicator_) {
            indicator_->hide();
        }
        if (stripIndicator_) {
            stripIndicator_->hide();
        }
        const QRect groupRect(target.group->mapTo(viewport, QPoint(0, 0)),
                              target.group->size());
        outlineIndicator_->setGeometry(groupRect.adjusted(-3, -3, 3, 3));
        outlineIndicator_->show();
        outlineIndicator_->raise();
        return;
    }
    if (outlineIndicator_) {
        outlineIndicator_->hide();
    }
    const bool newColumn =
        target.kind == DropKind::NewColumnLeft || target.kind == DropKind::NewColumnRight;
    const bool compactKind =
        railMode_
        && (target.kind == DropKind::IntoGroup || target.kind == DropKind::AboveGroup
            || target.kind == DropKind::BelowGroup)
        && target.stripIndex >= 0;
    if (target.onStrip || compactKind) {
        if (!stripIndicator_ || stripEntries_.isEmpty()) {
            clearIndicator();
            return;
        }
        // M45 C2: a whole-group drag draws at the group container's insertion
        // boundary — above its drag-handle grip — not at an icon button, so the
        // line sits where the drop inserts rather than inside the group.
        if (!dragIsPanel_) {
            QWidget* box = nullptr;
            bool below = false;
            if (target.group) {
                box = stripGroupBoxFor(target.group);
                below = target.kind == DropKind::BelowGroup;
            }
            if (!box) {
                const int boundary = qBound(0, target.boundary, groups_.size());
                if (boundary < groups_.size()) {
                    box = stripGroupBoxFor(groups_.at(boundary));
                } else if (!groups_.isEmpty()) {
                    box = stripGroupBoxFor(groups_.last());
                    below = true;
                }
            }
            if (box) {
                const QPoint origin = box->mapTo(iconStrip_, QPoint(0, 0));
                const int y = below ? origin.y() + box->height() + 1 : origin.y() - 1;
                stripIndicator_->setGeometry(QRect(0, y, iconStrip_->width(), 3));
                stripIndicator_->show();
                stripIndicator_->raise();
                if (indicator_) {
                    indicator_->hide();
                }
                return;
            }
        }
        const int index = qBound(0, target.stripIndex, stripEntries_.size());
        QToolButton* anchor = index < stripEntries_.size() ? stripEntries_.at(index).button
                                                           : stripEntries_.last().button;
        if (!anchor) {
            clearIndicator();
            return;
        }
        const QPoint origin = anchor->mapTo(iconStrip_, QPoint(0, 0));
        const int y = index < stripEntries_.size() ? origin.y() - 1
                                                   : origin.y() + anchor->height() + 1;
        stripIndicator_->setGeometry(QRect(0, y, iconStrip_->width(), 3));
        stripIndicator_->show();
        stripIndicator_->raise();
        if (indicator_) {
            indicator_->hide();
        }
        return;
    }
    if (stripIndicator_) {
        stripIndicator_->hide();
    }
    if (newColumn) {
        if (indicator_) {
            indicator_->hide();
        }
        // The atomic Tools column draws no bare new-column mark for a drag over
        // its own body (the outer-band target it owns); a target resolved beside
        // it — a `columnEdgeAnchorAt` anchor — draws the mark like any column.
        if (isToolsColumn() && !target.anchorColumn) {
            clearIndicator();
            return;
        }
        showColumnEdgeIndicator(target.kind == DropKind::NewColumnLeft);
        return;
    }
    if (!indicator_ || !scroll_) {
        return;
    }
    QWidget* viewport = scroll_->viewport();
    if (!viewport) {
        return;
    }
    if (target.onTabBar && target.group) {
        QTabBar* bar = target.group->tabBar();
        if (!bar) {
            clearIndicator();
            return;
        }
        const int x = target.group->tabInsertionX(target.tabIndex);
        const QPoint origin = bar->mapTo(viewport, QPoint(x, 0));
        indicator_->setGeometry(QRect(origin.x() - 1, origin.y(), 3, bar->height()));
    } else {
        const QList<PanelGroup*> visible = visibleGroups();
        if (visible.isEmpty()) {
            clearIndicator();
            return;
        }
        auto topOf = [viewport](PanelGroup* group) {
            return group->mapTo(viewport, QPoint(0, 0)).y();
        };
        int prev = -1;
        int next = -1;
        for (int i = 0; i < groups_.size(); ++i) {
            PanelGroup* group = groups_.at(i);
            if (!group || !group->isVisible()) {
                continue;
            }
            if (i < target.boundary) {
                prev = i;
            } else if (next < 0) {
                next = i;
            }
        }
        int y = 0;
        if (prev >= 0 && next >= 0) {
            const int bottom = topOf(groups_.at(prev)) + groups_.at(prev)->height();
            y = (bottom + topOf(groups_.at(next))) / 2;
        } else if (next >= 0) {
            y = topOf(groups_.at(next)) - 1;
        } else if (prev >= 0) {
            y = topOf(groups_.at(prev)) + groups_.at(prev)->height() + 1;
        }
        y = qBound(0, y, qMax(0, viewport->height() - 3));
        indicator_->setGeometry(QRect(0, y, viewport->width(), 3));
    }
    indicator_->show();
    indicator_->raise();
}

void PanelColumn::clearIndicator()
{
    if (indicator_) {
        indicator_->hide();
    }
    if (stripIndicator_) {
        stripIndicator_->hide();
    }
    if (outlineIndicator_) {
        outlineIndicator_->hide();
    }
    if (edgeIndicator_) {
        edgeIndicator_->hide();
    }
    if (floatIndicator_) {
        floatIndicator_->hideTabIndicator();
        floatIndicator_ = nullptr;
    }
}

void PanelColumn::showEdgeDropIndicator(PanelSide side)
{
    // M45 T3: reuse the M43/M44 new-column `DropTarget` and the single
    // `#2a7fff` indicator; no second indicator system.
    DropTarget target;
    target.valid = true;
    target.anchorColumn = this;
    target.kind = side == PanelSide::Left ? DropKind::NewColumnLeft
                                          : DropKind::NewColumnRight;
    showIndicatorFor(target);
}

void PanelColumn::hideEdgeDropIndicator()
{
    clearIndicator();
}

} // namespace pictura
