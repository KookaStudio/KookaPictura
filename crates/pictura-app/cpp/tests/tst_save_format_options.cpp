// Save As per-format options dialogs (#958, #625; interop/save-format-options):
// the dispatch by suffix, the CS6 controls and defaults, the options the encoder
// honors, the JPEG size estimate and preview, and the remembered settings.

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtGui/QImageWriter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>

#include "bmp_options_dialog.h"
#include "api/document_api.h"
#include "frame.h"
#include "jpeg_options_dialog.h"
#include "png_options_dialog.h"
#include "save_format_options.h"
#include "session.h"
#include "tiff_options_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"
#include "pictura_app/src/cxxqt_object/save_format.cxxqt.h"

#include "qt_test_support.h"

namespace {

bool tiffWriterAvailable()
{
    const QList<QByteArray> formats = QImageWriter::supportedImageFormats();
    return formats.contains("tif") || formats.contains("tiff");
}

// The first JPEG frame marker (0xC0 baseline, 0xC2 progressive), or -1.
int jpegFrameMarker(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return -1;
    }
    const QByteArray b = file.readAll();
    int i = 2; // past SOI
    while (i + 1 < b.size()) {
        if (quint8(b[i]) != 0xFF) {
            ++i;
            continue;
        }
        const int marker = quint8(b[i + 1]);
        if (marker == 0xD8 || marker == 0x01 || (marker >= 0xD0 && marker <= 0xD7)) {
            i += 2;
            continue;
        }
        if (marker == 0xDA || i + 3 >= b.size()) {
            break;
        }
        const int length = (quint8(b[i + 2]) << 8) | quint8(b[i + 3]);
        if (marker >= 0xC0 && marker <= 0xCF && marker != 0xC4 && marker != 0xC8
            && marker != 0xCC) {
            return marker;
        }
        i += 2 + length;
    }
    return -1;
}

// The TIFF Compression tag (259), or -1.
int readTiffCompressionTag(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return -1;
    }
    const QByteArray b = file.readAll();
    if (b.size() < 8) {
        return -1;
    }
    const bool little = b[0] == 'I' && b[1] == 'I';
    const bool big = b[0] == 'M' && b[1] == 'M';
    if (!little && !big) {
        return -1;
    }
    const auto u16 = [&](int o) {
        return little ? (quint8(b[o]) | (quint8(b[o + 1]) << 8))
                      : ((quint8(b[o]) << 8) | quint8(b[o + 1]));
    };
    const auto u32 = [&](int o) {
        return little ? (quint32(quint8(b[o])) | (quint32(quint8(b[o + 1])) << 8)
                        | (quint32(quint8(b[o + 2])) << 16) | (quint32(quint8(b[o + 3])) << 24))
                      : ((quint32(quint8(b[o])) << 24) | (quint32(quint8(b[o + 1])) << 16)
                         | (quint32(quint8(b[o + 2])) << 8) | quint32(quint8(b[o + 3])));
    };
    const quint32 ifd = u32(4);
    if (ifd + 2 > quint32(b.size())) {
        return -1;
    }
    const int count = u16(int(ifd));
    for (int i = 0; i < count; ++i) {
        const int entry = int(ifd) + 2 + i * 12;
        if (entry + 12 > b.size()) {
            break;
        }
        if (u16(entry) == 259) {
            return u16(entry + 8);
        }
    }
    return -1;
}

qint64 fileSize(const QString& path)
{
    return QFileInfo(path).size();
}

} // namespace

class SaveFormatOptionsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void formatKindForSuffix();
    void dialogDefaults();
    void unsupportedControlsDisabled();
    void jpegQualityChangesSize();
    void jpegQualityMapping();
    void jpegFormatOption();
    void jpegMatte();
    void pngCompressionChangesSize();
    void tiffCompressionTag();
    void tiffTransparencyOffFlattens();
    void cancelWritesNothing();
    void rememberedSettings();
    void sizeEstimateFollowsQuality();
    void previewDoesNotMutate();

private:
    pictura::PictureView* newView(const QString& background);
    QString path(const char* name) const { return dir_.filePath(QString::fromLatin1(name)); }
    void closeActive() { window_->closeDocument(window_->activeDocumentIndex(), false); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void SaveFormatOptionsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

pictura::PictureView* SaveFormatOptionsTest::newView(const QString& background)
{
    if (!window_->newDocument(QStringLiteral("Format"), 32, 32, QStringLiteral("rgb"), 8,
                              background)) {
        return nullptr;
    }
    return window_->activeView();
}

void SaveFormatOptionsTest::formatKindForSuffix()
{
    using pictura::SaveFormatKind;
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("jpg")), SaveFormatKind::Jpeg);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("JPEG")), SaveFormatKind::Jpeg);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("jpe")), SaveFormatKind::Jpeg);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("png")), SaveFormatKind::Png);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("bmp")), SaveFormatKind::Bmp);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("tif")), SaveFormatKind::Tiff);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("tiff")), SaveFormatKind::Tiff);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("psd")), SaveFormatKind::None);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("psb")), SaveFormatKind::None);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("webp")), SaveFormatKind::None);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("pdf")), SaveFormatKind::None);
}

void SaveFormatOptionsTest::dialogDefaults()
{
    const pictura::SaveFormatOptions defaults;

    pictura::JpegOptionsDialog jpeg(defaults, nullptr, Qt::black, Qt::white);
    QCOMPARE(jpeg.quality(), 10);
    QCOMPARE(jpeg.format(), 0);
    QCOMPARE(jpeg.matte(), -1);
    QVERIFY(jpeg.preview());
    QVERIFY(jpeg.findChild<QRadioButton*>(QStringLiteral("jpegOptionsBaseline"))->isChecked());

    pictura::PngOptionsDialog png(defaults);
    QCOMPARE(png.compression(), 0);
    QCOMPARE(png.interlace(), 0);

    pictura::TiffOptionsDialog tiff(defaults);
    QCOMPARE(tiff.compression(), 1);
    QVERIFY(tiff.transparency());
    QCOMPARE(tiff.pixelOrder(), 0);
    QCOMPARE(tiff.byteOrder(), 0);

    pictura::BmpOptionsDialog bmp(defaults);
    QCOMPARE(bmp.format(), 0);
    QCOMPARE(bmp.depth(), 24);
}

void SaveFormatOptionsTest::unsupportedControlsDisabled()
{
    const pictura::SaveFormatOptions defaults;

    pictura::JpegOptionsDialog jpeg(defaults, nullptr, Qt::black, Qt::white);
    auto* scans = jpeg.findChild<QSpinBox*>(QStringLiteral("jpegOptionsScans"));
    QVERIFY(scans);
    QVERIFY2(!scans->isEnabled(), "the progressive scan count is not written");
    QVERIFY(!scans->toolTip().isEmpty());
    auto* matte = jpeg.findChild<QComboBox*>(QStringLiteral("jpegOptionsMatte"));
    QVERIFY(matte);
    QVERIFY2(!matte->isEnabled(), "the JPEG matte is not offered");
    QVERIFY(!matte->toolTip().isEmpty());

    pictura::PngOptionsDialog png(defaults);
    for (const char* name : {"pngOptionsInterlaceNone", "pngOptionsInterlaced"}) {
        auto* button = png.findChild<QRadioButton*>(QString::fromLatin1(name));
        QVERIFY(button);
        QVERIFY2(!button->isEnabled(), "PNG interlacing is not written");
        QVERIFY(!button->toolTip().isEmpty());
    }

    pictura::BmpOptionsDialog bmp(defaults);
    for (const char* name : {"bmpOptionsWindows", "bmpOptionsOs2", "bmpOptionsRle",
                             "bmpOptionsFlip", "bmpOptionsDepth24"}) {
        auto* widget = bmp.findChild<QWidget*>(QString::fromLatin1(name));
        QVERIFY(widget);
        QVERIFY2(!widget->isEnabled(), name);
        QVERIFY(!widget->toolTip().isEmpty());
    }

    pictura::TiffOptionsDialog tiff(defaults);
    for (const char* name : {"tiffOptionsCompressionRLE", "tiffOptionsCompressionZIP",
                             "tiffOptionsPixelOrder0", "tiffOptionsByteOrder0",
                             "tiffOptionsLayer0", "tiffOptionsPyramid"}) {
        auto* widget = tiff.findChild<QWidget*>(QString::fromLatin1(name));
        QVERIFY(widget);
        QVERIFY2(!widget->isEnabled(), name);
        QVERIFY(!widget->toolTip().isEmpty());
    }
    QVERIFY2(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsTransparency"))->isEnabled(),
             "Save Transparency is honored");
    QVERIFY2(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsCompressionLZW"))->isEnabled(),
             "LZW compression is honored");
}

void SaveFormatOptionsTest::jpegQualityChangesSize()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    const QString high = path("quality-high.jpg");
    const QString low = path("quality-low.jpg");
    pictura::set_save_raster_options(*view, 100, -1, false, false, -1);
    QVERIFY(view->save(high));
    pictura::set_save_raster_options(*view, 5, -1, false, false, -1);
    QVERIFY(view->save(low));
    QVERIFY2(fileSize(low) < fileSize(high), "a lower quality is a smaller file");
    closeActive();
}

void SaveFormatOptionsTest::jpegQualityMapping()
{
    // Photoshop's Save As 0-12 is a high-quality range: 0 is roughly libjpeg 60,
    // not the libjpeg floor. See jpegQualityPercent.
    QCOMPARE(pictura::jpegQualityPercent(0), 60);
    QCOMPARE(pictura::jpegQualityPercent(12), 98);
    QCOMPARE(pictura::jpegQualityPercent(6), 81);
    QVERIFY(pictura::jpegQualityPercent(0) < pictura::jpegQualityPercent(6));
    QVERIFY(pictura::jpegQualityPercent(6) < pictura::jpegQualityPercent(12));
}

void SaveFormatOptionsTest::jpegFormatOption()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    const QString baseline = path("baseline.jpg");
    const QString progressive = path("progressive.jpg");
    pictura::set_save_raster_options(*view, 80, -1, false, false, -1);
    QVERIFY(view->save(baseline));
    QCOMPARE(jpegFrameMarker(baseline), 0xC0);
    pictura::set_save_raster_options(*view, 80, -1, true, true, -1);
    QVERIFY(view->save(progressive));
    QCOMPARE(jpegFrameMarker(progressive), 0xC2);
    closeActive();
}

void SaveFormatOptionsTest::jpegMatte()
{
    pictura::PictureView* view = newView(QStringLiteral("transparent"));
    QVERIFY(view);
    const QString file = path("matte.jpg");
    pictura::set_save_raster_options(*view, 90, -1, false, false, 0xCC3300);
    QVERIFY(view->save(file));
    const QImage image(file);
    QVERIFY(!image.isNull());
    const QColor pixel = image.pixelColor(1, 1);
    QVERIFY2(std::abs(pixel.red() - 0xCC) <= 4 && std::abs(pixel.green() - 0x33) <= 4
                 && std::abs(pixel.blue() - 0x00) <= 4,
             "the transparent pixel took the matte colour");
    closeActive();
}

void SaveFormatOptionsTest::pngCompressionChangesSize()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    const QString stored = path("stored.png");
    const QString packed = path("packed.png");
    pictura::set_save_raster_options(*view, -1, 0, false, false, -1);
    QVERIFY(view->save(stored));
    pictura::set_save_raster_options(*view, -1, 100, false, false, -1);
    QVERIFY(view->save(packed));
    QVERIFY2(fileSize(packed) < fileSize(stored), "compression shrinks the file");
    closeActive();
}

void SaveFormatOptionsTest::tiffCompressionTag()
{
    if (!tiffWriterAvailable()) {
        QSKIP("Qt TIFF writer unavailable");
    }
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    const QString none = path("none.tif");
    const QString lzw = path("lzw.tif");
    pictura::set_save_raster_options(*view, -1, 0, false, false, -1);
    QVERIFY(view->save(none));
    QCOMPARE(readTiffCompressionTag(none), 1);
    pictura::set_save_raster_options(*view, -1, 1, false, false, -1);
    QVERIFY(view->save(lzw));
    QCOMPARE(readTiffCompressionTag(lzw), 5);
    closeActive();
}

void SaveFormatOptionsTest::tiffTransparencyOffFlattens()
{
    if (!tiffWriterAvailable()) {
        QSKIP("Qt TIFF writer unavailable");
    }
    pictura::PictureView* view = newView(QStringLiteral("transparent"));
    QVERIFY(view);
    const QString file = path("opaque.tif");
    pictura::set_save_raster_options(*view, -1, 1, false, false, 0xFFFFFF);
    QVERIFY(view->save(file));
    const QImage image(file);
    QVERIFY(!image.isNull());
    QCOMPARE(qAlpha(image.pixel(1, 1)), 255);
    closeActive();
}

void SaveFormatOptionsTest::cancelWritesNothing()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return false; });
    const QString file = path("cancelled.jpg");
    QVERIFY(!window_->saveActiveAsWithOptions(file));
    QVERIFY2(!QFileInfo::exists(file), "cancelling writes nothing");
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::rememberedSettings()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.pngCompression = 100;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("remembered.png")));

    const pictura::SaveFormatOptions stored =
        pictura::saveFormatOptionsFromJson(pictura::loadSession().saveFormatOptions);
    QCOMPARE(stored.pngCompression, 100);

    int seeded = -1;
    window_->setSaveFormatDialogForTest(
        [&seeded](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            seeded = options.pngCompression;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("remembered2.png")));
    QCOMPARE(seeded, 100);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::sizeEstimateFollowsQuality()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    const pictura::SaveFormatOptions defaults;
    pictura::JpegOptionsDialog dialog(defaults, view, Qt::black, Qt::white);
    auto* quality = dialog.findChild<QSpinBox*>(QStringLiteral("jpegOptionsQuality"));
    QVERIFY(quality);

    quality->setValue(12);
    dialog.refreshEstimateForTest();
    const qint64 high = dialog.estimatedBytesForTest();

    quality->setValue(2);
    dialog.refreshEstimateForTest();
    const qint64 low = dialog.estimatedBytesForTest();

    QVERIFY(high > 0);
    QVERIFY2(low < high, "a lower quality estimates a smaller file");
    QVERIFY(!dialog.sizeTextForTest().isEmpty());
    closeActive();
}

void SaveFormatOptionsTest::previewDoesNotMutate()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    const int history = view->history_count();
    const bool dirty = pictura::document_api::isDirty(*view);
    const QRgb pixel = view->sample_argb(2, 2);

    const pictura::SaveFormatOptions defaults;
    pictura::JpegOptionsDialog dialog(defaults, view, Qt::black, Qt::white);
    auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("jpegOptionsPreview"));
    QVERIFY(preview);
    preview->setChecked(false);
    dialog.refreshEstimateForTest();
    preview->setChecked(true);
    dialog.refreshEstimateForTest();

    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);
    QCOMPARE(view->sample_argb(2, 2), pixel);
    closeActive();
}

QTEST_MAIN(SaveFormatOptionsTest)
#include "tst_save_format_options.moc"
