#include "acr_panels.h"
#include "acr_settings.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

AcrCalibrationPanel::AcrCalibrationPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addSection(column, tr("Process"));
    auto* processRow = new QHBoxLayout;
    processRow->addWidget(new QLabel(tr("Process"), this));
    auto* process = new QComboBox(this);
    process->setObjectName(QStringLiteral("acrProcess"));
    process->addItems({tr("2012 (Current)"), tr("2010"), tr("2003")});
    const int version = settings_->intValue(acr::ProcessVersion, 2012);
    process->setCurrentIndex(version == 2010 ? 1 : version == 2003 ? 2 : 0);
    connect(process, &QComboBox::currentIndexChanged, this, [this](int index) {
        settings_->setInt(acr::ProcessVersion, index == 1 ? 2010 : index == 2 ? 2003 : 2012);
        emit changed();
    });
    processRow->addWidget(process);
    processRow->addStretch(1);
    column->addLayout(processRow);

    addCombo(column, acr::CameraProfile, tr("Camera Profile"),
             {tr("Embedded"), tr("Pictura Standard"), tr("Camera Matching")}, tr("Embedded"));
    addSlider(column, acr::ShadowTint, tr("Shadows Tint"), -100, 100, 0, 0);

    addSection(column, tr("Primary"));
    addSlider(column, acr::RedHue, tr("Red Hue"), -100, 100, 0, 0);
    addSlider(column, acr::RedSat, tr("Red Saturation"), -100, 100, 0, 0);
    addSlider(column, acr::GreenHue, tr("Green Hue"), -100, 100, 0, 0);
    addSlider(column, acr::GreenSat, tr("Green Saturation"), -100, 100, 0, 0);
    addSlider(column, acr::BlueHue, tr("Blue Hue"), -100, 100, 0, 0);
    addSlider(column, acr::BlueSat, tr("Blue Saturation"), -100, 100, 0, 0);
    column->addStretch(1);
}

} // namespace pictura
