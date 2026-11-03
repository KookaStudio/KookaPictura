#include "update_banner.h"

#include <QtCore/QEvent>
#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

UpdateBanner::UpdateBanner(QWidget* parent)
    : QFrame(parent)
    , title_(new QLabel(this))
    , command_(new QLabel(this))
    , error_(new QLabel(this))
    , progress_(new QProgressBar(this))
    , automaticChecks_(new QCheckBox(tr("Don't check for updates automatically"), this))
    , notes_(new QPushButton(tr("Release Notes"), this))
    , later_(new QPushButton(tr("Later"), this))
    , update_(new QPushButton(tr("Update Now"), this))
    , releasePage_(new QPushButton(tr("Release Page"), this))
    , cancel_(new QPushButton(tr("Cancel"), this))
{
    setObjectName(QStringLiteral("updateBanner"));
    setFrameShape(QFrame::StyledPanel);
    setAutoFillBackground(true);
    setFocusPolicy(Qt::NoFocus);
    setAttribute(Qt::WA_ShowWithoutActivating);

    title_->setObjectName(QStringLiteral("updateBannerTitle"));
    title_->setWordWrap(true);
    title_->setTextFormat(Qt::PlainText);
    command_->setObjectName(QStringLiteral("updateBannerCommand"));
    command_->setWordWrap(true);
    command_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    error_->setObjectName(QStringLiteral("updateBannerError"));
    error_->setWordWrap(true);
    error_->setTextFormat(Qt::PlainText);
    progress_->setObjectName(QStringLiteral("updateBannerProgress"));
    progress_->setTextVisible(true);
    automaticChecks_->setObjectName(QStringLiteral("updateBannerAutomatic"));
    notes_->setObjectName(QStringLiteral("updateBannerNotes"));
    later_->setObjectName(QStringLiteral("updateBannerLater"));
    update_->setObjectName(QStringLiteral("updateBannerUpdate"));
    releasePage_->setObjectName(QStringLiteral("updateBannerReleasePage"));
    cancel_->setObjectName(QStringLiteral("updateBannerCancel"));

    QWidget* const controls[] = {automaticChecks_, notes_, later_, update_, releasePage_,
                                 cancel_};
    for (QWidget* control : controls) {
        control->setFocusPolicy(Qt::NoFocus);
    }

    auto* layout = new QVBoxLayout(this);
    auto* top = new QHBoxLayout;
    top->addWidget(title_, 1);
    top->addWidget(automaticChecks_, 0, Qt::AlignTop);
    layout->addLayout(top);

    auto* buttons = new QHBoxLayout;
    buttons->addWidget(notes_);
    buttons->addStretch(1);
    buttons->addWidget(releasePage_);
    buttons->addWidget(later_);
    buttons->addWidget(update_);
    layout->addLayout(buttons);

    auto* progressRow = new QHBoxLayout;
    progressRow->addWidget(progress_, 1);
    progressRow->addWidget(cancel_);
    layout->addLayout(progressRow);
    layout->addWidget(command_);
    layout->addWidget(error_);

    connect(update_, &QPushButton::clicked, this, [this] {
        if (canInstall_ || command_->text().isEmpty()) {
            emit updateRequested();
        } else {
            emit commandRequested();
        }
    });
    connect(notes_, &QPushButton::clicked, this, &UpdateBanner::releaseNotesRequested);
    connect(releasePage_, &QPushButton::clicked, this, &UpdateBanner::releasePageRequested);
    connect(later_, &QPushButton::clicked, this, [this] {
        hide();
        emit laterRequested();
    });
    connect(cancel_, &QPushButton::clicked, this, [this] {
        resetToAvailable();
        emit cancelRequested();
    });
    connect(automaticChecks_, &QCheckBox::toggled, this,
            &UpdateBanner::automaticChecksChanged);

    hide();
}

void UpdateBanner::showUpdate(const QString& currentVersion, const QString& availableVersion,
                              bool canInstall, const QString& command)
{
    currentVersion_ = currentVersion;
    availableVersion_ = availableVersion;
    canInstall_ = canInstall;
    command_->setText(command);
    resetToAvailable();
    show();
    raise();
}

void UpdateBanner::showProgress(qint64 received, qint64 total)
{
    title_->setText(tr("Downloading Kooka Pictura %1…").arg(availableVersion_));
    progress_->setRange(0, total > 0 ? int(total) : 0);
    progress_->setValue(total > 0 ? int(received) : 0);
    progress_->show();
    cancel_->show();
    notes_->hide();
    later_->hide();
    update_->hide();
    releasePage_->hide();
    command_->hide();
    error_->hide();
    setBusy(true);
    reposition();
    show();
    raise();
}

void UpdateBanner::showError(const QString& message)
{
    title_->setText(tr("The update could not be downloaded."));
    error_->setText(message);
    error_->show();
    progress_->hide();
    cancel_->hide();
    command_->hide();
    notes_->show();
    later_->show();
    releasePage_->show();
    update_->setText(tr("Retry"));
    update_->show();
    setBusy(false);
    reposition();
    show();
    raise();
}

void UpdateBanner::setAutomaticChecks(bool on)
{
    const QSignalBlocker block(automaticChecks_);
    automaticChecks_->setChecked(on);
}

void UpdateBanner::reposition()
{
    QWidget* parent = parentWidget();
    if (!parent) {
        return;
    }
    adjustSize();
    const int width = qMax(1, qMin(parent->width() - 24, qMax(640, sizeHint().width())));
    setGeometry((parent->width() - width) / 2, 8, width, sizeHint().height());
}

QString UpdateBanner::titleText() const
{
    return title_->text();
}

QString UpdateBanner::commandText() const
{
    return command_->text();
}

QString UpdateBanner::errorText() const
{
    return error_->text();
}

bool UpdateBanner::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == parentWidget() && event->type() == QEvent::Resize) {
        reposition();
    }
    return QFrame::eventFilter(watched, event);
}

void UpdateBanner::setBusy(bool busy)
{
    automaticChecks_->setEnabled(!busy);
    notes_->setEnabled(!busy);
    later_->setEnabled(!busy);
    update_->setEnabled(!busy);
    releasePage_->setEnabled(!busy);
}

void UpdateBanner::resetToAvailable()
{
    if (!availableVersion_.isEmpty()) {
        title_->setText(tr("Kooka Pictura %1 is available. You have %2. (%2 → %1)")
                            .arg(availableVersion_, currentVersion_));
    }
    error_->clear();
    error_->hide();
    progress_->hide();
    cancel_->hide();
    command_->setVisible(!command_->text().isEmpty());
    notes_->show();
    later_->show();
    automaticChecks_->show();
    update_->setText(canInstall_ ? tr("Update Now")
                                 : (command_->text().isEmpty() ? tr("Release Page")
                                                               : tr("Show Update Command")));
    update_->setVisible(canInstall_ || !command_->text().isEmpty());
    releasePage_->setVisible(!canInstall_);
    setBusy(false);
    reposition();
}

} // namespace pictura

#include "moc_update_banner.cpp"
