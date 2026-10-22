// The options bar of the four Type tools.
// Part of OptionsBar; split from options_bar.cpp along the page seam.

#include "options_bar.h"

#include "color_picker_dialog.h"
#include "commands.h"
#include "font_combo.h"
#include "icons.h"

#include <QtCore/QLocale>
#include <QtCore/QSignalBlocker>
#include <QtGui/QDoubleValidator>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtSvg/QSvgRenderer>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <algorithm>

namespace pictura {

namespace {

// The same tool in the other orientation (the bar's Toggle Text Orientation).
ToolId otherOrientation(ToolId id)
{
    switch (id) {
    case ToolId::HorizontalType:
        return ToolId::VerticalType;
    case ToolId::VerticalType:
        return ToolId::HorizontalType;
    case ToolId::HorizontalTypeMask:
        return ToolId::VerticalTypeMask;
    default:
        return ToolId::HorizontalTypeMask;
    }
}

QString sizeText(double size)
{
    return QString::number(size, 'g', 6);
}

} // namespace

QList<int> typeSizes()
{
    // CS6's point list, extended up to 288 as photorust's is.
    return {6, 7, 8, 9, 10, 11, 12, 14, 18, 24, 30, 36, 48, 60, 72, 96, 144, 192, 288};
}

// photorust's paragraph-alignment glyphs: ruled lines flush left, centred, or
// flush right, turned a quarter turn for vertical type (top / centre / bottom).
QIcon typeAlignIcon(int justification, bool vertical, const QColor& color)
{
    // Three ruled lines per icon, each {from, across, to} on a 20 px grid.
    struct Line {
        double from, across, to;
    };
    static const Line lines[3][3] = {
        {{3, 5, 17}, {3, 10, 12}, {3, 15, 15}},
        {{3, 5, 17}, {8, 10, 17}, {5, 15, 17}},
        {{3, 5, 17}, {5.5, 10, 14.5}, {4.2, 15, 15.8}},
    };
    QString path;
    for (const Line& line : lines[std::clamp(justification, 0, 2)]) {
        path += vertical ? QStringLiteral("M%1 %2V%3").arg(line.across).arg(line.from).arg(line.to)
                         : QStringLiteral("M%1 %2H%3").arg(line.from).arg(line.across).arg(line.to);
    }
    const QString svg = QStringLiteral(
                            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20'>"
                            "<path d='%1' fill='none' stroke='%2' stroke-width='1.6' "
                            "stroke-linecap='round'/></svg>")
                            .arg(path, color.name());
    QSvgRenderer renderer(svg.toUtf8());
    QPixmap pixmap(40, 40);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    renderer.render(&painter);
    painter.end();
    return QIcon(pixmap);
}

QIcon typeSwatchIcon(const QColor& color)
{
    QPixmap pixmap(16, 16);
    pixmap.fill(color);
    QPainter painter(&pixmap);
    painter.setPen(QColor(0, 0, 0, 160));
    painter.drawRect(pixmap.rect().adjusted(0, 0, -1, -1));
    return QIcon(pixmap);
}

// Toggle Text Orientation, font family, size (px; a scrolling list or any typed
// value), anti-aliasing (None / Sharp / Crisp / Strong / Smooth), alignment
// (left / centre / right; top / centre / bottom for vertical type), the text
// colour swatch, the Character / Paragraph panels toggle, then Cancel and
// Commit while text is being typed. The four tools share one TypeOptions, so
// each page re-reads it when shown.
// ponytail: no font style.
QWidget* OptionsBar::buildTypePage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    const bool vertical = id == ToolId::VerticalType || id == ToolId::VerticalTypeMask;

    auto* orientation = new QToolButton(page);
    orientation->setObjectName(QStringLiteral("optionsTypeOrientation"));
    orientation->setText(vertical ? QStringLiteral("↕T") : QStringLiteral("↔T"));
    orientation->setToolTip(QStringLiteral("Toggle text orientation"));
    layout->addWidget(orientation);

    auto* family = new QFontComboBox(page);
    family->setObjectName(QStringLiteral("optionsTypeFamily"));
    useFastFontPreviews(family);
    family->setToolTip(QStringLiteral("Font family"));
    layout->addWidget(family);

    auto* size = new QComboBox(page);
    size->setObjectName(QStringLiteral("optionsTypeSize"));
    size->setToolTip(QStringLiteral("Set the font size (px)"));
    size->setEditable(true);
    size->setInsertPolicy(QComboBox::NoInsert);
    size->setMaxVisibleItems(10);
    size->setMinimumContentsLength(5);
    auto* sizeValidator = new QDoubleValidator(1.0, 1296.0, 1, size);
    sizeValidator->setNotation(QDoubleValidator::StandardNotation);
    size->setValidator(sizeValidator);
    for (const int px : typeSizes()) {
        size->addItem(QString::number(px));
    }
    layout->addWidget(size);
    layout->addWidget(new QLabel(QStringLiteral("px"), page));

    auto* antialias = new QComboBox(page);
    antialias->setObjectName(QStringLiteral("optionsTypeAntialias"));
    antialias->setToolTip(QStringLiteral("Anti-aliasing method"));
    antialias->addItems({QStringLiteral("None"), QStringLiteral("Sharp"), QStringLiteral("Crisp"),
                         QStringLiteral("Strong"), QStringLiteral("Smooth")});
    layout->addWidget(antialias);

    auto* align = new QButtonGroup(page);
    const char* const names[] = {"optionsTypeAlignLeft", "optionsTypeAlignRight",
                                 "optionsTypeAlignCenter"};
    const QStringList tips = vertical
        ? QStringList{QStringLiteral("Top align text"), QStringLiteral("Bottom align text"),
                      QStringLiteral("Center text")}
        : QStringList{QStringLiteral("Left align text"), QStringLiteral("Right align text"),
                      QStringLiteral("Center text")};
    // photorust's options-bar glyph tint: light enough to read on the dark bar
    // (the palette's text colour is not themed yet while the page is built).
    const QColor glyph(0xd4, 0xd4, 0xd4);
    // Laid out left, centre, right as CS6 does; the ids are the justification.
    for (int justification : {0, 2, 1}) {
        auto* button = new QToolButton(page);
        button->setObjectName(QLatin1String(names[justification]));
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setIcon(typeAlignIcon(justification, vertical, glyph));
        button->setIconSize(QSize(20, 20));
        button->setToolTip(tips.at(justification));
        align->addButton(button, justification);
        layout->addWidget(button);
    }

    auto* color = new QToolButton(page);
    color->setObjectName(QStringLiteral("optionsTypeColor"));
    color->setToolTip(QStringLiteral("Set the text color"));
    color->setAutoRaise(true);
    color->setIconSize(QSize(16, 16));
    layout->addWidget(color);

    // Type > Warp Text… for the current type layer; the frame enables it.
    auto* warp = new QToolButton(page);
    warp->setObjectName(QStringLiteral("optionsWarpText"));
    warp->setIcon(icon(QStringLiteral("options.warpText")));
    warp->setToolTip(QStringLiteral("Create warped text"));
    warp->setAutoRaise(true);
    warp->setEnabled(false);
    layout->addWidget(warp);
    connect(warp, &QToolButton::clicked, this, &OptionsBar::warpTextRequested);
    warpTextButtons_.push_back(warp);

    // CS6's Toggle the Character and Paragraph panels.
    auto* panels = new QToolButton(page);
    panels->setObjectName(QStringLiteral("optionsToggleCharacterPanel"));
    panels->setIcon(icon(QString::fromLatin1(command_ids::WindowPanelsCharacter)));
    panels->setToolTip(QStringLiteral("Toggle the Character and Paragraph panels"));
    panels->setAutoRaise(true);
    layout->addWidget(panels);
    connect(panels, &QToolButton::clicked, this,
            [this]() { emit panelToggleRequested(QStringLiteral("characterPanel")); });

    auto* cancel = new QToolButton(page);
    cancel->setObjectName(QStringLiteral("optionsTypeCancel"));
    cancel->setText(QStringLiteral("✘"));
    cancel->setToolTip(QStringLiteral("Cancel all current edits (Esc)"));
    layout->addWidget(cancel);
    auto* commit = new QToolButton(page);
    commit->setObjectName(QStringLiteral("optionsTypeCommit"));
    commit->setText(QStringLiteral("✓"));
    commit->setToolTip(QStringLiteral("Commit all current edits (Ctrl+Enter)"));
    layout->addWidget(commit);

    // The Panel button opens the Character and Paragraph panels, as CS6's does.
    auto* panel = new QToolButton(page);
    panel->setObjectName(QStringLiteral("optionsTypePanel"));
    panel->setText(QStringLiteral("⧉"));
    panel->setToolTip(QStringLiteral("Toggle the Character and Paragraph panels"));
    layout->addWidget(panel);
    connect(panel, &QToolButton::clicked, this, [this]() {
        emit panelToggleRequested(QStringLiteral("characterPanel"));
        emit panelToggleRequested(QStringLiteral("paragraphPanel"));
    });

    layout->addStretch(1);
    if (!controller_) {
        return page;
    }

    const auto sync = [=]() {
        const TypeOptions o = type_->get();
        const QSignalBlocker blockFamily(family);
        const QSignalBlocker blockAntialias(antialias);
        family->setCurrentFont(QFont(o.family));
        size->setEditText(sizeText(o.size));
        color->setIcon(typeSwatchIcon(o.color));
        antialias->setCurrentIndex(o.antialias);
        if (QAbstractButton* button = align->button(o.justification)) {
            button->setChecked(true);
        }
        TextSession* session = textSession();
        const bool editing = session && session->textActive();
        cancel->setEnabled(editing);
        commit->setEnabled(editing);
    };
    sync();
    const auto update = [this](auto edit) {
        TypeOptions o = type_->get();
        edit(o);
        type_->set(o);
    };
    connect(controller_, &ToolController::activeToolChanged, page, sync);
    connect(controller_, &ToolController::typeOptionsChanged, page, sync);
    connect(controller_, &ToolController::textEditingChanged, page, [cancel, commit](bool on) {
        cancel->setEnabled(on);
        commit->setEnabled(on);
    });
    connect(orientation, &QToolButton::clicked, this,
            [this, id]() { controller_->setActiveTool(otherOrientation(id)); });
    connect(family, &QFontComboBox::currentFontChanged, this, [update](const QFont& font) {
        update([&font](TypeOptions& o) { o.family = font.family(); });
    });
    // A size applies on a pick or Enter (and when the field loses focus), not
    // per keystroke: typing "12" must not restyle the text as 1 px first.
    const auto applySize = [this, size, update]() {
        bool ok = false;
        // C locale: sizeText() formats with QString::number, so the field's own
        // output must re-parse under every system locale (comma decimals).
        const double v = QLocale::c().toDouble(size->currentText(), &ok);
        if (!ok || v < 1.0 || v > 1296.0) {
            size->setEditText(sizeText(type_->get().size));
            return;
        }
        if (v != type_->get().size) {
            update([v](TypeOptions& o) { o.size = v; });
        }
    };
    connect(size, &QComboBox::activated, this, applySize);
    connect(size->lineEdit(), &QLineEdit::editingFinished, this, applySize);
    // The swatch sets the foreground, which the text colour follows (CS6 links
    // the two); a type layer being edited or selected takes the colour.
    connect(color, &QToolButton::clicked, this, [this]() {
        const QColor picked = ColorPickerDialog::getColor(type_->get().color, this,
                                                          QStringLiteral("Text Color"));
        if (picked.isValid()) {
            controller_->setForeground(picked);
        }
    });
    connect(antialias, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](TypeOptions& o) { o.antialias = i; }); });
    connect(align, &QButtonGroup::idClicked, this,
            [update](int j) { update([j](TypeOptions& o) { o.justification = j; }); });
    connect(cancel, &QToolButton::clicked, this, [this]() {
        if (textSession()) {
            textSession()->cancelText();
        }
    });
    connect(commit, &QToolButton::clicked, this, [this]() {
        if (textSession()) {
            textSession()->commitText();
        }
    });
    return page;
}

} // namespace pictura
