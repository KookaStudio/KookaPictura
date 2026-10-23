#pragma once

#include <QtWidgets/QWidget>

class QDoubleSpinBox;
class QSpinBox;

namespace pictura {

// The nine Local Adaptation numeric controls shared by the HDR Toning and HDR
// Conversion dialogs. The preset menu and live preview stay in the HDR Toning
// dialog; this widget owns only the rows so their ranges cannot drift.
class HdrToningControls : public QWidget {
    Q_OBJECT

public:
    struct Values {
        int radius = 187;
        double strength = 4.0;
        double gamma = 0.99;
        double exposure = 0.0;
        int detail = 30;
        int shadow = 0;
        int highlight = 0;
        int vibrance = 0;
        int saturation = 20;
    };

    explicit HdrToningControls(QWidget* parent = nullptr);

    Values values() const;
    void setValues(const Values& values);

signals:
    void edited();

private:
    bool loading_ = false;
    QSpinBox* radius_ = nullptr;
    QDoubleSpinBox* strength_ = nullptr;
    QDoubleSpinBox* gamma_ = nullptr;
    QDoubleSpinBox* exposure_ = nullptr;
    QSpinBox* detail_ = nullptr;
    QSpinBox* shadow_ = nullptr;
    QSpinBox* highlight_ = nullptr;
    QSpinBox* vibrance_ = nullptr;
    QSpinBox* saturation_ = nullptr;
};

} // namespace pictura
