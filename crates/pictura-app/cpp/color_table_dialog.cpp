#include "color_table_dialog.h"

#include <QtCore/QFile>
#include <QtCore/QSaveFile>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTableWidget>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

#include "api/document_bridge.h"

namespace pictura {

namespace {

constexpr int columns = 16;

} // namespace

ColorTableDialog::ColorTableDialog(const QByteArray& rgba, int minimumEntries, QWidget* parent)
    : QDialog(parent), rgba_(rgba), minimumEntries_(minimumEntries),
      grid_(new QTableWidget(this))
{
    setWindowTitle(tr("Color Table"));
    setObjectName(QStringLiteral("indexedColorTableDialog"));
    auto* layout = new QVBoxLayout(this);
    grid_->setObjectName(QStringLiteral("indexedColorTable"));
    grid_->setColumnCount(columns);
    grid_->horizontalHeader()->hide();
    grid_->verticalHeader()->hide();
    grid_->horizontalHeader()->setSectionResizeMode(QHeaderView::Fixed);
    grid_->horizontalHeader()->setDefaultSectionSize(26);
    grid_->verticalHeader()->setSectionResizeMode(QHeaderView::Fixed);
    grid_->verticalHeader()->setDefaultSectionSize(26);
    grid_->setMinimumWidth(columns * 26 + 24);
    grid_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    grid_->setSelectionMode(QAbstractItemView::ContiguousSelection);
    layout->addWidget(grid_);

    auto* tools = new QHBoxLayout;
    auto* edit = new QPushButton(tr("Edit Selection…"), this);
    auto* transparent = new QPushButton(tr("Set Transparent"), this);
    transparent->setObjectName(QStringLiteral("tableSetTransparent"));
    auto* opaque = new QPushButton(tr("Clear Transparency"), this);
    tools->addWidget(edit);
    tools->addWidget(transparent);
    tools->addWidget(opaque);
    layout->addLayout(tools);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    auto* load = buttons->addButton(tr("Load ACT…"), QDialogButtonBox::ActionRole);
    auto* save = buttons->addButton(tr("Save ACT…"), QDialogButtonBox::ActionRole);
    layout->addWidget(buttons);

    connect(edit, &QPushButton::clicked, this, &ColorTableDialog::editSelection);
    connect(grid_, &QTableWidget::itemDoubleClicked, this, &ColorTableDialog::editSelection);
    connect(transparent, &QPushButton::clicked, this, [this] {
        const int entry = currentEntry();
        if (entry >= 0) {
            markTransparent(entry);
        }
    });
    connect(opaque, &QPushButton::clicked, this, [this] { markTransparent(-1); });
    connect(load, &QPushButton::clicked, this, &ColorTableDialog::loadFile);
    connect(save, &QPushButton::clicked, this, &ColorTableDialog::saveFile);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    refresh();
    grid_->setCurrentCell(0, 0);
}

int ColorTableDialog::currentEntry() const
{
    const auto* item = grid_->currentItem();
    return item ? item->row() * columns + item->column() : -1;
}

QColor ColorTableDialog::entryColor(int entry) const
{
    const auto* rgb = reinterpret_cast<const unsigned char*>(rgba_.constData()) + 4 * entry;
    return QColor(rgb[0], rgb[1], rgb[2]);
}

void ColorTableDialog::editSelection()
{
    const auto selection = grid_->selectedItems();
    if (selection.isEmpty()) {
        return;
    }
    int first = static_cast<int>(rgba_.size() / 4);
    int last = 0;
    for (const auto* item : selection) {
        const int entry = item->row() * columns + item->column();
        first = std::min(first, entry);
        last = std::max(last, entry);
    }
    const auto from = QColorDialog::getColor(entryColor(first), this, tr("First Color"));
    if (!from.isValid()) {
        return;
    }
    const auto to = first == last
        ? from
        : QColorDialog::getColor(entryColor(last), this, tr("Last Color"));
    if (to.isValid()) {
        interpolate(first, last, from, to);
    }
}

void ColorTableDialog::interpolate(int first, int last, const QColor& from, const QColor& to)
{
    if (first < 0 || last < first || last >= rgba_.size() / 4) {
        return;
    }
    const int span = std::max(1, last - first);
    for (int i = first; i <= last; ++i) {
        const int position = i - first;
        const auto channel = [&](int a, int b) {
            return static_cast<char>((a * (span - position) + b * position + span / 2) / span);
        };
        rgba_[4 * i] = channel(from.red(), to.red());
        rgba_[4 * i + 1] = channel(from.green(), to.green());
        rgba_[4 * i + 2] = channel(from.blue(), to.blue());
    }
    refresh();
}

void ColorTableDialog::markTransparent(int entry)
{
    for (qsizetype i = 0; i < rgba_.size() / 4; ++i) {
        rgba_[4 * i + 3] = i == entry ? char(0) : char(255);
    }
    refresh();
}

void ColorTableDialog::refresh()
{
    const int selected = currentEntry();
    const int count = static_cast<int>(rgba_.size() / 4);
    grid_->clearContents();
    grid_->setRowCount((count + columns - 1) / columns);
    for (int i = 0; i < count; ++i) {
        auto* item = new QTableWidgetItem;
        const QColor color = entryColor(i);
        item->setBackground(color);
        item->setToolTip(tr("Index %1 · %2").arg(i).arg(color.name()));
        if (rgba_[4 * i + 3] == 0) {
            item->setText(QStringLiteral("×"));
            item->setTextAlignment(Qt::AlignCenter);
            item->setForeground(color.lightness() >= 128 ? QColor(Qt::black) : QColor(Qt::white));
            item->setToolTip(tr("Index %1 · Transparent").arg(i));
        }
        grid_->setItem(i / columns, i % columns, item);
    }
    if (selected >= 0 && selected < count) {
        grid_->setCurrentCell(selected / columns, selected % columns);
    }
}

bool ColorTableDialog::loadAct(const QByteArray& bytes)
{
    const auto table = decode_indexed_act(bytes);
    if (table.isEmpty() || table.size() / 4 < minimumEntries_) {
        return false;
    }
    rgba_ = table;
    refresh();
    return true;
}

QByteArray ColorTableDialog::saveAct() const
{
    return encode_indexed_act(rgba_);
}

void ColorTableDialog::loadFile()
{
    const auto path = QFileDialog::getOpenFileName(this, tr("Load Color Table"), {},
                                                   tr("Adobe Color Table (*.act)"));
    if (path.isEmpty()) {
        return;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly) || file.size() > 772 || !loadAct(file.read(773))) {
        QMessageBox::warning(this, tr("Load Color Table"),
                             tr("The file is invalid or omits a palette index used by the image."));
    }
}

void ColorTableDialog::saveFile()
{
    auto path = QFileDialog::getSaveFileName(this, tr("Save Color Table"), {},
                                             tr("Adobe Color Table (*.act)"));
    if (path.isEmpty()) {
        return;
    }
    if (!path.endsWith(QStringLiteral(".act"), Qt::CaseInsensitive)) {
        path += QStringLiteral(".act");
    }
    QSaveFile file(path);
    const auto bytes = saveAct();
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size() || !file.commit()) {
        QMessageBox::warning(this, tr("Save Color Table"), tr("The color table could not be saved."));
    }
}

} // namespace pictura
