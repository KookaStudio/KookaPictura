#pragma once
#include <QtCore/QPointer>
#include <QtGui/QColor>
#include <array>
namespace pictura {
class PictureView;
struct QuickMaskUiState {
    bool syncing = false;
    bool selected = false;
    QColor color { Qt::red };
    int opacity = 50;
    QPointer<PictureView> settingsView;
    QPointer<PictureView> colorView;
    bool colorsActive = false;
    bool returnMask = false;
    std::array<double, 3> returnForeground{}, returnBackground{};
};
} // namespace pictura
