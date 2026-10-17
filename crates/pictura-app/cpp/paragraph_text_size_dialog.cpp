// The "Paragraph Text Size" dialog an Alt-click with a Type tool opens: the
// paragraph box's Width and Height (docs/03-tools/type-tools.md, Type
// creation modes).

#include "paragraph_text_size_dialog.h"

#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QDoubleSpinBox* pixelField(const QString& name, double value, QWidget* parent)
{
    auto* field = new QDoubleSpinBox(parent);
    field->setObjectName(name);
    field->setDecimals(2);
    field->setRange(1.0, 300000.0);
    field->setSuffix(QStringLiteral(" px"));
    field->setValue(value);
    return field;
}

} // namespace

bool execParagraphTextSizeDialog(QSizeF& size, QWidget* parent)
{
    QDialog dialog(parent);
    dialog.setObjectName(QStringLiteral("paragraphTextSizeDialog"));
    dialog.setWindowTitle(QObject::tr("Paragraph Text Size"));
    auto* layout = new QVBoxLayout(&dialog);
    auto* form = new QFormLayout;
    auto* width = pixelField(QStringLiteral("paragraphTextWidth"), size.width(), &dialog);
    auto* height = pixelField(QStringLiteral("paragraphTextHeight"), size.height(), &dialog);
    form->addRow(QObject::tr("Width:"), width);
    form->addRow(QObject::tr("Height:"), height);
    layout->addLayout(form);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    layout->addWidget(buttons);
    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }
    size = QSizeF(width->value(), height->value());
    return true;
}

} // namespace pictura
