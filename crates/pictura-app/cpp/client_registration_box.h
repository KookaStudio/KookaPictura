#pragma once

#include <QtCore/QList>
#include <QtWidgets/QGroupBox>

#include <functional>

#include "client_registration.h"

class QLabel;
class QProcess;
class QPushButton;

namespace pictura {

// One row per MCP client on the Agent Control page. Clicking is the person's
// consent to look; `confirm` is their consent to write.
class ClientRegistrationBox : public QGroupBox {
    Q_OBJECT

public:
    // Answers a question with its details. Replaceable for tests.
    using Confirm = std::function<bool(const QString& question, const QString& details)>;

    ClientRegistrationBox(QList<McpClient> clients, const QString& command,
                          QWidget* parent = nullptr);

    void setConfirm(Confirm confirm) { confirm_ = std::move(confirm); }
    QString rowStatus(int row) const;
    QPushButton* rowButton(int row) const { return rows_.at(row).button; }

private:
    struct Row {
        McpClient client;
        QLabel* status = nullptr;
        QPushButton* button = nullptr;
    };

    void refreshRow(int row);
    void toggleRow(int row);
    void changeThroughFile(int row, bool registered, EntryState state);
    void changeThroughCli(int row, bool registered);

    QList<Row> rows_;
    QString command_;
    Confirm confirm_;
};

} // namespace pictura
