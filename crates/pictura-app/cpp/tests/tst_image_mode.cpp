// Image > Mode: color-mode and bit-depth conversions, ported from photorust.

#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtTest/QtTest>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTableWidget>

#include <functional>

#include "bitmap_mode_dialog.h"
#include "commands.h"
#include "color_table_dialog.h"
#include "frame.h"
#include "indexed_color_dialog.h"

#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/selection_bridge.h"
#include "api/paint_bridge.h"

#include "qt_test_support.h"

class ImageModeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void menuChecksTheModeAndDimsUnavailableTargets();
    void grayscaleIsOneStateAndUndoRestoresColor();
    void multichannelPromptsAndUndoes();
    void multichannelDimsUnsupportedSources();
    void multichannelRejectsLayerInsertion();
    void multichannelReturnsToRgbAndUndoes();
    void cmykAndLabSaveInTheirMode();
    void indexedPreviewsCancelsAndFlattens();
    void bitmapNeedsGrayscaleAndSavesAsBitmap();
    void depthConversionsSaveTheirDepth();
    void indexedDialogEnablesOnlyMeaningfulOptions();
    void bitmapDialogShowsMethodOptions();
    void recordedBitmapMethodsReplayTheirOptions();
    void modalBitmapReplayPrefillsEditsAndCancels();
    void silentBitmapReplayRefusesMissingPattern();
    void exactFallsBackWhenOptionsExceedCapacity();
    void exactRemainsAvailableAtCapacity();
    void indexedColorTableCancelKeepsDocument();
    void colorTableCommitCanBeUndone();
    void colorTableInterpolatesRange();
    void colorTableActRoundTrip();
    void colorTableRejectsShortAct();
    void colorTableUnchangedAcceptance();

private:
    // A new white RGB document filled with `color`; returns its view.
    pictura::PictureView* newFilled(const QColor& color);
    pictura::PictureView* newBitmapSource();
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
          pictura::command_ids::ImageModeMultichannel,
          pictura::command_ids::ImageMode16Bits, pictura::command_ids::ImageMode32Bits}) {
        QVERIFY2(action(id)->isEnabled(), id);
        QVERIFY2(!action(id)->isChecked(), id);
    }
    QVERIFY(!action(pictura::command_ids::ImageModeBitmap)->isEnabled());
    for (const char* name : {"Duotone", "Color Table"}) {
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

void ImageModeTest::multichannelPromptsAndUndoes()
{
    auto* view = newFilled(QColor(100, 150, 200));
    QVERIFY(view);
    QVERIFY(view->add_layer(0) >= 0);
    const int history = view->history_count();
    QAction* multichannel = action(pictura::command_ids::ImageModeMultichannel);
    QVERIFY(multichannel && multichannel->isEnabled());

    for (auto choice : {QMessageBox::Cancel, QMessageBox::Ok}) {
        bool prompted = false;
        QTimer::singleShot(0, window_.get(), [&] {
            for (QWidget* widget : QApplication::topLevelWidgets()) {
                auto* box = qobject_cast<QMessageBox*>(widget);
                if (box && box->isVisible() && box->button(choice)) {
                    prompted = true;
                    box->button(choice)->click();
                    break;
                }
            }
        });
        multichannel->trigger();
        QVERIFY(prompted);
        if (choice == QMessageBox::Cancel) {
            QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
            QCOMPARE(view->layer_count(), 2);
            QCOMPARE(view->history_count(), history);
        }
    }
    QVERIFY(multichannel->isChecked());
    QCOMPARE(view->layer_count(), 0);
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Convert Mode"));
    const QStringList names{QStringLiteral("Cyan"), QStringLiteral("Magenta"),
                            QStringLiteral("Yellow")};
    QCOMPARE(pictura::channels_alpha_names(*view), names);
    for (int channel = 0; channel < names.size(); ++channel) {
        QVERIFY(pictura::channels_alpha_is_spot(*view, channel));
    }

    QVERIFY(view->undo());
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("rgb"));
    QCOMPARE(view->layer_count(), 2);
    closeActive();
}

void ImageModeTest::multichannelDimsUnsupportedSources()
{
    QAction* multichannel = action(pictura::command_ids::ImageModeMultichannel);
    QVERIFY(multichannel);

    auto* view = newFilled(QColor(32, 71, 120));
    QVERIFY(view && pictura::convert_to_indexed(*view, 1, 256, 0, 0));
    window_->registry()->refresh();
    QVERIFY(!multichannel->isEnabled());
    closeActive();

    view = newFilled(QColor(32, 71, 120));
    QVERIFY(view);
    QVERIFY(pictura::convert_image_depth(*view, 32));
    QCOMPARE(pictura::image_depth_bits(*view), 32);
    window_->registry()->refresh();
    QVERIFY(!multichannel->isEnabled());
    closeActive();
}

void ImageModeTest::multichannelRejectsLayerInsertion()
{
    auto* view = newFilled(QColor(32, 71, 120));
    QVERIFY(view && pictura::convert_image_mode(*view, QStringLiteral("multichannel")));
    const int history = view->history_count();
    QVERIFY(view->add_layer(-1) < 0);
    QCOMPARE(view->layer_count(), 0);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("multichannel"));
    closeActive();
}

void ImageModeTest::multichannelReturnsToRgbAndUndoes()
{
    auto* view = newFilled(QColor(32, 71, 120));
    QVERIFY(view);
    view->select_all();
    QVERIFY(pictura::save_selection_to(*view, QStringLiteral("new"), QStringLiteral("Saved"),
                                       QStringLiteral("new")));
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("multichannel")));
    const int history = view->history_count();
    const auto preview = view->composite_argb(4, 4);
    const QStringList names{QStringLiteral("Cyan"), QStringLiteral("Magenta"),
                            QStringLiteral("Yellow"), QStringLiteral("Saved")};
    QCOMPARE(pictura::channels_alpha_names(*view), names);

    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("rgb")));
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->composite_argb(4, 4), preview);
    QVERIFY(!pictura::channels_multichannel_plates(*view));
    QCOMPARE(pictura::channels_alpha_names(*view), names);

    QVERIFY(view->undo());
    QCOMPARE(view->layer_count(), 0);
    QVERIFY(pictura::channels_multichannel_plates(*view));
    QCOMPARE(pictura::channels_alpha_names(*view), names);
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

namespace {

void answerBitmapDialog(QTimer& timer, const std::function<bool(pictura::BitmapModeDialog&)>& answer)
{
    QObject::connect(&timer, &QTimer::timeout, &timer, [&timer, answer] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (!widget->isVisible()) {
                continue;
            }
            if (auto* dialog = qobject_cast<pictura::BitmapModeDialog*>(widget)) {
                timer.stop();
                if (answer(*dialog)) {
                    dialog->accept();
                } else {
                    dialog->reject();
                }
                return;
            }
            if (auto* warning = qobject_cast<QMessageBox*>(widget);
                warning && warning->text() == QStringLiteral("Flatten layers?")) {
                if (auto* ok = warning->button(QMessageBox::Ok)) {
                    ok->click();
                    return;
                }
            }
        }
    });
    timer.start(2);
}

QVector<quint32> bitmapPixels(const pictura::PictureView& view)
{
    QVector<quint32> pixels;
    for (int y = 0; y < 12; ++y) {
        for (int x = 0; x < 16; ++x) {
            pixels.append(view.composite_argb(x, y));
        }
    }
    return pixels;
}

} // namespace

pictura::PictureView* ImageModeTest::newBitmapSource()
{
    auto* view = newFilled(QColor(128, 128, 128));
    if (!view || !pictura::convert_image_mode(*view, QStringLiteral("grayscale"))) {
        return nullptr;
    }
    window_->registry()->refresh();
    return view;
}

void ImageModeTest::bitmapDialogShowsMethodOptions()
{
    auto* view = newBitmapSource();
    QVERIFY(view);
    pictura::BitmapModeDialog dialog(*view);
    auto* method = dialog.findChild<QComboBox*>(QStringLiteral("bitmapMethod"));
    auto* screen = dialog.findChild<QWidget*>(QStringLiteral("bitmapScreenOptions"));
    auto* pattern = dialog.findChild<QWidget*>(QStringLiteral("bitmapPatternOptions"));
    auto* unit = dialog.findChild<QComboBox*>(QStringLiteral("bitmapFrequencyUnit"));
    auto* frequency = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bitmapFrequency"));
    auto* shape = dialog.findChild<QComboBox*>(QStringLiteral("bitmapShape"));
    QVERIFY(method && screen && pattern && unit && frequency && shape);
    QCOMPARE(method->count(), 5);
    QCOMPARE(dialog.method(), 0);
    QVERIFY(screen->isHidden());
    QVERIFY(pattern->isHidden());

    method->setCurrentIndex(method->findData(3));
    QVERIFY(!screen->isHidden());
    QVERIFY(pattern->isHidden());
    QCOMPARE(shape->count(), 6);
    QCOMPARE(frequency->minimum(), 1.0);
    QCOMPARE(frequency->maximum(), 999.999);
    unit->setCurrentIndex(1);
    QCOMPARE(frequency->minimum(), 0.4);
    QCOMPARE(frequency->maximum(), 400.0);

    method->setCurrentIndex(method->findData(4));
    QVERIFY(screen->isHidden());
    QVERIFY(!pattern->isHidden());
    method->setCurrentIndex(method->findData(2));
    QCOMPARE(dialog.method(), 2);
    QVERIFY(screen->isHidden());
    QVERIFY(pattern->isHidden());
    closeActive();
}

void ImageModeTest::recordedBitmapMethodsReplayTheirOptions()
{
    const QString id = QString::fromLatin1(pictura::command_ids::ImageModeBitmap);
    auto* registry = window_->registry();
    for (int selected : {3, 4}) {
        auto* view = newBitmapSource();
        QVERIFY(view);
        const int originalHistory = view->history_count();
        pictura::CommandParams recorded;
        registry->setRecordingSink([&](const QString& commandId, const auto& params) {
            if (commandId == id) {
                recorded = params;
            }
        });
        QTimer timer;
        answerBitmapDialog(timer, [selected](pictura::BitmapModeDialog& dialog) {
            auto* method = dialog.findChild<QComboBox*>(QStringLiteral("bitmapMethod"));
            method->setCurrentIndex(method->findData(selected));
            if (selected == 3) {
                dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bitmapFrequency"))->setValue(9.0);
                dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bitmapAngle"))->setValue(23.0);
                dialog.findChild<QComboBox*>(QStringLiteral("bitmapShape"))->setCurrentIndex(3);
            } else {
                auto* pattern = dialog.findChild<QComboBox*>(QStringLiteral("bitmapPattern"));
                pattern->setCurrentIndex(pattern->findData(QStringLiteral("Pictura Checkerboard")));
            }
            return true;
        });
        const bool dispatched = registry->dispatch(id);
        registry->setRecordingSink({});
        QVERIFY(dispatched);
        QCOMPARE(recorded.values.value(QStringLiteral("method")).toInt(), selected);
        QCOMPARE(view->history_count(), originalHistory + 1);
        QCOMPARE(pictura::image_depth_bits(*view), 1);
        const auto expected = bitmapPixels(*view);
        QVERIFY(expected.contains(qRgb(0, 0, 0)));
        QVERIFY(expected.contains(qRgb(255, 255, 255)));
        closeActive();

        view = newBitmapSource();
        QVERIFY(view);
        const int replayHistory = view->history_count();
        QVERIFY(registry->executeRecorded(id, recorded));
        QCOMPARE(bitmapPixels(*view), expected);
        QCOMPARE(view->history_count(), replayHistory + 1);
        QCOMPARE(view->history_label(replayHistory), QStringLiteral("Convert Mode"));
        closeActive();
    }
}

void ImageModeTest::modalBitmapReplayPrefillsEditsAndCancels()
{
    const QString id = QString::fromLatin1(pictura::command_ids::ImageModeBitmap);
    auto* registry = window_->registry();
    for (int selected : {3, 4}) {
        auto* view = newBitmapSource();
        QVERIFY(view);
        const int history = view->history_count();
        pictura::CommandParams params;
        params.values = {
            {QStringLiteral("method"), selected},
            {QStringLiteral("frequency"), 11.0},
            {QStringLiteral("perCm"), true},
            {QStringLiteral("angle"), -22.0},
            {QStringLiteral("shape"), 4},
            {QStringLiteral("patternId"), QStringLiteral("Pictura Checkerboard")},
        };
        bool prefilled = false;
        QTimer accepted;
        answerBitmapDialog(accepted, [&](pictura::BitmapModeDialog& dialog) {
            if (dialog.method() != selected) {
                return false;
            }
            if (selected == 3) {
                auto* frequency = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bitmapFrequency"));
                const auto* unit = dialog.findChild<QComboBox*>(QStringLiteral("bitmapFrequencyUnit"));
                const auto* angle = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("bitmapAngle"));
                const auto* shape = dialog.findChild<QComboBox*>(QStringLiteral("bitmapShape"));
                prefilled = frequency->value() == 11.0 && unit->currentIndex() == 1
                            && angle->value() == -22.0 && shape->currentIndex() == 4;
                frequency->setValue(4.0);
            } else {
                auto* pattern = dialog.findChild<QComboBox*>(QStringLiteral("bitmapPattern"));
                prefilled = pattern->currentData().toString() == QStringLiteral("Pictura Checkerboard");
                pattern->setCurrentIndex(pattern->findData(QStringLiteral("Pictura Diagonal")));
            }
            return true;
        });
        QVERIFY(registry->executeRecorded(id, &params, pictura::CommandMode::Modal));
        QVERIFY(prefilled);
        QCOMPARE(view->history_count(), history + 1);
        QCOMPARE(pictura::image_depth_bits(*view), 1);
        if (selected == 3) {
            QCOMPARE(params.values.value(QStringLiteral("frequency")).toDouble(), 4.0);
        } else {
            QCOMPARE(params.values.value(QStringLiteral("patternId")).toString(),
                     QStringLiteral("Pictura Diagonal"));
        }
        closeActive();

        view = newBitmapSource();
        QVERIFY(view);
        const int cancelHistory = view->history_count();
        const auto before = params.values;
        QTimer rejected;
        answerBitmapDialog(rejected, [](auto&) { return false; });
        QVERIFY(!registry->executeRecorded(id, &params, pictura::CommandMode::Modal));
        QCOMPARE(params.values, before);
        QCOMPARE(view->history_count(), cancelHistory);
        QCOMPARE(pictura::image_mode(*view), QStringLiteral("grayscale"));
        closeActive();
    }
}

void ImageModeTest::silentBitmapReplayRefusesMissingPattern()
{
    auto* view = newBitmapSource();
    QVERIFY(view);
    const int history = view->history_count();
    const auto before = bitmapPixels(*view);
    pictura::CommandParams params;
    params.values.insert(QStringLiteral("method"), 4);
    params.values.insert(QStringLiteral("patternId"), QStringLiteral("missing-test-pattern"));
    QVERIFY(!window_->registry()->executeRecorded(
        QString::fromLatin1(pictura::command_ids::ImageModeBitmap), params));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("grayscale"));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(bitmapPixels(*view), before);
    closeActive();
}

namespace {

bool openExactSource(pictura::PictureView& view, const QString& path, int opaqueCount)
{
    QImage image(opaqueCount + 1, 1, QImage::Format_ARGB32);
    for (int i = 0; i < opaqueCount; ++i) {
        image.setPixel(i, 0, qRgb(i, 30, 80));
    }
    image.setPixel(opaqueCount, 0, qRgba(0, 0, 0, 0));
    return image.save(path) && view.open_image(path);
}

QDialog* visibleColorTableDialog()
{
    for (auto* widget : QApplication::topLevelWidgets()) {
        if (widget->objectName() == QStringLiteral("indexedColorTableDialog")
            && widget->isVisible()) {
            return qobject_cast<QDialog*>(widget);
        }
    }
    return nullptr;
}

} // namespace

void ImageModeTest::exactFallsBackWhenOptionsExceedCapacity()
{
    pictura::PictureView view;
    QVERIFY(openExactSource(view, dir_.filePath(QStringLiteral("exact-overflow.png")), 255));
    QVERIFY(pictura::indexed_exact_available_with_options(view, true, {}));
    pictura::IndexedColorDialog dialog(true, true, nullptr);
    connect(&dialog, &pictura::IndexedColorDialog::optionsChanged, &dialog, [&] {
        dialog.setExactAvailable(pictura::indexed_exact_available_with_options(
            view, dialog.transparency(), dialog.forcedColors()));
    });
    auto* forced = dialog.findChild<QComboBox*>(QStringLiteral("indexedForced"));
    QVERIFY(forced);
    forced->setCurrentIndex(1);
    QCOMPARE(dialog.palette(), 4);
    QCOMPARE(dialog.findChild<QComboBox*>(QStringLiteral("indexedPalette"))->findData(0), -1);
}

void ImageModeTest::exactRemainsAvailableAtCapacity()
{
    pictura::PictureView view;
    QVERIFY(openExactSource(view, dir_.filePath(QStringLiteral("exact-capacity.png")), 253));
    QVERIFY(pictura::indexed_exact_available_with_options(view, true, {}));
    pictura::IndexedColorDialog dialog(true, true, nullptr);
    connect(&dialog, &pictura::IndexedColorDialog::optionsChanged, &dialog, [&] {
        dialog.setExactAvailable(pictura::indexed_exact_available_with_options(
            view, dialog.transparency(), dialog.forcedColors()));
    });
    auto* forced = dialog.findChild<QComboBox*>(QStringLiteral("indexedForced"));
    QVERIFY(forced);
    forced->setCurrentIndex(1);
    QCOMPARE(dialog.palette(), 0);
    QVERIFY(pictura::convert_to_indexed_with_options(
        view, dialog.palette(), dialog.colors(), dialog.dither(), dialog.amount(),
        dialog.transparency(), dialog.forcedColors()));
    QCOMPARE(pictura::indexed_color_table(view).size(), qsizetype(256 * 4));
}

void ImageModeTest::indexedColorTableCancelKeepsDocument()
{
    auto* view = newFilled(Qt::red);
    QVERIFY(view);
    QVERIFY(pictura::convert_to_indexed(*view, 0, 256, 0, 0));
    window_->registry()->refresh();
    const auto before = pictura::indexed_color_table(*view);
    const auto history = view->history_count();
    auto* command = action(pictura::command_ids::ImageModeColorTable);
    QVERIFY(command && command->isEnabled());
    bool opened = false;
    QTimer::singleShot(0, this, [&] {
        auto* dialog = visibleColorTableDialog();
        if (!dialog) {
            return;
        }
        auto* table = dialog->findChild<QTableWidget*>(QStringLiteral("indexedColorTable"));
        auto* clear = dialog->findChild<QPushButton*>(QStringLiteral("tableSetTransparent"));
        opened = table && table->item(0, 0) && clear;
        if (opened) {
            table->setCurrentCell(0, 0);
            clear->click();
        }
        dialog->reject();
    });
    command->trigger();
    QVERIFY(opened);
    QCOMPARE(pictura::indexed_color_table(*view), before);
    QCOMPARE(view->history_count(), history);
    closeActive();
}

void ImageModeTest::colorTableCommitCanBeUndone()
{
    auto* view = newFilled(Qt::red);
    QVERIFY(view);
    QVERIFY(pictura::convert_to_indexed(*view, 0, 256, 0, 0));
    window_->registry()->refresh();
    const auto before = pictura::indexed_color_table(*view);
    const auto history = view->history_count();
    auto edited = before;
    edited[3] = 0;
    auto* command = action(pictura::command_ids::ImageModeColorTable);
    QVERIFY(command && command->isEnabled());
    QTimer::singleShot(0, this, [] {
        if (auto* dialog = visibleColorTableDialog()) {
            auto* clear = dialog->findChild<QPushButton*>(QStringLiteral("tableSetTransparent"));
            if (clear) {
                clear->click();
            }
            dialog->accept();
        }
    });
    command->trigger();
    QCOMPARE(pictura::indexed_color_table(*view), edited);
    QCOMPARE(qAlpha(view->composite_argb(4, 4)), 0);
    QCOMPARE(view->history_count(), history + 1);
    QVERIFY(view->undo());
    QCOMPARE(pictura::indexed_color_table(*view), before);
    QCOMPARE(qAlpha(view->composite_argb(4, 4)), 255);
    closeActive();
}

void ImageModeTest::colorTableInterpolatesRange()
{
    pictura::PictureView view;
    QVERIFY(openExactSource(view, dir_.filePath(QStringLiteral("interpolate.png")), 2));
    QVERIFY(pictura::convert_to_indexed(view, 0, 256, 0, 0));
    pictura::ColorTableDialog dialog(pictura::indexed_color_table(view), 3);
    dialog.interpolate(0, 2, QColor(Qt::black), QColor(200, 100, 50));
    QCOMPARE(dialog.table(), QByteArray::fromHex("000000ff643219ffc86432ff"));
    QVERIFY(pictura::apply_indexed_color_table(view, dialog.table()));
    QCOMPARE(view.composite_argb(0, 0), qRgb(0, 0, 0));
    QCOMPARE(view.composite_argb(1, 0), qRgb(100, 50, 25));
    QCOMPARE(view.composite_argb(2, 0), qRgb(200, 100, 50));
}

void ImageModeTest::colorTableActRoundTrip()
{
    const auto table = QByteArray::fromHex("104080ff90602000");
    pictura::ColorTableDialog dialog(table, 2);
    pictura::ColorTableDialog reader(QByteArray::fromHex("000000ffffffffff"), 2);
    QVERIFY(reader.loadAct(dialog.saveAct()));
    QCOMPARE(reader.table(), table);
}

void ImageModeTest::colorTableRejectsShortAct()
{
    const auto table = QByteArray::fromHex("104080ff90602000");
    pictura::ColorTableDialog dialog(table, 2);
    const auto shortAct = pictura::encode_indexed_act(QByteArray::fromHex("000000ff"));
    QVERIFY(!dialog.loadAct(shortAct));
    QCOMPARE(dialog.table(), table);
}

void ImageModeTest::colorTableUnchangedAcceptance()
{
    auto* view = newFilled(Qt::blue);
    QVERIFY(view);
    QVERIFY(pictura::convert_to_indexed(*view, 0, 256, 0, 0));
    window_->registry()->refresh();
    const auto before = pictura::indexed_color_table(*view);
    const auto history = view->history_count();
    auto* command = action(pictura::command_ids::ImageModeColorTable);
    QVERIFY(command && command->isEnabled());
    bool accepted = false;
    QTimer::singleShot(0, this, [&accepted] {
        if (auto* dialog = visibleColorTableDialog()) {
            accepted = true;
            dialog->accept();
        }
    });
    command->trigger();
    QVERIFY(accepted);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(pictura::indexed_color_table(*view), before);
    closeActive();
}

PICTURA_QT_TEST_MAIN(ImageModeTest)
#include "tst_image_mode.moc"
