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
#include <QtWidgets/QMenu>
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
    void resetRestoresCanvasBox();
    void straightenToggleArmsLineMode();
    void noContentAwareCheckbox();
    void cropOptionsPersist();
    void historyNamesTheOverlay();
    void overlayMenuShowsIcons();
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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);  // Modern: start in preview
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* combo = frame.findChild<QComboBox*>(QStringLiteral("optionsCropRatio"));
    QVERIFY(combo);
    combo->setCurrentIndex(combo->findText(QStringLiteral("1 : 1 (Square)")));
    // Dragging inside the preview draws a new box honoring the square ratio.
    drag(canvas, QPointF(30, 30), QPointF(10, 0));
    QCOMPARE(tools->pendingCropRect(), QRect(10, 10, 20, 20));

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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
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

    // The free Ratio default is unconstrained: the W/H boxes are blank.
    combo->setCurrentIndex(0);
    QVERIFY(width->isBlank());
    QVERIFY(height->isBlank());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::resetRestoresCanvasBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptReset"), 120, 120, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* combo = frame.findChild<QComboBox*>(QStringLiteral("optionsCropRatio"));
    auto* reset = frame.findChild<QToolButton*>(QStringLiteral("optionsCropReset"));
    auto* width = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsCropWidth"));
    auto* height = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsCropHeight"));
    QVERIFY(canvas && tools && combo && reset && width && height);
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Crop);
    drag(canvas, QPointF(30, 30), QPointF(0, 0));
    combo->setCurrentIndex(combo->findText(QStringLiteral("16 : 9")));

    // Reset makes the box the whole canvas and blanks the ratio fields.
    reset->click();
    QCOMPARE(tools->pendingCropRect(), QRect(0, 0, 120, 120));
    QCOMPARE(combo->currentIndex(), 0);
    QVERIFY(width->isBlank());
    QVERIFY(height->isBlank());

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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();

    canvas->actualPixels();
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setRatio(0.0);
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
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    const int doc = frame.activeDocumentIndex();
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* spirit = frame.findChild<QToolButton*>(QStringLiteral("optionsCropSpirit"));
    QVERIFY(spirit);
    // The icon carries its label to the right; a click on either toggles.
    QCOMPARE(spirit->text(), QStringLiteral("Straighten"));
    QCOMPARE(spirit->toolButtonStyle(), Qt::ToolButtonTextBesideIcon);
    QVERIFY(!spirit->icon().pixmap(18, 18).isNull());  // the spirit level

    QVERIFY(!tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->straightenMode());
    spirit->click();
    QVERIFY(tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->straightenMode());
    spirit->click();
    QVERIFY(!tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->straightenMode());

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// Content-Aware crop arrived after CS6 (CC 2015.5), so the bar has no checkbox.
void CropOptionsTest::noContentAwareCheckbox()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptContentAware"), 32, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(!frame.findChild<QCheckBox*>(QStringLiteral("optionsCropContentAware")));
    for (QCheckBox* box : frame.findChildren<QCheckBox*>()) {
        QVERIFY(box->text() != QStringLiteral("Content-Aware"));
    }

    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

// The commit's History state names the guide overlay: "Crop: Rule of Thirds",
// "Crop: Grid", and so on.
void CropOptionsTest::historyNamesTheOverlay()
{
    pictura::PicturaMainWindow& frame = *window_;
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int before = crop->gridOverlay();

    const QStringList& names = pictura::CropOptions::overlayNames();
    for (int i : {0, 1, 5}) {
        QVERIFY(frame.newDocument(QStringLiteral("OptHistory"), 40, 40, QStringLiteral("rgb"), 8,
                                  QStringLiteral("white")));
        const int doc = frame.activeDocumentIndex();
        pictura::PictureView* view = frame.activeView();
        pictura::ImageView* canvas = frame.imageView();
        QVERIFY(view && canvas);
        canvas->actualPixels();
        crop->setGridOverlay(i);
        frame.setActiveTool(pictura::ToolId::Crop);
        drag(canvas, QPointF(30, 30), QPointF(5, 5));
        QVERIFY(tools->cropCommitSession()->commitCrop());
        QCOMPARE(view->history_label(view->history_count() - 1),
                 QStringLiteral("Crop: ") + names.value(i));
        frame.setActiveTool(pictura::ToolId::Move);
        frame.closeDocument(doc, false);
    }
    QCOMPARE(names.value(0), QStringLiteral("Rule of Thirds"));
    QCOMPARE(names.value(1), QStringLiteral("Grid"));
    crop->setGridOverlay(before);
}

// Every overlay in the grid menu is a plain item with its icon, picked by
// clicking it (no check marks), and the menu button shows the selected
// overlay's icon however the overlay changes.
void CropOptionsTest::overlayMenuShowsIcons()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("OptIcons"), 32, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    const int doc = frame.activeDocumentIndex();
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());
    auto* crop = tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop);
    const int before = crop->gridOverlay();
    frame.setActiveTool(pictura::ToolId::Crop);
    auto* button = frame.findChild<QToolButton*>(QStringLiteral("optionsCropGrid"));
    QVERIFY(button && button->menu());

    const auto image = [](const QIcon& i) { return i.pixmap(QSize(18, 18)).toImage(); };
    for (QAction* a : button->menu()->actions()) {
        QVERIFY2(!a->isCheckable(), qPrintable(a->text()));
    }
    const QStringList& names = pictura::CropOptions::overlayNames();
    QList<QImage> seen;
    for (int i = 0; i < names.size(); ++i) {
        QAction* action = nullptr;
        for (QAction* a : button->menu()->actions()) {
            if (a->text() == names[i]) {
                action = a;
            }
        }
        QVERIFY2(action, qPrintable(names[i]));
        QVERIFY2(!action->icon().isNull(), qPrintable(names[i]));
        const QImage art = image(action->icon());
        QVERIFY2(!seen.contains(art), "each overlay has its own icon");
        seen.append(art);
        action->trigger();
        QCOMPARE(crop->gridOverlay(), i);
        QCOMPARE(image(button->icon()), art);
    }
    crop->setGridOverlay(1);
    QCOMPARE(image(button->icon()), seen.value(1));

    crop->setGridOverlay(before);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
}

void CropOptionsTest::cropOptionsPersist()
{
    pictura::PicturaMainWindow& frame = *window_;
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->cropCommitSession());

    // Changing a crop option persists it through the session store.
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(true);  // force a known start, then toggle
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setGridOverlay(2);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setDeletePixels(false);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setRatio(1.5);

    const pictura::SessionState saved = pictura::loadSession();
    QCOMPARE(saved.cropClassicMode, false);
    QCOMPARE(saved.cropGridOverlay, 2);
    QCOMPARE(saved.cropDeletePixels, false);
    QCOMPARE(saved.cropRatio, 1.5);

    // Restore the defaults so the sibling tests are unaffected.
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setClassicMode(false);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setGridOverlay(0);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setDeletePixels(true);
    tools->optionsFor<pictura::CropOptions>(pictura::ToolId::Crop)->setRatio(0.0);
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