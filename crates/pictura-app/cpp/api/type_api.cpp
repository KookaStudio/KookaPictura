#include "api/type_api.h"

#include <utility>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/pdf_export.cxxqt.h"
#include "pictura_app/src/cxxqt_object/type_tools/fonts.cxxqt.h"

namespace pictura::type_api {

namespace {

QVector<double> doubles(const ::rust::Vec<double>& values)
{
    QVector<double> out;
    out.reserve(int(values.size()));
    for (const double value : values) {
        out.append(value);
    }
    return out;
}

} // namespace

QString layerAt(const PictureView& view, double x, double y)
{
    return ::pictura::type_layer_at(view, x, y);
}

QVector<double> pathAt(const PictureView& view, double x, double y, double tolerance)
{
    return doubles(::pictura::type_path_at(view, x, y, tolerance));
}

LayerType readLayer(const PictureView& view, const QString& path)
{
    LayerType out;
    out.isType = view.layer_is_type(path);
    out.spec.text = ::pictura::type_layer_text(view, path);
    out.spec.font = ::pictura::type_layer_font(view, path);
    out.spec.setting = ::pictura::type_layer_setting(view, path);
    out.spec.character = ::pictura::type_layer_character_setting(view, path);
    out.spec.paragraph = ::pictura::type_layer_paragraph_setting(view, path);
    return out;
}

bool isWarped(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_is_warped(view, path);
}

CharacterSetting defaultCharacterSetting()
{
    return ::pictura::type_default_character_setting();
}

ParagraphSetting defaultParagraphSetting()
{
    return ::pictura::type_default_paragraph_setting();
}

bool registerFont(const QString& family, const QString& style,
                  ::rust::Slice<const std::uint8_t> bytes)
{
    return ::pictura::type_register_font(family, style, bytes);
}

std::uint32_t openTypeFeatures(const QString& family, const QString& style)
{
    return ::pictura::type_font_opentype_features(family, style);
}

QVector<int> previewRect(const TypeSpec& spec)
{
    const ::rust::Vec<std::int32_t> rect = ::pictura::type_preview_rect(
        spec.text, spec.font, spec.setting, spec.character, spec.paragraph);
    QVector<int> out;
    out.reserve(int(rect.size()));
    for (const std::int32_t value : rect) {
        out.append(int(value));
    }
    return out;
}

QVector<double> caretStops(const TypeSpec& spec)
{
    return doubles(::pictura::type_caret_stops(spec.text, spec.font, spec.setting, spec.character,
                                               spec.paragraph));
}

QVector<double> baselines(const TypeSpec& spec)
{
    return doubles(::pictura::type_baselines(spec.text, spec.font, spec.setting, spec.character,
                                             spec.paragraph));
}

bool overflows(const TypeSpec& spec)
{
    return ::pictura::type_overflows(spec.text, spec.font, spec.setting, spec.character,
                                     spec.paragraph);
}

QVector<double> pathOverflowPoint(const TypeSpec& spec)
{
    return doubles(::pictura::type_path_overflow_point(spec.text, spec.font, spec.setting,
                                                       spec.character, spec.paragraph));
}

QVector<double> localBounds(const TypeSpec& spec)
{
    return doubles(::pictura::type_local_bounds(spec.text, spec.font, spec.setting,
                                                spec.character, spec.paragraph));
}

bool isParagraph(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_is_paragraph(view, path);
}

bool convert(PictureView& view, const QString& path, bool paragraph)
{
    return ::pictura::type_convert(view, path, paragraph);
}

QByteArray previewRgba(const TypeSpec& spec)
{
    const ::rust::Vec<std::uint8_t> rgba = ::pictura::type_preview_rgba(
        spec.text, spec.font, spec.setting, spec.character, spec.paragraph);
    return QByteArray(reinterpret_cast<const char*>(rgba.data()), int(rgba.size()));
}

QString commitLayer(PictureView& view, const TypeSpec& spec)
{
    return ::pictura::type_commit_layer(view, spec.text, spec.font, spec.setting, spec.character,
                                        spec.paragraph);
}

bool commitMask(PictureView& view, const TypeSpec& spec, const QString& mode)
{
    return ::pictura::type_commit_mask(view, spec.text, spec.font, spec.setting, spec.character,
                                       spec.paragraph, mode);
}

bool updateLayer(PictureView& view, const QString& path, const TypeSpec& spec)
{
    return ::pictura::type_update_layer(view, path, spec.text, spec.font, spec.setting,
                                        spec.character, spec.paragraph);
}

bool previewLayer(PictureView& view, const QString& path, const TypeSpec& spec)
{
    return ::pictura::type_preview_layer(view, path, spec.text, spec.font, spec.setting,
                                         spec.character, spec.paragraph);
}

bool applyStyle(PictureView& view, const QString& path, const QString& name, bool paragraph)
{
    return ::pictura::type_apply_style(view, path, name, paragraph);
}

bool createCharacterStyle(PictureView& view, const QString& name, const QString& font,
                          const CharacterSetting& character)
{
    return ::pictura::type_create_character_style(view, name, font, character);
}

bool editCharacterStyle(PictureView& view, const QString& name, const QString& newName,
                        const QString& font, const CharacterSetting& character)
{
    return ::pictura::type_edit_character_style(view, name, newName, font, character);
}

bool previewCharacterStyle(PictureView& view, const QString& name, const QString& font,
                           const CharacterSetting& character)
{
    return ::pictura::type_preview_character_style(view, name, font, character);
}

bool deleteCharacterStyle(PictureView& view, const QString& name)
{
    return ::pictura::type_delete_character_style(view, name);
}

QVector<CharacterStyle> characterStyles(const PictureView& view)
{
    QVector<CharacterStyle> out;
    for (const ::pictura::CharacterStyleRow& row : ::pictura::type_character_styles(view)) {
        out.append({row.name, row.font, row.character});
    }
    return out;
}

bool createParagraphStyle(PictureView& view, const QString& name, const QString& font,
                          const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    return ::pictura::type_create_paragraph_style(view, name, font, character, paragraph);
}

bool editParagraphStyle(PictureView& view, const QString& name, const QString& font,
                        const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    return ::pictura::type_edit_paragraph_style(view, name, font, character, paragraph);
}

bool previewParagraphStyle(PictureView& view, const QString& name, const QString& font,
                           const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    return ::pictura::type_preview_paragraph_style(view, name, font, character, paragraph);
}

bool deleteParagraphStyle(PictureView& view, const QString& name)
{
    return ::pictura::type_delete_paragraph_style(view, name);
}

CharacterStyle findStyle(const QVector<CharacterStyle>& styles, const QString& name)
{
    for (const CharacterStyle& style : styles) {
        if (style.name == name) {
            return style;
        }
    }
    return {QString(), QString(), defaultCharacterSetting()};
}

ParagraphStyle findStyle(const QVector<ParagraphStyle>& styles, const QString& name)
{
    for (const ParagraphStyle& style : styles) {
        if (style.name == name) {
            return style;
        }
    }
    return {QString(), QString(), defaultCharacterSetting(), defaultParagraphSetting()};
}

QVector<ParagraphStyle> paragraphStyles(const PictureView& view)
{
    QVector<ParagraphStyle> out;
    for (const ::pictura::ParagraphStyleRow& row : ::pictura::type_paragraph_styles(view)) {
        out.append({row.name, row.font, row.character, row.paragraph});
    }
    return out;
}

bool rasterize(PictureView& view, const QString& path)
{
    return view.rasterize_type(path);
}

std::optional<EditSession> EditSession::begin(PictureView& view, const QString& path)
{
    if (!view.layer_is_type(path) || !::pictura::type_edit_begin(view, path)) {
        return std::nullopt;
    }
    return EditSession(view, path);
}

EditSession::EditSession(PictureView& view, const QString& path)
    : view_(&view)
    , path_(path)
    , open_(true)
{
}

EditSession::EditSession(EditSession&& other) noexcept
    : view_(other.view_)
    , path_(std::move(other.path_))
    , open_(other.open_)
{
    other.open_ = false;
}

EditSession& EditSession::operator=(EditSession&& other) noexcept
{
    if (this != &other) {
        cancel();
        view_ = other.view_;
        path_ = std::move(other.path_);
        open_ = other.open_;
        other.open_ = false;
    }
    return *this;
}

EditSession::~EditSession()
{
    cancel();
}

bool EditSession::commit(const TypeSpec& spec)
{
    if (!open_) {
        return false;
    }
    open_ = false;
    return view_ && ::pictura::type_commit_edit(*view_, path_, spec.text, spec.font, spec.setting,
                                                spec.character, spec.paragraph);
}

void EditSession::cancel()
{
    if (!open_) {
        return;
    }
    open_ = false;
    if (view_) {
        ::pictura::type_edit_cancel(*view_, path_);
    }
}

WarpSetting layerWarp(const PictureView& view, const QString& path)
{
    const TypeSetting s = ::pictura::type_layer_setting(view, path);
    return {s.warp_style, s.warp_bend, s.warp_h, s.warp_v, s.warp_vertical};
}

WarpRefusal warpRefusal(const PictureView& view, const QString& path)
{
    switch (::pictura::type_warp_refusal(view, path)) {
    case 0:
        return WarpRefusal::None;
    case 1:
        return WarpRefusal::OnPath;
    case 2:
        return WarpRefusal::FauxBold;
    case 3:
        return WarpRefusal::NoOutlines;
    default:
        return WarpRefusal::NotType;
    }
}

WarpSession::WarpSession(PictureView& view, const QString& path)
    : view_(&view)
    , path_(path)
{
}

WarpSession::~WarpSession()
{
    cancel();
}

bool WarpSession::preview(const WarpSetting& warp)
{
    if (!view_) {
        open_ = false;
    }
    if (!open_) {
        return false;
    }
    previewed_ = true;
    return ::pictura::type_warp_preview(*view_, path_, warp.style, warp.bend, warp.horizontal,
                                        warp.vertical, warp.verticalOrientation);
}

bool WarpSession::commit(const WarpSetting& warp)
{
    if (!open_ || !view_) {
        open_ = false;
        return false;
    }
    open_ = false;
    return ::pictura::type_warp_commit(*view_, path_, warp.style, warp.bend, warp.horizontal,
                                       warp.vertical, warp.verticalOrientation);
}

bool WarpSession::cancel()
{
    if (!open_) {
        return false;
    }
    open_ = false;
    return view_ && previewed_ && ::pictura::type_warp_cancel(*view_);
}

void setTextGamma(bool enabled, double gamma)
{
    ::pictura::type_set_text_gamma(enabled, gamma);
}

void refreshTextGamma(PictureView& view)
{
    ::pictura::type_text_gamma_refresh(view);
}

void setMissingGlyphProtection(bool on)
{
    ::pictura::type_set_missing_glyph_protection(on);
}

bool missingGlyphProtection()
{
    return ::pictura::type_missing_glyph_protection();
}

void refreshMissingGlyphProtection(PictureView& view)
{
    ::pictura::type_missing_glyph_protection_refresh(view);
}

QStringList documentFonts(const PictureView& view)
{
    return ::pictura::pdf_type_fonts(view).split(QLatin1Char('\n'), Qt::SkipEmptyParts);
}

bool layersEditable(const PictureView& view)
{
    return ::pictura::type_layers_editable(view);
}

int replaceFonts(PictureView& view, const QStringList& missing)
{
    return ::pictura::type_replace_fonts(view, missing.join(QLatin1Char('\n')));
}

} // namespace pictura::type_api
