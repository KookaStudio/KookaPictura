#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtWidgets/QWidget>

class QEvent;
class QListWidget;
class QListWidgetItem;
class QPushButton;

namespace pictura {

class PictureView;

class HistoryPanel : public QWidget {
    Q_OBJECT

public:
    struct Target {
        bool snapshot = false;
        int index = -1;
        bool valid() const { return index >= 0; }
    };

    explicit HistoryPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();

    // Phase D: run a wired History per-widget menu entry by its action id.
    // Returns false for ids this panel does not own.
    bool performPanelMenuAction(const QString& actionId);

    // The selected snapshot/state row, or the current state when none is
    // selected.
    Target selectedTarget() const;

    // Clear History: plain keeps named snapshots; purge drops them too.
    void clearHistory(bool purge);
    void openHistoryOptions();

    bool autoFirstSnapshot() const { return autoFirstSnapshot_; }
    bool autoSnapshotOnSave() const { return autoSnapshotOnSave_; }
    void setOptionsForTest(bool autoFirst, bool autoSave, bool nonLinear, bool showDialog,
                           bool visibilityUndoable);

protected:
    // A press in the left (icon) column chooses the History Brush source
    // instead of jumping to the state.
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void activate(QListWidgetItem* item);
    void createSnapshot(bool prompt);
    void applyOptionsToView();
    void persistOptions();

    PictureView* view_ = nullptr;
    QListWidget* list_ = nullptr;
    QPushButton* snapshotButton_ = nullptr;
    QMetaObject::Connection viewConnection_;
    bool autoFirstSnapshot_ = true;
    bool autoSnapshotOnSave_ = false;
    bool allowNonLinear_ = false;
    bool showSnapshotDialog_ = false;
    bool visibilityUndoable_ = false;
};

} // namespace pictura
