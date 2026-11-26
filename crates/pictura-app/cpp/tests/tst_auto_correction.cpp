#include <QtTest/QtTest>

#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/layers_bridge.h"
#include "auto_correction_options.h"
#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/properties_panel.h"
#include "preferences.h"
#include "qt_test_support.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QMenu>
#include <QtWidgets/QRadioButton>

#include <functional>

namespace {

QByteArray block(const pictura::PictureView& view, const QString& path)
{
    const auto bytes = pictura::adjustment_block(view, path);
    return QByteArray(reinterpret_cast<const char*>(bytes.data()), qsizetype(bytes.size()));
}

QDoubleSpinBox* shadowClip(QDialog& dialog)
{
    return dialog.findChild<QDoubleSpinBox*>(QStringLiteral("autoCorrectionShadowClip"));
}

bool saveTestDefaults()
{
    auto settings = pictura::autoCorrectionSettings(pictura::defaultPreferences());
    settings.algorithm = 2;
    settings.shadow_clip = 0.0;
    settings.highlight_clip = 0.0;
    settings.shadow = 0x202020u;
    settings.highlight = 0xe0e0e0u;
    return pictura::saveAutoCorrectionSettings(settings);
}

} // namespace

class AutoCorrectionTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void autoOptionsEntryPoints();
    void propertiesPresetMenuNames();
    void algorithmAvailability();
    void autoOptionsPreviewAndUndo();
    void autoOptionsCancelExactPreview();
    void autoOptionsRestoredPreviewIsClean();
    void autoOptionsDefaultsPersist();
    void directAutoUsesSavedOptions();
    void directAutoFactoryCompatibility();
    void directAutoTargetDispatch();

private:
    bool openRamp();
    bool addAdjustment(const QString& kind);
    bool withOptions(bool alt, const std::function<void(QDialog&)>& interact);
    QAction* imageAuto(const QString& name) const;

    std::unique_ptr<pictura::test::ScopedStateHome> profile_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir files_;
    pictura::PictureView* view_ = nullptr;
    pictura::PropertiesPanel* panel_ = nullptr;
    QString path_;
};

void AutoCorrectionTest::init()
{
    profile_ = std::make_unique<pictura::test::ScopedStateHome>();
    QVERIFY(profile_->isValid() && files_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    QVERIFY(panel_);
}

void AutoCorrectionTest::cleanup()
{
    window_.reset();
    profile_.reset();
}

bool AutoCorrectionTest::openRamp()
{
    if (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    QImage image(20, 16, QImage::Format_RGB32);
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            const int v = 30 + 10 * x;
            image.setPixel(x, y, qRgb(v, v, v));
        }
    }
    const auto file = files_.filePath(QStringLiteral("auto-ramp.png"));
    if (!image.save(file) || !window_->openDocumentAtPath(file)) {
        return false;
    }
    view_ = window_->activeView();
    if (view_) {
        view_->set_presented_view(0, 0, 0, 20, 16);
    }
    window_->registry()->refresh();
    return view_;
}

bool AutoCorrectionTest::addAdjustment(const QString& kind)
{
    if (!view_->add_adjustment(kind)) {
        return false;
    }
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!layers) {
        return false;
    }
    layers->setView(view_);
    panel_->setView(view_);
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const auto row = pictura::layers_api::rowAt(*view_, i);
        if (row.adjustmentKind == kind) {
            path_ = row.path;
            if (!layers->selectRowForTest(path_)) {
                return false;
            }
            panel_->refresh();
            window_->registry()->refresh();
            return true;
        }
    }
    return false;
}

bool AutoCorrectionTest::withOptions(bool alt, const std::function<void(QDialog&)>& interact)
{
    bool opened = false;
    QTimer interaction;
    interaction.setSingleShot(true);
    connect(&interaction, &QTimer::timeout, this, [&] {
        auto* dialog = window_->findChild<QDialog*>(QStringLiteral("autoCorrectionOptions"));
        if (dialog) {
            opened = true;
            interact(*dialog);
            if (dialog->isVisible()) {
                dialog->reject();
            }
        }
    });
    QTimer watchdog;
    watchdog.setSingleShot(true);
    connect(&watchdog, &QTimer::timeout, this, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* dialog = qobject_cast<QDialog*>(widget); dialog && dialog->isVisible()) {
                dialog->reject();
            }
        }
    });
    watchdog.start(3000);
    interaction.start(0);
    if (alt) {
        auto* button = panel_->findChild<QPushButton*>(QStringLiteral("propertiesAuto"));
        if (!button || button->isHidden()) {
            return false;
        }
        QTest::mouseClick(button, Qt::LeftButton, Qt::AltModifier);
    } else {
        window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::PanelPropertiesAutoOptions));
    }
    return opened;
}

QAction* AutoCorrectionTest::imageAuto(const QString& name) const
{
    return window_->registry()->action(pictura::commandIdForPath({QStringLiteral("Image"), name}));
}

void AutoCorrectionTest::autoOptionsEntryPoints()
{
    QVERIFY(openRamp());
    for (const auto& kind : {QStringLiteral("levels"), QStringLiteral("curves")}) {
        QVERIFY(addAdjustment(kind));
        QVERIFY(withOptions(false, [&](QDialog& dialog) {
            auto* algorithm = dialog.findChild<QRadioButton*>(QStringLiteral("autoCorrectionAlgorithm1"));
            QVERIFY(algorithm && algorithm->isChecked());
            QVERIFY(shadowClip(dialog));
            QCOMPARE(shadowClip(dialog)->value(), 0.1);
        }));
        QVERIFY(withOptions(true, [](QDialog& dialog) { dialog.reject(); }));
    }
    QVERIFY(addAdjustment(QStringLiteral("threshold")));
    QVERIFY(!panel_->canAutoCorrect());
    QVERIFY(panel_->findChild<QPushButton*>(QStringLiteral("propertiesAuto"))->isHidden());
    QVERIFY(!withOptions(false, [](QDialog&) {}));
}

void AutoCorrectionTest::algorithmAvailability()
{
    QVERIFY(openRamp());
    for (const auto& kind : {QStringLiteral("levels"), QStringLiteral("curves")}) {
        QVERIFY(addAdjustment(kind));
        QVERIFY(withOptions(false, [](QDialog& dialog) {
            auto* brightness = dialog.findChild<QRadioButton*>(QStringLiteral("autoCorrectionAlgorithm3"));
            auto* colors = dialog.findChild<QRadioButton*>(QStringLiteral("autoCorrectionAlgorithm2"));
            auto* snap = dialog.findChild<QCheckBox*>(QStringLiteral("autoCorrectionSnap"));
            auto* clip = shadowClip(dialog);
            QVERIFY(brightness && colors && snap && clip);
            brightness->click();
            QVERIFY(!snap->isEnabled());
            QVERIFY(!clip->isEnabled());
            colors->click();
            QVERIFY(snap->isEnabled());
            QVERIFY(clip->isEnabled());
        }));
    }
}

void AutoCorrectionTest::propertiesPresetMenuNames()
{
    QVERIFY(openRamp());
    QMenu menu;
    auto* save = menu.addAction(QStringLiteral("Save Preset…"));
    save->setData(QStringLiteral("panel.properties.savePreset"));
    auto* load = menu.addAction(QStringLiteral("Load Preset…"));
    load->setData(QStringLiteral("panel.properties.loadPreset"));
    for (const auto& kind : {QStringLiteral("levels"), QStringLiteral("curves")}) {
        QVERIFY(addAdjustment(kind));
        panel_->updatePanelMenu(&menu);
        const QString name = kind == QLatin1String("levels") ? QStringLiteral("Levels")
                                                               : QStringLiteral("Curves");
        QCOMPARE(save->text(), QStringLiteral("Save %1 Preset…").arg(name));
        QCOMPARE(load->text(), QStringLiteral("Load %1 Preset…").arg(name));
        QVERIFY(save->isEnabled());
        QVERIFY(!load->isEnabled());
    }
}

void AutoCorrectionTest::autoOptionsPreviewAndUndo()
{
    QVERIFY(openRamp());
    QVERIFY(addAdjustment(QStringLiteral("levels")));
    QVERIFY(view_->save(files_.filePath(QStringLiteral("auto-preview.psd"))));
    QVERIFY(!view_->is_dirty());
    const QByteArray opening = block(*view_, path_);
    const int history = view_->history_count();
    const auto change = [&](QDialog& dialog) {
        auto* clip = shadowClip(dialog);
        QVERIFY(clip);
        clip->setValue(15.0);
        QVERIFY(block(*view_, path_) != opening);
        QTest::qWait(600);
        QCOMPARE(view_->history_count(), history);
    };
    QVERIFY(withOptions(false, change));
    QCOMPARE(block(*view_, path_), opening);
    QCOMPARE(view_->history_count(), history);
    QVERIFY(!view_->is_dirty());
    QVERIFY(withOptions(false, [&](QDialog& dialog) {
        change(dialog);
        dialog.accept();
    }));
    QCOMPARE(view_->history_count(), history + 1);
    QVERIFY(view_->undo());
    QCOMPARE(block(*view_, path_), opening);

    auto* button = panel_->findChild<QPushButton*>(QStringLiteral("propertiesAuto"));
    QVERIFY(button);
    button->click();
    QVERIFY(block(*view_, path_) != opening);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Modify Levels Layer"));
}

void AutoCorrectionTest::autoOptionsCancelExactPreview()
{
    QVERIFY(openRamp());
    QVERIFY(addAdjustment(QStringLiteral("levels")));
    QVERIFY(view_->save(files_.filePath(QStringLiteral("auto-exact-preview.psd"))));
    QVERIFY(!view_->is_dirty());
    view_->set_presented_view(-1, 0, 0, 20, 16);
    const QByteArray opening = block(*view_, path_);
    const auto pixel = view_->composite_argb(0, 0);
    const int history = view_->history_count();
    QVERIFY(withOptions(false, [&](QDialog& dialog) {
        QVERIFY(shadowClip(dialog));
        shadowClip(dialog)->setValue(15.0);
        QVERIFY(block(*view_, path_) != opening);
        dialog.reject();
    }));
    QCOMPARE(block(*view_, path_), opening);
    QCOMPARE(view_->composite_argb(0, 0), pixel);
    QCOMPARE(view_->history_count(), history);
    QVERIFY(!view_->is_dirty());
}

void AutoCorrectionTest::autoOptionsRestoredPreviewIsClean()
{
    QVERIFY(openRamp());
    QVERIFY(addAdjustment(QStringLiteral("levels")));
    auto* button = panel_->findChild<QPushButton*>(QStringLiteral("propertiesAuto"));
    QVERIFY(button);
    button->click();
    QVERIFY(view_->save(files_.filePath(QStringLiteral("auto-noop.psd"))));
    const QByteArray opening = block(*view_, path_);
    const int history = view_->history_count();
    QVERIFY(withOptions(false, [&](QDialog& dialog) {
        auto* clip = shadowClip(dialog);
        QVERIFY(clip);
        clip->setValue(15.0);
        QVERIFY(block(*view_, path_) != opening);
        clip->setValue(0.1);
        QCOMPARE(block(*view_, path_), opening);
        dialog.accept();
    }));
    QCOMPARE(view_->history_count(), history);
    QVERIFY(!view_->is_dirty());
}

void AutoCorrectionTest::autoOptionsDefaultsPersist()
{
    QVERIFY(openRamp());
    QVERIFY(addAdjustment(QStringLiteral("levels")));
    QVERIFY(withOptions(false, [](QDialog& dialog) {
        auto* save = dialog.findChild<QCheckBox*>(QStringLiteral("autoCorrectionSaveDefaults"));
        auto* clip = shadowClip(dialog);
        QVERIFY(save && clip);
        clip->setValue(17.0);
        save->setChecked(true);
        dialog.accept();
    }));
    QVERIFY(pictura::loadPreferences().autoCorrectionSaved);
    QCOMPARE(pictura::loadPreferences().autoCorrectionShadowClip, 17.0);
    QVERIFY(openRamp());
    QVERIFY(addAdjustment(QStringLiteral("curves")));
    QVERIFY(withOptions(true, [](QDialog& dialog) {
        auto* clip = shadowClip(dialog);
        QVERIFY(clip);
        QCOMPARE(clip->value(), 17.0);
        clip->setValue(9.0);
        dialog.accept();
    }));
    QVERIFY(withOptions(false, [](QDialog& dialog) {
        auto* save = dialog.findChild<QCheckBox*>(QStringLiteral("autoCorrectionSaveDefaults"));
        QVERIFY(save && shadowClip(dialog));
        QCOMPARE(shadowClip(dialog)->value(), 17.0);
        shadowClip(dialog)->setValue(23.0);
        save->setChecked(true);
        dialog.reject();
    }));
    QCOMPARE(pictura::loadPreferences().autoCorrectionShadowClip, 17.0);
}

void AutoCorrectionTest::directAutoUsesSavedOptions()
{
    QVERIFY(openRamp());
    QVERIFY(saveTestDefaults());
    QVERIFY(view_->select_rect(5, 0, 10, 16, QStringLiteral("new"), 0.0));
    const int history = view_->history_count();
    auto* tone = imageAuto(QStringLiteral("Auto Tone"));
    QVERIFY(tone && tone->isEnabled());
    tone->trigger();
    QCOMPARE(view_->composite_argb(5, 0), qRgb(32, 32, 32));
    QCOMPARE(view_->composite_argb(14, 0), qRgb(224, 224, 224));
    QCOMPARE(view_->composite_argb(0, 0), qRgb(30, 30, 30));
    QCOMPARE(view_->composite_argb(19, 0), qRgb(220, 220, 220));
    QCOMPARE(view_->history_count(), history + 1);
    QVERIFY(view_->undo());
    QCOMPARE(view_->composite_argb(5, 0), qRgb(80, 80, 80));
}

void AutoCorrectionTest::directAutoFactoryCompatibility()
{
    QVERIFY(openRamp());
    QVERIFY(addAdjustment(QStringLiteral("levels")));
    QVERIFY(withOptions(false, [](QDialog& dialog) { dialog.reject(); }));
    QVERIFY(!pictura::loadPreferences().autoCorrectionSaved);
    for (bool legacy : {false, true}) {
        if (legacy) {
            QFile file(pictura::preferencesFilePath());
            QVERIFY(QDir().mkpath(pictura::configurationDirectory()));
            QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
            QVERIFY(file.write("schema_version = 1\n[interface]\nbrightnessLevel = 0\n") > 0);
        }
        for (const auto& [name, kind] : std::array<std::pair<QString, QString>, 3>{{
            {QStringLiteral("Auto Tone"), QStringLiteral("auto-tone")},
            {QStringLiteral("Auto Contrast"), QStringLiteral("auto-contrast")},
            {QStringLiteral("Auto Color"), QStringLiteral("auto-color")},
        }}) {
            QVERIFY(openRamp());
            QVERIFY(pictura::image_adjust_direct(*view_, kind));
            const QImage baseline = view_->display_image(0, 0, 0, 20, 16);
            QVERIFY(openRamp());
            auto* action = imageAuto(name);
            QVERIFY(action && action->isEnabled());
            action->trigger();
            QCOMPARE(view_->display_image(0, 0, 0, 20, 16), baseline);
        }
        QVERIFY(!pictura::loadPreferences().autoCorrectionSaved);
    }
}

void AutoCorrectionTest::directAutoTargetDispatch()
{
    QVERIFY(openRamp());
    QVERIFY(saveTestDefaults());
    QVERIFY(pictura::convert_image_depth(*view_, 16));
    auto* tone = imageAuto(QStringLiteral("Auto Tone"));
    QVERIFY(tone && tone->isEnabled());
    tone->trigger();
    QCOMPARE(pictura::image_depth_bits(*view_), 16);
    QCOMPARE(view_->composite_argb(0, 0), qRgb(32, 32, 32));
    QVERIFY(view_->undo());

    QVERIFY(openRamp());
    QVERIFY(view_->select_rect(4, 0, 8, 16, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::quick_mask_set(*view_, true));
    const auto settings = pictura::autoCorrectionSettings(pictura::loadPreferences());
    QVERIFY(pictura::image_auto_configured(*view_, QStringLiteral("auto-tone"), settings));
    QCOMPARE(pictura::quick_mask_coverage(*view_, 5, 2), 224);
    QVERIFY(view_->undo());
    QCOMPARE(pictura::quick_mask_coverage(*view_, 5, 2), 255);
    QVERIFY(pictura::quick_mask_set(*view_, false));

    // A selection-derived mask has both endpoints, so the configured targets
    // can visibly change its values without changing the original pixel layer.
    QVERIFY(pictura::layer_mask_add(*view_, QStringLiteral("reveal-selection")));
    const QString path = pictura::layers_api::rowAt(*view_, 0).path;
    QVERIFY(pictura::mask_edit_target_set(*view_, path));
    QVERIFY(pictura::image_auto_configured(*view_, QStringLiteral("auto-tone"), settings));
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Auto Tone"));
    QVERIFY(view_->undo());
}

PICTURA_QT_TEST_MAIN(AutoCorrectionTest)
#include "tst_auto_correction.moc"
