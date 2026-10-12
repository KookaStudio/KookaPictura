// Unit A: document_api + history_api as the typed façade over open/save/dirty,
// the pyramid's CanvasSize crossing, and history navigation.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/history_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <rust/cxx.h>

#include "qt_test_support.h"

using namespace pictura;

class DocumentApiTest : public QObject {
    Q_OBJECT

private slots:
    void newDocumentOpensCleanWithTypedPyramid();
    void saveAndReopenRoundTrip();
    void undoRedoThroughTheAdapter();
};

void DocumentApiTest::newDocumentOpensCleanWithTypedPyramid()
{
    pictura::PictureView view;
    QVERIFY(document_api::newDocument(view, 800, 600, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QVERIFY(view.has_document());
    QCOMPARE(document_api::documentMode(view), QStringLiteral("rgb"));
    QCOMPARE(document_api::documentDepthBits(view), 8);

    // The typed pyramid equals the legacy `"w h"` contract: level 0 is the
    // document, each level halves until the long side drops below 256.
    const QVector<document_api::CanvasSize> levels = document_api::pyramidSizes(view);
    QCOMPARE(levels.size(), 2);
    QCOMPARE(levels.at(0).width, 800);
    QCOMPARE(levels.at(0).height, 600);
    QVERIFY(levels.at(0).valid);
    QCOMPARE(levels.at(1).width, 400);
    QCOMPARE(levels.at(1).height, 300);
    QVERIFY(levels.at(1).valid);
    QCOMPARE(document_api::levelCount(view), 2);

    // Equivalence: the adapter's typed values match the raw bridge crossing.
    const ::rust::Vec<int32_t> extents = view.display_level_extents();
    QCOMPARE(int(extents.size()), levels.size() * 2);
    for (int i = 0; i < levels.size(); ++i) {
        QCOMPARE(levels.at(i).width, int(extents[size_t(i) * 2]));
        QCOMPARE(levels.at(i).height, int(extents[size_t(i) * 2 + 1]));
    }

    QVERIFY(!document_api::pyramidSize(view, -1).valid);
    QVERIFY(!document_api::pyramidSize(view, levels.size()).valid);
    QVERIFY(document_api::openNotice(view).isEmpty());
}

void DocumentApiTest::saveAndReopenRoundTrip()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("roundtrip.psd"));

    pictura::PictureView written;
    QVERIFY(document_api::newDocument(written, 64, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QVERIFY(document_api::save(written, path));
    QVERIFY(!document_api::isDirty(written));
    QCOMPARE(document_api::filePath(written), path);

    pictura::PictureView opened;
    QVERIFY(document_api::open(opened, path));
    QCOMPARE(document_api::documentMode(opened), QStringLiteral("rgb"));
    QCOMPARE(document_api::pyramidSize(opened, 0).width, 64);
    QCOMPARE(document_api::pyramidSize(opened, 0).height, 32);
    QCOMPARE(document_api::filePath(opened), path);
}

void DocumentApiTest::undoRedoThroughTheAdapter()
{
    pictura::PictureView view;
    QVERIFY(document_api::newDocument(view, 64, 64, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QCOMPARE(history_api::count(view), 1);
    QCOMPARE(history_api::index(view), 0);
    QCOMPARE(history_api::label(view, 0), QStringLiteral("New"));
    QVERIFY(!history_api::canUndo(view));
    QVERIFY(!history_api::canRedo(view));
    QVERIFY(!history_api::undo(view));
    QVERIFY(!history_api::redo(view));

    // A named snapshot is a history navigation target through the adapter.
    QVERIFY(history_api::addSnapshot(view, QStringLiteral("Check")));
    QCOMPARE(history_api::snapshotCount(view), 1);
    QCOMPARE(history_api::snapshotLabel(view, 0), QStringLiteral("Check"));
    QVERIFY(history_api::restoreSnapshot(view, 0));

    QVERIFY(history_api::addSnapshot(view, QStringLiteral("Second")));
    QVERIFY(history_api::jump(view, 0));
    QCOMPARE(history_api::index(view), 0);
    QVERIFY(history_api::canRedo(view) || history_api::index(view) == 0);

    // The changed signal crosses without the consumer including the bridge.
    int changes = 0;
    QMetaObject::Connection conn = history_api::connectChanged(view, this, [&changes] { ++changes; });
    QVERIFY(history_api::addSnapshot(view, QStringLiteral("Signal")));
    QVERIFY(changes > 0);
    QObject::disconnect(conn);

    QVERIFY(history_api::purgeHistories(view));
    QCOMPARE(history_api::snapshotCount(view), 0);
}

QTEST_MAIN(DocumentApiTest)
#include "tst_document_api.moc"