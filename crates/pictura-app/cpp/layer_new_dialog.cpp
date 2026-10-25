#include "layer_new_dialog.h"
#include "blend_modes.h"
#include "dialogs.h"

#include "panels/layers_panel_internal.h"
#include "panels/numeric_field.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLineEdit>

namespace pictura {

LayerNewDialog::LayerNewDialog(bool group, QWidget* parent)
    : QDialog(parent)
    , group_(group)
{
    setWindowTitle(group ? QStringLiteral("New Group") : QStringLiteral("New Layer"));

    nameEdit_ = new QLineEdit(this);

    colorCombo_ = new QComboBox(this);
    colorCombo_->addItem(QStringLiteral("None"), 0);
    colorCombo_->addItem(QStringLiteral("Red"), 1);
    colorCombo_->addItem(QStringLiteral("Orange"), 2);
    colorCombo_->addItem(QStringLiteral("Yellow"), 3);
    colorCombo_->addItem(QStringLiteral("Green"), 4);
    colorCombo_->addItem(QStringLiteral("Blue"), 5);
    colorCombo_->addItem(QStringLiteral("Violet"), 6);
    colorCombo_->addItem(QStringLiteral("Gray"), 7);

    modeCombo_ = new QComboBox(this);
    modeCombo_->setObjectName(QStringLiteral("layerNewMode"));
    for (const BlendEntry& entry : blendModes()) {
        modeCombo_->addItem(entry.name, entry.key);
    }

    NumericFieldConfig opacityConfig;
    opacityConfig.minimum = 0;
    opacityConfig.maximum = 255;
    opacityConfig.decimals = 0;
    opacityConfig.objectName = QStringLiteral("layerNewOpacity");
    opacitySpin_ = new NumericField(QString(), opacityConfig, this);
    opacitySpin_->setValue(255);

    if (!group_) {
        neutralCheck_ = new QCheckBox(QStringLiteral("Fill With Mode-Neutral Color"), this);
        clippingCheck_ =
            new QCheckBox(QStringLiteral("Use Previous Layer to Create Clipping Mask"), this);
    }

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &LayerNewDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &LayerNewDialog::reject);

    auto* form = new QFormLayout(this);
    form->addRow(QStringLiteral("Name:"), nameEdit_);
    form->addRow(QStringLiteral("Color:"), colorCombo_);
    form->addRow(QStringLiteral("Mode:"), modeCombo_);
    form->addRow(QStringLiteral("Opacity:"), opacitySpin_);
    if (neutralCheck_) {
        form->addRow(neutralCheck_);
    }
    if (clippingCheck_) {
        form->addRow(clippingCheck_);
    }
    form->addRow(buttons);

    connect(modeCombo_, &QComboBox::currentIndexChanged, this,
            [this](int) { syncNeutralForMode(); });
    syncNeutralForMode();
}

void LayerNewDialog::syncNeutralForMode()
{
    if (!neutralCheck_) {
        return;
    }
    // The engine's `neutral_color` decides which modes have a neutral fill.
    const int index = modeCombo_->currentIndex();
    const bool available = index >= 0 && index < blendModes().size()
                           && blendModes().at(index).hasNeutral;
    neutralCheck_->setEnabled(available);
    if (!available) {
        neutralCheck_->setChecked(false);
    }
}

LayerNewDialog::LayerNewDialog(const QString& defaultName, QWidget* parent)
    : QDialog(parent)
    , nameColorOnly_(true)
{
    setWindowTitle(QStringLiteral("Layer from Background"));
    nameEdit_ = new QLineEdit(defaultName, this);
    colorCombo_ = new QComboBox(this);
    colorCombo_->addItem(QStringLiteral("None"), 0);
    colorCombo_->addItem(QStringLiteral("Red"), 1);
    colorCombo_->addItem(QStringLiteral("Orange"), 2);
    colorCombo_->addItem(QStringLiteral("Yellow"), 3);
    colorCombo_->addItem(QStringLiteral("Green"), 4);
    colorCombo_->addItem(QStringLiteral("Blue"), 5);
    colorCombo_->addItem(QStringLiteral("Violet"), 6);
    colorCombo_->addItem(QStringLiteral("Gray"), 7);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &LayerNewDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &LayerNewDialog::reject);

    auto* form = new QFormLayout(this);
    form->addRow(QStringLiteral("Name:"), nameEdit_);
    form->addRow(QStringLiteral("Color:"), colorCombo_);
    form->addRow(buttons);
}

LayerNewSpec LayerNewDialog::spec() const
{
    LayerNewSpec result;
    result.name = nameEdit_->text();
    result.color = colorCombo_->currentData().toInt();
    result.blend = modeCombo_ ? modeCombo_->currentData().toString() : QStringLiteral("norm");
    result.opacity = opacitySpin_ ? qRound(opacitySpin_->value()) : 255;
    result.neutralFill = neutralCheck_ && neutralCheck_->isChecked();
    result.clipping = clippingCheck_ && clippingCheck_->isChecked();
    return result;
}

bool LayerNewDialog::getNameColor(QWidget* parent, const QString& defaultName, LayerNewSpec* out)
{
    LayerNewDialog dialog(defaultName, parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.spec();
    return true;
}

bool LayerNewDialog::get(bool group, QWidget* parent, LayerNewSpec* out)
{
    LayerNewDialog dialog(group, parent);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.spec();
    return true;
}

bool LayerNewDialog::neutralEnabledForTest() const
{
    return neutralCheck_ && neutralCheck_->isEnabled();
}

bool LayerNewDialog::clippingVisibleForTest() const
{
    return clippingCheck_ != nullptr;
}

void LayerNewDialog::setModeForTest(const QString& blendKey)
{
    const int index = modeCombo_->findData(blendKey);
    if (index >= 0) {
        modeCombo_->setCurrentIndex(index);
    }
}

} // namespace pictura

#include "moc_layer_new_dialog.cpp"
