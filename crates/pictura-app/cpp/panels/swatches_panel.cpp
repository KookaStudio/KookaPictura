#include "swatches_panel.h"

#include "color_panel.h"

#include <QtCore/QEvent>
#include <QtGui/QAction>
#include <QtGui/QHelpEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QMenu>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QToolTip>
#include <QtWidgets/QVBoxLayout>

#include <iterator>

namespace pictura {

namespace {

constexpr int kCell = 17;
constexpr int kGap = 1;
constexpr int kMargin = 2;

const QColor kMarkOuter(0x1e, 0x1e, 0x1e);
const QColor kMarkInner(0xff, 0xff, 0xff);

// The hues the current Kooka palette sweeps, in spectrum order.
struct Hue {
    int degrees;
    const char* name;
};

const Hue kHues[] = {
    {0, "Red"},         {30, "Orange"},      {60, "Yellow"},  {90, "Chartreuse"},
    {120, "Green"},     {150, "Spring"},     {180, "Cyan"},   {210, "Azure"},
    {240, "Blue"},      {270, "Violet"},     {300, "Magenta"}, {330, "Rose"},
};

} // namespace

SwatchGrid::SwatchGrid(QWidget* parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("swatchGrid"));
    setSwatches(defaultSwatches());
    setMouseTracking(true);
}

QList<Swatch> SwatchGrid::defaultSwatches()
{
    QList<Swatch> out;
    const auto add = [&out](const QColor& color, const QString& name) {
        out.append({color, name});
    };

    const int columns = int(std::size(kHues));
    for (int col = 0; col < columns; ++col) {
        const Hue& hue = kHues[col];
        add(QColor::fromHsv(hue.degrees, 255, 255), QObject::tr(hue.name));
    }
    for (int col = 0; col < columns; ++col) {
        const Hue& hue = kHues[col];
        add(QColor::fromHsv(hue.degrees, 130, 255),
            QObject::tr("Light %1").arg(QObject::tr(hue.name)));
    }
    for (int col = 0; col < columns; ++col) {
        const Hue& hue = kHues[col];
        add(QColor::fromHsv(hue.degrees, 255, 160),
            QObject::tr("Dark %1").arg(QObject::tr(hue.name)));
    }
    for (int col = 0; col < columns; ++col) {
        const int gray = col * 255 / (columns - 1);
        const QString name = col == 0       ? QObject::tr("Black")
                             : col == columns - 1 ? QObject::tr("White")
                                                  : QObject::tr("%1% Gray").arg(100 - col * 100 / (columns - 1));
        add(QColor(gray, gray, gray), name);
    }
    return out;
}

void SwatchGrid::setSwatches(QList<Swatch> swatches)
{
    swatches_ = std::move(swatches);
    current_ = -1;
    updateGeometry();
    update();
    emit countChanged();
    emit currentChanged(current_);
}

void SwatchGrid::addSwatch(const Swatch& swatch)
{
    swatches_.append(swatch);
    current_ = int(swatches_.size()) - 1;
    updateGeometry();
    update();
    emit countChanged();
    emit currentChanged(current_);
}

bool SwatchGrid::removeSwatch(int index)
{
    if (index < 0 || index >= swatches_.size()) {
        return false;
    }
    swatches_.removeAt(index);
    // Hold the place rather than the index: deleting a run keeps taking the
    // next one along, and the last deletion marks what is now the end.
    current_ = swatches_.isEmpty() ? -1 : qMin(index, int(swatches_.size()) - 1);
    updateGeometry();
    update();
    emit countChanged();
    emit currentChanged(current_);
    return true;
}

QString SwatchGrid::nextSwatchName() const
{
    const QString prefix = tr("Swatch ");
    int next = 1;
    for (const Swatch& swatch : swatches_) {
        if (!swatch.name.startsWith(prefix)) {
            continue;
        }
        bool ok = false;
        const int number = swatch.name.mid(prefix.size()).toInt(&ok);
        if (ok) {
            next = qMax(next, number + 1);
        }
    }
    return tr("Swatch %1").arg(next);
}

int SwatchGrid::columnsForWidth(int width) const
{
    const int usable = width - kMargin * 2 + kGap;
    return qMax(1, usable / (kCell + kGap));
}

int SwatchGrid::columns() const { return columnsForWidth(width()); }

int SwatchGrid::heightFor(int width) const
{
    const int across = columnsForWidth(width);
    const int rows = (int(swatches_.size()) + across - 1) / across;
    return kMargin * 2 + rows * (kCell + kGap);
}

QSize SwatchGrid::sizeHint() const
{
    return QSize(kMargin * 2 + kCell * 8 + kGap * 7, heightFor(width()));
}

QSize SwatchGrid::minimumSizeHint() const
{
    return QSize(kMargin * 2 + kCell, heightFor(width()));
}

QRect SwatchGrid::cellRect(int index) const
{
    const int across = columns();
    const int column = index % across;
    const int row = index / across;
    return QRect(kMargin + column * (kCell + kGap), kMargin + row * (kCell + kGap), kCell, kCell);
}

int SwatchGrid::indexAt(const QPoint& pos) const
{
    if (swatches_.isEmpty()) {
        return -1;
    }
    const int across = columns();
    const int column = (pos.x() - kMargin) / (kCell + kGap);
    const int row = (pos.y() - kMargin) / (kCell + kGap);
    if (pos.x() < kMargin || pos.y() < kMargin || column >= across) {
        return -1;
    }
    const int index = row * across + column;
    if (index < 0 || index >= swatches_.size()) {
        return -1;
    }
    // Inside the square, not the gap past it; treating the gap as a hit makes
    // the grid's edges feel sloppy.
    return cellRect(index).contains(pos) ? index : -1;
}

void SwatchGrid::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    updateGeometry();
}

void SwatchGrid::paintEvent(QPaintEvent* event)
{
    Q_UNUSED(event)
    QPainter painter(this);
    for (int i = 0; i < swatches_.size(); ++i) {
        const QRect cell = cellRect(i);
        painter.fillRect(cell, swatches_.at(i).color);
        painter.setPen(QPen(QColor(0, 0, 0, 90), 1));
        painter.drawRect(cell.adjusted(0, 0, -1, -1));
    }
    if (current_ >= 0 && current_ < swatches_.size()) {
        const QRect cell = cellRect(current_);
        painter.setPen(QPen(kMarkOuter, 1));
        painter.drawRect(cell.adjusted(0, 0, -1, -1));
        painter.setPen(QPen(kMarkInner, 1));
        painter.drawRect(cell.adjusted(1, 1, -2, -2));
    }
}

void SwatchGrid::mousePressEvent(QMouseEvent* event)
{
    if (event->button() != Qt::LeftButton) {
        QWidget::mousePressEvent(event);
        return;
    }
    const int index = indexAt(event->pos());
    if (index < 0) {
        // CS6 fills the empty space past the last swatch with a paint bucket.
        emit newSwatchRequested();
        return;
    }
    if (event->modifiers() & Qt::AltModifier) {
        removeSwatch(index);
        return;
    }
    current_ = index;
    update();
    emit currentChanged(current_);
    if (event->modifiers() & Qt::ControlModifier) {
        emit backgroundPicked(swatches_.at(index).color);
    } else {
        emit foregroundPicked(swatches_.at(index).color);
    }
}

bool SwatchGrid::event(QEvent* event)
{
    if (event->type() == QEvent::ToolTip) {
        auto* help = static_cast<QHelpEvent*>(event);
        const int index = indexAt(help->pos());
        if (index >= 0) {
            QToolTip::showText(help->globalPos(), swatches_.at(index).name, this);
        } else {
            QToolTip::hideText();
            event->ignore();
        }
        return true;
    }
    return QWidget::event(event);
}

SwatchesPanel::SwatchesPanel(ColorState* state, QWidget* parent)
    : QWidget(parent)
    , state_(state)
{
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    grid_ = new SwatchGrid(this);
    auto* scroll = new QScrollArea(this);
    scroll->setObjectName(QStringLiteral("swatchScroll"));
    scroll->setWidget(grid_);
    scroll->setWidgetResizable(true);
    scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    scroll->setFrameShape(QFrame::NoFrame);
    scroll->setAlignment(Qt::AlignTop | Qt::AlignLeft);
    root->addWidget(scroll, 1);

    auto* footer = new QWidget(this);
    footer->setObjectName(QStringLiteral("panelFooter"));
    auto* footerLayout = new QHBoxLayout(footer);
    footerLayout->setContentsMargins(4, 2, 4, 2);
    footerLayout->setSpacing(2);
    footerLayout->addStretch(1);

    auto makeButton = [&](const QString& glyph, const QString& tip, const QString& name) {
        auto* button = new QToolButton(footer);
        button->setObjectName(name);
        button->setText(glyph);
        button->setToolTip(tip);
        button->setAutoRaise(true);
        footerLayout->addWidget(button);
        return button;
    };
    auto* newButton = makeButton(QStringLiteral("+"),
                                 tr("Create a new swatch from the foreground color"),
                                 QStringLiteral("swatchesNew"));
    deleteButton_ = makeButton(QStringLiteral("\u2212"), tr("Delete the marked swatch"),
                               QStringLiteral("swatchesDelete"));
    root->addWidget(footer);

    connect(newButton, &QToolButton::clicked, this, &SwatchesPanel::addSwatchFromForeground);
    connect(deleteButton_, &QToolButton::clicked, this, &SwatchesPanel::deleteCurrentSwatch);
    connect(grid_, &SwatchGrid::newSwatchRequested, this,
            &SwatchesPanel::addSwatchFromForeground);
    connect(grid_, &SwatchGrid::currentChanged, this,
            [this](int index) { deleteButton_->setEnabled(index >= 0); });
    deleteButton_->setEnabled(grid_->current() >= 0);

    if (state_) {
        connect(grid_, &SwatchGrid::foregroundPicked, state_, &ColorState::setForeground);
        connect(grid_, &SwatchGrid::backgroundPicked, state_, &ColorState::setBackground);
    }

    setContextMenuPolicy(Qt::CustomContextMenu);
    connect(this, &QWidget::customContextMenuRequested, this, &SwatchesPanel::showContextMenu);
}

void SwatchesPanel::addSwatchFromForeground()
{
    const QColor color = state_ ? state_->foreground() : QColor(Qt::black);
    grid_->addSwatch({color, grid_->nextSwatchName()});
}

void SwatchesPanel::deleteCurrentSwatch() { grid_->removeSwatch(grid_->current()); }

void SwatchesPanel::resetSwatches() { grid_->setSwatches(SwatchGrid::defaultSwatches()); }

void SwatchesPanel::showContextMenu(const QPoint& at)
{
    QMenu menu(this);
    QAction* add = menu.addAction(tr("New Swatch"));
    connect(add, &QAction::triggered, this, &SwatchesPanel::addSwatchFromForeground);
    QAction* remove = menu.addAction(tr("Delete Swatch"));
    remove->setEnabled(grid_->current() >= 0);
    connect(remove, &QAction::triggered, this, &SwatchesPanel::deleteCurrentSwatch);
    menu.addSeparator();
    QAction* reset = menu.addAction(tr("Reset Swatches"));
    connect(reset, &QAction::triggered, this, &SwatchesPanel::resetSwatches);
    menu.exec(mapToGlobal(at));
}

} // namespace pictura
