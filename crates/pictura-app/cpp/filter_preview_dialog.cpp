#include "filter_preview_dialog.h"
#include "filter_param_controls.h"
#include "dialogs.h"
#include "image_view.h"
#include "panels/jump_slider.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtCore/QVariant>
#include <QtGui/QColor>
#include <QtGui/QFontMetrics>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtGui/QPixmap>
#include <QtGui/QResizeEvent>
#include <QtGui/QShowEvent>
#include <QtMath>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSizePolicy>
#include <QtWidgets/QSlider>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <algorithm>
#include <cmath>
#include <functional>
#include <limits>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/jobs.cxxqt.h"

namespace pictura {

namespace {

// More parameter rows than this spill into a second input column, matching CS6's
// 3-column dialogs for the input-heavy filters (Smart Sharpen, Wave, ...).
const int kManyParams = 8;

// A stacked dialog's rows span this width, wider than the preview alone.
const int kStackedRowWidth = 360;

// The bottom preview pane (Shear) re-filters a proxy of this size, so a
// curve drag stays responsive while the whole-layer canvas render waits.
const int kProxySize = 320;

// A Placement pad's picture: a reduced copy at least this size on its long side.
const int kPadImageEdge = 256;

// A zoomed-out canvas preview over more document pixels than this filters a
// reduced copy instead of the document (#152): on the 16507×16196 map an exact
// preview cost seconds per change, the copy tens of milliseconds.
// ponytail: one budget for every filter, not a per-filter cost model.
const double kExactPreviewPixels = 4.0e6;

// The reduced copy holds at most this many pixels, whatever the canvas size.
const double kCanvasProxyPixels = 4.0e6;

// CS6 preview zoom steps, as a percentage of the base thumbnail size.
const int kZoomLevels[] = {25, 50, 100, 200, 400};
const int kZoomCount = 5;

// A magnifier zoom button; falls back to a plain +/- glyph on a theme that has
// no zoom icons.
QToolButton* makeZoomButton(bool zoomIn, QWidget* parent)
{
    auto* button = new QToolButton(parent);
    const QIcon icon =
        QIcon::fromTheme(zoomIn ? QStringLiteral("zoom-in") : QStringLiteral("zoom-out"));
    if (!icon.isNull()) {
        button->setIcon(icon);
    } else {
        button->setText(zoomIn ? QStringLiteral("+") : QStringLiteral("\u2212"));
        button->setToolButtonStyle(Qt::ToolButtonTextOnly);
    }
    button->setAutoRaise(true);
    return button;
}

} // namespace

FilterPreviewDialog::FilterPreviewDialog(PictureView* view, const FilterCommandSpec& spec,
                                         const FilterPreviewView& previewView, QWidget* parent)
    : QDialog(parent), view_(view), spec_(spec)
{
    setWindowTitle(spec.label);
    previewTimer_ = new QTimer(this);
    previewTimer_->setSingleShot(true);
    previewTimer_->setInterval(0);
    connect(previewTimer_, &QTimer::timeout, this, &FilterPreviewDialog::runPreview);
    exactTimer_ = new QTimer(this);
    exactTimer_->setInterval(60);
    connect(exactTimer_, &QTimer::timeout, this, &FilterPreviewDialog::pollExactPreview);
    previewVisible_ = previewView.visible;
    canvasZoom_ = previewView.canvasZoom;
    canvas_ = previewView.canvas;
    devicePixelRatio_ = previewView.devicePixelRatio;
    positional_ = filter_kind_positional(spec.kind);
    // Seed the preview zoom from the canvas zoom, snapped to a preview level.
    const double percent = canvasZoom_ * 100.0;
    int best = 2;
    double bestDelta = std::numeric_limits<double>::max();
    for (int i = 0; i < kZoomCount; ++i) {
        const double delta = qAbs(kZoomLevels[i] - percent);
        if (delta < bestDelta) {
            bestDelta = delta;
            best = i;
        }
    }
    zoom_ = best;

    auto* outer = new QVBoxLayout(this);

    // CS6's columns: preview and the first inputs on the left, any overflow
    // inputs in the middle, OK / Cancel / Preview on the right.
    auto* body = new QHBoxLayout;
    body->setSpacing(12);

    auto* leftColumn = new QVBoxLayout;
    leftColumn->setSpacing(8);
    const bool previewBelow = spec.previewBelow;
    if (spec.previewPane && !previewBelow) {
        thumbnail_ = new QLabel(this);
        thumbnail_->setObjectName(QStringLiteral("filterThumbnail"));
        thumbnail_->setMinimumSize(200, 200);
        thumbnail_->setAlignment(Qt::AlignCenter);
        thumbnail_->setFrameShape(QFrame::StyledPanel);
        // Ignored: the pixmap must not drive the layout back through sizeHint;
        // the label just fills its column and the image is re-rendered to fit.
        thumbnail_->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Ignored);
        // Stretch 1: extra dialog height grows the preview, not a blank gap.
        leftColumn->addWidget(thumbnail_, 1);

        auto* zoomRow = new QHBoxLayout;
        auto* zoomOut = makeZoomButton(false, this);
        zoomOut->setObjectName(QStringLiteral("filterZoomOut"));
        auto* zoomIn = makeZoomButton(true, this);
        zoomIn->setObjectName(QStringLiteral("filterZoomIn"));
        zoomLabel_ = new QLabel(this);
        zoomLabel_->setObjectName(QStringLiteral("filterZoomLabel"));
        zoomLabel_->setAlignment(Qt::AlignCenter);
        connect(zoomIn, &QToolButton::clicked, this, [this] {
            zoom_ = qMin(zoom_ + 1, kZoomCount - 1);
            zoomChanged();
        });
        connect(zoomOut, &QToolButton::clicked, this, [this] {
            zoom_ = qMax(zoom_ - 1, 0);
            zoomChanged();
        });
        addZoomShortcuts(this, zoomIn, zoomOut);
        zoomRow->addStretch(1);
        zoomRow->addWidget(zoomOut);
        zoomRow->addWidget(zoomLabel_);
        zoomRow->addWidget(zoomIn);
        zoomRow->addStretch(1);
        leftColumn->addLayout(zoomRow);
    }
    if (previewBelow) {
        // CS6's Shear: the picture under the inputs, no zoom row.
        thumbnail_ = new QLabel(this);
        thumbnail_->setObjectName(QStringLiteral("filterThumbnail"));
        thumbnail_->setMinimumSize(200, 120);
        thumbnail_->setAlignment(Qt::AlignCenter);
        thumbnail_->setFrameShape(QFrame::StyledPanel);
        thumbnail_->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Ignored);
        // Cache the picture before any canvas preview: the document with the
        // active layer hidden, so each pane update composites the filtered
        // active layer over it rather than reading the whole canvas.
        if (view_ && view_->has_document()) {
            baseProxy_ = filter_proxy_base(*view_, kProxySize);
        }
    }
    body->addLayout(leftColumn, 1);

    // One self-contained row per parameter, built by addControl; a filter with
    // many inputs spills the overflow into the middle column.
    // Only a Placement pad shows the picture, and only a pad-sized one.
    const bool hasPad = std::any_of(spec_.params.cbegin(), spec_.params.cend(),
                                    [](const FilterParamSpec& param) {
                                        return param.control == FilterControl::Placement;
                                    });
    controls_ = new FilterParamControls(spec_.params, this,
                                        hasPad ? documentPreviewImage(view_, kPadImageEdge)
                                               : QImage());
    connect(controls_, &FilterParamControls::changed, this, &FilterPreviewDialog::valuesChanged);
    connect(controls_, &FilterParamControls::changedLive, this, &FilterPreviewDialog::updateThumbnail);
    QList<QWidget*> rows = controls_->rows();
    if (spec.stacked) {
        // Checkboxes trail the column so the sliders run on unbroken.
        QList<QWidget*> checks;
        for (int i = rows.size() - 1; i >= 0; --i) {
            if (spec.params.at(i).control == FilterControl::CheckBox) {
                checks.prepend(rows.takeAt(i));
            }
        }
        rows.append(checks);
        for (QWidget* row : rows) {
            row->setMinimumWidth(kStackedRowWidth);
        }
    }
    QVBoxLayout* middleColumn = nullptr;
    const int split = rows.size() > kManyParams && !spec.stacked ? (rows.size() + 1) / 2
                                                                 : rows.size();
    for (int i = 0; i < rows.size(); ++i) {
        QVBoxLayout* column = leftColumn;
        if (i >= split) {
            if (!middleColumn) {
                middleColumn = new QVBoxLayout;
                middleColumn->setSpacing(8);
                body->addLayout(middleColumn, 1);
            }
            column = middleColumn;
        }
        column->addWidget(rows.at(i));
    }
    if (!spec.previewPane) {
        leftColumn->addStretch(1);
    }
    if (middleColumn) {
        middleColumn->addStretch(1);
    }

    auto* buttonColumn = new QVBoxLayout;
    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    if (!previewBelow && spec.livePreview) {
        preview_ = new QCheckBox(QStringLiteral("Preview"), this);
        preview_->setObjectName(QStringLiteral("filterPreview"));
        preview_->setChecked(true);
        connect(preview_, &QCheckBox::toggled, this, [this](bool on) {
            if (on) {
                valuesChanged();
            } else {
                discardPreview();
            }
        });
    }
    buttonColumn->addWidget(ok);
    buttonColumn->addWidget(cancel);
    if (preview_) {
        buttonColumn->addSpacing(6);
        buttonColumn->addWidget(preview_);
    }
    // CS6 blinks a line while a preview renders; this bar runs while the
    // exact picture is computed.
    exactBusy_ = new QProgressBar(this);
    exactBusy_->setObjectName(QStringLiteral("filterExactBusy"));
    exactBusy_->setRange(0, 0);
    exactBusy_->setTextVisible(false);
    exactBusy_->setMaximumHeight(6);
    exactBusy_->setToolTip(tr("Rendering the preview at full resolution"));
    exactBusy_->hide();
    buttonColumn->addWidget(exactBusy_);
    buttonColumn->addStretch(1);
    body->addLayout(buttonColumn);
    outer->addLayout(body);
    if (previewBelow) {
        outer->addWidget(thumbnail_, 1);
    }

    connect(this, &QDialog::rejected, this, &FilterPreviewDialog::discardPreview);
}

FilterPreviewDialog::~FilterPreviewDialog()
{
    exactTimer_->stop();
    // On OK the canvas keeps the preview until the commit lands (the caller
    // clears it), and a finished exact preview is kept for the commit to reuse.
    if (result() != QDialog::Accepted) {
        clearProxy();
    }
    if (view_) {
        filter_canvas_proxy_release(*view_);
    }
}

void FilterPreviewDialog::applyInitial(const QList<double>& initial)
{
    controls_->setValues(initial);
    updateThumbnail();
}

QList<double> FilterPreviewDialog::values() const
{
    return resolvedValues(controls_->values());
}

QList<double> FilterPreviewDialog::resolvedValues(QList<double> values) const
{
    if (spec_.kind == QStringLiteral("lens-blur") && values.size() == 15 && view_) {
        values[13] = filter_lens_blur_depth(*view_, static_cast<int>(values[1]));
    }
    return values;
}

void FilterPreviewDialog::valuesChanged()
{
    if (!spec_.livePreview || (preview_ && !preview_->isChecked())) {
        return;
    }
    previewTimer_->start();
}

void FilterPreviewDialog::runPreview()
{
    if (preview_ && !preview_->isChecked()) {
        return;
    }
    if (!view_ || !view_->has_document()) {
        return;
    }
    const QList<double> current = values();
    if (previewsOnProxy() && showProxy()) {
        // The canvas shows the copy as a draft at once; the pane shows its own
        // section exactly (a positional filter's pane crops the canvas picture).
        startExactPreview(current);
        updatePaneSection();
        updateThumbnail();
        return;
    }
    clearProxy();
    cancelExactPreview();
    // Restrict the preview to the visible section when the caller supplied one;
    // the commit still filters the whole layer.
    const bool shown =
        previewVisible_.isEmpty()
            ? filter_preview(*view_, spec_.kind, current)
            : filter_preview_section(*view_, spec_.kind, current,
                                     static_cast<int>(previewVisible_.left()),
                                     static_cast<int>(previewVisible_.top()),
                                     static_cast<int>(previewVisible_.width()),
                                     static_cast<int>(previewVisible_.height()));
    if (shown) {
        previewShown_ = true;
        updateThumbnail();
    }
}

bool FilterPreviewDialog::previewsOnProxy() const
{
    if (!canvas_ || canvasZoom_ >= 1.0) {
        return false;
    }
    const QRect region = proxyRegion();
    return double(region.width()) * region.height() > kExactPreviewPixels;
}

QRect FilterPreviewDialog::proxyRegion() const
{
    const QRect document(0, 0, view_->document_width(), view_->document_height());
    if (positional_ || previewVisible_.isEmpty()) {
        return document;
    }
    return previewVisible_.toAlignedRect().intersected(document);
}

bool FilterPreviewDialog::showProxy()
{
    const QRect region = proxyRegion();
    const double area = double(region.width()) * region.height();
    double scale = qMin(1.0, canvasZoom_ * devicePixelRatio_);
    if (area * scale * scale > kCanvasProxyPixels) {
        scale = std::sqrt(kCanvasProxyPixels / area);
    }
    const QImage picture =
        filter_canvas_proxy(*view_, spec_.kind, resolvedValues(controls_->scaledValues(scale)),
                            region.x(), region.y(), region.width(), region.height(), scale);
    if (picture.isNull()) {
        return false;
    }
    proxyPicture_ = picture;
    proxyArea_ = region;
    proxyScale_ = scale;
    proxyShown_ = true;
    canvas_->setFilterPreview(picture, region);
    return true;
}

void FilterPreviewDialog::clearProxy()
{
    if (proxyShown_ && canvas_) {
        canvas_->clearFilterPreview();
    }
    proxyShown_ = false;
}

void FilterPreviewDialog::startExactPreview(const QList<double>& values)
{
    if (filter_exact_preview_start(*view_, spec_.kind, values, proxyArea_.x(), proxyArea_.y(),
                                   proxyArea_.width(), proxyArea_.height(), proxyScale_)) {
        exactTimer_->start();
        pollExactPreview();
    }
}

void FilterPreviewDialog::pollExactPreview()
{
    if (!view_) {
        return;
    }
    const QImage exact = filter_exact_preview_poll(*view_);
    if (!exact.isNull() && proxyShown_ && canvas_) {
        proxyPicture_ = exact;
        canvas_->setFilterPreview(exact, proxyArea_);
        if (positional_) {
            updateThumbnail();
        }
    }
    const bool busy = filter_exact_preview_busy(*view_);
    exactBusy_->setVisible(busy);
    if (!busy) {
        exactTimer_->stop();
    }
}

void FilterPreviewDialog::cancelExactPreview()
{
    exactTimer_->stop();
    exactBusy_->hide();
    if (view_) {
        filter_exact_preview_cancel(*view_);
    }
}

void FilterPreviewDialog::updatePaneSection()
{
    sectionImage_ = QImage();
    if (!thumbnail_ || positional_ || spec_.previewBelow || !proxyShown_) {
        return;
    }
    const QRect crop = thumbnailCrop();
    sectionImage_ = filter_section_image(*view_, spec_.kind, values(), crop.x(), crop.y(),
                                         crop.width(), crop.height());
}

void FilterPreviewDialog::zoomChanged()
{
    // On the reduced copy the pane's section is filtered on its own, so a new
    // section needs a new render.
    if (proxyShown_) {
        updatePaneSection();
    }
    updateThumbnail();
}

void FilterPreviewDialog::discardPreview()
{
    previewTimer_->stop();
    clearProxy();
    cancelExactPreview();
    sectionImage_ = QImage();
    if (previewShown_ && view_) {
        filter_preview_cancel(*view_);
        previewShown_ = false;
    }
}

void FilterPreviewDialog::updateThumbnail()
{
    const int level = kZoomLevels[qBound(0, zoom_, kZoomCount - 1)];
    if (zoomLabel_) {
        zoomLabel_->setText(QStringLiteral("%1%").arg(level));
    }
    if (!thumbnail_) {
        return;
    }
    const QSize target = thumbnail_->size().expandedTo(thumbnail_->minimumSize());
    if (spec_.previewBelow) {
        // CS6's Shear shows the whole picture under the inputs: the active
        // layer filtered on the small proxy and composited over the document
        // with it hidden, so a drag stays responsive and the other layers are
        // not filtered with it.
        QImage shown;
        if (view_ && !baseProxy_.isNull()) {
            const QImage layer = filter_proxy_layer(*view_, spec_.kind, values(), kProxySize);
            if (!layer.isNull()) {
                shown = baseProxy_.copy();
                QPainter painter(&shown);
                painter.drawImage(0, 0, layer);
            }
        }
        if (!shown.isNull()) {
            thumbnail_->setPixmap(QPixmap::fromImage(shown).scaled(
                target, Qt::KeepAspectRatio, Qt::SmoothTransformation));
        }
        return;
    }
    if (!view_ || !view_->has_document()) {
        return;
    }
    const QRect pixels = thumbnailCrop();
    if (pixels.isEmpty()) {
        return;
    }
    QImage section;
    if (proxyShown_ && positional_ && !proxyArea_.isEmpty()) {
        const double sx = double(proxyPicture_.width()) / proxyArea_.width();
        const double sy = double(proxyPicture_.height()) / proxyArea_.height();
        const QRectF source((pixels.x() - proxyArea_.x()) * sx, (pixels.y() - proxyArea_.y()) * sy,
                            pixels.width() * sx, pixels.height() * sy);
        section = proxyPicture_.copy(source.toAlignedRect());
    } else if (proxyShown_ && !sectionImage_.isNull()) {
        section = sectionImage_;
    } else {
        // Only the crop is read, never the full-resolution image.
        section = view_->display_image(0, pixels.x(), pixels.y(), pixels.width(), pixels.height());
    }
    if (section.isNull()) {
        return;
    }
    thumbnail_->setPixmap(
        QPixmap::fromImage(section).scaled(target, Qt::KeepAspectRatio, Qt::SmoothTransformation));
}

QRect FilterPreviewDialog::thumbnailCrop() const
{
    const QRectF document(0, 0, view_->document_width(), view_->document_height());
    if (document.isEmpty()) {
        return QRect();
    }
    // The current canvas section, not the whole image: at 100% the pane shows
    // 200 document pixels 1:1; a higher zoom shows fewer pixels larger.
    const int level = kZoomLevels[qBound(0, zoom_, kZoomCount - 1)];
    const double shown = 200.0 * 100.0 / static_cast<double>(level);
    const QPointF center = previewVisible_.isNull() ? document.center() : previewVisible_.center();
    QRectF crop(center.x() - shown / 2.0, center.y() - shown / 2.0, shown, shown);
    crop = crop.intersected(document);
    return (crop.isEmpty() ? document : crop).toRect();
}

void FilterPreviewDialog::resizeEvent(QResizeEvent* event)
{
    QDialog::resizeEvent(event);
    // The preview pane grows with the dialog; re-render it at the new size.
    updateThumbnail();
}

void FilterPreviewDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    // Preview on open, as CS6 does. Deferred a turn so the dialog paints before
    // a slow filter runs.
    if (!shownOnce_) {
        shownOnce_ = true;
        QTimer::singleShot(0, this, [this] {
            if (isVisible()) {
                valuesChanged();
            }
        });
    }
}

bool FilterPreviewDialog::get(PictureView* view, const FilterCommandSpec& spec,
                              const FilterPreviewView& previewView, const QList<double>& initial,
                              QList<double>* out, QWidget* parent)
{
    FilterPreviewDialog dialog(view, spec, previewView, parent);
    dialog.applyInitial(initial);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.values();
    return true;
}

} // namespace pictura

#include "moc_filter_preview_dialog.cpp"
