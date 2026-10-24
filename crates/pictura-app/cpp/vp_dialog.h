#pragma once

// CS6's Vanishing Point workspace: the menu button (Show Edges, Render Grids
// To Photoshop), the tool options (Grid Size, Angle) over a help line, OK /
// Cancel, the tool column (Edit Plane, Create Plane, Hand, Zoom) and the
// preview with its zoom row. The planes live in a Rust session; OK keeps them
// on the document, and with Render Grids on fills the `vanishing-point`
// slot with the kept grid edit's id.
// ponytail: no Marquee, Stamp, Brush, Transform or Eyedropper tool yet, and
// the Extended-only Measure tool, Render Measurements and DXF/3DS export are
// not offered.

#include <QtCore/QList>
#include <QtWidgets/QDialog>

class QAction;
class QButtonGroup;
class QLabel;
class QSpinBox;

namespace pictura {

class PictureView;
class VpCanvas;

class VanishingPointDialog : public QDialog {
    Q_OBJECT

public:
    enum Tool { EditPlane, CreatePlane, Hand, Zoom };

    // `documentName` titles the dialog, as "Vanishing Point (name @ zoom)".
    VanishingPointDialog(PictureView* view, const QString& documentName,
                         QWidget* parent = nullptr);
    ~VanishingPointDialog() override;

    // Whether the session opened over the active layer.
    bool ready() const { return ready_; }
    // Restore a committed slot (Last Filter Settings): the planes the edit
    // was made from, with Render Grids on. False when it is no longer kept.
    bool load(const QList<double>& values);

    int tool() const;
    void selectTool(int tool);
    VpCanvas* canvas() const { return canvas_; }
    QAction* showEdgesAction() const { return showEdges_; }
    QAction* renderGridsAction() const { return renderGrids_; }
    // The kind's slot after OK with something to draw; empty otherwise.
    const QList<double>& committed() const { return committed_; }

    void accept() override;
    void reject() override;

    // Run the dialog modally; true with `out` holding the slot when OK left
    // an edit to commit.
    static bool get(PictureView* view, const QString& documentName, const QList<double>& initial,
                    QList<double>* out, QWidget* parent);

private:
    // Re-read Grid Size and Angle for the active plane.
    void syncOptions();
    void updateZoomLabel();
    void finish(bool keep);

    PictureView* view_ = nullptr;
    bool ready_ = false;
    bool finished_ = false;
    QList<double> committed_;

    VpCanvas* canvas_ = nullptr;
    QButtonGroup* tools_ = nullptr;
    QLabel* help_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    QSpinBox* gridSize_ = nullptr;
    QSpinBox* angle_ = nullptr;
    QAction* showEdges_ = nullptr;
    QAction* renderGrids_ = nullptr;
    QString documentName_;
};

} // namespace pictura
