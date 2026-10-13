#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QKeyEvent>
#include <QtGui/QShortcut>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStatusBar>

#include <functional>
#include <memory>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "panels/panel_column.h"
#include "session.h"
#include "tools.h"

#include "qt_test_support.h"

namespace {

using ScreenMode = pictura::PicturaMainWindow::ScreenMode;

// Answers the next modal dialog from the dialog's own event loop and counts it
// in `*seen`. Destroy the timer once the action that may open the dialog has
// returned.
std::unique_ptr<QTimer> answerNextModal(std::function<void(QWidget*)> answer, int* seen)
{
    auto timer = std::make_unique<QTimer>();
    timer->setInterval(5);
    QTimer* raw = timer.get();
    QObject::connect(raw, &QTimer::timeout, [raw, answer, seen]() {
        if (QWidget* modal = QApplication::activeModalWidget()) {
            raw->stop();
            ++*seen;
            answer(modal);
        }
    });
    timer->start();
    return timer;
}

QMessageBox* fullScreenBox(QWidget* modal)
{
    auto* box = qobject_cast<QMessageBox*>(modal);
    return box && box->objectName() == QStringLiteral("fullScreenMessage") ? box : nullptr;
}

QPushButton* buttonWithText(QMessageBox* box, const QString& text)
{
    for (QAbstractButton* button : box->buttons()) {
        if (button->text() == text) {
            return qobject_cast<QPushButton*>(button);
        }
    }
    return nullptr;
}

void setAdvisorySuppressed(bool suppressed)
{
    pictura::SessionState state = pictura::loadSession();
    state.showFullScreenMessage = !suppressed;
    QVERIFY(pictura::saveSession(state));
}

void sendKey(QWidget* widget, int key, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    QKeyEvent event(QEvent::KeyPress, key, mods);
    QApplication::sendEvent(widget, &event);
}

void activateTabShortcut(pictura::PicturaMainWindow& frame, bool shift)
{
    const QKeySequence want =
        shift ? QKeySequence(Qt::SHIFT | Qt::Key_Tab) : QKeySequence(Qt::Key_Tab);
    for (QShortcut* shortcut : frame.findChildren<QShortcut*>()) {
        if (shortcut->key() == want) {
            shortcut->activated();
            return;
        }
    }
    QFAIL("the frame has the Tab panel shortcut");
}

} // namespace

class ScreenModesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void advisoryShownAndAccepted();
    void advisoryCancelKeepsStandard();
    void dontShowAgainPersists();
    void noAdvisoryBetweenFullModes();
    void fAndEscReturnToStandard();
    void fReachesScreenModeThroughShortcutMap();
    void spaceFCyclesCanvasColor();
    void toolCancelWinsOverEsc();
    void fullWithMenuBarKeepsPanels();
    void tabRevealsFullScreenChrome();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ScreenModesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("ScreenModes"), 128, 128,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    window_->resize(1100, 700);
    window_->show();
    QCoreApplication::processEvents();
}

void ScreenModesTest::advisoryShownAndAccepted()
{
    setAdvisorySuppressed(false);
    int seen = 0;
    auto timer = answerNextModal(
        [](QWidget* modal) {
            QMessageBox* box = fullScreenBox(modal);
            QVERIFY(box != nullptr);
            QCOMPARE(box->windowTitle(), QStringLiteral("Message"));
            QVERIFY(box->text().contains(QStringLiteral("panels are hidden")));
            QVERIFY(box->text().contains(QStringLiteral("pressing 'F' or Esc")));
            QPushButton* proceed = buttonWithText(box, QStringLiteral("Full Screen"));
            QVERIFY(proceed != nullptr);
            QVERIFY(buttonWithText(box, QStringLiteral("Cancel")) != nullptr);
            QCheckBox* dontShow = box->checkBox();
            QVERIFY(dontShow != nullptr);
            QCOMPARE(dontShow->text(), QStringLiteral("Don't show again"));
            QVERIFY(!dontShow->isChecked());
            proceed->click();
        },
        &seen);
    QAction* full =
        window_->registry()->action(QString::fromLatin1(pictura::command_ids::ViewScreenModeFull));
    QVERIFY(full != nullptr);
    full->trigger();
    timer.reset();
    QCOMPARE(seen, 1);
    QCOMPARE(window_->screenMode(), ScreenMode::Full);
    window_->setScreenMode(ScreenMode::Standard);
}

void ScreenModesTest::advisoryCancelKeepsStandard()
{
    setAdvisorySuppressed(false);
    int seen = 0;
    auto timer = answerNextModal(
        [](QWidget* modal) {
            QMessageBox* box = fullScreenBox(modal);
            QVERIFY(box != nullptr);
            QPushButton* cancel = buttonWithText(box, QStringLiteral("Cancel"));
            QVERIFY(cancel != nullptr);
            cancel->click();
        },
        &seen);
    QAction* withMenuBar = window_->registry()->action(
        QString::fromLatin1(pictura::command_ids::ViewScreenModeFullWithMenuBar));
    QVERIFY(withMenuBar != nullptr);
    withMenuBar->trigger();
    timer.reset();
    QCOMPARE(seen, 1);
    QCOMPARE(window_->screenMode(), ScreenMode::Standard);
}

void ScreenModesTest::dontShowAgainPersists()
{
    setAdvisorySuppressed(false);
    int seen = 0;
    auto timer = answerNextModal(
        [](QWidget* modal) {
            QMessageBox* box = fullScreenBox(modal);
            QVERIFY(box != nullptr);
            QCheckBox* dontShow = box->checkBox();
            QVERIFY(dontShow != nullptr);
            dontShow->setChecked(true);
            buttonWithText(box, QStringLiteral("Full Screen"))->click();
        },
        &seen);
    window_->requestScreenMode(ScreenMode::Full);
    timer.reset();
    QCOMPARE(seen, 1);
    QCOMPARE(window_->screenMode(), ScreenMode::Full);
    QVERIFY(!pictura::loadSession().showFullScreenMessage);
    // Suppressed: the next request switches without a modal.
    window_->setScreenMode(ScreenMode::Standard);
    window_->requestScreenMode(ScreenMode::FullWithMenuBar);
    QCOMPARE(window_->screenMode(), ScreenMode::FullWithMenuBar);
    window_->setScreenMode(ScreenMode::Standard);
}

void ScreenModesTest::noAdvisoryBetweenFullModes()
{
    setAdvisorySuppressed(false);
    int seen = 0;
    auto timer = answerNextModal([](QWidget* modal) { modal->close(); }, &seen);
    window_->setScreenMode(ScreenMode::FullWithMenuBar);
    window_->requestScreenMode(ScreenMode::Full);
    timer.reset();
    QCOMPARE(seen, 0);
    QCOMPARE(window_->screenMode(), ScreenMode::Full);
    window_->setScreenMode(ScreenMode::Standard);
}

void ScreenModesTest::fAndEscReturnToStandard()
{
    setAdvisorySuppressed(true);
    sendKey(window_.get(), Qt::Key_F);
    QCOMPARE(window_->screenMode(), ScreenMode::FullWithMenuBar);
    sendKey(window_.get(), Qt::Key_F);
    QCOMPARE(window_->screenMode(), ScreenMode::Full);
    sendKey(window_.get(), Qt::Key_F);
    QCOMPARE(window_->screenMode(), ScreenMode::Standard);
    sendKey(window_.get(), Qt::Key_F, Qt::ShiftModifier);
    QCOMPARE(window_->screenMode(), ScreenMode::Full);
    sendKey(window_.get(), Qt::Key_Escape);
    QCOMPARE(window_->screenMode(), ScreenMode::Standard);
}

void ScreenModesTest::fReachesScreenModeThroughShortcutMap()
{
    // QTest::keyClick routes through the shortcut map, where a multi-key
    // sequence starting with F would swallow the press (#383).
    setAdvisorySuppressed(true);
    window_->setScreenMode(ScreenMode::Standard);
    for (QShortcut* shortcut : window_->findChildren<QShortcut*>()) {
        QVERIFY2(shortcut->key()[0] != QKeyCombination(Qt::Key_F), "no shortcut claims bare F");
    }
    QTest::keyClick(window_.get(), Qt::Key_F);
    QCOMPARE(window_->screenMode(), ScreenMode::FullWithMenuBar);
    QTest::keyClick(window_.get(), Qt::Key_F);
    QCOMPARE(window_->screenMode(), ScreenMode::Full);
    QTest::keyClick(window_.get(), Qt::Key_Escape);
    QCOMPARE(window_->screenMode(), ScreenMode::Standard);
}

void ScreenModesTest::spaceFCyclesCanvasColor()
{
    window_->setScreenMode(ScreenMode::Standard);
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    const QColor before = canvas->canvasColor();
    sendKey(window_.get(), Qt::Key_Space);
    QVERIFY(canvas->spacePanForTest());
    sendKey(window_.get(), Qt::Key_F);
    QCOMPARE(window_->screenMode(), ScreenMode::Standard);
    QVERIFY(canvas->canvasColor() != before);
    sendKey(window_.get(), Qt::Key_F, Qt::ShiftModifier);
    QCOMPARE(canvas->canvasColor(), before);
    QKeyEvent release(QEvent::KeyRelease, Qt::Key_Space, Qt::NoModifier);
    QApplication::sendEvent(window_.get(), &release);
    QVERIFY(!canvas->spacePanForTest());
}

void ScreenModesTest::toolCancelWinsOverEsc()
{
    setAdvisorySuppressed(true);
    window_->setScreenMode(ScreenMode::Full);
    window_->setActiveTool(pictura::ToolId::RotateView);
    sendKey(window_.get(), Qt::Key_Escape);
    QCOMPARE(window_->screenMode(), ScreenMode::Full);
    window_->setActiveTool(pictura::ToolId::Move);
    sendKey(window_.get(), Qt::Key_Escape);
    QCOMPARE(window_->screenMode(), ScreenMode::Standard);
}

void ScreenModesTest::fullWithMenuBarKeepsPanels()
{
    setAdvisorySuppressed(true);
    pictura::PanelColumn* tools = window_->toolsColumn();
    pictura::PanelColumn* panels = window_->panelColumn();
    QVERIFY(tools != nullptr);
    QVERIFY(panels != nullptr);

    // CS6 keeps the Tools panel, Options bar, and panels in this mode; only the
    // document-window chrome (tab, scroll bars, status bar) is hidden.
    window_->setScreenMode(ScreenMode::FullWithMenuBar);
    QVERIFY(window_->menuBar()->isVisible());
    QVERIFY(tools->isVisible());
    QVERIFY(panels->isVisible());
    QVERIFY(!window_->statusBar()->isVisible());

    // Tab toggles the panels exactly as in Standard mode.
    activateTabShortcut(*window_, false);
    QVERIFY(!tools->isVisible());
    QVERIFY(!panels->isVisible());
    QVERIFY(window_->menuBar()->isVisible());
    activateTabShortcut(*window_, false);
    QVERIFY(tools->isVisible());
    QVERIFY(panels->isVisible());

    window_->setScreenMode(ScreenMode::Standard);
    QVERIFY(window_->menuBar()->isVisible());
    QVERIFY(window_->statusBar()->isVisible());
    QVERIFY(tools->isVisible());
    QVERIFY(panels->isVisible());
}

void ScreenModesTest::tabRevealsFullScreenChrome()
{
    setAdvisorySuppressed(true);
    pictura::PanelColumn* tools = window_->toolsColumn();
    pictura::PanelColumn* panels = window_->panelColumn();
    QVERIFY(tools != nullptr);
    QVERIFY(panels != nullptr);
    QVERIFY(tools->isVisible());
    QVERIFY(panels->isVisible());

    window_->setScreenMode(ScreenMode::Full);
    QVERIFY(!tools->isVisible());
    QVERIFY(!panels->isVisible());
    QVERIFY(!window_->menuBar()->isVisible());
    activateTabShortcut(*window_, false);
    QVERIFY(tools->isVisible());
    QVERIFY(panels->isVisible());
    QVERIFY(window_->menuBar()->isVisible());
    activateTabShortcut(*window_, false);
    QVERIFY(!tools->isVisible());
    QVERIFY(!panels->isVisible());
    QVERIFY(!window_->menuBar()->isVisible());
    activateTabShortcut(*window_, true);
    QVERIFY(tools->isVisible());
    QVERIFY(window_->menuBar()->isVisible());
    // The reveal is transient: Standard mode restores the pre-full state.
    window_->setScreenMode(ScreenMode::Standard);
    QVERIFY(tools->isVisible());
    QVERIFY(panels->isVisible());
    QVERIFY(window_->menuBar()->isVisible());

    // A hidden standard layout stays hidden after a full-screen Tab reveal.
    window_->setPanelsHidden(true);
    window_->setScreenMode(ScreenMode::Full);
    activateTabShortcut(*window_, false);
    QVERIFY(tools->isVisible());
    window_->setScreenMode(ScreenMode::Standard);
    QVERIFY(!tools->isVisible());
    QVERIFY(!panels->isVisible());
    window_->setPanelsHidden(false);
}

QTEST_MAIN(ScreenModesTest)
#include "tst_screen_modes.moc"
