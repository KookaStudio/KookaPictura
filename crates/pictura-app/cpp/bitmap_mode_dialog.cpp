#include "bitmap_mode_dialog.h"

#include "api/layers_bridge.h"
#include "preset_store.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

constexpr int HalftoneMethod = 3;
constexpr int CustomPatternMethod = 4;
constexpr int PatternNativeRole = Qt::UserRole + 1;
constexpr double CentimetersPerInch = 2.54;

void setFrequencyUnits(QDoubleSpinBox& frequency, bool perCm)
{
    frequency.setDecimals(perCm ? 2 : 3);
    frequency.setRange(perCm ? 0.4 : 1.0, perCm ? 400.0 : 999.999);
}

} // namespace

BitmapModeDialog::BitmapModeDialog(PictureView& view, const QJsonObject& initial, QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Bitmap"));
    auto* root = new QVBoxLayout(this);

    auto* methodForm = new QFormLayout;
    method_ = new QComboBox(this);
    method_->setObjectName(QStringLiteral("bitmapMethod"));
    method_->addItem(tr("50% Threshold"), 0);
    method_->addItem(tr("Pattern Dither"), 1);
    method_->addItem(tr("Diffusion Dither"), 2);
    method_->addItem(tr("Halftone Screen"), HalftoneMethod);
    method_->addItem(tr("Custom Pattern"), CustomPatternMethod);
    const int initialMethod = method_->findData(initial.value(QStringLiteral("method")).toInt());
    method_->setCurrentIndex(initialMethod >= 0 ? initialMethod : 0);
    methodForm->addRow(tr("Use:"), method_);
    root->addLayout(methodForm);

    auto* screenBox = new QGroupBox(tr("Halftone Screen"), this);
    screenBox->setObjectName(QStringLiteral("bitmapScreenOptions"));
    auto* screenForm = new QFormLayout(screenBox);
    auto* frequencyRow = new QHBoxLayout;
    frequency_ = new QDoubleSpinBox(screenBox);
    frequency_->setObjectName(QStringLiteral("bitmapFrequency"));
    unit_ = new QComboBox(screenBox);
    unit_->setObjectName(QStringLiteral("bitmapFrequencyUnit"));
    unit_->addItem(tr("lines/inch"));
    unit_->addItem(tr("lines/cm"));
    const bool perCm = initial.value(QStringLiteral("perCm")).toBool();
    unit_->setCurrentIndex(perCm ? 1 : 0);
    setFrequencyUnits(*frequency_, perCm);
    frequency_->setValue(initial.value(QStringLiteral("frequency"))
                             .toDouble(perCm ? 45.0 / CentimetersPerInch : 45.0));
    connect(unit_, &QComboBox::currentIndexChanged, this, [this](int index) {
        const bool perCm = index == 1;
        const double value = frequency_->value();
        setFrequencyUnits(*frequency_, perCm);
        frequency_->setValue(perCm ? value / CentimetersPerInch : value * CentimetersPerInch);
    });
    frequencyRow->addWidget(frequency_);
    frequencyRow->addWidget(unit_);
    screenForm->addRow(tr("Frequency:"), frequencyRow);

    angle_ = new QDoubleSpinBox(screenBox);
    angle_->setObjectName(QStringLiteral("bitmapAngle"));
    angle_->setRange(-180.0, 180.0);
    angle_->setSuffix(QStringLiteral("°"));
    angle_->setValue(initial.value(QStringLiteral("angle")).toDouble(45.0));
    screenForm->addRow(tr("Angle:"), angle_);

    shape_ = new QComboBox(screenBox);
    shape_->setObjectName(QStringLiteral("bitmapShape"));
    shape_->addItems({tr("Round"), tr("Diamond"), tr("Ellipse"), tr("Line"), tr("Square"), tr("Cross")});
    const int initialShape = initial.value(QStringLiteral("shape")).toInt();
    shape_->setCurrentIndex(initialShape >= 0 && initialShape < shape_->count() ? initialShape : 0);
    screenForm->addRow(tr("Shape:"), shape_);
    root->addWidget(screenBox);

    auto* patternBox = new QWidget(this);
    patternBox->setObjectName(QStringLiteral("bitmapPatternOptions"));
    auto* patternForm = new QFormLayout(patternBox);
    pattern_ = new QComboBox(patternBox);
    pattern_->setObjectName(QStringLiteral("bitmapPattern"));
    patternForm->addRow(tr("Pattern:"), pattern_);
    root->addWidget(patternBox);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    auto* ok = buttons->button(QDialogButtonBox::Ok);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);

    auto updateControls = [this, &view, initial, screenBox, patternBox, ok,
                           populated = false]() mutable {
        const bool custom = method() == CustomPatternMethod;
        if (custom && !populated) {
            populated = true;
            for (const auto& id : available_pattern_ids(view)) {
                pattern_->addItem(id, id);
            }
            for (const auto& preset : PresetStore::current().rows(PresetType::Patterns)) {
                const auto id = preset.values.value(QStringLiteral("patternId")).toString();
                if (!id.isEmpty() && !preset.native.isEmpty() && pattern_->findData(id) < 0) {
                    pattern_->addItem(preset.name, id);
                    pattern_->setItemData(pattern_->count() - 1, preset.native, PatternNativeRole);
                }
            }
            const auto id = initial.value(QStringLiteral("patternId")).toString();
            if (!id.isEmpty()) {
                const auto native = QByteArray::fromBase64(
                    initial.value(QStringLiteral("patternNative")).toString().toLatin1(),
                    QByteArray::AbortOnBase64DecodingErrors);
                int index = pattern_->findData(id);
                if (index < 0) {
                    pattern_->addItem(id, id);
                    index = pattern_->count() - 1;
                }
                if (!native.isEmpty()) {
                    pattern_->setItemData(index, native, PatternNativeRole);
                }
                pattern_->setCurrentIndex(index);
            }
        }
        screenBox->setVisible(method() == HalftoneMethod);
        patternBox->setVisible(custom);
        ok->setEnabled(!custom || pattern_->currentIndex() >= 0);
    };
    updateControls();
    connect(method_, &QComboBox::currentIndexChanged, this, updateControls);
    connect(pattern_, &QComboBox::currentIndexChanged, this, [this, ok](int index) {
        if (method() == CustomPatternMethod) {
            ok->setEnabled(index >= 0);
        }
    });
}

int BitmapModeDialog::method() const { return method_->currentData().toInt(); }

QJsonObject BitmapModeDialog::options() const
{
    QJsonObject values{{QStringLiteral("mode"), QStringLiteral("bitmap")},
                       {QStringLiteral("method"), method()}};
    if (method() == HalftoneMethod) {
        values.insert(QStringLiteral("frequency"), frequency_->value());
        values.insert(QStringLiteral("perCm"), unit_->currentIndex() == 1);
        values.insert(QStringLiteral("angle"), angle_->value());
        values.insert(QStringLiteral("shape"), shape_->currentIndex());
    } else if (method() == CustomPatternMethod) {
        values.insert(QStringLiteral("patternId"), pattern_->currentData().toString());
        const auto native = pattern_->currentData(PatternNativeRole).toByteArray();
        if (!native.isEmpty()) {
            values.insert(QStringLiteral("patternNative"), QString::fromLatin1(native.toBase64()));
        }
    }
    return values;
}

} // namespace pictura

#include "moc_bitmap_mode_dialog.cpp"
