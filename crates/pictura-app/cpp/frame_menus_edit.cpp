#include "frame_includes.h"

#include "pictura_app/src/cxxqt_object/clipboard.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_history/purge.cxxqt.h"

#include <QtCore/QMimeData>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>

namespace pictura {

namespace {

bool systemClipboardHasImage()
{
    const QMimeData* data = QGuiApplication::clipboard()->mimeData();
    return data && data->hasImage();
}

} // namespace

// Put the bridge clipboard on the system clipboard so other applications can
// paste it. The bridge copy keeps its document position and selection coverage,
// which the system image cannot carry.
void PicturaMainWindow::exportClipboard()
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
void PicturaMainWindow::importSystemClipboard()
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
void PicturaMainWindow::registerEditHandlers()
{
    // The canvas zoom keys keep working behind a running dialog.
    setDialogNavigationKeys([window = QPointer<PicturaMainWindow>(this)](QKeyEvent* event) {
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
        window->registry()->dispatch(QLatin1String(id));
        return true;
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
        return layersPanel_ ? layersPanel_->currentPath() : QString();
    };
    const auto layerCommand = [this, currentPath](const auto& run) {
        PictureView* view = activeView();
        const QString path = currentPath();
        if (view && !path.isEmpty() && run(*view, path)) {
            refresh();
        }
    };
    registry_->setHandler(command_ids::EditCopy, [this, layerCommand]() {
        layerCommand([this](PictureView& view, const QString& path) {
            const bool copied = clipboard_copy(view, path, false);
            if (copied) {
                exportClipboard();
            }
            return copied;
        });
    });
    registry_->setHandler(command_ids::EditCut, [this, layerCommand]() {
        layerCommand([this](PictureView& view, const QString& path) {
            const bool cut = clipboard_cut(view, path);
            if (cut) {
                exportClipboard();
            }
            return cut;
        });
    });
    registry_->setHandler(command_ids::EditClear, [layerCommand]() {
        layerCommand(
            [](PictureView& view, const QString& path) { return clipboard_clear(view, path); });
    });
    registry_->setHandler(command_ids::EditCopyMerged, [this]() {
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
    registry_->setHandler(command_ids::EditPaste, [paste]() { paste(PasteKind::Plain); });
    registry_->setHandler(command_ids::EditPasteInPlace, [paste]() { paste(PasteKind::InPlace); });
    registry_->setHandler(command_ids::EditPasteInto, [paste]() { paste(PasteKind::Into); });
    registry_->setHandler(command_ids::EditPasteOutside, [paste]() { paste(PasteKind::Outside); });

    // Edit > Fill… fills the active pixel layer (within the selection) through
    // the fill engine. The dialog's contents, mode, opacity, and Preserve
    // Transparency drive one "Fill" state.
    registry_->setHandler(command_ids::EditFill, [this, currentPath]() {
        PictureView* view = activeView();
        if (!view || !view->has_document() || !tools_ || currentPath().isEmpty()) {
            return;
        }
        FillDialog dialog(tools_->foreground(), tools_->background(), this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return;
        }
        const bool pattern = dialog.isPatternFill();
        const QColor color = dialog.fillColor();
        const int patternIndex = pattern ? dialog.selectedPatternIndex() : -1;
        const int argb = pattern ? 0 : int(color.rgba());
        if (edit_fill(*view, uint32_t(argb), patternIndex, dialog.blendMode(),
                      dialog.opacity(), dialog.preserveTransparency())) {
            refresh();
        }
    });

    // Edit > Stroke… outlines the active selection on the active pixel layer
    // through the stroke engine. Width, colour, location, mode, and opacity
    // drive one "Stroke" state.
    registry_->setHandler(command_ids::EditStroke, [this, currentPath]() {
        PictureView* view = activeView();
        if (!view || !view->has_document() || !view->has_selection()
            || currentPath().isEmpty()) {
            return;
        }
        StrokeDialog dialog(tools_ ? tools_->foreground() : QColor(Qt::black), this);
        if (runDialog(dialog, this) != QDialog::Accepted) {
            return;
        }
        const int argb = int(dialog.strokeColor().rgba());
        if (edit_stroke(*view, uint32_t(argb), dialog.strokeWidth(), dialog.location(),
                        dialog.blendMode(), dialog.opacity())) {
            refresh();
        }
    });

    // Purge drops our copy and, if the system clipboard still holds our export,
    // that too; another application's clipboard is left alone.
    registry_->setHandler(command_ids::EditPurgeClipboard, [this]() {
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
        registry_->setEnabledProvider(
            id, [hasDocument, currentPath]() { return hasDocument() && !currentPath().isEmpty(); });
    }
    registry_->setEnabledProvider(command_ids::EditCopyMerged, hasDocument);
    for (const char* id : {command_ids::EditPaste, command_ids::EditPasteInPlace}) {
        registry_->setEnabledProvider(
            id, [hasDocument, hasContents]() { return hasDocument() && hasContents(); });
    }
    for (const char* id : {command_ids::EditPasteInto, command_ids::EditPasteOutside}) {
        registry_->setEnabledProvider(id, [this, hasDocument, hasContents]() {
            return hasDocument() && hasContents() && activeView()->has_selection();
        });
    }
    registry_->setEnabledProvider(command_ids::EditPurgeClipboard,
                                  []() { return clipboard_has_contents(); });

    // Purge Undo drops the undo stack; Purge Histories drops the named restore
    // points; Purge All does both (and the clipboard, as CS6's All does).
    const auto purgeView = [this](auto run) {
        PictureView* view = activeView();
        if (view && view->has_document() && run(*view)) {
            refresh();
        }
    };
    registry_->setHandler(command_ids::EditPurgeUndo,
                          [purgeView]() { purgeView([](PictureView& v) { return purge_history(v); }); });
    registry_->setHandler(command_ids::EditPurgeHistories, [purgeView]() {
        purgeView([](PictureView& v) { return purge_named_history(v); });
    });
    registry_->setHandler(command_ids::EditPurgeAll, [this, purgeView]() {
        purgeView([](PictureView& v) { return purge_all(v); });
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
        registry_->setEnabledProvider(id, purgeEnabled);
    }
    registry_->setEnabledProvider(command_ids::EditFill, [hasDocument, currentPath]() {
        return hasDocument() && !currentPath().isEmpty();
    });
    registry_->setEnabledProvider(command_ids::EditStroke, [this, hasDocument, currentPath]() {
        return hasDocument() && activeView()->has_selection() && !currentPath().isEmpty();
    });
}

bool PicturaMainWindow::dispatchCommandKey(QKeyEvent* event)
{
    // Typing `+` takes Shift on most layouts, so Zoom In's Ctrl++ arrives as
    // Ctrl+=, Ctrl+Shift+=, or keypad Ctrl++ and misses the menu shortcut;
    // only keys no shortcut or widget took reach the frame.
    const Qt::KeyboardModifiers mods = event->modifiers();
    if ((mods & Qt::ControlModifier) && !(mods & Qt::AltModifier)
        && (event->key() == Qt::Key_Equal || event->key() == Qt::Key_Plus)) {
        registry_->dispatch(QLatin1String(command_ids::ViewZoomIn));
        event->accept();
        return true;
    }
    // CS6: with a selection, Delete / Backspace clear it (Edit > Clear) once
    // no tool has claimed the key.
    PictureView* view = activeView();
    if (!event->isAutoRepeat() && mods == Qt::NoModifier
        && (event->key() == Qt::Key_Delete || event->key() == Qt::Key_Backspace) && view
        && view->has_selection()) {
        registry_->dispatch(QLatin1String(command_ids::EditClear));
        event->accept();
        return true;
    }
    return false;
}

bool PicturaMainWindow::copyLayerFromDocument(const void* source, const QString& path)
{
    PictureView* target = activeView();
    PictureView* from = nullptr;
    for (const DocEntry& entry : docs_) {
        if (entry.view == source) {
            from = entry.view;
        }
    }
    // The bridge holds the source shared and the target exclusive, so the two
    // must be different documents.
    if (!target || !from || from == target || path.isEmpty()) {
        return false;
    }
    const QString selection = layersPanel_ ? layersPanel_->currentPath() : QString();
    const QString created = copy_layer_from_document(*target, *from, path, selection);
    if (created.isEmpty()) {
        return false;
    }
    refresh();
    selectLayerPath(created);
    return true;
}

// Edit > Undo/Redo/Step. While the active tool holds a modal session (the Crop
// straighten steps) the session is undone first; otherwise the document history.
void PicturaMainWindow::wireEditHistory()
{
    registry_->setHandler(command_ids::EditUndo, [this]() {
        if (tools_ && tools_->toolUndo()) {
            refresh();
            return;
        }
        PictureView* view = activeView();
        if (!view) {
            return;
        }
        view->can_undo() ? view->undo() : view->redo();
    });
    registry_->setEnabledProvider(command_ids::EditUndo, [this]() {
        PictureView* view = activeView();
        return (tools_ && tools_->canToolUndo()) || (view && (view->can_undo() || view->can_redo()));
    });
    registry_->setLabelProvider(command_ids::EditUndo, [this]() {
        if (tools_ && tools_->canToolUndo()) {
            return QStringLiteral("Undo");
        }
        PictureView* view = activeView();
        return view && !view->can_undo() ? QStringLiteral("Redo") : QStringLiteral("Undo");
    });

    registry_->setHandler(command_ids::EditRedo, [this]() {
        if (tools_ && tools_->toolRedo()) {
            refresh();
            return;
        }
        if (PictureView* view = activeView()) {
            view->redo();
        }
    });
    registry_->setEnabledProvider(command_ids::EditRedo, [this]() {
        return (tools_ && tools_->canToolRedo()) || (activeView() && activeView()->can_redo());
    });

    registry_->setHandler(command_ids::EditStepBackward, [this]() {
        if (PictureView* view = activeView()) {
            view->undo();
        }
    });
    registry_->setEnabledProvider(command_ids::EditStepBackward,
                                  [this]() { return activeView() && activeView()->can_undo(); });

    registry_->setHandler(command_ids::EditStepForward, [this]() {
        if (PictureView* view = activeView()) {
            view->redo();
        }
    });
    registry_->setEnabledProvider(command_ids::EditStepForward,
                                  [this]() { return activeView() && activeView()->can_redo(); });
}

} // namespace pictura
