#include <QtTest/QtTest>

#include "panels/clone_source_panel.h"
#include "panels/numeric_field.h"
#include "selftest_paint_fixture.h"
#include "qt_test_support.h"
#include "api/paint_bridge.h"
#include <QtCore/QElapsedTimer>
#include <QtCore/QTimer>

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QToolButton>
#include <QtGui/QPainter>
#include <utility>

class CloneSourceOverlayTest : public QObject {
    Q_OBJECT
private slots:
    void slotsKeepSources();
    void flipAndOffset();
    void controlsAndSlots();
    void previewMapping();
    void nonAlignedMapping();
    void healingTransformedSampling();
    void previewEligibility();
    void autoHide();
    void brushClipping();
    void rotatedBrushClipping();
    void healingSourceCancelled();
    void blendAndOpacity();
    void sourceAlpha();
    void grayAlignment();
    void sourceStaysSnapshot();
    void asynchronousSourceInvalidation();
private:
    pictura::test::ScopedStateHome stateHome_;
};

void CloneSourceOverlayTest::slotsKeepSources()
{
    QVERIFY(stateHome_.isValid());
    auto frame = pictura::test::makeMainWindow();
    QVERIFY(frame);
    QImage image(64, 64, QImage::Format_RGB32);
    image.fill(Qt::white);
    paint_fixture::Fixture f(*frame, image, QStringLiteral("clone_overlay_slots"));
    QVERIFY(f.ok());
    frame->setActiveTool(pictura::ToolId::CloneStamp);
    auto* sourceSlots = f.tools->optionsFor<pictura::CloneSourceOptions>(pictura::ToolId::CloneStamp);
    QVERIFY(sourceSlots);
    sourceSlots->setSlot(1);
    f.drag({QPointF(10, 10)}, Qt::AltModifier);
    QVERIFY(sourceSlots->active().hasSource);
    sourceSlots->setSlot(0);
    const int before = f.view->history_index();
    f.drag({QPointF(30, 30)});
    QCOMPARE(f.view->history_index(), before);
    sourceSlots->setSlot(1);
    QCOMPARE(sourceSlots->active().source, QPoint(10, 10));
}

void CloneSourceOverlayTest::flipAndOffset()
{
    QVERIFY(stateHome_.isValid());
    auto frame = pictura::test::makeMainWindow();
    QVERIFY(frame);
    QImage image(60, 20, QImage::Format_RGB32);
    image.fill(Qt::white);
    QPainter painter(&image);
    painter.fillRect(0, 0, 5, 20, QColor(220, 30, 30));
    painter.fillRect(5, 0, 5, 20, QColor(30, 200, 30));
    painter.end();
    paint_fixture::Fixture f(*frame, image, QStringLiteral("clone_overlay_flip"));
    QVERIFY(f.ok());
    const paint_fixture::BrushState brush(f.tools);
    frame->setActiveTool(pictura::ToolId::CloneStamp);
    auto* sourceSlots = f.tools->optionsFor<pictura::CloneSourceOptions>(pictura::ToolId::CloneStamp);
    QVERIFY(sourceSlots);
    sourceSlots->setSlot(0);
    sourceSlots->setActive(pictura::CloneSource{});
    f.drag({QPointF(5, 10)}, Qt::AltModifier);
    auto* flip = frame->findChild<QToolButton*>(QStringLiteral("cloneSourceFlipH"));
    auto* offset = frame->findChild<pictura::NumericField*>(QStringLiteral("cloneSourceOffsetX"));
    QVERIFY(flip && offset);
    flip->click();
    const int before = f.view->history_index();
    f.drag({QPointF(40, 10), QPointF(42, 10), QPointF(44, 10)});
    QVERIFY(f.committedOnce(before, "Clone Stamp"));
    QCOMPARE(f.view->sample_argb(43, 10), QColor(220, 30, 30).rgba());
    QCOMPARE(offset->value(), 35.0);
}

void CloneSourceOverlayTest::controlsAndSlots()
{
    QVERIFY(stateHome_.isValid());
    auto frame = pictura::test::makeMainWindow();
    QVERIFY(frame);
    QImage image(64, 64, QImage::Format_RGB32);
    image.fill(Qt::white);
    paint_fixture::Fixture f(*frame, image, QStringLiteral("clone_overlay_controls"));
    QVERIFY(f.ok());
    auto* panel = frame->findChild<pictura::CloneSourcePanel*>();
    QVERIFY(panel);
    auto* sourceSlots = f.tools->optionsFor<pictura::CloneSourceOptions>(pictura::ToolId::CloneStamp);
    QVERIFY(sourceSlots);
    auto* show = panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceShowOverlay"));
    auto* hide = panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceAutoHide"));
    auto* clip = panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceClipped"));
    auto* invert = panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceInvert"));
    auto* opacity = panel->findChild<pictura::NumericField*>(QStringLiteral("cloneSourceOverlayOpacity"));
    auto* mode = panel->findChild<QComboBox*>(QStringLiteral("cloneSourceOverlayMode"));
    QVERIFY(show && hide && clip && invert && opacity && mode);
    QVERIFY(show->isEnabled() && hide->isEnabled() && clip->isEnabled() && invert->isEnabled());
    const int before = f.view->history_index();
    const auto pixel = f.view->sample_argb(4, 4);
    show->setChecked(true);
    hide->setChecked(true);
    clip->setChecked(true);
    invert->setChecked(true);
    opacity->setValue(37);
    mode->setCurrentIndex(3);
    sourceSlots->setSlot(1);
    QVERIFY(!show->isChecked());
    QCOMPARE(opacity->value(), 100.0);
    sourceSlots->setSlot(0);
    QVERIFY(show->isChecked() && hide->isChecked() && clip->isChecked() && invert->isChecked());
    QCOMPARE(opacity->value(), 37.0);
    QCOMPARE(mode->currentIndex(), 3);
    QCOMPARE(f.view->history_index(), before);
    QCOMPARE(f.view->sample_argb(4, 4), pixel);
}

namespace {
QImage overlaySeed()
{
    QImage image(64, 64, QImage::Format_ARGB32);
    image.fill(QColor(60, 120, 180));
    QPainter p(&image);
    p.fillRect(0, 0, 24, 64, QColor(200, 80, 40));
    p.fillRect(8, 16, 3, 9, QColor(20, 240, 40));
    p.fillRect(20, 16, 3, 9, QColor(20, 40, 240));
    return image;
}

struct OverlayFixture {
    std::unique_ptr<pictura::PicturaMainWindow> frame = pictura::test::makeMainWindow();
    paint_fixture::Fixture f;
    pictura::CloneSourcePanel* panel;
    pictura::CloneSourceOptions* sourceSlots;
    QCheckBox *show, *hide, *clip, *invert;
    QComboBox* mode;
    pictura::NumericField* opacity;
    explicit OverlayFixture(const QImage& image = overlaySeed())
        : f(*frame, image, QStringLiteral("clone_overlay_canvas"))
        , panel(frame->findChild<pictura::CloneSourcePanel*>())
        , sourceSlots(f.tools->optionsFor<pictura::CloneSourceOptions>(pictura::ToolId::CloneStamp))
        , show(panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceShowOverlay")))
        , hide(panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceAutoHide")))
        , clip(panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceClipped")))
        , invert(panel->findChild<QCheckBox*>(QStringLiteral("cloneSourceInvert")))
        , mode(panel->findChild<QComboBox*>(QStringLiteral("cloneSourceOverlayMode")))
        , opacity(panel->findChild<pictura::NumericField*>(QStringLiteral("cloneSourceOverlayOpacity")))
    {
        frame->show();
        QApplication::processEvents();
    }
    bool ready() const { return f.ok() && panel && sourceSlots && show && hide && clip && invert && mode && opacity; }
    void arm(pictura::ToolId tool)
    {
        frame->setActiveTool(tool);
        sourceSlots->setSlot(0);
        sourceSlots->setActive(pictura::CloneSource{});
        f.drag({QPointF(12, 20)}, Qt::AltModifier);
        show->setChecked(true);
        move(QPointF(40, 20));
    }
    void move(QPointF p) { f.canvas->mouseMoved(p); QApplication::processEvents(); }
    QColor pixel(QPointF p) const
    {
        if (!QTest::qWaitFor([this] { return !f.tools->cloneOverlayPendingForTest(); }, 10000)) {
            QTest::qFail("Clone overlay preview did not finish", __FILE__, __LINE__);
            return {};
        }
        const QPoint where = f.canvas->imageToWidget(p).toPoint();
        return f.canvas->grab().toImage().pixelColor(where);
    }
};
}

void CloneSourceOverlayTest::previewMapping()
{
    for (auto tool : {pictura::ToolId::CloneStamp, pictura::ToolId::HealingBrush}) {
        OverlayFixture t;
        QVERIFY(t.ready());
        t.arm(tool);
        const int history = t.f.view->history_index();
        const auto before = t.f.view->sample_argb(40, 20);
        const QColor first = t.pixel(QPointF(42, 20));
        t.move(QPointF(46, 20));
        const QColor moved = t.pixel(QPointF(48, 20));
        QCOMPARE(first, moved);
        QVERIFY(first != QColor(60, 120, 180));
        QCOMPARE(t.f.view->sample_argb(40, 20), before);
        QCOMPARE(t.f.view->history_index(), history);
    }
}

void CloneSourceOverlayTest::nonAlignedMapping()
{
    for (auto tool : {pictura::ToolId::CloneStamp, pictura::ToolId::HealingBrush}) {
        OverlayFixture t;
        QVERIFY(t.ready());
        t.arm(tool);
        t.f.drag({QPointF(40, 20), QPointF(41, 20)});
        if (tool == pictura::ToolId::CloneStamp) {
            auto* group = t.f.tools->optionsFor<pictura::StampGroup>(tool);
            auto settings = group->get(); settings.cloneAligned = false; group->set(settings);
        } else {
            auto* group = t.f.tools->optionsFor<pictura::HealingGroup>(tool);
            auto settings = group->get(); settings.aligned = false; group->set(settings);
        }
        t.move(QPointF(48, 20));
        const QColor hover = t.pixel(QPointF(52, 20));
        t.f.canvas->mousePressed(QPointF(48, 20), Qt::LeftButton, 0);
        const QColor down = t.pixel(QPointF(52, 20));
        t.f.canvas->mouseReleased(QPointF(48, 20));
        QCOMPARE(hover, QColor(200, 80, 40));
        QCOMPARE(hover, down);
    }
}

void CloneSourceOverlayTest::healingTransformedSampling()
{
    auto stroke = [](bool flip) -> std::pair<QColor, QList<quint32>> {
        OverlayFixture t;
        if (!t.ready()) return {};
        t.arm(pictura::ToolId::HealingBrush);
        t.f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
        auto slot = t.sourceSlots->active();
        slot.flipH = flip;
        t.sourceSlots->setActive(slot);
        t.move(QPointF(40, 20));
        const QColor preview = t.pixel(QPointF(42, 20));
        t.f.drag({QPointF(40, 20), QPointF(44, 20)});
        QList<quint32> result;
        for (int y = 16; y < 25; ++y)
            for (int x = 36; x < 49; ++x)
                result.append(t.f.view->sample_argb(x, y));
        return {preview, result};
    };
    const auto plain = stroke(false);
    const auto flipped = stroke(true);
    QVERIFY(!plain.second.isEmpty());
    QVERIFY(plain.first != flipped.first);
    QVERIFY(plain.second != flipped.second);
}

void CloneSourceOverlayTest::healingSourceCancelled()
{
    OverlayFixture t;
    QVERIFY(t.ready());
    t.arm(pictura::ToolId::HealingBrush);
    QVERIFY(t.sourceSlots->active().healingSourceValid);
    t.f.tools->onSessionCancelled();
    QVERIFY(!t.sourceSlots->active().healingSourceValid);
    QVERIFY(t.sourceSlots->active().hasSource);
    const int before = t.f.view->history_index();
    t.f.drag({QPointF(40, 20)});
    QCOMPARE(t.f.view->history_index(), before);
    t.f.drag({QPointF(12, 20)}, Qt::AltModifier);
    QVERIFY(t.sourceSlots->active().healingSourceValid);
}

void CloneSourceOverlayTest::previewEligibility()
{
    OverlayFixture t;
    QVERIFY(t.ready());
    t.arm(pictura::ToolId::CloneStamp);
    const QColor visible = t.pixel(QPointF(42, 20));
    QEvent leave(QEvent::Leave);
    QApplication::sendEvent(t.f.canvas, &leave);
    const QColor outside = t.pixel(QPointF(42, 20));
    QVERIFY(visible != outside);
    t.move(QPointF(40, 20));
    t.frame->setActiveTool(pictura::ToolId::Move);
    const QColor inactive = t.pixel(QPointF(42, 20));
    QVERIFY(visible != inactive);
    t.frame->setActiveTool(pictura::ToolId::CloneStamp);
    t.sourceSlots->setSlot(1);
    t.move(QPointF(40, 20));
    QCOMPARE(t.pixel(QPointF(42, 20)), inactive);
}

void CloneSourceOverlayTest::autoHide()
{
    OverlayFixture t;
    QVERIFY(t.ready());
    t.arm(pictura::ToolId::CloneStamp);
    t.hide->setChecked(true);
    const QColor hover = t.pixel(QPointF(44, 26));
    t.f.canvas->mousePressed(QPointF(40, 20), Qt::LeftButton, 0);
    const QColor pressed = t.pixel(QPointF(44, 26));
    QVERIFY(hover != pressed);
    t.f.canvas->mouseReleased(QPointF(40, 20));
    t.move(QPointF(40, 20));
    QVERIFY(t.pixel(QPointF(44, 26)) != pressed);
    t.f.canvas->mousePressed(QPointF(40, 20), Qt::LeftButton, 0);
    QTest::keyClick(t.f.canvas, Qt::Key_Escape);
    t.move(QPointF(40, 20));
    QVERIFY(t.pixel(QPointF(44, 26)) != pressed);
}

void CloneSourceOverlayTest::brushClipping()
{
    OverlayFixture t;
    QVERIFY(t.ready());
    t.arm(pictura::ToolId::CloneStamp);
    t.f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(6);
    const QColor unclipped = t.pixel(QPointF(50, 20));
    t.clip->setChecked(true);
    const QColor clipped = t.pixel(QPointF(50, 20));
    QVERIFY(unclipped != clipped);
    t.clip->setChecked(false);
    QCOMPARE(t.pixel(QPointF(50, 20)), unclipped);
}

void CloneSourceOverlayTest::rotatedBrushClipping()
{
    for (auto tool : {pictura::ToolId::CloneStamp, pictura::ToolId::HealingBrush}) {
        OverlayFixture t;
        QVERIFY(t.ready());
        t.arm(tool);
        auto* paint = t.f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
        paint->setSize(20);
        paint->setRoundness(20);
        paint->setTipAngle(45);
        const QPointF along(44, 24), across(36, 24);
        const QColor alongUnclipped = t.pixel(along);
        const QColor acrossUnclipped = t.pixel(across);
        t.clip->setChecked(true);
        QCOMPARE(t.pixel(along), alongUnclipped);
        if (tool == pictura::ToolId::CloneStamp)
            QVERIFY(t.pixel(across) != acrossUnclipped);
        else
            QCOMPARE(t.pixel(across), acrossUnclipped);
    }
}

void CloneSourceOverlayTest::blendAndOpacity()
{
    OverlayFixture t;
    QVERIFY(t.ready());
    t.arm(pictura::ToolId::CloneStamp);
    QList<QColor> colors;
    for (int i = 0; i < 4; ++i) {
        t.mode->setCurrentIndex(i);
        colors.append(t.pixel(QPointF(42, 20)));
    }
    QCOMPARE(colors[0], QColor(200, 80, 40));
    QCOMPARE(colors[1], QColor(60, 80, 40));
    QCOMPARE(colors[2], QColor(200, 120, 180));
    QCOMPARE(colors[3], QColor(140, 40, 140));
    t.opacity->setValue(0);
    const QColor zero = t.pixel(QPointF(42, 20));
    t.show->setChecked(false);
    QCOMPARE(t.pixel(QPointF(42, 20)), zero);
}

void CloneSourceOverlayTest::sourceAlpha()
{
    QImage seed = overlaySeed();
    QPainter painter(&seed);
    painter.setCompositionMode(QPainter::CompositionMode_Source);
    painter.fillRect(12, 18, 3, 5, QColor(200, 80, 40, 0));
    painter.fillRect(16, 18, 3, 5, QColor(200, 80, 40, 128));
    painter.end();
    OverlayFixture t(seed);
    QVERIFY(t.ready());
    t.arm(pictura::ToolId::CloneStamp);
    t.invert->setChecked(true);
    t.opacity->setValue(50);
    const QColor transparent = t.pixel(QPointF(42, 20));
    const QColor partial = t.pixel(QPointF(46, 20));
    t.show->setChecked(false);
    QCOMPARE(t.pixel(QPointF(42, 20)), transparent);
    QVERIFY(qAbs(partial.red() - 59) <= 2);
    QVERIFY(qAbs(partial.green() - 134) <= 2);
    QVERIFY(qAbs(partial.blue() - 189) <= 2);
}

void CloneSourceOverlayTest::grayAlignment()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(QColor(60, 120, 180));
    OverlayFixture t(seed);
    QVERIFY(t.ready());
    t.arm(pictura::ToolId::CloneStamp);
    t.opacity->setValue(50);
    t.invert->setChecked(true);
    const QColor c = t.pixel(QPointF(42, 20));
    QVERIFY(qAbs(c.red() - 128) <= 2);
    QVERIFY(qAbs(c.green() - 128) <= 2);
    QVERIFY(qAbs(c.blue() - 128) <= 2);
}

void CloneSourceOverlayTest::sourceStaysSnapshot()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(QColor(200, 80, 40));
    QPainter painter(&seed);
    painter.fillRect(10, 17, 5, 7, QColor(20, 240, 40));
    painter.end();
    OverlayFixture t(seed);
    QVERIFY(t.ready());
    t.frame->setActiveTool(pictura::ToolId::CloneStamp);
    t.sourceSlots->setSlot(0);
    t.sourceSlots->setActive(pictura::CloneSource{});
    t.f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(6);
    t.f.drag({QPointF(12, 20)}, Qt::AltModifier);
    t.show->setChecked(true);
    t.move(QPointF(16, 20));
    const QColor before = t.pixel(QPointF(20, 20));
    t.f.canvas->mousePressed(QPointF(16, 20), Qt::LeftButton, 0);
    QCOMPARE(t.pixel(QPointF(20, 20)), before);
    t.f.canvas->mouseReleased(QPointF(16, 20));
}

void CloneSourceOverlayTest::asynchronousSourceInvalidation()
{
    OverlayFixture t;
    QVERIFY(t.ready());
    pictura::clone_overlay_test_delay(*t.f.view, 1000);
    QElapsedTimer elapsed;
    elapsed.start();
    t.arm(pictura::ToolId::CloneStamp);
    for (int x = 35; x < 55; ++x) t.f.canvas->mouseMoved(QPointF(x, 20));
    QVERIFY2(elapsed.elapsed() < 800, "pointer events waited for source rendering");
    bool delivered = false;
    QTimer::singleShot(0, [&delivered] { delivered = true; });
    QTRY_VERIFY(delivered);
    QTRY_VERIFY(!t.f.tools->cloneOverlayPendingForTest());
    const QColor first = t.pixel(QPointF(56, 20));
    t.f.drag({QPointF(8, 20)}, Qt::AltModifier);
    t.move(QPointF(54, 20));
    QTRY_VERIFY(!t.f.tools->cloneOverlayPendingForTest());
    const QColor next = t.pixel(QPointF(56, 20));
    QVERIFY(first != next);
    t.frame->setActiveTool(pictura::ToolId::Brush);
    t.f.tools->setForeground(Qt::black);
    t.f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(10);
    t.f.drag({QPointF(10, 20)});
    t.frame->setActiveTool(pictura::ToolId::CloneStamp);
    t.move(QPointF(54, 20));
    QTRY_VERIFY(!t.f.tools->cloneOverlayPendingForTest());
    QVERIFY(t.pixel(QPointF(56, 20)) != next);
    pictura::clone_overlay_test_delay(*t.f.view, 0);
}

PICTURA_QT_TEST_MAIN(CloneSourceOverlayTest)
#include "tst_clone_source_overlay.moc"
