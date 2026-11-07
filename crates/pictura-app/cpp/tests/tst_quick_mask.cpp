#include "api/document_api.h"
#include "api/filters_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/selection_api.h"
#include "api/selection_bridge.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "panels/channels_panel.h"
#include "panels/color_panel.h"
#include "panels/color_swatch_widget.h"
#include "panels/layers_panel.h"
#include "qt_test_support.h"
#include "theme.h"
#include "toolbox.h"
#include "tools.h"
#include <QtCore/QItemSelectionModel>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QPainter>
#include <QtTest/QtTest>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>
#include <array>
using namespace pictura;
class QuickMaskTest : public QObject {
    Q_OBJECT
private slots:
    void init();
    void cleanup();
    void entryWithoutPixelSelection();
    void entryWithPixelSelection();
    void entryWithEmptySelection();
    void exitCreatesPaintedSelection();
    void softCoverageAndContour();
    void brushPencilEraserEditCoverage();
    void partialPaintSurvivesExit();
    void paintOutsideOriginalSelection();
    void fillsAndFilterEditCoverage();
    void blendModesRespectPolarity();
    void unsupportedToolsRefuse();
    void quickMaskOverridesLayerMaskTarget();
    void noLayerBlocksCursorAndPaint();
    void selectingLayerRestoresPainting();
    void optionsChangeOnlyDisplay();
    void optionsCancelPreservesSettings();
    void polarityControlsAgree();
    void polarityWaitsForStroke();
    void grayscaleAndSwatchesFollowMode();
    void optionsPersistWithoutMode();
    void historyAcrossModeBoundary();
    void cancelRestoresCoverage();
    void documentsKeepIndependentModes();
    void presentationTracksTransform();
    void coverageIgnoresDocumentDepthAndMode();
    void saveOmitsTemporaryMask();
    void savedAlphaRoundTrips();
    void toolbarAndShortcutShareState();
    void toolbarTracksActiveDocument();
    void optionsGesturesDoNotToggleMode();
    void textInputOwnsShortcut();
    void temporaryChannelFollowsCoverage();
    void temporaryChannelRejectsDurableActions();
    void channelNavigationCommitsSelection();
    void activeLayerHighlightFollowsMode();
    void redHighlightFollowsActiveTarget();
    void contentMovesRefuseWithoutPreview();
    void entryCancelsContentMovePreview();
    void targetSwatchesSurviveQuickMaskAndDocumentSwitch();

private:
    bool enter() { return quick_mask_set(*view_, true); }
    int coverage(int x, int y) { return quick_mask_active(*view_) ? quick_mask_coverage(*view_,x,y) : view_->selection_coverage(x,y); }
    QImage crop() { return view_->canvas_display_image(0, 0, 0, 32, 32); }
    QToolButton* button()
    {
        return window_->findChild<QToolButton*>(QStringLiteral("paintMaskButton"));
    }
    ChannelsPanel* channels() { return window_->findChild<ChannelsPanel*>(); }
    bool begin(quint32 color, bool pencil = false, int opacity = 100,
        const QString& mode = QStringLiteral("normal"))
    {
        const PaintTip tip { 8, 100, 100, 0, 25, 0, 1, 0, 0, 0 };
        return begin_brush(*view_, color, 0xffffffffu, tip, opacity, 100, mode,
            pencil, false, false, false);
    }
    bool paint(QColor color, int x = 16, int y = 16, bool pencil = false,
        int opacity = 100,
        const QString& mode = QStringLiteral("normal"))
    {
        return begin(color.rgba(), pencil, opacity, mode) && view_->paint_dab(x, y, 1) && view_->end_paint();
    }
    QByteArray maskBytes()
    {
        int w = 0, h = 0;
        const auto bytes = channels_active_mask_thumbnail(*view_, 32, w, h);
        return QByteArray(reinterpret_cast<const char*>(bytes.data()), bytes.size());
    }
    QColor layerRowSurface(LayersPanel* panel, const QString& path)
    {
        const QImage image = panel->rowImageForTest(path);
        // The trailing padding isolates the highlight from thumbnails and icons.
        return image.isNull() ? QColor() : image.pixelColor(image.width() - 2, 2);
    }
    test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
    PictureView* view_ = nullptr;
};
void QuickMaskTest::init()
{
    QVERIFY(stateHome_.isValid());
    window_ = test::makeMainWindow();
    QVERIFY(window_);
    QVERIFY(window_->newDocument(QStringLiteral("Quick Mask"), 32, 32,
        QStringLiteral("rgb"), 8,
        QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    const QString target = view_->add_layer_in(QString());
    QVERIFY(!target.isEmpty());
    window_->selectLayerPath(target);
    view_->set_gpu_compute(false);
    QVERIFY(quick_mask_set_options(*view_, false, 0xffff0000u, 50));
    window_->show();
    QCoreApplication::processEvents();
}
void QuickMaskTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0)
        window_->closeDocument(window_->activeDocumentIndex(), false);
    window_.reset();
    view_ = nullptr;
}
void QuickMaskTest::entryWithoutPixelSelection()
{
    QVERIFY(enter());
    QCOMPARE(crop().pixelColor(16, 16), QColor(Qt::white));
    QVERIFY(paint(Qt::black));
    QCOMPARE(coverage(16, 16), 0);
    QCOMPARE(crop().pixelColor(16, 16), QColor(255, 128, 128));
}

void QuickMaskTest::entryWithPixelSelection()
{
    QVERIFY(view_->select_rect(8, 8, 8, 8, QStringLiteral("new"), 0));
    QVERIFY(enter());
    QCOMPARE(crop().pixelColor(10, 10), QColor(Qt::white));
    QCOMPARE(crop().pixelColor(2, 2), QColor(255, 128, 128));
    QCOMPARE(coverage(10, 10), 255);
}

void QuickMaskTest::entryWithEmptySelection()
{
    const int alpha = channels_new_alpha(*view_);
    QVERIFY(alpha >= 0);
    QVERIFY(view_->load_selection(QStringLiteral("Alpha %1").arg(alpha + 1)));
    QVERIFY(enter());
    QCOMPARE(crop().pixelColor(16, 16), QColor(255, 128, 128));
}

void QuickMaskTest::exitCreatesPaintedSelection()
{
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    QVERIFY(paint(Qt::black, 8, 8));
    QVERIFY(paint(Qt::white, 8, 8));
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(coverage(16, 16), 0);
    QCOMPARE(coverage(8, 8), 255);
    QVERIFY(!quick_mask_active(*view_));
}

void QuickMaskTest::softCoverageAndContour()
{
    for (int value : { 0, 64, 127, 128, 192, 255 }) {
        QVERIFY(quick_mask_set(*view_, false));
        view_->deselect();
        QVERIFY(enter());
        // First exclude the whole canvas, then paint one soft island.
        QVERIFY(bucket_fill_at(*view_, 0, 0, 0xff000000u, -1,
            QStringLiteral("normal"), 100, 0, false, true,
            false));
        if (value > 0)
            QVERIFY(paint(QColor(value, value, value)));
        QVERIFY(quick_mask_set(*view_, false));
        QCOMPARE(coverage(16, 16), value);
        QCOMPARE(selection_api::contour(*view_).isEmpty(), value < 128);
        QVERIFY(enter());
        QCOMPARE(coverage(16, 16), value);
        QVERIFY(selection_api::contour(*view_).isEmpty());
    }
}

void QuickMaskTest::brushPencilEraserEditCoverage()
{
    const QImage original = view_->image();
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    QVERIFY(paint(Qt::black, 8, 8, true));
    const PaintTip tip { 8, 100, 100, 0, 25, 0, 1, 0, 0, 0 };
    QVERIFY(begin_eraser(*view_, 0xffffffffu, tip, 0, 100, 100, false, 1));
    QVERIFY(view_->paint_dab(16, 16, 1));
    QVERIFY(view_->end_paint());
    QCOMPARE(coverage(16, 16), 255);
    QCOMPARE(coverage(8, 8), 0);
    auto* colors = window_->findChild<ColorState*>();
    colors->setForeground(Qt::black);
    colors->setBackground(Qt::white);
    for (auto id : { ToolId::Brush, ToolId::Eraser, ToolId::Pencil }) {
        window_->setActiveTool(id);
        auto* canvas = window_->imageView();
        canvas->mousePressed(QPointF(24, 24), Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseReleased(QPointF(24, 24));
        QCOMPARE(coverage(24, 24), id == ToolId::Eraser ? 255 : 0);
    }
    QCOMPARE(view_->image(), original);
}

void QuickMaskTest::partialPaintSurvivesExit()
{
    QVERIFY(enter());
    QVERIFY(paint(Qt::black, 16, 16, false, 50));
    const int value = coverage(16, 16);
    QVERIFY(value > 0 && value < 255);
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(coverage(16, 16), value);
}

void QuickMaskTest::paintOutsideOriginalSelection()
{
    QVERIFY(view_->select_rect(0, 0, 4, 4, QStringLiteral("new"), 0));
    QVERIFY(enter());
    QVERIFY(paint(Qt::white));
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(coverage(16, 16), 255);
}

void QuickMaskTest::fillsAndFilterEditCoverage()
{
    const QImage original = view_->image();
    QVERIFY(enter());
    QVERIFY(bucket_fill_at(*view_, 16, 16, 0xff000000u, -1,
        QStringLiteral("normal"), 100, 0, false, true, false));
    QCOMPARE(coverage(16, 16), 0);
    QVERIFY(draw_gradient(*view_, 0, 0xff000000u, 0xffffffffu, 0,
        QStringLiteral("normal"), 100, false, false, true, 0, 0,
        32, 0));
    const int old = coverage(16, 16);
    QVERIFY(view_->apply_filter(QStringLiteral("gaussian-blur")));
    QVERIFY(view_->undo());
    QCOMPARE(coverage(16, 16), old);
    QCOMPARE(view_->image(), original);
}

void QuickMaskTest::blendModesRespectPolarity()
{
    for (bool selected : { false, true }) {
        for (const auto& mode :
            { QStringLiteral("normal"), QStringLiteral("multiply") }) {
            QVERIFY(quick_mask_set(*view_, false));
            view_->deselect();
            QVERIFY(enter());
            QVERIFY(paint(QColor(128, 128, 128)));
            QVERIFY(quick_mask_set_options(*view_, selected, 0xffff0000u, 50));
            QVERIFY(paint(Qt::black, 16, 16, false, 100, mode));
            QCOMPARE(coverage(16, 16), selected ? 255 : 0);
            QVERIFY(paint(QColor(128, 128, 128), 16, 16, false, 100,
                QStringLiteral("normal")));
            QVERIFY(bucket_fill_at(*view_, 16, 16, 0xff000000u, -1, mode, 100, 0,
                false, true, false));
            QCOMPARE(coverage(16, 16), selected ? 255 : 0);
        }
    }
}

void QuickMaskTest::unsupportedToolsRefuse()
{
    QVERIFY(paint(Qt::black));
    QTemporaryDir dir;
    QVERIFY(document_api::save(*view_, dir.filePath(QStringLiteral("unsupported.psd"))));
    QVERIFY(enter());
    const QImage image = view_->image();
    const QImage overlay = crop();
    const int history = view_->history_count();
    QVERIFY(!view_->is_dirty());
    const PaintTip tip { 8, 100, 100, 0, 25, 0, 1, 0, 0, 0 };
    QVERIFY(!healing_begin(*view_, 8, 100));
    QVERIFY(!begin_pattern_stamp(*view_, tip, 100, 100, QStringLiteral("normal"),
        0, 0, 0));
    QVERIFY(!begin_eraser(*view_, 0xffffffffu, tip, 0, 100, 100, true, 1));
    QVERIFY(!vp_begin(*view_));
    const std::array<double, 8> plane { 4, 4, 28, 4, 28, 28, 4, 28 };
    QCOMPARE(vp_create(::rust::Slice<const double>(plane.data(), plane.size())), -1);
    QCOMPARE(vp_finish(*view_, true, true), 0);
    QCOMPARE(view_->history_count(), history);
    QCOMPARE(view_->image(), image);
    QCOMPARE(crop(), overlay);
    QVERIFY(!view_->is_dirty());
    QVERIFY(quick_mask_set(*view_, false));
    QVERIFY(vp_begin(*view_));
    QCOMPARE(vp_plane_count(), 0);
    QCOMPARE(vp_finish(*view_, false, false), 0);
}

void QuickMaskTest::quickMaskOverridesLayerMaskTarget()
{
    QVERIFY(layer_mask_add(*view_, QStringLiteral("reveal-all")));
    const QString target = mask_edit_target_get(*view_);
    const QImage original = view_->image();
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    QCOMPARE(coverage(16, 16), 0);
    QCOMPARE(view_->image(), original);
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(mask_edit_target_get(*view_), target);
}

void QuickMaskTest::noLayerBlocksCursorAndPaint()
{
    QVERIFY(enter());
    set_layer_target(*view_, QStringList(), QString());
    auto* tools = window_->findChild<ToolController*>();
    QVERIFY(tools);
    for (auto id : { ToolId::Brush, ToolId::Pencil, ToolId::Eraser,
             ToolId::PaintBucket, ToolId::Gradient }) {
        window_->setActiveTool(id);
        tools->refreshCursor();
        QCOMPARE(window_->imageView()->cursor().shape(), Qt::ForbiddenCursor);
    }
    const int history = view_->history_count();
    QVERIFY(!paint(Qt::black));
    QCOMPARE(view_->history_count(), history);
    QVERIFY(!view_->has_selection());
}

void QuickMaskTest::selectingLayerRestoresPainting()
{
    const QString path = view_->active_layer_path();
    QVERIFY(enter());
    set_layer_target(*view_, QStringList(), QString());
    QVERIFY(!paint(Qt::black));
    window_->selectLayerPath(path);
    window_->setActiveTool(ToolId::Brush);
    window_->findChild<ToolController*>()->refreshCursor();
    QVERIFY(window_->imageView()->cursor().shape() != Qt::ForbiddenCursor);
    QVERIFY(paint(Qt::black));
}

void QuickMaskTest::optionsChangeOnlyDisplay()
{
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    const int history = view_->history_count();
    for (int opacity : { 0, 50, 100 }) {
        QTimer::singleShot(0, [opacity] {
            auto* dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
            QVERIFY(dialog);
            auto* spin = dialog->findChild<QSpinBox*>(QStringLiteral("quickMaskOpacity"));
            QVERIFY(spin);
            spin->setValue(opacity);
            auto* swatch = dialog->findChild<ColorSwatchWidget*>(
                QStringLiteral("quickMaskColor"));
            QVERIFY(swatch);
            swatch->setColor(Qt::blue);
            dialog->accept();
        });
        QTest::mouseDClick(button(), Qt::LeftButton);
        QCOMPARE(quick_mask_opacity(*view_), opacity);
        QCOMPARE(coverage(16, 16), 0);
        const QColor color = crop().pixelColor(16, 16);
        QCOMPARE(color.blue(), 255);
        QCOMPARE(color.red(), qRound(255 * (1 - opacity / 100.0)));
    }
    QCOMPARE(view_->history_count(), history);
}

void QuickMaskTest::optionsCancelPreservesSettings()
{
    QVERIFY(enter());
    const int opacity = quick_mask_opacity(*view_);
    QTimer::singleShot(0, [] {
        auto* d = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        QVERIFY(d);
        d->findChild<QSpinBox*>(QStringLiteral("quickMaskOpacity"))->setValue(3);
        d->reject();
    });
    QTest::mouseDClick(button(), Qt::LeftButton);
    QCOMPARE(quick_mask_opacity(*view_), opacity);
    QVERIFY(quick_mask_active(*view_));
}

void QuickMaskTest::polarityControlsAgree()
{
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    QTest::mouseClick(button(), Qt::LeftButton, Qt::AltModifier);
    QVERIFY(quick_mask_selected_areas(*view_));
    QCOMPARE(coverage(16, 16), 0);
    QCOMPARE(crop().pixelColor(16, 16), QColor(Qt::white));
    QVERIFY(paint(Qt::black));
    QCOMPARE(coverage(16, 16), 255);
}

void QuickMaskTest::polarityWaitsForStroke()
{
    QVERIFY(enter());
    QVERIFY(begin(0xff000000u));
    QVERIFY(view_->paint_dab(8, 8, 1));
    QVERIFY(quick_mask_set_options(*view_, true, 0xffff0000u, 50));
    QVERIFY(view_->paint_dab(16, 16, 1));
    QCOMPARE(crop().pixelColor(16, 16), QColor(255, 128, 128));
    QVERIFY(view_->end_paint());
    QVERIFY(quick_mask_selected_areas(*view_));
    QVERIFY(paint(Qt::black));
    QCOMPARE(coverage(16, 16), 255);
}

void QuickMaskTest::grayscaleAndSwatchesFollowMode()
{
    auto* c = window_->findChild<ColorState*>();
    QVERIFY(c);
    c->setForeground(QColor(25, 60, 90));
    QVERIFY(enter());
    QCOMPARE(c->foreground(), QColor(Qt::black));
    QCOMPARE(c->background(), QColor(Qt::white));
    QVERIFY(paint(Qt::black));
    QVERIFY(quick_mask_set_grayscale(*view_, true));
    QCOMPARE(crop().pixelColor(16, 16), QColor(Qt::black));
    QVERIFY(quick_mask_set_grayscale(*view_, false));
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(c->foreground(), QColor(25, 60, 90));
}

void QuickMaskTest::optionsPersistWithoutMode()
{
    QVERIFY(enter());
    QVERIFY(quick_mask_set_options(*view_, true, 0xff123456u, 73));
    window_->closeDocument(window_->activeDocumentIndex(), false);
    window_->saveSessionNow();
    window_.reset();
    window_ = test::makeMainWindow();
    QVERIFY(window_);
    QVERIFY(window_->newDocument(QStringLiteral("Reload"), 32, 32,
        QStringLiteral("rgb"), 8,
        QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(!quick_mask_active(*view_));
    QVERIFY(enter());
    QCOMPARE(quick_mask_opacity(*view_), 73);
    QCOMPARE(quick_mask_color(*view_), quint32(0xff123456u));
    QVERIFY(quick_mask_selected_areas(*view_));
}

void QuickMaskTest::historyAcrossModeBoundary()
{
    const int history = view_->history_count();
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    QVERIFY(paint(Qt::white));
    QCOMPARE(view_->history_count(), history + 2);
    QVERIFY(view_->undo());
    QCOMPARE(coverage(16, 16), 0);
    QVERIFY(view_->redo());
    QCOMPARE(coverage(16, 16), 255);
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(view_->history_count(), history + 2);
    QVERIFY(view_->undo());
    QCOMPARE(coverage(16, 16), 0);
    QVERIFY(view_->redo());
    QCOMPARE(coverage(16, 16), 255);
}

void QuickMaskTest::cancelRestoresCoverage()
{
    QVERIFY(enter());
    const int history = view_->history_count();
    QVERIFY(begin(0xff000000u));
    QVERIFY(view_->paint_dab(16, 16, 1));
    view_->cancel_paint();
    QVERIFY(!view_->has_selection());
    QCOMPARE(crop().pixelColor(16, 16), QColor(Qt::white));
    QCOMPARE(view_->history_count(), history);
    QVERIFY(paint(Qt::black));
    const int before = coverage(16, 16);
    const QImage image = crop();
    const int filteredHistory = view_->history_count();
    QVERIFY(filter_preview(*view_, QStringLiteral("gaussian-blur"), { 4.0 }));
    QVERIFY(coverage(16, 16) != before);
    QVERIFY(filter_preview_cancel(*view_));
    QCOMPARE(coverage(16, 16), before);
    QCOMPARE(crop(), image);
    QCOMPARE(view_->history_count(), filteredHistory);
}

void QuickMaskTest::documentsKeepIndependentModes()
{
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    PictureView* first = view_;
    const int index = window_->activeDocumentIndex();
    QVERIFY(window_->newDocument(QStringLiteral("Other"), 32, 32,
        QStringLiteral("rgb"), 8,
        QStringLiteral("white")));
    QVERIFY(!quick_mask_active(*window_->activeView()));
    window_->setActiveDocumentIndex(index);
    view_ = window_->activeView();
    QCOMPARE(view_, first);
    QVERIFY(quick_mask_active(*view_));
    QCOMPARE(coverage(16, 16), 0);
}

void QuickMaskTest::presentationTracksTransform()
{
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    const QImage original = view_->image();
    for (int side : { 1, 512 }) {
        PictureView fixture;
        QVERIFY(document_api::newDocument(fixture, side, side, QStringLiteral("rgb"), 8,
            QStringLiteral("white")));
        QVERIFY(quick_mask_set(fixture, true));
        QVERIFY(bucket_fill_at(fixture, 0, 0, 0xff000000u, -1,
            QStringLiteral("normal"), 100, 0, false, true, false));
        for (int level = 0; level <= (side == 1 ? 0 : 1); ++level) {
            const int size = side == 1 ? 1 : 8;
            const QImage sample = fixture.canvas_display_image(level, 0, 0, size, size);
            QVERIFY(!sample.isNull());
            QCOMPARE(sample.size(), QSize(size, size));
            QCOMPARE(sample.pixelColor(0, 0), QColor(255, 128, 128));
        }
    }
    QImage target(64, 64, QImage::Format_ARGB32_Premultiplied);
    target.setDevicePixelRatio(2);
    target.fill(Qt::transparent);
    QPainter painter(&target);
    painter.drawImage(QRect(0, 0, 32, 32), crop());
    painter.end();
    QCOMPARE(target.pixelColor(32, 32), QColor(255, 128, 128));
    QCOMPARE(view_->image(), original);
}

void QuickMaskTest::coverageIgnoresDocumentDepthAndMode()
{
    for (const auto& mode : { QStringLiteral("rgb"), QStringLiteral("grayscale"),
             QStringLiteral("cmyk"), QStringLiteral("lab") }) {
        for (int depth : { 8, 16, 32 }) {
            PictureView view;
            if (!document_api::newDocument(view, 8, 8, mode, depth,
                    QStringLiteral("white")))
                continue;
            const QImage before = view.image();
            QVERIFY(quick_mask_set(view, true));
            const PaintTip tip { 8, 100, 100, 0, 25, 0, 1, 0, 0, 0 };
            QVERIFY(begin_brush(view, 0xff404040u, 0xffffffffu, tip, 100, 100,
                QStringLiteral("normal"), false, false, false,
                false));
            QVERIFY(view.paint_dab(4, 4, 1));
            QVERIFY(view.end_paint());
            QVERIFY(quick_mask_set(view, false));
            QCOMPARE(view.selection_coverage(4, 4), 64);
            QCOMPARE(view.image(), before);
        }
    }
}

void QuickMaskTest::saveOmitsTemporaryMask()
{
    const QImage original = view_->image();
    QTemporaryDir dir;
    const QString baselinePath = dir.filePath(QStringLiteral("baseline.psd"));
    QVERIFY(document_api::save(*view_, baselinePath));
    PictureView baseline;
    QVERIFY(document_api::open(baseline, baselinePath));
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    const QString path = dir.filePath(QStringLiteral("quick.psd"));
    QVERIFY(document_api::save(*view_, path));
    PictureView other;
    QVERIFY(document_api::open(other, path));
    QVERIFY(!quick_mask_active(other));
    QCOMPARE(other.image(), original);
    QCOMPARE(other.selection_channel_count(), baseline.selection_channel_count());
}

void QuickMaskTest::savedAlphaRoundTrips()
{
    QVERIFY(enter());
    QVERIFY(paint(QColor(64, 64, 64)));
    QVERIFY(quick_mask_set(*view_, false));
    QVERIFY(view_->save_selection(QStringLiteral("Mask")));
    QTemporaryDir dir;
    const QString path = dir.filePath(QStringLiteral("alpha.psd"));
    QVERIFY(document_api::save(*view_, path));
    PictureView other;
    QVERIFY(document_api::open(other, path));
    QVERIFY(other.load_selection(QStringLiteral("Alpha %1").arg(other.selection_channel_count())));
    QCOMPARE(other.selection_coverage(16, 16), 64);
}

void QuickMaskTest::toolbarAndShortcutShareState()
{
    auto* toolbox = window_->findChild<Toolbox*>();
    QVERIFY(toolbox);
    QVERIFY(button());
    for (int columns : { 1, 2 }) {
        toolbox->setColumns(columns);
        QTest::mouseClick(button(), Qt::LeftButton);
        QTRY_VERIFY(quick_mask_active(*view_));
        QVERIFY(button()->isChecked());
        QTest::keyClick(window_->imageView(), Qt::Key_Q);
        QTRY_VERIFY(!quick_mask_active(*view_));
        QVERIFY(!button()->isChecked());
    }
    auto* action = window_->registry()->action(QStringLiteral("view.quickMask"));
    QVERIFY(action);
    action->setShortcut(QKeySequence(QStringLiteral("Ctrl+Alt+Shift+F12")));
    QTest::keyClick(window_->imageView(), Qt::Key_Q);
    QVERIFY(!quick_mask_active(*view_));
    QTest::keyClick(window_->imageView(), Qt::Key_F12,
        Qt::ControlModifier | Qt::AltModifier | Qt::ShiftModifier);
    QTRY_VERIFY(quick_mask_active(*view_));
    action->setShortcut(QKeySequence());
    QTest::keyClick(window_->imageView(), Qt::Key_Q);
    QTest::keyClick(window_->imageView(), Qt::Key_F12,
        Qt::ControlModifier | Qt::AltModifier | Qt::ShiftModifier);
    QVERIFY(quick_mask_active(*view_));
}

void QuickMaskTest::toolbarTracksActiveDocument()
{
    QVERIFY(enter());
    QVERIFY(button()->isChecked());
    window_->closeDocument(window_->activeDocumentIndex(), false);
    QVERIFY(!button()->isEnabled());
}

void QuickMaskTest::optionsGesturesDoNotToggleMode()
{
    for (bool active : { false, true }) {
        QVERIFY(quick_mask_set(*view_, active));
        auto* toolbox = window_->findChild<Toolbox*>();
        QSignalSpy options(toolbox, &Toolbox::quickMaskOptionsRequested);
        for (auto mouseButton : { Qt::RightButton, Qt::MiddleButton }) {
            QTest::mouseClick(button(), mouseButton);
            QTest::mouseDClick(button(), mouseButton);
        }
        // Wait past the delayed single-click timer to detect an unwanted toggle.
        QTest::qWait(QApplication::doubleClickInterval() + 10);
        QCOMPARE(quick_mask_active(*view_), active);
        QCOMPARE(options.count(), 0);
        QTimer::singleShot(0, [] {
            auto* d = qobject_cast<QDialog*>(QApplication::activeModalWidget());
            QVERIFY(d);
            d->reject();
        });
        QTest::mouseDClick(button(), Qt::LeftButton);
        QCOMPARE(quick_mask_active(*view_), active);
        QTest::mouseClick(button(), Qt::LeftButton, Qt::AltModifier);
        QCOMPARE(quick_mask_active(*view_), active);
    }
}

void QuickMaskTest::textInputOwnsShortcut()
{
    QLineEdit edit(window_.get());
    edit.show();
    edit.setFocus();
    QTest::keyClick(&edit, Qt::Key_Q);
    QCOMPARE(edit.text(), QStringLiteral("q"));
    QVERIFY(!quick_mask_active(*view_));
}

void QuickMaskTest::temporaryChannelFollowsCoverage()
{
    auto* p = channels();
    const int count = p->channelCountForTest();
    QVERIFY(enter());
    p->refresh();
    QCOMPARE(p->channelCountForTest(), count + 1);
    QCOMPARE(p->channelNameForTest(count), QStringLiteral("Quick Mask"));
    const QImage before = p->channelThumbnailForTest(count).toImage();
    QVERIFY(paint(Qt::black));
    p->refresh();
    QVERIFY(p->channelThumbnailForTest(count).toImage() != before);
    QVERIFY(quick_mask_set(*view_, false));
    p->refresh();
    QCOMPARE(p->channelCountForTest(), count);
}

void QuickMaskTest::temporaryChannelRejectsDurableActions()
{
    QVERIFY(enter());
    auto* p = channels();
    p->refresh();
    auto* list = p->findChild<QListWidget*>();
    QVERIFY(list);
    list->setCurrentRow(p->channelCountForTest() - 1);
    const auto* remove = p->findChild<QToolButton*>(QStringLiteral("channelsDelete"));
    QVERIFY(remove);
    QVERIFY(!remove->isEnabled());
    QCOMPARE(view_->selection_channel_count(), 0);
}

void QuickMaskTest::channelNavigationCommitsSelection()
{
    QVERIFY(enter());
    QVERIFY(paint(Qt::black));
    auto* p = channels();
    p->refresh();
    auto* list = p->findChild<QListWidget*>();
    QVERIFY(list);
    list->setCurrentRow(p->channelCountForTest() - 1);
    QCOMPARE(crop().pixelColor(16, 16), QColor(Qt::black));
    list->setCurrentRow(0);
    QVERIFY(!quick_mask_active(*view_));
    QCOMPARE(coverage(16, 16), 0);
}

void QuickMaskTest::activeLayerHighlightFollowsMode()
{
    auto* p = window_->findChild<LayersPanel*>();
    QVERIFY(p);
    auto* tree = p->findChild<QTreeView*>();
    QVERIFY(tree);
    const QString path = layer_selection_anchor(*view_);
    const QColor normal = layerRowSurface(p, path);
    const QColor red = Theme::tokenColor(QStringLiteral("quick-mask-active"));
    QVERIFY(normal.isValid());
    QVERIFY(normal != red);
    const int history = view_->history_count();
    QVERIFY(enter());
    QCOMPARE(tree->property("quickMaskActivePath").toString(), path);
    QCOMPARE(layerRowSurface(p, path), red);
    QVERIFY(quick_mask_set(*view_, false));
    QVERIFY(tree->property("quickMaskActivePath").toString().isEmpty());
    QCOMPARE(layerRowSurface(p, path), normal);
    QCOMPARE(view_->history_count(), history);
}

void QuickMaskTest::redHighlightFollowsActiveTarget()
{
    QVERIFY(enter());
    auto* p = window_->findChild<LayersPanel*>();
    QVERIFY(p);
    auto* tree = p->findChild<QTreeView*>();
    QVERIFY(tree);
    const QString path = layer_selection_anchor(*view_);
    const QColor red = Theme::tokenColor(QStringLiteral("quick-mask-active"));
    const int history = view_->history_count();
    QCOMPARE(tree->property("quickMaskActivePath").toString(), path);
    QCOMPARE(layerRowSurface(p, path), red);
    clear_layer_target(*view_);
    QCoreApplication::processEvents();
    QVERIFY(layer_selection_anchor(*view_).isEmpty());
    QVERIFY(tree->property("quickMaskActivePath").toString().isEmpty());
    const QColor unselected = layerRowSurface(p, path);
    QVERIFY(unselected.isValid());
    QVERIFY(unselected != red);
    QVERIFY(p->clickRowForTest(path));
    QCOMPARE(layer_selection_anchor(*view_), path);
    QCOMPARE(tree->property("quickMaskActivePath").toString(), path);
    QCOMPARE(layerRowSurface(p, path), red);
    tree->selectionModel()->clearSelection();
    QVERIFY(layer_selection_anchor(*view_).isEmpty());
    QVERIFY(tree->property("quickMaskActivePath").toString().isEmpty());
    QCOMPARE(layerRowSurface(p, path), unselected);
    QVERIFY(p->clickRowForTest(path));
    QCOMPARE(layer_selection_anchor(*view_), path);
    QCOMPARE(tree->property("quickMaskActivePath").toString(), path);
    QCOMPARE(layerRowSurface(p, path), red);
    QCOMPARE(view_->history_count(), history);
}

void QuickMaskTest::contentMovesRefuseWithoutPreview()
{
    QVERIFY(view_->select_rect(4, 4, 16, 16, QStringLiteral("new"), 0));
    for (bool masked : { false, true }) {
        if (masked) {
            QVERIFY(layer_mask_add(*view_, QStringLiteral("reveal-all")));
            QVERIFY(layer_mask_set_linked(*view_, false));
        }
        QVERIFY(enter());
        const QImage image = view_->image();
        const QByteArray mask = maskBytes();
        const QString anchor = layer_selection_anchor(*view_);
        const QString maskTarget = mask_edit_target_get(*view_);
        const int history = view_->history_count();
        for (bool duplicate : { false, true }) {
            QVERIFY(!pictura::move_selection_content(*view_, 4, 4, duplicate,
                0xffffffffu));
            QVERIFY(!begin_selection_move(*view_));
            QVERIFY(!preview_selection_move(*view_, 4, 4));
            QVERIFY(!view_->begin_move_preview());
            QVERIFY(!view_->begin_move_duplicate());
            QVERIFY(!view_->commit_move(4, 4));
            QVERIFY(!view_->translate_layer(1, 0));
            QVERIFY(!view_->move_preview(1, 0));
            window_->setActiveTool(ToolId::Move);
            auto* tools = window_->findChild<ToolController*>();
            QVERIFY(tools->handleNudgeKey(Qt::Key_Right, Qt::NoModifier));
            QVERIFY(tools->handleNudgeKey(Qt::Key_Down, Qt::ShiftModifier));
            auto* canvas = window_->imageView();
            canvas->mousePressed(
                QPointF(8, 8), Qt::LeftButton,
                int(duplicate ? Qt::AltModifier : Qt::ControlModifier));
            canvas->mouseMoved(QPointF(12, 12));
            canvas->mouseReleased(QPointF(12, 12));
        }
        QCOMPARE(view_->history_count(), history);
        QCOMPARE(view_->image(), image);
        QCOMPARE(maskBytes(), mask);
        QCOMPARE(layer_selection_anchor(*view_), anchor);
        QCOMPARE(mask_edit_target_get(*view_), maskTarget);
        QCOMPARE(coverage(5, 5), 255);
        QVERIFY(quick_mask_set(*view_, false));
    }
}

void QuickMaskTest::entryCancelsContentMovePreview()
{
    QVERIFY(view_->select_rect(4, 4, 16, 16, QStringLiteral("new"), 0));
    const int history = view_->history_count();
    QVERIFY(begin_selection_move(*view_));
    QVERIFY(preview_selection_move(*view_, 4, 4));
    QVERIFY(enter());
    QCOMPARE(coverage(5, 5), 255);
    QCOMPARE(coverage(23, 23), 0);
    QCOMPARE(view_->history_count(), history);
    QVERIFY(quick_mask_set(*view_, false));
    const QImage image = view_->image();
    QVERIFY(view_->begin_move_duplicate());
    QVERIFY(enter());
    QCOMPARE(view_->image(), image);
    QCOMPARE(coverage(5, 5), 255);
    QCOMPARE(view_->history_count(), history);
}

void QuickMaskTest::targetSwatchesSurviveQuickMaskAndDocumentSwitch()
{
    auto* c = window_->findChild<ColorState*>();
    QVERIFY(c);
    c->setForeground(QColor(20, 40, 60));
    QVERIFY(layer_mask_add(*view_, QStringLiteral("reveal-all")));
    auto* layers = window_->findChild<LayersPanel*>();
    layers->refresh();
    QVERIFY(layers->clickMaskThumbnailForTest(mask_edit_target_get(*view_)));
    c->setForeground(QColor(80, 80, 80));
    const QString target = mask_edit_target_get(*view_);
    const int index = window_->activeDocumentIndex();
    QVERIFY(enter());
    c->setForeground(QColor(100, 100, 100));
    QVERIFY(window_->newDocument(QStringLiteral("Other"), 8, 8,
        QStringLiteral("rgb"), 8,
        QStringLiteral("white")));
    window_->setActiveDocumentIndex(index);
    view_ = window_->activeView();
    QCOMPARE(c->foreground(), QColor(100, 100, 100));
    QVERIFY(quick_mask_set(*view_, false));
    QCOMPARE(mask_edit_target_get(*view_), target);
    QCOMPARE(c->foreground(), QColor(80, 80, 80));
    auto* p = window_->findChild<LayersPanel*>();
    QVERIFY(p->clickLayerThumbnailForTest(target));
    QCOMPARE(c->foreground(), QColor(20, 40, 60));
}

QTEST_MAIN(QuickMaskTest)
#include "tst_quick_mask.moc"
