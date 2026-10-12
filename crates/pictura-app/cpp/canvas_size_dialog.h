#pragma once

#include <QtGui/QColor>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QToolButton;

namespace pictura {

class ColorSwatchWidget;
class PictureView;

// Canvas Size's nine-square anchor grid. Painted whole: every square's arrow
// points away from the chosen one, so each cell depends on the selection.
class AnchorSelector : public QWidget {
    Q_OBJECT

public:
    explicit AnchorSelector(QWidget* parent = nullptr);

    // 0..2 across (left, centre, right) and down (top, centre, bottom).
    int anchorX() const { return x_; }
    int anchorY() const { return y_; }
    void setAnchor(int x, int y);
    // The engine anchor name: "top-left" … "bottom-right".
    QString anchorName() const;

    QSize sizeHint() const override;

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;

private:
    QRect cellRect(int cx, int cy) const;

    int x_ = 1;
    int y_ = 1;
};

// Image > Canvas Size: grows or crops the room the image sits in around an
// anchor without scaling a pixel; the Background layer's new area takes the
// Canvas extension color. Width/Height take Percent, Pixels, or a physical
// unit (through the document resolution); Relative counts from the current
// size. Ported from photorust's CanvasSizeDialog.
class CanvasSizeDialog : public QDialog {
    Q_OBJECT

public:
    CanvasSizeDialog(PictureView* view, const QColor& foreground, const QColor& background,
                     QWidget* parent = nullptr);

    int resultWidth() const;
    int resultHeight() const;
    QString anchorName() const;
    QColor extensionColor() const;

    QWidget* controlForTest(const QString& name) const;

private:
    void buildUi();
    void updateSizes();
    void relativeToggled(bool on);
    void extensionChosen();
    void pickExtensionColor();
    void updateSwatch();
    double unitScale(int unit) const;
    int toPixels(const QDoubleSpinBox* field, int unit, int base) const;
    // Restate `field` in `unit`, keeping the size it describes; `previous` is
    // the unit its number is written in now.
    void changeUnit(QDoubleSpinBox* field, int unit, int& previous, int base);

    int pixelWidth_ = 1;
    int pixelHeight_ = 1;
    double resolution_ = 72.0;
    double bytesPerPixel_ = 3.0;
    QColor foreground_;
    QColor background_;
    QColor customColor_ = Qt::white;

    QLabel* currentSize_ = nullptr;
    QLabel* currentWidth_ = nullptr;
    QLabel* currentHeight_ = nullptr;
    QLabel* newSize_ = nullptr;
    QDoubleSpinBox* width_ = nullptr;
    QDoubleSpinBox* height_ = nullptr;
    QComboBox* widthUnit_ = nullptr;
    QComboBox* heightUnit_ = nullptr;
    int widthUnitPrevious_ = 0;
    int heightUnitPrevious_ = 0;
    QCheckBox* relative_ = nullptr;
    AnchorSelector* anchor_ = nullptr;
    QComboBox* extension_ = nullptr;
    ColorSwatchWidget* swatch_ = nullptr;
};

} // namespace pictura
