#include "client_registration_box.h"

#include <QtCore/QProcess>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

namespace pictura {

ClientRegistrationBox::ClientRegistrationBox(QList<McpClient> clients, const QString& command,
                                             QWidget* parent)
    : QGroupBox(tr("Register with Clients"), parent)
    , command_(command)
{
    setObjectName(QStringLiteral("agentClients"));
    confirm_ = [this](const QString& question, const QString& details) {
        QMessageBox box(QMessageBox::Question, tr("Agent Control"), question,
                        QMessageBox::Ok | QMessageBox::Cancel, this);
        box.setDetailedText(details);
        return box.exec() == QMessageBox::Ok;
    };
    auto* layout = new QGridLayout(this);
    for (const McpClient& client : clients) {
        const int index = rows_.size();
        Row row;
        row.client = client;
        row.status = new QLabel(this);
        row.status->setWordWrap(true);
        row.button = new QPushButton(this);
        layout->addWidget(new QLabel(client.name, this), index, 0);
        layout->addWidget(row.status, index, 1);
        layout->addWidget(row.button, index, 2);
        layout->setColumnStretch(1, 1);
        rows_.append(row);
        connect(row.button, &QPushButton::clicked, this, [this, index] { toggleRow(index); });
        refreshRow(index);
    }
}

QString ClientRegistrationBox::rowStatus(int row) const
{
    return rows_.at(row).status->text();
}

void ClientRegistrationBox::refreshRow(int index)
{
    const Row& row = rows_.at(index);
    if (row.client.cliMissing) {
        row.status->setText(tr("Its command-line tool was not found. Use the configuration above."));
        row.button->setText(tr("Register"));
        row.button->setEnabled(false);
        return;
    }
    const EntryState state = registrationState(row.client.configPath, command_);
    const bool throughFile = row.client.cliProgram.isEmpty();
    if (state == EntryState::Different && throughFile) {
        row.button->setText(tr("Replace"));
    } else {
        const bool present = state == EntryState::Same || state == EntryState::Different;
        row.button->setText(present ? tr("Unregister") : tr("Register"));
    }
    row.button->setEnabled(state != EntryState::Unreadable);
    switch (state) {
    case EntryState::Absent:
        row.status->setText(tr("Not registered."));
        break;
    case EntryState::Same:
        row.status->setText(tr("Registered."));
        break;
    case EntryState::Different:
        row.status->setText(tr("Registered with another Kooka Pictura."));
        break;
    case EntryState::Unreadable:
        row.status->setText(tr("Its configuration is not plain JSON, so it is left alone."));
        break;
    }
}

void ClientRegistrationBox::toggleRow(int index)
{
    const Row& row = rows_.at(index);
    const EntryState state = registrationState(row.client.configPath, command_);
    if (row.client.cliProgram.isEmpty()) {
        changeThroughFile(index, state == EntryState::Same, state);
    } else {
        // Its `add` refuses an existing name, so another entry is removed first.
        changeThroughCli(index, state != EntryState::Absent);
    }
}

void ClientRegistrationBox::changeThroughFile(int index, bool registered, EntryState state)
{
    const McpClient& client = rows_.at(index).client;
    QString question = registered
        ? tr("Remove Kooka Pictura from %1?\n\n%2").arg(client.name, client.configPath)
        : tr("Add Kooka Pictura to %1?\n\n%2").arg(client.name, client.configPath);
    if (state == EntryState::Different) {
        question = tr("%1 already has a different Kooka Pictura entry. Replace it?\n\n%2")
                       .arg(client.name, client.configPath);
    }
    const ConfigChange change =
        registered ? planUnregister(client.configPath) : planRegister(client.configPath, command_);
    if (!change.problem.isEmpty()) {
        QMessageBox::warning(this, tr("Agent Control"), change.problem);
        return;
    }
    if (!confirm_(question, QString::fromUtf8(change.after))) {
        return;
    }
    QString problem;
    if (!applyChange(client.configPath, change, &problem)) {
        QMessageBox::warning(this, tr("Agent Control"), problem);
    }
    refreshRow(index);
}

void ClientRegistrationBox::changeThroughCli(int index, bool registered)
{
    Row& row = rows_[index];
    const QStringList arguments = registered
        ? QStringList{QStringLiteral("mcp"), QStringLiteral("remove"), QStringLiteral("--scope"),
                      QStringLiteral("user"), QStringLiteral("pictura")}
        : QStringList{QStringLiteral("mcp"), QStringLiteral("add"), QStringLiteral("--scope"),
                      QStringLiteral("user"), QStringLiteral("pictura"), QStringLiteral("--"),
                      command_, QStringLiteral("--mcp")};
    const QString shown = row.client.cliProgram + QLatin1Char(' ') + arguments.join(QLatin1Char(' '));
    if (!confirm_(tr("Run this to change %1?").arg(row.client.name), shown)) {
        return;
    }
    // Not waited on: a client's tool can be slow, and this is the GUI thread.
    auto* process = new QProcess(this);
    row.button->setEnabled(false);
    row.status->setText(tr("Working…"));
    const auto finish = [this, index, process](const QString& failure) {
        refreshRow(index);
        if (!failure.isEmpty()) {
            rows_.at(index).status->setText(failure);
        }
        process->deleteLater();
    };
    connect(process, &QProcess::errorOccurred, this, [finish, process](QProcess::ProcessError) {
        if (process->state() == QProcess::NotRunning) {
            finish(tr("It could not be run."));
        }
    });
    connect(process, &QProcess::finished, this, [finish](int code, QProcess::ExitStatus status) {
        finish(code == 0 && status == QProcess::NormalExit ? QString() : tr("It reported a failure."));
    });
    process->start(row.client.cliProgram, arguments);
}

} // namespace pictura

#include "moc_client_registration_box.cpp"
