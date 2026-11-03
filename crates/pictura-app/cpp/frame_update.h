#pragma once

class QWidget;

namespace pictura {

class PicturaMainWindow;
class UpdateChecker;
class UpdatePreferencesPage;

UpdateChecker* ensureUpdateChecker(PicturaMainWindow& frame, bool checkForUpdatesOnLaunch);
void scheduleUpdateCheck(PicturaMainWindow& frame);
void repositionUpdateBanner(PicturaMainWindow& frame);
void registerUpdateHandlers(PicturaMainWindow& frame);
UpdatePreferencesPage* makeUpdatePreferencesPage(PicturaMainWindow& frame, QWidget* parent);

// The running version and detected package method, shown at the bottom of
// Help > About.
QString aboutVersionText();

} // namespace pictura
