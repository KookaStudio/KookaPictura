#include "info_panel.h"

#include "icons.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QEvent>
#include <QtCore/QPair>
#include <QtCore/QStringList>
#include <QtCore/QtGlobal>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QFont>
#include <QtGui/QScreen>
#include <QtWidgets/QApplication>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <cmath>
#include <cstdint>

namespace pictura {

namespace {

// CS6 shows CMYK as 0-100%; QColor's CMYK components are 0-255.
int cmykPercent(int component)
{
    return int(std::lround(component * 100.0 / 255.0));
}

// Photoshop's "Doc:" memory line: kilobytes below a megabyte, else megabytes.
QString formatBytes(double bytes)
{
    if (bytes >= 1024.0 * 1024.0) {
        return QStringLiteral("%1M").arg(bytes / (1024.0 * 1024.0), 0, 'f', 1);
    }
    return QStringLiteral("%1K").arg(bytes / 1024.0, 0, 'f', 1);
}

QString capitaliseFirst(QString text)
{
    for (int i = 0; i < text.size(); ++i) {
        if (text.at(i).isLetter()) {
            text[i] = text.at(i).toUpper();
            break;
        }
    }
    return text;
}

struct Lab {
    double l;
    double a;
    double b;
};

// sRGB (D65) to CIE L*a*b*; L 0-100, a/b roughly -128..127.
Lab srgbToLab(const QColor& color)
{
    const auto linear = [](double v) {
        v /= 255.0;
        return v <= 0.04045 ? v / 12.92 : std::pow((v + 0.055) / 1.055, 2.4);
    };
    const double r = linear(color.red());
    const double g = linear(color.green());
    const double b = linear(color.blue());
    const double x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047;
    const double y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    const double z = (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / 1.08883;
    const auto f = [](double t) {
        const double e = 216.0 / 24389.0;
        const double k = 24389.0 / 27.0;
        return t > e ? std::cbrt(t) : (k * t + 16.0) / 116.0;
    };
    const double fx = f(x);
    const double fy = f(y);
    const double fz = f(z);
    return Lab{116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)};
}

} // namespace

InfoPanel::InfoPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(4, 4, 4, 4);
    outer->setSpacing(4);

    grid_ = new QGridLayout;
    grid_->setContentsMargins(0, 0, 0, 0);
    grid_->setHorizontalSpacing(0);
    grid_->setVerticalSpacing(0);
    grid_->setColumnStretch(0, 1);
    grid_->setColumnStretch(1, 1);
    outer->addLayout(grid_);

    topLeft_ = addReadout(grid_, 0, 0,
                          {QStringLiteral("R"), QStringLiteral("G"), QStringLiteral("B")},
                          QStringLiteral("tool.eyedropper"), QStringLiteral("8-bit"),
                          MenuKind::Color);
    topLeft_->colorMode = ColorReadout::ActualColor;
    topRight_ = addReadout(grid_, 0, 1,
                           {QStringLiteral("C"), QStringLiteral("M"), QStringLiteral("Y"),
                            QStringLiteral("K")},
                           QStringLiteral("tool.eyedropper"), QStringLiteral("8-bit"),
                           MenuKind::Color);
    topRight_->colorMode = ColorReadout::Cmyk;
    position_ = addReadout(grid_, 1, 0, {QStringLiteral("X"), QStringLiteral("Y")},
                           QStringLiteral("info.crosshair"), QString(), MenuKind::Unit);
    size_ = addReadout(grid_, 1, 1, {QStringLiteral("W"), QStringLiteral("H")},
                       QStringLiteral("info.bounds"), QString(), MenuKind::None);

    auto* rule = new QWidget(this);
    rule->setObjectName(QStringLiteral("infoRule"));
    rule->setAttribute(Qt::WA_StyledBackground, true);
    rule->setFixedHeight(1);
    outer->addWidget(rule);

    docLabel_ = new QLabel(this);
    docLabel_->setObjectName(QStringLiteral("infoDocSize"));
    outer->addWidget(docLabel_);

    samplersLabel_ = new QLabel(this);
    samplersLabel_->setObjectName(QStringLiteral("infoSamplers"));
    samplersLabel_->setWordWrap(true);
    outer->addWidget(samplersLabel_);

    toolLabel_ = new QLabel(this);
    toolLabel_->setObjectName(QStringLiteral("infoTool"));
    toolLabel_->setWordWrap(true);
    toolLabel_->setAlignment(Qt::AlignLeft | Qt::AlignTop);
    int baseFontPx = QApplication::font().pixelSize();
    if (baseFontPx <= 0) {
        baseFontPx = 12;
    }
    QFont toolFont = toolLabel_->font();
    toolFont.setPixelSize(baseFontPx - 2);
    toolLabel_->setFont(toolFont);
    outer->addWidget(toolLabel_);

    // The readouts keep their natural height: a trailing stretch absorbs the
    // panel's leftover space instead of the top row flex-growing into it.
    outer->addStretch(1);

    refresh();
}

InfoPanel::Readout* InfoPanel::addReadout(QGridLayout* grid, int row, int column,
                                          const QStringList& keys, const QString& iconId,
                                          const QString& footer, MenuKind menu)
{
    auto* block = new QWidget(this);
    block->setObjectName(QStringLiteral("infoBlock"));
    block->setAttribute(Qt::WA_StyledBackground, true);
    // The readout grid draws only its inner cross (styled in theme.cpp), so the
    // last column/row keeps no outer line. ponytail: fixed 2x2 grid.
    block->setProperty("gridRight", column == 0);
    block->setProperty("gridBottom", row == 0);
    auto* layout = new QGridLayout(block);
    layout->setContentsMargins(4, 3, 4, 3);
    layout->setHorizontalSpacing(4);
    layout->setVerticalSpacing(1);

    auto* readout = new Readout;
    readout->widget = block;

    readout->button = new QToolButton(block);
    readout->button->setIcon(pictura::icon(iconId));
    readout->button->setIconSize(QSize(14, 14));
    readout->button->setAutoRaise(true);
    readout->button->setCursor(Qt::PointingHandCursor);
    layout->addWidget(readout->button, 0, 0, 1, 1, Qt::AlignTop);

    auto* keysHost = new QWidget(block);
    auto* valuesHost = new QWidget(block);
    readout->keysHost = new QGridLayout(keysHost);
    readout->valuesHost = new QVBoxLayout(valuesHost);
    readout->keysHost->setContentsMargins(0, 0, 0, 0);
    readout->keysHost->setHorizontalSpacing(4);
    readout->keysHost->setVerticalSpacing(1);
    readout->valuesHost->setContentsMargins(0, 0, 0, 0);
    readout->valuesHost->setSpacing(1);
    layout->addWidget(keysHost, 0, 1);
    layout->addWidget(valuesHost, 0, 2);
    layout->setColumnStretch(2, 1);

    if (!footer.isEmpty()) {
        readout->footer = new QLabel(footer, block);
        readout->footer->setObjectName(QStringLiteral("infoFooter"));
        readout->footer->setAlignment(Qt::AlignLeft | Qt::AlignBottom);
        // Keys live in keysHost's own rows, so the footer goes in block row 1;
        // the stretch pins it to the block's bottom whatever the key count is.
        layout->addWidget(readout->footer, 1, 1, 1, 2);
        layout->setRowStretch(1, 1);
    }

    rebuildRows(readout, keys);

    if (menu != MenuKind::None) {
        auto* m = new QMenu(readout->button);
        m->installEventFilter(this);
        if (menu == MenuKind::Color) {
            const QList<QPair<QString, ColorReadout>> options = {
                {QStringLiteral("Actual Color"), ColorReadout::ActualColor},
                {QStringLiteral("Proof Color"), ColorReadout::ProofColor},
                {QStringLiteral("Grayscale"), ColorReadout::Grayscale},
                {QStringLiteral("RGB"), ColorReadout::Rgb},
                {QStringLiteral("HSB"), ColorReadout::Hsb},
                {QStringLiteral("CMYK"), ColorReadout::Cmyk},
                {QStringLiteral("Lab"), ColorReadout::Lab},
                {QStringLiteral("Total Ink"), ColorReadout::TotalInk},
                {QStringLiteral("Opacity"), ColorReadout::Opacity},
            };
            const int separatorAt[] = {2, 7};
            int nextSeparator = 0;
            for (int i = 0; i < options.size(); ++i) {
                if (nextSeparator < 2 && i == separatorAt[nextSeparator]) {
                    m->addSeparator();
                    ++nextSeparator;
                }
                const auto& option = options.at(i);
                QAction* action = m->addAction(option.first);
                action->setCheckable(true);
                action->setData(int(option.second));
                connect(action, &QAction::triggered, this,
                        [this, readout, mode = option.second] { applyColorMode(readout, mode); });
            }
            m->addSeparator();
            for (const int bits : {8, 16, 32}) {
                QAction* action = m->addAction(QStringLiteral("%1-bit").arg(bits));
                action->setCheckable(true);
                action->setData(-bits);
                connect(action, &QAction::triggered, this,
                        [this, readout, bits] { setBitDepth(readout, bits); });
            }
            connect(m, &QMenu::aboutToShow, this, [readout, m] {
                for (QAction* action : m->actions()) {
                    if (action->isSeparator()) {
                        continue;
                    }
                    const int data = action->data().toInt();
                    action->setChecked(data < 0 ? -data == readout->bits
                                                : data == int(readout->colorMode));
                }
            });
        } else {
            const QList<QPair<QString, MeasureUnit>> options = {
                {QStringLiteral("Pixels"), MeasureUnit::Pixels},
                {QStringLiteral("Inches"), MeasureUnit::Inches},
                {QStringLiteral("Centimeters"), MeasureUnit::Centimeters},
                {QStringLiteral("Millimeters"), MeasureUnit::Millimeters},
                {QStringLiteral("Points"), MeasureUnit::Points},
                {QStringLiteral("Picas"), MeasureUnit::Picas},
                {QStringLiteral("Percent"), MeasureUnit::Percent},
            };
            for (const auto& option : options) {
                QAction* action = m->addAction(option.first);
                action->setCheckable(true);
                action->setData(int(option.second));
                connect(action, &QAction::triggered, this,
                        [this, readout, unit = option.second] { applyUnit(readout, unit); });
            }
            connect(m, &QMenu::aboutToShow, this, [readout, m] {
                for (QAction* action : m->actions()) {
                    action->setChecked(action->data().toInt() == int(readout->unit));
                }
            });
        }
        readout->button->setMenu(m);
        readout->button->setPopupMode(QToolButton::InstantPopup);
    }

    grid->addWidget(block, row, column);
    return readout;
}

// QToolButton drops its menu below the icon (or above when out of room), in
// front of the readout; open ours beside the button so the icon stays visible.
// ponytail: LTR and one screen only; flip the sides for RTL or fall through to
// a neighbouring screen's geometry if placement ever matters beyond that.
bool InfoPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (event->type() == QEvent::Show) {
        if (auto* menu = qobject_cast<QMenu*>(watched)) {
            if (auto* button = qobject_cast<QToolButton*>(menu->parentWidget())) {
                // Polish and resize before measuring: on the first show the menu
                // is not yet laid out, so its raw size hint clips the items.
                menu->ensurePolished();
                menu->adjustSize();
                if (QScreen* screen = button->screen()) {
                    const QSize menuSize = menu->sizeHint();
                    const QRect area = screen->availableGeometry();
                    const QPoint buttonTopLeft = button->mapToGlobal(QPoint(0, 0));
                    QPoint pos = buttonTopLeft + QPoint(button->width() + 1, 0);
                    if (pos.x() + menuSize.width() > area.right() + 1) {
                        pos.setX(buttonTopLeft.x() - 1 - menuSize.width());
                    }
                    // Neither side fits: keep QToolButton's own placement.
                    if (pos.x() >= area.left() && pos.x() + menuSize.width() <= area.right() + 1) {
                        const int maxY = qMax(area.top(), area.bottom() - menuSize.height() + 1);
                        menu->move(QPoint(pos.x(), qBound(area.top(), pos.y(), maxY)));
                    }
                }
            }
        }
    }
    return QWidget::eventFilter(watched, event);
}

void InfoPanel::rebuildRows(Readout* readout, const QStringList& keys)
{
    while (QLayoutItem* item = readout->keysHost->takeAt(0)) {
        delete item->widget();
        delete item;
    }
    while (QLayoutItem* item = readout->valuesHost->takeAt(0)) {
        delete item->widget();
        delete item;
    }
    readout->values.clear();
    readout->keyNames = keys;
    for (int i = 0; i < keys.size(); ++i) {
        const QString& key = keys.at(i);
        // Left-aligned keys in their own column, the ":" right-aligned in a
        // second one: proportional fonts otherwise stagger both.
        auto* keyLabel = new QLabel(key, readout->widget);
        keyLabel->setObjectName(QStringLiteral("infoKey"));
        keyLabel->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        auto* colonLabel = new QLabel(QStringLiteral(":"), readout->widget);
        colonLabel->setObjectName(QStringLiteral("infoKey"));
        colonLabel->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        auto* valueLabel = new QLabel(readout->widget);
        valueLabel->setObjectName(QStringLiteral("infoValue"));
        valueLabel->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        valueLabel->setMinimumWidth(26);
        readout->keysHost->addWidget(keyLabel, i, 0);
        readout->keysHost->addWidget(colonLabel, i, 1);
        readout->valuesHost->addWidget(valueLabel);
        readout->values.append(valueLabel);
    }
}

void InfoPanel::setValues(Readout* readout, const QStringList& values)
{
    if (!readout) {
        return;
    }
    for (int i = 0; i < readout->values.size(); ++i) {
        readout->values.at(i)->setText(i < values.size() ? values.at(i) : QString());
    }
}

void InfoPanel::applyColorMode(Readout* readout, ColorReadout mode)
{
    readout->colorMode = mode;
    QStringList keys;
    switch (mode) {
    case ColorReadout::ActualColor:
    case ColorReadout::ProofColor:
    case ColorReadout::Rgb:
        keys << QStringLiteral("R") << QStringLiteral("G") << QStringLiteral("B");
        break;
    case ColorReadout::Grayscale:
        keys << QStringLiteral("K");
        break;
    case ColorReadout::Hsb:
        keys << QStringLiteral("H") << QStringLiteral("S") << QStringLiteral("B");
        break;
    case ColorReadout::Cmyk:
        keys << QStringLiteral("C") << QStringLiteral("M") << QStringLiteral("Y")
             << QStringLiteral("K");
        break;
    case ColorReadout::Lab:
        keys << QStringLiteral("L") << QStringLiteral("a") << QStringLiteral("b");
        break;
    case ColorReadout::TotalInk:
        keys << QStringLiteral("Ink");
        break;
    case ColorReadout::Opacity:
        keys << QStringLiteral("Op");
        break;
    }
    rebuildRows(readout, keys);
    refresh();
}

void InfoPanel::applyUnit(Readout* readout, MeasureUnit unit)
{
    readout->unit = unit;
    (readout == position_ ? size_ : position_)->unit = unit;
    refresh();
}

void InfoPanel::setBitDepth(Readout* readout, int bits)
{
    readout->bits = bits;
    if (readout->footer) {
        readout->footer->setText(QStringLiteral("%1-bit").arg(bits));
    }
    refresh();
}

void InfoPanel::refreshColorBlock(Readout* readout, const QColor& color)
{
    const int bits = readout->bits;
    switch (readout->colorMode) {
    // ponytail: no proofing engine; Proof Color reads the same sRGB values as Actual Color.
    case ColorReadout::ActualColor:
    case ColorReadout::ProofColor:
        setValues(readout, {formatChannel(color.red(), bits), formatChannel(color.green(), bits),
                            formatChannel(color.blue(), bits)});
        break;
    case ColorReadout::Grayscale:
        setValues(readout, {formatChannel(qGray(color.rgb()), bits)});
        break;
    case ColorReadout::Rgb:
        setValues(readout, {formatChannel(color.red(), bits), formatChannel(color.green(), bits),
                            formatChannel(color.blue(), bits)});
        break;
    case ColorReadout::Hsb: {
        const int hue = color.hue() < 0 ? 0 : color.hue();
        setValues(readout, {QString::number(hue),
                            QString::number(qRound(color.saturationF() * 100.0)),
                            QString::number(qRound(color.valueF() * 100.0))});
        break;
    }
    case ColorReadout::Cmyk: {
        const QColor cmyk = color.toCmyk();
        setValues(readout, {QString::number(cmykPercent(cmyk.cyan())),
                            QString::number(cmykPercent(cmyk.magenta())),
                            QString::number(cmykPercent(cmyk.yellow())),
                            QString::number(cmykPercent(cmyk.black()))});
        break;
    }
    case ColorReadout::Lab: {
        const Lab lab = srgbToLab(color);
        setValues(readout, {QString::number(qRound(lab.l)), QString::number(qRound(lab.a)),
                            QString::number(qRound(lab.b))});
        break;
    }
    case ColorReadout::TotalInk: {
        const QColor cmyk = color.toCmyk();
        const int total = cmykPercent(cmyk.cyan()) + cmykPercent(cmyk.magenta())
                          + cmykPercent(cmyk.yellow()) + cmykPercent(cmyk.black());
        setValues(readout, {QString::number(total)});
        break;
    }
    case ColorReadout::Opacity:
        setValues(readout, {QString::number(qRound(color.alpha() / 255.0 * 100.0))});
        break;
    }
}

// ponytail: the engine samples 8-bit; 16/32-bit readouts scale the 8-bit sample
// rather than reading a true deep-color document.
QString InfoPanel::formatChannel(int value, int bits) const
{
    switch (bits) {
    case 16:
        return QString::number(value * 257);
    case 32:
        return QString::number(value / 255.0, 'f', 3);
    default:
        return QString::number(value);
    }
}

QString InfoPanel::formatMeasure(double value, MeasureUnit unit, double percentBase) const
{
    switch (unit) {
    case MeasureUnit::Pixels:
        return QString::number(qRound(value));
    case MeasureUnit::Inches:
        return QString::number(value / 72.0, 'f', 2);
    case MeasureUnit::Centimeters:
        return QString::number(value / 72.0 * 2.54, 'f', 2);
    case MeasureUnit::Millimeters:
        return QString::number(value / 72.0 * 25.4, 'f', 2);
    case MeasureUnit::Points:
        return QString::number(value / 72.0 * 72.0, 'f', 2);
    case MeasureUnit::Picas:
        return QString::number(value / 72.0 * 6.0, 'f', 2);
    case MeasureUnit::Percent:
        return percentBase > 0.0 ? QString::number(value / percentBase * 100.0, 'f', 2)
                                 : QString();
    }
    return QString();
}

QString InfoPanel::blockText(const Readout* readout) const
{
    QStringList parts;
    for (int i = 0; i < readout->values.size(); ++i) {
        const QString key = i < readout->keyNames.size() ? readout->keyNames.at(i) : QString();
        parts << QStringLiteral("%1 : %2").arg(key, readout->values.at(i)->text());
    }
    return parts.join(QStringLiteral("  "));
}

QString InfoPanel::colorModeName(ColorReadout mode) const
{
    switch (mode) {
    case ColorReadout::ActualColor:
        return QStringLiteral("Actual Color");
    case ColorReadout::ProofColor:
        return QStringLiteral("Proof Color");
    case ColorReadout::Grayscale:
        return QStringLiteral("Grayscale");
    case ColorReadout::Rgb:
        return QStringLiteral("RGB");
    case ColorReadout::Hsb:
        return QStringLiteral("HSB");
    case ColorReadout::Cmyk:
        return QStringLiteral("CMYK");
    case ColorReadout::Lab:
        return QStringLiteral("Lab");
    case ColorReadout::TotalInk:
        return QStringLiteral("Total Ink");
    case ColorReadout::Opacity:
        return QStringLiteral("Opacity");
    }
    return QString();
}

void InfoPanel::rebuildTopRight()
{
    grid_->removeWidget(topRight_->widget);
    delete topRight_->widget;
    delete topRight_;
    if (rulerMode_) {
        topRight_ = addReadout(grid_, 0, 1, {QStringLiteral("A"), QStringLiteral("L")},
                               QStringLiteral("info.protractor"), QString(), MenuKind::None);
    } else {
        topRight_ = addReadout(grid_, 0, 1,
                               {QStringLiteral("C"), QStringLiteral("M"), QStringLiteral("Y"),
                                QStringLiteral("K")},
                               QStringLiteral("tool.eyedropper"), QStringLiteral("8-bit"),
                               MenuKind::Color);
        topRight_->colorMode = ColorReadout::Cmyk;
    }
}

void InfoPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

void InfoPanel::setCursorPosition(const QPointF& imagePos)
{
    cursor_ = imagePos;
    refresh();
}

void InfoPanel::setRulerMode(bool on)
{
    if (rulerMode_ == on) {
        return;
    }
    rulerMode_ = on;
    rebuildTopRight();
    refresh();
}

void InfoPanel::setToolInfo(const QString& name, const QStringList& hints)
{
    toolName_ = name;
    toolHints_ = hints;
    refreshToolLabel();
}

void InfoPanel::refreshToolLabel()
{
    if (!toolLabel_) {
        return;
    }
    if (toolName_.isEmpty() && toolHints_.isEmpty()) {
        toolLabel_->clear();
        return;
    }
    QStringList lines;
    if (!toolName_.isEmpty()) {
        lines << toolName_;
    }
    for (const QString& hint : toolHints_) {
        lines << capitaliseFirst(hint);
    }
    toolLabel_->setText(lines.join(QLatin1Char('\n')));
}

void InfoPanel::refresh()
{
    if (!view_ || !view_->has_document()) {
        setValues(topLeft_, {});
        setValues(topRight_, {});
        setValues(position_, {});
        setValues(size_, {});
        docLabel_->clear();
        samplersLabel_->clear();
        samplersLabel_->setVisible(false);
        return;
    }

    const int width = view_->document_width();
    const int height = view_->document_height();

    const int x = int(std::floor(cursor_.x()));
    const int y = int(std::floor(cursor_.y()));

    setValues(position_, {formatMeasure(x, position_->unit, width),
                          formatMeasure(y, position_->unit, height)});

    const bool inside = x >= 0 && y >= 0 && x < width && y < height;
    if (inside) {
        const QColor color = QColor::fromRgba(QRgb(view_->sample_argb(x, y)));
        refreshColorBlock(topLeft_, color);
        if (!rulerMode_) {
            refreshColorBlock(topRight_, color);
        }
    } else {
        setValues(topLeft_, {});
        if (!rulerMode_) {
            setValues(topRight_, {});
        }
    }

    if (rulerMode_) {
        // [X, Y, W, H, A, D1]; empty when no measuring line exists.
        const ::rust::Vec<double> m = ruler_measurement(*view_);
        if (m.size() == 6) {
            setValues(topRight_, {QStringLiteral("%1°").arg(m[4], 0, 'f', 1),
                                  QString::number(m[5], 'f', 1)});
            setValues(size_, {formatMeasure(m[2], size_->unit, width),
                              formatMeasure(m[3], size_->unit, height)});
        } else {
            setValues(topRight_, {});
            setValues(size_, {});
        }
    } else {
        const QString bounds = view_->selection_bounds();
        const QStringList parts = bounds.split(QLatin1Char(' '), Qt::SkipEmptyParts);
        if (parts.size() == 4) {
            setValues(size_, {formatMeasure(parts[2].toDouble(), size_->unit, width),
                              formatMeasure(parts[3].toDouble(), size_->unit, height)});
        } else {
            setValues(size_, {});
        }
    }

    const ::rust::Vec<double> bytes = document_size_bytes(*view_);
    if (bytes.size() == 2) {
        docLabel_->setText(tr("Doc: %1/%2").arg(formatBytes(bytes[0]), formatBytes(bytes[1])));
    } else {
        docLabel_->clear();
    }

    // CS6 lists the colour samplers below the readouts, numbered as on the
    // canvas; each reads the composite at its pixel.
    QStringList samplers;
    const int count = marker_count(*view_, 0);
    for (int i = 0; i < count; ++i) {
        const ::rust::Vec<std::int32_t> p = marker_at(*view_, 0, i);
        if (p.size() != 2) {
            continue;
        }
        const bool pixelInside = p[0] >= 0 && p[1] >= 0 && p[0] < width && p[1] < height;
        const QColor c = QColor::fromRgba(QRgb(pixelInside ? view_->sample_argb(p[0], p[1]) : 0));
        samplers << (pixelInside ? QStringLiteral("#%1  %2, %3  R %4  G %5  B %6")
                                       .arg(i + 1)
                                       .arg(p[0])
                                       .arg(p[1])
                                       .arg(c.red())
                                       .arg(c.green())
                                       .arg(c.blue())
                                 : QStringLiteral("#%1  %2, %3  —").arg(i + 1).arg(p[0]).arg(p[1]));
    }
    samplersLabel_->setText(samplers.join(QLatin1Char('\n')));
    samplersLabel_->setVisible(!samplers.isEmpty());
}

QString InfoPanel::samplerTextForTest() const { return samplersLabel_->text(); }

QString InfoPanel::colorBlockTextForTest(int index) const
{
    return blockText(index == 1 ? topRight_ : topLeft_);
}

QString InfoPanel::colorFooterForTest(int index) const
{
    const Readout* readout = index == 1 ? topRight_ : topLeft_;
    return readout->footer ? readout->footer->text() : QString();
}

QString InfoPanel::colorModeForTest(int index) const
{
    return colorModeName((index == 1 ? topRight_ : topLeft_)->colorMode);
}

bool InfoPanel::sizeBlockHasMenuForTest() const { return size_->button->menu() != nullptr; }

QString InfoPanel::positionTextForTest() const
{
    QStringList values;
    for (const QLabel* label : position_->values) {
        values << label->text();
    }
    return values.join(QString()).isEmpty() ? QString() : values.join(QStringLiteral(", "));
}

QString InfoPanel::sizeTextForTest() const
{
    QStringList values;
    for (const QLabel* label : size_->values) {
        values << label->text();
    }
    return values.join(QString()).isEmpty() ? QString() : values.join(QStringLiteral(" × "));
}

QString InfoPanel::rulerTextForTest() const
{
    QStringList values;
    for (const QLabel* label : topRight_->values) {
        values << label->text();
    }
    return values.join(QString()).isEmpty() ? QString() : values.join(QStringLiteral("  "));
}

QString InfoPanel::docTextForTest() const { return docLabel_->text(); }

QString InfoPanel::toolTextForTest() const
{
    return toolLabel_ ? toolLabel_->text() : QString();
}

void InfoPanel::setColorModeForTest(int index, const QString& mode)
{
    Readout* readout = index == 1 ? topRight_ : topLeft_;
    if (mode.compare(QStringLiteral("Actual Color"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::ActualColor);
    } else if (mode.compare(QStringLiteral("Proof Color"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::ProofColor);
    } else if (mode.compare(QStringLiteral("Grayscale"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::Grayscale);
    } else if (mode.compare(QStringLiteral("RGB"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::Rgb);
    } else if (mode.compare(QStringLiteral("HSB"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::Hsb);
    } else if (mode.compare(QStringLiteral("CMYK"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::Cmyk);
    } else if (mode.compare(QStringLiteral("Lab"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::Lab);
    } else if (mode.compare(QStringLiteral("Total Ink"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::TotalInk);
    } else if (mode.compare(QStringLiteral("Opacity"), Qt::CaseInsensitive) == 0) {
        applyColorMode(readout, ColorReadout::Opacity);
    }
}

void InfoPanel::setBitDepthForTest(int index, int bits)
{
    setBitDepth(index == 1 ? topRight_ : topLeft_, bits);
}

QStringList InfoPanel::colorMenuTextsForTest(int index) const
{
    const Readout* readout = index == 1 ? topRight_ : topLeft_;
    QStringList texts;
    if (readout && readout->button && readout->button->menu()) {
        for (QAction* action : readout->button->menu()->actions()) {
            texts << (action->isSeparator() ? QStringLiteral("---") : action->text());
        }
    }
    return texts;
}

void InfoPanel::setMeasurementUnitForTest(int index, const QString& unit)
{
    Readout* readout = index == 1 ? size_ : position_;
    if (unit.compare(QStringLiteral("Pixels"), Qt::CaseInsensitive) == 0) {
        applyUnit(readout, MeasureUnit::Pixels);
    } else if (unit.compare(QStringLiteral("Inches"), Qt::CaseInsensitive) == 0) {
        applyUnit(readout, MeasureUnit::Inches);
    } else if (unit.compare(QStringLiteral("Centimeters"), Qt::CaseInsensitive) == 0) {
        applyUnit(readout, MeasureUnit::Centimeters);
    } else if (unit.compare(QStringLiteral("Millimeters"), Qt::CaseInsensitive) == 0) {
        applyUnit(readout, MeasureUnit::Millimeters);
    } else if (unit.compare(QStringLiteral("Points"), Qt::CaseInsensitive) == 0) {
        applyUnit(readout, MeasureUnit::Points);
    } else if (unit.compare(QStringLiteral("Picas"), Qt::CaseInsensitive) == 0) {
        applyUnit(readout, MeasureUnit::Picas);
    } else if (unit.compare(QStringLiteral("Percent"), Qt::CaseInsensitive) == 0) {
        applyUnit(readout, MeasureUnit::Percent);
    }
}

} // namespace pictura
