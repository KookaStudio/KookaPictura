// ImageView overlays for the Eyedropper tool group: color samplers, notes, and
// the Ruler's measuring line. Ported from photorust's
// CanvasView::paintAnnotations.

#include "image_view.h"

#include <QtGui/QFontMetrics>
#include <QtGui/QPainter>

namespace pictura {

void ImageView::setAnnotationOverlay(const QList<QPointF>& samplers, const QList<QPointF>& notes,
                                     int currentNote)
{
    if (samplers == samplerOverlay_ && notes == noteOverlay_ && currentNote == currentNote_) {
        return;
    }
    samplerOverlay_ = samplers;
    noteOverlay_ = notes;
    currentNote_ = currentNote;
    update();
}

void ImageView::setCountOverlay(const QList<CountOverlayMark>& marks)
{
    if (countOverlay_.size() == marks.size()) {
        bool same = true;
        for (qsizetype i = 0; i < marks.size(); ++i) {
            const CountOverlayMark& a = countOverlay_.at(i);
            const CountOverlayMark& b = marks.at(i);
            if (a.pos != b.pos || a.number != b.number || a.color != b.color
                || a.markerSize != b.markerSize || a.labelSize != b.labelSize) {
                same = false;
                break;
            }
        }
        if (same) {
            return;
        }
    }
    countOverlay_ = marks;
    update();
}

void ImageView::setRulerLine(const QLineF& line)
{
    rulerLine_ = line;
    rulerShown_ = true;
    update();
}

void ImageView::clearRulerLine()
{
    rulerLine_ = QLineF();
    rulerShown_ = false;
    update();
}

// Drawn in widget space so lines, glyphs, and badges keep their screen size at
// any zoom.
void ImageView::paintAnnotations(QPainter& painter)
{
    if (samplerOverlay_.isEmpty() && noteOverlay_.isEmpty() && countOverlay_.isEmpty()
        && !rulerShown_) {
        return;
    }
    const auto toWidget = [this](const QPointF& p) { return p * zoom_ + offset_; };
    painter.save();
    painter.setTransform(viewRotation());
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, true);

    if (rulerShown_) {
        const QPointF a = toWidget(rulerLine_.p1());
        const QPointF b = toWidget(rulerLine_.p2());
        // White under black keeps the line visible over light and dark pixels.
        painter.setPen(QPen(Qt::white, 3));
        painter.drawLine(a, b);
        painter.setPen(QPen(Qt::black, 1));
        painter.drawLine(a, b);
        painter.setBrush(Qt::white);
        for (const QPointF& p : {a, b}) {
            painter.drawRect(QRectF(p.x() - 3, p.y() - 3, 6, 6));
        }
    }

    const QColor noteColor(0xe8, 0xc0, 0x3a);
    for (int i = 0; notesVisible_ && i < noteOverlay_.size(); ++i) {
        const QPointF p = toWidget(noteOverlay_.at(i));
        const QRectF page(p.x() - 5, p.y() - 6, 10, 12);
        painter.setPen(QPen(Qt::black, i == currentNote_ ? 2 : 1));
        painter.setBrush(noteColor);
        painter.drawRect(page);
        painter.setPen(QPen(QColor(0x40, 0x35, 0x10), 1));
        for (int line = 0; line < 3; ++line) {
            const double y = p.y() - 3 + line * 3;
            painter.drawLine(QPointF(p.x() - 3, y), QPointF(p.x() + 3, y));
        }
    }

    // A crosshair over the exact sampled pixel with a numbered badge beside it.
    const QColor samplerColor(0x2c, 0x6f, 0xd6);
    QFont badgeFont = painter.font();
    badgeFont.setPixelSize(9);
    painter.setFont(badgeFont);
    const QFontMetrics metrics(badgeFont);
    for (int i = 0; samplersVisible_ && i < samplerOverlay_.size(); ++i) {
        const QPointF p = toWidget(samplerOverlay_.at(i));
        for (const QPen& pen : {QPen(Qt::white, 2.5), QPen(samplerColor, 1.2)}) {
            painter.setPen(pen);
            painter.drawLine(QPointF(p.x() - 6, p.y()), QPointF(p.x() + 6, p.y()));
            painter.drawLine(QPointF(p.x(), p.y() - 6), QPointF(p.x(), p.y() + 6));
        }
        const QString label = QString::number(i + 1);
        const QRectF badge(p.x() + 5, p.y() - 15, metrics.horizontalAdvance(label) + 6, 12);
        painter.setPen(Qt::NoPen);
        painter.setBrush(samplerColor);
        painter.drawRect(badge);
        painter.setPen(Qt::white);
        painter.drawText(badge, Qt::AlignCenter, label);
    }

    // Count marks: a numbered disc per mark, per its group's colour, marker
    // size, and label size (Photoshop Extended). The disc grows to hold the
    // label so a larger Label Size never clips the number.
    for (const CountOverlayMark& mark : countOverlay_) {
        if (!countVisible_) {
            break;
        }
        const QPointF p = toWidget(mark.pos);
        QFont countFont = painter.font();
        countFont.setPixelSize(mark.labelSize);
        const QFontMetrics fm(countFont);
        const QString text = QString::number(mark.number);
        const double labelRadius = qMax(fm.horizontalAdvance(text), fm.height()) * 0.75;
        const double radius = qMax(3.0 + mark.markerSize * 1.5, labelRadius + 2.0);
        const QRectF disc(p.x() - radius, p.y() - radius, radius * 2, radius * 2);
        painter.setFont(countFont);
        painter.setPen(QPen(Qt::white, 2));
        painter.setBrush(mark.color);
        painter.drawEllipse(disc);
        painter.setPen(Qt::white);
        painter.drawText(disc, Qt::AlignCenter, text);
    }
    painter.restore();
}

} // namespace pictura
