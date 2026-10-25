#pragma once

// CS6's Vanishing Point workspace: the menu button (Show Edges, Render Grids
// To Photoshop), the tool options over a help line, OK / Cancel, the tool
// column (Edit Plane, Create Plane, Marquee, Stamp, Brush, Transform,
// Eyedropper, Hand, Zoom) and the preview with its zoom row. Ctrl+V pastes the
// clipboard as a floating selection, Ctrl+T picks Transform for it, Ctrl+D
// drops it and clears the marquee, and [ / ] size the Stamp and Brush. The
// planes and the edited layer live in a Rust session; OK keeps the planes on
// the document and, when the session changed pixels or Render Grids is on,
// fills the `vanishing-point` slot with the kept edit's id.
// ponytail: the Extended-only Measure tool, Render Measurements and DXF/3DS
// export are not offered.

#include <QtCore/QList>
#include <QtWidgets/QDialog>

class QAction;
class QButtonGroup;
class QLabel;

namespace pictura {

class PictureView;
class VpCanvas;
class VpOptionsBar;

class VanishingPointDialog : public QDialog {
    Q_OBJECT

public:
    // In CS6's column order.
    enum Tool { EditPlane, CreatePlane, Marquee, Stamp, Brush, Transform, Eyedropper, Hand, Zoom };

    // `documentName` titles the dialog, as "Vanishing Point (name @ zoom)".
    // A main-window `parent` gives Brush Color its foreground colour.
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
    VpOptionsBar* options() const { return options_; }
    QAction* showEdgesAction() const { return showEdges_; }
    QAction* renderGridsAction() const { return renderGrids_; }
    QLabel* helpLine() const { return help_; }
    // Paste the clipboard (Ctrl+V); false, with the reason on the help line,
    // when it cannot.
    bool paste();
    // The kind's slot after OK with something to commit; empty otherwise.
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
    void toolChanged(int tool);

    PictureView* view_ = nullptr;
    bool ready_ = false;
    bool finished_ = false;
    QList<double> committed_;

    VpCanvas* canvas_ = nullptr;
    VpOptionsBar* options_ = nullptr;
    QButtonGroup* tools_ = nullptr;
    QLabel* help_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    QAction* showEdges_ = nullptr;
    QAction* renderGrids_ = nullptr;
    QString documentName_;
};

} // namespace pictura
