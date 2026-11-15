// File > Save for Web & Devices (#61, #629).

#include <QtTest/QtTest>

#include "api/engine_bridge.h"
#include "api/transform_bridge.h"
#include "commands.h"
#include "frame.h"
#include "save_for_web_dialog.h"
#include "save_optimized_dialog.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QWidget>

#include <cstdint>

#include "qt_test_support.h"

namespace {

// A 64 x 48 colour field with a clear 8 x 8 top-left corner.
QImage picture()
{
    QImage image(64, 48, QImage::Format_RGBA8888);
    for (int y = 0; y < 48; ++y) {
        for (int x = 0; x < 64; ++x) {
            const int alpha = (x < 8 && y < 8) ? 0 : 255;
            image.setPixel(x, y, qRgba(x * 4, y * 5, (x + y) * 2, alpha));
        }
    }
    return image;
}

QImage opaquePicture()
{
    QImage image(8, 8, QImage::Format_RGB32);
    image.fill(qRgb(200, 20, 20));
    return image;
}

template <typename T>
T* child(QObject& parent, const char* name)
{
    return parent.findChild<T*>(QString::fromLatin1(name));
}

int distinctColors(const QImage& image)
{
    QSet<QRgb> seen;
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            seen.insert(image.pixel(x, y));
        }
    }
    return int(seen.size());
}

::rust::Vec<std::uint8_t> rustBytes(const QByteArray& data)
{
    ::rust::Vec<std::uint8_t> out;
    for (char byte : data) {
        out.push_back(static_cast<std::uint8_t>(byte));
    }
    return out;
}

pictura::WebMetadata metadataWith(const QByteArray& exif = QByteArray(),
                                  const QByteArray& iptc = QByteArray(),
                                  const QByteArray& xmp = QByteArray())
{
    pictura::WebMetadata metadata;
    metadata.exif = rustBytes(exif);
    metadata.iptc = rustBytes(iptc);
    metadata.xmp = rustBytes(xmp);
    return metadata;
}

QVector<pictura::ExportSlice> twoSlices()
{
    return {{0, 0, 32, 48, 1, 0}, {32, 0, 32, 48, 2, 1}};
}

QByteArray fileBytes(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return QByteArray();
    }
    return file.readAll();
}

} // namespace

class SaveForWebTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void formats();
    void sizePresetsSaveAndDone();
    void dialogSurfaces();
    void fourUp();
    void repopulate();
    void colorTableEdit();
    void colorTableTransparency();
    void colorTableUnmap();
    void colorBudget();
    void colorTableSortAct();
    void metadataAllNone();
    void metadataFormats();
    void metadataRemember();
    void sliceImages();
    void sliceImageSize();
    void htmlTable();
    void htmlOnly();
    void cssLayout();
    void saveFormats();
    void saveCancel();
    void unslicedSave();
    void menuCommand();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SaveForWebTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SaveForWebTest::formats()
{
    pictura::SaveForWebDialog dialog(picture());
    auto* format = child<QComboBox>(dialog, "saveForWebFormat");
    QVERIFY(format);

    // The first open is GIF 128 Dithered: a GIF that decodes to the image's
    // size, its clear corner still clear.
    QCOMPARE(child<QComboBox>(dialog, "saveForWebPreset")->currentText(),
             QStringLiteral("GIF 128 Dithered"));
    QByteArray gif = dialog.encoded();
    QVERIFY(gif.startsWith("GIF89a"));
    QImage decoded = QImage::fromData(gif).convertToFormat(QImage::Format_RGBA8888);
    QCOMPARE(decoded.size(), QSize(64, 48));
    QCOMPARE(qAlpha(decoded.pixel(2, 2)), 0);
    QCOMPARE(qAlpha(decoded.pixel(30, 30)), 255);
    QVERIFY(child<QLabel>(dialog, "saveForWebOptimizedInfo")->text().contains(
        QStringLiteral("sec @ 56.6 Kbps")));
    child<QCheckBox>(dialog, "saveForWebInterlaced")->setChecked(true);
    QVERIFY(dialog.encoded() != gif);

    // JPEG: quality trades size.
    format->setCurrentIndex(pictura::SaveForWebDialog::Jpeg);
    auto* quality = child<QSpinBox>(dialog, "saveForWebQuality");
    quality->setValue(10);
    const QByteArray low = dialog.encoded();
    quality->setValue(100);
    const QByteArray high = dialog.encoded();
    QVERIFY(low.startsWith("\xff\xd8") && high.startsWith("\xff\xd8"));
    QVERIFY(low.size() < high.size());
    QCOMPARE(dialog.suffix(), QStringLiteral("jpg"));

    // PNG-8 keeps to its colour table (plus the clear entry).
    format->setCurrentIndex(pictura::SaveForWebDialog::Png8);
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(16);
    const QImage png8 = QImage::fromData(dialog.encoded());
    QCOMPARE(png8.size(), QSize(64, 48));
    QVERIFY2(distinctColors(png8) <= 17, qPrintable(QString::number(distinctColors(png8))));

    // PNG-24 without transparency lays the corner over the white matte.
    format->setCurrentIndex(pictura::SaveForWebDialog::Png24);
    child<QCheckBox>(dialog, "saveForWebPngTransparency")->setChecked(false);
    const QImage png24 = QImage::fromData(dialog.encoded());
    QCOMPARE(png24.pixel(2, 2), qRgb(255, 255, 255));

    // WBMP: a type-0 header and one bit per pixel.
    format->setCurrentIndex(pictura::SaveForWebDialog::Wbmp);
    const QByteArray wbmp = dialog.encoded();
    QCOMPARE(QByteArray(wbmp.constData(), 4), QByteArray("\x00\x00\x40\x30", 4));
    QCOMPARE(wbmp.size(), 4 + 8 * 48);
    QCOMPARE(dialog.suffix(), QStringLiteral("wbmp"));
}

void SaveForWebTest::sizePresetsSaveAndDone()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    {
        pictura::SaveForWebDialog dialog(picture());
        // Image Size: 50 % halves both sides, proportions kept.
        child<QSpinBox>(dialog, "saveForWebPercent")->setValue(50);
        QCOMPARE(child<QSpinBox>(dialog, "saveForWebWidth")->value(), 32);
        QCOMPARE(dialog.sized().size(), QSize(32, 24));
        QCOMPARE(QImage::fromData(dialog.encoded()).size(), QSize(32, 24));

        // A preset sets the format and its settings.
        child<QComboBox>(dialog, "saveForWebPreset")->setCurrentText(QStringLiteral("JPEG Low"));
        QCOMPARE(child<QComboBox>(dialog, "saveForWebFormat")->currentIndex(),
                 int(pictura::SaveForWebDialog::Jpeg));
        QCOMPARE(child<QSpinBox>(dialog, "saveForWebQuality")->value(), 10);

        // Save writes exactly the optimised file.
        const QString path = dir.filePath(QStringLiteral("web.jpg"));
        QVERIFY(dialog.saveTo(path));
        QCOMPARE(fileBytes(path), dialog.encoded());

        // Done keeps the settings for the next open.
        child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Png24);
        child<QPushButton>(dialog, "saveForWebDone")->click();
    }
    pictura::SaveForWebDialog again(picture());
    QCOMPARE(child<QComboBox>(again, "saveForWebFormat")->currentIndex(),
             int(pictura::SaveForWebDialog::Png24));
    QCOMPARE(child<QComboBox>(again, "saveForWebPreset")->currentIndex(), 0);
}

void SaveForWebTest::dialogSurfaces()
{
    pictura::SaveForWebDialog dialog(picture());
    auto* tabs = child<QTabBar>(dialog, "saveForWebTabs");
    QVERIFY(tabs);
    for (int i = 0; i < 4; ++i) {
        QVERIFY(tabs->isTabEnabled(i));
    }
    auto* format = child<QComboBox>(dialog, "saveForWebFormat");
    auto* table = child<QWidget>(dialog, "saveForWebColorTable");
    auto* metadata = child<QComboBox>(dialog, "saveForWebMetadata");
    QVERIFY(table);
    QVERIFY(metadata);
    format->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    QVERIFY(!table->isHidden());
    format->setCurrentIndex(pictura::SaveForWebDialog::Png8);
    QVERIFY(!table->isHidden());
    format->setCurrentIndex(pictura::SaveForWebDialog::Jpeg);
    QVERIFY(table->isHidden());
    format->setCurrentIndex(pictura::SaveForWebDialog::Png24);
    QVERIFY(table->isHidden());
}

void SaveForWebTest::fourUp()
{
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    const int otherQuality = dialog.paneSettingsForTest(1).quality;
    child<QTabBar>(dialog, "saveForWebTabs")->setCurrentIndex(3);
    dialog.show();
    auto* pane3 = child<QLabel>(dialog, "saveForWebPane3");
    QVERIFY(pane3);
    QTest::mouseClick(pane3, Qt::LeftButton);
    QCOMPARE(dialog.activePaneForTest(), 2);
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Jpeg);
    child<QSpinBox>(dialog, "saveForWebQuality")->setValue(10);
    QCOMPARE(dialog.paneSettingsForTest(2).format, int(pictura::SaveForWebDialog::Jpeg));
    QCOMPARE(dialog.paneSettingsForTest(2).quality, 10);
    QCOMPARE(dialog.paneSettingsForTest(1).format, int(pictura::SaveForWebDialog::Gif));
    QCOMPARE(dialog.paneSettingsForTest(1).quality, otherQuality);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("pane.jpg"));
    QVERIFY(dialog.saveTo(path));
    QVERIFY(fileBytes(path).startsWith("\xff\xd8"));
}

void SaveForWebTest::repopulate()
{
    pictura::SaveForWebDialog dialog(picture());
    child<QTabBar>(dialog, "saveForWebTabs")->setCurrentIndex(3);
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(128);
    dialog.repopulateForTest();
    QVERIFY(dialog.paneSettingsForTest(2).colors <= 128);
    QVERIFY(dialog.paneSettingsForTest(3).colors <= dialog.paneSettingsForTest(2).colors);

    // An active non-default pane keeps its own settings; only the others lower.
    dialog.setActivePaneForTest(2);
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(128);
    dialog.repopulateForTest();
    QCOMPARE(dialog.paneSettingsForTest(2).colors, 128);
    QVERIFY(dialog.paneSettingsForTest(1).colors <= 128);
    QVERIFY(dialog.paneSettingsForTest(3).colors <= 128);

    // A custom table does not survive Repopulate: the lowered panes re-reduce.
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(16);
    dialog.editSwatchForTest(0, QColor(Qt::red));
    QVERIFY(!dialog.paneSettingsForTest(2).custom.isEmpty());
    dialog.repopulateForTest();
    QVERIFY(dialog.paneSettingsForTest(1).custom.isEmpty());
    QVERIFY(dialog.paneSettingsForTest(3).custom.isEmpty());
}

void SaveForWebTest::colorTableEdit()
{
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(16);
    QVERIFY(!dialog.colorTableForTest().isEmpty());
    // Shift a swatch that the image actually uses and that is opaque. The GIF
    // decoder may return ARGB32 rather than Indexed8, so match the decoded
    // colour against the dialog's palette instead of reading palette indices.
    const QImage before =
        QImage::fromData(dialog.encoded()).convertToFormat(QImage::Format_RGBA8888);
    const QVector<QRgb> table = dialog.colorTableForTest();
    int used = -1;
    for (int y = 0; y < before.height() && used < 0; ++y) {
        for (int x = 0; x < before.width() && used < 0; ++x) {
            const QRgb pixel = before.pixel(x, y);
            if (qAlpha(pixel) == 0) {
                continue;
            }
            for (int i = 0; i < table.size(); ++i) {
                if (qRgba(qRed(table[i]), qGreen(table[i]), qBlue(table[i]), 255)
                    == pixel) {
                    used = i;
                    break;
                }
            }
        }
    }
    QVERIFY(used >= 0);
    dialog.editSwatchForTest(used, QColor(Qt::red));
    const QByteArray gif = dialog.encoded();
    const QImage decoded = QImage::fromData(gif).convertToFormat(QImage::Format_RGBA8888);
    QVERIFY(!decoded.isNull());
    bool red = false;
    for (int y = 0; y < decoded.height() && !red; ++y) {
        for (int x = 0; x < decoded.width() && !red; ++x) {
            red = decoded.pixel(x, y) == qRgba(255, 0, 0, 255);
        }
    }
    QVERIFY(red);
    const QVector<QRgb> edited = dialog.colorTableForTest();
    QCOMPARE(edited[used], QColor(Qt::red).rgba());
    // Changing Colors drops the custom table so the reduction takes effect.
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(8);
    QVERIFY(dialog.colorTableForTest() != edited);
}

void SaveForWebTest::colorTableTransparency()
{
    pictura::SaveForWebDialog dialog(opaquePicture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(16);
    const QVector<QRgb> table = dialog.colorTableForTest();
    QVERIFY(!table.isEmpty());
    const QRgb swatch = qRgb(qRed(table.first()), qGreen(table.first()), qBlue(table.first()));
    QCOMPARE(swatch, qRgb(200, 20, 20));
    dialog.mapSwatchForTest(0, true);
    const QImage decoded =
        QImage::fromData(dialog.encoded()).convertToFormat(QImage::Format_RGBA8888);
    QVERIFY(!decoded.isNull());
    QCOMPARE(decoded.size(), QSize(8, 8));
    // The picture was solid in the mapped swatch, so every pixel is transparent.
    for (int y = 0; y < decoded.height(); ++y) {
        for (int x = 0; x < decoded.width(); ++x) {
            QCOMPARE(qAlpha(decoded.pixel(x, y)), 0);
        }
    }
}

void SaveForWebTest::colorTableUnmap()
{
    pictura::SaveForWebDialog dialog(opaquePicture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    dialog.mapSwatchForTest(0, true);
    dialog.mapSwatchForTest(0, false);
    const QVector<QRgb> table = dialog.colorTableForTest();
    QVERIFY(!table.isEmpty());
    QCOMPARE(qAlpha(table.first()), 255);
    const QByteArray gif = dialog.encoded();
    QVERIFY(!gif.contains(QByteArray("\x21\xf9", 2)));
    const QImage decoded = QImage::fromData(gif).convertToFormat(QImage::Format_RGBA8888);
    QVERIFY(!decoded.isNull());
    for (int y = 0; y < decoded.height(); ++y) {
        for (int x = 0; x < decoded.width(); ++x) {
            QCOMPARE(qAlpha(decoded.pixel(x, y)), 255);
        }
    }
}

void SaveForWebTest::colorBudget()
{
    pictura::SaveForWebDialog dialog(opaquePicture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    for (int i = 0; i < 256; ++i) {
        dialog.addSwatchForTest(QColor(i, (i * 7) % 256, (i * 13) % 256));
    }
    const int pane = dialog.activePaneForTest();
    QCOMPARE(int(dialog.paneSettingsForTest(pane).locked.size()), 256);
    dialog.addSwatchForTest(QColor(Qt::red));
    QCOMPARE(int(dialog.paneSettingsForTest(pane).locked.size()), 256);
}

void SaveForWebTest::colorTableSortAct()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    child<QSpinBox>(dialog, "saveForWebColors")->setValue(16);
    dialog.sortSwatchesForTest(1);
    const QVector<QRgb> sorted = dialog.colorTableForTest();
    const QString path = dir.filePath(QStringLiteral("table.act"));
    QVERIFY(dialog.saveColorTableForTest(path));
    QVERIFY(dialog.loadColorTableForTest(path));
    QCOMPARE(dialog.colorTableForTest().size(), sorted.size());
}

void SaveForWebTest::metadataAllNone()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QByteArray iptc = QByteArray("\x1c\x02\x74\x00\x09(c) Kooka", 14);
    const QByteArray xmp = "<x:xmpmeta/>";
    const QByteArray exif = QByteArray("II*\0exif", 7);
    pictura::SaveForWebDialog dialog(picture(), metadataWith(exif, iptc, xmp));
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Jpeg);
    child<QComboBox>(dialog, "saveForWebMetadata")->setCurrentIndex(4);
    const QString allPath = dir.filePath(QStringLiteral("all.jpg"));
    QVERIFY(dialog.saveTo(allPath));
    const QByteArray all = fileBytes(allPath);
    QVERIFY(all.contains("Exif\0\0"));
    QVERIFY(all.contains("http://ns.adobe.com/xap/1.0/"));
    QVERIFY(all.contains("Photoshop 3.0\0"));
    child<QComboBox>(dialog, "saveForWebMetadata")->setCurrentIndex(0);
    const QString nonePath = dir.filePath(QStringLiteral("none.jpg"));
    QVERIFY(dialog.saveTo(nonePath));
    const QByteArray none = fileBytes(nonePath);
    QVERIFY(!none.contains("Exif\0\0"));
    QVERIFY(!none.contains("http://ns.adobe.com/xap/1.0/"));
    QVERIFY(!none.contains("Photoshop 3.0\0"));
}

void SaveForWebTest::metadataFormats()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QByteArray xmp = "<x:xmpmeta/>";
    pictura::SaveForWebDialog dialog(picture(), metadataWith(QByteArray(), QByteArray(), xmp));
    child<QComboBox>(dialog, "saveForWebMetadata")->setCurrentIndex(4);
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Png24);
    const QString pngPath = dir.filePath(QStringLiteral("all.png"));
    QVERIFY(dialog.saveTo(pngPath));
    QVERIFY(fileBytes(pngPath).contains("XML:com.adobe.xmp"));
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    const QString gifPath = dir.filePath(QStringLiteral("all.gif"));
    QVERIFY(dialog.saveTo(gifPath));
    QVERIFY(!fileBytes(gifPath).contains("XML:com.adobe.xmp"));
}

void SaveForWebTest::metadataRemember()
{
    {
        pictura::SaveForWebDialog dialog(picture());
        child<QComboBox>(dialog, "saveForWebMetadata")->setCurrentIndex(4);
        child<QPushButton>(dialog, "saveForWebDone")->click();
    }
    pictura::SaveForWebDialog again(picture());
    QCOMPARE(child<QComboBox>(again, "saveForWebMetadata")->currentIndex(), 4);
}

void SaveForWebTest::sliceImages()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    const QVector<pictura::SlicePlacement> cells = pictura::SaveForWebDialog::buildSliceCells(
        twoSlices(), QSize(64, 48), QSize(64, 48), QStringLiteral("doc"), QStringLiteral("gif"));
    QCOMPARE(cells.size(), 2);
    const QString path = dir.filePath(QStringLiteral("doc.html"));
    QVERIFY(dialog.writeSlices(path, 0, 0, cells, QStringLiteral("doc")));
    const QImage first = QImage::fromData(fileBytes(dir.filePath(QStringLiteral("images/doc_01.gif"))));
    const QImage second = QImage::fromData(fileBytes(dir.filePath(QStringLiteral("images/doc_02.gif"))));
    QCOMPARE(first.size(), QSize(32, 48));
    QCOMPARE(second.size(), QSize(32, 48));

    // PNG-24 is lossless: a slice must equal the matching region of the
    // image the dialog sized.
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Png24);
    const QVector<pictura::SlicePlacement> exactCells = pictura::SaveForWebDialog::buildSliceCells(
        twoSlices(), QSize(64, 48), QSize(64, 48), QStringLiteral("exact"),
        QStringLiteral("png"));
    const QString exactPath = dir.filePath(QStringLiteral("exact.html"));
    QVERIFY(dialog.writeSlices(exactPath, 1, 0, exactCells, QStringLiteral("exact")));
    const QImage exact = QImage::fromData(fileBytes(dir.filePath(QStringLiteral("images/exact_01.png"))))
                             .convertToFormat(QImage::Format_RGBA8888);
    const QImage expected =
        dialog.sized().copy(0, 0, 32, 48).convertToFormat(QImage::Format_RGBA8888);
    QCOMPARE(exact.size(), QSize(32, 48));
    for (int y = 0; y < expected.height(); ++y) {
        for (int x = 0; x < expected.width(); ++x) {
            if (qAlpha(expected.pixel(x, y)) == 0) {
                continue; // the clear corner's RGB is not meaningful
            }
            QCOMPARE(exact.pixel(x, y), expected.pixel(x, y));
        }
    }
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);

    // One user slice smaller than the canvas: auto slices fill the rest.
    auto local = pictura::test::makeMainWindow();
    QVERIFY(local != nullptr);
    QVERIFY(local->newDocument(QStringLiteral("Auto"), 64, 48, QStringLiteral("rgb"), 8,
                               QStringLiteral("white")));
    pictura::PictureView* view = local->activeView();
    QVERIFY(view);
    QVERIFY(pictura::add_user_slice(*view, 0, 0, 32, 24) >= 0);
    QVERIFY(pictura::web_has_user_slices(*view));
    QVector<pictura::ExportSlice> resolved;
    const int count = pictura::web_export_slice_count(*view);
    QVERIFY(count > 1);
    for (int i = 0; i < count; ++i) {
        const ::rust::Vec<std::int32_t> f = pictura::web_export_slice_at(*view, i);
        QVERIFY(f.size() == 6);
        resolved.append({f[0], f[1], f[2], f[3], f[4], f[5]});
    }
    bool userImage = false;
    bool autoImage = false;
    int area = 0;
    for (const pictura::ExportSlice& slice : resolved) {
        area += slice.width * slice.height;
        userImage |= slice.userIndex >= 0;
        autoImage |= slice.userIndex < 0;
    }
    QVERIFY(userImage);
    QVERIFY(autoImage);
    QCOMPARE(area, 64 * 48);
    const QVector<pictura::SlicePlacement> autoCells = pictura::SaveForWebDialog::buildSliceCells(
        resolved, QSize(64, 48), QSize(64, 48), QStringLiteral("auto"), QStringLiteral("gif"));
    const QString autoPath = dir.filePath(QStringLiteral("auto.html"));
    QVERIFY(dialog.writeSlices(autoPath, 0, 0, autoCells, QStringLiteral("auto")));
    for (const pictura::ExportSlice& slice : resolved) {
        const QString name =
            QStringLiteral("images/auto_%1.gif").arg(slice.number, 2, 10, QLatin1Char('0'));
        QVERIFY2(QFile::exists(dir.filePath(name)), qPrintable(name));
        QCOMPARE(QImage::fromData(fileBytes(dir.filePath(name))).size(),
                 QSize(slice.width, slice.height));
    }
}

void SaveForWebTest::sliceImageSize()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    child<QSpinBox>(dialog, "saveForWebPercent")->setValue(50);
    const QVector<pictura::SlicePlacement> cells = pictura::SaveForWebDialog::buildSliceCells(
        twoSlices(), QSize(64, 48), dialog.sized().size(), QStringLiteral("half"),
        QStringLiteral("gif"));
    QCOMPARE(cells.size(), 2);
    const QString path = dir.filePath(QStringLiteral("half.html"));
    QVERIFY(dialog.writeSlices(path, 1, 0, cells, QStringLiteral("half")));
    const QImage first = QImage::fromData(fileBytes(dir.filePath(QStringLiteral("images/half_01.gif"))));
    QCOMPARE(first.size(), QSize(16, 24));
}

void SaveForWebTest::htmlTable()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    const QVector<pictura::SlicePlacement> cells = pictura::SaveForWebDialog::buildSliceCells(
        twoSlices(), QSize(64, 48), QSize(64, 48), QStringLiteral("doc"), QStringLiteral("gif"));
    const QString path = dir.filePath(QStringLiteral("doc.html"));
    QVERIFY(dialog.writeSlices(path, 0, 0, cells, QStringLiteral("doc")));
    const QString html = QString::fromUtf8(fileBytes(path));
    QVERIFY(html.contains(QStringLiteral("<table id=\"Table_01\"")));
    QVERIFY(html.contains(QStringLiteral("images/doc_01.gif")));
    QVERIFY(html.contains(QStringLiteral("images/doc_02.gif")));
}

void SaveForWebTest::htmlOnly()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    const QVector<pictura::SlicePlacement> cells = pictura::SaveForWebDialog::buildSliceCells(
        twoSlices(), QSize(64, 48), QSize(64, 48), QStringLiteral("only"), QStringLiteral("gif"));
    const QString path = dir.filePath(QStringLiteral("only.html"));
    QVERIFY(dialog.writeSlices(path, 2, 0, cells, QStringLiteral("only")));
    QVERIFY(QFile::exists(path));
    QVERIFY(!QDir(dir.filePath(QStringLiteral("images"))).exists());
}

void SaveForWebTest::cssLayout()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    const QVector<pictura::SlicePlacement> cells = pictura::SaveForWebDialog::buildSliceCells(
        twoSlices(), QSize(64, 48), QSize(64, 48), QStringLiteral("css"), QStringLiteral("gif"));
    const QString path = dir.filePath(QStringLiteral("css.html"));
    QVERIFY(dialog.writeSlices(path, 0, 1, cells, QStringLiteral("css")));
    const QString html = QString::fromUtf8(fileBytes(path));
    QVERIFY(!html.contains(QStringLiteral("<table")));
    QVERIFY(html.contains(QStringLiteral("position:absolute")));
    QVERIFY(html.contains(QStringLiteral("left:32px")));
}

void SaveForWebTest::saveFormats()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    child<QComboBox>(dialog, "saveForWebFormat")->setCurrentIndex(pictura::SaveForWebDialog::Gif);
    const QVector<pictura::SlicePlacement> cells = pictura::SaveForWebDialog::buildSliceCells(
        twoSlices(), QSize(64, 48), QSize(64, 48), QStringLiteral("fmt"), QStringLiteral("gif"));
    const QString both = dir.filePath(QStringLiteral("both.html"));
    QVERIFY(dialog.writeSlices(both, 0, 0, cells, QStringLiteral("fmt")));
    QVERIFY(QFile::exists(both));
    QVERIFY(QFile::exists(dir.filePath(QStringLiteral("images/fmt_01.gif"))));

    QTemporaryDir imagesOnly;
    QVERIFY(imagesOnly.isValid());
    const QString imagesPath = imagesOnly.filePath(QStringLiteral("images.html"));
    QVERIFY(dialog.writeSlices(imagesPath, 1, 0, cells, QStringLiteral("fmt")));
    QVERIFY(!QFile::exists(imagesPath));
    QVERIFY(QFile::exists(imagesOnly.filePath(QStringLiteral("images/fmt_01.gif"))));

    QTemporaryDir htmlOnly;
    QVERIFY(htmlOnly.isValid());
    const QString htmlPath = htmlOnly.filePath(QStringLiteral("html.html"));
    QVERIFY(dialog.writeSlices(htmlPath, 2, 0, cells, QStringLiteral("fmt")));
    QVERIFY(QFile::exists(htmlPath));
    QVERIFY(!QDir(htmlOnly.filePath(QStringLiteral("images"))).exists());
}

void SaveForWebTest::saveCancel()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("cancel.html"));
    pictura::SaveOptimizedDialog dialog(path);
    dialog.reject();
    QCOMPARE(dialog.result(), int(QDialog::Rejected));
    QVERIFY(!QFile::exists(path));
}

void SaveForWebTest::unslicedSave()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::SaveForWebDialog dialog(picture());
    const QString path = dir.filePath(QStringLiteral("single.gif"));
    QVERIFY(dialog.saveTo(path));
    QVERIFY(QFile::exists(path));
    QVERIFY(!QFile::exists(dir.filePath(QStringLiteral("single.html"))));
    QVERIFY(!QDir(dir.filePath(QStringLiteral("images"))).exists());
}

void SaveForWebTest::menuCommand()
{
    pictura::CommandRegistry* registry = window_->registry();
    QAction* action = registry->action(QString::fromLatin1(pictura::command_ids::FileSaveForWeb));
    QVERIFY(action);
    QCOMPARE(action->shortcut(), QKeySequence(QStringLiteral("Ctrl+Alt+Shift+S")));
    registry->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(window_->newDocument(QStringLiteral("Web"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    registry->refresh();
    QVERIFY(action->isEnabled());
}

QTEST_MAIN(SaveForWebTest)
#include "tst_save_for_web.moc"
