#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QPixmap>

#include "icons.h"
#include "theme.h"

class IconAssetsTest : public QObject {
    Q_OBJECT

private slots:
    void panelGlyphsResolve();
    void pathGlyphsResolve();
    void infoGlyphsResolve();
    void adjustmentsGlyphsResolve();
    void actionsGlyphsResolve();
    void unknownIdIsNull();
    void explicitColour();
    void disabledDiffers();
    void hidpiRenders();
    void themeChangeRerenders();
};

void IconAssetsTest::panelGlyphsResolve()
{
    const QStringList ids = {
        QStringLiteral("layers.search"),
        QStringLiteral("layers.kindShape"),
        QStringLiteral("layers.kindSmartObject"),
        QStringLiteral("layers.reset"),
    };
    for (const QString& id : ids) {
        const QIcon icon = pictura::icon(id);
        QVERIFY2(!icon.isNull(), qPrintable(id + QStringLiteral(" resolves")));
        QVERIFY2(!icon.pixmap(16, 16).isNull(),
                 qPrintable(id + QStringLiteral(" renders at 16px")));
    }
}

void IconAssetsTest::pathGlyphsResolve()
{
    const QStringList ids = {
        QStringLiteral("path.thumbnail"),
        QStringLiteral("path.fill"),
        QStringLiteral("path.stroke"),
        QStringLiteral("path.loadSelection"),
        QStringLiteral("path.makeWorkPath"),
        QStringLiteral("path.newPath"),
        QStringLiteral("path.delete"),
    };
    for (const QString& id : ids) {
        const QIcon icon = pictura::icon(id);
        QVERIFY2(!icon.isNull(), qPrintable(id + QStringLiteral(" resolves")));
        QVERIFY2(!icon.pixmap(16, 16).isNull(),
                 qPrintable(id + QStringLiteral(" renders at 16px")));
    }
}

void IconAssetsTest::actionsGlyphsResolve()
{
    const QStringList ids = {
        QStringLiteral("actions.record"),      QStringLiteral("actions.play"),
        QStringLiteral("actions.stop"),        QStringLiteral("actions.newSet"),
        QStringLiteral("actions.newAction"),   QStringLiteral("actions.delete"),
        QStringLiteral("actions.modalOn"),     QStringLiteral("actions.modalOff"),
        QStringLiteral("actions.modalPartial"),
    };
    for (const QString& id : ids) {
        const QIcon icon = pictura::icon(id);
        QVERIFY2(!icon.isNull(), qPrintable(id + QStringLiteral(" resolves")));
        QVERIFY2(!icon.pixmap(16, 16).isNull(),
                 qPrintable(id + QStringLiteral(" renders at 16px")));
    }
}

void IconAssetsTest::infoGlyphsResolve()
{
    const QStringList ids = {
        QStringLiteral("info.crosshair"),
        QStringLiteral("info.bounds"),
        QStringLiteral("info.protractor"),
    };
    for (const QString& id : ids) {
        const QIcon icon = pictura::icon(id);
        QVERIFY2(!icon.isNull(), qPrintable(id + QStringLiteral(" resolves")));
        QVERIFY2(!icon.pixmap(16, 16).isNull(),
                 qPrintable(id + QStringLiteral(" renders at 16px")));
    }
}

void IconAssetsTest::adjustmentsGlyphsResolve()
{
    const QStringList ids = {
        QStringLiteral("adjustments.solid-color"),
        QStringLiteral("adjustments.gradient"),
        QStringLiteral("adjustments.pattern"),
        QStringLiteral("adjustments.brightness-contrast"),
        QStringLiteral("adjustments.levels"),
        QStringLiteral("adjustments.curves"),
        QStringLiteral("adjustments.exposure"),
        QStringLiteral("adjustments.vibrance"),
        QStringLiteral("adjustments.hue-saturation"),
        QStringLiteral("adjustments.color-balance"),
        QStringLiteral("adjustments.black-white"),
        QStringLiteral("adjustments.photo-filter"),
        QStringLiteral("adjustments.channel-mixer"),
        QStringLiteral("adjustments.color-lookup"),
        QStringLiteral("adjustments.invert"),
        QStringLiteral("adjustments.posterize"),
        QStringLiteral("adjustments.threshold"),
        QStringLiteral("adjustments.gradient-map"),
        QStringLiteral("adjustments.selective-color"),
    };
    for (const QString& id : ids) {
        const QIcon icon = pictura::icon(id);
        QVERIFY2(!icon.isNull(), qPrintable(id + QStringLiteral(" resolves")));
        QVERIFY2(!icon.pixmap(16, 16).isNull(),
                 qPrintable(id + QStringLiteral(" renders at 16px")));
    }
}

void IconAssetsTest::unknownIdIsNull()
{
    QVERIFY(pictura::icon(QStringLiteral("no.such.icon")).isNull());
}

void IconAssetsTest::explicitColour()
{
    const QImage img = pictura::icon(QStringLiteral("tool.brush"), QColor(255, 0, 0))
                           .pixmap(48, 48)
                           .toImage()
                           .convertToFormat(QImage::Format_ARGB32);
    QVERIFY(!img.isNull());

    int maxAlpha = 0;
    QRgb strongest = 0;
    for (int y = 0; y < img.height(); ++y) {
        const QRgb* line = reinterpret_cast<const QRgb*>(img.constScanLine(y));
        for (int x = 0; x < img.width(); ++x) {
            if (qAlpha(line[x]) > maxAlpha) {
                maxAlpha = qAlpha(line[x]);
                strongest = line[x];
            }
        }
    }
    QVERIFY2(maxAlpha > 0, "icon rendered fully transparent");
    QVERIFY2(qRed(strongest) > 240 && qGreen(strongest) < 16 && qBlue(strongest) < 16,
             "opaque pixel is not the requested colour");
}

void IconAssetsTest::disabledDiffers()
{
    const QIcon ic = pictura::icon(QStringLiteral("tool.brush"));
    const QImage normal = ic.pixmap(32, 32, QIcon::Normal)
                              .toImage()
                              .convertToFormat(QImage::Format_ARGB32);
    const QImage disabled = ic.pixmap(32, 32, QIcon::Disabled)
                                .toImage()
                                .convertToFormat(QImage::Format_ARGB32);
    QVERIFY(!normal.isNull());
    QVERIFY2(normal != disabled, "disabled icon did not change colour");
}

void IconAssetsTest::hidpiRenders()
{
    const QPixmap pm = pictura::icon(QStringLiteral("tool.brush")).pixmap(QSize(16, 16), 2.0);
    QVERIFY(!pm.isNull());
    QCOMPARE(pm.width(), 32);
    QCOMPARE(pm.devicePixelRatio(), 2.0);
}

void IconAssetsTest::themeChangeRerenders()
{
    // A theme change must invalidate the icon cache: the engine folds
    // `paletteGeneration()` into its cache key so the same QIcon instance
    // re-renders. Icon tints track the brightness-constant ButtonText, so the
    // generation counter is the observable that the key changed.
    pictura::Theme::apply(0);
    const quint64 dark = pictura::Theme::paletteGeneration();
    const QIcon ic = pictura::icon(QStringLiteral("tool.brush"));
    QVERIFY(!ic.pixmap(16, 16).isNull());
    pictura::Theme::apply(3);
    const quint64 light = pictura::Theme::paletteGeneration();
    pictura::Theme::apply(pictura::Theme::kDefaultLevel);
    QVERIFY2(dark != light, "palette generation did not advance on a theme change");
}

QTEST_MAIN(IconAssetsTest)
#include "tst_icon_assets.moc"
