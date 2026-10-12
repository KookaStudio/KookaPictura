#pragma once

// CS6's Lighting Effects workspace: an options bar (Presets, the three
// add-light buttons, Reset, Preview, Cancel, OK) over the picture with its
// on-canvas light controls, beside a Properties panel (the selected light's
// type, Color, Intensity, Hotspot; the rig's Colorize, Exposure, Gloss,
// Metallic, Ambience, Texture, Height) and a Lights panel (one row per light
// with its eye, and a trash button). The rig is a list of lights, so it does
// not fit the generic FilterPreviewDialog. The workspace previews on its own
// canvas; the document is untouched until OK.

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include "filter_commands.h"
#include "lighting_rig.h"

class QCheckBox;
class QComboBox;
class QLabel;
class QListWidget;
class QPushButton;
class QSlider;
class QSpinBox;
class QToolButton;

namespace pictura {

class ColorSwatchWidget;
class LightingCanvas;
class PictureView;

class LightingEffectsDialog : public QDialog {
    Q_OBJECT

public:
    LightingEffectsDialog(PictureView* view, const FilterCommandSpec& spec,
                          QWidget* parent = nullptr);

    // The `lighting-effects` slots: the rig, then each light.
    QList<double> values() const;
    // Ignored unless `values` is a whole rig.
    void setValues(const QList<double>& values);

    const LightingRig& rig() const { return rig_; }
    int selectedLight() const;
    void selectLight(int index);
    // Add a default light of `kind` and select it; false at the 16-light cap.
    bool addLight(LightKind kind);
    // Remove the selected light; false when it is the last one.
    bool deleteSelectedLight();
    // Load a style from the Presets menu by name.
    void applyPreset(const QString& name);

    LightingCanvas* canvas() const { return canvas_; }
    // The workspace's picture, rendered up to date.
    QImage previewImage();

    // Run the workspace modally, committing `out` on OK.
    static bool get(PictureView* view, const FilterCommandSpec& spec,
                    const QList<double>& initial, QList<double>* out, QWidget* parent);

private:
    struct Field {
        QLabel* label = nullptr;
        QSpinBox* spin = nullptr;
        QSlider* slider = nullptr;
        void setValue(double value);
        void setEnabled(bool on);
    };

    QWidget* buildOptionsBar();
    QWidget* buildPropertiesPanel();
    QWidget* buildLightsPanel();
    Field makeField(QWidget* parent, const QString& label, const QString& name, int minimum,
                    int maximum, void (LightingEffectsDialog::*apply)(int));
    ColorSwatchWidget* makeSwatch(QWidget* parent, const QString& name, const QString& title,
                                  bool rigColor);

    void setIntensity(int value);
    void setHotspot(int value);
    void setExposure(int value);
    void setGloss(int value);
    void setMetallic(int value);
    void setAmbience(int value);
    void setHeight(int value);

    LightSpec* current();
    void syncControls();
    void rebuildLightList();
    // An edit from the panels or the canvas: the preset becomes Custom and the
    // picture re-renders.
    void rigEdited();
    void scheduleRender();
    void render();

    PictureView* view_ = nullptr;
    const FilterCommandSpec spec_;
    LightingRig rig_;
    LightingCanvas* canvas_ = nullptr;

    QComboBox* presets_ = nullptr;
    QToolButton* addButtons_[3] = {nullptr, nullptr, nullptr};
    QCheckBox* preview_ = nullptr;

    QComboBox* type_ = nullptr;
    ColorSwatchWidget* color_ = nullptr;
    ColorSwatchWidget* colorize_ = nullptr;
    Field intensity_;
    Field hotspot_;
    Field exposure_;
    Field gloss_;
    Field metallic_;
    Field ambience_;
    QComboBox* texture_ = nullptr;
    Field height_;

    QListWidget* lights_ = nullptr;
    QToolButton* trash_ = nullptr;

    QImage proxy_;
    QByteArray proxyRgba_;
    bool renderPending_ = false;
};

} // namespace pictura
