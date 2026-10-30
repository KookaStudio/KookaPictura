// Who may do what: permission tiers, the command allowlist, and path verdicts.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>

#ifndef _WIN32
#include <sys/stat.h>
#endif

#include "commands.h"
#include "control_policy.h"
#include "control_server.h"
#include "control_test_client.h"
#include "frame.h"
#include "qt_test_support.h"

using pictura::ApprovalMode;
using pictura::ControlPolicy;
using pictura::ControlTier;
using pictura::PathVerdict;
using pictura::test::ControlClient;
using pictura::test::errorCode;
using pictura::test::succeeded;

Q_DECLARE_METATYPE(pictura::ApprovalMode)
Q_DECLARE_METATYPE(pictura::PathVerdict)

namespace {

ControlPolicy restricted(bool edit, bool save)
{
    ControlPolicy policy;
    policy.restricted = true;
    policy.allowEdit = edit;
    policy.allowSave = save;
    return policy;
}

const QJsonObject kFilter{{QStringLiteral("kind"), QStringLiteral("invert")}};

} // namespace

class ControlPolicyTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void socketIsUserOnly();
    void noEvalMethodExists();
    void oversizedRequestIsRejected();
    void readOnlyRefusesEdits();
    void editWithoutSaveRefusesSaving();
    void everyMethodHasATier();
    void adminMethodsAreRefused();
    void commandsAreDefaultDeny();
    void bareControlIsUnchanged();
    void headlessRefusesOutsideItsRoots();
    void symlinkDoesNotEscape();
    void siblingPrefixIsNotInside();
    void protectedDirectoriesAreRefused();
    void approvalModesDecide_data();
    void approvalModesDecide();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlPolicyTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Policy"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
}

void ControlPolicyTest::init()
{
    server_ = std::make_unique<pictura::ControlServer>(
        window_.get(), dir_.filePath(QStringLiteral("control.sock")));
    QVERIFY(server_->listen());
}

void ControlPolicyTest::cleanup()
{
    server_.reset();
}

void ControlPolicyTest::socketIsUserOnly()
{
#ifdef _WIN32
    QSKIP("a named pipe has no file mode");
#else
    struct stat info{};
    QCOMPARE(::stat(server_->socketPath().toLocal8Bit().constData(), &info), 0);
    QCOMPARE(info.st_mode & 0077, 0u);
#endif
}

void ControlPolicyTest::noEvalMethodExists()
{
    ControlClient client(*server_);
    QCOMPARE(errorCode(client.call(QStringLiteral("eval"))), QStringLiteral("unknown_method"));
    server_->setPolicy(restricted(true, true));
    QCOMPARE(errorCode(client.call(QStringLiteral("eval"))), QStringLiteral("unknown_method"));
}

void ControlPolicyTest::oversizedRequestIsRejected()
{
    ControlClient client(*server_);
    client.sendRaw(QByteArray(pictura::ControlServer::kMaxRequestBytes + 1, 'x'));
    QCOMPARE(errorCode(client.read()), QStringLiteral("bad_request"));
}

void ControlPolicyTest::readOnlyRefusesEdits()
{
    server_->setPolicy(restricted(false, false));
    ControlClient client(*server_);
    QCOMPARE(errorCode(client.call(QStringLiteral("filter"), kFilter)), QStringLiteral("refused"));
    QVERIFY(succeeded(client.call(QStringLiteral("status"))));
}

void ControlPolicyTest::editWithoutSaveRefusesSaving()
{
    server_->setPolicy(restricted(true, false));
    const QString path = dir_.filePath(QStringLiteral("no.psd"));
    ControlClient client(*server_);
    QCOMPARE(errorCode(client.call(QStringLiteral("document"),
                                   {{QStringLiteral("op"), QStringLiteral("save_as")},
                                    {QStringLiteral("path"), path}})),
             QStringLiteral("refused"));
    QVERIFY(!QFile::exists(path));

    server_->setPolicy(restricted(false, true));
    QVERIFY(!server_->policy().grants(ControlTier::Save));
}

void ControlPolicyTest::everyMethodHasATier()
{
    const QStringList names = pictura::ControlServer::methodNames();
    QCOMPARE(names.size(), 21);
    QCOMPARE(server_->tierOf(QStringLiteral("key"), {}), ControlTier::Admin);
    QCOMPARE(server_->tierOf(QStringLiteral("set_unsaved_policy"), {}), ControlTier::Admin);
    QCOMPARE(server_->tierOf(QStringLiteral("status"), {}), ControlTier::Read);
    QCOMPARE(server_->tierOf(QStringLiteral("filter"), {}), ControlTier::Edit);
    QCOMPARE(server_->tierOf(QStringLiteral("document"),
                             {{QStringLiteral("op"), QStringLiteral("save")}}),
             ControlTier::Save);
    QCOMPARE(server_->tierOf(QStringLiteral("pointer"),
                             {{QStringLiteral("space"), QStringLiteral("image")}}),
             ControlTier::Edit);
    QCOMPARE(server_->tierOf(QStringLiteral("pointer"), {}), ControlTier::Admin);
    QCOMPARE(server_->tierOf(QStringLiteral("not_a_method"), {}), ControlTier::Admin);
}

void ControlPolicyTest::adminMethodsAreRefused()
{
    server_->setPolicy(restricted(true, true));
    ControlClient client(*server_);
    QCOMPARE(errorCode(client.call(QStringLiteral("key"), {{QStringLiteral("key"), QStringLiteral("a")}})),
             QStringLiteral("refused"));
    QCOMPARE(errorCode(client.call(QStringLiteral("pointer"),
                                   {{QStringLiteral("op"), QStringLiteral("click")},
                                    {QStringLiteral("space"), QStringLiteral("window")}})),
             QStringLiteral("refused"));
    QCOMPARE(errorCode(client.call(QStringLiteral("set_unsaved_policy"))),
             QStringLiteral("refused"));
}

void ControlPolicyTest::commandsAreDefaultDeny()
{
    QVERIFY(!pictura::commandAllowed(QStringLiteral("file.exit")));
    QStringList ids;
    for (const pictura::CommandInfo& info : window_->registry()->describe()) {
        if (info.implemented) {
            ids.append(info.id);
        }
    }
    for (const QString& id : pictura::allowedCommands()) {
        QVERIFY2(ids.contains(id), qPrintable(id));
    }

    server_->setPolicy(restricted(true, false));
    ControlClient client(*server_);
    QCOMPARE(errorCode(client.call(QStringLiteral("dispatch_command"),
                                   {{QStringLiteral("id"), QStringLiteral("file.exit")}})),
             QStringLiteral("refused"));
    QVERIFY(succeeded(client.call(QStringLiteral("dispatch_command"),
                                  {{QStringLiteral("id"), QStringLiteral("select.all")}})));
}

void ControlPolicyTest::bareControlIsUnchanged()
{
    ControlClient client(*server_);
    QVERIFY(!server_->policy().restricted);
    const QJsonObject response =
        client.call(QStringLiteral("key"), {{QStringLiteral("key"), QStringLiteral("nope")}});
    QVERIFY(errorCode(response) != QStringLiteral("refused"));
}

void ControlPolicyTest::headlessRefusesOutsideItsRoots()
{
    ControlPolicy policy = restricted(false, false);
    policy.roots = {dir_.filePath(QStringLiteral("root"))};
    server_->setPolicy(policy);
    ControlClient client(*server_);
    const int count = window_->documentCount();
    QCOMPARE(errorCode(client.call(QStringLiteral("document"),
                                   {{QStringLiteral("op"), QStringLiteral("open")},
                                    {QStringLiteral("path"), dir_.filePath(QStringLiteral("x.psd"))}})),
             QStringLiteral("refused"));
    QCOMPARE(window_->documentCount(), count);
}

void ControlPolicyTest::symlinkDoesNotEscape()
{
    QDir base(QFileInfo(dir_.path()).canonicalFilePath());
    QVERIFY(base.mkpath(QStringLiteral("root")));
    QVERIFY(base.mkpath(QStringLiteral("outside")));
    QVERIFY(QFile::link(base.filePath(QStringLiteral("outside")),
                        base.filePath(QStringLiteral("root/link"))));
    ControlPolicy policy = restricted(true, true);
    policy.live = true;
    policy.roots = {base.filePath(QStringLiteral("root"))};
    const QString target = pictura::canonicalTarget(base.filePath(QStringLiteral("root/link/x.psd")));
    QCOMPARE(target, base.filePath(QStringLiteral("outside/x.psd")));
    QCOMPARE(pictura::judgePath(policy, target, true, false, false), PathVerdict::Ask);
}

void ControlPolicyTest::siblingPrefixIsNotInside()
{
    ControlPolicy policy = restricted(true, true);
    policy.live = true;
    policy.roots = {QStringLiteral("/a/b")};
    QCOMPARE(pictura::judgePath(policy, QStringLiteral("/a/b2/x.psd"), false, true, false),
             PathVerdict::Ask);
    QCOMPARE(pictura::judgePath(policy, QStringLiteral("/a/b/x.psd"), false, true, false),
             PathVerdict::Allow);
}

void ControlPolicyTest::protectedDirectoriesAreRefused()
{
    ControlPolicy policy = restricted(true, true);
    policy.live = true;
    policy.approval = ApprovalMode::AllowAll;
    policy.denied = {QStringLiteral("/state")};
    QCOMPARE(pictura::judgePath(policy, QStringLiteral("/state/state.json"), true, true, false),
             PathVerdict::Refuse);
    QCOMPARE(pictura::judgePath(policy, QStringLiteral("/elsewhere/a.psd"), true, true, false),
             PathVerdict::Allow);
}

void ControlPolicyTest::approvalModesDecide_data()
{
    QTest::addColumn<ApprovalMode>("mode");
    QTest::addColumn<QString>("target");
    QTest::addColumn<bool>("exists");
    QTest::addColumn<bool>("known");
    QTest::addColumn<PathVerdict>("verdict");

    const QString in = QStringLiteral("/root/a.psd");
    const QString out = QStringLiteral("/other/a.psd");
    const auto manual = ApprovalMode::Manual;
    const auto autoApprove = ApprovalMode::AutoApprove;
    const auto all = ApprovalMode::AllowAll;
    QTest::newRow("manual inside new") << manual << in << false << false << PathVerdict::Allow;
    QTest::newRow("manual inside overwrite") << manual << in << true << false << PathVerdict::Ask;
    QTest::newRow("manual known overwrite") << manual << in << true << true << PathVerdict::Ask;
    QTest::newRow("manual outside") << manual << out << false << false << PathVerdict::Ask;
    QTest::newRow("auto inside new") << autoApprove << in << false << false << PathVerdict::Allow;
    QTest::newRow("auto inside overwrite") << autoApprove << in << true << false << PathVerdict::Ask;
    QTest::newRow("auto known overwrite") << autoApprove << in << true << true << PathVerdict::Allow;
    QTest::newRow("auto outside") << autoApprove << out << false << false << PathVerdict::Ask;
    QTest::newRow("auto known outside") << autoApprove << out << true << true << PathVerdict::Allow;
    QTest::newRow("all outside overwrite") << all << out << true << false << PathVerdict::Allow;
}

void ControlPolicyTest::approvalModesDecide()
{
    QFETCH(ApprovalMode, mode);
    QFETCH(QString, target);
    QFETCH(bool, exists);
    QFETCH(bool, known);
    QFETCH(PathVerdict, verdict);

    ControlPolicy policy = restricted(true, true);
    policy.live = true;
    policy.approval = mode;
    policy.roots = {QStringLiteral("/root")};
    QCOMPARE(pictura::judgePath(policy, target, true, exists, known), verdict);

    // What a live session would ask, a headless one refuses.
    policy.live = false;
    const PathVerdict headless = verdict == PathVerdict::Ask ? PathVerdict::Refuse : verdict;
    QCOMPARE(pictura::judgePath(policy, target, true, exists, known), headless);
}

QTEST_MAIN(ControlPolicyTest)
#include "tst_control_policy.moc"
