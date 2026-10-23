#pragma once

// Per-filter dialog descriptors, ported from perfecto25/photorust's
// MainWindow applyFilterWith parameter table.
// Source: https://github.com/perfecto25/photorust

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

// The control a filter parameter slot is edited with. Every control maps to one
// or more consecutive slots in the ordered value list the Rust mapping expects.
enum class FilterControl {
    Slider,    // one slot
    Angle,     // one slot, degrees, with a wheel
    Choice,    // one slot, an index into `choiceValues`
    Radio,     // one slot, an index; a vertical radio group
    CheckBox,  // one slot, 0 or 1
    Color,     // three slots: red, green, blue (0..255)
    Placement, // two slots: x, y (normalized 0..1)
    BlurCenter, // two slots: x, y (normalized 0..1); a draggable centre pad
    ShearCurve, // a point count and up to kShearMaxPoints (position, offset) pairs
    Hidden,     // one slot carried through the dialog with no widget (Displace's map id)
    Number,     // one slot, a value box with no slider
    Matrix,     // kMatrixCells slots: Custom's 5x5 weights, row by row
    Shape,      // one slot, a Custom Shape index picked from a grid (Shape Blur)
};

// The dialog that edits a kind. Values mirror Rust's `DialogKind`.
enum class FilterDialogKind {
    Shared,          // the shared parameter dialog
    LensFlare,
    LightingEffects,
    PictureFrame,
    ReduceNoise,
    BlurGallery,
    Liquify,
    AdaptiveWideAngle,
};

// The most control points a Shear curve carries. Lock-step with
// `pictura_filters::SHEAR_MAX_POINTS`; the curve box edits the points and the
// filter is the same points, so the two never disagree.
constexpr int kShearMaxPoints = 8;

// Custom's grid is 5x5. Lock-step with the descriptor's Matrix control.
constexpr int kMatrixSide = 5;
constexpr int kMatrixCells = kMatrixSide * kMatrixSide;

// One parameter's control identity and range. Ported from photorust's
// per-filter dialog descriptors.
struct FilterParamSpec {
    FilterControl control = FilterControl::Slider;
    QString label;
    double minimum = 0.0;
    double maximum = 100.0;
    double value = 0.0;
    // The number of engine slots this control contributes.
    int slotCount = 1;
    int decimals = 0;
    QString suffix;
    QStringList choices;
    QList<double> choiceValues;
    // Defaults for multi-slot controls (Color: r,g,b; Placement: x,y;
    // Matrix: its cells).
    QList<double> initial;
    // The value is a distance in document pixels, so a reduced proxy scales it.
    bool measuresPixels = false;
};

// One implemented Filter-menu leaf: its menu path, the `kind` the bridge maps,
// and the ordered parameters its dialog edits.
struct FilterCommandSpec {
    QStringList path; // {"Filter", [<family>], <leaf label>}; blur-gallery is {"Filter", "Blur Gallery"}
    QString label;
    QString kind;
    QList<FilterParamSpec> params;
    FilterDialogKind dialog = FilterDialogKind::Shared;
    // Displace chooses its map after the dialog's OK, not in the dialog.
    bool mapAfterOk = false;
    bool previewPane = true; // Radial Blur uses a Blur Center instead
    // One wide column under the preview instead of spilling into a second
    // input column; checkboxes trail the column.
    bool stacked = false;
    // The preview sits under the inputs (Shear) instead of beside them, with
    // no zoom row and no Preview checkbox, as CS6's Shear has none.
    bool previewBelow = false;
    // No preview at all, no Preview checkbox (Displace: its map is chosen
    // after OK, so there is nothing to show).
    bool livePreview = true;
};

// Every Filter-menu leaf that has an engine kernel, in table order.
const QList<FilterCommandSpec>& filterCommands();

// The definition for a Filter-menu leaf path, or nullptr when the leaf has no
// engine kernel (a disabled stub).
const FilterCommandSpec* filterCommandForPath(const QStringList& path);

// The definition for a bridge `kind`, or nullptr when unknown.
const FilterCommandSpec* filterCommandForKind(const QString& kind);

} // namespace pictura
