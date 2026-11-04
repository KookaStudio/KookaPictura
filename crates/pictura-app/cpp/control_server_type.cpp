// `type_op`: add and re-set type layers over the control server, so
// scripts/profile-tools.py can time text edits the way the Type panels run them.

#include "control_server.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtCore/QString>

#include "api/type_api.h"
#include "fonts.h"
#include "frame.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

namespace pictura {

namespace {

type_api::TypeSpec newSpec(const QJsonObject& params)
{
    TypeSetting setting{};
    setting.x = params.value(QStringLiteral("x")).toDouble(100.0);
    setting.y = params.value(QStringLiteral("y")).toDouble(200.0);
    setting.xx = 1.0;
    setting.yy = 1.0;
    const QJsonArray box = params.value(QStringLiteral("box")).toArray();
    if (box.size() == 4) {
        setting.boxed = true;
        setting.box_left = box[0].toDouble();
        setting.box_top = box[1].toDouble();
        setting.box_right = box[2].toDouble();
        setting.box_bottom = box[3].toDouble();
    }
    CharacterSetting character = type_api::defaultCharacterSetting();
    character.size = params.value(QStringLiteral("size")).toDouble(72.0);
    return {params.value(QStringLiteral("text")).toString(QStringLiteral("Lorem Ipsum Pictura")),
            params.value(QStringLiteral("font")).toString(QString::fromLatin1(kBundledUiFamily)),
            setting, character, type_api::defaultParagraphSetting()};
}

// Only the fields `params` names change, as a panel changes one field.
void restyle(type_api::TypeSpec& spec, const QJsonObject& params)
{
    spec.font = familyForFontName(spec.font);
    if (params.contains(QStringLiteral("font"))) {
        spec.font = params.value(QStringLiteral("font")).toString();
    }
    if (params.contains(QStringLiteral("size"))) {
        spec.character.size = params.value(QStringLiteral("size")).toDouble();
    }
    if (params.contains(QStringLiteral("color"))) {
        spec.character.color = quint32(params.value(QStringLiteral("color")).toInteger());
    }
    if (params.contains(QStringLiteral("justify"))) {
        spec.paragraph.justify = params.value(QStringLiteral("justify")).toInt();
    }
    if (params.contains(QStringLiteral("space_before"))) {
        spec.paragraph.space_before = params.value(QStringLiteral("space_before")).toDouble();
    }
    registerTypeFont(spec.font, spec.character.font_style);
}

} // namespace

QJsonObject ControlServer::methodTypeOp(const QJsonObject& params)
{
    PictureView* view = frame_->activeView();
    if (!view || !view->has_document()) {
        return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
    }
    const QString op = params.value(QStringLiteral("op")).toString();
    QJsonObject result;
    result.insert(QStringLiteral("ok"), true);
    if (op == QStringLiteral("add")) {
        const QString path = type_api::commitLayer(*view, newSpec(params));
        if (path.isEmpty()) {
            return error(QStringLiteral("refused"), QStringLiteral("add refused"));
        }
        result.insert(QStringLiteral("path"), path);
        return ok(result);
    }
    if (op == QStringLiteral("warp_cancel")) {
        type_warp_cancel(*view);
        return ok(result);
    }

    const QString path = params.value(QStringLiteral("path")).toString();
    const type_api::LayerType layer = type_api::readLayer(*view, path);
    if (!layer.isType) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("path is not a type layer"));
    }
    type_api::TypeSpec spec = layer.spec;
    restyle(spec, params);
    bool done = false;
    if (op == QStringLiteral("set")) {
        done = type_api::updateLayer(*view, path, spec);
    } else if (op == QStringLiteral("preview")) {
        done = type_api::previewLayer(*view, path, spec);
    } else if (op == QStringLiteral("warp_preview")) {
        done = type_warp_preview(*view, path, params.value(QStringLiteral("style")).toString(),
                                 params.value(QStringLiteral("bend")).toDouble(50.0), 0.0, 0.0,
                                 false);
    } else {
        return error(QStringLiteral("invalid_param"), QStringLiteral("unknown op: %1").arg(op));
    }
    if (!done) {
        return error(QStringLiteral("refused"), QStringLiteral("%1 refused").arg(op));
    }
    return ok(result);
}

} // namespace pictura
