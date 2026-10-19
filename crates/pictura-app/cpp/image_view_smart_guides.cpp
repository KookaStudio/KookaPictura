// ImageView smart alignment guides: the magenta lines and gap labels shown
// while the Move tool or the Type tool drags content into alignment.

#include "image_view.h"

#include "canvas_ruler.h"

#include <QtGui/QFont>
#include <QtGui/QFontMetrics>
#include <QtGui/QPainter>

#include <cmath>

namespace pictura {

void ImageView::setSmartGuides(const QList<SmartGuideLine>& lines,
                               const QList<SmartGuideGap>& gaps)
{
    if (lines == smartGuideLines_ && gaps.size() == smartGuideGaps_.size()) {
        bool same = true;
        for (int i = 0; i < gaps.size() && same; ++i) {
            same = gaps.at(i).distance == smartGuideGaps_.at(i).distance
                && gaps.at(i).anchor == smartGuideGaps_.at(i).anchor
                && gaps.at(i).verticalGuide == smartGuideGaps_.at(i).verticalGuide;
        }
        if (same) {
            return;
        }
    }
    smartGuideLines_ = lines;
    smartGuideGaps_ = gaps;
    update();
}

void ImageView::clearSmartGuides()
{
    if (smartGuideLines_.isEmpty() && smartGuideGaps_.isEmpty()) {
        return;
    }
    smartGuideLines_.clear();
    smartGuideGaps_.clear();
    update();
}

void ImageView::setSmartGuidesEnabled(bool on)
{
    if (smartGuidesEnabled_ == on) {
        return;
    }
    smartGuidesEnabled_ = on;
    if (!on) {
        clearSmartGuides();
    }
}

QString ImageView::formatGuideDistance(double documentPixels) const
{
    const double ppi = measurePpi_ ? measurePpi_() : 72.0;
    return formatRulerLength(documentPixels, RulerUnit(measureUnit_), ppi, measureTraditional_,
                             docSize_.width());
}

QString ImageView::smartGuideGapForTest() const
{
    return smartGuideGaps_.isEmpty() ? QString() : formatGuideDistance(smartGuideGaps_.first().distance);
}

// Lines run the whole view, like the document guides; labels sit in widget
// space so they stay readable at any zoom.
void ImageView::paintSmartGuides(QPainter& painter)
{
    if (docSize_.isEmpty() || smartGuideLines_.isEmpty()) {
        return;
    }
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setPen(QPen(QColor(255, 74, 255), 0, Qt::SolidLine));
    const QRectF frame = viewRect();
    for (const SmartGuideLine& line : smartGuideLines_) {
        if (line.vertical) {
            const double x = std::floor(line.position * zoom_ + offset_.x()) + 0.5;
            painter.drawLine(QPointF(x, frame.top()), QPointF(x, frame.bottom()));
        } else {
            const double y = std::floor(line.position * zoom_ + offset_.y()) + 0.5;
            painter.drawLine(QPointF(frame.left(), y), QPointF(frame.right(), y));
        }
    }
    painter.restore();

    for (const SmartGuideGap& gap : smartGuideGaps_) {
        const QString text = formatGuideDistance(gap.distance);
        if (text.isEmpty()) {
            continue;
        }
        painter.save();
        painter.resetTransform();
        painter.setClipping(false);
        const QPointF widgetPos = imageToWidget(gap.anchor);
        QFont font = painter.font();
        font.setPointSize(10);
        painter.setFont(font);
        const QFontMetrics fm(painter.font());
        QRectF boxRect(widgetPos + QPointF(10, -30), fm.boundingRect(text).size() + QSize(8, 6));
        if (boxRect.right() > rect().right()) {
            boxRect.moveRight(rect().right());
        }
        if (boxRect.bottom() > rect().bottom()) {
            boxRect.moveBottom(rect().bottom());
        }
        boxRect.moveLeft(std::max(boxRect.left(), qreal(rect().left())));
        boxRect.moveTop(std::max(boxRect.top(), qreal(rect().top())));
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(0, 0, 0, 200));
        painter.drawRoundedRect(boxRect, 3, 3);
        painter.setPen(Qt::white);
        painter.drawText(boxRect, Qt::AlignCenter, text);
        painter.restore();
    }
}

} // namespace pictura
