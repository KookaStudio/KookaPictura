#include "control_server.h"

#include <QtCore/QBuffer>
#include <QtCore/QJsonArray>
#include <QtGui/QImage>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>

#include "frame.h"
#include "image_view.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

namespace pictura {

namespace {

// In-memory PNG encode; returns empty on failure.
QByteArray encodePngBase64(const QImage& image)
{
    QByteArray bytes;
    QBuffer buffer(&bytes);
    buffer.open(QIODevice::WriteOnly);
    if (!image.save(&buffer, "PNG")) {
        return QByteArray();
    }
    buffer.close();
    return bytes.toBase64();
}

// ponytail: one capped JSON line is the whole response-size guard; the long
// edge is bounded to maxDim before encoding.
QImage scaleToMaxDim(const QImage& image, int maxDim)
{
    if (qMax(image.width(), image.height()) <= maxDim) {
        return image;
    }
    return image.scaled(maxDim, maxDim, Qt::KeepAspectRatio, Qt::SmoothTransformation);
}

QString widgetText(QWidget* widget)
{
    if (const QAbstractButton* button = qobject_cast<QAbstractButton*>(widget)) {
        if (!button->text().isEmpty()) {
            return button->text();
        }
    }
    if (const QLabel* label = qobject_cast<QLabel*>(widget)) {
        if (!label->text().isEmpty()) {
            return label->text();
        }
    }
    if (const QMenu* menu = qobject_cast<QMenu*>(widget)) {
        if (!menu->title().isEmpty()) {
            return menu->title();
        }
    }
    if (const QGroupBox* group = qobject_cast<QGroupBox*>(widget)) {
        if (!group->title().isEmpty()) {
            return group->title();
        }
    }
    if (!widget->windowTitle().isEmpty()) {
        return widget->windowTitle();
    }
    return QString();
}

// Pre-order DFS of the QWidget hierarchy, capped in depth and children per
// node. Rects are window-local to `root`.
void appendWidgetTree(QWidget* widget, QWidget* root, int depth, int maxDepth, int maxChildren,
                      QJsonArray& nodes)
{
    const QPoint origin = widget->mapTo(root, QPoint(0, 0));
    const QSize size = widget->size();
    nodes.append(QJsonObject{
        {QStringLiteral("class"), QString::fromUtf8(widget->metaObject()->className())},
        {QStringLiteral("objectName"), widget->objectName()},
        {QStringLiteral("rect"),
         QJsonObject{{QStringLiteral("x"), origin.x()},
                     {QStringLiteral("y"), origin.y()},
                     {QStringLiteral("w"), size.width()},
                     {QStringLiteral("h"), size.height()}}},
        {QStringLiteral("visible"), widget->isVisible()},
        {QStringLiteral("enabled"), widget->isEnabled()},
        {QStringLiteral("text"), widgetText(widget)},
        {QStringLiteral("tooltip"), widget->toolTip()}});

    if (depth >= maxDepth) {
        return;
    }
    int taken = 0;
    for (QObject* child : widget->children()) {
        if (taken >= maxChildren) {
            break;
        }
        QWidget* childWidget = qobject_cast<QWidget*>(child);
        if (!childWidget) {
            continue;
        }
        ++taken;
        appendWidgetTree(childWidget, root, depth + 1, maxDepth, maxChildren, nodes);
    }
}

} // namespace

QJsonObject ControlServer::methodScreenshot(const QJsonObject& params)
{
    const QString scope = params.value(QStringLiteral("scope")).toString(QStringLiteral("window"));
    const int maxDim =
        params.contains(QStringLiteral("max_dim"))
            ? params.value(QStringLiteral("max_dim")).toInt(1280)
            : 1280;
    if (maxDim <= 0 || maxDim > kMaxScreenshotDim) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("max_dim must be in 1..%1").arg(kMaxScreenshotDim));
    }

    QImage image;
    if (scope == QStringLiteral("window")) {
        image = frame_->grab().toImage();
    } else if (scope == QStringLiteral("canvas")) {
        ImageView* canvas = frame_->imageView();
        if (!canvas) {
            return error(QStringLiteral("no_document"), QStringLiteral("no active canvas"));
        }
        image = canvas->grab().toImage();
    } else if (scope == QStringLiteral("document")) {
        PictureView* view = frame_->activeView();
        if (!view || !view->has_document()) {
            return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
        }
        image = view->image();
    } else {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("unknown scope: %1").arg(scope));
    }
    if (image.isNull()) {
        return error(QStringLiteral("internal"), QStringLiteral("could not capture image"));
    }

    const int sourceWidth = image.width();
    const int sourceHeight = image.height();
    image = scaleToMaxDim(image, maxDim);
    const QByteArray base64 = encodePngBase64(image);
    if (base64.isEmpty()) {
        return error(QStringLiteral("internal"), QStringLiteral("PNG encode failed"));
    }
    QJsonObject result;
    result.insert(QStringLiteral("mime"), QStringLiteral("image/png"));
    result.insert(QStringLiteral("base64"), QString::fromLatin1(base64));
    result.insert(QStringLiteral("width"), image.width());
    result.insert(QStringLiteral("height"), image.height());
    result.insert(QStringLiteral("source_width"), sourceWidth);
    result.insert(QStringLiteral("source_height"), sourceHeight);
    return ok(result);
}

QJsonObject ControlServer::methodUiTree(const QJsonObject& params)
{
    const int maxDepth =
        params.contains(QStringLiteral("max_depth"))
            ? params.value(QStringLiteral("max_depth")).toInt(12)
            : 12;
    const int maxChildren =
        params.contains(QStringLiteral("max_children"))
            ? params.value(QStringLiteral("max_children")).toInt(64)
            : 64;
    if (maxDepth < 0 || maxDepth > kMaxUiTreeDepth || maxChildren < 0
        || maxChildren > kMaxUiTreeChildren) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("max_depth must be in 0..%1 and max_children in 0..%2")
                         .arg(kMaxUiTreeDepth)
                         .arg(kMaxUiTreeChildren));
    }
    QJsonArray nodes;
    appendWidgetTree(frame_, frame_, 0, maxDepth, maxChildren, nodes);
    QJsonObject result;
    result.insert(QStringLiteral("nodes"), nodes);
    result.insert(QStringLiteral("count"), nodes.size());
    return ok(result);
}

QJsonObject ControlServer::methodLayerThumbnail(const QJsonObject& params)
{
    PictureView* view = frame_->activeView();
    if (!view || !view->has_document()) {
        return error(QStringLiteral("no_document"), QStringLiteral("no active document"));
    }
    if (!params.contains(QStringLiteral("index"))) {
        return error(QStringLiteral("invalid_param"), QStringLiteral("index is required"));
    }
    const int index = params.value(QStringLiteral("index")).toInt(-1);
    const int size = params.contains(QStringLiteral("size"))
                         ? params.value(QStringLiteral("size")).toInt(64)
                         : 64;
    if (index < 0 || size <= 0 || size > kMaxThumbnailDim) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("index must be non-negative and size must be in 1..%1")
                         .arg(kMaxThumbnailDim));
    }
    const QImage image = view->layer_thumbnail(index, size);
    if (image.isNull()) {
        return error(QStringLiteral("invalid_param"),
                     QStringLiteral("layer has no thumbnail (group, adjustment, or out of range)"));
    }
    const QByteArray base64 = encodePngBase64(image);
    if (base64.isEmpty()) {
        return error(QStringLiteral("internal"), QStringLiteral("PNG encode failed"));
    }
    QJsonObject result;
    result.insert(QStringLiteral("mime"), QStringLiteral("image/png"));
    result.insert(QStringLiteral("base64"), QString::fromLatin1(base64));
    result.insert(QStringLiteral("width"), image.width());
    result.insert(QStringLiteral("height"), image.height());
    result.insert(QStringLiteral("source_width"), image.width());
    result.insert(QStringLiteral("source_height"), image.height());
    return ok(result);
}

} // namespace pictura
