// Select > Save/Load Selection and Channels > Duplicate Channel dialogs.
// These frame methods own the dialog and cross-document transfer; the menu and
// panel translation units stay free of the frame seam.

#include "frame.h"

#include "duplicate_channel_dialog.h"
#include "panel_host.h"
#include "panels/channels_panel.h"
#include "save_load_selection_dialog.h"

#include "api/selection_bridge.h"

#include <QtWidgets/QMessageBox>

#include <cstddef>

namespace pictura {

namespace {

QList<int> sameSizeDocumentIndexes(const PicturaMainWindow& frame)
{
    QList<int> indexes;
    PictureView* active = frame.activeView();
    if (!active || !active->has_document()) {
        return indexes;
    }
    const int width = active->document_width();
    const int height = active->document_height();
    for (int i = 0; i < frame.documentCount(); ++i) {
        PictureView* view = frame.viewAt(i);
        if (view && view->has_document() && view->document_width() == width
            && view->document_height() == height) {
            indexes.push_back(i);
        }
    }
    return indexes;
}

} // namespace

QStringList PicturaMainWindow::selectionDocumentLabelsForTest() const
{
    QStringList labels;
    for (int i : sameSizeDocumentIndexes(*this)) {
        labels.push_back(documentName(i));
    }
    return labels;
}

void PicturaMainWindow::openSaveSelectionDialog()
{
    PictureView* active = activeView();
    if (!active || !active->has_document() || !active->has_selection()) {
        return;
    }
    const int width = active->document_width();
    const int height = active->document_height();
    const QList<int> indexes = sameSizeDocumentIndexes(*this);
    if (indexes.isEmpty()) {
        return;
    }
    QList<SelectionDocument> documents;
    for (int i : indexes) {
        PictureView* view = viewAt(i);
        documents.push_back({view, documentName(i), view == active});
    }
    const auto refuse = [this]() {
        QMessageBox::warning(
            this, tr("Save Selection"),
            tr("Could not save the selection. A new channel may exceed the document's "
               "56-channel limit, or the target is no longer available."));
    };
    SaveLoadSelectionDialog dialog(SaveLoadSelectionDialog::Mode::Save, documents, this);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    const SelectionSaveRequest request = dialog.saveRequest();
    // ponytail: the whole width x height coverage plane crosses FFI here and is
    // cloned again by the Rust receiver. Same-size, user-initiated only; tile the
    // transfer if large-document latency matters.
    const auto coverage = selection_coverage_bytes(*active);
    const ::rust::Slice<const std::uint8_t> bytes(coverage.data(), coverage.size());
    if (request.newDocument) {
        auto* view = new PictureView(this);
        if (!new_selection_document(*view, bytes, width, height, request.name, false)) {
            delete view;
            refuse();
            return;
        }
        addDocument(view, QString());
        refresh();
        return;
    }
    PictureView* target = request.document ? request.document : active;
    if (target == active) {
        if (save_selection_to(*active, request.target, request.name, request.mode)) {
            refresh();
        } else {
            refuse();
        }
        return;
    }
    if (save_selection_coverage(*target, bytes, width, height, request.target, request.name,
                                        request.mode)) {
        refresh();
    } else {
        refuse();
    }
}

void PicturaMainWindow::openLoadSelectionDialog()
{
    PictureView* active = activeView();
    if (!active || !active->has_document()) {
        return;
    }
    const int width = active->document_width();
    const int height = active->document_height();
    const QList<int> indexes = sameSizeDocumentIndexes(*this);
    if (indexes.isEmpty()) {
        return;
    }
    QList<SelectionDocument> documents;
    for (int i : indexes) {
        PictureView* view = viewAt(i);
        documents.push_back({view, documentName(i), view == active});
    }
    const auto refuse = [this]() {
        QMessageBox::warning(this, tr("Load Selection"),
                             tr("Could not load the selection from the chosen source."));
    };
    SaveLoadSelectionDialog dialog(SaveLoadSelectionDialog::Mode::Load, documents, this);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    const SelectionLoadRequest request = dialog.loadRequest();
    if (!request.document) {
        return;
    }
    if (request.document == active) {
        if (!load_selection_target(*active, request.target, request.invert, request.mode)) {
            refuse();
        }
        return;
    }
    // ponytail: same full-plane FFI copy as Save Selection.
    const auto coverage = channel_coverage(*request.document, request.target);
    if (coverage.size() == 0) {
        refuse();
        return;
    }
    if (load_selection_coverage(*active,
            ::rust::Slice<const std::uint8_t>(coverage.data(), coverage.size()), width, height,
            request.invert, request.mode)) {
        refresh();
    } else {
        refuse();
    }
}

void PicturaMainWindow::openDuplicateChannelDialog()
{
    PictureView* source = activeView();
    ChannelsPanel* panel = panels_ ? panels_->channelsPanel() : nullptr;
    if (!source || !source->has_document() || !panel) {
        return;
    }
    const int row = panel->currentRow();
    if (!panel->canDuplicateRow(row)) {
        return;
    }
    const int width = source->document_width();
    const int height = source->document_height();
    QList<SelectionDocument> documents;
    for (int i = 0; i < documentCount(); ++i) {
        PictureView* view = viewAt(i);
        if (!view || !view->has_document() || view->document_width() != width
            || view->document_height() != height) {
            continue;
        }
        documents.push_back({view, documentName(i), view == source});
    }
    DuplicateChannelDialog dialog(panel->nameAt(row), documents, this);
    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    const DuplicateChannelRequest request = dialog.request();
    const int alpha = panel->alphaIndexAt(row);
    const auto coverage =
        alpha >= 0
            ? channel_coverage(*source, QStringLiteral("alpha:%1").arg(alpha))
            : color_row_coverage(*source, row, panel->channelMask());
    if (coverage.size() != std::size_t(width) * std::size_t(height)) {
        QMessageBox::warning(
            this, tr("Duplicate Channel"),
            tr("Could not duplicate the channel. The source channel is no longer available."));
        return;
    }
    const ::rust::Slice<const std::uint8_t> bytes(coverage.data(), coverage.size());
    if (request.newDocument) {
        auto* view = new PictureView(this);
        if (!new_selection_document(*view, bytes, width, height, request.name, request.invert)) {
            delete view;
            QMessageBox::warning(
                this, tr("Duplicate Channel"),
                tr("Could not create a document for the duplicated channel."));
            return;
        }
        addDocument(view, QString());
        refresh();
        return;
    }
    PictureView* target = request.document ? request.document : source;
    if (!duplicate_channel_coverage(*target, bytes, width, height, request.name,
                                     request.invert)) {
        QMessageBox::warning(
            this, tr("Duplicate Channel"),
            tr("Could not duplicate the channel. The destination may have reached its "
               "56-channel limit."));
        return;
    }
    const int index = indexOfView(target);
    if (index >= 0) {
        setActiveDocumentIndex(index);
    }
    refresh();
}

} // namespace pictura
