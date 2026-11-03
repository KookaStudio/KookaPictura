// The visible update layer: the banner, the Preferences > Updates page, and
// the Help > Check for Updates command. The network is a local test server.

#include <QtTest/QtTest>

#include <QtCore/QByteArray>
#include <QtCore/QUrl>
#include <QtNetwork/QHostAddress>
#include <QtNetwork/QTcpServer>
#include <QtNetwork/QTcpSocket>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QWidget>

#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "frame_update.h"
#include "preferences_dialog.h"
#include "session.h"
#include "update_banner.h"
#include "update_package.h"
#include "update_preferences_page.h"

#include "qt_test_support.h"

using pictura::UpdateBanner;

namespace {

class TestReleaseServer : public QTcpServer {
public:
    explicit TestReleaseServer(QObject* parent = nullptr)
        : QTcpServer(parent)
    {
        connect(this, &QTcpServer::newConnection, this, &TestReleaseServer::acceptConnections);
    }

    bool start() { return listen(QHostAddress::LocalHost, 0); }

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
        const QByteArray body = QByteArrayLiteral(
            "{\"tag_name\":\"v0.4.0\",\"html_url\":\"https://example.test/release\","
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

} // namespace

class UpdatePreferencesTest : public QObject {
    Q_OBJECT

private slots:
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
    void aboutShowsVersionAndMethod();
};

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
        pictura::packageUpdateCommand(pictura::PackageMethod::Flatpak, QStringLiteral("0.4.0")));

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
        pictura::packageUpdateCommand(pictura::PackageMethod::Snap, QStringLiteral("0.4.0")));

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
    QVERIFY(!pictura::loadSession().checkForUpdatesOnLaunch);

    page->automaticChecksBox()->setChecked(true);
    QVERIFY(pictura::loadSession().checkForUpdatesOnLaunch);
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
    QTRY_VERIFY_WITH_TIMEOUT(page->resultText().contains(QStringLiteral("0.4.0")), 3000);
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
    QVERIFY(!pictura::loadSession().checkForUpdatesOnLaunch);

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
    QVERIFY(!pictura::loadSession().checkForUpdatesOnLaunch);

    window->showPreferences(pictura::PreferencesDialog::kUpdates);
    auto* page = window->preferencesDialog()->findChild<pictura::UpdatePreferencesPage*>();
    QVERIFY(page != nullptr);
    page->automaticChecksBox()->setChecked(true);
    QVERIFY(pictura::loadSession().checkForUpdatesOnLaunch);
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
    QVERIFY(banner->titleText().contains(QStringLiteral("0.4.0")));
}

void UpdatePreferencesTest::aboutShowsVersionAndMethod()
{
    const QString text = pictura::aboutVersionText();
    QVERIFY(text.contains(QStringLiteral("Kooka Pictura %1").arg(QStringLiteral(PICTURA_VERSION))));
    QVERIFY(text.contains(QStringLiteral("Package: ")));
}

QTEST_MAIN(UpdatePreferencesTest)
#include "tst_update_preferences.moc"
