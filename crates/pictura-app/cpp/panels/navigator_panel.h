#pragma once

#include <QtCore/QPointF>
#include <QtCore/QPointer>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include <functional>

class QLabel;
class QSlider;

namespace pictura {

class ImageView;
class PictureView;

// Paints the document thumbnail and the proxy rectangle for the region the
// canvas currently shows.
class NavigatorThumbnail : public QWidget {
public:
    explicit NavigatorThumbnail(QWidget* parent = nullptr);

    // `image` is the drawn thumbnail (from a pyramid level); `size` is the full
    // document pixel size, used for the proxy and cursor mapping.
    void setImage(const QImage& image);
    void setDocumentSize(const QSize& size);
    void setView(double zoom, const QPointF& offset, const QSize& viewport);
    void setPointPicked(std::function<void(const QPointF&)> callback);

    // The document point under the proxy cursor (last hover or click), used as
    // the slider's zoom anchor so the point under the cursor stays fixed.
    QPointF cursorImagePoint() const { return cursorImage_; }
    bool hasCursorImagePoint() const { return cursorValid_; }
    QSize sourceImageSizeForTest() const { return source_.size(); }
    QImage sourceImage() const { return source_; }

protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;

private:
    QSize documentSize() const;
    QRect imageRect() const;
    QPointF mapToImage(const QPointF& pos) const;
    void pickAt(const QPointF& pos);

    QImage source_;
    QImage scaled_;
    QSize documentSize_;
    double zoom_ = 1.0;
    QPointF offset_;
    QSize viewport_;
    std::function<void(const QPointF&)> picked_;
    QPointF cursorImage_;
    bool cursorValid_ = false;
};

class NavigatorPanel : public QWidget {
    Q_OBJECT

public:
    explicit NavigatorPanel(QWidget* parent = nullptr);

    void setCanvas(ImageView* canvas);
    // The document's pyramid source; thumbnails are drawn from its coarsest
    // level rather than scaling a second full-resolution copy.
    void setView(PictureView* view);
    void refresh();

    QSize thumbnailSourceSizeForTest() const { return thumbnail_->sourceImageSizeForTest(); }
    QImage thumbnailSourceImageForTest() const { return thumbnail_->sourceImage(); }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void syncFromCanvas();
    void centerOn(const QPointF& imagePoint);

    // A floating window's canvas can be deleted before the panels retarget.
    QPointer<ImageView> canvas_;
    PictureView* view_ = nullptr;
    NavigatorThumbnail* thumbnail_ = nullptr;
    QSlider* slider_ = nullptr;
    QLabel* zoomLabel_ = nullptr;
    bool updating_ = false;
};

} // namespace pictura
