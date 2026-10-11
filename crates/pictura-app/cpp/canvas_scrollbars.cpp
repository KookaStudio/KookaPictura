#include "canvas_scrollbars.h"

#include "canvas_range.h"
#include "canvas_ruler.h"
#include "image_view.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QScrollBar>

#include <algorithm>
#include <cmath>

namespace pictura {

CanvasScrollBars::CanvasScrollBars(QWidget* parent)
    : QWidget(parent)
{
    grid_ = new QGridLayout(this);
    grid_->setContentsMargins(0, 0, 0, 0);
    grid_->setSpacing(0);
    // Rulers on row/column 0 (hidden until View > Rulers), the canvas at
    // (1, 1), the scrollbars on row/column 2.
    hruler_ = new CanvasRuler(Qt::Horizontal, this);
    vruler_ = new CanvasRuler(Qt::Vertical, this);
    rulerCorner_ = new QWidget(this);
    rulerCorner_->setObjectName(QStringLiteral("canvasRulerCorner"));
    rulerCorner_->setAttribute(Qt::WA_StyledBackground, true);
    grid_->addWidget(rulerCorner_, 0, 0);
    grid_->addWidget(hruler_, 0, 1);
    grid_->addWidget(vruler_, 1, 0);
    for (CanvasRuler* ruler : {hruler_, vruler_}) {
        connect(ruler, &CanvasRuler::guideDropped, this, &CanvasScrollBars::guideDropped);
        connect(ruler, &CanvasRuler::unitChosen, this, &CanvasScrollBars::rulerUnitChosen);
        connect(ruler, &CanvasRuler::preferencesRequested, this,
                &CanvasScrollBars::rulerPreferencesRequested);
    }
    setRulersVisible(false);

    hbar_ = new QScrollBar(Qt::Horizontal, this);
    vbar_ = new QScrollBar(Qt::Vertical, this);
    grid_->addWidget(vbar_, 1, 2);
    grid_->addWidget(hbar_, 2, 0, 1, 2);
    // The empty corner where the two bars meet: styled to the scrollbar track so
    // the workspace colour does not show through the notch.
    auto* corner = new QWidget(this);
    corner->setObjectName(QStringLiteral("canvasScrollCorner"));
    corner->setAttribute(Qt::WA_StyledBackground, true);
    grid_->addWidget(corner, 2, 2);
    grid_->setColumnStretch(1, 1);
    grid_->setRowStretch(1, 1);
    hbar_->hide();
    vbar_->hide();

    connect(hbar_, &QScrollBar::valueChanged, this, [this](int value) {
        if (syncing_ || !view_) {
            return;
        }
        const OffsetRange range = offsetRangeFor(QSizeF(view_->documentSize()), view_->zoom(),
                                                 QSizeF(view_->size()));
        view_->setOffset(QPointF(range.minX + value, view_->offset().y()));
    });
    connect(vbar_, &QScrollBar::valueChanged, this, [this](int value) {
        if (syncing_ || !view_) {
            return;
        }
        const OffsetRange range = offsetRangeFor(QSizeF(view_->documentSize()), view_->zoom(),
                                                 QSizeF(view_->size()));
        view_->setOffset(QPointF(view_->offset().x(), range.minY + value));
    });
}

void CanvasScrollBars::setView(ImageView* view)
{
    if (view_ == view) {
        syncFromView();
        return;
    }
    if (view_) {
        disconnect(view_, nullptr, this, nullptr);
        view_->hide();
        grid_->removeWidget(view_);
    }
    view_ = view;
    if (view_) {
        view_->setParent(this);
        grid_->addWidget(view_, 1, 1);
        view_->show();
        connect(view_, &ImageView::viewChanged, this, [this]() { syncFromView(); });
        connect(view_, &ImageView::mouseMoved, this, [this](const QPointF& imagePos) {
            hruler_->setCursorPosition(imagePos);
            vruler_->setCursorPosition(imagePos);
        });
    }
    hruler_->setView(view_);
    vruler_->setView(view_);
    syncFromView();
}

void CanvasScrollBars::setRulerUnit(RulerUnit unit, bool traditionalPoints)
{
    for (CanvasRuler* ruler : {hruler_, vruler_}) {
        ruler->setUnit(unit);
        ruler->setTraditionalPoints(traditionalPoints);
    }
}

void CanvasScrollBars::setPpiProvider(const std::function<double()>& ppi)
{
    hruler_->setPpiProvider(ppi);
    vruler_->setPpiProvider(ppi);
}

void CanvasScrollBars::setRulersVisible(bool on)
{
    hruler_->setVisible(on);
    vruler_->setVisible(on);
    rulerCorner_->setVisible(on);
}

void CanvasScrollBars::resizeEvent(QResizeEvent* event)
{
    syncFromView();
    QWidget::resizeEvent(event);
}

void CanvasScrollBars::syncFromView()
{
    if (!view_ || syncing_) {
        return;
    }
    syncing_ = true;
    hruler_->update();
    vruler_->update();

    const double zoom = view_->zoom();
    const QSizeF imageSize(view_->documentSize());
    // ponytail: visibility is fixed on. The canvas is pannable within the
    // reveal margin, so an as-needed policy would remove the only pan
    // affordance; the bars stay a projection of the shared offset range.
    hbar_->show();
    vbar_->show();

    const OffsetRange range = offsetRangeFor(imageSize, zoom, QSizeF(view_->size()));
    {
        const QSignalBlocker blockH(hbar_);
        const QSignalBlocker blockV(vbar_);
        const int rangeX = int(std::lround(std::max(0.0, range.maxX - range.minX)));
        const int rangeY = int(std::lround(std::max(0.0, range.maxY - range.minY)));
        hbar_->setRange(0, rangeX);
        hbar_->setPageStep(std::max(1, view_->width()));
        hbar_->setSingleStep(std::max(1, view_->width() / 10));
        hbar_->setValue(int(std::lround(view_->offset().x() - range.minX)));
        vbar_->setRange(0, rangeY);
        vbar_->setPageStep(std::max(1, view_->height()));
        vbar_->setSingleStep(std::max(1, view_->height() / 10));
        vbar_->setValue(int(std::lround(view_->offset().y() - range.minY)));
    }

    syncing_ = false;
}

} // namespace pictura
