// Crop options bar (#282, #252): the ratio preset locks the box, the W/H fields
// show ratio values (pixels only in W x H x Resolution mode), the spirit-level
// toggle arms the straighten line tool, and Clear resets the ratio.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "panels/numeric_field.h"
#include "session.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>
#include <QtGui/QValidator>

class CropOptionsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void ratioPresetLocksTheBox();
    void ratioFieldsShowRatioValues();
    void ratioFieldsAcceptDecimals();
    void resolutionModeShowsPixels();
    void straightenToggleArmsLineMode();
    void contentAwareIsADisabledPlaceholder();
    void cropOptionsPersist();
    void overlayModesRenderDifferently();
    void integerFieldsRejectFractions();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    void drag(pictura::ImageView* canvas, const QPointF& from, const QPointF& to)
    {
        canvas->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(to);
        canvas->mouseReleased(to);
    }

    static void typeInto(pictura::NumericField* field, const QString& text)
    {
        auto* edit = field->findChild<QLineEdit*>();
        QVERIFY(edit);
        edit->clear();
        QTest::keyClicks(edit, text);
        QTest::keyClick(edit, Qt::Key_Return);
    }
};

void CropOptionsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void CropOptionsTest::ratioPresetLocksTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptRatio"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);  // Modern: start in preview
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* combo = frame.findChild<QComboBox*>(QStringLiteral("optionsCropRatio"));
    QVERIFY(combo);
    combo->setCurrentIndex(combo->findText(QStringLiteral("1 : 1 (Square)")));
    // Dragging inside the preview draws a new box honoring the square ratio.
    drag(canvas, QPointF(40, 40), QPointF(20, 10));
    QCOMPARE(tools->pendingCropRect(), QRect(20, 20, 20, 20));

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::ratioFieldsShowRatioValues()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptRatioFields"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* combo = frame.findChild<QComboBox*>(QStringLiteral("optionsCropRatio"));
    auto* width = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsCropWidth"));
    auto* height = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsCropHeight"));
    QVERIFY(combo);
    QVERIFY(width);
    QVERIFY(height);

    // A preset shows its ratio values, not the pixel size.
    combo->setCurrentIndex(combo->findText(QStringLiteral("16 : 9")));
    QCOMPARE(width->value(), 16.0);
    QCOMPARE(height->value(), 9.0);

    // The free Ratio default shows 1 : 1.
    combo->setCurrentIndex(0);
    QCOMPARE(width->value(), 1.0);
    QCOMPARE(height->value(), 1.0);

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::ratioFieldsAcceptDecimals()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptDecimals"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    frame.setActiveTool(pictura::ToolId::Crop);
    auto* width = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsCropWidth"));
    QVERIFY(width);
    // A typed decimal is accepted and kept (an integer field only rounds when
    // no decimal was typed).
    typeInto(width, QStringLiteral("2.5"));
    QCOMPARE(width->value(), 2.5);

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::resolutionModeShowsPixels()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptResolution"), 40, 24, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(canvas);
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->setCropClassicMode(false);
    tools->setCropRatio(0.0);
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* combo = frame.findChild<QComboBox*>(QStringLiteral("optionsCropRatio"));
    auto* width = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsCropWidth"));
    auto* height = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsCropHeight"));
    QVERIFY(combo);
    QVERIFY(width);
    QVERIFY(height);

    combo->setCurrentIndex(combo->findText(QStringLiteral("W x H x Resolution")));
    // The fields now show the box's pixel size (the full canvas).
    QCOMPARE(width->value(), 40.0);
    QCOMPARE(height->value(), 24.0);

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::straightenToggleArmsLineMode()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptStraighten"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const int doc = frame.activeDocumentIndex();
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* spirit = frame.findChild<QToolButton*>(QStringLiteral("optionsCropSpirit"));
    QVERIFY(spirit);

    QVERIFY(!tools->cropStraightenMode());
    spirit->click();
    QVERIFY(tools->cropStraightenMode());
    spirit->click();
    QVERIFY(!tools->cropStraightenMode());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::contentAwareIsADisabledPlaceholder()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptContentAware"), 32, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* contentAware =
        frame.findChild<QCheckBox*>(QStringLiteral("optionsCropContentAware"));
    QVERIFY(contentAware);
    QVERIFY(!contentAware->isEnabled());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::cropOptionsPersist()
{
    pictura::PicturaMainWindow& frame = *window_;
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);

    // Changing a crop option persists it through the session store.
    tools->setCropClassicMode(true);  // force a known start, then toggle
    tools->setCropClassicMode(false);
    tools->setCropGridOverlay(2);
    tools->setCropDeletePixels(false);
    tools->setCropRatio(1.5);

    const pictura::SessionState saved = pictura::loadSession();
    QCOMPARE(saved.cropClassicMode, false);
    QCOMPARE(saved.cropGridOverlay, 2);
    QCOMPARE(saved.cropDeletePixels, false);
    QCOMPARE(saved.cropRatio, 1.5);

    // Restore the defaults so the sibling tests are unaffected.
    tools->setCropClassicMode(true);
    tools->setCropGridOverlay(0);
    tools->setCropDeletePixels(true);
    tools->setCropRatio(0.0);
}

void CropOptionsTest::overlayModesRenderDifferently()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Overlay"), 200, 200, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    QVERIFY(canvas);
    canvas->fitOnScreen();
    canvas->setCropBox(QRectF(40, 40, 120, 120));
    canvas->setCropPreview(false);
    QCoreApplication::processEvents();

    // The selected overlay mode changes the guide geometry that is painted.
    canvas->setCropOverlay(0);
    QCoreApplication::processEvents();
    const QImage thirds = canvas->grab().toImage();
    canvas->setCropOverlay(2);
    QCoreApplication::processEvents();
    const QImage diagonal = canvas->grab().toImage();
    canvas->setCropOverlay(1);
    QCoreApplication::processEvents();
    const QImage grid = canvas->grab().toImage();

    QCOMPARE(canvas->cropOverlayForTest(), 1);
    QVERIFY2(thirds != diagonal, "diagonal overlay differs from rule of thirds");
    QVERIFY2(thirds != grid, "grid overlay differs from rule of thirds");
    canvas->clearCropBox();
}

void CropOptionsTest::integerFieldsRejectFractions()
{
    pictura::NumericFieldConfig plain;
    pictura::NumericField integer(QString(), plain);
    auto* iedit = integer.findChild<QLineEdit*>();
    QVERIFY(iedit);
    int pos = 0;
    QString fraction = QStringLiteral("2.5");
    QCOMPARE(iedit->validator()->validate(fraction, pos), QValidator::Invalid);

    pictura::NumericFieldConfig cfg;
    cfg.allowFractional = true;
    pictura::NumericField fractional(QString(), cfg);
    auto* fedit = fractional.findChild<QLineEdit*>();
    QVERIFY(fedit);
    QString fraction2 = QStringLiteral("2.5");
    QCOMPARE(fedit->validator()->validate(fraction2, pos), QValidator::Acceptable);
}

QTEST_MAIN(CropOptionsTest)
#include "tst_crop_options.moc"