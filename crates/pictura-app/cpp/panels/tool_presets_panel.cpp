#include "tool_presets_panel.h"

#include "api/layers_bridge.h"
#include "icons.h"
#include "preset_manager.h"
#include "tool_options_json.h"
#include "tool_presets.h"

#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QScopedValueRollback>
#include <QtCore/QSet>
#include <QtCore/QSignalBlocker>
#include <QtCore/QVariant>
#include <QtGui/QAction>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cstdint>

namespace pictura {

namespace {

constexpr int kSmallIcon = 16;
constexpr int kLargeIcon = 32;

QString toolLabel(ToolId id)
{
    return QString::fromLatin1(toolInfo(id).label);
}

QByteArray asBytes(const ::rust::Vec<std::uint8_t>& bytes)
{
    return QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
}

} // namespace

ToolPresetsPanel::ToolPresetsPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("toolPresetList"));
    list_->setSelectionMode(QAbstractItemView::SingleSelection);
    list_->setDragDropMode(QAbstractItemView::InternalMove);
    list_->setDefaultDropAction(Qt::MoveAction);
    list_->setDropIndicatorShown(true);
    list_->setResizeMode(QListView::Adjust);
    list_->setUniformItemSizes(true);
    list_->setIconSize(QSize(kSmallIcon, kSmallIcon));
    root->addWidget(list_, 1);

    auto* footer = new QWidget(this);
    footer->setObjectName(QStringLiteral("panelFooter"));
    auto* footerLayout = new QHBoxLayout(footer);
    footerLayout->setContentsMargins(4, 2, 4, 2);
    footerLayout->setSpacing(2);

    currentToolOnly_ = new QCheckBox(tr("Current Tool Only"), footer);
    currentToolOnly_->setObjectName(QStringLiteral("toolPresetCurrentOnly"));
    footerLayout->addWidget(currentToolOnly_);
    footerLayout->addStretch(1);

    auto makeButton = [&](const QString& glyph, const QString& tip, const QString& name) {
        auto* button = new QToolButton(footer);
        button->setObjectName(name);
        button->setText(glyph);
        button->setToolTip(tip);
        button->setAutoRaise(true);
        footerLayout->addWidget(button);
        return button;
    };
    newButton_ = makeButton(QStringLiteral("+"), tr("Create New Tool Preset"),
                            QStringLiteral("toolPresetsNew"));
    deleteButton_ = makeButton(QStringLiteral("\u2212"), tr("Delete the current tool preset"),
                               QStringLiteral("toolPresetsDelete"));
    deleteButton_->setEnabled(false);
    root->addWidget(footer);

    connect(newButton_, &QToolButton::clicked, this, &ToolPresetsPanel::createInteractive);
    connect(deleteButton_, &QToolButton::clicked, this, &ToolPresetsPanel::deleteCurrent);
    connect(currentToolOnly_, &QCheckBox::toggled, this, &ToolPresetsPanel::setCurrentToolOnly);
    connect(list_, &QListWidget::itemClicked, this, [this](QListWidgetItem*) { applyCurrent(); });
    connect(list_, &QListWidget::itemActivated, this, [this](QListWidgetItem*) { applyCurrent(); });
    connect(list_, &QListWidget::currentItemChanged, this,
            [this](QListWidgetItem* current, QListWidgetItem*) {
                if (updating_) {
                    return;
                }
                currentId_ = current ? current->data(Qt::UserRole).toULongLong() : 0;
                deleteButton_->setEnabled(currentId_ != 0);
            });
    connect(list_->model(), &QAbstractItemModel::rowsMoved, this,
            [this] { reorderFromList(); });
}

void ToolPresetsPanel::attachPresetStore(PresetStore& store)
{
    presets_ = &store;
    refresh();
    connect(&store, &PresetStore::changed, this, [this](int type) {
        if (type == int(PresetType::Tools) && !updating_) {
            refresh();
        }
    });
}

void ToolPresetsPanel::setController(ToolController* controller)
{
    controller_ = controller;
    if (!controller_) {
        return;
    }
    activeTool_ = controller_->activeTool();
    connect(controller_, &ToolController::activeToolChanged, this,
            &ToolPresetsPanel::setActiveTool);
    refresh();
}

void ToolPresetsPanel::setActiveTool(ToolId id)
{
    if (activeTool_ == id) {
        return;
    }
    activeTool_ = id;
    if (currentToolOnlyOn_) {
        refresh();
    }
}

void ToolPresetsPanel::setCurrentToolOnly(bool on)
{
    if (currentToolOnlyOn_ == on) {
        return;
    }
    currentToolOnlyOn_ = on;
    {
        const QSignalBlocker block(currentToolOnly_);
        currentToolOnly_->setChecked(on);
    }
    refresh();
}

void ToolPresetsPanel::setSortByTool(bool on)
{
    if (sortByTool_ == on) {
        return;
    }
    sortByTool_ = on;
    refresh();
}

void ToolPresetsPanel::setDisplayMode(DisplayMode mode)
{
    if (displayMode_ == mode) {
        return;
    }
    displayMode_ = mode;
    updatePresentation();
}

void ToolPresetsPanel::refresh()
{
    if (!presets_) {
        return;
    }
    const quint64 keep = currentId_;
    const QScopedValueRollback guard(updating_, true);

    struct Entry {
        PresetRow row;
        ToolId tool = ToolId::Move;
    };
    QList<Entry> entries;
    for (const auto& row : presets_->rows(PresetType::Tools)) {
        bool ok = false;
        const ToolId tool = toolPresetToolId(row.values, &ok);
        if (!ok || (currentToolOnlyOn_ && tool != activeTool_)) {
            continue;
        }
        entries.push_back({row, tool});
    }
    if (sortByTool_) {
        std::stable_sort(entries.begin(), entries.end(), [](const Entry& a, const Entry& b) {
            return toolLabel(a.tool) < toolLabel(b.tool);
        });
    }

    list_->clear();
    QListWidgetItem* current = nullptr;
    for (const Entry& entry : entries) {
        auto* item = new QListWidgetItem(entry.row.name, list_);
        item->setData(Qt::UserRole, QVariant::fromValue<qulonglong>(entry.row.id));
        item->setData(Qt::UserRole + 1, toolIdName(entry.tool));
        item->setData(Qt::UserRole + 2, toolLabel(entry.tool));
        item->setToolTip(tr("%1 — %2").arg(entry.row.name, toolLabel(entry.tool)));
        if (entry.row.id == keep) {
            current = item;
        }
    }
    if (current) {
        list_->setCurrentItem(current);
    } else if (list_->count() > 0) {
        list_->setCurrentRow(0);
    }
    currentId_ = current ? keep : (list_->currentItem()
                                       ? list_->currentItem()->data(Qt::UserRole).toULongLong()
                                       : 0);
    deleteButton_->setEnabled(currentId_ != 0);
    updatePresentation();
    updateDragPolicy();
}

void ToolPresetsPanel::updatePresentation()
{
    const bool textOnly = displayMode_ == DisplayMode::TextOnly;
    const int edge = displayMode_ == DisplayMode::LargeList ? kLargeIcon : kSmallIcon;
    list_->setIconSize(textOnly ? QSize() : QSize(edge, edge));
    for (int index = 0; index < list_->count(); ++index) {
        QListWidgetItem* item = list_->item(index);
        if (textOnly) {
            item->setIcon(QIcon());
            continue;
        }
        const QString name = item->data(Qt::UserRole + 1).toString();
        for (ToolId id : allToolIds()) {
            if (toolIdName(id) == name) {
                item->setIcon(pictura::icon(QStringLiteral("tool.") + name));
                break;
            }
        }
    }
}

void ToolPresetsPanel::updateDragPolicy()
{
    const bool canReorder = !currentToolOnlyOn_ && !sortByTool_;
    list_->setDragDropMode(canReorder ? QAbstractItemView::InternalMove
                                      : QAbstractItemView::NoDragDrop);
    list_->setDefaultDropAction(Qt::MoveAction);
}

QString ToolPresetsPanel::nextPresetName() const
{
    return uniqueName(tr("%1 Preset").arg(toolLabel(activeTool_)));
}

QString ToolPresetsPanel::uniqueName(const QString& base) const
{
    if (!presets_) {
        return base;
    }
    QSet<QString> names;
    for (const auto& row : presets_->rows(PresetType::Tools)) {
        names.insert(row.name);
    }
    if (!names.contains(base)) {
        return base;
    }
    for (int suffix = 2;; ++suffix) {
        const QString candidate = tr("%1 %2").arg(base).arg(suffix);
        if (!names.contains(candidate)) {
            return candidate;
        }
    }
}

quint64 ToolPresetsPanel::currentPresetId() const
{
    const QListWidgetItem* item = list_->currentItem();
    return item ? item->data(Qt::UserRole).toULongLong() : 0;
}

const PresetRow* ToolPresetsPanel::currentRow() const
{
    if (!presets_) {
        return nullptr;
    }
    const quint64 id = currentPresetId();
    if (id == 0) {
        return nullptr;
    }
    for (const auto& row : presets_->rows(PresetType::Tools)) {
        if (row.id == id) {
            return &row;
        }
    }
    return nullptr;
}

void ToolPresetsPanel::createInteractive()
{
    bool accepted = false;
    const QString name = QInputDialog::getText(this, tr("New Tool Preset"), tr("Name:"),
                                               QLineEdit::Normal, nextPresetName(), &accepted);
    if (accepted && !name.trimmed().isEmpty()) {
        createPreset(name.trimmed());
    }
}

void ToolPresetsPanel::createPreset(const QString& name)
{
    if (!controller_ || !presets_ || name.isEmpty()) {
        return;
    }
    const QString tool = toolPresetToolName(activeTool_);
    const QJsonObject options = toolOptionsToJson(*controller_);
    const QByteArray encoded = QJsonDocument(options).toJson(QJsonDocument::Compact);
    QJsonObject values = options;
    values.insert(QStringLiteral("tool"), tool);
    values.insert(QStringLiteral("toolId"), toolIdName(activeTool_));
    values.insert(QStringLiteral("options"), options);
    PresetRow row;
    row.name = name;
    row.native = asBytes(preset_make_tool(name, tool, QString::fromUtf8(encoded)));
    row.values = values;
    QString error;
    if (!presets_->append(PresetType::Tools, {row}, &error)) {
        QMessageBox::warning(this, tr("Tool Presets"), error);
        return;
    }
    refresh();
    if (list_->count() > 0) {
        list_->setCurrentRow(list_->count() - 1);
    }
}

void ToolPresetsPanel::renameInteractive()
{
    const PresetRow* row = currentRow();
    if (!row) {
        return;
    }
    bool accepted = false;
    const QString name = QInputDialog::getText(this, tr("Rename Tool Preset"), tr("Name:"),
                                               QLineEdit::Normal, row->name, &accepted);
    if (accepted && !name.trimmed().isEmpty()) {
        renameCurrent(name.trimmed());
    }
}

void ToolPresetsPanel::renameCurrent(const QString& name)
{
    const PresetRow* row = currentRow();
    if (!presets_ || !row || name.isEmpty()) {
        return;
    }
    const quint64 id = row->id;
    auto rows = presets_->rows(PresetType::Tools);
    for (auto& entry : rows) {
        if (entry.id == id) {
            entry.name = name;
            break;
        }
    }
    QString error;
    if (!presets_->replace(PresetType::Tools, std::move(rows), &error)) {
        QMessageBox::warning(this, tr("Tool Presets"), error);
        return;
    }
    currentId_ = id;
    refresh();
}

void ToolPresetsPanel::deleteCurrent()
{
    const PresetRow* row = currentRow();
    if (!presets_ || !row) {
        return;
    }
    const quint64 id = row->id;
    auto rows = presets_->rows(PresetType::Tools);
    rows.removeIf([id](const PresetRow& entry) { return entry.id == id; });
    QString error;
    if (!presets_->replace(PresetType::Tools, std::move(rows), &error)) {
        QMessageBox::warning(this, tr("Tool Presets"), error);
        return;
    }
    currentId_ = 0;
    refresh();
}

void ToolPresetsPanel::applyCurrent()
{
    const PresetRow* row = currentRow();
    if (!row || !controller_) {
        return;
    }
    bool ok = false;
    const ToolId tool = toolPresetToolId(row->values, &ok);
    if (!ok) {
        QMessageBox::warning(this, tr("Tool Presets"),
                             tr("This preset's tool is not available."));
        return;
    }
    if (controller_->activeTool() != tool) {
        controller_->setActiveTool(tool);
    }
    applyToolPreset(*controller_, row->values);
}

void ToolPresetsPanel::reorderFromList()
{
    if (updating_ || !presets_) {
        return;
    }
    QList<quint64> order;
    for (int index = 0; index < list_->count(); ++index) {
        order.push_back(list_->item(index)->data(Qt::UserRole).toULongLong());
    }
    auto stored = presets_->rows(PresetType::Tools);
    PresetRows rows;
    rows.reserve(stored.size());
    for (quint64 id : order) {
        const auto it = std::find_if(stored.cbegin(), stored.cend(),
                                     [id](const PresetRow& row) { return row.id == id; });
        if (it != stored.cend()) {
            rows.push_back(*it);
        }
    }
    for (const auto& row : stored) {
        if (!order.contains(row.id)) {
            rows.push_back(row);
        }
    }
    QString error;
    if (!presets_->replace(PresetType::Tools, std::move(rows), &error)) {
        QMessageBox::warning(this, tr("Tool Presets"), error);
        refresh();
    }
}

void ToolPresetsPanel::moveRowsForTest(int from, int to)
{
    if (from < 0 || from >= list_->count() || to < 0 || to > list_->count() || from == to) {
        return;
    }
    QListWidgetItem* item = list_->takeItem(from);
    list_->insertItem(to > from ? to - 1 : to, item);
    list_->setCurrentItem(item);
    reorderFromList();
}

void ToolPresetsPanel::runPresetCommand(const QString& action)
{
    if (action == QStringLiteral("toolPresets.new")) {
        createInteractive();
    } else if (action == QStringLiteral("toolPresets.rename")) {
        renameInteractive();
    } else if (action == QStringLiteral("toolPresets.delete")) {
        deleteCurrent();
    } else if (action == QStringLiteral("toolPresets.currentToolOnly")) {
        setCurrentToolOnly(!currentToolOnlyOn_);
    } else if (action == QStringLiteral("toolPresets.sortByTool")) {
        setSortByTool(!sortByTool_);
    } else if (action == QStringLiteral("toolPresets.display.text")) {
        setDisplayMode(DisplayMode::TextOnly);
    } else if (action == QStringLiteral("toolPresets.display.small")) {
        setDisplayMode(DisplayMode::SmallList);
    } else if (action == QStringLiteral("toolPresets.display.large")) {
        setDisplayMode(DisplayMode::LargeList);
    } else if (action.startsWith(QStringLiteral("toolPresets."))) {
        openPresetManager(window(), PresetType::Tools, action);
    } else if (action.startsWith(QStringLiteral("append:"))
               || action.startsWith(QStringLiteral("replace:"))) {
        openPresetManager(window(), PresetType::Tools, action);
    }
}

void ToolPresetsPanel::updatePanelMenu(QMenu* menu)
{
    for (QAction* action : menu->actions()) {
        const QString id = action->data().toString();
        if (id == QStringLiteral("toolPresets.currentToolOnly")) {
            action->setChecked(currentToolOnlyOn_);
        } else if (id == QStringLiteral("toolPresets.sortByTool")) {
            action->setChecked(sortByTool_);
        } else if (id == QStringLiteral("toolPresets.display.text")) {
            action->setChecked(displayMode_ == DisplayMode::TextOnly);
        } else if (id == QStringLiteral("toolPresets.display.small")) {
            action->setChecked(displayMode_ == DisplayMode::SmallList);
        } else if (id == QStringLiteral("toolPresets.display.large")) {
            action->setChecked(displayMode_ == DisplayMode::LargeList);
        } else if (id == QStringLiteral("toolPresets.rename")
                   || id == QStringLiteral("toolPresets.delete")) {
            action->setEnabled(currentPresetId() != 0);
        }
    }
    for (QAction* action : menu->actions()) {
        if (action->property("toolPresetsLibrary").toBool()) {
            menu->removeAction(action);
            action->deleteLater();
        }
    }
    for (const QString& file : discoveredPresetFiles(PresetType::Tools)) {
        QMenu* library = menu->addMenu(QFileInfo(file).completeBaseName());
        library->menuAction()->setProperty("toolPresetsLibrary", true);
        library->addAction(tr("Append"), this,
                           [this, file] { runPresetCommand(QStringLiteral("append:") + file); });
        library->addAction(tr("Replace"), this,
                           [this, file] { runPresetCommand(QStringLiteral("replace:") + file); });
    }
}

} // namespace pictura

#include "moc_tool_presets_panel.cpp"
