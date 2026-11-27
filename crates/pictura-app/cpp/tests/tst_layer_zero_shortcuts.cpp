#include <QtTest/QtTest>

#include "api/layers_api.h"
#include "api/paint_bridge.h"
#include "command_ids.h"
#include "commands.h"
#include "api/view_bridge.h"
#include "frame.h"
#include "image_view.h"
#include "panels/layers_panel.h"
#include "qt_test_support.h"

#include <QtCore/QThread>
#include <QtGui/QKeyEvent>
#include <QtGui/QShortcut>
#include <QtTest/QSignalSpy>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QTreeView>

#include <memory>

namespace {

void pressPair(QWidget* target, Qt::Key key = Qt::Key_0,
               Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QTest::keyClick(target, key, modifiers);
    QTest::keyClick(target, key, modifiers);
}

void waitOutShortcut()
{
    // No signal marks the end of a canceled shortcut's 500 ms window.
    QTest::qWait(550);
}

int percentByte(int percent)
{
    return qRound(percent * 255.0 / 100.0);
}

} // namespace

class LayerZeroShortcutsTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void canvasOpacity();
    void treeFill();
    void singleDigits();
    void exactPairs();
    void registeredNumericShortcut();
    void paintingCanvas();
    void eligibilityAndNoOp();
    void interruptedSequence();
    void contextChanges();
    void modifiedKeys();
    void pendingTransform();
    void editorInput();

private:
    void select(const QStringList& paths);
    pictura::layers_api::LayerRow row(const QString& path) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
    QTreeView* tree_ = nullptr;
    pictura::ImageView* canvas_ = nullptr;
    QString layer_;
};

void LayerZeroShortcutsTest::init()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
    QVERIFY(window_->newDocument(QStringLiteral("Numeric shortcuts"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel_);
    tree_ = panel_->findChild<QTreeView*>();
    canvas_ = window_->imageView();
    QVERIFY(tree_ && canvas_);
    layer_ = view_->add_layer_in(QString());
    QVERIFY(!layer_.isEmpty());
    select({layer_});
    window_->setActiveTool(pictura::ToolId::Move);
    window_->show();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void LayerZeroShortcutsTest::cleanup()
{
    if (window_) {
        window_->hide();
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    window_.reset();
}

void LayerZeroShortcutsTest::select(const QStringList& paths)
{
    panel_->refresh();
    panel_->selectPaths(paths, paths.first());
}

pictura::layers_api::LayerRow LayerZeroShortcutsTest::row(const QString& path) const
{
    return pictura::layers_api::rowForPath(*view_, path);
}

void LayerZeroShortcutsTest::canvasOpacity()
{
    const QString other = view_->add_layer_in(QString());
    QVERIFY(!other.isEmpty());
    QCOMPARE(view_->set_layers_opacity({layer_}, 100), 1);
    QCOMPARE(view_->set_layers_opacity({other}, 150), 1);
    QCOMPARE(view_->set_layers_fill({layer_}, 180), 1);
    select({layer_, other});
    canvas_->setFocus();
    const int history = view_->history_count();

    pressPair(canvas_, Qt::Key_0, Qt::KeypadModifier);

    QCOMPARE(row(layer_).opacity, 0);
    QCOMPARE(row(other).opacity, 0);
    QCOMPARE(row(layer_).fill, 180);
    QCOMPARE(row(other).fill, 255);
    QCOMPARE(view_->history_count(), history + 1);
    QTRY_COMPARE(panel_->opacityPercentForTest(), 0);
    QVERIFY(view_->undo());
    QCOMPARE(row(layer_).opacity, 100);
    QCOMPARE(row(other).opacity, 150);
}

void LayerZeroShortcutsTest::treeFill()
{
    QCOMPARE(view_->set_layers_opacity({layer_}, 180), 1);
    select({layer_});
    tree_->setFocus();
    const int history = view_->history_count();

    pressPair(tree_, Qt::Key_0, Qt::ShiftModifier);

    QCOMPARE(row(layer_).fill, 0);
    QCOMPARE(row(layer_).opacity, 180);
    QCOMPARE(view_->history_count(), history + 1);
    QTRY_COMPARE(panel_->fillPercentForTest(), 0);
    QVERIFY(view_->undo());
    QCOMPARE(row(layer_).fill, 255);
}

void LayerZeroShortcutsTest::eligibilityAndNoOp()
{
    const QString background = QStringLiteral("0");
    if (row(background).kind != QStringLiteral("background")) {
        QVERIFY(view_->background_from_layer(background));
    }
    QCOMPARE(row(background).kind, QStringLiteral("background"));
    QVERIFY(view_->set_layers_lock({layer_}, QStringLiteral("all"), true));
    const QString group = view_->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    tree_->setFocus();
    const int history = view_->history_count();
    for (const QString& path : {background, layer_}) {
        select({path});
        pressPair(tree_);
        pressPair(tree_, Qt::Key_0, Qt::ShiftModifier);
        QCOMPARE(row(path).opacity, 255);
        QCOMPARE(row(path).fill, 255);
    }

    select({group});
    pressPair(tree_, Qt::Key_0, Qt::ShiftModifier);
    QCOMPARE(row(group).fill, 255);
    QCOMPARE(view_->history_count(), history);
    pressPair(tree_);
    QCOMPARE(row(group).opacity, 0);
    pressPair(tree_);
    QCOMPARE(view_->history_count(), history + 1);
}

void LayerZeroShortcutsTest::singleDigits()
{
    QCOMPARE(view_->set_layers_opacity({layer_}, 128), 1);
    QCOMPARE(view_->set_layers_fill({layer_}, 128), 1);
    select({layer_});
    for (const bool fill : {false, true}) {
        QWidget* source = fill ? static_cast<QWidget*>(tree_)
                               : static_cast<QWidget*>(canvas_);
        source->setFocus();
        QVERIFY(source->hasFocus());
        const auto value = [&] { return fill ? row(layer_).fill : row(layer_).opacity; };
        const auto other = [&] { return fill ? row(layer_).opacity : row(layer_).fill; };
        const int otherBefore = other();
        for (int digit = 0; digit <= 9; ++digit) {
            const int before = value();
            const int history = view_->history_count();
            QTest::keyClick(source, Qt::Key(Qt::Key_0 + digit),
                            fill ? Qt::ShiftModifier : Qt::NoModifier);
            // Process queued events well before the 500 ms deadline to catch early commits.
            QTest::qWait(100);
            QCOMPARE(value(), before);
            QCOMPARE(view_->history_count(), history);
            QTRY_COMPARE_WITH_TIMEOUT(value(), percentByte(digit == 0 ? 100 : digit * 10),
                                      2000);
            QCOMPARE(other(), otherBefore);
            QCOMPARE(view_->history_count(), history + 1);
        }
    }
}

void LayerZeroShortcutsTest::exactPairs()
{
    for (const bool fill : {false, true}) {
        QWidget* source = fill ? static_cast<QWidget*>(tree_)
                               : static_cast<QWidget*>(canvas_);
        source->setFocus();
        QVERIFY(source->hasFocus());
        for (const int percent : {5, 54, 99}) {
            Qt::KeyboardModifiers modifiers = fill ? Qt::ShiftModifier : Qt::NoModifier;
            if (percent == 54) {
                modifiers |= Qt::KeypadModifier;
            }
            const int history = view_->history_index();
            const auto value = [&] { return fill ? row(layer_).fill : row(layer_).opacity; };
            QCOMPARE(value(), 255);
            QTest::keyClick(source, Qt::Key(Qt::Key_0 + percent / 10), modifiers);
            QCOMPARE(value(), 255);
            QCOMPARE(view_->history_index(), history);
            QTest::keyClick(source, Qt::Key(Qt::Key_0 + percent % 10), modifiers);
            QCOMPARE(value(), percentByte(percent));
            QCOMPARE(view_->history_index(), history + 1);
            QVERIFY(view_->undo());
            QCOMPARE(value(), 255);
            QCOMPARE(view_->history_index(), history);
        }
    }
}

void LayerZeroShortcutsTest::registeredNumericShortcut()
{
    canvas_->setFocus();
    QVERIFY(canvas_->hasFocus());
    QShortcut shortcut(QKeySequence(Qt::Key_8), canvas_);
    QSignalSpy activated(&shortcut, &QShortcut::activated);
    const int history = view_->history_count();

    QTest::keyClick(canvas_, Qt::Key_4);
    QTest::keyClick(canvas_, Qt::Key_8);
    QCOMPARE(activated.count(), 1);
    QTest::keyClick(canvas_, Qt::Key_5);

    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(view_->history_count(), history);
    QTRY_COMPARE_WITH_TIMEOUT(row(layer_).opacity, percentByte(50), 2000);
    QCOMPARE(view_->history_count(), history + 1);

    // A registered action can enter a nested loop before returning to Qt.
    connect(&shortcut, &QShortcut::activated, this, waitOutShortcut);
    QTest::keyClick(canvas_, Qt::Key_4);
    QTest::keyClick(canvas_, Qt::Key_8);
    QCOMPARE(activated.count(), 2);
    QCOMPARE(row(layer_).opacity, percentByte(50));
    QCOMPARE(view_->history_count(), history + 1);
}

void LayerZeroShortcutsTest::paintingCanvas()
{
    window_->setActiveTool(pictura::ToolId::Brush);
    canvas_->setFocus();
    QVERIFY(canvas_->hasFocus());
    const int history = view_->history_count();

    pressPair(canvas_, Qt::Key_9);
    pressPair(canvas_, Qt::Key_5, Qt::ShiftModifier);
    QTest::keyClick(canvas_, Qt::Key_3);
    waitOutShortcut();

    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(row(layer_).fill, 255);
    QCOMPARE(view_->history_count(), history);
}

void LayerZeroShortcutsTest::interruptedSequence()
{
    canvas_->setFocus();
    const int history = view_->history_count();

    QTest::keyPress(canvas_, Qt::Key_4);
    QKeyEvent repeat(QEvent::KeyPress, Qt::Key_4, Qt::NoModifier,
                     QStringLiteral("4"), true);
    QCoreApplication::sendEvent(canvas_, &repeat);
    QTest::keyRelease(canvas_, Qt::Key_4);
    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(view_->history_count(), history);
    QTRY_COMPARE_WITH_TIMEOUT(row(layer_).opacity, percentByte(40), 2000);
    QCOMPARE(view_->history_count(), history + 1);

    QTest::keyClick(canvas_, Qt::Key_5);
    QTRY_COMPARE_WITH_TIMEOUT(row(layer_).opacity, percentByte(50), 2000);
    QCOMPARE(view_->history_count(), history + 2);

    QTest::keyClick(canvas_, Qt::Key_7);
    QTest::keyClick(canvas_, Qt::Key_3, Qt::ShiftModifier);
    QCOMPARE(row(layer_).fill, 255);
    QTRY_COMPARE_WITH_TIMEOUT(row(layer_).fill, percentByte(30), 2000);
    QCOMPARE(row(layer_).opacity, percentByte(50));
    QCOMPARE(view_->history_count(), history + 3);

    QTest::keyClick(canvas_, Qt::Key_6);
    // Stall without processing timer events, then deliver a different-mode digit directly.
    QThread::msleep(550);
    QKeyEvent shiftedDigit(QEvent::KeyPress, Qt::Key_8, Qt::ShiftModifier,
                           QStringLiteral("8"));
    QCoreApplication::sendEvent(canvas_, &shiftedDigit);
    QCOMPARE(row(layer_).opacity, percentByte(60));
    QCOMPARE(row(layer_).fill, percentByte(30));
    QCOMPARE(view_->history_count(), history + 4);
    QTRY_COMPARE_WITH_TIMEOUT(row(layer_).fill, percentByte(80), 2000);
    QCOMPARE(view_->history_count(), history + 5);

    QTest::keyClick(canvas_, Qt::Key_9);
    QTest::keyClick(canvas_, Qt::Key_F12);
    waitOutShortcut();
    QCOMPARE(row(layer_).opacity, percentByte(60));
    QCOMPARE(view_->history_count(), history + 5);
}

void LayerZeroShortcutsTest::contextChanges()
{
    const QString other = view_->add_layer_in(QString());
    QVERIFY(!other.isEmpty());
    select({layer_});
    const int original = window_->activeDocumentIndex();
    QVERIFY(window_->newDocument(QStringLiteral("Second"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("transparent")));
    auto* second = window_->activeView();
    QVERIFY(second && second != view_);
    const int secondIndex = window_->activeDocumentIndex();
    const int secondHistory = second->history_count();
    window_->setActiveDocumentIndex(original);
    QCOMPARE(window_->activeView(), view_);
    canvas_->setFocus();
    QVERIFY(canvas_->hasFocus());
    const int history = view_->history_count();

    QTest::keyClick(canvas_, Qt::Key_6);
    select({other});
    select({layer_});
    waitOutShortcut();
    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(row(other).opacity, 255);

    QTest::keyClick(canvas_, Qt::Key_6);
    tree_->setFocus();
    QVERIFY(tree_->hasFocus());
    canvas_->setFocus();
    QVERIFY(canvas_->hasFocus());
    waitOutShortcut();
    QCOMPARE(row(layer_).opacity, 255);

    QTest::keyClick(canvas_, Qt::Key_6);
    window_->setActiveDocumentIndex(secondIndex);
    QCOMPARE(window_->activeView(), second);
    window_->setActiveDocumentIndex(original);
    QCOMPARE(window_->activeView(), view_);
    canvas_->setFocus();
    QVERIFY(canvas_->hasFocus());
    waitOutShortcut();
    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(second->history_count(), secondHistory);

    QTest::keyClick(canvas_, Qt::Key_6);
    window_->setActiveTool(pictura::ToolId::Brush);
    window_->setActiveTool(pictura::ToolId::Move);
    canvas_->setFocus();
    waitOutShortcut();
    QCOMPARE(row(layer_).opacity, 255);

    QTest::keyClick(canvas_, Qt::Key_6);
    QTest::mouseClick(canvas_, Qt::MiddleButton, Qt::NoModifier, canvas_->rect().center());
    waitOutShortcut();
    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(view_->history_count(), history);
}

void LayerZeroShortcutsTest::modifiedKeys()
{
    canvas_->setFocus();
    QVERIFY(canvas_->hasFocus());
    const int history = view_->history_count();
    for (const Qt::KeyboardModifier modifier : {Qt::AltModifier, Qt::MetaModifier}) {
        QTest::keyClick(canvas_, Qt::Key_0);
        QTest::keyClick(canvas_, Qt::Key_0, modifier);
        QTest::keyClick(canvas_, Qt::Key_0);
        QCOMPARE(row(layer_).opacity, 255);
        QCOMPARE(row(layer_).fill, 255);
        QCOMPARE(view_->history_count(), history);
        tree_->setFocus();
        canvas_->setFocus();
    }

    canvas_->setZoom(2.0, canvas_->rect().center());
    const double zoom = canvas_->zoom();
    QTest::keyClick(canvas_, Qt::Key_0);
    QTest::keyClick(canvas_, Qt::Key_0, Qt::ControlModifier);
    QVERIFY(canvas_->zoom() != zoom);
    QTest::keyClick(canvas_, Qt::Key_0);
    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(view_->history_count(), history);
    tree_->setFocus();
    canvas_->setFocus();

    QCOMPARE(view_->set_layers_opacity({layer_}, 179), 1);
    const int beforeReset = view_->history_count();
    window_->setUiScalePercent(120);
    QCOMPARE(window_->uiScalePercent(), 120);
    QTest::keyClick(canvas_, Qt::Key_0);
    QTest::keyClick(canvas_, Qt::Key_0, Qt::ControlModifier | Qt::AltModifier);
    QCOMPARE(window_->uiScalePercent(), 100);
    QTest::keyClick(canvas_, Qt::Key_0);
    QCOMPARE(row(layer_).opacity, 179);
    QCOMPARE(row(layer_).fill, 255);
    QCOMPARE(view_->history_count(), beforeReset);
    QTRY_COMPARE_WITH_TIMEOUT(row(layer_).opacity, 255, 2000);
    QCOMPARE(view_->history_count(), beforeReset + 1);
}

void LayerZeroShortcutsTest::pendingTransform()
{
    QVERIFY(pictura::edit_fill(*view_, qRgba(0, 0, 0, 255), -1,
                               QStringLiteral("normal"), 100, false));
    const int history = view_->history_count();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditFreeTransform)));
    QVERIFY(view_->transform_session_active());
    for (QWidget* source : {static_cast<QWidget*>(canvas_), static_cast<QWidget*>(tree_)}) {
        source->setFocus();
        QVERIFY(source->hasFocus());
        pressPair(source);
        pressPair(source, Qt::Key_0, Qt::ShiftModifier);
        QCOMPARE(row(layer_).opacity, 255);
        QCOMPARE(row(layer_).fill, 255);
        QCOMPARE(view_->history_count(), history);
    }
    canvas_->setFocus();
    QTest::keyClick(canvas_, Qt::Key_Escape);
    QVERIFY(!view_->transform_session_active());
    QCOMPARE(view_->history_count(), history);
}

void LayerZeroShortcutsTest::editorInput()
{
    auto* field = panel_->findChild<QWidget*>(QStringLiteral("layersOpacityField"));
    QVERIFY(field);
    auto* edit = field->findChild<QLineEdit*>(QStringLiteral("percentEdit"));
    QVERIFY(edit);
    edit->setFocus();
    edit->selectAll();
    const int history = view_->history_count();

    QTest::keyClicks(edit, QStringLiteral("00"));

    QCOMPARE(edit->text(), QStringLiteral("00"));
    QCOMPARE(row(layer_).opacity, 255);
    QCOMPARE(view_->history_count(), history);

    QTest::keyClick(edit, Qt::Key_Return);
    QCOMPARE(row(layer_).opacity, 0);
    QCOMPARE(view_->history_count(), history + 1);
}

PICTURA_QT_TEST_MAIN(LayerZeroShortcutsTest)
#include "tst_layer_zero_shortcuts.moc"
