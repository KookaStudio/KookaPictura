#include "paragraph_style_dialog.h"

#include "color_picker_dialog.h"

#include <QtGui/QFontDatabase>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

ParagraphStyleDialog::ParagraphStyleDialog(const QString& name, const QString& family,
                                           const CharacterSetting& character,
                                           const ParagraphSetting& paragraph,
                                           bool previewAvailable, QWidget* parent)
    : QDialog(parent)
    , character_(character)
    , paragraph_(paragraph)
    , initialFamily_(family)
    , color_(QColor::fromRgba(character.color))
    , previewAvailable_(previewAvailable)
{
    setWindowTitle(tr("Paragraph Style Options"));

    auto* root = new QVBoxLayout(this);
    auto* body = new QHBoxLayout();
    body->setSpacing(10);

    pageList_ = new QListWidget(this);
    pageList_->setObjectName(QStringLiteral("paragraphStylePages"));
    pageList_->setFixedWidth(190);
    body->addWidget(pageList_);

    auto* right = new QVBoxLayout();
    auto* top = new QHBoxLayout();
    top->addWidget(new QLabel(tr("Style Name:"), this));
    name_ = new QLineEdit(name, this);
    name_->setObjectName(QStringLiteral("paragraphStyleName"));
    top->addWidget(name_, 1);

    auto* buttons = new QVBoxLayout();
    buttons->setSpacing(4);
    auto* ok = new QPushButton(tr("OK"), this);
    ok->setObjectName(QStringLiteral("paragraphStyleOk"));
    ok->setDefault(true);
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setObjectName(QStringLiteral("paragraphStyleCancel"));
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    top->addLayout(buttons);
    right->addLayout(top);

    pageTitle_ = new QLabel(this);
    pageTitle_->setObjectName(QStringLiteral("paragraphStylePageTitle"));
    right->addWidget(pageTitle_);
    auto* separator = new QFrame(this);
    separator->setFrameShape(QFrame::HLine);
    separator->setFrameShadow(QFrame::Sunken);
    right->addWidget(separator);

    pages_ = new QStackedWidget(this);
    right->addWidget(pages_, 1);
    body->addLayout(right, 1);
    root->addLayout(body, 1);

    addPage(tr("Basic Character Formats"), buildBasicCharacterPage());
    addPage(tr("Advanced Character Formats"), buildAdvancedCharacterPage());
    addPage(tr("OpenType Features"), buildOpenTypePage());
    addPage(tr("Indents and Spacing"), buildIndentsPage());
    addPage(tr("Composition"), buildCompositionPage());
    addPage(tr("Justification"), buildJustificationPage());
    addPage(tr("Hyphenation"), buildHyphenationPage());

    connect(pageList_, &QListWidget::currentRowChanged, this, [this](int row) {
        pages_->setCurrentIndex(row);
        const QListWidgetItem* item = pageList_->item(row);
        pageTitle_->setText(item ? item->text() : QString());
    });
    pageList_->setCurrentRow(0);

    preview_ = new QCheckBox(tr("Preview"), this);
    preview_->setObjectName(QStringLiteral("paragraphStylePreview"));
    preview_->setChecked(true);
    preview_->setEnabled(previewAvailable_);
    if (!previewAvailable_) {
        preview_->setToolTip(tr("Preview is available when editing an existing style"));
    }
    connect(preview_, &QCheckBox::toggled, this, [this] { previewChangedIfChecked(); });
    root->addWidget(preview_, 0, Qt::AlignLeft);

    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);

    resize(640, 400);
}

void ParagraphStyleDialog::addPage(const QString& title, QWidget* page)
{
    new QListWidgetItem(title, pageList_);
    pages_->addWidget(page);
}

void ParagraphStyleDialog::refreshStyles(const QString& family, const QString& wanted)
{
    const QSignalBlocker blocker(fontStyle_);
    fontStyle_->clear();
    QStringList styles = QFontDatabase::styles(family);
    if (styles.isEmpty()) {
        styles << tr("Regular");
    }
    fontStyle_->addItems(styles);
    const int index = fontStyle_->findText(wanted);
    fontStyle_->setCurrentIndex(index >= 0 ? index : 0);
}

void ParagraphStyleDialog::refreshSwatch()
{
    QPixmap pixmap(48, 14);
    pixmap.fill(color_);
    QPainter painter(&pixmap);
    painter.setPen(QColor(0, 0, 0, 160));
    painter.drawRect(pixmap.rect().adjusted(0, 0, -1, -1));
    colorSwatch_->setIcon(QIcon(pixmap));
    colorSwatch_->setIconSize(pixmap.size());
}

void ParagraphStyleDialog::watch(QWidget* control)
{
    if (auto* spin = qobject_cast<QDoubleSpinBox*>(control)) {
        connect(spin, &QDoubleSpinBox::valueChanged, this,
                [this] { previewChangedIfChecked(); });
        return;
    }
    if (auto* ispin = qobject_cast<QSpinBox*>(control)) {
        connect(ispin, &QSpinBox::valueChanged, this, [this] { previewChangedIfChecked(); });
        return;
    }
    if (auto* box = qobject_cast<QCheckBox*>(control)) {
        connect(box, &QCheckBox::toggled, this, [this] { previewChangedIfChecked(); });
        return;
    }
    if (auto* combo = qobject_cast<QComboBox*>(control)) {
        // `activated` (a picked item), not `currentIndexChanged`: the popup
        // tracks the mouse, and previewing every hovered entry recomposites the
        // whole document mid-list, which makes the list crawl and close.
        connect(combo, qOverload<int>(&QComboBox::activated), this,
                [this] { previewChangedIfChecked(); });
    }
}

void ParagraphStyleDialog::previewChangedIfChecked()
{
    if (previewAvailable_ && preview_->isChecked()) {
        emit previewChanged();
    }
}

QString ParagraphStyleDialog::styleName() const
{
    const QString text = name_->text().trimmed();
    return text.isEmpty() ? QStringLiteral("Basic Paragraph") : text;
}

QString ParagraphStyleDialog::family() const { return family_->currentText(); }

CharacterSetting ParagraphStyleDialog::characterSetting() const
{
    CharacterSetting c = character_;
    c.font_style = fontStyle_->currentText();
    c.size = size_->value();
    c.leading_mode = leadingMode_->currentIndex() == 1 ? 1 : 0;
    c.leading_value = leading_->value();
    c.kerning_mode = kerningMode_->currentIndex() == 1 ? 2 : 0;
    c.kerning_value = kerning_->value();
    c.tracking = tracking_->value();
    c.horizontal_scale = hScale_->value();
    c.vertical_scale = vScale_->value();
    c.baseline_shift = baselineShift_->value();
    c.color = color_.rgba();
    c.all_caps = caseBox_->currentData().toInt() == 1;
    c.small_caps = caseBox_->currentData().toInt() == 2;
    c.superscript = positionBox_->currentData().toInt() == 1;
    c.subscript = positionBox_->currentData().toInt() == 2;
    c.strikethrough = strikethrough_->isChecked();
    c.underline = underline_->isChecked();
    c.faux_bold = fauxBold_->isChecked();
    c.faux_italic = fauxItalic_->isChecked();
    c.vertical_roman_alignment = verticalRoman_->isChecked();
    c.standard_ligatures = standardLigatures_->isChecked();
    c.contextual_alternates = contextualAlternates_->isChecked();
    c.discretionary_ligatures = discretionaryLigatures_->isChecked();
    c.swash = swash_->isChecked();
    c.oldstyle = oldstyle_->isChecked();
    c.stylistic_alternates = stylisticAlternates_->isChecked();
    c.titling_alternates = titlingAlternates_->isChecked();
    c.ornaments = ornaments_->isChecked();
    c.ordinals = ordinals_->isChecked();
    c.fractions = fractions_->isChecked();
    c.language = language_->currentText();
    return c;
}

ParagraphSetting ParagraphStyleDialog::paragraphSetting() const
{
    ParagraphSetting p = paragraph_;
    p.justify = alignment_->currentData().toInt();
    p.start_indent = startIndent_->value();
    p.end_indent = endIndent_->value();
    p.first_line_indent = firstLineIndent_->value();
    p.space_before = spaceBefore_->value();
    p.space_after = spaceAfter_->value();
    p.composer = composer_->currentIndex() == 1 ? 1 : 0;
    p.hanging = hanging_->isChecked();
    p.word_spacing_min = wordMin_->value();
    p.word_spacing_desired = wordDesired_->value();
    p.word_spacing_max = wordMax_->value();
    p.letter_spacing_min = letterMin_->value();
    p.letter_spacing_desired = letterDesired_->value();
    p.letter_spacing_max = letterMax_->value();
    p.glyph_spacing_min = glyphMin_->value();
    p.glyph_spacing_desired = glyphDesired_->value();
    p.glyph_spacing_max = glyphMax_->value();
    p.auto_leading = autoLeading_->value();
    p.hyphenate = hyphenate_->isChecked();
    p.hyphenate_word_size = hyphenWordSize_->value();
    p.hyphenate_pre = hyphenPre_->value();
    p.hyphenate_post = hyphenPost_->value();
    p.hyphen_limit = hyphenLimit_->value();
    p.hyphenation_zone = hyphenZone_->value() * 12.0;
    p.hyphenate_caps = hyphenateCaps_->isChecked();
    return p;
}

} // namespace pictura

#include "moc_paragraph_style_dialog.cpp"
