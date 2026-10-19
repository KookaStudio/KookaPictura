// Coalesced Opacity/Fill previews: timer, target cancellation, and regions.

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>
#include "api/layers_api.h"
#include "api/transform_api.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtWidgets/QSlider>

#include <memory>

#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/percent_field.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

namespace {

QSlider* fieldSlider(pictura::LayersPanel* panel, const QString& fieldName)
{
    if (!panel) {
        return nullptr;
    }
    auto* field = panel->findChild<pictura::PercentField*>(fieldName);
    return field ? field->findChild<QSlider*>() : nullptr;
}

QString backgroundPath(pictura::PictureView& view)
{
    for (int i = 0; i < pictura::layers_api::rowCount(view); ++i) {
        const auto row = pictura::layers_api::rowAt(view, i);
        if (row.kind == QStringLiteral("background")) {
            return row.path;
        }
    }
    return {};
}

QString ensureBackground(pictura::PictureView& view)
{
    QString path = backgroundPath(view);
    if (!path.isEmpty()) {
        return path;
    }
    QString bottom;
    for (int i = 0; i < pictura::layers_api::rowCount(view); ++i) {
        const auto row = pictura::layers_api::rowAt(view, i);
        if (!row.path.contains(QLatin1Char('/')) && row.kind != QStringLiteral("group")) {
            bottom = row.path;
        }
    }
    if (bottom.isEmpty() || !view.background_from_layer(bottom)) {
        return {};
    }
    return backgroundPath(view);
}

QString placeTestLayer(pictura::PictureView& view, int side)
{
    QTemporaryDir dir;
    if (!dir.isValid()) {
        return {};
    }
    const QString png = dir.filePath(QStringLiteral("placed.png"));
    QImage image(side, side, QImage::Format_ARGB32);
    image.fill(0xff00ff00u);
    if (!image.save(png)) {
        return {};
    }
    return view.place_image(png);
}

int previewSignals(const QSignalSpy& changed, const QSignalSpy& blitted)
{
    return changed.count() + blitted.count();
}

} // namespace

class LayersPreviewCoalescingTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void opacityDragCoalescesPreview();
    void fillDragCoalescesPreview();
    void midDragPausePreviewsWhileHeld();
    void fastOpacityRoundTripRecordsNothing();
    void fillFastRoundTripRecordsNothing();
    void pendingPreviewSurvivesPanelRefresh();
    void documentSwitchCancelsPendingPreview();
    void selectionChangeCancelsPendingPreview();
    void sameViewReplacementCancelsPendingPreview();
    void continuousTicksDoNotRestartTheTimer();
    void fastRoundTripAtNonFixedByteRecordsOneState();
    void noSelectionShowsDefaultPercent();
    void ineligibleTargetsShowDefaultPercent();
    void multiSelectionKeepsEligibleFieldEnabled();
    void opacityPreviewUsesTheRegionPath();
    void opacityCommitUsesTheRegionPath();
    void canvasSizedOpacityUsesTheRegionPath();
    void groupOpacityPreviewFallsBackToFullRecomposite();

private:
    bool setupNest();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
    QString group_;
    QString a_;
};

void LayersPreviewCoalescingTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayersPreviewCoalescingTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
    group_.clear();
    a_.clear();
}

bool LayersPreviewCoalescingTest::setupNest()
{
    const bool created = window_->newDocument(QStringLiteral("Fixes"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!created || !view_ || !panel_) {
        return false;
    }
    group_ = view_->add_group_in(QString());
    a_ = view_->add_layer_in(group_);
    view_->set_layer_name_path(a_, QStringLiteral("A"));
    return true;
}

void LayersPreviewCoalescingTest::opacityDragCoalescesPreview()
{
    QVERIFY2(setupNest(), "coalesce fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();

    slider->setSliderDown(true);
    slider->setValue(40);
    slider->setValue(60);
    slider->setValue(80);
    QCOMPARE(previewSignals(changed, blitted), 0);
    QCOMPARE(view_->history_count(), base);

    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 204);
    QCOMPARE(view_->history_count(), base);

    slider->setValue(90);
    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 230);
}

void LayersPreviewCoalescingTest::fillDragCoalescesPreview()
{
    QVERIFY2(setupNest(), "fill coalesce fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersFillField"));
    QVERIFY2(slider != nullptr, "fill slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();

    slider->setSliderDown(true);
    slider->setValue(40);
    slider->setValue(60);
    slider->setValue(80);
    QCOMPARE(previewSignals(changed, blitted), 0);

    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).fill, 204);

    slider->setValue(90);
    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).fill, 230);
}

void LayersPreviewCoalescingTest::midDragPausePreviewsWhileHeld()
{
    QVERIFY2(setupNest(), "pause fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();

    slider->setSliderDown(true);
    slider->setValue(70);
    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 179);
    QCOMPARE(view_->history_count(), base);

    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 179);
}

void LayersPreviewCoalescingTest::fastOpacityRoundTripRecordsNothing()
{
    QVERIFY2(setupNest(), "round trip fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();

    slider->setSliderDown(true);
    slider->setValue(40);
    slider->setValue(100);
    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QTest::qWait(250);
    QCOMPARE(previewSignals(changed, blitted), 0);
    QCOMPARE(view_->history_count(), base);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 255);
}

void LayersPreviewCoalescingTest::fillFastRoundTripRecordsNothing()
{
    QVERIFY2(setupNest(), "fill round trip fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersFillField"));
    QVERIFY2(slider != nullptr, "fill slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();

    slider->setSliderDown(true);
    slider->setValue(40);
    slider->setValue(100);
    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QTest::qWait(250);
    QCOMPARE(previewSignals(changed, blitted), 0);
    QCOMPARE(view_->history_count(), base);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).fill, 255);
}

void LayersPreviewCoalescingTest::pendingPreviewSurvivesPanelRefresh()
{
    QVERIFY2(setupNest(), "preserve fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();

    slider->setSliderDown(true);
    slider->setValue(70);
    panel_->refresh();
    QCoreApplication::processEvents();
    QCOMPARE(panel_->opacityPercentForTest(), 70);
    QCOMPARE(previewSignals(changed, blitted), 0);
    QCOMPARE(view_->history_count(), base);

    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 179);

    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QCOMPARE(view_->history_count(), base + 1);
}

void LayersPreviewCoalescingTest::documentSwitchCancelsPendingPreview()
{
    QVERIFY2(setupNest(), "document switch fixture");
    panel_->setView(view_);
    QVERIFY(view_->set_layers_opacity(QStringList{a_}, 102) == 1);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* field = panel_->findChild<pictura::NumericField*>(QStringLiteral("layersOpacityField"));
    QVERIFY(field != nullptr);
    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    QSignalSpy committed(field, &pictura::NumericField::valueCommitted);
    pictura::PictureView* first = view_;
    const int firstBase = first->history_count();
    slider->setSliderDown(true);
    slider->setValue(70);
    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(pictura::layers_api::rowForPath(*first, a_).opacity, 179);
    QVERIFY(first->is_dirty());

    QVERIFY(window_->newDocument(QStringLiteral("Second"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    QCoreApplication::processEvents();
    pictura::PictureView* second = window_->activeView();
    QVERIFY2(second != nullptr && second != first, "second document active");
    const int secondBase = second->history_count();

    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QTest::qWait(220);
    QCOMPARE(committed.count(), 0);
    QCOMPARE(first->history_count(), firstBase);
    QCOMPARE(second->history_count(), secondBase);
    QCOMPARE(second->layer_opacity(0), 255);
}

void LayersPreviewCoalescingTest::selectionChangeCancelsPendingPreview()
{
    QVERIFY2(setupNest(), "selection cancel fixture");
    panel_->setView(view_);
    QVERIFY(view_->set_layers_opacity(QStringList{a_}, 102) == 1);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* field = panel_->findChild<pictura::NumericField*>(QStringLiteral("layersOpacityField"));
    QVERIFY(field != nullptr);
    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    QSignalSpy committed(field, &pictura::NumericField::valueCommitted);
    const int base = view_->history_count();
    slider->setSliderDown(true);
    slider->setValue(70);
    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 179);
    QVERIFY(view_->is_dirty());

    panel_->selectPaths({group_}, group_);
    QCoreApplication::processEvents();
    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QTest::qWait(220);
    QCOMPARE(committed.count(), 0);
    QCOMPARE(view_->history_count(), base);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 179);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, group_).opacity, 255);
}

void LayersPreviewCoalescingTest::sameViewReplacementCancelsPendingPreview()
{
    QVERIFY(window_->newDocument(QStringLiteral("SameView"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ != nullptr && panel_ != nullptr);
    a_ = view_->add_layer_in(QString());
    view_->set_layer_name_path(a_, QStringLiteral("A"));
    QVERIFY(view_->set_layers_opacity(QStringList{a_}, 102) == 1);
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* field = panel_->findChild<pictura::NumericField*>(QStringLiteral("layersOpacityField"));
    QVERIFY(field != nullptr);
    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    QSignalSpy committed(field, &pictura::NumericField::valueCommitted);
    slider->setSliderDown(true);
    slider->setValue(70);
    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 179);

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("A.png"));
    QImage image(16, 16, QImage::Format_ARGB32);
    image.fill(0xff0000ffu);
    QVERIFY(image.save(png));
    QVERIFY(view_->open_image(png));
    panel_->refresh();
    QCoreApplication::processEvents();
    const int replacementBase = view_->history_count();

    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QTest::qWait(220);
    QCOMPARE(committed.count(), 0);
    QCOMPARE(view_->history_count(), replacementBase);
    QCOMPARE(view_->layer_opacity(0), 255);
}

void LayersPreviewCoalescingTest::continuousTicksDoNotRestartTheTimer()
{
    QVERIFY2(setupNest(), "continuous tick fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();
    slider->setSliderDown(true);
    for (int value = 35; value <= 95; value += 10) {
        slider->setValue(value);
        QTest::qWait(40);
    }
    QTRY_VERIFY_WITH_TIMEOUT(previewSignals(changed, blitted) >= 2, 500);
    QCOMPARE(view_->history_count(), base);

    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 242);
}

void LayersPreviewCoalescingTest::fastRoundTripAtNonFixedByteRecordsOneState()
{
    QVERIFY2(setupNest(), "non-fixed fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    QVERIFY(view_->set_layers_opacity(QStringList{a_}, 127) == 1);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    const int base = view_->history_count();
    slider->setSliderDown(true);
    slider->setValue(40);
    slider->setValue(50);
    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, a_).opacity, 128);
}

void LayersPreviewCoalescingTest::noSelectionShowsDefaultPercent()
{
    QVERIFY2(setupNest(), "no selection fixture");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    panel_->selectPaths({}, QString());
    QCoreApplication::processEvents();
    auto* opacity = panel_->findChild<pictura::PercentField*>(QStringLiteral("layersOpacityField"));
    auto* fill = panel_->findChild<pictura::PercentField*>(QStringLiteral("layersFillField"));
    QVERIFY(opacity != nullptr && fill != nullptr);
    QCOMPARE(opacity->value(), 100.0);
    QCOMPARE(fill->value(), 100.0);
    QVERIFY(!opacity->isEnabled());
    QVERIFY(!fill->isEnabled());
}

void LayersPreviewCoalescingTest::ineligibleTargetsShowDefaultPercent()
{
    QVERIFY2(setupNest(), "ineligible fixture");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    auto* opacity = panel_->findChild<pictura::PercentField*>(QStringLiteral("layersOpacityField"));
    auto* fill = panel_->findChild<pictura::PercentField*>(QStringLiteral("layersFillField"));
    QVERIFY(opacity != nullptr && fill != nullptr);

    const QString bg = ensureBackground(*view_);
    QVERIFY2(!bg.isEmpty(), "background row");
    panel_->selectPaths({bg}, bg);
    QCoreApplication::processEvents();
    QCOMPARE(opacity->value(), 100.0);
    QCOMPARE(fill->value(), 100.0);
    QVERIFY(!opacity->isEnabled());
    QVERIFY(!fill->isEnabled());

    QVERIFY(view_->set_layers_opacity(QStringList{a_}, 128) == 1);
    QVERIFY(view_->set_layers_lock(QStringList{a_}, QStringLiteral("all"), true));
    panel_->refresh();
    panel_->selectPaths({a_}, a_);
    QCoreApplication::processEvents();
    QCOMPARE(opacity->value(), 100.0);
    QCOMPARE(fill->value(), 100.0);
    QVERIFY(!opacity->isEnabled());
    QVERIFY(!fill->isEnabled());

    panel_->selectPaths({group_}, group_);
    QCoreApplication::processEvents();
    QCOMPARE(fill->value(), 100.0);
    QVERIFY(!fill->isEnabled());
    QVERIFY(opacity->isEnabled());

    panel_->selectPaths({bg, a_}, a_);
    QCoreApplication::processEvents();
    QCOMPARE(opacity->value(), 100.0);
    QCOMPARE(fill->value(), 100.0);
    QVERIFY(!opacity->isEnabled());
    QVERIFY(!fill->isEnabled());

    const QString smart = view_->add_solid_fill(0xff808080u);
    QVERIFY(!smart.isEmpty());
    QVERIFY(view_->rasterize_fill_content(smart));
    QVERIFY(view_->convert_to_smart_object(smart));
    QVERIFY(view_->apply_pictura_raw_filter(smart, QStringLiteral("Temp=20;Ex12=1.5")));
    panel_->refresh();
    const QString synthetic = smart + QStringLiteral("/@sf/0");
    panel_->selectPaths({synthetic}, synthetic);
    QCoreApplication::processEvents();
    QCOMPARE(opacity->value(), 100.0);
    QCOMPARE(fill->value(), 100.0);
    QVERIFY(!opacity->isEnabled());
    QVERIFY(!fill->isEnabled());
}

void LayersPreviewCoalescingTest::multiSelectionKeepsEligibleFieldEnabled()
{
    QVERIFY2(setupNest(), "multi selection fixture");
    panel_->setView(view_);
    QVERIFY(view_->set_layers_opacity(QStringList{a_}, 102) == 1);
    QVERIFY(view_->set_layers_fill(QStringList{a_}, 102) == 1);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    const QString bg = ensureBackground(*view_);
    QVERIFY2(!bg.isEmpty(), "background row");
    panel_->selectPaths({bg, a_}, a_);
    QCoreApplication::processEvents();
    auto* opacity = panel_->findChild<pictura::PercentField*>(QStringLiteral("layersOpacityField"));
    auto* fill = panel_->findChild<pictura::PercentField*>(QStringLiteral("layersFillField"));
    QVERIFY(opacity != nullptr && fill != nullptr);
    QVERIFY(opacity->isEnabled());
    QCOMPARE(opacity->value(), 40.0);

    panel_->selectPaths({group_, a_}, a_);
    QCoreApplication::processEvents();
    QVERIFY(opacity->isEnabled());
    QVERIFY(fill->isEnabled());
    QCOMPARE(opacity->value(), 40.0);
    QCOMPARE(fill->value(), 40.0);
}

void LayersPreviewCoalescingTest::opacityPreviewUsesTheRegionPath()
{
    QVERIFY(window_->newDocument(QStringLiteral("RegionPreview"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ != nullptr && panel_ != nullptr);
    const QString placed = placeTestLayer(*view_, 8);
    QVERIFY2(!placed.isEmpty(), "placed layer");
    panel_->setView(view_);
    panel_->selectPaths({placed}, placed);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    const auto rect = pictura::transform_api::layerRect(*view_, placed);
    QVERIFY2(rect.valid, "placed rect");
    QCOMPARE(rect.width(), 8);
    QCOMPARE(rect.height(), 8);
    auto* slider = fieldSlider(panel_, QStringLiteral("layersOpacityField"));
    QVERIFY2(slider != nullptr, "opacity slider");
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    const int base = view_->history_count();
    const QImage before = view_->image();
    QPoint outside(rect.right, rect.bottom);
    if (outside.x() >= before.width() || outside.y() >= before.height()) {
        outside = QPoint(qMax(0, rect.left - 1), qMax(0, rect.top - 1));
    }
    const QRgb outsideBefore = before.pixel(outside);

    slider->setSliderDown(true);
    slider->setValue(70);
    QTRY_COMPARE_WITH_TIMEOUT(previewSignals(changed, blitted), 1, 1000);
    QCOMPARE(blitted.count(), 1);
    QCOMPARE(changed.count(), 0);
    const QList<QVariant> args = blitted.takeFirst();
    const QImage region = args.at(0).value<QImage>();
    QCOMPARE(region.width(), 8);
    QCOMPARE(region.height(), 8);
    QCOMPARE(args.at(1).toInt(), rect.left);
    QCOMPARE(args.at(2).toInt(), rect.top);
    QCOMPARE(view_->image().pixel(outside), outsideBefore);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, placed).opacity, 179);
    QCOMPARE(view_->history_count(), base);

    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QCOMPARE(view_->history_count(), base + 1);
}

void LayersPreviewCoalescingTest::opacityCommitUsesTheRegionPath()
{
    QVERIFY(window_->newDocument(QStringLiteral("RegionCommit"), 64, 64, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_ != nullptr);
    const QString placed = placeTestLayer(*view_, 8);
    QVERIFY2(!placed.isEmpty(), "placed layer");
    const auto rect = pictura::transform_api::layerRect(*view_, placed);
    QVERIFY(rect.valid);
    const int base = view_->history_count();
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    (void)view_->image();
    (void)view_->take_canvas_damage();

    QCOMPARE(view_->commit_layers_opacity(QStringList{placed}, 128), 1);
    QVERIFY(!view_->take_canvas_damage().isEmpty());
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(blitted.count(), 1);
    const QList<QVariant> args = blitted.takeFirst();
    const QImage region = args.at(0).value<QImage>();
    QCOMPARE(region.width(), 8);
    QCOMPARE(region.height(), 8);
    QCOMPARE(args.at(1).toInt(), rect.left);
    QCOMPARE(args.at(2).toInt(), rect.top);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, placed).opacity, 128);
}

void LayersPreviewCoalescingTest::canvasSizedOpacityUsesTheRegionPath()
{
    QVERIFY(window_->newDocument(QStringLiteral("CanvasRegion"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_ != nullptr);
    const QString placed = placeTestLayer(*view_, 16);
    QVERIFY2(!placed.isEmpty(), "placed canvas layer");
    const auto rect = pictura::transform_api::layerRect(*view_, placed);
    QVERIFY(rect.valid);
    QCOMPARE(rect.width(), 16);
    QCOMPARE(rect.height(), 16);
    const int base = view_->history_count();
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);
    (void)view_->image();
    (void)view_->take_canvas_damage();

    QCOMPARE(view_->commit_layers_opacity(QStringList{placed}, 128), 1);
    QVERIFY(!view_->take_canvas_damage().isEmpty());
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(blitted.count(), 1);
    const QImage region = blitted.takeFirst().at(0).value<QImage>();
    QCOMPARE(region.width(), 16);
    QCOMPARE(region.height(), 16);
}

void LayersPreviewCoalescingTest::groupOpacityPreviewFallsBackToFullRecomposite()
{
    QVERIFY2(setupNest(), "group fallback fixture");
    panel_->setView(view_);
    panel_->refresh();
    QSignalSpy changed(view_, &pictura::PictureView::changed);
    QSignalSpy blitted(view_, &pictura::PictureView::regionBlitted);

    QCOMPARE(view_->preview_layers_opacity(QStringList{group_}, 128), 1);
    QCOMPARE(changed.count(), 1);
    QCOMPARE(blitted.count(), 0);
    QCOMPARE(pictura::layers_api::rowForPath(*view_, group_).opacity, 128);
}


QTEST_MAIN(LayersPreviewCoalescingTest)
#include "tst_layers_preview_coalescing.moc"
