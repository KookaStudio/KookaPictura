// The Character and Paragraph panels (#73, #74): live editors over the active
// type layer, and the Window / Type / options-bar toggles that open them.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "panels/character_panel.h"
#include "panels/paragraph_panel.h"
#include "selftest_paint_fixture.h"

#include "api/type_api.h"

#include <QtGui/QAction>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
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

// The first type layer's path among the top-level layers, or empty.
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

class CharacterParagraphPanelsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void characterPanelReflectsAndEdits();
    void characterDefaultsWithoutType();
    void paragraphPanelReflectsAndEdits();
    void menusTogglePanels();
    void optionsBarTogglesBoth();

private:
    void showFrame();
    QString makeTypeLayer(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CharacterParagraphPanelsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CharacterParagraphPanelsTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

QString CharacterParagraphPanelsTest::makeTypeLayer(Fixture& f)
{
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    pictura::TypeOptions o = f.tools->typeOptions();
    o.size = 30.0;
    f.tools->setTypeOptions(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    return typeLayer(f);
}

void CharacterParagraphPanelsTest::characterPanelReflectsAndEdits()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_panel"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    const QColor foreground = f.tools->foreground();
    const pictura::TypeOptions saved = f.tools->typeOptions();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");

    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    // The panel reflects the selected layer.
    QVERIFY(panel->editingEnabledForTest());
    QCOMPARE(panel->sizeFieldForTest()->value(), 30.0);
    QCOMPARE(panel->trackingFieldForTest()->value(), 0.0);
    QCOMPARE(panel->underlineForTest()->isChecked(), false);

    // Refreshing the controls from the layer records nothing.
    const int shownBase = f.view->history_index();
    panel->refresh();
    panel->setView(f.view);
    QCOMPARE(f.view->history_index(), shownBase);

    // A committed size is one state; one undo restores it.
    const int base = f.view->history_index();
    panel->sizeFieldForTest()->setValue(48.0);
    emit panel->sizeFieldForTest()->editingFinished();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 48.0);
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 30.0);

    // A toggle edit is also one state.
    panel->refresh();
    const int toggleBase = f.view->history_index();
    panel->underlineForTest()->setChecked(true);
    QVERIFY2(f.committedOnce(toggleBase, "Edit Type Layer"), "one toggle state");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).underline, true);

    f.tools->setTypeOptions(saved);
    f.tools->setForeground(foreground);
}

void CharacterParagraphPanelsTest::characterDefaultsWithoutType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_defaults"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    // No type layer active: the model defaults, editing disabled.
    QVERIFY(!panel->editingEnabledForTest());
    QCOMPARE(panel->sizeFieldForTest()->value(), pictura::type_api::defaultCharacterSetting().size);
    QCOMPARE(panel->trackingFieldForTest()->value(),
             pictura::type_api::defaultCharacterSetting().tracking);

    // Committing while disabled records nothing.
    const int base = f.view->history_index();
    panel->commitForTest();
    QCOMPARE(f.view->history_index(), base);
}

void CharacterParagraphPanelsTest::paragraphPanelReflectsAndEdits()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_panel"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    const pictura::TypeOptions saved = f.tools->typeOptions();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");

    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    const pictura::ParagraphSetting shown = pictura::type_api::layerParagraphSetting(*f.view, path);
    QVERIFY(panel->editingEnabledForTest());
    QCOMPARE(panel->justifyFieldForTest()->currentIndex(), int(shown.justify));
    QCOMPARE(panel->firstLineIndentFieldForTest()->value(), shown.first_line_indent);

    // Committing the first-line indent is one state; one undo restores it.
    const int base = f.view->history_index();
    panel->firstLineIndentFieldForTest()->setValue(20.0);
    emit panel->firstLineIndentFieldForTest()->editingFinished();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).first_line_indent, 20.0);
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).first_line_indent, 0.0);

    // Alignment is editable too.
    panel->refresh();
    const int alignBase = f.view->history_index();
    panel->justifyFieldForTest()->setCurrentIndex(2);
    QVERIFY2(f.committedOnce(alignBase, "Edit Type Layer"), "one alignment state");
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).justify, 2);

    f.tools->setTypeOptions(saved);
}

void CharacterParagraphPanelsTest::menusTogglePanels()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_menu"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* column = frame.columnForPanel(QStringLiteral("characterPanel"));
    QVERIFY(column);
    column->showPanel(QStringLiteral("characterPanel"), false);
    column->showPanel(QStringLiteral("paragraphPanel"), false);
    frame.registry()->refresh();

    // Both menu paths must drive the panel with their OWN action: sharing one
    // registry id across the Type and Window entries made the handler read the
    // other menu's check state and hide instead of show.
    QVERIFY(frame.registry()->action(QString::fromLatin1(pictura::command_ids::WindowPanelsCharacter))
            != frame.registry()->action(
                QString::fromLatin1(pictura::command_ids::TypePanelsCharacter)));

    struct Toggle {
        const char* id;
        const char* panel;
    };
    const Toggle toggles[] = {
        {pictura::command_ids::WindowPanelsCharacter, "characterPanel"},
        {pictura::command_ids::TypePanelsCharacter, "characterPanel"},
        {pictura::command_ids::WindowPanelsParagraph, "paragraphPanel"},
        {pictura::command_ids::TypePanelsParagraph, "paragraphPanel"},
    };
    for (const Toggle& toggle : toggles) {
        const QString id = QString::fromLatin1(toggle.id);
        const QString panel = QString::fromLatin1(toggle.panel);
        QAction* action = frame.registry()->action(id);
        QVERIFY2(action, qPrintable(id));
        QVERIFY2(action->isEnabled(), qPrintable(id));
        QVERIFY(action->isCheckable());
        QVERIFY2(!action->isChecked(), qPrintable(id));

        action->setChecked(true);
        QVERIFY2(frame.registry()->dispatch(id), qPrintable(id));
        frame.registry()->refresh();
        QVERIFY2(column->isPanelVisible(panel), qPrintable(id));

        action->setChecked(false);
        QVERIFY2(frame.registry()->dispatch(id), qPrintable(id));
        frame.registry()->refresh();
        QVERIFY2(!column->isPanelVisible(panel), qPrintable(id));
    }
}

void CharacterParagraphPanelsTest::optionsBarTogglesBoth()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_panel_button"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* column = frame.columnForPanel(QStringLiteral("characterPanel"));
    QVERIFY(column);
    column->showPanel(QStringLiteral("characterPanel"), false);
    column->showPanel(QStringLiteral("paragraphPanel"), false);

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    auto* button = frame.findChild<QToolButton*>(QStringLiteral("optionsTypePanel"));
    QVERIFY2(button, "the Panel button");

    button->click();
    QVERIFY(column->isPanelVisible(QStringLiteral("characterPanel")));
    QVERIFY(column->isPanelVisible(QStringLiteral("paragraphPanel")));

    button->click();
    QVERIFY(!column->isPanelVisible(QStringLiteral("characterPanel")));
    QVERIFY(!column->isPanelVisible(QStringLiteral("paragraphPanel")));
}

QTEST_MAIN(CharacterParagraphPanelsTest)
#include "tst_character_paragraph_panels.moc"
