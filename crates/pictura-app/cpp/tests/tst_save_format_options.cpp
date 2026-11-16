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
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>

#include <cstddef>

#include "bmp_advanced_modes_dialog.h"
#include "bmp_options_dialog.h"
#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "cineon_options_dialog.h"
#include "dcs_options_dialog.h"
#include "dialogs.h"
#include "eps_options_dialog.h"
#include "frame.h"
#include "gif_options_dialog.h"
#include "jpeg_options_dialog.h"
#include "png_options_dialog.h"
#include "raw_options_dialog.h"
#include "save_format_options.h"
#include "session.h"
#include "tga_options_dialog.h"
#include "tiff_options_dialog.h"

#include "api/paint_bridge.h"
#include "api/engine_bridge.h"

#include "qt_test_support.h"

namespace {

void setTypedSaveOptions(pictura::PictureView& view, pictura::SaveFormatKind kind,
                         const QByteArray& json)
{
    pictura::set_save_options(view, static_cast<int>(kind),
                              ::rust::Str(json.constData(),
                                          static_cast<std::size_t>(json.size())));
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

// A TIFF tag's first value, or -1. Handles both byte orders and inline values.
qint64 readTiffTagValue(const QString& path, int wanted)
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
    if (!little && !(b[0] == 'M' && b[1] == 'M')) {
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
        if (u16(entry) != wanted) {
            continue;
        }
        const int type = u16(entry + 2);
        const quint32 n = u32(entry + 4);
        if (type == 3) {
            return u16(entry + 8);
        }
        if (type == 4) {
            return u32(entry + 8);
        }
        return n;
    }
    return -1;
}

// The interlace bit of a GIF image descriptor, or -1.
int gifInterlaceBit(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return -1;
    }
    const QByteArray b = file.readAll();
    if (!b.startsWith("GIF89a") || b.size() < 13) {
        return -1;
    }
    const int packed = quint8(b[10]);
    int at = 13;
    if (packed & 0x80) {
        at += 3 * (1 << ((packed & 0x07) + 1));
    }
    while (at < b.size()) {
        const int kind = quint8(b[at]);
        if (kind == 0x2C) {
            return at + 9 < b.size() ? (quint8(b[at + 9]) & 0x40) : -1;
        }
        if (kind == 0x21) {
            at += 2;
            while (at < b.size()) {
                const int size = quint8(b[at]);
                at += 1 + size;
                if (size == 0) {
                    break;
                }
            }
            continue;
        }
        break;
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
    void filterHintChoosesDcs();
    void filterHintChoosesEps();
    void unknownSuffixRefused();
    void missingSuffixRefused();
    void saveStartRefusesNewFormats();
    void featureWarningNewFormats();
    void pngInterlaceWrite();
    void bmpOptionsWrite();
    void tiffPixelByteOrderWrite();
    void tiffPyramidWrite();
    void tiffLayerCompressionWrite();
    void epsOptionsWrite();
    void rawOptionsWrite();
    void tgaOptionsWrite();
    void cineonOptionsWrite();
    void gifSaveConverts();
    void gifCancelLeavesMode();
    void gifWarningBeforeConversion();
    void gifRowOrderWrite();
    void plainSaveLayeredTiff();
    void dcsFiles();
    void dcsSpotCeiling();
    void plainSaveKind();

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
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("gif")), SaveFormatKind::Gif);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("eps")), SaveFormatKind::Eps);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("raw")), SaveFormatKind::Raw);
    for (const char* suffix : {"tga", "vda", "icb", "vst"}) {
        QCOMPARE(pictura::saveFormatKindForSuffix(QString::fromLatin1(suffix)),
                 SaveFormatKind::Tga);
    }
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("cin")), SaveFormatKind::Cineon);
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
    QCOMPARE(tiff.compression(), 2);
    QVERIFY(tiff.transparency());
    QCOMPARE(tiff.pixelOrder(), 0);
    QCOMPARE(tiff.byteOrder(), 0);
    auto* byteNote = tiff.findChild<QLabel*>(QStringLiteral("tiffOptionsByteOrderNote"));
    QVERIFY(byteNote);
    QVERIFY(byteNote->text().contains(QStringLiteral("Layered TIFF")));

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
    QVERIFY(png.findChild<QRadioButton*>(QStringLiteral("pngOptionsInterlaced"))->isEnabled());

    pictura::BmpOptionsDialog bmp(defaults);
    QVERIFY(bmp.findChild<QWidget*>(QStringLiteral("bmpOptionsDepth8"))->isEnabled());

    pictura::TiffOptionsDialog tiff(defaults);
    QVERIFY(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsCompressionRLE"))->isEnabled());
    QVERIFY(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsCompressionZIP"))->isEnabled());
    QVERIFY(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsPixelOrder0"))->isEnabled());
    QVERIFY(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsByteOrder0"))->isEnabled());
    QVERIFY(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsLayer0"))->isEnabled());
    QVERIFY(tiff.findChild<QWidget*>(QStringLiteral("tiffOptionsPyramid"))->isEnabled());
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
    setTypedSaveOptions(*view, pictura::SaveFormatKind::Tiff,
                        QByteArrayLiteral("{\"tiff\":{\"compression\":4,\"transparency\":false}}"));
    QVERIFY(view->save(path("jpeg.tif")));
    QCOMPARE(readTiffCompressionTag(path("jpeg.tif")), 7);
    QCOMPARE(readTiffTagValue(path("jpeg.tif"), 262), qint64(6));
    closeActive();
}

void SaveFormatOptionsTest::tiffTransparencyOffFlattens()
{
    pictura::PictureView* view = newView(QStringLiteral("transparent"));
    QVERIFY(view);
    const QString file = path("opaque.tif");
    pictura::set_save_raster_options(*view, -1, 1, false, false, 0xFFFFFF);
    QVERIFY(view->save(file));
    // CI's Qt package has no TIFF image plugin; assert the written tags here.
    // The codec test checks the flattened pixel itself.
    QCOMPARE(readTiffTagValue(file, 277), qint64(3));
    QCOMPARE(readTiffTagValue(file, 338), qint64(-1));
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

    // An older session stored Qt's TIFF enum: 1 was LZW, 5 JPEG.
    QJsonObject oldTiff;
    oldTiff.insert(QStringLiteral("compression"), 1);
    QJsonObject oldRoot;
    oldRoot.insert(QStringLiteral("tiff"), oldTiff);
    QCOMPARE(pictura::saveFormatOptionsFromJson(oldRoot).tiffCompression, 2);
    QJsonObject newTiff;
    newTiff.insert(QStringLiteral("version"), 2);
    newTiff.insert(QStringLiteral("compression"), 1);
    QJsonObject newRoot;
    newRoot.insert(QStringLiteral("tiff"), newTiff);
    QCOMPARE(pictura::saveFormatOptionsFromJson(newRoot).tiffCompression, 1);

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


void SaveFormatOptionsTest::filterHintChoosesDcs()
{
    QCOMPARE(pictura::saveFormatKindForFilter(10), pictura::SaveFormatKind::Dcs);
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("eps")),
             pictura::SaveFormatKind::Eps);
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    pictura::SaveFormatKind seen = pictura::SaveFormatKind::None;
    window_->setWarningDriverForTest(
        [](const QString&, const QString&) { return true; });
    window_->setSaveFormatDialogForTest(
        [&seen](pictura::SaveFormatKind kind, pictura::SaveFormatOptions& options) {
            seen = kind;
            options.dcsVersion = 1;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("hint.eps"), pictura::SaveFormatKind::Dcs));
    QCOMPARE(seen, pictura::SaveFormatKind::Dcs);
    QVERIFY(QFileInfo::exists(path("hint.C")));
    window_->setSaveFormatDialogForTest(nullptr);
    window_->setWarningDriverForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::filterHintChoosesEps()
{
    QCOMPARE(pictura::saveFormatKindForFilter(9), pictura::SaveFormatKind::Eps);
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    pictura::SaveFormatKind seen = pictura::SaveFormatKind::None;
    window_->setSaveFormatDialogForTest(
        [&seen](pictura::SaveFormatKind kind, pictura::SaveFormatOptions&) {
            seen = kind;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("hint.eps"), pictura::SaveFormatKind::Eps));
    QCOMPARE(seen, pictura::SaveFormatKind::Eps);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::unknownSuffixRefused()
{
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("xyz")),
             pictura::SaveFormatKind::None);
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(!view->save(path("unknown.xyz")));
    QVERIFY(!QFileInfo::exists(path("unknown.xyz")));
    closeActive();
}

void SaveFormatOptionsTest::missingSuffixRefused()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(!view->save(path("no-extension")));
    QVERIFY(!view->save(path("trailing.")));
    QVERIFY(!QFileInfo::exists(path("trailing.")));
    closeActive();
}

void SaveFormatOptionsTest::saveStartRefusesNewFormats()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(!pictura::document_api::saveStart(*view, path("out.gif")));
    QVERIFY(!pictura::document_api::saveStart(*view, path("out.xyz")));
    QVERIFY(!pictura::document_api::saveStart(*view, path("suffixless")));
    QVERIFY(!pictura::document_api::saveStart(*view, path("trailing.")));
    closeActive();
}

void SaveFormatOptionsTest::featureWarningNewFormats()
{
    QVERIFY(pictura::isRasterImageSuffix(QStringLiteral("gif")));
    QVERIFY(pictura::isRasterImageSuffix(QStringLiteral("tga")));
    QVERIFY(pictura::isRasterImageSuffix(QStringLiteral("eps")));
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(view->duplicate_layer(0) >= 0);
    QVERIFY2(!pictura::saveAsNeedsFeatureWarning(path("layers.tif"), view),
             "layered TIFF keeps its layer section");
    QStringList warnings;
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    window_->setWarningDriverForTest([&warnings](const QString&, const QString& text) {
        warnings.append(text);
        return true;
    });
    QVERIFY(window_->saveActiveAsWithOptions(path("layered.tga")));
    QVERIFY(!warnings.isEmpty());
    // GIF converts to Indexed during the prompt; the layer warning must still
    // be shown before that flattening.
    const int beforeGif = warnings.size();
    window_->setGifDialogForTest(
        [](pictura::PictureView*, pictura::SaveFormatOptions&) { return true; });
    QVERIFY(window_->saveActiveAsWithOptions(path("layered.gif")));
    QVERIFY2(warnings.size() > beforeGif, "layered GIF warns before converting");
    window_->setGifDialogForTest(nullptr);
    window_->setWarningDriverForTest(nullptr);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::pngInterlaceWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.pngInterlace = 1;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("interlaced.png")));
    QFile file(path("interlaced.png"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray bytes = file.readAll();
    QVERIFY(bytes.size() > 28);
    QCOMPARE(quint8(bytes[28]), quint8(1)); // IHDR interlace method
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::bmpOptionsWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.bmpDepth = 8;
            options.bmpRle = true;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("rle.bmp")));
    QFile file(path("rle.bmp"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray bytes = file.readAll();
    QVERIFY(bytes.size() > 34);
    QCOMPARE(quint16(quint8(bytes[28])), quint16(8)); // biBitCount
    QCOMPARE(quint32(quint8(bytes[30])), quint32(1)); // BI_RLE8
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::tiffPixelByteOrderWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    setTypedSaveOptions(*view, pictura::SaveFormatKind::Tiff,
                        QByteArrayLiteral("{\"tiff\":{\"pixelOrder\":1,\"byteOrder\":1}}"));
    QVERIFY(view->save(path("planar-mm.tif")));
    QFile file(path("planar-mm.tif"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    QVERIFY(file.read(2) == QByteArrayLiteral("MM"));
    QCOMPARE(readTiffTagValue(path("planar-mm.tif"), 284), qint64(2));
    closeActive();
}

void SaveFormatOptionsTest::tiffPyramidWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    setTypedSaveOptions(*view, pictura::SaveFormatKind::Tiff,
                        QByteArrayLiteral("{\"tiff\":{\"pyramid\":true}}"));
    QVERIFY(view->save(path("pyramid.tif")));
    QVERIFY(readTiffTagValue(path("pyramid.tif"), 330) >= 0);
    closeActive();
}

void SaveFormatOptionsTest::tiffLayerCompressionWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(view->duplicate_layer(0) >= 0);
    setTypedSaveOptions(*view, pictura::SaveFormatKind::Tiff,
                        QByteArrayLiteral("{\"tiff\":{\"layerCompression\":1}}"));
    QVERIFY(view->save(path("layers-zip.tif")));
    QVERIFY(readTiffTagValue(path("layers-zip.tif"), 37724) >= 0);
    setTypedSaveOptions(*view, pictura::SaveFormatKind::Tiff,
                        QByteArrayLiteral("{\"tiff\":{\"layerCompression\":2}}"));
    QVERIFY(view->save(path("layers-flat.tif")));
    QCOMPARE(readTiffTagValue(path("layers-flat.tif"), 37724), qint64(-1));
    closeActive();
}

void SaveFormatOptionsTest::epsOptionsWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(pictura::path_append_corner(*view, 1.0, 1.0, false));
    pictura::path_commit_anchor(*view);
    QVERIFY(pictura::path_append_corner(*view, 3.0, 1.0, false));
    pictura::path_commit_anchor(*view);
    QVERIFY(pictura::path_append_corner(*view, 2.0, 3.0, false));
    pictura::path_commit_anchor(*view);
    QVERIFY(pictura::path_close(*view));
    pictura::path_finish(*view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.epsPreview = 0;
            options.epsEncoding = 0;
            options.epsHalftone = true;
            options.epsVectorData = true;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("options.eps")));
    QFile file(path("options.eps"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QString text = QString::fromUtf8(file.readAll());
    QVERIFY2(text.contains(QStringLiteral("sethalftone")), "halftone PostScript");
    QVERIFY2(!text.contains(QStringLiteral("clip")),
             "a Work Path alone is not a designated clipping path");
    QVERIFY2(text.contains(QStringLiteral("stroke")), "vector path operators");
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::rawOptionsWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.rawHeader = 16;
            options.rawInterleave = 0;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("header.raw")));
    QFile file(path("header.raw"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray bytes = file.readAll();
    QCOMPARE(bytes.left(16), QByteArray(16, char(0)));
    QCOMPARE(bytes.size(), 16 + 32 * 32 * 3);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::tgaOptionsWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.tgaDepth = 32;
            options.tgaRle = true;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("rle32.tga")));
    QFile file(path("rle32.tga"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray bytes = file.readAll();
    QVERIFY(bytes.size() > 18);
    QCOMPARE(quint8(bytes[2]), quint8(10)); // RLE true-colour
    QCOMPARE(quint8(bytes[16]), quint8(32));
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::cineonOptionsWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    QVERIFY(window_->saveActiveAsWithOptions(path("frame.cin")));
    QFile file(path("frame.cin"));
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray bytes = file.readAll();
    QVERIFY(bytes.size() > 1024);
    QCOMPARE(bytes.left(4), QByteArray::fromHex("802a5fd7"));
    QCOMPARE(quint8(bytes[193]), quint8(3));
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::gifSaveConverts()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setGifDialogForTest([](pictura::PictureView*, pictura::SaveFormatOptions&) {
        return true;
    });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions&) { return true; });
    QVERIFY(window_->saveActiveAsWithOptions(path("converted.gif")));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("indexed"));
    window_->setGifDialogForTest(nullptr);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::gifCancelLeavesMode()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setGifDialogForTest([](pictura::PictureView*, pictura::SaveFormatOptions&) {
        return false;
    });
    QVERIFY(!window_->saveActiveAsWithOptions(path("cancelled.gif")));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    window_->setGifDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::gifWarningBeforeConversion()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(view->duplicate_layer(0) >= 0);
    bool gifDialogCalled = false;
    QStringList warnings;
    window_->setWarningDriverForTest([&warnings](const QString&, const QString& text) {
        warnings.append(text);
        return false;
    });
    window_->setGifDialogForTest(
        [&gifDialogCalled](pictura::PictureView*, pictura::SaveFormatOptions&) {
            gifDialogCalled = true;
            return true;
        });
    QVERIFY(!window_->saveActiveAsWithOptions(path("blocked.gif")));
    QVERIFY(!warnings.isEmpty());
    QVERIFY2(!gifDialogCalled, "the warning runs before the indexed conversion prompt");
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QCOMPARE(view->layer_count(), 2);
    QVERIFY(!QFileInfo::exists(path("blocked.gif")));
    window_->setWarningDriverForTest(nullptr);
    window_->setGifDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::gifRowOrderWrite()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setGifDialogForTest(
        [](pictura::PictureView*, pictura::SaveFormatOptions&) { return true; });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.gifRowOrder = 1;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("interlaced.gif")));
    QCOMPARE(gifInterlaceBit(path("interlaced.gif")), 0x40);
    window_->setGifDialogForTest(nullptr);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::dcsFiles()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setWarningDriverForTest(
        [](const QString&, const QString&) { return true; });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.dcsVersion = 1;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("art.eps"), pictura::SaveFormatKind::Dcs));
    for (const QString& suffix : {QStringLiteral("eps"), QStringLiteral("C"),
                                  QStringLiteral("M"), QStringLiteral("Y"),
                                  QStringLiteral("K")}) {
        QVERIFY(QFileInfo::exists(dir_.filePath(QStringLiteral("art.") + suffix)));
    }
    window_->setSaveFormatDialogForTest(nullptr);
    window_->setWarningDriverForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::dcsSpotCeiling()
{
    QVERIFY(pictura::isRasterImageSuffix(QStringLiteral("eps")));
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(pictura::channels_new_alpha(*view) >= 0);
    QStringList warnings;
    window_->setWarningDriverForTest([&warnings](const QString&, const QString& text) {
        warnings.append(text);
        return true;
    });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.dcsVersion = 2;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("spots.eps"), pictura::SaveFormatKind::Dcs));
    QVERIFY2(!warnings.isEmpty(), "the extra-channel omission warning runs");
    window_->setWarningDriverForTest(nullptr);
    window_->setSaveFormatDialogForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::plainSaveKind()
{
    QCOMPARE(pictura::saveFormatKindForSuffix(QStringLiteral("tga")),
             pictura::SaveFormatKind::Tga);
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    window_->setWarningDriverForTest(
        [](const QString&, const QString&) { return true; });
    window_->setSaveFormatDialogForTest(
        [](pictura::SaveFormatKind, pictura::SaveFormatOptions& options) {
            options.dcsVersion = 1;
            return true;
        });
    QVERIFY(window_->saveActiveAsWithOptions(path("plain.eps"), pictura::SaveFormatKind::Dcs));
    QVERIFY(QFileInfo::exists(path("plain.C")));
    QFile::remove(path("plain.C"));
    window_->setSaveFormatDialogForTest(nullptr);
    QVERIFY(window_->saveActive());
    QVERIFY2(QFileInfo::exists(path("plain.C")), "plain Save keeps the DCS kind");
    window_->setWarningDriverForTest(nullptr);
    closeActive();
}

void SaveFormatOptionsTest::plainSaveLayeredTiff()
{
    pictura::PictureView* view = newView(QStringLiteral("white"));
    QVERIFY(view);
    QVERIFY(view->duplicate_layer(0) >= 0);
    QVERIFY(view->save(path("plain-layers.tif")));
    QVERIFY2(!pictura::savePathNeedsFormatDialog(path("plain-layers.tif"), view, false),
             "a layered TIFF reuses its path");
    QVERIFY(pictura::savePathNeedsFormatDialog(path("layers.eps"), view, false, false));
    QVERIFY2(!pictura::savePathNeedsFormatDialog(path("layers.eps"), view, false, true),
             "a remembered DCS kind reuses its .eps path");
    closeActive();
}


QTEST_MAIN(SaveFormatOptionsTest)
#include "tst_save_format_options.moc"
