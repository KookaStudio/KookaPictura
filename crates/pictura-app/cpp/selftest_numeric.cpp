#include "selftest_numeric.h"
#include "selftest_report.h"

#include "frame.h"
#include "options_bar.h"
#include "panels/brush_preset_picker.h"
#include "panels/jump_slider.h"
#include "panels/numeric_field.h"
#include "tools.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QMetaObject>
#include <QtCore/QPoint>
#include <QtCore/QtGlobal>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

namespace pictura {

namespace {

NumericFieldConfig scrubConfig()
{
    NumericFieldConfig config;
    config.minimum = 0;
    config.maximum = 1000;
    config.step = 1;
    config.decimals = 0;
    config.namePrefix = QStringLiteral("scrub");
    return config;
}

} // namespace

int runNumericFieldChecks(pictura::PicturaMainWindow& frame)
{
        // lpn_scrub (303): dragging the label scrubs, Shift scales up 10x and
        // Ctrl down 10x, and each drag commits exactly once on release.
        NumericField scrub(QStringLiteral("Scrub"), scrubConfig());
        scrub.setValue(100);
        scrub.resize(240, 24);
        scrub.show();
        QApplication::processEvents();
        QLabel* scrubLabel = scrub.findChild<QLabel*>(QStringLiteral("scrubLabel"));

        int previews = 0;
        int commits = 0;
        QObject::connect(&scrub, &NumericField::valueChanged, [&previews](double) { ++previews; });
        QObject::connect(&scrub, &NumericField::valueCommitted, [&commits](double) { ++commits; });

        auto drag = [&](int dx, Qt::KeyboardModifiers mods) {
            scrub.setValue(100);
            const QPoint local(2, 2);
            const QPoint global = scrubLabel->mapToGlobal(local);
            QMouseEvent press(QEvent::MouseButtonPress, local, global, Qt::LeftButton,
                              Qt::LeftButton, mods);
            QCoreApplication::sendEvent(scrubLabel, &press);
            const QPoint moved(dx + 2, 2);
            const QPoint globalMoved = scrubLabel->mapToGlobal(moved);
            QMouseEvent move(QEvent::MouseMove, moved, globalMoved, Qt::NoButton,
                             Qt::LeftButton, mods);
            QCoreApplication::sendEvent(scrubLabel, &move);
            const double value = scrub.value();
            QMouseEvent release(QEvent::MouseButtonRelease, moved, globalMoved, Qt::LeftButton,
                                Qt::NoButton, mods);
            QCoreApplication::sendEvent(scrubLabel, &release);
            return value;
        };

        const double plain = drag(30, Qt::NoModifier);
        const double shifted = drag(30, Qt::ShiftModifier);
        const double ctrl = drag(30, Qt::ControlModifier);
        const bool scrubOk = qFuzzyCompare(plain, 130.0) && qFuzzyCompare(shifted, 400.0)
            && qFuzzyCompare(ctrl, 103.0) && previews >= 3 && commits == 3;
        ST_BEGIN("lpn_scrub");
        ST_PASS("lpn_scrub plain=%.0f shift=%.0f ctrl=%.0f previews=%d commits=%d", plain, shifted,
                ctrl, previews, commits);
        if (!scrubOk) {
            return pictura::selfTest().fail(303, "numeric field scrub");
        }
        scrub.hide();

        // lpn_popup_keys (304): Left/Right step one unit, Home/End jump to the
        // ends, and PageUp/PageDown move one page, all through the popup filter.
        NumericFieldConfig popupConfig;
        popupConfig.minimum = 0;
        popupConfig.maximum = 100;
        popupConfig.step = 1;
        popupConfig.page = 10;
        popupConfig.popup = true;
        popupConfig.namePrefix = QStringLiteral("pop");
        NumericField popupField(QStringLiteral("Popup"), popupConfig);
        popupField.setValue(50);
        popupField.resize(200, 24);
        popupField.show();
        QApplication::processEvents();
        if (auto* arrow = popupField.findChild<QToolButton*>(QStringLiteral("popArrow"))) {
            arrow->click();
        }
        QApplication::processEvents();
        QWidget* popup = popupField.findChild<QWidget*>(QStringLiteral("popPopup"));
        const auto press = [popup](int key) {
            QKeyEvent event(QEvent::KeyPress, key, Qt::NoModifier);
            QCoreApplication::sendEvent(popup, &event);
        };
        bool keysOk = popup != nullptr;
        if (popup) {
            press(Qt::Key_Right);
            const bool rightOk = qRound(popupField.value()) == 51;
            press(Qt::Key_Left);
            const bool leftOk = qRound(popupField.value()) == 50;
            press(Qt::Key_End);
            const bool endOk = qRound(popupField.value()) == 100;
            press(Qt::Key_Home);
            const bool homeOk = qRound(popupField.value()) == 0;
            press(Qt::Key_PageUp);
            const bool pageUpOk = qRound(popupField.value()) == 10;
            press(Qt::Key_PageDown);
            const bool pageDownOk = qRound(popupField.value()) == 0;
            keysOk = rightOk && leftOk && endOk && homeOk && pageUpOk && pageDownOk;
        }
        ST_BEGIN("lpn_popup_keys");
        ST_PASS("lpn_popup_keys popup=%d ok=%d", popup ? 1 : 0, keysOk ? 1 : 0);
        if (!keysOk) {
            return pictura::selfTest().fail(304, "popup key handling");
        }
        popupField.hide();

        // lpn_jump_track (305): a press on the tracking slider jumps to the
        // point and a held drag follows the pointer instead of page-stepping.
        JumpSlider slider(Qt::Horizontal);
        slider.setRange(0, 100);
        slider.setMinimumWidth(120);
        slider.resize(200, 20);
        slider.show();
        QApplication::processEvents();
        const int width = qMax(1, slider.width());
        const int y = qMax(1, slider.height()) / 2;
        const auto at = [width, y](int fraction) {
            return QPoint(qBound(0, width * fraction / 1000, width - 1), y);
        };
        const QPoint pressPoint = at(100);
        QMouseEvent sliderPress(QEvent::MouseButtonPress, pressPoint,
                                slider.mapToGlobal(pressPoint), Qt::LeftButton, Qt::LeftButton,
                                Qt::NoModifier);
        QCoreApplication::sendEvent(&slider, &sliderPress);
        const int afterPress = slider.value();
        const QPoint movePoint = at(800);
        QMouseEvent sliderMove(QEvent::MouseMove, movePoint, slider.mapToGlobal(movePoint),
                               Qt::NoButton, Qt::LeftButton, Qt::NoModifier);
        QCoreApplication::sendEvent(&slider, &sliderMove);
        const int afterMove = slider.value();
        QMouseEvent sliderRelease(QEvent::MouseButtonRelease, movePoint,
                                  slider.mapToGlobal(movePoint), Qt::LeftButton, Qt::NoButton,
                                  Qt::NoModifier);
        QCoreApplication::sendEvent(&slider, &sliderRelease);
        const bool trackOk =
            afterPress >= 5 && afterPress <= 25 && afterMove >= 70 && afterMove <= 95
            && afterMove > afterPress;
        ST_BEGIN("lpn_jump_track");
        ST_PASS("lpn_jump_track press=%d move=%d", afterPress, afterMove);
        if (!trackOk) {
            return pictura::selfTest().fail(305, "tracking slider drag");
        }
        slider.hide();

        // las_no_scratch (306): the launch gate seeds the scratch document only
        // for the self-test path and only when no document was loaded.
        const bool noScratch = !pictura::launchCreatesScratchDocument(false, false);
        const bool selfTestScratch = pictura::launchCreatesScratchDocument(true, false);
        const bool codecScratch = !pictura::launchCreatesScratchDocument(true, true);
        ST_BEGIN("las_no_scratch");
        ST_PASS("las_no_scratch normal=%d selftest=%d codec=%d", noScratch ? 1 : 0,
                selfTestScratch ? 1 : 0, codecScratch ? 1 : 0);
        if (!noScratch || !selfTestScratch || !codecScratch) {
            return pictura::selfTest().fail(306, "scratch document gating");
        }

        // lpn_brush_resync (307): the brush-size signal updates the tip button
        // and the Brush Preset picker it opens, and the picker's Size box
        // drives the controller back.
        auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
        auto* tipButton = frame.findChild<QToolButton*>(QStringLiteral("optionsBrushTip"));
        ToolController* tools = bar ? bar->controllerForTest() : nullptr;
        QSpinBox* sizeField = nullptr;
        bool resyncOk = false;
        if (tools && tipButton) {
            tipButton->click();
            sizeField = bar->brushPicker()->findChild<QSpinBox*>(QStringLiteral("brushPickerSize"));
            tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(77);
            const bool fromController = sizeField && sizeField->value() == 77
                && tipButton->text() == QStringLiteral("77");
            if (sizeField) {
                sizeField->setValue(55);
            }
            resyncOk = fromController && tools->optionsFor<PaintOptions>(ToolId::Brush)->size() == 55;
            bar->brushPicker()->hide();
        }
        ST_BEGIN("lpn_brush_resync");
        ST_PASS("lpn_brush_resync bar=%d field=%d ok=%d", bar ? 1 : 0, sizeField ? 1 : 0,
                resyncOk ? 1 : 0);
        if (!resyncOk) {
            return pictura::selfTest().fail(307, "brush size resync");
        }

        return 0;
}

} // namespace pictura
