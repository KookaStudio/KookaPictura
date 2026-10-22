// New type starts with "Lorem Ipsum Pictura" selected (Photoshop CC, not CS6;
// an owner decision for #684), and Edit > Preferences > Type turns it off.

#include <QtTest/QtTest>

#include "api/type_api.h"
#include "commands.h"
#include "preferences_dialog.h"
#include "selftest_paint_fixture.h"
#include "session.h"

#include <memory>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

const QString kPlaceholder = QStringLiteral("Lorem Ipsum Pictura");

QImage seed()
{
    QImage image(360, 240, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

// The type layers' paths, bottom to top.
QStringList typeLayers(const Fixture& f)
{
    QStringList out;
    for (int i = 0; i < f.view->layer_count(); ++i) {
        if (pictura::type_api::isType(*f.view, QString::number(i))) {
            out << QString::number(i);
        }
    }
    return out;
}

void commit(const Fixture& f)
{
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
}

} // namespace

class TypePlaceholderTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void dragFillsBox();
    void clickFillsPointType();
    void typingReplacesPlaceholder();
    void untouchedPlaceholderCommits();
    void maskAndReopenAreExempt();
    void typePreferencePage();
    void preferenceOffStartsEmpty();
    void preferenceIsSaved();

private:
    pictura::PreferencesDialog* openTypePreferences();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypePlaceholderTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void TypePlaceholderTest::cleanup()
{
    if (auto* prefs = window_->findChild<pictura::PreferencesDialog*>()) {
        prefs->setCheckboxForTest(QStringLiteral("fillPlaceholderText"), true);
        prefs->close();
    }
    if (auto* tools = window_->findChild<pictura::ToolController*>()) {
        tools->setTypePlaceholderText(true);
    }
    window_->setActiveTool(pictura::ToolId::Move);
}

pictura::PreferencesDialog* TypePlaceholderTest::openTypePreferences()
{
    window_->registry()->refresh();
    if (!window_->registry()->dispatch(
            QString::fromLatin1(pictura::command_ids::EditPreferencesType))) {
        return nullptr;
    }
    return window_->findChild<pictura::PreferencesDialog*>();
}

void TypePlaceholderTest::dragFillsBox()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_placeholder_box"));
    QVERIFY2(f.ok(), "placeholder fixture");
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(40, 40), QPointF(140, 90), QPointF(260, 150)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QVERIFY2(f.canvas->typeOverlaySelectionForTest() > 0, "the placeholder is selected");
    commit(f);
    const QStringList layers = typeLayers(f);
    QCOMPARE(layers.size(), 1);
    QVERIFY(pictura::type_api::isParagraph(*f.view, layers.first()));
    QCOMPARE(pictura::type_api::layerText(*f.view, layers.first()), kPlaceholder);
}

void TypePlaceholderTest::clickFillsPointType()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_placeholder_click"));
    QVERIFY2(f.ok(), "placeholder fixture");
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(40, 120)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QVERIFY(f.canvas->typeOverlayHasTextForTest());
    QVERIFY2(f.canvas->typeOverlaySelectionForTest() > 0, "the placeholder is selected");
    commit(f);
    const QStringList layers = typeLayers(f);
    QCOMPARE(layers.size(), 1);
    QVERIFY(!pictura::type_api::isParagraph(*f.view, layers.first()));
    QCOMPARE(pictura::type_api::layerText(*f.view, layers.first()), kPlaceholder);
}

void TypePlaceholderTest::typingReplacesPlaceholder()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_placeholder_typing"));
    QVERIFY2(f.ok(), "placeholder fixture");
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(40, 40), QPointF(140, 90), QPointF(260, 150)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    commit(f);
    const QStringList layers = typeLayers(f);
    QCOMPARE(layers.size(), 1);
    QCOMPARE(pictura::type_api::layerText(*f.view, layers.first()), QStringLiteral("Hello"));
}

void TypePlaceholderTest::untouchedPlaceholderCommits()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_placeholder_commit"));
    QVERIFY2(f.ok(), "placeholder fixture");
    const int layers = f.view->layer_count();
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(40, 120)});
    commit(f);
    QCOMPARE(f.view->layer_count(), layers + 1);
    QCOMPARE(typeLayers(f).size(), 1);
    QCOMPARE(pictura::type_api::layerText(*f.view, typeLayers(f).first()), kPlaceholder);
}

void TypePlaceholderTest::maskAndReopenAreExempt()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_placeholder_exempt"));
    QVERIFY2(f.ok(), "placeholder fixture");

    window_->setActiveTool(pictura::ToolId::HorizontalTypeMask);
    f.drag({QPointF(40, 120)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QVERIFY2(!f.canvas->typeOverlayHasTextForTest(), "the mask session starts empty");
    QVERIFY(f.tools->textSession()->cancelText());

    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(40, 120)});
    QTest::keyClicks(f.canvas, QStringLiteral("Mine"));
    commit(f);
    const QString path = typeLayers(f).value(0);
    QVERIFY(!path.isEmpty());
    const int base = f.view->history_index();
    f.drag({QPointF(45, 110)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QCOMPARE(f.canvas->typeOverlaySelectionForTest(), 0);
    commit(f);
    QCOMPARE(pictura::type_api::layerText(*f.view, path), QStringLiteral("Mine"));
    QVERIFY(f.view->history_index() <= base + 1);
}

void TypePlaceholderTest::typePreferencePage()
{
    pictura::PreferencesDialog* prefs = openTypePreferences();
    QVERIFY2(prefs, "Edit > Preferences > Type opens the dialog");
    QCOMPARE(prefs->currentPageForTest(), pictura::PreferencesDialog::kType);
    QVERIFY(prefs->checkboxForTest(QStringLiteral("fillPlaceholderText")));
}

void TypePlaceholderTest::preferenceOffStartsEmpty()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_placeholder_off"));
    QVERIFY2(f.ok(), "placeholder fixture");
    pictura::PreferencesDialog* prefs = openTypePreferences();
    QVERIFY(prefs);
    QVERIFY(prefs->setCheckboxForTest(QStringLiteral("fillPlaceholderText"), false));
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(40, 120)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QVERIFY2(!f.canvas->typeOverlayHasTextForTest(), "the session starts empty");
    QCOMPARE(f.canvas->typeOverlaySelectionForTest(), 0);
    QVERIFY(f.tools->textSession()->cancelText());
}

void TypePlaceholderTest::preferenceIsSaved()
{
    pictura::PreferencesDialog* prefs = openTypePreferences();
    QVERIFY(prefs);
    QVERIFY(prefs->setCheckboxForTest(QStringLiteral("fillPlaceholderText"), false));
    QVERIFY(!pictura::loadSession().typePlaceholderText);
    QVERIFY(prefs->setCheckboxForTest(QStringLiteral("fillPlaceholderText"), true));
    QVERIFY(pictura::loadSession().typePlaceholderText);
}

QTEST_MAIN(TypePlaceholderTest)
#include "tst_type_placeholder.moc"
