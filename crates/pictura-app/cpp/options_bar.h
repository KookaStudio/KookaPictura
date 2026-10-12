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
    // Show the Free Transform page (appended after the per-tool pages); the
    // frame's `transformChanged` handler switches between the two.
    void showTransform();
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
    // Test hooks: the Free Transform page and the stack's current page.
    QWidget* transformPageForTest() const { return transformPage_; }
    QWidget* currentPageForTest() const;
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
    QWidget* buildPerspectiveCropPage(ToolId id);
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
    // options_bar_transform.cpp: the session-keyed Free Transform page and its
    // refresh from the live session.
    QWidget* buildTransformPage();
    void refreshTransformPage();
    // Apply the current page's size policy so only it sets the bar's minimum.
    void updateStackPolicies();
    void addStampPaintFields(QHBoxLayout* layout, QWidget* page);
    NumericField* addPercentField(QHBoxLayout* layout, QWidget* page, const QString& label,
                                  const QString& name, int value,
                                  std::function<void(int)> setter);
    static NumericFieldConfig numericConfig(double lo, double hi, double step, int decimals,
                                            const QString& suffix, bool popup,
                                            const QString& name, bool allowFractional = false);
    void addModeButtons(QHBoxLayout* layout, QWidget* page, bool withIntersect);
    void addMagneticFields(QHBoxLayout* layout, QWidget* page);
    QToolButton* toolButton(ToolId id, QWidget* parent);
    // Re-seed the Crop W/H fields from their mode (ratio values, or pixels in
    // `W x H x Resolution` mode).
    void syncCropFields();
    // Show/hide the Perspective Crop Apply/Cancel buttons from the quad state.
    void syncPerspectiveButtons();

    ToolController* controller_ = nullptr;
    // The family option groups, resolved once from the handlers' ports; the
    // pages write these directly (the group setters clamp and notify).
    PaintOptions* paint_ = nullptr;
    SelectionOptions* selection_ = nullptr;
    CropOptions* crop_ = nullptr;
    MoveGroup* move_ = nullptr;
    EyedropperGroup* eyedropper_ = nullptr;
    HealingGroup* healing_ = nullptr;
    PatchGroup* patch_ = nullptr;
    ContentAwareMoveGroup* contentAware_ = nullptr;
    RedEyeGroup* redEye_ = nullptr;
    ColorReplaceGroup* colorReplace_ = nullptr;
    MixerGroup* mixer_ = nullptr;
    StampGroup* stamp_ = nullptr;
    EraserGroup* eraser_ = nullptr;
    BackgroundEraseGroup* backgroundErase_ = nullptr;
    MagicEraseGroup* magicErase_ = nullptr;
    ArtHistoryGroup* artHistory_ = nullptr;
    GradientGroup* gradient_ = nullptr;
    BucketGroup* bucket_ = nullptr;
    PenGroup* pen_ = nullptr;
    ShapeGroup* shape_ = nullptr;
    TypeGroup* type_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QToolButton* workspaceSwitcher_ = nullptr;
    BrushPresetPicker* brushPicker_ = nullptr;
    // The Free Transform page, appended after the per-tool pages.
    QWidget* transformPage_ = nullptr;
    // Transform page state: relative X/Y display and the W/H aspect link.
    bool transformRelative_ = false;
    bool transformLink_ = true;
    std::vector<QToolButton*> alignButtons_;
    std::vector<QToolButton*> distributeButtons_;
    // The Crop tool's user aspect-ratio presets (label, width:height pair).
    std::vector<std::pair<QString, QPointF>> cropPresets_;
    NumericField* cropWidthField_ = nullptr;
    NumericField* cropHeightField_ = nullptr;
    // The Perspective Crop Apply/Cancel buttons (visible only when the quad is
    // complete).
    QToolButton* perspectiveApply_ = nullptr;
    QToolButton* perspectiveCancel_ = nullptr;
    // True while the `W x H x Resolution` entry drives the W/H fields as pixels.
    bool cropResolutionMode_ = false;
    // The W/H ratio values shown in ratio mode (1,1 for the free `Ratio` entry).
    double cropRatioW_ = 0.0;
    double cropRatioH_ = 0.0;
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
