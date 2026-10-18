#pragma once

#include <QColor>
#include <QDialog>
#include <QImage>
#include <QPointer>
#include <QWidget>

#include <functional>

class QCheckBox;
class QLabel;
class QLineEdit;
class QRadioButton;
class QSpinBox;

namespace pictura {

class SwatchGrid;
class ToolController;

/// The component the vertical ramp controls. In Photoshop this is chosen by the
/// radio buttons beside the numeric fields, and it re-maps both the ramp and
/// the two axes of the square field.
enum class ColorAxis { Hue, Saturation, Brightness, Red, Green, Blue };

/// The square colour field: the plane of the two components the current
/// [`ColorAxis`] does not control, with a ring marker at the current colour.
class ColorPlane : public QWidget
{
    Q_OBJECT

public:
    explicit ColorPlane(QWidget* parent = nullptr);

    void setAxis(ColorAxis axis);
    /// Update the displayed colour. Does not emit `picked`.
    void setHsv(int hue, int sat, int val);
    void setWebColorsOnly(bool webOnly);

    QSize sizeHint() const override;

signals:
    void picked(int hue, int sat, int val);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;

private:
    void rebuildCache();
    void pickAt(const QPoint& pos);

    ColorAxis m_axis = ColorAxis::Hue;
    int m_hue = 0;
    int m_sat = 255;
    int m_val = 255;
    bool m_webOnly = false;

    QImage m_cache;
    ColorAxis m_cacheAxis = ColorAxis::Hue;
    int m_cacheValue = -1;
    bool m_cacheWebOnly = false;
};

/// The vertical ramp beside the field, with Photoshop's inward-pointing arrow
/// markers on either side.
class ColorRamp : public QWidget
{
    Q_OBJECT

public:
    explicit ColorRamp(QWidget* parent = nullptr);

    void setAxis(ColorAxis axis);
    void setHsv(int hue, int sat, int val);
    void setWebColorsOnly(bool webOnly);

    QSize sizeHint() const override;

signals:
    void picked(int hue, int sat, int val);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;

private:
    void rebuildCache();
    void pickAt(const QPoint& pos);
    QRect stripRect() const;

    ColorAxis m_axis = ColorAxis::Hue;
    int m_hue = 0;
    int m_sat = 255;
    int m_val = 255;
    bool m_webOnly = false;

    QImage m_cache;
    ColorAxis m_cacheAxis = ColorAxis::Hue;
    int m_cacheHue = -1;
    int m_cacheSat = -1;
    int m_cacheVal = -1;
    bool m_cacheWebOnly = false;
};

/// The new/current colour comparison swatch. Clicking the lower half reverts to
/// the colour the picker opened with, as Photoshop does.
class ColorCompare : public QWidget
{
    Q_OBJECT

public:
    explicit ColorCompare(QWidget* parent = nullptr);

    void setCurrentColor(const QColor& color);
    void setOriginalColor(const QColor& color);

    QSize sizeHint() const override;

signals:
    void originalClicked();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;

private:
    QColor m_current{Qt::black};
    QColor m_original{Qt::black};
};

/// Photoshop's inline Color Picker: the field + ramp, the new/current compare,
/// and the HSB/RGB/Lab/CMYK/hex fields with Only Web Colors. Used both inside
/// [`ColorPickerDialog`] and embedded in the Color panel.
class ColorPicker : public QWidget
{
    Q_OBJECT

public:
    explicit ColorPicker(QWidget* parent = nullptr);

    /// The colour currently shown.
    QColor color() const { return m_color; }

    /// Adopt `color` and refresh every control. Does NOT emit `colorChanged`
    /// (for reflecting external state without feeding a loop back).
    void setColor(const QColor& color);

    /// The colour the compare's lower swatch restores.
    void setOriginalColor(const QColor& color);

signals:
    /// Emitted when the user changes the colour (field, ramp, fields, hex, or
    /// the web-colors toggle) — not for `setColor`.
    void colorChanged(QColor color);

private slots:
    void onAxisChanged();
    void onPlanePicked(int hue, int sat, int val);
    void onHsbFieldsEdited();
    void onRgbFieldsEdited();
    void onLabFieldsEdited();
    void onHexEdited();
    void onHexCommitted();
    void onWebColorsToggled(bool on);
    void revertToOriginal();

private:
    void buildUi();
    void syncControls(QWidget* except = nullptr);
    void setHsv(int hue, int sat, int val);
    bool applyHex(const QString& text, bool normalize);
    void notify();
    ColorAxis currentAxis() const;

    // Both representations are kept, and which one is authoritative depends on
    // what the user last touched. HSV must persist because `QColor::hue()`
    // collapses to -1 for greys; RGB must persist because HSV is a lossy
    // intermediate at 8-bit precision.
    int m_hue = 0;
    int m_sat = 255;
    int m_val = 255;
    QColor m_color{Qt::black};
    QColor m_original{Qt::black};
    bool m_updating = false;

    ColorPlane* m_plane = nullptr;
    ColorRamp* m_ramp = nullptr;
    ColorCompare* m_compare = nullptr;

    QRadioButton* m_radioH = nullptr;
    QRadioButton* m_radioS = nullptr;
    QRadioButton* m_radioB = nullptr;
    QRadioButton* m_radioR = nullptr;
    QRadioButton* m_radioG = nullptr;
    QRadioButton* m_radioBlue = nullptr;

    QSpinBox* m_spinH = nullptr;
    QSpinBox* m_spinS = nullptr;
    QSpinBox* m_spinB = nullptr;
    QSpinBox* m_spinR = nullptr;
    QSpinBox* m_spinG = nullptr;
    QSpinBox* m_spinBlue = nullptr;
    QSpinBox* m_spinL = nullptr;
    QSpinBox* m_spinLabA = nullptr;
    QSpinBox* m_spinLabB = nullptr;
    QSpinBox* m_spinC = nullptr;
    QSpinBox* m_spinM = nullptr;
    QSpinBox* m_spinY = nullptr;
    QSpinBox* m_spinK = nullptr;

    QLineEdit* m_hex = nullptr;
    QCheckBox* m_webOnly = nullptr;
};

/// Photoshop's Color Picker dialog: a [`ColorPicker`] with OK / Cancel.
///
/// \code
///   const QColor picked =
///       ColorPickerDialog::getColor(startColor, this, tr("Count Group Color"));
///   if (picked.isValid()) { ... }
/// \endcode
///
/// Ported from photorust's `shell/src/dialogs/ColorPickerDialog.{h,cpp}`
/// (<https://github.com/perfecto25/photorust>). Given a canvas, a click on the
/// image takes its colour, as CS6's picker eyedropper does.
/// ponytail: the eyedropper reaches only the canvas (not the screen), and
/// Color Libraries are not wired.
class ColorPickerDialog : public QDialog
{
    Q_OBJECT

public:
    explicit ColorPickerDialog(const QColor& initial, QWidget* parent = nullptr,
                               const QString& title = {}, SwatchGrid* swatches = nullptr);
    ~ColorPickerDialog() override;

    QColor selectedColor() const;

    /// The grid Add to Swatches appends to. `PicturaMainWindow` registers its
    /// Swatches grid so every picker entry point can add to the panel; the
    /// pointer clears itself when the panel is destroyed.
    static void setDefaultSwatchGrid(SwatchGrid* grid);
    static SwatchGrid* defaultSwatchGrid();

    /// While the dialog is open, a canvas click (through `tools`' canvas
    /// sampler) takes `colorAt` of the clicked document pixel; an invalid
    /// colour (outside the document) is ignored.
    void sampleCanvas(ToolController* tools, std::function<QColor(const QPoint&)> colorAt);

    /// Modal convenience wrapper. Returns an invalid QColor if cancelled.
    /// With `tools` and `colorAt`, the image can be sampled (`sampleCanvas`).
    static QColor getColor(const QColor& initial, QWidget* parent = nullptr,
                           const QString& title = {}, ToolController* tools = nullptr,
                           std::function<QColor(const QPoint&)> colorAt = {});

private slots:
    void addToSwatches();

private:
    ColorPicker* m_picker = nullptr;
    QPointer<ToolController> m_tools;
    SwatchGrid* m_swatches = nullptr;
};

/// sRGB -> CIE L*a*b* (D50), the space Photoshop's Lab readout uses.
void rgbToLab(const QColor& color, double* l, double* a, double* b);
/// CIE L*a*b* (D50) -> sRGB, clamped into gamut.
QColor labToRgb(double l, double a, double b);
/// Snap each channel to the nearest web-safe value (multiples of 0x33).
QColor snapToWebColor(const QColor& color);
/// Whether every channel is already a multiple of 0x33.
bool isWebColor(const QColor& color);

} // namespace pictura
