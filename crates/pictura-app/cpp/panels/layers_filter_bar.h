#pragma once

#include "layers_filter_proxy.h"

#include <QtWidgets/QWidget>

#include <array>

class QComboBox;
class QLineEdit;
class QStackedWidget;
class QToolButton;

namespace pictura {

class LayerFilterBar : public QWidget {
    Q_OBJECT

public:
    explicit LayerFilterBar(QWidget* parent = nullptr);

    void setFilter(const LayerFilter& filter);
    LayerFilter filter() const { return filter_; }

    /// Enter isolation mode: choose the Selected dimension and turn the filter
    /// on, emitting `filterChanged` once.
    void activateSelected();
    /// Turn the filter toggle on or off, emitting `filterChanged` on a change.
    void setFilterEnabled(bool on);

    int dimensionIndexForTest() const;
    void setDimensionForTest(const QString& key);
    bool toggleOnForTest() const;
    bool toggleHasIconForTest() const;

signals:
    void filterChanged(const LayerFilter& filter);

private:
    LayerFilter buildFilter() const;
    QString activeDimension() const;
    void userChanged();

    QComboBox* dimension_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QToolButton* toggle_ = nullptr;
    QLineEdit* name_ = nullptr;
    std::array<QToolButton*, 5> kindButtons_{};
    QComboBox* effect_ = nullptr;
    QComboBox* mode_ = nullptr;
    QComboBox* attribute_ = nullptr;
    QComboBox* color_ = nullptr;
    LayerFilter filter_;
    bool syncing_ = false;
};

} // namespace pictura
