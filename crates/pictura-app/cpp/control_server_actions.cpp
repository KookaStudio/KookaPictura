#include "control_server.h"

#include <QtCore/QJsonObject>
#include <QtCore/QJsonValue>
#include <QtCore/QString>

#include "frame.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_path.cxxqt.h"

namespace pictura {

namespace {

// The app exposes tools by two names: the lowercase asset name `toolIdName`
// (also the `status` `active_tool` value, e.g. "marquee") and the display label
// `toolInfo(id).label` (e.g. "Marquee"). Accept either, case-insensitively.
bool toolIdFromString(const QString& name, ToolId& out)
{
    for (ToolId id : allToolIds()) {
        if (name.compare(toolIdName(id), Qt::CaseInsensitive) == 0
            || name.compare(QString::fromLatin1(toolInfo(id).label), Qt::CaseInsensitive) == 0) {
            out = id;
            return true;
        }
    }
    return false;
}

} // namespace

QJsonObject ControlServer::methodSetTool(const QJsonObject& params)
{
    if (!params.value(QStringLiteral("tool")).isString()) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("tool must be a string"));
    }
    const QString name = params.value(QStringLiteral("tool")).toString();
    ToolId id = ToolId::Move;
    if (!toolIdFromString(name, id)) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown tool: %1").arg(name));
    }
    frame_->setActiveTool(id);
    QJsonObject result;
    result.insert(QStringLiteral("active_tool"), toolIdName(frame_->activeTool()));
    return ok(result);
}

QJsonObject ControlServer::methodSelection(const QJsonObject& params)
{
    PictureView* view = frame_->activeView();
    if (!view || !view->has_document()) {
        return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
    }
    const QString op = params.value(QStringLiteral("op")).toString();
    if (op.isEmpty()) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("op is required"));
    }
    const int x = params.value(QStringLiteral("x")).toInt(0);
    const int y = params.value(QStringLiteral("y")).toInt(0);
    const int w = params.value(QStringLiteral("w")).toInt(0);
    const int h = params.value(QStringLiteral("h")).toInt(0);
    const QString mode = params.value(QStringLiteral("mode")).toString(QStringLiteral("new"));
    if (params.contains(QStringLiteral("mode")) && mode != QStringLiteral("new")
        && mode != QStringLiteral("add") && mode != QStringLiteral("subtract")
        && mode != QStringLiteral("intersect")) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("mode must be new, add, subtract, or intersect"));
    }
    const int tolerance = params.value(QStringLiteral("tolerance")).toInt(0);
    const bool contiguous = params.value(QStringLiteral("contiguous")).toBool(true);
    const double feather = params.value(QStringLiteral("feather")).toDouble(0.0);

    // The bridge applies an empty shape for a non-positive rect/ellipse rather
    // than refusing it, so reject it here.
    if ((op == QStringLiteral("rect") || op == QStringLiteral("ellipse"))
        && (w <= 0 || h <= 0)) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("rect and ellipse require positive w and h"));
    }

    bool applied = true;
    if (op == QStringLiteral("all")) {
        view->select_all();
    } else if (op == QStringLiteral("deselect")) {
        view->deselect();
    } else if (op == QStringLiteral("rect")) {
        applied = view->select_rect(x, y, w, h, mode, feather);
    } else if (op == QStringLiteral("ellipse")) {
        applied = view->select_ellipse(x, y, w, h, mode, feather);
    } else if (op == QStringLiteral("lasso_begin")) {
        applied = view->begin_lasso(mode);
    } else if (op == QStringLiteral("lasso_point")) {
        view->lasso_add_point(x, y);
    } else if (op == QStringLiteral("lasso_end")) {
        applied = view->end_lasso(feather);
    } else if (op == QStringLiteral("quick")) {
        applied = view->quick_select(x, y, tolerance, mode);
    } else if (op == QStringLiteral("wand")) {
        applied = view->magic_wand(x, y, tolerance, contiguous, mode);
    } else {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown op: %1").arg(op));
    }
    if (!applied) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("selection op did not apply"));
    }

    QJsonObject result;
    result.insert(QStringLiteral("has_selection"), view->has_selection());
    result.insert(QStringLiteral("count"), view->selection_count());
    result.insert(QStringLiteral("bounds"), view->selection_bounds());
    return ok(result);
}

QJsonObject ControlServer::methodFilter(const QJsonObject& params)
{
    PictureView* view = frame_->activeView();
    if (!view || !view->has_document()) {
        return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
    }
    const QString kind = params.value(QStringLiteral("kind")).toString();
    if (kind.isEmpty()) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("kind is required"));
    }
    // The bridge's `apply_filter` returns false for an unknown kind *and* for an
    // engine refusal, so pre-check the app's refusal conditions (no editable
    // pixel target, hidden, or pixel-locked) to report `refused`; a false after
    // that is an unknown kind.
    //
    // ponytail: the app fixes the filter seed; typed params/seed are future work.
    const QString activePath = view->active_layer_path();
    const QString activeKind = layer_kind_path(*view, activePath);
    const bool editablePixel = !activeKind.isEmpty()
                               && activeKind != QStringLiteral("group")
                               && activeKind != QStringLiteral("adjustment");
    const bool pixelLocked = editablePixel && (layer_lock_path(*view, activePath) & 0x02) != 0;
    if (!editablePixel || !view->active_layer_visible() || pixelLocked) {
        return error(QStringLiteral("refused"),
                     QStringLiteral("filter target layer is unavailable, hidden, or locked"));
    }
    if (!view->apply_filter(kind)) {
        // ponytail: `apply_filter` also returns false for a malformed layer
        // (missing/short colour channels -> InvalidParams); without a known-kind
        // list that edge is not distinguished from an unknown kind.
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown filter kind: %1").arg(kind));
    }
    QJsonObject result;
    result.insert(QStringLiteral("ok"), true);
    result.insert(QStringLiteral("kind"), kind);
    return ok(result);
}

QJsonObject ControlServer::methodAdjustment(const QJsonObject& params)
{
    PictureView* view = frame_->activeView();
    if (!view || !view->has_document()) {
        return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
    }
    const QString kind = params.value(QStringLiteral("kind")).toString();
    if (kind.isEmpty()) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("kind is required"));
    }
    if (!view->add_adjustment(kind)) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown or refused adjustment kind: %1").arg(kind));
    }
    QJsonObject result;
    result.insert(QStringLiteral("ok"), true);
    result.insert(QStringLiteral("layers"), view->layer_count());
    return ok(result);
}

QJsonObject ControlServer::methodLayerOp(const QJsonObject& params)
{
    PictureView* view = frame_->activeView();
    if (!view || !view->has_document()) {
        return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
    }
    const QString op = params.value(QStringLiteral("op")).toString();
    if (op.isEmpty()) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("op is required"));
    }
    const int layerCount = view->layer_count();
    const int index = params.value(QStringLiteral("index")).toInt(-1);
    const bool indexValid = index >= 0 && index < layerCount;

    const auto invalid = [this](const QString& message) {
        return error(QStringLiteral("invalid_param"), message);
    };
    const auto refused = [this](const QString& message) {
        return error(QStringLiteral("refused"), message);
    };

    if (op == QStringLiteral("set_name")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        const QString name = params.value(QStringLiteral("name")).toString();
        if (name.isEmpty()) {
            return invalid(QStringLiteral("set_name requires a non-empty name"));
        }
        if (!view->set_layer_name(index, name)) {
            return refused(QStringLiteral("set_name refused"));
        }
    } else if (op == QStringLiteral("set_opacity")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (!view->set_layer_opacity(index, params.value(QStringLiteral("value")).toInt(255))) {
            return refused(QStringLiteral("set_opacity refused"));
        }
    } else if (op == QStringLiteral("set_visible")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        const QJsonValue visibleValue = params.contains(QStringLiteral("visible"))
                                            ? params.value(QStringLiteral("visible"))
                                            : params.value(QStringLiteral("on"));
        if (!visibleValue.isBool()) {
            return invalid(QStringLiteral("set_visible requires a boolean 'visible' or 'on'"));
        }
        view->set_layer_visible(index, visibleValue.toBool());
    } else if (op == QStringLiteral("set_blend")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (!view->set_layer_blend(index, params.value(QStringLiteral("key")).toString())) {
            return refused(QStringLiteral("set_blend refused"));
        }
    } else if (op == QStringLiteral("set_fill")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (!view->set_layer_fill(index, params.value(QStringLiteral("value")).toInt(255))) {
            return refused(QStringLiteral("set_fill refused"));
        }
    } else if (op == QStringLiteral("set_lock")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (!view->set_layer_lock(index, params.value(QStringLiteral("flag")).toString(),
                                  params.value(QStringLiteral("on")).toBool(true))) {
            return refused(QStringLiteral("set_lock refused (flag must be transparency, pixels, "
                                          "position, nesting, or all)"));
        }
    } else if (op == QStringLiteral("set_color")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (!view->set_layer_color(index, params.value(QStringLiteral("value")).toInt(0))) {
            return refused(QStringLiteral("set_color refused"));
        }
    } else if (op == QStringLiteral("move")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (!view->move_layer(index, params.value(QStringLiteral("delta")).toInt(0))) {
            return refused(QStringLiteral("move refused"));
        }
    } else if (op == QStringLiteral("translate")) {
        // `translate_layer` targets the active layer, so honor `index` by
        // making it active first (the bridge reserves `index` for this op).
        if (params.contains(QStringLiteral("index"))) {
            if (!indexValid) {
                return invalid(QStringLiteral("index out of range"));
            }
            view->set_active_layer(QString::number(index));
        }
        if (!view->translate_layer(params.value(QStringLiteral("dx")).toInt(0),
                                   params.value(QStringLiteral("dy")).toInt(0))) {
            return refused(QStringLiteral("translate refused"));
        }
    } else if (op == QStringLiteral("delete")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        view->remove_layer(index);
    } else if (op == QStringLiteral("duplicate")) {
        if (!indexValid) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (view->duplicate_layer(index) < 0) {
            return refused(QStringLiteral("duplicate refused"));
        }
    } else if (op == QStringLiteral("add")) {
        // `index` is the insertion anchor; -1 (absent) inserts at the top.
        if (index < -1 || index >= layerCount) {
            return invalid(QStringLiteral("index out of range"));
        }
        if (view->add_layer(index) < 0) {
            return error(QStringLiteral("internal"), QStringLiteral("add_layer failed"));
        }
    } else {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown op: %1").arg(op));
    }

    QJsonObject result;
    result.insert(QStringLiteral("ok"), true);
    result.insert(QStringLiteral("layers"), view->layer_count());
    return ok(result);
}

QJsonObject ControlServer::methodSetGpuCompute(const QJsonObject& params)
{
    PictureView* view = frame_->activeView();
    if (!view || !view->has_document()) {
        return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
    }
    if (!params.value(QStringLiteral("on")).isBool()) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("on must be a boolean"));
    }
    view->set_gpu_compute(params.value(QStringLiteral("on")).toBool());
    QJsonObject result;
    result.insert(QStringLiteral("gpu_compute"), view->gpu_compute());
    result.insert(QStringLiteral("backend"), view->active_backend());
    return ok(result);
}

} // namespace pictura
