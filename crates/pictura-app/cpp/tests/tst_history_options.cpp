#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"

#include "api/history_api.h"
#include "api/paint_bridge.h"
#include "commands.h"
#include "command_ids.h"
#include "panels/history_panel.h"
#include "panels/panel_group.h"
#include "session.h"

#include "qt_test_support.h"

#include <functional>

#include <QtCore/QCoreApplication>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>

class HistoryOptionsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();

    void optionsDialogShowsCurrentValues();
    void optionsPersistAcrossRestart();
    void clearKeepsSnapshots();
    void altClearDropsSnapshots();
    void altClickClearHistoryDropsSnapshots();
    void newDocumentFromState();
    void newDocumentFromSnapshot();
    void newDocumentLeavesSourceUnchanged();
    void snapshotButtonAutomatic();
    void nonLinearAppend();
    void nonLinearUndoOrder();
    void visibilityUndoOff();
    void visibilityUndoOn();
    void autoFirstSnapshotOnAdd();
    void autoSaveSnapshot();
    void invalidEditLeavesHistoryUnchanged();
    void jumpBackAcrossStrokesRefreshesTheDisplay();

private:
    pictura::PictureView* newDocument(int width = 24, int height = 24);
    void setOptions(bool autoFirst, bool autoSave, bool nonLinear, bool showDialog,
                    bool visibilityUndoable);
    void openOptionsDialog(const std::function<void(QDialog*)>& inspect);
    pictura::HistoryPanel* panel() const;
    QListWidget* historyList() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir files_;
};

void HistoryOptionsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void HistoryOptionsTest::init()
{
    pictura::SessionState state = pictura::loadSession();
    state.historyAutoFirstSnapshot = true;
    state.historyAutoSnapshotOnSave = false;
    state.historyAllowNonLinear = false;
    state.historyShowSnapshotDialog = false;
    state.historyVisibilityUndoable = false;
    pictura::saveSession(state);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void HistoryOptionsTest::cleanup()
{
    while (window_ && window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
    window_.reset();
}

pictura::HistoryPanel* HistoryOptionsTest::panel() const
{
    return window_ ? window_->findChild<pictura::HistoryPanel*>(QStringLiteral("historyPanel"))
                   : nullptr;
}

QListWidget* HistoryOptionsTest::historyList() const
{
    pictura::HistoryPanel* history = panel();
    return history ? history->findChild<QListWidget*>() : nullptr;
}

pictura::PictureView* HistoryOptionsTest::newDocument(int width, int height)
{
    if (!window_->newDocument(QStringLiteral("HistoryOptions"), width, height,
                              QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

void HistoryOptionsTest::setOptions(bool autoFirst, bool autoSave, bool nonLinear,
                                    bool showDialog, bool visibilityUndoable)
{
    pictura::HistoryPanel* history = panel();
    QVERIFY(history);
    history->setOptionsForTest(autoFirst, autoSave, nonLinear, showDialog, visibilityUndoable);
}

void HistoryOptionsTest::openOptionsDialog(const std::function<void(QDialog*)>& inspect)
{
    pictura::HistoryPanel* history = panel();
    QVERIFY(history);
    bool opened = false;
    QTimer::singleShot(0, [&opened, inspect] {
        auto* dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        opened = dialog != nullptr;
        if (dialog) {
            inspect(dialog);
            dialog->accept();
        }
    });
    history->openHistoryOptions();
    QVERIFY(opened);
}

void HistoryOptionsTest::optionsDialogShowsCurrentValues()
{
    setOptions(true, true, true, true, true);
    bool first = false;
    bool save = false;
    bool nonlinear = false;
    bool prompt = false;
    bool visibility = false;
    openOptionsDialog([&](QDialog* dialog) {
        first = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionFirstSnapshot"))
                    ->isChecked();
        save = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionAutoSave"))->isChecked();
        nonlinear = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionNonLinear"))
                        ->isChecked();
        prompt = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionSnapshotDialog"))
                     ->isChecked();
        visibility = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionVisibilityUndo"))
                         ->isChecked();
    });
    QVERIFY(first);
    QVERIFY(save);
    QVERIFY(nonlinear);
    QVERIFY(prompt);
    QVERIFY(visibility);
}

void HistoryOptionsTest::optionsPersistAcrossRestart()
{
    setOptions(true, false, false, false, false);
    openOptionsDialog([](QDialog* dialog) {
        dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionFirstSnapshot"))->setChecked(false);
        dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionAutoSave"))->setChecked(true);
        dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionNonLinear"))->setChecked(true);
        dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionSnapshotDialog"))->setChecked(false);
        dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionVisibilityUndo"))->setChecked(true);
    });
    const pictura::SessionState stored = pictura::loadSession();
    QVERIFY(!stored.historyAutoFirstSnapshot);
    QVERIFY(stored.historyAutoSnapshotOnSave);
    QVERIFY(stored.historyAllowNonLinear);
    QVERIFY(!stored.historyShowSnapshotDialog);
    QVERIFY(stored.historyVisibilityUndoable);

    pictura::HistoryPanel reloaded;
    bool first = true;
    bool save = false;
    bool nonlinear = false;
    bool prompt = true;
    bool visibility = false;
    bool opened = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        opened = dialog != nullptr;
        if (!dialog) {
            return;
        }
        first = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionFirstSnapshot"))->isChecked();
        save = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionAutoSave"))->isChecked();
        nonlinear = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionNonLinear"))->isChecked();
        prompt = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionSnapshotDialog"))->isChecked();
        visibility = dialog->findChild<QCheckBox*>(QStringLiteral("historyOptionVisibilityUndo"))->isChecked();
        dialog->accept();
    });
    reloaded.openHistoryOptions();
    QVERIFY(opened);
    QVERIFY(!first);
    QVERIFY(save);
    QVERIFY(nonlinear);
    QVERIFY(!prompt);
    QVERIFY(visibility);
}

void HistoryOptionsTest::clearKeepsSnapshots()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("Keep")));
    QVERIFY(pictura::history_api::count(*view) > 1);

    panel()->clearHistory(false);
    QCOMPARE(pictura::history_api::count(*view), 1);
    QCOMPARE(pictura::history_api::snapshotCount(*view), 1);
    QVERIFY(!pictura::history_api::canUndo(*view));
}

void HistoryOptionsTest::altClearDropsSnapshots()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("Gone")));

    panel()->performPanelMenuAction(QStringLiteral("clearHistoryPurge"));
    QCOMPARE(pictura::history_api::count(*view), 1);
    QCOMPARE(pictura::history_api::snapshotCount(*view), 0);
    QVERIFY(!pictura::history_api::canUndo(*view));
}

void HistoryOptionsTest::newDocumentFromState()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument(20, 20);
    QVERIFY(view);
    view->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0);
    QVERIFY(pictura::edit_fill(*view, qRgba(220, 20, 20, 255), -1, QStringLiteral("normal"), 100,
                               false));
    const quint32 red = view->sample_argb(8, 8);
    const int redState = pictura::history_api::index(*view);
    QVERIFY(pictura::edit_fill(*view, qRgba(20, 20, 220, 255), -1, QStringLiteral("normal"), 100,
                               false));
    const quint32 blue = view->sample_argb(8, 8);
    QVERIFY(red != blue);

    QCoreApplication::processEvents();
    panel()->refresh();
    QListWidget* list = historyList();
    QVERIFY(list);
    list->setCurrentRow(redState);
    window_->registry()->refresh();
    const int before = window_->documentCount();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::PanelHistoryNewDocument)));
    QCOMPARE(window_->documentCount(), before + 1);
    pictura::PictureView* created = window_->activeView();
    QVERIFY(created);
    QCOMPARE(pictura::history_api::count(*created), 1);
    QCOMPARE(pictura::history_api::label(*created, 0), QStringLiteral("Duplicate State"));
    QCOMPARE(created->sample_argb(8, 8), red);
    QCOMPARE(view->sample_argb(8, 8), blue);
}

void HistoryOptionsTest::newDocumentFromSnapshot()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument(20, 20);
    QVERIFY(view);
    view->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0);
    QVERIFY(pictura::edit_fill(*view, qRgba(220, 20, 20, 255), -1, QStringLiteral("normal"), 100,
                               false));
    const quint32 red = view->sample_argb(8, 8);
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("Red")));
    QVERIFY(pictura::edit_fill(*view, qRgba(20, 20, 220, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY(view->sample_argb(8, 8) != red);

    QCoreApplication::processEvents();
    panel()->refresh();
    QListWidget* list = historyList();
    QVERIFY(list);
    QCOMPARE(list->item(0)->data(Qt::UserRole).toInt(), 1);
    list->setCurrentRow(0);
    window_->registry()->refresh();
    const int before = window_->documentCount();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::PanelHistoryNewDocument)));
    QCOMPARE(window_->documentCount(), before + 1);
    pictura::PictureView* created = window_->activeView();
    QVERIFY(created);
    QCOMPARE(pictura::history_api::count(*created), 1);
    QCOMPARE(pictura::history_api::label(*created, 0), QStringLiteral("Duplicate State"));
    QCOMPARE(created->sample_argb(8, 8), red);
}

void HistoryOptionsTest::newDocumentLeavesSourceUnchanged()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument(16, 16);
    QVERIFY(view);
    QVERIFY(view->add_layer(-1) >= 0);
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("Source")));
    const int states = pictura::history_api::count(*view);
    const int index = pictura::history_api::index(*view);
    const int snapshots = pictura::history_api::snapshotCount(*view);
    const quint32 pixel = view->sample_argb(3, 3);

    QCoreApplication::processEvents();
    panel()->refresh();
    QListWidget* list = historyList();
    QVERIFY(list);
    list->setCurrentRow(0);
    window_->registry()->refresh();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::PanelHistoryNewDocument)));

    QCOMPARE(pictura::history_api::count(*view), states);
    QCOMPARE(pictura::history_api::index(*view), index);
    QCOMPARE(pictura::history_api::snapshotCount(*view), snapshots);
    QCOMPARE(view->sample_argb(3, 3), pixel);
}

void HistoryOptionsTest::snapshotButtonAutomatic()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    auto* button = panel()->findChild<QPushButton*>(QStringLiteral("snapshotButton"));
    QVERIFY(button);
    QCOMPARE(pictura::history_api::snapshotCount(*view), 0);
    button->click();
    QCOMPARE(pictura::history_api::snapshotCount(*view), 1);
    QCOMPARE(pictura::history_api::snapshotLabel(*view, 0), QStringLiteral("Snapshot 1"));
}

void HistoryOptionsTest::nonLinearAppend()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument(8, 8);
    QVERIFY(view);
    for (int i = 0; i < 3; ++i) {
        QVERIFY(view->add_layer(-1) >= 0);
    }
    QCOMPARE(pictura::history_api::count(*view), 4);
    QVERIFY(pictura::history_api::undo(*view));
    QVERIFY(pictura::history_api::undo(*view));
    QCOMPARE(pictura::history_api::index(*view), 1);

    setOptions(false, false, true, false, false);
    QVERIFY(view->add_layer(-1) >= 0);
    QCOMPARE(pictura::history_api::count(*view), 5);
    QCOMPARE(pictura::history_api::index(*view), 4);
    QCOMPARE(view->layer_count(), 3);
}

void HistoryOptionsTest::nonLinearUndoOrder()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument(8, 8);
    QVERIFY(view);
    for (int i = 0; i < 3; ++i) {
        QVERIFY(view->add_layer(-1) >= 0);
    }
    QVERIFY(pictura::history_api::undo(*view));
    QVERIFY(pictura::history_api::undo(*view));
    QCOMPARE(view->layer_count(), 2);

    setOptions(false, false, true, false, false);
    QVERIFY(view->add_layer(-1) >= 0);
    QCOMPARE(view->layer_count(), 3);
    QVERIFY(pictura::history_api::undo(*view));
    QCOMPARE(view->layer_count(), 4);
    QVERIFY(pictura::history_api::undo(*view));
    QCOMPARE(view->layer_count(), 3);
}

void HistoryOptionsTest::visibilityUndoOff()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument(8, 8);
    QVERIFY(view);
    QVERIFY(view->add_layer(-1) >= 0);
    const int top = view->layer_count() - 1;
    const int states = pictura::history_api::count(*view);
    view->set_layer_visible(top, false);
    QVERIFY(!view->layer_visible(top));
    QCOMPARE(pictura::history_api::count(*view), states);
}

void HistoryOptionsTest::visibilityUndoOn()
{
    setOptions(false, false, false, false, true);
    pictura::PictureView* view = newDocument(8, 8);
    QVERIFY(view);
    QVERIFY(view->add_layer(-1) >= 0);
    const int top = view->layer_count() - 1;
    const int states = pictura::history_api::count(*view);
    view->set_layer_visible(top, false);
    QVERIFY(!view->layer_visible(top));
    QCOMPARE(pictura::history_api::count(*view), states + 1);
    QVERIFY(pictura::history_api::undo(*view));
    QVERIFY(view->layer_visible(top));
}

void HistoryOptionsTest::autoFirstSnapshotOnAdd()
{
    setOptions(true, false, false, false, false);
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    QCOMPARE(pictura::history_api::snapshotCount(*view), 1);
    QCOMPARE(pictura::history_api::snapshotLabel(*view, 0), QStringLiteral("Snapshot 1"));
    window_->closeDocument(window_->activeDocumentIndex(), false);

    setOptions(false, false, false, false, false);
    view = newDocument();
    QVERIFY(view);
    QCOMPARE(pictura::history_api::snapshotCount(*view), 0);
}

void HistoryOptionsTest::autoSaveSnapshot()
{
    setOptions(false, true, false, false, false);
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    QCOMPARE(pictura::history_api::snapshotCount(*view), 0);
    const QString path = files_.filePath(QStringLiteral("auto-save.psd"));
    QVERIFY(window_->saveActiveAs(path));
    QCOMPARE(pictura::history_api::snapshotCount(*view), 1);
    QCOMPARE(pictura::history_api::snapshotLabel(*view, 0), QStringLiteral("Snapshot 1"));
}

void HistoryOptionsTest::invalidEditLeavesHistoryUnchanged()
{
    pictura::PictureView* view = newDocument(8, 8);
    QVERIFY(view);
    const int states = pictura::history_api::count(*view);
    QVERIFY(!view->set_layer_blend(99, QStringLiteral("norm")));
    QCOMPARE(pictura::history_api::count(*view), states);
}



void HistoryOptionsTest::jumpBackAcrossStrokesRefreshesTheDisplay()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument(200, 200);
    QVERIFY(view);
    const QList<QPoint> points = {QPoint(20, 20), QPoint(60, 60), QPoint(100, 100),
                                  QPoint(140, 140)};
    for (int i = 0; i < 3; ++i) {
        const QPoint point = points.at(i);
        QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 20, 100, 100, 0, 100, 100, 25,
                                  QStringLiteral("normal"), false, false));
        QVERIFY(view->paint_dab(point.x(), point.y(), 1.0));
        QVERIFY(view->end_paint());
    }
    QVERIFY(pictura::history_api::jump(*view, 1));
    for (int i = 1; i < 3; ++i) {
        QCOMPARE(view->sample_argb(points.at(i).x(), points.at(i).y()), 0xFFFFFFFFu);
    }

    // A non-linear branch capture after the jump appends a full anchor. The
    // display must still rebuild every stroke region the jump crosses.
    setOptions(false, false, true, false, false);
    const QPoint last = points.last();
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 20, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(last.x(), last.y(), 1.0));
    QVERIFY(view->end_paint());
    QCOMPARE(pictura::history_api::count(*view), 5);
    QCOMPARE(view->sample_argb(last.x(), last.y()), 0xFFFF0000u);

    QVERIFY(pictura::history_api::jump(*view, 1));
    QCOMPARE(view->sample_argb(points.first().x(), points.first().y()), 0xFFFF0000u);
    for (int i = 1; i < points.size(); ++i) {
        QCOMPARE(view->sample_argb(points.at(i).x(), points.at(i).y()), 0xFFFFFFFFu);
    }
    QCoreApplication::processEvents();
    const QImage shown = window_->imageView()->image();
    QVERIFY(!shown.isNull());
    QCOMPARE(shown.pixelColor(last.x(), last.y()), QColor(Qt::white));
}


void HistoryOptionsTest::altClickClearHistoryDropsSnapshots()
{
    setOptions(false, false, false, false, false);
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("Gone")));
    auto* column = window_->columnForPanel(QStringLiteral("historyPanel"));
    QVERIFY(column);
    auto* group = column->groupForPanel(QStringLiteral("historyPanel"));
    QVERIFY(group);
    group->setLastMenuModifiersForTest(Qt::AltModifier);
    window_->registry()->refresh();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::PanelHistoryClear)));
    QCOMPARE(pictura::history_api::snapshotCount(*view), 0);
    QCOMPARE(pictura::history_api::count(*view), 1);
}

PICTURA_QT_TEST_MAIN(HistoryOptionsTest)
#include "tst_history_options.moc"
