// One presentation per committed edit: the transaction notifies once, the
// command handler does not refresh again, and the command table sees the
// history state that was just committed.

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtWidgets/QApplication>

#include "api/type_api.h"
#include "commands.h"
#include "frame.h"
#include "panels/channels_panel.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_ops.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"
#include "qt_test_support.h"

namespace {

QString typeLayerPath(const pictura::PictureView& view)
{
    for (int i = 0; i < view.layer_count(); ++i) {
        const QString path = QString::number(i);
        if (pictura::type_api::isType(view, path)) {
            return path;
        }
    }
    return QString();
}

} // namespace

class EditRefreshTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void aCommandRefreshesOnce();
    void aCommitNotifiesOnce();
    void aSaveSelectionNotifies();
    void aCommittedEditShowsUndoState();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void EditRefreshTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1000, 700);
}

void EditRefreshTest::aCommandRefreshesOnce()
{
    QVERIFY(window_->newDocument(QStringLiteral("Refresh Count"), 64, 64,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    panel->selectPaths({layer}, layer);

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    const int before = registry->refreshCount();
    QVERIFY2(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerFlattenImage)),
             "flatten image dispatch");
    const int after = registry->refreshCount();
    QCOMPARE(after - before, 1);
}

void EditRefreshTest::aCommitNotifiesOnce()
{
    pictura::PictureView view;
    QVERIFY(view.new_document(64, 64, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    QSignalSpy changes(&view, &pictura::PictureView::changed);

    QVERIFY(!view.duplicate_layers(QStringList{QStringLiteral("0")}).isEmpty());
    QCOMPARE(changes.count(), 1);

    QVERIFY(view.new_document(24, 24, QStringLiteral("rgb"), 8, QStringLiteral("transparent")));
    QVERIFY(view.select_rect(3, 2, 5, 4, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(view, QColor(200, 0, 0).rgba(), -1, QStringLiteral("normal"),
                               100, false));
    view.deselect();
    changes.clear();
    QVERIFY(pictura::trim_image(view));
    QCOMPARE(changes.count(), 1);

    changes.clear();
    QVERIFY(pictura::canvas_size_apply(view, QStringLiteral("center"), 10, 10, 0));
    QCOMPARE(changes.count(), 1);

    QVERIFY(view.new_document(200, 200, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    changes.clear();
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = 30.0;
    const QString font = QStringLiteral("Liberation Sans");
    QVERIFY(pictura::type_api::createCharacterStyle(view, QStringLiteral("Probe"), font, character));
    QCOMPARE(changes.count(), 1);

    const pictura::TypeSetting placement = pictura::type_api::layerSetting(view, QString());
    const pictura::ParagraphSetting paragraph = pictura::type_api::defaultParagraphSetting();
    QVERIFY(!pictura::type_api::commitLayer(view, QStringLiteral("Hello"), font, placement, character,
                                            paragraph)
                 .isEmpty());
    const QString path = typeLayerPath(view);
    QVERIFY2(!path.isEmpty(), "a type layer");
    changes.clear();
    QVERIFY(pictura::type_api::applyStyle(view, path, QStringLiteral("Probe"), false));
    QCOMPARE(changes.count(), 1);
}

void EditRefreshTest::aSaveSelectionNotifies()
{
    QVERIFY(window_->newDocument(QStringLiteral("Save Selection"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    view->select_all();

    auto* panel = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel != nullptr);
    panel->setView(view);
    panel->refresh();
    const int channelsBefore = panel->channelCountForTest();
    const int historyBefore = view->history_index();

    QSignalSpy changes(view, &pictura::PictureView::changed);
    QVERIFY(view->save_selection(QStringLiteral("Alpha 1")));
    QCOMPARE(changes.count(), 1);
    QCOMPARE(view->history_index(), historyBefore + 1);
    QCOMPARE(view->selection_channel_count(), 1);
    QTRY_COMPARE_WITH_TIMEOUT(panel->channelCountForTest(), channelsBefore + 1, 1000);
}

void EditRefreshTest::aCommittedEditShowsUndoState()
{
    QVERIFY(window_->newDocument(QStringLiteral("Undo Label"), 64, 64,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    panel->selectPaths({layer}, layer);

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerDuplicateLayer)));

    QAction* undo = registry->action(QString::fromLatin1(pictura::command_ids::EditUndo));
    QVERIFY(undo != nullptr);
    QVERIFY(undo->isEnabled());
    QVERIFY2(undo->text().contains(QStringLiteral("Duplicate")),
             qPrintable(undo->text()));
}

QTEST_MAIN(EditRefreshTest)
#include "tst_edit_refresh.moc"
