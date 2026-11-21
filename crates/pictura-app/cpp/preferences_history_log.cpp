#include "preferences_dialog.h"

#include "dialogs.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QPushButton>

namespace pictura {

void PreferencesDialog::addHistoryLogControls(QFormLayout* form, QWidget* page)
{
    historyLogEnabled_ = new QCheckBox(tr("History Log"), page);
    historyLogEnabled_->setObjectName(QStringLiteral("preferencesHistoryLog"));
    form->addRow(historyLogEnabled_);

    historyLogDestination_ = new QComboBox(page);
    historyLogDestination_->setObjectName(QStringLiteral("historyLogDestination"));
    historyLogDestination_->addItems({tr("Metadata"), tr("Text File"), tr("Both")});
    form->addRow(tr("Save Log Items To"), historyLogDestination_);

    historyLogChoose_ = new QPushButton(tr("Choose…"), page);
    historyLogChoose_->setEnabled(false);
    form->addRow(tr("History Log File"), historyLogChoose_);

    historyLogVerbosity_ = new QComboBox(page);
    historyLogVerbosity_->setObjectName(QStringLiteral("historyLogVerbosity"));
    historyLogVerbosity_->addItems({tr("Sessions Only"), tr("Concise"), tr("Detailed")});
    form->addRow(tr("Edit Log Items"), historyLogVerbosity_);

    connect(historyLogEnabled_, &QCheckBox::toggled, this, &PreferencesDialog::updateHistoryLog);
    connect(historyLogDestination_, &QComboBox::currentIndexChanged,
            this, &PreferencesDialog::updateHistoryLog);
    connect(historyLogVerbosity_, &QComboBox::currentIndexChanged,
            this, &PreferencesDialog::updateHistoryLog);
    connect(historyLogChoose_, &QPushButton::clicked, this, [this] {
        if (chooseHistoryLogPath()) {
            updateHistoryLog();
        }
    });
}

void PreferencesDialog::setHistoryLogSettings(const HistoryLogSettings& settings)
{
    historyLogSettings_ = settings;
    const QSignalBlocker enabledBlocker(historyLogEnabled_);
    const QSignalBlocker destinationBlocker(historyLogDestination_);
    const QSignalBlocker verbosityBlocker(historyLogVerbosity_);
    historyLogEnabled_->setChecked(settings.enabled);
    historyLogDestination_->setCurrentIndex(settings.destination);
    historyLogVerbosity_->setCurrentIndex(settings.verbosity);
    historyLogChoose_->setEnabled(settings.destination != 0);
    historyLogChoose_->setToolTip(settings.path);
}

bool PreferencesDialog::chooseHistoryLogPath()
{
    QString filter;
    const QString path = getSaveFileName(this, tr("Choose History Log File"),
        historyLogSettings_.path, {tr("Text Files (*.txt)"), tr("All Files (*)")}, &filter, false);
    if (path.isEmpty()) {
        return false;
    }
    historyLogSettings_.path = path;
    return true;
}

void PreferencesDialog::updateHistoryLog()
{
    const int destination = historyLogDestination_->currentIndex();
    if (destination != 0 && historyLogSettings_.path.isEmpty() && !chooseHistoryLogPath()) {
        setHistoryLogSettings(historyLogSettings_);
        return;
    }
    historyLogSettings_.enabled = historyLogEnabled_->isChecked();
    historyLogSettings_.destination = destination;
    historyLogSettings_.verbosity = historyLogVerbosity_->currentIndex();
    historyLogChoose_->setEnabled(destination != 0);
    historyLogChoose_->setToolTip(historyLogSettings_.path);
    emit historyLogChanged(historyLogSettings_);
}

} // namespace pictura
