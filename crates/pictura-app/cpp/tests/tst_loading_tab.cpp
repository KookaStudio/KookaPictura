// The document-open loading tab (#729): every open inserts a tab at once, the
// decode/composite/history run off the GUI thread, the tab's close button is an
// animated spinner that cancels, and completion fills, removes, or restores the
// tab. The delay seam holds a real open in flight so the tab, spinner, hover,
// click, and middle-click are observable.

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QStandardPaths>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QEnterEvent>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QTabBar>

#include "api/document_api.h"
#include "frame.h"
#include "image_view.h"
#include "loading_card.h"
#include "tab_spinner.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

using namespace pictura;

namespace {

constexpr int kOpenDelayMs = 150;
constexpr int kSpinnerDelayMs = 600;

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

// A minimal flat 1x1 CMYK PSD so the reopened view reports the CMYK notice.
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
    append16(1);
    bytes.append(6, char(0));
    append16(4);
    append32(1);
    append32(1);
    append16(8);
    append16(4);
    append32(0);
    append32(0);
    append32(0);
    append16(0);
    bytes.append(char(128));
    bytes.append(char(64));
    bytes.append(char(32));
    bytes.append(char(200));
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}

bool haveGhostscript()
{
    return !QStandardPaths::findExecutable(QStringLiteral("gs")).isEmpty();
}

QByteArray pdf(const QList<QByteArray>& objects)
{
    QByteArray out("%PDF-1.7\n");
    QList<qsizetype> offsets;
    for (int i = 0; i < objects.size(); ++i) {
        offsets.append(out.size());
        out += QByteArray::number(i + 1) + " 0 obj\n" + objects[i] + "\nendobj\n";
    }
    const qsizetype xref = out.size();
    out += "xref\n0 " + QByteArray::number(objects.size() + 1) + "\n0000000000 65535 f \n";
    for (qsizetype offset : offsets) {
        out += QByteArray::number(offset).rightJustified(10, '0') + " 00000 n \n";
    }
    out += "trailer\n<< /Size " + QByteArray::number(objects.size() + 1)
           + " /Root 1 0 R >>\nstartxref\n" + QByteArray::number(xref) + "\n%%EOF\n";
    return out;
}

QByteArray stream(const QByteArray& dict, const QByteArray& data)
{
    return "<< " + dict + " /Length " + QByteArray::number(data.size()) + " >>\nstream\n" + data
           + "\nendstream";
}

QByteArray pages(const QList<QPair<QByteArray, QByteArray>>& list)
{
    QList<QByteArray> objects = {"<< /Type /Catalog /Pages 2 0 R >>", QByteArray()};
    QByteArray kids;
    for (const auto& [media, content] : list) {
        const int page = int(objects.size()) + 1;
        kids += QByteArray::number(page) + " 0 R ";
        objects.append("<< /Type /Page /Parent 2 0 R /MediaBox [" + media + "] /Contents "
                       + QByteArray::number(page + 1) + " 0 R >>");
        objects.append(stream(QByteArray(), content));
    }
    objects[1] = "<< /Type /Pages /Kids [" + kids + "] /Count "
                 + QByteArray::number(list.size()) + " >>";
    return pdf(objects);
}

QByteArray genericPdf()
{
    return pages({{"0 0 144 72", "1 0 0 rg 20 20 60 30 re f"}});
}

} // namespace

class LoadingTabTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void init();
    void cleanup();
    void cleanupTestCase();

    void loadingTabShowsWhileWorkerRuns();
    void openInsertsLoadingTab();
    void backgroundOpenKeepsCurrentDocument();
    void firstOpenShowsEmptyWorkspace();
    void busyCursorSpansConcurrentOpens();
    void genericPdfProbeHasNoTab();
    void entryPointsShareThePath();
    void spinnerAnimates();
    void progressCardShowsPhases();
    void progressCardCancelCancels();
    void hoverSwapsToClose();
    void clickingSpinnerCancels();
    void middleClickCancels();
    void middleClickClosesNormalTab();
    void closeButtonClosesTheDocument();
    void imageTabHasCloseButton();
    void longTitlesKeepTheCloseButton();
    void tooltipShowsFullUntitledLabel();
    void tooltipShowsFullSavedLabel();
    void cancelledResultIsDiscarded();
    void successFillsTabAndActivates();
    void userSelectedTabIsNotStolen();
    void successKeepsStatusNotice();
    void failureRemovesTab();
    void cancelIsNotFailure();
    void bannerIsGone();

private:
    QTabBar* tabBar() const;
    QString fullLabel(const QString& name) const;
    TabSpinner* spinnerAt(int index) const;
    QString makePsd(const QTemporaryDir& dir, const QString& name) const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

QTabBar* LoadingTabTest::tabBar() const
{
    return window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
}

// The unelided label of the active 8-bit RGB document: name, zoom, mode/bits.
QString LoadingTabTest::fullLabel(const QString& name) const
{
    ImageView* canvas = window_->imageView();
    if (!canvas) {
        return QString();
    }
    return QStringLiteral("%1 @ %2% (RGB/8)").arg(name).arg(qRound(canvas->zoom() * 100.0));
}

TabSpinner* LoadingTabTest::spinnerAt(int index) const
{
    QTabBar* bar = tabBar();
    return bar ? qobject_cast<TabSpinner*>(bar->tabButton(index, QTabBar::RightSide)) : nullptr;
}

QString LoadingTabTest::makePsd(const QTemporaryDir& dir, const QString& name) const
{
    const QString path = dir.filePath(name);
    return writePsd(path) ? path : QString();
}

void LoadingTabTest::initTestCase()
{
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QTest::qWait(0);
}

void LoadingTabTest::init()
{
    window_->setOpenDelayForTest(kOpenDelayMs);
    window_->statusBar()->clearMessage();
}

void LoadingTabTest::cleanup()
{
    while (QApplication::overrideCursor()) {
        QApplication::restoreOverrideCursor();
    }
    window_->cancelAllOpens();
    while (window_->documentCount() > 0) {
        window_->closeDocument(0, false);
    }
    window_->setImportDialogDriverForTest({});
}

void LoadingTabTest::cleanupTestCase()
{
    window_.reset();
}

void LoadingTabTest::loadingTabShowsWhileWorkerRuns()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("slow.psd"));
    QVERIFY(!path.isEmpty());

    window_->setOpenDelayForTest(kSpinnerDelayMs);
    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QVERIFY(window_->imageView() == nullptr);
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(!window_->activeView()->has_document());

    TabSpinner* spinner = spinnerAt(0);
    QVERIFY(spinner != nullptr);
    const int angle = spinner->angleForTest();
    QTRY_VERIFY2(spinner->angleForTest() != angle, "the spinner advances while the worker runs");

    QTRY_VERIFY(!window_->isLoadingDocument(0));
    QVERIFY(window_->activeView()->has_document());
}

void LoadingTabTest::openInsertsLoadingTab()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString psd = makePsd(dir, QStringLiteral("tab.psd"));
    const QString png = dir.filePath(QStringLiteral("tab.png"));
    QVERIFY(!psd.isEmpty());
    QVERIFY(writePng(png));

    window_->startOpenDocumentAtPath(psd);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    QCOMPARE(window_->documentName(0), QStringLiteral("tab.psd"));
    QTRY_VERIFY(!window_->isLoadingDocument(0));
    window_->closeDocument(0, false);

    window_->startOpenDocumentAtPath(png);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    QCOMPARE(window_->documentName(0), QStringLiteral("tab.png"));
    QVERIFY(window_->imageView() == nullptr);
    QTRY_VERIFY(!window_->isLoadingDocument(0));
}

void LoadingTabTest::backgroundOpenKeepsCurrentDocument()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = makePsd(dir, QStringLiteral("a.psd"));
    const QString second = makePsd(dir, QStringLiteral("b.psd"));
    QVERIFY(!first.isEmpty() && !second.isEmpty());

    QVERIFY(window_->openPath(first));
    window_->startOpenPath(second);
    QTRY_VERIFY(window_->isLoadingDocument(1));
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->documentName(0), QStringLiteral("a.psd"));
    QCOMPARE(window_->documentName(1), QStringLiteral("b.psd"));

    QTRY_VERIFY(!window_->isLoadingDocument(1));
    QCOMPARE(window_->activeDocumentIndex(), 1);
    QCOMPARE(window_->documentName(1), QStringLiteral("b.psd"));
}

void LoadingTabTest::firstOpenShowsEmptyWorkspace()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("only.psd"));
    QVERIFY(!path.isEmpty());

    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QVERIFY(window_->imageView() == nullptr);
    QCOMPARE(window_->documentCount(), 1);
    QTRY_VERIFY(!window_->isLoadingDocument(0));
    QVERIFY(window_->imageView() != nullptr);
}

void LoadingTabTest::busyCursorSpansConcurrentOpens()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = makePsd(dir, QStringLiteral("one.psd"));
    const QString second = makePsd(dir, QStringLiteral("two.psd"));
    QVERIFY(!first.isEmpty() && !second.isEmpty());

    window_->setOpenDelayForTest(150);
    window_->startOpenPath(first);
    window_->setOpenDelayForTest(700);
    window_->startOpenPath(second);
    QTRY_VERIFY(window_->isLoadingDocument(0) && window_->isLoadingDocument(1));
    QVERIFY(QApplication::overrideCursor() != nullptr);

    QTRY_VERIFY(!window_->isLoadingDocument(0));
    QVERIFY2(QApplication::overrideCursor() != nullptr,
             "the cursor stays busy while another open is in flight");
    QTRY_VERIFY(!window_->isLoadingDocument(1));
    QVERIFY(QApplication::overrideCursor() == nullptr);
}

void LoadingTabTest::genericPdfProbeHasNoTab()
{
    if (!haveGhostscript()) {
        QSKIP("Ghostscript (gs) is not installed");
    }
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("generic.pdf"));
    {
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        QVERIFY(file.write(genericPdf()) > 0);
    }

    bool dialogHadNoTab = false;
    bool dialogHadNoCard = false;
    bool renderHadTab = false;
    bool renderHadCard = false;
    window_->setImportDialogDriverForTest([&](QDialog&) {
        dialogHadNoTab = window_->documentCount() == 0;
        dialogHadNoCard = window_->findChild<LoadingCard*>() == nullptr;
        QTimer::singleShot(50, window_.get(), [&] {
            renderHadTab = window_->documentCount() == 1 && window_->isLoadingDocument(0);
            renderHadCard = window_->findChild<LoadingCard*>() != nullptr;
        });
        return true;
    });

    window_->startOpenDocumentAtPath(path);
    QTRY_VERIFY(!window_->isLoadingDocument(0) && window_->documentCount() == 1);
    QVERIFY2(dialogHadNoTab, "the probe and dialog create no loading tab");
    QVERIFY2(dialogHadNoCard, "the probe and dialog create no progress card");
    QVERIFY2(renderHadTab, "a chosen page render creates a loading tab");
    QVERIFY2(renderHadCard, "a chosen page render creates a progress card");
    QVERIFY(window_->activeView()->has_document());
}

void LoadingTabTest::entryPointsShareThePath()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString psd = makePsd(dir, QStringLiteral("entry.psd"));
    const QString png = dir.filePath(QStringLiteral("entry.png"));
    QVERIFY(!psd.isEmpty());
    QVERIFY(writePng(png));

    const QList<std::function<quint64()>> starts = {
        [&] { return window_->startOpenDocumentAtPath(psd); },
        [&] { return window_->startOpenDocumentAtPath(png); },
        [&] { return window_->startOpenAsSmartObjectPath(png); },
    };
    for (const auto& start : starts) {
        start();
        QTRY_VERIFY(window_->isLoadingDocument(window_->documentCount() - 1));
        window_->closeDocument(window_->documentCount() - 1, false);
        QTRY_VERIFY(window_->documentCount() == 0);
    }
}

void LoadingTabTest::spinnerAnimates()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("spin.psd"));
    QVERIFY(!path.isEmpty());

    window_->setOpenDelayForTest(kSpinnerDelayMs);
    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    TabSpinner* spinner = spinnerAt(0);
    QVERIFY(spinner != nullptr);
    const int first = spinner->angleForTest();
    QTRY_VERIFY(spinner->angleForTest() != first);
    window_->closeDocument(0, false);
}

void LoadingTabTest::progressCardShowsPhases()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("progress.psd"));
    QVERIFY(!path.isEmpty());

    window_->setOpenDelayForTest(kSpinnerDelayMs);
    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    LoadingCard* card = window_->findChild<LoadingCard*>();
    QVERIFY(card != nullptr);
    QVERIFY(card->isVisible());
    QTRY_COMPARE(card->progressForTest(), 90);
    QCOMPARE(card->phaseForTest(), QStringLiteral("Preparing layers…"));
    QCOMPARE(card->percentForTest(), QStringLiteral("90%"));
    QVERIFY(card->findChild<QPushButton*>(QStringLiteral("loadingCardCancel")) != nullptr);

    QTRY_VERIFY(!window_->isLoadingDocument(0));
    QTRY_VERIFY(window_->findChild<LoadingCard*>() == nullptr);
}

void LoadingTabTest::progressCardCancelCancels()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = makePsd(dir, QStringLiteral("card-a.psd"));
    const QString second = makePsd(dir, QStringLiteral("card-b.psd"));
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    QVERIFY(window_->openPath(first));

    window_->setOpenDelayForTest(kSpinnerDelayMs);
    window_->startOpenPath(second);
    QTRY_VERIFY(window_->isLoadingDocument(1));
    LoadingCard* card = window_->findChild<LoadingCard*>();
    QVERIFY(card != nullptr);
    auto* cancel = card->findChild<QPushButton*>(QStringLiteral("loadingCardCancel"));
    QVERIFY(cancel != nullptr);
    QTest::mouseClick(cancel, Qt::LeftButton);
    QTRY_COMPARE(window_->documentCount(), 1);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->documentName(0), QStringLiteral("card-a.psd"));
    // negative window: proves the held worker's late result adds nothing
    QTest::qWait(kOpenDelayMs + 150);
    QCOMPARE(window_->documentCount(), 1);
}

void LoadingTabTest::hoverSwapsToClose()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("hover.psd"));
    QVERIFY(!path.isEmpty());

    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    TabSpinner* spinner = spinnerAt(0);
    QVERIFY(spinner != nullptr);
    QVERIFY(!spinner->hoveringForTest());

    QEnterEvent enter(QPointF(2, 2), QPointF(2, 2), QPointF(2, 2));
    QApplication::sendEvent(spinner, &enter);
    QVERIFY(spinner->hoveringForTest());
    QEvent leave(QEvent::Leave);
    QApplication::sendEvent(spinner, &leave);
    QVERIFY(!spinner->hoveringForTest());
    window_->closeDocument(0, false);
}

void LoadingTabTest::clickingSpinnerCancels()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = makePsd(dir, QStringLiteral("click-a.psd"));
    const QString second = makePsd(dir, QStringLiteral("click-b.psd"));
    QVERIFY(!first.isEmpty() && !second.isEmpty());
    QVERIFY(window_->openPath(first));

    window_->startOpenPath(second);
    QTRY_VERIFY(window_->isLoadingDocument(1));
    TabSpinner* spinner = spinnerAt(1);
    QVERIFY(spinner != nullptr);
    QTest::mouseClick(spinner, Qt::LeftButton);
    QTRY_COMPARE(window_->documentCount(), 1);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->documentName(0), QStringLiteral("click-a.psd"));

    // The worker is still held by the delay; its result must be discarded.
    QTest::qWait(kOpenDelayMs + 150);
    QCOMPARE(window_->documentCount(), 1);
    QVERIFY(window_->statusBar()->currentMessage().isEmpty());
}

void LoadingTabTest::middleClickCancels()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("middle.psd"));
    QVERIFY(!path.isEmpty());

    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    const QRect rect = bar->tabRect(0);
    QTest::mouseClick(bar, Qt::MiddleButton, Qt::NoModifier, rect.center());
    QTRY_COMPARE(window_->documentCount(), 0);
    // negative window: proves the held worker's late result adds nothing
    QTest::qWait(kOpenDelayMs + 150);
    QCOMPARE(window_->documentCount(), 0);
}

void LoadingTabTest::middleClickClosesNormalTab()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("normal.psd"));
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openPath(path));

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QVERIFY2(qobject_cast<TabCloseButton*>(bar->tabButton(0, QTabBar::RightSide)) != nullptr,
             "a ready tab carries its close button");
    // The loading filter only cancels loading tabs; a ready tab closes.
    const QRect rect = bar->tabRect(0);
    QTest::mouseClick(bar, Qt::MiddleButton, Qt::NoModifier, rect.center());
    QTRY_COMPARE(window_->documentCount(), 0);
}

void LoadingTabTest::closeButtonClosesTheDocument()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("close-button.psd"));
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openPath(path));

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    auto* close = qobject_cast<TabCloseButton*>(bar->tabButton(0, QTabBar::RightSide));
    QVERIFY(close != nullptr);
    QTest::mouseClick(close, Qt::LeftButton);
    QTRY_COMPARE(window_->documentCount(), 0);
}

void LoadingTabTest::imageTabHasCloseButton()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("image-tab.png"));
    QVERIFY(writePng(path));
    QVERIFY(window_->openImagePath(path));

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    auto* close = qobject_cast<TabCloseButton*>(bar->tabButton(0, QTabBar::RightSide));
    QVERIFY2(close != nullptr, "an image tab carries a close button");
    QVERIFY2(close->isVisible(), "the image tab's close button is visible");
    QTest::mouseClick(close, Qt::LeftButton);
    QTRY_COMPARE(window_->documentCount(), 0);
}

void LoadingTabTest::longTitlesKeepTheCloseButton()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(
        dir, QStringLiteral("this-is-a-very-long-document-name-for-the-tab-close-button.psd"));
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openPath(path));

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    const QRect tab = bar->tabRect(0);
    QVERIFY(tab.isValid());
    QVERIFY2(bar->tabText(0).contains(QChar(0x2026)),
             qPrintable(bar->tabText(0)));

    auto* close = qobject_cast<TabCloseButton*>(bar->tabButton(0, QTabBar::RightSide));
    QVERIFY2(close != nullptr, "the tab has a close button");
    const QRect closeRect = close->geometry();
    QVERIFY2(closeRect.left() >= tab.left(), "the close button starts inside the tab");
    QVERIFY2(closeRect.right() <= tab.right(), "the close button ends inside the tab");
    QVERIFY2(tab.right() <= bar->width(), "the tab fits inside the tab bar");
    QVERIFY(bar->tabToolTip(0).contains(QStringLiteral("this-is-a-very-long")));
}

void LoadingTabTest::tooltipShowsFullUntitledLabel()
{
    QVERIFY(window_->newDocument(QStringLiteral("Untitled"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    const QString name =
        QStringLiteral("an-untitled-document-whose-name-is-far-wider-than-its-tab");
    window_->setDocumentNameForTest(window_->activeDocumentIndex(), name);

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QCOMPARE(bar->count(), 1);
    QTRY_COMPARE(bar->tabToolTip(0), fullLabel(name));
    QVERIFY2(bar->tabText(0).contains(QChar(0x2026)), qPrintable(bar->tabText(0)));
    QVERIFY(bar->tabToolTip(0) != bar->tabText(0));
}

void LoadingTabTest::tooltipShowsFullSavedLabel()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString fileName =
        QStringLiteral("a-saved-document-whose-name-is-far-wider-than-its-tab.psd");
    const QString path = makePsd(dir, fileName);
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openPath(path));

    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    QCOMPARE(bar->count(), 1);
    QTRY_COMPARE(bar->tabToolTip(0), fullLabel(fileName));
    QVERIFY(bar->tabToolTip(0) != path);
    QVERIFY(!bar->tabToolTip(0).contains(dir.path()));
    QVERIFY(bar->tabToolTip(0) != bar->tabText(0));
}

void LoadingTabTest::cancelledResultIsDiscarded()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("discard.psd"));
    QVERIFY(!path.isEmpty());

    int outcome = -1;
    const quint64 token = window_->startOpenPath(path);
    connect(window_.get(), &PicturaMainWindow::documentOpenFinished, this,
            [&](quint64 finished, int result, const QString&) {
                if (finished == token) {
                    outcome = result;
                }
            });
    QTRY_VERIFY(window_->isLoadingDocument(0));
    window_->closeDocument(0, false);
    QTRY_COMPARE(outcome, int(PicturaMainWindow::OpenOutcome::Cancelled));
    // negative window: proves the held worker's late result adds nothing
    QTest::qWait(kOpenDelayMs + 150);
    QCOMPARE(window_->documentCount(), 0);
    QVERIFY(window_->statusBar()->currentMessage().isEmpty());
}

void LoadingTabTest::successFillsTabAndActivates()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("success.psd"));
    QVERIFY(!path.isEmpty());

    window_->startOpenPath(path);
    QTRY_VERIFY(window_->isLoadingDocument(0));
    QVERIFY(spinnerAt(0) != nullptr);
    QTRY_VERIFY(!window_->isLoadingDocument(0));
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QVERIFY(window_->activeView()->has_document());
    QVERIFY(window_->imageView() != nullptr);
    QVERIFY2(qobject_cast<TabCloseButton*>(tabBar()->tabButton(0, QTabBar::RightSide)) != nullptr,
             "the spinner is removed and the close button returns");
}

void LoadingTabTest::userSelectedTabIsNotStolen()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = makePsd(dir, QStringLiteral("keep-a.psd"));
    const QString second = makePsd(dir, QStringLiteral("keep-b.psd"));
    QVERIFY(!first.isEmpty() && !second.isEmpty());

    QVERIFY(window_->openPath(first));
    window_->startOpenPath(second);
    QTRY_VERIFY(window_->isLoadingDocument(1));
    QTabBar* bar = tabBar();
    QVERIFY(bar != nullptr);
    // Select the loading tab, then go back to the first: the user explicitly
    // chose another tab during the load.
    QTest::mouseClick(bar, Qt::LeftButton, Qt::NoModifier, bar->tabRect(1).center());
    QTest::mouseClick(bar, Qt::LeftButton, Qt::NoModifier, bar->tabRect(0).center());
    QCOMPARE(window_->activeDocumentIndex(), 0);

    QTRY_VERIFY(!window_->isLoadingDocument(1));
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QVERIFY(window_->viewAt(1)->has_document());
}

void LoadingTabTest::successKeepsStatusNotice()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("cmyk.psd"));
    QVERIFY(writeFlatCmykPsd(path));

    QVERIFY(window_->openDocumentAtPath(path));
    QCOMPARE(window_->statusBar()->currentMessage(),
             QStringLiteral("Converted from CMYK; saved as CMYK"));
    QVERIFY(!window_->isLoadingDocument(0));
}

void LoadingTabTest::failureRemovesTab()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString first = makePsd(dir, QStringLiteral("failure-a.psd"));
    const QString missing = dir.filePath(QStringLiteral("missing.psd"));
    QVERIFY(!first.isEmpty());
    QVERIFY(window_->openPath(first));

    int outcome = -1;
    const quint64 token = window_->startOpenPath(missing);
    connect(window_.get(), &PicturaMainWindow::documentOpenFinished, this,
            [&](quint64 finished, int result, const QString&) {
                if (finished == token) {
                    outcome = result;
                }
            });
    QTRY_VERIFY(window_->isLoadingDocument(1));
    QTRY_COMPARE(outcome, int(PicturaMainWindow::OpenOutcome::Failed));
    QCOMPARE(window_->documentCount(), 1);
    QCOMPARE(window_->activeDocumentIndex(), 0);
    QCOMPARE(window_->documentName(0), QStringLiteral("failure-a.psd"));
}

void LoadingTabTest::cancelIsNotFailure()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("cancel.psd"));
    QVERIFY(!path.isEmpty());

    int outcome = -1;
    QString error;
    const quint64 token = window_->startOpenPath(path);
    connect(window_.get(), &PicturaMainWindow::documentOpenFinished, this,
            [&](quint64 finished, int result, const QString& reason) {
                if (finished == token) {
                    outcome = result;
                    error = reason;
                }
            });
    QTRY_VERIFY(window_->isLoadingDocument(0));
    window_->closeDocument(0, false);
    QTRY_COMPARE(outcome, int(PicturaMainWindow::OpenOutcome::Cancelled));
    QVERIFY(error.isEmpty());
    QVERIFY(window_->statusBar()->currentMessage().isEmpty());
}

void LoadingTabTest::bannerIsGone()
{
    QVERIFY(window_->findChild<QLabel*>(QStringLiteral("documentOpenBanner")) == nullptr);
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = makePsd(dir, QStringLiteral("banner.psd"));
    QVERIFY(!path.isEmpty());
    QVERIFY(window_->openPath(path));
    QVERIFY(window_->findChild<QLabel*>(QStringLiteral("documentOpenBanner")) == nullptr);
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<LoadingTabTest>(argc, argv);
}
#include "tst_loading_tab.moc"
