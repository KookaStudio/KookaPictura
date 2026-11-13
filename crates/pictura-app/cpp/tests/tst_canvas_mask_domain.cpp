#include <QtTest/QtTest>
#include <limits>
#include <array>
#include <QtWidgets/QTreeView>
#include <QtCore/QDataStream>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtCore/QScopeGuard>
#include <QtWidgets/QToolButton>

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/layers_api.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/selection_bridge.h"
#include "api/type_api.h"
#include "api/transform_api.h"
#include "api/transform_bridge.h"
#include "frame.h"
#include "memory_refusal.h"
#include "panels/color_panel.h"
#include "panels/properties_panel.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "qt_test_support.h"

namespace {
// A generated PSD with independent layer/mask geometry, including an optional
// mask header without a channel image. No application creation command can
// produce these legacy import states.
QByteArray importedMaskPsd(bool missing, int outside, quint8 exterior, int width, int height, bool uniformMask)
{
    QByteArray mask;
    QDataStream ms(&mask, QIODevice::WriteOnly);
    ms << qint32(1) << qint32(2 + outside) << qint32(3) << qint32(5 + outside)
       << exterior << quint8(1) << quint16(0);
    QByteArray extra;
    QDataStream es(&extra, QIODevice::WriteOnly);
    es << quint32(mask.size());
    es.writeRawData(mask.constData(), mask.size());
    es << quint32(0);
    es.writeRawData("\x01x\0\0", 4);
    QByteArray record;
    QDataStream rs(&record, QIODevice::WriteOnly);
    rs << qint32(1) << qint32(2) << qint32(3) << qint32(5) << quint16(missing ? 4 : 5);
    for (qint16 id : {qint16(0), qint16(1), qint16(2), qint16(-1)}) {
        rs << id << quint32(8);
    }
    if (!missing) { rs << qint16(-2) << quint32(8); }
    rs.writeRawData("8BIMnorm", 8);
    rs << quint8(255) << quint8(0) << quint8(0) << quint8(0) << quint32(extra.size());
    rs.writeRawData(extra.constData(), extra.size());
    QByteArray layers;
    QDataStream ls(&layers, QIODevice::WriteOnly);
    ls << qint16(1);
    ls.writeRawData(record.constData(), record.size());
    for (quint8 value : {quint8(32), quint8(64), quint8(192), quint8(255)}) {
        ls << quint16(0);
        ls.writeRawData(QByteArray(6, char(value)).constData(), 6);
    }
    if (!missing) {
        ls << quint16(0);
        const char values[] {0, 1, 64, char(128), char(254), char(255)};
        if (uniformMask) {ls.writeRawData(QByteArray(6,char(exterior)).constData(),6);}
        else {ls.writeRawData(values, 6);}
    }
    if (layers.size() % 2) { ls << quint8(0); }
    QByteArray section;
    QDataStream ss(&section, QIODevice::WriteOnly);
    ss << quint32(layers.size());
    ss.writeRawData(layers.constData(), layers.size());
    ss << quint32(0);
    QByteArray psd;
    QDataStream out(&psd, QIODevice::WriteOnly);
    out.writeRawData("8BPS", 4);
    out << quint16(1);
    out.writeRawData(QByteArray(6, '\0').constData(), 6);
    out << quint16(3) << quint32(height) << quint32(width) << quint16(8) << quint16(3)
        << quint32(0) << quint32(0) << quint32(section.size());
    out.writeRawData(section.constData(), section.size());
    out << quint16(0);
    out.writeRawData(QByteArray(width * height * 3, '\0').constData(), width * height * 3);
    return psd;
}

std::array<QRect,3> thumbCards(pictura::LayersPanel& panel,const QString& path)
{
    auto* tree = panel.findChild<QTreeView*>();
    if (!tree || !tree->model() || !tree->model()->rowCount()) {return {};}
    const auto matches = tree->model()->match(tree->model()->index(0,0),pictura::PathRole,path,1,Qt::MatchExactly|Qt::MatchRecursive);
    if (matches.isEmpty()) {return {};}
    const auto index = matches.front();
    const QRect vr = tree->visualRect(index);
    const QRect row(0,0,vr.width(),vr.height());
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel.itemDelegateForTest());
    return {delegate->thumbRect(row,index),delegate->maskThumbRect(row,index),delegate->vectorMaskThumbRect(row,index)};
}

QByteArray coverage(const pictura::PictureView& view)
{
    int width = 0, height = 0;
    const auto samples = pictura::channels_active_mask_thumbnail(view, 12, width, height);
    if (width != 12 || height != 8) { return {}; }
    return QByteArray(reinterpret_cast<const char*>(samples.data()), qsizetype(samples.size()));
}

bool dab(pictura::PictureView& view, quint32 color = 0xffffffffu)
{
    return view.begin_paint(color, 0xff000000u, 3, 100, 100, 0, 100, 100, 25,
                           QStringLiteral("normal"), false, false)
        && view.paint_dab(9.0, 6.0, 1.0) && view.end_paint();
}
} // namespace

class CanvasMaskDomainTest : public QObject {
    Q_OBJECT
private slots:
    void initTestCase();
    void init();
    void cleanup();
    void allMaskCardsFollowDocumentAspect();
    void layerRowClickDefaultsToContent();
    void adjustmentCreationActivatesMask();
    void importedPortraitMaskCardIsOpaque();
    void maskThumbnailUsesOnlyBrackets();
    void maskBracketsFollowRasterAndLayerClicks();
    void maskModifierIndicatorsRemain();
    void createdMaskBecomesTarget();
    void creationVariantsKeepDefaults();
    void maskTargetLifecycle();
    void canvasMaskBasicEdits();
    void layerEditingWithoutMaskTarget();
    void importedMaskCanvasTools();
    void importedMaskFillTools();
    void importedMaskFilters();
    void importedMaskReadOnlyViews();
    void dataLessImportFirstEditPreservesViews();
    void canvasMaskEditUndoRedo();
    void soloMaskRestoreShowsTheMask();
    void canvasMaskCancelAndNoop();
    void canvasMaskRefusalIsAtomic();
    void wholeMaskMoveBlackPreview();
    void maskMoveRouting();
    void selectedMaskMoveRoutesCoverage();
    void selectedMaskMoveBlackBackground();
    void selectedMaskAltRetainsCoverage();
    void selectedMaskCtrlRoutesCoverage();
    void selectedMaskMoveHistory();
    void propertiesMaskRefinementUndo();
    void propertiesInvertPreservesOffcanvasCoverage();
    void maskThumbCompanions();
    void vectorOnlyMaskThumbCompanion();
    void inactiveLayerThumbClicks();
    void otherLayerClearsThumbSelection();
private:
    pictura::PictureView* imported(bool missing = false, int outside = 0, quint8 exterior = 128, int width = 12, int height = 8, bool uniformMask = false);
    pictura::PictureView* maskedLayer(const QString& kind,const QSize& canvas);
    pictura::test::ScopedStateHome state_;
    QTemporaryDir files_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CanvasMaskDomainTest::initTestCase()
{
    QVERIFY(state_.isValid());
    QVERIFY(files_.isValid());
    window_ = pictura::test::makeMainWindow();
}
void CanvasMaskDomainTest::init()
{
    while (window_->documentCount()) { QVERIFY(window_->closeDocument(0, false)); }
}
void CanvasMaskDomainTest::cleanup() { pictura::set_available_memory_for_test(-1); }

pictura::PictureView* CanvasMaskDomainTest::imported(bool missing, int outside, quint8 exterior, int width, int height, bool uniformMask)
{
    const QString path = files_.filePath(QStringLiteral("mask.psd"));
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) { return nullptr; }
    const QByteArray bytes = importedMaskPsd(missing, outside, exterior, width, height, uniformMask);
    if (file.write(bytes) != bytes.size()) { return nullptr; }
    file.close();
    if (!window_->openPath(path)) { return nullptr; }
    auto* view = window_->activeView();
    if (!view) { return nullptr; }
    view->set_gpu_compute(false);
    window_->selectLayerPath(QStringLiteral("0"));
    if (!pictura::mask_edit_target_set(*view, QStringLiteral("0"))) { return nullptr; }
    return view;
}

pictura::PictureView* CanvasMaskDomainTest::maskedLayer(const QString& kind,const QSize& canvas)
{
    while (window_->documentCount()) {if (!window_->closeDocument(0,false)) {return nullptr;}}
    const QString file=QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/stroke_pattern.psd");
    if (file.isEmpty() || !window_->openPath(file)) {return nullptr;}
    auto* v=window_->activeView();if (!v) {return nullptr;}
    v->set_gpu_compute(false);
    if (!v->resize_canvas(QStringLiteral("center"),canvas.width(),canvas.height())) {return nullptr;}
    QString path=v->active_layer_path();
    if (kind==QStringLiteral("group")) {path=v->add_group_in(QString());}
    else if (kind==QStringLiteral("adjustment")) {if (!v->add_adjustment(QStringLiteral("invert"))) {return nullptr;}path=v->active_layer_path();}
    else if (kind==QStringLiteral("solid")) {path=v->add_solid_fill(0xff4080c0u);}
    else if (kind==QStringLiteral("gradient")) {path=v->add_gradient_fill();}
    else if (kind==QStringLiteral("pattern")) {
        const auto ids=pictura::available_pattern_ids(*v);if (ids.isEmpty()) {return nullptr;}
        path=pictura::add_fill_layer(*v,QStringLiteral("pattern"),ids.first()+QStringLiteral("|100|1|0|0"),path,QString(),0,QStringLiteral("norm"),255,false);
    } else if (kind==QStringLiteral("type")) {
        pictura::TypeSetting setting{};setting.x=5;setting.y=25;setting.xx=1;setting.yy=1;
        auto character=pictura::type_api::defaultCharacterSetting();character.size=24;
        path=pictura::type_api::commitLayer(*v,{QStringLiteral("Mask"),QStringLiteral("Liberation Sans"),setting,character,pictura::type_api::defaultParagraphSetting()});
    }
    if (path.isEmpty()) {return nullptr;}
    window_->selectLayerPath(path);
    if (!pictura::layer_mask_present(*v) && !pictura::layer_mask_add(*v,QStringLiteral("reveal-all"))) {return nullptr;}
    const bool fill=kind==QStringLiteral("solid") || kind==QStringLiteral("gradient") || kind==QStringLiteral("pattern");
    if (!fill && !pictura::vector_mask_present(*v) && !pictura::vector_mask_add(*v,QStringLiteral("reveal-all"))) {return nullptr;}
    return v;
}

void CanvasMaskDomainTest::allMaskCardsFollowDocumentAspect()
{
    window_->resize(1800,1000);window_->show();
    for (const QSize canvas : {QSize(120,400),QSize(400,120)}) {
        for (const QString& kind : {QStringLiteral("pixel"),QStringLiteral("group"),QStringLiteral("adjustment"),QStringLiteral("solid"),QStringLiteral("gradient"),QStringLiteral("pattern"),QStringLiteral("type")}) {
            auto* v=maskedLayer(kind,canvas);QVERIFY2(v,qPrintable(kind));
            const QString path=v->active_layer_path();
            auto* panel=window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));QVERIFY(panel);
            panel->setView(v);panel->refresh();QCoreApplication::processEvents();
            const auto cards=thumbCards(*panel,path);
            QVERIFY2(!cards[1].isEmpty(),qPrintable(kind));
            const bool fill=kind==QStringLiteral("solid") || kind==QStringLiteral("gradient") || kind==QStringLiteral("pattern");
            QCOMPARE(cards[2].isEmpty(),fill);
            for (int i=1;i<=(fill?1:2);++i) {
                const QRect card=cards[i];QVERIFY2(!card.isEmpty(),qPrintable(kind));
                QVERIFY2(qAbs(card.width()*canvas.height()-card.height()*canvas.width())<=qMax(canvas.width(),canvas.height())/2,qPrintable(kind));
            }
            if (!fill) {QCOMPARE(cards[1].size(),cards[2].size());QVERIFY(cards[2].left()>cards[1].right());}
            QVERIFY(cards[1].left()>cards[0].right());
            QVERIFY(!panel->rowLinkGlyphRectForTest(path).isEmpty());
            if (kind!=QStringLiteral("pixel") && kind!=QStringLiteral("group")) {QCOMPARE(cards[0].width(),cards[0].height());}
            const QImage painted=panel->rowImageForTest(path);
            QCOMPARE(painted.pixelColor(cards[1].right(),cards[1].center().y()),QColor(Qt::black));
            if (fill || kind==QStringLiteral("adjustment")) {QCOMPARE(painted.pixelColor(cards[1].right()-2,cards[1].center().y()),QColor(Qt::white));}
            QVERIFY(panel->clickMaskThumbnailForTest(path));QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);
            QCOMPARE(pictura::mask_edit_target_get(*v),path);
        }
    }
}

void CanvasMaskDomainTest::layerRowClickDefaultsToContent()
{
    window_->resize(1800,1000);window_->show();
    for (const QString& kind : {QStringLiteral("pixel"),QStringLiteral("group"),QStringLiteral("adjustment"),QStringLiteral("solid"),QStringLiteral("gradient"),QStringLiteral("pattern"),QStringLiteral("type")}) {
        auto* v=maskedLayer(kind,QSize(120,400));QVERIFY2(v,qPrintable(kind));
        const QString path=v->active_layer_path();
        auto* panel=window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));QVERIFY(panel);
        panel->setView(v);panel->refresh();QCoreApplication::processEvents();
        QVERIFY(panel->altClickMaskThumbnailForTest(path));QCOMPARE(pictura::mask_solo_get(*v),path);
        int history=v->history_index();
        QVERIFY(panel->clickRowForTest(path));QCoreApplication::processEvents();
        QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbContent);
        QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());QVERIFY(pictura::mask_solo_get(*v).isEmpty());QCOMPARE(v->history_index(),history);
        QVERIFY(panel->clickMaskThumbnailForTest(path));QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);QCOMPARE(pictura::mask_edit_target_get(*v),path);
        const auto cards=thumbCards(*panel,path);auto* tree=panel->findChild<QTreeView*>();QVERIFY(tree);
        if (!cards[2].isEmpty()) {
            QTest::mouseClick(tree->viewport(),Qt::LeftButton,Qt::NoModifier,panel->rowViewportRectForTest(path).topLeft()+cards[2].center());
            QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask|pictura::ActiveThumbVector);
            QCOMPARE(pictura::mask_edit_target_get(*v),path);
        }
        const QString other=v->add_layer_in(QString());QVERIFY(!other.isEmpty());panel->refresh();QCoreApplication::processEvents();
        history=v->history_index();QVERIFY(panel->clickRowForTest(path));QCoreApplication::processEvents();
        QCOMPARE(v->active_layer_path(),path);QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbContent);QCOMPARE(v->history_index(),history);
    }
}

void CanvasMaskDomainTest::adjustmentCreationActivatesMask()
{
    auto* v=maskedLayer(QStringLiteral("pixel"),QSize(120,400));QVERIFY(v);
    auto* panel=window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));QVERIFY(panel);
    window_->show();panel->setView(v);panel->refresh();QCoreApplication::processEvents();
    QVERIFY(panel->clickRowForTest(v->active_layer_path()));
    for (const QString& kind : {QStringLiteral("invert"),QStringLiteral("levels")}) {
        const int history=v->history_index();QVERIFY(v->add_adjustment(kind));
        const QString path=v->active_layer_path();QVERIFY(!path.isEmpty());panel->refresh();QCoreApplication::processEvents();
        QCOMPARE(panel->selectedPaths(),QStringList{path});QCOMPARE(pictura::mask_edit_target_get(*v),path);QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);QCOMPARE(v->history_index(),history+1);
        QVERIFY(panel->clickRowForTest(path));QCoreApplication::processEvents();QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbContent);QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());QCOMPARE(v->history_index(),history+1);
    }
}

void CanvasMaskDomainTest::importedPortraitMaskCardIsOpaque()
{
    window_->resize(1600,1000); window_->show();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel->itemDelegateForTest()); QVERIFY(delegate);
    const int originalSize = delegate->thumbnailSize();
    const auto resetSize = qScopeGuard([&] {delegate->setThumbnailSize(originalSize);panel->refresh();});
    for (const QSize document : {QSize(120,400),QSize(343,1000),QSize(400,120)}) {
        while (window_->documentCount()) {QVERIFY(window_->closeDocument(0,false));}
        auto* v = imported(false,0,128,document.width(),document.height(),true); QVERIFY(v);
        QVERIFY(pictura::mask_edit_target_set(*v,QString()));
        for (int size : {16,24,40}) {
            delegate->setThumbnailSize(size); panel->refresh(); QCoreApplication::processEvents();
            const auto snap = pictura::layers_api::snapshot(*v,size,true);
            QVERIFY(!snap.rows.empty());
            const QImage source = snap.rows[0].maskThumbnail;
            QVERIFY(!source.isNull());
            for (int y=0;y<source.height();++y) for (int x=0;x<source.width();++x) QCOMPARE(source.pixelColor(x,y),QColor(128,128,128));
            const QRect card = thumbCards(*panel,QStringLiteral("0"))[1]; QVERIFY(!card.isEmpty());
            for (qreal dpr : {1.0,1.5,2.0}) {
                const QImage painted = panel->rowImageForTest(QStringLiteral("0"),dpr);
                QVERIFY(!painted.isNull());
                QCOMPARE(painted.pixelColor(qFloor((card.center().x()+0.5)*dpr),qFloor((card.center().y()+0.5)*dpr)),QColor(128,128,128));
                for (int y=card.top()+1;y<card.bottom();++y) for (int x=card.left()+1;x<card.right();++x) {
                    const QColor pixel = painted.pixelColor(qFloor((x+0.5)*dpr),qFloor((y+0.5)*dpr));
                    // At fractional scales a one-pixel pen can cover the
                    // sampled half of the logical pixel beside the outline.
                    const bool besideOutline = x==card.left()+1 || x==card.right()-1 || y==card.top()+1 || y==card.bottom()-1;
                    QVERIFY2(pixel==QColor(128,128,128) || (besideOutline && pixel==QColor(Qt::black)),qPrintable(QStringLiteral("doc %1x%2 thumb %3 dpr %4 card %5,%6 %7x%8 pixel %9,%10").arg(document.width()).arg(document.height()).arg(size).arg(dpr).arg(card.x()).arg(card.y()).arg(card.width()).arg(card.height()).arg(x).arg(y)));
                }
            }
            const QImage painted = panel->rowImageForTest(QStringLiteral("0"));
            QCOMPARE(painted.pixelColor(card.right(),card.center().y()),QColor(Qt::black));
        }
    }
}

void CanvasMaskDomainTest::maskThumbnailUsesOnlyBrackets()
{
    auto* v = imported(false,0,128,120,400); QVERIFY(v);
    const QString group = v->add_group_in(QString()); QVERIFY(!group.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*v,group,QStringLiteral("reveal-all")));
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    window_->show();
    for (const QString& path : {QStringLiteral("0"),group}) {
        window_->selectLayerPath(path);panel->refresh();QCoreApplication::processEvents();
        const int history = v->history_index();
        QVERIFY(panel->clickMaskThumbnailForTest(path));
        QCOMPARE(pictura::mask_edit_target_get(*v),path);
        QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);
        const auto cards = thumbCards(*panel,path);
        const QRect card = cards[1]; QVERIFY(!card.isEmpty());
        const QImage row = panel->rowImageForTest(path);
        QCOMPARE(row.pixelColor(card.center().x(),card.top()),QColor(Qt::black));
        QCOMPARE(row.pixelColor(card.right(),card.center().y()),QColor(Qt::black));
        QCOMPARE(row.pixelColor(card.left()-1,card.top()-1),QColor(Qt::white));
        if (path==group) {QVERIFY(row.pixelColor(cards[0].left()-1,cards[0].top()-1)!=QColor(Qt::white));}
        QCOMPARE(v->history_index(),history);
    }
}

void CanvasMaskDomainTest::maskBracketsFollowRasterAndLayerClicks()
{
    auto* v = imported(false,0,128); QVERIFY(v);
    QVERIFY(pictura::mask_edit_target_set(*v,QString()));
    const QString other = v->add_layer_in(QString()); QVERIFY(!other.isEmpty());
    QVERIFY(pictura::edit_fill(*v,0xff808080u,-1,QStringLiteral("normal"),100,false));
    window_->selectLayerPath(QStringLiteral("0"));
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    window_->show(); panel->refresh(); QCoreApplication::processEvents();
    QVERIFY(panel->clickMaskThumbnailForTest(QStringLiteral("0")));
    QVERIFY(panel->clickLayerThumbnailForTest(QStringLiteral("0")));
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
    QCOMPARE(panel->rowActiveThumbForTest(QStringLiteral("0")),pictura::ActiveThumbContent);
    auto cards = thumbCards(*panel,QStringLiteral("0"));
    QImage row = panel->rowImageForTest(QStringLiteral("0"));
    QCOMPARE(row.pixelColor(cards[1].center().x(),cards[1].top()),QColor(Qt::black));
    QVERIFY(row.pixelColor(cards[1].left()-1,cards[1].top()-1)!=QColor(Qt::white));
    QCOMPARE(row.pixelColor(cards[0].left()-1,cards[0].top()-1),QColor(Qt::white));
    QVERIFY(panel->clickMaskThumbnailForTest(QStringLiteral("0")));
    QVERIFY(panel->clickLayerThumbnailForTest(other)); QCoreApplication::processEvents();
    QCOMPARE(v->active_layer_path(),other);
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
    cards = thumbCards(*panel,QStringLiteral("0")); row = panel->rowImageForTest(QStringLiteral("0"));
    QCOMPARE(row.pixelColor(cards[1].center().x(),cards[1].top()),QColor(Qt::black));
    QVERIFY(row.pixelColor(cards[1].left()-1,cards[1].top()-1)!=QColor(Qt::white));
    cards = thumbCards(*panel,other);row=panel->rowImageForTest(other);
    QCOMPARE(row.pixelColor(cards[0].left()-1,cards[0].top()-1),QColor(Qt::white));
}

void CanvasMaskDomainTest::maskModifierIndicatorsRemain()
{
    auto* v=imported();QVERIFY(v);
    QVERIFY(pictura::layer_mask_set_linked(*v,true));
    auto* panel=window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));QVERIFY(panel);
    window_->show();panel->refresh();QCoreApplication::processEvents();
    const QString path=QStringLiteral("0");
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);
    QVERIFY(panel->rowMaskLinkedForTest(path));
    QVERIFY(!panel->rowLinkGlyphRectForTest(path).isEmpty());
    int history=v->history_index();
    QVERIFY(panel->clickLinkGlyphForTest(path));QTRY_VERIFY(!panel->rowMaskLinkedForTest(path));QCOMPARE(v->history_index(),history+1);
    history=v->history_index();
    QVERIFY(panel->shiftClickMaskThumbnailForTest(path));QTRY_VERIFY(panel->rowMaskDisabledForTest(path));QCOMPARE(v->history_index(),history+1);
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);
    auto card=thumbCards(*panel,path)[1];auto row=panel->rowImageForTest(path);
    QVERIFY(row.pixelColor(card.center()).red()>row.pixelColor(card.center()).green()+80);
    history=v->history_index();
    QVERIFY(panel->altClickMaskThumbnailForTest(path));QCOMPARE(pictura::mask_solo_get(*v),path);QCOMPARE(v->history_index(),history);
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);QCOMPARE(pictura::mask_edit_target_get(*v),path);
    QVERIFY(panel->shiftClickMaskThumbnailForTest(path));QTRY_VERIFY(!panel->rowMaskDisabledForTest(path));QCOMPARE(pictura::mask_solo_get(*v),path);
    QVERIFY(panel->altClickMaskThumbnailForTest(path));QVERIFY(pictura::mask_solo_get(*v).isEmpty());
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);QCOMPARE(pictura::mask_edit_target_get(*v),path);
    const bool disabled=panel->rowMaskDisabledForTest(path);
    QVERIFY(panel->altShiftClickMaskThumbnailForTest(path));QVERIFY(pictura::mask_solo_get(*v).isEmpty());QCOMPARE(panel->rowMaskDisabledForTest(path),disabled);
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);QCOMPARE(pictura::mask_edit_target_get(*v),path);
    QVERIFY(panel->ctrlClickMaskThumbnailForTest(path));QCoreApplication::processEvents();
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);QCOMPARE(pictura::mask_edit_target_get(*v),path);
    QVERIFY(pictura::vector_mask_add(*v,QStringLiteral("reveal-all")));panel->refresh();
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);
    QVERIFY(panel->rowVectorMaskLinkedForTest(path));
    card=thumbCards(*panel,path)[2];QVERIFY(!card.isEmpty());
    history=v->history_index();QVERIFY(panel->clickVectorLinkGlyphForTest(path));QTRY_VERIFY(!panel->rowVectorMaskLinkedForTest(path));QCOMPARE(v->history_index(),history+1);
    history=v->history_index();QVERIFY(panel->shiftClickVectorMaskThumbnailForTest(path));QTRY_VERIFY(panel->rowVectorMaskDisabledForTest(path));QCOMPARE(v->history_index(),history+1);
    QCOMPARE(panel->rowActiveThumbForTest(path),pictura::ActiveThumbMask);
    row=panel->rowImageForTest(path);QVERIFY(row.pixelColor(card.center()).red()>row.pixelColor(card.center()).green()+80);
}

void CanvasMaskDomainTest::maskThumbCompanions()
{
    auto* v = maskedLayer(QStringLiteral("pixel"), QSize(120, 400)); QVERIFY(v);
    const QString path = v->active_layer_path();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    window_->show(); panel->setView(v); panel->refresh(); QCoreApplication::processEvents();
    auto* tree = panel->findChild<QTreeView*>(); QVERIFY(tree);
    const auto vectorClick = [&] {
        const auto cards = thumbCards(*panel, path); QVERIFY(!cards[2].isEmpty());
        QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::NoModifier,
                          panel->rowViewportRectForTest(path).topLeft() + cards[2].center());
        QCoreApplication::processEvents();
    };
    QVERIFY(panel->clickLayerThumbnailForTest(path));
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbContent);
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
    QVERIFY(panel->clickMaskThumbnailForTest(path));
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbMask);
    QCOMPARE(pictura::mask_edit_target_get(*v), path);
    vectorClick();
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbMask | pictura::ActiveThumbVector);
    QCOMPARE(pictura::mask_edit_target_get(*v), path);
    auto cards = thumbCards(*panel, path); auto row = panel->rowImageForTest(path);
    QCOMPARE(row.pixelColor(cards[1].left() - 1, cards[1].top() - 1), QColor(Qt::white));
    QCOMPARE(row.pixelColor(cards[2].left() - 1, cards[2].top() - 1), QColor(Qt::white));
    QVERIFY(row.pixelColor(cards[0].left() - 1, cards[0].top() - 1) != QColor(Qt::white));
    vectorClick();
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbMask);
    QCOMPARE(pictura::mask_edit_target_get(*v), path);
    cards = thumbCards(*panel, path); row = panel->rowImageForTest(path);
    QCOMPARE(row.pixelColor(cards[1].left() - 1, cards[1].top() - 1), QColor(Qt::white));
    QVERIFY(row.pixelColor(cards[2].left() - 1, cards[2].top() - 1) != QColor(Qt::white));
    QVERIFY(panel->clickMaskThumbnailForTest(path));
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbMask);
    vectorClick();
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbMask | pictura::ActiveThumbVector);
    QVERIFY(panel->clickMaskThumbnailForTest(path));
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbMask | pictura::ActiveThumbVector);
    QVERIFY(panel->clickLayerThumbnailForTest(path));
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbContent | pictura::ActiveThumbVector);
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
    QVERIFY(panel->clickMaskThumbnailForTest(path));
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbMask | pictura::ActiveThumbVector);
    QCOMPARE(pictura::mask_edit_target_get(*v), path);
}

void CanvasMaskDomainTest::vectorOnlyMaskThumbCompanion()
{
    while (window_->documentCount()) { QVERIFY(window_->closeDocument(0, false)); }
    QVERIFY(window_->newDocument(QStringLiteral("VectorOnly"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    auto* v = window_->activeView(); QVERIFY(v);
    const QString path = v->add_layer_in(QString()); QVERIFY(!path.isEmpty());
    v->set_active_layer(path);
    QVERIFY(pictura::vector_mask_add(*v, QStringLiteral("reveal-all")));
    QVERIFY(!pictura::layer_mask_present(*v));
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    window_->show(); panel->setView(v); panel->refresh(); QCoreApplication::processEvents();
    auto* tree = panel->findChild<QTreeView*>(); QVERIFY(tree);
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbContent);
    const auto cards = thumbCards(*panel, path); QVERIFY(!cards[2].isEmpty());
    const QPoint at = panel->rowViewportRectForTest(path).topLeft() + cards[2].center();
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::NoModifier, at);
    QCoreApplication::processEvents();
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbContent | pictura::ActiveThumbVector);
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::NoModifier, at);
    QCoreApplication::processEvents();
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbContent);
}

void CanvasMaskDomainTest::inactiveLayerThumbClicks()
{
    auto* v = maskedLayer(QStringLiteral("pixel"), QSize(120, 400)); QVERIFY(v);
    const QString path = v->active_layer_path();
    const QString other = v->add_layer_in(QString()); QVERIFY(!other.isEmpty());
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    window_->show(); panel->setView(v); panel->refresh(); QCoreApplication::processEvents();
    auto* tree = panel->findChild<QTreeView*>(); QVERIFY(tree);
    const auto activateOther = [&] {
        QVERIFY(panel->clickRowForTest(other));
        QCoreApplication::processEvents();
        QCOMPARE(v->active_layer_path(), other);
    };
    const auto clickThumb = [&](int i) {
        const auto cards = thumbCards(*panel, path); QVERIFY(!cards[i].isEmpty());
        QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::NoModifier,
                          panel->rowViewportRectForTest(path).topLeft() + cards[i].center());
        QCoreApplication::processEvents();
    };
    activateOther();
    clickThumb(0);
    QCOMPARE(v->active_layer_path(), path);
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbContent);
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
    activateOther();
    clickThumb(1);
    QCOMPARE(v->active_layer_path(), path);
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbMask);
    QCOMPARE(pictura::mask_edit_target_get(*v), path);
    activateOther();
    clickThumb(2);
    QCOMPARE(v->active_layer_path(), path);
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbContent | pictura::ActiveThumbVector);
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
}

void CanvasMaskDomainTest::otherLayerClearsThumbSelection()
{
    auto* v = maskedLayer(QStringLiteral("pixel"), QSize(120, 400)); QVERIFY(v);
    const QString path = v->active_layer_path();
    const QString other = v->add_layer_in(QString()); QVERIFY(!other.isEmpty());
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    window_->show(); panel->setView(v); panel->refresh(); QCoreApplication::processEvents();
    auto* tree = panel->findChild<QTreeView*>(); QVERIFY(tree);
    QVERIFY(panel->clickMaskThumbnailForTest(path));
    const auto cards = thumbCards(*panel, path); QVERIFY(!cards[2].isEmpty());
    QTest::mouseClick(tree->viewport(), Qt::LeftButton, Qt::NoModifier,
                      panel->rowViewportRectForTest(path).topLeft() + cards[2].center());
    QCoreApplication::processEvents();
    QCOMPARE(panel->rowActiveThumbForTest(path),
             pictura::ActiveThumbMask | pictura::ActiveThumbVector);
    window_->selectLayerPath(other);
    panel->refresh(); QCoreApplication::processEvents();
    QCOMPARE(v->active_layer_path(), other);
    QVERIFY((panel->rowActiveThumbForTest(path)
             & (pictura::ActiveThumbMask | pictura::ActiveThumbVector)) == 0);
    QVERIFY(panel->clickRowForTest(path)); QCoreApplication::processEvents();
    QCOMPARE(panel->rowActiveThumbForTest(path), pictura::ActiveThumbContent);
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
}

void CanvasMaskDomainTest::createdMaskBecomesTarget()
{
    auto* v = imported(); QVERIFY(v);
    QVERIFY(pictura::layer_mask_delete(*v));
    const int history = v->history_index();
    QVERIFY(pictura::layer_mask_add(*v, QStringLiteral("reveal-all")));
    QCOMPARE(pictura::mask_edit_target_get(*v), QStringLiteral("0"));
    QCOMPARE(v->history_index(), history + 1);
    QCOMPARE(quint8(coverage(*v)[0]), quint8(0));
    QVERIFY(dab(*v));
    QCOMPARE(quint8(coverage(*v)[6 * 12 + 9]), quint8(255));
}

void CanvasMaskDomainTest::creationVariantsKeepDefaults()
{
    QVERIFY(window_->newDocument(QStringLiteral("Empty"), 12, 8, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    auto* v = window_->activeView(); QVERIFY(v);
    const QString empty = v->add_layer_in(QString());
    v->set_active_layer(empty);
    QVERIFY(pictura::layer_mask_add(*v, QStringLiteral("reveal-all")));
    QCOMPARE(coverage(*v), QByteArray(96, char(255)));
    QVERIFY(dab(*v, 0xff000000u));
    QCOMPARE(quint8(coverage(*v)[6 * 12 + 9]), quint8(0));
    const QString group = v->add_group_in(QString());
    v->set_active_layer(group);
    QVERIFY(pictura::layer_mask_add(*v, QStringLiteral("reveal-all")));
    QCOMPARE(coverage(*v), QByteArray(96, char(255)));
    QVERIFY(pictura::edit_fill(*v, 0xff000000u, -1, QStringLiteral("normal"), 100, false));
    QCOMPARE(coverage(*v), QByteArray(96, char(0)));
    const auto rows = pictura::layers_api::rows(*v);
    QVERIFY(!rows.empty());
}

void CanvasMaskDomainTest::maskTargetLifecycle()
{
    auto* v = imported(); QVERIFY(v);
    const int history = v->history_index();
    QVERIFY(pictura::mask_edit_target_set(*v, QString()));
    QVERIFY(!pictura::mask_edit_target_set(*v, QStringLiteral("99")));
    QVERIFY(pictura::mask_edit_target_set(*v, QStringLiteral("0")));
    QCOMPARE(v->history_index(), history);
    QVERIFY(pictura::layer_mask_delete(*v));
    QVERIFY(pictura::mask_edit_target_get(*v).isEmpty());
}

void CanvasMaskDomainTest::canvasMaskBasicEdits()
{
    auto* v = imported(); QVERIFY(v);
    const int base = v->history_index();
    QVERIFY(dab(*v));
    QCOMPARE(v->history_index(), base + 1);
    QCOMPARE(quint8(coverage(*v)[81]), quint8(255));
    QVERIFY(dab(*v, 0xff000000u));
    QCOMPARE(quint8(coverage(*v)[81]), quint8(0));
    QVERIFY(v->select_rect(8, 5, 3, 2, QStringLiteral("new"), 0.0));
    const int before = v->history_index();
    QVERIFY(pictura::edit_fill(*v, 0xff808080u, -1, QStringLiteral("normal"), 100, false));
    QCOMPARE(v->history_index(), before + 1);
    QCOMPARE(quint8(coverage(*v)[81]), quint8(128));
}

void CanvasMaskDomainTest::layerEditingWithoutMaskTarget()
{
    auto* v = imported(); QVERIFY(v);
    const QByteArray before = coverage(*v);
    QVERIFY(pictura::mask_edit_target_set(*v, QString()));
    QVERIFY(dab(*v));
    QCOMPARE(coverage(*v), before);
    QVERIFY(pictura::layer_mask_delete(*v));
    QCOMPARE(v->sample_argb(9, 6), 0xffffffffu);
}

void CanvasMaskDomainTest::importedMaskCanvasTools()
{
    auto* v = imported(); QVERIFY(v);
    const pictura::PaintTip tip{3, 100, 100, 0, 25, 0, 1, 0, 0, 0};
    for (int tool = 0; tool < 3; ++tool) {
        const QByteArray before = coverage(*v);
        const int history = v->history_index();
        if (tool == 2) { QVERIFY(pictura::begin_eraser(*v, 0xff000000u, tip, 0, 100, 100, false, 1.0)); }
        else { QVERIFY(v->begin_paint(0xffffffffu, 0xff000000u, 3, 100, 100, 0, 100, 100, 25, QStringLiteral("normal"), tool == 1, false)); }
        QVERIFY(v->paint_dab(9, 6, 1));
        QVERIFY(v->end_paint());
        QCOMPARE(v->history_index(), history + 1);
        QCOMPARE(quint8(coverage(*v)[81]), quint8(tool == 2 ? 0 : 255));
        QCOMPARE(coverage(*v)[15], before[15]);
        QVERIFY(v->undo());
    }
}

void CanvasMaskDomainTest::importedMaskFillTools()
{
    auto* v = imported(); QVERIFY(v);
    QVERIFY(v->select_rect(7, 4, 4, 3, QStringLiteral("new"), 0.0));
    for (int operation = 0; operation < 7; ++operation) {
        const QByteArray before = coverage(*v);
        bool changed = false;
        if (operation == 0) { changed = pictura::edit_fill(*v, 0xff000000u, -1, QStringLiteral("normal"), 100, false); }
        if (operation == 1) { changed = pictura::draw_gradient(*v, 0, 0xff000000u, 0xff000000u, 0, QStringLiteral("normal"), 100, false, false, true, 7, 6, 11, 6); }
        if (operation == 2) { changed = pictura::bucket_fill_at(*v, 9, 6, 0xff000000u, -1, QStringLiteral("normal"), 100, 255, false, true, false); }
        if (operation == 3) { changed = pictura::edit_stroke(*v, 0xff000000u, 2, 1, QStringLiteral("normal"), 100); }
        if (operation == 4 || operation == 5) {
            for (const QPointF p : {QPointF(7,4),QPointF(11,4),QPointF(11,7),QPointF(7,7)}) {
                pictura::path_append_corner(*v, p.x(), p.y(), false);
                pictura::path_commit_anchor(*v);
            }
            pictura::path_close(*v);
            QVERIFY(pictura::paths_set_active(*v, -1));
            changed = operation == 4 ? pictura::paths_fill(*v, 0xff000000u)
                                    : pictura::paths_stroke(*v, 0xff000000u, 4, 100, 100, 100, 25);
        }
        if (operation == 6) {
            pictura::ShapeSpec spec{}; spec.kind=0; spec.boxed=true;
            spec.x0=7; spec.y0=4; spec.x1=11; spec.y1=7;
            changed = pictura::shape_fill_pixels(*v, spec, 0xff000000u);
        }
        QVERIFY2(changed, qPrintable(QString::number(operation)));
        QVERIFY(coverage(*v) != before);
        QCOMPARE(coverage(*v)[15], before[15]);
        QVERIFY(v->undo());
    }
}

void CanvasMaskDomainTest::importedMaskFilters()
{
    auto* v = imported(); QVERIFY(v);
    QVERIFY(v->select_rect(8, 5, 3, 2, QStringLiteral("new"), 0.0));
    const QByteArray before = coverage(*v);
    QVERIFY(pictura::image_adjust_direct(*v, QStringLiteral("invert")));
    QCOMPARE(quint8(coverage(*v)[81]), quint8(127));
    QCOMPARE(coverage(*v)[15], before[15]);
    QVERIFY(v->undo());
    QVERIFY(dab(*v));
    const QByteArray seeded = coverage(*v);
    QVERIFY(pictura::apply_filter_params(*v,QStringLiteral("gaussian-blur"),{1.0}));
    QVERIFY(quint8(coverage(*v)[81]) < quint8(seeded[81]));
    QCOMPARE(coverage(*v)[15],before[15]);
}

void CanvasMaskDomainTest::importedMaskReadOnlyViews()
{
    auto* v = imported(); QVERIFY(v);
    const QByteArray before = coverage(*v);
    const int history = v->history_index();
    QVERIFY(pictura::mask_solo_set(*v, QStringLiteral("0")));
    const auto snapshot = pictura::layers_api::snapshot(*v, 24, true);
    QVERIFY(!snapshot.rows.empty());
    QCOMPARE(coverage(*v), before);
    QVERIFY(pictura::mask_solo_set(*v, QString()));
    QCOMPARE(v->history_index(), history);
    QCOMPARE(coverage(*v), before);
}

void CanvasMaskDomainTest::dataLessImportFirstEditPreservesViews()
{
    auto* v = imported(true, 0, 128); QVERIFY(v);
    const int history = v->history_index();
    QCOMPARE(coverage(*v), QByteArray(96, char(255)));
    QVERIFY(pictura::mask_solo_set(*v, QStringLiteral("0")));
    QCOMPARE(v->canvas_display_image(0, 0, 0, 1, 1).pixelColor(0,0), QColor(Qt::white));
    QVERIFY(pictura::mask_solo_set(*v, QString()));
    QVERIFY(dab(*v, 0xff000000u));
    QCOMPARE(quint8(coverage(*v)[0]), quint8(255));
    QCOMPARE(quint8(coverage(*v)[81]), quint8(0));
    QCOMPARE(v->history_index(), history + 1);
    QVERIFY(v->undo());
    QCOMPARE(coverage(*v), QByteArray(96, char(255)));
    QVERIFY(v->redo());
    QCOMPARE(quint8(coverage(*v)[81]), quint8(0));
}

// [tp_solo_restore] A soloed mask is presented from mask tiles: undo, redo and
// a history jump while soloed show the restored coverage, and leaving solo
// shows the document again (#1216).
void CanvasMaskDomainTest::soloMaskRestoreShowsTheMask()
{
    auto* v = imported(false, 0, 255); QVERIFY(v);
    const auto shown = [v]() { return v->canvas_display_image(0, 9, 6, 1, 1).pixelColor(0, 0); };
    const int history = v->history_index();
    const quint8 before = quint8(coverage(*v)[81]);
    QVERIFY(pictura::mask_solo_set(*v, QStringLiteral("0")));
    QCOMPARE(shown().red(), int(before));
    QVERIFY(dab(*v, 0xff000000u));
    QCOMPARE(quint8(coverage(*v)[81]), quint8(0));
    QCOMPARE(shown().red(), 0);
    QVERIFY(v->undo());
    QCOMPARE(shown().red(), int(before));
    QVERIFY(v->redo());
    QCOMPARE(shown().red(), 0);
    QVERIFY(v->history_jump(history));
    QCOMPARE(shown().red(), int(before));
    QCOMPARE(pictura::mask_solo_get(*v), QStringLiteral("0"));
    QVERIFY(pictura::mask_solo_set(*v, QString()));
    QCOMPARE(v->canvas_display_image(0, 9, 6, 1, 1).pixel(0, 0), QRgb(v->sample_argb(9, 6)));
}

void CanvasMaskDomainTest::canvasMaskEditUndoRedo()
{
    auto* v = imported(false, -30, 0); QVERIFY(v);
    const QByteArray before = coverage(*v);
    const int history = v->history_index();
    QVERIFY(dab(*v));
    const QByteArray after = coverage(*v);
    QVERIFY(after != before);
    QCOMPARE(v->history_index(), history + 1);
    QVERIFY(v->undo()); QCOMPARE(coverage(*v), before);
    QVERIFY(v->redo()); QCOMPARE(coverage(*v), after);
    QVERIFY(v->commit_move(30, 0));
    QCOMPARE(quint8(coverage(*v)[15]), quint8(1));
}

void CanvasMaskDomainTest::canvasMaskCancelAndNoop()
{
    auto* v = imported(true); QVERIFY(v);
    const QByteArray before = coverage(*v);
    const int history = v->history_index();
    QVERIFY(v->begin_paint(0xff000000u, 0xffffffffu, 3, 100, 100, 0, 100, 100, 25, QStringLiteral("normal"), false, false));
    QVERIFY(v->paint_dab(9,6,1));
    v->cancel_paint();
    QCOMPARE(coverage(*v), before);
    QCOMPARE(v->history_index(), history);
    QVERIFY(pictura::filter_preview(*v,QStringLiteral("gaussian-blur"),{1.0}));
    QVERIFY(pictura::filter_preview_cancel(*v));
    QCOMPARE(coverage(*v),before);
    QCOMPARE(v->history_index(),history);
    QVERIFY(!v->commit_move(0,0));
    QCOMPARE(coverage(*v), before);
    QVERIFY(v->select_rect(2,1,3,2,QStringLiteral("new"),0));
    const int selected = v->history_index();
    QVERIFY(!pictura::move_selection_content(*v,0,0,false,0xffffffffu));
    QCOMPARE(v->history_index(), selected);
}

void CanvasMaskDomainTest::canvasMaskRefusalIsAtomic()
{
    auto* v = imported(); QVERIFY(v);
    const QByteArray before = coverage(*v);
    const int history = v->history_index();
    pictura::set_available_memory_for_test(0);
    QVERIFY(!dab(*v));
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!v->commit_move(30,0));
    QVERIFY(!pictura::layer_mask_invert(*v));
    QVERIFY(pictura::edit_refused_for_memory());
    QCOMPARE(v->history_index(), history);
    pictura::set_available_memory_for_test(-1);
    QCOMPARE(coverage(*v), before);
    QVERIFY(!pictura::layer_mask_add(*v, QStringLiteral("reveal-all")));
    QVERIFY(!pictura::edit_refused_for_memory());
    QCOMPARE(v->history_index(), history);
}

void CanvasMaskDomainTest::wholeMaskMoveBlackPreview()
{
    auto* v = imported(false, 0, 255); QVERIFY(v);
    auto* colors = window_->findChild<pictura::ColorState*>(); QVERIFY(colors);
    colors->setForeground(Qt::white); colors->setBackground(Qt::white);
    QVERIFY(pictura::mask_solo_set(*v,QStringLiteral("0")));
    QVERIFY(v->begin_move_preview());
    const QByteArray stored = coverage(*v);
    QVERIFY(pictura::move_preview_follow(*v,3,0));
    QCOMPARE(coverage(*v),stored);
    QCOMPARE(v->move_preview_x(),-3);
    QVERIFY(!v->move_preview_layer().isNull());
    QCOMPARE(v->move_preview_layer().pixelColor(0,0),QColor(Qt::black));
    QCOMPARE(v->move_preview_layer().pixelColor(6,1).red(),1);
    v->end_move_preview();
    QVERIFY(v->commit_move(3,0));
    QCOMPARE(quint8(coverage(*v)[0]), quint8(0));
    QCOMPARE(quint8(coverage(*v)[18]), quint8(1));
    QVERIFY(v->commit_move(std::numeric_limits<int>::min()+1,0));
    QCOMPARE(coverage(*v),QByteArray(96,char(0)));
    QVERIFY(v->commit_move(std::numeric_limits<int>::max(),0));
    QCOMPARE(quint8(coverage(*v)[18]),quint8(1));
}

void CanvasMaskDomainTest::maskMoveRouting()
{
    auto* v = imported(false,0,0); QVERIFY(v);
    const QString before = pictura::transform_api::layerRect(*v,QStringLiteral("0")).toString();
    QVERIFY(v->commit_move(2,0));
    QCOMPARE(pictura::transform_api::layerRect(*v,QStringLiteral("0")).toString(), before);
    QVERIFY(pictura::layer_mask_set_linked(*v, true));
    QVERIFY(v->commit_move(2,0));
    QVERIFY(pictura::transform_api::layerRect(*v,QStringLiteral("0")).toString() != before);
}

void CanvasMaskDomainTest::selectedMaskMoveRoutesCoverage()
{
    auto* v = imported(false,0,0); QVERIFY(v);
    QVERIFY(v->select_rect(3,1,2,1,QStringLiteral("new"),0));
    const QString pixels = pictura::transform_api::layerRect(*v,QStringLiteral("0")).toString();
    QVERIFY(pictura::move_selection_content(*v,5,0,false,0xffffffffu));
    QCOMPARE(quint8(coverage(*v)[20]),quint8(1));
    QCOMPARE(quint8(coverage(*v)[21]),quint8(64));
    QCOMPARE(pictura::transform_api::layerRect(*v,QStringLiteral("0")).toString(),pixels);
}
void CanvasMaskDomainTest::selectedMaskMoveBlackBackground()
{
    auto* v = imported(false,0,0); QVERIFY(v);
    QVERIFY(v->select_rect(4,2,1,1,QStringLiteral("new"),0));
    QVERIFY(pictura::move_selection_content(*v,3,0,false,0xffffffffu));
    QCOMPARE(quint8(coverage(*v)[28]),quint8(0));
    QCOMPARE(quint8(coverage(*v)[31]),quint8(255));
}
void CanvasMaskDomainTest::selectedMaskAltRetainsCoverage()
{
    auto* v = imported(false,0,0); QVERIFY(v);
    const int layers = v->layer_count();
    QVERIFY(v->select_rect(3,1,2,1,QStringLiteral("new"),0));
    QVERIFY(pictura::move_selection_content(*v,30,0,true,0xffffffffu));
    QCOMPARE(quint8(coverage(*v)[15]),quint8(1));
    QCOMPARE(v->layer_count(),layers);
    QCOMPARE(v->active_layer_path(),QStringLiteral("0"));
    QVERIFY(v->commit_move(-30,0));
    QCOMPARE(quint8(coverage(*v)[15]),quint8(1));
}
void CanvasMaskDomainTest::selectedMaskCtrlRoutesCoverage()
{
    auto* v = imported(false,0,0); QVERIFY(v);
    QVERIFY(v->select_rect(3,1,2,1,QStringLiteral("new"),0));
    QVERIFY(pictura::begin_selection_move(*v));
    QVERIFY(pictura::preview_selection_move(*v,3,0));
    QVERIFY(pictura::move_selection_content(*v,3,0,false,0xffffffffu));
    QCOMPARE(quint8(coverage(*v)[18]),quint8(1));
    QCOMPARE(quint8(coverage(*v)[15]),quint8(0));
}
void CanvasMaskDomainTest::selectedMaskMoveHistory()
{
    auto* v = imported(false,0,0); QVERIFY(v);
    QVERIFY(v->select_rect(3,1,2,1,QStringLiteral("new"),0));
    const QByteArray before = coverage(*v);
    const int index = v->history_index();
    QVERIFY(pictura::move_selection_content(*v,30,0,false,0xffffffffu));
    QCOMPARE(v->history_index(),index+1);
    QCOMPARE(v->history_label(index+1),QStringLiteral("Move Selection"));
    QVERIFY(v->undo()); QCOMPARE(coverage(*v),before);
    QVERIFY(v->redo());
    QVERIFY(v->commit_move(-30,0));
    QCOMPARE(quint8(coverage(*v)[15]),quint8(1));
}

void CanvasMaskDomainTest::propertiesMaskRefinementUndo()
{
    auto* v = imported(); QVERIFY(v);
    const QByteArray before = coverage(*v);
    QVERIFY(pictura::layer_mask_set_density(*v,128,true));
    QCOMPARE(pictura::layer_mask_density(*v),128);
    QVERIFY(v->undo());
    QCOMPARE(pictura::layer_mask_density(*v),255);
    QCOMPARE(coverage(*v),before);
}
void CanvasMaskDomainTest::propertiesInvertPreservesOffcanvasCoverage()
{
    auto* v = imported(false,-30,0); QVERIFY(v);
    QVERIFY(v->select_rect(8,5,1,1,QStringLiteral("new"),0));
    const int history = v->history_index();
    window_->selectLayerPath(QStringLiteral("0"));
    auto* button = window_->findChild<QToolButton*>(QStringLiteral("propertiesMaskInvert"));
    QVERIFY(button); QVERIFY(button->isEnabled());
    button->click();
    QCOMPARE(coverage(*v),QByteArray(96,char(255)));
    QCOMPARE(v->history_index(),history+1);
    QVERIFY(v->undo()); QCOMPARE(coverage(*v),QByteArray(96,char(0)));
    QVERIFY(v->redo());
    QVERIFY(v->commit_move(30,0));
    QCOMPARE(quint8(coverage(*v)[15]),quint8(1));
    QCOMPARE(quint8(coverage(*v)[16]),quint8(64));
}

int main(int argc, char** argv)
{
    return pictura::test::shardedMain<CanvasMaskDomainTest>(argc, argv);
}
#include "tst_canvas_mask_domain.moc"
