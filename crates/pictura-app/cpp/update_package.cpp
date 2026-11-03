#include "update_package.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QProcess>
#include <QtCore/QProcessEnvironment>
#include <QtCore/QSettings>
#include <QtCore/QStandardPaths>
#include <QtCore/QtGlobal>

namespace pictura {

namespace {

QString canonicalOrClean(const QString& path)
{
    if (path.isEmpty()) {
        return {};
    }
    const QString canonical = QFileInfo(path).canonicalFilePath();
    return canonical.isEmpty() ? QDir::cleanPath(path) : canonical;
}

bool isUnderInstallPrefix(const QString& path)
{
    return path == QStringLiteral("/opt/kooka-pictura")
        || path.startsWith(QStringLiteral("/opt/kooka-pictura/"));
}

bool isX86_64(const QString& architecture)
{
    const QString lower = architecture.toLower();
    return lower == QStringLiteral("x86_64") || lower == QStringLiteral("amd64")
        || lower == QStringLiteral("x64");
}

} // namespace

PackageMethod detectPackageMethod(const PackageDetectionInput& input)
{
    if (!input.environment.value(QStringLiteral("FLATPAK_ID")).isEmpty()) {
        return PackageMethod::Flatpak;
    }
    if (!input.environment.value(QStringLiteral("SNAP")).isEmpty()) {
        return PackageMethod::Snap;
    }
    if (!input.environment.value(QStringLiteral("APPIMAGE")).isEmpty()) {
        return PackageMethod::AppImage;
    }

    const QString os = input.os.toLower();
    if (os == QStringLiteral("windows")) {
        return input.windowsInstallMarker ? PackageMethod::WindowsSetup
                                          : PackageMethod::WindowsPortable;
    }
    if (os == QStringLiteral("macos")) {
        const QString path = canonicalOrClean(input.executablePath);
        return path.contains(QStringLiteral(".app/Contents/MacOS/")) ? PackageMethod::MacDmg
                                                                     : PackageMethod::Development;
    }

    const QString path = canonicalOrClean(input.executablePath);
    if (isUnderInstallPrefix(path)) {
        if (input.debPackageInstalled) {
            return PackageMethod::Deb;
        }
        if (input.rpmPackageInstalled) {
            return PackageMethod::Rpm;
        }
    }
    return PackageMethod::Development;
}

PackageMethod detectPackageMethod()
{
    PackageDetectionInput input;
#ifdef Q_OS_WIN
    input.os = QStringLiteral("windows");
#elif defined(Q_OS_MACOS)
    input.os = QStringLiteral("macos");
#else
    input.os = QStringLiteral("linux");
#endif
    input.executablePath = QCoreApplication::applicationFilePath();
    const QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
    for (const QString& key : environment.keys()) {
        input.environment.insert(key, environment.value(key));
    }

#ifdef Q_OS_WIN
    QSettings settings(QStringLiteral("HKEY_LOCAL_MACHINE\\Software\\KookaPictura"),
                       QSettings::NativeFormat);
    const QString installDir = settings.value(QStringLiteral("InstallDir")).toString();
    if (!installDir.isEmpty()) {
        const QString executable = QDir::cleanPath(input.executablePath);
        const QString prefix = QDir::cleanPath(installDir) + QLatin1Char('/');
        input.windowsInstallMarker = executable.startsWith(prefix);
    }
#endif

#ifdef Q_OS_LINUX
    const QString executable = canonicalOrClean(input.executablePath);
    if (isUnderInstallPrefix(executable)) {
        input.debPackageInstalled =
            QFileInfo(QStringLiteral("/var/lib/dpkg/info/kooka-pictura.list")).isFile();
        if (!input.debPackageInstalled) {
            const QString rpm = QStandardPaths::findExecutable(QStringLiteral("rpm"));
            if (!rpm.isEmpty()) {
                QProcess process;
                process.start(rpm, {QStringLiteral("-q"), QStringLiteral("kooka-pictura")});
                input.rpmPackageInstalled =
                    process.waitForFinished(500) && process.exitStatus() == QProcess::NormalExit
                    && process.exitCode() == 0;
            }
        }
    }
#endif

    return detectPackageMethod(input);
}

QString packageMethodName(PackageMethod method)
{
    switch (method) {
    case PackageMethod::AppImage:
        return QStringLiteral("AppImage");
    case PackageMethod::Flatpak:
        return QStringLiteral("Flatpak");
    case PackageMethod::Snap:
        return QStringLiteral("Snap");
    case PackageMethod::Deb:
        return QStringLiteral("deb package");
    case PackageMethod::Rpm:
        return QStringLiteral("rpm package");
    case PackageMethod::WindowsSetup:
        return QStringLiteral("Windows installer");
    case PackageMethod::WindowsPortable:
        return QStringLiteral("Windows portable");
    case PackageMethod::MacDmg:
        return QStringLiteral("macOS DMG");
    case PackageMethod::Development:
        return QStringLiteral("Development build");
    }
    return {};
}

bool packageMethodSelfUpdates(PackageMethod method)
{
    return method == PackageMethod::AppImage || method == PackageMethod::WindowsSetup
        || method == PackageMethod::MacDmg;
}

bool packageMethodUsesPackageManager(PackageMethod method)
{
    return method == PackageMethod::Flatpak || method == PackageMethod::Snap
        || method == PackageMethod::Deb || method == PackageMethod::Rpm;
}

QString packageAssetSuffix(PackageMethod method, const QString& architecture)
{
    if (method == PackageMethod::MacDmg) {
        return QStringLiteral("-macOS.dmg");
    }
    if (method == PackageMethod::WindowsSetup || method == PackageMethod::WindowsPortable) {
        if (!isX86_64(architecture)) {
            return {};
        }
        return method == PackageMethod::WindowsSetup
            ? QStringLiteral("-windows-x64-setup.exe")
            : QStringLiteral("-windows-x64-portable.zip");
    }
    if (method == PackageMethod::Development || !isX86_64(architecture)) {
        return {};
    }
    switch (method) {
    case PackageMethod::AppImage:
        return QStringLiteral("-x86_64.AppImage");
    case PackageMethod::Deb:
        return QStringLiteral("_amd64.deb");
    case PackageMethod::Rpm:
        return QStringLiteral("-1.x86_64.rpm");
    case PackageMethod::Snap:
        return QStringLiteral("_amd64.snap");
    case PackageMethod::Flatpak:
        return QStringLiteral("-x86_64.flatpak");
    default:
        return {};
    }
}

QString packageUpdateCommand(PackageMethod method, const QString& version)
{
    QString cleanVersion = version.trimmed();
    if (!cleanVersion.isEmpty()
        && (cleanVersion.at(0).unicode() == u'v' || cleanVersion.at(0).unicode() == u'V')) {
        cleanVersion.remove(0, 1);
    }
    switch (method) {
    case PackageMethod::Flatpak:
        return QStringLiteral("flatpak update io.github.KookaStudio.KookaPictura");
    case PackageMethod::Snap:
        return QStringLiteral("snap refresh kooka-pictura");
    case PackageMethod::Deb:
        if (cleanVersion.isEmpty()) {
            return {};
        }
        return QStringLiteral("sudo apt install ./KookaPictura_%1_amd64.deb").arg(cleanVersion);
    case PackageMethod::Rpm:
        if (cleanVersion.isEmpty()) {
            return {};
        }
        return QStringLiteral("sudo dnf install ./KookaPictura-%1-1.x86_64.rpm").arg(cleanVersion);
    default:
        return {};
    }
}

} // namespace pictura
