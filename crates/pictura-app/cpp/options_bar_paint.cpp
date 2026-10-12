// The options bars of the tools ported from photorust's healing and paint
// families: Red Eye, Color Replacement, and Mixer Brush. Part of OptionsBar;
// split from options_bar.cpp along the page seam.

#include "options_bar.h"

#include "paint_tip.h"
#include "panels/brush_preset_picker.h"
#include "panels/numeric_field.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QIcon>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <array>

namespace pictura {

namespace {

// photorust's Wet / Load / Mix combinations; the spec leaves CS6's exact
// preset values open (docs/03-tools/mixer-brush.md, Open questions).
struct MixerPreset {
    const char* name;
    int wet;
    int load;
    int mix;
};

constexpr std::array<MixerPreset, 8> kMixerPresets = {{
    {"Dry", 0, 50, 0},
    {"Dry, Light Load", 0, 1, 0},
    {"Moist, Light Mix", 20, 50, 5},
    {"Moist, Heavy Mix", 20, 50, 60},
    {"Wet, Light Mix", 50, 50, 5},
    {"Wet, Heavy Mix", 50, 50, 60},
    {"Very Wet, Light Mix", 80, 80, 5},
    {"Very Wet, Heavy Mix", 80, 80, 60},
}};

// The load swatch: the brush's paint over a checkerboard, so a clean brush
// (transparent) reads differently from one loaded with white.
QIcon reservoirIcon(const QColor& paint)
{
    QPixmap swatch(20, 20);
    QPainter painter(&swatch);
    for (int y = 0; y < 20; y += 5) {
        for (int x = 0; x < 20; x += 5) {
            painter.fillRect(x, y, 5, 5, ((x + y) / 5) % 2 ? Qt::white : QColor(0xcc, 0xcc, 0xcc));
        }
    }
    painter.fillRect(swatch.rect(), paint);
    painter.setPen(QColor(0x1a, 0x1a, 0x1a));
    painter.drawRect(0, 0, 19, 19);
    return QIcon(swatch);
}

QComboBox* addCombo(QHBoxLayout* layout, QWidget* page, const QString& label,
                    const QString& name, std::initializer_list<const char*> items)
{
    layout->addWidget(new QLabel(label, page));
    auto* combo = new QComboBox(page);
    combo->setObjectName(name);
    for (const char* item : items) {
        combo->addItem(QString::fromLatin1(item));
    }
    layout->addWidget(combo);
    return combo;
}

} // namespace

// The brush Size and Hardness, shared with the paint tools through the same
// controller fields.
// CS6's brush tip button: the tip with its size beside it; a click opens the
// Brush Preset picker (Size, Hardness, and the preset grid).
void OptionsBar::addBrushTipFields(QHBoxLayout* layout, QWidget* page)
{
    auto* button = new QToolButton(page);
    button->setObjectName(QStringLiteral("optionsBrushTip"));
    button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    button->setIconSize(QSize(16, 16));
    button->setAutoRaise(true);
    button->setToolTip(QStringLiteral("Click to open the Brush Preset picker"));
    layout->addWidget(button);
    if (!controller_) {
        return;
    }
    const auto refresh = [this, button] {
        button->setText(QString::number(controller_->brushSize()));
        button->setIcon(QIcon(BrushPresetPicker::tipIcon(paintTip(*controller_), 16)));
    };
    refresh();
    connect(controller_, &ToolController::brushSizeChanged, button, refresh);
    connect(controller_, &ToolController::brushTipChanged, button, refresh);
    connect(button, &QToolButton::clicked, this,
            [this, button] { brushPicker()->popUpUnder(button); });
}

BrushPresetPicker* OptionsBar::brushPicker()
{
    if (!brushPicker_) {
        brushPicker_ = new BrushPresetPicker(controller_, this);
    }
    return brushPicker_;
}

// CS6's Red Eye bar: Pupil Size and Darken Amount (both 1-100 %, default 50).
QWidget* OptionsBar::buildRedEyePage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    auto addField = [&](const QString& label, const QString& name, int value,
                        void (ToolController::*setter)(int)) {
        auto* field = new NumericField(
            label, numericConfig(1, 100, 1, 0, QStringLiteral("%"), true, name), page);
        field->setValue(value);
        layout->addWidget(field);
        if (controller_) {
            connect(field, &NumericField::valueChanged, this,
                    [this, setter](double v) { (controller_->*setter)(qRound(v)); });
        }
    };
    addField(QStringLiteral("Pupil Size:"), QStringLiteral("optionsRedEyePupil"),
             controller_ ? controller_->redEyePupil() : 50, &ToolController::setRedEyePupil);
    addField(QStringLiteral("Darken Amount:"), QStringLiteral("optionsRedEyeDarken"),
             controller_ ? controller_->redEyeDarken() : 50, &ToolController::setRedEyeDarken);

    layout->addStretch(1);
    return page;
}

// CS6's Color Replacement bar: the brush tip, Mode, Sampling, Limits,
// Tolerance, and Anti-alias.
QWidget* OptionsBar::buildColorReplacementPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addBrushTipFields(layout, page);

    const ColorReplaceOptions initial = controller_ ? controller_->colorReplaceOptions()
                                                    : ColorReplaceOptions{};
    // Each control edits one field of the controller's options.
    const auto update = [this](auto edit) {
        if (controller_) {
            ColorReplaceOptions o = controller_->colorReplaceOptions();
            edit(o);
            controller_->setColorReplaceOptions(o);
        }
    };

    auto* mode = addCombo(layout, page, QStringLiteral("Mode:"),
                          QStringLiteral("optionsColorReplaceMode"),
                          {"Hue", "Saturation", "Color", "Luminosity"});
    mode->setCurrentIndex(initial.mode);
    connect(mode, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](ColorReplaceOptions& o) { o.mode = i; }); });

    auto* sampling = addCombo(layout, page, QStringLiteral("Sampling:"),
                              QStringLiteral("optionsColorReplaceSampling"),
                              {"Continuous", "Once", "Background Swatch"});
    sampling->setCurrentIndex(initial.sampling);
    connect(sampling, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](ColorReplaceOptions& o) { o.sampling = i; }); });

    auto* limits = addCombo(layout, page, QStringLiteral("Limits:"),
                            QStringLiteral("optionsColorReplaceLimits"),
                            {"Discontiguous", "Contiguous", "Find Edges"});
    limits->setCurrentIndex(initial.limits);
    connect(limits, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](ColorReplaceOptions& o) { o.limits = i; }); });

    auto* tolerance = new NumericField(
        QStringLiteral("Tolerance:"),
        numericConfig(0, 100, 1, 0, QStringLiteral("%"), true,
                      QStringLiteral("optionsColorReplaceTolerance")),
        page);
    tolerance->setValue(initial.tolerance);
    layout->addWidget(tolerance);
    connect(tolerance, &NumericField::valueChanged, this, [update](double v) {
        update([v](ColorReplaceOptions& o) { o.tolerance = qRound(v); });
    });

    auto* antialias = new QCheckBox(QStringLiteral("Anti-alias"), page);
    antialias->setObjectName(QStringLiteral("optionsColorReplaceAntialias"));
    antialias->setChecked(initial.antialias);
    layout->addWidget(antialias);
    connect(antialias, &QCheckBox::toggled, this,
            [update](bool on) { update([on](ColorReplaceOptions& o) { o.antialias = on; }); });

    layout->addStretch(1);
    return page;
}

// CS6's Mixer Brush bar: the brush tip, the Current Brush Load swatch (Load /
// Clean), the after-stroke Load and Clean toggles, the Wet/Load/Mix preset
// menu, Wet, Load, Mix, Flow, and Sample All Layers.
// ponytail: Sample All Layers and Load Solid Colors Only are not wired.
QWidget* OptionsBar::buildMixerBrushPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addBrushTipFields(layout, page);

    auto* load = new QToolButton(page);
    load->setObjectName(QStringLiteral("optionsMixerLoad"));
    load->setPopupMode(QToolButton::InstantPopup);
    load->setIconSize(QSize(20, 20));
    load->setToolTip(QStringLiteral("Current brush load"));
    auto* loadMenu = new QMenu(load);
    QAction* loadBrush = loadMenu->addAction(QStringLiteral("Load Brush"));
    QAction* cleanBrush = loadMenu->addAction(QStringLiteral("Clean Brush"));
    load->setMenu(loadMenu);
    layout->addWidget(load);

    const MixerOptions initial = controller_ ? controller_->mixerOptions() : MixerOptions{};
    const auto update = [this](auto edit) {
        if (controller_) {
            MixerOptions o = controller_->mixerOptions();
            edit(o);
            controller_->setMixerOptions(o);
        }
    };

    auto addToggle = [&](const QString& text, const QString& name, const QString& tip,
                         bool on) {
        auto* button = new QToolButton(page);
        button->setObjectName(name);
        button->setText(text);
        button->setToolTip(tip);
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setChecked(on);
        layout->addWidget(button);
        return button;
    };
    auto* autoLoad = addToggle(QStringLiteral("Load"), QStringLiteral("optionsMixerAutoLoad"),
                               QStringLiteral("Load the brush after each stroke"),
                               initial.loadAfterStroke);
    auto* autoClean = addToggle(QStringLiteral("Clean"), QStringLiteral("optionsMixerAutoClean"),
                                QStringLiteral("Clean the brush after each stroke"),
                                initial.cleanAfterStroke);
    connect(autoLoad, &QToolButton::toggled, this,
            [update](bool on) { update([on](MixerOptions& o) { o.loadAfterStroke = on; }); });
    connect(autoClean, &QToolButton::toggled, this,
            [update](bool on) { update([on](MixerOptions& o) { o.cleanAfterStroke = on; }); });

    auto* preset = new QComboBox(page);
    preset->setObjectName(QStringLiteral("optionsMixerPreset"));
    preset->setToolTip(QStringLiteral("Useful mixing brush combinations"));
    preset->addItem(QStringLiteral("Custom"));
    for (const MixerPreset& p : kMixerPresets) {
        preset->addItem(QString::fromLatin1(p.name));
    }
    layout->addWidget(preset);

    struct Field {
        const char* label;
        const char* name;
        int MixerOptions::*member;
    };
    const Field fields[] = {
        {"Wet:", "optionsMixerWet", &MixerOptions::wet},
        {"Load:", "optionsMixerLoadAmount", &MixerOptions::load},
        {"Mix:", "optionsMixerMix", &MixerOptions::mix},
        {"Flow:", "optionsMixerFlow", &MixerOptions::flow},
    };
    std::array<NumericField*, 4> spins{};
    for (size_t i = 0; i < spins.size(); ++i) {
        const Field& f = fields[i];
        spins[i] = new NumericField(QString::fromLatin1(f.label),
                                    numericConfig(0, 100, 1, 0, QStringLiteral("%"), true,
                                                  QString::fromLatin1(f.name)),
                                    page);
        spins[i]->setValue(initial.*(f.member));
        layout->addWidget(spins[i]);
    }

    auto* sampleAll = new QCheckBox(QStringLiteral("Sample All Layers"), page);
    sampleAll->setToolTip(QStringLiteral("Sample All Layers: not implemented yet"));
    sampleAll->setEnabled(false);
    layout->addWidget(sampleAll);

    // Load and Mix do nothing on a dry canvas: a dry brush neither runs out nor
    // mixes, so they grey out rather than look effective.
    const auto followWet = [spins](int wet) {
        spins[1]->setEnabled(wet > 0);
        spins[2]->setEnabled(wet > 0);
    };
    // The preset menu shows the preset matching Wet/Load/Mix, else Custom.
    const auto syncPreset = [preset](const MixerOptions& o) {
        int match = 0;
        for (size_t i = 0; i < kMixerPresets.size(); ++i) {
            const MixerPreset& p = kMixerPresets[i];
            if (p.wet == o.wet && p.load == o.load && p.mix == o.mix) {
                match = int(i) + 1;
                break;
            }
        }
        const QSignalBlocker block(preset);
        preset->setCurrentIndex(match);
    };
    followWet(initial.wet);
    syncPreset(initial);

    for (size_t i = 0; i < spins.size(); ++i) {
        int MixerOptions::*member = fields[i].member;
        connect(spins[i], &NumericField::valueChanged, this,
                [this, update, member, followWet, syncPreset](double v) {
                    update([member, v](MixerOptions& o) { o.*member = qRound(v); });
                    if (controller_) {
                        followWet(controller_->mixerOptions().wet);
                        syncPreset(controller_->mixerOptions());
                    }
                });
    }
    connect(preset, &QComboBox::activated, this, [this, update, spins, followWet](int index) {
        if (index < 1 || !controller_) {
            return;
        }
        const MixerPreset& p = kMixerPresets[size_t(index - 1)];
        update([&p](MixerOptions& o) {
            o.wet = p.wet;
            o.load = p.load;
            o.mix = p.mix;
        });
        spins[0]->setValue(p.wet);
        spins[1]->setValue(p.load);
        spins[2]->setValue(p.mix);
        followWet(p.wet);
    });

    if (controller_) {
        load->setIcon(reservoirIcon(controller_->mixerReservoir()));
        connect(controller_, &ToolController::mixerReservoirChanged, load,
                [load](const QColor& paint) { load->setIcon(reservoirIcon(paint)); });
        connect(loadBrush, &QAction::triggered, this,
                [this] { controller_->setMixerReservoir(controller_->foreground()); });
        connect(cleanBrush, &QAction::triggered, this,
                [this] { controller_->setMixerReservoir(QColor(Qt::transparent)); });
    }

    layout->addStretch(1);
    return page;
}

} // namespace pictura
