#include "bmp_advanced_modes_dialog.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

BmpAdvancedModesDialog::BmpAdvancedModesDialog(int initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("bmpAdvancedModesDialog"));
    setWindowTitle(tr("Advanced Modes"));
    auto* root = new QVBoxLayout(this);
    auto* box = new QGroupBox(tr("16-bit layout"), this);
    auto* layout = new QVBoxLayout(box);
    auto* group = new QButtonGroup(box);
    const char* names[] = {"X1R5G5B5", "A1R5G5B5", "R5G6B5"};
    const char* objects[] = {"bmpAdvancedX1R5G5B5", "bmpAdvancedA1R5G5B5", "bmpAdvancedR5G6B5"};
    for (int i = 0; i < 3; ++i) {
        layouts_[i] = new QRadioButton(QString::fromLatin1(names[i]), box);
        layouts_[i]->setObjectName(QString::fromLatin1(objects[i]));
        group->addButton(layouts_[i], i);
        if (i == initial) {
            layouts_[i]->setChecked(true);
        }
        layout->addWidget(layouts_[i]);
    }
    root->addWidget(box);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &BmpAdvancedModesDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &BmpAdvancedModesDialog::reject);
    root->addWidget(buttons);
    setMinimumWidth(300);
}

int BmpAdvancedModesDialog::layout() const
{
    for (int i = 0; i < 3; ++i) {
        if (layouts_[i]->isChecked()) {
            return i;
        }
    }
    return 0;
}

} // namespace pictura

#include "moc_bmp_advanced_modes_dialog.cpp"
