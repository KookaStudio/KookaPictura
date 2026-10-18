#pragma once

#include <QtCore/QList>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

class QToolButton;

namespace pictura {

class ColorState;

// One colour in the Swatches panel; the name is what the tooltip shows.
struct Swatch {
    QColor color;
    QString name;
};

// The reflowing grid of colour squares. How many fit on a row follows the
// width it is given, so the panel can show more per row as it is dragged wider.
class SwatchGrid : public QWidget {
    Q_OBJECT

public:
    explicit SwatchGrid(QWidget* parent = nullptr);

    const QList<Swatch>& swatches() const { return swatches_; }
    void setSwatches(QList<Swatch> swatches);
    void addSwatch(const Swatch& swatch);
    bool removeSwatch(int index);

    // The next unused "Swatch N" name, counting the names already in the grid.
    QString nextSwatchName() const;

    static QList<Swatch> defaultSwatches();

    int indexAt(const QPoint& pos) const;
    int heightFor(int width) const;
    int columnsForWidth(int width) const;
    QSize sizeHint() const override;
    QSize minimumSizeHint() const override;

    int current() const { return current_; }

signals:
    void foregroundPicked(const QColor& color);
    void backgroundPicked(const QColor& color);
    void newSwatchRequested();
    void currentChanged(int index);
    void countChanged();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    bool event(QEvent* event) override;

private:
    QRect cellRect(int index) const;
    int columns() const;

    QList<Swatch> swatches_;
    int current_ = -1;
};

// CS6's Swatches panel: the grid and its footer. Click a swatch for the
// foreground, ctrl-click for the background, alt-click to delete; the empty
// space past the last swatch adds one from the foreground.
//
// ponytail: loading and saving `.aco` libraries is deferred (needs a presets
// crate that does not exist yet); the footer carries only New and Delete.
class SwatchesPanel : public QWidget {
    Q_OBJECT

public:
    explicit SwatchesPanel(ColorState* state, QWidget* parent = nullptr);

    SwatchGrid* grid() const { return grid_; }

    // Add a swatch from the foreground colour; an empty `name` uses its hex.
    void addSwatchFromForeground();
    void deleteCurrentSwatch();

    // Restore the built-in default palette.
    void resetSwatches();

private:
    void showContextMenu(const QPoint& at);

    ColorState* state_ = nullptr;
    SwatchGrid* grid_ = nullptr;
    QToolButton* deleteButton_ = nullptr;
};

} // namespace pictura
