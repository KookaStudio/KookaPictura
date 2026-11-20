// Select > Color Range (#613): the dialog previews the coverage mask, samples
// from a canvas click, and OK selects (or narrows a live selection).

#include <QtTest/QtTest>

#include "color_range_dialog.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/proofing_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

class ColorRangeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void sampleAndSelect();
    void refinesALiveSelection();
    void samplesAddAndSubtract();
    void localizedClustersNarrowThePreview();
    void skinTonesSelectSkin();
    void outOfGamutFollowsTheProofSetup();
    void selectionAndImagePreviews();
    void ctrlTogglesTheDisplayPreview();
    void imageWindowPreviewModes();
    void previewIgnoresProofColors();
    void saveLoadRoundTrip();
    void saveLoadRejectsInvalid();
    void previewReflectsTheLiveSelection();

private:
    bool newDocument(const QString& name, int width, int height);
    bool addSquare(int x0, int y0, int x1, int y1, unsigned argb);
    bool setupRedSquare();
    bool setupSquares();
    bool setupRedPair();
    bool setupSkin();
    bool setupGreen();
    pictura::ColorRangeDialog* open();
    QAction* command() const;
    QImage canvas() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
};

void ColorRangeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void ColorRangeTest::cleanup()
{
    for (auto* dialog : window_->findChildren<pictura::ColorRangeDialog*>()) {
        dialog->reject();
    }
    QCoreApplication::processEvents();
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool ColorRangeTest::newDocument(const QString& name, int width, int height)
{
    if (!window_->newDocument(name, width, height, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    return view_ != nullptr;
}

bool ColorRangeTest::addSquare(int x0, int y0, int x1, int y1, unsigned argb)
{
    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = x0;
    square.y0 = y0;
    square.x1 = x1;
    square.y1 = y1;
    return !pictura::shape_add_layer(*view_, square, argb).isEmpty();
}

bool ColorRangeTest::setupRedSquare()
{
    return newDocument(QStringLiteral("Range"), 40, 40)
        && addSquare(2, 2, 10, 10, 0xffff0000u) && view_->sample_argb(5, 5) == 0xffff0000u;
}

bool ColorRangeTest::setupSquares()
{
    return newDocument(QStringLiteral("Squares"), 40, 40)
        && addSquare(2, 2, 10, 10, 0xffff0000u)
        && addSquare(20, 2, 28, 10, 0xff0000ffu) && view_->sample_argb(5, 5) == 0xffff0000u
        && view_->sample_argb(25, 5) == 0xff0000ffu;
}

bool ColorRangeTest::setupRedPair()
{
    return newDocument(QStringLiteral("Pair"), 200, 40)
        && addSquare(2, 2, 8, 8, 0xffff0000u)
        && addSquare(180, 2, 188, 8, 0xffff0000u)
        && view_->sample_argb(5, 5) == 0xffff0000u
        && view_->sample_argb(184, 5) == 0xffff0000u;
}

bool ColorRangeTest::setupSkin()
{
    return newDocument(QStringLiteral("Skin"), 40, 40)
        && addSquare(2, 2, 10, 10, 0xffe0ac8cu)
        && addSquare(20, 2, 28, 10, 0xff0000ffu) && view_->sample_argb(5, 5) == 0xffe0ac8cu
        && view_->sample_argb(24, 5) == 0xff0000ffu;
}

bool ColorRangeTest::setupGreen()
{
    return newDocument(QStringLiteral("Gamut"), 40, 40)
        && addSquare(2, 2, 10, 10, 0xff00ff00u) && view_->sample_argb(5, 5) == 0xff00ff00u;
}

QAction* ColorRangeTest::command() const
{
    return window_->registry()->action(
        pictura::commandIdForPath({QStringLiteral("Select"), QStringLiteral("Color Range…")}));
}

pictura::ColorRangeDialog* ColorRangeTest::open()
{
    QAction* action = command();
    window_->registry()->refresh();
    if (!action || !action->isEnabled()) {
        return nullptr;
    }
    action->trigger();
    QCoreApplication::processEvents();
    return window_->findChild<pictura::ColorRangeDialog*>();
}

QImage ColorRangeTest::canvas() const
{
    return view_->canvas_display_image(0, 0, 0, view_->document_width(), view_->document_height());
}

void ColorRangeTest::sampleAndSelect()
{
    QVERIFY(setupRedSquare());
    pictura::ToolController* tools = window_->findChild<pictura::ToolController*>();
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog && dialog->isVisible() && tools);
    QVERIFY(tools->canvasSamplerActive());

    // A canvas click samples the red instead of reaching the active tool.
    const int before = view_->history_count();
    emit window_->imageView()->mousePressed(QPointF(5, 5), Qt::LeftButton, 0);
    QCOMPARE(dialog->sampledColor(), QColor(Qt::red));
    QCOMPARE(view_->history_count(), before);

    // The preview is the mask: white over the square, black elsewhere.
    QImage preview = dialog->previewForTest();
    QCOMPARE(preview.size(), QSize(40, 40));
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
    QCOMPARE(qGray(preview.pixel(30, 30)), 0);

    // A tonal band needs no sample: Highlights takes the white, not the red.
    auto* select = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelect"));
    QVERIFY(select);
    select->setCurrentIndex(select->findData(8));
    QVERIFY(!tools->canvasSamplerActive());
    preview = dialog->previewForTest();
    QVERIFY(qGray(preview.pixel(30, 30)) > 200);
    QVERIFY(qGray(preview.pixel(5, 5)) < 100);
    select->setCurrentIndex(select->findData(0));
    QVERIFY(tools->canvasSamplerActive());

    // OK selects the red square as one "Color Range" state.
    dialog->findChild<QDialogButtonBox*>()->button(QDialogButtonBox::Ok)->click();
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_count(), before + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Color Range"));
    QCOMPARE(view_->selection_coverage(5, 5), 255);
    QCOMPARE(view_->selection_coverage(30, 30), 0);
    QVERIFY(!tools->canvasSamplerActive());

    // Cancel changes nothing.
    dialog = open();
    QVERIFY(dialog);
    const int after = view_->history_count();
    dialog->reject();
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_count(), after);
}

void ColorRangeTest::refinesALiveSelection()
{
    QVERIFY(setupRedSquare());
    // Run on a selection, Color Range narrows it to the matching subset.
    QVERIFY(view_->select_rect(0, 0, 6, 40, QStringLiteral("new"), 0.0));
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    dialog->accept();
    QCoreApplication::processEvents();
    QCOMPARE(view_->selection_coverage(4, 4), 255);
    QCOMPARE(view_->selection_coverage(8, 8), 0);
    QCOMPARE(view_->selection_coverage(3, 30), 0);
}

void ColorRangeTest::samplesAddAndSubtract()
{
    QVERIFY(setupSquares());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    emit window_->imageView()->mousePressed(QPointF(5, 5), Qt::LeftButton, 0);
    QCOMPARE(dialog->sampleCount(), 1);
    auto* add = dialog->findChild<QToolButton*>(QStringLiteral("colorRangeAdd"));
    QVERIFY(add);
    add->click();
    emit window_->imageView()->mousePressed(QPointF(25, 5), Qt::LeftButton, 0);
    QCOMPARE(dialog->sampleCount(), 2);
    QImage preview = dialog->previewForTest();
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
    QCOMPARE(qGray(preview.pixel(25, 5)), 255);

    emit window_->imageView()->mousePressed(QPointF(25, 5), Qt::LeftButton, Qt::AltModifier);
    QCOMPARE(dialog->sampleCount(), 1);
    preview = dialog->previewForTest();
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
    QCOMPARE(qGray(preview.pixel(25, 5)), 0);

    dialog->findChild<QDialogButtonBox*>()->button(QDialogButtonBox::Ok)->click();
    QCoreApplication::processEvents();
    QCOMPARE(view_->selection_coverage(5, 5), 255);
    QCOMPARE(view_->selection_coverage(25, 5), 0);
}

void ColorRangeTest::localizedClustersNarrowThePreview()
{
    QVERIFY(setupRedPair());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    auto* localized = dialog->findChild<QCheckBox*>(QStringLiteral("colorRangeLocalized"));
    auto* range = dialog->findChild<QSpinBox*>(QStringLiteral("colorRangeRange"));
    QVERIFY(localized && range);
    QVERIFY(!range->isEnabled());

    dialog->sampleAt(QPointF(5, 5));
    localized->setChecked(true);
    QVERIFY(range->isEnabled());
    range->setValue(0);
    QImage preview = dialog->previewForTest();
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
    QCOMPARE(qGray(preview.pixel(184, 5)), 0);

    range->setValue(100);
    preview = dialog->previewForTest();
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
    QCOMPARE(qGray(preview.pixel(184, 5)), 255);
}

void ColorRangeTest::skinTonesSelectSkin()
{
    QVERIFY(setupSkin());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    auto* select = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelect"));
    QVERIFY(select);
    select->setCurrentIndex(select->findData(1));
    auto* replace = dialog->findChild<QToolButton*>(QStringLiteral("colorRangeReplace"));
    QVERIFY(replace && !replace->isEnabled());
    const QImage preview = dialog->previewForTest();
    QVERIFY(qGray(preview.pixel(5, 5)) > 0);
    QCOMPARE(qGray(preview.pixel(24, 5)), 0);
}

void ColorRangeTest::outOfGamutFollowsTheProofSetup()
{
    QVERIFY(setupGreen());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    auto* select = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelect"));
    QVERIFY(select);
    const int outOfGamut = select->findData(11);
    QVERIFY(outOfGamut >= 0);
    select->setCurrentIndex(outOfGamut);
    const QImage preview = dialog->previewForTest();
    QVERIFY(qGray(preview.pixel(5, 5)) > 0);
    QCOMPARE(qGray(preview.pixel(30, 30)), 0);
    dialog->reject();
    QCoreApplication::processEvents();

    // A fresh CMYK document does not offer Out Of Gamut.
    window_->closeDocument(window_->activeDocumentIndex(), false);
    QVERIFY(newDocument(QStringLiteral("Cmyk"), 40, 40));
    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("cmyk")));
    dialog = open();
    QVERIFY(dialog);
    select = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelect"));
    QVERIFY(select);
    QCOMPARE(select->findData(11), -1);
}

void ColorRangeTest::selectionAndImagePreviews()
{
    QVERIFY(setupRedSquare());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    QImage preview = dialog->previewForTest();
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
    QCOMPARE(qGray(preview.pixel(30, 30)), 0);

    auto* image = dialog->findChild<QRadioButton*>(QStringLiteral("colorRangeImage"));
    QVERIFY(image);
    image->setChecked(true);
    QCOMPARE(dialog->displayMode(), 1);
    const QImage picture = dialog->imagePreviewForTest();
    QVERIFY(!picture.isNull());
    QCOMPARE(picture.pixelColor(5, 5), QColor(Qt::red));
    QCOMPARE(picture.pixelColor(30, 30), QColor(Qt::white));

    auto* selection = dialog->findChild<QRadioButton*>(QStringLiteral("colorRangeSelection"));
    QVERIFY(selection);
    selection->setChecked(true);
    QCOMPARE(dialog->displayMode(), 0);
    preview = dialog->previewForTest();
    QCOMPARE(qGray(preview.pixel(5, 5)), 255);
}

void ColorRangeTest::ctrlTogglesTheDisplayPreview()
{
    QVERIFY(setupRedSquare());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    QCOMPARE(dialog->displayMode(), 0);
    QTest::keyClick(dialog, Qt::Key_Control);
    QCOMPARE(dialog->displayMode(), 1);
    QTest::keyClick(dialog, Qt::Key_Control);
    QCOMPARE(dialog->displayMode(), 0);
}

void ColorRangeTest::imageWindowPreviewModes()
{
    QVERIFY(setupRedSquare());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    auto* mode = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelectionPreview"));
    QVERIFY(mode);

    QImage image = canvas();
    QCOMPARE(image.pixelColor(5, 5), QColor(Qt::red));
    QCOMPARE(image.pixelColor(30, 30), QColor(Qt::white));

    mode->setCurrentIndex(1);
    image = canvas();
    QCOMPARE(image.pixelColor(5, 5), QColor(Qt::white));
    QCOMPARE(image.pixelColor(30, 30), QColor(Qt::black));

    mode->setCurrentIndex(2);
    image = canvas();
    QCOMPARE(image.pixelColor(5, 5), QColor(Qt::red));
    QCOMPARE(image.pixelColor(30, 30), QColor(Qt::black));

    mode->setCurrentIndex(3);
    image = canvas();
    QCOMPARE(image.pixelColor(5, 5), QColor(Qt::red));
    QCOMPARE(image.pixelColor(30, 30), QColor(Qt::white));

    mode->setCurrentIndex(4);
    image = canvas();
    QCOMPARE(image.pixelColor(5, 5), QColor(Qt::red));
    const QColor quick = image.pixelColor(30, 30);
    QVERIFY(quick.red() > 200 && quick.green() < 200);

    mode->setCurrentIndex(0);
    image = canvas();
    QCOMPARE(image.pixelColor(30, 30), QColor(Qt::white));

    dialog->reject();
    QCoreApplication::processEvents();
    image = canvas();
    QCOMPARE(image.pixelColor(5, 5), QColor(Qt::red));
    QCOMPARE(image.pixelColor(30, 30), QColor(Qt::white));
}

void ColorRangeTest::previewIgnoresProofColors()
{
    QVERIFY(setupRedSquare());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    auto* mode = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelectionPreview"));
    QVERIFY(mode);
    mode->setCurrentIndex(2);
    QCOMPARE(canvas().pixelColor(5, 5), QColor(Qt::red));

    QVERIFY(pictura::set_proof_colors(*view_, true));
    QCOMPARE(canvas().pixelColor(5, 5), QColor(Qt::red));
    pictura::set_proof_colors(*view_, false);
}

void ColorRangeTest::saveLoadRoundTrip()
{
    QVERIFY(setupSquares());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    auto* add = dialog->findChild<QToolButton*>(QStringLiteral("colorRangeAdd"));
    QVERIFY(add);
    add->click();
    emit window_->imageView()->mousePressed(QPointF(25, 5), Qt::LeftButton, 0);
    auto* localized = dialog->findChild<QCheckBox*>(QStringLiteral("colorRangeLocalized"));
    auto* range = dialog->findChild<QSpinBox*>(QStringLiteral("colorRangeRange"));
    auto* invert = dialog->findChild<QCheckBox*>(QStringLiteral("colorRangeInvert"));
    auto* fuzziness = dialog->findChild<QSpinBox*>(QStringLiteral("colorRangeFuzziness"));
    auto* select = dialog->findChild<QComboBox*>(QStringLiteral("colorRangeSelect"));
    QVERIFY(localized && range && invert && fuzziness && select);
    localized->setChecked(true);
    range->setValue(30);
    invert->setChecked(true);
    fuzziness->setValue(55);

    const QString path = QDir::tempPath() + QStringLiteral("/pictura-color-range-test.json");
    QVERIFY(dialog->saveSettings(path));
    const QString savedSamples = dialog->samplesString();
    const QImage savedPreview = dialog->previewForTest();

    select->setCurrentIndex(select->findData(2));
    fuzziness->setValue(10);
    localized->setChecked(false);
    range->setValue(100);
    invert->setChecked(false);

    QVERIFY(dialog->loadSettings(path));
    QCOMPARE(dialog->samplesString(), savedSamples);
    QCOMPARE(dialog->fuzziness(), 55);
    QVERIFY(dialog->localized());
    QCOMPARE(dialog->localizedRange(), 30);
    QVERIFY(dialog->inverted());
    QCOMPARE(dialog->range(), 0);
    QCOMPARE(dialog->previewForTest(), savedPreview);
    QFile::remove(path);
}

void ColorRangeTest::saveLoadRejectsInvalid()
{
    QVERIFY(setupRedSquare());
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    const QString before = dialog->samplesString();
    const QString path = QDir::tempPath() + QStringLiteral("/pictura-color-range-invalid.json");
    {
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("{ not json");
    }
    QVERIFY(!dialog->loadSettings(path));
    QVERIFY(!dialog->settingsError().isEmpty());
    QCOMPARE(dialog->samplesString(), before);
    {
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("{\"format\":\"pictura-color-range\",\"version\":99}");
    }
    QVERIFY(!dialog->loadSettings(path));
    QVERIFY(!dialog->settingsError().isEmpty());
    QCOMPARE(dialog->samplesString(), before);
    QFile::remove(path);
}

void ColorRangeTest::previewReflectsTheLiveSelection()
{
    QVERIFY(setupRedSquare());
    QVERIFY(view_->select_rect(0, 0, 6, 40, QStringLiteral("new"), 0.0));
    pictura::ColorRangeDialog* dialog = open();
    QVERIFY(dialog);
    dialog->sampleAt(QPointF(5, 5));
    QImage preview = dialog->previewForTest();
    QCOMPARE(qGray(preview.pixel(4, 4)), 255);
    QCOMPARE(qGray(preview.pixel(8, 8)), 0);
    QCOMPARE(qGray(preview.pixel(30, 30)), 0);

    dialog->accept();
    QCoreApplication::processEvents();
    QCOMPARE(view_->selection_coverage(4, 4), 255);
    QCOMPARE(view_->selection_coverage(8, 8), 0);
}

QTEST_MAIN(ColorRangeTest)
#include "tst_color_range.moc"
