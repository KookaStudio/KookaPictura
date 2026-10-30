// The canvas checkerboard is the only backdrop: a transparent document shows
// it, opaque content covers it, and hiding a Background reveals it (#900).

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "qt_test_support.h"

class CanvasBackdropTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void transparentDocumentShowsCheckerboard();
    void opaqueContentCoversCheckerboard();
    void hiddenBackgroundShowsCheckerboard();

private:
    bool newDoc(const QString& name, const QString& background);
    QColor centerPixel() const;
    bool checker(const QColor& color) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::ImageView* canvas_ = nullptr;
};

void CanvasBackdropTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(420, 320);
    window_->show();
    QCoreApplication::processEvents();
}

void CanvasBackdropTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    canvas_ = nullptr;
}

bool CanvasBackdropTest::newDoc(const QString& name, const QString& background)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8, background)) {
        return false;
    }
    view_ = window_->activeView();
    canvas_ = window_->imageView();
    // settle: lets the new document's 120 ms panel refresh land; no hook marks it
    QTest::qWait(150);
    QCoreApplication::processEvents();
    if (canvas_) {
        canvas_->repaint();
        QCoreApplication::processEvents();
    }
    return view_ && canvas_;
}

bool CanvasBackdropTest::checker(const QColor& color) const
{
    return color == pictura::ImageView::transparencyColorA()
           || color == pictura::ImageView::transparencyColorB();
}

QColor CanvasBackdropTest::centerPixel() const
{
    canvas_->repaint();
    QCoreApplication::processEvents();
    const QImage image = canvas_->grab().toImage();
    return image.pixelColor(image.rect().center());
}

void CanvasBackdropTest::transparentDocumentShowsCheckerboard()
{
    QVERIFY(newDoc(QStringLiteral("Transparent"), QStringLiteral("transparent")));
    QVERIFY2(checker(centerPixel()), "a transparent document shows the checkerboard");
}

void CanvasBackdropTest::opaqueContentCoversCheckerboard()
{
    QVERIFY(newDoc(QStringLiteral("Opaque"), QStringLiteral("white")));
    const QImage composite = view_->image();
    QVERIFY(!composite.isNull());
    QCOMPARE(composite.pixelColor(8, 8).alpha(), 255);
    QCOMPARE(centerPixel(), QColor(Qt::white));
}

void CanvasBackdropTest::hiddenBackgroundShowsCheckerboard()
{
    QVERIFY(newDoc(QStringLiteral("HideBg"), QStringLiteral("white")));
    QCOMPARE(view_->image().pixelColor(8, 8).alpha(), 255);
    QVERIFY(view_->set_layers_visible(QStringList{QStringLiteral("0")}, false) == 1);
    QCoreApplication::processEvents();
    canvas_->repaint();
    QCoreApplication::processEvents();
    QTRY_COMPARE(view_->image().pixelColor(8, 8).alpha(), 0);
    QTRY_VERIFY2(checker(centerPixel()), "hiding the Background reveals the checkerboard");
}

QTEST_MAIN(CanvasBackdropTest)
#include "tst_canvas_backdrop.moc"
