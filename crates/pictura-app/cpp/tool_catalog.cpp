#include "tools.h"

#include "pictura_app/src/cxxqt_object/tool_catalogue.cxxqt.h"

#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtCore/QVector>

#include <deque>

namespace pictura {

namespace {

Qt::CursorShape cursorShape(const QString& name)
{
    if (name == QLatin1String("arrow")) {
        return Qt::ArrowCursor;
    }
    if (name == QLatin1String("ibeam")) {
        return Qt::IBeamCursor;
    }
    if (name == QLatin1String("openhand")) {
        return Qt::OpenHandCursor;
    }
    if (name == QLatin1String("sizeall")) {
        return Qt::SizeAllCursor;
    }
    return Qt::CrossCursor;
}

// The engine tool catalogue (`pictura_script::tool_catalogue`) as the shell's
// `ToolInfo` table, read once. `ToolInfo` keeps `const char*` fields, so the
// UTF-8 strings live in `strings` (a deque never moves its elements).
struct Catalogue {
    QVector<ToolInfo> infos;
    QVector<QList<ToolHint>> hints;
    QList<ToolId> all;
    std::deque<QByteArray> strings;

    const char* keep(const QString& text)
    {
        strings.push_back(text.toUtf8());
        return strings.back().constData();
    }
};

const Catalogue& catalogue()
{
    static const Catalogue table = [] {
        Catalogue out;
        int index = 0;
        for (const ToolSpecRow& row : tool_catalogue_rows()) {
            const ToolId id = ToolId(index++);
            ToolInfo info{id,
                          out.keep(row.name),
                          out.keep(row.label),
                          row.shortcut.isEmpty() ? QChar() : row.shortcut.at(0),
                          cursorShape(row.cursor),
                          out.keep(row.hint),
                          row.group,
                          row.implemented,
                          row.hotspot_x,
                          row.hotspot_y};
            QList<ToolHint> hints;
            for (const ToolHintRow& hint : row.hints) {
                hints.append({hint.key, hint.text,
                              hint.command_id.isEmpty() ? nullptr : out.keep(hint.command_id)});
            }
            out.infos.append(info);
            out.hints.append(hints);
            out.all.append(id);
        }
        return out;
    }();
    return table;
}

int toolIndex(ToolId id) { return static_cast<int>(id); }

QHash<int, QChar>& toolShortcutOverrideTable()
{
    static QHash<int, QChar> overrides;
    return overrides;
}

} // namespace

const ToolInfo& toolInfo(ToolId id)
{
    const QVector<ToolInfo>& infos = catalogue().infos;
    const int index = toolIndex(id);
    return (index >= 0 && index < infos.size()) ? infos.at(index) : infos.at(0);
}

bool toolImplemented(ToolId id) { return toolInfo(id).implemented; }

bool isBrushTool(ToolId id)
{
    return id == ToolId::Brush || id == ToolId::Pencil || id == ToolId::ColorReplacement
        || id == ToolId::MixerBrush || id == ToolId::CloneStamp || id == ToolId::PatternStamp
        || id == ToolId::HistoryBrush || id == ToolId::ArtHistoryBrush || id == ToolId::Eraser
        || id == ToolId::BackgroundEraser || id == ToolId::Blur || id == ToolId::Sharpen
        || id == ToolId::Smudge || id == ToolId::Dodge || id == ToolId::Burn
        || id == ToolId::Sponge;
}

QString toolIdName(ToolId id)
{
    return QString::fromLatin1(toolInfo(id).name);
}

QString toolCursorId(ToolId id, Qt::KeyboardModifiers mods)
{
    const QString base = QStringLiteral("tool.") + toolIdName(id);
    if (id != ToolId::Marquee && id != ToolId::EllipticalMarquee) {
        return base;
    }
    if (mods.testFlag(Qt::ShiftModifier)) {
        return base + QStringLiteral(".add");
    }
    if (mods.testFlag(Qt::AltModifier)) {
        return base + QStringLiteral(".remove");
    }
    return base;
}

QChar toolShortcut(ToolId id)
{
    return toolShortcutOverrideTable().value(toolIndex(id), toolInfo(id).shortcut);
}

void setToolShortcut(ToolId id, QChar key)
{
    if (key == toolInfo(id).shortcut) {
        toolShortcutOverrideTable().remove(toolIndex(id));
    } else {
        toolShortcutOverrideTable().insert(toolIndex(id), key);
    }
}

void resetToolShortcuts() { toolShortcutOverrideTable().clear(); }

QMap<ToolId, QChar> toolShortcutOverrides()
{
    QMap<ToolId, QChar> overrides;
    for (auto it = toolShortcutOverrideTable().cbegin(); it != toolShortcutOverrideTable().cend();
         ++it) {
        overrides.insert(ToolId(it.key()), it.value());
    }
    return overrides;
}

QList<QChar> toolShortcutKeys()
{
    QList<QChar> keys;
    for (ToolId id : allToolIds()) {
        const QChar key = toolShortcut(id);
        if (!key.isNull() && !keys.contains(key)) {
            keys << key;
        }
    }
    return keys;
}

int toolGroupForKey(QChar key)
{
    if (key.isNull()) {
        return 0;
    }
    const QChar upper = key.toUpper();
    for (ToolId id : allToolIds()) {
        const QChar shortcut = toolShortcut(id);
        if (!shortcut.isNull() && shortcut.toUpper() == upper) {
            return toolInfo(id).group;
        }
    }
    return 0;
}

QList<ToolHint> toolHintEntries(ToolId id)
{
    const QVector<QList<ToolHint>>& hints = catalogue().hints;
    const int index = toolIndex(id);
    return (index >= 0 && index < hints.size()) ? hints.at(index) : QList<ToolHint>();
}

const QList<ToolId>& allToolIds()
{
    return catalogue().all;
}

QString selectionModeString(SelectionMode mode)
{
    switch (mode) {
    case SelectionMode::New:
        return QStringLiteral("new");
    case SelectionMode::Add:
        return QStringLiteral("add");
    case SelectionMode::Subtract:
        return QStringLiteral("subtract");
    case SelectionMode::Intersect:
        return QStringLiteral("intersect");
    }
    return QStringLiteral("new");
}

} // namespace pictura
