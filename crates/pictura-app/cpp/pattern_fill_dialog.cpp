#include "pattern_fill_dialog.h"

#include "dialogs.h"
#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object/impl_layers/fill_content.cxxqt.h"

#include <QtCore/QStringList>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

PatternFillDialog::PatternFillDialog(PictureView& view, const QString& payload, QWidget* parent)
    : QDialog(parent)
    , view_(view)
{
    setObjectName(QStringLiteral("patternFillDialog"));
    setWindowTitle(tr("Pattern Fill"));

    const QStringList parts = payload.split(QLatin1Char('|'));
    if (!parts.isEmpty()) {
        id_ = parts.at(0);
    }
    const double scale = parts.size() > 1 ? parts.at(1).toDouble() : 100.0;
    const bool link = parts.size() <= 2 || parts.at(2) == QLatin1String("1");
    if (parts.size() > 4) {
        originX_ = parts.at(3).toInt();
        originY_ = parts.at(4).toInt();
    }

    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(10);

    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("fillPatternList"));
    list_->setViewMode(QListView::IconMode);
    list_->setIconSize(QSize(32, 32));
    list_->setGridSize(QSize(72, 56));
    list_->setResizeMode(QListView::Adjust);
    const QStringList ids = document_pattern_ids(view_);
    for (const QString& id : ids) {
        const QImage thumbnail = pattern_thumbnail(view_, id, 32);
        auto* item = new QListWidgetItem(QIcon(QPixmap::fromImage(thumbnail)), id, list_);
        if (id == id_) {
            list_->setCurrentItem(item);
        }
        if (id_.isEmpty()) {
            id_ = id;
            list_->setCurrentItem(item);
        }
    }
    // An edited layer may name a pattern the document no longer carries; fall
    // back to the first available entry instead of accepting a stale id.
    if (list_->currentItem() == nullptr && list_->count() > 0) {
        list_->setCurrentItem(list_->item(0));
    }
    outer->addWidget(list_);

    empty_ = ids.isEmpty();
    if (empty_) {
        emptyLabel_ = new QLabel(tr("No patterns are loaded — this document has no pattern "
                                    "resources."),
                                 this);
        emptyLabel_->setObjectName(QStringLiteral("fillPatternEmpty"));
        emptyLabel_->setWordWrap(true);
        outer->addWidget(emptyLabel_);
    }

    auto* form = new QFormLayout;
    NumericFieldConfig scaleConfig;
    scaleConfig.minimum = 1.0;
    scaleConfig.maximum = 1000.0;
    scaleConfig.decimals = 0;
    scaleConfig.suffix = QStringLiteral(" %");
    scaleConfig.objectName = QStringLiteral("fillPatternScale");
    scale_ = new NumericField(QString(), scaleConfig, this);
    scale_->setValue(scale);
    form->addRow(tr("Scale:"), scale_);

    link_ = new QCheckBox(tr("Link With Layer"), this);
    link_->setObjectName(QStringLiteral("fillPatternLink"));
    link_->setChecked(link);
    form->addRow(QString(), link_);

    snap_ = new QPushButton(tr("Snap To Origin"), this);
    snap_->setObjectName(QStringLiteral("fillPatternSnap"));
    connect(snap_, &QPushButton::clicked, this, [this] {
        originX_ = 0;
        originY_ = 0;
    });
    form->addRow(QString(), snap_);
    outer->addLayout(form);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    if (empty_) {
        buttons->button(QDialogButtonBox::Ok)->setEnabled(false);
    }
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    outer->addWidget(buttons);
}

QString PatternFillDialog::payload() const
{
    if (empty_) {
        return QString();
    }
    QString id = id_;
    if (const QListWidgetItem* item = list_->currentItem()) {
        id = item->text();
    }
    return QStringLiteral("%1|%2|%3|%4|%5")
        .arg(id)
        .arg(scale_->value())
        .arg(link_->isChecked() ? 1 : 0)
        .arg(originX_)
        .arg(originY_);
}

bool PatternFillDialog::getFill(PictureView& view, const QString& initial, QWidget* parent,
                                QString* out)
{
    PatternFillDialog dialog(view, initial, parent);
    if (runDialog(dialog, parent) != QDialog::Accepted || !out) {
        return false;
    }
    *out = dialog.payload();
    return !out->isEmpty();
}

} // namespace pictura

#include "moc_pattern_fill_dialog.cpp"
