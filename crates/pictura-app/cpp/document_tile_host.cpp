#include "document_tile_host.h"

#include "document_float_window.h"
#include "tab_spinner.h"

#include <QtGui/QContextMenuEvent>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QVBoxLayout>

#include <cmath>

namespace pictura {

namespace {

// The pane header: a press activates the window, a right-click asks for the
// header menu. Panes have a fixed place, so the header does not drag.
class DocumentTileHeader : public QWidget {
public:
    explicit DocumentTileHeader(DocumentTilePane* pane)
        : QWidget(pane), pane_(pane)
    {
        setObjectName(QStringLiteral("documentTileHeader"));
        setAttribute(Qt::WA_StyledBackground, true);
        setFixedHeight(DocumentFloatWindow::kHeaderHeight);
    }

protected:
    void mousePressEvent(QMouseEvent* event) override
    {
        if (pane_->onActivated) {
            pane_->onActivated();
        }
        event->accept();
    }

    void contextMenuEvent(QContextMenuEvent* event) override
    {
        if (pane_->onContextMenu) {
            pane_->onContextMenu(event->globalPos());
        }
        event->accept();
    }

private:
    DocumentTilePane* pane_;
};

} // namespace

DocumentTilePane::DocumentTilePane(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("documentTilePane"));
    setAttribute(Qt::WA_StyledBackground, true);
    // The grid shares the host by stretch alone, so every pane gets the same
    // share whatever its page would prefer.
    setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Ignored);

    layout_ = new QVBoxLayout(this);
    layout_->setContentsMargins(0, 0, 0, 0);
    layout_->setSpacing(0);

    header_ = new DocumentTileHeader(this);
    auto* headerLayout = new QHBoxLayout(header_);
    headerLayout->setContentsMargins(8, 0, 4, 0);
    headerLayout->setSpacing(4);
    title_ = new DocumentWindowTitle(header_);
    title_->setObjectName(QStringLiteral("documentTileTitle"));
    headerLayout->addWidget(title_, 1);
    auto* close = new TabCloseButton(header_);
    close->setObjectName(QStringLiteral("documentTileClose"));
    close_ = close;
    // The frame closes the window on the next event-loop turn, so the button's
    // own click handler has returned before the pane goes away.
    connect(close, &TabCloseButton::closeRequested, this, [this]() {
        if (onCloseRequested) {
            onCloseRequested();
        }
    });
    headerLayout->addWidget(close);
    layout_->addWidget(header_);
}

void DocumentTilePane::setPage(QWidget* page)
{
    if (!page) {
        return;
    }
    page_ = page;
    layout_->addWidget(page, 1);
    page->show();
}

QWidget* DocumentTilePane::takePage()
{
    QWidget* page = page_;
    page_ = nullptr;
    if (page) {
        layout_->removeWidget(page);
        page->setParent(nullptr);
    }
    return page;
}

void DocumentTilePane::setLabel(const QString& label, bool dirty)
{
    title_->setFullText(label, dirty);
}

QString DocumentTilePane::headerText() const
{
    return title_->fullText();
}

QAbstractButton* DocumentTilePane::closeButton() const
{
    return close_;
}

void DocumentTilePane::setHeaderVisible(bool visible)
{
    header_->setVisible(visible);
}

DocumentTileHost::DocumentTileHost(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("documentTileHost"));
    relayout();
}

DocumentTilePane* DocumentTileHost::addPane(QWidget* page)
{
    auto* pane = new DocumentTilePane(this);
    pane->setPage(page);
    panes_.append(pane);
    relayout();
    pane->show();
    return pane;
}

QWidget* DocumentTileHost::removePane(DocumentTilePane* pane)
{
    if (!panes_.removeOne(pane)) {
        return nullptr;
    }
    QWidget* page = pane->takePage();
    relayout();
    pane->hide();
    // The pane may be handling the event that removed it.
    pane->deleteLater();
    return page;
}

int DocumentTileHost::columnCount() const
{
    return panes_.isEmpty() ? 0 : int(std::ceil(std::sqrt(double(panes_.size()))));
}

void DocumentTileHost::relayout()
{
    // A fresh grid, so rows and columns a larger tiling used leave no stretch
    // behind.
    delete grid_;
    grid_ = new QGridLayout(this);
    grid_->setContentsMargins(0, 0, 0, 0);
    grid_->setSpacing(1);
    const int columns = columnCount();
    for (int i = 0; i < panes_.size(); ++i) {
        grid_->addWidget(panes_.at(i), i / columns, i % columns);
    }
    for (int column = 0; column < columns; ++column) {
        grid_->setColumnStretch(column, 1);
    }
    for (int row = 0; columns > 0 && row * columns < panes_.size(); ++row) {
        grid_->setRowStretch(row, 1);
    }
}

} // namespace pictura

#include "moc_document_tile_host.cpp"
