// One presentation per committed edit: the transaction notifies once, the
// command handler does not refresh again, and the command table sees the
// history state that was just committed.

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QList>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QToolButton>

#include "api/type_api.h"
#include "commands.h"
#include "frame.h"
#include "panels/adjustments_panel.h"
#include "panels/channels_panel.h"
#include "panels/history_panel.h"
#include "panels/histogram_panel.h"
#include "panels/layers_panel.h"
#include "panels/navigator_panel.h"
#include "panels/paths_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/jobs.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_history/display.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_ops.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"
#include "qt_test_support.h"

namespace {

QString typeLayerPath(const pictura::PictureView& view)
{
    for (int i = 0; i < view.layer_count(); ++i) {
        const QString path = QString::number(i);
        if (pictura::type_api::readLayer(view, path).isType) {
            return path;
        }
    }
    return QString();
}

QString patternFixture()
{
    QString fixture =
        QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/stroke_pattern.psd");
    QDir dir(QCoreApplication::applicationDirPath());
    for (int i = 0; i < 6 && fixture.isEmpty(); ++i) {
        const QString candidate =
            dir.filePath(QStringLiteral("crates/pictura-codec/tests/fixtures/stroke_pattern.psd"));
        if (QFileInfo::exists(candidate)) {
            fixture = candidate;
        } else if (!dir.cdUp()) {
            break;
        }
    }
    return fixture;
}

QVector<quint32> histogramBins(pictura::HistogramPanel* panel)
{
    QVector<quint32> bins;
    if (!panel) {
        return bins;
    }
    bins.reserve(4 * 256);
    for (int channel = 0; channel < 4; ++channel) {
        for (int bin = 0; bin < 256; ++bin) {
            bins.append(panel->binForTest(channel, bin));
        }
    }
    return bins;
}

void closeAll(pictura::PicturaMainWindow& window)
{
    while (window.activeDocumentIndex() >= 0) {
        window.closeDocument(window.activeDocumentIndex(), false);
    }
}

QString paintedLayer(pictura::PictureView* view)
{
    if (!view) {
        return QString();
    }
    const QString layer = view->add_layer_in(QString());
    if (layer.isEmpty()) {
        return QString();
    }
    view->set_active_layer(layer);
    if (!view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 20, 100, 100, 0, 100, 100, 25,
                           QStringLiteral("normal"), false, false)
        || !view->paint_dab(32.0, 32.0, 1.0) || !view->end_paint()) {
        return QString();
    }
    return layer;
}

} // namespace

class EditRefreshTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void aCommandRefreshesOnce();
    void aCommitNotifiesOnce();
    void aSaveSelectionNotifies();
    void aCommittedEditShowsUndoState();
    void newLayerDoesNotRecomposite();
    void newGroupDoesNotRecomposite();
    void newLayerInClippingChainRecomposites();
    void newLayerRecordsOneStateWithoutRecomposite();
    void anEditRefreshesTheDockInPlace();
    void aDocumentSwitchRebindsTheDock();
    void aNewDocumentRefreshesLayersOnce();
    void aDeferredBindSurvivesClosingTheBoundDocument();
    void anEditDoesNotRebindTheDock();
    void aCommittedEditRefreshesLayersOnce();
    void aStrokeCommitRefreshesPanels();
    void aStrokeCommitDoesNotRetraceTheContour();
    void previewTicksCoalesceTheRegistry();
    void aCommitRefreshIsNotDelayedByPreviews();
    void anInactiveCloseDoesNotRebindTheDock();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void EditRefreshTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1000, 700);
}

void EditRefreshTest::cleanup()
{
    qunsetenv("PICTURA_TEST_FILTER_COMMIT_DELAY_MS");
    if (window_) {
        closeAll(*window_);
    }
}

void EditRefreshTest::aCommandRefreshesOnce()
{
    QVERIFY(window_->newDocument(QStringLiteral("Refresh Count"), 64, 64,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    panel->selectPaths({layer}, layer);

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    const int before = registry->refreshCount();
    QVERIFY2(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerFlattenImage)),
             "flatten image dispatch");
    const int after = registry->refreshCount();
    QCOMPARE(after - before, 1);
}

void EditRefreshTest::aCommitNotifiesOnce()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QSignalSpy changes(&view, &pictura::PictureView::changed);

    QVERIFY(!view.duplicate_layers(QStringList{QStringLiteral("0")}).isEmpty());
    QCOMPARE(changes.count(), 1);

    QVERIFY(view.new_document(24, 24, QStringLiteral("rgb"), 8, QStringLiteral("transparent")));
    QVERIFY(view.select_rect(3, 2, 5, 4, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(view, QColor(200, 0, 0).rgba(), -1, QStringLiteral("normal"),
                               100, false));
    view.deselect();
    changes.clear();
    QVERIFY(pictura::trim_image(view));
    QCOMPARE(changes.count(), 1);

    changes.clear();
    QVERIFY(pictura::canvas_size_apply(view, QStringLiteral("center"), 10, 10, 0));
    QCOMPARE(changes.count(), 1);

    QVERIFY(view.new_document(200, 200, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    changes.clear();
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = 30.0;
    const QString font = QStringLiteral("Liberation Sans");
    QVERIFY(pictura::type_api::createCharacterStyle(view, QStringLiteral("Probe"), font, character));
    QCOMPARE(changes.count(), 1);

    const pictura::TypeSetting placement = pictura::type_api::readLayer(view, QString()).spec.setting;
    const pictura::ParagraphSetting paragraph = pictura::type_api::defaultParagraphSetting();
    QVERIFY(!pictura::type_api::commitLayer(view, {QStringLiteral("Hello"), font, placement, character, paragraph})
                 .isEmpty());
    const QString path = typeLayerPath(view);
    QVERIFY2(!path.isEmpty(), "a type layer");
    changes.clear();
    QVERIFY(pictura::type_api::applyStyle(view, path, QStringLiteral("Probe"), false));
    QCOMPARE(changes.count(), 1);
}

void EditRefreshTest::aSaveSelectionNotifies()
{
    QVERIFY(window_->newDocument(QStringLiteral("Save Selection"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    view->select_all();

    auto* panel = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel != nullptr);
    panel->setView(view);
    panel->refresh();
    const int channelsBefore = panel->channelCountForTest();
    const int historyBefore = view->history_index();

    QSignalSpy changes(view, &pictura::PictureView::changed);
    QVERIFY(view->save_selection(QStringLiteral("Alpha 1")));
    QCOMPARE(changes.count(), 1);
    QCOMPARE(view->history_index(), historyBefore + 1);
    QCOMPARE(view->selection_channel_count(), 1);
    QTRY_COMPARE_WITH_TIMEOUT(panel->channelCountForTest(), channelsBefore + 1, 1000);
}

void EditRefreshTest::aCommittedEditShowsUndoState()
{
    QVERIFY(window_->newDocument(QStringLiteral("Undo Label"), 64, 64,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    panel->selectPaths({layer}, layer);

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerDuplicateLayer)));

    QAction* undo = registry->action(QString::fromLatin1(pictura::command_ids::EditUndo));
    QVERIFY(undo != nullptr);
    QVERIFY(undo->isEnabled());
    QVERIFY2(undo->text().contains(QStringLiteral("Duplicate")),
             qPrintable(undo->text()));
}

// A content-neutral insertion keeps the level-0 frame: the frame revision
// only moves on a full rebuild, and no region is blitted.
void EditRefreshTest::newLayerDoesNotRecomposite()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const quint64 frame = pictura::frame_revision(view);
    QSignalSpy blits(&view, &pictura::PictureView::regionDamaged);
    QSignalSpy changes(&view, &pictura::PictureView::changed);
    QVERIFY(!view.add_layer_in(QStringLiteral("0")).isEmpty());
    QCOMPARE(view.layer_count(), 2);
    QCOMPARE(pictura::frame_revision(view), frame);
    QCOMPARE(blits.count(), 0);
    QCOMPARE(changes.count(), 1);
}

void EditRefreshTest::newGroupDoesNotRecomposite()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const quint64 frame = pictura::frame_revision(view);
    QVERIFY(!view.add_group_in(QStringLiteral("0")).isEmpty());
    QCOMPARE(view.layer_kind(1), QStringLiteral("group"));
    QCOMPARE(pictura::frame_revision(view), frame);
}

// A new unclipped layer slipped under a clipped one rebases the clipped run,
// so it is not content-neutral and rebuilds the frame.
void EditRefreshTest::newLayerInClippingChainRecomposites()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QCOMPARE(view.add_layer_in(QStringLiteral("0")), QStringLiteral("1"));
    QCOMPARE(view.add_layer_in(QStringLiteral("1")), QStringLiteral("2"));
    QVERIFY(pictura::clipping_create(view, QStringList{QStringLiteral("2")}) > 0);
    const quint64 frame = pictura::frame_revision(view);
    QCOMPARE(view.add_layer_in(QStringLiteral("1")), QStringLiteral("2"));
    QVERIFY(pictura::frame_revision(view) != frame);
}

void EditRefreshTest::newLayerRecordsOneStateWithoutRecomposite()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    const int history = view.history_count();
    const quint64 frame = pictura::frame_revision(view);
    QVERIFY(!view.add_layer_in(QStringLiteral("0")).isEmpty());
    QCOMPARE(view.history_count(), history + 1);
    QCOMPARE(pictura::frame_revision(view), frame);
    QVERIFY(view.undo());
    QCOMPARE(view.layer_count(), 1);
}

void EditRefreshTest::anEditRefreshesTheDockInPlace()
{
    QVERIFY(window_->newDocument(QStringLiteral("DockInPlace"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    pictura::PictureView* view = window_->activeView();
    auto* layers = window_->layersPanelForTest();
    QVERIFY(view && layers);

    const int retarget = window_->dockRetargetCountForTest();
    const int base = layers->refreshCountForTest();
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    QTRY_COMPARE_WITH_TIMEOUT(layers->refreshCountForTest(), base + 1, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), base + 1);
    QCOMPARE(window_->dockRetargetCountForTest(), retarget);

    const int previewBase = layers->refreshCountForTest();
    QVERIFY(view->preview_layers_opacity(QStringList{layer}, 200) > 0);
    QTRY_VERIFY_WITH_TIMEOUT(layers->refreshCountForTest() > previewBase, 1000);
    QTest::qWait(250);
    QCOMPARE(window_->dockRetargetCountForTest(), retarget);

    closeAll(*window_);
}

void EditRefreshTest::aDocumentSwitchRebindsTheDock()
{
    QVERIFY(window_->newDocument(QStringLiteral("SwitchA"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    auto* paths = window_->findChild<pictura::PathsPanel*>(QStringLiteral("pathsPanel"));
    QVERIFY(paths);
    QVERIFY(paths->newPath());
    QCOMPARE(paths->listForTest()->count(), 1);
    QVERIFY(window_->newDocument(QStringLiteral("SwitchB"), 512, 512, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    QCOMPARE(paths->listForTest()->count(), 0);
    auto* layers = window_->layersPanelForTest();
    QVERIFY(layers);

    const int retargetA = window_->dockRetargetCountForTest();
    const int layersA = layers->refreshCountForTest();
    window_->setActiveDocumentIndex(0);
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > retargetA, 1000);
    QTRY_COMPARE_WITH_TIMEOUT(layers->refreshCountForTest(), layersA + 1, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), layersA + 1);
    QCOMPARE(paths->listForTest()->count(), 1);

    const int retargetB = window_->dockRetargetCountForTest();
    const int layersB = layers->refreshCountForTest();
    window_->setActiveDocumentIndex(1);
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > retargetB, 1000);
    QTRY_COMPARE_WITH_TIMEOUT(layers->refreshCountForTest(), layersB + 1, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), layersB + 1);
    QCOMPARE(paths->listForTest()->count(), 0);

    const int windows = window_->windowCountForTest(1);
    const int retargetWindow = window_->dockRetargetCountForTest();
    const int layersBeforeWindow = layers->refreshCountForTest();
    QVERIFY(window_->registry()->dispatch(QStringLiteral("window.arrange.new.window.for.file")));
    QTRY_COMPARE_WITH_TIMEOUT(window_->windowCountForTest(1), windows + 1, 1000);
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > retargetWindow, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), layersBeforeWindow + 1);

    const int windowSwitchA = layers->refreshCountForTest();
    const int windowRetargetA = window_->dockRetargetCountForTest();
    window_->activateWindowForTest(1, 0);
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > windowRetargetA, 1000);
    QTRY_COMPARE_WITH_TIMEOUT(layers->refreshCountForTest(), windowSwitchA + 1, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), windowSwitchA + 1);

    const int windowSwitchB = layers->refreshCountForTest();
    const int windowRetargetB = window_->dockRetargetCountForTest();
    window_->activateWindowForTest(1, 1);
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > windowRetargetB, 1000);
    QTRY_COMPARE_WITH_TIMEOUT(layers->refreshCountForTest(), windowSwitchB + 1, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), windowSwitchB + 1);

    const int generationRetarget = window_->dockRetargetCountForTest();
    QVERIFY(window_->activeView()->new_document(512, 512, QStringLiteral("rgb"), 8,
                                                QStringLiteral("white")));
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > generationRetarget, 1000);
    QTest::qWait(250);
    QCOMPARE(paths->listForTest()->count(), 0);

    const QString fixture = patternFixture();
    QVERIFY2(!fixture.isEmpty(), "pattern fixture");
    QVERIFY(window_->openDocumentAtPath(fixture));
    QTest::qWait(250);
    auto* adjustments =
        window_->findChild<pictura::AdjustmentsPanel*>(QStringLiteral("adjustmentsPanel"));
    QVERIFY(adjustments);
    QToolButton* pattern = adjustments->buttonByIdForTest(QStringLiteral("pattern"));
    QVERIFY(pattern);
    QTRY_VERIFY_WITH_TIMEOUT(pattern->isEnabled(), 1000);
    QVERIFY(layers->patternActionEnabledForTest());

    window_->setActiveDocumentIndex(1);
    QTest::qWait(250);
    QVERIFY(!pattern->isEnabled());
    QVERIFY(!layers->patternActionEnabledForTest());

    // A document switch whose new view has a filter commit running defers the
    // bind until the commit lands.
    window_->setActiveDocumentIndex(1);
    QTest::qWait(200);
    pictura::PictureView* busy = window_->activeView();
    QVERIFY(busy);
    const QString busyLayer = busy->add_layer_in(QString());
    QVERIFY(!busyLayer.isEmpty());
    busy->set_active_layer(busyLayer);
    QTest::qWait(200);
    qputenv("PICTURA_TEST_FILTER_COMMIT_DELAY_MS", "1000");
    const bool started = pictura::filter_commit_start(
        *busy, QStringLiteral("gaussian-blur"), QList<double>{8.0},
        QStringLiteral("Gaussian Blur"));
    qunsetenv("PICTURA_TEST_FILTER_COMMIT_DELAY_MS");
    QVERIFY2(started, "filter commit started");
    QVERIFY2(pictura::filter_commit_running(*busy), "filter commit did not start");
    window_->setActiveDocumentIndex(0);
    QTest::qWait(200);
    const int deferred = window_->dockRetargetCountForTest();
    window_->setActiveDocumentIndex(1);
    QTest::qWait(200);
    QCOMPARE(window_->dockRetargetCountForTest(), deferred);
    QVERIFY(pictura::filter_commit_finish(*busy));
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > deferred, 2000);

    closeAll(*window_);
}

void EditRefreshTest::aNewDocumentRefreshesLayersOnce()
{
    QVERIFY(window_->newDocument(QStringLiteral("First"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    auto* layers = window_->layersPanelForTest();
    QVERIFY(layers);
    const int base = layers->refreshCountForTest();

    QVERIFY(window_->newDocument(QStringLiteral("Second"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), base + 1);
}

void EditRefreshTest::aDeferredBindSurvivesClosingTheBoundDocument()
{
    QVERIFY(window_->newDocument(QStringLiteral("Bound"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QVERIFY(window_->newDocument(QStringLiteral("Busy"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    pictura::PictureView* busy = window_->activeView();
    QVERIFY(busy);
    const QString layer = busy->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    busy->set_active_layer(layer);
    QTest::qWait(250);

    qputenv("PICTURA_TEST_FILTER_COMMIT_DELAY_MS", "1000");
    QVERIFY(pictura::filter_commit_start(*busy, QStringLiteral("gaussian-blur"),
                                         QList<double>{8.0}, QStringLiteral("Gaussian Blur")));
    qunsetenv("PICTURA_TEST_FILTER_COMMIT_DELAY_MS");
    QVERIFY(pictura::filter_commit_running(*busy));
    window_->setActiveDocumentIndex(0);
    QTest::qWait(200);
    window_->setActiveDocumentIndex(1);
    QTest::qWait(200);
    const int deferred = window_->dockRetargetCountForTest();

    QVERIFY(window_->closeDocument(0, false));
    QTRY_VERIFY_WITH_TIMEOUT(window_->dockRetargetCountForTest() > deferred, 1000);
    QVERIFY(window_->activeView() == busy);
    QVERIFY(pictura::filter_commit_finish(*busy));
    QTest::qWait(250);

    closeAll(*window_);
}

void EditRefreshTest::anEditDoesNotRebindTheDock()
{
    QVERIFY(window_->newDocument(QStringLiteral("NoRebind"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    const int retarget = window_->dockRetargetCountForTest();
    QVERIFY(!window_->activeView()->add_layer_in(QString()).isEmpty());
    QTest::qWait(250);
    QCOMPARE(window_->dockRetargetCountForTest(), retarget);
    closeAll(*window_);
}

void EditRefreshTest::aCommittedEditRefreshesLayersOnce()
{
    QVERIFY(window_->newDocument(QStringLiteral("LayersOnce"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    auto* layers = window_->layersPanelForTest();
    QVERIFY(layers);

    const int base = layers->refreshCountForTest();
    QVERIFY(!window_->activeView()->add_layer_in(QString()).isEmpty());
    QTRY_COMPARE_WITH_TIMEOUT(layers->refreshCountForTest(), base + 1, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), base + 1);

    QVERIFY(window_->newDocument(QStringLiteral("Background"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    window_->setActiveDocumentIndex(0);
    QTest::qWait(250);
    pictura::PictureView* active = window_->activeView();
    pictura::PictureView* background = window_->viewAt(1);
    QVERIFY(active && background);
    const int beforeBackground = layers->refreshCountForTest();
    QVERIFY(!background->add_layer_in(QString()).isEmpty());
    QTest::qWait(250);
    QVERIFY2(layers->refreshCountForTest() > beforeBackground,
             "background change suppressed the active Layers panel");
    const int beforeActive = layers->refreshCountForTest();
    QVERIFY(!active->add_layer_in(QString()).isEmpty());
    QTRY_COMPARE_WITH_TIMEOUT(layers->refreshCountForTest(), beforeActive + 1, 1000);
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), beforeActive + 1);

    closeAll(*window_);
}

void EditRefreshTest::aStrokeCommitRefreshesPanels()
{
    QVERIFY(window_->newDocument(QStringLiteral("StrokePanels"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    pictura::PictureView* view = window_->activeView();
    auto* layers = window_->layersPanelForTest();
    auto* history = window_->findChild<pictura::HistoryPanel*>(QStringLiteral("historyPanel"));
    auto* histogram =
        window_->findChild<pictura::HistogramPanel*>(QStringLiteral("histogramPanel"));
    auto* navigator =
        window_->findChild<pictura::NavigatorPanel*>(QStringLiteral("navigatorPanel"));
    auto* channels = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view && layers && history && histogram && navigator && channels);
    auto* historyList = history->findChild<QListWidget*>();
    QVERIFY(historyList);

    view->set_gpu_compute(false);
    QTest::qWait(250);
    const int layerBase = layers->refreshCountForTest();
    const int historyRows = historyList->count();
    const QVector<quint32> bins = histogramBins(histogram);
    const QImage navigatorBefore = navigator->thumbnailSourceImageForTest();
    const QImage channelBefore = channels->channelThumbnailForTest(0).toImage();

    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 20, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(32.0, 32.0, 1.0));
    const int midStroke = layers->refreshCountForTest();
    QTest::qWait(250);
    QCOMPARE(layers->refreshCountForTest(), midStroke);
    QVERIFY(view->end_paint());

    QTRY_VERIFY_WITH_TIMEOUT(layers->refreshCountForTest() > layerBase, 1000);
    QTRY_VERIFY_WITH_TIMEOUT(historyList->count() > historyRows, 1000);
    QTRY_VERIFY_WITH_TIMEOUT(histogramBins(histogram) != bins, 1000);
    QTRY_VERIFY_WITH_TIMEOUT(navigator->thumbnailSourceImageForTest() != navigatorBefore, 1000);
    QTRY_VERIFY_WITH_TIMEOUT(channels->channelThumbnailForTest(0).toImage() != channelBefore, 1000);

    closeAll(*window_);
}

void EditRefreshTest::aStrokeCommitDoesNotRetraceTheContour()
{
    QVERIFY(window_->newDocument(QStringLiteral("StrokeContour"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    QVERIFY(view->select_rect(8, 8, 20, 20, QStringLiteral("new"), 0.0));
    window_->refreshSelectionOverlay();
    const quint64 computes = view->selection_contour_compute_count();

    view->set_gpu_compute(false);
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 20, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(32.0, 32.0, 1.0));
    QVERIFY(view->end_paint());
    QTest::qWait(250);
    window_->refreshSelectionOverlay();

    QCOMPARE(view->selection_contour_compute_count(), computes);
    closeAll(*window_);
}

void EditRefreshTest::previewTicksCoalesceTheRegistry()
{
    QVERIFY(window_->newDocument(QStringLiteral("PreviewRegistry"), 64, 64,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QTest::qWait(250);
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    view->set_gpu_compute(false);
    const QString layer = paintedLayer(view);
    QVERIFY(!layer.isEmpty());
    QTest::qWait(250);
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);

    const int before = registry->refreshCount();
    for (const int value : {240, 230, 220, 210}) {
        QVERIFY(view->preview_layers_opacity(QStringList{layer}, value) > 0);
    }
    QTest::qWait(250);
    const int after = registry->refreshCount();
    QVERIFY(after - before >= 1);
    QVERIFY(after - before <= 1);

    closeAll(*window_);
}

void EditRefreshTest::aCommitRefreshIsNotDelayedByPreviews()
{
    QVERIFY(window_->newDocument(QStringLiteral("CommitAfterPreview"), 64, 64,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QTest::qWait(250);
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    view->set_gpu_compute(false);
    const QString layer = paintedLayer(view);
    QVERIFY(!layer.isEmpty());
    QTest::qWait(250);
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);

    const int before = registry->refreshCount();
    QVERIFY(view->preview_layers_opacity(QStringList{layer}, 200) > 0);
    QVERIFY(!view->add_layer_in(QString()).isEmpty());
    const int after = registry->refreshCount();
    QCOMPARE(after - before, 1);
    QTest::qWait(250);
    QCOMPARE(registry->refreshCount(), after);

    closeAll(*window_);
}

void EditRefreshTest::anInactiveCloseDoesNotRebindTheDock()
{
    QVERIFY(window_->newDocument(QStringLiteral("Active"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QVERIFY(window_->newDocument(QStringLiteral("Inactive"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QTest::qWait(250);
    const int before = window_->dockRetargetCountForTest();
    QVERIFY(window_->closeDocument(0, false));
    QTest::qWait(250);
    QCOMPARE(window_->dockRetargetCountForTest(), before);
    closeAll(*window_);
}

QTEST_MAIN(EditRefreshTest)
#include "tst_edit_refresh.moc"
