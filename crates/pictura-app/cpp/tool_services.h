#pragma once

#include "tool_options.h"

#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/Qt>
#include <QtGui/QColor>

namespace pictura {

class ImageView;
class PictureView;

// The shared services a tool handler may use, implemented by `ToolController`.
// Options are not here: each handler owns or shares its family's
// `ToolOptions` group and reads it directly.
struct ToolServices {
    virtual ~ToolServices() = default;

    virtual PictureView* view() const = 0;
    virtual ImageView* canvas() const = 0;
    virtual void sampledForeground(const QColor& color) = 0;

    virtual QColor foreground() const = 0;
    virtual QColor background() const = 0;

    // A modal tool session's history changed; the frame re-evaluates commands.
    virtual void notifyToolSessionChanged() = 0;
    // Re-resolve the canvas cursor from the tool's current state and the live
    // pointer (a handler's `hoverCursor`).
    virtual void refreshCursor() = 0;
    // The Rotate View tool's canvas rotation in degrees clockwise.
    virtual double viewRotation() const = 0;
    virtual void setViewRotation(double degrees) = 0;

    // The controller's selection-move service: start a mask/content translate
    // from `imagePos`. The Move handler and the selection pre-block share it.
    virtual void beginContentMove(PictureView* v, const QPointF& imagePos, bool duplicate) = 0;

    virtual bool dragging() const = 0;
    virtual void setDragging(bool dragging) = 0;
    virtual bool dragCommitted() const = 0;
    virtual void setDragCommitted(bool committed) = 0;
    virtual SelectionMode dragMode() const = 0;
    virtual void setDragMode(SelectionMode mode) = 0;
    virtual SelectionMode resolveSelectionMode(Qt::KeyboardModifiers mods,
                                               bool hasExistingSelection) const = 0;

    // Annotations: re-read the document's color samplers and notes onto the
    // canvas; the note shown in the Notes panel (-1 none); the Ruler's line
    // changed, so its readouts must update.
    virtual void refreshAnnotations() = 0;
    virtual int currentNote() const = 0;
    virtual void setCurrentNote(int index) = 0;
    virtual void notifyRulerChanged() = 0;
    // A Count group or mark changed; the options bar re-reads its state.
    virtual void notifyCountChanged() = 0;

    virtual void refused(const QString& message) = 0;
    // A tool created the layer at `path`; the Layers panel selects it.
    virtual void notifyLayerCreated(const QString& path) = 0;
    // The Layers panel selects the existing layer at `path` (the Move tool's
    // Auto-Select hit).
    virtual void selectLayer(const QString& path) = 0;
    // A Type tool started or stopped taking keystrokes.
    virtual void notifyTextEditing(bool active) = 0;
    virtual void emitSelectionCommitted() = 0;

    // Canvas context-menu requests a handler raises; the frame wires them to the
    // widget that owns the menu, which the controller cannot reach.
    virtual void requestZoomMenu(const QPoint& globalPos) = 0;
    virtual void requestBrushPicker(const QPoint& globalPos) = 0;
    virtual void requestIsolationMode() = 0;
};

} // namespace pictura
