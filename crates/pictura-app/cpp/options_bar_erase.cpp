// The options bars of the colour erasers: Background Eraser and Magic Eraser.
// Part of OptionsBar; split from options_bar.cpp along the page seam.

#include "options_bar.h"

#include "panels/numeric_field.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

namespace {

QComboBox* addErasePicker(QHBoxLayout* layout, QWidget* page, const QString& label,
                          const QString& name, const QStringList& items, int current)
{
    layout->addWidget(new QLabel(label, page));
    auto* combo = new QComboBox(page);
    combo->setObjectName(name);
    combo->addItems(items);
    combo->setCurrentIndex(current);
    layout->addWidget(combo);
    return combo;
}

QCheckBox* addEraseToggle(QHBoxLayout* layout, QWidget* page, const QString& label,
                          const QString& name, bool checked)
{
    auto* box = new QCheckBox(label, page);
    box->setObjectName(name);
    box->setChecked(checked);
    layout->addWidget(box);
    return box;
}

} // namespace

// CS6's Background Eraser bar: the brush tip, Sampling, Limits, Tolerance
// (50 %), and Protect Foreground Color.
// ponytail: no pen-pressure Size / Tolerance controls.
QWidget* OptionsBar::buildBackgroundEraserPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addBrushTipFields(layout, page);

    const BackgroundEraseOptions initial =
        controller_ ? backgroundErase_->get() : BackgroundEraseOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            BackgroundEraseOptions o = backgroundErase_->get();
            edit(o);
            backgroundErase_->set(o);
        }
    };
    auto* sampling = addErasePicker(
        layout, page, QStringLiteral("Sampling:"), QStringLiteral("optionsBackgroundEraseSampling"),
        {QStringLiteral("Continuous"), QStringLiteral("Once"), QStringLiteral("Background Swatch")},
        initial.sampling);
    connect(sampling, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](BackgroundEraseOptions& o) { o.sampling = i; }); });
    auto* limits = addErasePicker(
        layout, page, QStringLiteral("Limits:"), QStringLiteral("optionsBackgroundEraseLimits"),
        {QStringLiteral("Discontiguous"), QStringLiteral("Contiguous"), QStringLiteral("Find Edges")},
        initial.limits);
    connect(limits, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](BackgroundEraseOptions& o) { o.limits = i; }); });

    auto* tolerance = new NumericField(
        QStringLiteral("Tolerance:"),
        numericConfig(0, 100, 1, 0, QStringLiteral("%"), true,
                      QStringLiteral("optionsBackgroundEraseTolerance")),
        page);
    tolerance->setValue(initial.tolerance);
    layout->addWidget(tolerance);
    connect(tolerance, &NumericField::valueChanged, this, [update](double v) {
        update([v](BackgroundEraseOptions& o) { o.tolerance = qRound(v); });
    });

    auto* protect =
        addEraseToggle(layout, page, QStringLiteral("Protect Foreground Color"),
                       QStringLiteral("optionsBackgroundEraseProtect"), initial.protectForeground);
    connect(protect, &QCheckBox::toggled, this, [update](bool on) {
        update([on](BackgroundEraseOptions& o) { o.protectForeground = on; });
    });

    layout->addStretch(1);
    return page;
}

// CS6's Magic Eraser bar: Tolerance (32, the Magic Wand's scale), Anti-alias,
// Contiguous, Sample All Layers, and Opacity.
QWidget* OptionsBar::buildMagicEraserPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    const MagicEraseOptions initial =
        controller_ ? magicErase_->get() : MagicEraseOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            MagicEraseOptions o = magicErase_->get();
            edit(o);
            magicErase_->set(o);
        }
    };
    auto* tolerance = new NumericField(
        QStringLiteral("Tolerance:"),
        numericConfig(0, 255, 1, 0, QString(), true, QStringLiteral("optionsMagicEraseTolerance")),
        page);
    tolerance->setValue(initial.tolerance);
    layout->addWidget(tolerance);
    connect(tolerance, &NumericField::valueChanged, this, [update](double v) {
        update([v](MagicEraseOptions& o) { o.tolerance = qRound(v); });
    });

    auto* antialias = addEraseToggle(layout, page, QStringLiteral("Anti-alias"),
                                     QStringLiteral("optionsMagicEraseAntialias"),
                                     initial.antialias);
    connect(antialias, &QCheckBox::toggled, this,
            [update](bool on) { update([on](MagicEraseOptions& o) { o.antialias = on; }); });
    auto* contiguous = addEraseToggle(layout, page, QStringLiteral("Contiguous"),
                                      QStringLiteral("optionsMagicEraseContiguous"),
                                      initial.contiguous);
    connect(contiguous, &QCheckBox::toggled, this,
            [update](bool on) { update([on](MagicEraseOptions& o) { o.contiguous = on; }); });
    auto* sampleAll = addEraseToggle(layout, page, QStringLiteral("Sample All Layers"),
                                     QStringLiteral("optionsMagicEraseSampleAll"),
                                     initial.sampleAllLayers);
    connect(sampleAll, &QCheckBox::toggled, this, [update](bool on) {
        update([on](MagicEraseOptions& o) { o.sampleAllLayers = on; });
    });

    auto* opacity = new NumericField(
        QStringLiteral("Opacity:"),
        numericConfig(0, 100, 1, 0, QStringLiteral("%"), true,
                      QStringLiteral("optionsMagicEraseOpacity")),
        page);
    opacity->setValue(initial.opacity);
    layout->addWidget(opacity);
    connect(opacity, &NumericField::valueChanged, this, [update](double v) {
        update([v](MagicEraseOptions& o) { o.opacity = qRound(v); });
    });

    layout->addStretch(1);
    return page;
}

} // namespace pictura
