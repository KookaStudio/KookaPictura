// View menu view-state wiring: Extras, Show, Show Extra Options, grid, pixel
// grid, layer edges, target path, selection edges, color samplers, slice
// commands and the View zoom presets.

#include <QtTest/QtTest>

#include "canvas_scrollbars.h"
#include "commands.h"
#include "image_view.h"
#include "preferences_dialog.h"
#include "session.h"
#include "show_extras_dialog.h"

#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/transform_api.h"
#include "api/transform_bridge.h"

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QSpinBox>

#include <cmath>
#include <functional>
#include <memory>

#include "qt_test_support.h"

namespace {

using pictura::DocWindow;
using pictura::ImageView;
using pictura::PicturaMainWindow;
using pictura::PictureView;
using pictura::ShowExtrasDialog;

QAction* action(PicturaMainWindow& window, const char* id)
{
    window.registry()->refresh();
    return window.registry()->action(QString::fromLatin1(id));
}

bool dispatch(PicturaMainWindow& window, const char* id)
{
    return window.registry()->dispatch(QString::fromLatin1(id));
}

QList<ImageView*> canvases(PicturaMainWindow& window)
{
    QList<ImageView*> out;
    for (ImageView* canvas : window.findChildren<ImageView*>()) {
        if (canvas->hasDocument()) {
            out.append(canvas);
        }
    }
    return out;
}

pictura::CanvasScrollBars* hostOf(PicturaMainWindow& window, ImageView* view)
{
    for (auto* host : window.findChildren<pictura::CanvasScrollBars*>()) {
        if (host->view() == view) {
            return host;
        }
    }
    return nullptr;
}

bool openExtrasDialog(PicturaMainWindow& window,
                      const std::function<void(ShowExtrasDialog*)>& edit)
{
    bool opened = false;
    QTimer::singleShot(0, [&window, &opened, edit]() {
        auto* dialog = window.findChild<ShowExtrasDialog*>();
        if (!dialog) {
            return;
        }
        opened = true;
        edit(dialog);
    });
    return dispatch(window, pictura::command_ids::ViewShowExtraOptions) && opened;
}

void activateCombo(QComboBox* combo, int index)
{
    combo->setCurrentIndex(index);
    combo->activated(index);
}

class ViewMenuTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanupTestCase();
    void init();
    void cleanup();

    void defaultsAndPersistence();
    void extrasMembership();
    void extrasOptionsWindow();
    void selectionEdgesScope();
    void colorSamplers();
    void globalCommands();
    void gridOverlay();
    void gridPreferences();
    void pixelGrid();
    void layerEdges();
    void targetPath();
    void sliceVisibility();
    void sliceLock();
    void clearSlices();
    void countVisibility();
    void noteVisibility();
    void smartGuidesExtras();
    void guidesExtras();
    void zoomPresets();

private:
    std::unique_ptr<pictura::test::ScopedStateHome> stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

void ViewMenuTest::initTestCase()
{
    qputenv("PICTURA_EDITION", "extended");
}

void ViewMenuTest::cleanupTestCase()
{
    qunsetenv("PICTURA_EDITION");
}

void ViewMenuTest::init()
{
    stateHome_ = std::make_unique<pictura::test::ScopedStateHome>();
    QVERIFY(stateHome_->isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("ViewMenu"), 100, 100,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    window_->show();
    QCoreApplication::processEvents();
    window_->registry()->refresh();
}

void ViewMenuTest::cleanup()
{
    window_.reset();
    stateHome_.reset();
}

void ViewMenuTest::defaultsAndPersistence()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);

    QVERIFY(action(*window_, pictura::command_ids::ViewShowTargetPath)->isChecked());
    QVERIFY(!action(*window_, pictura::command_ids::ViewShowGrid)->isChecked());
    QVERIFY(action(*window_, pictura::command_ids::ViewExtras)->isChecked());
    QVERIFY(!action(*window_, pictura::command_ids::ViewLockSlices)->isChecked());
    QVERIFY(action(*window_, pictura::command_ids::View200)->isEnabled());

    QVERIFY(window_->newDocument(QStringLiteral("Second"), 50, 50,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QCoreApplication::processEvents();
    const QList<ImageView*> all = canvases(*window_);
    QVERIFY(all.size() >= 2);
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowLayerEdges));
    for (ImageView* item : all) {
        QVERIFY(!item->layerEdgesVisibleForTest());
    }

    const int historyBefore = window_->activeView()->history_index();
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowGuides));
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowGrid));
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewExtras));
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowSlices));
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowNotes));
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowCount));
    for (ImageView* item : all) {
        QVERIFY(!item->slicesVisibleForTest());
        QVERIFY(!item->notesVisibleForTest());
        QVERIFY(!item->countVisibleForTest());
    }
    QCOMPARE(window_->activeView()->history_index(), historyBefore);

    window_->saveSession();
    const pictura::SessionState stored = pictura::loadSession();
    QVERIFY(!stored.showLayerEdges);
    QVERIFY(!stored.guidesVisible);
    QVERIFY(stored.showGrid);
    QVERIFY(!stored.extras);
    QVERIFY(!stored.showSlices);
    QVERIFY(!stored.showNotes);
    QVERIFY(!stored.showCount);

    // A fresh window against the same state home is the restart.
    window_.reset();
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    window_->registry()->refresh();
    QVERIFY(action(*window_, pictura::command_ids::ViewShowGrid)->isChecked());
    QVERIFY(!action(*window_, pictura::command_ids::ViewExtras)->isChecked());
    QVERIFY(!action(*window_, pictura::command_ids::ViewShowSlices)->isChecked());
    QVERIFY(!action(*window_, pictura::command_ids::ViewShowNotes)->isChecked());
    QVERIFY(!action(*window_, pictura::command_ids::ViewShowCount)->isChecked());
    QVERIFY(action(*window_, pictura::command_ids::ViewShowTargetPath)->isChecked());

    while (window_->documentCount() > 0) {
        QVERIFY(window_->closeDocument(window_->activeDocumentIndex(), false));
    }
    window_->registry()->refresh();
    QVERIFY(!action(*window_, pictura::command_ids::ViewShowGrid)->isEnabled());
    QVERIFY(!action(*window_, pictura::command_ids::View200)->isEnabled());
    const bool before = action(*window_, pictura::command_ids::ViewExtras)->isChecked();
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QCOMPARE(action(*window_, pictura::command_ids::ViewExtras)->isChecked(), before);

    QVERIFY(window_->newDocument(QStringLiteral("Later"), 40, 40,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QCoreApplication::processEvents();
    QVERIFY(action(*window_, pictura::command_ids::ViewShowTargetPath)->isChecked());
    QVERIFY(!window_->imageView()->targetPathVisibleForTest()); // Extras is still off

    // Restore visible extras so the grid is a positive check on the new doc.
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewExtras));

    // A document held in flight receives the state when its canvas is built.
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QImage seed(8, 8, QImage::Format_RGB32);
    seed.fill(Qt::red);
    const QString path = dir.filePath(QStringLiteral("late.png"));
    QVERIFY(seed.save(path));
    window_->setOpenDelayForTest(100);
    bool sawLoading = false;
    QObject alive;
    QTimer::singleShot(20, &alive, [&]() {
        if (window_->documentCount() > 0) {
            sawLoading = window_->isLoadingDocument(window_->documentCount() - 1);
            dispatch(*window_, pictura::command_ids::ViewShowTargetPath);
            dispatch(*window_, pictura::command_ids::ViewShowTargetPath);
        }
    });
    QVERIFY(window_->openImagePath(path));
    QVERIFY(sawLoading);
    QTRY_VERIFY_WITH_TIMEOUT(window_->imageView() != nullptr, 5000);
    QTRY_VERIFY_WITH_TIMEOUT(window_->imageView()->gridVisibleForTest(), 5000);
}

void ViewMenuTest::extrasMembership()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    bool gridFound = false;
    QVERIFY(openExtrasDialog(*window_, [&gridFound](ShowExtrasDialog* dialog) {
        auto* grid = dialog->findChild<QCheckBox*>(
            QStringLiteral("extrasOption_view.show.grid"));
        gridFound = grid != nullptr;
        if (grid) {
            grid->setChecked(false);
        }
        dialog->accept();
    }));
    QVERIFY(gridFound);

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowGrid));
    QVERIFY(canvas->gridVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(action(*window_, pictura::command_ids::ViewShowGuides)->isChecked());
    QVERIFY(!canvas->guidesVisible());
    QVERIFY(canvas->gridVisibleForTest());

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowAll));
    QVERIFY(action(*window_, pictura::command_ids::ViewExtras)->isChecked());
    QVERIFY(action(*window_, pictura::command_ids::ViewShowGrid)->isChecked());
    QVERIFY(canvas->gridVisibleForTest());

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowNone));
    QVERIFY(!action(*window_, pictura::command_ids::ViewShowGrid)->isChecked());
    QVERIFY(!canvas->gridVisibleForTest());
    QVERIFY(!canvas->samplersVisibleForTest());
}

void ViewMenuTest::extrasOptionsWindow()
{
    int boxes = -1;
    bool gridFound = false;
    QVERIFY(openExtrasDialog(*window_, [&boxes, &gridFound](ShowExtrasDialog* dialog) {
        boxes = int(dialog->findChildren<QCheckBox*>().size());
        auto* grid = dialog->findChild<QCheckBox*>(
            QStringLiteral("extrasOption_view.show.grid"));
        gridFound = grid != nullptr;
        if (grid) {
            grid->setChecked(false);
        }
        dialog->accept();
    }));
    QCOMPARE(boxes, 10);
    QVERIFY(gridFound);
    window_->saveSession();
    QVERIFY(!pictura::loadSession().extrasMembers.contains(
        QString::fromLatin1(pictura::command_ids::ViewShowGrid)));

    bool slicesFound = false;
    QVERIFY(openExtrasDialog(*window_, [&slicesFound](ShowExtrasDialog* dialog) {
        auto* slices = dialog->findChild<QCheckBox*>(
            QStringLiteral("extrasOption_view.show.slices"));
        slicesFound = slices != nullptr;
        if (slices) {
            slices->setChecked(false);
        }
        dialog->reject();
    }));
    QVERIFY(slicesFound);
    window_->saveSession();
    QVERIFY(pictura::loadSession().extrasMembers.contains(
        QString::fromLatin1(pictura::command_ids::ViewShowSlices)));
}

void ViewMenuTest::selectionEdgesScope()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    view->select_all();
    window_->refresh();
    QVERIFY(canvas->selectionEdgesVisible());

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowSelectionEdges));
    QVERIFY(!canvas->selectionEdgesVisible());

    QVERIFY(view->modify_selection(QStringLiteral("expand"), 1.0));
    window_->refresh();
    QVERIFY(!canvas->selectionEdgesVisible());

    pictura::add_user_slice(*view, 0, 0, 10, 10);
    view->undo();
    window_->refresh();
    QVERIFY(!canvas->selectionEdgesVisible());

    view->deselect();
    view->select_all();
    window_->refresh();
    QVERIFY(canvas->selectionEdgesVisible());

    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->selectionEdgesVisible());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(canvas->selectionEdgesVisible());

    QVERIFY(window_->newDocument(QStringLiteral("Second"), 40, 40,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    PictureView* second = window_->activeView();
    second->select_all();
    window_->refresh();
    dispatch(*window_, pictura::command_ids::ViewShowSelectionEdges);
    QVERIFY(!window_->imageView()->selectionEdgesVisible());
    second->deselect();
    second->select_all();
    window_->refresh();
    for (ImageView* item : canvases(*window_)) {
        QVERIFY(item->selectionEdgesVisible());
    }
}

void ViewMenuTest::colorSamplers()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(pictura::add_marker(*view, 0, 10, 10) >= 0);
    QVERIFY(pictura::add_marker(*view, 0, 20, 20) >= 0);
    window_->refresh();
    QCOMPARE(canvas->samplerOverlayCountForTest(), 2);

    dispatch(*window_, pictura::command_ids::ViewShowNone);
    QVERIFY(!canvas->samplersVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewShowAll);
    QVERIFY(canvas->samplersVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->samplersVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(canvas->samplersVisibleForTest());
}

void ViewMenuTest::globalCommands()
{
    while (window_->documentCount() > 0) {
        QVERIFY(window_->closeDocument(window_->activeDocumentIndex(), false));
    }
    window_->registry()->refresh();
    QVERIFY(action(*window_, pictura::command_ids::ViewShowExtraOptions)->isEnabled());
    QVERIFY(action(*window_, pictura::command_ids::ViewLockSlices)->isEnabled());
    QVERIFY(!action(*window_, pictura::command_ids::ViewShowGrid)->isEnabled());

    bool opened = false;
    QTimer::singleShot(0, [&]() {
        if (auto* dialog = window_->findChild<ShowExtrasDialog*>()) {
            opened = true;
            dialog->reject();
        }
    });
    dispatch(*window_, pictura::command_ids::ViewShowExtraOptions);
    QVERIFY(opened);

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewLockSlices));
    QVERIFY(action(*window_, pictura::command_ids::ViewLockSlices)->isChecked());
    if (auto* tools = window_->findChild<pictura::ToolController*>()) {
        QVERIFY(tools->slicesLockedForTest());
    }
    window_->saveSession();
    QVERIFY(pictura::loadSession().lockSlices);
}

void ViewMenuTest::gridOverlay()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(std::abs(pictura::document_ppi(*view) - 72.0) < 0.01);
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowGrid));
    QVERIFY(canvas->gridVisibleForTest());
    QVERIFY(std::abs(canvas->gridEveryXForTest() - 72.0) < 0.01);
    QVERIFY(std::abs(canvas->gridEveryYForTest() - 72.0) < 0.01);
    QCOMPARE(canvas->gridSubdivisionsForTest(), 4);
    QCOMPARE(canvas->gridStyleForTest(), 0);

    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->gridVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(canvas->gridVisibleForTest());
}

void ViewMenuTest::gridPreferences()
{
    window_->showPreferences(pictura::PreferencesDialog::kGuides);
    QCoreApplication::processEvents();
    auto* dialog = window_->findChild<pictura::PreferencesDialog*>();
    QVERIFY(dialog != nullptr);
    auto* color = dialog->findChild<QComboBox*>(QStringLiteral("preferencesGridColor"));
    auto* style = dialog->findChild<QComboBox*>(QStringLiteral("preferencesGridStyle"));
    auto* unit = dialog->findChild<QComboBox*>(QStringLiteral("preferencesGridUnit"));
    auto* every = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("preferencesGridEvery"));
    auto* subdivisions =
        dialog->findChild<QSpinBox*>(QStringLiteral("preferencesGridSubdivisions"));
    QVERIFY(color && style && unit && every && subdivisions);
    QVERIFY(color->isEnabled());
    QVERIFY(style->isEnabled());
    QVERIFY(unit->isEnabled());
    QVERIFY(every->isEnabled());
    QVERIFY(subdivisions->isEnabled());

    activateCombo(color, 5); // Magenta
    activateCombo(style, 2); // Dots
    activateCombo(unit, 6);  // Percent
    every->setValue(25.0);
    subdivisions->setValue(5);
    window_->saveSession();
    const pictura::SessionState state = pictura::loadSession();
    QCOMPARE(state.gridStyle, 2);
    QCOMPARE(state.gridUnit, 6);
    QVERIFY(std::abs(state.gridEvery - 25.0) < 0.01);
    QCOMPARE(state.gridSubdivisions, 5);
    QCOMPARE(state.gridColor, QStringLiteral("#ff4aff"));

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewShowGrid));
    ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    QVERIFY(std::abs(canvas->gridEveryXForTest() - 25.0) < 0.01);
    QVERIFY(std::abs(canvas->gridEveryYForTest() - 25.0) < 0.01);
    QCOMPARE(canvas->gridStyleForTest(), 2);
}

void ViewMenuTest::pixelGrid()
{
    ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    canvas->setZoom(5.0, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    QVERIFY(!canvas->pixelGridShownForTest());
    canvas->setZoom(6.0, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    QVERIFY(canvas->pixelGridShownForTest());

    dispatch(*window_, pictura::command_ids::ViewShowPixelGrid);
    QVERIFY(!canvas->pixelGridShownForTest());
    dispatch(*window_, pictura::command_ids::ViewShowPixelGrid);
    QVERIFY(canvas->pixelGridShownForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->pixelGridShownForTest());
}

void ViewMenuTest::layerEdges()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    window_->refresh();
    QVERIFY(canvas->layerEdgesForTest().width() > 0);

    const int added = view->add_layer(0);
    QVERIFY(added >= 0);
    view->set_active_layer(QString::number(added));
    window_->refresh();
    QVERIFY(canvas->layerEdgesForTest().isEmpty());

    view->set_active_layer(QStringLiteral("0"));
    window_->refresh();
    QVERIFY(canvas->layerEdgesForTest().width() > 0);

    pictura::add_user_slice(*view, 0, 0, 10, 10);
    window_->refresh();
    QVERIFY(canvas->layerEdgesForTest().width() > 0);
}

void ViewMenuTest::targetPath()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    pictura::path_append_corner(*view, 10, 10, false);
    pictura::path_commit_anchor(*view);
    pictura::path_append_corner(*view, 50, 10, false);
    pictura::path_commit_anchor(*view);
    pictura::path_finish(*view);
    window_->refresh();
    QVERIFY(!canvas->targetPathOverlayForTest().curve.isEmpty());
    QCOMPARE(canvas->pathOverlayAnchorCountForTest(), 0);

    dispatch(*window_, pictura::command_ids::ViewShowTargetPath);
    QVERIFY(!canvas->targetPathVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewShowTargetPath);
    QVERIFY(canvas->targetPathVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->targetPathVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);

    window_->setActiveTool(pictura::ToolId::Pen);
    QCoreApplication::processEvents();
    QVERIFY(!canvas->targetPathVisibleForTest());
}

void ViewMenuTest::sliceVisibility()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(pictura::add_user_slice(*view, 0, 0, 32, 24) >= 0);
    window_->setActiveTool(pictura::ToolId::Move);
    window_->refresh();
    QVERIFY(canvas->slicesVisibleForTest());
    QVERIFY(canvas->sliceOverlayCountForTest() > 0);

    dispatch(*window_, pictura::command_ids::ViewShowSlices);
    QVERIFY(!canvas->slicesVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewShowSlices);
    QVERIFY(canvas->slicesVisibleForTest());

    dispatch(*window_, pictura::command_ids::ViewShowSlices);
    window_->setActiveTool(pictura::ToolId::Slice);
    QCoreApplication::processEvents();
    QVERIFY(canvas->slicesVisibleForTest());
    window_->setActiveTool(pictura::ToolId::Move);
    QCoreApplication::processEvents();
    QVERIFY(!canvas->slicesVisibleForTest());
    window_->saveSession();
    QVERIFY(!pictura::loadSession().showSlices);
}

void ViewMenuTest::sliceLock()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(pictura::add_user_slice(*view, 0, 0, 32, 24) >= 0);
    window_->setActiveTool(pictura::ToolId::Slice);
    const int beforeCount = pictura::user_slice_count(*view);
    const int beforeHistory = view->history_index();

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewLockSlices));
    canvas->mousePressed(QPointF(60, 60), Qt::LeftButton, 0);
    canvas->mouseMoved(QPointF(80, 80));
    canvas->mouseReleased(QPointF(80, 80));
    QCOMPARE(pictura::user_slice_count(*view), beforeCount);
    QCOMPARE(view->history_index(), beforeHistory);

    dispatch(*window_, pictura::command_ids::ViewLockSlices);
    canvas->mousePressed(QPointF(60, 60), Qt::LeftButton, 0);
    canvas->mouseMoved(QPointF(80, 80));
    canvas->mouseReleased(QPointF(80, 80));
    QVERIFY(pictura::user_slice_count(*view) > beforeCount);
    QVERIFY(view->history_index() > beforeHistory);

    dispatch(*window_, pictura::command_ids::ViewLockSlices);
    QVERIFY(dispatch(*window_, pictura::command_ids::ViewClearSlices));
    QCOMPARE(pictura::user_slice_count(*view), 0);
    window_->saveSession();
    QVERIFY(pictura::loadSession().lockSlices);
}

void ViewMenuTest::clearSlices()
{
    PictureView* view = window_->activeView();
    QVERIFY(view);
    QVERIFY(pictura::add_user_slice(*view, 0, 0, 20, 20) >= 0);
    QVERIFY(pictura::add_user_slice(*view, 30, 30, 20, 20) >= 0);
    QCOMPARE(pictura::user_slice_count(*view), 2);
    const int before = view->history_index();

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewClearSlices));
    QCOMPARE(pictura::user_slice_count(*view), 0);
    QCOMPARE(view->history_index(), before + 1);
    QCOMPARE(view->history_label(before + 1), QStringLiteral("Clear Slices"));
    QCOMPARE(pictura::slice_count(*view), 1);
    QVERIFY(!action(*window_, pictura::command_ids::ViewClearSlices)->isEnabled());

    QVERIFY(view->undo());
    QCOMPARE(pictura::user_slice_count(*view), 2);

    while (window_->documentCount() > 0) {
        QVERIFY(window_->closeDocument(window_->activeDocumentIndex(), false));
    }
    window_->registry()->refresh();
    QVERIFY(!action(*window_, pictura::command_ids::ViewClearSlices)->isEnabled());
}

void ViewMenuTest::countVisibility()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(pictura::count_add_mark(*view, 10, 10) >= 0);
    window_->setActiveTool(pictura::ToolId::Move);
    window_->refresh();
    QVERIFY(canvas->countVisibleForTest());
    QVERIFY(canvas->countOverlayCountForTest() > 0);

    dispatch(*window_, pictura::command_ids::ViewShowCount);
    QVERIFY(!canvas->countVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewShowCount);
    QVERIFY(canvas->countVisibleForTest());

    window_->setActiveTool(pictura::ToolId::Count);
    QCoreApplication::processEvents();
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->countVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(canvas->countVisibleForTest());
}

void ViewMenuTest::noteVisibility()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(pictura::add_marker(*view, 1, 10, 10) >= 0);
    window_->setActiveTool(pictura::ToolId::Move);
    window_->refresh();
    QVERIFY(canvas->notesVisibleForTest());
    QVERIFY(canvas->noteOverlayCountForTest() > 0);

    dispatch(*window_, pictura::command_ids::ViewShowNotes);
    QVERIFY(!canvas->notesVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewShowNotes);
    QVERIFY(canvas->notesVisibleForTest());

    window_->setActiveTool(pictura::ToolId::Note);
    QCoreApplication::processEvents();
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->notesVisibleForTest());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(canvas->notesVisibleForTest());
}

void ViewMenuTest::smartGuidesExtras()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(canvas->smartGuidesEnabled());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->smartGuidesEnabled());
    QVERIFY(action(*window_, pictura::command_ids::ViewShowSmartGuides)->isChecked());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(canvas->smartGuidesEnabled());
}

void ViewMenuTest::guidesExtras()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    QVERIFY(pictura::add_guide(*view, true, 20) >= 0);
    window_->refresh();
    QVERIFY(canvas->guidesVisible());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->guidesVisible());
    QVERIFY(action(*window_, pictura::command_ids::ViewShowGuides)->isChecked());
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(canvas->guidesVisible());

    dispatch(*window_, pictura::command_ids::ViewShowGuides);
    dispatch(*window_, pictura::command_ids::ViewExtras);
    QVERIFY(!canvas->guidesVisible());
    const int before = pictura::guide_count(*view);
    if (auto* host = hostOf(*window_, canvas)) {
        host->guideDropped(true, 30.0);
    }
    QCoreApplication::processEvents();
    QVERIFY(action(*window_, pictura::command_ids::ViewShowGuides)->isChecked());
    QVERIFY(action(*window_, pictura::command_ids::ViewExtras)->isChecked());
    QVERIFY(canvas->guidesVisible());
    QVERIFY(pictura::guide_count(*view) > before);
}

void ViewMenuTest::zoomPresets()
{
    PictureView* view = window_->activeView();
    ImageView* canvas = window_->imageView();
    QVERIFY(view && canvas);
    canvas->setZoom(0.5, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    QVERIFY(dispatch(*window_, pictura::command_ids::View200));
    QVERIFY(std::abs(canvas->zoom() - 2.0) < 1e-6);

    QVERIFY(dispatch(*window_, pictura::command_ids::ViewPrintSize));
    QVERIFY(std::abs(canvas->zoom() - 96.0 / pictura::document_ppi(*view)) < 1e-6);

    while (window_->documentCount() > 0) {
        QVERIFY(window_->closeDocument(window_->activeDocumentIndex(), false));
    }
    window_->registry()->refresh();
    QVERIFY(!action(*window_, pictura::command_ids::View200)->isEnabled());
    QVERIFY(!action(*window_, pictura::command_ids::ViewPrintSize)->isEnabled());
    QVERIFY(window_->activeView() == nullptr);
}

} // namespace

QTEST_MAIN(ViewMenuTest)
#include "tst_view_menu.moc"
