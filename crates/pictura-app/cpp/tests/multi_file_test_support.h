#pragma once

// File-dialog driving shared by the multi-file Open and Place suites.

#include <QtTest/QtTest>

#include <memory>

#include <QtCore/QFileInfo>
#include <QtCore/QItemSelectionModel>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QFileSystemModel>
#include <QtGui/QImage>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QApplication>
#include <QtWidgets/QFileDialog>

#include "frame.h"
#include "qt_test_support.h"

namespace {

struct OpenDialogProbe {
    bool seen = false;
    QFileDialog::FileMode mode = QFileDialog::AnyFile;
    QStringList filters;
    QStringList selected;
    QString error;
};

QFileDialog* visibleFileDialog()
{
    // The dialog is parented to the frame, so it is not in topLevelWidgets().
    for (QWidget* widget : QApplication::allWidgets()) {
        if (auto* dialog = qobject_cast<QFileDialog*>(widget); dialog && dialog->isVisible()) {
            return dialog;
        }
    }
    return nullptr;
}

// QFileSystemModel populates a folder asynchronously, so wait until the view
// shows the chosen folder before selecting in it.
bool waitForFiles(QFileSystemModel* model, QAbstractItemView* list, const QStringList& paths)
{
    for (int waited = 0; waited < 5000; waited += 20) {
        bool ready = list->rootIndex().isValid() && model->rowCount(list->rootIndex()) > 0;
        for (int i = 0; ready && i < paths.size(); ++i) {
            ready = model->index(paths.at(i)).isValid();
        }
        if (ready) {
            return true;
        }
        QTest::qWait(20); // QFileSystemModel populates the folder asynchronously.
    }
    return false;
}

// Select `paths` in the open dialog's file list and accept it.
void chooseFiles(QFileDialog* dialog, const QStringList& paths, OpenDialogProbe* probe)
{
    dialog->setDirectory(QFileInfo(paths.first()).absolutePath());
    auto* list = dialog->findChild<QAbstractItemView*>(QStringLiteral("listView"));
    auto* model = list ? qobject_cast<QFileSystemModel*>(list->model()) : nullptr;
    if (!list || !model || !list->selectionModel()) {
        probe->error = QStringLiteral("the dialog has no file list");
        dialog->reject();
        return;
    }
    if (!waitForFiles(model, list, paths)) {
        probe->error = QStringLiteral("the dialog did not show the folder");
        dialog->reject();
        return;
    }
    QItemSelection selection;
    for (const QString& path : paths) {
        const QModelIndex index = model->index(path);
        if (!index.isValid()) {
            probe->error = QStringLiteral("no index for %1").arg(path);
            dialog->reject();
            return;
        }
        selection.select(index, index);
    }
    list->selectionModel()->select(selection,
                                   QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows);
    probe->selected = dialog->selectedFiles();
    // QFileDialog overrides accept() as protected; access is checked on the
    // static type while virtual dispatch still runs the dialog's override.
    static_cast<QDialog*>(dialog)->accept();
}

// Schedule the interaction with the dialog that File > Open is about to show.
// The watchdog rejects a dialog that never gets chosen, so a failed probe fails
// the test instead of hanging it; `context` cancels both timers with the window.
void queueOpenDialogInteraction(QWidget* context, const QStringList& paths,
                                OpenDialogProbe* probe)
{
    QTimer::singleShot(0, context, [paths, probe] {
        // showOpenDialog() shows the dialog after this timer is queued; poll so
        // a slow platform cannot lose the interaction before it is visible.
        for (int waited = 0; waited < 5000; waited += 10) {
            QFileDialog* dialog = visibleFileDialog();
            if (dialog) {
                probe->seen = true;
                probe->mode = dialog->fileMode();
                chooseFiles(dialog, paths, probe);
                return;
            }
            QTest::qWait(10); // Let the dialog finish being shown.
        }
        probe->error = QStringLiteral("no open dialog appeared");
    });
    QTimer::singleShot(10000, context, [probe] {
        if (QFileDialog* dialog = visibleFileDialog()) {
            probe->error = QStringLiteral("the open dialog did not accept");
            dialog->reject();
        }
    });
}

bool writePng(const QString& path, const QColor& color)
{
    QImage image(8, 8, QImage::Format_ARGB32);
    image.fill(color);
    return image.save(path, "PNG");
}

std::unique_ptr<pictura::PicturaMainWindow> makeShownWindow()
{
    auto window = pictura::test::makeMainWindow();
    if (window) {
        window->show();
        QTest::qWait(0);
    }
    return window;
}

} // namespace
