#pragma once

#include "export_batch.h"

#include <QtCore/QString>

class QWidget;

namespace pictura {
class PictureView;
}

namespace pictura {

QString layerCompsSuffix(const QString& format);

// The batch spec the export runs, for tests that replace only the writer.
ExportBatchSpec layerCompsExportBatchSpec(PictureView& view, QWidget* parent,
                                          const QString& folder, const QString& prefix,
                                          const QString& format, bool includeIcc);

bool exportLayerCompsFromView(QWidget* parent, PictureView* view);
bool exportLayerComps(PictureView* view, const QString& folder, const QString& prefix,
                      const QString& format, bool includeIcc, QWidget* parent = nullptr);
bool exportLayerCompsPdfFromView(QWidget* parent, PictureView* view);
bool exportLayerCompsPdf(PictureView* view, const QString& path, QWidget* parent = nullptr);

} // namespace pictura
