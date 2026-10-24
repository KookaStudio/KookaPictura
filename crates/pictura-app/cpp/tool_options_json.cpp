// Tool options ↔ the session store's "toolOptions" object. Each options struct
// lists its persisted fields once (`fields`); one visitor writes them and one
// reads them back, so a new field is persisted by adding one line. Mirrors of
// document state (the shape W / H readout, the text colour, the mixer
// reservoir) are left out: they follow the document, not the user's choice.

#include "tool_options_json.h"

#include "tools.h"

#include <QtCore/QJsonValue>

#include <type_traits>

namespace pictura {

namespace {

void put(QJsonObject& o, const char* key, bool v) { o.insert(QLatin1String(key), v); }
void put(QJsonObject& o, const char* key, int v) { o.insert(QLatin1String(key), v); }
void put(QJsonObject& o, const char* key, double v) { o.insert(QLatin1String(key), v); }
void put(QJsonObject& o, const char* key, const QString& v) { o.insert(QLatin1String(key), v); }
void put(QJsonObject& o, const char* key, const QColor& v)
{
    o.insert(QLatin1String(key), v.isValid() ? v.name(QColor::HexArgb) : QString());
}

void take(const QJsonValue& j, bool& v) { v = j.toBool(v); }
void take(const QJsonValue& j, int& v) { v = j.toInt(v); }
void take(const QJsonValue& j, double& v) { v = j.toDouble(v); }
void take(const QJsonValue& j, QString& v) { v = j.toString(v); }
void take(const QJsonValue& j, QColor& v)
{
    const QString name = j.toString();
    v = name.isEmpty() ? QColor() : QColor::fromString(name);
}

struct Writer {
    QJsonObject& out;
    template <typename V> void operator()(const char* key, V& value) { put(out, key, value); }
};

struct Reader {
    const QJsonObject& in;
    template <typename V> void operator()(const char* key, V& value)
    {
        const QJsonValue j = in.value(QLatin1String(key));
        if (!j.isUndefined() && !j.isNull()) {
            take(j, value);
        }
    }
};

template <typename F> void fields(MoveOptions& o, F&& f)
{
    f("autoSelectOn", o.autoSelectOn);
    f("autoSelect", o.autoSelect);
    f("showTransformControls", o.showTransformControls);
    f("alignTo", o.alignTo);
}

template <typename F> void fields(TypeOptions& o, F&& f)
{
    f("family", o.family);
    f("size", o.size);
    f("antialias", o.antialias);
    f("justification", o.justification);
}

template <typename F> void fields(MixerOptions& o, F&& f)
{
    f("wet", o.wet);
    f("load", o.load);
    f("mix", o.mix);
    f("flow", o.flow);
    f("loadAfterStroke", o.loadAfterStroke);
    f("cleanAfterStroke", o.cleanAfterStroke);
}

template <typename F> void fields(EyedropperOptions& o, F&& f)
{
    f("sampleSize", o.sampleSize);
    f("scope", o.scope);
    f("ring", o.ring);
}

template <typename F> void fields(PenOptions& o, F&& f)
{
    f("autoAddDelete", o.autoAddDelete);
    f("rubberBand", o.rubberBand);
    f("curveFit", o.curveFit);
    f("showBoundingBox", o.showBoundingBox);
}

template <typename F> void fields(ShapeOptions& o, F&& f)
{
    f("mode", o.mode);
    f("radius", o.radius);
    f("sides", o.sides);
    f("weight", o.weight);
    f("arrowStart", o.arrowStart);
    f("arrowEnd", o.arrowEnd);
    f("arrowWidth", o.arrowWidth);
    f("arrowLength", o.arrowLength);
    f("arrowConcavity", o.arrowConcavity);
    f("custom", o.custom);
    f("fillEnabled", o.fillEnabled);
    f("fillColor", o.fillColor);
    f("strokeEnabled", o.strokeEnabled);
    f("strokeColor", o.strokeColor);
    f("strokeWidth", o.strokeWidth);
    f("strokeAlign", o.strokeAlign);
    f("geometry", o.geometry);
    f("fixedWidth", o.fixedWidth);
    f("fixedHeight", o.fixedHeight);
    f("fromCenter", o.fromCenter);
    f("star", o.star);
    f("indent", o.indent);
    f("smoothCorners", o.smoothCorners);
    f("smoothIndents", o.smoothIndents);
    f("alignEdges", o.alignEdges);
    f("linkSize", o.linkSize);
}

template <typename F> void fields(CropSettings& o, F&& f)
{
    f("showCroppedArea", o.showCroppedArea);
    f("autoCenterPreview", o.autoCenterPreview);
    f("shieldEnabled", o.shieldEnabled);
    f("shieldCustomColor", o.shieldCustomColor);
    f("shieldColor", o.shieldColor);
    f("shieldOpacity", o.shieldOpacity);
    f("autoAdjustOpacity", o.autoAdjustOpacity);
    f("overlayShow", o.overlayShow);
    f("overlayOrientation", o.overlayOrientation);
}

template <typename F> void fields(HealingOptions& o, F&& f)
{
    f("spotType", o.spotType);
    f("aligned", o.aligned);
}

template <typename F> void fields(PatchOptions& o, F&& f)
{
    f("contentAware", o.contentAware);
    f("destination", o.destination);
    f("transparent", o.transparent);
}

template <typename F> void fields(ContentAwareMoveOptions& o, F&& f)
{
    f("extend", o.extend);
    f("adaptation", o.adaptation);
}

template <typename F> void fields(RedEyeOptions& o, F&& f)
{
    f("pupil", o.pupil);
    f("darken", o.darken);
}

template <typename F> void fields(HandOptions& o, F&& f) { f("scrollAll", o.scrollAll); }

template <typename F> void fields(ZoomOptions& o, F&& f)
{
    f("resizeToFit", o.resizeToFit);
    f("zoomAll", o.zoomAll);
}

template <typename F> void fields(ColorReplaceOptions& o, F&& f)
{
    f("mode", o.mode);
    f("sampling", o.sampling);
    f("limits", o.limits);
    f("tolerance", o.tolerance);
    f("antialias", o.antialias);
}

template <typename F> void fields(StampOptions& o, F&& f)
{
    f("cloneAligned", o.cloneAligned);
    f("cloneSample", o.cloneSample);
    f("ignoreAdjustments", o.ignoreAdjustments);
    f("pattern", o.pattern);
    f("patternAligned", o.patternAligned);
}

template <typename F> void fields(EraserOptions& o, F&& f)
{
    f("mode", o.mode);
    f("toHistory", o.toHistory);
}

template <typename F> void fields(BackgroundEraseOptions& o, F&& f)
{
    f("sampling", o.sampling);
    f("limits", o.limits);
    f("tolerance", o.tolerance);
    f("protectForeground", o.protectForeground);
}

template <typename F> void fields(MagicEraseOptions& o, F&& f)
{
    f("tolerance", o.tolerance);
    f("antialias", o.antialias);
    f("contiguous", o.contiguous);
    f("sampleAllLayers", o.sampleAllLayers);
    f("opacity", o.opacity);
}

template <typename F> void fields(ArtHistoryOptions& o, F&& f)
{
    f("style", o.style);
    f("area", o.area);
    f("tolerance", o.tolerance);
}

template <typename F> void fields(GradientOptions& o, F&& f)
{
    f("preset", o.preset);
    f("style", o.style);
    f("mode", o.mode);
    f("opacity", o.opacity);
    f("reverse", o.reverse);
    f("dither", o.dither);
    f("transparency", o.transparency);
}

template <typename F> void fields(BucketOptions& o, F&& f)
{
    f("fill", o.fill);
    f("pattern", o.pattern);
    f("mode", o.mode);
    f("opacity", o.opacity);
    f("tolerance", o.tolerance);
    f("antialias", o.antialias);
    f("contiguous", o.contiguous);
    f("allLayers", o.allLayers);
}

template <typename F> void fields(RetouchOptions& o, F&& f)
{
    f("strength", o.strength);
    f("mode", o.mode);
    f("sampleAllLayers", o.sampleAllLayers);
    f("protectDetail", o.protectDetail);
    f("fingerPainting", o.fingerPainting);
}

template <typename F> void fields(ToneOptions& o, F&& f)
{
    f("amount", o.amount);
    f("range", o.range);
    f("protectTones", o.protectTones);
    f("spongeMode", o.spongeMode);
    f("vibrance", o.vibrance);
}

// The paint family's shared brush, read through its getters and written back
// through its clamping setters.
struct PaintFields {
    int size, hardness, opacity, flow, roundness, tipAngle, spacing, grain;
    QString mode;
    bool autoErase, flipX, flipY, penTip, aliased, wetEdges;
    BrushDynamics dynamics;
};

template <typename F> void fields(PaintFields& o, F&& f)
{
    f("size", o.size);
    f("hardness", o.hardness);
    f("opacity", o.opacity);
    f("flow", o.flow);
    f("mode", o.mode);
    f("autoErase", o.autoErase);
    f("roundness", o.roundness);
    f("tipAngle", o.tipAngle);
    f("spacing", o.spacing);
    f("grain", o.grain);
    f("flipX", o.flipX);
    f("flipY", o.flipY);
    f("penTip", o.penTip);
    f("aliased", o.aliased);
    f("wetEdges", o.wetEdges);
    f("scatter", o.dynamics.scatter);
    f("count", o.dynamics.count);
    f("sizeJitter", o.dynamics.sizeJitter);
    f("angleJitter", o.dynamics.angleJitter);
    f("roundnessJitter", o.dynamics.roundnessJitter);
}

PaintFields paintFields(const PaintOptions& p)
{
    return {p.size(),     p.hardness(), p.opacity(), p.flow(),  p.roundness(),
            p.tipAngle(), p.spacing(),  p.grain(),   p.mode(),    p.autoErase(),
            p.flipX(),    p.flipY(),    p.penTip(),  p.aliased(), p.wetEdges(),
            p.dynamics()};
}

void applyPaint(PaintOptions& p, const PaintFields& v)
{
    p.setSize(v.size);
    p.setHardness(v.hardness);
    p.setOpacity(v.opacity);
    p.setFlow(v.flow);
    p.setMode(v.mode);
    p.setAutoErase(v.autoErase);
    p.setRoundness(v.roundness);
    p.setTipAngle(v.tipAngle);
    p.setSpacing(v.spacing);
    p.setGrain(v.grain);
    p.setFlip(v.flipX, v.flipY);
    p.setPenTip(v.penTip);
    p.setAliased(v.aliased);
    p.setWetEdges(v.wetEdges);
    p.setDynamics(v.dynamics);
}

// The selection family's options-bar fields.
struct SelectionFields {
    int combineMode, marqueeStyle, fixedSizeW, fixedSizeH, tolerance;
    double feather, fixedRatioW, fixedRatioH;
    bool contiguous;
    int magneticWidth, magneticContrast, magneticFrequency;
};

template <typename F> void fields(SelectionFields& o, F&& f)
{
    f("combineMode", o.combineMode);
    f("marqueeStyle", o.marqueeStyle);
    f("feather", o.feather);
    f("fixedRatioWidth", o.fixedRatioW);
    f("fixedRatioHeight", o.fixedRatioH);
    f("fixedSizeWidth", o.fixedSizeW);
    f("fixedSizeHeight", o.fixedSizeH);
    f("tolerance", o.tolerance);
    f("contiguous", o.contiguous);
    f("magneticWidth", o.magneticWidth);
    f("magneticContrast", o.magneticContrast);
    f("magneticFrequency", o.magneticFrequency);
}

SelectionFields selectionFields(const SelectionOptions& s)
{
    return {int(s.combineMode()),  int(s.marqueeStyle()), s.fixedSizeWidth(),
            s.fixedSizeHeight(),   s.tolerance(),         s.feather(),
            s.fixedRatioWidth(),   s.fixedRatioHeight(),  s.contiguous(),
            s.magneticWidth(),     s.magneticContrast(),  s.magneticFrequency()};
}

void applySelection(SelectionOptions& s, const SelectionFields& v)
{
    s.setCombineMode(SelectionMode(std::clamp(v.combineMode, 0, 3)));
    s.setMarqueeStyle(MarqueeStyle(std::clamp(v.marqueeStyle, 0, 2)));
    s.setFeather(v.feather);
    s.setFixedRatio(v.fixedRatioW, v.fixedRatioH);
    s.setFixedSize(v.fixedSizeW, v.fixedSizeH);
    s.setTolerance(v.tolerance);
    s.setContiguous(v.contiguous);
    s.setMagneticWidth(v.magneticWidth);
    s.setMagneticContrast(v.magneticContrast);
    s.setMagneticFrequency(v.magneticFrequency);
}

template <typename T> QJsonObject write(T value)
{
    QJsonObject out;
    fields(value, Writer{out});
    return out;
}

template <typename T> T read(const QJsonObject& in, T value)
{
    fields(value, Reader{in});
    return value;
}

// Each `ToolOption` group, by session key and the tool that owns it.
template <typename F> void eachGroup(F&& f)
{
    f("move", ToolId::Move, static_cast<MoveGroup*>(nullptr));
    f("type", ToolId::HorizontalType, static_cast<TypeGroup*>(nullptr));
    f("mixer", ToolId::MixerBrush, static_cast<MixerGroup*>(nullptr));
    f("eyedropper", ToolId::Eyedropper, static_cast<EyedropperGroup*>(nullptr));
    f("pen", ToolId::Pen, static_cast<PenGroup*>(nullptr));
    f("shape", ToolId::Rectangle, static_cast<ShapeGroup*>(nullptr));
    f("healing", ToolId::HealingBrush, static_cast<HealingGroup*>(nullptr));
    f("patch", ToolId::Patch, static_cast<PatchGroup*>(nullptr));
    f("contentAwareMove", ToolId::ContentAwareMove, static_cast<ContentAwareMoveGroup*>(nullptr));
    f("redEye", ToolId::RedEye, static_cast<RedEyeGroup*>(nullptr));
    f("colorReplace", ToolId::ColorReplacement, static_cast<ColorReplaceGroup*>(nullptr));
    f("stamp", ToolId::CloneStamp, static_cast<StampGroup*>(nullptr));
    f("eraser", ToolId::Eraser, static_cast<EraserGroup*>(nullptr));
    f("backgroundEraser", ToolId::BackgroundEraser, static_cast<BackgroundEraseGroup*>(nullptr));
    f("magicEraser", ToolId::MagicEraser, static_cast<MagicEraseGroup*>(nullptr));
    f("artHistory", ToolId::ArtHistoryBrush, static_cast<ArtHistoryGroup*>(nullptr));
    f("gradient", ToolId::Gradient, static_cast<GradientGroup*>(nullptr));
    f("bucket", ToolId::PaintBucket, static_cast<BucketGroup*>(nullptr));
    // Blur, Sharpen, and Smudge (and Dodge, Burn, and Sponge) keep their own.
    f("blur", ToolId::Blur, static_cast<RetouchGroup*>(nullptr));
    f("sharpen", ToolId::Sharpen, static_cast<RetouchGroup*>(nullptr));
    f("smudge", ToolId::Smudge, static_cast<RetouchGroup*>(nullptr));
    f("dodge", ToolId::Dodge, static_cast<ToneGroup*>(nullptr));
    f("burn", ToolId::Burn, static_cast<ToneGroup*>(nullptr));
    f("sponge", ToolId::Sponge, static_cast<ToneGroup*>(nullptr));
    f("hand", ToolId::Hand, static_cast<HandGroup*>(nullptr));
    f("zoom", ToolId::Zoom, static_cast<ZoomGroup*>(nullptr));
}

} // namespace

QJsonObject toolOptionsToJson(const ToolController& tools)
{
    QJsonObject out;
    eachGroup([&](const char* key, ToolId id, auto* tag) {
        using Group = std::remove_pointer_t<decltype(tag)>;
        if (const Group* group = tools.optionsFor<Group>(id)) {
            out.insert(QLatin1String(key), write(group->get()));
        }
    });
    if (const PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush)) {
        out.insert(QStringLiteral("brush"), write(paintFields(*paint)));
    }
    if (const SelectionOptions* s = tools.optionsFor<SelectionOptions>(ToolId::Marquee)) {
        out.insert(QStringLiteral("selection"), write(selectionFields(*s)));
    }
    if (const CropOptions* crop = tools.optionsFor<CropOptions>(ToolId::Crop)) {
        out.insert(QStringLiteral("crop"), write(crop->settings()));
    }
    return out;
}

void applyToolOptionsJson(ToolController& tools, const QJsonObject& json)
{
    eachGroup([&](const char* key, ToolId id, auto* tag) {
        using Group = std::remove_pointer_t<decltype(tag)>;
        const QJsonValue value = json.value(QLatin1String(key));
        Group* group = tools.optionsFor<Group>(id);
        if (group && value.isObject()) {
            group->update(clampOptions(read(value.toObject(), group->get())));
        }
    });
    const QJsonValue crop = json.value(QStringLiteral("crop"));
    if (CropOptions* c = tools.optionsFor<CropOptions>(ToolId::Crop); c && crop.isObject()) {
        c->setSettings(read(crop.toObject(), c->settings()));
    }
    const QJsonValue brush = json.value(QStringLiteral("brush"));
    if (PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush); paint && brush.isObject()) {
        applyPaint(*paint, read(brush.toObject(), paintFields(*paint)));
    }
    const QJsonValue selection = json.value(QStringLiteral("selection"));
    if (SelectionOptions* s = tools.optionsFor<SelectionOptions>(ToolId::Marquee);
        s && selection.isObject()) {
        applySelection(*s, read(selection.toObject(), selectionFields(*s)));
    }
}

} // namespace pictura
