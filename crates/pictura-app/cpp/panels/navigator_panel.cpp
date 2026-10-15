#include "navigator_panel.h"

#include "theme.h"

#include "api/document_api.h"
#include "image_view.h"
#include "jump_slider.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QEvent>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {
constexpr double kMinZoom = 0.01;
constexpr double kMaxZoom = 32.0;
constexpr int kSliderSteps = 1000;

double zoomForSlider(int value)
{
    const double t = double(value) / kSliderSteps;
    return kMinZoom * std::pow(kMaxZoom / kMinZoom, t);
}

int sliderForZoom(double zoom)
{
    const double t = std::log(std::clamp(zoom, kMinZoom, kMaxZoom) / kMinZoom)
                     / std::log(kMaxZoom / kMinZoom);
    return int(std::lround(t * kSliderSteps));
}
} // namespace

NavigatorThumbnail::NavigatorThumbnail(QWidget* parent)
    : QWidget(parent)
{
}

void NavigatorThumbnail::setImage(const QImage& image)
{
    source_ = image;
    scaled_ = QImage();
    update();
}

void NavigatorThumbnail::setDocumentSize(const QSize& size)
{
    documentSize_ = size;
    scaled_ = QImage();
    update();
}

QSize NavigatorThumbnail::documentSize() const
{
    return documentSize_.isEmpty() ? source_.size() : documentSize_;
}

void NavigatorThumbnail::setView(double zoom, const QPointF& offset, const QSize& viewport)
{
    zoom_ = zoom;
    offset_ = offset;
    viewport_ = viewport;
    update();
}

void NavigatorThumbnail::setPointPicked(std::function<void(const QPointF&)> callback)
{
    picked_ = std::move(callback);
}

QRect NavigatorThumbnail::imageRect() const
{
    const QSize doc = documentSize();
    if (source_.isNull() || doc.isEmpty()) {
        return rect();
    }
    const double fit = std::min(double(width()) / doc.width(),
                                double(height()) / doc.height());
    const double scale = std::min(fit, 1.0);
    const QSize size(std::max(1, int(doc.width() * scale)),
                     std::max(1, int(doc.height() * scale)));
    return QRect(QPoint((width() - size.width()) / 2, (height() - size.height()) / 2),
                 size);
}

void NavigatorThumbnail::paintEvent(QPaintEvent*)
{
    QPainter painter(this);
    painter.fillRect(rect(), Theme::tokenColor(QStringLiteral("inputBg")));
    const QRect area = imageRect();
    if (source_.isNull() || area.isEmpty()) {
        painter.setPen(Theme::tokenColor(QStringLiteral("text")));
        painter.drawText(rect(), Qt::AlignCenter, QStringLiteral("No document"));
        return;
    }
    if (scaled_.size() != area.size()) {
        scaled_ = source_.scaled(area.size(), Qt::IgnoreAspectRatio, Qt::SmoothTransformation);
    }
    painter.drawImage(area.topLeft(), scaled_);

    const double zoom = zoom_ > 0.0 ? zoom_ : 1.0;
    const QSize doc = documentSize();
    const double x0 = std::clamp(-offset_.x() / zoom, 0.0, double(doc.width()));
    const double y0 = std::clamp(-offset_.y() / zoom, 0.0, double(doc.height()));
    const double x1 = std::clamp((viewport_.width() - offset_.x()) / zoom, 0.0,
                                 double(doc.width()));
    const double y1 = std::clamp((viewport_.height() - offset_.y()) / zoom, 0.0,
                                 double(doc.height()));
    if (x1 > x0 && y1 > y0) {
        const double sx = double(area.width()) / doc.width();
        const double sy = double(area.height()) / doc.height();
        const QRectF proxy(area.x() + x0 * sx, area.y() + y0 * sy,
                           (x1 - x0) * sx, (y1 - y0) * sy);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(Qt::white, 1));
        painter.drawRect(proxy);
    }
}

void NavigatorThumbnail::resizeEvent(QResizeEvent* event)
{
    scaled_ = QImage();
    QWidget::resizeEvent(event);
}

void NavigatorThumbnail::mousePressEvent(QMouseEvent* event)
{
    if (event->button() == Qt::LeftButton) {
        pickAt(event->position());
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void NavigatorThumbnail::mouseMoveEvent(QMouseEvent* event)
{
    if (event->buttons() & Qt::LeftButton) {
        pickAt(event->position());
        event->accept();
        return;
    }
    if (!source_.isNull() && !imageRect().isEmpty()) {
        cursorImage_ = mapToImage(event->position());
        cursorValid_ = true;
    }
    QWidget::mouseMoveEvent(event);
}

QPointF NavigatorThumbnail::mapToImage(const QPointF& pos) const
{
    const QRect area = imageRect();
    const QSize doc = documentSize();
    if (source_.isNull() || doc.isEmpty() || area.isEmpty()) {
        return QPointF();
    }
    const double x = std::clamp((pos.x() - area.x()) / area.width(), 0.0, 1.0)
                     * (doc.width() - 1);
    const double y = std::clamp((pos.y() - area.y()) / area.height(), 0.0, 1.0)
                     * (doc.height() - 1);
    return QPointF(x, y);
}

void NavigatorThumbnail::pickAt(const QPointF& pos)
{
    const QRect area = imageRect();
    if (source_.isNull() || area.isEmpty() || !picked_) {
        return;
    }
    cursorImage_ = mapToImage(pos);
    cursorValid_ = true;
    picked_(cursorImage_);
}

NavigatorPanel::NavigatorPanel(QWidget* parent)
    : QWidget(parent)
{
    QWidget* body = this;
    auto* layout = new QVBoxLayout(body);

    thumbnail_ = new NavigatorThumbnail(body);
    thumbnail_->setPointPicked([this](const QPointF& point) { centerOn(point); });
    layout->addWidget(thumbnail_, 1);

    auto* zoomRow = new QHBoxLayout();
    slider_ = new JumpSlider(Qt::Horizontal, body);
    slider_->setRange(0, kSliderSteps);
    slider_->setEnabled(false);
    zoomLabel_ = new QLabel(QStringLiteral("100%"), body);
    zoomRow->addWidget(slider_);
    zoomRow->addWidget(zoomLabel_);
    layout->addLayout(zoomRow);

    auto* buttonRow = new QHBoxLayout();
    auto* fit = new QPushButton(tr("Fit"), body);
    auto* actual = new QPushButton(QStringLiteral("100%"), body);
    buttonRow->addWidget(fit);
    buttonRow->addWidget(actual);
    buttonRow->addStretch(1);
    layout->addLayout(buttonRow);

    refresh();

    connect(slider_, &QSlider::valueChanged, this, [this](int value) {
        if (updating_ || !canvas_) {
            return;
        }
        // Anchor at the point under the proxy cursor (last hover/click) so it
        // stays fixed; fall back to the viewport centre when there is none.
        QPointF anchor(canvas_->width() / 2.0, canvas_->height() / 2.0);
        if (thumbnail_->hasCursorImagePoint()) {
            anchor = thumbnail_->cursorImagePoint() * canvas_->zoom() + canvas_->offset();
        }
        canvas_->setZoom(zoomForSlider(value), anchor);
    });
    connect(fit, &QPushButton::clicked, this, [this] {
        if (canvas_) {
            canvas_->fitOnScreen();
        }
    });
    connect(actual, &QPushButton::clicked, this, [this] {
        if (canvas_) {
            canvas_->actualPixels();
        }
    });
}

void NavigatorPanel::setCanvas(ImageView* canvas)
{
    if (canvas_ == canvas) {
        refresh();
        return;
    }
    if (canvas_) {
        disconnect(canvas_, nullptr, this, nullptr);
        canvas_->removeEventFilter(this);
    }
    canvas_ = canvas;
    if (canvas_) {
        connect(canvas_, &ImageView::zoomChanged, this, [this](double) { syncFromCanvas(); });
        canvas_->installEventFilter(this);
    }
    refresh();
}

bool NavigatorPanel::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == canvas_ && canvas_) {
        if (event->type() == QEvent::Paint || event->type() == QEvent::Resize) {
            syncFromCanvas();
        }
    }
    return QWidget::eventFilter(watched, event);
}

void NavigatorPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

void NavigatorPanel::refresh()
{
    if (!canvas_) {
        thumbnail_->setImage(QImage());
        thumbnail_->setDocumentSize(QSize());
        thumbnail_->setView(1.0, QPointF(), QSize());
        updating_ = true;
        slider_->setValue(0);
        slider_->setEnabled(false);
        updating_ = false;
        zoomLabel_->setText(QStringLiteral("—"));
        return;
    }
    QSize docSize = canvas_->documentSize();
    QImage thumb;
    if (!view_ || !view_->has_document()) {
        thumb = canvas_->image();
    } else {
        docSize = QSize(view_->document_width(), view_->document_height());
        const QVector<document_api::CanvasSize> levels = document_api::pyramidSizes(*view_);
        if (!levels.isEmpty()) {
            const document_api::CanvasSize& level = levels.last();
            thumb = view_->display_image(levels.size() - 1, 0, 0, level.width, level.height);
        }
    }
    thumbnail_->setDocumentSize(docSize);
    thumbnail_->setImage(thumb);
    syncFromCanvas();
}

void NavigatorPanel::syncFromCanvas()
{
    if (!canvas_) {
        return;
    }
    thumbnail_->setView(canvas_->zoom(), canvas_->offset(), canvas_->size());
    updating_ = true;
    slider_->setEnabled(true);
    slider_->setValue(sliderForZoom(canvas_->zoom()));
    updating_ = false;
    zoomLabel_->setText(QString::number(canvas_->zoom() * 100.0, 'f', 0)
                        + QStringLiteral("%"));
}

void NavigatorPanel::centerOn(const QPointF& imagePoint)
{
    if (!canvas_) {
        return;
    }
    const QPointF center(canvas_->width() / 2.0, canvas_->height() / 2.0);
    const QPointF target = center - imagePoint * canvas_->zoom();
    canvas_->panBy(target - canvas_->offset());
    syncFromCanvas();
}

} // namespace pictura
