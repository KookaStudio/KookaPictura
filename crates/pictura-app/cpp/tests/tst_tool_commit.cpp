// Tool commits on large documents (#719): a stroke presents into the composite
// as it goes and commits only what is still pending, a cancel puts the
// composite back, a shape layer or a fill composites only the region it
// changed (the whole document when a layer style could spread it), and the
// control server's `tool_options` and `pointer` `down`/`move`/`up` drive a
// tool the way the profiler does.

#include <QtTest/QtTest>

#include "control_server.h"
#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object/impl_history/display.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

constexpr quint32 kRed = 0xFFFF0000u;
constexpr quint32 kWhite = 0xFFFFFFFFu;

// A seed with a different colour at every pixel, so a region composited from
// the wrong place, or left stale, shows.
QImage texturedSeed(int w, int h)
{
    QImage seed(w, h, QImage::Format_RGB32);
    for (int y = 0; y < h; ++y) {
        for (int x = 0; x < w; ++x) {
            seed.setPixel(x, y, qRgb((x * 7) % 256, (y * 5) % 256, (x + y) % 256));
        }
    }
    return seed;
}

// The stored composite on a 3 px grid.
QList<quint32> compositeGrid(const pictura::PictureView& view, int w, int h)
{
    QList<quint32> out;
    for (int y = 0; y < h; y += 3) {
        for (int x = 0; x < w; x += 3) {
            out.append(view.composite_argb(x, y));
        }
    }
    return out;
}

// Whether the composite a bounded commit left equals a full CPU recomposite.
// `set_gpu_compute` recomposites the whole document.
bool matchesFullRecomposite(pictura::PictureView& view, int w, int h)
{
    const QList<quint32> bounded = compositeGrid(view, w, h);
    view.set_gpu_compute(false);
    return bounded == compositeGrid(view, w, h);
}

pictura::ShapeSpec ellipse(double x0, double y0, double x1, double y1)
{
    pictura::ShapeSpec spec{};
    spec.kind = 2;
    spec.boxed = true;
    spec.x0 = x0;
    spec.y0 = y0;
    spec.x1 = x1;
    spec.y1 = y1;
    spec.align_edges = true;
    return spec;
}

bool gradient(pictura::PictureView& view, double x0, double x1)
{
    return pictura::draw_gradient(view, 0, kRed, 0xFF0000FFu, 0, QStringLiteral("normal"), 100,
                                  false, false, true, x0, 20.0, x1, 20.0);
}

} // namespace

class ToolCommitTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void aStrokeCommitsOnlyWhatIsPending();
    void aCancelledStrokePutsTheCompositeBack();
    void aShapeLayerCompositesItsBounds();
    void aFillCompositesItsRect();
    void aLayerStyleRecompositesTheDocument();
    void toolOptionsPatchAndReadBack();
    void pointerDownMoveUpPaints();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ToolCommitTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ToolCommitTest::aStrokeCommitsOnlyWhatIsPending()
{
    Fixture f(*window_, texturedSeed(300, 200), QStringLiteral("pictura_commit_stroke"));
    QVERIFY2(f.ok(), "stroke fixture");
    f.view->set_gpu_compute(false);
    const quint64 frame = pictura::frame_revision(*f.view);

    // A 200 px dab is split across threads; the presents patch the composite
    // and the last dabs stay pending until the commit.
    QVERIFY(f.view->begin_paint(kRed, kWhite, 200, 50, 100, 0, 80, 70, 25,
                                QStringLiteral("normal"), false, false));
    for (int i = 0; i < 8; ++i) {
        f.view->paint_dab(40.0 + i * 30.0, 90.0 + (i % 3) * 10.0, 1.0);
        if (i % 3 == 0) {
            f.view->flush_present();
        }
    }
    QVERIFY(f.view->end_paint());
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);
    QVERIFY2(f.view->composite_argb(150, 100) != f.view->composite_argb(150, 2),
             "the stroke reached the composite");
    QVERIFY(matchesFullRecomposite(*f.view, 300, 200));
}

void ToolCommitTest::aCancelledStrokePutsTheCompositeBack()
{
    Fixture f(*window_, texturedSeed(240, 160), QStringLiteral("pictura_commit_cancel"));
    QVERIFY2(f.ok(), "cancel fixture");
    f.view->set_gpu_compute(false);
    const QList<quint32> before = compositeGrid(*f.view, 240, 160);

    QVERIFY(f.view->begin_paint(kRed, kWhite, 150, 100, 100, 0, 100, 100, 25,
                                QStringLiteral("normal"), false, false));
    for (int i = 0; i < 5; ++i) {
        f.view->paint_dab(40.0 + i * 40.0, 80.0, 1.0);
        f.view->flush_present();
    }
    QCOMPARE(f.view->composite_argb(120, 80), kRed);
    f.view->cancel_paint();
    QVERIFY(!f.view->is_painting());
    QCOMPARE(compositeGrid(*f.view, 240, 160), before);
}

void ToolCommitTest::aShapeLayerCompositesItsBounds()
{
    Fixture f(*window_, texturedSeed(300, 200), QStringLiteral("pictura_commit_shape"));
    QVERIFY2(f.ok(), "shape fixture");
    f.view->set_gpu_compute(false);
    const quint64 frame = pictura::frame_revision(*f.view);
    const quint32 outside = f.view->composite_argb(10, 10);

    pictura::ShapeSpec spec = ellipse(60, 40, 240, 160);
    spec.stroke = true;
    spec.stroke_color = 0xFF00FF00u;
    spec.stroke_width = 6;
    spec.stroke_align = 2;
    QVERIFY(!pictura::shape_add_layer(*f.view, spec, kRed).isEmpty());
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);
    QCOMPARE(f.view->composite_argb(150, 100), kRed);
    QCOMPARE(f.view->composite_argb(10, 10), outside);
    QVERIFY(matchesFullRecomposite(*f.view, 300, 200));
}

void ToolCommitTest::aFillCompositesItsRect()
{
    Fixture f(*window_, texturedSeed(200, 120), QStringLiteral("pictura_commit_fill"));
    QVERIFY2(f.ok(), "fill fixture");
    f.view->set_gpu_compute(false);
    const quint64 frame = pictura::frame_revision(*f.view);
    QVERIFY(f.view->select_rect(30, 20, 90, 60, QStringLiteral("new"), 0.0));
    QVERIFY(gradient(*f.view, 30.0, 120.0));
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);
    QVERIFY(matchesFullRecomposite(*f.view, 200, 120));
}

void ToolCommitTest::aLayerStyleRecompositesTheDocument()
{
    Fixture f(*window_, texturedSeed(200, 120), QStringLiteral("pictura_commit_styled"));
    QVERIFY2(f.ok(), "styled fixture");
    f.view->set_gpu_compute(false);
    const int added = f.view->add_layer(0);
    const QString top = QString::number(added);
    QVERIFY(pictura::layer_style_set(*f.view, top, QStringLiteral("colorOverlay.on"), 1.0));
    pictura::layer_style_commit(*f.view, QStringLiteral("Layer Style"));
    f.view->set_active_layer(top);

    const quint64 frame = pictura::frame_revision(*f.view);
    QVERIFY(gradient(*f.view, 20.0, 80.0));
    QVERIFY2(quint64(pictura::frame_revision(*f.view)) != frame,
             "a styled document must recomposite whole");
    QVERIFY(matchesFullRecomposite(*f.view, 200, 120));
}

void ToolCommitTest::toolOptionsPatchAndReadBack()
{
    pictura::ControlServer server(window_.get(), QString());
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const int size = tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->size();

    const QJsonObject patched = server.dispatch(
        QStringLiteral("tool_options"),
        {{QStringLiteral("options"),
          QJsonObject{{QStringLiteral("brush"), QJsonObject{{QStringLiteral("size"), 321}}}}}});
    QVERIFY(patched.value(QStringLiteral("ok")).toBool());
    const QJsonObject options =
        patched.value(QStringLiteral("result")).toObject().value(QStringLiteral("options")).toObject();
    QCOMPARE(options.value(QStringLiteral("brush")).toObject().value(QStringLiteral("size")).toInt(),
             321);
    QCOMPARE(tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->size(), 321);

    const QJsonObject read = server.dispatch(QStringLiteral("tool_options"), {});
    QVERIFY(read.value(QStringLiteral("ok")).toBool());
    QVERIFY(read.value(QStringLiteral("result")).toObject().contains(QStringLiteral("options")));

    const QJsonObject bad = server.dispatch(QStringLiteral("tool_options"),
                                            {{QStringLiteral("options"), 5}});
    QVERIFY(!bad.value(QStringLiteral("ok")).toBool());
    QCOMPARE(bad.value(QStringLiteral("error")).toObject().value(QStringLiteral("code")).toString(),
             QStringLiteral("invalid_param"));

    tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(size);
}

void ToolCommitTest::pointerDownMoveUpPaints()
{
    QImage seed(120, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_commit_pointer"));
    QVERIFY2(f.ok(), "pointer fixture");
    paint_fixture::BrushState brush(f.tools);
    f.tools->setForeground(Qt::red);
    pictura::ControlServer server(window_.get(), QString());
    QVERIFY(server.dispatch(QStringLiteral("set_tool"), {{QStringLiteral("tool"), QStringLiteral("brush")}})
                .value(QStringLiteral("ok"))
                .toBool());
    QVERIFY(server
                .dispatch(QStringLiteral("tool_options"),
                          {{QStringLiteral("options"),
                            QJsonObject{{QStringLiteral("brush"),
                                         QJsonObject{{QStringLiteral("size"), 9},
                                                     {QStringLiteral("hardness"), 100},
                                                     {QStringLiteral("opacity"), 100},
                                                     {QStringLiteral("flow"), 100}}}}}})
                .value(QStringLiteral("ok"))
                .toBool());
    const int base = f.view->history_count();
    auto pointer = [&](const QString& op, double x) {
        return server
            .dispatch(QStringLiteral("pointer"), {{QStringLiteral("op"), op},
                                                  {QStringLiteral("space"), QStringLiteral("image")},
                                                  {QStringLiteral("x"), x},
                                                  {QStringLiteral("y"), 30.0}})
            .value(QStringLiteral("ok"))
            .toBool();
    };
    QVERIFY(pointer(QStringLiteral("down"), 20.0));
    QVERIFY(f.view->is_painting());
    for (double x = 30.0; x <= 100.0; x += 10.0) {
        QVERIFY(pointer(QStringLiteral("move"), x));
    }
    QVERIFY(pointer(QStringLiteral("up"), 100.0));
    QVERIFY(!f.view->is_painting());
    QCOMPARE(f.view->history_count(), base + 1);
    QCOMPARE(f.view->sample_argb(60, 30), kRed);
    QCOMPARE(f.view->sample_argb(60, 5), kWhite);

    // A plain move after the release carries no button: nothing paints.
    QVERIFY(pointer(QStringLiteral("move"), 10.0));
    QCOMPARE(f.view->history_count(), base + 1);
}

QTEST_MAIN(ToolCommitTest)
#include "tst_tool_commit.moc"
