#include "update_checker.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QSysInfo>
#include <QtCore/QVariant>
#include <QtCore/QtGlobal>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>

#include "update_version.h"

namespace pictura {

namespace {

bool endpointAllowed(const QUrl& url)
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

bool processModeDisallowsNetwork()
{
    for (const QString& argument : QCoreApplication::arguments()) {
        if (argument == QStringLiteral("--headless") || argument == QStringLiteral("--self-test")
            || argument == QStringLiteral("--dump-theme") || argument == QStringLiteral("--mcp")
            || argument == QStringLiteral("--gpu-probe")) {
            return true;
        }
    }
    return false;
}

} // namespace

UpdateCheckSettings updateCheckSettingsFromEnvironment(bool automaticChecksEnabled,
                                                        PackageMethod packageMethod,
                                                        const QString& currentVersion)
{
    UpdateCheckSettings settings;
    settings.automaticChecksEnabled = automaticChecksEnabled
        && !qEnvironmentVariableIsSet("PICTURA_DISABLE_UPDATE_CHECK");
    settings.networkAllowed = !processModeDisallowsNetwork();
    settings.packageMethod = packageMethod;
    settings.currentVersion = currentVersion;
    const QString endpoint = qEnvironmentVariable("PICTURA_UPDATE_URL");
    if (!endpoint.isEmpty()) {
        settings.endpoint = QUrl(endpoint);
    }
    return settings;
}

UpdateChecker::UpdateChecker(QObject* parent)
    : QObject(parent)
    , manager_(new QNetworkAccessManager(this))
{
}

UpdateChecker::~UpdateChecker() = default;

void UpdateChecker::setSettings(const UpdateCheckSettings& settings)
{
    settings_ = settings;
}

bool UpdateChecker::checkAutomatically()
{
    if (automaticCheckStarted_ || checking_) {
        return false;
    }
    if (!settings_.automaticChecksEnabled || !settings_.networkAllowed
        || settings_.packageMethod == PackageMethod::Development) {
        return false;
    }
    automaticCheckStarted_ = true;
    return start(false);
}

bool UpdateChecker::checkManually()
{
    if (checking_ || !settings_.networkAllowed) {
        return false;
    }
    return start(true);
}

bool UpdateChecker::start(bool manual)
{
    if (!endpointAllowed(settings_.endpoint)) {
        lastError_ = QStringLiteral("The update endpoint is not an HTTPS URL.");
        if (manual) {
            emit failed(lastError_, true);
        }
        return false;
    }

    release_.reset();
    asset_.reset();
    lastError_.clear();
    checking_ = true;
    emit checkingChanged(true);

    QNetworkRequest request(settings_.endpoint);
    request.setHeader(QNetworkRequest::UserAgentHeader,
                      QStringLiteral("KookaPictura/%1").arg(settings_.currentVersion));
    request.setRawHeader(QByteArrayLiteral("Accept"),
                         QByteArrayLiteral("application/vnd.github+json"));
    request.setRawHeader(QByteArrayLiteral("X-GitHub-Api-Version"),
                         QByteArrayLiteral("2022-11-28"));
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute,
                         QNetworkRequest::NoLessSafeRedirectPolicy);
    request.setTransferTimeout(10000);

    QNetworkReply* reply = manager_->get(request);
    connect(reply, &QNetworkReply::finished, this,
            [this, reply, manual]() { handleReply(reply, manual); });
    return true;
}

void UpdateChecker::handleReply(QNetworkReply* reply, bool manual)
{
    const QByteArray body = reply->readAll();
    const QVariant statusValue = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute);
    const int status = statusValue.isValid() ? statusValue.toInt() : 0;
    const QNetworkReply::NetworkError error = reply->error();

    if (error == QNetworkReply::NoError && status >= 200 && status < 300) {
        const std::optional<ReleaseInfo> release = parseReleaseJson(body);
        if (!release) {
            lastError_ = QStringLiteral("The update response could not be read.");
        } else {
            release_ = release;
            lastError_.clear();
            if (!isNewerVersion(release->version, settings_.currentVersion)) {
                emit upToDate(manual);
                finishReply(reply);
                return;
            }
            const std::optional<ReleaseAsset> asset = selectReleaseAsset(
                *release, settings_.packageMethod, QSysInfo::currentCpuArchitecture());
            if (asset) {
                asset_ = asset;
            }
            emit updateAvailable(manual);
            finishReply(reply);
            return;
        }
    } else if (status == 403 || status == 429) {
        lastError_ = QStringLiteral("GitHub rate limit reached. Try again later.");
    } else if (error != QNetworkReply::NoError) {
        lastError_ = reply->errorString();
    } else {
        lastError_ = QStringLiteral("The update server returned HTTP %1.").arg(status);
    }

    if (manual) {
        emit failed(lastError_, true);
    }
    finishReply(reply);
}

void UpdateChecker::finishReply(QNetworkReply* reply)
{
    reply->deleteLater();
    checking_ = false;
    emit checkingChanged(false);
}

} // namespace pictura

#include "moc_update_checker.cpp"
