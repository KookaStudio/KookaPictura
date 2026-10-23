#pragma once

// CS6's Adaptive Wide Angle workspace: the tool column (Constraint,
// Polygon Constraint, Move, Hand, Zoom) on the left under a help line that
// names the control under the pointer; the preview with its zoom row and the
// Camera Model / Lens Model lines; and on the right OK / Cancel, Correction,
// Scale, Focal Length, Crop Factor, As Shot, the Detail view, and Preview /
// Show Constraints / Show Mesh. The correction and constraints live in a
// Rust session; OK fills the `adaptive-wide-angle` slots, the constraints
// kept by id.
// ponytail: no Auto / lens-profile database, no Panorama, no rotation
// handles, no Save/Load Constraints, and the Detail view does not draw the
// constraints.

#include <QtCore/QList>
#include <QtWidgets/QDialog>

class QButtonGroup;
class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QSpinBox;

namespace pictura {

class AwaCanvas;
class AwaDetail;
class PictureView;

class AdaptiveWideAngleDialog : public QDialog {
    Q_OBJECT

public:
    enum Tool { Constraint, Polygon, Move, Hand, Zoom };

    // `documentName` titles the dialog, as CS6's "Adaptive Wide Angle
    // (name @ zoom)".
    AdaptiveWideAngleDialog(PictureView* view, const QString& documentName,
                            QWidget* parent = nullptr);
    ~AdaptiveWideAngleDialog() override;

    // Whether the session opened over the active layer.
    bool ready() const { return ready_; }
    // Restore committed slots (Last Filter Settings).
    bool load(const QList<double>& values);

    int tool() const;
    void selectTool(int tool);
    AwaCanvas* canvas() const { return canvas_; }
    AwaDetail* detail() const { return detail_; }
    // The kind's slots after OK; empty otherwise.
    const QList<double>& committed() const { return committed_; }

    void accept() override;
    void reject() override;

    // Run the dialog modally; on OK `out` holds the kind's slots.
    static bool get(PictureView* view, const QString& documentName, const QList<double>& initial,
                    QList<double>* out, QWidget* parent);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void pushLens();
    // Re-range Focal Length (field and slider) for the current Crop Factor.
    void updateFocalRange();
    void asShotToggled(bool on);
    void correctionChanged();
    void updateZoomLabel();
    void finish(bool keep);

    PictureView* view_ = nullptr;
    bool ready_ = false;
    bool finished_ = false;
    QList<double> committed_;
    QList<double> exifLens_;

    AwaCanvas* canvas_ = nullptr;
    AwaDetail* detail_ = nullptr;
    QButtonGroup* tools_ = nullptr;
    QLabel* help_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    QComboBox* correction_ = nullptr;
    QSpinBox* scale_ = nullptr;
    QDoubleSpinBox* focal_ = nullptr;
    QDoubleSpinBox* crop_ = nullptr;
    QCheckBox* asShot_ = nullptr;
    QCheckBox* preview_ = nullptr;
    QCheckBox* showConstraints_ = nullptr;
    QCheckBox* showMesh_ = nullptr;
    QString documentName_;
};

} // namespace pictura
