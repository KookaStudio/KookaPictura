// Raster import: the embedded ICC profile is applied under the incoming policy
// and Place converts to the host's sRGB working space.

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>

#include "color_settings_dialog.h"
#include "frame.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

namespace {

template <typename T>
T* findTopLevel(const char* name)
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<T*>(widget); dialog && dialog->objectName() == name) {
            return dialog;
        }
    }
    return nullptr;
}

void append32(QByteArray& out, quint32 value)
{
    out.append(char((value >> 24) & 0xFF));
    out.append(char((value >> 16) & 0xFF));
    out.append(char((value >> 8) & 0xFF));
    out.append(char(value & 0xFF));
}

quint32 crc32(const QByteArray& data)
{
    quint32 crc = 0xFFFFFFFFu;
    for (char byte : data) {
        crc ^= quint8(byte);
        for (int bit = 0; bit < 8; ++bit) {
            crc = (crc >> 1) ^ (0xEDB88320u & (0u - (crc & 1u)));
        }
    }
    return ~crc;
}

QByteArray pngChunk(const char* type, const QByteArray& data)
{
    QByteArray out;
    append32(out, quint32(data.size()));
    QByteArray body(type);
    body.append(data);
    out.append(body);
    append32(out, crc32(body));
    return out;
}

QByteArray pngWithIccp(QByteArray png, const QByteArray& profile)
{
    QByteArray iccp(QStringLiteral("ICC").toUtf8());
    iccp.append('\0');
    iccp.append('\0');
    const QByteArray compressed = qCompress(profile, 9).mid(4);
    iccp.append(compressed);
    if (png.size() < 33) {
        return {};
    }
    png.insert(33, pngChunk("iCCP", iccp));
    return png;
}

} // namespace

class ImageImportTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void jpegPreserve();
    void tiffConvert();
    void pngOff();
    void untagged();
    void malformedProfileIgnored();
    void placeConvertsProfile();
    void profileNotice();

private:
    pictura::PictureView* newFilled(const QColor& color, bool adobe);
    QString makeEmbedded(const QString& name, const char* format, bool adobe);
    QString makeAdundantPng(const QString& name, bool malformed);
    void setPolicy(int index);
    void closeActive() { window_->closeDocument(window_->activeDocumentIndex(), false); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void ImageImportTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

pictura::PictureView* ImageImportTest::newFilled(const QColor& color, bool adobe)
{
    if (!window_->newDocument(QStringLiteral("Import"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, color.rgba(), -1, QStringLiteral("normal"), 100, false);
    view->deselect();
    if (adobe && !view->assign_profile(1)) {
        return nullptr;
    }
    return view;
}

QString ImageImportTest::makeEmbedded(const QString& name, const char* format, bool adobe)
{
    pictura::PictureView* view = newFilled(QColor(30, 120, 220), adobe);
    if (!view) {
        return {};
    }
    pictura::set_save_embed_profile(*view, true);
    const QString path = dir_.filePath(name);
    const bool saved = view->save(path);
    closeActive();
    return saved ? path : QString();
}

QString ImageImportTest::makeAdundantPng(const QString& name, bool malformed)
{
    QImage image(8, 8, QImage::Format_RGB32);
    image.fill(QColor(30, 120, 220));
    const QString base = dir_.filePath(QStringLiteral("base.png"));
    if (!image.save(base, "PNG")) {
        return {};
    }
    QFile file(base);
    if (!file.open(QIODevice::ReadOnly)) {
        return {};
    }
    QByteArray png = file.readAll();
    QByteArray profile;
    if (malformed) {
        profile = QByteArray(64, char(0xAA));
    } else {
        const auto bytes = pictura::builtin_profile_icc(1);
        profile = QByteArray(reinterpret_cast<const char*>(bytes.data()), int(bytes.size()));
    }
    png = pngWithIccp(png, profile);
    const QString path = dir_.filePath(name);
    QFile out(path);
    if (png.isEmpty() || !out.open(QIODevice::WriteOnly) || out.write(png) != png.size()) {
        return {};
    }
    return path;
}

void ImageImportTest::setPolicy(int index)
{
    auto* timer = new QTimer(this);
    timer->setInterval(10);
    connect(timer, &QTimer::timeout, this, [timer, index]() {
        auto* dialog = findTopLevel<pictura::ColorSettingsDialog>("colorSettingsDialog");
        if (!dialog) {
            return;
        }
        timer->stop();
        dialog->setPolicyForTest(index);
        dialog->accept();
        timer->deleteLater();
    });
    timer->start();
    QTimer::singleShot(5000, timer, [timer]() {
        timer->stop();
        timer->deleteLater();
    });
    window_->showColorSettings();
    timer->stop();
    timer->deleteLater();
}

void ImageImportTest::jpegPreserve()
{
    const QString path = makeEmbedded(QStringLiteral("preserve.jpg"), "JPG", true);
    QVERIFY(!path.isEmpty());
    const QImage raw(path);
    QVERIFY(!raw.isNull());
    setPolicy(1);
    QVERIFY(window_->openImagePath(path));
    QCOMPARE(pictura::document_profile_name(*window_->activeView()), QStringLiteral("RGB built-in"));
    QVERIFY2(window_->activeView()->open_notice().contains(QStringLiteral("preserved")),
             qPrintable(window_->activeView()->open_notice()));
    QCOMPARE(window_->activeView()->sample_argb(2, 2) & 0xFFFFFFu, raw.pixel(2, 2) & 0xFFFFFFu);
    closeActive();
}

void ImageImportTest::tiffConvert()
{
    const QString path = makeEmbedded(QStringLiteral("convert.tif"), "TIF", true);
    QVERIFY(!path.isEmpty());
    const QImage raw(path);
    QVERIFY(!raw.isNull());
    setPolicy(2);
    QVERIFY(window_->openImagePath(path));
    QVERIFY(pictura::document_profile_name(*window_->activeView()).contains(QStringLiteral("sRGB")));
    QVERIFY(window_->activeView()->sample_argb(2, 2) != raw.pixel(2, 2));
    closeActive();
}

void ImageImportTest::pngOff()
{
    const QString path = makeAdundantPng(QStringLiteral("off.png"), false);
    QVERIFY(!path.isEmpty());
    const QImage raw(path);
    QVERIFY(!raw.isNull());
    setPolicy(0);
    QVERIFY(window_->openImagePath(path));
    QVERIFY(pictura::document_profile_name(*window_->activeView()).contains(QStringLiteral("sRGB")));
    QCOMPARE(window_->activeView()->sample_argb(2, 2) & 0xFFFFFFu, raw.pixel(2, 2) & 0xFFFFFFu);
    closeActive();
}

void ImageImportTest::untagged()
{
    const QString path = makeEmbedded(QStringLiteral("untagged.jpg"), "JPG", false);
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openImagePath(path));
    QVERIFY(pictura::document_profile_name(*window_->activeView()).contains(QStringLiteral("sRGB")));
    closeActive();
}

void ImageImportTest::malformedProfileIgnored()
{
    const QString path = makeAdundantPng(QStringLiteral("malformed.png"), true);
    QVERIFY(!path.isEmpty());
    const QImage raw(path);
    QVERIFY(!raw.isNull());
    QVERIFY(window_->openImagePath(path));
    QCOMPARE(window_->activeView()->sample_argb(2, 2) & 0xFFFFFFu, raw.pixel(2, 2) & 0xFFFFFFu);
    closeActive();
}

void ImageImportTest::placeConvertsProfile()
{
    const QString path = makeEmbedded(QStringLiteral("place.jpg"), "JPG", true);
    QVERIFY(!path.isEmpty());
    setPolicy(2);
    QVERIFY(window_->openImagePath(path));
    const unsigned converted = window_->activeView()->sample_argb(2, 2);
    closeActive();

    QVERIFY(newFilled(QColor(255, 255, 255), false));
    pictura::PictureView* host = window_->activeView();
    const QString layer = host->place_image(path);
    QVERIFY(!layer.isEmpty());
    QCOMPARE(host->sample_argb(2, 2), converted);
    closeActive();
}

void ImageImportTest::profileNotice()
{
    const QString path = makeEmbedded(QStringLiteral("notice.jpg"), "JPG", true);
    QVERIFY(!path.isEmpty());
    setPolicy(1);
    QVERIFY(window_->openImagePath(path));
    QVERIFY2(window_->activeView()->open_notice().contains(QStringLiteral("preserved")),
             qPrintable(window_->activeView()->open_notice()));
    closeActive();
}

QTEST_MAIN(ImageImportTest)
#include "tst_image_import.moc"
