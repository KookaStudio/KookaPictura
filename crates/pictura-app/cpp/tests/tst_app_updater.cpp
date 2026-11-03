// The pure update logic: SemVer ordering, package-method detection, release
// JSON parsing, and asset selection. No network, no window, no installer.

#include <QtTest/QtTest>

#include <QtCore/QByteArray>
#include <QtCore/QCryptographicHash>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QUrl>
#include <QtNetwork/QHostAddress>
#include <QtNetwork/QTcpServer>
#include <QtNetwork/QTcpSocket>

#include "api/document_api.h"
#include "dialogs.h"
#include "qt_test_support.h"
#include "session.h"
#include "update_checker.h"
#include "update_installer.h"
#include "update_package.h"
#include "update_release.h"
#include "update_version.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

using pictura::PackageDetectionInput;
using pictura::PackageMethod;

namespace {

QString digest(char fill)
{
    return QStringLiteral("sha256:") + QString(64, QLatin1Char(fill));
}

QJsonObject releaseAsset(const QString& name, const QString& url, qint64 size, char fill)
{
    QJsonObject object;
    object.insert(QStringLiteral("name"), name);
    object.insert(QStringLiteral("browser_download_url"), url);
    object.insert(QStringLiteral("size"), double(size));
    object.insert(QStringLiteral("digest"), digest(fill));
    return object;
}

QByteArray releaseFixture()
{
    QJsonArray assets;
    assets.append(releaseAsset(
        QStringLiteral("kooka-pictura_0.4.0_amd64.snap"),
        QStringLiteral("https://example.test/kooka-pictura_0.4.0_amd64.snap"),
        23162880, 'a'));
    assets.append(releaseAsset(
        QStringLiteral("KookaPictura-0.4.0-1.x86_64.rpm"),
        QStringLiteral("https://example.test/KookaPictura-0.4.0-1.x86_64.rpm"),
        57574166, 'b'));
    assets.append(releaseAsset(QStringLiteral("KookaPictura-0.4.0-macOS.dmg"),
                               QStringLiteral("https://example.test/KookaPictura-0.4.0-macOS.dmg"),
                               38878237, 'c'));
    assets.append(releaseAsset(
        QStringLiteral("KookaPictura-0.4.0-windows-x64-portable.zip"),
        QStringLiteral("https://example.test/KookaPictura-0.4.0-windows-x64-portable.zip"),
        73384386, 'd'));
    assets.append(releaseAsset(
        QStringLiteral("KookaPictura-0.4.0-windows-x64-setup.exe"),
        QStringLiteral("https://example.test/KookaPictura-0.4.0-windows-x64-setup.exe"),
        56543552, 'e'));
    assets.append(releaseAsset(
        QStringLiteral("KookaPictura-0.4.0-x86_64.AppImage"),
        QStringLiteral("https://example.test/KookaPictura-0.4.0-x86_64.AppImage"),
        53590520, 'f'));
    assets.append(releaseAsset(
        QStringLiteral("KookaPictura-0.4.0-x86_64.flatpak"),
        QStringLiteral("https://example.test/KookaPictura-0.4.0-x86_64.flatpak"),
        24663768, '1'));
    assets.append(releaseAsset(QStringLiteral("KookaPictura_0.4.0_amd64.deb"),
                               QStringLiteral("https://example.test/KookaPictura_0.4.0_amd64.deb"),
                               39105444, '2'));

    QJsonObject root;
    root.insert(QStringLiteral("tag_name"), QStringLiteral("v0.4.0"));
    root.insert(QStringLiteral("html_url"),
                QStringLiteral("https://github.com/KookaStudio/KookaPictura/releases/tag/v0.4.0"));
    root.insert(QStringLiteral("body"),
                QStringLiteral("## What's new\n\n- Faster update checks\n"));
    root.insert(QStringLiteral("assets"), assets);
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

PackageDetectionInput linuxInput()
{
    PackageDetectionInput input;
    input.os = QStringLiteral("linux");
    input.executablePath = QStringLiteral("/opt/kooka-pictura/bin/pictura");
    return input;
}

// A one-request-at-a-time HTTP server for the checker tests. It captures the
// request headers and serves the fixture body with the chosen status.
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

    int requestCount() const { return requestCount_; }
    QByteArray lastHeaders() const { return lastHeaders_; }

    QByteArray body;
    int status = 200;
    qint64 declaredContentLength = -1;

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
        ++requestCount_;
        lastHeaders_ = requestBuffer_.left(headerEnd + 4);
        requestBuffer_.clear();

        QByteArray response = "HTTP/1.1 " + QByteArray::number(status) + " "
            + (status == 200 ? "OK" : "Error") + "\r\n";
        response += "Content-Type: application/json\r\n";
        const qint64 contentLength =
            declaredContentLength >= 0 ? declaredContentLength : body.size();
        response += "Content-Length: " + QByteArray::number(contentLength) + "\r\n";
        response += "Connection: close\r\n\r\n";
        response += body;
        socket->write(response);
        socket->disconnectFromHost();
    }

    int requestCount_ = 0;
    QByteArray lastHeaders_;
    QByteArray requestBuffer_;
};

pictura::UpdateCheckSettings settingsFor(const TestReleaseServer& server,
                                         pictura::PackageMethod method,
                                         const QString& currentVersion, bool automatic = true,
                                         bool networkAllowed = true)
{
    pictura::UpdateCheckSettings settings;
    settings.automaticChecksEnabled = automatic;
    settings.networkAllowed = networkAllowed;
    settings.packageMethod = method;
    settings.currentVersion = currentVersion;
    settings.endpoint = server.url();
    return settings;
}

bool waitForIdle(pictura::UpdateChecker& checker)
{
    return QTest::qWaitFor([&checker] { return !checker.isChecking(); }, 3000);
}

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

QString sha256Hex(const QByteArray& data)
{
    return QString::fromLatin1(QCryptographicHash::hash(data, QCryptographicHash::Sha256).toHex());
}

bool writeFile(const QString& path, const QByteArray& data)
{
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    return file.write(data) == data.size();
}

QByteArray readFile(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return {};
    }
    return file.readAll();
}

bool waitForInstaller(pictura::UpdateInstaller& installer)
{
    return QTest::qWaitFor([&installer] { return !installer.isDownloading(); }, 5000);
}

} // namespace

class AppUpdaterTest : public QObject {
    Q_OBJECT

private slots:
    void cleanup();
    void parsesVersions();
    void rejectsInvalidVersions();
    void newerReleaseNotifies();
    void sameVersionDoesNotNotify();
    void olderReleaseDoesNotNotify();
    void prereleaseOrdering();
    void detectsEnvironmentMarkers();
    void detectsLinuxPackages();
    void detectsWindowsInstallKinds();
    void detectsDevelopmentBuild();
    void detectsPackageMarkers();
    void parsesReleaseFixture();
    void rejectsMalformedReleaseJson();
    void selectsAppImageAsset();
    void selectsWindowsSetupAsset();
    void selectsMacDmgAsset();
    void missingAssetFallsBackToReleasePage();
    void packageCommands();
    void debRpmUsesPackageManager();
    void sessionCheckDefault();
    void environmentSettings();
    void checkOnLaunchSendsOneRequest();
    void disabledCheckMakesNoRequest();
    void headlessSkipsCheck();
    void developmentBuildDoesNotAutoCheck();
    void requestCarriesNoIdentifier();
    void rateLimitIsSilent();
    void networkFailureIsSilent();
    void offlineIsSilent();
    void manualCheckWorksWhenAutoOff();
    void manualCheckUpToDate();
    void manualCheckError();
    void acceptsMatchingDigest();
    void refusesMissingDigest();
    void refusesMismatchedDigest();
    void refusesOversizeDownload();
    void appImageReplacesAndRelaunches();
    void installFailureKeepsAppRunning();
    void savePromptRunsBeforeInstall();
    void cancelSaveAbortsInstall();
    void saveFailureAbortsInstall();
};

void AppUpdaterTest::cleanup()
{
    pictura::setUnsavedPromptInteractive(true);
    pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Discard);
}

void AppUpdaterTest::parsesVersions()
{
    const auto version = pictura::parseAppVersion(QStringLiteral("v0.4.0-beta.1+build.5"));
    QVERIFY(version.has_value());
    QCOMPARE(version->major, 0);
    QCOMPARE(version->minor, 4);
    QCOMPARE(version->patch, 0);
    const QStringList expectedPrerelease = {QStringLiteral("beta"), QStringLiteral("1")};
    QCOMPARE(version->prerelease, expectedPrerelease);
    QCOMPARE(version->build, QStringLiteral("build.5"));

    const auto plain = pictura::parseAppVersion(QStringLiteral("0.3.0"));
    QVERIFY(plain.has_value());
    QCOMPARE(plain->major, 0);
    QCOMPARE(plain->minor, 3);
    QCOMPARE(plain->patch, 0);
    QVERIFY(plain->prerelease.isEmpty());
}

void AppUpdaterTest::rejectsInvalidVersions()
{
    for (const QString& text : {
             QString(),
             QStringLiteral("v"),
             QStringLiteral("0.3"),
             QStringLiteral("0.3.0.1"),
             QStringLiteral("0.3.x"),
             QStringLiteral("0.3.0-"),
             QStringLiteral("0.3.0+"),
             QStringLiteral("0.3.0-beta..1"),
             QStringLiteral("0.3.0-01"),
             QStringLiteral("00.3.0"),
         }) {
        QVERIFY2(!pictura::parseAppVersion(text).has_value(), qPrintable(text));
    }
}

void AppUpdaterTest::newerReleaseNotifies()
{
    QVERIFY(pictura::isNewerVersion(QStringLiteral("0.4.0"), QStringLiteral("0.3.0")));
    QVERIFY(pictura::isNewerVersion(QStringLiteral("v0.4.0"), QStringLiteral("v0.3.0")));
    QVERIFY(pictura::isNewerVersion(QStringLiteral("0.4.0"), QStringLiteral("0.4.0-beta.1")));
}

void AppUpdaterTest::sameVersionDoesNotNotify()
{
    QVERIFY(!pictura::isNewerVersion(QStringLiteral("0.3.0"), QStringLiteral("0.3.0")));
    QVERIFY(!pictura::isNewerVersion(QStringLiteral("v0.3.0"), QStringLiteral("0.3.0")));
    QVERIFY(!pictura::isNewerVersion(QStringLiteral("0.3.0+build.1"),
                                     QStringLiteral("0.3.0+build.2")));
}

void AppUpdaterTest::olderReleaseDoesNotNotify()
{
    QVERIFY(!pictura::isNewerVersion(QStringLiteral("0.2.9"), QStringLiteral("0.3.0")));
    QVERIFY(!pictura::isNewerVersion(QStringLiteral("0.4.0-beta.1"),
                                     QStringLiteral("0.4.0")));
}

void AppUpdaterTest::prereleaseOrdering()
{
    QVERIFY(pictura::isNewerVersion(QStringLiteral("0.4.0-beta.2"),
                                    QStringLiteral("0.4.0-beta.1")));
    QVERIFY(pictura::isNewerVersion(QStringLiteral("0.4.0-beta.10"),
                                    QStringLiteral("0.4.0-beta.2")));
    QVERIFY(!pictura::isNewerVersion(QStringLiteral("0.4.0-beta"),
                                     QStringLiteral("0.4.0-beta.1")));
    QVERIFY(pictura::isNewerVersion(QStringLiteral("0.4.0-beta.1"),
                                    QStringLiteral("0.4.0-alpha.1")));
    QVERIFY(pictura::isNewerVersion(QStringLiteral("0.4.0-alpha"),
                                    QStringLiteral("0.4.0-1")));
}

void AppUpdaterTest::detectsEnvironmentMarkers()
{
    PackageDetectionInput input = linuxInput();
    input.environment.insert(QStringLiteral("FLATPAK_ID"),
                             QStringLiteral("io.github.KookaStudio.KookaPictura"));
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Flatpak);

    input.environment.clear();
    input.environment.insert(QStringLiteral("SNAP"), QStringLiteral("/snap/kooka-pictura"));
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Snap);

    input.environment.clear();
    input.environment.insert(QStringLiteral("APPIMAGE"), QStringLiteral("/tmp/pictura.AppImage"));
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::AppImage);
}

void AppUpdaterTest::detectsLinuxPackages()
{
    PackageDetectionInput input = linuxInput();
    input.debPackageInstalled = true;
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Deb);

    input.debPackageInstalled = false;
    input.rpmPackageInstalled = true;
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Rpm);

    input.executablePath = QStringLiteral("/usr/local/bin/pictura");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Development);
}

void AppUpdaterTest::detectsWindowsInstallKinds()
{
    PackageDetectionInput input;
    input.os = QStringLiteral("windows");
    input.executablePath = QStringLiteral("C:/Program Files/Kooka Pictura/bin/pictura.exe");
    input.windowsInstallMarker = true;
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::WindowsSetup);

    input.windowsInstallMarker = false;
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::WindowsPortable);
}

void AppUpdaterTest::detectsDevelopmentBuild()
{
    PackageDetectionInput input;
    input.os = QStringLiteral("linux");
    input.executablePath = QStringLiteral("/home/person/build/pictura");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Development);

    input.os = QStringLiteral("macos");
    input.executablePath = QStringLiteral("/tmp/pictura");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Development);

    input.executablePath =
        QStringLiteral("/Applications/Kooka Pictura.app/Contents/MacOS/pictura");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::MacDmg);
}

void AppUpdaterTest::detectsPackageMarkers()
{
    PackageDetectionInput input = linuxInput();
    input.packageMarker = QStringLiteral("deb");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Deb);
    input.packageMarker = QStringLiteral("rpm");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Rpm);
    input.packageMarker = QStringLiteral("appimage");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::AppImage);
    input.packageMarker = QStringLiteral("flatpak");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Flatpak);
    input.packageMarker = QStringLiteral("snap");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Snap);

    input.os = QStringLiteral("windows");
    input.packageMarker = QStringLiteral("windows-setup");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::WindowsSetup);
    input.packageMarker = QStringLiteral("windows-portable");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::WindowsPortable);

    input.os = QStringLiteral("macos");
    input.packageMarker = QStringLiteral("macos-dmg");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::MacDmg);

    input.os = QStringLiteral("linux");
    input.packageMarker = QStringLiteral("unknown");
    input.executablePath = QStringLiteral("/home/person/build/pictura");
    QVERIFY(pictura::detectPackageMethod(input) == PackageMethod::Development);
}

void AppUpdaterTest::parsesReleaseFixture()
{
    const auto release = pictura::parseReleaseJson(releaseFixture());
    QVERIFY(release.has_value());
    QCOMPARE(release->tag, QStringLiteral("v0.4.0"));
    QCOMPARE(release->version, QStringLiteral("0.4.0"));
    QCOMPARE(release->htmlUrl,
             QStringLiteral("https://github.com/KookaStudio/KookaPictura/releases/tag/v0.4.0"));
    QVERIFY(release->body.contains(QStringLiteral("Faster update checks")));
    QCOMPARE(int(release->assets.size()), 8);
    QCOMPARE(int(release->assets.first().sha256.size()), 64);
}

void AppUpdaterTest::rejectsMalformedReleaseJson()
{
    QVERIFY(!pictura::parseReleaseJson(QByteArray()).has_value());
    QVERIFY(!pictura::parseReleaseJson(QByteArrayLiteral("{")).has_value());
    QVERIFY(!pictura::parseReleaseJson(QByteArrayLiteral("{}")).has_value());
    QVERIFY(!pictura::parseReleaseJson(
                 QByteArrayLiteral("{\"tag_name\":\"v0.4.0\"}"))
                 .has_value());
    QVERIFY(!pictura::parseReleaseJson(QByteArrayLiteral(
                 "{\"tag_name\":\"v0.4.0\",\"html_url\":\"x\",\"assets\":{}}"))
                 .has_value());
    QVERIFY(!pictura::parseReleaseJson(QByteArrayLiteral(
                 "{\"tag_name\":\"v0.4.0\",\"html_url\":\"x\",\"assets\":[1]}"))
                 .has_value());

    const auto missingDigest = pictura::parseReleaseJson(QByteArrayLiteral(
        "{\"tag_name\":\"v0.4.0\",\"html_url\":\"x\",\"assets\":[{\"name\":\"x.AppImage\","
        "\"browser_download_url\":\"https://example.test/x\"}]}"));
    QVERIFY(missingDigest.has_value());
    QCOMPARE(int(missingDigest->assets.size()), 1);
    QVERIFY(missingDigest->assets.first().sha256.isEmpty());
}

void AppUpdaterTest::selectsAppImageAsset()
{
    const auto release = pictura::parseReleaseJson(releaseFixture());
    QVERIFY(release.has_value());
    const auto asset = pictura::selectReleaseAsset(*release, PackageMethod::AppImage,
                                                   QStringLiteral("x86_64"));
    QVERIFY(asset.has_value());
    QCOMPARE(asset->name, QStringLiteral("KookaPictura-0.4.0-x86_64.AppImage"));
    QCOMPARE(asset->size, qint64(53590520));
    QCOMPARE(int(asset->sha256.size()), 64);
}

void AppUpdaterTest::selectsWindowsSetupAsset()
{
    const auto release = pictura::parseReleaseJson(releaseFixture());
    QVERIFY(release.has_value());
    const auto asset = pictura::selectReleaseAsset(*release, PackageMethod::WindowsSetup,
                                                   QStringLiteral("x64"));
    QVERIFY(asset.has_value());
    QCOMPARE(asset->name, QStringLiteral("KookaPictura-0.4.0-windows-x64-setup.exe"));
}

void AppUpdaterTest::selectsMacDmgAsset()
{
    const auto release = pictura::parseReleaseJson(releaseFixture());
    QVERIFY(release.has_value());
    const auto asset = pictura::selectReleaseAsset(*release, PackageMethod::MacDmg,
                                                   QStringLiteral("arm64"));
    QVERIFY(asset.has_value());
    QCOMPARE(asset->name, QStringLiteral("KookaPictura-0.4.0-macOS.dmg"));
}

void AppUpdaterTest::missingAssetFallsBackToReleasePage()
{
    const auto release = pictura::parseReleaseJson(releaseFixture());
    QVERIFY(release.has_value());
    QVERIFY(!release->htmlUrl.isEmpty());
    QVERIFY(!pictura::selectReleaseAsset(*release, PackageMethod::AppImage,
                                         QStringLiteral("aarch64"))
                 .has_value());
    QVERIFY(!pictura::selectReleaseAsset(*release, PackageMethod::Development,
                                         QStringLiteral("x86_64"))
                 .has_value());
}

void AppUpdaterTest::packageCommands()
{
    QCOMPARE(pictura::packageUpdateCommand(PackageMethod::Flatpak, QStringLiteral("0.4.0")),
             QStringLiteral("flatpak update io.github.KookaStudio.KookaPictura"));
    QCOMPARE(pictura::packageUpdateCommand(PackageMethod::Snap, QStringLiteral("0.4.0")),
             QStringLiteral("snap refresh kooka-pictura"));
    QCOMPARE(pictura::packageUpdateCommand(PackageMethod::Deb, QStringLiteral("v0.4.0")),
             QStringLiteral("sudo apt install ./KookaPictura_0.4.0_amd64.deb"));
    QCOMPARE(pictura::packageUpdateCommand(PackageMethod::Rpm, QStringLiteral("0.4.0")),
             QStringLiteral("sudo dnf install ./KookaPictura-0.4.0-1.x86_64.rpm"));
    QVERIFY(pictura::packageUpdateCommand(PackageMethod::AppImage, QStringLiteral("0.4.0"))
                .isEmpty());

    QVERIFY(pictura::packageMethodSelfUpdates(PackageMethod::AppImage));
    QVERIFY(pictura::packageMethodSelfUpdates(PackageMethod::WindowsSetup));
    QVERIFY(pictura::packageMethodSelfUpdates(PackageMethod::MacDmg));
    QVERIFY(!pictura::packageMethodSelfUpdates(PackageMethod::Flatpak));
    QVERIFY(!pictura::packageMethodSelfUpdates(PackageMethod::Development));

    QVERIFY(pictura::packageMethodUsesPackageManager(PackageMethod::Flatpak));
    QVERIFY(pictura::packageMethodUsesPackageManager(PackageMethod::Snap));
    QVERIFY(pictura::packageMethodUsesPackageManager(PackageMethod::Deb));
    QVERIFY(pictura::packageMethodUsesPackageManager(PackageMethod::Rpm));
    QVERIFY(!pictura::packageMethodUsesPackageManager(PackageMethod::AppImage));

    QCOMPARE(pictura::packageMethodName(PackageMethod::Development),
             QStringLiteral("Development build"));
}

void AppUpdaterTest::debRpmUsesPackageManager()
{
    QVERIFY(pictura::packageMethodSelfUpdates(PackageMethod::Deb));
    QVERIFY(pictura::packageMethodSelfUpdates(PackageMethod::Rpm));
    QCOMPARE(pictura::packageManagerInstallCommand(PackageMethod::Deb,
                                                   QStringLiteral("/tmp/a.deb")),
             QStringLiteral("pkexec apt-get install -y '/tmp/a.deb'"));
    QCOMPARE(pictura::packageManagerInstallCommand(PackageMethod::Rpm,
                                                   QStringLiteral("/tmp/a.rpm")),
             QStringLiteral("pkexec dnf install -y '/tmp/a.rpm'"));
}

void AppUpdaterTest::sessionCheckDefault()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());

    pictura::SessionState state;
    QVERIFY(state.checkForUpdatesOnLaunch);
    QCOMPARE(state.schemaVersion, 12);

    state.checkForUpdatesOnLaunch = false;
    QVERIFY(pictura::saveSession(state));
    const pictura::SessionState loaded = pictura::loadSession();
    QVERIFY(!loaded.checkForUpdatesOnLaunch);
    QCOMPARE(loaded.schemaVersion, 12);
}

void AppUpdaterTest::environmentSettings()
{
    ScopedEnvironment endpoint("PICTURA_UPDATE_URL",
                               QByteArrayLiteral("http://127.0.0.1:4567/latest"));
    ScopedEnvironment disable("PICTURA_DISABLE_UPDATE_CHECK", QByteArrayLiteral("1"));

    const pictura::UpdateCheckSettings settings =
        pictura::updateCheckSettingsFromEnvironment(true, PackageMethod::AppImage,
                                                    QStringLiteral("0.3.0"));
    QCOMPARE(settings.endpoint, QUrl(QStringLiteral("http://127.0.0.1:4567/latest")));
    QVERIFY(!settings.automaticChecksEnabled);
    QVERIFY(settings.networkAllowed);
    QCOMPARE(settings.packageMethod, PackageMethod::AppImage);
    QCOMPARE(settings.currentVersion, QStringLiteral("0.3.0"));
}

void AppUpdaterTest::checkOnLaunchSendsOneRequest()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();

    pictura::UpdateChecker checker;
    checker.setSettings(settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0")));
    QSignalSpy available(&checker, &pictura::UpdateChecker::updateAvailable);

    QVERIFY(checker.checkAutomatically());
    QVERIFY(!checker.checkAutomatically());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(available.count(), 1);
    QCOMPARE(server.requestCount(), 1);
    QVERIFY(checker.release().has_value());
    QVERIFY(checker.asset().has_value());
    QCOMPARE(checker.asset()->name, QStringLiteral("KookaPictura-0.4.0-x86_64.AppImage"));
}

void AppUpdaterTest::disabledCheckMakesNoRequest()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();

    pictura::UpdateChecker checker;
    checker.setSettings(
        settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0"), false));
    QVERIFY(!checker.checkAutomatically());
    QTest::qWait(100);
    QCOMPARE(server.requestCount(), 0);
}

void AppUpdaterTest::headlessSkipsCheck()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();

    pictura::UpdateChecker checker;
    checker.setSettings(settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0"),
                                    true, false));
    QVERIFY(!checker.checkAutomatically());
    QVERIFY(!checker.checkManually());
    QTest::qWait(100);
    QCOMPARE(server.requestCount(), 0);
}

void AppUpdaterTest::developmentBuildDoesNotAutoCheck()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();

    pictura::UpdateChecker checker;
    checker.setSettings(
        settingsFor(server, PackageMethod::Development, QStringLiteral("0.3.0")));
    QVERIFY(!checker.checkAutomatically());
    QTest::qWait(100);
    QCOMPARE(server.requestCount(), 0);

    QSignalSpy available(&checker, &pictura::UpdateChecker::updateAvailable);
    QVERIFY(checker.checkManually());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(available.count(), 1);
    QCOMPARE(server.requestCount(), 1);
}

void AppUpdaterTest::requestCarriesNoIdentifier()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();

    pictura::UpdateChecker checker;
    checker.setSettings(settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0")));
    QVERIFY(checker.checkAutomatically());
    QVERIFY(waitForIdle(checker));

    const QByteArray headers = server.lastHeaders().toLower();
    QVERIFY(headers.contains("user-agent: kookapictura/0.3.0"));
    QVERIFY(headers.contains("accept: application/vnd.github+json"));
    QVERIFY(headers.contains("x-github-api-version: 2022-11-28"));
    QVERIFY(!headers.contains("authorization:"));
    QVERIFY(!headers.contains("cookie:"));
}

void AppUpdaterTest::rateLimitIsSilent()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.status = 403;
    server.body = QByteArrayLiteral("{\"message\":\"rate limit\"}");

    pictura::UpdateChecker checker;
    checker.setSettings(settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0")));
    QSignalSpy failed(&checker, &pictura::UpdateChecker::failed);
    QSignalSpy available(&checker, &pictura::UpdateChecker::updateAvailable);
    QSignalSpy upToDate(&checker, &pictura::UpdateChecker::upToDate);

    QVERIFY(checker.checkAutomatically());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(failed.count(), 0);
    QCOMPARE(available.count(), 0);
    QCOMPARE(upToDate.count(), 0);
    QVERIFY(!checker.lastError().isEmpty());

    server.status = 429;
    QVERIFY(checker.checkManually());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(failed.count(), 1);
    QVERIFY(failed.first().at(0).toString().contains(QStringLiteral("rate limit"),
                                                      Qt::CaseInsensitive));
}

void AppUpdaterTest::networkFailureIsSilent()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.status = 500;
    server.body = QByteArrayLiteral("{}");

    pictura::UpdateChecker checker;
    checker.setSettings(settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0")));
    QSignalSpy failed(&checker, &pictura::UpdateChecker::failed);

    QVERIFY(checker.checkAutomatically());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(failed.count(), 0);
    QVERIFY(!checker.lastError().isEmpty());

    QVERIFY(checker.checkManually());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(failed.count(), 1);
}

void AppUpdaterTest::offlineIsSilent()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();
    const QUrl endpoint = server.url();
    server.close();

    pictura::UpdateChecker checker;
    pictura::UpdateCheckSettings settings =
        settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0"));
    settings.endpoint = endpoint;
    checker.setSettings(settings);
    QSignalSpy failed(&checker, &pictura::UpdateChecker::failed);

    QVERIFY(checker.checkAutomatically());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(failed.count(), 0);
    QVERIFY(!checker.lastError().isEmpty());

    QVERIFY(checker.checkManually());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(failed.count(), 1);
}

void AppUpdaterTest::manualCheckWorksWhenAutoOff()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();

    pictura::UpdateChecker checker;
    checker.setSettings(
        settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0"), false));
    QSignalSpy available(&checker, &pictura::UpdateChecker::updateAvailable);

    QVERIFY(checker.checkManually());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(available.count(), 1);
    QCOMPARE(server.requestCount(), 1);
}

void AppUpdaterTest::manualCheckUpToDate()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = releaseFixture();

    pictura::UpdateChecker checker;
    checker.setSettings(settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.4.0")));
    QSignalSpy upToDate(&checker, &pictura::UpdateChecker::upToDate);
    QSignalSpy available(&checker, &pictura::UpdateChecker::updateAvailable);

    QVERIFY(checker.checkManually());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(upToDate.count(), 1);
    QCOMPARE(available.count(), 0);
}

void AppUpdaterTest::manualCheckError()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.status = 500;
    server.body = QByteArrayLiteral("{}");

    pictura::UpdateChecker checker;
    checker.setSettings(settingsFor(server, PackageMethod::AppImage, QStringLiteral("0.3.0")));
    QSignalSpy failed(&checker, &pictura::UpdateChecker::failed);

    QVERIFY(checker.checkManually());
    QVERIFY(waitForIdle(checker));
    QCOMPARE(failed.count(), 1);
    QVERIFY(!failed.first().at(0).toString().isEmpty());
}

void AppUpdaterTest::acceptsMatchingDigest()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = QByteArrayLiteral("verified update payload");
    QTemporaryDir dir;
    QVERIFY(dir.isValid());

    pictura::ReleaseAsset asset;
    asset.name = QStringLiteral("update.AppImage");
    asset.downloadUrl = server.url().toString();
    asset.size = server.body.size();
    asset.sha256 = sha256Hex(server.body);

    pictura::UpdateInstaller installer;
    QSignalSpy downloaded(&installer, &pictura::UpdateInstaller::downloaded);
    QSignalSpy failed(&installer, &pictura::UpdateInstaller::failed);
    QVERIFY(installer.startDownload(asset, dir.path()));
    QVERIFY(waitForInstaller(installer));
    QCOMPARE(failed.count(), 0);
    QTRY_COMPARE(downloaded.count(), 1);
    const QString path = downloaded.first().at(0).toString();
    QCOMPARE(readFile(path), server.body);
}

void AppUpdaterTest::refusesMissingDigest()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = QByteArrayLiteral("payload");
    QTemporaryDir dir;
    QVERIFY(dir.isValid());

    pictura::ReleaseAsset asset;
    asset.name = QStringLiteral("update.AppImage");
    asset.downloadUrl = server.url().toString();
    asset.size = server.body.size();

    pictura::UpdateInstaller installer;
    QSignalSpy failed(&installer, &pictura::UpdateInstaller::failed);
    QVERIFY(!installer.startDownload(asset, dir.path()));
    QCOMPARE(server.requestCount(), 0);
    QCOMPARE(failed.count(), 1);
}

void AppUpdaterTest::refusesMismatchedDigest()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = QByteArrayLiteral("payload");
    QTemporaryDir dir;
    QVERIFY(dir.isValid());

    pictura::ReleaseAsset asset;
    asset.name = QStringLiteral("update.AppImage");
    asset.downloadUrl = server.url().toString();
    asset.size = server.body.size();
    asset.sha256 = QString(64, QLatin1Char('0'));

    pictura::UpdateInstaller installer;
    QSignalSpy downloaded(&installer, &pictura::UpdateInstaller::downloaded);
    QSignalSpy failed(&installer, &pictura::UpdateInstaller::failed);
    QVERIFY(installer.startDownload(asset, dir.path()));
    QVERIFY(waitForInstaller(installer));
    QCOMPARE(downloaded.count(), 0);
    QCOMPARE(failed.count(), 1);
    QCOMPARE(QDir(dir.path()).entryList(QDir::Files | QDir::Hidden).size(), 0);
}

void AppUpdaterTest::refusesOversizeDownload()
{
    TestReleaseServer server;
    QVERIFY(server.start());
    server.body = QByteArrayLiteral("small");
    server.declaredContentLength = 600LL * 1024 * 1024;
    QTemporaryDir dir;
    QVERIFY(dir.isValid());

    pictura::ReleaseAsset asset;
    asset.name = QStringLiteral("update.AppImage");
    asset.downloadUrl = server.url().toString();
    asset.size = server.declaredContentLength;
    asset.sha256 = sha256Hex(server.body);

    pictura::UpdateInstaller installer;
    QSignalSpy failed(&installer, &pictura::UpdateInstaller::failed);
    QVERIFY(installer.startDownload(asset, dir.path()));
    QVERIFY(waitForInstaller(installer));
    QCOMPARE(failed.count(), 1);
}

void AppUpdaterTest::appImageReplacesAndRelaunches()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString appImage = dir.filePath(QStringLiteral("KookaPictura.AppImage"));
    QVERIFY(writeFile(appImage, QByteArrayLiteral("old image")));
    const QString downloaded = dir.filePath(QStringLiteral("downloaded.AppImage"));
    QVERIFY(writeFile(downloaded, QByteArrayLiteral("new image")));

    pictura::UpdateInstaller installer;
    QString restarted;
    installer.setRestartHandler([&restarted](const QString& path) {
        restarted = path;
        return true;
    });
    const pictura::UpdateInstallOutcome outcome =
        installer.install(pictura::PackageMethod::AppImage, downloaded, appImage);
    QVERIFY(outcome.ok);
    QVERIFY(outcome.quit);
    QCOMPARE(restarted, appImage);
    QCOMPARE(readFile(appImage), QByteArrayLiteral("new image"));
    QCOMPARE(readFile(appImage + QStringLiteral(".old")), QByteArrayLiteral("old image"));
}

void AppUpdaterTest::installFailureKeepsAppRunning()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString appImage = dir.filePath(QStringLiteral("KookaPictura.AppImage"));
    QVERIFY(writeFile(appImage, QByteArrayLiteral("old image")));
    const QString downloaded = dir.filePath(QStringLiteral("downloaded.AppImage"));
    QVERIFY(writeFile(downloaded, QByteArrayLiteral("new image")));

    pictura::UpdateInstaller installer;
    installer.setRestartHandler([](const QString&) { return false; });
    const pictura::UpdateInstallOutcome outcome =
        installer.install(pictura::PackageMethod::AppImage, downloaded, appImage);
    QVERIFY(!outcome.ok);
    QVERIFY(!outcome.error.isEmpty());
    QCOMPARE(readFile(appImage), QByteArrayLiteral("old image"));
}

void AppUpdaterTest::savePromptRunsBeforeInstall()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Gate"), 32, 32, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(view != nullptr);
    QVERIFY(window->saveActiveAs(dir.filePath(QStringLiteral("gate.psd"))));
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(8.0, 8.0, 1.0));
    QVERIFY(view->end_paint());
    QVERIFY(pictura::document_api::isDirty(*view));

    pictura::setUnsavedPromptInteractive(false);
    pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Save);
    QVERIFY(window->saveAllDocumentsForRestart());
    QVERIFY(!pictura::document_api::isDirty(*view));
}

void AppUpdaterTest::cancelSaveAbortsInstall()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Gate"), 32, 32, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(view != nullptr);
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(8.0, 8.0, 1.0));
    QVERIFY(view->end_paint());
    QVERIFY(pictura::document_api::isDirty(*view));

    pictura::setUnsavedPromptInteractive(false);
    pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Cancel);
    QVERIFY(!window->saveAllDocumentsForRestart());
    QVERIFY(pictura::document_api::isDirty(*view));
}

void AppUpdaterTest::saveFailureAbortsInstall()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);
    QVERIFY(window->newDocument(QStringLiteral("Gate"), 32, 32, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(view != nullptr);
    const QString path = dir.filePath(QStringLiteral("gate.psd"));
    QVERIFY(window->saveActiveAs(path));
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 16, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(8.0, 8.0, 1.0));
    QVERIFY(view->end_paint());
    QVERIFY(pictura::document_api::isDirty(*view));

    QDir(dir.path()).removeRecursively();
    pictura::setUnsavedPromptInteractive(false);
    pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Save);
    QVERIFY(!window->saveAllDocumentsForRestart());
    QVERIFY(pictura::document_api::isDirty(*view));
}

QTEST_MAIN(AppUpdaterTest)
#include "tst_app_updater.moc"
