#include <QtTest/QtTest>
#include "api/layers_api.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QMimeData>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMenu>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>

#include "file_drop_router.h"
#include "frame.h"
#include "image_view.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "theme.h"
#include "commands.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include "qt_test_support.h"

namespace {

QModelIndex indexForPath(QAbstractItemModel* model, const QString& path,
                         const QModelIndex& parent = QModelIndex())
{
    if (!model) {
        return {};
    }
    for (int row = 0; row < model->rowCount(parent); ++row) {
        const QModelIndex index = model->index(row, 0, parent);
        if (index.data(pictura::PathRole).toString() == path) {
            return index;
        }
        const QModelIndex child = indexForPath(model, path, index);
        if (child.isValid()) {
            return child;
        }
    }
    return {};
}

bool dropOnViewport(QWidget* viewport, const void* document, const QString& source,
                    const QPoint& pos)
{
    QMimeData mime;
    mime.setData(pictura::kLayerMimeType, source.toUtf8());
    mime.setData(pictura::kLayerSourceMimeType,
                 QByteArray::number(reinterpret_cast<quintptr>(document)));
    QDragEnterEvent enter(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(viewport, &enter);
    QDragMoveEvent move(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(viewport, &move);
    QDropEvent drop(QPointF(pos), Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(viewport, &drop);
    return drop.isAccepted();
}

// Send a layer drag from `document` over `target` at `pos`; with `drop`, release
// it there. Returns whether the last event was accepted.
bool dragLayerOver(QWidget* target, const void* document, const QString& path,
                   const QPoint& pos, bool drop)
{
    QMimeData mime;
    mime.setData(pictura::kLayerMimeType, path.toUtf8());
    mime.setData(pictura::kLayerSourceMimeType,
                 QByteArray::number(reinterpret_cast<quintptr>(document)));
    const auto actions = Qt::MoveAction | Qt::CopyAction;
    QDragEnterEvent enter(pos, actions, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &enter);
    QDragMoveEvent move(pos, actions, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &move);
    if (!drop) {
        return move.isAccepted();
    }
    QDropEvent release(QPointF(pos), actions, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &release);
    return release.isAccepted();
}

int countNamed(pictura::PictureView* view, const QString& name)
{
    int count = 0;
    for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
        count += pictura::layers_api::rowAt(*view, i).name == name ? 1 : 0;
    }
    return count;
}

} // namespace

class LayersPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void nestingLockRefusalAndReorder();
    void panelChrome();
    void rowWidgets();
    void layerRowSurface();
    void gutterClickTogglesWithoutSelecting();
    void dragReorder();
    void dropAnywhereOnLayerRowReorders();
    void dropOnGroupRowEdgesPlacesSibling();
    void dragLayerOntoAnotherDocument();
    void moveToolFindsAnotherDocumentTab();
    void dropOnDelete();
    void clippingMasks();
    void backgroundOnlyStaysActive();
    void activeLayerNotFirstRow();
    void maskRowIndicators();
    void maskEditTargetActivation();
    void maskRowActionsTargetTheClickedRow();
    void vectorMaskRowIndicators();
    void fillAdjustmentMenu();
    void layerSurfaceCompleteness();
    void panelOptionsFlags();
    void renameTabNavigation();
    void shapeRowActions();
    void layerStylePanelIntegration();
    void blendIfBadge();
    void isolationMode();

private:
    bool setupNest();
    int rowOf(const QString& path);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
    QString group_;
    QString a_;
    QString b_;
};

void LayersPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayersPanelTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
    group_.clear();
    a_.clear();
    b_.clear();
}

bool LayersPanelTest::setupNest()
{
    const bool created = window_->newDocument(QStringLiteral("NestLock"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!created || !view_ || !panel_) {
        return false;
    }
    group_ = view_->add_group_in(QString());
    a_ = view_->add_layer_in(group_);
    b_ = view_->add_layer_in(group_);
    view_->set_layer_name_path(a_, QStringLiteral("A"));
    view_->set_layer_name_path(b_, QStringLiteral("B"));
    return true;
}

int LayersPanelTest::rowOf(const QString& path)
{
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        if (pictura::layers_api::rowAt(*view_, i).path == path) {
            return i;
        }
    }
    return -1;
}

void LayersPanelTest::nestingLockRefusalAndReorder()
{
    QVERIFY2(setupNest(), "nesting fixture");
    const int doc = window_->activeDocumentIndex();
    const int lockBase = view_->history_count();
    const int locked =
        view_->set_layers_lock(QStringList{a_}, QStringLiteral("nesting"), true);
    QVERIFY2(locked == 1 && (pictura::layers_api::rowAt(*view_, rowOf(a_)).lockBits & 0x08) != 0
                 && view_->history_count() == lockBase + 1,
             "nesting lock bit");
    const int groupBase = view_->history_count();
    const QString wrapped = view_->group_layers(QStringList{a_});
    QVERIFY2(wrapped.isEmpty() && view_->history_count() == groupBase
                 && pictura::layers_api::rowAt(*view_, rowOf(a_)).path == a_,
             "group layers refused");
    const int moveBase = view_->history_count();
    const bool moved = view_->move_layer_path(a_, 1);
    QVERIFY2(moved && pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("1/1"))).name == QStringLiteral("A")
                 && view_->history_count() == moveBase + 1,
             "move in container");
    const int all = view_->set_layers_lock(QStringList{QStringLiteral("1/1")},
                                           QStringLiteral("all"), true);
    QVERIFY2(all == 1
                 && (pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("1/1"))).lockBits & 0x0F) == 0x0F,
             "all locks");
    window_->closeDocument(doc, false);
}

void LayersPanelTest::panelChrome()
{
    QVERIFY2(setupNest(), "chrome fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();
    QVERIFY2(panel_->headerOrderOkForTest(), "header order");
    QVERIFY2(panel_->opacityLabelPresentForTest() && panel_->fillLabelPresentForTest(),
             "header labels");
    QVERIFY2(!panel_->hasPanelMenuButtonForTest(), "no panel menu button");
    QVERIFY2(panel_->filterToggleOnForTest() && panel_->filterToggleHasIconForTest(),
             "filter lightswitch");
    panel_->expandForTest(group_);
    const int eyeGroup = panel_->eyeLeftForTest(group_);
    const int eyeChild = panel_->eyeLeftForTest(QStringLiteral("1/0"));
    QVERIFY2(eyeGroup >= 0 && eyeGroup == eyeChild, "nested eye left-anchored");
    QVERIFY2(panel_->chevronClickExpandsForTest(group_), "chevron expands group");
    window_->closeDocument(doc, false);
}

void LayersPanelTest::rowWidgets()
{
    QVERIFY2(setupNest(), "rows fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();
    QVERIFY2(panel_->opacityLabelTextForTest() == QStringLiteral("Opacity"), "opacity label");
    QVERIFY2(panel_->opacitySuffixPresentForTest(), "percent suffix");
    QVERIFY2(panel_->lockIconsPresentForTest(), "semantic lock icons");
    QVERIFY2(panel_->treeDragEnabledForTest(), "tree drag enabled");
    window_->closeDocument(doc, false);
}

void LayersPanelTest::layerRowSurface()
{
    QVERIFY2(setupNest(), "row surface fixture");
    const int doc = window_->activeDocumentIndex();
    // The row image seams read the painted row, so the tree must be laid out.
    window_->show();
    panel_->setView(view_);
    panel_->refresh();
    panel_->expandForTest(group_);
    QTest::qWait(50);

    // The paint helper seeds the palette with a fixed neutral grey, so the
    // exact surfaces are deterministic without depending on the live palette.
    const QColor canvasBase(128, 128, 128);
    const QColor rowSurface = pictura::Theme::shade(canvasBase, 1);

    panel_->selectPaths({}, QString());
    const QImage unselected = panel_->rowImageForTest(a_);
    const QColor bg = unselected.pixelColor(unselected.width() - 4, 1);
    QCOMPARE(bg.rgba(), rowSurface.rgba());
    QVERIFY2(bg.rgba() != canvasBase.rgba(), "row surface steps lighter than the list");

    // The content column begins kContentPad past the gutter; the name shares
    // that left, and the 4 px name gap (kNameGap) follows the thumbnail.
    const QRect thumb = panel_->rowThumbRectForTest(a_);
    const QRect name = panel_->rowNameRectForTest(a_);
    QCOMPARE(thumb.left(), pictura::LayerRowDelegate::kEyeColumn
                               + pictura::LayerRowDelegate::kContentPad
                               + pictura::LayerRowDelegate::kIndent);
    QCOMPARE(name.left(), thumb.left() + thumb.width()
                              + pictura::LayerRowDelegate::kNameGap);

    // The paint helper seeds Highlight with blue; the delegate must paint the
    // neutral grey list-selection fill instead, and keep the eye gutter on the
    // row surface so a selected row's gutter matches an unselected one.
    panel_->selectPaths({a_}, a_);
    const QImage selected = panel_->rowImageForTest(a_);
    const QColor expectedSelection = pictura::Theme::shade(canvasBase, 3);
    const QColor selPixel = selected.pixelColor(selected.width() - 4, 1);
    QCOMPARE(selPixel.rgba(), expectedSelection.rgba());
    QVERIFY2(selPixel.rgba() != bg.rgba(), "selection changes the row surface");
    QVERIFY2(selPixel.red() == selPixel.green() && selPixel.green() == selPixel.blue(),
             "selection uses grey, not the palette blue");
    const QRect eye = panel_->rowEyeRectForTest(a_);
    QCOMPARE(selected.pixel(eye.left() + 2, eye.top() + 1),
             unselected.pixel(eye.left() + 2, eye.top() + 1));

    window_->closeDocument(doc, false);
}

void LayersPanelTest::gutterClickTogglesWithoutSelecting()
{
    QVERIFY2(setupNest(), "gutter fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();
    panel_->expandForTest(group_);
    window_->show();
    QTest::qWait(50);

    QTreeView* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    QWidget* viewport = tree->viewport();
    QVERIFY(viewport != nullptr);
    const auto rowRect = [&]() -> QRect { return panel_->rowViewportRectForTest(a_); };

    const int kEyeColumn = pictura::LayerRowDelegate::kEyeColumn;
    QRect vr = rowRect();
    QVERIFY2(vr.isValid() && vr.width() > kEyeColumn, "layer A row is laid out");

    panel_->selectPaths({b_}, b_);
    const bool before = pictura::layers_api::rowAt(*view_, rowOf(a_)).visible;
    QCOMPARE(panel_->selectedPaths(), QStringList{b_});

    // Re-read the row geometry: selecting b_ may scroll the view, which moves
    // layer A's rect.
    vr = rowRect();
    QVERIFY2(vr.isValid() && vr.width() > kEyeColumn, "layer A row is laid out after select");

    // The whole left gutter toggles visibility and never selects: the first
    // column pixel and the last gutter pixel before the padded content both
    // work.
    QTest::mouseClick(viewport, Qt::LeftButton, Qt::NoModifier,
                      QPoint(vr.left(), vr.center().y()));
    QCoreApplication::processEvents();
    QVERIFY2(pictura::layers_api::rowAt(*view_, rowOf(a_)).visible != before, "gutter left edge toggles visibility");
    QCOMPARE(panel_->selectedPaths(), QStringList{b_});

    // Restore, then the last gutter pixel toggles identically.
    view_->set_layers_visible({a_}, before);
    QCoreApplication::processEvents();
    vr = rowRect();
    QVERIFY2(vr.isValid(), "layer A row still laid out");
    QTest::mouseClick(viewport, Qt::LeftButton, Qt::NoModifier,
                      QPoint(vr.left() + kEyeColumn - 1, vr.center().y()));
    QCoreApplication::processEvents();
    QVERIFY2(pictura::layers_api::rowAt(*view_, rowOf(a_)).visible != before, "gutter last column toggles visibility");
    QCOMPARE(panel_->selectedPaths(), QStringList{b_});

    // The padded content column just past the gutter selects instead of toggling.
    vr = rowRect();
    QVERIFY2(vr.isValid(), "layer A row still laid out");
    const bool visibleBeforeContent = pictura::layers_api::rowAt(*view_, rowOf(a_)).visible;
    QTest::mouseClick(viewport, Qt::LeftButton, Qt::NoModifier,
                      QPoint(vr.left() + kEyeColumn, vr.center().y()));
    QCoreApplication::processEvents();
    QVERIFY2(pictura::layers_api::rowAt(*view_, rowOf(a_)).visible == visibleBeforeContent,
             "content click leaves visibility alone");
    QVERIFY2(panel_->selectedPaths().contains(a_), "content click selects the row");

    // Right-clicking the gutter opens the eye menu but must not disturb the
    // selection. The old glyph-only guard fell through to the row menu and
    // re-selected the row; the widened gutter guard does not. Emit the
    // custom-context-menu signal the tree emits on a real right-click and close
    // the menu at once so its nested event loop returns headlessly.
    panel_->selectPaths({b_}, b_);
    QTimer::singleShot(0, qApp, []() {
        if (QWidget* popup = QApplication::activePopupWidget()) {
            popup->close();
        }
    });
    vr = rowRect();
    QVERIFY2(vr.isValid(), "layer A row still laid out");
    QMetaObject::invokeMethod(tree, "customContextMenuRequested", Qt::DirectConnection,
                              Q_ARG(QPoint, QPoint(vr.left() + 1, vr.center().y())));
    QCoreApplication::processEvents();
    QCOMPARE(panel_->selectedPaths(), QStringList{b_});

    window_->closeDocument(doc, false);
}

void LayersPanelTest::dragReorder()
{
    QVERIFY2(setupNest(), "drag fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();
    const int base = view_->history_count();
    const bool dragged =
        panel_->moveForTest(QStringLiteral("1/0"), QStringLiteral("1/1"), 0);
    QVERIFY2(dragged && view_->history_count() == base + 1, "drag reorder is one undo step");
    window_->closeDocument(doc, false);
}

// A real drag lets go somewhere inside a row, not on its 2 px edge. A plain
// layer row is a sibling target across its whole height: the upper half drops
// above it, the lower half below it.
void LayersPanelTest::dropAnywhereOnLayerRowReorders()
{
    const bool created = window_->newDocument(QStringLiteral("DropRow"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "drop row fixture");
    const int doc = window_->activeDocumentIndex();
    const QString a = view_->add_layer_in(QString());
    const QString b = view_->add_layer_in(QString());
    view_->set_layer_name_path(a, QStringLiteral("A"));
    view_->set_layer_name_path(b, QStringLiteral("B"));
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    QTreeView* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);

    QRect row = panel_->rowViewportRectForTest(b);
    QVERIFY2(row.height() > 8, "layer B row is laid out");
    const int base = view_->history_count();
    QVERIFY2(dropOnViewport(tree->viewport(), view_, a,
                            QPoint(row.center().x(), row.top() + row.height() / 4)),
             "upper-half drop accepted");
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("2"))).name, QStringLiteral("A"));
    QCOMPARE(view_->history_count(), base + 1);

    panel_->refresh();
    QCoreApplication::processEvents();
    row = panel_->rowViewportRectForTest(QStringLiteral("1"));
    QVERIFY2(dropOnViewport(tree->viewport(), view_, QStringLiteral("2"),
                            QPoint(row.center().x(), row.bottom() - row.height() / 4)),
             "lower-half drop accepted");
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("1"))).name, QStringLiteral("A"));
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("2"))).name, QStringLiteral("B"));
    window_->closeDocument(doc, false);
}

// A group row keeps its centre for drop-into; its upper and lower quarters are
// sibling targets. Drops well inside those quarters (past the old 2 px edge)
// must place the layer above or below the group, never inside it.
void LayersPanelTest::dropOnGroupRowEdgesPlacesSibling()
{
    const bool created = window_->newDocument(QStringLiteral("DropGroup"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "drop group fixture");
    const int doc = window_->activeDocumentIndex();
    const QString layer = view_->add_layer_in(QString());
    const QString group = view_->add_group_in(QString());
    view_->set_layer_name_path(layer, QStringLiteral("L"));
    view_->set_layer_name_path(group, QStringLiteral("G"));
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    QTreeView* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);

    QRect row = panel_->rowViewportRectForTest(QStringLiteral("2"));
    QVERIFY2(row.height() >= 16, "group row is laid out");
    const int base = view_->history_count();
    QVERIFY2(dropOnViewport(tree->viewport(), view_, QStringLiteral("1"),
                            QPoint(row.center().x(), row.top() + row.height() / 8)),
             "upper-quarter drop accepted");
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("2"))).name, QStringLiteral("L"));
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("1"))).name, QStringLiteral("G"));
    QCOMPARE(view_->history_count(), base + 1);

    panel_->refresh();
    QCoreApplication::processEvents();
    row = panel_->rowViewportRectForTest(QStringLiteral("1"));
    QVERIFY2(dropOnViewport(tree->viewport(), view_, QStringLiteral("2"),
                            QPoint(row.center().x(), row.bottom() - row.height() / 8)),
             "lower-quarter drop accepted");
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("1"))).name, QStringLiteral("L"));
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(QStringLiteral("2"))).name, QStringLiteral("G"));
    QCOMPARE(view_->history_count(), base + 2);
    window_->closeDocument(doc, false);
}

// Issue #231: a layer dragged onto another document's tab brings that document
// forward and copies the layer in; its canvas takes the drop too. The source is
// untouched, and the other document's panel never treats the drag as its own.
void LayersPanelTest::dragLayerOntoAnotherDocument()
{
    QVERIFY(window_->newDocument(QStringLiteral("Src"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* src = window_->activeView();
    const QString sky = src->add_layer_in(QString());
    src->set_layer_name_path(sky, QStringLiteral("Sky"));
    src->set_layers_opacity(QStringList{sky}, 40);
    QVERIFY(window_->newDocument(QStringLiteral("Dst"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* dst = window_->activeView();
    QVERIFY(window_->newDocument(QStringLiteral("Gray"), 16, 16, QStringLiteral("grayscale"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* gray = window_->activeView();
    window_->setActiveDocumentIndex(0);
    window_->show();
    QTest::qWait(50);
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar != nullptr && bar->count() == 3);
    const int srcRows = pictura::layers_api::rowCount(*src);
    const int base = dst->history_count();

    QVERIFY2(!dragLayerOver(bar, src, sky, bar->tabRect(0).center(), true),
             "a drop on its own document's tab copies nothing");
    QCOMPARE(pictura::layers_api::rowCount(*src), srcRows);

    QVERIFY2(dragLayerOver(bar, src, sky, bar->tabRect(1).center(), false),
             "hovering another tab accepts the drag");
    QCOMPARE(window_->activeView(), dst);
    QVERIFY2(dragLayerOver(bar, src, sky, bar->tabRect(1).center(), true), "tab drop accepted");
    QCOMPARE(countNamed(dst, QStringLiteral("Sky")), 1);
    QCOMPARE(dst->history_count(), base + 1);
    view_ = dst;
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel_ != nullptr);
    const QString copied = panel_->currentPath();
    QCOMPARE(pictura::layers_api::rowAt(*dst, rowOf(copied)).name, QStringLiteral("Sky"));
    view_ = src;
    const int srcOpacity = pictura::layers_api::rowAt(*src, rowOf(sky)).opacity;
    view_ = dst;
    QVERIFY(srcOpacity < 100);
    QCOMPARE(pictura::layers_api::rowAt(*dst, rowOf(copied)).opacity, srcOpacity);
    QCOMPARE(pictura::layers_api::rowCount(*src), srcRows);

    QWidget* canvas = window_->canvasAt(1);
    QVERIFY2(dragLayerOver(canvas, src, sky, canvas->rect().center(), true),
             "canvas drop accepted");
    QCOMPARE(countNamed(dst, QStringLiteral("Sky")), 2);

    const int dstRows = pictura::layers_api::rowCount(*dst);
    QTreeView* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    QVERIFY2(!dropOnViewport(tree->viewport(), src, QStringLiteral("1"),
                             tree->viewport()->rect().center()),
             "another document's drag is not a drop on this panel");
    QCOMPARE(pictura::layers_api::rowCount(*dst), dstRows);

    // Back over the source tab after another document came forward: the drop
    // is the own-document no-op, not a copy into the last-hovered document.
    QVERIFY(dragLayerOver(bar, src, sky, bar->tabRect(1).center(), false));
    QVERIFY2(!dragLayerOver(bar, src, sky, bar->tabRect(0).center(), true),
             "returning to the source tab copies nothing");
    QCOMPARE(window_->activeView(), src);
    QCOMPARE(countNamed(dst, QStringLiteral("Sky")), 2);
    QCOMPARE(pictura::layers_api::rowCount(*src), srcRows);

    // Empty tab-bar space is no target, whatever document is active.
    QVERIFY(dragLayerOver(bar, src, sky, bar->tabRect(1).center(), false));
    const QPoint pastTabs(bar->tabRect(bar->count() - 1).right() + 4, bar->height() / 2);
    if (bar->tabAt(pastTabs) < 0 && bar->rect().contains(pastTabs)) {
        QVERIFY2(!dragLayerOver(bar, src, sky, pastTabs, true),
                 "a drop past the last tab copies nothing");
        QCOMPARE(countNamed(dst, QStringLiteral("Sky")), 2);
    }

    QVERIFY2(!dragLayerOver(bar, src, sky, bar->tabRect(2).center(), true),
             "a Grayscale document refuses an RGB layer");
    QCOMPARE(window_->activeView(), gray);
    QCOMPARE(countNamed(gray, QStringLiteral("Sky")), 0);
}

// Issue #231: a Move-tool drag hands off to the layer drag once the pointer is
// over another document's tab; the current tab and the canvas keep the move.
void LayersPanelTest::moveToolFindsAnotherDocumentTab()
{
    QVERIFY(window_->newDocument(QStringLiteral("MoveSrc"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const int src = window_->activeDocumentIndex();
    QVERIFY(window_->newDocument(QStringLiteral("MoveDst"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const int dst = window_->activeDocumentIndex();
    window_->setActiveDocumentIndex(src);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar != nullptr);

    using pictura::FileDropRouter;
    QCOMPARE(FileDropRouter::otherDocumentTabAt(bar->mapToGlobal(bar->tabRect(dst).center())),
             dst);
    QCOMPARE(FileDropRouter::otherDocumentTabAt(bar->mapToGlobal(bar->tabRect(src).center())),
             -1);
    QWidget* canvas = window_->canvasAt(src);
    QCOMPARE(FileDropRouter::otherDocumentTabAt(canvas->mapToGlobal(canvas->rect().center())), -1);
    window_->closeDocument(dst, false);
    window_->closeDocument(src, false);
}

void LayersPanelTest::dropOnDelete()
{
    const bool created = window_->newDocument(QStringLiteral("DropCtl"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view && panel, "drop fixture");
    const int doc = window_->activeDocumentIndex();
    // A second layer to drop: the document's last layer cannot be deleted (#670).
    const QString target = view->add_layer_in(QString());
    panel->setView(view);
    panel->refresh();
    const int rows = pictura::layers_api::rowCount(*view);
    const int base = view->history_count();
    const bool dropped = panel->dropOnStripButtonForTest(
        QStringLiteral("layersStripDelete"), {target});
    QVERIFY2(dropped && pictura::layers_api::rowCount(*view) == rows - 1
                 && view->history_count() == base + 1,
             "drop on delete is one undo step");
    window_->closeDocument(doc, false);
}

// Create Clipping Mask clips a full-canvas green fill to a red square below
// it; Release from the base frees it; Alt-clicking the line between the rows
// toggles it back and forth (#63).
void LayersPanelTest::clippingMasks()
{
    QVERIFY(window_->newDocument(QStringLiteral("Clip"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 2;
    square.y0 = 2;
    square.x1 = 8;
    square.y1 = 8;
    const QString base = pictura::shape_add_layer(*view_, square, 0xffff0000u);
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!base.isEmpty() && !fill.isEmpty());
    const QRgb green = 0xff00ff00u;
    const QRgb white = 0xffffffffu;
    QCOMPARE(view_->sample_argb(12, 12), green);
    // The panel picks the new rows up on its queued refresh.
    QCoreApplication::processEvents();

    pictura::CommandRegistry* registry = window_->registry();
    QAction* create = registry->action(QString::fromLatin1(pictura::command_ids::LayerCreateClippingMask));
    QAction* release =
        registry->action(QString::fromLatin1(pictura::command_ids::LayerReleaseClippingMask));
    QVERIFY(create && release);
    QCOMPARE(create->shortcut(), QKeySequence(QStringLiteral("Ctrl+Alt+G")));

    panel_->selectPaths({fill}, fill);
    registry->refresh();
    QVERIFY(create->isEnabled());
    QVERIFY(!release->isEnabled());
    int before = view_->history_count();
    create->trigger();
    QCOMPARE(view_->history_count(), before + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Create Clipping Mask"));
    QVERIFY(pictura::layers_api::rowAt(*view_, rowOf(fill)).clipping);
    QCOMPARE(view_->sample_argb(4, 4), green);
    QCOMPARE(view_->sample_argb(12, 12), white);

    // Release from the base frees the layer clipped to it.
    QCoreApplication::processEvents();
    panel_->selectPaths({base}, base);
    registry->refresh();
    QVERIFY(release->isEnabled());
    release->trigger();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Release Clipping Mask"));
    QVERIFY(!pictura::layers_api::rowAt(*view_, rowOf(fill)).clipping);
    QCOMPARE(view_->sample_argb(12, 12), green);

    // Alt-click on the line between the rows clips, and again releases.
    QCoreApplication::processEvents();
    panel_->altClickBelowRowForTest(fill);
    QVERIFY(pictura::layers_api::rowAt(*view_, rowOf(fill)).clipping);
    QCOMPARE(view_->sample_argb(12, 12), white);
    QCoreApplication::processEvents();
    panel_->altClickBelowRowForTest(fill);
    QVERIFY(!pictura::layers_api::rowAt(*view_, rowOf(fill)).clipping);
}

// Issue #119: a document whose only layer is the locked Background must always
// report that row as active, including after a refresh that finds no selected
// row (a model reset, a document switch, or a cleared selection).
void LayersPanelTest::backgroundOnlyStaysActive()
{
    const bool created = window_->newDocument(QStringLiteral("BgOnly"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    QVERIFY(created);
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);

    // A fully opaque import becomes the single locked Background.
    QImage opaque(16, 16, QImage::Format_RGB32);
    opaque.fill(QColor(200, 30, 30));
    const QString path = QDir::tempPath() + QStringLiteral("/pictura_bg_only.png");
    QVERIFY(opaque.save(path));
    QVERIFY(view_->open_image(path));
    QFile::remove(path);

    QCOMPARE(pictura::layers_api::rowCount(*view_), 1);
    QCOMPARE(pictura::layers_api::rowAt(*view_, 0).kind, QStringLiteral("background"));
    QCOMPARE(view_->active_layer_path(), QStringLiteral("0"));
    QCOMPARE(panel_->currentPath(), QStringLiteral("0"));

    // Clearing the tree selection while the document keeps its active layer
    // must not leave the Background unselected after a refresh.
    panel_->selectPaths(QStringList{}, QString());
    QVERIFY(panel_->selectedPaths().isEmpty());
    view_->set_active_layer(QStringLiteral("0"));
    panel_->refresh();
    QCOMPARE(panel_->currentPath(), QStringLiteral("0"));
    QCOMPARE(panel_->selectedPaths(), QStringList{QStringLiteral("0")});
}

// Issue #119: a refresh that finds no selected row must restore the document's
// active layer, not the first displayed row. A top group (or adjustment) makes
// the active pixel layer differ from the first row.
void LayersPanelTest::activeLayerNotFirstRow()
{
    const bool created = window_->newDocument(QStringLiteral("ActiveNotTop"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    QVERIFY(created);
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);

    QVERIFY(!view_->add_layer_in(QString()).isEmpty());
    QVERIFY(!view_->add_group_in(QString()).isEmpty());
    // The group is the first displayed row; the active pixel layer is "0".
    QCOMPARE(pictura::layers_api::rowAt(*view_, 0).path, QStringLiteral("2"));
    QCOMPARE(pictura::layers_api::rowAt(*view_, 0).kind, QStringLiteral("group"));
    // Clearing the selection syncs an empty active layer, so re-establish the
    // document's active layer (off the first row) before the refresh.
    panel_->selectPaths(QStringList{}, QString());
    view_->set_active_layer(QStringLiteral("0"));
    panel_->refresh();
    QCOMPARE(view_->active_layer_path(), QStringLiteral("0"));
    QCOMPARE(panel_->currentPath(), QStringLiteral("0"));
    QCOMPARE(panel_->selectedPaths(), QStringList{QStringLiteral("0")});
}

// lmk_row_link / lmk_row_disabled / lmk_row_link_click / lmk_row_shift: a
// masked row reports its linked and disabled states, the link-glyph click
// toggles linkage, and a Shift-click on the mask thumbnail toggles enablement.
void LayersPanelTest::maskRowIndicators()
{
    const bool created = window_->newDocument(QStringLiteral("MaskRow"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "mask row fixture");
    const int doc = window_->activeDocumentIndex();
    const QString layer = pictura::layers_api::rowAt(*view_, 0).path;
    QVERIFY(!layer.isEmpty());
    view_->set_active_layer(layer);
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "add mask");
    QVERIFY(pictura::layer_mask_linked(*view_));

    panel_->setView(view_);
    panel_->selectPaths({layer}, layer);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    QVERIFY2(panel_->rowMaskLinkedForTest(layer), "row reports linked");
    QVERIFY2(!panel_->rowMaskDisabledForTest(layer), "row reports enabled");

    // Clicking the link glyph unlinks the mask and the row reflects it.
    QVERIFY2(panel_->clickLinkGlyphForTest(layer), "link glyph target exists");
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(!panel_->rowMaskLinkedForTest(layer), "glyph click unlinked the mask");

    // A linked, disabled mask reports both flags.
    QVERIFY(pictura::layer_mask_set_linked(*view_, true));
    QVERIFY(pictura::layer_mask_set_enabled(*view_, false));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(panel_->rowMaskLinkedForTest(layer), "row reports linked again");
    QVERIFY2(panel_->rowMaskDisabledForTest(layer), "row reports disabled");

    // Shift-clicking the mask thumbnail re-enables the mask.
    QVERIFY2(panel_->shiftClickMaskThumbnailForTest(layer), "mask thumbnail target exists");
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(!panel_->rowMaskDisabledForTest(layer), "shift-click re-enabled the mask");

    window_->closeDocument(doc, false);
}

// The mask edit target: a plain mask-thumbnail click activates the mask, a
// plain layer-thumbnail click returns to the layer's pixels, and the row
// reports the target for the delegate's border.
void LayersPanelTest::maskEditTargetActivation()
{
    const bool created = window_->newDocument(QStringLiteral("MaskActivate"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "mask activation fixture");
    const int doc = window_->activeDocumentIndex();
    const QString layer = pictura::layers_api::rowAt(*view_, 0).path;
    view_->set_active_layer(layer);
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "add mask");

    panel_->setView(view_);
    panel_->selectPaths({layer}, layer);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    // Adding a mask selects its thumbnail: the mask becomes the active target,
    // so its brackets follow the mask and the bridge target is set.
    QCOMPARE(view_->active_layer_path(), layer);
    QVERIFY2(panel_->rowMaskTargetForTest(layer), "adding a mask activates the mask");
    QCOMPARE(panel_->rowActiveThumbForTest(layer), 1);
    QCOMPARE(pictura::mask_edit_target_get(*view_), layer);

    QVERIFY2(panel_->clickLayerThumbnailForTest(layer), "layer thumbnail target exists");
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(!panel_->rowMaskTargetForTest(layer), "layer click deactivated the mask");
    QCOMPARE(panel_->rowActiveThumbForTest(layer), 0);
    QVERIFY(pictura::mask_edit_target_get(*view_).isEmpty());

    QVERIFY2(panel_->clickMaskThumbnailForTest(layer), "mask thumbnail target exists");
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(panel_->rowMaskTargetForTest(layer), "mask click activated the mask");
    QCOMPARE(panel_->rowActiveThumbForTest(layer), 1);

    window_->closeDocument(doc, false);
}

// The row-click contract: a mask glyph click edits the clicked row, not the
// active layer. A click on a non-active row must leave the active layer alone.
void LayersPanelTest::maskRowActionsTargetTheClickedRow()
{
    const bool created = window_->newDocument(QStringLiteral("MaskTarget"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "mask target fixture");
    const int doc = window_->activeDocumentIndex();
    const QString active = view_->add_layer_in(QString());
    const QString clicked = view_->add_layer_in(QString());
    QVERIFY(!active.isEmpty() && !clicked.isEmpty());

    view_->set_active_layer(active);
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "active mask");
    view_->set_active_layer(clicked);
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "clicked mask");
    QVERIFY(pictura::layer_mask_linked(*view_));
    view_->set_active_layer(active);
    QVERIFY(pictura::layer_mask_linked(*view_));

    panel_->setView(view_);
    panel_->selectPaths({active}, active);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    QVERIFY2(panel_->clickLinkGlyphForTest(clicked), "clicked row has a link glyph");
    QCoreApplication::processEvents();
    panel_->refresh();

    QCOMPARE(view_->active_layer_path(), active);
    QVERIFY2(!panel_->rowMaskLinkedForTest(clicked), "the clicked row unlinked");
    QVERIFY2(panel_->rowMaskLinkedForTest(active), "the active row is untouched");

    // The row-menu mask actions are path-based too: deleting the clicked row's
    // mask leaves the active row's mask alone.
    QVERIFY2(panel_->performRowActionForTest(QStringLiteral("deleteLayerMask"), clicked),
             "delete the clicked row's mask");
    panel_->refresh();
    QVERIFY2(!panel_->rowHasMaskForTest(clicked), "the row action hit the clicked row");
    QVERIFY2(panel_->rowHasMaskForTest(active), "the active row keeps its mask");
    QCOMPARE(view_->active_layer_path(), active);

    window_->closeDocument(doc, false);
}

// vmk_row_vector / vmk_row_vector_disabled / vmk_row_vector_link_click /
// vmk_row_vector_shift: a vector-masked row reports its linked and disabled
// states, the vector link-glyph click toggles linkage, and a Shift-click on the
// vector thumbnail toggles enablement.
void LayersPanelTest::vectorMaskRowIndicators()
{
    const bool created = window_->newDocument(QStringLiteral("VectorMaskRow"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "vector mask row fixture");
    const int doc = window_->activeDocumentIndex();
    const QString layer = pictura::layers_api::rowAt(*view_, 0).path;
    QVERIFY(!layer.isEmpty());
    view_->set_active_layer(layer);
    QVERIFY2(pictura::vector_mask_add(*view_, QStringLiteral("reveal-all")), "add vector mask");
    QVERIFY(pictura::vector_mask_present(*view_));
    QVERIFY(pictura::vector_mask_linked(*view_));

    panel_->setView(view_);
    panel_->selectPaths({layer}, layer);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    QVERIFY2(panel_->rowHasVectorMaskForTest(layer), "row reports a vector mask");
    QVERIFY2(panel_->rowVectorMaskLinkedForTest(layer), "row reports linked");
    QVERIFY2(!panel_->rowVectorMaskDisabledForTest(layer), "row reports enabled");

    QVERIFY2(panel_->clickVectorLinkGlyphForTest(layer), "vector link glyph target exists");
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(!panel_->rowVectorMaskLinkedForTest(layer), "glyph click unlinked");

    QVERIFY(pictura::vector_mask_set_linked(*view_, true));
    QVERIFY(pictura::vector_mask_set_enabled(*view_, false));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(panel_->rowVectorMaskLinkedForTest(layer), "row reports linked again");
    QVERIFY2(panel_->rowVectorMaskDisabledForTest(layer), "row reports disabled");

    QVERIFY2(panel_->shiftClickVectorMaskThumbnailForTest(layer), "vector thumbnail target exists");
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(!panel_->rowVectorMaskDisabledForTest(layer), "shift-click re-enabled");

    window_->closeDocument(doc, false);
}

// The New Fill / Adjustment strip menu offers the two implemented fill entries,
// a disabled Pattern… entry, and all sixteen CS6 adjustment kinds; choosing a
// kind creates its layer.
void LayersPanelTest::fillAdjustmentMenu()
{
    QVERIFY2(setupNest(), "strip menu fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();

    auto* button =
        panel_->findChild<QToolButton*>(QStringLiteral("layersStripFillAdjustment"));
    QVERIFY(button != nullptr);
    QMenu* menu = button->menu();
    QVERIFY(menu != nullptr);

    QStringList texts;
    for (QAction* action : menu->actions()) {
        texts << (action->isSeparator() ? QString() : action->text());
    }
    const QStringList expected = {
        QStringLiteral("Solid Color…"),        QStringLiteral("Gradient…"),
        QStringLiteral("Pattern…"),            QString(),
        QStringLiteral("Brightness/Contrast…"), QStringLiteral("Levels…"),
        QStringLiteral("Curves…"),             QStringLiteral("Exposure…"),
        QStringLiteral("Vibrance…"),           QStringLiteral("Hue/Saturation…"),
        QStringLiteral("Color Balance…"),      QStringLiteral("Black & White…"),
        QStringLiteral("Photo Filter…"),       QStringLiteral("Channel Mixer…"),
        QStringLiteral("Color Lookup…"),       QStringLiteral("Invert"),
        QStringLiteral("Posterize…"),          QStringLiteral("Threshold…"),
        QStringLiteral("Gradient Map…"),       QStringLiteral("Selective Color…")};
    QCOMPARE(texts, expected);

    QAction* pattern = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->text() == QStringLiteral("Pattern…")) {
            pattern = action;
        }
    }
    QVERIFY(pattern != nullptr);
    QVERIFY2(!pattern->isEnabled(), "Pattern stays disabled until authoring exists");
    QVERIFY(pattern->toolTip().contains(QStringLiteral("not implemented yet")));

    const int before = pictura::layers_api::rowCount(*view_);
    QAction* levels = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->text() == QStringLiteral("Levels…")) {
            levels = action;
        }
    }
    QVERIFY(levels != nullptr);
    levels->trigger();
    QCOMPARE(pictura::layers_api::rowCount(*view_), before + 1);
    QVERIFY2(countNamed(view_, QStringLiteral("Levels")) == 1, "Levels layer created");

    window_->closeDocument(doc, false);
}

// lpr_label_chip / lpr_smart_badge: a labeled row paints a colour chip at the
// row content edge and a smart-object layer reports the smart-object role.
void LayersPanelTest::layerSurfaceCompleteness()
{
    QVERIFY2(setupNest(), "surface fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();
    panel_->expandForTest(group_);
    window_->show();
    QTest::qWait(50);

    QVERIFY2(!panel_->rowLabelChipColorForTest(a_).isValid(), "no chip before a label");
    QVERIFY2(view_->set_layers_color(QStringList{a_}, 1) == 1, "set red label");
    panel_->refresh();
    const QColor chip = panel_->rowLabelChipColorForTest(a_);
    QVERIFY2(chip.isValid(), "labeled row paints a chip");
    QCOMPARE(chip.rgb(), QColor(255, 0, 0).rgb());
    const QColor gutter = panel_->rowGutterColorForTest(a_);
    QVERIFY2(gutter.red() > gutter.green() && gutter.red() > gutter.blue(),
             "label still tints the eye toggle");
    QVERIFY2(!panel_->rowLabelChipColorForTest(b_).isValid(), "unlabeled row has no chip");

    QVERIFY2(!panel_->rowSmartObjectForTest(a_), "plain layer is not a smart object");
    QVERIFY2(view_->convert_to_smart_object(a_), "convert to smart object");
    panel_->refresh();
    QVERIFY2(panel_->rowSmartObjectForTest(a_), "smart-object row reports the role");

    window_->closeDocument(doc, false);
}

// lpo_copy_name / lpo_default_mask: the two Panel Options flags gate duplicate
// naming and the selection mask a new fill layer receives.
void LayersPanelTest::panelOptionsFlags()
{
    QVERIFY2(setupNest(), "options fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();
    QVERIFY2(panel_->addCopyOnDuplicateForTest(), "Add copy defaults on");
    QVERIFY2(panel_->useDefaultMasksOnFillForTest(), "Default masks default on");

    // Add "copy" off: the copy keeps the source name.
    panel_->setOptionFlagsForTest(false, true);
    const QStringList plain = view_->duplicate_layers(QStringList{a_});
    QCOMPARE(plain.size(), 1);
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(plain.first())).name, QStringLiteral("A"));

    // Add "copy" on: duplicating that copy names it "<name> copy" again.
    panel_->setOptionFlagsForTest(true, true);
    const QStringList named = view_->duplicate_layers(QStringList{plain.first()});
    QCOMPARE(named.size(), 1);
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(named.first())).name, QStringLiteral("A copy"));

    // Default masks on: a fill layer created with a selection carries a mask.
    panel_->setOptionFlagsForTest(true, true);
    view_->select_all();
    const QString maskedFill = view_->add_solid_fill(0xff0000ffu);
    QVERIFY(!maskedFill.isEmpty());
    QVERIFY2(pictura::layers_api::rowAt(*view_, rowOf(maskedFill)).hasMask, "fill took the selection mask");

    // Default masks off: a fill layer created with a selection has no mask.
    panel_->setOptionFlagsForTest(true, false);
    view_->select_all();
    const QString plainFill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!plainFill.isEmpty());
    QVERIFY2(!pictura::layers_api::rowAt(*view_, rowOf(plainFill)).hasMask, "fill has no mask when off");

    window_->closeDocument(doc, false);
}

// m39_rename: Tab commits the inline editor and opens the next visible row,
// Shift+Tab the previous, with no wrap at the ends.
void LayersPanelTest::renameTabNavigation()
{
    QVERIFY2(setupNest(), "rename fixture");
    const int doc = window_->activeDocumentIndex();
    panel_->setView(view_);
    panel_->refresh();
    panel_->expandForTest(group_);
    window_->show();
    QTest::qWait(50);
    auto* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(delegate != nullptr);

    // Shift+Tab on the first visible row commits without wrapping to the last.
    const QString first = pictura::layers_api::rowAt(*view_, 0).path;
    QVERIFY(panel_->beginRenameForTest(first));
    QLineEdit* editor0 = tree->viewport()->findChild<QLineEdit*>();
    QVERIFY(editor0 != nullptr);
    const int firstBase = view_->history_count();
    QKeyEvent backtab0(QEvent::KeyPress, Qt::Key_Backtab, Qt::ShiftModifier);
    QVERIFY(delegate->eventFilter(editor0, &backtab0));
    QCoreApplication::processEvents();
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    QVERIFY2(!panel_->inlineEditorOpenForTest(), "no wrap at the first visible row");
    QCOMPARE(view_->history_count(), firstBase);

    // Tab commits the edit and opens the next visible row's editor.
    QVERIFY(panel_->beginRenameForTest(a_));
    QLineEdit* editor = tree->viewport()->findChild<QLineEdit*>();
    QVERIFY2(editor != nullptr, "inline editor exists");
    editor->setText(QStringLiteral("A2"));
    QKeyEvent tab(QEvent::KeyPress, Qt::Key_Tab, Qt::NoModifier);
    QVERIFY(delegate->eventFilter(editor, &tab));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(a_)).name, QStringLiteral("A2"));
    const QString nextPath = pictura::layers_api::rowAt(*view_, rowOf(a_) + 1).path;
    QCOMPARE(panel_->currentPath(), nextPath);
    QVERIFY2(panel_->inlineEditorOpenForTest(), "Tab opened the next row's editor");

    // Shift+Tab returns to the previous visible row.
    QLineEdit* editor2 = tree->viewport()->findChild<QLineEdit*>();
    QVERIFY(editor2 != nullptr);
    QKeyEvent backtab(QEvent::KeyPress, Qt::Key_Backtab, Qt::ShiftModifier);
    QVERIFY(delegate->eventFilter(editor2, &backtab));
    QCoreApplication::processEvents();
    QCOMPARE(panel_->currentPath(), a_);

    window_->closeDocument(doc, false);
}

// Shape rows offer Copy/Paste Shape Attributes and Rasterize Shape (not the
// adjustment edit), and a shape layer forces the Transparency and Image locks
// on and refuses to clear them.
void LayersPanelTest::shapeRowActions()
{
    const bool created = window_->newDocument(QStringLiteral("ShapeRow"), 32, 32,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "shape row fixture");
    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 4;
    square.y0 = 4;
    square.x1 = 20;
    square.y1 = 20;
    const QString shape = pictura::shape_add_layer(*view_, square, 0xffff0000u);
    QVERIFY(!shape.isEmpty());
    panel_->refresh();
    QCoreApplication::processEvents();

    const QStringList texts = panel_->shapeRowMenuTextsForTest();
    QVERIFY2(texts.contains(QStringLiteral("Copy Shape Attributes")), "copy row");
    QVERIFY2(texts.contains(QStringLiteral("Paste Shape Attributes")), "paste row");
    QVERIFY2(texts.contains(QStringLiteral("Rasterize Shape")), "rasterize row");
    QVERIFY2(!texts.contains(QStringLiteral("Edit Adjustment…")), "no adjustment edit");
    QVERIFY2(panel_->shapeRowMenuEnabledForTest(QStringLiteral("Rasterize Shape")),
             "rasterize shape is enabled");

    panel_->selectPaths({shape}, shape);
    panel_->refresh();
    QVERIFY2((pictura::layers_api::rowAt(*view_, rowOf(shape)).lockBits & 0x03) == 0x03, "forced bits reported");
    QVERIFY2(panel_->lockToggleCheckedForTest(0), "transparency checked");
    QVERIFY2(panel_->lockToggleCheckedForTest(1), "image checked");
    QVERIFY2(!panel_->lockToggleEnabledForTest(0), "transparency forced");
    QVERIFY2(!panel_->lockToggleEnabledForTest(1), "image forced");

    const int before = view_->history_count();
    QCOMPARE(view_->set_layers_lock({shape}, QStringLiteral("transparency"), false), 0);
    QCOMPARE(view_->history_count(), before);
    QVERIFY2((pictura::layers_api::rowAt(*view_, rowOf(shape)).lockBits & 0x01) != 0, "bit stays set");

    window_->closeDocument(window_->activeDocumentIndex(), false);
}

// The Layers panel reaches the layer-style feature: the row menu style rows and
// their enablement, the fx badge projection, the FX Alt-click toggle, and the
// Effect filter dimension.
void LayersPanelTest::layerStylePanelIntegration()
{
    QVERIFY(window_->newDocument(QStringLiteral("Style"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(view_ && panel_, "layer style fixture");
    const int doc = window_->activeDocumentIndex();
    view_->select_rect(10, 10, 10, 10, QStringLiteral("new"), 0.0);
    QVERIFY(!view_->layer_via_copy(QStringLiteral("0")).isEmpty());
    view_->deselect();
    QCoreApplication::processEvents();
    panel_->setView(view_);
    panel_->selectPaths({QStringLiteral("1")}, QStringLiteral("1"));
    panel_->refresh();

    const QString layer = QStringLiteral("1");
    QVERIFY2(!panel_->rowHasStyleForTest(layer), "unstyled row reports no style");
    QVERIFY2(panel_->rowFxRectForTest(layer).isEmpty(), "unstyled row paints no fx badge");
    QVERIFY2(panel_->rowMenuEnabledForPathForTest(layer, QStringLiteral("Blending Options…")),
             "Blending Options is enabled for a pixel layer");
    QVERIFY2(!panel_->rowMenuEnabledForPathForTest(layer, QStringLiteral("Copy Layer Style")),
             "Copy is disabled without a style");
    QVERIFY2(!panel_->rowMenuEnabledForPathForTest(layer, QStringLiteral("Clear Layer Style")),
             "Clear is disabled without a style");
    QVERIFY2(!panel_->rowMenuEnabledForPathForTest(layer, QStringLiteral("Paste Layer Style")),
             "Paste is disabled with an empty clipboard");
    QVERIFY2(!panel_->rowMenuTextsForTest(QStringLiteral("adjustment"))
                  .contains(QStringLiteral("Blending Options…")),
             "adjustment rows omit the style commands");

    // A color overlay makes the row project the style and enables Copy / Clear.
    QVERIFY(pictura::layer_style_set(*view_, layer, QStringLiteral("colorOverlay.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(panel_->rowHasStyleForTest(layer), "styled row reports a style");
    QVERIFY2(!panel_->rowFxRectForTest(layer).isEmpty(), "styled row paints the fx badge");
    QVERIFY2(panel_->rowMenuEnabledForPathForTest(layer, QStringLiteral("Copy Layer Style")),
             "Copy is enabled with a style");
    QVERIFY2(panel_->rowMenuEnabledForPathForTest(layer, QStringLiteral("Clear Layer Style")),
             "Clear is enabled with a style");
    QVERIFY(pictura::layer_style_copy(*view_, layer));
    panel_->refresh();
    QVERIFY2(panel_->rowMenuEnabledForPathForTest(layer, QStringLiteral("Paste Layer Style")),
             "Paste is enabled after a copy");

    // The Effect dimension is populated from the effect keys and matches only a
    // row whose layer carries that effect.
    auto* effectCombo = panel_->findChild<QComboBox*>(QStringLiteral("layersFilterEffect"));
    QVERIFY(effectCombo != nullptr);
    QVERIFY(effectCombo->count() >= 10);
    QVERIFY(effectCombo->findData(QStringLiteral("colorOverlay")) >= 0);
    panel_->setFilterEffectForTest(QStringLiteral("colorOverlay"), true);
    QVERIFY2(panel_->visiblePathsForTest().contains(layer), "styled row passes its effect filter");
    QVERIFY2(!panel_->visiblePathsForTest().contains(QStringLiteral("0")),
             "unstyled background is filtered out");
    panel_->setFilterEffectForTest(QStringLiteral("dropShadow"), true);
    QVERIFY2(!panel_->visiblePathsForTest().contains(layer),
             "a row without the effect is filtered out");
    panel_->setFilterEffectForTest(QString(), false);
    panel_->refresh();

    // Alt-clicking the fx badge toggles every layer's effects, one undo step.
    QVERIFY2(panel_->rowHasStyleForTest(layer), "row still styled before the fx toggle");
    QVERIFY2(!panel_->rowFxRectForTest(layer).isEmpty(), "fx badge present before the toggle");
    const int before = view_->history_count();
    panel_->altClickRowFxForTest(layer);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Hide All Effects"));
    panel_->altClickRowFxForTest(layer);
    QCOMPARE(view_->history_count(), before + 2);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Show All Effects"));

    // The row-menu Clear removes the style and the badge, in one undo step.
    QVERIFY(panel_->performRowActionForTest(QStringLiteral("clearLayerStyle"), layer));
    QCoreApplication::processEvents();
    panel_->refresh();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Clear Layer Style"));
    QVERIFY2(!panel_->rowHasStyleForTest(layer), "cleared row reports no style");
    QVERIFY2(panel_->rowFxRectForTest(layer).isEmpty(), "cleared row paints no fx badge");

    window_->closeDocument(doc, false);
}

void LayersPanelTest::blendIfBadge()
{
    QVERIFY(window_->newDocument(QStringLiteral("BlendIf"), 20, 20, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(view_ && panel_, "blend if fixture");
    const int doc = window_->activeDocumentIndex();
    view_->select_rect(2, 2, 8, 8, QStringLiteral("new"), 0.0);
    QVERIFY(!view_->layer_via_copy(QStringLiteral("0")).isEmpty());
    view_->deselect();
    QCoreApplication::processEvents();
    panel_->setView(view_);
    panel_->selectPaths({QStringLiteral("1")}, QStringLiteral("1"));
    panel_->refresh();

    const QString layer = QStringLiteral("1");
    QVERIFY2(!panel_->rowHasBlendIfForTest(layer), "default row has no Blend If badge");
    QVERIFY2(panel_->rowBlendIfRectForTest(layer).isEmpty(), "default row paints no chip");

    // A narrowed composite-source range customises Blend If.
    QVERIFY(pictura::layer_style_set_blend_if(*view_, layer, 0, 40000));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(panel_->rowHasBlendIfForTest(layer), "customised row reports a Blend If badge");
    QVERIFY2(!panel_->rowBlendIfRectForTest(layer).isEmpty(), "customised row paints the chip");

    // Restoring the full default range removes the badge.
    QVERIFY(pictura::layer_style_set_blend_if(*view_, layer, 0, 65535));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY2(!panel_->rowHasBlendIfForTest(layer), "restored default has no badge");
    QVERIFY2(panel_->rowBlendIfRectForTest(layer).isEmpty(), "restored default paints no chip");

    window_->closeDocument(doc, false);
}

void LayersPanelTest::isolationMode()
{
    QVERIFY(window_->newDocument(QStringLiteral("Isolation"), 30, 30, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(view_ && panel_, "isolation fixture");
    const int doc = window_->activeDocumentIndex();
    view_->select_rect(5, 5, 10, 10, QStringLiteral("new"), 0.0);
    QVERIFY(!view_->layer_via_copy(QStringLiteral("0")).isEmpty());
    view_->deselect();
    QCoreApplication::processEvents();
    panel_->setView(view_);
    panel_->selectPaths({QStringLiteral("1")}, QStringLiteral("1"));
    panel_->refresh();

    QVERIFY2(!panel_->isolationActive(), "not isolated by default");
    QVERIFY2(!view_->isolation_active(), "Auto-Select is unscoped by default");

    // Entering isolation captures the selection as the fixed set: only those
    // layers show, and Auto-Select is scoped to them.
    panel_->selectPaths({QStringLiteral("0"), QStringLiteral("1")}, QStringLiteral("0"));
    panel_->enterIsolation();
    QCoreApplication::processEvents();
    QVERIFY2(panel_->isolationActive(), "the Selected filter isolates");
    QVERIFY2(view_->isolation_active(), "Auto-Select is scoped");
    QVERIFY2(panel_->visiblePathsForTest().contains(QStringLiteral("0"))
                 && panel_->visiblePathsForTest().contains(QStringLiteral("1")),
             "both isolated layers are shown");
    QVERIFY2(panel_->rowMenuTextsForPathForTest(QStringLiteral("1"))
                 .contains(QStringLiteral("Release from Isolation")),
             "the row menu offers Release from Isolation");

    // Releasing one layer leaves the other isolated and hides the released row.
    panel_->releaseIsolationForTest(QStringLiteral("0"));
    QCoreApplication::processEvents();
    QVERIFY2(panel_->isolationActive(), "one layer remains isolated");
    QVERIFY2(!panel_->visiblePathsForTest().contains(QStringLiteral("0")),
             "the released layer is hidden");
    QVERIFY2(panel_->visiblePathsForTest().contains(QStringLiteral("1")), "the other remains");

    // Releasing the last layer ends isolation and unscopes Auto-Select.
    panel_->releaseIsolationForTest(QStringLiteral("1"));
    QCoreApplication::processEvents();
    QVERIFY2(!panel_->isolationActive(), "releasing the last layer ends isolation");
    QVERIFY2(panel_->visiblePathsForTest().contains(QStringLiteral("0"))
                 && panel_->visiblePathsForTest().contains(QStringLiteral("1")),
             "every layer is shown again");
    QVERIFY2(!view_->isolation_active(), "Auto-Select is unscoped again");

    // The frame's Escape path releases isolation.
    panel_->selectPaths({QStringLiteral("1")}, QStringLiteral("1"));
    panel_->enterIsolation();
    QCoreApplication::processEvents();
    QVERIFY(panel_->isolationActive());
    QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QApplication::sendEvent(window_.get(), &esc);
    QCoreApplication::processEvents();
    QVERIFY2(!panel_->isolationActive(), "Escape leaves isolation");

    // Re-enabling the filter after a release starts a fresh set from the current
    // selection rather than resuming an empty one (a blank panel).
    panel_->setFilterEnabledForTest(true);
    QCoreApplication::processEvents();
    QVERIFY2(panel_->isolationActive(), "re-enabling isolates again");
    QVERIFY2(!panel_->visiblePathsForTest().isEmpty(), "re-enabling does not blank the panel");
    panel_->releaseIsolation();
    QCoreApplication::processEvents();
    QVERIFY2(!panel_->isolationActive(), "released again");

    window_->closeDocument(doc, false);
}

QTEST_MAIN(LayersPanelTest)
#include "tst_layers_panel.moc"
