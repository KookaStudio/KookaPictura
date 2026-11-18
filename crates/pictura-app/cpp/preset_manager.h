#pragma once

#include "preset_store.h"

#include <QtWidgets/QDialog>

class QComboBox;
class QListView;
class QMenu;

namespace pictura {

class PresetListModel;

class PresetManager : public QDialog {
    Q_OBJECT

public:
    explicit PresetManager(PresetStore& store, PresetType type = PresetType::Brushes,
                           QWidget* parent = nullptr);
    void runLibraryAction(const QString& action);
    void importFile(QString path, bool replace);
    void exportFile(QString path);

private:
    PresetType type() const;
    void changePage();
    void changeDisplay();
    void pickImport(bool replace);
    void cancelImport();
    void saveSet(const QString& format = {});
    void resetType();
    void buildLibraryMenu();

    PresetStore& store_;
    QComboBox* type_ = nullptr;
    QComboBox* display_ = nullptr;
    QListView* list_ = nullptr;
    QMenu* menu_ = nullptr;
    PresetListModel* model_ = nullptr;
    PresetType importType_ = PresetType::Brushes;
    quint64 importRequest_ = 0;
};

void openPresetManager(QWidget* parent, PresetType type = PresetType::Brushes,
                       const QString& action = {});

} // namespace pictura
