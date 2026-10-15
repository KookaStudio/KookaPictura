// The Glyphs panel (#65): a glyph lands at the open type edit's caret. The
// Character and Paragraph panels are covered by tst_character_paragraph_panels.

#include <QtTest/QtTest>

#include "commands.h"
#include "panels/glyphs_panel.h"
#include "panels/panel_column.h"
#include "selftest_paint_fixture.h"

#include "api/type_api.h"

#include <QtGui/QAction>
#include <QtGui/QFontDatabase>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(160, 120, QImage::Format_RGB32);
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

class TypePanelsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void glyphsPanel();

private:
    void showFrame();
    // Show `panel` through its Window > Panels command.
    bool openPanel(const char* command, const QString& panel);
    template <typename T>
    T* child(const QString& name) const
    {
        return window_->findChild<T*>(name);
    }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypePanelsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TypePanelsTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    QCoreApplication::processEvents();
}

bool TypePanelsTest::openPanel(const char* command, const QString& panel)
{
    QAction* action = window_->registry()->action(QString::fromLatin1(command));
    if (!action || !action->isCheckable()) {
        return false;
    }
    window_->registry()->refresh();
    if (!action->isChecked()) {
        action->trigger();
    }
    QCoreApplication::processEvents();
    pictura::PanelColumn* owner = window_->columnForPanel(panel);
    return owner != nullptr && owner->isPanelVisible(panel);
}

void TypePanelsTest::glyphsPanel()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_glyphs_panel"));
    QVERIFY2(f.ok(), "fixture");
    showFrame();
    QVERIFY(openPanel(pictura::command_ids::WindowPanelsGlyphs, QStringLiteral("glyphsPanel")));
    auto* panel = frame.findChild<pictura::GlyphsPanel*>();
    auto* grid = child<QListWidget>(QStringLiteral("glyphsGrid"));
    auto* subset = child<QComboBox>(QStringLiteral("glyphsSubset"));
    auto* hint = child<QLabel>(QStringLiteral("glyphsHint"));
    QVERIFY(panel && grid && subset && hint);

    // Basic Latin is at most its 95 printable characters; the entire font is more.
    subset->setCurrentIndex(1);
    const int basic = grid->count();
    QVERIFY2(basic > 50 && basic <= 95, "Basic Latin");
    subset->setCurrentIndex(0);
    QVERIFY(grid->count() > basic);
    subset->setCurrentIndex(1);
    const auto items = grid->findItems(QStringLiteral("Z"), Qt::MatchExactly);
    QCOMPARE(items.size(), 1);

    // Without an open type edit there is nowhere for the glyph to go.
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    QVERIFY(!f.tools->textSession() || !f.tools->textSession()->textActive());
    panel->chooseGlyph(items.first());
    QVERIFY(hint->isVisible());

    // With one, it lands at the caret.
    f.drag({QPointF(20, 60)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QTest::keyClicks(f.canvas, QStringLiteral("a"));
    panel->chooseGlyph(items.first());
    QVERIFY(!hint->isVisible());
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    const int base = f.view->history_index();
    QVERIFY(f.tools->textSession() && f.tools->textSession()->commitText());
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "commit");
    QCOMPARE(QString(pictura::type_api::layerText(*f.view, typeLayer(f))), QStringLiteral("aZ"));
}

QTEST_MAIN(TypePanelsTest)
#include "tst_type_panels.moc"
