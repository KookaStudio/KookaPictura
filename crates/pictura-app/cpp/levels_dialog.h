#pragma once

#include "adjustment_dialog.h"

#include <QtCore/QPointer>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include <array>

class QComboBox;
class QDoubleSpinBox;
class QSpinBox;
class QToolButton;

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
    void setTrackInsets(int left, int right);
    void setInvertedAppearance(bool inverted);

signals:
    void valueChanged(int index, int value);
    void dragStarted();
    void dragEnded();
    void clippingThumbChanged(int index);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;
    bool event(QEvent* event) override;
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    int xForValue(int value) const;
    int valueForX(int index, int x) const;
    bool draggingInputEndpoint() const;
    void updateClipping(bool alt);
    void endDrag();

    struct Thumb {
        int min = 0;
        int max = 255;
        int value = 0;
        QColor color;
    };
    QVector<Thumb> thumbs_;
    int leftInset_ = LevelsStrip::kInset;
    int rightInset_ = LevelsStrip::kInset + 1;
    int keyboardThumb_ = 0;
    int dragging_ = -1;
    bool inverted_ = false;
    bool clipping_ = false;
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

    ~LevelsDialog() override;

    QWidget* controlForTest(const QString& key) const override;
    void reloadFromBlock() override { loadChannel(); }
    void setClippingCanvas(ImageView* canvas);
    void done(int result) override;

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    enum PointKind { BlackPoint, GrayPoint, WhitePoint };

    QHBoxLayout* makePointControls();
    void armPoint(PointKind kind);
    void disarmPoint();
    void chooseTarget(PointKind kind);
    void showSampleMenu(PointKind kind, const QPoint& globalPos);
    void pickPoint(PointKind kind, const QPointF& pos);
    void setClippingThumb(int index);
    void renderClipping();
    void clearClipping();

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
    std::array<QToolButton*, 3> points_{};
    std::array<int, 3> targets_{0, 128, 255};
    QPointer<ImageView> canvas_;
    QImage clippingSource_;
    QRect clippingSourceCrop_;
    int clippingSourceLevel_ = -1;
    int sampleSize_ = 1;
    int clippingThumb_ = -1;
    bool canvasControlsAvailable_ = false;
    bool grayscale_ = false;
    bool pointArmed_ = false;
    bool loading_ = false;
};

} // namespace pictura
