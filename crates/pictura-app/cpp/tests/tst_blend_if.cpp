#include <QtTest/QtTest>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QApplication>
#include <QtCore/QTimer>
#include <QtCore/QTemporaryDir>
#include <QtCore/QFile>
#include <QtCore/QtEndian>
#include "panels/layers_panel.h"

#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "blend_if_slider.h"
#include "frame.h"
#include "image_view.h"
#include "layer_style_dialog.h"
#include "qt_test_support.h"

class BlendIfTest : public QObject {
    Q_OBJECT
private slots:
    void ranges();
    void preview();
    void split();
    void history();
    void unsupportedModes();
    void save();
    void badge();
    void opaqueRanges();
    void passThroughGroup();
};

// Modify the second layer record in a PSD freshly saved by the application.
// A four-byte range is deliberately malformed (valid bodies are 8-byte groups).
struct RangeOffsets {
    qsizetype section;
    qsizetype info;
    qsizetype extra;
    qsizetype ranges;
};

static bool rangeOffsets(const QByteArray& bytes, RangeOffsets& out, int layerIndex)
{
    auto u16 = [&bytes](qsizetype at) -> quint16 {
        return qFromBigEndian<quint16>(reinterpret_cast<const uchar*>(bytes.constData() + at));
    };
    auto u32 = [&bytes](qsizetype at) -> quint32 {
        return qFromBigEndian<quint32>(reinterpret_cast<const uchar*>(bytes.constData() + at));
    };
    auto has = [&bytes](qsizetype at, qsizetype count) {
        return at >= 0 && count >= 0 && at <= bytes.size() && count <= bytes.size() - at;
    };
    if (!has(0, 30) || bytes.left(4) != "8BPS" || u16(4) != 1) return false;
    qsizetype p = 26;
    for (int i = 0; i < 2; ++i) {
        if (!has(p, 4)) return false;
        const quint32 length = u32(p);
        p += 4;
        if (!has(p, length)) return false;
        p += length;
    }
    if (!has(p, 10)) return false;
    out.section = p;
    out.info = p + 4;
    p += 8;
    if (u16(p) <= layerIndex) return false;
    p += 2;
    for (int index = 0; index <= layerIndex; ++index) {
        if (!has(p, 18)) return false;
        const quint16 channels = u16(p + 16);
        p += 18;
        if (!has(p, qsizetype(channels) * 6 + 16)) return false;
        p += qsizetype(channels) * 6 + 12;
        const qsizetype extra = p;
        const quint32 extraLength = u32(p);
        p += 4;
        if (!has(p, extraLength) || extraLength < 8) return false;
        if (index == layerIndex) {
            out.extra = extra;
            const quint32 maskLength = u32(p);
            if (maskLength > extraLength - 8) return false;
            out.ranges = p + 4 + maskLength;
            const quint32 rangeLength = u32(out.ranges);
            return rangeLength <= extraLength - 8 - maskLength;
        }
        p += extraLength;
    }
    return false;
}

static QByteArray fixtureRanges(const QString& file, int layerIndex = 1)
{
    QFile input(file);
    if (!input.open(QIODevice::ReadOnly)) return {};
    const QByteArray bytes = input.readAll();
    RangeOffsets offsets{};
    if (!rangeOffsets(bytes, offsets, layerIndex)) return {};
    const auto length = qFromBigEndian<quint32>(reinterpret_cast<const uchar*>(bytes.constData() + offsets.ranges));
    return bytes.mid(offsets.ranges + 4, length);
}

static bool insertOpaqueFixture(const QString& file, int layerIndex = 1)
{
    QFile input(file);
    if (!input.open(QIODevice::ReadOnly)) return false;
    QByteArray bytes = input.readAll();
    input.close();
    RangeOffsets offsets{};
    if (!rangeOffsets(bytes, offsets, layerIndex)) return false;
    auto read32 = [&bytes](qsizetype at) {
        return qFromBigEndian<quint32>(reinterpret_cast<const uchar*>(bytes.constData() + at));
    };
    auto write32 = [&bytes](qsizetype at, quint32 value) {
        qToBigEndian<quint32>(value, reinterpret_cast<uchar*>(bytes.data() + at));
    };
    const quint32 oldLength = read32(offsets.ranges);
    if (oldLength % 4 != 0) return false;
    const qint64 delta = 4 - qint64(oldLength);
    for (qsizetype at : {offsets.section, offsets.info, offsets.extra}) {
        const qint64 value = qint64(read32(at)) + delta;
        if (value < 0 || value > 0xffffffffLL) return false;
        write32(at, quint32(value));
    }
    write32(offsets.ranges, 4);
    bytes.replace(offsets.ranges + 4, oldLength, QByteArray::fromHex("12345678"));
    if (!input.open(QIODevice::WriteOnly | QIODevice::Truncate)) return false;
    return input.write(bytes) == bytes.size();
}

static bool addLayer(pictura::PictureView* view)
{
    view->select_rect(2, 2, 8, 8, QStringLiteral("new"), 0.0);
    const bool added = !view->layer_via_copy(QStringLiteral("0")).isEmpty();
    view->deselect();
    return added;
}

static void dragBlack(pictura::BlendIfSlider* slider, int target)
{
    QTest::mousePress(slider, Qt::LeftButton, Qt::NoModifier, QPoint(8, 24));
    QTest::mouseMove(slider, QPoint(target, 24));
    QTest::mouseRelease(slider, Qt::LeftButton, Qt::NoModifier, QPoint(target, 24));
}

void BlendIfTest::ranges()
{
    pictura::test::ScopedStateHome home;
    QVERIFY(home.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(addLayer(view));
    pictura::LayerStyleDialog dialog(view, QStringLiteral("1"), QString());
    dialog.show();
    auto* channel = dialog.findChild<QComboBox*>(QStringLiteral("blendIf.channel"));
    auto* source = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
    auto* dest = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.destination"));
    QVERIFY(channel && source && dest);
    QCOMPARE(channel->count(), 4);
    QTest::mousePress(source, Qt::LeftButton, Qt::NoModifier, QPoint(8, 24));
    QTest::mouseMove(source, QPoint(source->width() / 4, 24));
    QTest::mouseRelease(source, Qt::LeftButton, Qt::NoModifier, QPoint(source->width() / 4, 24));
    QVERIFY(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), 0, false, false) > 0);
    QCOMPARE(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), 0, true, false), 0);
    channel->setCurrentIndex(1);
    QCOMPARE(source->blackWord(), 0);
    QTest::mousePress(dest, Qt::LeftButton, Qt::NoModifier, QPoint(8, 24));
    QTest::mouseMove(dest, QPoint(dest->width() / 3, 24));
    QTest::mouseRelease(dest, Qt::LeftButton, Qt::NoModifier, QPoint(dest->width() / 3, 24));
    QVERIFY(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), 1, true, false) > 0);
    QCOMPARE(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), 1, false, false), 0);
    dialog.reject();
}

void BlendIfTest::preview()
{
    pictura::test::ScopedStateHome home;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    const QString path = view->add_solid_fill(0xff000000u);
    QVERIFY(!path.isEmpty());
    const QImage before = view->image();
    QCOMPARE(before.pixelColor(4, 4).rgb(), QColor(Qt::black).rgb());
    pictura::LayerStyleDialog dialog(view, path, QString());
    dialog.show();
    auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
    QVERIFY(slider);
    dragBlack(slider, slider->width() / 4);
    QTRY_COMPARE(view->image().pixelColor(4, 4).rgb(), QColor(Qt::white).rgb());
    dialog.reject();
    QTRY_COMPARE(view->image().pixelColor(4, 4).rgb(), QColor(Qt::black).rgb());
}

void BlendIfTest::split()
{
    pictura::BlendIfSlider slider;
    slider.resize(271, 36);
    slider.show();
    QSignalSpy changed(&slider, &pictura::BlendIfSlider::rangeChanged);
    QTest::mousePress(&slider, Qt::LeftButton, Qt::AltModifier, QPoint(11, 24));
    QTest::mouseMove(&slider, QPoint(111, 24));
    QTest::mouseRelease(&slider, Qt::LeftButton, Qt::AltModifier, QPoint(111, 24));
    QCOMPARE(slider.blackWord(), 100);
    QVERIFY(!changed.isEmpty());
    QTest::mousePress(&slider, Qt::LeftButton, Qt::AltModifier, QPoint(108, 24));
    QTest::mouseMove(&slider, QPoint(270, 24));
    QTest::mouseRelease(&slider, Qt::LeftButton, Qt::AltModifier, QPoint(270, 24));
    QCOMPARE(slider.blackWord(), 255);
    slider.setRange(0, 0xffff);
    QTest::mousePress(&slider, Qt::LeftButton, Qt::AltModifier, QPoint(260, 24));
    QTest::mouseMove(&slider, QPoint(155, 24));
    QTest::mouseRelease(&slider, Qt::LeftButton, Qt::AltModifier, QPoint(155, 24));
    QCOMPARE(slider.whiteWord(), (150 << 8) | 255);
    slider.setRange(0, 0xffff);
    QTest::mousePress(&slider, Qt::LeftButton, Qt::NoModifier, QPoint(8, 24));
    QTest::mouseMove(&slider, QPoint(80, 24));
    QVERIFY(slider.blackWord() != 0);
    QTest::mouseMove(&slider, QPoint(8, 24));
    QCOMPARE(slider.blackWord(), 0);
    QCOMPARE(changed.last().at(0).toInt(), 0);
    QTest::mouseRelease(&slider, Qt::LeftButton, Qt::NoModifier, QPoint(8, 24));
}

void BlendIfTest::history()
{
    pictura::test::ScopedStateHome home;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(addLayer(view));
    const int count = view->history_count();
    {
        pictura::LayerStyleDialog dialog(view, QStringLiteral("1"), QString());
        auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
        QVERIFY(slider);
        dialog.show();
        dragBlack(slider, slider->width() / 4);
        dialog.reject();
    }
    QCOMPARE(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), 0, false, false), 0);
    QCOMPARE(view->history_count(), count);
    {
        pictura::LayerStyleDialog dialog(view, QStringLiteral("1"), QString());
        auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
        QVERIFY(slider);
        dialog.show();
        dragBlack(slider, slider->width() / 4);
        dialog.accept();
    }
    QCOMPARE(view->history_count(), count + 1);
    QVERIFY(view->undo());
    QCOMPARE(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), 0, false, false), 0);
}

void BlendIfTest::unsupportedModes()
{
    pictura::test::ScopedStateHome home;
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    auto window = pictura::test::makeMainWindow();
    for (const QString& mode : {QStringLiteral("cmyk"), QStringLiteral("lab")}) {
        QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                    QStringLiteral("white")));
        auto* view = window->activeView();
        QVERIFY(addLayer(view));
        QVERIFY(pictura::convert_image_mode(*view, mode));
        const QString path = QStringLiteral("1");
        const QString file = directory.filePath(mode + QStringLiteral(".psd"));
        QVERIFY(window->saveActiveAs(file));
        window->closeDocument(window->activeDocumentIndex(), false);
        QVERIFY(insertOpaqueFixture(file));
        QVERIFY(window->openDocumentAtPath(file));
        view = window->activeView();
        {
            pictura::LayerStyleDialog dialog(view, path, QString());
            auto* channel = dialog.findChild<QComboBox*>(QStringLiteral("blendIf.channel"));
            auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
            auto* replace = dialog.findChild<QPushButton*>(QStringLiteral("blendIf.replace"));
            QVERIFY(channel && slider && replace);
            QVERIFY(!channel->isEnabled());
            QVERIFY(!slider->isEnabled());
            QVERIFY(!replace->isEnabled());
            QVERIFY(!pictura::layer_style_replace_blend_if(*view, path));
            dialog.accept();
        }
        QVERIFY(window->saveActiveAs(file));
        window->closeDocument(window->activeDocumentIndex(), false);
        QVERIFY(window->openDocumentAtPath(file));
        view = window->activeView();
        QCOMPARE(fixtureRanges(file), QByteArray::fromHex("12345678"));
        window->closeDocument(window->activeDocumentIndex(), false);
    }
}

void BlendIfTest::save()
{
    pictura::test::ScopedStateHome home;
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(addLayer(view));
    int expected = 0;
    {
        pictura::LayerStyleDialog dialog(view, QStringLiteral("1"), QString());
        dialog.show();
        auto* channel = dialog.findChild<QComboBox*>(QStringLiteral("blendIf.channel"));
        auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.destination"));
        QVERIFY(channel && slider);
        channel->setCurrentIndex(1);
        dragBlack(slider, slider->width() / 4);
        expected = pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), 1, true, false);
        QVERIFY(expected > 0);
        for (int channelIndex = 0; channelIndex < 4; ++channelIndex) {
            for (bool dest : {false, true}) {
                if (channelIndex == 1 && dest) continue;
                const int value = (20 + channelIndex * 20 + (dest ? 5 : 0)) * 257;
                QVERIFY(pictura::layer_style_blend_if_range(*view, QStringLiteral("1"),
                                                            channelIndex, dest, value, 0xffff));
            }
        }
        dialog.accept();
    }
    const QString file = directory.filePath(QStringLiteral("ranges.psd"));
    QVERIFY(window->saveActiveAs(file));
    window->closeDocument(window->activeDocumentIndex(), false);
    QVERIFY(window->openDocumentAtPath(file));
    view = window->activeView();
    for (int channelIndex = 0; channelIndex < 4; ++channelIndex) {
        for (bool dest : {false, true}) {
            const int black = channelIndex == 1 && dest
                ? expected : (20 + channelIndex * 20 + (dest ? 5 : 0)) * 257;
            QCOMPARE(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), channelIndex, dest, false), black);
            QCOMPARE(pictura::layer_style_blend_if_word(*view, QStringLiteral("1"), channelIndex, dest, true), 0xffff);
        }
    }
}

void BlendIfTest::badge()
{
    pictura::test::ScopedStateHome home;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(addLayer(view));
    auto* panel = window->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();
    QVERIFY(!panel->rowHasBlendIfForTest(QStringLiteral("1")));
    {
        pictura::LayerStyleDialog dialog(view, QStringLiteral("1"), QString());
        dialog.show();
        auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
        QVERIFY(slider);
        dragBlack(slider, slider->width() / 4);
        dialog.accept();
    }
    QCoreApplication::processEvents();
    panel->refresh();
    QVERIFY(panel->rowHasBlendIfForTest(QStringLiteral("1")));
    const QRect badge = panel->rowBlendIfRectForTest(QStringLiteral("1"));
    QVERIFY(!badge.isEmpty());
    QVERIFY(!badge.intersects(panel->rowNameRectForTest(QStringLiteral("1"))));
    QVERIFY(view->undo());
    QCoreApplication::processEvents();
    panel->refresh();
    QVERIFY(!panel->rowHasBlendIfForTest(QStringLiteral("1")));
}

void BlendIfTest::opaqueRanges()
{
    pictura::test::ScopedStateHome home;
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    auto* view = window->activeView();
    QVERIFY(addLayer(view));
    const QString path = QStringLiteral("1");
    const QString file = directory.filePath(QStringLiteral("opaque.psd"));
    QVERIFY(window->saveActiveAs(file));
    window->closeDocument(window->activeDocumentIndex(), false);
    QVERIFY(insertOpaqueFixture(file));
    QVERIFY(window->openDocumentAtPath(file));
    view = window->activeView();
    QCOMPARE(pictura::layer_style_blend_if_status(*view, path), 1);
    auto* panel = window->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();
    QVERIFY(panel->rowHasBlendIfForTest(path));
    {
        pictura::LayerStyleDialog dialog(view, path, QString());
        dialog.show();
        auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
        auto* replace = dialog.findChild<QPushButton*>(QStringLiteral("blendIf.replace"));
        QVERIFY(slider && replace);
        QVERIFY(!slider->isEnabled());
        QVERIFY(replace->isEnabled());
        QTimer::singleShot(0, [] {
            auto* box = qobject_cast<QMessageBox*>(QApplication::activeModalWidget());
            if (box) {
                box->button(QMessageBox::No)->click();
            }
        });
        QTest::mouseClick(replace, Qt::LeftButton);
        QCOMPARE(pictura::layer_style_blend_if_status(*view, path), 1);
        dialog.accept();
    }
    QVERIFY(window->saveActiveAs(file));
    QCOMPARE(fixtureRanges(file), QByteArray::fromHex("12345678"));
    window->closeDocument(window->activeDocumentIndex(), false);
    QVERIFY(window->openDocumentAtPath(file));
    view = window->activeView();
    QCOMPARE(pictura::layer_style_blend_if_status(*view, path), 1);
    for (bool accept : {false, true}) {
        pictura::LayerStyleDialog dialog(view, path, QString());
        dialog.show();
        auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
        auto* replace = dialog.findChild<QPushButton*>(QStringLiteral("blendIf.replace"));
        QVERIFY(slider && replace);
        QTimer::singleShot(0, [] {
            auto* box = qobject_cast<QMessageBox*>(QApplication::activeModalWidget());
            if (box) {
                box->button(QMessageBox::Yes)->click();
            }
        });
        QTest::mouseClick(replace, Qt::LeftButton);
        QVERIFY(slider->isEnabled());
        QCOMPARE(slider->blackWord(), 0);
        QCOMPARE(slider->whiteWord(), 0xffff);
        QCOMPARE(pictura::layer_style_blend_if_status(*view, path), 0);
        if (accept) dialog.accept(); else dialog.reject();
        if (accept) QVERIFY(view->undo());
        QCOMPARE(pictura::layer_style_blend_if_status(*view, path), 1);
    }
}

void BlendIfTest::passThroughGroup()
{
    pictura::test::ScopedStateHome home;
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window->newDocument(QStringLiteral("Blend"), 20, 20, QStringLiteral("rgb"), 8,
                                QStringLiteral("white")));
    QTemporaryDir directory;
    QVERIFY(directory.isValid());
    auto* view = window->activeView();
    const QString group = view->add_group_in(QString());
    QVERIFY(!group.isEmpty());
    QCOMPARE(view->set_layers_blend(QStringList{group}, QStringLiteral("pass")), 1);
    const QString file = directory.filePath(QStringLiteral("group.psd"));
    QVERIFY(window->saveActiveAs(file));
    window->closeDocument(window->activeDocumentIndex(), false);
    // A group emits a divider followed by its group record.
    QVERIFY(insertOpaqueFixture(file, 2));
    QVERIFY(window->openDocumentAtPath(file));
    view = window->activeView();
    pictura::LayerStyleDialog dialog(view, group, QString());
    dialog.show();
    auto* slider = dialog.findChild<pictura::BlendIfSlider*>(QStringLiteral("blendIf.source"));
    auto* replace = dialog.findChild<QPushButton*>(QStringLiteral("blendIf.replace"));
    QVERIFY(slider && replace);
    QVERIFY(!slider->isEnabled());
    QVERIFY(!replace->isEnabled());
    QCOMPARE(pictura::layer_style_blend_if_status(*view, group), -2);
    QVERIFY(!pictura::layer_style_replace_blend_if(*view, group));
    dialog.accept();
    QVERIFY(window->saveActiveAs(file));
    QCOMPARE(fixtureRanges(file, 2), QByteArray::fromHex("12345678"));
}

PICTURA_QT_TEST_MAIN(BlendIfTest)
#include "tst_blend_if.moc"
