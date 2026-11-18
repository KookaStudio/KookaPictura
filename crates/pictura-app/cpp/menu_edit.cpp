#include "menu_registrar.h"

#include "commands.h"
#include <QtGui/QKeyEvent>

#include "menu_registrar_internal.h"

#include "document_registry.h"
#include "panel_host.h"
#include "panels/layers_panel.h"
#include "tools.h"

#include "api/filters_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/engine_bridge.h"

#include "fill_dialog.h"
#include "frame_place.h"
#include "stroke_dialog.h"
#include "fade_dialog.h"

#include "api/history_api.h"
#include "document_command.h"
#include "dialogs.h"
#include "memory_refusal.h"
#include "text_input.h"


#include <QtCore/QMimeData>
#include <QtCore/QPointer>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

#include <cstddef>
#include <cstdint>

namespace pictura {

namespace {

bool systemClipboardHasImage()
{
    const QMimeData* data = QGuiApplication::clipboard()->mimeData();
    return data && data->hasImage();
}

const QString kPdfClipboard = QStringLiteral("application/pdf");

bool systemClipboardHasPdf()
{
    const QMimeData* mime = QGuiApplication::clipboard()->mimeData();
    return mime && mime->hasFormat(kPdfClipboard);
}

bool choosePdfPaste(QWidget* parent, bool& smart)
{
    QDialog dialog(parent);
    dialog.setObjectName(QStringLiteral("vectorPasteAs"));
    dialog.setWindowTitle(QObject::tr("Paste As"));
    auto* layout = new QVBoxLayout(&dialog);
    const auto addChoice = [&](const QString& label, const QString& name, bool available) {
        auto* choice = new QRadioButton(label, &dialog);
        choice->setObjectName(name);
        choice->setEnabled(available);
        layout->addWidget(choice);
        return choice;
    };
    auto* smartObject = addChoice(QObject::tr("Smart Object"), QStringLiteral("vectorPasteSmart"),
                                  true);
    addChoice(QObject::tr("Pixels"), QStringLiteral("vectorPastePixels"), true);
    addChoice(QObject::tr("Paths"), QStringLiteral("vectorPastePaths"), false);
    addChoice(QObject::tr("Shape Layer"), QStringLiteral("vectorPasteShape"), false);
    smartObject->setChecked(true);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    smart = smartObject->isChecked();
    return true;
}

} // namespace

// Put the bridge clipboard on the system clipboard so other applications can
// paste it. The bridge copy keeps its document position and selection coverage,
// which the system image cannot carry.
void MenuRegistrar::exportClipboard()
{
    int width = 0;
    int height = 0;
    const ::rust::Vec<std::uint8_t> rgba = clipboard_export(width, height);
    if (width <= 0 || height <= 0) {
        return;
    }
    const QImage image =
        QImage(rgba.data(), width, height, width * 4, QImage::Format_RGBA8888).copy();
    clipboardSetting_ = true;
    QGuiApplication::clipboard()->setImage(image);
    clipboardSetting_ = false;
    clipboardMirrored_ = true;
    clipboardExported_ = true;
}

// Before a paste: when another application has taken the system clipboard since
// our last copy, its image replaces the bridge copy (placed at the canvas
// origin). Without an image there the bridge copy stays.
void MenuRegistrar::importSystemClipboard()
{
    if (clipboardMirrored_) {
        return;
    }
    const QImage image = QGuiApplication::clipboard()->image();
    if (image.isNull()) {
        return;
    }
    const QImage rgba = image.convertToFormat(QImage::Format_RGBA8888);
    const auto* bits = rgba.constBits();
    const std::size_t size = std::size_t(rgba.width()) * rgba.height() * 4;
    if (rgba.bytesPerLine() == rgba.width() * 4
        && clipboard_import(rgba.width(), rgba.height(),
                            ::rust::Slice<const std::uint8_t>(bits, size))) {
        clipboardMirrored_ = true;
    }
}

// Edit clipboard. Copy/Cut/Clear act on the Layers panel's current layer; a
// paste lands above it and becomes the current layer. The clipboard is shared by
// every open document and mirrored to the system clipboard; each mutating
// command records one state in the bridge, and a refusal records nothing.
void MenuRegistrar::registerEditHandlers()
{
    // The canvas zoom keys keep working behind a running dialog.
    setDialogNavigationKeys([window = QPointer<MenuRegistrar>(this)](QKeyEvent* event) {
        const Qt::KeyboardModifiers mods = event->modifiers() & ~Qt::KeypadModifier;
        if (!window || !(mods & Qt::ControlModifier) || (mods & Qt::AltModifier)) {
            return false;
        }
        const char* id = nullptr;
        switch (event->key()) {
        case Qt::Key_Plus:
        case Qt::Key_Equal:
            id = command_ids::ViewZoomIn;
            break;
        case Qt::Key_Minus:
        case Qt::Key_Underscore:
            id = command_ids::ViewZoomOut;
            break;
        case Qt::Key_0:
            id = command_ids::ViewFitOnScreen;
            break;
        case Qt::Key_1:
            id = command_ids::ViewActualPixels;
            break;
        default:
            return false;
        }
        window->commands()->dispatch(QLatin1String(id));
        return true;
    });
    setDialogCanvasSampling([window = QPointer<MenuRegistrar>(this)]() {
        return window && window->tools_ && window->tools_->canvasSamplerActive();
    });
    // ponytail: a platform that reports our own write asynchronously drops the
    // mirror, so the next paste re-imports our export and Paste in Place loses
    // the source position (it lands at the origin).
    connect(QGuiApplication::clipboard(), &QClipboard::dataChanged, this, [this]() {
        if (!clipboardSetting_) {
            clipboardMirrored_ = false;
            clipboardExported_ = false;
        }
    });

    const auto currentPath = [this]() {
        return layerAnchor();
    };
    const auto layerCommand = [this, currentPath](const auto& run) {
        PictureView* view = activeView();
        const QString path = currentPath();
        if (view && !path.isEmpty() && run(*view, path)) {
            refresh();
        }
    };
    commands_->setHandler(command_ids::EditCopy, [this, layerCommand]() {
        layerCommand([this](PictureView& view, const QString& path) {
            const bool copied = clipboard_copy(view, path, false);
            if (copied) {
                exportClipboard();
            }
            return copied;
        });
    });
    commands_->setHandler(command_ids::EditCut, [this, layerCommand]() {
        layerCommand([this](PictureView& view, const QString& path) {
            const bool cut = clipboard_cut(view, path);
            if (cut) {
                exportClipboard();
            }
            return cut;
        });
    });
    commands_->setHandler(command_ids::EditClear, [layerCommand]() {
        layerCommand(
            [](PictureView& view, const QString& path) { return clipboard_clear(view, path); });
    });
    commands_->setHandler(command_ids::EditCopyMerged, [this]() {
        if (PictureView* view = activeView(); view && clipboard_copy(*view, QString(), true)) {
            exportClipboard();
            refresh();
        }
    });

    // A plain paste and Paste in Place land at the clip's own position in the
    // bridge; Paste Into centres on the selection and Paste Outside on what the
    // canvas shows.
    const auto paste = [this, currentPath](PasteKind kind) {
        PictureView* view = activeView();
        if (!view || !view->has_document()) {
            return;
        }
        importSystemClipboard();
        QPointF centre(view->document_width() / 2.0, view->document_height() / 2.0);
        if (ImageView* canvas = imageView()) {
            centre = canvas->widgetToImage(QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
        }
        const QString created =
            clipboard_paste(*view, currentPath(), kind, centre.x(), centre.y());
        if (!created.isEmpty()) {
            refresh();
            selectLayerPath(created);
        }
    };
    commands_->setHandler(command_ids::EditPaste, [this, paste]() {
        const QMimeData* data = QGuiApplication::clipboard()->mimeData();
        if (!data || !data->hasFormat(kPdfClipboard)) {
            paste(PasteKind::Plain);
            return;
        }
        const QPointer<PictureView> view(activeView());
        if (!view || !view->has_document() || view->transform_session_active()) {
            return;
        }
        const auto generation = view->document_generation();
        const QByteArray bytes = data->data(kPdfClipboard);
        bool smart = true;
        if (!choosePdfPaste(dialogParent(), smart) || !view
            || view.data() != activeView() || view->document_generation() != generation) {
            return;
        }
        QString error;
        const QString path = importPdfArtwork(*view, bytes, tr("Pasted Artwork.pdf"), smart, 1, 2,
                                              QStringLiteral("Paste"), error);
        if (!path.isEmpty()) {
            refresh();
            selectLayerPath(path);
        } else if (!error.isEmpty()) {
            QMessageBox::warning(dialogParent(), tr("Paste As"), error);
        }
    });
    commands_->setHandler(command_ids::EditPasteInPlace, [paste]() { paste(PasteKind::InPlace); });
    commands_->setHandler(command_ids::EditPasteInto, [paste]() { paste(PasteKind::Into); });
    commands_->setHandler(command_ids::EditPasteOutside, [paste]() { paste(PasteKind::Outside); });

    // Edit > Fill… fills the active pixel layer (within the selection) through
    // the fill engine. The dialog's contents, mode, opacity, and Preserve
    // Transparency drive one "Fill" state.
    commands_->setHandler(command_ids::EditFill, [this, currentPath]() {
        PictureView* view = activeView();
        if (!view || !view->has_document() || !tools_ || currentPath().isEmpty()
            || tools_->transformPending()) {
            return;
        }
        FillDialog dialog(tools_->foreground(), tools_->background(), dialogParent());
        if (runDialog(dialog, dialogParent()) != QDialog::Accepted) {
            return;
        }
        const bool pattern = dialog.isPatternFill();
        const QColor color = dialog.fillColor();
        const int patternIndex = pattern ? dialog.selectedPatternIndex() : -1;
        const int argb = pattern ? 0 : int(color.rgba());
        // A large document's fill runs as a job behind a progress window (#1300).
        const bool filled = runDocumentCommand(dialogParent(), *view, tr("Fill"), [&] {
            edit_fill(*view, uint32_t(argb), patternIndex, dialog.blendMode(), dialog.opacity(),
                      dialog.preserveTransparency());
        });
        if (filled) {
            refresh();
            CommandParams command;
            command.values.insert(QStringLiteral("contents"),
                                  pattern ? QStringLiteral("pattern") : QStringLiteral("color"));
            command.values.insert(QStringLiteral("color"),
                                  static_cast<double>(uint32_t(argb)));
            command.values.insert(QStringLiteral("pattern"), patternIndex);
            command.values.insert(QStringLiteral("mode"), dialog.blendMode());
            command.values.insert(QStringLiteral("opacity"), dialog.opacity());
            command.values.insert(QStringLiteral("preserveTransparency"),
                                  dialog.preserveTransparency());
            commands_->recordCommand(command_ids::EditFill, command);
        } else if (edit_refused_for_memory()) {
            showStatus(fillMemoryRefusalMessage(), 6000);
        }
    });
    commands_->setExecutor(command_ids::EditFill, [this](const CommandParams& params) {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || !view->has_document() || path.isEmpty()) {
            return false;
        }
        const bool applied = edit_fill(
            *view, uint32_t(params.values.value(QStringLiteral("color")).toDouble()),
            params.values.value(QStringLiteral("pattern")).toInt(-1),
            params.values.value(QStringLiteral("mode")).toString(QStringLiteral("normal")),
            params.values.value(QStringLiteral("opacity")).toInt(100),
            params.values.value(QStringLiteral("preserveTransparency")).toBool());
        if (applied) {
            refresh();
        }
        return applied;
    });

    // Edit > Stroke… outlines the active selection on the active pixel layer
    // through the stroke engine. Width, colour, location, mode, and opacity
    // drive one "Stroke" state.
    commands_->setHandler(command_ids::EditStroke, [this, currentPath]() {
        PictureView* view = activeView();
        if (!view || !view->has_document() || !view->has_selection()
            || currentPath().isEmpty() || (tools_ && tools_->transformPending())) {
            return;
        }
        StrokeDialog dialog(tools_ ? tools_->foreground() : QColor(Qt::black), dialogParent());
        if (runDialog(dialog, dialogParent()) != QDialog::Accepted) {
            return;
        }
        const int argb = int(dialog.strokeColor().rgba());
        const bool stroked = runDocumentCommand(dialogParent(), *view, tr("Stroke"), [&] {
            edit_stroke(*view, uint32_t(argb), dialog.strokeWidth(), dialog.location(),
                        dialog.blendMode(), dialog.opacity());
        });
        if (stroked) {
            refresh();
        } else if (edit_refused_for_memory()) {
            showStatus(fillMemoryRefusalMessage(), 6000);
        }
    });

    // Edit > Fade… changes the opacity and blend mode of the last destructive
    // filter, adjustment, or paint/erase without re-running it. The record is
    // dropped by the next edit, so the entry is enabled only while it exists.
    commands_->setImplemented(command_ids::EditFade, true);
    commands_->setHandler(command_ids::EditFade, [this]() {
        PictureView* view = activeView();
        if (!view || !fade_available(*view)) {
            return;
        }
        FadeDialog dialog(*view, dialogParent());
        if (runDialog(dialog, dialogParent()) != QDialog::Accepted) {
            return;
        }
        if (dialog.apply()) {
            refresh();
        } else {
            showStatus(tr("Could not apply Fade."), 6000);
        }
    });
    commands_->setEnabledProvider(command_ids::EditFade, [this]() {
        PictureView* view = activeView();
        return view && view->has_document() && fade_available(*view);
    });
    commands_->setLabelProvider(command_ids::EditFade, [this]() {
        PictureView* view = activeView();
        if (!view) {
            return QStringLiteral("Fade…");
        }
        const QString name = fade_name(*view);
        return name.isEmpty() ? QStringLiteral("Fade…") : QStringLiteral("Fade %1…").arg(name);
    });

    // Purge drops our copy and, if the system clipboard still holds our export,
    // that too; another application's clipboard is left alone.
    commands_->setHandler(command_ids::EditPurgeClipboard, [this]() {
        clipboard_purge();
        if (clipboardExported_) {
            clipboardSetting_ = true;
            QGuiApplication::clipboard()->clear();
            clipboardSetting_ = false;
        }
        clipboardMirrored_ = false;
        clipboardExported_ = false;
        refresh();
    });

    const auto hasDocument = [this]() { return activeView() && activeView()->has_document(); };
    const auto hasContents = []() { return clipboard_has_contents() || systemClipboardHasImage(); };
    for (const char* id : {command_ids::EditCut, command_ids::EditCopy, command_ids::EditClear}) {
        commands_->setEnabledProvider(
            id, [hasDocument, currentPath]() { return hasDocument() && !currentPath().isEmpty(); });
    }
    commands_->setEnabledProvider(command_ids::EditCopyMerged, hasDocument);
    commands_->setEnabledProvider(command_ids::EditPaste, [hasDocument, hasContents]() {
        return hasDocument() && (hasContents() || systemClipboardHasPdf());
    });
    commands_->setEnabledProvider(command_ids::EditPasteInPlace, [hasDocument, hasContents]() {
        return hasDocument() && hasContents();
    });
    for (const char* id : {command_ids::EditPasteInto, command_ids::EditPasteOutside}) {
        commands_->setEnabledProvider(id, [this, hasDocument, hasContents]() {
            return hasDocument() && hasContents() && activeView()->has_selection();
        });
    }
    commands_->setEnabledProvider(command_ids::EditPurgeClipboard,
                                  []() { return clipboard_has_contents(); });

    // Purge Undo drops the undo stack; Purge Histories drops the named restore
    // points; Purge All does both (and the clipboard, as CS6's All does).
    const auto purgeView = [this](auto run) {
        PictureView* view = activeView();
        if (view && view->has_document() && run(*view)) {
            refresh();
        }
    };
    commands_->setHandler(command_ids::EditPurgeUndo,
                          [purgeView]() { purgeView([](PictureView& v) { return history_api::purgeUndo(v); }); });
    commands_->setHandler(command_ids::EditPurgeHistories, [purgeView]() {
        purgeView([](PictureView& v) { return history_api::purgeHistories(v); });
    });
    commands_->setHandler(command_ids::EditPurgeAll, [this, purgeView]() {
        purgeView([](PictureView& v) { return history_api::purgeAll(v); });
        clipboard_purge();
        if (clipboardExported_) {
            clipboardSetting_ = true;
            QGuiApplication::clipboard()->clear();
            clipboardSetting_ = false;
        }
        clipboardMirrored_ = false;
        clipboardExported_ = false;
        refresh();
    });
    const auto purgeEnabled = [this]() {
        return activeView() && activeView()->has_document();
    };
    for (const char* id :
         {command_ids::EditPurgeUndo, command_ids::EditPurgeHistories, command_ids::EditPurgeAll}) {
        commands_->setEnabledProvider(id, purgeEnabled);
    }
    commands_->setEnabledProvider(command_ids::EditFill, [hasDocument, currentPath]() {
        return hasDocument() && !currentPath().isEmpty();
    });
    commands_->setEnabledProvider(command_ids::EditStroke, [this, hasDocument, currentPath]() {
        return hasDocument() && activeView()->has_selection() && !currentPath().isEmpty();
    });
}

bool MenuRegistrar::handleDeleteKey()
{
    TextSession* session = tools_ ? tools_->textSession() : nullptr;
    if (session && session->textActive()) {
        return false;
    }
    if (isTextInput(QApplication::focusWidget())) {
        return false;
    }
    // An active lasso trace, a selected slice, or a selected path component
    // owns the key.
    if (tools_ && tools_->handleDeletePoint()) {
        return true;
    }
    // The mask edit target owns the key: with a pixel selection, Delete fills
    // that selection's area of the mask with the background colour (as a
    // background-colour fill does); with no selection it removes the mask (no
    // apply, no confirmation), ahead of the pixel-selection and layer branches.
    if (panels_->layersPanel() && panels_->layersPanel()->maskEditing()) {
        PictureView* view = activeView();
        if (view && view->has_selection() && tools_) {
            const QColor background = tools_->background();
            edit_fill(*view, uint32_t(background.rgba()), -1, QStringLiteral("normal"), 100,
                      false);
        } else {
            panels_->layersPanel()->deleteActiveMask();
        }
        refresh();
        return true;
    }
    PictureView* view = activeView();
    if (view && view->has_selection()) {
        commands_->dispatch(QLatin1String(command_ids::EditClear));
        return true;
    }
    if (panels_->layersPanel()) {
        panels_->layersPanel()->deleteSelection();
        return true;
    }
    return false;
}

// Edit > Undo/Redo/Step. While the active tool holds a modal session (the Crop
// straighten steps) the session is undone first; otherwise the document history.
void MenuRegistrar::wireEditHistory()
{
    commands_->setHandler(command_ids::EditUndo, [this]() {
        if (tools_ && tools_->toolUndo()) {
            refresh();
            return;
        }
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        const bool undo = history_api::canUndo(*view);
        if (!(undo ? history_api::undo(*view) : history_api::redo(*view))) {
            reportMemoryRefusal(dialogParent(), undo ? tr("undo") : tr("redo"));
        }
    });
    commands_->setEnabledProvider(command_ids::EditUndo, [this]() {
        PictureView* view = activeView();
        return (tools_ && tools_->canToolUndo())
            || (view && (history_api::canUndo(*view) || history_api::canRedo(*view)));
    });
    commands_->setLabelProvider(command_ids::EditUndo, [this]() {
        if (tools_ && tools_->canToolUndo()) {
            return QStringLiteral("Undo");
        }
        PictureView* view = activeView();
        if (!view) {
            return QStringLiteral("Undo");
        }
        if (history_api::canUndo(*view)) {
            const int index = history_api::index(*view);
            return QStringLiteral("Undo %1").arg(history_api::label(*view, index));
        }
        if (history_api::canRedo(*view)) {
            const int index = history_api::index(*view) + 1;
            return QStringLiteral("Redo %1").arg(history_api::label(*view, index));
        }
        return QStringLiteral("Undo");
    });

    commands_->setHandler(command_ids::EditRedo, [this]() {
        if (tools_ && tools_->toolRedo()) {
            refresh();
            return;
        }
        if (PictureView* view = activeView(); view && !history_api::redo(*view)) {
            reportMemoryRefusal(dialogParent(), tr("redo"));
        }
    });
    commands_->setEnabledProvider(command_ids::EditRedo, [this]() {
        return (tools_ && tools_->canToolRedo())
            || (activeView() && history_api::canRedo(*activeView()));
    });

    commands_->setHandler(command_ids::EditStepBackward, [this]() {
        if (PictureView* view = activeView(); view && !history_api::undo(*view)) {
            reportMemoryRefusal(dialogParent(), tr("undo"));
        }
    });
    commands_->setEnabledProvider(command_ids::EditStepBackward,
                                  [this]() { return activeView() && history_api::canUndo(*activeView()); });

    commands_->setHandler(command_ids::EditStepForward, [this]() {
        if (PictureView* view = activeView(); view && !history_api::redo(*view)) {
            reportMemoryRefusal(dialogParent(), tr("redo"));
        }
    });
    commands_->setEnabledProvider(command_ids::EditStepForward,
                                  [this]() { return activeView() && history_api::canRedo(*activeView()); });
}

} // namespace pictura
