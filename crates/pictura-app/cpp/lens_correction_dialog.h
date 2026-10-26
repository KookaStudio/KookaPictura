#pragma once

// CS6's Lens Correction dialog: the tool column (Remove Distortion,
// Straighten, Move Grid, Hand, Zoom) under a help line that names the active
// tool or the control under the pointer; the preview with its zoom box, the
// Camera Model / Lens Model / Camera Settings lines, and Preview / Show Grid
// / grid Color / Size; and on the right OK / Cancel over the Auto Correction
// and Custom tabs. Every setting is one of the `lens-correction` slots; the
// Rust session renders them.

#include <QtCore/QList>
#include <QtWidgets/QDialog>

class QButtonGroup;
class QCheckBox;
class QLabel;
class QSpinBox;
class QTabWidget;

namespace pictura {

class ColorSwatchWidget;
class LensAutoPanel;
class LensCorrectionCanvas;
class LensCustomPanel;
class PictureView;

class LensCorrectionDialog : public QDialog {
    Q_OBJECT

public:
    enum Tool { RemoveDistortion, Straighten, MoveGrid, Hand, Zoom };

    // CS6 titles the dialog "Lens Correction (zoom)", without the document.
    explicit LensCorrectionDialog(PictureView* view, QWidget* parent = nullptr);
    ~LensCorrectionDialog() override;

    // Whether the session opened over the active layer.
    bool ready() const { return ready_; }
    // Restore committed slots (Last Filter Settings). False, leaving the
    // dialog as it was, when they no longer apply (a profile not installed).
    bool load(const QList<double>& values);

    // The current slots, Auto Correction's then Custom's.
    QList<double> values() const;
    int tool() const;
    void selectTool(int tool);
    LensCorrectionCanvas* canvas() const { return canvas_; }
    LensAutoPanel* autoPanel() const { return auto_; }
    LensCustomPanel* customPanel() const { return custom_; }
    QString helpText() const;
    // The kind's slots after OK; empty otherwise.
    const QList<double>& committed() const { return committed_; }

    void accept() override;
    void reject() override;

    // Run the dialog modally; on OK `out` holds the kind's slots.
    static bool get(PictureView* view, const QList<double>& initial, QList<double>* out,
                    QWidget* parent);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void pushValues();
    void showToolHelp();
    void updateZoomLabel();
    void pickGridColor();
    void finish(bool keep);

    PictureView* view_ = nullptr;
    bool ready_ = false;
    bool finished_ = false;
    QList<double> committed_;

    LensCorrectionCanvas* canvas_ = nullptr;
    LensAutoPanel* auto_ = nullptr;
    LensCustomPanel* custom_ = nullptr;
    QTabWidget* tabs_ = nullptr;
    QButtonGroup* tools_ = nullptr;
    QLabel* help_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    QCheckBox* preview_ = nullptr;
    QCheckBox* showGrid_ = nullptr;
    ColorSwatchWidget* gridColor_ = nullptr;
    QSpinBox* gridSize_ = nullptr;
};

} // namespace pictura
