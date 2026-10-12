#include "character_panel.h"

#include "color_picker_dialog.h"
#include "font_combo.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "api/type_api.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QFont>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QIcon swatchIcon(const QColor& color)
{
    QPixmap pixmap(16, 16);
    pixmap.fill(color);
    QPainter painter(&pixmap);
    painter.setPen(QColor(0, 0, 0, 160));
    painter.drawRect(pixmap.rect().adjusted(0, 0, -1, -1));
    return QIcon(pixmap);
}

QDoubleSpinBox* spin(QWidget* parent, const QString& name, double lo, double hi, int decimals)
{
    auto* box = new QDoubleSpinBox(parent);
    box->setObjectName(name);
    box->setRange(lo, hi);
    box->setDecimals(decimals);
    box->setKeyboardTracking(false);
    return box;
}

} // namespace

CharacterPanel::CharacterPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(4, 4, 4, 4);

    auto* font = new QGroupBox(tr("Font"), this);
    auto* fontForm = new QFormLayout(font);
    family_ = new QFontComboBox(font);
    family_->setObjectName(QStringLiteral("characterFamily"));
    useFastFontPreviews(family_);
    fontForm->addRow(tr("Family"), family_);
    size_ = spin(font, QStringLiteral("characterSize"), 0.0, 1296.0, 1);
    fontForm->addRow(tr("Size"), size_);
    leadingMode_ = new QComboBox(font);
    leadingMode_->setObjectName(QStringLiteral("characterLeadingMode"));
    leadingMode_->addItems({tr("Auto"), tr("Fixed")});
    leading_ = spin(font, QStringLiteral("characterLeading"), 0.0, 5000.0, 1);
    auto* leadingRow = new QWidget(font);
    auto* leadingLayout = new QHBoxLayout(leadingRow);
    leadingLayout->setContentsMargins(0, 0, 0, 0);
    leadingLayout->addWidget(leadingMode_);
    leadingLayout->addWidget(leading_);
    fontForm->addRow(tr("Leading"), leadingRow);
    kerningMode_ = new QComboBox(font);
    kerningMode_->setObjectName(QStringLiteral("characterKerningMode"));
    kerningMode_->addItems({tr("Metrics"), tr("Manual")});
    kerning_ = new QSpinBox(font);
    kerning_->setObjectName(QStringLiteral("characterKerning"));
    kerning_->setRange(-1000, 10000);
    auto* kerningRow = new QWidget(font);
    auto* kerningLayout = new QHBoxLayout(kerningRow);
    kerningLayout->setContentsMargins(0, 0, 0, 0);
    kerningLayout->addWidget(kerningMode_);
    kerningLayout->addWidget(kerning_);
    fontForm->addRow(tr("Kerning"), kerningRow);
    tracking_ = spin(font, QStringLiteral("characterTracking"), -1000.0, 10000.0, 1);
    fontForm->addRow(tr("Tracking"), tracking_);
    horizontalScale_ = spin(font, QStringLiteral("characterHorizontalScale"), 1.0, 1000.0, 1);
    fontForm->addRow(tr("Horizontal Scale"), horizontalScale_);
    verticalScale_ = spin(font, QStringLiteral("characterVerticalScale"), 1.0, 1000.0, 1);
    fontForm->addRow(tr("Vertical Scale"), verticalScale_);
    baselineShift_ = spin(font, QStringLiteral("characterBaselineShift"), -1000.0, 1000.0, 1);
    fontForm->addRow(tr("Baseline Shift"), baselineShift_);
    antiAlias_ = new QComboBox(font);
    antiAlias_->setObjectName(QStringLiteral("characterAntiAlias"));
    antiAlias_->addItems({tr("None"), tr("Sharp"), tr("Crisp"), tr("Strong"), tr("Smooth")});
    fontForm->addRow(tr("Anti-aliasing"), antiAlias_);
    colorButton_ = new QToolButton(font);
    colorButton_->setObjectName(QStringLiteral("characterColor"));
    colorButton_->setAutoRaise(true);
    colorButton_->setIconSize(QSize(16, 16));
    fontForm->addRow(tr("Color"), colorButton_);
    layout->addWidget(font);

    auto* toggles = new QGroupBox(tr("OpenType"), this);
    auto* togglesLayout = new QVBoxLayout(toggles);
    const auto makeToggle = [this, toggles, togglesLayout](const QString& name, const QString& text) {
        auto* box = new QCheckBox(text, toggles);
        box->setObjectName(name);
        togglesLayout->addWidget(box);
        return box;
    };
    allCaps_ = makeToggle(QStringLiteral("characterAllCaps"), tr("All Caps"));
    smallCaps_ = makeToggle(QStringLiteral("characterSmallCaps"), tr("Small Caps"));
    superscript_ = makeToggle(QStringLiteral("characterSuperscript"), tr("Superscript"));
    subscript_ = makeToggle(QStringLiteral("characterSubscript"), tr("Subscript"));
    underline_ = makeToggle(QStringLiteral("characterUnderline"), tr("Underline"));
    strikethrough_ = makeToggle(QStringLiteral("characterStrikethrough"), tr("Strikethrough"));
    layout->addWidget(toggles);
    layout->addStretch(1);

    const auto commitField = [this](QDoubleSpinBox* box) {
        connect(box, &QAbstractSpinBox::editingFinished, this, &CharacterPanel::apply);
    };
    commitField(size_);
    commitField(leading_);
    commitField(tracking_);
    commitField(horizontalScale_);
    commitField(verticalScale_);
    commitField(baselineShift_);
    connect(kerning_, &QAbstractSpinBox::editingFinished, this, &CharacterPanel::apply);
    connect(antiAlias_, &QComboBox::currentIndexChanged, this, &CharacterPanel::apply);
    connect(family_, &QFontComboBox::currentFontChanged, this, &CharacterPanel::apply);
    connect(leadingMode_, &QComboBox::currentIndexChanged, this, [this](int mode) {
        leading_->setEnabled(hasTypeLayer() && mode == 1);
        apply();
    });
    connect(kerningMode_, &QComboBox::currentIndexChanged, this, [this](int mode) {
        kerning_->setEnabled(hasTypeLayer() && mode == 1);
        apply();
    });
    for (QCheckBox* box : {allCaps_, smallCaps_, superscript_, subscript_, underline_,
                           strikethrough_}) {
        connect(box, &QCheckBox::toggled, this, &CharacterPanel::apply);
    }
    connect(colorButton_, &QToolButton::clicked, this, [this]() {
        const QColor picked =
            ColorPickerDialog::getColor(color_, this, tr("Text Color"));
        if (picked.isValid()) {
            color_ = picked;
            updateColorIcon();
            apply();
        }
    });

    refresh();
}

bool CharacterPanel::hasTypeLayer() const
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const QString path = view_->active_layer_path();
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

void CharacterPanel::refresh()
{
    const bool type = hasTypeLayer();
    const QString path = type ? view_->active_layer_path() : QString();
    const CharacterSetting c = type ? pictura::type_api::layerCharacterSetting(*view_, path)
                                    : pictura::type_api::defaultCharacterSetting();
    const QString family = type ? familyForFontName(pictura::type_api::layerFont(*view_, path)) : QString();

    const QSignalBlocker blockFamily(family_);
    const QSignalBlocker blockSize(size_);
    const QSignalBlocker blockLeadingMode(leadingMode_);
    const QSignalBlocker blockLeading(leading_);
    const QSignalBlocker blockKerningMode(kerningMode_);
    const QSignalBlocker blockKerning(kerning_);
    const QSignalBlocker blockTracking(tracking_);
    const QSignalBlocker blockHorizontal(horizontalScale_);
    const QSignalBlocker blockVertical(verticalScale_);
    const QSignalBlocker blockBaseline(baselineShift_);
    const QSignalBlocker blockAntiAlias(antiAlias_);
    const QSignalBlocker blockAllCaps(allCaps_);
    const QSignalBlocker blockSmallCaps(smallCaps_);
    const QSignalBlocker blockSuper(superscript_);
    const QSignalBlocker blockSub(subscript_);
    const QSignalBlocker blockUnderline(underline_);
    const QSignalBlocker blockStrike(strikethrough_);

    if (!family.isEmpty()) {
        family_->setCurrentFont(QFont(family));
    }
    size_->setValue(c.size);
    leadingMode_->setCurrentIndex(c.leading_mode == 1 ? 1 : 0);
    leading_->setValue(c.leading_value);
    kerningMode_->setCurrentIndex(c.kerning_mode == 2 ? 1 : 0);
    kerning_->setValue(c.kerning_value);
    tracking_->setValue(c.tracking);
    horizontalScale_->setValue(c.horizontal_scale);
    verticalScale_->setValue(c.vertical_scale);
    baselineShift_->setValue(c.baseline_shift);
    antiAlias_->setCurrentIndex(c.anti_alias);
    color_ = QColor::fromRgba(c.color);
    updateColorIcon();
    allCaps_->setChecked(c.all_caps);
    smallCaps_->setChecked(c.small_caps);
    superscript_->setChecked(c.superscript);
    subscript_->setChecked(c.subscript);
    underline_->setChecked(c.underline);
    strikethrough_->setChecked(c.strikethrough);

    for (QWidget* widget : QList<QWidget*>{family_, size_, leadingMode_, tracking_,
                                           horizontalScale_, verticalScale_, baselineShift_,
                                           antiAlias_, colorButton_, allCaps_, smallCaps_,
                                           superscript_, subscript_, underline_, strikethrough_}) {
        widget->setEnabled(type);
    }
    leading_->setEnabled(type && c.leading_mode == 1);
    kerningMode_->setEnabled(type);
    kerning_->setEnabled(type && c.kerning_mode == 2);
}

void CharacterPanel::apply()
{
    if (!hasTypeLayer()) {
        return;
    }
    const QString path = view_->active_layer_path();
    const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*view_, path);
    CharacterSetting c = layer.character;
    c.size = size_->value();
    c.leading_mode = leadingMode_->currentIndex() == 1 ? 1 : 0;
    c.leading_value = leading_->value();
    c.kerning_mode = kerningMode_->currentIndex() == 1 ? 2 : 0;
    c.kerning_value = kerning_->value();
    c.tracking = tracking_->value();
    c.horizontal_scale = horizontalScale_->value();
    c.vertical_scale = verticalScale_->value();
    c.baseline_shift = baselineShift_->value();
    c.anti_alias = antiAlias_->currentIndex();
    c.color = color_.rgba();
    c.all_caps = allCaps_->isChecked();
    c.small_caps = smallCaps_->isChecked();
    c.superscript = superscript_->isChecked();
    c.subscript = subscript_->isChecked();
    c.underline = underline_->isChecked();
    c.strikethrough = strikethrough_->isChecked();

    const QString family = family_->currentFont().family();
    registerTypeFont(family);
    pictura::type_api::updateLayer(*view_, path, layer.text, family, layer.setting, c,
                                   layer.paragraph);
}

bool CharacterPanel::editingEnabledForTest() const { return size_->isEnabled(); }

void CharacterPanel::commitForTest() { apply(); }

} // namespace pictura
