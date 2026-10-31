#include "preferences_agent_page.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSignalBlocker>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

#include "agent_control.h"
#include "control_server.h"

namespace pictura {

namespace {

QString clientSnippet()
{
    const QJsonObject server{{QStringLiteral("command"), AgentControl::launchCommand()},
                             {QStringLiteral("args"), QJsonArray{QStringLiteral("--mcp")}}};
    const QJsonObject root{
        {QStringLiteral("mcpServers"), QJsonObject{{QStringLiteral("pictura"), server}}}};
    return QString::fromUtf8(QJsonDocument(root).toJson(QJsonDocument::Indented));
}

QLabel* wrappedLabel(const QString& text, QWidget* parent)
{
    auto* label = new QLabel(text, parent);
    label->setWordWrap(true);
    return label;
}

} // namespace

AgentControlPage::AgentControlPage(AgentControl* agent, QWidget* parent)
    : QWidget(parent)
    , agent_(agent)
    , enable_(new QCheckBox(tr("Enable Agent Control"), this))
    , status_(new QLabel(this))
    , allowEdit_(new QCheckBox(tr("Agents May Edit Documents"), this))
    , allowSave_(new QCheckBox(tr("Agents May Save and Export Files"), this))
    , approval_(new QComboBox(this))
    , allowAllWarning_(wrappedLabel(
          tr("Allow All never asks. An agent can read and overwrite any file you can, "
             "except the app's own settings."),
          this))
    , folders_(new QListWidget(this))
    , addFolder_(new QPushButton(tr("Add…"), this))
    , removeFolder_(new QPushButton(tr("Remove"), this))
    , snippet_(new QPlainTextEdit(this))
    , copySnippet_(new QPushButton(tr("Copy"), this))
{
    setObjectName(QStringLiteral("preferencesAgentPage"));
    enable_->setObjectName(QStringLiteral("agentEnable"));
    status_->setObjectName(QStringLiteral("agentStatus"));
    status_->setWordWrap(true);
    allowEdit_->setObjectName(QStringLiteral("agentAllowEdit"));
    allowSave_->setObjectName(QStringLiteral("agentAllowSave"));
    approval_->setObjectName(QStringLiteral("agentApproval"));
    allowAllWarning_->setObjectName(QStringLiteral("agentAllowAllWarning"));
    folders_->setObjectName(QStringLiteral("agentFolders"));
    snippet_->setObjectName(QStringLiteral("agentSnippet"));
    snippet_->setReadOnly(true);

    approval_->addItem(tr("Manual: Ask for Every Overwrite"), int(ApprovalMode::Manual));
    approval_->addItem(tr("Auto-Approve Files the Agent Made"), int(ApprovalMode::AutoApprove));
    approval_->addItem(tr("Allow All"), int(ApprovalMode::AllowAll));
    snippet_->setPlainText(clientSnippet());

    auto* layout = new QVBoxLayout(this);
    layout->addWidget(enable_);
    layout->addWidget(status_);

    auto* permissions = new QGroupBox(tr("Permissions"), this);
    auto* permissionsForm = new QFormLayout(permissions);
    permissionsForm->addRow(allowEdit_);
    permissionsForm->addRow(allowSave_);
    permissionsForm->addRow(tr("Approval:"), approval_);
    permissionsForm->addRow(allowAllWarning_);
    layout->addWidget(permissions);

    auto* folders = new QGroupBox(tr("Allowed Folders"), this);
    auto* foldersLayout = new QHBoxLayout(folders);
    foldersLayout->addWidget(folders_, 1);
    auto* folderButtons = new QVBoxLayout;
    folderButtons->addWidget(addFolder_);
    folderButtons->addWidget(removeFolder_);
    folderButtons->addStretch(1);
    foldersLayout->addLayout(folderButtons);
    layout->addWidget(folders);

    auto* client = new QGroupBox(tr("MCP Client Configuration"), this);
    auto* clientLayout = new QHBoxLayout(client);
    clientLayout->addWidget(snippet_, 1);
    clientLayout->addWidget(copySnippet_, 0, Qt::AlignTop);
    layout->addWidget(client);

    auto* limits = wrappedLabel(
        tr("These permissions limit agents using this connection. They do not limit an "
           "agent that also has a shell on this computer. Agents wait while this window "
           "is open."),
        this);
    limits->setObjectName(QStringLiteral("agentLimits"));
    layout->addWidget(limits);
    layout->addStretch(1);

    showSettings();
    showStatus();

    for (QCheckBox* box : {enable_, allowEdit_, allowSave_}) {
        connect(box, &QCheckBox::toggled, this, &AgentControlPage::applyControls);
    }
    connect(approval_, &QComboBox::currentIndexChanged, this, &AgentControlPage::applyControls);
    connect(addFolder_, &QPushButton::clicked, this, [this] {
        const QString path = QFileDialog::getExistingDirectory(this, tr("Allow Folder"));
        if (!path.isEmpty()) {
            addFolder(path);
        }
    });
    connect(removeFolder_, &QPushButton::clicked, this, [this] {
        delete folders_->currentItem();
        applyControls();
    });
    connect(copySnippet_, &QPushButton::clicked, this,
            [this] { QGuiApplication::clipboard()->setText(snippet_->toPlainText()); });
    connect(agent_, &AgentControl::statusChanged, this, &AgentControlPage::showStatus);
}

void AgentControlPage::addFolder(const QString& path)
{
    if (folders_->findItems(path, Qt::MatchExactly).isEmpty()) {
        folders_->addItem(path);
        applyControls();
    }
}

void AgentControlPage::applyControls()
{
    AgentSettings settings;
    settings.enabled = enable_->isChecked();
    settings.allowEdit = allowEdit_->isChecked();
    settings.allowSave = settings.allowEdit && allowSave_->isChecked();
    settings.approval = static_cast<ApprovalMode>(approval_->currentData().toInt());
    for (int i = 0; i < folders_->count(); ++i) {
        settings.roots.append(folders_->item(i)->text());
    }
    agent_->apply(settings);
    showSettings();
    showStatus();
}

void AgentControlPage::showSettings()
{
    const AgentSettings& settings = agent_->settings();
    const QSignalBlocker blockEnable(enable_);
    const QSignalBlocker blockEdit(allowEdit_);
    const QSignalBlocker blockSave(allowSave_);
    const QSignalBlocker blockApproval(approval_);
    enable_->setChecked(settings.enabled);
    allowEdit_->setChecked(settings.allowEdit);
    allowSave_->setChecked(settings.allowEdit && settings.allowSave);
    allowSave_->setEnabled(settings.allowEdit);
    approval_->setCurrentIndex(approval_->findData(int(settings.approval)));
    allowAllWarning_->setVisible(settings.approval == ApprovalMode::AllowAll);
    if (folders_->count() != settings.roots.size()) {
        folders_->clear();
        folders_->addItems(settings.roots);
    }
    if (AgentControl::isSandboxed()) {
        for (QWidget* control : QList<QWidget*>{enable_, allowEdit_, allowSave_, approval_,
                                                folders_, addFolder_, removeFolder_,
                                                copySnippet_, snippet_}) {
            control->setEnabled(false);
        }
    }
}

void AgentControlPage::showStatus()
{
    if (AgentControl::isSandboxed()) {
        status_->setText(tr("Agent Control is not available in this sandboxed build."));
    } else if (!agent_->problem().isEmpty()) {
        status_->setText(agent_->problem());
    } else if (!agent_->isRunning()) {
        status_->setText(tr("Off."));
    } else {
        status_->setText(tr("Listening. Connected agents: %1")
                             .arg(agent_->server()->clientCount()));
    }
}

} // namespace pictura

#include "moc_preferences_agent_page.cpp"
