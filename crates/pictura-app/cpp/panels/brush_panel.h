#pragma once

#include <QtCore/QHash>
#include <QtGui/QImage>
#include <QtWidgets/QWidget>

#include "tool_options.h"

#include <functional>
#include <vector>

class QCheckBox;
class QComboBox;
class QLabel;
class QListWidget;
class QListWidgetItem;
class QStackedWidget;
class QToolButton;

namespace pictura {

class NumericField;
class ToolController;
class TipShapeView;

// Window > Brush (F5): the Brush Tip Shape page and the Shape Dynamics,
// Scattering, Color Dynamics, Transfer, and Brush Pose pages. A row of standard
// round tips, Size, Flip X / Flip Y, Angle, Roundness, Hardness, and Spacing,
// over the controller's brush fields that every painting tool shares, with the
// engine's stroke preview underneath. Each dynamics set has an enable checkbox
// and a padlock. The Build-up row is the Brush's Airbrush flag, shared with the
// options bar.
// ponytail: no sampled or erodible tips, no Brush Presets panel, Spacing is
// always on (no velocity spacing), and Texture and Dual Brush are the next
// changes.
class BrushPanel : public QWidget {
    Q_OBJECT

public:
    explicit BrushPanel(QWidget* parent = nullptr);

    void setController(ToolController* controller);
    // Re-read the controller's tip into the fields and redraw the preview.
    void refresh();

    // Self-test hook: the preview's current pixels.
    QImage previewForTest() const;

private:
    void refreshAirbrush();
    void buildDynamicsPages();
    void refreshDynamics();
    void updateDynamics(const std::function<void(BrushDynamics&)>& edit);
    void setSelectedSetLock(bool locked);
    QListWidgetItem* selectedSetItem() const;

    ToolController* controller_ = nullptr;
    PaintOptions* paint_ = nullptr;
    NumericField* size_ = nullptr;
    NumericField* angle_ = nullptr;
    NumericField* roundness_ = nullptr;
    NumericField* hardness_ = nullptr;
    NumericField* spacing_ = nullptr;
    QCheckBox* flipX_ = nullptr;
    QCheckBox* flipY_ = nullptr;
    QCheckBox* bristleEnabled_ = nullptr;
    QComboBox* bristleShape_ = nullptr;
    NumericField* bristleDensity_ = nullptr;
    NumericField* bristleLength_ = nullptr;
    NumericField* bristleThickness_ = nullptr;
    NumericField* bristleStiffness_ = nullptr;
    TipShapeView* shape_ = nullptr;
    QLabel* preview_ = nullptr;
    QListWidgetItem* buildUp_ = nullptr;
    QListWidget* sets_ = nullptr;
    QStackedWidget* pages_ = nullptr;
    QToolButton* lockButton_ = nullptr;
    QHash<QString, QWidget*> dynWidgets_;
    std::vector<std::function<void(const BrushDynamics&)>> dynRefreshers_;
};

} // namespace pictura
