#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QPointF>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtGui/QScreen>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include <cmath>

#include "color_models.h"
#include "color_picker_dialog.h"
#include "frame.h"
#include "panels/info_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_size.cxxqt.h"

#include "qt_test_support.h"

class InfoPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void defaultColorModeIsActualColor();
    void colorBlockReadsRgbAndSwitchesModes();
    void colorMenuModes();
    void colorMenuHasCs6Entries();
    void bitDepthScalesRgb();
    void measurementUnitReformatsPosition();
    void physicalUnitsUseDocumentPpi();
    void opacityReadoutUsesLayerOpacity();
    void unitInheritanceAndNoSizeMenu();
    void selectionDrivesSizeAndBlanksWithoutOne();
    void rulerModeSwapsTopRightBlock();
    void offCanvasBlanksColorReadouts();
    void docLineIsPresent();
    void cursorMoveDoesNotRestatTheDocument();
    void toolRowDescribesActiveTool();
    void toolHintsOnePerLineCapitalisedReducedFont();
    void menuOpensBesideItsButton();
    void labMatchesColorPicker();
    void cmykMatchesColorPicker();
    void cmykReadoutFollowsProfile();

private:
    void openDocument(const QString& name);
    // Fill the open document with `rgb` and hover it.
    void hoverColor(QRgb rgb);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::InfoPanel* panel_ = nullptr;
};

void InfoPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void InfoPanelTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

void InfoPanelTest::openDocument(const QString& name)
{
    const bool created = window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8,
                                              QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::InfoPanel*>(QStringLiteral("infoPanel"));
    QVERIFY2(created && view_ && panel_, "info fixture");
    panel_->setView(view_);
    // The panel outlives a document, so reset its readout state for isolation.
    panel_->setColorModeForTest(0, QStringLiteral("Actual Color"));
    panel_->setColorModeForTest(1, QStringLiteral("CMYK"));
    panel_->setMeasurementUnitForTest(0, QStringLiteral("Pixels"));
    panel_->setMeasurementUnitForTest(1, QStringLiteral("Pixels"));
    panel_->setBitDepthForTest(0, 8);
    panel_->setBitDepthForTest(1, 8);
}

void InfoPanelTest::defaultColorModeIsActualColor()
{
    panel_ = window_->findChild<pictura::InfoPanel*>(QStringLiteral("infoPanel"));
    QVERIFY2(panel_ != nullptr, "panel exists before any document");
    QCOMPARE(panel_->colorModeForTest(0), QStringLiteral("Actual Color"));
    QCOMPARE(panel_->colorModeForTest(1), QStringLiteral("CMYK"));
}

void InfoPanelTest::colorBlockReadsRgbAndSwitchesModes()
{
    openDocument(QStringLiteral("Color"));
    panel_->setCursorPosition(QPointF(2, 2));

    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("R : 255")), "rgb readout");
    QVERIFY2(panel_->colorBlockTextForTest(1).contains(QStringLiteral("C : 0")),
             "top-right defaults to CMYK");

    panel_->setColorModeForTest(0, QStringLiteral("CMYK"));
    const QString cmyk = panel_->colorBlockTextForTest(0);
    QVERIFY2(cmyk.contains(QStringLiteral("C : 0")) && cmyk.contains(QStringLiteral("M : 0"))
                 && cmyk.contains(QStringLiteral("Y : 0")) && cmyk.contains(QStringLiteral("K : 0")),
             "top-left switches to CMYK rows");

    panel_->setColorModeForTest(0, QStringLiteral("Grayscale"));
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("K : 255")),
             "grayscale uses qGray");
}

void InfoPanelTest::colorMenuModes()
{
    openDocument(QStringLiteral("ColorMenu"));
    const int layer = view_->add_layer(0);
    QVERIFY(layer >= 0);
    view_->set_active_layer(QString::number(layer));
    panel_->setCursorPosition(QPointF(2, 2));

    panel_->setColorModeForTest(0, QStringLiteral("Proof Color"));
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("R : 255")),
             "proof color reads rgb");

    panel_->setColorModeForTest(0, QStringLiteral("Total Ink"));
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("Ink : 0")),
             "total ink sums cmyk");

    panel_->setColorModeForTest(0, QStringLiteral("Opacity"));
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("Op : 100")),
             "opacity is a percentage");

    panel_->setColorModeForTest(0, QStringLiteral("Lab"));
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("L : 100")), "lab rows");
}

void InfoPanelTest::colorMenuHasCs6Entries()
{
    openDocument(QStringLiteral("MenuTexts"));
    const QStringList expected = {QStringLiteral("Actual Color"), QStringLiteral("Proof Color"),
                                  QStringLiteral("---"),          QStringLiteral("Grayscale"),
                                  QStringLiteral("RGB"),          QStringLiteral("HSB"),
                                  QStringLiteral("CMYK"),         QStringLiteral("Lab"),
                                  QStringLiteral("---"),          QStringLiteral("Total Ink"),
                                  QStringLiteral("Opacity"),      QStringLiteral("---"),
                                  QStringLiteral("8-bit"),        QStringLiteral("16-bit"),
                                  QStringLiteral("32-bit")};
    QCOMPARE(panel_->colorMenuTextsForTest(0), expected);
    QCOMPARE(panel_->colorMenuTextsForTest(1), expected);
}

void InfoPanelTest::bitDepthScalesRgb()
{
    openDocument(QStringLiteral("Depth"));
    panel_->setCursorPosition(QPointF(2, 2));
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("R : 255")), "8-bit raw");
    QCOMPARE(panel_->colorFooterForTest(0), QStringLiteral("8-bit"));

    panel_->setBitDepthForTest(0, 16);
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("R : 65535")),
             "16-bit scales 255");
    QCOMPARE(panel_->colorFooterForTest(0), QStringLiteral("16-bit"));

    panel_->setBitDepthForTest(0, 32);
    QVERIFY2(panel_->colorBlockTextForTest(0).contains(QStringLiteral("R : 1.000")),
             "32-bit is a unit value");
    QCOMPARE(panel_->colorFooterForTest(0), QStringLiteral("32-bit"));
}

void InfoPanelTest::measurementUnitReformatsPosition()
{
    openDocument(QStringLiteral("Units"));
    panel_->setCursorPosition(QPointF(2, 2));
    QCOMPARE(panel_->positionTextForTest(), QStringLiteral("2, 2"));

    // 72 PPI: 2 px = 0.71 mm.
    panel_->setMeasurementUnitForTest(0, QStringLiteral("Millimeters"));
    QCOMPARE(panel_->positionTextForTest(), QStringLiteral("0.71, 0.71"));

    panel_->setMeasurementUnitForTest(0, QStringLiteral("Pixels"));
    QCOMPARE(panel_->positionTextForTest(), QStringLiteral("2, 2"));
}

void InfoPanelTest::physicalUnitsUseDocumentPpi()
{
    QVERIFY(window_->newDocument(QStringLiteral("Ppi"), 600, 600, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::InfoPanel*>(QStringLiteral("infoPanel"));
    QVERIFY(view_ && panel_);
    QVERIFY(pictura::image_size_apply(*view_, QString(), 600, 600, 300.0, false));
    panel_->setView(view_);
    panel_->setCursorPosition(QPointF(300, 300));
    panel_->setMeasurementUnitForTest(0, QStringLiteral("Inches"));
    QCOMPARE(panel_->positionTextForTest(), QStringLiteral("1.00, 1.00"));
}

void InfoPanelTest::opacityReadoutUsesLayerOpacity()
{
    openDocument(QStringLiteral("Opacity"));
    QVERIFY2(view_->background_from_layer(QStringLiteral("0")),
             "the fixture starts with a real Background");
    const int layer = view_->add_layer(0);
    QVERIFY(layer >= 0);
    view_->set_active_layer(QString::number(layer));
    QVERIFY(view_->set_layer_opacity(layer, 128));
    panel_->setCursorPosition(QPointF(2, 2));
    panel_->setColorModeForTest(0, QStringLiteral("Opacity"));
    QCOMPARE(panel_->colorBlockTextForTest(0), QStringLiteral("Op : 50"));

    view_->set_active_layer(QStringLiteral("0"));
    panel_->refresh();
    QCOMPARE(panel_->colorBlockTextForTest(0), QStringLiteral("Op : "));
}

void InfoPanelTest::unitInheritanceAndNoSizeMenu()
{
    openDocument(QStringLiteral("SharedUnit"));
    QVERIFY2(!panel_->sizeBlockHasMenuForTest(), "W/H inherits its unit, no menu");

    QVERIFY2(view_->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0.0), "select rect");
    panel_->refresh();
    QCOMPARE(panel_->sizeTextForTest(), QStringLiteral("4 × 4"));

    // 72 PPI: 4 px = 1.41 mm; the X/Y unit drives the W/H block too.
    panel_->setMeasurementUnitForTest(0, QStringLiteral("Millimeters"));
    QCOMPARE(panel_->sizeTextForTest(), QStringLiteral("1.41 × 1.41"));
}

void InfoPanelTest::selectionDrivesSizeAndBlanksWithoutOne()
{
    openDocument(QStringLiteral("Selection"));
    QVERIFY2(panel_->sizeTextForTest().isEmpty(), "no selection blanks W/H");

    QVERIFY2(view_->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0.0), "select rect");
    panel_->refresh();
    QCOMPARE(panel_->sizeTextForTest(), QStringLiteral("4 × 4"));

    panel_->setMeasurementUnitForTest(1, QStringLiteral("Millimeters"));
    QCOMPARE(panel_->sizeTextForTest(), QStringLiteral("1.41 × 1.41"));
}

void InfoPanelTest::rulerModeSwapsTopRightBlock()
{
    openDocument(QStringLiteral("Ruler"));
    panel_->setRulerMode(true);
    QVERIFY2(panel_->rulerMode(), "ruler mode on");
    QVERIFY2(panel_->rulerTextForTest().isEmpty(), "no line -> blank A/L");
    QVERIFY2(panel_->sizeTextForTest().isEmpty(), "no line -> blank W/H");

    pictura::set_ruler(*view_, 2.0, 3.0, 2.0, 13.0, false);
    panel_->refresh();
    QVERIFY2(panel_->rulerTextForTest().contains(QStringLiteral("-90.0")), "angle readout");
    QVERIFY2(panel_->rulerTextForTest().contains(QStringLiteral("10.0")), "length readout");
    QCOMPARE(panel_->sizeTextForTest(), QStringLiteral("0 × 10"));

    panel_->setRulerMode(false);
    QVERIFY2(panel_->colorBlockTextForTest(1).contains(QStringLiteral("C :")),
             "CMYK block restored");
}

void InfoPanelTest::offCanvasBlanksColorReadouts()
{
    openDocument(QStringLiteral("OffCanvas"));
    panel_->setCursorPosition(QPointF(100, 100));
    QVERIFY2(!panel_->colorBlockTextForTest(0).contains(QStringLiteral("255")),
             "off-canvas RGB blank");
    QVERIFY2(!panel_->colorBlockTextForTest(1).contains(QStringLiteral("0")),
             "off-canvas CMYK blank");
}

void InfoPanelTest::docLineIsPresent()
{
    openDocument(QStringLiteral("Doc"));
    panel_->setCursorPosition(QPointF(2, 2));
    QVERIFY2(panel_->docTextForTest().startsWith(QStringLiteral("Doc: ")), "doc line present");
    // A 16x16 RGB document holds an 8-plane footprint = 2048 bytes = 2.0K.
    QVERIFY2(panel_->docTextForTest().contains(QStringLiteral("2.0K")), "memory footprint");
}

// #754: a pointer move must not restat the document file. Removing the file
// after opening makes a per-move stat observable: the disk size would fall to
// zero and the line would change.
void InfoPanelTest::cursorMoveDoesNotRestatTheDocument()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("restat.png"));
    QImage image(16, 16, QImage::Format_RGB32);
    image.fill(Qt::white);
    QVERIFY(image.save(path));
    QVERIFY(window_->openDocumentAtPath(path));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::InfoPanel*>(QStringLiteral("infoPanel"));
    QVERIFY2(view_ && panel_, "open fixture");
    panel_->setView(view_);
    panel_->setCursorPosition(QPointF(2, 2));
    const QString before = panel_->docTextForTest();
    QVERIFY2(before.startsWith(QStringLiteral("Doc: ")), "doc line present");

    QVERIFY(QFile::remove(path));
    panel_->setCursorPosition(QPointF(4, 4));
    QCOMPARE(panel_->docTextForTest(), before);
}

void InfoPanelTest::toolRowDescribesActiveTool()
{
    openDocument(QStringLiteral("ToolRow"));
    window_->setActiveTool(pictura::ToolId::Move);
    QCoreApplication::processEvents();
    QVERIFY2(!panel_->toolTextForTest().isEmpty(), "tool row populated");
    QVERIFY2(panel_->toolTextForTest().contains(QStringLiteral("Nudge")),
             "move hints present");
}

void InfoPanelTest::toolHintsOnePerLineCapitalisedReducedFont()
{
    openDocument(QStringLiteral("Hints"));
    const QStringList hints = {QStringLiteral("Arrows nudge 1 px"),
                               QStringLiteral("alt subtract from selection")};
    panel_->setToolInfo(QStringLiteral("Move"), hints);

    const QString text = panel_->toolTextForTest();
    QVERIFY2(text.contains(QStringLiteral("Arrows nudge 1 px\n")), "one hint per line");
    QVERIFY2(text.contains(QStringLiteral("Alt subtract from selection")),
             "hint first letter capitalised");

    auto* label = panel_->findChild<QLabel*>(QStringLiteral("infoTool"));
    QVERIFY(label != nullptr);
    int base = QApplication::font().pixelSize();
    if (base <= 0) {
        base = 12;
    }
    QCOMPARE(label->font().pixelSize(), base - 2);
}

void InfoPanelTest::menuOpensBesideItsButton()
{
    openDocument(QStringLiteral("MenuPos"));
    // The reposition needs the button's screen, so the window must be up.
    window_->show();
    QTest::qWait(50);

    QToolButton* button = nullptr;
    for (QToolButton* candidate : panel_->findChildren<QToolButton*>()) {
        if (candidate->menu()) {
            button = candidate;
            break;
        }
    }
    QVERIFY2(button, "a readout button owns a menu");
    QMenu* menu = button->menu();
    QVERIFY2(menu != nullptr, "the menu is attached");
    // QToolButton::showMenu() blocks until the menu closes, and a real click
    // goes through the same popup(); open it where QToolButton would (below)
    // and let the panel's Show filter move it aside.
    menu->popup(button->mapToGlobal(QPoint(0, button->height())));
    const bool visible = menu->isVisible();
    const QPoint buttonTopLeft = button->mapToGlobal(QPoint(0, 0));
    const QRect buttonRect(buttonTopLeft, button->size());
    const QRect menuRect(menu->geometry());
    const QSize hint = menu->sizeHint();
    const int shownHeight = menu->height();
    // Hide before asserting so a failure never leaves a popup behind.
    menu->hide();

    QVERIFY2(visible, "menu opened");
    QVERIFY2(shownHeight >= hint.height(), "first-open menu is laid out, not clipped");
    QVERIFY2(!menuRect.intersects(buttonRect), "menu does not cover its button");
    QVERIFY2(menuRect.left() >= buttonRect.right() || menuRect.right() <= buttonRect.left(),
             "menu sits beside the button");
    // The panel's Show filter opens the menu flush to the button's right side
    // (`button width + 1`), flipping left only when that would overflow the
    // available area, so the first open is placed explicitly, not merely
    // sized.
    if (QScreen* screen = button->screen()) {
        const QRect area = screen->availableGeometry();
        const int rightPlacement = buttonTopLeft.x() + button->width() + 1;
        if (rightPlacement + hint.width() <= area.right() + 1) {
            QCOMPARE(menuRect.left(), rightPlacement);
        }
        QVERIFY2(area.contains(menuRect),
                 "the first-open menu stays inside the available geometry");
    }
}

void InfoPanelTest::hoverColor(QRgb rgb)
{
    QVERIFY(!view_->add_solid_fill(0xff000000u | rgb).isEmpty());
    QCoreApplication::processEvents();
    panel_->setCursorPosition(QPointF(2, 2));
}

// The picker's Lab fields and the Info panel's Lab readout come from one
// conversion (D50), so the same colour reads the same everywhere.
void InfoPanelTest::labMatchesColorPicker()
{
    openDocument(QStringLiteral("LabAgree"));
    const QColor color(0x22, 0x44, 0xaa);
    hoverColor(color.rgb() & 0xffffff);
    panel_->setColorModeForTest(0, QStringLiteral("Lab"));
    pictura::ColorPickerDialog picker(color);
    const auto field = [&](const char* name) {
        auto* spin = picker.findChild<QSpinBox*>(QLatin1String(name));
        return spin ? spin->value() : 9999;
    };
    const QString expected = QStringLiteral("L : %1  a : %2  b : %3")
                                 .arg(field("colorPickerL"))
                                 .arg(field("colorPickerLabA"))
                                 .arg(field("colorPickerLabB"));
    QCOMPARE(panel_->colorBlockTextForTest(0), expected);
}

void InfoPanelTest::cmykMatchesColorPicker()
{
    openDocument(QStringLiteral("CmykAgree"));
    const QColor color(0xc8, 0x78, 0x50);
    hoverColor(color.rgb() & 0xffffff);
    panel_->setColorModeForTest(0, QStringLiteral("CMYK"));
    pictura::ColorPickerDialog picker(color);
    const auto field = [&](const char* name) {
        auto* spin = picker.findChild<QSpinBox*>(QLatin1String(name));
        return spin ? spin->value() : 9999;
    };
    const QString expected = QStringLiteral("C : %1  M : %2  Y : %3  K : %4")
                                 .arg(field("colorPickerC"))
                                 .arg(field("colorPickerM"))
                                 .arg(field("colorPickerY"))
                                 .arg(field("colorPickerK"));
    QCOMPARE(panel_->colorBlockTextForTest(0), expected);
}

// CMYK goes through the working profile, so black is a rich black rather than
// the naive 0/0/0/100.
void InfoPanelTest::cmykReadoutFollowsProfile()
{
    openDocument(QStringLiteral("CmykBlack"));
    hoverColor(0x000000);
    panel_->setColorModeForTest(0, QStringLiteral("CMYK"));
    const pictura::CmykPercent inks = pictura::rgbToCmyk(QColor(0, 0, 0));
    const QString expected = QStringLiteral("C : %1  M : %2  Y : %3  K : %4")
                                 .arg(std::lround(inks.c))
                                 .arg(std::lround(inks.m))
                                 .arg(std::lround(inks.y))
                                 .arg(std::lround(inks.k));
    QCOMPARE(panel_->colorBlockTextForTest(0), expected);
    QVERIFY(expected != QStringLiteral("C : 0  M : 0  Y : 0  K : 100"));
}

QTEST_MAIN(InfoPanelTest)
#include "tst_info_panel.moc"
