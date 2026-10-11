// The automatic initial view (#324): a document set before the canvas has its
// laid-out viewport settles once the real size arrives — 100% and centred when
// the document fits, fit-to-screen and centred when it does not — while an
// explicit user adjustment is kept by later resizes.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"

#include "qt_test_support.h"

namespace {

double distance(const QPointF& a, const QPointF& b) { return QLineF(a, b).length(); }

QPointF viewportCentre(const pictura::ImageView& canvas)
{
    return QPointF(canvas.width() / 2.0, canvas.height() / 2.0);
}

} // namespace

class InitialViewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void smallDocumentCentresAtHundredPercent();
    void largeDocumentFitsAndCentres();
    void userZoomSurvivesLaterResizes();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    bool openDocument(int width, int height);
    void closeActiveDocument();
};

void InitialViewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

bool InitialViewTest::openDocument(int width, int height)
{
    return window_->newDocument(QStringLiteral("InitialView"), width, height,
                                QStringLiteral("rgb"), 8, QStringLiteral("white"));
}

void InitialViewTest::closeActiveDocument()
{
    window_->closeDocument(window_->activeDocumentIndex(), false);
}

void InitialViewTest::smallDocumentCentresAtHundredPercent()
{
    QVERIFY(openDocument(512, 512));
    window_->resize(1400, 1000);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    QCoreApplication::processEvents();

    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    QCOMPARE(canvas->zoom(), 1.0);
    const QPointF docCentre(256.0, 256.0);
    QVERIFY2(distance(canvas->imageToWidget(docCentre), viewportCentre(*canvas)) < 1.0,
             "a document smaller than the viewport must be centred at 100%");
    closeActiveDocument();
}

void InitialViewTest::largeDocumentFitsAndCentres()
{
    QVERIFY(openDocument(2000, 2000));
    window_->resize(900, 700);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    QCoreApplication::processEvents();

    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    const double fit = std::min(double(canvas->width()) / 2000.0,
                                double(canvas->height()) / 2000.0)
                       * 0.95;
    QVERIFY2(qAbs(canvas->zoom() - fit) < 0.001,
             "a document larger than the viewport must settle at the laid-out fit zoom");
    const QPointF docCentre(1000.0, 1000.0);
    QVERIFY2(distance(canvas->imageToWidget(docCentre), viewportCentre(*canvas)) < 1.0,
             "a fit document must be centred in the viewport");
    closeActiveDocument();
}

void InitialViewTest::userZoomSurvivesLaterResizes()
{
    QVERIFY(openDocument(512, 512));
    window_->resize(1400, 1000);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    QCoreApplication::processEvents();

    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    canvas->setZoom(2.0, viewportCentre(*canvas));
    window_->resize(1600, 1100);
    QCoreApplication::processEvents();
    QCOMPARE(canvas->zoom(), 2.0);
    closeActiveDocument();
}

QTEST_MAIN(InitialViewTest)
#include "tst_initial_view.moc"
