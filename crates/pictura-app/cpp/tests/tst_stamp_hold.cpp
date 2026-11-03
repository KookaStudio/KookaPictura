// The stamp tools' held button (#799): Airbrush on Clone Stamp, Pattern Stamp,
// and History Brush keeps painting a still pointer; Art History Brush always
// does; every held stroke stops with the stroke.

#include <QtTest/QtTest>

#include "options_bar.h"
#include "selftest_paint_fixture.h"
#include "tool_options_json.h"

#include <QtGui/QPainter>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

namespace {
using paint_fixture::BrushState;
using paint_fixture::Fixture;

// Long enough for several 50 ms held dabs.
constexpr int kHoldWaitMs = 250;

const QPointF kAt(38, 10);

QImage cloneSeed()
{
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(0, 0, 30, 20, QColor(220, 30, 30));
    return seed;
}

QImage whiteSeed()
{
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(Qt::white);
    return seed;
}

// The canvas, one number per image state.
quint64 signature(const Fixture& f)
{
    quint64 sum = 0;
    for (int y = 0; y < 20; ++y) {
        for (int x = 0; x < 60; ++x) {
            sum = sum * 1099511628211ULL + f.view->sample_argb(x, y);
        }
    }
    return sum;
}

// Soft enough that a repeated dab shows: a large tip at low Flow.
void lowFlow(const Fixture& f)
{
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    paint->setSize(12);
    paint->setFlow(10);
}

void setAirbrush(const Fixture& f, bool pictura::StampOptions::*flag, bool on)
{
    auto* group = f.tools->optionsFor<pictura::StampGroup>(pictura::ToolId::CloneStamp);
    pictura::StampOptions o = group->get();
    o.*flag = on;
    group->set(o);
}

void press(const Fixture& f)
{
    f.canvas->mousePressed(kAt, Qt::LeftButton, 0);
}

void release(const Fixture& f)
{
    f.canvas->mouseReleased(kAt);
}

void armClone(pictura::PicturaMainWindow& frame, const Fixture& f, bool airbrush)
{
    lowFlow(f);
    setAirbrush(f, &pictura::StampOptions::cloneAirbrush, airbrush);
    frame.setActiveTool(pictura::ToolId::CloneStamp);
    f.drag({QPointF(8, 10)}, Qt::AltModifier);
}

void armPattern(pictura::PicturaMainWindow& frame, const Fixture& f, bool airbrush)
{
    lowFlow(f);
    setAirbrush(f, &pictura::StampOptions::patternAirbrush, airbrush);
    frame.setActiveTool(pictura::ToolId::PatternStamp);
}

// Black paint over the white opening state, for the History Brush to lift.
void armHistory(pictura::PicturaMainWindow& frame, const Fixture& f, bool airbrush)
{
    const QColor foreground = f.tools->foreground();
    f.tools->setForeground(Qt::black);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(30);
    frame.setActiveTool(pictura::ToolId::Brush);
    f.drag({kAt, kAt + QPointF(1, 0)});
    f.tools->setForeground(foreground);
    lowFlow(f);
    setAirbrush(f, &pictura::StampOptions::historyAirbrush, airbrush);
    frame.setActiveTool(pictura::ToolId::HistoryBrush);
}
} // namespace

class StampHoldTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cloneHoldAirbrushOff();
    void cloneHoldAirbrushOn();
    void patternHoldAirbrushOff();
    void patternHoldAirbrushOn();
    void historyHoldAirbrushOff();
    void historyHoldAirbrushOn();
    void artHistoryPaintsWhileHeld();
    void holdStopsOnRelease();
    void holdStopsOnToolSwitch();
    void holdStopsOnSessionCancel();
    void toolSwitchCommitsStroke();
    void airbrushTogglesPersist();

private:
    void expectStill(const Fixture& f);
    void expectBuilds(const Fixture& f, const char* label);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void StampHoldTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void StampHoldTest::expectStill(const Fixture& f)
{
    press(f);
    QTest::qWait(0);
    const quint64 before = signature(f);
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), before);
    release(f);
}

void StampHoldTest::expectBuilds(const Fixture& f, const char* label)
{
    const int base = f.view->history_index();
    press(f);
    const quint64 before = signature(f);
    QTRY_VERIFY(signature(f) != before);
    release(f);
    QVERIFY(f.committedOnce(base, label));
}

void StampHoldTest::cloneHoldAirbrushOff()
{
    Fixture f(*window_, cloneSeed(), QStringLiteral("pictura_hold_clone_off"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armClone(*window_, f, false);
    expectStill(f);
}

void StampHoldTest::cloneHoldAirbrushOn()
{
    Fixture f(*window_, cloneSeed(), QStringLiteral("pictura_hold_clone_on"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armClone(*window_, f, true);
    expectBuilds(f, "Clone Stamp");
}

void StampHoldTest::patternHoldAirbrushOff()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_hold_pattern_off"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armPattern(*window_, f, false);
    expectStill(f);
}

void StampHoldTest::patternHoldAirbrushOn()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_hold_pattern_on"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armPattern(*window_, f, true);
    expectBuilds(f, "Pattern Stamp");
}

void StampHoldTest::historyHoldAirbrushOff()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_hold_history_off"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armHistory(*window_, f, false);
    expectStill(f);
}

void StampHoldTest::historyHoldAirbrushOn()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_hold_history_on"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armHistory(*window_, f, true);
    expectBuilds(f, "History Brush");
}

void StampHoldTest::artHistoryPaintsWhileHeld()
{
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(Qt::red);
    Fixture f(*window_, seed, QStringLiteral("pictura_hold_art_history"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    const QColor foreground = f.tools->foreground();
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);

    // Paint the canvas white so it no longer matches the red source.
    f.tools->setForeground(Qt::white);
    paint->setSize(100);
    window_->setActiveTool(pictura::ToolId::Brush);
    f.drag({QPointF(30, 10), QPointF(31, 10)});
    f.tools->setForeground(foreground);
    paint->setSize(4);

    const auto redCount = [&f] {
        int n = 0;
        for (int y = 0; y < 20; ++y) {
            for (int x = 0; x < 60; ++x) {
                const QRgb px = f.view->sample_argb(x, y);
                n += qRed(px) > 200 && qGreen(px) < 100 ? 1 : 0;
            }
        }
        return n;
    };
    window_->setActiveTool(pictura::ToolId::ArtHistoryBrush);
    const int base = f.view->history_index();
    press(f);
    QTest::qWait(0);
    const int first = redCount();
    QTRY_VERIFY(redCount() > first);
    release(f);
    QVERIFY(f.committedOnce(base, "Art History Brush"));
}

void StampHoldTest::holdStopsOnRelease()
{
    Fixture f(*window_, cloneSeed(), QStringLiteral("pictura_hold_release"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armClone(*window_, f, true);
    press(f);
    release(f);
    QTest::qWait(0);
    const quint64 after = signature(f);
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), after);
}

void StampHoldTest::holdStopsOnToolSwitch()
{
    Fixture f(*window_, cloneSeed(), QStringLiteral("pictura_hold_switch"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armClone(*window_, f, true);
    press(f);
    window_->setActiveTool(pictura::ToolId::Move);
    QTest::qWait(0);
    const quint64 after = signature(f);
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), after);
}

void StampHoldTest::holdStopsOnSessionCancel()
{
    Fixture f(*window_, cloneSeed(), QStringLiteral("pictura_hold_cancel"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armClone(*window_, f, true);
    const int base = f.view->history_index();
    const quint64 before = signature(f);
    press(f);
    f.view->cancel_paint();
    f.tools->onSessionCancelled();
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), before);
    QCOMPARE(f.view->history_index(), base);
}

void StampHoldTest::toolSwitchCommitsStroke()
{
    Fixture f(*window_, cloneSeed(), QStringLiteral("pictura_hold_commit"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armClone(*window_, f, false);
    const int base = f.view->history_index();
    press(f);
    window_->setActiveTool(pictura::ToolId::Move);
    QVERIFY(f.committedOnce(base, "Clone Stamp"));
    QVERIFY(!f.view->is_painting());
}

void StampHoldTest::airbrushTogglesPersist()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_hold_toggles"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    auto* group = f.tools->optionsFor<pictura::StampGroup>(pictura::ToolId::CloneStamp);
    const auto click = [this](pictura::ToolId tool, const char* name) {
        window_->setActiveTool(tool);
        auto* button = window_->findChild<QToolButton*>(QString::fromLatin1(name));
        QVERIFY2(button, name);
        QVERIFY(!button->isChecked());
        button->click();
    };
    click(pictura::ToolId::CloneStamp, "optionsCloneAirbrush");
    QVERIFY(group->get().cloneAirbrush);
    QVERIFY(!group->get().patternAirbrush);
    click(pictura::ToolId::PatternStamp, "optionsPatternAirbrush");
    QVERIFY(group->get().patternAirbrush);
    QVERIFY(!group->get().historyAirbrush);
    click(pictura::ToolId::HistoryBrush, "optionsHistoryAirbrush");
    QVERIFY(group->get().historyAirbrush);

    const QJsonObject saved = pictura::toolOptionsToJson(*f.tools);
    pictura::StampOptions cleared = group->get();
    cleared.cloneAirbrush = cleared.patternAirbrush = cleared.historyAirbrush = false;
    group->set(cleared);
    pictura::applyToolOptionsJson(*f.tools, saved);
    QVERIFY(group->get().cloneAirbrush);
    QVERIFY(group->get().patternAirbrush);
    QVERIFY(group->get().historyAirbrush);

    // Leave the shared window's buttons as the other cases expect them.
    for (const char* name : {"optionsCloneAirbrush", "optionsPatternAirbrush",
                             "optionsHistoryAirbrush"}) {
        window_->findChild<QToolButton*>(QString::fromLatin1(name))->setChecked(false);
    }
}

QTEST_MAIN(StampHoldTest)
#include "tst_stamp_hold.moc"
