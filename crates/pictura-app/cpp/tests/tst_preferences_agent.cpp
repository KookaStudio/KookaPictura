// Edit > Preferences > Agent Control.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>

#include "agent_control.h"
#include "commands.h"
#include "control_approval_bar.h"
#include "control_server.h"
#include "control_test_client.h"
#include "frame.h"
#include "preferences_agent_page.h"
#include "preferences_dialog.h"
#include "qt_test_support.h"

using pictura::AgentControl;
using pictura::AgentControlPage;
using pictura::AgentSettings;

class PreferencesAgentTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void enablingStartsTheServer();
    void permissionsApplyAtOnce();
    void saveNeedsEdit();
    void allowAllWarns();
    void foldersPersist();
    void clientSnippetIsValid();
    void limitsAreStated();
    void sandboxedBuildsExplainThemselves();
    void stoppingWithdrawsAnApproval();
    void menuOpensThePage();

private:
    template <typename T>
    T* child(const char* name) const
    {
        return page_->findChild<T*>(QString::fromLatin1(name));
    }

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    QString endpoint_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    AgentControl* agent_ = nullptr;
    std::unique_ptr<AgentControlPage> page_;
};

void PreferencesAgentTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    QVERIFY(QDir(dir_.path()).mkpath(QStringLiteral("endpoint")));
    endpoint_ = dir_.filePath(QStringLiteral("endpoint/live.sock"));
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Agent"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
}

void PreferencesAgentTest::init()
{
    agent_ = new AgentControl(window_.get(), endpoint_);
    agent_->apply(AgentSettings{});
    page_ = std::make_unique<AgentControlPage>(agent_);
}

void PreferencesAgentTest::cleanup()
{
    page_.reset();
    delete agent_;
    agent_ = nullptr;
    qunsetenv("FLATPAK_ID");
}

void PreferencesAgentTest::enablingStartsTheServer()
{
    const auto* status = child<QLabel>("agentStatus");
    QVERIFY(!agent_->isRunning());
    child<QCheckBox>("agentEnable")->setChecked(true);
    QVERIFY(agent_->isRunning());
    QVERIFY(status->text().contains(QStringLiteral("Listening")));
    {
        pictura::test::ControlClient client(*agent_->server());
        QVERIFY(pictura::test::succeeded(client.call(QStringLiteral("status"))));
        QTRY_VERIFY(status->text().endsWith(QLatin1Char('1')));
    }
    child<QCheckBox>("agentEnable")->setChecked(false);
    QVERIFY(!agent_->isRunning());
    QVERIFY(status->text().contains(QStringLiteral("Off")));
}

void PreferencesAgentTest::permissionsApplyAtOnce()
{
    child<QCheckBox>("agentEnable")->setChecked(true);
    child<QCheckBox>("agentAllowEdit")->setChecked(true);
    child<QCheckBox>("agentAllowSave")->setChecked(true);
    auto* approval = child<QComboBox>("agentApproval");
    approval->setCurrentIndex(approval->findData(int(pictura::ApprovalMode::Manual)));

    const pictura::ControlPolicy& policy = agent_->server()->policy();
    QVERIFY(policy.allowEdit);
    QVERIFY(policy.allowSave);
    QCOMPARE(policy.approval, pictura::ApprovalMode::Manual);

    page_ = std::make_unique<AgentControlPage>(agent_);
    QVERIFY(child<QCheckBox>("agentAllowSave")->isChecked());
    QCOMPARE(child<QComboBox>("agentApproval")->currentData().toInt(),
             int(pictura::ApprovalMode::Manual));
}

void PreferencesAgentTest::saveNeedsEdit()
{
    auto* save = child<QCheckBox>("agentAllowSave");
    QVERIFY(!save->isEnabled());
    child<QCheckBox>("agentAllowEdit")->setChecked(true);
    QVERIFY(save->isEnabled());
    save->setChecked(true);
    child<QCheckBox>("agentAllowEdit")->setChecked(false);
    QVERIFY(!save->isChecked());
    QVERIFY(!save->isEnabled());
    QVERIFY(!agent_->settings().allowSave);
}

void PreferencesAgentTest::allowAllWarns()
{
    auto* approval = child<QComboBox>("agentApproval");
    const auto* warning = child<QLabel>("agentAllowAllWarning");
    QVERIFY(warning->isHidden());
    approval->setCurrentIndex(approval->findData(int(pictura::ApprovalMode::AllowAll)));
    QVERIFY(!warning->isHidden());
    approval->setCurrentIndex(approval->findData(int(pictura::ApprovalMode::Manual)));
    QVERIFY(warning->isHidden());
}

void PreferencesAgentTest::foldersPersist()
{
    page_->addFolder(QStringLiteral("/one"));
    page_->addFolder(QStringLiteral("/two"));
    page_->addFolder(QStringLiteral("/one"));
    QCOMPARE(agent_->settings().roots, (QStringList{QStringLiteral("/one"), QStringLiteral("/two")}));
    auto* folders = child<QListWidget>("agentFolders");
    folders->setCurrentRow(0);
    const auto buttons = page_->findChildren<QPushButton*>();
    for (QPushButton* button : buttons) {
        if (button->text() == QStringLiteral("Remove")) {
            button->click();
        }
    }
    QCOMPARE(agent_->settings().roots, QStringList{QStringLiteral("/two")});
}

void PreferencesAgentTest::clientSnippetIsValid()
{
    const QJsonDocument document =
        QJsonDocument::fromJson(child<QPlainTextEdit>("agentSnippet")->toPlainText().toUtf8());
    const QJsonObject server = document.object()
                                   .value(QStringLiteral("mcpServers")).toObject()
                                   .value(QStringLiteral("pictura")).toObject();
    QCOMPARE(server.value(QStringLiteral("command")).toString(), AgentControl::launchCommand());
    QCOMPARE(server.value(QStringLiteral("args")).toArray().first().toString(),
             QStringLiteral("--mcp"));
}

void PreferencesAgentTest::limitsAreStated()
{
    QVERIFY(child<QLabel>("agentLimits")->text().contains(QStringLiteral("shell")));
}

void PreferencesAgentTest::sandboxedBuildsExplainThemselves()
{
    qputenv("FLATPAK_ID", "io.github.KookaStudio.KookaPictura");
    page_ = std::make_unique<AgentControlPage>(agent_);
    QVERIFY(!child<QCheckBox>("agentEnable")->isEnabled());
    QVERIFY(!child<QComboBox>("agentApproval")->isEnabled());
    QVERIFY(child<QLabel>("agentStatus")->text().contains(QStringLiteral("sandboxed")));
}

void PreferencesAgentTest::stoppingWithdrawsAnApproval()
{
    AgentSettings settings;
    settings.enabled = true;
    settings.allowEdit = true;
    settings.allowSave = true;
    agent_->apply(settings);
    pictura::ControlApprovalBar* bar = nullptr;
    {
        pictura::test::ControlClient client(*agent_->server());
        client.send(1, QStringLiteral("document"),
                    {{QStringLiteral("op"), QStringLiteral("save_as")},
                     {QStringLiteral("path"), dir_.filePath(QStringLiteral("out.psd"))}});
        QTRY_VERIFY(agent_->server()->approvalBar() != nullptr);
        bar = agent_->server()->approvalBar();
        QTRY_VERIFY(!bar->isHidden());
        agent_->apply(AgentSettings{});
    }
    QVERIFY(bar->isHidden());
}

void PreferencesAgentTest::menuOpensThePage()
{
    QVERIFY(window_->registry()->dispatch(QStringLiteral("edit.preferences.agentControl")));
    auto* dialog = window_->findChild<pictura::PreferencesDialog*>();
    QVERIFY(dialog != nullptr);
    QCOMPARE(dialog->currentPageForTest(), pictura::PreferencesDialog::kAgentControl);
    QVERIFY(dialog->findChild<AgentControlPage*>() != nullptr);
    dialog->close();
}

QTEST_MAIN(PreferencesAgentTest)
#include "tst_preferences_agent.moc"
