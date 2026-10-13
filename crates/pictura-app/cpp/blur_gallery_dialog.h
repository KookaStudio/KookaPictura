#pragma once

// CS6's Blur Gallery workspace as the Field Blur, Iris Blur, and Tilt-Shift
// entries open it: the picture with its on-image pins beside a Blur Tools
// panel and the Effects / Motion Effects / Noise tabbed panel. Each of the
// three effects keeps its own pin list here and can be switched on with its
// row's checkbox; the selected effect (its row's name, or checking it) is the
// one the canvas edits, and its arrow expands its controls. The command's
// kind opens with that effect on and selected. The preview re-renders the
// active layer through `filter_proxy_layer` over the rest of the document;
// the document is untouched until OK.

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

    // The kind OK commits: the one effect switched on (`field-blur`,
    // `iris-blur`, `tilt-shift`), `blur-gallery` for several, or empty for
    // none.
    QString kind() const;
    // `kind()`'s slots. A single effect: the count, then per pin `(x, y,
    // blur)` for Field, `(x, y, radius x, radius y, angle, squareness,
    // feather, blur)` for Iris, or `(x, y, angle, focus above, focus below,
    // fade above, fade below, distortion, symmetric, blur)` for Tilt-Shift.
    // `blur-gallery`: each effect's count and records in that order (0 for
    // one switched off). Then the effects tail (bokeh
    // enabled/light/color/range, noise
    // enabled/amount/size/roughness/color/highlights/kind/seed).
    QList<double> values() const;
    // Slots in the opening command's layout; ignored unless whole, so empty
    // or malformed input keeps the defaults.
    void setValues(const QList<double>& values);

    BlurGalleryCanvas* canvas() const { return canvas_; }
    BlurGalleryMode selectedEffect() const { return active_; }
    bool effectEnabled(BlurGalleryMode mode) const { return effects_[int(mode)].enabled; }

    // Run the workspace modally; on OK `*kind` and `*out` are what to commit.
    // False for Cancel, or OK with every effect off.
    static bool get(PictureView* view, const FilterCommandSpec& spec,
                    const QList<double>& initial, QList<double>* out, QString* kind,
                    QWidget* parent);

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
    // One Blur Tools row: arrow, name, checkbox. `effect` is null for the
    // unimplemented rows, which list disabled.
    struct Effect;
    QWidget* effectRow(QWidget* parent, const QString& label, const QString& name,
                       Effect* effect);
    QWidget* buildEffectControls(QWidget* parent, BlurGalleryMode mode, const QString& name);
    Field addField(QVBoxLayout* layout, QWidget* parent, const QString& label, const QString& name,
                   double minimum, double maximum, double value, int decimals,
                   const QString& suffix);
    QDoubleSpinBox* makeValueBox(QWidget* parent, const QString& name, double minimum,
                                 double maximum, double value);

    // Make `mode` the edited effect, switching it on (with a default pin
    // when it has none) and expanding its row.
    void selectEffect(BlurGalleryMode mode);
    void setEffectEnabled(BlurGalleryMode mode, bool on);
    void setExpanded(BlurGalleryMode mode, bool on);
    QList<BlurGalleryPin>& pins() { return effects_[int(active_)].pins; }
    // `values()` with every pin's amount multiplied by `scale`.
    QList<double> slotList(double scale) const;
    void appendPins(QList<double>* out, BlurGalleryMode mode, double scale) const;
    // Reads `count` pins of `mode` from `values` at `at`; false if short.
    bool readPins(const QList<double>& values, int* at, BlurGalleryMode mode,
                  QList<BlurGalleryPin>* pins) const;
    void readTail(const QList<double>& values, int tail);
    BlurGalleryPin defaultPin(BlurGalleryMode mode) const;
    void selectPin(int index);
    void pinsEdited();
    void deleteSelectedPin();
    void setSelectedBlur(double blur);
    void setSelectedDistortion(double distortion, bool symmetric);
    void syncControls();
    void effectsEdited();
    void scheduleRender();
    // A drag or held slider: preview the draft size.
    bool interacting() const;
    void render();

    PictureView* view_ = nullptr;
    const FilterCommandSpec spec_;
    struct Effect {
        QList<BlurGalleryPin> pins;
        int selected = 0;
        bool enabled = false;
        QCheckBox* check = nullptr;
        QToolButton* arrow = nullptr;
        QToolButton* title = nullptr;
        QWidget* controls = nullptr;
        QDoubleSpinBox* blurSpin = nullptr;
        QSlider* blurSlider = nullptr;
    };
    Effect effects_[3];
    BlurGalleryMode active_ = BlurGalleryMode::Field;
    BlurGalleryCanvas* canvas_ = nullptr;
    QLabel* hint_ = nullptr;

    // Tilt-Shift's row.
    QDoubleSpinBox* distortionSpin_ = nullptr;
    QSlider* distortionSlider_ = nullptr;
    QCheckBox* symmetric_ = nullptr;

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
    // The document as shown with Preview off.
    QImage proxy_;
    // The document with the active layer hidden, at the full and the draft
    // edge; the preview draws the filtered layer over one of them, so the
    // other layers are never blurred with it (the commit filters only the
    // active layer). The draft is null when the full proxy is already small.
    QImage base_;
    QImage draftBase_;
    int proxyEdge_ = 0;
    int draftEdge_ = 0;
    bool renderPending_ = false;
};

} // namespace pictura
