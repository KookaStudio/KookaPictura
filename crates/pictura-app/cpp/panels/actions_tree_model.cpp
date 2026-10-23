#include "actions_tree_model.h"

#include <QtCore/QDataStream>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonValue>
#include <QtCore/QMimeData>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QKeySequence>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtGui/QPixmap>
#include <QtWidgets/QApplication>

namespace pictura {

namespace {

const QString kMimeType = QStringLiteral("application/x-pictura-actions");
const QChar kSegmentSeparator(0x1f);
const QChar kOrdinalSeparator(0x1e);

QString valueText(const QJsonValue& value)
{
    if (value.isString()) {
        return value.toString();
    }
    if (value.isBool()) {
        return value.toBool() ? QStringLiteral("true") : QStringLiteral("false");
    }
    if (value.isDouble()) {
        return QString::number(value.toDouble());
    }
    if (value.isNull()) {
        return QStringLiteral("null");
    }
    return QString::fromUtf8(QJsonDocument(QJsonArray{value}).toJson(QJsonDocument::Compact));
}

QString shortJson(const QJsonObject& params)
{
    QStringList parts;
    for (auto it = params.constBegin(); it != params.constEnd(); ++it) {
        parts << it.key() + QStringLiteral(": ") + valueText(it.value());
    }
    return parts.join(QStringLiteral(", "));
}

} // namespace

QColor actionColor(int color)
{
    switch (color) {
    case 1:
        return QColor(QStringLiteral("#c0392b"));
    case 2:
        return QColor(QStringLiteral("#d35400"));
    case 3:
        return QColor(QStringLiteral("#f1c40f"));
    case 4:
        return QColor(QStringLiteral("#27ae60"));
    case 5:
        return QColor(QStringLiteral("#2980b9"));
    case 6:
        return QColor(QStringLiteral("#8e44ad"));
    case 7:
        return QColor(QStringLiteral("#7f8c8d"));
    default:
        return QColor(QStringLiteral("#3c3c3c"));
    }
}

QString actionColorName(int color)
{
    static const QStringList names = {
        QStringLiteral("None"),   QStringLiteral("Red"),    QStringLiteral("Orange"),
        QStringLiteral("Yellow"), QStringLiteral("Green"),  QStringLiteral("Blue"),
        QStringLiteral("Violet"), QStringLiteral("Gray"),
    };
    return color >= 0 && color < names.size() ? names.at(color) : names.first();
}

QStringList actionColorNames()
{
    return {QStringLiteral("None"),   QStringLiteral("Red"),    QStringLiteral("Orange"),
            QStringLiteral("Yellow"), QStringLiteral("Green"),  QStringLiteral("Blue"),
            QStringLiteral("Violet"), QStringLiteral("Gray")};
}

QIcon actionCheckIcon(int state)
{
    static QIcon checked;
    static QIcon partial;
    auto make = [](const QColor& color) {
        QPixmap pixmap(14, 14);
        pixmap.fill(Qt::transparent);
        QPainter painter(&pixmap);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setPen(QPen(color, 2.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        painter.drawLine(QPointF(3, 7), QPointF(6, 10));
        painter.drawLine(QPointF(6, 10), QPointF(11, 4));
        return QIcon(pixmap);
    };
    if (checked.isNull()) {
        checked = make(QColor(QStringLiteral("#2ecc71")));
    }
    if (partial.isNull()) {
        partial = make(QColor(QStringLiteral("#e74c3c")));
    }
    if (state == Qt::PartiallyChecked) {
        return partial;
    }
    if (state == Qt::Checked) {
        return checked;
    }
    return QIcon();
}

QIcon actionModalIcon(int state)
{
    static QIcon on;
    static QIcon partial;
    auto make = [](const QColor& color) {
        QPixmap pixmap(14, 14);
        pixmap.fill(Qt::transparent);
        QPainter painter(&pixmap);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setPen(QPen(color, 1.4));
        painter.drawRoundedRect(QRectF(2.5, 3.5, 9, 7), 1.5, 1.5);
        painter.drawLine(QPointF(2.5, 6), QPointF(11.5, 6));
        painter.setBrush(color);
        painter.drawEllipse(QPointF(5, 8.4), 0.8, 0.8);
        painter.drawEllipse(QPointF(7, 8.4), 0.8, 0.8);
        painter.drawEllipse(QPointF(9, 8.4), 0.8, 0.8);
        return QIcon(pixmap);
    };
    if (on.isNull()) {
        on = make(QColor(QStringLiteral("#bdc3c7")));
    }
    if (partial.isNull()) {
        partial = make(QColor(QStringLiteral("#e74c3c")));
    }
    if (state == Qt::PartiallyChecked) {
        return partial;
    }
    if (state == Qt::Checked) {
        return on;
    }
    return QIcon();
}

QString actionFunctionKeyLabel(int key, bool shift, bool ctrl)
{
    if (key == 0) {
        return QString();
    }
    int sequence = key;
    if (shift) {
        sequence |= Qt::ShiftModifier;
    }
    if (ctrl) {
        sequence |= Qt::ControlModifier;
    }
    return QKeySequence(sequence).toString();
}

ActionsEngineApi::ActionsEngineApi(QObject* parent)
    : QObject(parent)
{
}

ActionsEngineApi::~ActionsEngineApi() = default;

ActionsTreeModel::ActionsTreeModel(QObject* parent)
    : QAbstractItemModel(parent)
{
}

ActionsTreeModel::~ActionsTreeModel() = default;

void ActionsTreeModel::setRows(const QList<ActionNodeRow>& rows)
{
    beginResetModel();
    nodes_.clear();
    roots_.clear();
    byId_.clear();

    QHash<QString, Node*> created;
    for (const ActionNodeRow& row : rows) {
        if (row.id.isEmpty() || row.kind == actionKindParam()) {
            continue;
        }
        auto node = std::make_unique<Node>();
        node->row = row;
        Node* raw = node.get();
        nodes_.push_back(std::move(node));
        created.insert(row.id, raw);
        byId_.insert(row.id, raw);
    }

    for (const auto& node : nodes_) {
        Node* parent = created.value(node->row.parentId, nullptr);
        if (!parent || parent == node.get()) {
            node->parent = nullptr;
            node->rowInParent = roots_.size();
            roots_.push_back(node.get());
        } else {
            node->parent = parent;
            node->rowInParent = parent->children.size();
            parent->children.push_back(node.get());
        }
    }

    const int baseCount = int(nodes_.size());
    for (int i = 0; i < baseCount; ++i) {
        Node* node = nodes_.at(size_t(i)).get();
        if (!node->children.isEmpty() || node->row.params.isEmpty()
            || node->row.kind != actionKindStep()) {
            continue;
        }
        int index = 0;
        for (auto it = node->row.params.constBegin(); it != node->row.params.constEnd();
             ++it, ++index) {
            auto param = std::make_unique<Node>();
            param->row.id = node->row.id + QStringLiteral("::param::") + it.key();
            param->row.parentId = node->row.id;
            param->row.kind = actionKindParam();
            param->row.name = it.key() + QStringLiteral(": ") + valueText(it.value());
            param->parent = node;
            param->rowInParent = index;
            param->param = true;
            Node* raw = param.get();
            nodes_.push_back(std::move(param));
            node->children.push_back(raw);
        }
    }
    endResetModel();
}

QModelIndex ActionsTreeModel::index(int row, int column, const QModelIndex& parentIndex) const
{
    if (column < 0 || column >= columnCount() || row < 0
        || (parentIndex.isValid() && parentIndex.column() != 0)) {
        return QModelIndex();
    }
    const Node* parentNode = nodeFor(parentIndex);
    const QList<Node*>& siblings = parentNode ? parentNode->children : roots_;
    if (row >= siblings.size()) {
        return QModelIndex();
    }
    return createIndex(row, column, siblings.at(row));
}

QModelIndex ActionsTreeModel::parent(const QModelIndex& child) const
{
    Node* node = nodeFor(child);
    if (!node || !node->parent) {
        return QModelIndex();
    }
    return indexForNode(node->parent, 0);
}

int ActionsTreeModel::rowCount(const QModelIndex& parentIndex) const
{
    if (parentIndex.isValid() && parentIndex.column() != 0) {
        return 0;
    }
    const Node* parentNode = nodeFor(parentIndex);
    return parentNode ? parentNode->children.size() : roots_.size();
}

int ActionsTreeModel::columnCount(const QModelIndex&) const
{
    return 3;
}

QVariant ActionsTreeModel::data(const QModelIndex& modelIndex, int role) const
{
    Node* node = nodeFor(modelIndex);
    if (!node) {
        return QVariant();
    }
    const int column = modelIndex.column();
    const int included = stateFor(node);
    const int modal = modalStateFor(node);
    const bool param = node->param;

    if (role == ActionNodeIdRole) {
        return node->row.id;
    }
    if (role == ActionNodeKindRole) {
        return node->row.kind;
    }
    if (role == ActionParentIdRole) {
        return node->row.parentId;
    }
    if (role == ActionIncludedStateRole) {
        return included;
    }
    if (role == ActionModalStateRole) {
        return modal;
    }
    if (role == ActionModalEditableRole) {
        return !param && modalEditableFor(node);
    }
    if (role == ActionInclusionEditableRole) {
        return !param;
    }
    if (role == ActionFunctionKeyRole) {
        return node->row.functionKey;
    }
    if (role == ActionStepCommandRole) {
        return node->row.commandId;
    }
    if (role == ActionStepParamsRole) {
        return QVariant::fromValue(node->row.params);
    }
    if (role == ActionColorRole) {
        return node->row.color;
    }

    if (column == 0) {
        if (param) {
            return QVariant();
        }
        if (role == Qt::CheckStateRole) {
            return included;
        }
        if (role == Qt::DecorationRole) {
            return actionCheckIcon(included);
        }
        if (role == Qt::ForegroundRole && included == Qt::PartiallyChecked) {
            return QColor(QStringLiteral("#e74c3c"));
        }
        if (role == Qt::ToolTipRole) {
            return included == Qt::Checked ? tr("Included")
                                           : (included == Qt::PartiallyChecked
                                                  ? tr("Partially included")
                                                  : tr("Excluded"));
        }
        if (role == Qt::AccessibleTextRole || role == Qt::AccessibleDescriptionRole) {
            return included == Qt::Checked
                ? tr("Included")
                : (included == Qt::PartiallyChecked ? tr("Partially included")
                                                    : tr("Excluded"));
        }
        return QVariant();
    }

    if (column == 1) {
        if (param) {
            return QVariant();
        }
        if (role == Qt::CheckStateRole) {
            return modal;
        }
        if (role == Qt::DecorationRole) {
            return actionModalIcon(modal);
        }
        if (role == Qt::ForegroundRole && modal == Qt::PartiallyChecked) {
            return QColor(QStringLiteral("#e74c3c"));
        }
        if (role == Qt::ToolTipRole) {
            if (!modalEditableFor(node)) {
                return modal == Qt::Checked ? tr("Dialog required")
                                            : (modal == Qt::PartiallyChecked
                                                   ? tr("Partially modal")
                                                   : tr("No dialog"));
            }
            return modal == Qt::Checked ? tr("Toggle dialog off")
                                        : (modal == Qt::PartiallyChecked
                                               ? tr("Partially modal")
                                               : tr("Toggle dialog on"));
        }
        if (role == Qt::AccessibleTextRole || role == Qt::AccessibleDescriptionRole) {
            return modal == Qt::Checked
                ? tr("Modal")
                : (modal == Qt::PartiallyChecked ? tr("Partially modal")
                                                 : tr("No dialog"));
        }
        return QVariant();
    }

    if (column != 2) {
        return QVariant();
    }

    if (role == Qt::DisplayRole) {
        if (node->row.kind == actionKindAction() && node->row.functionKey != 0) {
            return node->row.name + QStringLiteral("    ")
                   + actionFunctionKeyLabel(node->row.functionKey, node->row.functionShift,
                                            node->row.functionCtrl);
        }
        return node->row.name;
    }
    if (role == Qt::EditRole) {
        return node->row.name;
    }
    if (role == Qt::DecorationRole && node->row.kind == actionKindAction()) {
        QPixmap swatch(10, 10);
        swatch.fill(actionColor(node->row.color));
        return QIcon(swatch);
    }
    if (role == Qt::ToolTipRole) {
        if (node->row.kind == actionKindStep()) {
            QString detail = node->row.commandId;
            const QString params = shortJson(node->row.params);
            if (!params.isEmpty()) {
                detail += QStringLiteral(" (") + params + QStringLiteral(")");
            }
            return detail;
        }
        if (node->row.kind == actionKindAction() && node->row.functionKey != 0) {
            return actionFunctionKeyLabel(node->row.functionKey, node->row.functionShift,
                                          node->row.functionCtrl);
        }
        return node->row.name;
    }
    if (role == Qt::AccessibleTextRole) {
        return node->row.name;
    }
    return QVariant();
}

bool ActionsTreeModel::setData(const QModelIndex& modelIndex, const QVariant& value, int role)
{
    Node* node = nodeFor(modelIndex);
    if (!node || node->param) {
        return false;
    }
    if (modelIndex.column() == 0 && role == Qt::CheckStateRole) {
        const bool included = value.toInt() != Qt::Unchecked;
        setIncludedLocally(node, included);
        emit inclusionRequested(node->row.id, included);
        return true;
    }
    if (modelIndex.column() == 1 && role == Qt::CheckStateRole) {
        if (!modalEditableFor(node)) {
            return false;
        }
        const bool modal = value.toInt() == Qt::Checked;
        setModalLocally(node, modal);
        emit modalRequested(node->row.id, modal);
        return true;
    }
    if (modelIndex.column() == 2 && (role == Qt::EditRole || role == Qt::DisplayRole)) {
        if (!nodeEditableName(node)) {
            return false;
        }
        const QString name = value.toString().trimmed();
        if (name.isEmpty() || name == node->row.name) {
            return false;
        }
        node->row.name = name;
        emit dataChanged(modelIndex, modelIndex, {Qt::DisplayRole, Qt::EditRole});
        emit renameRequested(node->row.id, name);
        return true;
    }
    return false;
}

QVariant ActionsTreeModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (orientation != Qt::Horizontal || role != Qt::DisplayRole) {
        return QVariant();
    }
    if (section == 2) {
        return tr("Name");
    }
    return QString();
}

Qt::ItemFlags ActionsTreeModel::flags(const QModelIndex& modelIndex) const
{
    if (!modelIndex.isValid()) {
        return Qt::ItemIsDropEnabled;
    }
    Node* node = nodeFor(modelIndex);
    if (!node) {
        return Qt::NoItemFlags;
    }
    if (node->param) {
        return Qt::ItemIsEnabled | Qt::ItemIsSelectable;
    }
    Qt::ItemFlags out = Qt::ItemIsEnabled | Qt::ItemIsSelectable;
    if (modelIndex.column() == 0) {
        out |= Qt::ItemIsUserCheckable | Qt::ItemIsDragEnabled;
    } else if (modelIndex.column() == 1) {
        if (modalEditableFor(node)) {
            out |= Qt::ItemIsUserCheckable;
        }
    } else if (modelIndex.column() == 2) {
        if (nodeEditableName(node)) {
            out |= Qt::ItemIsEditable;
        }
    }
    const QString kind = node->row.kind;
    if (kind == actionKindSet() || kind == actionKindAction() || kind == actionKindStep()) {
        out |= Qt::ItemIsDragEnabled;
    }
    if (kind == actionKindSet() || kind == actionKindAction()) {
        out |= Qt::ItemIsDropEnabled;
    }
    return out;
}

QStringList ActionsTreeModel::mimeTypes() const
{
    return {kMimeType};
}

QMimeData* ActionsTreeModel::mimeData(const QModelIndexList& indexes) const
{
    QStringList ids;
    QSet<QString> seen;
    for (const QModelIndex& modelIndex : indexes) {
        Node* node = nodeFor(modelIndex);
        if (!node || node->param || node->row.kind == actionKindParam()) {
            continue;
        }
        bool ancestor = false;
        for (Node* p = node->parent; p; p = p->parent) {
            if (seen.contains(p->row.id)) {
                ancestor = true;
                break;
            }
        }
        if (ancestor || seen.contains(node->row.id)) {
            continue;
        }
        seen.insert(node->row.id);
        ids << node->row.id;
    }
    if (ids.isEmpty()) {
        return nullptr;
    }
    auto* mime = new QMimeData;
    QByteArray bytes;
    QDataStream stream(&bytes, QIODevice::WriteOnly);
    stream << ids;
    mime->setData(kMimeType, bytes);
    return mime;
}

bool ActionsTreeModel::canDropMimeData(const QMimeData* data, Qt::DropAction action, int row,
                                       int column, const QModelIndex& parentIndex) const
{
    Q_UNUSED(row)
    Q_UNUSED(column)
    if (!data || !data->hasFormat(kMimeType)
        || (action != Qt::MoveAction && action != Qt::CopyAction)) {
        return false;
    }
    const QStringList ids = selectedIdsFromMime(data);
    if (ids.isEmpty()) {
        return false;
    }
    Node* parentNode = nodeFor(parentIndex);
    if (!parentNode) {
        for (const QString& id : ids) {
            Node* node = nodeForId(id);
            if (!node || node->row.kind != actionKindSet()) {
                return false;
            }
        }
        return true;
    }
    if (parentNode->param || parentNode->row.kind == actionKindStep()
        || parentNode->row.kind == actionKindParam()) {
        return false;
    }
    for (const QString& id : ids) {
        Node* node = nodeForId(id);
        if (!node || node == parentNode) {
            return false;
        }
        for (Node* p = parentNode; p; p = p->parent) {
            if (p == node) {
                return false;
            }
        }
        if (parentNode->row.kind == actionKindSet()
            && node->row.kind != actionKindAction()) {
            return false;
        }
        if (parentNode->row.kind == actionKindAction()
            && node->row.kind != actionKindStep()) {
            return false;
        }
    }
    return true;
}

bool ActionsTreeModel::dropMimeData(const QMimeData* data, Qt::DropAction action, int row,
                                    int column, const QModelIndex& parentIndex)
{
    Q_UNUSED(column)
    if (!canDropMimeData(data, action, row, column, parentIndex)) {
        return false;
    }
    Node* parentNode = nodeFor(parentIndex);
    const QString parentId = parentNode ? parentNode->row.id : QString();
    const bool copy = action == Qt::CopyAction
                      || (QGuiApplication::keyboardModifiers() & Qt::AltModifier);
    const QStringList ids = selectedIdsFromMime(data);
    for (const QString& id : ids) {
        emit moveRequested(id, parentId, row, copy);
    }
    return true;
}

Qt::DropActions ActionsTreeModel::supportedDropActions() const
{
    return Qt::MoveAction | Qt::CopyAction;
}

Qt::DropActions ActionsTreeModel::supportedDragActions() const
{
    return Qt::MoveAction | Qt::CopyAction;
}

QString ActionsTreeModel::nodeId(const QModelIndex& modelIndex) const
{
    Node* node = nodeFor(modelIndex);
    return node ? node->row.id : QString();
}

QString ActionsTreeModel::nodeKind(const QString& id) const
{
    Node* node = nodeForId(id);
    return node ? node->row.kind : QString();
}

ActionNodeRow ActionsTreeModel::rowForId(const QString& id) const
{
    Node* node = nodeForId(id);
    return node ? node->row : ActionNodeRow{};
}

QModelIndex ActionsTreeModel::indexForId(const QString& id) const
{
    return indexForNode(nodeForId(id), 0);
}

QStringList ActionsTreeModel::actionIdsForSet(const QString& setId) const
{
    QStringList ids;
    Node* node = nodeForId(setId);
    if (!node) {
        return ids;
    }
    for (Node* child : node->children) {
        if (child->row.kind == actionKindAction()) {
            ids << child->row.id;
        }
    }
    return ids;
}

QStringList ActionsTreeModel::stepIdsForAction(const QString& actionId) const
{
    QStringList ids;
    Node* node = nodeForId(actionId);
    if (!node) {
        return ids;
    }
    for (Node* child : node->children) {
        if (child->row.kind == actionKindStep()) {
            ids << child->row.id;
        }
    }
    return ids;
}

QString ActionsTreeModel::identityForId(const QString& id) const
{
    Node* node = nodeForId(id);
    if (!node) {
        return QString();
    }
    QList<QPair<QString, int>> reversed;
    for (Node* current = node; current; current = current->parent) {
        if (current->param) {
            continue;
        }
        if (current->row.kind == actionKindStep()) {
            reversed.prepend({QStringLiteral("#") + QString::number(current->row.stepIndex >= 0
                                                                       ? current->row.stepIndex
                                                                       : current->rowInParent),
                              -1});
            continue;
        }
        if (current->row.name.isEmpty()) {
            return QString();
        }
        int ordinal = 0;
        const QList<Node*>& siblings =
            current->parent ? current->parent->children : roots_;
        for (Node* sibling : siblings) {
            if (sibling == current) {
                break;
            }
            if (!sibling->param && sibling->row.name == current->row.name) {
                ++ordinal;
            }
        }
        reversed.prepend({current->row.name, ordinal});
    }
    return encodeIdentity(reversed);
}

QString ActionsTreeModel::idForIdentity(const QString& identity) const
{
    if (identity.isEmpty()) {
        return QString();
    }
    const QStringList parts = identity.split(kSegmentSeparator);
    QList<Node*> level = roots_;
    Node* found = nullptr;
    for (const QString& part : parts) {
        if (part.startsWith(QLatin1Char('#'))) {
            const int stepIndex = part.mid(1).toInt();
            for (Node* node : level) {
                const int index = node->row.stepIndex >= 0 ? node->row.stepIndex
                                                           : node->rowInParent;
                if (node->row.kind == actionKindStep() && index == stepIndex) {
                    found = node;
                    break;
                }
            }
            if (!found) {
                return QString();
            }
            level = found->children;
            continue;
        }
        const int separator = part.lastIndexOf(kOrdinalSeparator);
        if (separator < 0) {
            return QString();
        }
        const QString name = part.left(separator);
        const int wanted = part.mid(separator + 1).toInt();
        int ordinal = 0;
        found = nullptr;
        for (Node* node : level) {
            if (node->param || node->row.name != name) {
                continue;
            }
            if (ordinal == wanted) {
                found = node;
                break;
            }
            ++ordinal;
        }
        if (!found) {
            return QString();
        }
        level = found->children;
    }
    return found ? found->row.id : QString();
}

QStringList ActionsTreeModel::identitiesForIds(const QStringList& ids) const
{
    QStringList out;
    for (const QString& id : ids) {
        const QString identity = identityForId(id);
        if (!identity.isEmpty() && !out.contains(identity)) {
            out << identity;
        }
    }
    return out;
}

QStringList ActionsTreeModel::idsForIdentities(const QStringList& identities) const
{
    QStringList out;
    for (const QString& identity : identities) {
        const QString id = idForIdentity(identity);
        if (!id.isEmpty() && !out.contains(id)) {
            out << id;
        }
    }
    return out;
}

int ActionsTreeModel::inclusionState(const QModelIndex& modelIndex) const
{
    return stateFor(nodeFor(modelIndex));
}

int ActionsTreeModel::modalState(const QModelIndex& modelIndex) const
{
    return modalStateFor(nodeFor(modelIndex));
}

bool ActionsTreeModel::modalEditable(const QModelIndex& modelIndex) const
{
    Node* node = nodeFor(modelIndex);
    return node && !node->param && modalEditableFor(node);
}

void ActionsTreeModel::requestInclusion(const QModelIndex& modelIndex, bool included)
{
    setData(modelIndex.siblingAtColumn(0), included ? Qt::Checked : Qt::Unchecked,
            Qt::CheckStateRole);
}

void ActionsTreeModel::toggleSiblingsIncluded(const QModelIndex& modelIndex)
{
    Node* node = nodeFor(modelIndex);
    if (!node || node->param || node->row.kind != actionKindStep() || !node->parent) {
        return;
    }
    emit siblingInclusionRequested(node->row.id);
    const bool included = node->row.enabled;
    for (Node* sibling : node->parent->children) {
        if (sibling->row.kind == actionKindStep() && sibling != node) {
            sibling->row.enabled = !included;
            const QModelIndex index = indexForNode(sibling, 0);
            emit dataChanged(index, index, {Qt::CheckStateRole, Qt::DecorationRole});
        }
    }
    notifyAncestors(node->parent);
}

void ActionsTreeModel::requestModal(const QModelIndex& modelIndex, bool modal)
{
    setData(modelIndex.siblingAtColumn(1), modal ? Qt::Checked : Qt::Unchecked,
            Qt::CheckStateRole);
}

ActionsTreeModel::Node* ActionsTreeModel::nodeFor(const QModelIndex& modelIndex) const
{
    if (!modelIndex.isValid()) {
        return nullptr;
    }
    return static_cast<Node*>(modelIndex.internalPointer());
}

ActionsTreeModel::Node* ActionsTreeModel::nodeForId(const QString& id) const
{
    return byId_.value(id, nullptr);
}

QModelIndex ActionsTreeModel::indexForNode(const Node* node, int column) const
{
    if (!node) {
        return QModelIndex();
    }
    return createIndex(node->rowInParent, column, const_cast<Node*>(node));
}

int ActionsTreeModel::stateFor(const Node* node) const
{
    if (!node || node->param) {
        return Qt::Unchecked;
    }
    int count = 0;
    int checked = 0;
    for (const Node* child : node->children) {
        if (child->param || child->row.kind == actionKindParam()) {
            continue;
        }
        ++count;
        if (stateFor(child) == Qt::Checked) {
            ++checked;
        } else if (stateFor(child) == Qt::PartiallyChecked) {
            checked = -100;
            break;
        }
    }
    if (count == 0 || checked < 0) {
        return count == 0 ? (node->row.enabled ? Qt::Checked : Qt::Unchecked)
                          : Qt::PartiallyChecked;
    }
    if (checked == count) {
        return Qt::Checked;
    }
    return checked == 0 ? Qt::Unchecked : Qt::PartiallyChecked;
}

int ActionsTreeModel::modalStateFor(const Node* node) const
{
    if (!node || node->param) {
        return Qt::Unchecked;
    }
    if (node->row.kind == actionKindStep()) {
        if (node->row.dialogOptions == 1) {
            return Qt::Checked;
        }
        if (node->row.dialogOptions == 2 || node->row.dialogOptions == 3) {
            return Qt::Unchecked;
        }
        return node->row.modal ? Qt::Checked : Qt::Unchecked;
    }
    int count = 0;
    int checked = 0;
    for (const Node* child : node->children) {
        if (child->param || child->row.kind == actionKindParam()) {
            continue;
        }
        const int state = modalStateFor(child);
        if (state == Qt::PartiallyChecked) {
            return Qt::PartiallyChecked;
        }
        ++count;
        if (state == Qt::Checked) {
            ++checked;
        }
    }
    if (count == 0) {
        return Qt::Unchecked;
    }
    if (checked == count) {
        return Qt::Checked;
    }
    return checked == 0 ? Qt::Unchecked : Qt::PartiallyChecked;
}

bool ActionsTreeModel::modalEditableFor(const Node* node) const
{
    if (!node || node->param) {
        return false;
    }
    if (node->row.kind == actionKindStep()) {
        return node->row.dialogOptions == 0 || node->row.dialogOptions == 4;
    }
    bool hasChild = false;
    for (const Node* child : node->children) {
        if (child->param || child->row.kind == actionKindParam()) {
            continue;
        }
        hasChild = true;
        if (!modalEditableFor(child)) {
            return false;
        }
    }
    return hasChild;
}

bool ActionsTreeModel::nodeEditableName(const Node* node) const
{
    return node && !node->param
           && (node->row.kind == actionKindSet() || node->row.kind == actionKindAction());
}

void ActionsTreeModel::notifyAncestors(Node* node)
{
    for (Node* parent = node ? node->parent : nullptr; parent; parent = parent->parent) {
        const QModelIndex check = indexForNode(parent, 0);
        const QModelIndex modal = indexForNode(parent, 1);
        emit dataChanged(check, check,
                         {Qt::CheckStateRole, Qt::DecorationRole, Qt::ForegroundRole});
        emit dataChanged(modal, modal,
                         {Qt::CheckStateRole, Qt::DecorationRole, Qt::ForegroundRole});
    }
}

void ActionsTreeModel::setIncludedLocally(Node* node, bool included)
{
    if (!node) {
        return;
    }
    node->row.enabled = included;
    if (node->row.kind != actionKindStep()) {
        for (Node* child : node->children) {
            if (child->param) {
                continue;
            }
            setIncludedLocally(child, included);
        }
    }
    const QModelIndex check = indexForNode(node, 0);
    emit dataChanged(check, check,
                     {Qt::CheckStateRole, Qt::DecorationRole, Qt::ForegroundRole});
    notifyAncestors(node);
}

void ActionsTreeModel::setModalLocally(Node* node, bool modal)
{
    if (!node) {
        return;
    }
    if (node->row.kind == actionKindStep()) {
        node->row.modal = modal;
    } else {
        for (Node* child : node->children) {
            if (child->param || !modalEditableFor(child)) {
                continue;
            }
            setModalLocally(child, modal);
        }
    }
    const QModelIndex modalIndex = indexForNode(node, 1);
    emit dataChanged(modalIndex, modalIndex,
                     {Qt::CheckStateRole, Qt::DecorationRole, Qt::ForegroundRole});
    notifyAncestors(node);
}

QStringList ActionsTreeModel::selectedIdsFromMime(const QMimeData* data) const
{
    if (!data || !data->hasFormat(kMimeType)) {
        return {};
    }
    QByteArray bytes = data->data(kMimeType);
    QDataStream stream(&bytes, QIODevice::ReadOnly);
    QStringList ids;
    stream >> ids;
    return ids;
}

QString ActionsTreeModel::encodeIdentity(const QList<QPair<QString, int>>& segments)
{
    QStringList parts;
    for (const auto& segment : segments) {
        if (segment.second < 0) {
            parts << segment.first;
        } else {
            parts << segment.first + kOrdinalSeparator + QString::number(segment.second);
        }
    }
    return parts.join(kSegmentSeparator);
}

ActionsTreeView::ActionsTreeView(QWidget* parent)
    : QTreeView(parent)
{
    setExpandsOnDoubleClick(false);
    setAllColumnsShowFocus(true);
    setUniformRowHeights(true);
    setSelectionMode(QAbstractItemView::ExtendedSelection);
    setEditTriggers(QAbstractItemView::DoubleClicked | QAbstractItemView::EditKeyPressed);
    setDragEnabled(true);
    setAcceptDrops(true);
    setDropIndicatorShown(true);
    setDragDropMode(QAbstractItemView::DragDrop);
    setDefaultDropAction(Qt::MoveAction);
    setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
}

void ActionsTreeView::mousePressEvent(QMouseEvent* event)
{
    const QModelIndex index = indexAt(event->position().toPoint());
    const bool alt = event->modifiers() & Qt::AltModifier;
    if (index.isValid() && alt) {
        if (index.column() == 0) {
            emit siblingInclusionToggleRequested(index);
            event->accept();
            return;
        }
        if (index.column() == treePosition() && atDisclosure(index, event->position().toPoint())) {
            const QModelIndex rowIndex = index.siblingAtColumn(0);
            if (isExpanded(rowIndex)) {
                collapse(rowIndex);
            } else {
                expandRecursively(rowIndex);
            }
            event->accept();
            return;
        }
    }
    QTreeView::mousePressEvent(event);
}

void ActionsTreeView::keyPressEvent(QKeyEvent* event)
{
    if (event->key() == Qt::Key_Escape) {
        emit escapePressed();
        event->accept();
        return;
    }
    if (event->key() == Qt::Key_Space) {
        const QModelIndex index = currentIndex();
        if (index.isValid()) {
            if (event->modifiers() & Qt::ControlModifier) {
                emit modalToggleRequested(index);
            } else {
                emit inclusionToggleRequested(index);
            }
            event->accept();
            return;
        }
    }
    QTreeView::keyPressEvent(event);
}

bool ActionsTreeView::atDisclosure(const QModelIndex& index, const QPoint& pos) const
{
    if (index.column() != treePosition()) {
        return false;
    }
    int depth = 0;
    for (QModelIndex current = index; current.isValid(); current = current.parent()) {
        ++depth;
    }
    const int limit = indentation() * qMax(0, depth - 1) + 18;
    return pos.x() - visualRect(index).left() < limit;
}

} // namespace pictura

#include "moc_actions_tree_model.cpp"
