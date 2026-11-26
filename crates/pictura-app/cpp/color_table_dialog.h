#pragma once

#include <QtCore/QByteArray>
#include <QtGui/QColor>
#include <QtWidgets/QDialog>

class QTableWidget;

namespace pictura {

class ColorTableDialog : public QDialog {
public:
    ColorTableDialog(const QByteArray& rgba, int minimumEntries, QWidget* parent = nullptr);

    const QByteArray& table() const { return rgba_; }
    void interpolate(int first, int last, const QColor& from, const QColor& to);
    bool loadAct(const QByteArray& bytes);
    QByteArray saveAct() const;

private:
    int currentEntry() const;
    QColor entryColor(int entry) const;
    void editSelection();
    void markTransparent(int entry);
    void refresh();
    void loadFile();
    void saveFile();

    QByteArray rgba_;
    int minimumEntries_;
    QTableWidget* grid_;
};

} // namespace pictura
