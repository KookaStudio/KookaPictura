#pragma once

#include <QtCore/QString>

class QWidget;

namespace pictura {
class PictureView;
}

namespace pictura {

QString layerCompsSuffix(const QString& format);
bool exportLayerCompsFromView(QWidget* parent, PictureView* view);
bool exportLayerComps(PictureView* view, const QString& folder, const QString& prefix,
                      const QString& format, bool includeIcc, QWidget* parent = nullptr);
bool exportLayerCompsPdfFromView(QWidget* parent, PictureView* view);
bool exportLayerCompsPdf(PictureView* view, const QString& path, QWidget* parent = nullptr);

} // namespace pictura
