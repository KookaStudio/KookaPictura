// The app's C++ edges with untrusted input (#844): the control socket's
// session token and document paths (#646), Qt image decode under the import
// budget, and the packed thumbnail unpacking of the Import PDF dialog.

#include <QtTest/QtTest>

#include "control_server.h"
#include "decode_image.h"
#include "import_pdf_dialog.h"

#include <QtCore/QBuffer>
#include <QtCore/QEventLoop>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtCore/QtEndian>
#include <QtGui/QImage>
#include <QtNetwork/QLocalSocket>

#include "qt_test_support.h"

namespace {

// One request line and its response, driving the server on this thread.
QJsonObject transact(QLocalSocket& socket, const QJsonObject& request)
{
    socket.write(QJsonDocument(request).toJson(QJsonDocument::Compact) + "\n");
    socket.flush();
    QByteArray buffer;
    QEventLoop loop;
    QTimer timer;
    timer.setSingleShot(true);
    QObject::connect(&timer, &QTimer::timeout, &loop, &QEventLoop::quit);
    QObject::connect(&socket, &QLocalSocket::readyRead, &loop, &QEventLoop::quit);
    timer.start(5000);
    while (buffer.indexOf('\n') < 0 && timer.isActive()) {
        loop.exec();
        buffer.append(socket.readAll());
    }
    return QJsonDocument::fromJson(buffer.left(buffer.indexOf('\n'))).object();
}

QString errorCode(const QJsonObject& response)
{
    return response.value(QStringLiteral("error"))
        .toObject()
        .value(QStringLiteral("code"))
        .toString();
}

QJsonObject statusRequest(const QJsonValue& token)
{
    QJsonObject request{{QStringLiteral("id"), 9},
                        {QStringLiteral("method"), QStringLiteral("status")},
                        {QStringLiteral("params"), QJsonObject()}};
    if (!token.isUndefined()) {
        request.insert(QStringLiteral("token"), token);
    }
    return request;
}

quint32 crc32(const QByteArray& bytes)
{
    quint32 crc = 0xFFFFFFFFu;
    for (const char byte : bytes) {
        crc ^= static_cast<quint8>(byte);
        for (int bit = 0; bit < 8; ++bit) {
            crc = (crc >> 1) ^ (0xEDB88320u & (0u - (crc & 1u)));
        }
    }
    return ~crc;
}

void appendChunk(QByteArray& png, const QByteArray& kind, const QByteArray& data)
{
    char length[4];
    qToBigEndian<quint32>(quint32(data.size()), length);
    png.append(length, 4);
    png.append(kind + data);
    char crc[4];
    qToBigEndian<quint32>(crc32(kind + data), crc);
    png.append(crc, 4);
}

// A complete, valid `width` × `height` transparent RGBA PNG: Qt decodes it
// unless the budget refuses it first.
QByteArray pngImage(quint32 width, quint32 height)
{
    QByteArray png("\x89PNG\r\n\x1a\n", 8);
    QByteArray ihdr(13, '\0');
    qToBigEndian<quint32>(width, ihdr.data());
    qToBigEndian<quint32>(height, ihdr.data() + 4);
    ihdr[8] = 8;
    ihdr[9] = 6;
    appendChunk(png, "IHDR", ihdr);
    const QByteArray rows(qsizetype(height) * (1 + qsizetype(width) * 4), '\0');
    // qCompress prefixes the zlib stream with a 4-byte length.
    appendChunk(png, "IDAT", qCompress(rows).mid(4));
    appendChunk(png, "IEND", QByteArray());
    return png;
}

int decodedWidth(const QByteArray& bytes)
{
    int32_t width = -1;
    int32_t height = -1;
    const ::rust::Vec<uint8_t> rgba = decode_image_rgba(
        ::rust::Slice<const uint8_t>(reinterpret_cast<const uint8_t*>(bytes.constData()),
                                     size_t(bytes.size())),
        width, height);
    if (rgba.size() != size_t(width) * size_t(height) * 4) {
        return -2;
    }
    return width;
}

QByteArray packedThumbnail(quint32 width, quint32 height, qsizetype bytes)
{
    QByteArray packed(8, '\0');
    qToLittleEndian<quint32>(width, packed.data());
    qToLittleEndian<quint32>(height, packed.data() + 4);
    packed.append(QByteArray(bytes, '\x7f'));
    return packed;
}

} // namespace

class UntrustedInputTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanupTestCase();
    void controlRequestsNeedTheSessionToken();
    void controlTokenFileIsOwnerOnly();
    void controlTokenFileNeverFollowsAPlantedLink();
    void qtDecodeRefusesSizesOverTheBudget();
    void thumbnailUnpackRejectsHostileSizes();
    void controlRefusesAmbiguousDocumentPaths();
    void controlRefusesASaveOverASymlink();
    void controlAcceptsAPlainAbsolutePath();

private:
    QJsonObject document(const QString& op, const QString& path);

    std::unique_ptr<pictura::test::ScopedStateHome> stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void UntrustedInputTest::initTestCase()
{
    stateHome_ = std::make_unique<pictura::test::ScopedStateHome>();
    QVERIFY(stateHome_->isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
}

void UntrustedInputTest::cleanupTestCase()
{
    window_.reset();
    stateHome_.reset();
}

// A same-user process that can reach the socket but not read the token file
// gets no open/save_as oracle (#844 M5).
void UntrustedInputTest::controlRequestsNeedTheSessionToken()
{
    pictura::ControlServer server(window_.get(), dir_.filePath(QStringLiteral("token.sock")));
    QVERIFY(server.listen());
    QLocalSocket socket;
    socket.connectToServer(server.socketPath());
    QVERIFY(socket.waitForConnected(2000));

    QJsonObject response = transact(socket, statusRequest(QJsonValue::Undefined));
    QCOMPARE(errorCode(response), QStringLiteral("unauthorized"));
    QCOMPARE(response.value(QStringLiteral("id")).toInt(), 9);

    const QString wrong(server.token().size(), QLatin1Char('0'));
    response = transact(socket, statusRequest(wrong));
    QCOMPARE(errorCode(response), QStringLiteral("unauthorized"));
    response = transact(socket, statusRequest(QStringLiteral("short")));
    QCOMPARE(errorCode(response), QStringLiteral("unauthorized"));

    response = transact(socket, statusRequest(QString::fromLatin1(server.token())));
    QVERIFY2(response.value(QStringLiteral("ok")).toBool(),
             qPrintable(QJsonDocument(response).toJson()));
}

void UntrustedInputTest::controlTokenFileIsOwnerOnly()
{
    QString tokenPath;
    QByteArray token;
    {
        pictura::ControlServer server(window_.get(), dir_.filePath(QStringLiteral("mode.sock")));
        QVERIFY(server.listen());
        tokenPath = server.tokenPath();
        token = server.token();
        QCOMPARE(token.size(), 64);
        QFile file(tokenPath);
        QVERIFY(file.open(QIODevice::ReadOnly));
        QCOMPARE(file.readAll(), token);
#ifndef _WIN32
        const QFileDevice::Permissions others = QFileDevice::ReadGroup | QFileDevice::WriteGroup
                                                | QFileDevice::ReadOther
                                                | QFileDevice::WriteOther;
        QCOMPARE(file.permissions() & others, QFileDevice::Permissions());
#endif
        pictura::ControlServer second(window_.get(), dir_.filePath(QStringLiteral("other.sock")));
        QVERIFY(second.listen());
        QVERIFY(second.token() != token);
    }
    QVERIFY2(!QFileInfo::exists(tokenPath), "shutdown removes the token file");
}

void UntrustedInputTest::controlTokenFileNeverFollowsAPlantedLink()
{
#ifdef _WIN32
    QSKIP("symlink planting is a POSIX concern");
#else
    const QString socketPath = dir_.filePath(QStringLiteral("planted.sock"));
    const QString victim = dir_.filePath(QStringLiteral("victim.txt"));
    QFile victimFile(victim);
    QVERIFY(victimFile.open(QIODevice::WriteOnly));
    victimFile.write("keep");
    victimFile.close();
    QVERIFY(QFile::link(victim, socketPath + QStringLiteral(".token")));

    pictura::ControlServer server(window_.get(), socketPath);
    QVERIFY(server.listen());
    QVERIFY(!QFileInfo(server.tokenPath()).isSymLink());
    QVERIFY(victimFile.open(QIODevice::ReadOnly));
    QCOMPARE(victimFile.readAll(), QByteArray("keep"));
#endif
}

// Qt decoded up to its 4 GiB allocation limit before the engine's 2 GiB budget
// saw the size (#844 H9).
void UntrustedInputTest::qtDecodeRefusesSizesOverTheBudget()
{
    QCOMPARE(decodedWidth(pngImage(29999, 1)), 29999);
    QCOMPARE(decodedWidth(pngImage(30001, 1)), 0);
    QCOMPARE(decodedWidth(pngImage(1, 30001)), 0);

    QImage small(3, 2, QImage::Format_RGBA8888);
    small.fill(Qt::red);
    QByteArray encoded;
    QBuffer buffer(&encoded);
    QVERIFY(buffer.open(QIODevice::WriteOnly));
    QVERIFY(small.save(&buffer, "PNG"));
    QCOMPARE(decodedWidth(encoded), 3);
}

// `w * h * 4` overflowed and the QImage was built from a short buffer (#844 M8).
void UntrustedInputTest::thumbnailUnpackRejectsHostileSizes()
{
    QCOMPARE(pictura::unpackThumbnails(packedThumbnail(2, 2, 16)).size(), 1);
    QVERIFY(pictura::unpackThumbnails(packedThumbnail(2, 2, 15)).isEmpty());
    QVERIFY(pictura::unpackThumbnails(packedThumbnail(0xFFFFFFFFu, 0xFFFFFFFFu, 16)).isEmpty());
    QVERIFY(pictura::unpackThumbnails(packedThumbnail(0x40000000u, 4, 16)).isEmpty());
    QVERIFY(pictura::unpackThumbnails(packedThumbnail(40000, 1, 16)).isEmpty());
    QVERIFY(pictura::unpackThumbnails(QByteArray(7, '\x01')).isEmpty());
}

QJsonObject UntrustedInputTest::document(const QString& op, const QString& path)
{
    pictura::ControlServer server(window_.get(), QString());
    return server.dispatch(QStringLiteral("document"),
                           {{QStringLiteral("op"), op}, {QStringLiteral("path"), path}});
}

void UntrustedInputTest::controlRefusesAmbiguousDocumentPaths()
{
    const int count = window_->documentCount();
    for (const QString& path : {QStringLiteral("relative.psd"), QStringLiteral("/tmp/a\nb.psd"),
                                dir_.filePath(QStringLiteral("x/../y.psd"))}) {
        const QJsonObject response = document(QStringLiteral("open"), path);
        QCOMPARE(errorCode(response), QStringLiteral("invalid_param"));
    }
    QCOMPARE(window_->documentCount(), count);
}

void UntrustedInputTest::controlRefusesASaveOverASymlink()
{
    QVERIFY(window_->newDocument(QStringLiteral("Paths"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const QString victim = dir_.filePath(QStringLiteral("victim.txt"));
    const QString link = dir_.filePath(QStringLiteral("link.psd"));
    QFile file(victim);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("keep");
    file.close();
    QVERIFY(QFile::link(victim, link));

    const QJsonObject response = document(QStringLiteral("save_as"), link);
    QCOMPARE(errorCode(response), QStringLiteral("invalid_param"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    QCOMPARE(file.readAll(), QByteArray("keep"));
    QVERIFY(QFileInfo(link).isSymLink());
}

void UntrustedInputTest::controlAcceptsAPlainAbsolutePath()
{
    QVERIFY(window_->newDocument(QStringLiteral("Plain"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const QString path = dir_.filePath(QStringLiteral("plain.psd"));
    QVERIFY(document(QStringLiteral("save_as"), path).value(QStringLiteral("ok")).toBool());
    QVERIFY(QFileInfo(path).isFile());
    QVERIFY(document(QStringLiteral("open"), path).value(QStringLiteral("ok")).toBool());
}

QTEST_MAIN(UntrustedInputTest)
#include "tst_untrusted_input.moc"
