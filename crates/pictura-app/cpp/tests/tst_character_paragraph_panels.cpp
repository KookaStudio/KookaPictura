// The Character and Paragraph panels (#73, #74): live editors over the active
// type layer, and the Window / Type / options-bar toggles that open them.

#include <QtTest/QtTest>

#include <QtCore/QTimer>

#include "commands.h"
#include "frame.h"
#include "fonts.h"
#include "panels/character_panel.h"
#include "panels/numeric_field.h"
#include "panels/panel_group.h"
#include "panels/paragraph_panel.h"
#include "justification_dialog.h"
#include "selftest_paint_fixture.h"
#include "type_fonts.h"

#include "api/type_api.h"

#include <QtGui/QAction>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
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

// Type `text` into an editable combo and commit it as Enter would.
void typeInto(QComboBox* combo, const QString& text)
{
    combo->setEditText(text);
    emit combo->lineEdit()->editingFinished();
}

// Pick the item reading `text`, as a click in the popup does.
void pick(QComboBox* combo, const QString& text)
{
    const int index = combo->findText(text);
    QVERIFY2(index >= 0, qPrintable(text));
    combo->setCurrentIndex(index);
    emit combo->activated(index);
}

// Press on `widget`, drag `dx` pixels right in 1 px moves, release.
void scrub(QWidget* widget, int dx)
{
    const QPointF local(widget->width() / 2.0, widget->height() / 2.0);
    const QPointF global = widget->mapToGlobal(local);
    const auto send = [widget](QEvent::Type type, QPointF at, QPointF globalAt,
                               Qt::MouseButtons buttons) {
        QMouseEvent event(type, at, globalAt, Qt::LeftButton, buttons, Qt::NoModifier);
        QCoreApplication::sendEvent(widget, &event);
    };
    send(QEvent::MouseButtonPress, local, global, Qt::LeftButton);
    for (int x = 1; x <= dx; ++x) {
        send(QEvent::MouseMove, local + QPointF(x, 0), global + QPointF(x, 0), Qt::LeftButton);
    }
    send(QEvent::MouseButtonRelease, local + QPointF(dx, 0), global + QPointF(dx, 0),
         Qt::NoButton);
}

// The live per-panel menu from the group's header button. Making the panel the
// current tab rebuilds the button's menu for that panel.
QMenu* livePanelMenu(pictura::PicturaMainWindow& frame, const QString& panelName)
{
    for (pictura::PanelGroup* group : frame.findChildren<pictura::PanelGroup*>()) {
        if (group->containsPanel(panelName)) {
            group->setCurrentPanel(panelName);
            break;
        }
    }
    auto* button = frame.findChild<QToolButton*>(QStringLiteral("panelWidgetMenu_") + panelName);
    return button ? button->menu() : nullptr;
}

QAction* menuAction(QMenu* menu, const QString& id)
{
    for (QAction* action : menu->actions()) {
        if (action->data().toString() == id) {
            return action;
        }
        if (QMenu* submenu = action->menu()) {
            if (QAction* found = menuAction(submenu, id)) {
                return found;
            }
        }
    }
    return nullptr;
}

QAction* menuText(QMenu* menu, const QString& text)
{
    for (QAction* action : menu->actions()) {
        if (action->text() == text) {
            return action;
        }
    }
    return nullptr;
}

QStringList menuTexts(QMenu* menu)
{
    QStringList out;
    for (QAction* action : menu->actions()) {
        if (!action->isSeparator()) {
            out << action->text();
        }
    }
    return out;
}

QStringList submenuTexts(QMenu* menu, const QString& text)
{
    QAction* parent = menuText(menu, text);
    if (!parent || !parent->menu()) {
        return {};
    }
    QStringList out;
    for (QAction* action : parent->menu()->actions()) {
        if (!action->isSeparator()) {
            out << action->text();
        }
    }
    return out;
}

// Restores the Horizontal Type options a test's type layer changed.
struct TypeOptionsGuard {
    explicit TypeOptionsGuard(Fixture& fixture)
        : f(fixture)
        , saved(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get())
    {
    }
    ~TypeOptionsGuard()
    {
        f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
    }
    Fixture& f;
    pictura::TypeOptions saved;
};

pictura::CharacterSetting layerCharacter(const Fixture& f, const QString& path)
{
    return pictura::type_api::layerCharacterSetting(*f.view, path);
}

} // namespace

class CharacterParagraphPanelsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void characterPanelReflectsAndEdits();
    void characterDefaultsWithoutType();
    void characterPanelEditsNewText();
    void fontStyleApplies();
    void styleFieldListsFamilyFaces();
    void languageRoundTrips();
    void tsumeFollowsAsianPreference();
    void fractionalWidthsMenu();
    void characterMenuMatchesCs6();
    void characterMenuCheckStates();
    void characterMenuToggles();
    void characterMenuReset();
    void styleButtonAppliesToggle();
    void capsButtonsAreExclusive();
    void positionButtonsAreExclusive();
    void openTypeButtonAppliesFeature();
    void openTypeButtonsFollowTheFont();
    void kerningPicksOptical();
    void kerningTypedNumberIsManual();
    void leadingPicksAuto();
    void scrubIconRaisesValue();
    void scrubKerningLeavesMetrics();
    void paragraphPanelReflectsAndEdits();
    void justificationDialogRoundTrips();
    void justificationCancelWritesNothing();
    void paragraphMenuMatchesCs6();
    void paragraphMenuCheckStates();
    void paragraphMenuToggles();
    void paragraphMenuReset();
    void menuDisabledRows();
    void menuCloseEntriesLast();
    void directionFollowsMiddleEasternPreference();
    void rtlDirectionApplies();
    void menusTogglePanels();
    void optionsBarTogglesBoth();

private:
    void showFrame();
    QString makeTypeLayer(Fixture& f);
    // Make a type layer in `f`, point the Character panel at it, and return
    // the panel (the layer's path in `path`).
    pictura::CharacterPanel* characterPanelOn(Fixture& f, QString& path);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CharacterParagraphPanelsTest::initTestCase()
{
    pictura::registerBundledFonts();
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
    pictura::TypeOptions o = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
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
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");

    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    // The panel reflects the selected layer.
    QVERIFY(panel->editingEnabledForTest());
    QCOMPARE(panel->sizeFieldForTest()->currentText(), QStringLiteral("30 px"));
    QCOMPARE(panel->trackingFieldForTest()->currentText(), QStringLiteral("0"));
    QCOMPARE(panel->kerningFieldForTest()->currentText(), QStringLiteral("Metrics"));
    QCOMPARE(panel->underlineForTest()->isChecked(), false);
    QCOMPARE(panel->languageFieldForTest()->currentText(), QStringLiteral("English: USA"));
    panel->setShowAsianOptions(true);
    panel->refresh();
    QVERIFY(panel->tsumeFieldForTest()->isVisibleTo(panel));
    QCOMPARE(panel->tsumeFieldForTest()->value(), 0.0);
    panel->setShowAsianOptions(false);

    // Refreshing the controls from the layer records nothing.
    const int shownBase = f.view->history_index();
    panel->refresh();
    panel->setView(f.view);
    QCOMPARE(f.view->history_index(), shownBase);

    // A committed size is one state; one undo restores it.
    const int base = f.view->history_index();
    typeInto(panel->sizeFieldForTest(), QStringLiteral("48"));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(panel->sizeFieldForTest()->currentText(), QStringLiteral("48 px"));
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 48.0);
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).size, 30.0);

    // A toggle edit is also one state.
    panel->refresh();
    const int toggleBase = f.view->history_index();
    panel->underlineForTest()->setChecked(true);
    QVERIFY2(f.committedOnce(toggleBase, "Edit Type Layer"), "one toggle state");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).underline, true);

    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
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
    const pictura::CharacterSetting defaults = pictura::type_api::defaultCharacterSetting();
    QVERIFY(!panel->editingEnabledForTest());
    QCOMPARE(panel->sizeFieldForTest()->currentText(),
             QStringLiteral("%1 px").arg(defaults.size));
    QCOMPARE(panel->trackingFieldForTest()->currentText(), QString::number(defaults.tracking));
    for (int i = 0; i < 8; ++i) {
        QVERIFY2(!panel->openTypeButtonForTest(i)->isEnabled(), qPrintable(QString::number(i)));
        QVERIFY(!panel->styleButtonForTest(i)->isEnabled());
    }

    // Committing while disabled records nothing.
    const int base = f.view->history_index();
    panel->commitForTest();
    QCOMPARE(f.view->history_index(), base);
}

// While new type is being set (no layer yet), the panel edits that text: a
// style toggle previews at once and lands on the new layer, not on the layer
// selected before.
void CharacterParagraphPanelsTest::characterPanelEditsNewText()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_new_text"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString old = makeTypeLayer(f);
    QVERIFY2(!old.isEmpty(), "a type layer");

    frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 130)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();
    QVERIFY(panel->editingEnabledForTest());
    QVERIFY(!panel->styleButtonForTest(0)->isChecked());

    const QImage plain = f.canvas->typeOverlayImageForTest();
    QVERIFY2(!plain.isNull(), "the placeholder previews");
    panel->styleButtonForTest(0)->click();
    const QImage bold = f.canvas->typeOverlayImageForTest();
    QVERIFY2(bold != plain, "Faux Bold previews at once");
    panel->underlineForTest()->click();
    QVERIFY2(f.canvas->typeOverlayImageForTest() != bold, "Underline previews at once");

    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QString added;
    for (int i = 0; i < f.view->layer_count(); ++i) {
        const QString path = QString::number(i);
        if (path != old && pictura::type_api::isType(*f.view, path)) {
            added = path;
        }
    }
    QVERIFY2(!added.isEmpty(), "the new text is committed");
    const pictura::CharacterSetting committed
        = pictura::type_api::layerCharacterSetting(*f.view, added);
    QVERIFY(committed.faux_bold);
    QVERIFY(committed.underline);
    const pictura::CharacterSetting untouched
        = pictura::type_api::layerCharacterSetting(*f.view, old);
    QVERIFY2(!untouched.faux_bold && !untouched.underline, "the old layer is unchanged");
}

void CharacterParagraphPanelsTest::paragraphPanelReflectsAndEdits()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_panel"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
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
    panel->setMiddleEastern(true);
    panel->refresh();
    QCOMPARE(panel->directionLtrForTest()->isChecked(), shown.direction == 0);
    QCOMPARE(panel->directionRtlForTest()->isChecked(), shown.direction == 1);
    panel->setMiddleEastern(false);

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

    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void CharacterParagraphPanelsTest::justificationDialogRoundTrips()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_justify"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    QMenu* menu = livePanelMenu(frame, QStringLiteral("paragraphPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* action = menuAction(menu, QStringLiteral("paragraph.justification"));
    QVERIFY(action);

    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::JustificationDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->fieldForTest(1)->setValue(110.0);
        dialog->fieldForTest(3)->setValue(-5.0);
        dialog->fieldForTest(8)->setValue(120.0);
        dialog->fieldForTest(9)->setValue(130.0);
        dialog->accept();
    });
    const int base = f.view->history_index();
    action->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    const pictura::ParagraphSetting p =
        pictura::type_api::layerParagraphSetting(*f.view, path);
    QCOMPARE(p.word_spacing_desired, 1.1);
    QCOMPARE(p.letter_spacing_min, -0.05);
    QCOMPARE(p.glyph_spacing_max, 1.2);
    QCOMPARE(p.auto_leading, 130.0);
    QVERIFY(f.view->undo());
}

void CharacterParagraphPanelsTest::justificationCancelWritesNothing()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_justify_cancel"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    QMenu* menu = livePanelMenu(frame, QStringLiteral("paragraphPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* action = menuAction(menu, QStringLiteral("paragraph.justification"));
    QVERIFY(action);

    const pictura::ParagraphSetting before =
        pictura::type_api::layerParagraphSetting(*f.view, path);
    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::JustificationDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->fieldForTest(1)->setValue(150.0);
        dialog->reject();
    });
    const int base = f.view->history_index();
    action->trigger();
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).word_spacing_desired,
             before.word_spacing_desired);
}

void CharacterParagraphPanelsTest::paragraphMenuMatchesCs6()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_menu_rows"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    QMenu* menu = livePanelMenu(frame, QStringLiteral("paragraphPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    const QStringList expected = {
        QStringLiteral("Roman Hanging Punctuation"),
        QStringLiteral("Justification…"),
        QStringLiteral("Hyphenation…"),
        QStringLiteral("Latin and East Asian Layout"),
        QStringLiteral("World-Ready Layout"),
        QStringLiteral("Default Features"),
        QStringLiteral("East Asian Features"),
        QStringLiteral("Middle Eastern Features"),
        QStringLiteral("Single-line Composer"),
        QStringLiteral("Every-line Composer"),
        QStringLiteral("Reset Paragraph"),
        QStringLiteral("Close"),
        QStringLiteral("Close Tab Group"),
    };
    QCOMPARE(menuTexts(menu), expected);

    const QStringList disabled = {
        QStringLiteral("Hyphenation…"),
        QStringLiteral("Latin and East Asian Layout"),
        QStringLiteral("World-Ready Layout"),
        QStringLiteral("Default Features"),
        QStringLiteral("East Asian Features"),
        QStringLiteral("Middle Eastern Features"),
    };
    for (const QString& label : disabled) {
        QAction* action = menuText(menu, label);
        QVERIFY2(action, qPrintable(label));
        QVERIFY2(!action->isEnabled(), qPrintable(label));
        QVERIFY2(action->toolTip().contains(QStringLiteral("not implemented yet")),
                 qPrintable(label));
    }
    QVERIFY(menuText(menu, QStringLiteral("Latin and East Asian Layout"))->isChecked());
    QVERIFY(menuText(menu, QStringLiteral("Default Features"))->isChecked());
}

void CharacterParagraphPanelsTest::paragraphMenuCheckStates()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_menu_state"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* character = characterPanelOn(f, path);
    QVERIFY2(character, "a type layer and the panel");
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    QMenu* menu = livePanelMenu(frame, QStringLiteral("paragraphPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* hanging = menuAction(menu, QStringLiteral("paragraph.hanging"));
    QAction* single = menuAction(menu, QStringLiteral("paragraph.singleLineComposer"));
    QAction* every = menuAction(menu, QStringLiteral("paragraph.everyLineComposer"));
    QVERIFY(hanging && single && every);
    QVERIFY(!hanging->isChecked());
    QVERIFY(single->isChecked());
    QVERIFY(!every->isChecked());
}

void CharacterParagraphPanelsTest::paragraphMenuToggles()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_menu_toggle"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* character = characterPanelOn(f, path);
    QVERIFY2(character, "a type layer and the panel");
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    QMenu* menu = livePanelMenu(frame, QStringLiteral("paragraphPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* hanging = menuAction(menu, QStringLiteral("paragraph.hanging"));
    QVERIFY(hanging && !hanging->isChecked());
    int base = f.view->history_index();
    hanging->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one hanging state");
    QVERIFY(pictura::type_api::layerParagraphSetting(*f.view, path).hanging);
    QVERIFY(hanging->isChecked());
    QVERIFY(f.view->undo());
    QVERIFY(!pictura::type_api::layerParagraphSetting(*f.view, path).hanging);

    panel->refresh();
    panel->updatePanelMenu(menu);
    QAction* single = menuAction(menu, QStringLiteral("paragraph.singleLineComposer"));
    QAction* every = menuAction(menu, QStringLiteral("paragraph.everyLineComposer"));
    QVERIFY(single && every);
    QVERIFY(single->isChecked());
    QVERIFY(!every->isChecked());
    base = f.view->history_index();
    every->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one composer state");
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).composer, 1);
    panel->updatePanelMenu(menu);
    QVERIFY(every->isChecked());
    QVERIFY(!single->isChecked());
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).composer, 0);
}

void CharacterParagraphPanelsTest::paragraphMenuReset()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_menu_reset"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* character = characterPanelOn(f, path);
    QVERIFY2(character, "a type layer and the panel");
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    character->styleButtonForTest(0)->click();
    auto* hangingBox = panel->findChild<QCheckBox*>(QStringLiteral("paragraphHanging"));
    QVERIFY(hangingBox);
    hangingBox->setChecked(true);
    panel->refresh();
    const pictura::ParagraphSetting before =
        pictura::type_api::layerParagraphSetting(*f.view, path);
    QVERIFY(before.hanging);
    const QString text = pictura::type_api::layerText(*f.view, path);
    const QString family =
        pictura::familyForFontName(pictura::type_api::layerFont(*f.view, path));

    QMenu* menu = livePanelMenu(frame, QStringLiteral("paragraphPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* reset = menuAction(menu, QStringLiteral("paragraph.reset"));
    QVERIFY(reset && reset->isEnabled());
    const int base = f.view->history_index();
    reset->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Reset Paragraph state");

    const pictura::ParagraphSetting defaults = pictura::type_api::defaultParagraphSetting();
    const pictura::ParagraphSetting after =
        pictura::type_api::layerParagraphSetting(*f.view, path);
    QCOMPARE(after.hanging, defaults.hanging);
    QCOMPARE(after.composer, defaults.composer);
    QCOMPARE(after.justify, defaults.justify);
    QVERIFY(layerCharacter(f, path).faux_bold);
    QCOMPARE(pictura::type_api::layerText(*f.view, path), text);
    QCOMPARE(pictura::familyForFontName(pictura::type_api::layerFont(*f.view, path)), family);
    QVERIFY(f.view->undo());
    QVERIFY(pictura::type_api::layerParagraphSetting(*f.view, path).hanging);
}

void CharacterParagraphPanelsTest::menuDisabledRows()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_menu_disabled"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* character = frame.findChild<pictura::CharacterPanel*>();
    auto* paragraph = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(character && paragraph);
    character->setView(f.view);
    character->refresh();
    paragraph->setView(f.view);
    paragraph->refresh();

    QMenu* characterMenu = livePanelMenu(frame, QStringLiteral("characterPanel"));
    QVERIFY(characterMenu);
    character->updatePanelMenu(characterMenu);
    for (const QString& label : {QStringLiteral("No Break"),
                                 QStringLiteral("Change Text Orientation"),
                                 QStringLiteral("Middle Eastern Features")}) {
        QAction* action = menuText(characterMenu, label);
        QVERIFY2(action, qPrintable(label));
        QVERIFY2(!action->isEnabled(), qPrintable(label));
        QVERIFY2(action->toolTip().contains(QStringLiteral("not implemented yet")),
                 qPrintable(label));
    }

    QMenu* paragraphMenu = livePanelMenu(frame, QStringLiteral("paragraphPanel"));
    QVERIFY(paragraphMenu);
    paragraph->updatePanelMenu(paragraphMenu);
    for (const QString& label : {QStringLiteral("Hyphenation…"),
                                 QStringLiteral("Latin and East Asian Layout"),
                                 QStringLiteral("World-Ready Layout"),
                                 QStringLiteral("Default Features"),
                                 QStringLiteral("East Asian Features"),
                                 QStringLiteral("Middle Eastern Features")}) {
        QAction* action = menuText(paragraphMenu, label);
        QVERIFY2(action, qPrintable(label));
        QVERIFY2(!action->isEnabled(), qPrintable(label));
        QVERIFY2(action->toolTip().contains(QStringLiteral("not implemented yet")),
                 qPrintable(label));
    }
}

void CharacterParagraphPanelsTest::menuCloseEntriesLast()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_menu_close_last"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    QVERIFY2(!makeTypeLayer(f).isEmpty(), "a type layer");

    const auto assertLast = [&frame](const QString& panelName) {
        QMenu* menu = livePanelMenu(frame, panelName);
        if (!menu) {
            return false;
        }
        const QStringList texts = menuTexts(menu);
        return texts.size() >= 2
            && texts.mid(texts.size() - 2)
                == QStringList{QStringLiteral("Close"), QStringLiteral("Close Tab Group")};
    };
    QVERIFY(assertLast(QStringLiteral("characterPanel")));
    QVERIFY(assertLast(QStringLiteral("paragraphPanel")));
}

void CharacterParagraphPanelsTest::directionFollowsMiddleEasternPreference()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_direction_visible"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    QVERIFY(!makeTypeLayer(f).isEmpty());
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->setMiddleEastern(false);
    QVERIFY(!panel->directionVisibleForTest());
    panel->setMiddleEastern(true);
    QVERIFY(panel->directionVisibleForTest());
    panel->setMiddleEastern(false);
    QVERIFY(!panel->directionVisibleForTest());
}

void CharacterParagraphPanelsTest::rtlDirectionApplies()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_rtl"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(panel);
    panel->setMiddleEastern(true);
    panel->setView(f.view);
    panel->refresh();
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).direction, 0);

    const int base = f.view->history_index();
    panel->directionRtlForTest()->click();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).direction, 1);
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerParagraphSetting(*f.view, path).direction, 0);
    panel->setMiddleEastern(false);
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

pictura::CharacterPanel* CharacterParagraphPanelsTest::characterPanelOn(Fixture& f, QString& path)
{
    showFrame();
    path = makeTypeLayer(f);
    auto* panel = window_->findChild<pictura::CharacterPanel*>();
    if (panel && !path.isEmpty()) {
        panel->setView(f.view);
        panel->refresh();
    }
    return path.isEmpty() ? nullptr : panel;
}

void CharacterParagraphPanelsTest::fontStyleApplies()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_style"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    auto* family = panel->findChild<QFontComboBox*>(QStringLiteral("characterFamily"));
    QVERIFY(family);
    const int familyIndex = family->findText(QStringLiteral("Noto Sans"), Qt::MatchFixedString);
    QVERIFY2(familyIndex >= 0, "the bundled UI family is installed");
    family->setCurrentIndex(familyIndex);
    const int bold = panel->styleFieldForTest()->findText(QStringLiteral("Bold"),
                                                           Qt::MatchFixedString);
    QVERIFY2(bold >= 0, "Noto Sans has a Bold face");

    const int base = f.view->history_index();
    panel->styleFieldForTest()->setCurrentIndex(bold);
    QCOMPARE(panel->styleFieldForTest()->currentText(), QStringLiteral("Bold"));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).font_style,
             QStringLiteral("Bold"));
    QVERIFY(f.view->undo());
    QVERIFY(pictura::type_api::layerCharacterSetting(*f.view, path).font_style
            != QStringLiteral("Bold"));
}

void CharacterParagraphPanelsTest::styleFieldListsFamilyFaces()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_faces"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    QVERIFY(!makeTypeLayer(f).isEmpty());
    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    auto* family = panel->findChild<QFontComboBox*>(QStringLiteral("characterFamily"));
    QVERIFY(family);
    const int familyIndex = family->findText(QStringLiteral("Noto Sans"), Qt::MatchFixedString);
    QVERIFY2(familyIndex >= 0, "the bundled UI family is installed");
    family->setCurrentIndex(familyIndex);
    QVERIFY(panel->styleFieldForTest()->count() >= 2);
    QVERIFY(panel->styleFieldForTest()->findText(QStringLiteral("Bold"),
                                                 Qt::MatchFixedString) >= 0);
}

void CharacterParagraphPanelsTest::languageRoundTrips()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_language"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    const QString before = pictura::type_api::layerCharacterSetting(*f.view, path).language;
    const int base = f.view->history_index();
    panel->languageFieldForTest()->setCurrentText(QStringLiteral("French"));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).language,
             QStringLiteral("French"));
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).language, before);
}

void CharacterParagraphPanelsTest::tsumeFollowsAsianPreference()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_tsume"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    panel->setShowAsianOptions(false);
    QVERIFY(!panel->tsumeFieldForTest()->isVisibleTo(panel));
    panel->setShowAsianOptions(true);
    QVERIFY(panel->tsumeFieldForTest()->isVisibleTo(panel));

    const int base = f.view->history_index();
    panel->tsumeFieldForTest()->setValue(50.0);
    emit panel->tsumeFieldForTest()->valueCommitted(50.0);
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).tsume, 50.0);
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).tsume, 0.0);
    panel->setShowAsianOptions(false);
}

void CharacterParagraphPanelsTest::fractionalWidthsMenu()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_fractional"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();
    QVERIFY(pictura::type_api::layerCharacterSetting(*f.view, path).fractional_widths);

    QMenu* menu = livePanelMenu(frame, QStringLiteral("characterPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* action = menuAction(menu, QStringLiteral("character.fractionalWidths"));
    QVERIFY(action);
    QVERIFY(action->isChecked());
    const int base = f.view->history_index();
    action->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QVERIFY(!pictura::type_api::layerCharacterSetting(*f.view, path).fractional_widths);
    QVERIFY(f.view->undo());
    QVERIFY(pictura::type_api::layerCharacterSetting(*f.view, path).fractional_widths);
}

void CharacterParagraphPanelsTest::characterMenuMatchesCs6()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_menu_rows"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    auto* panel = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();

    QMenu* menu = livePanelMenu(frame, QStringLiteral("characterPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);

    const QStringList expected = {
        QStringLiteral("Change Text Orientation"),
        QStringLiteral("OpenType"),
        QStringLiteral("Faux Bold"),
        QStringLiteral("Faux Italic"),
        QStringLiteral("All Caps"),
        QStringLiteral("Small Caps"),
        QStringLiteral("Superscript"),
        QStringLiteral("Subscript"),
        QStringLiteral("Underline"),
        QStringLiteral("Strikethrough"),
        QStringLiteral("Fractional Widths"),
        QStringLiteral("System Layout"),
        QStringLiteral("No Break"),
        QStringLiteral("Middle Eastern Features"),
        QStringLiteral("Reset Character"),
        QStringLiteral("Close"),
        QStringLiteral("Close Tab Group"),
    };
    QCOMPARE(menuTexts(menu), expected);
    QCOMPARE(submenuTexts(menu, QStringLiteral("OpenType")),
             (QStringList{QStringLiteral("Standard Ligatures"),
                          QStringLiteral("Contextual Alternates"),
                          QStringLiteral("Discretionary Ligatures"),
                          QStringLiteral("Swash"),
                          QStringLiteral("Stylistic Alternates"),
                          QStringLiteral("Titling Alternates"),
                          QStringLiteral("Ordinals"),
                          QStringLiteral("Fractions")}));
}

void CharacterParagraphPanelsTest::characterMenuCheckStates()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_menu_state"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    panel->styleButtonForTest(0)->click();
    int feature = 0;
    while (feature < 8 && !panel->openTypeButtonForTest(feature)->isEnabled()) {
        ++feature;
    }
    if (feature < 8) {
        // Standard Ligatures can be on in the layer's defaults; start from a
        // known unchecked state so the assertion proves the menu reflects the
        // click rather than the inherited default.
        if (panel->openTypeButtonForTest(feature)->isChecked()) {
            panel->openTypeButtonForTest(feature)->click();
            panel->refresh();
        }
        panel->openTypeButtonForTest(feature)->click();
    }
    panel->refresh();

    QMenu* menu = livePanelMenu(frame, QStringLiteral("characterPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* bold = menuAction(menu, QStringLiteral("character.fauxBold"));
    QAction* fractional = menuAction(menu, QStringLiteral("character.fractionalWidths"));
    QAction* system = menuAction(menu, QStringLiteral("character.systemLayout"));
    QVERIFY(bold && fractional && system);
    QVERIFY(bold->isChecked());
    QVERIFY(fractional->isChecked());
    QVERIFY(!system->isChecked());
    if (feature < 8) {
        QAction* openType = menuText(menu, QStringLiteral("OpenType"));
        QVERIFY(openType && openType->menu());
        QVERIFY(openType->menu()->actions().at(feature)->isChecked());
    }
}

void CharacterParagraphPanelsTest::characterMenuToggles()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_menu_toggle"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    QMenu* menu = livePanelMenu(frame, QStringLiteral("characterPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* bold = menuAction(menu, QStringLiteral("character.fauxBold"));
    QVERIFY(bold && bold->isEnabled());
    int base = f.view->history_index();
    bold->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Faux Bold state");
    QVERIFY(layerCharacter(f, path).faux_bold);
    QVERIFY(panel->styleButtonForTest(0)->isChecked());
    QVERIFY(bold->isChecked());
    QVERIFY(f.view->undo());
    QVERIFY(!layerCharacter(f, path).faux_bold);

    panel->refresh();
    panel->updatePanelMenu(menu);
    QAction* fractional = menuAction(menu, QStringLiteral("character.fractionalWidths"));
    QVERIFY(fractional && fractional->isChecked());
    base = f.view->history_index();
    fractional->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Fractional Widths state");
    QVERIFY(!layerCharacter(f, path).fractional_widths);
    QVERIFY(!fractional->isChecked());
    QVERIFY(f.view->undo());
    QVERIFY(layerCharacter(f, path).fractional_widths);

    panel->refresh();
    panel->updatePanelMenu(menu);
    QAction* system = menuAction(menu, QStringLiteral("character.systemLayout"));
    QVERIFY(system && !system->isChecked());
    base = f.view->history_index();
    system->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one System Layout state");
    QVERIFY(!layerCharacter(f, path).fractional_widths);
    QVERIFY(f.view->undo());
    QVERIFY(layerCharacter(f, path).fractional_widths);
}

void CharacterParagraphPanelsTest::characterMenuReset()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_char_menu_reset"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    panel->styleButtonForTest(0)->click();
    typeInto(panel->trackingFieldForTest(), QStringLiteral("120"));
    panel->refresh();
    const pictura::CharacterSetting before = layerCharacter(f, path);
    QVERIFY(before.faux_bold);
    QCOMPARE(before.tracking, 120.0);
    const QString text = pictura::type_api::layerText(*f.view, path);
    const QString family =
        pictura::familyForFontName(pictura::type_api::layerFont(*f.view, path));

    QMenu* menu = livePanelMenu(frame, QStringLiteral("characterPanel"));
    QVERIFY(menu);
    panel->updatePanelMenu(menu);
    QAction* reset = menuAction(menu, QStringLiteral("character.reset"));
    QVERIFY(reset && reset->isEnabled());
    const int base = f.view->history_index();
    reset->trigger();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Reset Character state");

    const pictura::CharacterSetting defaults = pictura::type_api::defaultCharacterSetting();
    const pictura::CharacterSetting after = layerCharacter(f, path);
    QVERIFY(!after.faux_bold);
    QCOMPARE(after.tracking, defaults.tracking);
    QCOMPARE(after.size, before.size);
    QCOMPARE(after.font_style, before.font_style);
    QCOMPARE(after.color, before.color);
    QCOMPARE(pictura::type_api::layerText(*f.view, path), text);
    QCOMPARE(pictura::familyForFontName(pictura::type_api::layerFont(*f.view, path)), family);
    QVERIFY(f.view->undo());
    QVERIFY(layerCharacter(f, path).faux_bold);
    QCOMPARE(layerCharacter(f, path).tracking, 120.0);
}

void CharacterParagraphPanelsTest::styleButtonAppliesToggle()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_style"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    const int base = f.view->history_index();
    panel->styleButtonForTest(0)->click();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Faux Bold state");
    QVERIFY(layerCharacter(f, path).faux_bold);
    QVERIFY(panel->styleButtonForTest(0)->isChecked());
}

void CharacterParagraphPanelsTest::capsButtonsAreExclusive()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_caps"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    panel->styleButtonForTest(2)->click();
    QVERIFY(layerCharacter(f, path).all_caps);
    panel->refresh();
    const int base = f.view->history_index();
    panel->styleButtonForTest(3)->click();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Small Caps state");
    QVERIFY(layerCharacter(f, path).small_caps);
    QVERIFY(!layerCharacter(f, path).all_caps);
    QVERIFY(!panel->styleButtonForTest(2)->isChecked());
}

void CharacterParagraphPanelsTest::positionButtonsAreExclusive()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_position"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    panel->styleButtonForTest(4)->click();
    QVERIFY(layerCharacter(f, path).superscript);
    panel->refresh();
    panel->styleButtonForTest(5)->click();
    QVERIFY(layerCharacter(f, path).subscript);
    QVERIFY(!layerCharacter(f, path).superscript);
    QVERIFY(!panel->styleButtonForTest(4)->isChecked());
}

void CharacterParagraphPanelsTest::openTypeButtonAppliesFeature()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_opentype"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    // The first button the font supports; else force one on to exercise the
    // wiring (the greying itself is openTypeButtonsFollowTheFont's).
    int index = 0;
    while (index < 8 && !panel->openTypeButtonForTest(index)->isEnabled()) {
        ++index;
    }
    if (index == 8) {
        index = 2;
        panel->openTypeButtonForTest(index)->setEnabled(true);
    }
    const auto flag = [&](const pictura::CharacterSetting& c) {
        const bool flags[] = {c.standard_ligatures,      c.contextual_alternates,
                              c.discretionary_ligatures, c.swash,
                              c.stylistic_alternates,    c.titling_alternates,
                              c.ordinals,                c.fractions};
        return flags[index];
    };
    const bool before = flag(layerCharacter(f, path));
    const int base = f.view->history_index();
    panel->openTypeButtonForTest(index)->click();
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one OpenType state");
    QCOMPARE(flag(layerCharacter(f, path)), !before);
}

void CharacterParagraphPanelsTest::openTypeButtonsFollowTheFont()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_opentype_font"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    const QString family =
        pictura::familyForFontName(pictura::type_api::layerFont(*f.view, path));
    const QString style = pictura::type_api::layerCharacterSetting(*f.view, path).font_style;
    pictura::registerTypeFont(family, style);
    const std::uint32_t features = pictura::type_api::openTypeFeatures(family, style);
    for (int i = 0; i < 8; ++i) {
        QCOMPARE(panel->openTypeButtonForTest(i)->isEnabled(), bool(features >> i & 1u));
    }
}

void CharacterParagraphPanelsTest::kerningPicksOptical()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_optical"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    QVERIFY(panel->kerningFieldForTest()->isEnabled());
    const int base = f.view->history_index();
    pick(panel->kerningFieldForTest(), QStringLiteral("Optical"));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one kerning state");
    QCOMPARE(layerCharacter(f, path).kerning_mode, 1);
}

void CharacterParagraphPanelsTest::kerningTypedNumberIsManual()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_manual_kern"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    const int base = f.view->history_index();
    typeInto(panel->kerningFieldForTest(), QStringLiteral("50"));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one kerning state");
    QCOMPARE(layerCharacter(f, path).kerning_mode, 2);
    QCOMPARE(layerCharacter(f, path).kerning_value, 50);
}

void CharacterParagraphPanelsTest::leadingPicksAuto()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_leading"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    typeInto(panel->leadingFieldForTest(), QStringLiteral("40"));
    QCOMPARE(layerCharacter(f, path).leading_mode, 1);
    QCOMPARE(layerCharacter(f, path).leading_value, 40.0);
    panel->refresh();
    const int base = f.view->history_index();
    pick(panel->leadingFieldForTest(), QStringLiteral("(Auto)"));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one leading state");
    QCOMPARE(layerCharacter(f, path).leading_mode, 0);
}

void CharacterParagraphPanelsTest::scrubIconRaisesValue()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_scrub"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    pictura::NumericField* field = panel->horizontalScaleFieldForTest();
    auto* icon = field->findChild<QLabel*>(QStringLiteral("characterHorizontalScaleLabel"));
    QVERIFY2(icon, "the horizontal-scale icon");
    QCOMPARE(field->value(), 100.0);
    const int base = f.view->history_index();
    scrub(icon, 20);
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one scrub state");
    QCOMPARE(field->value(), 120.0);
    QCOMPARE(layerCharacter(f, path).horizontal_scale, 120.0);
}

void CharacterParagraphPanelsTest::scrubKerningLeavesMetrics()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_char_scrub_kern"));
    QVERIFY2(f.ok(), "type fixture");
    TypeOptionsGuard guard(f);
    QString path;
    pictura::CharacterPanel* panel = characterPanelOn(f, path);
    QVERIFY2(panel, "a type layer and the panel");

    QCOMPARE(panel->kerningFieldForTest()->currentText(), QStringLiteral("Metrics"));
    const int base = f.view->history_index();
    scrub(panel->kerningIconForTest(), 10);
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one scrub state");
    QCOMPARE(layerCharacter(f, path).kerning_mode, 2);
    QCOMPARE(layerCharacter(f, path).kerning_value, 10);
    QCOMPARE(panel->kerningFieldForTest()->currentText(), QStringLiteral("10"));
}

QTEST_MAIN(CharacterParagraphPanelsTest)
#include "tst_character_paragraph_panels.moc"
