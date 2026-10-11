#pragma once

#include <QtGui/QImage>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QLabel;
class QSlider;
class QSpinBox;

namespace pictura {

class PictureView;

// Select > Refine Edge (SEL-003): improve a selection's edge quality. The
// dialog shows a live proxy of the refined coverage mask; OK applies the
// settings to the active selection once and writes the result to the chosen
// Output To target. Non-modal, presented through `runDialog`.
// ponytail: the interactive Refine Radius / Erase Refinements brushes, the
// on-canvas refinement strokes, and the view-mode shortcuts F/X/P/J are not
// modelled; the View Mode combo and the Show Radius / Show Original toggles are
// the preview surface.
class RefineEdgeDialog : public QDialog {
    Q_OBJECT

public:
    explicit RefineEdgeDialog(PictureView* view, QWidget* parent = nullptr);

    // The engine parameters, read on OK.
    bool smartRadius() const;
    int radius() const;
    int smooth() const;
    int feather() const;
    int contrast() const;
    int shiftEdge() const;
    bool decontaminate() const;
    int amount() const;
    // 0 Selection, 1 Layer Mask, 2 New Layer, 3 New Layer with Mask.
    int outputTo() const;

    // The preview's mask, as shown (for the Qt Test).
    QImage previewForTest() const { return preview_; }

private:
    void refreshPreview();
    void refreshEnabled();

    PictureView* view_ = nullptr;
    QComboBox* viewMode_ = nullptr;
    QCheckBox* showRadius_ = nullptr;
    QCheckBox* showOriginal_ = nullptr;
    QCheckBox* smartRadius_ = nullptr;
    QSpinBox* radius_ = nullptr;
    QSlider* radiusSlider_ = nullptr;
    QSpinBox* smooth_ = nullptr;
    QSlider* smoothSlider_ = nullptr;
    QSpinBox* feather_ = nullptr;
    QSlider* featherSlider_ = nullptr;
    QSpinBox* contrast_ = nullptr;
    QSlider* contrastSlider_ = nullptr;
    QSpinBox* shift_ = nullptr;
    QSlider* shiftSlider_ = nullptr;
    QCheckBox* decontaminate_ = nullptr;
    QSpinBox* amount_ = nullptr;
    QSlider* amountSlider_ = nullptr;
    QComboBox* output_ = nullptr;
    QLabel* previewLabel_ = nullptr;
    QImage preview_;
};

} // namespace pictura
