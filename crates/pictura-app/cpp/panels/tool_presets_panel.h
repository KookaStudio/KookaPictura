#pragma once

#include <QtWidgets/QWidget>

#include "preset_store.h"
#include "tools.h"

class QCheckBox;
class QListWidget;
class QMenu;
class QToolButton;

namespace pictura {

class ToolController;

// The CS6 Tool Presets panel: a list over the shared Tools preset set, with a
// Current Tool Only filter, Sort By Tool, display modes, and library commands.
class ToolPresetsPanel : public QWidget {
    Q_OBJECT

public:
    enum class DisplayMode { TextOnly, SmallList, LargeList };

    explicit ToolPresetsPanel(QWidget* parent = nullptr);

    void attachPresetStore(PresetStore& store);
    void setController(ToolController* controller);
    void setActiveTool(ToolId id);
    ToolId activeTool() const { return activeTool_; }

    void createPreset(const QString& name);
    void renameCurrent(const QString& name);
    void deleteCurrent();
    void applyCurrent();

    void setCurrentToolOnly(bool on);
    bool currentToolOnly() const { return currentToolOnlyOn_; }
    void setSortByTool(bool on);
    bool sortByTool() const { return sortByTool_; }
    void setDisplayMode(DisplayMode mode);
    DisplayMode displayMode() const { return displayMode_; }

    void runPresetCommand(const QString& action);
    void updatePanelMenu(QMenu* menu);
    // Move a row as an internal drag would, then persist the new order.
    void moveRowsForTest(int from, int to);

    QListWidget* listForTest() const { return list_; }
    QCheckBox* currentToolOnlyForTest() const { return currentToolOnly_; }
    QToolButton* newButtonForTest() const { return newButton_; }
    QToolButton* deleteButtonForTest() const { return deleteButton_; }

private:
    void refresh();
    void updatePresentation();
    void updateDragPolicy();
    void createInteractive();
    void renameInteractive();
    void reorderFromList();
    QString nextPresetName() const;
    QString uniqueName(const QString& base) const;
    const PresetRow* currentRow() const;
    quint64 currentPresetId() const;

    PresetStore* presets_ = nullptr;
    ToolController* controller_ = nullptr;
    QListWidget* list_ = nullptr;
    QCheckBox* currentToolOnly_ = nullptr;
    QToolButton* newButton_ = nullptr;
    QToolButton* deleteButton_ = nullptr;
    ToolId activeTool_ = ToolId::Move;
    DisplayMode displayMode_ = DisplayMode::SmallList;
    bool currentToolOnlyOn_ = false;
    bool sortByTool_ = false;
    bool updating_ = false;
    quint64 currentId_ = 0;
};

} // namespace pictura
