// A control server in a window a person is using: off by default, and it leaves
// the person's work and dialogs alone.

#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QScopeGuard>
#include <QtCore/QSysInfo>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QDialog>

#include "agent_control.h"
#include "control_scope.h"
#include "control_server.h"
#include "control_test_client.h"
#include "dialogs.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "qt_test_support.h"

using pictura::AgentControl;
using pictura::AgentSettings;
using pictura::test::ControlClient;
using pictura::test::errorCode;
using pictura::test::succeeded;

class ControlLiveTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void offByDefault();
    void startsAndStopsAtRuntime();
    void settingsPersist();
    void secondWindowDoesNotTakeOver();
    void deadHolderDoesNotBlock();
    void sandboxedBuildsStart();
    void flatpakInstancesShareTheEndpoint();
    void otherBuildsKeepTheirEndpoint();
    void dirtyCloseIsRefused();
    void personsPromptIsUnaffected();
    void editsWaitForAPerson();
    void openLeavesAPersonsDialogAlone();

private:
    AgentSettings enabled() const;
    void makeDirty();

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    QString endpoint_;
    QString root_;
    QString fixture_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<AgentControl> agent_;
};

void ControlLiveTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    QDir base(QFileInfo(dir_.path()).canonicalFilePath());
    QVERIFY(base.mkpath(QStringLiteral("endpoint")));
    QVERIFY(base.mkpath(QStringLiteral("root")));
    endpoint_ = base.filePath(QStringLiteral("endpoint/live.sock"));
    root_ = base.filePath(QStringLiteral("root"));
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->newDocument(QStringLiteral("Live"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    fixture_ = root_ + QStringLiteral("/fixture.psd");
    QVERIFY(window_->saveActiveAs(fixture_));
}

void ControlLiveTest::init()
{
    agent_ = std::make_unique<AgentControl>(window_.get(), endpoint_);
    agent_->setParent(nullptr);
    agent_->apply(AgentSettings{});
}

void ControlLiveTest::cleanup()
{
    agent_.reset();
    qunsetenv("FLATPAK_ID");
    qunsetenv("SNAP");
}

AgentSettings ControlLiveTest::enabled() const
{
    AgentSettings settings;
    settings.enabled = true;
    settings.allowEdit = true;
    settings.allowSave = true;
    settings.roots = {root_};
    return settings;
}

void ControlLiveTest::makeDirty()
{
    QVERIFY(succeeded(agent_->server()->dispatch(QStringLiteral("layer_op"),
                                                 {{QStringLiteral("op"), QStringLiteral("add")},
                                                  {QStringLiteral("index"), 0}})));
    QVERIFY(window_->isDocumentDirty(window_->activeDocumentIndex()));
}

void ControlLiveTest::offByDefault()
{
    QVERIFY(!agent_->isRunning());
    QVERIFY(!QFile::exists(endpoint_));
    QVERIFY(agent_->problem().isEmpty());
}

void ControlLiveTest::startsAndStopsAtRuntime()
{
    agent_->apply(enabled());
    QVERIFY(agent_->isRunning());
    const QString tokenPath = agent_->server()->tokenPath();
    {
        ControlClient client(*agent_->server());
        QVERIFY(succeeded(client.call(QStringLiteral("status"))));
    }
    agent_->apply(AgentSettings{});
    QVERIFY(!agent_->isRunning());
    QVERIFY(!QFile::exists(endpoint_));
    QVERIFY(!QFile::exists(tokenPath));
}

void ControlLiveTest::settingsPersist()
{
    AgentSettings settings = enabled();
    settings.approval = pictura::ApprovalMode::Manual;
    agent_->apply(settings);
    agent_->apply(AgentSettings{});
    agent_->apply(settings);
    const AgentSettings stored = settings;
    agent_.reset();
    agent_ = std::make_unique<AgentControl>(window_.get(), endpoint_);
    agent_->setParent(nullptr);
    QVERIFY(agent_->settings() == stored);
    QVERIFY(agent_->isRunning());
}

void ControlLiveTest::secondWindowDoesNotTakeOver()
{
    agent_->apply(enabled());
    AgentControl second(window_.get(), endpoint_);
    second.setParent(nullptr);
    QVERIFY(!second.isRunning());
    QVERIFY(!second.problem().isEmpty());
    ControlClient client(*agent_->server());
    QVERIFY(succeeded(client.call(QStringLiteral("status"))));
}

void ControlLiveTest::deadHolderDoesNotBlock()
{
    // Names a live PID, as a dead Flatpak window's lock can in the next window.
    QFile lock(endpoint_ + QStringLiteral(".lock"));
    QVERIFY(lock.open(QIODevice::WriteOnly));
    lock.write(QByteArray::number(QCoreApplication::applicationPid())
               + "\npictura\n" + QSysInfo::machineHostName().toUtf8() + "\n");
    lock.close();
    agent_->apply(enabled());
    QVERIFY(agent_->isRunning());
    QVERIFY(agent_->problem().isEmpty());
}

void ControlLiveTest::sandboxedBuildsStart()
{
    for (const char* variable : {"FLATPAK_ID", "SNAP"}) {
        qputenv(variable, "/sandbox");
        agent_->apply(enabled());
        QVERIFY(agent_->isRunning());
        QVERIFY(agent_->problem().isEmpty());
        agent_->apply(AgentSettings{});
        qunsetenv(variable);
    }
}

void ControlLiveTest::flatpakInstancesShareTheEndpoint()
{
    const QByteArray runtime = qgetenv("XDG_RUNTIME_DIR");
    const auto restore = qScopeGuard([&runtime] { qputenv("XDG_RUNTIME_DIR", runtime); });
    QTemporaryDir fake;
    QVERIFY(fake.isValid());
    const QString shared = QStringLiteral("app/io.github.KookaStudio.KookaPictura");
    QVERIFY(QDir(fake.path()).mkpath(shared));
    qputenv("XDG_RUNTIME_DIR", fake.path().toLocal8Bit());
    qputenv("FLATPAK_ID", "io.github.KookaStudio.KookaPictura");
    QString problem;
    QCOMPARE(AgentControl::liveEndpoint(&problem),
             fake.filePath(shared + QStringLiteral("/pictura/live.sock")));
    QVERIFY(problem.isEmpty());
}

void ControlLiveTest::otherBuildsKeepTheirEndpoint()
{
    const QByteArray runtime = qgetenv("XDG_RUNTIME_DIR");
    const auto restore = qScopeGuard([&runtime] { qputenv("XDG_RUNTIME_DIR", runtime); });
    QTemporaryDir fake;
    QVERIFY(fake.isValid());
    qputenv("XDG_RUNTIME_DIR", fake.path().toLocal8Bit());
    const QString expected = fake.filePath(QStringLiteral("pictura/live.sock"));
    QCOMPARE(AgentControl::liveEndpoint(nullptr), expected);
    qputenv("SNAP", "/snap/kooka-pictura/current");
    QCOMPARE(AgentControl::liveEndpoint(nullptr), expected);
}

void ControlLiveTest::dirtyCloseIsRefused()
{
    agent_->apply(enabled());
    makeDirty();
    const int count = window_->documentCount();
    ControlClient client(*agent_->server());
    QCOMPARE(errorCode(client.call(QStringLiteral("document"),
                                   {{QStringLiteral("op"), QStringLiteral("close")}})),
             QStringLiteral("unsaved"));
    QCOMPARE(window_->documentCount(), count);
}

void ControlLiveTest::personsPromptIsUnaffected()
{
    pictura::setUnsavedPromptInteractive(false);
    pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Discard);
    const pictura::UnsavedChoice cancel = pictura::UnsavedChoice::Cancel;
    {
        const pictura::ControlScope scope(&cancel);
        QCOMPARE(pictura::askUnsaved(window_.get(), QStringLiteral("x")),
                 pictura::UnsavedChoice::Cancel);
    }
    const int before = pictura::nonInteractiveUnsavedPromptCount();
    QCOMPARE(pictura::askUnsaved(window_.get(), QStringLiteral("x")),
             pictura::UnsavedChoice::Discard);
    QCOMPARE(pictura::nonInteractiveUnsavedPromptCount(), before + 1);
}

void ControlLiveTest::editsWaitForAPerson()
{
    agent_->apply(enabled());
    ControlClient client(*agent_->server());
    QDialog dialog(window_.get());
    dialog.show();
    QCOMPARE(errorCode(client.call(QStringLiteral("filter"),
                                   {{QStringLiteral("kind"), QStringLiteral("gaussian-blur")}})),
             QStringLiteral("human_busy"));
    QVERIFY(succeeded(client.call(QStringLiteral("status"))));
    QVERIFY(dialog.isVisible());
    dialog.reject();
}

void ControlLiveTest::openLeavesAPersonsDialogAlone()
{
    agent_->apply(enabled());
    ControlClient client(*agent_->server());
    QDialog dialog(window_.get());
    client.send(1, QStringLiteral("document"),
                {{QStringLiteral("op"), QStringLiteral("open")},
                 {QStringLiteral("path"), fixture_}});
    // The dialog appears mid-request, which is what a guarded load would cancel.
    QTimer::singleShot(0, &dialog, [&dialog] { dialog.show(); });
    QVERIFY(succeeded(client.read()));
    QVERIFY(dialog.isVisible());
    dialog.reject();
}

QTEST_MAIN(ControlLiveTest)
#include "tst_control_live.moc"
