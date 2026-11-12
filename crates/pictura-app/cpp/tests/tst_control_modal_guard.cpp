// ControlScope: a dialog reached during a request is cancelled and refused, and
// a dialog outside one is left alone.

#include <QtTest/QtTest>

#include <QtCore/QJsonObject>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QProgressDialog>

#include "commands.h"
#include "control_server.h"
#include "dialogs.h"
#include "frame.h"
#include "api/document_bridge.h"
#include "api/view_bridge.h"

#include "qt_test_support.h"

namespace {

QStringList visibleDialogs()
{
    QStringList titles;
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (qobject_cast<QDialog*>(widget) && widget->isVisible()) {
            titles.append(widget->windowTitle());
        }
    }
    return titles;
}

QString errorCode(const QJsonObject& response)
{
    return response.value(QStringLiteral("error"))
        .toObject()
        .value(QStringLiteral("code"))
        .toString();
}

} // namespace

class ControlModalGuardTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void imageSizeIsRefused();
    void panelModalsAreRefused();
    void dialogOutsideARequestStaysOpen();
    void everyCommandAnswers();
    void aJobProgressWindowRunsToTheEnd();

private:
    void ensureDocument();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlModalGuardTest::initTestCase()
{
    qputenv("PICTURA_DISABLE_UPDATE_CHECK", "1");
    QVERIFY(stateHome_.isValid());
    pictura::setUnsavedPromptInteractive(false);
    pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Discard);
    pictura::setTransformPromptInteractive(false);
    pictura::setCropPromptInteractive(false);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    server_ = std::make_unique<pictura::ControlServer>(window_.get(), QString());
    ensureDocument();
}

void ControlModalGuardTest::ensureDocument()
{
    if (window_->documentCount() == 0) {
        QVERIFY(window_->newDocument(QStringLiteral("Guard"), 32, 32, QStringLiteral("rgb"), 8,
                                     QStringLiteral("white")));
    }
}

void ControlModalGuardTest::imageSizeIsRefused()
{
    const int width = window_->activeView()->document_width();
    const QJsonObject response =
        server_->dispatch(QStringLiteral("dispatch_command"),
                          {{QStringLiteral("id"), QStringLiteral("image.image.size")}});
    QCOMPARE(errorCode(response), QStringLiteral("refused"));
    QCoreApplication::processEvents();
    QVERIFY2(visibleDialogs().isEmpty(), qPrintable(visibleDialogs().join(u',')));
    QCOMPARE(window_->activeView()->document_width(), width);
}

void ControlModalGuardTest::panelModalsAreRefused()
{
    ensureDocument();
    const char* const ids[] = {
        pictura::command_ids::PanelHistoryNewSnapshot,
        pictura::command_ids::PanelPropertiesSavePreset,
        pictura::command_ids::ActionsLoad,
        pictura::command_ids::ActionsSave,
        pictura::command_ids::WindowWorkspaceNew,
    };
    for (const char* id : ids) {
        const QJsonObject response = server_->dispatch(
            QStringLiteral("dispatch_command"),
            {{QStringLiteral("id"), QString::fromLatin1(id)}});
        QCOMPARE(errorCode(response), QStringLiteral("refused"));
    }
}

void ControlModalGuardTest::dialogOutsideARequestStaysOpen()
{
    QDialog dialog(window_.get());
    dialog.show();
    // Nothing to wait on: this proves a cancel never arrives.
    QTest::qWait(20);
    QVERIFY(dialog.isVisible());
    dialog.reject();
}

void ControlModalGuardTest::everyCommandAnswers()
{
    QStringList left;
    for (const pictura::CommandInfo& info : window_->registry()->describe()) {
        if (info.id.isEmpty() || !info.implemented) {
            continue;
        }
        ensureDocument();
        // A hang here is the failure this test exists for: the suite times out.
        const QJsonObject response = server_->dispatch(QStringLiteral("dispatch_command"),
                                                       {{QStringLiteral("id"), info.id}});
        QVERIFY2(response.contains(QStringLiteral("ok")), qPrintable(info.id));
        QCoreApplication::processEvents();
        if (!visibleDialogs().isEmpty()) {
            left.append(info.id);
            for (QWidget* widget : QApplication::topLevelWidgets()) {
                if (auto* dialog = qobject_cast<QDialog*>(widget)) {
                    dialog->reject();
                }
            }
        }
    }
    QVERIFY2(left.isEmpty(), qPrintable(left.join(u',')));
}

// [ac_guard_job_progress] A commit job a request starts shows its cancellable
// window; the guard leaves it to land (#1290).
void ControlModalGuardTest::aJobProgressWindowRunsToTheEnd()
{
    QVERIFY(window_->newDocument(QStringLiteral("Job"), 4200, 4200, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    QCOMPARE(view->duplicate_layer(0), 1);
    pictura::transform_set_delay(*view, 800);
    const int states = view->history_count();
    bool sawWindow = false;
    QTimer probe;
    probe.setInterval(20);
    connect(&probe, &QTimer::timeout, this, [&] {
        auto* d = window_->findChild<QProgressDialog*>(QStringLiteral("jobProgress"));
        sawWindow = sawWindow || (d && d->isVisible());
    });
    probe.start();
    const QJsonObject response = server_->dispatch(
        QStringLiteral("layer_op"), {{QStringLiteral("op"), QStringLiteral("free_transform")},
                                     {QStringLiteral("index"), 1},
                                     {QStringLiteral("scale"), 50.0}});
    probe.stop();
    pictura::transform_set_delay(*view, 0);
    QVERIFY2(response.value(QStringLiteral("ok")).toBool(),
             qPrintable(errorCode(response)));
    QVERIFY2(sawWindow, "the progress window showed");
    // The state lands with the deferred composite, after the job.
    QTRY_COMPARE_WITH_TIMEOUT(view->history_count(), states + 1, 15000);
    QCOMPARE(view->history_label(states), QStringLiteral("Free Transform"));
    QVERIFY(window_->closeActiveDocument(false));
}

QTEST_MAIN(ControlModalGuardTest)
#include "tst_control_modal_guard.moc"
