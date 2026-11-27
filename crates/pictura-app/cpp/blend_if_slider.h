#pragma once

#include <QtWidgets/QWidget>

namespace pictura {

class BlendIfSlider : public QWidget {
    Q_OBJECT
public:
    explicit BlendIfSlider(QWidget* parent = nullptr);
    void setRange(int black, int white);
    int blackWord() const { return (values_[0] << 8) | values_[1]; }
    int whiteWord() const { return (values_[2] << 8) | values_[3]; }
    QSize sizeHint() const override;

signals:
    void rangeChanged(int black, int white);

protected:
    void paintEvent(QPaintEvent*) override;
    void mousePressEvent(QMouseEvent*) override;
    void mouseMoveEvent(QMouseEvent*) override;
    void mouseReleaseEvent(QMouseEvent*) override;

private:
    int position(int value) const;
    int valueAt(int x) const;
    int values_[4] = {0, 0, 255, 255};
    int original_[4] = {0, 0, 255, 255};
    int active_ = -1;
    int pressValue_ = 0;
    bool split_ = false;
};

} // namespace pictura
