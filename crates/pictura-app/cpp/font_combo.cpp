#include "font_combo.h"

#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontInfo>
#include <QtGui/QFontMetrics>
#include <QtGui/QIcon>
#include <QtGui/QPainter>
#include <QtGui/QRawFont>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QStyledItemDelegate>

namespace pictura {

namespace {

bool hasColorGlyphs(const QRawFont& raw)
{
    for (const char* tag : {"COLR", "CBDT", "sbix", "SVG "}) {
        if (!raw.fontTable(tag).isEmpty()) {
            return true;
        }
    }
    return false;
}

// CS6's font menu row: the family name in the UI font, so a thin or dotted face
// reads as clearly as any other, then a short sample in the face itself.
// ponytail: one preview size; Type > Font Preview Size is not built yet.
class FontRowDelegate : public QStyledItemDelegate {
public:
    explicit FontRowDelegate(QObject* parent)
        : QStyledItemDelegate(parent)
        , scalable_(
              QStringLiteral(":/qt-project.org/styles/commonstyle/images/fonttruetype-16.png"))
    {
    }

    void paint(QPainter* painter, const QStyleOptionViewItem& option,
               const QModelIndex& index) const override
    {
        const QString family = index.data(Qt::DisplayRole).toString();
        const QFont& ui = option.font;
        painter->save();
        if (option.state & QStyle::State_Selected) {
            painter->fillRect(option.rect, option.palette.highlight());
            painter->setPen(option.palette.highlightedText().color());
        } else {
            painter->setPen(option.palette.text().color());
        }
        QRect rect = option.rect.adjusted(2, 0, -kGap, 0);
        const QSize iconSize = scalable_.actualSize(rect.size());
        scalable_.paint(painter, rect, Qt::AlignLeft | Qt::AlignVCenter);
        rect.setLeft(rect.left() + iconSize.width() + 4);
        painter->setFont(ui);
        const int nameWidth = QFontMetrics(ui).horizontalAdvance(family);
        painter->drawText(rect, Qt::AlignVCenter | Qt::TextSingleLine, family);

        const QString sample = sampleFor(family);
        if (!sample.isEmpty()) {
            const QFont face = sampleFont(family, ui);
            const int sampleWidth = QFontMetrics(face).horizontalAdvance(sample);
            rect.setLeft(rect.left() + nameWidth + kGap);
            if (rect.width() >= sampleWidth) {
                painter->setFont(face);
                painter->setClipRect(option.rect);
                painter->drawText(rect, Qt::AlignRight | Qt::AlignVCenter | Qt::TextSingleLine,
                                  sample);
            }
        }
        painter->restore();
    }

    // Rows are sized from the UI font alone, so opening the list loads no face
    // just to measure it.
    QSize sizeHint(const QStyleOptionViewItem& option, const QModelIndex& index) const override
    {
        const QFontMetrics metrics(option.font);
        const QString family = index.data(Qt::DisplayRole).toString();
        const int width = 2 + scalable_.actualSize(QSize(16, 16)).width() + 4
            + metrics.horizontalAdvance(family) + kGap
            + metrics.horizontalAdvance(QStringLiteral("Sample")) * 3 / 2 + kGap;
        return QSize(width, metrics.height() * 3 / 2);
    }

private:
    static constexpr int kGap = 12;

    static QFont sampleFont(const QString& family, const QFont& ui)
    {
        QFont face(family);
        face.setPixelSize(QFontInfo(ui).pixelSize() + 2);
        return face;
    }

    // "Sample" for a face with Latin letters, its own script's sample otherwise,
    // and nothing for a color face (previewing one costs ~400 ms per repaint)
    // or a face that lacks the sample's glyphs, which would draw fallback junk.
    QString sampleFor(const QString& family) const
    {
        auto it = samples_.constFind(family);
        if (it == samples_.constEnd()) {
            QString sample;
            const QRawFont raw = QRawFont::fromFont(QFont(family));
            if (raw.isValid() && !hasColorGlyphs(raw)) {
                const QList<QFontDatabase::WritingSystem> systems =
                    QFontDatabase::writingSystems(family);
                sample = systems.isEmpty() || systems.contains(QFontDatabase::Latin)
                    ? QStringLiteral("Sample")
                    : QFontDatabase::writingSystemSample(systems.first());
                for (const QChar c : std::as_const(sample)) {
                    if (!c.isSpace() && !raw.supportsCharacter(c)) {
                        sample.clear();
                        break;
                    }
                }
            }
            it = samples_.insert(family, sample);
        }
        return *it;
    }

    QIcon scalable_;
    mutable QHash<QString, QString> samples_;
};

} // namespace

bool isColorFont(const QString& family)
{
    const QRawFont raw = QRawFont::fromFont(QFont(family));
    return raw.isValid() && hasColorGlyphs(raw);
}

void useFastFontPreviews(QFontComboBox* combo)
{
    combo->setItemDelegate(new FontRowDelegate(combo));
}

} // namespace pictura
