// The Type tools: Horizontal / Vertical Type and their Type Mask twins. A
// click opens a point-type session at the click; a drag opens a paragraph-type
// session in the dragged box, and an Alt-click (Horizontal / Vertical Type)
// asks for the box size first. The box shows eight handles that resize it and
// reflow the text, with a `+` on the bottom-right one while text overflows.
// Away from the text the pointer is the Move tool's: a drag there moves the
// whole text (a plain click still commits). The session edits like a text field (`TypeTextEdit`): click to place the caret, drag or Shift-click
// to select, double-click a word, arrows / Home / End (Ctrl by word or whole
// text, Shift to extend), Backspace / Delete, Ctrl+A / C / X / V, Enter for a
// new line or column; Ctrl+Enter or keypad Enter commits and Esc cancels. The
// caret stops come from the engine's layout (`type_api::caretStops`), and the
// canvas shows the engine's own render through
// `type_api::preview*`, so what is shown is what commits. Commit adds a type
// layer (`type_api::commitLayer`) or, for the mask tools, merges the type into
// the selection (`type_api::commitMask`). A click elsewhere, a tool switch, or
// a switch to another document commits. Clicking an existing type layer with
// Horizontal or Vertical Type reopens it: the layer is hidden while its text
// is retyped over it, in its own orientation and settings, and the commit
// re-sets it in place (`type_api::commitEdit`); the mask tools start new type.
// Ported from photorust's CanvasView type entry (typeKeyPress / commit).
//
// ponytail: no input-method composition, a reopened layer is retyped in one
// style (its first run's), and the box handles only resize (rotate and skew
// come from Free Transform on the committed layer).

#include "tool_handler.h"
#include "tool_sessions.h"

#include "icons.h"
#include "image_view.h"
#include "paragraph_text_size_dialog.h"
#include "text_input.h"
#include "tools.h"
#include "type_fonts.h"
#include "type_text_edit.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "api/type_api.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QObject>
#include <QtCore/QtMath>
#include <QtCore/QPointer>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeyEvent>
#include <QtGui/QPolygonF>
#include <QtWidgets/QApplication>

#include <array>
#include <functional>
#include <memory>

namespace pictura {

namespace {

bool commitKey(const QKeyEvent& key)
{
    const bool enter = key.key() == Qt::Key_Return || key.key() == Qt::Key_Enter;
    return enter
        && (key.modifiers().testFlag(Qt::ControlModifier)
            || key.modifiers().testFlag(Qt::KeypadModifier));
}

// The keys a typing session takes: the window's single-letter tool shortcuts
// go quiet, as CS6's do while there is a text cursor, and Ctrl+A / C / X / V
// work on the text. Other Ctrl/Meta chords still reach their shortcuts.
bool sessionKey(const QKeyEvent& key)
{
    if (commitKey(key)) {
        return true;
    }
    if (key.modifiers() & Qt::MetaModifier) {
        return false;
    }
    const bool ctrl = key.modifiers() & Qt::ControlModifier;
    switch (key.key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
    case Qt::Key_Escape:
    case Qt::Key_Backspace:
    case Qt::Key_Delete:
    case Qt::Key_Left:
    case Qt::Key_Right:
    case Qt::Key_Up:
    case Qt::Key_Down:
    case Qt::Key_Home:
    case Qt::Key_End:
        return true;
    case Qt::Key_A:
    case Qt::Key_C:
    case Qt::Key_X:
    case Qt::Key_V:
        if (ctrl) {
            return true;
        }
        break;
    default:
        break;
    }
    return !ctrl && !key.text().isEmpty() && key.text().at(0).isPrint();
}

double distanceToSegment(const QPointF& p, const QLineF& line)
{
    const QPointF d = line.p2() - line.p1();
    const double length2 = d.x() * d.x() + d.y() * d.y();
    double t = 0.0;
    if (length2 > 0.0) {
        const QPointF w = p - line.p1();
        t = qBound(0.0, (w.x() * d.x() + w.y() * d.y()) / length2, 1.0);
    }
    return QLineF(p, line.p1() + t * d).length();
}

// The box handles in overlay order (TL, T, TR, R, BR, B, BL, L) as the edge
// each moves across and down: -1 left / top, 1 right / bottom, 0 neither.
constexpr std::array<std::array<int, 2>, 8> kBoxHandles{
    {{-1, -1}, {0, -1}, {1, -1}, {1, 0}, {1, 1}, {0, 1}, {-1, 1}, {-1, 0}}};

// The three settings a Type session edits: where the text sits, its character
// attributes, and its paragraph attributes.
struct TypeEdit {
    TypeSetting placement;
    CharacterSetting character;
    ParagraphSetting paragraph;
};

// Routes the window's key events to the session while it is open: an
// application filter, because the shortcut map consults the focus widget,
// which need not be the canvas. Text fields keep their keys.
class TypeKeyFilter : public QObject {
public:
    TypeKeyFilter(std::function<QWidget*()> window, std::function<void(const QKeyEvent&)> key)
        : window_(std::move(window))
        , key_(std::move(key))
    {
    }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() != QEvent::ShortcutOverride && event->type() != QEvent::KeyPress) {
            return false;
        }
        auto* widget = qobject_cast<QWidget*>(watched);
        QWidget* window = window_();
        if (!widget || !window || widget->window() != window || isTextInput(widget)) {
            return false;
        }
        auto* key = static_cast<QKeyEvent*>(event);
        if (!sessionKey(*key)) {
            return false;
        }
        if (event->type() == QEvent::ShortcutOverride) {
            event->accept();
        } else {
            key_(*key);
        }
        return true;
    }

private:
    std::function<QWidget*()> window_;
    std::function<void(const QKeyEvent&)> key_;
};

class TypeToolHandler : public ToolHandler, public TextSession {
public:
    TypeToolHandler(ToolId id, std::shared_ptr<TypeGroup> type)
        : toolVertical_(id == ToolId::VerticalType || id == ToolId::VerticalTypeMask)
        , mask_(id == ToolId::HorizontalTypeMask || id == ToolId::VerticalTypeMask)
        , type_(std::move(type))
    {
        addOptions(type_.get());
    }

    void onActivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        layerPath_.clear();
        loadLayerOptions();
    }

    void onDeactivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        commitText();
        ctx_ = nullptr;
    }

    // Switching documents commits the session into the one it was typed in.
    void onDocumentRefreshed(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        if (active_ && ctx.view() != view_) {
            commitText();
        }
        if (!active_) {
            loadLayerOptions();
        }
    }

    // While typing, the bar restyles the text being typed. Otherwise a change
    // re-sets the selected type layer, as CS6's bar does with a type layer
    // selected: only the field that changed, as one state.
    void onOptionsChanged(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        const TypeOptions now = type_->get();
        const TypeOptions was = known_;
        known_ = now;
        if (active_) {
            refreshOverlay();
            return;
        }
        PictureView* v = ctx.view();
        if (syncing_ || !v || layerPath_.isEmpty() || !pictura::type_api::isType(*v, layerPath_)) {
            return;
        }
        TypeSetting s = pictura::type_api::layerSetting(*v, layerPath_);
        CharacterSetting c = pictura::type_api::layerCharacterSetting(*v, layerPath_);
        ParagraphSetting p = pictura::type_api::layerParagraphSetting(*v, layerPath_);
        QString family = familyForFontName(pictura::type_api::layerFont(*v, layerPath_));
        if (now.family != was.family) {
            family = now.family;
        }
        if (now.size != was.size) {
            c.size = now.size;
        }
        if (now.antialias != was.antialias) {
            c.anti_alias = now.antialias;
        }
        if (now.color != was.color) {
            c.color = now.color.rgba();
        }
        if (now.justification != was.justification) {
            p.justify = now.justification;
        }
        registerTypeFont(family);
        pictura::type_api::updateLayer(*v, layerPath_, pictura::type_api::layerText(*v, layerPath_), family, s, c, p);
    }

    bool onPress(ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        ctx_ = &ctx;
        PictureView* v = ctx.view();
        const bool doubleClick = lastPress_.isValid()
            && lastPress_.elapsed() < QApplication::doubleClickInterval()
            && QLineF(lastPos_, imagePos).length() * zoom() < 4.0;
        lastPress_.start();
        lastPos_ = imagePos;
        pressPos_ = imagePos;
        // A handle resizes the box; a click in the text (or anywhere in the
        // box) works it like any text field; one elsewhere ends the edit.
        if (active_) {
            if (v == view_ && boxed_) {
                handle_ = handleAt(imagePos);
                if (handle_ >= 0) {
                    return true;
                }
            }
            if (v == view_ && (hitsText(imagePos) || insideBox(imagePos))) {
                if (doubleClick) {
                    edit_.selectWord(indexAt(imagePos));
                } else {
                    edit_.moveTo(indexAt(imagePos), mods.testFlag(Qt::ShiftModifier));
                    selecting_ = true;
                }
                refreshOverlay();
                return true;
            }
            // Away from the text: a drag moves it, a plain click commits
            // (decided on release).
            if (v == view_) {
                movePending_ = true;
                moveStart_ = imagePos;
                moveOrigin_ = origin_;
                return true;
            }
            commitText();
            return true;
        }
        if (!v || !v->has_document()) {
            return true;
        }
        const QString hit = mask_ ? QString() : pictura::type_api::layerAt(*v, imagePos.x(), imagePos.y());
        // The mask tools keep Alt for selection subtract.
        if (hit.isEmpty() && !mask_ && mods.testFlag(Qt::AltModifier)) {
            altClick_ = true;
            return true;
        }
        beginSession(ctx, *v, imagePos, mods, hit);
        boxDrag_ = editPath_.isEmpty();
        return true;
    }

    void onMove(ToolServices&, const QPointF& imagePos, Qt::KeyboardModifiers) override
    {
        if (!active_) {
            return;
        }
        if (movePending_) {
            if (!movingText_ && QLineF(moveStart_, imagePos).length() * zoom() <= 3.0) {
                return;
            }
            movingText_ = true;
            origin_ = moveOrigin_ + (imagePos - moveStart_);
            refreshOverlay();
            return;
        }
        if (handle_ >= 0) {
            resizeBox(imagePos);
            refreshOverlay();
        } else if (boxDrag_) {
            // Past four screen pixels the press becomes a paragraph box.
            if (!boxed_ && QLineF(pressPos_, imagePos).length() * zoom() <= 4.0) {
                return;
            }
            boxed_ = true;
            const QRectF r = QRectF(pressPos_, imagePos).normalized();
            origin_ = r.topLeft();
            box_ = {0.0, 0.0, qMax(r.width(), 1.0), qMax(r.height(), 1.0)};
            refreshOverlay();
        } else if (selecting_) {
            edit_.moveTo(indexAt(imagePos), true);
            refreshOverlay();
        }
    }

    // An Alt-click asks for the box on release, as the shape tools' Create
    // dialogs do, and opens the box with its top-left at the click.
    void onRelease(ToolServices& ctx, const QPointF&, Qt::KeyboardModifiers) override
    {
        if (movePending_) {
            const bool moved = movingText_;
            movePending_ = false;
            movingText_ = false;
            ctx_ = &ctx;
            if (!moved) {
                commitText();
            }
            return;
        }
        selecting_ = false;
        boxDrag_ = false;
        handle_ = -1;
        if (!altClick_) {
            return;
        }
        altClick_ = false;
        ctx_ = &ctx;
        PictureView* v = ctx.view();
        if (!v || !v->has_document() || !execParagraphTextSizeDialog(boxSize_, ctx.canvas())) {
            return;
        }
        beginSession(ctx, *v, pressPos_, Qt::NoModifier, QString());
        boxed_ = true;
        box_ = {0.0, 0.0, boxSize_.width(), boxSize_.height()};
        refreshOverlay();
    }

    bool commitText() override
    {
        if (!active_ || !ctx_) {
            return false;
        }
        const QPointer<PictureView> v = view_;
        const QString editPath = editPath_;
        const bool current = v == ctx_->view();
        const QString text = edit_.text();
        const SelectionMode mode = mode_;
        const TypeEdit edit = currentSetting();
        const QString family = type_->get().family;
        registerTypeFont(family);
        endSession();
        if (!v) {
            return true;
        }
        if (!editPath.isEmpty()) {
            if (pictura::type_api::commitEdit(*v, editPath, text, family, edit.placement, edit.character,
                                 edit.paragraph)
                && current) {
                ctx_->notifyLayerCreated(editPath);
            }
            return true;
        }
        if (text.trimmed().isEmpty()) {
            return true;
        }
        if (mask_) {
            if (pictura::type_api::commitMask(*v, text, family, edit.placement, edit.character, edit.paragraph,
                                 selectionModeString(mode))) {
                ctx_->emitSelectionCommitted();
            }
            return true;
        }
        const QString created = pictura::type_api::commitLayer(*v, text, family, edit.placement, edit.character,
                                                  edit.paragraph);
        if (created.isEmpty()) {
            ctx_->refused(QStringLiteral("Type layers can only be added to an 8-bit RGB document"));
        } else if (current) {
            ctx_->notifyLayerCreated(created);
        }
        return true;
    }

    bool cancelText() override
    {
        if (!active_) {
            return false;
        }
        const QPointer<PictureView> v = view_;
        const QString editPath = editPath_;
        endSession();
        if (v && !editPath.isEmpty()) {
            pictura::type_api::cancelEdit(*v, editPath);
        }
        return true;
    }

    bool textActive() const override { return active_; }

    // While typing, the cursor follows the pointer: the Move tool's away from
    // the text (where a drag moves it), the tool's own I-beam over it.
    bool pointerCursor() const override { return active_; }

    bool hoverCursor(const ToolServices& ctx, const QPointF& imagePos, Qt::KeyboardModifiers mods,
                     QCursor& out) const override
    {
        if (pointerCursorKind(ctx, imagePos, mods) != 1) {
            return false;
        }
        const ToolInfo& info = toolInfo(ToolId::Move);
        const QCursor c = cursor(toolCursorId(ToolId::Move, Qt::NoModifier), info.hotspotX,
                                 info.hotspotY);
        out = c.pixmap().isNull() ? QCursor(info.cursor) : c;
        return true;
    }

    // 1 where a press would move the text, -1 elsewhere.
    int pointerCursorKind(const ToolServices&, const QPointF& imagePos,
                          Qt::KeyboardModifiers) const override
    {
        if (!active_) {
            return -1;
        }
        if (movingText_) {
            return 1;
        }
        const bool onText = hitsText(imagePos) || insideBox(imagePos)
            || (boxed_ && handleAt(imagePos) >= 0);
        return onText || selecting_ || handle_ >= 0 ? -1 : 1;
    }

    bool insertText(const QString& text) override
    {
        if (!active_) {
            return false;
        }
        edit_.insert(text);
        refreshOverlay();
        return true;
    }

private:
    // Open a session at `pos`: new type, or the type layer `hit` reopened in
    // its own orientation, box, and settings.
    void beginSession(ToolServices& ctx, PictureView& v, const QPointF& pos,
                      Qt::KeyboardModifiers mods, const QString& hit)
    {
        active_ = true;
        view_ = &v;
        origin_ = pos;
        edit_.reset(QString(), 0);
        vertical_ = toolVertical_;
        matrix_ = {1.0, 0.0, 0.0, 1.0};
        boxed_ = false;
        mode_ = ctx.resolveSelectionMode(mods, v.has_selection());
        if (!hit.isEmpty() && pictura::type_api::beginEdit(v, hit)) {
            editPath_ = hit;
            edit_.reset(pictura::type_api::layerText(v, hit), 0);
            const TypeSetting s = pictura::type_api::layerSetting(v, hit);
            const CharacterSetting c = pictura::type_api::layerCharacterSetting(v, hit);
            const ParagraphSetting p = pictura::type_api::layerParagraphSetting(v, hit);
            origin_ = QPointF(s.x, s.y);
            vertical_ = s.vertical;
            matrix_ = {s.xx, s.xy, s.yx, s.yy};
            boxed_ = s.boxed;
            box_ = {s.box_left, s.box_top, s.box_right, s.box_bottom};
            TypeOptions o = type_->get();
            const QString family = familyForFontName(pictura::type_api::layerFont(v, hit));
            if (!family.isEmpty()) {
                o.family = family;
            }
            o.size = c.size;
            o.justification = p.justify;
            o.color = QColor::fromRgba(c.color);
            o.antialias = c.anti_alias;
            type_->set(o);
        }
        filter_ = std::make_unique<TypeKeyFilter>(
            [this]() { return ctx_ && ctx_->canvas() ? ctx_->canvas()->window() : nullptr; },
            [this](const QKeyEvent& key) { keyPressed(key); });
        qApp->installEventFilter(filter_.get());
        ctx.notifyTextEditing(true);
        refreshOverlay();
        if (!editPath_.isEmpty()) {
            edit_.moveTo(indexAt(pos), false);
            selecting_ = true;
            refreshOverlay();
        }
    }

    // A point in the box's frame (relative to the origin, before the matrix)
    // in document pixels.
    QPointF boxPoint(double u, double v) const
    {
        return QPointF(origin_.x() + matrix_[0] * u + matrix_[2] * v,
                       origin_.y() + matrix_[1] * u + matrix_[3] * v);
    }

    QPolygonF boxPolygon() const
    {
        return QPolygonF({boxPoint(box_[0], box_[1]), boxPoint(box_[2], box_[1]),
                          boxPoint(box_[2], box_[3]), boxPoint(box_[0], box_[3])});
    }

    bool insideBox(const QPointF& pos) const
    {
        return boxed_ && boxPolygon().containsPoint(pos, Qt::OddEvenFill);
    }

    // The handle within five screen pixels of `pos`, or -1.
    int handleAt(const QPointF& pos) const
    {
        for (int i = 0; i < int(kBoxHandles.size()); ++i) {
            const auto [sx, sy] = kBoxHandles[i];
            const double u = sx < 0 ? box_[0] : sx > 0 ? box_[2] : (box_[0] + box_[2]) / 2.0;
            const double v = sy < 0 ? box_[1] : sy > 0 ? box_[3] : (box_[1] + box_[3]) / 2.0;
            if (QLineF(pos, boxPoint(u, v)).length() * zoom() <= 5.0) {
                return i;
            }
        }
        return -1;
    }

    // Move the dragged handle's edges to `pos`, mapped back into the box's
    // frame; the box keeps at least one pixel a side.
    void resizeBox(const QPointF& pos)
    {
        const double det = matrix_[0] * matrix_[3] - matrix_[2] * matrix_[1];
        if (qFuzzyIsNull(det)) {
            return;
        }
        const QPointF d = pos - origin_;
        const double u = (matrix_[3] * d.x() - matrix_[2] * d.y()) / det;
        const double v = (matrix_[0] * d.y() - matrix_[1] * d.x()) / det;
        const auto [sx, sy] = kBoxHandles[handle_];
        if (sx < 0) {
            box_[0] = qMin(u, box_[2] - 1.0);
        } else if (sx > 0) {
            box_[2] = qMax(u, box_[0] + 1.0);
        }
        if (sy < 0) {
            box_[1] = qMin(v, box_[3] - 1.0);
        } else if (sy > 0) {
            box_[3] = qMax(v, box_[1] + 1.0);
        }
    }

    void keyPressed(const QKeyEvent& key)
    {
        if (commitKey(key)) {
            commitText();
            return;
        }
        const bool extend = key.modifiers() & Qt::ShiftModifier;
        const bool ctrl = key.modifiers() & Qt::ControlModifier;
        switch (key.key()) {
        case Qt::Key_Escape:
            cancelText();
            return;
        case Qt::Key_Return:
        case Qt::Key_Enter:
            edit_.insert(QStringLiteral("\r"));
            break;
        case Qt::Key_Backspace:
            edit_.backspace();
            break;
        case Qt::Key_Delete:
            edit_.deleteForward();
            break;
        case Qt::Key_Left:
        case Qt::Key_Right:
        case Qt::Key_Up:
        case Qt::Key_Down: {
            // The arrows follow the text: along the line for horizontal type,
            // down the column for vertical, the crosswise pair stepping
            // between lines. Vertical columns run right to left, so Left is
            // the next one.
            const bool along = vertical_
                ? (key.key() == Qt::Key_Up || key.key() == Qt::Key_Down)
                : (key.key() == Qt::Key_Left || key.key() == Qt::Key_Right);
            int direction = (key.key() == Qt::Key_Right || key.key() == Qt::Key_Down) ? 1 : -1;
            if (vertical_ && !along) {
                direction = -direction;
            }
            if (along) {
                edit_.step(direction, ctrl, extend);
            } else {
                edit_.stepLine(direction, extend);
            }
            break;
        }
        case Qt::Key_Home:
            edit_.home(ctrl, extend);
            break;
        case Qt::Key_End:
            edit_.end(ctrl, extend);
            break;
        case Qt::Key_A:
        case Qt::Key_C:
        case Qt::Key_X:
        case Qt::Key_V:
            if (ctrl) {
                clipboardKey(key.key());
                break;
            }
            [[fallthrough]];
        default:
            edit_.insert(key.text());
            break;
        }
        refreshOverlay();
    }

    void clipboardKey(int key)
    {
        QClipboard* clipboard = QGuiApplication::clipboard();
        if (key == Qt::Key_A) {
            edit_.selectAll();
        } else if (key == Qt::Key_V) {
            edit_.insert(clipboard->text());
        } else if (edit_.hasSelection()) {
            QString copied = edit_.selectedText();
            copied.replace(QLatin1Char('\r'), QLatin1Char('\n'));
            clipboard->setText(copied);
            if (key == Qt::Key_X) {
                edit_.insert(QString());
            }
        }
    }

    double zoom() const
    {
        return ctx_ && ctx_->canvas() ? qMax(ctx_->canvas()->zoom(), 1e-6) : 1.0;
    }

    // The caret stop nearest `pos`: the near half of a letter puts the caret
    // before it, the far half after it.
    int indexAt(const QPointF& pos) const
    {
        int nearest = 0;
        double shortest = -1.0;
        for (int i = 0; i < stops_.size(); ++i) {
            const double d = distanceToSegment(pos, stops_.at(i));
            if (shortest < 0.0 || d < shortest) {
                shortest = d;
                nearest = i;
            }
        }
        return nearest;
    }

    // Within the box the caret stops span, give or take four screen pixels.
    bool hitsText(const QPointF& pos) const
    {
        QRectF box;
        for (const QLineF& stop : stops_) {
            box |= QRectF(stop.p1(), stop.p2()).normalized().adjusted(-0.5, -0.5, 0.5, 0.5);
        }
        const double slack = 4.0 / zoom();
        return box.adjusted(-slack, -slack, slack, slack).contains(pos);
    }

    // Whether the image under the caret is dark (mean luminance below half,
    // transparency read as the light checkerboard), so the caret draws white.
    bool darkUnder(const QLineF& caret) const
    {
        if (!view_) {
            return false;
        }
        double sum = 0.0;
        constexpr int kSamples = 5;
        for (int i = 0; i < kSamples; ++i) {
            const QPointF p = caret.pointAt((i + 0.5) / kSamples);
            const QColor c = QColor::fromRgba(view_->sample_argb(qFloor(p.x()), qFloor(p.y())));
            const double a = c.alphaF();
            sum += a * (0.299 * c.red() + 0.587 * c.green() + 0.114 * c.blue()) + (1.0 - a) * 255.0;
        }
        return sum / kSamples < 128.0;
    }

    QList<QPolygonF> selectionShapes() const
    {
        QList<QPolygonF> shapes;
        const QString& text = edit_.text();
        for (int i = edit_.selectionStart(); i < edit_.selectionEnd(); ++i) {
            if (text.at(i) == QLatin1Char('\r') || text.at(i).isLowSurrogate()) {
                continue;
            }
            const int next = text.at(i).isHighSurrogate() ? i + 2 : i + 1;
            if (next >= stops_.size()) {
                break;
            }
            const QLineF a = stops_.at(i);
            const QLineF b = stops_.at(next);
            shapes.append(QPolygonF({a.p1(), a.p2(), b.p2(), b.p1()}));
        }
        return shapes;
    }

    // Show the selected type layer's family, size, and alignment in the bar
    // (once per selection), without re-setting it.
    void loadLayerOptions()
    {
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        const QString path = v ? QString(v->active_layer_path()) : QString();
        if (path == layerPath_) {
            return;
        }
        layerPath_ = path;
        if (path.isEmpty() || !pictura::type_api::isType(*v, path)) {
            known_ = ctx_ ? type_->get() : TypeOptions{};
            return;
        }
        const CharacterSetting c = pictura::type_api::layerCharacterSetting(*v, path);
        const ParagraphSetting p = pictura::type_api::layerParagraphSetting(*v, path);
        TypeOptions o = type_->get();
        const QString family = familyForFontName(pictura::type_api::layerFont(*v, path));
        if (!family.isEmpty()) {
            o.family = family;
        }
        o.size = c.size;
        o.justification = p.justify;
        o.color = QColor::fromRgba(c.color);
        o.antialias = c.anti_alias;
        syncing_ = true;
        type_->set(o);
        syncing_ = false;
        known_ = type_->get();
    }

    void endSession()
    {
        const bool was = active_;
        if (filter_) {
            qApp->removeEventFilter(filter_.get());
            filter_.reset();
        }
        active_ = false;
        view_ = nullptr;
        editPath_.clear();
        edit_.reset(QString(), 0);
        stops_.clear();
        selecting_ = false;
        boxDrag_ = false;
        handle_ = -1;
        boxed_ = false;
        movePending_ = false;
        movingText_ = false;
        if (ctx_ && ctx_->canvas()) {
            ctx_->canvas()->clearTypeOverlay();
        }
        if (was && ctx_) {
            ctx_->notifyTextEditing(false);
        }
    }

    TypeEdit currentSetting() const
    {
        const TypeOptions o = type_->get();
        TypeEdit edit;
        edit.character = pictura::type_api::defaultCharacterSetting();
        edit.paragraph = pictura::type_api::defaultParagraphSetting();
        // A reopened layer keeps its full attribute set; the bar overrides only
        // the fields it exposes.
        if (!editPath_.isEmpty() && view_ && pictura::type_api::isType(*view_, editPath_)) {
            edit.character = pictura::type_api::layerCharacterSetting(*view_, editPath_);
            edit.paragraph = pictura::type_api::layerParagraphSetting(*view_, editPath_);
        }
        edit.character.size = o.size;
        edit.character.color = o.color.rgba();
        edit.character.anti_alias = o.antialias;
        edit.paragraph.justify = o.justification;
        edit.placement.vertical = vertical_;
        edit.placement.x = origin_.x();
        edit.placement.y = origin_.y();
        edit.placement.xx = matrix_[0];
        edit.placement.xy = matrix_[1];
        edit.placement.yx = matrix_[2];
        edit.placement.yy = matrix_[3];
        edit.placement.boxed = boxed_;
        edit.placement.box_left = box_[0];
        edit.placement.box_top = box_[1];
        edit.placement.box_right = box_[2];
        edit.placement.box_bottom = box_[3];
        return edit;
    }

    QRect previewRect(const QString& text, const TypeEdit& edit) const
    {
        const QVector<int> r = pictura::type_api::previewRect(
            text, type_->get().family, edit.placement, edit.character, edit.paragraph);
        return r.size() == 4 ? QRect(r[0], r[1], r[2], r[3]) : QRect();
    }

    void refreshOverlay()
    {
        ImageView* canvas = ctx_ ? ctx_->canvas() : nullptr;
        if (!view_ || !canvas) {
            return;
        }
        const TypeEdit edit = currentSetting();
        ImageView::TypeOverlay overlay;
        overlay.active = true;
        overlay.mask = mask_;
        overlay.canvas = QSize(view_->document_width(), view_->document_height());
        const QString& text = edit_.text();
        const QString family = type_->get().family;
        registerTypeFont(family);
        stops_.clear();
        const QVector<double> stops = pictura::type_api::caretStops(
            text, family, edit.placement, edit.character, edit.paragraph);
        for (int i = 0; i + 3 < stops.size(); i += 4) {
            stops_.append(QLineF(stops[i], stops[i + 1], stops[i + 2], stops[i + 3]));
        }
        const QRect rect = previewRect(text, edit);
        const QByteArray rgba = rect.isEmpty()
            ? QByteArray()
            : pictura::type_api::previewRgba(text, family, edit.placement, edit.character,
                                             edit.paragraph);
        const uchar* px = reinterpret_cast<const uchar*>(rgba.constData());
        if (std::size_t(rect.width()) * rect.height() * 4 == std::size_t(rgba.size())
            && !rect.isEmpty()) {
            QImage image(rect.size(), QImage::Format_RGBA8888);
            for (int y = 0; y < rect.height(); ++y) {
                uchar* row = image.scanLine(y);
                for (int x = 0; x < rect.width(); ++x) {
                    const std::size_t i = (std::size_t(y) * rect.width() + x) * 4;
                    if (mask_) {
                        row[4 * x] = 255;
                        row[4 * x + 1] = 0;
                        row[4 * x + 2] = 0;
                        row[4 * x + 3] = uchar((255 - px[i + 3]) / 2);
                    } else {
                        for (int c = 0; c < 4; ++c) {
                            row[4 * x + c] = px[i + c];
                        }
                    }
                }
            }
            overlay.image = image;
            overlay.topLeft = rect.topLeft();
        }
        if (edit_.caret() < stops_.size()) {
            overlay.caret = stops_.at(edit_.caret());
        }
        overlay.selection = selectionShapes();
        overlay.lightInk = darkUnder(overlay.caret);
        overlay.hasPlacement = !boxed_;
        overlay.placement = origin_;
        const QVector<double> lines = pictura::type_api::baselines(
            text, family, edit.placement, edit.character, edit.paragraph);
        for (int i = 0; i + 3 < lines.size(); i += 4) {
            overlay.baselines.append(QLineF(lines[i], lines[i + 1], lines[i + 2], lines[i + 3]));
        }
        if (boxed_) {
            overlay.box = boxPolygon();
            overlay.overflow = pictura::type_api::overflows(text, family, edit.placement,
                                                            edit.character, edit.paragraph);
        }
        canvas->setTypeOverlay(overlay);
    }

    const bool toolVertical_;
    const bool mask_;
    ToolServices* ctx_ = nullptr;
    bool active_ = false;
    // Guarded: the document may close while text is being typed.
    QPointer<PictureView> view_;
    // The reopened type layer, empty for new type.
    QString editPath_;
    bool vertical_ = false;
    // A reopened layer's linear transform (Free Transform's scale / rotation),
    // `xx, xy, yx, yy` as `TySh` orders it.
    std::array<double, 4> matrix_{1.0, 0.0, 0.0, 1.0};
    QPointF origin_;
    TypeTextEdit edit_;
    // The caret segment at every UTF-16 boundary, from the last refresh.
    QList<QLineF> stops_;
    // True while a press in the text drags out a selection.
    bool selecting_ = false;
    // Paragraph type: the box as left, top, right, bottom relative to the
    // origin, before the matrix.
    bool boxed_ = false;
    std::array<double, 4> box_{0.0, 0.0, 0.0, 0.0};
    // The press that opened new type, which a drag turns into a box; the box
    // handle being dragged (-1 none); an Alt-click waiting for its release.
    QPointF pressPos_;
    bool boxDrag_ = false;
    int handle_ = -1;
    bool altClick_ = false;
    QSizeF boxSize_{200.0, 100.0};
    // A press away from the text: armed until release (a click commits), and
    // `movingText_` once it drags, carrying the origin from `moveOrigin_` by
    // the pointer's travel from `moveStart_`.
    bool movePending_ = false;
    bool movingText_ = false;
    QPointF moveStart_;
    QPointF moveOrigin_;
    QElapsedTimer lastPress_;
    QPointF lastPos_;
    SelectionMode mode_ = SelectionMode::New;
    // The selected layer the bar last showed, and the options it last saw, so
    // a bar change applies only what changed.
    QString layerPath_;
    TypeOptions known_;
    bool syncing_ = false;
    std::unique_ptr<TypeKeyFilter> filter_;
    std::shared_ptr<TypeGroup> type_;
};

} // namespace

std::unique_ptr<ToolHandler> makeTypeToolHandler(ToolId id, std::shared_ptr<TypeGroup> type)
{
    return std::make_unique<TypeToolHandler>(id, std::move(type));
}

} // namespace pictura
