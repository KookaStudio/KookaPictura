#include "filter_commands.h"

#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include <QtCore/QStringList>

namespace pictura {

namespace {

FilterParamSpec paramFromRow(const FilterParamRow& row)
{
    FilterParamSpec param;
    param.control = static_cast<FilterControl>(row.control);
    param.label = row.label;
    param.minimum = row.minimum;
    param.maximum = row.maximum;
    param.value = row.value;
    param.slotCount = row.slot_count;
    param.decimals = row.decimals;
    param.suffix = row.suffix;
    param.choices = row.choices;
    for (int i = 0; i < param.choices.size(); ++i) {
        param.choiceValues.append(static_cast<double>(i));
    }
    param.initial = row.initial;
    param.measuresPixels = row.measures_pixels;
    return param;
}

FilterCommandSpec specFromRow(const FilterDescriptorRow& row)
{
    FilterCommandSpec spec;
    spec.path = row.path;
    spec.label = row.label;
    spec.kind = row.kind;
    spec.dialog = static_cast<FilterDialogKind>(row.dialog);
    spec.mapAfterOk = row.map_after_ok;
    spec.previewPane = row.preview_pane;
    spec.stacked = row.stacked;
    spec.previewBelow = row.preview_below;
    spec.livePreview = row.live_preview;
    spec.galleryRank = row.gallery_rank;
    for (const FilterParamRow& source : row.params) {
        spec.params.append(paramFromRow(source));
    }
    return spec;
}

struct CommandTables {
    QList<FilterCommandSpec> all;
    QList<FilterCommandSpec> menu;
};

const CommandTables& commandTables()
{
    static const CommandTables tables = [] {
        CommandTables result;
        for (const FilterDescriptorRow& row : filter_descriptors()) {
            const FilterCommandSpec spec = specFromRow(row);
            result.all.append(spec);
            if (row.menu) {
                result.menu.append(spec);
            }
        }
        return result;
    }();
    return tables;
}

} // namespace

const QList<FilterCommandSpec>& filterCommands()
{
    return commandTables().menu;
}

const FilterCommandSpec* filterCommandForPath(const QStringList& path)
{
    for (const FilterCommandSpec& spec : filterCommands()) {
        if (spec.path == path) {
            return &spec;
        }
    }
    return nullptr;
}

const FilterCommandSpec* filterCommandForKind(const QString& kind)
{
    for (const FilterCommandSpec& spec : filterCommands()) {
        if (spec.kind == kind) {
            return &spec;
        }
    }
    for (const FilterCommandSpec& spec : commandTables().all) {
        if (spec.kind == kind) {
            return &spec;
        }
    }
    return nullptr;
}

} // namespace pictura
