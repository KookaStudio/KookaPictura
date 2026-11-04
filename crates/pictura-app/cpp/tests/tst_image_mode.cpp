// Image > Mode: color-mode and bit-depth conversions, ported from photorust.

#include <QtTest/QtTest>

#include "bitmap_mode_dialog.h"
#include "commands.h"
#include "frame.h"
#include "indexed_color_dialog.h"

#include "api/document_bridge.h"
#include "api/paint_bridge.h"

#include "qt_test_support.h"

class ImageModeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void menuChecksTheModeAndDimsUnavailableTargets();
    void grayscaleIsOneStateAndUndoRestoresColor();
    void cmykAndLabSaveInTheirMode();
    void indexedPreviewsCancelsAndFlattens();
    void bitmapNeedsGrayscaleAndSavesAsBitmap();
    void depthConversionsSaveTheirDepth();
    void indexedDialogEnablesOnlyMeaningfulOptions();
    void bitmapDialogOffersThreeMethods();

private:
    // A new white RGB document filled with `color`; returns its view.
    pictura::PictureView* newFilled(const QColor& color);
    // Save `view` to a temporary PSD, reopen it in a fresh view, and report the
    // reopened document's mode and depth.
    QPair<QString, int> reopened(pictura::PictureView* view);
    QAction* action(const char* id) const { return window_->registry()->action(id); }
    void closeActive() { window_->closeDocument(window_->activeDocumentIndex(), false); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void ImageModeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

pictura::PictureView* ImageModeTest::newFilled(const QColor& color)
{
    if (!window_->newDocument(QStringLiteral("Mode"), 16, 12, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, color.rgba(), -1, QStringLiteral("normal"), 100, false);
    view->deselect();
    window_->registry()->refresh();
    return view;
}

QPair<QString, int> ImageModeTest::reopened(pictura::PictureView* view)
{
    static int serial = 0;
    const QString path = dir_.filePath(QStringLiteral("mode%1.psd").arg(++serial));
    if (!view->save(path)) {
        return {QStringLiteral("save failed"), 0};
    }
    pictura::PictureView reader;
    if (!reader.open(path)) {
        return {QStringLiteral("open failed"), 0};
    }
    return {pictura::image_mode(reader), pictura::image_depth_bits(reader)};
}

void ImageModeTest::menuChecksTheModeAndDimsUnavailableTargets()
{
    window_->registry()->refresh();
    QVERIFY(!action(pictura::command_ids::ImageModeGrayscale)->isEnabled());
    QVERIFY(newFilled(QColor(200, 40, 40)));

    QVERIFY(action(pictura::command_ids::ImageModeRgb)->isChecked());
    QVERIFY(action(pictura::command_ids::ImageModeRgb)->isEnabled());
    QVERIFY(action(pictura::command_ids::ImageMode8Bits)->isChecked());
    for (const char* id :
         {pictura::command_ids::ImageModeGrayscale, pictura::command_ids::ImageModeIndexed,
          pictura::command_ids::ImageModeCmyk, pictura::command_ids::ImageModeLab,
          pictura::command_ids::ImageMode16Bits, pictura::command_ids::ImageMode32Bits}) {
        QVERIFY2(action(id)->isEnabled(), id);
        QVERIFY2(!action(id)->isChecked(), id);
    }
    QVERIFY(!action(pictura::command_ids::ImageModeBitmap)->isEnabled());
    for (const char* name : {"Duotone", "Multichannel", "Color Table"}) {
        QAction* stub = window_->registry()->action(pictura::commandIdForPath(
            {QStringLiteral("Image"), QStringLiteral("Mode"), QString::fromUtf8(name)}));
        QVERIFY2(stub && !stub->isEnabled(), name);
    }
    closeActive();
}

void ImageModeTest::grayscaleIsOneStateAndUndoRestoresColor()
{
    pictura::PictureView* view = newFilled(QColor(200, 40, 40));
    QVERIFY(view);
    const QRgb before = view->composite_argb(4, 4);
    const int history = view->history_count();

    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("grayscale")));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("grayscale"));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_count() - 1), QStringLiteral("Convert Mode"));
    // 0.299 * 200 + 0.587 * 40 + 0.114 * 40 = 87.84.
    QCOMPARE(qRed(view->composite_argb(4, 4)), 88);
    QCOMPARE(qBlue(view->composite_argb(4, 4)), 88);
    window_->registry()->refresh();
    QVERIFY(action(pictura::command_ids::ImageModeBitmap)->isEnabled());

    QVERIFY(view->undo());
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QCOMPARE(view->composite_argb(4, 4), before);
    // Converting to the current mode is refused and records nothing.
    QVERIFY(!pictura::convert_image_mode(*view, QStringLiteral("rgb")));
    closeActive();
}

void ImageModeTest::cmykAndLabSaveInTheirMode()
{
    pictura::PictureView* view = newFilled(QColor(30, 120, 220));
    QVERIFY(view);
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("cmyk")));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("cmyk"));
    QCOMPARE(reopened(view), qMakePair(QStringLiteral("cmyk"), 8));

    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("lab")));
    QCOMPARE(reopened(view), qMakePair(QStringLiteral("lab"), 8));
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("rgb")));
    QCOMPARE(reopened(view), qMakePair(QStringLiteral("rgb"), 8));
    closeActive();
}

void ImageModeTest::indexedPreviewsCancelsAndFlattens()
{
    pictura::PictureView* view = newFilled(QColor(10, 200, 90));
    QVERIFY(view);
    QVERIFY(view->add_layer(0) >= 0);
    QCOMPARE(view->layer_count(), 2);
    const int history = view->history_count();

    QVERIFY(pictura::preview_indexed(*view, 1, 256, 0, 0));
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->history_count(), history);
    QVERIFY(pictura::cancel_mode_preview(*view));
    QCOMPARE(view->layer_count(), 2);
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QVERIFY(!pictura::cancel_mode_preview(*view));

    QVERIFY(pictura::preview_indexed(*view, 2, 8, 1, 75));
    QVERIFY(pictura::convert_to_indexed(*view, 1, 256, 0, 0));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("indexed"));
    // Web snaps 200 to 0xCC and 90 to 0x66.
    QCOMPARE(view->composite_argb(3, 3), QColor(0, 204, 102).rgba());
    window_->registry()->refresh();
    QVERIFY(action(pictura::command_ids::ImageModeIndexed)->isChecked());
    QVERIFY(!action(pictura::command_ids::ImageMode16Bits)->isEnabled());
    QCOMPARE(reopened(view), qMakePair(QStringLiteral("indexed"), 8));
    closeActive();
}

void ImageModeTest::bitmapNeedsGrayscaleAndSavesAsBitmap()
{
    pictura::PictureView* view = newFilled(QColor(250, 250, 250));
    QVERIFY(view);
    QVERIFY(!pictura::image_mode_available(*view, QStringLiteral("bitmap")));
    QVERIFY(!pictura::convert_to_bitmap(*view, 0));
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("grayscale")));
    QVERIFY(pictura::convert_to_bitmap(*view, 2));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("bitmap"));
    QCOMPARE(pictura::image_depth_bits(*view), 1);
    QCOMPARE(view->layer_count(), 0);
    QCOMPARE(view->composite_argb(2, 2), QColor(255, 255, 255).rgba());
    window_->registry()->refresh();
    QVERIFY(!action(pictura::command_ids::ImageModeRgb)->isEnabled());
    QVERIFY(action(pictura::command_ids::ImageModeGrayscale)->isEnabled());
    QCOMPARE(reopened(view), qMakePair(QStringLiteral("bitmap"), 1));

    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("grayscale")));
    QCOMPARE(view->layer_count(), 1);
    closeActive();
}

void ImageModeTest::depthConversionsSaveTheirDepth()
{
    pictura::PictureView* view = newFilled(QColor(90, 60, 30));
    QVERIFY(view);
    QVERIFY(pictura::convert_image_depth(*view, 16));
    QCOMPARE(pictura::image_depth_bits(*view), 16);
    window_->registry()->refresh();
    QVERIFY(action(pictura::command_ids::ImageMode16Bits)->isChecked());
    QVERIFY(!action(pictura::command_ids::ImageModeIndexed)->isEnabled());
    QCOMPARE(reopened(view), qMakePair(QStringLiteral("rgb"), 16));

    QVERIFY(pictura::convert_image_depth(*view, 32));
    QCOMPARE(reopened(view), qMakePair(QStringLiteral("rgb"), 32));
    // Leaving 32-bit goes through the HDR Conversion bridge, not this one.
    QVERIFY(!pictura::convert_image_depth(*view, 8));
    window_->registry()->refresh();
    QVERIFY(!action(pictura::command_ids::ImageModeCmyk)->isEnabled());
    QVERIFY(view->convert_depth(8, 0.0, 1.0));
    QCOMPARE(pictura::image_depth_bits(*view), 8);
    closeActive();
}

void ImageModeTest::indexedDialogEnablesOnlyMeaningfulOptions()
{
    pictura::IndexedColorDialog local(false);
    auto* palette = local.findChild<QComboBox*>(QStringLiteral("indexedPalette"));
    auto* colors = local.findChild<QSpinBox*>(QStringLiteral("indexedColors"));
    auto* dither = local.findChild<QComboBox*>(QStringLiteral("indexedDither"));
    auto* amount = local.findChild<QSpinBox*>(QStringLiteral("indexedAmount"));
    QVERIFY(palette && colors && dither && amount);
    QCOMPARE(palette->findData(0), -1);
    QCOMPARE(local.palette(), 2);
    QCOMPARE(local.dither(), 1);
    QCOMPARE(local.amount(), 75);
    QVERIFY(local.preview());
    QVERIFY(colors->isEnabled());

    QSignalSpy changed(&local, &pictura::IndexedColorDialog::optionsChanged);
    palette->setCurrentIndex(palette->findData(1));
    QCOMPARE(changed.count(), 1);
    QVERIFY(!colors->isEnabled());
    QCOMPARE(local.colors(), 216);
    dither->setCurrentIndex(dither->findData(0));
    QVERIFY(!amount->isEnabled());

    pictura::IndexedColorDialog exact(true);
    QCOMPARE(exact.palette(), 0);
    QCOMPARE(exact.colors(), 256);
    QVERIFY(!exact.findChild<QComboBox*>(QStringLiteral("indexedDither"))->isEnabled());
}

void ImageModeTest::bitmapDialogOffersThreeMethods()
{
    pictura::BitmapModeDialog dialog;
    auto* method = dialog.findChild<QComboBox*>(QStringLiteral("bitmapMethod"));
    QVERIFY(method);
    QCOMPARE(method->count(), 3);
    QCOMPARE(dialog.method(), 0);
    method->setCurrentIndex(2);
    QCOMPARE(dialog.method(), 2);
}

QTEST_MAIN(ImageModeTest)
#include "tst_image_mode.moc"
