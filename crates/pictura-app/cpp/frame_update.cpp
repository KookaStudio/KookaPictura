#include "frame_update.h"

#include "frame_includes.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QProcess>
#include <QtCore/QStandardPaths>
#include <QtCore/QTimer>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QTextBrowser>
#include <QtWidgets/QVBoxLayout>

#include "command_ids.h"
#include "update_banner.h"
#include "update_checker.h"
#include "update_installer.h"
#include "update_package.h"
#include "update_preferences_page.h"
#include "update_release.h"

namespace pictura {

namespace {

void beginUpdate(PicturaMainWindow& frame);
void cancelUpdate(PicturaMainWindow& frame);
void openUpdateReleasePage(PicturaMainWindow& frame);
void showUpdateReleaseNotes(PicturaMainWindow& frame);
void setAutomaticUpdateChecks(PicturaMainWindow& frame, bool on);
void showUpdateBanner(PicturaMainWindow& frame);

UpdateBanner* ensureUpdateBanner(PicturaMainWindow& frame)
{
    if (auto* existing = frame.findChild<UpdateBanner*>(QStringLiteral("updateBanner"))) {
        return existing;
    }
    auto* banner = new UpdateBanner(&frame);
    banner->setObjectName(QStringLiteral("updateBanner"));
    QObject::connect(banner, &UpdateBanner::updateRequested, &frame,
                     [&frame] { beginUpdate(frame); });
    QObject::connect(banner, &UpdateBanner::releaseNotesRequested, &frame,
                     [&frame] { showUpdateReleaseNotes(frame); });
    QObject::connect(banner, &UpdateBanner::releasePageRequested, &frame,
                     [&frame] { openUpdateReleasePage(frame); });
    QObject::connect(banner, &UpdateBanner::automaticChecksChanged, &frame,
                     [&frame](bool on) { setAutomaticUpdateChecks(frame, on); });
    QObject::connect(banner, &UpdateBanner::cancelRequested, &frame,
                     [&frame] { cancelUpdate(frame); });
    return banner;
}

void showUpdateBanner(PicturaMainWindow& frame)
{
    if (QApplication::activeModalWidget()) {
        QTimer::singleShot(500, &frame, [&frame] { showUpdateBanner(frame); });
        return;
    }
    auto* checker = frame.findChild<UpdateChecker*>(QStringLiteral("updateChecker"));
    if (!checker || !checker->release()) {
        return;
    }
    UpdateBanner* banner = ensureUpdateBanner(frame);
    const ReleaseInfo& release = *checker->release();
    const PackageMethod method = checker->settings().packageMethod;
    const bool canInstall = checker->asset().has_value() && packageMethodSelfUpdates(method);
    banner->setAutomaticChecks(checker->settings().automaticChecksEnabled);
    banner->showUpdate(QStringLiteral(PICTURA_VERSION), release.version, canInstall,
                       packageUpdateCommand(method, release.version));
}

QString appImagePath()
{
    const QString path = qEnvironmentVariable("APPIMAGE");
    if (path.isEmpty()) {
        return {};
    }
    const QFileInfo info(path);
    const QString canonical = info.canonicalFilePath();
    return canonical.isEmpty() ? info.absoluteFilePath() : canonical;
}

void beginUpdate(PicturaMainWindow& frame)
{
    auto* checker = frame.findChild<UpdateChecker*>(QStringLiteral("updateChecker"));
    if (!checker || !checker->release()) {
        return;
    }
    const PackageMethod method = checker->settings().packageMethod;
    if (!checker->asset() || !packageMethodSelfUpdates(method)) {
        openUpdateReleasePage(frame);
        return;
    }
    if (QApplication::activeModalWidget()) {
        QMessageBox::information(&frame, QObject::tr("Update"),
                                 QObject::tr("Finish or cancel the open dialog first."));
        return;
    }
    if (!frame.saveAllDocumentsForRestart()) {
        return;
    }

    const QString imagePath = appImagePath();
    QString destination;
    if (method == PackageMethod::AppImage) {
        if (imagePath.isEmpty() || !QFileInfo::exists(imagePath)) {
            if (auto* banner = frame.findChild<UpdateBanner*>(QStringLiteral("updateBanner"))) {
                banner->showError(QObject::tr("The running AppImage could not be found."));
            }
            return;
        }
        destination = QFileInfo(imagePath).absolutePath();
    } else {
        destination = QStandardPaths::writableLocation(QStandardPaths::TempLocation);
    }

    auto* installer = frame.findChild<UpdateInstaller*>(QStringLiteral("updateInstaller"));
    if (!installer) {
        installer = new UpdateInstaller(&frame);
        installer->setObjectName(QStringLiteral("updateInstaller"));
        installer->setRestartHandler([](const QString& path) {
            return QProcess::startDetached(path, {});
        });
        QObject::connect(installer, &UpdateInstaller::progress, &frame,
                         [&frame](qint64 received, qint64 total) {
                             if (auto* banner = frame.findChild<UpdateBanner*>(
                                     QStringLiteral("updateBanner"))) {
                                 banner->showProgress(received, total);
                             }
                         });
        QObject::connect(installer, &UpdateInstaller::downloaded, &frame,
                         [&frame, method](const QString& path) {
                             auto* active = frame.findChild<UpdateInstaller*>(
                                 QStringLiteral("updateInstaller"));
                             if (!active) {
                                 return;
                             }
                             const UpdateInstallOutcome outcome =
                                 active->install(method, path, appImagePath());
                             if (!outcome.ok) {
                                 if (auto* banner = frame.findChild<UpdateBanner*>(
                                         QStringLiteral("updateBanner"))) {
                                     banner->showError(outcome.error);
                                 }
                                 return;
                             }
                             if (outcome.quit) {
                                 QCoreApplication::quit();
                                 return;
                             }
                             QMessageBox box(&frame);
                             box.setWindowTitle(QObject::tr("Update"));
                             box.setText(QObject::tr(
                                 "Drag Kooka Pictura to Applications, then relaunch it."));
                             box.setIcon(QMessageBox::Information);
                             auto* quit = box.addButton(QObject::tr("Quit Now"),
                                                        QMessageBox::AcceptRole);
                             box.addButton(QObject::tr("Later"), QMessageBox::RejectRole);
                             box.exec();
                             if (box.clickedButton() == quit) {
                                 QCoreApplication::quit();
                             }
                         });
        QObject::connect(installer, &UpdateInstaller::failed, &frame,
                         [&frame](const QString& message) {
                             if (auto* banner = frame.findChild<UpdateBanner*>(
                                     QStringLiteral("updateBanner"))) {
                                 banner->showError(message);
                             }
                         });
        QObject::connect(installer, &UpdateInstaller::cancelled, &frame,
                         [&frame] { showUpdateBanner(frame); });
    }
    if (installer->isDownloading()) {
        return;
    }
    if (auto* banner = frame.findChild<UpdateBanner*>(QStringLiteral("updateBanner"))) {
        banner->showProgress(0, checker->asset()->size);
    }
    if (!installer->startDownload(*checker->asset(), destination)) {
        if (auto* banner = frame.findChild<UpdateBanner*>(QStringLiteral("updateBanner"))) {
            banner->showError(installer->lastError());
        }
    }
}

void cancelUpdate(PicturaMainWindow& frame)
{
    if (auto* installer = frame.findChild<UpdateInstaller*>(QStringLiteral("updateInstaller"))) {
        installer->cancel();
    }
    auto* checker = frame.findChild<UpdateChecker*>(QStringLiteral("updateChecker"));
    if (checker && checker->release()) {
        showUpdateBanner(frame);
    }
}

void openUpdateReleasePage(PicturaMainWindow& frame)
{
    auto* checker = frame.findChild<UpdateChecker*>(QStringLiteral("updateChecker"));
    if (!checker || !checker->release()) {
        return;
    }
    QDesktopServices::openUrl(QUrl(checker->release()->htmlUrl));
}

void showUpdateReleaseNotes(PicturaMainWindow& frame)
{
    auto* checker = frame.findChild<UpdateChecker*>(QStringLiteral("updateChecker"));
    if (!checker || !checker->release()) {
        return;
    }
    QDialog dialog(&frame);
    dialog.setWindowTitle(QObject::tr("Release Notes"));
    dialog.resize(640, 480);
    auto* layout = new QVBoxLayout(&dialog);
    auto* browser = new QTextBrowser(&dialog);
    browser->setOpenExternalLinks(true);
    browser->setMarkdown(checker->release()->body);
    layout->addWidget(browser);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);
    dialog.exec();
}

void setAutomaticUpdateChecks(PicturaMainWindow& frame, bool on)
{
    auto* checker = ensureUpdateChecker(frame, true);
    UpdateCheckSettings settings = checker->settings();
    settings.automaticChecksEnabled = on;
    checker->setSettings(settings);
    frame.saveSessionNow();
    if (auto* banner = frame.findChild<UpdateBanner*>(QStringLiteral("updateBanner"))) {
        banner->setAutomaticChecks(on);
    }
    if (auto* dialog = frame.preferencesDialog()) {
        if (auto* page = dialog->findChild<UpdatePreferencesPage*>()) {
            page->setAutomaticChecks(on);
        }
    }
}

void checkForUpdatesManually(PicturaMainWindow& frame)
{
    auto* checker = ensureUpdateChecker(frame, loadSession().checkForUpdatesOnLaunch);
    if (!checker->settings().networkAllowed) {
        frame.statusBar()->showMessage(
            QObject::tr("Update checks are not available in this run."), 5000);
        return;
    }
    if (!checker->checkManually() && checker->isChecking()) {
        frame.statusBar()->showMessage(QObject::tr("An update check is already running."), 5000);
    }
}

} // namespace

UpdateChecker* ensureUpdateChecker(PicturaMainWindow& frame, bool checkForUpdatesOnLaunch)
{
    if (auto* existing = frame.findChild<UpdateChecker*>(QStringLiteral("updateChecker"))) {
        return existing;
    }
    auto* checker = new UpdateChecker(&frame);
    checker->setObjectName(QStringLiteral("updateChecker"));
    checker->setSettings(updateCheckSettingsFromEnvironment(
        checkForUpdatesOnLaunch, detectPackageMethod(), QStringLiteral(PICTURA_VERSION)));
    QObject::connect(checker, &UpdateChecker::updateAvailable, &frame,
                     [&frame](bool) { showUpdateBanner(frame); });
    QObject::connect(checker, &UpdateChecker::upToDate, &frame, [&frame](bool manual) {
        if (manual) {
            frame.statusBar()->showMessage(QObject::tr("Kooka Pictura is up to date."), 5000);
        }
    });
    QObject::connect(checker, &UpdateChecker::failed, &frame,
                     [&frame](const QString& message, bool manual) {
                         if (manual) {
                             frame.statusBar()->showMessage(message, 8000);
                         }
                     });
    return checker;
}

void scheduleUpdateCheck(PicturaMainWindow& frame)
{
    if (!frame.updateChecksEnabled()) {
        return;
    }
    ensureUpdateChecker(frame, loadSession().checkForUpdatesOnLaunch)->checkAutomatically();
}

void registerUpdateHandlers(PicturaMainWindow& frame)
{
    frame.registry()->setHandler(command_ids::HelpCheckForUpdates,
                                 [&frame] { checkForUpdatesManually(frame); });
}

UpdatePreferencesPage* makeUpdatePreferencesPage(PicturaMainWindow& frame, QWidget* parent)
{
    auto* checker = ensureUpdateChecker(frame, loadSession().checkForUpdatesOnLaunch);
    auto* page = new UpdatePreferencesPage(checker, parent);
    QObject::connect(page, &UpdatePreferencesPage::automaticChecksChanged, &frame,
                     [&frame](bool on) { setAutomaticUpdateChecks(frame, on); });
    QObject::connect(page, &UpdatePreferencesPage::checkRequested, &frame,
                     [&frame] { checkForUpdatesManually(frame); });
    return page;
}

QString aboutVersionText()
{
    return QObject::tr("Kooka Pictura %1\nPackage: %2")
        .arg(QStringLiteral(PICTURA_VERSION), packageMethodName(detectPackageMethod()));
}

} // namespace pictura
