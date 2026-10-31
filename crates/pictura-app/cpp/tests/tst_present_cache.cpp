// The document present path: a stored level is drawn from a zero-copy view, a
// level-0 crop is cached and patched from a damage rectangle, and a document
// region refresh crosses the bridge as a rectangle rather than pixels (#968).

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
#include <QtCore/QElapsedTimer>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QImage>

#include <memory>

#include "frame.h"
#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {

constexpr quint32 kRed = 0xFFFF0000u;
constexpr quint32 kWhite = 0xFFFFFFFFu;

QImage renderCanvas(pictura::ImageView& canvas)
{
    canvas.repaint();
    QCoreApplication::processEvents();
    QImage shot(canvas.size(), QImage::Format_ARGB32);
    canvas.render(&shot);
    return shot;
}

bool samePixels(const QImage& a, const QImage& b)
{
    if (a.size() != b.size()) {
        return false;
    }
    return a.convertToFormat(QImage::Format_ARGB32) == b.convertToFormat(QImage::Format_ARGB32);
}

} // namespace

class PresentCacheTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void documentRefreshArrivesAsDamage();
    void nonDocumentCanvasBlitsPixels();
    void documentRefreshMatchesFullRecomposite();
    void presentDoesNotBuildTheFullImage();
    void storedLevelIsDrawnFromALevelView();
    void storedLevelViewMatchesTheCropDraw();
    void level0CropIsReusedAcrossPaints();
    void zoomChangeSelectsTheNewLevel();
    void level0PresentMatchesTheTransformDraw();
    void largeDocumentPresentsThroughALevel();
    void panPresentsWithoutResamplingTheDocument();
    void panAndZoomReuseTheLevel0Crop();
    void panThenZoomFrameTimes();
    void idlePresentLevelIsZoomSelected();
    void level0PatchMatchesFreshCrop();
    void damageOutsideCacheChangesNothing();
    void damageForAnotherLevelDiscardsCrop();
    void filterCommitPresentsThroughDamage();

private:
    bool newDoc(int width, int height);
    void setZoom(double zoom);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::ImageView* canvas_ = nullptr;
};

void PresentCacheTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(800, 600);
    window_->show();
    QCoreApplication::processEvents();
}

void PresentCacheTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    canvas_ = nullptr;
}

bool PresentCacheTest::newDoc(int width, int height)
{
    if (!window_->newDocument(QStringLiteral("PresentCache"), width, height,
                              QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    canvas_ = window_->imageView();
    QTest::qWait(0);
    QCoreApplication::processEvents();
    return view_ && canvas_;
}

void PresentCacheTest::setZoom(double zoom)
{
    canvas_->setZoom(zoom, canvas_->rect().center());
    QCoreApplication::processEvents();
}

void PresentCacheTest::documentRefreshArrivesAsDamage()
{
    QVERIFY2(newDoc(520, 120), "document fixture");
    renderCanvas(*canvas_);
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    QVERIFY(damaged.isValid() && blitted.isValid());

    QVERIFY(view_->set_layers_visible(QStringList{QStringLiteral("0")}, false) >= 0);
    QCoreApplication::processEvents();

    QVERIFY2(damaged.count() >= 1, "a document region refresh emits damage");
    QCOMPARE(blitted.count(), 0);
    QVERIFY(damaged.last().at(2).toInt() > 0);
    QVERIFY(damaged.last().at(3).toInt() > 0);
}

void PresentCacheTest::nonDocumentCanvasBlitsPixels()
{
    pictura::ImageView canvas;
    QImage base(8, 8, QImage::Format_ARGB32_Premultiplied);
    base.fill(Qt::white);
    canvas.setImage(base);

    QImage red(3, 3, QImage::Format_ARGB32_Premultiplied);
    red.fill(Qt::red);
    canvas.blitRegion(red, 2, 3);

    QCOMPARE(canvas.image().pixelColor(3, 4), QColor(Qt::red));
    QCOMPARE(canvas.image().pixelColor(0, 0), QColor(Qt::white));
}

void PresentCacheTest::documentRefreshMatchesFullRecomposite()
{
    QVERIFY2(newDoc(520, 120), "document fixture");
    setZoom(1.0);
    renderCanvas(*canvas_);

    QVERIFY(view_->set_layers_visible(QStringList{QStringLiteral("0")}, false) >= 0);
    QCoreApplication::processEvents();
    // The region path refreshes the tool cursor, which can arm the Move tool's
    // idle transform-controls box. It is not part of the present, and the box
    // is cleared again by the panel refresh, so drop it before both shots.
    canvas_->setControlsBox(QPolygonF());
    const QImage regionShot = renderCanvas(*canvas_);

    // Force a full recomposite of the same document.
    view_->set_gpu_compute(view_->gpu_compute());
    QCoreApplication::processEvents();
    canvas_->setControlsBox(QPolygonF());
    const QImage fullShot = renderCanvas(*canvas_);

    QVERIFY2(samePixels(regionShot, fullShot),
             "the damaged present equals a full recomposite");
}

void PresentCacheTest::presentDoesNotBuildTheFullImage()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    setZoom(0.4);
    renderCanvas(*canvas_);
    QVERIFY(!canvas_->holdsImageForTest());
    QCOMPARE(canvas_->presentLevelForTest(), 1);
}

void PresentCacheTest::storedLevelIsDrawnFromALevelView()
{
    QVERIFY2(newDoc(600, 400), "document fixture");
    setZoom(0.4);
    const QImage before = renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    const int builds = canvas_->presentCacheRebuildCount();
    QVERIFY(canvas_->presentCacheImageSize().width() <= 300);
    QVERIFY(canvas_->presentCacheImageSize().height() <= 200);

    QVERIFY(view_->begin_paint(kRed, kWhite, 12, 100, 100, 0, 100, 100, 25,
                               QStringLiteral("normal"), false, false));
    QVERIFY(view_->paint_dab(300.0, 200.0, 1.0));
    view_->flush_present();
    QCoreApplication::processEvents();
    const QImage after = renderCanvas(*canvas_);
    QVERIFY2(!samePixels(before, after), "the stored-level present shows the dab");
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    QCOMPARE(canvas_->presentCacheRebuildCount(), builds);
    QVERIFY(view_->end_paint());
}

void PresentCacheTest::storedLevelViewMatchesTheCropDraw()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    const double zooms[] = {0.4, 0.25};
    for (const double zoom : zooms) {
        setZoom(zoom);
        QVERIFY2(canvas_->presentLevelForTest() > 0, "the zoom must select a stored level");
        const QImage view = renderCanvas(*canvas_);
        // Without the cache the same paint falls back to a crop of the level,
        // so this compares the zero-copy draw with the crop draw.
        canvas_->setPresentCacheEnabledForTest(false);
        const QImage crop = renderCanvas(*canvas_);
        canvas_->setPresentCacheEnabledForTest(true);
        QVERIFY2(samePixels(view, crop),
                 qPrintable(QStringLiteral("the stored-level view differs from the crop at %1")
                                .arg(zoom)));
    }

    // A pan moves neither the level's source rectangle nor the pixels it maps.
    setZoom(0.4);
    canvas_->panBy(QPointF(10.0, 10.0));
    const QImage panned = renderCanvas(*canvas_);
    canvas_->setPresentCacheEnabledForTest(false);
    const QImage pannedCrop = renderCanvas(*canvas_);
    canvas_->setPresentCacheEnabledForTest(true);
    QVERIFY2(samePixels(panned, pannedCrop),
             "the stored-level view differs from the crop after a pan");
}

void PresentCacheTest::level0CropIsReusedAcrossPaints()
{
    QVERIFY2(newDoc(520, 120), "document fixture");
    setZoom(0.55);
    renderCanvas(*canvas_);
    const int builds = canvas_->presentCacheRebuildCount();
    renderCanvas(*canvas_);

    QCOMPARE(canvas_->presentCacheRebuildCount(), builds);
    QVERIFY(!canvas_->presentCacheRebuiltOnLastPaint());
    QVERIFY(canvas_->presentCacheValidForTest());
    QVERIFY(canvas_->presentCacheImageSize().width() > 0);
}

void PresentCacheTest::zoomChangeSelectsTheNewLevel()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    setZoom(1.0);
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 0);
    QVERIFY(canvas_->presentCacheImageSize().width() > 0);

    setZoom(0.4);
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    const QSize size = canvas_->presentCacheImageSize();
    QVERIFY(size.width() > 0 && size.width() <= 600);
    QVERIFY(size.height() > 0 && size.height() <= 400);
}

void PresentCacheTest::level0PresentMatchesTheTransformDraw()
{
    QVERIFY2(newDoc(520, 120), "document fixture");
    setZoom(1.0);
    const QImage present = renderCanvas(*canvas_);
    canvas_->setPresentLevelCropForTest(false);
    const QImage direct = renderCanvas(*canvas_);
    canvas_->setPresentLevelCropForTest(true);
    QVERIFY2(samePixels(present, direct),
             "the level-0 present equals a direct transform draw");
}

void PresentCacheTest::largeDocumentPresentsThroughALevel()
{
    QVERIFY2(newDoc(4000, 3000), "document fixture");
    setZoom(0.2);
    QCOMPARE(canvas_->presentLevelForTest(), 2);
    const QImage shot = renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 2);
    QVERIFY(!canvas_->holdsImageForTest());
    QVERIFY(canvas_->presentCacheRebuildCount() == 0);
    QVERIFY(!shot.isNull());
}

void PresentCacheTest::panPresentsWithoutResamplingTheDocument()
{
    QVERIFY2(newDoc(2048, 1536), "document fixture");
    setZoom(0.4);
    renderCanvas(*canvas_);
    const int builds = canvas_->presentCacheRebuildCount();
    QCOMPARE(canvas_->presentLevelForTest(), 1);

    canvas_->panBy(QPointF(10.0, 10.0));
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    QCOMPARE(canvas_->presentCacheRebuildCount(), builds);
    QVERIFY(!canvas_->holdsImageForTest());
}

void PresentCacheTest::panAndZoomReuseTheLevel0Crop()
{
    QVERIFY2(newDoc(4000, 3000), "document fixture");
    setZoom(1.0);
    QCOMPARE(canvas_->presentLevelForTest(), 0);
    renderCanvas(*canvas_);
    const int builds = canvas_->presentCacheRebuildCount();

    // A pan at level 0 reuses the cached crop's overlap instead of re-cropping
    // the whole viewport.
    canvas_->panBy(QPointF(-200.0, -150.0));
    const QImage panned = renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 0);
    QCOMPARE(canvas_->presentCacheRebuildCount(), builds);

    // The reused pan present equals a fresh level-0 crop over the same rect.
    canvas_->setPresentCacheEnabledForTest(false);
    const QImage pannedFresh = renderCanvas(*canvas_);
    canvas_->setPresentCacheEnabledForTest(true);
    QVERIFY2(samePixels(panned, pannedFresh),
             "the reused pan present differs from a fresh level-0 crop");

    // A zoom step that stays in the level-0 band reuses the cache too.
    renderCanvas(*canvas_);
    const int builds2 = canvas_->presentCacheRebuildCount();
    canvas_->setZoom(1.2, canvas_->rect().center());
    QCOMPARE(canvas_->presentLevelForTest(), 0);
    const QImage zoomed = renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentCacheRebuildCount(), builds2);

    canvas_->setPresentCacheEnabledForTest(false);
    const QImage zoomedFresh = renderCanvas(*canvas_);
    canvas_->setPresentCacheEnabledForTest(true);
    QVERIFY2(samePixels(zoomed, zoomedFresh),
             "the reused zoom present differs from a fresh level-0 crop");
}

void PresentCacheTest::panThenZoomFrameTimes()
{
    // The document size is overridable so the same profile can run at 16000².
    const int side = qEnvironmentVariableIntValue("PICTURA_PAN_ZOOM_SIZE");
    const int n = side > 0 ? side : 4000;
    QVERIFY2(newDoc(n, (n * 3) / 4), "document fixture");
    // A realistic viewport: the frame cost scales with it.
    window_->resize(2560, 1440);
    QCoreApplication::processEvents();
    const auto timeRender = [this]() {
        QElapsedTimer clock;
        clock.start();
        QImage shot(canvas_->size(), QImage::Format_ARGB32);
        canvas_->render(&shot);
        return double(clock.nsecsElapsed()) / 1e6;
    };
    // Warm the paint path at level 0.
    setZoom(1.0);
    timeRender();

    // Average the first render after a zoom step, from rest and after a pan.
    const auto averageZoom = [&](bool afterPan) {
        double sum = 0.0;
        const int trials = 5;
        for (int i = 0; i < trials; ++i) {
            setZoom(1.0);
            timeRender();
            if (afterPan) {
                canvas_->panBy(QPointF(200.0, 150.0));
                timeRender();
            }
            canvas_->setZoom(1.08, canvas_->rect().center());
            sum += timeRender();
        }
        return sum / trials;
    };
    const double zoomFromRest = averageZoom(false);
    const double zoomAfterPan = averageZoom(true);
    canvas_->setPresentCacheEnabledForTest(false);
    const double zoomFromRestFullCrop = averageZoom(false);
    const double zoomAfterPanFullCrop = averageZoom(true);
    canvas_->setPresentCacheEnabledForTest(true);

    qInfo("pictura profile: pan_zoom %dx%d canvas=%dx%d zoom_from_rest_ms=%.2f "
          "zoom_after_pan_ms=%.2f | full-crop zoom_from_rest_ms=%.2f zoom_after_pan_ms=%.2f",
          n, (n * 3) / 4, canvas_->width(), canvas_->height(), zoomFromRest, zoomAfterPan,
          zoomFromRestFullCrop, zoomAfterPanFullCrop);
}

void PresentCacheTest::idlePresentLevelIsZoomSelected()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    setZoom(0.55);
    QCOMPARE(canvas_->presentLevelForTest(), 0);
    setZoom(0.4);
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    setZoom(0.2);
    QCOMPARE(canvas_->presentLevelForTest(), 2);
}

void PresentCacheTest::level0PatchMatchesFreshCrop()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    setZoom(1.0);
    renderCanvas(*canvas_);
    QVERIFY(canvas_->presentCacheValidForTest());
    const int builds = canvas_->presentCacheRebuildCount();

    QVERIFY(view_->begin_paint(kRed, kWhite, 12, 100, 100, 0, 100, 100, 25,
                               QStringLiteral("normal"), false, false));
    QVERIFY(view_->paint_dab(400.0, 300.0, 1.0));
    view_->flush_present();
    QCoreApplication::processEvents();
    const QImage patched = renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentCacheRebuildCount(), builds);
    QVERIFY(view_->end_paint());
    QCoreApplication::processEvents();

    view_->set_gpu_compute(view_->gpu_compute());
    QCoreApplication::processEvents();
    const QImage full = renderCanvas(*canvas_);
    QVERIFY2(samePixels(patched, full),
             "the patched level-0 crop equals a fresh presentation");
}

void PresentCacheTest::damageOutsideCacheChangesNothing()
{
    QVERIFY2(newDoc(4000, 3000), "document fixture");
    setZoom(1.0);
    renderCanvas(*canvas_);
    QVERIFY(canvas_->presentCacheValidForTest());
    const QImage before = canvas_->presentCacheCropForTest();

    // Far outside the visible crop at 100 %.
    canvas_->damageRegion(QRect(3000, 2000, 40, 40));
    QVERIFY(canvas_->presentCacheValidForTest());
    const QImage after = canvas_->presentCacheCropForTest();
    QVERIFY2(samePixels(before, after),
             "a damage outside the cached rectangle leaves the crop unchanged");
    QVERIFY(canvas_->presentCacheValidForTest());
}

void PresentCacheTest::damageForAnotherLevelDiscardsCrop()
{
    QVERIFY2(newDoc(4000, 3000), "document fixture");
    setZoom(1.0);
    renderCanvas(*canvas_);
    QVERIFY(canvas_->presentCacheValidForTest());

    // Change the selected level without a paint, then damage: the handler
    // itself must discard the level-0 crop.
    canvas_->setZoom(0.4, canvas_->rect().center());
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    canvas_->damageRegion(QRect(10, 10, 40, 40));
    QVERIFY(!canvas_->presentCacheValidForTest());

    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 1);
}

void PresentCacheTest::filterCommitPresentsThroughDamage()
{
    QVERIFY2(newDoc(520, 120), "document fixture");
    renderCanvas(*canvas_);
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    QVERIFY(damaged.isValid() && blitted.isValid());

    QVERIFY(view_->apply_filter(QStringLiteral("add-noise")));
    QCoreApplication::processEvents();

    QVERIFY2(damaged.count() >= 1, "a filter commit emits a damage rectangle");
    QCOMPARE(blitted.count(), 0);
    QVERIFY(damaged.last().at(2).toInt() > 0);
}

QTEST_MAIN(PresentCacheTest)
#include "tst_present_cache.moc"
