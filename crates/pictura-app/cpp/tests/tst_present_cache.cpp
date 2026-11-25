// The document present path: every level is drawn from a crop of the tile
// pyramid (#1216), a level-0 crop is cached and patched from a damage rectangle, and a document
// region refresh crosses the bridge as a rectangle rather than pixels (#968).

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QCoreApplication>
#include <QtCore/QDebug>
#include <QtCore/QDir>
#include <QtCore/QElapsedTimer>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtCore/QTemporaryDir>

#include <memory>

#include "frame.h"
#include "image_view.h"
#include "panels/histogram_panel.h"
#include "panels/navigator_panel.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/view_bridge.h"

#include "qt_test_support.h"
#include "selftest_paint_fixture.h"

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

bool replayExactRed(pictura::PictureView& view, int diameter, const QList<QPointF>& points)
{
    QJsonArray samples;
    for (const QPointF& point : points) {
        samples.append(QJsonObject{{QStringLiteral("x"), point.x()},
                                   {QStringLiteral("y"), point.y()},
                                   {QStringLiteral("pressure"), 1.0},
                                   {QStringLiteral("time"), 0.0}});
    }
    const QJsonObject settings{{QStringLiteral("foreground"), double(kRed)},
                               {QStringLiteral("background"), double(kWhite)},
                               {QStringLiteral("diameter"), diameter},
                               {QStringLiteral("hardness"), 100},
                               {QStringLiteral("roundness"), 100},
                               {QStringLiteral("opacity"), 100},
                               {QStringLiteral("flow"), 100},
                               {QStringLiteral("spacing"), 25},
                               {QStringLiteral("mode"), QStringLiteral("normal")}};
    const QJsonObject stroke{{QStringLiteral("version"), 1},
                             {QStringLiteral("tool"), QStringLiteral("Brush")},
                             {QStringLiteral("settings"), settings},
                             {QStringLiteral("samples"), samples}};
    return pictura::replay_tool_stroke(
        view, QString::fromUtf8(QJsonDocument(stroke).toJson(QJsonDocument::Compact)));
}

// Whole-image reference, independent of the renderer's regional/tiled reducers.
QImage referenceLevel(const QImage& straight, int level)
{
    const QImage source = straight.convertToFormat(QImage::Format_ARGB32);
    QImage current(source.size(), QImage::Format_ARGB32_Premultiplied);
    for (int y = 0; y < source.height(); ++y) {
        for (int x = 0; x < source.width(); ++x) {
            const QRgb pixel = source.pixel(x, y);
            const int alpha = qAlpha(pixel);
            current.setPixel(x, y, qRgba((qRed(pixel) * alpha + 127) / 255,
                                       (qGreen(pixel) * alpha + 127) / 255,
                                       (qBlue(pixel) * alpha + 127) / 255, alpha));
        }
    }
    for (int k = 0; k < level; ++k) {
        QImage next((current.width() + 1) / 2, (current.height() + 1) / 2,
                    QImage::Format_ARGB32_Premultiplied);
        for (int y = 0; y < next.height(); ++y) {
            for (int x = 0; x < next.width(); ++x) {
                int r = 0, g = 0, b = 0, a = 0, n = 0;
                for (int dy = 0; dy < 2 && 2 * y + dy < current.height(); ++dy) {
                    for (int dx = 0; dx < 2 && 2 * x + dx < current.width(); ++dx) {
                        const QRgb pixel = current.pixel(2 * x + dx, 2 * y + dy);
                        r += qRed(pixel);
                        g += qGreen(pixel);
                        b += qBlue(pixel);
                        a += qAlpha(pixel);
                        ++n;
                    }
                }
                next.setPixel(x, y, qRgba((r + n / 2) / n, (g + n / 2) / n,
                                         (b + n / 2) / n, (a + n / 2) / n));
            }
        }
        current = std::move(next);
    }
    return current;
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
    void hiddenLayerHidesAtFitZoom();
    void openedBackgroundHidesAtFitZoom();
    void soloHideRefreshesTheWholeCanvas();
    void lazyOpenedBackgroundHidesAtFitZoom();
    void presentDoesNotBuildTheFullImage();
    void reducedLevelPresentsADab();
    void cachedLevelCropMatchesTheUncachedDraw();
    void level0CropIsReusedAcrossPaints();
    void zoomChangeSelectsTheNewLevel();
    void level0PresentMatchesTheTransformDraw();
    void largeDocumentPresentsThroughALevel();
    void panPresentsWithoutResamplingTheDocument();
    void panAndZoomReuseTheLevel0Crop();
    void cropPanPresentsThroughALevel();
    void cropPanMatchesAViewPan();
    void cropPanKeepsTheSingleChannel();
    void panThenZoomFrameTimes();
    void idlePresentLevelIsZoomSelected();
    void level0PatchMatchesFreshCrop();
    void damageOutsideCacheChangesNothing();
    void damageForAnotherLevelDiscardsCrop();
    void filterCommitPresentsThroughDamage();
    void previewStrokeUsesTilesAndRestoresZoom();
    void frameBoundedPaintMatchesCpu();
    void frameBoundedPaintMatchesCpu_data();
    void panelsUseTileCrops();

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

void PresentCacheTest::hiddenLayerHidesAtFitZoom()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    setZoom(0.4);
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 1);

    QVERIFY(view_->set_layers_visible(QStringList{QStringLiteral("0")}, false) >= 0);
    QCoreApplication::processEvents();
    canvas_->setControlsBox(QPolygonF());
    const QImage regionShot = renderCanvas(*canvas_);

    view_->set_gpu_compute(view_->gpu_compute());
    QCoreApplication::processEvents();
    canvas_->setControlsBox(QPolygonF());
    const QImage fullShot = renderCanvas(*canvas_);

    QVERIFY2(samePixels(regionShot, fullShot),
             "the hidden layer still shows at the fit level");
}

void PresentCacheTest::openedBackgroundHidesAtFitZoom()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::red);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_bg_hide"));
    QVERIFY2(f.ok(), "opened image fixture");
    QVERIFY(f.view->add_layer(0) >= 0);
    f.canvas->setZoom(0.19, f.canvas->rect().center());
    QCoreApplication::processEvents();
    QVERIFY2(f.canvas->presentLevelForTest() > 0, "fit zoom presents a level");

    QSignalSpy settled(f.view, &pictura::PictureView::pyramid_settled);
    renderCanvas(*f.canvas);
    if (settled.count() == 0) {
        QVERIFY2(settled.wait(30000), "the initial tiles landed");
    }
    const QImage before = renderCanvas(*f.canvas);

    const int landed = settled.count();
    QCOMPARE(f.view->set_layers_visible(QStringList{QStringLiteral("0")}, false), 1);
    QCoreApplication::processEvents();
    if (settled.count() == landed) {
        QVERIFY2(settled.wait(30000), "the hidden tiles landed");
    }
    QCoreApplication::processEvents();
    f.canvas->setControlsBox(QPolygonF());
    const QImage regionShot = renderCanvas(*f.canvas);

    const int shown = settled.count();
    f.view->set_gpu_compute(f.view->gpu_compute());
    QCoreApplication::processEvents();
    if (settled.count() == shown) {
        QVERIFY2(settled.wait(30000), "the full rebuild landed");
    }
    QCoreApplication::processEvents();
    f.canvas->setControlsBox(QPolygonF());
    const QImage fullShot = renderCanvas(*f.canvas);

    QVERIFY2(!samePixels(before, regionShot), "the hidden Background still shows at fit zoom");
    QVERIFY2(samePixels(regionShot, fullShot),
             "the hidden Background's present differs from a full recomposite");
}

void PresentCacheTest::soloHideRefreshesTheWholeCanvas()
{
    QImage seed(1200, 800, QImage::Format_RGB32);
    seed.fill(Qt::red);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_solo_hide"));
    QVERIFY2(f.ok(), "opened image fixture");
    QVERIFY(f.view->add_layer(0) >= 0);
    f.view->set_active_layer(QStringLiteral("1"));
    QVERIFY(f.view->begin_paint(kRed, kWhite, 12, 100, 100, 0, 100, 100, 25,
                                QStringLiteral("normal"), false, false));
    QVERIFY(f.view->paint_dab(100.0, 100.0, 1.0));
    QVERIFY(f.view->end_paint());
    f.canvas->setZoom(0.4, f.canvas->rect().center());
    QCoreApplication::processEvents();
    renderCanvas(*f.canvas);

    // Alt-click solo: only the painted layer stays visible, so the Background
    // outside its small reach must go too.
    pictura::solo_set(*f.view, QStringLiteral("1"));
    QCoreApplication::processEvents();
    f.canvas->setControlsBox(QPolygonF());
    const QImage regionShot = renderCanvas(*f.canvas);

    f.view->set_gpu_compute(f.view->gpu_compute());
    QCoreApplication::processEvents();
    f.canvas->setControlsBox(QPolygonF());
    const QImage fullShot = renderCanvas(*f.canvas);

    QVERIFY2(samePixels(regionShot, fullShot),
             "solo left the hidden Background outside the soloed layer");

    // Restoring the snapshot flips the Background back on; that edit must also
    // reach the whole canvas.
    pictura::solo_restore(*f.view);
    QCoreApplication::processEvents();
    f.canvas->setControlsBox(QPolygonF());
    const QImage restored = renderCanvas(*f.canvas);

    f.view->set_gpu_compute(f.view->gpu_compute());
    QCoreApplication::processEvents();
    f.canvas->setControlsBox(QPolygonF());
    const QImage restoredFull = renderCanvas(*f.canvas);

    QVERIFY2(samePixels(restored, restoredFull),
             "restoring solo left the shown Background outside the soloed layer");
}

void PresentCacheTest::lazyOpenedBackgroundHidesAtFitZoom()
{
    const QString fixture = QFINDTESTDATA("../../../../assets/layers_panel_smoke.psd");
    if (fixture.isEmpty()) {
        QSKIP("layers_panel_smoke.psd fixture not found");
    }
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVERIFY(window_->openDocumentAtPath(fixture));
    pictura::PictureView* v = window_->activeView();
    pictura::ImageView* c = window_->imageView();
    QVERIFY(v && c);
    QVERIFY(v->resize_image(QStringLiteral("nearest"), 512, 512));
    const QString psd = dir.filePath(QStringLiteral("lazy.psd"));
    QVERIFY(window_->saveActiveAs(psd));
    window_->closeDocument(window_->activeDocumentIndex(), false);

    QVERIFY(window_->openDocumentAtPath(psd));
    v = window_->activeView();
    c = window_->imageView();
    QVERIFY(v && c);
    c->setZoom(0.25, c->rect().center());
    QCoreApplication::processEvents();
    QCOMPARE(c->presentLevelForTest(), 1);

    const QImage before = renderCanvas(*c);
    QCOMPARE(v->set_layers_visible(QStringList{QStringLiteral("0")}, false), 1);
    QCoreApplication::processEvents();
    c->setControlsBox(QPolygonF());
    const QImage regionShot = renderCanvas(*c);

    v->set_gpu_compute(v->gpu_compute());
    QCoreApplication::processEvents();
    c->setControlsBox(QPolygonF());
    const QImage fullShot = renderCanvas(*c);

    QVERIFY2(!samePixels(before, regionShot), "the hidden Background still shows");
    QVERIFY2(samePixels(regionShot, fullShot),
             "the lazy-open preview kept the hidden Background");
}

void PresentCacheTest::presentDoesNotBuildTheFullImage()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    setZoom(0.4);
    renderCanvas(*canvas_);
    QVERIFY(!canvas_->holdsImageForTest());
    QCOMPARE(canvas_->presentLevelForTest(), 1);
}

void PresentCacheTest::reducedLevelPresentsADab()
{
    QVERIFY2(newDoc(600, 400), "document fixture");
    setZoom(0.4);
    const QImage before = renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    QVERIFY(canvas_->presentCacheImageSize().width() <= 300);
    QVERIFY(canvas_->presentCacheImageSize().height() <= 200);

    QVERIFY(view_->begin_paint(kRed, kWhite, 12, 100, 100, 0, 100, 100, 25,
                               QStringLiteral("normal"), false, false));
    QVERIFY(view_->paint_dab(300.0, 200.0, 1.0));
    view_->flush_present();
    QCoreApplication::processEvents();
    const QImage after = renderCanvas(*canvas_);
    QVERIFY2(!samePixels(before, after), "the reduced-level present shows the dab");
    QCOMPARE(canvas_->presentLevelForTest(), 1);
    QVERIFY(view_->end_paint());
}

void PresentCacheTest::cachedLevelCropMatchesTheUncachedDraw()
{
    QVERIFY2(newDoc(1200, 800), "document fixture");
    const double zooms[] = {0.4, 0.25};
    for (const double zoom : zooms) {
        setZoom(zoom);
        QVERIFY2(canvas_->presentLevelForTest() > 0, "the zoom must select a stored level");
        const QImage view = renderCanvas(*canvas_);
        const int builds = canvas_->presentCacheRebuildCount();
        const QImage repeated = renderCanvas(*canvas_);
        QCOMPARE(canvas_->presentCacheRebuildCount(), builds);
        QVERIFY(samePixels(view, repeated));
        // Without the cache the same paint crops the level afresh, so this
        // compares the cached crop with a fresh one.
        canvas_->setPresentCacheEnabledForTest(false);
        const QImage crop = renderCanvas(*canvas_);
        canvas_->setPresentCacheEnabledForTest(true);
        QVERIFY2(samePixels(view, crop),
                 qPrintable(QStringLiteral("the cached crop differs from a fresh one at %1")
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
             "the cached crop differs from a fresh one after a pan");
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
    // A crop of the level's tiles, never larger than the level, and reused by
    // the next paint (#1216).
    const int builds = canvas_->presentCacheRebuildCount();
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentCacheRebuildCount(), builds);
    QVERIFY(canvas_->presentCacheImageSize().width() <= 1000);
    QVERIFY(canvas_->presentCacheImageSize().height() <= 750);
    QVERIFY(!shot.isNull());

    setZoom(4.0);
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 0);
    QVERIFY(!canvas_->holdsImageForTest());
    QVERIFY(qint64(canvas_->presentCacheImageSize().width())
                * canvas_->presentCacheImageSize().height() < 64 * 1024 * 1024);

    window_->closeDocument(window_->activeDocumentIndex(), false);
    QVERIFY2(newDoc(16000, 16000), "sparse large document");
    view_->set_gpu_compute(false);
    setZoom(0.05);
    QVERIFY(view_->begin_paint(kRed, kWhite, 8, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view_->paint_dab(8000.0, 8000.0, 1.0));
    view_->flush_present();
    renderCanvas(*canvas_);
    QVERIFY(canvas_->presentLevelForTest() > 0);
    QVERIFY(!canvas_->holdsImageForTest());
    view_->cancel_paint();
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

void PresentCacheTest::cropPanPresentsThroughALevel()
{
    pictura::ImageView canvas;
    canvas.resize(800, 600);
    canvas.show();
    QCoreApplication::processEvents();

    QSize doc(4000, 3000);
    QList<QString> requests;
    QList<QRect> reported;
    canvas.setLevelProvider(pictura::ImageView::LevelProvider{
        [&requests](int level, int x, int y, int w, int h) {
            requests.append(
                QStringLiteral("%1 %2 %3 %4 %5").arg(level).arg(x).arg(y).arg(w).arg(h));
            QImage crop(w, h, QImage::Format_ARGB32_Premultiplied);
            crop.fill(QColor(20 + 40 * level, 30, 30));
            return crop;
        },
        [] { return 2; },
        [&doc](int level) {
            return level == 0 ? doc : QSize((doc.width() + 1) / 2, (doc.height() + 1) / 2);
        },
        [] { return quint64(0); },
        [] { return false; },
        [] { return 0; },
        [&reported](int, const QRect& rect) { reported.append(rect); }});
    canvas.setDocument(doc);
    canvas.setCropContentOffset(QPointF(100.0, 100.0));

    // Every crop is level-space; the viewport is compared in document pixels,
    // plus presentLevelRect's sampling margin: one level pixel per edge and
    // whole-pixel floor/ceil rounding.
    const auto bounded = [&canvas, &doc, &requests](double zoom) {
        QVERIFY2(!requests.isEmpty(), "the present crops a level");
        for (const QString& request : requests) {
            const QStringList parts = request.split(QLatin1Char(' '));
            const int level = parts.at(0).toInt();
            const double w = parts.at(3).toDouble();
            const double h = parts.at(4).toDouble();
            const double scale = double(1 << level);
            QVERIFY2(w * scale <= canvas.width() / zoom + 4 * scale + 1
                         && h * scale <= canvas.height() / zoom + 4 * scale + 1,
                     qPrintable(QStringLiteral("crop %1 exceeds the viewport").arg(request)));
            QVERIFY2(!(level == 0 && w == doc.width() && h == doc.height()),
                     "the paint requested the whole document");
        }
        requests.clear();
    };

    for (const double zoom : {1.0, 0.4}) {
        canvas.setZoom(zoom, canvas.rect().center());
        renderCanvas(canvas);
        bounded(zoom);
        // Without a level present, the fallback still crops the viewport at level 0.
        canvas.setPresentLevelCropForTest(false);
        renderCanvas(canvas);
        bounded(zoom);
        canvas.setPresentLevelCropForTest(true);
    }

    // The presented (and pinned) rectangle follows the content pan.
    canvas.setZoom(1.0, canvas.rect().center());
    reported.clear();
    renderCanvas(canvas);
    QVERIFY2(!reported.isEmpty(), "the presented view is reported");
    const QRect withOffset = reported.last();
    canvas.setCropContentOffset(QPointF());
    reported.clear();
    renderCanvas(canvas);
    QVERIFY2(!reported.isEmpty(), "the presented view is reported");
    const QRect withoutOffset = reported.last();
    QCOMPARE(withOffset.topLeft(), withoutOffset.topLeft() - QPoint(100, 100));

    // A document narrower than the viewport: the offset moves content into the
    // vacated margin, and that band must still be cropped — the whole document
    // stays presented. Pre-fix this reported (0, 0, 511, 111).
    doc = QSize(520, 120);
    canvas.setDocument(doc);
    canvas.setZoom(1.0, canvas.rect().center());
    canvas.setCropContentOffset(QPointF(10.0, 10.0));
    reported.clear();
    renderCanvas(canvas);
    QVERIFY2(!reported.isEmpty(), "the presented view is reported");
    QCOMPARE(reported.last(), QRect(QPoint(0, 0), doc));
}

void PresentCacheTest::cropPanMatchesAViewPan()
{
    for (const double zoom : {1.0, 0.4}) {
        QVERIFY2(newDoc(4000, 3000), "document fixture");
        view_->set_gpu_compute(false);
        // Non-uniform content: a plain white document makes any two draws equal.
        QVERIFY(replayExactRed(*view_, 200, {QPointF(2000.0, 1500.0)}));
        setZoom(zoom);
        renderCanvas(*canvas_);

        canvas_->setCropContentOffset(QPointF(10.0, 10.0));
        const QImage shifted = renderCanvas(*canvas_);
        const int builds = canvas_->presentCacheRebuildCount();

        // A content pan is a pan: the present cache patches instead of re-cropping.
        canvas_->setCropContentOffset(QPointF(20.0, 20.0));
        const QImage shiftedMore = renderCanvas(*canvas_);
        QCOMPARE(canvas_->presentCacheRebuildCount(), builds);
        QVERIFY2(!samePixels(shifted, shiftedMore), "the content moved");

        // The same content through the equivalent view pan.
        canvas_->setCropContentOffset(QPointF());
        canvas_->panBy(QPointF(10.0 * zoom, 10.0 * zoom));
        const QImage panned = renderCanvas(*canvas_);
        QVERIFY2(samePixels(shifted, panned), "the content pan is the view pan");
        QCOMPARE(canvas_->presentCacheRebuildCount(), builds);

        // Above 50 % the pan still draws level 0, so it equals the direct draw.
        if (zoom >= 1.0) {
            canvas_->setPresentLevelCropForTest(false);
            const QImage direct = renderCanvas(*canvas_);
            canvas_->setPresentLevelCropForTest(true);
            QVERIFY2(samePixels(direct, panned), "the panned present is the direct draw");

            // Profile note for the issue: the removed per-frame work was one
            // full-document level-0 crop; quote it against a panned frame.
            QElapsedTimer clock;
            clock.start();
            const QImage whole = canvas_->image();
            const double fullDocMs = double(clock.nsecsElapsed()) / 1e6;
            clock.restart();
            renderCanvas(*canvas_);
            const double panFrameMs = double(clock.nsecsElapsed()) / 1e6;
            qInfo("pictura profile: crop_pan 4000x3000 full_document_crop_ms=%.2f "
                  "pan_frame_ms=%.2f whole=%dx%d",
                  fullDocMs, panFrameMs, whole.width(), whole.height());
        }
        QVERIFY(!canvas_->holdsImageForTest());

        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void PresentCacheTest::cropPanKeepsTheSingleChannel()
{
    QVERIFY2(newDoc(520, 120), "document fixture");
    view_->set_gpu_compute(false);
    QVERIFY(replayExactRed(*view_, 100, {QPointF(260.0, 60.0)}));
    setZoom(1.0);

    // The 520x120 document is centred at 100 %, so the dab sits at the widget centre.
    const QPoint centre = canvas_->rect().center();
    const QPoint outsideDab = centre + QPoint(55, 0);  // just past the 50 px radius
    const QImage composite = renderCanvas(*canvas_);
    QCOMPARE(composite.pixelColor(centre), QColor(Qt::red));
    QCOMPARE(composite.pixelColor(outsideDab), QColor(Qt::white));

    // Green: the red dab is black, the background white. The 10 px pan brings the
    // dab over a point it did not cover, so a black sample proves the channel moved.
    canvas_->setChannelMask(0x2);
    canvas_->setCropContentOffset(QPointF(10.0, 10.0));
    const QImage channel = renderCanvas(*canvas_);
    QCOMPARE(channel.pixelColor(outsideDab), QColor(Qt::black));
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

void PresentCacheTest::previewStrokeUsesTilesAndRestoresZoom()
{
    QVERIFY2(newDoc(2048, 2048), "preview fixture");
    view_->set_gpu_compute(false);
    setZoom(0.4);
    renderCanvas(*canvas_);
    const int zoomLevel = canvas_->presentLevelForTest();
    const int history = view_->history_count();
    const QImage before = view_->display_image(0, 650, 650, 700, 700);
    QVERIFY(view_->begin_paint(kRed, kWhite, 600, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    const int preview = pictura::preview_present_level(*view_);
    QVERIFY(preview > zoomLevel);
    QVERIFY(view_->paint_dab(1000.0, 1000.0, 1.0));
    view_->flush_present();
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), preview);
    QCOMPARE(view_->history_count(), history);
    QVERIFY(samePixels(view_->display_image(0, 650, 650, 700, 700), before));
    QVERIFY(view_->end_paint());
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), zoomLevel);
    QCOMPARE(pictura::preview_present_level(*view_), 0);
    QCOMPARE(view_->history_count(), history + 1);
    const QImage committed = view_->image();

    QVERIFY(view_->undo());
    renderCanvas(*canvas_);
    const QImage restored = view_->display_image(0, 650, 650, 700, 700);
    QVERIFY(samePixels(restored, before));
    const int cancelledHistory = view_->history_count();
    QVERIFY(view_->begin_paint(kRed, kWhite, 600, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view_->paint_dab(1000.0, 1000.0, 1.0));
    view_->flush_present();
    renderCanvas(*canvas_);
    view_->cancel_paint();
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), zoomLevel);
    QCOMPARE(pictura::preview_present_level(*view_), 0);
    QCOMPARE(view_->history_count(), cancelledHistory);
    QVERIFY(samePixels(view_->display_image(0, 650, 650, 700, 700), before));

    pictura::PictureView exact;
    QVERIFY(exact.new_document(2048, 2048,
                               QStringLiteral("rgb"), 8, QStringLiteral("white")));
    exact.set_gpu_compute(false);
    exact.set_active_layer(QStringLiteral("0"));
    QVERIFY(replayExactRed(exact, 600, {QPointF(1000.0, 1000.0)}));
    QVERIFY(samePixels(committed, exact.image()));
}

void PresentCacheTest::frameBoundedPaintMatchesCpu_data()
{
    QTest::addColumn<bool>("gpu");
    QTest::newRow("cpu") << false;
    QTest::newRow("gpu") << true;
}

void PresentCacheTest::frameBoundedPaintMatchesCpu()
{
    QFETCH(bool, gpu);
    QVERIFY(newDoc(1024, 1024));
    view_->set_gpu_compute(gpu);
    if (gpu && !view_->gpu_available()) {
        QSKIP("GPU paint adapter unavailable; CPU case runs separately");
    }
    setZoom(0.4);
    const int diameter = gpu ? 600 : 8;
    const QList<QPointF> samples{QPointF(300, 300), QPointF(450, 450),
                                QPointF(600, 600), QPointF(750, 750)};
    const quint64 revision = view_->canvas_revision();
    QVERIFY(view_->begin_paint(kRed, kWhite, diameter, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QCOMPARE(pictura::preview_present_level(*view_), 0);
    for (const QPointF& sample : samples) {
        QVERIFY(view_->paint_dab(sample.x(), sample.y(), 1.0));
    }
    const quint64 beforeFlush = view_->canvas_revision();
    QVERIFY(beforeFlush - revision <= 1);
    QVERIFY(view_->flush_present());
    QCOMPARE(view_->canvas_revision(), beforeFlush + 1);
    QVERIFY(!view_->flush_present());
    QCOMPARE(view_->canvas_revision(), beforeFlush + 1);
    renderCanvas(*canvas_);
    QCOMPARE(canvas_->presentLevelForTest(), 1);

    const QImage working = view_->image();
    const auto levels = pictura::document_api::pyramidSizes(*view_);
    for (int level = 0; level < levels.size(); ++level) {
        const auto size = levels.at(level);
        const QImage tiles = view_->display_image(level, 0, 0, size.width, size.height);
        QVERIFY(samePixels(tiles, referenceLevel(working, level)));
    }
    QVERIFY(view_->end_paint());
    const QImage committed = view_->image();
    pictura::PictureView cpu;
    QVERIFY(cpu.new_document(1024, 1024, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    cpu.set_active_layer(QStringLiteral("0"));
    cpu.set_gpu_compute(false);
    QVERIFY(replayExactRed(cpu, diameter, samples));
    QVERIFY(samePixels(committed, cpu.image()));
}

void PresentCacheTest::panelsUseTileCrops()
{
    auto* navigator = window_->findChild<pictura::NavigatorPanel*>(QStringLiteral("navigatorPanel"));
    auto* histogram = window_->findChild<pictura::HistogramPanel*>(QStringLiteral("histogramPanel"));
    QVERIFY(navigator && histogram);
    for (const QSize size : {QSize(1024, 768), QSize(2048, 1536), QSize(4096, 2)}) {
        QVERIFY(newDoc(size.width(), size.height()));
        view_->set_gpu_compute(false);
        navigator->setCanvas(canvas_);
        navigator->setView(view_);
        histogram->setView(view_);
        const auto levels = pictura::document_api::pyramidSizes(*view_);
        QVERIFY(!levels.isEmpty());
        const auto last = levels.last();
        const QSize sourceSize(last.width, last.height);
        const QImage expected = view_->display_image(levels.size() - 1, 0, 0,
                                                     last.width, last.height);
        navigator->refresh();
        histogram->refresh();
        QCOMPARE(navigator->thumbnailSourceSizeForTest(), sourceSize);
        QVERIFY(samePixels(navigator->thumbnailSourceImageForTest(), expected));
        QCOMPARE(histogram->sourceSizeForTest(), sourceSize);
        QImage binned = expected;
        if (sourceSize.width() > 512 || sourceSize.height() > 512) {
            binned = expected.scaled(512, 512, Qt::KeepAspectRatio, Qt::FastTransformation);
        }
        QCOMPARE(histogram->binnedSizeForTest(), binned.size());
        quint64 count = 0;
        for (int bin = 0; bin < 256; ++bin) {
            count += histogram->binForTest(0, bin);
        }
        QCOMPARE(count, quint64(binned.width()) * quint64(binned.height()));
        QVERIFY(!canvas_->holdsImageForTest());
        if (size.height() == 2) {
            QCOMPARE(levels.size(), 1);
            QCOMPARE(sourceSize, size);
        } else {
            QVERIFY(sourceSize.width() <= 512 && sourceSize.height() <= 512);
            QVERIFY(navigator->thumbnailSourceSizeForTest() != size);
        }
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

PICTURA_QT_TEST_MAIN(PresentCacheTest)
#include "tst_present_cache.moc"
