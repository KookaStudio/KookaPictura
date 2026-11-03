#include "update_banner.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QFont>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

UpdateBanner::UpdateBanner(QWidget* parent)
    : QDialog(parent)
    , title_(new QLabel(this))
    , command_(new QLabel(this))
    , error_(new QLabel(this))
    , progress_(new QProgressBar(this))
    , automaticChecks_(new QCheckBox(tr("Check for updates automatically"), this))
    , notes_(new QPushButton(tr("Release Notes"), this))
    , later_(new QPushButton(tr("Later"), this))
    , update_(new QPushButton(tr("Update now"), this))
    , releasePage_(new QPushButton(tr("Release Page"), this))
    , cancel_(new QPushButton(tr("Cancel"), this))
{
    setObjectName(QStringLiteral("updateBanner"));
    setWindowTitle(tr("Update Available"));
    setModal(false);
    setWindowModality(Qt::NonModal);
    setAttribute(Qt::WA_ShowWithoutActivating);
    setSizeGripEnabled(false);
    setWindowFlag(Qt::WindowContextHelpButtonHint, false);
    setFixedWidth(560);

    title_->setObjectName(QStringLiteral("updateBannerTitle"));
    title_->setWordWrap(true);
    title_->setTextFormat(Qt::PlainText);
    QFont titleFont = title_->font();
    titleFont.setBold(true);
    title_->setFont(titleFont);

    command_->setObjectName(QStringLiteral("updateBannerCommand"));
    command_->setWordWrap(true);
    command_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    error_->setObjectName(QStringLiteral("updateBannerError"));
    error_->setWordWrap(true);
    error_->setTextFormat(Qt::PlainText);
    progress_->setObjectName(QStringLiteral("updateBannerProgress"));
    progress_->setTextVisible(true);
    automaticChecks_->setObjectName(QStringLiteral("updateBannerAutomatic"));
    automaticChecks_->setChecked(true);
    notes_->setObjectName(QStringLiteral("updateBannerNotes"));
    later_->setObjectName(QStringLiteral("updateBannerLater"));
    update_->setObjectName(QStringLiteral("updateBannerUpdate"));
    releasePage_->setObjectName(QStringLiteral("updateBannerReleasePage"));
    cancel_->setObjectName(QStringLiteral("updateBannerCancel"));

    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(16, 16, 16, 12);
    layout->setSpacing(8);
    layout->addWidget(title_);
    layout->addWidget(command_);
    layout->addWidget(error_);
    layout->addWidget(automaticChecks_);

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

    connect(update_, &QPushButton::clicked, this, [this] { emit updateRequested(); });
    connect(notes_, &QPushButton::clicked, this, [this] { emit releaseNotesRequested(); });
    connect(releasePage_, &QPushButton::clicked, this,
            [this] { emit releasePageRequested(); });
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

    resetToAvailable();
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
    centerOnParent();
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
    adjustSize();
    show();
    raise();
    centerOnParent();
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
    adjustSize();
    show();
    raise();
    centerOnParent();
}

void UpdateBanner::setAutomaticChecks(bool on)
{
    const QSignalBlocker block(automaticChecks_);
    automaticChecks_->setChecked(on);
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
    update_->setText(tr("Update now"));
    update_->show();
    releasePage_->setVisible(!canInstall_);
    setBusy(false);
    adjustSize();
}

void UpdateBanner::centerOnParent()
{
    QWidget* parent = parentWidget();
    if (!parent) {
        return;
    }
    const QRect frame = parent->window()->frameGeometry();
    move(frame.center() - rect().center());
}

} // namespace pictura

#include "moc_update_banner.cpp"
