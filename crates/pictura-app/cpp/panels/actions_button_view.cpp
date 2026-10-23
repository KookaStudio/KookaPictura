#include "actions_button_view.h"

#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtWidgets/QStyle>
#include <QtWidgets/QStyledItemDelegate>

namespace pictura {

namespace {

QColor textColorFor(const QColor& background)
{
    const int luminance = (background.red() * 299 + background.green() * 587
                           + background.blue() * 114)
                          / 1000;
    return luminance > 150 ? QColor(Qt::black) : QColor(Qt::white);
}

class ActionButtonDelegate : public QStyledItemDelegate {
public:
    using QStyledItemDelegate::QStyledItemDelegate;

    void paint(QPainter* painter, const QStyleOptionViewItem& option,
               const QModelIndex& index) const override
    {
        painter->save();
        painter->setRenderHint(QPainter::Antialiasing);
        QColor color = index.data(Qt::BackgroundRole).value<QColor>();
        if (!color.isValid()) {
            color = option.palette.color(QPalette::Button);
        }
        const QRectF rect = option.rect.adjusted(3, 3, -3, -3);
        QPainterPath path;
        path.addRoundedRect(rect, 4, 4);
        painter->fillPath(path, color);
        if (option.state & QStyle::State_Selected) {
            painter->setPen(QPen(option.palette.color(QPalette::Highlight), 2));
            painter->drawPath(path);
        }
        painter->setPen(textColorFor(color));
        QFont font = option.font;
        font.setBold(true);
        painter->setFont(font);
        const QString text = index.data(Qt::DisplayRole).toString();
        painter->drawText(rect.adjusted(5, 3, -5, -3),
                          Qt::AlignHCenter | Qt::AlignVCenter | Qt::TextWordWrap, text);
        painter->restore();
    }

    QSize sizeHint(const QStyleOptionViewItem& option,
                   const QModelIndex& index) const override
    {
        QSize hint = QStyledItemDelegate::sizeHint(option, index);
        hint = hint.expandedTo(QSize(80, 44));
        return hint;
    }
};

} // namespace

ActionsButtonModel::ActionsButtonModel(QObject* parent)
    : QAbstractListModel(parent)
{
}

void ActionsButtonModel::setActions(const QList<ActionNodeRow>& actions)
{
    beginResetModel();
    actions_ = actions;
    endResetModel();
}

int ActionsButtonModel::rowCount(const QModelIndex& parent) const
{
    return parent.isValid() ? 0 : actions_.size();
}

QVariant ActionsButtonModel::data(const QModelIndex& index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= actions_.size()) {
        return QVariant();
    }
    const ActionNodeRow& action = actions_.at(index.row());
    if (role == Qt::DisplayRole) {
        if (action.functionKey != 0) {
            return action.name + QLatin1Char('\n')
                   + actionFunctionKeyLabel(action.functionKey, action.functionShift,
                                            action.functionCtrl);
        }
        return action.name;
    }
    if (role == Qt::ToolTipRole) {
        return action.name;
    }
    if (role == Qt::BackgroundRole) {
        return actionColor(action.color);
    }
    if (role == ActionNodeIdRole) {
        return action.id;
    }
    if (role == ActionColorRole) {
        return action.color;
    }
    if (role == Qt::AccessibleTextRole) {
        return action.name;
    }
    return QVariant();
}

Qt::ItemFlags ActionsButtonModel::flags(const QModelIndex& index) const
{
    if (!index.isValid()) {
        return Qt::NoItemFlags;
    }
    return Qt::ItemIsEnabled | Qt::ItemIsSelectable;
}

QString ActionsButtonModel::idAt(int row) const
{
    return row >= 0 && row < actions_.size() ? actions_.at(row).id : QString();
}

int ActionsButtonModel::rowForId(const QString& id) const
{
    for (int i = 0; i < actions_.size(); ++i) {
        if (actions_.at(i).id == id) {
            return i;
        }
    }
    return -1;
}

ActionsButtonView::ActionsButtonView(QWidget* parent)
    : QListView(parent)
{
    model_ = new ActionsButtonModel(this);
    setModel(model_);
    setViewMode(QListView::IconMode);
    setMovement(QListView::Static);
    setResizeMode(QListView::Adjust);
    setWrapping(true);
    setUniformItemSizes(true);
    setGridSize(QSize(90, 54));
    setSpacing(4);
    setSelectionMode(QAbstractItemView::SingleSelection);
    setEditTriggers(QAbstractItemView::NoEditTriggers);
    setItemDelegate(new ActionButtonDelegate(this));
    setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    connect(this, &QListView::clicked, this,
            [this](const QModelIndex& index) { activateRow(index.row()); });
}

void ActionsButtonView::setActions(const QList<ActionNodeRow>& actions)
{
    QList<ActionNodeRow> actionsOnly;
    for (const ActionNodeRow& row : actions) {
        if (row.kind == actionKindAction()) {
            actionsOnly << row;
        }
    }
    model_->setActions(actionsOnly);
}

QStringList ActionsButtonView::actionIdsForTest() const
{
    QStringList ids;
    for (const ActionNodeRow& action : model_->actions()) {
        ids << action.id;
    }
    return ids;
}

QStringList ActionsButtonView::actionNamesForTest() const
{
    QStringList names;
    for (const ActionNodeRow& action : model_->actions()) {
        names << action.name;
    }
    return names;
}

QList<QColor> ActionsButtonView::actionColorsForTest() const
{
    QList<QColor> colors;
    for (const ActionNodeRow& action : model_->actions()) {
        colors << actionColor(action.color);
    }
    return colors;
}

void ActionsButtonView::activateActionForTest(int row)
{
    activateRow(row);
}

void ActionsButtonView::activateRow(int row)
{
    const QString id = model_->idAt(row);
    if (!id.isEmpty()) {
        emit actionActivated(id);
    }
}

} // namespace pictura

#include "moc_actions_button_view.cpp"
