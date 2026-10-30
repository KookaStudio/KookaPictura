// The approval bar: a request waits for a person without stopping the window.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QPushButton>

#include "control_approval_bar.h"
#include "control_policy.h"
#include "control_server.h"
#include "control_test_client.h"
#include "frame.h"
#include "qt_test_support.h"

using pictura::test::ControlClient;
using pictura::test::errorCode;
using pictura::test::succeeded;

namespace {

// Long enough for a reply to arrive if the request were not really waiting.
constexpr int kNoReplyMs = 150;

} // namespace

class ControlApprovalTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void insideARootDoesNotAsk();
    void requestWaitsForApproval();
    void allowRunsTheRequest();
    void denyRefusesTheRequest();
    void unansweredPromptTimesOut();
    void disconnectWithdrawsThePrompt();

private:
    QJsonObject saveAs(const QString& path) const;
    bool barVisible() const;

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    QString root_;
    QString outside_;
    QString fixture_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlApprovalTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    QDir base(QFileInfo(dir_.path()).canonicalFilePath());
    QVERIFY(base.mkpath(QStringLiteral("root")));
    QVERIFY(base.mkpath(QStringLiteral("outside")));
    root_ = base.filePath(QStringLiteral("root"));
    outside_ = base.filePath(QStringLiteral("outside"));
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Approval"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    fixture_ = root_ + QStringLiteral("/fixture.psd");
    QVERIFY(window_->saveActiveAs(fixture_));
}

void ControlApprovalTest::init()
{
    server_ = std::make_unique<pictura::ControlServer>(
        window_.get(), dir_.filePath(QStringLiteral("control.sock")));
    pictura::ControlPolicy policy;
    policy.restricted = true;
    policy.live = true;
    policy.allowEdit = true;
    policy.allowSave = true;
    policy.roots = {root_};
    server_->setPolicy(policy);
    QVERIFY(server_->listen());
}

void ControlApprovalTest::cleanup()
{
    server_.reset();
}

QJsonObject ControlApprovalTest::saveAs(const QString& path) const
{
    return {{QStringLiteral("op"), QStringLiteral("save_as")}, {QStringLiteral("path"), path}};
}

bool ControlApprovalTest::barVisible() const
{
    // Not isVisible(): the window is never shown here, so no child of it would be.
    return server_->approvalBar() && !server_->approvalBar()->isHidden();
}

void ControlApprovalTest::insideARootDoesNotAsk()
{
    ControlClient client(*server_);
    QVERIFY(succeeded(client.call(QStringLiteral("document"),
                                  {{QStringLiteral("op"), QStringLiteral("open")},
                                   {QStringLiteral("path"), fixture_}})));
    QVERIFY(!barVisible());
}

void ControlApprovalTest::requestWaitsForApproval()
{
    const QString path = outside_ + QStringLiteral("/waits.psd");
    ControlClient client(*server_);
    client.send(1, QStringLiteral("document"), saveAs(path));
    client.send(2, QStringLiteral("status"));
    QVERIFY(client.read(kNoReplyMs).isEmpty());
    QVERIFY(barVisible());
    QVERIFY(server_->approvalBar()->question().contains(path));
    QCOMPARE(server_->approvalBar()->focusPolicy(), Qt::NoFocus);
    QVERIFY(!server_->approvalBar()->allowButton()->isEnabled());
    QVERIFY(!QFile::exists(path));
}

void ControlApprovalTest::allowRunsTheRequest()
{
    const QString path = outside_ + QStringLiteral("/allowed.psd");
    ControlClient client(*server_);
    client.send(1, QStringLiteral("document"), saveAs(path));
    client.send(2, QStringLiteral("status"));
    QTRY_VERIFY(barVisible());
    QTRY_VERIFY(server_->approvalBar()->allowButton()->isEnabled());
    server_->approvalBar()->allowButton()->click();

    const QJsonObject first = client.read();
    QCOMPARE(first.value(QStringLiteral("id")).toInt(), 1);
    QVERIFY(succeeded(first));
    const QJsonObject second = client.read();
    QCOMPARE(second.value(QStringLiteral("id")).toInt(), 2);
    QVERIFY(QFile::exists(path));
    QVERIFY(!barVisible());
}

void ControlApprovalTest::denyRefusesTheRequest()
{
    const QString path = outside_ + QStringLiteral("/denied.psd");
    ControlClient client(*server_);
    client.send(1, QStringLiteral("document"), saveAs(path));
    QTRY_VERIFY(barVisible());
    server_->approvalBar()->denyButton()->click();
    QCOMPARE(errorCode(client.read()), QStringLiteral("refused"));
    QVERIFY(!QFile::exists(path));
}

void ControlApprovalTest::unansweredPromptTimesOut()
{
    server_->setApprovalTimeoutMs(50);
    ControlClient client(*server_);
    client.send(1, QStringLiteral("document"), saveAs(outside_ + QStringLiteral("/late.psd")));
    QCOMPARE(errorCode(client.read()), QStringLiteral("approval_timeout"));
    QVERIFY(!barVisible());
}

void ControlApprovalTest::disconnectWithdrawsThePrompt()
{
    {
        ControlClient leaving(*server_);
        leaving.send(1, QStringLiteral("document"), saveAs(outside_ + QStringLiteral("/gone.psd")));
        QTRY_VERIFY(barVisible());
    }
    QTRY_VERIFY(!barVisible());
    ControlClient next(*server_);
    QVERIFY(succeeded(next.call(QStringLiteral("status"))));
}

QTEST_MAIN(ControlApprovalTest)
#include "tst_control_approval.moc"
