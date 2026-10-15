// WebP import/export through the engine codec, independent of the optional Qt
// WebP plugin (#483).

#include <QtTest/QtTest>

#include "api/document_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/export.cxxqt.h"

#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>

#include "qt_test_support.h"

using namespace pictura;

class WebpTest : public QObject {
    Q_OBJECT

private slots:
    void saveAndReopenRoundTrips();
    void exportHonorsScale();
};

// Save writes a RIFF/WEBP container through the engine encoder and reopening
// decodes it through the engine decoder, so the round trip needs no Qt plugin.
void WebpTest::saveAndReopenRoundTrips()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("round-trip.webp"));

    PictureView written;
    QVERIFY(document_api::newDocument(written, 4, 3, QStringLiteral("rgb"), 8,
                                      QStringLiteral("transparent")));
    QVERIFY(document_api::save(written, path));

    QFile file(path);
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray head = file.read(12);
    QCOMPARE(head.left(4), QByteArrayLiteral("RIFF"));
    QCOMPARE(head.mid(8, 4), QByteArrayLiteral("WEBP"));

    PictureView opened;
    QVERIFY(document_api::openImage(opened, path));
    QCOMPARE(document_api::pyramidSize(opened, 0).width, 4);
    QCOMPARE(document_api::pyramidSize(opened, 0).height, 3);
    // The engine's lossless codec keeps the transparent pixel transparent.
    QCOMPARE(opened.image().pixelColor(0, 0).alpha(), 0);
}

// Export As honors the percent scale for WebP, and leaves the document's path
// untouched.
void WebpTest::exportHonorsScale()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("scaled.webp"));

    PictureView view;
    QVERIFY(document_api::newDocument(view, 8, 6, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QVERIFY(export_image(view, path, QStringLiteral("WEBP"), -1, 50));
    QVERIFY(document_api::filePath(view).isEmpty());

    PictureView opened;
    QVERIFY(document_api::openImage(opened, path));
    QCOMPARE(document_api::pyramidSize(opened, 0).width, 4);
    QCOMPARE(document_api::pyramidSize(opened, 0).height, 3);
}

QTEST_MAIN(WebpTest)
#include "tst_webp.moc"
