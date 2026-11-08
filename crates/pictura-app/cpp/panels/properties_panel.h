#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include "api/layers_api.h"

#include <functional>
#include <utility>

class QComboBox;
class QFormLayout;
class QLabel;
class QMenu;
class QSpinBox;
class QStackedWidget;
class QTimer;
class QToolButton;
class QVBoxLayout;

namespace pictura {

class AdjustmentControls;
class NumericField;
class PictureView;
class PresetCombo;

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
// ponytail: no mask page, Previous State toggle, or Auto-Select menu items;
// the controls an engine descriptor omits are absent.
// The Canvas section has no X/Y, orientation toggle, or Fill menu, and the
// resolution is read-only.
class CommandRegistry;

class PropertiesPanel : public QWidget {
    Q_OBJECT

public:
    explicit PropertiesPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();
    // Commit a pending live edit before a footer command changes the layer.
    void flushPending() { commit(); }
    void setCommandRegistry(CommandRegistry* registry) { registry_ = registry; }
    QString currentPath() const { return path_; }

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
    // The Preset menu, for the Qt Test.
    PresetCombo* presetComboForTest() const { return preset_; }
    bool canSavePresetForTest() const;
    // Run a Properties panel-menu entry (`savePreset`, `loadPreset`).
    void performPanelMenuAction(const QString& id);
    // Enable / disable the panel-menu entries as the active layer changes.
    void updatePanelMenu(QMenu* menu);
    // Replace the modal name / pick prompts in a test.
    void setPresetNameProviderForTest(std::function<QString()> provider)
    {
        presetNameProvider_ = std::move(provider);
    }
    void setPresetPickProviderForTest(std::function<QString(const QStringList&)> provider)
    {
        presetPickProvider_ = std::move(provider);
    }

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
    // Apply a preset block as one "Modify … Layer" state.
    void applyPresetBlock(const QByteArray& block);
    void savePreset();
    void loadPreset();
    // Shrink the hidden pages so the stack sizes to the current one.
    void setPage(int index);
    void buildCanvas(QVBoxLayout* layout);
    void refreshCanvas();
    // W or H was edited: resize the canvas about its centre.
    void applyCanvasSize();
    // Write one parameter live, then (re)arm the commit.
    void push(const QString& key, double value);
    // Apply the coalesced preview (latest value or curve) and re-arm the commit.
    void applyPendingPreview();
    // Apply a pending preview immediately, before a commit.
    void flushPendingPreview();
    void edited();
    void commit();

    PictureView* view_ = nullptr;
    QString path_;
    QString title_;
    QString kind_;
    bool dirty_ = false;
    std::function<QString()> presetNameProvider_;
    std::function<QString(const QStringList&)> presetPickProvider_;

    QLabel* header_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QTimer* commitTimer_ = nullptr;
    QTimer* previewTimer_ = nullptr;
    QString pendingKey_;
    double pendingValue_ = 0.0;
    bool pendingValueValid_ = false;
    int pendingCurveChannel_ = -1;
    QString pendingCurvePoints_;
    bool pendingCurveValid_ = false;

    AdjustmentControls* controls_ = nullptr;
    PresetCombo* preset_ = nullptr;
    QWidget* footer_ = nullptr;
    QToolButton* clip_ = nullptr;
    QToolButton* visible_ = nullptr;
    QToolButton* remove_ = nullptr;

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
    CommandRegistry* registry_ = nullptr;
};

} // namespace pictura
