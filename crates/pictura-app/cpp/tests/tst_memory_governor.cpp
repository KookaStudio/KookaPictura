// The memory governor in the application (#1211): the Memory Usage
// preference, the gates of the memory benchmark's paths, their status-bar
// messages, the control server's `refused` answers and `status.memory`, and a
// failed fallible allocation.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtCore/QTemporaryDir>
#include <QtCore/QThread>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QToolButton>

#include <functional>

#include "commands.h"
#include "command_ids.h"
#include "control_server.h"
#include "frame.h"
#include "memory_refusal.h"
#include "preferences_dialog.h"
#include "session.h"
#include "tools.h"

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/history_api.h"
#include "api/paint_bridge.h"
#include "api/transform_api.h"

#include "qt_test_support.h"

class MemoryGovernorTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void memoryUsageDefaultsToSixty();
    void memoryUsageChangeAppliesAndPersists();
    void everyPathRefusesWithoutRoom();
    void refusalsShowTheStatusMessage();
    void controlAnswersRefused();
    void statusReportsTheGovernor();
    void statusCarriesScratchReadouts();
    void aFailedAllocationLeavesTheEditUndone();
    void transformRefusalNamesFullScratchDisks();
    void moveChargesOnlyTheTiledComposite();
    void moveRefusalNamesFullScratchDisks();
    void fillsSpillToScratch();
    void fillRefusalNamesFullScratchDisks();

private:
    // A white 40×40 document, made active.
    pictura::PictureView* newDocument();
    QJsonObject call(const QString& method, const QJsonObject& params = {});
    static void pinNoRoom() { pictura::set_available_memory_for_test(0); }
    static void unpin() { pictura::set_available_memory_for_test(-1); }
    // A 4100² document whose top layer covers the canvas; a (5, 5) move changes
    // about 67 MB of composite at 4 bytes a pixel, 202 MB at 12.
    pictura::PictureView* largeMoveDocument();
    // Commit a (5, 5) move with `roomMiB` of governor room (256 MiB of
    // headroom is kept under the pinned available memory) and no scratch.
    static bool commitMoveWithRoom(pictura::PictureView* view, qint64 roomMiB);
    // A white 4100² document: over 16 Mpx, so a whole-canvas fill's refresh
    // only marks display tiles stale, and its Background forks about 50 MB of
    // tiles.
    pictura::PictureView* largeFillDocument();
    // Pin `roomMiB` of governor room and `scratchMiB` of free scratch.
    static void pinRoom(qint64 roomMiB, qint64 scratchMiB);

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir files_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void MemoryGovernorTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(files_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    server_ = std::make_unique<pictura::ControlServer>(window_.get(), QString());
}

void MemoryGovernorTest::init()
{
    while (window_->documentCount() > 0) {
        QVERIFY(window_->closeDocument(0, false));
    }
}

void MemoryGovernorTest::cleanup()
{
    unpin();
    pictura::set_memory_usage_percent(60);
}

pictura::PictureView* MemoryGovernorTest::newDocument()
{
    if (!window_->newDocument(QStringLiteral("Memory"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

QJsonObject MemoryGovernorTest::call(const QString& method, const QJsonObject& params)
{
    return server_->dispatch(method, params);
}

void MemoryGovernorTest::memoryUsageDefaultsToSixty()
{
    QCOMPARE(pictura::memory_usage_percent(), 60);
    window_->showPreferences(pictura::PreferencesDialog::kPerformance);
    auto* spin = window_->findChild<QSpinBox*>(QStringLiteral("preferencesMemoryUsage"));
    QVERIFY(spin);
    QCOMPARE(spin->value(), 60);
    QCOMPARE(spin->minimum(), 5);
    QCOMPARE(spin->maximum(), 100);
    window_->preferencesDialog()->hide();
}

void MemoryGovernorTest::memoryUsageChangeAppliesAndPersists()
{
    window_->showPreferences(pictura::PreferencesDialog::kPerformance);
    auto* spin = window_->findChild<QSpinBox*>(QStringLiteral("preferencesMemoryUsage"));
    QVERIFY(spin);
    spin->setValue(45);
    QCOMPARE(pictura::memory_usage_percent(), 45);
    QCOMPARE(pictura::loadSession().memoryUsagePercent, 45);
    spin->setValue(60);
    window_->preferencesDialog()->hide();
}

void MemoryGovernorTest::everyPathRefusesWithoutRoom()
{
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    // A history to undo and redo, a snapshot to restore, and a file to open.
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("Before")));
    const QString written = files_.filePath(QStringLiteral("memory-written.psd"));
    QVERIFY(window_->saveActiveAs(written));
    // A path no tab holds, so the open decodes instead of activating a tab.
    const QString psd = files_.filePath(QStringLiteral("memory-open.psd"));
    QVERIFY(QFile::copy(written, psd));
    QVERIFY(pictura::history_api::undo(*view));
    const int documents = window_->documentCount();
    const int layers = view->layer_count();
    const int index = view->history_index();
    const int snapshots = pictura::history_api::snapshotCount(*view);
    const bool dirty = pictura::document_api::isDirty(*view);

    pinNoRoom();
    QVERIFY(!window_->newDocument(QStringLiteral("Refused"), 40, 40, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white")));
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(view->add_layer(-1) < 0);
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!view->begin_paint(0xff000000u, 0xffffffffu, 10, 100, 100, 0, 100, 100, 25,
                               QStringLiteral("normal"), false, false));
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!pictura::history_api::undo(*view));
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!pictura::history_api::redo(*view));
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!pictura::history_api::addSnapshot(*view, QStringLiteral("Refused")));
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!pictura::history_api::restoreSnapshot(*view, 0));
    QVERIFY(pictura::edit_refused_for_memory());
    const QString saved = files_.filePath(QStringLiteral("memory-refused.psd"));
    QVERIFY(!window_->saveActiveAs(saved));
    QVERIFY(pictura::edit_refused_for_memory());
    QVERIFY(!QFileInfo::exists(saved));
    QVERIFY(window_->openDocumentAtPathOutcome(psd, false)
            != pictura::PicturaMainWindow::OpenOutcome::Opened);
    QVERIFY(pictura::open_refused_for_memory());

    QCOMPARE(window_->documentCount(), documents);
    QCOMPARE(window_->activeView(), view);
    QCOMPARE(view->layer_count(), layers);
    QCOMPARE(view->history_index(), index);
    QCOMPARE(pictura::history_api::snapshotCount(*view), snapshots);
    QCOMPARE(pictura::document_api::isDirty(*view), dirty);

    // With room again, each runs.
    unpin();
    QVERIFY(pictura::history_api::redo(*view));
    QVERIFY(pictura::history_api::undo(*view));
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("After")));
    QVERIFY(pictura::history_api::restoreSnapshot(*view, 0));
    QVERIFY(window_->saveActiveAs(saved));
    QVERIFY(QFileInfo::exists(saved));
    QVERIFY(window_->openDocumentAtPathOutcome(psd, false)
            == pictura::PicturaMainWindow::OpenOutcome::Opened);
    QVERIFY(!pictura::open_refused_for_memory());
    QVERIFY(newDocument());
    QVERIFY(!pictura::edit_refused_for_memory());
}

void MemoryGovernorTest::refusalsShowTheStatusMessage()
{
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    QVERIFY(view->add_layer(-1) >= 0);
    QStatusBar* bar = window_->statusBar();
    pinNoRoom();

    bar->clearMessage();
    auto* newLayer = window_->findChild<QToolButton*>(QStringLiteral("layersStripNewLayer"));
    QVERIFY(newLayer);
    newLayer->click();
    QCOMPARE(bar->currentMessage(), pictura::memoryRefusalMessage(QStringLiteral("add a layer")));

    bar->clearMessage();
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditUndo)));
    QCOMPARE(bar->currentMessage(), pictura::memoryRefusalMessage(QStringLiteral("undo")));

    bar->clearMessage();
    QVERIFY(!window_->newDocument(QStringLiteral("Refused"), 40, 40, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white")));
    QCOMPARE(bar->currentMessage(),
             pictura::memoryRefusalMessage(QStringLiteral("create the document")));
}

void MemoryGovernorTest::controlAnswersRefused()
{
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    const int documents = window_->documentCount();
    const int layers = view->layer_count();
    pinNoRoom();
    const auto refused = [](const QJsonObject& response) {
        return !response.value(QStringLiteral("ok")).toBool()
            && response.value(QStringLiteral("error")).toObject().value(QStringLiteral("code"))
                   == QStringLiteral("refused");
    };
    QVERIFY(refused(call(QStringLiteral("document"), {{QStringLiteral("op"), QStringLiteral("new")},
                                                      {QStringLiteral("width"), 64},
                                                      {QStringLiteral("height"), 64}})));
    QVERIFY(refused(call(QStringLiteral("layer_op"),
                         {{QStringLiteral("op"), QStringLiteral("add")},
                          {QStringLiteral("index"), 0}})));
    QCOMPARE(window_->documentCount(), documents);
    QCOMPARE(view->layer_count(), layers);
}

void MemoryGovernorTest::statusReportsTheGovernor()
{
    const auto memory = [this] {
        const QJsonObject response = call(QStringLiteral("status"));
        return response.value(QStringLiteral("result"))
            .toObject()
            .value(QStringLiteral("memory"))
            .toObject();
    };
    QJsonObject status = memory();
    for (const char* key : {"ceiling", "used", "reserved", "room", "pressure", "usage_percent"}) {
        QVERIFY2(status.contains(QLatin1String(key)), key);
    }
    QCOMPARE(status.value(QStringLiteral("usage_percent")).toInt(), 60);
    pictura::set_memory_usage_percent(45);
    status = memory();
    QCOMPARE(status.value(QStringLiteral("usage_percent")).toInt(), 45);
}

void MemoryGovernorTest::statusCarriesScratchReadouts()
{
    const QString volume = files_.filePath(QStringLiteral("scratch-a"));
    pictura::set_scratch_volumes(volume);
    const QJsonObject memory = call(QStringLiteral("status"))
                                   .value(QStringLiteral("result"))
                                   .toObject()
                                   .value(QStringLiteral("memory"))
                                   .toObject();
    const QJsonObject scratch = memory.value(QStringLiteral("scratch")).toObject();
    const QJsonArray volumes = scratch.value(QStringLiteral("volumes")).toArray();
    QCOMPARE(volumes.size(), 1);
    QCOMPARE(volumes.at(0).toObject().value(QStringLiteral("path")).toString(), volume);
    QVERIFY(scratch.contains(QStringLiteral("used")));
    QVERIFY(scratch.contains(QStringLiteral("available")));
    const double percent = memory.value(QStringLiteral("efficiency"))
                               .toObject()
                               .value(QStringLiteral("percent"))
                               .toDouble(-1);
    QVERIFY(percent >= 0 && percent <= 100);
    const QJsonObject tiles = memory.value(QStringLiteral("tiles")).toObject();
    for (const char* key : {"packed", "spilled", "unreadable"}) {
        QVERIFY2(tiles.contains(QLatin1String(key)), key);
    }
    pictura::set_scratch_volumes(QString());
}

void MemoryGovernorTest::aFailedAllocationLeavesTheEditUndone()
{
    const int documents = window_->documentCount();
    pictura::fail_next_plane_for_test();
    QVERIFY(!window_->newDocument(QStringLiteral("Failed"), 40, 40, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white")));
    QVERIFY(pictura::edit_refused_for_memory());
    QCOMPARE(window_->documentCount(), documents);
    // The Paint Bucket was the other edit with a canvas-sized allocation to
    // fail; its flood is tiled since #1214 and allocates none.
}

// [oct_gate_scratch_full]
void MemoryGovernorTest::transformRefusalNamesFullScratchDisks()
{
    QVERIFY(window_->newDocument(QStringLiteral("Scratch"), 2000, 2000, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    QCOMPARE(view->duplicate_layer(0), 1);
    view->set_active_layer(QStringLiteral("1"));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    QVERIFY(tools->beginFreeTransform(QStringLiteral("1")));
    const QPolygonF quad = pictura::transform_api::quad(*view).corners;
    QCOMPARE(quad.size(), 4);
    // Drag the corner out to about 14000²: over a gigabyte of output tiles.
    const QPointF corner = quad.at(2);
    QVERIFY(view->transform_press(corner.x(), corner.y(), 1.0, false, false) >= 0);
    QVERIFY(view->transform_move(corner.x() + 12000.0, corner.y() + 12000.0, 1.0, false, false));
    view->transform_release();

    // Room for the working set (one 8 MiB window a thread, plus the strips)
    // but not for the output, with nothing free on the scratch disks.
    const qint64 mib = 1024 * 1024;
    pictura::set_available_memory_for_test(256 * mib + QThread::idealThreadCount() * 8 * mib +
                                           128 * mib);
    pictura::set_scratch_available_for_test(0);
    const int count = view->history_count();
    tools->commitFreeTransform();
    pictura::set_scratch_available_for_test(-1);

    QVERIFY(pictura::edit_refused_for_scratch_full());
    QCOMPARE(view->history_count(), count);
    const QString message = window_->statusBar()->currentMessage();
    QCOMPARE(message, pictura::transformMemoryRefusalMessage());
    QVERIFY2(message.contains(QStringLiteral("scratch disks")), qPrintable(message));
}

pictura::PictureView* MemoryGovernorTest::largeMoveDocument()
{
    if (!window_->newDocument(QStringLiteral("Move"), 4100, 4100, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    pictura::PictureView* view = window_->activeView();
    if (!view || view->duplicate_layer(0) != 1) {
        return nullptr;
    }
    view->set_active_layer(QStringLiteral("1"));
    return view;
}

bool MemoryGovernorTest::commitMoveWithRoom(pictura::PictureView* view, qint64 roomMiB)
{
    const qint64 mib = 1024 * 1024;
    pictura::set_available_memory_for_test((256 + roomMiB) * mib);
    pictura::set_scratch_available_for_test(0);
    const bool moved = view->commit_move(5, 5);
    pictura::set_scratch_available_for_test(-1);
    unpin();
    return moved;
}

// [oct_move_commits]
void MemoryGovernorTest::moveChargesOnlyTheTiledComposite()
{
    pictura::PictureView* view = largeMoveDocument();
    QVERIFY(view);
    const int count = view->history_count();
    QSignalSpy committed(view, &pictura::PictureView::moveCommitted);
    // 128 MiB holds the 4-byte composite but not the old 12-byte charge.
    QVERIFY(commitMoveWithRoom(view, 128));
    QVERIFY(!pictura::edit_refused_for_memory());
    // A commit this large lands its composite, and its history state, from a
    // worker.
    QVERIFY(committed.count() > 0 || committed.wait(30000));
    QCOMPARE(view->history_count(), count + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Move Layer"));
}

// [oct_move_scratch_full]
void MemoryGovernorTest::moveRefusalNamesFullScratchDisks()
{
    pictura::PictureView* view = largeMoveDocument();
    QVERIFY(view);
    const int count = view->history_count();
    // 32 MiB holds the composite's strips (about 12 MiB) but not its tiles.
    QVERIFY(!commitMoveWithRoom(view, 32));
    QVERIFY(pictura::edit_refused_for_scratch_full());
    QCOMPARE(view->history_count(), count);
    const QString message = pictura::moveMemoryRefusalMessage();
    QVERIFY2(message.contains(QStringLiteral("scratch disks")), qPrintable(message));
}

pictura::PictureView* MemoryGovernorTest::largeFillDocument()
{
    if (!window_->newDocument(QStringLiteral("Fill"), 4100, 4100, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

void MemoryGovernorTest::pinRoom(qint64 roomMiB, qint64 scratchMiB)
{
    const qint64 mib = 1024 * 1024;
    // The governor keeps 256 MiB of headroom under the pinned available memory.
    pictura::set_available_memory_for_test((256 + roomMiB) * mib);
    pictura::set_scratch_available_for_test(scratchMiB * mib);
}

// [ocf_fills_commit]
void MemoryGovernorTest::fillsSpillToScratch()
{
    pictura::PictureView* view = largeFillDocument();
    QVERIFY(view);
    const QString normal = QStringLiteral("normal");
    // 96 MiB holds a flood's three masks (about 71 MiB across the canvas) but
    // not any fill's old charge: its forked tiles plus 6 bytes a pixel of
    // refresh (over 150 MB).
    const auto run = [&](const char* label, const std::function<bool()>& fill) {
        const int count = view->history_count();
        pinRoom(96, 4096);
        const bool applied = fill();
        const bool refused = pictura::edit_refused_for_memory();
        pictura::set_scratch_available_for_test(-1);
        unpin();
        QVERIFY2(applied, label);
        QVERIFY2(!refused, label);
        QCOMPARE(view->history_count(), count + 1);
        QCOMPARE(view->history_label(view->history_index()), QString::fromLatin1(label));
    };
    run("Gradient", [&] {
        return pictura::draw_gradient(*view, 0, 0xFF000000u, 0xFFFFFFFFu, 0, normal, 100, false,
                                      false, false, 0.0, 0.0, 4099.0, 4099.0);
    });
    run("Paint Bucket", [&] {
        return pictura::bucket_fill_at(*view, 4099, 4099, 0xFF204080u, -1, normal, 100, 0, false,
                                       true, false);
    });
    run("Fill", [&] {
        return pictura::edit_fill(*view, 0xFF808080u, -1, normal, 100, false);
    });
    QVERIFY(view->select_rect(8, 8, 4084, 4084, QStringLiteral("new"), 0.0));
    run("Stroke", [&] {
        return pictura::edit_stroke(*view, 0xFFFF0000u, 4, 1, normal, 100);
    });
}

// [ocf_scratch_full]
void MemoryGovernorTest::fillRefusalNamesFullScratchDisks()
{
    pictura::PictureView* view = largeFillDocument();
    QVERIFY(view);
    const int count = view->history_count();
    // The gradient holds nothing resident here, but its 50 MB of forked tiles
    // do not fit 16 MiB of room with no scratch.
    pinRoom(16, 0);
    const bool drawn =
        pictura::draw_gradient(*view, 0, 0xFF000000u, 0xFFFFFFFFu, 0, QStringLiteral("normal"),
                               100, false, false, false, 0.0, 0.0, 4099.0, 4099.0);
    const bool full = pictura::edit_refused_for_scratch_full();
    const QString message = pictura::fillMemoryRefusalMessage();
    pictura::set_scratch_available_for_test(-1);
    unpin();
    QVERIFY(!drawn);
    QVERIFY(full);
    QCOMPARE(view->history_count(), count);
    QVERIFY2(message.contains(QStringLiteral("scratch disks")), qPrintable(message));
}

QTEST_MAIN(MemoryGovernorTest)
#include "tst_memory_governor.moc"
