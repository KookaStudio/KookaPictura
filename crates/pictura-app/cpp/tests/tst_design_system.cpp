#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtGui/QPalette>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

#include "panels/color_swatch_widget.h"
#include "panels/numeric_field.h"
#include "theme.h"

using pictura::Theme;
using pictura::ThemeToken;

namespace {

QColor tokenColor(int level, const QString& key)
{
    for (const ThemeToken& token : Theme::tokens(level)) {
        if (token.key == key) {
            return QColor(token.value);
        }
    }
    return {};
}

QString tokenValue(int level, const QString& key)
{
    for (const ThemeToken& token : Theme::tokens(level)) {
        if (token.key == key) {
            return token.value;
        }
    }
    return {};
}

} // namespace

class DesignSystemTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();

    void tokensCoverColoursAndMetrics();
    void tokensClampTheLevel();
    void stylesheetSubstitutesEveryToken();
    void stylesheetDeclaresPrimitives();
    void primitivesCarryTheirContract();
    void paletteMatchesTokens();
    void swatchPaintsDataColour();
};

void DesignSystemTest::initTestCase()
{
    Theme::apply(Theme::kDefaultLevel);
}

void DesignSystemTest::tokensCoverColoursAndMetrics()
{
    for (int level = 0; level < Theme::kLevelCount; ++level) {
        const QVector<ThemeToken> tokens = Theme::tokens(level);
        QVERIFY(!tokens.isEmpty());
        for (const ThemeToken& token : tokens) {
            QVERIFY2(!token.key.isEmpty(), "every token has a key");
            QVERIFY2(!token.value.isEmpty(), qPrintable(token.key));
            if (token.color) {
                QVERIFY2(QColor(token.value).isValid(), qPrintable(token.value));
            }
        }
        // The derived shades the stylesheet uses are part of the contract.
        for (const char* key : {"menuDisabledText", "separatorLight", "alternateTableBg",
                                "panelHeaderBorder", "scrollbarThumbHover", "hover", "pressed"}) {
            QVERIFY2(tokenColor(level, QLatin1String(key)).isValid(), key);
        }
    }

    // The CS6 drop accent is explicit and constant across levels.
    QCOMPARE(tokenColor(0, QStringLiteral("accent")), QColor(0x2a, 0x7f, 0xff));
    QCOMPARE(tokenColor(3, QStringLiteral("accent")), QColor(0x2a, 0x7f, 0xff));

    // Inline warning / info text is dark-on-yellow at every level.
    for (int level = 0; level < Theme::kLevelCount; ++level) {
        const QColor bg = tokenColor(level, QStringLiteral("noticeBg"));
        const QColor text = tokenColor(level, QStringLiteral("noticeText"));
        QCOMPARE(bg, tokenColor(0, QStringLiteral("noticeBg")));
        QVERIFY(bg.hue() >= 40 && bg.hue() <= 60 && bg.lightness() > 140);
        QVERIFY(text.lightness() < 48);
    }

    // Metrics join the same contract, once.
    QCOMPARE(tokenValue(1, QStringLiteral("panelBorderWidth")),
             QString::number(Theme::kPanelBorderWidth));
    QCOMPARE(tokenValue(1, QStringLiteral("chromeBorderWidth")),
             QString::number(Theme::kChromeBorderWidth));
    QCOMPARE(tokenValue(1, QStringLiteral("groupDividerWidth")),
             QString::number(Theme::kGroupDividerWidth));
    QCOMPARE(tokenValue(1, QStringLiteral("slotBaseW")), QString::number(Theme::kSlotBaseW));
    QCOMPARE(tokenValue(1, QStringLiteral("slotBaseH")), QString::number(Theme::kSlotBaseH));
    QCOMPARE(tokenValue(1, QStringLiteral("slotIconMaxW")), QString::number(Theme::kSlotIconMaxW));
    QCOMPARE(tokenValue(1, QStringLiteral("slotIconMaxH")), QString::number(Theme::kSlotIconMaxH));
    QCOMPARE(tokenValue(1, QStringLiteral("shadeStep")), QString::number(Theme::kShadeStep));
}

void DesignSystemTest::tokensClampTheLevel()
{
    QCOMPARE(Theme::tokens(-1).size(), Theme::tokens(0).size());
    for (const ThemeToken& token : Theme::tokens(0)) {
        QCOMPARE(tokenValue(-1, token.key), token.value);
    }
    for (const ThemeToken& token : Theme::tokens(Theme::kLevelCount - 1)) {
        QCOMPARE(tokenValue(99, token.key), token.value);
    }
}

void DesignSystemTest::stylesheetSubstitutesEveryToken()
{
    for (int level = 0; level < Theme::kLevelCount; ++level) {
        const QString qss = Theme::styleSheet(level);
        QVERIFY2(!qss.contains(QStringLiteral("${")), "no placeholder survives substitution");
        for (const ThemeToken& token : Theme::tokens(level)) {
            if (!token.color) {
                continue;
            }
            QVERIFY2(qss.contains(token.value),
                     qPrintable(QStringLiteral("%1 (%2) is not used").arg(token.key, token.value)));
        }
    }
}

void DesignSystemTest::stylesheetDeclaresPrimitives()
{
    const QString qss = Theme::styleSheet(Theme::kDefaultLevel);
    const char* selectors[] = {
        "QPushButton:hover",
        "QPushButton:pressed",
        "QPushButton:checked",
        "QPushButton:disabled",
        "QPushButton:focus",
        "QToolButton:hover",
        "QToolButton:pressed",
        "QToolButton:checked",
        "QToolButton:disabled",
        "QToolButton:focus",
        "QLineEdit:hover",
        "QLineEdit:focus",
        "QLineEdit:disabled",
        "QComboBox:focus",
        "QComboBox::drop-down",
        "QComboBox QAbstractItemView",
        "QCheckBox::indicator",
        "QCheckBox::indicator:hover",
        "QCheckBox::indicator:checked",
        "QCheckBox::indicator:disabled",
        "QSlider::groove:horizontal",
        "QSlider::handle:horizontal",
        "QSlider::handle:horizontal:hover",
        "QSlider::groove:vertical",
        "QSlider::handle:vertical",
        "QTabBar::tab:hover",
        "QTabBar::tab:selected",
        "QTabBar#panelTabBar::tab",
        "QTabBar#documentTabBar::tab",
        "QMenu::item:selected",
        "QMenu::item:disabled",
        "QMenu::separator",
        "QPushButton#colorSwatch",
        "QWidget#numericField",
        "QWidget#percentField",
        "QWidget#panelHeaderBand",
        "QWidget#panelColumnHeader",
    };
    for (const char* selector : selectors) {
        QVERIFY2(qss.contains(QLatin1String(selector)), selector);
    }
}

void DesignSystemTest::primitivesCarryTheirContract()
{
    QPushButton button(QStringLiteral("OK"));
    QVERIFY(button.objectName().isEmpty());
    button.setCheckable(true);
    button.setChecked(true);
    QVERIFY(button.isChecked());

    QToolButton iconButton;
    iconButton.setAutoRaise(true);
    iconButton.setCheckable(true);
    QVERIFY(iconButton.autoRaise());
    QVERIFY(iconButton.isCheckable());

    QLineEdit edit;
    edit.setDisabled(true);
    QVERIFY(!edit.isEnabled());
    edit.setEnabled(true);
    QVERIFY(edit.focusPolicy() != Qt::NoFocus);

    pictura::NumericFieldConfig config;
    config.objectName = QStringLiteral("numericField");
    pictura::NumericField field(QStringLiteral("Size:"), config);
    QCOMPARE(field.objectName(), QStringLiteral("numericField"));

    QComboBox combo;
    combo.addItems({QStringLiteral("Normal"), QStringLiteral("Dissolve")});
    QCOMPARE(combo.count(), 2);

    QCheckBox box(QStringLiteral("Preview"));
    box.setChecked(true);
    QVERIFY(box.isChecked());
    box.setEnabled(false);
    QVERIFY(!box.isEnabled());

    QSlider slider(Qt::Horizontal);
    slider.setRange(0, 100);
    slider.setValue(40);
    QCOMPARE(slider.value(), 40);

    QTabBar panelTabs;
    panelTabs.setObjectName(QStringLiteral("panelTabBar"));
    panelTabs.addTab(QStringLiteral("Layers"));
    panelTabs.addTab(QStringLiteral("Channels"));
    QCOMPARE(panelTabs.objectName(), QStringLiteral("panelTabBar"));
    QCOMPARE(panelTabs.count(), 2);
    panelTabs.setCurrentIndex(1);
    QCOMPARE(panelTabs.currentIndex(), 1);

    QTabBar documentTabs;
    documentTabs.setObjectName(QStringLiteral("documentTabBar"));
    documentTabs.addTab(QStringLiteral("Untitled-1"));
    QCOMPARE(documentTabs.objectName(), QStringLiteral("documentTabBar"));

    QMenu menu;
    QAction* action = menu.addAction(QStringLiteral("Levels…"));
    QVERIFY(action != nullptr);
    QCOMPARE(menu.actions().size(), 1);

    pictura::ColorSwatchWidget swatch;
    QCOMPARE(swatch.objectName(), QStringLiteral("colorSwatch"));
    swatch.setColor(QColor(10, 20, 30));
    QCOMPARE(swatch.color(), QColor(10, 20, 30));
    swatch.setActive(true);
    QVERIFY(swatch.isActive());
    swatch.setActive(false);
    QVERIFY(!swatch.isActive());

    QWidget header;
    header.setObjectName(QStringLiteral("panelHeaderBand"));
    QCOMPARE(header.objectName(), QStringLiteral("panelHeaderBand"));
}

void DesignSystemTest::paletteMatchesTokens()
{
    const struct {
        QPalette::ColorRole role;
        const char* token;
    } roles[] = {
        {QPalette::Window, "window"},
        {QPalette::WindowText, "windowText"},
        {QPalette::Base, "tableBg"},
        {QPalette::Text, "text"},
        {QPalette::Button, "button"},
        {QPalette::ButtonText, "buttonText"},
        {QPalette::Highlight, "highlight"},
        {QPalette::HighlightedText, "highlightedText"},
        {QPalette::ToolTipBase, "toolTipBase"},
        {QPalette::ToolTipText, "toolTipText"},
        {QPalette::Link, "link"},
        {QPalette::Mid, "buttonBorder"},
        {QPalette::Dark, "border"},
    };
    for (int level = 0; level < Theme::kLevelCount; ++level) {
        Theme::apply(level);
        const QPalette palette = QApplication::palette();
        for (const auto& entry : roles) {
            // The palette carries 16-bit shade precision; the tokens are the
            // 8-bit contract, so compare the quantised values.
            QCOMPARE(palette.color(QPalette::Active, entry.role).rgba(),
                     tokenColor(level, QLatin1String(entry.token)).rgba());
        }
        QCOMPARE(palette.color(QPalette::Disabled, QPalette::Text).rgba(),
                 tokenColor(level, QStringLiteral("disabledText")).rgba());
    }
    Theme::apply(Theme::kDefaultLevel);
}

void DesignSystemTest::swatchPaintsDataColour()
{
    pictura::ColorSwatchWidget swatch;
    swatch.resize(40, 24);

    QSignalSpy spy(&swatch, &QPushButton::clicked);
    QVERIFY(spy.isValid());
    QTest::mouseClick(&swatch, Qt::LeftButton, Qt::NoModifier, QPoint(20, 12));
    QCOMPARE(spy.count(), 1);

    swatch.setColor(QColor(255, 0, 0));
    QCOMPARE(swatch.property("color").value<QColor>(), QColor(255, 0, 0));
    const QImage painted = swatch.grab().toImage();
    QCOMPARE(painted.pixelColor(20, 12), QColor(255, 0, 0));

    // An invalid colour is the empty state: the data fill is gone.
    swatch.setColor(QColor());
    QVERIFY(!swatch.color().isValid());
    const QImage empty = swatch.grab().toImage();
    QVERIFY2(empty.pixelColor(20, 12) != QColor(255, 0, 0), "the fill cleared");
}

QTEST_MAIN(DesignSystemTest)
#include "tst_design_system.moc"
