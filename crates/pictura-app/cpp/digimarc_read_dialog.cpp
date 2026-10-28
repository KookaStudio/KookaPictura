#include "digimarc_read_dialog.h"

#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include "dialogs.h"

namespace pictura {

DigimarcReadDialog::DigimarcReadDialog(bool found, const QString& kind, int creatorId, int id,
                                       QWidget* parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Read Watermark"));

    auto* layout = new QVBoxLayout(this);
    const QString text =
        found ? tr("An in-tree watermark was found (approximate; not Digimarc).\n\n"
                   "Type: %1\nCreator ID: %2\nID: %3")
                    .arg(kind)
                    .arg(creatorId)
                    .arg(id)
              : tr("No watermark was found.");
    detail_ = new QLabel(text, this);
    detail_->setObjectName(QStringLiteral("digimarcReadResult"));
    detail_->setWordWrap(true);
    detail_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(detail_);

    auto* ok = new QPushButton(tr("OK"), this);
    ok->setDefault(true);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    layout->addWidget(ok, 0, Qt::AlignRight);
}

void DigimarcReadDialog::showResult(bool found, const QString& kind, int creatorId, int id,
                                    QWidget* parent)
{
    DigimarcReadDialog dialog(found, kind, creatorId, id, parent);
    runDialog(dialog, parent);
}

} // namespace pictura

#include "moc_digimarc_read_dialog.cpp"
