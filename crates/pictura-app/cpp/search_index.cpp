#include "search_index.h"

#include "commands.h"
#include "frame.h"
#include "preferences_dialog.h"
#include "tools.h"

#include <QtCore/QHash>
#include <QtCore/QRegularExpression>
#include <QtGui/QAction>

#include <algorithm>

namespace pictura {

namespace {

QStringList cleanPath(const QStringList& path)
{
    QStringList cleaned;
    for (const QString& segment : path) {
        cleaned << cleanSearchLabel(segment);
    }
    return cleaned;
}

bool occursIn(const SearchEntry& entry, const QString& word)
{
    const auto contains = [&word](const QString& text) {
        return text.contains(word, Qt::CaseInsensitive);
    };
    return std::any_of(entry.path.cbegin(), entry.path.cend(), contains)
        || std::any_of(entry.keywords.cbegin(), entry.keywords.cend(), contains);
}

// -1 when a word is missing; otherwise the ranking tier (0 is best).
int matchTier(const SearchEntry& entry, const QStringList& words, const QString& phrase)
{
    for (const QString& word : words) {
        if (!occursIn(entry, word)) {
            return -1;
        }
    }
    const QString label = entry.label().toLower();
    if (label == phrase) {
        return 0;
    }
    if (label.startsWith(phrase)) {
        return 1;
    }
    static const QRegularExpression separators(QStringLiteral("[^\\w]+"));
    const QStringList labelWords = label.split(separators, Qt::SkipEmptyParts);
    const bool wordPrefixes = std::all_of(words.cbegin(), words.cend(), [&](const QString& w) {
        return std::any_of(labelWords.cbegin(), labelWords.cend(),
                           [&w](const QString& lw) { return lw.startsWith(w); });
    });
    if (wordPrefixes) {
        return 2;
    }
    const bool substrings = std::all_of(words.cbegin(), words.cend(),
                                        [&label](const QString& w) { return label.contains(w); });
    return substrings ? 3 : 4;
}

} // namespace

QString cleanSearchLabel(const QString& label)
{
    QString cleaned;
    cleaned.reserve(label.size());
    for (int i = 0; i < label.size(); ++i) {
        const QChar c = label.at(i);
        if (c == QLatin1Char('&') && i + 1 < label.size() && !label.at(i + 1).isSpace()) {
            // "&&" is a literal ampersand; "&x" marks a mnemonic.
            if (label.at(i + 1) == QLatin1Char('&')) {
                cleaned += c;
                ++i;
            }
            continue;
        }
        // A tab starts an action text's inline shortcut column.
        if (c == QLatin1Char('\t')) {
            break;
        }
        cleaned += c;
    }
    // A label provider may append an ellipsis to one already there.
    for (cleaned = cleaned.trimmed();; cleaned = cleaned.trimmed()) {
        if (cleaned.endsWith(QChar(0x2026))) {
            cleaned.chop(1);
        } else if (cleaned.endsWith(QStringLiteral("..."))) {
            cleaned.chop(3);
        } else {
            return cleaned;
        }
    }
}

QList<SearchEntry> buildSearchIndex(const CommandRegistry& registry)
{
    QHash<QString, QString> panelForCommand;
    for (const PanelToggle& toggle : panelToggles()) {
        panelForCommand.insert(QString::fromLatin1(toggle.command),
                               QString::fromLatin1(toggle.panel));
    }
    const QStringList pages = {PreferencesDialog::kGeneral,     PreferencesDialog::kInterface,
                               PreferencesDialog::kFileHandling, PreferencesDialog::kPerformance,
                               PreferencesDialog::kUnits,       PreferencesDialog::kGuides};
    const QString search = QString::fromLatin1(command_ids::HelpSearch);

    QList<SearchEntry> entries;
    QHash<QString, int> panelEntry;
    for (const CommandInfo& info : registry.describe()) {
        if (info.id == search || info.path.isEmpty()) {
            continue;
        }
        SearchEntry entry;
        entry.id = info.id;
        entry.command = info.id;
        entry.path = cleanPath(info.path);
        if (const QAction* action = registry.action(info.id); action && !action->text().isEmpty()) {
            entry.path.last() = cleanSearchLabel(action->text());
        }
        entry.implemented = info.implemented;
        entry.runnable = info.implemented && info.enabled;
        const QList<QKeySequence> keys = registry.shortcuts(info.id);
        if (!keys.isEmpty()) {
            entry.shortcut = keys.first().toString(QKeySequence::NativeText);
        }

        const QString panel = panelForCommand.value(info.id);
        if (!panel.isEmpty()) {
            entry.kind = SearchEntry::Kind::Panel;
            entry.id = panel;
            entry.runnable = true;
            entry.keywords << QStringLiteral("panel");
            // One entry per panel; the Window menu's path wins over the Type
            // menu's shortcut to the same panel.
            if (const auto it = panelEntry.constFind(panel); it != panelEntry.cend()) {
                if (info.path.first() == QLatin1String("Window")) {
                    entries[it.value()] = entry;
                }
                continue;
            }
            panelEntry.insert(panel, int(entries.size()));
        } else if (info.path.size() == 3 && info.path.at(0) == QLatin1String("Edit")
                   && info.path.at(1) == QLatin1String("Preferences")
                   && pages.contains(info.path.at(2))) {
            entry.kind = SearchEntry::Kind::Preference;
            entry.id = info.path.at(2);
            entry.implemented = true;
            entry.runnable = true;
            entry.keywords << QStringLiteral("settings");
        }
        entries << entry;
    }

    for (ToolId id : allToolIds()) {
        const ToolInfo& info = toolInfo(id);
        SearchEntry entry;
        entry.kind = SearchEntry::Kind::Tool;
        entry.id = toolIdName(id);
        entry.path = {QStringLiteral("Tools"), QString::fromUtf8(info.label)};
        entry.keywords << QStringLiteral("tool");
        entry.implemented = info.implemented;
        entry.runnable = info.implemented;
        if (const QChar key = toolShortcut(id); !key.isNull()) {
            entry.shortcut = QString(key);
        }
        entries << entry;
    }

    for (const SearchControl& control : searchControls()) {
        SearchEntry entry;
        entry.kind = SearchEntry::Kind::Control;
        entry.id = QString::fromLatin1(control.control);
        entry.container = QString::fromLatin1(control.container);
        entry.tool = control.tool ? QString::fromLatin1(control.tool) : QString();
        entry.path = QString::fromUtf8(control.path).split(QStringLiteral(" > "));
        entry.keywords = QString::fromUtf8(control.keywords).split(QLatin1Char(' '),
                                                                   Qt::SkipEmptyParts);
        entry.runnable = true;
        entries << entry;
    }
    return entries;
}

QList<SearchHit> searchIndex(const QList<SearchEntry>& entries, const QString& query, int limit)
{
    const QStringList words = query.toLower().split(QLatin1Char(' '), Qt::SkipEmptyParts);
    if (words.isEmpty()) {
        return {};
    }
    const QString phrase = words.join(QLatin1Char(' '));
    QList<SearchHit> hits;
    for (int i = 0; i < entries.size(); ++i) {
        const int tier = matchTier(entries.at(i), words, phrase);
        if (tier >= 0) {
            hits << SearchHit{i, tier};
        }
    }
    std::stable_sort(hits.begin(), hits.end(), [&entries](const SearchHit& a, const SearchHit& b) {
        const SearchEntry& ea = entries.at(a.index);
        const SearchEntry& eb = entries.at(b.index);
        if (a.tier != b.tier) {
            return a.tier < b.tier;
        }
        if (ea.runnable != eb.runnable) {
            return ea.runnable;
        }
        if (ea.implemented != eb.implemented) {
            return ea.implemented;
        }
        return ea.path.size() < eb.path.size();
    });
    if (limit >= 0 && hits.size() > limit) {
        hits.resize(limit);
    }
    return hits;
}

} // namespace pictura
