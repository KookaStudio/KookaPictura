#pragma once

#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtWidgets/QGroupBox>

class QComboBox;
class QListWidget;

namespace pictura {

// The Save for Web Color Table: swatches, selection, double-click shift,
// add/delete, sort, and map/unmap transparency. The dialog applies the
// operations so the active pane's settings stay authoritative.
class ColorTableWidget : public QGroupBox {
    Q_OBJECT

public:
    explicit ColorTableWidget(QWidget* parent = nullptr);

    void setColors(const QVector<QRgb>& colors);
    QVector<QRgb> colors() const { return colors_; }
    int selectedIndex() const;
    void selectIndex(int index);

    void editSelected(const QColor& color);
    void deleteSelected();
    void addColor(const QColor& color);
    void mapSelected(bool transparent);
    void sortBy(int sort);

signals:
    void colorsEdited(const QVector<QRgb>& colors);
    void addRequested(QRgb color);
    void deleteRequested(int index);
    void sortRequested(int sort);
    void transparencyRequested(int index, bool transparent);
    void loadRequested();
    void saveRequested();

private:
    void rebuild();

    QListWidget* swatches_ = nullptr;
    QComboBox* sort_ = nullptr;
    QVector<QRgb> colors_;
};

} // namespace pictura
