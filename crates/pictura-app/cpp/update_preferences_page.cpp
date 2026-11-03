#include "update_preferences_page.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include "update_checker.h"
#include "update_package.h"
#include "update_version.h"

namespace pictura {

UpdatePreferencesPage::UpdatePreferencesPage(UpdateChecker* checker, QWidget* parent)
    : QWidget(parent)
    , checker_(checker)
    , automaticChecks_(new QCheckBox(tr("Automatically check for updates on launch"), this))
    , version_(new QLabel(this))
    , package_(new QLabel(this))
    , checkNow_(new QPushButton(tr("Check Now"), this))
    , result_(new QLabel(this))
{
    setObjectName(QStringLiteral("preferencesUpdatesPage"));
    automaticChecks_->setObjectName(QStringLiteral("preferencesCheckForUpdates"));
    version_->setObjectName(QStringLiteral("preferencesUpdateVersion"));
    package_->setObjectName(QStringLiteral("preferencesUpdatePackage"));
    checkNow_->setObjectName(QStringLiteral("preferencesUpdateCheckNow"));
    result_->setObjectName(QStringLiteral("preferencesUpdateResult"));
    result_->setWordWrap(true);

    version_->setText(tr("Current version: %1").arg(QStringLiteral(PICTURA_VERSION)));
    package_->setText(
        tr("Package: %1").arg(packageMethodName(checker_->settings().packageMethod)));

    auto* layout = new QVBoxLayout(this);
    layout->addWidget(automaticChecks_);
    layout->addWidget(version_);
    layout->addWidget(package_);
    auto* checkRow = new QHBoxLayout;
    checkRow->addWidget(checkNow_);
    checkRow->addStretch(1);
    layout->addLayout(checkRow);
    layout->addWidget(result_);
    layout->addStretch(1);

    setAutomaticChecks(checker_->settings().automaticChecksEnabled);
    connect(automaticChecks_, &QCheckBox::toggled, this,
            &UpdatePreferencesPage::automaticChecksChanged);
    connect(checkNow_, &QPushButton::clicked, this, &UpdatePreferencesPage::checkRequested);
    connect(checker_, &UpdateChecker::checkingChanged, this, &UpdatePreferencesPage::refresh);
    connect(checker_, &UpdateChecker::updateAvailable, this, &UpdatePreferencesPage::refresh);
    connect(checker_, &UpdateChecker::upToDate, this, &UpdatePreferencesPage::refresh);
    connect(checker_, &UpdateChecker::failed, this, &UpdatePreferencesPage::refresh);
    refresh();
}

void UpdatePreferencesPage::setAutomaticChecks(bool on)
{
    const QSignalBlocker block(automaticChecks_);
    automaticChecks_->setChecked(on);
}

bool UpdatePreferencesPage::automaticChecks() const
{
    return automaticChecks_->isChecked();
}

QString UpdatePreferencesPage::resultText() const
{
    return result_->text();
}

void UpdatePreferencesPage::refresh()
{
    const bool allowed = checker_->settings().networkAllowed;
    checkNow_->setEnabled(allowed && !checker_->isChecking());
    if (checker_->isChecking()) {
        result_->setText(tr("Checking…"));
        return;
    }
    if (!allowed) {
        result_->setText(tr("Update checks are not available in this run."));
        return;
    }
    if (!checker_->lastError().isEmpty()) {
        result_->setText(tr("Check failed: %1").arg(checker_->lastError()));
        return;
    }
    if (checker_->release()) {
        if (isNewerVersion(checker_->release()->version, QStringLiteral(PICTURA_VERSION))) {
            result_->setText(tr("Update %1 is available.").arg(checker_->release()->version));
        } else {
            result_->setText(tr("Up to date (%1).").arg(QStringLiteral(PICTURA_VERSION)));
        }
        return;
    }
    result_->setText(tr("Never checked."));
}

} // namespace pictura

#include "moc_update_preferences_page.cpp"
