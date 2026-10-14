#include "acr_controls.h"
#include "acr_settings.h"

#include <QtGui/QFont>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QVBoxLayout>

#include <cmath>

namespace pictura {

AcrSlider::AcrSlider(const QString& label, double minimum, double maximum, double value,
                     int decimals, QWidget* parent)
    : QWidget(parent)
{
    for (int i = 0; i < decimals; ++i) {
        factor_ *= 10;
    }

    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);
    outer->setSpacing(2);

    auto* row = new QHBoxLayout;
    row->setContentsMargins(0, 0, 0, 0);
    row->addWidget(new QLabel(label, this));
    row->addStretch(1);
    spin_ = new QDoubleSpinBox(this);
    spin_->setDecimals(decimals);
    spin_->setRange(minimum, maximum);
    spin_->setSingleStep(decimals > 0 ? 0.05 : 1.0);
    spin_->setValue(value);
    row->addWidget(spin_);
    outer->addLayout(row);

    slider_ = new QSlider(Qt::Horizontal, this);
    slider_->setRange(static_cast<int>(std::lround(minimum * factor_)),
                      static_cast<int>(std::lround(maximum * factor_)));
    slider_->setValue(static_cast<int>(std::lround(value * factor_)));
    outer->addWidget(slider_);

    connect(slider_, &QSlider::valueChanged, this, &AcrSlider::handleSlider);
    connect(spin_, QOverload<double>::of(&QDoubleSpinBox::valueChanged), this,
            &AcrSlider::handleSpin);
}

double AcrSlider::value() const
{
    return spin_->value();
}

void AcrSlider::setValue(double value)
{
    updating_ = true;
    spin_->setValue(value);
    slider_->setValue(static_cast<int>(std::lround(value * factor_)));
    updating_ = false;
}

void AcrSlider::handleSlider(int raw)
{
    if (updating_) {
        return;
    }
    updating_ = true;
    spin_->setValue(double(raw) / factor_);
    updating_ = false;
    emit valueChanged();
}

void AcrSlider::handleSpin(double value)
{
    if (updating_) {
        return;
    }
    updating_ = true;
    slider_->setValue(static_cast<int>(std::lround(value * factor_)));
    updating_ = false;
    emit valueChanged();
}

AcrCombo::AcrCombo(const QString& label, const QStringList& items, const QString& value,
                   QWidget* parent)
    : QWidget(parent)
{
    auto* row = new QHBoxLayout(this);
    row->setContentsMargins(0, 0, 0, 0);
    row->addWidget(new QLabel(label, this));
    row->addStretch(1);
    combo_ = new QComboBox(this);
    combo_->addItems(items);
    setValue(value);
    row->addWidget(combo_);
    connect(combo_, &QComboBox::currentTextChanged, this, [this](const QString&) {
        emit valueChanged();
    });
}

QString AcrCombo::value() const
{
    return combo_->currentText();
}

void AcrCombo::setValue(const QString& value)
{
    int index = combo_->findText(value);
    if (index < 0) {
        // A stored text value may carry a trailing NUL (e.g. a profile name);
        // match it against the label without the terminator.
        index = combo_->findText(QString(value).remove(QChar('\0')));
    }
    if (index >= 0) {
        combo_->setCurrentIndex(index);
    }
}

AcrCheck::AcrCheck(const QString& label, bool checked, QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    box_ = new QCheckBox(label, this);
    box_->setChecked(checked);
    layout->addWidget(box_);
    connect(box_, &QCheckBox::toggled, this, &AcrCheck::toggled);
}

void AcrCheck::setChecked(bool checked)
{
    box_->setChecked(checked);
}

QLabel* acrSection(const QString& text, QWidget* parent)
{
    auto* label = new QLabel(text, parent);
    QFont font = label->font();
    font.setBold(true);
    label->setFont(font);
    return label;
}

AcrPanel::AcrPanel(AcrSettings* settings, QWidget* parent)
    : QWidget(parent), settings_(settings)
{
}

AcrSlider* AcrPanel::addSlider(QVBoxLayout* column, const QByteArray& key, const QString& label,
                               double minimum, double maximum, int decimals, double defaultValue)
{
    const QByteArray stored = settings_->raw(key);
    const double value = stored.isEmpty() ? defaultValue : stored.toDouble();
    auto* slider = new AcrSlider(label, minimum, maximum, value, decimals, this);
    slider->setObjectName(QString::fromLatin1(key));
    connect(slider, &AcrSlider::valueChanged, this, [this, key, slider, decimals]() {
        if (decimals == 0) {
            settings_->setInt(key, static_cast<int>(std::lround(slider->value())));
        } else {
            settings_->setDouble(key, slider->value());
        }
        emit changed();
    });
    column->addWidget(slider);
    return slider;
}

AcrCombo* AcrPanel::addCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                             const QStringList& items, const QString& defaultValue)
{
    const QString value = settings_->textValue(key, defaultValue);
    auto* combo = new AcrCombo(label, items, value, this);
    combo->setObjectName(QString::fromLatin1(key));
    connect(combo, &AcrCombo::valueChanged, this, [this, key, combo]() {
        settings_->setText(key, combo->value());
        emit changed();
    });
    column->addWidget(combo);
    return combo;
}

AcrCombo* AcrPanel::addEnumCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                                 const QList<AcrEnumItem>& items)
{
    QStringList labels;
    labels.reserve(items.size());
    for (const AcrEnumItem& item : items) {
        labels.append(item.label);
    }
    const QByteArray stored = settings_->raw(key);
    QString value;
    for (const AcrEnumItem& item : items) {
        if (item.code == stored) {
            value = item.label;
            break;
        }
    }
    // An unknown stored code leaves the combo on its first item and is preserved
    // until the user actually picks a different one.
    auto* combo = new AcrCombo(label, labels, value, this);
    combo->setObjectName(QString::fromLatin1(key));
    connect(combo, &AcrCombo::valueChanged, this, [this, key, combo, items]() {
        for (const AcrEnumItem& item : items) {
            if (item.label == combo->value()) {
                settings_->setRaw(key, item.code);
                break;
            }
        }
        emit changed();
    });
    column->addWidget(combo);
    return combo;
}

AcrCheck* AcrPanel::addCheck(QVBoxLayout* column, const QByteArray& key, const QString& label,
                             bool defaultValue)
{
    const bool checked = settings_->boolValue(key, defaultValue);
    auto* check = new AcrCheck(label, checked, this);
    check->setObjectName(QString::fromLatin1(key));
    connect(check, &AcrCheck::toggled, this, [this, key](bool on) {
        settings_->setBool(key, on);
        emit changed();
    });
    column->addWidget(check);
    return check;
}

void AcrPanel::addSection(QVBoxLayout* column, const QString& text)
{
    column->addWidget(acrSection(text, this));
}

} // namespace pictura
