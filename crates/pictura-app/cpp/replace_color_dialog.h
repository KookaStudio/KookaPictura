#pragma once

#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

class QCheckBox;
class QLabel;
class QRadioButton;
class QSlider;
class QSpinBox;
class QToolButton;

namespace pictura {

class ColorSwatchWidget;
class ImageView;
class PictureView;
class ToolController;

// Image > Adjustments > Replace Color (#164): pick one or more sample colours
// from the canvas, preview an HSL shift feathered by how close each pixel is
// to a sample, and commit it as one "Replace Color" state. It opens on the
// foreground colour as its sample, as CS6 does, so the selection thumbnail and
// the sliders work before the first click; the Result swatch opens the colour
// picker and sets the shift that reaches the picked colour. Non-modal, so its
// eyedropper can reach the canvas. Ported from photorust's ReplaceColorDialog.
class ReplaceColorDialog : public QDialog {
    Q_OBJECT

public:
    ReplaceColorDialog(PictureView* view, ToolController* tools, ImageView* canvas,
                       QWidget* parent = nullptr);
    ~ReplaceColorDialog() override;

    PictureView* view() const { return view_; }
    // Take the composite colour at document `imagePos` (the canvas sampler's entry).
    void sampleAt(const QPointF& imagePos);
    QString samplesString() const;
    int sampleCount() const { return int(samples_.size()); }
    // The selection thumbnail as shown (for the Qt Test).
    QImage maskForTest() const { return mask_; }
    // The Result swatch's colour, and the shift that reaches `result` (as
    // picking it in the swatch's colour picker does).
    QColor resultForTest() const;
    void pickResultForTest(const QColor& result) { pickResult(result); }

    void accept() override;
    void reject() override;

protected:
    void hideEvent(QHideEvent* event) override;

private:
    struct Sample {
        QPoint pos;
        QColor color;
    };
    enum class PickMode { Replace, Add, Subtract };

    void buildUi();
    void refreshMask();
    void refreshSwatches();
    void applyChange(bool maskDirty);
    void applyPreview();
    void cancelPreview();
    void refreshSampler();
    // Set Hue / Saturation / Lightness so the sampled colour becomes `result`.
    void pickResult(const QColor& result);

    PictureView* view_ = nullptr;
    ToolController* tools_ = nullptr;
    ImageView* canvas_ = nullptr;
    QVector<Sample> samples_;
    PickMode mode_ = PickMode::Replace;
    bool previewing_ = false;
    QImage mask_;

    QToolButton* sampleButton_ = nullptr;
    QToolButton* addButton_ = nullptr;
    QToolButton* subtractButton_ = nullptr;
    QSpinBox* fuzziness_ = nullptr;
    QSlider* fuzzinessSlider_ = nullptr;
    QCheckBox* localized_ = nullptr;
    ColorSwatchWidget* colorSwatch_ = nullptr;
    ColorSwatchWidget* resultSwatch_ = nullptr;
    QLabel* maskLabel_ = nullptr;
    QRadioButton* selectionButton_ = nullptr;
    QRadioButton* imageButton_ = nullptr;
    QSlider* hueSlider_ = nullptr;
    QSpinBox* hueSpin_ = nullptr;
    QSlider* saturationSlider_ = nullptr;
    QSpinBox* saturationSpin_ = nullptr;
    QSlider* lightnessSlider_ = nullptr;
    QSpinBox* lightnessSpin_ = nullptr;
    QCheckBox* preview_ = nullptr;
};

} // namespace pictura
