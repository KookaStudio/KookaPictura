#include "selftest_recent_files.h"
#include "selftest_report.h"

#include "frame.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QMenu>

int pictura::runRecentFilesChecks(pictura::PicturaMainWindow& frame)
{
    ST_BEGIN("recent_files");
    const QStringList saved = frame.recentFiles();
    QMenu* menu = frame.recentMenu();
    const QString imagePath = QDir::temp().filePath(QStringLiteral("pictura_recent_seed.png"));
    const QString missingPath = QDir::temp().filePath(QStringLiteral("pictura_recent_missing.png"));
    QImage seed(8, 8, QImage::Format_RGB32);
    seed.fill(Qt::red);
    if (!menu || !seed.save(imagePath, "PNG")) {
        return pictura::selfTest().fail(pictura::selftest_codes::kCode533, "recent files fixture");
    }
    QFile::remove(missingPath);
    // The visible file rows (everything but the placeholder, separator, Clear).
    const auto rows = [menu]() {
        emit menu->aboutToShow();
        QList<QAction*> files;
        for (QAction* action : menu->actions()) {
            if (action->isVisible() && !action->isSeparator() && action->isEnabled()
                && action->text() != QStringLiteral("Clear Recent File List")) {
                files.append(action);
            }
        }
        return files;
    };
    const auto hasClear = [menu]() {
        for (QAction* action : menu->actions()) {
            if (action->text() == QStringLiteral("Clear Recent File List")) {
                return true;
            }
        }
        return false;
    };

    frame.setRecentFiles({});
    const QList<QAction*> emptyRows = rows();
    bool placeholder = false;
    for (QAction* action : menu->actions()) {
        placeholder |= action->isVisible() && !action->isEnabled()
            && action->text() == QStringLiteral("No Recent Files");
    }
    const bool empty = emptyRows.isEmpty() && placeholder && !hasClear();

    const int docsBefore = frame.documentCount();
    const bool opened = frame.openImagePath(imagePath);
    const bool front = !frame.recentFiles().isEmpty() && frame.recentFiles().first() == imagePath;
    frame.setRecentFiles({missingPath, imagePath});
    const QList<QAction*> listed = rows();
    const bool offered = listed.size() == 1
        && listed.first()->text() == QFileInfo(imagePath).fileName()
        && listed.first()->toolTip() == imagePath && hasClear()
        && frame.recentFiles() == QStringList{imagePath};

    const int docsMid = frame.documentCount();
    if (!listed.isEmpty()) {
        listed.first()->trigger();
    }
    const bool reopened = frame.documentCount() == docsMid + 1;

    for (QAction* action : menu->actions()) {
        if (action->text() == QStringLiteral("Clear Recent File List")) {
            action->trigger();
        }
    }
    const bool cleared = frame.recentFiles().isEmpty() && rows().isEmpty();

    while (frame.documentCount() > docsBefore) {
        frame.closeDocument(frame.documentCount() - 1, false);
    }
    frame.setRecentFiles(saved);
    QFile::remove(imagePath);

    ST_PASS("recent_files empty=%d opened=%d front=%d offered=%d reopened=%d cleared=%d",
            empty ? 1 : 0, opened ? 1 : 0, front ? 1 : 0, offered ? 1 : 0, reopened ? 1 : 0,
            cleared ? 1 : 0);
    if (!empty || !opened || !front || !offered || !reopened || !cleared) {
        return pictura::selfTest().fail(pictura::selftest_codes::kCode533, "recent files");
    }
    return 0;
}
