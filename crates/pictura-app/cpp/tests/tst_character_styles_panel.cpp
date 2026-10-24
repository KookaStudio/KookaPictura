// The Character Styles panel (#519): lists the document's named character
// styles, applies one to the active type layer, creates/edits/deletes through
// the style sheet, and opens from the Window / Type menu paths.

#include <QtTest/QtTest>

#include <QtCore/QTimer>

#include "commands.h"
#include "frame.h"
#include "panels/character_styles_panel.h"
#include "paragraph_style_dialog.h"
#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "api/type_api.h"

#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(200, 160, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QString typeLayer(const Fixture& f)
{
    for (int i = 0; i < f.view->layer_count(); ++i) {
        if (pictura::type_api::isType(*f.view, QString::number(i))) {
            return QString::number(i);
        }
    }
    return QString();
}

} // namespace

class CharacterStylesPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void listsAndApplies();
    void noTypeLayerRecordsNothing();
    void emptyListIsValid();
    void createsAndDeletes();
    void editsAndReapplies();
    void renameKeepsApplied();
    void dialogPages();
    void dialogRoundTrips();
    void dialogPreviewRestoresOnCancel();
    void menusToggle();

private:
    void showFrame();
    QString makeTypeLayer(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CharacterStylesPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CharacterStylesPanelTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

QString CharacterStylesPanelTest::makeTypeLayer(Fixture& f)
{
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    pictura::TypeOptions o =
        f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    return typeLayer(f);
}

void CharacterStylesPanelTest::listsAndApplies()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_list"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");

    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    QCOMPARE(panel->listForTest()->count(), 0);

    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    c.size = 48.0;
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Heading"), QString(), c));
    pictura::CharacterSetting caption = pictura::type_api::defaultCharacterSetting();
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Caption"), QString(),
                                                     caption));
    panel->refresh();
    QCOMPARE(panel->listForTest()->count(), 2);
    QCOMPARE(panel->listForTest()->item(0)->text(), QStringLiteral("Heading"));
    QCOMPARE(panel->listForTest()->item(1)->text(), QStringLiteral("Caption"));

    const int base = f.view->history_index();
    panel->applyRowForTest(0);
    QVERIFY2(f.committedOnce(base, "Apply Type Style"), "one apply state");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 48.0);
    QVERIFY(f.view->undo());
    QVERIFY(pictura::type_api::layerCharacterSetting(*f.view, path).size != 48.0);
}

void CharacterStylesPanelTest::noTypeLayerRecordsNothing()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_no_type"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Heading"), QString(), c));
    panel->refresh();
    const int base = f.view->history_index();
    panel->applyRowForTest(0);
    QCOMPARE(f.view->history_index(), base);
}

void CharacterStylesPanelTest::emptyListIsValid()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_empty"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    QCOMPARE(panel->listForTest()->count(), 0);
    auto* remove = panel->findChild<QToolButton*>(QStringLiteral("characterStyleDelete"));
    QVERIFY(remove);
    QVERIFY(!remove->isEnabled());
}

void CharacterStylesPanelTest::createsAndDeletes()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_admin"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->findChild<QLineEdit*>(QStringLiteral("paragraphStyleName"))
            ->setText(QStringLiteral("Quote"));
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleFauxItalic"))
            ->setChecked(true);
        dialog->accept();
    });
    QVERIFY(panel->createStyle());
    QCOMPARE(panel->listForTest()->count(), 1);
    QCOMPARE(panel->listForTest()->item(0)->text(), QStringLiteral("Quote"));
    QVERIFY(pictura::type_api::characterStyleCharacter(*f.view, QStringLiteral("Quote")).faux_italic);

    panel->listForTest()->setCurrentRow(0);
    const int base = f.view->history_index();
    QVERIFY(panel->deleteSelectedStyle());
    QVERIFY2(f.committedOnce(base, "Delete Character Style"), "one delete state");
    QCOMPARE(panel->listForTest()->count(), 0);
    QVERIFY(f.view->undo());
    panel->refresh();
    QCOMPARE(panel->listForTest()->count(), 1);
}

void CharacterStylesPanelTest::editsAndReapplies()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_edit"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    c.size = 40.0;
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Quote"), QString(), c));
    panel->refresh();
    panel->applyRowForTest(0);
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 40.0);

    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->findChild<QDoubleSpinBox*>(QStringLiteral("paragraphStyleSize"))->setValue(55.0);
        dialog->accept();
    });
    const int base = f.view->history_index();
    QVERIFY(panel->editStyle(0));
    QVERIFY2(f.committedOnce(base, "Edit Character Style"), "one edit state");
    QCOMPARE(pictura::type_api::characterStyleCharacter(*f.view, QStringLiteral("Quote")).size,
             55.0);
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 55.0);
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 40.0);
}

void CharacterStylesPanelTest::renameKeepsApplied()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_rename"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    c.size = 40.0;
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Quote"), QString(), c));
    panel->refresh();
    panel->applyRowForTest(0);

    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->findChild<QLineEdit*>(QStringLiteral("paragraphStyleName"))
            ->setText(QStringLiteral("Quote 2"));
        dialog->accept();
    });
    QVERIFY(panel->editStyle(0));
    QCOMPARE(panel->listForTest()->item(0)->text(), QStringLiteral("Quote 2"));

    // The layer still applies the renamed style: editing it changes the layer.
    pictura::CharacterSetting edited = pictura::type_api::characterStyleCharacter(
        *f.view, QStringLiteral("Quote 2"));
    edited.size = 60.0;
    QVERIFY(pictura::type_api::editCharacterStyle(*f.view, QStringLiteral("Quote 2"),
                                                  QStringLiteral("Quote 2"), QString(), edited));
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 60.0);
}

void CharacterStylesPanelTest::dialogPages()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_pages"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Heading"), QString(), c));
    panel->refresh();

    bool inspected = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        QCOMPARE(dialog->windowTitle(), QStringLiteral("Character Style Options"));
        QCOMPARE(dialog->pageListForTest()->count(), 3);
        QCOMPARE(dialog->pageListForTest()->item(0)->text(),
                 QStringLiteral("Basic Character Formats"));
        QCOMPARE(dialog->pageListForTest()->item(1)->text(),
                 QStringLiteral("Advanced Character Formats"));
        QCOMPARE(dialog->pageListForTest()->item(2)->text(), QStringLiteral("OpenType Features"));
        QVERIFY(dialog->previewAvailableForTest());
        QVERIFY(dialog->previewForTest()->isChecked());
        inspected = true;
        dialog->reject();
    });
    QVERIFY(!panel->editStyle(0));
    QVERIFY(inspected);
}

void CharacterStylesPanelTest::dialogRoundTrips()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_roundtrip"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Heading"), QString(), c));
    panel->refresh();

    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleFauxBold"))->setChecked(true);
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleFractions"))->setChecked(true);
        dialog->accept();
    });
    QVERIFY(panel->editStyle(0));
    const pictura::CharacterSetting edited =
        pictura::type_api::characterStyleCharacter(*f.view, QStringLiteral("Heading"));
    QVERIFY(edited.faux_bold);
    QVERIFY(edited.fractions);
}

void CharacterStylesPanelTest::dialogPreviewRestoresOnCancel()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_preview"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    auto* panel = frame.findChild<pictura::CharacterStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    c.size = 40.0;
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Heading"), QString(), c));
    panel->refresh();

    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->findChild<QDoubleSpinBox*>(QStringLiteral("paragraphStyleSize"))->setValue(70.0);
        QCOMPARE(pictura::type_api::characterStyleCharacter(*f.view, QStringLiteral("Heading")).size,
                 70.0);
        dialog->reject();
    });
    QVERIFY(!panel->editStyle(0));
    QCOMPARE(pictura::type_api::characterStyleCharacter(*f.view, QStringLiteral("Heading")).size,
             40.0);
}

void CharacterStylesPanelTest::menusToggle()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_styles_menu"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* column = frame.columnForPanel(QStringLiteral("characterStylesPanel"));
    QVERIFY(column);
    column->showPanel(QStringLiteral("characterStylesPanel"), false);
    frame.registry()->refresh();

    const char* ids[] = {pictura::command_ids::WindowPanelsCharacterStyles,
                         pictura::command_ids::TypePanelsCharacterStyles};
    for (const char* id : ids) {
        const QString command = QString::fromLatin1(id);
        QAction* action = frame.registry()->action(command);
        QVERIFY2(action, qPrintable(command));
        QVERIFY2(action->isEnabled(), qPrintable(command));
        QVERIFY(action->isCheckable());
        QVERIFY(!action->isChecked());

        action->setChecked(true);
        QVERIFY2(frame.registry()->dispatch(command), qPrintable(command));
        frame.registry()->refresh();
        QVERIFY2(column->isPanelVisible(QStringLiteral("characterStylesPanel")),
                 qPrintable(command));

        action->setChecked(false);
        QVERIFY2(frame.registry()->dispatch(command), qPrintable(command));
        frame.registry()->refresh();
        QVERIFY2(!column->isPanelVisible(QStringLiteral("characterStylesPanel")),
                 qPrintable(command));
    }
}

QTEST_MAIN(CharacterStylesPanelTest)
#include "tst_character_styles_panel.moc"
