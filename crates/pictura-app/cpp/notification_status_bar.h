#pragma once

#include <QtCore/QRect>
#include <QtWidgets/QStatusBar>

namespace pictura {

// QStatusBar paints its temporary message directly, with no child widget to
// style. This bar paints a light-yellow pill behind that message and redraws
// the text in a dark colour, so a short notice stays readable on every theme.
class NotificationStatusBar : public QStatusBar {
    Q_OBJECT

public:
    explicit NotificationStatusBar(QWidget* parent = nullptr);

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QRect messageRect() const;
};

} // namespace pictura
