#include "rasterize_eps_dialog.h"

#include "raster_size_fields.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

RasterizeEpsDialog::RasterizeEpsDialog(QSizeF boundingPoints, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("rasterizeEpsDialog"));
    setWindowTitle(tr("Rasterize Generic EPS Format"));
    auto* root = new QHBoxLayout(this);
    auto* left = new QVBoxLayout;
    size_ = new RasterSizeFields(false, this);
    size_->setSourceSize(boundingPoints);
    left->addWidget(size_);
    antiAliased_ = new QCheckBox(tr("Anti-aliased"), this);
    antiAliased_->setObjectName(QStringLiteral("antiAliased"));
    antiAliased_->setChecked(true);
    left->addWidget(antiAliased_);
    left->addStretch();
    root->addLayout(left, 1);
    auto* buttons =
        new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, Qt::Vertical, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons, 0, Qt::AlignTop);
}

bool RasterizeEpsDialog::antiAliased() const
{
    return antiAliased_->isChecked();
}

void RasterizeEpsDialog::setFieldForTest(const QString& name, const QString& value)
{
    if (name == QStringLiteral("antiAliased")) {
        antiAliased_->setChecked(value == QStringLiteral("1"));
    } else {
        size_->setFieldForTest(name, value);
    }
}

QString RasterizeEpsDialog::fieldForTest(const QString& name) const
{
    if (name == QStringLiteral("antiAliased")) {
        return antiAliased_->isChecked() ? QStringLiteral("1") : QStringLiteral("0");
    }
    return size_->fieldForTest(name);
}

} // namespace pictura
