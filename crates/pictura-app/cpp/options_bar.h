#pragma once

#include <QtWidgets/QToolBar>

#include <functional>
#include <utility>
#include <vector>

#include "tools.h"

class QStackedWidget;
class QHBoxLayout;
class QIcon;
class QMenu;
class QResizeEvent;
class QToolButton;

namespace pictura {

class ToolController;
class BrushPresetPicker;
class NumericField;
struct NumericFieldConfig;

// Context-sensitive options bar: one stacked page per tool, switched by the
// frame when the active tool changes.
class OptionsBar : public QToolBar {
    Q_OBJECT

public:
    explicit OptionsBar(ToolController* controller, QWidget* parent = nullptr);

    void showTool(ToolId id);
    // Enable the Move tool's Align and Distribute buttons (the frame decides
    // from the selected layers and the selection).
    void setAlignEnabled(bool align, bool distribute);
    // The right-aligned workspace switcher: shows the active workspace and opens
    // the `Window > Workspace` menu.
    void setWorkspaceMenu(QMenu* menu);
    void setActiveWorkspace(const QString& name);
    QToolButton* workspaceSwitcherForTest() const { return workspaceSwitcher_; }

signals:
    // A panel toggle (Toggle the Brush panel, Toggle the Clone Source panel)
    // asks the frame to show or hide the panel with this object name.
    void panelToggleRequested(const QString& panel);
    // The Move tool's Align / Distribute buttons; `edge` is 0 Top ... 5 Right.
    void alignRequested(int edge);
    void distributeRequested(int edge);
    // A selection tool's `Select and Mask…` button (opens Refine Edge).
    void refineEdgeRequested();

public:
    // Self-test hook: the controller the size field is wired to.
    ToolController* controllerForTest() const { return controller_; }
    // Test hook: the built options page for `id` (stack index = enum value).
    QWidget* pageForTest(ToolId id) const;
    // The Brush Preset picker behind every brush tip button, made on first use.
    BrushPresetPicker* brushPicker();

protected:
    void resizeEvent(QResizeEvent* event) override;

private:
    QWidget* buildPage(ToolId id);
    QWidget* buildPageBody(ToolId id);
    QWidget* buildCombinePage(ToolId id, bool withTolerance);
    QWidget* buildSelectionPage(ToolId id);
    QWidget* buildWandPage(ToolId id);
    // The `Select and Mask…` button the selection tools share.
    void addRefineEdgeButton(QHBoxLayout* layout, QWidget* page);
    QWidget* buildCropPage(ToolId id);
    QWidget* buildAnnotationPage(ToolId id);
    QWidget* buildPaintPage(ToolId id);
    QWidget* buildHealingPage(ToolId id);
    QWidget* buildPatchPage(ToolId id);
    QWidget* buildContentAwareMovePage(ToolId id);
    QWidget* buildCountPage(ToolId id);
    // options_bar_paint.cpp: Red Eye, Color Replacement, and Mixer Brush, and
    // the brush tip button every brush bar shares.
    QWidget* buildRedEyePage(ToolId id);
    QWidget* buildColorReplacementPage(ToolId id);
    QWidget* buildMixerBrushPage(ToolId id);
    void addBrushTipFields(QHBoxLayout* layout, QWidget* page);
    // options_bar_stamp.cpp: Clone Stamp, Pattern Stamp, History Brush, Art
    // History Brush, and Eraser.
    QWidget* buildStampPage(ToolId id);
    QWidget* buildHistoryBrushPage(ToolId id);
    QWidget* buildArtHistoryBrushPage(ToolId id);
    QWidget* buildEraserPage(ToolId id);
    // options_bar_erase.cpp: Background Eraser and Magic Eraser.
    QWidget* buildBackgroundEraserPage(ToolId id);
    QWidget* buildMagicEraserPage(ToolId id);
    // options_bar_fill.cpp: Gradient and Paint Bucket.
    QWidget* buildGradientPage(ToolId id);
    QWidget* buildPaintBucketPage(ToolId id);
    // options_bar_retouch.cpp: Blur, Sharpen, Smudge, Dodge, Burn, and Sponge.
    QWidget* buildRetouchPage(ToolId id);
    QWidget* buildTonePage(ToolId id);
    // options_bar_pen.cpp: Pen and Freeform Pen.
    QWidget* buildPenPage(ToolId id);
    // The shape tools' page and its parts (options_bar_shape.cpp). An update
    // edits the shared ShapeOptions; the syncs re-read them into the page.
    using ShapeUpdate = std::function<void(const std::function<void(ShapeOptions&)>&)>;
    using ShapeSyncs = std::vector<std::function<void(const ShapeOptions&)>>;
    QWidget* buildShapePage(ToolId id);
    QWidget* buildRotateViewPage(ToolId id);
    // options_bar_move.cpp: the Move tool's Align and Distribute buttons.
    QWidget* buildMovePage(ToolId id);
    // options_bar_eyedropper.cpp: the Eyedropper's Sample Size / Sample / ring.
    QWidget* buildEyedropperPage(ToolId id);
    void buildShapeAppearance(QWidget* page, QHBoxLayout* layout, const ShapeUpdate& update,
                              ShapeSyncs& syncs);
    QToolButton* buildShapeGeometryButton(ToolId id, QWidget* page, const ShapeUpdate& update,
                                          ShapeSyncs& syncs);
    QToolButton* buildShapePicker(QWidget* page, const ShapeUpdate& update, ShapeSyncs& syncs);
    QToolButton* buildArrowheadsButton(QWidget* page, const ShapeOptions& initial,
                                       const ShapeUpdate& update);
    // options_bar_type.cpp: the four Type tools.
    QWidget* buildTypePage(ToolId id);
    void addStampPaintFields(QHBoxLayout* layout, QWidget* page);
    NumericField* addPercentField(QHBoxLayout* layout, QWidget* page, const QString& label,
                                  const QString& name, int value,
                                  void (ToolController::*setter)(int));
    static NumericFieldConfig numericConfig(double lo, double hi, double step, int decimals,
                                            const QString& suffix, bool popup,
                                            const QString& name, bool allowFractional = false);
    void addModeButtons(QHBoxLayout* layout, QWidget* page, bool withIntersect);
    void addMagneticFields(QHBoxLayout* layout, QWidget* page);
    QToolButton* toolButton(ToolId id, QWidget* parent);
    // Re-seed the Crop W/H fields from their mode (ratio values, or pixels in
    // `W x H x Resolution` mode).
    void syncCropFields();

    ToolController* controller_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QToolButton* workspaceSwitcher_ = nullptr;
    BrushPresetPicker* brushPicker_ = nullptr;
    std::vector<QToolButton*> alignButtons_;
    std::vector<QToolButton*> distributeButtons_;
    // The Crop tool's user aspect-ratio presets (label, width:height pair).
    std::vector<std::pair<QString, QPointF>> cropPresets_;
    NumericField* cropWidthField_ = nullptr;
    NumericField* cropHeightField_ = nullptr;
    // True while the `W x H x Resolution` entry drives the W/H fields as pixels.
    bool cropResolutionMode_ = false;
    // The W/H ratio values shown in ratio mode (1,1 for the free `Ratio` entry).
    double cropRatioW_ = 1.0;
    double cropRatioH_ = 1.0;
};

// Built-in pattern `index` as an icon, for the pattern pickers.
QIcon patternIcon(int index);

// options_bar_type.cpp, shared with the Character and Paragraph panels: the
// Type tools' size menu (px; any value 1-1296 can be typed), the paragraph
// alignment glyph (`justification` 0 left, 1 right, 2 centre; top / bottom /
// centre for vertical type), and the text colour swatch.
QList<int> typeSizes();
QIcon typeAlignIcon(int justification, bool vertical, const QColor& color);
QIcon typeSwatchIcon(const QColor& color);

} // namespace pictura
