#pragma once

#include <QtCore/QMetaObject>
#include <QtWidgets/QWidget>

class QListWidget;
class QListWidgetItem;
class QMenu;
class QToolButton;

namespace pictura {

class PictureView;

class LayerCompsPanel : public QWidget {
    Q_OBJECT

public:
    explicit LayerCompsPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();
    void updatePanelMenu(QMenu* menu);

    bool createComp(bool skipDialog = false);
    bool applySelected();
    bool previousComp();
    bool nextComp();
    bool updateSelected();
    bool deleteSelected();
    void renameSelected();
    void openOptions();
    bool duplicateSelected();
    bool clearWarningSelected();
    bool clearAllWarnings();

    QListWidget* listForTest() const { return list_; }
    int selectedCompId() const;

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void addLastRow();
    void refreshControls();
    void selectCompId(int id);
    void addCompRow(int index);
    bool applyItem(QListWidgetItem* item);
    void showWarningMenu(const QPoint& globalPos);
    bool anyWarning() const;
    bool selectedIsLast() const;
    QListWidgetItem* currentCompItem() const;

    PictureView* view_ = nullptr;
    QListWidget* list_ = nullptr;
    QToolButton* create_ = nullptr;
    QToolButton* apply_ = nullptr;
    QToolButton* previous_ = nullptr;
    QToolButton* next_ = nullptr;
    QToolButton* update_ = nullptr;
    QToolButton* delete_ = nullptr;
    QMetaObject::Connection viewConnection_;
    int recordBits_ = 1;
    bool updating_ = false;
};

} // namespace pictura
