#pragma once

// CS6's shared filter dialog, ported from perfecto25/photorust's
// shell/src/dialogs/FilterPreviewDialog.{h,cpp}.
// Source: https://github.com/perfecto25/photorust

#include <QtCore/QList>
#include <QtCore/QPointer>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QRectF>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include "filter_commands.h"

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QProgressBar;
class QPushButton;
class QResizeEvent;
class QShowEvent;
class QTimer;
class QSlider;

namespace pictura {

class FilterParamControls;
class ImageView;
class PictureView;

// The canvas state a filter dialog previews against: the document-space rect
// the user is looking at and its zoom, so the thumbnail shows that section at a
// comparable scale and the live preview only filters the visible area.
struct FilterPreviewView {
    QRectF visible;
    double canvasZoom = 1.0;
    // The canvas a zoomed-out preview draws its reduced copy on, and its
    // device pixel ratio. Without one every preview filters the document.
    ImageView* canvas = nullptr;
    double devicePixelRatio = 1.0;
    // The document's other windows' canvases, which show the same preview.
    QList<ImageView*> mirrors;
};

// CS6's shared filter dialog: a preview thumbnail with its own zoom and a
// Preview checkbox, plus one control per filter parameter. Radial Blur lays out
// without the thumbnail; every other filter re-renders a non-committing preview
// on the canvas as a control changes. Ported from photorust's FilterPreviewDialog.
class FilterPreviewDialog : public QDialog {
    Q_OBJECT

public:
    FilterPreviewDialog(PictureView* view, const FilterCommandSpec& spec,
                        const FilterPreviewView& previewView = {}, QWidget* parent = nullptr);
    ~FilterPreviewDialog() override;

    // The parameter slot values in mapping order.
    QList<double> values() const;

    // Set the controls from slot values; slots past the end keep their value.
    // Silent: emits no changed().
    void applyInitial(const QList<double>& initial);

    // Run the dialog modally, committing `out` on OK. On cancel the canvas
    // preview is discarded.
    static bool get(PictureView* view, const FilterCommandSpec& spec,
                    const FilterPreviewView& previewView, const QList<double>& initial,
                    QList<double>* out, QWidget* parent);

private:
    // Schedule a canvas preview of the current values; changes that arrive
    // before it runs (queued behind a slow preview) coalesce into one.
    void valuesChanged();
    void runPreview();
    void discardPreview();
    void updateThumbnail();
    void zoomChanged();
    // Below 100% zoom over a large area the canvas shows a reduced copy
    // filtered with scaled pixel parameters (photocraft's proxy preview,
    // #152); the document is filtered only on OK.
    bool previewsOnProxy() const;
    bool showProxy();
    void clearProxy();
    // The exact picture: the worker filters the full-resolution layer and the
    // canvas swaps the draft copy for it when it lands.
    bool startExactPreview(const QList<double>& values);
    // Set the exact-preview region and scale without drawing a synchronous
    // draft, so a large whole-layer preview at full zoom never filters on the
    // GUI thread.
    void prepareExactPreview();
    void pollExactPreview();
    void cancelExactPreview();
    // The tail of runPreview: filter the live layer/selection synchronously.
    void runSynchronousPreview(const QList<double>& values);
    // The pane's section filtered off the document while the canvas shows
    // the copy.
    void updatePaneSection();
    // Lens Blur's Source slot is document state, so its depth-map id is
    // materialized fresh into the hidden slot on every read.
    QList<double> resolvedValues(QList<double> values) const;
    QRect proxyRegion() const;
    // The document section the pane shows at its zoom level.
    QRect thumbnailCrop() const;

protected:
    void resizeEvent(QResizeEvent* event) override;
    void showEvent(QShowEvent* event) override;

private:
    PictureView* view_ = nullptr;
    const FilterCommandSpec spec_;
    FilterParamControls* controls_ = nullptr;
    QCheckBox* preview_ = nullptr;
    QLabel* thumbnail_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    int zoom_ = 2;
    bool previewShown_ = false;
    bool shownOnce_ = false;
    QTimer* previewTimer_ = nullptr;
    QPointer<ImageView> canvas_;
    QList<QPointer<ImageView>> mirrors_;
    double devicePixelRatio_ = 1.0;
    // The filter places itself against the whole layer, so neither a section
    // preview nor a section of the proxy stands for it.
    bool positional_ = false;
    bool proxyShown_ = false;
    QImage proxyPicture_;
    QRect proxyArea_;
    double proxyScale_ = 1.0;
    QImage sectionImage_;
    QTimer* exactTimer_ = nullptr;
    QProgressBar* exactBusy_ = nullptr;
    // A started exact request whose picture has not landed yet.
    bool exactPending_ = false;
    QRectF previewVisible_;
    double canvasZoom_ = 1.0;
    // A small pre-preview picture for a bottom pane (Shear): the document with
    // the active layer hidden, so each pane update filters only the active
    // layer at proxy scale and composites it over this base. The pane follows a
    // drag while the whole-layer preview waits for release.
    QImage baseProxy_;
};

} // namespace pictura
