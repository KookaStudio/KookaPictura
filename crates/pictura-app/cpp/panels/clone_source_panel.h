#pragma once

#include "tools.h"

#include <QtWidgets/QWidget>

#include "tool_options.h"

#include <array>
#include <functional>

class QCheckBox;
class QToolButton;

namespace pictura {

class NumericField;
class ToolController;

// Window > Clone Source: the Clone Stamp's five source slots (each keeps its
// own Alt-clicked source), the Offset between destination and source, and the
// source transform (W / H with Maintain Aspect Ratio, Rotate, Flip Horizontal /
// Vertical, Reset Transform). Everything edits the controller's active slot.
// ponytail: the overlay (Show Overlay, Opacity, mode, Clipped, Auto Hide,
// Invert) is not drawn, and the Extended frame controls are not wired; both are
// shown disabled.
class CloneSourcePanel : public QWidget {
    Q_OBJECT

public:
    explicit CloneSourcePanel(QWidget* parent = nullptr);

    void setController(ToolController* controller);
    // Re-read the active slot into the controls.
    void refresh();

private:
    void editSlot(const std::function<void(CloneSource&)>& edit);

    ToolController* controller_ = nullptr;
    CloneSourceOptions* clone_ = nullptr;
    std::array<QToolButton*, 5> slots_{};
    NumericField* offsetX_ = nullptr;
    NumericField* offsetY_ = nullptr;
    NumericField* width_ = nullptr;
    NumericField* height_ = nullptr;
    NumericField* angle_ = nullptr;
    QToolButton* link_ = nullptr;
    QToolButton* flipH_ = nullptr;
    QToolButton* flipV_ = nullptr;
};

} // namespace pictura
