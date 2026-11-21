// Select > Save/Load Selection and Channels > Duplicate Channel dialogs.
// These frame methods own the dialog and cross-document transfer; the menu and
// panel translation units stay free of the frame seam.

#include "frame.h"

#include "duplicate_channel_dialog.h"
#include "panel_host.h"
#include "panels/channels_panel.h"
#include "save_load_selection_dialog.h"

#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QRadioButton>

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

// A Load source that is a Channels-panel colour row (`color:<row>`), derived
// from the document's picture rather than a stored channel.
bool isColourTarget(const QString& target)
{
    return target.startsWith(QLatin1String("color:"));
}

int colourRowOf(const QString& target)
{
    return target.mid(QLatin1String("color:").size()).toInt();
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
            invalidateMaskOverlayFor(active);
            refresh();
        } else {
            refuse();
        }
        return;
    }
    if (save_selection_coverage(*target, bytes, width, height, request.target, request.name,
                                        request.mode)) {
        invalidateMaskOverlayFor(target);
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
        if (isColourTarget(request.target)) {
            const auto coverage = color_row_coverage(*active, colourRowOf(request.target), 0x7);
            if (!load_selection_coverage(*active,
                    ::rust::Slice<const std::uint8_t>(coverage.data(), coverage.size()), width,
                    height, request.invert, request.mode)) {
                refuse();
            }
            return;
        }
        if (!load_selection_target(*active, request.target, request.invert, request.mode)) {
            refuse();
        }
        return;
    }
    // ponytail: same full-plane FFI copy as Save Selection.
    const auto coverage = isColourTarget(request.target)
        ? color_row_coverage(*request.document, colourRowOf(request.target), 0x7)
        : channel_coverage(*request.document, request.target);
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

void PicturaMainWindow::openChannelOptionsDialog()
{
    PictureView* view = activeView();
    ChannelsPanel* panel = panels_ ? panels_->channelsPanel() : nullptr;
    if (!view || !view->has_document() || !panel) {
        return;
    }
    const int alpha = panel->alphaIndexAt(panel->currentRow());
    if (alpha < 0 || channels_alpha_is_spot(*view, alpha)) {
        return;
    }

    QDialog dialog(this);
    dialog.setObjectName(QStringLiteral("channelOptionsDialog"));
    dialog.setWindowTitle(tr("Channel Options"));
    auto* layout = new QFormLayout(&dialog);
    auto* masked = new QRadioButton(tr("Masked Areas"), &dialog);
    auto* selected = new QRadioButton(tr("Selected Areas"), &dialog);
    masked->setObjectName(QStringLiteral("channelOptionsMaskedAreas"));
    selected->setObjectName(QStringLiteral("channelOptionsSelectedAreas"));
    const bool selectedAreas = channels_selected_areas(*view, alpha);
    selected->setChecked(selectedAreas);
    masked->setChecked(!selectedAreas);
    layout->addRow(tr("Color Indicates:"), masked);
    layout->addRow(QString(), selected);
    auto* buttons = new QDialogButtonBox(
        QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    buttons->setObjectName(QStringLiteral("channelOptionsButtons"));
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addRow(buttons);

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    if (!channels_set_sense(*view, alpha, selected->isChecked())) {
        return;
    }
    panel->refresh();
    invalidateMaskOverlay();
    refresh();
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
