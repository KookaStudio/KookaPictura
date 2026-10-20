// The issue #731 brush presets and the on-canvas pen-tip model preview.

#include <QtTest/QtTest>

#include "brush_tip_preview.h"
#include "image_view.h"
#include "options_bar.h"
#include "panels/brush_preset_picker.h"
#include "panels/numeric_field.h"
#include "qt_test_support.h"
#include "selftest_paint_fixture.h"
#include "toolbox.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QPoint>
#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/qrgb.h>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <memory>

namespace {

using paint_fixture::Fixture;

QImage seedImage()
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    return seed;
}

pictura::BrushPresetPicker* buildPicker(pictura::ToolController* tools,
                                        pictura::ImageView* canvas,
                                        pictura::PicturaMainWindow& frame)
{
    tools->routeCanvasContextMenu(canvas, QPointF(2, 2), QPoint(10, 10));
    auto* picker = frame.findChild<pictura::BrushPresetPicker*>();
    if (picker) {
        picker->hide();
    }
    return picker;
}

bool choosePreset(pictura::BrushPresetPicker* picker, const QString& name)
{
    auto* grid = picker->findChild<QListWidget*>(QStringLiteral("brushPickerGrid"));
    if (!grid) {
        return false;
    }
    for (int i = 0; i < grid->count(); ++i) {
        if (grid->item(i)->toolTip() == name) {
            picker->choosePresetForTest(i);
            return true;
        }
    }
    return false;
}

} // namespace

class BrushTipPreviewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void brushToolMenuListsThePresets();
    void fourAdditionalPresets();
    void presetUpdatesToolboxForeground();
    void historyBrushOmitsAdditionalPresets();
    void additionalPresetsApplyTheirPaintSettings();
    void presetUpdatesTheOptionsBar();
    void existingPresetSetsTipOnly();
    void pencilPresetsSetThePenTipModel();
    void ovalPresetSetsWetEdges();
    void pencilPresetShowsThePenTip();
    void nonPencilPresetHidesThePenTip();
    void anotherToolHidesThePenTip();
    void overlayDragsWithoutPainting();
    void previewFollowsTheActiveDocument();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void BrushTipPreviewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void BrushTipPreviewTest::brushToolMenuListsThePresets()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_menu"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* bar = f.frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* stack = bar->findChild<QStackedWidget*>();
    QWidget* page = stack ? stack->currentWidget() : nullptr;
    QVERIFY(page);

    QMenu* menu = nullptr;
    for (QToolButton* tool : page->findChildren<QToolButton*>()) {
        if (tool->menu() && tool->menu()->actions().size() == 4) {
            menu = tool->menu();
            break;
        }
    }
    QVERIFY(menu);
    QStringList labels;
    for (QAction* action : menu->actions()) {
        labels << action->text();
    }
    QCOMPARE(labels, QStringList({QStringLiteral("2B Pencil"),
                                  QStringLiteral("Airbrush Soft Round 50% flow"),
                                  QStringLiteral("Non-Photo Blue Pencil"),
                                  QStringLiteral("Paintbrush Oval 45 pixels Multiply")}));

    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint);
    menu->actions().at(0)->trigger();
    QCOMPARE(paint->size(), 8);
    QCOMPARE(paint->hardness(), 25);
    QCOMPARE(paint->grain(), 50);
    QCOMPARE(paint->dynamics().sizeJitter, 20);
    QVERIFY(paint->penTip());
    QVERIFY(!paint->aliased());
    QCOMPARE(f.tools->foreground(), QColor(QStringLiteral("#47484a")));
    QCOMPARE(paint->mode(), QStringLiteral("normal"));
    QCOMPARE(paint->opacity(), 100);
    QCOMPARE(paint->flow(), 100);
}

void BrushTipPreviewTest::fourAdditionalPresets()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_four"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* grid = picker->findChild<QListWidget*>(QStringLiteral("brushPickerGrid"));
    QVERIFY(grid);
    QCOMPARE(picker->presetCount(), 48);
    QCOMPARE(grid->count(), 48);

    const QStringList names{
        QStringLiteral("2B Pencil"),
        QStringLiteral("Airbrush Soft Round 50% flow"),
        QStringLiteral("Non-Photo Blue Pencil"),
        QStringLiteral("Paintbrush Oval 45 pixels Multiply"),
    };
    const int first = picker->presetCount() - int(names.size());
    for (int i = 0; i < names.size(); ++i) {
        QCOMPARE(grid->item(first + i)->toolTip(), names.at(i));
    }
}

void BrushTipPreviewTest::presetUpdatesToolboxForeground()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_swatch"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* toolbox = f.frame.findChild<pictura::Toolbox*>();
    QVERIFY(toolbox);
    auto* swatch = toolbox->findChild<pictura::ForegroundBackgroundWidget*>();
    QVERIFY(swatch);

    QVERIFY(choosePreset(picker, QStringLiteral("Non-Photo Blue Pencil")));
    QCOMPARE(swatch->foregroundForTest(), QColor(QStringLiteral("#73d6ff")));
    QVERIFY(choosePreset(picker, QStringLiteral("2B Pencil")));
    QCOMPARE(swatch->foregroundForTest(), QColor(QStringLiteral("#47484a")));
}

void BrushTipPreviewTest::historyBrushOmitsAdditionalPresets()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_history"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::HistoryBrush);
    auto* bar = f.frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* stack = bar->findChild<QStackedWidget*>();
    QWidget* page = stack ? stack->currentWidget() : nullptr;
    QVERIFY(page);

    const QStringList names{
        QStringLiteral("2B Pencil"),
        QStringLiteral("Airbrush Soft Round 50% flow"),
        QStringLiteral("Non-Photo Blue Pencil"),
        QStringLiteral("Paintbrush Oval 45 pixels Multiply"),
    };
    for (QToolButton* tool : page->findChildren<QToolButton*>()) {
        if (!tool->menu()) {
            continue;
        }
        for (QAction* action : tool->menu()->actions()) {
            QVERIFY2(!names.contains(action->text()),
                     qPrintable(QStringLiteral("History Brush lists %1").arg(action->text())));
        }
    }
}

void BrushTipPreviewTest::additionalPresetsApplyTheirPaintSettings()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_defaults"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint);

    const QColor previous = f.tools->foreground();
    paint->setMode(QStringLiteral("behind"));
    paint->setOpacity(10);
    paint->setFlow(20);
    f.tools->setForeground(QColor(0x11, 0x22, 0x33));

    struct Expected {
        const char* name;
        int size;
        int hardness;
        int roundness;
        int spacing;
        int grain;
        bool penTip;
        bool aliased;
        const char* color;
        const char* mode;
        int opacity;
        int flow;
    };
    const Expected expected[] = {
        {"2B Pencil", 8, 25, 100, 25, 50, true, false, "#47484a", "normal", 100, 100},
        {"Airbrush Soft Round 50% flow", 45, 0, 100, 25, 0, false, false, "#47484a",
         "normal", 100, 50},
        {"Non-Photo Blue Pencil", 4, 0, 100, 20, 55, true, false, "#73d6ff", "normal", 100,
         100},
        {"Paintbrush Oval 45 pixels Multiply", 45, 100, 50, 5, 0, false, false, "#47484a",
         "multiply", 35, 50},
    };
    for (const Expected& e : expected) {
        QVERIFY2(choosePreset(picker, QString::fromLatin1(e.name)), e.name);
        QCOMPARE(paint->size(), e.size);
        QCOMPARE(paint->hardness(), e.hardness);
        QCOMPARE(paint->roundness(), e.roundness);
        QCOMPARE(paint->spacing(), e.spacing);
        QCOMPARE(paint->grain(), e.grain);
        QCOMPARE(paint->penTip(), e.penTip);
        QCOMPARE(paint->aliased(), e.aliased);
        QCOMPARE(f.tools->foreground(), QColor(QString::fromLatin1(e.color)));
        QCOMPARE(paint->mode(), QString::fromLatin1(e.mode));
        QCOMPARE(paint->opacity(), e.opacity);
        QCOMPARE(paint->flow(), e.flow);
    }
    f.tools->setForeground(previous);
}

void BrushTipPreviewTest::presetUpdatesTheOptionsBar()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_bar"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* bar = f.frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* opacity =
        bar->findChild<pictura::NumericField*>(QStringLiteral("optionsBrushOpacity"));
    auto* flow = bar->findChild<pictura::NumericField*>(QStringLiteral("optionsBrushFlow"));
    QVERIFY(opacity && flow);
    auto* stack = bar->findChild<QStackedWidget*>();
    QWidget* page = stack ? stack->currentWidget() : nullptr;
    QVERIFY(page);
    auto* mode = page->findChild<QComboBox*>();
    QVERIFY(mode);

    QVERIFY(choosePreset(picker, QStringLiteral("Airbrush Soft Round 50% flow")));
    QCOMPARE(opacity->value(), 100.0);
    QCOMPARE(flow->value(), 50.0);
    QCOMPARE(mode->currentData().toString(), QStringLiteral("normal"));

    QVERIFY(choosePreset(picker, QStringLiteral("Paintbrush Oval 45 pixels Multiply")));
    QCOMPARE(opacity->value(), 35.0);
    QCOMPARE(flow->value(), 50.0);
}

void BrushTipPreviewTest::existingPresetSetsTipOnly()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_tip_only"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint);

    const QColor previous = f.tools->foreground();
    const QColor blue(0x4b, 0x9c, 0xd3);
    paint->setMode(QStringLiteral("multiply"));
    paint->setOpacity(35);
    paint->setFlow(50);
    f.tools->setForeground(blue);

    QVERIFY(choosePreset(picker, QStringLiteral("Soft Round 5")));
    QCOMPARE(paint->size(), 5);
    QCOMPARE(paint->hardness(), 0);
    QCOMPARE(paint->roundness(), 100);
    QCOMPARE(paint->tipAngle(), 0);
    QCOMPARE(paint->spacing(), 25);
    const pictura::BrushDynamics dynamics = paint->dynamics();
    QCOMPARE(dynamics.scatter, 0);
    QCOMPARE(dynamics.count, 1);
    QCOMPARE(dynamics.sizeJitter, 0);
    QCOMPARE(dynamics.angleJitter, 0);
    QCOMPARE(dynamics.roundnessJitter, 0);
    QCOMPARE(paint->grain(), 0);
    QCOMPARE(paint->mode(), QStringLiteral("multiply"));
    QCOMPARE(paint->opacity(), 35);
    QCOMPARE(paint->flow(), 50);
    QCOMPARE(f.tools->foreground(), blue);
    QVERIFY(!paint->penTip());
    QVERIFY(!paint->aliased());
    f.tools->setForeground(previous);
}

void BrushTipPreviewTest::pencilPresetsSetThePenTipModel()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_model"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint);

    QVERIFY(choosePreset(picker, QStringLiteral("2B Pencil")));
    QVERIFY(paint->penTip());
    QVERIFY(!paint->aliased());
    QCOMPARE(paint->size(), 8);
    QCOMPARE(paint->grain(), 50);
    QCOMPARE(f.tools->foreground(), QColor(QStringLiteral("#47484a")));
    QVERIFY(choosePreset(picker, QStringLiteral("Non-Photo Blue Pencil")));
    QVERIFY(paint->penTip());
    QVERIFY(!paint->aliased());
    QCOMPARE(paint->size(), 4);
    QCOMPARE(paint->grain(), 55);
    QCOMPARE(f.tools->foreground(), QColor(QStringLiteral("#73d6ff")));
    QVERIFY(choosePreset(picker, QStringLiteral("Soft Round 5")));
    QVERIFY(!paint->penTip());
    QVERIFY(!paint->aliased());
}

void BrushTipPreviewTest::ovalPresetSetsWetEdges()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_wet"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint);

    QVERIFY(choosePreset(picker, QStringLiteral("Paintbrush Oval 45 pixels Multiply")));
    QCOMPARE(paint->size(), 45);
    QCOMPARE(paint->hardness(), 100);
    QCOMPARE(paint->roundness(), 50);
    QCOMPARE(paint->tipAngle(), -50);
    QCOMPARE(paint->spacing(), 5);
    QVERIFY(paint->wetEdges());
    QVERIFY(choosePreset(picker, QStringLiteral("Soft Round 5")));
    QVERIFY(!paint->wetEdges());
}

void BrushTipPreviewTest::pencilPresetShowsThePenTip()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_show"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    QVERIFY(choosePreset(picker, QStringLiteral("2B Pencil")));

    auto* preview = f.canvas->findChild<pictura::BrushTipPreview*>();
    QVERIFY(preview);
    QVERIFY(!preview->isHidden());
    QCOMPARE(preview->pos(), QPoint(pictura::BrushTipPreview::kMargin,
                                    pictura::BrushTipPreview::kMargin));

    const QImage image = preview->grab().toImage();
    QCOMPARE(image.size(), QSize(pictura::BrushTipPreview::kWidth,
                                 pictura::BrushTipPreview::kHeight));
    QVERIFY(qGray(image.pixel(image.width() / 2, 20)) > 180);
    QVERIFY(qGray(image.pixel(image.width() / 2 - 2, image.height() * 65 / 100)) < 100);
}

void BrushTipPreviewTest::nonPencilPresetHidesThePenTip()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_hide"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    auto* preview = f.canvas->findChild<pictura::BrushTipPreview*>();
    QVERIFY(preview);

    QVERIFY(choosePreset(picker, QStringLiteral("2B Pencil")));
    QVERIFY(!preview->isHidden());
    QVERIFY(choosePreset(picker, QStringLiteral("Soft Round 5")));
    QVERIFY(preview->isHidden());
}

void BrushTipPreviewTest::anotherToolHidesThePenTip()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_tool"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    QVERIFY(choosePreset(picker, QStringLiteral("2B Pencil")));
    auto* preview = f.canvas->findChild<pictura::BrushTipPreview*>();
    QVERIFY(preview);
    QVERIFY(!preview->isHidden());

    f.frame.setActiveTool(pictura::ToolId::Pencil);
    QVERIFY(!preview->isHidden());
    f.frame.setActiveTool(pictura::ToolId::Move);
    QVERIFY(preview->isHidden());
    f.frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY(!preview->isHidden());
}

void BrushTipPreviewTest::overlayDragsWithoutPainting()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_drag"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    QVERIFY(choosePreset(picker, QStringLiteral("2B Pencil")));
    auto* preview = f.canvas->findChild<pictura::BrushTipPreview*>();
    QVERIFY(preview);
    QVERIFY(!preview->isHidden());

    QSignalSpy pressSpy(f.canvas, &pictura::ImageView::mousePressed);
    const QPoint start = preview->pos();
    const QPoint pressAt(10, 10);
    QTest::mousePress(preview, Qt::LeftButton, Qt::NoModifier, pressAt);
    QTest::mouseMove(preview, pressAt + QPoint(20, 20));
    QTest::mouseRelease(preview, Qt::LeftButton, Qt::NoModifier, pressAt + QPoint(20, 20));
    QCOMPARE(pressSpy.count(), 0);
    QCOMPARE(preview->pos(), start + QPoint(20, 20));
    QVERIFY(!f.view->is_painting());

    QTest::mousePress(preview, Qt::LeftButton, Qt::NoModifier, pressAt);
    QTest::mouseMove(preview, QPoint(10000, 10000));
    QTest::mouseRelease(preview, Qt::LeftButton, Qt::NoModifier, QPoint(10000, 10000));
    QVERIFY(f.canvas->rect().contains(preview->geometry()));
}

void BrushTipPreviewTest::previewFollowsTheActiveDocument()
{
    Fixture f(*window_, seedImage(), QStringLiteral("btp_active"));
    QVERIFY2(f.ok(), "brush fixture");
    f.frame.setActiveTool(pictura::ToolId::Brush);
    auto* picker = buildPicker(f.tools, f.canvas, f.frame);
    QVERIFY(picker);
    QVERIFY(choosePreset(picker, QStringLiteral("2B Pencil")));
    auto* first = f.canvas->findChild<pictura::BrushTipPreview*>();
    QVERIFY(first);
    QVERIFY(!first->isHidden());

    QVERIFY(f.frame.newDocument(QStringLiteral("second"), 32, 32, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    const int secondIndex = f.frame.activeDocumentIndex();
    pictura::ImageView* secondCanvas = f.frame.imageView();
    QVERIFY(secondCanvas && secondCanvas != f.canvas);
    auto* second = secondCanvas->findChild<pictura::BrushTipPreview*>();
    QVERIFY(second);
    QVERIFY(!second->isHidden());
    QVERIFY(first->isHidden());

    f.frame.setActiveDocumentIndex(f.doc);
    QVERIFY(!first->isHidden());
    QVERIFY(second->isHidden());
    f.frame.closeDocument(secondIndex, false);
}

QTEST_MAIN(BrushTipPreviewTest)
#include "tst_brush_tip_preview.moc"
