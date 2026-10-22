#include <QtTest/QtTest>

#include <memory>

#include <QtCore/QCoreApplication>
#include <QtCore/QSysInfo>
#include <QtCore/QUrlQuery>
#include <QtGui/QAction>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>

#include "commands.h"
#include "frame.h"
#include "system_info.h"
#include "system_info_dialog.h"

#include "qt_test_support.h"

class SystemInfoTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void reportHasAllSections();
    void reportHasGpuFields();
    void reportUsesUnavailable();
    void dialogCopiesReport();
    void reopenRefreshesReport();
    void submitUrlEncodesReport();
    void submitUrlCapsLongReport();
    void parsesCpuInfo();
    void parsesMemInfo();
    void formatsBytes();
    void collectorOmitsHostAndUser();
    void helpMenuWiresSystemInfo();
    void helpMenuWiresSubmitIssue();

private:
    static pictura::SystemInfo fixture();
    static pictura::CommandInfo command(const pictura::CommandRegistry& registry,
                                        const QString& id);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

pictura::SystemInfo SystemInfoTest::fixture()
{
    pictura::SystemInfo info;
    info.appVersion = QStringLiteral("0.2.0-test");
    info.buildType = QStringLiteral("Debug-test");
    info.qtVersion = QStringLiteral("6.11.1-test");
    info.osType = QStringLiteral("Linux-test");
    info.osVersion = QStringLiteral("TestOS 1");
    info.kernel = QStringLiteral("test-kernel");
    info.architecture = QStringLiteral("x86_64-test");
    info.desktop = QStringLiteral("GNOME-test");
    info.session = QStringLiteral("wayland-test");
    info.cpuModel = QStringLiteral("Test CPU");
    info.cpuPhysicalCores = QStringLiteral("8");
    info.cpuLogicalCores = QStringLiteral("16");
    info.cpuClock = QStringLiteral("3.60 GHz");
    info.gpuName = QStringLiteral("Test GPU");
    info.gpuType = QStringLiteral("Discrete");
    info.gpuBackend = QStringLiteral("Vulkan");
    info.gpuDriver = QStringLiteral("Test Driver 1.2");
    info.gpuVram = QStringLiteral("Unavailable");
    info.memoryTotal = QStringLiteral("32.0 GB");
    info.memoryAvailable = QStringLiteral("16.0 GB");
    info.swapTotal = QStringLiteral("8.0 GB");
    info.swapAvailable = QStringLiteral("8.0 GB");
    info.systemCpu = QStringLiteral("12.3 %");
    info.processMemory = QStringLiteral("245.0 MB");
    info.processCpu = QStringLiteral("1.2 %");
    info.processCpuAverage = QStringLiteral("0.8 %");
    pictura::MonitorInfo monitor;
    monitor.name = QStringLiteral("DP-1");
    monitor.manufacturer = QStringLiteral("Test Corp");
    monitor.model = QStringLiteral("Monitor");
    monitor.resolution = QStringLiteral("1920 x 1080");
    monitor.physicalSize = QStringLiteral("24.0 in");
    monitor.dpi = QStringLiteral("96 DPI");
    monitor.refreshRate = QStringLiteral("60 Hz");
    info.monitors.append(monitor);
    return info;
}

pictura::CommandInfo SystemInfoTest::command(const pictura::CommandRegistry& registry,
                                             const QString& id)
{
    for (const pictura::CommandInfo& info : registry.describe()) {
        if (info.id == id) {
            return info;
        }
    }
    return {};
}

void SystemInfoTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1000, 700);
    window_->show();
    QCoreApplication::processEvents();
}

void SystemInfoTest::reportHasAllSections()
{
    const QString report = pictura::formatSystemInfo(fixture());
    for (const QString& text : {
             QStringLiteral("Kooka Pictura System Info"),
             QStringLiteral("Application"),
             QStringLiteral("Version: 0.2.0-test"),
             QStringLiteral("Build: Debug-test"),
             QStringLiteral("Qt: 6.11.1-test"),
             QStringLiteral("System"),
             QStringLiteral("OS: Linux-test"),
             QStringLiteral("OS version: TestOS 1"),
             QStringLiteral("Kernel: test-kernel"),
             QStringLiteral("Architecture: x86_64-test"),
             QStringLiteral("Desktop: GNOME-test"),
             QStringLiteral("Session: wayland-test"),
             QStringLiteral("CPU"),
             QStringLiteral("Model: Test CPU"),
             QStringLiteral("Physical cores: 8"),
             QStringLiteral("Logical cores: 16"),
             QStringLiteral("Clock: 3.60 GHz"),
             QStringLiteral("Memory"),
             QStringLiteral("RAM total: 32.0 GB"),
             QStringLiteral("RAM available: 16.0 GB"),
             QStringLiteral("Swap total: 8.0 GB"),
             QStringLiteral("Swap available: 8.0 GB"),
             QStringLiteral("System CPU: 12.3 %"),
             QStringLiteral("Pictura RAM: 245.0 MB"),
             QStringLiteral("Pictura CPU: 1.2 %"),
             QStringLiteral("Pictura CPU average: 0.8 %"),
             QStringLiteral("GPU"),
             QStringLiteral("Adapter: Test GPU"),
             QStringLiteral("Type: Discrete"),
             QStringLiteral("Backend: Vulkan"),
             QStringLiteral("Driver: Test Driver 1.2"),
             QStringLiteral("VRAM: Unavailable"),
             QStringLiteral("Displays"),
             QStringLiteral("Display 1: DP-1 (Test Corp Monitor)"),
             QStringLiteral("Resolution: 1920 x 1080"),
             QStringLiteral("Physical size: 24.0 in"),
             QStringLiteral("DPI: 96 DPI"),
             QStringLiteral("Refresh rate: 60 Hz"),
         }) {
        QVERIFY2(report.contains(text), qPrintable(text));
    }
}

void SystemInfoTest::reportHasGpuFields()
{
    pictura::SystemInfo info;
    info.gpuName = QStringLiteral("Fixture GPU");
    info.gpuType = QStringLiteral("Integrated");
    info.gpuBackend = QStringLiteral("Vulkan");
    info.gpuDriver = QStringLiteral("Fixture Driver 9");
    const QString report = pictura::formatSystemInfo(info);
    QVERIFY(report.contains(QStringLiteral("Adapter: Fixture GPU")));
    QVERIFY(report.contains(QStringLiteral("Type: Integrated")));
    QVERIFY(report.contains(QStringLiteral("Backend: Vulkan")));
    QVERIFY(report.contains(QStringLiteral("Driver: Fixture Driver 9")));
}

void SystemInfoTest::reportUsesUnavailable()
{
    const QString report = pictura::formatSystemInfo(pictura::SystemInfo());
    for (const QString& text : {
             QStringLiteral("Application"),
             QStringLiteral("Version: Unavailable"),
             QStringLiteral("OS: Unavailable"),
             QStringLiteral("Model: Unavailable"),
             QStringLiteral("RAM total: Unavailable"),
             QStringLiteral("Adapter: Unavailable"),
             QStringLiteral("VRAM: Unavailable"),
             QStringLiteral("Displays"),
             QStringLiteral("Connected: 0"),
         }) {
        QVERIFY2(report.contains(text), qPrintable(text));
    }
}

void SystemInfoTest::dialogCopiesReport()
{
    const pictura::SystemInfo info = fixture();
    pictura::SystemInfoDialog dialog(info);
    QCOMPARE(dialog.reportForTest(), pictura::formatSystemInfo(info));
    auto* text = dialog.findChild<QPlainTextEdit*>(QStringLiteral("systemInfoText"));
    QVERIFY(text != nullptr);
    QCOMPARE(text->toPlainText(), dialog.reportForTest());
    auto* copy = dialog.findChild<QPushButton*>(QStringLiteral("systemInfoCopy"));
    QVERIFY(copy != nullptr);
    copy->click();
    QCOMPARE(QGuiApplication::clipboard()->text(), dialog.reportForTest());
}

void SystemInfoTest::reopenRefreshesReport()
{
    pictura::SystemInfo first = fixture();
    first.appVersion = QStringLiteral("first");
    pictura::SystemInfoDialog dialog(first);
    pictura::SystemInfo second = fixture();
    second.appVersion = QStringLiteral("second");
    dialog.setInfo(second);
    QCOMPARE(dialog.reportForTest(), pictura::formatSystemInfo(second));
    auto* text = dialog.findChild<QPlainTextEdit*>(QStringLiteral("systemInfoText"));
    QVERIFY(text != nullptr);
    QCOMPARE(text->toPlainText(), dialog.reportForTest());
    QVERIFY(dialog.reportForTest().contains(QStringLiteral("Version: second")));
}

void SystemInfoTest::submitUrlEncodesReport()
{
    const QString report = QStringLiteral("Kooka Pictura System Info\nVersion: 0.2.0\n");
    const QUrl url = pictura::submitIssueUrl(report);
    QCOMPARE(url.scheme(), QStringLiteral("https"));
    QCOMPARE(url.host(), QStringLiteral("github.com"));
    QCOMPARE(url.path(), QStringLiteral("/KookaStudio/KookaPictura/issues/new"));
    const QUrlQuery query(url);
    const QString body = query.queryItemValue(QStringLiteral("body"), QUrl::FullyDecoded);
    QVERIFY(body.startsWith(QStringLiteral("### System Info")));
    QVERIFY(body.contains(report));
    QVERIFY(body.contains(QStringLiteral("```")));
}

void SystemInfoTest::submitUrlCapsLongReport()
{
    const QString report(10000, QLatin1Char('x'));
    const QUrl url = pictura::submitIssueUrl(report);
    const QUrlQuery query(url);
    const QString body = query.queryItemValue(QStringLiteral("body"), QUrl::FullyDecoded);
    QVERIFY2(body.size() <= 6000, qPrintable(QString::number(body.size())));
    QVERIFY(body.endsWith(QStringLiteral("[System info truncated]\n")));
}

void SystemInfoTest::parsesCpuInfo()
{
    const QString cpuInfo = QStringLiteral(
        "processor\t: 0\n"
        "model name\t: Test CPU @ 3.60GHz\n"
        "physical id\t: 0\n"
        "core id\t\t: 0\n"
        "cpu MHz\t\t: 3600.000\n"
        "cpu cores\t: 2\n"
        "\n"
        "processor\t: 1\n"
        "model name\t: Test CPU @ 3.60GHz\n"
        "physical id\t: 0\n"
        "core id\t\t: 1\n"
        "cpu MHz\t\t: 3600.000\n"
        "cpu cores\t: 2\n"
        "\n"
        "processor\t: 2\n"
        "model name\t: Test CPU @ 3.60GHz\n"
        "physical id\t: 0\n"
        "core id\t\t: 0\n"
        "cpu MHz\t\t: 3600.000\n"
        "cpu cores\t: 2\n"
        "\n"
        "processor\t: 3\n"
        "model name\t: Test CPU @ 3.60GHz\n"
        "physical id\t: 0\n"
        "core id\t\t: 1\n"
        "cpu MHz\t\t: 3600.000\n"
        "cpu cores\t: 2\n");
    QCOMPARE(pictura::parseCpuModel(cpuInfo), QStringLiteral("Test CPU @ 3.60GHz"));
    QCOMPARE(pictura::parsePhysicalCores(cpuInfo), 2);
    QCOMPARE(pictura::parseCpuClock(cpuInfo), QStringLiteral("3.60 GHz"));
}

void SystemInfoTest::parsesMemInfo()
{
    const QString memInfo = QStringLiteral(
        "MemTotal:       32768000 kB\n"
        "MemFree:         1000000 kB\n"
        "MemAvailable:   16384000 kB\n"
        "Buffers:          500000 kB\n"
        "Cached:           2000000 kB\n"
        "SwapTotal:       8388608 kB\n"
        "SwapFree:        4194304 kB\n");
    QCOMPARE(pictura::parseMemInfoBytes(memInfo, QStringLiteral("MemTotal")),
             32768000ULL * 1024ULL);
    QCOMPARE(pictura::parseMemInfoBytes(memInfo, QStringLiteral("MemAvailable")),
             16384000ULL * 1024ULL);
    QCOMPARE(pictura::parseMemInfoBytes(memInfo, QStringLiteral("SwapTotal")),
             8388608ULL * 1024ULL);
    QCOMPARE(pictura::parseMemInfoBytes(memInfo, QStringLiteral("SwapFree")),
             4194304ULL * 1024ULL);
    QCOMPARE(pictura::parseMemInfoBytes(memInfo, QStringLiteral("NoSuchKey")), 0ULL);
}

void SystemInfoTest::formatsBytes()
{
    QCOMPARE(pictura::formatBytes(512), QStringLiteral("512 B"));
    QCOMPARE(pictura::formatBytes(1536), QStringLiteral("1.5 KB"));
    QCOMPARE(pictura::formatBytes(1024ULL * 1024ULL), QStringLiteral("1.0 MB"));
    QCOMPARE(pictura::formatBytes(3ULL * 1024ULL * 1024ULL * 1024ULL),
             QStringLiteral("3.0 GB"));
}

void SystemInfoTest::collectorOmitsHostAndUser()
{
    const QString report = pictura::formatSystemInfo(pictura::collectSystemInfo(QStringList()));
    const QString host = QSysInfo::machineHostName();
    if (host.size() > 3) {
        QVERIFY2(!report.contains(host), "hostname leaked into the report");
    }
    for (const char* key : {"USER", "USERNAME"}) {
        const QString value = qEnvironmentVariable(key);
        if (value.size() > 3) {
            QVERIFY2(!report.contains(value), "user name leaked into the report");
        }
    }
}

void SystemInfoTest::helpMenuWiresSystemInfo()
{
    auto* registry = window_->registry();
    QVERIFY(registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::HelpSystemInfo);
    const pictura::CommandInfo info = command(*registry, id);
    QVERIFY2(info.implemented, "System Info is implemented");
    QVERIFY2(info.enabled, "System Info is enabled without a document");
    QVERIFY(registry->dispatch(id));
    QCoreApplication::processEvents();
    auto* dialog = window_->findChild<pictura::SystemInfoDialog*>(
        QStringLiteral("systemInfoDialog"));
    QVERIFY(dialog != nullptr);
    QVERIFY(dialog->isVisible());
    QVERIFY(!dialog->reportForTest().isEmpty());
    dialog->close();
}

void SystemInfoTest::helpMenuWiresSubmitIssue()
{
    auto* registry = window_->registry();
    QVERIFY(registry != nullptr);
    const QString id = QString::fromLatin1(pictura::command_ids::HelpSubmitIssue);
    const pictura::CommandInfo info = command(*registry, id);
    QVERIFY2(info.implemented, "Submit Issue is implemented");
    QVERIFY2(info.enabled, "Submit Issue is enabled without a document");

    QMenu* help = nullptr;
    for (QAction* top : window_->menuBar()->actions()) {
        if (top->text().remove(QLatin1Char('&')) == QStringLiteral("Help")) {
            help = top->menu();
        }
    }
    QVERIFY(help != nullptr);
    QStringList labels;
    for (QAction* action : help->actions()) {
        if (!action->isSeparator()) {
            labels.append(action->text());
        }
    }
    const int systemInfo = labels.indexOf(QStringLiteral("System Info"));
    QVERIFY(systemInfo >= 0);
    QCOMPARE(labels.value(systemInfo + 1), QStringLiteral("Submit Issue"));
}

QTEST_MAIN(SystemInfoTest)
#include "tst_system_info.moc"
