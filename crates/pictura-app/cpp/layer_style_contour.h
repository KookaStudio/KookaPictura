#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>
#include <QtWidgets/QPushButton>

#include <array>

class QComboBox;
class QPaintEvent;

namespace pictura {

class CurveWidget;

// A contour curve as `"x,y x,y …"` on the 0–255 scale.
std::array<qreal, 256> contourLut(const QString& points);

// A small clickable curve preview. The Layer Style page opens the contour
// editor when it is clicked.
class ContourSwatch : public QPushButton {
    Q_OBJECT

public:
    explicit ContourSwatch(QWidget* parent = nullptr);

    void setPointsText(const QString& points);
    QString pointsText() const { return points_; }

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QString points_;
};

// The contour editor: a piecewise-linear CurveWidget, a small preset list,
// Reset, and OK / Cancel. The accepted value is applied by the caller.
class ContourEditorDialog : public QDialog {
    Q_OBJECT

public:
    explicit ContourEditorDialog(const QString& points, QWidget* parent = nullptr);

    QString pointsText() const;

private:
    CurveWidget* curve_ = nullptr;
    QComboBox* preset_ = nullptr;
};

} // namespace pictura
