#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>

#include "awa_canvas.h"
#include "awa_dialog.h"
#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/adaptive_wide_angle.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include "qt_test_support.h"

using pictura::AdaptiveWideAngleDialog;
using pictura::AwaCanvas;

namespace {

const QStringList kAwaPath = {QStringLiteral("Filter"), QStringLiteral("Adaptive Wide Angle…")};

AdaptiveWideAngleDialog* openAwaDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<AdaptiveWideAngleDialog*>(widget);
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

// A left drag across the canvas between display points.
void drag(AwaCanvas* canvas, const QPointF& from, const QPointF& to,
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

void click(AwaCanvas* canvas, const QPointF& display)
{
    const QPointF at = canvas->toWidget(display);
    sendMouse(canvas, QEvent::MouseButtonPress, at, Qt::LeftButton, Qt::LeftButton);
    sendMouse(canvas, QEvent::MouseButtonRelease, at, Qt::LeftButton, Qt::NoButton);
}

// The view's pixel under display point `p`.
QColor viewPixel(AwaCanvas* canvas, const QPointF& p)
{
    const QImage& view = canvas->previewImage();
    const QPointF at = canvas->toWidget(p) * view.devicePixelRatio();
    return view.pixelColor(static_cast<int>(at.x()), static_cast<int>(at.y()));
}

// The codec's `metadata.psd` fixture: a pixel layer and a real EXIF IFD
// (Make "ACME Cameras", Model "ACME One", FocalLength 50 mm).
QString metadataFixture()
{
    QString fixture =
        QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/metadata.psd");
    QDir dir(QCoreApplication::applicationDirPath());
    for (int i = 0; i < 6 && fixture.isEmpty(); ++i) {
        const QString candidate =
            dir.filePath(QStringLiteral("crates/pictura-codec/tests/fixtures/metadata.psd"));
        if (QFileInfo::exists(candidate)) {
            fixture = candidate;
        } else if (!dir.cdUp()) {
            break;
        }
    }
    return fixture;
}

} // namespace

class AdaptiveWideAngleTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void menuFollowsDepth();
    void opensWithControls();
    void exifPrefillsLens();
    void fullSphericalNeedsTwoToOne();
    void dragDrawsConstraints();
    void contextMenuOrientsAndDeletes();
    void polygonClicksClose();
    void previewToggleShowsSource();
    void showMeshDrawsGrid();
    void moveToolOffsets();
    void toolShortcuts();
    void detailFollowsPointer();
    void cancelAndOk();
    void lastSettingsRestoresConstraints();
    void lastSettingsRefusesAnUnkeptSet();
    void lastFilterRefusesFullSphericalOffTwoToOne();
    void lastFilterRefusesOffEightBit();

private:
    // Show a dialog over the active document, exposed.
    std::unique_ptr<AdaptiveWideAngleDialog> showDialog(const QString& name = QStringLiteral("lens.png"));

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void AdaptiveWideAngleTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // Diagonal bands over a gradient, so every warp shows.
    QImage picture(300, 200, QImage::Format_RGB32);
    for (int y = 0; y < picture.height(); ++y) {
        for (int x = 0; x < picture.width(); ++x) {
            const bool band = ((x + y) / 10) % 2;
            picture.setPixelColor(x, y, QColor(x * 255 / 300, y * 255 / 200, band ? 230 : 30));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("lens.png"));
    QVERIFY(picture.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
}

std::unique_ptr<AdaptiveWideAngleDialog> AdaptiveWideAngleTest::showDialog(const QString& name)
{
    auto dialog = std::make_unique<AdaptiveWideAngleDialog>(window_->activeView(), name);
    dialog->show();
    if (!QTest::qWaitForWindowExposed(dialog.get())) {
        return nullptr;
    }
    return dialog;
}

void AdaptiveWideAngleTest::menuFollowsDepth()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kAwaPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("adaptive-wide-angle"));
    QCOMPARE(pictura::filter_param_arity(spec->kind), 7);
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kAwaPath));
    QVERIFY(action != nullptr);
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
    QVERIFY(window_->newDocument(QStringLiteral("Deep"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::ImageMode16Bits)));
    QCOMPARE(pictura::image_depth_bits(*window_->activeView()), 16);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(window_->closeActiveDocument(false));
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
}

void AdaptiveWideAngleTest::opensWithControls()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    QCOMPARE(dialog->windowTitle(), QStringLiteral("Adaptive Wide Angle (lens.png @ 100%)"));
    auto* correction = dialog->findChild<QComboBox*>(QStringLiteral("awaCorrection"));
    QStringList items;
    for (int i = 0; i < correction->count(); ++i) {
        items << correction->itemText(i);
    }
    QCOMPARE(items, QStringList({QStringLiteral("Fisheye"), QStringLiteral("Perspective"),
                                 QStringLiteral("Full Spherical")}));
    auto* scale = dialog->findChild<QSpinBox*>(QStringLiteral("awaScale"));
    QCOMPARE(scale->value(), 100);
    QCOMPARE(scale->minimum(), 50);
    QCOMPARE(scale->maximum(), 150);
    // No EXIF: a normal lens at CS6's opening crop, 4.05–121.41 mm.
    auto* focal = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("awaFocal"));
    auto* crop = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("awaCrop"));
    QCOMPARE(crop->value(), 0.57);
    QCOMPARE(crop->minimum(), 0.1);
    QCOMPARE(crop->maximum(), 10.0);
    QCOMPARE(focal->value(), 75.91);
    QCOMPARE(focal->minimum(), 4.05);
    QCOMPARE(focal->maximum(), 121.41);
    // Focal Length's range follows Crop Factor, slider included.
    crop->setValue(0.1);
    QVERIFY(std::abs(focal->minimum() - 23.085) <= 0.006);
    QCOMPARE(focal->maximum(), 692.04);
    QCOMPARE(dialog->findChild<QSlider*>(QStringLiteral("awaFocalSlider"))->maximum(), 69204);
    focal->setValue(694.29);
    QCOMPARE(focal->value(), 692.04);
    QVERIFY(!dialog->findChild<QCheckBox*>(QStringLiteral("awaAsShot"))->isEnabled());
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("awaPreview"))->isChecked());
    QVERIFY(dialog->findChild<QCheckBox*>(QStringLiteral("awaShowConstraints"))->isChecked());
    QVERIFY(!dialog->findChild<QCheckBox*>(QStringLiteral("awaShowMesh"))->isChecked());
    QCOMPARE(dialog->findChild<QLabel*>(QStringLiteral("awaCamera"))->text(),
             QStringLiteral("Camera Model: --"));
    QCOMPARE(dialog->tool(), int(AdaptiveWideAngleDialog::Constraint));
    dialog->reject();
}

void AdaptiveWideAngleTest::exifPrefillsLens()
{
    const QString fixture = metadataFixture();
    if (fixture.isEmpty()) {
        QSKIP("metadata.psd fixture not found");
    }
    QVERIFY(window_->openDocumentAtPath(fixture));
    {
        auto dialog = showDialog(QStringLiteral("metadata.psd"));
        QVERIFY(dialog && dialog->ready());
        // FocalLength 50 mm with no 35 mm value: crop 1.
        QCOMPARE(dialog->findChild<QDoubleSpinBox*>(QStringLiteral("awaFocal"))->value(), 50.0);
        QCOMPARE(dialog->findChild<QDoubleSpinBox*>(QStringLiteral("awaCrop"))->value(), 1.0);
        auto* asShot = dialog->findChild<QCheckBox*>(QStringLiteral("awaAsShot"));
        QVERIFY(asShot->isEnabled() && asShot->isChecked());
        const QString camera = dialog->findChild<QLabel*>(QStringLiteral("awaCamera"))->text();
        QVERIFY2(camera.contains(QStringLiteral("ACME Cameras ACME One")), qPrintable(camera));
        dialog->reject();
    }
    QVERIFY(window_->closeActiveDocument(false));
}

void AdaptiveWideAngleTest::fullSphericalNeedsTwoToOne()
{
    auto sphericalEnabled = [](const AdaptiveWideAngleDialog& dialog) {
        auto* combo = dialog.findChild<QComboBox*>(QStringLiteral("awaCorrection"));
        return qobject_cast<QStandardItemModel*>(combo->model())->item(2)->isEnabled();
    };
    {
        auto dialog = showDialog();
        QVERIFY(dialog);
        QVERIFY(!sphericalEnabled(*dialog));
        dialog->reject();
    }
    QImage pano(200, 100, QImage::Format_RGB32);
    pano.fill(QColor(90, 140, 200));
    const QString path = dir_.filePath(QStringLiteral("pano.png"));
    QVERIFY(pano.save(path) && window_->openDocumentAtPath(path));
    {
        auto dialog = showDialog(QStringLiteral("pano.png"));
        QVERIFY(dialog);
        QVERIFY(sphericalEnabled(*dialog));
        dialog->reject();
    }
    QVERIFY(window_->closeActiveDocument(false));
}

void AdaptiveWideAngleTest::dragDrawsConstraints()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    AwaCanvas* canvas = dialog->canvas();
    const QImage before = canvas->previewImage();
    drag(canvas, QPointF(60, 60), QPointF(240, 70));
    QCOMPARE(pictura::awa_constraint_count(), 1);
    QCOMPARE(pictura::awa_constraint_state(0), 0);
    QCOMPARE(canvas->constraintColor(0), QColor(0, 255, 255));
    drag(canvas, QPointF(60, 150), QPointF(240, 165), Qt::ShiftModifier);
    QCOMPARE(pictura::awa_constraint_count(), 2);
    QCOMPARE(pictura::awa_constraint_state(1), 1);
    QCOMPARE(canvas->constraintColor(1), QColor(255, 255, 0));
    QVERIFY(canvas->previewImage() != before);
    dialog->reject();
}

void AdaptiveWideAngleTest::contextMenuOrientsAndDeletes()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    AwaCanvas* canvas = dialog->canvas();
    drag(canvas, QPointF(60, 60), QPointF(240, 80));
    QCOMPARE(pictura::awa_constraint_count(), 1);
    std::unique_ptr<QMenu> menu(canvas->constraintMenu(0, dialog.get()));
    auto trigger = [&menu](const char* name) {
        QAction* action = menu->findChild<QAction*>(QString::fromLatin1(name));
        QVERIFY2(action, name);
        action->trigger();
    };
    trigger("awaOrientVertical");
    QCOMPARE(canvas->constraintColor(0), QColor(255, 0, 255));
    trigger("awaOrientArbitrary");
    QCOMPARE(pictura::awa_constraint_state(0), 3);
    QCOMPARE(canvas->constraintColor(0), QColor(0, 255, 0));
    trigger("awaDelete");
    QCOMPARE(pictura::awa_constraint_count(), 0);
    dialog->reject();
}

void AdaptiveWideAngleTest::polygonClicksClose()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    AwaCanvas* canvas = dialog->canvas();
    dialog->selectTool(AdaptiveWideAngleDialog::Polygon);
    for (const QPointF& p : {QPointF(60, 40), QPointF(240, 40), QPointF(240, 160), QPointF(70, 150)}) {
        click(canvas, p);
    }
    QCOMPARE(canvas->pendingCorners(), 4);
    canvas->setFocus();
    QTest::keyClick(canvas, Qt::Key_E);
    QCOMPARE(canvas->pendingCorners(), 3);
    click(canvas, QPointF(60, 160));
    click(canvas, QPointF(60, 40));
    QCOMPARE(canvas->pendingCorners(), 0);
    QCOMPARE(pictura::awa_constraint_count(), 1);
    QVERIFY(pictura::awa_constraint_polygon(0));
    QCOMPARE(pictura::awa_constraint_points(0, true).size(), size_t(8));
    dialog->reject();
}

void AdaptiveWideAngleTest::previewToggleShowsSource()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    AwaCanvas* canvas = dialog->canvas();
    dialog->findChild<QSpinBox*>(QStringLiteral("awaScale"))->setValue(50);
    const QPointF corner(2, 2);
    QCOMPARE(viewPixel(canvas, corner).alpha(), 0);
    auto* preview = dialog->findChild<QCheckBox*>(QStringLiteral("awaPreview"));
    preview->setChecked(false);
    const QColor source = viewPixel(canvas, corner);
    QCOMPARE(source.alpha(), 255);
    QCOMPARE(source.rgb(), window_->activeView()->image().pixelColor(2, 2).rgb());
    preview->setChecked(true);
    QCOMPARE(viewPixel(canvas, corner).alpha(), 0);
    dialog->reject();
}

void AdaptiveWideAngleTest::showMeshDrawsGrid()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    AwaCanvas* canvas = dialog->canvas();
    QVERIFY(canvas->meshPathForTest().isEmpty());
    dialog->findChild<QCheckBox*>(QStringLiteral("awaShowMesh"))->setChecked(true);
    QVERIFY(canvas->meshShown());
    const QRectF full = canvas->meshPathForTest().boundingRect();
    QVERIFY(!full.isEmpty());
    dialog->findChild<QSpinBox*>(QStringLiteral("awaScale"))->setValue(60);
    const QRectF shrunk = canvas->meshPathForTest().boundingRect();
    QVERIFY2(shrunk.width() < full.width() * 0.7, "the mesh did not follow Scale");
    dialog->reject();
}

void AdaptiveWideAngleTest::moveToolOffsets()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    dialog->selectTool(AdaptiveWideAngleDialog::Move);
    QCOMPARE(dialog->canvas()->zoom(), 1.0);
    drag(dialog->canvas(), QPointF(150, 100), QPointF(170, 100));
    dialog->accept();
    const QList<double> values = dialog->committed();
    QCOMPARE(values.size(), 7);
    QVERIFY2(std::abs(values.at(4) - 20.0 / 300.0) < 1e-6, qPrintable(QString::number(values.at(4))));
    QVERIFY(std::abs(values.at(5)) < 1e-9);
}

void AdaptiveWideAngleTest::toolShortcuts()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    dialog->canvas()->setFocus();
    const struct {
        Qt::Key key;
        AdaptiveWideAngleDialog::Tool tool;
    } keys[] = {{Qt::Key_Y, AdaptiveWideAngleDialog::Polygon},
                {Qt::Key_M, AdaptiveWideAngleDialog::Move},
                {Qt::Key_H, AdaptiveWideAngleDialog::Hand},
                {Qt::Key_Z, AdaptiveWideAngleDialog::Zoom},
                {Qt::Key_C, AdaptiveWideAngleDialog::Constraint}};
    for (const auto& k : keys) {
        QTest::keyClick(dialog.get(), k.key);
        QCOMPARE(dialog->tool(), int(k.tool));
    }
    dialog->reject();
}

void AdaptiveWideAngleTest::detailFollowsPointer()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    AwaCanvas* canvas = dialog->canvas();
    sendMouse(canvas, QEvent::MouseMove, canvas->toWidget(QPointF(60, 50)), Qt::NoButton,
              Qt::NoButton);
    const QImage first = dialog->detail()->image();
    QVERIFY(!first.isNull());
    sendMouse(canvas, QEvent::MouseMove, canvas->toWidget(QPointF(240, 150)), Qt::NoButton,
              Qt::NoButton);
    QVERIFY(dialog->detail()->image() != first);
    dialog->reject();
}

void AdaptiveWideAngleTest::cancelAndOk()
{
    pictura::PictureView* view = window_->activeView();
    const QImage document = view->image();
    const int history = view->history_count();
    const QString id = pictura::commandIdForPath(kAwaPath);
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (AdaptiveWideAngleDialog* dialog = openAwaDialog()) {
            opened = true;
            drag(dialog->canvas(), QPointF(60, 60), QPointF(240, 70));
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(id));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), document);

    QTimer::singleShot(0, [] {
        if (AdaptiveWideAngleDialog* dialog = openAwaDialog()) {
            drag(dialog->canvas(), QPointF(60, 60), QPointF(240, 70));
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(id));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("adaptive-wide-angle"));
    QCOMPARE(filter_last_params(*view).size(), 7);
    QVERIFY(view->image() != document);
}

void AdaptiveWideAngleTest::lastSettingsRestoresConstraints()
{
    pictura::PictureView* view = window_->activeView();
    QTimer::singleShot(0, [] {
        if (AdaptiveWideAngleDialog* dialog = openAwaDialog()) {
            dialog->findChild<QComboBox*>(QStringLiteral("awaCorrection"))->setCurrentIndex(1);
            dialog->findChild<QSpinBox*>(QStringLiteral("awaScale"))->setValue(120);
            drag(dialog->canvas(), QPointF(60, 60), QPointF(240, 70));
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kAwaPath)));
    const int history = view->history_count();
    int correction = -1;
    int scale = -1;
    int constraints = -1;
    QTimer::singleShot(0, [&] {
        if (AdaptiveWideAngleDialog* dialog = openAwaDialog()) {
            correction =
                dialog->findChild<QComboBox*>(QStringLiteral("awaCorrection"))->currentIndex();
            scale = dialog->findChild<QSpinBox*>(QStringLiteral("awaScale"))->value();
            constraints = pictura::awa_constraint_count();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(correction, 1);
    QCOMPARE(scale, 120);
    QCOMPARE(constraints, 1);
    QCOMPARE(view->history_count(), history);
}

void AdaptiveWideAngleTest::lastSettingsRefusesAnUnkeptSet()
{
    // Id 4e9 names no kept constraint set: refuse rather than open on
    // defaults that OK would commit.
    QList<double> out;
    QVERIFY(!AdaptiveWideAngleDialog::get(window_->activeView(), QStringLiteral("lens.png"),
                                          {0, 100, 75.91, 0.57, 0, 0, 4e9}, &out, nullptr));
    QVERIFY(out.isEmpty());
    QVERIFY(openAwaDialog() == nullptr);
}

// Opens a picture, commits Adaptive Wide Angle on it through the menu with
// `correction`, and returns the view.
static pictura::PictureView* commitOn(pictura::PicturaMainWindow& window, const QString& path,
                                      int correction)
{
    if (!window.openDocumentAtPath(path)) {
        return nullptr;
    }
    QTimer::singleShot(0, [correction] {
        if (AdaptiveWideAngleDialog* dialog = openAwaDialog()) {
            dialog->findChild<QComboBox*>(QStringLiteral("awaCorrection"))->setCurrentIndex(correction);
            dialog->accept();
        }
    });
    window.registry()->dispatch(
        pictura::commandIdForPath({QStringLiteral("Filter"), QStringLiteral("Adaptive Wide Angle…")}));
    return window.activeView();
}

void AdaptiveWideAngleTest::lastFilterRefusesFullSphericalOffTwoToOne()
{
    QImage pano(200, 100, QImage::Format_RGB32);
    for (int y = 0; y < 100; ++y) {
        for (int x = 0; x < 200; ++x) {
            pano.setPixelColor(x, y, QColor(x, y * 2, ((x + y) / 8) % 2 ? 220 : 40));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("pano-crop.png"));
    QVERIFY(pano.save(path));
    pictura::PictureView* view = commitOn(*window_, path, 2);
    QVERIFY(view);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("adaptive-wide-angle"));
    QCOMPARE(filter_last_params(*view).value(0), 2.0);
    // Now 3:2: Full Spherical no longer applies on any path.
    QVERIFY(pictura::canvas_size_apply(*view, QStringLiteral("center"), 200, 133, 0xffffffu));
    const int history = view->history_count();
    const QImage document = view->image();
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (AdaptiveWideAngleDialog* dialog = openAwaDialog()) {
            opened = true;
            dialog->accept();
        }
    });
    window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings));
    QVERIFY(!opened);
    window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), document);
    QVERIFY(filter_last_error(*view).contains(QStringLiteral("2:1")));
    QVERIFY(window_->closeActiveDocument(false));
}

void AdaptiveWideAngleTest::lastFilterRefusesOffEightBit()
{
    QImage picture(60, 40, QImage::Format_RGB32);
    picture.fill(QColor(120, 160, 200));
    const QString path = dir_.filePath(QStringLiteral("deep.png"));
    QVERIFY(picture.save(path));
    pictura::PictureView* view = commitOn(*window_, path, 0);
    QVERIFY(view);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("adaptive-wide-angle"));
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::ImageMode16Bits)));
    QCOMPARE(pictura::image_depth_bits(*view), 16);
    const int history = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (AdaptiveWideAngleDialog* dialog = openAwaDialog()) {
            opened = true;
            dialog->accept();
        }
    });
    window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings));
    QVERIFY(!opened);
    window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QCOMPARE(view->history_count(), history);
    QVERIFY(filter_last_error(*view).contains(QStringLiteral("8-bit")));
    QVERIFY(window_->closeActiveDocument(false));
}

QTEST_MAIN(AdaptiveWideAngleTest)
#include "tst_adaptive_wide_angle.moc"
