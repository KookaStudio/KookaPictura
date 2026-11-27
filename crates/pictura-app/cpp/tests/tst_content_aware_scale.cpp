#include <QtTest/QtTest>

#include "api/content_aware_test_api.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/selection_bridge.h"
#include "api/transform_api.h"
#include "commands.h"
#include "frame.h"
#include "panels/numeric_field.h"
#include "panels/layers_panel.h"
#include "panels/channels_panel.h"
#include "qt_test_support.h"
#include "tools.h"

#include <QtCore/QCoreApplication>
#include <QtGui/QAction>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QToolButton>
#include <limits>

class ContentAwareScaleTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void sessionGeometry();
    void refusesTargets();
    void selectionLimitsTarget();
    void exclusiveSession();
    void protectControls();
    void protectOffsetLayer();
    void supersededPreview();
    void cancelRestoresState();
    void commitUndoRedo();

private:
    QString beginLayer();
    bool beginScale();
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ContentAwareScaleTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
    window_->show();
    QCoreApplication::processEvents();
}

void ContentAwareScaleTest::cleanup()
{
    if (auto* view = window_->activeView())
        pictura::content_aware_test::holdJobs(*view, false);
    if (auto* tools = window_->findChild<pictura::ToolController*>()) {
        tools->setTransformDecisionPrompt(nullptr);
        if (tools->transformPending())
            tools->cancelFreeTransform();
    }
    while (window_->activeDocumentIndex() >= 0)
        window_->closeDocument(window_->activeDocumentIndex(), false);
}

QString ContentAwareScaleTest::beginLayer()
{
    if (!window_->newDocument(QStringLiteral("Scale"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")))
        return {};
    auto* view = window_->activeView();
    if (auto* tools = window_->findChild<pictura::ToolController*>())
        tools->setActiveTool(pictura::ToolId::Move);
    const int added = view->add_layer(0);
    if (added <= 0)
        return {};
    const QString path = QString::number(added);
    window_->selectLayerPath(path);
    if (!view->select_rect(10, 8, 12, 10, QStringLiteral("new"), 0.0)
        || !pictura::edit_fill(*view, qRgba(0, 0, 0, 255), -1, QStringLiteral("normal"), 100, false))
        return {};
    view->deselect();
    return path;
}

bool ContentAwareScaleTest::beginScale()
{
    window_->registry()->refresh();
    auto* action = window_->registry()->action(QStringLiteral("edit.seam.aware.scale"));
    if (!action || !action->isEnabled()) return false;
    action->trigger();
    auto* tools = window_->findChild<pictura::ToolController*>();
    return tools && tools->transformPending();
}

void ContentAwareScaleTest::sessionGeometry()
{
    QVERIFY(!beginLayer().isEmpty());
    QVERIFY(beginScale());
    auto* tools = window_->findChild<pictura::ToolController*>();
    auto* view = window_->activeView();
    const auto before = pictura::transform_api::quad(*view).corners.boundingRect();
    QVERIFY(tools->setTransformPivotPreset(0));
    QVERIFY(tools->setTransformWidth(150.0, true));
    const auto after = pictura::transform_api::quad(*view).corners.boundingRect();
    QVERIFY(!tools->setTransformWidth(std::numeric_limits<double>::quiet_NaN(), false));
    QCOMPARE(pictura::transform_api::quad(*view).corners.boundingRect(), after);
    QVERIFY(qAbs(after.width() - before.width() * 1.5) < 0.01);
    QVERIFY(qAbs(after.height() - before.height() * 1.5) < 0.01);
    QVERIFY(qAbs(after.left() - before.left()) < 0.01);
    QVERIFY(qAbs(after.top() - before.top()) < 0.01);
    auto* commit = window_->findChild<QToolButton*>(QStringLiteral("optionsContentAwareCommit"));
    QVERIFY(commit);
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    const QImage preview = pictura::content_aware_test::previewImage(*view);
    QCOMPARE(preview.width(), qRound(after.width()));
    QCOMPARE(preview.height(), qRound(after.height()));
    const auto corner = pictura::transform_api::quad(*view).corners.at(1);
    QCOMPARE(view->transform_press(corner.x(), corner.y(), 1.0, false, false), 1);
    QVERIFY(view->transform_move(corner.x() + 3.0, corner.y(), 1.0, false, false));
    view->transform_release();
    QVERIFY(pictura::transform_api::quad(*view).corners.boundingRect().width() > after.width());
}

void ContentAwareScaleTest::refusesTargets()
{
    const QString layer = beginLayer();
    QVERIFY(!layer.isEmpty());
    auto* view = window_->activeView();
    auto* action = window_->registry()->action(QStringLiteral("edit.seam.aware.scale"));
    QVERIFY(action);
    auto* channels = window_->findChild<pictura::ChannelsPanel*>();
    QVERIFY(channels);
    channels->selectChannelForTest(1);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    channels->selectChannelForTest(0);
    auto* panel = window_->findChild<pictura::LayersPanel*>();
    QVERIFY(panel);
    panel->selectPaths({QStringLiteral("0"), layer}, layer);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    panel->selectPaths({layer}, layer);
    QVERIFY(view->background_from_layer(QStringLiteral("0")));
    window_->selectLayerPath(QStringLiteral("0"));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    window_->selectLayerPath(layer);
    QVERIFY(view->set_layer_lock(layer.toInt(), QStringLiteral("all"), true));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(view->set_layer_lock(layer.toInt(), QStringLiteral("all"), false));
    QVERIFY(pictura::vector_mask_add(*view, QStringLiteral("reveal-all")));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(pictura::vector_mask_set_linked(*view, false));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(pictura::vector_mask_delete(*view));
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-all")));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(pictura::layer_mask_set_linked_path(*view, layer, false));
    const QImage before = view->image();
    const int history = view->history_count();
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    action->trigger();
    QCOMPARE(view->image(), before);
    QCOMPARE(view->history_count(), history);
    QVERIFY(!window_->findChild<pictura::ToolController*>()->transformPending());
    QVERIFY(pictura::layer_mask_delete_path(*view, layer));
    QVERIFY(view->convert_to_smart_object(layer));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(view->undo());
    const int group = view->add_group(999);
    QVERIFY(group >= 0);
    window_->selectLayerPath(QString::number(group));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    window_->closeDocument(window_->activeDocumentIndex(), false);
    for (int depth : {16, 32}) {
        QVERIFY(window_->newDocument(QStringLiteral("Depth"), 40, 30, QStringLiteral("rgb"), 8,
                                     QStringLiteral("white")));
        view = window_->activeView();
        QVERIFY(pictura::convert_image_depth(*view, depth));
        QCOMPARE(view->document_depth_bits(), depth);
        const QString path = QString::number(view->add_layer(0));
        window_->selectLayerPath(path);
        QVERIFY(view->select_rect(1, 1, 8, 8, QStringLiteral("new"), 0.0));
        const QImage pixels = view->image();
        const int selected = view->selection_count();
        const int count = view->history_count();
        window_->registry()->refresh();
        QVERIFY(!action->isEnabled());
        action->trigger();
        QCOMPARE(view->image(), pixels);
        QCOMPARE(view->selection_count(), selected);
        QCOMPARE(view->history_count(), count);
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    QVERIFY(!beginLayer().isEmpty());
    view = window_->activeView();
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("cmyk")));
    const QString newLayer = QString::number(view->add_layer(0));
    window_->selectLayerPath(newLayer);
    QVERIFY(view->select_rect(5, 5, 8, 8, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(10, 20, 30, 255), -1,
                               QStringLiteral("normal"), 100, false));
    view->deselect();
    QVERIFY(view->layer_can_free_transform(newLayer));
    const QImage convertedPixels = view->image();
    const int convertedHistory = view->history_count();
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    action->trigger();
    QCOMPARE(view->image(), convertedPixels);
    QCOMPARE(view->history_count(), convertedHistory);
}

void ContentAwareScaleTest::selectionLimitsTarget()
{
    QVERIFY(!beginLayer().isEmpty());
    auto* view = window_->activeView();
    QVERIFY(view->select_rect(10, 8, 8, 10, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1,
                               QStringLiteral("normal"), 100, false));
    QVERIFY(view->select_rect(10, 8, 8, 10, QStringLiteral("new"), 1.0));
    const QImage original = view->image();
    const int coverage = view->selection_coverage(10, 10);
    const int history = view->history_count();
    QVERIFY(coverage > 0 && coverage < 255);
    QVERIFY(beginScale());
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools->setTransformPivotPreset(0));
    QVERIFY(tools->setTransformWidth(50.0, false));
    QVERIFY(tools->setTransformPosition(17.0, 8.0));
    auto* commit = window_->findChild<QToolButton*>(QStringLiteral("optionsContentAwareCommit"));
    QVERIFY(commit);
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    commit->click();
    QTRY_COMPARE_WITH_TIMEOUT(view->history_count(), history + 1, 10000);
    const QImage result = view->image();
    QVERIFY(result != original);
    QCOMPARE(result.pixelColor(21, 10), original.pixelColor(21, 10));
    QVERIFY(result.pixelColor(10, 10).green() > 0);
    QVERIFY(result.pixelColor(10, 10).green() < 255);
    QVERIFY(result.pixelColor(19, 10).red() > original.pixelColor(19, 10).red());
    QCOMPARE(view->selection_count(), 0);
    QVERIFY(view->undo());
    QCOMPARE(view->image(), original);
    QCOMPARE(view->selection_coverage(10, 10), coverage);
    QVERIFY(view->redo());
    QCOMPARE(view->image(), result);
    QCOMPARE(view->selection_count(), 0);
}

void ContentAwareScaleTest::exclusiveSession()
{
    const QString layer = beginLayer();
    QVERIFY(!layer.isEmpty());
    auto* view = window_->activeView();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(beginScale());
    QVERIFY(tools->setTransformWidth(75.0, false));
    const int history = view->history_count();
    const int selection = view->selection_count();
    QVERIFY(!view->select_rect(0, 0, 2, 2, QStringLiteral("new"), 0.0));
    view->select_all();
    QCOMPARE(view->selection_count(), selection);
    QVERIFY(!view->undo());
    QVERIFY(!view->redo());
    QVERIFY(!tools->beginTransformMode(layer, QStringLiteral("skew")));
    QVERIFY(!pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1,
                                QStringLiteral("normal"), 100, false));
    tools->setTransformDecisionPrompt([] { return pictura::TransformDecision::Cancel; });
    const auto previous = tools->activeTool();
    tools->setActiveTool(pictura::ToolId::Brush);
    QCOMPARE(tools->activeTool(), previous);
    QVERIFY(tools->transformPending());
    QCOMPARE(view->history_count(), history);
    tools->setTransformDecisionPrompt([] { return pictura::TransformDecision::DontApply; });
    tools->setActiveTool(pictura::ToolId::Brush);
    QVERIFY(!tools->transformPending());
    QCOMPARE(view->history_count(), history);
    QVERIFY(beginScale());
    QVERIFY(tools->setTransformWidth(75.0, false));
    auto* commit = window_->findChild<QToolButton*>(QStringLiteral("optionsContentAwareCommit"));
    QVERIFY(commit);
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    tools->setTransformDecisionPrompt([] { return pictura::TransformDecision::Apply; });
    tools->setActiveTool(pictura::ToolId::Move);
    QTRY_VERIFY_WITH_TIMEOUT(!tools->transformPending(), 10000);
    QCOMPARE(view->history_count(), history + 1);

    auto* panel = window_->findChild<pictura::LayersPanel*>();
    QVERIFY(panel);
    const auto clickOther = [&]() {
        QVERIFY(panel->clickRowForTest(QStringLiteral("0")));
        QCoreApplication::processEvents();
    };
    QVERIFY(beginScale());
    QVERIFY(tools->setTransformWidth(75.0, false));
    tools->setTransformDecisionPrompt([] { return pictura::TransformDecision::Cancel; });
    clickOther();
    QVERIFY(tools->transformPending());
    QVERIFY(panel->selectedPaths().contains(layer));
    tools->setTransformDecisionPrompt([] { return pictura::TransformDecision::DontApply; });
    clickOther();
    QVERIFY(!tools->transformPending());
    QVERIFY(panel->selectedPaths().contains(QStringLiteral("0")));
    panel->selectPaths({layer}, layer);
    QVERIFY(beginScale());
    QVERIFY(tools->setTransformWidth(75.0, false));
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    tools->setTransformDecisionPrompt([] { return pictura::TransformDecision::Apply; });
    clickOther();
    QVERIFY(!tools->transformPending());
    QVERIFY(panel->selectedPaths().contains(QStringLiteral("0")));
    QCOMPARE(view->history_count(), history + 2);
}

void ContentAwareScaleTest::protectControls()
{
    QVERIFY(!beginLayer().isEmpty());
    auto* view = window_->activeView();
    QVERIFY(view->select_rect(10, 8, 4, 4, QStringLiteral("new"), 0.0));
    QVERIFY(view->save_selection(QStringLiteral("Subject")));
    view->deselect();
    QVERIFY(beginScale());
    auto* amount = window_->findChild<pictura::NumericField*>(QStringLiteral("optionsContentAwareAmount"));
    auto* protect = window_->findChild<QComboBox*>(QStringLiteral("optionsContentAwareProtect"));
    QVERIFY(amount && protect);
    QCOMPARE(amount->value(), 100.0);
    QCOMPARE(protect->currentText(), QStringLiteral("None"));
    QVERIFY(protect->findText(QStringLiteral("Subject")) >= 0);
}

void ContentAwareScaleTest::protectOffsetLayer()
{
    QVERIFY(!beginLayer().isEmpty());
    auto* view = window_->activeView();
    QVERIFY(view->select_rect(15, 8, 1, 10, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1,
                               QStringLiteral("normal"), 100, false));
    QVERIFY(view->select_rect(10, 8, 12, 10, QStringLiteral("new"), 0.0));
    QVERIFY(view->select_rect(15, 8, 1, 10, QStringLiteral("subtract"), 0.0));
    QVERIFY(view->save_selection(QStringLiteral("Offset protection")));
    view->deselect();
    QVERIFY(beginScale());
    auto* tools = window_->findChild<pictura::ToolController*>();
    auto* protect = window_->findChild<QComboBox*>(QStringLiteral("optionsContentAwareProtect"));
    auto* commit = window_->findChild<QToolButton*>(QStringLiteral("optionsContentAwareCommit"));
    QVERIFY(protect && commit);
    QVERIFY(tools->setTransformPivotPreset(0));
    QVERIFY(tools->setTransformWidth(91.6666667, false));
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    const QImage plain = pictura::content_aware_test::previewImage(*view);
    protect->setCurrentIndex(protect->findText(QStringLiteral("Offset protection")));
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    const QImage protectedImage = pictura::content_aware_test::previewImage(*view);
    QCOMPARE(protectedImage.size(), plain.size());
    QVERIFY(protectedImage != plain);
    commit->click();
    QVERIFY(pictura::load_selection_target(*view, QStringLiteral("alpha:0"), false, QStringLiteral("new")));
    QCOMPARE(view->selection_coverage(10, 8), 255);
    QCOMPARE(view->selection_coverage(15, 8), 0);
    QCOMPARE(view->selection_coverage(22, 8), 0);
}

void ContentAwareScaleTest::supersededPreview()
{
    QVERIFY(!beginLayer().isEmpty());
    auto* view = window_->activeView();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(beginScale());
    auto* commit = window_->findChild<QToolButton*>(QStringLiteral("optionsContentAwareCommit"));
    QVERIFY(commit);
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    pictura::content_aware_test::holdJobs(*view, true);
    QVERIFY(tools->setTransformWidth(50.0, false));
    const quint64 older = pictura::content_aware_test::currentGeneration(*view);
    QTRY_VERIFY_WITH_TIMEOUT(pictura::content_aware_test::jobStarted(*view, older), 10000);
    QVERIFY(tools->setTransformWidth(75.0, false));
    const quint64 current = pictura::content_aware_test::currentGeneration(*view);
    QVERIFY(current != older);
    QVERIFY(!pictura::content_aware_test::jobStarted(*view, current));
    pictura::content_aware_test::releaseJob(*view, older);
    QTRY_VERIFY_WITH_TIMEOUT(pictura::content_aware_test::jobFinished(*view, older), 10000);
    QTRY_VERIFY_WITH_TIMEOUT(pictura::content_aware_test::jobStarted(*view, current), 10000);
    QVERIFY(!commit->isEnabled());
    pictura::content_aware_test::releaseJob(*view, current);
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    const QImage newest = pictura::content_aware_test::previewImage(*view);
    QVERIFY(!newest.isNull());
    QCOMPARE(pictura::content_aware_test::previewImage(*view), newest);
    const int history = view->history_count();
    commit->click();
    QTRY_COMPARE_WITH_TIMEOUT(view->history_count(), history + 1, 10000);
    QVERIFY(!tools->transformPending());
    const QImage committed = view->image();
    pictura::content_aware_test::holdJobs(*view, false);
    QVERIFY(beginScale());
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    pictura::content_aware_test::holdJobs(*view, true);
    QVERIFY(tools->setTransformWidth(50.0, false));
    const quint64 cancelled = pictura::content_aware_test::currentGeneration(*view);
    QTRY_VERIFY_WITH_TIMEOUT(pictura::content_aware_test::jobStarted(*view, cancelled), 10000);
    tools->cancelFreeTransform();
    pictura::content_aware_test::releaseJob(*view, cancelled);
    QTRY_VERIFY_WITH_TIMEOUT(pictura::content_aware_test::jobFinished(*view, cancelled), 10000);
    QVERIFY(!tools->transformPending());
    QCOMPARE(view->image(), committed);
    QCOMPARE(view->history_count(), history + 1);
}

void ContentAwareScaleTest::cancelRestoresState()
{
    const QString layer = beginLayer();
    QVERIFY(!layer.isEmpty());
    auto* view = window_->activeView();
    QVERIFY(view->select_rect(10, 8, 8, 10, QStringLiteral("new"), 1.0));
    const int coverage = view->selection_coverage(10, 10);
    QVERIFY(coverage > 0);
    const QImage original = view->image();
    const int history = view->history_count();
    const auto originalRect = pictura::transform_api::layerRect(*view, layer).toString();
    QVERIFY(beginScale());
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools->setTransformWidth(75.0, false));
    tools->cancelFreeTransform();
    QCOMPARE(view->image(), original);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->selection_coverage(10, 10), coverage);
    QCOMPARE(pictura::transform_api::layerRect(*view, layer).toString(), originalRect);
}

void ContentAwareScaleTest::commitUndoRedo()
{
    QVERIFY(!beginLayer().isEmpty());
    auto* view = window_->activeView();
    const QImage original = view->image();
    const int history = view->history_count();
    QVERIFY(beginScale());
    auto* tools = window_->findChild<pictura::ToolController*>();
    auto* commit = window_->findChild<QToolButton*>(QStringLiteral("optionsContentAwareCommit"));
    QVERIFY(commit);
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    commit->click();
    QVERIFY(!tools->transformPending());
    QCOMPARE(view->image(), original);
    QCOMPARE(view->history_count(), history);
    QVERIFY(beginScale());
    QVERIFY(tools->setTransformWidth(75.0, false));
    QTRY_VERIFY_WITH_TIMEOUT(commit->isEnabled(), 10000);
    commit->click();
    QTRY_COMPARE_WITH_TIMEOUT(view->history_count(), history + 1, 10000);
    const QImage committed = view->image();
    QVERIFY(committed != original);
    QVERIFY(view->undo());
    QCOMPARE(view->image(), original);
    QVERIFY(view->redo());
    QCOMPARE(view->image(), committed);
}

PICTURA_QT_TEST_MAIN(ContentAwareScaleTest)
#include "tst_content_aware_scale.moc"
