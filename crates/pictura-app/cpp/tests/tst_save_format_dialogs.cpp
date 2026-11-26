// The six new Save As dialog surfaces and the BMP Advanced Modes sub-dialog.

#include <QtTest/QtTest>

#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>

#include "bmp_advanced_modes_dialog.h"
#include "cineon_options_dialog.h"
#include "dcs_options_dialog.h"
#include "eps_options_dialog.h"
#include "gif_options_dialog.h"
#include "pict_options_dialog.h"
#include "raw_options_dialog.h"
#include "save_format_options.h"
#include "tga_options_dialog.h"

class SaveFormatDialogsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase() {}
    void gifControls();
    void epsControls();
    void dcsControls();
    void rawControls();
    void tgaControls();
    void cineonControls();
    void pictControls();
    void bmpAdvancedControls();
    void rawMacDisabled();
};

void SaveFormatDialogsTest::gifControls()
{
    const pictura::SaveFormatOptions defaults;
    pictura::GifOptionsDialog dialog(defaults);
    QCOMPARE(dialog.rowOrder(), 0);
    dialog.findChild<QRadioButton*>(QStringLiteral("gifOptionsInterlaced"))->setChecked(true);
    QCOMPARE(dialog.rowOrder(), 1);
    pictura::SaveFormatOptions remembered;
    remembered.gifRowOrder = 1;
    pictura::GifOptionsDialog seeded(remembered);
    QCOMPARE(seeded.rowOrder(), 1);
}

void SaveFormatDialogsTest::epsControls()
{
    const pictura::SaveFormatOptions defaults;
    pictura::EpsOptionsDialog dialog(defaults);
    QCOMPARE(dialog.preview(), 2);
    QCOMPARE(dialog.encoding(), 2);
    QVERIFY(dialog.interpolation());
    auto* preview = dialog.findChild<QComboBox*>(QStringLiteral("epsOptionsPreview"));
    QVERIFY(preview);
    preview->setCurrentIndex(0);
    QCOMPARE(dialog.preview(), 0);
    pictura::SaveFormatOptions remembered;
    remembered.epsPreview = 1;
    remembered.epsEncoding = 3;
    remembered.epsInterpolation = false;
    pictura::EpsOptionsDialog seeded(remembered);
    QCOMPARE(seeded.preview(), 1);
    QCOMPARE(seeded.encoding(), 3);
    QVERIFY(!seeded.interpolation());
}

void SaveFormatDialogsTest::dcsControls()
{
    const pictura::SaveFormatOptions defaults;
    pictura::DcsOptionsDialog dialog(defaults);
    QCOMPARE(dialog.version(), 2);
    QCOMPARE(dialog.preview(), 2);
    pictura::SaveFormatOptions remembered;
    remembered.dcsVersion = 1;
    remembered.dcsPreview = 1;
    pictura::DcsOptionsDialog seeded(remembered);
    QCOMPARE(seeded.version(), 1);
    QCOMPARE(seeded.preview(), 1);
}

void SaveFormatDialogsTest::rawControls()
{
    const pictura::SaveFormatOptions defaults;
    pictura::RawOptionsDialog dialog(defaults);
    QCOMPARE(dialog.header(), 0);
    QCOMPARE(dialog.interleave(), 0);
    auto* header = dialog.findChild<QSpinBox*>(QStringLiteral("rawOptionsHeader"));
    QVERIFY(header);
    header->setValue(16);
    QCOMPARE(dialog.header(), 16);
    pictura::SaveFormatOptions remembered;
    remembered.rawHeader = 16;
    remembered.rawInterleave = 1;
    pictura::RawOptionsDialog seeded(remembered);
    QCOMPARE(seeded.header(), 16);
    QCOMPARE(seeded.interleave(), 1);
}

void SaveFormatDialogsTest::tgaControls()
{
    const pictura::SaveFormatOptions defaults;
    pictura::TgaOptionsDialog dialog(defaults);
    QCOMPARE(dialog.depth(), 24);
    QVERIFY(!dialog.rle());
    pictura::SaveFormatOptions remembered;
    remembered.tgaDepth = 32;
    remembered.tgaRle = true;
    pictura::TgaOptionsDialog seeded(remembered);
    QCOMPARE(seeded.depth(), 32);
    QVERIFY(seeded.rle());
}

void SaveFormatDialogsTest::cineonControls()
{
    const pictura::SaveFormatOptions defaults;
    pictura::CineonOptionsDialog dialog(defaults);
    QCOMPARE(dialog.bits(), 10);
    QVERIFY(dialog.findChild<QWidget*>(QStringLiteral("cineonOptionsBits")));
}

void SaveFormatDialogsTest::pictControls()
{
    const pictura::SaveFormatOptions defaults;
    pictura::PictOptionsDialog dialog(defaults);
    QCOMPARE(dialog.resolution(), 32);
    QCOMPARE(dialog.compression(), 0);
    auto* low = dialog.findChild<QRadioButton*>(QStringLiteral("pictOptionsResolution16"));
    QVERIFY(low);
    low->setChecked(true);
    QCOMPARE(dialog.resolution(), 16);
    auto* jpeg = dialog.findChild<QRadioButton*>(QStringLiteral("pictOptionsCompressionHigh"));
    QVERIFY(jpeg);
    jpeg->setChecked(true);
    QCOMPARE(dialog.compression(), 3);
    pictura::SaveFormatOptions remembered;
    remembered.pictResolution = 16;
    remembered.pictCompression = 3;
    pictura::PictOptionsDialog seeded(remembered);
    QCOMPARE(seeded.resolution(), 16);
    QCOMPARE(seeded.compression(), 3);
}

void SaveFormatDialogsTest::bmpAdvancedControls()
{
    pictura::BmpAdvancedModesDialog dialog(0);
    QCOMPARE(dialog.layout(), 0);
    dialog.findChild<QRadioButton*>(QStringLiteral("bmpAdvancedR5G6B5"))->setChecked(true);
    QCOMPARE(dialog.layout(), 2);
    pictura::BmpAdvancedModesDialog seeded(2);
    QCOMPARE(seeded.layout(), 2);
}

void SaveFormatDialogsTest::rawMacDisabled()
{
    const pictura::SaveFormatOptions defaults;
    pictura::RawOptionsDialog dialog(defaults);
    QVERIFY(!dialog.findChild<QLineEdit*>(QStringLiteral("rawOptionsFileType"))->isEnabled());
    QVERIFY(!dialog.findChild<QLineEdit*>(QStringLiteral("rawOptionsFileCreator"))->isEnabled());
}

#include "qt_test_support.h"

PICTURA_QT_TEST_MAIN(SaveFormatDialogsTest)
#include "tst_save_format_dialogs.moc"
