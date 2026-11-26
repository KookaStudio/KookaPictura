// The Performance page of Preferences (#636): History States, Cache Levels and
// the presets, Cache Tile Size, Scratch Disks, the graphics processor, the
// Memory Usage slider and ceiling, and the Scratch Sizes and Efficiency status
// bar readouts.

#include <QtTest/QtTest>

#include <QtCore/QRegularExpression>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "frame.h"
#include "preferences.h"
#include "preferences_dialog.h"
#include "status_readouts.h"

#include "api/document_bridge.h"
#include "api/history_api.h"
#include "api/paint_bridge.h"

#include "qt_test_support.h"

class PerformancePreferencesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void historyStatesDefaultsToTwenty();
    void loweringHistoryStatesTrimsOpenDocuments();
    void historyStatesPersists();
    void cacheLevelsRebuildOpenViews();
    void cacheLevelsPersists();
    void presetsSetCacheLevels();
    void cacheTileSizeIsFixed();
    void scratchVolumesAddReorderRemove();
    void scratchVolumesStopAtFour();
    void emptyScratchListShowsTheDefault();
    void gpuLineNamesTheAdapter();
    void memorySliderAndCeilingFollow();
    void scratchSizesFormatLikeCs6();
    void scratchSizesMenuShowsAPair();
    void efficiencyCoversTheLatestInterval();
    void efficiencyMenuStartsAtHundred();

private:
    pictura::PictureView* newDocument(int side = 40);
    pictura::PreferencesDialog* showPerformance();
    template <typename T> T* find(const char* name)
    {
        return window_->findChild<T*>(QString::fromLatin1(name));
    }
    // Choose the status bar readout whose action carries `key`.
    bool chooseReadout(const QString& key);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PerformancePreferencesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void PerformancePreferencesTest::init()
{
    while (window_->documentCount() > 0) {
        QVERIFY(window_->closeDocument(0, false));
    }
}

void PerformancePreferencesTest::cleanup()
{
    pictura::set_history_states(20);
    pictura::set_cache_levels(8);
    pictura::set_scratch_volumes(QString());
    pictura::set_memory_usage_percent(60);
    if (pictura::PreferencesDialog* dialog = window_->preferencesDialog()) {
        dialog->setHistoryStates(20);
        dialog->setCacheLevels(8);
        dialog->setScratchVolumes({});
        dialog->setMemoryUsagePercent(60);
        dialog->hide();
    }
}

pictura::PictureView* PerformancePreferencesTest::newDocument(int side)
{
    if (!window_->newDocument(QStringLiteral("Performance"), side, side, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

pictura::PreferencesDialog* PerformancePreferencesTest::showPerformance()
{
    window_->showPreferences(pictura::PreferencesDialog::kPerformance);
    return window_->preferencesDialog();
}

bool PerformancePreferencesTest::chooseReadout(const QString& key)
{
    auto* button = find<QToolButton>("statusOptionsButton");
    if (!button || !button->menu()) {
        return false;
    }
    for (QAction* action : button->menu()->actions()) {
        if (action->data().toString() == key) {
            action->trigger();
            return true;
        }
    }
    return false;
}

void PerformancePreferencesTest::historyStatesDefaultsToTwenty()
{
    QCOMPARE(pictura::history_states(), 20);
    QVERIFY(showPerformance());
    auto* spin = find<QSpinBox>("preferencesHistoryStates");
    QVERIFY(spin);
    QCOMPARE(spin->value(), 20);
    QCOMPARE(spin->minimum(), 1);
    QCOMPARE(spin->maximum(), 1000);
}

void PerformancePreferencesTest::loweringHistoryStatesTrimsOpenDocuments()
{
    pictura::PictureView* view = newDocument();
    QVERIFY(view);
    for (int i = 0; i < 10; ++i) {
        QVERIFY(view->add_layer(-1) >= 0);
    }
    QVERIFY(pictura::history_api::addSnapshot(*view, QStringLiteral("Kept")));
    QCOMPARE(pictura::history_api::count(*view), 11);
    int changed = 0;
    pictura::history_api::connectChanged(*view, this, [&changed]() { ++changed; });

    QVERIFY(showPerformance());
    auto* spin = find<QSpinBox>("preferencesHistoryStates");
    QVERIFY(spin);
    spin->setValue(4);
    QCOMPARE(pictura::history_states(), 4);
    QCOMPARE(pictura::history_api::count(*view), 5);
    QCOMPARE(pictura::history_api::snapshotCount(*view), 1);
    QVERIFY(changed > 0);
}

void PerformancePreferencesTest::historyStatesPersists()
{
    QVERIFY(showPerformance());
    auto* spin = find<QSpinBox>("preferencesHistoryStates");
    QVERIFY(spin);
    spin->setValue(50);
    QCOMPARE(pictura::history_states(), 50);
    QCOMPARE(pictura::loadPreferences().historyStates, 50);
}

void PerformancePreferencesTest::cacheLevelsRebuildOpenViews()
{
    pictura::PictureView* view = newDocument(4096);
    QVERIFY(view);
    QCOMPARE(view->display_level_extents().size(), 10);
    QVERIFY(showPerformance());
    auto* spin = find<QSpinBox>("preferencesCacheLevels");
    QVERIFY(spin);
    QCOMPARE(spin->value(), 8);
    spin->setValue(2);
    QCOMPARE(pictura::cache_levels(), 2);
    QCOMPARE(view->display_level_extents().size(), 4);
    spin->setValue(8);
    QCOMPARE(view->display_level_extents().size(), 10);
}

void PerformancePreferencesTest::cacheLevelsPersists()
{
    QVERIFY(showPerformance());
    auto* spin = find<QSpinBox>("preferencesCacheLevels");
    QVERIFY(spin);
    QCOMPARE(spin->minimum(), 2);
    QCOMPARE(spin->maximum(), 8);
    spin->setValue(6);
    QCOMPARE(pictura::loadPreferences().cacheLevels, 6);
}

void PerformancePreferencesTest::presetsSetCacheLevels()
{
    QVERIFY(showPerformance());
    auto* spin = find<QSpinBox>("preferencesCacheLevels");
    QVERIFY(spin);
    const std::pair<const char*, int> presets[] = {
        {"preferencesPresetTallThin", 4},
        {"preferencesPresetDefault", 8},
        {"preferencesPresetBigFlat", 8},
    };
    for (const auto& [name, levels] : presets) {
        auto* button = find<QPushButton>(name);
        QVERIFY2(button, name);
        button->click();
        QCOMPARE(spin->value(), levels);
        QCOMPARE(pictura::cache_levels(), levels);
    }
}

void PerformancePreferencesTest::cacheTileSizeIsFixed()
{
    QVERIFY(showPerformance());
    auto* tile = find<QComboBox>("preferencesCacheTileSize");
    QVERIFY(tile);
    QVERIFY(!tile->isEnabled());
    bool titled = false;
    for (QGroupBox* group : window_->preferencesDialog()->findChildren<QGroupBox*>()) {
        // `&&` shows one ampersand; a single `&` would turn into an underline.
        titled = titled || group->title() == QStringLiteral("History && Cache");
    }
    QVERIFY(titled);
    QVERIFY(tile->currentText().contains(QStringLiteral("256")));
}

void PerformancePreferencesTest::scratchVolumesAddReorderRemove()
{
    QTemporaryDir first;
    QTemporaryDir second;
    QVERIFY(first.isValid() && second.isValid());
    pictura::PreferencesDialog* dialog = showPerformance();
    QVERIFY(dialog);
    auto* list = find<QListWidget>("preferencesScratchVolumes");
    QVERIFY(list);

    dialog->addScratchVolume(first.path());
    const QString fallback = list->item(0)->data(Qt::UserRole).toString();
    QVERIFY(fallback.endsWith(QStringLiteral("kooka-pictura/scratch")));
    QCOMPARE(pictura::scratch_volumes(), fallback + QLatin1Char('\n') + first.path());
    list->setCurrentRow(0);
    find<QPushButton>("preferencesScratchRemove")->click();
    dialog->addScratchVolume(second.path());
    QCOMPARE(pictura::scratch_volumes(), first.path() + QLatin1Char('\n') + second.path());
    QCOMPARE(list->count(), 2);

    list->setCurrentRow(1);
    find<QPushButton>("preferencesScratchUp")->click();
    QCOMPARE(pictura::scratch_volumes(), second.path() + QLatin1Char('\n') + first.path());
    QCOMPARE(pictura::loadPreferences().scratchVolumes, QStringList({second.path(), first.path()}));

    list->setCurrentRow(0);
    find<QPushButton>("preferencesScratchRemove")->click();
    QCOMPARE(pictura::scratch_volumes(), first.path());
    QCOMPARE(pictura::loadPreferences().scratchVolumes, QStringList({first.path()}));
    QVERIFY(list->item(0)->text().contains(QStringLiteral("free")));
}

void PerformancePreferencesTest::scratchVolumesStopAtFour()
{
    QTemporaryDir dirs[5];
    pictura::PreferencesDialog* dialog = showPerformance();
    QVERIFY(dialog);
    auto* add = find<QPushButton>("preferencesScratchAdd");
    QVERIFY(add);
    // The first Add keeps the default volume, so three more reach four.
    for (int i = 0; i < 3; ++i) {
        QVERIFY(dirs[i].isValid());
        QVERIFY(add->isEnabled());
        dialog->addScratchVolume(dirs[i].path());
    }
    QVERIFY(!add->isEnabled());
    dialog->addScratchVolume(dirs[4].path());
    QCOMPARE(find<QListWidget>("preferencesScratchVolumes")->count(), 4);
}

void PerformancePreferencesTest::emptyScratchListShowsTheDefault()
{
    pictura::PreferencesDialog* dialog = showPerformance();
    QVERIFY(dialog);
    dialog->setScratchVolumes({});
    auto* list = find<QListWidget>("preferencesScratchVolumes");
    QVERIFY(list);
    QCOMPARE(list->count(), 1);
    QCOMPARE(list->item(0)->flags(), Qt::NoItemFlags);
    QVERIFY(list->item(0)->text().contains(QStringLiteral("kooka-pictura/scratch")));
    QVERIFY(!find<QPushButton>("preferencesScratchRemove")->isEnabled());
    QVERIFY(find<QPushButton>("preferencesScratchAdd")->isEnabled());
}

void PerformancePreferencesTest::gpuLineNamesTheAdapter()
{
    pictura::PreferencesDialog* dialog = showPerformance();
    QVERIFY(dialog);
    auto* line = find<QLabel>("preferencesGpuAdapter");
    QVERIFY(line);
    dialog->setGpuAdapter(QStringLiteral("Test Adapter 9000"));
    QCOMPARE(line->text(), QStringLiteral("Detected Graphics Processor: Test Adapter 9000"));
    dialog->setGpuAdapter(QString());
    QCOMPARE(line->text(), QStringLiteral("No graphics processor found"));
}

void PerformancePreferencesTest::memorySliderAndCeilingFollow()
{
    QVERIFY(showPerformance());
    auto* slider = find<QSlider>("preferencesMemorySlider");
    auto* spin = find<QSpinBox>("preferencesMemoryUsage");
    auto* ceiling = find<QLabel>("preferencesMemoryCeiling");
    QVERIFY(slider && spin && ceiling);
    slider->setValue(30);
    QCOMPARE(spin->value(), 30);
    QCOMPARE(pictura::memory_usage_percent(), 30);
    const qint64 bytes = pictura::readMemory().ceiling;
    if (bytes < 0) {
        QVERIFY(ceiling->text().endsWith(QStringLiteral("—")));
    } else {
        QVERIFY2(ceiling->text().contains(QLocale().toString(bytes >> 20)),
                 qPrintable(ceiling->text()));
    }
    spin->setValue(40);
    QCOMPARE(slider->value(), 40);
}

void PerformancePreferencesTest::scratchSizesFormatLikeCs6()
{
    QCOMPARE(pictura::formatScratchSizes(327'680'000, 2'190'433'320LL),
             QStringLiteral("312.5M/2.04G"));
    QCOMPARE(pictura::formatScratchSizes(327'680'000, -1), QStringLiteral("312.5M/—"));
}

void PerformancePreferencesTest::scratchSizesMenuShowsAPair()
{
    QVERIFY(newDocument());
    QVERIFY(chooseReadout(QStringLiteral("scratchSizes")));
    auto* label = find<QLabel>("statusSizeLabel");
    QVERIFY(label);
    static const QRegularExpression pair(
        QStringLiteral("^([0-9.]+[MG]|—)/([0-9.]+[MG]|—)$"));
    QVERIFY2(pair.match(label->text()).hasMatch(), qPrintable(label->text()));
    QVERIFY(chooseReadout(QStringLiteral("sizes")));
    QVERIFY(label->text().contains(QStringLiteral("px")));
}

void PerformancePreferencesTest::efficiencyCoversTheLatestInterval()
{
    pictura::EfficiencyMeter meter;
    QCOMPARE(meter.percent(), 100);
    QCOMPARE(meter.update(0, 0), 100);
    QCOMPARE(meter.update(900, 100), 90);
    QCOMPARE(meter.update(900, 100), 90);
    QCOMPARE(meter.update(1000, 200), 50);
}

void PerformancePreferencesTest::efficiencyMenuStartsAtHundred()
{
    QVERIFY(newDocument());
    QVERIFY(chooseReadout(QStringLiteral("efficiency")));
    auto* label = find<QLabel>("statusSizeLabel");
    QVERIFY(label);
    QCOMPARE(label->text(), QStringLiteral("100%"));
    QVERIFY(chooseReadout(QStringLiteral("sizes")));
}

PICTURA_QT_TEST_MAIN(PerformancePreferencesTest)
#include "tst_performance_preferences.moc"
