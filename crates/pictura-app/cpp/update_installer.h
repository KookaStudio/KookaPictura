#pragma once

#include <QtCore/QCryptographicHash>
#include <QtCore/QObject>
#include <QtCore/QString>

#include <functional>

#include "update_package.h"
#include "update_release.h"

class QNetworkAccessManager;
class QNetworkReply;
class QTemporaryFile;

namespace pictura {

struct UpdateInstallOutcome {
    bool ok = false;
    bool quit = false;
    QString error;
};

// Downloads one verified release asset and installs it through the method's
// path. The restart handler is a seam for tests and for the frame's quit.
class UpdateInstaller : public QObject {
    Q_OBJECT

public:
    explicit UpdateInstaller(QObject* parent = nullptr);
    ~UpdateInstaller() override;

    void setRestartHandler(std::function<bool(const QString&)> handler);

    // Download `asset` into a temporary file in `destinationDirectory`. The
    // digest must be present; a missing or mismatched digest never installs.
    bool startDownload(const ReleaseAsset& asset, const QString& destinationDirectory);
    void cancel();
    bool isDownloading() const { return downloading_; }
    QString lastError() const { return lastError_; }

    // Install a downloaded file. AppImage replaces the running image and calls
    // the restart handler; Windows setup launches the installer; macOS opens
    // the DMG. `quit` tells the frame to close after a successful install.
    UpdateInstallOutcome install(PackageMethod method, const QString& downloadedPath,
                                 const QString& appImagePath);

signals:
    void progress(qint64 received, qint64 total);
    void downloaded(const QString& path);
    void failed(const QString& message);
    void cancelled();

private:
    void fail(const QString& message);
    void cleanupTemp();

    QNetworkAccessManager* manager_ = nullptr;
    QNetworkReply* reply_ = nullptr;
    QTemporaryFile* temp_ = nullptr;
    QCryptographicHash hash_{QCryptographicHash::Sha256};
    ReleaseAsset asset_;
    QString lastError_;
    bool downloading_ = false;
    std::function<bool(const QString&)> restart_;
};

} // namespace pictura
