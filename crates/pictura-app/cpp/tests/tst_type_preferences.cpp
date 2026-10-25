// The Type preferences pane (#517/#518/#887): the East Asian and Middle
// Eastern options gate the panel controls, and Missing Glyph Protection gates
// per-character font fallback. The settings are process-wide and persist in the
// session store.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "fonts.h"
#include "panels/character_panel.h"
#include "panels/paragraph_panel.h"
#include "panels/numeric_field.h"
#include "preferences_dialog.h"
#include "selftest_paint_fixture.h"
#include "session.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "api/type_api.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>

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
        if (pictura::type_api::readLayer(*f.view, QString::number(i)).isType) {
            return QString::number(i);
        }
    }
    return QString();
}

} // namespace

class TypePreferencesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void defaults();
    void persists();
    void engineTogglesPanelVisibility();
    void tsumeAppearsWithAsianOptions();
    void directionAppearsWithMiddleEastern();
    void hidingKeepsStoredValue();
    void glyphProtectionTogglesFallback();

private:
    void showFrame();
    void openTypePreferences();
    QString makeTypeLayer(Fixture& f, const QString& text = QStringLiteral("Hello"));

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypePreferencesTest::initTestCase()
{
    pictura::registerBundledFonts();
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TypePreferencesTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

void TypePreferencesTest::openTypePreferences()
{
    window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditPreferencesType));
    QCoreApplication::processEvents();
}

QString TypePreferencesTest::makeTypeLayer(Fixture& f, const QString& text)
{
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    pictura::TypeOptions o =
        f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, text);
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    return typeLayer(f);
}

void TypePreferencesTest::defaults()
{
    pictura::PreferencesDialog dialog;
    QVERIFY(!dialog.checkboxForTest(QStringLiteral("showAsianTextOptions")));
    QVERIFY(!dialog.checkboxForTest(QStringLiteral("middleEastern")));
    QVERIFY(dialog.checkboxForTest(QStringLiteral("missingGlyphProtection")));
}

void TypePreferencesTest::persists()
{
    pictura::SessionState state = pictura::loadSession();
    state.typeShowAsianOptions = true;
    state.typeMiddleEastern = true;
    state.typeMissingGlyphProtection = false;
    QVERIFY(pictura::saveSession(state));

    const pictura::SessionState loaded = pictura::loadSession();
    QVERIFY(loaded.typeShowAsianOptions);
    QVERIFY(loaded.typeMiddleEastern);
    QVERIFY(!loaded.typeMissingGlyphProtection);

    pictura::type_api::setMissingGlyphProtection(loaded.typeMissingGlyphProtection);
    QVERIFY(!pictura::type_api::missingGlyphProtection());
    pictura::type_api::setMissingGlyphProtection(true);
}

void TypePreferencesTest::engineTogglesPanelVisibility()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_prefs_panels"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    QVERIFY(!makeTypeLayer(f).isEmpty());
    auto* paragraph = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(paragraph);
    paragraph->setView(f.view);
    paragraph->setMiddleEastern(false);
    QVERIFY(!paragraph->directionVisibleForTest());

    openTypePreferences();
    auto* prefs = frame.findChild<pictura::PreferencesDialog*>();
    QVERIFY(prefs);
    prefs->setCheckboxForTest(QStringLiteral("middleEastern"), true);
    QVERIFY(paragraph->directionVisibleForTest());
    prefs->setCheckboxForTest(QStringLiteral("middleEastern"), false);
    QVERIFY(!paragraph->directionVisibleForTest());
}

void TypePreferencesTest::tsumeAppearsWithAsianOptions()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_prefs_tsume"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    QVERIFY(!makeTypeLayer(f).isEmpty());
    auto* character = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(character);
    character->setView(f.view);
    character->setShowAsianOptions(false);
    QVERIFY(!character->tsumeFieldForTest()->isVisibleTo(character));

    openTypePreferences();
    auto* prefs = frame.findChild<pictura::PreferencesDialog*>();
    QVERIFY(prefs);
    prefs->setCheckboxForTest(QStringLiteral("showAsianTextOptions"), true);
    QVERIFY(character->tsumeFieldForTest()->isVisibleTo(character));
    prefs->setCheckboxForTest(QStringLiteral("showAsianTextOptions"), false);
}

void TypePreferencesTest::directionAppearsWithMiddleEastern()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_prefs_direction"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    QVERIFY(!makeTypeLayer(f).isEmpty());
    auto* paragraph = frame.findChild<pictura::ParagraphPanel*>();
    QVERIFY(paragraph);
    paragraph->setView(f.view);
    paragraph->setMiddleEastern(false);
    QVERIFY(!paragraph->directionVisibleForTest());
    paragraph->setMiddleEastern(true);
    QVERIFY(paragraph->directionVisibleForTest());
    paragraph->setMiddleEastern(false);
}

void TypePreferencesTest::hidingKeepsStoredValue()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_prefs_tsume_kept"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = makeTypeLayer(f);
    QVERIFY(!path.isEmpty());
    auto* character = frame.findChild<pictura::CharacterPanel*>();
    QVERIFY(character);
    character->setShowAsianOptions(true);
    character->setView(f.view);
    character->refresh();
    character->tsumeFieldForTest()->setValue(50.0);
    emit character->tsumeFieldForTest()->valueCommitted(50.0);
    QCOMPARE(pictura::type_api::readLayer(*f.view, path).spec.character.tsume, 50.0);

    character->setShowAsianOptions(false);
    character->setShowAsianOptions(true);
    QCOMPARE(character->tsumeFieldForTest()->value(), 50.0);
    character->setShowAsianOptions(false);
}

void TypePreferencesTest::glyphProtectionTogglesFallback()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_prefs_glyph"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    pictura::registerTypeFallbacks();
    const QString text = QStringLiteral("A") + QChar(0x0370);
    pictura::TypeSetting setting{};
    setting.x = 30.0;
    setting.y = 120.0;
    setting.xx = 1.0;
    setting.yy = 1.0;
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = 48.0;
    QVERIFY(!pictura::type_api::commitLayer(*f.view, {text, QStringLiteral("Liberation Sans"), setting, character, pictura::type_api::defaultParagraphSetting()}).isEmpty());
    f.frame.refresh();
    const QImage on = f.canvas->image();

    pictura::type_api::setMissingGlyphProtection(false);
    pictura::type_api::refreshMissingGlyphProtection(*f.view);
    f.frame.refresh();
    const QImage off = f.canvas->image();
    pictura::type_api::setMissingGlyphProtection(true);
    pictura::type_api::refreshMissingGlyphProtection(*f.view);
    QVERIFY2(on != off, "the fallback face changes the uncovered glyph");
}

QTEST_MAIN(TypePreferencesTest)
#include "tst_type_preferences.moc"
