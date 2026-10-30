#pragma once

#include <QtWidgets/QFrame>

class QLabel;
class QPushButton;

namespace pictura {

// Asks the person about one agent request. Not a dialog: a modal would stop the
// window, and a focused button could be pressed by keys meant for the canvas.
class ControlApprovalBar : public QFrame {
    Q_OBJECT

public:
    // Allow arms after this long, so a click aimed elsewhere cannot land on it.
    static constexpr int kArmDelayMs = 500;

    explicit ControlApprovalBar(QWidget* window);

    void ask(const QString& question);
    QString question() const;
    QPushButton* allowButton() const { return allow_; }
    QPushButton* denyButton() const { return deny_; }

signals:
    void answered(bool allow);

private:
    QLabel* label_;
    QPushButton* allow_;
    QPushButton* deny_;
};

} // namespace pictura
