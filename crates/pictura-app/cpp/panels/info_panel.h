#pragma once

#include <QtCore/QPointF>
#include <QtCore/QStringList>
#include <QtWidgets/QWidget>

class QColor;
class QEvent;
class QGridLayout;
class QLabel;
class QToolButton;
class QVBoxLayout;

namespace pictura {

class PictureView;

class InfoPanel : public QWidget {
    Q_OBJECT

public:
    explicit InfoPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void setCursorPosition(const QPointF& imagePos);
    // The Ruler tool's mode: the panel shows A/L and points W/H at the ruler.
    void setRulerMode(bool on);
    bool rulerMode() const { return rulerMode_; }
    // The active tool and its keyboard hints, shown as their own readout row.
    void setToolInfo(const QString& name, const QStringList& hints);
    void refresh();

    QString samplerTextForTest() const;
    QString colorBlockTextForTest(int index = 0) const;
    QString colorFooterForTest(int index = 0) const;
    QString colorModeForTest(int index = 0) const;
    QStringList colorMenuTextsForTest(int index = 0) const;
    QString positionTextForTest() const;
    QString sizeTextForTest() const;
    QString rulerTextForTest() const;
    QString docTextForTest() const;
    QString toolTextForTest() const;
    bool sizeBlockHasMenuForTest() const;
    void setColorModeForTest(int index, const QString& mode);
    void setMeasurementUnitForTest(int index, const QString& unit);
    void setBitDepthForTest(int index, int bits);

private:
    enum class ColorReadout {
        ActualColor,
        ProofColor,
        Grayscale,
        Rgb,
        Hsb,
        Cmyk,
        Lab,
        TotalInk,
        Opacity
    };
    enum class MeasureUnit { Pixels, Inches, Centimeters, Millimeters, Points, Picas, Percent };
    enum class MenuKind { None, Color, Unit };

    struct Readout {
        QWidget* widget = nullptr;
        QToolButton* button = nullptr;
        QGridLayout* keysHost = nullptr;
        QVBoxLayout* valuesHost = nullptr;
        QStringList keyNames;
        QList<QLabel*> values;
        QLabel* footer = nullptr;
        ColorReadout colorMode = ColorReadout::ActualColor;
        MeasureUnit unit = MeasureUnit::Pixels;
        int bits = 8;
    };

    Readout* addReadout(QGridLayout* grid, int row, int column, const QStringList& keys,
                        const QString& iconId, const QString& footer, MenuKind menu);
    bool eventFilter(QObject* watched, QEvent* event) override;
    void rebuildRows(Readout* readout, const QStringList& keys);
    void setValues(Readout* readout, const QStringList& values);
    void applyColorMode(Readout* readout, ColorReadout mode);
    void applyUnit(Readout* readout, MeasureUnit unit);
    void setBitDepth(Readout* readout, int bits);
    void refreshColorBlock(Readout* readout, const QColor& color);
    void refresh(bool full);
    void refreshDocumentRows(int width, int height);
    void rebuildTopRight();
    void refreshToolLabel();
    QString blockText(const Readout* readout) const;
    QString colorModeName(ColorReadout mode) const;
    QString formatChannel(int value, int bits) const;
    QString formatMeasure(double value, MeasureUnit unit, double percentBase) const;

    PictureView* view_ = nullptr;
    QPointF cursor_;
    bool rulerMode_ = false;
    QGridLayout* grid_ = nullptr;
    Readout* topLeft_ = nullptr;
    Readout* topRight_ = nullptr;
    Readout* position_ = nullptr;
    Readout* size_ = nullptr;
    QLabel* docLabel_ = nullptr;
    QLabel* samplersLabel_ = nullptr;
    QLabel* toolLabel_ = nullptr;
    QString toolName_;
    QStringList toolHints_;
};

} // namespace pictura
