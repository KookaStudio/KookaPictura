#include "control_approval_bar.h"

#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>

namespace pictura {

ControlApprovalBar::ControlApprovalBar(QWidget* window)
    : QFrame(window)
    , label_(new QLabel(this))
    , allow_(new QPushButton(tr("Allow"), this))
    , deny_(new QPushButton(tr("Deny"), this))
{
    setObjectName(QStringLiteral("controlApprovalBar"));
    setFrameShape(QFrame::StyledPanel);
    setAutoFillBackground(true);
    setFocusPolicy(Qt::NoFocus);
    setAttribute(Qt::WA_ShowWithoutActivating);
    // The path comes from an agent, so it must not be able to style itself.
    label_->setTextFormat(Qt::PlainText);
    allow_->setFocusPolicy(Qt::NoFocus);
    deny_->setFocusPolicy(Qt::NoFocus);

    auto* layout = new QHBoxLayout(this);
    layout->addWidget(label_, 1);
    layout->addWidget(allow_);
    layout->addWidget(deny_);

    connect(allow_, &QPushButton::clicked, this, [this] {
        hide();
        emit answered(true);
    });
    connect(deny_, &QPushButton::clicked, this, [this] {
        hide();
        emit answered(false);
    });
    hide();
}

void ControlApprovalBar::ask(const QString& question)
{
    label_->setText(question);
    allow_->setEnabled(false);
    QTimer::singleShot(kArmDelayMs, allow_, [button = allow_] { button->setEnabled(true); });
    if (QWidget* window = parentWidget()) {
        adjustSize();
        const int width = qMin(window->width(), qMax(this->width(), 480));
        setGeometry((window->width() - width) / 2, 0, width, sizeHint().height());
        QApplication::alert(window);
    }
    show();
    raise();
}

QString ControlApprovalBar::question() const
{
    return label_->text();
}

} // namespace pictura

#include "moc_control_approval_bar.cpp"
