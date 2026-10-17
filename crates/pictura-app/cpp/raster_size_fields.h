#pragma once

#include <QtCore/QSize>
#include <QtCore/QSizeF>
#include <QtWidgets/QGroupBox>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;

namespace pictura {

// The Image Size group the Import PDF and Rasterize Generic EPS Format dialogs
// share: an uncompressed-size title, Width and Height with units, Constrain
// Proportions, Resolution, Mode, and (Import PDF only) Bit Depth.
//
// The physical size in inches is the truth: Width/Height edits set it (in any
// unit), Resolution turns it into pixels, and a new source box resets it to
// the box's size. With Constrain Proportions on, the box's aspect ratio binds
// Width and Height.
class RasterSizeFields : public QGroupBox {
    Q_OBJECT

public:
    explicit RasterSizeFields(bool withBitDepth, QWidget* parent = nullptr);

    // The source box in points; resets Width and Height to it.
    void setSourceSize(QSizeF points);
    QSizeF sourceSize() const { return source_; }
    // The output in pixels at the current resolution.
    QSize pixelSize() const;
    double ppi() const { return ppi_; }
    bool perCm() const;
    // "grayscale", "rgb", "cmyk", or "lab".
    QString mode() const;
    int depth() const;
    // Enable or disable Width, Height, their units, Constrain, and
    // Resolution; Mode and Bit Depth stay live.
    void setSizeEnabled(bool on);

    // Test seam: set a control by object name ("width", "height", "widthUnit",
    // "resolution", "resolutionUnit", "constrain", "mode", "depth") as the
    // user would.
    void setFieldForTest(const QString& name, const QString& value);
    QString fieldForTest(const QString& name) const;

signals:
    void changed();

private:
    void sync();
    void widthEdited();
    void heightEdited();
    void resolutionEdited();
    // Inches per unit of Width/Height unit `unit` at the current resolution.
    double inchesPerUnit(int unit) const;

    QSizeF source_{72, 72};
    double widthIn_ = 1;
    double heightIn_ = 1;
    double ppi_ = 300;
    bool updating_ = false;

    QDoubleSpinBox* width_ = nullptr;
    QDoubleSpinBox* height_ = nullptr;
    QComboBox* widthUnit_ = nullptr;
    QComboBox* heightUnit_ = nullptr;
    QCheckBox* constrain_ = nullptr;
    QDoubleSpinBox* resolution_ = nullptr;
    QComboBox* resolutionUnit_ = nullptr;
    QComboBox* mode_ = nullptr;
    QComboBox* depth_ = nullptr;
};

} // namespace pictura
