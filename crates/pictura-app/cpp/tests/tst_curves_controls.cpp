#include <QtTest/QtTest>

#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/layers_bridge.h"
#include "curves_dialog.h"
#include "frame.h"
#include "image_view.h"
#include "levels_dialog.h"
#include "panels/curve_widget.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"
#include "panels/properties_panel.h"
#include "qt_test_support.h"
#include "tools.h"

#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

class CurvesControlsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void curvesDisplayOnBothSurfaces();
    void curvesPropertiesLayout();
    void curvesModeChannels();
    void curvesPencilHistory();
    void curvesCanvasSampling();
    void curvesEndpointSamplingAndLifecycle();
    void curvesGrayPoint();
    void curvesAutoModesAndNoOp();

private:
    bool openPatch(bool ramp = false);
    pictura::PropertiesPanel* addCurves();
    std::unique_ptr<pictura::CurvesDialog> makeDialog();
    pictura::ToolController* tools();
    pictura::test::ScopedStateHome state_;
    QTemporaryDir directory_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
};

void CurvesControlsTest::initTestCase()
{
    QVERIFY(state_.isValid() && directory_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

void CurvesControlsTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool CurvesControlsTest::openPatch(bool ramp)
{
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(QColor(85, 125, 170));
    if (ramp) {
        for (int y = 0; y < 20; ++y) {
            for (int x = 0; x < 20; ++x) {
                const int v = x < 10 ? 40 : 192;
                image.setPixel(x, y, qRgb(v, v, v));
            }
        }
    }
    const QString file = directory_.filePath(QStringLiteral("curves.png"));
    if (!image.save(file) || !window_->openDocumentAtPath(file)) {
        return false;
    }
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    return view_;
}

pictura::PropertiesPanel* CurvesControlsTest::addCurves()
{
    auto* panel = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!panel || !layers || !view_->add_adjustment(QStringLiteral("curves"))) {
        return nullptr;
    }
    layers->setView(view_);
    panel->setView(view_);
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const auto row = pictura::layers_api::rowAt(*view_, i);
        if (row.hasAdjustment && layers->selectRowForTest(row.path)) {
            panel->refresh();
            return panel;
        }
    }
    return nullptr;
}

std::unique_ptr<pictura::CurvesDialog> CurvesControlsTest::makeDialog()
{
    const QString kind = QStringLiteral("curves");
    const auto block = pictura::image_adjustment_default(kind, 0, 0xffffff, false);
    const QByteArray encoded(reinterpret_cast<const char*>(block.data()), qsizetype(block.size()));
    auto dialog = std::make_unique<pictura::CurvesDialog>(view_, encoded, QRect(), window_.get());
    dialog->setCanvasSampling(tools());
    return dialog;
}

pictura::ToolController* CurvesControlsTest::tools()
{
    return window_->findChild<pictura::ToolController*>();
}

void CurvesControlsTest::curvesDisplayOnBothSurfaces()
{
    QVERIFY(openPatch());
    auto dialog = makeDialog();
    const auto editAndCheck = [](QWidget& owner, const QString& name) {
        auto* curve = owner.findChild<pictura::CurveWidget*>(name);
        auto* white = owner.findChild<QSlider*>(QStringLiteral("curvesWhiteSlider"));
        auto* grid = owner.findChild<QCheckBox*>(QStringLiteral("curvesDetailedGrid"));
        auto* overlays = owner.findChild<QCheckBox*>(QStringLiteral("curvesOverlays"));
        auto* endpoints = owner.findChild<pictura::TriangleSlider*>(QStringLiteral("curvesEndpoints"));
        QVERIFY(curve && (white || endpoints) && grid && overlays);
        const auto moveWhite = [&](int value) {
            if (white) {
                white->setValue(value);
            } else {
                endpoints->setValue(1, value);
                QVERIFY(QMetaObject::invokeMethod(endpoints, "valueChanged",
                                                  Q_ARG(int, 1), Q_ARG(int, value)));
            }
        };
        moveWhite(243);
        QCOMPARE(curve->outputAt(243), 255);
        QCOMPARE(curve->outputAt(250), 255);
        const QString before = curve->pointsText();
        grid->toggle();
        overlays->toggle();
        QCOMPARE(curve->pointsText(), before);
        curve->setPointsText(QStringLiteral("0,33 255,211"));
        moveWhite(230);
        QCOMPARE(qRound(curve->points().last().y() * 255), 211);
    };
    editAndCheck(*dialog, QStringLiteral("curve"));
    dialog->reject();
    auto* panel = addCurves();
    QVERIFY(panel);
    editAndCheck(*panel, QStringLiteral("adjustmentCurve"));
    panel->flushPending();
}

void CurvesControlsTest::curvesPropertiesLayout()
{
    QVERIFY(openPatch());
    auto* panel = addCurves();
    QVERIFY(panel);
    const auto panelName = panel->objectName();
    auto* column = window_->columnForPanel(panelName);
    QVERIFY(column);
    const QSize minimum = panel->minimumSize();
    const QSize maximum = panel->maximumSize();
    const bool panelWasOpen = column->isPanelVisible(panelName);
    const bool railMode = column->railMode();
    const auto restore = qScopeGuard([&] {
        panel->setMinimumSize(minimum);
        panel->setMaximumSize(maximum);
        column->showPanel(panelName, panelWasOpen);
        column->setRailMode(railMode);
        window_->hide();
    });
    window_->show();
    window_->setScreenMode(pictura::PicturaMainWindow::ScreenMode::Standard);
    window_->setPanelsHidden(false);
    column->setRailMode(false);
    QVERIFY(column->showPanel(panelName, true));
    QTRY_VERIFY(panel->isVisible());
    auto* preset = panel->findChild<QWidget*>(QStringLiteral("propertiesPreset"));
    auto* channel = panel->findChild<QComboBox*>(QStringLiteral("adjustmentCurveChannel"));
    auto* autoButton = panel->findChild<QPushButton*>(QStringLiteral("propertiesAuto"));
    auto* plot = panel->findChild<pictura::CurveWidget*>(QStringLiteral("adjustmentCurve"));
    auto* endpoints = panel->findChild<pictura::TriangleSlider*>(QStringLiteral("curvesEndpoints"));
    auto* input = panel->findChild<QSpinBox*>(QStringLiteral("adjustmentCurveInput"));
    auto* output = panel->findChild<QSpinBox*>(QStringLiteral("adjustmentCurveOutput"));
    QVERIFY(preset && channel && autoButton && plot && endpoints && input && output);
    QTRY_VERIFY(preset->isVisible() && channel->isVisible() && plot->isVisible());
    const auto bounds = [panel](QWidget* widget) {
        return QRect(widget->mapTo(panel, QPoint()), widget->size());
    };
    const auto button = [panel](const char* name) {
        return panel->findChild<QToolButton*>(QString::fromLatin1(name));
    };
    const int widths[] = {244, 308};
    int previousPlotWidth = 0;
    for (const int width : widths) {
        panel->setFixedWidth(width);
        QTRY_COMPARE(panel->width(), width);
        QCoreApplication::processEvents();
        QVERIFY(bounds(preset).bottom() < bounds(channel).top());
        QVERIFY(bounds(channel).top() <= bounds(autoButton).center().y());
        QVERIFY(bounds(autoButton).top() <= bounds(channel).center().y());
        QVERIFY(bounds(channel).bottom() < bounds(plot).top());
        QVERIFY(qAbs(plot->width() - plot->height()) <= 1);
        QVERIFY(plot->width() >= width - 90);
        QVERIFY(plot->width() > previousPlotWidth);
        previousPlotWidth = plot->width();
        QVERIFY(bounds(plot).bottom() < bounds(endpoints).top());
        QVERIFY(bounds(endpoints).bottom() < bounds(input).top());
        QCOMPARE(bounds(input).top(), bounds(output).top());
        for (auto* bar : panel->findChildren<QScrollBar*>()) {
            if (bar->orientation() == Qt::Horizontal) {
                QCOMPARE(bar->maximum(), 0);
            }
        }
    }

    int previousTop = -1;
    for (const char* name : {"curvesTarget", "curvesBlackPoint", "curvesGrayPoint",
                             "curvesWhitePoint", "curvesPointMode", "curvesPencil",
                             "curvesSmooth", "curvesClipping"}) {
        auto* tool = button(name);
        QVERIFY(tool);
        QVERIFY(!tool->toolTip().isEmpty() && !tool->accessibleName().isEmpty());
        for (auto mode : {QIcon::Normal, QIcon::Disabled}) {
            const QImage glyph = tool->icon().pixmap(QSize(16, 16), mode).toImage();
            bool visibleInk = false;
            for (int y = 0; y < glyph.height() && !visibleInk; ++y) {
                for (int x = 0; x < glyph.width(); ++x) {
                    visibleInk |= qAlpha(glyph.pixel(x, y)) > 0;
                }
            }
            QVERIFY(visibleInk);
        }
        QVERIFY(bounds(tool).right() < bounds(plot).left());
        QVERIFY(bounds(tool).top() > previousTop);
        previousTop = bounds(tool).top();
    }
    auto* point = button("curvesPointMode");
    auto* pencil = button("curvesPencil");
    auto* clipping = button("curvesClipping");
    QVERIFY(point->isChecked() && !pencil->isChecked());
    pencil->click();
    QVERIFY(pencil->isChecked() && !point->isChecked());
    point->click();
    QVERIFY(point->isChecked() && !pencil->isChecked());
    QCOMPARE(endpoints->value(1), 255);
    QTest::keyClick(endpoints, Qt::Key_Down);
    QTest::keyClick(endpoints, Qt::Key_Left);
    QCOMPARE(endpoints->value(1), 254);
    QCOMPARE(qRound(plot->points().last().x() * 255), 254);

    QMenu menu;
    auto* clippingMenu = menu.addAction(QStringLiteral("Clipping"));
    clippingMenu->setData(QStringLiteral("properties.showClipping"));
    clippingMenu->setCheckable(true);
    auto* canvas = tools() ? tools()->canvas() : nullptr;
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(canvas && layers);
    clipping->click();
    panel->updatePanelMenu(&menu);
    QVERIFY(clipping->isChecked() && clippingMenu->isChecked());
    QVERIFY(canvas->filterPreviewShownForTest());
    QCOMPARE(canvas->filterPreviewImageForTest().pixel(0, 0), qRgb(128, 128, 128));
    clippingMenu->trigger();
    QVERIFY(!clipping->isChecked() && !canvas->filterPreviewShownForTest());
    panel->updatePanelMenu(&menu);
    QVERIFY(!clippingMenu->isChecked());

    clippingMenu->trigger();
    QVERIFY(clipping->isChecked() && canvas->filterPreviewShownForTest());
    QVERIFY(view_->add_adjustment(QStringLiteral("levels")));
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const auto row = pictura::layers_api::rowAt(*view_, i);
        if (row.adjustmentKind == QLatin1String("levels")) {
            QVERIFY(layers->selectRowForTest(row.path));
            break;
        }
    }
    panel->refresh();
    QVERIFY(!canvas->filterPreviewShownForTest());
    QVERIFY(!panel->findChild<pictura::CurveWidget*>(QStringLiteral("adjustmentCurve")));
    QCOMPARE(panel->findChild<QPushButton*>(QStringLiteral("propertiesAuto")), autoButton);
    QVERIFY(!autoButton->isHidden());
}

void CurvesControlsTest::curvesModeChannels()
{
    QVERIFY(openPatch());
    for (const auto& [mode, count] : {std::pair{"grayscale", 1}, {"cmyk", 5}, {"lab", 3}}) {
        QCOMPARE(bool(pictura::convert_image_mode(*view_, QString::fromLatin1(mode))), true);
        auto dialog = makeDialog();
        auto* channel = dialog->findChild<QComboBox*>(QStringLiteral("curvesChannel"));
        auto* graph = dialog->findChild<pictura::CurveWidget*>(QStringLiteral("curve"));
        auto* input = dialog->findChild<QSpinBox*>(QStringLiteral("curvesInput"));
        auto* output = dialog->findChild<QSpinBox*>(QStringLiteral("curvesOutput"));
        QVERIFY(channel && graph && input && output);
        QCOMPARE(channel->count(), count);
        for (int index = 0; index < count; ++index) {
            channel->setCurrentIndex(index);
            QVERIFY(graph->selectTone(128));
            const QByteArray before = dialog->block();
            graph->moveSelected(128, 190);
            QVERIFY(dialog->block() != before);
            output->setValue(output->value() - 1);
            QCOMPARE(qRound(graph->points()[graph->selected()].x() * 255), 128);
            const int outputCode = qRound(graph->points()[graph->selected()].y() * 255);
            input->setValue(input->value() + 1);
            QCOMPARE(qRound(graph->points()[graph->selected()].y() * 255), outputCode);
            if (mode == QLatin1String("cmyk")) {
                QCOMPARE(input->suffix(), QStringLiteral(" %"));
            } else if (mode == QLatin1String("lab") && index > 0) {
                QCOMPARE(input->minimum(), -128);
                QCOMPARE(input->maximum(), 127);
            }
        }
        dialog->reject();
    }
}

void CurvesControlsTest::curvesPencilHistory()
{
    QVERIFY(openPatch());
    auto dialog = makeDialog();
    const auto draw = [](QWidget& owner, const QString& name) {
        auto* graph = owner.findChild<pictura::CurveWidget*>(name);
        auto* pencil = owner.findChild<QToolButton*>(QStringLiteral("curvesPencil"));
        auto* smooth = owner.findChild<QToolButton*>(QStringLiteral("curvesSmooth"));
        QVERIFY(graph && pencil && smooth);
        graph->resize(200, 200);
        pencil->click();
        QTest::mousePress(graph, Qt::LeftButton, Qt::NoModifier, QPoint(45, 40));
        QTest::mouseMove(graph, QPoint(150, 65));
        QTest::mouseRelease(graph, Qt::LeftButton, Qt::NoModifier, QPoint(150, 65));
        QVERIFY(graph->hasDenseCurve());
        const QString before = graph->pointsText();
        smooth->click();
        const QString after = graph->pointsText();
        QVERIFY(before != after);
        smooth->click();
        QVERIFY(after != graph->pointsText());
    };
    draw(*dialog, QStringLiteral("curve"));
    dialog->reject();
    auto* panel = addCurves();
    QVERIFY(panel);
    const int history = view_->history_count();
    draw(*panel, QStringLiteral("adjustmentCurve"));
    QCOMPARE(view_->history_count(), history + 3);
    const QString path = panel->currentPath();
    const QString pencil = pictura::adjustment_curve(*view_, path, 0);
    QVERIFY(view_->undo());
    QVERIFY(view_->redo());
    QCOMPARE(pictura::adjustment_curve(*view_, path, 0), pencil);
}

void CurvesControlsTest::curvesCanvasSampling()
{
    QVERIFY(openPatch());
    auto dialog = makeDialog();
    auto* target = dialog->findChild<QToolButton*>(QStringLiteral("curvesTarget"));
    auto* graph = dialog->findChild<pictura::CurveWidget*>(QStringLiteral("curve"));
    auto* canvas = window_->imageView();
    QVERIFY(target && graph && tools() && canvas);
    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();
    target->click();
    QVERIFY(tools()->canvasBrushActive());
    emit canvas->mousePressed(QPointF(5, 5), Qt::LeftButton, Qt::NoModifier);
    emit canvas->mouseMoved(QPointF(5, 0));
    emit canvas->mouseReleased(QPointF(5, 0));
    const int tone = qGray(before);
    QCOMPARE(qRound(graph->points()[graph->selected()].x() * 255), tone);
    QVERIFY(graph->outputAt(tone) > tone);
    dialog->reject();
    QVERIFY(!tools()->canvasBrushActive());
    QCOMPARE(view_->composite_argb(5, 5), before);
    QCOMPARE(view_->history_count(), history);
}

void CurvesControlsTest::curvesEndpointSamplingAndLifecycle()
{
    QVERIFY(openPatch(true));
    auto sampleEndpoints = [this](QWidget& owner, const QString& name) {
        auto* black = owner.findChild<QToolButton*>(QStringLiteral("curvesBlackPoint"));
        auto* white = owner.findChild<QToolButton*>(QStringLiteral("curvesWhitePoint"));
        auto* graph = owner.findChild<pictura::CurveWidget*>(name);
        auto* canvas = window_->imageView();
        QVERIFY(black && white && graph && canvas);
        black->click();
        emit canvas->mousePressed(QPointF(5, 5), Qt::LeftButton, Qt::NoModifier);
        emit canvas->mouseReleased(QPointF(5, 5));
        QCOMPARE(qRound(graph->points().first().x() * 255), 40);
        QCoreApplication::processEvents();
        white->click();
        emit canvas->mousePressed(QPointF(15, 5), Qt::LeftButton, Qt::NoModifier);
        emit canvas->mouseReleased(QPointF(15, 5));
        QCOMPARE(qRound(graph->points().last().x() * 255), 192);
        QCoreApplication::processEvents();
    };
    auto dialog = makeDialog();
    sampleEndpoints(*dialog, QStringLiteral("curve"));
    QCOMPARE(qRound(dialog->findChild<pictura::CurveWidget*>(QStringLiteral("curve"))->points().last().x() * 255), 192);
    dialog->reject();
    auto* panel = addCurves();
    QVERIFY(panel);
    const int history = view_->history_count();
    sampleEndpoints(*panel, QStringLiteral("adjustmentCurve"));
    QCOMPARE(view_->history_count(), history + 2);
    auto* target = panel->findChild<QToolButton*>(QStringLiteral("curvesTarget"));
    QVERIFY(target);
    target->click();
    QVERIFY(tools()->canvasBrushActive());
    panel->setView(nullptr);
    QVERIFY(!tools()->canvasBrushActive());
}

void CurvesControlsTest::curvesGrayPoint()
{
    QVERIFY(openPatch());
    const auto pickGray = [this](QWidget& owner) {
        auto* gray = owner.findChild<QToolButton*>(QStringLiteral("curvesGrayPoint"));
        auto* canvas = window_->imageView();
        QVERIFY(gray && canvas && gray->isEnabled());
        gray->click();
        emit canvas->mousePressed(QPointF(5, 5), Qt::LeftButton, Qt::NoModifier);
        emit canvas->mouseReleased(QPointF(5, 5));
        QCoreApplication::processEvents();
    };
    auto dialog = makeDialog();
    const QByteArray before = dialog->block();
    pickGray(*dialog);
    QVERIFY(dialog->block() != before);
    dialog->reject();
    auto* panel = addCurves();
    QVERIFY(panel);
    const int history = view_->history_count();
    const QRgb initial = view_->sample_argb(5, 5);
    pickGray(*panel);
    const QRgb neutral = view_->sample_argb(5, 5);
    QCOMPARE(qRed(neutral), qGreen(neutral));
    QCOMPARE(qGreen(neutral), qBlue(neutral));
    QCOMPARE(view_->history_count(), history + 1);
    QVERIFY(view_->undo());
    QCOMPARE(view_->sample_argb(5, 5), initial);
    QVERIFY(view_->redo());
    QCOMPARE(view_->sample_argb(5, 5), neutral);
}

void CurvesControlsTest::curvesAutoModesAndNoOp()
{
    QVERIFY(openPatch());
    auto dialog = makeDialog();
    auto* dialogAuto = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
    QVERIFY(dialogAuto);
    const QByteArray before = dialog->block();
    dialogAuto->click();
    QCOMPARE(dialog->block(), before);
    dialog->reject();
    QVERIFY(openPatch(true));
    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("grayscale")));
    auto* panel = addCurves();
    QVERIFY(panel);
    auto* autoButton = panel->findChild<QPushButton*>(QStringLiteral("propertiesAuto"));
    QVERIFY(autoButton && autoButton->isEnabled());
    autoButton->click();
    const QString path = panel->currentPath();
    QCOMPARE(pictura::adjustment_curve(*view_, path, 0), QStringLiteral("40,0 192,255"));
    for (const QString& mode : {QStringLiteral("cmyk"), QStringLiteral("lab")}) {
        QVERIFY(pictura::convert_image_mode(*view_, mode));
        auto modeDialog = makeDialog();
        QVERIFY(!modeDialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"))->isEnabled());
        modeDialog->reject();
    }
}

PICTURA_QT_TEST_MAIN(CurvesControlsTest)
#include "tst_curves_controls.moc"
