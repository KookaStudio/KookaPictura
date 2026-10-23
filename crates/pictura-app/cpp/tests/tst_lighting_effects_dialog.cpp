#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "lighting_canvas.h"
#include "lighting_effects_dialog.h"
#include "lighting_rig.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

using pictura::LightingCanvas;
using pictura::LightingEffectsDialog;
using pictura::LightKind;

const QStringList kLightingPath = {QStringLiteral("Filter"), QStringLiteral("Render"),
                                   QStringLiteral("Lighting Effects")};

const pictura::FilterCommandSpec& lightingSpec()
{
    return *pictura::filterCommandForPath(kLightingPath);
}

LightingEffectsDialog* openWorkspace()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<LightingEffectsDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

int luma(const QImage& image, double fx, double fy)
{
    const QColor c = image.pixelColor(static_cast<int>(fx * (image.width() - 1)),
                                      static_cast<int>(fy * (image.height() - 1)));
    return c.red() + c.green() + c.blue();
}

QStringList lightNames(const LightingEffectsDialog& dialog)
{
    QStringList names;
    for (QLabel* label : dialog.findChildren<QLabel*>(QStringLiteral("lightingName"))) {
        names.append(label->text());
    }
    return names;
}

void drag(QWidget* widget, const QPointF& from, const QPointF& to,
          Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QTest::mousePress(widget, Qt::LeftButton, modifiers, from.toPoint());
    QTest::mouseMove(widget, ((from + to) / 2.0).toPoint());
    QTest::mouseMove(widget, to.toPoint());
    QTest::mouseRelease(widget, Qt::LeftButton, modifiers, to.toPoint());
}

} // namespace

class LightingEffectsDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void layoutFollowsCs6();
    void slotsRoundTripTheRig();
    void presetsLoadCs6Styles();
    void lightsAreAddedUpToSixteenAndDeleted();
    void propertiesFollowTheSelectedLight();
    void canvasHandlesEditTheSelectedLight();
    void previewShowsTheLightWhereItIs();
    void menuCommitsTheRig();
    void cancelLeavesTheDocumentAlone();
    void lastFilterSettingsReopensTheRig();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LightingEffectsDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // An even grey shows where light lands and where it does not.
    QImage grey(120, 80, QImage::Format_RGB32);
    grey.fill(QColor(128, 128, 128));
    const QString path = dir_.filePath(QStringLiteral("grey.png"));
    QVERIFY(grey.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kLightingPath) != nullptr);
}

void LightingEffectsDialogTest::layoutFollowsCs6()
{
    LightingEffectsDialog dialog(window_->activeView(), lightingSpec());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Lighting Effects"));
    // CS6's Default style: one white spotlight, intensity 35, hotspot 69.
    QCOMPARE(dialog.values(), pictura::defaultLightingRig().toSlots());
    QCOMPARE(dialog.values().size(), pictura::kLightingRigSlots + pictura::kLightingLightSlots);
    QCOMPARE(dialog.values().mid(pictura::kLightingRigSlots + 5, 2), QList<double>({35.0, 69.0}));

    auto* presets = dialog.findChild<QComboBox*>(QStringLiteral("lightingPresets"));
    QVERIFY(presets != nullptr);
    QCOMPARE(presets->count(), 18); // seventeen styles and Custom
    QCOMPARE(presets->currentText(), QStringLiteral("Default"));
    QCOMPARE(presets->itemText(0), QStringLiteral("2 o'clock Spotlight"));
    QCOMPARE(presets->itemText(presets->count() - 1), QStringLiteral("Custom"));

    auto* type = dialog.findChild<QComboBox*>(QStringLiteral("lightingType"));
    QVERIFY(type != nullptr);
    QCOMPARE(type->count(), 3);
    QCOMPARE(type->currentText(), QStringLiteral("Spot"));
    auto* texture = dialog.findChild<QComboBox*>(QStringLiteral("lightingTexture"));
    QVERIFY(texture != nullptr);
    QCOMPARE(texture->itemText(3), QStringLiteral("Blue"));
    for (const char* name :
         {"lightingIntensity", "lightingHotspot", "lightingExposure", "lightingGloss",
          "lightingMetallic", "lightingAmbience", "lightingHeight"}) {
        auto* spin = dialog.findChild<QSpinBox*>(QLatin1String(name));
        QVERIFY2(spin != nullptr, name);
        QCOMPARE(spin->maximum(), 100);
    }
    QVERIFY(dialog.findChild<QSpinBox*>(QStringLiteral("lightingHotspot"))->isEnabled());
    // Height stands idle until a texture is chosen.
    QVERIFY(!dialog.findChild<QSpinBox*>(QStringLiteral("lightingHeight"))->isEnabled());
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->isChecked());
    QCOMPARE(lightNames(dialog), QStringList({QStringLiteral("Spot Light 1")}));
    QVERIFY(!dialog.findChild<QToolButton*>(QStringLiteral("lightingDelete"))->isEnabled());

    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    const auto at = [&dialog](QWidget* w) { return QRect(w->mapTo(&dialog, QPoint(0, 0)), w->size()); };
    const QRect canvas = at(dialog.canvas());
    const QRect bar = at(dialog.findChild<QWidget*>(QStringLiteral("lightingOptionsBar")));
    const QRect properties = at(dialog.findChild<QWidget*>(QStringLiteral("lightingProperties")));
    const QRect lights = at(dialog.findChild<QWidget*>(QStringLiteral("lightingLightsPanel")));
    // The options bar over the canvas; Properties over Lights to its right.
    QVERIFY(bar.bottom() < canvas.top());
    QVERIFY(properties.left() > canvas.right());
    QVERIFY(lights.top() > properties.bottom());
    for (QPushButton* button : dialog.findChildren<QPushButton*>()) {
        if (button->text() == QStringLiteral("OK") || button->text() == QStringLiteral("Cancel")) {
            QVERIFY(bar.contains(at(button)));
        }
    }
    dialog.reject();
}

void LightingEffectsDialogTest::slotsRoundTripTheRig()
{
    pictura::LightingRig rig = pictura::lightingPresets().at(2).rig; // Circle Of Light
    rig.texture = 2;
    rig.gloss = 40;
    rig.lights[1].on = false;
    const QList<double> rigSlots = rig.toSlots();
    QCOMPARE(rigSlots.size(), pictura::kLightingRigSlots + 4 * pictura::kLightingLightSlots);
    pictura::LightingRig back;
    QVERIFY(pictura::LightingRig::fromSlots(rigSlots, &back));
    QCOMPARE(back.toSlots(), rigSlots);
    QVERIFY(!pictura::LightingRig::fromSlots(rigSlots.mid(0, rigSlots.size() - 1), &back));
    QVERIFY(!pictura::LightingRig::fromSlots(rigSlots.mid(0, pictura::kLightingRigSlots), &back));
    // The bridge maps every rig the workspace can produce.
    QCOMPARE(pictura::filter_param_arity(QStringLiteral("lighting-effects")),
             pictura::kLightingRigSlots + pictura::kLightingLightSlots);
    QVERIFY(!pictura::filter_thumbnail(QByteArray(4 * 4 * 4, char(128)), 4, 4,
                              QStringLiteral("lighting-effects"), rigSlots, 1.0)
                 .isNull());
}

void LightingEffectsDialogTest::presetsLoadCs6Styles()
{
    LightingEffectsDialog dialog(window_->activeView(), lightingSpec());
    auto* presets = dialog.findChild<QComboBox*>(QStringLiteral("lightingPresets"));
    dialog.applyPreset(QStringLiteral("RGB Lights"));
    QCOMPARE(presets->currentText(), QStringLiteral("RGB Lights"));
    QCOMPARE(dialog.rig().lights.size(), 3);
    QCOMPARE(lightNames(dialog), QStringList({QStringLiteral("Spot Light 1"),
                                              QStringLiteral("Spot Light 2"),
                                              QStringLiteral("Spot Light 3")}));
    QVERIFY(dialog.rig().lights[0].color.red() > dialog.rig().lights[0].color.blue());
    dialog.applyPreset(QStringLiteral("Five Lights Down"));
    QCOMPARE(dialog.rig().lights.size(), 5);
    dialog.applyPreset(QStringLiteral("Soft Direct Lights"));
    QCOMPARE(dialog.rig().lights.first().kind, LightKind::Infinite);

    // Any edit makes the rig the user's own.
    dialog.findChild<QSpinBox*>(QStringLiteral("lightingGloss"))->setValue(30);
    QCOMPARE(presets->currentText(), QStringLiteral("Custom"));
    QCOMPARE(dialog.rig().gloss, 30.0);
    dialog.findChild<QPushButton*>(QStringLiteral("lightingReset"))->click();
    QCOMPARE(dialog.values(), pictura::defaultLightingRig().toSlots());
    QCOMPARE(presets->currentText(), QStringLiteral("Default"));
}

void LightingEffectsDialogTest::lightsAreAddedUpToSixteenAndDeleted()
{
    LightingEffectsDialog dialog(window_->activeView(), lightingSpec());
    dialog.findChild<QToolButton*>(QStringLiteral("lightingAddPoint"))->click();
    dialog.findChild<QToolButton*>(QStringLiteral("lightingAddInfinite"))->click();
    QCOMPARE(lightNames(dialog),
             QStringList({QStringLiteral("Spot Light 1"), QStringLiteral("Point Light 1"),
                          QStringLiteral("Infinite Light 1")}));
    QCOMPARE(dialog.selectedLight(), 2);
    QCOMPARE(dialog.values().size(), pictura::kLightingRigSlots + 3 * pictura::kLightingLightSlots);

    while (dialog.addLight(LightKind::Spot)) {
    }
    QCOMPARE(dialog.rig().lights.size(), pictura::kMaxLights);
    QVERIFY(!dialog.findChild<QToolButton*>(QStringLiteral("lightingAddSpot"))->isEnabled());

    auto* trash = dialog.findChild<QToolButton*>(QStringLiteral("lightingDelete"));
    dialog.selectLight(1);
    trash->click();
    QCOMPARE(dialog.rig().lights.size(), pictura::kMaxLights - 1);
    QCOMPARE(dialog.rig().lights.at(1).kind, LightKind::Infinite);
    QVERIFY(dialog.findChild<QToolButton*>(QStringLiteral("lightingAddSpot"))->isEnabled());
    while (dialog.deleteSelectedLight()) {
    }
    QCOMPARE(dialog.rig().lights.size(), 1); // a rig keeps at least one light
    QVERIFY(!trash->isEnabled());
}

void LightingEffectsDialogTest::propertiesFollowTheSelectedLight()
{
    LightingEffectsDialog dialog(window_->activeView(), lightingSpec());
    dialog.addLight(LightKind::Point);
    auto* intensity = dialog.findChild<QSpinBox*>(QStringLiteral("lightingIntensity"));
    auto* hotspot = dialog.findChild<QSpinBox*>(QStringLiteral("lightingHotspot"));
    auto* type = dialog.findChild<QComboBox*>(QStringLiteral("lightingType"));
    QCOMPARE(type->currentText(), QStringLiteral("Point"));
    QVERIFY(!hotspot->isEnabled()); // CS6 greys Hotspot for a Point light
    intensity->setValue(-40);
    QCOMPARE(dialog.rig().lights.at(1).intensity, -40.0);
    QCOMPARE(dialog.rig().lights.at(0).intensity, 35.0);

    // The panel's light list selects, and the controls follow.
    dialog.findChild<QListWidget*>(QStringLiteral("lightingLights"))->setCurrentRow(0);
    QCOMPARE(dialog.selectedLight(), 0);
    QCOMPARE(intensity->value(), 35);
    QVERIFY(hotspot->isEnabled());

    // The rig-wide properties apply whatever light is selected.
    dialog.findChild<QComboBox*>(QStringLiteral("lightingTexture"))->setCurrentIndex(1);
    QVERIFY(dialog.findChild<QSpinBox*>(QStringLiteral("lightingHeight"))->isEnabled());
    QCOMPARE(dialog.values().at(7), 1.0);

    // Hiding a light keeps it in the rig but out of the light.
    auto* eye = dialog.findChildren<QToolButton*>(QStringLiteral("lightingEye")).at(1);
    eye->setChecked(false);
    QVERIFY(!dialog.rig().lights.at(1).on);
    QCOMPARE(dialog.values().at(pictura::kLightingRigSlots + pictura::kLightingLightSlots + 1), 0.0);
}

void LightingEffectsDialogTest::canvasHandlesEditTheSelectedLight()
{
    LightingEffectsDialog dialog(window_->activeView(), lightingSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    LightingCanvas* canvas = dialog.canvas();
    using Part = LightingCanvas::Part;

    // Inside the ellipse moves the light.
    const QPointF centre = canvas->handlePosition(0, Part::Move);
    drag(canvas, centre, canvas->toWidget(QPointF(0.3, 0.4)));
    QVERIFY(qAbs(dialog.rig().lights[0].center.x() - 0.3) < 0.02);
    QVERIFY(qAbs(dialog.rig().lights[0].center.y() - 0.4) < 0.02);

    // The handle on the long axis stretches it; the one across widens it.
    const double size = dialog.rig().lights[0].size;
    const QPointF major = canvas->handlePosition(0, Part::Major);
    const QPointF outward = major + (major - canvas->handlePosition(0, Part::Move)) * 0.25;
    drag(canvas, major, outward);
    QVERIFY(dialog.rig().lights[0].size > size * 1.1);
    const double width = dialog.rig().lights[0].width;
    const QPointF minor = canvas->handlePosition(0, Part::Minor);
    drag(canvas, minor, minor + (minor - canvas->handlePosition(0, Part::Move)) * 0.5);
    QVERIFY(dialog.rig().lights[0].width > width * 1.2);

    // The Intensity ring: a quarter turn from the top reads -50.
    const QPointF c = canvas->handlePosition(0, Part::Move);
    const QPointF top = canvas->handlePosition(0, Part::Intensity);
    int index = -1;
    QCOMPARE(canvas->hitTest(top, &index), Part::Intensity);
    drag(canvas, top + QPointF(2, 0), c + QPointF(13, 0));
    // Integer mouse positions put the quarter turn within a step of -50.
    QVERIFY(qAbs(dialog.rig().lights[0].intensity + 50.0) <= 1.0);
    QCOMPARE(dialog.findChild<QSpinBox*>(QStringLiteral("lightingIntensity"))->value(),
             qRound(dialog.rig().lights[0].intensity));

    // Beyond the ellipse turns it.
    drag(canvas, c + QPointF(0, -canvas->height()), c + QPointF(0, canvas->height()));
    QVERIFY(qAbs(dialog.rig().lights[0].angle - 90.0) < 1.0);

    // Another light's centre selects it; Alt-dragging copies the selected one.
    dialog.addLight(LightKind::Point);
    QCOMPARE(dialog.selectedLight(), 1);
    QCOMPARE(canvas->hitTest(c, &index), Part::Move);
    QCOMPARE(index, 0);
    drag(canvas, canvas->handlePosition(1, Part::Move),
         canvas->handlePosition(1, Part::Move) + QPointF(30, 30), Qt::AltModifier);
    QCOMPARE(dialog.rig().lights.size(), 3);
    QCOMPARE(dialog.selectedLight(), 2);
    QCOMPARE(dialog.rig().lights[2].kind, LightKind::Point);
    QVERIFY(dialog.rig().lights[2].center != dialog.rig().lights[1].center);
    QCOMPARE(lightNames(dialog).last(), QStringLiteral("Point Light 2"));

    // An Infinite light's end handle sets where it shines from.
    dialog.addLight(LightKind::Infinite);
    const int sun = dialog.selectedLight();
    const QPointF from = canvas->handlePosition(sun, Part::Move);
    QCOMPARE(canvas->hitTest(canvas->handlePosition(sun, Part::Direction), &index),
             Part::Direction);
    drag(canvas, canvas->handlePosition(sun, Part::Direction), from + QPointF(-35, 0));
    QVERIFY(qAbs(dialog.rig().lights[sun].angle - 180.0) < 1.0);
    QVERIFY(qAbs(dialog.rig().lights[sun].elevation - 60.0) < 1.0);
    dialog.reject();
}

void LightingEffectsDialogTest::previewShowsTheLightWhereItIs()
{
    LightingEffectsDialog dialog(window_->activeView(), lightingSpec());
    QImage lit = dialog.previewImage();
    QVERIFY(!lit.isNull());
    QCOMPARE(lit.size(), QSize(120, 80));
    // Lit in the ellipse, black outside it with no ambience.
    QVERIFY(luma(lit, 0.5, 0.5) > 150);
    QCOMPARE(luma(lit, 0.02, 0.98), 0);

    dialog.findChild<QSpinBox*>(QStringLiteral("lightingAmbience"))->setValue(50);
    QVERIFY(luma(dialog.previewImage(), 0.02, 0.98) > 150);

    dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"))->setChecked(false);
    QCOMPARE(luma(dialog.previewImage(), 0.02, 0.98), 3 * 128);
    dialog.reject();
}

void LightingEffectsDialogTest::menuCommitsTheRig()
{
    pictura::PictureView* view = window_->activeView();
    const int before = view->history_count();
    QList<double> committed;
    QTimer::singleShot(0, [&committed] {
        if (auto* dialog = openWorkspace()) {
            dialog->addLight(LightKind::Point);
            committed = dialog->values();
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kLightingPath)));
    QCOMPARE(committed.size(), pictura::kLightingRigSlots + 2 * pictura::kLightingLightSlots);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("lighting-effects"));
    QCOMPARE(filter_last_params(*view), committed);
    // The spot's far corner stays dark; its middle is lit.
    const QImage image = view->image();
    QVERIFY(luma(image, 0.5, 0.5) > 150);
    QCOMPARE(luma(image, 0.02, 0.98), 0);
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditUndo)));
}

void LightingEffectsDialogTest::cancelLeavesTheDocumentAlone()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    const int history = view->history_count();
    QTimer::singleShot(0, [] {
        if (auto* dialog = openWorkspace()) {
            dialog->applyPreset(QStringLiteral("Blue Omni"));
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kLightingPath)));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), before);
}

void LightingEffectsDialogTest::lastFilterSettingsReopensTheRig()
{
    pictura::PictureView* view = window_->activeView();
    pictura::LightingRig rig = pictura::lightingPresets().at(4).rig; // Crossing Down
    rig.ambience = 20;
    QVERIFY(apply_filter_params(*view, QStringLiteral("lighting-effects"), rig.toSlots()));
    QList<double> seen;
    QString preset;
    QTimer::singleShot(0, [&seen, &preset] {
        if (auto* dialog = openWorkspace()) {
            seen = dialog->values();
            preset = dialog->findChild<QComboBox*>(QStringLiteral("lightingPresets"))->currentText();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(seen, rig.toSlots());
    QCOMPARE(preset, QStringLiteral("Custom"));
}

QTEST_MAIN(LightingEffectsDialogTest)
#include "tst_lighting_effects_dialog.moc"
