#include "character_panel.h"

#include "character_panel_icons.h"
#include "color_picker_dialog.h"
#include "font_combo.h"
#include "numeric_field.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "api/type_api.h"
#include "tool_sessions.h"

#include <QtCore/QEvent>
#include <QtCore/QLocale>
#include <QtCore/QSignalBlocker>
#include <QtGui/QFont>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <cstdint>

namespace pictura {

namespace {

using character_icons::Field;

constexpr QSize kIconSize(24, 20);

QIcon swatchIcon(const QColor& color)
{
    QPixmap pixmap(16, 16);
    pixmap.fill(color);
    QPainter painter(&pixmap);
    painter.setPen(QColor(0, 0, 0, 160));
    painter.drawRect(pixmap.rect().adjusted(0, 0, -1, -1));
    return QIcon(pixmap);
}

// One pixel is one step; Shift coarsens, Ctrl refines (as NumericField).
double modifierScale(Qt::KeyboardModifiers modifiers)
{
    if (modifiers.testFlag(Qt::ShiftModifier)) {
        return 10.0;
    }
    if (modifiers.testFlag(Qt::ControlModifier)) {
        return 0.1;
    }
    return 1.0;
}

QString number(double value) { return QLocale::c().toString(value, 'g', 6); }

// A typed or picked number, unit suffix ignored; false when it is not one.
bool parseNumber(QString text, double& out)
{
    text = text.trimmed();
    for (const QString& unit : {QStringLiteral("px"), QStringLiteral("pt"), QStringLiteral("%")}) {
        if (text.endsWith(unit, Qt::CaseInsensitive)) {
            text.chop(unit.size());
        }
    }
    bool ok = false;
    out = QLocale::c().toDouble(text.trimmed(), &ok);
    return ok && std::isfinite(out);
}

QStringList sizePresets()
{
    QStringList out;
    for (const int px : {6, 7, 8, 9, 10, 11, 12, 14, 18, 24, 30, 36, 48, 60, 72}) {
        out << QStringLiteral("%1 px").arg(px);
    }
    return out;
}

QStringList spacingPresets()
{
    QStringList out;
    for (const int v : {-100, -75, -50, -25, -10, -5, 0, 5, 10, 25, 50, 75, 100, 200}) {
        out << QString::number(v);
    }
    return out;
}

// Copy what the panel edits from `from` onto `to`, keeping every attribute
// the panel does not show (language, oldstyle, ornaments, …) as `to` has it.
void copyPanelFields(const CharacterSetting& from, CharacterSetting& to)
{
    to.size = from.size;
    to.leading_mode = from.leading_mode;
    to.leading_value = from.leading_value;
    to.kerning_mode = from.kerning_mode;
    to.kerning_value = from.kerning_value;
    to.tracking = from.tracking;
    to.horizontal_scale = from.horizontal_scale;
    to.vertical_scale = from.vertical_scale;
    to.baseline_shift = from.baseline_shift;
    to.anti_alias = from.anti_alias;
    to.faux_bold = from.faux_bold;
    to.faux_italic = from.faux_italic;
    to.all_caps = from.all_caps;
    to.small_caps = from.small_caps;
    to.superscript = from.superscript;
    to.subscript = from.subscript;
    to.underline = from.underline;
    to.strikethrough = from.strikethrough;
    to.standard_ligatures = from.standard_ligatures;
    to.contextual_alternates = from.contextual_alternates;
    to.discretionary_ligatures = from.discretionary_ligatures;
    to.swash = from.swash;
    to.stylistic_alternates = from.stylistic_alternates;
    to.titling_alternates = from.titling_alternates;
    to.ordinals = from.ordinals;
    to.fractions = from.fractions;
}

// The style toggles in panel order.
bool* styleFlag(CharacterSetting& c, int index)
{
    bool* flags[] = {&c.faux_bold,   &c.faux_italic, &c.all_caps,  &c.small_caps,
                     &c.superscript, &c.subscript,   &c.underline, &c.strikethrough};
    return flags[index];
}

// The OpenType toggles in panel order (the bits of `openTypeFeatures`).
bool* openTypeFlag(CharacterSetting& c, int index)
{
    bool* flags[] = {&c.standard_ligatures,      &c.contextual_alternates,
                     &c.discretionary_ligatures, &c.swash,
                     &c.stylistic_alternates,    &c.titling_alternates,
                     &c.ordinals,                &c.fractions};
    return flags[index];
}

const char* const kStyleNames[] = {"FauxBold",  "FauxItalic", "AllCaps",   "SmallCaps",
                                   "Superscript", "Subscript", "Underline", "Strikethrough"};
const char* const kOpenTypeNames[] = {"StandardLigatures",   "ContextualAlternates",
                                      "DiscretionaryLigatures", "Swash",
                                      "StylisticAlternates", "TitlingAlternates",
                                      "Ordinals",            "Fractions"};

QToolButton* toggleButton(QWidget* parent, const QString& name, const QString& tip)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(name);
    button->setToolTip(tip);
    button->setCheckable(true);
    button->setAutoRaise(true);
    button->setIconSize(kIconSize);
    return button;
}

} // namespace

CharacterPanel::CharacterPanel(QWidget* parent)
    : QWidget(parent)
    , shown_(pictura::type_api::defaultCharacterSetting())
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(6);

    family_ = new QFontComboBox(this);
    family_->setObjectName(QStringLiteral("characterFamily"));
    family_->setToolTip(tr("Set the font family"));
    useFastFontPreviews(family_);
    layout->addWidget(family_);

    size_ = {nullptr, nullptr, 1.0, 1.0, 1296.0,
             [this](const QString& text) {
                 double v = 0.0;
                 if (!parseNumber(text, v) || v < 1.0 || v > 1296.0) {
                     return false;
                 }
                 shown_.size = v;
                 return true;
             },
             [this]() { return QStringLiteral("%1 px").arg(number(shown_.size)); },
             [this]() { return shown_.size; }, [this](double v) { shown_.size = v; }};
    leading_ = {nullptr, nullptr, 1.0, 0.0, 5000.0,
                [this](const QString& text) {
                    if (text.trimmed().remove(u'(').startsWith(QStringLiteral("auto"),
                                                                Qt::CaseInsensitive)) {
                        shown_.leading_mode = 0;
                        return true;
                    }
                    double v = 0.0;
                    if (!parseNumber(text, v) || v < 0.0 || v > 5000.0) {
                        return false;
                    }
                    shown_.leading_mode = 1;
                    shown_.leading_value = v;
                    return true;
                },
                [this]() {
                    return shown_.leading_mode == 1
                        ? QStringLiteral("%1 px").arg(number(shown_.leading_value))
                        : tr("(Auto)");
                },
                // Auto leading is CS6's 120% of the size (the renderer's rule).
                [this]() {
                    return shown_.leading_mode == 1 ? shown_.leading_value : shown_.size * 1.2;
                },
                [this](double v) {
                    shown_.leading_mode = 1;
                    shown_.leading_value = v;
                }};
    kerning_ = {nullptr, nullptr, 1.0, -1000.0, 10000.0,
                [this](const QString& text) {
                    const QString t = text.trimmed();
                    if (t.compare(tr("Metrics"), Qt::CaseInsensitive) == 0) {
                        shown_.kerning_mode = 0;
                        return true;
                    }
                    if (t.compare(tr("Optical"), Qt::CaseInsensitive) == 0) {
                        shown_.kerning_mode = 1;
                        return true;
                    }
                    double v = 0.0;
                    if (!parseNumber(t, v) || v < -1000.0 || v > 10000.0) {
                        return false;
                    }
                    shown_.kerning_mode = 2;
                    shown_.kerning_value = int(std::lround(v));
                    return true;
                },
                [this]() {
                    switch (shown_.kerning_mode) {
                    case 0:
                        return tr("Metrics");
                    case 1:
                        return tr("Optical");
                    default:
                        return QString::number(shown_.kerning_value);
                    }
                },
                [this]() { return shown_.kerning_mode == 2 ? double(shown_.kerning_value) : 0.0; },
                [this](double v) {
                    shown_.kerning_mode = 2;
                    shown_.kerning_value = int(std::lround(v));
                }};
    tracking_ = {nullptr, nullptr, 1.0, -1000.0, 10000.0,
                 [this](const QString& text) {
                     double v = 0.0;
                     if (!parseNumber(text, v) || v < -1000.0 || v > 10000.0) {
                         return false;
                     }
                     shown_.tracking = v;
                     return true;
                 },
                 [this]() { return number(shown_.tracking); },
                 [this]() { return shown_.tracking; },
                 [this](double v) { shown_.tracking = std::round(v); }};

    buildComboField(size_, QStringLiteral("characterSize"), tr("Set the font size"),
                    sizePresets());
    buildComboField(leading_, QStringLiteral("characterLeading"), tr("Set the leading"),
                    QStringList{tr("(Auto)")} + sizePresets());
    buildComboField(kerning_, QStringLiteral("characterKerning"),
                    tr("Set the kerning between two characters"),
                    QStringList{tr("Metrics"), tr("Optical")} + spacingPresets());
    buildComboField(tracking_, QStringLiteral("characterTracking"),
                    tr("Set the tracking for the selected characters"), spacingPresets());

    const auto numeric = [this](const QString& name, const QString& tip, double lo, double hi,
                                const QString& suffix) {
        NumericFieldConfig config;
        config.minimum = lo;
        config.maximum = hi;
        config.decimals = 1;
        config.suffix = suffix;
        config.namePrefix = name;
        config.objectName = name;
        // A blank label is replaced by the field's icon below.
        auto* field = new NumericField(QStringLiteral(" "), config, this);
        field->setToolTip(tip);
        return field;
    };
    verticalScale_ = numeric(QStringLiteral("characterVerticalScale"), tr("Vertically scale"),
                             1.0, 1000.0, QStringLiteral("%"));
    horizontalScale_ = numeric(QStringLiteral("characterHorizontalScale"),
                               tr("Horizontally scale"), 1.0, 1000.0, QStringLiteral("%"));
    baselineShift_ = numeric(QStringLiteral("characterBaselineShift"),
                             tr("Set the baseline shift"), -1000.0, 1000.0, tr(" px"));

    colorButton_ = new QToolButton(this);
    colorButton_->setObjectName(QStringLiteral("characterColor"));
    colorButton_->setToolTip(tr("Set the text color"));
    colorButton_->setAutoRaise(true);
    colorButton_->setIconSize(QSize(36, 16));

    auto* grid = new QGridLayout();
    grid->setHorizontalSpacing(4);
    grid->setVerticalSpacing(4);
    grid->addWidget(size_.icon, 0, 0);
    grid->addWidget(size_.combo, 0, 1);
    grid->addWidget(leading_.icon, 0, 2);
    grid->addWidget(leading_.combo, 0, 3);
    grid->addWidget(kerning_.icon, 1, 0);
    grid->addWidget(kerning_.combo, 1, 1);
    grid->addWidget(tracking_.icon, 1, 2);
    grid->addWidget(tracking_.combo, 1, 3);
    grid->addWidget(verticalScale_, 2, 0, 1, 2);
    grid->addWidget(horizontalScale_, 2, 2, 1, 2);
    grid->addWidget(baselineShift_, 3, 0, 1, 2);
    auto* colorRow = new QHBoxLayout();
    colorRow->addStretch(1);
    colorRow->addWidget(new QLabel(tr("Color:"), this));
    colorRow->addWidget(colorButton_);
    grid->addLayout(colorRow, 3, 2, 1, 2);
    grid->setColumnStretch(1, 1);
    grid->setColumnStretch(3, 1);
    layout->addLayout(grid);

    const QStringList styleTips = {tr("Faux Bold"), tr("Faux Italic"), tr("All Caps"),
                                   tr("Small Caps"), tr("Superscript"), tr("Subscript"),
                                   tr("Underline"), tr("Strikethrough")};
    const QStringList openTypeTips = {
        tr("Standard Ligatures"),   tr("Contextual Alternates"), tr("Discretionary Ligatures"),
        tr("Swash"),                tr("Stylistic Alternates"),  tr("Titling Alternates"),
        tr("Ordinals"),             tr("Fractions")};
    auto* styleRow = new QHBoxLayout();
    auto* openTypeRow = new QHBoxLayout();
    for (int i = 0; i < 8; ++i) {
        styles_[i] = toggleButton(
            this, QStringLiteral("character%1").arg(QLatin1String(kStyleNames[i])), styleTips[i]);
        styleRow->addWidget(styles_[i]);
        openType_[i] = toggleButton(
            this, QStringLiteral("character%1").arg(QLatin1String(kOpenTypeNames[i])),
            openTypeTips[i]);
        openTypeRow->addWidget(openType_[i]);
    }
    layout->addLayout(styleRow);
    layout->addLayout(openTypeRow);

    antiAliasIcon_ = new QLabel(this);
    antiAliasIcon_->setToolTip(tr("Set the anti-aliasing method"));
    antiAlias_ = new QComboBox(this);
    antiAlias_->setObjectName(QStringLiteral("characterAntiAlias"));
    antiAlias_->setToolTip(tr("Set the anti-aliasing method"));
    antiAlias_->addItems({tr("None"), tr("Sharp"), tr("Crisp"), tr("Strong"), tr("Smooth")});
    auto* antiAliasRow = new QHBoxLayout();
    antiAliasRow->addStretch(1);
    antiAliasRow->addWidget(antiAliasIcon_);
    antiAliasRow->addWidget(antiAlias_);
    layout->addLayout(antiAliasRow);
    layout->addStretch(1);

    updateIcons();

    connect(family_, &QFontComboBox::currentFontChanged, this, &CharacterPanel::apply);
    connect(antiAlias_, &QComboBox::currentIndexChanged, this, [this](int index) {
        shown_.anti_alias = index;
        apply();
    });
    const auto commitNumeric = [this](NumericField* field, double CharacterSetting::* member) {
        connect(field, &NumericField::valueCommitted, this, [this, member](double v) {
            shown_.*member = v;
            apply();
        });
    };
    commitNumeric(verticalScale_, &CharacterSetting::vertical_scale);
    commitNumeric(horizontalScale_, &CharacterSetting::horizontal_scale);
    commitNumeric(baselineShift_, &CharacterSetting::baseline_shift);
    for (int i = 0; i < 8; ++i) {
        connect(styles_[i], &QToolButton::toggled, this, [this, i](bool on) {
            *styleFlag(shown_, i) = on;
            // All Caps / Small Caps and Superscript / Subscript exclude each
            // other, as CS6's FontCaps and FontBaseline enums do.
            const int partner = i == 2 ? 3 : i == 3 ? 2 : i == 4 ? 5 : i == 5 ? 4 : -1;
            if (on && partner >= 0) {
                *styleFlag(shown_, partner) = false;
                const QSignalBlocker block(styles_[partner]);
                styles_[partner]->setChecked(false);
            }
            apply();
        });
        connect(openType_[i], &QToolButton::toggled, this, [this, i](bool on) {
            *openTypeFlag(shown_, i) = on;
            apply();
        });
    }
    connect(colorButton_, &QToolButton::clicked, this, [this]() {
        const QColor picked = ColorPickerDialog::getColor(color_, this, tr("Text Color"));
        if (picked.isValid()) {
            color_ = picked;
            updateColorIcon();
            apply();
        }
    });

    refresh();
}

void CharacterPanel::buildComboField(ComboField& field, const QString& name, const QString& tip,
                                     const QStringList& presets)
{
    field.icon = new QLabel(this);
    field.icon->setObjectName(name + QStringLiteral("Icon"));
    field.icon->setToolTip(tip);
    field.icon->setCursor(Qt::SizeHorCursor);
    field.icon->installEventFilter(this);
    field.combo = new QComboBox(this);
    field.combo->setObjectName(name);
    field.combo->setToolTip(tip);
    field.combo->setEditable(true);
    field.combo->setInsertPolicy(QComboBox::NoInsert);
    field.combo->setMaxVisibleItems(16);
    field.combo->setMinimumContentsLength(5);
    field.combo->addItems(presets);
    field.combo->lineEdit()->installEventFilter(this);
    // A pick or Enter (or leaving the field) commits, not each keystroke.
    connect(field.combo, &QComboBox::activated, this, [this, &field]() { commitCombo(field); });
    connect(field.combo->lineEdit(), &QLineEdit::editingFinished, this,
            [this, &field]() { commitCombo(field); });
}

void CharacterPanel::commitCombo(ComboField& field)
{
    const QString before = field.show();
    if (field.parse(field.combo->currentText()) && field.show() != before) {
        apply();
    }
    const QSignalBlocker block(field.combo);
    field.combo->setEditText(field.show());
}

CharacterPanel::ComboField* CharacterPanel::comboFor(QObject* watched)
{
    for (ComboField* field : {&size_, &leading_, &kerning_, &tracking_}) {
        if (watched == field->icon || watched == field->combo->lineEdit()) {
            return field;
        }
    }
    return nullptr;
}

bool CharacterPanel::eventFilter(QObject* watched, QEvent* event)
{
    ComboField* field = comboFor(watched);
    if (!field || !field->combo->isEnabled()) {
        return QWidget::eventFilter(watched, event);
    }
    const bool onIcon = watched == field->icon;
    switch (event->type()) {
    case QEvent::MouseButtonPress: {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton) {
            scrubbing_ = field;
            scrubMoved_ = false;
            scrubOrigin_ = mouse->globalPosition().toPoint();
            scrubStart_ = field->scrubFrom();
            // The value box keeps its press (caret, focus) until a drag starts.
            return onIcon;
        }
        break;
    }
    case QEvent::MouseMove: {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (scrubbing_ != field || !(mouse->buttons() & Qt::LeftButton)) {
            break;
        }
        const int delta = mouse->globalPosition().toPoint().x() - scrubOrigin_.x();
        if (scrubMoved_ || qAbs(delta) >= 3) {
            scrubMoved_ = true;
            const double step = field->step * modifierScale(mouse->modifiers());
            field->scrubTo(std::clamp(scrubStart_ + delta * step, field->minimum, field->maximum));
            const QSignalBlocker block(field->combo);
            field->combo->setEditText(field->show());
            return true;
        }
        break;
    }
    case QEvent::MouseButtonRelease: {
        const bool scrubbed = scrubbing_ == field && scrubMoved_;
        scrubbing_ = nullptr;
        scrubMoved_ = false;
        if (scrubbed) {
            apply();
            return true;
        }
        break;
    }
    default:
        break;
    }
    return QWidget::eventFilter(watched, event);
}

void CharacterPanel::changeEvent(QEvent* event)
{
    QWidget::changeEvent(event);
    if (event->type() == QEvent::PaletteChange || event->type() == QEvent::StyleChange) {
        updateIcons();
    }
}

void CharacterPanel::updateIcons()
{
    const QColor ink = palette().color(QPalette::WindowText);
    const auto pixmap = [&ink](Field which) {
        return character_icons::field(which, ink).pixmap(kIconSize);
    };
    size_.icon->setPixmap(pixmap(Field::Size));
    leading_.icon->setPixmap(pixmap(Field::Leading));
    kerning_.icon->setPixmap(pixmap(Field::Kerning));
    tracking_.icon->setPixmap(pixmap(Field::Tracking));
    antiAliasIcon_->setPixmap(pixmap(Field::AntiAlias));
    const auto label = [](NumericField* field) {
        return field->findChild<QLabel*>(field->objectName() + QStringLiteral("Label"));
    };
    label(verticalScale_)->setPixmap(pixmap(Field::VerticalScale));
    label(horizontalScale_)->setPixmap(pixmap(Field::HorizontalScale));
    label(baselineShift_)->setPixmap(pixmap(Field::BaselineShift));
    for (int i = 0; i < 8; ++i) {
        styles_[i]->setIcon(character_icons::style(i, ink));
        openType_[i]->setIcon(character_icons::openType(i, ink));
    }
}

bool CharacterPanel::hasTypeLayer() const
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const QString path = layer_selection_anchor(*view_);
    return !path.isEmpty() && pictura::type_api::isType(*view_, path);
}

void CharacterPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

QSize CharacterPanel::minimumSizeHint() const
{
    return QSize(0, QWidget::minimumSizeHint().height());
}

void CharacterPanel::updateColorIcon() { colorButton_->setIcon(swatchIcon(color_)); }

void CharacterPanel::syncCombos()
{
    for (ComboField* field : {&size_, &leading_, &kerning_, &tracking_}) {
        const QSignalBlocker block(field->combo);
        field->combo->setEditText(field->show());
    }
}

void CharacterPanel::setTextSession(std::function<TextSession*()> session)
{
    session_ = std::move(session);
}

void CharacterPanel::refresh()
{
    CharacterSetting live = pictura::type_api::defaultCharacterSetting();
    QString liveFamily;
    TextSession* session = session_ ? session_() : nullptr;
    const bool newText = session && session->newTextCharacter(live, liveFamily);
    const bool type = newText || hasTypeLayer();
    const QString path = type && !newText ? layer_selection_anchor(*view_) : QString();
    shown_ = newText ? live
             : type  ? pictura::type_api::layerCharacterSetting(*view_, path)
                     : pictura::type_api::defaultCharacterSetting();
    const QString family
        = newText ? liveFamily
        : type    ? familyForFontName(pictura::type_api::layerFont(*view_, path))
                  : QString();
    std::uint32_t features = 0;
    if (type) {
        // The query must see the host face, not the bundled fallback.
        registerTypeFont(family);
        features = pictura::type_api::openTypeFeatures(family);
    }

    if (!family.isEmpty()) {
        const QSignalBlocker block(family_);
        family_->setCurrentFont(QFont(family));
    }
    syncCombos();
    verticalScale_->setValue(shown_.vertical_scale);
    horizontalScale_->setValue(shown_.horizontal_scale);
    baselineShift_->setValue(shown_.baseline_shift);
    {
        const QSignalBlocker block(antiAlias_);
        antiAlias_->setCurrentIndex(shown_.anti_alias);
    }
    color_ = QColor::fromRgba(shown_.color);
    updateColorIcon();
    for (int i = 0; i < 8; ++i) {
        const QSignalBlocker blockStyle(styles_[i]);
        styles_[i]->setChecked(*styleFlag(shown_, i));
        styles_[i]->setEnabled(type);
        const QSignalBlocker blockOpenType(openType_[i]);
        openType_[i]->setChecked(*openTypeFlag(shown_, i));
        openType_[i]->setEnabled(type && (features >> i & 1u));
    }
    for (QWidget* widget : QList<QWidget*>{family_, size_.combo, leading_.combo, kerning_.combo,
                                           tracking_.combo, verticalScale_, horizontalScale_,
                                           baselineShift_, antiAlias_, colorButton_}) {
        widget->setEnabled(type);
    }
}

void CharacterPanel::apply()
{
    CharacterSetting live = pictura::type_api::defaultCharacterSetting();
    QString liveFamily;
    if (TextSession* session = session_ ? session_() : nullptr;
        session && session->newTextCharacter(live, liveFamily)) {
        copyPanelFields(shown_, live);
        live.color = color_.rgba();
        const QString family = family_->currentFont().family();
        registerTypeFont(family);
        session->setNewTextCharacter(live, family);
        return;
    }
    if (!hasTypeLayer()) {
        return;
    }
    const QString path = layer_selection_anchor(*view_);
    const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*view_, path);
    CharacterSetting c = layer.character;
    copyPanelFields(shown_, c);
    c.color = color_.rgba();

    const QString family = family_->currentFont().family();
    registerTypeFont(family);
    pictura::type_api::updateLayer(*view_, path, layer.text, family, layer.setting, c,
                                   layer.paragraph);
}

bool CharacterPanel::editingEnabledForTest() const { return size_.combo->isEnabled(); }

void CharacterPanel::commitForTest() { apply(); }

} // namespace pictura

#include "moc_character_panel.cpp"
