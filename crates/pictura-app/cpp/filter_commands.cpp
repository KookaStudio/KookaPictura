#include "filter_commands.h"

#include <QtCore/QStringList>

namespace pictura {

namespace {

FilterParamSpec slider(const QString& label, double minimum, double maximum, double value,
                       int decimals = 0, const QString& suffix = QString())
{
    FilterParamSpec p;
    p.control = FilterControl::Slider;
    p.label = label;
    p.minimum = minimum;
    p.maximum = maximum;
    p.value = value;
    p.decimals = decimals;
    p.suffix = suffix;
    return p;
}

FilterParamSpec angle(const QString& label, double value)
{
    FilterParamSpec p;
    p.control = FilterControl::Angle;
    p.label = label;
    p.minimum = 0.0;
    p.maximum = 360.0;
    p.value = value;
    p.suffix = QStringLiteral(" degrees");
    return p;
}

FilterParamSpec choice(const QString& label, const QStringList& names, int index)
{
    FilterParamSpec p;
    p.control = FilterControl::Choice;
    p.label = label;
    p.choices = names;
    for (int i = 0; i < names.size(); ++i) {
        p.choiceValues.append(static_cast<double>(i));
    }
    p.minimum = 0.0;
    p.maximum = static_cast<double>(names.size() - 1);
    p.value = static_cast<double>(index);
    return p;
}

// CS6's Light menu, in the engine's index order (Bottom = 0 … Top = 4).
FilterParamSpec lightDirection(int index)
{
    return choice(QStringLiteral("Light Direction:"),
                  {QStringLiteral("Bottom"), QStringLiteral("Bottom Left"), QStringLiteral("Left"),
                   QStringLiteral("Top Left"), QStringLiteral("Top"), QStringLiteral("Top Right"),
                   QStringLiteral("Right"), QStringLiteral("Bottom Right")},
                  index);
}

FilterParamSpec check(const QString& label, bool on)
{
    FilterParamSpec p;
    p.control = FilterControl::CheckBox;
    p.label = label;
    p.minimum = 0.0;
    p.maximum = 1.0;
    p.value = on ? 1.0 : 0.0;
    return p;
}

FilterParamSpec color(const QString& label, double r, double g, double b)
{
    FilterParamSpec p;
    p.control = FilterControl::Color;
    p.label = label;
    p.minimum = 0.0;
    p.maximum = 255.0;
    p.initial = {r, g, b};
    return p;
}

FilterParamSpec placement(const QString& label, double x, double y)
{
    FilterParamSpec p;
    p.control = FilterControl::Placement;
    p.label = label;
    p.minimum = 0.0;
    p.maximum = 1.0;
    p.initial = {x, y};
    return p;
}

FilterParamSpec blurCenter(const QString& label)
{
    FilterParamSpec p;
    p.control = FilterControl::BlurCenter;
    p.label = label;
    return p;
}

FilterParamSpec shearCurve(const QString& label)
{
    FilterParamSpec p;
    p.control = FilterControl::ShearCurve;
    p.label = label;
    return p;
}

FilterParamSpec hidden(double value)
{
    FilterParamSpec p;
    p.control = FilterControl::Hidden;
    p.value = value;
    return p;
}

FilterParamSpec number(const QString& label, double minimum, double maximum, double value)
{
    FilterParamSpec p = slider(label, minimum, maximum, value);
    p.control = FilterControl::Number;
    return p;
}

// Every cell in -999..999, opening on the pass-through kernel.
FilterParamSpec matrix()
{
    FilterParamSpec p;
    p.control = FilterControl::Matrix;
    p.minimum = -999.0;
    p.maximum = 999.0;
    p.initial = QList<double>(kMatrixCells, 0.0);
    p.initial[kMatrixCells / 2] = 1.0;
    return p;
}

FilterParamSpec shapePicker(const QString& label, int index)
{
    FilterParamSpec p;
    p.control = FilterControl::Shape;
    p.label = label;
    p.value = index;
    return p;
}

FilterParamSpec radio(const QString& label, const QStringList& names, int index)
{
    FilterParamSpec p = choice(label, names, index);
    p.control = FilterControl::Radio;
    return p;
}

FilterCommandSpec def(const QString& family, const QString& label, const QString& kind,
                      QList<FilterParamSpec> params, bool previewPane = true)
{
    FilterCommandSpec d;
    d.path = {QStringLiteral("Filter"), family, label};
    d.label = label;
    d.kind = kind;
    d.params = std::move(params);
    d.previewPane = previewPane;
    return d;
}

FilterCommandSpec stacked(FilterCommandSpec d)
{
    d.stacked = true;
    return d;
}

FilterCommandSpec bottomPreview(FilterCommandSpec d)
{
    d.previewBelow = true;
    return d;
}

FilterCommandSpec noPreview(FilterCommandSpec d)
{
    d.livePreview = false;
    return d;
}

FilterCommandSpec defLeaf(const QString& label, const QString& kind, QList<FilterParamSpec> params)
{
    FilterCommandSpec d;
    d.path = {QStringLiteral("Filter"), label};
    d.label = label;
    d.kind = kind;
    d.params = std::move(params);
    return d;
}

const QList<FilterCommandSpec>& buildCommands()
{
    static const QList<FilterCommandSpec> commands = {
        // Top-level leaves (no family submenu)
        defLeaf(QStringLiteral("Oil Paint…"), QStringLiteral("oil-paint"),
                {slider(QStringLiteral("Stylization:"), 0.1, 10, 3.5, 1),
                 slider(QStringLiteral("Cleanliness:"), 0, 10, 4.5, 1),
                 slider(QStringLiteral("Scale:"), 0.1, 10, 0.75, 2),
                 slider(QStringLiteral("Bristle Detail:"), 0, 10, 3.0, 1),
                 angle(QStringLiteral("Lighting Angle:"), 85.0),
                 slider(QStringLiteral("Shine:"), 0, 1, 0.55, 2)}),

        // Artistic
        def(QStringLiteral("Artistic"), QStringLiteral("Colored Pencil"),
            QStringLiteral("colored-pencil"),
            {slider(QStringLiteral("Pencil Width:"), 1, 24, 4),
             slider(QStringLiteral("Stroke Pressure:"), 0, 15, 8),
             slider(QStringLiteral("Paper Brightness:"), 0, 50, 25),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Cutout"), QStringLiteral("cutout"),
            {slider(QStringLiteral("Levels:"), 2, 8, 4),
             slider(QStringLiteral("Edge Simplicity:"), 0, 10, 4),
             slider(QStringLiteral("Edge Fidelity:"), 1, 3, 2)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Dry Brush"), QStringLiteral("dry-brush"),
            {slider(QStringLiteral("Brush Size:"), 0, 10, 2),
             slider(QStringLiteral("Brush Detail:"), 0, 10, 8),
             slider(QStringLiteral("Texture:"), 1, 3, 1),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Film Grain"), QStringLiteral("film-grain"),
            {slider(QStringLiteral("Grain:"), 0, 20, 4),
             slider(QStringLiteral("Highlight Area:"), 0, 20, 0),
             slider(QStringLiteral("Intensity:"), 0, 10, 10),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Fresco"), QStringLiteral("fresco"),
            {slider(QStringLiteral("Brush Size:"), 0, 10, 2),
             slider(QStringLiteral("Brush Detail:"), 0, 10, 8),
             slider(QStringLiteral("Texture:"), 1, 3, 1),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Neon Glow"), QStringLiteral("neon-glow"),
            {slider(QStringLiteral("Glow Size:"), -24, 24, 5),
             slider(QStringLiteral("Glow Brightness:"), 0, 50, 15),
             color(QStringLiteral("Glow Color:"), 0, 0, 255)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Paint Daubs"), QStringLiteral("paint-daubs"),
            {slider(QStringLiteral("Brush Size:"), 1, 50, 8),
             slider(QStringLiteral("Sharpness:"), 0, 40, 7),
             choice(QStringLiteral("Brush Type:"),
                    {QStringLiteral("Simple"), QStringLiteral("Light Rough"),
                     QStringLiteral("Dark Rough"), QStringLiteral("Wide Sharp"),
                     QStringLiteral("Wide Blurry"), QStringLiteral("Sparkle")},
                    0),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Palette Knife"),
            QStringLiteral("palette-knife"),
            {slider(QStringLiteral("Stroke Size:"), 1, 50, 25),
             slider(QStringLiteral("Stroke Detail:"), 1, 3, 3),
             slider(QStringLiteral("Softness:"), 0, 10, 0)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Plastic Wrap"), QStringLiteral("plastic-wrap"),
            {slider(QStringLiteral("Highlight Strength:"), 0, 20, 15),
             slider(QStringLiteral("Detail:"), 1, 15, 9),
             slider(QStringLiteral("Smoothness:"), 1, 15, 7)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Poster Edges"), QStringLiteral("poster-edges"),
            {slider(QStringLiteral("Edge Thickness:"), 0, 10, 2),
             slider(QStringLiteral("Edge Intensity:"), 0, 10, 1),
             slider(QStringLiteral("Posterization:"), 0, 10, 2)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Rough Pastels"),
            QStringLiteral("rough-pastels"),
            {slider(QStringLiteral("Stroke Length:"), 0, 40, 6),
             slider(QStringLiteral("Stroke Detail:"), 1, 20, 4),
             choice(QStringLiteral("Texture:"),
                    {QStringLiteral("Brick"), QStringLiteral("Burlap"), QStringLiteral("Canvas"),
                     QStringLiteral("Sandstone")},
                    2),
             slider(QStringLiteral("Scaling:"), 50, 200, 100, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Relief:"), 0, 50, 20),
             lightDirection(0),
             check(QStringLiteral("Invert"), false)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Smudge Stick"), QStringLiteral("smudge-stick"),
            {slider(QStringLiteral("Stroke Length:"), 0, 10, 2),
             slider(QStringLiteral("Highlight Area:"), 0, 20, 0),
             slider(QStringLiteral("Intensity:"), 0, 10, 10),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Sponge"), QStringLiteral("sponge"),
            {slider(QStringLiteral("Brush Size:"), 0, 10, 2),
             slider(QStringLiteral("Definition:"), 0, 25, 12),
             slider(QStringLiteral("Smoothness:"), 1, 15, 5),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Underpainting"),
            QStringLiteral("underpainting"),
            {slider(QStringLiteral("Brush Size:"), 0, 40, 6),
             slider(QStringLiteral("Texture Coverage:"), 0, 40, 16),
             choice(QStringLiteral("Texture:"),
                    {QStringLiteral("Brick"), QStringLiteral("Burlap"), QStringLiteral("Canvas"),
                     QStringLiteral("Sandstone")},
                    2),
             slider(QStringLiteral("Scaling:"), 50, 200, 100, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Relief:"), 0, 50, 4),
             lightDirection(4),
             check(QStringLiteral("Invert"), false),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Artistic"), QStringLiteral("Watercolor"), QStringLiteral("watercolor"),
            {slider(QStringLiteral("Brush Detail:"), 1, 14, 9),
             slider(QStringLiteral("Shadow Intensity:"), 0, 10, 1),
             slider(QStringLiteral("Texture:"), 1, 3, 1),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),

        // Blur
        // Blur Gallery: the pins and the Effects/Noise tabs live on its
        // workspace, not this pane; the canonical slots are one centred pin
        // plus the effects tail (`filter_map.rs`).
        def(QStringLiteral("Blur Gallery"), QStringLiteral("Field Blur"),
            QStringLiteral("field-blur"),
            {hidden(1), // pin count
             placement(QStringLiteral("Pin:"), 0.5, 0.5),
             slider(QStringLiteral("Blur:"), 0, 500, 0, 0, QStringLiteral(" pixels")),
             check(QStringLiteral("Bokeh"), true),
             slider(QStringLiteral("Light Bokeh:"), 0, 100, 14, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Bokeh Color:"), 0, 100, 40, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Light Range Low:"), 0, 255, 40),
             slider(QStringLiteral("Light Range High:"), 0, 255, 118),
             check(QStringLiteral("Noise"), false),
             slider(QStringLiteral("Noise Amount:"), 0, 100, 0, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Noise Size:"), 0, 100, 50, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Noise Roughness:"), 0, 100, 50, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Noise Color:"), 0, 100, 0, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Noise Highlights:"), 0, 100, 0, 0, QStringLiteral(" %")),
             choice(QStringLiteral("Noise Type:"),
                    {QStringLiteral("Gaussian"), QStringLiteral("Uniform"),
                     QStringLiteral("Grain")},
                    0),
             hidden(1)}), // seed
        def(QStringLiteral("Blur"), QStringLiteral("Gaussian Blur"), QStringLiteral("gaussian-blur"),
            {slider(QStringLiteral("Radius:"), 0.1, 250.0, 5.0, 1, QStringLiteral(" pixels"))}),
        def(QStringLiteral("Blur"), QStringLiteral("Motion Blur"), QStringLiteral("motion-blur"),
            {angle(QStringLiteral("Angle:"), 0.0),
             slider(QStringLiteral("Distance:"), 1, 999, 15, 0, QStringLiteral(" pixels"))}),
        def(QStringLiteral("Blur"), QStringLiteral("Radial Blur"), QStringLiteral("radial-blur"),
            {choice(QStringLiteral("Blur Method:"), {QStringLiteral("Spin"), QStringLiteral("Zoom")},
                    0),
             slider(QStringLiteral("Amount:"), 1, 100, 10),
             choice(QStringLiteral("Quality:"),
                    {QStringLiteral("Draft"), QStringLiteral("Good"), QStringLiteral("Best")}, 1),
             blurCenter(QStringLiteral("Blur Center:"))},
            false),
        def(QStringLiteral("Blur"), QStringLiteral("Box Blur"), QStringLiteral("box-blur"),
            {slider(QStringLiteral("Radius:"), 1, 250, 3, 0, QStringLiteral(" pixels"))}),
        // The kernel is a Custom Shape preset, Radius its half-width; the
        // range and default are inferred (FILT-002 leaves them unverified).
        stacked(def(QStringLiteral("Blur"), QStringLiteral("Shape Blur"),
                    QStringLiteral("shape-blur"),
                    {slider(QStringLiteral("Radius:"), 5, 1000, 10, 0, QStringLiteral(" pixels")),
                     shapePicker(QStringLiteral("Shape:"), 0)})),
        def(QStringLiteral("Blur"), QStringLiteral("Surface Blur"), QStringLiteral("surface-blur"),
            {slider(QStringLiteral("Radius:"), 1, 100, 10, 0, QStringLiteral(" pixels")),
             slider(QStringLiteral("Threshold:"), 2, 255, 20, 0, QStringLiteral(" levels"))}),
        // Range and Normal mode are sourced; the defaults (Radius 3, Threshold
        // 25, Quality Low) are inferred (FILT-002 leaves them unverified).
        def(QStringLiteral("Blur"), QStringLiteral("Smart Blur"), QStringLiteral("smart-blur"),
            {slider(QStringLiteral("Radius:"), 0.1, 100.0, 3.0, 1),
             slider(QStringLiteral("Threshold:"), 0.1, 100.0, 25.0, 1),
             choice(QStringLiteral("Quality:"),
                    {QStringLiteral("Low"), QStringLiteral("Medium"), QStringLiteral("High")}, 0),
             choice(QStringLiteral("Mode:"),
                    {QStringLiteral("Normal"), QStringLiteral("Edge Only"),
                     QStringLiteral("Overlay Edge")},
                    0)}),
        // CS6's dialog values (#338). The Source slot is resolved into a depth
        // map id in the first hidden slot on every preview and commit.
        stacked(def(QStringLiteral("Blur"), QStringLiteral("Lens Blur"), QStringLiteral("lens-blur"),
                    {radio(QStringLiteral("Preview:"),
                           {QStringLiteral("Faster"), QStringLiteral("More Accurate")}, 0),
                     choice(QStringLiteral("Source:"),
                            {QStringLiteral("None"), QStringLiteral("Transparency"),
                             QStringLiteral("Layer Mask"), QStringLiteral("Selection")},
                            0),
                     slider(QStringLiteral("Blur Focal Distance:"), 0, 255, 126),
                     check(QStringLiteral("Invert"), false),
                     choice(QStringLiteral("Shape:"),
                            {QStringLiteral("Triangle (3)"), QStringLiteral("Square (4)"),
                             QStringLiteral("Pentagon (5)"), QStringLiteral("Hexagon (6)"),
                             QStringLiteral("Heptagon (7)"), QStringLiteral("Octagon (8)")},
                            0),
                     slider(QStringLiteral("Radius:"), 0, 100, 50, 0, QStringLiteral(" pixels")),
                     slider(QStringLiteral("Blade Curvature:"), 0, 100, 33),
                     angle(QStringLiteral("Rotation:"), 191.0),
                     slider(QStringLiteral("Specular Brightness:"), 0, 100, 44),
                     slider(QStringLiteral("Specular Threshold:"), 0, 255, 93),
                     slider(QStringLiteral("Noise Amount:"), 0, 100, 47),
                     radio(QStringLiteral("Distribution:"),
                           {QStringLiteral("Uniform"), QStringLiteral("Gaussian")}, 1),
                     check(QStringLiteral("Monochromatic"), false),
                     hidden(0), // depth map id
                     hidden(1)})), // noise seed
        def(QStringLiteral("Blur"), QStringLiteral("Average"), QStringLiteral("average"), {}),
        def(QStringLiteral("Blur"), QStringLiteral("Blur"), QStringLiteral("blur"), {}),
        def(QStringLiteral("Blur"), QStringLiteral("Blur More"), QStringLiteral("blur-more"), {}),

        // Brush Strokes
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Accented Edges"),
            QStringLiteral("accented-edges"),
            {slider(QStringLiteral("Edge Width:"), 1, 14, 2),
             slider(QStringLiteral("Edge Brightness:"), 0, 50, 38),
             slider(QStringLiteral("Smoothness:"), 1, 15, 5)}),
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Angled Strokes"),
            QStringLiteral("angled-strokes"),
            {slider(QStringLiteral("Direction Balance:"), 0, 100, 50),
             slider(QStringLiteral("Stroke Length:"), 3, 50, 15),
             slider(QStringLiteral("Sharpness:"), 0, 10, 3)}),
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Crosshatch"), QStringLiteral("crosshatch"),
            {slider(QStringLiteral("Stroke Length:"), 3, 50, 9),
             slider(QStringLiteral("Sharpness:"), 0, 20, 6),
             slider(QStringLiteral("Strength:"), 1, 3, 1)}),
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Dark Strokes"),
            QStringLiteral("dark-strokes"),
            {slider(QStringLiteral("Balance:"), 0, 10, 5),
             slider(QStringLiteral("Black Intensity:"), 0, 10, 6),
             slider(QStringLiteral("White Intensity:"), 0, 10, 5)}),
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Ink Outlines"),
            QStringLiteral("ink-outlines"),
            {slider(QStringLiteral("Stroke Length:"), 1, 50, 10),
             slider(QStringLiteral("Dark Intensity:"), 0, 50, 25),
             slider(QStringLiteral("Light Intensity:"), 0, 50, 25)}),
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Spatter"), QStringLiteral("spatter"),
            {slider(QStringLiteral("Spray Radius:"), 0, 25, 10),
             slider(QStringLiteral("Smoothness:"), 1, 15, 5), slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Sprayed Strokes"),
            QStringLiteral("sprayed-strokes"),
            {slider(QStringLiteral("Stroke Length:"), 0, 20, 12),
             slider(QStringLiteral("Spray Radius:"), 0, 25, 7),
             choice(QStringLiteral("Direction:"),
                    {QStringLiteral("Right Diagonal"), QStringLiteral("Horizontal"),
                     QStringLiteral("Left Diagonal"), QStringLiteral("Vertical")},
                    0),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Brush Strokes"), QStringLiteral("Sumi-e"), QStringLiteral("sumi-e"),
            {slider(QStringLiteral("Stroke Width:"), 3, 15, 8),
             slider(QStringLiteral("Stroke Pressure:"), 0, 15, 5),
             slider(QStringLiteral("Contrast:"), 0, 40, 20)}),

        // Distort
        // photorust's defaults; the map is chosen after OK and its id fills
        // the hidden slot.
        noPreview(def(QStringLiteral("Distort"), QStringLiteral("Displace"),
                      QStringLiteral("displace"),
                      {slider(QStringLiteral("Horizontal Scale:"), -999, 999, 10, 0,
                              QStringLiteral(" %")),
                       slider(QStringLiteral("Vertical Scale:"), -999, 999, 10, 0,
                              QStringLiteral(" %")),
                       radio(QStringLiteral("Displacement Map:"),
                             {QStringLiteral("Stretch To Fit"), QStringLiteral("Tile")}, 0),
                       radio(QStringLiteral("Undefined Areas:"),
                             {QStringLiteral("Wrap Around"),
                              QStringLiteral("Repeat Edge Pixels")},
                             1),
                       hidden(0)},
                      false)),
        def(QStringLiteral("Distort"), QStringLiteral("Diffuse Glow"), QStringLiteral("diffuse-glow"),
            {slider(QStringLiteral("Graininess:"), 0, 10, 6),
             slider(QStringLiteral("Glow Amount:"), 0, 20, 10),
             slider(QStringLiteral("Clear Amount:"), 0, 20, 15),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Distort"), QStringLiteral("Glass"), QStringLiteral("glass"),
            {slider(QStringLiteral("Distortion:"), 0, 20, 5),
             slider(QStringLiteral("Smoothness:"), 1, 15, 3),
             choice(QStringLiteral("Texture:"),
                    {QStringLiteral("Blocks"), QStringLiteral("Canvas"), QStringLiteral("Frosted"),
                     QStringLiteral("Tiny Lens")},
                    2),
             slider(QStringLiteral("Scaling:"), 50, 200, 100, 0, QStringLiteral(" %")),
             check(QStringLiteral("Invert"), false)}),
        def(QStringLiteral("Distort"), QStringLiteral("Ocean Ripple"), QStringLiteral("ocean-ripple"),
            {slider(QStringLiteral("Ripple Size:"), 1, 15, 9),
             slider(QStringLiteral("Ripple Magnitude:"), 0, 20, 9),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Distort"), QStringLiteral("Pinch"), QStringLiteral("pinch"),
            {slider(QStringLiteral("Amount:"), -100, 100, 50)}),
        def(QStringLiteral("Distort"), QStringLiteral("Polar Coordinates"),
            QStringLiteral("polar-coordinates"),
            {choice(QStringLiteral("Type:"),
                    {QStringLiteral("Rectangular to Polar"), QStringLiteral("Polar to Rectangular")},
                    0)}),
        def(QStringLiteral("Distort"), QStringLiteral("Ripple"), QStringLiteral("ripple"),
            {slider(QStringLiteral("Amount:"), -999, 999, 100),
             choice(QStringLiteral("Size:"),
                    {QStringLiteral("Small"), QStringLiteral("Medium"), QStringLiteral("Large")}, 1)}),
        bottomPreview(def(QStringLiteral("Distort"), QStringLiteral("Shear"),
            QStringLiteral("shear"),
            {shearCurve(QStringLiteral("Curve:")),
             radio(QStringLiteral("Undefined Areas:"),
                   {QStringLiteral("Wrap Around"), QStringLiteral("Repeat Edge Pixels")}, 0)})),
        def(QStringLiteral("Distort"), QStringLiteral("Spherize"), QStringLiteral("spherize"),
            {slider(QStringLiteral("Amount:"), -100, 100, 100),
             choice(QStringLiteral("Mode:"),
                    {QStringLiteral("Normal"), QStringLiteral("Horizontal Only"),
                     QStringLiteral("Vertical Only")},
                    0)}),
        def(QStringLiteral("Distort"), QStringLiteral("Twirl"), QStringLiteral("twirl"),
            {angle(QStringLiteral("Angle:"), 90.0)}),
        def(QStringLiteral("Distort"), QStringLiteral("Wave"), QStringLiteral("wave"),
            {slider(QStringLiteral("Number of Generators:"), 1, 999, 5),
             slider(QStringLiteral("Wavelength Min:"), 1, 998, 10),
             slider(QStringLiteral("Wavelength Max:"), 1, 999, 120),
             slider(QStringLiteral("Amplitude Min:"), 1, 998, 5),
             slider(QStringLiteral("Amplitude Max:"), 1, 999, 35),
             choice(QStringLiteral("Type:"),
                    {QStringLiteral("Sine"), QStringLiteral("Triangle"), QStringLiteral("Square")}, 0),
             slider(QStringLiteral("Scale X:"), 1, 100, 100),
             slider(QStringLiteral("Scale Y:"), 1, 100, 100),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Distort"), QStringLiteral("ZigZag"), QStringLiteral("zigzag"),
            {slider(QStringLiteral("Amount:"), -100, 100, 50),
             slider(QStringLiteral("Ridges:"), 1, 20, 5),
             choice(QStringLiteral("Style:"),
                    {QStringLiteral("Around Center"), QStringLiteral("Out From Center"),
                     QStringLiteral("Pond Ripples")},
                    0)}),

        // Noise
        def(QStringLiteral("Noise"), QStringLiteral("Add Noise"), QStringLiteral("add-noise"),
            {slider(QStringLiteral("Amount:"), 0.1, 400.0, 25.0, 2, QStringLiteral(" %")),
             choice(QStringLiteral("Distribution:"),
                    {QStringLiteral("Uniform"), QStringLiteral("Gaussian")}, 0),
             check(QStringLiteral("Monochromatic"), false),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Noise"), QStringLiteral("Despeckle"), QStringLiteral("despeckle"), {}),
        def(QStringLiteral("Noise"), QStringLiteral("Dust & Scratches"),
            QStringLiteral("dust-and-scratches"),
            {slider(QStringLiteral("Radius:"), 1, 16, 1, 0, QStringLiteral(" pixels")),
             slider(QStringLiteral("Threshold:"), 0, 255, 0, 0, QStringLiteral(" levels"))}),
        def(QStringLiteral("Noise"), QStringLiteral("Median"), QStringLiteral("median"),
            {slider(QStringLiteral("Radius:"), 1, 100, 2, 0, QStringLiteral(" pixels"))}),

        // Pixelate
        def(QStringLiteral("Pixelate"), QStringLiteral("Color Halftone"),
            QStringLiteral("color-halftone"),
            {slider(QStringLiteral("Max Radius:"), 4, 127, 5),
             slider(QStringLiteral("Screen Angle 1:"), -360, 360, 108),
             slider(QStringLiteral("Screen Angle 2:"), -360, 360, 162),
             slider(QStringLiteral("Screen Angle 3:"), -360, 360, 90),
             slider(QStringLiteral("Screen Angle 4:"), -360, 360, 45)}),
        def(QStringLiteral("Pixelate"), QStringLiteral("Crystallize"), QStringLiteral("crystallize"),
            {slider(QStringLiteral("Cell Size:"), 3, 300, 10),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Pixelate"), QStringLiteral("Facet"), QStringLiteral("facet"), {}),
        def(QStringLiteral("Pixelate"), QStringLiteral("Fragment"), QStringLiteral("fragment"), {}),
        def(QStringLiteral("Pixelate"), QStringLiteral("Mezzotint"), QStringLiteral("mezzotint"),
            {choice(QStringLiteral("Type:"),
                    {QStringLiteral("Fine Dots"), QStringLiteral("Medium Dots"),
                     QStringLiteral("Grainy Dots"), QStringLiteral("Coarse Dots"),
                     QStringLiteral("Short Lines"), QStringLiteral("Medium Lines"),
                     QStringLiteral("Long Lines"), QStringLiteral("Short Strokes"),
                     QStringLiteral("Medium Strokes"), QStringLiteral("Long Strokes")},
                    0),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Pixelate"), QStringLiteral("Mosaic"), QStringLiteral("mosaic"),
            {slider(QStringLiteral("Cell Size:"), 2, 200, 10, 0, QStringLiteral(" square"))}),
        def(QStringLiteral("Pixelate"), QStringLiteral("Pointillize"), QStringLiteral("pointillize"),
            {slider(QStringLiteral("Cell Size:"), 3, 300, 5),
             color(QStringLiteral("Background:"), 0, 0, 0),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),

        // Render
        def(QStringLiteral("Render"), QStringLiteral("Clouds"), QStringLiteral("clouds"),
            {color(QStringLiteral("Color A:"), 0, 0, 0), color(QStringLiteral("Color B:"), 255, 255, 255),
             check(QStringLiteral("Starker"), false), slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Render"), QStringLiteral("Difference Clouds"),
            QStringLiteral("difference-clouds"),
            {color(QStringLiteral("Color A:"), 0, 0, 0), color(QStringLiteral("Color B:"), 255, 255, 255),
             check(QStringLiteral("Starker"), false), slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Render"), QStringLiteral("Fibers"), QStringLiteral("fibers"),
            {slider(QStringLiteral("Variance:"), 1, 64, 16),
             slider(QStringLiteral("Strength:"), 1, 64, 4),
             color(QStringLiteral("Color A:"), 0, 0, 0), color(QStringLiteral("Color B:"), 255, 255, 255),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Render"), QStringLiteral("Lens Flare"), QStringLiteral("lens-flare"),
            {slider(QStringLiteral("Brightness:"), 10, 300, 100),
             placement(QStringLiteral("Flare Center:"), 0.5, 0.5),
             choice(QStringLiteral("Lens Type:"),
                    {QStringLiteral("50-300mm Zoom"), QStringLiteral("35mm Prime"),
                     QStringLiteral("105mm Prime"), QStringLiteral("Movie Prime")},
                    0)}),
        // Lighting Effects' rig, then one light: its workspace adds lights as
        // further runs of the light's slots (`filter_map.rs`).
        def(QStringLiteral("Render"), QStringLiteral("Lighting Effects"),
            QStringLiteral("lighting-effects"),
            {color(QStringLiteral("Colorize:"), 255, 255, 255),
             slider(QStringLiteral("Exposure:"), -100, 100, 0),
             slider(QStringLiteral("Gloss:"), -100, 100, 0),
             slider(QStringLiteral("Metallic:"), -100, 100, 0),
             slider(QStringLiteral("Ambience:"), -100, 100, 0),
             choice(QStringLiteral("Texture:"),
                    {QStringLiteral("None"), QStringLiteral("Red"), QStringLiteral("Green"),
                     QStringLiteral("Blue")},
                    0),
             slider(QStringLiteral("Height:"), 0, 100, 50),
             choice(QStringLiteral("Light Type:"),
                    {QStringLiteral("Spot"), QStringLiteral("Point"), QStringLiteral("Infinite")},
                    0),
             check(QStringLiteral("Visible"), true),
             color(QStringLiteral("Color:"), 255, 255, 255),
             slider(QStringLiteral("Intensity:"), -100, 100, 35),
             slider(QStringLiteral("Hotspot:"), -100, 100, 69),
             placement(QStringLiteral("Light Center:"), 0.5, 0.5),
             angle(QStringLiteral("Angle:"), 325.0),
             slider(QStringLiteral("Size:"), 0.01, 3.0, 0.6, 2),
             slider(QStringLiteral("Width:"), 0.01, 3.0, 0.32, 2),
             slider(QStringLiteral("Elevation:"), 0, 90, 50)}),

        // Sharpen
        stacked(def(QStringLiteral("Sharpen"), QStringLiteral("Smart Sharpen"),
            QStringLiteral("smart-sharpen"),
            {slider(QStringLiteral("Amount:"), 1, 500, 100, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Radius:"), 0.1, 64.0, 1.0, 1, QStringLiteral(" px")),
             slider(QStringLiteral("Reduce Noise:"), 0, 100, 0, 0, QStringLiteral(" %")),
             choice(QStringLiteral("Remove:"),
                    {QStringLiteral("Gaussian Blur"), QStringLiteral("Lens Blur"),
                     QStringLiteral("Motion Blur")},
                    0),
             angle(QStringLiteral("Angle:"), 0.0),
             check(QStringLiteral("More Accurate"), false),
             slider(QStringLiteral("Shadow Amount:"), 0, 100, 0, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Shadow Width:"), 0, 100, 50, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Shadow Radius:"), 1, 100, 1),
             slider(QStringLiteral("Highlight Amount:"), 0, 100, 0, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Highlight Width:"), 0, 100, 50, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Highlight Radius:"), 1, 100, 1)})),
        def(QStringLiteral("Sharpen"), QStringLiteral("Unsharp Mask"),
            QStringLiteral("unsharp-mask"),
            {slider(QStringLiteral("Amount:"), 1, 500, 150, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Radius:"), 0.1, 250.0, 1.0, 1, QStringLiteral(" pixels")),
             slider(QStringLiteral("Threshold:"), 0, 255, 0, 0, QStringLiteral(" levels"))}),
        def(QStringLiteral("Sharpen"), QStringLiteral("Sharpen"), QStringLiteral("sharpen"), {}),
        def(QStringLiteral("Sharpen"), QStringLiteral("Sharpen Edges"),
            QStringLiteral("sharpen-edges"), {}),
        def(QStringLiteral("Sharpen"), QStringLiteral("Sharpen More"),
            QStringLiteral("sharpen-more"), {}),

        // Sketch
        def(QStringLiteral("Sketch"), QStringLiteral("Bas Relief"), QStringLiteral("bas-relief"),
            {slider(QStringLiteral("Detail:"), 1, 15, 6),
             slider(QStringLiteral("Smoothness:"), 1, 15, 3),
             choice(QStringLiteral("Light:"),
                    {QStringLiteral("Bottom"), QStringLiteral("Bottom Left"), QStringLiteral("Left"),
                     QStringLiteral("Top Left"), QStringLiteral("Top"), QStringLiteral("Top Right"),
                     QStringLiteral("Right"), QStringLiteral("Bottom Right")},
                    0),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Chalk & Charcoal"),
            QStringLiteral("chalk-charcoal"),
            {slider(QStringLiteral("Charcoal Area:"), 0, 20, 6),
             slider(QStringLiteral("Chalk Area:"), 0, 20, 6),
             slider(QStringLiteral("Stroke Pressure:"), 0, 5, 1),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Charcoal"), QStringLiteral("charcoal"),
            {slider(QStringLiteral("Thickness:"), 1, 7, 1),
             slider(QStringLiteral("Detail:"), 0, 5, 3),
             slider(QStringLiteral("Light/Dark Balance:"), 0, 100, 50),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Chrome"), QStringLiteral("chrome"),
            {slider(QStringLiteral("Detail:"), 0, 10, 4),
             slider(QStringLiteral("Smoothness:"), 0, 10, 7)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Conté Crayon"), QStringLiteral("conte-crayon"),
            {slider(QStringLiteral("Foreground Level:"), 1, 15, 8),
             slider(QStringLiteral("Background Level:"), 1, 15, 7),
             choice(QStringLiteral("Texture:"),
                    {QStringLiteral("Brick"), QStringLiteral("Burlap"), QStringLiteral("Canvas"),
                     QStringLiteral("Sandstone")},
                    2),
             slider(QStringLiteral("Texture Scaling:"), 50, 200, 100),
             slider(QStringLiteral("Texture Relief:"), 0, 50, 4),
             slider(QStringLiteral("Texture Light:"), 0, 7, 0),
             check(QStringLiteral("Invert Texture"), false),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Graphic Pen"), QStringLiteral("graphic-pen"),
            {slider(QStringLiteral("Stroke Length:"), 1, 15, 6),
             slider(QStringLiteral("Light/Dark Balance:"), 0, 100, 50),
             choice(QStringLiteral("Stroke Direction:"),
                    {QStringLiteral("Right Diagonal"), QStringLiteral("Horizontal"),
                     QStringLiteral("Left Diagonal"), QStringLiteral("Vertical")},
                    0),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Halftone Pattern"),
            QStringLiteral("halftone-pattern"),
            {slider(QStringLiteral("Size:"), 1, 12, 5),
             slider(QStringLiteral("Contrast:"), 0, 50, 5),
             choice(QStringLiteral("Pattern Type:"),
                    {QStringLiteral("Dot"), QStringLiteral("Line"), QStringLiteral("Circle")}, 0)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Note Paper"), QStringLiteral("note-paper"),
            {slider(QStringLiteral("Image Balance:"), 0, 50, 25),
             slider(QStringLiteral("Graininess:"), 0, 20, 10),
             slider(QStringLiteral("Relief:"), 0, 25, 11),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Photocopy"), QStringLiteral("photocopy"),
            {slider(QStringLiteral("Detail:"), 1, 24, 5),
             slider(QStringLiteral("Darkness:"), 1, 50, 20)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Plaster"), QStringLiteral("plaster"),
            {slider(QStringLiteral("Image Balance:"), 0, 50, 25),
             slider(QStringLiteral("Smoothness:"), 1, 15, 2),
             choice(QStringLiteral("Light:"),
                    {QStringLiteral("Bottom"), QStringLiteral("Bottom Left"), QStringLiteral("Left"),
                     QStringLiteral("Top Left"), QStringLiteral("Top"), QStringLiteral("Top Right"),
                     QStringLiteral("Right"), QStringLiteral("Bottom Right")},
                    0),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Reticulation"),
            QStringLiteral("reticulation"),
            {slider(QStringLiteral("Density:"), 0, 50, 13),
             slider(QStringLiteral("Black Level:"), 0, 50, 10),
             slider(QStringLiteral("White Level:"), 0, 50, 40),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Stamp"), QStringLiteral("stamp"),
            {slider(QStringLiteral("Light/Dark Balance:"), 0, 50, 25),
             slider(QStringLiteral("Smoothness:"), 1, 50, 5),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Torn Edges"), QStringLiteral("torn-edges"),
            {slider(QStringLiteral("Image Balance:"), 0, 50, 25),
             slider(QStringLiteral("Smoothness:"), 1, 15, 1),
             slider(QStringLiteral("Contrast:"), 1, 25, 8),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255)}),
        def(QStringLiteral("Sketch"), QStringLiteral("Water Paper"), QStringLiteral("water-paper"),
            {slider(QStringLiteral("Fiber Length:"), 3, 50, 15),
             slider(QStringLiteral("Brightness:"), 0, 100, 45),
             slider(QStringLiteral("Contrast:"), 0, 100, 60),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),

        // Stylize
        def(QStringLiteral("Stylize"), QStringLiteral("Emboss"), QStringLiteral("emboss"),
            {angle(QStringLiteral("Angle:"), 135.0),
             slider(QStringLiteral("Height:"), 1, 10, 2),
             slider(QStringLiteral("Amount:"), 1, 500, 100, 0, QStringLiteral(" %"))}),
        def(QStringLiteral("Stylize"), QStringLiteral("Extrude"), QStringLiteral("extrude"),
            {choice(QStringLiteral("Type:"),
                    {QStringLiteral("Blocks"), QStringLiteral("Pyramids")}, 0),
             slider(QStringLiteral("Size:"), 2, 255, 30),
             slider(QStringLiteral("Depth:"), 1, 255, 30),
             check(QStringLiteral("Level-based"), true),
             check(QStringLiteral("Solid Front Faces"), false),
             check(QStringLiteral("Mask Incomplete Blocks"), false)}),
        def(QStringLiteral("Stylize"), QStringLiteral("Find Edges"), QStringLiteral("find-edges"), {}),
        def(QStringLiteral("Stylize"), QStringLiteral("Solarize"), QStringLiteral("solarize"), {}),
        def(QStringLiteral("Stylize"), QStringLiteral("Diffuse"), QStringLiteral("diffuse"),
            {choice(QStringLiteral("Mode:"),
                    {QStringLiteral("Normal"), QStringLiteral("Darken Only"),
                     QStringLiteral("Lighten Only"), QStringLiteral("Anisotropic")},
                    0)}),
        def(QStringLiteral("Stylize"), QStringLiteral("Glowing Edges"),
            QStringLiteral("glowing-edges"),
            {slider(QStringLiteral("Edge Width:"), 1, 14, 2),
             slider(QStringLiteral("Edge Brightness:"), 0, 20, 6),
             slider(QStringLiteral("Smoothness:"), 1, 15, 1)}),
        def(QStringLiteral("Stylize"), QStringLiteral("Tiles"), QStringLiteral("tiles"),
            {slider(QStringLiteral("Number of Tiles:"), 1, 99, 10),
             slider(QStringLiteral("Maximum Offset:"), 1, 99, 10, 0, QStringLiteral(" %")),
             choice(QStringLiteral("Fill Empty Area With:"),
                    {QStringLiteral("Background Color"), QStringLiteral("Foreground Color"),
                     QStringLiteral("Inverse Image"), QStringLiteral("Unaltered Image")},
                    0),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             color(QStringLiteral("Background:"), 255, 255, 255)}),
        def(QStringLiteral("Stylize"), QStringLiteral("Trace Contour"),
            QStringLiteral("trace-contour"),
            {slider(QStringLiteral("Level:"), 0, 255, 128, 0, QStringLiteral(" levels")),
             choice(QStringLiteral("Edge:"),
                    {QStringLiteral("Lower"), QStringLiteral("Upper")}, 0)}),
        def(QStringLiteral("Stylize"), QStringLiteral("Wind"), QStringLiteral("wind"),
            {choice(QStringLiteral("Method:"),
                    {QStringLiteral("Wind"), QStringLiteral("Blast"), QStringLiteral("Stagger")}, 0),
             choice(QStringLiteral("Direction:"),
                    {QStringLiteral("From the Right"), QStringLiteral("From the Left")}, 0)}),

        // Texture
        def(QStringLiteral("Texture"), QStringLiteral("Craquelure"), QStringLiteral("craquelure"),
            {slider(QStringLiteral("Crack Spacing:"), 2, 100, 10),
             slider(QStringLiteral("Crack Depth:"), 0, 10, 6),
             slider(QStringLiteral("Crack Brightness:"), 0, 10, 9)}),
        def(QStringLiteral("Texture"), QStringLiteral("Grain"), QStringLiteral("grain"),
            {slider(QStringLiteral("Intensity:"), 0, 100, 40),
             slider(QStringLiteral("Contrast:"), 0, 100, 50),
             choice(QStringLiteral("Grain Type:"),
                    {QStringLiteral("Regular"), QStringLiteral("Soft"), QStringLiteral("Sprinkles"),
                     QStringLiteral("Clumped"), QStringLiteral("Contrasty"), QStringLiteral("Enlarged"),
                     QStringLiteral("Stippled"), QStringLiteral("Horizontal"), QStringLiteral("Vertical"),
                     QStringLiteral("Speckle")},
                    0),
             color(QStringLiteral("Background:"), 255, 255, 255),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Texture"), QStringLiteral("Mosaic Tiles"), QStringLiteral("mosaic-tiles"),
            {slider(QStringLiteral("Tile Size:"), 2, 100, 12),
             slider(QStringLiteral("Grout Width:"), 1, 15, 3),
             slider(QStringLiteral("Lighten Grout:"), 0, 10, 1),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Texture"), QStringLiteral("Patchwork"), QStringLiteral("patchwork"),
            {slider(QStringLiteral("Square Size:"), 1, 10, 5),
             slider(QStringLiteral("Relief:"), 0, 20, 8),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Texture"), QStringLiteral("Stained Glass"),
            QStringLiteral("stained-glass"),
            {slider(QStringLiteral("Cell Size:"), 2, 50, 10),
             slider(QStringLiteral("Border Thickness:"), 1, 20, 4),
             slider(QStringLiteral("Light Intensity:"), 0, 10, 5),
             color(QStringLiteral("Foreground:"), 0, 0, 0),
             slider(QStringLiteral("Seed:"), 0, 999, 1)}),
        def(QStringLiteral("Texture"), QStringLiteral("Texturizer"), QStringLiteral("texturizer"),
            {choice(QStringLiteral("Texture:"),
                    {QStringLiteral("Brick"), QStringLiteral("Burlap"), QStringLiteral("Canvas"),
                     QStringLiteral("Sandstone")},
                    2),
             slider(QStringLiteral("Scaling:"), 50, 200, 100, 0, QStringLiteral(" %")),
             slider(QStringLiteral("Relief:"), 0, 50, 4),
             slider(QStringLiteral("Light Direction:"), 0, 7, 0),
             check(QStringLiteral("Invert"), false)}),

        // Other
        // Scale and Offset are plain value boxes, as in CS6.
        def(QStringLiteral("Other"), QStringLiteral("Custom"), QStringLiteral("custom"),
            {matrix(), number(QStringLiteral("Scale:"), 1, 9999, 1),
             number(QStringLiteral("Offset:"), -9999, 9999, 0)}),
        def(QStringLiteral("Other"), QStringLiteral("High Pass"), QStringLiteral("high-pass"),
            {slider(QStringLiteral("Radius:"), 0.1, 250.0, 4.0, 1, QStringLiteral(" pixels"))}),
        def(QStringLiteral("Other"), QStringLiteral("Maximum"), QStringLiteral("maximum"),
            {slider(QStringLiteral("Radius:"), 1, 100, 2, 0, QStringLiteral(" pixels"))}),
        def(QStringLiteral("Other"), QStringLiteral("Minimum"), QStringLiteral("minimum"),
            {slider(QStringLiteral("Radius:"), 1, 100, 2, 0, QStringLiteral(" pixels"))}),
        def(QStringLiteral("Other"), QStringLiteral("Offset"), QStringLiteral("offset"),
            {slider(QStringLiteral("Horizontal:"), -2000, 2000, 4, 0, QStringLiteral(" pixels right")),
             slider(QStringLiteral("Vertical:"), -2000, 2000, 4, 0, QStringLiteral(" pixels down")),
             check(QStringLiteral("Wrap Around"), true),
             color(QStringLiteral("Background:"), 0, 0, 0)}),
    };
    return commands;
}

} // namespace

const QList<FilterCommandSpec>& filterCommands()
{
    return buildCommands();
}

const FilterCommandSpec* filterCommandForPath(const QStringList& path)
{
    for (const FilterCommandSpec& spec : filterCommands()) {
        if (spec.path == path) {
            return &spec;
        }
    }
    return nullptr;
}

const FilterCommandSpec* filterCommandForKind(const QString& kind)
{
    for (const FilterCommandSpec& spec : filterCommands()) {
        if (spec.kind == kind) {
            return &spec;
        }
    }
    return nullptr;
}

} // namespace pictura
