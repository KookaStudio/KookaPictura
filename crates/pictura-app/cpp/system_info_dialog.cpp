#include "system_info_dialog.h"

#include <QtGui/QClipboard>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

SystemInfoDialog::SystemInfoDialog(const SystemInfo& info, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("systemInfoDialog"));
    setWindowTitle(tr("System Info"));
    setMinimumSize(620, 440);
    resize(760, 560);

    text_ = new QPlainTextEdit(this);
    text_->setObjectName(QStringLiteral("systemInfoText"));
    text_->setReadOnly(true);
    text_->setLineWrapMode(QPlainTextEdit::NoWrap);
    text_->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Close, this);
    auto* copy = new QPushButton(tr("Copy"), buttons);
    copy->setObjectName(QStringLiteral("systemInfoCopy"));
    buttons->addButton(copy, QDialogButtonBox::ActionRole);
    connect(copy, &QPushButton::clicked, this, [this] {
        QGuiApplication::clipboard()->setText(report_);
    });
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    auto* layout = new QVBoxLayout(this);
    layout->addWidget(text_, 1);
    layout->addWidget(buttons);

    setInfo(info);
}

void SystemInfoDialog::setInfo(const SystemInfo& info)
{
    report_ = formatSystemInfo(info);
    text_->setPlainText(report_);
}

QString SystemInfoDialog::reportForTest() const
{
    return report_;
}

} // namespace pictura
