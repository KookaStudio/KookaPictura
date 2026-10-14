#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include "api/layers_api.h"


class QComboBox;
class QFormLayout;
class QLabel;
class QSpinBox;
class QStackedWidget;
class QTimer;
class QToolButton;
class QVBoxLayout;

namespace pictura {

class AdjustmentControls;
class NumericField;
class PictureView;

// Window > Properties (PAN-006). For an adjustment layer it builds the
// adjustment's controls from the engine's per-kind descriptor
// (`adjustment_page`), edits live on the canvas, and records one "Modify …
// Layer" state per gesture (a slider release, a field or toggle change, or a
// pause in a curve drag); the footer carries Clip to Layer, Reset, Toggle
// Visibility, and Delete. Any other layer gets a read-only summary (kind, size,
// position, blend, opacity, fill, mask, locks) — a post-CS6 page photorust
// has, ported on request (#70). Below either the layer summary or, with no
// active layer, a "Document" header sits the post-CS6 Canvas section: W/H (a
// centred Canvas Size, proportions optionally linked), the resolution, and the
// Mode / Bits per Channel menus, which route through Image ▸ Mode. With no
// document it reads No Properties.
// Ported from photorust's PropertiesPanel; Kooka's panel column hosts it.
// ponytail: no adjustment Presets menu, mask page, Previous State toggle, or
// Auto-Select menu items; the controls an engine descriptor omits are absent.
// The Canvas section has no X/Y, orientation toggle, or Fill menu, and the
// resolution is read-only.
class PropertiesPanel : public QWidget {
    Q_OBJECT

public:
    explicit PropertiesPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();

    // The header: the adjustment's name, "Layer Properties", "Document", or
    // "No Properties".
    QString messageForTest() const;
    // The control editing `key`, for the Qt Test.
    QWidget* controlForTest(const QString& key) const;
    QString pathForTest() const { return path_; }
    // Commit any pending live edit now (as the commit timer would).
    void commitForTest() { commit(); }
    // The Canvas section's controls, for the Qt Test.
    QSpinBox* canvasWidthForTest() const { return width_; }
    QSpinBox* canvasHeightForTest() const { return height_; }
    QToolButton* canvasLinkForTest() const { return link_; }
    QComboBox* modeForTest() const { return mode_; }
    QComboBox* depthForTest() const { return depth_; }
    QString resolutionForTest() const;
    // Whether the layer page's Mask section is currently shown.
    bool maskSectionVisibleForTest() const;
    // Whether the layer page's Vector Mask section is currently shown.
    bool vectorMaskSectionVisibleForTest() const;

signals:
    // The Canvas section's Mode / Bits per Channel menus; the window runs the
    // Image ▸ Mode conversion (with its dialogs) and refreshes.
    void modeRequested(const QString& mode);
    void depthRequested(int bits);

private:
    // The selected layer's path and its Layers-panel row, or empty / -1.
    QString activePath() const;
    void showNothing();
    void showLayer(const pictura::layers_api::LayerRow& row);
    void showAdjustment(const QStringList& page);
    // Shrink the hidden pages so the stack sizes to the current one.
    void setPage(int index);
    void buildCanvas(QVBoxLayout* layout);
    void refreshCanvas();
    // W or H was edited: resize the canvas about its centre.
    void applyCanvasSize();
    // Write one parameter live, then (re)arm the commit.
    void push(const QString& key, double value);
    void edited();
    void commit();

    PictureView* view_ = nullptr;
    QString path_;
    QString title_;
    bool dirty_ = false;

    QLabel* header_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QTimer* commitTimer_ = nullptr;

    AdjustmentControls* controls_ = nullptr;
    QWidget* footer_ = nullptr;
    QToolButton* clip_ = nullptr;
    QToolButton* visible_ = nullptr;

    // Layer page.
    QFormLayout* info_ = nullptr;
    QWidget* layerPage_ = nullptr;
    // Mask section, shown only when the active layer carries a layer mask.
    QWidget* maskSection_ = nullptr;
    QToolButton* maskEnable_ = nullptr;
    QToolButton* maskDisable_ = nullptr;
    QToolButton* maskLink_ = nullptr;
    QToolButton* maskUnlink_ = nullptr;
    QToolButton* maskDelete_ = nullptr;
    QToolButton* maskApply_ = nullptr;
    NumericField* maskDensity_ = nullptr;
    NumericField* maskFeather_ = nullptr;
    QToolButton* maskInvert_ = nullptr;
    // Vector Mask section, shown only when the active layer carries a vector mask.
    QWidget* vectorMaskSection_ = nullptr;
    QToolButton* vectorMaskEnable_ = nullptr;
    QToolButton* vectorMaskDisable_ = nullptr;
    QToolButton* vectorMaskLink_ = nullptr;
    QToolButton* vectorMaskUnlink_ = nullptr;
    QToolButton* vectorMaskDelete_ = nullptr;
    QToolButton* vectorMaskRasterize_ = nullptr;
    NumericField* vectorMaskDensity_ = nullptr;
    NumericField* vectorMaskFeather_ = nullptr;

    // Canvas section.
    QWidget* canvas_ = nullptr;
    QSpinBox* width_ = nullptr;
    QSpinBox* height_ = nullptr;
    QToolButton* link_ = nullptr;
    QLabel* resolution_ = nullptr;
    QComboBox* mode_ = nullptr;
    QComboBox* depth_ = nullptr;
};

} // namespace pictura
