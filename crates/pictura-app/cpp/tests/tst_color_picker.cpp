// The Color Picker's hex shorthand/prefix parsing and Add to Swatches action
// (#603): the field accepts #/0x prefixes and three-digit shorthand, and the
// dialog appends its current colour to the Swatches grid.

#include <QtTest/QtTest>

#include <QtCore/QElapsedTimer>
#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtGui/QCursor>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QColorDialog>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

#include "color_picker_dialog.h"
#include "panels/color_panel.h"
#include "panels/swatches_panel.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"
#include "qt_test_support.h"

#include <cmath>

namespace {

// A paste: the text lands in the field and the edit signal fires, without
// depending on the platform keyboard layout for '#' and 'x'.
void enterHex(QLineEdit* hex, const QString& text)
{
    hex->setText(text);
    QMetaObject::invokeMethod(hex, "textEdited", Q_ARG(QString, text));
}

QDialog* findSwatchNamePrompt(const QDialog& picker)
{
    if (auto* prompt = picker.findChild<QDialog*>(QStringLiteral("swatchNameDialog"))) {
        return prompt;
    }
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (widget->objectName() == QStringLiteral("swatchNameDialog")) {
            return qobject_cast<QDialog*>(widget);
        }
    }
    return nullptr;
}

} // namespace

class ColorPickerTest : public QObject {
    Q_OBJECT

private slots:
    void hexAcceptsPrefixes();
    void hexShorthandExpandsOnCommit();
    void invalidHexLeavesColor();
    void addToSwatchesAppendsCurrentColor();
    void addToSwatchesCancelsCleanly();
    void addToSwatchesIsDisabledWithoutPanel();
    void labEditUpdatesRgb();
    void webAlertAppearsAndSnaps();
    void gamutAlertAppliesClosest();
    void cmykFieldsUpdate();
    void alertsHiddenInGamut();
    void fieldDragUpdatesEverySample();
    void fieldCursorIsACircle();
    void defaultAdobePicker();
    void pickerPreferenceSurvivesRestart();
    void systemPickerReturnsColor();
    void colorPanelSwatchOpensPicker();
};

void ColorPickerTest::hexAcceptsPrefixes()
{
    pictura::ColorPickerDialog dialog(QColor(0, 0, 0));
    auto* hex = dialog.findChild<QLineEdit*>(QStringLiteral("colorPickerHex"));
    QVERIFY2(hex, "hex field present");

    enterHex(hex, QStringLiteral("#aabbcc"));
    QCOMPARE(dialog.selectedColor(), QColor(0xaa, 0xbb, 0xcc));

    enterHex(hex, QStringLiteral("0xAABBCC"));
    QCOMPARE(dialog.selectedColor(), QColor(0xaa, 0xbb, 0xcc));
}

void ColorPickerTest::hexShorthandExpandsOnCommit()
{
    pictura::ColorPickerDialog dialog(QColor(1, 2, 3));
    auto* hex = dialog.findChild<QLineEdit*>(QStringLiteral("colorPickerHex"));
    QVERIFY2(hex, "hex field present");

    enterHex(hex, QStringLiteral("#123"));
    QCOMPARE(dialog.selectedColor(), QColor(1, 2, 3));

    QMetaObject::invokeMethod(hex, "editingFinished");
    QCOMPARE(dialog.selectedColor(), QColor(0x11, 0x22, 0x33));
    QCOMPARE(hex->text(), QStringLiteral("112233"));
}

void ColorPickerTest::invalidHexLeavesColor()
{
    pictura::ColorPickerDialog dialog(QColor(1, 2, 3));
    auto* hex = dialog.findChild<QLineEdit*>(QStringLiteral("colorPickerHex"));
    QVERIFY2(hex, "hex field present");

    enterHex(hex, QStringLiteral("#12"));
    QMetaObject::invokeMethod(hex, "editingFinished");

    QCOMPARE(dialog.selectedColor(), QColor(1, 2, 3));
    QCOMPARE(hex->text(), QStringLiteral("010203"));
}

void ColorPickerTest::addToSwatchesAppendsCurrentColor()
{
    pictura::SwatchGrid grid;
    pictura::ColorPickerDialog dialog(QColor(0x12, 0x34, 0x56), nullptr, QString(), &grid);
    dialog.show();
    auto* button = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerAddToSwatches"));
    QVERIFY2(button && button->isEnabled(), "Add to Swatches enabled with a panel");

    QString suggested;
    QTimer::singleShot(0, &dialog, [&] {
        QDialog* prompt = findSwatchNamePrompt(dialog);
        if (!prompt) {
            return;
        }
        auto* edit = prompt->findChild<QLineEdit*>(QStringLiteral("swatchNameEdit"));
        if (!edit) {
            prompt->reject();
            return;
        }
        suggested = edit->text();
        edit->setText(QStringLiteral("Mine"));
        prompt->accept();
    });
    // Unblock the modal loop if the prompt was not found, so a wiring mistake
    // fails the assertions instead of hanging the suite.
    QTimer::singleShot(2000, &dialog, [&] {
        if (QDialog* prompt = findSwatchNamePrompt(dialog)) {
            prompt->reject();
        }
    });

    const int before = grid.swatches().size();
    button->click();

    QCOMPARE(suggested, QStringLiteral("Swatch 1"));
    QCOMPARE(grid.swatches().size(), before + 1);
    QCOMPARE(grid.swatches().last().color, QColor(0x12, 0x34, 0x56));
    QCOMPARE(grid.swatches().last().name, QStringLiteral("Mine"));
    QVERIFY2(dialog.isVisible(), "the picker stays open");
    dialog.hide();
}

void ColorPickerTest::addToSwatchesCancelsCleanly()
{
    pictura::SwatchGrid grid;
    pictura::ColorPickerDialog dialog(QColor(0x12, 0x34, 0x56), nullptr, QString(), &grid);
    auto* button = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerAddToSwatches"));
    QVERIFY2(button && button->isEnabled(), "Add to Swatches enabled with a panel");

    QTimer::singleShot(0, &dialog, [&] {
        if (QDialog* prompt = findSwatchNamePrompt(dialog)) {
            prompt->reject();
        }
    });
    QTimer::singleShot(2000, &dialog, [&] {
        if (QDialog* prompt = findSwatchNamePrompt(dialog)) {
            prompt->reject();
        }
    });

    const int before = grid.swatches().size();
    button->click();
    QCOMPARE(grid.swatches().size(), before);
}

void ColorPickerTest::addToSwatchesIsDisabledWithoutPanel()
{
    pictura::ColorPickerDialog dialog(QColor(0x12, 0x34, 0x56));
    auto* button = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerAddToSwatches"));
    QVERIFY2(button, "Add to Swatches button present");
    QVERIFY2(!button->isEnabled(), "disabled without a Swatches panel");
}

// Typing Lab values re-derives RGB and hex through the shared D50 conversion.
void ColorPickerTest::labEditUpdatesRgb()
{
    pictura::ColorPickerDialog dialog(QColor(0, 0, 0));
    auto* l = dialog.findChild<QSpinBox*>(QStringLiteral("colorPickerL"));
    auto* a = dialog.findChild<QSpinBox*>(QStringLiteral("colorPickerLabA"));
    auto* b = dialog.findChild<QSpinBox*>(QStringLiteral("colorPickerLabB"));
    auto* hex = dialog.findChild<QLineEdit*>(QStringLiteral("colorPickerHex"));
    QVERIFY(l && a && b && hex);
    {
        const QSignalBlocker blockL(l);
        const QSignalBlocker blockA(a);
        l->setValue(60);
        a->setValue(20);
    }
    b->setValue(-30);
    const QColor expected = pictura::labToRgb(60, 20, -30);
    QCOMPARE(dialog.selectedColor(), expected);
    QCOMPARE(hex->text(), expected.name().mid(1).toUpper());

    double back[3] = {0.0, 0.0, 0.0};
    pictura::rgbToLab(expected, &back[0], &back[1], &back[2]);
    QVERIFY(std::abs(back[0] - 60.0) < 1.0 && std::abs(back[1] - 20.0) < 1.5
            && std::abs(back[2] + 30.0) < 1.5);
}

void ColorPickerTest::webAlertAppearsAndSnaps()
{
    pictura::ColorPickerDialog dialog(QColor(1, 2, 3));
    auto* alert = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerWebAlert"));
    QVERIFY2(alert, "web-safe alert present");
    QVERIFY2(!alert->isHidden(), "a non-web-safe colour shows the alert");
    alert->click();
    QCOMPARE(dialog.selectedColor(), QColor(0, 0, 0));
}

void ColorPickerTest::gamutAlertAppliesClosest()
{
    pictura::ColorPickerDialog dialog(QColor(0, 0, 255));
    auto* alert = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerGamutAlert"));
    QVERIFY2(alert, "gamut alert present");
    QVERIFY2(!alert->isHidden(), "saturated blue is outside the CMYK gamut");
    const uint32_t closest = pictura::picker_closest_rgb(0, 0, 255);
    alert->click();
    QCOMPARE(dialog.selectedColor().rgb() & 0xFFFFFFu, QColor::fromRgb(QRgb(closest)).rgb() & 0xFFFFFFu);
}

void ColorPickerTest::cmykFieldsUpdate()
{
    pictura::ColorPickerDialog dialog(QColor(200, 50, 100));
    auto* cyan = dialog.findChild<QSpinBox*>(QStringLiteral("colorPickerC"));
    auto* magenta = dialog.findChild<QSpinBox*>(QStringLiteral("colorPickerM"));
    auto* yellow = dialog.findChild<QSpinBox*>(QStringLiteral("colorPickerY"));
    auto* black = dialog.findChild<QSpinBox*>(QStringLiteral("colorPickerK"));
    QVERIFY(cyan && magenta && yellow && black);
    const int m = magenta->value();
    const int y = yellow->value();
    const int k = black->value();
    cyan->setValue(50);
    const uint32_t expected = pictura::picker_rgb(50, m, y, k);
    QCOMPARE(dialog.selectedColor().rgb() & 0xFFFFFFu,
             QColor::fromRgb(QRgb(expected)).rgb() & 0xFFFFFFu);
    auto* alert = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerGamutAlert"));
    QVERIFY2(alert && alert->isHidden(), "a profile CMYK value is printable");
}

void ColorPickerTest::alertsHiddenInGamut()
{
    pictura::ColorPickerDialog dialog(QColor(255, 255, 255));
    auto* web = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerWebAlert"));
    auto* gamut = dialog.findChild<QPushButton*>(QStringLiteral("colorPickerGamutAlert"));
    QVERIFY(web && gamut);
    QVERIFY(web->isHidden());
    QVERIFY(gamut->isHidden());
}

// #1037: every drag sample re-ran the gamut test, which rebuilt two lcms2
// transforms per call. The readouts must still follow each sample, and the
// sample cost must stay interactive.
void ColorPickerTest::fieldDragUpdatesEverySample()
{
    // Pure red keeps the plane's hue at exactly 0 (the Hue axis): the field
    // then runs saturation along x and brightness down y.
    pictura::ColorPickerDialog dialog(QColor(255, 0, 0));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* plane = dialog.findChild<pictura::ColorPlane*>();
    auto* hex = dialog.findChild<QLineEdit*>(QStringLiteral("colorPickerHex"));
    QVERIFY(plane && hex);
    const int w = plane->width();
    const int h = plane->height();
    QVERIFY(w > 1 && h > 1);

    const auto expectedHex = [w, h](const QPoint& pos) {
        const auto to255 = [](double t) { return int(qBound(0.0, t, 1.0) * 255.0 + 0.5); };
        const QColor c =
            QColor::fromHsv(0, to255(pos.x() / double(w - 1)), to255(1.0 - pos.y() / double(h - 1)));
        return c.name().mid(1).toUpper();
    };
    const auto send = [plane](QEvent::Type type, const QPoint& pos, Qt::MouseButton button) {
        QMouseEvent event(type, QPointF(pos), QPointF(plane->mapToGlobal(pos)), button,
                          Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(plane, &event);
    };

    constexpr int kSamples = 200;
    QElapsedTimer timer;
    timer.start();
    send(QEvent::MouseButtonPress, QPoint(0, 0), Qt::LeftButton);
    for (int i = 1; i <= kSamples; ++i) {
        // A zig-zag diagonal: saturated (out-of-gamut) reds through greys.
        const QPoint pos((i * 7) % w, (i * (h - 1)) / kSamples);
        send(QEvent::MouseMove, pos, Qt::NoButton);
        QCoreApplication::processEvents();
        QCOMPARE(hex->text(), expectedHex(pos));
    }
    const qint64 elapsed = timer.elapsed();
    send(QEvent::MouseButtonRelease, QPoint(0, h - 1), Qt::LeftButton);
    qInfo().noquote() << "color picker drag:" << kSamples << "samples in" << elapsed << "ms";
    // ponytail: a wall-clock budget; generous so a loaded CI runner passes, and
    // far under the per-sample transform rebuilds it guards against.
    QVERIFY2(elapsed < 3000, qPrintable(QStringLiteral("%1 ms").arg(elapsed)));
    dialog.reject();
}

// The field's pointer is the ring the field paints, not the platform crosshair.
void ColorPickerTest::fieldCursorIsACircle()
{
    pictura::ColorPickerDialog dialog(QColor(255, 0, 0));
    auto* plane = dialog.findChild<pictura::ColorPlane*>();
    QVERIFY2(plane, "colour field present");
    const QCursor c = plane->cursor();
    QVERIFY2(!c.pixmap().isNull(), "the field loads the circular cursor asset");
    QCOMPARE(c.shape(), Qt::BitmapCursor);
}

void ColorPickerTest::defaultAdobePicker()
{
    pictura::ColorPickerDialog::setPickerPreference(0);
    QCOMPARE(pictura::ColorPickerDialog::pickerPreference(), 0);
}

void ColorPickerTest::pickerPreferenceSurvivesRestart()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    pictura::SessionState state;
    state.pickerChoice = 1;
    QVERIFY(pictura::saveSession(state));
    QCOMPARE(pictura::loadSession().pickerChoice, 1);
    pictura::ColorPickerDialog::setPickerPreference(0);
}

void ColorPickerTest::systemPickerReturnsColor()
{
    pictura::ColorPickerDialog::setPickerPreference(1);
    QTimer::singleShot(0, []() {
        auto* dialog = qobject_cast<QColorDialog*>(QApplication::activeModalWidget());
        if (!dialog) {
            QTimer::singleShot(20, []() {
                if (auto* late = qobject_cast<QColorDialog*>(QApplication::activeModalWidget())) {
                    late->setCurrentColor(QColor(10, 200, 30));
                    late->accept();
                }
            });
            return;
        }
        dialog->setCurrentColor(QColor(10, 200, 30));
        dialog->accept();
    });
    const QColor chosen = pictura::ColorPickerDialog::getColor(QColor(1, 2, 3));
    pictura::ColorPickerDialog::setPickerPreference(0);
    QCOMPARE(chosen, QColor(10, 200, 30));
}

void ColorPickerTest::colorPanelSwatchOpensPicker()
{
    pictura::ColorPickerDialog::setPickerPreference(0);
    pictura::ColorState state;
    pictura::ColorPanel panel(&state);
    auto* fg = panel.findChild<QPushButton*>(QStringLiteral("colorPanelForeground"));
    QVERIFY2(fg, "foreground swatch present");

    bool opened = false;
    QTimer timer;
    timer.setInterval(10);
    connect(&timer, &QTimer::timeout, [&]() {
        pictura::ColorPickerDialog* dialog = nullptr;
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* candidate = qobject_cast<pictura::ColorPickerDialog*>(widget)) {
                dialog = candidate;
                break;
            }
        }
        if (!dialog) {
            return;
        }
        auto* picker = dialog->findChild<pictura::ColorPicker*>();
        if (!picker) {
            timer.stop();
            dialog->reject();
            return;
        }
        picker->setColor(QColor(12, 34, 56));
        opened = true;
        timer.stop();
        dialog->accept();
    });
    timer.start();
    fg->click();
    timer.stop();
    QVERIFY2(opened, "the swatch opens the color picker");
    QCOMPARE(state.foreground(), QColor(12, 34, 56));
}

QTEST_MAIN(ColorPickerTest)
#include "tst_color_picker.moc"
