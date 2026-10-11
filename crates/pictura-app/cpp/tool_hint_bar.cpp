#include "tool_hint_bar.h"

#include "icons.h"
#include "theme.h"

#include <QtCore/QPointF>
#include <QtCore/QStringList>
#include <QtGui/QFontMetrics>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QTransform>

namespace pictura {

namespace {

// Total width of the four-chevron cluster; caps and gaps are derived from it.
// Wide enough that each cap keeps a little horizontal padding around its glyph.
constexpr int kArrowsKeycapWidth = 46;
constexpr int kArrowsCapGap = 2;

// The Move hint's four chevrons, in draw order: up, left, down, right across
// one row. One source of truth for the painter and the test hooks.
constexpr char kArrowsChevronDirections[] = "ULDR";

int chevronDirectionFor(char c)
{
    switch (c) {
    case 'U':
        return 0;
    case 'R':
        return 1;
    case 'D':
        return 2;
    default: // 'L'
        return 3;
    }
}

// A hint key renders its chevrons when any '+'-separated segment contains
// "Arrows", so "Shift+Arrows" picks up the direction cluster too.
bool isArrowsKey(const QString& key)
{
    return key.contains(QStringLiteral("Arrows"), Qt::CaseInsensitive);
}

int keycapWidth(const QFontMetrics& fm, const QString& key)
{
    if (!isArrowsKey(key)) {
        return fm.horizontalAdvance(key) + 12;
    }
    const QStringList parts = key.split(QLatin1Char('+'));
    int width = 0;
    for (int i = 0; i < parts.size(); ++i) {
        if (i > 0) {
            width += fm.horizontalAdvance(QStringLiteral(" + "));
        }
        width += isArrowsKey(parts.at(i)) ? kArrowsKeycapWidth
                                          : fm.horizontalAdvance(parts.at(i)) + 12;
    }
    return width;
}

struct ChevronGlyph {
    const char* asset;
    bool flipped;
};

ChevronGlyph chevronGlyphFor(int direction)
{
    switch (direction) {
    case 0: // up
        return {"layers.disclosureDown", true};
    case 1: // right
        return {"layers.disclosureRight", false};
    case 2: // down
        return {"layers.disclosureDown", false};
    default: // left
        return {"layers.disclosureRight", true};
    }
}

void paintChevron(QPainter& painter, const QPointF& c, int direction, int iconSize,
                  const QColor& color)
{
    const ChevronGlyph glyph = chevronGlyphFor(direction);
    QPixmap pixmap =
        pictura::icon(QString::fromLatin1(glyph.asset), color).pixmap(QSize(iconSize, iconSize));
    if (glyph.flipped) {
        pixmap = pixmap.transformed(QTransform().rotate(180.0), Qt::SmoothTransformation);
    }
    painter.drawPixmap(QPointF(c.x() - iconSize / 2.0, c.y() - iconSize / 2.0), pixmap);
}

// The Move tool's nudge hint: four small keycaps side by side in a single row,
// each carrying its own chevron.
void paintArrowsKeycaps(QPainter& painter, const QRect& keyRect, const QColor& border,
                        const QColor& fill, const QColor& chevron)
{
    constexpr int capW = (kArrowsKeycapWidth - 3 * kArrowsCapGap) / 4;
    const int capH = qMax(6, keyRect.height() - 3);
    const int totalW = 4 * capW + 3 * kArrowsCapGap;
    const int startX = keyRect.left() + qMax(0, (keyRect.width() - totalW) / 2);
    const int y = keyRect.top() + (keyRect.height() - capH) / 2;
    const int iconSize = qMax(8, capW + 4);

    for (int i = 0; i < 4; ++i) {
        const QRect r(startX + i * (capW + kArrowsCapGap), y, capW, capH);
        painter.setPen(QPen(border, 1));
        painter.setBrush(fill);
        painter.drawRoundedRect(r, 2, 2);
        paintChevron(painter, QRectF(r).center(), chevronDirectionFor(kArrowsChevronDirections[i]),
                     iconSize, chevron);
    }
}

// A whole hint key, possibly composite ("Shift+Arrows"): literal keycaps for
// each non-arrow segment, the four-chevron cluster for any "Arrows" segment,
// joined by a literal " + ".
void paintKey(QPainter& painter, const QRect& keyRect, const QString& key, const QFontMetrics& fm,
              const QColor& border, const QColor& fill, const QColor& keyColor)
{
    const QStringList parts = key.split(QLatin1Char('+'));
    int x = keyRect.left();
    for (int i = 0; i < parts.size(); ++i) {
        if (i > 0) {
            const QString sep = QStringLiteral(" + ");
            const int sepW = fm.horizontalAdvance(sep);
            painter.setPen(keyColor);
            painter.drawText(QRect(x, keyRect.top(), sepW, keyRect.height()), Qt::AlignCenter, sep);
            x += sepW;
        }
        const QString& part = parts.at(i);
        if (isArrowsKey(part)) {
            paintArrowsKeycaps(painter,
                               QRect(x, keyRect.top(), kArrowsKeycapWidth, keyRect.height()), border,
                               fill, keyColor);
            x += kArrowsKeycapWidth;
        } else {
            const int partW = fm.horizontalAdvance(part) + 12;
            const QRect r(x, keyRect.top(), partW, keyRect.height());
            painter.setPen(QPen(border, 1));
            painter.setBrush(fill);
            painter.drawRoundedRect(r, 3, 3);
            painter.setPen(keyColor);
            painter.drawText(r, Qt::AlignCenter, part);
            x += partW;
        }
    }
}

} // namespace

ToolHintBar::ToolHintBar(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("toolHintBar"));
    setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
}

void ToolHintBar::setHints(const QList<ToolHint>& hints, const QString& fallbackText)
{
    hints_ = hints;
    fallbackText_ = fallbackText;
    pressed_ = -1;
    updateGeometry();
    update();
}

void ToolHintBar::setPressedKey(const QString& key)
{
    int index = -1;
    if (!key.isEmpty()) {
        for (int i = 0; i < hints_.size(); ++i) {
            const QString& hintKey = hints_.at(i).key;
            if (hintKey.compare(key, Qt::CaseInsensitive) == 0) {
                index = i;
                break;
            }
            // A composite key ("Shift+Arrows") also lights up when its leading
            // modifier is held.
            const int plus = hintKey.indexOf(QLatin1Char('+'));
            if (plus > 0 && hintKey.left(plus).compare(key, Qt::CaseInsensitive) == 0) {
                index = i;
                break;
            }
        }
    }
    if (index != pressed_) {
        pressed_ = index;
        update();
    }
}

QString ToolHintBar::keyForTest(int index) const
{
    return index >= 0 && index < hints_.size() ? hints_.at(index).key : QString();
}

QString ToolHintBar::textForTest(int index) const
{
    return index >= 0 && index < hints_.size() ? hints_.at(index).text : QString();
}

int ToolHintBar::chevronCountForTest(int index) const
{
    if (index < 0 || index >= hints_.size() || !isArrowsKey(hints_.at(index).key)) {
        return 0;
    }
    return int(sizeof(kArrowsChevronDirections) - 1);
}

QString ToolHintBar::chevronDirectionsForTest() const
{
    return QString::fromLatin1(kArrowsChevronDirections);
}

QString ToolHintBar::chevronAssetForTest(int index) const
{
    if (index < 0 || index >= int(sizeof(kArrowsChevronDirections) - 1)) {
        return QString();
    }
    return QString::fromLatin1(
        chevronGlyphFor(chevronDirectionFor(kArrowsChevronDirections[index])).asset);
}

bool ToolHintBar::chevronFlippedForTest(int index) const
{
    if (index < 0 || index >= int(sizeof(kArrowsChevronDirections) - 1)) {
        return false;
    }
    return chevronGlyphFor(chevronDirectionFor(kArrowsChevronDirections[index])).flipped;
}

QSize ToolHintBar::sizeHint() const
{
    const QFontMetrics fm(font());
    int width = 8;
    if (hints_.isEmpty()) {
        width += fm.horizontalAdvance(fallbackText_);
    } else {
        for (const ToolHint& hint : hints_) {
            width += keycapWidth(fm, hint.key) + 6;
            width += fm.horizontalAdvance(hint.text) + 12;
        }
    }
    return QSize(width, fm.height() + 8);
}

void ToolHintBar::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    const QFontMetrics fm(font());
    if (hints_.isEmpty()) {
        painter.setPen(palette().color(QPalette::WindowText));
        painter.drawText(rect().adjusted(4, 0, -4, 0), Qt::AlignVCenter | Qt::AlignLeft,
                         fallbackText_);
        return;
    }
    const int keyHeight = fm.height() + 4;
    const int y = (height() - keyHeight) / 2;
    // Keycaps sit flush on the footer surface: the fill is the panel colour and
    // only a one-step-lighter rule outlines them.
    const QColor footerBg = Theme::panelColor();
    const QColor border = Theme::shade(footerBg, 1);
    const QColor hotBg = palette().color(QPalette::Highlight);
    int x = 4;
    for (int i = 0; i < hints_.size(); ++i) {
        const ToolHint& hint = hints_.at(i);
        const int keyWidth = keycapWidth(fm, hint.key);
        const QRect keyRect(x, y, keyWidth, keyHeight);
        const bool hot = i == pressed_;
        const QColor fill = hot ? hotBg : footerBg;
        const QColor keyColor = hot ? palette().color(QPalette::HighlightedText)
                                    : palette().color(QPalette::WindowText);
        if (isArrowsKey(hint.key)) {
            paintKey(painter, keyRect, hint.key, fm, border, fill, keyColor);
        } else {
            painter.setPen(QPen(border, 1));
            painter.setBrush(fill);
            painter.drawRoundedRect(keyRect, 3, 3);
            painter.setPen(keyColor);
            painter.drawText(keyRect, Qt::AlignCenter, hint.key);
        }
        x += keyWidth + 6;
        painter.setPen(palette().color(QPalette::WindowText));
        painter.drawText(QRect(x, y, fm.horizontalAdvance(hint.text), keyHeight),
                         Qt::AlignVCenter | Qt::AlignLeft, hint.text);
        x += fm.horizontalAdvance(hint.text) + 14;
    }
}

} // namespace pictura
