// A pending transform locks layer edits and guards tool switches and layer
// changes (#750, #905): while a modal Free Transform / Transform submenu
// session is uncommitted, every Layer menu command is disabled (and refuses
// dispatch), the Layers panel's editing controls are disabled while its layer
// list stays selectable, and a tool switch or a layer click asks "Apply the
// transformation?": Apply commits, Don't Apply cancels, Cancel stays.

#include <QtTest/QtTest>

#include "api/layers_api.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "selftest_paint_fixture.h"
#include "toolbox.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipboard.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

#include <QtCore/QCoreApplication>
#include <QtGui/QAction>
#include <QtWidgets/QTreeView>

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(100, 80, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

// Drag the box from inside (off the centred reference point) by (5, 3): an
// uncommitted, non-identity move.
bool nudge(pictura::PictureView* view)
{
    if (view->transform_press(15.0, 12.0, 1.0, false, false) != 9) {
        return false;
    }
    const bool moved = view->transform_move(20.0, 15.0, 1.0, false, false);
    view->transform_release();
    return moved;
}

QAction* action(pictura::PicturaMainWindow& frame, const char* id)
{
    frame.registry()->refresh();
    return frame.registry()->action(QString::fromLatin1(id));
}

} // namespace

class TransformLockTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void layerMenuIsLockedWhilePending();
    void panelEditingControlsLockedWhilePending();
    void layerClickAppliesThePendingTransform();
    void layerClickDontApplyCancels();
    void layerClickCancelKeepsTheTransform();
    void documentEditsDuringPendingTransform();
    void maskFillIsRefusedWhilePending();
    void applyCommitsAndSwitches();
    void dontApplyCancelsAndSwitches();
    void cancelKeepsTheTransform();
    void toolShortcutAsksToo();
    void noPromptWithoutAPendingTransform();

private:
    QString beginLayer(Fixture& fixture);
    // A document layer path other than `exclude`, or empty when there is none.
    QString otherLayerPath(Fixture& fixture, const QString& exclude);
    // Answer the next prompts with `decision`, counting how often it is asked.
    void answer(Fixture& fixture, pictura::TransformDecision decision);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    int asked_ = 0;
};

void TransformLockTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // The layer-click tests read row geometry, which needs a laid-out panel.
    window_->show();
    QCoreApplication::processEvents();
}

void TransformLockTest::cleanup()
{
    if (window_) {
        if (auto* tools = window_->findChild<pictura::ToolController*>()) {
            tools->setTransformDecisionPrompt(
                [this]() { return pictura::askApplyTransform(window_.get()); });
            tools->cancelFreeTransform();
        }
        window_->setActiveTool(pictura::ToolId::Move);
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    asked_ = 0;
}

QString TransformLockTest::beginLayer(Fixture& fixture)
{
    pictura::PictureView* view = fixture.view;
    const int added = view->add_layer(0);
    if (added <= 0) {
        return QString();
    }
    const QString layer = QString::number(added);
    fixture.frame.selectLayerPath(layer);
    if (!view->select_rect(10, 8, 30, 22, QStringLiteral("new"), 0.0)
        || !pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false)) {
        return QString();
    }
    view->deselect();
    return layer;
}

QString TransformLockTest::otherLayerPath(Fixture& fixture, const QString& exclude)
{
    const QVector<pictura::layers_api::LayerRow> rows = pictura::layers_api::rows(*fixture.view);
    for (const pictura::layers_api::LayerRow& row : rows) {
        if (!row.path.isEmpty() && row.path != exclude) {
            return row.path;
        }
    }
    return QString();
}

void TransformLockTest::answer(Fixture& fixture, pictura::TransformDecision decision)
{
    fixture.tools->setTransformDecisionPrompt([this, decision]() {
        ++asked_;
        return decision;
    });
}

void TransformLockTest::layerMenuIsLockedWhilePending()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_menu"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    const char* ids[] = {pictura::command_ids::LayerNewLayer,
                         pictura::command_ids::LayerDeleteLayer,
                         pictura::command_ids::LayerHideLayers,
                         pictura::command_ids::LayerArrangeBackward};
    QVERIFY(action(frame, pictura::command_ids::LayerNewLayer)->isEnabled());
    QVERIFY(f.view->select_rect(10, 8, 10, 10, QStringLiteral("new"), 0.0));
    QVERIFY(action(frame, pictura::command_ids::EditFill)->isEnabled());

    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(frame.layersLockedForTest());
    for (const char* id : ids) {
        QAction* a = action(frame, id);
        QVERIFY2(a && !a->isEnabled(), id);
    }
    QVERIFY(!action(frame, pictura::command_ids::EditFill)->isEnabled());
    QVERIFY(action(frame, pictura::command_ids::EditTransformPerspective)->isEnabled());
    const int layers = f.view->layer_count();
    QVERIFY(!frame.registry()->dispatch(QString::fromLatin1(pictura::command_ids::LayerNewLayer)));
    QCOMPARE(f.view->layer_count(), layers);
    // Other menus are unaffected.
    QVERIFY(action(frame, pictura::command_ids::EditTransformPerspective) != nullptr);

    f.tools->commitFreeTransform();
    QVERIFY(!frame.layersLockedForTest());
    QVERIFY(action(frame, pictura::command_ids::LayerNewLayer)->isEnabled());
}

void TransformLockTest::panelEditingControlsLockedWhilePending()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_panel"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    pictura::LayersPanel* panel = frame.layersPanelForTest();
    QVERIFY(panel != nullptr);
    auto* tree = panel->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);
    QVERIFY(panel->isEnabled());
    QVERIFY(panel->maskButtonEnabledForTest());
    QVERIFY(panel->deleteButtonEnabledForTest());
    QVERIFY(panel->lockToggleEnabledForTest(0));

    QVERIFY(f.tools->beginTransformMode(layer, QStringLiteral("perspective")));
    // The panel and its layer list stay interactive; the editing controls lock.
    QVERIFY(panel->isEnabled());
    QVERIFY(tree->isEnabled());
    QVERIFY(!panel->maskButtonEnabledForTest());
    QVERIFY(!panel->deleteButtonEnabledForTest());
    QVERIFY(!panel->lockToggleEnabledForTest(0));

    f.tools->cancelFreeTransform();
    QVERIFY(panel->isEnabled());
    QVERIFY(panel->maskButtonEnabledForTest());
    QVERIFY(panel->deleteButtonEnabledForTest());
    QVERIFY(panel->lockToggleEnabledForTest(0));
}

void TransformLockTest::layerClickAppliesThePendingTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_click_apply"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    const QString other = otherLayerPath(f, layer);
    QVERIFY(!other.isEmpty());
    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(nudge(f.view));
    answer(f, pictura::TransformDecision::Apply);
    const int base = f.view->history_index();

    pictura::LayersPanel* panel = frame.layersPanelForTest();
    QVERIFY(panel != nullptr);
    QVERIFY(panel->clickRowForTest(other));
    QCoreApplication::processEvents();

    QCOMPARE(asked_, 1);
    QVERIFY(!f.view->transform_session_active());
    QVERIFY(f.committedOnce(base, "Free Transform"));
    QCOMPARE(f.view->active_layer_path(), other);
    QVERIFY(!frame.layersLockedForTest());
}

void TransformLockTest::layerClickDontApplyCancels()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_click_dont"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    const QString other = otherLayerPath(f, layer);
    QVERIFY(!other.isEmpty());
    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(nudge(f.view));
    answer(f, pictura::TransformDecision::DontApply);
    const int base = f.view->history_index();

    pictura::LayersPanel* panel = frame.layersPanelForTest();
    QVERIFY(panel != nullptr);
    QVERIFY(panel->clickRowForTest(other));
    QCoreApplication::processEvents();

    QCOMPARE(asked_, 1);
    QVERIFY(!f.view->transform_session_active());
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(f.view->active_layer_path(), other);
}

void TransformLockTest::layerClickCancelKeepsTheTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_click_cancel"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    const QString other = otherLayerPath(f, layer);
    QVERIFY(!other.isEmpty());
    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(nudge(f.view));
    answer(f, pictura::TransformDecision::Cancel);

    pictura::LayersPanel* panel = frame.layersPanelForTest();
    QVERIFY(panel != nullptr);
    QVERIFY(panel->clickRowForTest(other));
    QCoreApplication::processEvents();

    QCOMPARE(asked_, 1);
    QVERIFY(f.view->transform_session_active());
    QCOMPARE(f.view->active_layer_path(), layer);
    QVERIFY(frame.layersLockedForTest());
    QVERIFY(!panel->maskButtonEnabledForTest());
}

void TransformLockTest::documentEditsDuringPendingTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_edits"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::clipboard_copy(*f.view, layer, false));
    QVERIFY(f.view->apply_filter(QStringLiteral("add-noise")));
    QVERIFY(f.view->select_rect(10, 8, 30, 22, QStringLiteral("new"), 0.0));
    QVERIFY(action(frame, pictura::command_ids::ImageRotate180)->isEnabled());
    QVERIFY(action(frame, pictura::command_ids::FilterLastFilter)->isEnabled());
    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(nudge(f.view));
    const int base = f.view->history_index();
    const auto pendingPixel = f.view->sample_argb(20, 20);

    QVERIFY(!action(frame, pictura::command_ids::EditFill)->isEnabled());
    QVERIFY(!action(frame, pictura::command_ids::EditStroke)->isEnabled());
    QVERIFY(!action(frame, pictura::command_ids::EditAssignProfile)->isEnabled());
    QVERIFY(!action(frame, pictura::command_ids::EditConvertProfile)->isEnabled());
    QVERIFY(!action(frame, pictura::command_ids::ImageRotate180)->isEnabled());
    QVERIFY(!action(frame, pictura::command_ids::FilterLastFilter)->isEnabled());
    QVERIFY(!frame.registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditFill)));
    QVERIFY(!pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY(!pictura::edit_stroke(*f.view, qRgba(0, 0, 0, 255), 2, 1,
                                  QStringLiteral("normal"), 100));
    QVERIFY(!f.view->assign_profile(0));
    QVERIFY(!f.view->convert_profile(0));

    QVERIFY(!pictura::clipboard_cut(*f.view, layer));
    QVERIFY(!pictura::clipboard_clear(*f.view, layer));
    QVERIFY(pictura::clipboard_paste(*f.view, layer, pictura::PasteKind::Plain, 0.0, 0.0)
                .isEmpty());
    QVERIFY(!f.view->apply_filter(QStringLiteral("add-noise")));
    QVERIFY(!f.view->add_adjustment(QStringLiteral("invert")));
    QVERIFY(!f.view->resize_image(QStringLiteral("bicubic"), 10, 10));
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(f.view->sample_argb(20, 20), pendingPixel);
    QVERIFY(f.view->transform_session_active());

    f.tools->commitFreeTransform();
    QVERIFY(f.committedOnce(base, "Free Transform"));
    QVERIFY(action(frame, pictura::command_ids::EditFill)->isEnabled());
    frame.selectLayerPath(layer);
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY(f.view->apply_filter(QStringLiteral("add-noise")));

    // The Move tool's per-gesture controls box is not a modal lock.
    frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY(f.tools->beginControlsGestureForTest(f.view));
    QVERIFY(f.tools->transformControlsSession());
    QVERIFY(action(frame, pictura::command_ids::EditFill)->isEnabled());
    QVERIFY(action(frame, pictura::command_ids::FilterLastFilter)->isEnabled());
    QVERIFY(pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY(f.tools->transformControlsSession());
    f.tools->cancelFreeTransform();
}

void TransformLockTest::maskFillIsRefusedWhilePending()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_mask"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*f.view, layer, QStringLiteral("reveal-all")));
    QVERIFY(pictura::mask_edit_target_set(*f.view, QString()));
    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(nudge(f.view));
    QVERIFY(pictura::mask_edit_target_set(*f.view, layer));
    const int base = f.view->history_index();

    QVERIFY(!pictura::edit_fill(*f.view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100,
                               false));
    QCOMPARE(f.view->history_index(), base);

    QVERIFY(pictura::mask_edit_target_set(*f.view, QString()));
    f.tools->cancelFreeTransform();
}

void TransformLockTest::applyCommitsAndSwitches()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_apply"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(nudge(f.view));
    answer(f, pictura::TransformDecision::Apply);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::Brush);
    QCOMPARE(asked_, 1);
    QCOMPARE(frame.activeTool(), pictura::ToolId::Brush);
    QVERIFY(!f.view->transform_session_active());
    QVERIFY(f.committedOnce(base, "Free Transform"));
    QVERIFY(!frame.layersLockedForTest());
}

void TransformLockTest::dontApplyCancelsAndSwitches()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_dont"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    QVERIFY(f.tools->beginFreeTransform(layer));
    QVERIFY(nudge(f.view));
    answer(f, pictura::TransformDecision::DontApply);
    const int base = f.view->history_index();

    frame.setActiveTool(pictura::ToolId::Gradient);
    QCOMPARE(asked_, 1);
    QCOMPARE(frame.activeTool(), pictura::ToolId::Gradient);
    QVERIFY(!f.view->transform_session_active());
    QCOMPARE(f.view->history_index(), base);
}

void TransformLockTest::cancelKeepsTheTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_cancel"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    const pictura::ToolId before = frame.activeTool();
    QVERIFY(f.tools->beginFreeTransform(layer));
    answer(f, pictura::TransformDecision::Cancel);
    QSignalSpy announced(f.tools, &pictura::ToolController::activeToolChanged);

    frame.setActiveTool(pictura::ToolId::Brush);
    QCOMPARE(asked_, 1);
    QCOMPARE(frame.activeTool(), before);
    QVERIFY(f.view->transform_session_active());
    QVERIFY(frame.layersLockedForTest());
    // The tool strip is told to drop the click (re-announcing the old tool).
    QCOMPARE(announced.count(), 1);
}

void TransformLockTest::toolShortcutAsksToo()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_shortcut"));
    QVERIFY(f.ok());
    const QString layer = beginLayer(f);
    QVERIFY(!layer.isEmpty());
    auto* toolbox = frame.findChild<pictura::Toolbox*>();
    QVERIFY(toolbox != nullptr);
    QVERIFY(f.tools->beginFreeTransform(layer));
    answer(f, pictura::TransformDecision::Cancel);

    QVERIFY(toolbox->handleToolKey(QChar(u'B'), false));
    QCOMPARE(asked_, 1);
    QVERIFY(f.view->transform_session_active());

    answer(f, pictura::TransformDecision::DontApply);
    QVERIFY(toolbox->handleToolKey(QChar(u'B'), false));
    QCOMPARE(asked_, 2);
    QVERIFY(!f.view->transform_session_active());
    QCOMPARE(frame.activeTool(), pictura::ToolId::Brush);
}

void TransformLockTest::noPromptWithoutAPendingTransform()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_lock_idle"));
    QVERIFY(f.ok());
    QVERIFY(!beginLayer(f).isEmpty());
    answer(f, pictura::TransformDecision::Cancel);
    frame.setActiveTool(pictura::ToolId::Brush);
    QCOMPARE(asked_, 0);
    QCOMPARE(frame.activeTool(), pictura::ToolId::Brush);
    QVERIFY(!frame.layersLockedForTest());
}

QTEST_MAIN(TransformLockTest)
#include "tst_transform_lock.moc"
