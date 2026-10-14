// Frameless dialog chrome (ui/dialog-chrome): every Qt-drawn dialog loses the
// window-manager frame and gains a drawn title bar, outline, move, and edge
// resize; platform pickers keep their frame.

#include <QtTest/QtTest>

#include <QtGui/QMouseEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include "dialog_chrome.h"
#include "new_document_dialog.h"

namespace {

// A plain resizable dialog with one field and one button.
std::unique_ptr<QDialog> makeDialog(const QString& title)
{
    auto dialog = std::make_unique<QDialog>();
    dialog->setWindowTitle(title);
    auto* layout = new QVBoxLayout(dialog.get());
    layout->addWidget(new QLineEdit(dialog.get()));
    layout->addWidget(new QPushButton(QStringLiteral("OK"), dialog.get()));
    dialog->resize(320, 200);
    return dialog;
}

QLabel* titleLabel(const QDialog& dialog)
{
    QWidget* bar = pictura::dialogTitleBar(&dialog);
    return bar ? bar->findChild<QLabel*>(QStringLiteral("dialogTitleLabel")) : nullptr;
}

void sendMouse(QWidget* target, QEvent::Type type, QPoint local, Qt::MouseButton button,
               Qt::MouseButtons buttons)
{
    QMouseEvent event(type, QPointF(local), QPointF(target->mapToGlobal(local)), button, buttons,
                      Qt::NoModifier);
    QCoreApplication::sendEvent(target, &event);
}

} // namespace

class DialogChromeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase() { pictura::installDialogChrome(); }
    void appDialogIsFrameless();
    void contentStaysBelowTitleBar();
    void closeButtonRejects();
    void titleFollowsWindowTitle();
    void dragMovesDialog();
    void resizeEdgesOnlyWhenResizable();
    void pickersAndExemptions();
    void messageBoxGetsChrome();
};

void DialogChromeTest::appDialogIsFrameless()
{
    pictura::NewDocumentDialog dialog(QStringLiteral("Untitled-1"), QColor(Qt::white));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QVERIFY(dialog.windowFlags().testFlag(Qt::FramelessWindowHint));
    QWidget* bar = pictura::dialogTitleBar(&dialog);
    QVERIFY(bar);
    QVERIFY(bar->isVisible());
    QCOMPARE(bar->geometry().topLeft(), QPoint(0, 0));
    QCOMPARE(bar->width(), dialog.width());
    QVERIFY(titleLabel(dialog));
    QCOMPARE(titleLabel(dialog)->text(), dialog.windowTitle());
    QWidget* outline = dialog.findChild<QWidget*>(QStringLiteral("dialogFrame"));
    QVERIFY(outline);
    QCOMPARE(outline->geometry(), dialog.rect());
    QVERIFY(outline->testAttribute(Qt::WA_TransparentForMouseEvents));
}

void DialogChromeTest::contentStaysBelowTitleBar()
{
    pictura::NewDocumentDialog dialog(QStringLiteral("Untitled-1"), QColor(Qt::white));
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QWidget* bar = pictura::dialogTitleBar(&dialog);
    QVERIFY(bar);
    QLayout* layout = dialog.layout();
    QVERIFY(layout);
    int checked = 0;
    for (int i = 0; i < layout->count(); ++i) {
        const QRect cell = layout->itemAt(i)->geometry();
        if (cell.isValid()) {
            QVERIFY2(cell.top() >= bar->height(), qPrintable(QStringLiteral("item %1").arg(i)));
            ++checked;
        }
    }
    QVERIFY(checked > 0);
}

void DialogChromeTest::closeButtonRejects()
{
    auto dialog = makeDialog(QStringLiteral("Close Me"));
    dialog->show();
    QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));
    auto* close = dialog->findChild<QToolButton*>(QStringLiteral("dialogCloseButton"));
    QVERIFY(close);
    close->click();
    QCOMPARE(dialog->result(), int(QDialog::Rejected));
    QVERIFY(!dialog->isVisible());
}

void DialogChromeTest::titleFollowsWindowTitle()
{
    auto dialog = makeDialog(QStringLiteral("Before"));
    dialog->show();
    QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));
    QCOMPARE(titleLabel(*dialog)->text(), QStringLiteral("Before"));
    dialog->setWindowTitle(QStringLiteral("After"));
    QCOMPARE(titleLabel(*dialog)->text(), QStringLiteral("After"));
}

void DialogChromeTest::dragMovesDialog()
{
    auto dialog = makeDialog(QStringLiteral("Drag"));
    dialog->move(100, 100);
    dialog->show();
    QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));
    QWidget* bar = pictura::dialogTitleBar(dialog.get());
    QVERIFY(bar);
    const QPoint start = dialog->pos();
    const QPoint grab(20, bar->height() / 2);
    const QPoint target = bar->mapToGlobal(grab) + QPoint(30, 40);
    sendMouse(bar, QEvent::MouseButtonPress, grab, Qt::LeftButton, Qt::LeftButton);
    QMouseEvent move(QEvent::MouseMove, QPointF(bar->mapFromGlobal(target)), QPointF(target),
                     Qt::NoButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(bar, &move);
    sendMouse(bar, QEvent::MouseButtonRelease, bar->mapFromGlobal(target), Qt::LeftButton,
              Qt::NoButton);
    QCOMPARE(dialog->pos(), start + QPoint(30, 40));
}

void DialogChromeTest::resizeEdgesOnlyWhenResizable()
{
    auto resizable = makeDialog(QStringLiteral("Resizable"));
    resizable->show();
    QVERIFY(QTest::qWaitForWindowExposed(resizable.get()));
    const QPoint rightEdge(resizable->width() - 2, resizable->height() / 2);
    sendMouse(resizable.get(), QEvent::MouseMove, rightEdge, Qt::NoButton, Qt::NoButton);
    QCOMPARE(resizable->cursor().shape(), Qt::SizeHorCursor);
    const QPoint corner(resizable->width() - 2, resizable->height() - 2);
    sendMouse(resizable.get(), QEvent::MouseMove, corner, Qt::NoButton, Qt::NoButton);
    QCOMPARE(resizable->cursor().shape(), Qt::SizeFDiagCursor);
    sendMouse(resizable.get(), QEvent::MouseMove, QPoint(resizable->width() / 2, 60), Qt::NoButton,
              Qt::NoButton);
    QCOMPARE(resizable->cursor().shape(), Qt::ArrowCursor);

    auto fixed = makeDialog(QStringLiteral("Fixed"));
    fixed->setFixedSize(320, 200);
    fixed->show();
    QVERIFY(QTest::qWaitForWindowExposed(fixed.get()));
    sendMouse(fixed.get(), QEvent::MouseMove, QPoint(fixed->width() - 2, 100), Qt::NoButton,
              Qt::NoButton);
    QCOMPARE(fixed->cursor().shape(), Qt::ArrowCursor);
}

void DialogChromeTest::pickersAndExemptions()
{
    // Offscreen has no native helper, so both pickers are Qt-drawn and get
    // the chrome, with or without DontUseNativeDialog.
    QFileDialog drawn;
    drawn.setOption(QFileDialog::DontUseNativeDialog, true);
    drawn.show();
    QVERIFY(QTest::qWaitForWindowExposed(&drawn));
    QVERIFY(pictura::dialogTitleBar(&drawn));
    QVERIFY(drawn.windowFlags().testFlag(Qt::FramelessWindowHint));
    drawn.reject();

    QFileDialog fallback;
    fallback.show();
    QVERIFY(QTest::qWaitForWindowExposed(&fallback));
    QVERIFY(pictura::dialogTitleBar(&fallback));
    fallback.reject();

    QDialog popup(nullptr, Qt::Popup);
    popup.ensurePolished();
    QVERIFY(!pictura::dialogTitleBar(&popup));
    QDialog bare(nullptr, Qt::Dialog | Qt::FramelessWindowHint);
    bare.ensurePolished();
    QVERIFY(!pictura::dialogTitleBar(&bare));
}

void DialogChromeTest::messageBoxGetsChrome()
{
    QMessageBox box(QMessageBox::Warning, QStringLiteral("Alert"), QStringLiteral("Body text"),
                    QMessageBox::Ok);
    box.show();
    QVERIFY(QTest::qWaitForWindowExposed(&box));
    QWidget* bar = pictura::dialogTitleBar(&box);
    QVERIFY(bar);
    QVERIFY(box.windowFlags().testFlag(Qt::FramelessWindowHint));
    QVERIFY(box.layout());
    QVERIFY(box.layout()->contentsMargins().top() >= bar->height());
    auto* text = box.findChild<QLabel*>(QStringLiteral("qt_msgbox_label"));
    QVERIFY(text);
    QVERIFY(text->geometry().top() >= bar->height());
}

QTEST_MAIN(DialogChromeTest)
#include "tst_dialog_chrome.moc"
