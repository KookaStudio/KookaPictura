// The canvas's "Rendering…" badge (#1608): when it shows, when it hides, and
// the view's pending-tiles state it reads.

#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtGui/QMouseEvent>

#include "image_view.h"
#include "rendering_indicator.h"

#include "api/document_bridge.h"
#include "api/view_bridge.h"

namespace {

// A canvas whose pending state is a test flag, so no case depends on how fast
// a worker builds tiles.
class FlaggedCanvas : public pictura::ImageView {
public:
    int presses = 0;

protected:
    void mousePressEvent(QMouseEvent* event) override
    {
        ++presses;
        event->accept();
    }
};

} // namespace

class RenderingIndicatorTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void showsWhenTilesStayPending();
    void hidesWhenTilesLand();
    void fastRedrawShowsNothing();
    void passesMouseThrough();
    void noTimerWhileIdle();
    void staysOutOfCanvasRenders();
    void requestedTilesArePendingUntilTheyLand();

private:
    void repaint();
    pictura::RenderingIndicator* badge() const { return canvas_->renderingIndicatorForTest(); }

    std::unique_ptr<FlaggedCanvas> canvas_;
    bool pending_ = false;
};

void RenderingIndicatorTest::init()
{
    pending_ = false;
    canvas_ = std::make_unique<FlaggedCanvas>();
    canvas_->resize(640, 480);
    pictura::ImageView::LevelProvider provider;
    provider.tilesPending = [this] { return pending_; };
    canvas_->setLevelProvider(std::move(provider));
    canvas_->show();
    QVERIFY(QTest::qWaitForWindowExposed(canvas_.get()));
}

void RenderingIndicatorTest::cleanup()
{
    canvas_.reset();
}

void RenderingIndicatorTest::repaint()
{
    canvas_->repaint();
    QCoreApplication::processEvents();
}

// [cri_shows]
void RenderingIndicatorTest::showsWhenTilesStayPending()
{
    pending_ = true;
    repaint();
    QVERIFY(!badge()->isVisible());
    QTRY_VERIFY_WITH_TIMEOUT(badge()->isVisible(), 2000);
    QVERIFY(badge()->animationActiveForTest());
    QCOMPARE(badge()->geometry().right(), canvas_->width() - 11);
}

// [cri_hides]
void RenderingIndicatorTest::hidesWhenTilesLand()
{
    pending_ = true;
    repaint();
    QTRY_VERIFY_WITH_TIMEOUT(badge()->isVisible(), 2000);
    // No paint follows the landing here: the animation tick alone hides it.
    pending_ = false;
    QTRY_VERIFY_WITH_TIMEOUT(!badge()->isVisible(), 1000);
}

// [cri_fast]
void RenderingIndicatorTest::fastRedrawShowsNothing()
{
    pending_ = true;
    repaint();
    // Tiles that land before the 250 ms delay: pending for 100 ms only.
    QTest::qWait(100);
    pending_ = false;
    repaint();
    // Past the point where the delay would have shown the badge.
    QTest::qWait(pictura::RenderingIndicator::kDelayMs + 150);
    QVERIFY(!badge()->isVisible());
}

// [cri_transparent]
void RenderingIndicatorTest::passesMouseThrough()
{
    pending_ = true;
    repaint();
    QTRY_VERIFY_WITH_TIMEOUT(badge()->isVisible(), 2000);
    QTest::mousePress(canvas_.get(), Qt::LeftButton, Qt::NoModifier, badge()->geometry().center());
    QCOMPARE(canvas_->presses, 1);
    QVERIFY(badge()->focusPolicy() == Qt::NoFocus);
}

// [cri_idle]
void RenderingIndicatorTest::noTimerWhileIdle()
{
    pending_ = true;
    repaint();
    QTRY_VERIFY_WITH_TIMEOUT(badge()->isVisible(), 2000);
    pending_ = false;
    QTRY_VERIFY_WITH_TIMEOUT(!badge()->isVisible(), 1000);
    repaint();
    QVERIFY(!badge()->delayActiveForTest());
    QVERIFY(!badge()->animationActiveForTest());
}

// [cri_not_in_render]
void RenderingIndicatorTest::staysOutOfCanvasRenders()
{
    // A canvas inside a host, as the document window lays it out.
    QWidget host;
    host.resize(700, 520);
    auto* canvas = new pictura::ImageView(&host);
    canvas->setGeometry(20, 30, 640, 480);
    bool pending = false;
    pictura::ImageView::LevelProvider provider;
    provider.tilesPending = [&pending] { return pending; };
    canvas->setLevelProvider(std::move(provider));
    host.show();
    QVERIFY(QTest::qWaitForWindowExposed(&host));
    const auto shot = [canvas] {
        QImage image(canvas->size(), QImage::Format_ARGB32);
        canvas->render(&image);
        return image;
    };
    const QImage idle = shot();
    pending = true;
    canvas->repaint();
    auto* badge = canvas->renderingIndicatorForTest();
    QTRY_VERIFY_WITH_TIMEOUT(badge->isVisible(), 2000);
    QCOMPARE(badge->parentWidget(), &host);
    // Over the canvas's top-right corner, in the host's coordinates.
    QCOMPARE(badge->geometry().right(), canvas->geometry().right() - 10);
    QCOMPARE(badge->geometry().top(), canvas->geometry().top() + 10);
    QVERIFY(shot() == idle);
}

// [cri_pending]
void RenderingIndicatorTest::requestedTilesArePendingUntilTheyLand()
{
    // Over the 16 Mpx limit, so missing tiles are built on a worker.
    pictura::PictureView view;
    QVERIFY(view.new_document(5000, 4000, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QTRY_VERIFY_WITH_TIMEOUT(!pictura::tiles_pending(view), 10000);
    const QImage crop = view.canvas_display_image(0, 2048, 2048, 512, 512);
    QVERIFY(!crop.isNull());
    QVERIFY(pictura::tiles_pending(view));
    QTRY_VERIFY_WITH_TIMEOUT(!pictura::tiles_pending(view), 10000);
}

QTEST_MAIN(RenderingIndicatorTest)
#include "tst_rendering_indicator.moc"
