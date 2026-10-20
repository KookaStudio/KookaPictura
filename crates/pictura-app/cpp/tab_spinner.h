#pragma once

#include <QtCore/QString>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

class QTimer;

namespace pictura {

// The loading document tab's right-side button: an animated arc while the open
// runs, the normal close glyph on hover, and a cancel request on click.
class TabSpinner : public QWidget {
    Q_OBJECT

public:
    explicit TabSpinner(QWidget* parent = nullptr);

    // Test accessors: the current arc angle and whether the close glyph shows.
    int angleForTest() const { return angle_; }
    bool hoveringForTest() const { return hovered_; }

    void setOpeningName(const QString& name);

Q_SIGNALS:
    void cancelRequested();

protected:
    void paintEvent(QPaintEvent* event) override;
    void enterEvent(QEnterEvent* event) override;
    void leaveEvent(QEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void showEvent(QShowEvent* event) override;
    void hideEvent(QHideEvent* event) override;

private:
    QTimer* timer_ = nullptr;
    int angle_ = 0;
    bool hovered_ = false;
    bool pressed_ = false;
};

// A ready tab's close button. A custom tab button (not the QSS close-button
// subcontrol) so the tab's size hint includes it and it cannot be clipped.
class TabCloseButton : public QToolButton {
    Q_OBJECT

public:
    explicit TabCloseButton(QWidget* parent = nullptr);

Q_SIGNALS:
    void closeRequested();
};

} // namespace pictura
