// File > Place fits an oversized source to the canvas and keeps the placed
// smart-object source through the Place Free Transform's scale/translation.

#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QPolygonF>

#include "api/engine_bridge.h"
#include "api/layers_api.h"
#include "api/paint_bridge.h"
#include "api/transform_api.h"
#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "frame_place.h"
#include "image_view.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

std::unique_ptr<PicturaMainWindow> makeShownWindow()
{
    auto window = pictura::test::makeMainWindow();
    if (window) {
        window->show();
        QTest::qWait(0);
    }
    return window;
}

bool writeSourcePsd(PicturaMainWindow& window, const QString& path, int width, int height)
{
    if (!window.newDocument(QStringLiteral("PlaceSource"), width, height, QStringLiteral("rgb"),
                            8, QStringLiteral("white"))) {
        return false;
    }
    PictureView* view = window.activeView();
    view->select_all();
    pictura::edit_fill(*view, QColor(30, 120, 220).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    view->deselect();
    const bool saved = window.saveActiveAs(path);
    window.closeDocument(window.activeDocumentIndex(), false);
    return saved;
}

QString writeSourcePng(const QString& path, int width, int height)
{
    QImage image(width, height, QImage::Format_RGB32);
    image.fill(QColor(30, 120, 220));
    return image.save(path, "PNG") ? path : QString();
}

// Commits the Place Free Transform once it opens, optionally scaling the box
// to 2× from a corner before pressing Enter.
struct PlaceCommitter {
    QTimer timer;
    PicturaMainWindow* window = nullptr;
    bool enlarge = false;

    void start(PicturaMainWindow& target, bool scaleUp)
    {
        window = &target;
        enlarge = scaleUp;
        QObject::connect(&timer, &QTimer::timeout, &timer, [this]() {
            PictureView* view = window ? window->activeView() : nullptr;
            if (!view || !view->transform_session_active()) {
                return;
            }
            if (enlarge) {
                const QPolygonF quad = transform_api::quad(*view).corners;
                if (quad.size() == 4) {
                    const QPointF corner = quad.at(2);
                    const double width = corner.x() - quad.at(0).x();
                    const double height = corner.y() - quad.at(0).y();
                    view->transform_press(corner.x(), corner.y(), 1.0, false, false);
                    view->transform_move(corner.x() + width, corner.y() + height, 1.0, false,
                                         false);
                    view->transform_release();
                }
            }
            timer.stop();
            if (ImageView* canvas = window->imageView()) {
                QKeyEvent key(QEvent::KeyPress, Qt::Key_Return, Qt::NoModifier);
                QCoreApplication::sendEvent(canvas, &key);
            }
        });
        timer.start(10);
    }
};

transform_api::LayerRect placedRect(PictureView& view, const QString& path)
{
    return transform_api::layerRect(view, path);
}

} // namespace

class PlaceFitTest : public QObject {
    Q_OBJECT

private slots:
    void fittedPsdStartsAtTheFittedRect();
    void sourceThatFitsIsNotUpscaled();
    void fittedPlaceEnlargesWithoutLosingTheSource();
    void fittedRasterKeepsItsNativeSource();
    void fitIsPartOfThePlaceState();
    void transparentRasterKeepsAlphaThroughResetAndRasterize();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void PlaceFitTest::fittedPsdStartsAtTheFittedRect()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString source = dir.filePath(QStringLiteral("source.psd"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(writeSourcePsd(*window, source, 400, 300));
    QVERIFY(window->newDocument(QStringLiteral("Target"), 100, 100, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    QVERIFY(view != nullptr);

    PlaceCommitter committer;
    committer.start(*window, false);
    pictura::placeDocuments(*window, QStringList{source});

    QVERIFY(!view->transform_session_active());
    const QString placed = layers_api::rowAt(*view, 0).path;
    const transform_api::LayerRect rect = placedRect(*view, placed);
    QVERIFY(rect.valid);
    QCOMPARE(rect.left, 0);
    QCOMPARE(rect.top, 12);
    QCOMPARE(rect.width(), 100);
    QCOMPARE(rect.height(), 75);
    QVERIFY(view->layer_can_replace_smart_object_contents(placed));
}

void PlaceFitTest::sourceThatFitsIsNotUpscaled()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString source = dir.filePath(QStringLiteral("source.psd"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(writeSourcePsd(*window, source, 40, 30));
    QVERIFY(window->newDocument(QStringLiteral("Target"), 100, 100, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    QVERIFY(view != nullptr);

    PlaceCommitter committer;
    committer.start(*window, false);
    pictura::placeDocuments(*window, QStringList{source});

    const QString placed = layers_api::rowAt(*view, 0).path;
    const transform_api::LayerRect rect = placedRect(*view, placed);
    QVERIFY(rect.valid);
    QCOMPARE(rect.width(), 40);
    QCOMPARE(rect.height(), 30);
}

void PlaceFitTest::fittedPlaceEnlargesWithoutLosingTheSource()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString source = dir.filePath(QStringLiteral("source.psd"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(writeSourcePsd(*window, source, 400, 300));
    QVERIFY(window->newDocument(QStringLiteral("Target"), 100, 100, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    QVERIFY(view != nullptr);
    const int before = view->history_count();

    PlaceCommitter committer;
    committer.start(*window, true);
    pictura::placeDocuments(*window, QStringList{source});

    const QString placed = layers_api::rowAt(*view, 0).path;
    const transform_api::LayerRect enlarged = placedRect(*view, placed);
    QVERIFY(enlarged.valid);
    QCOMPARE(enlarged.width(), 200);
    QCOMPARE(enlarged.height(), 150);
    QVERIFY(view->layer_can_replace_smart_object_contents(placed));
    QCOMPARE(view->history_count(), before + 2);

    QVERIFY(window->registry()->dispatch(
        QString::fromLatin1(command_ids::LayerSmartObjectResetTransform)));
    const transform_api::LayerRect native = placedRect(*view, placed);
    QVERIFY(native.valid);
    QCOMPARE(native.width(), 400);
    QCOMPARE(native.height(), 300);
}

void PlaceFitTest::fittedRasterKeepsItsNativeSource()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString source = writeSourcePng(dir.filePath(QStringLiteral("source.png")), 400, 300);
    QVERIFY(!source.isEmpty());

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Target"), 100, 100, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    QVERIFY(view != nullptr);

    const QString placed = view->place_image(source);
    QVERIFY(!placed.isEmpty());
    const transform_api::LayerRect fitted = placedRect(*view, placed);
    QVERIFY(fitted.valid);
    QCOMPARE(fitted.width(), 100);
    QCOMPARE(fitted.height(), 75);
    QVERIFY(view->layer_can_replace_smart_object_contents(placed));

    QVERIFY(window->registry()->dispatch(
        QString::fromLatin1(command_ids::LayerSmartObjectResetTransform)));
    const transform_api::LayerRect native = placedRect(*view, placed);
    QVERIFY(native.valid);
    QCOMPARE(native.width(), 400);
    QCOMPARE(native.height(), 300);
}

void PlaceFitTest::fitIsPartOfThePlaceState()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString source = dir.filePath(QStringLiteral("source.psd"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(writeSourcePsd(*window, source, 400, 300));
    QVERIFY(window->newDocument(QStringLiteral("Target"), 100, 100, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    PictureView* view = window->activeView();
    QVERIFY(view != nullptr);
    const int before = view->history_count();

    PlaceCommitter committer;
    committer.start(*window, false);
    pictura::placeDocuments(*window, QStringList{source});

    QCOMPARE(view->history_count(), before + 1);
}

void PlaceFitTest::transparentRasterKeepsAlphaThroughResetAndRasterize()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString source = dir.filePath(QStringLiteral("alpha.png"));
    QImage image(400, 300, QImage::Format_ARGB32);
    image.fill(Qt::transparent);
    for (int y = 10; y < 200; ++y) {
        for (int x = 10; x < 300; ++x) {
            image.setPixel(x, y, qRgba(30, 120, 220, 255));
        }
    }
    QVERIFY(image.save(source, "PNG"));

    auto window = makeShownWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Target"), 100, 100, QStringLiteral("rgb"), 8,
                                QStringLiteral("transparent")));
    PictureView* view = window->activeView();
    QVERIFY(view != nullptr);

    const QString placed = view->place_image(source);
    QVERIFY(!placed.isEmpty());
    QCOMPARE(qAlpha(view->sample_argb(0, 12)), 0);
    QCOMPARE(qAlpha(view->sample_argb(25, 37)), 255);

    const QString exported = dir.filePath(QStringLiteral("exported.psd"));
    QVERIFY(view->export_smart_object_contents(placed, exported));
    QFile exportedFile(exported);
    QVERIFY(exportedFile.open(QIODevice::ReadOnly));
    const QByteArray payload = exportedFile.readAll();
    QVERIFY(payload.size() > 26);
    QCOMPARE(quint16((quint8(payload.at(12)) << 8) | quint8(payload.at(13))), quint16(4));

    QVERIFY(window->registry()->dispatch(
        QString::fromLatin1(command_ids::LayerSmartObjectResetTransform)));
    QCOMPARE(qAlpha(view->sample_argb(0, 0)), 0);
    QCOMPARE(qAlpha(view->sample_argb(50, 50)), 255);

    QVERIFY(window->registry()->dispatch(
        QString::fromLatin1(command_ids::LayerRasterizeSmartObject)));
    QCOMPARE(qAlpha(view->sample_argb(0, 0)), 0);
    QCOMPARE(qAlpha(view->sample_argb(50, 50)), 255);
}

PICTURA_QT_TEST_MAIN(PlaceFitTest)
#include "tst_place_fit.moc"
