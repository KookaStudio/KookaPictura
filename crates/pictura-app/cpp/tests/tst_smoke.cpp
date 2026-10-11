#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "panels/numeric_field.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "session.h"
#include "theme.h"
#include "tool_hint_bar.h"

#include <QtCore/QFile>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

class SmokeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void frameOpensOffscreen();
    void stateHomeIsolationRestoresPriorValue();
    void themeFoundation();
    void statusBarChromeAndZoom();
    void statusZoomKeysAndSuffix();
    void numericFieldAlignment();
    void sessionWindowGeometryRoundTrips();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SmokeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SmokeTest::frameOpensOffscreen()
{
    QCOMPARE(qApp->platformName(), QStringLiteral("offscreen"));
    QVERIFY(window_->newDocument(QStringLiteral("Untitled"), 512, 512, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QVERIFY(window_->activeView() != nullptr);

    // An open canvas paints the empty-workspace shade, never black, and follows
    // the applied brightness level.
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    const QColor level1 = canvas->canvasColor();
    QCOMPARE(level1, pictura::Theme::workspaceColor());
    QVERIFY2(level1 != QColor(Qt::black), "workspace canvas is not black");
    window_->setBrightnessLevel(2);
    QCOMPARE(window_->brightnessLevel(), 2);
    QCOMPARE(canvas->canvasColor(), pictura::Theme::workspaceColor());
    QVERIFY2(canvas->canvasColor() != level1, "canvas follows the brightness level");
    window_->setBrightnessLevel(1);
    QCOMPARE(canvas->canvasColor(), pictura::Theme::workspaceColor());
}

void SmokeTest::stateHomeIsolationRestoresPriorValue()
{
    qputenv("XDG_STATE_HOME", QByteArray("/prior-sentinel"));
    {
        pictura::test::ScopedStateHome guard;
        QVERIFY(guard.isValid());
        QVERIFY(qgetenv("XDG_STATE_HOME") != QByteArray("/prior-sentinel"));
    }
    QCOMPARE(qgetenv("XDG_STATE_HOME"), QByteArray("/prior-sentinel"));

    qunsetenv("XDG_STATE_HOME");
    {
        pictura::test::ScopedStateHome guard;
        QVERIFY(guard.isValid());
        QVERIFY(qEnvironmentVariableIsSet("XDG_STATE_HOME"));
    }
    QVERIFY(!qEnvironmentVariableIsSet("XDG_STATE_HOME"));
}

void SmokeTest::themeFoundation()
{
    using pictura::Theme;
    QCOMPARE(Theme::kPanelBorderWidth, 1);
    QCOMPARE(Theme::kChromeBorderWidth, 2);
    QCOMPARE(Theme::kSlotBaseW, 36);
    QCOMPARE(Theme::kSlotBaseH, 28);
    QCOMPARE(Theme::kSlotIconMaxW, 24);
    QCOMPARE(Theme::kSlotIconMaxH, 20);
    QCOMPARE(Theme::kShadeStep, 112);

    const QColor window = qApp->palette().color(QPalette::Window);
    const QString windowHex = window.name(QColor::HexRgb);
    QCOMPARE(windowHex, QStringLiteral("#363636"));
    const QString panelHex = QStringLiteral("#4d4d4d");
    const QString hoverHex = QStringLiteral("#565656");
    const QString pressedHex = QStringLiteral("#2e2e2e");
    const QString outlineHex = QStringLiteral("#595959");
    QVERIFY2(panelHex != windowHex, "panel is distinct from the window");
    QVERIFY2(pressedHex != hoverHex, "pressed is distinct from hover");
    QCOMPARE(Theme::workspaceColor().name(QColor::HexRgb), QStringLiteral("#1f1f1f"));

    const QString sheet = qApp->styleSheet();
    QVERIFY2(sheet.contains(QStringLiteral("QToolButton { background: transparent;")),
             "idle-less button base rule");
    QVERIFY2(sheet.contains(QStringLiteral("QToolButton:hover { background: ") + hoverHex
                                + QStringLiteral("; border-color: ") + outlineHex),
             "hover uses the dedicated icon tokens");
    QVERIFY2(sheet.contains(QStringLiteral("QToolButton:pressed { background: ") + pressedHex
                                + QStringLiteral("; border-color: ") + outlineHex),
             "pressed is one step darker with the same outline");
}

void SmokeTest::statusBarChromeAndZoom()
{
    window_->resize(1100, 700);
    window_->show();
    QCoreApplication::processEvents();
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }

    QStatusBar* bar = window_->statusBar();
    QVERIFY(bar != nullptr);
    QVERIFY2(bar->height() <= 28, "status bar trimmed to ~28px");
    auto* zoom = window_->findChild<pictura::NumericField*>(QStringLiteral("statusZoomField"));
    auto* size = window_->findChild<QLabel*>(QStringLiteral("statusSizeLabel"));
    auto* backend = window_->findChild<QLabel*>(QStringLiteral("statusBackendLabel"));
    auto* options = window_->findChild<QToolButton*>(QStringLiteral("statusOptionsButton"));
    auto* hint = window_->findChild<pictura::ToolHintBar*>(QStringLiteral("toolHintBar"));
    QVERIFY(zoom && size && backend && options && hint);

    QVERIFY2(!zoom->isVisible(), "no zoom readout without a document");
    QVERIFY2(!size->isVisible(), "no size readout without a document");
    QVERIFY2(!backend->isVisible(), "no backend readout without a document");
    QVERIFY2(!hint->isVisible(), "no tool hint without a document");
    QVERIFY2(!options->isVisible(), "no options button without a document");
    QVERIFY2(!bar->isSizeGripEnabled(), "no corner size grip");

    QVERIFY(window_->newDocument(QStringLiteral("Footer"), 64, 48, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QVERIFY2(zoom->isVisible() && size->isVisible() && backend->isVisible(), "readouts restored");
    QVERIFY2(hint->isVisible() && options->isVisible(), "footer restored");

    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    canvas->setZoom(2.0, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    QCoreApplication::processEvents();
    // The document tab tracks the canvas zoom live.
    const int tabIndex = window_->activeDocumentIndex();
    QVERIFY(tabIndex >= 0);
    QVERIFY2(window_->documentTabTextForTest(tabIndex).contains(QStringLiteral("@ 200%")),
             "tab title tracks the zoom percent");
    auto* edit = zoom->findChild<QLineEdit*>(QStringLiteral("statusZoomEdit"));
    auto* suffix = zoom->findChild<QLabel*>(QStringLiteral("statusZoomSuffix"));
    QVERIFY(edit && suffix);
    // Same value and format as the Navigator readout: zoom * 100, no decimals, "%".
    const QString expected =
        QString::number(canvas->zoom() * 100.0, 'f', 0) + QStringLiteral("%");
    QCOMPARE(edit->text() + suffix->text(), expected);
    QVERIFY2(expected.endsWith(QStringLiteral("%")), "percent format");

    // A fractional zoom renders rounded, so it fits the value box and agrees
    // with the tab title instead of clipping a raw "52.3438".
    canvas->setZoom(0.5234, QPointF(canvas->width() / 2.0, canvas->height() / 2.0));
    QCoreApplication::processEvents();
    QCOMPARE(edit->text() + suffix->text(), QStringLiteral("52%"));
    QVERIFY2(window_->documentTabTextForTest(tabIndex).contains(QStringLiteral("@ 52%")),
             "tab title tracks the fractional zoom percent");
    QVERIFY2(edit->fontMetrics().horizontalAdvance(edit->text())
                     + edit->textMargins().right() + 4 <= edit->width(),
             "the zoom value fits its box");

    hint->setHints(pictura::toolHintEntries(pictura::ToolId::Move), QString());
    QCOMPARE(hint->keyForTest(0), QStringLiteral("Arrows"));
    QCOMPARE(hint->chevronCountForTest(0), 4);
    QCOMPARE(hint->keyForTest(1), QStringLiteral("Shift+Arrows"));
    QCOMPARE(hint->chevronCountForTest(1), 4);
    QCOMPARE(hint->chevronDirectionsForTest(), QStringLiteral("ULDR"));
    hint->setPressedKey(QStringLiteral("Arrows"));
    QVERIFY2(hint->highlightedForTest(0), "arrows keycap highlights while held");
    hint->setPressedKey(QStringLiteral("Shift"));
    QVERIFY2(hint->highlightedForTest(1), "shift+arrows keycap highlights while shift held");
    hint->setPressedKey(QString());
    QVERIFY(!hint->highlightedForTest(0));
    QVERIFY(!hint->highlightedForTest(1));
}

// The footer zoom field steps with Up/Down, hugs its "%" to the value, and
// shows the widest zoom without scrolling its start out of view (#318).
void SmokeTest::statusZoomKeysAndSuffix()
{
    window_->resize(1100, 700);
    window_->show();
    QVERIFY(window_->newDocument(QStringLiteral("ZoomKeys"), 64, 48, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::ImageView* canvas = window_->imageView();
    auto* zoom = window_->findChild<pictura::NumericField*>(QStringLiteral("statusZoomField"));
    QVERIFY(canvas && zoom);
    auto* edit = zoom->findChild<QLineEdit*>(QStringLiteral("statusZoomEdit"));
    auto* suffix = zoom->findChild<QLabel*>(QStringLiteral("statusZoomSuffix"));
    QVERIFY(edit && suffix);
    const QPointF centre(canvas->width() / 2.0, canvas->height() / 2.0);
    canvas->setZoom(0.25, centre);
    QCoreApplication::processEvents();
    QCOMPARE(edit->text(), QStringLiteral("25"));

    QTest::keyClick(edit, Qt::Key_Up);
    QCOMPARE(qRound(canvas->zoom() * 100.0), 26);
    QTest::keyClick(edit, Qt::Key_Down);
    QTest::keyClick(edit, Qt::Key_Down);
    QCOMPARE(qRound(canvas->zoom() * 100.0), 24);
    QTest::keyClick(edit, Qt::Key_Up, Qt::ShiftModifier);
    QCOMPARE(qRound(canvas->zoom() * 100.0), 34);
    QCOMPARE(edit->text(), QStringLiteral("34"));

    // The cursor at the end of the text marks where the value ends.
    const auto textEnd = [edit]() {
        edit->end(false);
        return edit->inputMethodQuery(Qt::ImCursorRectangle).toRect().center().x();
    };
    QVERIFY2(qAbs(suffix->x() - textEnd()) <= 2,
             qPrintable(QStringLiteral("suffix at %1, value ends at %2")
                            .arg(suffix->x())
                            .arg(textEnd())));

    canvas->setZoom(32.0, centre);
    QCoreApplication::processEvents();
    QCOMPARE(edit->text(), QStringLiteral("3200"));
    const int start = textEnd() - edit->fontMetrics().horizontalAdvance(edit->text());
    QVERIFY2(start >= 2, qPrintable(QStringLiteral("value starts at %1").arg(start)));
    QVERIFY2(suffix->geometry().right() < edit->width(), "the suffix stays inside the box");
}

void SmokeTest::numericFieldAlignment()
{
    window_->resize(1100, 700);
    window_->show();
    QVERIFY(window_->newDocument(QStringLiteral("Align"), 64, 48, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    auto* layersEdit = window_->findChild<QLineEdit*>(QStringLiteral("percentEdit"));
    QVERIFY(layersEdit != nullptr);
    QVERIFY2(layersEdit->alignment().testFlag(Qt::AlignLeft), "layers field is left-aligned");
    QVERIFY2(!layersEdit->alignment().testFlag(Qt::AlignRight), "layers field is not right-aligned");

    window_->setActiveTool(pictura::ToolId::Blur);
    QCoreApplication::processEvents();
    auto* strength =
        window_->findChild<pictura::NumericField*>(QStringLiteral("optionsBlurStrength"));
    QVERIFY(strength != nullptr);
    auto* optionsEdit = strength->findChild<QLineEdit*>(QStringLiteral("numericEdit"));
    QVERIFY(optionsEdit != nullptr);
    QVERIFY2(optionsEdit->alignment().testFlag(Qt::AlignLeft), "options field is left-aligned");
    QVERIFY2(!optionsEdit->alignment().testFlag(Qt::AlignRight),
             "options field is not right-aligned");
}

void SmokeTest::sessionWindowGeometryRoundTrips()
{
    // Encode/decode: arbitrary binary geometry survives base64 through the store.
    pictura::SessionState state;
    state.windowGeometry = QByteArray::fromHex("0102abff00fe");
    state.windowMaximized = true;
    QVERIFY(pictura::saveSession(state));
    const pictura::SessionState loaded = pictura::loadSession();
    QCOMPARE(loaded.windowGeometry, state.windowGeometry);
    QVERIFY(loaded.windowMaximized);

    // Real window geometry round-trips.
    window_->resize(1013, 657);
    window_->move(37, 41);
    window_->show();
    QCoreApplication::processEvents();
    window_->saveSession();
    const pictura::SessionState geometry = pictura::loadSession();
    QVERIFY(!geometry.windowGeometry.isEmpty());
    QVERIFY(geometry.windowGeometry != state.windowGeometry);

    // An older store without the fields decodes to the safe defaults.
    {
        QFile file(pictura::sessionFilePath());
        QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
        file.write(QByteArrayLiteral("{\"schemaVersion\":9}"));
    }
    const pictura::SessionState legacy = pictura::loadSession();
    QVERIFY(legacy.windowGeometry.isEmpty());
    QVERIFY(!legacy.windowMaximized);
}

QTEST_MAIN(SmokeTest)
#include "tst_smoke.moc"
