#include "character_panel_icons.h"

#include <QtGui/QFont>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPixmap>

#include <cmath>
#include <functional>

namespace pictura::character_icons {

namespace {

constexpr int kWidth = 24;
constexpr int kHeight = 20;
constexpr qreal kRatio = 2.0;

QFont typeFont(int pixels, bool bold = false, bool italic = false)
{
    QFont font(QStringLiteral("serif"));
    font.setStyleHint(QFont::Serif);
    font.setPixelSize(pixels);
    font.setBold(bold);
    font.setItalic(italic);
    return font;
}

// A transparent canvas painted by `draw` with antialiased `color` strokes.
QIcon drawn(const QColor& color, const std::function<void(QPainter&)>& draw)
{
    QPixmap pixmap(QSize(kWidth, kHeight) * kRatio);
    pixmap.setDevicePixelRatio(kRatio);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setRenderHint(QPainter::TextAntialiasing);
    painter.setPen(QPen(color, 1.2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    draw(painter);
    painter.end();
    return QIcon(pixmap);
}

void text(QPainter& painter, const QRectF& box, const QString& glyphs, const QFont& font,
          Qt::Alignment align = Qt::AlignCenter)
{
    painter.setFont(font);
    painter.drawText(box, int(align), glyphs);
}

// A line from `from` to `to` with an open arrowhead at each end listed.
void arrow(QPainter& painter, QPointF from, QPointF to, bool atFrom, bool atTo)
{
    painter.drawLine(from, to);
    const QPointF d = to - from;
    const qreal length = std::hypot(d.x(), d.y());
    if (length <= 0.0) {
        return;
    }
    const QPointF u = d / length;
    const QPointF n(-u.y(), u.x());
    const qreal head = 2.2;
    const auto tip = [&](QPointF at, QPointF back) {
        painter.drawLine(at, at + back * head + n * head);
        painter.drawLine(at, at + back * head - n * head);
    };
    if (atTo) {
        tip(to, -u);
    }
    if (atFrom) {
        tip(from, u);
    }
}

const QRectF kFull(0, 0, kWidth, kHeight);

} // namespace

QIcon field(Field which, const QColor& color)
{
    return drawn(color, [which](QPainter& p) {
        switch (which) {
        case Field::Size:
            text(p, QRectF(0, 7, 10, 12), QStringLiteral("T"), typeFont(10), Qt::AlignCenter);
            text(p, QRectF(7, 0, 17, 20), QStringLiteral("T"), typeFont(18), Qt::AlignCenter);
            break;
        case Field::Leading:
            text(p, QRectF(8, 0, 16, 10), QStringLiteral("A"), typeFont(10));
            text(p, QRectF(8, 10, 16, 10), QStringLiteral("A"), typeFont(10));
            p.drawLine(QPointF(10, 10), QPointF(22, 10));
            arrow(p, QPointF(4, 3), QPointF(4, 17), true, true);
            break;
        case Field::Kerning:
            text(p, QRectF(0, 0, 24, 15), QStringLiteral("V/A"), typeFont(11));
            arrow(p, QPointF(14, 17.5), QPointF(7, 17.5), false, true);
            break;
        case Field::Tracking:
            text(p, QRectF(0, 0, 24, 14), QStringLiteral("VA"), typeFont(12, true));
            p.drawLine(QPointF(4, 13.5), QPointF(20, 13.5));
            arrow(p, QPointF(4, 17.5), QPointF(20, 17.5), true, true);
            break;
        case Field::VerticalScale:
            text(p, QRectF(7, 0, 17, 20), QStringLiteral("T"), typeFont(18));
            arrow(p, QPointF(4, 2), QPointF(4, 18), true, true);
            break;
        case Field::HorizontalScale:
            text(p, QRectF(0, -2, 24, 17), QStringLiteral("T"), typeFont(15));
            arrow(p, QPointF(3, 17.5), QPointF(21, 17.5), true, true);
            break;
        case Field::BaselineShift:
            text(p, QRectF(0, 2, 13, 16), QStringLiteral("A"), typeFont(14));
            text(p, QRectF(12, 0, 12, 10), QStringLiteral("a"), typeFont(9));
            p.drawLine(QPointF(1, 18), QPointF(12, 18));
            arrow(p, QPointF(18, 18), QPointF(18, 10), false, true);
            break;
        case Field::AntiAlias:
            text(p, QRectF(0, 0, 11, 14), QStringLiteral("a"), typeFont(9));
            text(p, QRectF(8, 4, 16, 16), QStringLiteral("a"), typeFont(15));
            break;
        }
    });
}

QIcon style(int index, const QColor& color)
{
    return drawn(color, [index](QPainter& p) {
        const QFont plain = typeFont(15);
        switch (index) {
        case 0:
            text(p, kFull, QStringLiteral("T"), typeFont(16, true));
            break;
        case 1:
            text(p, kFull, QStringLiteral("T"), typeFont(16, false, true));
            break;
        case 2:
            text(p, kFull, QStringLiteral("TT"), plain);
            break;
        case 3:
            text(p, QRectF(0, 0, 13, 20), QStringLiteral("T"), plain, Qt::AlignRight | Qt::AlignVCenter);
            text(p, QRectF(13, 5, 11, 13), QStringLiteral("T"), typeFont(10),
                 Qt::AlignLeft | Qt::AlignVCenter);
            break;
        case 4:
            text(p, QRectF(0, 0, 15, 20), QStringLiteral("T"), plain, Qt::AlignRight | Qt::AlignVCenter);
            text(p, QRectF(15, 0, 9, 10), QStringLiteral("1"), typeFont(8), Qt::AlignLeft);
            break;
        case 5:
            text(p, QRectF(0, 0, 15, 20), QStringLiteral("T"), plain, Qt::AlignRight | Qt::AlignVCenter);
            text(p, QRectF(15, 10, 9, 10), QStringLiteral("1"), typeFont(8), Qt::AlignLeft);
            break;
        case 6:
            text(p, QRectF(0, -2, 24, 20), QStringLiteral("T"), plain);
            p.drawLine(QPointF(6, 17.5), QPointF(18, 17.5));
            break;
        case 7:
            text(p, kFull, QStringLiteral("T"), plain);
            p.drawLine(QPointF(6, 11), QPointF(18, 11));
            break;
        }
    });
}

QIcon openType(int index, const QColor& color)
{
    return drawn(color, [index](QPainter& p) {
        switch (index) {
        case 0:
            text(p, kFull, QStringLiteral("fi"), typeFont(15));
            break;
        case 1:
            text(p, kFull, QStringLiteral("σ"), typeFont(15, false, true));
            break;
        case 2:
            text(p, kFull, QStringLiteral("st"), typeFont(15));
            p.drawArc(QRectF(6, 2, 8, 6), 0, 180 * 16);
            break;
        case 3:
            text(p, kFull, QStringLiteral("A"), typeFont(16, false, true));
            p.drawArc(QRectF(1, 12, 10, 8), 180 * 16, 120 * 16);
            break;
        case 4:
            text(p, QRectF(0, 3, 24, 17), QStringLiteral("aa"), typeFont(13));
            arrow(p, QPointF(5, 3), QPointF(19, 3), false, true);
            break;
        case 5: {
            QPainterPath path;
            path.addText(QPointF(6, 17), typeFont(17), QStringLiteral("T"));
            p.setBrush(Qt::NoBrush);
            p.drawPath(path);
            break;
        }
        case 6:
            text(p, QRectF(0, 0, 11, 20), QStringLiteral("1"), typeFont(14),
                 Qt::AlignRight | Qt::AlignVCenter);
            text(p, QRectF(11, 1, 13, 10), QStringLiteral("st"), typeFont(8), Qt::AlignLeft);
            break;
        case 7:
            text(p, kFull, QStringLiteral("½"), typeFont(15));
            break;
        }
    });
}

} // namespace pictura::character_icons
