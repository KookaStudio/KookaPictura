// Image > Duplicate / Trim (#84), Image Size / Canvas Size: ported from
// photorust.

#include <QtTest/QtTest>

#include "canvas_size_dialog.h"
#include "memory_refusal.h"
#include "commands.h"
#include "frame.h"
#include "image_size_dialog.h"

#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/filters_bridge.h"

#include "qt_test_support.h"

#include <rust/cxx.h>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QToolButton>

class ImageOpsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void duplicateDocument();
    void trimDocument();
    void trimIsUndoable();
    void trimRefusesALiveSession();
    void imageSizeResamplesAndSetsResolution();
    void imageSizePhysicalUnitsUseInches();
    void imageSizeEstimatesTheStoredMode();
    void imageSizeRefusesAnOverBudgetTarget();
    void resizeImageRefusesAnOverBudgetTarget();
    void imageSizeShowsTheBudgetWarning();
    void imageSizeRefusesASideOverTheLimit();
    void imageSizeNamesFullScratchDisks();
    void canvasSizeGrowsAroundTheAnchor();
    void canvasSizeRefusalNamesMemory();
    void canvasSizeNamesFullScratchDisks();
    void canvasSizeRefusesASideOverTheLimit();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ImageOpsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ImageOpsTest::cleanup()
{
    pictura::setImageSizeDialogDriverForTest({});
    pictura::set_available_memory_for_test(-1);
    pictura::set_scratch_available_for_test(-1);
}

namespace {

constexpr qint64 kMiB = 1024 * 1024;

// Pin the Image Size gate's host (#1293): `available` bytes of memory and
// `scratch` bytes free on the scratch disks. `cleanup()` reads the host again.
void pinHost(qint64 available, qint64 scratch)
{
    pictura::set_available_memory_for_test(available);
    pictura::set_scratch_available_for_test(scratch);
}

} // namespace

void ImageOpsTest::duplicateDocument()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Dup"), 20, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* source = frame.activeView();
    QVERIFY(source);
    source->select_all();
    QVERIFY(pictura::edit_fill(*source, QColor(30, 60, 90).rgba(), -1, QStringLiteral("normal"),
                              100, false));
    source->deselect();
    const int doc = frame.activeDocumentIndex();

    const int before = frame.documentCount();
    auto* target = new pictura::PictureView(&frame);
    QVERIFY(pictura::duplicate_into(*source, *target, QStringLiteral("copy"), false));
    QCOMPARE(target->document_width(), 20);
    QCOMPARE(target->document_height(), 16);
    QCOMPARE(target->sample_argb(5, 5), QColor(30, 60, 90).rgba());
    QCOMPARE(target->history_count(), 1);
    QCOMPARE(target->history_label(0), QStringLiteral("Duplicate"));
    delete target;
    QCOMPARE(frame.documentCount(), before);

    QVERIFY(pictura::duplicate_name(*source).endsWith(QStringLiteral(" copy")));
    frame.closeDocument(doc, false);
}

void ImageOpsTest::trimDocument()
{
    pictura::PicturaMainWindow& frame = *window_;
    // A transparent 24×24 canvas; fill a 5×4 block at (3, 2), then trim.
    QVERIFY(frame.newDocument(QStringLiteral("Trim"), 24, 24, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    QVERIFY(view->select_rect(3, 2, 5, 4, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, QColor(200, 0, 0).rgba(), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();

    const int before = view->history_count();
    QVERIFY(pictura::trim_image(*view));
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Trim"));
    QCOMPARE(view->document_width(), 5);
    QCOMPARE(view->document_height(), 4);
    QCOMPARE(view->sample_argb(0, 0), QColor(200, 0, 0).rgba());
    QVERIFY(view->can_undo());

    // A second trim has nothing to remove.
    const bool dirty = view->is_dirty();
    const QStringList target = pictura::selected_layer_paths(*view);
    const int trimmed = view->history_count();
    QVERIFY(!pictura::trim_image(*view));
    QCOMPARE(view->history_count(), trimmed);
    QCOMPARE(view->is_dirty(), dirty);
    QCOMPARE(pictura::selected_layer_paths(*view), target);
    QVERIFY(!view->has_selection());

    // An empty composite has nothing to trim either.
    QVERIFY(frame.newDocument(QStringLiteral("Empty"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent")));
    pictura::PictureView* empty = frame.activeView();
    QVERIFY(empty);
    const int emptyHistory = empty->history_count();
    QVERIFY(!pictura::trim_image(*empty));
    QCOMPARE(empty->history_count(), emptyHistory);
    QVERIFY(!empty->is_dirty());

    // Without a layer target, Trim picks the top layer.
    QVERIFY(frame.newDocument(QStringLiteral("Target"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent")));
    pictura::PictureView* untargeted = frame.activeView();
    QVERIFY(untargeted);
    QVERIFY(untargeted->add_layer(0) >= 0);
    QVERIFY(untargeted->select_rect(1, 1, 2, 2, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*untargeted, QColor(200, 0, 0).rgba(), -1,
                               QStringLiteral("normal"), 100, false));
    untargeted->deselect();
    pictura::clear_layer_target(*untargeted);
    QVERIFY(pictura::trim_image(*untargeted));
    QCOMPARE(pictura::selected_layer_paths(*untargeted),
             QStringList{QString::number(untargeted->layer_count() - 1)});
    frame.closeDocument(frame.activeDocumentIndex(), false);
    frame.closeDocument(frame.activeDocumentIndex(), false);
    frame.closeDocument(doc, false);
}

void ImageOpsTest::trimIsUndoable()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("UndoTrim"), 24, 24, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    const int added = view->add_layer(0);
    QVERIFY(added >= 0);
    const QString top = QString::number(added);
    frame.selectLayerPath(top);
    QVERIFY(view->select_rect(3, 2, 5, 4, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, QColor(200, 0, 0).rgba(), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    QVERIFY(view->select_rect(4, 3, 3, 2, QStringLiteral("new"), 0.0));
    QVERIFY(view->history_add_snapshot(QStringLiteral("PreTrim")));
    pictura::set_layer_target(*view, QStringList{QStringLiteral("0"), top}, top);
    const QStringList target = pictura::selected_layer_paths(*view);
    QCOMPARE(int(target.size()), 2);
    const int history = view->history_count();
    const int snapshots = pictura::history_snapshot_count(*view);
    const int depth = view->history_index();
    const QColor pixel = QColor::fromRgba(view->sample_argb(4, 3));
    const ::rust::Vec<int32_t> bounds = view->selection_bounds_extents();

    QVERIFY(pictura::trim_image(*view));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Trim"));
    QCOMPARE(view->history_label(0), QStringLiteral("New"));
    QCOMPARE(view->history_label(depth), QStringLiteral("Selection"));
    QCOMPARE(pictura::history_snapshot_label(*view, 0), QStringLiteral("PreTrim"));
    QCOMPARE(pictura::history_snapshot_count(*view), snapshots);
    QCOMPARE(pictura::selected_layer_paths(*view), target);
    QVERIFY(view->can_undo());
    QVERIFY(!view->has_selection());

    QVERIFY(view->undo());
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_index(), depth);
    QCOMPARE(pictura::selected_layer_paths(*view), target);
    QVERIFY(view->has_selection());
    const ::rust::Vec<int32_t> restored = view->selection_bounds_extents();
    QCOMPARE(int(restored.size()), int(bounds.size()));
    for (size_t i = 0; i < bounds.size(); ++i) {
        QCOMPARE(restored[i], bounds[i]);
    }
    QCOMPARE(view->sample_argb(4, 3), pixel.rgba());
    QCOMPARE(pictura::history_snapshot_count(*view), snapshots);
    frame.closeDocument(doc, false);
}

// A live session owns the document, so Trim refuses without recording.
void ImageOpsTest::trimRefusesALiveSession()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("TrimLive"), 24, 24, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    QVERIFY(view->select_rect(3, 2, 5, 4, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, QColor(200, 0, 0).rgba(), -1, QStringLiteral("normal"), 100,
                               false));
    view->deselect();
    const int history = view->history_count();
    const bool dirty = view->is_dirty();

    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(8.0, 8.0, 1.0));
    QVERIFY(!pictura::trim_image(*view));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->is_dirty(), dirty);
    QVERIFY(view->is_painting());
    view->cancel_paint();
    frame.closeDocument(doc, false);
}

// Image Size: the chained Width keeps the proportions, the Fit To presets
// and the summary follow, and Resample off changes only the resolution.
void ImageOpsTest::imageSizeResamplesAndSetsResolution()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Size"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    QCOMPARE(pictura::document_ppi(*view), 72.0);

    {
        pictura::ImageSizeDialog dialog(view);
        auto* width = qobject_cast<QDoubleSpinBox*>(dialog.controlForTest(QStringLiteral("imageSizeWidth")));
        auto* fit = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("imageSizeFitTo")));
        QVERIFY(width && fit);
        width->setValue(20);
        QCOMPARE(dialog.resultHeight(), 10);
        QCOMPARE(fit->currentText(), QStringLiteral("Custom"));
        // A landscape image takes the 4 x 6 in preset on its side.
        fit->setCurrentIndex(fit->findText(QStringLiteral("4 x 6 in 300 dpi")));
        QCOMPARE(dialog.resultWidth(), 1800);
        QCOMPARE(dialog.resultHeight(), 1200);
        QCOMPARE(dialog.resultPpi(), 300.0);
        fit->setCurrentIndex(fit->findText(QStringLiteral("Original Size")));
        width->setValue(20);
        QCOMPARE(dialog.resampleKind(), QStringLiteral("bicubic"));
        QVERIFY(pictura::image_size_apply(*view, dialog.resampleKind(), dialog.resultWidth(),
                                          dialog.resultHeight(), dialog.resultPpi(),
                                          dialog.resultPerCm()));
    }
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 10);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Image Size"));

    {
        pictura::ImageSizeDialog dialog(view);
        auto* resample = qobject_cast<QCheckBox*>(dialog.controlForTest(QStringLiteral("imageSizeResample")));
        auto* resolution =
            qobject_cast<QDoubleSpinBox*>(dialog.controlForTest(QStringLiteral("imageSizeResolution")));
        QVERIFY(resample && resolution);
        resample->setChecked(false);
        resolution->setValue(300);
        QCOMPARE(dialog.resultWidth(), 20);
        QVERIFY(dialog.resampleKind().isEmpty());
        QVERIFY(pictura::image_size_apply(*view, dialog.resampleKind(), dialog.resultWidth(),
                                          dialog.resultHeight(), dialog.resultPpi(),
                                          dialog.resultPerCm()));
    }
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(pictura::document_ppi(*view), 300.0);
    QVERIFY(!pictura::image_size_apply(*view, QString(), 20, 10, 300.0, false));
    view->undo();
    QCOMPARE(pictura::document_ppi(*view), 72.0);
    frame.closeDocument(doc, false);
}

// Image Size: a printed width in cm/mm/points sets pixels per inch, and a
// resolution edit resamples whichever axis is physical, both when chained.
void ImageOpsTest::imageSizePhysicalUnitsUseInches()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Units"), 3000, 1500, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const auto find = [](pictura::ImageSizeDialog& dialog, const char* name) {
        return dialog.controlForTest(QLatin1String(name));
    };
    {
        pictura::ImageSizeDialog dialog(view);
        auto* resample = qobject_cast<QCheckBox*>(find(dialog, "imageSizeResample"));
        auto* width = qobject_cast<QDoubleSpinBox*>(find(dialog, "imageSizeWidth"));
        auto* unit = qobject_cast<QComboBox*>(find(dialog, "imageSizeWidthUnit"));
        QVERIFY(resample && width && unit);
        resample->setChecked(false);
        unit->setCurrentIndex(unit->findText(QStringLiteral("Centimeters")));
        width->setValue(10.0);
        QCOMPARE(qRound(dialog.resultPpi()), 762);
        unit->setCurrentIndex(unit->findText(QStringLiteral("Millimeters")));
        width->setValue(254.0);
        QCOMPARE(qRound(dialog.resultPpi()), 300);
        unit->setCurrentIndex(unit->findText(QStringLiteral("Points")));
        width->setValue(360.0);
        QCOMPARE(qRound(dialog.resultPpi()), 600);
    }
    {
        // Width in pixels, Height in inches: the Height drives the resample.
        pictura::ImageSizeDialog dialog(view);
        auto* height = qobject_cast<QComboBox*>(find(dialog, "imageSizeHeightUnit"));
        auto* chain = qobject_cast<QToolButton*>(find(dialog, "imageSizeConstrain"));
        auto* resolution = qobject_cast<QDoubleSpinBox*>(find(dialog, "imageSizeResolution"));
        QVERIFY(height && chain && resolution);
        height->setCurrentIndex(height->findText(QStringLiteral("Inches")));
        resolution->setValue(144.0);
        QCOMPARE(dialog.resultWidth(), 6000);
        QCOMPARE(dialog.resultHeight(), 3000);
        chain->setChecked(false);
        resolution->setValue(72.0);
        QCOMPARE(dialog.resultWidth(), 6000);
        QCOMPARE(dialog.resultHeight(), 1500);
    }
    frame.closeDocument(doc, false);
}

// Image Size's estimate counts the stored mode's planes and bits.
void ImageOpsTest::imageSizeEstimatesTheStoredMode()
{
    const auto bytes = [](const QString& mode, int bits) {
        pictura::PictureView view;
        if (!pictura::create_document(view, 8, 8, mode, bits, 0xffffffu, false, 72.0, false)) {
            return -1.0;
        }
        return pictura::bytesPerPixel(&view);
    };
    QCOMPARE(bytes(QStringLiteral("rgb"), 8), 3.0);
    QCOMPARE(bytes(QStringLiteral("cmyk"), 8), 4.0);
    QCOMPARE(bytes(QStringLiteral("cmyk"), 16), 8.0);
    QCOMPARE(bytes(QStringLiteral("grayscale"), 8), 1.0);
    QCOMPARE(bytes(QStringLiteral("bitmap"), 1), 0.125);
}

// Canvas Size: Relative grows the canvas around the anchor and the
// Background's new area takes the extension colour.
void ImageOpsTest::canvasSizeGrowsAroundTheAnchor()
{
    pictura::PicturaMainWindow& frame = *window_;
    // An opaque image opens as a Background; a new white document's Layer 0
    // would keep the new area transparent, as CS6 does for a plain layer.
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QImage image(20, 10, QImage::Format_RGB32);
    image.fill(Qt::white);
    const QString path = dir.filePath(QStringLiteral("canvas.png"));
    QVERIFY(image.save(path) && frame.openDocumentAtPath(path));
    QCoreApplication::processEvents();
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    pictura::CanvasSizeDialog dialog(view, Qt::black, Qt::white);
    auto* relative = qobject_cast<QCheckBox*>(dialog.controlForTest(QStringLiteral("canvasSizeRelative")));
    auto* width = qobject_cast<QDoubleSpinBox*>(dialog.controlForTest(QStringLiteral("canvasSizeWidth")));
    auto* unit = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("canvasSizeWidthUnit")));
    auto* extension = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("canvasSizeExtension")));
    QVERIFY(relative && width && unit && extension);
    QCOMPARE(dialog.resultWidth(), 20);
    unit->setCurrentIndex(unit->findText(QStringLiteral("Percent")));
    QCOMPARE(width->value(), 100.0);
    unit->setCurrentIndex(unit->findText(QStringLiteral("Pixels")));
    relative->setChecked(true);
    width->setValue(10);
    QCOMPARE(dialog.resultWidth(), 30);
    QCOMPARE(dialog.resultHeight(), 10);
    QCOMPARE(dialog.anchorName(), QStringLiteral("center"));
    extension->setCurrentIndex(extension->findText(QStringLiteral("Foreground")));
    QCOMPARE(dialog.extensionColor(), QColor(Qt::black));

    QVERIFY(pictura::canvas_size_apply(*view, dialog.anchorName(), dialog.resultWidth(),
                                       dialog.resultHeight(), dialog.extensionColor().rgb() & 0xffffffu));
    QCOMPARE(view->document_width(), 30);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Canvas Size"));
    QCOMPARE(QColor::fromRgba(view->composite_argb(2, 5)), QColor(Qt::black));
    QCOMPARE(QColor::fromRgba(view->composite_argb(15, 5)), QColor(Qt::white));
    QCOMPARE(QColor::fromRgba(view->composite_argb(27, 5)), QColor(Qt::black));
    frame.closeDocument(doc, false);
}

// Image Size refuses a target that does not fit the document budget, and a
// resolution-only change is still allowed (#1156).
namespace {

// Open a `width` × `height` opaque white PNG from `dir`, so it has a
// Background as an opened image does; the view, or null.
pictura::PictureView* openOpaque(pictura::PicturaMainWindow& frame, const QTemporaryDir& dir,
                                 int width, int height)
{
    QImage image(width, height, QImage::Format_RGB32);
    image.fill(Qt::white);
    const QString path = dir.filePath(QStringLiteral("opaque.png"));
    if (!image.save(path) || !frame.openDocumentAtPath(path)) {
        return nullptr;
    }
    QCoreApplication::processEvents();
    return frame.activeView();
}

} // namespace

// [ocg_canvas_refused] Canvas Size spills to the scratch disks (#1294), so a
// 30000² target is refused only on a host with 4 MiB of room, under one
// 30000-wide strip, and no scratch space.
void ImageOpsTest::canvasSizeRefusalNamesMemory()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Canvas"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int history = view->history_count();
    pinHost(260 * kMiB, 0);

    QTimer answer;
    answer.setInterval(20);
    connect(&answer, &QTimer::timeout, this, [&] {
        auto* dialog = frame.findChild<pictura::CanvasSizeDialog*>();
        if (!dialog || !dialog->isVisible()) {
            return;
        }
        auto* relative = qobject_cast<QCheckBox*>(
            dialog->controlForTest(QStringLiteral("canvasSizeRelative")));
        auto* width = qobject_cast<QDoubleSpinBox*>(
            dialog->controlForTest(QStringLiteral("canvasSizeWidth")));
        auto* height = qobject_cast<QDoubleSpinBox*>(
            dialog->controlForTest(QStringLiteral("canvasSizeHeight")));
        if (relative && width && height) {
            relative->setChecked(false);
            width->setValue(30000);
            height->setValue(30000);
        }
        dialog->accept();
    });
    answer.start();
    QVERIFY(frame.registry()->dispatch(QStringLiteral("image.canvas.size")));
    answer.stop();

    QCOMPARE(view->document_width(), 40);
    QCOMPARE(view->document_height(), 20);
    QCOMPARE(view->history_count(), history);
    const QString message = frame.statusBar()->currentMessage();
    QVERIFY2(message.contains(QStringLiteral("resize the canvas")), qPrintable(message));
    QVERIFY2(message.contains(QStringLiteral("memory"), Qt::CaseInsensitive), qPrintable(message));
    frame.closeDocument(doc, false);
}

// [ocg_scratch_message] The working set fits 1.75 GiB of room; the extended
// Background's 30000² planes do not, with no scratch space.
void ImageOpsTest::canvasSizeNamesFullScratchDisks()
{
    pictura::PicturaMainWindow& frame = *window_;
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    pictura::PictureView* view = openOpaque(frame, dir, 40, 20);
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    pinHost(2048 * kMiB, 0);

    QVERIFY(!pictura::canvas_size_fits_budget(*view, QStringLiteral("center"), 30000, 30000));
    QVERIFY(pictura::edit_refused_for_scratch_full());
    const QString message = pictura::commitRefusalMessage(QStringLiteral("resize the canvas"));
    QVERIFY(message.contains(QStringLiteral("resize the canvas")));
    QVERIFY(message.contains(QStringLiteral("scratch disks"), Qt::CaseInsensitive));
    frame.closeDocument(doc, false);
}

// [rs_canvas_side_limit]
void ImageOpsTest::canvasSizeRefusesASideOverTheLimit()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("CanvasSide"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    pinHost(64 * 1024 * kMiB, 64 * 1024 * kMiB);

    const QString center = QStringLiteral("center");
    QVERIFY(pictura::canvas_size_fits_budget(*view, center, 30000, 10));
    QVERIFY(!pictura::canvas_size_fits_budget(*view, center, 30001, 10));
    QVERIFY(!pictura::canvas_size_fits_budget(*view, center, 10, 30001));
    frame.closeDocument(doc, false);
}

void ImageOpsTest::imageSizeRefusesAnOverBudgetTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Budget"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    // 4 MiB of room, under one 30000-wide output strip: a memory refusal.
    pinHost(260 * kMiB, 0);
    const int doc = frame.activeDocumentIndex();
    const int history = view->history_count();

    QVERIFY(!pictura::imageSizeFitsBudget(*view, 30000, 30000));
    const QString message = pictura::imageSizeBudgetMessage(30000, 30000);
    QVERIFY(message.contains(QStringLiteral("30000")));
    QVERIFY(message.contains(QStringLiteral("memory"), Qt::CaseInsensitive));

    QVERIFY(!pictura::image_size_apply(*view, QStringLiteral("bicubic"), 30000, 30000, 72.0,
                                       false));
    QCOMPARE(view->document_width(), 40);
    QCOMPARE(view->document_height(), 20);
    QCOMPARE(view->history_count(), history);

    QVERIFY(pictura::image_size_apply(*view, QString(), 40, 20, 300.0, false));
    QCOMPARE(view->document_width(), 40);
    QCOMPARE(view->document_height(), 20);
    QCOMPARE(pictura::document_ppi(*view), 300.0);
    frame.closeDocument(doc, false);
}

// The resize bridge applies the same gate and still allows a size inside it.
void ImageOpsTest::resizeImageRefusesAnOverBudgetTarget()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Bridge"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    // 4 MiB of room, under one 30000-wide output strip: a memory refusal.
    pinHost(260 * kMiB, 0);
    const int doc = frame.activeDocumentIndex();
    const int history = view->history_count();

    QVERIFY(!view->resize_image(QStringLiteral("bicubic"), 30000, 30000));
    QCOMPARE(view->document_width(), 40);
    QCOMPARE(view->document_height(), 20);
    QCOMPARE(view->history_count(), history);

    pinHost(-1, -1);
    QVERIFY(view->resize_image(QStringLiteral("nearest"), 20, 10));
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 10);
    frame.closeDocument(doc, false);
}

// Accepting an over-budget Image Size shows the warning and leaves the
// document alone.
void ImageOpsTest::imageSizeShowsTheBudgetWarning()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Warning"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    // 4 MiB of room, under one 30000-wide output strip: a memory refusal.
    pinHost(260 * kMiB, 0);
    const int doc = frame.activeDocumentIndex();
    const int history = view->history_count();

    int warnings = 0;
    QString text;
    pictura::setImageSizeDialogDriverForTest([&](QDialog& dialog) {
        if (auto* box = qobject_cast<QMessageBox*>(&dialog)) {
            ++warnings;
            text = box->text();
            return true;
        }
        auto* size = qobject_cast<pictura::ImageSizeDialog*>(&dialog);
        if (!size) {
            return false;
        }
        auto* chain = qobject_cast<QToolButton*>(
            size->controlForTest(QStringLiteral("imageSizeConstrain")));
        auto* width = qobject_cast<QDoubleSpinBox*>(
            size->controlForTest(QStringLiteral("imageSizeWidth")));
        auto* height = qobject_cast<QDoubleSpinBox*>(
            size->controlForTest(QStringLiteral("imageSizeHeight")));
        if (!chain || !width || !height) {
            return false;
        }
        chain->setChecked(false);
        width->setValue(30000);
        height->setValue(30000);
        return true;
    });
    QVERIFY(frame.registry()->dispatch(QStringLiteral("image.image.size")));
    pictura::setImageSizeDialogDriverForTest({});

    QCOMPARE(warnings, 1);
    QVERIFY(text.contains(QStringLiteral("30000")));
    QVERIFY(text.contains(QStringLiteral("memory"), Qt::CaseInsensitive));
    QCOMPARE(view->document_width(), 40);
    QCOMPARE(view->document_height(), 20);
    QCOMPARE(view->history_count(), history);
    frame.closeDocument(doc, false);
}

// A side over the image budget is refused however much memory there is.
void ImageOpsTest::imageSizeRefusesASideOverTheLimit()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Side"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    pinHost(64 * 1024 * kMiB, 64 * 1024 * kMiB);

    QVERIFY(pictura::imageSizeFitsBudget(*view, 30000, 10));
    QVERIFY(!pictura::imageSizeFitsBudget(*view, 30001, 10));
    QVERIFY(!pictura::imageSizeFitsBudget(*view, 10, 30001));
    frame.closeDocument(doc, false);
}

// A resize whose working set fits but whose planes do not fit the room plus
// the scratch disks names the scratch disks.
void ImageOpsTest::imageSizeNamesFullScratchDisks()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Scratch"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int history = view->history_count();
    // 1.5 GiB of room: the strips and windows fit, the 30000² planes do not.
    pinHost(1792 * kMiB, 0);

    QVERIFY(!pictura::imageSizeFitsBudget(*view, 30000, 30000));
    QVERIFY(pictura::edit_refused_for_scratch_full());
    const QString message = pictura::imageSizeBudgetMessage(30000, 30000);
    QVERIFY(message.contains(QStringLiteral("30000")));
    QVERIFY(message.contains(QStringLiteral("scratch disks"), Qt::CaseInsensitive));
    QCOMPARE(view->history_count(), history);
    frame.closeDocument(doc, false);
}

QTEST_MAIN(ImageOpsTest)
#include "tst_image_ops.moc"
