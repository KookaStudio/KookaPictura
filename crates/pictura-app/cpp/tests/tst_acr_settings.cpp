#include <QtTest/QtTest>

#include "acr/acr_settings.h"

using namespace pictura;

// The same literal is committed in the Rust `text::tests::committed_bridge_fixture_parses`;
// keep the two in step.
static const char kFixture[] =
    "Temp=20;Ex12=1.5;Dhze=7;Crv =0,0,128,140,255,255;CamP=Pictura Standard\\x00";

class AcrSettingsTest : public QObject {
    Q_OBJECT

private slots:
    void parsesBridgeFixture();
    void roundTripsThroughEncode();
    void roundTripsEscapedSeparators();
    void preservesUnknownKeys();
};

void AcrSettingsTest::parsesBridgeFixture()
{
    const AcrSettings settings = AcrSettings::decode(QString::fromUtf8(kFixture));
    QCOMPARE(settings.intValue(acr::Temp), 20);
    QCOMPARE(settings.doubleValue(acr::Exposure), 1.5);
    QCOMPARE(settings.intValue(QByteArrayLiteral("Dhze")), 7);
    QCOMPARE(settings.curveValue(acr::CurveRgb), QList<int>({0, 0, 128, 140, 255, 255}));
    QCOMPARE(settings.raw(acr::CameraProfile), QByteArray("Pictura Standard\0", 17));
}

void AcrSettingsTest::roundTripsThroughEncode()
{
    const AcrSettings settings = AcrSettings::decode(QString::fromUtf8(kFixture));
    const AcrSettings again = AcrSettings::decode(settings.encode());
    QCOMPARE(again.raw(acr::Temp), QByteArrayLiteral("20"));
    QCOMPARE(again.curveValue(acr::CurveRgb), settings.curveValue(acr::CurveRgb));
    QCOMPARE(again.raw(acr::CameraProfile), settings.raw(acr::CameraProfile));
    // A trailing-space key survives the round trip.
    QVERIFY(again.raw(acr::CurveRgb) == settings.raw(acr::CurveRgb));
}

void AcrSettingsTest::roundTripsEscapedSeparators()
{
    AcrSettings settings;
    settings.setRaw(QByteArrayLiteral("GuUr"), QByteArrayLiteral("a;b=c\\d"));
    settings.setRaw(QByteArrayLiteral("CMod"), QByteArrayLiteral("x\ny"));
    const QString text = settings.encode();
    // The separators are escaped, so the pair does not split.
    QVERIFY2(text.contains(QStringLiteral("GuUr=a\\;b\\=c\\\\d")), qPrintable(text));
    QVERIFY2(text.contains(QStringLiteral("CMod=x\\x0ay")), qPrintable(text));
    const AcrSettings again = AcrSettings::decode(text);
    QCOMPARE(again.raw(QByteArrayLiteral("GuUr")), QByteArrayLiteral("a;b=c\\d"));
    QCOMPARE(again.raw(QByteArrayLiteral("CMod")), QByteArrayLiteral("x\ny"));
}

void AcrSettingsTest::preservesUnknownKeys()
{
    AcrSettings settings = AcrSettings::decode(QString::fromUtf8(kFixture));
    // A key the dialog has no control for is carried through untouched.
    settings.setRaw(QByteArrayLiteral("Upri"), QByteArrayLiteral("opaque"));
    settings.setInt(acr::Clarity, 5);
    const QString out = settings.encode();
    QVERIFY(out.contains(QStringLiteral("Upri=opaque")));
    QVERIFY(out.contains(QStringLiteral("Cl12=5")));
    QVERIFY(out.contains(QStringLiteral("Dhze=7")));
    QVERIFY(out.contains(QStringLiteral("Crv =0,0,128,140,255,255")));
}

QTEST_MAIN(AcrSettingsTest)
#include "tst_acr_settings.moc"
