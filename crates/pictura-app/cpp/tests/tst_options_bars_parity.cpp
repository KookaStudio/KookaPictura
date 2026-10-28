// Options-bars parity (#252): the tool-icon separator on every page, the Move
// bar's Auto-Select / Show Transform Controls / three-dots Align To, and the
// Eyedropper's Sample Size / Sample scope / Show Sampling Ring. Ctrl's
// temporary Auto-Select flip (#981).

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "panels/layers_panel.h"
#include "panels/numeric_field.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/eyedropper_sample.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QAction>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

class OptionsBarsParityTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void toolIconSeparatorOnEveryPage();
    void moveControlsAndAlignToCanvas();
    void moveAutoSelectPicksTheTopmostLayer();
    void moveCtrlClickAutoSelects();
    void moveCtrlClickSuppressesAutoSelect();
    void moveCtrlClickFollowsGroupMode();
    void moveCtrlShowsFlippedAutoSelect();
    void eraserBarMatchesTheBrush();
    void pencilAutoEraseDoesNotLeakToBrush();
    void mixerBarMatchesCc();
    void eyedropperSampleSize();
    void eyedropperSampleScope();
    void eyedropperSamplingRing();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    // The layer's rect offset as (left, top).
    QPoint offset(pictura::PictureView* view, const QString& path) const
    {
        const QStringList parts = pictura::transform_api::layerRect(*view, path).toString().split(QLatin1Char(' '));
        return parts.size() == 4 ? QPoint(parts[0].toInt(), parts[1].toInt()) : QPoint(-999, -999);
    }

    // A new document whose Background carries a 6x6 copy over (8, 5), with the
    // Move tool active and Auto-Select set to `on` with the `target` dropdown
    // (0 Group, 1 Layer). Returns the copy's path.
    QString ctrlFixture(const QString& name, bool on, int target)
    {
        if (!window_->newDocument(name, 40, 40, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
            return {};
        }
        pictura::PictureView* view = window_->activeView();
        view->select_rect(8, 5, 6, 6, QStringLiteral("new"), 0.0);
        const QString copy = view->layer_via_copy(QStringLiteral("0"));
        view->deselect();
        window_->setActiveTool(pictura::ToolId::Move);
        auto* autoSelectOn =
            window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveAutoSelectOn"));
        auto* autoSelect = window_->findChild<QComboBox*>(QStringLiteral("optionsMoveAutoSelect"));
        if (!autoSelectOn || !autoSelect) {
            return {};
        }
        autoSelect->setCurrentIndex(target);
        autoSelectOn->setChecked(on);
        return copy;
    }

    void ctrlClick(const QPointF& pos)
    {
        window_->imageView()->mousePressed(pos, Qt::LeftButton, int(Qt::ControlModifier));
        window_->imageView()->mouseReleased(pos);
    }

    static QAction* menuAction(QToolButton* menuButton, const QString& objectName)
    {
        for (QAction* action : menuButton->menu()->actions()) {
            if (action->objectName() == objectName) {
                return action;
            }
        }
        return nullptr;
    }
};

void OptionsBarsParityTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void OptionsBarsParityTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void OptionsBarsParityTest::toolIconSeparatorOnEveryPage()
{
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    for (pictura::ToolId id : pictura::allToolIds()) {
        QWidget* page = bar->pageForTest(id);
        QVERIFY(page);
        auto* layout = qobject_cast<QHBoxLayout*>(page->layout());
        QVERIFY(layout);
        auto* icon = qobject_cast<QToolButton*>(layout->itemAt(0)->widget());
        QVERIFY(icon);
        auto* separator = qobject_cast<QFrame*>(layout->itemAt(1)->widget());
        QVERIFY(separator);
        QCOMPARE(separator->objectName(), QStringLiteral("optionsToolSeparator"));
        QCOMPARE(separator->frameShape(), QFrame::VLine);
    }
}

void OptionsBarsParityTest::moveControlsAndAlignToCanvas()
{
    // Show Transform Controls starts checked (owner decision); turn it off
    // before any Move box opens, so the align below sees the untrimmed,
    // document-sized copy.
    auto* startTransform = window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(startTransform && startTransform->isChecked());
    startTransform->setChecked(false);
    QVERIFY(window_->newDocument(QStringLiteral("Move"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(view && panel && tools);

    view->select_rect(8, 5, 6, 6, QStringLiteral("new"), 0.0);
    const QString layer = view->layer_via_copy(QStringLiteral("0"));
    QVERIFY(!layer.isEmpty());
    view->deselect();
    QCoreApplication::processEvents();
    // The Layers model refreshes on a 120 ms debounce; wait for the copy's row
    // before selecting it.
    QTRY_VERIFY(panel->selectRowForTest(layer));
    window_->setActiveTool(pictura::ToolId::Move);

    auto* autoSelect = window_->findChild<QComboBox*>(QStringLiteral("optionsMoveAutoSelect"));
    auto* autoSelectOn =
        window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveAutoSelectOn"));
    auto* transform = window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    auto* menuButton = window_->findChild<QToolButton*>(QStringLiteral("optionsMoveMenu"));
    QVERIFY(autoSelect && autoSelectOn && transform && menuButton && menuButton->menu());
    QCOMPARE(autoSelect->count(), 2);
    QVERIFY(!autoSelectOn->isChecked());
    QVERIFY(!autoSelect->isEnabled());
    QVERIFY2(!menuButton->icon().isNull(), "the menu button uses the ellipsis icon");
    QVERIFY(menuButton->text().isEmpty());
    QVERIFY(menuAction(menuButton, QStringLiteral("moveMenuAlignLeft")));
    QVERIFY(menuAction(menuButton, QStringLiteral("moveMenuDistributeTop")));
    QVERIFY(menuAction(menuButton, QStringLiteral("moveMenuAlignToCanvas")));

    autoSelectOn->setChecked(true);
    QVERIFY(autoSelect->isEnabled());
    QVERIFY(tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move)->get().autoSelectOn);
    autoSelect->setCurrentIndex(1);
    QCOMPARE(tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move)->get().autoSelect, 1);
    autoSelectOn->setChecked(false);
    QVERIFY(!autoSelect->isEnabled());
    QVERIFY(!tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move)->get().autoSelectOn);
    // Align To Canvas: the square's content left (8) meets the canvas left (0),
    // so the document-sized copy shifts by -8. (Before the controls box is
    // checked: showing it trims the layer to its content, like Free Transform.)
    menuAction(menuButton, QStringLiteral("moveMenuAlignToCanvas"))->trigger();
    QCOMPARE(tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move)->get().alignTo, 1);
    menuAction(menuButton, QStringLiteral("moveMenuAlignLeft"))->trigger();
    QCOMPARE(offset(view, layer), QPoint(-8, 0));

    // Show Transform Controls records the option; tst_transform_controls
    // covers the box behavior.
    transform->setChecked(true);
    QVERIFY(tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move)->get().showTransformControls);
    transform->setChecked(false);
}

void OptionsBarsParityTest::moveAutoSelectPicksTheTopmostLayer()
{
    QVERIFY(window_->newDocument(QStringLiteral("AutoSelect"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* canvas = window_->imageView();
    QVERIFY(view && panel && canvas);

    // A 6x6 copy of the background over (8, 5): two stacked opaque layers now
    // cover that point, and the copy is the topmost one.
    view->select_rect(8, 5, 6, 6, QStringLiteral("new"), 0.0);
    const QString copy = view->layer_via_copy(QStringLiteral("0"));
    QVERIFY(!copy.isEmpty());
    view->deselect();
    QTRY_VERIFY(panel->selectRowForTest(copy));
    view->set_active_layer(QStringLiteral("0"));

    window_->setActiveTool(pictura::ToolId::Move);
    auto* autoSelectOn =
        window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveAutoSelectOn"));
    auto* autoSelect = window_->findChild<QComboBox*>(QStringLiteral("optionsMoveAutoSelect"));
    QVERIFY(autoSelectOn && autoSelect);
    autoSelect->setCurrentIndex(1); // Layer
    autoSelectOn->setChecked(true);

    // A press on the copy's pixels activates it.
    canvas->mousePressed(QPointF(10, 8), Qt::LeftButton, 0);
    canvas->mouseReleased(QPointF(10, 8));
    QCOMPARE(view->active_layer_path(), copy);

    // A press on the uncovered background falls back to the Background layer.
    canvas->mousePressed(QPointF(30, 30), Qt::LeftButton, 0);
    canvas->mouseReleased(QPointF(30, 30));
    QCOMPARE(view->active_layer_path(), QStringLiteral("0"));

    // With Auto-Select off the active layer is left alone.
    view->set_active_layer(QStringLiteral("0"));
    autoSelectOn->setChecked(false);
    canvas->mousePressed(QPointF(10, 8), Qt::LeftButton, 0);
    canvas->mouseReleased(QPointF(10, 8));
    QCOMPARE(view->active_layer_path(), QStringLiteral("0"));
}

// mca_ctrl_click_selects
void OptionsBarsParityTest::moveCtrlClickAutoSelects()
{
    const QString copy = ctrlFixture(QStringLiteral("CtrlSelect"), false, 1);
    QVERIFY(!copy.isEmpty());
    pictura::PictureView* view = window_->activeView();
    // An empty layer on top: (10, 8) is transparent there, so the press falls
    // through to the copy's opaque pixel.
    const int top = view->add_layer(-1);
    QVERIFY(top >= 0);
    const QString empty = QString::number(top);
    view->set_active_layer(empty);
    QCOMPARE(view->active_layer_path(), empty);

    ctrlClick(QPointF(10, 8));
    QCOMPARE(view->active_layer_path(), copy);
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(!tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move)->get().autoSelectOn);
}

// mca_ctrl_disables
void OptionsBarsParityTest::moveCtrlClickSuppressesAutoSelect()
{
    const QString copy = ctrlFixture(QStringLiteral("CtrlSuppress"), true, 1);
    QVERIFY(!copy.isEmpty());
    pictura::PictureView* view = window_->activeView();
    view->set_active_layer(QStringLiteral("0"));

    ctrlClick(QPointF(10, 8));
    QCOMPARE(view->active_layer_path(), QStringLiteral("0"));
    // Without Ctrl the same press auto-selects the copy.
    window_->imageView()->mousePressed(QPointF(10, 8), Qt::LeftButton, 0);
    window_->imageView()->mouseReleased(QPointF(10, 8));
    QCOMPARE(view->active_layer_path(), copy);
    window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveAutoSelectOn"))->setChecked(false);
}

// mca_ctrl_follows_mode
void OptionsBarsParityTest::moveCtrlClickFollowsGroupMode()
{
    const QString copy = ctrlFixture(QStringLiteral("CtrlGroup"), false, 0);
    QVERIFY(!copy.isEmpty());
    pictura::PictureView* view = window_->activeView();
    const QString group = view->group_layers(QStringList{copy});
    QVERIFY(!group.isEmpty());
    view->set_active_layer(QStringLiteral("0"));

    ctrlClick(QPointF(10, 8));
    QCOMPARE(view->active_layer_path(), group);
}

// mca_checkbox_shows_flip
void OptionsBarsParityTest::moveCtrlShowsFlippedAutoSelect()
{
    QVERIFY(!ctrlFixture(QStringLiteral("CtrlShow"), false, 1).isEmpty());
    auto* canvas = window_->imageView();
    auto* tools = window_->findChild<pictura::ToolController*>();
    auto* autoSelectOn =
        window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveAutoSelectOn"));
    auto* autoSelect = window_->findChild<QComboBox*>(QStringLiteral("optionsMoveAutoSelect"));
    auto* move = tools->optionsFor<pictura::MoveGroup>(pictura::ToolId::Move);
    QVERIFY(!autoSelectOn->isChecked() && !autoSelect->isEnabled());

    emit canvas->modifierKeyChanged(int(Qt::ControlModifier));
    QVERIFY(autoSelectOn->isChecked());
    QVERIFY(autoSelect->isEnabled());
    QVERIFY(!move->get().autoSelectOn);

    emit canvas->modifierKeyChanged(int(Qt::NoModifier));
    QVERIFY(!autoSelectOn->isChecked());
    QVERIFY(!autoSelect->isEnabled());
    QVERIFY(!move->get().autoSelectOn);

    // A click on the box with Ctrl held still turns the stored option on; the
    // box then shows it flipped off until Ctrl is released.
    emit canvas->modifierKeyChanged(int(Qt::ControlModifier));
    autoSelectOn->setChecked(false);
    QVERIFY(move->get().autoSelectOn);
    emit canvas->modifierKeyChanged(int(Qt::NoModifier));
    QVERIFY(autoSelectOn->isChecked());
    autoSelectOn->setChecked(false);
    QVERIFY(!move->get().autoSelectOn);
}

void OptionsBarsParityTest::pencilAutoEraseDoesNotLeakToBrush()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    seed.setPixel(16, 16, qRgb(255, 0, 0));
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_auto_erase"));
    QVERIFY(f.ok());
    paint_fixture::BrushState brush(f.tools);

    auto* pencil = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Pencil);
    QVERIFY(pencil);
    pencil->setAutoErase(true);
    f.tools->setForeground(QColor(255, 0, 0));
    f.tools->setBackground(QColor(0, 0, 255));
    f.tools->setActiveTool(pictura::ToolId::Brush);
    const int base = f.view->history_count();

    f.drag({QPointF(16, 16), QPointF(26, 16)});

    QCOMPARE(f.view->sample_argb(16, 16), 0xFFFF0000u);
    QCOMPARE(f.view->sample_argb(26, 16), 0xFFFF0000u);
    QCOMPARE(f.view->history_count(), base + 1);
    QCOMPARE(f.view->history_label(f.view->history_index()), QStringLiteral("Brush"));
    pencil->setAutoErase(false);
}

void OptionsBarsParityTest::mixerBarMatchesCc()
{
    QVERIFY(window_->newDocument(QStringLiteral("Mixer"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    window_->setActiveTool(pictura::ToolId::MixerBrush);
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(bar && tools);
    QWidget* page = bar->pageForTest(pictura::ToolId::MixerBrush);
    QVERIFY(page);
    QVERIFY(!page->findChild<QComboBox*>(QStringLiteral("optionsMixerMode")));
    QVERIFY(!page->findChild<QWidget*>(QStringLiteral("optionsMixerOpacity")));
    auto* sampleAll = page->findChild<QCheckBox*>(QStringLiteral("optionsMixerSampleAll"));
    QVERIFY(sampleAll);
    QVERIFY(sampleAll->isEnabled());
    auto* mixer = tools->optionsFor<pictura::MixerGroup>(pictura::ToolId::MixerBrush);
    QVERIFY(mixer);
    QVERIFY(!mixer->get().sampleAllLayers);

    sampleAll->setChecked(true);
    QVERIFY(mixer->get().sampleAllLayers);
    pictura::MixerOptions o = mixer->get();
    o.sampleAllLayers = false;
    mixer->set(o);
    QVERIFY(!mixer->get().sampleAllLayers);
}

void OptionsBarsParityTest::eraserBarMatchesTheBrush()
{
    QVERIFY(window_->newDocument(QStringLiteral("Eraser"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(bar && tools);
    window_->setActiveTool(pictura::ToolId::Eraser);

    QWidget* page = bar->pageForTest(pictura::ToolId::Eraser);
    QVERIFY(page);
    auto* tip = page->findChild<QToolButton*>(QStringLiteral("optionsBrushTip"));
    auto* mode = page->findChild<QComboBox*>(QStringLiteral("optionsEraserMode"));
    auto* opacity = page->findChild<QWidget*>(QStringLiteral("optionsEraserOpacity"));
    auto* flow = page->findChild<QWidget*>(QStringLiteral("optionsEraserFlow"));
    auto* toHistory = page->findChild<QCheckBox*>(QStringLiteral("optionsEraseToHistory"));
    QVERIFY(tip && mode && opacity && flow && toHistory);

    // The Eraser shares the Brush's tip and Opacity/Flow, with Brush / Pencil /
    // Block modes instead of the Brush's blend modes.
    auto* paint = tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Eraser);
    QVERIFY(paint);
    QCOMPARE(tip->text(), QString::number(paint->size()));
    QCOMPARE(mode->count(), 3);
    QCOMPARE(mode->itemText(0), QStringLiteral("Brush"));
    QCOMPARE(mode->itemText(1), QStringLiteral("Pencil"));
    QCOMPARE(mode->itemText(2), QStringLiteral("Block"));
    QCOMPARE(mode->currentIndex(), 0);
    QVERIFY(opacity->isEnabled() && flow->isEnabled());
    QVERIFY(!toHistory->isChecked());

    // Block erases at full strength: Opacity and Flow grey out.
    mode->setCurrentIndex(2);
    QVERIFY(!opacity->isEnabled() && !flow->isEnabled());
    QCOMPARE(tools->optionsFor<pictura::EraserGroup>(pictura::ToolId::Eraser)->get().mode, 2);
    mode->setCurrentIndex(0);
    QVERIFY(opacity->isEnabled() && flow->isEnabled());

    // Erase to History records on the shared group.
    toHistory->setChecked(true);
    QVERIFY(tools->optionsFor<pictura::EraserGroup>(pictura::ToolId::Eraser)->get().toHistory);
    toHistory->setChecked(false);
}

void OptionsBarsParityTest::eyedropperSampleSize()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(20, 0, 20, 40, Qt::black);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_eyedropper_size"));
    QVERIFY(f.ok());

    // Point Sample on the black half.
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 30, 20, 1, 2), QRgb(0xFF000000));
    // A 5x5 average straddling the boundary is a mid grey, neither pure.
    const quint32 average = pictura::sample_argb_scoped(*f.view, 20, 20, 5, 2);
    QVERIFY(qRed(average) > 0 && qRed(average) < 255);

    // The handler's foreground follows the chosen size.
    f.tools->setForeground(QColor(1, 2, 3));
    f.tools->setActiveTool(pictura::ToolId::Eyedropper);
    auto* sizeCombo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSize"));
    QVERIFY(sizeCombo);
    sizeCombo->setCurrentIndex(1); // 3 by 3 Average
    f.canvas->mousePressed(QPointF(20, 20), Qt::LeftButton, 0);
    QVERIFY(f.tools->foreground() != QColor(1, 2, 3));
}

void OptionsBarsParityTest::eyedropperSampleScope()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_eyedropper_scope"));
    QVERIFY(f.ok());

    // An invert adjustment darkens the composite but not the source pixels.
    QVERIFY(f.view->add_adjustment(QStringLiteral("invert")));
    f.view->set_active_layer(QStringLiteral("0"));

    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 2), QRgb(0xFF000000)); // All Layers
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 3), QRgb(0xFFFFFFFF)); // All, no adjustments
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 0), QRgb(0xFFFFFFFF)); // Current Layer
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 1), QRgb(0xFFFFFFFF)); // Current & Below
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 4), QRgb(0xFFFFFFFF)); // Below, no adjustments

    auto* sampleCombo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSample"));
    QVERIFY(sampleCombo);
    QCOMPARE(sampleCombo->count(), 5);
}

void OptionsBarsParityTest::eyedropperSamplingRing()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_eyedropper_ring"));
    QVERIFY(f.ok());
    f.tools->setActiveTool(pictura::ToolId::Eyedropper);

    auto* ring = window_->findChild<QCheckBox*>(QStringLiteral("optionsEyedropperRing"));
    auto* sizeCombo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSize"));
    QVERIFY(ring && sizeCombo);
    QVERIFY(!f.canvas->hasSamplingRingForTest());

    // Point Sample explicitly: the eyedropper size persists on the shared tool
    // controller, so an earlier case must not leak its 3 by 3 Average here.
    sizeCombo->setCurrentIndex(0);
    ring->setChecked(true);
    f.canvas->mouseMoved(QPointF(10, 10));
    QVERIFY(f.canvas->hasSamplingRingForTest());
    QCOMPARE(f.canvas->samplingRingDiameterForTest(), 1.0);

    sizeCombo->setCurrentIndex(2); // 5 by 5 Average
    f.canvas->mouseMoved(QPointF(10, 10));
    QCOMPARE(f.canvas->samplingRingDiameterForTest(), 5.0);

    ring->setChecked(false);
    QVERIFY(!f.canvas->hasSamplingRingForTest());
}

QTEST_MAIN(OptionsBarsParityTest)
#include "tst_options_bars_parity.moc"
