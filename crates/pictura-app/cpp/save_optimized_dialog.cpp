#include "save_optimized_dialog.h"

#include <QtCore/QFileInfo>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

SaveOptimizedDialog::SaveOptimizedDialog(const QString& defaultPath, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("saveOptimizedDialog"));
    setWindowTitle(QStringLiteral("Save Optimized As"));

    auto* root = new QVBoxLayout(this);
    auto* form = new QFormLayout;
    auto* pathRow = new QHBoxLayout;
    path_ = new QLineEdit(defaultPath, this);
    path_->setObjectName(QStringLiteral("saveOptimizedPath"));
    auto* browse = new QPushButton(QStringLiteral("Browse..."), this);
    browse->setObjectName(QStringLiteral("saveOptimizedBrowse"));
    pathRow->addWidget(path_, 1);
    pathRow->addWidget(browse);
    form->addRow(QStringLiteral("File name:"), pathRow);

    format_ = new QComboBox(this);
    format_->setObjectName(QStringLiteral("saveOptimizedFormat"));
    format_->addItems({QStringLiteral("HTML and Images"), QStringLiteral("Images Only"),
                       QStringLiteral("HTML Only")});
    form->addRow(QStringLiteral("Format:"), format_);

    auto* slices = new QComboBox(this);
    slices->setObjectName(QStringLiteral("saveOptimizedSlices"));
    slices->addItem(QStringLiteral("All Slices"));
    slices->setEnabled(false);
    form->addRow(QStringLiteral("Slices:"), slices);

    layout_ = new QComboBox(this);
    layout_->setObjectName(QStringLiteral("saveOptimizedLayout"));
    layout_->addItems({QStringLiteral("Table"), QStringLiteral("CSS")});
    form->addRow(QStringLiteral("Layout:"), layout_);
    root->addLayout(form);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel,
                                         this);
    connect(browse, &QPushButton::clicked, this, [this] {
        const QString chosen = QFileDialog::getSaveFileName(
            this, QStringLiteral("Save Optimized As"), path_->text(),
            QStringLiteral("HTML (*.html);;Images (*.gif *.jpg *.png *.wbmp)"));
        if (!chosen.isEmpty()) {
            path_->setText(chosen);
        }
    });
    connect(buttons, &QDialogButtonBox::accepted, this, [this] {
        if (!path_->text().trimmed().isEmpty()) {
            accept();
        }
    });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);
}

QString SaveOptimizedDialog::path() const
{
    return path_->text();
}

int SaveOptimizedDialog::format() const
{
    return format_->currentIndex();
}

int SaveOptimizedDialog::layout() const
{
    return layout_->currentIndex();
}

} // namespace pictura
