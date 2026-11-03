#include "adjustment_presets.h"

#include "pictura_app/src/cxxqt_object/adjustment_presets.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust.cxxqt.h"

#include <QtCore/QHash>
#include <QtCore/QSignalBlocker>

#include <utility>

namespace pictura {

namespace {

QByteArray bytes(const ::rust::Vec<std::uint8_t>& v)
{
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

::rust::Slice<const std::uint8_t> slice(const QByteArray& block)
{
    return {reinterpret_cast<const std::uint8_t*>(block.constData()),
            std::size_t(block.size())};
}

QByteArray setParam(const QByteArray& block, const char* key, double value)
{
    return bytes(image_adjustment_set(slice(block), QString::fromLatin1(key), value));
}

QByteArray setCurve(const QByteArray& block, int channel, const QList<QPair<int, int>>& points)
{
    QStringList text;
    for (const auto& [x, y] : points) {
        text << QStringLiteral("%1,%2").arg(x).arg(y);
    }
    return bytes(image_adjustment_set_curve(slice(block), channel,
                                            text.join(QLatin1Char(' '))));
}

double paramValue(const QByteArray& block, const char* key)
{
    const QStringList page = image_adjustment_page(slice(block));
    for (const QString& row : page) {
        const QStringList cells = row.split(QLatin1Char('\t'));
        if (cells.size() > 4 && cells[1] == QLatin1String(key)) {
            return cells[4].toDouble();
        }
    }
    return -1.0;
}

// Levels' built-in table (ported from the dialog's photorust table; the shared
// `Default` entry is the identity and is not repeated here).
struct LevelPreset {
    const char* name;
    int inBlack;
    int inWhite;
    double gamma;
    int outBlack;
    int outWhite;
};

constexpr LevelPreset kLevels[] = {
    {"Darker", 0, 255, 0.75, 0, 255},
    {"Increase Contrast 1", 5, 250, 1.00, 0, 255},
    {"Increase Contrast 2", 10, 245, 1.00, 0, 255},
    {"Increase Contrast 3", 15, 240, 1.00, 0, 255},
    {"Lighten Shadows", 0, 255, 1.50, 0, 255},
    {"Lighter", 0, 255, 1.50, 0, 255},
    {"Midtones Brighter", 0, 255, 1.25, 0, 255},
    {"Midtones Darker", 0, 255, 0.75, 0, 255},
};

QByteArray levelsBlock(const LevelPreset& preset)
{
    QByteArray block = defaultPresetBlock(QStringLiteral("levels"));
    block = setParam(block, "inputBlack", preset.inBlack);
    block = setParam(block, "inputWhite", preset.inWhite);
    block = setParam(block, "gamma", preset.gamma);
    block = setParam(block, "outputBlack", preset.outBlack);
    block = setParam(block, "outputWhite", preset.outWhite);
    return block;
}

QList<BuiltinPreset> levelPresets()
{
    QList<BuiltinPreset> presets;
    for (const LevelPreset& preset : kLevels) {
        const QByteArray canonical = levelsBlock(preset);
        presets.append(
            {QString::fromLatin1(preset.name),
             [preset](const QByteArray&) { return levelsBlock(preset); },
             [canonical](const QByteArray& block) {
                 return adjustmentPresetMatches(block, canonical);
             }});
    }
    return presets;
}

// The community Curves list from `docs/04-image-ops/adjustments/curves.md`.
// The control points are an approximation of the named look, not a CS6
// capture; `-1` is the composite channel and 1-3 are red, green, blue.
struct CurvePreset {
    const char* name;
    int channel;
    QList<QPair<int, int>> points;
};

const QList<CurvePreset>& curveTable()
{
    static const QList<CurvePreset> table = {
        {"Color Negative", -1, {{0, 255}, {255, 0}}},
        {"Cross Process", 0, {{0, 0}, {64, 80}, {192, 180}, {255, 255}}},
        {"Darker", 0, {{0, 0}, {64, 40}, {192, 150}, {255, 200}}},
        {"Increase Contrast", 0, {{0, 0}, {64, 32}, {192, 224}, {255, 255}}},
        {"Lighter", 0, {{0, 55}, {128, 155}, {255, 255}}},
        {"Linear Contrast", 0, {{0, 0}, {64, 0}, {192, 255}, {255, 255}}},
        {"Medium Contrast", 0, {{0, 0}, {64, 48}, {192, 208}, {255, 255}}},
        {"Negative", 0, {{0, 255}, {255, 0}}},
        {"Strong Contrast", 0, {{0, 0}, {64, 16}, {192, 240}, {255, 255}}},
    };
    return table;
}

QByteArray curveBlock(const CurvePreset& preset)
{
    QByteArray block = defaultPresetBlock(QStringLiteral("curves"));
    if (preset.channel < 0) {
        for (int channel = 0; channel < 4; ++channel) {
            block = setCurve(block, channel, preset.points);
        }
    } else {
        block = setCurve(block, preset.channel, preset.points);
    }
    return block;
}

QList<BuiltinPreset> curvePresets()
{
    QList<BuiltinPreset> presets;
    for (const CurvePreset& preset : curveTable()) {
        const QByteArray canonical = curveBlock(preset);
        presets.append(
            {QString::fromLatin1(preset.name),
             [preset](const QByteArray&) { return curveBlock(preset); },
             [canonical](const QByteArray& block) {
                 return adjustmentPresetMatches(block, canonical);
             }});
    }
    return presets;
}

// Photo Filter's Wratten/colour filters (the dialog's table). A preset changes
// only the filter colour, so Density and Preserve Luminosity stay put.
struct FilterPreset {
    const char* name;
    unsigned rgb;
};

constexpr FilterPreset kFilters[] = {
    {"Warming Filter (85)", 0xec8a00},  {"Warming Filter (LBA)", 0xfa962d},
    {"Warming Filter (81)", 0xebb113},  {"Cooling Filter (80)", 0x006dff},
    {"Cooling Filter (LBB)", 0x005dba}, {"Cooling Filter (82)", 0x0088ea},
    {"Red", 0xea1a00},                  {"Orange", 0xeb7500},
    {"Yellow", 0xffe600},               {"Green", 0x009400},
    {"Cyan", 0x00b7ef},                 {"Blue", 0x0033d1},
    {"Violet", 0x4b0082},               {"Magenta", 0xff0090},
    {"Sepia", 0xac7a33},                {"Deep Red", 0x820500},
    {"Deep Blue", 0x000082},            {"Deep Emerald", 0x006412},
    {"Deep Yellow", 0xffcc00},          {"Underwater", 0x00c2be},
};

QList<BuiltinPreset> photoFilterPresets()
{
    QList<BuiltinPreset> presets;
    for (const FilterPreset& preset : kFilters) {
        presets.append(
            {QString::fromLatin1(preset.name),
             [rgb = preset.rgb](const QByteArray& current) {
                 return setParam(current, "color", double(rgb));
             },
             [rgb = preset.rgb](const QByteArray& block) {
                 return qRound(paramValue(block, "color")) == int(rgb);
             }});
    }
    return presets;
}

} // namespace

const QList<BuiltinPreset>& builtinPresets(const QString& kind)
{
    static const QHash<QString, QList<BuiltinPreset>> table = {
        {QStringLiteral("levels"), levelPresets()},
        {QStringLiteral("curves"), curvePresets()},
        {QStringLiteral("photo-filter"), photoFilterPresets()},
    };
    static const QList<BuiltinPreset> empty;
    const auto it = table.find(kind);
    return it == table.end() ? empty : it.value();
}

QStringList savedPresetNames(const QString& kind) { return adjustment_preset_saved_names(kind); }

QByteArray savedPresetBlock(const QString& kind, const QString& name)
{
    return bytes(adjustment_preset_saved_block(kind, name));
}

bool saveAdjustmentPreset(const QString& kind, const QString& name, const QByteArray& block)
{
    return adjustment_preset_save(kind, name, slice(block));
}

bool deleteAdjustmentPreset(const QString& kind, const QString& name)
{
    return adjustment_preset_delete(kind, name);
}

bool adjustmentPresetCanSave(const QString& kind) { return adjustment_preset_can_save(kind); }

bool adjustmentPresetSupportsMenu(const QString& kind)
{
    return adjustment_preset_supports(kind);
}

QString adjustmentPresetKind(const QByteArray& block) { return adjustment_preset_kind(slice(block)); }

bool adjustmentPresetMatches(const QByteArray& a, const QByteArray& b)
{
    return adjustment_preset_matches(slice(a), slice(b));
}

QByteArray defaultPresetBlock(const QString& kind)
{
    return bytes(image_adjustment_default(kind, 0x000000u, 0xffffffu));
}

PresetCombo::PresetCombo(QWidget* parent)
    : QComboBox(parent)
{
    connect(this, &QComboBox::activated, this, &PresetCombo::activate);
}

void PresetCombo::setBlock(const QString& kind, const QByteArray& block)
{
    if (kind != kind_ || entries_.isEmpty()) {
        kind_ = kind;
        rebuild();
    }
    block_ = block;
    updateCurrent();
}

void PresetCombo::reload()
{
    if (kind_.isEmpty()) {
        return;
    }
    rebuild();
    updateCurrent();
}

void PresetCombo::rebuild()
{
    entries_.clear();
    const QSignalBlocker blocker(this);
    clear();
    if (kind_.isEmpty()) {
        return;
    }
    const auto add = [this](Entry entry) {
        entries_.append(std::move(entry));
        addItem(entries_.last().name);
    };
    const auto separator = [this]() {
        insertSeparator(count());
        entries_.append(Entry{});
    };

    add({QStringLiteral("Default"), defaultPresetBlock(kind_), {}, {}, true});
    for (const BuiltinPreset& preset : builtinPresets(kind_)) {
        add({preset.name, {}, preset.build, preset.matches, true});
    }
    const QStringList saved = savedPresetNames(kind_);
    if (!saved.isEmpty()) {
        separator();
        for (const QString& name : saved) {
            add({name, savedPresetBlock(kind_, name), {}, {}, true});
        }
    }
    separator();
    add({QStringLiteral("Custom"), {}, {}, {}, false});
}

void PresetCombo::updateCurrent()
{
    const QSignalBlocker blocker(this);
    int match = entries_.isEmpty() ? -1 : entries_.size() - 1;
    for (int i = 0; i < entries_.size(); ++i) {
        const Entry& entry = entries_.at(i);
        if (!entry.selectable) {
            continue;
        }
        const bool hit = entry.matches
                             ? entry.matches(block_)
                             : (!entry.block.isEmpty()
                                && adjustmentPresetMatches(block_, entry.block));
        if (hit) {
            match = i;
            break;
        }
    }
    setCurrentIndex(match);
}

void PresetCombo::activate(int index)
{
    if (index < 0 || index >= entries_.size()) {
        return;
    }
    const Entry& entry = entries_.at(index);
    if (!entry.selectable) {
        updateCurrent();
        return;
    }
    const QByteArray next = entry.build ? entry.build(block_) : entry.block;
    if (next.isEmpty()) {
        updateCurrent();
        return;
    }
    block_ = next;
    emit presetChosen(next);
}

} // namespace pictura

#include "moc_adjustment_presets.cpp"
