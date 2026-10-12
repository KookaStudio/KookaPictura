#pragma once

#include <QtWidgets/QWidget>

class QLabel;
class QListWidget;
class QListWidgetItem;
class QToolButton;

namespace pictura {

class PictureView;
struct ToolServices;

// Window > Paths: the active shape layer's "<Layer> Shape Path", the saved
// paths, and the temporary Work Path (italic), each with a thumbnail. The
// selected row is the path the Pen group and the path selection tools edit and
// the only one shown; clicking empty space hides them all. Double-click the
// Work Path to save it, a saved path to rename it. The footer fills the path
// with the foreground, strokes it with the current brush, loads it as a
// selection (nonzero winding), and makes or deletes paths; the row menu adds
// Duplicate Path and a feathered Make Selection. Ported from photorust's
// PathsPanel.
// ponytail: Make Work Path from a selection is shown disabled; no panel menu,
// Clipping Path, or thumbnail-size options; paths are not saved to PSD.
class PathsPanel : public QWidget {
    Q_OBJECT

public:
    explicit PathsPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    // Supplies the foreground and brush for Fill / Stroke Path; without it they
    // use black and a 5 px hard round brush.
    void setToolServices(const ToolServices* tools) { tools_ = tools; }
    void refresh();

    // The footer actions on the selected row; false when nothing happened.
    bool newPath();
    bool deletePath();
    bool duplicatePath();
    bool saveWorkPath(const QString& name);
    bool fillPath();
    bool strokePath();
    bool loadSelection(double feather = 0.0);

    QListWidget* listForTest() const { return list_; }

private:
    // A row's Qt::UserRole value: a saved path's index, or one of these.
    static constexpr int kWorkRow = -1;
    static constexpr int kShapeRow = -3;

    int selectedRow() const;
    // Run `command` on the selected row's path, pointing the path calls at the
    // shape layer's outline for the shape row.
    template <typename Command>
    bool onSelected(Command command);
    void onSelectionChanged();
    void onItemChanged(QListWidgetItem* item);
    void onDoubleClicked(QListWidgetItem* item);
    void onContextMenu(const QPoint& pos);
    void updateFooter();
    bool eventFilter(QObject* watched, QEvent* event) override;

    PictureView* view_ = nullptr;
    const ToolServices* tools_ = nullptr;
    QListWidget* list_ = nullptr;
    QLabel* empty_ = nullptr;
    QToolButton* fill_ = nullptr;
    QToolButton* stroke_ = nullptr;
    QToolButton* load_ = nullptr;
    QToolButton* makeWork_ = nullptr;
    QToolButton* add_ = nullptr;
    QToolButton* remove_ = nullptr;
    // refresh() rewrites the list; its signals must not write back.
    bool updating_ = false;
};

} // namespace pictura
