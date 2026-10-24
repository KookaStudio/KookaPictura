#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include <functional>
#include <memory>
#include <vector>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipboard.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point_retouch.cxxqt.h"
#include "vp_canvas.h"
#include "vp_dialog.h"
#include "vp_options.h"

#include "qt_test_support.h"

using pictura::VanishingPointDialog;
using pictura::VpCanvas;

namespace {

const QStringList kVpPath = {QStringLiteral("Filter"), QStringLiteral("Vanishing Point…")};
// A square-on plane inside the 400×300 test documents: 200×180 px.
const QList<QPointF> kSquare = {QPointF(100, 60), QPointF(300, 60), QPointF(300, 240),
                                QPointF(100, 240)};

VanishingPointDialog* openVpDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<VanishingPointDialog*>(widget);
            dialog && dialog->isVisible()) {
            return dialog;
        }
    }
    return nullptr;
}

void sendMouse(QWidget* widget, QEvent::Type type, const QPointF& at, Qt::MouseButton button,
               Qt::MouseButtons buttons, Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QMouseEvent event(type, at, widget->mapToGlobal(at), button, buttons, modifiers);
    QApplication::sendEvent(widget, &event);
}

void drag(VpCanvas* canvas, const QPointF& from, const QPointF& to,
          Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    const QPointF a = canvas->toWidget(from);
    const QPointF b = canvas->toWidget(to);
    sendMouse(canvas, QEvent::MouseButtonPress, a, Qt::LeftButton, Qt::LeftButton, modifiers);
    for (int i = 1; i <= 5; ++i) {
        sendMouse(canvas, QEvent::MouseMove, a + (b - a) * (i / 5.0), Qt::NoButton, Qt::LeftButton,
                  modifiers);
    }
    sendMouse(canvas, QEvent::MouseButtonRelease, b, Qt::LeftButton, Qt::NoButton, modifiers);
}

void click(VpCanvas* canvas, const QPointF& document,
           Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    const QPointF at = canvas->toWidget(document);
    sendMouse(canvas, QEvent::MouseButtonPress, at, Qt::LeftButton, Qt::LeftButton, modifiers);
    sendMouse(canvas, QEvent::MouseButtonRelease, at, Qt::LeftButton, Qt::NoButton, modifiers);
}

void makePlane(VanishingPointDialog* dialog, const QList<QPointF>& corners)
{
    dialog->selectTool(VanishingPointDialog::CreatePlane);
    for (const QPointF& p : corners) {
        click(dialog->canvas(), p);
    }
}

// Put a `w`×`h` block of `colour` on the app clipboard.
void clipColour(int w, int h, const QColor& colour)
{
    std::vector<std::uint8_t> rgba;
    for (int i = 0; i < w * h; ++i) {
        rgba.insert(rgba.end(), {std::uint8_t(colour.red()), std::uint8_t(colour.green()),
                                 std::uint8_t(colour.blue()), 255});
    }
    QVERIFY(pictura::clipboard_import(w, h, ::rust::Slice<const std::uint8_t>(rgba.data(), rgba.size())));
}

QColor at(const QImage& image, int x, int y)
{
    return image.pixelColor(x, y);
}

} // namespace

class VanishingPointRetouchTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void optionsFollowTheTool();
    void ctrlVPastesIntoThePlane();
    void pastedFloatCommitsOnOk();
    void okAppliesAStrokeAsOneState();
    void stampAndEyedropperWork();
    void bracketsChangeDiameter();

private:
    pictura::PictureView* fresh(const QString& name);
    std::unique_ptr<VanishingPointDialog> showDialog();
    bool runCommand(const std::function<void(VanishingPointDialog*)>& body);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void VanishingPointRetouchTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

pictura::PictureView* VanishingPointRetouchTest::fresh(const QString& name)
{
    if (!window_->newDocument(name, 400, 300, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

std::unique_ptr<VanishingPointDialog> VanishingPointRetouchTest::showDialog()
{
    if (!fresh(QStringLiteral("vp"))) {
        return nullptr;
    }
    auto dialog = std::make_unique<VanishingPointDialog>(window_->activeView(),
                                                         QStringLiteral("vp.png"));
    dialog->show();
    if (!QTest::qWaitForWindowExposed(dialog.get())) {
        return nullptr;
    }
    return dialog;
}

bool VanishingPointRetouchTest::runCommand(const std::function<void(VanishingPointDialog*)>& body)
{
    // A refused command never spins a modal loop, so the timer owns copies.
    auto opened = std::make_shared<bool>(false);
    auto* timer = new QTimer(window_.get());
    timer->setSingleShot(true);
    connect(timer, &QTimer::timeout, timer, [opened, body, timer] {
        if (VanishingPointDialog* dialog = openVpDialog()) {
            *opened = true;
            body(dialog);
        }
        timer->deleteLater();
    });
    timer->start(0);
    window_->registry()->dispatch(pictura::commandIdForPath(kVpPath));
    if (!*opened) {
        delete timer;
    }
    return *opened;
}

void VanishingPointRetouchTest::optionsFollowTheTool()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    auto shown = [&](const char* name) {
        QWidget* w = dialog->findChild<QWidget*>(QString::fromLatin1(name));
        return w && w->isVisibleTo(dialog.get());
    };
    auto spin = [&](const char* name) { return dialog->findChild<QSpinBox*>(QString::fromLatin1(name)); };
    auto items = [&](const char* name) {
        QStringList out;
        auto* box = dialog->findChild<QComboBox*>(QString::fromLatin1(name));
        for (int i = 0; box && i < box->count(); ++i) {
            out << box->itemText(i);
        }
        return out;
    };
    const QStringList heal = {QStringLiteral("Off"), QStringLiteral("Luminance"), QStringLiteral("On")};

    dialog->selectTool(VanishingPointDialog::EditPlane);
    QVERIFY(shown("vpGridSize") && shown("vpAngle") && !shown("vpFeather"));

    dialog->selectTool(VanishingPointDialog::Marquee);
    QVERIFY(shown("vpFeather") && shown("vpMoveMode") && !shown("vpGridSize"));
    QCOMPARE(spin("vpFeather")->value(), 0);
    QCOMPARE(spin("vpFeather")->maximum(), 250);
    QCOMPARE(spin("vpMarqueeOpacity")->value(), 100);
    QCOMPARE(items("vpMarqueeHeal"), heal);
    QCOMPARE(items("vpMoveMode"), QStringList({QStringLiteral("Destination"), QStringLiteral("Source")}));

    dialog->selectTool(VanishingPointDialog::Stamp);
    QVERIFY(shown("vpStampDiameter") && shown("vpAligned") && !shown("vpBrushColour"));
    QCOMPARE(spin("vpStampDiameter")->value(), 100);
    QCOMPARE(spin("vpStampDiameter")->minimum(), 1);
    QCOMPARE(spin("vpStampDiameter")->maximum(), 5000);
    QCOMPARE(spin("vpStampHardness")->maximum(), 100);
    QCOMPARE(items("vpStampHeal"), heal);
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("vpAligned"))->isChecked());

    dialog->selectTool(VanishingPointDialog::Brush);
    QVERIFY(shown("vpBrushDiameter") && shown("vpBrushColour") && !shown("vpAligned"));
    QCOMPARE(spin("vpBrushOpacity")->value(), 100);

    dialog->selectTool(VanishingPointDialog::Transform);
    QVERIFY(shown("vpFlip") && shown("vpFlop"));

    dialog->selectTool(VanishingPointDialog::Eyedropper);
    QVERIFY(shown("vpEyedropperColour") && !shown("vpBrushDiameter"));

    dialog->selectTool(VanishingPointDialog::Hand);
    QVERIFY(!shown("vpGridSize") && !shown("vpFeather"));
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointRetouchTest::ctrlVPastesIntoThePlane()
{
    clipColour(40, 30, Qt::red);
    {
        auto dialog = showDialog();
        QVERIFY(dialog);
        QTest::keyClick(dialog.get(), Qt::Key_V, Qt::ControlModifier);
        QVERIFY(!pictura::vp_has_float());
        QVERIFY(dialog->helpLine()->text().contains(QStringLiteral("plane")));
        dialog->reject();
        QVERIFY(window_->closeActiveDocument(false));
    }
    auto dialog = showDialog();
    QVERIFY(dialog);
    makePlane(dialog.get(), kSquare);
    QTest::keyClick(dialog.get(), Qt::Key_V, Qt::ControlModifier);
    QVERIFY(pictura::vp_has_float());
    QCOMPARE(dialog->tool(), int(VanishingPointDialog::Marquee));
    QCOMPARE(pictura::vp_float_corners().size(), size_t(8));
    QTest::keyClick(dialog.get(), Qt::Key_T, Qt::ControlModifier);
    QCOMPARE(dialog->tool(), int(VanishingPointDialog::Transform));
    // The preview shows the floating red block at the plane's centre.
    QCOMPARE(at(dialog->canvas()->previewImage(), 200, 150), QColor(Qt::red));
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointRetouchTest::pastedFloatCommitsOnOk()
{
    clipColour(40, 40, Qt::red);
    pictura::PictureView* view = fresh(QStringLiteral("paste"));
    QVERIFY(view);
    const int history = view->history_count();
    QVERIFY(runCommand([](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        QVERIFY(dialog->paste());
        drag(dialog->canvas(), QPointF(200, 150), QPointF(230, 170));
        dialog->accept();
    }));
    QCOMPARE(view->history_count(), history + 1);
    const QImage image = view->image();
    QCOMPARE(at(image, 230, 170), QColor(Qt::red));
    QCOMPARE(at(image, 200, 150), QColor(Qt::white));
    QCOMPARE(at(image, 50, 20), QColor(Qt::white));
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointRetouchTest::okAppliesAStrokeAsOneState()
{
    pictura::PictureView* view = fresh(QStringLiteral("stroke"));
    QVERIFY(view);
    const QImage document = view->image();
    const int history = view->history_count();
    auto stroke = [](VanishingPointDialog* dialog) {
        makePlane(dialog, kSquare);
        dialog->selectTool(VanishingPointDialog::Brush);
        dialog->findChild<QSpinBox*>(QStringLiteral("vpBrushDiameter"))->setValue(20);
        drag(dialog->canvas(), QPointF(150, 150), QPointF(250, 150));
    };
    QVERIFY(runCommand([&](VanishingPointDialog* dialog) {
        stroke(dialog);
        dialog->reject();
    }));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), document);

    QVERIFY(runCommand([&](VanishingPointDialog* dialog) {
        stroke(dialog);
        dialog->accept();
    }));
    QCOMPARE(view->history_count(), history + 1);
    QVERIFY(view->history_label(view->history_count() - 1).contains(QStringLiteral("Vanishing Point")));
    const QImage image = view->image();
    QCOMPARE(at(image, 200, 150), QColor(Qt::black));
    QCOMPARE(at(image, 200, 200), QColor(Qt::white));
    QCOMPARE(at(image, 50, 150), QColor(Qt::white));
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointRetouchTest::stampAndEyedropperWork()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    makePlane(dialog.get(), kSquare);
    VpCanvas* canvas = dialog->canvas();
    // Paint a red patch, then clone it beside itself.
    dialog->options()->setColour(Qt::red);
    dialog->selectTool(VanishingPointDialog::Brush);
    dialog->findChild<QSpinBox*>(QStringLiteral("vpBrushDiameter"))->setValue(30);
    click(canvas, QPointF(150, 150));
    dialog->selectTool(VanishingPointDialog::Stamp);
    dialog->findChild<QSpinBox*>(QStringLiteral("vpStampDiameter"))->setValue(10);
    click(canvas, QPointF(250, 150));
    QVERIFY(dialog->helpLine()->text().contains(QStringLiteral("source")));
    click(canvas, QPointF(150, 150), Qt::AltModifier);
    click(canvas, QPointF(250, 200));
    QCOMPARE(at(canvas->previewImage(), 250, 200), QColor(Qt::red));
    // The Eyedropper picks it back as Brush Color.
    dialog->options()->setColour(Qt::blue);
    dialog->selectTool(VanishingPointDialog::Eyedropper);
    click(canvas, QPointF(250, 200));
    QCOMPARE(dialog->options()->colour(), QColor(Qt::red));
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

void VanishingPointRetouchTest::bracketsChangeDiameter()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    dialog->selectTool(VanishingPointDialog::Brush);
    auto* diameter = dialog->findChild<QSpinBox*>(QStringLiteral("vpBrushDiameter"));
    QCOMPARE(diameter->value(), 100);
    QTest::keyClick(dialog.get(), Qt::Key_BracketRight);
    QCOMPARE(diameter->value(), 150);
    QTest::keyClick(dialog.get(), Qt::Key_BracketLeft);
    QCOMPARE(diameter->value(), 100);
    dialog->reject();
    QVERIFY(window_->closeActiveDocument(false));
}

QTEST_MAIN(VanishingPointRetouchTest)
#include "tst_vanishing_point_retouch.moc"
