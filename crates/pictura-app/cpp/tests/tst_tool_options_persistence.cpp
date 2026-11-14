// Tool options survive a restart: the session store carries every tool's
// options-bar settings, and bad entries fall back to the defaults.

#include <QtTest/QtTest>

#include "options_bar.h"
#include "panels/numeric_field.h"
#include "tool_options_json.h"
#include "tools.h"

#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSet>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLineEdit>

#include <memory>

#include "qt_test_support.h"

namespace {

struct OptionRow {
    const char* key;
    QJsonObject values;
};

QString optionKey(const OptionRow& row) { return QString::fromLatin1(row.key); }

const QVector<OptionRow>& optionRows()
{
    static const QVector<OptionRow> rows = {
        {"move",
         QJsonObject{{QStringLiteral("autoSelectOn"), true},
                     {QStringLiteral("autoSelect"), 1},
                     {QStringLiteral("showTransformControls"), false},
                     {QStringLiteral("alignTo"), 1}}},
        {"type",
         QJsonObject{{QStringLiteral("family"), QStringLiteral("DejaVu Sans")},
                     {QStringLiteral("size"), 36.0},
                     {QStringLiteral("antialias"), 2},
                     {QStringLiteral("justification"), 2}}},
        {"mixer",
         QJsonObject{{QStringLiteral("wet"), 10},
                     {QStringLiteral("load"), 60},
                     {QStringLiteral("mix"), 20},
                     {QStringLiteral("flow"), 90},
                     {QStringLiteral("sampleAllLayers"), true},
                     {QStringLiteral("loadAfterStroke"), true},
                     {QStringLiteral("cleanAfterStroke"), true}}},
        {"eyedropper",
         QJsonObject{{QStringLiteral("sampleSize"), 5},
                     {QStringLiteral("scope"), 3},
                     {QStringLiteral("ring"), true}}},
        {"pen",
         QJsonObject{{QStringLiteral("autoAddDelete"), false},
                     {QStringLiteral("rubberBand"), true},
                     {QStringLiteral("curveFit"), 5.0},
                     {QStringLiteral("showBoundingBox"), true}}},
        {"shape",
         QJsonObject{{QStringLiteral("mode"), 1},
                     {QStringLiteral("radius"), 20.0},
                     {QStringLiteral("sides"), 6},
                     {QStringLiteral("weight"), 2.5},
                     {QStringLiteral("arrowStart"), true},
                     {QStringLiteral("arrowEnd"), true},
                     {QStringLiteral("arrowWidth"), 600.0},
                     {QStringLiteral("arrowLength"), 1200.0},
                     {QStringLiteral("arrowConcavity"), 10.0},
                     {QStringLiteral("custom"), 1},
                     {QStringLiteral("fillEnabled"), false},
                     {QStringLiteral("fillColor"), QStringLiteral("#ff123456")},
                     {QStringLiteral("strokeEnabled"), true},
                     {QStringLiteral("strokeColor"), QStringLiteral("#ffabcdef")},
                     {QStringLiteral("strokeWidth"), 4.0},
                     {QStringLiteral("strokeAlign"), 2},
                     {QStringLiteral("geometry"), 1},
                     {QStringLiteral("fixedWidth"), 120.0},
                     {QStringLiteral("fixedHeight"), 130.0},
                     {QStringLiteral("fromCenter"), true},
                     {QStringLiteral("star"), true},
                     {QStringLiteral("indent"), 60.0},
                     {QStringLiteral("smoothCorners"), true},
                     {QStringLiteral("smoothIndents"), true},
                     {QStringLiteral("alignEdges"), false},
                     {QStringLiteral("linkSize"), true}}},
        {"crop",
         QJsonObject{{QStringLiteral("showCroppedArea"), false},
                     {QStringLiteral("autoCenterPreview"), false},
                     {QStringLiteral("shieldEnabled"), false},
                     {QStringLiteral("shieldCustomColor"), true},
                     {QStringLiteral("shieldColor"), QStringLiteral("#ff112233")},
                     {QStringLiteral("shieldOpacity"), 40},
                     {QStringLiteral("autoAdjustOpacity"), false},
                     {QStringLiteral("overlayShow"), 1},
                     {QStringLiteral("overlayOrientation"), 2}}},
        {"healing",
         QJsonObject{{QStringLiteral("spotType"), 1}, {QStringLiteral("aligned"), false}}},
        {"patch",
         QJsonObject{{QStringLiteral("contentAware"), true},
                     {QStringLiteral("destination"), true},
                     {QStringLiteral("transparent"), true}}},
        {"contentAwareMove",
         QJsonObject{{QStringLiteral("extend"), true}, {QStringLiteral("adaptation"), 3}}},
        {"redEye",
         QJsonObject{{QStringLiteral("pupil"), 60}, {QStringLiteral("darken"), 40}}},
        {"colorReplace",
         QJsonObject{{QStringLiteral("mode"), 3},
                     {QStringLiteral("sampling"), 1},
                     {QStringLiteral("limits"), 2},
                     {QStringLiteral("tolerance"), 40},
                     {QStringLiteral("antialias"), false}}},
        {"stamp",
         QJsonObject{{QStringLiteral("cloneAligned"), false},
                     {QStringLiteral("cloneSample"), 1},
                     {QStringLiteral("ignoreAdjustments"), true},
                     {QStringLiteral("pattern"), 1},
                     {QStringLiteral("patternAligned"), false},
                     {QStringLiteral("cloneAirbrush"), true},
                     {QStringLiteral("patternAirbrush"), true},
                     {QStringLiteral("historyAirbrush"), true}}},
        {"eraser",
         QJsonObject{{QStringLiteral("mode"), 1}, {QStringLiteral("toHistory"), true}}},
        {"backgroundEraser",
         QJsonObject{{QStringLiteral("sampling"), 1},
                     {QStringLiteral("limits"), 2},
                     {QStringLiteral("tolerance"), 60},
                     {QStringLiteral("protectForeground"), true}}},
        {"magicEraser",
         QJsonObject{{QStringLiteral("tolerance"), 33},
                     {QStringLiteral("antialias"), false},
                     {QStringLiteral("contiguous"), false},
                     {QStringLiteral("sampleAllLayers"), true},
                     {QStringLiteral("opacity"), 90}}},
        {"artHistory",
         QJsonObject{{QStringLiteral("style"), 1},
                     {QStringLiteral("area"), 60},
                     {QStringLiteral("tolerance"), 1}}},
        {"gradient",
         QJsonObject{{QStringLiteral("preset"), 1},
                     {QStringLiteral("style"), 1},
                     {QStringLiteral("mode"), QStringLiteral("screen")},
                     {QStringLiteral("opacity"), 90},
                     {QStringLiteral("reverse"), true},
                     {QStringLiteral("dither"), true},
                     {QStringLiteral("transparency"), false}}},
        {"bucket",
         QJsonObject{{QStringLiteral("fill"), 1},
                     {QStringLiteral("pattern"), 1},
                     {QStringLiteral("mode"), QStringLiteral("multiply")},
                     {QStringLiteral("opacity"), 90},
                     {QStringLiteral("tolerance"), 33},
                     {QStringLiteral("antialias"), false},
                     {QStringLiteral("contiguous"), false},
                     {QStringLiteral("allLayers"), true}}},
        {"blur",
         QJsonObject{{QStringLiteral("strength"), 60},
                     {QStringLiteral("mode"), 1},
                     {QStringLiteral("sampleAllLayers"), true},
                     {QStringLiteral("protectDetail"), false},
                     {QStringLiteral("fingerPainting"), true}}},
        {"sharpen",
         QJsonObject{{QStringLiteral("strength"), 51},
                     {QStringLiteral("mode"), 1},
                     {QStringLiteral("sampleAllLayers"), true},
                     {QStringLiteral("protectDetail"), false},
                     {QStringLiteral("fingerPainting"), true}}},
        {"smudge",
         QJsonObject{{QStringLiteral("strength"), 52},
                     {QStringLiteral("mode"), 1},
                     {QStringLiteral("sampleAllLayers"), true},
                     {QStringLiteral("protectDetail"), false},
                     {QStringLiteral("fingerPainting"), true}}},
        {"dodge",
         QJsonObject{{QStringLiteral("amount"), 60},
                     {QStringLiteral("range"), 2},
                     {QStringLiteral("protectTones"), false},
                     {QStringLiteral("spongeMode"), 1},
                     {QStringLiteral("vibrance"), false}}},
        {"burn",
         QJsonObject{{QStringLiteral("amount"), 61},
                     {QStringLiteral("range"), 0},
                     {QStringLiteral("protectTones"), false},
                     {QStringLiteral("spongeMode"), 1},
                     {QStringLiteral("vibrance"), false}}},
        {"sponge",
         QJsonObject{{QStringLiteral("amount"), 62},
                     {QStringLiteral("range"), 2},
                     {QStringLiteral("protectTones"), false},
                     {QStringLiteral("spongeMode"), 1},
                     {QStringLiteral("vibrance"), false}}},
        {"hand", QJsonObject{{QStringLiteral("scrollAll"), true}}},
        {"zoom",
         QJsonObject{{QStringLiteral("resizeToFit"), true},
                     {QStringLiteral("zoomAll"), true}}},
        {"brush",
         QJsonObject{{QStringLiteral("size"), 33},
                     {QStringLiteral("hardness"), 90},
                     {QStringLiteral("opacity"), 90},
                     {QStringLiteral("flow"), 80},
                     {QStringLiteral("mode"), QStringLiteral("multiply")},
                     {QStringLiteral("autoErase"), true},
                     {QStringLiteral("airbrush"), true},
                     {QStringLiteral("pressureSize"), true},
                     {QStringLiteral("pressureOpacity"), true},
                     {QStringLiteral("roundness"), 90},
                     {QStringLiteral("tipAngle"), 15},
                     {QStringLiteral("spacing"), 30},
                     {QStringLiteral("grain"), 10},
                     {QStringLiteral("flipX"), true},
                     {QStringLiteral("flipY"), true},
                     {QStringLiteral("penTip"), true},
                     {QStringLiteral("aliased"), true},
                     {QStringLiteral("wetEdges"), true},
                     {QStringLiteral("scatter"), 10},
                     {QStringLiteral("count"), 2},
                     {QStringLiteral("sizeJitter"), 5},
                     {QStringLiteral("angleJitter"), 10},
                     {QStringLiteral("roundnessJitter"), 15},
                     {QStringLiteral("shapeEnabled"), true},
                     {QStringLiteral("sizeControl"), 1},
                     {QStringLiteral("sizeFade"), 26},
                     {QStringLiteral("minDiameter"), 5},
                     {QStringLiteral("angleControl"), 1},
                     {QStringLiteral("angleFade"), 26},
                     {QStringLiteral("roundnessControl"), 1},
                     {QStringLiteral("roundnessFade"), 26},
                     {QStringLiteral("minRoundness"), 5},
                     {QStringLiteral("tiltScale"), 90},
                     {QStringLiteral("brushProjection"), true},
                     {QStringLiteral("scatterEnabled"), false},
                     {QStringLiteral("scatterControl"), 1},
                     {QStringLiteral("scatterFade"), 26},
                     {QStringLiteral("bothAxes"), false},
                     {QStringLiteral("countControl"), 1},
                     {QStringLiteral("countFade"), 26},
                     {QStringLiteral("countJitter"), 5},
                     {QStringLiteral("colorEnabled"), true},
                     {QStringLiteral("applyPerTip"), true},
                     {QStringLiteral("fgBgJitter"), 5},
                     {QStringLiteral("fgBgControl"), 1},
                     {QStringLiteral("fgBgFade"), 26},
                     {QStringLiteral("hueJitter"), 5},
                     {QStringLiteral("saturationJitter"), 5},
                     {QStringLiteral("brightnessJitter"), 5},
                     {QStringLiteral("purity"), 5},
                     {QStringLiteral("transferEnabled"), true},
                     {QStringLiteral("opacityJitter"), 5},
                     {QStringLiteral("opacityControl"), 1},
                     {QStringLiteral("opacityFade"), 26},
                     {QStringLiteral("flowJitter"), 5},
                     {QStringLiteral("flowControl"), 1},
                     {QStringLiteral("flowFade"), 26},
                     {QStringLiteral("poseEnabled"), true},
                     {QStringLiteral("poseTiltX"), 1},
                     {QStringLiteral("poseTiltY"), 2},
                     {QStringLiteral("poseRotation"), 3},
                     {QStringLiteral("posePressure"), 90},
                     {QStringLiteral("overrideTiltX"), true},
                     {QStringLiteral("overrideTiltY"), true},
                     {QStringLiteral("overrideRotation"), true},
                     {QStringLiteral("overridePressure"), true},
                     {QStringLiteral("lockShape"), true},
                     {QStringLiteral("lockScatter"), true},
                     {QStringLiteral("lockColor"), true},
                     {QStringLiteral("lockTransfer"), true},
                     {QStringLiteral("lockPose"), true},
                     {QStringLiteral("lockTexture"), true},
                     {QStringLiteral("lockDual"), true},
                     {QStringLiteral("bristleEnabled"), true},
                     {QStringLiteral("bristleShape"), 1},
                     {QStringLiteral("bristleDensity"), 60},
                     {QStringLiteral("bristleLength"), 61},
                     {QStringLiteral("bristleThickness"), 62},
                     {QStringLiteral("bristleStiffness"), 63}}},
        {"selection",
         QJsonObject{{QStringLiteral("combineMode"), 1},
                     {QStringLiteral("marqueeStyle"), 1},
                     {QStringLiteral("feather"), 5.0},
                     {QStringLiteral("fixedRatioWidth"), 2.0},
                     {QStringLiteral("fixedRatioHeight"), 1.5},
                     {QStringLiteral("fixedSizeWidth"), 120},
                     {QStringLiteral("fixedSizeHeight"), 130},
                     {QStringLiteral("tolerance"), 40},
                     {QStringLiteral("contiguous"), false},
                     {QStringLiteral("magneticWidth"), 20},
                     {QStringLiteral("magneticContrast"), 20},
                     {QStringLiteral("magneticFrequency"), 60}}},
        {"ellipticalSelection",
         QJsonObject{{QStringLiteral("combineMode"), 2},
                     {QStringLiteral("marqueeStyle"), 2},
                     {QStringLiteral("feather"), 6.0},
                     {QStringLiteral("fixedRatioWidth"), 3.0},
                     {QStringLiteral("fixedRatioHeight"), 2.0},
                     {QStringLiteral("fixedSizeWidth"), 140},
                     {QStringLiteral("fixedSizeHeight"), 90},
                     {QStringLiteral("tolerance"), 50},
                     {QStringLiteral("contiguous"), false},
                     {QStringLiteral("magneticWidth"), 30},
                     {QStringLiteral("magneticContrast"), 30},
                     {QStringLiteral("magneticFrequency"), 70}}},
        {"quickSelection",
         QJsonObject{{QStringLiteral("combineMode"), 3},
                     {QStringLiteral("marqueeStyle"), 1},
                     {QStringLiteral("feather"), 7.0},
                     {QStringLiteral("fixedRatioWidth"), 4.0},
                     {QStringLiteral("fixedRatioHeight"), 3.0},
                     {QStringLiteral("fixedSizeWidth"), 80},
                     {QStringLiteral("fixedSizeHeight"), 60},
                     {QStringLiteral("tolerance"), 60},
                     {QStringLiteral("contiguous"), false},
                     {QStringLiteral("magneticWidth"), 40},
                     {QStringLiteral("magneticContrast"), 40},
                     {QStringLiteral("magneticFrequency"), 80}}},
    };
    return rows;
}

QJsonObject optionDefaults()
{
    pictura::ToolController tools;
    return pictura::toolOptionsToJson(tools);
}

QJsonObject mergedOptionRows()
{
    QJsonObject merged;
    for (const OptionRow& row : optionRows()) {
        merged.insert(optionKey(row), row.values);
    }
    return merged;
}

QString describe(const QJsonValue& value)
{
    switch (value.type()) {
    case QJsonValue::Bool:
        return value.toBool() ? QStringLiteral("true") : QStringLiteral("false");
    case QJsonValue::Double:
        return QString::number(value.toDouble(), 'g', 17);
    case QJsonValue::String:
        return QStringLiteral("\"") + value.toString() + QStringLiteral("\"");
    case QJsonValue::Null:
        return QStringLiteral("null");
    default:
        return QStringLiteral("<non-scalar>");
    }
}

QString jsonDiff(const QJsonObject& actual, const QJsonObject& expected)
{
    if (actual == expected) {
        return {};
    }
    QSet<QString> keys;
    for (const QString& key : actual.keys()) {
        keys.insert(key);
    }
    for (const QString& key : expected.keys()) {
        keys.insert(key);
    }
    QStringList ordered = keys.values();
    ordered.sort();
    QStringList lines;
    for (const QString& key : ordered) {
        const QJsonValue actualValue = actual.value(key);
        const QJsonValue expectedValue = expected.value(key);
        if (actualValue == expectedValue) {
            continue;
        }
        lines << key + QStringLiteral("\n  actual:   ") + describe(actualValue)
                    + QStringLiteral("\n  expected: ") + describe(expectedValue);
    }
    return lines.join(QLatin1Char('\n'));
}

bool isColorField(const QString& field)
{
    return field == QStringLiteral("fillColor") || field == QStringLiteral("strokeColor")
        || field == QStringLiteral("shieldColor");
}

bool isCanonicalColor(const QJsonValue& value)
{
    if (!value.isString()) {
        return false;
    }
    const QString name = value.toString();
    const QColor color = QColor::fromString(name);
    return color.isValid() && color.name(QColor::HexArgb) == name;
}

} // namespace

class ToolOptionsPersistenceTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void optionsSurviveARestart();
    void quickSelectionToleranceFromOptionsBarPersists();
    void quickSelectionToleranceTypedWithoutEnterPersists();
    void badEntriesKeepDefaults();
    void restoreDoesNotNotify();
    void missingFieldsRestoreDefaults();
    void patchMergesAndNotifies();
    void ellipticalOptionsRoundTrip();
    void roundTripTableIsComplete();
    void everyToolOptionsKeyRoundTrips_data();
    void everyToolOptionsKeyRoundTrips();
    void everyToolOptionsKeySurvivesARestart();
    void wrongTypeEntriesRestoreDefaults_data();
    void wrongTypeEntriesRestoreDefaults();
    void groupEntriesRestoreDefaults_data();
    void groupEntriesRestoreDefaults();
    void outOfRangeEntriesClamp_data();
    void outOfRangeEntriesClamp();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ToolOptionsPersistenceTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
}

void ToolOptionsPersistenceTest::optionsSurviveARestart()
{
    using namespace pictura;
    {
        auto window = test::makeMainWindow();
        auto* tools = window->findChild<ToolController*>();
        QVERIFY(tools);
        MoveGroup* move = tools->optionsFor<MoveGroup>(ToolId::Move);
        MoveOptions m = move->get();
        m.showTransformControls = false;
        move->set(m);
        TypeGroup* type = tools->optionsFor<TypeGroup>(ToolId::HorizontalType);
        TypeOptions t = type->get();
        t.size = 48.0;
        type->set(t);
        PaintOptions* paint = tools->optionsFor<PaintOptions>(ToolId::Brush);
        paint->setSize(77);
        paint->setOpacity(40);
        paint->setGrain(75);
        paint->setPenTip(true);
        paint->setAliased(true);
        paint->setWetEdges(true);
        paint->setAirbrush(true);
        paint->setPressureSize(true);
        paint->setPressureOpacity(true);
        tools->optionsFor<SelectionOptions>(ToolId::Marquee)->setFeather(5.5);
        tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->setTolerance(77);
        RetouchGroup* blur = tools->optionsFor<RetouchGroup>(ToolId::Blur);
        RetouchOptions r = blur->get();
        r.strength = 80;
        blur->set(r);
        ToneGroup* burn = tools->optionsFor<ToneGroup>(ToolId::Burn);
        ToneOptions b = burn->get();
        b.amount = 20;
        burn->set(b);
        window->saveSession();
    }

    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    QVERIFY(tools);
    QVERIFY(!tools->optionsFor<MoveGroup>(ToolId::Move)->get().showTransformControls);
    // The options bar starts from the restored values.
    auto* transform = window->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    QVERIFY(transform && !transform->isChecked());
    QCOMPARE(tools->optionsFor<TypeGroup>(ToolId::HorizontalType)->get().size, 48.0);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->size(), 77);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->opacity(), 40);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->grain(), 75);
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->penTip());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->aliased());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->wetEdges());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->airbrush());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->pressureSize());
    QVERIFY(tools->optionsFor<PaintOptions>(ToolId::Brush)->pressureOpacity());
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::Marquee)->feather(), 5.5);
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 77);
    QCOMPARE(tools->optionsFor<RetouchGroup>(ToolId::Blur)->get().strength, 80);
    QCOMPARE(tools->optionsFor<ToneGroup>(ToolId::Burn)->get().amount, 20);
    // Each retouch and toning tool keeps its own options.
    QCOMPARE(tools->optionsFor<RetouchGroup>(ToolId::Sharpen)->get().strength,
             RetouchOptions{}.strength);
    QCOMPARE(tools->optionsFor<ToneGroup>(ToolId::Dodge)->get().amount, ToneOptions{}.amount);
}

void ToolOptionsPersistenceTest::quickSelectionToleranceFromOptionsBarPersists()
{
    using namespace pictura;
    {
        auto window = test::makeMainWindow();
        window->show();
        auto* tools = window->findChild<ToolController*>();
        auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
        QVERIFY(tools && bar);
        window->setActiveTool(ToolId::QuickSelection);
        QWidget* page = bar->pageForTest(ToolId::QuickSelection);
        QVERIFY(page);
        auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
        QVERIFY(spin);
        auto* edit = spin->findChild<QLineEdit*>();
        QVERIFY(edit);
        edit->setFocus();
        edit->selectAll();
        QTest::keyClicks(edit, QStringLiteral("77"));
        QTest::keyClick(edit, Qt::Key_Return);
        QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 77);
        window->saveSession();
    }

    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(tools && bar);
    window->setActiveTool(ToolId::QuickSelection);
    QWidget* page = bar->pageForTest(ToolId::QuickSelection);
    QVERIFY(page);
    auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
    QVERIFY(spin);
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 77);
    QCOMPARE(spin->value(), 77.0);
}

void ToolOptionsPersistenceTest::quickSelectionToleranceTypedWithoutEnterPersists()
{
    using namespace pictura;
    {
        auto window = test::makeMainWindow();
        window->show();
        auto* tools = window->findChild<ToolController*>();
        auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
        QVERIFY(tools && bar);
        window->setActiveTool(ToolId::QuickSelection);
        QWidget* page = bar->pageForTest(ToolId::QuickSelection);
        QVERIFY(page);
        auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
        QVERIFY(spin);
        auto* edit = spin->findChild<QLineEdit*>();
        QVERIFY(edit);
        edit->setFocus();
        edit->selectAll();
        QTest::keyClicks(edit, QStringLiteral("88"));
        window->close();
    }

    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    auto* bar = window->findChild<OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(tools && bar);
    window->setActiveTool(ToolId::QuickSelection);
    QWidget* page = bar->pageForTest(ToolId::QuickSelection);
    QVERIFY(page);
    auto* spin = page->findChild<NumericField*>(QStringLiteral("optionsTolerance"));
    QVERIFY(spin);
    QCOMPARE(tools->optionsFor<SelectionOptions>(ToolId::QuickSelection)->tolerance(), 88);
    QCOMPARE(spin->value(), 88.0);
}

void ToolOptionsPersistenceTest::badEntriesKeepDefaults()
{
    using namespace pictura;
    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    QVERIFY(tools);
    const int brushDefault = PaintOptions{}.size();
    QJsonObject json;
    json.insert(QStringLiteral("type"), QJsonObject{{QStringLiteral("size"), 99999.0}});
    json.insert(QStringLiteral("brush"), 5);
    json.insert(QStringLiteral("eraser"), QJsonObject{{QStringLiteral("mode"), QStringLiteral("x")}});
    json.insert(QStringLiteral("unknownTool"), QJsonObject{{QStringLiteral("a"), 1}});
    applyToolOptionsJson(*tools, json);
    QCOMPARE(tools->optionsFor<TypeGroup>(ToolId::HorizontalType)->get().size, 1296.0);
    QCOMPARE(tools->optionsFor<PaintOptions>(ToolId::Brush)->size(), brushDefault);
    QCOMPARE(tools->optionsFor<EraserGroup>(ToolId::Eraser)->get().mode, EraserOptions{}.mode);
}

void ToolOptionsPersistenceTest::restoreDoesNotNotify()
{
    using namespace pictura;
    ToolController tools;
    PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush);
    SelectionOptions* marquee = tools.optionsFor<SelectionOptions>(ToolId::Marquee);
    SelectionOptions* quick = tools.optionsFor<SelectionOptions>(ToolId::QuickSelection);
    CropOptions* crop = tools.optionsFor<CropOptions>(ToolId::Crop);
    QVERIFY(paint && marquee && quick && crop);

    int brushSignals = 0;
    paint->sizeChanged = [&brushSignals](int) { ++brushSignals; };
    paint->tipChanged = [&brushSignals]() { ++brushSignals; };
    paint->paintSettingsChanged = [&brushSignals]() { ++brushSignals; };
    int selectionSignals = 0;
    marquee->combineModeChanged = [&selectionSignals](SelectionMode) { ++selectionSignals; };
    marquee->magneticWidthChanged = [&selectionSignals](int) { ++selectionSignals; };
    quick->combineModeChanged = [&selectionSignals](SelectionMode) { ++selectionSignals; };
    quick->magneticWidthChanged = [&selectionSignals](int) { ++selectionSignals; };
    int cropSignals = 0;
    crop->optionsChanged = [&cropSignals]() { ++cropSignals; };

    applyToolOptionsJson(
        tools,
        QJsonObject{
            {QStringLiteral("brush"),
             QJsonObject{{QStringLiteral("size"), 99},
                         {QStringLiteral("hardness"), 10},
                         {QStringLiteral("opacity"), 50},
                         {QStringLiteral("flow"), 60},
                         {QStringLiteral("mode"), QStringLiteral("multiply")},
                         {QStringLiteral("roundness"), 40},
                         {QStringLiteral("tipAngle"), 30},
                         {QStringLiteral("spacing"), 10},
                         {QStringLiteral("grain"), 20},
                         {QStringLiteral("flipX"), true},
                         {QStringLiteral("flipY"), true},
                         {QStringLiteral("penTip"), true},
                         {QStringLiteral("aliased"), true},
                         {QStringLiteral("wetEdges"), true},
                         {QStringLiteral("airbrush"), true},
                         {QStringLiteral("pressureSize"), true},
                         {QStringLiteral("pressureOpacity"), true},
                         {QStringLiteral("scatter"), 10},
                         {QStringLiteral("count"), 3},
                         {QStringLiteral("sizeJitter"), 10},
                         {QStringLiteral("angleJitter"), 20},
                         {QStringLiteral("roundnessJitter"), 30}}},
            {QStringLiteral("selection"),
             QJsonObject{{QStringLiteral("combineMode"), 1},
                         {QStringLiteral("magneticWidth"), 40}}},
            {QStringLiteral("quickSelection"),
             QJsonObject{{QStringLiteral("combineMode"), 2},
                         {QStringLiteral("magneticWidth"), 50}}},
            {QStringLiteral("crop"),
             QJsonObject{{QStringLiteral("shieldOpacity"), 40},
                         {QStringLiteral("overlayShow"), 1}}}});

    QCOMPARE(brushSignals, 0);
    QCOMPARE(selectionSignals, 0);
    QCOMPARE(cropSignals, 0);
    QCOMPARE(paint->size(), 99);
    QCOMPARE(paint->hardness(), 10);
    QCOMPARE(paint->opacity(), 50);
    QCOMPARE(paint->flow(), 60);
    QCOMPARE(paint->mode(), QStringLiteral("multiply"));
    QVERIFY(paint->airbrush());
    QVERIFY(paint->pressureSize());
    QVERIFY(paint->pressureOpacity());
    QCOMPARE(int(marquee->combineMode()), 1);
    QCOMPARE(marquee->magneticWidth(), 40);
    QCOMPARE(int(quick->combineMode()), 2);
    QCOMPARE(quick->magneticWidth(), 50);
    QCOMPARE(crop->settings().shieldOpacity, 40);
    QCOMPARE(crop->settings().overlayShow, int(CropSettings::AlwaysShow));
}

void ToolOptionsPersistenceTest::missingFieldsRestoreDefaults()
{
    using namespace pictura;
    ToolController tools;
    PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush);
    SelectionOptions* marquee = tools.optionsFor<SelectionOptions>(ToolId::Marquee);
    SelectionOptions* ellipse = tools.optionsFor<SelectionOptions>(ToolId::EllipticalMarquee);
    MoveGroup* move = tools.optionsFor<MoveGroup>(ToolId::Move);
    QVERIFY(paint && marquee && ellipse && move);

    paint->setSize(77);
    paint->setHardness(10);
    paint->setOpacity(40);
    marquee->setTolerance(200);
    marquee->setMagneticWidth(40);
    ellipse->setMarqueeStyle(MarqueeStyle::FixedSize);
    ellipse->setFixedRatio(3.0, 2.0);
    ellipse->setFixedSize(20, 10);
    MoveOptions options = move->get();
    options.autoSelectOn = true;
    move->set(options);

    applyToolOptionsJson(
        tools,
        QJsonObject{
            {QStringLiteral("brush"), QJsonObject{{QStringLiteral("size"), 33}}},
            {QStringLiteral("selection"), QJsonObject{{QStringLiteral("tolerance"), 100}}},
            {QStringLiteral("move"), QJsonObject{{QStringLiteral("alignTo"), 1}}}});

    QCOMPARE(paint->size(), 33);
    QCOMPARE(paint->hardness(), PaintOptions{}.hardness());
    QCOMPARE(paint->opacity(), PaintOptions{}.opacity());
    QCOMPARE(marquee->tolerance(), 100);
    QCOMPARE(marquee->magneticWidth(), SelectionOptions{}.magneticWidth());
    QCOMPARE(int(ellipse->marqueeStyle()), int(MarqueeStyle::Normal));
    QCOMPARE(ellipse->fixedRatioWidth(), SelectionOptions{}.fixedRatioWidth());
    QCOMPARE(ellipse->fixedRatioHeight(), SelectionOptions{}.fixedRatioHeight());
    QCOMPARE(ellipse->fixedSizeWidth(), SelectionOptions{}.fixedSizeWidth());
    QCOMPARE(ellipse->fixedSizeHeight(), SelectionOptions{}.fixedSizeHeight());
    QVERIFY(!move->get().autoSelectOn);
    QCOMPARE(move->get().alignTo, 1);
}

void ToolOptionsPersistenceTest::patchMergesAndNotifies()
{
    using namespace pictura;
    ToolController tools;
    PaintOptions* paint = tools.optionsFor<PaintOptions>(ToolId::Brush);
    QVERIFY(paint);
    paint->setSize(77);
    paint->setOpacity(40);

    int notified = 0;
    paint->sizeChanged = [&notified](int) { ++notified; };
    paint->paintSettingsChanged = [&notified]() { ++notified; };

    patchToolOptionsJson(tools,
                         QJsonObject{{QStringLiteral("brush"),
                                      QJsonObject{{QStringLiteral("size"), 123}}}});
    QCOMPARE(paint->size(), 123);
    QCOMPARE(paint->opacity(), 40);
    QCOMPARE(notified, 1);
}


void ToolOptionsPersistenceTest::ellipticalOptionsRoundTrip()
{
    using namespace pictura;
    ToolController tools;
    SelectionOptions* rect = tools.optionsFor<SelectionOptions>(ToolId::Marquee);
    SelectionOptions* ellipse = tools.optionsFor<SelectionOptions>(ToolId::EllipticalMarquee);
    QVERIFY(rect && ellipse);
    rect->setMarqueeStyle(MarqueeStyle::FixedSize);
    rect->setFixedSize(100, 50);
    ellipse->setMarqueeStyle(MarqueeStyle::FixedRatio);
    ellipse->setFixedRatio(2.0, 1.0);

    const QJsonObject json = toolOptionsToJson(tools);

    ToolController restored;
    applyToolOptionsJson(restored, json);
    SelectionOptions* restoredRect = restored.optionsFor<SelectionOptions>(ToolId::Marquee);
    SelectionOptions* restoredEllipse =
        restored.optionsFor<SelectionOptions>(ToolId::EllipticalMarquee);
    QVERIFY(restoredRect && restoredEllipse);
    QCOMPARE(int(restoredRect->marqueeStyle()), int(MarqueeStyle::FixedSize));
    QCOMPARE(restoredRect->fixedSizeWidth(), 100);
    QCOMPARE(int(restoredEllipse->marqueeStyle()), int(MarqueeStyle::FixedRatio));
    QCOMPARE(restoredEllipse->fixedRatioWidth(), 2.0);
}

void ToolOptionsPersistenceTest::roundTripTableIsComplete()
{
    const QJsonObject defaults = optionDefaults();
    pictura::ToolController appliedController;
    pictura::applyToolOptionsJson(appliedController, defaults);
    const QJsonObject applied = pictura::toolOptionsToJson(appliedController);
    const QString idempotence = jsonDiff(applied, defaults);
    QVERIFY2(idempotence.isEmpty(),
             qPrintable(QStringLiteral("defaults do not round-trip: ") + idempotence));

    QStringList emittedKeys = defaults.keys();
    emittedKeys.sort();
    QSet<QString> seen;
    QStringList rowKeys;
    for (const OptionRow& row : optionRows()) {
        const QString key = optionKey(row);
        QVERIFY2(!seen.contains(key), qPrintable(QStringLiteral("duplicate row: ") + key));
        seen.insert(key);
        rowKeys << key;
    }
    rowKeys.sort();
    QVERIFY2(rowKeys == emittedKeys,
             qPrintable(QStringLiteral("rows: ") + rowKeys.join(QStringLiteral(", "))
                        + QStringLiteral("\nemitted: ") + emittedKeys.join(QStringLiteral(", "))));

    for (const OptionRow& row : optionRows()) {
        const QString key = optionKey(row);
        const QJsonObject group = defaults.value(key).toObject();
        QStringList actualFields = row.values.keys();
        actualFields.sort();
        QStringList expectedFields = group.keys();
        expectedFields.sort();
        QVERIFY2(actualFields == expectedFields,
                 qPrintable(key + QStringLiteral("\nrow: ") + actualFields.join(QStringLiteral(", "))
                            + QStringLiteral("\nemitted: ")
                            + expectedFields.join(QStringLiteral(", "))));
        for (const QString& field : actualFields) {
            const QJsonValue value = row.values.value(field);
            QVERIFY2(value != group.value(field),
                     qPrintable(key + QLatin1Char('/') + field
                                + QStringLiteral(": row value equals the default")));
            if (isColorField(field)) {
                QVERIFY2(isCanonicalColor(value),
                         qPrintable(key + QLatin1Char('/') + field
                                    + QStringLiteral(": colour is not canonical HexArgb")));
            }
        }
    }
}

void ToolOptionsPersistenceTest::everyToolOptionsKeyRoundTrips_data()
{
    QTest::addColumn<QString>("key");
    QTest::addColumn<QJsonObject>("values");
    for (const OptionRow& row : optionRows()) {
        QTest::newRow(row.key) << optionKey(row) << row.values;
    }
}

void ToolOptionsPersistenceTest::everyToolOptionsKeyRoundTrips()
{
    QFETCH(QString, key);
    QFETCH(QJsonObject, values);

    pictura::ToolController source;
    pictura::applyToolOptionsJson(source, QJsonObject{{key, values}});
    const QJsonObject snapshot = pictura::toolOptionsToJson(source);
    const QJsonObject sourceGroup = snapshot.value(key).toObject();
    const QString sourceDiff = jsonDiff(sourceGroup, values);
    QVERIFY2(sourceDiff.isEmpty(), qPrintable(key + QStringLiteral(": ") + sourceDiff));

    pictura::ToolController restored;
    pictura::applyToolOptionsJson(restored, snapshot);
    const QJsonObject restoredGroup = pictura::toolOptionsToJson(restored).value(key).toObject();
    const QString restoredDiff = jsonDiff(restoredGroup, values);
    QVERIFY2(restoredDiff.isEmpty(), qPrintable(key + QStringLiteral(": ") + restoredDiff));
}

void ToolOptionsPersistenceTest::everyToolOptionsKeySurvivesARestart()
{
    using namespace pictura;

    struct ResetStore {
        ~ResetStore()
        {
            auto window = pictura::test::makeMainWindow();
            auto* tools = window->findChild<pictura::ToolController*>();
            if (tools) {
                pictura::applyToolOptionsJson(*tools, QJsonObject{});
                tools->applyCropOptions(false, true, 0.0, 0);
                window->saveSession();
            }
        }
    } resetStore;

    const QJsonObject expected = mergedOptionRows();
    {
        auto window = test::makeMainWindow();
        auto* tools = window->findChild<ToolController*>();
        QVERIFY(tools);
        applyToolOptionsJson(*tools, expected);
        CropOptions* crop = tools->optionsFor<CropOptions>(ToolId::Crop);
        QVERIFY(crop);
        crop->setClassicMode(true);
        crop->setDeletePixels(false);
        crop->setRatio(1.5);
        crop->setGridOverlay(2);
        window->saveSession();
    }

    auto window = test::makeMainWindow();
    auto* tools = window->findChild<ToolController*>();
    QVERIFY(tools);
    const QJsonObject restored = toolOptionsToJson(*tools);
    const QString diff = jsonDiff(restored, expected);
    QVERIFY2(diff.isEmpty(), qPrintable(diff));

    CropOptions* crop = tools->optionsFor<CropOptions>(ToolId::Crop);
    QVERIFY(crop);
    QVERIFY(crop->classicMode());
    QVERIFY(!crop->deletePixels());
    QCOMPARE(crop->ratio(), 1.5);
    QCOMPARE(crop->gridOverlay(), 2);
}

void ToolOptionsPersistenceTest::wrongTypeEntriesRestoreDefaults_data()
{
    QTest::addColumn<QString>("key");
    QTest::addColumn<QJsonObject>("payload");
    QTest::addColumn<QJsonObject>("expected");

    const QJsonObject base = optionDefaults();
    const QJsonObject seeded = mergedOptionRows();
    for (auto it = base.begin(); it != base.end(); ++it) {
        const QString key = it.key();
        const QJsonObject group = it.value().toObject();
        QStringList fields = group.keys();
        fields.sort();
        for (const QString& field : fields) {
            const QJsonValue value = group.value(field);
            QJsonValue wrong;
            if (value.isBool()) {
                wrong = 7;
            } else if (value.isDouble()) {
                wrong = QStringLiteral("bad");
            } else if (value.isString()) {
                wrong = 7;
            } else {
                continue;
            }

            if (!(key == QStringLiteral("shape") && field == QStringLiteral("fillColor"))) {
                const QJsonObject seededGroup = seeded.value(key).toObject();
                QJsonObject groupCopy = seededGroup;
                groupCopy.insert(field, wrong);
                QJsonObject payload = seeded;
                payload.insert(key, groupCopy);
                QJsonObject expected = seededGroup;
                expected.insert(field, isColorField(field) ? QJsonValue(QString())
                                                           : group.value(field));
                QTest::newRow(
                    qPrintable(key + QLatin1Char('/') + field + QStringLiteral("/wrong-type")))
                    << key << payload << expected;
            }

            const QJsonObject seededGroup = seeded.value(key).toObject();
            QJsonObject groupCopy = seededGroup;
            groupCopy.insert(field, QJsonValue(QJsonValue::Null));
            QJsonObject payload = seeded;
            payload.insert(key, groupCopy);
            QJsonObject expected = seededGroup;
            expected.insert(field, group.value(field));
            QTest::newRow(qPrintable(key + QLatin1Char('/') + field + QStringLiteral("/null")))
                << key << payload << expected;
        }

        for (const QString& field : fields) {
            if (!isColorField(field)
                || (key == QStringLiteral("shape") && field == QStringLiteral("fillColor"))) {
                continue;
            }
            const QJsonObject seededGroup = seeded.value(key).toObject();
            for (const QString& malformed :
                 {QStringLiteral("nope"), QStringLiteral("#12345")}) {
                QJsonObject groupCopy = seededGroup;
                groupCopy.insert(field, malformed);
                QJsonObject payload = seeded;
                payload.insert(key, groupCopy);
                QJsonObject expected = seededGroup;
                expected.insert(field, QString());
                QTest::newRow(qPrintable(key + QLatin1Char('/') + field + QLatin1Char('/')
                                         + malformed))
                    << key << payload << expected;
            }
        }
    }
}

void ToolOptionsPersistenceTest::wrongTypeEntriesRestoreDefaults()
{
    QFETCH(QString, key);
    QFETCH(QJsonObject, payload);
    QFETCH(QJsonObject, expected);

    pictura::ToolController tools;
    pictura::applyToolOptionsJson(tools, payload);
    const QJsonObject actual = pictura::toolOptionsToJson(tools).value(key).toObject();
    const QString diff = jsonDiff(actual, expected);
    QVERIFY2(diff.isEmpty(), qPrintable(key + QStringLiteral(": ") + diff));
}

void ToolOptionsPersistenceTest::groupEntriesRestoreDefaults_data()
{
    QTest::addColumn<QString>("key");
    QTest::addColumn<QJsonObject>("values");
    for (const OptionRow& row : optionRows()) {
        QTest::newRow(row.key) << optionKey(row) << row.values;
    }
}

void ToolOptionsPersistenceTest::groupEntriesRestoreDefaults()
{
    QFETCH(QString, key);
    QFETCH(QJsonObject, values);

    const QJsonObject expected = optionDefaults().value(key).toObject();
    const QJsonObject badPayloads[] = {
        QJsonObject{{key, 5}},
        QJsonObject{{key, QJsonValue(QJsonValue::Null)}},
        QJsonObject{},
    };
    for (const QJsonObject& bad : badPayloads) {
        pictura::ToolController tools;
        pictura::applyToolOptionsJson(tools, QJsonObject{{key, values}});
        pictura::applyToolOptionsJson(tools, bad);
        const QJsonObject actual = pictura::toolOptionsToJson(tools).value(key).toObject();
        const QString diff = jsonDiff(actual, expected);
        QVERIFY2(diff.isEmpty(), qPrintable(key + QStringLiteral(": ") + diff));
    }
}

void ToolOptionsPersistenceTest::outOfRangeEntriesClamp_data()
{
    QTest::addColumn<QString>("key");
    QTest::addColumn<QString>("field");
    QTest::addColumn<QJsonValue>("bad");
    QTest::addColumn<QJsonValue>("expected");
    const auto add = [](const QString& key, const QString& field, const QJsonValue& bad,
                        const QJsonValue& expected) {
        QTest::newRow(qPrintable(key + QLatin1Char('/') + field + QLatin1Char('=')
                                 + describe(bad)))
            << key << field << bad << expected;
    };

    add(QStringLiteral("type"), QStringLiteral("size"), 99999.0, 1296.0);
    add(QStringLiteral("type"), QStringLiteral("justification"), 99, 2);
    add(QStringLiteral("shape"), QStringLiteral("radius"), -5.0, 0.0);
    add(QStringLiteral("shape"), QStringLiteral("sides"), 0, 3);
    add(QStringLiteral("shape"), QStringLiteral("weight"), 5000.0, 1000.0);
    add(QStringLiteral("shape"), QStringLiteral("arrowWidth"), 1.0, 10.0);
    add(QStringLiteral("shape"), QStringLiteral("arrowLength"), 1.0, 10.0);
    add(QStringLiteral("shape"), QStringLiteral("arrowConcavity"), -100.0, -50.0);
    add(QStringLiteral("shape"), QStringLiteral("strokeWidth"), 999.0, 250.0);
    add(QStringLiteral("shape"), QStringLiteral("strokeAlign"), 9, 2);
    add(QStringLiteral("shape"), QStringLiteral("geometry"), 9, 2);
    add(QStringLiteral("shape"), QStringLiteral("indent"), 0.0, 1.0);
    add(QStringLiteral("shape"), QStringLiteral("fixedWidth"), 0.0, 0.01);
    add(QStringLiteral("shape"), QStringLiteral("fixedHeight"), -5.0, 0.01);
    add(QStringLiteral("pen"), QStringLiteral("curveFit"), 100.0, 10.0);
    add(QStringLiteral("eyedropper"), QStringLiteral("sampleSize"), 4, 5);
    add(QStringLiteral("eyedropper"), QStringLiteral("sampleSize"), 999, 101);
    add(QStringLiteral("eyedropper"), QStringLiteral("scope"), 9, 4);
    add(QStringLiteral("selection"), QStringLiteral("feather"), 999.0, 250.0);
    add(QStringLiteral("selection"), QStringLiteral("fixedSizeWidth"), -5, 1);
    add(QStringLiteral("selection"), QStringLiteral("fixedSizeHeight"), 0, 1);
    add(QStringLiteral("selection"), QStringLiteral("tolerance"), 999, 255);
    add(QStringLiteral("selection"), QStringLiteral("magneticWidth"), 999, 256);
    add(QStringLiteral("selection"), QStringLiteral("magneticContrast"), 999, 100);
    add(QStringLiteral("selection"), QStringLiteral("magneticFrequency"), 999, 100);
    add(QStringLiteral("selection"), QStringLiteral("combineMode"), 9, 3);
    add(QStringLiteral("selection"), QStringLiteral("marqueeStyle"), 9, 2);
    add(QStringLiteral("healing"), QStringLiteral("spotType"), 9, 2);
    add(QStringLiteral("contentAwareMove"), QStringLiteral("adaptation"), 9, 4);
    add(QStringLiteral("redEye"), QStringLiteral("pupil"), 200, 100);
    add(QStringLiteral("redEye"), QStringLiteral("darken"), -5, 0);
    add(QStringLiteral("move"), QStringLiteral("autoSelect"), 7, 0);
    add(QStringLiteral("move"), QStringLiteral("alignTo"), 7, 0);
    add(QStringLiteral("brush"), QStringLiteral("size"), 99999, 5000);
    add(QStringLiteral("brush"), QStringLiteral("hardness"), 999, 100);
    add(QStringLiteral("brush"), QStringLiteral("opacity"), 999, 100);
    add(QStringLiteral("brush"), QStringLiteral("roundness"), 999, 100);
    add(QStringLiteral("brush"), QStringLiteral("spacing"), 0, 1);
    add(QStringLiteral("brush"), QStringLiteral("tipAngle"), 999, 180);
    add(QStringLiteral("brush"), QStringLiteral("bristleShape"), 9, 5);
    add(QStringLiteral("brush"), QStringLiteral("scatter"), 99999, 1000);
    add(QStringLiteral("brush"), QStringLiteral("count"), 99, 16);
    add(QStringLiteral("brush"), QStringLiteral("angleJitter"), 999, 180);
    add(QStringLiteral("brush"), QStringLiteral("sizeFade"), 0, 1);
    add(QStringLiteral("brush"), QStringLiteral("purity"), -999, -100);
    add(QStringLiteral("brush"), QStringLiteral("poseTiltX"), 999, 90);
    add(QStringLiteral("brush"), QStringLiteral("poseRotation"), 999, 279);
    add(QStringLiteral("crop"), QStringLiteral("shieldOpacity"), 999, 100);
    add(QStringLiteral("crop"), QStringLiteral("overlayShow"), 9, 2);
    add(QStringLiteral("crop"), QStringLiteral("overlayOrientation"), 9, 3);
}

void ToolOptionsPersistenceTest::outOfRangeEntriesClamp()
{
    QFETCH(QString, key);
    QFETCH(QString, field);
    QFETCH(QJsonValue, bad);
    QFETCH(QJsonValue, expected);

    pictura::ToolController tools;
    pictura::applyToolOptionsJson(tools, QJsonObject{{key, QJsonObject{{field, bad}}}});
    const QJsonValue actual =
        pictura::toolOptionsToJson(tools).value(key).toObject().value(field);
    QVERIFY2(actual == expected,
             qPrintable(key + QLatin1Char('/') + field + QStringLiteral(": actual ")
                        + describe(actual) + QStringLiteral(", expected ")
                        + describe(expected)));
}

QTEST_MAIN(ToolOptionsPersistenceTest)
#include "tst_tool_options_persistence.moc"
