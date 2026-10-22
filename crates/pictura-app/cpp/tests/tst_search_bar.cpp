// Help > Search (#788): the Shift+Space overlay over every UI entry point. Not a
// CS6 feature; post-CS6 by owner decision. The shortcut and the keys text input
// keeps, the index and its ranking, run and reveal, dismissal, and the
// status-bar hint.

#include <QtTest/QtTest>

#include <algorithm>

#include <QtGui/QAction>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QToolButton>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "preferences_dialog.h"
#include "search_bar.h"
#include "search_index.h"
#include "selftest_paint_fixture.h"
#include "tool_hint_bar.h"
#include "toolbox.h"
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

// Make the result row holding the entry `kind`/`id` current.
bool selectEntry(pictura::SearchBar* searchBar, pictura::SearchEntry::Kind kind, const QString& id)
{
    for (int row = 0; searchBar && row < searchBar->results()->count(); ++row) {
        const pictura::SearchEntry* e = searchBar->entryAt(row);
        if (e && e->kind == kind && e->id == id) {
            searchBar->results()->setCurrentRow(row);
            return true;
        }
    }
    return false;
}

// Whether a reveal outline (`flashWidget`) sits over `target`.
bool flashedOver(QWidget* target)
{
    if (!target) {
        return false;
    }
    QWidget* window = target->window();
    const QRect want =
        QRect(target->mapTo(window, QPoint(0, 0)), target->size()).adjusted(-2, -2, 2, 2);
    const QList<QWidget*> flashes = window->findChildren<QWidget*>(QStringLiteral("searchFlash"));
    return std::any_of(flashes.cbegin(), flashes.cend(),
                       [&want](const QWidget* flash) { return flash->geometry() == want; });
}

bool insideNamed(QWidget* widget, const QString& name)
{
    for (; widget; widget = widget->parentWidget()) {
        if (widget->objectName() == name) {
            return true;
        }
    }
    return false;
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
    void enterRunsPanelPreferenceControl();
    void stubIsNotRunnable();
    void altEnterRevealsMenu();
    void altEnterOutlinesInPlace();
    void upDownMovesAndClamps();
    void cleanLabelsAndWhitespace();
    void dismissal();
    void focusLossCloses();
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
    qDeleteAll(window_->findChildren<QWidget*>(QStringLiteral("searchFlash")));
    if (pictura::PreferencesDialog* prefs = window_->preferencesDialog()) {
        prefs->close();
    }
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

void SearchBarTest::enterRunsPanelPreferenceControl()
{
    using Kind = pictura::SearchEntry::Kind;
    QVERIFY(openTransformableDocument());
    pictura::CommandRegistry* registry = window_->registry();

    const QString layers = QStringLiteral("layersPanel");
    QAction* layersToggle =
        registry->action(QString::fromLatin1(pictura::command_ids::WindowPanelsLayers));
    QVERIFY(layersToggle);
    registry->refresh();
    if (layersToggle->isChecked()) {
        layersToggle->trigger();
    }
    registry->refresh();
    QVERIFY2(!layersToggle->isChecked(), "the Layers panel starts hidden");
    pictura::SearchBar* searchBar = openBar(QStringLiteral("layers"));
    QVERIFY(selectEntry(searchBar, Kind::Panel, layers));
    QTest::keyClick(searchBar->input(), Qt::Key_Return);
    QVERIFY(!searchBar->isVisible());
    registry->refresh();
    QVERIFY2(layersToggle->isChecked(), "Enter shows the panel");
    QVERIFY2(flashedOver(window_->findChild<QWidget*>(layers)), "the shown panel is outlined");

    searchBar = openBar(QStringLiteral("general"));
    QVERIFY(selectEntry(searchBar, Kind::Preference, pictura::PreferencesDialog::kGeneral));
    QTest::keyClick(searchBar->input(), Qt::Key_Return);
    pictura::PreferencesDialog* prefs = window_->preferencesDialog();
    QVERIFY2(prefs && prefs->isVisible(), "Enter opens Preferences");
    QCOMPARE(prefs->currentPageForTest(), pictura::PreferencesDialog::kGeneral);
    prefs->close();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));

    // A drop-down control opens its list.
    auto* blend = window_->findChild<QComboBox*>(QStringLiteral("layersBlendCombo"));
    QVERIFY(blend);
    searchBar = openBar(QStringLiteral("blend mode"));
    QVERIFY(selectEntry(searchBar, Kind::Control, QStringLiteral("layersBlendCombo")));
    QTest::keyClick(searchBar->input(), Qt::Key_Return);
    QTRY_VERIFY2(blend->view()->isVisible(), "Enter opens the Blend Mode list");
    QVERIFY(flashedOver(blend));
    closePopups();

    // An options-bar control activates its tool, and its number field takes the focus.
    searchBar = openBar(QStringLiteral("flow"));
    QVERIFY(selectEntry(searchBar, Kind::Control, QStringLiteral("optionsBrushFlow")));
    QTest::keyClick(searchBar->input(), Qt::Key_Return);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Brush);
    auto* field = qobject_cast<QLineEdit*>(QApplication::focusWidget());
    QVERIFY2(field && insideNamed(field, QStringLiteral("optionsBrushFlow")),
             "the Flow field has the focus");
    QCOMPARE(field->selectedText(), field->text());
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

    // Alt+Enter still opens the chain. QMenu does not activate a disabled
    // action, so the stub item itself is not highlighted.
    QAction* zoomify = window_->registry()->action(searchBar->entryAt(0)->command);
    QVERIFY(zoomify);
    QMenu* exportMenu = owningMenu(zoomify);
    QVERIFY(exportMenu);
    QMenu* file = owningMenu(exportMenu->menuAction());
    QVERIFY(file);
    QTest::keyClick(searchBar->input(), Qt::Key_Return, Qt::AltModifier);
    QVERIFY(!searchBar->isVisible());
    QTRY_VERIFY(file->isVisible());
    QTRY_VERIFY(exportMenu->isVisible());
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

void SearchBarTest::altEnterOutlinesInPlace()
{
    using Kind = pictura::SearchEntry::Kind;
    QVERIFY(openTransformableDocument());

    pictura::SearchBar* searchBar = openBar(QStringLiteral("magic wand"));
    QVERIFY(selectEntry(searchBar, Kind::Tool, pictura::toolIdName(pictura::ToolId::MagicWand)));
    QTest::keyClick(searchBar->input(), Qt::Key_Return, Qt::AltModifier);
    QVERIFY(!searchBar->isVisible());
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
    auto* toolbox = window_->findChild<pictura::Toolbox*>();
    QVERIFY(toolbox);
    QVERIFY2(flashedOver(toolbox->slotButton(pictura::toolInfo(pictura::ToolId::MagicWand).group)),
             "the toolbox slot is outlined");

    searchBar = openBar(QStringLiteral("layers"));
    QVERIFY(selectEntry(searchBar, Kind::Panel, QStringLiteral("layersPanel")));
    QTest::keyClick(searchBar->input(), Qt::Key_Return, Qt::AltModifier);
    QVERIFY2(flashedOver(window_->findChild<QWidget*>(QStringLiteral("layersPanel"))),
             "the panel is outlined");

    auto* blend = window_->findChild<QComboBox*>(QStringLiteral("layersBlendCombo"));
    QVERIFY(blend);
    searchBar = openBar(QStringLiteral("blend mode"));
    QVERIFY(selectEntry(searchBar, Kind::Control, QStringLiteral("layersBlendCombo")));
    QTest::keyClick(searchBar->input(), Qt::Key_Return, Qt::AltModifier);
    QVERIFY2(flashedOver(blend), "the control is outlined");
    QVERIFY2(!blend->view()->isVisible(), "reveal does not open the list");
}

void SearchBarTest::upDownMovesAndClamps()
{
    pictura::SearchBar* searchBar = openBar(QStringLiteral("layer"));
    QVERIFY(searchBar);
    const int count = searchBar->results()->count();
    QVERIFY(count > 2);
    QCOMPARE(searchBar->results()->currentRow(), 0);
    QTest::keyClick(searchBar->input(), Qt::Key_Up);
    QCOMPARE(searchBar->results()->currentRow(), 0);
    QTest::keyClick(searchBar->input(), Qt::Key_Down);
    QCOMPARE(searchBar->results()->currentRow(), 1);
    QTest::keyClick(searchBar->input(), Qt::Key_Up);
    QCOMPARE(searchBar->results()->currentRow(), 0);
    for (int i = 0; i <= count; ++i) {
        QTest::keyClick(searchBar->input(), Qt::Key_Down);
    }
    QCOMPARE(searchBar->results()->currentRow(), count - 1);
    QVERIFY(searchBar->isVisible());
}

void SearchBarTest::cleanLabelsAndWhitespace()
{
    QCOMPARE(pictura::cleanSearchLabel(QStringLiteral("&File")), QStringLiteral("File"));
    QCOMPARE(pictura::cleanSearchLabel(QStringLiteral("R&&D")), QStringLiteral("R&D"));
    QCOMPARE(pictura::cleanSearchLabel(QStringLiteral("Fish & Chips")),
             QStringLiteral("Fish & Chips"));
    QCOMPARE(pictura::cleanSearchLabel(QStringLiteral("Liquify\u2026\u2026")),
             QStringLiteral("Liquify"));
    QCOMPARE(pictura::cleanSearchLabel(QStringLiteral("Save &As...\tCtrl+Shift+S")),
             QStringLiteral("Save As"));

    const QList<pictura::SearchEntry> entries = index();
    const QList<pictura::SearchHit> hits =
        pictura::searchIndex(entries, QStringLiteral("transform\twarp\n"));
    QVERIFY(!hits.isEmpty());
    QCOMPARE(entries.at(hits.first().index).id, kWarp);
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

void SearchBarTest::focusLossCloses()
{
    pictura::SearchBar* searchBar = openBar();
    QVERIFY(searchBar && searchBar->isVisible());
    QFocusEvent toPopup(QEvent::FocusOut, Qt::PopupFocusReason);
    QApplication::sendEvent(searchBar->input(), &toPopup);
    QVERIFY2(searchBar->isVisible(), "focus lost to a popup keeps the bar");

    auto* field = new QLineEdit(window_->statusBar());
    field->show();
    field->setFocus(Qt::OtherFocusReason);
    QTRY_VERIFY2(!searchBar->isVisible(), "focus lost to another widget closes the bar");
    delete field;
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
