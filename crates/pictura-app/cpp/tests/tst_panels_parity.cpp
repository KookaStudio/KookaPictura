#include <QtTest/QtTest>
#include "api/layers_api.h"

#include <QtCore/QAbstractItemModel>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QTemporaryDir>
#include <QtGui/QColor>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QListView>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>

#include <limits>

#include "frame.h"
#include "icons.h"
#include "image_view.h"
#include "panels/channels_panel.h"
#include "panels/color_panel.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "panels/panel_column.h"
#include "panels/panel_group.h"
#include "theme.h"
#include "toolbox.h"
#include "api/layers_bridge.h"

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

void moveMouse(QWidget* target, const QPoint& pos, Qt::KeyboardModifiers mods)
{
    QMouseEvent event(QEvent::MouseMove, QPointF(pos), target->mapToGlobal(pos), Qt::NoButton,
                      Qt::NoButton, mods);
    QCoreApplication::sendEvent(target, &event);
}

void clickMouse(QWidget* target, const QPoint& pos, Qt::KeyboardModifiers mods)
{
    QMouseEvent press(QEvent::MouseButtonPress, QPointF(pos), target->mapToGlobal(pos),
                      Qt::LeftButton, Qt::LeftButton, mods);
    QCoreApplication::sendEvent(target, &press);
    QMouseEvent release(QEvent::MouseButtonRelease, QPointF(pos), target->mapToGlobal(pos),
                        Qt::LeftButton, Qt::NoButton, mods);
    QCoreApplication::sendEvent(target, &release);
}

// A point over the document area, clear of every column and of the workspace's
// new-column edge bands, so a drag there resolves an outside tear-off.
QPoint outsideColumnPoint(pictura::PicturaMainWindow* window)
{
    QWidget* central = window ? window->centralWidget() : nullptr;
    if (!central) {
        return QPoint(-10000, -10000);
    }
    return central->mapToGlobal(QPoint(central->width() / 2, central->height() / 2));
}

// The summed per-channel difference between two equally sized, formatted images.
qint64 imageDistance(const QImage& a, const QImage& b)
{
    if (a.size() != b.size() || a.format() != b.format()) {
        return std::numeric_limits<qint64>::max();
    }
    qint64 total = 0;
    for (int y = 0; y < a.height(); ++y) {
        for (int x = 0; x < a.width(); ++x) {
            const QRgb pa = a.pixel(x, y);
            const QRgb pb = b.pixel(x, y);
            total += qAbs(qRed(pa) - qRed(pb)) + qAbs(qGreen(pa) - qGreen(pb))
                + qAbs(qBlue(pa) - qBlue(pb)) + qAbs(qAlpha(pa) - qAlpha(pb));
        }
    }
    return total;
}

} // namespace

class PanelsParityTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void layerHoverCursor();
    void lockBadgeClickUnlocks();
    void lockBadgeShrunk();
    void kindBadgesHidpi();
    void maskingBackgroundConvertsIt();
    void thumbnailAdjacencyAndActiveThumb();
    void emptyGroupShowsChevron();
    void visibilityToggleIsResponsive();
    void channelHoverAndCtrlOverlay();
    void channelRowsUseLayerSurfaces();
    void compactFlyoutClosesWhenGroupTearsOff();
    void compactFlyoutStaysOpenForTabTearOff();
    void compactFlyoutClosesWhenTabEmptiesIt();

private:
    bool setupLayers();
    int rowOf(const QString& path);
    void checkKindBadge(const QString& path, const QString& assetId);
    void redockAllFloats(pictura::PanelColumn* column);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void PanelsParityTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void PanelsParityTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

bool PanelsParityTest::setupLayers()
{
    if (!window_->newDocument(QStringLiteral("Parity"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_) {
        return false;
    }
    panel_->setView(view_);
    panel_->refresh();
    return true;
}

int PanelsParityTest::rowOf(const QString& path)
{
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        if (pictura::layers_api::rowAt(*view_, i).path == path) {
            return i;
        }
    }
    return -1;
}

void PanelsParityTest::layerHoverCursor()
{
    QVERIFY2(setupLayers(), "layers fixture");
    const int doc = window_->activeDocumentIndex();
    window_->show();
    QTest::qWait(0);
    QTreeView* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    QWidget* viewport = tree->viewport();

    const QString path = pictura::layers_api::rowAt(*view_, 0).path;
    const QRect row = panel_->rowViewportRectForTest(path);
    QVERIFY(row.isValid());
    moveMouse(viewport, row.center(), Qt::NoModifier);
    QCOMPARE(viewport->cursor().shape(), Qt::PointingHandCursor);

    // Off every row: the default arrow.
    moveMouse(viewport, QPoint(row.center().x(), viewport->height() - 2), Qt::NoModifier);
    QCOMPARE(viewport->cursor().shape(), Qt::ArrowCursor);

    // Leaving the viewport restores the inherited cursor.
    QEvent leave(QEvent::Leave);
    QCoreApplication::sendEvent(viewport, &leave);
    QCOMPARE(viewport->cursor().shape(), Qt::ArrowCursor);
    window_->closeDocument(doc, false);
}

void PanelsParityTest::lockBadgeClickUnlocks()
{
    QVERIFY2(setupLayers(), "lock fixture");
    const int doc = window_->activeDocumentIndex();
    const QString path = view_->add_layer_in(QString());
    QVERIFY(!path.isEmpty());
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(path)).kind, QStringLiteral("pixel"));
    view_->set_active_layer(path);
    QCOMPARE(view_->set_layers_lock(QStringList{path}, QStringLiteral("position"), true), 1);
    QVERIFY((pictura::layers_api::rowAt(*view_, rowOf(path)).lockBits & 0x04) != 0);

    panel_->setView(view_);
    panel_->selectPaths({path}, path);
    panel_->refresh();
    window_->show();
    QTest::qWait(0);

    QVERIFY2(panel_->clickLockBadgeForTest(path), "lock badge click");
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(path)).lockBits, 0);
    window_->closeDocument(doc, false);
}

void PanelsParityTest::lockBadgeShrunk()
{
    QVERIFY2(setupLayers(), "lock size fixture");
    const int doc = window_->activeDocumentIndex();
    const QString path = view_->add_layer_in(QString());
    view_->set_active_layer(path);
    view_->set_layers_lock(QStringList{path}, QStringLiteral("position"), true);
    panel_->refresh();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(delegate != nullptr);
    QTreeView* tree = panel_->findChild<QTreeView*>();
    const QModelIndex index = indexForPath(tree->model(), path);
    const int expected = qMax(10, delegate->thumbnailSize() * 2 / 3);
    QCOMPARE(delegate->lockRect(tree->visualRect(index)).width(), expected);
    QVERIFY(expected < delegate->thumbnailSize());
    window_->closeDocument(doc, false);
}

// Issue #1201: a shape or smart-object row's lower-right kind badge is painted
// from a pixmap at the painter's device pixel ratio, so it stays crisp on a
// high-density display instead of being upscaled from a logical-size pixmap.
void PanelsParityTest::kindBadgesHidpi()
{
    QVERIFY2(setupLayers(), "hidpi badge fixture");
    const int doc = window_->activeDocumentIndex();

    const QString smart = view_->add_layer_in(QString());
    QVERIFY(!smart.isEmpty());
    QVERIFY2(view_->convert_to_smart_object(smart), "convert to smart object");

    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 2;
    square.y0 = 2;
    square.x1 = 8;
    square.y1 = 8;
    const QString shape = pictura::shape_add_layer(*view_, square, 0xffff0000u);
    QVERIFY(!shape.isEmpty());

    panel_->refresh();
    window_->show();
    QTest::qWait(0);

    // The active-thumbnail brackets paint after the badge; clear the selection
    // so the badge corner holds only the base fill and the badge.
    if (auto* tree = panel_->findChild<QTreeView*>()) {
        tree->clearSelection();
    }

    checkKindBadge(smart, QStringLiteral("layers.kindSmartObject"));
    checkKindBadge(shape, QStringLiteral("layers.kindShape"));
    window_->closeDocument(doc, false);
}

void PanelsParityTest::checkKindBadge(const QString& path, const QString& assetId)
{
    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree != nullptr);
    QVERIFY(delegate != nullptr);
    const QModelIndex index = indexForPath(tree->model(), path);
    QVERIFY2(index.isValid(), qPrintable(path));
    const QRect vr = tree->visualRect(index);
    QVERIFY(vr.isValid());
    const QRect corner = delegate->kindBadgeRect(QRect(0, 0, vr.width(), vr.height()), index);
    QVERIFY2(!corner.isEmpty(), qPrintable(assetId));

    const qreal dpr = 2.0;
    const QImage row = panel_->rowImageForTest(path, dpr);
    QVERIFY(!row.isNull());
    QImage flat = row;
    flat.setDevicePixelRatio(1.0);
    const QRect device(qRound(corner.x() * dpr), qRound(corner.y() * dpr),
                       qRound(corner.width() * dpr), qRound(corner.height() * dpr));
    const QImage actual = flat.copy(device);
    QCOMPARE(actual.size(), device.size());

    // Two references for the same corner: the badge drawn from a 2x pixmap and
    // the badge drawn from a 1x pixmap upscaled to 2x. The painted badge must
    // be closer to the crisp one.
    const QColor base(128, 128, 128);
    const QIcon badge = pictura::icon(assetId);
    const auto reference = [&](qreal badgeDpr) {
        QImage image(device.size(), QImage::Format_ARGB32_Premultiplied);
        image.setDevicePixelRatio(dpr);
        image.fill(base);
        QPainter painter(&image);
        painter.drawPixmap(QRect(0, 0, corner.width(), corner.height()),
                           badge.pixmap(corner.size(), badgeDpr));
        painter.end();
        return image;
    };
    const QImage crisp = reference(dpr);
    const QImage blurred = reference(1.0);
    QVERIFY2(imageDistance(actual, crisp) < imageDistance(actual, blurred),
             qPrintable(QStringLiteral("%1 badge is rendered at the screen DPR").arg(assetId)));
}

void PanelsParityTest::maskingBackgroundConvertsIt()
{
    const bool created = window_->newDocument(QStringLiteral("BgMask"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    QVERIFY(created);
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);

    QImage opaque(16, 16, QImage::Format_RGB32);
    opaque.fill(QColor(200, 30, 30));
    const QString file = QDir::tempPath() + QStringLiteral("/pictura_bg_mask.png");
    QVERIFY(opaque.save(file));
    QVERIFY(view_->open_image(file));
    QFile::remove(file);

    const QString bg = pictura::layers_api::rowAt(*view_, 0).path;
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(bg)).kind, QStringLiteral("background"));
    panel_->setView(view_);
    panel_->selectPaths({bg}, bg);
    panel_->refresh();

    auto* maskButton = panel_->findChild<QToolButton*>(QStringLiteral("layersStripMask"));
    QVERIFY(maskButton != nullptr);
    maskButton->click();
    QCoreApplication::processEvents();

    QVERIFY2(pictura::layers_api::rowAt(*view_, rowOf(bg)).kind != QStringLiteral("background"),
             "Background converted to a regular layer");
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(bg)).lockBits, 0);
    QVERIFY2(pictura::layers_api::rowAt(*view_, rowOf(bg)).hasMask, "mask added");
    window_->closeDocument(window_->activeDocumentIndex(), false);
}

void PanelsParityTest::thumbnailAdjacencyAndActiveThumb()
{
    QVERIFY2(setupLayers(), "thumb fixture");
    const int doc = window_->activeDocumentIndex();
    const QString path = view_->add_layer_in(QString());
    view_->set_active_layer(path);
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "raster mask");
    QVERIFY2(pictura::vector_mask_add(*view_, QStringLiteral("reveal-all")), "vector mask");

    panel_->setView(view_);
    panel_->selectPaths({path}, path);
    panel_->refresh();
    window_->show();
    QTest::qWait(0);

    QTreeView* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    const QModelIndex index = indexForPath(tree->model(), path);
    QVERIFY(index.isValid());
    const QRect vr = tree->visualRect(index);

    const QRect image = delegate->thumbRect(vr, index);
    const QRect maskLink = delegate->linkGlyphRect(vr, index);
    const QRect mask = delegate->maskThumbRect(vr, index);
    const QRect vectorLink = delegate->vectorLinkGlyphRect(vr, index);
    const QRect vector = delegate->vectorMaskThumbRect(vr, index);
    const QRect name = delegate->nameRect(vr, index);
    QVERIFY(!image.isEmpty() && !maskLink.isEmpty() && !mask.isEmpty());
    QVERIFY(!vectorLink.isEmpty() && !vector.isEmpty());
    QVERIFY(maskLink.left() >= image.right());
    QVERIFY(mask.left() > maskLink.left());
    QVERIFY(vectorLink.left() >= mask.right());
    QVERIFY(vector.left() > vectorLink.left());
    QVERIFY(name.left() >= vector.right());

    QWidget* viewport = tree->viewport();
    auto* colors = window_->findChild<pictura::ColorState*>();
    auto* toolbox = window_->findChild<pictura::Toolbox*>();
    QVERIFY(colors && toolbox);

    // A mask/vector click also selects the row and refreshes the model, so the
    // index is re-fetched after each click rather than reused.
    const QColor imageFg(200, 40, 40);
    const QColor imageBg(20, 90, 200);
    clickMouse(viewport, delegate->thumbRect(tree->visualRect(index), index).center(),
               Qt::NoModifier);
    QCOMPARE(indexForPath(tree->model(), path).data(pictura::ActiveThumbRole).toInt(), 0);
    colors->setForeground(imageFg);
    colors->setBackground(imageBg);

    const QModelIndex beforeMask = indexForPath(tree->model(), path);
    clickMouse(viewport,
               delegate->maskThumbRect(tree->visualRect(beforeMask), beforeMask).center(),
               Qt::NoModifier);
    QCOMPARE(indexForPath(tree->model(), path).data(pictura::ActiveThumbRole).toInt(), 1);
    toolbox->resetForegroundBackground();
    QCOMPARE(colors->foreground(), QColor(Qt::white));
    QCOMPARE(colors->background(), QColor(Qt::black));

    const QModelIndex afterMask = indexForPath(tree->model(), path);
    clickMouse(viewport, delegate->vectorMaskThumbRect(tree->visualRect(afterMask), afterMask).center(),
               Qt::NoModifier);
    QCOMPARE(indexForPath(tree->model(), path).data(pictura::ActiveThumbRole).toInt(), 2);
    // Leaving the raster target restores the image pair, including reset.
    QCOMPARE(colors->foreground(), imageFg);
    QCOMPARE(colors->background(), imageBg);

    const QModelIndex afterVector = indexForPath(tree->model(), path);
    clickMouse(viewport, delegate->thumbRect(tree->visualRect(afterVector), afterVector).center(),
               Qt::NoModifier);
    QCOMPARE(indexForPath(tree->model(), path).data(pictura::ActiveThumbRole).toInt(), 0);
    QCOMPARE(colors->foreground(), imageFg);
    QCOMPARE(colors->background(), imageBg);
    window_->closeDocument(doc, false);
}

void PanelsParityTest::emptyGroupShowsChevron()
{
    QVERIFY2(setupLayers(), "group fixture");
    const int doc = window_->activeDocumentIndex();
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QCOMPARE(pictura::layers_api::rowAt(*view_, rowOf(group)).childCount, 0);
    QVERIFY2(pictura::layers_api::rowAt(*view_, rowOf(group)).expandable, "empty group is expandable");
    panel_->refresh();
    QTreeView* tree = panel_->findChild<QTreeView*>();
    const QModelIndex index = indexForPath(tree->model(), group);
    QVERIFY(index.isValid());
    QVERIFY2(index.data(pictura::ExpandableRole).toBool(), "group row reserves the chevron");
    QCOMPARE(index.data(pictura::KindRole).toString(), QStringLiteral("group"));
    window_->closeDocument(doc, false);
}

void PanelsParityTest::visibilityToggleIsResponsive()
{
    QVERIFY2(setupLayers(), "visibility fixture");
    const int doc = window_->activeDocumentIndex();
    const QString path = view_->add_layer_in(QString());
    panel_->setView(view_);
    panel_->selectPaths({path}, path);
    panel_->refresh();
    window_->show();
    QTest::qWait(0);

    QTreeView* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    QWidget* viewport = tree->viewport();
    const int r = rowOf(path);
    const bool before = pictura::layers_api::rowAt(*view_, r).visible;
    const QRect row = panel_->rowViewportRectForTest(path);
    QVERIFY(row.isValid());

    // Two quick toggles without pumping the event loop: the in-place model
    // patch must let the second click read the first's value.
    clickMouse(viewport, QPoint(row.left() + 2, row.center().y()), Qt::NoModifier);
    clickMouse(viewport, QPoint(row.left() + 2, row.center().y()), Qt::NoModifier);
    QCOMPARE(pictura::layers_api::rowAt(*view_, r).visible, before);
    window_->closeDocument(doc, false);
}

void PanelsParityTest::channelHoverAndCtrlOverlay()
{
    QVERIFY(window_->newDocument(QStringLiteral("Chan"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    auto* panel = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel);
    panel->setView(view_);
    panel->refresh();
    window_->show();
    QTest::qWait(0);

    QCOMPARE(panel->hoverChannelForTest(0, Qt::NoModifier),
             static_cast<int>(Qt::PointingHandCursor));
    QVERIFY2(!panel->ctrlOverlayVisibleForTest(), "no overlay without Ctrl");
    QCOMPARE(panel->hoverChannelForTest(0, Qt::ControlModifier),
             static_cast<int>(Qt::PointingHandCursor));
    QVERIFY2(panel->ctrlOverlayVisibleForTest(), "Ctrl shows the select-all overlay");
    panel->hoverChannelForTest(0, Qt::NoModifier);
    QVERIFY2(!panel->ctrlOverlayVisibleForTest(), "releasing Ctrl hides the overlay");
}

void PanelsParityTest::channelRowsUseLayerSurfaces()
{
    QVERIFY(window_->newDocument(QStringLiteral("ChanStyle"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    auto* panel = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel);
    panel->setView(view_);
    panel->refresh();

    const QColor base = panel->palette().color(QPalette::Window);
    QCOMPARE(panel->rowSurfaceForTest(0).rgba(), pictura::Theme::shade(base, 3).rgba());
    QCOMPARE(panel->rowSurfaceForTest(1).rgba(), pictura::Theme::shade(base, 1).rgba());
}

void PanelsParityTest::redockAllFloats(pictura::PanelColumn* column)
{
    for (int i = 0; i < 8 && column->floatCountForTest() > 0; ++i) {
        if (!column->redockForTest(0, 0)) {
            break;
        }
        QCoreApplication::processEvents();
    }
}

void PanelsParityTest::compactFlyoutClosesWhenGroupTearsOff()
{
    window_->resize(1100, 700);
    window_->show();
    QTest::qWait(0);
    pictura::PanelColumn* column = window_->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column != nullptr);
    column->setRailMode(true);
    QTest::qWait(0);

    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    QVERIFY(column->iconFlyoutVisibleForTest());
    pictura::PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group != nullptr);
    QStringList expected;
    for (QWidget* panel : group->panels()) {
        if (panel) {
            expected << panel->objectName();
        }
    }
    QVERIFY(expected.contains(QStringLiteral("layersPanel")));

    const int before = column->floatCountForTest();
    const QPoint outside = outsideColumnPoint(window_.get());
    QVERIFY(column->beginGroupDragForTest(QStringLiteral("layersPanel")));
    column->dragToForTest(outside);
    QVERIFY(column->dropForTest(outside));
    QCoreApplication::processEvents();

    QVERIFY2(!column->iconFlyoutVisibleForTest(), "the empty flyout is closed");
    QCOMPARE(column->floatCountForTest(), before + 1);
    QCOMPARE(column->floatPanelNamesForTest(column->floatCountForTest() - 1), expected);
    QVERIFY(column->groupForPanel(QStringLiteral("layersPanel")) == nullptr);

    redockAllFloats(column);
    column->setRailMode(false);
    QCoreApplication::processEvents();
}

void PanelsParityTest::compactFlyoutStaysOpenForTabTearOff()
{
    window_->resize(1100, 700);
    window_->show();
    QTest::qWait(0);
    pictura::PanelColumn* column = window_->columnForPanel(QStringLiteral("layersPanel"));
    QVERIFY(column != nullptr);
    column->setRailMode(true);
    QTest::qWait(0);

    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("layersPanel")));
    QVERIFY(column->iconFlyoutVisibleForTest());
    pictura::PanelGroup* group = column->groupForPanel(QStringLiteral("layersPanel"));
    QVERIFY(group != nullptr);
    QVERIFY(group->containsPanel(QStringLiteral("channelsPanel")));

    const int before = column->floatCountForTest();
    const QPoint outside = outsideColumnPoint(window_.get());
    QVERIFY(column->beginTabDragForTest(QStringLiteral("channelsPanel")));
    column->dragToForTest(outside);
    QVERIFY(column->dropForTest(outside));
    QCoreApplication::processEvents();

    QVERIFY2(column->iconFlyoutVisibleForTest(), "the flyout keeps the remaining panels");
    QCOMPARE(column->iconFlyoutGroupForTest(), group);
    QVERIFY(group->containsPanel(QStringLiteral("layersPanel")));
    QVERIFY(group->containsPanel(QStringLiteral("pathsPanel")));
    QVERIFY(!group->containsPanel(QStringLiteral("channelsPanel")));
    QCOMPARE(column->floatCountForTest(), before + 1);
    QCOMPARE(column->floatPanelNamesForTest(column->floatCountForTest() - 1),
             QStringList{QStringLiteral("channelsPanel")});

    QVERIFY(column->triggerFlyoutCloseForTest());
    QCoreApplication::processEvents();
    redockAllFloats(column);
    column->setRailMode(false);
    QCoreApplication::processEvents();
}

void PanelsParityTest::compactFlyoutClosesWhenTabEmptiesIt()
{
    window_->resize(1100, 700);
    window_->show();
    QTest::qWait(0);
    pictura::PanelColumn* column = window_->columnForPanel(QStringLiteral("propertiesPanel"));
    QVERIFY(column != nullptr);
    column->setRailMode(true);
    QTest::qWait(0);

    QVERIFY(column->openIconFlyoutForTest(QStringLiteral("propertiesPanel")));
    QVERIFY(column->iconFlyoutVisibleForTest());
    pictura::PanelGroup* group = column->groupForPanel(QStringLiteral("propertiesPanel"));
    QVERIFY(group != nullptr);
    QCOMPARE(static_cast<int>(group->visibleTitles().size()), 1);

    const int before = column->floatCountForTest();
    const QPoint outside = outsideColumnPoint(window_.get());
    QVERIFY(column->beginTabDragForTest(QStringLiteral("propertiesPanel")));
    column->dragToForTest(outside);
    QVERIFY(column->dropForTest(outside));
    QCoreApplication::processEvents();

    QVERIFY2(!column->iconFlyoutVisibleForTest(), "the emptied flyout is closed");
    QCOMPARE(column->floatCountForTest(), before + 1);
    QCOMPARE(column->floatPanelNamesForTest(column->floatCountForTest() - 1),
             QStringList{QStringLiteral("propertiesPanel")});

    redockAllFloats(column);
    column->setRailMode(false);
    QCoreApplication::processEvents();
}

QTEST_MAIN(PanelsParityTest)
#include "tst_panels_parity.moc"
