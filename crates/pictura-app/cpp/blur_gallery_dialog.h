#pragma once

// CS6's Blur Gallery workspace as the Field Blur entry opens it: the picture
// with its on-image pins beside a Blur Tools panel and the Effects / Motion
// Effects / Noise tabbed panel. The pin list lives here; the canvas edits it
// and the preview re-renders through `filter_thumbnail`. Iris Blur,
// Tilt-Shift, Path Blur, and Spin Blur list as disabled rows until their
// kernels land. The document is untouched until OK.

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include "blur_gallery_canvas.h"
#include "filter_commands.h"

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QSlider;
class QToolButton;
class QVBoxLayout;

namespace pictura {

class LightRangeSlider;
class PictureView;

class BlurGalleryDialog : public QDialog {
    Q_OBJECT

public:
    BlurGalleryDialog(PictureView* view, const FilterCommandSpec& spec,
                      QWidget* parent = nullptr);

    // The `field-blur` slots: the count, then `(x, y, blur)` per pin, then the
    // effects tail (bokeh enabled/light/color/range, noise
    // enabled/amount/size/roughness/color/highlights/kind/seed).
    QList<double> values() const;
    // Ignored unless `values` is a whole pin list with its effects tail; empty
    // or malformed input keeps the defaults.
    void setValues(const QList<double>& values);

    BlurGalleryCanvas* canvas() const { return canvas_; }

    // Run the workspace modally, committing `out` on OK.
    static bool get(PictureView* view, const FilterCommandSpec& spec,
                    const QList<double>& initial, QList<double>* out, QWidget* parent);

private:
    struct Field {
        QLabel* label = nullptr;
        QDoubleSpinBox* spin = nullptr;
        QSlider* slider = nullptr;
        void setValue(double value);
        void setEnabled(bool on);
    };

    QWidget* buildOptionsBar();
    QWidget* buildBlurToolsPanel();
    QWidget* buildEffectsPanel();
    QWidget* buildBokehTab();
    QWidget* buildMotionTab();
    QWidget* buildNoiseTab();
    QWidget* effectRow(QWidget* parent, const QString& label, const QString& checkName,
                       bool checked, bool enabled, QToolButton** arrow);
    Field addField(QVBoxLayout* layout, QWidget* parent, const QString& label, const QString& name,
                   double minimum, double maximum, double value, int decimals,
                   const QString& suffix);
    QDoubleSpinBox* makeValueBox(QWidget* parent, const QString& name, double minimum,
                                 double maximum, double value);

    void selectPin(int index);
    void pinsEdited();
    void deleteSelectedPin();
    void setSelectedBlur(double blur);
    void syncControls();
    void effectsEdited();
    void scheduleRender();
    void render();

    PictureView* view_ = nullptr;
    const FilterCommandSpec spec_;
    QList<BlurGalleryPin> pins_;
    BlurGalleryCanvas* canvas_ = nullptr;

    QToolButton* fieldArrow_ = nullptr;
    QWidget* fieldControls_ = nullptr;
    QDoubleSpinBox* blurSpin_ = nullptr;
    QSlider* blurSlider_ = nullptr;

    QCheckBox* bokehEnabled_ = nullptr;
    Field bokehLight_;
    Field bokehColor_;
    LightRangeSlider* bokehRange_ = nullptr;
    QDoubleSpinBox* bokehRangeLow_ = nullptr;
    QDoubleSpinBox* bokehRangeHigh_ = nullptr;

    QCheckBox* noiseEnabled_ = nullptr;
    QComboBox* noiseKind_ = nullptr;
    Field noiseAmount_;
    Field noiseSize_;
    Field noiseRoughness_;
    Field noiseColor_;
    Field noiseHighlights_;
    double noiseSeed_ = 1.0;

    QCheckBox* preview_ = nullptr;
    QImage proxy_;
    QByteArray proxyRgba_;
    double proxyScale_ = 1.0;
    bool renderPending_ = false;
};

} // namespace pictura
