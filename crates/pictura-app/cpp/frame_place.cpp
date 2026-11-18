// File > Place for a multi-file selection: each placed file gets its own
// interactive Free Transform, the next file waits for that session to settle,
// Escape or the transform's Cancel control rolls the placement back, and one
// summary names the files that could not be placed.

#include "frame_place.h"

#include "frame_includes.h"
#include "memory_refusal.h"

#include <QtCore/QEventLoop>
#include <QtCore/QFile>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtCore/QThread>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>

#include <cstddef>
#include <cstdint>
#include <memory>

namespace pictura {

namespace {

enum class TransformWait { Settled, Aborted };

::rust::Slice<const std::uint8_t> pdfBytes(const QByteArray& bytes)
{
    return {reinterpret_cast<const std::uint8_t*>(bytes.constData()),
            static_cast<std::size_t>(bytes.size())};
}

template <typename Work>
auto prepareOnWorker(Work work)
{
    decltype(work()) result{};
    QEventLoop loop;
    std::unique_ptr<QThread> worker(QThread::create([&]() { result = work(); }));
    QObject::connect(worker.get(), &QThread::finished, &loop, &QEventLoop::quit);
    worker->start();
    loop.exec(QEventLoop::ExcludeUserInputEvents);
    worker->wait();
    return result;
}

bool chooseVectorPage(PicturaMainWindow& frame, int pageCount, int& page, int& crop)
{
    QDialog dialog(&frame);
    dialog.setObjectName(QStringLiteral("vectorPlaceOptions"));
    dialog.setWindowTitle(PicturaMainWindow::tr("Place Vector Artwork"));
    auto* layout = new QFormLayout(&dialog);
    auto* pages = new QComboBox(&dialog);
    pages->setObjectName(QStringLiteral("vectorPlacePage"));
    for (int index = 1; index <= pageCount; ++index) {
        pages->addItem(QString::number(index));
    }
    layout->addRow(PicturaMainWindow::tr("Page"), pages);
    auto* boxes = new QComboBox(&dialog);
    boxes->setObjectName(QStringLiteral("vectorPlaceCrop"));
    boxes->addItems({PicturaMainWindow::tr("Bounding Box"), PicturaMainWindow::tr("Media Box"),
                     PicturaMainWindow::tr("Crop Box"), PicturaMainWindow::tr("Bleed Box"),
                     PicturaMainWindow::tr("Trim Box"), PicturaMainWindow::tr("Art Box")});
    boxes->setCurrentIndex(2);
    layout->addRow(PicturaMainWindow::tr("Crop To"), boxes);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addRow(buttons);
    if (runDialog(dialog, &frame) != QDialog::Accepted) {
        return false;
    }
    page = pages->currentIndex() + 1;
    crop = boxes->currentIndex();
    return true;
}

TransformWait waitForTransform(PicturaMainWindow& frame, const QPointer<PictureView>& view,
                               bool* cancelled)
{
    if (!view || !view->transform_session_active()) {
        return TransformWait::Settled;
    }

    // Latch only genuine cancel intents: the ImageView emits
    // transformCancelRequested only while its transform is active, and the
    // options bar's clicked fires after its slot ends the session, so a press
    // dragged off the control (released) is not a cancel and cannot latch.
    QObject latch;
    bool cancelledNow = false;
    if (cancelled) {
        if (ImageView* canvas = frame.imageView()) {
            QObject::connect(canvas, &ImageView::transformCancelRequested, &latch,
                             [&cancelledNow]() { cancelledNow = true; });
        }
        if (QToolButton* button =
                frame.findChild<QToolButton*>(QStringLiteral("optionsTransformCancel"))) {
            QObject::connect(button, &QToolButton::clicked, &latch,
                             [&cancelledNow]() { cancelledNow = true; });
        }
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

    if (cancelled) {
        *cancelled = cancelledNow;
    }
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

QString importPdfArtwork(PictureView& view, const QByteArray& bytes,
                         const QString& filename, bool smart, unsigned int page,
                         unsigned char crop, const QString& label, QString& error)
{
    const QPointer<PictureView> target(&view);
    const auto generation = view.document_generation();
    const auto width = static_cast<unsigned int>(view.document_width());
    const auto height = static_cast<unsigned int>(view.document_height());
    const PreparedVectorPdf prepared = prepareOnWorker([&]() {
        return prepare_vector_pdf(pdfBytes(bytes), page, crop, width, height);
    });
    if (!target || target->document_generation() != generation
        || target->transform_session_active()) {
        return {};
    }
    if (!prepared.error.isEmpty()) {
        error = prepared.error;
        return {};
    }
    return place_prepared_pdf(*target, prepared, pdfBytes(bytes), filename, smart, label);
}

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
        const QFileInfo info(path);
        const QString suffix = info.suffix().toLower();
        const bool vector = !psd && (suffix == QStringLiteral("pdf") || suffix == QStringLiteral("ai")
                                    || vector_sniff(path) == QStringLiteral("pdf"));
        QString created;
        if (vector) {
            QFile file(path);
            const auto limit = vector_pdf_import_limit();
            if (!file.open(QIODevice::ReadOnly) || file.size() < 0
                || quint64(file.size()) > limit) {
                failures.append(path);
                continue;
            }
            const QByteArray bytes = file.read(qint64(limit) + 1);
            const int pageCount =
                prepareOnWorker([&]() { return vector_pdf_page_count(pdfBytes(bytes)); });
            if (pageCount == 0) {
                failures.append(path);
                continue;
            }
            if (!view || frame.activeView() != view.data()) {
                aborted = true;
                break;
            }
            const auto generation = view->document_generation();
            int page = 1;
            int crop = 2;
            if (!chooseVectorPage(frame, pageCount, page, crop)) {
                continue;
            }
            if (!view || frame.activeView() != view.data()
                || view->document_generation() != generation) {
                aborted = true;
                break;
            }
            QString error;
            created = importPdfArtwork(*view, bytes, info.fileName(), true,
                                       static_cast<unsigned int>(page),
                                       static_cast<unsigned char>(crop),
                                       QStringLiteral("Place"), error);
        } else {
            if (!psd && !frame.confirmProfileForOpen(view.data(), path)) {
                continue;
            }
            created = psd ? view->place_smart_object(path) : view->place_image(path);
        }
        if (created.isEmpty()) {
            failures.append(path);
            continue;
        }
        const int placeState = view->history_index();
        const bool transforming = frame.beginFreeTransform(created);
        frame.selectLayerPath(created);
        if (!transforming) {
            if (vector) {
                view->cancel_transform();
                failures.append(path);
            }
            continue;
        }

        bool cancelled = false;
        if (waitForTransform(frame, view, vector ? nullptr : &cancelled) == TransformWait::Aborted) {
            if (vector && view) {
                view->cancel_transform();
            }
            aborted = true;
            break;
        }
        if (vector) {
            const auto result = vector_place_result(*view);
            if (result == VectorPlaceResult::Pending) {
                view->cancel_transform();
            }
            if (result == VectorPlaceResult::Refused) {
                failures.append(path);
                continue;
            }
            cancelled = result != VectorPlaceResult::Committed;
        }
        if (!cancelled) {
            continue;
        }
        // Escape/Cancel removes the placement, but only when the history still
        // points at this item's "Place" state; an undo already removed it.
        if (!vector && view->history_index() == placeState
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
