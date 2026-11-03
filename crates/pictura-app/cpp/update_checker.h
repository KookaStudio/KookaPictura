#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QUrl>

#include <optional>

#include "update_package.h"
#include "update_release.h"

class QNetworkAccessManager;
class QNetworkReply;

namespace pictura {

// Everything the checker needs. The frame builds this from the session, the
// process mode, the detected package method, and the running version.
struct UpdateCheckSettings {
    bool automaticChecksEnabled = true;
    bool networkAllowed = true;
    PackageMethod packageMethod = PackageMethod::Development;
    QString currentVersion;
    QUrl endpoint = QUrl(
        QStringLiteral("https://api.github.com/repos/KookaStudio/KookaPictura/releases/latest"));
};

// The production settings: the endpoint can be overridden with
// PICTURA_UPDATE_URL, automatic checks can be disabled with
// PICTURA_DISABLE_UPDATE_CHECK, and headless/test modes disallow the network.
UpdateCheckSettings updateCheckSettingsFromEnvironment(bool automaticChecksEnabled,
                                                        PackageMethod packageMethod,
                                                        const QString& currentVersion);

// One asynchronous release check. It owns no UI; the banner and Preferences
// page read the result through the accessors and the signals.
class UpdateChecker : public QObject {
    Q_OBJECT

public:
    explicit UpdateChecker(QObject* parent = nullptr);
    ~UpdateChecker() override;

    void setSettings(const UpdateCheckSettings& settings);
    const UpdateCheckSettings& settings() const { return settings_; }

    // Start the one automatic check this launch allows. Returns false when the
    // setting is off, the network is disallowed, the build is a development
    // build, a check is already running, or the automatic check already ran.
    bool checkAutomatically();

    // Start an explicit check. Allowed when automatic checks are off and for a
    // development build; refused only while another check is running or when
    // the process mode disallows the network.
    bool checkManually();

    bool isChecking() const { return checking_; }
    bool automaticCheckStarted() const { return automaticCheckStarted_; }

    const std::optional<ReleaseInfo>& release() const { return release_; }
    const std::optional<ReleaseAsset>& asset() const { return asset_; }
    QString lastError() const { return lastError_; }

signals:
    void checkingChanged(bool checking);
    void updateAvailable(bool manual);
    void upToDate(bool manual);
    // Emitted only for a manual check; an automatic failure stays silent.
    void failed(const QString& message, bool manual);

private:
    bool start(bool manual);
    void handleReply(QNetworkReply* reply, bool manual);
    void finishReply(QNetworkReply* reply);

    QNetworkAccessManager* manager_ = nullptr;
    UpdateCheckSettings settings_;
    std::optional<ReleaseInfo> release_;
    std::optional<ReleaseAsset> asset_;
    QString lastError_;
    bool checking_ = false;
    bool automaticCheckStarted_ = false;
};

} // namespace pictura
