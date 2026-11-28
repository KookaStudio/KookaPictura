#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QPointer>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include <array>
#include <functional>
#include <utility>

class QComboBox;
class QHBoxLayout;
class QPushButton;
class QSlider;
class QSpinBox;
class QToolButton;

namespace pictura {

class CurveWidget;
class PictureView;
class ToolController;
class TriangleSlider;

using CurveHistogram = std::array<int, 256>;
using CurveHistograms = std::array<CurveHistogram, 5>;

CurveHistograms curvesHistograms(PictureView* view, const QString& mode);
QByteArray curvesGrayPointBlock(const QByteArray& block, const QColor& color);
bool curveAutoEndpoints(const CurveHistogram& bins, int& low, int& high);

class CurvesControls : public QWidget {
    Q_OBJECT

public:
    using CurveSource = std::function<QString(int)>;
    using CanvasSample = std::function<QColor(const QPoint&)>;

    explicit CurvesControls(bool dialog, QWidget* parent = nullptr);
    ~CurvesControls() override;
    void setContext(const QString& mode, const CurveHistograms& histograms);
    void setCurveSource(CurveSource source) { source_ = std::move(source); }
    void setCanvas(ToolController* tools, CanvasSample sample);
    void reload();
    void disarm();
    void setTargeting(bool enabled);
    int channel() const;
    void applyAuto();
    void setAutoButton(QPushButton* button);
    void setClippingPreview(bool enabled);
    bool autoAvailable() const;
    void openDisplayOptions();
    CurveWidget* graph() const { return graph_; }

signals:
    void curveChanged(int channel, const QString& points);
    void grayPointPicked(const QColor& color);
    void clippingPreviewChanged(bool enabled);
    void gestureStarted();
    void gestureEnded();

private:
    enum class Pick { None, Target, Black, Gray, White };
    void refreshFields();
    void refreshGraph();
    void arm(Pick pick);
    int displayCode(int code) const;
    int storedCode(int display) const;
    bool reversed() const;

    struct DisplayOptions {
        bool ink = false;
        bool detailedGrid = false;
        bool channelOverlays = false;
        bool histogram = true;
        bool baseline = true;
        bool intersectionLines = false;
    };

    DisplayOptions displayOptions_;
    QString mode_;
    CurveHistograms histograms_{};
    QComboBox* channel_ = nullptr;
    QHBoxLayout* channelRow_ = nullptr;
    CurveWidget* graph_ = nullptr;
    QSpinBox* input_ = nullptr;
    QSpinBox* output_ = nullptr;
    QSlider* black_ = nullptr;
    QSlider* white_ = nullptr;
    QToolButton* pencil_ = nullptr;
    QToolButton* smooth_ = nullptr;
    QToolButton* target_ = nullptr;
    QToolButton* blackPick_ = nullptr;
    QToolButton* grayPick_ = nullptr;
    QToolButton* whitePick_ = nullptr;
    QPushButton* auto_ = nullptr;
    QToolButton* point_ = nullptr;
    QToolButton* clipping_ = nullptr;
    TriangleSlider* endpoints_ = nullptr;
    CurveSource source_;
    QPointer<ToolController> tools_;
    CanvasSample sample_;
    Pick pick_ = Pick::None;
    int targetInput_ = -1;
    int targetOutput_ = 0;
    qreal targetY_ = 0;
    bool targetGesture_ = false;
};

} // namespace pictura
