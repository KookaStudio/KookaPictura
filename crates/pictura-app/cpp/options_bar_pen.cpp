// The options bars of the Pen, Freeform Pen, and Path Selection tools.
// Part of OptionsBar; split from options_bar.cpp along the page seam.

#include "options_bar.h"
#include <QtCore/QCoreApplication>

#include "panels/numeric_field.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

// The Pen and Freeform Pen bars: Mode (Shape / Path / Pixels). The Pen adds
// Auto Add/Delete (on) and Rubber Band (off; CS6 keeps it in a pop-up); the
// Freeform Pen adds Curve Fit (2 px). The Path Selection bar: Show Bounding
// Box (off).
// ponytail: no path operations or the Freeform Pen's Magnetic option.
QWidget* OptionsBar::buildPenPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    const PenOptions initial = controller_ ? pen_->get() : PenOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            PenOptions o = pen_->get();
            edit(o);
            pen_->set(o);
        }
    };
    if (id != ToolId::PathSelection) {
        auto* mode = new QComboBox(page);
        mode->setObjectName(QStringLiteral("optionsPenMode"));
        mode->setToolTip(QCoreApplication::translate("PicturaUi", "Pick tool mode"));
        mode->addItems(
            {QCoreApplication::translate("PicturaUi", "Shape"), QCoreApplication::translate("PicturaUi", "Path"), QCoreApplication::translate("PicturaUi", "Pixels")});
        mode->setCurrentIndex(initial.mode);
        layout->addWidget(mode);
        connect(mode, &QComboBox::currentIndexChanged, this,
                [update](int i) { update([i](PenOptions& o) { o.mode = i; }); });
        if (controller_) {
            connect(controller_, &ToolController::penOptionsChanged, page, [this, mode]() {
                const QSignalBlocker block(mode);
                mode->setCurrentIndex(pen_->get().mode);
            });
        }
    }
    if (id == ToolId::PathSelection) {
        auto* box = new QCheckBox(QCoreApplication::translate("PicturaUi", "Show Bounding Box"), page);
        box->setObjectName(QStringLiteral("optionsPathShowBoundingBox"));
        box->setChecked(initial.showBoundingBox);
        layout->addWidget(box);
        connect(box, &QCheckBox::toggled, this,
                [update](bool on) { update([on](PenOptions& o) { o.showBoundingBox = on; }); });
    } else if (id == ToolId::Pen) {
        auto* autoAdd = new QCheckBox(QCoreApplication::translate("PicturaUi", "Auto Add/Delete"), page);
        autoAdd->setObjectName(QStringLiteral("optionsPenAutoAddDelete"));
        autoAdd->setChecked(initial.autoAddDelete);
        layout->addWidget(autoAdd);
        connect(autoAdd, &QCheckBox::toggled, this,
                [update](bool on) { update([on](PenOptions& o) { o.autoAddDelete = on; }); });
        auto* rubber = new QCheckBox(QCoreApplication::translate("PicturaUi", "Rubber Band"), page);
        rubber->setObjectName(QStringLiteral("optionsPenRubberBand"));
        rubber->setChecked(initial.rubberBand);
        layout->addWidget(rubber);
        connect(rubber, &QCheckBox::toggled, this,
                [update](bool on) { update([on](PenOptions& o) { o.rubberBand = on; }); });
    } else {
        auto* fit = new NumericField(
            QCoreApplication::translate("PicturaUi", "Curve Fit:"),
            numericConfig(0.5, 10, 0.5, 1, QStringLiteral(" px"), false,
                          QStringLiteral("optionsFreeformPenCurveFit")),
            page);
        fit->setValue(initial.curveFit);
        layout->addWidget(fit);
        connect(fit, &NumericField::valueChanged, this,
                [update](double v) { update([v](PenOptions& o) { o.curveFit = v; }); });
    }
    layout->addStretch(1);
    return page;
}

} // namespace pictura
