#pragma once

#include <QtCore/QString>
#include <functional>

class QWidget;

namespace pictura {

class PicturaMainWindow;
class UpdateChecker;
class UpdatePreferencesPage;

UpdateChecker* ensureUpdateChecker(PicturaMainWindow& frame, bool checkForUpdatesOnLaunch);
void scheduleUpdateCheck(PicturaMainWindow& frame);
void registerUpdateHandlers(PicturaMainWindow& frame);
UpdatePreferencesPage* makeUpdatePreferencesPage(PicturaMainWindow& frame, QWidget* parent);
bool launchAppRestart();
void setAppRestartLauncherForTest(std::function<bool(const QString&)> launcher);

// The running version and detected package method, shown at the bottom of
// Help > About.
QString aboutVersionText();

} // namespace pictura
