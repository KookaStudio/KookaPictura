#pragma once

// The ACR preview pane: the active layer rendered with the current settings,
// over a transparency checkerboard, with a zoom readout and the shadow /
// highlight clipping toggles.

#include <QtCore/QString>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

class QLabel;
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

    // Tint pixels at or below black (blue) / at or above white (red).
    static QImage applyClipping(const QImage& image, bool shadow, bool highlight);

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    PictureView* view_ = nullptr;
    QString path_;
    QImage image_;
    QLabel* zoomLabel_ = nullptr;
    QToolButton* shadowToggle_ = nullptr;
    QToolButton* highlightToggle_ = nullptr;
};

} // namespace pictura
