#include "api/type_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

namespace pictura::type_api {

bool isType(const PictureView& view, const QString& path)
{
    return view.layer_is_type(path);
}

QString layerAt(const PictureView& view, double x, double y)
{
    return ::pictura::type_layer_at(view, x, y);
}

QVector<double> pathAt(const PictureView& view, double x, double y, double tolerance)
{
    const ::rust::Vec<double> values = ::pictura::type_path_at(view, x, y, tolerance);
    QVector<double> out;
    out.reserve(int(values.size()));
    for (const double value : values) {
        out.append(value);
    }
    return out;
}

QString layerText(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_text(view, path);
}

QString layerFont(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_font(view, path);
}

TypeSetting layerSetting(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_setting(view, path);
}

CharacterSetting layerCharacterSetting(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_character_setting(view, path);
}

ParagraphSetting layerParagraphSetting(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_paragraph_setting(view, path);
}

LayerType readLayer(const PictureView& view, const QString& path)
{
    LayerType out;
    out.isType = view.layer_is_type(path);
    out.text = layerText(view, path);
    out.font = layerFont(view, path);
    out.setting = layerSetting(view, path);
    out.character = layerCharacterSetting(view, path);
    out.paragraph = layerParagraphSetting(view, path);
    return out;
}

CharacterSetting defaultCharacterSetting()
{
    return ::pictura::type_default_character_setting();
}

ParagraphSetting defaultParagraphSetting()
{
    return ::pictura::type_default_paragraph_setting();
}

bool registerFont(const QString& family, ::rust::Slice<const std::uint8_t> bytes)
{
    return ::pictura::type_register_font(family, bytes);
}

std::uint32_t openTypeFeatures(const QString& family)
{
    return ::pictura::type_font_opentype_features(family);
}

QVector<int> previewRect(const QString& text, const QString& font, const TypeSetting& setting,
                         const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    const ::rust::Vec<std::int32_t> rect
        = ::pictura::type_preview_rect(text, font, setting, character, paragraph);
    QVector<int> out;
    out.reserve(int(rect.size()));
    for (const std::int32_t value : rect) {
        out.append(int(value));
    }
    return out;
}

QVector<double> caretStops(const QString& text, const QString& font, const TypeSetting& setting,
                           const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    const ::rust::Vec<double> stops
        = ::pictura::type_caret_stops(text, font, setting, character, paragraph);
    QVector<double> out;
    out.reserve(int(stops.size()));
    for (const double value : stops) {
        out.append(value);
    }
    return out;
}

QVector<double> baselines(const QString& text, const QString& font, const TypeSetting& setting,
                          const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    const ::rust::Vec<double> lines
        = ::pictura::type_baselines(text, font, setting, character, paragraph);
    QVector<double> out;
    out.reserve(int(lines.size()));
    for (const double value : lines) {
        out.append(value);
    }
    return out;
}

bool overflows(const QString& text, const QString& font, const TypeSetting& setting,
               const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    return ::pictura::type_overflows(text, font, setting, character, paragraph);
}

QVector<double> pathOverflowPoint(const QString& text, const QString& font,
                                  const TypeSetting& setting, const CharacterSetting& character,
                                  const ParagraphSetting& paragraph)
{
    const ::rust::Vec<double> point
        = ::pictura::type_path_overflow_point(text, font, setting, character, paragraph);
    QVector<double> out;
    for (const double value : point) {
        out.append(value);
    }
    return out;
}

QVector<double> localBounds(const QString& text, const QString& font, const TypeSetting& setting,
                            const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    const ::rust::Vec<double> bounds
        = ::pictura::type_local_bounds(text, font, setting, character, paragraph);
    QVector<double> out;
    for (const double value : bounds) {
        out.append(value);
    }
    return out;
}

bool isParagraph(const PictureView& view, const QString& path)
{
    return ::pictura::type_layer_is_paragraph(view, path);
}

bool convert(PictureView& view, const QString& path, bool paragraph)
{
    return ::pictura::type_convert(view, path, paragraph);
}

QByteArray previewRgba(const QString& text, const QString& font, const TypeSetting& setting,
                       const CharacterSetting& character, const ParagraphSetting& paragraph)
{
    const ::rust::Vec<std::uint8_t> rgba
        = ::pictura::type_preview_rgba(text, font, setting, character, paragraph);
    return QByteArray(reinterpret_cast<const char*>(rgba.data()), int(rgba.size()));
}

QString commitLayer(PictureView& view, const QString& text, const QString& font,
                    const TypeSetting& setting, const CharacterSetting& character,
                    const ParagraphSetting& paragraph)
{
    return ::pictura::type_commit_layer(view, text, font, setting, character, paragraph);
}

bool commitMask(PictureView& view, const QString& text, const QString& font,
                const TypeSetting& setting, const CharacterSetting& character,
                const ParagraphSetting& paragraph, const QString& mode)
{
    return ::pictura::type_commit_mask(view, text, font, setting, character, paragraph, mode);
}

bool updateLayer(PictureView& view, const QString& path, const QString& text, const QString& font,
                 const TypeSetting& setting, const CharacterSetting& character,
                 const ParagraphSetting& paragraph)
{
    return ::pictura::type_update_layer(view, path, text, font, setting, character, paragraph);
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

int paragraphStyleCount(const PictureView& view)
{
    return int(::pictura::type_paragraph_style_count(view));
}

QString paragraphStyleName(const PictureView& view, int index)
{
    return ::pictura::type_paragraph_style_name(view, index);
}

CharacterSetting paragraphStyleCharacter(const PictureView& view, const QString& name)
{
    return ::pictura::type_paragraph_style_character(view, name);
}

ParagraphSetting paragraphStyleParagraph(const PictureView& view, const QString& name)
{
    return ::pictura::type_paragraph_style_paragraph(view, name);
}

QString paragraphStyleFont(const PictureView& view, const QString& name)
{
    return ::pictura::type_paragraph_style_font(view, name);
}

bool rasterize(PictureView& view, const QString& path)
{
    return view.rasterize_type(path);
}

bool beginEdit(PictureView& view, const QString& path)
{
    return view.layer_is_type(path) && ::pictura::type_edit_begin(view, path);
}

void cancelEdit(PictureView& view, const QString& path)
{
    ::pictura::type_edit_cancel(view, path);
}

bool commitEdit(PictureView& view, const QString& path, const QString& text, const QString& font,
                const TypeSetting& setting, const CharacterSetting& character,
                const ParagraphSetting& paragraph)
{
    return ::pictura::type_commit_edit(view, path, text, font, setting, character, paragraph);
}

} // namespace pictura::type_api
