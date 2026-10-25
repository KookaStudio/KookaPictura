#include "paragraph_style_dialog.h"

#include "color_picker_dialog.h"
#include "font_combo.h"

#include <QtGui/QFont>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <array>

namespace pictura {

namespace {

QDoubleSpinBox* spin(QWidget* parent, const char* name, double lo, double hi, int decimals,
                     const QString& suffix)
{
    auto* box = new QDoubleSpinBox(parent);
    box->setObjectName(QLatin1String(name));
    box->setRange(lo, hi);
    box->setDecimals(decimals);
    if (!suffix.isEmpty()) {
        box->setSuffix(suffix);
    }
    box->setKeyboardTracking(false);
    return box;
}

QSpinBox* intSpin(QWidget* parent, const char* name, int lo, int hi, const QString& suffix)
{
    auto* box = new QSpinBox(parent);
    box->setObjectName(QLatin1String(name));
    box->setRange(lo, hi);
    if (!suffix.isEmpty()) {
        box->setSuffix(suffix);
    }
    box->setKeyboardTracking(false);
    return box;
}

QCheckBox* check(QWidget* parent, const char* name, const QString& text, bool on)
{
    auto* box = new QCheckBox(text, parent);
    box->setObjectName(QLatin1String(name));
    box->setChecked(on);
    return box;
}

} // namespace

QWidget* ParagraphStyleDialog::buildBasicCharacterPage()
{
    auto* page = new QWidget(this);
    auto* row = new QHBoxLayout(page);
    row->setSpacing(16);
    auto* form = new QFormLayout();
    form->setLabelAlignment(Qt::AlignRight);

    // The Type options bar's own widget: an editable font combo drops a plain
    // list under the field, where a read-only combo's screen-tall menu popup
    // auto-scrolls under the pointer and flickers.
    family_ = new QFontComboBox(page);
    family_->setObjectName(QStringLiteral("paragraphStyleFamily"));
    useFastFontPreviews(family_);
    if (!initialFamily_.isEmpty()) {
        // A style's family need not be installed; keep its name rather than
        // letting the combo substitute the nearest match.
        if (family_->findText(initialFamily_) >= 0) {
            family_->setCurrentFont(QFont(initialFamily_));
        } else {
            family_->setEditText(initialFamily_);
        }
    }
    form->addRow(tr("Font Family:"), family_);

    fontStyle_ = new QComboBox(page);
    fontStyle_->setObjectName(QStringLiteral("paragraphStyleFontStyle"));
    form->addRow(tr("Font Style:"), fontStyle_);
    refreshStyles(family_->currentText(), character_.font_style);
    // `activated`, not `currentTextChanged`: the popup tracks the mouse, and
    // rebuilding the style list on every hover flickers.
    connect(family_, qOverload<int>(&QComboBox::activated), this, [this](int) {
        refreshStyles(family_->currentText(), fontStyle_->currentText());
    });

    size_ = spin(page, "paragraphStyleSize", 1.0, 1296.0, 1, tr(" pt"));
    size_->setValue(character_.size);
    form->addRow(tr("Size:"), size_);

    leadingMode_ = new QComboBox(page);
    leadingMode_->setObjectName(QStringLiteral("paragraphStyleLeadingMode"));
    leadingMode_->addItems({tr("(Auto)"), tr("Fixed")});
    leadingMode_->setCurrentIndex(character_.leading_mode == 1 ? 1 : 0);
    leading_ = spin(page, "paragraphStyleLeading", 0.0, 5000.0, 1, tr(" pt"));
    leading_->setValue(character_.leading_value);
    auto* leadingRow = new QWidget(page);
    auto* leadingLayout = new QHBoxLayout(leadingRow);
    leadingLayout->setContentsMargins(0, 0, 0, 0);
    leadingLayout->addWidget(leadingMode_);
    leadingLayout->addWidget(leading_);
    form->addRow(tr("Leading:"), leadingRow);
    leading_->setEnabled(character_.leading_mode == 1);
    connect(leadingMode_, &QComboBox::currentIndexChanged, this, [this](int mode) {
        leading_->setEnabled(mode == 1);
    });

    kerningMode_ = new QComboBox(page);
    kerningMode_->setObjectName(QStringLiteral("paragraphStyleKerningMode"));
    kerningMode_->addItems({tr("Metrics"), tr("Manual")});
    kerningMode_->setCurrentIndex(character_.kerning_mode == 2 ? 1 : 0);
    kerning_ = intSpin(page, "paragraphStyleKerning", -1000, 10000, QString());
    kerning_->setValue(character_.kerning_value);
    auto* kerningRow = new QWidget(page);
    auto* kerningLayout = new QHBoxLayout(kerningRow);
    kerningLayout->setContentsMargins(0, 0, 0, 0);
    kerningLayout->addWidget(kerningMode_);
    kerningLayout->addWidget(kerning_);
    form->addRow(tr("Kerning:"), kerningRow);
    kerning_->setEnabled(character_.kerning_mode == 2);
    connect(kerningMode_, &QComboBox::currentIndexChanged, this, [this](int mode) {
        kerning_->setEnabled(mode == 1);
    });

    tracking_ = spin(page, "paragraphStyleTracking", -1000.0, 10000.0, 1, QString());
    tracking_->setValue(character_.tracking);
    form->addRow(tr("Tracking:"), tracking_);

    caseBox_ = new QComboBox(page);
    caseBox_->setObjectName(QStringLiteral("paragraphStyleCase"));
    caseBox_->addItem(tr("Normal"), 0);
    caseBox_->addItem(tr("All Caps"), 1);
    caseBox_->addItem(tr("Small Caps"), 2);
    caseBox_->setCurrentIndex(caseBox_->findData(character_.all_caps ? 1
                                                                     : (character_.small_caps ? 2 : 0)));
    form->addRow(tr("Case:"), caseBox_);

    positionBox_ = new QComboBox(page);
    positionBox_->setObjectName(QStringLiteral("paragraphStylePosition"));
    positionBox_->addItem(tr("Normal"), 0);
    positionBox_->addItem(tr("Superscript"), 1);
    positionBox_->addItem(tr("Subscript"), 2);
    positionBox_->setCurrentIndex(
        positionBox_->findData(character_.superscript ? 1 : (character_.subscript ? 2 : 0)));
    form->addRow(tr("Position:"), positionBox_);

    colorSwatch_ = new QToolButton(page);
    colorSwatch_->setObjectName(QStringLiteral("paragraphStyleColor"));
    colorSwatch_->setFixedSize(60, 22);
    refreshSwatch();
    connect(colorSwatch_, &QToolButton::clicked, this, [this] {
        const QColor picked = ColorPickerDialog::getColor(color_, this, tr("Text Color"));
        if (picked.isValid()) {
            color_ = picked;
            refreshSwatch();
            previewChangedIfChecked();
        }
    });
    form->addRow(tr("Color:"), colorSwatch_);

    row->addLayout(form, 1);

    auto* checks = new QVBoxLayout();
    checks->addStretch(1);
    strikethrough_ =
        check(page, "paragraphStyleStrikethrough", tr("Strikethrough"), character_.strikethrough);
    underline_ = check(page, "paragraphStyleUnderline", tr("Underline"), character_.underline);
    fauxBold_ = check(page, "paragraphStyleFauxBold", tr("Faux Bold"), character_.faux_bold);
    fauxItalic_ = check(page, "paragraphStyleFauxItalic", tr("Faux Italic"), character_.faux_italic);
    verticalRoman_ = check(page, "paragraphStyleVerticalRoman",
                           tr("Standard Vertical Roman Alignment"),
                           character_.vertical_roman_alignment);
    for (QCheckBox* box : {strikethrough_, underline_, fauxBold_, fauxItalic_, verticalRoman_}) {
        checks->addWidget(box);
    }
    checks->addStretch(1);
    row->addLayout(checks);

    for (QWidget* control : QList<QWidget*>{family_, fontStyle_, size_, leadingMode_, leading_,
                                            kerningMode_, kerning_, tracking_, caseBox_, positionBox_,
                                            strikethrough_, underline_, fauxBold_, fauxItalic_,
                                            verticalRoman_}) {
        watch(control);
    }
    return page;
}

QWidget* ParagraphStyleDialog::buildAdvancedCharacterPage()
{
    auto* page = new QWidget(this);
    auto* form = new QFormLayout(page);
    form->setLabelAlignment(Qt::AlignRight);

    hScale_ = spin(page, "paragraphStyleHScale", 1.0, 1000.0, 1, tr("%"));
    hScale_->setValue(character_.horizontal_scale);
    form->addRow(tr("Horizontal Scale:"), hScale_);

    vScale_ = spin(page, "paragraphStyleVScale", 1.0, 1000.0, 1, tr("%"));
    vScale_->setValue(character_.vertical_scale);
    form->addRow(tr("Vertical Scale:"), vScale_);

    baselineShift_ = spin(page, "paragraphStyleBaselineShift", -1000.0, 1000.0, 1, tr(" pt"));
    baselineShift_->setValue(character_.baseline_shift);
    form->addRow(tr("Baseline Shift:"), baselineShift_);

    tsume_ = spin(page, "paragraphStyleTsume", 0.0, 100.0, 0, tr("%"));
    tsume_->setValue(character_.tsume);
    form->addRow(tr("Tsume:"), tsume_);

    fractionalWidths_ = check(page, "paragraphStyleFractionalWidths", tr("Fractional Widths"),
                              character_.fractional_widths);
    form->addRow(QString(), fractionalWidths_);

    language_ = new QComboBox(page);
    language_->setObjectName(QStringLiteral("paragraphStyleLanguage"));
    language_->addItems({tr("English: USA"), tr("English: UK"), tr("French"), tr("German"),
                         tr("Italian"), tr("Spanish"), tr("Portuguese"), tr("Dutch"),
                         tr("Swedish"), tr("Norwegian"), tr("Danish"), tr("Finnish"),
                         tr("Polish"), tr("Czech"), tr("Hungarian"), tr("Russian"),
                         tr("Greek"), tr("Turkish"), tr("Japanese"), tr("Korean"),
                         tr("Chinese: Simplified"), tr("Chinese: Traditional")});
    if (language_->findText(character_.language) < 0 && !character_.language.isEmpty()) {
        language_->addItem(character_.language);
    }
    language_->setCurrentText(character_.language);
    form->addRow(tr("Language:"), language_);

    for (QWidget* control :
         QList<QWidget*>{hScale_, vScale_, baselineShift_, tsume_, fractionalWidths_, language_}) {
        watch(control);
    }
    return page;
}

QWidget* ParagraphStyleDialog::buildOpenTypePage()
{
    auto* page = new QWidget(this);
    auto* grid = new QGridLayout(page);
    grid->setHorizontalSpacing(24);

    standardLigatures_ = check(page, "paragraphStyleStandardLigatures", tr("Standard Ligatures"),
                               character_.standard_ligatures);
    contextualAlternates_ = check(page, "paragraphStyleContextualAlternates",
                                  tr("Contextual Alternates"), character_.contextual_alternates);
    discretionaryLigatures_ = check(page, "paragraphStyleDiscretionaryLigatures",
                                    tr("Discretionary Ligatures"),
                                    character_.discretionary_ligatures);
    swash_ = check(page, "paragraphStyleSwash", tr("Swash"), character_.swash);
    oldstyle_ = check(page, "paragraphStyleOldstyle", tr("Oldstyle"), character_.oldstyle);

    stylisticAlternates_ = check(page, "paragraphStyleStylisticAlternates",
                                 tr("Stylistic Alternates"), character_.stylistic_alternates);
    titlingAlternates_ = check(page, "paragraphStyleTitlingAlternates", tr("Titling Alternates"),
                               character_.titling_alternates);
    ornaments_ = check(page, "paragraphStyleOrnaments", tr("Ornaments"), character_.ornaments);
    ordinals_ = check(page, "paragraphStyleOrdinals", tr("Ordinals"), character_.ordinals);
    fractions_ = check(page, "paragraphStyleFractions", tr("Fractions"), character_.fractions);

    const QList<QCheckBox*> left = {standardLigatures_, contextualAlternates_,
                                    discretionaryLigatures_, swash_, oldstyle_};
    const QList<QCheckBox*> right = {stylisticAlternates_, titlingAlternates_, ornaments_,
                                     ordinals_, fractions_};
    for (int i = 0; i < left.size(); ++i) {
        grid->addWidget(left[i], i, 0, Qt::AlignLeft);
        grid->addWidget(right[i], i, 1, Qt::AlignLeft);
    }
    grid->setRowStretch(left.size(), 1);

    for (QCheckBox* box : left + right) {
        watch(box);
    }
    return page;
}

QWidget* ParagraphStyleDialog::buildIndentsPage()
{
    auto* page = new QWidget(this);
    auto* form = new QFormLayout(page);
    form->setLabelAlignment(Qt::AlignRight);

    alignment_ = new QComboBox(page);
    alignment_->setObjectName(QStringLiteral("paragraphStyleAlignment"));
    alignment_->addItem(tr("Left"), 0);
    alignment_->addItem(tr("Center"), 2);
    alignment_->addItem(tr("Right"), 1);
    alignment_->addItem(tr("Justify Last Left"), 3);
    alignment_->addItem(tr("Justify Last Right"), 4);
    alignment_->addItem(tr("Justify Last Center"), 5);
    alignment_->addItem(tr("Justify All"), 6);
    const int alignmentIndex = alignment_->findData(paragraph_.justify);
    alignment_->setCurrentIndex(alignmentIndex >= 0 ? alignmentIndex : 0);
    form->addRow(tr("Alignment:"), alignment_);

    startIndent_ = spin(page, "paragraphStyleStartIndent", -5000.0, 5000.0, 1, tr(" pt"));
    startIndent_->setValue(paragraph_.start_indent);
    form->addRow(tr("Left Indent:"), startIndent_);

    endIndent_ = spin(page, "paragraphStyleEndIndent", -5000.0, 5000.0, 1, tr(" pt"));
    endIndent_->setValue(paragraph_.end_indent);
    form->addRow(tr("Right Indent:"), endIndent_);

    firstLineIndent_ =
        spin(page, "paragraphStyleFirstLineIndent", -5000.0, 5000.0, 1, tr(" pt"));
    firstLineIndent_->setValue(paragraph_.first_line_indent);
    form->addRow(tr("First Line:"), firstLineIndent_);

    spaceBefore_ = spin(page, "paragraphStyleSpaceBefore", 0.0, 5000.0, 1, tr(" pt"));
    spaceBefore_->setValue(paragraph_.space_before);
    form->addRow(tr("Space Before:"), spaceBefore_);

    spaceAfter_ = spin(page, "paragraphStyleSpaceAfter", 0.0, 5000.0, 1, tr(" pt"));
    spaceAfter_->setValue(paragraph_.space_after);
    form->addRow(tr("Space After:"), spaceAfter_);

    for (QWidget* control : QList<QWidget*>{alignment_, startIndent_, endIndent_, firstLineIndent_,
                                            spaceBefore_, spaceAfter_}) {
        watch(control);
    }
    return page;
}

QWidget* ParagraphStyleDialog::buildCompositionPage()
{
    auto* page = new QWidget(this);
    auto* form = new QFormLayout(page);
    form->setLabelAlignment(Qt::AlignRight);

    composer_ = new QComboBox(page);
    composer_->setObjectName(QStringLiteral("paragraphStyleComposer"));
    composer_->addItem(tr("Adobe Single-line Composer"), 0);
    composer_->addItem(tr("Adobe Every-line Composer"), 1);
    composer_->setCurrentIndex(paragraph_.composer == 1 ? 1 : 0);
    form->addRow(tr("Composer:"), composer_);

    hanging_ = check(page, "paragraphStyleHanging", tr("Roman Hanging Punctuation"),
                     paragraph_.hanging);
    form->addRow(QString(), hanging_);

    watch(composer_);
    watch(hanging_);
    return page;
}

QWidget* ParagraphStyleDialog::buildJustificationPage()
{
    auto* page = new QWidget(this);
    auto* grid = new QGridLayout(page);
    grid->setHorizontalSpacing(12);

    const auto header = [page](const QString& text) {
        auto* label = new QLabel(text, page);
        label->setAlignment(Qt::AlignCenter);
        return label;
    };
    grid->addWidget(header(tr("Minimum")), 0, 1);
    grid->addWidget(header(tr("Desired")), 0, 2);
    grid->addWidget(header(tr("Maximum")), 0, 3);

    const auto row = [this, page, grid](int r, const QString& label, const char* minName,
                                        const char* desiredName, const char* maxName, double lo,
                                        double hi, const std::array<double, 3>& values) {
        grid->addWidget(new QLabel(label, page), r, 0);
        auto* min = spin(page, minName, lo, hi, 1, tr("%"));
        auto* desired = spin(page, desiredName, lo, hi, 1, tr("%"));
        auto* max = spin(page, maxName, lo, hi, 1, tr("%"));
        min->setValue(values[0]);
        desired->setValue(values[1]);
        max->setValue(values[2]);
        grid->addWidget(min, r, 1);
        grid->addWidget(desired, r, 2);
        grid->addWidget(max, r, 3);
        watch(min);
        watch(desired);
        watch(max);
        return std::array<QDoubleSpinBox*, 3>{min, desired, max};
    };

    const auto words = row(1, tr("Word Spacing:"), "paragraphStyleWordMin",
                           "paragraphStyleWordDesired", "paragraphStyleWordMax", 0.0, 1000.0,
                           {paragraph_.word_spacing_min, paragraph_.word_spacing_desired,
                            paragraph_.word_spacing_max});
    wordMin_ = words[0];
    wordDesired_ = words[1];
    wordMax_ = words[2];

    const auto letters = row(2, tr("Letter Spacing:"), "paragraphStyleLetterMin",
                             "paragraphStyleLetterDesired", "paragraphStyleLetterMax", -100.0,
                             500.0,
                             {paragraph_.letter_spacing_min, paragraph_.letter_spacing_desired,
                              paragraph_.letter_spacing_max});
    letterMin_ = letters[0];
    letterDesired_ = letters[1];
    letterMax_ = letters[2];

    const auto glyphs = row(3, tr("Glyph Scaling:"), "paragraphStyleGlyphMin",
                            "paragraphStyleGlyphDesired", "paragraphStyleGlyphMax", 50.0, 200.0,
                            {paragraph_.glyph_spacing_min, paragraph_.glyph_spacing_desired,
                             paragraph_.glyph_spacing_max});
    glyphMin_ = glyphs[0];
    glyphDesired_ = glyphs[1];
    glyphMax_ = glyphs[2];

    grid->addWidget(new QLabel(tr("Auto Leading:"), page), 4, 0);
    autoLeading_ = spin(page, "paragraphStyleAutoLeading", 0.0, 500.0, 1, tr("%"));
    autoLeading_->setValue(paragraph_.auto_leading);
    grid->addWidget(autoLeading_, 4, 1);
    watch(autoLeading_);
    grid->setRowStretch(5, 1);
    return page;
}

QWidget* ParagraphStyleDialog::buildHyphenationPage()
{
    auto* page = new QWidget(this);
    auto* group = new QGroupBox(tr("Hyphenation"), page);
    auto* form = new QFormLayout(group);
    form->setLabelAlignment(Qt::AlignRight);

    hyphenate_ = check(group, "paragraphStyleHyphenate", tr("Hyphenation"), paragraph_.hyphenate);
    form->addRow(QString(), hyphenate_);

    hyphenWordSize_ = intSpin(group, "paragraphStyleHyphenWordSize", 1, 100, tr(" letters"));
    hyphenWordSize_->setValue(paragraph_.hyphenate_word_size);
    form->addRow(tr("Words Longer Than:"), hyphenWordSize_);

    hyphenPre_ = intSpin(group, "paragraphStyleHyphenPre", 1, 100, tr(" letters"));
    hyphenPre_->setValue(paragraph_.hyphenate_pre);
    form->addRow(tr("After First:"), hyphenPre_);

    hyphenPost_ = intSpin(group, "paragraphStyleHyphenPost", 1, 100, tr(" letters"));
    hyphenPost_->setValue(paragraph_.hyphenate_post);
    form->addRow(tr("Before Last:"), hyphenPost_);

    hyphenLimit_ = intSpin(group, "paragraphStyleHyphenLimit", 1, 100, tr(" hyphens"));
    hyphenLimit_->setValue(paragraph_.hyphen_limit);
    form->addRow(tr("Hyphen Limit:"), hyphenLimit_);

    hyphenZone_ = spin(group, "paragraphStyleHyphenZone", 0.0, 1000.0, 1, tr(" pica"));
    hyphenZone_->setValue(paragraph_.hyphenation_zone / 12.0);
    form->addRow(tr("Hyphenation Zone:"), hyphenZone_);

    hyphenateCaps_ = check(group, "paragraphStyleHyphenateCaps",
                           tr("Hyphenate Capitalized Words"), paragraph_.hyphenate_caps);
    form->addRow(QString(), hyphenateCaps_);

    const QList<QWidget*> details = {hyphenWordSize_, hyphenPre_, hyphenPost_, hyphenLimit_,
                                     hyphenZone_, hyphenateCaps_};
    for (QWidget* widget : details) {
        widget->setEnabled(paragraph_.hyphenate);
    }
    connect(hyphenate_, &QCheckBox::toggled, this, [details](bool on) {
        for (QWidget* widget : details) {
            widget->setEnabled(on);
        }
    });

    auto* layout = new QVBoxLayout(page);
    layout->addWidget(group);
    layout->addStretch(1);

    watch(hyphenate_);
    for (QWidget* widget : details) {
        watch(widget);
    }
    return page;
}

} // namespace pictura
