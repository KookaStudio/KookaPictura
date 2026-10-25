// The Color Picker's hex shorthand/prefix parsing and Add to Swatches action
// (#603): the field accepts #/0x prefixes and three-digit shorthand, and the
// dialog appends its current colour to the Swatches grid.

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtGui/QColor>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

#include "color_picker_dialog.h"
#include "panels/swatches_panel.h"

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

QTEST_MAIN(ColorPickerTest)
#include "tst_color_picker.moc"
