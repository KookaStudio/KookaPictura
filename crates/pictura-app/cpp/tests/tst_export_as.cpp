// File > Export As… — the CC-style dialog (#457).

#include <QtTest/QtTest>

#include "export_as_dialog.h"

#include <QtCore/QTemporaryDir>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

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
    void imageSizeAndCanvas();
    void layerList();
    void saveToBytes();
    void sizeEstimate();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ExportAsTest::formatsAndControls()
{
    pictura::ExportAsDialog dialog(entries());
    auto* format = child<QComboBox>(dialog, "exportAsFormat");
    QVERIFY(format);
    QCOMPARE(format->count(), 4);
    QCOMPARE(format->itemText(0), QStringLiteral("PNG"));
    QCOMPARE(format->itemText(1), QStringLiteral("JPG"));
    QCOMPARE(format->itemText(2), QStringLiteral("PNG-8"));
    QCOMPARE(format->itemText(3), QStringLiteral("GIF"));

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

QTEST_MAIN(ExportAsTest)
#include "tst_export_as.moc"
