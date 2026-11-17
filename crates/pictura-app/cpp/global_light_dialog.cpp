#include "global_light_dialog.h"

#include "panels/angle_dial.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

namespace pictura {

GlobalLightDialog::GlobalLightDialog(int angle, int altitude, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("globalLightDialog"));
    setWindowTitle(tr("Global Light"));

    auto* layout = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    auto* angleRow = new QWidget(this);
    auto* angleLayout = new QHBoxLayout(angleRow);
    angleLayout->setContentsMargins(0, 0, 0, 0);
    auto* dial = new AngleDial(angleRow);
    dial->setObjectName(QStringLiteral("globalLight.angle.dial"));
    angle_ = new QSpinBox(angleRow);
    angle_->setObjectName(QStringLiteral("globalLight.angle"));
    angle_->setRange(0, 359);
    angle_->setSuffix(QStringLiteral("°"));
    angle_->setFixedWidth(80);
    const int startAngle = ((angle % 360) + 360) % 360;
    dial->setAngle(startAngle);
    angle_->setValue(startAngle);
    connect(dial, &AngleDial::angleChanged, this, [this](double degrees) {
        dirty_ = true;
        const int value = int(std::lround(degrees)) % 360;
        const QSignalBlocker block(angle_);
        angle_->setValue(value);
    });
    connect(angle_, &QSpinBox::valueChanged, this, [this, dial](int value) {
        dirty_ = true;
        const QSignalBlocker block(dial);
        dial->setAngle(value);
    });
    angleLayout->addWidget(dial);
    angleLayout->addWidget(angle_);
    angleLayout->addStretch();
    form->addRow(tr("Angle:"), angleRow);

    auto* altitudeRow = new QWidget(this);
    auto* altitudeLayout = new QHBoxLayout(altitudeRow);
    altitudeLayout->setContentsMargins(0, 0, 0, 0);
    auto* slider = new QSlider(Qt::Horizontal, altitudeRow);
    slider->setObjectName(QStringLiteral("globalLight.altitude.slider"));
    slider->setRange(0, 90);
    altitude_ = new QSpinBox(altitudeRow);
    altitude_->setObjectName(QStringLiteral("globalLight.altitude"));
    altitude_->setRange(0, 90);
    altitude_->setSuffix(QStringLiteral("°"));
    altitude_->setFixedWidth(80);
    const int startAltitude = std::clamp(altitude, 0, 90);
    slider->setValue(startAltitude);
    altitude_->setValue(startAltitude);
    connect(slider, &QSlider::valueChanged, this, [this](int value) {
        dirty_ = true;
        const QSignalBlocker block(altitude_);
        altitude_->setValue(value);
    });
    connect(altitude_, &QSpinBox::valueChanged, this, [this, slider](int value) {
        dirty_ = true;
        const QSignalBlocker block(slider);
        slider->setValue(value);
    });
    altitudeLayout->addWidget(slider, 1);
    altitudeLayout->addWidget(altitude_);
    form->addRow(tr("Altitude:"), altitudeRow);
    layout->addLayout(form);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    layout->addWidget(buttons);
}

int GlobalLightDialog::angle() const
{
    return angle_->value();
}

int GlobalLightDialog::altitude() const
{
    return altitude_->value();
}

} // namespace pictura

#include "moc_global_light_dialog.cpp"
