// Image > Adjustments > Replace Color (#164): the dialog samples the canvas,
// previews an HSL shift, and OK commits one "Replace Color" state while
// Cancel restores the layer exactly.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "replace_color_dialog.h"
#include "tools.h"

#include "api/view_bridge.h"

#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtWidgets/QSpinBox>

#include "qt_test_support.h"

class ReplaceColorTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void samplesAndApplies();
    void opensOnTheForegroundAndPicksAResult();
    void rejectsWhenDocumentChangesOrCloses();

private:
    bool openImage(const QColor& color);
    QAction* leaf() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
    pictura::PictureView* view_ = nullptr;
};

void ReplaceColorTest::initTestCase()
{
    QVERIFY(stateHome_.isValid() && dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ReplaceColorTest::cleanup()
{
    for (auto* dialog : window_->findChildren<pictura::ReplaceColorDialog*>()) {
        dialog->reject();
    }
    QCoreApplication::processEvents();
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool ReplaceColorTest::openImage(const QColor& color)
{
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(color);
    const QString path = dir_.filePath(QStringLiteral("photo.png"));
    if (!image.save(path) || !window_->openDocumentAtPath(path)) {
        return false;
    }
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    return view_ != nullptr;
}

QAction* ReplaceColorTest::leaf() const
{
    return window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Replace Color")}));
}

void ReplaceColorTest::samplesAndApplies()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    window_->registry()->refresh();
    QAction* action = leaf();
    QVERIFY(action && action->isEnabled());
    QCOMPARE(action->text(), QStringLiteral("Replace Color…"));

    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();
    action->trigger();
    QCoreApplication::processEvents();
    auto* dialog = window_->findChild<pictura::ReplaceColorDialog*>();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(dialog && dialog->isVisible() && tools);
    QVERIFY(tools->canvasSamplerActive());

    // A canvas click samples instead of reaching the active tool.
    emit window_->imageView()->mousePressed(QPointF(5, 5), Qt::LeftButton, 0);
    QCOMPARE(dialog->sampleCount(), 1);
    QVERIFY(!dialog->maskForTest().isNull());
    QCOMPARE(view_->history_count(), history);

    auto* hue = dialog->findChild<QSpinBox*>(QStringLiteral("replaceColorHue"));
    QVERIFY(hue);
    hue->setValue(120);
    QVERIFY(view_->composite_argb(5, 5) != before);

    dialog->accept();
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Replace Color"));
    QVERIFY(view_->composite_argb(5, 5) != before);
    QVERIFY(!tools->canvasSamplerActive());
    QVERIFY(!window_->findChild<pictura::ReplaceColorDialog*>());

    // Cancel restores the pre-preview pixels exactly and records nothing.
    const QRgb changed = view_->composite_argb(5, 5);
    action->trigger();
    QCoreApplication::processEvents();
    dialog = window_->findChild<pictura::ReplaceColorDialog*>();
    QVERIFY(dialog);
    emit window_->imageView()->mousePressed(QPointF(5, 5), Qt::LeftButton, 0);
    hue = dialog->findChild<QSpinBox*>(QStringLiteral("replaceColorHue"));
    hue->setValue(-60);
    QVERIFY(view_->composite_argb(5, 5) != changed);
    dialog->reject();
    QCoreApplication::processEvents();
    QCOMPARE(view_->composite_argb(5, 5), changed);
    QCOMPARE(view_->history_count(), history + 1);
}

// The dialog opens on the foreground colour (black here) as its sample, so
// the thumbnail and Lightness work before any click; picking a Result colour
// sets the shift that reaches it.
void ReplaceColorTest::opensOnTheForegroundAndPicksAResult()
{
    QVERIFY(openImage(QColor(0, 0, 0)));
    window_->registry()->refresh();
    leaf()->trigger();
    QCoreApplication::processEvents();
    auto* dialog = window_->findChild<pictura::ReplaceColorDialog*>();
    QVERIFY(dialog);
    QCOMPARE(dialog->sampleCount(), 1);
    const QImage mask = dialog->maskForTest();
    QVERIFY(!mask.isNull());
    QCOMPARE(qGray(mask.pixel(mask.width() / 2, mask.height() / 2)), 255);

    auto* lightness = dialog->findChild<QSpinBox*>(QStringLiteral("replaceColorLightness"));
    QVERIFY(lightness);
    lightness->setValue(50);
    QCOMPARE(qRed(view_->composite_argb(5, 5)), 128);
    dialog->pickResultForTest(Qt::white);
    QCOMPARE(lightness->value(), 100);
    QCOMPARE(dialog->resultForTest().rgb(), qRgb(255, 255, 255));
    QCOMPARE(view_->composite_argb(5, 5), qRgb(255, 255, 255));
    // Black is achromatic: as CS6 Help says, it cannot take a colour, so
    // picking blue moves only Lightness, and the swatch shows what the
    // canvas gets.
    dialog->pickResultForTest(QColor(0, 0, 255));
    QCOMPARE(dialog->findChild<QSpinBox*>(QStringLiteral("replaceColorHue"))->value(), 0);
    QCOMPARE(lightness->value(), 50);
    QCOMPARE(dialog->resultForTest().rgb(), qRgb(128, 128, 128));
    QCOMPARE(view_->composite_argb(5, 5), qRgb(128, 128, 128));
    dialog->reject();
}

void ReplaceColorTest::rejectsWhenDocumentChangesOrCloses()
{
    QVERIFY(openImage(QColor(100, 150, 200)));
    window_->registry()->refresh();
    QAction* action = leaf();
    QVERIFY(action && action->isEnabled());
    action->trigger();
    QCoreApplication::processEvents();
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(window_->findChild<pictura::ReplaceColorDialog*>() && tools);
    QVERIFY(tools->canvasSamplerActive());

    // Switching documents retargets the dock and must cancel the dialog before
    // it is left holding the previous document's raw view.
    QVERIFY(window_->newDocument(QStringLiteral("Second"), 20, 20, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QCoreApplication::processEvents();
    QVERIFY(!window_->findChild<pictura::ReplaceColorDialog*>());
    QVERIFY(!tools->canvasSamplerActive());

    // Closing the dialog's document must cancel it too.
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
    action->trigger();
    QCoreApplication::processEvents();
    QVERIFY(window_->findChild<pictura::ReplaceColorDialog*>());
    QVERIFY(tools->canvasSamplerActive());
    QVERIFY(window_->closeDocument(window_->activeDocumentIndex(), false));
    QCoreApplication::processEvents();
    QVERIFY(!window_->findChild<pictura::ReplaceColorDialog*>());
    QVERIFY(!tools->canvasSamplerActive());
}

QTEST_MAIN(ReplaceColorTest)
#include "tst_replace_color.moc"
