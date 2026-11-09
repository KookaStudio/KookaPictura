#include "fade_dialog.h"

#include "blend_modes.h"

#include "api/filters_bridge.h"

#include <QtGui/QStandardItemModel>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

FadeDialog::FadeDialog(PictureView& view, QWidget* parent)
    : QDialog(parent)
    , view_(&view)
{
    setWindowTitle(tr("Fade"));
    setFixedSize(320, 190);

    auto* outer = new QVBoxLayout(this);
    auto* form = new QFormLayout;

    mode_ = new QComboBox(this);
    mode_->setObjectName(QStringLiteral("fadeMode"));
    for (const BlendEntry& entry : blendModes()) {
        mode_->addItem(entry.name, entry.key);
    }
    if (auto* model = qobject_cast<QStandardItemModel*>(mode_->model())) {
        for (int i = 0; i < mode_->count(); ++i) {
            const QString key = mode_->itemData(i).toString();
            if (!fade_mode_allowed(view, key)) {
                if (auto* item = model->item(i)) {
                    item->setEnabled(false);
                }
            }
        }
    }
    const int current = mode_->findData(fade_mode(view));
    mode_->setCurrentIndex(current >= 0 ? current : 0);
    form->addRow(tr("Mode:"), mode_);

    auto* opacityRow = new QWidget(this);
    auto* opacityLayout = new QHBoxLayout(opacityRow);
    opacityLayout->setContentsMargins(0, 0, 0, 0);
    opacitySlider_ = new QSlider(Qt::Horizontal, opacityRow);
    opacitySlider_->setObjectName(QStringLiteral("fadeOpacitySlider"));
    opacitySlider_->setRange(0, 100);
    opacityLayout->addWidget(opacitySlider_, 1);
    opacity_ = new QSpinBox(opacityRow);
    opacity_->setObjectName(QStringLiteral("fadeOpacity"));
    opacity_->setRange(0, 100);
    opacity_->setSuffix(QStringLiteral(" %"));
    opacity_->setValue(fade_opacity(view));
    opacitySlider_->setValue(opacity_->value());
    opacityLayout->addWidget(opacity_);
    form->addRow(tr("Opacity:"), opacityRow);

    outer->addLayout(form);

    preview_ = new QCheckBox(tr("Preview"), this);
    preview_->setObjectName(QStringLiteral("fadePreview"));
    preview_->setChecked(true);
    outer->addWidget(preview_);
    outer->addStretch();

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    buttons->button(QDialogButtonBox::Ok)->setObjectName(QStringLiteral("fadeOk"));
    buttons->button(QDialogButtonBox::Cancel)->setObjectName(QStringLiteral("fadeCancel"));
    outer->addWidget(buttons);

    connect(opacitySlider_, &QSlider::valueChanged, opacity_, &QSpinBox::setValue);
    connect(opacity_, QOverload<int>::of(&QSpinBox::valueChanged), opacitySlider_,
            &QSlider::setValue);
    connect(opacity_, QOverload<int>::of(&QSpinBox::valueChanged), this,
            [this](int) { preview(); });
    connect(mode_, QOverload<int>::of(&QComboBox::currentIndexChanged), this,
            [this](int) { preview(); });
    connect(preview_, &QCheckBox::toggled, this, [this](bool on) {
        if (on) {
            preview();
        } else {
            fade_cancel(*view_);
        }
    });
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
}

FadeDialog::~FadeDialog()
{
    if (!applied_) {
        fade_cancel(*view_);
    }
}

int FadeDialog::opacity() const
{
    return opacity_->value();
}

QString FadeDialog::modeKey() const
{
    return mode_->currentData().toString();
}

void FadeDialog::preview()
{
    if (preview_->isChecked()) {
        fade_preview(*view_, opacity_->value(), modeKey());
    }
}

bool FadeDialog::apply()
{
    if (!fade_apply(*view_, opacity_->value(), modeKey())) {
        return false;
    }
    applied_ = true;
    return true;
}

void FadeDialog::reject()
{
    if (!applied_) {
        fade_cancel(*view_);
    }
    QDialog::reject();
}

} // namespace pictura
