#include "selftest_erasers.h"
#include "selftest_report.h"

#include "selftest_paint_fixture.h"

#include "panels/numeric_field.h"

#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>

namespace {
using paint_fixture::BrushState;
using paint_fixture::Fixture;
} // namespace

int pictura::runEraserChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_eraser_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(547, "eraser fixture");
    }
    const BrushState brush(f.tools);
    const QColor foreground = f.tools->foreground();
    const QColor background = f.tools->background();
    const pictura::EraserOptions options = f.tools->optionsFor<EraserGroup>(ToolId::Eraser)->get();
    const QRgb red = QColor(Qt::red).rgba();
    const QRgb white = QColor(Qt::white).rgba();

    frame.setActiveTool(pictura::ToolId::Eraser);
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsEraserMode"));
    auto* opacity = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsEraserOpacity"));
    auto* toHistory = frame.findChild<QCheckBox*>(QStringLiteral("optionsEraseToHistory"));
    const bool bar = mode && opacity && toHistory && mode->count() == 3
        && mode->currentIndex() == 0 && opacity->isEnabled() && !toHistory->isChecked();

    // The opened PNG is the Background: erasing paints the background colour.
    f.tools->setBackground(Qt::red);
    const int base = f.view->history_index();
    f.drag({QPointF(5, 20), QPointF(20, 20), QPointF(35, 20)});
    const bool toBackground = f.committedOnce(base, "Eraser") && f.view->sample_argb(10, 20) == red;

    // Alt paints the oldest (white) state back.
    f.drag({QPointF(5, 20), QPointF(10, 20), QPointF(15, 20)}, Qt::AltModifier);
    const bool toHistoryOk = f.committedOnce(base + 1, "Eraser")
        && f.view->sample_argb(10, 20) == white && f.view->sample_argb(30, 20) == red;

    // On an ordinary layer it erases to transparency, showing the Background.
    const int layer = f.view->add_layer(0);
    f.view->set_active_layer(QString::number(layer));
    f.tools->setForeground(Qt::black);
    frame.setActiveTool(pictura::ToolId::Brush);
    f.drag({QPointF(5, 8), QPointF(35, 8)});
    frame.setActiveTool(pictura::ToolId::Eraser);
    const bool black = f.view->sample_argb(10, 8) == QColor(Qt::black).rgba();
    f.drag({QPointF(5, 8), QPointF(20, 8)});
    const bool transparent = black && f.view->sample_argb(10, 8) == white
        && f.view->sample_argb(30, 8) == QColor(Qt::black).rgba();

    bool block = false;
    if (mode) {
        mode->setCurrentIndex(2);
        block = f.tools->optionsFor<EraserGroup>(ToolId::Eraser)->get().mode == 2 && !opacity->isEnabled();
        mode->setCurrentIndex(0);
    }
    f.tools->optionsFor<EraserGroup>(ToolId::Eraser)->set(options);
    f.tools->setForeground(foreground);
    f.tools->setBackground(background);

    ST_BEGIN("eraser_tool");
    ST_PASS("eraser bar=%d background=%d history=%d transparent=%d block=%d", bar ? 1 : 0,
            toBackground ? 1 : 0, toHistoryOk ? 1 : 0, transparent ? 1 : 0, block ? 1 : 0);
    if (!bar || !toBackground || !toHistoryOk || !transparent || !block) {
        return pictura::selfTest().fail(547, "eraser tool");
    }
    return 0;
}

int pictura::runBackgroundEraserChecks(pictura::PicturaMainWindow& frame)
{
    // Sky (blue) left of x = 30, a subject (yellow) right of it.
    QImage seed(60, 40, QImage::Format_RGB32);
    seed.fill(QColor(30, 120, 220));
    QPainter(&seed).fillRect(30, 0, 30, 40, QColor(220, 200, 40));
    Fixture f(frame, seed, QStringLiteral("pictura_background_eraser_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(550, "background eraser fixture");
    }
    const BrushState brush(f.tools);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(16);

    frame.setActiveTool(pictura::ToolId::BackgroundEraser);
    auto* sampling = frame.findChild<QComboBox*>(QStringLiteral("optionsBackgroundEraseSampling"));
    auto* limits = frame.findChild<QComboBox*>(QStringLiteral("optionsBackgroundEraseLimits"));
    auto* tolerance =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsBackgroundEraseTolerance"));
    auto* protect = frame.findChild<QCheckBox*>(QStringLiteral("optionsBackgroundEraseProtect"));
    const bool bar = sampling && limits && tolerance && protect && sampling->currentIndex() == 0
        && limits->currentIndex() == 1 && tolerance->value() == 50 && !protect->isChecked();

    // The crosshair runs down the sky, 4 px from the subject.
    const int base = f.view->history_index();
    f.drag({QPointF(26, 8), QPointF(26, 20), QPointF(26, 32)});
    const bool committed = f.committedOnce(base, "Background Eraser");
    const bool skyGone = qAlpha(f.view->sample_argb(24, 20)) == 0
        && qAlpha(f.view->sample_argb(29, 20)) == 0;
    const bool subjectKept = f.view->sample_argb(32, 20) == QColor(220, 200, 40).rgba();
    const bool farSkyKept = f.view->sample_argb(5, 20) == QColor(30, 120, 220).rgba();

    ST_BEGIN("background_eraser_tool");
    ST_PASS("background eraser bar=%d commit=%d sky=%d subject=%d far=%d", bar ? 1 : 0,
            committed ? 1 : 0, skyGone ? 1 : 0, subjectKept ? 1 : 0, farSkyKept ? 1 : 0);
    if (!bar || !committed || !skyGone || !subjectKept || !farSkyKept) {
        return pictura::selfTest().fail(550, "background eraser tool");
    }
    return 0;
}

int pictura::runMagicEraserChecks(pictura::PicturaMainWindow& frame)
{
    // Two separate red squares on white.
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    {
        QPainter painter(&seed);
        painter.fillRect(5, 5, 10, 10, QColor(220, 20, 20));
        painter.fillRect(25, 25, 10, 10, QColor(220, 20, 20));
    }
    Fixture f(frame, seed, QStringLiteral("pictura_magic_eraser_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(551, "magic eraser fixture");
    }
    const pictura::MagicEraseOptions options = f.tools->optionsFor<MagicEraseGroup>(ToolId::MagicEraser)->get();

    frame.setActiveTool(pictura::ToolId::MagicEraser);
    auto* tolerance =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsMagicEraseTolerance"));
    auto* antialias = frame.findChild<QCheckBox*>(QStringLiteral("optionsMagicEraseAntialias"));
    auto* contiguous = frame.findChild<QCheckBox*>(QStringLiteral("optionsMagicEraseContiguous"));
    const bool bar = tolerance && antialias && contiguous && tolerance->value() == 32
        && antialias->isChecked() && contiguous->isChecked()
        && !pictura::isBrushTool(pictura::ToolId::MagicEraser);

    const QRgb red = QColor(220, 20, 20).rgba();
    const int base = f.view->history_index();
    f.drag({QPointF(9, 9)});
    const bool committed = f.committedOnce(base, "Magic Eraser");
    const bool squareGone = qAlpha(f.view->sample_argb(9, 9)) == 0;
    const bool otherKept = f.view->sample_argb(29, 29) == red
        && f.view->sample_argb(1, 1) == QColor(Qt::white).rgba();

    pictura::MagicEraseOptions global = options;
    global.contiguous = false;
    f.tools->optionsFor<MagicEraseGroup>(ToolId::MagicEraser)->set(global);
    f.drag({QPointF(1, 1)});
    const bool whiteGone = f.view->history_index() == base + 2
        && qAlpha(f.view->sample_argb(1, 1)) == 0 && qAlpha(f.view->sample_argb(38, 38)) == 0
        && f.view->sample_argb(29, 29) == red;
    f.tools->optionsFor<MagicEraseGroup>(ToolId::MagicEraser)->set(options);

    ST_BEGIN("magic_eraser_tool");
    ST_PASS("magic eraser bar=%d commit=%d square=%d kept=%d global=%d", bar ? 1 : 0,
            committed ? 1 : 0, squareGone ? 1 : 0, otherKept ? 1 : 0, whiteGone ? 1 : 0);
    if (!bar || !committed || !squareGone || !otherKept || !whiteGone) {
        return pictura::selfTest().fail(551, "magic eraser tool");
    }
    return 0;
}
