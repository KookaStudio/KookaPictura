#include "glyphs_panel.h"

#include "font_combo.h"
#include "jump_slider.h"
#include "tools.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontMetrics>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QSlider>
#include <QtWidgets/QVBoxLayout>

#include <iterator>

namespace pictura {

namespace {

struct Block {
    const char* name;
    char32_t first;
    char32_t last;
};

// The blocks the grid looks in: those a CS6-era Latin font fills, then the
// symbol ranges the panel offers as subsets.
constexpr Block kBlocks[] = {
    {"Basic Latin", 0x0020, 0x007E},
    {"Latin-1 Supplement", 0x00A0, 0x00FF},
    {"Latin Extended-A", 0x0100, 0x017F},
    {"Latin Extended-B", 0x0180, 0x024F},
    {"Greek", 0x0370, 0x03FF},
    {"Cyrillic", 0x0400, 0x04FF},
    {"Punctuation", 0x2000, 0x206F},
    {"Superscripts & Subscripts", 0x2070, 0x209F},
    {"Currency", 0x20A0, 0x20BF},
    {"Letterlike Symbols", 0x2100, 0x214F},
    {"Number Forms", 0x2150, 0x218F},
    {"Arrows", 0x2190, 0x21FF},
    {"Mathematical Operators", 0x2200, 0x22FF},
    {"Geometric Shapes", 0x25A0, 0x25FF},
    {"Miscellaneous Symbols", 0x2600, 0x26FF},
    {"Dingbats", 0x2700, 0x27BF},
};

constexpr int kMinGlyphPoints = 10;
constexpr int kMaxGlyphPoints = 40;
constexpr int kCellPadding = 12;

} // namespace

GlyphsPanel::GlyphsPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(6, 6, 6, 6);
    root->setSpacing(5);

    auto* fontRow = new QHBoxLayout();
    fontRow->setSpacing(4);
    family_ = new QFontComboBox(this);
    family_->setObjectName(QStringLiteral("glyphsFamily"));
    useFastFontPreviews(family_);
    family_->setToolTip(QStringLiteral("Show the glyphs of this font family"));
    // A font combo asks for a very wide minimum; the panel column must not scroll.
    family_->setSizeAdjustPolicy(QComboBox::AdjustToMinimumContentsLengthWithIcon);
    family_->setMinimumContentsLength(10);
    fontRow->addWidget(family_, 2);
    style_ = new QComboBox(this);
    style_->setObjectName(QStringLiteral("glyphsStyle"));
    style_->setToolTip(QStringLiteral("Show the glyphs of this font style"));
    fontRow->addWidget(style_, 1);
    root->addLayout(fontRow);

    subset_ = new QComboBox(this);
    subset_->setObjectName(QStringLiteral("glyphsSubset"));
    subset_->addItem(QStringLiteral("Entire Font"));
    for (const Block& block : kBlocks) {
        subset_->addItem(QLatin1String(block.name));
    }
    subset_->setToolTip(QStringLiteral("Show only part of the font"));
    root->addWidget(subset_);

    // Icon mode with wrapping reflows the glyphs to the panel's width.
    grid_ = new QListWidget(this);
    grid_->setObjectName(QStringLiteral("glyphsGrid"));
    grid_->setViewMode(QListView::IconMode);
    grid_->setResizeMode(QListView::Adjust);
    grid_->setMovement(QListView::Static);
    grid_->setUniformItemSizes(true);
    grid_->setWrapping(true);
    grid_->setSelectionMode(QAbstractItemView::SingleSelection);
    grid_->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    root->addWidget(grid_, 1);

    hint_ = new QLabel(QStringLiteral("Click in the image with the Type tool before inserting a "
                                      "glyph."),
                       this);
    hint_->setObjectName(QStringLiteral("glyphsHint"));
    hint_->setWordWrap(true);
    hint_->hide();
    root->addWidget(hint_);

    auto* zoomRow = new QHBoxLayout();
    zoomRow->setSpacing(4);
    zoomRow->addStretch(1);
    auto* small = new QLabel(QStringLiteral("A"), this);
    QFont smallFont = small->font();
    smallFont.setPointSize(8);
    small->setFont(smallFont);
    zoomRow->addWidget(small);
    zoom_ = new JumpSlider(Qt::Horizontal, this);
    zoom_->setRange(kMinGlyphPoints, kMaxGlyphPoints);
    zoom_->setValue(18);
    zoom_->setFixedWidth(90);
    zoom_->setToolTip(QStringLiteral("Glyph size"));
    zoomRow->addWidget(zoom_);
    auto* large = new QLabel(QStringLiteral("A"), this);
    QFont largeFont = large->font();
    largeFont.setPointSize(13);
    large->setFont(largeFont);
    zoomRow->addWidget(large);
    root->addLayout(zoomRow);

    connect(family_, &QFontComboBox::currentFontChanged, this, [this]() {
        familyChosenHere_ = familyChosenHere_ || !following_;
        rebuildStyles();
        rebuildGlyphs();
    });
    connect(style_, &QComboBox::currentIndexChanged, this, [this]() { rebuildGlyphs(); });
    connect(subset_, &QComboBox::currentIndexChanged, this, [this]() { rebuildGlyphs(); });
    connect(zoom_, &QSlider::valueChanged, this, [this]() { rebuildGlyphs(); });
    connect(grid_, &QListWidget::itemDoubleClicked, this, &GlyphsPanel::chooseGlyph);
    rebuildStyles();
}

void GlyphsPanel::setController(ToolController* controller)
{
    controller_ = controller;
    type_ = controller ? controller->optionsFor<TypeGroup>(ToolId::HorizontalType) : nullptr;
    if (controller_) {
        connect(controller_, &ToolController::typeOptionsChanged, this,
                &GlyphsPanel::followTypeFont);
        followTypeFont();
    }
}

void GlyphsPanel::followTypeFont()
{
    // Browsing to a symbol font here should survive reaching for another tool.
    if (!type_ || familyChosenHere_) {
        return;
    }
    following_ = true;
    family_->setCurrentFont(QFont(type_->get().family));
    following_ = false;
}

void GlyphsPanel::chooseGlyph(QListWidgetItem* item)
{
    if (!item || !controller_) {
        return;
    }
    // A glyph needs a live caret to land, as in Photoshop.
    TextSession* session = controller_->textSession();
    if (!session || !session->textActive()) {
        hint_->show();
        return;
    }
    hint_->hide();
    const QString text = item->text();
    const char32_t code = text.toUcs4().value(0);
    // The type has one font per layer: switch it to the glyph's family only
    // when the current one has nothing at this code point, rather than put an
    // empty box in the document.
    TypeOptions o = type_->get();
    if (!QFontMetrics(QFont(o.family)).inFontUcs4(code)) {
        o.family = family_->currentFont().family();
        type_->set(o);
    }
    session->insertText(text);
}

void GlyphsPanel::showEvent(QShowEvent* event)
{
    QWidget::showEvent(event);
    if (stale_) {
        rebuildGlyphs();
    }
}

void GlyphsPanel::rebuildStyles()
{
    const QSignalBlocker blocker(style_);
    const QString wanted = style_->currentText();
    style_->clear();
    QStringList styles = QFontDatabase::styles(family_->currentFont().family());
    if (styles.isEmpty()) {
        styles << QStringLiteral("Regular");
    }
    style_->addItems(styles);
    const int index = style_->findText(wanted);
    style_->setCurrentIndex(index >= 0 ? index : 0);
}

QFont GlyphsPanel::glyphFont() const
{
    // By style name: a style is whatever the family calls it, not bold/italic bits.
    QFont font = QFontDatabase::font(family_->currentFont().family(), style_->currentText(),
                                     zoom_->value());
    font.setPointSize(zoom_->value());
    return font;
}

void GlyphsPanel::rebuildGlyphs()
{
    if (!isVisible()) {
        stale_ = true;
        return;
    }
    stale_ = false;
    grid_->clear();
    const QFont font = glyphFont();
    const QFontMetrics metrics(font);
    // Subset 0 is the entire font; any other is the block at index - 1.
    const int chosen = subset_->currentIndex();
    const int firstBlock = chosen <= 0 ? 0 : chosen - 1;
    const int lastBlock = chosen <= 0 ? int(std::size(kBlocks)) - 1 : chosen - 1;
    const int cell = zoom_->value() + kCellPadding * 2;
    grid_->setGridSize(QSize(cell, cell));
    for (int b = firstBlock; b <= lastBlock; ++b) {
        for (char32_t code = kBlocks[b].first; code <= kBlocks[b].last; ++code) {
            if (!metrics.inFontUcs4(code)) {
                continue;
            }
            auto* item = new QListWidgetItem(QString::fromUcs4(&code, 1), grid_);
            item->setFont(font);
            item->setTextAlignment(Qt::AlignCenter);
            item->setSizeHint(QSize(cell, cell));
            item->setToolTip(
                QStringLiteral("U+%1").arg(uint(code), 4, 16, QLatin1Char('0')).toUpper());
        }
    }
}

} // namespace pictura
