// The Layer Style dialog's pages, one per list row. Each control binds to a
// `"<effect>.<field>"` key (see `layer_style.rs`); the option order of every
// combo is the engine's choice order.

#include "layer_style_dialog.h"

#include "api/layers_bridge.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

#include <utility>

namespace pictura {

namespace {

// A page of group boxes stacked from the top.
struct Page {
    QWidget* widget = new QWidget;
    QVBoxLayout* layout = new QVBoxLayout(widget);

    QFormLayout* group(const QString& title)
    {
        auto* box = new QGroupBox(title);
        layout->addWidget(box);
        return new QFormLayout(box);
    }

    QWidget* done()
    {
        layout->addStretch();
        return widget;
    }
};

QHBoxLayout* pair(QWidget* first, QWidget* second)
{
    auto* row = new QHBoxLayout;
    row->addWidget(first, 1);
    row->addWidget(second);
    return row;
}

QComboBox* combo(const QString& name, const QStringList& items = {})
{
    auto* box = new QComboBox;
    box->setObjectName(name);
    box->addItems(items);
    return box;
}

QCheckBox* check(const QString& name, const QString& label)
{
    auto* box = new QCheckBox(label);
    box->setObjectName(name);
    return box;
}

} // namespace

QWidget* LayerStyleDialog::buildBlendingOptionsPage()
{
    Page page;
    QFormLayout* general = page.group(tr("General Blending"));
    QComboBox* mode = combo(QStringLiteral("blending.mode"));
    bindBlendMode(mode, QStringLiteral("blending.mode"));
    general->addRow(tr("Blend Mode:"), mode);
    general->addRow(tr("Opacity:"),
                    sliderRow(QStringLiteral("blending.opacity"), 0, 100, QStringLiteral("%")));

    QFormLayout* advanced = page.group(tr("Advanced Blending"));
    advanced->addRow(tr("Fill Opacity:"), sliderRow(QStringLiteral("blending.fillOpacity"), 0,
                                                    100, QStringLiteral("%")));
    QComboBox* knockout =
        combo(QStringLiteral("blending.knockout"), {tr("None"), tr("Shallow"), tr("Deep")});
    bindChoice(knockout, QStringLiteral("blending.knockout"));
    advanced->addRow(tr("Knockout:"), knockout);
    QCheckBox* interior =
        check(QStringLiteral("blending.blendInterior"), tr("Blend Interior Effects as Group"));
    bindCheck(interior, QStringLiteral("blending.blendInterior"));
    advanced->addRow(QString(), interior);
    QCheckBox* clipped =
        check(QStringLiteral("blending.blendClipped"), tr("Blend Clipped Layers as Group"));
    bindCheck(clipped, QStringLiteral("blending.blendClipped"));
    advanced->addRow(QString(), clipped);
    return page.done();
}

QWidget* LayerStyleDialog::buildBevelPage()
{
    Page page;
    QFormLayout* structure = page.group(tr("Structure"));
    QComboBox* style = combo(QStringLiteral("bevel.style"),
                             {tr("Outer Bevel"), tr("Inner Bevel"), tr("Emboss"),
                              tr("Pillow Emboss"), tr("Stroke Emboss")});
    bindChoice(style, QStringLiteral("bevel.style"));
    structure->addRow(tr("Style:"), style);
    QComboBox* technique = combo(QStringLiteral("bevel.technique"),
                                 {tr("Smooth"), tr("Chisel Hard"), tr("Chisel Soft")});
    bindChoice(technique, QStringLiteral("bevel.technique"));
    structure->addRow(tr("Technique:"), technique);
    structure->addRow(tr("Depth:"),
                      sliderRow(QStringLiteral("bevel.depth"), 1, 1000, QStringLiteral("%")));

    // Direction is a radio pair in CS6, the control people flip back and forth.
    auto* direction = new QWidget;
    auto* directionRow = new QHBoxLayout(direction);
    directionRow->setContentsMargins(0, 0, 0, 0);
    auto* up = new QRadioButton(tr("Up"));
    up->setObjectName(QStringLiteral("bevel.direction.up"));
    auto* down = new QRadioButton(tr("Down"));
    (value(QStringLiteral("bevel.direction")) < 0.5 ? up : down)->setChecked(true);
    directionRow->addWidget(up);
    directionRow->addWidget(down);
    directionRow->addStretch();
    connect(up, &QRadioButton::toggled, this,
            [this](bool on) { setValue(QStringLiteral("bevel.direction"), on ? 0.0 : 1.0); });
    structure->addRow(tr("Direction:"), direction);
    structure->addRow(tr("Size:"), sliderRow(QStringLiteral("bevel.size"), 0, 250, tr(" px")));
    structure->addRow(tr("Soften:"), sliderRow(QStringLiteral("bevel.soften"), 0, 16, tr(" px")));

    QFormLayout* shading = page.group(tr("Shading"));
    shading->addRow(tr("Angle:"), angleRow(QStringLiteral("bevel.angle")));
    QCheckBox* global = check(QStringLiteral("bevel.useGlobalLight"), tr("Use Global Light"));
    bindCheck(global, QStringLiteral("bevel.useGlobalLight"));
    shading->addRow(QString(), global);
    shading->addRow(tr("Altitude:"),
                    sliderRow(QStringLiteral("bevel.altitude"), 0, 90, QStringLiteral("°")));
    for (const auto& [label, prefix] :
         {std::pair{tr("Highlight Mode:"), QStringLiteral("bevel.highlight")},
          std::pair{tr("Shadow Mode:"), QStringLiteral("bevel.shadow")}}) {
        QComboBox* mode = combo(prefix + QStringLiteral("Mode"));
        bindBlendMode(mode, prefix + QStringLiteral("Mode"));
        shading->addRow(label, pair(mode, colorButton(prefix + QStringLiteral("Color"))));
        shading->addRow(tr("Opacity:"),
                        sliderRow(prefix + QStringLiteral("Opacity"), 0, 100, QStringLiteral("%")));
    }
    return page.done();
}

QWidget* LayerStyleDialog::buildStrokePage()
{
    Page page;
    QFormLayout* structure = page.group(tr("Structure"));
    structure->addRow(tr("Size:"), sliderRow(QStringLiteral("stroke.size"), 1, 250, tr(" px")));
    QComboBox* position = combo(QStringLiteral("stroke.position"),
                                {tr("Outside"), tr("Inside"), tr("Center")});
    bindChoice(position, QStringLiteral("stroke.position"));
    structure->addRow(tr("Position:"), position);
    QComboBox* mode = combo(QStringLiteral("stroke.mode"));
    bindBlendMode(mode, QStringLiteral("stroke.mode"));
    structure->addRow(tr("Blend Mode:"), mode);
    structure->addRow(tr("Opacity:"),
                      sliderRow(QStringLiteral("stroke.opacity"), 0, 100, QStringLiteral("%")));
    QFormLayout* fill = page.group(tr("Fill Type: Color"));
    fill->addRow(tr("Color:"), colorButton(QStringLiteral("stroke.color")));
    return page.done();
}

QWidget* LayerStyleDialog::buildShadowPage(const QString& key, bool inner)
{
    const auto k = [&key](const char* field) { return key + QLatin1Char('.') + QLatin1String(field); };
    Page page;
    QFormLayout* structure = page.group(tr("Structure"));
    QComboBox* mode = combo(k("mode"));
    bindBlendMode(mode, k("mode"));
    structure->addRow(tr("Blend Mode:"), pair(mode, colorButton(k("color"))));
    structure->addRow(tr("Opacity:"), sliderRow(k("opacity"), 0, 100, QStringLiteral("%")));
    structure->addRow(tr("Angle:"), angleRow(k("angle")));
    QCheckBox* global = check(k("useGlobalLight"), tr("Use Global Light"));
    bindCheck(global, k("useGlobalLight"));
    structure->addRow(QString(), global);
    structure->addRow(tr("Distance:"), distanceRow(k("distance")));
    // CS6 calls the same slider Choke on an inner shadow: it eats into the
    // shape rather than growing out of it.
    structure->addRow(inner ? tr("Choke:") : tr("Spread:"),
                      sliderRow(k(inner ? "choke" : "spread"), 0, 100, QStringLiteral("%")));
    structure->addRow(tr("Size:"), sliderRow(k("size"), 0, 250, tr(" px")));

    QFormLayout* quality = page.group(tr("Quality"));
    QCheckBox* antiAlias = check(k("antiAlias"), tr("Anti-aliased"));
    bindCheck(antiAlias, k("antiAlias"));
    quality->addRow(QString(), antiAlias);
    quality->addRow(tr("Noise:"), sliderRow(k("noise"), 0, 100, QStringLiteral("%")));
    if (!inner) {
        QCheckBox* knocks = check(k("knocksOut"), tr("Layer Knocks Out Drop Shadow"));
        bindCheck(knocks, k("knocksOut"));
        quality->addRow(QString(), knocks);
    }
    return page.done();
}

QWidget* LayerStyleDialog::buildGlowPage(const QString& key, bool inner)
{
    const auto k = [&key](const char* field) { return key + QLatin1Char('.') + QLatin1String(field); };
    Page page;
    QFormLayout* structure = page.group(tr("Structure"));
    QComboBox* mode = combo(k("mode"));
    bindBlendMode(mode, k("mode"));
    structure->addRow(tr("Blend Mode:"), mode);
    structure->addRow(tr("Opacity:"), sliderRow(k("opacity"), 0, 100, QStringLiteral("%")));
    structure->addRow(tr("Noise:"), sliderRow(k("noise"), 0, 100, QStringLiteral("%")));
    structure->addRow(tr("Color:"), colorButton(k("color")));

    QFormLayout* elements = page.group(tr("Elements"));
    QComboBox* technique = combo(k("technique"), {tr("Softer"), tr("Precise")});
    bindChoice(technique, k("technique"));
    elements->addRow(tr("Technique:"), technique);
    if (inner) {
        QComboBox* source = combo(k("source"), {tr("Center"), tr("Edge")});
        bindChoice(source, k("source"));
        elements->addRow(tr("Source:"), source);
    }
    elements->addRow(inner ? tr("Choke:") : tr("Spread:"),
                     sliderRow(k(inner ? "choke" : "spread"), 0, 100, QStringLiteral("%")));
    elements->addRow(tr("Size:"), sliderRow(k("size"), 0, 250, tr(" px")));

    QFormLayout* quality = page.group(tr("Quality"));
    quality->addRow(tr("Range:"), sliderRow(k("range"), 1, 100, QStringLiteral("%")));
    quality->addRow(tr("Jitter:"), sliderRow(k("jitter"), 0, 100, QStringLiteral("%")));
    return page.done();
}

QWidget* LayerStyleDialog::buildSatinPage()
{
    Page page;
    QFormLayout* structure = page.group(tr("Structure"));
    QComboBox* mode = combo(QStringLiteral("satin.mode"));
    bindBlendMode(mode, QStringLiteral("satin.mode"));
    structure->addRow(tr("Blend Mode:"), pair(mode, colorButton(QStringLiteral("satin.color"))));
    structure->addRow(tr("Opacity:"),
                      sliderRow(QStringLiteral("satin.opacity"), 0, 100, QStringLiteral("%")));
    structure->addRow(tr("Angle:"), angleRow(QStringLiteral("satin.angle")));
    structure->addRow(tr("Distance:"),
                      sliderRow(QStringLiteral("satin.distance"), 1, 250, tr(" px")));
    structure->addRow(tr("Size:"), sliderRow(QStringLiteral("satin.size"), 0, 250, tr(" px")));
    QCheckBox* invert = check(QStringLiteral("satin.invert"), tr("Invert"));
    bindCheck(invert, QStringLiteral("satin.invert"));
    structure->addRow(QString(), invert);
    return page.done();
}

QWidget* LayerStyleDialog::buildColorOverlayPage()
{
    Page page;
    QFormLayout* color = page.group(tr("Color"));
    QComboBox* mode = combo(QStringLiteral("colorOverlay.mode"));
    bindBlendMode(mode, QStringLiteral("colorOverlay.mode"));
    color->addRow(tr("Blend Mode:"), pair(mode, colorButton(QStringLiteral("colorOverlay.color"))));
    color->addRow(tr("Opacity:"),
                  sliderRow(QStringLiteral("colorOverlay.opacity"), 0, 100, QStringLiteral("%")));
    return page.done();
}

QWidget* LayerStyleDialog::buildGradientOverlayPage()
{
    Page page;
    QFormLayout* gradient = page.group(tr("Gradient"));
    QComboBox* mode = combo(QStringLiteral("gradientOverlay.mode"));
    bindBlendMode(mode, QStringLiteral("gradientOverlay.mode"));
    gradient->addRow(tr("Blend Mode:"), mode);
    gradient->addRow(tr("Opacity:"), sliderRow(QStringLiteral("gradientOverlay.opacity"), 0, 100,
                                               QStringLiteral("%")));
    // Two stops rather than the gradient editor.
    auto* stops = new QHBoxLayout;
    stops->addWidget(colorButton(QStringLiteral("gradientOverlay.from")));
    stops->addWidget(colorButton(QStringLiteral("gradientOverlay.to")));
    QCheckBox* reverse = check(QStringLiteral("gradientOverlay.reverse"), tr("Reverse"));
    bindCheck(reverse, QStringLiteral("gradientOverlay.reverse"));
    stops->addWidget(reverse);
    stops->addStretch();
    gradient->addRow(tr("Gradient:"), stops);
    QComboBox* style = combo(QStringLiteral("gradientOverlay.style"),
                             {tr("Linear"), tr("Radial"), tr("Angle"), tr("Reflected"),
                              tr("Diamond")});
    bindChoice(style, QStringLiteral("gradientOverlay.style"));
    QCheckBox* align = check(QStringLiteral("gradientOverlay.align"), tr("Align with Layer"));
    bindCheck(align, QStringLiteral("gradientOverlay.align"));
    gradient->addRow(tr("Style:"), pair(style, align));
    gradient->addRow(tr("Angle:"), angleRow(QStringLiteral("gradientOverlay.angle")));
    gradient->addRow(tr("Scale:"), sliderRow(QStringLiteral("gradientOverlay.scale"), 10, 150,
                                             QStringLiteral("%")));
    QCheckBox* dither = check(QStringLiteral("gradientOverlay.dither"), tr("Dither"));
    bindCheck(dither, QStringLiteral("gradientOverlay.dither"));
    gradient->addRow(QString(), dither);
    return page.done();
}

QWidget* LayerStyleDialog::buildPatternOverlayPage()
{
    Page page;
    QFormLayout* pattern = page.group(tr("Pattern"));
    QComboBox* mode = combo(QStringLiteral("patternOverlay.mode"));
    bindBlendMode(mode, QStringLiteral("patternOverlay.mode"));
    pattern->addRow(tr("Blend Mode:"), mode);
    pattern->addRow(tr("Opacity:"), sliderRow(QStringLiteral("patternOverlay.opacity"), 0, 100,
                                              QStringLiteral("%")));
    // The built-in set; a pattern from an opened file that is not one of them
    // shows no selection until another is picked.
    QComboBox* picker =
        combo(QStringLiteral("patternOverlay.pattern"), layer_style_pattern_names());
    bindChoice(picker, QStringLiteral("patternOverlay.pattern"));
    pattern->addRow(tr("Pattern:"), picker);
    pattern->addRow(tr("Scale:"), sliderRow(QStringLiteral("patternOverlay.scale"), 1, 1000,
                                            QStringLiteral("%")));
    QCheckBox* link = check(QStringLiteral("patternOverlay.link"), tr("Link with Layer"));
    bindCheck(link, QStringLiteral("patternOverlay.link"));
    pattern->addRow(QString(), link);
    return page.done();
}

} // namespace pictura
