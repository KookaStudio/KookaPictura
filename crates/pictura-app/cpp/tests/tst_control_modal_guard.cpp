// ControlScope: a dialog reached during a request is cancelled and refused, and
// a dialog outside one is left alone.

#include <QtTest/QtTest>

#include <QtCore/QJsonObject>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>

#include "commands.h"
#include "control_server.h"
#include "dialogs.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
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
    void dialogOutsideARequestStaysOpen();
    void everyCommandAnswers();

private:
    void ensureDocument();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    std::unique_ptr<pictura::ControlServer> server_;
};

void ControlModalGuardTest::initTestCase()
{
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

QTEST_MAIN(ControlModalGuardTest)
#include "tst_control_modal_guard.moc"
