// The visible update layer: the banner, the Preferences > Updates page, and
// the Help > Check for Updates command. The network is a local test server.

#include <QtTest/QtTest>

#include <QtCore/QByteArray>
#include <QtCore/QDir>
#include <QtCore/QProcess>
#include <QtCore/QProcessEnvironment>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtCore/QUrl>
#include <QtGui/QAction>
#include <QtNetwork/QHostAddress>
#include <QtNetwork/QTcpServer>
#include <QtNetwork/QTcpSocket>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QWidget>

#include "api/view_bridge.h"
#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "frame_update.h"
#include "gpu_probe.h"
#include "preferences_dialog.h"
#include "session.h"
#include "preferences.h"
#include "update_banner.h"
#include "update_package.h"
#include "update_preferences_page.h"

#include "qt_test_support.h"

using pictura::UpdateBanner;

namespace {

constexpr char kAdvertisedVersion[] = "9999.0.0";

class TestReleaseServer : public QTcpServer {
public:
    explicit TestReleaseServer(QObject* parent = nullptr)
        : QTcpServer(parent)
    {
        connect(this, &QTcpServer::newConnection, this, &TestReleaseServer::acceptConnections);
    }

    bool start() { return listen(QHostAddress::LocalHost, 0); }
    int requestCount() const { return requestCount_; }

    QUrl url() const
    {
        return QUrl(QStringLiteral("http://127.0.0.1:%1/latest").arg(serverPort()));
    }

private:
    void acceptConnections()
    {
        while (hasPendingConnections()) {
            QTcpSocket* socket = nextPendingConnection();
            connect(socket, &QTcpSocket::readyRead, this, [this, socket] { readRequest(socket); });
            connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
        }
    }

    void readRequest(QTcpSocket* socket)
    {
        requestBuffer_ += socket->readAll();
        const int headerEnd = requestBuffer_.indexOf("\r\n\r\n");
        if (headerEnd < 0) {
            return;
        }
        requestBuffer_.clear();
        ++requestCount_;
        const QByteArray body = QByteArrayLiteral("{\"tag_name\":\"v")
            + QByteArray(kAdvertisedVersion)
            + QByteArrayLiteral("\",\"html_url\":\"https://example.test/release\","
                                "\"body\":\"Release notes\",\"assets\":[]}");
        QByteArray response = "HTTP/1.1 200 OK\r\n";
        response += "Content-Type: application/json\r\n";
        response += "Content-Length: " + QByteArray::number(body.size()) + "\r\n";
        response += "Connection: close\r\n\r\n";
        response += body;
        socket->write(response);
        socket->disconnectFromHost();
    }

    QByteArray requestBuffer_;
    int requestCount_ = 0;
};

class ScopedEnvironment {
public:
    ScopedEnvironment(const char* name, const QByteArray& value)
        : name_(name)
        , prior_(qgetenv(name))
        , hadPrior_(!prior_.isNull())
    {
        qputenv(name_, value);
    }

    ~ScopedEnvironment()
    {
        if (hadPrior_) {
            qputenv(name_, prior_);
        } else {
            qunsetenv(name_);
        }
    }

    ScopedEnvironment(const ScopedEnvironment&) = delete;
    ScopedEnvironment& operator=(const ScopedEnvironment&) = delete;

private:
    const char* name_;
    QByteArray prior_;
    bool hadPrior_;
};

QStringList openAboutLines(pictura::PicturaMainWindow& window)
{
    auto* action = window.registry()->action(QString::fromLatin1(pictura::command_ids::HelpAbout));
    if (!action || !action->isEnabled()) {
        return {};
    }

    QString text;
    const auto capture = [&] {
        auto* dialog = window.findChild<QMessageBox*>();
        if (dialog && dialog->isVisible()) {
            text = dialog->text();
            const auto format = dialog->textFormat();
            dialog->accept();
            QCOMPARE(format, Qt::PlainText);
        }
    };
    // About is modal on some platforms and modeless on macOS.
    QObject pendingCapture;
    QTimer::singleShot(0, &pendingCapture, capture);
    action->trigger();
    capture();
    return text.split(QLatin1Char('\n'));
}

} // namespace

class UpdatePreferencesTest : public QObject {
    Q_OBJECT

private slots:
    void launchCheckShowsDialog();
    void bannerShowsVersionTransition();
    void bannerDoesNotStealFocus();
    void laterHidesBanner();
    void progressIsShown();
    void cancelStopsDownload();
    void failureOffersRetry();
    void flatpakShowsUpdateCommand();
    void snapShowsUpdateCommand();
    void debRpmShowsUpdateNow();
    void preferencesDefaultOn();
    void togglePersists();
    void checkNowChecks();
    void dontCheckDisablesAutoChecks();
    void reenableFromPreferences();
    void menuEntryChecks();
    void versionFlagWorksWithoutDisplay();
    void aboutShowsVersionAndMethod();
    void aboutShowsRuntimeDetails();
    void aboutSeparatesCpuRendererFromAdapter();
    void aboutDisplaysUnavailableAdapterFields();
};

void UpdatePreferencesTest::launchCheckShowsDialog()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    ScopedEnvironment endpoint("PICTURA_UPDATE_URL", server.url().toString().toUtf8());
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    pictura::PicturaMainWindow window(nullptr, true);
    window.show();
    pictura::UpdateBanner* banner = nullptr;
    QTRY_VERIFY_WITH_TIMEOUT((banner = window.findChild<pictura::UpdateBanner*>()), 3000);
    QTRY_VERIFY_WITH_TIMEOUT(banner->isVisible(), 3000);
    QVERIFY(banner->titleText().contains(QString::fromLatin1(kAdvertisedVersion)));
    QCOMPARE(banner->updateButton()->text(), QStringLiteral("Update now"));
}

void UpdatePreferencesTest::bannerShowsVersionTransition()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), true, QString());

    QVERIFY(banner.isVisible());
    QVERIFY(banner.titleText().contains(QStringLiteral("0.3.0 → 0.4.0")));
    QCOMPARE(banner.updateButton()->text(), QStringLiteral("Update now"));
    QCOMPARE(banner.automaticChecksBox()->text(),
             QStringLiteral("Check for updates automatically"));
    QVERIFY(banner.automaticChecksBox()->isChecked());
    QVERIFY(banner.laterButton()->isVisible());
    QVERIFY(banner.notesButton()->isVisible());
    QVERIFY(!banner.releasePageButton()->isVisible());
    QVERIFY(banner.width() <= 640);
    QVERIFY(banner.height() < 400);
}

void UpdatePreferencesTest::bannerDoesNotStealFocus()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    QLineEdit edit(&host);
    edit.move(10, 10);
    edit.show();
    host.activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(&host));
    edit.setFocus();
    QTRY_VERIFY(QApplication::focusWidget() == &edit);

    UpdateBanner banner(&host);
    banner.showUpdate(QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), true, QString());
    QCOMPARE(QApplication::focusWidget(), static_cast<QWidget*>(&edit));
}

void UpdatePreferencesTest::laterHidesBanner()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), true, QString());
    QVERIFY(banner.isVisible());

    banner.laterButton()->click();
    QVERIFY(!banner.isVisible());
}

void UpdatePreferencesTest::progressIsShown()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), true, QString());
    banner.showProgress(50, 100);

    QVERIFY(banner.progressBar()->isVisible());
    QCOMPARE(banner.progressBar()->value(), 50);
    QCOMPARE(banner.progressBar()->maximum(), 100);
    QVERIFY(banner.cancelButton()->isVisible());
    QVERIFY(!banner.updateButton()->isVisible());
}

void UpdatePreferencesTest::cancelStopsDownload()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), true, QString());
    banner.showProgress(50, 100);

    QSignalSpy cancelled(&banner, &UpdateBanner::cancelRequested);
    banner.cancelButton()->click();
    QCOMPARE(cancelled.count(), 1);
    QVERIFY(!banner.progressBar()->isVisible());
    QVERIFY(banner.updateButton()->isVisible());
    QVERIFY(banner.titleText().contains(QStringLiteral("0.4.0")));
}

void UpdatePreferencesTest::failureOffersRetry()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), true, QString());
    banner.showError(QStringLiteral("The checksum does not match."));

    QVERIFY(banner.errorText().contains(QStringLiteral("checksum")));
    QCOMPARE(banner.updateButton()->text(), QStringLiteral("Retry"));
    QVERIFY(banner.releasePageButton()->isVisible());
    QVERIFY(!banner.progressBar()->isVisible());
}

void UpdatePreferencesTest::flatpakShowsUpdateCommand()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(
        QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), false,
        pictura::packageUpdateCommand(pictura::PackageMethod::Flatpak, QStringLiteral("0.4.0"),
                                      QStringLiteral("x86_64")));

    QVERIFY(banner.commandText().contains(QStringLiteral("flatpak update")));
    QCOMPARE(banner.updateButton()->text(), QStringLiteral("Update now"));
    QVERIFY(banner.releasePageButton()->isVisible());
}

void UpdatePreferencesTest::snapShowsUpdateCommand()
{
    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(
        QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), false,
        pictura::packageUpdateCommand(pictura::PackageMethod::Snap, QStringLiteral("0.4.0"),
                                      QStringLiteral("x86_64")));

    QVERIFY(banner.commandText().contains(QStringLiteral("snap refresh")));
    QCOMPARE(banner.updateButton()->text(), QStringLiteral("Update now"));
    QVERIFY(banner.releasePageButton()->isVisible());
}

void UpdatePreferencesTest::debRpmShowsUpdateNow()
{
    QCOMPARE(pictura::packageManagerInstallCommand(pictura::PackageMethod::Deb,
                                                   QStringLiteral("/tmp/a.deb")),
             QStringLiteral("pkexec apt-get install -y '/tmp/a.deb'"));
    QCOMPARE(pictura::packageManagerInstallCommand(pictura::PackageMethod::Rpm,
                                                   QStringLiteral("/tmp/a.rpm")),
             QStringLiteral("pkexec dnf install -y '/tmp/a.rpm'"));

    QWidget host;
    host.resize(900, 300);
    host.show();
    UpdateBanner banner(&host);
    banner.showUpdate(QStringLiteral("0.3.0"), QStringLiteral("0.4.0"), true, QString());
    QCOMPARE(banner.updateButton()->text(), QStringLiteral("Update now"));
    QVERIFY(banner.commandText().isEmpty());
}

void UpdatePreferencesTest::preferencesDefaultOn()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->showPreferences(pictura::PreferencesDialog::kUpdates);
    auto* page = window->preferencesDialog()->findChild<pictura::UpdatePreferencesPage*>();
    QVERIFY(page != nullptr);
    QVERIFY(page->automaticChecks());
    QVERIFY(page->resultText().contains(QStringLiteral("Never")));

    auto* version = page->findChild<QLabel*>(QStringLiteral("preferencesUpdateVersion"));
    auto* package = page->findChild<QLabel*>(QStringLiteral("preferencesUpdatePackage"));
    QVERIFY(version != nullptr);
    QVERIFY(package != nullptr);
    QVERIFY(version->text().contains(QStringLiteral(PICTURA_VERSION)));
    QVERIFY(package->text().contains(QStringLiteral("Package:")));
}

void UpdatePreferencesTest::togglePersists()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->showPreferences(pictura::PreferencesDialog::kUpdates);
    auto* page = window->preferencesDialog()->findChild<pictura::UpdatePreferencesPage*>();
    QVERIFY(page != nullptr);

    page->automaticChecksBox()->setChecked(false);
    QVERIFY(!pictura::loadPreferences().checkForUpdatesOnLaunch);

    page->automaticChecksBox()->setChecked(true);
    QVERIFY(pictura::loadPreferences().checkForUpdatesOnLaunch);
}

void UpdatePreferencesTest::checkNowChecks()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    ScopedEnvironment endpoint("PICTURA_UPDATE_URL", server.url().toString().toUtf8());
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    window->showPreferences(pictura::PreferencesDialog::kUpdates);
    auto* page = window->preferencesDialog()->findChild<pictura::UpdatePreferencesPage*>();
    QVERIFY(page != nullptr);
    page->checkNowButton()->click();
    QTRY_VERIFY_WITH_TIMEOUT(page->resultText().contains(QString::fromLatin1(kAdvertisedVersion)),
                             3000);
}

void UpdatePreferencesTest::dontCheckDisablesAutoChecks()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    ScopedEnvironment endpoint("PICTURA_UPDATE_URL", server.url().toString().toUtf8());
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    QVERIFY(window->registry()->dispatch(pictura::command_ids::HelpCheckForUpdates));
    pictura::UpdateBanner* banner = nullptr;
    QTRY_VERIFY_WITH_TIMEOUT((banner = window->findChild<pictura::UpdateBanner*>()), 3000);
    QTRY_VERIFY_WITH_TIMEOUT(banner->isVisible(), 3000);

    banner->automaticChecksBox()->setChecked(false);
    QVERIFY(!pictura::loadPreferences().checkForUpdatesOnLaunch);

    window->showPreferences(pictura::PreferencesDialog::kUpdates);
    auto* page = window->preferencesDialog()->findChild<pictura::UpdatePreferencesPage*>();
    QVERIFY(page != nullptr);
    QVERIFY(!page->automaticChecks());
}

void UpdatePreferencesTest::reenableFromPreferences()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    ScopedEnvironment endpoint("PICTURA_UPDATE_URL", server.url().toString().toUtf8());
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    QVERIFY(window->registry()->dispatch(pictura::command_ids::HelpCheckForUpdates));
    pictura::UpdateBanner* banner = nullptr;
    QTRY_VERIFY_WITH_TIMEOUT((banner = window->findChild<pictura::UpdateBanner*>()), 3000);
    QTRY_VERIFY_WITH_TIMEOUT(banner->isVisible(), 3000);
    banner->automaticChecksBox()->setChecked(false);
    QVERIFY(!pictura::loadPreferences().checkForUpdatesOnLaunch);

    window->showPreferences(pictura::PreferencesDialog::kUpdates);
    auto* page = window->preferencesDialog()->findChild<pictura::UpdatePreferencesPage*>();
    QVERIFY(page != nullptr);
    page->automaticChecksBox()->setChecked(true);
    QVERIFY(pictura::loadPreferences().checkForUpdatesOnLaunch);
}

void UpdatePreferencesTest::menuEntryChecks()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    ScopedEnvironment endpoint("PICTURA_UPDATE_URL", server.url().toString().toUtf8());
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    window->show();
    QVERIFY(window->registry()->dispatch(pictura::command_ids::HelpCheckForUpdates));
    pictura::UpdateBanner* banner = nullptr;
    QTRY_VERIFY_WITH_TIMEOUT((banner = window->findChild<pictura::UpdateBanner*>()), 3000);
    QTRY_VERIFY_WITH_TIMEOUT(banner->isVisible(), 3000);
    QVERIFY(banner->titleText().contains(QString::fromLatin1(kAdvertisedVersion)));
}

void UpdatePreferencesTest::versionFlagWorksWithoutDisplay()
{
    const QString executable = qEnvironmentVariable("PICTURA_TEST_EXECUTABLE");
    QVERIFY2(!executable.isEmpty(), "Run this test through CTest to locate the pictura executable");
    TestReleaseServer server;
    QVERIFY(server.start());
    QTemporaryDir stateHome;
    QVERIFY(stateHome.isValid());

    auto environment = QProcessEnvironment::systemEnvironment();
    environment.remove(QStringLiteral("DISPLAY"));
    environment.remove(QStringLiteral("WAYLAND_DISPLAY"));
    environment.remove(QStringLiteral("PICTURA_DISABLE_UPDATE_CHECK"));
    environment.insert(QStringLiteral("QT_QPA_PLATFORM"), QStringLiteral("pictura-invalid-qpa"));
    environment.insert(QStringLiteral("XDG_STATE_HOME"), stateHome.path());
    environment.insert(QStringLiteral("PICTURA_UPDATE_URL"), server.url().toString());

    QProcess child;
    child.setProcessEnvironment(environment);
    child.start(executable, {QStringLiteral("--version")});
    QVERIFY2(child.waitForStarted(), qPrintable(child.errorString()));
    QTRY_COMPARE_WITH_TIMEOUT(child.state(), QProcess::NotRunning, 10000);
    QCOMPARE(child.exitStatus(), QProcess::NormalExit);
    QCOMPARE(child.exitCode(), 0);
    QCOMPARE(child.readAllStandardError(), QByteArray());
    QCOMPARE(child.readAllStandardOutput().replace("\r\n", "\n"),
             QByteArrayLiteral("Kooka Pictura " PICTURA_VERSION "\n"));

    QCoreApplication::processEvents();
    QCOMPARE(server.requestCount(), 0);
    QVERIFY(QDir(stateHome.path()).isEmpty(QDir::AllEntries | QDir::NoDotAndDotDot
                                         | QDir::Hidden | QDir::System));
}

void UpdatePreferencesTest::aboutShowsVersionAndMethod()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->show();

    const auto lines = openAboutLines(*window);
    const QString method = pictura::packageMethodName(pictura::detectPackageMethod());
    QVERIFY(lines.contains(QStringLiteral("Kooka Pictura " PICTURA_VERSION)));
    QVERIFY(lines.contains(QStringLiteral("Package: %1").arg(method)));
}

void UpdatePreferencesTest::aboutShowsRuntimeDetails()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    window->show();
    QCOMPARE(window->documentCount(), 0);
    window->applyGpuComputePreference(false);

    const auto lines = openAboutLines(*window);
    const QString unavailable = QStringLiteral("Unavailable");
    const QString buildType = QStringLiteral(PICTURA_BUILD_TYPE);
    const QString renderer = pictura::activeBackendLabel(false);
    pictura::PictureView probe;
    const QStringList gpuFields = probe.gpu_adapter_info();
    const QString adapter = gpuFields.value(0).trimmed();
    const QString driver = (gpuFields.value(3) + QLatin1Char(' ') + gpuFields.value(4)).trimmed();

    QVERIFY(lines.contains(QStringLiteral("Build type: %1")
                              .arg(buildType.isEmpty() ? unavailable : buildType)));
    QVERIFY(lines.contains(QStringLiteral("Qt version: %1").arg(QString::fromLatin1(qVersion()))));
    QVERIFY(lines.contains(QStringLiteral("Renderer: %1").arg(renderer)));
    QVERIFY(lines.contains(QStringLiteral("GPU adapter: %1").arg(adapter.isEmpty() ? unavailable : adapter)));
    QVERIFY(lines.contains(QStringLiteral("GPU driver: %1").arg(driver.isEmpty() ? unavailable : driver)));
}

void UpdatePreferencesTest::aboutSeparatesCpuRendererFromAdapter()
{
    const QStringList gpuFields{QStringLiteral("Test GPU"), QStringLiteral("Discrete"),
                                QStringLiteral("Vulkan"), QStringLiteral("Test Driver"),
                                QStringLiteral("1.2")};
    const auto lines = pictura::aboutText(QStringLiteral("CPU"), gpuFields).split(QLatin1Char('\n'));
    QVERIFY(lines.contains(QStringLiteral("Renderer: CPU")));
    QVERIFY(lines.contains(QStringLiteral("GPU adapter: Test GPU")));
    QVERIFY(lines.contains(QStringLiteral("GPU driver: Test Driver 1.2")));
}

void UpdatePreferencesTest::aboutDisplaysUnavailableAdapterFields()
{
    const auto absent = pictura::aboutText(QStringLiteral("CPU (no GPU)"), {}).split(QLatin1Char('\n'));
    QVERIFY(absent.contains(QStringLiteral("Renderer: CPU (no GPU)")));
    QVERIFY(absent.contains(QStringLiteral("GPU adapter: Unavailable")));
    QVERIFY(absent.contains(QStringLiteral("GPU driver: Unavailable")));

    const auto partial = pictura::aboutText(QStringLiteral("CPU"), {QStringLiteral("Test GPU")})
                             .split(QLatin1Char('\n'));
    QVERIFY(partial.contains(QStringLiteral("Renderer: CPU")));
    QVERIFY(partial.contains(QStringLiteral("GPU adapter: Test GPU")));
    QVERIFY(partial.contains(QStringLiteral("GPU driver: Unavailable")));
}

PICTURA_QT_TEST_MAIN(UpdatePreferencesTest)
#include "tst_update_preferences.moc"
