#pragma once

#include <QtCore/QString>

namespace pictura {
class PictureView;
}

namespace pictura::layer_comps_api {

int count(const PictureView& view);
int id(const PictureView& view, int index);
QString name(const PictureView& view, int index);
QString comment(const PictureView& view, int index);
int record(const PictureView& view, int index);
bool warning(const PictureView& view, int index);
bool lastAvailable(const PictureView& view);

int create(PictureView& view, const QString& name, const QString& comment, int recordBits);
bool apply(PictureView& view, int id);
bool update(PictureView& view, int id);
bool remove(PictureView& view, int id);
bool rename(PictureView& view, int id, const QString& name);
bool setOptions(PictureView& view, int id, const QString& name, const QString& comment,
                int recordBits);
int duplicate(PictureView& view, int id);
bool clearWarning(PictureView& view, int id);
bool clearAllWarnings(PictureView& view);
bool restoreLast(PictureView& view);

QString exportTargets(const PictureView& view, const QString& prefix);
bool exportFile(const PictureView& view, int id, const QString& format, bool includeIcc,
                const QString& path);
bool exportPdf(const PictureView& view, const QString& path);

} // namespace pictura::layer_comps_api
