// The live selection outline is clipped to the canvas (#796): a shape that
// leaves the canvas is drawn as the part of it inside the canvas, closed along
// the border, so the rubber band and the committed outline agree at the edge.

#include <QtTest/QtTest>

#include "selftest_paint_fixture.h"

#include "api/selection_api.h"
#include "commands.h"
#include "qt_test_support.h"
#include "selection_geometry.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QtMath>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPolygonF>

#include <array>
#include <memory>
#include <set>
#include <utility>

namespace {
using paint_fixture::Fixture;

// The canvas the fixtures open.
constexpr int kCanvas = 64;

// A vertex of `loop` at `p`, compared exactly: a point-in-polygon test is
// ambiguous on an edge, and the clip puts vertices exactly on the border.
bool hasVertex(const QPolygonF& loop, const QPointF& p)
{
    for (const QPointF& v : loop) {
        if (v == p) {
            return true;
        }
    }
    return false;
}

// The undirected segments the loop draws, so two outlines are compared as the
// shapes they draw and not as the order they list their vertices in.
using Segment = std::array<int, 4>;

std::set<Segment> segments(const QPolygonF& loop)
{
    std::set<Segment> out;
    const qsizetype n = loop.size();
    for (qsizetype i = 0; i < n; ++i) {
        const QPointF a = loop[i];
        const QPointF b = loop[(i + 1) % n];
        int x0 = qRound(a.x());
        int y0 = qRound(a.y());
        int x1 = qRound(b.x());
        int y1 = qRound(b.y());
        if (x1 < x0 || (x1 == x0 && y1 < y0)) {
            std::swap(x0, x1);
            std::swap(y0, y1);
        }
        out.insert(Segment{x0, y0, x1, y1});
    }
    return out;
}

// Every vertex of the loop inside the canvas rectangle.
bool insideCanvas(const QPolygonF& loop)
{
    for (const QPointF& v : loop) {
        if (v.x() < 0.0 || v.y() < 0.0 || v.x() > kCanvas || v.y() > kCanvas) {
            return false;
        }
    }
    return true;
}

// A vertex of the loop on the canvas border.
bool touchesBorder(const QPolygonF& loop)
{
    for (const QPointF& v : loop) {
        if (v.x() == kCanvas || v.y() == kCanvas) {
            return true;
        }
    }
    return false;
}
} // namespace

class SelectionPreviewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void clipToCanvasClosesAlongTheBorder();
    void clipToCanvasLeavesInnerShapesAlone();
    void clipToCanvasDropsOuterShapes();
    void clipToCanvasKeepsDegenerateLoops();
    void clipToCanvasKeepsATraceThatEnclosesNothing();
    void clipToCanvasClosesASelfOverlappingShape();
    void marqueePastTheEdgeClosesAlongTheBorder();
    void ellipticalMarqueePastTheEdgeClosesAlongTheBorder();
    void lassoPastTheEdgeClosesAlongTheBorder();
    void magneticLassoPastTheEdgeClosesAlongTheBorder();
    void regionTracePastTheEdgeClosesAlongTheBorder();
    void liveOutlineMatchesTheCommittedOne();
    void polygonalBandStaysOpenAtTheBorder();
    void combineModeShowsTheNewShape();
    void zoomMarqueeStaysUnclipped();
    void committedOutlineStaysVisibleAtBorderAcrossPanAndZoom();
    void anEditKeepsTheCachedContour();
    void aSelectionChangeRetracesTheContour();
    void antsTickRepaintsOnlyTheOutline();

private:
    static QImage seed();
    pictura::SelectionOptions* selectOptions(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SelectionPreviewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

QImage SelectionPreviewTest::seed()
{
    QImage image(kCanvas, kCanvas, QImage::Format_RGB32);
    image.fill(Qt::white);
    QPainter(&image).fillRect(10, 10, 20, 20, Qt::black);
    return image;
}

// The selection tools share one options object, so the state a previous test
// left behind is reset here.
pictura::SelectionOptions* SelectionPreviewTest::selectOptions(Fixture& f)
{
    auto* options = f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee);
    if (!options) {
        return nullptr;
    }
    options->setCombineMode(pictura::SelectionMode::New);
    options->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    options->setFeather(0.0);
    options->setMagneticWidth(6);
    options->setMagneticContrast(10);
    options->setMagneticFrequency(0);
    return options;
}

void SelectionPreviewTest::clipToCanvasClosesAlongTheBorder()
{
    const QList<QPolygonF> clipped = pictura::clipSelectionShapeToCanvas(
        {QPolygonF(QRectF(50, 50, 50, 50))}, QSize(kCanvas, kCanvas));
    QCOMPARE(int(clipped.size()), 1);
    const QPolygonF& loop = clipped.first();
    // The run along the border and the corner are part of the outline.
    QVERIFY(hasVertex(loop, QPointF(kCanvas, 50)));
    QVERIFY(hasVertex(loop, QPointF(kCanvas, kCanvas)));
    QVERIFY(hasVertex(loop, QPointF(50, kCanvas)));
    QVERIFY(!hasVertex(loop, QPointF(100, 50)));
}

void SelectionPreviewTest::clipToCanvasLeavesInnerShapesAlone()
{
    const QPolygonF shape(QRectF(10, 10, 20, 20));
    const QList<QPolygonF> clipped = pictura::clipSelectionShapeToCanvas({shape},
                                                                         QSize(kCanvas, kCanvas));
    QCOMPARE(int(clipped.size()), 1);
    QCOMPARE(int(clipped.first().size()), int(shape.size()));
    for (qsizetype i = 0; i < shape.size(); ++i) {
        QVERIFY(clipped.first().at(i) == shape.at(i));
    }
}

void SelectionPreviewTest::clipToCanvasDropsOuterShapes()
{
    const QList<QPolygonF> clipped = pictura::clipSelectionShapeToCanvas(
        {QPolygonF(QRectF(80, 80, 20, 20))}, QSize(kCanvas, kCanvas));
    QVERIFY(clipped.isEmpty());
}

void SelectionPreviewTest::clipToCanvasKeepsDegenerateLoops()
{
    // A trace that has not closed yet is one point; it must not disappear.
    QPolygonF point;
    point << QPointF(5, 5);
    const QList<QPolygonF> clipped = pictura::clipSelectionShapeToCanvas({point},
                                                                         QSize(kCanvas, kCanvas));
    QCOMPARE(int(clipped.size()), 1);
    QCOMPARE(int(clipped.first().size()), 1);
    QVERIFY(clipped.first().first() == point.first());
}

void SelectionPreviewTest::clipToCanvasKeepsATraceThatEnclosesNothing()
{
    // A stroke that goes out past the canvas edge and comes back encloses
    // nothing: the band must stay, cropped at the border, instead of vanishing.
    QPolygonF stroke;
    stroke << QPointF(20, 20) << QPointF(90, 20) << QPointF(90, 20) << QPointF(60, 20)
           << QPointF(20, 20);
    const QList<QPolygonF> clipped = pictura::clipSelectionShapeToCanvas({stroke},
                                                                         QSize(kCanvas, kCanvas));
    QCOMPARE(int(clipped.size()), 1);
    QCOMPARE(int(clipped.first().size()), int(stroke.size()));
    QVERIFY(clipped.first().first() == stroke.first());
}

void SelectionPreviewTest::clipToCanvasClosesASelfOverlappingShape()
{
    // A trace that overlaps itself: the band is the boundary of the region it
    // encloses, closed along the border, which is the region the release
    // selects.
    QPolygonF shape;
    shape << QPointF(20, 20) << QPointF(90, 20) << QPointF(20, 50) << QPointF(90, 50)
          << QPointF(20, 20);
    const QList<QPolygonF> clipped = pictura::clipSelectionShapeToCanvas({shape},
                                                                         QSize(kCanvas, kCanvas));
    QCOMPARE(int(clipped.size()), 1);
    QVERIFY(insideCanvas(clipped.first()));
    QVERIFY(touchesBorder(clipped.first()));
}

void SelectionPreviewTest::marqueePastTheEdgeClosesAlongTheBorder()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_marquee"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::Marquee);
    QVERIFY(selectOptions(f) != nullptr);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 100));

    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    // The outline runs to the border, turns the corner there, and follows it.
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, 50)));
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, kCanvas)));
    QVERIFY(hasVertex(loops.first(), QPointF(50, kCanvas)));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::ellipticalMarqueePastTheEdgeClosesAlongTheBorder()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_ellipse"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::EllipticalMarquee);
    QVERIFY(selectOptions(f) != nullptr);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 100));

    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    // The ellipse is closed along the border: it reaches the corner.
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, kCanvas)));
    QVERIFY(touchesBorder(loops.first()));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::lassoPastTheEdgeClosesAlongTheBorder()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_lasso"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::Lasso);
    QVERIFY(selectOptions(f) != nullptr);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 50));
    f.canvas->mouseMoved(QPointF(100, 100));

    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    // The trace is a triangle whose hypotenuse runs through the canvas corner,
    // so the closed outline carries the border run and the corner.
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, 50)));
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, kCanvas)));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::magneticLassoPastTheEdgeClosesAlongTheBorder()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_magnetic"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::MagneticLasso);
    QVERIFY(selectOptions(f) != nullptr);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 100));

    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    // The wire leaves the canvas: every drawn vertex is inside it.
    QVERIFY(insideCanvas(loops.first()));
    QVERIFY(touchesBorder(loops.first()));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::regionTracePastTheEdgeClosesAlongTheBorder()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_region"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::Patch);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 50));
    f.canvas->mouseMoved(QPointF(100, 100));

    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    // The region trace is closed along the border, like every other shape.
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, 50)));
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, kCanvas)));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::liveOutlineMatchesTheCommittedOne()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_match"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::Marquee);
    QVERIFY(selectOptions(f) != nullptr);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 100));
    const QPolygonF live = f.canvas->selectionPreviewContours().value(0);
    f.canvas->mouseReleased(QPointF(100, 100));
    window_->refreshSelectionOverlay();

    const QList<QPolygonF>& committed = f.canvas->selectionContours();
    QCOMPARE(int(committed.size()), 1);
    // The same region, compared as the shapes they draw: the clipped live
    // outline and the traced mask contour list their vertices in another order.
    const std::set<Segment> liveEdges = segments(live);
    const std::set<Segment> committedEdges = segments(committed.first());
    QCOMPARE(int(liveEdges.size()), int(committedEdges.size()));
    QVERIFY2(liveEdges == committedEdges,
             "the live outline and the committed contour draw different shapes");
}

void SelectionPreviewTest::polygonalBandStaysOpenAtTheBorder()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_polygonal"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::PolygonalLasso);
    QVERIFY(selectOptions(f) != nullptr);

    // The polygonal lasso adds a vertex per click, and the rubber band runs
    // from the last vertex to the pointer. The clicks are 50 px apart, so
    // neither the close-by-the-first-vertex rule nor the double-click rule
    // can fire.
    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseReleased(QPointF(50, 50));
    f.canvas->mousePressed(QPointF(100, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseReleased(QPointF(100, 50));
    f.canvas->mousePressed(QPointF(100, 100), Qt::LeftButton, int(Qt::NoModifier));

    // The band is open: it has no closing edge, so it is not closed along the
    // border and keeps the trace the user laid down.
    QVERIFY(f.canvas->selectionPreviewOpenForTest());
    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    QCOMPARE(int(loops.first().size()), 3);
    QVERIFY(hasVertex(loops.first(), QPointF(100, 100)));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::combineModeShowsTheNewShape()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_combine"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::Marquee);
    QVERIFY(selectOptions(f) != nullptr);
    QVERIFY(f.view->select_rect(5, 5, 10, 10, QStringLiteral("new"), 0.0));
    window_->refreshSelectionOverlay();

    auto* options = f.tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee);
    options->setCombineMode(pictura::SelectionMode::Add);
    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 100));

    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    // The live outline is the shape being dragged, not the union with the
    // selection that is already there.
    QVERIFY(hasVertex(loops.first(), QPointF(kCanvas, kCanvas)));
    QVERIFY(!hasVertex(loops.first(), QPointF(5, 5)));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::zoomMarqueeStaysUnclipped()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_zoom"));
    QVERIFY2(f.ok(), "selection fixture");
    window_->setActiveTool(pictura::ToolId::Zoom);

    f.canvas->mousePressed(QPointF(50, 50), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(100, 100));

    // The zoom rubber band is not a selection shape: it keeps its own corners.
    const QList<QPolygonF>& loops = f.canvas->selectionPreviewContours();
    QCOMPARE(int(loops.size()), 1);
    QVERIFY(hasVertex(loops.first(), QPointF(100, 100)));
    f.canvas->mouseReleased(QPointF(100, 100));
}

void SelectionPreviewTest::committedOutlineStaysVisibleAtBorderAcrossPanAndZoom()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_preview_border"));
    QVERIFY2(f.ok(), "selection fixture");
    // The backdrop is white too, so a dark pixel near the border can only be
    // the ant dash, never the pasteboard.
    f.canvas->setCanvasColor(QColor(Qt::white));
    f.view->select_all();
    QVERIFY(f.view->has_selection());
    window_->refreshSelectionOverlay();

    const QList<QPolygonF> before = f.canvas->selectionContours();
    const pictura::selection_api::SelectionContour traced =
        pictura::selection_api::contour(*f.view);
    QVERIFY(!before.isEmpty());
    QVERIFY(!traced.isEmpty());
    const QList<QPolygonF> tracedLoops(traced.loops.begin(), traced.loops.end());
    QVERIFY(before == tracedLoops);

    // A dark dash within one device pixel of the reported border, anywhere
    // along that edge.
    const auto darkDashAlong = [](const QImage& shot, const QPointF& from, const QPointF& to,
                                  bool horizontal) {
        const int first = qRound(horizontal ? from.x() : from.y());
        const int last = qRound(horizontal ? to.x() : to.y());
        const int fixed = qRound(horizontal ? from.y() : from.x());
        for (int t = first; t <= last; ++t) {
            for (int d = -1; d <= 1; ++d) {
                const int x = horizontal ? t : fixed + d;
                const int y = horizontal ? fixed + d : t;
                if (x < 0 || x >= shot.width() || y < 0 || y >= shot.height()) {
                    continue;
                }
                const QColor colour = shot.pixelColor(x, y);
                if (colour.red() < 80 && colour.green() < 80 && colour.blue() < 80) {
                    return true;
                }
            }
        }
        return false;
    };

    const QList<QPointF> offsets{QPointF(40.25, 30.75), QPointF(41.5, 31.5),
                                 QPointF(40.9, 30.1)};
    for (const double zoom : {1.0, 1.37, 2.21}) {
        for (const QPointF& offset : offsets) {
            f.canvas->setZoom(zoom, QPointF(0, 0));
            f.canvas->setOffset(offset);
            f.canvas->update();
            QCoreApplication::processEvents();
            const QImage shot = f.canvas->grab().toImage();
            QVERIFY(!shot.isNull());

            const QPointF tl = f.canvas->imageToWidget(QPointF(0, 0));
            const QPointF tr = f.canvas->imageToWidget(QPointF(kCanvas, 0));
            const QPointF bl = f.canvas->imageToWidget(QPointF(0, kCanvas));
            const QPointF br = f.canvas->imageToWidget(QPointF(kCanvas, kCanvas));
            const QString where = QStringLiteral("zoom %1 offset %2,%3")
                                      .arg(zoom)
                                      .arg(offset.x())
                                      .arg(offset.y());
            QVERIFY2(darkDashAlong(shot, tl, tr, true),
                     qPrintable(where + QStringLiteral(" top")));
            QVERIFY2(darkDashAlong(shot, bl, br, true),
                     qPrintable(where + QStringLiteral(" bottom")));
            QVERIFY2(darkDashAlong(shot, tl, bl, false),
                     qPrintable(where + QStringLiteral(" left")));
            QVERIFY2(darkDashAlong(shot, tr, br, false),
                     qPrintable(where + QStringLiteral(" right")));
            QVERIFY(f.canvas->selectionContours() == before);
            const pictura::selection_api::SelectionContour after =
                pictura::selection_api::contour(*f.view);
            QVERIFY(QList<QPolygonF>(after.loops.begin(), after.loops.end()) == tracedLoops);
        }
    }
}

void SelectionPreviewTest::anEditKeepsTheCachedContour()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_cache_hit"));
    QVERIFY2(f.ok(), "selection fixture");
    QVERIFY(f.view->select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    window_->refreshSelectionOverlay();

    const quint64 computes = f.view->selection_contour_compute_count();
    const int sets = f.canvas->selectionContourSetCountForTest();
    QVERIFY(!f.view->add_layer_in(QString()).isEmpty());

    QCOMPARE(f.view->selection_contour_compute_count(), computes);
    QCOMPARE(f.canvas->selectionContourSetCountForTest(), sets);
}

void SelectionPreviewTest::aSelectionChangeRetracesTheContour()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_cache_change"));
    QVERIFY2(f.ok(), "selection fixture");
    QVERIFY(f.view->select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    window_->refreshSelectionOverlay();

    const quint64 computes = f.view->selection_contour_compute_count();
    const int firstSets = f.canvas->selectionContourSetCountForTest();

    const int windows = window_->windowCountForTest(f.doc);
    QVERIFY(window_->registry()->dispatch(QStringLiteral("window.arrange.new.window.for.file")));
    QTRY_COMPARE_WITH_TIMEOUT(window_->windowCountForTest(f.doc), windows + 1, 1000);
    pictura::ImageView* second = window_->windowCanvasForTest(f.doc, 1);
    QVERIFY(second != nullptr);

    QVERIFY(f.view->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    QCOMPARE(f.view->selection_contour_compute_count(), computes + 1);
    QVERIFY(f.canvas->selectionContourSetCountForTest() > firstSets);
    QVERIFY(second->selectionContourSetCountForTest() > 0);

    QVERIFY(!f.canvas->selectionContours().isEmpty());
    QVERIFY(hasVertex(f.canvas->selectionContours().first(), QPointF(4, 4)));
    QVERIFY(hasVertex(f.canvas->selectionContours().first(), QPointF(12, 12)));
    QVERIFY(!second->selectionContours().isEmpty());
    QVERIFY(hasVertex(second->selectionContours().first(), QPointF(4, 4)));
    QVERIFY(hasVertex(second->selectionContours().first(), QPointF(12, 12)));

    f.view->deselect();
    window_->refreshSelectionOverlay();
    QVERIFY(!f.canvas->hasSelectionContourForTest());
    QVERIFY(!second->hasSelectionContourForTest());
}

void SelectionPreviewTest::antsTickRepaintsOnlyTheOutline()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_sel_ants_region"));
    QVERIFY2(f.ok(), "selection fixture");
    QVERIFY(f.view->select_rect(10, 10, 20, 20, QStringLiteral("new"), 0.0));
    window_->refreshSelectionOverlay();

    window_->show();
    QTest::qWait(200);
    QVERIFY(f.canvas->isVisible());

    const int ticks = f.canvas->antsTicksForTest();
    QTRY_VERIFY_WITH_TIMEOUT(f.canvas->antsTicksForTest() > ticks, 1000);
    QTest::qWait(30);

    const QRect dirty = f.canvas->lastPaintRectForTest();
    QVERIFY(!dirty.isEmpty());
    QVERIFY(dirty.width() < f.canvas->width() || dirty.height() < f.canvas->height());

    QRectF selectionBounds;
    for (const QPolygonF& loop : f.canvas->selectionContours()) {
        selectionBounds = selectionBounds.united(
            loop.boundingRect().translated(f.canvas->selectionContourOffsetForTest()));
    }
    QVERIFY(!selectionBounds.isEmpty());
    const QRectF selectionWidget(f.canvas->imageToWidget(selectionBounds.topLeft()),
                                 f.canvas->imageToWidget(selectionBounds.bottomRight()));
    QVERIFY(dirty.contains(selectionWidget.normalized().toAlignedRect().adjusted(-2, -2, 2, 2)));

    const QImage moveBase(kCanvas, kCanvas, QImage::Format_ARGB32_Premultiplied);
    const QImage moveLayer(kCanvas, kCanvas, QImage::Format_ARGB32_Premultiplied);
    f.canvas->beginMovePreview(moveBase, moveLayer, QPointF(), 1.0);
    f.canvas->setMoveOutline(QRectF(40, 40, 10, 10));
    const int moveTicks = f.canvas->antsTicksForTest();
    QTRY_VERIFY_WITH_TIMEOUT(f.canvas->antsTicksForTest() > moveTicks, 1000);
    QTest::qWait(30);
    const QRect moveDirty = f.canvas->lastPaintRectForTest();
    const QRectF moveWidget(f.canvas->imageToWidget(QPointF(40, 40)),
                            f.canvas->imageToWidget(QPointF(50, 50)));
    QVERIFY(moveDirty.contains(moveWidget.normalized().toAlignedRect().adjusted(-2, -2, 2, 2)));
    f.canvas->endMovePreview();
}

QTEST_MAIN(SelectionPreviewTest)
#include "tst_selection_preview.moc"
