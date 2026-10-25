#include "slider_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

SliderDialog::SliderDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                           const QString& title, const QVector<Row>& rows,
                           const QStringList& disabledButtons, QWidget* parent)
    : AdjustmentDialog(view, block, visible, title, parent)
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);

    auto* left = new QVBoxLayout;
    auto* grid = new QGridLayout;
    grid->setColumnStretch(1, 1);
    grid->setVerticalSpacing(6);
    // Later rows indent their label a little, as CS6 does.
    for (int i = 0; i < rows.size(); ++i) {
        addStackedRow(grid, i * 3, rows[i].label, rows[i].key, i == 0 ? 0 : 14);
        grid->setRowMinimumHeight(i * 3 + 2, 8);
    }
    left->addLayout(grid);
    left->addStretch(1);
    left->addWidget(previewCheck(), 0, Qt::AlignRight);
    outer->addLayout(left, 1);

    QVBoxLayout* buttons = buttonColumn(disabledButtons);
    for (const QString& text : disabledButtons) {
        button(text)->setEnabled(false);
    }
    buttons->addStretch(1);
    outer->addLayout(buttons);

    setMinimumWidth(420);
}

QWidget* SliderDialog::controlForTest(const QString& key) const
{
    return findChild<QSpinBox*>(key);
}

} // namespace pictura

#include "moc_slider_dialog.cpp"
