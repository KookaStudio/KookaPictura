#include "auto_align_dialog.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QRadioButton* radio(const QString& text, const QString& name, QWidget* parent)
{
    auto* button = new QRadioButton(text, parent);
    button->setObjectName(name);
    return button;
}

} // namespace

AutoAlignDialog::AutoAlignDialog(QWidget* parent, const AutoAlignChoice& choice)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("autoAlignDialog"));
    setWindowTitle(tr("Auto-Align Layers"));

    auto* projection = new QGroupBox(tr("Projection"), this);
    auto* layout = new QVBoxLayout(projection);
    auto_ = radio(tr("Auto"), QStringLiteral("autoAlignAuto"), projection);
    perspective_ = radio(tr("Perspective"), QStringLiteral("autoAlignPerspective"), projection);
    cylindrical_ = radio(tr("Cylindrical"), QStringLiteral("autoAlignCylindrical"), projection);
    spherical_ = radio(tr("Spherical"), QStringLiteral("autoAlignSpherical"), projection);
    collage_ = radio(tr("Scene Collage"), QStringLiteral("autoAlignCollage"), projection);
    reposition_ = radio(tr("Reposition Only"), QStringLiteral("autoAlignReposition"), projection);
    lens_ = radio(tr("Lens Correction"), QStringLiteral("autoAlignLens"), projection);
    for (QRadioButton* button : {auto_, perspective_, cylindrical_, spherical_, collage_,
                                 reposition_, lens_}) {
        layout->addWidget(button);
    }

    auto* corrections = new QGroupBox(tr("Lens Correction"), this);
    auto* correctionsLayout = new QVBoxLayout(corrections);
    vignette_ = new QCheckBox(tr("Vignette Removal"), corrections);
    vignette_->setObjectName(QStringLiteral("autoAlignVignette"));
    vignette_->setChecked(choice.vignette);
    geometric_ = new QCheckBox(tr("Geometric Distortion"), corrections);
    geometric_->setObjectName(QStringLiteral("autoAlignGeometric"));
    geometric_->setChecked(choice.geometric);
    correctionsLayout->addWidget(vignette_);
    correctionsLayout->addWidget(geometric_);
    auto* hint = new QLabel(tr("Uses the installed lens profile that matches the document's EXIF."),
                            corrections);
    hint->setWordWrap(true);
    correctionsLayout->addWidget(hint);

    QRadioButton* selected = auto_;
    switch (choice.projection) {
    case 1: selected = perspective_; break;
    case 2: selected = cylindrical_; break;
    case 3: selected = spherical_; break;
    case 4: selected = collage_; break;
    case 5: selected = reposition_; break;
    case 6: selected = lens_; break;
    default: break;
    }
    selected->setChecked(true);
    connect(lens_, &QRadioButton::toggled, this, [this] { syncLensOptions(); });
    syncLensOptions();

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                                         Qt::Vertical, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    auto* outer = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;
    left->addWidget(projection);
    left->addWidget(corrections);
    left->addStretch();
    outer->addLayout(left);
    outer->addWidget(buttons);
}

void AutoAlignDialog::syncLensOptions()
{
    const bool on = lens_->isChecked();
    vignette_->setEnabled(on);
    geometric_->setEnabled(on);
}

int AutoAlignDialog::projection() const
{
    if (perspective_->isChecked()) {
        return 1;
    }
    if (cylindrical_->isChecked()) {
        return 2;
    }
    if (spherical_->isChecked()) {
        return 3;
    }
    if (collage_->isChecked()) {
        return 4;
    }
    if (reposition_->isChecked()) {
        return 5;
    }
    if (lens_->isChecked()) {
        return 6;
    }
    return 0;
}

bool AutoAlignDialog::vignette() const
{
    return vignette_->isEnabled() && vignette_->isChecked();
}

bool AutoAlignDialog::geometric() const
{
    return geometric_->isEnabled() && geometric_->isChecked();
}

AutoAlignChoice AutoAlignDialog::choice() const
{
    return {projection(), vignette_->isChecked(), geometric_->isChecked()};
}

} // namespace pictura
