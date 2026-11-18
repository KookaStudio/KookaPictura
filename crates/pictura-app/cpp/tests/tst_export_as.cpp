// File > Export As… — the CC-style dialog (#457).

#include <QtTest/QtTest>

#include "export_as_dialog.h"
#include "frame.h"

#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

#include "api/engine_bridge.h"
#include "api/layers_bridge.h"

#include "qt_test_support.h"

#include <memory>

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

// A detailed 64 x 48 field with deterministic noise, where lossy coding wins.
QImage photo()
{
    QImage image(64, 48, QImage::Format_RGBA8888);
    unsigned seed = 0x12345678u;
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            seed = seed * 1664525u + 1013904223u;
            const int noise = int((seed >> 24) & 0xffu);
            const int base = (x * 2 + y * 3) % 256;
            image.setPixel(x, y,
                           qRgba(qBound(0, base + noise / 2, 255),
                                 qBound(0, base + noise, 255),
                                 qBound(0, base - noise / 3, 255), 255));
        }
    }
    return image;
}

pictura::ExportEntry entry(const QString& name, const QImage& image)
{
    return {name, QStringLiteral("pixel"), image};
}

QVector<pictura::ExportEntry> entries()
{
    return {entry(QStringLiteral("photo"), picture()),
            entry(QStringLiteral("Layer 1"), picture().scaled(32, 24)),
            entry(QStringLiteral("Layer 2"), picture().scaled(16, 12))};
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

} // namespace

class ExportAsTest : public QObject {
    Q_OBJECT

private slots:
    void formatsAndControls();
    void png8AndGif();
    void webpExport();
    void webpQualityControls();
    void webpLossyIsSmallerThanLossless();
    void imageSizeAndCanvas();
    void layerList();
    void saveToBytes();
    void sizeEstimate();
    void svgExport();
    void svgExportPerLayer();
    void svgExportPaintServers();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ExportAsTest::formatsAndControls()
{
    pictura::ExportAsDialog dialog(entries());
    auto* format = child<QComboBox>(dialog, "exportAsFormat");
    QVERIFY(format);
    QCOMPARE(format->count(), 6);
    QCOMPARE(format->itemText(0), QStringLiteral("PNG"));
    QCOMPARE(format->itemText(1), QStringLiteral("JPG"));
    QCOMPARE(format->itemText(2), QStringLiteral("PNG-8"));
    QCOMPARE(format->itemText(3), QStringLiteral("GIF"));
    QCOMPARE(format->itemText(4), QStringLiteral("SVG"));
    QCOMPARE(format->itemText(5), QStringLiteral("WebP"));

    // PNG: transparency on, quality off.
    QCOMPARE(dialog.suffix(), QStringLiteral("png"));
    QVERIFY(child<QCheckBox>(dialog, "exportAsTransparency")->isEnabled());
    QVERIFY(!child<QSpinBox>(dialog, "exportAsQuality")->isEnabled());

    // JPG: quality on, transparency off.
    format->setCurrentIndex(pictura::ExportAsDialog::Jpg);
    QCOMPARE(dialog.suffix(), QStringLiteral("jpg"));
    QVERIFY(!child<QCheckBox>(dialog, "exportAsTransparency")->isEnabled());
    QVERIFY(child<QSpinBox>(dialog, "exportAsQuality")->isEnabled());

    // GIF: transparency on, quality off.
    format->setCurrentIndex(pictura::ExportAsDialog::Gif);
    QCOMPARE(dialog.suffix(), QStringLiteral("gif"));
    QVERIFY(child<QCheckBox>(dialog, "exportAsTransparency")->isEnabled());
    QVERIFY(!child<QSpinBox>(dialog, "exportAsQuality")->isEnabled());

    // SVG: neither raster control applies.
    format->setCurrentIndex(pictura::ExportAsDialog::Svg);
    QCOMPARE(dialog.suffix(), QStringLiteral("svg"));
    QVERIFY(!child<QCheckBox>(dialog, "exportAsTransparency")->isEnabled());
    QVERIFY(!child<QSpinBox>(dialog, "exportAsQuality")->isEnabled());

    // WebP: transparency on (it carries alpha), lossy by default.
    format->setCurrentIndex(pictura::ExportAsDialog::Webp);
    QCOMPARE(dialog.suffix(), QStringLiteral("webp"));
    QVERIFY(child<QCheckBox>(dialog, "exportAsTransparency")->isEnabled());
    QVERIFY(child<QSpinBox>(dialog, "exportAsQuality")->isEnabled());
    QVERIFY(!child<QCheckBox>(dialog, "exportAsLossless")->isHidden());
}

// WebP export encodes through the engine, so it works without the optional Qt
// WebP plugin.
void ExportAsTest::webpExport()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::ExportAsDialog dialog(entries());
    auto* format = child<QComboBox>(dialog, "exportAsFormat");
    format->setCurrentIndex(pictura::ExportAsDialog::Webp);
    QCOMPARE(dialog.suffix(), QStringLiteral("webp"));

    const QByteArray bytes = dialog.encoded(0);
    QVERIFY(bytes.size() > 12);
    QCOMPARE(bytes.left(4), QByteArrayLiteral("RIFF"));
    QCOMPARE(bytes.mid(8, 4), QByteArrayLiteral("WEBP"));
    // The default is lossy, not the old VP8L stream.
    QVERIFY(bytes.contains(QByteArrayLiteral("VP8 ")));
    QVERIFY(!bytes.contains(QByteArrayLiteral("VP8L")));

    // A matching suffix writes exactly the encoded bytes.
    const QString path = dir.filePath(QStringLiteral("out.webp"));
    QVERIFY(dialog.saveTo(0, path));
    QFile file(path);
    QVERIFY(file.open(QIODevice::ReadOnly));
    QCOMPARE(file.readAll(), bytes);
}

void ExportAsTest::webpQualityControls()
{
    pictura::ExportAsDialog dialog(entries());
    auto* format = child<QComboBox>(dialog, "exportAsFormat");
    auto* quality = child<QSpinBox>(dialog, "exportAsQuality");
    auto* lossless = child<QCheckBox>(dialog, "exportAsLossless");
    auto* transparency = child<QCheckBox>(dialog, "exportAsTransparency");
    QVERIFY(format && quality && lossless && transparency);

    format->setCurrentIndex(pictura::ExportAsDialog::Webp);
    QVERIFY(!lossless->isChecked());
    QVERIFY(!lossless->isHidden());
    QCOMPARE(quality->value(), 90);
    QVERIFY(quality->isEnabled());
    QVERIFY(transparency->isEnabled());

    lossless->setChecked(true);
    QVERIFY(!quality->isEnabled());
    QVERIFY(transparency->isEnabled());
    const QByteArray exact = dialog.encoded(0);
    QVERIFY(exact.contains(QByteArrayLiteral("VP8L")));
    QVERIFY(!exact.contains(QByteArrayLiteral("VP8 ")));

    format->setCurrentIndex(pictura::ExportAsDialog::Png);
    QVERIFY(lossless->isHidden());
    QVERIFY(!quality->isEnabled());

    format->setCurrentIndex(pictura::ExportAsDialog::Jpg);
    QVERIFY(quality->isEnabled());
    QVERIFY(!transparency->isEnabled());
}

void ExportAsTest::webpLossyIsSmallerThanLossless()
{
    QVector<pictura::ExportEntry> noisy;
    noisy.append(entry(QStringLiteral("photo"), photo()));
    pictura::ExportAsDialog dialog(noisy);
    auto* format = child<QComboBox>(dialog, "exportAsFormat");
    auto* quality = child<QSpinBox>(dialog, "exportAsQuality");
    auto* lossless = child<QCheckBox>(dialog, "exportAsLossless");
    format->setCurrentIndex(pictura::ExportAsDialog::Webp);

    const QByteArray high = dialog.encoded(0);
    quality->setValue(10);
    const QByteArray low = dialog.encoded(0);
    lossless->setChecked(true);
    const QByteArray exact = dialog.encoded(0);

    QVERIFY2(low.size() < high.size(),
             qPrintable(QStringLiteral("q10 %1, q90 %2").arg(low.size()).arg(high.size())));
    QVERIFY2(high.size() < exact.size(),
             qPrintable(QStringLiteral("q90 %1, lossless %2")
                            .arg(high.size())
                            .arg(exact.size())));
}

void ExportAsTest::png8AndGif()
{
    pictura::ExportAsDialog dialog(entries());
    auto* format = child<QComboBox>(dialog, "exportAsFormat");

    format->setCurrentIndex(pictura::ExportAsDialog::Png8);
    const QImage png8 = QImage::fromData(dialog.encoded(0));
    QCOMPARE(png8.size(), QSize(64, 48));
    QVERIFY2(distinctColors(png8) <= 256, qPrintable(QString::number(distinctColors(png8))));

    format->setCurrentIndex(pictura::ExportAsDialog::Gif);
    const QByteArray gif = dialog.encoded(0);
    QVERIFY(gif.startsWith("GIF89a"));
    const QImage decoded = QImage::fromData(gif);
    QCOMPARE(decoded.size(), QSize(64, 48));
}

void ExportAsTest::imageSizeAndCanvas()
{
    pictura::ExportAsDialog dialog(entries());
    auto* scale = child<QSpinBox>(dialog, "exportAsScale");
    auto* width = child<QSpinBox>(dialog, "exportAsWidth");
    auto* height = child<QSpinBox>(dialog, "exportAsHeight");
    auto* resample = child<QComboBox>(dialog, "exportAsResample");

    // The Resample menu carries the CC options.
    QCOMPARE(resample->count(), 7);
    QCOMPARE(resample->itemText(5), QStringLiteral("Nearest Neighbor"));
    QCOMPARE(resample->itemText(6), QStringLiteral("Preserve Details"));

    // Scale drives the linked width and height.
    scale->setValue(50);
    QCOMPARE(width->value(), 32);
    QCOMPARE(height->value(), 24);
    QCOMPARE(dialog.sized(0).size(), QSize(32, 24));

    // Canvas Size crops the output, and Reset restores it to the image size.
    auto* canvasWidth = child<QSpinBox>(dialog, "exportAsCanvasWidth");
    auto* canvasHeight = child<QSpinBox>(dialog, "exportAsCanvasHeight");
    canvasWidth->setValue(20);
    canvasHeight->setValue(10);
    QCOMPARE(dialog.sized(0).size(), QSize(20, 10));
    child<QPushButton>(dialog, "exportAsCanvasReset")->click();
    QCOMPARE(canvasWidth->value(), width->value());
    QCOMPARE(canvasHeight->value(), height->value());
    QCOMPARE(dialog.sized(0).size(), QSize(32, 24));
}

void ExportAsTest::layerList()
{
    pictura::ExportAsDialog dialog(entries());
    auto* list = child<QListWidget>(dialog, "exportAsList");
    QCOMPARE(list->count(), 3);
    // The document row is checked by default.
    QCOMPARE(dialog.checkedEntries(), QVector<int>{0});

    // Selecting a layer resets the sizing to that layer's own size.
    list->setCurrentRow(1);
    QCOMPARE(child<QSpinBox>(dialog, "exportAsWidth")->value(), 32);
    QCOMPARE(child<QSpinBox>(dialog, "exportAsHeight")->value(), 24);

    list->item(1)->setCheckState(Qt::Checked);
    QCOMPARE(dialog.checkedEntries(), (QVector<int>{0, 1}));
    list->item(0)->setCheckState(Qt::Unchecked);
    QCOMPARE(dialog.checkedEntries(), QVector<int>{1});
}

void ExportAsTest::saveToBytes()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::ExportAsDialog dialog(entries());
    auto* format = child<QComboBox>(dialog, "exportAsFormat");

    // A matching suffix writes exactly the encoded bytes.
    const QString pngPath = dir.filePath(QStringLiteral("out.png"));
    QVERIFY(dialog.saveTo(0, pngPath));
    QFile png(pngPath);
    QVERIFY(png.open(QIODevice::ReadOnly));
    QCOMPARE(png.readAll(), dialog.encoded(0));

    // A typed `.jpg` writes JPEG even though the combo says PNG.
    const QString jpgPath = dir.filePath(QStringLiteral("out.jpg"));
    QVERIFY(dialog.saveTo(0, jpgPath));
    QFile jpg(jpgPath);
    QVERIFY(jpg.open(QIODevice::ReadOnly));
    const QByteArray jpgBytes = jpg.readAll();
    QVERIFY(jpgBytes.startsWith("\xff\xd8"));
    QVERIFY(!jpgBytes.startsWith("\x89PNG"));

    // The quality control feeds the JPEG encoder.
    format->setCurrentIndex(pictura::ExportAsDialog::Jpg);
    child<QSpinBox>(dialog, "exportAsQuality")->setValue(10);
    const QByteArray low = dialog.encoded(0);
    child<QSpinBox>(dialog, "exportAsQuality")->setValue(100);
    const QByteArray high = dialog.encoded(0);
    QVERIFY(low.size() < high.size());
}

void ExportAsTest::sizeEstimate()
{
    pictura::ExportAsDialog dialog(entries());
    auto* label = child<QLabel>(dialog, "exportAsSizeLabel");
    QVERIFY(label);
    // The estimate is computed on a short debounce; a small image gets a size.
    QTRY_VERIFY_WITH_TIMEOUT(!label->text().isEmpty(), 2000);
    QVERIFY(label->text().contains(QStringLiteral("64 x 48")));
}

void ExportAsTest::svgExport()
{
    QVERIFY(stateHome_.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Svg"), 16, 12, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    pictura::PictureView* view = window->activeView();
    QVERIFY(view);

    const QString svg = pictura::svg_from_view(*view);
    QVERIFY(svg.startsWith(QStringLiteral("<svg ")));
    QVERIFY(svg.contains(QStringLiteral("viewBox=\"0 0 16 12\"")));
    QVERIFY(svg.contains(QStringLiteral("data:img/png;base64,")));
    QVERIFY(svg.endsWith(QStringLiteral("</svg>\n")));
}

void ExportAsTest::svgExportPerLayer()
{
    QVERIFY(stateHome_.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Svg"), 16, 12, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    pictura::PictureView* view = window->activeView();
    QVERIFY(view);
    QVERIFY(!view->add_layer_in(QStringLiteral("0")).isEmpty());
    QCOMPARE(view->layer_count(), 2);

    const QString bottom = pictura::svg_from_layer(*view, 0);
    const QString top = pictura::svg_from_layer(*view, 1);
    QVERIFY(bottom.contains(QStringLiteral("viewBox=\"0 0 16 12\"")));
    QVERIFY(top.contains(QStringLiteral("viewBox=\"0 0 16 12\"")));
    QCOMPARE(bottom.count(QStringLiteral("<image ")), 1);
    QCOMPARE(top.count(QStringLiteral("<image ")), 1);
    QVERIFY(bottom != top);
    QVERIFY(pictura::svg_from_layer(*view, 2).isEmpty());
}

void ExportAsTest::svgExportPaintServers()
{
    QVERIFY(stateHome_.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("SvgPrimitives"), 24, 16, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    pictura::PictureView* view = window->activeView();
    QVERIFY(view);

    // The app gives new fill layers a full-white default mask; that must not
    // force a gradient to a raster.
    const QString gradient = view->add_gradient_fill();
    QVERIFY(!gradient.isEmpty());

    const QStringList ids = available_pattern_ids(*view);
    QVERIFY(!ids.isEmpty());
    const QString pattern =
        add_fill_layer(*view, QStringLiteral("pattern"),
                       ids.first() + QStringLiteral("|100|1|0|0"), QString(), QString(), 0,
                       QStringLiteral("norm"), 255, false);
    QVERIFY(!pattern.isEmpty());

    const QString svg = pictura::svg_from_view(*view);
    QVERIFY2(svg.contains(QStringLiteral("<linearGradient")), qPrintable(svg));
    QVERIFY2(svg.contains(QStringLiteral("<pattern")), qPrintable(svg));
    QVERIFY2(svg.contains(QStringLiteral("fill=\"url(#")), qPrintable(svg));
    QVERIFY2(svg.contains(QStringLiteral("viewBox=\"0 0 24 16\"")), qPrintable(svg));
    // The pattern tile is an <image> inside <defs>; only the white background
    // is a document-level raster fallback.
    const int defsEnd = svg.indexOf(QStringLiteral("</defs>"));
    QVERIFY(defsEnd >= 0);
    QCOMPARE(svg.mid(defsEnd).count(QStringLiteral("<image ")), 1);

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("primitives.svg"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    const QByteArray bytes = svg.toUtf8();
    QCOMPARE(file.write(bytes), qint64(bytes.size()));
    file.close();
    QVERIFY(QFileInfo(path).size() > 0);

    // Keep the generated file for a command-line run that wants to inspect it.
    const QString keep = qEnvironmentVariable("PICTURA_SVG_OUT");
    if (!keep.isEmpty()) {
        QFile out(keep);
        QVERIFY(out.open(QIODevice::WriteOnly));
        QCOMPARE(out.write(bytes), qint64(bytes.size()));
    }
}

QTEST_MAIN(ExportAsTest)
#include "tst_export_as.moc"
