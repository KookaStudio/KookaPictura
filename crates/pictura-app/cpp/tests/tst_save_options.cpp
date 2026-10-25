// Save As Embed Color Profile: defaults, the written profile bytes, and the
// per-document choice.

#include <QtTest/QtTest>

#include <QtGui/QImage>

#include "frame.h"
#include "save_options_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"
#include "pictura_app/src/cxxqt_object/export.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

class SaveOptionsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void taggedDefaultsOn();
    void untaggedDefaultsOff();
    void pngNoOption();
    void cancel();
    void psdOn();
    void psdOff();
    void jpegApp2();
    void tiffTag();
    void tiffOff();
    void cmykSourceNotMistagged();
    void saveKeepsChoice();
    void saveDefaultsOn();
    void workingProfilePixels();
    void exportAsSrgb();
    void psbOn();
    void unsupportedFormats();

private:
    pictura::PictureView* newView(const QColor& color, bool adobe);
    QString path(const char* name) const { return dir_.filePath(QString::fromLatin1(name)); }
    QString profileKind(const QString& file) const
    {
        return pictura::probe_file_profile(file).value(0);
    }
    void closeActive() { window_->closeDocument(window_->activeDocumentIndex(), false); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void SaveOptionsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

pictura::PictureView* SaveOptionsTest::newView(const QColor& color, bool adobe)
{
    if (!window_->newDocument(QStringLiteral("Save"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, color.rgba(), -1, QStringLiteral("normal"), 100, false);
    view->deselect();
    if (adobe && !view->assign_profile(1)) {
        return nullptr;
    }
    return view;
}

void SaveOptionsTest::taggedDefaultsOn()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    QVERIFY(pictura::default_save_embed_profile(*view, true));
    closeActive();
}

void SaveOptionsTest::untaggedDefaultsOff()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), false);
    QVERIFY(view);
    QVERIFY(!pictura::default_save_embed_profile(*view, true));
    closeActive();
}

void SaveOptionsTest::pngNoOption()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, true);
    const QString file = path("no-option.png");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("missing"));
    closeActive();
}

void SaveOptionsTest::cancel()
{
    pictura::SaveOptionsDialog dialog(true, true);
    dialog.reject();
    QCOMPARE(dialog.result(), int(QDialog::Rejected));
}

void SaveOptionsTest::psdOn()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, true);
    const QString file = path("on.psd");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("mismatch"));
    closeActive();
}

void SaveOptionsTest::psdOff()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, false);
    const QString file = path("off.psd");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("missing"));
    closeActive();
}

void SaveOptionsTest::jpegApp2()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, true);
    const QString file = path("embedded.jpg");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("mismatch"));
    closeActive();
}

void SaveOptionsTest::tiffTag()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, true);
    const QString file = path("embedded.tif");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("mismatch"));
    closeActive();
}

void SaveOptionsTest::tiffOff()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, false);
    const QString file = path("untagged.tif");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("missing"));
    closeActive();
}

void SaveOptionsTest::cmykSourceNotMistagged()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), false);
    QVERIFY(view);
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("cmyk")));
    pictura::set_save_embed_profile(*view, true);
    const QString file = path("cmyk-source.tif");
    QVERIFY(view->save(file));
    QVERIFY2(profileKind(file) != QStringLiteral("mismatch"),
             "a CMYK source must not embed its CMYK profile in a TIFF");
    closeActive();
}

void SaveOptionsTest::saveKeepsChoice()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, false);
    QVERIFY(view->save(path("first.psd")));
    QVERIFY(view->save(path("second.psd")));
    QCOMPARE(profileKind(path("second.psd")), QStringLiteral("missing"));
    closeActive();
}

void SaveOptionsTest::saveDefaultsOn()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    const QString file = path("default.psd");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("mismatch"));
    closeActive();
}

void SaveOptionsTest::workingProfilePixels()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    const QRgb working = view->sample_argb(2, 2);
    pictura::set_save_embed_profile(*view, true);
    const QString tagged = path("working.jpg");
    QVERIFY(view->save(tagged));
    const QImage taggedImage(tagged);
    QVERIFY(!taggedImage.isNull());
    const QRgb decoded = taggedImage.pixel(2, 2);
    for (int shift : {16, 8, 0}) {
        const int a = (working >> shift) & 0xFF;
        const int b = (decoded >> shift) & 0xFF;
        QVERIFY2(std::abs(a - b) <= 8, "JPEG keeps the working RGB samples");
    }
    pictura::set_save_embed_profile(*view, false);
    const QString untagged = path("working-untagged.jpg");
    QVERIFY(view->save(untagged));
    QCOMPARE(profileKind(untagged), QStringLiteral("missing"));
    const QImage untaggedImage(untagged);
    QVERIFY(!untaggedImage.isNull());
    QCOMPARE(untaggedImage.pixel(2, 2), decoded);
    closeActive();
}

void SaveOptionsTest::exportAsSrgb()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    const QString file = path("export.jpg");
    QVERIFY(pictura::export_image(*view, file, QStringLiteral("JPG"), 90, 100));
    QCOMPARE(profileKind(file), QStringLiteral("missing"));
    const QImage exported(file);
    QVERIFY(!exported.isNull());
    closeActive();
}

void SaveOptionsTest::psbOn()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, true);
    const QString file = path("on.psb");
    QVERIFY(view->save(file));
    QCOMPARE(profileKind(file), QStringLiteral("mismatch"));
    closeActive();
}

void SaveOptionsTest::unsupportedFormats()
{
    pictura::PictureView* view = newView(QColor(30, 120, 220), true);
    QVERIFY(view);
    pictura::set_save_embed_profile(*view, true);
    for (const char* name : {"no-option.png", "no-option.webp", "no-option.bmp"}) {
        const QString file = path(name);
        QVERIFY(view->save(file));
        QCOMPARE(profileKind(file), QStringLiteral("missing"));
    }
    closeActive();
}

QTEST_MAIN(SaveOptionsTest)
#include "tst_save_options.moc"
