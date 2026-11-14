// File > Place for a multi-file selection: each placed file gets its own
// interactive Free Transform, the next file waits for that session to settle,
// Escape or the transform's Cancel control rolls the placement back, and one
// summary names the files that could not be placed.

#include "frame_place.h"

#include "frame_includes.h"
#include "memory_refusal.h"

#include <QtCore/QEventLoop>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolButton>

namespace pictura {

namespace {

enum class TransformWait { Settled, Aborted };

TransformWait waitForTransform(PicturaMainWindow& frame, const QPointer<PictureView>& view,
                               bool* cancelled)
{
    *cancelled = false;
    if (!view || !view->transform_session_active()) {
        return TransformWait::Settled;
    }

    // Latch only genuine cancel intents: the ImageView emits
    // transformCancelRequested only while its transform is active, and the
    // options bar's clicked fires after its slot ends the session, so a press
    // dragged off the control (released) is not a cancel and cannot latch.
    QObject latch;
    bool cancelledNow = false;
    if (ImageView* canvas = frame.imageView()) {
        QObject::connect(canvas, &ImageView::transformCancelRequested, &latch,
                         [&cancelledNow]() { cancelledNow = true; });
    }
    if (QToolButton* button =
            frame.findChild<QToolButton*>(QStringLiteral("optionsTransformCancel"))) {
        QObject::connect(button, &QToolButton::clicked, &latch,
                         [&cancelledNow]() { cancelledNow = true; });
    }

    QEventLoop loop;
    QTimer poll;
    poll.setInterval(10);
    QObject::connect(&poll, &QTimer::timeout, &loop, [&]() {
        if (!view || !view->transform_session_active() || frame.activeView() != view.data()
            || QApplication::closingDown()) {
            loop.quit();
        }
    });
    poll.start();
    loop.exec();
    poll.stop();

    *cancelled = cancelledNow;
    if (!view || frame.activeView() != view.data() || QApplication::closingDown()) {
        return TransformWait::Aborted;
    }
    return TransformWait::Settled;
}

bool askContinueAfterCancel(PicturaMainWindow& frame)
{
    QMessageBox box(QMessageBox::Question, PicturaMainWindow::tr("Place"),
                    PicturaMainWindow::tr(
                        "Cancel the whole Place or continue with the next item?"),
                    QMessageBox::NoButton, &frame);
    box.setObjectName(QStringLiteral("placeCancelDialog"));
    auto* whole = box.addButton(PicturaMainWindow::tr("Cancel the whole Place"),
                                QMessageBox::RejectRole);
    whole->setObjectName(QStringLiteral("placeCancelWhole"));
    auto* next = box.addButton(PicturaMainWindow::tr("Continue with the next item"),
                               QMessageBox::AcceptRole);
    next->setObjectName(QStringLiteral("placeCancelContinue"));
    box.setDefaultButton(next);
    box.setEscapeButton(whole);
    box.exec();
    return box.clickedButton() == next;
}

void showFailureSummary(PicturaMainWindow& frame, const QStringList& failures)
{
    QStringList names;
    names.reserve(failures.size());
    for (const QString& path : failures) {
        names.append(QFileInfo(path).fileName());
    }
    QMessageBox box(QMessageBox::Warning, PicturaMainWindow::tr("Place"),
                    PicturaMainWindow::tr("Could not place the following files:\n%1")
                        .arg(names.join(QLatin1Char('\n'))),
                    QMessageBox::Ok, &frame);
    box.setObjectName(QStringLiteral("placeSummaryDialog"));
    box.exec();
}

} // namespace

void placeDocuments(PicturaMainWindow& frame, const QStringList& paths)
{
    PictureView* active = frame.activeView();
    if (!active || !active->has_document() || active->transform_session_active()
        || paths.isEmpty()) {
        return;
    }
    // dispatch ignores an action's enabled state, so the re-entry set is the
    // real guard for a second batch started from a nested wait.
    static QSet<const PicturaMainWindow*> running;
    if (running.contains(&frame)) {
        return;
    }
    running.insert(&frame);
    struct RunningGuard {
        QSet<const PicturaMainWindow*>* set;
        const PicturaMainWindow* frame;
        ~RunningGuard() { set->remove(frame); }
    } guard{&running, &frame};

    QPointer<PictureView> view(active);
    frame.applyColorSettings(view.data());
    QStringList failures;
    bool aborted = false;
    bool cancelledBatch = false;

    for (int index = 0; index < paths.size(); ++index) {
        if (!view || frame.activeView() != view.data() || !view->has_document()
            || view->transform_session_active()) {
            aborted = true;
            break;
        }
        const QString& path = paths.at(index);
        const bool psd = PicturaMainWindow::isNativeDocumentPath(path);
        if (!psd && !frame.confirmProfileForOpen(view.data(), path)) {
            continue;
        }
        const QString created = psd ? view->place_smart_object(path) : view->place_image(path);
        if (created.isEmpty()) {
            failures.append(path);
            continue;
        }
        const int placeState = view->history_index();
        const bool transforming = frame.beginFreeTransform(created);
        frame.selectLayerPath(created);
        if (!transforming) {
            continue;
        }

        bool cancelled = false;
        if (waitForTransform(frame, view, &cancelled) == TransformWait::Aborted) {
            aborted = true;
            break;
        }
        if (!cancelled) {
            continue;
        }
        // Escape/Cancel removes the placement, but only when the history still
        // points at this item's "Place" state; an undo already removed it.
        if (view->history_index() == placeState
            && view->history_label(placeState) == QStringLiteral("Place")) {
            if (!view->undo()) {
                reportMemoryRefusal(&frame, PicturaMainWindow::tr("cancel the placement"));
                aborted = true;
                break;
            }
        }
        if (index + 1 < paths.size() && !askContinueAfterCancel(frame)) {
            cancelledBatch = true;
            break;
        }
    }

    if (aborted || cancelledBatch) {
        return;
    }
    if (!failures.isEmpty()) {
        showFailureSummary(frame, failures);
    }
}

} // namespace pictura
