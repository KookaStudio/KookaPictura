// Enter in a focused value input commits and deactivates it; Shift+Enter keeps
// the value selected; a dialog's default button waits for an unfocused field;
// multi-line editors keep Enter for a new line.

#include <QtTest/QtTest>

#include "fill_dialog.h"
#include "layer_new_dialog.h"
#include "panels/numeric_field.h"
#include "value_input_enter.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

class ValueInputEnterTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void numericFieldEnterCommitsAndLeaves();
    void numericFieldInvalidEnterRevertsAndLeaves();
    void numericFieldShiftEnterKeepsFocusAndHighlights();
    void spinBoxEnterCommitsAndLeaves();
    void spinBoxInvalidEnterRevertsAndLeaves();
    void spinBoxShiftEnterKeepsFocusAndHighlights();
    void editableComboEnterCommitsAndLeaves();
    void dialogLineEditEnterCommitsAndLeaves();
    void dialogFieldEnterDoesNotPressDefaultButton();
    void dialogNumericFieldEnterDoesNotPressDefaultButton();
    void plainShellLineEditKeepsEnter();
    void multiLineEditorKeepsEnterForNewline();

private:
    static void showAndFocus(QWidget* widget)
    {
        QVERIFY(QTest::qWaitForWindowExposed(widget));
        widget->activateWindow();
        QCoreApplication::processEvents();
    }

    static QLineEdit* editOf(QWidget* field)
    {
        return field->findChild<QLineEdit*>();
    }

    static pictura::NumericField* makeField(QWidget* parent, double value)
    {
        pictura::NumericFieldConfig config;
        config.namePrefix = QStringLiteral("valueInput");
        config.minimum = 0.0;
        config.maximum = 100.0;
        auto* field = new pictura::NumericField(QString(), config, parent);
        field->setValue(value);
        return field;
    }
};

void ValueInputEnterTest::initTestCase()
{
    pictura::installValueInputEnter();
}

void ValueInputEnterTest::numericFieldEnterCommitsAndLeaves()
{
    QWidget host;
    auto* layout = new QVBoxLayout(&host);
    auto* field = makeField(&host, 10.0);
    layout->addWidget(field);
    host.show();
    showAndFocus(&host);

    QSignalSpy committed(field, &pictura::NumericField::valueCommitted);
    QLineEdit* edit = editOf(field);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("42"));
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(field->value(), 42.0);
    QCOMPARE(committed.count(), 1);
    QVERIFY(!edit->hasFocus());
}

void ValueInputEnterTest::numericFieldInvalidEnterRevertsAndLeaves()
{
    QWidget host;
    auto* layout = new QVBoxLayout(&host);
    auto* field = makeField(&host, 42.0);
    layout->addWidget(field);
    host.show();
    showAndFocus(&host);

    QSignalSpy committed(field, &pictura::NumericField::valueCommitted);
    QLineEdit* edit = editOf(field);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->clear();
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(field->value(), 42.0);
    QCOMPARE(edit->text(), QStringLiteral("42"));
    QCOMPARE(committed.count(), 0);
    QVERIFY(!edit->hasFocus());
}

void ValueInputEnterTest::numericFieldShiftEnterKeepsFocusAndHighlights()
{
    QWidget host;
    auto* layout = new QVBoxLayout(&host);
    auto* field = makeField(&host, 42.0);
    layout->addWidget(field);
    host.show();
    showAndFocus(&host);

    QLineEdit* edit = editOf(field);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("50"));
    QTest::keyClick(edit, Qt::Key_Return, Qt::ShiftModifier);

    QCOMPARE(field->value(), 50.0);
    QVERIFY(edit->hasFocus());
    QCOMPARE(edit->selectedText(), QStringLiteral("50"));
}

void ValueInputEnterTest::spinBoxEnterCommitsAndLeaves()
{
    QDialog dialog;
    auto* layout = new QVBoxLayout(&dialog);
    auto* spin = new QSpinBox(&dialog);
    spin->setRange(1, 100);
    spin->setValue(10);
    spin->setKeyboardTracking(false);
    layout->addWidget(spin);
    auto* ok = new QPushButton(QStringLiteral("OK"), &dialog);
    ok->setDefault(true);
    layout->addWidget(ok);
    connect(ok, &QPushButton::clicked, &dialog, &QDialog::accept);
    dialog.show();
    showAndFocus(&dialog);

    QSignalSpy accepted(&dialog, &QDialog::accepted);
    QLineEdit* edit = editOf(spin);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("55"));
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(spin->value(), 55);
    QCOMPARE(accepted.count(), 0);
    QVERIFY(!edit->hasFocus());
}

void ValueInputEnterTest::spinBoxInvalidEnterRevertsAndLeaves()
{
    QDialog dialog;
    auto* layout = new QVBoxLayout(&dialog);
    auto* spin = new QSpinBox(&dialog);
    spin->setRange(1, 100);
    spin->setValue(10);
    layout->addWidget(spin);
    dialog.show();
    showAndFocus(&dialog);

    QLineEdit* edit = editOf(spin);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->clear();
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(spin->value(), 10);
    QCOMPARE(spin->text().trimmed(), QStringLiteral("10"));
    QVERIFY(!edit->hasFocus());
}

void ValueInputEnterTest::spinBoxShiftEnterKeepsFocusAndHighlights()
{
    QDialog dialog;
    auto* layout = new QVBoxLayout(&dialog);
    auto* spin = new QSpinBox(&dialog);
    spin->setRange(1, 100);
    spin->setValue(10);
    layout->addWidget(spin);
    dialog.show();
    showAndFocus(&dialog);

    QLineEdit* edit = editOf(spin);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("55"));
    QTest::keyClick(edit, Qt::Key_Return, Qt::ShiftModifier);

    QCOMPARE(spin->value(), 55);
    QVERIFY(edit->hasFocus());
    QCOMPARE(edit->selectedText(), QStringLiteral("55"));
}

void ValueInputEnterTest::editableComboEnterCommitsAndLeaves()
{
    QWidget host;
    auto* layout = new QVBoxLayout(&host);
    auto* combo = new QComboBox(&host);
    combo->setEditable(true);
    combo->setInsertPolicy(QComboBox::NoInsert);
    combo->addItems({QStringLiteral("10"), QStringLiteral("20")});
    layout->addWidget(combo);
    host.show();
    showAndFocus(&host);

    QLineEdit* edit = combo->lineEdit();
    QVERIFY(edit);
    QSignalSpy finished(edit, &QLineEdit::editingFinished);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("55"));
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(combo->currentText(), QStringLiteral("55"));
    QVERIFY(finished.count() >= 1);
    QVERIFY(!edit->hasFocus());
}

void ValueInputEnterTest::dialogLineEditEnterCommitsAndLeaves()
{
    QDialog dialog;
    auto* layout = new QVBoxLayout(&dialog);
    auto* edit = new QLineEdit(&dialog);
    layout->addWidget(edit);
    auto* ok = new QPushButton(QStringLiteral("OK"), &dialog);
    ok->setDefault(true);
    layout->addWidget(ok);
    connect(ok, &QPushButton::clicked, &dialog, &QDialog::accept);
    dialog.show();
    showAndFocus(&dialog);

    QSignalSpy accepted(&dialog, &QDialog::accepted);
    QSignalSpy finished(edit, &QLineEdit::editingFinished);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    QTest::keyClicks(edit, QStringLiteral("Name"));
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(edit->text(), QStringLiteral("Name"));
    QVERIFY(finished.count() >= 1);
    QCOMPARE(accepted.count(), 0);
    QVERIFY(!edit->hasFocus());
}

void ValueInputEnterTest::dialogFieldEnterDoesNotPressDefaultButton()
{
    pictura::FillDialog dialog(QColor(10, 20, 30), QColor(200, 210, 220));
    dialog.show();
    showAndFocus(&dialog);

    auto* opacity = dialog.findChild<QSpinBox*>(QStringLiteral("fillOpacity"));
    QVERIFY(opacity);
    QPushButton* ok = nullptr;
    for (QPushButton* button : dialog.findChildren<QPushButton*>()) {
        if (button->isDefault()) {
            ok = button;
            break;
        }
    }
    QVERIFY(ok);
    QSignalSpy accepted(&dialog, &QDialog::accepted);

    QLineEdit* edit = editOf(opacity);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("40"));
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(dialog.opacity(), 40);
    QCOMPARE(accepted.count(), 0);
    QVERIFY(dialog.isVisible());
    QVERIFY(!edit->hasFocus());

    // With no value input focused, Enter reaches the default button.
    QTest::keyClick(&dialog, Qt::Key_Return);
    QCOMPARE(accepted.count(), 1);
    QCOMPARE(dialog.result(), int(QDialog::Accepted));
}

void ValueInputEnterTest::dialogNumericFieldEnterDoesNotPressDefaultButton()
{
    pictura::LayerNewDialog dialog(false);
    dialog.show();
    showAndFocus(&dialog);

    auto* opacity = dialog.findChild<pictura::NumericField*>(QStringLiteral("layerNewOpacity"));
    QVERIFY(opacity);
    auto* buttons = dialog.findChild<QDialogButtonBox*>();
    QVERIFY(buttons);
    QPushButton* ok = buttons->button(QDialogButtonBox::Ok);
    QVERIFY(ok);
    ok->setDefault(true);
    QSignalSpy accepted(&dialog, &QDialog::accepted);

    QLineEdit* edit = editOf(opacity);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    edit->selectAll();
    QTest::keyClicks(edit, QStringLiteral("128"));
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(dialog.spec().opacity, 128);
    QCOMPARE(accepted.count(), 0);
    QVERIFY(dialog.isVisible());
    QVERIFY(!edit->hasFocus());

    QTest::keyClick(&dialog, Qt::Key_Return);
    QCOMPARE(accepted.count(), 1);
    QCOMPARE(dialog.result(), int(QDialog::Accepted));
}

void ValueInputEnterTest::plainShellLineEditKeepsEnter()
{
    QWidget host;
    auto* layout = new QVBoxLayout(&host);
    auto* edit = new QLineEdit(&host);
    layout->addWidget(edit);
    host.show();
    showAndFocus(&host);

    QSignalSpy returned(edit, &QLineEdit::returnPressed);
    edit->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(edit->hasFocus());
    QTest::keyClicks(edit, QStringLiteral("Name"));
    QTest::keyClick(edit, Qt::Key_Return);

    QCOMPARE(returned.count(), 1);
    QVERIFY(edit->hasFocus());
}

void ValueInputEnterTest::multiLineEditorKeepsEnterForNewline()
{
    QDialog dialog;
    auto* layout = new QVBoxLayout(&dialog);
    auto* text = new QPlainTextEdit(&dialog);
    layout->addWidget(text);
    auto* ok = new QPushButton(QStringLiteral("OK"), &dialog);
    ok->setDefault(true);
    layout->addWidget(ok);
    connect(ok, &QPushButton::clicked, &dialog, &QDialog::accept);
    dialog.show();
    showAndFocus(&dialog);

    QSignalSpy accepted(&dialog, &QDialog::accepted);
    text->setFocus();
    QCoreApplication::processEvents();
    QVERIFY(text->hasFocus());
    QTest::keyClicks(text, QStringLiteral("ab"));
    QTest::keyClick(text, Qt::Key_Return);
    QTest::keyClicks(text, QStringLiteral("c"));

    QCOMPARE(text->toPlainText(), QStringLiteral("ab\nc"));
    QCOMPARE(accepted.count(), 0);
    QVERIFY(text->hasFocus());
}

QTEST_MAIN(ValueInputEnterTest)
#include "tst_value_input_enter.moc"
