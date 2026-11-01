#include "auto_blend_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

AutoBlendDialog::AutoBlendDialog(QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("autoBlendDialog"));
    setWindowTitle(tr("Auto-Blend Layers"));

    auto* method = new QGroupBox(tr("Blend Method"), this);
    auto* methodLayout = new QVBoxLayout(method);
    auto* panorama = new QRadioButton(tr("Panorama"), method);
    panorama->setObjectName(QStringLiteral("autoBlendPanorama"));
    panorama->setChecked(true);
    stack_ = new QRadioButton(tr("Stack Images"), method);
    stack_->setObjectName(QStringLiteral("autoBlendStack"));
    methodLayout->addWidget(panorama);
    methodLayout->addWidget(stack_);

    seamless_ = new QCheckBox(tr("Seamless Tones and Colors"), this);
    seamless_->setObjectName(QStringLiteral("autoBlendSeamless"));

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    auto* layout = new QVBoxLayout(this);
    layout->addWidget(method);
    layout->addWidget(seamless_);
    layout->addWidget(buttons);
}

bool AutoBlendDialog::stackImages() const
{
    return stack_->isChecked();
}

bool AutoBlendDialog::seamless() const
{
    return seamless_->isChecked();
}

} // namespace pictura
