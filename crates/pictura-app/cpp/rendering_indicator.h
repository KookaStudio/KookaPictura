#pragma once

#include <QtCore/QPointer>
#include <QtWidgets/QWidget>

#include <functional>

class QPaintEvent;
class QTimer;

namespace pictura {

// The canvas's "Rendering…" badge (#1608): shown over the canvas's top-right
// corner once the display tiles the view asked for have been pending for
// `kDelayMs`, hidden as soon as none are. It is laid over the canvas from the
// canvas's host widget, not drawn as the canvas's child, so a render or grab of
// the canvas never contains it. Mouse-transparent and focus-less, so the
// canvas keeps every press. No timer runs while it is hidden and nothing is
// pending.
class RenderingIndicator : public QWidget {
    Q_OBJECT

public:
    static constexpr int kDelayMs = 250;
    static constexpr int kTickMs = 80;

    explicit RenderingIndicator(QWidget* canvas);

    // Where the pending state is read; empty reads as nothing pending.
    void setPendingSource(std::function<bool()> pending);
    // Re-read the pending state after the canvas painted: arm the delay when
    // tiles are pending, hide when none are.
    void poke();

    bool delayActiveForTest() const;
    bool animationActiveForTest() const;

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;
    void paintEvent(QPaintEvent* event) override;

private:
    bool pending() const;
    void place();
    void showNow();
    void stop();
    void tick();

    QPointer<QWidget> canvas_;
    std::function<bool()> pending_;
    QTimer* delay_ = nullptr;
    QTimer* animation_ = nullptr;
    int angle_ = 0;
};

} // namespace pictura
