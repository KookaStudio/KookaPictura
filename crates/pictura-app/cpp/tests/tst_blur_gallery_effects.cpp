#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QToolButton>

#include "blur_gallery_canvas.h"
#include "blur_gallery_dialog.h"
#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

// The Blur Gallery's Blur Tools rows: every implemented effect can be
// selected, switched on, and expanded, and the ones switched on combine.

namespace {

using Mode = pictura::BlurGalleryMode;

const pictura::FilterCommandSpec& spec(const QString& leaf)
{
    return *pictura::filterCommandForPath(
        {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"), leaf});
}

QString menuId(const QString& leaf)
{
    return pictura::commandIdForPath(
        {QStringLiteral("Filter"), QStringLiteral("Blur Gallery"), leaf});
}

pictura::BlurGalleryDialog* openWorkspace()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::BlurGalleryDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

QImage pattern()
{
    QImage image(96, 64, QImage::Format_RGB32);
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 96; ++x) {
            image.setPixelColor(x, y, QColor((x * 4) % 256, (y * 4) % 256, (x * 9) % 256));
        }
    }
    return image;
}

QList<double> defaultTail()
{
    return {1.0, 14.0, 40.0, 40.0, 118.0, 1.0, 0.0, 50.0, 50.0, 0.0, 0.0, 0.0, 1.0};
}

void click(QWidget* dialog, const QString& name)
{
    auto* button = dialog->findChild<QToolButton*>(name);
    QVERIFY2(button != nullptr, qPrintable(name));
    button->click();
}

bool shown(QWidget* dialog, const QString& spin)
{
    return dialog->findChild<QDoubleSpinBox*>(spin)->isVisibleTo(dialog);
}

} // namespace

class BlurGalleryEffectsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void aRowNameSelectsAndExpandsItsEffect();
    void theArrowOnlyExpands();
    void theCheckboxSwitchesAnEffect();
    void pinsStayWithTheirEffect();
    void theCombinedListRoundTrips();
    void combinedPreviewBlursWhereEitherDoes();
    void menuCommitsTheCombinedGalleryAndReopensIt();
    void okWithEveryEffectOffCommitsNothing();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void BlurGalleryEffectsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern().save(path) && window_->openDocumentAtPath(path));
}

void BlurGalleryEffectsTest::aRowNameSelectsAndExpandsItsEffect()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spec(QStringLiteral("Field Blur")));
    QCOMPARE(dialog.selectedEffect(), Mode::Field);
    QCOMPARE(dialog.kind(), QStringLiteral("field-blur"));
    QVERIFY(shown(&dialog, QStringLiteral("fieldBlurSpin")));

    click(&dialog, QStringLiteral("irisBlurSelect"));
    QCOMPARE(dialog.selectedEffect(), Mode::Iris);
    QCOMPARE(dialog.canvas()->mode(), Mode::Iris);
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("irisBlurEnabled"))->isChecked());
    QVERIFY(dialog.effectEnabled(Mode::Field) && dialog.effectEnabled(Mode::Iris));
    QVERIFY(shown(&dialog, QStringLiteral("irisBlurSpin")));
    // Selecting another effect keeps the first one expanded and on.
    QVERIFY(shown(&dialog, QStringLiteral("fieldBlurSpin")));
    QCOMPARE(dialog.kind(), QStringLiteral("blur-gallery"));
    // The new effect opens on its default pin.
    QCOMPARE(dialog.findChild<QDoubleSpinBox*>(QStringLiteral("irisBlurSpin"))->value(), 15.0);
    dialog.reject();
}

void BlurGalleryEffectsTest::theArrowOnlyExpands()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spec(QStringLiteral("Iris Blur")));
    QVERIFY(!shown(&dialog, QStringLiteral("tiltShiftSpin")));
    click(&dialog, QStringLiteral("tiltShiftEnabledArrow"));
    QVERIFY(shown(&dialog, QStringLiteral("tiltShiftSpin")));
    QVERIFY(dialog.findChild<QWidget*>(QStringLiteral("tiltDistortionSpin"))->isVisibleTo(&dialog));
    QCOMPARE(dialog.selectedEffect(), Mode::Iris);
    QVERIFY(!dialog.effectEnabled(Mode::TiltShift));
    click(&dialog, QStringLiteral("irisBlurEnabledArrow"));
    QVERIFY(!shown(&dialog, QStringLiteral("irisBlurSpin")));
    QCOMPARE(dialog.selectedEffect(), Mode::Iris);
    dialog.reject();
}

void BlurGalleryEffectsTest::theCheckboxSwitchesAnEffect()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spec(QStringLiteral("Field Blur")));
    dialog.findChild<QCheckBox*>(QStringLiteral("tiltShiftEnabled"))->setChecked(true);
    QCOMPARE(dialog.selectedEffect(), Mode::TiltShift);
    QCOMPARE(dialog.kind(), QStringLiteral("blur-gallery"));
    // Field, Iris, Tilt-Shift, Path counts and records, the motion section,
    // the tail.
    QCOMPARE(dialog.values().size(), 1 + 3 + 1 + 1 + 10 + 1 + 6 + 13);

    // Field off: Tilt-Shift alone commits as itself.
    dialog.findChild<QCheckBox*>(QStringLiteral("fieldBlurEnabled"))->setChecked(false);
    QVERIFY(!dialog.effectEnabled(Mode::Field));
    QCOMPARE(dialog.kind(), QStringLiteral("tilt-shift"));
    QList<double> tilt = {1.0, 0.5, 0.5, 0.0, 0.1, 0.1, 0.15, 0.15, 0.0, 0.0, 15.0};
    tilt += defaultTail();
    QCOMPARE(dialog.values(), tilt);

    // Field back on keeps its pin.
    dialog.findChild<QCheckBox*>(QStringLiteral("fieldBlurEnabled"))->setChecked(true);
    QCOMPARE(dialog.kind(), QStringLiteral("blur-gallery"));
    QCOMPARE(dialog.values().at(0), 1.0);
    dialog.reject();
}

void BlurGalleryEffectsTest::pinsStayWithTheirEffect()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spec(QStringLiteral("Field Blur")));
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* canvas = dialog.canvas();
    const QPoint corner = canvas->rect().center() - QPoint(150, 100);
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, corner);
    QCOMPARE(dialog.values().at(0), 2.0);
    QCOMPARE(canvas->selected(), 1);

    click(&dialog, QStringLiteral("irisBlurSelect"));
    QCOMPARE(canvas->selected(), 0);
    QList<double> v = dialog.values();
    QCOMPARE(v.at(0), 2.0); // the Field pins stay
    QCOMPARE(v.at(7), 1.0); // one iris

    click(&dialog, QStringLiteral("fieldBlurSelect"));
    QCOMPARE(canvas->mode(), Mode::Field);
    QCOMPARE(canvas->selected(), 1);
    dialog.reject();
}

void BlurGalleryEffectsTest::theCombinedListRoundTrips()
{
    const pictura::FilterCommandSpec* gallery =
        pictura::filterCommandForKind(QStringLiteral("blur-gallery"));
    QVERIFY(gallery != nullptr);
    QCOMPARE(gallery->label, QStringLiteral("Blur Gallery"));
    QCOMPARE(int(gallery->params.size()), pictura::filter_param_arity(gallery->kind));

    pictura::BlurGalleryDialog dialog(window_->activeView(), *gallery);
    QList<double> list = {0.0, 1.0, 0.3, 0.4, 0.2, 0.1, 30.0, 60.0, 0.5, 12.0, 1.0,
                          0.5, 0.4, 15.0, 0.1, 0.05, 0.2, 0.3, 40.0, 1.0, 20.0,
                          0.0, // no paths
                          50.0, 0.0, 1.0, 0.0, 0.0, 4.0}; // the motion section
    list += defaultTail();
    dialog.setValues(list);
    QCOMPARE(dialog.kind(), QStringLiteral("blur-gallery"));
    QCOMPARE(dialog.values(), list);
    QVERIFY(!dialog.effectEnabled(Mode::Field));
    QCOMPARE(dialog.selectedEffect(), Mode::Iris);
    dialog.reject();
}

void BlurGalleryEffectsTest::combinedPreviewBlursWhereEitherDoes()
{
    pictura::BlurGalleryDialog dialog(window_->activeView(), spec(QStringLiteral("Tilt-Shift")));
    dialog.findChild<QCheckBox*>(QStringLiteral("bokehEnabled"))->setChecked(false);
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(false);
    QCoreApplication::processEvents();
    const QImage unfiltered = dialog.canvas()->image();
    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(true);
    QTRY_VERIFY(dialog.canvas()->image() != unfiltered);
    // Alone, the band keeps its centre row sharp...
    QCOMPARE(dialog.canvas()->image().pixel(10, 32), unfiltered.pixel(10, 32));
    // ...and a Field pin blurring everywhere blurs it too.
    click(&dialog, QStringLiteral("fieldBlurSelect"));
    dialog.findChild<QDoubleSpinBox*>(QStringLiteral("fieldBlurSpin"))->setValue(20.0);
    QTRY_VERIFY(dialog.canvas()->image().pixel(10, 32) != unfiltered.pixel(10, 32));
    dialog.reject();
}

void BlurGalleryEffectsTest::menuCommitsTheCombinedGalleryAndReopensIt()
{
    pictura::PictureView* view = window_->activeView();
    const int before = view->history_count();
    QList<double> committed;
    QTimer::singleShot(0, [&committed] {
        if (auto* dialog = openWorkspace()) {
            click(dialog, QStringLiteral("tiltShiftSelect"));
            committed = dialog->values();
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(menuId(QStringLiteral("Iris Blur"))));
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: Blur Gallery"));
    QCOMPARE(filter_last_kind(*view), QStringLiteral("blur-gallery"));
    QCOMPARE(filter_last_params(*view), committed);

    // Last Filter Settings reopens both effects, Iris selected.
    bool reopened = false;
    QTimer::singleShot(0, [&reopened, &committed] {
        if (auto* dialog = openWorkspace()) {
            reopened = dialog->effectEnabled(Mode::Iris)
                       && dialog->effectEnabled(Mode::TiltShift)
                       && !dialog->effectEnabled(Mode::Field)
                       && dialog->selectedEffect() == Mode::Iris && dialog->values() == committed;
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QVERIFY(reopened);
    QCOMPARE(view->history_count(), before + 1);
}

void BlurGalleryEffectsTest::okWithEveryEffectOffCommitsNothing()
{
    pictura::PictureView* view = window_->activeView();
    const int before = view->history_count();
    QTimer::singleShot(0, [] {
        if (auto* dialog = openWorkspace()) {
            dialog->findChild<QCheckBox*>(QStringLiteral("fieldBlurEnabled"))->setChecked(false);
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(menuId(QStringLiteral("Field Blur"))));
    QCOMPARE(view->history_count(), before);
}

QTEST_MAIN(BlurGalleryEffectsTest)

#include "tst_blur_gallery_effects.moc"
