#include <QtTest/QtTest>

#include "api/layer_comps_bridge.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "qt_test_support.h"

#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonArray>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTabWidget>

namespace {

const QString defineId = QStringLiteral("image.variables.define");
const QString dataSetsId = QStringLiteral("image.variables.data.sets");
const QString applyId = QStringLiteral("image.apply.data.set");
const QString exportId = QStringLiteral("file.export.data.sets.as.files");

QString send(pictura::PictureView& view, const QString& action, const QJsonObject& fields = {})
{
    return pictura::variables_command(view, action,
        QString::fromUtf8(QJsonDocument(fields).toJson(QJsonDocument::Compact)));
}

bool startTemplate(pictura::PicturaMainWindow& window)
{
    return window.newDocument(QStringLiteral("Template"), 32, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"));
}

void defineSet(pictura::PictureView& view, bool withSet = true)
{
    const QString path = view.add_layer_in(QString());
    QVERIFY(!path.isEmpty());
    QCOMPARE(send(view, QStringLiteral("begin")), QString());
    QCOMPARE(send(view, QStringLiteral("bind"), {{"path", path}, {"kind", "visibility"}, {"name", "show"}}), QString());
    if (withSet) {
        QCOMPARE(send(view, QStringLiteral("newSet"), {{"name", "Example"}}), QString());
    }
    QCOMPARE(send(view, QStringLiteral("commit")), QString());
}

QJsonObject state(pictura::PictureView& view, bool draft = false)
{
    return QJsonDocument::fromJson(pictura::variables_state(view, draft).toUtf8()).object();
}

QDialog* dialogNamed(const QString& name)
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<QDialog*>(widget); dialog && dialog->objectName() == name) {
            return dialog;
        }
    }
    return nullptr;
}

} // namespace

class VariablesTest : public QObject {
    Q_OBJECT
private slots:
    void commandsTrackTemplateState();
    void definitionAndSetCommitTogether();
    void cancelDiscardsStagedEdits();
    void previewSwitchAndCancel();
    void exportDialogCancelsWithoutWriting();
    void desktopOpenLoadsTemplateMetadata();
};

void VariablesTest::commandsTrackTemplateState()
{
    pictura::test::ScopedStateHome home;
    const auto window = pictura::test::makeMainWindow();
    const auto enabled = [&](const QString& id) { return window->registry()->action(id)->isEnabled(); };
    QVERIFY(!enabled(defineId));
    QVERIFY(startTemplate(*window));
    QVERIFY(enabled(defineId));
    QVERIFY(!enabled(dataSetsId));
    defineSet(*window->activeView(), false);
    QVERIFY(enabled(dataSetsId));
    QVERIFY(!enabled(applyId));
    auto& view = *window->activeView();
    QCOMPARE(send(view, QStringLiteral("begin")), QString());
    QCOMPARE(send(view, QStringLiteral("newSet"), {{"name", "Example"}}), QString());
    QCOMPARE(send(view, QStringLiteral("commit")), QString());
    QVERIFY(enabled(applyId));
    QVERIFY(enabled(exportId));
    QVERIFY(view.begin_free_transform(QStringLiteral("0")));
    QVERIFY(!pictura::variables_available(view, QStringLiteral("sets")));
    QVERIFY(!send(view, QStringLiteral("apply"), {{"set", 0}}).isEmpty());
    view.cancel_transform();
}

void VariablesTest::definitionAndSetCommitTogether()
{
    pictura::test::ScopedStateHome home;
    const auto window = pictura::test::makeMainWindow();
    QVERIFY(startTemplate(*window));
    auto& view = *window->activeView();
    const auto path = view.add_layer_in(QString());
    QVERIFY(!path.isEmpty());
    const int before = view.history_count();
    bool opened = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = dialogNamed(QStringLiteral("variablesDialog"));
        if (!dialog) { return; }
        auto* name = dialog->findChild<QComboBox*>(QStringLiteral("variableBindingName"));
        auto* bind = dialog->findChild<QPushButton*>(QStringLiteral("variableSetBinding"));
        auto* pages = dialog->findChild<QTabWidget*>(QStringLiteral("variablesPages"));
        auto* add = dialog->findChild<QPushButton*>(QStringLiteral("variableNewDataSet"));
        auto* buttons = dialog->findChild<QDialogButtonBox*>();
        if (!name || !bind || !pages || !add || !buttons) { dialog->reject(); return; }
        opened = true;
        name->setEditText(QStringLiteral("show"));
        bind->click();
        pages->setCurrentIndex(1);
        add->click();
        buttons->button(QDialogButtonBox::Ok)->click();
    });
    window->registry()->action(defineId)->trigger();
    QVERIFY(opened);
    QCOMPARE(state(view).value("sets").toArray().size(), 1);
    QCOMPARE(view.history_count(), before + 1);
    QVERIFY(view.undo());
    QVERIFY(state(view).value("variables").toArray().isEmpty());
}

void VariablesTest::cancelDiscardsStagedEdits()
{
    pictura::test::ScopedStateHome home;
    const auto window = pictura::test::makeMainWindow();
    QVERIFY(startTemplate(*window));
    auto& view = *window->activeView();
    defineSet(view);
    const int before = view.history_count();
    const auto original = state(view);
    QTemporaryDir directory;
    QFile file(directory.filePath(QStringLiteral("values.csv")));
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write("another\nfalse\n");
    file.close();
    bool staged = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = dialogNamed(QStringLiteral("variablesDialog"));
        if (!dialog) { return; }
        staged = send(view, QStringLiteral("renameVariable"), {{"from", "show"}, {"name", "another"}}).isEmpty()
            && send(view, QStringLiteral("import"), {{"path", file.fileName()}}).isEmpty();
        const auto changed = state(view, true);
        staged = staged && changed.value("variables") != original.value("variables")
            && changed.value("sets").toArray().size() == 2;
        dialog->reject();
    });
    window->registry()->action(dataSetsId)->trigger();
    QVERIFY(staged);
    QCOMPARE(view.history_count(), before);
    QCOMPARE(state(view), original);
}

void VariablesTest::previewSwitchAndCancel()
{
    pictura::test::ScopedStateHome home;
    const auto window = pictura::test::makeMainWindow();
    QVERIFY(startTemplate(*window));
    auto& view = *window->activeView();
    QTemporaryDir directory;
    QImage replacement(16, 8, QImage::Format_RGBA8888);
    replacement.fill(Qt::red);
    const auto image = directory.filePath(QStringLiteral("red.png"));
    QVERIFY(replacement.save(image));
    const auto original = state(view);
    const int history = view.history_count();
    const auto makePreview = [&](int index) {
        const QJsonObject request{{"set", index}, {"draft", true}};
        const auto bytes = pictura::variables_preview(view, QString::fromUtf8(QJsonDocument(request).toJson()));
        return QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
    };
    QCOMPARE(send(view, QStringLiteral("begin")), QString());
    QCOMPARE(send(view, QStringLiteral("bind"), {{"path", "0"}, {"kind", "pixels"}, {"name", "photo"}, {"method", "asIs"}}), QString());
    QCOMPARE(send(view, QStringLiteral("newSet"), {{"name", "Baseline"}}), QString());
    const auto baseline = makePreview(0);
    QCOMPARE(send(view, QStringLiteral("newSet"), {{"name", "Red"}}), QString());
    QCOMPARE(send(view, QStringLiteral("setValue"), {{"set", 1}, {"name", "photo"}, {"value", image}}), QString());
    const auto red = makePreview(1);
    QCOMPARE(red.size(), baseline.size());
    QVERIFY(red.size() > 8 && red != baseline);
    QCOMPARE(makePreview(0), baseline);
    QCOMPARE(send(view, QStringLiteral("cancel")), QString());
    QCOMPARE(state(view), original);
    QCOMPARE(view.history_count(), history);
}

void VariablesTest::exportDialogCancelsWithoutWriting()
{
    pictura::test::ScopedStateHome home;
    QTemporaryDir directory;
    const auto window = pictura::test::makeMainWindow();
    QVERIFY(startTemplate(*window));
    defineSet(*window->activeView());
    QVERIFY(window->saveActiveAs(directory.filePath(QStringLiteral("template.psd"))));
    bool ran = false;
    QTimer::singleShot(0, [&] {
        if (auto* dialog = dialogNamed(QStringLiteral("dataSetsExportDialog"))) {
            ran = dialog->findChild<QWidget*>(QStringLiteral("dataSetsExportSelection")) != nullptr;
            dialog->reject();
        }
    });
    window->registry()->action(exportId)->trigger();
    QVERIFY(ran);
    QCOMPARE(QDir(directory.path()).entryList(QDir::Files), QStringList{QStringLiteral("template.psd")});
}

void VariablesTest::desktopOpenLoadsTemplateMetadata()
{
    pictura::test::ScopedStateHome home;
    QTemporaryDir directory;
    const auto window = pictura::test::makeMainWindow();
    QVERIFY(startTemplate(*window));
    defineSet(*window->activeView());
    const auto expected = state(*window->activeView());
    const auto path = directory.filePath(QStringLiteral("template.psd"));
    QVERIFY(window->saveActiveAs(path));
    const auto reopened = pictura::test::makeMainWindow();
    QVERIFY(reopened->openDocumentAtPath(path, false));
    QTRY_VERIFY(reopened->activeView() && reopened->activeView()->has_document());
    QCOMPARE(state(*reopened->activeView()), expected);
    QVERIFY(reopened->registry()->action(applyId)->isEnabled());
}

PICTURA_QT_TEST_MAIN(VariablesTest)
#include "tst_variables.moc"
