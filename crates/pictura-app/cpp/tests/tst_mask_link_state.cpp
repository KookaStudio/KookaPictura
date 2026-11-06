#include <QtTest/QtTest>

#include "api/layers_api.h"
#include "api/layers_bridge.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "qt_test_support.h"
#include "theme.h"

#include <QtGui/QPainter>
#include <QtWidgets/QTreeView>

class MaskLinkStateTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void importedMaskStates_data();
    void importedMaskStates();
    void rasterGlyphOpacity();
    void vectorGlyphOpacity();
    void togglesAndHistory_data();
    void togglesAndHistory();

private:
    bool newDocument();
    bool addMask(bool vector);
    bool setLinked(bool vector, bool linked);
    bool rowLinked(const QString& path, bool vector) const;
    void refresh();
    QRect glyphRect(const QString& path, bool vector) const;
    QRect drawnRect(const QString& path, bool vector) const;
    QImage referenceGlyph(const QString& path, bool vector, bool linked, qreal dpr) const;
    void verifyGlyph(const QString& path, bool vector, bool linked, qreal dpr);
    void verifyOpacity(bool vector);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void MaskLinkStateTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

void MaskLinkStateTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
}

bool MaskLinkStateTest::newDocument()
{
    if (!window_->newDocument(QStringLiteral("Mask links"), 16, 16,
                              QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    return view_ && panel_;
}

bool MaskLinkStateTest::addMask(bool vector)
{
    return vector ? pictura::vector_mask_add(*view_, QStringLiteral("reveal-all"))
                  : pictura::layer_mask_add(*view_, QStringLiteral("reveal-all"));
}

bool MaskLinkStateTest::setLinked(bool vector, bool linked)
{
    return vector ? pictura::vector_mask_set_linked(*view_, linked)
                  : pictura::layer_mask_set_linked(*view_, linked);
}

bool MaskLinkStateTest::rowLinked(const QString& path, bool vector) const
{
    return vector ? panel_->rowVectorMaskLinkedForTest(path)
                  : panel_->rowMaskLinkedForTest(path);
}

void MaskLinkStateTest::refresh()
{
    QCoreApplication::processEvents();
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(0);
}

QRect MaskLinkStateTest::glyphRect(const QString& path, bool vector) const
{
    auto* tree = panel_->findChild<QTreeView*>();
    if (!tree) {
        return {};
    }
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(tree->itemDelegate());
    for (int row = 0; row < tree->model()->rowCount(); ++row) {
        const QModelIndex index = tree->model()->index(row, 0);
        if (index.data(pictura::PathRole).toString() == path) {
            const QRect item(QPoint(), tree->visualRect(index).size());
            return vector ? delegate->vectorLinkGlyphRect(item, index)
                          : delegate->linkGlyphRect(item, index);
        }
    }
    return {};
}

QRect MaskLinkStateTest::drawnRect(const QString& path, bool vector) const
{
    const QRect glyph = glyphRect(path, vector);
    const int side = panel_->linkIconSideForTest();
    return QRect(glyph.center().x() - side / 2, glyph.center().y() - side / 2,
                 side, side);
}

QImage MaskLinkStateTest::referenceGlyph(const QString& path, bool vector, bool linked,
                                        qreal dpr) const
{
    const QImage actual = panel_->rowImageForTest(path, dpr);
    const int side = panel_->linkIconSideForTest();
    const QPixmap asset = pictura::icon(linked ? QStringLiteral("layers.link2")
                                              : QStringLiteral("layers.unlink2"))
                              .pixmap(side, side)
                              .transformed(QTransform().rotate(90), Qt::SmoothTransformation);
    if (asset.isNull() || actual.isNull()) {
        return {};
    }
    QImage ink(actual.size(), actual.format());
    ink.setDevicePixelRatio(dpr);
    ink.fill(Qt::transparent);
    QPainter painter(&ink);
    painter.setOpacity(linked ? 1.0 : 0.5);
    painter.drawPixmap(drawnRect(path, vector), asset);
    painter.end();
    return ink;
}

void MaskLinkStateTest::verifyGlyph(const QString& path, bool vector, bool linked, qreal dpr)
{
    QCOMPARE(rowLinked(path, vector), linked);
    const QImage actual = panel_->rowImageForTest(path, dpr);
    QVERIFY(!actual.isNull());
    QCOMPARE(actual.devicePixelRatio(), dpr);
    QVERIFY(!glyphRect(path, vector).isEmpty());
    const QImage reference = referenceGlyph(path, vector, linked, dpr);
    QVERIFY(!reference.isNull());
    const QColor background = actual.pixelColor(actual.width() - 4, 1);
    QImage expected(actual.size(), actual.format());
    expected.setDevicePixelRatio(dpr);
    expected.fill(background);
    QPainter painter(&expected);
    painter.drawImage(QPoint(), reference);
    painter.end();
    const QRect drawn = drawnRect(path, vector);
    const QRect pixels(drawn.topLeft() * dpr, drawn.size() * dpr);
    QVERIFY(actual.rect().contains(pixels));
    int ink = 0;
    for (int y = pixels.top(); y <= pixels.bottom(); ++y) {
        for (int x = pixels.left(); x <= pixels.right(); ++x) {
            // Transparent padding can overlap a neighbouring thumbnail. Only
            // the ink composites over the row surface; padding is checked by
            // the unchanged-pixels comparison in verifyOpacity().
            if (reference.pixelColor(x, y).alpha() == 0) {
                continue;
            }
            const QColor want = expected.pixelColor(x, y);
            const QColor got = actual.pixelColor(x, y);
            ink += want != background;
            QVERIFY2(qAbs(got.red() - want.red()) <= 2
                         && qAbs(got.green() - want.green()) <= 2
                         && qAbs(got.blue() - want.blue()) <= 2
                         && got.alpha() == want.alpha(),
                     qPrintable(QStringLiteral("glyph pixel (%1,%2), linked=%3 dpr=%4: got %5 expected %6")
                                    .arg(x).arg(y).arg(linked).arg(dpr)
                                    .arg(got.name()).arg(want.name())));
        }
    }
    QVERIFY2(ink > 0, "the reference glyph must contain visible ink");
}

void MaskLinkStateTest::verifyOpacity(bool vector)
{
    QVERIFY(newDocument());
    const QString path = pictura::layers_api::rowAt(*view_, 0).path;
    view_->set_active_layer(path);
    QVERIFY(addMask(vector));
    refresh();
    for (const qreal dpr : {1.0, 2.0}) {
        if (!rowLinked(path, vector)) {
            QVERIFY(setLinked(vector, true));
            refresh();
        }
        const QRect geometry = glyphRect(path, vector);
        const QImage linked = panel_->rowImageForTest(path, dpr);
        const QImage linkedInk = referenceGlyph(path, vector, true, dpr);
        verifyGlyph(path, vector, true, dpr);
        QVERIFY(setLinked(vector, false));
        refresh();
        const QImage unlinked = panel_->rowImageForTest(path, dpr);
        const QImage unlinkedInk = referenceGlyph(path, vector, false, dpr);
        verifyGlyph(path, vector, false, dpr);
        QCOMPARE(glyphRect(path, vector), geometry);
        QCOMPARE(linked.size(), unlinked.size());
        for (int y = 0; y < linked.height(); ++y) {
            for (int x = 0; x < linked.width(); ++x) {
                if (linkedInk.pixelColor(x, y).alpha() == 0
                    && unlinkedInk.pixelColor(x, y).alpha() == 0) {
                    QCOMPARE(linked.pixel(x, y), unlinked.pixel(x, y));
                }
            }
        }
    }
}

void MaskLinkStateTest::rasterGlyphOpacity()
{
    verifyOpacity(false);
}

void MaskLinkStateTest::vectorGlyphOpacity()
{
    verifyOpacity(true);
}

void MaskLinkStateTest::importedMaskStates_data()
{
    QTest::addColumn<bool>("linked");
    QTest::newRow("linked") << true;
    QTest::newRow("unlinked") << false;
}

void MaskLinkStateTest::importedMaskStates()
{
    QFETCH(bool, linked);
    const QString fixture = linked
        ? QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/mask_linked.psd")
        : QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/mask_unlinked.psd");
    QVERIFY(!fixture.isEmpty());
    QVERIFY(window_->openDocumentAtPath(fixture));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);
    view_->set_active_layer(QStringLiteral("0"));
    QCOMPARE(pictura::layer_mask_linked(*view_), linked);
    refresh();
    verifyGlyph(QStringLiteral("0"), false, linked, 1.0);
}

void MaskLinkStateTest::togglesAndHistory_data()
{
    QTest::addColumn<bool>("vector");
    QTest::newRow("raster") << false;
    QTest::newRow("vector") << true;
}

void MaskLinkStateTest::togglesAndHistory()
{
    QFETCH(bool, vector);
    QVERIFY(newDocument());
    const QString active = view_->add_layer_in(QString());
    const QString clicked = view_->add_layer_in(QString());
    QVERIFY(!active.isEmpty() && !clicked.isEmpty());
    for (const QString& path : {active, clicked}) {
        view_->set_active_layer(path);
        QVERIFY(addMask(vector));
    }
    QVERIFY(setLinked(vector, false));
    view_->set_active_layer(active);
    panel_->setView(view_);
    panel_->selectPaths({active}, active);
    refresh();
    const QImage dimmed = panel_->rowImageForTest(clicked);
    verifyGlyph(clicked, vector, false, 1.0);
    const int history = view_->history_count();
    QVERIFY(vector ? panel_->clickVectorLinkGlyphForTest(clicked)
                   : panel_->clickLinkGlyphForTest(clicked));
    refresh();
    QCOMPARE(view_->active_layer_path(), active);
    QVERIFY(rowLinked(active, vector));
    QCOMPARE(view_->history_count(), history + 1);
    verifyGlyph(clicked, vector, true, 1.0);
    const QImage full = panel_->rowImageForTest(clicked);
    QVERIFY(view_->undo());
    refresh();
    QVERIFY(rowLinked(active, vector));
    verifyGlyph(clicked, vector, false, 1.0);
    QCOMPARE(panel_->rowImageForTest(clicked), dimmed);
    QVERIFY(view_->redo());
    refresh();
    QVERIFY(rowLinked(active, vector));
    verifyGlyph(clicked, vector, true, 1.0);
    QCOMPARE(panel_->rowImageForTest(clicked), full);
}

QTEST_MAIN(MaskLinkStateTest)
#include "tst_mask_link_state.moc"
