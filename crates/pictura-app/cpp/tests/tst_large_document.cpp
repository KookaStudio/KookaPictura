// Stroke boundaries on large documents (#185): a bounded undo/redo restores a
// stroke's pixels without rebuilding the canvas frame, an undo during a stroke
// cancels it, the Move tool does not warm a preview it may not use, and a layer
// property repaints no more than the layer it changed.

#include <QtTest/QtTest>

#include <QtCore/QElapsedTimer>
#include <QtWidgets/QProgressDialog>
#include <QtWidgets/QPushButton>

#include "commands.h"
#include "panels/info_panel.h"
#include "selftest_paint_fixture.h"

#include "api/document_bridge.h"
#include "api/paint_bridge.h"
#include "api/transform_bridge.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

constexpr quint32 kRed = 0xFFFF0000u;
constexpr quint32 kWhite = 0xFFFFFFFFu;

bool beginRed(pictura::PictureView& view)
{
    return view.begin_paint(kRed, kWhite, 12, 100, 100, 0, 100, 100, 25,
                            QStringLiteral("normal"), false, false);
}

// One committed horizontal stroke along row `y`.
bool paintLine(pictura::PictureView& view, double y)
{
    if (!beginRed(view)) {
        return false;
    }
    for (double x = 10.0; x <= 70.0; x += 6.0) {
        view.paint_dab(x, y, 1.0);
    }
    return view.end_paint();
}

QRgb canvasPixel(const pictura::ImageView& canvas, int x, int y)
{
    return canvas.image().pixel(x, y);
}

} // namespace

class LargeDocumentTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void boundedUndoKeepsTheFrame();
    void aNeutralLayerUndoKeepsTheFrame();
    void aLargeGeometryUndoLandsTheExactTiles();
    void undoDuringAStrokeCancelsIt();
    void moveDoesNotWarmTheBackground();
    void layerPropertiesRepaintOnlyTheLayer();
    void aDocumentCanvasHoldsNoImage();
    void aLargeMoveCommitLandsOffThread();
    void aSmallMoveCommitIsSynchronous();
    void aReadSettlesThePendingMove();
    void aSecondPressReusesTheFullPreview();
    void aSecondPressKeepsTheViewPreview();
    void aHoverReadoutDoesNotSettleTheMove();
    void aTransformCommitLandsOffThread();
    void cancellingATransformResumesTheSession();
    void aRotatedPixelEdgeIsAntialiased();

private:
    void showFrame();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LargeDocumentTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LargeDocumentTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void LargeDocumentTest::boundedUndoKeepsTheFrame()
{
    QImage seed(120, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_bounded_undo"));
    QVERIFY2(f.ok(), "bounded undo fixture");

    QVERIFY(paintLine(*f.view, 20.0));
    QCOMPARE(f.view->sample_argb(40, 20), kRed);
    QCOMPARE(canvasPixel(*f.canvas, 40, 20), QRgb(kRed));
    const quint64 frame = pictura::frame_revision(*f.view);

    QVERIFY(window_->registry()->dispatch(QStringLiteral("edit.undo")));
    QCOMPARE(f.view->sample_argb(40, 20), kWhite);
    QCOMPARE(canvasPixel(*f.canvas, 40, 20), QRgb(kWhite));
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);

    QVERIFY(window_->registry()->dispatch(QStringLiteral("edit.stepForward")));
    QCOMPARE(f.view->sample_argb(40, 20), kRed);
    QCOMPARE(canvasPixel(*f.canvas, 40, 20), QRgb(kRed));
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);
}

void LargeDocumentTest::aNeutralLayerUndoKeepsTheFrame()
{
    QImage seed(120, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_neutral_undo"));
    QVERIFY2(f.ok(), "neutral undo fixture");

    QVERIFY(f.view->add_layer(0) >= 0);
    const quint64 frame = pictura::frame_revision(*f.view);
    QVERIFY(window_->registry()->dispatch(QStringLiteral("edit.undo")));
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);
}

// A whole-document restore on a large document leaves its tiles stale; the
// canvas presents stand-ins at once and the exact visible tiles land from a
// worker (#1216).
void LargeDocumentTest::aLargeGeometryUndoLandsTheExactTiles()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_deferred_tiles"));
    QVERIFY2(f.ok(), "deferred tiles fixture");

    QVERIFY(f.view->resize_image(QStringLiteral("nearest"), 2100, 2100));
    QSignalSpy settled(f.view, &pictura::PictureView::pyramid_settled);
    QVERIFY(window_->registry()->dispatch(QStringLiteral("edit.undo")));
    QCOMPARE(f.view->document_width(), 4200);

    // The canvas asks for its visible tiles when it paints.
    f.canvas->repaint();
    QVERIFY(settled.count() > 0 || settled.wait(30000));
    QCOMPARE(pictura::preview_present_level(*f.view), 0);
    QCOMPARE(f.view->composite_argb(2100, 2100), kWhite);
}

void LargeDocumentTest::undoDuringAStrokeCancelsIt()
{
    QImage seed(120, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_undo_mid_stroke"));
    QVERIFY2(f.ok(), "undo mid-stroke fixture");

    QVERIFY(beginRed(*f.view));
    f.view->paint_dab(40.0, 30.0, 1.0);
    QVERIFY2(!f.view->undo(), "nothing to undo yet: the stroke must survive");
    QVERIFY(f.view->is_painting());
    QVERIFY(f.view->end_paint());

    QVERIFY(paintLine(*f.view, 10.0));
    QVERIFY(beginRed(*f.view));
    f.view->paint_dab(40.0, 50.0, 1.0);
    QVERIFY(f.view->undo());
    QVERIFY(!f.view->is_painting());
    QCOMPARE(f.view->sample_argb(40, 50), kWhite);
    QCOMPARE(f.view->sample_argb(40, 10), kWhite);
    QCOMPARE(f.view->sample_argb(40, 30), kRed);
}

void LargeDocumentTest::moveDoesNotWarmTheBackground()
{
    QImage seed(80, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_move_warm"));
    QVERIFY2(f.ok(), "move warm fixture");

    QVERIFY2(!f.view->prepare_move_preview(), "a Background may not be moved");
    QVERIFY(f.view->move_preview_base().isNull());

    const int added = f.view->add_layer(0);
    f.view->set_active_layer(QString::number(added));
    // A new layer is empty-rect (#1208); paint it so there is a preview.
    QVERIFY(paintLine(*f.view, 10.0));
    QVERIFY(f.view->prepare_move_preview());
    QVERIFY(!f.view->move_preview_base().isNull());
}

void LargeDocumentTest::layerPropertiesRepaintOnlyTheLayer()
{
    QImage seed(120, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_layer_properties"));
    QVERIFY2(f.ok(), "layer properties fixture");
    const int added = f.view->add_layer(0);
    f.view->set_active_layer(QString::number(added));
    QVERIFY(paintLine(*f.view, 20.0));
    const QStringList paths{QString::number(added)};

    const quint64 canvas = f.view->canvas_revision();
    const quint64 frame = pictura::frame_revision(*f.view);
    QCOMPARE(f.view->set_layers_lock(paths, QStringLiteral("pixels"), true), 1);
    QVERIFY(f.view->layer_lock(added) != 0);
    QCOMPARE(f.view->canvas_revision(), canvas);
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);

    QCOMPARE(f.view->set_layers_blend(paths, QStringLiteral("mul ")), 1);
    QCOMPARE(f.view->layer_blend(added), QStringLiteral("mul "));
    QVERIFY2(f.view->canvas_revision() != canvas, "the blend change was not presented");
    QCOMPARE(quint64(pictura::frame_revision(*f.view)), frame);
    QCOMPARE(f.view->sample_argb(40, 20), kRed);
    QCOMPARE(canvasPixel(*f.canvas, 40, 20), QRgb(kRed));
}

void LargeDocumentTest::aDocumentCanvasHoldsNoImage()
{
    QImage seed(90, 50, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_canvas_no_image"));
    QVERIFY2(f.ok(), "canvas fixture");
    QVERIFY(paintLine(*f.view, 20.0));
    QCOMPARE(f.canvas->documentSize(), QSize(90, 50));
    QVERIFY2(!f.canvas->holdsImageForTest(), "the canvas kept a full-resolution image");
    const QImage shown = f.canvas->image();
    QCOMPARE(shown.size(), QSize(90, 50));
    QCOMPARE(shown.pixel(40, 20), QRgb(kRed));
    QCOMPARE(shown.pixel(40, 45), QRgb(kWhite));
}

// [mcr_deferred] [mcr_preview]
void LargeDocumentTest::aLargeMoveCommitLandsOffThread()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_deferred_move"));
    QVERIFY2(f.ok(), "deferred move fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    pictura::move_composite_set_delay(*f.view, 400);
    QSignalSpy committed(f.view, &pictura::PictureView::moveCommitted);
    QSignalSpy settled(f.view, &pictura::PictureView::pyramid_settled);

    // Stand in for the drag preview: the release keeps it until the composite
    // lands.
    f.canvas->beginMovePreview(QImage(), QImage(), QPointF(), 1.0);
    QVERIFY(f.view->commit_move(4, 4));
    QVERIFY2(pictura::move_composite_pending(*f.view), "a large commit defers");
    QVERIFY2(f.canvas->movePreviewActive(), "the preview survives the release");
    QCOMPARE(committed.count(), 0);

    QVERIFY(committed.wait(30000));
    QVERIFY(!pictura::move_composite_pending(*f.view));
    QVERIFY2(!f.canvas->movePreviewActive(), "the signal drops the preview");
    QVERIFY2(settled.wait(30000), "the landing defers the exact pyramid");
    QCOMPARE(pictura::preview_present_level(*f.view), 0);
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Move Layer"));
}

// [mcr_small_sync]
void LargeDocumentTest::aSmallMoveCommitIsSynchronous()
{
    QImage seed(120, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_sync_move"));
    QVERIFY2(f.ok(), "sync move fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    QSignalSpy committed(f.view, &pictura::PictureView::moveCommitted);

    QVERIFY(f.view->commit_move(2, 2));
    QVERIFY(!pictura::move_composite_pending(*f.view));
    QCOMPARE(committed.count(), 1);
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Move Layer"));
}

// [mcr_settle_read]
void LargeDocumentTest::aReadSettlesThePendingMove()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_read_settles_move"));
    QVERIFY2(f.ok(), "read settle fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    QVERIFY(paintLine(*f.view, 100.0));
    pictura::move_composite_set_delay(*f.view, 500);
    QVERIFY(f.view->commit_move(4, 4));
    QVERIFY(pictura::move_composite_pending(*f.view));

    // The red line moved from row 100 to row 104; the stale composite still has
    // white there, so the sample must settle the worker's composite first.
    QCOMPARE(f.view->sample_argb(40, 104), kRed);
    QVERIFY(!pictura::move_composite_pending(*f.view));
}

// [mcr_second_press]
void LargeDocumentTest::aSecondPressReusesTheFullPreview()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_second_press_full"));
    QVERIFY2(f.ok(), "second press fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    QVERIFY(f.view->begin_move_preview());
    f.view->end_move_preview();
    pictura::move_composite_set_delay(*f.view, 500);
    QVERIFY(f.view->commit_move(4, 4));
    QVERIFY(pictura::move_composite_pending(*f.view));

    QElapsedTimer timer;
    timer.start();
    QVERIFY(f.view->begin_move_preview());
    const qint64 elapsed = timer.elapsed();
    QVERIFY2(f.view->move_preview_cache_hit(), "the second press reuses the cached base");
    QVERIFY2(elapsed < 1000, "the second press must not settle the whole composite");
    f.view->end_move_preview();
}

// [mcr_second_press]
void LargeDocumentTest::aSecondPressKeepsTheViewPreview()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_second_press_view"));
    QVERIFY2(f.ok(), "second view press fixture");
    showFrame();
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    f.canvas->setZoom(0.125, QPointF(f.canvas->width() / 2.0, f.canvas->height() / 2.0));
    f.canvas->repaint();
    QVERIFY(f.view->begin_move_preview());
    QVERIFY2(pictura::move_preview_scale(*f.view) > 1, "the first press is a view preview");
    f.view->end_move_preview();
    pictura::move_composite_set_delay(*f.view, 500);
    QVERIFY(f.view->commit_move(4, 4));
    QVERIFY(pictura::move_composite_pending(*f.view));

    QSignalSpy committed(f.view, &pictura::PictureView::moveCommitted);
    QElapsedTimer timer;
    timer.start();
    QVERIFY(f.view->begin_move_preview());
    const qint64 elapsed = timer.elapsed();
    QVERIFY2(pictura::move_preview_scale(*f.view) > 1, "the second press is a view preview");
    QVERIFY2(elapsed < 1000, "the second press must not settle the whole composite");
    f.canvas->beginMovePreview(f.view->move_preview_base(), f.view->move_preview_layer(),
                               QPointF(f.view->move_preview_x(), f.view->move_preview_y()),
                               f.view->move_preview_opacity() / 255.0);

    // The first commit lands while the second drag is down; it must not clear
    // the second drag's preview.
    QTest::qWait(2000); // wait out the 500 ms worker delay and its composite
    QVERIFY(f.canvas->movePreviewActive());
    QCOMPARE(committed.count(), 0);
    f.view->end_move_preview();
    f.canvas->endMovePreview();
}

// [mcr_hover_no_settle]
void LargeDocumentTest::aHoverReadoutDoesNotSettleTheMove()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_hover_settles_move"));
    QVERIFY2(f.ok(), "hover readout fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    pictura::move_composite_set_delay(*f.view, 500);
    QVERIFY(f.view->commit_move(4, 4));
    QVERIFY(pictura::move_composite_pending(*f.view));

    auto* panel = window_->findChild<pictura::InfoPanel*>(QStringLiteral("infoPanel"));
    QVERIFY(panel != nullptr);
    panel->setCursorPosition(QPointF(40, 104));
    QElapsedTimer timer;
    timer.start();
    panel->refresh();
    QVERIFY2(timer.elapsed() < 1000, "the hover readout must not settle the composite");
    QVERIFY(pictura::move_composite_pending(*f.view));

    QSignalSpy committed(f.view, &pictura::PictureView::moveCommitted);
    QVERIFY(committed.wait(30000));
    panel->refresh();
    QVERIFY(!pictura::move_composite_pending(*f.view));
}

// [mcr_transform_deferred]
void LargeDocumentTest::aTransformCommitLandsOffThread()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_deferred_transform"));
    QVERIFY2(f.ok(), "deferred transform fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    QVERIFY(f.tools->beginFreeTransform(QStringLiteral("1")));
    const QPolygonF quad = pictura::transform_api::quad(*f.view).corners;
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(2);
    QVERIFY(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false) >= 0);
    QVERIFY(f.view->transform_move(corner.x() - 200.0, corner.y() - 200.0, 1.0, false, false));
    f.view->transform_release();

    pictura::transform_set_delay(*f.view, 800);
    QSignalSpy committed(f.view, &pictura::PictureView::moveCommitted);
    const int count = f.view->history_count();
    bool sawWindow = false;
    bool sawHandles = true;
    int sawValue = 0;
    QTimer probe;
    probe.setInterval(20);
    connect(&probe, &QTimer::timeout, this, [&] {
        auto* d = window_->findChild<QProgressDialog*>(QStringLiteral("jobProgress"));
        if (d && d->isVisible()) {
            sawWindow = true;
            sawValue = std::max(sawValue, d->value());
            sawHandles = sawHandles && f.canvas->transformPreviewActive();
        }
    });
    probe.start();
    f.tools->commitFreeTransform();
    probe.stop();
    QVERIFY2(sawWindow, "the progress window appeared");
    QVERIFY2(sawValue > 0, "the progress value rose");
    QVERIFY2(sawHandles, "the handles stay while the job runs");
    QVERIFY(!pictura::transform_pending(*f.view));
    QVERIFY2(!f.canvas->transformPreviewActive(), "the transform handles are gone");
    // The history state is captured when the deferred composite lands.
    if (pictura::move_composite_pending(*f.view)) {
        QVERIFY2(f.canvas->movePreviewActive(), "the transform image waits for the composite");
        QVERIFY(committed.wait(30000));
    }
    QVERIFY(!pictura::move_composite_pending(*f.view));
    QVERIFY(!f.canvas->movePreviewActive());
    QCOMPARE(f.view->history_count(), count + 1);
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Free Transform"));
}

// [mcr_transform_cancel] [lop_cancel_unchanged]
void LargeDocumentTest::cancellingATransformResumesTheSession()
{
    QImage seed(4200, 4200, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_cancelled_transform"));
    QVERIFY2(f.ok(), "cancelled transform fixture");
    QCOMPARE(f.view->duplicate_layer(0), 1);
    f.view->set_active_layer(QStringLiteral("1"));
    QVERIFY(f.tools->beginFreeTransform(QStringLiteral("1")));
    const QPolygonF quad = pictura::transform_api::quad(*f.view).corners;
    QCOMPARE(quad.size(), 4);
    const QPointF corner = quad.at(2);
    QVERIFY(f.view->transform_press(corner.x(), corner.y(), 1.0, false, false) >= 0);
    QVERIFY(f.view->transform_move(corner.x() - 200.0, corner.y() - 200.0, 1.0, false, false));
    f.view->transform_release();
    const QPolygonF moved = pictura::transform_api::quad(*f.view).corners;
    const int count = f.view->history_count();
    const int index = f.view->history_index();
    const QImage before = f.view->display_image(2, 0, 0, 1050, 1050);

    pictura::transform_set_delay(*f.view, 30000);
    bool clicked = false;
    QTimer probe;
    probe.setInterval(20);
    connect(&probe, &QTimer::timeout, this, [&] {
        auto* d = window_->findChild<QProgressDialog*>(QStringLiteral("jobProgress"));
        if (!clicked && d && d->isVisible()) {
            if (auto* cancel = d->findChild<QPushButton*>()) {
                clicked = true;
                cancel->click();
            }
        }
    });
    probe.start();
    QElapsedTimer clock;
    clock.start();
    f.tools->commitFreeTransform();
    probe.stop();
    QVERIFY2(clicked, "the progress window offered Cancel");
    QVERIFY2(clock.elapsed() < 20000, "the cancel stopped the held job");
    QVERIFY(!pictura::transform_pending(*f.view));
    QVERIFY2(f.view->transform_session_active(), "the session resumes");
    QCOMPARE(pictura::transform_api::quad(*f.view).corners, moved);
    QVERIFY2(f.canvas->transformPreviewActive(), "the handles are back");
    QCOMPARE(f.view->history_count(), count);
    QCOMPARE(f.view->history_index(), index);
    QVERIFY2(f.view->display_image(2, 0, 0, 1050, 1050) == before, "the document is unchanged");

    // The resumed session commits as one state.
    pictura::transform_set_delay(*f.view, 0);
    QSignalSpy committed(f.view, &pictura::PictureView::moveCommitted);
    f.tools->commitFreeTransform();
    QVERIFY(!f.view->transform_session_active());
    if (pictura::move_composite_pending(*f.view)) {
        QVERIFY(committed.wait(30000));
    }
    QCOMPARE(f.view->history_count(), count + 1);
}

// [mcr_transform_antialias]
void LargeDocumentTest::aRotatedPixelEdgeIsAntialiased()
{
    QImage seed(1280, 800, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_rotated_edge"));
    QVERIFY2(f.ok(), "rotated edge fixture");
    const int added = f.view->add_layer(0);
    QVERIFY(added > 0);
    f.view->set_active_layer(QString::number(added));
    QVERIFY(f.view->select_rect(200, 200, 400, 300, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    f.view->deselect();
    QVERIFY(f.view->begin_free_transform(QString::number(added)));
    QVERIFY(pictura::transform_set_angle(*f.view, 30.0));
    QVERIFY(f.view->commit_transform());

    bool partial = false;
    for (int y = 100; y < 700 && !partial; ++y) {
        for (int x = 100; x < 700; ++x) {
            const int r = qRed(f.view->composite_argb(x, y));
            if (r > 0 && r < 255) {
                partial = true;
                break;
            }
        }
    }
    QVERIFY2(partial, "the rotated edge has no partial-coverage pixels");
}

QTEST_MAIN(LargeDocumentTest)
#include "tst_large_document.moc"
