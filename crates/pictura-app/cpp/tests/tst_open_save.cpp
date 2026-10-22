// Open/Save fixes (#874, #875): a raster open records its path, the Save As
// filter owns the extension, and JPEG/TIFF keep the document metadata.

#include <QtTest/QtTest>

#include "dialogs.h"
#include "frame.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QProcess>
#include <QtCore/QStandardPaths>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtGui/QImageReader>

#include "qt_test_support.h"

class OpenSaveTest : public QObject {
    Q_OBJECT

private slots:
    void rasterOpenKeepsFilePath();
    void chosenFormatWinsOverTypedSuffix();
    void rasterSaveKeepsFileInfo();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void OpenSaveTest::rasterOpenKeepsFilePath()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("source.png"));
    QImage image(16, 16, QImage::Format_RGBA8888);
    image.fill(QColor(10, 20, 30));
    QVERIFY(image.save(path));
    const QByteArray before = [&path] {
        QFile file(path);
        return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
    }();

    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->openDocumentAtPath(path));
    QCOMPARE(window->documentCount(), 1);
    QCOMPARE(window->documentPath(0), path);

    pictura::PictureView* view = window->viewAt(0);
    QVERIFY(view);
    view->set_active_layer(QStringLiteral("0"));
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(8.0, 8.0, 1.0));
    QVERIFY(view->end_paint());
    QVERIFY(window->isDocumentDirty(0));

    QVERIFY2(window->saveActive(), "Save writes the recorded path without Save As");
    QFile saved(path);
    QVERIFY(saved.open(QIODevice::ReadOnly));
    QVERIFY2(saved.readAll() != before, "the source file is rewritten in place");
    QVERIFY2(window->revertActive(), "Revert is available for a raster open");
}

void OpenSaveTest::chosenFormatWinsOverTypedSuffix()
{
    const QStringList& filters = pictura::saveFileFilters();
    const int png = filters.indexOf(QStringLiteral("PNG (*.PNG)"));
    QVERIFY(png >= 0);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString typed = dir.filePath(QStringLiteral("report.jpg"));
    const QString normalized = pictura::pathForSaveFilter(typed, png, 0);
    QCOMPARE(normalized, dir.filePath(QStringLiteral("report.png")));
    QCOMPARE(pictura::pathForSaveFilter(typed, filters.size() - 1, 0), typed);

    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Save"), 8, 8, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    QVERIFY(window->saveActiveAs(normalized));
    QVERIFY2(QFileInfo::exists(normalized), "the PNG is written");
    QVERIFY2(!QFileInfo::exists(typed), "the typed suffix does not win");
    QImageReader reader(normalized);
    QCOMPARE(reader.format(), QByteArrayLiteral("png"));
}

void OpenSaveTest::rasterSaveKeepsFileInfo()
{
    const QString exiftool = QStandardPaths::findExecutable(QStringLiteral("exiftool"));
    if (exiftool.isEmpty()) {
        QSKIP("exiftool is not installed");
    }
    QString fixture =
        QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/metadata.psd");
    if (fixture.isEmpty()) {
        QDir dir(QCoreApplication::applicationDirPath());
        for (int i = 0; i < 6 && fixture.isEmpty(); ++i) {
            const QString candidate =
                dir.filePath(QStringLiteral("crates/pictura-codec/tests/fixtures/metadata.psd"));
            if (QFileInfo::exists(candidate)) {
                fixture = candidate;
            } else if (!dir.cdUp()) {
                break;
            }
        }
    }
    if (fixture.isEmpty()) {
        QSKIP("metadata.psd fixture not found");
    }
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->openDocumentAtPath(fixture));

    for (const QString& suffix : {QStringLiteral("jpg"), QStringLiteral("tif")}) {
        const QString out = dir.filePath(QStringLiteral("metadata.") + suffix);
        QVERIFY(window->saveActiveAs(out));
        QProcess tool;
        tool.start(exiftool, {QStringLiteral("-j"), QStringLiteral("-Title"),
                              QStringLiteral("-ObjectName"), QStringLiteral("-Make"), out});
        QVERIFY(tool.waitForFinished(30000));
        const QJsonDocument json = QJsonDocument::fromJson(tool.readAllStandardOutput());
        QVERIFY(json.isArray());
        const QJsonObject fields = json.array().first().toObject();
        QCOMPARE(fields.value(QStringLiteral("Title")).toString(),
                 QStringLiteral("Fixture Title"));
        QCOMPARE(fields.value(QStringLiteral("ObjectName")).toString(),
                 QStringLiteral("Fixture Title"));
        QCOMPARE(fields.value(QStringLiteral("Make")).toString(),
                 QStringLiteral("ACME Cameras"));
    }
}

QTEST_MAIN(OpenSaveTest)
#include "tst_open_save.moc"
