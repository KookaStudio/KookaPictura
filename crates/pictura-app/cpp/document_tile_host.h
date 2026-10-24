#pragma once

#include <QtCore/QList>
#include <QtCore/QPoint>
#include <QtCore/QString>
#include <QtWidgets/QWidget>

#include <functional>

class QAbstractButton;
class QGridLayout;
class QVBoxLayout;

namespace pictura {

class DocumentWindowTitle;

// A tile pane: a header (label, modified marker, close control) over one
// document window's page. The frame owns the document state; the pane only
// reports what the user does to it.
class DocumentTilePane : public QWidget {
    Q_OBJECT

public:
    explicit DocumentTilePane(QWidget* parent = nullptr);

    // Host `page` under the header. The page is reparented into the pane.
    void setPage(QWidget* page);
    QWidget* page() const { return page_; }
    // Give the page up, unparented and hidden, for the caller to rehost.
    QWidget* takePage();

    // The header shows `label` elided to its width, then the modified marker.
    void setLabel(const QString& label, bool dirty);
    // The unelided header text: the label and, while modified, the marker.
    QString headerText() const;
    QWidget* header() const { return header_; }
    QAbstractButton* closeButton() const;
    // A full-screen mode shows the active pane's page without its header.
    void setHeaderVisible(bool visible);

    // The header was pressed.
    std::function<void()> onActivated;
    std::function<void()> onCloseRequested;
    std::function<void(const QPoint& globalPos)> onContextMenu;

private:
    QWidget* header_ = nullptr;
    DocumentWindowTitle* title_ = nullptr;
    QAbstractButton* close_ = nullptr;
    QVBoxLayout* layout_ = nullptr;
    QWidget* page_ = nullptr;
};

// Window > Arrange > Tile: the panes in a grid of `ceil(sqrt(n))` columns and
// as many rows as needed, filled row by row in the order they were added, that
// fills the host.
class DocumentTileHost : public QWidget {
    Q_OBJECT

public:
    explicit DocumentTileHost(QWidget* parent = nullptr);

    // Add a pane holding `page` after the others and re-tile.
    DocumentTilePane* addPane(QWidget* page);
    // Take `pane` out of the grid, re-tile, and delete it; its page comes
    // back unparented and hidden.
    QWidget* removePane(DocumentTilePane* pane);
    QList<DocumentTilePane*> panes() const { return panes_; }
    int paneCount() const { return int(panes_.size()); }
    int columnCount() const;

private:
    void relayout();

    QGridLayout* grid_ = nullptr;
    QList<DocumentTilePane*> panes_;
};

} // namespace pictura
