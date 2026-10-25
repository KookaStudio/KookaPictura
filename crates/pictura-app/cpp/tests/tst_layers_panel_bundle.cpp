// The anchored creation, default-mask, and mask-target fixes for the Layers
// panel bundle (#898, #895a, #897 plumbing).

#include <QtTest/QtTest>

#include <QtCore/QRect>
#include <QtGui/QColor>
#include <QtGui/QImage>

#include "api/layers_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/fill_content.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"

#include "frame.h"
#include "panels/layers_panel.h"
#include "qt_test_support.h"

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
    QTest::qWait(20);
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

QTEST_MAIN(LayersPanelBundleTest)
#include "tst_layers_panel_bundle.moc"
