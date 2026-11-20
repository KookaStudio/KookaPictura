#pragma once

#include <QtCore/QPointF>
#include <QtCore/QVector>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include <vector>

class QCheckBox;
class QComboBox;
class QLabel;
class QSlider;
class QSpinBox;

namespace pictura {

class ImageView;
class PictureView;
class ToolController;

// One refinement-brush dab recorded by the dialog, in document pixels.
struct RefineBrushDab {
    double x = 0.0;
    double y = 0.0;
    double radius = 0.0;
    bool erase = false;
};

// Select > Refine Edge (SEL-003): improve a selection's edge quality. Given a
// layer path it is the mask menu's Refine Mask instead: it refines that
// layer's mask and preselects Layer Mask output. The
// dialog shows a live proxy of the refined coverage mask; OK applies the
// settings to the active selection once and writes the result to the chosen
// Output To target. Non-modal, presented through `runDialog`.
// The refinement brush is armed on the document canvas while the dialog runs:
// a Refine Edge Brush that expands the local band, Restore Original (Alt, or a
// latched Shift+E) that reverts the input under the stroke, and Show Edge (J)
// for the band overlay. Dab radius is half the size (a diameter). The view-mode
// preview shortcuts F/X/P and animated ants remain a ceiling.
class RefineEdgeDialog : public QDialog {
    Q_OBJECT

public:
    explicit RefineEdgeDialog(PictureView* view, QWidget* parent = nullptr,
                              const QString& maskPath = QString());
    ~RefineEdgeDialog() override;
    void done(int result) override;

    // The layer whose mask this refines; empty for Refine Edge.
    QString maskPath() const { return maskPath_; }

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

    // The brush size in document pixels (a diameter) and the recorded strokes.
    int brushSize() const;
    QVector<RefineBrushDab> dabsForTest() const { return dabs_; }
    std::vector<double> dabValues() const;

    // The preview's mask, as shown (for the Qt Test).
    QImage previewForTest() const { return preview_; }

private:
    void refreshPreview();
    void refreshEnabled();
    void armBrush();
    void disarmBrush();
    void onCanvasBrush(int phase, const QPointF& imagePos, Qt::KeyboardModifiers mods);
    void addDab(const QPointF& imagePos, Qt::KeyboardModifiers mods);
    void adjustBrush(int direction);
    void refreshRing();
    bool restoreActive(Qt::KeyboardModifiers mods) const;

    PictureView* view_ = nullptr;
    QString maskPath_;
    ToolController* tools_ = nullptr;
    ImageView* canvas_ = nullptr;
    QVector<RefineBrushDab> dabs_;
    QPointF lastDabPos_;
    QPointF lastHoverPos_;
    bool hasHover_ = false;
    bool strokeActive_ = false;

    QComboBox* viewMode_ = nullptr;
    QCheckBox* showRadius_ = nullptr;
    QCheckBox* showOriginal_ = nullptr;
    QSpinBox* brushSize_ = nullptr;
    QCheckBox* brushRestore_ = nullptr;
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
