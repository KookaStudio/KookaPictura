// Blend Text Colors Using Gamma in Color Settings and missing fonts (#685): the
// advanced controls, their effect and persistence, the missing-font check, the
// alert on open, the Layers-row mark, and Type > Replace All Missing Fonts.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/type_api.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "color_settings_dialog.h"
#include "commands.h"
#include "panels/layers_panel.h"
#include "selftest_paint_fixture.h"
#include "session.h"
#include "type_fonts.h"

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QFontDatabase>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>

#include <memory>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

const QString kBundled = QStringLiteral("Liberation Sans");
const QString kMissing = QStringLiteral("NoSuchFontPictura");

QImage seed()
{
    QImage image(360, 240, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QString replaceCommand()
{
    return pictura::commandIdForPath(
        {QStringLiteral("Type"), QStringLiteral("Replace All Missing Fonts")});
}

// Commit `text` in `font` as black point type at (30, y).
QString addType(Fixture& f, const QString& text, const QString& font, double y = 120.0)
{
    pictura::TypeSetting setting{};
    setting.x = 30.0;
    setting.y = y;
    setting.xx = 1.0;
    setting.yy = 1.0;
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = 40.0;
    const QString path = pictura::type_api::commitLayer(
        *f.view, text, font, setting, character, pictura::type_api::defaultParagraphSetting());
    f.frame.selectLayerPath(path);
    f.frame.refresh();
    return path;
}

// The red channel summed over the composite under the type.
qint64 ink(const Fixture& f)
{
    qint64 sum = 0;
    for (int y = 60; y < 140; ++y) {
        for (int x = 20; x < 340; ++x) {
            sum += (f.view->composite_argb(x, y) >> 16) & 0xff;
        }
    }
    return sum;
}

QList<QMessageBox*> alerts(QWidget& window)
{
    return window.findChildren<QMessageBox*>(QStringLiteral("missingFontsAlert"));
}

} // namespace

class TextGammaFontsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void colorSettingsDefaults();
    void colorSettingsAppliesGamma();
    void colorSettingsAppliesValue();
    void gammaSessionRoundTrip();
    void detectsMissingFonts();
    void openAlertsMissingFonts();
    void openWithoutMissingFontsIsQuiet();
    void layersRowMarksMissingFont();
    void replaceAllMissingFonts();
    void replaceDisabledWithoutMissingFonts();
    void replaceDisabledOutsideRgb8();

private:
    // Accept Color Settings with Blend Text Colors Using Gamma `enabled` and,
    // when positive, the gamma `value`.
    void runColorSettings(bool enabled, double value = 0.0);
    // Open Color Settings, read its text gamma controls, and cancel.
    QPair<bool, double> readColorSettings();
    // Save `f` as a PSD, open it, close it again, and return how many
    // missing-font alerts the open raised; -1 when it could not.
    int openAsPsd(Fixture& f, const QString& name);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TextGammaFontsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void TextGammaFontsTest::cleanup()
{
    for (QMessageBox* box : alerts(*window_)) {
        box->close();
    }
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
}

void TextGammaFontsTest::runColorSettings(bool enabled, double value)
{
    QTimer::singleShot(0, window_.get(), [this, enabled, value]() {
        auto* dialog = window_->findChild<pictura::ColorSettingsDialog*>();
        QVERIFY(dialog);
        dialog->findChild<QCheckBox*>(QStringLiteral("colorSettingsBlendTextGamma"))
            ->setChecked(enabled);
        if (value > 0.0) {
            dialog->findChild<QDoubleSpinBox*>(QStringLiteral("colorSettingsTextGamma"))
                ->setValue(value);
        }
        dialog->accept();
    });
    window_->registry()->refresh();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::EditColorSettings)));
}

QPair<bool, double> TextGammaFontsTest::readColorSettings()
{
    QPair<bool, double> shown{false, 0.0};
    QTimer::singleShot(0, window_.get(), [this, &shown]() {
        auto* dialog = window_->findChild<pictura::ColorSettingsDialog*>();
        QVERIFY(dialog);
        shown = {dialog->textGammaEnabled(), dialog->textGamma()};
        dialog->reject();
    });
    window_->registry()->refresh();
    window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditColorSettings));
    return shown;
}

int TextGammaFontsTest::openAsPsd(Fixture& f, const QString& name)
{
    QTemporaryDir dir;
    const QString path = dir.filePath(name + QStringLiteral(".psd"));
    if (!pictura::document_api::save(*f.view, path) || !window_->openDocumentAtPath(path)) {
        return -1;
    }
    QCoreApplication::processEvents();
    const int count = int(alerts(*window_).size());
    window_->closeDocument(window_->activeDocumentIndex(), false);
    return count;
}

void TextGammaFontsTest::colorSettingsDefaults()
{
    const pictura::SessionState fresh;
    QVERIFY(fresh.textGammaEnabled);
    QCOMPARE(fresh.textGamma, 1.45);
    pictura::ColorSettingsDialog dialog(0);
    dialog.show();
    auto* advanced = dialog.findChild<QGroupBox*>(QStringLiteral("colorSettingsAdvanced"));
    auto* check = dialog.findChild<QCheckBox*>(QStringLiteral("colorSettingsBlendTextGamma"));
    auto* value = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("colorSettingsTextGamma"));
    QVERIFY(advanced && check && value);
    QVERIFY(!advanced->isVisible());
    dialog.findChild<QPushButton*>(QStringLiteral("colorSettingsMoreOptions"))->click();
    QVERIFY(advanced->isVisible());
    QVERIFY(check->isChecked());
    QCOMPARE(value->value(), 1.45);
    QCOMPARE(value->minimum(), 1.0);
    QCOMPARE(value->maximum(), 2.2);
    check->setChecked(false);
    QVERIFY(!value->isEnabled());
}

void TextGammaFontsTest::colorSettingsAppliesGamma()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_text_gamma_apply"));
    QVERIFY2(f.ok(), "gamma fixture");
    addType(f, QStringLiteral("Gamma"), kBundled);
    const qint64 on = ink(f);
    const int base = f.view->history_index();
    runColorSettings(false);
    const qint64 off = ink(f);
    QVERIFY2(off < on, "with the gamma off the type's edges are darker");
    QCOMPARE(f.view->history_index(), base);
    QVERIFY(!pictura::loadSession().textGammaEnabled);
    QCOMPARE(readColorSettings(), qMakePair(false, 1.45));
    QCOMPARE(ink(f), off);
    runColorSettings(true);
    QCOMPARE(ink(f), on);
    QVERIFY(pictura::loadSession().textGammaEnabled);
}

void TextGammaFontsTest::colorSettingsAppliesValue()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_text_gamma_value"));
    QVERIFY2(f.ok(), "gamma fixture");
    addType(f, QStringLiteral("Gamma"), kBundled);
    const qint64 at145 = ink(f);
    const int base = f.view->history_index();
    runColorSettings(true, 1.8);
    const qint64 at180 = ink(f);
    QVERIFY2(at180 < at145, "a gamma nearer 2.2 thins the type less");
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(pictura::loadSession().textGamma, 1.8);
    QCOMPARE(readColorSettings(), qMakePair(true, 1.8));
    runColorSettings(true, 1.45);
    QCOMPARE(ink(f), at145);
}

void TextGammaFontsTest::gammaSessionRoundTrip()
{
    pictura::SessionState state = pictura::loadSession();
    state.textGammaEnabled = false;
    state.textGamma = 1.8;
    QVERIFY(pictura::saveSession(state));
    const pictura::SessionState loaded = pictura::loadSession();
    QVERIFY(!loaded.textGammaEnabled);
    QCOMPARE(loaded.textGamma, 1.8);
    state.textGammaEnabled = true;
    state.textGamma = 1.45;
    QVERIFY(pictura::saveSession(state));
}

void TextGammaFontsTest::detectsMissingFonts()
{
    QVERIFY(pictura::fontIsInstalled(kBundled));
    QVERIFY(pictura::fontIsInstalled(QStringLiteral("LiberationSans-Bold")));
    QVERIFY(!pictura::fontIsInstalled(kMissing));
    const QStringList families = QFontDatabase::families();
    QVERIFY2(!families.isEmpty(), "the test host has an installed font");
    const QString installed = families.first();
    QVERIFY(pictura::fontIsInstalled(installed));
    const QString postscript
        = QString(installed).remove(QLatin1Char(' ')) + QStringLiteral("-Regular");
    QVERIFY(pictura::fontIsInstalled(postscript));
    QCOMPARE(pictura::missingFonts({kBundled, kMissing, installed, kMissing}),
             QStringList{kMissing});
}

void TextGammaFontsTest::openAlertsMissingFonts()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_missing_alert"));
    QVERIFY2(f.ok(), "missing-font fixture");
    addType(f, QStringLiteral("One"), kMissing, 80.0);
    addType(f, QStringLiteral("Two"), kMissing, 180.0);
    QTemporaryDir dir;
    const QString path = dir.filePath(QStringLiteral("missing.psd"));
    QVERIFY(pictura::document_api::save(*f.view, path));
    QVERIFY(window_->openDocumentAtPath(path));
    QCoreApplication::processEvents();
    const QList<QMessageBox*> boxes = alerts(*window_);
    QCOMPARE(boxes.size(), 1);
    QCOMPARE(boxes.first()->text().count(kMissing), 1);
    window_->closeDocument(window_->activeDocumentIndex(), false);
}

void TextGammaFontsTest::openWithoutMissingFontsIsQuiet()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_missing_quiet"));
    QVERIFY2(f.ok(), "missing-font fixture");
    addType(f, QStringLiteral("Here"), kBundled);
    QCOMPARE(openAsPsd(f, QStringLiteral("quiet")), 0);
}

void TextGammaFontsTest::layersRowMarksMissingFont()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_missing_row"));
    QVERIFY2(f.ok(), "missing-font fixture");
    const QString missing = addType(f, QStringLiteral("Gone"), kMissing, 80.0);
    const QString present = addType(f, QStringLiteral("Here"), kBundled, 180.0);
    pictura::LayersPanel* panel = window_->layersPanelForTest();
    QVERIFY(panel);
    QTRY_VERIFY(panel->rowMissingFontForTest(missing));
    QVERIFY(!panel->rowMissingFontForTest(present));
}

void TextGammaFontsTest::replaceAllMissingFonts()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_missing_replace"));
    QVERIFY2(f.ok(), "missing-font fixture");
    const QString first = addType(f, QStringLiteral("One"), kMissing, 80.0);
    const QString second = addType(f, QStringLiteral("Two"), kMissing, 180.0);
    const int base = f.view->history_index();
    window_->registry()->refresh();
    QVERIFY(window_->registry()->action(replaceCommand())->isEnabled());
    QVERIFY(window_->registry()->dispatch(replaceCommand()));
    QVERIFY(f.committedOnce(base, "Replace All Missing Fonts"));
    QCOMPARE(pictura::type_api::layerFont(*f.view, first), kBundled);
    QCOMPARE(pictura::type_api::layerFont(*f.view, second), kBundled);
    QCOMPARE(pictura::type_api::layerText(*f.view, first), QStringLiteral("One"));
    QCOMPARE(pictura::type_api::layerText(*f.view, second), QStringLiteral("Two"));
    QTRY_VERIFY(!window_->layersPanelForTest()->rowMissingFontForTest(first));
    QVERIFY(!window_->layersPanelForTest()->rowMissingFontForTest(second));
    QVERIFY(f.view->undo());
    QCOMPARE(pictura::type_api::layerFont(*f.view, first), kMissing);
    QCOMPARE(pictura::type_api::layerFont(*f.view, second), kMissing);
}

void TextGammaFontsTest::replaceDisabledWithoutMissingFonts()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_missing_disabled"));
    QVERIFY2(f.ok(), "missing-font fixture");
    addType(f, QStringLiteral("Here"), kBundled);
    window_->registry()->refresh();
    QVERIFY(!window_->registry()->action(replaceCommand())->isEnabled());
}

void TextGammaFontsTest::replaceDisabledOutsideRgb8()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_missing_gray"));
    QVERIFY2(f.ok(), "missing-font fixture");
    const QString path = addType(f, QStringLiteral("Gray"), kMissing);
    QVERIFY(pictura::convert_image_mode(*f.view, QStringLiteral("grayscale")));
    QVERIFY2(pictura::type_api::isType(*f.view, path), "conversion keeps the type layer");
    QVERIFY(!pictura::type_api::layersEditable(*f.view));
    window_->registry()->refresh();
    QVERIFY(!window_->registry()->action(replaceCommand())->isEnabled());
    QCOMPARE(pictura::type_api::replaceFonts(*f.view, {kMissing}), 0);
    QCOMPARE(pictura::type_api::layerFont(*f.view, path), kMissing);
}

QTEST_MAIN(TextGammaFontsTest)
#include "tst_text_gamma_fonts.moc"
