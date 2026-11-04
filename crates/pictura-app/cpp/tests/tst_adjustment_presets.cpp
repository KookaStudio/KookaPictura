// Adjustment presets (#576): the Properties panel's Preset menu, Save / Load
// Preset, the established built-in lists, and the per-user store.

#include <QtTest/QtTest>

#include "adjustment_dialog.h"
#include "adjustment_presets.h"
#include "api/layers_api.h"
#include "curves_dialog.h"
#include "frame.h"
#include "levels_dialog.h"
#include "panels/layers_panel.h"
#include "panels/properties_panel.h"
#include "photo_filter_dialog.h"
#include "api/layers_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>

#include <memory>

#include "qt_test_support.h"

namespace {

QByteArray toBytes(const ::rust::Vec<std::uint8_t>& v)
{
    return QByteArray(reinterpret_cast<const char*>(v.data()), qsizetype(v.size()));
}

} // namespace

class AdjustmentPresetsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void cleanupTestCase();
    void propertiesMenuListsBuiltinsAndSaved();
    void unsupportedKindHasNoMenu();
    void applyingAPresetIsOneHistoryState();
    void aHandEditShowsCustom();
    void savePresetAppendsAndPersists();
    void reservedNamesAreRejected();
    void saveIsWhitelisted();
    void panelLoadAppliesASavedPreset();
    void dialogLoadAppliesASavedPreset();
    void establishedBuiltinsAreListed();
    void photoFilterPresetKeepsDensity();
    void presetsSurviveARestart();
    void malformedStoreReadsEmpty();

private:
    bool setupAdjustment(const QString& kind);
    QString presetDir() const;

    QTemporaryDir dataHome_;
    QByteArray priorDataHome_;
    bool hadPriorDataHome_ = false;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PropertiesPanel* panel_ = nullptr;
    pictura::PictureView* view_ = nullptr;
    QString path_;
};

void AdjustmentPresetsTest::initTestCase()
{
    QVERIFY(dataHome_.isValid());
    hadPriorDataHome_ = qEnvironmentVariableIsSet("XDG_DATA_HOME");
    priorDataHome_ = hadPriorDataHome_ ? qgetenv("XDG_DATA_HOME") : QByteArray();
    qputenv("XDG_DATA_HOME", dataHome_.path().toUtf8());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void AdjustmentPresetsTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    QDir(presetDir()).removeRecursively();
}

void AdjustmentPresetsTest::cleanupTestCase()
{
    if (hadPriorDataHome_) {
        qputenv("XDG_DATA_HOME", priorDataHome_);
    } else {
        qunsetenv("XDG_DATA_HOME");
    }
}

QString AdjustmentPresetsTest::presetDir() const
{
    return dataHome_.path() + QStringLiteral("/kooka-pictura/Presets/Adjustments");
}

bool AdjustmentPresetsTest::setupAdjustment(const QString& kind)
{
    if (!window_->newDocument(QStringLiteral("Presets"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_ || !layers || !view_->add_adjustment(kind)) {
        return false;
    }
    path_.clear();
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view_, i);
        if (row.hasAdjustment) {
            path_ = row.path;
        }
    }
    panel_->setView(view_);
    layers->setView(view_);
    if (path_.isEmpty() || !layers->selectRowForTest(path_)) {
        return false;
    }
    view_->set_presented_view(0, 0, 0, 16, 16);
    panel_->refresh();
    return panel_->pathForTest() == path_;
}

void AdjustmentPresetsTest::propertiesMenuListsBuiltinsAndSaved()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* combo = panel_->presetComboForTest();
    QVERIFY(combo != nullptr);
    QVERIFY(combo->isVisibleTo(panel_));
    QVERIFY(combo->findText(QStringLiteral("Default")) >= 0);
    QVERIFY(combo->findText(QStringLiteral("Increase Contrast 1")) >= 0);
    QVERIFY(combo->findText(QStringLiteral("Custom")) >= 0);

    panel_->setPresetNameProviderForTest([] { return QStringLiteral("My Look"); });
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));
    QVERIFY(combo->findText(QStringLiteral("My Look")) >= 0);
}

void AdjustmentPresetsTest::unsupportedKindHasNoMenu()
{
    QVERIFY(setupAdjustment(QStringLiteral("brightness-contrast")));
    auto* combo = panel_->presetComboForTest();
    QVERIFY(combo != nullptr);
    QVERIFY(!combo->isVisibleTo(panel_));
}

void AdjustmentPresetsTest::applyingAPresetIsOneHistoryState()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* combo = panel_->presetComboForTest();
    const int index = combo->findText(QStringLiteral("Increase Contrast 1"));
    QVERIFY(index >= 0);
    const int history = view_->history_count();
    emit combo->activated(index);
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Modify Levels Layer"));
    auto* inBlack = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(inBlack != nullptr);
    QCOMPARE(inBlack->value(), 5);
}

void AdjustmentPresetsTest::aHandEditShowsCustom()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* combo = panel_->presetComboForTest();
    const int index = combo->findText(QStringLiteral("Increase Contrast 1"));
    QVERIFY(index >= 0);
    emit combo->activated(index);
    QCoreApplication::processEvents();
    auto* inBlack = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(inBlack != nullptr);
    inBlack->setValue(10);
    QCoreApplication::processEvents();
    panel_->commitForTest();
    QCOMPARE(combo->currentText(), QStringLiteral("Custom"));
}

void AdjustmentPresetsTest::savePresetAppendsAndPersists()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* inBlack = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(inBlack != nullptr);
    inBlack->setValue(10);
    QCoreApplication::processEvents();
    panel_->commitForTest();

    panel_->setPresetNameProviderForTest([] { return QStringLiteral("My Look"); });
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));
    auto* combo = panel_->presetComboForTest();
    QVERIFY(combo->findText(QStringLiteral("My Look")) >= 0);

    pictura::PropertiesPanel fresh(window_.get());
    fresh.setView(view_);
    fresh.refresh();
    QCOMPARE(fresh.pathForTest(), path_);
    auto* freshCombo = fresh.presetComboForTest();
    QVERIFY(freshCombo != nullptr);
    const int index = freshCombo->findText(QStringLiteral("My Look"));
    QVERIFY(index >= 0);
    emit freshCombo->activated(index);
    QCoreApplication::processEvents();
    auto* freshInBlack =
        qobject_cast<QSlider*>(fresh.controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(freshInBlack != nullptr);
    QCOMPARE(freshInBlack->value(), 10);
}

void AdjustmentPresetsTest::reservedNamesAreRejected()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    QString name = QStringLiteral("Default");
    panel_->setPresetNameProviderForTest([&name] { return name; });
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));
    QVERIFY(pictura::savedPresetNames(QStringLiteral("levels")).isEmpty());

    name = QStringLiteral("Custom");
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));
    QVERIFY(pictura::savedPresetNames(QStringLiteral("levels")).isEmpty());

    name.clear();
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));
    QVERIFY(pictura::savedPresetNames(QStringLiteral("levels")).isEmpty());
}

void AdjustmentPresetsTest::saveIsWhitelisted()
{
    for (const char* kind : {"levels", "curves", "exposure", "hue-saturation", "black-white",
                             "channel-mixer", "selective-color"}) {
        QVERIFY2(pictura::adjustmentPresetCanSave(QString::fromLatin1(kind)), kind);
    }
    for (const char* kind : {"photo-filter", "vibrance", "brightness-contrast"}) {
        QVERIFY2(!pictura::adjustmentPresetCanSave(QString::fromLatin1(kind)), kind);
    }
    QVERIFY(setupAdjustment(QStringLiteral("vibrance")));
    QVERIFY(!panel_->canSavePresetForTest());
}

void AdjustmentPresetsTest::panelLoadAppliesASavedPreset()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* inBlack = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(inBlack != nullptr);
    inBlack->setValue(10);
    QCoreApplication::processEvents();
    panel_->commitForTest();
    panel_->setPresetNameProviderForTest([] { return QStringLiteral("My Look"); });
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));

    inBlack->setValue(0);
    QCoreApplication::processEvents();
    panel_->commitForTest();
    QCOMPARE(inBlack->value(), 0);

    panel_->setPresetPickProviderForTest([](const QStringList& names) {
        return names.contains(QStringLiteral("My Look")) ? QStringLiteral("My Look") : QString();
    });
    panel_->performPanelMenuAction(QStringLiteral("loadPreset"));
    QCOMPARE(inBlack->value(), 10);
}

void AdjustmentPresetsTest::dialogLoadAppliesASavedPreset()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* inBlack = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(inBlack != nullptr);
    inBlack->setValue(10);
    QCoreApplication::processEvents();
    panel_->commitForTest();
    panel_->setPresetNameProviderForTest([] { return QStringLiteral("My Look"); });
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));

    const QByteArray block = toBytes(pictura::adjustment_block(*view_, path_));
    QVERIFY(!block.isEmpty());
    pictura::LevelsDialog dialog(view_, block, QRect());
    auto* preset = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("levelsPreset")));
    QVERIFY(preset != nullptr);
    const int index = preset->findText(QStringLiteral("My Look"));
    QVERIFY(index >= 0);
    emit preset->activated(index);
    QCoreApplication::processEvents();
    auto* spin = qobject_cast<QSpinBox*>(dialog.controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(spin != nullptr);
    QCOMPARE(spin->value(), 10);
    dialog.reject();
}

void AdjustmentPresetsTest::establishedBuiltinsAreListed()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* levelsCombo = panel_->presetComboForTest();
    QVERIFY(levelsCombo->findText(QStringLiteral("Increase Contrast 1")) >= 0);

    pictura::CurvesDialog curves(view_, pictura::defaultPresetBlock(QStringLiteral("curves")),
                                 QRect());
    auto* curvePreset = curves.findChild<pictura::PresetCombo*>(QStringLiteral("adjustmentPreset"));
    QVERIFY(curvePreset != nullptr);
    QVERIFY(curvePreset->findText(QStringLiteral("Strong Contrast")) >= 0);
    curves.reject();

    pictura::PhotoFilterDialog photo(
        view_, pictura::defaultPresetBlock(QStringLiteral("photo-filter")), QRect());
    auto* photoPreset = photo.findChild<pictura::PresetCombo*>(QStringLiteral("adjustmentPreset"));
    QVERIFY(photoPreset != nullptr);
    QVERIFY(photoPreset->findText(QStringLiteral("Cooling Filter (80)")) >= 0);
    photo.reject();

    const auto dialog = pictura::makeAdjustmentDialog(
        QStringLiteral("selective-color"),
        view_, pictura::defaultPresetBlock(QStringLiteral("selective-color")), QRect());
    auto* selectivePreset = qobject_cast<pictura::PresetCombo*>(
        dialog->controlForTest(QStringLiteral("preset")));
    QVERIFY(selectivePreset != nullptr);
    QVERIFY(selectivePreset->findText(QStringLiteral("Default")) >= 0);
    QVERIFY(selectivePreset->findText(QStringLiteral("Custom")) >= 0);
    QCOMPARE(selectivePreset->findText(QStringLiteral("Increase Contrast 1")), -1);
    dialog->reject();
}

void AdjustmentPresetsTest::photoFilterPresetKeepsDensity()
{
    QVERIFY(setupAdjustment(QStringLiteral("photo-filter")));
    auto* density = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("density")));
    QVERIFY(density != nullptr);
    density->setValue(50);
    QCoreApplication::processEvents();
    panel_->commitForTest();
    QCOMPARE(density->value(), 50);

    auto* combo = panel_->presetComboForTest();
    const int index = combo->findText(QStringLiteral("Cooling Filter (80)"));
    QVERIFY(index >= 0);
    emit combo->activated(index);
    QCoreApplication::processEvents();
    QCOMPARE(density->value(), 50);
}

void AdjustmentPresetsTest::presetsSurviveARestart()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    auto* inBlack = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(inBlack != nullptr);
    inBlack->setValue(10);
    QCoreApplication::processEvents();
    panel_->commitForTest();
    panel_->setPresetNameProviderForTest([] { return QStringLiteral("My Look"); });
    panel_->performPanelMenuAction(QStringLiteral("savePreset"));

    pictura::PropertiesPanel fresh(window_.get());
    fresh.setView(view_);
    fresh.refresh();
    auto* combo = fresh.presetComboForTest();
    QVERIFY(combo != nullptr);
    const int index = combo->findText(QStringLiteral("My Look"));
    QVERIFY(index >= 0);
    emit combo->activated(index);
    QCoreApplication::processEvents();
    auto* freshInBlack =
        qobject_cast<QSlider*>(fresh.controlForTest(QStringLiteral("inputBlack")));
    QVERIFY(freshInBlack != nullptr);
    QCOMPARE(freshInBlack->value(), 10);
}

void AdjustmentPresetsTest::malformedStoreReadsEmpty()
{
    QVERIFY(setupAdjustment(QStringLiteral("levels")));
    QVERIFY(QDir().mkpath(presetDir()));
    QFile file(presetDir() + QStringLiteral("/levels.presets"));
    QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
    file.write("garbage\nnot-a-record\nBad\tzzzz\n");
    file.close();

    auto* combo = panel_->presetComboForTest();
    combo->reload();
    QVERIFY(pictura::savedPresetNames(QStringLiteral("levels")).isEmpty());
    QCOMPARE(combo->findText(QStringLiteral("garbage")), -1);
}

QTEST_MAIN(AdjustmentPresetsTest)

#include "tst_adjustment_presets.moc"
