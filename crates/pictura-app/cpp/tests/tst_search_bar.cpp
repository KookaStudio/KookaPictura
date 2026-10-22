// Help > Search (#788): the Shift+Space overlay over every UI entry point. Not a
// CS6 feature; post-CS6 by owner decision. The shortcut and the keys text input
// keeps, the index and its ranking, run and reveal, dismissal, and the
// status-bar hint.

#include <QtTest/QtTest>

#include <QtGui/QAction>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QStatusBar>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "search_bar.h"
#include "search_index.h"
#include "selftest_paint_fixture.h"
#include "tool_hint_bar.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/warp.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

const QString kSearch = QString::fromLatin1(pictura::command_ids::HelpSearch);
const QString kWarp = QString::fromLatin1(pictura::command_ids::EditTransformWarp);

pictura::SearchEntry entry(const QStringList& path, bool runnable = true)
{
    pictura::SearchEntry e;
    e.path = path;
    e.runnable = runnable;
    return e;
}

QMenu* owningMenu(QAction* action)
{
    for (QObject* object : action->associatedObjects()) {
        if (auto* menu = qobject_cast<QMenu*>(object)) {
            return menu;
        }
    }
    return nullptr;
}

void closePopups()
{
    while (QWidget* popup = QApplication::activePopupWidget()) {
        popup->close();
    }
}

} // namespace

class SearchBarTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void shiftSpaceOpens();
    void textFieldKeepsShiftSpace();
    void typeEditKeepsShiftSpace();
    void spaceStillPans();
    void indexCoversSources();
    void curatedControlsExist();
    void warpFindsTransformWarp();
    void rankingOrder();
    void enterRuns();
    void stubIsNotRunnable();
    void altEnterRevealsMenu();
    void dismissal();
    void reopensAfterEscape();
    void rebindUpdatesHint();

private:
    pictura::SearchBar* openBar(const QString& query = QString());
    pictura::SearchBar* bar() const { return window_->findChild<pictura::SearchBar*>(); }
    // A document whose second layer carries a filled box, so a transform can run.
    bool openTransformableDocument();
    QList<pictura::SearchEntry> index() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SearchBarTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
}

void SearchBarTest::cleanup()
{
    closePopups();
    if (pictura::SearchBar* searchBar = bar()) {
        searchBar->dismiss();
    }
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    window_->setActiveTool(pictura::ToolId::Move);
}

pictura::SearchBar* SearchBarTest::openBar(const QString& query)
{
    window_->registry()->dispatch(kSearch);
    pictura::SearchBar* searchBar = bar();
    if (searchBar && !query.isEmpty()) {
        QTest::keyClicks(searchBar->input(), query);
    }
    return searchBar;
}

bool SearchBarTest::openTransformableDocument()
{
    if (!window_->newDocument(QStringLiteral("Search"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    pictura::PictureView* view = window_->activeView();
    const int added = view ? view->add_layer(0) : 0;
    if (added <= 0) {
        return false;
    }
    window_->selectLayerPath(QString::number(added));
    const bool filled = view->select_rect(10, 8, 12, 9, QStringLiteral("new"), 0.0)
                        && pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1,
                                              QStringLiteral("normal"), 100, false);
    view->deselect();
    window_->registry()->refresh();
    return filled;
}

QList<pictura::SearchEntry> SearchBarTest::index() const
{
    window_->registry()->refresh();
    return pictura::buildSearchIndex(*window_->registry());
}

void SearchBarTest::shiftSpaceOpens()
{
    window_->activateWindow();
    window_->setFocus();
    QTest::keyClick(window_.get(), Qt::Key_Space, Qt::ShiftModifier);
    pictura::SearchBar* searchBar = bar();
    QVERIFY2(searchBar, "Shift+Space creates the bar");
    QTRY_VERIFY(searchBar->isVisible());
    QVERIFY(searchBar->input()->text().isEmpty());
    QTRY_VERIFY(searchBar->input()->hasFocus());
}

void SearchBarTest::textFieldKeepsShiftSpace()
{
    auto* field = new QLineEdit(window_->statusBar());
    field->show();
    field->setFocus();
    QTRY_VERIFY(field->hasFocus());
    QTest::keyClick(field, Qt::Key_Space, Qt::ShiftModifier);
    QCOMPARE(field->text(), QStringLiteral(" "));
    QVERIFY(!bar() || !bar()->isVisible());
    delete field;
}

void SearchBarTest::typeEditKeepsShiftSpace()
{
    QImage seed(200, 160, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_search_type"));
    QVERIFY2(f.ok(), "type fixture");
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(20, 60)});
    f.canvas->setFocus();
    QTest::keyClicks(f.canvas, QStringLiteral("ab"));
    QVERIFY(f.tools->textSession() && f.tools->textSession()->textActive());
    const QLineF before = f.canvas->typeOverlayCaretForTest();

    QTest::keyClick(f.canvas, Qt::Key_Space, Qt::ShiftModifier);
    QVERIFY(!bar() || !bar()->isVisible());
    QVERIFY2(f.canvas->typeOverlayCaretForTest() != before, "the space moved the caret");
    QVERIFY(f.tools->textSession()->cancelText());
}

void SearchBarTest::spaceStillPans()
{
    QImage seed(160, 160, QImage::Format_RGB32);
    seed.fill(Qt::red);
    Fixture f(*window_, seed, QStringLiteral("pictura_search_pan"));
    QVERIFY2(f.ok(), "pan fixture");
    const auto send = [this](QEvent::Type type, Qt::KeyboardModifiers mods) {
        QKeyEvent event(type, Qt::Key_Space, mods, QStringLiteral(" "));
        QApplication::sendEvent(window_.get(), &event);
    };

    send(QEvent::KeyPress, Qt::NoModifier);
    QVERIFY2(f.canvas->spacePanForTest(), "bare Space pans");
    send(QEvent::KeyRelease, Qt::NoModifier);
    QVERIFY(!f.canvas->spacePanForTest());

    send(QEvent::KeyPress, Qt::ShiftModifier);
    QVERIFY2(!f.canvas->spacePanForTest(), "Shift+Space does not pan");
    send(QEvent::KeyRelease, Qt::ShiftModifier);
}

void SearchBarTest::indexCoversSources()
{
    const QList<pictura::SearchEntry> entries = index();
    using Kind = pictura::SearchEntry::Kind;
    const auto find = [&entries](Kind kind, const QString& id) {
        int count = 0;
        const pictura::SearchEntry* found = nullptr;
        for (const pictura::SearchEntry& e : entries) {
            if (e.kind == kind && e.id == id) {
                ++count;
                found = &e;
            }
        }
        return std::make_pair(count, found);
    };

    const auto warp = find(Kind::Command, kWarp);
    QCOMPARE(warp.first, 1);
    QCOMPARE(warp.second->path,
             QStringList({QStringLiteral("Edit"), QStringLiteral("Transform"),
                          QStringLiteral("Warp")}));
    QVERIFY(warp.second->implemented);

    const auto brush = find(Kind::Tool, QStringLiteral("brush"));
    QCOMPARE(brush.first, 1);
    QCOMPARE(brush.second->path.first(), QStringLiteral("Tools"));
    QCOMPARE(brush.second->shortcut, QStringLiteral("B"));

    const auto layers = find(Kind::Panel, QStringLiteral("layersPanel"));
    QCOMPARE(layers.first, 1);
    QVERIFY(layers.second->runnable);
    // Type > Panels repeats Character; the Window menu's entry is the one kept.
    const auto character = find(Kind::Panel, QStringLiteral("characterPanel"));
    QCOMPARE(character.first, 1);
    QCOMPARE(character.second->path.first(), QStringLiteral("Window"));

    const auto general = find(Kind::Preference, QStringLiteral("General"));
    QCOMPARE(general.first, 1);
    QVERIFY(general.second->runnable);
    QCOMPARE(general.second->path.mid(0, 2),
             QStringList({QStringLiteral("Edit"), QStringLiteral("Preferences")}));

    const auto blend = find(Kind::Control, QStringLiteral("layersBlendCombo"));
    QCOMPARE(blend.first, 1);
    QCOMPARE(blend.second->container, QStringLiteral("layersPanel"));

    QCOMPARE(find(Kind::Command, kSearch).first, 0);
    for (const pictura::SearchEntry& e : entries) {
        QVERIFY2(!e.label().endsWith(QChar(0x2026)) && !e.label().endsWith(QStringLiteral("...")),
                 qPrintable(e.path.join(QStringLiteral(" > "))));
    }
}

void SearchBarTest::curatedControlsExist()
{
    for (const pictura::SearchControl& control : pictura::searchControls()) {
        const QString name = QString::fromLatin1(control.control);
        auto* container = window_->findChild<QWidget*>(QString::fromLatin1(control.container));
        QVERIFY2(container, control.container);
        QVERIFY2(!container->findChildren<QWidget*>(name).isEmpty(), control.control);
        if (control.tool) {
            bool known = false;
            for (pictura::ToolId id : pictura::allToolIds()) {
                known = known || pictura::toolIdName(id) == QString::fromLatin1(control.tool);
            }
            QVERIFY2(known, control.tool);
        }
    }
}

void SearchBarTest::warpFindsTransformWarp()
{
    const QList<pictura::SearchEntry> entries = index();
    const QList<pictura::SearchHit> hits = pictura::searchIndex(entries, QStringLiteral("WARP"));
    QVERIFY(!hits.isEmpty());
    QCOMPARE(entries.at(hits.first().index).id, kWarp);

    pictura::SearchBar* searchBar = openBar(QStringLiteral("warp"));
    QVERIFY(searchBar && searchBar->isVisible());
    QVERIFY(searchBar->results()->count() > 0);
    QCOMPARE(searchBar->results()->item(0)->text(), QStringLiteral("Edit > Transform > Warp"));
}

void SearchBarTest::rankingOrder()
{
    const QList<pictura::SearchEntry> synthetic = {
        entry({QStringLiteral("Layer"), QStringLiteral("New"), QStringLiteral("Group")}),
        entry({QStringLiteral("X"), QStringLiteral("Multilayered")}),
        entry({QStringLiteral("X"), QStringLiteral("New Layer")}),
        entry({QStringLiteral("X"), QStringLiteral("Layer Mask")}),
        entry({QStringLiteral("X"), QStringLiteral("Layer")}, false),
        entry({QStringLiteral("A"), QStringLiteral("B"), QStringLiteral("Layer")}),
        entry({QStringLiteral("X"), QStringLiteral("Layer")}),
        entry({QStringLiteral("X"), QStringLiteral("Unrelated")}),
    };
    QList<int> order;
    for (const pictura::SearchHit& hit : pictura::searchIndex(synthetic, QStringLiteral("layer"))) {
        order << hit.index;
    }
    // Exact (runnable, shorter path first, then greyed), prefix, word prefix,
    // substring, path only; the unrelated entry is left out.
    QCOMPARE(order, QList<int>({6, 5, 4, 3, 2, 1, 0}));

    // Every word must match; one may come from the path.
    const QList<pictura::SearchEntry> entries = index();
    const QList<pictura::SearchHit> hits =
        pictura::searchIndex(entries, QStringLiteral("transform warp"));
    QVERIFY(!hits.isEmpty());
    QCOMPARE(entries.at(hits.first().index).id, kWarp);
    for (const pictura::SearchHit& hit : hits) {
        const QString joined = entries.at(hit.index).path.join(QLatin1Char(' ')).toLower();
        QVERIFY2(joined.contains(QStringLiteral("transform")) && joined.contains(QStringLiteral("warp")),
                 qPrintable(joined));
    }
    QVERIFY(pictura::searchIndex(entries, QStringLiteral("   ")).isEmpty());
}

void SearchBarTest::enterRuns()
{
    QVERIFY(openTransformableDocument());
    pictura::PictureView* view = window_->activeView();

    pictura::SearchBar* searchBar = openBar(QStringLiteral("warp"));
    QVERIFY(searchBar);
    QVERIFY(searchBar->results()->item(0)->data(pictura::SearchBar::kRunnableRole).toBool());
    QTest::keyClick(searchBar->input(), Qt::Key_Return);
    QVERIFY(!searchBar->isVisible());
    QVERIFY2(view->transform_session_active(), "Enter starts the transform");
    QVERIFY(pictura::transform_warp_active(*view));
    window_->findChild<pictura::ToolController*>()->cancelFreeTransform();

    searchBar = openBar(QStringLiteral("magic wand"));
    QCOMPARE(searchBar->entryAt(0)->kind, pictura::SearchEntry::Kind::Tool);
    QTest::keyClick(searchBar->input(), Qt::Key_Return);
    QCOMPARE(window_->activeTool(), pictura::ToolId::MagicWand);
}

void SearchBarTest::stubIsNotRunnable()
{
    pictura::SearchBar* searchBar = openBar(QStringLiteral("zoomify"));
    QVERIFY(searchBar && searchBar->results()->count() > 0);
    QListWidgetItem* item = searchBar->results()->item(0);
    QCOMPARE(item->text(), QStringLiteral("File > Export > Zoomify"));
    QVERIFY2(!item->data(pictura::SearchBar::kRunnableRole).toBool(), "the stub row is greyed");
    QVERIFY(!searchBar->entryAt(0)->implemented);

    QTest::keyClick(searchBar->input(), Qt::Key_Return);
    QVERIFY2(searchBar->isVisible(), "Enter on a stub runs nothing");
    const QRect row = searchBar->results()->visualItemRect(item);
    QTest::mouseClick(searchBar->results()->viewport(), Qt::LeftButton, Qt::NoModifier,
                      row.center());
    QVERIFY2(searchBar->isVisible(), "a click on a stub runs nothing");
}

void SearchBarTest::altEnterRevealsMenu()
{
    QVERIFY(openTransformableDocument());
    QAction* warp = window_->registry()->action(kWarp);
    QMenu* transform = owningMenu(warp);
    QVERIFY(transform);
    QMenu* edit = owningMenu(transform->menuAction());
    QVERIFY(edit);

    pictura::SearchBar* searchBar = openBar(QStringLiteral("warp"));
    QTest::keyClick(searchBar->input(), Qt::Key_Return, Qt::AltModifier);
    QVERIFY(!searchBar->isVisible());
    QTRY_VERIFY(edit->isVisible());
    QTRY_VERIFY(transform->isVisible());
    QTRY_COMPARE(transform->activeAction(), warp);
    QVERIFY2(!window_->activeView()->transform_session_active(), "reveal runs nothing");
}

void SearchBarTest::dismissal()
{
    pictura::SearchBar* searchBar = openBar();
    QVERIFY(searchBar && searchBar->isVisible());
    QTest::keyClick(searchBar->input(), Qt::Key_Escape);
    QVERIFY2(!searchBar->isVisible(), "Escape closes");

    searchBar = openBar(QStringLiteral("layer"));
    QVERIFY(searchBar->isVisible());
    QTest::mouseClick(window_->statusBar(), Qt::LeftButton, Qt::NoModifier, QPoint(4, 4));
    QVERIFY2(!searchBar->isVisible(), "a press outside closes");
}

// Shift+Space, Escape, Shift+Space with nothing focused beforehand: closing must
// not hand the focus to a text field, which would then swallow the key.
void SearchBarTest::reopensAfterEscape()
{
    if (QWidget* focused = QApplication::focusWidget()) {
        focused->clearFocus();
    }
    QVERIFY(!QApplication::focusWidget());
    const auto press = [this](Qt::Key key, Qt::KeyboardModifiers mods) {
        QWidget* target = QApplication::focusWidget();
        QTest::keyClick(target ? target : window_.get(), key, mods);
    };

    for (int round = 0; round < 2; ++round) {
        press(Qt::Key_Space, Qt::ShiftModifier);
        pictura::SearchBar* searchBar = bar();
        QVERIFY(searchBar);
        QTRY_VERIFY2(searchBar->isVisible(), qPrintable(QStringLiteral("round %1").arg(round)));
        QTRY_VERIFY(searchBar->input()->hasFocus());
        press(Qt::Key_Escape, Qt::NoModifier);
        QVERIFY(!searchBar->isVisible());
        QWidget* focused = QApplication::focusWidget();
        QVERIFY2(!qobject_cast<QLineEdit*>(focused),
                 qPrintable(QStringLiteral("focus went to %1").arg(
                     focused ? focused->objectName() : QString())));
    }
}

void SearchBarTest::rebindUpdatesHint()
{
    auto* hint = window_->findChild<pictura::ToolHintBar*>(QStringLiteral("statusSearchHint"));
    QVERIFY(hint);
    QCOMPARE(hint->hintCountForTest(), 1);
    QCOMPARE(hint->keyForTest(0),
             QKeySequence(QStringLiteral("Shift+Space")).toString(QKeySequence::NativeText));
    QCOMPARE(hint->textForTest(0), QStringLiteral("Search"));

    const QKeySequence rebound(QStringLiteral("Ctrl+Shift+F12"));
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry->commandForShortcut(rebound).isEmpty());
    const QList<QKeySequence> defaults = registry->defaultShortcuts(kSearch);
    registry->setShortcuts(kSearch, {rebound});
    QCOMPARE(hint->keyForTest(0), rebound.toString(QKeySequence::NativeText));

    registry->setShortcuts(kSearch, {});
    QVERIFY(hint->isHidden());
    registry->setShortcuts(kSearch, defaults);
    QVERIFY(!hint->isHidden());
}

QTEST_MAIN(SearchBarTest)

#include "tst_search_bar.moc"
