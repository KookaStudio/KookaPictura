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
// re-sets it in place (`type_api::EditSession::commit`); the mask tools start new type.
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
#include "path_overlay.h"
#include "smart_guides.h"
#include "tools.h"
#include "type_fonts.h"
#include "type_frame.h"
#include "type_key_filter.h"
#include "type_text_edit.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "api/type_api.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QObject>
#include <QtCore/QtMath>
#include <QtCore/QPointer>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeyEvent>
#include <QtGui/QPolygonF>
#include <QtWidgets/QApplication>

#include <array>
#include <functional>
#include <memory>
#include <optional>

namespace pictura {

namespace {

const QString kPlaceholderText = QStringLiteral("Lorem Ipsum Pictura");

// The pointer zones `pointerCursorKind` reports (-1 is the text's I-beam).
constexpr int kCursorArrow = 0;
constexpr int kCursorMove = 1;
constexpr int kCursorHandle = 2;
constexpr int kCursorRotate = 3;
constexpr int kCursorPivot = 4;
constexpr int kCursorTypeOnPath = 5;

// The ink box of a rendered preview: the box round its non-transparent pixels
// inside `rect`, or `rect` itself when the buffer is missing or fully blank.
QRectF inkBox(const QRect& rect, const QByteArray& rgba)
{
    if (rect.isEmpty()) {
        return QRectF();
    }
    const int width = rect.width();
    const int height = rect.height();
    if (rgba.size() != width * height * 4) {
        return QRectF(rect);
    }
    int left = width;
    int right = -1;
    int top = height;
    int bottom = -1;
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            if (uchar(rgba.at((y * width + x) * 4 + 3)) != 0) {
                left = qMin(left, x);
                right = qMax(right, x);
                top = qMin(top, y);
                bottom = qMax(bottom, y);
            }
        }
    }
    if (right < 0) {
        return QRectF(rect);
    }
    return QRectF(rect.left() + left, rect.top() + top, right - left + 1, bottom - top + 1);
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

// The three settings a Type session edits: where the text sits, its character
// attributes, and its paragraph attributes.
struct TypeEdit {
    TypeSetting placement{};
    CharacterSetting character;
    ParagraphSetting paragraph;
};

class TypeToolHandler : public ToolHandler, public TextSession {
public:
    TypeToolHandler(ToolId id, std::shared_ptr<TypeGroup> type,
                    std::shared_ptr<const bool> placeholder)
        : toolVertical_(id == ToolId::VerticalType || id == ToolId::VerticalTypeMask)
        , mask_(id == ToolId::HorizontalTypeMask || id == ToolId::VerticalTypeMask)
        , type_(std::move(type))
        , placeholder_(std::move(placeholder))
    {
        addOptions(type_.get());
    }

    void onActivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        layerPath_.clear();
        loadLayerOptions();
        if (ImageView* canvas = ctx.canvas()) { showWorkPathOverlay(*canvas, ctx.view()); }
    }

    void onDeactivate(ToolServices& ctx) override
    {
        ctx_ = &ctx;
        commitText();
        if (ImageView* canvas = ctx.canvas()) {
            canvas->clearPathOverlay();
        }
        ctx_ = nullptr;
    }

    void onSessionCancelled(ToolServices&) override
    {
        if (active_) {
            if (editSession_) {
                editSession_->abandon();
            }
            endSession();
        }
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
        if (ImageView* canvas = ctx.canvas()) { showWorkPathOverlay(*canvas, ctx.view()); }
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
        if (syncing_ || !v || layerPath_.isEmpty()) {
            return;
        }
        const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*v, layerPath_);
        if (!layer.isType) {
            return;
        }
        TypeSetting s = layer.spec.setting;
        CharacterSetting c = layer.spec.character;
        ParagraphSetting p = layer.spec.paragraph;
        QString family = familyForFontName(layer.spec.font);
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
        registerTypeFont(family, c.font_style);
        pictura::type_api::updateLayer(*v, layerPath_, {layer.spec.text, family, s, c, p});
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
        // On the box: the reference point drags, a handle resizes (scales with
        // Ctrl, or on point type's Ctrl box), and just outside it rotates. A
        // click in the text (or the box) works it like any text field; one
        // elsewhere moves the text on a drag, or ends the edit.
        if (active_) {
            if (v == view_ && beginFrameGesture(imagePos, mods)) {
                return true;
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
            // Away from path text: a plain click commits. Away from other
            // type: a drag moves it, a plain click commits (decided on release).
            if (v == view_ && frame_.pathKind == 0) {
                movePending_ = true;
                moveStart_ = imagePos;
                moveOrigin_ = frame_.origin;
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
        QVector<double> path;
        if (hit.isEmpty() && !mods.testFlag(Qt::AltModifier)) {
            path = pictura::type_api::pathAt(*v, imagePos.x(), imagePos.y(), 4.0 / zoom());
        }
        beginSession(ctx, *v, imagePos, mods, hit, path);
        boxDrag_ = editPath_.isEmpty() && frame_.pathKind == 0;
        return true;
    }

    void onMove(ToolServices&, const QPointF& imagePos, Qt::KeyboardModifiers mods) override
    {
        if (!active_) {
            return;
        }
        setCtrlBox(mods.testFlag(Qt::ControlModifier));
        if (movePending_) {
            if (!movingText_ && QLineF(moveStart_, imagePos).length() * zoom() <= 3.0) {
                return;
            }
            if (!movingText_) {
                movingText_ = true;
                const TypeEdit edit = currentSetting();
                const QRect rect = previewRect(edit_.text(), edit);
                const QByteArray rgba =
                    rect.isEmpty()
                        ? QByteArray()
                        : pictura::type_api::previewRgba({edit_.text(), type_->get().family, edit.placement, edit.character, edit.paragraph});
                movingInk_ = inkBox(rect, rgba);
                movingInkOrigin_ = frame_.origin;
                referenceRects_ = ctx_ && ctx_->view() && ctx_->canvas()
                        && ctx_->canvas()->smartGuidesEnabled()
                    ? smartGuideReferences(*ctx_->view(), {editPath_})
                    : QList<QRectF>();
            }
            const QPointF next = moveOrigin_ + (imagePos - moveStart_);
            SmartGuideResult guides;
            if (ctx_ && ctx_->view() && ctx_->canvas()) {
                PictureView* v = ctx_->view();
                if (ctx_->canvas()->smartGuidesEnabled()) {
                    guides = smartGuideSnap(
                        movingInk_.translated(next - movingInkOrigin_), referenceRects_,
                        QSizeF(v->document_width(), v->document_height()),
                        ctx_->canvas()->zoom());
                }
                ctx_->canvas()->setSmartGuides(guides.lines, guides.gaps);
            }
            const QPointF snapped = next + guides.snap;
            frame_.movePivotBy(snapped - frame_.origin);
            frame_.origin = snapped;
            refreshOverlay();
            return;
        }
        if (frame_.gesture() != TypeFrame::Gesture::None) {
            frame_.drag(imagePos);
            showFrameReadout(imagePos);
            refreshOverlay();
        } else if (boxDrag_) {
            // Past four screen pixels the press becomes a paragraph box.
            if (!frame_.paragraph && QLineF(pressPos_, imagePos).length() * zoom() <= 4.0) {
                return;
            }
            frame_.paragraph = true;
            const QRectF r = QRectF(pressPos_, imagePos).normalized();
            frame_.origin = r.topLeft();
            frame_.box = {0.0, 0.0, qMax(r.width(), 1.0), qMax(r.height(), 1.0)};
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
            if (ImageView* canvas = ctx.canvas()) {
                canvas->clearSmartGuides();
            }
            if (!moved) {
                commitText();
            }
            return;
        }
        selecting_ = false;
        boxDrag_ = false;
        if (frame_.gesture() != TypeFrame::Gesture::None) {
            frame_.end();
            if (ImageView* canvas = ctx.canvas()) {
                canvas->clearDragSizeHint();
            }
            refreshOverlay();
            return;
        }
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
        frame_.paragraph = true;
        frame_.box = {0.0, 0.0, boxSize_.width(), boxSize_.height()};
        refreshOverlay();
    }

    bool commitText() override
    {
        if (!active_ || !ctx_) {
            return false;
        }
        const QPointer<PictureView> v = view_;
        std::optional<pictura::type_api::EditSession> session = std::move(editSession_);
        editSession_.reset();
        const bool current = v == ctx_->view();
        const QString text = edit_.text();
        const SelectionMode mode = mode_;
        const TypeEdit edit = currentSetting();
        const QString family = type_->get().family;
        registerTypeFont(family, edit.character.font_style);
        endSession();
        if (!v) {
            return true;
        }
        if (session) {
            const QString editPath = session->path();
            if (session->commit({text, family, edit.placement, edit.character, edit.paragraph})
                && current) {
                ctx_->notifyLayerCreated(editPath);
            }
            return true;
        }
        if (text.trimmed().isEmpty()) {
            return true;
        }
        if (mask_) {
            if (pictura::type_api::commitMask(*v, {text, family, edit.placement, edit.character, edit.paragraph}, selectionModeString(mode))) {
                ctx_->emitSelectionCommitted();
            }
            return true;
        }
        const QString created = pictura::type_api::commitLayer(*v, {text, family, edit.placement, edit.character, edit.paragraph});
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
        std::optional<pictura::type_api::EditSession> session = std::move(editSession_);
        editSession_.reset();
        endSession();
        if (session) {
            session->cancel();
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
        switch (pointerCursorKind(ctx, imagePos, mods)) {
        case kCursorArrow:
            out = QCursor(Qt::ArrowCursor);
            return true;
        case kCursorTypeOnPath: {
            const QCursor c = cursor(QStringLiteral("cursor.typeOnPath"), 10, 9);
            out = c.pixmap().isNull() ? QCursor(Qt::IBeamCursor) : c;
            return true;
        }
        case kCursorMove: {
            const ToolInfo& info = toolInfo(ToolId::Move);
            const QCursor c = cursor(toolCursorId(ToolId::Move, Qt::NoModifier), info.hotspotX,
                                     info.hotspotY);
            out = c.pixmap().isNull() ? QCursor(info.cursor) : c;
            return true;
        }
        case kCursorHandle: {
            // Corners diagonal, edges along their axis (TL, T, TR, R, BR, B, BL, L).
            static constexpr std::array<Qt::CursorShape, 8> kShapes{
                Qt::SizeFDiagCursor, Qt::SizeVerCursor, Qt::SizeBDiagCursor, Qt::SizeHorCursor,
                Qt::SizeFDiagCursor, Qt::SizeVerCursor, Qt::SizeBDiagCursor, Qt::SizeHorCursor};
            const int h = qBound(0, frame_.handleAt(imagePos, zoom()), 7);
            out = QCursor(kShapes[h]);
            return true;
        }
        case kCursorRotate:
            if (ImageView* canvas = ctx.canvas()) {
                out = rotateCursor(canvas->imageToWidget(frame_.pivotPoint()),
                                   canvas->imageToWidget(imagePos));
                return true;
            }
            return false;
        case kCursorPivot:
            out = QCursor(Qt::SizeAllCursor);
            return true;
        default:
            return false;
        }
    }

    // The zone a press at `imagePos` would act on: the text (-1, the I-beam),
    // a move, a box handle, the rotate band, or the reference point.
    int pointerCursorKind(const ToolServices& ctx, const QPointF& imagePos,
                          Qt::KeyboardModifiers) const override
    {
        if (!active_) {
            PictureView* v = ctx.view();
            if (v && v->has_document()
                && pictura::type_api::layerAt(*v, imagePos.x(), imagePos.y()).isEmpty()
                && !pictura::type_api::pathAt(*v, imagePos.x(), imagePos.y(), 4.0 / zoom())
                        .isEmpty()) {
                return kCursorTypeOnPath;
            }
            return -1;
        }
        if (movingText_) {
            return kCursorMove;
        }
        if (frame_.pathKind != 0) {
            const bool onText = hitsText(imagePos) || insideBox(imagePos);
            return onText || selecting_ ? -1 : kCursorArrow;
        }
        switch (frame_.gesture()) {
        case TypeFrame::Gesture::Resize:
        case TypeFrame::Gesture::Scale:
            return kCursorHandle;
        case TypeFrame::Gesture::Rotate:
            return kCursorRotate;
        case TypeFrame::Gesture::Pivot:
            return kCursorPivot;
        case TypeFrame::Gesture::None:
            break;
        }
        if (frame_.onPivot(imagePos, zoom())) {
            return kCursorPivot;
        }
        if (frame_.handleAt(imagePos, zoom()) >= 0) {
            return kCursorHandle;
        }
        if (frame_.inRotateZone(imagePos, zoom())) {
            return kCursorRotate;
        }
        const bool onText = hitsText(imagePos) || insideBox(imagePos);
        return onText || selecting_ ? -1 : kCursorMove;
    }

    void onModifiers(ToolServices& ctx, Qt::KeyboardModifiers mods) override
    {
        ctx_ = &ctx;
        if (active_) {
            setCtrlBox(mods.testFlag(Qt::ControlModifier));
        }
    }

    bool newTextCharacter(CharacterSetting& character, QString& family) const override
    {
        if (!active_ || !editPath_.isEmpty() || mask_) {
            return false;
        }
        character = currentSetting().character;
        family = type_->get().family;
        return true;
    }

    // The options bar's fields (family, size, colour, anti-aliasing) stay in
    // the shared TypeOptions; the rest rides on the session.
    bool setNewTextCharacter(const CharacterSetting& character, const QString& family) override
    {
        if (!active_ || !editPath_.isEmpty() || mask_) {
            return false;
        }
        character_ = character;
        TypeOptions o = type_->get();
        o.family = family.isEmpty() ? o.family : family;
        o.size = character.size;
        o.color = QColor::fromRgba(character.color);
        o.antialias = character.anti_alias;
        type_->set(o);
        refreshOverlay();
        return true;
    }

    bool previewTextColor(const QColor& color) override
    {
        if (active_) {
            TypeOptions o = type_->get();
            o.color = color;
            type_->set(o);
            return true;
        }
        PictureView* v = ctx_ ? ctx_->view() : nullptr;
        if (!v || layerPath_.isEmpty()) {
            return false;
        }
        const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*v, layerPath_);
        if (!layer.isType) {
            return false;
        }
        CharacterSetting c = layer.spec.character;
        c.color = color.rgba();
        return pictura::type_api::previewLayer(
            *v, layerPath_, {layer.spec.text, familyForFontName(layer.spec.font), layer.spec.setting,
                             c, layer.spec.paragraph});
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
                      Qt::KeyboardModifiers mods, const QString& hit,
                      const QVector<double>& path = QVector<double>())
    {
        active_ = true;
        view_ = &v;
        frame_ = TypeFrame();
        frame_.origin = pos;
        character_.reset();
        // New type starts with its placeholder selected, so typing replaces
        // it (Photoshop CC, not CS6); the mask tools make selections, not
        // layers, and start empty.
        if (!mask_ && placeholder_ && *placeholder_) {
            edit_.reset(kPlaceholderText, 0);
            edit_.selectAll();
        } else {
            edit_.reset(QString(), 0);
        }
        vertical_ = toolVertical_;
        mode_ = ctx.resolveSelectionMode(mods, v.has_selection());
        if (!hit.isEmpty()) {
            editSession_ = pictura::type_api::EditSession::begin(v, hit);
        }
        if (editSession_) {
            editPath_ = hit;
            const pictura::type_api::TypeSpec layer = pictura::type_api::readLayer(v, hit).spec;
            edit_.reset(layer.text, 0);
            const TypeSetting s = layer.setting;
            const CharacterSetting c = layer.character;
            const ParagraphSetting p = layer.paragraph;
            vertical_ = s.vertical;
            frame_.load(s);
            TypeOptions o = type_->get();
            const QString family = familyForFontName(layer.font);
            if (!family.isEmpty()) {
                o.family = family;
            }
            o.size = c.size;
            o.justification = p.justify;
            o.color = QColor::fromRgba(c.color);
            o.antialias = c.anti_alias;
            type_->set(o);
        }
        if (frame_.pathKind == 0 && !path.isEmpty()) {
            applyPathHit(path);
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

    void applyPathHit(const QVector<double>& hit)
    {
        if (hit.size() < 4) {
            return;
        }
        frame_.pathKind = int(hit.at(0));
        frame_.pathClosed = hit.at(1) != 0.0;
        frame_.pathStart = hit.at(2);
        frame_.pathPoints = typePathPointString(hit);
        frame_.pathPolygon = typePathPolygon(frame_.pathPoints);
        if (frame_.pathKind == 2) {
            frame_.paragraph = true;
            const QRectF box = frame_.pathPolygon.boundingRect();
            frame_.origin = box.topLeft();
            frame_.box = {0.0, 0.0, box.width(), box.height()};
        } else {
            frame_.paragraph = false;
            frame_.box = {0.0, 0.0, 0.0, 0.0};
        }
    }

    bool insideBox(const QPointF& pos) const
    {
        if (frame_.pathKind == 2) {
            return frame_.pathPolygon.containsPoint(pos, Qt::OddEvenFill);
        }
        return frame_.paragraph && frame_.polygon().containsPoint(pos, Qt::OddEvenFill);
    }

    // Start a box gesture at `pos` when it lands on the reference point, a
    // handle, or the rotate band.
    bool beginFrameGesture(const QPointF& pos, Qt::KeyboardModifiers mods)
    {
        if (frame_.pathKind != 0) {
            return false;
        }
        setCtrlBox(mods.testFlag(Qt::ControlModifier));
        if (!frame_.frameBox()) {
            return false;
        }
        using Gesture = TypeFrame::Gesture;
        if (frame_.onPivot(pos, zoom())) {
            frame_.begin(Gesture::Pivot, -1, pos);
            return true;
        }
        if (const int handle = frame_.handleAt(pos, zoom()); handle >= 0) {
            const bool resize = frame_.paragraph && !mods.testFlag(Qt::ControlModifier);
            frame_.begin(resize ? Gesture::Resize : Gesture::Scale, handle, pos);
            return true;
        }
        if (frame_.inRotateZone(pos, zoom())) {
            frame_.begin(Gesture::Rotate, -1, pos);
            return true;
        }
        return false;
    }

    // Point type shows its box while Ctrl is held (paragraph type always has
    // one); the box is the text's own bounds in its frame.
    void setCtrlBox(bool held)
    {
        if (frame_.pathKind != 0 || held == ctrlHeld_
            || frame_.gesture() != TypeFrame::Gesture::None) {
            return;
        }
        ctrlHeld_ = held;
        if (!frame_.paragraph) {
            refreshOverlay();
        }
    }

    // The readout by the pointer during a box gesture: W / H, the angle, or
    // the reference point's X / Y, in the rulers' unit.
    void showFrameReadout(const QPointF& pos)
    {
        ImageView* canvas = ctx_ ? ctx_->canvas() : nullptr;
        if (!canvas) {
            return;
        }
        QString text;
        switch (frame_.gesture()) {
        case TypeFrame::Gesture::Resize:
        case TypeFrame::Gesture::Scale:
            text = canvas->measureReadout(QStringLiteral("W"), frame_.width(), QStringLiteral("H"),
                                          frame_.height());
            break;
        case TypeFrame::Gesture::Rotate:
            text = QStringLiteral("∠: %1°").arg(frame_.angleDegrees(), 0, 'f', 1);
            break;
        case TypeFrame::Gesture::Pivot: {
            const QPointF p = frame_.pivotPoint();
            text = canvas->measureReadout(QStringLiteral("X"), p.x(), QStringLiteral("Y"), p.y());
            break;
        }
        case TypeFrame::Gesture::None:
            break;
        }
        canvas->setDragSizeHint(text, pos);
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
        const QString path = v ? layer_selection_anchor(*v) : QString();
        if (path == layerPath_) {
            return;
        }
        layerPath_ = path;
        const pictura::type_api::LayerType layer =
            path.isEmpty() ? pictura::type_api::LayerType{} : pictura::type_api::readLayer(*v, path);
        if (!layer.isType) {
            known_ = ctx_ ? type_->get() : TypeOptions{};
            return;
        }
        const CharacterSetting c = layer.spec.character;
        const ParagraphSetting p = layer.spec.paragraph;
        TypeOptions o = type_->get();
        const QString family = familyForFontName(layer.spec.font);
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
        editSession_.reset();
        character_.reset();
        edit_.reset(QString(), 0);
        stops_.clear();
        selecting_ = false;
        boxDrag_ = false;
        frame_ = TypeFrame();
        ctrlHeld_ = false;
        if (ctx_ && ctx_->canvas()) {
            ctx_->canvas()->clearDragSizeHint();
        }
        movePending_ = false;
        movingText_ = false;
        referenceRects_.clear();
        movingInk_ = QRectF();
        if (ctx_ && ctx_->canvas()) {
            ctx_->canvas()->clearSmartGuides();
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
        edit.character = character_ ? *character_ : pictura::type_api::defaultCharacterSetting();
        edit.paragraph = pictura::type_api::defaultParagraphSetting();
        // A reopened layer keeps its full attribute set; the bar overrides only
        // the fields it exposes.
        if (!editPath_.isEmpty() && view_) {
            const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*view_, editPath_);
            if (layer.isType) {
                edit.character = layer.spec.character;
                edit.paragraph = layer.spec.paragraph;
            }
        }
        edit.character.size = o.size;
        edit.character.color = o.color.rgba();
        edit.character.anti_alias = o.antialias;
        edit.paragraph.justify = o.justification;
        edit.placement.vertical = vertical_;
        frame_.store(edit.placement);
        return edit;
    }

    QRect previewRect(const QString& text, const TypeEdit& edit) const
    {
        const QVector<int> r = pictura::type_api::previewRect({text, type_->get().family, edit.placement, edit.character, edit.paragraph});
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
        overlay.pathKind = frame_.pathKind;
        overlay.canvas = QSize(view_->document_width(), view_->document_height());
        const QString& text = edit_.text();
        const QString family = type_->get().family;
        registerTypeFont(family, edit.character.font_style);
        stops_.clear();
        const QVector<double> stops = pictura::type_api::caretStops({text, family, edit.placement, edit.character, edit.paragraph});
        for (int i = 0; i + 3 < stops.size(); i += 4) {
            stops_.append(QLineF(stops[i], stops[i + 1], stops[i + 2], stops[i + 3]));
        }
        const QRect rect = previewRect(text, edit);
        const QByteArray rgba = rect.isEmpty()
            ? QByteArray()
            : pictura::type_api::previewRgba({text, family, edit.placement, edit.character, edit.paragraph});
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
        overlay.hasPlacement = !frame_.paragraph && frame_.pathKind == 0;
        overlay.placement = frame_.origin;
        const QVector<double> lines = pictura::type_api::baselines({text, family, edit.placement, edit.character, edit.paragraph});
        for (int i = 0; i + 3 < lines.size(); i += 4) {
            overlay.baselines.append(QLineF(lines[i], lines[i + 1], lines[i + 2], lines[i + 3]));
        }
        frame_.pointBox.reset();
        if (!frame_.paragraph && ctrlHeld_) {
            const QVector<double> b = pictura::type_api::localBounds({text, family, edit.placement, edit.character, edit.paragraph});
            if (b.size() == 4) {
                frame_.pointBox = std::array<double, 4>{b[0], b[1], b[2], b[3]};
            }
        }
        overlay.box = frame_.pathKind == 2 ? frame_.pathPolygon : frame_.polygon();
        if (!overlay.box.isEmpty() && frame_.pathKind == 0) {
            overlay.hasPivot = true;
            overlay.pivot = frame_.pivotPoint();
        }
        if (frame_.paragraph || frame_.pathKind == 1) {
            overlay.overflow = pictura::type_api::overflows({text, family, edit.placement, edit.character, edit.paragraph});
        }
        if (frame_.pathKind == 1) {
            const QVector<double> point = pictura::type_api::pathOverflowPoint({text, family, edit.placement, edit.character, edit.paragraph});
            if (point.size() == 2) {
                overlay.hasOverflowPoint = true;
                overlay.overflowPoint = QPointF(point[0], point[1]);
            }
        }
        canvas->setTypeOverlay(overlay);
    }

    const bool toolVertical_;
    const bool mask_;
    ToolServices* ctx_ = nullptr;
    bool active_ = false;
    // Guarded: the document may close while text is being typed.
    QPointer<PictureView> view_;
    // The reopened type layer, empty for new type, and its open edit session.
    QString editPath_;
    std::optional<pictura::type_api::EditSession> editSession_;
    bool vertical_ = false;
    // Where the text sits, its transform, paragraph type's box, the Ctrl box,
    // and the reference point, with the box gestures on them.
    TypeFrame frame_;
    TypeTextEdit edit_;
    // The caret segment at every UTF-16 boundary, from the last refresh.
    QList<QLineF> stops_;
    // True while a press in the text drags out a selection.
    bool selecting_ = false;
    // The press that opened new type, which a drag turns into a box; the box
    // handle being dragged (-1 none); an Alt-click waiting for its release.
    QPointF pressPos_;
    bool boxDrag_ = false;
    // Ctrl is held: point type shows its transform box.
    bool ctrlHeld_ = false;
    bool altClick_ = false;
    QSizeF boxSize_{200.0, 100.0};
    // A press away from the text: armed until release (a click commits), and
    // `movingText_` once it drags, carrying the origin from `moveOrigin_` by
    // the pointer's travel from `moveStart_`.
    bool movePending_ = false;
    bool movingText_ = false;
    QPointF moveStart_;
    QPointF moveOrigin_;
    // The text's ink box when the move began, and the origin it was measured
    // at; the reference boxes gathered then.
    QRectF movingInk_;
    QPointF movingInkOrigin_;
    QList<QRectF> referenceRects_;
    QElapsedTimer lastPress_;
    QPointF lastPos_;
    SelectionMode mode_ = SelectionMode::New;
    // The selected layer the bar last showed, and the options it last saw, so
    // a bar change applies only what changed.
    QString layerPath_;
    TypeOptions known_;
    // The Character panel's attributes for new type; unset until it edits them.
    std::optional<CharacterSetting> character_;
    bool syncing_ = false;
    std::unique_ptr<TypeKeyFilter> filter_;
    std::shared_ptr<TypeGroup> type_;
    std::shared_ptr<const bool> placeholder_;
};

} // namespace

std::unique_ptr<ToolHandler> makeTypeToolHandler(ToolId id, std::shared_ptr<TypeGroup> type,
                                                 std::shared_ptr<const bool> placeholder)
{
    return std::make_unique<TypeToolHandler>(id, std::move(type), std::move(placeholder));
}

} // namespace pictura
