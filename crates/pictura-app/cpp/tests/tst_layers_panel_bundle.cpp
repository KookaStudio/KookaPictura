// The anchored creation, default-mask, and mask-target fixes for the Layers
// panel bundle (#898, #895a, #897 plumbing).

#include <QtTest/QtTest>

#include <QtCore/QAbstractItemModel>
#include <QtCore/QRect>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>

#include "api/layers_api.h"

#include "api/layers_bridge.h"

#include "commands.h"
#include "frame.h"
#include "layer_style_dialog.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "qt_test_support.h"
#include "theme.h"

namespace {
QModelIndex rowIndexForPath(QAbstractItemModel* model, const QString& path,
                            const QModelIndex& parent = QModelIndex())
{
    for (int row = 0; row < model->rowCount(parent); ++row) {
        const QModelIndex index = model->index(row, 0, parent);
        if (index.data(pictura::PathRole).toString() == path) {
            return index;
        }
        const QModelIndex child = rowIndexForPath(model, path, index);
        if (child.isValid()) {
            return child;
        }
    }
    return {};
}

int differingPixels(const QImage& a, const QImage& b)
{
    if (a.size() != b.size()) {
        return -1;
    }
    int diff = 0;
    for (int y = 0; y < a.height(); ++y) {
        for (int x = 0; x < a.width(); ++x) {
            if (a.pixel(x, y) != b.pixel(x, y)) {
                ++diff;
            }
        }
    }
    return diff;
}
} // namespace

class LayersPanelBundleTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void groupInsertsAtTopWithoutSelection();
    void duplicateInsertsAboveAndSelects();
    void deleteRemovesTheLayer();
    void duplicateBackgroundMakesOrdinaryLayer();
    void adjustmentInsertsAboveActive();
    void anchoredInsertionClearsLinks();
    void fillInsertsAboveActive();
    void fillMaskFromSelection();
    void fillMaskWhiteWithoutSelection();
    void fillMaskOff();
    void fillClippingFromBridge();
    void fillThumbnailPaintsPreview();
    void copyToggleOffKeepsName();
    void addMaskByDefaultToggles();
    void adjustmentMaskFromSelection();
    void adjustmentMaskWhite();
    void adjustmentMaskOff();
    void flyoutStateFollowsDocument();
    void renameHitAreaMatchesRenderedLabel();
    void shortLayerNameIsNotElided();

private:
    bool newDoc(const QString& name);
    QString makeLayer(const QString& name);
    pictura::layers_api::Snapshot snapshot() const;
    pictura::layers_api::LayerRow snapshotRow(const QString& path) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void LayersPanelBundleTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

// The panel flyout is built at construction, before a document exists; opening
// a document must re-enable its registry commands without a tab switch.
void LayersPanelBundleTest::flyoutStateFollowsDocument()
{
    QCOMPARE(window_->documentCount(), 0);
    QVERIFY(window_->newDocument(QStringLiteral("FlyoutState"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    auto* button =
        window_->findChild<QToolButton*>(QStringLiteral("panelWidgetMenu_layersPanel"));
    QVERIFY(button != nullptr);
    QMenu* menu = button->menu();
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();
    QAction* newLayer = nullptr;
    for (QAction* action : menu->actions()) {
        if (action->data().toString()
            == QString::fromLatin1(pictura::command_ids::PanelLayersNewLayer)) {
            newLayer = action;
            break;
        }
    }
    QVERIFY2(newLayer != nullptr, "the flyout carries the New Layer command");
    QVERIFY2(newLayer->isEnabled(),
             "New Layer follows a document opened after the menu was built");
}

void LayersPanelBundleTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

bool LayersPanelBundleTest::newDoc(const QString& name)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    return view_ && panel_;
}

QString LayersPanelBundleTest::makeLayer(const QString& name)
{
    const QString path = view_->add_layer_in(QString());
    if (!path.isEmpty()) {
        view_->set_layer_name_path(path, name);
    }
    return path;
}

pictura::layers_api::Snapshot LayersPanelBundleTest::snapshot() const
{
    return pictura::layers_api::snapshot(*view_, 24, false);
}

pictura::layers_api::LayerRow LayersPanelBundleTest::snapshotRow(const QString& path) const
{
    for (const auto& row : snapshot().rows) {
        if (row.path == path) {
            return row;
        }
    }
    return {};
}

void LayersPanelBundleTest::groupInsertsAtTopWithoutSelection()
{
    QVERIFY(newDoc(QStringLiteral("GroupTop")));
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QCOMPARE(pictura::layers_api::rowCount(*view_), 2);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, group).kind, QStringLiteral("group"));
}

void LayersPanelBundleTest::duplicateInsertsAboveAndSelects()
{
    QVERIFY(newDoc(QStringLiteral("DupAbove")));
    const QString a = makeLayer(QStringLiteral("A"));
    const QString b = makeLayer(QStringLiteral("B"));
    QVERIFY(!a.isEmpty() && !b.isEmpty());
    pictura::select_layer_path(*view_, a);
    panel_->duplicateSelection();
    const QStringList selected = pictura::selected_layer_paths(*view_);
    QVERIFY(!selected.isEmpty());
    QCOMPARE(selected.last(), QStringLiteral("2"));
    QCOMPARE(pictura::layers_api::rowForPath(*view_, QStringLiteral("2")).name,
             QStringLiteral("A copy"));
}

void LayersPanelBundleTest::deleteRemovesTheLayer()
{
    QVERIFY(newDoc(QStringLiteral("Delete")));
    const QString path = makeLayer(QStringLiteral("Gone"));
    pictura::select_layer_path(*view_, path);
    const int before = pictura::layers_api::rowCount(*view_);
    panel_->deleteSelection();
    QCOMPARE(pictura::layers_api::rowCount(*view_), before - 1);
}

void LayersPanelBundleTest::duplicateBackgroundMakesOrdinaryLayer()
{
    QVERIFY(newDoc(QStringLiteral("BgDup")));
    pictura::select_layer_path(*view_, QStringLiteral("0"));
    panel_->duplicateSelection();
    const auto copy = pictura::layers_api::rowForPath(*view_, QStringLiteral("1"));
    QCOMPARE(copy.kind, QStringLiteral("pixel"));
    QCOMPARE(copy.name,
             pictura::layers_api::rowForPath(*view_, QStringLiteral("0")).name
                 + QStringLiteral(" copy"));
}

void LayersPanelBundleTest::adjustmentInsertsAboveActive()
{
    QVERIFY(newDoc(QStringLiteral("AdjAnchor")));
    QVERIFY(!makeLayer(QStringLiteral("A")).isEmpty());
    const QString middle = makeLayer(QStringLiteral("Middle"));
    QVERIFY(!middle.isEmpty());
    QVERIFY(!makeLayer(QStringLiteral("C")).isEmpty());
    pictura::select_layer_path(*view_, middle);
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    const QString path = pictura::selected_layer_paths(*view_).last();
    const auto created = snapshotRow(path);
    QVERIFY2(!created.path.isEmpty(), "the created adjustment row exists");
    QCOMPARE(created.kind, QStringLiteral("adjustment"));
    QCOMPARE(created.name, QStringLiteral("Invert"));
    QVERIFY2(created.hasMask, "an adjustment gets its default mask");
    QCOMPARE(snapshot().maskTargetPath, created.path);
}

void LayersPanelBundleTest::anchoredInsertionClearsLinks()
{
    QVERIFY(newDoc(QStringLiteral("AnchorLinks")));
    const QString a = makeLayer(QStringLiteral("A"));
    const QString b = makeLayer(QStringLiteral("B"));
    QVERIFY(!a.isEmpty() && !b.isEmpty());
    QVERIFY(view_->link_layers(QStringList{a, b}, true) == 2);
    pictura::select_layer_path(*view_, a);
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    // The insertion shifts B's path, so the old path-keyed link set must not
    // survive and silently link a different pair.
    for (const auto& row : snapshot().rows) {
        if (row.name == QStringLiteral("A") || row.name == QStringLiteral("B")) {
            QVERIFY2(!row.linked, qPrintable(row.name));
        }
    }
}

void LayersPanelBundleTest::fillInsertsAboveActive()
{
    QVERIFY(newDoc(QStringLiteral("FillAnchor")));
    QVERIFY(!makeLayer(QStringLiteral("A")).isEmpty());
    const QString middle = makeLayer(QStringLiteral("Middle"));
    pictura::select_layer_path(*view_, middle);
    const QString created = view_->add_solid_fill(0xffff0000u);
    QCOMPARE(created, QStringLiteral("3"));
    QCOMPARE(pictura::layers_api::rowForPath(*view_, created).kind,
             QStringLiteral("adjustment"));
    QVERIFY(pictura::selected_layer_paths(*view_).contains(created));
}

void LayersPanelBundleTest::fillMaskFromSelection()
{
    QVERIFY(newDoc(QStringLiteral("FillSelMask")));
    view_->select_all();
    panel_->setOptionFlagsForTest(true, true);
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!fill.isEmpty());
    QVERIFY(pictura::layers_api::rowForPath(*view_, fill).hasMask);
}

void LayersPanelBundleTest::fillMaskWhiteWithoutSelection()
{
    QVERIFY(newDoc(QStringLiteral("FillWhiteMask")));
    view_->deselect();
    panel_->setOptionFlagsForTest(true, true);
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!fill.isEmpty());
    const auto row = pictura::layers_api::rowForPath(*view_, fill);
    QVERIFY(row.hasMask);
    QVERIFY2(!snapshotRow(fill).maskThumbnail.isNull(), "a white mask has a thumbnail");
}

void LayersPanelBundleTest::fillMaskOff()
{
    QVERIFY(newDoc(QStringLiteral("FillNoMask")));
    view_->select_all();
    panel_->setOptionFlagsForTest(true, false);
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!fill.isEmpty());
    QVERIFY(!pictura::layers_api::rowForPath(*view_, fill).hasMask);
}

void LayersPanelBundleTest::fillClippingFromBridge()
{
    QVERIFY(newDoc(QStringLiteral("FillClip")));
    const QString fill = pictura::add_fill_layer(
        *view_, QStringLiteral("solid-color"), QStringLiteral("00ff00"), QString(),
        QString(), 0, QStringLiteral("norm"), 255, true);
    QVERIFY(!fill.isEmpty());
    QVERIFY2(pictura::layers_api::rowForPath(*view_, fill).clipping,
             "the New Layer dialog's clipping option reaches the fill");
}

void LayersPanelBundleTest::fillThumbnailPaintsPreview()
{
    QVERIFY(newDoc(QStringLiteral("FillPaint")));
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!fill.isEmpty());
    panel_->refresh();
    QTest::qWait(0);
    const QImage row = panel_->rowImageForTest(fill);
    const QRect thumb = panel_->rowThumbRectForTest(fill);
    QVERIFY(!row.isNull() && !thumb.isEmpty());
    const QColor center = row.pixelColor(thumb.center());
    QVERIFY2(center.green() > 200 && center.red() < 60,
             "a fill row paints its content preview, not the adjustment icon");
}

void LayersPanelBundleTest::copyToggleOffKeepsName()
{
    QVERIFY(newDoc(QStringLiteral("CopyOff")));
    const QString path = makeLayer(QStringLiteral("Base"));
    panel_->setOptionFlagsForTest(false, true);
    pictura::select_layer_path(*view_, path);
    panel_->duplicateSelection();
    const QString copy = pictura::selected_layer_paths(*view_).last();
    QCOMPARE(pictura::layers_api::rowForPath(*view_, copy).name, QStringLiteral("Base"));
}

void LayersPanelBundleTest::addMaskByDefaultToggles()
{
    QVERIFY(newDoc(QStringLiteral("MaskToggle")));
    QVERIFY(panel_->addMaskByDefault());
    QVERIFY(panel_->performPanelMenuAction(QStringLiteral("addMaskByDefault")));
    QVERIFY(!panel_->addMaskByDefault());
    view_->deselect();
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    const QString offPath = pictura::selected_layer_paths(*view_).last();
    QVERIFY(!pictura::layers_api::rowForPath(*view_, offPath).hasMask);
    QVERIFY(panel_->performPanelMenuAction(QStringLiteral("addMaskByDefault")));
    QVERIFY(panel_->addMaskByDefault());
}

void LayersPanelBundleTest::adjustmentMaskFromSelection()
{
    QVERIFY(newDoc(QStringLiteral("AdjSelMask")));
    view_->select_all();
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    const QString path = pictura::selected_layer_paths(*view_).last();
    const auto row = pictura::layers_api::rowForPath(*view_, path);
    QVERIFY(row.hasMask);
    QCOMPARE(snapshot().maskTargetPath, row.path);
}

void LayersPanelBundleTest::adjustmentMaskWhite()
{
    QVERIFY(newDoc(QStringLiteral("AdjWhiteMask")));
    view_->deselect();
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    const QString path = pictura::selected_layer_paths(*view_).last();
    const auto row = pictura::layers_api::rowForPath(*view_, path);
    QVERIFY(row.hasMask);
    QVERIFY(!snapshotRow(path).maskThumbnail.isNull());
    QCOMPARE(snapshot().maskTargetPath, row.path);
}

void LayersPanelBundleTest::adjustmentMaskOff()
{
    QVERIFY(newDoc(QStringLiteral("AdjNoMask")));
    QVERIFY(panel_->performPanelMenuAction(QStringLiteral("addMaskByDefault")));
    view_->deselect();
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    const QString offPath = pictura::selected_layer_paths(*view_).last();
    QVERIFY(!pictura::layers_api::rowForPath(*view_, offPath).hasMask);
    QVERIFY(snapshot().maskTargetPath.isEmpty());
    QVERIFY(panel_->performPanelMenuAction(QStringLiteral("addMaskByDefault")));
}

// #896: the rename target is the rendered label, not the empty rest of the row.
void LayersPanelBundleTest::renameHitAreaMatchesRenderedLabel()
{
    QVERIFY(newDoc(QStringLiteral("RenameHit")));
    const QString layer = makeLayer(QStringLiteral("A"));
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::layer_style_set(*view_, layer, QStringLiteral("colorOverlay.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    QCoreApplication::processEvents();
    panel_->refresh();

    const QRect name = panel_->rowNameRectForTest(layer);
    const QRect row = panel_->rowViewportRectForTest(layer);
    QVERIFY2(name.width() > 0, "the rendered label has a hit width");
    QVERIFY2(name.width() < row.width() / 2, "the hit rect stops at the rendered label");

    QVERIFY2(panel_->doubleClickAtForTest(layer, true),
             "a double-click inside the rendered label opens the editor");
    QVERIFY2(!panel_->doubleClickAtForTest(layer, false),
             "a double-click on the eye opens no editor");

    bool styleOpened = false;
    panel_->setDialogDriverForTest([&styleOpened](QDialog& dialog) {
        styleOpened = qobject_cast<pictura::LayerStyleDialog*>(&dialog) != nullptr;
        return false;
    });
    QVERIFY2(panel_->doubleClickRightOfNameForTest(layer),
             "a double-click right of the label reaches the row");
    panel_->setDialogDriverForTest(nullptr);
    QVERIFY2(styleOpened, "a double-click right of the label opens Layer Style");
    QVERIFY2(!panel_->inlineEditorOpenForTest(),
             "a double-click right of the label opens no inline editor");
}

// #1533: a name that fits its label band renders in full for every row kind.
// The row elided the label a second time using the first elision's width, so a
// fitting name lost its tail and showed an ellipsis despite the row having room
// to spare.
void LayersPanelBundleTest::shortLayerNameIsNotElided()
{
    QVERIFY(newDoc(QStringLiteral("NameFit")));
    const QString layer = makeLayer(QStringLiteral("r1 r2"));
    QVERIFY(!layer.isEmpty());
    // Lay the window out so the panel is wide enough to show the full names.
    window_->resize(1280, 800);
    window_->show();
    QTest::qWait(0);
    panel_->selectPaths({}, QString());
    QCoreApplication::processEvents();
    panel_->refresh();

    auto* tree = panel_->findChild<QTreeView*>();
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(tree && delegate);
    // Paint reads option.font from the view; pin both to the app font so the
    // reference below uses the delegate's own font.
    tree->setFont(QApplication::font());
    QCoreApplication::processEvents();
    const QModelIndex index = rowIndexForPath(tree->model(), layer);
    QVERIFY(index.isValid());

    const QRect vr = tree->visualRect(index);
    const QRect name = delegate->nameRect(QRect(0, 0, vr.width(), vr.height()), index);
    QVERIFY2(name.width() > 0, "the row has a name band");
    const QImage row = panel_->rowImageForTest(layer);
    QVERIFY(!row.isNull());

    // Repaint the full name exactly as the delegate should: same surface,
    // font, pen, and left-aligned band. The old double elision drew "r1 …",
    // so its trailing glyphs differ from this reference.
    QImage expected(row.size(), row.format());
    expected.setDevicePixelRatio(row.devicePixelRatio());
    expected.fill(pictura::Theme::contrastShade(QColor(128, 128, 128), 1));
    QPainter painter(&expected);
    painter.setFont(delegate->nameFont(QApplication::font(), index));
    painter.setPen(tree->palette().color(QPalette::Text));
    painter.drawText(name, Qt::AlignVCenter | Qt::AlignLeft, QStringLiteral("r1 r2"));
    painter.end();

    // Compare the name band, excluding the row's bottom rule.
    const QRect band(name.left(), name.top(), name.width(), qMax(1, name.height() - 2));
    QCOMPARE(differingPixels(row.copy(band), expected.copy(band)), 0);

    // The Background (lock badge), a group (folder glyph), and a styled row
    // (fx + list toggles) each use the band their own badges leave free.
    QVERIFY(view_->background_from_layer(QStringLiteral("0")));
    QVERIFY(view_->set_layer_name_path(QStringLiteral("0"), QStringLiteral("Background")));
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QVERIFY(view_->set_layer_name_path(group, QStringLiteral("Group Name")));
    const QString styled = makeLayer(QStringLiteral("Styled"));
    QVERIFY(!styled.isEmpty());
    QVERIFY(pictura::layer_style_set(*view_, styled, QStringLiteral("colorOverlay.on"), 1.0));
    pictura::layer_style_commit(*view_, QStringLiteral("Layer Style"));
    QCoreApplication::processEvents();
    panel_->refresh();
    QCOMPARE(panel_->rowNameTextForTest(QStringLiteral("0")), QStringLiteral("Background"));
    QCOMPARE(panel_->rowNameTextForTest(group), QStringLiteral("Group Name"));
    QCOMPARE(panel_->rowNameTextForTest(styled), QStringLiteral("Styled"));

    // A name too wide for the band elides in the middle, keeping both ends.
    const QString longName =
        QStringLiteral("start") + QString(200, QLatin1Char('W')) + QStringLiteral("end");
    QVERIFY(view_->set_layer_name_path(layer, longName));
    panel_->refresh();
    const QString shown = panel_->rowNameTextForTest(layer);
    QVERIFY2(shown.size() < longName.size(), "an overlong name still elides");
    QVERIFY2(shown.startsWith(QStringLiteral("start")), "the start is kept");
    QVERIFY2(shown.endsWith(QStringLiteral("end")), "the end is kept");
    QVERIFY2(shown.contains(QChar(0x2026)), "the middle carries an ellipsis");
}

QTEST_MAIN(LayersPanelBundleTest)
#include "tst_layers_panel_bundle.moc"
