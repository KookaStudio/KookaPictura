// Document windows (#317): the active window is the command target, tabs
// open and close documents, Ctrl+Tab / Shift+Ctrl+Tab cycle every document
// window (#499), and the tab context menu acts on the clicked tab. The suite
// shares one frame, which cleanup() returns to no documents and the Move tool;
// floats are hosted in the frame.

#include <QtTest/QtTest>

#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtCore/QUrl>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QShortcut>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QWidgetAction>

#include "api/document_api.h"
#include "api/type_api.h"
#include "command_ids.h"
#include "commands.h"
#include "dialogs.h"
#include "document_float_window.h"
#include "frame.h"
#include "image_view.h"
#include "keyboard_shortcuts_dialog.h"
#include "panels/panel_column.h"
#include "tool_sessions.h"
#include "tools.h"

#include "api/view_bridge.h"

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

bool writePsd(const QString& path)
{
    PictureView view;
    return document_api::newDocument(view, 4, 4, QStringLiteral("rgb"), 8,
                                     QStringLiteral("white"))
        && document_api::save(view, path);
}

QStringList menuEntries(const QMenu* menu)
{
    QStringList entries;
    for (const QAction* action : menu->actions()) {
        entries << (action->isSeparator() ? QStringLiteral("-") : action->text());
    }
    return entries;
}

QAction* menuAction(const QMenu* menu, const QString& text)
{
    for (QAction* action : menu->actions()) {
        if (action->text() == text) {
            return action;
        }
    }
    return nullptr;
}

QMenu* windowMenu(PicturaMainWindow& window)
{
    QWidget* control = window.findChild<QWidget*>(QStringLiteral("uiSizeControl"));
    if (!control) {
        return nullptr;
    }
    return qobject_cast<QMenu*>(control->parentWidget());
}

bool triggerCommand(PicturaMainWindow& window, const QString& id)
{
    window.registry()->refresh();
    QAction* action = window.registry()->action(id);
    if (!action || !action->isEnabled()) {
        return false;
    }
    action->trigger();
    QCoreApplication::processEvents();
    return true;
}

QList<QAction*> documentEntries(QMenu* menu)
{
    QList<QAction*> entries;
    for (QAction* action : menu->actions()) {
        if (action->objectName() == QLatin1String("windowDocumentEntry")) {
            entries.append(action);
        }
    }
    return entries;
}

QStringList documentEntryTexts(QMenu* menu)
{
    QStringList texts;
    for (QAction* action : documentEntries(menu)) {
        texts << action->text();
    }
    return texts;
}

// A menu label escapes a literal ampersand so it cannot become a mnemonic.
QString menuLabel(const QString& label)
{
    QString text = label;
    text.replace(QLatin1Char('&'), QStringLiteral("&&"));
    return text;
}

QAction* documentEntry(QMenu* menu, const QString& label)
{
    for (QAction* action : documentEntries(menu)) {
        if (action->text() == menuLabel(label)) {
            return action;
        }
    }
    return nullptr;
}

QAction* uiSizeRowAction(QMenu* menu)
{
    QWidget* control = menu->findChild<QWidget*>(QStringLiteral("uiSizeControl"));
    if (!control) {
        return nullptr;
    }
    for (QAction* action : menu->actions()) {
        auto* widgetAction = qobject_cast<QWidgetAction*>(action);
        if (widgetAction && widgetAction->defaultWidget() == control) {
            return action;
        }
    }
    return nullptr;
}

// Press a floating window's header without moving: it activates the window.
void clickHeader(DocumentFloatWindow* floating)
{
    QWidget* header = floating->header();
    QTest::mouseClick(header, Qt::LeftButton, Qt::NoModifier, QPoint(8, header->height() / 2));
}

} // namespace

class DocumentWindowsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void cleanupTestCase();

    void openAddsTab();
    void switchingTabsChangesCanvas();
    void commandsTargetActiveDocument();
    void noDocumentsDisablesCommands();
    void closeDirtyCancelKeepsTab();
    void closeDirtyDiscardClosesTab();
    void closeDirtySaveClosesTab();
    void ctrlTabCyclesTabs();
    void ctrlTabWraps();
    void ctrlTabLeavesPanels();
    void ctrlTabFromTextField();
    void ctrlTabBlockedByModal();
    void ctrlTabSingleWindowNoop();
    void ctrlTabTypeSessionMatchesTabSwitch();
    void cycleSequencesAreReserved();
    void floatingWindowBecomesTarget();
    void floatActivatedDuringLoadKeepsFocus();
    void ctrlTabReachesFloat();
    void tabMenuEntries();
    void tabMenuActsOnClickedTab();
    void closeOthersDisabledWhenAlone();
    void closeOthersKeepsClicked();
    void revealDisabledWithoutPath();
    void revealOpensContainingFolder();
    void moveDisabledForLoadingTab();
    void consolidateAllToHere();
    void windowMenuListOrder();
    void windowMenuFullLabels();
    void windowMenuChecksActive();
    void windowMenuActivatesHosts();
    void windowMenuFollowsChanges();
    void windowMenuListIsLast();
    void windowMenuEmptyWithoutDocuments();

private:
    bool newDoc(const QString& name, int width, int height);
    bool newNamedDoc(const QString& name, int width = 16, int height = 16);
    QTabBar* tabBar() const;
    void closeAll();
    // Open the context menu of tab `tab` and return it; the caller closes it.
    QMenu* tabMenu(int tab);
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

bool DocumentWindowsTest::newNamedDoc(const QString& name, int width, int height)
{
    if (!newDoc(name, width, height)) {
        return false;
    }
    // newDocument keeps the generated Untitled-N name; the labels use `name`.
    window_->setDocumentNameForTest(window_->documentCount() - 1, name);
    return true;
}

QTabBar* DocumentWindowsTest::tabBar() const
{
    return window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
}

QMenu* DocumentWindowsTest::tabMenu(int tab)
{
    QTabBar* bar = tabBar();
    if (!bar) {
        return nullptr;
    }
    const QPoint local = bar->tabRect(tab).center();
    QContextMenuEvent event(QContextMenuEvent::Mouse, local, bar->mapToGlobal(local));
    QApplication::sendEvent(bar, &event);
    return window_->documentMenuForTest();
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
    PanelFloat::setForceChildOverlayForTest(true);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1100, 700);
    window_->show();
}

void DocumentWindowsTest::init()
{
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
    PanelFloat::setForceChildOverlayForTest(true);
    QVERIFY(window_ != nullptr);
    QCOMPARE(window_->documentCount(), 0);
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void DocumentWindowsTest::cleanup()
{
    if (window_) {
        if (QMenu* menu = window_->documentMenuForTest()) {
            menu->close();
        }
        window_->setActiveTool(ToolId::Move);
        window_->setPanelsHidden(false);
        window_->setOpenDelayForTest(0);
        window_->setUrlOpenerForTest({});
        window_->cancelAllOpens();
        closeAll();
        QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    }
    PanelFloat::setForceChildOverlayForTest(false);
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
}

void DocumentWindowsTest::cleanupTestCase()
{
    window_.reset();
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
    // negative window: proves the cancelled close leaves the document open
    QTest::qWait(50);
    QCOMPARE(window_->documentCount(), 1);
    QCOMPARE(bar->count(), 1);
    QVERIFY(window_->isDocumentDirty(0));
}

void DocumentWindowsTest::closeDirtyDiscardClosesTab()
{
    QVERIFY(newDoc(QStringLiteral("Discard"), 16, 16));
    invertAction(*window_)->trigger();
    QVERIFY(window_->isDocumentDirty(0));
    const int prompts = nonInteractiveUnsavedPromptCount();
    setNonInteractiveUnsavedChoice(UnsavedChoice::Discard);
    QVERIFY(window_->closeDocument(0, true));
    QCOMPARE(window_->documentCount(), 0);
    QCOMPARE(nonInteractiveUnsavedPromptCount(), prompts + 1);
}

void DocumentWindowsTest::closeDirtySaveClosesTab()
{
    QTemporaryDir dir;
    const QString path = dir.filePath(QStringLiteral("close.psd"));
    QVERIFY(newDoc(QStringLiteral("Save"), 16, 16));
    QVERIFY(window_->saveActiveAs(path));
    invertAction(*window_)->trigger();
    QVERIFY(window_->isDocumentDirty(0));
    setNonInteractiveUnsavedChoice(UnsavedChoice::Save);
    QVERIFY(window_->closeDocument(0, true));
    QCOMPARE(window_->documentCount(), 0);
    PictureView reopened;
    QVERIFY(document_api::open(reopened, path));
    QCOMPARE(QColor(reopened.composite_argb(8, 8)), QColor(Qt::black));
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
        if (type_api::readLayer(*view, path).isType) {
            out.text += type_api::readLayer(*view, path).spec.text + QLatin1Char('|');
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

void DocumentWindowsTest::floatingWindowBecomesTarget()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 64, 48));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 24));
    QVERIFY(window_->floatDocumentForTest(1));
    window_->setActiveDocumentIndex(0);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    DocumentFloatWindow* floating = window_->floatWindowForTest(1);
    QVERIFY(floating != nullptr);

    clickHeader(floating);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QCOMPARE(window_->activeView(), window_->viewAt(1));
    QCOMPARE(window_->imageView(), window_->canvasAt(1));
    QVERIFY(floating->isAncestorOf(window_->imageView()));
    auto* size = window_->findChild<QLabel*>(QStringLiteral("statusSizeLabel"));
    QVERIFY(size != nullptr);
    QVERIFY2(size->text() == QStringLiteral("32 px x 24 px (72 ppi)")
                 || size->text() == QStringLiteral("W 32  H 24"),
             qPrintable(size->text()));

    // Menu commands follow: Invert changes only the floating document.
    PictureView* tabbed = window_->viewAt(0);
    const int tabbedHistory = tabbed->history_index();
    window_->registry()->refresh();
    invertAction(*window_)->trigger();
    QCOMPARE(tabbed->history_index(), tabbedHistory);
    QCOMPARE(QColor(window_->viewAt(1)->composite_argb(4, 4)), QColor(0, 0, 0));
}

void DocumentWindowsTest::floatActivatedDuringLoadKeepsFocus()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("held.psd"));
    QVERIFY(writePsd(path));
    QVERIFY(newDoc(QStringLiteral("Floating"), 32, 32));
    QVERIFY(window_->floatDocumentForTest(0));
    DocumentFloatWindow* floating = window_->floatWindowForTest(0);
    QVERIFY(floating != nullptr);

    window_->setOpenDelayForTest(300);
    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(1));
    clickHeader(floating);
    QCOMPARE(window_->activeDocumentIndex(), 0);

    QTRY_VERIFY(!window_->isLoadingDocument(1));
    QVERIFY(window_->viewAt(1)->has_document());
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->imageView(), window_->canvasAt(0));
}

void DocumentWindowsTest::ctrlTabReachesFloat()
{
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Floating"), 16, 16));
    QVERIFY(window_->floatDocumentForTest(1));
    window_->setActiveDocumentIndex(0);
    QCOMPARE(window_->activeDocumentIndex(), 0);

    QTest::keyClick(window_->imageView(), Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QVERIFY(window_->floatWindowForTest(1)->isAncestorOf(window_->imageView()));
    QTest::keyClick(window_->imageView(), Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 0);
}

void DocumentWindowsTest::windowMenuListOrder()
{
    QVERIFY(newNamedDoc(QStringLiteral("A")));
    QVERIFY(newNamedDoc(QStringLiteral("B")));
    QVERIFY(newNamedDoc(QStringLiteral("C")));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    bar->moveTab(0, 1);
    QCoreApplication::processEvents();
    QVERIFY(window_->floatDocumentForTest(2));
    QCOMPARE(window_->tabOfDocumentForTest(2), -1);

    QMenu* menu = windowMenu(*window_);
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();
    QCOMPARE(documentEntryTexts(menu),
             QStringList({menuLabel(window_->windowLabelForTest(1, 0)),
                          menuLabel(window_->windowLabelForTest(0, 0)),
                          menuLabel(window_->windowLabelForTest(2, 0))}));

    QVERIFY(triggerCommand(*window_, QStringLiteral("window.arrange.tile")));
    emit menu->aboutToShow();
    QCOMPARE(documentEntryTexts(menu),
             QStringList({menuLabel(window_->windowLabelForTest(1, 0)),
                          menuLabel(window_->windowLabelForTest(0, 0)),
                          menuLabel(window_->windowLabelForTest(2, 0))}));
}

void DocumentWindowsTest::windowMenuFullLabels()
{
    QVERIFY(newNamedDoc(QStringLiteral("A&B")));
    QVERIFY(triggerCommand(*window_, QStringLiteral("window.arrange.new.window.for.file")));
    QCOMPARE(window_->documentCount(), 1);
    QCOMPARE(window_->windowCountForTest(0), 2);
    QCOMPARE(window_->imageView(), window_->windowCanvasForTest(0, 1));
    const QString first = window_->windowLabelForTest(0, 0);
    const QString second = window_->windowLabelForTest(0, 1);
    QVERIFY2(second.startsWith(QStringLiteral("A&B:2 ")), qPrintable(second));
    QVERIFY2(!first.startsWith(QStringLiteral("A&B:2")), qPrintable(first));

    QMenu* menu = windowMenu(*window_);
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();
    QList<QAction*> entries = documentEntries(menu);
    QCOMPARE(entries.size(), 2);
    QVERIFY(entries.at(1)->isChecked());
    QCOMPARE(documentEntryTexts(menu), QStringList({menuLabel(first), menuLabel(second)}));
}

void DocumentWindowsTest::windowMenuChecksActive()
{
    QVERIFY(newNamedDoc(QStringLiteral("A")));
    QVERIFY(newNamedDoc(QStringLiteral("B")));
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QMenu* menu = windowMenu(*window_);
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();
    QList<QAction*> entries = documentEntries(menu);
    QCOMPARE(entries.size(), 2);
    QVERIFY(!entries.at(0)->isChecked());
    QVERIFY(entries.at(1)->isChecked());

    QTest::keyClick(window_->imageView(), Qt::Key_Tab, Qt::ControlModifier);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    emit menu->aboutToShow();
    entries = documentEntries(menu);
    QCOMPARE(entries.size(), 2);
    QVERIFY(entries.at(0)->isChecked());
    QVERIFY(!entries.at(1)->isChecked());
}

void DocumentWindowsTest::windowMenuActivatesHosts()
{
    QVERIFY(newNamedDoc(QStringLiteral("Tab")));
    QVERIFY(newNamedDoc(QStringLiteral("Float")));
    QVERIFY(window_->floatDocumentForTest(1));
    window_->setActiveDocumentIndex(0);
    QCOMPARE(window_->activeDocumentIndex(), 0);

    QMenu* menu = windowMenu(*window_);
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();
    QAction* floating = documentEntry(menu, window_->windowLabelForTest(1, 0));
    QVERIFY(floating != nullptr);
    floating->trigger();
    QCoreApplication::processEvents();
    QCOMPARE(window_->activeDocumentIndex(), 1);

    emit menu->aboutToShow();
    QAction* tabbed = documentEntry(menu, window_->windowLabelForTest(0, 0));
    QVERIFY(tabbed != nullptr);
    tabbed->trigger();
    QCoreApplication::processEvents();
    QCOMPARE(window_->activeDocumentIndex(), 0);

    QVERIFY(triggerCommand(*window_, QStringLiteral("window.arrange.tile")));
    emit menu->aboutToShow();
    QAction* pane = documentEntry(menu, window_->windowLabelForTest(1, 0));
    QVERIFY(pane != nullptr);
    pane->trigger();
    QCoreApplication::processEvents();
    QCOMPARE(window_->activeDocumentIndex(), 1);
}

void DocumentWindowsTest::windowMenuFollowsChanges()
{
    QVERIFY(newNamedDoc(QStringLiteral("A")));
    QVERIFY(newNamedDoc(QStringLiteral("B")));
    QVERIFY(newNamedDoc(QStringLiteral("C")));
    QMenu* menu = windowMenu(*window_);
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();
    QCOMPARE(documentEntries(menu).size(), 3);

    QVERIFY(newNamedDoc(QStringLiteral("D")));

    window_->setDocumentNameForTest(0, QStringLiteral("Renamed"));
    QVERIFY(window_->floatDocumentForTest(1));
    QVERIFY(window_->closeDocument(2, false));
    QCOMPARE(window_->documentCount(), 3);

    emit menu->aboutToShow();
    QCOMPARE(documentEntryTexts(menu),
             QStringList({menuLabel(window_->windowLabelForTest(0, 0)),
                          menuLabel(window_->windowLabelForTest(2, 0)),
                          menuLabel(window_->windowLabelForTest(1, 0))}));
    QVERIFY(documentEntries(menu).at(0)->text().startsWith(QStringLiteral("Renamed ")));
    QVERIFY(documentEntries(menu).at(1)->text().startsWith(QStringLiteral("D ")));
}

void DocumentWindowsTest::windowMenuListIsLast()
{
    QVERIFY(newNamedDoc(QStringLiteral("A")));
    QMenu* menu = windowMenu(*window_);
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();

    QAction* row = uiSizeRowAction(menu);
    QVERIFY(row != nullptr);
    QAction* options = window_->registry()->action(QStringLiteral("view.options"));
    QVERIFY(options != nullptr);
    const QList<QAction*> actions = menu->actions();
    const int rowIndex = actions.indexOf(row);
    const int optionsIndex = actions.indexOf(options);
    QVERIFY2(rowIndex > optionsIndex, "the UI Size row follows Options");

    int firstEntry = -1;
    for (int i = rowIndex + 1; i < actions.size(); ++i) {
        if (actions.at(i)->objectName() == QLatin1String("windowDocumentEntry")) {
            firstEntry = i;
            break;
        }
    }
    QVERIFY2(firstEntry > rowIndex, "the document list follows the UI Size row");
    QVERIFY(actions.at(firstEntry - 1)->isSeparator());
    QCOMPARE(actions.at(firstEntry - 1)->objectName(),
             QStringLiteral("windowDocumentSeparator"));
    QVERIFY(actions.at(firstEntry - 1)->isVisible());
    for (int i = firstEntry; i < actions.size(); ++i) {
        QCOMPARE(actions.at(i)->objectName(), QStringLiteral("windowDocumentEntry"));
        QVERIFY(actions.at(i)->isVisible());
    }
}

void DocumentWindowsTest::windowMenuEmptyWithoutDocuments()
{
    QCOMPARE(window_->documentCount(), 0);
    QMenu* menu = windowMenu(*window_);
    QVERIFY(menu != nullptr);
    emit menu->aboutToShow();
    QVERIFY(documentEntries(menu).isEmpty());

    QAction* row = uiSizeRowAction(menu);
    QVERIFY(row != nullptr);
    const QList<QAction*> actions = menu->actions();
    const int rowIndex = actions.indexOf(row);
    QVERIFY(rowIndex >= 0);
    for (int i = rowIndex + 1; i < actions.size(); ++i) {
        QVERIFY2(!actions.at(i)->isVisible(), qPrintable(actions.at(i)->text()));
    }
}

void DocumentWindowsTest::tabMenuEntries()
{
    QVERIFY(newDoc(QStringLiteral("Menu"), 16, 16));
    QMenu* menu = tabMenu(0);
    QVERIFY(menu != nullptr);
    QCOMPARE(menuEntries(menu),
             QStringList({QStringLiteral("Consolidate All to Here"), QStringLiteral("Close"),
                          QStringLiteral("Close All"), QStringLiteral("Close Others"),
                          QStringLiteral("Move to New Window"), QStringLiteral("-"),
                          QStringLiteral("New Document…"), QStringLiteral("Open Document…"),
                          QStringLiteral("-"), QStringLiteral("Reveal in File Manager")}));
    menu->close();
}

void DocumentWindowsTest::tabMenuActsOnClickedTab()
{
    QVERIFY(newDoc(QStringLiteral("First"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Second"), 16, 16));
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QMenu* menu = tabMenu(0);
    QVERIFY(menu != nullptr);
    QAction* move = menuAction(menu, QStringLiteral("Move to New Window"));
    QVERIFY(move && move->isEnabled());
    menu->close();
    move->trigger();

    QVERIFY(window_->floatWindowForTest(0) != nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(0), -1);
    QVERIFY(window_->floatWindowForTest(1) == nullptr);
    QCOMPARE(window_->tabOfDocumentForTest(1), 0);
}

void DocumentWindowsTest::closeOthersDisabledWhenAlone()
{
    QVERIFY(newDoc(QStringLiteral("Alone"), 16, 16));
    QMenu* menu = tabMenu(0);
    QVERIFY(menu != nullptr);
    QAction* others = menuAction(menu, QStringLiteral("Close Others"));
    QVERIFY(others != nullptr);
    QVERIFY(!others->isEnabled());
    menu->close();
}

void DocumentWindowsTest::closeOthersKeepsClicked()
{
    QVERIFY(newDoc(QStringLiteral("One"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Two"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Three"), 16, 16));
    PictureView* kept = window_->viewAt(1);
    QMenu* menu = tabMenu(1);
    QVERIFY(menu != nullptr);
    QAction* others = menuAction(menu, QStringLiteral("Close Others"));
    QVERIFY(others && others->isEnabled());
    menu->close();
    others->trigger();

    QCOMPARE(window_->documentCount(), 1);
    QCOMPARE(window_->viewAt(0), kept);
    QCOMPARE(tabBar()->count(), 1);
}

void DocumentWindowsTest::revealDisabledWithoutPath()
{
    QVERIFY(newDoc(QStringLiteral("Untitled"), 16, 16));
    QMenu* menu = tabMenu(0);
    QVERIFY(menu != nullptr);
    QAction* reveal = menuAction(menu, QStringLiteral("Reveal in File Manager"));
    QVERIFY(reveal != nullptr);
    QVERIFY(!reveal->isEnabled());
    menu->close();
}

void DocumentWindowsTest::revealOpensContainingFolder()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("reveal.psd"));
    QVERIFY(writePsd(path));
    QVERIFY(window_->openPath(path));
    QList<QUrl> opened;
    window_->setUrlOpenerForTest([&opened](const QUrl& url) { opened << url; });

    QMenu* menu = tabMenu(0);
    QVERIFY(menu != nullptr);
    QAction* reveal = menuAction(menu, QStringLiteral("Reveal in File Manager"));
    QVERIFY(reveal && reveal->isEnabled());
    menu->close();
    reveal->trigger();
    window_->setUrlOpenerForTest({});

    QCOMPARE(opened.size(), 1);
    QCOMPARE(opened.first(), QUrl::fromLocalFile(QFileInfo(path).absolutePath()));
}

void DocumentWindowsTest::moveDisabledForLoadingTab()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("loading.psd"));
    QVERIFY(writePsd(path));
    window_->setOpenDelayForTest(500);
    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));

    QMenu* menu = tabMenu(0);
    QVERIFY(menu != nullptr);
    QAction* move = menuAction(menu, QStringLiteral("Move to New Window"));
    QVERIFY(move != nullptr);
    QVERIFY(!move->isEnabled());
    menu->close();
    QVERIFY(!window_->floatDocumentForTest(0));
}

void DocumentWindowsTest::consolidateAllToHere()
{
    QVERIFY(newDoc(QStringLiteral("FloatA"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("FloatB"), 16, 16));
    QVERIFY(newDoc(QStringLiteral("Tabbed"), 16, 16));
    QVERIFY(window_->floatDocumentForTest(0));
    QVERIFY(window_->floatDocumentForTest(1));
    QCOMPARE(tabBar()->count(), 1);
    QCOMPARE(window_->activeDocumentIndex(), 1);

    QMenu* menu = tabMenu(0);
    QVERIFY(menu != nullptr);
    QAction* consolidate = menuAction(menu, QStringLiteral("Consolidate All to Here"));
    QVERIFY(consolidate && consolidate->isEnabled());
    menu->close();
    consolidate->trigger();

    QCOMPARE(tabBar()->count(), 3);
    for (int i = 0; i < 3; ++i) {
        QVERIFY(window_->floatWindowForTest(i) == nullptr);
        QVERIFY(window_->tabOfDocumentForTest(i) >= 0);
    }
    QCOMPARE(window_->activeDocumentIndex(), 2);
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<DocumentWindowsTest>(argc, argv);
}

#include "tst_document_windows.moc"
