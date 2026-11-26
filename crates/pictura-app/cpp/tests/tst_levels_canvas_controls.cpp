#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/paint_bridge.h"
#include "frame.h"
#include "image_view.h"
#include "levels_dialog.h"
#include "qt_test_support.h"
#include "tools.h"

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QContextMenuEvent>
#include <QtGui/QMouseEvent>
#include <QtTest/QtTest>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QMenu>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

namespace {

QByteArray levelsBlock()
{
    const auto block = pictura::image_adjustment_default(
        QStringLiteral("levels"), 0, 0xffffff, false);
    return QByteArray(reinterpret_cast<const char*>(block.data()), qsizetype(block.size()));
}

void sendDrag(QWidget* widget, QEvent::Type type, int level, Qt::KeyboardModifiers modifiers)
{
    const QPoint local(level + pictura::LevelsStrip::kInset, widget->height() / 2);
    const auto button = type == QEvent::MouseMove ? Qt::NoButton : Qt::LeftButton;
    const auto held = type == QEvent::MouseButtonRelease ? Qt::NoButton : Qt::LeftButton;
    QMouseEvent event(type, local, widget->mapToGlobal(local), button, held, modifiers);
    QApplication::sendEvent(widget, &event);
}

QToolButton* dropper(pictura::LevelsDialog& dialog, const char* point)
{
    return dialog.findChild<QToolButton*>(
        QStringLiteral("levels") + QString::fromLatin1(point) + QStringLiteral("Point"));
}

bool setTarget(pictura::LevelsDialog& dialog, QToolButton* button, int value)
{
    bool set = false;
    QTimer edit;
    edit.setSingleShot(true);
    QObject::connect(&edit, &QTimer::timeout, &dialog, [&] {
        if (auto* target = dialog.findChild<QInputDialog*>(QStringLiteral("levelsPointTarget"))) {
            target->setIntValue(value);
            target->accept();
            set = true;
        } else if (auto* unexpected = qobject_cast<QDialog*>(QApplication::activeModalWidget())) {
            unexpected->reject();
        }
    });
    edit.start(0);
    QTest::mouseDClick(button, Qt::LeftButton);
    return set;
}

} // namespace

class LevelsCanvasControlsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void levelsPickEndpoints();
    void levelsPickGray();
    void levelsPickLimits();
    void levelsAltDragClips();
    void levelsOtherDragsStayNormal();

private:
    bool open(const QImage& image);
    std::unique_ptr<pictura::LevelsDialog> makeDialog();
    void pick(const QPoint& pixel);

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::ToolController* tools_ = nullptr;
    pictura::ImageView* canvas_ = nullptr;
    int cacheLevels_ = 8;
};

void LevelsCanvasControlsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid() && dir_.isValid());
    cacheLevels_ = pictura::cache_levels();
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
    window_->resize(900, 700);
    window_->show();
    tools_ = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools_);
}

void LevelsCanvasControlsTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    canvas_ = nullptr;
    pictura::set_cache_levels(cacheLevels_);
}

bool LevelsCanvasControlsTest::open(const QImage& image)
{
    const QString path = dir_.filePath(QStringLiteral("levels.png"));
    if (!image.save(path) || !window_->openDocumentAtPath(path)) {
        return false;
    }
    view_ = window_->activeView();
    canvas_ = window_->imageView();
    QCoreApplication::processEvents();
    return view_ && canvas_;
}

std::unique_ptr<pictura::LevelsDialog> LevelsCanvasControlsTest::makeDialog()
{
    auto dialog = std::make_unique<pictura::LevelsDialog>(
        view_, levelsBlock(), canvas_->visibleDocumentRect().toAlignedRect(), window_.get());
    dialog->setCanvasSampling(tools_);
    dialog->setClippingCanvas(canvas_);
    dialog->show();
    return dialog;
}

void LevelsCanvasControlsTest::pick(const QPoint& pixel)
{
    // Keep the pixel centre at fractional zoom, where integer rounding can
    // move the click into a different source pixel.
    const QPointF local = canvas_->imageToWidget(QPointF(pixel) + QPointF(0.5, 0.5));
    const QPointF global = canvas_->mapToGlobal(local);
    QMouseEvent press(QEvent::MouseButtonPress, local, global, Qt::LeftButton,
                      Qt::LeftButton, Qt::NoModifier);
    QApplication::sendEvent(canvas_, &press);
    QMouseEvent release(QEvent::MouseButtonRelease, local, global, Qt::LeftButton,
                        Qt::NoButton, Qt::NoModifier);
    QApplication::sendEvent(canvas_, &release);
}

void LevelsCanvasControlsTest::levelsPickEndpoints()
{
    QImage image(20, 20, QImage::Format_RGB32);
    const QRgb original = qRgb(30, 100, 210);
    image.fill(original);
    image.setPixel(6, 6, qRgb(253, 2, 120));
    QVERIFY(open(image));
    const int history = view_->history_count();
    auto dialog = makeDialog();
    auto* black = dropper(*dialog, "Black");
    auto* white = dropper(*dialog, "White");
    auto* inBlack = dialog->findChild<QSpinBox*>(QStringLiteral("inputBlack"));
    QVERIFY(black && white && inBlack);
    inBlack->setValue(20);
    QTRY_VERIFY(view_->composite_argb(5, 5) != original);
    QVERIFY(setTarget(*dialog, black, 40));
    black->click();
    QVERIFY(tools_->canvasSamplerActive());
    pick({5, 5});
    QCOMPARE(inBlack->value(), 0);
    QTRY_COMPARE(view_->composite_argb(5, 5), qRgb(40, 40, 40));
    QVERIFY(setTarget(*dialog, white, 225));
    if (!white->isChecked()) {
        white->click();
    }
    pick({6, 6});
    QTRY_COMPARE(view_->composite_argb(6, 6), qRgb(225, 225, 225));
    dialog->reject();
    QVERIFY(!tools_->canvasSamplerActive());
    QCOMPARE(view_->composite_argb(5, 5), original);
    QCOMPARE(view_->composite_argb(6, 6), qRgb(253, 2, 120));
    QCOMPARE(view_->history_count(), history);
}

void LevelsCanvasControlsTest::levelsPickGray()
{
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(qRgb(60, 120, 190));
    QVERIFY(open(image));
    const int history = view_->history_count();
    auto dialog = makeDialog();
    auto* gray = dropper(*dialog, "Gray");
    QVERIFY(gray);
    gray->click();
    pick({5, 5});
    QTRY_VERIFY(std::abs(qRed(view_->composite_argb(5, 5)) - 128) <= 1);
    QVERIFY(std::abs(qGreen(view_->composite_argb(5, 5)) - 128) <= 1);
    QVERIFY(std::abs(qBlue(view_->composite_argb(5, 5)) - 128) <= 1);
    QVERIFY(setTarget(*dialog, gray, 1));
    if (!gray->isChecked()) {
        gray->click();
    }
    pick({5, 5});
    QTRY_VERIFY(qRed(view_->composite_argb(5, 5)) < 8);
    QVERIFY(qBlue(view_->composite_argb(5, 5)) >= 12);
    dialog->accept();
    QCOMPARE(dialog->result(), int(QDialog::Accepted));
    QCOMPARE(view_->history_count(), history + 1);
    QVERIFY(!tools_->canvasSamplerActive());
}

void LevelsCanvasControlsTest::levelsPickLimits()
{
    QImage image(20, 20, QImage::Format_ARGB32);
    image.fill(qRgb(90, 90, 90));
    image.setPixel(5, 5, qRgb(180, 180, 180));
    image.setPixel(11, 11, 0);
    image.setPixel(13, 13, qRgb(255, 60, 80));
    QVERIFY(open(image));
    auto dialog = makeDialog();
    auto* black = dropper(*dialog, "Black");
    auto* gray = dropper(*dialog, "Gray");
    QVERIFY(black && gray);
    const QPoint at = black->rect().center();
    QContextMenuEvent context(QContextMenuEvent::Mouse, at, black->mapToGlobal(at));
    QApplication::sendEvent(black, &context);
    auto* menu = black->findChild<QMenu*>(QStringLiteral("levelsSampleSize"));
    QVERIFY(menu);
    QList<int> sizes;
    QAction* point = nullptr;
    QAction* average = nullptr;
    QAction* largest = nullptr;
    for (auto* action : menu->actions()) {
        if (!action->data().isValid()) {
            continue;
        }
        const int size = action->data().toInt();
        sizes.append(size);
        if (size == 1) {
            point = action;
            QVERIFY(action->isChecked());
        } else if (size == 3) {
            average = action;
        } else if (size == 101) {
            largest = action;
        }
        action->trigger();
        QVERIFY(action->isChecked());
    }
    QCOMPARE(sizes, (QList<int>{1, 3, 5, 11, 31, 51, 101}));
    QVERIFY(average && point && largest);
    largest->trigger();
    menu->hide();
    black->click();
    pick({0, 0});
    QTRY_COMPARE(view_->composite_argb(5, 5), qRgb(139, 139, 139));
    average->trigger();
    pick({5, 5});
    QTRY_COMPARE(view_->composite_argb(5, 5), qRgb(132, 132, 132));
    point->trigger();
    const QByteArray before = dialog->block();
    auto* preview = dialog->findChild<QCheckBox*>(QStringLiteral("adjustmentPreview"));
    QVERIFY(preview);
    preview->setChecked(false);
    QCOMPARE(view_->composite_argb(5, 5), qRgb(180, 180, 180));
    preview->setChecked(true);
    pick({11, 11});
    QCOMPARE(dialog->block(), before);
    QTRY_COMPARE(view_->composite_argb(5, 5), qRgb(132, 132, 132));
    pick({13, 13});
    QCOMPARE(dialog->block(), before);
    gray->click();
    pick({13, 13});
    QCOMPARE(dialog->block(), before);
    dialog->reject();
    dialog.reset();

    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("grayscale")));
    dialog = makeDialog();
    QVERIFY(dropper(*dialog, "Black")->isEnabled());
    QVERIFY(dropper(*dialog, "White")->isEnabled());
    QVERIFY(!dropper(*dialog, "Gray")->isEnabled());
    dialog->reject();
}

void LevelsCanvasControlsTest::levelsAltDragClips()
{
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(qRgb(20, 100, 200));
    QVERIFY(open(image));
    const int history = view_->history_count();
    auto dialog = makeDialog();
    auto* slider = dialog->findChild<QWidget*>(QStringLiteral("levelsInputSlider"));
    auto* black = dialog->findChild<QSpinBox*>(QStringLiteral("inputBlack"));
    auto* white = dialog->findChild<QSpinBox*>(QStringLiteral("inputWhite"));
    auto* preview = dialog->findChild<QCheckBox*>(QStringLiteral("adjustmentPreview"));
    QVERIFY(slider && black && white && preview);
    sendDrag(slider, QEvent::MouseButtonPress, 0, Qt::AltModifier);
    sendDrag(slider, QEvent::MouseMove, 60, Qt::AltModifier);
    QTRY_VERIFY(canvas_->filterPreviewShownForTest());
    QCOMPARE(canvas_->filterPreviewImageForTest().pixel(5, 5), qRgb(0, 255, 255));
    QTest::keyRelease(slider, Qt::Key_Alt);
    QVERIFY(!canvas_->filterPreviewShownForTest());
    QCOMPARE(black->value(), 60);
    sendDrag(slider, QEvent::MouseButtonRelease, 60, Qt::NoModifier);

    preview->setChecked(false);
    sendDrag(slider, QEvent::MouseButtonPress, 255, Qt::AltModifier);
    sendDrag(slider, QEvent::MouseMove, 150, Qt::AltModifier);
    QTRY_VERIFY(canvas_->filterPreviewShownForTest());
    QCOMPARE(canvas_->filterPreviewImageForTest().pixel(5, 5), qRgb(0, 0, 255));
    sendDrag(slider, QEvent::MouseButtonRelease, 150, Qt::AltModifier);
    QVERIFY(!canvas_->filterPreviewShownForTest());
    QCOMPARE(white->value(), 150);
    QCOMPARE(view_->composite_argb(5, 5), qRgb(20, 100, 200));
    QCOMPARE(view_->history_count(), history);
    dialog->reject();
    dialog.reset();
    window_->closeDocument(window_->activeDocumentIndex(), false);

    pictura::set_cache_levels(2);
    QImage wide(4000, 64, QImage::Format_RGB32);
    wide.fill(qRgb(20, 100, 200));
    QVERIFY(open(wide));
    canvas_->fitOnScreen();
    QVERIFY(canvas_->visibleDocumentRect().width() > 3070);
    dialog = makeDialog();
    slider = dialog->findChild<QWidget*>(QStringLiteral("levelsInputSlider"));
    QVERIFY(slider);
    sendDrag(slider, QEvent::MouseButtonPress, 0, Qt::AltModifier);
    sendDrag(slider, QEvent::MouseMove, 60, Qt::AltModifier);
    QTRY_VERIFY(canvas_->filterPreviewShownForTest());
    QCOMPARE(canvas_->filterPreviewImageForTest().pixel(5, 5), qRgb(0, 255, 255));
    sendDrag(slider, QEvent::MouseMove, 120, Qt::AltModifier);
    QCOMPARE(canvas_->filterPreviewImageForTest().pixel(5, 5), qRgb(0, 0, 255));
    sendDrag(slider, QEvent::MouseButtonRelease, 120, Qt::AltModifier);
    QVERIFY(!canvas_->filterPreviewShownForTest());
    dialog->reject();
}

void LevelsCanvasControlsTest::levelsOtherDragsStayNormal()
{
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(qRgb(60, 120, 190));
    QVERIFY(open(image));
    auto dialog = makeDialog();
    auto* input = dialog->findChild<QWidget*>(QStringLiteral("levelsInputSlider"));
    auto* output = dialog->findChild<QWidget*>(QStringLiteral("levelsOutputSlider"));
    QVERIFY(input && output);
    const auto drag = [this](QWidget* widget, int from, int to, Qt::KeyboardModifiers mods) {
        sendDrag(widget, QEvent::MouseButtonPress, from, mods);
        sendDrag(widget, QEvent::MouseMove, to, mods);
        QVERIFY(!canvas_->filterPreviewShownForTest());
        sendDrag(widget, QEvent::MouseButtonRelease, to, mods);
    };
    drag(input, 128, 110, Qt::AltModifier);
    drag(output, 0, 20, Qt::AltModifier);
    drag(input, 0, 30, Qt::NoModifier);
    dialog->reject();
}

PICTURA_QT_TEST_MAIN(LevelsCanvasControlsTest)
#include "tst_levels_canvas_controls.moc"
