#pragma once

#include <QtWidgets/QWidget>

class QListWidget;
class QToolButton;

namespace pictura {

class PictureView;

// Type > Character Styles / Window > Character Styles: the document's named
// character styles, applied to the active type layer by clicking one. The
// footer creates a style from the options dialog and deletes the selected one;
// double-clicking a row edits it in place, which re-applies it to every layer
// that set it.
class CharacterStylesPanel : public QWidget {
    Q_OBJECT

public:
    explicit CharacterStylesPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    void refresh();

    QListWidget* listForTest() const { return list_; }
    void applyRowForTest(int row) { applyRow(row); }

    bool createStyle();
    bool editStyle(int row);
    bool deleteSelectedStyle();

private:
    int selectedRow() const;
    void applyRow(int row);

    PictureView* view_ = nullptr;
    QListWidget* list_ = nullptr;
    QToolButton* add_ = nullptr;
    QToolButton* remove_ = nullptr;
    bool updating_ = false;
};

} // namespace pictura
