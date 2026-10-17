// The document-open loading indication (#667): every open entry point shows
// "Opening <file>…" with a wait cursor before the blocking read/decode, the
// indication clears when the open finishes (success or refusal), and a notice
// replaces it.

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QStatusBar>

#include "api/document_api.h"
#include "frame.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

// A tiny valid PSD written through the codec: new white RGB document, saved.
bool writePsd(const QString& path)
{
    PictureView view;
    return document_api::newDocument(view, 4, 4, QStringLiteral("rgb"), 8,
                                     QStringLiteral("white"))
        && document_api::save(view, path);
}

bool writePng(const QString& path)
{
    QImage image(4, 4, QImage::Format_ARGB32);
    image.fill(Qt::red);
    return image.save(path, "PNG");
}

// A minimal flat 1x1 CMYK PSD (the self-test's `color_mode_open` recipe) so the
// reopened view reports the CMYK conversion notice.
bool writeFlatCmykPsd(const QString& path)
{
    QByteArray bytes;
    const auto append16 = [&bytes](unsigned short value) {
        bytes.append(char((value >> 8) & 0xff));
        bytes.append(char(value & 0xff));
    };
    const auto append32 = [&bytes](unsigned int value) {
        bytes.append(char((value >> 24) & 0xff));
        bytes.append(char((value >> 16) & 0xff));
        bytes.append(char((value >> 8) & 0xff));
        bytes.append(char(value & 0xff));
    };
    bytes.append("8BPS", 4);
    append16(1);              // version
    bytes.append(6, char(0)); // reserved
    append16(4);              // channels
    append32(1);              // height
    append32(1);              // width
    append16(8);              // depth
    append16(4);              // color mode CMYK
    append32(0);              // color mode data
    append32(0);              // image resources
    append32(0);              // layer/mask section
    append16(0);              // raw compression
    bytes.append(char(128));  // C
    bytes.append(char(64));   // M
    bytes.append(char(32));   // Y
    bytes.append(char(200));  // K
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}

} // namespace

class OpenLoadingBannerTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void bannerShownForEachOpenKind();
    void bannerClearsOnSuccessAndRefusal();
    void noticeReplacesBanner();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QString capturedBannerText_;
    bool bannerVisibleDuringLoad_ = false;

    void captureOnOpen();
    void closeActive();
};

void OpenLoadingBannerTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void OpenLoadingBannerTest::captureOnOpen()
{
    capturedBannerText_.clear();
    bannerVisibleDuringLoad_ = false;
    connect(window_.get(), &PicturaMainWindow::documentOpenStarted, this,
            [this](const QString&) {
                capturedBannerText_ = window_->openBannerTextForTest();
                bannerVisibleDuringLoad_ = window_->openBannerVisibleForTest();
            });
}

void OpenLoadingBannerTest::closeActive()
{
    window_->closeDocument(window_->activeDocumentIndex(), false);
}

void OpenLoadingBannerTest::bannerShownForEachOpenKind()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString psd = dir.filePath(QStringLiteral("big.psd"));
    const QString png = dir.filePath(QStringLiteral("big.png"));
    QVERIFY(writePsd(psd));
    QVERIFY(writePng(png));

    captureOnOpen();

    // PSD/PSB goes through the native reader.
    QVERIFY(window_->openDocumentAtPath(psd));
    QCOMPARE(capturedBannerText_, QStringLiteral("Opening %1…").arg(QFileInfo(psd).fileName()));
    QVERIFY(bannerVisibleDuringLoad_);
    QVERIFY(QApplication::overrideCursor() == nullptr);
    QVERIFY(!window_->openBannerVisibleForTest());
    closeActive();

    // A raster goes through the Qt image import.
    QVERIFY(window_->openDocumentAtPath(png));
    QCOMPARE(capturedBannerText_, QStringLiteral("Opening %1…").arg(QFileInfo(png).fileName()));
    QVERIFY(bannerVisibleDuringLoad_);
    QVERIFY(QApplication::overrideCursor() == nullptr);
    QVERIFY(!window_->openBannerVisibleForTest());
    closeActive();

    // Open As Smart Object is its own entry point.
    QVERIFY(window_->openAsSmartObjectPath(png));
    QCOMPARE(capturedBannerText_, QStringLiteral("Opening %1…").arg(QFileInfo(png).fileName()));
    QVERIFY(bannerVisibleDuringLoad_);
    QVERIFY(QApplication::overrideCursor() == nullptr);
    QVERIFY(!window_->openBannerVisibleForTest());
    closeActive();
}

void OpenLoadingBannerTest::bannerClearsOnSuccessAndRefusal()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString png = dir.filePath(QStringLiteral("ok.png"));
    QVERIFY(writePng(png));

    QVERIFY(window_->openDocumentAtPath(png));
    QVERIFY2(!window_->openBannerVisibleForTest(),
             "the loading banner must be hidden after a successful open");
    QVERIFY(QApplication::overrideCursor() == nullptr);
    closeActive();

    const QString missing = dir.filePath(QStringLiteral("nope.png"));
    QVERIFY(!window_->openDocumentAtPath(missing));
    QVERIFY2(!window_->openBannerVisibleForTest(),
             "the loading banner must be hidden after a refused open");
    QVERIFY(QApplication::overrideCursor() == nullptr);
}

void OpenLoadingBannerTest::noticeReplacesBanner()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString cmyk = dir.filePath(QStringLiteral("cmyk.psd"));
    QVERIFY(writeFlatCmykPsd(cmyk));

    QVERIFY(window_->openDocumentAtPath(cmyk));
    QVERIFY(!window_->openBannerVisibleForTest());
    QCOMPARE(window_->statusBar()->currentMessage(),
             QStringLiteral("Converted from CMYK; saved as CMYK"));
    QVERIFY(QApplication::overrideCursor() == nullptr);
    closeActive();
}

QTEST_MAIN(OpenLoadingBannerTest)
#include "tst_open_loading_banner.moc"
