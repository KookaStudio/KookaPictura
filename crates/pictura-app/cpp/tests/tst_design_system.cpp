#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QCoreApplication>
#include <QtGui/QImage>
#include <QtGui/QPalette>
#include <QtWidgets/QApplication>

#include <array>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

#include "panels/color_swatch_widget.h"
#include "panels/curve_widget.h"
#include "panels/histogram_panel.h"
#include "panels/navigator_panel.h"
#include "panels/numeric_field.h"
#include "panels/spectrum_bar.h"
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

// Tokens consumed only by custom-painted widgets, which the global stylesheet
// never references; the single-source rule below covers stylesheet-referenced
// tokens, not the whole paint-only set.
bool isPaintOnlyToken(const QString& key)
{
    return key == QLatin1String("graphFill") || key == QLatin1String("graphAxis");
}

bool imageHasColor(const QImage& image, const QColor& color)
{
    if (!color.isValid()) {
        return false;
    }
    const QRgb target = color.rgb() & 0x00ffffffu;
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            if ((image.pixel(x, y) & 0x00ffffffu) == target) {
                return true;
            }
        }
    }
    return false;
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
    void paintedWidgetsUseTokens();
    void lightFamilyTokensAreConsistent();
    void numericFieldArrowIsInset();
    void inputsShareOneHeight();
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
            if (!token.color || isPaintOnlyToken(token.key)) {
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
        "QComboBox::down-arrow",
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
        "QLineEdit QToolButton",
        "QComboBox QLineEdit",
        "QToolButton#workspaceSwitcher",
        "QWidget#percentField",
        "QWidget#panelHeaderBand",
        "QWidget#panelColumnHeader",
        "QGroupBox::title",
        "QRadioButton::indicator",
        "QRadioButton::indicator:checked",
        "QProgressBar::chunk",
        "QHeaderView::section",
        "QSplitter::handle",
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

    QGroupBox group(QStringLiteral("Units"));
    group.setCheckable(true);
    group.setChecked(true);
    QVERIFY(group.isChecked());
    group.setEnabled(false);
    QVERIFY(!group.isEnabled());

    QRadioButton radio(QStringLiteral("PostScript"));
    radio.setChecked(true);
    QVERIFY(radio.isChecked());
    radio.setEnabled(false);
    QVERIFY(!radio.isEnabled());

    QProgressBar progress;
    progress.setRange(0, 100);
    progress.setValue(40);
    QCOMPARE(progress.value(), 40);
    progress.setEnabled(false);
    QVERIFY(!progress.isEnabled());

    QHeaderView tableHeader(Qt::Horizontal);
    QCOMPARE(tableHeader.orientation(), Qt::Horizontal);
    QCOMPARE(tableHeader.count(), 0);

    QSplitter splitter(Qt::Horizontal);
    splitter.addWidget(new QWidget);
    splitter.addWidget(new QWidget);
    QCOMPARE(splitter.count(), 2);
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

void DesignSystemTest::paintedWidgetsUseTokens()
{
    Theme::apply(Theme::kDefaultLevel);
    const QColor inputBg = Theme::tokenColor(QStringLiteral("inputBg"));
    const QColor inputBorder = Theme::tokenColor(QStringLiteral("inputBorder"));
    const QColor graphFill = Theme::tokenColor(QStringLiteral("graphFill"));
    QVERIFY(inputBg.isValid());
    QVERIFY(inputBorder.isValid());
    QVERIFY(graphFill.isValid());

    pictura::CurveWidget curve;
    curve.resize(200, 120);
    std::array<int, 256> bins{};
    for (int i = 0; i < 128; ++i) {
        bins[static_cast<std::size_t>(i)] = 100;
    }
    curve.setBins(bins);
    const QImage curveImage = curve.grab().toImage();
    QVERIFY2(imageHasColor(curveImage, inputBg), "curve plot background is the inputBg token");
    QVERIFY2(imageHasColor(curveImage, graphFill), "curve histogram uses the graphFill token");

    pictura::SpectrumBar spectrum;
    spectrum.resize(120, 16);
    QVERIFY2(imageHasColor(spectrum.grab().toImage(), inputBorder),
             "spectrum border is the inputBorder token");

    pictura::HistogramView histogram;
    histogram.resize(200, 120);
    QVERIFY2(imageHasColor(histogram.grab().toImage(), inputBg),
             "histogram background is the inputBg token");

    pictura::NavigatorThumbnail navigator;
    navigator.resize(160, 120);
    QVERIFY2(imageHasColor(navigator.grab().toImage(), inputBg),
             "navigator thumbnail background is the inputBg token");
}

void DesignSystemTest::lightFamilyTokensAreConsistent()
{
    const auto tokenOf = [](const QVector<ThemeToken>& tokens, const QString& key) {
        for (const ThemeToken& token : tokens) {
            if (token.key == key) {
                return QColor(token.value);
            }
        }
        return QColor();
    };
    for (int level = 0; level < Theme::kLevelCount; ++level) {
        const QVector<ThemeToken> tokens = Theme::tokens(Theme::ThemeFamily::Light, level);
        QVERIFY2(!tokens.isEmpty(), "the light family exposes its tokens");
        // A light surface must be lighter than its text/icon roles.
        QVERIFY2(tokenOf(tokens, QStringLiteral("panel")).lightness()
                     > tokenOf(tokens, QStringLiteral("text")).lightness(),
                 "the light panel is lighter than its text");
        // The derived alternate row must not clamp back onto the base surface.
        QVERIFY2(tokenOf(tokens, QStringLiteral("alternateTableBg"))
                     != tokenOf(tokens, QStringLiteral("tableBg")),
                 "the light alternate row stays distinct");
    }
    QCOMPARE(Theme::familyName(Theme::ThemeFamily::Light), QStringLiteral("light"));
    QCOMPARE(Theme::familyFromName(QStringLiteral("light")), Theme::ThemeFamily::Light);
    QCOMPARE(Theme::familyFromName(QStringLiteral("nonsense")), Theme::ThemeFamily::Dark);

    // The baked select chevron (QSS cannot tint) is chosen per family.
    QVERIFY2(Theme::styleSheet(Theme::ThemeFamily::Dark, 1)
                 .contains(QStringLiteral(":/ui/select-chevron.svg")),
             "the dark family uses the light-coloured chevron asset");
    QVERIFY2(Theme::styleSheet(Theme::ThemeFamily::Light, 1)
                 .contains(QStringLiteral(":/ui/select-chevron-light.svg")),
             "the light family uses the dark-coloured chevron asset");
}

// The popup slider's arrow is an inset child of the value box (not a sibling
// outside its frame), carries the Lucide chevron, and darkens while open.
void DesignSystemTest::numericFieldArrowIsInset()
{
    pictura::NumericFieldConfig config;
    config.minimum = 0.0;
    config.maximum = 100.0;
    config.suffix = QStringLiteral("%");
    config.popup = true;
    config.namePrefix = QStringLiteral("arrowTest");
    pictura::NumericField field(QStringLiteral("Opacity:"), config);
    field.setValue(50);
    field.show();
    QVERIFY(QTest::qWaitForWindowExposed(&field));

    auto* edit = field.findChild<QLineEdit*>(QStringLiteral("arrowTestEdit"));
    auto* arrow = field.findChild<QToolButton*>(QStringLiteral("arrowTestArrow"));
    QVERIFY(edit != nullptr);
    QVERIFY(arrow != nullptr);
    QCOMPARE(arrow->parentWidget(), static_cast<QWidget*>(edit));
    QVERIFY2(edit->rect().contains(arrow->geometry()), "the arrow sits inside the input");
    QVERIFY2(edit->textMargins().right() >= arrow->width(),
             "the value keeps clear of the arrow");
    QCOMPARE(arrow->arrowType(), Qt::NoArrow);
    QVERIFY2(!arrow->icon().isNull(), "the arrow carries the chevron icon");

    QVERIFY(!arrow->isChecked());
    arrow->click();
    QCoreApplication::processEvents();
    QVERIFY2(arrow->isChecked(), "the arrow is active while the slider popup is open");
    auto* popup = field.findChild<QWidget*>(QStringLiteral("arrowTestPopup"));
    QVERIFY(popup != nullptr);
    popup->hide();
    QCoreApplication::processEvents();
    QVERIFY2(!arrow->isChecked(), "the arrow rests when the slider closes");

    field.hide();
}

// The input family renders at one height: the spin box's button floor is carried
// to the line edit and the select. The size hint follows the platform font
// metrics, which can differ by a pixel between Qt builds, so assert that no
// member is visibly thinner rather than a pixel-exact match.
void DesignSystemTest::inputsShareOneHeight()
{
    QLineEdit edit;
    QComboBox combo;
    combo.addItem(QStringLiteral("A"));
    QDoubleSpinBox spin;
    for (QWidget* widget : {static_cast<QWidget*>(&edit), static_cast<QWidget*>(&combo),
                            static_cast<QWidget*>(&spin)}) {
        widget->ensurePolished();
    }
    const int lineEdit = edit.sizeHint().height();
    const int comboBox = combo.sizeHint().height();
    const int spinBox = spin.sizeHint().height();
    const int tallest = qMax(lineEdit, qMax(comboBox, spinBox));
    const int shortest = qMin(lineEdit, qMin(comboBox, spinBox));
    QVERIFY2(tallest - shortest <= 1,
             qPrintable(QStringLiteral("input heights %1/%2/%3")
                            .arg(lineEdit)
                            .arg(comboBox)
                            .arg(spinBox)));
}

QTEST_MAIN(DesignSystemTest)
#include "tst_design_system.moc"
