#include "loading_card.h"

#include <QtCore/QtMath>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

LoadingCard::LoadingCard(const QString& fileName, QWidget* parent)
    : QFrame(parent)
{
    setObjectName(QStringLiteral("loadingCard"));
    setFrameShape(QFrame::NoFrame);
    setAttribute(Qt::WA_StyledBackground, true);
    setMaximumWidth(380);
    setMinimumWidth(280);
    setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);

    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(16, 14, 16, 14);
    layout->setSpacing(8);

    auto* name = new QLabel(fileName, this);
    name->setObjectName(QStringLiteral("loadingCardName"));
    name->setWordWrap(true);
    QFont nameFont = name->font();
    nameFont.setBold(true);
    name->setFont(nameFont);

    phase_ = new QLabel(tr("Starting…"), this);
    phase_->setObjectName(QStringLiteral("loadingCardPhase"));
    phase_->setWordWrap(true);

    bar_ = new QProgressBar(this);
    bar_->setObjectName(QStringLiteral("loadingCardBar"));
    bar_->setRange(0, 100);
    bar_->setValue(0);
    bar_->setTextVisible(false);
    bar_->setFixedHeight(6);

    auto* footer = new QHBoxLayout();
    footer->setContentsMargins(0, 0, 0, 0);
    footer->setSpacing(8);
    percent_ = new QLabel(QStringLiteral("0%"), this);
    percent_->setObjectName(QStringLiteral("loadingCardPercent"));
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setObjectName(QStringLiteral("loadingCardCancel"));
    footer->addWidget(percent_);
    footer->addStretch();
    footer->addWidget(cancel);

    layout->addWidget(name);
    layout->addWidget(phase_);
    layout->addWidget(bar_);
    layout->addLayout(footer);

    connect(cancel, &QPushButton::clicked, this, &LoadingCard::cancelRequested);
}

void LoadingCard::setProgress(int percent, const QString& phase)
{
    const int value = qBound(0, percent, 100);
    bar_->setValue(value);
    percent_->setText(QStringLiteral("%1%").arg(value));
    if (!phase.isEmpty()) {
        phase_->setText(phase + QStringLiteral("…"));
    }
    setAccessibleDescription(tr("%1, %2%").arg(phase_->text(), QString::number(value)));
}

int LoadingCard::progressForTest() const
{
    return bar_->value();
}

QString LoadingCard::phaseForTest() const
{
    return phase_->text();
}

QString LoadingCard::percentForTest() const
{
    return percent_->text();
}

} // namespace pictura

#include "moc_loading_card.cpp"
