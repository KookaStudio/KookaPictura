// Edit > Find And Replace Text: the dialog, its matching options, the
// current-layer/all-layer scopes, and the one-state replacement.

#include <QtTest/QtTest>

#include "api/document_bridge.h"
#include "api/history_api.h"
#include "api/layers_api.h"
#include "api/type_api.h"
#include "command_ids.h"
#include "commands.h"
#include "find_replace_dialog.h"
#include "import_save_coordinator.h"
#include "selftest_paint_fixture.h"
#include "tool_sessions.h"
#include "tools.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>

#include <QtCore/QCoreApplication>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(360, 240, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QString addType(Fixture& f, const QString& text)
{
    pictura::TypeSetting setting{};
    setting.x = 30.0;
    setting.y = 120.0;
    setting.xx = 1.0;
    setting.yy = 1.0;
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = 40.0;
    const QString path = pictura::type_api::commitLayer(
        *f.view, {text, QStringLiteral("Liberation Sans"), setting, character,
                  pictura::type_api::defaultParagraphSetting()});
    f.frame.selectLayerPath(path);
    f.frame.refresh();
    return path;
}

QString text(Fixture& f, const QString& path)
{
    return pictura::type_api::readLayer(*f.view, path).spec.text;
}

QString findReplaceId()
{
    return pictura::commandIdForPath(
        {QStringLiteral("Edit"), QStringLiteral("Find And Replace Text…")});
}

pictura::FindReplaceDialog* open(Fixture& f)
{
    f.frame.registry()->refresh();
    QAction* action = f.frame.registry()->action(findReplaceId());
    if (!action || !action->isEnabled() || !f.frame.registry()->dispatch(findReplaceId())) {
        return nullptr;
    }
    QCoreApplication::processEvents();
    return f.frame.findChild<pictura::FindReplaceDialog*>(QStringLiteral("findReplaceDialog"));
}

void setFields(pictura::FindReplaceDialog* dialog, const QString& find, const QString& replace,
               bool allLayers = false, bool forward = true, bool caseSensitive = false,
               bool wholeWord = false)
{
    dialog->findChild<QLineEdit*>(QStringLiteral("findReplaceFind"))->setText(find);
    dialog->findChild<QLineEdit*>(QStringLiteral("findReplaceReplace"))->setText(replace);
    dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceAllLayers"))->setChecked(allLayers);
    dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceForward"))->setChecked(forward);
    dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceCaseSensitive"))
        ->setChecked(caseSensitive);
    dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceWholeWord"))->setChecked(wholeWord);
}

bool replaceAll(pictura::FindReplaceDialog* dialog)
{
    QPushButton* button
        = dialog->findChild<QPushButton*>(QStringLiteral("findReplaceReplaceAll"));
    if (!button || !button->isEnabled()) {
        return false;
    }
    button->click();
    QCoreApplication::processEvents();
    return true;
}

void close(pictura::FindReplaceDialog* dialog)
{
    dialog->reject();
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
}

} // namespace

class FindReplaceTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void dialogFields();
    void nonEditableDisabled();
    void sessionGate();
    void currentLayerScope();
    void allLayersScope();
    void noMatchRecordsNothing();
    void lockedLayersSkipped();
    void emptyFindDisabled();
    void blankResultKeepsType();
    void caseSensitive();
    void wholeWordOnly();
    void forwardOverlap();
    void roundTripsAsType();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void FindReplaceTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void FindReplaceTest::cleanup()
{
    for (auto* dialog : window_->findChildren<pictura::FindReplaceDialog*>()) {
        close(dialog);
    }
}

void FindReplaceTest::dialogFields()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_fields"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Find me"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    QVERIFY(dialog->findChild<QLineEdit*>(QStringLiteral("findReplaceFind")));
    QVERIFY(dialog->findChild<QLineEdit*>(QStringLiteral("findReplaceReplace")));
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceAllLayers")));
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceForward")));
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceCaseSensitive")));
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("findReplaceWholeWord")));
    QVERIFY(dialog->findChild<QPushButton*>(QStringLiteral("findReplaceReplaceAll")));
    close(dialog);
}

void FindReplaceTest::nonEditableDisabled()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_noneditable"));
    QVERIFY2(f.ok(), "fixture");
    window_->registry()->refresh();
    QAction* action = window_->registry()->action(findReplaceId());
    QVERIFY(action && !action->isEnabled());
    const QString path = addType(f, QStringLiteral("cat"));
    QVERIFY(action->isEnabled());

    f.frame.selectLayerPath(QStringLiteral("0"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QStringLiteral("dog"));
    QVERIFY(!replaceAll(dialog));
    close(dialog);

    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::ImageMode16Bits)));
    QCOMPARE(pictura::image_depth_bits(*f.view), 16);
    QVERIFY(f.view->layer_is_type(path));
    QVERIFY(!pictura::type_api::layersEditable(*f.view));
    f.frame.selectLayerPath(path);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    const int base = f.view->history_index();
    window_->registry()->dispatch(findReplaceId());
    QVERIFY(!window_->findChild<pictura::FindReplaceDialog*>());
    QCOMPARE(f.view->history_index(), base);
}

void FindReplaceTest::sessionGate()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_session"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Find me"));
    f.frame.setActiveTool(pictura::ToolId::HorizontalType);
    QTest::mouseClick(f.canvas, Qt::LeftButton, Qt::NoModifier, QPoint(40, 120));
    QCoreApplication::processEvents();
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    window_->registry()->refresh();
    QAction* action = window_->registry()->action(findReplaceId());
    QVERIFY(action && !action->isEnabled());
    const int base = f.view->history_index();
    window_->registry()->dispatch(findReplaceId());
    QVERIFY(!window_->findChild<pictura::FindReplaceDialog*>());
    QCOMPARE(f.view->history_index(), base);
    f.tools->textSession()->cancelText();
    f.frame.setActiveTool(pictura::ToolId::Move);
}

void FindReplaceTest::currentLayerScope()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_current"));
    QVERIFY2(f.ok(), "fixture");
    const QString first = addType(f, QStringLiteral("cat"));
    const QString second = addType(f, QStringLiteral("cat"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QStringLiteral("dog"));
    QVERIFY(replaceAll(dialog));
    QCOMPARE(text(f, second), QStringLiteral("dog"));
    QCOMPARE(text(f, first), QStringLiteral("cat"));
    QCOMPARE(pictura::history_api::label(*f.view, f.view->history_index()),
             QStringLiteral("Find And Replace Text"));
    close(dialog);
}

void FindReplaceTest::allLayersScope()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_all"));
    QVERIFY2(f.ok(), "fixture");
    const QString first = addType(f, QStringLiteral("cat"));
    const QString second = addType(f, QStringLiteral("cat"));
    const QString group = f.view->group_layers(QStringList{first, second});
    QVERIFY(!group.isEmpty());
    f.frame.refresh();
    QStringList typePaths;
    for (const pictura::layers_api::LayerRow& row : pictura::layers_api::rows(*f.view)) {
        if (row.kind == QStringLiteral("type")) {
            typePaths << row.path;
        }
    }
    QCOMPARE(typePaths.size(), 2);
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QStringLiteral("dog"), true);
    QVERIFY(replaceAll(dialog));
    for (const QString& path : typePaths) {
        QCOMPARE(text(f, path), QStringLiteral("dog"));
    }
    close(dialog);
}

void FindReplaceTest::noMatchRecordsNothing()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_no_match"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("cat"));
    const int base = f.view->history_index();
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("zzz"), QStringLiteral("dog"));
    QVERIFY(replaceAll(dialog));
    QCOMPARE(f.view->history_index(), base);
    close(dialog);
}

void FindReplaceTest::lockedLayersSkipped()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_locked"));
    QVERIFY2(f.ok(), "fixture");
    const QString locked = addType(f, QStringLiteral("cat"));
    QVERIFY(window_->registry()->dispatch(QStringLiteral("layer.lock.layers.all")));
    const QString openLayer = addType(f, QStringLiteral("cat"));
    const int base = f.view->history_index();
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QStringLiteral("dog"), true);
    QVERIFY(replaceAll(dialog));
    QCOMPARE(text(f, locked), QStringLiteral("cat"));
    QCOMPARE(text(f, openLayer), QStringLiteral("dog"));
    QCOMPARE(f.view->history_index(), base + 1);
    close(dialog);
}

void FindReplaceTest::emptyFindDisabled()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_empty"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("cat"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    QLineEdit* find = dialog->findChild<QLineEdit*>(QStringLiteral("findReplaceFind"));
    find->setText(QString());
    QVERIFY(!dialog->findChild<QPushButton*>(QStringLiteral("findReplaceReplaceAll"))->isEnabled());
    close(dialog);
}

void FindReplaceTest::blankResultKeepsType()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_blank"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("cat"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QString());
    QVERIFY(replaceAll(dialog));
    QVERIFY(pictura::type_api::readLayer(*f.view, path).isType);
    QVERIFY(text(f, path).isEmpty());
    close(dialog);
    const int base = f.view->history_index();
    const QString vertical = pictura::commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Orientation"), QStringLiteral("Vertical")});
    QVERIFY(window_->registry()->dispatch(vertical));
    QVERIFY(pictura::type_api::readLayer(*f.view, path).spec.setting.vertical);
    QVERIFY(text(f, path).isEmpty());
    QCOMPARE(f.view->history_index(), base + 1);
}

void FindReplaceTest::caseSensitive()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_case"));
    QVERIFY2(f.ok(), "fixture");
    const QString lower = addType(f, QStringLiteral("cat"));
    const QString upper = addType(f, QStringLiteral("Cat"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QStringLiteral("dog"), true, true, true);
    QVERIFY(replaceAll(dialog));
    QCOMPARE(text(f, lower), QStringLiteral("dog"));
    QCOMPARE(text(f, upper), QStringLiteral("Cat"));
    close(dialog);
}

void FindReplaceTest::wholeWordOnly()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_word"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("cat catalog"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QStringLiteral("dog"), true, true, false, true);
    QVERIFY(replaceAll(dialog));
    QCOMPARE(text(f, path), QStringLiteral("dog catalog"));
    close(dialog);
}

void FindReplaceTest::forwardOverlap()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_forward"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("aaa"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("aa"), QStringLiteral("X"));
    QVERIFY(replaceAll(dialog));
    QCOMPARE(text(f, path), QStringLiteral("Xa"));
    close(dialog);
    const QString other = addType(f, QStringLiteral("aaa"));
    dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("aa"), QStringLiteral("X"), false, false);
    QVERIFY(replaceAll(dialog));
    QCOMPARE(text(f, other), QStringLiteral("aX"));
    close(dialog);
}

void FindReplaceTest::roundTripsAsType()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_find_round"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("cat"));
    pictura::FindReplaceDialog* dialog = open(f);
    QVERIFY(dialog);
    setFields(dialog, QStringLiteral("cat"), QStringLiteral("dog"));
    QVERIFY(replaceAll(dialog));
    close(dialog);

    QTemporaryDir output;
    QVERIFY(output.isValid());
    const QString psd = output.filePath(QStringLiteral("replace.psd"));
    auto* coordinator = window_->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    QVERIFY(coordinator->saveActiveAtPath(psd));
    QVERIFY(window_->closeActiveDocument(false));
    f.doc = -1;
    QVERIFY(coordinator->openDocumentAtPath(psd, false));
    coordinator->waitForOpensForTest();
    auto* reopened = window_->activeView();
    QVERIFY(reopened);
    const auto layer = pictura::type_api::readLayer(*reopened, path);
    QVERIFY(layer.isType);
    QCOMPARE(layer.spec.text, QStringLiteral("dog"));
    QCOMPARE(layer.spec.font, QStringLiteral("Liberation Sans"));
    QVERIFY(window_->closeActiveDocument(false));
}

PICTURA_QT_TEST_MAIN(FindReplaceTest)
#include "tst_find_replace.moc"
