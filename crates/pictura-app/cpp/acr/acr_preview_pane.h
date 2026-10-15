#pragma once

// The ACR preview pane: the active layer rendered with the current settings on
// the workspace surface, with zoom controls and the shadow / highlight clipping
// toggles.

#include <QtCore/QString>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

class QLabel;
class QLineEdit;
class QToolButton;

namespace pictura {

class PictureView;

class AcrPreviewPane : public QWidget {
    Q_OBJECT

public:
    AcrPreviewPane(PictureView* view, const QString& path, QWidget* parent = nullptr);

    // Re-render from the bridge with `text` as the settings.
    void setSettingsText(const QString& text);

    const QImage& image() const { return image_; }
    bool shadowClipping() const;
    bool highlightClipping() const;

    QToolButton* shadowToggleForTest() const { return shadowToggle_; }
    QToolButton* highlightToggleForTest() const { return highlightToggle_; }
    QLineEdit* zoomEditForTest() const { return zoomEdit_; }

    // Tint pixels at or below black (blue) / at or above white (red).
    static QImage applyClipping(const QImage& image, bool shadow, bool highlight);

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    void setZoom(double zoom);
    void stepZoom(int direction);
    void commitZoomEdit();
    void refreshZoomEdit();

    PictureView* view_ = nullptr;
    QString path_;
    QImage image_;
    double zoom_ = 1.0;
    bool fit_ = true;
    QToolButton* zoomOut_ = nullptr;
    QToolButton* zoomIn_ = nullptr;
    QLineEdit* zoomEdit_ = nullptr;
    QToolButton* shadowToggle_ = nullptr;
    QToolButton* highlightToggle_ = nullptr;
};

} // namespace pictura
