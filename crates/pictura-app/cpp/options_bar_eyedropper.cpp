// The Eyedropper tool's options bar: Sample Size, Sample scope, and Show
// Sampling Ring. Part of OptionsBar.

#include "options_bar.h"
#include <QtCore/QCoreApplication>

#include "api/engine_bridge.h"
#include "api/paint_bridge.h"
#include "image_view.h"
#include "localization.h"
#include "tools.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <cmath>

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

void OptionsBar::refreshRulerReadout()
{
    if (!rulerReadout_) {
        return;
    }
    const PictureView* view = controller_ ? controller_->view() : nullptr;
    const auto values = view ? ruler_measurement(*view) : ::rust::Vec<double>();
    QStringList fields;
    const char* names[] = {"X", "Y", "W", "H", "A", "D1", "D2"};
    const auto locale = uiNumericLocale();
    ImageView* canvas = controller_ ? controller_->canvas() : nullptr;
    for (int i = 0; i < (values.size() == 7 ? 7 : 6); ++i) {
        QString value;
        if (values.size() >= 6) {
            double v = values[i];
            if (canvas && i == 0) v -= canvas->rulerOrigin().x();
            if (canvas && i == 1) v -= canvas->rulerOrigin().y();
            value = i == 4 ? locale.toString(v, 'f', 1) + QStringLiteral("°")
                           : canvas ? canvas->rulerLength(v, i != 1 && i != 3)
                                    : locale.toString(v, 'f', 1);
        }
        fields << QStringLiteral("%1: %2").arg(QLatin1String(names[i]), value);
    }
    if (view && edition_is_extended()) {
        const double length = calibrated_ruler_length(*view);
        if (std::isfinite(length)) {
            fields << tr("Length %1").arg(locale.toString(length, 'g', QLocale::FloatingPointShortest)
                                          + QLatin1Char(' ') + measurement_scale_units(*view));
        }
    }
    rulerReadout_->setText(fields.join(QStringLiteral("   ")));
    if (rulerStraighten_) rulerStraighten_->setEnabled(values.size() >= 6 && values[5] > 0.0);
}

// ponytail: no Sample Size for samplers or Author and Color for notes yet.
QWidget* OptionsBar::buildAnnotationPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    if (id == ToolId::Ruler) {
        rulerStraighten_ = new QToolButton(page);
        rulerStraighten_->setObjectName(QStringLiteral("optionsRulerStraighten"));
        rulerStraighten_->setText(tr("Straighten"));
        layout->addWidget(rulerStraighten_);
        connect(rulerStraighten_, &QToolButton::clicked, this, &OptionsBar::rulerStraightenRequested);
        rulerReadout_ = new QLabel(page);
        rulerReadout_->setObjectName(QStringLiteral("optionsRulerReadout"));
        rulerReadout_->setTextFormat(Qt::PlainText);
        rulerReadout_->setTextInteractionFlags(Qt::TextSelectableByMouse);
        layout->addWidget(rulerReadout_);
        refreshRulerReadout();
        if (controller_) {
            connect(controller_, &ToolController::rulerChanged, this, &OptionsBar::refreshRulerReadout);
        }
    }

    auto* clear = new QToolButton(page);
    clear->setObjectName(QStringLiteral("optionsAnnotationClear"));
    clear->setText(id == ToolId::Note ? QCoreApplication::translate("PicturaUi", "Clear All") : QCoreApplication::translate("PicturaUi", "Clear"));
    clear->setToolTip(id == ToolId::ColorSampler ? QCoreApplication::translate("PicturaUi", "Delete every color sampler")
                      : id == ToolId::Note       ? QCoreApplication::translate("PicturaUi", "Delete every note")
                                                 : QCoreApplication::translate("PicturaUi", "Remove the measuring line"));
    layout->addWidget(clear);
    if (controller_) {
        connect(clear, &QToolButton::clicked, this, [this]() { controller_->clearAnnotations(); });
    }
    layout->addStretch(1);
    return page;
}

} // namespace pictura
