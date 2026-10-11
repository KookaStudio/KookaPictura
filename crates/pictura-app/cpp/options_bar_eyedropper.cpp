// The Eyedropper tool's options bar: Sample Size, Sample scope, and Show
// Sampling Ring. Part of OptionsBar.

#include "options_bar.h"

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

    layout->addWidget(new QLabel(QStringLiteral("Sample Size:"), page));
    auto* size = new QComboBox(page);
    size->setObjectName(QStringLiteral("optionsEyedropperSize"));
    for (int n = 1; n <= 101; n += 2) {
        size->addItem(n == 1 ? QStringLiteral("Point Sample")
                             : QStringLiteral("%1 by %1 Average").arg(n),
                      n);
    }
    size->setCurrentIndex(controller_ ? (controller_->eyedropperOptions().sampleSize - 1) / 2 : 0);
    layout->addWidget(size);

    layout->addWidget(new QLabel(QStringLiteral("Sample:"), page));
    auto* sample = new QComboBox(page);
    sample->setObjectName(QStringLiteral("optionsEyedropperSample"));
    sample->addItem(QStringLiteral("Current Layer"), 0);
    sample->addItem(QStringLiteral("Current & Below"), 1);
    sample->addItem(QStringLiteral("All Layers"), 2);
    sample->addItem(QStringLiteral("All Layers No Adjustments"), 3);
    sample->addItem(QStringLiteral("Current & Below No Adjustments"), 4);
    sample->setCurrentIndex(controller_ ? controller_->eyedropperOptions().scope : 2);
    layout->addWidget(sample);

    auto* ring = new QCheckBox(QStringLiteral("Show Sampling Ring"), page);
    ring->setObjectName(QStringLiteral("optionsEyedropperRing"));
    ring->setChecked(controller_ && controller_->eyedropperOptions().ring);
    layout->addWidget(ring);

    if (controller_) {
        connect(size, &QComboBox::currentIndexChanged, this, [this, size](int) {
            EyedropperOptions options = controller_->eyedropperOptions();
            options.sampleSize = size->currentData().toInt();
            controller_->setEyedropperOptions(options);
        });
        connect(sample, &QComboBox::currentIndexChanged, this, [this, sample](int) {
            EyedropperOptions options = controller_->eyedropperOptions();
            options.scope = sample->currentData().toInt();
            controller_->setEyedropperOptions(options);
        });
        connect(ring, &QCheckBox::toggled, this, [this](bool on) {
            EyedropperOptions options = controller_->eyedropperOptions();
            options.ring = on;
            controller_->setEyedropperOptions(options);
        });
    }

    layout->addStretch(1);
    return page;
}

} // namespace pictura
