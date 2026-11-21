// The options bar of the Rotate View tool: Rotation Angle, the Set Angle of
// Rotation dial, and Reset View. Part of OptionsBar; split from
// options_bar.cpp along the page seam.

#include "options_bar.h"
#include <QtCore/QCoreApplication>

#include "panels/angle_dial.h"
#include "panels/numeric_field.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <cmath>

namespace pictura {

namespace {

// The dial measures counter-clockwise with 90° up and shows where the
// document's top points; the view rotation is clockwise.
double dialFromRotation(double rotation) { return 90.0 - rotation; }

double rotationFromDial(double dial) { return 90.0 - dial; }

} // namespace

QWidget* OptionsBar::buildRotateViewPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    auto* angle = new NumericField(
        QCoreApplication::translate("PicturaUi", "Rotation Angle:"),
        numericConfig(-180, 180, 1, 0, QStringLiteral("°"), true,
                      QStringLiteral("optionsRotateAngle")),
        page);
    layout->addWidget(angle);
    auto* dial = new AngleDial(page);
    dial->setObjectName(QStringLiteral("optionsRotateDial"));
    dial->setToolTip(QCoreApplication::translate("PicturaUi", "Set Angle of Rotation"));
    dial->setFixedSize(26, 26);
    layout->addWidget(dial);
    auto* reset = new QPushButton(QCoreApplication::translate("PicturaUi", "Reset View"), page);
    reset->setObjectName(QStringLiteral("optionsRotateReset"));
    layout->addWidget(reset);
    layout->addStretch(1);

    const auto show = [angle, dial](double rotation) {
        const QSignalBlocker blockAngle(angle);
        const QSignalBlocker blockDial(dial);
        angle->setValue(rotation);
        dial->setAngle(dialFromRotation(rotation));
    };
    if (controller_) {
        show(controller_->viewRotation());
        connect(controller_, &ToolController::viewRotationChanged, page, show);
        connect(angle, &NumericField::valueChanged, this,
                [this](double v) { controller_->setViewRotation(v); });
        connect(dial, &AngleDial::angleChanged, this,
                [this](double v) { controller_->setViewRotation(rotationFromDial(v)); });
        connect(reset, &QPushButton::clicked, this,
                [this]() { controller_->setViewRotation(0.0); });
    }
    return page;
}

} // namespace pictura
