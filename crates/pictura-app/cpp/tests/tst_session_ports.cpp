// The session-command ports (#358): each typed resolver reaches the handler that
// implements it and returns null for a tool that does not.

#include <QtTest/QtTest>

#include "frame.h"
#include "tools.h"

#include "qt_test_support.h"

class SessionPortsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void resolversPickImplementingHandlers();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SessionPortsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SessionPortsTest::resolversPickImplementingHandlers()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Ports"), 32, 32, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    auto* tools = frame.findChild<pictura::ToolController*>();
    QVERIFY(tools);
    QVERIFY(tools->handlerFor(pictura::ToolId::Brush));

    // Crop and Perspective Crop both implement the commit port; the crop box is
    // Crop-only and Perspective Crop is always registered.
    frame.setActiveTool(pictura::ToolId::Crop);
    QVERIFY(tools->cropCommitSession()
            == dynamic_cast<pictura::CropCommitSession*>(
                tools->handlerFor(pictura::ToolId::Crop)));
    QVERIFY(tools->cropBoxSession());
    QVERIFY(tools->perspectiveCropSession());
    QVERIFY(dynamic_cast<pictura::TextSession*>(tools->handlerFor(pictura::ToolId::Crop))
            == nullptr);

    // With Perspective Crop active the commit resolver targets its handler, not
    // the Crop one (the active-vs-fixed ToolId choice).
    frame.setActiveTool(pictura::ToolId::PerspectiveCrop);
    QVERIFY(tools->cropCommitSession()
            == dynamic_cast<pictura::CropCommitSession*>(
                tools->handlerFor(pictura::ToolId::PerspectiveCrop)));
    QVERIFY(tools->cropCommitSession()
            != dynamic_cast<pictura::CropCommitSession*>(
                tools->handlerFor(pictura::ToolId::Crop)));

    // The type port follows the active tool: present for a type tool, null for
    // one that does not implement it.
    frame.setActiveTool(pictura::ToolId::HorizontalType);
    QVERIFY(tools->textSession());
    frame.setActiveTool(pictura::ToolId::Brush);
    QVERIFY(!tools->textSession());

    // A tool that does not own a port resolves to null when cast directly.
    QVERIFY(dynamic_cast<pictura::CropBoxSession*>(tools->handlerFor(pictura::ToolId::Brush))
            == nullptr);
    QVERIFY(dynamic_cast<pictura::PerspectiveCropSession*>(tools->handlerFor(pictura::ToolId::Crop))
            == nullptr);
}

QTEST_MAIN(SessionPortsTest)
#include "tst_session_ports.moc"
