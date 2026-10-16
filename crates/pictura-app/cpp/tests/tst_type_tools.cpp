// The Type tools: Horizontal Type (#37), Vertical Type (#38), Horizontal Type
// Mask (#39), and Vertical Type Mask (#40).

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "panels/layers_panel.h"
#include "type_fonts.h"
#include "type_text_edit.h"
#include "panels/numeric_field.h"
#include "selftest_paint_fixture.h"

#include "api/type_api.h"

#include <QtCore/QDataStream>
#include <QtCore/QElapsedTimer>
#include <QtGui/QClipboard>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontInfo>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QRect layerRect(const Fixture& f, const QString& path)
{
    const QStringList parts = pictura::transform_api::layerRect(*f.view, path).toString().split(QLatin1Char(' '));
    if (parts.size() != 4) {
        return QRect();
    }
    return QRect(QPoint(parts[0].toInt(), parts[1].toInt()),
                 QPoint(parts[2].toInt() - 1, parts[3].toInt() - 1));
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

QImage seed()
{
    QImage image(200, 160, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

// A stable digest of the layer-rect pixels, so a cancel can be compared
// bit-for-bit.
quint64 layerPixelHash(const Fixture& f, const QString& path)
{
    const QRect rect = layerRect(f, path);
    quint64 hash = 1469598103934665603ULL;
    for (int y = rect.top(); y <= rect.bottom(); ++y) {
        for (int x = rect.left(); x <= rect.right(); ++x) {
            const quint32 rgba = f.view->sample_argb(x, y);
            for (int byte = 0; byte < 4; ++byte) {
                hash ^= (rgba >> (8 * byte)) & 0xffu;
                hash *= 1099511628211ULL;
            }
        }
    }
    return hash;
}

QByteArray characterFingerprint(const pictura::CharacterSetting& c)
{
    QByteArray bytes;
    QDataStream stream(&bytes, QIODevice::WriteOnly);
    stream << c.font_style << c.size << c.leading_mode << c.leading_value << c.kerning_mode
           << c.kerning_value << c.tracking << c.horizontal_scale << c.vertical_scale
           << c.baseline_shift << c.anti_alias << c.color << c.all_caps << c.small_caps
           << c.superscript << c.subscript << c.underline << c.strikethrough
           << c.fractional_widths;
    return bytes;
}

QByteArray paragraphFingerprint(const pictura::ParagraphSetting& p)
{
    QByteArray bytes;
    QDataStream stream(&bytes, QIODevice::WriteOnly);
    stream << p.justify << p.word_spacing_min << p.word_spacing_desired << p.word_spacing_max
           << p.letter_spacing_min << p.letter_spacing_desired << p.letter_spacing_max
           << p.glyph_spacing_min << p.glyph_spacing_desired << p.glyph_spacing_max
           << p.start_indent << p.end_indent << p.first_line_indent << p.space_before
           << p.space_after << p.hanging << p.hyphenate << p.direction << p.composer;
    return bytes;
}

} // namespace

class TypeToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void horizontalTypeTool();
    void typeKeysAndCancel();
    void reopenTypeLayer();
    void freeTransformKeepsType();
    void freeTransformRotateCursor();
    void textEditModel();
    void editLikeATextField();
    void barRestylesTheSelectedLayer();
    void characterAttributeEdits();
    void manualOverrideSurvivesStyleApply();
    void reopenKeepsAntiAlias();
    void verticalTypeTool();
    void typeMaskTools();
    void typeOptionsBar();
    void typeBarSizeAlignAndColor();
    void caretBlinks();
    void caretPlacementAndBaseline();
    void caretContrastsWithTheImage();
    void typeLayerRowHasNoLockBadge();
    void dragOutsideMovesTheText();

private:
    void showFrame();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypeToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TypeToolsTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

void TypeToolsTest::horizontalTypeTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_seed"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    // T and Shift+T cycle all four Type tools.
    const auto sendKey = [&frame](int key, Qt::KeyboardModifiers mods, const QString& text) {
        QKeyEvent event(QEvent::KeyPress, key, mods, text);
        QApplication::sendEvent(&frame, &event);
    };
    frame.setActiveTool(pictura::ToolId::Move);
    sendKey(Qt::Key_T, Qt::NoModifier, QStringLiteral("t"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::HorizontalType);
    for (pictura::ToolId next : {pictura::ToolId::VerticalType,
                                 pictura::ToolId::HorizontalTypeMask,
                                 pictura::ToolId::VerticalTypeMask,
                                 pictura::ToolId::HorizontalType}) {
        sendKey(Qt::Key_T, Qt::ShiftModifier, QStringLiteral("T"));
        QCOMPARE(frame.activeTool(), next);
    }

    // A click opens a session; typing previews without touching history, and
    // the window's letter shortcuts (B for the Brush) go quiet.
    const int layers = f.view->layer_count();
    const int base = f.view->history_index();
    f.drag({QPointF(20, 60)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QVERIFY(f.canvas->typeOverlayActiveForTest());
    QVERIFY(!f.canvas->typeOverlayHasTextForTest());
    QTest::keyClicks(f.canvas, QStringLiteral("Big"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::HorizontalType);
    QTest::keyClick(f.canvas, Qt::Key_Return);
    QTest::keyClicks(f.canvas, QStringLiteral("type"));
    QVERIFY(f.canvas->typeOverlayHasTextForTest());
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(f.view->layer_count(), layers);

    // Ctrl+Enter commits one type layer, selected, with the text as typed.
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one Horizontal Type state");
    QVERIFY(!f.tools->textSession() || !f.tools->textSession()->textActive());
    QVERIFY(!f.canvas->typeOverlayActiveForTest());
    QCOMPARE(f.view->layer_count(), layers + 1);
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    QCOMPARE(f.view->active_layer_path(), path);
    const QRect rect = layerRect(f, path);
    QVERIFY2(rect.left() == 20 && rect.top() < 60 && rect.bottom() > 60 + 24, "two lines down");
    QVERIFY2(rect.width() > 30 && rect.width() < 80, "as wide as \"type\"");

    // The Layers panel shows it as a type layer: a dark T on a white card, not
    // its pixels.
    auto* panel = frame.findChild<pictura::LayersPanel*>();
    QVERIFY(panel);
    const QImage thumb = panel->rowThumbnailForTest(path);
    QVERIFY2(!thumb.isNull(), "type row thumbnail");
    int white = 0;
    int dark = 0;
    for (int y = 0; y < thumb.height(); ++y) {
        for (int x = 0; x < thumb.width(); ++x) {
            const int v = qGray(thumb.pixel(x, y));
            white += v > 240 ? 1 : 0;
            dark += v < 80 ? 1 : 0;
        }
    }
    const int area = thumb.width() * thumb.height();
    QVERIFY2(white > area / 2 && dark > area / 30, "a T on white");
    const QColor stem = thumb.pixelColor(thumb.width() / 2, thumb.height() / 2);
    QVERIFY2(qGray(stem.rgb()) < 128, "the T's stem at the centre");
}

void TypeToolsTest::typeKeysAndCancel()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_keys"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const int layers = f.view->layer_count();
    const int base = f.view->history_index();

    // Esc drops the text without a state.
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("gone"));
    QTest::keyClick(f.canvas, Qt::Key_Escape);
    QVERIFY(!f.tools->textSession() || !f.tools->textSession()->textActive());
    QCOMPARE(f.view->history_index(), base);

    // Backspace back to nothing commits nothing.
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("ab"));
    QTest::keyClick(f.canvas, Qt::Key_Backspace);
    QTest::keyClick(f.canvas, Qt::Key_Backspace);
    QVERIFY(!f.canvas->typeOverlayHasTextForTest());
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QCOMPARE(f.view->history_index(), base);

    // A click away commits; so does switching tools.
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("one"));
    f.drag({QPointF(150, 150)});
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "click away commits");
    QVERIFY(!f.tools->textSession() || !f.tools->textSession()->textActive());
    f.drag({QPointF(20, 120)});
    QTest::keyClicks(f.canvas, QStringLiteral("two"));
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY2(f.committedOnce(base + 1, "Horizontal Type"), "a tool switch commits");
    QCOMPARE(f.view->layer_count(), layers + 2);
}

void TypeToolsTest::reopenTypeLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_reopen"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    const int index = path.toInt();
    const QRect before = layerRect(f, path);
    const int layers = f.view->layer_count();
    const int base = f.view->history_index();
    // Another tool's bar change leaves the layer alone; coming back to the
    // Type tool shows the selected layer's size.
    frame.setActiveTool(pictura::ToolId::Move);
    o.size = 12.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    QCOMPARE(layerRect(f, path), before);
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().size, 30.0);
    o.size = 12.0;

    // A click on the type reopens it: hidden under its own text, nothing
    // recorded, and the bar shows the layer's size.
    f.drag({QPointF(before.center())});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QVERIFY(!f.view->layer_visible(index));
    QVERIFY(f.canvas->typeOverlayHasTextForTest());
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().size, 30.0);
    QCOMPARE(f.view->history_index(), base);
    QTest::keyClick(f.canvas, Qt::Key_End);
    QTest::keyClicks(f.canvas, QStringLiteral(" there"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");
    QCOMPARE(f.view->layer_count(), layers);
    QVERIFY(f.view->layer_visible(index));
    QCOMPARE(f.view->layer_name(index), QStringLiteral("Hello there"));
    const QRect after = layerRect(f, path);
    QCOMPARE(after.left(), before.left());
    QCOMPARE(after.top(), before.top());
    QVERIFY(after.width() > before.width());

    // Esc leaves it as it was; Backspacing it blank and committing does too.
    f.drag({QPointF(after.center())});
    QTest::keyClicks(f.canvas, QStringLiteral("!!"));
    QTest::keyClick(f.canvas, Qt::Key_Escape);
    QVERIFY(f.view->layer_visible(index));
    QCOMPARE(f.view->history_index(), base + 1);
    f.drag({QPointF(after.center())});
    QTest::keyClick(f.canvas, Qt::Key_A, Qt::ControlModifier);
    QTest::keyClick(f.canvas, Qt::Key_Backspace);
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY(f.view->layer_visible(index));
    QCOMPARE(f.view->history_index(), base + 1);
    QCOMPARE(layerRect(f, path), after);

    // The text keeps its orientation under the Vertical Type tool; the mask
    // tools start new type over it.
    frame.setActiveTool(pictura::ToolId::VerticalType);
    f.drag({QPointF(after.center())});
    QTest::keyClicks(f.canvas, QStringLiteral("!"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base + 1, "Edit Type Layer"), "reopened by Vertical Type");
    QVERIFY(layerRect(f, path).width() > layerRect(f, path).height());
    frame.setActiveTool(pictura::ToolId::HorizontalTypeMask);
    f.drag({QPointF(after.center())});
    QVERIFY(f.view->layer_visible(index));
    QTest::keyClicks(f.canvas, QStringLiteral("M"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base + 2, "Horizontal Type Mask"), "the mask starts new type");
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void TypeToolsTest::freeTransformKeepsType()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_transform"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Wide"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    const QRect before = layerRect(f, path);
    frame.setActiveTool(pictura::ToolId::Move);

    // Drag the right-middle handle out to twice the width and commit.
    QVERIFY(f.view->begin_free_transform(path));
    const double y = before.center().y();
    QVERIFY(f.view->transform_press(before.right() + 1, y, 1.0, false, false) >= 0);
    f.view->transform_move(before.right() + 1 + before.width(), y, 1.0, false, false);
    f.view->transform_release();
    const int base = f.view->history_index();
    QVERIFY(f.view->commit_transform());
    QVERIFY2(f.committedOnce(base, "Free Transform"), "one Free Transform state");

    // Still type, twice as wide, and reopening it keeps the stretch.
    QVERIFY(pictura::type_api::isType(*f.view, path));
    const QRect stretched = layerRect(f, path);
    QVERIFY2(qAbs(stretched.width() - 2 * before.width()) <= 4, "twice as wide");
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(stretched.center())});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QTest::keyClicks(f.canvas, QStringLiteral("r"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base + 1, "Edit Type Layer"), "edited after the transform");
    QVERIFY(layerRect(f, path).width() > stretched.width() + 10);
    QCOMPARE(layerRect(f, path).height(), stretched.height());
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

// Just outside a Free Transform corner the cursor is the curved rotate arrow,
// turned to face that corner; it stays while the rotation drags.
void TypeToolsTest::freeTransformRotateCursor()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_rotate_cursor"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(40, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Spin"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    const QRect box = layerRect(f, path);
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY(f.tools->beginFreeTransform(path));

    const auto cursorImage = [&f]() { return f.canvas->cursor().pixmap().toImage(); };
    const double zoom = f.canvas->zoom();
    const QPointF topLeft = QPointF(box.topLeft()) - QPointF(8, 8) / zoom;
    const QPointF bottomRight = QPointF(box.right() + 1, box.bottom() + 1) + QPointF(8, 8) / zoom;
    f.canvas->mouseMoved(topLeft);
    QCOMPARE(f.canvas->cursor().shape(), Qt::BitmapCursor);
    const QImage atTopLeft = cursorImage();
    QVERIFY(!atTopLeft.isNull());
    f.canvas->mouseMoved(bottomRight);
    QCOMPARE(f.canvas->cursor().shape(), Qt::BitmapCursor);
    // The arc bulges toward the corner, so its ink sits on the box's side of
    // the centred hotspot: down-right at the top-left, up-left at the bottom-right.
    const auto inkOffset = [](const QImage& image) {
        QPointF sum;
        int n = 0;
        for (int y = 0; y < image.height(); ++y) {
            for (int x = 0; x < image.width(); ++x) {
                if (qAlpha(image.pixel(x, y)) > 128) {
                    sum += QPointF(x + 0.5, y + 0.5);
                    ++n;
                }
            }
        }
        return n ? sum / n - QPointF(image.width(), image.height()) / 2.0 : QPointF();
    };
    const QPointF tl = inkOffset(atTopLeft);
    const QPointF br = inkOffset(cursorImage());
    QVERIFY2(tl.x() > 0.3 && tl.y() > 0.3, "top-left arrow faces out of the corner");
    QVERIFY2(br.x() < -0.3 && br.y() < -0.3, "bottom-right arrow faces out of the corner");

    // Inside the box it is the move hand again; with the reference point
    // hidden the box centre is a move press (a shown pivot would grab there).
    f.tools->setTransformShowPivot(false);
    f.canvas->mouseMoved(QPointF(box.center()));
    QCOMPARE(f.canvas->cursor().shape(), Qt::OpenHandCursor);
    f.tools->setTransformShowPivot(true);

    // A rotation drag keeps the arrow, even inside the box, and rotates.
    f.canvas->mousePressed(bottomRight, Qt::LeftButton, 0);
    f.canvas->mouseMoved(QPointF(box.center()) + QPointF(2, box.height()));
    QCOMPARE(f.canvas->cursor().shape(), Qt::BitmapCursor);
    f.canvas->mouseMoved(QPointF(box.center()) + QPointF(1, 1));
    QCOMPARE(f.canvas->cursor().shape(), Qt::BitmapCursor);
    f.canvas->mouseReleased(QPointF(box.center()) + QPointF(1, 1));
    QVERIFY2(qAbs(f.view->transform_angle()) > 0.1, "rotated (radians)");
    f.tools->cancelFreeTransform();
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void TypeToolsTest::textEditModel()
{
    pictura::TypeTextEdit e;
    e.reset(QStringLiteral("hello world\nnext"), 0);
    QCOMPARE(e.text(), QStringLiteral("hello world\rnext"));
    e.step(1, true, false);
    QCOMPARE(e.caret(), 5);
    e.step(1, true, true);
    QCOMPARE(e.selectedText(), QStringLiteral(" world"));
    e.insert(QStringLiteral(" there"));
    QCOMPARE(e.text(), QStringLiteral("hello there\rnext"));
    e.stepLine(1, false);
    QCOMPARE(e.caret(), 16);
    e.home(false, true);
    QCOMPARE(e.selectedText(), QStringLiteral("next"));
    e.step(-1, false, false);
    QCOMPARE(e.caret(), 12);
    e.backspace();
    QCOMPARE(e.text(), QStringLiteral("hello therenext"));
    e.deleteForward();
    QCOMPARE(e.text(), QStringLiteral("hello thereext"));
    e.selectWord(2);
    QCOMPARE(e.selectedText(), QStringLiteral("hello"));
    e.selectWord(5);
    QCOMPARE(e.selectedText(), QStringLiteral(" "));
    e.selectAll();
    e.end(true, false);
    QCOMPARE(e.caret(), int(e.text().size()));
    e.reset(QStringLiteral("a\U0001F600b"), 3);
    QCOMPARE(e.caret(), 3);
    e.step(-1, false, false);
    QCOMPARE(e.caret(), 1);
    e.moveTo(2, false);
    QCOMPARE(e.caret(), 1);
    e.step(1, false, false);
    e.backspace();
    QCOMPARE(e.text(), QStringLiteral("ab"));
}

void TypeToolsTest::editLikeATextField()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_field"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.size = 30.0;
    o.justification = 0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("one two"));
    const QLineF end = f.canvas->typeOverlayCaretForTest();

    // Clicking into the text puts the caret there; typing inserts at it.
    f.drag({QPointF(11, 50)});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QVERIFY(f.canvas->typeOverlayCaretForTest().p1().x() < end.p1().x() - 40);
    QTest::keyClicks(f.canvas, QStringLiteral(">"));
    // Shift+arrows select, which typing replaces; Delete removes forward.
    QTest::keyClick(f.canvas, Qt::Key_End);
    QTest::keyClick(f.canvas, Qt::Key_Left, Qt::ShiftModifier | Qt::ControlModifier);
    QCOMPARE(f.canvas->typeOverlaySelectionForTest(), 3);
    QTest::keyClicks(f.canvas, QStringLiteral("2"));
    QTest::keyClick(f.canvas, Qt::Key_Home);
    QTest::keyClick(f.canvas, Qt::Key_Delete);

    // Ctrl+A, Ctrl+C copies with real newlines; Ctrl+V pastes them back.
    QTest::keyClick(f.canvas, Qt::Key_A, Qt::ControlModifier);
    QTest::keyClick(f.canvas, Qt::Key_C, Qt::ControlModifier);
    QCOMPARE(QGuiApplication::clipboard()->text(), QStringLiteral("one 2"));
    QTest::keyClick(f.canvas, Qt::Key_End);
    QGuiApplication::clipboard()->setText(QStringLiteral("\nthree"));
    QTest::keyClick(f.canvas, Qt::Key_V, Qt::ControlModifier);
    QCOMPARE(frame.activeTool(), pictura::ToolId::HorizontalType);

    // Double-click a word selects it; a drag across the text selects too.
    const QLineF caret = f.canvas->typeOverlayCaretForTest();
    const QPointF onThree(caret.p1().x() - 10, caret.center().y());
    f.drag({onThree});
    f.drag({onThree});
    QCOMPARE(f.canvas->typeOverlaySelectionForTest(), 5);
    QTest::keyClicks(f.canvas, QStringLiteral("3"));
    f.drag({QPointF(11, 50), QPointF(40, 50)});
    QVERIFY(f.canvas->typeOverlaySelectionForTest() >= 2);
    QTest::keyClick(f.canvas, Qt::Key_Right);
    QCOMPARE(f.canvas->typeOverlaySelectionForTest(), 0);

    const int base = f.view->history_index();
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "commit");
    QCOMPARE(QString(pictura::type_api::layerText(*f.view, typeLayer(f))),
             QStringLiteral("one 2\r3"));
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void TypeToolsTest::barRestylesTheSelectedLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_restyle"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.family = QStringLiteral("Liberation Sans");
    o.size = 24.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("iiiiiiii"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    const QRect small = layerRect(f, path);
    const int layers = f.view->layer_count();
    const int base = f.view->history_index();

    // Size: one state, the same text twice as tall.
    o.size = 48.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "size restyles the layer");
    QCOMPARE(f.view->layer_count(), layers);
    const QRect big = layerRect(f, path);
    QVERIFY(qAbs(big.height() - 2 * small.height()) <= 3);
    QCOMPARE(QString(pictura::type_api::layerText(*f.view, path)), QStringLiteral("iiiiiiii"));

    // Family: the layer is re-set in that face — monospace i's are wider —
    // and records it, keeping the size.
    const QString mono = QFontInfo(QFontDatabase::systemFont(QFontDatabase::FixedFont)).family();
    QVERIFY2(!pictura::typeFontBytes(mono).isEmpty(), "Qt hands over the face");
    o.family = mono;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    QVERIFY2(f.committedOnce(base + 1, "Edit Type Layer"), "family restyles the layer");
    const QRect monoRect = layerRect(f, path);
    QVERIFY2(monoRect.width() > big.width() * 3 / 2, "set in the monospace face");
    // Its own ascent and descent, at the same size.
    QVERIFY(monoRect.height() > big.height() * 3 / 4 && monoRect.height() < big.height() * 3 / 2);
    QCOMPARE(QString(pictura::type_api::layerFont(*f.view, path)), mono);

    // Selecting the layer again shows its family and size in the bar.
    frame.setActiveTool(pictura::ToolId::Move);
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().family, mono);
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().size, 48.0);
    QCOMPARE(f.view->history_index(), base + 2);

    // A registered generic alias reads back as itself, not as whichever
    // family its spaceless name happens to match.
    pictura::registerTypeFont(QStringLiteral("monospace"));
    QCOMPARE(pictura::familyForFontName(QStringLiteral("monospace")), QStringLiteral("monospace"));
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void TypeToolsTest::characterAttributeEdits()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_attrs"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Attr"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    const int base = f.view->history_index();
    const QString family = pictura::type_api::layerFont(*f.view, path);
    const QString text = pictura::type_api::layerText(*f.view, path);

    // A non-default character attribute records exactly one state...
    const pictura::TypeSetting placement = pictura::type_api::layerSetting(*f.view, path);
    const pictura::ParagraphSetting paragraph = pictura::type_api::layerParagraphSetting(*f.view, path);
    pictura::CharacterSetting character = pictura::type_api::layerCharacterSetting(*f.view, path);
    QCOMPARE(character.tracking, 0.0);
    QCOMPARE(character.horizontal_scale, 100.0);
    character.tracking = 120.0;
    character.horizontal_scale = 50.0;
    QVERIFY(
        pictura::type_api::updateLayer(*f.view, path, text, family, placement, character, paragraph));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "one Edit Type Layer state");

    // ...and the re-authored layer reopens with it intact (6.4).
    const pictura::CharacterSetting set = pictura::type_api::layerCharacterSetting(*f.view, path);
    QCOMPARE(set.tracking, 120.0);
    QCOMPARE(set.horizontal_scale, 50.0);

    // One undo restores the previous values (6.3).
    QVERIFY(f.view->undo());
    const pictura::CharacterSetting back = pictura::type_api::layerCharacterSetting(*f.view, path);
    QCOMPARE(back.tracking, 0.0);
    QCOMPARE(back.horizontal_scale, 100.0);

    // The Type tool reopens it (hide then Esc): unchanged, no state.
    const int restoredBase = f.view->history_index();
    const quint64 beforePixels = layerPixelHash(f, path);
    const QByteArray beforeCharacter =
        characterFingerprint(pictura::type_api::layerCharacterSetting(*f.view, path));
    const QByteArray beforeParagraph =
        paragraphFingerprint(pictura::type_api::layerParagraphSetting(*f.view, path));
    f.drag({QPointF(layerRect(f, path).center())});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).tracking, 0.0);
    QTest::keyClick(f.canvas, Qt::Key_Escape);
    QCOMPARE(f.view->history_index(), restoredBase);
    QCOMPARE(layerPixelHash(f, path), beforePixels);
    QCOMPARE(characterFingerprint(pictura::type_api::layerCharacterSetting(*f.view, path)),
             beforeCharacter);
    QCOMPARE(paragraphFingerprint(pictura::type_api::layerParagraphSetting(*f.view, path)),
             beforeParagraph);
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

// A manual attribute edit records only the changed field as an override, so a
// later style application keeps it while still updating the fields the user did
// not touch (spec `type-style-model`, "Applying a style keeps an existing
// manual override").
void TypeToolsTest::manualOverrideSurvivesStyleApply()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_override"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.family = QStringLiteral("Liberation Sans");
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Attr"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");

    // Through the real edit path, change tracking only; size stays inherited.
    const QString text = pictura::type_api::layerText(*f.view, path);
    const QString family = pictura::type_api::layerFont(*f.view, path);
    const pictura::TypeSetting placement = pictura::type_api::layerSetting(*f.view, path);
    const pictura::ParagraphSetting paragraph =
        pictura::type_api::layerParagraphSetting(*f.view, path);
    pictura::CharacterSetting character =
        pictura::type_api::layerCharacterSetting(*f.view, path);
    character.tracking = 120.0;
    QVERIFY(pictura::type_api::updateLayer(*f.view, path, text, family, placement, character,
                                       paragraph));

    // A style that sets size, but not the manually changed tracking.
    pictura::CharacterSetting style = pictura::type_api::defaultCharacterSetting();
    style.size = 48.0;
    QVERIFY(pictura::type_api::createCharacterStyle(*f.view, QStringLiteral("Override Probe"),
                                                 family, style));
    QVERIFY(pictura::type_api::applyStyle(*f.view, path, QStringLiteral("Override Probe"), false));

    const pictura::CharacterSetting applied =
        pictura::type_api::layerCharacterSetting(*f.view, path);
    QCOMPARE(applied.tracking, 120.0); // the manual override survives
    QCOMPARE(applied.size, 48.0);      // the untouched field follows the style
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void TypeToolsTest::reopenKeepsAntiAlias()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_antialias"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    pictura::TypeOptions o = saved;
    o.size = 30.0;
    o.antialias = 0; // None, not the default Sharp
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("AA"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).anti_alias, 0);

    // Selecting the layer seeds the bar's combo from the stored method.
    frame.setActiveTool(pictura::ToolId::Move);
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().antialias, 0);

    // Reopen and commit without touching the combo: still None.
    const int base = f.view->history_index();
    f.drag({QPointF(layerRect(f, path).center())});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QTest::keyClick(f.canvas, Qt::Key_End);
    QTest::keyClicks(f.canvas, QStringLiteral("!"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QCOMPARE(f.view->history_index(), base + 1);
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).anti_alias, 0);

    // Esc leaves it as it was.
    f.drag({QPointF(layerRect(f, path).center())});
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    QTest::keyClicks(f.canvas, QStringLiteral("?"));
    QTest::keyClick(f.canvas, Qt::Key_Escape);
    QCOMPARE(f.view->history_index(), base + 1);
    QCOMPARE(pictura::type_api::layerCharacterSetting(*f.view, path).anti_alias, 0);
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void TypeToolsTest::verticalTypeTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_vertical"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::VerticalType);
    const int base = f.view->history_index();
    f.drag({QPointF(100, 10)});
    QTest::keyClicks(f.canvas, QStringLiteral("TALL"));
    QTest::keyClick(f.canvas, Qt::Key_Enter, Qt::KeypadModifier);
    QVERIFY2(f.committedOnce(base, "Vertical Type"), "keypad Enter commits");
    const QRect rect = layerRect(f, typeLayer(f));
    QCOMPARE(rect.top(), 10);
    QVERIFY2(rect.height() > 3 * rect.width(), "one column, top to bottom");
    QVERIFY(rect.left() < 100 && rect.right() > 100);
}

void TypeToolsTest::typeMaskTools()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_mask"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const int layers = f.view->layer_count();
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::HorizontalTypeMask);
    f.drag({QPointF(20, 60)});
    QVERIFY(f.canvas->typeOverlayActiveForTest());
    QTest::keyClicks(f.canvas, QStringLiteral("MASK"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type Mask"), "one mask state");
    QCOMPARE(f.view->layer_count(), layers);
    QVERIFY(f.view->has_selection());
    const int horizontal = f.view->selection_count();
    QVERIFY(horizontal > 50);
    QCOMPARE(f.view->selection_coverage(5, 5), 0);

    // Shift at the click adds the Vertical Type Mask's column to it.
    frame.setActiveTool(pictura::ToolId::VerticalTypeMask);
    f.drag({QPointF(170, 10)}, Qt::ShiftModifier);
    QTest::keyClicks(f.canvas, QStringLiteral("IO"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base + 1, "Vertical Type Mask"), "one vertical mask state");
    QVERIFY(f.view->selection_count() > horizontal);
}

void TypeToolsTest::typeOptionsBar()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_bar"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    frame.setActiveTool(pictura::ToolId::HorizontalType);

    auto visible = [&frame](const QString& name) -> QToolButton* {
        for (auto* button : frame.findChildren<QToolButton*>(name)) {
            if (button->isVisible()) {
                return button;
            }
        }
        return nullptr;
    };
    QToolButton* commit = visible(QStringLiteral("optionsTypeCommit"));
    QVERIFY2(commit, "type bar");
    QVERIFY(!commit->isEnabled());

    // Size and alignment feed the next commit: right-aligned type ends at the click.
    pictura::TypeOptions o = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    o.size = 40.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    visible(QStringLiteral("optionsTypeAlignRight"))->click();
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().justification, 1);
    const int base = f.view->history_index();
    f.drag({QPointF(180, 80)});
    QVERIFY(commit->isEnabled());
    QTest::keyClicks(f.canvas, QStringLiteral("Right"));
    commit->click();
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "the bar's Commit");
    QVERIFY(!commit->isEnabled());
    const QRect rect = layerRect(f, typeLayer(f));
    QVERIFY2(rect.right() <= 181 && rect.right() >= 178, "ends at the click");
    QVERIFY(rect.height() >= 40);

    // Toggle Text Orientation swaps to the Vertical Type tool.
    visible(QStringLiteral("optionsTypeOrientation"))->click();
    QCOMPARE(frame.activeTool(), pictura::ToolId::VerticalType);
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
}

void TypeToolsTest::typeBarSizeAlignAndColor()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_bar_color"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const pictura::TypeOptions saved = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    const QColor foreground = f.tools->foreground();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    auto visible = [&frame](auto* kind, const QString& name) {
        using Widget = std::remove_pointer_t<decltype(kind)>;
        for (auto* widget : frame.findChildren<Widget*>(name)) {
            if (widget->isVisible()) {
                return widget;
            }
        }
        return static_cast<Widget*>(nullptr);
    };

    // Size: a scrolling list of presets that also takes a typed value, applied
    // on Enter rather than per keystroke; an out-of-range entry reverts.
    auto* size = visible(static_cast<QComboBox*>(nullptr), QStringLiteral("optionsTypeSize"));
    QVERIFY2(size && size->isEditable(), "editable size");
    QVERIFY(size->count() > size->maxVisibleItems());
    QVERIFY(size->findText(QStringLiteral("72")) >= 0);
    size->lineEdit()->setText(QStringLiteral("37.5"));
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().size, saved.size);
    emit size->lineEdit()->editingFinished();
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().size, 37.5);
    size->setCurrentIndex(size->findText(QStringLiteral("72")));
    emit size->activated(size->currentIndex());
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().size, 72.0);
    size->lineEdit()->setText(QStringLiteral("0"));
    emit size->lineEdit()->editingFinished();
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().size, 72.0);
    QCOMPARE(size->currentText(), QStringLiteral("72"));

    // Alignment shows drawn line icons, not glyph text.
    for (const char* name : {"optionsTypeAlignLeft", "optionsTypeAlignCenter",
                             "optionsTypeAlignRight"}) {
        QToolButton* button = visible(static_cast<QToolButton*>(nullptr), QLatin1String(name));
        QVERIFY2(button && !button->icon().isNull() && button->text().isEmpty(), name);
    }

    // Colour: the swatch shows the text colour, which follows the foreground,
    // and new type is set in it.
    QToolButton* swatch = visible(static_cast<QToolButton*>(nullptr), QStringLiteral("optionsTypeColor"));
    QVERIFY2(swatch, "colour swatch");
    f.tools->setForeground(QColor(200, 0, 0));
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().color, QColor(200, 0, 0));
    const QImage icon = swatch->icon().pixmap(16, 16).toImage();
    QCOMPARE(QColor(icon.pixel(8, 8)), QColor(200, 0, 0));
    pictura::TypeOptions o = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    o.size = 40.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("HH"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    const QString path = typeLayer(f);
    auto inked = [&f](const QRect& rect, const QColor& want) {
        for (int y = rect.top(); y <= rect.bottom(); ++y) {
            for (int x = rect.left(); x <= rect.right(); ++x) {
                if (QColor::fromRgba(f.view->sample_argb(x, y)) == want) {
                    return true;
                }
            }
        }
        return false;
    };
    QVERIFY2(inked(layerRect(f, path), QColor(200, 0, 0)), "set in the text colour");

    // With the type layer selected, a new colour re-sets it as one state, and
    // selecting it again shows its colour in the swatch.
    const int base = f.view->history_index();
    f.tools->setForeground(QColor(0, 0, 200));
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "colour restyles the layer");
    QVERIFY2(inked(layerRect(f, path), QColor(0, 0, 200)), "re-set in the new colour");
    frame.setActiveTool(pictura::ToolId::Move);
    f.tools->setForeground(foreground);
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    QCOMPARE(f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().color, QColor(0, 0, 200));

    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(saved);
    f.tools->setForeground(foreground);
}

void TypeToolsTest::caretBlinks()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_blink"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);

    // Shown on open, hidden after one 750 ms half-cycle, then back.
    f.drag({QPointF(20, 60)});
    QCOMPARE(f.canvas->typeCaretBlinkIntervalForTest(), 750);
    QElapsedTimer clock;
    clock.start();
    QVERIFY(f.canvas->typeCaretShownForTest());
    QTRY_VERIFY_WITH_TIMEOUT(!f.canvas->typeCaretShownForTest(), 3000);
    QVERIFY2(clock.elapsed() >= 600, "hidden only after the interval");
    QTRY_VERIFY_WITH_TIMEOUT(f.canvas->typeCaretShownForTest(), 3000);

    // A keystroke while hidden shows the caret at once.
    QTRY_VERIFY_WITH_TIMEOUT(!f.canvas->typeCaretShownForTest(), 3000);
    QTest::keyClicks(f.canvas, QStringLiteral("a"));
    QVERIFY(f.canvas->typeCaretShownForTest());
    QTest::keyClick(f.canvas, Qt::Key_Escape);
    QVERIFY(!f.canvas->typeOverlayActiveForTest());
}

void TypeToolsTest::caretPlacementAndBaseline()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_baseline"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);

    // Empty text: the caret and the placement point at the click, no guide.
    f.drag({QPointF(20, 60)});
    QVERIFY(f.canvas->typeOverlayHasPlacementForTest());
    QCOMPARE(f.canvas->typeOverlayPlacementForTest(), QPointF(20, 60));
    QVERIFY(f.canvas->typeOverlayBaselinesForTest().isEmpty());

    // Typed text: one guide from the click along its baseline to the caret.
    QTest::keyClicks(f.canvas, QStringLiteral("some text"));
    const QList<QLineF> guides = f.canvas->typeOverlayBaselinesForTest();
    QCOMPARE(guides.size(), 1);
    const QLineF guide = guides.first();
    QVERIFY2(qAbs(guide.p1().x() - 20.0) <= 1.0 && qAbs(guide.p1().y() - 60.0) <= 1.0,
             "starts at the click");
    QVERIFY2(qAbs(guide.p2().y() - guide.p1().y()) < 0.01, "along the baseline");
    const QLineF caret = f.canvas->typeOverlayCaretForTest();
    QVERIFY2(qAbs(guide.p2().x() - caret.p1().x()) < 0.01, "ends at the caret");
    QVERIFY2(caret.p1().y() < 60.0 && caret.p2().y() > 60.0, "the caret crosses the guide");
    QVERIFY(f.canvas->typeOverlayHasPlacementForTest());
    QTest::keyClick(f.canvas, Qt::Key_Escape);
}

void TypeToolsTest::caretContrastsWithTheImage()
{
    pictura::PicturaMainWindow& frame = *window_;
    for (const bool dark : {true, false}) {
        QImage image = seed();
        image.fill(dark ? Qt::black : Qt::white);
        Fixture f(frame, image, QStringLiteral("pictura_type_contrast"));
        QVERIFY2(f.ok(), "type fixture");
        showFrame();
        frame.setActiveTool(pictura::ToolId::HorizontalType);
        f.drag({QPointF(20, 60)});
        QVERIFY(f.canvas->typeOverlayActiveForTest());
        QCOMPARE(f.canvas->typeOverlayLightInkForTest(), dark);
        QTest::keyClick(f.canvas, Qt::Key_Escape);
    }
}

void TypeToolsTest::typeLayerRowHasNoLockBadge()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_type_lockbadge"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("locks"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    frame.setActiveTool(pictura::ToolId::Move);
    const QString path = typeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");
    QCOMPARE(f.view->layer_lock(path.toInt()) & 0x03, 0x03);

    auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->refresh();
    QVERIFY2(panel->lockBadgeLeftForTest(path) < 0, "forced locks show no badge");

    f.view->set_layers_lock(QStringList{path}, QStringLiteral("position"), true);
    panel->refresh();
    QVERIFY2(panel->lockBadgeLeftForTest(path) >= 0, "a user lock shows the badge");
}

void TypeToolsTest::dragOutsideMovesTheText()
{
    pictura::PicturaMainWindow& frame = *window_;
    QRect unmoved;
    {
        Fixture f(frame, seed(), QStringLiteral("pictura_type_unmoved"));
        QVERIFY2(f.ok(), "type fixture");
        showFrame();
        frame.setActiveTool(pictura::ToolId::HorizontalType);
        f.drag({QPointF(20, 60)});
        QTest::keyClicks(f.canvas, QStringLiteral("Move me"));
        QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
        unmoved = layerRect(f, typeLayer(f));
        QVERIFY(unmoved.isValid());
    }

    Fixture f(frame, seed(), QStringLiteral("pictura_type_moved"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Move me"));

    // Away from the text the pointer would move it; over the text it edits.
    QCOMPARE(f.tools->activeCursorKindForTest(QPointF(140, 120), 0), 1);
    const QLineF caret = f.canvas->typeOverlayCaretForTest();
    QCOMPARE(f.tools->activeCursorKindForTest(caret.center() - QPointF(8, 0), 0), -1);

    // The drag carries the text, its placement point, and its guide along.
    const int base = f.view->history_index();
    const QLineF guide = f.canvas->typeOverlayBaselinesForTest().value(0);
    f.canvas->mousePressed(QPointF(140, 120), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(QPointF(155, 130));
    f.canvas->mouseMoved(QPointF(170, 140));
    QCOMPARE(f.canvas->typeOverlayPlacementForTest(), QPointF(50, 80));
    const QLineF movedGuide = f.canvas->typeOverlayBaselinesForTest().value(0);
    QVERIFY2(QLineF(movedGuide.p1() - guide.p1(), QPointF(30, 20)).length() < 0.01,
             "the guide moved with the text");
    f.canvas->mouseReleased(QPointF(170, 140));
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(),
             "a drag does not commit");
    QCOMPARE(f.view->history_index(), base);

    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    QVERIFY2(f.committedOnce(base, "Horizontal Type"), "one Horizontal Type state");
    const QRect moved = layerRect(f, typeLayer(f));
    QCOMPARE(moved.topLeft() - unmoved.topLeft(), QPoint(30, 20));
    QCOMPARE(moved.size(), unmoved.size());
}

QTEST_MAIN(TypeToolsTest)
#include "tst_type_tools.moc"
