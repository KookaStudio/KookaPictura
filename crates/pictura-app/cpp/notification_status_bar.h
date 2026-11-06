#pragma once

#include <QtCore/QRect>
#include <QtWidgets/QStatusBar>

namespace pictura {

// QStatusBar paints its temporary message directly, with no child widget to
// style. This bar paints a light-yellow pill behind that message and draws the
// text in a dark colour, so a short notice stays readable on every theme. It is
// the single message painter: the base class's message text is suppressed.
class NotificationStatusBar : public QStatusBar {
    Q_OBJECT

public:
    explicit NotificationStatusBar(QWidget* parent = nullptr);

    // The rect the temporary-message banner owns, in this bar's coordinates.
    QRect messageRect() const;

protected:
    void paintEvent(QPaintEvent* event) override;
};

} // namespace pictura
