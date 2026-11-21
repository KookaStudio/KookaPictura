// The options bars of the retouch and toning tools: Blur, Sharpen, Smudge,
// Dodge, Burn, and Sponge.
// Part of OptionsBar; split from options_bar.cpp along the page seam.

#include "options_bar.h"
#include <QtCore/QCoreApplication>

#include "panels/numeric_field.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

// CS6's Blur, Sharpen, and Smudge bars: the brush tip, Mode (the cut-down list
// a tool working on its own pixels offers), Strength (50 %), and Sample All
// Layers; Sharpen adds Protect Detail and Smudge Finger Painting. Object names
// carry the tool's label (`optionsSharpenMode`, …).
QWidget* OptionsBar::buildRetouchPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addBrushTipFields(layout, page);

    const QString tool = QString::fromLatin1(toolInfo(id).label);
    const auto name = [&tool](const char* field) {
        return QStringLiteral("options%1%2").arg(tool, QLatin1String(field));
    };
    auto* retouch = controller_ ? controller_->optionsFor<RetouchGroup>(id) : nullptr;
    const RetouchOptions initial = retouch ? retouch->get() : RetouchOptions{};
    const auto update = [retouch](auto edit) {
        if (retouch) {
            RetouchOptions o = retouch->get();
            edit(o);
            retouch->set(o);
        }
    };
    layout->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Mode:"), page));
    auto* mode = new QComboBox(page);
    mode->setObjectName(name("Mode"));
    mode->addItems({QCoreApplication::translate("PicturaUi", "Normal"), QCoreApplication::translate("PicturaUi", "Darken"), QCoreApplication::translate("PicturaUi", "Lighten"),
                    QCoreApplication::translate("PicturaUi", "Hue"), QCoreApplication::translate("PicturaUi", "Saturation"), QCoreApplication::translate("PicturaUi", "Color"),
                    QCoreApplication::translate("PicturaUi", "Luminosity")});
    mode->setCurrentIndex(initial.mode);
    layout->addWidget(mode);
    connect(mode, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](RetouchOptions& o) { o.mode = i; }); });

    auto* strength = new NumericField(
        QCoreApplication::translate("PicturaUi", "Strength:"),
        numericConfig(1, 100, 1, 0, QStringLiteral("%"), true, name("Strength")), page);
    strength->setValue(initial.strength);
    layout->addWidget(strength);
    connect(strength, &NumericField::valueChanged, this, [update](double v) {
        update([v](RetouchOptions& o) { o.strength = qRound(v); });
    });

    const auto addToggle = [&](const QString& label, const char* field, bool on) {
        auto* box = new QCheckBox(label, page);
        box->setObjectName(name(field));
        box->setChecked(on);
        layout->addWidget(box);
        return box;
    };
    auto* sampleAll = addToggle(QCoreApplication::translate("PicturaUi", "Sample All Layers"), "SampleAll",
                                initial.sampleAllLayers);
    connect(sampleAll, &QCheckBox::toggled, this,
            [update](bool on) { update([on](RetouchOptions& o) { o.sampleAllLayers = on; }); });
    if (id == ToolId::Sharpen) {
        auto* protect =
            addToggle(QCoreApplication::translate("PicturaUi", "Protect Detail"), "ProtectDetail", initial.protectDetail);
        connect(protect, &QCheckBox::toggled, this,
                [update](bool on) { update([on](RetouchOptions& o) { o.protectDetail = on; }); });
    } else if (id == ToolId::Smudge) {
        auto* finger =
            addToggle(QCoreApplication::translate("PicturaUi", "Finger Painting"), "FingerPainting", initial.fingerPainting);
        connect(finger, &QCheckBox::toggled, this,
                [update](bool on) { update([on](RetouchOptions& o) { o.fingerPainting = on; }); });
    }

    layout->addStretch(1);
    return page;
}

// CS6's Dodge and Burn bars: the brush tip, Range (Midtones), Exposure
// (50 %), and Protect Tones; the Sponge's: the tip, Mode (Desaturate), Flow
// (50 %), and Vibrance. Object names carry the tool's label.
// ponytail: no Airbrush toggle.
QWidget* OptionsBar::buildTonePage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addBrushTipFields(layout, page);

    const QString tool = QString::fromLatin1(toolInfo(id).label);
    const auto name = [&tool](const char* field) {
        return QStringLiteral("options%1%2").arg(tool, QLatin1String(field));
    };
    auto* tone = controller_ ? controller_->optionsFor<ToneGroup>(id) : nullptr;
    const ToneOptions initial = tone ? tone->get() : ToneOptions{};
    const auto update = [tone](auto edit) {
        if (tone) {
            ToneOptions o = tone->get();
            edit(o);
            tone->set(o);
        }
    };
    const bool sponge = id == ToolId::Sponge;
    auto* menu = new QComboBox(page);
    if (sponge) {
        layout->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Mode:"), page));
        menu->setObjectName(name("Mode"));
        menu->addItems({QCoreApplication::translate("PicturaUi", "Desaturate"), QCoreApplication::translate("PicturaUi", "Saturate")});
        menu->setCurrentIndex(initial.spongeMode);
        connect(menu, &QComboBox::currentIndexChanged, this,
                [update](int i) { update([i](ToneOptions& o) { o.spongeMode = i; }); });
    } else {
        layout->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Range:"), page));
        menu->setObjectName(name("Range"));
        menu->addItems(
            {QCoreApplication::translate("PicturaUi", "Shadows"), QCoreApplication::translate("PicturaUi", "Midtones"), QCoreApplication::translate("PicturaUi", "Highlights")});
        menu->setCurrentIndex(initial.range);
        connect(menu, &QComboBox::currentIndexChanged, this,
                [update](int i) { update([i](ToneOptions& o) { o.range = i; }); });
    }
    layout->addWidget(menu);

    auto* amount = new NumericField(
        sponge ? QCoreApplication::translate("PicturaUi", "Flow:") : QCoreApplication::translate("PicturaUi", "Exposure:"),
        numericConfig(1, 100, 1, 0, QStringLiteral("%"), true,
                      name(sponge ? "Flow" : "Exposure")),
        page);
    amount->setValue(initial.amount);
    layout->addWidget(amount);
    connect(amount, &NumericField::valueChanged, this, [update](double v) {
        update([v](ToneOptions& o) { o.amount = qRound(v); });
    });

    QToolButton* airbrush = nullptr;
    if (!sponge) {
        airbrush = addAirbrushToggle(layout, page, name("Airbrush"), initial.airbrush,
                                     [update](bool on) {
                                         update([on](ToneOptions& o) { o.airbrush = on; });
                                     });
    }

    auto* toggle = new QCheckBox(sponge ? QCoreApplication::translate("PicturaUi", "Vibrance") : QCoreApplication::translate("PicturaUi", "Protect Tones"),
                                 page);
    toggle->setObjectName(name(sponge ? "Vibrance" : "ProtectTones"));
    toggle->setChecked(sponge ? initial.vibrance : initial.protectTones);
    layout->addWidget(toggle);
    connect(toggle, &QCheckBox::toggled, this, [update, sponge](bool on) {
        update([on, sponge](ToneOptions& o) { (sponge ? o.vibrance : o.protectTones) = on; });
    });

    if (controller_) {
        connect(controller_, &ToolController::toneOptionsChanged, page,
                [tone, menu, amount, toggle, airbrush, sponge] {
                    if (!tone) {
                        return;
                    }
                    const ToneOptions o = tone->get();
                    const QSignalBlocker blockMenu(menu);
                    const QSignalBlocker blockAmount(amount);
                    const QSignalBlocker blockToggle(toggle);
                    menu->setCurrentIndex(sponge ? o.spongeMode : o.range);
                    amount->setValue(o.amount);
                    toggle->setChecked(sponge ? o.vibrance : o.protectTones);
                    if (airbrush) {
                        const QSignalBlocker blockAirbrush(airbrush);
                        airbrush->setChecked(o.airbrush);
                    }
                });
    }

    layout->addStretch(1);
    return page;
}

} // namespace pictura
