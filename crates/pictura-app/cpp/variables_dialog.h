#pragma once

#include <QtCore/QJsonObject>
#include <QtCore/QString>

class QWidget;

namespace pictura {
class ImageView;
class PictureView;

namespace variable_dialogs {

QJsonObject state(const PictureView& view, bool draft = false);
QString command(PictureView& view, const QString& action, const QJsonObject& fields = {});
void define(PictureView& view, ImageView* canvas, QWidget* parent, bool setsPage);
void importSets(PictureView& view, QWidget* parent);
void apply(PictureView& view, ImageView* canvas, QWidget* parent);
void exportSets(PictureView& view, QWidget* parent, const QString& filePath, const QString& name);

} // namespace variable_dialogs
} // namespace pictura
