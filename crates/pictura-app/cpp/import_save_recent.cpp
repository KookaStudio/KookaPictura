// File > Open Recent: the persisted recent-files list and its open path. The
// submenu itself stays on the frame until the menu registrar owns it.

#include "import_save_coordinator.h"

#include "dialogs.h"
#include "session.h"

namespace pictura {

namespace {

// docs/02-ui-ux/preferences.md: "Recent File List Contains", default 20.
constexpr int kRecentLimit = 20;

} // namespace

void ImportSaveCoordinator::rememberRecent(const QString& path)
{
    if (path.isEmpty()) {
        return;
    }
    QStringList files = recent_;
    files.removeAll(path);
    files.prepend(path);
    setRecentFiles(files);
}

void ImportSaveCoordinator::setRecentFiles(const QStringList& files)
{
    recent_ = files.mid(0, kRecentLimit);
    // Keep the file dialogs' Places sidebar in step with the recent list.
    setFileDialogRecentPaths(recent_);
    if (hooks_.saveSession) {
        hooks_.saveSession();
        return;
    }
    SessionState state = loadSession();
    state.recent = recent_;
    pictura::saveSession(state);
}

void ImportSaveCoordinator::openRecent(const QString& path)
{
    // The same routing as File > Open; the completion shows the failure message
    // for a file that could not be read.
    startUserOpen(path, true);
}

} // namespace pictura
