// The Eyedropper tool's options bar: Sample Size, Sample scope, and Show
// Sampling Ring. Part of OptionsBar.

#include "options_bar.h"
#include <QtCore/QCoreApplication>

#include "tools.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

QWidget* OptionsBar::buildEyedropperPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    layout->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Sample Size:"), page));
    auto* size = new QComboBox(page);
    size->setObjectName(QStringLiteral("optionsEyedropperSize"));
    for (int n = 1; n <= 101; n += 2) {
        size->addItem(n == 1 ? QCoreApplication::translate("PicturaUi", "Point Sample")
                             : QCoreApplication::translate("PicturaUi", "%1 by %1 Average").arg(n),
                      n);
    }
    size->setCurrentIndex(controller_ ? (eyedropper_->get().sampleSize - 1) / 2 : 0);
    layout->addWidget(size);

    layout->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Sample:"), page));
    auto* sample = new QComboBox(page);
    sample->setObjectName(QStringLiteral("optionsEyedropperSample"));
    sample->addItem(QCoreApplication::translate("PicturaUi", "Current Layer"), 0);
    sample->addItem(QCoreApplication::translate("PicturaUi", "Current & Below"), 1);
    sample->addItem(QCoreApplication::translate("PicturaUi", "All Layers"), 2);
    sample->addItem(QCoreApplication::translate("PicturaUi", "All Layers No Adjustments"), 3);
    sample->addItem(QCoreApplication::translate("PicturaUi", "Current & Below No Adjustments"), 4);
    sample->setCurrentIndex(controller_ ? eyedropper_->get().scope : 2);
    layout->addWidget(sample);

    auto* ring = new QCheckBox(QCoreApplication::translate("PicturaUi", "Show Sampling Ring"), page);
    ring->setObjectName(QStringLiteral("optionsEyedropperRing"));
    ring->setChecked(controller_ && eyedropper_->get().ring);
    layout->addWidget(ring);

    if (controller_) {
        connect(size, &QComboBox::currentIndexChanged, this, [this, size](int) {
            EyedropperOptions options = eyedropper_->get();
            options.sampleSize = size->currentData().toInt();
            eyedropper_->set(options);
        });
        connect(sample, &QComboBox::currentIndexChanged, this, [this, sample](int) {
            EyedropperOptions options = eyedropper_->get();
            options.scope = sample->currentData().toInt();
            eyedropper_->set(options);
        });
        connect(ring, &QCheckBox::toggled, this, [this](bool on) {
            EyedropperOptions options = eyedropper_->get();
            options.ring = on;
            eyedropper_->set(options);
        });
        // A canvas-context-menu change reaches the bar through the controller.
        connect(controller_, &ToolController::eyedropperOptionsChanged, this,
                [this, size, sample, ring]() {
                    const EyedropperOptions o = eyedropper_->get();
                    const QSignalBlocker blockSize(size);
                    size->setCurrentIndex((o.sampleSize - 1) / 2);
                    const QSignalBlocker blockSample(sample);
                    sample->setCurrentIndex(o.scope);
                    const QSignalBlocker blockRing(ring);
                    ring->setChecked(o.ring);
                });
    }

    layout->addStretch(1);
    return page;
}

} // namespace pictura
