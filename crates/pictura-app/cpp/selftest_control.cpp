#include "selftest_control.h"
#include "selftest_report.h"

#include "control_server.h"
#include "frame.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QEventLoop>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QJsonParseError>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtNetwork/QLocalSocket>

#ifndef _WIN32
#include <sys/stat.h>
#endif

namespace {

// One blocking send/response round-trip. The server dispatches synchronously on
// the GUI thread, so a nested event loop drives the reply; the self-test never
// re-enters exec().
QByteArray transact(QLocalSocket& socket, const QByteArray& request, bool appendNewline = true,
                    int timeoutMs = 2000)
{
    socket.write(request + (appendNewline ? QByteArrayLiteral("\n") : QByteArray()));
    socket.flush();
    QByteArray buffer;
    QEventLoop loop;
    QTimer timer;
    timer.setSingleShot(true);
    QObject::connect(&timer, &QTimer::timeout, &loop, &QEventLoop::quit);
    QObject::connect(&socket, &QLocalSocket::readyRead, &loop, &QEventLoop::quit);
    timer.start(timeoutMs);
    for (;;) {
        buffer.append(socket.readAll());
        if (buffer.indexOf('\n') >= 0 || !timer.isActive()) {
            break;
        }
        loop.exec();
    }
    const int newline = buffer.indexOf('\n');
    if (newline < 0) {
        return QByteArray();
    }
    socket.readAll();
    return buffer.left(newline);
}

QJsonObject responseObject(const QByteArray& line, bool* parsed)
{
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(line, &error);
    *parsed = error.error == QJsonParseError::NoError && document.isObject();
    return *parsed ? document.object() : QJsonObject();
}

QJsonObject request(QLocalSocket& socket, int id, const QString& method, const QJsonObject& params,
                    bool* parsed)
{
    const QJsonObject body{{QStringLiteral("id"), id},
                           {QStringLiteral("method"), method},
                           {QStringLiteral("params"), params}};
    const QByteArray line =
        QJsonDocument(body).toJson(QJsonDocument::Compact) + QByteArrayLiteral("\n");
    return responseObject(transact(socket, line, false), parsed);
}

QString errorCode(const QJsonObject& response)
{
    return response.value(QStringLiteral("error"))
        .toObject()
        .value(QStringLiteral("code"))
        .toString();
}

QImage decodePng(const QString& base64, bool* ok)
{
    const QByteArray bytes = QByteArray::fromBase64(base64.toLatin1());
    QImage image;
    *ok = image.loadFromData(bytes, "PNG");
    return *ok ? image : QImage();
}

} // namespace

int pictura::runControlChecks(pictura::PicturaMainWindow& frame)
{
    QTemporaryDir dir;
    const QString socketPath = dir.path() + QStringLiteral("/control.sock");
    pictura::ControlServer server(&frame, socketPath);
    ST_BEGIN("mcp_control_server");
    if (!dir.isValid() || !server.listen()) {
        return pictura::selfTest().fail(463, "control server listen");
    }
    ST_PASS("socket=%s", qPrintable(socketPath));

    QLocalSocket socket;
    socket.connectToServer(socketPath);
    if (!socket.waitForConnected(2000)) {
        return pictura::selfTest().fail(464, "control client connect");
    }

    ST_BEGIN("mcp_control_socket_mode");
#ifdef _WIN32
    // A Windows QLocalServer is a machine-global named pipe with no filesystem
    // mode to stat; there is no per-user ACL to assert, so the POSIX mode check
    // has no analogue here.
    ST_SKIP("named pipe (no filesystem mode to check)");
#else
    struct stat status{};
    const bool userOnly = ::stat(socketPath.toLocal8Bit().constData(), &status) == 0
                          && (status.st_mode & 0077) == 0;
    if (!userOnly) {
        return pictura::selfTest().fail(465, "socket is not user-only");
    }
    ST_PASS("mode=%03o", static_cast<int>(status.st_mode & 0777));
#endif

    bool parsed = false;
    QJsonObject response =
        request(socket, 1, QStringLiteral("list_commands"), QJsonObject(), &parsed);
    const QJsonArray commands = response.value(QStringLiteral("result"))
                                    .toObject()
                                    .value(QStringLiteral("commands"))
                                    .toArray();
    ST_BEGIN("mcp_control_commands");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool() || commands.isEmpty()) {
        return pictura::selfTest().fail(466, "list_commands response malformed");
    }
    ST_PASS("commands=%d", static_cast<int>(commands.size()));

    response = request(socket, 2, QStringLiteral("status"), QJsonObject(), &parsed);
    const QJsonObject statusResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_status");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !statusResult.contains(QStringLiteral("documents"))
        || !statusResult.contains(QStringLiteral("documents_info"))
        || !statusResult.contains(QStringLiteral("active_tool"))) {
        return pictura::selfTest().fail(467, "status response malformed");
    }
    ST_PASS("documents=%d active_tool=%s", statusResult.value("documents").toInt(),
            qPrintable(statusResult.value("active_tool").toString()));

    response = request(socket, 3, QStringLiteral("document"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("new")},
                                   {QStringLiteral("width"), 8},
                                   {QStringLiteral("height"), 8}},
                       &parsed);
    ST_BEGIN("mcp_control_new");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()) {
        return pictura::selfTest().fail(468, "document new failed");
    }
    ST_PASS("count=%d", response.value("result").toObject().value("count").toInt());

    response = request(socket, 4, QStringLiteral("dispatch_command"),
                       QJsonObject{{QStringLiteral("id"), QStringLiteral("select.all")}}, &parsed);
    const QJsonObject dispatchResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_dispatch");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !dispatchResult.value(QStringLiteral("dispatched")).toBool()) {
        return pictura::selfTest().fail(469, "dispatch_command response malformed");
    }
    ST_PASS("dispatched=%d", dispatchResult.value("dispatched").toBool() ? 1 : 0);

    response = request(socket, 5, QStringLiteral("dispatch_command"),
                       QJsonObject{{QStringLiteral("id"), QStringLiteral("image.rotate90cw")}},
                       &parsed);
    const QJsonObject mutateResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_mutate");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !mutateResult.value(QStringLiteral("dispatched")).toBool()) {
        return pictura::selfTest().fail(482, "history mutation not dispatched");
    }
    ST_PASS("dispatched=%d", mutateResult.value("dispatched").toBool() ? 1 : 0);

    response = request(socket, 6, QStringLiteral("edit"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("undo")}}, &parsed);
    const QJsonObject undoResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_edit");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !undoResult.value(QStringLiteral("success")).toBool()
        || !undoResult.value(QStringLiteral("can_redo")).toBool()
        || !undoResult.contains(QStringLiteral("depth"))) {
        return pictura::selfTest().fail(470, "edit undo did not succeed");
    }
    ST_PASS("undo success=1 can_redo=1 depth=%d", undoResult.value("depth").toInt());

    response = request(socket, 7, QStringLiteral("edit"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("redo")}}, &parsed);
    const QJsonObject redoResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_redo");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !redoResult.value(QStringLiteral("success")).toBool()
        || !redoResult.value(QStringLiteral("can_undo")).toBool()) {
        return pictura::selfTest().fail(471, "edit redo did not succeed");
    }
    ST_PASS("redo success=1 depth=%d", redoResult.value("depth").toInt());

    response = request(socket, 7, QStringLiteral("dispatch_command"),
                       QJsonObject{{QStringLiteral("id"), QStringLiteral("file.open")}}, &parsed);
    ST_BEGIN("mcp_control_refused");
    if (!parsed || errorCode(response) != QStringLiteral("refused")) {
        return pictura::selfTest().fail(472, "modal command not refused");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    response = request(socket, 8, QStringLiteral("document"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("save")}}, &parsed);
    ST_BEGIN("mcp_control_save_no_path");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(473, "untitled save did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    QImage raster(4, 4, QImage::Format_ARGB32);
    raster.fill(Qt::red);
    const QString rasterPath = dir.filePath(QStringLiteral("raster.png"));
    const bool rasterWritten = raster.save(rasterPath, "PNG");
    response = request(socket, 9, QStringLiteral("document"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("open")},
                                   {QStringLiteral("path"), rasterPath}},
                       &parsed);
    ST_BEGIN("mcp_control_open_raster");
    if (!rasterWritten || !parsed || !response.value(QStringLiteral("ok")).toBool()) {
        return pictura::selfTest().fail(474, "raster document open failed");
    }
    ST_PASS("count=%d", response.value("result").toObject().value("count").toInt());

    response = request(socket, 10, QStringLiteral("status"), QJsonObject(), &parsed);
    const QJsonObject statusAfter = response.value(QStringLiteral("result")).toObject();
    const QJsonArray infos = statusAfter.value(QStringLiteral("documents_info")).toArray();
    const int activeIndex = statusAfter.value(QStringLiteral("active")).toInt();
    const QJsonObject activeInfo = (activeIndex >= 0 && activeIndex < infos.size())
                                       ? infos.at(activeIndex).toObject()
                                       : QJsonObject();
    ST_BEGIN("mcp_control_status_fields");
    if (!parsed || !activeInfo.contains(QStringLiteral("mode"))
        || !activeInfo.contains(QStringLiteral("depth"))
        || !activeInfo.contains(QStringLiteral("layers"))
        || !activeInfo.contains(QStringLiteral("selection_px"))
        || !activeInfo.contains(QStringLiteral("zoom"))) {
        return pictura::selfTest().fail(475, "status per-document fields missing");
    }
    ST_PASS("mode=%s depth=%d layers=%d", qPrintable(activeInfo.value("mode").toString()),
            activeInfo.value("depth").toInt(), activeInfo.value("layers").toInt());

    response = request(socket, 11, QStringLiteral("list_layers"), QJsonObject(), &parsed);
    const QJsonArray layers =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("layers")).toArray();
    const QJsonObject layer0 = layers.isEmpty() ? QJsonObject() : layers.at(0).toObject();
    ST_BEGIN("mcp_control_layers");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool() || layers.isEmpty()
        || !layer0.contains(QStringLiteral("index")) || !layer0.contains(QStringLiteral("name"))
        || !layer0.contains(QStringLiteral("kind")) || !layer0.contains(QStringLiteral("visible"))
        || !layer0.contains(QStringLiteral("opacity")) || !layer0.contains(QStringLiteral("blend"))
        || !layer0.contains(QStringLiteral("fill")) || !layer0.contains(QStringLiteral("lock"))
        || !layer0.contains(QStringLiteral("color"))) {
        return pictura::selfTest().fail(476, "list_layers fields missing");
    }
    ST_PASS("layers=%d name=%s", static_cast<int>(layers.size()),
            qPrintable(layer0.value("name").toString()));

    response = request(socket, 12, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QJsonObject pixel = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_pixel");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool() || !pixel.contains(QStringLiteral("argb"))
        || !pixel.contains(QStringLiteral("r")) || !pixel.contains(QStringLiteral("g"))
        || !pixel.contains(QStringLiteral("b")) || !pixel.contains(QStringLiteral("a"))
        || pixel.value(QStringLiteral("a")).toInt() != 255) {
        return pictura::selfTest().fail(477, "get_pixel response malformed");
    }
    ST_PASS("argb=%s a=%d", qPrintable(pixel.value("argb").toString()),
            pixel.value("a").toInt());

    response = request(socket, 13, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 100000}, {QStringLiteral("y"), 100000}},
                       &parsed);
    ST_BEGIN("mcp_control_pixel_oob");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(478, "out-of-range get_pixel did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    response = responseObject(transact(socket, QByteArrayLiteral("this is not json")), &parsed);
    ST_BEGIN("mcp_control_bad_request");
    if (!parsed || response.value(QStringLiteral("ok")).toBool()
        || errorCode(response) != QStringLiteral("bad_request")) {
        return pictura::selfTest().fail(479, "malformed line did not yield bad_request");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    response = request(socket, 7, QStringLiteral("no_such_method"), QJsonObject(), &parsed);
    ST_BEGIN("mcp_control_unknown_method");
    if (!parsed || response.value(QStringLiteral("id")).toInt() != 7
        || response.value(QStringLiteral("ok")).toBool()
        || errorCode(response) != QStringLiteral("unknown_method")) {
        return pictura::selfTest().fail(480, "unknown method did not yield unknown_method");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // Unterminated request larger than the buffer ceiling: rejected, not buffered.
    const QByteArray oversized(pictura::ControlServer::kMaxRequestBytes + 4096, 'x');
    const QByteArray oversizedLine = transact(socket, oversized, false, 5000);
    response = responseObject(oversizedLine, &parsed);
    ST_BEGIN("mcp_control_oversized");
    if (!parsed || errorCode(response) != QStringLiteral("bad_request")) {
        return pictura::selfTest().fail(481, "oversized request not rejected");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // The oversized request disconnected that socket; open a fresh one for the
    // vision checks. `ui_tree` below uses a default call.
    QLocalSocket visionSocket;
    ST_BEGIN("mcp_control_vision_connect");
    visionSocket.connectToServer(socketPath);
    if (!visionSocket.waitForConnected(2000)) {
        return pictura::selfTest().fail(483, "vision client reconnect");
    }

    response = request(visionSocket, 20, QStringLiteral("screenshot"),
                       QJsonObject{{QStringLiteral("scope"), QStringLiteral("window")}}, &parsed);
    QJsonObject imageResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_vision_shot");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || imageResult.value(QStringLiteral("mime")).toString() != QStringLiteral("image/png")
        || imageResult.value(QStringLiteral("base64")).toString().isEmpty()) {
        return pictura::selfTest().fail(484, "screenshot response malformed");
    }
    bool pngOk = false;
    QImage png = decodePng(imageResult.value(QStringLiteral("base64")).toString(), &pngOk);
    if (!pngOk || png.width() != imageResult.value(QStringLiteral("width")).toInt()
        || png.height() != imageResult.value(QStringLiteral("height")).toInt()) {
        return pictura::selfTest().fail(485, "screenshot PNG/dimensions mismatch");
    }
    ST_PASS("png=%dx%d source=%dx%d", png.width(), png.height(),
            imageResult.value("source_width").toInt(),
            imageResult.value("source_height").toInt());

    response = request(visionSocket, 21, QStringLiteral("screenshot"),
                       QJsonObject{{QStringLiteral("scope"), QStringLiteral("window")},
                                   {QStringLiteral("max_dim"), 64}},
                       &parsed);
    imageResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_vision_downscale");
    const int scaledLong =
        qMax(imageResult.value("width").toInt(), imageResult.value("height").toInt());
    const int sourceLong =
        qMax(imageResult.value("source_width").toInt(), imageResult.value("source_height").toInt());
    if (!parsed || !response.value(QStringLiteral("ok")).toBool() || scaledLong > 64
        || sourceLong <= scaledLong) {
        return pictura::selfTest().fail(486, "screenshot downscale did not report source");
    }
    ST_PASS("scaled=%d source=%d", scaledLong, sourceLong);

    response = request(visionSocket, 22, QStringLiteral("ui_tree"), QJsonObject(), &parsed);
    const QJsonArray nodes =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("nodes")).toArray();
    QJsonObject layersPanelRect;
    for (const QJsonValue& value : nodes) {
        const QJsonObject node = value.toObject();
        if (node.value(QStringLiteral("objectName")).toString() != QStringLiteral("layersPanel")) {
            continue;
        }
        layersPanelRect = node.value(QStringLiteral("rect")).toObject();
        break;
    }
    ST_BEGIN("mcp_control_vision_tree");
    const int rectX = layersPanelRect.value(QStringLiteral("x")).toInt(-1);
    const int rectY = layersPanelRect.value(QStringLiteral("y")).toInt(-1);
    const int rectW = layersPanelRect.value(QStringLiteral("w")).toInt(-1);
    const int rectH = layersPanelRect.value(QStringLiteral("h")).toInt(-1);
    const bool rectSane = rectX > 0 && rectY >= 0 && rectW > 0 && rectH > 0
                          && rectX < frame.width() && rectY < frame.height();
    if (!parsed || !response.value(QStringLiteral("ok")).toBool() || !rectSane) {
        return pictura::selfTest().fail(
            487, "ui_tree layersPanel rect x=%d y=%d w=%d h=%d frame=%dx%d", rectX, rectY, rectW,
            rectH, frame.width(), frame.height());
    }
    ST_PASS("nodes=%d layersPanel_rect=(%d,%d,%d,%d)", static_cast<int>(nodes.size()), rectX, rectY,
            rectW, rectH);

    response = request(visionSocket, 23, QStringLiteral("layer_thumbnail"),
                       QJsonObject{{QStringLiteral("index"), 0}, {QStringLiteral("size"), 64}},
                       &parsed);
    imageResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_vision_layer");
    bool thumbOk = false;
    const QImage thumb = decodePng(imageResult.value(QStringLiteral("base64")).toString(), &thumbOk);
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || imageResult.value(QStringLiteral("mime")).toString() != QStringLiteral("image/png")
        || !thumbOk || qMax(thumb.width(), thumb.height()) > 64) {
        return pictura::selfTest().fail(488, "pixel layer thumbnail is not a PNG");
    }
    ST_PASS("thumbnail=%dx%d", thumb.width(), thumb.height());

    PictureView* active = frame.activeView();
    const int groupIndex = active ? active->add_group(0) : -1;
    response = request(visionSocket, 24, QStringLiteral("layer_thumbnail"),
                       QJsonObject{{QStringLiteral("index"), groupIndex},
                                   {QStringLiteral("size"), 64}},
                       &parsed);
    ST_BEGIN("mcp_control_vision_group");
    if (groupIndex < 0 || !parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(489, "group layer thumbnail did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // Caller-supplied image dimensions are bounded at the trust boundary.
    response = request(visionSocket, 25, QStringLiteral("layer_thumbnail"),
                       QJsonObject{{QStringLiteral("index"), 0},
                                   {QStringLiteral("size"), 100000}},
                       &parsed);
    ST_BEGIN("mcp_control_vision_thumb_cap");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(490, "oversized thumbnail size not rejected");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    response = request(visionSocket, 26, QStringLiteral("screenshot"),
                       QJsonObject{{QStringLiteral("scope"), QStringLiteral("window")},
                                   {QStringLiteral("max_dim"), 0}},
                       &parsed);
    ST_BEGIN("mcp_control_vision_maxdim");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(491, "non-positive max_dim not rejected");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // Close every document so the canvas-with-no-document scenario is testable.
    while (frame.documentCount() > 0) {
        frame.closeDocument(0, false);
    }
    response = request(visionSocket, 27, QStringLiteral("screenshot"),
                       QJsonObject{{QStringLiteral("scope"), QStringLiteral("canvas")}}, &parsed);
    ST_BEGIN("mcp_control_vision_canvas");
    if (!parsed || errorCode(response) != QStringLiteral("no_document")) {
        return pictura::selfTest().fail(492, "canvas screenshot with no document not reported");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // Action surface.
    response = request(visionSocket, 40, QStringLiteral("set_tool"),
                       QJsonObject{{QStringLiteral("tool"), QStringLiteral("marquee")}}, &parsed);
    QJsonObject actionResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_action_tool");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || actionResult.value(QStringLiteral("active_tool")).toString()
               != QStringLiteral("marquee")) {
        return pictura::selfTest().fail(493, "set_tool did not switch to marquee");
    }
    ST_PASS("active_tool=%s", qPrintable(actionResult.value("active_tool").toString()));

    response = request(visionSocket, 41, QStringLiteral("status"), QJsonObject(), &parsed);
    if (!parsed
        || response.value(QStringLiteral("result"))
                   .toObject()
                   .value(QStringLiteral("active_tool"))
                   .toString()
               != QStringLiteral("marquee")) {
        return pictura::selfTest().fail(494, "status disagrees with set_tool");
    }

    response = request(visionSocket, 42, QStringLiteral("set_tool"),
                       QJsonObject{{QStringLiteral("tool"), QStringLiteral("no-such-tool")}},
                       &parsed);
    ST_BEGIN("mcp_control_action_tool_unknown");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(495, "unknown tool did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    const int scratchDocuments = frame.documentCount();
    response = request(visionSocket, 43, QStringLiteral("document"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("new")},
                                   {QStringLiteral("width"), 8},
                                   {QStringLiteral("height"), 8}},
                       &parsed);
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()) {
        return pictura::selfTest().fail(496, "action document new failed");
    }

    response = request(visionSocket, 44, QStringLiteral("selection"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("rect")},
                                   {QStringLiteral("x"), 1},
                                   {QStringLiteral("y"), 1},
                                   {QStringLiteral("w"), 4},
                                   {QStringLiteral("h"), 4}},
                       &parsed);
    actionResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_action_selection");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !actionResult.contains(QStringLiteral("has_selection"))
        || !actionResult.contains(QStringLiteral("count"))
        || !actionResult.value(QStringLiteral("has_selection")).toBool()
        || actionResult.value(QStringLiteral("count")).toInt() <= 0
        || actionResult.value(QStringLiteral("bounds")).toString().isEmpty()) {
        return pictura::selfTest().fail(497, "rect selection produced no selection");
    }
    ST_PASS("count=%d bounds=%s", actionResult.value("count").toInt(),
            qPrintable(actionResult.value("bounds").toString()));

    response = request(visionSocket, 45, QStringLiteral("selection"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("deselect")}}, &parsed);
    actionResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_action_deselect");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || actionResult.value(QStringLiteral("has_selection")).toBool()
        || actionResult.value(QStringLiteral("count")).toInt() != 0) {
        return pictura::selfTest().fail(498, "deselect did not clear the selection");
    }
    ST_PASS("has_selection=0 count=0");

    response = request(visionSocket, 60, QStringLiteral("selection"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("rect")},
                                   {QStringLiteral("x"), 0},
                                   {QStringLiteral("y"), 0},
                                   {QStringLiteral("w"), 0},
                                   {QStringLiteral("h"), 4}},
                       &parsed);
    ST_BEGIN("mcp_control_action_selection_zero");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(507, "zero-size rect did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    response = request(visionSocket, 61, QStringLiteral("selection"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("rect")},
                                   {QStringLiteral("x"), 0},
                                   {QStringLiteral("y"), 0},
                                   {QStringLiteral("w"), 4},
                                   {QStringLiteral("h"), 4},
                                   {QStringLiteral("mode"), QStringLiteral("union")}},
                       &parsed);
    ST_BEGIN("mcp_control_action_selection_mode");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(508, "unknown selection mode did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // A fixed-seed filter is deterministic: apply, undo, apply again and the
    // pixel must match the first application and differ from the pre-filter one.
    response = request(visionSocket, 46, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString pixelBefore =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();
    response = request(visionSocket, 47, QStringLiteral("filter"),
                       QJsonObject{{QStringLiteral("kind"), QStringLiteral("add-noise")}}, &parsed);
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()) {
        return pictura::selfTest().fail(499, "add-noise filter failed");
    }
    response = request(visionSocket, 48, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString pixelFiltered =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();
    ST_BEGIN("mcp_control_action_filter");
    if (pixelFiltered == pixelBefore) {
        return pictura::selfTest().fail(500, "filter did not change the pixel");
    }
    response = request(visionSocket, 49, QStringLiteral("edit"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("undo")}}, &parsed);
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !response.value(QStringLiteral("result"))
                .toObject()
                .value(QStringLiteral("success"))
                .toBool()) {
        return pictura::selfTest().fail(509, "edit undo after filter failed");
    }
    response = request(visionSocket, 50, QStringLiteral("filter"),
                       QJsonObject{{QStringLiteral("kind"), QStringLiteral("add-noise")}}, &parsed);
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()) {
        return pictura::selfTest().fail(501, "second add-noise filter failed");
    }
    response = request(visionSocket, 51, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString pixelRefiltered =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();
    if (pixelRefiltered != pixelFiltered) {
        return pictura::selfTest().fail(502, "filter is not deterministic");
    }
    ST_PASS("deterministic=1 argb=%s", qPrintable(pixelFiltered));

    response = request(visionSocket, 52, QStringLiteral("filter"),
                       QJsonObject{{QStringLiteral("kind"), QStringLiteral("no-such-kind")}}, &parsed);
    ST_BEGIN("mcp_control_action_filter_unknown");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(503, "unknown filter kind did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // The filters above may now alter transparency (an unlocked layer's alpha is
    // filtered), so restore the pre-filter layer before the translate check
    // below, which assumes an opaque layer covers the one beneath it.
    request(visionSocket, 80, QStringLiteral("edit"),
            QJsonObject{{QStringLiteral("op"), QStringLiteral("undo")}}, &parsed);

    response = request(visionSocket, 53, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("set_opacity")},
                                   {QStringLiteral("index"), 0},
                                   {QStringLiteral("value"), 128}},
                       &parsed);
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()) {
        return pictura::selfTest().fail(504, "layer_op set_opacity failed");
    }
    response = request(visionSocket, 54, QStringLiteral("list_layers"), QJsonObject(), &parsed);
    const QJsonArray actionLayers =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("layers")).toArray();
    ST_BEGIN("mcp_control_action_layer");
    if (!parsed || actionLayers.isEmpty()
        || actionLayers.at(0).toObject().value(QStringLiteral("opacity")).toInt() != 128) {
        return pictura::selfTest().fail(505, "layer opacity did not round-trip");
    }
    ST_PASS("opacity=%d", actionLayers.at(0).toObject().value("opacity").toInt());

    response = request(visionSocket, 55, QStringLiteral("set_gpu_compute"),
                       QJsonObject{{QStringLiteral("on"), false}}, &parsed);
    actionResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_action_gpu");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !actionResult.contains(QStringLiteral("gpu_compute"))
        || actionResult.value(QStringLiteral("gpu_compute")).toBool()
        || !actionResult.contains(QStringLiteral("backend"))
        || actionResult.value(QStringLiteral("backend")).toString().isEmpty()) {
        return pictura::selfTest().fail(506, "set_gpu_compute false not reported");
    }
    ST_PASS("gpu_compute=0 backend=%s", qPrintable(actionResult.value("backend").toString()));

    // A handler that ignored `on` would fail this half.
    response = request(visionSocket, 57, QStringLiteral("set_gpu_compute"),
                       QJsonObject{{QStringLiteral("on"), true}}, &parsed);
    actionResult = response.value(QStringLiteral("result")).toObject();
    ST_BEGIN("mcp_control_action_gpu_on");
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()
        || !actionResult.contains(QStringLiteral("gpu_compute"))
        || !actionResult.value(QStringLiteral("gpu_compute")).toBool()
        || actionResult.value(QStringLiteral("backend")).toString().isEmpty()) {
        return pictura::selfTest().fail(512, "set_gpu_compute true not reported");
    }
    ST_PASS("gpu_compute=1 backend=%s", qPrintable(actionResult.value("backend").toString()));

    // set_visible must honor an explicit bool in either key form and reject a
    // non-bool value.
    response = request(visionSocket, 62, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("set_visible")},
                                   {QStringLiteral("index"), 0},
                                   {QStringLiteral("visible"), false}},
                       &parsed);
    const bool hidOk = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 63, QStringLiteral("list_layers"), QJsonObject(), &parsed);
    const QJsonArray hiddenLayers =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("layers")).toArray();
    ST_BEGIN("mcp_control_action_visible");
    if (!hidOk || !parsed || hiddenLayers.isEmpty()
        || hiddenLayers.at(0).toObject().value(QStringLiteral("visible")).toBool()) {
        return pictura::selfTest().fail(513, "set_visible visible:false did not hide the layer");
    }
    ST_PASS("visible=0");

    response = request(visionSocket, 64, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("set_visible")},
                                   {QStringLiteral("index"), 0},
                                   {QStringLiteral("on"), true}},
                       &parsed);
    const bool showOk = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 65, QStringLiteral("list_layers"), QJsonObject(), &parsed);
    const QJsonArray shownLayers =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("layers")).toArray();
    ST_BEGIN("mcp_control_action_visible_on");
    if (!showOk || !parsed || shownLayers.isEmpty()
        || !shownLayers.at(0).toObject().value(QStringLiteral("visible")).toBool()) {
        return pictura::selfTest().fail(514, "set_visible on:true did not show the layer");
    }
    ST_PASS("visible=1");

    response = request(visionSocket, 66, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("set_visible")},
                                   {QStringLiteral("index"), 0},
                                   {QStringLiteral("visible"), QStringLiteral("yes")}},
                       &parsed);
    ST_BEGIN("mcp_control_action_visible_type");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(515, "non-bool set_visible did not yield invalid_param");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // A pixel-locked, confirmed-visible layer must refuse the filter: assert the
    // layer is visible first so `refused` cannot come from a hidden layer.
    response = request(visionSocket, 67, QStringLiteral("list_layers"), QJsonObject(), &parsed);
    const QJsonArray visibleBeforeLock =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("layers")).toArray();
    const bool layerShown = !visibleBeforeLock.isEmpty()
                            && visibleBeforeLock.at(0).toObject().value(QStringLiteral("visible")).toBool();
    response = request(visionSocket, 68, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("set_lock")},
                                   {QStringLiteral("index"), 0},
                                   {QStringLiteral("flag"), QStringLiteral("pixels")},
                                   {QStringLiteral("on"), true}},
                       &parsed);
    const bool lockedOk = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 69, QStringLiteral("filter"),
                       QJsonObject{{QStringLiteral("kind"), QStringLiteral("add-noise")}}, &parsed);
    ST_BEGIN("mcp_control_action_filter_locked");
    if (!layerShown || !lockedOk || !parsed
        || errorCode(response) != QStringLiteral("refused")) {
        return pictura::selfTest().fail(511, "filter on a pixel-locked layer did not yield refused");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));
    request(visionSocket, 70, QStringLiteral("layer_op"),
            QJsonObject{{QStringLiteral("op"), QStringLiteral("set_lock")},
                        {QStringLiteral("index"), 0},
                        {QStringLiteral("flag"), QStringLiteral("pixels")},
                        {QStringLiteral("on"), false}},
            &parsed);

    // translate must honor `index`. Make the top layer opaque so moving the
    // bottom layer is invisible: with index handling, translating index 0 leaves
    // the composite unchanged and index 1 changes it; if `index` were ignored
    // both would move the active layer and the index-1 check would fail.
    response = request(visionSocket, 71, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("duplicate")},
                                   {QStringLiteral("index"), 0}},
                       &parsed);
    const int layersAfterDuplicate =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("layers")).toInt();
    response = request(visionSocket, 78, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("set_opacity")},
                                   {QStringLiteral("index"), 1},
                                   {QStringLiteral("value"), 255}},
                       &parsed);
    const bool opaqueTop = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 72, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString translatePixelBefore =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();

    response = request(visionSocket, 73, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("translate")},
                                   {QStringLiteral("index"), 0},
                                   {QStringLiteral("dx"), 2},
                                   {QStringLiteral("dy"), 2}},
                       &parsed);
    const bool translate0Ok = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 74, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString pixelAfterBottom =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();
    response = request(visionSocket, 79, QStringLiteral("edit"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("undo")}}, &parsed);
    const bool undoOk = parsed && response.value(QStringLiteral("ok")).toBool();

    response = request(visionSocket, 75, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("translate")},
                                   {QStringLiteral("index"), 1},
                                   {QStringLiteral("dx"), 2},
                                   {QStringLiteral("dy"), 2}},
                       &parsed);
    const bool translate1Ok = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 76, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString pixelAfterTop =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();

    ST_BEGIN("mcp_control_action_translate");
    if (layersAfterDuplicate != 2 || !opaqueTop || translatePixelBefore.isEmpty() || !translate0Ok
        || !undoOk || !translate1Ok || pixelAfterBottom != translatePixelBefore
        || pixelAfterTop == translatePixelBefore) {
        return pictura::selfTest().fail(516, "translate did not move only the indexed layer");
    }
    ST_PASS("layer1_moved=1 layer0_intact=1");

    response = request(visionSocket, 77, QStringLiteral("layer_op"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("translate")},
                                   {QStringLiteral("index"), 99},
                                   {QStringLiteral("dx"), 1},
                                   {QStringLiteral("dy"), 1}},
                       &parsed);
    ST_BEGIN("mcp_control_action_translate_oob");
    if (!parsed || errorCode(response) != QStringLiteral("invalid_param")) {
        return pictura::selfTest().fail(517, "out-of-range translate index not rejected");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // Close only the documents this block created, leaving earlier state intact.
    while (frame.documentCount() > scratchDocuments) {
        frame.closeDocument(frame.documentCount() - 1, false);
    }

    // Input synthesis: drive the real event path. A `pointer` drag in image
    // space over the marquee tool commits a selection. The `key` half uses the
    // Move tool's arrow-key nudge: `PicturaMainWindow::keyPressEvent` handles it
    // (frame.cpp) and no `QShortcut`/`QAction` claims an arrow key, so it reaches
    // the widget handler whether or not the window is active. (A tool-selection
    // letter is itself a `QShortcut`, and a lone `F` is the first key of the
    // `F,F` chord, so the shortcut machinery consumes both once the window is
    // active.)
    ST_BEGIN("mcp_control_pointer_key_input");
    const int inputScratch = frame.documentCount();
    response = request(visionSocket, 80, QStringLiteral("document"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("new")},
                                   {QStringLiteral("width"), 32},
                                   {QStringLiteral("height"), 32}},
                       &parsed);
    if (!parsed || !response.value(QStringLiteral("ok")).toBool()) {
        return pictura::selfTest().fail(523, "input fixture document new failed");
    }
    request(visionSocket, 81, QStringLiteral("set_tool"),
            QJsonObject{{QStringLiteral("tool"), QStringLiteral("marquee")}}, &parsed);

    response = request(visionSocket, 82, QStringLiteral("pointer"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("drag")},
                                   {QStringLiteral("space"), QStringLiteral("image")},
                                   {QStringLiteral("x"), 4},
                                   {QStringLiteral("y"), 4},
                                   {QStringLiteral("x2"), 20},
                                   {QStringLiteral("y2"), 20},
                                   {QStringLiteral("steps"), 6}},
                       &parsed);
    const bool dragOk = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 83, QStringLiteral("status"), QJsonObject(), &parsed);
    const QJsonObject inputStatus = response.value(QStringLiteral("result")).toObject();
    const QJsonArray inputInfos = inputStatus.value(QStringLiteral("documents_info")).toArray();
    const int inputActive = inputStatus.value(QStringLiteral("active")).toInt();
    const int selectionPx =
        (inputActive >= 0 && inputActive < inputInfos.size())
            ? inputInfos.at(inputActive).toObject().value(QStringLiteral("selection_px")).toInt()
            : 0;

    response = request(visionSocket, 84, QStringLiteral("pointer"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("zoom")},
                                   {QStringLiteral("space"), QStringLiteral("image")},
                                   {QStringLiteral("x"), 1},
                                   {QStringLiteral("y"), 1}},
                       &parsed);
    const bool badOp = errorCode(response) == QStringLiteral("invalid_param");
    response = request(visionSocket, 85, QStringLiteral("pointer"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("drag")},
                                   {QStringLiteral("space"), QStringLiteral("image")},
                                   {QStringLiteral("x"), 1},
                                   {QStringLiteral("y"), 1}},
                       &parsed);
    const bool dragNoEnd = errorCode(response) == QStringLiteral("invalid_param");
    response = request(visionSocket, 86, QStringLiteral("key"),
                       QJsonObject{{QStringLiteral("sequence"), QStringLiteral("Ctrl+NotAKey")}},
                       &parsed);
    const bool badKey = errorCode(response) == QStringLiteral("invalid_param");

    // Nudge the active layer right, then left. `get_pixel` is the readback: a
    // 1 px move of the full-canvas white layer exposes transparency at (0,0),
    // and the reverse nudge restores it.
    response = request(visionSocket, 87, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString nudgeBefore =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();
    request(visionSocket, 88, QStringLiteral("set_tool"),
            QJsonObject{{QStringLiteral("tool"), QStringLiteral("move")}}, &parsed);
    response = request(visionSocket, 89, QStringLiteral("key"),
                       QJsonObject{{QStringLiteral("sequence"), QStringLiteral("Right")}}, &parsed);
    const bool keyOk = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 90, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString nudgeMoved =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();
    response = request(visionSocket, 91, QStringLiteral("key"),
                       QJsonObject{{QStringLiteral("sequence"), QStringLiteral("Left")}}, &parsed);
    const bool restoreOk = parsed && response.value(QStringLiteral("ok")).toBool();
    response = request(visionSocket, 92, QStringLiteral("get_pixel"),
                       QJsonObject{{QStringLiteral("x"), 0}, {QStringLiteral("y"), 0}}, &parsed);
    const QString nudgeRestored =
        response.value(QStringLiteral("result")).toObject().value(QStringLiteral("argb")).toString();

    while (frame.documentCount() > inputScratch) {
        frame.closeDocument(frame.documentCount() - 1, false);
    }
    if (!dragOk || selectionPx <= 0 || !badOp || !dragNoEnd || !badKey || !keyOk || !restoreOk
        || nudgeBefore.isEmpty() || nudgeMoved == nudgeBefore || nudgeRestored != nudgeBefore) {
        return pictura::selfTest().fail(522, "pointer/key input synthesis");
    }
    ST_PASS("selection_px=%d nudge=%s->%s->%s", selectionPx, qPrintable(nudgeBefore),
            qPrintable(nudgeMoved), qPrintable(nudgeRestored));

    // With every document closed, an image-space pointer has no target.
    response = request(visionSocket, 93, QStringLiteral("pointer"),
                       QJsonObject{{QStringLiteral("op"), QStringLiteral("drag")},
                                   {QStringLiteral("space"), QStringLiteral("image")},
                                   {QStringLiteral("x"), 1},
                                   {QStringLiteral("y"), 1},
                                   {QStringLiteral("x2"), 2},
                                   {QStringLiteral("y2"), 2}},
                       &parsed);
    ST_BEGIN("mcp_control_pointer_no_document");
    if (!parsed || errorCode(response) != QStringLiteral("no_document")) {
        return pictura::selfTest().fail(524, "image-space pointer with no document not reported");
    }
    ST_PASS("code=%s", qPrintable(errorCode(response)));

    // Best-effort: restore the tool the self-test started with.
    request(visionSocket, 56, QStringLiteral("set_tool"),
            QJsonObject{{QStringLiteral("tool"), QStringLiteral("move")}}, &parsed);
    return 0;
}
