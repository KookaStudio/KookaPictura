// The main window's own key handling: Space pan, the tool-hint keycaps, and
// the keys that reach the window rather than a focused widget (moved out of
// frame.cpp unchanged).

#include "frame_includes.h"

namespace pictura {

namespace {

// Keycap label for a key event, matching the labels in `toolHintEntries`; empty
// for a key that no hint shows.
QString hintKeyLabel(const QKeyEvent* event)
{
    switch (event->key()) {
    case Qt::Key_Shift:
        return QStringLiteral("Shift");
    case Qt::Key_Alt:
        return QStringLiteral("Alt");
    case Qt::Key_Control:
        return QStringLiteral("Ctrl");
    case Qt::Key_BracketLeft:
        return QStringLiteral("[");
    case Qt::Key_BracketRight:
        return QStringLiteral("]");
    case Qt::Key_Left:
    case Qt::Key_Right:
    case Qt::Key_Up:
    case Qt::Key_Down:
        return QStringLiteral("Arrows");
    default:
        break;
    }
    if (event->key() >= Qt::Key_A && event->key() <= Qt::Key_Z) {
        return QString(QChar(event->key()));
    }
    return QString();
}

} // namespace

void PicturaMainWindow::keyPressEvent(QKeyEvent* event)
{
    if (hintBar_) {
        hintBar_->setPressedKey(hintKeyLabel(event));
    }
    // Shift+Space is Help > Search's key; when unbound it must not start a pan.
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Space
        && !event->modifiers().testFlag(Qt::ShiftModifier)) {
        ImageView* canvas = imageView();
        // A modal transform keeps Space; the Move tool's controls box (on by
        // default) is passive and pans like the bare canvas.
        if (canvas
            && (!tools_ || !tools_->transformSessionActive() || tools_->transformControlsSession())) {
            canvas->setSpacePan(true);
            event->accept();
            return;
        }
    }
    if (tools_
        && (event->key() == Qt::Key_Shift || event->key() == Qt::Key_Alt
            || event->key() == Qt::Key_Control)) {
        tools_->refreshCursor();
    }
    if (!event->isAutoRepeat()
        && (event->key() == Qt::Key_Return || event->key() == Qt::Key_Enter)
        && tools_
        && (tools_->activeTool() == ToolId::Crop
            || tools_->activeTool() == ToolId::PerspectiveCrop)) {
        commitCrop();
        return;
    }
    if (!event->isAutoRepeat()
        && (event->key() == Qt::Key_Return || event->key() == Qt::Key_Enter)
        && tools_
        && tools_->handleFinishPath()) {
        return;
    }
    if (!event->isAutoRepeat() && tools_ && event->modifiers() == Qt::NoModifier
        && tools_->handleCropSettingsKey(event->key())) {
        event->accept();
        return;
    }
    if (dispatchCommandKey(event)) {
        return;
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Escape && tools_
        && tools_->handleEscape()) {
        return;
    }
    // Escape leaves isolation mode before it leaves a full-screen screen mode.
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Escape && releaseIsolation()) {
        return;
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Escape
        && screenMode_ != ScreenMode::Standard) {
        setScreenMode(ScreenMode::Standard);
        return;
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_F) {
        // Space+F cycles the pane colour; bare F (Shift reverses) is the screen
        // mode. A multi-key QShortcut on F would swallow the first press.
        if (ImageView* canvas = imageView(); canvas && canvas->spacePanForTest()) {
            cycleCanvasColor(!(event->modifiers() & Qt::ShiftModifier));
            return;
        }
        requestScreenMode(nextScreenMode(!(event->modifiers() & Qt::ShiftModifier)));
        return;
    }
    // EU/Scandinavian fallback for the `[`/`]` brush shortcuts: the US
    // QShortcuts above consume the event first, so this only runs when the
    // layout produced a different key value; the native scan code still maps.
    if (!event->isAutoRepeat() && tools_
        && tools_->applyBrushShortcut(event->key(), event->nativeScanCode(),
                                      event->modifiers().testFlag(Qt::ShiftModifier))) {
        return;
    }
    if (tools_ && tools_->handleNudgeKey(event->key(), event->modifiers())) {
        event->accept();
        return;
    }
    QMainWindow::keyPressEvent(event);
}

void PicturaMainWindow::keyReleaseEvent(QKeyEvent* event)
{
    if (hintBar_) {
        hintBar_->setPressedKey(QString());
    }
    if (!event->isAutoRepeat() && event->key() == Qt::Key_Space) {
        if (ImageView* canvas = imageView(); canvas && canvas->spacePanForTest()) {
            canvas->setSpacePan(false);
            canvas->setPanEnabled(tools_ && tools_->activeTool() == ToolId::Hand);
            if (tools_) {
                tools_->refreshCursor();
            }
        }
    }
    if (tools_
        && (event->key() == Qt::Key_Shift || event->key() == Qt::Key_Alt
            || event->key() == Qt::Key_Control)) {
        tools_->refreshCursor();
    }
    QMainWindow::keyReleaseEvent(event);
}

} // namespace pictura
