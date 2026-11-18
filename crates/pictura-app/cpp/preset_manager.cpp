#include "preset_manager.h"

#include "dialogs.h"
#include "gradient_editor_dialog.h"
#include "api/paint_bridge.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <QtCore/QAbstractListModel>
#include <QtCore/QCoreApplication>
#include <QtCore/QDataStream>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QMimeData>
#include <QtCore/QPointer>
#include <QtCore/QSet>
#include <QtCore/QSignalBlocker>
#include <QtCore/QThreadPool>
#include <QtCore/QTimer>
#include <QtGui/QIcon>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QPolygon>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListView>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cstdint>

namespace pictura {
namespace {

constexpr auto dragType = "application/x-pictura-preset-rows";

void failure(QWidget* parent, const QString& message)
{
    QMessageBox::warning(parent, QObject::tr("Preset Manager"),
                         message.isEmpty() ? QObject::tr("The preset library operation failed.") : message);
}

QIcon previewIcon(PresetType type, const PresetRow& row, bool stroke)
{
    constexpr int edge = 36;
    const int builtin = row.values.value(QStringLiteral("index")).toInt(-1);
    QImage preview = row.preview;
    QPixmap icon(edge, edge);
    icon.fill(Qt::transparent);
    QPainter painter(&icon);
    if (row.color.isValid()) {
        painter.fillRect(icon.rect(), row.color);
    } else if (type == PresetType::Gradients) {
        paintGradient(painter, icon.rect(), libraryRamp(row, Qt::black, Qt::white));
    } else if (type == PresetType::Contours && row.values.contains(QStringLiteral("points"))) {
        painter.setPen(QPen(QColor(120, 120, 120), 2));
        QPolygonF contour;
        for (const auto& pair : row.values.value(QStringLiteral("points")).toString().split(QLatin1Char(' '))) {
            const auto point = pair.split(QLatin1Char(','));
            if (point.size() == 2) {
                contour << QPointF(point[0].toInt() * (edge - 4) / 255.0 + 2,
                                   edge - 2 - point[1].toInt() * (edge - 4) / 255.0);
            }
        }
        painter.drawPolyline(contour);
    } else {
        if (preview.isNull() && type == PresetType::CustomShapes && builtin >= 0) {
            preview = shape_custom_preview(builtin, edge);
        }
        if (preview.isNull() && type == PresetType::Patterns && builtin >= 0) {
            const int side = stamp_pattern_side();
            const auto tile = stamp_pattern_tile(builtin);
            if (tile.size() == size_t(side * side * 4)) {
                preview = QImage(tile.data(), side, side, QImage::Format_RGBA8888).copy();
            }
        }
        if (!preview.isNull()) {
            if (stroke && type == PresetType::Brushes) {
                for (int x = 0; x < edge - 8; x += 4) {
                    painter.drawImage(QRect(x, (x - 18) * (x - 18) / 32 + 9, 12, 12), preview);
                }
            } else {
                painter.drawImage(icon.rect(), preview);
            }
        } else {
            painter.setPen(QColor(120, 120, 120));
            painter.drawText(icon.rect(), Qt::AlignCenter, row.name.left(2));
        }
    }
    painter.end();
    return QIcon(icon);
}

} // namespace

class PresetListModel : public QAbstractListModel {
public:
    explicit PresetListModel(PresetStore& store, QObject* parent)
        : QAbstractListModel(parent), store_(store)
    {
        setObjectName(QStringLiteral("presetListModel"));
        connect(&store, &PresetStore::changed, this, [this](int type) {
            if (type == int(type_)) {
                refresh();
            }
        });
    }

    int rowCount(const QModelIndex& parent = {}) const override
    {
        return parent.isValid() ? 0 : int(store_.rows(type_).size());
    }

    QVariant data(const QModelIndex& index, int role) const override
    {
        if (!index.isValid() || index.row() >= rowCount()) {
            return {};
        }
        const auto& row = store_.rows(type_).at(index.row());
        switch (role) {
        case Qt::DisplayRole: return labels_ ? row.name : QString();
        case Qt::EditRole:
        case Qt::ToolTipRole: return row.name;
        case Qt::UserRole: return QVariant::fromValue(row.id);
        case Qt::DecorationRole:
            return decoration_ ? QVariant::fromValue(previewIcon(type_, row, stroke_)) : QVariant();
        default: return {};
        }
    }

    bool setData(const QModelIndex& index, const QVariant& value, int role) override
    {
        if (!index.isValid() || index.row() >= rowCount() || role != Qt::EditRole || value.toString().isEmpty()) {
            return false;
        }
        auto rows = store_.rows(type_);
        rows[index.row()].name = value.toString();
        return saveRows(std::move(rows));
    }

    Qt::ItemFlags flags(const QModelIndex& index) const override
    {
        return index.isValid() ? QAbstractListModel::flags(index) | Qt::ItemIsEditable | Qt::ItemIsDragEnabled
                               : Qt::ItemIsDropEnabled;
    }

    Qt::DropActions supportedDropActions() const override { return Qt::MoveAction; }
    QStringList mimeTypes() const override { return {QString::fromLatin1(dragType)}; }

    QMimeData* mimeData(const QModelIndexList& indexes) const override
    {
        auto* mime = new QMimeData;
        QByteArray data;
        QDataStream stream(&data, QIODevice::WriteOnly);
        stream << quint64(reinterpret_cast<quintptr>(this));
        for (const auto& index : indexes) {
            stream << index.data(Qt::UserRole).toULongLong();
        }
        mime->setData(QString::fromLatin1(dragType), data);
        return mime;
    }

    bool dropMimeData(const QMimeData* mime, Qt::DropAction action, int row, int column,
                      const QModelIndex& parent) override
    {
        if (action != Qt::MoveAction || column > 0 || !mime->hasFormat(QString::fromLatin1(dragType))) {
            return false;
        }
        QDataStream stream(mime->data(QString::fromLatin1(dragType)));
        quint64 owner = 0;
        stream >> owner;
        if (owner != quint64(reinterpret_cast<quintptr>(this))) {
            return false;
        }
        QSet<quint64> ids;
        while (!stream.atEnd()) {
            quint64 id = 0;
            stream >> id;
            ids.insert(id);
        }
        return stream.status() == QDataStream::Ok
            && moveIds(ids, row < 0 ? (parent.isValid() ? parent.row() : rowCount()) : row);
    }

    bool moveRows(const QModelIndex& source, int row, int count,
                  const QModelIndex& dest, int destination) override
    {
        if (source.isValid() || dest.isValid() || row < 0 || count < 1 || count > rowCount() - row
            || destination < 0 || destination > rowCount()) {
            return false;
        }
        QSet<quint64> ids;
        for (int i = 0; i < count; ++i) {
            ids.insert(store_.rows(type_).at(row + i).id);
        }
        return moveIds(ids, destination);
    }

    bool removeRows(int row, int count, const QModelIndex& parent = {}) override
    {
        if (parent.isValid() || row < 0 || count <= 0 || count > rowCount() - row) {
            return false;
        }
        auto rows = store_.rows(type_);
        rows.remove(row, count);
        return saveRows(std::move(rows));
    }

    void removeSelection(const QModelIndexList& selected)
    {
        QSet<quint64> ids;
        for (const auto& index : selected) {
            ids.insert(index.data(Qt::UserRole).toULongLong());
        }
        auto rows = store_.rows(type_);
        rows.removeIf([&](const auto& row) { return ids.contains(row.id); });
        if (!ids.isEmpty()) {
            saveRows(std::move(rows));
        }
    }

    void setPage(PresetType type)
    {
        beginResetModel();
        type_ = type;
        endResetModel();
    }

    void setDisplay(bool thumbnail, bool labels, bool stroke)
    {
        decoration_ = thumbnail;
        labels_ = labels;
        stroke_ = stroke;
        refresh();
    }

private:
    bool moveIds(const QSet<quint64>& ids, int destination)
    {
        if (ids.isEmpty()) {
            return false;
        }
        PresetRows rows;
        PresetRows moved;
        int position = 0;
        int insertion = 0;
        for (const auto& entry : store_.rows(type_)) {
            if (ids.contains(entry.id)) {
                moved.push_back(entry);
            } else {
                rows.push_back(entry);
                if (position < destination) {
                    ++insertion;
                }
            }
            ++position;
        }
        if (moved.isEmpty()) {
            return false;
        }
        for (const auto& row : moved) {
            rows.insert(insertion++, row);
        }
        return saveRows(std::move(rows));
    }

    bool saveRows(PresetRows rows)
    {
        QString error;
        if (!store_.replace(type_, std::move(rows), &error)) {
            failure(qobject_cast<QWidget*>(parent()), error);
            return false;
        }
        return true;
    }

    void refresh() { beginResetModel(); endResetModel(); }

    PresetStore& store_;
    PresetType type_ = PresetType::Brushes;
    bool decoration_ = true;
    bool stroke_ = false;
    bool labels_ = true;
};

PresetManager::PresetManager(PresetStore& store, PresetType initialType, QWidget* parent)
    : QDialog(parent), store_(store)
{
    setWindowTitle(tr("Preset Manager"));
    resize(610, 420);
    auto* layout = new QVBoxLayout(this);
    auto* selectors = new QHBoxLayout;
    type_ = new QComboBox(this);
    type_->setObjectName(QStringLiteral("presetType"));
    for (int i = 0; i < presetTypeCount; ++i) {
        type_->addItem(presetTypeName(PresetType(i)));
    }
    selectors->addWidget(type_);
    display_ = new QComboBox(this);
    display_->setObjectName(QStringLiteral("presetDisplay"));
    selectors->addWidget(display_);
    selectors->addStretch();
    auto* library = new QToolButton(this);
    library->setText(tr("Libraries"));
    library->setPopupMode(QToolButton::InstantPopup);
    menu_ = new QMenu(library);
    menu_->setObjectName(QStringLiteral("presetLibraries"));
    library->setMenu(menu_);
    selectors->addWidget(library);
    layout->addLayout(selectors);

    list_ = new QListView(this);
    list_->setObjectName(QStringLiteral("presetList"));
    model_ = new PresetListModel(store, this);
    list_->setModel(model_);
    list_->setSelectionMode(QAbstractItemView::ExtendedSelection);
    list_->setDragDropMode(QAbstractItemView::InternalMove);
    list_->setDefaultDropAction(Qt::MoveAction);
    list_->setDropIndicatorShown(true);
    list_->setResizeMode(QListView::Adjust);
    list_->setUniformItemSizes(true);
    layout->addWidget(list_, 1);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    auto* load = buttons->addButton(tr("Load…"), QDialogButtonBox::ActionRole);
    auto* save = buttons->addButton(tr("Save Set…"), QDialogButtonBox::ActionRole);
    auto* rename = buttons->addButton(tr("Rename…"), QDialogButtonBox::ActionRole);
    auto* remove = buttons->addButton(tr("Delete"), QDialogButtonBox::ActionRole);
    layout->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(load, &QPushButton::clicked, this, [this] { pickImport(false); });
    connect(save, &QPushButton::clicked, this, [this] { saveSet(); });
    connect(rename, &QPushButton::clicked, this, [this] {
        const auto index = list_->currentIndex();
        if (!index.isValid()) {
            return;
        }
        const auto id = index.data(Qt::UserRole).toULongLong();
        const auto selectedType = type();
        bool accepted = false;
        const auto name = QInputDialog::getText(this, tr("Rename Preset"), tr("Name"), QLineEdit::Normal,
                                                index.data(Qt::EditRole).toString(), &accepted);
        if (accepted && type() == selectedType) {
            for (int row = 0; row < model_->rowCount(); ++row) {
                const auto current = model_->index(row, 0);
                if (current.data(Qt::UserRole).toULongLong() == id) {
                    model_->setData(current, name, Qt::EditRole);
                    break;
                }
            }
        }
    });
    connect(remove, &QPushButton::clicked, this, [this] { model_->removeSelection(list_->selectionModel()->selectedIndexes()); });
    connect(type_, &QComboBox::currentIndexChanged, this, &PresetManager::changePage);
    connect(display_, &QComboBox::currentIndexChanged, this, &PresetManager::changeDisplay);
    connect(menu_, &QMenu::aboutToShow, this, &PresetManager::buildLibraryMenu);
    connect(this, &QDialog::finished, this, [this] { cancelImport(); });
    type_->setCurrentIndex(int(initialType));
    changePage();
}

PresetType PresetManager::type() const { return PresetType(type_->currentIndex()); }

void PresetManager::changePage()
{
    cancelImport();
    const int old = display_->currentIndex() < 0 ? 1 : display_->currentIndex();
    const QSignalBlocker block(display_);
    display_->clear();
    display_->addItems({tr("Text Only"), tr("Small Thumbnail"), tr("Large Thumbnail"),
                        tr("Small List"), tr("Large List")});
    if (type() == PresetType::Brushes) {
        display_->addItem(tr("Stroke Thumbnail"));
    }
    display_->setCurrentIndex(std::min(old, display_->count() - 1));
    model_->setPage(type());
    changeDisplay();
}

void PresetManager::changeDisplay()
{
    const int mode = display_->currentIndex();
    const bool grid = mode == 1 || mode == 2;
    const int size = mode == 2 || mode == 4 || mode == 5 ? 44 : 24;
    list_->setViewMode(grid ? QListView::IconMode : QListView::ListMode);
    list_->setIconSize(QSize(size, size));
    model_->setDisplay(mode != 0, !grid || type() != PresetType::Swatches, mode == 5);
}

void PresetManager::pickImport(bool replace)
{
    const auto path = QFileDialog::getOpenFileName(this, tr("Open Preset Library"),
                                                  presetLibraryFolder(type()), presetFileFilter(type(), false));
    if (!path.isEmpty()) {
        importFile(path, replace);
    }
}

void PresetManager::importFile(QString path, bool replace)
{
    cancelImport();
    const auto kind = type();
    const auto request = store_.beginImport(kind);
    importType_ = kind;
    importRequest_ = request;
    const QPointer<PresetManager> dialog(this);
    QThreadPool::globalInstance()->start([dialog, kind, request, path = std::move(path), replace] {
        auto report = readPresetFile(path, kind);
        QMetaObject::invokeMethod(QCoreApplication::instance(),
            [dialog, kind, request, replace, report = std::move(report)] {
                if (!dialog || dialog->type() != kind || !dialog->store_.isCurrentImport(kind, request)) {
                    return;
                }
                if (!report.ok || (report.rows.isEmpty() && !report.explicitEmpty)) {
                    failure(dialog, report.diagnostics);
                    return;
                }
                if (!report.diagnostics.isEmpty()
                    && QMessageBox::question(dialog, tr("Partial Preset Library"),
                        tr("Import the usable presets?\n\n%1").arg(report.diagnostics)) != QMessageBox::Yes) {
                    return;
                }
                if (!dialog || dialog->type() != kind || !dialog->store_.isCurrentImport(kind, request)) return;
                if (!replace && report.rows.isEmpty()) return;
                auto rows = replace ? PresetRows{} : dialog->store_.rows(kind);
                for (auto row : report.rows) {
                    row.id = 0;
                    rows.push_back(std::move(row));
                }
                QThreadPool::globalInstance()->start([dialog, kind, request, rows = std::move(rows)]() mutable {
                    QString error;
                    auto staged = stagePresetImport(kind, rows, &error);
                    QMetaObject::invokeMethod(QCoreApplication::instance(),
                        [dialog, kind, request, rows = std::move(rows), staged = std::move(staged), error]() mutable {
                            if (!dialog || dialog->type() != kind || !dialog->store_.isCurrentImport(kind, request)) return;
                            QString failureText = error;
                            if (!staged || !dialog->store_.commitStagedImport(kind, std::move(rows), staged, &failureText)) {
                                failure(dialog, failureText);
                            }
                        }, Qt::QueuedConnection);
                });
            }, Qt::QueuedConnection);
    });
}

void PresetManager::cancelImport()
{
    if (importRequest_ != 0) {
        store_.cancelImport(importType_, importRequest_);
        importRequest_ = 0;
    }
}

void PresetManager::saveSet(const QString& format)
{
    const auto kind = type();
    QString filter;
    const auto filters = format.isEmpty() ? presetFileFilter(kind, true)
                                          : tr("Adobe Swatches (*.%1)").arg(format);
    auto path = QFileDialog::getSaveFileName(this, tr("Save Preset Set"), presetLibraryFolder(kind),
                                             filters, &filter);
    if (path.isEmpty()) {
        return;
    }
    if (QFileInfo(path).suffix().isEmpty()) {
        if (filter.contains(QStringLiteral("*.aco"))) {
            path += QStringLiteral(".aco");
        } else if (filter.contains(QStringLiteral("*.ase"))) {
            path += QStringLiteral(".ase");
        } else {
            path += QStringLiteral(".kpset");
        }
    }
    exportFile(std::move(path));
}

void PresetManager::exportFile(QString path)
{
    const auto kind = type();
    auto rows = store_.rows(kind);
    const auto indices = list_->selectionModel()->selectedIndexes();
    if (!indices.isEmpty()) {
        QSet<quint64> ids;
        for (const auto& index : indices) {
            ids.insert(index.data(Qt::UserRole).toULongLong());
        }
        rows.removeIf([&](const auto& row) { return !ids.contains(row.id); });
    }
    const QPointer<PresetManager> dialog(this);
    QThreadPool::globalInstance()->start([dialog, kind, path = std::move(path), rows = std::move(rows)] {
        QString error;
        if (!savePresetFile(path, kind, rows, &error)) {
            QMetaObject::invokeMethod(QCoreApplication::instance(), [dialog, error] {
                if (dialog) {
                    failure(dialog, error);
                }
            }, Qt::QueuedConnection);
        }
    });
}

void PresetManager::resetType()
{
    QMessageBox prompt(QMessageBox::Question, tr("Reset Presets"),
                       tr("Replace the current presets or append the defaults?"), QMessageBox::Cancel, this);
    auto* replace = prompt.addButton(tr("Replace"), QMessageBox::AcceptRole);
    auto* append = prompt.addButton(tr("Append"), QMessageBox::ActionRole);
    prompt.exec();
    if (prompt.clickedButton() == replace || prompt.clickedButton() == append) {
        QString error;
        if (!store_.reset(type(), prompt.clickedButton() == append, &error)) {
            failure(this, error);
        }
    }
}

void PresetManager::buildLibraryMenu()
{
    menu_->clear();
    menu_->addAction(tr("Replace…"), this, [this] { pickImport(true); });
    menu_->addAction(tr("Reset…"), this, &PresetManager::resetType);
    const auto files = discoveredPresetFiles(type());
    if (!files.isEmpty()) {
        menu_->addSeparator();
    }
    for (const auto& file : files) {
        auto* library = menu_->addMenu(QFileInfo(file).completeBaseName());
        library->addAction(tr("Append"), this, [this, file] { importFile(file, false); });
        library->addAction(tr("Replace"), this, [this, file] { importFile(file, true); });
    }
}

void PresetManager::runLibraryAction(const QString& action)
{
    if (action == QStringLiteral("swatches.load")) {
        pickImport(false);
    } else if (action == QStringLiteral("swatches.replace")) {
        pickImport(true);
    } else if (action == QStringLiteral("swatches.save")) {
        saveSet(QStringLiteral("aco"));
    } else if (action == QStringLiteral("swatches.exchange")) {
        saveSet(QStringLiteral("ase"));
    } else if (action == QStringLiteral("swatches.reset")) {
        resetType();
    } else if (action.startsWith(QStringLiteral("append:"))) {
        importFile(action.mid(7), false);
    } else if (action.startsWith(QStringLiteral("replace:"))) {
        importFile(action.mid(8), true);
    }
}

void openPresetManager(QWidget* parent, PresetType type, const QString& action)
{
    PresetManager dialog(PresetStore::current(), type, parent);
    if (!action.isEmpty()) {
        QTimer::singleShot(0, &dialog, [&dialog, action] { dialog.runLibraryAction(action); });
    }
    runDialog(dialog, parent);
}

} // namespace pictura

#include "moc_preset_manager.cpp"
