#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtGui/QWheelEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QSlider>
#include <QtWidgets/QToolButton>

#include "commands.h"
#include "filter_gallery_dialog.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

QString galleryId()
{
    return pictura::commandIdForPath({QStringLiteral("Filter"), QStringLiteral("Filter Gallery…")});
}

QStringList kinds(const QList<pictura::FilterGalleryDialog::Effect>& effects)
{
    QStringList out;
    for (const auto& effect : effects) {
        out.append(effect.kind);
    }
    return out;
}

} // namespace

class FilterGalleryTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void categoriesFollowCs6();
    void menuEntryIsEnabled();
    void stackPreviewsInThePaneOnly();
    void largePicturePreviewsReduced();
    void okCommitsOneHistoryState();
    void effectLayersAddDeleteAndHide();
    void thumbnailsRender();
    void clickingTheEyeHidesTheEffect();
    void spaceTogglesTheEye();
    void keyboardZoomAndDragPan();
    void zoomDropdownAndWheel();
    void sliderClickFocusesValueBox();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void FilterGalleryTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void FilterGalleryTest::init()
{
    QVERIFY(window_->newDocument(QStringLiteral("Gallery"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
}

void FilterGalleryTest::categoriesFollowCs6()
{
    const auto categories = pictura::FilterGalleryDialog::categories();
    QStringList names;
    for (const auto& category : categories) {
        names.append(category.first);
    }
    QCOMPARE(names, (QStringList{QStringLiteral("Artistic"), QStringLiteral("Brush Strokes"),
                                 QStringLiteral("Distort"), QStringLiteral("Sketch"),
                                 QStringLiteral("Stylize"), QStringLiteral("Texture")}));
    QCOMPARE(categories.at(0).second.size(), 15);
    QCOMPARE(categories.at(1).second.size(), 8);
    QCOMPARE(categories.at(2).second.size(), 3);
    QCOMPARE(categories.at(2).second.first()->kind, QStringLiteral("diffuse-glow"));
    QCOMPARE(categories.at(2).second.at(1)->kind, QStringLiteral("glass"));
    QCOMPARE(categories.at(3).second.size(), 14);
    QCOMPARE(categories.at(4).second.size(), 1);
    QCOMPARE(categories.at(4).second.first()->kind, QStringLiteral("glowing-edges"));
    QCOMPARE(categories.at(5).second.size(), 6);
}

void FilterGalleryTest::menuEntryIsEnabled()
{
    bool found = false;
    for (const pictura::CommandInfo& info : window_->registry()->describe()) {
        if (info.id == galleryId()) {
            found = true;
            QVERIFY(info.implemented);
            QVERIFY(info.enabled);
        }
    }
    QVERIFY(found);
}

// The stack previews in the dialog's pane; the canvas and document stay as
// they were until OK.
void FilterGalleryTest::stackPreviewsInThePaneOnly()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    const int states = view->history_count();
    pictura::FilterGalleryDialog dialog(view);
    while (dialog.effects().size() > 1) {
        dialog.deleteEffect();
    }
    dialog.selectFilter(QStringLiteral("grain"));
    dialog.setEffectVisible(0, false);
    dialog.show();
    QTRY_VERIFY_WITH_TIMEOUT(!dialog.previewImage().isNull(), 3000);
    const QImage plain = dialog.previewImage();
    dialog.setEffectVisible(0, true);
    QTRY_VERIFY_WITH_TIMEOUT(dialog.previewImage() != plain, 3000);
    QCOMPARE(view->image(), before);
    dialog.reject();
    QCOMPARE(view->image(), before);
    QCOMPARE(view->history_count(), states);
}

// A picture far larger than the pane is previewed on a reduced copy, about
// one pixel per pane pixel, never at document resolution.
void FilterGalleryTest::largePicturePreviewsReduced()
{
    QVERIFY(window_->newDocument(QStringLiteral("Large"), 4000, 3000, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    pictura::FilterGalleryDialog dialog(view);
    dialog.selectFilter(QStringLiteral("grain"));
    dialog.resize(900, 600);
    dialog.show();
    QTRY_VERIFY_WITH_TIMEOUT(!dialog.previewImage().isNull(), 3000);
    const QImage preview = dialog.previewImage();
    QVERIFY2(preview.width() < 1000 * dialog.devicePixelRatioF(), qPrintable(QString::number(preview.width())));
    QCOMPARE(preview.width() * 3 / 4, preview.height());
    dialog.reject();
}

void FilterGalleryTest::okCommitsOneHistoryState()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    const int states = view->history_count();
    pictura::FilterGalleryDialog dialog(view);
    dialog.selectFilter(QStringLiteral("grain"));
    dialog.addEffect();
    dialog.selectFilter(QStringLiteral("texturizer"));
    QCOMPARE(kinds(dialog.effects()), (QStringList{QStringLiteral("grain"),
                                                   QStringLiteral("texturizer")}));
    dialog.show();
    QVERIFY(dialog.commit() == pictura::FilterGalleryDialog::CommitResult::Applied);
    QVERIFY(view->image() != before);
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()),
             QStringLiteral("Filter Gallery: Grain, Texturizer"));
    // CS6 reopens the gallery on the stack it last applied.
    pictura::FilterGalleryDialog again(view);
    QCOMPARE(kinds(again.effects()), kinds(dialog.effects()));
}

void FilterGalleryTest::effectLayersAddDeleteAndHide()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    pictura::FilterGalleryDialog dialog(view);
    // The gallery reopens on the session's last stack; start from one effect.
    while (dialog.effects().size() > 1) {
        dialog.deleteEffect();
    }
    dialog.selectFilter(QStringLiteral("cutout"));
    dialog.addEffect();
    QCOMPARE(dialog.effects().size(), 2);
    QCOMPARE(dialog.selectedEffect(), 1);
    // The new layer copies the selected one, settings included.
    QCOMPARE(dialog.effects().at(1).values, dialog.effects().at(0).values);
    dialog.deleteEffect();
    QCOMPARE(dialog.effects().size(), 1);
    // The last effect cannot be deleted.
    dialog.deleteEffect();
    QCOMPARE(dialog.effects().size(), 1);
    // With every effect hidden, OK changes nothing and is not a refusal.
    const int states = view->history_count();
    dialog.setEffectVisible(0, false);
    QVERIFY(!dialog.effects().at(0).visible);
    QVERIFY(dialog.commit() == pictura::FilterGalleryDialog::CommitResult::NothingVisible);
    QCOMPARE(view->image(), before);
    QCOMPARE(view->history_count(), states);
}

void FilterGalleryTest::thumbnailsRender()
{
    pictura::PictureView* view = window_->activeView();
    // A textured picture, so a filter has something to change.
    QVERIFY(apply_filter_params(*view, QStringLiteral("clouds"), {}));
    const QImage sample = pictura::FilterGalleryDialog::thumbnailSample(view->image());
    pictura::FilterGalleryDialog dialog(view);
    for (const QString& kind : {QStringLiteral("cutout"), QStringLiteral("glowing-edges")}) {
        auto* thumb = dialog.findChild<QToolButton*>(QStringLiteral("galleryThumb:") + kind);
        QVERIFY(thumb != nullptr);
        QTRY_VERIFY_WITH_TIMEOUT(!thumb->icon().isNull(), 3000);
        const QImage shown =
            thumb->icon().pixmap(sample.size()).toImage().convertToFormat(QImage::Format_RGBA8888);
        QCOMPARE(shown.size(), sample.size());
        // The unfiltered crop is the fallback; the thumbnail must be filtered.
        QVERIFY2(shown != sample, qPrintable(kind));
    }
}

void FilterGalleryTest::clickingTheEyeHidesTheEffect()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    pictura::FilterGalleryDialog dialog(view);
    while (dialog.effects().size() > 1) {
        dialog.deleteEffect();
    }
    dialog.selectFilter(QStringLiteral("grain"));
    dialog.setEffectVisible(0, true);
    dialog.show();
    QTRY_VERIFY_WITH_TIMEOUT(!dialog.previewImage().isNull(), 3000);
    const QImage filtered = dialog.previewImage();
    auto* list = dialog.findChild<QListWidget*>(QStringLiteral("galleryEffects"));
    QVERIFY(list != nullptr);
    const QRect row = list->visualItemRect(list->item(0));
    // The eye sits at the row's left edge.
    QTest::mouseClick(list->viewport(), Qt::LeftButton, {}, QPoint(row.left() + 10, row.center().y()));
    QVERIFY(!dialog.effects().at(0).visible);
    // Hidden, the effect leaves the preview.
    QTRY_VERIFY_WITH_TIMEOUT(dialog.previewImage() != filtered, 3000);
    QTest::mouseClick(list->viewport(), Qt::LeftButton, {}, QPoint(row.left() + 10, row.center().y()));
    QVERIFY(dialog.effects().at(0).visible);
    QTRY_COMPARE_WITH_TIMEOUT(dialog.previewImage(), filtered, 3000);
    dialog.reject();
    QCOMPARE(view->image(), before);
}

void FilterGalleryTest::spaceTogglesTheEye()
{
    pictura::FilterGalleryDialog dialog(window_->activeView());
    while (dialog.effects().size() > 1) {
        dialog.deleteEffect();
    }
    dialog.setEffectVisible(0, true);
    dialog.show();
    auto* list = dialog.findChild<QListWidget*>(QStringLiteral("galleryEffects"));
    QVERIFY(list != nullptr);
    // Visibility is the row's check state, readable by assistive technology.
    QVERIFY(list->item(0)->flags() & Qt::ItemIsUserCheckable);
    QCOMPARE(list->item(0)->checkState(), Qt::Checked);
    list->setFocus();
    list->setCurrentRow(0);
    QTest::keyClick(list, Qt::Key_Space);
    QVERIFY(!dialog.effects().at(0).visible);
    QCOMPARE(list->item(0)->checkState(), Qt::Unchecked);
    QTest::keyClick(list, Qt::Key_Space);
    QVERIFY(dialog.effects().at(0).visible);
    dialog.reject();
}

// The preview zooms on Ctrl++ / Ctrl+- and pans on a left drag (#243), with
// the dialog run from the menu so the frame's input blocker is live.
void FilterGalleryTest::keyboardZoomAndDragPan()
{
    QVERIFY(window_->newDocument(QStringLiteral("Pan"), 400, 400, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    double fit = 0.0;
    double in = 0.0;
    double out = 0.0;
    QString max;
    QPoint panned(-1, -1);
    QTimer::singleShot(0, [&] {
        pictura::FilterGalleryDialog* dialog = nullptr;
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* found = qobject_cast<pictura::FilterGalleryDialog*>(widget)) {
                dialog = found;
            }
        }
        if (!dialog) {
            return;
        }
        dialog->activateWindow();
        auto* zoom = dialog->findChild<QComboBox*>(QStringLiteral("galleryZoom"));
        auto* area = dialog->findChild<QScrollArea*>(QStringLiteral("galleryPreview"));
        if (!QTest::qWaitForWindowActive(dialog) || !zoom || !area) {
            dialog->reject();
            return;
        }
        const auto percent = [zoom] { return zoom->currentText().chopped(1).toDouble(); };
        fit = percent();
        QTest::keyClick(dialog, Qt::Key_Equal, Qt::ControlModifier);
        in = percent();
        QTest::keyClick(dialog, Qt::Key_Minus, Qt::ControlModifier);
        out = percent();
        // Enough steps to walk the whole ladder; the top preset must be
        // reachable from the +/- buttons and the wheel.
        for (int i = 0; i < 20; ++i) {
            QTest::keyClick(dialog, Qt::Key_Plus, Qt::ControlModifier | Qt::ShiftModifier);
        }
        max = zoom->currentText();
        QCoreApplication::processEvents();

        // 400 px at high zoom overflows the pane: dragging left/up scrolls
        // right/down.
        QScrollBar* h = area->horizontalScrollBar();
        QScrollBar* v = area->verticalScrollBar();
        h->setValue(0);
        v->setValue(0);
        QWidget* viewport = area->viewport();
        const QPoint start(viewport->width() / 2, viewport->height() / 2);
        QTest::mousePress(viewport, Qt::LeftButton, {}, start);
        QTest::mouseMove(viewport, start - QPoint(30, 20));
        QTest::mouseRelease(viewport, Qt::LeftButton, {}, start - QPoint(30, 20));
        panned = QPoint(h->value(), v->value());
        dialog->reject();
    });
    QVERIFY(window_->registry()->dispatch(galleryId()));
    QVERIFY(in > fit);
    QVERIFY(out < in);
    QCOMPARE(max, QStringLiteral("1600%"));
    QCOMPARE(panned, QPoint(30, 20));
}

// The zoom level is a drop down of CS6's preset levels and named views, and the
// mouse wheel zooms the preview.
void FilterGalleryTest::zoomDropdownAndWheel()
{
    pictura::PictureView* view = window_->activeView();
    pictura::FilterGalleryDialog dialog(view);
    dialog.resize(900, 600);
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));

    auto* zoom = dialog.findChild<QComboBox*>(QStringLiteral("galleryZoom"));
    QVERIFY(zoom != nullptr);
    QStringList options;
    for (int i = 0; i < zoom->count(); ++i) {
        options.append(zoom->itemText(i));
    }
    QVERIFY2(options.contains(QStringLiteral("1600%")), qPrintable(options.join(QLatin1Char(','))));
    QVERIFY(options.contains(QStringLiteral("Actual Pixels")));
    QVERIFY(options.contains(QStringLiteral("Fit in View")));
    QVERIFY(options.contains(QStringLiteral("Fit on Screen")));

    const double before = zoom->currentText().chopped(1).toDouble();
    auto* area = dialog.findChild<QScrollArea*>(QStringLiteral("galleryPreview"));
    QVERIFY(area != nullptr);
    QWidget* viewport = area->viewport();
    const QPointF centre(viewport->rect().center());
    QWheelEvent wheel(centre, viewport->mapToGlobal(centre.toPoint()), QPoint(), QPoint(0, 120),
                      Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(viewport, &wheel);
    const double after = zoom->currentText().chopped(1).toDouble();
    QVERIFY2(after > before, qPrintable(QStringLiteral("%1 -> %2").arg(before).arg(after)));

    // Choosing a preset applies it.
    const int index = zoom->findText(QStringLiteral("200%"));
    QVERIFY(index >= 0);
    QMetaObject::invokeMethod(zoom, "activated", Qt::DirectConnection, Q_ARG(int, index));
    QCOMPARE(zoom->currentText(), QStringLiteral("200%"));

    dialog.reject();
}

// Clicking a parameter's slider activates its value box, so Up/Down adjust it.
void FilterGalleryTest::sliderClickFocusesValueBox()
{
    pictura::PictureView* view = window_->activeView();
    pictura::FilterGalleryDialog dialog(view);
    dialog.selectFilter(QStringLiteral("grain"));
    dialog.show();
    dialog.activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(&dialog));

    auto* slider = dialog.findChild<QSlider*>();
    auto* spin = dialog.findChild<QDoubleSpinBox*>();
    QVERIFY(slider != nullptr);
    QVERIFY(spin != nullptr);
    slider->setSliderDown(true);
    QVERIFY2(spin->hasFocus(), "clicking the slider focuses the value box");
    slider->setSliderDown(false);

    dialog.reject();
}

QTEST_MAIN(FilterGalleryTest)
#include "tst_filter_gallery.moc"
