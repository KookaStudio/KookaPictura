// Image > Calculations (#610): the dialog's channel lists, defaults, mode
// rows, preview, cancel, and the three result targets.

#include <QtTest/QtTest>

#include "calculations_dialog.h"
#include "commands.h"
#include "frame.h"

#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/selection_bridge.h"

#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>

#include "qt_test_support.h"

using pictura::CalculationsDialog;

class CalculationsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void menuRowIsLive();
    void sourcesMatchDimensions();
    void channelListHasGrayAndNoComposite();
    void scaleOffsetFollowMode();
    void defaults();
    void modeList();
    void disabledModes();
    void resultNewDocument();
    void resultNewChannel();
    void resultNewChannelUndo();
    void resultNewChannelAtCap();
    void resultSelection();
    void resultSelectionReplaces();
    void resultsAgree();
    void previewFollowsControls();
    void previewOff();
    void cancelChangesNothing();
    void previewMatchesCommit();

private:
    pictura::PictureView* open(const QString& name, QSize size = {20, 20});
    QList<CalculationsDialog::Source> documents() const;
    static void pick(QDialog& dialog, const char* combo, const QString& text);
    static void fill(pictura::PictureView& view, int gray);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CalculationsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CalculationsTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

pictura::PictureView* CalculationsTest::open(const QString& name, QSize size)
{
    if (!window_->newDocument(name, size.width(), size.height(), QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    QCoreApplication::processEvents();
    return window_->activeView();
}

QList<CalculationsDialog::Source> CalculationsTest::documents() const
{
    QList<CalculationsDialog::Source> out;
    for (int i = 0; i < window_->documentCount(); ++i) {
        out.append({window_->documentName(i), window_->viewAt(i)});
    }
    return out;
}

void CalculationsTest::pick(QDialog& dialog, const char* combo, const QString& text)
{
    auto* box = dialog.findChild<QComboBox*>(QString::fromUtf8(combo));
    QVERIFY(box);
    const int index = box->findText(text);
    QVERIFY2(index >= 0, qPrintable(text));
    box->setCurrentIndex(index);
}

void CalculationsTest::fill(pictura::PictureView& view, int gray)
{
    QVERIFY(pictura::edit_fill(view, qRgba(gray, gray, gray, 255), -1,
                               QStringLiteral("normal"), 100, false));
}

void CalculationsTest::menuRowIsLive()
{
    QAction* action = window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Calculations…")}));
    QVERIFY(action);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(open(QStringLiteral("menu")));
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
}

void CalculationsTest::sourcesMatchDimensions()
{
    QVERIFY(open(QStringLiteral("small")));
    QVERIFY(open(QStringLiteral("big"), {30, 30}));
    pictura::PictureView* active = open(QStringLiteral("active"));
    QVERIFY(active);
    CalculationsDialog dialog(active, documents(), window_.get());
    auto* source = dialog.findChild<QComboBox*>(QStringLiteral("calculationsSource1Source"));
    auto* mask = dialog.findChild<QComboBox*>(QStringLiteral("calculationsMaskSource"));
    QVERIFY(source && mask);
    QCOMPARE(source->count(), 2);
    QCOMPARE(source->itemText(0), window_->documentName(window_->activeDocumentIndex()));
    QCOMPARE(mask->count(), 2);
}

void CalculationsTest::channelListHasGrayAndNoComposite()
{
    pictura::PictureView* rgb = open(QStringLiteral("rgb"));
    QVERIFY(rgb);
    const int layer = rgb->add_layer(0);
    QVERIFY(layer >= 0);
    const QString path = QString::number(layer);
    rgb->set_active_layer(path);
    QVERIFY(pictura::layer_mask_add(*rgb, QStringLiteral("reveal-all")));
    QCOMPARE(pictura::channels_new_alpha(*rgb), 0);
    QVERIFY(rgb->select_rect(0, 0, 5, 5, QStringLiteral("new"), 0.0));
    QVERIFY(window_->newDocument(QStringLiteral("gray"), 20, 20, QStringLiteral("grayscale"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* gray = window_->activeView();
    QVERIFY(gray);

    CalculationsDialog dialog(rgb, documents(), window_.get());
    auto* source = dialog.findChild<QComboBox*>(QStringLiteral("calculationsSource1Source"));
    auto* layerBox = dialog.findChild<QComboBox*>(QStringLiteral("calculationsSource1Layer"));
    auto* channel = dialog.findChild<QComboBox*>(QStringLiteral("calculationsSource1Channel"));
    auto* mask = dialog.findChild<QComboBox*>(QStringLiteral("calculationsMaskChannel"));
    auto* useMask = dialog.findChild<QCheckBox*>(QStringLiteral("calculationsUseMask"));
    QVERIFY(source && layerBox && channel && mask && useMask);

    QCOMPARE(source->count(), 2);
    QVERIFY(channel->findText(QStringLiteral("Gray")) >= 0);
    QCOMPARE(channel->itemData(channel->findText(QStringLiteral("Gray"))).toInt(), 0);
    QVERIFY(channel->findText(QStringLiteral("RGB")) < 0);
    QVERIFY(channel->findText(QStringLiteral("Red")) >= 0);
    QVERIFY(channel->findText(QStringLiteral("Transparency")) >= 0);
    QVERIFY(channel->findText(QStringLiteral("Alpha 1")) >= 0);

    const int layerIndex = layerBox->findData(path);
    QVERIFY(layerIndex >= 0);
    layerBox->setCurrentIndex(layerIndex);
    QVERIFY(channel->findText(QStringLiteral("Layer Mask")) >= 0);

    source->setCurrentIndex(1);
    QVERIFY(channel->findText(QStringLiteral("Gray")) >= 0);
    QVERIFY(channel->findText(QStringLiteral("Red")) < 0);

    useMask->setChecked(true);
    auto* maskSource = dialog.findChild<QComboBox*>(QStringLiteral("calculationsMaskSource"));
    QVERIFY(maskSource);
    maskSource->setCurrentIndex(0);
    QVERIFY(mask->findText(QStringLiteral("Selection")) >= 0);
    rgb->deselect();
    maskSource->setCurrentIndex(1);
    maskSource->setCurrentIndex(0);
    QVERIFY(mask->findText(QStringLiteral("Selection")) < 0);
    dialog.reject();
}

void CalculationsTest::scaleOffsetFollowMode()
{
    pictura::PictureView* view = open(QStringLiteral("mode"));
    QVERIFY(view);
    CalculationsDialog dialog(view, documents(), window_.get());
    dialog.show();
    auto* scale = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("calculationsScale"));
    auto* offset = dialog.findChild<QSpinBox*>(QStringLiteral("calculationsOffset"));
    QVERIFY(scale && offset);
    QVERIFY(!scale->isVisible() && !offset->isVisible());
    for (const QString& mode : {QStringLiteral("Add"), QStringLiteral("Subtract")}) {
        pick(dialog, "calculationsBlending", mode);
        QVERIFY(scale->isVisible() && offset->isVisible());
    }
    pick(dialog, "calculationsBlending", QStringLiteral("Screen"));
    QVERIFY(!scale->isVisible() && !offset->isVisible());
    dialog.reject();
}

void CalculationsTest::defaults()
{
    pictura::PictureView* view = open(QStringLiteral("defaults"));
    QVERIFY(view);
    CalculationsDialog dialog(view, documents(), window_.get());
    dialog.show();
    auto* blending = dialog.findChild<QComboBox*>(QStringLiteral("calculationsBlending"));
    auto* opacity = dialog.findChild<QSpinBox*>(QStringLiteral("calculationsOpacity"));
    auto* result = dialog.findChild<QComboBox*>(QStringLiteral("calculationsResult"));
    auto* scale = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("calculationsScale"));
    auto* offset = dialog.findChild<QSpinBox*>(QStringLiteral("calculationsOffset"));
    QVERIFY(blending && opacity && result && scale && offset);
    QCOMPARE(blending->currentText(), QStringLiteral("Multiply"));
    QCOMPARE(opacity->value(), 100);
    QCOMPARE(result->currentText(), QStringLiteral("New Document"));
    QCOMPARE(scale->value(), 1.0);
    QCOMPARE(scale->minimum(), 1.0);
    QCOMPARE(scale->maximum(), 2.0);
    QCOMPARE(offset->value(), 0);
    QCOMPARE(offset->minimum(), -255);
    QCOMPARE(offset->maximum(), 255);
    QVERIFY(!scale->isVisible() && !offset->isVisible());
    dialog.reject();
}

void CalculationsTest::modeList()
{
    pictura::PictureView* view = open(QStringLiteral("list"));
    QVERIFY(view);
    CalculationsDialog dialog(view, documents(), window_.get());
    auto* blending = dialog.findChild<QComboBox*>(QStringLiteral("calculationsBlending"));
    QVERIFY(blending);
    QCOMPARE(blending->currentText(), QStringLiteral("Multiply"));
    QCOMPARE(blending->findText(QStringLiteral("Dissolve")), -1);
    int subtracts = 0;
    for (int i = 0; i < blending->count(); ++i) {
        subtracts += blending->itemText(i) == QLatin1String("Subtract");
    }
    QCOMPARE(subtracts, 1);
    QVERIFY(blending->findText(QStringLiteral("Add")) >= 0);
    dialog.reject();
}

void CalculationsTest::disabledModes()
{
    QAction* action = window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Calculations…")}));
    QVERIFY(action);
    pictura::PictureView* rgb = open(QStringLiteral("indexed"));
    QVERIFY(rgb);
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
    QVERIFY(pictura::convert_to_indexed(*rgb, 1, 256, 0, 0));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    window_->closeDocument(window_->activeDocumentIndex(), false);

    QVERIFY(window_->newDocument(QStringLiteral("bitmap"), 20, 20, QStringLiteral("grayscale"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* gray = window_->activeView();
    QVERIFY(gray);
    QVERIFY(pictura::convert_to_bitmap(*gray, 0));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    window_->closeDocument(window_->activeDocumentIndex(), false);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
}

void CalculationsTest::resultNewDocument()
{
    pictura::PictureView* view = open(QStringLiteral("document"), {8, 8});
    QVERIFY(view);
    fill(*view, 100);
    const int history = view->history_count();
    CalculationsDialog dialog(view, documents(), window_.get());
    QVERIFY(dialog.result() == CalculationsDialog::Result::NewDocument);
    pick(dialog, "calculationsBlending", QStringLiteral("Normal"));
    std::unique_ptr<pictura::PictureView> target(new pictura::PictureView(window_.get()));
    QVERIFY(dialog.apply(target.get()));
    QCOMPARE(target->document_depth_bits(), 8);
    QCOMPARE(qRed(target->composite_argb(0, 0)), 100);
    QCOMPARE(qRed(target->composite_argb(4, 4)), 100);
    QCOMPARE(view->history_count(), history);
}

void CalculationsTest::resultNewChannel()
{
    pictura::PictureView* view = open(QStringLiteral("channel"), {8, 8});
    QVERIFY(view);
    fill(*view, 100);
    CalculationsDialog dialog(view, documents(), window_.get());
    pick(dialog, "calculationsBlending", QStringLiteral("Normal"));
    pick(dialog, "calculationsResult", QStringLiteral("New Channel"));
    const int before = view->selection_channel_count();
    QVERIFY(dialog.apply(view));
    QCOMPARE(view->selection_channel_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Calculations"));
    int width = 0;
    int height = 0;
    const auto bytes = pictura::channels_alpha_thumbnail(*view, before, 64, width, height);
    QCOMPARE(width, 8);
    QCOMPARE(height, 8);
    QCOMPARE(int(bytes.size()), 64);
    QVERIFY(bytes[0] == 100 && bytes[63] == 100);
    QCOMPARE(pictura::channels_alpha_names(*view).last(), QStringLiteral("Alpha 1"));
}

void CalculationsTest::resultNewChannelUndo()
{
    pictura::PictureView* view = open(QStringLiteral("channel-undo"), {8, 8});
    QVERIFY(view);
    fill(*view, 120);
    CalculationsDialog dialog(view, documents(), window_.get());
    pick(dialog, "calculationsBlending", QStringLiteral("Normal"));
    pick(dialog, "calculationsResult", QStringLiteral("New Channel"));
    const int before = view->selection_channel_count();
    QVERIFY(dialog.apply(view));
    QCOMPARE(view->selection_channel_count(), before + 1);
    QVERIFY(view->undo());
    QCOMPARE(view->selection_channel_count(), before);
}

void CalculationsTest::resultNewChannelAtCap()
{
    pictura::PictureView* view = open(QStringLiteral("cap"), {8, 8});
    QVERIFY(view);
    for (int i = 0; i < 53; ++i) {
        QCOMPARE(pictura::channels_new_alpha(*view), i);
    }
    QVERIFY(!pictura::channels_can_add(*view));
    CalculationsDialog dialog(view, documents(), window_.get());
    pick(dialog, "calculationsResult", QStringLiteral("New Channel"));
    dialog.show();
    const int history = view->history_count();
    const int channels = view->selection_channel_count();
    dialog.accept();
    QVERIFY(dialog.isVisible());
    auto* warning = dialog.findChild<QLabel*>(QStringLiteral("calculationsWarning"));
    QVERIFY(warning);
    QVERIFY(!warning->isHidden());
    QVERIFY(warning->text().contains(QStringLiteral("56-channel")));
    QVERIFY(!dialog.apply(view));
    QCOMPARE(view->selection_channel_count(), channels);
    QCOMPARE(view->history_count(), history);
    dialog.reject();
}

void CalculationsTest::resultSelection()
{
    pictura::PictureView* view = open(QStringLiteral("selection"), {8, 8});
    QVERIFY(view);
    fill(*view, 100);
    CalculationsDialog dialog(view, documents(), window_.get());
    pick(dialog, "calculationsBlending", QStringLiteral("Normal"));
    pick(dialog, "calculationsResult", QStringLiteral("Selection"));
    QVERIFY(dialog.apply(view));
    QVERIFY(view->has_selection());
    QCOMPARE(view->selection_coverage(0, 0), 100);
    QCOMPARE(view->selection_coverage(7, 7), 100);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Calculations"));
    QVERIFY(view->undo());
    QVERIFY(!view->has_selection());
}

void CalculationsTest::resultSelectionReplaces()
{
    pictura::PictureView* view = open(QStringLiteral("replace"), {8, 8});
    QVERIFY(view);
    fill(*view, 100);
    QVERIFY(view->select_rect(0, 0, 4, 4, QStringLiteral("new"), 0.0));
    CalculationsDialog dialog(view, documents(), window_.get());
    pick(dialog, "calculationsBlending", QStringLiteral("Normal"));
    pick(dialog, "calculationsResult", QStringLiteral("Selection"));
    QVERIFY(dialog.apply(view));
    QCOMPARE(view->selection_coverage(1, 1), 100);
    QCOMPARE(view->selection_coverage(6, 6), 100);
    QVERIFY(view->undo());
    QCOMPARE(view->selection_coverage(1, 1), 255);
    QCOMPARE(view->selection_coverage(6, 6), 0);
}

void CalculationsTest::resultsAgree()
{
    pictura::PictureView* view = open(QStringLiteral("agree"), {8, 8});
    QVERIFY(view);
    fill(*view, 100);

    CalculationsDialog dialog(view, documents(), window_.get());
    pick(dialog, "calculationsResult", QStringLiteral("Selection"));
    QVERIFY(dialog.apply(view));
    const auto selection = pictura::selection_coverage_bytes(*view);
    QCOMPARE(int(selection.size()), 64);

    CalculationsDialog channelDialog(view, documents(), window_.get());
    pick(channelDialog, "calculationsResult", QStringLiteral("New Channel"));
    const int channel = view->selection_channel_count();
    QVERIFY(channelDialog.apply(view));
    int width = 0;
    int height = 0;
    const auto coverage = pictura::channels_alpha_thumbnail(*view, channel, 64, width, height);
    QCOMPARE(int(coverage.size()), int(selection.size()));

    CalculationsDialog documentDialog(view, documents(), window_.get());
    QVERIFY(documentDialog.result() == CalculationsDialog::Result::NewDocument);
    std::unique_ptr<pictura::PictureView> target(new pictura::PictureView(window_.get()));
    QVERIFY(documentDialog.apply(target.get()));
    for (int i = 0; i < 64; ++i) {
        QCOMPARE(int(coverage[i]), int(selection[i]));
        QCOMPARE(qRed(target->composite_argb(i % 8, i / 8)), int(selection[i]));
    }
}

void CalculationsTest::previewFollowsControls()
{
    pictura::PictureView* view = open(QStringLiteral("preview"), {8, 8});
    QVERIFY(view);
    QVERIFY(pictura::edit_fill(*view, qRgba(200, 100, 50, 255), -1, QStringLiteral("normal"), 100,
                               false));
    CalculationsDialog dialog(view, documents(), window_.get());
    auto* image = dialog.findChild<QLabel*>(QStringLiteral("calculationsPreviewImage"));
    auto* invert = dialog.findChild<QCheckBox*>(QStringLiteral("calculationsSource1Invert"));
    QVERIFY(image && invert);
    QVERIFY(!image->pixmap().isNull());
    const QImage before = image->pixmap().toImage();
    const int history = view->history_count();
    invert->setChecked(true);
    const QImage after = image->pixmap().toImage();
    QVERIFY(before != after);
    QCOMPARE(view->history_count(), history);
}

void CalculationsTest::previewOff()
{
    pictura::PictureView* view = open(QStringLiteral("preview-off"));
    QVERIFY(view);
    CalculationsDialog dialog(view, documents(), window_.get());
    auto* image = dialog.findChild<QLabel*>(QStringLiteral("calculationsPreviewImage"));
    auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("calculationsPreview"));
    QVERIFY(image && preview);
    QVERIFY(!image->pixmap().isNull());
    preview->setChecked(false);
    QVERIFY(image->pixmap().isNull());
}

void CalculationsTest::cancelChangesNothing()
{
    pictura::PictureView* view = open(QStringLiteral("cancel"), {8, 8});
    QVERIFY(view);
    fill(*view, 100);
    const int history = view->history_count();
    const int channels = view->selection_channel_count();
    const QRgb pixel = view->composite_argb(0, 0);
    CalculationsDialog dialog(view, documents(), window_.get());
    dialog.reject();
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->selection_channel_count(), channels);
    QCOMPARE(view->composite_argb(0, 0), pixel);
    QVERIFY(!view->has_selection());
}

void CalculationsTest::previewMatchesCommit()
{
    pictura::PictureView* view = open(QStringLiteral("match"), {8, 8});
    QVERIFY(view);
    QVERIFY(pictura::edit_fill(*view, qRgba(150, 80, 40, 255), -1, QStringLiteral("normal"), 100,
                               false));
    auto source1 = pictura::apply_image_read(*view, QString(), 0);
    auto source2 = pictura::apply_image_read(*view, QString(), 0);
    auto mask = pictura::apply_image_read(*view, QString(), -100);
    int width = 0;
    int height = 0;
    const auto preview = pictura::calculations_preview(
        *source1, false, *source2, false, QStringLiteral("mul "), 1.0, 0, 100, *mask, false, width,
        height);
    QCOMPARE(width, 8);
    QCOMPARE(height, 8);
    QCOMPARE(int(preview.size()), 64);

    CalculationsDialog dialog(view, documents(), window_.get());
    pick(dialog, "calculationsResult", QStringLiteral("Selection"));
    QVERIFY(dialog.apply(view));
    const auto committed = pictura::selection_coverage_bytes(*view);
    QCOMPARE(int(committed.size()), int(preview.size()));
    for (int i = 0; i < 64; ++i) {
        QCOMPARE(int(committed[i]), int(preview[i]));
    }
}

#include "tst_calculations.moc"

QTEST_MAIN(CalculationsTest)
