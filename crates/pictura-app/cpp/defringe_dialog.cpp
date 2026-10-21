#include "defringe_dialog.h"

#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

DefringeDialog::DefringeDialog(int defaultWidth, QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Defringe"));

    auto* outer = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;

    auto* row = new QHBoxLayout;
    auto* label = new QLabel(tr("Width:"), this);
    label->setFixedWidth(50);
    label->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
    row->addWidget(label);
    width_ = new QSpinBox(this);
    width_->setObjectName(QStringLiteral("defringeWidth"));
    width_->setRange(1, 250);
    width_->setValue(defaultWidth);
    width_->setSuffix(tr(" px"));
    width_->setFixedWidth(80);
    row->addWidget(width_);
    row->addStretch();
    left->addLayout(row);
    left->addStretch();
    outer->addLayout(left, 1);

    auto* buttons = new QVBoxLayout;
    auto* ok = new QPushButton(tr("OK"), this);
    ok->setDefault(true);
    ok->setFixedWidth(80);
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setFixedWidth(80);
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    buttons->addStretch();
    outer->addLayout(buttons);

    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
}

int DefringeDialog::width() const { return width_->value(); }

bool DefringeDialog::get(int* out, QWidget* parent)
{
    DefringeDialog dialog(1, parent);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    if (out) {
        *out = dialog.width();
    }
    return true;
}

} // namespace pictura

#include "moc_defringe_dialog.cpp"
