// The Character and Paragraph panels (#73, #74): live editors over the active
// type layer, and the Window / Type / options-bar toggles that open them.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "panels/character_panel.h"
#include "panels/numeric_field.h"
#include "panels/paragraph_panel.h"
#include "selftest_paint_fixture.h"
#include "type_fonts.h"

#include "api/type_api.h"

#include <QtGui/QAction>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
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

} // namespace

class CharacterParagraphPanelsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void characterPanelReflectsAndEdits();
    void characterDefaultsWithoutType();
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

namespace {

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
    pictura::registerTypeFont(family);
    const std::uint32_t features = pictura::type_api::openTypeFeatures(family);
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
