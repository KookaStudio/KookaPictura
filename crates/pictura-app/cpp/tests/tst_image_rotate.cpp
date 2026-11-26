// Image > Image Rotation > Arbitrary (#282): rotates the document about its
// centre by an arbitrary angle through pictura_render::rotate_document_in.
// The rotations and flips are gated as spilling commits (#1294).

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtCore/QScopeGuard>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QStatusBar>

#include <functional>
#include <limits>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "preferences.h"
#include "preferences_dialog.h"
#include "selftest_fixture.h"
#include "qt_test_support.h"

#include "api/filters_bridge.h"
#include "api/paint_bridge.h"

namespace {

using pictura::PictureView;

const QString kRotate = QString::fromLatin1(pictura::command_ids::ImageRotateArbitrary);

bool withRotationDialog(pictura::PicturaMainWindow& frame,
                        const std::function<void(QDialog&)>& answer,
                        const std::function<void()>& dispatch)
{
    bool seen = false;
    QTimer probe;
    QTimer watchdog;
    watchdog.setSingleShot(true);
    QObject::connect(&probe, &QTimer::timeout, &frame, [&] {
        auto* dialog = frame.findChild<QDialog*>(QStringLiteral("rotateCanvasDialog"));
        if (dialog && dialog->isVisible() && !seen) {
            seen = true;
            answer(*dialog);
            if (dialog->isVisible()) {
                dialog->reject();
            }
        }
    });
    QObject::connect(&watchdog, &QTimer::timeout, &frame, [&] {
        for (auto* dialog : frame.findChildren<QDialog*>()) {
            if (dialog->isVisible()) {
                dialog->reject();
            }
        }
    });
    probe.start(10);
    watchdog.start(3000);
    dispatch();
    return seen;
}

QLineEdit* angleField(QDialog& dialog)
{
    return dialog.findChild<QLineEdit*>(QStringLiteral("rotateCanvasAngle"));
}

void enterAngle(QDialog& dialog, double degrees)
{
    if (auto* field = angleField(dialog)) {
        field->setText(field->locale().toString(degrees, 'f', 2));
        dialog.accept();
    }
}

QImage contrastImage()
{
    QImage image(12, 9, QImage::Format_RGB32);
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            image.setPixelColor(x, y, (x + 2 * y) % 3 == 0 ? Qt::white : Qt::black);
        }
    }
    return image;
}

QImage pixels(PictureView& view)
{
    return view.display_image(0, 0, 0, view.document_width(), view.document_height());
}

} // namespace

class ImageRotateTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void arbitraryRotatesAndRecords();
    void invalidAngleIsRefused();
    void rotationRefusalsNameMemory();
    void dialogDirectionRotatesAndUndoes();
    void dialogRefusesEmptyAndZero();
    void rulerPrefillsCorrection();
    void rulerClearsOnlyAfterSuccessfulRotation();
    void interactiveRotationUsesSavedInterpolation();
    void rotationPlaybackUsesRecordedInterpolation();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    void storeInterpolation(const QString& interpolation);
};

void ImageRotateTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ImageRotateTest::arbitraryRotatesAndRecords()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Rot"), 20, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int before = view->history_count();

    QVERIFY((pictura::rotate_document_arbitrary_with(*view, 30.0, QStringLiteral("bilinear"))));
    // The 20x16 canvas grows to the rotated bounding box.
    QVERIFY(view->document_width() != 20 || view->document_height() != 16);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_count() - 1), QStringLiteral("Rotate"));

    // Undo restores the original dimensions.
    QVERIFY(view->undo());
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 16);

    frame.closeDocument(doc, false);
}

void ImageRotateTest::invalidAngleIsRefused()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("RotBad"), 12, 12, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    const int before = view->history_count();
    QVERIFY(!(pictura::rotate_document_arbitrary_with(*view, 0.0, QStringLiteral("bilinear"))));
    QVERIFY(!(pictura::rotate_document_arbitrary_with(*view, std::numeric_limits<double>::quiet_NaN(),
                                                  QStringLiteral("bilinear"))));
    QVERIFY(!(pictura::rotate_document_arbitrary_with(*view, 360.0, QStringLiteral("bilinear"))));
    QCOMPARE(view->history_count(), before);
    QCOMPARE(view->document_width(), 12);
    QCOMPARE(view->document_height(), 12);

    frame.closeDocument(doc, false);
}

// [ocg_rotate_refused] 4 MiB of room, under one worker's window, and no
// scratch space: every rotation and flip is a memory refusal.
void ImageRotateTest::rotationRefusalsNameMemory()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("RotRefused"), 40, 20, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int history = view->history_count();
    pictura::set_ruler(*view, 2.0, 2.0, 16.0, 5.0, false);
    pictura::set_available_memory_for_test(260LL * 1024 * 1024);
    pictura::set_scratch_available_for_test(0);

    const auto restoreMemory = qScopeGuard([] {
        pictura::set_available_memory_for_test(-1);
        pictura::set_scratch_available_for_test(-1);
    });
    const struct {
        const char* id;
        const char* action;
    } commands[] = {
        {pictura::command_ids::ImageRotate90Cw, "rotate the canvas"},
        {pictura::command_ids::ImageFlipHorizontal, "flip the canvas"},
        {pictura::command_ids::ImageRotateArbitrary, "rotate the canvas"},
    };
    for (const auto& command : commands) {
        frame.statusBar()->clearMessage();
        const QString id = QString::fromLatin1(command.id);
        if (id == kRotate) {
            bool dispatched = false;
            QVERIFY(withRotationDialog(frame, [](QDialog& dialog) { enterAngle(dialog, 15.0); },
                                       [&] { dispatched = frame.registry()->dispatch(id); }));
            QVERIFY(dispatched);
        } else {
            QVERIFY(frame.registry()->dispatch(id));
        }
        const QString message = frame.statusBar()->currentMessage();
        QVERIFY2(message.contains(QLatin1String(command.action)), qPrintable(message));
        QVERIFY2(message.contains(QStringLiteral("memory"), Qt::CaseInsensitive),
                 qPrintable(message));
        QCOMPARE(view->document_width(), 40);
        QCOMPARE(view->document_height(), 20);
        QCOMPARE(view->history_count(), history);
        QCOMPARE(pictura::ruler_line(*view).size(), std::size_t(4));
    }
    frame.closeDocument(doc, false);
}

void ImageRotateTest::dialogDirectionRotatesAndUndoes()
{
    pictura::selftest_fixture::DocumentFixture f(
        *window_, contrastImage(), QStringLiteral("pictura_rotate_ccw"));
    QVERIFY(f.ok());
    const QImage before = pixels(*f.view);
    QVERIFY(!before.isNull());
    const int history = f.view->history_count();

    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) {
        if (auto* ccw = dialog.findChild<QRadioButton*>(QStringLiteral("rotateCanvasCounterclockwise"))) {
            ccw->click();
            enterAngle(dialog, 90.0);
        }
    }, [&] { window_->registry()->dispatch(kRotate); }));

    QCOMPARE(f.view->history_count(), history + 1);
    const QImage after = pixels(*f.view);
    QCOMPARE(after.size(), QSize(before.height(), before.width()));
    for (int y = 0; y < before.height(); ++y) {
        for (int x = 0; x < before.width(); ++x) {
            QCOMPARE(after.pixel(y, before.width() - 1 - x), before.pixel(x, y));
        }
    }
    QVERIFY(f.view->undo());
    QCOMPARE(pixels(*f.view), before);
}

void ImageRotateTest::dialogRefusesEmptyAndZero()
{
    pictura::selftest_fixture::DocumentFixture f(
        *window_, contrastImage(), QStringLiteral("pictura_rotate_refusal"));
    QVERIFY(f.ok());
    const auto before = pixels(*f.view);
    const int history = f.view->history_count();
    const auto dispatch = [&] { window_->registry()->dispatch(kRotate); };

    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) { dialog.reject(); }, dispatch));
    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) { enterAngle(dialog, 0.0); }, dispatch));

    for (const QString& invalid : {QString(), QStringLiteral("360"), QStringLiteral("word")}) {
        bool disabled = false;
        QVERIFY(withRotationDialog(*window_, [&](QDialog& dialog) {
            auto* field = angleField(dialog);
            auto* buttons = dialog.findChild<QDialogButtonBox*>();
            if (field && buttons) {
                field->setText(invalid);
                disabled = !buttons->button(QDialogButtonBox::Ok)->isEnabled();
                dialog.accept();
            }
        }, dispatch));
        QVERIFY(disabled);
    }
    QCOMPARE(f.view->history_count(), history);
    QCOMPARE(pixels(*f.view), before);
}

void ImageRotateTest::rulerPrefillsCorrection()
{
    pictura::selftest_fixture::DocumentFixture f(
        *window_, contrastImage(), QStringLiteral("pictura_rotate_ruler"));
    QVERIFY(f.ok());
    const auto dispatch = [&] { window_->registry()->dispatch(kRotate); };
    for (bool reverse : {false, true}) {
        pictura::set_ruler(*f.view, reverse ? 10.0 : 0.0, reverse ? 2.0 : 0.0,
                          reverse ? 0.0 : 10.0, reverse ? 0.0 : 2.0, false);
        double angle = -1.0;
        bool ccw = false;
        QVERIFY(withRotationDialog(*window_, [&](QDialog& dialog) {
            if (auto* field = angleField(dialog)) {
                angle = field->locale().toDouble(field->text());
            }
            if (auto* radio = dialog.findChild<QRadioButton*>(QStringLiteral("rotateCanvasCounterclockwise"))) {
                ccw = radio->isChecked();
            }
        }, dispatch));
        QCOMPARE(angle, 11.31);
        QVERIFY(ccw);
    }
    pictura::clear_ruler(*f.view);
    double angle = -1.0;
    bool cw = false;
    QVERIFY(withRotationDialog(*window_, [&](QDialog& dialog) {
        if (auto* field = angleField(dialog)) {
            angle = field->locale().toDouble(field->text());
        }
        if (auto* radio = dialog.findChild<QRadioButton*>(QStringLiteral("rotateCanvasClockwise"))) {
            cw = radio->isChecked();
        }
    }, dispatch));
    QCOMPARE(angle, 0.0);
    QVERIFY(cw);
}

void ImageRotateTest::rulerClearsOnlyAfterSuccessfulRotation()
{
    pictura::selftest_fixture::DocumentFixture other(
        *window_, contrastImage(), QStringLiteral("pictura_rotate_other_ruler"));
    QVERIFY(other.ok());
    pictura::set_ruler(*other.view, 1.0, 2.0, 5.0, 4.0, false);

    pictura::selftest_fixture::DocumentFixture f(
        *window_, contrastImage(), QStringLiteral("pictura_rotate_ruler_cleanup"));
    QVERIFY(f.ok());
    const auto restoreTool = qScopeGuard([this, tool = window_->activeTool()] {
        window_->setActiveTool(tool);
    });
    window_->setActiveTool(pictura::ToolId::Ruler);
    f.canvas->mousePressed(QPointF(1, 1), Qt::LeftButton, Qt::NoModifier);
    f.canvas->mouseMoved(QPointF(11, 3));
    f.canvas->mouseReleased(QPointF(11, 3));
    QVERIFY(f.canvas->hasRulerLineForTest());
    QCOMPARE(pictura::ruler_line(*f.view).size(), std::size_t(4));

    const int history = f.view->history_count();
    const auto dispatch = [&] { window_->registry()->dispatch(kRotate); };
    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) { dialog.reject(); }, dispatch));
    QVERIFY(f.canvas->hasRulerLineForTest());
    QCOMPARE(pictura::ruler_line(*f.view).size(), std::size_t(4));
    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) { enterAngle(dialog, 0.0); }, dispatch));
    QVERIFY(f.canvas->hasRulerLineForTest());
    QCOMPARE(pictura::ruler_line(*f.view).size(), std::size_t(4));
    QCOMPARE(f.view->history_count(), history);

    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) { dialog.accept(); }, dispatch));
    QCOMPARE(f.view->history_count(), history + 1);
    QVERIFY(!f.canvas->hasRulerLineForTest());
    QVERIFY(pictura::ruler_line(*f.view).empty());

    const auto otherLine = pictura::ruler_line(*other.view);
    QCOMPARE(otherLine.size(), std::size_t(4));
    QCOMPARE(otherLine[0], 1.0);
    QCOMPARE(otherLine[1], 2.0);
    QCOMPARE(otherLine[2], 5.0);
    QCOMPARE(otherLine[3], 4.0);
}

void ImageRotateTest::storeInterpolation(const QString& interpolation)
{
    const auto before = pictura::loadPreferences();
    auto after = before;
    after.imageInterpolation = interpolation;
    QVERIFY(pictura::savePreferences(before, after));
}

void ImageRotateTest::interactiveRotationUsesSavedInterpolation()
{
    storeInterpolation(QStringLiteral("bicubic"));
    window_->showPreferences(pictura::PreferencesDialog::kGeneral);
    auto* preferences = window_->findChild<pictura::PreferencesDialog*>();
    QVERIFY(preferences);
    auto* choice = preferences->findChild<QComboBox*>(QStringLiteral("preferencesImageInterpolation"));
    QVERIFY(choice);
    QCOMPARE(choice->currentData().toString(), QStringLiteral("bicubic"));
    const int nearest = choice->findData(QStringLiteral("nearest"));
    QVERIFY(nearest >= 0);
    choice->setCurrentIndex(nearest);
    preferences->hide();
    QCOMPARE(pictura::loadPreferences().imageInterpolation, QStringLiteral("nearest"));
    window_->showPreferences(pictura::PreferencesDialog::kGeneral);
    QCOMPARE(choice->currentData().toString(), QStringLiteral("nearest"));
    preferences->hide();

    pictura::selftest_fixture::DocumentFixture f(
        *window_, contrastImage(), QStringLiteral("pictura_rotate_interpolation"));
    QVERIFY(f.ok());
    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) { enterAngle(dialog, 24.0); },
                               [&] { window_->registry()->dispatch(kRotate); }));
    const auto interactive = pixels(*f.view);
    QVERIFY(f.view->undo());
    QVERIFY(pictura::rotate_document_arbitrary_with(*f.view, 24.0, QStringLiteral("nearest")));
    QCOMPARE(pixels(*f.view), interactive);
}

void ImageRotateTest::rotationPlaybackUsesRecordedInterpolation()
{
    storeInterpolation(QStringLiteral("nearest"));
    pictura::selftest_fixture::DocumentFixture f(
        *window_, contrastImage(), QStringLiteral("pictura_rotate_action"));
    QVERIFY(f.ok());
    auto* registry = window_->registry();
    pictura::CommandParams recorded;
    registry->setRecordingSink([&](const QString& id, const pictura::CommandParams& params) {
        if (id == kRotate) {
            recorded = params;
        }
    });
    const bool shown = withRotationDialog(*window_, [](QDialog& dialog) {
        enterAngle(dialog, -24.0);
    }, [&] { registry->dispatch(kRotate); });
    registry->setRecordingSink({});
    QVERIFY(shown);
    const auto nearest = pixels(*f.view);
    QCOMPARE(recorded.values.value(QStringLiteral("angle")).toDouble(), -24.0);
    QCOMPARE(recorded.values.value(QStringLiteral("interpolation")).toString(), QStringLiteral("nearest"));

    QVERIFY(f.view->undo());
    storeInterpolation(QStringLiteral("bicubic"));
    QVERIFY(registry->executeRecorded(kRotate, recorded));
    QCOMPARE(pixels(*f.view), nearest);

    QVERIFY(f.view->undo());
    bool applied = false;
    QVERIFY(withRotationDialog(*window_, [](QDialog& dialog) { dialog.accept(); },
                               [&] { applied = registry->executeRecorded(
                                   kRotate, recorded, pictura::CommandMode::Modal); }));
    QVERIFY(applied);
    QCOMPARE(pixels(*f.view), nearest);

    QVERIFY(f.view->undo());
    auto legacy = recorded;
    legacy.values.remove(QStringLiteral("interpolation"));
    QVERIFY(registry->executeRecorded(kRotate, legacy));
    const auto bilinear = pixels(*f.view);
    QVERIFY(bilinear != nearest);

    QVERIFY(f.view->undo());
    QVERIFY(pictura::rotate_document_arbitrary(*f.view, -24.0));
    QCOMPARE(pixels(*f.view), bilinear);
}

PICTURA_QT_TEST_MAIN(ImageRotateTest)
#include "tst_image_rotate.moc"
