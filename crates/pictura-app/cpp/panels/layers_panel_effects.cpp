// The Layers panel's synthetic layer-effects rows (#960): an "Effects" group
// and one row per effect under each styled layer.

#include "layers_panel.h"

#include "api/layers_api.h"
#include "layer_style_dialog.h"
#include "layers_panel_model.h"

#include <QtCore/QSet>
#include <QtCore/QStringList>
#include <QtCore/QVector>

namespace pictura {

QVector<LayerRowNode> LayersPanel::appendEffectRows(const layers_api::Snapshot& snap,
                                                    QStringList& autoExpand, bool newDocument)
{
    QVector<LayerRowNode> synthetic;
    QSet<QString> styledNow;
    for (const layers_api::LayerRow& source : snap.rows) {
        if (!source.hasStyle || source.styleEffects.isEmpty()) {
            continue;
        }
        const bool firstSeen = !styledPathsSeen_.contains(source.path);
        styledNow.insert(source.path);
        if (firstSeen && expandNewEffects_ && !newDocument) {
            listsExpanded_.insert(source.path);
            autoExpand.push_back(source.path);
        }
        if (!listsExpanded_.contains(source.path)) {
            continue;
        }
        LayerRowNode group;
        group.path = source.path + QStringLiteral("/@fx");
        group.depth = source.depth + 1;
        group.name = QStringLiteral("Effects");
        group.kind = QStringLiteral("layer-effects");
        group.visible = source.styleVisible;
        group.expandable = false;
        group.childCount = source.styleEffects.size();
        group.synthetic = true;
        synthetic.push_back(std::move(group));
        for (const QString& key : source.styleEffects) {
            LayerRowNode child;
            child.path = source.path + QStringLiteral("/@fx/") + key;
            child.depth = source.depth + 2;
            child.kind = QStringLiteral("layer-effect");
            child.name = LayerStyleDialog::effectTitle(key);
            child.visible = source.styleEffectsOn.contains(key);
            child.synthetic = true;
            synthetic.push_back(std::move(child));
        }
    }
    styledPathsSeen_ = styledNow;
    return synthetic;
}

void LayersPanel::toggleLists(const QString& path)
{
    if (path.isEmpty()) {
        return;
    }
    const bool open = !listsExpanded_.contains(path);
    if (open) {
        listsExpanded_.insert(path);
    } else {
        listsExpanded_.remove(path);
    }
    refresh();
    if (open) {
        const QModelIndex layer = proxyIndexForPath(path);
        if (layer.isValid()) {
            tree_->expand(layer);
        }
        for (const QString& groupPath :
             {path + QStringLiteral("/@fx"), path + QStringLiteral("/@sf")}) {
            const QModelIndex group = proxyIndexForPath(groupPath);
            if (group.isValid()) {
                tree_->expand(group);
            }
        }
    }
}

} // namespace pictura
