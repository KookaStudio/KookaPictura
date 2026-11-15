#include "save_for_web_color_table.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QIcon>
#include <QtGui/QPixmap>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

ColorTableWidget::ColorTableWidget(QWidget* parent)
    : QGroupBox(QStringLiteral("Color Table"), parent)
{
    setObjectName(QStringLiteral("saveForWebColorTable"));
    auto* root = new QVBoxLayout(this);

    swatches_ = new QListWidget(this);
    swatches_->setObjectName(QStringLiteral("colorTableSwatches"));
    swatches_->setViewMode(QListView::IconMode);
    swatches_->setResizeMode(QListView::Adjust);
    swatches_->setMovement(QListView::Static);
    swatches_->setGridSize(QSize(20, 20));
    swatches_->setSpacing(1);
    swatches_->setSelectionMode(QAbstractItemView::SingleSelection);
    root->addWidget(swatches_, 1);

    sort_ = new QComboBox(this);
    sort_->setObjectName(QStringLiteral("colorTableSort"));
    sort_->addItems({QStringLiteral("Sort: Hue"), QStringLiteral("Sort: Luminance"),
                     QStringLiteral("Sort: Popularity")});
    root->addWidget(sort_);

    auto* buttons = new QHBoxLayout;
    const auto button = [this, buttons](const char* name, const QString& text) {
        auto* b = new QPushButton(text, this);
        b->setObjectName(QString::fromLatin1(name));
        buttons->addWidget(b);
        return b;
    };
    QPushButton* add = button("colorTableAdd", QStringLiteral("Add"));
    QPushButton* del = button("colorTableDelete", QStringLiteral("Delete"));
    QPushButton* map = button("colorTableMap", QStringLiteral("Map"));
    QPushButton* unmap = button("colorTableUnmap", QStringLiteral("Unmap"));
    root->addLayout(buttons);
    auto* files = new QHBoxLayout;
    QPushButton* load = new QPushButton(QStringLiteral("Load..."), this);
    load->setObjectName(QStringLiteral("colorTableLoad"));
    QPushButton* save = new QPushButton(QStringLiteral("Save..."), this);
    save->setObjectName(QStringLiteral("colorTableSave"));
    files->addWidget(load);
    files->addWidget(save);
    root->addLayout(files);

    connect(swatches_, &QListWidget::itemDoubleClicked, this, [this] {
        const int index = selectedIndex();
        if (index < 0) {
            return;
        }
        const QColor current = QColor::fromRgba(colors_.value(index));
        const QColor chosen = QColorDialog::getColor(current, this,
                                                     QStringLiteral("Shift Color"));
        if (chosen.isValid()) {
            editSelected(chosen);
        }
    });
    connect(sort_, &QComboBox::currentIndexChanged, this,
            [this](int index) { emit sortRequested(index); });
    connect(add, &QPushButton::clicked, this, [this] {
        const QColor chosen = QColorDialog::getColor(Qt::white, this,
                                                     QStringLiteral("Add Color"));
        if (chosen.isValid()) {
            addColor(chosen);
        }
    });
    connect(del, &QPushButton::clicked, this, [this] { deleteSelected(); });
    connect(map, &QPushButton::clicked, this, [this] { mapSelected(true); });
    connect(unmap, &QPushButton::clicked, this, [this] { mapSelected(false); });
    connect(load, &QPushButton::clicked, this, [this] { emit loadRequested(); });
    connect(save, &QPushButton::clicked, this, [this] { emit saveRequested(); });
}

void ColorTableWidget::setColors(const QVector<QRgb>& colors)
{
    colors_ = colors;
    rebuild();
}

int ColorTableWidget::selectedIndex() const
{
    return swatches_->currentRow();
}

void ColorTableWidget::selectIndex(int index)
{
    if (index >= 0 && index < swatches_->count()) {
        swatches_->setCurrentRow(index);
    }
}

void ColorTableWidget::rebuild()
{
    const QSignalBlocker blocker(swatches_);
    swatches_->clear();
    for (QRgb color : colors_) {
        auto* item = new QListWidgetItem;
        QPixmap pixmap(14, 14);
        if (qAlpha(color) == 0) {
            pixmap.fill(QColor(0x80, 0x80, 0x80, 128));
        } else {
            pixmap.fill(QColor::fromRgba(color));
        }
        item->setIcon(QIcon(pixmap));
        item->setToolTip(QColor::fromRgba(color).name(QColor::HexArgb));
        swatches_->addItem(item);
    }
}

void ColorTableWidget::editSelected(const QColor& color)
{
    const int index = selectedIndex();
    if (index < 0 || index >= colors_.size()) {
        return;
    }
    const QRgb previous = colors_[index];
    colors_[index] = qRgba(color.red(), color.green(), color.blue(), qAlpha(previous));
    rebuild();
    selectIndex(index);
    emit colorsEdited(colors_);
}

void ColorTableWidget::deleteSelected()
{
    const int index = selectedIndex();
    if (index >= 0) {
        emit deleteRequested(index);
    }
}

void ColorTableWidget::addColor(const QColor& color)
{
    emit addRequested(color.rgb());
}

void ColorTableWidget::mapSelected(bool transparent)
{
    const int index = selectedIndex();
    if (index >= 0) {
        emit transparencyRequested(index, transparent);
    }
}

void ColorTableWidget::sortBy(int sort)
{
    emit sortRequested(sort);
}

} // namespace pictura
