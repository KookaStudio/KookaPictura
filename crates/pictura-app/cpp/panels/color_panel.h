#pragma once

#include <QtCore/QObject>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include <array>
#include <functional>
#include <optional>

class QPushButton;

namespace pictura {

class ColorPlane;
class PictureView;
class ColorRamp;
class ColorSwatchWidget;

// Application foreground/background colour shared by the toolbox, Color,
// Swatches, and Eyedropper.
class ColorState : public QObject {
    Q_OBJECT

public:
    explicit ColorState(QObject* parent = nullptr);

    QColor foreground() const { return foreground_; }
    QColor background() const { return background_; }
    void setForeground(const QColor& color);
    void setBackground(const QColor& color);
    std::array<double, 3> sceneColor(bool foreground) const;
    void setSceneColor(bool foreground, const std::array<double, 3>& color);
    void swapColors();
    void setHdrPickerContext(std::function<PictureView*()> view, std::function<int()> offset);
    void openColorPicker(bool foreground, QWidget* parent, const QString& title);

    bool foregroundActive() const { return foregroundActive_; }
    void setForegroundActive(bool foreground);

    bool maskActive() const { return maskActive_; }
    void setMaskActive(bool mask);

signals:
    void foregroundChanged(QColor color);
    void backgroundChanged(QColor color);
    void activeChanged(bool foreground);

private:
    QColor foreground_{Qt::black};
    QColor background_{Qt::white};
    std::optional<std::array<double, 3>> hdrForeground_;
    std::optional<std::array<double, 3>> hdrBackground_;
    std::function<PictureView*()> view_;
    std::function<int()> offset_;
    bool foregroundActive_ = true;
    bool maskActive_ = false;
};

// The Color panel: the foreground/background swatches over Photoshop's colour
// field and hue ramp. Clicking the field or ramp sets the active swatch.
class ColorPanel : public QWidget {
    Q_OBJECT

public:
    explicit ColorPanel(ColorState* state, QWidget* parent = nullptr);

private:
    void syncControls();
    void selectColor(const QColor& color);
    QColor activeColor() const;
    void paintSwatch(ColorSwatchWidget* button, const QColor& color, bool active);

    ColorState* state_ = nullptr;
    ColorSwatchWidget* fgSwatch_ = nullptr;
    ColorSwatchWidget* bgSwatch_ = nullptr;
    ColorPlane* plane_ = nullptr;
    ColorRamp* ramp_ = nullptr;
};

} // namespace pictura
