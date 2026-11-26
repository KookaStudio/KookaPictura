#pragma once

// The ACR preview pane: the active layer rendered with the current settings on
// the workspace surface, with zoom controls and the shadow / highlight clipping
// toggles.

#include <QtCore/QString>
#include <QtCore/QRect>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

class QEvent;
class QLabel;
class QLineEdit;
class QMouseEvent;
class QToolButton;
class QWheelEvent;

namespace pictura {

class PictureView;

class AcrPreviewPane : public QWidget {
    Q_OBJECT

public:
    AcrPreviewPane(PictureView* view, const QString& path, QWidget* parent = nullptr);

    // Re-render from the bridge with `text` as the settings.
    void setSettingsText(const QString& text);
    // Render the view's pending camera raw source instead of the layer.
    void setRawSource(bool raw) { rawSource_ = raw; }

    const QImage& image() const { return image_; }
    bool shadowClipping() const;
    bool highlightClipping() const;

    // Test seam: replace the rendered image without a bridge call.
    void setPreviewImageForTest(const QImage& image);

    QToolButton* shadowToggleForTest() const { return shadowToggle_; }
    QToolButton* highlightToggleForTest() const { return highlightToggle_; }
    QLineEdit* zoomEditForTest() const { return zoomEdit_; }
    QRect imageRectForTest() const { return imageRect_; }

    // Tint pixels at or below black (blue) / at or above white (red).
    static QImage applyClipping(const QImage& image, bool shadow, bool highlight);

signals:
    // The pixel under the cursor, or -1 in each channel when it leaves.
    void picked(int r, int g, int b);

protected:
    void paintEvent(QPaintEvent* event) override;
    void wheelEvent(QWheelEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void leaveEvent(QEvent* event) override;

private:
    void setZoom(double zoom);
    void updatePicker(const QPoint& pos);
    void stepZoom(int direction);
    void commitZoomEdit();
    void refreshZoomEdit();

    PictureView* view_ = nullptr;
    QString path_;
    bool rawSource_ = false;
    QImage image_;
    QRect imageRect_;
    QPoint lastPickPos_;
    bool pickerActive_ = false;
    double zoom_ = 1.0;
    bool fit_ = true;
    QToolButton* zoomOut_ = nullptr;
    QToolButton* zoomIn_ = nullptr;
    QLineEdit* zoomEdit_ = nullptr;
    QToolButton* shadowToggle_ = nullptr;
    QToolButton* highlightToggle_ = nullptr;
};

} // namespace pictura
