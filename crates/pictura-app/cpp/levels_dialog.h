#pragma once

#include "adjustment_dialog.h"

#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include <array>

class QComboBox;
class QDoubleSpinBox;
class QSpinBox;

namespace pictura {

// The strip above a Levels slider: the input histogram (black bars on white)
// or the output ramp (black to white). Both inset by `kInset` so a slider
// thumb at 0 or 255 sits on the strip's edge.
class LevelsStrip : public QWidget {
    Q_OBJECT

public:
    static constexpr int kInset = 6;

    LevelsStrip(bool ramp, int height, QWidget* parent = nullptr);
    void setBins(const std::array<int, 256>& bins);

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    bool ramp_ = false;
    std::array<int, 256> bins_{};
};

// Draggable triangle thumbs under a LevelsStrip, over 0..255. Each thumb keeps
// its own range, so the owner can stop black passing white.
class TriangleSlider : public QWidget {
    Q_OBJECT

public:
    explicit TriangleSlider(int count, QWidget* parent = nullptr);
    void setRange(int index, int min, int max);
    void setValue(int index, int value);
    void setColor(int index, const QColor& color);
    int value(int index) const;

signals:
    void valueChanged(int index, int value);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;

private:
    int xForValue(int value) const;
    int valueForX(int index, int x) const;

    struct Thumb {
        int min = 0;
        int max = 255;
        int value = 0;
        QColor color;
    };
    QVector<Thumb> thumbs_;
    int dragging_ = -1;
};

// Image > Adjustments > Levels in CS6's layout: Preset and Channel menus, the
// input histogram with black / gamma / white thumbs and fields, the output ramp
// with its two thumbs and fields, and OK / Cancel / Auto / Options… with
// Preview down the right. Each channel (RGB, Red, Green, Blue) edits its own
// `levl` record. Ported from photorust's LevelsDialog.
// ponytail: Auto clips 0.1% off each end of the channel's histogram (CS6's
// default clip, not its full Auto Color Correction options); Options… stays
// disabled; the presets are photorust's composite-only values; the histogram
// is of the canvas composite, not the active layer alone.
class LevelsDialog : public AdjustmentDialog {
    Q_OBJECT

public:
    LevelsDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                 QWidget* parent = nullptr);

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override { loadChannel(); }

private:
    // The current channel's key prefix: "", "red.", "green.", "blue.".
    QString prefix() const;
    // Reload every field and thumb from the block for the current channel.
    void loadChannel();
    // Write one of the current channel's values; a refused edit reloads.
    void edit(const QString& name, double value);
    void applyPreset(int index);
    void applyAuto() override;
    void markCustom();
    int gammaPosition(double gamma) const;

    QComboBox* preset_ = nullptr;
    QComboBox* channel_ = nullptr;
    LevelsStrip* histogram_ = nullptr;
    TriangleSlider* input_ = nullptr;
    TriangleSlider* output_ = nullptr;
    QSpinBox* inBlack_ = nullptr;
    QDoubleSpinBox* gamma_ = nullptr;
    QSpinBox* inWhite_ = nullptr;
    QSpinBox* outBlack_ = nullptr;
    QSpinBox* outWhite_ = nullptr;
    bool loading_ = false;
};

} // namespace pictura
