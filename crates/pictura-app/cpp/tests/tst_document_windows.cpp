// Document windows (#317): the active window is the command target, tabs
// open and close documents, and Ctrl+Tab / Shift+Ctrl+Tab cycle every
// document window (#499). Each test function builds its own frame.

#include <QtTest/QtTest>

#include <QtGui/QShortcut>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QTabBar>

#include "api/type_api.h"
#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "image_view.h"
#include "keyboard_shortcuts_dialog.h"
#include "panels/panel_column.h"
#include "tool_sessions.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

// What a live type edit left behind in its document after a switch away.
struct TypeSnapshot {
    bool sessionActive = false;
    bool overlayActive = false;
    int layers = 0;
    int history = 0;
    QString historyLabel;
    QString text;
};

QAction* invertAction(const PicturaMainWindow& window)
{
    return window.registry()->action(commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Invert")}));
}

} // namespace

class DocumentWindowsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void openAddsTab();
    void switchingTabsChangesCanvas();
    void commandsTargetActiveDocument();
    void noDocumentsDisablesCommands();
    void closeDirtyCancelKeepsTab();
    void ctrlTabCyclesTabs();
    void ctrlTabWraps();
    void ctrlTabLeavesPanels();
    void ctrlTabFromTextField();
    void ctrlTabBlockedByModal();
    void ctrlTabSingleWindowNoop();
    void ctrlTabTypeSessionMatchesTabSwitch();
    void cycleSequencesAreReserved();

private:
    bool newDoc(const QString& name, int width, int height);
    QTabBar* tabBar() const;
    void closeAll();
    // Start a type edit in the first of two documents, switch to the second
    // with Ctrl+Tab or a tab click, and record the first document's state.
    void typeSessionAfterSwitch(bool viaShortcut, TypeSnapshot& out);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

bool DocumentWindowsTest::newDoc(const QString& name, int width, int height)
{
    return window_->newDocument(name, width, height, QStringLiteral("rgb"), 8,
                                QStringLiteral("white"));
}

QTabBar* DocumentWindowsTest::tabBar() const
{
    return window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
}

void DocumentWindowsTest::closeAll()
{
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
}

void DocumentWindowsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    setUnsavedPromptInteractive(false);
}

void DocumentWindowsTest::init()
{
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1100, 700);
    window_->show();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void DocumentWindowsTest::cleanup()
{
    if (window_) {
        window_->setActiveTool(ToolId::Move);
        window_->cancelAllOpens();
        closeAll();
    }
    window_.reset();
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
}

void DocumentWindowsTest::openAddsTab()
{
    QVERIFY(newDoc(QStringLiteral("First"), 64, 48));
    QVERIFY(newDoc(QStringLiteral("Second"), 32, 24));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QCOMPARE(bar->count(), 2);
    QCOMPARE(window_->documentCount(), 2);
    ImageView* first = window_->canvasAt(0);
    ImageView* second = window_->canvasAt(1);
    QVERIFY(first && second && first != second);
    QCOMPARE(first->documentSize(), QSize(64, 48));
    QCOMPARE(second->documentSize(), QSize(32, 24));
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentWindowsTest::switchingTabsChangesCanvas()
{
    QVERIFY(newDoc(QStringLiteral("First"), 64, 48));
    QVERIFY(newDoc(QStringLiteral("Second"), 32, 24));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QTest::mouseClick(bar, Qt::LeftButton, Qt::NoModifier, bar->tabRect(0).center());
    QTRY_COMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->imageView(), window_->canvasAt(0));
    QCOMPARE(window_->imageView()->documentSize(), QSize(64, 48));
    auto* size = window_->findChild<QLabel*>(QStringLiteral("statusSizeLabel"));
    QVERIFY(size != nullptr);
    QVERIFY2(size->text() == QStringLiteral("64 px x 48 px (72 ppi)")
                 || size->text() == QStringLiteral("W 64  H 48"),
             qPrintable(size->text()));
}

void DocumentWindowsTest::commandsTargetActiveDocument()
{
    QVERIFY(newDoc(QStringLiteral("First"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Second"), 16, 16));
    window_->setActiveDocumentIndex(0);
    window_->setActiveDocumentIndex(1);
    PictureView* first = window_->viewAt(0);
    PictureView* second = window_->viewAt(1);
    QVERIFY(first && second);
    const int firstHistory = first->history_index();
    const int secondHistory = second->history_index();

    window_->registry()->refresh();
    QAction* invert = invertAction(*window_);
    QVERIFY(invert && invert->isEnabled());
    invert->trigger();

    QCOMPARE(QColor(first->composite_argb(4, 4)), QColor(255, 255, 255));
    QCOMPARE(QColor(second->composite_argb(4, 4)), QColor(0, 0, 0));
    QCOMPARE(first->history_index(), firstHistory);
    QCOMPARE(second->history_index(), secondHistory + 1);
    QCOMPARE(second->history_label(second->history_index()), QStringLiteral("Invert"));
}

void DocumentWindowsTest::noDocumentsDisablesCommands()
{
    QVERIFY(newDoc(QStringLiteral("Only"), 16, 16));
    QVERIFY(window_->closeDocument(0, false));
    QCOMPARE(window_->documentCount(), 0);
    QVERIFY(window_->activeView() == nullptr);
    QVERIFY(window_->imageView() == nullptr);
    window_->registry()->refresh();
    QAction* invert = invertAction(*window_);
    QVERIFY(invert != nullptr);
    QVERIFY(!invert->isEnabled());
    QVERIFY(!window_->registry()->action(QLatin1String(command_ids::FileClose))->isEnabled());
    QVERIFY(!window_->registry()->action(QLatin1String(command_ids::FileSave))->isEnabled());
}

void DocumentWindowsTest::closeDirtyCancelKeepsTab()
{
    QVERIFY(newDoc(QStringLiteral("Dirty"), 16, 16));
    window_->registry()->refresh();
    invertAction(*window_)->trigger();
    QVERIFY(window_->isDocumentDirty(0));

    setNonInteractiveUnsavedChoice(UnsavedChoice::Cancel);
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    auto* close = qobject_cast<QAbstractButton*>(bar->tabButton(0, QTabBar::RightSide));
    QVERIFY(close != nullptr);
    QTest::mouseClick(close, Qt::LeftButton);
    QTest::qWait(50);
    QCOMPARE(window_->documentCount(), 1);
    QCOMPARE(bar->count(), 1);
    QVERIFY(window_->isDocumentDirty(0));
}

void DocumentWindowsTest::ctrlTabCyclesTabs()
{
    QVERIFY(newDoc(QStringLiteral("One"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Two"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Three"), 16, 16));
    window_->setActiveDocumentIndex(0);
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    bar->setFocus(Qt::OtherFocusReason);
    QTRY_VERIFY(bar->hasFocus());

    QTest::keyClick(bar, Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QTest::keyClick(bar, Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 2);
    QTest::keyClick(bar, Qt::Key_Tab, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(window_->activeDocumentIndex(), 1);

    // X11 delivers the backward key as Backtab; the same steps hold.
    window_->setActiveDocumentIndex(2);
    bar->setFocus(Qt::OtherFocusReason);
    QTest::keyClick(bar, Qt::Key_Backtab, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentWindowsTest::ctrlTabWraps()
{
    QVERIFY(newDoc(QStringLiteral("One"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Two"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Three"), 16, 16));
    QCOMPARE(window_->activeDocumentIndex(), 2);
    QTest::keyClick(window_->imageView(), Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QTest::keyClick(window_->imageView(), Qt::Key_Tab, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(window_->activeDocumentIndex(), 2);
}

void DocumentWindowsTest::ctrlTabLeavesPanels()
{
    QVERIFY(newDoc(QStringLiteral("One"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Two"), 16, 16));
    window_->setPanelsHidden(false);
    PanelColumn* column = window_->panelColumn();
    QVERIFY(column != nullptr);
    const bool columnShown = column->isVisible();

    QTest::keyClick(window_->imageView(), Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QVERIFY(!window_->panelsHidden());
    QCOMPARE(column->isVisible(), columnShown);
    QTest::keyClick(window_->imageView(), Qt::Key_Tab, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QVERIFY(!window_->panelsHidden());
    QCOMPARE(column->isVisible(), columnShown);
}

void DocumentWindowsTest::ctrlTabFromTextField()
{
    QVERIFY(newDoc(QStringLiteral("One"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Two"), 16, 16));
    window_->setPanelsHidden(false);
    QLineEdit* edit = nullptr;
    for (PanelColumn* column : window_->allPanelColumns()) {
        for (QLineEdit* candidate : column->findChildren<QLineEdit*>()) {
            if (candidate->isVisible() && candidate->isEnabled()) {
                edit = candidate;
                break;
            }
        }
        if (edit) {
            break;
        }
    }
    QVERIFY2(edit != nullptr, "a panel shows an enabled text field");
    edit->setFocus(Qt::OtherFocusReason);
    QTRY_VERIFY(edit->hasFocus());

    QTest::keyClick(edit, Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 0);
}

void DocumentWindowsTest::ctrlTabBlockedByModal()
{
    QVERIFY(newDoc(QStringLiteral("One"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Two"), 16, 16));
    QCOMPARE(window_->activeDocumentIndex(), 1);

    QDialog dialog(window_.get());
    dialog.setModal(true);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowActive(&dialog));
    QTest::keyClick(&dialog, Qt::Key_Tab, Qt::ControlModifier);
    QTest::keyClick(&dialog, Qt::Key_Tab, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    dialog.reject();
}

void DocumentWindowsTest::ctrlTabSingleWindowNoop()
{
    QVERIFY(newDoc(QStringLiteral("Only"), 16, 16));
    window_->setActiveTool(ToolId::Brush);
    const bool hidden = window_->panelsHidden();
    ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);

    QTest::keyClick(canvas, Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->imageView(), canvas);
    QCOMPARE(window_->panelsHidden(), hidden);
    QCOMPARE(window_->activeTool(), ToolId::Brush);
    QTest::keyClick(canvas, Qt::Key_Tab, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->panelsHidden(), hidden);
    QCOMPARE(window_->activeTool(), ToolId::Brush);
}

void DocumentWindowsTest::typeSessionAfterSwitch(bool viaShortcut, TypeSnapshot& out)
{
    QVERIFY(newDoc(QStringLiteral("Typed"), 200, 160));
    QVERIFY(newDoc(QStringLiteral("Other"), 200, 160));
    window_->setActiveDocumentIndex(0);
    auto* tools = window_->findChild<ToolController*>();
    QVERIFY(tools != nullptr);
    window_->setActiveTool(ToolId::HorizontalType);
    ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    canvas->mousePressed(QPointF(20, 60), int(Qt::LeftButton), 0);
    canvas->mouseReleased(QPointF(20, 60));
    QVERIFY(tools->textSession() && tools->textSession()->textActive());
    QTest::keyClicks(canvas, QStringLiteral("Hi"));

    if (viaShortcut) {
        QTest::keyClick(canvas, Qt::Key_Tab, Qt::ControlModifier);
    } else {
        QTabBar* bar = tabBar();
        QVERIFY(bar != nullptr);
        QTest::mouseClick(bar, Qt::LeftButton, Qt::NoModifier, bar->tabRect(1).center());
    }
    QTRY_COMPARE(window_->activeDocumentIndex(), 1);
    QCoreApplication::processEvents();

    PictureView* view = window_->viewAt(0);
    QVERIFY(view != nullptr);
    out.sessionActive = tools->textSession() && tools->textSession()->textActive();
    out.overlayActive = window_->canvasAt(0)->typeOverlayActiveForTest();
    out.layers = int(view->layer_count());
    out.history = int(view->history_index());
    out.historyLabel = view->history_label(view->history_index());
    for (int i = 0; i < out.layers; ++i) {
        const QString path = QString::number(i);
        if (type_api::isType(*view, path)) {
            out.text += type_api::layerText(*view, path) + QLatin1Char('|');
        }
    }

    window_->setActiveTool(ToolId::Move);
    closeAll();
}

void DocumentWindowsTest::ctrlTabTypeSessionMatchesTabSwitch()
{
    TypeSnapshot cycled;
    typeSessionAfterSwitch(true, cycled);
    if (QTest::currentTestFailed()) {
        return;
    }
    TypeSnapshot clicked;
    typeSessionAfterSwitch(false, clicked);
    if (QTest::currentTestFailed()) {
        return;
    }
    QCOMPARE(cycled.sessionActive, clicked.sessionActive);
    QCOMPARE(cycled.overlayActive, clicked.overlayActive);
    QCOMPARE(cycled.layers, clicked.layers);
    QCOMPARE(cycled.history, clicked.history);
    QCOMPARE(cycled.historyLabel, clicked.historyLabel);
    QCOMPARE(cycled.text, clicked.text);
}

void DocumentWindowsTest::cycleSequencesAreReserved()
{
    QList<QKeySequence> reserved;
    for (const WorkspaceKey& key : workspaceKeys()) {
        reserved << key.key;
    }
    QVERIFY(reserved.contains(QKeySequence(Qt::CTRL | Qt::Key_Tab)));
    QVERIFY(reserved.contains(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_Tab)));

    // The frame binds exactly these sequences to its cycle shortcuts.
    int bound = 0;
    for (const QShortcut* shortcut : window_->findChildren<QShortcut*>()) {
        if (shortcut->key() == QKeySequence(Qt::CTRL | Qt::Key_Tab)
            || shortcut->key() == QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_Tab)) {
            ++bound;
        }
    }
    QCOMPARE(bound, 2);
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<DocumentWindowsTest>(argc, argv);
}

#include "tst_document_windows.moc"
