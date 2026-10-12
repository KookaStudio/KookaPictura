#pragma once

// CS6's Liquify workspace: the tool column on the left, the preview in the
// middle, and on the right OK / Cancel / Load Last Mesh / Advanced Mode over
// the tool, reconstruct, mask and view options. Basic mode shows CS6's short
// column (Forward Warp, Reconstruct, Pucker, Bloat, Push Left, Hand, Zoom)
// with Size and Pressure; Advanced adds Twirl, Mirror, Turbulence and the
// Freeze/Thaw masks with Density, Rate and Turbulent Jitter. The mesh lives in
// a Rust session; OK keeps it and fills the `liquify` slot with its id.
// ponytail: no Save/Load Mesh files (CS6's format is closed), no mask from a
// selection/channel, no backdrop or mesh overlay, and Reconstruct's amount is
// applied on OK rather than previewed live.

#include <QtCore/QList>
#include <QtWidgets/QDialog>

class QButtonGroup;
class QCheckBox;
class QComboBox;
class QGroupBox;
class QLabel;
class QPushButton;
class QSpinBox;

namespace pictura {

class LiquifyCanvas;
class PictureView;

class LiquifyDialog : public QDialog {
    Q_OBJECT

public:
    // Tool ids; the first ten index the Rust session's tools.
    enum Tool {
        ForwardWarp,
        Reconstruct,
        Twirl,
        Pucker,
        Bloat,
        PushLeft,
        Mirror,
        Turbulence,
        Freeze,
        Thaw,
        Hand,
        Zoom,
    };

    // `documentName` titles the dialog, as CS6's "Liquify (name @ zoom)".
    LiquifyDialog(PictureView* view, const QString& documentName, QWidget* parent = nullptr);
    ~LiquifyDialog() override;

    // Whether the session opened over the active layer.
    bool ready() const { return ready_; }

    // Replace the distortion with kept mesh `id` (Last Filter Settings).
    bool loadMesh(int id);

    bool advanced() const;
    void setAdvanced(bool on);
    int tool() const;
    void selectTool(int tool);
    LiquifyCanvas* canvas() const { return canvas_; }
    // The kept mesh's id after OK; 0 when nothing was distorted.
    int meshId() const { return meshId_; }

    void accept() override;
    void reject() override;

    // Run the dialog modally. On OK with a distortion, `out` holds the mesh
    // id; an undistorted OK or Cancel returns false.
    static bool get(PictureView* view, const QString& documentName, const QList<double>& initial,
                    QList<double>* out, QWidget* parent);

private:
    QSpinBox* addSliderRow(QWidget* box, const QString& label, const QString& name, int minimum,
                           int maximum, int value, int sliderMaximum = -1);
    void pushBrush(bool reverse);
    void toolChanged();
    void resizeBrush(bool larger);
    void showMaskChanged();
    void reconstructWithAmount();
    void updateZoomLabel();
    void finish(bool keep);

    PictureView* view_ = nullptr;
    bool ready_ = false;
    bool finished_ = false;
    int meshId_ = 0;

    LiquifyCanvas* canvas_ = nullptr;
    QButtonGroup* tools_ = nullptr;
    QCheckBox* advanced_ = nullptr;
    QPushButton* loadLast_ = nullptr;
    QSpinBox* size_ = nullptr;
    QSpinBox* density_ = nullptr;
    QSpinBox* pressure_ = nullptr;
    QSpinBox* rate_ = nullptr;
    QSpinBox* jitter_ = nullptr;
    QCheckBox* stylus_ = nullptr;
    QPushButton* reconstruct_ = nullptr;
    QGroupBox* maskBox_ = nullptr;
    QGroupBox* viewBox_ = nullptr;
    QCheckBox* showMask_ = nullptr;
    QComboBox* maskColor_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    QList<QWidget*> advancedOnly_;
    QString documentName_;
};

} // namespace pictura
