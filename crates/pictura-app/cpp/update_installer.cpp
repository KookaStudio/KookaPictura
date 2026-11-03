#include "update_installer.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QProcess>
#include <QtCore/QStandardPaths>
#include <QtCore/QTemporaryFile>
#include <QtCore/QUrl>
#include <QtCore/QtGlobal>
#include <QtGui/QDesktopServices>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>

#include <utility>

namespace pictura {

namespace {

constexpr qint64 kMaxDownloadBytes = 512LL * 1024 * 1024;

bool downloadUrlAllowed(const QUrl& url)
{
    if (!url.isValid()) {
        return false;
    }
    if (url.scheme() == QStringLiteral("https")) {
        return true;
    }
    if (url.scheme() != QStringLiteral("http")) {
        return false;
    }
    const QString host = url.host();
    return host == QStringLiteral("127.0.0.1") || host == QStringLiteral("localhost")
        || host == QStringLiteral("::1") || host == QStringLiteral("[::1]");
}

QString shellQuote(const QString& value)
{
    QString escaped = value;
    escaped.replace(QLatin1Char('\''), QStringLiteral("'\\''"));
    return QLatin1Char('\'') + escaped + QLatin1Char('\'');
}

} // namespace

UpdateInstaller::UpdateInstaller(QObject* parent)
    : QObject(parent)
    , manager_(new QNetworkAccessManager(this))
{
}

UpdateInstaller::~UpdateInstaller()
{
    if (reply_) {
        reply_->abort();
        reply_->deleteLater();
    }
    cleanupTemp();
}

void UpdateInstaller::setRestartHandler(std::function<bool(const QString&)> handler)
{
    restart_ = std::move(handler);
}

bool UpdateInstaller::startDownload(const ReleaseAsset& asset,
                                    const QString& destinationDirectory)
{
    if (downloading_) {
        return false;
    }
    if (asset.sha256.size() != 64) {
        fail(QStringLiteral("The release has no usable checksum."));
        return false;
    }
    if (!downloadUrlAllowed(QUrl(asset.downloadUrl))) {
        fail(QStringLiteral("The update URL is not HTTPS."));
        return false;
    }
    if (destinationDirectory.isEmpty() || !QDir().mkpath(destinationDirectory)) {
        fail(QStringLiteral("The update destination is not writable."));
        return false;
    }

    temp_ = new QTemporaryFile(
        QDir(destinationDirectory).filePath(QStringLiteral(".pictura-update-XXXXXX")), this);
    if (!temp_->open()) {
        delete temp_;
        temp_ = nullptr;
        fail(QStringLiteral("The update destination is not writable."));
        return false;
    }
    temp_->setAutoRemove(false);

    asset_ = asset;
    lastError_.clear();
    hash_.reset();
    downloading_ = true;

    QNetworkRequest request(QUrl(asset.downloadUrl));
    request.setHeader(QNetworkRequest::UserAgentHeader,
                      QStringLiteral("KookaPictura/%1").arg(QStringLiteral(PICTURA_VERSION)));
    request.setRawHeader(QByteArrayLiteral("Accept"),
                         QByteArrayLiteral("application/octet-stream"));
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute,
                         QNetworkRequest::NoLessSafeRedirectPolicy);
    request.setTransferTimeout(60000);

    reply_ = manager_->get(request);
    connect(reply_, &QNetworkReply::downloadProgress, this,
            [this](qint64 received, qint64 total) { emit progress(received, total); });
    connect(reply_, &QNetworkReply::metaDataChanged, this, [this] {
        const qint64 total =
            reply_ ? reply_->header(QNetworkRequest::ContentLengthHeader).toLongLong() : 0;
        if (total > kMaxDownloadBytes) {
            fail(QStringLiteral("The update is larger than 512 MiB."));
        }
    });
    connect(reply_, &QNetworkReply::readyRead, this, [this] {
        if (!reply_ || !temp_) {
            return;
        }
        const QByteArray chunk = reply_->readAll();
        if (chunk.isEmpty()) {
            return;
        }
        if (temp_->size() + chunk.size() > kMaxDownloadBytes) {
            fail(QStringLiteral("The update is larger than 512 MiB."));
            return;
        }
        if (temp_->write(chunk) != chunk.size()) {
            fail(QStringLiteral("The update could not be written."));
            return;
        }
        hash_.addData(chunk);
    });
    connect(reply_, &QNetworkReply::finished, this, [this] {
        if (!reply_ || !downloading_) {
            return;
        }
        if (reply_->error() != QNetworkReply::NoError) {
            fail(reply_->errorString());
            return;
        }
        const qint64 declared =
            reply_->header(QNetworkRequest::ContentLengthHeader).toLongLong();
        if (declared > kMaxDownloadBytes) {
            fail(QStringLiteral("The update is larger than 512 MiB."));
            return;
        }
        if (!temp_) {
            fail(QStringLiteral("The update could not be written."));
            return;
        }
        temp_->flush();
        temp_->close();
        const QString path = temp_->fileName();
        const QString digest = QString::fromLatin1(hash_.result().toHex());
        if (digest.compare(asset_.sha256, Qt::CaseInsensitive) != 0) {
            fail(QStringLiteral("The downloaded update does not match its checksum."));
            return;
        }
        downloading_ = false;
        delete temp_;
        temp_ = nullptr;
        reply_->deleteLater();
        reply_ = nullptr;
        emit downloaded(path);
    });
    return true;
}

void UpdateInstaller::cancel()
{
    if (!downloading_) {
        return;
    }
    downloading_ = false;
    QNetworkReply* reply = reply_;
    reply_ = nullptr;
    if (reply) {
        reply->abort();
        reply->deleteLater();
    }
    cleanupTemp();
    emit cancelled();
}

UpdateInstallOutcome UpdateInstaller::install(PackageMethod method, const QString& downloadedPath,
                                              const QString& appImagePath)
{
    UpdateInstallOutcome outcome;
    if (method == PackageMethod::AppImage) {
        if (appImagePath.isEmpty() || !QFileInfo::exists(appImagePath)) {
            outcome.error = QStringLiteral("The running AppImage could not be found.");
            return outcome;
        }
        const QString oldPath = appImagePath + QStringLiteral(".old");
        QFile::remove(oldPath);
        if (!QFile::rename(appImagePath, oldPath)) {
            outcome.error = QStringLiteral("The running AppImage could not be replaced.");
            return outcome;
        }
        if (!QFile::rename(downloadedPath, appImagePath)) {
            QFile::rename(oldPath, appImagePath);
            outcome.error = QStringLiteral("The new AppImage could not be put in place.");
            return outcome;
        }
        const QFile::Permissions executable =
            QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner | QFile::ReadGroup
            | QFile::ExeGroup | QFile::ReadOther | QFile::ExeOther;
        if (!QFile::setPermissions(appImagePath, executable)) {
            QFile::remove(appImagePath);
            QFile::rename(oldPath, appImagePath);
            outcome.error = QStringLiteral("The new AppImage is not executable.");
            return outcome;
        }
        if (restart_ && !restart_(appImagePath)) {
            QFile::remove(appImagePath);
            QFile::rename(oldPath, appImagePath);
            outcome.error = QStringLiteral("The new version could not be started.");
            return outcome;
        }
        outcome.ok = true;
        outcome.quit = true;
        return outcome;
    }

    if (method == PackageMethod::Deb || method == PackageMethod::Rpm) {
        if (QStandardPaths::findExecutable(QStringLiteral("pkexec")).isEmpty()) {
            outcome.error = QStringLiteral(
                "The package manager helper (pkexec) is not available. Install the package "
                "from the release page.");
            return outcome;
        }
        QString installCommand = packageManagerInstallCommand(method, downloadedPath);
        if (method == PackageMethod::Rpm
            && QStandardPaths::findExecutable(QStringLiteral("dnf")).isEmpty()) {
            if (QStandardPaths::findExecutable(QStringLiteral("rpm")).isEmpty()) {
                outcome.error = QStringLiteral(
                    "Neither dnf nor rpm is available. Install the package from the release "
                    "page.");
                return outcome;
            }
            installCommand =
                QStringLiteral("pkexec rpm -U %1").arg(shellQuote(downloadedPath));
        }
        if (installCommand.isEmpty()) {
            outcome.error = QStringLiteral("The package manager command is not available.");
            return outcome;
        }

        QString appPath = QStandardPaths::findExecutable(QStringLiteral("pictura"));
        if (appPath.isEmpty()) {
            appPath = QCoreApplication::applicationFilePath();
        }
        QString tempDir =
            QStandardPaths::writableLocation(QStandardPaths::TempLocation);
        if (tempDir.isEmpty()) {
            tempDir = QDir::tempPath();
        }
        QTemporaryFile script(
            QDir(tempDir).filePath(QStringLiteral("pictura-update-XXXXXX.sh")));
        script.setAutoRemove(false);
        if (!script.open()) {
            outcome.error = QStringLiteral("The update helper could not be written.");
            return outcome;
        }
        const QString scriptPath = script.fileName();
        const QString content =
            QStringLiteral("#!/bin/sh\n"
                           "if %1; then\n"
                           "  nohup %2 >/dev/null 2>&1 &\n"
                           "fi\n"
                           "rm -f %3 %4\n")
                .arg(installCommand, shellQuote(appPath), shellQuote(scriptPath),
                     shellQuote(downloadedPath));
        script.write(content.toUtf8());
        script.close();
        QFile::setPermissions(scriptPath,
                              QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner);
        if (!QProcess::startDetached(QStringLiteral("/bin/sh"), {scriptPath})) {
            outcome.error = QStringLiteral("The package manager could not be started.");
            return outcome;
        }
        outcome.ok = true;
        outcome.quit = true;
        return outcome;
    }

    if (method == PackageMethod::WindowsSetup) {
        if (!restart_ || !restart_(downloadedPath)) {
            outcome.error = QStringLiteral("The installer could not be started.");
            return outcome;
        }
        outcome.ok = true;
        outcome.quit = true;
        return outcome;
    }

    if (method == PackageMethod::MacDmg) {
        if (!QDesktopServices::openUrl(QUrl::fromLocalFile(downloadedPath))) {
            outcome.error = QStringLiteral("The disk image could not be opened.");
            return outcome;
        }
        outcome.ok = true;
        return outcome;
    }

    outcome.error = QStringLiteral("This build does not install updates itself.");
    return outcome;
}

void UpdateInstaller::fail(const QString& message)
{
    downloading_ = false;
    lastError_ = message;
    QNetworkReply* reply = reply_;
    reply_ = nullptr;
    if (reply) {
        reply->abort();
        reply->deleteLater();
    }
    cleanupTemp();
    emit failed(message);
}

void UpdateInstaller::cleanupTemp()
{
    if (!temp_) {
        return;
    }
    temp_->close();
    temp_->remove();
    delete temp_;
    temp_ = nullptr;
}

} // namespace pictura

#include "moc_update_installer.cpp"
