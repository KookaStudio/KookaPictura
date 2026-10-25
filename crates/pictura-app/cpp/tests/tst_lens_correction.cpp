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
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QTabWidget>

#include <cmath>
#include <functional>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "lens_correction_canvas.h"
#include "lens_correction_dialog.h"
#include "lens_correction_panels.h"
#include "panels/angle_dial.h"
#include "panels/color_swatch_widget.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"

#include "qt_test_support.h"

using pictura::LensCorrectionCanvas;
using pictura::LensCorrectionDialog;
using pictura::LensCustomPanel;

namespace {

const QStringList kPath = {QStringLiteral("Filter"), QStringLiteral("Lens Correction…")};

LensCorrectionDialog* openDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<LensCorrectionDialog*>(widget);
            dialog && dialog->isVisible()) {
            return dialog;
        }
    }
    return nullptr;
}

void sendMouse(QWidget* widget, QEvent::Type type, const QPointF& at, Qt::MouseButton button,
               Qt::MouseButtons buttons)
{
    QMouseEvent event(type, at, widget->mapToGlobal(at), button, buttons, Qt::NoModifier);
    QApplication::sendEvent(widget, &event);
}

// A left drag across the canvas between display points.
void drag(LensCorrectionCanvas* canvas, const QPointF& from, const QPointF& to)
{
    const QPointF a = canvas->toWidget(from);
    const QPointF b = canvas->toWidget(to);
    sendMouse(canvas, QEvent::MouseButtonPress, a, Qt::LeftButton, Qt::LeftButton);
    for (int i = 1; i <= 5; ++i) {
        sendMouse(canvas, QEvent::MouseMove, a + (b - a) * (i / 5.0), Qt::NoButton, Qt::LeftButton);
    }
    sendMouse(canvas, QEvent::MouseButtonRelease, b, Qt::LeftButton, Qt::NoButton);
}

// The view's pixel under display point `p`.
QColor viewPixel(LensCorrectionCanvas* canvas, const QPointF& p)
{
    const QImage& view = canvas->previewImage();
    const QPointF at = canvas->toWidget(p) * view.devicePixelRatio();
    return view.pixelColor(static_cast<int>(at.x()), static_cast<int>(at.y()));
}

template <typename T> T* child(const QWidget* dialog, const char* name)
{
    return dialog->findChild<T*>(QString::fromLatin1(name));
}

QDoubleSpinBox* box(const QWidget* dialog, const char* name)
{
    return child<QDoubleSpinBox>(dialog, name);
}

// The codec's `metadata.psd` fixture: a pixel layer and a real EXIF IFD
// (Make "ACME Cameras", Model "ACME One", FocalLength 50 mm, FNumber 2.8).
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

class LensCorrectionTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void menuFollowsDepthAndMode();
    void menuRowHasItsShortcut();
    void opensWithControls();
    void exifPrefillsSearchAndStatus();
    void settingsComboSwitchesToCustom();
    void removeDistortionToolDragsSlider();
    void straightenToolSetsAngle();
    void angleDialAndFieldSync();
    void moveGridToolOffsetsGrid();
    void showGridSizeAndColor();
    void toolShortcuts();
    void hintFollowsTool();
    void previewToggleShowsSource();
    void grayscaleDisablesChromaticAberration();
    void cancelAndOk();
    void previousConversionRestoresLastCommit();
    void lastSettingsRestoresValues();
    void lastFilterRefusesOffSixteenBit();

private:
    std::unique_ptr<LensCorrectionDialog> showDialog();

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LensCorrectionTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // Diagonal bands over a gradient, so every correction shows.
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

std::unique_ptr<LensCorrectionDialog> LensCorrectionTest::showDialog()
{
    auto dialog = std::make_unique<LensCorrectionDialog>(window_->activeView());
    dialog->show();
    if (!QTest::qWaitForWindowExposed(dialog.get())) {
        return nullptr;
    }
    return dialog;
}

void LensCorrectionTest::menuFollowsDepthAndMode()
{
    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kPath);
    QVERIFY(spec != nullptr);
    QCOMPARE(spec->kind, QStringLiteral("lens-correction"));
    QCOMPARE(spec->dialog, pictura::FilterDialogKind::LensCorrection);
    QCOMPARE(pictura::filter_param_arity(spec->kind), 16);
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kPath));
    QVERIFY(action != nullptr);
    // Whether the menu row is enabled on a new 8-bit RGB document after
    // `change`; -1 when the document or the change fails.
    auto enabledAfter = [&](const std::function<bool(pictura::PictureView&)>& change) {
        if (!window_->newDocument(QStringLiteral("Gate"), 64, 64, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white"))) {
            return -1;
        }
        const bool changed = change(*window_->activeView());
        window_->registry()->refresh();
        const int enabled = changed ? int(action->isEnabled()) : -1;
        window_->closeActiveDocument(false);
        return enabled;
    };
    // 8- and 16-bit RGB and Grayscale take it; 32-bit, CMYK, and Lab do not.
    QCOMPARE(enabledAfter([](pictura::PictureView&) { return true; }), 1);
    QCOMPARE(enabledAfter([](pictura::PictureView& v) { return pictura::convert_image_depth(v, 16); }),
             1);
    QCOMPARE(enabledAfter([](pictura::PictureView& v) { return pictura::convert_image_depth(v, 32); }),
             0);
    for (const char* mode : {"grayscale", "cmyk", "lab"}) {
        const int enabled = enabledAfter([mode](pictura::PictureView& v) {
            return pictura::convert_image_mode(v, QString::fromLatin1(mode));
        });
        QCOMPARE(enabled, qstrcmp(mode, "grayscale") == 0 ? 1 : 0);
    }
}

void LensCorrectionTest::menuRowHasItsShortcut()
{
    QAction* action = window_->registry()->action(pictura::commandIdForPath(kPath));
    QVERIFY(action != nullptr);
    QCOMPARE(action->shortcut(), QKeySequence(QStringLiteral("Shift+Ctrl+R")));
}

void LensCorrectionTest::opensWithControls()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    QCOMPARE(dialog->windowTitle(), QStringLiteral("Lens Correction (100%)"));
    auto* tabs = child<QTabWidget>(dialog.get(), "lcTabs");
    QCOMPARE(tabs->count(), 2);
    QCOMPARE(tabs->tabText(0), QStringLiteral("Auto Correction"));
    QCOMPARE(tabs->tabText(1), QStringLiteral("Custom"));
    QCOMPARE(tabs->currentIndex(), 0);

    // Auto Correction: no profile, so the Correction set is unavailable.
    auto* geometric = child<QCheckBox>(dialog.get(), "lcGeometric");
    QVERIFY(geometric->isChecked() && !geometric->isEnabled());
    QVERIFY(!child<QCheckBox>(dialog.get(), "lcChromatic")->isChecked());
    QVERIFY(!child<QCheckBox>(dialog.get(), "lcVignette")->isChecked());
    QVERIFY(child<QCheckBox>(dialog.get(), "lcAutoScale")->isChecked());
    auto* edge = child<QComboBox>(dialog.get(), "lcEdge");
    QStringList edges;
    for (int i = 0; i < edge->count(); ++i) {
        edges << edge->itemText(i);
    }
    QCOMPARE(edges, QStringList({QStringLiteral("Edge Extension"), QStringLiteral("Transparency"),
                                 QStringLiteral("Black Color"), QStringLiteral("White Color")}));
    QCOMPARE(edge->currentText(), QStringLiteral("Transparency"));
    QCOMPARE(child<QComboBox>(dialog.get(), "lcMake")->currentIndex(), -1);
    auto* profiles = child<QListWidget>(dialog.get(), "lcProfiles");
    QCOMPARE(profiles->count(), 1);
    QCOMPARE(profiles->item(0)->text(), QStringLiteral("No matching profiles found"));
    QCOMPARE(dialog->autoPanel()->profileId(), 0);
    QVERIFY(!child<QPushButton>(dialog.get(), "lcSearchOnline")->isEnabled());

    // Custom: CS6's ranges and openings.
    QCOMPARE(child<QComboBox>(dialog.get(), "lcSettings")->currentText(),
             QStringLiteral("Default Correction"));
    for (const char* name : {"lcDistortion", "lcFringeRed", "lcFringeGreen", "lcFringeBlue"}) {
        QDoubleSpinBox* b = box(dialog.get(), name);
        QVERIFY2(b, name);
        QCOMPARE(b->minimum(), -100.0);
        QCOMPARE(b->maximum(), 100.0);
        QCOMPARE(b->decimals(), 2);
        QCOMPARE(b->value(), 0.0);
    }
    QCOMPARE(box(dialog.get(), "lcVignetteAmount")->value(), 0.0);
    QCOMPARE(box(dialog.get(), "lcVignetteMidpoint")->value(), 50.0);
    QCOMPARE(box(dialog.get(), "lcVertical")->minimum(), -100.0);
    QCOMPARE(box(dialog.get(), "lcHorizontal")->maximum(), 100.0);
    QCOMPARE(box(dialog.get(), "lcAngle")->value(), 0.0);
    QCOMPARE(box(dialog.get(), "lcScale")->value(), 100.0);
    QCOMPARE(box(dialog.get(), "lcScale")->minimum(), 50.0);
    QCOMPARE(box(dialog.get(), "lcScale")->maximum(), 150.0);

    QVERIFY(child<QCheckBox>(dialog.get(), "lcPreview")->isChecked());
    QVERIFY(!child<QCheckBox>(dialog.get(), "lcShowGrid")->isChecked());
    QCOMPARE(child<QSpinBox>(dialog.get(), "lcGridSize")->value(), 64);
    QCOMPARE(child<QLabel>(dialog.get(), "lcCamera")->text(), QStringLiteral("Camera Model: --"));
    QCOMPARE(child<QLabel>(dialog.get(), "lcLens")->text(), QStringLiteral("Lens Model: --"));
    QCOMPARE(child<QLabel>(dialog.get(), "lcCameraSettings")->text(),
             QStringLiteral("Camera Settings: --mm, f/--, --m"));
    QCOMPARE(dialog->tool(), int(LensCorrectionDialog::RemoveDistortion));
    QCOMPARE(dialog->values().size(), 16);
    dialog->reject();
}

void LensCorrectionTest::exifPrefillsSearchAndStatus()
{
    const QString fixture = metadataFixture();
    if (fixture.isEmpty()) {
        QSKIP("metadata.psd fixture not found");
    }
    QVERIFY(window_->openDocumentAtPath(fixture));
    {
        auto dialog = showDialog();
        QVERIFY(dialog && dialog->ready());
        QCOMPARE(child<QComboBox>(dialog.get(), "lcMake")->currentText(),
                 QStringLiteral("ACME Cameras"));
        QCOMPARE(child<QComboBox>(dialog.get(), "lcModel")->currentText(),
                 QStringLiteral("ACME One"));
        const QString camera = child<QLabel>(dialog.get(), "lcCamera")->text();
        QVERIFY2(camera.contains(QStringLiteral("ACME Cameras ACME One")), qPrintable(camera));
        const QString settings = child<QLabel>(dialog.get(), "lcCameraSettings")->text();
        QVERIFY2(settings.contains(QStringLiteral("50mm, f/2.8")), qPrintable(settings));
        // The synthetic profile matches and is chosen; its corrections apply.
        QCOMPARE(dialog->autoPanel()->profileId(), 1);
        auto* geometric = child<QCheckBox>(dialog.get(), "lcGeometric");
        QVERIFY(geometric->isEnabled());
        auto* vignette = child<QCheckBox>(dialog.get(), "lcVignette");
        const QImage before = dialog->canvas()->previewImage();
        vignette->setChecked(true);
        QVERIFY(dialog->canvas()->previewImage() != before);
        // No criteria match nothing.
        child<QComboBox>(dialog.get(), "lcMake")->setCurrentIndex(-1);
        QCOMPARE(dialog->autoPanel()->profileId(), 0);
        dialog->reject();
    }
    QVERIFY(window_->closeActiveDocument(false));
}

void LensCorrectionTest::settingsComboSwitchesToCustom()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    auto* settings = child<QComboBox>(dialog.get(), "lcSettings");
    box(dialog.get(), "lcVertical")->setValue(30);
    box(dialog.get(), "lcDistortion")->setValue(12.5);
    QCOMPARE(settings->currentIndex(), int(LensCustomPanel::Custom));
    dialog->customPanel()->applySetting(LensCustomPanel::DefaultCorrection);
    QCOMPARE(settings->currentIndex(), int(LensCustomPanel::DefaultCorrection));
    QCOMPARE(box(dialog.get(), "lcDistortion")->value(), 0.0);
    // The transform is per image; Settings leaves it.
    QCOMPARE(box(dialog.get(), "lcVertical")->value(), 30.0);
    dialog->reject();
}

void LensCorrectionTest::removeDistortionToolDragsSlider()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    LensCorrectionCanvas* canvas = dialog->canvas();
    auto* distortion = box(dialog.get(), "lcDistortion");
    // Toward the centre corrects barrel distortion (positive)...
    drag(canvas, QPointF(280, 180), QPointF(220, 140));
    QVERIFY2(distortion->value() > 0.0, qPrintable(QString::number(distortion->value())));
    // ...and away from it pincushion.
    distortion->setValue(0);
    drag(canvas, QPointF(200, 130), QPointF(290, 190));
    QVERIFY(distortion->value() < 0.0);
    dialog->reject();
}

void LensCorrectionTest::straightenToolSetsAngle()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    dialog->selectTool(LensCorrectionDialog::Straighten);
    // A line leaning clockwise is levelled by turning the other way.
    drag(dialog->canvas(), QPointF(50, 100), QPointF(250, 120));
    const double expected = 360.0 - std::atan2(20.0, 200.0) * 180.0 / M_PI;
    QVERIFY(std::abs(box(dialog.get(), "lcAngle")->value() - expected) <= 0.011);
    QCOMPARE(child<QComboBox>(dialog.get(), "lcSettings")->currentIndex(),
             int(LensCustomPanel::Custom));
    dialog->reject();
}

void LensCorrectionTest::angleDialAndFieldSync()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    auto* dial = child<pictura::AngleDial>(dialog.get(), "lcAngleDial");
    auto* angle = box(dialog.get(), "lcAngle");
    // The dial points up at 0° and turns clockwise.
    QCOMPARE(dial->angle(), 90.0);
    angle->setValue(30);
    QCOMPARE(dial->angle(), 60.0);
    emit dial->angleChanged(0.0);
    QCOMPARE(angle->value(), 90.0);
    dialog->reject();
}

void LensCorrectionTest::moveGridToolOffsetsGrid()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    dialog->selectTool(LensCorrectionDialog::MoveGrid);
    drag(dialog->canvas(), QPointF(100, 100), QPointF(110, 105));
    QCOMPARE(dialog->canvas()->gridOffset(), QPointF(10, 5));
    // Past a whole cell, the offset wraps.
    drag(dialog->canvas(), QPointF(100, 100), QPointF(170, 100));
    QCOMPARE(dialog->canvas()->gridOffset(), QPointF(16, 5));
    dialog->reject();
}

void LensCorrectionTest::showGridSizeAndColor()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    LensCorrectionCanvas* canvas = dialog->canvas();
    child<QCheckBox>(dialog.get(), "lcShowGrid")->setChecked(true);
    QVERIFY(canvas->gridShown());
    child<QSpinBox>(dialog.get(), "lcGridSize")->setValue(32);
    QCOMPARE(canvas->gridSize(), 32);
    // 300×200 at 32: x = 0…288 (10 lines), y = 0…192 (7 lines).
    QCOMPARE(canvas->gridLinesForTest().size(), 17);
    QCOMPARE(child<pictura::ColorSwatchWidget>(dialog.get(), "lcGridColor")->color(),
             canvas->gridColor());
    dialog->reject();
}

void LensCorrectionTest::toolShortcuts()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    dialog->canvas()->setFocus();
    const struct {
        Qt::Key key;
        LensCorrectionDialog::Tool tool;
    } keys[] = {{Qt::Key_A, LensCorrectionDialog::Straighten},
                {Qt::Key_M, LensCorrectionDialog::MoveGrid},
                {Qt::Key_H, LensCorrectionDialog::Hand},
                {Qt::Key_Z, LensCorrectionDialog::Zoom},
                {Qt::Key_D, LensCorrectionDialog::RemoveDistortion}};
    for (const auto& k : keys) {
        QTest::keyClick(dialog.get(), k.key);
        QCOMPARE(dialog->tool(), int(k.tool));
        QCOMPARE(int(dialog->canvas()->mode()), int(k.tool));
    }
    dialog->reject();
}

void LensCorrectionTest::hintFollowsTool()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    QCOMPARE(dialog->helpText(),
             QStringLiteral("Drag towards or away from center to correct distortion."));
    dialog->selectTool(LensCorrectionDialog::Straighten);
    QVERIFY(dialog->helpText().contains(QStringLiteral("straighten")));
    dialog->selectTool(LensCorrectionDialog::MoveGrid);
    QVERIFY(dialog->helpText().contains(QStringLiteral("grid")));
    dialog->reject();
}

void LensCorrectionTest::previewToggleShowsSource()
{
    auto dialog = showDialog();
    QVERIFY(dialog);
    LensCorrectionCanvas* canvas = dialog->canvas();
    box(dialog.get(), "lcScale")->setValue(50);
    const QPointF corner(2, 2);
    QCOMPARE(viewPixel(canvas, corner).alpha(), 0);
    auto* preview = child<QCheckBox>(dialog.get(), "lcPreview");
    preview->setChecked(false);
    const QColor source = viewPixel(canvas, corner);
    QCOMPARE(source.alpha(), 255);
    QCOMPARE(source.rgb(), window_->activeView()->image().pixelColor(2, 2).rgb());
    preview->setChecked(true);
    QCOMPARE(viewPixel(canvas, corner).alpha(), 0);
    // Black Color fills the blank.
    child<QComboBox>(dialog.get(), "lcEdge")->setCurrentIndex(2);
    QCOMPARE(viewPixel(canvas, corner), QColor(Qt::black));
    dialog->reject();
}

void LensCorrectionTest::grayscaleDisablesChromaticAberration()
{
    QVERIFY(window_->newDocument(QStringLiteral("Gray"), 64, 64, QStringLiteral("grayscale"), 8,
                                 QStringLiteral("white")));
    {
        auto dialog = showDialog();
        QVERIFY(dialog && dialog->ready());
        QVERIFY(!box(dialog.get(), "lcFringeRed")->isEnabled());
        QVERIFY(!box(dialog.get(), "lcFringeGreen")->isEnabled());
        QVERIFY(!child<QCheckBox>(dialog.get(), "lcChromatic")->isEnabled());
        QCOMPARE(dialog->values().at(1), 0.0);
        dialog->reject();
    }
    QVERIFY(window_->closeActiveDocument(false));
}

void LensCorrectionTest::cancelAndOk()
{
    pictura::PictureView* view = window_->activeView();
    const QImage document = view->image();
    const int history = view->history_count();
    const QString id = pictura::commandIdForPath(kPath);
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (LensCorrectionDialog* dialog = openDialog()) {
            opened = true;
            box(dialog, "lcDistortion")->setValue(40);
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(id));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), document);

    QTimer::singleShot(0, [] {
        if (LensCorrectionDialog* dialog = openDialog()) {
            box(dialog, "lcDistortion")->setValue(40);
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(id));
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("lens-correction"));
    QCOMPARE(filter_last_params(*view).size(), 16);
    QVERIFY(view->image() != document);
}

void LensCorrectionTest::previousConversionRestoresLastCommit()
{
    QTimer::singleShot(0, [] {
        if (LensCorrectionDialog* dialog = openDialog()) {
            box(dialog, "lcDistortion")->setValue(25);
            box(dialog, "lcFringeRed")->setValue(10);
            box(dialog, "lcVertical")->setValue(30);
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kPath)));
    auto dialog = showDialog();
    QVERIFY(dialog);
    auto* settings = child<QComboBox>(dialog.get(), "lcSettings");
    auto* model = qobject_cast<QStandardItemModel*>(settings->model());
    QVERIFY(model->item(LensCustomPanel::PreviousConversion)->isEnabled());
    dialog->customPanel()->applySetting(LensCustomPanel::PreviousConversion);
    QCOMPARE(box(dialog.get(), "lcDistortion")->value(), 25.0);
    QCOMPARE(box(dialog.get(), "lcFringeRed")->value(), 10.0);
    // Perspective is not part of a saved conversion.
    QCOMPARE(box(dialog.get(), "lcVertical")->value(), 0.0);
    QCOMPARE(settings->currentIndex(), int(LensCustomPanel::PreviousConversion));
    dialog->reject();
}

void LensCorrectionTest::lastSettingsRestoresValues()
{
    pictura::PictureView* view = window_->activeView();
    QTimer::singleShot(0, [] {
        if (LensCorrectionDialog* dialog = openDialog()) {
            box(dialog, "lcDistortion")->setValue(33);
            box(dialog, "lcVertical")->setValue(20);
            child<QComboBox>(dialog, "lcEdge")->setCurrentIndex(2);
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kPath)));
    const int history = view->history_count();
    double distortion = -1;
    double vertical = -1;
    int edge = -1;
    QTimer::singleShot(0, [&] {
        if (LensCorrectionDialog* dialog = openDialog()) {
            distortion = box(dialog, "lcDistortion")->value();
            vertical = box(dialog, "lcVertical")->value();
            edge = child<QComboBox>(dialog, "lcEdge")->currentIndex();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(distortion, 33.0);
    QCOMPARE(vertical, 20.0);
    QCOMPARE(edge, 2);
    QCOMPARE(view->history_count(), history);
}

void LensCorrectionTest::lastFilterRefusesOffSixteenBit()
{
    QImage picture(60, 40, QImage::Format_RGB32);
    picture.fill(QColor(120, 160, 200));
    const QString path = dir_.filePath(QStringLiteral("deep.png"));
    QVERIFY(picture.save(path) && window_->openDocumentAtPath(path));
    pictura::PictureView* view = window_->activeView();
    QTimer::singleShot(0, [] {
        if (LensCorrectionDialog* dialog = openDialog()) {
            box(dialog, "lcDistortion")->setValue(20);
            dialog->accept();
        }
    });
    window_->registry()->dispatch(pictura::commandIdForPath(kPath));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("lens-correction"));
    // 16-bit still takes it; 32-bit does not, on any path.
    QVERIFY(pictura::convert_image_depth(*view, 16));
    const int at16 = view->history_count();
    window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QCOMPARE(view->history_count(), at16 + 1);
    QVERIFY(pictura::convert_image_depth(*view, 32));
    const int history = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (LensCorrectionDialog* dialog = openDialog()) {
            opened = true;
            dialog->accept();
        }
    });
    window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings));
    QVERIFY(!opened);
    window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QCOMPARE(view->history_count(), history);
    QVERIFY(filter_last_error(*view).contains(QStringLiteral("8- or 16-bit")));
    QVERIFY(window_->closeActiveDocument(false));
}

QTEST_MAIN(LensCorrectionTest)
#include "tst_lens_correction.moc"
