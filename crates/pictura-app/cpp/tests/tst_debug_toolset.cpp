#include <QtTest/QtTest>

#include <memory>

#include <unistd.h>

#include <QtCore/QSettings>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSizeGrip>
#include <QtWidgets/QToolButton>

#include "command_ids.h"
#include "commands.h"
#include "debug_toolset.h"
#include "debug_toolset_panel.h"
#include "frame.h"
#include "system_info.h"

#include "qt_test_support.h"

namespace {

const QString kCommand = QString::fromLatin1(pictura::command_ids::HelpDeveloperDebugToolset);

// Tick rate 100/s is assumed by the sampler only through sysconf; fixtures use
// whole seconds so the arithmetic below holds for any tick rate.
QString processStat(quint64 ticks)
{
    return QStringLiteral("42 (pictura) S 1 1 1 0 -1 0 0 0 0 0 %1 0 0 0 20 0 9 0 5 0 0")
        .arg(ticks);
}

QString taskStat(const QString& comm, quint64 ticks)
{
    return QStringLiteral("7 (%1) S 1 1 1 0 -1 0 0 0 0 0 %2 0 0 0 20 0 1 0 5 0 0").arg(comm).arg(ticks);
}

pictura::DebugReadings readings(qint64 seconds, quint64 busyTicks)
{
    pictura::DebugReadings r;
    r.pid = 42;
    r.monotonicNs = seconds * 1000000000LL;
    r.status = QStringLiteral("VmRSS:\t  2048 kB\nVmSwap:\t   512 kB\nThreads:\t4\n");
    r.processStat = processStat(busyTicks);
    r.memInfo = QStringLiteral("MemTotal: 16000 kB\nMemAvailable: 6000 kB\nCached: 3000 kB\n"
                               "SwapTotal: 4000 kB\nSwapFree: 3000 kB\n");
    r.swaps = QStringLiteral("Filename Type Size Used Priority\n/dev/zram0 partition 4000 1000 100\n"
                             "/swapfile file 100 0 -2\n");
    r.vmStat = QStringLiteral("pswpin %1\npswpout %2\n").arg(seconds * 10).arg(seconds * 20);
    r.cpuStat = QStringLiteral("cpu  100 0 100 800 0 0 0 0 0 0\n"
                               "cpu0 %1 0 0 %2 0 0 0 0 0 0\n"
                               "cpu1 0 0 0 %3 0 0 0 0 0 0\n")
                    .arg(seconds * 50)
                    .arg(seconds * 50)
                    .arg(seconds * 100);
    r.loadAvg = QStringLiteral("0.50 1.25 2.00 1/200 999\n");
    const double scale = static_cast<double>(::sysconf(_SC_CLK_TCK));
    const quint64 perSecond = static_cast<quint64>(scale);
    r.tasks = {
        {42, taskStat(QStringLiteral("pictura"), static_cast<quint64>(seconds) * perSecond / 10)},
        {43, taskStat(QStringLiteral("pictura-rayon-3"), static_cast<quint64>(seconds) * perSecond)},
        {44, taskStat(QStringLiteral("pictura-open"), static_cast<quint64>(seconds) * perSecond / 2)},
        {45, taskStat(QStringLiteral("QXcbEvent"), 0)},
    };
    return r;
}

} // namespace

class DebugToolsetTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();

    void menuEntryIsCheckableAndOffAtStartup();
    void stateIsNotPersisted();
    void disabledRunsNothing();
    void toggleShowsAndHidesPanel();
    void headerDragMovesPanelOutsideWindow();
    void panelIsAFloatingToolWindow();
    void panelIsLightYellowWithDarkText();
    void rowsSitInTitledGroupTables();
    void closeButtonStopsCollection();
    void dragIconIsCentredAtTop();
    void sizeGripResizesFromBottomRight();
    void historyChartsSitUnderBusiest();
    void heatmapColoursRunColdToHot();
    void heatmapUsesOneOrTwoRows();
    void panelFeedsHeatmapFromSampler();
    void parsesActivityRows();
    void panelShowsActivityRows();
    void jobsComeFromNamedThreads();
    void parsesMemoryBudgetReport();
    void budgetRowsShowTheReport();
    void parsesDrmClientInformation();
    void parsesSystemGpuSysfs();
    void timingTextHasPercentiles();
    void groupChartGrowsAndClears();
    void headerReadsPicturaMetrics();
    void tablesSitInTwoColumns();
    void refreshIntervalIsTwoSeconds();
    void rateRowsWaitForSecondSample();
    void parsesProcessRows();
    void gpuRowReportsMeasuredOrNot();
    void threadsGroupByNameAndRank();
    void parsesMemoryAndSwap();
    void parsesCpuAndLoad();
    void platformRowsArePresent();
    void versionRowIsPlaceholder();

private:
    pictura::DebugToolsetPanel* panel() const
    {
        return window_->findChild<pictura::DebugToolsetPanel*>();
    }
    void toggle() { QVERIFY(window_->registry()->dispatch(kCommand)); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void DebugToolsetTest::init()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1000, 700);
    window_->show();
    QCoreApplication::processEvents();
}

void DebugToolsetTest::cleanup()
{
    window_.reset();
}

void DebugToolsetTest::menuEntryIsCheckableAndOffAtStartup()
{
    auto* registry = window_->registry();
    QVERIFY(registry != nullptr);
    bool found = false;
    for (const pictura::CommandInfo& info : registry->describe()) {
        if (info.id == kCommand) {
            found = true;
            QVERIFY(info.implemented);
            QVERIFY(info.enabled);
            QVERIFY(!info.checked);
            QCOMPARE(info.path, (QStringList{"Help", "Developer", "Pictura Metrics"}));
        }
    }
    QVERIFY(found);
    QVERIFY(registry->action(kCommand)->isCheckable());
    QCOMPARE(registry->action(kCommand)->text().remove(QLatin1Char('&')), QStringLiteral("Pictura Metrics"));
    QVERIFY(panel() == nullptr || !panel()->isVisible());
}

void DebugToolsetTest::stateIsNotPersisted()
{
    toggle();
    QVERIFY(panel() && panel()->isVisible());
    panel()->move(10, 10);
    const QStringList keysBefore = QSettings().allKeys();
    window_.reset();
    window_ = pictura::test::makeMainWindow();
    window_->resize(1000, 700);
    window_->show();
    QVERIFY(panel() == nullptr);
    QVERIFY(!window_->registry()->action(kCommand)->isChecked());
    for (const QString& key : QSettings().allKeys()) {
        QVERIFY2(!key.contains(QStringLiteral("debug"), Qt::CaseInsensitive), qPrintable(key));
    }
    QCOMPARE(QSettings().allKeys(), keysBefore);
}

void DebugToolsetTest::disabledRunsNothing()
{
    QVERIFY(panel() == nullptr);
    toggle();
    QVERIFY(panel()->isActive());
    QVERIFY(panel()->holdsSample());
    toggle();
    QVERIFY(!panel()->isActive());
    QVERIFY(!panel()->holdsSample());
}

void DebugToolsetTest::toggleShowsAndHidesPanel()
{
    toggle();
    QVERIFY(panel()->isVisible());
    QVERIFY(window_->debugToolsetEnabled());
    toggle();
    QVERIFY(!panel()->isVisible());
    QVERIFY(!window_->debugToolsetEnabled());
}

void DebugToolsetTest::headerDragMovesPanelOutsideWindow()
{
    toggle();
    auto* p = panel();
    QWidget* header = p->header();
    const QPoint start = p->pos();
    QTest::mousePress(header, Qt::LeftButton, Qt::NoModifier, QPoint(4, 4));
    QTest::mouseMove(header, QPoint(-1500, 4));
    QTest::mouseRelease(header, Qt::LeftButton, Qt::NoModifier, QPoint(-1500, 4));
    QVERIFY(p->pos().x() < start.x() - 1000);
    QVERIFY(!window_->frameGeometry().contains(p->frameGeometry()));
    QVERIFY(!window_->frameGeometry().intersects(p->frameGeometry()));
}

void DebugToolsetTest::panelIsAFloatingToolWindow()
{
    toggle();
    QVERIFY(panel()->isWindow());
    QVERIFY(panel()->windowFlags() & Qt::Tool);
    QVERIFY(panel()->parentWidget() == window_.get());
}

void DebugToolsetTest::panelIsLightYellowWithDarkText()
{
    toggle();
    const QImage image = panel()->grab().toImage();
    const QColor background = image.pixelColor(4, image.height() / 2);
    QVERIFY2(background.red() >= 0xF0 && background.green() >= 0xE6 && background.blue() <= 0xD8
                 && background.red() - background.blue() >= 40,
             qPrintable(background.name()));
    const QLabel* value = panel()->findChild<QLabel*>(QStringLiteral("debugToolsetRow_ram"));
    QVERIFY(value != nullptr);
    QVERIFY(value->palette().color(QPalette::WindowText).lightness() < 90);
}

void DebugToolsetTest::rowsSitInTitledGroupTables()
{
    toggle();
    auto* p = panel();
    const QList<QPair<QString, QString>> titles = {
        {QStringLiteral("usage"), QStringLiteral("Pictura Resource Usage")},
        {QStringLiteral("platform"), QStringLiteral("Platform")},
        {QStringLiteral("version"), QStringLiteral("Version")},
    };
    for (const auto& [group, title] : titles) {
        const QLabel* heading =
            p->findChild<QLabel*>(QStringLiteral("debugToolsetGroupTitle_") + group);
        QVERIFY2(heading != nullptr, qPrintable(group));
        QCOMPARE(heading->text(), title);
    }
    const QList<QPair<QString, QString>> rows = {
        {QStringLiteral("ram"), QStringLiteral("usage")},
        {QStringLiteral("gpu"), QStringLiteral("usage")},
        {QStringLiteral("busiest"), QStringLiteral("usage")},
        {QStringLiteral("os"), QStringLiteral("platform")},
        {QStringLiteral("load"), QStringLiteral("platform")},
        {QStringLiteral("build"), QStringLiteral("version")},
    };
    for (const auto& [key, group] : rows) {
        const QLabel* value = p->findChild<QLabel*>(QStringLiteral("debugToolsetRow_") + key);
        QVERIFY2(value != nullptr, qPrintable(key));
        QCOMPARE(value->parentWidget()->objectName(), QStringLiteral("debugToolsetGroup_") + group);
        auto* grid = qobject_cast<QGridLayout*>(value->parentWidget()->layout());
        QVERIFY(grid != nullptr);
        int row = 0;
        int column = 0;
        int rowSpan = 0;
        int columnSpan = 0;
        grid->getItemPosition(grid->indexOf(value), &row, &column, &rowSpan, &columnSpan);
        QCOMPARE(column, 1);
        const QLayoutItem* name = grid->itemAtPosition(row, 0);
        QVERIFY(name != nullptr);
        const auto* nameLabel = qobject_cast<QLabel*>(name->widget());
        QVERIFY(nameLabel != nullptr && !nameLabel->text().isEmpty());
    }
}

void DebugToolsetTest::closeButtonStopsCollection()
{
    toggle();
    auto* p = panel();
    p->refreshNow();
    QVERIFY(window_->registry()->action(kCommand)->isChecked());
    QVERIFY(!p->history(QStringLiteral("ram")).isEmpty());
    auto* close = p->findChild<QToolButton*>(QStringLiteral("debugToolsetClose"));
    QVERIFY(close != nullptr);
    QTest::mouseClick(close, Qt::LeftButton);
    QVERIFY(!p->isVisible());
    QVERIFY(!p->isActive());
    QVERIFY(!p->holdsSample());
    QVERIFY(p->history(QStringLiteral("ram")).isEmpty());
    QVERIFY(p->history(QStringLiteral("cpu")).isEmpty());
    QVERIFY(!window_->debugToolsetEnabled());
    QVERIFY(!window_->registry()->action(kCommand)->isChecked());
}

void DebugToolsetTest::dragIconIsCentredAtTop()
{
    toggle();
    auto* p = panel();
    p->layout()->activate();
    const QWidget* icon = p->findChild<QWidget*>(QStringLiteral("debugToolsetDragIcon"));
    QVERIFY(icon != nullptr);
    QVERIFY(icon->isVisible());
    const QPoint centre = icon->mapTo(p, icon->rect().center());
    QVERIFY2(qAbs(centre.x() - p->width() / 2) <= 6, qPrintable(QString::number(centre.x())));
    QVERIFY(centre.y() < p->header()->geometry().bottom());
    QVERIFY(centre.y() < 40);
}

void DebugToolsetTest::sizeGripResizesFromBottomRight()
{
    toggle();
    auto* p = panel();
    p->layout()->activate();
    const QSizeGrip* grip = p->findChild<QSizeGrip*>();
    QVERIFY(grip != nullptr);
    QVERIFY(grip->isVisible());
    const QPoint corner = grip->mapTo(p, grip->rect().bottomRight());
    QVERIFY(corner.x() >= p->width() - 12 && corner.y() >= p->height() - 12);
    const QSize bigger = p->size() + QSize(80, 60);
    p->resize(bigger);
    QCOMPARE(p->size(), bigger);
    const QPoint movedCorner = grip->mapTo(p, grip->rect().bottomRight());
    QVERIFY(movedCorner.x() >= p->width() - 12 && movedCorner.y() >= p->height() - 12);
}

void DebugToolsetTest::historyChartsSitUnderBusiest()
{
    toggle();
    auto* p = panel();
    p->refreshNow();
    p->refreshNow();
    p->layout()->activate();
    QCOMPARE(p->history(QStringLiteral("ram")).size(), 3);
    QCOMPARE(p->history(QStringLiteral("cpu")).size(), 2);
    const QWidget* busiest = p->findChild<QWidget*>(QStringLiteral("debugToolsetRow_busiest"));
    const QWidget* ram = p->findChild<QWidget*>(QStringLiteral("debugToolsetChart_ram"));
    const QWidget* cpu = p->findChild<QWidget*>(QStringLiteral("debugToolsetChart_cpu"));
    QVERIFY(busiest && ram && cpu);
    QVERIFY(ram->isVisible() && cpu->isVisible());
    const int busiestBottom = busiest->mapTo(p, busiest->rect().bottomLeft()).y();
    QVERIFY(ram->mapTo(p, QPoint(0, 0)).y() >= busiestBottom);
    QVERIFY(cpu->mapTo(p, QPoint(0, 0)).y() > ram->mapTo(p, QPoint(0, 0)).y());
    for (int i = 0; i < 70; ++i) {
        p->refreshNow();
    }
    QCOMPARE(p->history(QStringLiteral("ram")).size(), pictura::DebugToolsetPanel::kHistorySamples);
    QVERIFY(p->history(QStringLiteral("cpu")).size() <= pictura::DebugToolsetPanel::kHistorySamples);
}

void DebugToolsetTest::heatmapColoursRunColdToHot()
{
    pictura::CpuHeatmap heat;
    heat.setLoads({0.0, 50.0, 100.0});
    heat.resize(heat.sizeHint());
    const QImage image = heat.grab().toImage();
    const auto pixel = [&](int cell) {
        return image.pixelColor(heat.cellRect(cell).center());
    };
    const QColor cold = pixel(0);
    const QColor middle = pixel(1);
    const QColor hot = pixel(2);
    QVERIFY2(cold.blue() > cold.red() + 40, qPrintable(cold.name()));
    QVERIFY2(hot.red() > hot.blue() + 80, qPrintable(hot.name()));
    QVERIFY(middle != cold && middle != hot);
}

void DebugToolsetTest::heatmapUsesOneOrTwoRows()
{
    const QList<QPair<int, int>> cases = {{4, 1}, {8, 1}, {9, 2}, {16, 2}};
    for (const auto& [cpus, rows] : cases) {
        pictura::CpuHeatmap heat;
        heat.setLoads(QList<double>(cpus, 25.0));
        heat.resize(heat.sizeHint());
        QCOMPARE(heat.rowCount(), rows);
        for (int i = 0; i < cpus; ++i) {
            QVERIFY2(heat.rect().contains(heat.cellRect(i)), qPrintable(QString::number(i)));
        }
        QVERIFY(heat.cellRect(cpus - 1).top() > heat.cellRect(0).top() == (rows == 2));
    }
}

void DebugToolsetTest::panelFeedsHeatmapFromSampler()
{
    toggle();
    const pictura::CpuHeatmap* heat = panel()->cpuHeatmap();
    QVERIFY(heat != nullptr);
    QVERIFY(heat->loads().isEmpty());
    panel()->refreshNow();
    const int cpus = pictura::readDebugSources().cpuStat.isEmpty()
        ? 0
        : static_cast<int>(pictura::parsePerCpuTimes(pictura::readDebugSources().cpuStat).size());
    QVERIFY(cpus > 0);
    QCOMPARE(heat->loads().size(), cpus);
    toggle();
    QVERIFY(heat->loads().isEmpty());
}

void DebugToolsetTest::parsesActivityRows()
{
    pictura::DebugReadings before = readings(10, 0);
    pictura::DebugReadings after = readings(12, 0);
    before.status += QStringLiteral("RssAnon:\t1024 kB\nRssFile:\t256 kB\n");
    after.status = before.status;
    before.openFds = 7;
    after.openFds = 9;
    before.io = QStringLiteral("rchar: 0\nread_bytes: 1000\nwrite_bytes: 2000\n");
    after.io = QStringLiteral("rchar: 0\nread_bytes: 5000\nwrite_bytes: 6000\n");
    const auto stat = [](int minor, int major) {
        return QStringLiteral("42 (pictura) S 1 1 1 0 -1 0 %1 0 %2 0 0 0 0 0 20 0 9 0 5 0 0")
            .arg(minor)
            .arg(major);
    };
    before.processStat = stat(100, 10);
    after.processStat = stat(140, 14);
    const auto status = [](int voluntary, int involuntary) {
        return QStringLiteral("voluntary_ctxt_switches:\t%1\nnonvoluntary_ctxt_switches:\t%2\n")
            .arg(voluntary)
            .arg(involuntary);
    };
    before.tasks[0].status = status(100, 10);
    before.tasks[1].status = status(0, 0);
    after.tasks[0].status = status(300, 30);
    after.tasks[1].status = status(100, 10);

    pictura::DebugSampler sampler;
    const pictura::DebugSnapshot first = sampler.sample(before);
    QCOMPARE(first.rssAnon, quint64(1024) * 1024);
    QCOMPARE(first.rssFile, quint64(256) * 1024);
    QCOMPARE(first.openFds, 7);
    QVERIFY(!first.ioReadBytesPerSec && !first.minorFaultsPerSec && !first.ctxVoluntaryPerSec);
    const pictura::DebugSnapshot second = sampler.sample(after);
    QCOMPARE(second.openFds, 9);
    QVERIFY(second.ioReadBytesPerSec && second.ioWriteBytesPerSec);
    QVERIFY(qAbs(*second.ioReadBytesPerSec - 2000.0) < 0.01);
    QVERIFY(qAbs(*second.ioWriteBytesPerSec - 2000.0) < 0.01);
    QVERIFY(qAbs(*second.minorFaultsPerSec - 20.0) < 0.01);
    QVERIFY(qAbs(*second.majorFaultsPerSec - 2.0) < 0.01);
    QVERIFY(qAbs(*second.ctxVoluntaryPerSec - 150.0) < 0.01);
    QVERIFY(qAbs(*second.ctxInvoluntaryPerSec - 15.0) < 0.01);
}

void DebugToolsetTest::panelShowsActivityRows()
{
    toggle();
    panel()->refreshNow();
    for (const QString& key : {QStringLiteral("files"), QStringLiteral("disk"),
                               QStringLiteral("faults"), QStringLiteral("ctx"),
                               QStringLiteral("jobs"), QStringLiteral("ramSplit")}) {
        const QString text = panel()->rowText(key);
        QVERIFY2(!text.isEmpty() && !text.contains(QStringLiteral("…")), qPrintable(key + ": " + text));
    }
}

void DebugToolsetTest::jobsComeFromNamedThreads()
{
    const double ticks = static_cast<double>(::sysconf(_SC_CLK_TCK));
    const auto stat = [&](const QString& comm, double startSeconds) {
        return QStringLiteral("7 (%1) S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 %2 0 0")
            .arg(comm)
            .arg(static_cast<qint64>(startSeconds * ticks));
    };
    pictura::DebugReadings r = readings(10, 0);
    r.uptimeSeconds = 100.0;
    r.tasks = {{42, stat(QStringLiteral("pictura"), 0.0), {}},
               {43, stat(QStringLiteral("pictura-save"), 97.0), {}},
               {44, stat(QStringLiteral("pictura-open"), 88.0), {}}};
    pictura::DebugSampler sampler;
    const pictura::DebugSnapshot s = sampler.sample(r);
    QCOMPARE(s.jobs.size(), 2);
    QCOMPARE(s.jobs.at(0).kind, QStringLiteral("Open"));
    QVERIFY(qAbs(s.jobs.at(0).seconds - 12.0) < 0.05);
    QCOMPARE(s.jobs.at(1).kind, QStringLiteral("Save"));
    QVERIFY(qAbs(s.jobs.at(1).seconds - 3.0) < 0.05);
    r.tasks = {r.tasks.at(0)};
    pictura::DebugSampler idle;
    QVERIFY(idle.sample(r).jobs.isEmpty());
    toggle();
    QCOMPARE(panel()->rowText(QStringLiteral("jobs")), QStringLiteral("none"));
}

void DebugToolsetTest::parsesMemoryBudgetReport()
{
    const QString report = QStringLiteral(
        "{\"ceiling\":6442450944,\"used\":3221225472,\"reserved\":1024,\"room\":3221225472,"
        "\"pressure\":\"elevated\",\"usage_percent\":60,"
        "\"scratch\":{\"volumes\":[],\"used\":100,\"available\":900},"
        "\"efficiency\":{\"compute_ms\":1,\"io_ms\":1,\"percent\":50.0},"
        "\"tiles\":{\"packed\":5,\"packed_bytes\":4096,\"spilled\":2,\"backed\":1,\"unreadable\":0}}");
    const pictura::MemoryBudget budget = pictura::parseMemoryBudget(report);
    QVERIFY(budget.present);
    QCOMPARE(*budget.used, quint64(3221225472ull));
    QCOMPARE(*budget.ceiling, quint64(6442450944ull));
    QCOMPARE(budget.reserved, quint64(1024));
    QCOMPARE(budget.pressure, QStringLiteral("elevated"));
    QCOMPARE(budget.scratchUsed, quint64(100));
    QCOMPARE(budget.scratchAvailable, quint64(900));
    QCOMPARE(budget.packed, quint64(5));
    QCOMPARE(budget.packedBytes, quint64(4096));
    QCOMPARE(budget.spilled, quint64(2));
    QCOMPARE(budget.backed, quint64(1));
    QVERIFY(!pictura::parseMemoryBudget(QString()).present);
}

void DebugToolsetTest::budgetRowsShowTheReport()
{
    toggle();
    auto* p = panel();
    p->setMemoryProvider([] {
        return QStringLiteral(
            "{\"ceiling\":6442450944,\"used\":3221225472,\"reserved\":0,\"pressure\":\"elevated\","
            "\"scratch\":{\"used\":0,\"available\":0},\"tiles\":{\"packed\":0}}");
    });
    p->refreshNow();
    const QString governor = p->rowText(QStringLiteral("governor"));
    QVERIFY2(governor.contains(pictura::formatBytes(3221225472ull)), qPrintable(governor));
    QVERIFY2(governor.contains(pictura::formatBytes(6442450944ull)), qPrintable(governor));
    QVERIFY2(governor.contains(QStringLiteral("elevated")), qPrintable(governor));
    p->setMemoryProvider([] { return QString(); });
    p->refreshNow();
    QCOMPARE(p->rowText(QStringLiteral("governor")), QStringLiteral("n/a"));
    QCOMPARE(p->rowText(QStringLiteral("tiles")), QStringLiteral("n/a"));
}

void DebugToolsetTest::parsesDrmClientInformation()
{
    const auto client = [](int id, const QString& engine, quint64 vramKib, quint64 ns) {
        return QStringLiteral("drm-driver:\tamdgpu\ndrm-client-id:\t%1\ndrm-memory-vram:\t%2 KiB\n"
                              "drm-engine-%3:\t%4 ns\ndrm-engine-capacity-%3:\t1\n")
            .arg(id)
            .arg(vramKib)
            .arg(engine)
            .arg(ns);
    };
    pictura::DebugReadings before = readings(10, 0);
    before.drmFdinfo = {client(5, QStringLiteral("gfx"), 2048, 1000),
                        client(5, QStringLiteral("gfx"), 2048, 1000),
                        client(6, QStringLiteral("compute"), 1024, 1000)};
    pictura::DebugReadings after = readings(11, 0);
    after.drmFdinfo = {client(5, QStringLiteral("gfx"), 2048, 300001000),
                       client(5, QStringLiteral("gfx"), 2048, 300001000),
                       client(6, QStringLiteral("compute"), 1024, 200001000)};
    pictura::DebugSampler sampler;
    const pictura::DebugSnapshot first = sampler.sample(before);
    QVERIFY(first.gpu.present);
    QCOMPARE(first.gpu.vramBytes, quint64(2048 + 1024) * 1024);
    QVERIFY(!first.gpu.enginePercent);
    const pictura::DebugSnapshot second = sampler.sample(after);
    QCOMPARE(second.gpu.vramBytes, quint64(2048 + 1024) * 1024);
    QVERIFY(second.gpu.enginePercent);
    QVERIFY(qAbs(*second.gpu.enginePercent - 50.0) < 0.01);
    pictura::DebugSampler none;
    QVERIFY(!none.sample(readings(10, 0)).gpu.present);
}

void DebugToolsetTest::parsesSystemGpuSysfs()
{
    pictura::DebugReadings r = readings(10, 0);
    r.gpuBusy = QStringLiteral("34\n");
    r.vramUsed = QStringLiteral("1000\n");
    r.vramTotal = QStringLiteral("8000\n");
    pictura::DebugSampler sampler;
    const pictura::DebugSnapshot s = sampler.sample(r);
    QVERIFY(s.systemGpu.present);
    QVERIFY(s.systemGpu.busyPercent && qAbs(*s.systemGpu.busyPercent - 34.0) < 0.01);
    QCOMPARE(s.systemGpu.vramUsed, quint64(1000));
    QCOMPARE(s.systemGpu.vramTotal, quint64(8000));
    pictura::DebugSampler empty;
    const pictura::DebugSnapshot none = empty.sample(readings(10, 0));
    QVERIFY(!none.systemGpu.present);
    QVERIFY(!none.systemGpu.busyPercent);
}

void DebugToolsetTest::timingTextHasPercentiles()
{
    const QString json = QStringLiteral(
        "{\"edits\":[{\"wall_ms\":10},{\"wall_ms\":20},{\"wall_ms\":30},{\"wall_ms\":40}],"
        "\"frames\":[{\"paint_ms\":5.0},{\"paint_ms\":7.0}]}");
    QCOMPARE(pictura::timingText(json, true),
             QStringLiteral("p50 25.0 ms   p95 38.5 ms   last 40.0 ms (4)"));
    QCOMPARE(pictura::timingText(json, false),
             QStringLiteral("p50 6.0 ms   p95 6.9 ms   last 7.0 ms (2)"));
    QCOMPARE(pictura::timingText(QString(), true), QStringLiteral("not measured"));
    QCOMPARE(pictura::timingText(QStringLiteral("{\"edits\":[],\"frames\":[]}"), false),
             QStringLiteral("not measured"));
}

void DebugToolsetTest::groupChartGrowsAndClears()
{
    toggle();
    auto* p = panel();
    p->refreshNow();
    p->refreshNow();
    QCOMPARE(p->groupHistory(QStringLiteral("GUI")).size(), 2);
    QCOMPARE(p->groupHistory(QStringLiteral("Rayon workers")).size(), 2);
    QVERIFY(p->findChild<QWidget*>(QStringLiteral("debugToolsetChart_groups"))->isVisible());
    toggle();
    QVERIFY(p->groupHistory(QStringLiteral("GUI")).isEmpty());
}

void DebugToolsetTest::headerReadsPicturaMetrics()
{
    toggle();
    const QLabel* title = panel()->findChild<QLabel*>(QStringLiteral("debugToolsetTitle"));
    QVERIFY(title != nullptr);
    QCOMPARE(title->text(), QStringLiteral("Pictura Metrics"));
}

void DebugToolsetTest::tablesSitInTwoColumns()
{
    toggle();
    auto* p = panel();
    p->layout()->activate();
    const auto group = [p](const QString& key) {
        QWidget* widget = p->findChild<QWidget*>(QStringLiteral("debugToolsetGroup_") + key);
        return widget;
    };
    const QWidget* usage = group(QStringLiteral("usage"));
    const QWidget* version = group(QStringLiteral("version"));
    QVERIFY(usage && version);
    const int usageRight = usage->mapTo(p, QPoint(usage->width(), 0)).x();
    int stacked = 0;
    for (const QString& key : {QStringLiteral("usage"), QStringLiteral("version")}) {
        stacked += group(key)->height();
    }
    for (const QString& key : {QStringLiteral("budget"), QStringLiteral("activity"),
                               QStringLiteral("platform")}) {
        const QWidget* widget = group(key);
        QVERIFY2(widget != nullptr, qPrintable(key));
        QVERIFY2(widget->mapTo(p, QPoint(0, 0)).x() >= usageRight, qPrintable(key));
        stacked += widget->height();
    }
    QVERIFY(version->mapTo(p, QPoint(0, 0)).x() < usageRight);
    QVERIFY2(p->height() < stacked, qPrintable(QString::number(p->height()) + " vs " + QString::number(stacked)));
}

void DebugToolsetTest::refreshIntervalIsTwoSeconds()
{
    toggle();
    QCOMPARE(panel()->refreshIntervalMs(), 2000);
    QCOMPARE(pictura::DebugToolsetPanel::kRefreshMs, 2000);
    panel()->refreshNow();
    for (const QString& key : {QStringLiteral("ram"), QStringLiteral("threads"),
                               QStringLiteral("memory"), QStringLiteral("load")}) {
        QVERIFY2(panel()->rowText(key) != QStringLiteral("…"), qPrintable(key));
    }
}

void DebugToolsetTest::rateRowsWaitForSecondSample()
{
    toggle();
    QCOMPARE(panel()->rowText(QStringLiteral("cpu")), QStringLiteral("…"));
    QVERIFY(panel()->cpuHeatmap()->loads().isEmpty());
    panel()->refreshNow();
    QVERIFY(panel()->rowText(QStringLiteral("cpu")).endsWith(QStringLiteral("%")));
    QVERIFY(!panel()->cpuHeatmap()->loads().isEmpty());
}

void DebugToolsetTest::parsesProcessRows()
{
    pictura::DebugSampler sampler;
    const pictura::DebugSnapshot first = sampler.sample(readings(10, 100));
    QCOMPARE(first.processRss, quint64(2048) * 1024);
    QCOMPARE(first.processSwap, quint64(512) * 1024);
    QCOMPARE(first.processThreads, 4);
    QVERIFY(!first.processCpu);
    const double perSecond = static_cast<double>(::sysconf(_SC_CLK_TCK));
    const pictura::DebugSnapshot second =
        sampler.sample(readings(12, 100 + static_cast<quint64>(perSecond)));
    QVERIFY(second.processCpu);
    QVERIFY(qAbs(*second.processCpu - 50.0) < 0.01);
}

void DebugToolsetTest::gpuRowReportsMeasuredOrNot()
{
    QCOMPARE(pictura::gpuRowText(QString()), QStringLiteral("not measured"));
    QCOMPARE(pictura::gpuRowText(QStringLiteral("{\"edits\":[],\"frames\":[{\"paint_ms\":1.0}]}")),
             QStringLiteral("not measured"));
    QCOMPARE(pictura::gpuRowText(QStringLiteral(
                 "{\"edits\":[],\"frames\":[{\"gpu_composite_ms\":1.5},{\"gpu_composite_ms\":2.25}]}")),
             QStringLiteral("2.25 ms (last frame)"));
    QCOMPARE(pictura::gpuRowText(QStringLiteral("{\"edits\":[{\"gpu_ms\":3.0}],\"frames\":[]}")),
             QStringLiteral("3.00 ms (last edit)"));
}

void DebugToolsetTest::threadsGroupByNameAndRank()
{
    QCOMPARE(pictura::threadGroupName(QStringLiteral("pictura"), true), QStringLiteral("GUI"));
    QCOMPARE(pictura::threadGroupName(QStringLiteral("pictura-rayon-3"), false),
             QStringLiteral("Rayon workers"));
    QCOMPARE(pictura::threadGroupName(QStringLiteral("pictura-open"), false), QStringLiteral("Jobs"));
    QCOMPARE(pictura::threadGroupName(QStringLiteral("QXcbEvent"), false), QStringLiteral("Other"));
    QCOMPARE(pictura::parseTaskComm(QStringLiteral("7 (a (b) c) S 1")), QStringLiteral("a (b) c"));

    pictura::DebugSampler sampler;
    sampler.sample(readings(0, 0));
    const pictura::DebugSnapshot s = sampler.sample(readings(10, 0));
    QVERIFY(s.groups);
    QCOMPARE(s.groups->size(), 4);
    QCOMPARE(s.groups->at(0).group, QStringLiteral("Rayon workers"));
    QCOMPARE(s.groups->at(1).group, QStringLiteral("Jobs"));
    QCOMPARE(s.groups->at(2).group, QStringLiteral("GUI"));
    QCOMPARE(s.groups->at(3).group, QStringLiteral("Other"));
    QVERIFY(qAbs(s.groups->at(0).cpuPercent - 100.0) < 0.01);
    QVERIFY(qAbs(s.groups->at(1).cpuPercent - 50.0) < 0.01);
    QVERIFY(qAbs(s.groups->at(2).cpuPercent - 10.0) < 0.01);
}

void DebugToolsetTest::parsesMemoryAndSwap()
{
    pictura::DebugSampler sampler;
    sampler.sample(readings(0, 0));
    const pictura::DebugSnapshot s = sampler.sample(readings(10, 0));
    QCOMPARE(s.memTotal, quint64(16000) * 1024);
    QCOMPARE(s.memUsed, quint64(10000) * 1024);
    QCOMPARE(s.memCached, quint64(3000) * 1024);
    QCOMPARE(s.swapTotal, quint64(4000) * 1024);
    QCOMPARE(s.swapUsed, quint64(1000) * 1024);
    QCOMPARE(s.swapLocation, QStringLiteral("/dev/zram0, /swapfile"));
    const double page = static_cast<double>(::sysconf(_SC_PAGESIZE));
    QVERIFY(s.swapInBytesPerSec && s.swapOutBytesPerSec);
    QVERIFY(qAbs(*s.swapInBytesPerSec - 10.0 * page) < 1.0);
    QVERIFY(qAbs(*s.swapOutBytesPerSec - 20.0 * page) < 1.0);
}

void DebugToolsetTest::parsesCpuAndLoad()
{
    pictura::DebugSampler sampler;
    sampler.sample(readings(0, 0));
    const pictura::DebugSnapshot s = sampler.sample(readings(10, 0));
    QVERIFY(s.cpuPercents);
    QCOMPARE(s.cpuPercents->size(), 2);
    QVERIFY(qAbs(s.cpuPercents->at(0) - 50.0) < 0.01);
    QVERIFY(qAbs(s.cpuPercents->at(1) - 0.0) < 0.01);
    QCOMPARE(s.load1, 0.5);
    QCOMPARE(s.load5, 1.25);
    QCOMPARE(s.load15, 2.0);
}

void DebugToolsetTest::platformRowsArePresent()
{
    toggle();
    for (const QString& key : {QStringLiteral("os"), QStringLiteral("architecture"),
                               QStringLiteral("desktop"), QStringLiteral("session")}) {
        QVERIFY2(!panel()->rowText(key).isEmpty(), qPrintable(key));
        QVERIFY2(panel()->rowText(key) != QStringLiteral("…"), qPrintable(key));
    }
}

void DebugToolsetTest::versionRowIsPlaceholder()
{
    toggle();
    QCOMPARE(panel()->rowText(QStringLiteral("build")), QStringLiteral("n/a (placeholder)"));
}

PICTURA_QT_TEST_MAIN(DebugToolsetTest)
#include "tst_debug_toolset.moc"
