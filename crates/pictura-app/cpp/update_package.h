#pragma once

#include <QtCore/QMap>
#include <QtCore/QString>

namespace pictura {

// How the running build was installed. The updater never offers a path the
// build cannot complete.
enum class PackageMethod {
    AppImage,
    Flatpak,
    Snap,
    Deb,
    Rpm,
    WindowsSetup,
    WindowsPortable,
    MacDmg,
    Development,
};

// The facts detection depends on, injected so every row is testable on any
// host. Production fills them from the environment, the executable path, the
// Windows install marker, and the package database.
struct PackageDetectionInput {
    QString os;  // "linux", "windows", or "macos"
    QString executablePath;
    QMap<QString, QString> environment;
    QString packageMarker;  // contents of the package's marker file, if any
    bool windowsInstallMarker = false;
    bool debPackageInstalled = false;
    bool rpmPackageInstalled = false;
};

PackageMethod detectPackageMethod(const PackageDetectionInput& input);

// Detect the running process's package method from the environment, the
// executable path, the Windows install marker, and the package database.
PackageMethod detectPackageMethod();

QString packageMethodName(PackageMethod method);

// True when the application can download and start the install itself.
bool packageMethodSelfUpdates(PackageMethod method);

// True when the package manager owns the installed files.
bool packageMethodUsesPackageManager(PackageMethod method);

// The release-asset suffix for a method and CPU architecture, or empty when
// that method/architecture has no asset.
QString packageAssetSuffix(PackageMethod method, const QString& architecture);

// The command a package-manager-owned build should show, or empty when the
// method has no package-manager command.
QString packageUpdateCommand(PackageMethod method, const QString& version);

// The elevated command that installs a downloaded deb or rpm package.
QString packageManagerInstallCommand(PackageMethod method, const QString& packagePath);

} // namespace pictura
