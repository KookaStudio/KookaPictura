// Type menu wiring: point/paragraph conversion, the structural commands, the
// option rows (anti-alias, orientation, OpenType, font preview, language),
// Paste Lorem Ipsum, default type styles, Warp Text, and Replace All Missing
// Fonts. The menu rows reuse the type bridge; the panel toggles stay with the
// generic panel wiring.

#include "menu_registrar.h"

#include "commands.h"

#include "menu_registrar_internal.h"

#include "api/layers_bridge.h"
#include "document_registry.h"
#include "panel_host.h"
#include "path_overlay.h"
#include "tools.h"

#include "api/type_api.h"
#include "type_fonts.h"
#include "warp_text_dialog.h"

#include <QtCore/QDataStream>
#include <QtCore/QIODevice>
#include <QtWidgets/QMessageBox>

#include <cstdint>
#include <memory>

namespace pictura {

namespace {

const QString kLoremIpsum = QStringLiteral(
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor "
    "incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis "
    "nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. "
    "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu "
    "fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in "
    "culpa qui officia deserunt mollit anim id est laborum.");

QString warpRefusalText(type_api::WarpRefusal refusal)
{
    const QString lead
        = QObject::tr("Could not complete the Warp Text command because the type ");
    switch (refusal) {
    case type_api::WarpRefusal::OnPath:
        return lead + QObject::tr("is on a path.");
    case type_api::WarpRefusal::FauxBold:
        return lead + QObject::tr("layer uses a faux bold style.");
    case type_api::WarpRefusal::NoOutlines:
        return lead + QObject::tr("layer uses a font that has no outline data.");
    default:
        return lead + QObject::tr("layer cannot be warped.");
    }
}


// The default type styles are a versioned, base64-encoded stream in the session:
// compact, and all fields the bridge carries round-trip.
QString encodeDefaults(const type_api::TypeSpec& spec)
{
    QByteArray bytes;
    QDataStream out(&bytes, QIODevice::WriteOnly);
    const auto& c = spec.character;
    const auto& p = spec.paragraph;
    out << qint32(1) << spec.font << c.font_style << c.size << qint32(c.leading_mode)
        << c.leading_value << qint32(c.kerning_mode) << qint32(c.kerning_value) << c.tracking
        << c.horizontal_scale << c.vertical_scale << c.baseline_shift << c.tsume
        << qint32(c.anti_alias) << quint32(c.color) << c.all_caps << c.small_caps
        << c.superscript << c.subscript << c.underline << c.strikethrough
        << c.fractional_widths << c.faux_bold << c.faux_italic << c.standard_ligatures
        << c.contextual_alternates << c.discretionary_ligatures << c.swash << c.oldstyle
        << c.stylistic_alternates << c.titling_alternates << c.ornaments << c.ordinals
        << c.fractions << c.language << c.vertical_roman_alignment;
    out << qint32(p.justify) << p.word_spacing_min << p.word_spacing_desired
        << p.word_spacing_max << p.letter_spacing_min << p.letter_spacing_desired
        << p.letter_spacing_max << p.glyph_spacing_min << p.glyph_spacing_desired
        << p.glyph_spacing_max << p.start_indent << p.end_indent << p.first_line_indent
        << p.space_before << p.space_after << p.hanging << p.hyphenate << qint32(p.direction)
        << qint32(p.composer) << p.auto_leading << qint32(p.hyphenate_word_size)
        << qint32(p.hyphenate_pre) << qint32(p.hyphenate_post) << qint32(p.hyphen_limit)
        << p.hyphenation_zone << p.hyphenate_caps;
    return QString::fromLatin1(bytes.toBase64());
}

bool decodeDefaults(const QString& encoded, QString& font, CharacterSetting& character,
                    ParagraphSetting& paragraph)
{
    QByteArray bytes = QByteArray::fromBase64(encoded.toLatin1());
    QDataStream in(&bytes, QIODevice::ReadOnly);
    qint32 version = 0;
    in >> version;
    if (version != 1) {
        return false;
    }
    character = type_api::defaultCharacterSetting();
    paragraph = type_api::defaultParagraphSetting();
    qint32 leadingMode = 0;
    qint32 kerningMode = 0;
    qint32 kerningValue = 0;
    qint32 antiAlias = 0;
    quint32 color = 0;
    qint32 justify = 0;
    qint32 direction = 0;
    qint32 composer = 0;
    qint32 hyphenWordSize = 0;
    qint32 hyphenPre = 0;
    qint32 hyphenPost = 0;
    qint32 hyphenLimit = 0;
    in >> font >> character.font_style >> character.size >> leadingMode
        >> character.leading_value >> kerningMode >> kerningValue >> character.tracking
        >> character.horizontal_scale >> character.vertical_scale >> character.baseline_shift
        >> character.tsume >> antiAlias >> color >> character.all_caps >> character.small_caps
        >> character.superscript >> character.subscript >> character.underline
        >> character.strikethrough >> character.fractional_widths >> character.faux_bold
        >> character.faux_italic >> character.standard_ligatures
        >> character.contextual_alternates >> character.discretionary_ligatures
        >> character.swash >> character.oldstyle >> character.stylistic_alternates
        >> character.titling_alternates >> character.ornaments >> character.ordinals
        >> character.fractions >> character.language >> character.vertical_roman_alignment;
    in >> justify >> paragraph.word_spacing_min >> paragraph.word_spacing_desired
        >> paragraph.word_spacing_max >> paragraph.letter_spacing_min
        >> paragraph.letter_spacing_desired >> paragraph.letter_spacing_max
        >> paragraph.glyph_spacing_min >> paragraph.glyph_spacing_desired
        >> paragraph.glyph_spacing_max >> paragraph.start_indent >> paragraph.end_indent
        >> paragraph.first_line_indent >> paragraph.space_before >> paragraph.space_after
        >> paragraph.hanging >> paragraph.hyphenate >> direction >> composer
        >> paragraph.auto_leading >> hyphenWordSize >> hyphenPre >> hyphenPost
        >> hyphenLimit >> paragraph.hyphenation_zone >> paragraph.hyphenate_caps;
    if (in.status() != QDataStream::Ok) {
        return false;
    }
    character.leading_mode = leadingMode;
    character.kerning_mode = kerningMode;
    character.kerning_value = kerningValue;
    character.anti_alias = antiAlias;
    character.color = color;
    paragraph.justify = justify;
    paragraph.direction = direction;
    paragraph.composer = composer;
    paragraph.hyphenate_word_size = hyphenWordSize;
    paragraph.hyphenate_pre = hyphenPre;
    paragraph.hyphenate_post = hyphenPost;
    paragraph.hyphen_limit = hyphenLimit;
    return true;
}

} // namespace

void MenuRegistrar::wireTypeMenu()
{
    // The current type layer when it is paragraph type (`paragraph`) or point
    // type, else empty.
    const auto convertible = [this](bool paragraph) -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !type_api::readLayer(*view, path).isType
            || type_api::isParagraph(*view, path) != paragraph) {
            return QString();
        }
        return path;
    };
    for (const bool toParagraph : {false, true}) {
        const QString id = QString::fromLatin1(toParagraph
                                               ? command_ids::TypeConvertToParagraphText
                                               : command_ids::TypeConvertToPointText);
        commands_->setImplemented(id, true);
        commands_->setHandler(id, [this, convertible, toParagraph]() {
            PictureView* view = activeView();
            const QString path = convertible(!toParagraph);
            if (view && !path.isEmpty()) {
                type_api::convert(*view, path, toParagraph);
            }
        });
        commands_->setEnabledProvider(id, [convertible, toParagraph]() {
            return !convertible(!toParagraph).isEmpty();
        });
    }
    wireOutlineCommands();
    wireTypeOptions();
    wireTypeDefaults();
    wireWarpText();
    wireReplaceMissingFonts();
}

// Type > Create Work Path and Convert to Shape, over the current type layer.
void MenuRegistrar::wireOutlineCommands()
{
    const auto target = [this]() -> QString {
        const TextSession* session = tools_ ? tools_->textSession() : nullptr;
        if (session && session->textActive()) {
            return QString();
        }
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !type_api::layersEditable(*view)) {
            return QString();
        }
        const type_api::LayerType layer = type_api::readLayer(*view, path);
        return layer.isType ? path : QString();
    };
    const auto canOutline = [this, target]() {
        PictureView* view = activeView();
        const QString path = target();
        return view && !path.isEmpty() && type_api::canOutline(*view, path);
    };

    const QString create = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Create Work Path")});
    commands_->setImplemented(create, true);
    commands_->setEnabledProvider(create, canOutline);
    commands_->setHandler(create, [this, target]() {
        PictureView* view = activeView();
        const QString path = target();
        if (!view || path.isEmpty() || !type_api::createWorkPath(*view, path)) {
            return;
        }
        paths_set_active(*view, -1);
        if (ImageView* canvas = imageView()) {
            showWorkPathOverlay(*canvas, view);
        }
        refresh();
    });

    const QString shape
        = commandIdForPath({QStringLiteral("Type"), QStringLiteral("Convert to Shape")});
    commands_->setImplemented(shape, true);
    commands_->setEnabledProvider(shape, canOutline);
    commands_->setHandler(shape, [this, target]() {
        PictureView* view = activeView();
        const QString path = target();
        if (!view || path.isEmpty() || !type_api::convertToShape(*view, path)) {
            return;
        }
        refresh();
    });
}

void MenuRegistrar::wireTypeOptions()
{
    const auto textActive = [this]() {
        const TextSession* session = tools_ ? tools_->textSession() : nullptr;
        return session && session->textActive();
    };
    const auto target = [this, textActive]() -> QString {
        if (textActive()) {
            return QString();
        }
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty() || !type_api::layersEditable(*view)) {
            return QString();
        }
        const type_api::LayerType layer = type_api::readLayer(*view, path);
        return layer.isType ? path : QString();
    };

    const QStringList antiAlias{QStringLiteral("None"), QStringLiteral("Sharp"),
                                QStringLiteral("Crisp"), QStringLiteral("Strong"),
                                QStringLiteral("Smooth")};
    for (int i = 0; i < antiAlias.size(); ++i) {
        const QString id = commandIdForPath(
            {QStringLiteral("Type"), QStringLiteral("Anti-Alias"), antiAlias.at(i)});
        commands_->setImplemented(id, true);
        commands_->setEnabledProvider(id, [target]() { return !target().isEmpty(); });
        commands_->setCheckedProvider(id, [this, target, i]() {
            PictureView* view = activeView();
            const QString path = target();
            return view && !path.isEmpty()
                && type_api::readLayer(*view, path).spec.character.anti_alias == i;
        });
        commands_->setHandler(id, [this, target, i]() {
            PictureView* view = activeView();
            const QString path = target();
            if (!view || path.isEmpty()) {
                return;
            }
            type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
            spec.character.anti_alias = i;
            if (type_api::updateLayer(*view, path, spec)) {
                refresh();
            }
        });
    }

    for (const bool vertical : {false, true}) {
        const QString id = commandIdForPath(
            {QStringLiteral("Type"), QStringLiteral("Orientation"),
             vertical ? QStringLiteral("Vertical") : QStringLiteral("Horizontal")});
        commands_->setImplemented(id, true);
        commands_->setEnabledProvider(id, [target]() { return !target().isEmpty(); });
        commands_->setCheckedProvider(id, [this, target, vertical]() {
            PictureView* view = activeView();
            const QString path = target();
            return view && !path.isEmpty()
                && type_api::readLayer(*view, path).spec.setting.vertical == vertical;
        });
        commands_->setHandler(id, [this, target, vertical]() {
            PictureView* view = activeView();
            const QString path = target();
            if (!view || path.isEmpty()) {
                return;
            }
            type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
            spec.setting.vertical = vertical;
            if (type_api::updateLayer(*view, path, spec)) {
                refresh();
            }
        });
    }

    struct OpenTypeRow {
        const char* id;
        const char* label;
        bool CharacterSetting::* field;
    };
    static const OpenTypeRow kOpenTypeRows[] = {
        {"type.openType.standardLigatures", "Standard Ligatures",
         &CharacterSetting::standard_ligatures},
        {"type.openType.contextualAlternates", "Contextual Alternates",
         &CharacterSetting::contextual_alternates},
        {"type.openType.discretionaryLigatures", "Discretionary Ligatures",
         &CharacterSetting::discretionary_ligatures},
        {"type.openType.swash", "Swash", &CharacterSetting::swash},
        {"type.openType.oldstyle", "Oldstyle", &CharacterSetting::oldstyle},
        {"type.openType.stylisticAlternates", "Stylistic Alternates",
         &CharacterSetting::stylistic_alternates},
        {"type.openType.titlingAlternates", "Titling Alternates",
         &CharacterSetting::titling_alternates},
        {"type.openType.ornaments", "Ornaments", &CharacterSetting::ornaments},
        {"type.openType.ordinals", "Ordinals", &CharacterSetting::ordinals},
        {"type.openType.fractions", "Fractions", &CharacterSetting::fractions},
    };
    for (const OpenTypeRow& row : kOpenTypeRows) {
        const QString id = QString::fromLatin1(row.id);
        commands_->setImplemented(id, true);
        commands_->setEnabledProvider(id, [target]() { return !target().isEmpty(); });
        commands_->setCheckedProvider(id, [this, target, row]() {
            PictureView* view = activeView();
            const QString path = target();
            return view && !path.isEmpty()
                && type_api::readLayer(*view, path).spec.character.*(row.field);
        });
        commands_->setHandler(id, [this, target, row]() {
            PictureView* view = activeView();
            const QString path = target();
            if (!view || path.isEmpty()) {
                return;
            }
            type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
            spec.character.*(row.field) = !(spec.character.*(row.field));
            if (type_api::updateLayer(*view, path, spec)) {
                refresh();
            }
        });
    }

    const QString rasterize = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Rasterize Type Layer")});
    commands_->setImplemented(rasterize, true);
    commands_->setEnabledProvider(rasterize, [target]() { return !target().isEmpty(); });
    commands_->setHandler(rasterize, [this, target]() {
        PictureView* view = activeView();
        const QString path = target();
        if (view && !path.isEmpty() && type_api::rasterize(*view, path)) {
            refresh();
        }
    });

    const QString updateAll = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Update All Text Layers")});
    commands_->setImplemented(updateAll, true);
    commands_->setEnabledProvider(updateAll, [this, textActive]() {
        PictureView* view = activeView();
        return view && !textActive() && type_api::layersEditable(*view)
            && type_api::hasTypeLayers(*view);
    });
    commands_->setHandler(updateAll, [this]() {
        PictureView* view = activeView();
        if (view && type_api::updateAllTextLayers(*view) > 0) {
            refresh();
        }
    });

    const auto showAsian = [this]() {
        return runtime_.typeShowAsianOptions ? runtime_.typeShowAsianOptions() : false;
    };
    const auto showMiddle = [this]() {
        return runtime_.typeMiddleEastern ? runtime_.typeMiddleEastern() : false;
    };
    const auto setAsian = [this](bool on) {
        if (runtime_.setTypeShowAsianOptions) {
            runtime_.setTypeShowAsianOptions(on);
        }
        refresh();
    };
    const auto setMiddle = [this](bool on) {
        if (runtime_.setTypeMiddleEastern) {
            runtime_.setTypeMiddleEastern(on);
        }
        refresh();
    };
    const QString defaultLanguage = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Language Options"),
         QStringLiteral("Default features")});
    commands_->setImplemented(defaultLanguage, true);
    commands_->setEnabledProvider(defaultLanguage, []() { return true; });
    commands_->setCheckedProvider(
        defaultLanguage, [showAsian, showMiddle]() { return !showAsian() && !showMiddle(); });
    commands_->setHandler(defaultLanguage, [setAsian, setMiddle]() {
        setAsian(false);
        setMiddle(false);
    });
    const QString middleLanguage = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Language Options"),
         QStringLiteral("Middle Eastern features")});
    commands_->setImplemented(middleLanguage, true);
    commands_->setEnabledProvider(middleLanguage, []() { return true; });
    commands_->setCheckedProvider(middleLanguage, [showMiddle]() { return showMiddle(); });
    commands_->setHandler(middleLanguage, [showMiddle, setMiddle]() { setMiddle(!showMiddle()); });
    const QString eastAsianLanguage = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Language Options"),
         QStringLiteral("East Asian features")});
    commands_->setImplemented(eastAsianLanguage, true);
    commands_->setEnabledProvider(eastAsianLanguage, []() { return true; });
    commands_->setCheckedProvider(eastAsianLanguage, [showAsian]() { return showAsian(); });
    commands_->setHandler(eastAsianLanguage, [showAsian, setAsian]() { setAsian(!showAsian()); });

    const QStringList previewSizes{QStringLiteral("None"), QStringLiteral("Small"),
                                   QStringLiteral("Medium"), QStringLiteral("Large"),
                                   QStringLiteral("Extra Large"), QStringLiteral("Huge")};
    for (int i = 0; i < previewSizes.size(); ++i) {
        const QString id = commandIdForPath(
            {QStringLiteral("Type"), QStringLiteral("Font Preview Size"), previewSizes.at(i)});
        commands_->setImplemented(id, true);
        commands_->setEnabledProvider(id, []() { return true; });
        commands_->setCheckedProvider(id, [this, i]() {
            const int size = runtime_.fontPreviewSize ? runtime_.fontPreviewSize() : 2;
            return size == i;
        });
        commands_->setHandler(id, [this, i]() {
            if (runtime_.setFontPreviewSize) {
                runtime_.setFontPreviewSize(i);
            }
            refresh();
        });
    }

    const QString lorem = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Paste Lorem Ipsum")});
    commands_->setImplemented(lorem, true);
    commands_->setEnabledProvider(lorem, [target, textActive]() {
        return textActive() || !target().isEmpty();
    });
    commands_->setHandler(lorem, [this, target, textActive]() {
        if (textActive()) {
            if (TextSession* session = tools_ ? tools_->textSession() : nullptr) {
                session->insertText(kLoremIpsum);
                refresh();
            }
            return;
        }
        PictureView* view = activeView();
        const QString path = target();
        if (view && !path.isEmpty() && type_api::pasteLorem(*view, path)) {
            refresh();
        }
    });
}

void MenuRegistrar::wireTypeDefaults()
{
    const auto currentType = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        if (!view || path.isEmpty()) {
            return QString();
        }
        return type_api::readLayer(*view, path).isType ? path : QString();
    };
    const auto textActive = [this]() {
        const TextSession* session = tools_ ? tools_->textSession() : nullptr;
        return session && session->textActive();
    };
    const auto loadTarget = [this, currentType, textActive]() -> QString {
        if (textActive()) {
            return QString();
        }
        PictureView* view = activeView();
        const QString path = currentType();
        if (!view || path.isEmpty() || !type_api::layersEditable(*view)) {
            return QString();
        }
        return path;
    };
    const auto defaultsJson = [this]() {
        return runtime_.typeDefaultsJson ? runtime_.typeDefaultsJson() : QString();
    };

    const QString save = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Save Default Type Styles")});
    commands_->setImplemented(save, true);
    commands_->setEnabledProvider(save, [currentType]() { return !currentType().isEmpty(); });
    commands_->setHandler(save, [this, currentType]() {
        PictureView* view = activeView();
        const QString path = currentType();
        if (!view || path.isEmpty()) {
            return;
        }
        const type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
        if (runtime_.setTypeDefaultsJson) {
            runtime_.setTypeDefaultsJson(encodeDefaults(spec));
        }
    });

    const QString load = commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Load Default Type Styles")});
    commands_->setImplemented(load, true);
    commands_->setEnabledProvider(load, [loadTarget, defaultsJson]() {
        return !loadTarget().isEmpty() && !defaultsJson().isEmpty();
    });
    commands_->setHandler(load, [this, loadTarget, defaultsJson]() {
        PictureView* view = activeView();
        const QString path = loadTarget();
        if (!view || path.isEmpty()) {
            return;
        }
        QString font;
        CharacterSetting character{};
        ParagraphSetting paragraph{};
        if (!decodeDefaults(defaultsJson(), font, character, paragraph)) {
            return;
        }
        if (type_api::loadDefaults(*view, path, font, character, paragraph)) {
            refresh();
        }
    });
}

// The dialog is window-modal but not blocking: each change previews on the
// canvas, OK commits one "Warp Text" state, and Cancel puts the layer back.
void MenuRegistrar::wireWarpText()
{
    const auto target = [this]() -> QString {
        PictureView* view = activeView();
        const QString path = layerAnchor();
        return view && !path.isEmpty() && type_api::readLayer(*view, path).isType ? path : QString();
    };
    const QString id = QString::fromLatin1(command_ids::TypeWarpText);
    commands_->setImplemented(id, true);
    commands_->setEnabledProvider(id, [target]() { return !target().isEmpty(); });
    commands_->setHandler(id, [this, target]() {
        PictureView* view = activeView();
        const QString path = target();
        if (!view || path.isEmpty()) {
            return;
        }
        const type_api::WarpRefusal refusal = type_api::warpRefusal(*view, path);
        if (refusal != type_api::WarpRefusal::None) {
            auto* box = new QMessageBox(QMessageBox::Warning, tr("Warp Text"),
                                        warpRefusalText(refusal), QMessageBox::Ok, dialogParent());
            box->setObjectName(QStringLiteral("warpTextRefusal"));
            box->setAttribute(Qt::WA_DeleteOnClose);
            box->open();
            return;
        }
        auto* dialog = new WarpTextDialog(type_api::layerWarp(*view, path), dialogParent());
        dialog->setAttribute(Qt::WA_DeleteOnClose);
        // The dialog's connections own the session: closing the dialog
        // destroys them, and an unfinished session cancels.
        const auto session = std::make_shared<type_api::WarpSession>(*view, path);
        connect(dialog, &WarpTextDialog::changed, dialog, [this, dialog, session]() {
            if (session->preview(dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::accepted, dialog, [this, dialog, session]() {
            if (session->commit(dialog->setting())) {
                refresh();
            }
        });
        connect(dialog, &QDialog::rejected, dialog, [this, session]() {
            if (session->cancel()) {
                refresh();
            }
        });
        dialog->open();
    });
}

// CS6 substitutes its default font; ours is the Type tool's default, which the
// engine bundles. One "Replace All Missing Fonts" state.
void MenuRegistrar::wireReplaceMissingFonts()
{
    const auto missing = [this]() -> QStringList {
        PictureView* view = activeView();
        return view ? missingFonts(type_api::documentFonts(*view)) : QStringList();
    };
    const QString id = QString::fromLatin1(command_ids::TypeReplaceAllMissingFonts);
    commands_->setImplemented(id, true);
    // Type is only re-set in 8-bit RGB, so elsewhere the command stays off and
    // the open-time alert says why.
    commands_->setEnabledProvider(id, [this, missing]() {
        PictureView* view = activeView();
        return view && type_api::layersEditable(*view) && !missing().isEmpty();
    });
    commands_->setHandler(id, [this, missing]() {
        PictureView* view = activeView();
        const QStringList names = missing();
        if (view && !names.isEmpty() && type_api::replaceFonts(*view, names) > 0) {
            refresh();
        }
    });
}

} // namespace pictura
