// The Character and Paragraph panels edit the open type session (#1137): new,
// reopened, or mask text. An edit previews at once, records no state, and lands
// on commit; the layer under the session is left alone until then.

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

#include "api/type_api.h"
#include "fonts.h"
#include "frame.h"
#include "justification_dialog.h"
#include "panels/character_panel.h"
#include "panels/numeric_field.h"
#include "panels/paragraph_panel.h"
#include "selftest_paint_fixture.h"
#include "type_fonts.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

const QPointF kOnLayer(30, 50);
const QPointF kBelowLayer(20, 130);
// Mid-canvas, so centred text stays on it.
const QPointF kMaskStart(100, 130);

QImage seed()
{
    QImage image(200, 160, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

// The options the Type tools share, which outlive a test.
void resetTypeOptions(Fixture& f)
{
    auto group = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType);
    pictura::TypeOptions o = group->get();
    o.size = 30.0;
    o.justification = 0;
    group->set(o);
}

void commit(Fixture& f) { QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier); }

void typeSize(pictura::CharacterPanel* panel, const QString& text)
{
    panel->sizeFieldForTest()->setEditText(text);
    emit panel->sizeFieldForTest()->lineEdit()->editingFinished();
}

void setFirstLineIndent(pictura::ParagraphPanel* panel, double value)
{
    panel->firstLineIndentFieldForTest()->setValue(value);
    emit panel->firstLineIndentFieldForTest()->editingFinished();
}

// The first type layer other than `except`, or empty.
QString otherTypeLayer(const Fixture& f, const QString& except)
{
    for (int i = 0; i < f.view->layer_count(); ++i) {
        const QString path = QString::number(i);
        if (path != except && pictura::type_api::readLayer(*f.view, path).isType) {
            return path;
        }
    }
    return QString();
}

pictura::type_api::TypeSpec specOf(const Fixture& f, const QString& path)
{
    return pictura::type_api::readLayer(*f.view, path).spec;
}

// A one-item menu carrying `id`, so a panel's menu sync reaches that action.
QAction* addMenuItem(QMenu& menu, const QString& id)
{
    QAction* action = menu.addAction(id);
    action->setData(id);
    return action;
}

} // namespace

class TypeSessionPanelsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void reopenedCharacterEditsSession();
    void reopenedCharacterCancelKeepsLayer();
    void paragraphEditsNewText();
    void paragraphEditsReopenedLayer();
    void panelsEditMaskSession();
    void menuActionsEditSession();
    void optionsBarEditSurvivesPanelEdit();

private:
    QString makeTypeLayer(Fixture& f);
    void reopen(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::CharacterPanel* character_ = nullptr;
    pictura::ParagraphPanel* paragraph_ = nullptr;
};

void TypeSessionPanelsTest::initTestCase()
{
    pictura::registerBundledFonts();
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    character_ = window_->findChild<pictura::CharacterPanel*>();
    paragraph_ = window_->findChild<pictura::ParagraphPanel*>();
    QVERIFY(character_ && paragraph_);
}

void TypeSessionPanelsTest::init()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

QString TypeSessionPanelsTest::makeTypeLayer(Fixture& f)
{
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    resetTypeOptions(f);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    commit(f);
    character_->setView(f.view);
    paragraph_->setView(f.view);
    return otherTypeLayer(f, QString());
}

void TypeSessionPanelsTest::reopen(Fixture& f)
{
    f.drag({kOnLayer});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    character_->refresh();
    paragraph_->refresh();
}

void TypeSessionPanelsTest::reopenedCharacterEditsSession()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_session_character"));
    QVERIFY2(f.ok(), "type fixture");
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const pictura::type_api::TypeSpec before = specOf(f, path);
    const int base = f.view->history_index();
    reopen(f);
    QVERIFY(character_->editingEnabledForTest());

    const QImage plain = f.canvas->typeOverlayImageForTest();
    QVERIFY2(!plain.isNull(), "the text previews");
    typeSize(character_, QStringLiteral("48"));
    const QImage large = f.canvas->typeOverlayImageForTest();
    QVERIFY2(large != plain, "the size previews at once");

    auto* antiAlias = character_->findChild<QComboBox*>(QStringLiteral("characterAntiAlias"));
    QVERIFY(antiAlias);
    const int method = before.character.anti_alias == 0 ? 1 : 0;
    antiAlias->setCurrentIndex(method);
    const QImage smoothed = f.canvas->typeOverlayImageForTest();
    QVERIFY2(smoothed != large, "anti-aliasing previews at once");

    auto* family = character_->findChild<QFontComboBox*>(QStringLiteral("characterFamily"));
    QVERIFY(family);
    const QString picked = QStringLiteral("Noto Sans");
    QVERIFY2(pictura::familyForFontName(before.font) != picked, "a different family");
    const int familyIndex = family->findText(picked, Qt::MatchFixedString);
    QVERIFY2(familyIndex >= 0, "the bundled UI family is installed");
    family->setCurrentIndex(familyIndex);
    const QImage reset = f.canvas->typeOverlayImageForTest();
    QVERIFY2(reset != smoothed, "the family previews at once");

    character_->styleButtonForTest(0)->click();
    QVERIFY2(f.canvas->typeOverlayImageForTest() != reset, "Faux Bold previews at once");

    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(specOf(f, path).character.size, before.character.size);

    commit(f);
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    const pictura::type_api::TypeSpec after = specOf(f, path);
    QCOMPARE(after.character.size, 48.0);
    QCOMPARE(int(after.character.anti_alias), method);
    QCOMPARE(pictura::familyForFontName(after.font), picked);
    QVERIFY(after.character.faux_bold);
}

void TypeSessionPanelsTest::reopenedCharacterCancelKeepsLayer()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_session_cancel"));
    QVERIFY2(f.ok(), "type fixture");
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const int base = f.view->history_index();
    reopen(f);
    typeSize(character_, QStringLiteral("48"));
    character_->styleButtonForTest(0)->click();
    setFirstLineIndent(paragraph_, 20.0);

    QVERIFY(f.tools->textSession()->cancelText());
    QCOMPARE(f.view->history_index(), base);
    const pictura::type_api::TypeSpec after = specOf(f, path);
    QCOMPARE(after.character.size, 30.0);
    QVERIFY(!after.character.faux_bold);
    QCOMPARE(after.paragraph.first_line_indent, 0.0);
}

void TypeSessionPanelsTest::paragraphEditsNewText()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_session_para_new"));
    QVERIFY2(f.ok(), "type fixture");
    const QString old = makeTypeLayer(f);
    QVERIFY2(!old.isEmpty(), "a type layer");
    const int base = f.view->history_index();

    f.drag({kBelowLayer});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    paragraph_->refresh();
    QTest::keyClicks(f.canvas, QStringLiteral("New"));
    const QLineF caret = f.canvas->typeOverlayCaretForTest();
    paragraph_->justifyFieldForTest()->setCurrentIndex(1);
    QVERIFY2(f.canvas->typeOverlayCaretForTest() != caret, "the justification previews at once");
    setFirstLineIndent(paragraph_, 20.0);
    QCOMPARE(f.view->history_index(), base);

    commit(f);
    const QString added = otherTypeLayer(f, old);
    QVERIFY2(!added.isEmpty(), "the new text is committed");
    QCOMPARE(specOf(f, added).paragraph.first_line_indent, 20.0);
    QCOMPARE(specOf(f, old).paragraph.first_line_indent, 0.0);
}

void TypeSessionPanelsTest::paragraphEditsReopenedLayer()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_session_para_reopen"));
    QVERIFY2(f.ok(), "type fixture");
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const int base = f.view->history_index();
    reopen(f);

    const QLineF caret = f.canvas->typeOverlayCaretForTest();
    paragraph_->justifyFieldForTest()->setCurrentIndex(1);
    QVERIFY2(f.canvas->typeOverlayCaretForTest() != caret, "the justification previews at once");
    setFirstLineIndent(paragraph_, 20.0);
    paragraph_->justifyFieldForTest()->setCurrentIndex(2);
    QCOMPARE(paragraph_->firstLineIndentFieldForTest()->value(), 20.0);
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(specOf(f, path).paragraph.first_line_indent, 0.0);

    commit(f);
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(specOf(f, path).paragraph.first_line_indent, 20.0);
    QCOMPARE(int(specOf(f, path).paragraph.justify), 2);
}

void TypeSessionPanelsTest::panelsEditMaskSession()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_session_mask"));
    QVERIFY2(f.ok(), "type fixture");
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const pictura::type_api::TypeSpec before = specOf(f, path);

    window_->setActiveTool(pictura::ToolId::HorizontalTypeMask);
    f.drag({kMaskStart});
    QTest::keyClicks(f.canvas, QStringLiteral("MASK"));
    commit(f);
    QVERIFY(f.view->has_selection());
    const int plain = f.view->selection_count();
    QVERIFY(f.view->undo());

    const int base = f.view->history_index();
    f.drag({kMaskStart});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    character_->refresh();
    paragraph_->refresh();
    QTest::keyClicks(f.canvas, QStringLiteral("MASK"));
    const QImage shown = f.canvas->typeOverlayImageForTest();
    typeSize(character_, QStringLiteral("48"));
    QVERIFY2(f.canvas->typeOverlayImageForTest() != shown, "the size previews at once");
    const QLineF caret = f.canvas->typeOverlayCaretForTest();
    paragraph_->justifyFieldForTest()->setCurrentIndex(1);
    QVERIFY2(f.canvas->typeOverlayCaretForTest() != caret, "the justification previews at once");
    setFirstLineIndent(paragraph_, 20.0);
    QCOMPARE(f.view->history_index(), base);

    commit(f);
    QVERIFY2(f.committedOnce(base, "Horizontal Type Mask"), "one mask state");
    QVERIFY2(f.view->selection_count() > plain, "the larger text makes the larger selection");
    const pictura::type_api::TypeSpec after = specOf(f, path);
    QCOMPARE(after.character.size, before.character.size);
    QCOMPARE(after.paragraph.first_line_indent, before.paragraph.first_line_indent);
}

void TypeSessionPanelsTest::menuActionsEditSession()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_session_menu"));
    QVERIFY2(f.ok(), "type fixture");
    character_->setView(f.view);
    paragraph_->setView(f.view);
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    resetTypeOptions(f);

    // No type layer exists: only the session enables the items.
    f.drag({kBelowLayer});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QTest::keyClicks(f.canvas, QStringLiteral("New"));
    character_->refresh();
    paragraph_->refresh();
    QMenu menu;
    QAction* resetCharacter = addMenuItem(menu, QStringLiteral("character.reset"));
    QAction* resetParagraph = addMenuItem(menu, QStringLiteral("paragraph.reset"));
    QAction* justification = addMenuItem(menu, QStringLiteral("paragraph.justification"));
    resetCharacter->setEnabled(false);
    resetParagraph->setEnabled(false);
    justification->setEnabled(false);
    character_->updatePanelMenu(&menu);
    paragraph_->updatePanelMenu(&menu);
    QVERIFY(resetCharacter->isEnabled());
    QVERIFY(resetParagraph->isEnabled());
    QVERIFY(justification->isEnabled());

    const int base = f.view->history_index();
    character_->styleButtonForTest(0)->click();
    setFirstLineIndent(paragraph_, 20.0);

    QTimer::singleShot(0, [&] {
        auto* dialog = paragraph_->findChild<pictura::JustificationDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->fieldForTest(1)->setValue(110.0);
        dialog->accept();
    });
    paragraph_->performPanelMenuAction(QStringLiteral("paragraph.justification"));
    QCOMPARE(paragraph_->firstLineIndentFieldForTest()->value(), 20.0);

    character_->performPanelMenuAction(QStringLiteral("character.reset"));
    QVERIFY2(!character_->styleButtonForTest(0)->isChecked(), "Reset Character clears Faux Bold");
    QCOMPARE(f.view->history_index(), base);

    commit(f);
    QString added = otherTypeLayer(f, QString());
    QVERIFY2(!added.isEmpty(), "the text is committed");
    pictura::type_api::TypeSpec spec = specOf(f, added);
    QVERIFY(!spec.character.faux_bold);
    QCOMPARE(spec.character.size, 30.0);
    QCOMPARE(spec.paragraph.word_spacing_desired, 1.1);
    QCOMPARE(spec.paragraph.first_line_indent, 20.0);

    // Reset Paragraph, on the reopened layer, adds nothing until the commit.
    const int reopened = f.view->history_index();
    f.drag({QPointF(35, 120)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    paragraph_->refresh();
    paragraph_->performPanelMenuAction(QStringLiteral("paragraph.reset"));
    QCOMPARE(paragraph_->firstLineIndentFieldForTest()->value(), 0.0);
    QCOMPARE(specOf(f, added).paragraph.first_line_indent, 20.0);
    QCOMPARE(f.view->history_index(), reopened);
    commit(f);
    QVERIFY2(f.committedOnce(reopened, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(specOf(f, added).paragraph.first_line_indent, 0.0);
}

void TypeSessionPanelsTest::optionsBarEditSurvivesPanelEdit()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_session_bar"));
    QVERIFY2(f.ok(), "type fixture");
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    reopen(f);

    auto group = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType);
    pictura::TypeOptions o = group->get();
    o.size = 50.0;
    o.justification = 1;
    group->set(o);
    character_->styleButtonForTest(0)->click();
    setFirstLineIndent(paragraph_, 20.0);
    QCOMPARE(group->get().size, 50.0);
    QCOMPARE(group->get().justification, 1);

    commit(f);
    const pictura::type_api::TypeSpec after = specOf(f, path);
    QCOMPARE(after.character.size, 50.0);
    QVERIFY(after.character.faux_bold);
    QCOMPARE(int(after.paragraph.justify), 1);
    QCOMPARE(after.paragraph.first_line_indent, 20.0);
}

QTEST_MAIN(TypeSessionPanelsTest)
#include "tst_type_session_panels.moc"
