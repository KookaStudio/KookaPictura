// WebP import/export through the engine codec, independent of the optional Qt
// WebP plugin (#483).

#include <QtTest/QtTest>

#include "api/document_api.h"

#include "api/engine_bridge.h"
#include "export_as_dialog.h"

#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>

#include "qt_test_support.h"

using namespace pictura;

namespace {

// A 16 x 16 field with a clear 4 x 4 corner, so a lossless round trip has to
// keep both per-pixel colour and alpha.
QImage transparentCornerPicture()
{
    QImage picture(16, 16, QImage::Format_RGBA8888);
    for (int y = 0; y < 16; ++y) {
        for (int x = 0; x < 16; ++x) {
            const int alpha = (x < 4 && y < 4) ? 0 : 255;
            picture.setPixelColor(x, y, QColor(x * 16, y * 16, (x * y) % 256, alpha));
        }
    }
    return picture;
}

} // namespace

class WebpTest : public QObject {
    Q_OBJECT

private slots:
    void saveAndReopenRoundTrips();
    void exportHonorsScale();
    void saveStaysLossless();
    void exportLossyPreservesAlpha();
    void exportLosslessRoundTrips();
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

// Save As has no WebP options dialog and keeps the lossless round trip even
// though Export As now defaults to lossy.
void WebpTest::saveStaysLossless()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QImage source = transparentCornerPicture();
    const QString png = dir.filePath(QStringLiteral("source.png"));
    QVERIFY(source.save(png));

    PictureView view;
    QVERIFY(document_api::openImage(view, png));
    const QString webp = dir.filePath(QStringLiteral("saved.webp"));
    QVERIFY(document_api::save(view, webp));

    PictureView reopened;
    QVERIFY(document_api::openImage(reopened, webp));
    QCOMPARE(reopened.image().convertToFormat(QImage::Format_RGBA8888),
             view.image().convertToFormat(QImage::Format_RGBA8888));
}

// Export As defaults to lossy VP8 and keeps the clear corner transparent.
void WebpTest::exportLossyPreservesAlpha()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QVector<ExportEntry> entries;
    entries.append(ExportEntry{QStringLiteral("picture"), QStringLiteral("document"),
                                transparentCornerPicture()});
    ExportAsDialog dialog(entries);
    auto* format = dialog.findChild<QComboBox*>(QStringLiteral("exportAsFormat"));
    QVERIFY(format);
    format->setCurrentIndex(ExportAsDialog::Webp);

    const QByteArray bytes = dialog.encoded(0);
    QVERIFY(bytes.contains(QByteArrayLiteral("VP8 ")));
    QVERIFY(!bytes.contains(QByteArrayLiteral("VP8L")));
    const QString path = dir.filePath(QStringLiteral("lossy.webp"));
    QVERIFY(dialog.saveTo(0, path));

    PictureView opened;
    QVERIFY(document_api::openImage(opened, path));
    QCOMPARE(opened.image().pixelColor(0, 0).alpha(), 0);
    QCOMPARE(opened.image().pixelColor(8, 8).alpha(), 255);
}

// The Lossless control writes VP8L and reopens every pixel exactly.
void WebpTest::exportLosslessRoundTrips()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QImage source = transparentCornerPicture();
    QVector<ExportEntry> entries;
    entries.append(ExportEntry{QStringLiteral("picture"), QStringLiteral("document"), source});
    ExportAsDialog dialog(entries);
    auto* format = dialog.findChild<QComboBox*>(QStringLiteral("exportAsFormat"));
    auto* lossless = dialog.findChild<QCheckBox*>(QStringLiteral("exportAsLossless"));
    QVERIFY(format && lossless);
    format->setCurrentIndex(ExportAsDialog::Webp);
    lossless->setChecked(true);

    const QByteArray bytes = dialog.encoded(0);
    QVERIFY(bytes.contains(QByteArrayLiteral("VP8L")));
    const QString path = dir.filePath(QStringLiteral("lossless.webp"));
    QVERIFY(dialog.saveTo(0, path));

    PictureView opened;
    QVERIFY(document_api::openImage(opened, path));
    // Premultiplied comparison ignores the RGB under a fully transparent pixel,
    // which is not meaningful in either image.
    QCOMPARE(opened.image().convertToFormat(QImage::Format_RGBA8888_Premultiplied),
             source.convertToFormat(QImage::Format_RGBA8888_Premultiplied));
}

PICTURA_QT_TEST_MAIN(WebpTest)
#include "tst_webp.moc"
