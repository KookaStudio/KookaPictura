#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QJsonArray>
#include <QtGui/QAction>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"

#include "qt_test_support.h"

// Filters on a Smart Object become Smart Filters (#984): the Filter menu, the
// Filter Gallery, and the dialog preview append to the object's chain instead
// of filtering its proxy.
class SmartFilterApplyTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void filterMenuAppendsSmartFilter();
    void selectionBecomesFilterMask();
    void filterGalleryAddsOneEntry();
    void excludedFiltersDisabledOnSmartObject();
    void previewCancelRestoresSmartObject();
    void undoRemovesSmartFilter();
    void adjustmentsDisabledOnSmartObject();
    void editSmartFilterDisabledForGenericEntry();

private:
    // A new document whose active layer is an embedded Smart Object; returns
    // its layer path.
    QString smartObject(const QString& name);
    bool applyGaussianBlur(double radius);
    QAction* action(const QStringList& path);
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SmartFilterApplyTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QCoreApplication::processEvents();
}

void SmartFilterApplyTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

QString SmartFilterApplyTest::smartObject(const QString& name)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return {};
    }
    pictura::PictureView* view = window_->activeView();
    const QString path = view->add_solid_fill(0xff808080u);
    if (!view->rasterize_fill_content(path) || !view->convert_to_smart_object(path)) {
        return {};
    }
    view->set_active_layer(path);
    return path;
}

bool SmartFilterApplyTest::applyGaussianBlur(double radius)
{
    pictura::CommandParams params;
    params.values.insert(QStringLiteral("kind"), QStringLiteral("gaussian-blur"));
    params.values.insert(QStringLiteral("params"), QJsonArray{radius});
    return window_->registry()->executeRecorded(
        pictura::commandIdForPath(
            {QStringLiteral("Filter"), QStringLiteral("Blur"), QStringLiteral("Gaussian Blur")}),
        params);
}

QAction* SmartFilterApplyTest::action(const QStringList& path)
{
    return window_->registry()->action(pictura::commandIdForPath(path));
}

void SmartFilterApplyTest::filterMenuAppendsSmartFilter()
{
    const QString path = smartObject(QStringLiteral("Append"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QVERIFY(applyGaussianBlur(3.0));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 2);
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Gaussian Blur"));
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 1), QStringLiteral("Gaussian Blur"));
    QVERIFY(!pictura::layer_has_smart_filter_mask(*view, path));
}

void SmartFilterApplyTest::selectionBecomesFilterMask()
{
    const QString path = smartObject(QStringLiteral("Mask"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    view->select_rect(0, 0, 8, 16, QStringLiteral("new"), 0.0);
    QVERIFY(applyGaussianBlur(2.0));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QVERIFY(pictura::layer_has_smart_filter_mask(*view, path));
}

void SmartFilterApplyTest::filterGalleryAddsOneEntry()
{
    const QString path = smartObject(QStringLiteral("Gallery"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    const QStringList kinds{QStringLiteral("gaussian-blur"), QStringLiteral("gaussian-blur")};
    QVERIFY(pictura::apply_filter_stack(*view, kinds, QList<double>{1.0, 2.0}));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Filter Gallery"));
}

void SmartFilterApplyTest::excludedFiltersDisabledOnSmartObject()
{
    QVERIFY(!smartObject(QStringLiteral("Excluded")).isEmpty());
    window_->registry()->refresh();
    for (const char* kind :
         {"liquify", "vanishing-point", "lens-blur", "displace", "adaptive-wide-angle"}) {
        const pictura::FilterCommandSpec* spec =
            pictura::filterCommandForKind(QString::fromLatin1(kind));
        QVERIFY2(spec, kind);
        QAction* entry = action(spec->path);
        QVERIFY2(entry && !entry->isEnabled(), kind);
    }
    QAction* blur =
        action({QStringLiteral("Filter"), QStringLiteral("Blur"), QStringLiteral("Gaussian Blur")});
    QVERIFY(blur && blur->isEnabled());
}

void SmartFilterApplyTest::previewCancelRestoresSmartObject()
{
    const QString path = smartObject(QStringLiteral("Preview"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::filter_preview(*view, QStringLiteral("gaussian-blur"), {4.0}));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QVERIFY(pictura::filter_preview_cancel(*view));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
}

void SmartFilterApplyTest::undoRemovesSmartFilter()
{
    const QString path = smartObject(QStringLiteral("Undo"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QVERIFY(view->undo());
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
}

void SmartFilterApplyTest::adjustmentsDisabledOnSmartObject()
{
    QVERIFY(!smartObject(QStringLiteral("Adjust")).isEmpty());
    pictura::PictureView* view = window_->activeView();
    const QStringList invert{QStringLiteral("Image"), QStringLiteral("Adjustments"),
                             QStringLiteral("Invert")};
    QVERIFY(pictura::filter_target_smart(*view));
    window_->registry()->refresh();
    QVERIFY(action(invert) && !action(invert)->isEnabled());

    // The Background is a raster layer, where the adjustments apply again.
    view->set_active_layer(QStringLiteral("0"));
    QVERIFY(!pictura::filter_target_smart(*view));
    window_->registry()->refresh();
    QVERIFY(action(invert)->isEnabled());
}

void SmartFilterApplyTest::editSmartFilterDisabledForGenericEntry()
{
    const QString path = smartObject(QStringLiteral("Edit"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QCOMPARE(pictura::layer_smart_filter_edit_kind(*view, path, 0), 0);
}

QTEST_MAIN(SmartFilterApplyTest)
#include "tst_smart_filter_apply.moc"
