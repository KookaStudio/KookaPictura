// Linked layers (#959): the row chain badge, the action-strip Link button, and
// the Move tool moving a link set together.

#include <QtTest/QtTest>

#include "api/layers_api.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtGui/QIcon>
#include <QtWidgets/QApplication>
#include <QtWidgets/QToolButton>

#include "frame.h"
#include "icons.h"
#include "image_view.h"
#include "panels/layers_panel.h"
#include "smart_guides.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"

#include "qt_test_support.h"

namespace {

bool sameIcon(const QIcon& a, const QIcon& b)
{
    return !a.isNull() && !b.isNull() && a.pixmap(16, 16).toImage() == b.pixmap(16, 16).toImage();
}

// The layer at `path`'s rect as "left top right bottom".
QList<int> rectOf(pictura::PictureView& view, const QString& path)
{
    const ::rust::Vec<int32_t> rect = pictura::layer_rect_extents(view, path);
    return QList<int>({rect[0], rect[1], rect[2], rect[3]});
}

// A tight `size` square layer copied from the Background at `at`.
QString squareLayer(pictura::PictureView& view, const QPoint& at, int size)
{
    if (!view.select_rect(at.x(), at.y(), size, size, QStringLiteral("new"), 0.0)) {
        return {};
    }
    const QString path = view.layer_via_copy(QStringLiteral("0"));
    view.deselect();
    return path;
}

// Paint a small blob on the canvas-sized layer at `path`, so the layer's rect
// stays the canvas while its ink is a small square (the linked-pair report).
void paintBlob(pictura::PictureView& view, const QString& path, const QPointF& at)
{
    view.set_active_layer(path);
    if (!view.begin_paint(0xff202020u, 0xffffffffu, 12, 100, 100, 0, 100, 100, 25,
                          QStringLiteral("normal"), false, false)) {
        return;
    }
    for (int dx = -4; dx <= 4; dx += 4) {
        for (int dy = -4; dy <= 4; dy += 4) {
            view.paint_dab(at.x() + dx, at.y() + dy, 1.0);
        }
    }
    view.end_paint();
}

} // namespace

class LinkedLayersTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void stripButtonsCarryIcons();
    void stripLinkButtonToggles();
    void unlinkClearsTheSet();
    void linkedRowShowsChainBadge();
    void unlinkedRowShowsNoChainBadge();
    void dragLinkedLayersMovesSet();
    void moveMultiSelectionMovesEveryLayer();
    void linkedMovePreviewOriginMatchesTheCrop();
    void multiSelectionIsNotItsOwnReference();
    void canvasDragLandsAtTheDrop();
    void tightSquaresDragLandsAtTheDrop();
    void canvasSizedBlobsDragLandsAtTheDrop();
    void dragUnlinkedLayerAlone();
    void lockedMemberStaysPut();

private:
    bool newDoc(const QString& name, int layers);
    QToolButton* stripButton(const QString& objectName) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void LinkedLayersTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LinkedLayersTest::cleanup()
{
    if (window_ && window_->activeView()) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
}

bool LinkedLayersTest::newDoc(const QString& name, int layers)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_) {
        return false;
    }
    while (view_->layer_count() < layers) {
        if (view_->add_layer_in(QString()).isEmpty()) {
            return false;
        }
    }
    panel_->refresh();
    QCoreApplication::processEvents();
    return true;
}

QToolButton* LinkedLayersTest::stripButton(const QString& objectName) const
{
    return panel_ ? panel_->findChild<QToolButton*>(objectName) : nullptr;
}

void LinkedLayersTest::stripButtonsCarryIcons()
{
    QVERIFY(newDoc(QStringLiteral("StripIcons"), 2));
    const QStringList ids = {
        QStringLiteral("link"),           QStringLiteral("fx"),
        QStringLiteral("mask"),           QStringLiteral("fillAdjustment"),
        QStringLiteral("group"),          QStringLiteral("newLayer"),
        QStringLiteral("delete")};
    const QStringList objectNames = {
        QStringLiteral("layersStripLink"),   QStringLiteral("layersStripFx"),
        QStringLiteral("layersStripMask"),   QStringLiteral("layersStripFillAdjustment"),
        QStringLiteral("layersStripGroup"),  QStringLiteral("layersStripNewLayer"),
        QStringLiteral("layersStripDelete")};
    for (int i = 0; i < ids.size(); ++i) {
        QToolButton* button = stripButton(objectNames.at(i));
        QVERIFY2(button != nullptr, qPrintable(objectNames.at(i)));
        QVERIFY2(sameIcon(button->icon(), pictura::icon(QStringLiteral("layers.") + ids.at(i))),
                 qPrintable(ids.at(i)));
    }
}

void LinkedLayersTest::stripLinkButtonToggles()
{
    QVERIFY(newDoc(QStringLiteral("StripLink"), 2));
    panel_->selectPaths({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("1"));
    QCoreApplication::processEvents();
    QVERIFY(!view_->linked_together({QStringLiteral("0"), QStringLiteral("1")}));

    QToolButton* button = stripButton(QStringLiteral("layersStripLink"));
    QVERIFY(button != nullptr);
    QVERIFY(button->isEnabled());
    button->click();
    QCoreApplication::processEvents();

    QVERIFY(view_->linked_together({QStringLiteral("0"), QStringLiteral("1")}));
    QVERIFY(panel_->rowLinkedForTest(QStringLiteral("0")));
    QVERIFY(panel_->rowLinkedForTest(QStringLiteral("1")));
    QVERIFY2(sameIcon(button->icon(), pictura::icon(QStringLiteral("layers.unlink2"))),
             "a fully-linked selection shows the unlink glyph");

    button->click();
    QCoreApplication::processEvents();
    QVERIFY(!view_->linked_together({QStringLiteral("0"), QStringLiteral("1")}));
    QVERIFY(!panel_->rowLinkedForTest(QStringLiteral("0")));
    QVERIFY2(sameIcon(button->icon(), pictura::icon(QStringLiteral("layers.link"))),
             "an unlinked selection shows the link glyph");
}

void LinkedLayersTest::unlinkClearsTheSet()
{
    QVERIFY(newDoc(QStringLiteral("UnlinkSet"), 2));
    QCOMPARE(view_->link_layers({QStringLiteral("0"), QStringLiteral("1")}, true), 2);
    panel_->refresh();
    QCoreApplication::processEvents();
    QVERIFY(panel_->rowLinkedForTest(QStringLiteral("0")));

    QCOMPARE(view_->link_layers({QStringLiteral("0")}, false), 1);
    panel_->refresh();
    QCoreApplication::processEvents();

    // The partner is a singleton now: it is linked to nothing, so neither row
    // shows the badge and neither selects a set.
    QVERIFY(!panel_->rowLinkedForTest(QStringLiteral("0")));
    QVERIFY(!panel_->rowLinkedForTest(QStringLiteral("1")));
    QVERIFY(view_->select_linked(QStringLiteral("0")).isEmpty());
    QVERIFY(view_->select_linked(QStringLiteral("1")).isEmpty());
}

void LinkedLayersTest::linkedRowShowsChainBadge()
{
    QVERIFY(newDoc(QStringLiteral("LinkedBadge"), 2));
    QCOMPARE(view_->link_layers({QStringLiteral("0"), QStringLiteral("1")}, true), 2);
    panel_->refresh();
    QCoreApplication::processEvents();
    QVERIFY(!panel_->rowLinkBadgeRectForTest(QStringLiteral("0")).isEmpty());
    QVERIFY(!panel_->rowLinkBadgeRectForTest(QStringLiteral("1")).isEmpty());
}

void LinkedLayersTest::unlinkedRowShowsNoChainBadge()
{
    QVERIFY(newDoc(QStringLiteral("UnlinkedBadge"), 2));
    panel_->refresh();
    QCoreApplication::processEvents();
    QVERIFY(panel_->rowLinkBadgeRectForTest(QStringLiteral("0")).isEmpty());
    QVERIFY(panel_->rowLinkBadgeRectForTest(QStringLiteral("1")).isEmpty());
}

void LinkedLayersTest::dragLinkedLayersMovesSet()
{
    QVERIFY(newDoc(QStringLiteral("DragLinked"), 2));
    QCOMPARE(view_->link_layers({QStringLiteral("0"), QStringLiteral("1")}, true), 2);
    panel_->selectPaths({QStringLiteral("1")}, QStringLiteral("1"));
    QCoreApplication::processEvents();
    const QList<int> before0 = rectOf(*view_, QStringLiteral("0"));
    const QList<int> before1 = rectOf(*view_, QStringLiteral("1"));

    QVERIFY(view_->begin_move_preview());
    QCOMPARE(view_->move_preview_x(), 0);
    QCOMPARE(view_->move_preview_y(), 0);
    QVERIFY(!view_->move_preview_layer().isNull());
    const int history = view_->history_count();
    QVERIFY(view_->commit_move(5, 7));
    view_->end_move_preview();

    const QList<int> after0 = rectOf(*view_, QStringLiteral("0"));
    const QList<int> after1 = rectOf(*view_, QStringLiteral("1"));
    const auto shifted = [](const QList<int>& r, int dx, int dy) {
        return QList<int>({r.at(0) + dx, r.at(1) + dy, r.at(2) + dx, r.at(3) + dy});
    };
    QCOMPARE(after0, shifted(before0, 5, 7));
    QCOMPARE(after1, shifted(before1, 5, 7));
    QCOMPARE(view_->history_count(), history + 1);
    QVERIFY(view_->undo());
    QCOMPARE(rectOf(*view_, QStringLiteral("0")), before0);
    QCOMPARE(rectOf(*view_, QStringLiteral("1")), before1);
}

void LinkedLayersTest::moveMultiSelectionMovesEveryLayer()
{
    QVERIFY(newDoc(QStringLiteral("MultiMove"), 3));
    QCOMPARE(view_->link_layers({QStringLiteral("0"), QStringLiteral("1")}, true), 2);
    // Select one of the linked pair and a third unlinked layer.
    panel_->selectPaths({QStringLiteral("0"), QStringLiteral("2")}, QStringLiteral("0"));
    QCoreApplication::processEvents();
    const QList<int> before0 = rectOf(*view_, QStringLiteral("0"));
    const QList<int> before1 = rectOf(*view_, QStringLiteral("1"));
    const QList<int> before2 = rectOf(*view_, QStringLiteral("2"));

    QVERIFY(view_->begin_move_preview());
    const int history = view_->history_count();
    QVERIFY(view_->commit_move(6, -4));
    view_->end_move_preview();

    const auto shifted = [](const QList<int>& r, int dx, int dy) {
        return QList<int>({r.at(0) + dx, r.at(1) + dy, r.at(2) + dx, r.at(3) + dy});
    };
    QCOMPARE(rectOf(*view_, QStringLiteral("0")), shifted(before0, 6, -4));
    QCOMPARE(rectOf(*view_, QStringLiteral("1")), shifted(before1, 6, -4));
    QCOMPARE(rectOf(*view_, QStringLiteral("2")), shifted(before2, 6, -4));
    QCOMPARE(view_->history_count(), history + 1);
}

void LinkedLayersTest::linkedMovePreviewOriginMatchesTheCrop()
{
    QVERIFY(newDoc(QStringLiteral("PreviewOrigin"), 2));
    const QString a = view_->add_solid_fill(0xff000000u);
    QVERIFY(!a.isEmpty());
    QVERIFY(view_->rasterize_fill_content(a));
    const QString b = view_->add_solid_fill(0xffff0000u);
    QVERIFY(!b.isEmpty());
    QVERIFY(view_->rasterize_fill_content(b));
    QCOMPARE(view_->link_layers({a, b}, true), 2);
    panel_->selectPaths({a, b}, a);
    QCoreApplication::processEvents();

    // Drag the set up-left so the union origin is negative; the preview image is
    // cropped to the canvas, so the next preview's origin must be clamped too.
    QVERIFY(view_->begin_move_preview());
    QVERIFY(view_->commit_move(-8, -8));
    view_->end_move_preview();

    // The move changed the group's union rect, so the cached image (composited
    // at the old positions) must not be reused, and the fresh preview origin is
    // the canvas-clamped crop origin.
    QVERIFY(view_->begin_move_preview());
    QVERIFY2(!view_->move_preview_cache_hit(), "a moved group recomputes its preview");
    QVERIFY(!view_->move_preview_layer().isNull());
    QCOMPARE(view_->move_preview_x(), 0);
    QCOMPARE(view_->move_preview_y(), 0);
    view_->end_move_preview();

    // Without a move the union is unchanged, so the preview is reused.
    QVERIFY(view_->begin_move_preview());
    QVERIFY2(view_->move_preview_cache_hit(), "an unmoved group reuses its preview");
    view_->end_move_preview();

    // The commit still lands the whole set at the drop offset.
    const QList<int> beforeA = rectOf(*view_, a);
    const QList<int> beforeB = rectOf(*view_, b);
    QVERIFY(view_->commit_move(6, 4));
    const auto shifted = [](const QList<int>& r, int dx, int dy) {
        return QList<int>({r.at(0) + dx, r.at(1) + dy, r.at(2) + dx, r.at(3) + dy});
    };
    QCOMPARE(rectOf(*view_, a), shifted(beforeA, 6, 4));
    QCOMPARE(rectOf(*view_, b), shifted(beforeB, 6, 4));
}

void LinkedLayersTest::multiSelectionIsNotItsOwnReference()
{
    QVERIFY(newDoc(QStringLiteral("NoSelfSnap"), 2));
    const QString a = view_->add_solid_fill(0xff000000u);
    QVERIFY(!a.isEmpty());
    QVERIFY(view_->rasterize_fill_content(a));
    const QString b = view_->add_solid_fill(0xffff0000u);
    QVERIFY(!b.isEmpty());
    QVERIFY(view_->rasterize_fill_content(b));
    QCOMPARE(view_->link_layers({a, b}, true), 2);
    panel_->selectPaths({a, b}, a);
    QCoreApplication::processEvents();

    const QStringList moving = pictura::move_group_paths(*view_);
    QCOMPARE(moving.size(), 2);
    QVERIFY(moving.contains(a));
    QVERIFY(moving.contains(b));
    // The two moving layers are the only inked layers, so the guide references
    // must be empty: a selection never snaps to itself.
    QVERIFY2(pictura::smartGuideReferences(*view_, moving).isEmpty(),
             "the moving layers are not their own snap references");
}

void LinkedLayersTest::canvasDragLandsAtTheDrop()
{
    QVERIFY(newDoc(QStringLiteral("CanvasDrag"), 2));
    const QString a = view_->add_solid_fill(0xff000000u);
    QVERIFY(!a.isEmpty());
    QVERIFY(view_->rasterize_fill_content(a));
    const QString b = view_->add_solid_fill(0xffff0000u);
    QVERIFY(!b.isEmpty());
    QVERIFY(view_->rasterize_fill_content(b));
    // Offset the two layers so their rects differ, before linking.
    panel_->selectPaths({a}, a);
    QCoreApplication::processEvents();
    QVERIFY(view_->begin_move_preview());
    QVERIFY(view_->commit_move(4, 4));
    view_->end_move_preview();
    panel_->selectPaths({b}, b);
    QCoreApplication::processEvents();
    QVERIFY(view_->begin_move_preview());
    QVERIFY(view_->commit_move(30, 25));
    view_->end_move_preview();
    QCOMPARE(view_->link_layers({a, b}, true), 2);
    panel_->selectPaths({a, b}, a);
    QCoreApplication::processEvents();
    const QList<int> beforeA = rectOf(*view_, a);
    const QList<int> beforeB = rectOf(*view_, b);

    window_->show();
    window_->setActiveTool(pictura::ToolId::Move);
    auto* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    // Snapping off: this check is the canvas delta and commit, not guides.
    canvas->setSmartGuidesEnabled(false);
    // Press inside the active layer's controls box, drag, release.
    const QPointF grab(12, 12);
    const QPointF drop(32, 22);
    canvas->mousePressed(canvas->imageToWidget(grab), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(canvas->movePreviewActive());
    canvas->mouseMoved(canvas->imageToWidget(drop));
    canvas->mouseReleased(canvas->imageToWidget(drop));

    const auto shifted = [](const QList<int>& r, int dx, int dy) {
        return QList<int>({r.at(0) + dx, r.at(1) + dy, r.at(2) + dx, r.at(3) + dy});
    };
    QCOMPARE(rectOf(*view_, a), shifted(beforeA, 20, 10));
    QCOMPARE(rectOf(*view_, b), shifted(beforeB, 20, 10));
}

void LinkedLayersTest::tightSquaresDragLandsAtTheDrop()
{
    QVERIFY(window_->newDocument(QStringLiteral("TightSquares"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    // Two tight squares, as the linked-pair report shows.
    const QString red = squareLayer(*view_, QPoint(8, 8), 20);
    const QString blue = squareLayer(*view_, QPoint(30, 30), 20);
    QVERIFY(!red.isEmpty() && !blue.isEmpty());
    QCOMPARE(view_->link_layers({red, blue}, true), 2);
    panel_->selectPaths({red, blue}, red);
    QCoreApplication::processEvents();
    const QList<int> beforeRed = rectOf(*view_, red);
    const QList<int> beforeBlue = rectOf(*view_, blue);

    window_->show();
    window_->setActiveTool(pictura::ToolId::Move);
    auto* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    canvas->setSmartGuidesEnabled(false);
    // Press inside the red square, drag, release.
    const QPointF grab(18, 18);
    const QPointF drop(38, 28);
    canvas->mousePressed(canvas->imageToWidget(grab), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(canvas->movePreviewActive());
    canvas->mouseMoved(canvas->imageToWidget(drop));
    canvas->mouseReleased(canvas->imageToWidget(drop));

    const auto shifted = [](const QList<int>& r, int dx, int dy) {
        return QList<int>({r.at(0) + dx, r.at(1) + dy, r.at(2) + dx, r.at(3) + dy});
    };
    QCOMPARE(rectOf(*view_, red), shifted(beforeRed, 20, 10));
    QCOMPARE(rectOf(*view_, blue), shifted(beforeBlue, 20, 10));
}

void LinkedLayersTest::canvasSizedBlobsDragLandsAtTheDrop()
{
    QVERIFY(window_->newDocument(QStringLiteral("CanvasBlobs"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    const QString a = view_->add_layer_in(QString());
    const QString b = view_->add_layer_in(QString());
    QVERIFY(!a.isEmpty() && !b.isEmpty());
    paintBlob(*view_, a, QPointF(14, 14));
    paintBlob(*view_, b, QPointF(42, 42));
    QCOMPARE(view_->link_layers({a, b}, true), 2);
    panel_->selectPaths({a, b}, a);
    QCoreApplication::processEvents();
    const QList<int> beforeA = rectOf(*view_, a);
    const QList<int> beforeB = rectOf(*view_, b);

    window_->show();
    window_->setActiveTool(pictura::ToolId::Move);
    auto* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    canvas->setSmartGuidesEnabled(false);
    const QPointF grab(14, 14);
    const QPointF drop(34, 24);
    canvas->mousePressed(canvas->imageToWidget(grab), Qt::LeftButton, int(Qt::NoModifier));
    QVERIFY(canvas->movePreviewActive());
    canvas->mouseMoved(canvas->imageToWidget(drop));
    canvas->mouseReleased(canvas->imageToWidget(drop));

    const auto shifted = [](const QList<int>& r, int dx, int dy) {
        return QList<int>({r.at(0) + dx, r.at(1) + dy, r.at(2) + dx, r.at(3) + dy});
    };
    QCOMPARE(rectOf(*view_, a), shifted(beforeA, 20, 10));
    QCOMPARE(rectOf(*view_, b), shifted(beforeB, 20, 10));
}

void LinkedLayersTest::dragUnlinkedLayerAlone()
{
    QVERIFY(newDoc(QStringLiteral("DragAlone"), 2));
    panel_->selectPaths({QStringLiteral("1")}, QStringLiteral("1"));
    QCoreApplication::processEvents();
    const QList<int> before0 = rectOf(*view_, QStringLiteral("0"));
    const QList<int> before1 = rectOf(*view_, QStringLiteral("1"));

    QVERIFY(view_->begin_move_preview());
    QVERIFY(view_->commit_move(4, 4));
    view_->end_move_preview();

    QCOMPARE(rectOf(*view_, QStringLiteral("0")), before0);
    QCOMPARE(rectOf(*view_, QStringLiteral("1")),
             QList<int>({before1.at(0) + 4, before1.at(1) + 4, before1.at(2) + 4,
                         before1.at(3) + 4}));
}

void LinkedLayersTest::lockedMemberStaysPut()
{
    QVERIFY(newDoc(QStringLiteral("LockedMember"), 2));
    QVERIFY(view_->set_layers_lock({QStringLiteral("1")}, QStringLiteral("position"), true));
    QCOMPARE(view_->link_layers({QStringLiteral("0"), QStringLiteral("1")}, true), 2);
    panel_->selectPaths({QStringLiteral("0")}, QStringLiteral("0"));
    QCoreApplication::processEvents();
    const QList<int> before0 = rectOf(*view_, QStringLiteral("0"));
    const QList<int> before1 = rectOf(*view_, QStringLiteral("1"));

    QVERIFY(view_->begin_move_preview());
    QVERIFY(view_->commit_move(3, 3));
    view_->end_move_preview();

    QCOMPARE(rectOf(*view_, QStringLiteral("0")),
             QList<int>({before0.at(0) + 3, before0.at(1) + 3, before0.at(2) + 3,
                         before0.at(3) + 3}));
    QCOMPARE(rectOf(*view_, QStringLiteral("1")), before1);
}

QTEST_MAIN(LinkedLayersTest)
#include "tst_linked_layers.moc"
